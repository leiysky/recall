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

use assert_cmd::Command;
use serde_json::Value;
use serde_json::json;
use tempfile::TempDir;

fn recall_cmd() -> Command {
    Command::new(assert_cmd::cargo::cargo_bin!("recall"))
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
        {
            if corpus_obj.contains_key("bytes") {
                corpus_obj.insert("bytes".to_string(), json!(0));
            }
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

fn assert_repeatable(args: &[&str], runs: usize, cwd: &Path) {
    let mut baseline: Option<Value> = None;
    for _ in 0..runs {
        let mut cmd = recall_cmd();
        cmd.args(args);
        let json = normalize_json(run_json(&mut cmd, cwd));
        if let Some(ref expected) = baseline {
            assert_eq!(&json, expected);
        } else {
            baseline = Some(json);
        }
    }
}

#[test]
fn deterministic_outputs() {
    let temp = TempDir::new().expect("tempdir");
    let root = temp.path();
    fs::create_dir_all(root.join("docs")).expect("docs dir");
    fs::write(root.join("docs/a.txt"), "alpha beta gamma\n").expect("write file");
    fs::write(root.join("docs/b.txt"), "beta gamma delta\n").expect("write file");
    fs::write(root.join("docs/c.txt"), "gamma delta epsilon\n").expect("write file");

    let mut cmd = recall_cmd();
    cmd.args(["init", "."]);
    assert!(cmd.current_dir(root).output().unwrap().status.success());

    let mut cmd = recall_cmd();
    cmd.args(["add", "docs", "--glob", "**/*.txt", "--json"]);
    assert!(cmd.current_dir(root).output().unwrap().status.success());

    let snapshot = "2100-01-01T00:00:00Z";

    assert_repeatable(
        &[
            "search",
            "gamma",
            "--k",
            "4",
            "--snapshot",
            snapshot,
            "--json",
        ],
        20,
        root,
    );

    assert_repeatable(
        &[
            "query",
            "--rql",
            "SELECT chunk.text, doc.path FROM chunk USING semantic('gamma') LIMIT 4;",
            "--snapshot",
            snapshot,
            "--json",
        ],
        20,
        root,
    );

    assert_repeatable(
        &[
            "context",
            "gamma",
            "--budget-tokens",
            "64",
            "--snapshot",
            snapshot,
            "--json",
        ],
        20,
        root,
    );
}
