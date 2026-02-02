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
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;
use rusqlite::OptionalExtension;
use rusqlite::Row;
use rusqlite::params;
use rusqlite::params_from_iter;
use rusqlite::types::Value as SqlValue;
use serde_json::json;

use crate::ann;
use crate::config::Config;
use crate::embed::Embedder;
use crate::embed::HashEmbedder;
use crate::embed::cosine_similarity;
use crate::embed::from_bytes;
use crate::model::ChunkRow;
use crate::model::DocRow;
use crate::model::ScoredItem;
use crate::model::SearchResult;
use crate::output::StatsOut;
use crate::output::TimingBreakdown;
use crate::rql::CmpOp;
use crate::rql::FieldRef;
use crate::rql::FilterExpr;
use crate::rql::OrderBy;
use crate::rql::OrderDir;
use crate::rql::Predicate;
use crate::rql::RqlQuery;
use crate::rql::SelectField;
use crate::rql::Table;
use crate::rql::Value;
use crate::rql::parse_filter;
use crate::rql::parse_rql;
use crate::store::Store;

#[derive(Debug, Clone)]
pub struct SearchOptions {
    pub k: usize,
    pub use_lexical: bool,
    pub use_semantic: bool,
    pub filter: Option<String>,
    pub explain: bool,
    pub lexical_mode: LexicalMode,
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone)]
pub struct SearchInputs {
    pub semantic: Option<String>,
    pub lexical: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LexicalMode {
    Fts5,
    Literal,
}

#[derive(Debug, Clone)]
struct LexicalRun {
    results: Vec<ScoredItem>,
    warning: Option<String>,
    original: String,
    sanitized: Option<String>,
}

impl LexicalMode {
    pub fn as_label(self) -> &'static str {
        match self {
            LexicalMode::Fts5 => "fts5",
            LexicalMode::Literal => "literal",
        }
    }
}

impl SearchResult {
    pub fn to_json_results(&self) -> Vec<serde_json::Value> {
        let selection =
            Selection::from_fields(self.selected_fields.as_deref(), self.include_explain);
        self.items
            .iter()
            .map(|item| item.to_json(&selection))
            .collect()
    }
}

impl ScoredItem {
    fn to_json(&self, selection: &Selection) -> serde_json::Value {
        let mut obj = serde_json::Map::new();
        if selection.include_score {
            obj.insert("score".into(), serde_json::json!(self.score));
        }
        if selection.include_explain {
            let mut explain = serde_json::Map::new();
            if let Some(v) = self.lexical {
                explain.insert("lexical".into(), serde_json::json!(v));
            }
            if let Some(v) = self.semantic {
                explain.insert("semantic".into(), serde_json::json!(v));
            }
            if !explain.is_empty() {
                obj.insert("explain".into(), serde_json::Value::Object(explain));
            }
        }

        if let Some(doc_fields) = &selection.doc_fields {
            let mut doc = serde_json::Map::new();
            if doc_fields.contains(&"id") {
                doc.insert("id".into(), serde_json::json!(self.doc.id));
            }
            if doc_fields.contains(&"path") {
                doc.insert("path".into(), serde_json::json!(self.doc.path));
            }
            if doc_fields.contains(&"mtime") {
                doc.insert("mtime".into(), serde_json::json!(self.doc.mtime));
            }
            if doc_fields.contains(&"hash") {
                doc.insert("hash".into(), serde_json::json!(self.doc.hash));
            }
            if doc_fields.contains(&"tag") {
                doc.insert("tag".into(), serde_json::json!(self.doc.tag));
            }
            if doc_fields.contains(&"source") {
                doc.insert("source".into(), serde_json::json!(self.doc.source));
            }
            if doc_fields.contains(&"meta") {
                if let Some(meta) = &self.doc.meta {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(meta) {
                        doc.insert("meta".into(), value);
                    } else {
                        doc.insert("meta".into(), serde_json::json!(meta));
                    }
                }
            }
            if !doc.is_empty() {
                obj.insert("doc".into(), serde_json::Value::Object(doc));
            }
        }

        if let (Some(chunk_fields), Some(chunk)) = (&selection.chunk_fields, &self.chunk) {
            let mut ch = serde_json::Map::new();
            if chunk_fields.contains(&"id") {
                ch.insert("id".into(), serde_json::json!(chunk.id));
            }
            if chunk_fields.contains(&"doc_id") {
                ch.insert("doc_id".into(), serde_json::json!(chunk.doc_id));
            }
            if chunk_fields.contains(&"offset") {
                ch.insert("offset".into(), serde_json::json!(chunk.offset));
            }
            if chunk_fields.contains(&"tokens") {
                ch.insert("tokens".into(), serde_json::json!(chunk.tokens));
            }
            if chunk_fields.contains(&"text") {
                ch.insert("text".into(), serde_json::json!(chunk.text));
            }
            if !ch.is_empty() {
                obj.insert("chunk".into(), serde_json::Value::Object(ch));
            }
        }

        serde_json::Value::Object(obj)
    }
}

struct Selection {
    include_score: bool,
    include_explain: bool,
    doc_fields: Option<Vec<&'static str>>,
    chunk_fields: Option<Vec<&'static str>>,
}

