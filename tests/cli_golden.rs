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
use std::path::Path;
use std::path::PathBuf;

use assert_cmd::Command;
use jsonschema::JSONSchema;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;

fn recall_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("recall"))
}

fn recall_cmd_with_env(config_root: &Path) -> Command {
    let mut cmd = recall_cmd();
    cmd.env("XDG_CONFIG_HOME", config_root);
    cmd.env("HOME", config_root);
    cmd.env("APPDATA", config_root);
    cmd
}

fn global_config_path(config_root: &Path) -> PathBuf {
    let base = if cfg!(target_os = "macos") {
        config_root.join("Library").join("Application Support")
    } else {
        config_root.to_path_buf()
    };
    base.join("recall").join("recall.toml")
}

fn load_schema() -> JSONSchema {
    let schema_text = include_str!("../schemas/response.schema.json");
    let schema_json: Value = serde_json::from_str(schema_text).expect("schema json");
    JSONSchema::options()
        .compile(&schema_json)
        .expect("compile schema")
}

fn normalize_json(mut value: Value) -> Value {
    if let Some(stats) = value.get_mut("stats")
        && let Some(obj) = stats.as_object_mut()
    {
        obj.insert("took_ms".to_string(), json!(0));
        if obj.contains_key("snapshot") {
            obj.insert("snapshot".to_string(), json!(""));
        }
        if obj.contains_key("timings") {
            obj.insert("timings".to_string(), json!({}));
        }
        if let Some(memory) = obj.get_mut("memory")
            && memory.is_object()
        {
            *memory = json!({});
        }
        if let Some(corpus) = obj.get_mut("corpus")
            && let Some(corpus_obj) = corpus.as_object_mut()
            && corpus_obj.contains_key("bytes")
        {
            corpus_obj.insert("bytes".to_string(), json!(0));
        }
    }
    strip_mtime(&mut value);
    value
}

fn strip_mtime(value: &mut Value) {
    match value {
        Value::Object(map) => {
            if map.contains_key("mtime") {
                map.insert("mtime".to_string(), json!(""));
            }
            if map.contains_key("snapshot") {
                map.insert("snapshot".to_string(), json!(""));
            }
            for v in map.values_mut() {
                strip_mtime(v);
            }
        }
        Value::Array(items) => {
            for v in items {
                strip_mtime(v);
            }
        }
        _ => {}
    }
}

fn run_json(cmd: &mut Command, cwd: &Path) -> Value {
    let output = cmd.current_dir(cwd).output().expect("run command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    serde_json::from_str(&stdout).expect("parse json")
}

fn assert_schema(schema: &JSONSchema, value: &Value) {
    if let Err(errors) = schema.validate(value) {
        let msgs: Vec<String> = errors.map(|e| e.to_string()).collect();
        panic!("schema validation failed:\\n{}", msgs.join("\\n"));
    }
}

#[test]
fn golden_cli_outputs() {
    let schema = load_schema();
    let config_temp = TempDir::new().expect("config tempdir");
    let config_root = config_temp.path();
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();

    // Seed a tiny corpus
    fs::create_dir_all(root.join("docs")).expect("docs dir");
    fs::write(root.join("docs/a.txt"), "hello world\nthis is recall\n").expect("write file");

    // init
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["init", "."]);
    let output = cmd.current_dir(root).output().expect("init");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // add
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "add", "docs", "--glob", "**/*.txt", "--tag", "docs", "--json",
    ]);
    let add_json = run_json(&mut cmd, root);
    assert_schema(&schema, &add_json);
    insta::assert_json_snapshot!("add", normalize_json(add_json));

    // search
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "search",
        "hello",
        "--k",
        "4",
        "--filter",
        "doc.tag = 'docs'",
        "--json",
        "--explain",
    ]);
    let search_json = run_json(&mut cmd, root);
    assert_schema(&schema, &search_json);
    insta::assert_json_snapshot!("search", normalize_json(search_json));

    // search with punctuation (FTS5 sanitization fallback)
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "search",
        "?",
        "--k",
        "4",
        "--filter",
        "doc.tag = 'docs'",
        "--json",
        "--explain",
    ]);
    let search_sanitized_json = run_json(&mut cmd, root);
    assert_schema(&schema, &search_sanitized_json);
    insta::assert_json_snapshot!("search_sanitized", normalize_json(search_sanitized_json));

    // query
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "query",
        "--rql",
        "FROM chunk USING semantic('recall') LIMIT 3 SELECT chunk.text, chunk.offset, doc.path;",
        "--json",
    ]);
    let query_json = run_json(&mut cmd, root);
    assert_schema(&schema, &query_json);
    insta::assert_json_snapshot!("query", normalize_json(query_json));

    // structured query with filter + limit
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "query",
        "--rql",
        "FROM doc FILTER doc.path LIKE '%docs%' LIMIT 2 SELECT doc.path;",
        "--json",
    ]);
    let query_structured_json = run_json(&mut cmd, root);
    assert_schema(&schema, &query_structured_json);
    insta::assert_json_snapshot!(
        "query_structured_filter",
        normalize_json(query_structured_json)
    );

    // context
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["context", "recall", "--budget-tokens", "16", "--json"]);
    let context_json = run_json(&mut cmd, root);
    assert_schema(&schema, &context_json);
    insta::assert_json_snapshot!("context", normalize_json(context_json));

    // stats
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["stats", "--json"]);
    let stats_json = run_json(&mut cmd, root);
    assert_schema(&schema, &stats_json);
    insta::assert_json_snapshot!("stats", normalize_json(stats_json));

    // guide
    let mut cmd = recall_cmd();
    cmd.args(["guide"]);
    let output = cmd.current_dir(root).output().expect("guide");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!("guide", stdout);
}

