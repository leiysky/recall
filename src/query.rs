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
use std::collections::HashMap;
use std::time::Instant;

use anyhow::Result;
use rusqlite::Row;
use rusqlite::params_from_iter;
use rusqlite::types::Value as SqlValue;

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
}

#[derive(Debug, Clone)]
pub struct SearchInputs {
    pub semantic: Option<String>,
    pub lexical: Option<String>,
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
    vec!["id", "path", "mtime", "hash", "tag", "source"]
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

    let mut lexical_results = Vec::new();
    if opts.use_lexical {
        if let Some(lex_query) = inputs.lexical.clone() {
            let (results, warning) =
                lexical_search(store, &lex_query, &filter_sql, &filter_params, opts.k)?;
            lexical_results = results;
            if let Some(warning) = warning {
                explain_warnings.push(warning);
            }
        } else {
            explain_warnings
                .push("lexical search requested but no lexical query provided".to_string());
        }
    }

    let mut semantic_results = Vec::new();
    if opts.use_semantic {
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
    }

    let items = combine_results(config, lexical_results, semantic_results, opts.k);

    let stats = StatsOut {
        took_ms: started.elapsed().as_millis() as i64,
        total_hits: items.len() as i64,
        doc_count: None,
        chunk_count: None,
        db_size_bytes: None,
        snapshot: store.snapshot_token().ok(),
    };

    Ok(SearchResult {
        items,
        stats,
        filter: opts.filter,
        explain_warnings,
        selected_fields,
        include_explain: opts.explain,
        limit,
        offset,
    })
}

pub fn run_rql(
    store: &Store,
    config: &Config,
    rql_text: &str,
    explain: bool,
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

    run_structured_query(store, &q)
}