impl Selection {
    fn from_fields(fields: Option<&[SelectField]>, include_explain: bool) -> Self {
        if fields.is_none() {
            return Self {
                include_score: true,
                include_explain,
                doc_fields: Some(doc_field_list()),
                chunk_fields: Some(chunk_field_list()),
            };
        }
        let mut include_score = false;
        let mut doc_fields = Vec::new();
        let mut chunk_fields = Vec::new();
        for field in fields.unwrap() {
            match field {
                SelectField::All => {
                    doc_fields = doc_field_list();
                    chunk_fields = chunk_field_list();
                }
                SelectField::Score => include_score = true,
                SelectField::Field(f) => {
                    if let Some(t) = f.table.clone() {
                        match t {
                            Table::Doc => push_unique(&mut doc_fields, f.name.as_str()),
                            Table::Chunk => push_unique(&mut chunk_fields, f.name.as_str()),
                        }
                    } else if is_doc_field(&f.name) {
                        push_unique(&mut doc_fields, f.name.as_str());
                    } else if is_chunk_field(&f.name) {
                        push_unique(&mut chunk_fields, f.name.as_str());
                    }
                }
            }
        }
        let doc_fields = if doc_fields.is_empty() {
            None
        } else {
            Some(doc_fields)
        };
        let chunk_fields = if chunk_fields.is_empty() {
            None
        } else {
            Some(chunk_fields)
        };
        Self {
            include_score,
            include_explain,
            doc_fields,
            chunk_fields,
        }
    }
}

fn push_unique(vec: &mut Vec<&'static str>, name: &str) {
    if let Some(field) = normalize_doc_field(name).or_else(|| normalize_chunk_field(name))
        && !vec.contains(&field)
    {
        vec.push(field);
    }
}

fn doc_field_list() -> Vec<&'static str> {
    vec!["id", "path", "mtime", "hash", "tag", "source", "meta"]
}

fn chunk_field_list() -> Vec<&'static str> {
    vec!["id", "doc_id", "offset", "tokens", "text"]
}

fn normalize_doc_field(name: &str) -> Option<&'static str> {
    match name {
        "id" => Some("id"),
        "path" => Some("path"),
        "mtime" => Some("mtime"),
        "hash" => Some("hash"),
        "tag" => Some("tag"),
        "source" => Some("source"),
        "meta" => Some("meta"),
        _ => None,
    }
}

fn normalize_chunk_field(name: &str) -> Option<&'static str> {
    match name {
        "id" => Some("id"),
        "doc_id" => Some("doc_id"),
        "offset" => Some("offset"),
        "tokens" => Some("tokens"),
        "text" => Some("text"),
        _ => None,
    }
}

fn is_doc_field(name: &str) -> bool {
    normalize_doc_field(name).is_some()
}

fn is_chunk_field(name: &str) -> bool {
    normalize_chunk_field(name).is_some()
}

pub fn search_chunks(
    store: &Store,
    config: &Config,
    query: &str,
    opts: SearchOptions,
) -> Result<SearchResult> {
    let limit = opts.k;
    let inputs = SearchInputs {
        semantic: Some(query.to_string()),
        lexical: Some(query.to_string()),
    };
    search_chunks_with_inputs(store, config, inputs, opts, None, None, limit, 0)
}

pub fn search_chunks_with_inputs(
    store: &Store,
    config: &Config,
    inputs: SearchInputs,
    opts: SearchOptions,
    filter_expr: Option<FilterExpr>,
    selected_fields: Option<Vec<SelectField>>,
    limit: usize,
    offset: usize,
) -> Result<SearchResult> {
    let started = Instant::now();
    let mut explain_warnings = Vec::new();
    let mut timings = TimingBreakdown::default();

    let filter_start = Instant::now();
    let filter_expr = if let Some(expr) = filter_expr {
        Some(expr)
    } else {
        opts.filter.as_deref().map(parse_filter).transpose()?
    };
    let (filter_sql, filter_params) = if let Some(expr) = &filter_expr {
        filter_to_sql(expr)?
    } else {
        ("1=1".to_string(), Vec::new())
    };
    timings.filter_ms = Some(filter_start.elapsed().as_millis() as i64);
    let (filter_sql, filter_params) =
        apply_snapshot_filter(filter_sql, filter_params, &opts.snapshot);

    let mut lexical_results = Vec::new();
    let mut lexical_run: Option<LexicalRun> = None;
    if opts.use_lexical {
        let lex_start = Instant::now();
        if let Some(lex_query) = inputs.lexical.clone() {
            let run = lexical_search(
                store,
                &lex_query,
                &filter_sql,
                &filter_params,
                opts.k,
                opts.lexical_mode,
            )?;
            lexical_results = run.results.clone();
            if let Some(warning) = run.warning.clone() {
                explain_warnings.push(warning);
            }
            lexical_run = Some(run);
        } else {
            explain_warnings
                .push("lexical search requested but no lexical query provided".to_string());
        }
        timings.lexical_ms = Some(lex_start.elapsed().as_millis() as i64);
    }

    let mut semantic_results = Vec::new();
    if opts.use_semantic {
        let sem_start = Instant::now();
        if let Some(sem_query) = inputs.semantic.clone() {
            semantic_results = semantic_search(
                store,
                config,
                &sem_query,
                &filter_sql,
                &filter_params,
                opts.k,
            )?;
        } else {
            explain_warnings
                .push("semantic search requested but no semantic query provided".to_string());
        }
        timings.semantic_ms = Some(sem_start.elapsed().as_millis() as i64);
    }

    let lexical_count = lexical_run
        .as_ref()
        .map(|run| run.results.len())
        .unwrap_or(0);
    let semantic_count = semantic_results.len();
    let combine_start = Instant::now();
    let items = combine_results(config, lexical_results, semantic_results, opts.k);
    timings.combine_ms = Some(combine_start.elapsed().as_millis() as i64);

    let snapshot = opts.snapshot.clone().or_else(|| store.snapshot_token().ok());
    let stats = StatsOut {
        took_ms: started.elapsed().as_millis() as i64,
        total_hits: items.len() as i64,
        doc_count: None,
        chunk_count: None,
        db_size_bytes: None,
        snapshot: snapshot.clone(),
        timings: Some(timings),
        corpus: None,
        memory: None,
    };

    let explain = if opts.explain {
        Some(build_explain_payload(
            config,
            &opts,
            &inputs,
            lexical_run.as_ref(),
            lexical_count,
            semantic_count,
            items.len(),
            snapshot.as_deref(),
        ))
    } else {
        None
    };

    Ok(SearchResult {
        items,
        stats,
        filter: opts.filter,
        explain_warnings,
        explain,
        selected_fields,
        include_explain: opts.explain,
        limit,
        offset,
    })
}

