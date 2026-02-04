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

use std::fs;
use std::sync::OnceLock;

use anyhow::Context;
use anyhow::Result;
use sha2::Digest;
use sha2::Sha256;
use tempfile::TempDir;

use model2vec_rs::model::StaticModel;

use crate::config::Config;

pub const EMBEDDING_HASH: &str = "hash";
pub const EMBEDDING_MODEL2VEC: &str = "model2vec";
const MODEL2VEC_SAFETENSORS: &[u8] = include_bytes!("../assets/model2vec-rs/model.safetensors");
const MODEL2VEC_TOKENIZER: &[u8] = include_bytes!("../assets/model2vec-rs/tokenizer.json");
const MODEL2VEC_CONFIG: &[u8] = include_bytes!("../assets/model2vec-rs/config.json");

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
            let dim = model2vec_dim()?;
            if config.embedding_dim != 0 && config.embedding_dim != dim {
                anyhow::bail!(
                    "config embedding_dim {} does not match embedded model2vec dim {}",
                    config.embedding_dim,
                    dim
                );
            }
            Ok(EmbeddingSpec {
                name: EMBEDDING_MODEL2VEC,
                dim,
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

#[derive(Debug)]
struct Model2VecInner {
    dim: usize,
    model: StaticModel,
}

#[derive(Clone, Copy)]
pub struct Model2VecEmbedder {
    inner: &'static Model2VecInner,
}

impl Model2VecEmbedder {
    pub fn new() -> Result<Self> {
        let inner = model2vec_inner()?;
        Ok(Self { inner })
    }
}

impl Embedder for Model2VecEmbedder {
    fn embed(&self, text: &str) -> Vec<f32> {
        let mut batch = self.inner.model.encode(&[text.to_string()]);
        let Some(vec) = batch.pop() else {
            return HashEmbedder::new(self.inner.dim).embed(text);
        };
        if vec.is_empty() {
            return HashEmbedder::new(self.inner.dim).embed(text);
        }
        l2_normalize(vec)
    }
}

fn model2vec_inner() -> Result<&'static Model2VecInner> {
    static MODEL: OnceLock<Model2VecInner> = OnceLock::new();
    if let Some(inner) = MODEL.get() {
        return Ok(inner);
    }
    let inner = load_model2vec()?;
    Ok(MODEL.get_or_init(|| inner))
}

fn model2vec_dim() -> Result<usize> {
    Ok(model2vec_inner()?.dim)
}

fn load_model2vec() -> Result<Model2VecInner> {
    let temp_dir = TempDir::new().context("create model2vec tempdir")?;
    let dir = temp_dir.path();
    fs::write(dir.join("model.safetensors"), MODEL2VEC_SAFETENSORS)
        .context("write embedded model2vec safetensors")?;
    fs::write(dir.join("tokenizer.json"), MODEL2VEC_TOKENIZER)
        .context("write embedded model2vec tokenizer")?;
    fs::write(dir.join("config.json"), MODEL2VEC_CONFIG)
        .context("write embedded model2vec config")?;
    let model = StaticModel::from_pretrained(dir, None, Some(false), None)
        .context("load embedded model2vec")?;
    let dim = model
        .encode(&["".to_string()])
        .get(0)
        .map(|vec| vec.len())
        .unwrap_or(0);
    if dim == 0 {
        anyhow::bail!("embedded model2vec returned empty embedding");
    }
    Ok(Model2VecInner { dim, model })
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
    fn model2vec_embedding_dim_matches_spec() {
        let config = Config {
            embedding_dim: 0,
            ..Default::default()
        };
        let spec = resolve_embedding(&config).expect("spec");
        let model = Model2VecEmbedder::new().expect("model2vec");
        let vec = model.embed("hello world this is recall");
        assert_eq!(spec.dim, vec.len());
    }

    #[test]
    fn resolve_embedding_rejects_model2vec_dim_mismatch() {
        let dim = model2vec_dim().expect("model2vec dim");
        let config = Config {
            embedding_dim: dim + 1,
            ..Default::default()
        };
        assert!(resolve_embedding(&config).is_err());
    }
}
