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

//! Shared domain types used across ingestion, query, and context assembly.

use serde::Serialize;

use crate::output::StatsOut;
use crate::rql::SelectField;

#[derive(Debug, Clone, Serialize)]
pub struct DocRow {
    pub id: String,
    pub path: String,
    pub mtime: String,
    pub hash: String,
    pub tag: Option<String>,
    pub source: Option<String>,
    pub meta: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChunkRow {
    pub id: String,
    pub doc_id: String,
    pub offset: i64,
    pub tokens: i64,
    pub text: String,
}

#[derive(Debug, Clone)]
pub struct ScoredItem {
    pub score: f32,
    pub lexical: Option<f32>,
    pub semantic: Option<f32>,
    pub doc: DocRow,
    pub chunk: Option<ChunkRow>,
}

#[derive(Debug, Clone)]
pub struct SearchResult {
    pub items: Vec<ScoredItem>,
    pub stats: StatsOut,
    pub filter: Option<String>,
    pub explain_warnings: Vec<String>,
    pub explain: Option<serde_json::Value>,
    pub selected_fields: Option<Vec<SelectField>>,
    pub include_explain: bool,
    pub limit: usize,
    pub offset: usize,
}

impl SearchResult {
    pub fn filter_string(&self) -> Option<String> {
        self.filter.clone()
    }
}