fn apply_snapshot_filter(
    filter_sql: String,
    mut filter_params: Vec<SqlValue>,
    snapshot: &Option<String>,
) -> (String, Vec<SqlValue>) {
    if let Some(token) = snapshot {
        let sql = format!("({}) AND doc.mtime <= ?", filter_sql);
        filter_params.push(SqlValue::from(token.clone()));
        (sql, filter_params)
    } else {
        (filter_sql, filter_params)
    }
}

fn build_explain_payload(
    config: &Config,
    opts: &SearchOptions,
    inputs: &SearchInputs,
    lexical_run: Option<&LexicalRun>,
    lexical_count: usize,
    semantic_count: usize,
    combined_count: usize,
    snapshot: Option<&str>,
) -> serde_json::Value {
    let mode = if opts.use_lexical && opts.use_semantic {
        "hybrid"
    } else if opts.use_lexical {
        "lexical"
    } else if opts.use_semantic {
        "semantic"
    } else {
        "none"
    };

    let mut obj = serde_json::Map::new();
    obj.insert("mode".into(), json!(mode));
    obj.insert(
        "candidates".into(),
        json!({
            "lexical": lexical_count as i64,
            "semantic": semantic_count as i64,
            "combined": combined_count as i64,
        }),
    );
    obj.insert(
        "cache".into(),
        json!({
            "embedding": "none",
            "ann": "none",
            "fts": "none",
        }),
    );
    obj.insert(
        "resolved_config".into(),
        json!({
            "embedding": config.embedding,
            "embedding_dim": config.embedding_dim,
            "ann_backend": config.ann_backend,
            "ann_bits": config.ann_bits,
            "ann_seed": config.ann_seed,
            "bm25_weight": config.bm25_weight,
            "vector_weight": config.vector_weight,
            "max_limit": config.max_limit,
            "chunk_tokens": config.chunk_tokens,
            "overlap_tokens": config.overlap_tokens,
            "lexical_mode": opts.lexical_mode.as_label(),
        }),
    );
    if let Some(token) = snapshot {
        obj.insert("snapshot".into(), json!(token));
    }

    if opts.use_lexical {
        let original = lexical_run
            .map(|run| run.original.clone())
            .or_else(|| inputs.lexical.clone())
            .unwrap_or_default();
        let mut lex = serde_json::Map::new();
        lex.insert("mode".into(), json!(opts.lexical_mode.as_label()));
        lex.insert("original".into(), json!(original));
        if let Some(run) = lexical_run {
            if let Some(sanitized) = &run.sanitized {
                lex.insert("sanitized".into(), json!(sanitized));
                lex.insert("sanitized_applied".into(), json!(true));
            } else {
                lex.insert("sanitized_applied".into(), json!(false));
            }
        }
        obj.insert("lexical".into(), serde_json::Value::Object(lex));
    }

    serde_json::Value::Object(obj)
}

pub fn run_rql(
    store: &Store,
    config: &Config,
    rql_text: &str,
    explain: bool,
    lexical_mode: LexicalMode,
    snapshot: Option<String>,
) -> Result<SearchResult> {
    let q = parse_rql(rql_text)?;

    if q.using_semantic.is_some() || q.using_lexical.is_some() {
        let offset = q.offset.unwrap_or(0);
        let limit = q.limit.unwrap_or(config.max_limit);
        let inputs = SearchInputs {
            semantic: q.using_semantic.clone(),
            lexical: q.using_lexical.clone(),
        };
        let opts = SearchOptions {
            k: limit + offset,
            use_lexical: q.using_lexical.is_some(),
            use_semantic: q.using_semantic.is_some(),
            filter: None,
            explain,
            lexical_mode,
            snapshot: snapshot.clone(),
        };
        let mut result = search_chunks_with_inputs(
            store,
            config,
            inputs,
            opts,
            q.filter.clone(),
            Some(q.fields.clone()),
            limit,
            offset,
        )?;
        if q.table == Table::Doc {
            result = group_by_doc(result, q.fields.clone());
        }
        if let Some((order_by, dir)) = &q.order_by {
            apply_ordering(&mut result.items, order_by.clone(), dir.clone());
        }
        if offset > 0 {
            result.items = result.items.into_iter().skip(offset).take(limit).collect();
        } else {
            result.items.truncate(limit);
        }
        return Ok(result);
    }

    run_structured_query(store, &q, explain, snapshot)
}

