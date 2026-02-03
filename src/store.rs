// Copyright 2026 Recall Authors
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cmp::Ordering;
use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::path::Path;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use fs2::FileExt;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use rusqlite::OptionalExtension;
use rusqlite::params;
use sha2::Digest;
use sha2::Sha256;

use crate::ann;
use crate::config::Config;
use crate::embed::cosine_similarity;
use crate::embed::from_bytes;
use crate::output::CorpusStats;

pub struct Store {
    pub conn: Connection,
    pub path: PathBuf,
    lock: Option<StoreLock>,
}

struct StoreLock {
    _file: File,
    path: PathBuf,
    mode: StoreMode,
}

impl StoreLock {
    fn new(file: File, path: PathBuf, mode: StoreMode) -> Self {
        Self {
            _file: file,
            path,
            mode,
        }
    }
}

const SCHEMA_VERSION: i64 = 1;
const ANN_VERSION: &str = "lsh-v1";
const FTS_VERSION: &str = "fts5-v1";

#[derive(Debug, Clone, Copy)]
pub enum StoreMode {
    ReadOnly,
    ReadWrite,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct StoreStats {
    pub doc_count: i64,
    pub chunk_count: i64,
    pub db_size_bytes: u64,
}

#[derive(Debug)]
pub struct IntegrityReport {
    pub status: String,
    pub stats: StoreStats,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ConsistencyReport {
    pub chunk_count: i64,
    pub fts_count: i64,
    pub fts_missing: i64,
    pub ann_count: i64,
    pub ann_missing: i64,
    pub hnsw_count: i64,
    pub hnsw_missing: i64,
}

impl ConsistencyReport {
    pub fn fts_ok(&self) -> bool {
        self.chunk_count == self.fts_count && self.fts_missing == 0
    }

    pub fn ann_ok(&self) -> bool {
        self.chunk_count == self.ann_count && self.ann_missing == 0
    }

    pub fn hnsw_ok(&self) -> bool {
        self.chunk_count == self.hnsw_count && self.hnsw_missing == 0
    }
}

impl Store {
    pub fn init(path: &Path) -> Result<()> {
        if path.exists() {
            anyhow::bail!("store already exists at {}", path.display());
        }
        let _lock = Self::acquire_lock(path, StoreMode::ReadWrite)?;
        let conn = Self::open_connection(path, StoreMode::ReadWrite)?;
        Self::apply_pragmas(&conn, StoreMode::ReadWrite)?;
        Self::create_schema(&conn)?;
        Self::set_meta(&conn, "schema_version", &SCHEMA_VERSION.to_string())?;
        Self::set_meta(&conn, "ann_version", ANN_VERSION)?;
        Self::set_meta(&conn, "fts_version", FTS_VERSION)?;
        Ok(())
    }

    pub fn open(path: &Path, mode: StoreMode) -> Result<Self> {
        let mut lock = Self::acquire_lock(path, mode)?;
        let mut conn = Self::open_connection(path, mode)?;
        Self::apply_pragmas(&conn, mode)?;
        if matches!(mode, StoreMode::ReadWrite) {
            Self::create_schema(&conn)?;
            Self::migrate(&conn)?;
            return Ok(Self {
                conn,
                path: path.to_path_buf(),
                lock: Some(lock),
            });
        }

        let version = Self::schema_version(&conn)?;
        if version != SCHEMA_VERSION {
            drop(conn);
            drop(lock);
            let lock_rw = Self::acquire_lock(path, StoreMode::ReadWrite)?;
            let conn_rw = Self::open_connection(path, StoreMode::ReadWrite)?;
            Self::apply_pragmas(&conn_rw, StoreMode::ReadWrite)?;
            Self::create_schema(&conn_rw)?;
            Self::migrate(&conn_rw)?;
            drop(conn_rw);
            drop(lock_rw);

            lock = Self::acquire_lock(path, StoreMode::ReadOnly)?;
            conn = Self::open_connection(path, StoreMode::ReadOnly)?;
            Self::apply_pragmas(&conn, StoreMode::ReadOnly)?;
        }

        Ok(Self {
            conn,
            path: path.to_path_buf(),
            lock: Some(lock),
        })
    }

    fn open_connection(path: &Path, mode: StoreMode) -> Result<Connection> {
        let flags = match mode {
            StoreMode::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
            StoreMode::ReadWrite => {
                OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE
            }
        };
        let conn = Connection::open_with_flags(path, flags)
            .with_context(|| format!("open {}", path.display()))?;
        conn.busy_timeout(Duration::from_millis(5000))
            .context("set busy timeout")?;
        Ok(conn)
    }

