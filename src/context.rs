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

use std::collections::HashMap;
use std::collections::HashSet;

use serde_json::json;

use crate::model::ChunkRow;
use crate::model::SearchResult;

#[derive(Debug, Clone)]
pub struct AssembledContext {
    pub text: String,
    pub budget_tokens: usize,
    pub used_tokens: usize,
    pub chunks: Vec<ContextChunk>,
}

#[derive(Debug, Clone)]
pub struct ContextChunk {
    pub id: String,
    pub doc_id: String,
    pub offset: i64,
    pub tokens: i64,
    pub text: String,
    pub path: String,
    pub hash: String,
    pub mtime: String,
}

impl AssembledContext {
    pub fn to_json(&self) -> serde_json::Value {
        json!({
            "text": self.text,
            "budget_tokens": self.budget_tokens,
            "used_tokens": self.used_tokens,
            "chunks": self.chunks.iter().map(|c| json!({
                "id": c.id,
                "doc_id": c.doc_id,
                "offset": c.offset,
                "tokens": c.tokens,
                "text": c.text,
                "path": c.path,
                "hash": c.hash,
                "mtime": c.mtime,
            })).collect::<Vec<_>>()
        })
    }
}

pub fn assemble(
    result: &SearchResult,
    budget_tokens: usize,
    diversity: Option<usize>,
) -> AssembledContext {
    let mut used = 0usize;
    let mut text_parts = Vec::new();
    let mut chunks = Vec::new();
    let mut seen = HashSet::new();
    let mut per_doc: HashMap<String, usize> = HashMap::new();

    for item in &result.items {
        let Some(chunk) = &item.chunk else {
            continue;
        };
        if seen.contains(&chunk.id) {
            continue;
        }
        let count = per_doc.entry(item.doc.id.clone()).or_default();
        if let Some(limit) = diversity
            && *count >= limit
        {
            continue;
        }

        let remaining = budget_tokens.saturating_sub(used);
        if remaining == 0 {
            break;
        }

        let (chunk_text, token_count) = take_tokens(chunk, remaining);
        if token_count == 0 {
            continue;
        }

        used += token_count;
        *count += 1;
        seen.insert(chunk.id.clone());

        text_parts.push(chunk_text.clone());
        chunks.push(ContextChunk {
            id: chunk.id.clone(),
            doc_id: chunk.doc_id.clone(),
            offset: chunk.offset,
            tokens: token_count as i64,
            text: chunk_text,
            path: item.doc.path.clone(),
            hash: item.doc.hash.clone(),
            mtime: item.doc.mtime.clone(),
        });
    }

    AssembledContext {
        text: text_parts.join("\n\n"),
        budget_tokens,
        used_tokens: used,
        chunks,
    }
}

fn take_tokens(chunk: &ChunkRow, limit: usize) -> (String, usize) {
    let tokens: Vec<&str> = chunk.text.split_whitespace().collect();
    if tokens.is_empty() {
        return (String::new(), 0);
    }
    if tokens.len() <= limit {
        return (chunk.text.clone(), tokens.len());
    }
    let slice = tokens[..limit].join(" ");
    (slice, limit)
}