fn run_structured_query(
    store: &Store,
    q: &RqlQuery,
    explain: bool,
    snapshot: Option<String>,
) -> Result<SearchResult> {
    let started = Instant::now();
    let mut timings = TimingBreakdown::default();
    let limit = q.limit.unwrap_or(1000);
    let offset = q.offset.unwrap_or(0);
    let filter_start = Instant::now();
    let (filter_sql, filter_params) = if let Some(expr) = &q.filter {
        filter_to_sql(expr)?
    } else {
        ("1=1".to_string(), Vec::new())
    };
    timings.filter_ms = Some(filter_start.elapsed().as_millis() as i64);
    let (filter_sql, filter_params) = apply_snapshot_filter(filter_sql, filter_params, &snapshot);

    let mut items = Vec::new();

    if q.table == Table::Chunk {
        let order_sql = order_clause(q, Table::Chunk)?;
        let sql = format!(
            "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text, doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source, doc.meta\n             FROM chunk JOIN doc ON doc.id = chunk.doc_id\n             WHERE chunk.deleted=0 AND doc.deleted=0 AND ({})\n             {}\n             LIMIT ? OFFSET ?",
            filter_sql, order_sql
        );
        let mut params = filter_params.clone();
        params.push(SqlValue::from(limit as i64));
        params.push(SqlValue::from(offset as i64));
        let mut stmt = store.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), map_chunk_row)?;
        for row in rows {
            let (chunk, doc) = row?;
            items.push(ScoredItem {
                score: 0.0,
                lexical: None,
                semantic: None,
                doc,
                chunk: Some(chunk),
            });
        }
    } else {
        let order_sql = order_clause(q, Table::Doc)?;
        let sql = format!(
            "SELECT doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source, doc.meta\n             FROM doc\n             WHERE doc.deleted=0 AND ({})\n             {}\n             LIMIT ? OFFSET ?",
            filter_sql, order_sql
        );
        let mut params = filter_params.clone();
        params.push(SqlValue::from(limit as i64));
        params.push(SqlValue::from(offset as i64));
        let mut stmt = store.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            Ok(DocRow {
                id: row.get(0)?,
                path: row.get(1)?,
                mtime: row.get(2)?,
                hash: row.get(3)?,
                tag: row.get(4)?,
                source: row.get(5)?,
                meta: row.get(6)?,
            })
        })?;
        for row in rows {
            items.push(ScoredItem {
                score: 0.0,
                lexical: None,
                semantic: None,
                doc: row?,
                chunk: None,
            });
        }
    }

    let stats = StatsOut {
        took_ms: started.elapsed().as_millis() as i64,
        total_hits: items.len() as i64,
        doc_count: None,
        chunk_count: None,
        db_size_bytes: None,
        snapshot: snapshot.clone().or_else(|| store.snapshot_token().ok()),
        timings: Some(timings),
        corpus: None,
        memory: None,
    };

    Ok(SearchResult {
        items,
        stats,
        filter: None,
        explain_warnings: Vec::new(),
        explain: if explain {
            Some(json!({
                "mode": "structured",
                "resolved_config": {
                    "max_limit": q.limit.unwrap_or(1000),
                },
                "snapshot": snapshot,
            }))
        } else {
            None
        },
        selected_fields: Some(q.fields.clone()),
        include_explain: false,
        limit,
        offset,
    })
}

fn lexical_search(
    store: &Store,
    query: &str,
    filter_sql: &str,
    filter_params: &[SqlValue],
    k: usize,
    mode: LexicalMode,
) -> Result<LexicalRun> {
    let run = |query: &str| -> Result<Vec<ScoredItem>> {
        let sql = format!(
            "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text,\n                doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source, doc.meta,\n                bm25(chunk_fts) as bm25\n         FROM chunk_fts\n         JOIN chunk ON chunk_fts.rowid = chunk.rowid\n         JOIN doc ON doc.id = chunk.doc_id\n         WHERE chunk.deleted=0 AND doc.deleted=0 AND ({}) AND chunk_fts MATCH ?\n         ORDER BY bm25 ASC, doc.path ASC, chunk.offset ASC, chunk.id ASC\n         LIMIT ?",
            filter_sql
        );

        let mut params: Vec<SqlValue> = filter_params.to_vec();
        params.push(SqlValue::from(query.to_string()));
        params.push(SqlValue::from(k as i64));

        let mut stmt = store.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            let (chunk, doc) = map_chunk_row(row)?;
            let bm25: f64 = row.get(12)?;
            let score = 1.0 / (1.0 + bm25.max(0.0));
            Ok(ScoredItem {
                score: score as f32,
                lexical: Some(score as f32),
                semantic: None,
                doc,
                chunk: Some(chunk),
            })
        })?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row?);
        }
        Ok(results)
    };

    let original = query.to_string();
    let mut warning = None;
    let mut sanitized = None;
    let mut query_to_run = original.clone();

    if matches!(mode, LexicalMode::Literal) {
        let (safe, changed) = sanitize_fts5_query(&original);
        query_to_run = safe.clone();
        if changed {
            sanitized = Some(safe);
            warning = Some("lexical query sanitized for literal mode".to_string());
        }
        if query_to_run.is_empty() {
            return Ok(LexicalRun {
                results: Vec::new(),
                warning: Some("lexical query sanitized to empty; skipping lexical search".to_string()),
                original,
                sanitized,
            });
        }
    }

    match run(&query_to_run) {
        Ok(results) => Ok(LexicalRun {
            results,
            warning,
            original,
            sanitized,
        }),
        Err(err) => {
            if is_fts5_syntax_error(&err) && matches!(mode, LexicalMode::Fts5) {
                let (safe, changed) = sanitize_fts5_query(&original);
                if !changed {
                    return Err(err);
                }
                sanitized = Some(safe.clone());
                if safe.is_empty() {
                    return Ok(LexicalRun {
                        results: Vec::new(),
                        warning: Some(
                            "lexical query sanitized to empty; skipping lexical search".to_string(),
                        ),
                        original,
                        sanitized,
                    });
                }
                match run(&safe) {
                    Ok(results) => Ok(LexicalRun {
                        results,
                        warning: Some(format!("lexical query sanitized for FTS5: \"{}\"", safe)),
                        original,
                        sanitized,
                    }),
                    Err(_) => Err(err),
                }
            } else {
                Err(err)
            }
        }
    }
}