    fn apply_pragmas(conn: &Connection, mode: StoreMode) -> Result<()> {
        let mut batch = String::from("PRAGMA foreign_keys=ON;");
        if matches!(mode, StoreMode::ReadWrite) {
            batch = format!("PRAGMA journal_mode=DELETE;\nPRAGMA synchronous=NORMAL;\n{batch}");
        }
        conn.execute_batch(&batch).context("apply pragmas")?;
        Ok(())
    }

    fn lock_path_for(path: &Path) -> Result<PathBuf> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let mut hasher = Sha256::new();
        hasher.update(canonical.to_string_lossy().as_bytes());
        let hash = hex::encode(hasher.finalize());
        let mut dir = std::env::temp_dir();
        dir.push("recall");
        fs::create_dir_all(&dir).with_context(|| format!("create lock dir {}", dir.display()))?;
        Ok(dir.join(format!("recall-{hash}.lock")))
    }

    fn acquire_lock(path: &Path, mode: StoreMode) -> Result<StoreLock> {
        let lock_path = Self::lock_path_for(path)?;
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&lock_path)
            .with_context(|| format!("open lock file {}", lock_path.display()))?;
        let deadline = Instant::now() + Duration::from_millis(5000);
        loop {
            let locked = match mode {
                StoreMode::ReadOnly => file.try_lock_shared().map_err(|err| err.to_string()),
                StoreMode::ReadWrite => file.try_lock_exclusive().map_err(|err| err.to_string()),
            };
            match locked {
                Ok(()) => return Ok(StoreLock::new(file, lock_path, mode)),
                Err(_) if Instant::now() >= deadline => {
                    let mode_label = match mode {
                        StoreMode::ReadOnly => "read",
                        StoreMode::ReadWrite => "write",
                    };
                    anyhow::bail!(
                        "store is locked for {mode_label} access; another process may be using {}",
                        path.display()
                    );
                }
                Err(_) => {
                    sleep(Duration::from_millis(50));
                }
            }
        }
    }

    fn create_schema(conn: &Connection) -> Result<()> {
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS meta (\n  key TEXT PRIMARY KEY,\n  value TEXT\n);\n\nCREATE TABLE IF NOT EXISTS doc (\n  id TEXT PRIMARY KEY,\n  path TEXT,\n  mtime TEXT,\n  size INTEGER,\n  hash TEXT,\n  tag TEXT,\n  source TEXT,\n  meta TEXT,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE TABLE IF NOT EXISTS chunk (\n  rowid INTEGER PRIMARY KEY,\n  id TEXT UNIQUE,\n  doc_id TEXT,\n  offset INTEGER,\n  tokens INTEGER,\n  text TEXT,\n  embedding BLOB,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE INDEX IF NOT EXISTS idx_doc_path ON doc(path);\nCREATE INDEX IF NOT EXISTS idx_doc_tag ON doc(tag);\nCREATE INDEX IF NOT EXISTS idx_chunk_doc ON chunk(doc_id);\n\nCREATE VIRTUAL TABLE IF NOT EXISTS chunk_fts USING fts5(text, content='chunk', content_rowid='rowid');\n\nCREATE TABLE IF NOT EXISTS ann_lsh (\n  signature INTEGER,\n  chunk_id TEXT,\n  doc_id TEXT\n);\n\nCREATE INDEX IF NOT EXISTS idx_ann_sig ON ann_lsh(signature);\nCREATE INDEX IF NOT EXISTS idx_ann_doc ON ann_lsh(doc_id);\n\nCREATE TABLE IF NOT EXISTS ann_hnsw (\n  chunk_id TEXT PRIMARY KEY,\n  neighbors TEXT\n);\n\nCREATE INDEX IF NOT EXISTS idx_ann_hnsw_chunk ON ann_hnsw(chunk_id);\n\nCREATE TRIGGER IF NOT EXISTS chunk_ai AFTER INSERT ON chunk BEGIN\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;\n\nCREATE TRIGGER IF NOT EXISTS chunk_ad AFTER DELETE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\nEND;\n\nCREATE TRIGGER IF NOT EXISTS chunk_au AFTER UPDATE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;",
        )
        .context("create schema")?;
        Ok(())
    }

    fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
        conn.execute(
            "INSERT OR REPLACE INTO meta (key, value) VALUES (?1, ?2)",
            params![key, value],
        )
        .context("set meta")?;
        Ok(())
    }

