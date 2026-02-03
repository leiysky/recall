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

use std::fs;
use std::fs::File;
use std::fs::OpenOptions;
use std::os::raw::c_char;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Once;
use std::thread::sleep;
use std::time::Duration;
use std::time::Instant;

use anyhow::Context;
use anyhow::Result;
use fs2::FileExt;
use rusqlite::Connection;
use rusqlite::OpenFlags;
use rusqlite::OptionalExtension;
use rusqlite::ffi::sqlite3_auto_extension;
use rusqlite::params;
use sha2::Digest;
use sha2::Sha256;
use sqlite_vec::sqlite3_vec_init;

use crate::config::Config;
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

fn register_sqlite_vec() {
    type SqliteVecInit = unsafe extern "C" fn(
        *mut rusqlite::ffi::sqlite3,
        *mut *const c_char,
        *const rusqlite::ffi::sqlite3_api_routines,
    ) -> i32;

    static INIT: Once = Once::new();
    INIT.call_once(|| unsafe {
        let init = std::mem::transmute::<*const (), SqliteVecInit>(sqlite3_vec_init as *const ());
        sqlite3_auto_extension(Some(init));
    });
}

const SCHEMA_VERSION: i64 = 2;
const VEC_VERSION: &str = "vec0-v1";
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
    pub vec_count: i64,
    pub vec_missing: i64,
}

impl ConsistencyReport {
    pub fn fts_ok(&self) -> bool {
        self.chunk_count == self.fts_count && self.fts_missing == 0
    }

    pub fn vec_ok(&self) -> bool {
        self.chunk_count == self.vec_count && self.vec_missing == 0
    }
}

impl Store {
    pub fn init(path: &Path, config: &Config) -> Result<()> {
        if path.exists() {
            anyhow::bail!("store already exists at {}", path.display());
        }
        let embedding_dim = config.embedding_dim.max(1);
        let _lock = Self::acquire_lock(path, StoreMode::ReadWrite)?;
        let conn = Self::open_connection(path, StoreMode::ReadWrite)?;
        Self::apply_pragmas(&conn, StoreMode::ReadWrite)?;
        Self::create_schema(&conn, embedding_dim)?;
        Self::set_meta(&conn, "schema_version", &SCHEMA_VERSION.to_string())?;
        Self::set_meta(&conn, "vec_version", VEC_VERSION)?;
        Self::set_meta(&conn, "embedding_dim", &embedding_dim.to_string())?;
        Self::set_meta(&conn, "fts_version", FTS_VERSION)?;
        Ok(())
    }

    pub fn open(path: &Path, mode: StoreMode, config: &Config) -> Result<Self> {
        let lock = Self::acquire_lock(path, mode)?;
        let conn = Self::open_connection(path, mode)?;
        Self::apply_pragmas(&conn, mode)?;

        let version = Self::schema_version(&conn)?;
        if version != SCHEMA_VERSION {
            anyhow::bail!(
                "store schema version {} unsupported; re-init + re-ingest required",
                version
            );
        }

        let embedding_dim = config.embedding_dim.max(1);
        if matches!(mode, StoreMode::ReadWrite) {
            Self::create_schema(&conn, embedding_dim)?;
        }
        Self::validate_embedding_dim(&conn, embedding_dim)?;

        Ok(Self {
            conn,
            path: path.to_path_buf(),
            lock: Some(lock),
        })
    }

