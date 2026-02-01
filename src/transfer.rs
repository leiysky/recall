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

use std::io::BufRead;
use std::io::BufReader;
use std::io::Write;

use anyhow::Context;
use anyhow::Result;
use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde::Deserialize;
use serde::Serialize;

use crate::ann;
use crate::config::Config;
use crate::embed::from_bytes;
use crate::store::Store;

#[derive(Debug, Serialize)]
struct ExportDoc {
    r#type: String,
    id: String,
    path: String,
    mtime: String,
    size: i64,
    hash: String,
    tag: Option<String>,
    source: Option<String>,
}

#[derive(Debug, Serialize)]
struct ExportChunk {
    r#type: String,
    id: String,
    doc_id: String,
    offset: i64,
    tokens: i64,
    text: String,
    embedding: String,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ImportLine {
    #[serde(rename = "doc")]
    Doc {
        id: String,
        path: String,
        mtime: String,
        size: i64,
        hash: String,
        tag: Option<String>,
        source: Option<String>,
    },
    #[serde(rename = "chunk")]
    Chunk {
        id: String,
        doc_id: String,
        offset: i64,
        tokens: i64,
        text: String,
        embedding: String,
    },
}

#[derive(Debug)]
pub struct TransferStats {
    pub docs: usize,
    pub chunks: usize,
}

pub fn export_store(store: &Store, mut writer: impl Write) -> Result<TransferStats> {
    let mut docs = 0usize;
    let mut chunks = 0usize;

    let mut stmt = store
        .conn
        .prepare("SELECT id, path, mtime, size, hash, tag, source FROM doc WHERE deleted=0")?;
    let rows = stmt.query_map([], |row| {
        Ok(ExportDoc {
            r#type: "doc".to_string(),
            id: row.get(0)?,
            path: row.get(1)?,
            mtime: row.get(2)?,
            size: row.get(3)?,
            hash: row.get(4)?,
            tag: row.get(5)?,
            source: row.get(6)?,
        })
    })?;
    for row in rows {
        let doc = row?;
        let line = serde_json::to_string(&doc)?;
        writeln!(writer, "{}", line)?;
        docs += 1;
    }

    let mut stmt = store
        .conn
        .prepare("SELECT id, doc_id, offset, tokens, text, embedding FROM chunk WHERE deleted=0")?;
    let rows = stmt.query_map([], |row| {
        let embedding: Vec<u8> = row.get(5)?;
        Ok(ExportChunk {
            r#type: "chunk".to_string(),
            id: row.get(0)?,
            doc_id: row.get(1)?,
            offset: row.get(2)?,
            tokens: row.get(3)?,
            text: row.get(4)?,
            embedding: BASE64.encode(embedding),
        })
    })?;
    for row in rows {
        let chunk = row?;
        let line = serde_json::to_string(&chunk)?;
        writeln!(writer, "{}", line)?;
        chunks += 1;
    }

    Ok(TransferStats { docs, chunks })
}

pub fn import_store(
    store: &Store,
    config: &Config,
    reader: impl std::io::Read,
) -> Result<TransferStats> {
    let mut docs = 0usize;
    let mut chunks = 0usize;
    let mut buf = BufReader::new(reader);

    store.conn.execute_batch("BEGIN IMMEDIATE")?;
    let res = (|| -> Result<()> {
        let mut line = String::new();
        loop {
            line.clear();
            let bytes = buf.read_line(&mut line)?;
            if bytes == 0 {
                break;
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let parsed: ImportLine = serde_json::from_str(trimmed)
                .with_context(|| format!("parse import line: {}", trimmed))?;
            match parsed {
                ImportLine::Doc {
                    id,
                    path,
                    mtime,
                    size,
                    hash,
                    tag,
                    source,
                } => {
                    store.conn.execute(
                        "INSERT OR REPLACE INTO doc (id, path, mtime, size, hash, tag, source, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
                        rusqlite::params![id, path, mtime, size, hash, tag, source],
                    )?;
                    docs += 1;
                }
                ImportLine::Chunk {
                    id,
                    doc_id,
                    offset,
                    tokens,
                    text,
                    embedding,
                } => {
                    let bytes = BASE64.decode(embedding.as_bytes())?;
                    store.conn.execute(
                        "INSERT OR REPLACE INTO chunk (id, doc_id, offset, tokens, text, embedding, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                        rusqlite::params![id, doc_id, offset, tokens, text, bytes],
                    )?;
                    chunks += 1;
                }
            }
        }
        Ok(())
    })();

    if res.is_err() {
        store.conn.execute_batch("ROLLBACK")?;
        return res.map(|_| TransferStats { docs, chunks });
    }

    store.conn.execute_batch("COMMIT")?;
    rebuild_ann_lsh(store, config)?;

    Ok(TransferStats { docs, chunks })
}

fn rebuild_ann_lsh(store: &Store, config: &Config) -> Result<()> {
    store.conn.execute("DELETE FROM ann_lsh", [])?;
    let mut stmt = store
        .conn
        .prepare("SELECT id, doc_id, embedding FROM chunk WHERE deleted=0")?;
    let rows = stmt.query_map([], |row| {
        let id: String = row.get(0)?;
        let doc_id: String = row.get(1)?;
        let embedding: Vec<u8> = row.get(2)?;
        Ok((id, doc_id, embedding))
    })?;
    for row in rows {
        let (id, doc_id, embedding) = row?;
        let vec = from_bytes(&embedding);
        let sig = ann::signature(&vec, config.ann_bits, config.ann_seed);
        store.conn.execute(
            "INSERT INTO ann_lsh (signature, chunk_id, doc_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![sig as i64, id, doc_id],
        )?;
    }
    Ok(())
}