#[test]
fn export_import_roundtrip() {
    let schema = load_schema();
    let config_temp = TempDir::new().expect("config tempdir");
    let config_root = config_temp.path();
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).expect("docs dir");
    fs::write(root.join("docs/a.txt"), "export roundtrip test\n").expect("write file");

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["init", "."]);
    assert!(cmd.current_dir(root).output().unwrap().status.success());

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["add", "docs", "--glob", "**/*.txt", "--json"]);
    let add_json = run_json(&mut cmd, root);
    assert_schema(&schema, &add_json);

    let export_path = root.join("export.jsonl");
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "export",
        "--out",
        export_path.to_string_lossy().as_ref(),
        "--json",
    ]);
    let export_json = run_json(&mut cmd, root);
    assert_schema(&schema, &export_json);

    let temp2 = TempDir::new().expect("tempdir");
    let root2 = temp2.path();
    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["init", "."]);
    assert!(cmd.current_dir(root2).output().unwrap().status.success());

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["import", export_path.to_string_lossy().as_ref(), "--json"]);
    let import_json = run_json(&mut cmd, root2);
    assert_schema(&schema, &import_json);

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["search", "roundtrip", "--json"]);
    let search_json = run_json(&mut cmd, root2);
    assert_schema(&schema, &search_json);
}

#[test]
fn structured_query_default_ordering() {
    let schema = load_schema();
    let config_temp = TempDir::new().expect("config tempdir");
    let config_root = config_temp.path();
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).expect("docs dir");
    fs::write(root.join("docs/b.txt"), "beta\n").expect("write file");
    fs::write(root.join("docs/a.txt"), "alpha\n").expect("write file");

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["init", "."]);
    assert!(cmd.current_dir(root).output().unwrap().status.success());

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["add", "docs", "--glob", "**/*.txt", "--json"]);
    let add_json = run_json(&mut cmd, root);
    assert_schema(&schema, &add_json);

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "query",
        "--rql",
        "FROM doc FILTER doc.path GLOB '**/*.txt' LIMIT 10 SELECT doc.path;",
        "--json",
    ]);
    let query_json = run_json(&mut cmd, root);
    assert_schema(&schema, &query_json);

    let results = query_json
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    let paths: Vec<&str> = results
        .iter()
        .filter_map(|item| item.get("doc"))
        .filter_map(|doc| doc.get("path"))
        .filter_map(|path| path.as_str())
        .collect();
    assert_eq!(paths, vec!["docs/a.txt", "docs/b.txt"]);
}

#[test]
fn structured_query_order_by_tiebreaks() {
    let schema = load_schema();
    let config_temp = TempDir::new().expect("config tempdir");
    let config_root = config_temp.path();
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).expect("docs dir");
    fs::write(root.join("docs/b.txt"), "beta\n").expect("write file");
    fs::write(root.join("docs/a.txt"), "alpha\n").expect("write file");

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["init", "."]);
    assert!(cmd.current_dir(root).output().unwrap().status.success());

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "add", "docs", "--glob", "**/*.txt", "--tag", "docs", "--json",
    ]);
    let add_json = run_json(&mut cmd, root);
    assert_schema(&schema, &add_json);

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args([
        "query",
        "--rql",
        "FROM doc FILTER doc.tag = \"docs\" ORDER BY doc.tag DESC LIMIT 10 SELECT doc.path, doc.tag;",
        "--json",
    ]);
    let query_json = run_json(&mut cmd, root);
    assert_schema(&schema, &query_json);

    let results = query_json
        .get("results")
        .and_then(|v| v.as_array())
        .expect("results array");
    let paths: Vec<&str> = results
        .iter()
        .filter_map(|item| item.get("doc"))
        .filter_map(|doc| doc.get("path"))
        .filter_map(|path| path.as_str())
        .collect();
    assert_eq!(paths, vec!["docs/a.txt", "docs/b.txt"]);
}

#[test]
fn hnsw_backend_search() {
    let schema = load_schema();
    let config_temp = TempDir::new().expect("config tempdir");
    let config_root = config_temp.path();
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).expect("docs dir");
    fs::write(root.join("docs/a.txt"), "hello hnsw\n").expect("write file");

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["init", "."]);
    assert!(cmd.current_dir(root).output().unwrap().status.success());

    let config_path = global_config_path(config_root);
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("config dir");
    let config = String::from(
        "store_path = \"recall.db\"\nchunk_tokens = 256\noverlap_tokens = 32\nembedding_dim = 256\nembedding = \"hash\"\nann_backend = \"hnsw\"\nann_bits = 16\nann_seed = 42\nbm25_weight = 0.5\nvector_weight = 0.5\nmax_limit = 1000\n",
    );
    fs::write(&config_path, config).expect("write config");

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["add", "docs", "--glob", "**/*.txt", "--json"]);
    let add_json = run_json(&mut cmd, root);
    assert_schema(&schema, &add_json);

    let mut cmd = recall_cmd_with_env(config_root);
    cmd.args(["search", "hnsw", "--json"]);
    let search_json = run_json(&mut cmd, root);
    assert_schema(&schema, &search_json);
}