    fn open_connection(path: &Path, mode: StoreMode) -> Result<Connection> {
        register_sqlite_vec();
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

    fn create_schema(conn: &Connection, embedding_dim: usize) -> Result<()> {
        let dim = embedding_dim.max(1);
        let sql = format!(
            "CREATE TABLE IF NOT EXISTS meta (\n  key TEXT PRIMARY KEY,\n  value TEXT\n);\n\nCREATE TABLE IF NOT EXISTS doc (\n  id TEXT PRIMARY KEY,\n  path TEXT,\n  mtime TEXT,\n  size INTEGER,\n  hash TEXT,\n  tag TEXT,\n  source TEXT,\n  meta TEXT,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE TABLE IF NOT EXISTS chunk (\n  rowid INTEGER PRIMARY KEY,\n  id TEXT UNIQUE,\n  doc_id TEXT,\n  offset INTEGER,\n  tokens INTEGER,\n  text TEXT,\n  embedding BLOB,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE INDEX IF NOT EXISTS idx_doc_path ON doc(path);\nCREATE INDEX IF NOT EXISTS idx_doc_tag ON doc(tag);\nCREATE INDEX IF NOT EXISTS idx_chunk_doc ON chunk(doc_id);\n\nCREATE VIRTUAL TABLE IF NOT EXISTS chunk_fts USING fts5(text, content='chunk', content_rowid='rowid');\n\nCREATE VIRTUAL TABLE IF NOT EXISTS chunk_vec USING vec0(\n  chunk_rowid INTEGER PRIMARY KEY,\n  embedding FLOAT[{dim}] distance_metric=cosine\n);\n\nCREATE TRIGGER IF NOT EXISTS chunk_ai AFTER INSERT ON chunk BEGIN\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;\n\nCREATE TRIGGER IF NOT EXISTS chunk_ad AFTER DELETE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\nEND;\n\nCREATE TRIGGER IF NOT EXISTS chunk_au AFTER UPDATE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;"
        );
        conn.execute_batch(&sql).context("create schema")?;
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

    fn embedding_dim_meta(conn: &Connection) -> Result<Option<usize>> {
        let value: Option<String> = conn
            .query_row(
                "SELECT value FROM meta WHERE key='embedding_dim'",
                [],
                |row| row.get(0),
            )
            .optional()
            .context("read embedding_dim")?;
        Ok(value.and_then(|v| v.parse::<usize>().ok()))
    }

    fn validate_embedding_dim(conn: &Connection, embedding_dim: usize) -> Result<()> {
        let Some(stored) = Self::embedding_dim_meta(conn)? else {
            anyhow::bail!("store embedding_dim metadata missing; re-init + re-ingest required");
        };
        if stored != embedding_dim {
            anyhow::bail!(
                "config embedding_dim {} does not match store embedding_dim {}",
                embedding_dim,
                stored
            );
        }
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
        let vec_count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM chunk_vec", [], |row| row.get(0))
            .context("count chunk_vec")?;
        let vec_missing: i64 = self
            .conn
            .query_row(
                "SELECT COUNT(*)\n                 FROM chunk\n                 LEFT JOIN chunk_vec ON chunk_vec.chunk_rowid = chunk.rowid\n                 WHERE chunk.deleted=0 AND chunk_vec.chunk_rowid IS NULL",
                [],
                |row| row.get(0),
            )
            .context("chunk_vec missing")?;
        Ok(ConsistencyReport {
            chunk_count,
            fts_count,
            fts_missing,
            vec_count,
            vec_missing,
        })
    }

    pub fn rebuild_fts(&self) -> Result<()> {
        self.conn
            .execute("INSERT INTO chunk_fts(chunk_fts) VALUES('rebuild')", [])
            .context("rebuild fts")?;
        Ok(())
    }

    pub fn rebuild_vec(&self) -> Result<usize> {
        self.conn
            .execute("DELETE FROM chunk_vec", [])
            .context("clear chunk_vec")?;
        let mut stmt = self
            .conn
            .prepare("SELECT rowid, embedding FROM chunk WHERE deleted=0")?;
        let rows = stmt.query_map([], |row| {
            let rowid: i64 = row.get(0)?;
            let embedding: Vec<u8> = row.get(1)?;
            Ok((rowid, embedding))
        })?;
        let mut inserted = 0usize;
        for row in rows {
            let (rowid, embedding) = row?;
            self.conn.execute(
                "INSERT INTO chunk_vec (chunk_rowid, embedding) VALUES (?1, ?2)",
                params![rowid, embedding],
            )?;
            inserted += 1;
        }
        Ok(inserted)
    }

    pub fn compact(&self) -> Result<()> {
        self.conn.execute("DELETE FROM chunk WHERE deleted=1", [])?;
        self.conn.execute("DELETE FROM doc WHERE deleted=1", [])?;
        self.conn.execute(
            "DELETE FROM chunk_vec WHERE chunk_rowid NOT IN (SELECT rowid FROM chunk)",
            [],
        )?;
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
                self.conn.execute(
                    "DELETE FROM chunk_vec WHERE chunk_rowid IN (SELECT rowid FROM chunk WHERE doc_id = ?1)",
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
            self.conn.execute(
                "DELETE FROM chunk_vec WHERE chunk_rowid IN (SELECT rowid FROM chunk WHERE doc_id = ?1)",
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
    use crate::config::Config;

    #[test]
    fn shared_lock_allows_multiple_readers() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("recall.db");
        let config = Config::default();
        Store::init(&db_path, &config)?;

        let store_a = Store::open(&db_path, StoreMode::ReadOnly, &config)?;
        let store_b = Store::open(&db_path, StoreMode::ReadOnly, &config)?;

        store_a.stats()?;
        store_b.stats()?;
        Ok(())
    }

    #[test]
    fn rejects_unversioned_store() -> Result<()> {
        let dir = tempdir()?;
        let db_path = dir.path().join("recall.db");
        let conn = SqlConnection::open(&db_path)?;
        conn.execute_batch(
            "CREATE TABLE doc (\n  id TEXT PRIMARY KEY,\n  path TEXT,\n  mtime TEXT,\n  size INTEGER,\n  hash TEXT,\n  tag TEXT,\n  source TEXT,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE TABLE chunk (\n  rowid INTEGER PRIMARY KEY,\n  id TEXT UNIQUE,\n  doc_id TEXT,\n  offset INTEGER,\n  tokens INTEGER,\n  text TEXT,\n  embedding BLOB,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE INDEX idx_doc_path ON doc(path);\nCREATE INDEX idx_doc_tag ON doc(tag);\nCREATE INDEX idx_chunk_doc ON chunk(doc_id);\n\nCREATE VIRTUAL TABLE chunk_fts USING fts5(text, content='chunk', content_rowid='rowid');\n\nCREATE TRIGGER chunk_ai AFTER INSERT ON chunk BEGIN\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;\n\nCREATE TRIGGER chunk_ad AFTER DELETE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\nEND;\n\nCREATE TRIGGER chunk_au AFTER UPDATE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;",
        )?;
        drop(conn);

        let config = Config::default();
        let err = match Store::open(&db_path, StoreMode::ReadOnly, &config) {
            Ok(_) => anyhow::bail!("expected unsupported schema error"),
            Err(err) => err,
        };
        assert!(
            err.to_string().contains("schema version 0 unsupported"),
            "unexpected error: {err}"
        );
        Ok(())
    }
}