    fn table_exists(conn: &Connection, name: &str) -> Result<bool> {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name=?1",
                params![name],
                |row| row.get(0),
            )
            .context("check table")?;
        Ok(count > 0)
    }

    fn column_exists(conn: &Connection, table: &str, column: &str) -> Result<bool> {
        let mut stmt = conn
            .prepare(&format!("PRAGMA table_info({})", table))
            .context("table info")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for row in rows {
            if row? == column {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn schema_version(conn: &Connection) -> Result<i64> {
        if !Self::table_exists(conn, "meta")? {
            return Ok(0);
        }
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM meta WHERE key='schema_version'",
                [],
                |row| row.get(0),
            )
            .optional()
            .context("read schema_version")?;
        Ok(value.and_then(|v| v.parse::<i64>().ok()).unwrap_or(0))
    }

    fn ensure_doc_meta_column(conn: &Connection) -> Result<()> {
        if !Self::column_exists(conn, "doc", "meta")? {
            conn.execute("ALTER TABLE doc ADD COLUMN meta TEXT", [])
                .context("add doc.meta column")?;
        }
        Ok(())
    }

    fn migrate(conn: &Connection) -> Result<()> {
        let version = Self::schema_version(conn)?;
        if version > SCHEMA_VERSION {
            anyhow::bail!(
                "store schema version {} is newer than supported {}",
                version,
                SCHEMA_VERSION
            );
        }
        if version == SCHEMA_VERSION {
            return Ok(());
        }

        Self::create_schema(conn)?;
        Self::ensure_doc_meta_column(conn)?;
        Self::set_meta(conn, "schema_version", &SCHEMA_VERSION.to_string())?;
        Self::set_meta(conn, "ann_version", ANN_VERSION)?;
        Self::set_meta(conn, "fts_version", FTS_VERSION)?;
        Self::rebuild_ann_hnsw_conn(conn)?;
        Ok(())
    }

    pub fn stats(&self) -> Result<StoreStats> {
        let doc_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM doc WHERE deleted=0", [], |row| {
                row.get(0)
            })
            .context("count docs")?;
        let chunk_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chunk WHERE deleted=0", [], |row| {
                row.get(0)
            })
            .context("count chunks")?;
        let db_size_bytes = std::fs::metadata(&self.path).map(|m| m.len()).unwrap_or(0);
        Ok(StoreStats {
            doc_count,
            chunk_count,
            db_size_bytes,
        })
    }

    pub fn corpus_stats(&self) -> Result<CorpusStats> {
        let stats = self.stats()?;
        let tokens: i64 = self
            .conn
            .query_row(
                "SELECT COALESCE(SUM(tokens), 0) FROM chunk WHERE deleted=0",
                [],
                |row| row.get(0),
            )
            .context("sum tokens")?;
        Ok(CorpusStats {
            docs: stats.doc_count,
            chunks: stats.chunk_count,
            tokens,
            bytes: stats.db_size_bytes,
        })
    }

    pub fn snapshot_token(&self) -> Result<String> {
        let token: Option<String> = self
            .conn
            .query_row("SELECT MAX(mtime) FROM doc WHERE deleted=0", [], |row| {
                row.get(0)
            })
            .context("snapshot token")?;
        Ok(token.unwrap_or_default())
    }

    pub fn integrity_check(&self) -> Result<IntegrityReport> {
        let status: String = self
            .conn
            .query_row("PRAGMA integrity_check", [], |row| row.get(0))
            .context("integrity_check")?;
        let stats = self.stats()?;
        Ok(IntegrityReport { status, stats })
    }

    pub fn consistency_report(&self) -> Result<ConsistencyReport> {
        let chunk_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chunk WHERE deleted=0", [], |row| {
                row.get(0)
            })
            .context("count chunks")?;
        let fts_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chunk_fts", [], |row| row.get(0))
            .context("count fts")?;
        let fts_missing: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*)\n                 FROM chunk\n                 LEFT JOIN chunk_fts ON chunk_fts.rowid = chunk.rowid\n                 WHERE chunk.deleted=0 AND chunk_fts.rowid IS NULL",
                [],
                |row| row.get(0),
            )
            .context("fts missing")?;
        let ann_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM ann_lsh", [], |row| row.get(0))
            .context("count ann")?;
        let ann_missing: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*)\n                 FROM chunk\n                 LEFT JOIN ann_lsh ON ann_lsh.chunk_id = chunk.id\n                 WHERE chunk.deleted=0 AND ann_lsh.chunk_id IS NULL",
                [],
                |row| row.get(0),
            )
            .context("ann missing")?;
        let hnsw_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM ann_hnsw", [], |row| row.get(0))
            .context("count ann_hnsw")?;
        let hnsw_missing: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*)\n                 FROM chunk\n                 LEFT JOIN ann_hnsw ON ann_hnsw.chunk_id = chunk.id\n                 WHERE chunk.deleted=0 AND ann_hnsw.chunk_id IS NULL",
                [],
                |row| row.get(0),
            )
            .context("ann_hnsw missing")?;
        Ok(ConsistencyReport {
            chunk_count,
            fts_count,
            fts_missing,
            ann_count,
            ann_missing,
            hnsw_count,
            hnsw_missing,
        })
    }

    pub fn rebuild_fts(&self) -> Result<()> {
        self.conn
            .execute("INSERT INTO chunk_fts(chunk_fts) VALUES('rebuild')", [])
            .context("rebuild fts")?;
        Ok(())
    }

    pub fn rebuild_ann_lsh(&self, config: &Config) -> Result<usize> {
        self.conn
            .execute("DELETE FROM ann_lsh", [])
            .context("clear ann")?;
        let mut stmt = self
            .conn
            .prepare("SELECT id, doc_id, embedding FROM chunk WHERE deleted=0")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let doc_id: String = row.get(1)?;
            let embedding: Vec<u8> = row.get(2)?;
            Ok((id, doc_id, embedding))
        })?;
        let mut inserted = 0usize;
        for row in rows {
            let (id, doc_id, embedding) = row?;
            let vec = from_bytes(&embedding);
            let sig = ann::signature(&vec, config.ann_bits, config.ann_seed);
            self.conn.execute(
                "INSERT INTO ann_lsh (signature, chunk_id, doc_id) VALUES (?1, ?2, ?3)",
                params![sig as i64, id, doc_id],
            )?;
            inserted += 1;
        }
        Ok(inserted)
    }

    pub fn rebuild_ann_hnsw(&self) -> Result<usize> {
        Self::rebuild_ann_hnsw_conn(&self.conn)
    }

    fn rebuild_ann_hnsw_conn(conn: &Connection) -> Result<usize> {
        const HNSW_M: usize = 8;
        conn.execute("DELETE FROM ann_hnsw", [])
            .context("clear ann_hnsw")?;
        let mut stmt =
            conn.prepare("SELECT id, embedding FROM chunk WHERE deleted=0 ORDER BY id ASC")?;
        let rows = stmt.query_map([], |row| {
            let id: String = row.get(0)?;
            let embedding: Vec<u8> = row.get(1)?;
            Ok((id, from_bytes(&embedding)))
        })?;
        let mut items = Vec::new();
        for row in rows {
            items.push(row?);
        }
        let mut inserted = 0usize;
        for (idx, (id, vec)) in items.iter().enumerate() {
            let mut sims: Vec<(f32, &String)> = items
                .iter()
                .enumerate()
                .filter(|(j, _)| *j != idx)
                .map(|(_, (other_id, other_vec))| (cosine_similarity(vec, other_vec), other_id))
                .collect();
            sims.sort_by(|a, b| {
                let ord = b.0.partial_cmp(&a.0).unwrap_or(Ordering::Equal);
                if ord == Ordering::Equal {
                    a.1.cmp(b.1)
                } else {
                    ord
                }
            });
            let neighbors: Vec<&String> = sims.iter().take(HNSW_M).map(|(_, id)| *id).collect();
            let neighbors_json =
                serde_json::to_string(&neighbors).context("serialize ann_hnsw neighbors")?;
            conn.execute(
                "INSERT OR REPLACE INTO ann_hnsw (chunk_id, neighbors) VALUES (?1, ?2)",
                params![id, neighbors_json],
            )?;
            inserted += 1;
        }
        Ok(inserted)
    }

    pub fn compact(&self) -> Result<()> {
        self.conn.execute("DELETE FROM chunk WHERE deleted=1", [])?;
        self.conn.execute("DELETE FROM doc WHERE deleted=1", [])?;
        self.conn.execute_batch("VACUUM;")?;
        Ok(())
    }

    pub fn mark_doc_deleted(&self, path: &str) -> Result<Vec<String>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id FROM doc WHERE path = ?1 AND deleted = 0")?;
        let ids_iter = stmt.query_map(params![path], |row| row.get::<_, String>(0))?;
        let mut ids = Vec::new();
        for id in ids_iter {
            ids.push(id?);
        }
        if !ids.is_empty() {
            for id in &ids {
                self.conn
                    .execute("UPDATE doc SET deleted=1 WHERE id = ?1", params![id])?;
                self.conn
                    .execute("UPDATE chunk SET deleted=1 WHERE doc_id = ?1", params![id])?;
                self.conn
                    .execute("DELETE FROM ann_lsh WHERE doc_id = ?1", params![id])?;
                self.conn.execute(
                    "DELETE FROM ann_hnsw WHERE chunk_id IN (SELECT id FROM chunk WHERE doc_id = ?1)",
                    params![id],
                )?;
            }
        }
        Ok(ids)
    }

    pub fn mark_doc_deleted_by_id(&self, id: &str) -> Result<usize> {
        let updated = self
            .conn
            .execute("UPDATE doc SET deleted=1 WHERE id = ?1", params![id])?;
        if updated > 0 {
            self.conn
                .execute("UPDATE chunk SET deleted=1 WHERE doc_id = ?1", params![id])?;
            self.conn
                .execute("DELETE FROM ann_lsh WHERE doc_id = ?1", params![id])?;
            self.conn.execute(
                "DELETE FROM ann_hnsw WHERE chunk_id IN (SELECT id FROM chunk WHERE doc_id = ?1)",
                params![id],
            )?;
        }
        Ok(updated)
    }
}