fn sanitize_fts5_query(query: &str) -> (String, bool) {
    let mut out = String::with_capacity(query.len());
    let mut last_space = true;
    for ch in query.chars() {
        if ch.is_alphanumeric() || ch == '_' {
            out.push(ch);
            last_space = false;
        } else if ch.is_whitespace() {
            if !last_space {
                out.push(' ');
                last_space = true;
            }
        } else if !last_space {
            out.push(' ');
            last_space = true;
        }
    }
    let trimmed = out.trim().to_string();
    let changed = trimmed != query.trim();
    (trimmed, changed)
}

fn is_fts5_syntax_error(err: &anyhow::Error) -> bool {
    err.to_string().contains("fts5: syntax error")
}

fn semantic_search(
    store: &Store,
    config: &Config,
    query: &str,
    filter_sql: &str,
    filter_params: &[SqlValue],
    k: usize,
) -> Result<Vec<ScoredItem>> {
    let embedder = HashEmbedder::new(config.embedding_dim);
    let query_vec = embedder.embed(query);
    let backend = config.ann_backend.as_str();
    if backend.eq_ignore_ascii_case("hnsw") {
        if let Ok(results) =
            semantic_search_hnsw(store, config, &query_vec, filter_sql, filter_params, k)
            && results.len() >= k
        {
            return Ok(results);
        }
        if config.ann_bits > 0
            && let Ok(results) =
                semantic_search_lsh(store, config, &query_vec, filter_sql, filter_params, k)
            && results.len() >= k
        {
            return Ok(results);
        }
    } else if backend.eq_ignore_ascii_case("lsh") || backend.is_empty() {
        if config.ann_bits > 0
            && let Ok(results) =
                semantic_search_lsh(store, config, &query_vec, filter_sql, filter_params, k)
            && results.len() >= k
        {
            return Ok(results);
        }
    } else if backend.eq_ignore_ascii_case("linear") || backend.eq_ignore_ascii_case("flat") {
        // fall through to linear
    } else {
        if config.ann_bits > 0
            && let Ok(results) =
                semantic_search_lsh(store, config, &query_vec, filter_sql, filter_params, k)
            && results.len() >= k
        {
            return Ok(results);
        }
    }

    semantic_search_linear(store, &query_vec, filter_sql, filter_params, k)
}

fn semantic_search_lsh(
    store: &Store,
    config: &Config,
    query_vec: &[f32],
    filter_sql: &str,
    filter_params: &[SqlValue],
    k: usize,
) -> Result<Vec<ScoredItem>> {
    let sig = ann::signature(query_vec, config.ann_bits, config.ann_seed);
    let sigs = ann::neighbor_signatures(sig, config.ann_bits);

    let placeholders = vec!["?"; sigs.len()].join(", ");
    let sql = format!(
        "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text, chunk.embedding,\n                doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source, doc.meta\n         FROM ann_lsh\n         JOIN chunk ON ann_lsh.chunk_id = chunk.id\n         JOIN doc ON doc.id = chunk.doc_id\n         WHERE chunk.deleted=0 AND doc.deleted=0 AND ({}) AND ann_lsh.signature IN ({})",
        filter_sql, placeholders
    );

    let mut params: Vec<SqlValue> = filter_params.to_vec();
    for s in sigs {
        params.push(SqlValue::from(s as i64));
    }

    let mut stmt = store.conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params), |row| {
        let (chunk, doc) = map_chunk_row_with_embedding(row)?;
        Ok((chunk, doc))
    })?;

    let mut scored = Vec::new();
    for row in rows {
        let (chunk, doc) = row?;
        let score = cosine_similarity(query_vec, &chunk.embedding);
        scored.push(ScoredItem {
            score,
            lexical: None,
            semantic: Some(score),
            doc,
            chunk: Some(chunk.into_chunk_row()),
        });
    }

    sort_by_score_with_tiebreak(&mut scored, false);
    scored.truncate(k);
    Ok(scored)
}

