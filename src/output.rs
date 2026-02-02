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

use anyhow::Result;
use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Default)]
pub struct TimingBreakdown {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lexical_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub semantic_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub combine_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub order_ms: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assemble_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CorpusStats {
    pub docs: i64,
    pub chunks: i64,
    pub tokens: i64,
    pub bytes: u64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MemoryStats {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rss_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub virt_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct StatsOut {
    pub took_ms: i64,
    pub total_hits: i64,
    pub doc_count: Option<i64>,
    pub chunk_count: Option<i64>,
    pub db_size_bytes: Option<u64>,
    pub snapshot: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timings: Option<TimingBreakdown>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub corpus: Option<CorpusStats>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory: Option<MemoryStats>,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueryOut {
    pub text: String,
    pub rql: Option<String>,
    pub filters: Option<String>,
    pub limit: i64,
    pub offset: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lexical_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorOut {
    pub code: String,
    pub message: String,
    pub details: Option<String>,
    pub hint: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct JsonResponse {
    pub ok: bool,
    pub schema_version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub query: Option<QueryOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub results: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub explain: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<StatsOut>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actions: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ErrorOut>,
}

impl JsonResponse {
    pub fn ok() -> Self {
        Self {
            ok: true,
            schema_version: "1".to_string(),
            ..Default::default()
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
        Self {
            ok: false,
            schema_version: "1".to_string(),
            error: Some(ErrorOut {
                code: code.to_string(),
                message: message.to_string(),
                details: None,
                hint: None,
            }),
            ..Default::default()
        }
    }

    pub fn with_query(
        mut self,
        text: &str,
        rql: Option<&str>,
        filters: Option<String>,
        limit: usize,
        offset: usize,
    ) -> Self {
        self.query = Some(QueryOut {
            text: text.to_string(),
            rql: rql.map(|s| s.to_string()),
            filters,
            limit: limit as i64,
            offset: offset as i64,
            lexical_mode: None,
            snapshot: None,
        });
        self
    }

    pub fn with_query_meta(
        mut self,
        lexical_mode: Option<String>,
        snapshot: Option<String>,
    ) -> Self {
        if let Some(query) = &mut self.query {
            query.lexical_mode = lexical_mode;
            query.snapshot = snapshot;
        }
        self
    }

    pub fn with_results(mut self, results: Vec<Value>) -> Self {
        self.results = Some(results);
        self
    }

    pub fn with_context(mut self, context: Value) -> Self {
        self.context = Some(context);
        self
    }

    pub fn with_explain(mut self, explain: Value) -> Self {
        self.explain = Some(explain);
        self
    }

    pub fn with_stats(mut self, stats: StatsOut) -> Self {
        self.stats = Some(stats);
        self
    }

    pub fn with_diagnostics(mut self, diagnostics: Value) -> Self {
        self.diagnostics = Some(diagnostics);
        self
    }

    pub fn with_actions(mut self, actions: Vec<String>) -> Self {
        if actions.is_empty() {
            return self;
        }
        self.actions = Some(actions);
        self
    }

    pub fn with_warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings = warnings;
        self
    }
}

pub fn print_json(resp: &JsonResponse) -> Result<()> {
    let text = serde_json::to_string_pretty(resp)?;
    println!("{text}");
    Ok(())
}