impl Drop for Store {
    fn drop(&mut self) {
        if let Some(lock) = self.lock.take() {
            let path = lock.path.clone();
            let mode = lock.mode;
            drop(lock);
            if matches!(mode, StoreMode::ReadWrite) {
                let _ = fs::remove_file(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection as SqlConnection;
    use tempfile::tempdir;

    use super::*;

    #[test]
    fn shared_lock_allows_multiple_readers() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("recall.db");
        Store::init(&db_path)?;

        let store_a = Store::open(&db_path, StoreMode::ReadOnly)?;
        let store_b = Store::open(&db_path, StoreMode::ReadOnly)?;

        store_a.stats()?;
        store_b.stats()?;
        Ok(())
    }

    #[test]
    fn migrates_unversioned_store() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("recall.db");
        let conn = SqlConnection::open(&db_path)?;
        conn.execute_batch(
            "CREATE TABLE doc (\n  id TEXT PRIMARY KEY,\n  path TEXT,\n  mtime TEXT,\n  size INTEGER,\n  hash TEXT,\n  tag TEXT,\n  source TEXT,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE TABLE chunk (\n  rowid INTEGER PRIMARY KEY,\n  id TEXT UNIQUE,\n  doc_id TEXT,\n  offset INTEGER,\n  tokens INTEGER,\n  text TEXT,\n  embedding BLOB,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE INDEX idx_doc_path ON doc(path);\nCREATE INDEX idx_doc_tag ON doc(tag);\nCREATE INDEX idx_chunk_doc ON chunk(doc_id);\n\nCREATE VIRTUAL TABLE chunk_fts USING fts5(text, content='chunk', content_rowid='rowid');\n\nCREATE TABLE ann_lsh (\n  signature INTEGER,\n  chunk_id TEXT,\n  doc_id TEXT\n);\n\nCREATE INDEX idx_ann_sig ON ann_lsh(signature);\nCREATE INDEX idx_ann_doc ON ann_lsh(doc_id);\n\nCREATE TRIGGER chunk_ai AFTER INSERT ON chunk BEGIN\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;\n\nCREATE TRIGGER chunk_ad AFTER DELETE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\nEND;\n\nCREATE TRIGGER chunk_au AFTER UPDATE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;",
        )?;
        drop(conn);

        let store = Store::open(&db_path, StoreMode::ReadOnly)?;
        let version: String = store
            .conn
            .query_row(
                "SELECT value FROM meta WHERE key='schema_version'",
                [],
                |row| row.get(0),
            )
            .context("schema_version")?;
        assert_eq!(version, SCHEMA_VERSION.to_string());

        let mut stmt = store.conn.prepare("PRAGMA table_info(doc)")?;
        let mut has_meta = false;
        let rows = stmt.query_map([], |row| row.get::<_, String>(1))?;
        for row in rows {
            if row? == "meta" {
                has_meta = true;
                break;
            }
        }
        assert!(has_meta, "doc.meta column missing after migration");
        let table_count: i64 = store
            .conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='ann_hnsw'",
                [],
                |row| row.get(0),
            )
            .context("ann_hnsw table")?;
        assert_eq!(table_count, 1);
        Ok(())
    }
}
