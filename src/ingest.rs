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

use std::path::Path;
use std::path::PathBuf;

use anyhow::Context;
use anyhow::Result;
use globset::Glob;
use globset::GlobSet;
use globset::GlobSetBuilder;
use sha2::Digest;
use sha2::Sha256;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;
use walkdir::WalkDir;

use crate::ann;
use crate::config::Config;
use crate::embed::Embedder;
use crate::embed::HashEmbedder;
use crate::embed::to_bytes;
use crate::store::Store;

#[derive(Debug, Clone)]
pub struct IngestOptions {
    pub glob: Option<String>,
    pub tag: Option<String>,
    pub source: Option<String>,
    pub mtime_only: bool,
    pub ignore: Vec<String>,
}

#[derive(Debug)]
pub struct IngestReport {
    pub docs_added: usize,
    pub chunks_added: usize,
    pub warnings: Vec<String>,
}

impl IngestReport {
    pub fn stats(&self) -> crate::output::StatsOut {
        crate::output::StatsOut {
            took_ms: 0,
            total_hits: self.docs_added as i64,
            doc_count: None,
            chunk_count: None,
            db_size_bytes: None,
            snapshot: None,
        }
    }
}

pub fn ingest_paths(
    store: &Store,
    config: &Config,
    paths: Vec<PathBuf>,
    opts: IngestOptions,
) -> Result<IngestReport> {
    let include_set = build_globset(opts.glob.as_deref())?;
    let ignore_set = build_ignore_set(&opts.ignore)?;

    let embedder = HashEmbedder::new(config.embedding_dim);

    let mut report = IngestReport {
        docs_added: 0,
        chunks_added: 0,
        warnings: Vec::new(),
    };

    for path in resolve_paths(paths) {
        if path.is_file() {
            ingest_file(
                store,
                config,
                &embedder,
                &path,
                &include_set,
                &ignore_set,
                &opts,
                &mut report,
            )?;
        } else if path.is_dir() {
            for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
                if entry.file_type().is_file() {
                    ingest_file(
                        store,
                        config,
                        &embedder,
                        entry.path(),
                        &include_set,
                        &ignore_set,
                        &opts,
                        &mut report,
                    )?;
                }
            }
        }
    }

    Ok(report)
}

fn resolve_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    if paths.is_empty() {
        vec![PathBuf::from(".")]
    } else {
        paths
    }
}

fn build_globset(pattern: Option<&str>) -> Result<Option<GlobSet>> {
    if let Some(pat) = pattern {
        let mut builder = GlobSetBuilder::new();
        builder.add(Glob::new(pat)?);
        let set = builder.build()?;
        Ok(Some(set))
    } else {
        Ok(None)
    }
}

fn build_ignore_set(patterns: &[String]) -> Result<GlobSet> {
    let mut builder = GlobSetBuilder::new();
    for pat in patterns {
        builder.add(Glob::new(pat)?);
    }
    Ok(builder.build()?)
}

#[allow(clippy::too_many_arguments)]
fn ingest_file(
    store: &Store,
    config: &Config,
    embedder: &HashEmbedder,
    path: &Path,
    include_set: &Option<GlobSet>,
    ignore_set: &GlobSet,
    opts: &IngestOptions,
    report: &mut IngestReport,
) -> Result<()> {
    let path_str = path.to_string_lossy();
    if !ignore_set.is_empty() && ignore_set.is_match(path) {
        return Ok(());
    }
    if let Some(set) = include_set
        && !set.is_match(path)
    {
        return Ok(());
    }

    let metadata = std::fs::metadata(path).with_context(|| format!("metadata {}", path_str))?;
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|t| OffsetDateTime::from(t).format(&Rfc3339).ok());
    let mtime_str = mtime.unwrap_or_else(|| "".to_string());

    if opts.mtime_only {
        let mut stmt = store.conn.prepare(
            "SELECT mtime FROM doc WHERE path = ?1 AND deleted = 0 ORDER BY rowid DESC LIMIT 1",
        )?;
        if let Ok(existing) = stmt.query_row([path_str.as_ref()], |row| row.get::<_, String>(0))
            && existing == mtime_str
        {
            return Ok(());
        }
    }

    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(_) => {
            report
                .warnings
                .push(format!("skip non-text file: {}", path_str));
            return Ok(());
        }
    };

    let content_hash = sha256_hex(text.as_bytes());
    let doc_id = sha256_hex(format!("{}\0{}", path_str, content_hash).as_bytes());

    store.mark_doc_deleted(&path_str)?;

    store.conn.execute(
        "INSERT INTO doc (id, path, mtime, size, hash, tag, source, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0)",
        rusqlite::params![
            doc_id,
            path_str.as_ref(),
            mtime_str,
            metadata.len() as i64,
            content_hash,
            opts.tag.as_deref(),
            opts.source.as_deref(),
        ],
    )?;
    report.docs_added += 1;

    let tokens: Vec<&str> = text.split_whitespace().collect();
    let chunk_size = config.chunk_tokens.max(1);
    let overlap = config.overlap_tokens.min(chunk_size.saturating_sub(1));

    let mut start = 0usize;
    while start < tokens.len() {
        let end = usize::min(start + chunk_size, tokens.len());
        let chunk_tokens = &tokens[start..end];
        let chunk_text = chunk_tokens.join(" ");
        let chunk_id = sha256_hex(format!("{}:{}", doc_id, start).as_bytes());
        let embedding = embedder.embed(&chunk_text);
        let embedding_bytes = to_bytes(&embedding);
        let ann_sig = ann::signature(&embedding, config.ann_bits, config.ann_seed);

        store.conn.execute(
            "INSERT INTO chunk (id, doc_id, offset, tokens, text, embedding, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
            rusqlite::params![
                chunk_id,
                doc_id,
                start as i64,
                (end - start) as i64,
                chunk_text,
                embedding_bytes,
            ],
        )?;
        store.conn.execute(
            "INSERT INTO ann_lsh (signature, chunk_id, doc_id) VALUES (?1, ?2, ?3)",
            rusqlite::params![ann_sig as i64, chunk_id, doc_id],
        )?;
        report.chunks_added += 1;

        if end == tokens.len() {
            break;
        }
        start = end - overlap;
    }

    Ok(())
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let digest = hasher.finalize();
    hex::encode(digest)
}