fn semantic_search_hnsw(
    store: &Store,
    config: &Config,
    query_vec: &[f32],
    filter_sql: &str,
    filter_params: &[SqlValue],
    k: usize,
) -> Result<Vec<ScoredItem>> {
    if config.ann_bits == 0 {
        return Ok(Vec::new());
    }
    let sig = ann::signature(query_vec, config.ann_bits, config.ann_seed);
    let sigs = ann::neighbor_signatures(sig, config.ann_bits);
    let placeholders = vec!["?"; sigs.len()].join(", ");
    let seed_limit = usize::max(32, k.saturating_mul(4)) as i64;
    let sql = format!(
        "SELECT chunk_id FROM ann_lsh WHERE signature IN ({}) LIMIT {}",
        placeholders, seed_limit
    );
    let mut params: Vec<SqlValue> = Vec::new();
    for s in sigs {
        params.push(SqlValue::from(s as i64));
    }
    let mut stmt = store.conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params), |row| row.get::<_, String>(0))?;
    let mut candidates: BTreeSet<String> = BTreeSet::new();
    let mut seeds = Vec::new();
    for row in rows {
        let id = row?;
        seeds.push(id.clone());
        candidates.insert(id);
    }
    if seeds.is_empty() {
        return Ok(Vec::new());
    }

    for id in &seeds {
        let neighbors: Option<String> = store
            .conn
            .query_row(
                "SELECT neighbors FROM ann_hnsw WHERE chunk_id = ?1",
                params![id],
                |row| row.get(0),
            )
            .optional()?;
        if let Some(raw) = neighbors {
            if let Ok(list) = serde_json::from_str::<Vec<String>>(&raw) {
                for neighbor in list {
                    candidates.insert(neighbor);
                }
            }
        }
    }

    if candidates.is_empty() {
        return Ok(Vec::new());
    }

    let placeholders = vec!["?"; candidates.len()].join(", ");
    let sql = format!(
        "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text, chunk.embedding,\n                doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source, doc.meta\n         FROM chunk\n         JOIN doc ON doc.id = chunk.doc_id\n         WHERE chunk.deleted=0 AND doc.deleted=0 AND ({}) AND chunk.id IN ({})",
        filter_sql, placeholders
    );
    let mut params: Vec<SqlValue> = filter_params.to_vec();
    for id in candidates {
        params.push(SqlValue::from(id));
    }
    let mut stmt = store.conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(params), |row| {
        let (chunk, doc) = map_chunk_row_with_embedding(row)?;
        Ok((chunk, doc))
    })?;

    let mut scored = Vec::new();
    for row in rows {
        let (chunk, doc) = row?;
        let score = cosine_similarity(query_vec, &chunk.embedding);
        scored.push(ScoredItem {
            score,
            lexical: None,
            semantic: Some(score),
            doc,
            chunk: Some(chunk.into_chunk_row()),
        });
    }

    sort_by_score_with_tiebreak(&mut scored, false);
    scored.truncate(k);
    Ok(scored)
}

fn semantic_search_linear(
    store: &Store,
    query_vec: &[f32],
    filter_sql: &str,
    filter_params: &[SqlValue],
    k: usize,
) -> Result<Vec<ScoredItem>> {
    let sql = format!(
        "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text, chunk.embedding,\n                doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source, doc.meta\n         FROM chunk\n         JOIN doc ON doc.id = chunk.doc_id\n         WHERE chunk.deleted=0 AND doc.deleted=0 AND ({})",
        filter_sql
    );
    let mut stmt = store.conn.prepare(&sql)?;
    let rows = stmt.query_map(params_from_iter(filter_params.to_vec()), |row| {
        let (chunk, doc) = map_chunk_row_with_embedding(row)?;
        Ok((chunk, doc))
    })?;

    let mut scored = Vec::new();
    for row in rows {
        let (chunk, doc) = row?;
        let score = cosine_similarity(query_vec, &chunk.embedding);
        scored.push(ScoredItem {
            score,
            lexical: None,
            semantic: Some(score),
            doc,
            chunk: Some(chunk.into_chunk_row()),
        });
    }

    sort_by_score_with_tiebreak(&mut scored, false);
    scored.truncate(k);
    Ok(scored)
}

fn combine_results(
    config: &Config,
    lexical: Vec<ScoredItem>,
    semantic: Vec<ScoredItem>,
    k: usize,
) -> Vec<ScoredItem> {
    let mut items = if lexical.is_empty() && semantic.is_empty() {
        Vec::new()
    } else if lexical.is_empty() {
        semantic
    } else if semantic.is_empty() {
        lexical
    } else {
        let mut map: HashMap<String, ScoredItem> = HashMap::new();
        for item in lexical {
            map.insert(item.chunk.as_ref().unwrap().id.clone(), item);
        }
        for item in semantic {
            let key = item.chunk.as_ref().unwrap().id.clone();
            map.entry(key)
                .and_modify(|e| {
                    e.semantic = item.semantic;
                    e.score = 0.0;
                })
                .or_insert(item);
        }

        map.into_values()
            .map(|mut item| {
                let lex = item.lexical.unwrap_or(0.0);
                let sem = item.semantic.unwrap_or(0.0);
                let score = config.bm25_weight * lex + config.vector_weight * sem;
                item.score = score;
                item
            })
            .collect()
    };

    sort_by_score_with_tiebreak(&mut items, false);
    items.truncate(k);
    items
}

fn group_by_doc(result: SearchResult, fields: Vec<SelectField>) -> SearchResult {
    let mut map: HashMap<String, ScoredItem> = HashMap::new();
    for item in result.items {
        map.entry(item.doc.id.clone())
            .and_modify(|entry| {
                if item.score > entry.score {
                    entry.score = item.score;
                    entry.lexical = item.lexical;
                    entry.semantic = item.semantic;
                }
            })
            .or_insert_with(|| ScoredItem {
                score: item.score,
                lexical: item.lexical,
                semantic: item.semantic,
                doc: item.doc.clone(),
                chunk: None,
            });
    }
    let mut items: Vec<ScoredItem> = map.into_values().collect();
    sort_by_score_with_tiebreak(&mut items, false);
    SearchResult {
        items,
        selected_fields: Some(fields),
        ..result
    }
}

