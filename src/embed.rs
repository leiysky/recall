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
use std::sync::OnceLock;

use anyhow::Context;
use anyhow::Result;
use serde::Deserialize;
use sha2::Digest;
use sha2::Sha256;

use crate::config::Config;

pub const EMBEDDING_HASH: &str = "hash";
pub const EMBEDDING_MODEL2VEC: &str = "model2vec";
pub const MODEL2VEC_DIM: usize = 256;
const MODEL2VEC_NAME: &str = "recall-embedded-model2vec-v1";

#[derive(Debug, Clone, Copy)]
pub struct EmbeddingSpec {
    pub name: &'static str,
    pub dim: usize,
}

pub fn resolve_embedding(config: &Config) -> Result<EmbeddingSpec> {
    let name = config.embedding.trim().to_lowercase();
    match name.as_str() {
        EMBEDDING_HASH => Ok(EmbeddingSpec {
            name: EMBEDDING_HASH,
            dim: config.embedding_dim.max(1),
        }),
        EMBEDDING_MODEL2VEC => {
            if config.embedding_dim != 0 && config.embedding_dim != MODEL2VEC_DIM {
                anyhow::bail!(
                    "config embedding_dim {} does not match model2vec embedding_dim {}",
                    config.embedding_dim,
                    MODEL2VEC_DIM
                );
            }
            Ok(EmbeddingSpec {
                name: EMBEDDING_MODEL2VEC,
                dim: MODEL2VEC_DIM,
            })
        }
        _ => anyhow::bail!(
            "unsupported embedding '{}'; supported: {} or {}",
            config.embedding,
            EMBEDDING_MODEL2VEC,
            EMBEDDING_HASH
        ),
    }
}

pub fn build_embedder(config: &Config) -> Result<Box<dyn Embedder>> {
    let spec = resolve_embedding(config)?;
    match spec.name {
        EMBEDDING_HASH => Ok(Box::new(HashEmbedder::new(spec.dim))),
        EMBEDDING_MODEL2VEC => Ok(Box::new(Model2VecEmbedder::new()?)),
        _ => unreachable!("embedding spec validated"),
    }
}

pub trait Embedder {
    fn embed(&self, text: &str) -> Vec<f32>;
}

#[derive(Clone)]
pub struct HashEmbedder {
    dim: usize,
}

impl HashEmbedder {
    pub fn new(dim: usize) -> Self {
        Self { dim: dim.max(1) }
    }
}

impl Embedder for HashEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.dim];
        for token in text.split_whitespace() {
            let (idx, sign) = hash_token_feature(token, self.dim);
            vec[idx] += sign;
        }
        l2_normalize(vec)
    }
}

fn l2_normalize(mut vec: Vec<f32>) -> Vec<f32> {
    let mut norm = 0.0f32;
    for v in &vec {
        norm += v * v;
    }
    if norm > 0.0 {
        let inv = 1.0 / norm.sqrt();
        for v in &mut vec {
            *v *= inv;
        }
    }
    vec
}

#[derive(Debug, Deserialize)]
struct Model2VecFile {
    name: String,
    dim: usize,
    tokens: Vec<String>,
    vectors: Vec<Vec<f32>>,
}

#[derive(Debug)]
struct Model2VecInner {
    dim: usize,
    vocab: HashMap<String, Vec<f32>>,
}

#[derive(Clone, Copy)]
pub struct Model2VecEmbedder {
    inner: &'static Model2VecInner,
}

impl Model2VecEmbedder {
    pub fn new() -> Result<Self> {
        static MODEL: OnceLock<Model2VecInner> = OnceLock::new();
        let inner = match MODEL.get() {
            Some(inner) => inner,
            None => {
                let model = load_model2vec()?;
                MODEL.get_or_init(|| model)
            }
        };
        Ok(Self { inner })
    }
}

impl Embedder for Model2VecEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut vec = vec![0.0f32; self.inner.dim];
        let mut used = 0usize;
        for raw in text.split_whitespace() {
            if let Some(model_vec) = self.inner.vocab.get(raw) {
                for (i, val) in model_vec.iter().enumerate() {
                    vec[i] += val;
                }
            } else {
                let (idx, sign) = hash_token_feature(raw, self.inner.dim);
                vec[idx] += sign;
            }
            used += 1;
        }
        if used == 0 {
            return HashEmbedder::new(self.inner.dim).embed(text);
        }
        l2_normalize(vec)
    }
}

fn load_model2vec() -> Result<Model2VecInner> {
    let raw = include_str!("../assets/model2vec.json");
    let data: Model2VecFile = serde_json::from_str(raw).context("parse embedded model2vec")?;
    if data.name != MODEL2VEC_NAME {
        anyhow::bail!(
            "embedded model2vec name '{}' does not match expected '{}'",
            data.name,
            MODEL2VEC_NAME
        );
    }
    if data.dim != MODEL2VEC_DIM {
        anyhow::bail!(
            "embedded model2vec dim {} does not match expected {}",
            data.dim,
            MODEL2VEC_DIM
        );
    }
    if data.tokens.len() != data.vectors.len() {
        anyhow::bail!(
            "embedded model2vec token/vector mismatch (tokens {}, vectors {})",
            data.tokens.len(),
            data.vectors.len()
        );
    }
    let mut vocab = HashMap::new();
    for (token, mut vec) in data.tokens.into_iter().zip(data.vectors.into_iter()) {
        if vec.len() != MODEL2VEC_DIM {
            anyhow::bail!(
                "embedded model2vec token '{}' has dim {}, expected {}",
                token,
                vec.len(),
                MODEL2VEC_DIM
            );
        }
        vec = l2_normalize(vec);
        if vocab.insert(token, vec).is_some() {
            anyhow::bail!("embedded model2vec has duplicate token");
        }
    }
    Ok(Model2VecInner {
        dim: MODEL2VEC_DIM,
        vocab,
    })
}

fn hash_token_feature(token: &str, dim: usize) -> (usize, f32) {
    let mut hasher = Sha256::new();
    hasher.update(token.as_bytes());
    let hash = hasher.finalize();
    let mut bytes = [0u8; 8];
    bytes.copy_from_slice(&hash[..8]);
    let val = u64::from_le_bytes(bytes);
    let idx = (val as usize) % dim;
    let sign = if (val & (1 << 63)) != 0 { 1.0 } else { -1.0 };
    (idx, sign)
}

pub fn to_bytes(vec: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(vec.len() * 4);
    for v in vec {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model2vec_matches_hash_for_sample_text() {
        let text = "hello world this is recall";
        let hash = HashEmbedder::new(MODEL2VEC_DIM).embed(text);
        let model = Model2VecEmbedder::new().expect("model2vec").embed(text);
        assert_eq!(hash, model);
    }

    #[test]
    fn resolve_embedding_rejects_model2vec_dim_mismatch() {
        let config = Config {
            embedding_dim: MODEL2VEC_DIM + 1,
            ..Default::default()
        };
        assert!(resolve_embedding(&config).is_err());
    }
}
