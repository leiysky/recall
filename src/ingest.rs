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

use std::collections::BTreeMap;
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
    pub parser: ParserHint,
    pub extract_meta: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParserHint {
    Auto,
    Plain,
    Markdown,
    Code,
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
            timings: None,
            corpus: None,
            memory: None,
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

    if config.ann_backend == "hnsw" {
        store.rebuild_ann_hnsw()?;
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

    let parser = resolve_parser(path, opts.parser);
    let meta_json = if opts.extract_meta && parser == ParserHint::Markdown {
        extract_metadata(&text)?
    } else {
        None
    };

    let content_hash = sha256_hex(text.as_bytes());
    let doc_id = sha256_hex(format!("{}\0{}", path_str, content_hash).as_bytes());

    store.mark_doc_deleted(&path_str)?;

    store.conn.execute(
        "INSERT INTO doc (id, path, mtime, size, hash, tag, source, meta, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, 0)",
        rusqlite::params![
            doc_id,
            path_str.as_ref(),
            mtime_str,
            metadata.len() as i64,
            content_hash,
            opts.tag.as_deref(),
            opts.source.as_deref(),
            meta_json.as_deref(),
        ],
    )?;
    report.docs_added += 1;

    let chunk_size = config.chunk_tokens.max(1);
    let overlap = config.overlap_tokens.min(chunk_size.saturating_sub(1));
    let blocks = split_blocks(&text, parser);
    let mut global_offset = 0usize;
    for block in blocks {
        let tokens: Vec<&str> = block.split_whitespace().collect();
        if tokens.is_empty() {
            continue;
        }
        let mut start = 0usize;
        while start < tokens.len() {
            let end = usize::min(start + chunk_size, tokens.len());
            let chunk_tokens = &tokens[start..end];
            let chunk_text = chunk_tokens.join(" ");
            let offset = global_offset + start;
            let chunk_id = sha256_hex(format!("{}:{}", doc_id, offset).as_bytes());
            let embedding = embedder.embed(&chunk_text);
            let embedding_bytes = to_bytes(&embedding);
            let ann_sig = ann::signature(&embedding, config.ann_bits, config.ann_seed);

            store.conn.execute(
                "INSERT INTO chunk (id, doc_id, offset, tokens, text, embedding, deleted) VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)",
                rusqlite::params![
                    chunk_id,
                    doc_id,
                    offset as i64,
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
        global_offset += tokens.len();
    }

    Ok(())
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    let digest = hasher.finalize();
    hex::encode(digest)
}

fn resolve_parser(path: &Path, hint: ParserHint) -> ParserHint {
    if hint != ParserHint::Auto {
        return hint;
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    match ext.as_str() {
        "md" | "markdown" | "mdx" => ParserHint::Markdown,
        "rs" | "py" | "js" | "ts" | "go" | "java" | "c" | "cc" | "cpp" | "h" | "hpp"
        | "cs" | "rb" | "php" | "swift" | "kt" | "scala" | "sh" | "bash" | "zsh"
        | "toml" | "yaml" | "yml" | "json" => ParserHint::Code,
        _ => ParserHint::Plain,
    }
}

fn split_blocks(text: &str, parser: ParserHint) -> Vec<String> {
    match parser {
        ParserHint::Markdown => markdown_blocks(text),
        ParserHint::Code => code_blocks(text),
        _ => vec![text.to_string()],
    }
}

fn markdown_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Vec<String> = Vec::new();
    let mut in_code = false;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.starts_with("```") {
            if in_code {
                current.push(trimmed.to_string());
                blocks.push(current.join("\n"));
                current.clear();
                in_code = false;
            } else {
                if !current.is_empty() {
                    blocks.push(current.join("\n"));
                    current.clear();
                }
                current.push(trimmed.to_string());
                in_code = true;
            }
            continue;
        }
        if !in_code && trimmed.starts_with('#') {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
            current.push(trimmed.to_string());
            continue;
        }
        current.push(line.to_string());
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

fn code_blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Vec<String> = Vec::new();
    for line in text.lines() {
        let trimmed = line.trim_start();
        let is_boundary = trimmed.starts_with("fn ")
            || trimmed.starts_with("def ")
            || trimmed.starts_with("class ")
            || trimmed.starts_with("struct ")
            || trimmed.starts_with("enum ")
            || trimmed.starts_with("impl ")
            || trimmed.starts_with("interface ");
        if is_boundary && !current.is_empty() {
            blocks.push(current.join("\n"));
            current.clear();
        }
        if trimmed.is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
            continue;
        }
        current.push(line.to_string());
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

fn extract_metadata(text: &str) -> Result<Option<String>> {
    let mut map = BTreeMap::new();
    let mut lines = text.lines();
    if let Some(first) = lines.next() {
        if first.trim() == "---" {
            for line in lines.by_ref() {
                let trimmed = line.trim();
                if trimmed == "---" || trimmed == "..." {
                    break;
                }
                if let Some((k, v)) = line.split_once(':') {
                    if let Some(key) = normalize_meta_key(k) {
                        map.insert(key, v.trim().to_string());
                    }
                }
            }
            if !map.is_empty() {
                let json = serde_json::to_string(&map).context("serialize metadata")?;
                return Ok(Some(json));
            }
        }
    }

    let mut iter = text.lines().peekable();
    while matches!(iter.peek(), Some(line) if line.trim().is_empty()) {
        iter.next();
    }
    if matches!(iter.peek(), Some(line) if line.trim_start().starts_with('#')) {
        iter.next();
    }
    while matches!(iter.peek(), Some(line) if line.trim().is_empty()) {
        iter.next();
    }
    for line in iter {
        if line.trim().is_empty() {
            break;
        }
        let Some((k, v)) = line.split_once(':') else {
            break;
        };
        if let Some(key) = normalize_meta_key(k) {
            map.insert(key, v.trim().to_string());
        }
    }

    if map.is_empty() {
        Ok(None)
    } else {
        let json = serde_json::to_string(&map).context("serialize metadata")?;
        Ok(Some(json))
    }
}

fn normalize_meta_key(raw: &str) -> Option<String> {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if ch == ' ' || ch == '_' || ch == '-' {
            if !out.ends_with('_') {
                out.push('_');
            }
        }
    }
    let cleaned = out.trim_matches('_').to_string();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_blocks_respect_headings_and_code_fences() {
        let text = "# Title\npara one\n\n## Subtitle\nmore text\n```rs\nfn main() {}\n```\nend";
        let blocks = markdown_blocks(text);
        assert!(blocks.len() >= 3);
        assert!(blocks[0].starts_with("# Title"));
        assert!(blocks[1].starts_with("## Subtitle"));
        assert!(blocks.iter().any(|b| b.contains("```rs")));
    }

    #[test]
    fn code_blocks_split_on_defs() {
        let text = "fn a() {}\n\nfn b() {}\n";
        let blocks = code_blocks(text);
        assert!(blocks.len() >= 2);
        assert!(blocks[0].contains("fn a"));
        assert!(blocks.iter().any(|b| b.contains("fn b")));
    }

    #[test]
    fn extract_metadata_from_issue_header() -> Result<()> {
        let text = "# ISSUE-1\n\nStatus: active\nMilestone: M1\nOwner:\n\nContext:\n- note\n";
        let meta = extract_metadata(text)?.expect("meta");
        let value: serde_json::Value = serde_json::from_str(&meta)?;
        assert_eq!(value.get("status").and_then(|v| v.as_str()), Some("active"));
        assert_eq!(value.get("milestone").and_then(|v| v.as_str()), Some("M1"));
        Ok(())
    }
}
