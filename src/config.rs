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
use serde::Deserialize;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub store_path: PathBuf,
    pub chunk_tokens: usize,
    pub overlap_tokens: usize,
    pub embedding_dim: usize,
    pub embedding: String,
    pub ann_bits: u8,
    pub ann_seed: u64,
    pub bm25_weight: f32,
    pub vector_weight: f32,
    pub max_limit: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            store_path: PathBuf::from("recall.db"),
            chunk_tokens: 256,
            overlap_tokens: 32,
            embedding_dim: 256,
            embedding: "hash".to_string(),
            ann_bits: 16,
            ann_seed: 42,
            bm25_weight: 0.5,
            vector_weight: 0.5,
            max_limit: 1000,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConfigCtx {
    pub root: PathBuf,
    pub config: Config,
}

impl ConfigCtx {
    pub fn load_from_cwd() -> Result<Self> {
        let cwd = std::env::current_dir().context("get current dir")?;
        Self::load_from(&cwd)
    }

    pub fn load_from(start: &Path) -> Result<Self> {
        let mut cur = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
        loop {
            let candidate = cur.join("recall.toml");
            if candidate.exists() {
                let config = read_config(&candidate)?;
                return Ok(Self { root: cur, config });
            }
            if let Some(parent) = cur.parent() {
                cur = parent.to_path_buf();
            } else {
                anyhow::bail!("recall.toml not found; run `recall init` first");
            }
        }
    }

    pub fn store_path(&self) -> PathBuf {
        self.root.join(&self.config.store_path)
    }
}

pub fn read_config(path: &Path) -> Result<Config> {
    let text = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut config: Config = toml::from_str(&text).context("parse recall.toml")?;
    if config.overlap_tokens >= config.chunk_tokens {
        config.overlap_tokens = 0;
    }
    Ok(config)
}

pub fn write_config(path: &Path, config: &Config) -> Result<()> {
    let text = toml::to_string_pretty(config).context("serialize recall.toml")?;
    std::fs::write(path, text).with_context(|| format!("write {}", path.display()))?;
    Ok(())
}