fn map_chunk_row(row: &Row) -> rusqlite::Result<(ChunkRow, DocRow)> {
    let chunk = ChunkRow {
        id: row.get(0)?,
        doc_id: row.get(1)?,
        offset: row.get(2)?,
        tokens: row.get(3)?,
        text: row.get(4)?,
    };
    let doc = DocRow {
        id: row.get(5)?,
        path: row.get(6)?,
        mtime: row.get(7)?,
        hash: row.get(8)?,
        tag: row.get(9)?,
        source: row.get(10)?,
        meta: row.get(11)?,
    };
    Ok((chunk, doc))
}

#[derive(Debug, Clone)]
struct ChunkRowWithEmbedding {
    inner: ChunkRow,
    embedding: Vec<f32>,
}

impl ChunkRowWithEmbedding {
    fn into_chunk_row(self) -> ChunkRow {
        self.inner
    }
}

fn map_chunk_row_with_embedding(row: &Row) -> rusqlite::Result<(ChunkRowWithEmbedding, DocRow)> {
    let chunk = ChunkRowWithEmbedding {
        inner: ChunkRow {
            id: row.get(0)?,
            doc_id: row.get(1)?,
            offset: row.get(2)?,
            tokens: row.get(3)?,
            text: row.get(4)?,
        },
        embedding: from_bytes(row.get::<_, Vec<u8>>(5)?.as_slice()),
    };
    let doc = DocRow {
        id: row.get(6)?,
        path: row.get(7)?,
        mtime: row.get(8)?,
        hash: row.get(9)?,
        tag: row.get(10)?,
        source: row.get(11)?,
        meta: row.get(12)?,
    };
    Ok((chunk, doc))
}

fn filter_to_sql(expr: &FilterExpr) -> Result<(String, Vec<SqlValue>)> {
    match expr {
        FilterExpr::And(a, b) => {
            let (sa, mut pa) = filter_to_sql(a)?;
            let (sb, mut pb) = filter_to_sql(b)?;
            pa.append(&mut pb);
            Ok((format!("({}) AND ({})", sa, sb), pa))
        }
        FilterExpr::Or(a, b) => {
            let (sa, mut pa) = filter_to_sql(a)?;
            let (sb, mut pb) = filter_to_sql(b)?;
            pa.append(&mut pb);
            Ok((format!("({}) OR ({})", sa, sb), pa))
        }
        FilterExpr::Not(inner) => {
            let (s, p) = filter_to_sql(inner)?;
            Ok((format!("NOT ({})", s), p))
        }
        FilterExpr::Predicate(pred) => predicate_to_sql(pred),
    }
}

fn predicate_to_sql(pred: &Predicate) -> Result<(String, Vec<SqlValue>)> {
    match pred {
        Predicate::Cmp { field, op, value } => {
            let col = field_to_sql(field)?;
            let op_str = match op {
                CmpOp::Eq => "=",
                CmpOp::Ne => "!=",
                CmpOp::Lt => "<",
                CmpOp::Lte => "<=",
                CmpOp::Gt => ">",
                CmpOp::Gte => ">=",
                CmpOp::Like => "LIKE",
                CmpOp::Glob => "GLOB",
            };
            Ok((format!("{} {} ?", col, op_str), vec![value_to_sql(value)]))
        }
        Predicate::In { field, values } => {
            let col = field_to_sql(field)?;
            let placeholders = vec!["?"; values.len()].join(", ");
            let mut params = Vec::new();
            for v in values {
                params.push(value_to_sql(v));
            }
            Ok((format!("{} IN ({})", col, placeholders), params))
        }
    }
}

fn value_to_sql(value: &Value) -> SqlValue {
    match value {
        Value::String(s) => SqlValue::from(s.clone()),
        Value::Number(n) => SqlValue::from(*n),
    }
}

fn field_to_sql(field: &FieldRef) -> Result<String> {
    let (table, name) = match &field.table {
        Some(Table::Doc) => ("doc", field.name.as_str()),
        Some(Table::Chunk) => ("chunk", field.name.as_str()),
        None => anyhow::bail!("field must be qualified: {}", field.name),
    };

    if table == "doc" {
        if let Some(key) = name.strip_prefix("meta.") {
            let key = key.trim();
            if key.is_empty() {
                anyhow::bail!("metadata key required after doc.meta");
            }
            if !key
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
            {
                anyhow::bail!("metadata key contains unsupported characters: {key}");
            }
            return Ok(format!("json_extract(doc.meta, '$.{}')", key));
        }
        if !is_doc_field(name) {
            anyhow::bail!("unknown doc field: {name}");
        }
    }
    if table == "chunk" && !is_chunk_field(name) {
        anyhow::bail!("unknown chunk field: {name}");
    }

    Ok(format!("{}.{}", table, name))
}

fn field_to_sql_for_table(field: &FieldRef, table: Table) -> Result<String> {
    let table = field.table.clone().unwrap_or(table);
    let resolved = FieldRef {
        table: Some(table),
        name: field.name.clone(),
    };
    field_to_sql(&resolved)
}