fn run_structured_query(store: &Store, q: &RqlQuery) -> Result<SearchResult> {
    let started = Instant::now();
    let limit = q.limit.unwrap_or(1000);
    let offset = q.offset.unwrap_or(0);
    let (filter_sql, filter_params) = if let Some(expr) = &q.filter {
        filter_to_sql(expr)?
    } else {
        ("1=1".to_string(), Vec::new())
    };

    let mut items = Vec::new();

    if q.table == Table::Chunk {
        let order_sql = order_clause(q, Table::Chunk)?;
        let sql = format!(
            "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text, doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source\n             FROM chunk JOIN doc ON doc.id = chunk.doc_id\n             WHERE chunk.deleted=0 AND doc.deleted=0 AND ({})\n             {}\n             LIMIT ? OFFSET ?",
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
            "SELECT doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source\n             FROM doc\n             WHERE doc.deleted=0 AND ({})\n             {}\n             LIMIT ? OFFSET ?",
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
        snapshot: store.snapshot_token().ok(),
    };

    Ok(SearchResult {
        items,
        stats,
        filter: None,
        explain_warnings: Vec::new(),
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
) -> Result<(Vec<ScoredItem>, Option<String>)> {
    let run = |query: &str| -> Result<Vec<ScoredItem>> {
        let sql = format!(
            "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text,\n                doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source,\n                bm25(chunk_fts) as bm25\n         FROM chunk_fts\n         JOIN chunk ON chunk_fts.rowid = chunk.rowid\n         JOIN doc ON doc.id = chunk.doc_id\n         WHERE chunk.deleted=0 AND doc.deleted=0 AND ({}) AND chunk_fts MATCH ?\n         ORDER BY bm25\n         LIMIT ?",
            filter_sql
        );

        let mut params: Vec<SqlValue> = filter_params.to_vec();
        params.push(SqlValue::from(query.to_string()));
        params.push(SqlValue::from(k as i64));

        let mut stmt = store.conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(params), |row| {
            let (chunk, doc) = map_chunk_row(row)?;
            let bm25: f64 = row.get(11)?;
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

    match run(query) {
        Ok(results) => Ok((results, None)),
        Err(err) => {
            if is_fts5_syntax_error(&err) {
                let (sanitized, changed) = sanitize_fts5_query(query);
                if !changed {
                    return Err(err);
                }
                if sanitized.is_empty() {
                    return Ok((
                        Vec::new(),
                        Some(
                            "lexical query sanitized to empty; skipping lexical search".to_string(),
                        ),
                    ));
                }
                match run(&sanitized) {
                    Ok(results) => Ok((
                        results,
                        Some(format!(
                            "lexical query sanitized for FTS5: \"{}\"",
                            sanitized
                        )),
                    )),
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

    if config.ann_bits > 0
        && let Ok(results) =
            semantic_search_lsh(store, config, &query_vec, filter_sql, filter_params, k)
        && results.len() >= k
    {
        return Ok(results);
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
        "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text, chunk.embedding,\n                doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source\n         FROM ann_lsh\n         JOIN chunk ON ann_lsh.chunk_id = chunk.id\n         JOIN doc ON doc.id = chunk.doc_id\n         WHERE chunk.deleted=0 AND doc.deleted=0 AND ({}) AND ann_lsh.signature IN ({})",
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

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
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
        "SELECT chunk.id, chunk.doc_id, chunk.offset, chunk.tokens, chunk.text, chunk.embedding,\n                doc.id, doc.path, doc.mtime, doc.hash, doc.tag, doc.source\n         FROM chunk\n         JOIN doc ON doc.id = chunk.doc_id\n         WHERE chunk.deleted=0 AND doc.deleted=0 AND ({})",
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

    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    scored.truncate(k);
    Ok(scored)
}

fn combine_results(
    config: &Config,
    lexical: Vec<ScoredItem>,
    semantic: Vec<ScoredItem>,
    k: usize,
) -> Vec<ScoredItem> {
    if lexical.is_empty() {
        return semantic.into_iter().take(k).collect();
    }
    if semantic.is_empty() {
        return lexical.into_iter().take(k).collect();
    }

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

    let mut items: Vec<ScoredItem> = map
        .into_values()
        .map(|mut item| {
            let lex = item.lexical.unwrap_or(0.0);
            let sem = item.semantic.unwrap_or(0.0);
            let score = config.bm25_weight * lex + config.vector_weight * sem;
            item.score = score;
            item
        })
        .collect();

    items.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.doc.path.cmp(&b.doc.path))
            .then_with(|| {
                let ao = a.chunk.as_ref().map(|c| c.offset).unwrap_or(0);
                let bo = b.chunk.as_ref().map(|c| c.offset).unwrap_or(0);
                ao.cmp(&bo)
            })
            .then_with(|| {
                let aid = a.chunk.as_ref().map(|c| c.id.as_str()).unwrap_or("");
                let bid = b.chunk.as_ref().map(|c| c.id.as_str()).unwrap_or("");
                aid.cmp(bid)
            })
    });

    items.truncate(k);
    items
}

fn group_by_doc(result: SearchResult, fields: Vec<SelectField>) -> SearchResult {
    let mut map: BTreeMap<String, ScoredItem> = BTreeMap::new();
    for item in result.items {
        let entry = map
            .entry(item.doc.id.clone())
            .or_insert_with(|| ScoredItem {
                score: item.score,
                lexical: item.lexical,
                semantic: item.semantic,
                doc: item.doc.clone(),
                chunk: None,
            });
        if item.score > entry.score {
            entry.score = item.score;
        }
    }
    let items = map.into_values().collect();
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

    if table == "doc" && !is_doc_field(name) {
        anyhow::bail!("unknown doc field: {name}");
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
    let default_order = match table {
        Table::Doc => "ORDER BY doc.path ASC, doc.id ASC",
        Table::Chunk => "ORDER BY doc.path ASC, chunk.offset ASC, chunk.id ASC",
    };

    let Some((order_by, dir)) = &q.order_by else {
        return Ok(default_order.to_string());
    };
    let dir_sql = match dir {
        OrderDir::Asc => "ASC",
        OrderDir::Desc => "DESC",
    };
    match order_by {
        OrderBy::Score => Ok(default_order.to_string()),
        OrderBy::Field(field) => {
            let col = field_to_sql_for_table(field, table)?;
            Ok(format!("ORDER BY {} {}", col, dir_sql))
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
                let ord = a
                    .score
                    .partial_cmp(&b.score)
                    .unwrap_or(std::cmp::Ordering::Equal);
                if asc { ord } else { ord.reverse() }
            });
        }
        OrderBy::Field(field) => {
            items.sort_by(|a, b| {
                let va = field_value(a, &field);
                let vb = field_value(b, &field);
                let ord = va.cmp(&vb);
                if asc { ord } else { ord.reverse() }
            });
        }
    }
}

#[derive(Eq, PartialEq, Ord, PartialOrd)]
enum FieldValue {
    Str(String),
    Num(i64),
    None,
}

fn field_value(item: &ScoredItem, field: &FieldRef) -> FieldValue {
    let table = field.table.clone().unwrap_or(Table::Doc);
    match table {
        Table::Doc => match field.name.as_str() {
            "id" => FieldValue::Str(item.doc.id.clone()),
            "path" => FieldValue::Str(item.doc.path.clone()),
            "mtime" => FieldValue::Str(item.doc.mtime.clone()),
            "hash" => FieldValue::Str(item.doc.hash.clone()),
            "tag" => FieldValue::Str(item.doc.tag.clone().unwrap_or_default()),
            "source" => FieldValue::Str(item.doc.source.clone().unwrap_or_default()),
            _ => FieldValue::None,
        },
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
