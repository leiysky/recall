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
        let config = load_global_config()?;
        let root = find_store_root(start, &config.store_path)
            .ok_or_else(|| anyhow::anyhow!("store not found; run `recall init` first"))?;
        let legacy = root.join("recall.toml");
        if legacy.exists() {
            let global = global_config_path()
                .map(|path| path.display().to_string())
                .unwrap_or_else(|| "<config dir unavailable>".to_string());
            anyhow::bail!(
                "local recall.toml is no longer supported; move settings to {}",
                global
            );
        }
        Ok(Self { root, config })
    }

    pub fn store_path(&self) -> PathBuf {
        if self.config.store_path.is_absolute() {
            self.config.store_path.clone()
        } else {
            self.root.join(&self.config.store_path)
        }
    }
}

fn config_dir() -> Option<PathBuf> {
    if cfg!(target_os = "windows") {
        if let Ok(appdata) = std::env::var("APPDATA") {
            return Some(PathBuf::from(appdata));
        }
        if let Ok(profile) = std::env::var("USERPROFILE") {
            return Some(PathBuf::from(profile).join("AppData").join("Roaming"));
        }
        return None;
    }

    if cfg!(target_os = "macos") {
        let home = std::env::var("HOME").ok()?;
        return Some(
            PathBuf::from(home)
                .join("Library")
                .join("Application Support"),
        );
    }

    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        return Some(PathBuf::from(xdg));
    }
    let home = std::env::var("HOME").ok()?;
    Some(PathBuf::from(home).join(".config"))
}

pub fn global_config_path() -> Option<PathBuf> {
    config_dir().map(|dir| dir.join("recall").join("recall.toml"))
}

pub fn load_global_config() -> Result<Config> {
    let Some(path) = global_config_path() else {
        return Ok(Config::default());
    };
    if !path.exists() {
        return Ok(Config::default());
    }
    read_config(&path)
}

pub fn find_store_root(start: &Path, store_path: &Path) -> Option<PathBuf> {
    if store_path.is_absolute() {
        return store_path
            .exists()
            .then(|| store_path.parent().unwrap_or(store_path).to_path_buf());
    }

    let mut cur = start.canonicalize().unwrap_or_else(|_| start.to_path_buf());
    loop {
        let candidate = cur.join(store_path);
        if candidate.exists() {
            return Some(cur);
        }
        match cur.parent() {
            Some(parent) => cur = parent.to_path_buf(),
            None => return None,
        }
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

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::path::PathBuf;
    use std::sync::Mutex;

    use tempfile::tempdir;

    use super::*;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn config_path(config_root: &Path) -> PathBuf {
        let base = if cfg!(target_os = "macos") {
            config_root.join("Library").join("Application Support")
        } else {
            config_root.to_path_buf()
        };
        base.join("recall").join("recall.toml")
    }

    fn with_env<T>(config_root: &Path, f: impl FnOnce() -> T) -> T {
        let _guard = ENV_LOCK.lock().expect("env lock");
        let old_xdg = std::env::var("XDG_CONFIG_HOME").ok();
        let old_home = std::env::var("HOME").ok();
        let old_appdata = std::env::var("APPDATA").ok();
        set_env_var("XDG_CONFIG_HOME", config_root);
        set_env_var("HOME", config_root);
        set_env_var("APPDATA", config_root);
        let result = f();
        match old_xdg {
            Some(val) => set_env_var("XDG_CONFIG_HOME", val),
            None => remove_env_var("XDG_CONFIG_HOME"),
        }
        match old_home {
            Some(val) => set_env_var("HOME", val),
            None => remove_env_var("HOME"),
        }
        match old_appdata {
            Some(val) => set_env_var("APPDATA", val),
            None => remove_env_var("APPDATA"),
        }
        result
    }

    fn set_env_var(key: &str, value: impl AsRef<std::ffi::OsStr>) {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    fn remove_env_var(key: &str) {
        unsafe {
            std::env::remove_var(key);
        }
    }

    #[test]
    fn find_store_root_walks_up() {
        let dir = tempdir().expect("tempdir");
        let root = dir.path().join("repo");
        let nested = root.join("a").join("b");
        std::fs::create_dir_all(&nested).expect("mkdir");
        std::fs::write(root.join("recall.db"), "stub").expect("write db");

        let found = find_store_root(&nested, Path::new("recall.db"));
        let expected = root.canonicalize().unwrap_or(root);
        assert_eq!(found, Some(expected));
    }

    #[test]
    fn load_from_errors_when_store_missing() {
        let config_dir = tempdir().expect("config dir");
        let work_dir = tempdir().expect("work dir");
        with_env(config_dir.path(), || {
            let err = ConfigCtx::load_from(work_dir.path()).unwrap_err();
            assert!(err.to_string().contains("store not found"));
        });
    }

    #[test]
    fn load_from_errors_on_local_config() {
        let config_dir = tempdir().expect("config dir");
        let work_dir = tempdir().expect("work dir");
        let root = work_dir.path();
        std::fs::write(root.join("recall.db"), "stub").expect("write db");
        std::fs::write(root.join("recall.toml"), "store_path = \"recall.db\"").expect("write");
        with_env(config_dir.path(), || {
            let err = ConfigCtx::load_from(root).unwrap_err();
            assert!(
                err.to_string()
                    .contains("local recall.toml is no longer supported")
            );
        });
        let _ = std::fs::remove_file(root.join("recall.toml"));
        let _ = std::fs::remove_file(config_path(config_dir.path()));
    }
}