fn order_clause(q: &RqlQuery, table: Table) -> Result<String> {
    let tie_break = match table {
        Table::Doc => "doc.path ASC, doc.id ASC",
        Table::Chunk => "doc.path ASC, chunk.offset ASC, chunk.id ASC",
    };
    let default_order = format!("ORDER BY {tie_break}");

    let Some((order_by, dir)) = &q.order_by else {
        return Ok(default_order.to_string());
    };
    let dir_sql = match dir {
        OrderDir::Asc => "ASC",
        OrderDir::Desc => "DESC",
    };
    match order_by {
        OrderBy::Score => Ok(default_order),
        OrderBy::Field(field) => {
            let col = field_to_sql_for_table(field, table)?;
            Ok(format!("ORDER BY {} {}, {}", col, dir_sql, tie_break))
        }
    }
}

pub fn print_table(result: &SearchResult, explain: bool) {
    for item in &result.items {
        if let Some(chunk) = &item.chunk {
            if explain {
                println!(
                    "{:.3}\t{}\t{}\t{}",
                    item.score, item.doc.path, chunk.offset, chunk.text
                );
            } else {
                println!("{}\t{}\t{}", item.doc.path, chunk.offset, chunk.text);
            }
        } else {
            println!("{}\t{}", item.doc.path, item.doc.id);
        }
    }
}

fn apply_ordering(items: &mut [ScoredItem], order_by: OrderBy, dir: OrderDir) {
    let asc = matches!(dir, OrderDir::Asc);
    match order_by {
        OrderBy::Score => {
            items.sort_by(|a, b| {
                let ord = score_cmp(a, b, asc);
                if ord == Ordering::Equal {
                    deterministic_tiebreak(a, b)
                } else {
                    ord
                }
            });
        }
        OrderBy::Field(field) => {
            items.sort_by(|a, b| {
                let va = field_value(a, &field);
                let vb = field_value(b, &field);
                let ord = va.cmp(&vb);
                let ord = if asc { ord } else { ord.reverse() };
                if ord == Ordering::Equal {
                    deterministic_tiebreak(a, b)
                } else {
                    ord
                }
            });
        }
    }
}

fn sort_by_score_with_tiebreak(items: &mut [ScoredItem], asc: bool) {
    items.sort_by(|a, b| {
        let ord = score_cmp(a, b, asc);
        if ord == Ordering::Equal {
            deterministic_tiebreak(a, b)
        } else {
            ord
        }
    });
}

fn score_cmp(a: &ScoredItem, b: &ScoredItem, asc: bool) -> Ordering {
    let ord = a.score.partial_cmp(&b.score).unwrap_or(Ordering::Equal);
    if asc { ord } else { ord.reverse() }
}

fn deterministic_tiebreak(a: &ScoredItem, b: &ScoredItem) -> Ordering {
    let ord = a.doc.path.cmp(&b.doc.path);
    if ord != Ordering::Equal {
        return ord;
    }
    match (&a.chunk, &b.chunk) {
        (Some(achunk), Some(bchunk)) => achunk
            .offset
            .cmp(&bchunk.offset)
            .then_with(|| achunk.id.cmp(&bchunk.id)),
        _ => a.doc.id.cmp(&b.doc.id),
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum FieldValue {
    Str(String),
    Num(i64),
    None,
}

fn meta_field_value(meta: &Option<String>, key: &str) -> FieldValue {
    let Some(raw) = meta else {
        return FieldValue::None;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(raw) else {
        return FieldValue::None;
    };
    let Some(v) = value.get(key) else {
        return FieldValue::None;
    };
    match v {
        serde_json::Value::String(s) => FieldValue::Str(s.clone()),
        serde_json::Value::Number(n) => n.as_i64().map(FieldValue::Num).unwrap_or(FieldValue::None),
        serde_json::Value::Bool(b) => FieldValue::Str(b.to_string()),
        _ => FieldValue::None,
    }
}

fn field_value(item: &ScoredItem, field: &FieldRef) -> FieldValue {
    let table = field.table.clone().unwrap_or(Table::Doc);
    match table {
        Table::Doc => {
            if let Some(key) = field.name.strip_prefix("meta.") {
                return meta_field_value(&item.doc.meta, key);
            }
            match field.name.as_str() {
                "id" => FieldValue::Str(item.doc.id.clone()),
                "path" => FieldValue::Str(item.doc.path.clone()),
                "mtime" => FieldValue::Str(item.doc.mtime.clone()),
                "hash" => FieldValue::Str(item.doc.hash.clone()),
                "tag" => FieldValue::Str(item.doc.tag.clone().unwrap_or_default()),
                "source" => FieldValue::Str(item.doc.source.clone().unwrap_or_default()),
                "meta" => FieldValue::Str(item.doc.meta.clone().unwrap_or_default()),
                _ => FieldValue::None,
            }
        }
        Table::Chunk => {
            if let Some(chunk) = &item.chunk {
                match field.name.as_str() {
                    "id" => FieldValue::Str(chunk.id.clone()),
                    "doc_id" => FieldValue::Str(chunk.doc_id.clone()),
                    "offset" => FieldValue::Num(chunk.offset),
                    "tokens" => FieldValue::Num(chunk.tokens),
                    "text" => FieldValue::Str(chunk.text.clone()),
                    _ => FieldValue::None,
                }
            } else {
                FieldValue::None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_allows_doc_meta_key() {
        let expr = parse_filter("doc.meta.status = 'open'").expect("parse filter");
        let (sql, params) = filter_to_sql(&expr).expect("filter to sql");
        assert!(sql.contains("json_extract(doc.meta"));
        assert_eq!(params.len(), 1);
    }
}
