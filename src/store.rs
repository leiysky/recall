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
use rusqlite::params;

pub struct Store {
    pub conn: Connection,
    pub path: PathBuf,
    _lock: Option<File>,
}

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

impl Store {
    pub fn init(path: &Path) -> Result<()> {
        if path.exists() {
            anyhow::bail!("store already exists at {}", path.display());
        }
        let _lock = Self::acquire_lock(path, StoreMode::ReadWrite)?;
        let conn = Self::open_connection(path, StoreMode::ReadWrite)?;
        Self::apply_pragmas(&conn, StoreMode::ReadWrite)?;
        Self::create_schema(&conn)?;
        Ok(())
    }

    pub fn open(path: &Path, mode: StoreMode) -> Result<Self> {
        let lock = Self::acquire_lock(path, mode)?;
        let conn = Self::open_connection(path, mode)?;
        Self::apply_pragmas(&conn, mode)?;
        if matches!(mode, StoreMode::ReadWrite) {
            Self::create_schema(&conn)?;
        }
        Ok(Self {
            conn,
            path: path.to_path_buf(),
            _lock: Some(lock),
        })
    }

    fn open_connection(path: &Path, mode: StoreMode) -> Result<Connection> {
        let flags = match mode {
            StoreMode::ReadOnly => OpenFlags::SQLITE_OPEN_READ_ONLY,
            StoreMode::ReadWrite => OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE,
        };
        let conn =
            Connection::open_with_flags(path, flags).with_context(|| format!("open {}", path.display()))?;
        conn.busy_timeout(Duration::from_millis(5000))
            .context("set busy timeout")?;
        Ok(conn)
    }

    fn apply_pragmas(conn: &Connection, mode: StoreMode) -> Result<()> {
        let mut batch = String::from("PRAGMA foreign_keys=ON;");
        if matches!(mode, StoreMode::ReadWrite) {
            batch = format!(
                "PRAGMA journal_mode=DELETE;\nPRAGMA synchronous=NORMAL;\n{batch}"
            );
        }
        conn.execute_batch(&batch).context("apply pragmas")?;
        Ok(())
    }

    fn acquire_lock(path: &Path, mode: StoreMode) -> Result<File> {
        let lock_path = path.with_extension("lock");
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .open(&lock_path)
            .with_context(|| format!("open lock file {}", lock_path.display()))?;
        let deadline = Instant::now() + Duration::from_millis(5000);
        loop {
            let locked = match mode {
                StoreMode::ReadOnly => file.try_lock_shared().map_err(|err| err.to_string()),
                StoreMode::ReadWrite => file
                    .try_lock_exclusive()
                    .map_err(|err| err.to_string()),
            };
            match locked {
                Ok(()) => return Ok(file),
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
            "CREATE TABLE IF NOT EXISTS doc (\n  id TEXT PRIMARY KEY,\n  path TEXT,\n  mtime TEXT,\n  size INTEGER,\n  hash TEXT,\n  tag TEXT,\n  source TEXT,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE TABLE IF NOT EXISTS chunk (\n  rowid INTEGER PRIMARY KEY,\n  id TEXT UNIQUE,\n  doc_id TEXT,\n  offset INTEGER,\n  tokens INTEGER,\n  text TEXT,\n  embedding BLOB,\n  deleted INTEGER DEFAULT 0\n);\n\nCREATE INDEX IF NOT EXISTS idx_doc_path ON doc(path);\nCREATE INDEX IF NOT EXISTS idx_doc_tag ON doc(tag);\nCREATE INDEX IF NOT EXISTS idx_chunk_doc ON chunk(doc_id);\n\nCREATE VIRTUAL TABLE IF NOT EXISTS chunk_fts USING fts5(text, content='chunk', content_rowid='rowid');\n\nCREATE TABLE IF NOT EXISTS ann_lsh (\n  signature INTEGER,\n  chunk_id TEXT,\n  doc_id TEXT\n);\n\nCREATE INDEX IF NOT EXISTS idx_ann_sig ON ann_lsh(signature);\nCREATE INDEX IF NOT EXISTS idx_ann_doc ON ann_lsh(doc_id);\n\nCREATE TRIGGER IF NOT EXISTS chunk_ai AFTER INSERT ON chunk BEGIN\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;\n\nCREATE TRIGGER IF NOT EXISTS chunk_ad AFTER DELETE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\nEND;\n\nCREATE TRIGGER IF NOT EXISTS chunk_au AFTER UPDATE ON chunk BEGIN\n  INSERT INTO chunk_fts(chunk_fts, rowid, text) VALUES('delete', old.rowid, old.text);\n  INSERT INTO chunk_fts(rowid, text) VALUES (new.rowid, new.text);\nEND;",
        )
        .context("create schema")?;
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
        }
        Ok(updated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

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
}
