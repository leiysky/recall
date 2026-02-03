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

mod ann;
mod cli;
mod config;
mod context;
mod embed;
mod ingest;
mod model;
mod output;
mod query;
mod rql;
mod sql;
mod store;
mod transfer;

use std::io::Read;
use std::io::Write;
use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use clap::CommandFactory;
use clap::Parser;
use clap_complete::Shell;
use clap_complete::generate;
use clap_mangen::Man;
use serde_json::json;
use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::cli::Cli;
use crate::cli::Commands;
use crate::config::ConfigCtx;
use crate::output::JsonResponse;
use crate::output::MemoryStats;
use crate::output::StatsOut;
use crate::output::print_json;
use crate::store::StoreMode;

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Init { path } => cmd_init(path),
        Commands::Add(args) => handle_result(
            cmd_add(
                args.paths,
                args.glob,
                args.tag,
                args.source,
                args.mtime_only,
                args.ignore,
                args.parser,
                args.extract_meta,
                args.json,
            ),
            args.json,
        ),
        Commands::Search(args) => handle_result(
            cmd_search(
                args.query,
                args.k,
                args.bm25,
                args.vector,
                args.filter,
                args.explain,
                args.lexical_mode,
                args.snapshot,
                args.json,
                args.jsonl,
            ),
            args.json || args.jsonl,
        ),
        Commands::Rm(args) => handle_result(cmd_rm(args.targets, args.purge, args.json), args.json),
        Commands::Query(args) => handle_result(
            cmd_query(
                args.rql,
                args.rql_stdin,
                args.json,
                args.explain,
                args.lexical_mode,
                args.snapshot,
                args.jsonl,
            ),
            args.json || args.jsonl,
        ),
        Commands::Context(args) => handle_result(
            cmd_context(
                args.query,
                args.budget_tokens,
                args.diversity,
                args.format,
                args.filter,
                args.explain,
                args.lexical_mode,
                args.snapshot,
                args.json,
            ),
            args.json,
        ),
        Commands::Stats { json } => handle_result(cmd_stats(json), json),
        Commands::Doctor { json, fix } => handle_result(cmd_doctor(json, fix), json),
        Commands::Compact { json } => handle_result(cmd_compact(json), json),
        Commands::Export(args) => handle_result(cmd_export(args.out, args.json), args.json),
        Commands::Import(args) => handle_result(cmd_import(args.path, args.json), args.json),
        Commands::Completions { shell } => handle_result(cmd_completions(shell), false),
        Commands::Man => handle_result(cmd_man(), false),
    }
}

fn handle_result(result: Result<()>, json: bool) -> Result<()> {
    match result {
        Ok(()) => Ok(()),
        Err(err) => {
            if json {
                let resp = JsonResponse::error("error", &err.to_string());
                print_json(&resp)?;
                Ok(())
            } else {
                Err(err)
            }
        }
    }
}

fn cmd_init(path: Option<PathBuf>) -> Result<()> {
    let root = path.unwrap_or_else(|| PathBuf::from("."));
    std::fs::create_dir_all(&root).with_context(|| format!("create dir {root:?}"))?;
    let config = config::load_global_config()?;
    let store_path = root.join(&config.store_path);
    store::Store::init(&store_path)?;

    println!("Initialized Recall store at {}", store_path.display());
    let config_hint = config::global_config_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<config dir unavailable>".to_string());
    println!("Optional config: {config_hint}");
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_add(
    paths: Vec<PathBuf>,
    glob: Option<String>,
    tag: Option<String>,
    source: Option<String>,
    mtime_only: bool,
    ignore: Vec<String>,
    parser: Option<String>,
    extract_meta: bool,
    json: bool,
) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadWrite)?;

    let parser_hint = parse_parser_hint(parser.as_deref())?;
    let opts = ingest::IngestOptions {
        glob,
        tag,
        source,
        mtime_only,
        ignore,
        parser: parser_hint,
        extract_meta,
    };

    let report = ingest::ingest_paths(&store, &ctx.config, paths, opts)?;

    if json {
        let resp = JsonResponse::ok()
            .with_stats(report.stats())
            .with_warnings(report.warnings);
        print_json(&resp)?;
    } else {
        println!(
            "Added {} docs ({} chunks)",
            report.docs_added, report.chunks_added
        );
        for warn in report.warnings {
            eprintln!("warning: {warn}");
        }
    }

    Ok(())
}

fn cmd_rm(targets: Vec<String>, purge: bool, json: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadWrite)?;
    let mut removed = 0usize;
    for target in targets {
        if std::path::Path::new(&target).exists() || target.contains(std::path::MAIN_SEPARATOR) {
            removed += store.mark_doc_deleted(&target)?.len();
        } else {
            removed += store.mark_doc_deleted_by_id(&target)?;
        }
    }

    if purge {
        store.compact()?;
    }

    if json {
        let resp = JsonResponse::ok().with_stats(StatsOut {
            took_ms: 0,
            total_hits: removed as i64,
            doc_count: None,
            chunk_count: None,
            db_size_bytes: None,
            snapshot: store.snapshot_token().ok(),
            timings: None,
            corpus: None,
            memory: None,
        });
        print_json(&resp)?;
    } else {
        println!("Removed {removed} docs (tombstoned)");
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_search(
    query: String,
    k: usize,
    bm25: bool,
    vector: bool,
    filter: Option<String>,
    explain: bool,
    lexical_mode: String,
    snapshot: Option<String>,
    json: bool,
    jsonl: bool,
) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadOnly)?;

    let filter = load_filter(filter)?;
    let lexical_mode = parse_lexical_mode(&lexical_mode)?;
    let lexical_mode_label = lexical_mode.as_label().to_string();
    let snapshot = validate_snapshot(snapshot)?;
    let opts = query::SearchOptions {
        k,
        use_lexical: bm25 || !vector,
        use_semantic: vector || !bm25,
        filter,
        explain,
        lexical_mode,
        snapshot: snapshot.clone(),
    };

    let result = query::search_chunks(&store, &ctx.config, &query, opts)?;

    if json || jsonl {
        let mut resp = JsonResponse::ok().with_query(
            &query,
            None,
            result.filter_string(),
            result.limit,
            result.offset,
        );
        let snapshot_meta = snapshot.clone().or_else(|| result.stats.snapshot.clone());
        resp = resp.with_query_meta(Some(lexical_mode_label), snapshot_meta);
        resp = resp.with_results(result.to_json_results());
        resp = resp.with_stats(result.stats);
        if explain {
            resp = resp.with_warnings(result.explain_warnings);
            if let Some(explain) = result.explain {
                resp = resp.with_explain(explain);
            }
        }
        if jsonl {
            print_jsonl_response(resp)?;
        } else {
            print_json(&resp)?;
        }
    } else {
        query::print_table(&result, explain);
        if explain && !result.explain_warnings.is_empty() {
            for warning in result.explain_warnings {
                eprintln!("warning: {warning}");
            }
        }
    }

    Ok(())
}

fn cmd_query(
    rql_input: Option<String>,
    rql_stdin: bool,
    json: bool,
    explain: bool,
    lexical_mode: String,
    snapshot: Option<String>,
    jsonl: bool,
) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadOnly)?;

    let rql_text = if rql_stdin {
        let mut input = String::new();
        std::io::stdin()
            .read_to_string(&mut input)
            .context("read RQL from stdin")?;
        let trimmed = input.trim().to_string();
        if trimmed.is_empty() {
            anyhow::bail!("--rql-stdin provided but stdin was empty");
        }
        trimmed
    } else {
        let Some(rql_input) = rql_input else {
            anyhow::bail!("--rql is required unless --rql-stdin is provided");
        };
        if let Some(path) = rql_input.strip_prefix('@') {
            std::fs::read_to_string(path).with_context(|| format!("read RQL file {path}"))?
        } else {
            rql_input
        }
    };

    let lexical_mode = parse_lexical_mode(&lexical_mode)?;
    let lexical_mode_label = lexical_mode.as_label().to_string();
    let snapshot = validate_snapshot(snapshot)?;

    let result = query::run_rql(
        &store,
        &ctx.config,
        &rql_text,
        explain,
        lexical_mode,
        snapshot.clone(),
    )?;

    if json || jsonl {
        let mut resp = JsonResponse::ok().with_query(
            "",
            Some(&rql_text),
            result.filter_string(),
            result.limit,
            result.offset,
        );
        let snapshot_meta = snapshot.clone().or_else(|| result.stats.snapshot.clone());
        resp = resp.with_query_meta(Some(lexical_mode_label), snapshot_meta);
        resp = resp.with_results(result.to_json_results());
        resp = resp.with_stats(result.stats);
        if explain {
            resp = resp.with_warnings(result.explain_warnings);
            if let Some(explain) = result.explain {
                resp = resp.with_explain(explain);
            }
        }
        if jsonl {
            print_jsonl_response(resp)?;
        } else {
            print_json(&resp)?;
        }
    } else {
        query::print_table(&result, explain);
        if explain && !result.explain_warnings.is_empty() {
            for warning in result.explain_warnings {
                eprintln!("warning: {warning}");
            }
        }
    }

    Ok(())
}

fn cmd_export(out: Option<PathBuf>, json: bool) -> Result<()> {
    if json && out.is_none() {
        anyhow::bail!("--json requires --out for export");
    }
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadOnly)?;

    let stats = if let Some(path) = out {
        let file =
            std::fs::File::create(&path).with_context(|| format!("create {}", path.display()))?;
        transfer::export_store(&store, file)?
    } else {
        let stdout = std::io::stdout();
        let handle = stdout.lock();
        transfer::export_store(&store, handle)?
    };

    if json {
        let resp = JsonResponse::ok().with_stats(StatsOut {
            took_ms: 0,
            total_hits: (stats.docs + stats.chunks) as i64,
            doc_count: Some(stats.docs as i64),
            chunk_count: Some(stats.chunks as i64),
            db_size_bytes: None,
            snapshot: store.snapshot_token().ok(),
            timings: None,
            corpus: None,
            memory: None,
        });
        print_json(&resp)?;
    }

    Ok(())
}

fn cmd_import(path: PathBuf, json: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadWrite)?;
    let file = std::fs::File::open(&path).with_context(|| format!("open {}", path.display()))?;
    let stats = transfer::import_store(&store, &ctx.config, file)?;

    if json {
        let resp = JsonResponse::ok().with_stats(StatsOut {
            took_ms: 0,
            total_hits: (stats.docs + stats.chunks) as i64,
            doc_count: Some(stats.docs as i64),
            chunk_count: Some(stats.chunks as i64),
            db_size_bytes: None,
            snapshot: store.snapshot_token().ok(),
            timings: None,
            corpus: None,
            memory: None,
        });
        print_json(&resp)?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn cmd_context(
    query: String,
    budget_tokens: usize,
    diversity: Option<usize>,
    format: Option<String>,
    filter: Option<String>,
    explain: bool,
    lexical_mode: String,
    snapshot: Option<String>,
    json: bool,
) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadOnly)?;

    let output_json = match format.as_deref() {
        Some("json") => true,
        Some("text") => false,
        Some(_) => json,
        None => json,
    };
    if json && format.as_deref() == Some("text") {
        anyhow::bail!("--format text cannot be combined with --json");
    }
    if explain && !output_json {
        anyhow::bail!("context --explain requires --json or --format json");
    }

    let filter = load_filter(filter)?;
    let lexical_mode = parse_lexical_mode(&lexical_mode)?;
    let lexical_mode_label = lexical_mode.as_label().to_string();
    let snapshot = validate_snapshot(snapshot)?;
    let opts = query::SearchOptions {
        k: 100,
        use_lexical: true,
        use_semantic: true,
        filter,
        explain,
        lexical_mode,
        snapshot: snapshot.clone(),
    };

    let search = query::search_chunks(&store, &ctx.config, &query, opts)?;
    let assemble_start = std::time::Instant::now();
    let assembled = context::assemble(&search, budget_tokens, diversity);
    let assemble_ms = assemble_start.elapsed().as_millis() as i64;
    let mut stats = search.stats.clone();
    if let Some(timings) = stats.timings.as_mut() {
        timings.assemble_ms = Some(assemble_ms);
    } else {
        stats.timings = Some(crate::output::TimingBreakdown {
            assemble_ms: Some(assemble_ms),
            ..Default::default()
        });
    }

    if output_json {
        let mut resp = JsonResponse::ok().with_query(
            &query,
            None,
            search.filter_string(),
            search.limit,
            search.offset,
        );
        let snapshot_meta = snapshot.clone().or_else(|| stats.snapshot.clone());
        resp = resp.with_query_meta(Some(lexical_mode_label), snapshot_meta);
        resp = resp.with_context(assembled.to_json());
        resp = resp.with_stats(stats);
        if explain {
            resp = resp.with_warnings(search.explain_warnings);
            if let Some(explain) = search.explain {
                resp = resp.with_explain(explain);
            }
        }
        print_json(&resp)?;
    } else {
        println!("{}", assembled.text);
    }

    Ok(())
}

fn cmd_stats(json: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadOnly)?;
    let stats = store.stats()?;
    let corpus = store.corpus_stats()?;
    let memory = memory_stats();
    let stats_out = StatsOut {
        took_ms: 0,
        total_hits: 0,
        doc_count: Some(stats.doc_count),
        chunk_count: Some(stats.chunk_count),
        db_size_bytes: Some(stats.db_size_bytes),
        snapshot: store.snapshot_token().ok(),
        timings: None,
        corpus: Some(corpus),
        memory: Some(memory),
    };

    if json {
        let resp = JsonResponse::ok().with_stats(stats_out);
        print_json(&resp)?;
    } else {
        println!("Docs: {}", stats.doc_count);
        println!("Chunks: {}", stats.chunk_count);
        println!("DB size: {} bytes", stats.db_size_bytes);
        println!("Embedding: {}", ctx.config.embedding);
    }

    Ok(())
}

fn cmd_doctor(json: bool, fix: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let mode = if fix {
        StoreMode::ReadWrite
    } else {
        StoreMode::ReadOnly
    };
    let store = store::Store::open(&ctx.store_path(), mode)?;
    let report = store.integrity_check()?;
    let mut consistency = store.consistency_report()?;
    let mut actions = Vec::new();

    if fix {
        if !consistency.fts_ok() {
            store.rebuild_fts()?;
            actions.push("rebuild fts index".to_string());
        }
        if !consistency.ann_ok() {
            let rebuilt = store.rebuild_ann_lsh(&ctx.config)?;
            actions.push(format!("rebuild ann_lsh ({rebuilt} entries)"));
        }
        if ctx.config.ann_backend == "hnsw" && !consistency.hnsw_ok() {
            let rebuilt = store.rebuild_ann_hnsw()?;
            actions.push(format!("rebuild ann_hnsw ({rebuilt} nodes)"));
        }
        consistency = store.consistency_report()?;
    }

    let stats_out = StatsOut {
        took_ms: 0,
        total_hits: 0,
        doc_count: Some(report.stats.doc_count),
        chunk_count: Some(report.stats.chunk_count),
        db_size_bytes: Some(report.stats.db_size_bytes),
        snapshot: store.snapshot_token().ok(),
        timings: None,
        corpus: None,
        memory: None,
    };

    if json {
        let diagnostics = json!({
            "integrity": {
                "status": report.status,
                "ok": report.status == "ok",
            },
            "fts": {
                "status": if consistency.fts_ok() { "ok" } else { "stale" },
                "chunk_count": consistency.chunk_count,
                "index_count": consistency.fts_count,
                "missing": consistency.fts_missing,
                "hint": if consistency.fts_ok() { "" } else { "Run `recall doctor --fix` to rebuild the FTS index." },
            },
            "ann": {
                "status": if consistency.ann_ok() { "ok" } else { "stale" },
                "chunk_count": consistency.chunk_count,
                "index_count": consistency.ann_count,
                "missing": consistency.ann_missing,
                "hint": if consistency.ann_ok() { "" } else { "Run `recall doctor --fix` to rebuild the ANN index." },
            },
            "hnsw": {
                "status": if consistency.hnsw_ok() { "ok" } else { "stale" },
                "chunk_count": consistency.chunk_count,
                "index_count": consistency.hnsw_count,
                "missing": consistency.hnsw_missing,
                "hint": if consistency.hnsw_ok() { "" } else { "Run `recall doctor --fix` to rebuild the HNSW index." },
            }
        });
        let resp = JsonResponse::ok()
            .with_stats(stats_out)
            .with_diagnostics(diagnostics)
            .with_actions(actions);
        print_json(&resp)?;
    } else {
        println!("Integrity: {}", report.status);
        println!(
            "FTS: {} (missing {})",
            if consistency.fts_ok() { "ok" } else { "stale" },
            consistency.fts_missing
        );
        println!(
            "ANN: {} (missing {})",
            if consistency.ann_ok() { "ok" } else { "stale" },
            consistency.ann_missing
        );
        if ctx.config.ann_backend == "hnsw" {
            println!(
                "HNSW: {} (missing {})",
                if consistency.hnsw_ok() { "ok" } else { "stale" },
                consistency.hnsw_missing
            );
        }
        for action in actions {
            println!("Action: {action}");
        }
    }

    Ok(())
}

fn cmd_compact(json: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path(), StoreMode::ReadWrite)?;
    let integrity = store.integrity_check()?;
    let consistency = store.consistency_report()?;
    let hnsw_issue = ctx.config.ann_backend == "hnsw" && !consistency.hnsw_ok();
    if integrity.status != "ok" || !consistency.fts_ok() || !consistency.ann_ok() || hnsw_issue {
        anyhow::bail!(
            "compact aborted: integrity={}, fts_missing={}, ann_missing={}, hnsw_missing={}",
            integrity.status,
            consistency.fts_missing,
            consistency.ann_missing,
            consistency.hnsw_missing
        );
    }
    store.compact()?;

    if json {
        let resp = JsonResponse::ok().with_actions(vec![
            "delete tombstoned rows".to_string(),
            "vacuum".to_string(),
        ]);
        print_json(&resp)?;
    } else {
        println!("Compaction complete");
    }

    Ok(())
}

fn cmd_completions(shell: Shell) -> Result<()> {
    let mut cmd = Cli::command();
    let name = cmd.get_name().to_string();
    generate(shell, &mut cmd, name, &mut std::io::stdout());
    Ok(())
}

fn cmd_man() -> Result<()> {
    let cmd = Cli::command();
    let man = Man::new(cmd);
    let mut buf = Vec::new();
    man.render(&mut buf)?;
    std::io::stdout().write_all(&buf)?;
    Ok(())
}

fn parse_parser_hint(input: Option<&str>) -> Result<ingest::ParserHint> {
    let hint = input.unwrap_or("auto");
    let parsed = match hint {
        "auto" => ingest::ParserHint::Auto,
        "plain" => ingest::ParserHint::Plain,
        "markdown" => ingest::ParserHint::Markdown,
        "code" => ingest::ParserHint::Code,
        _ => anyhow::bail!("unknown parser hint: {hint}"),
    };
    Ok(parsed)
}

fn parse_lexical_mode(input: &str) -> Result<query::LexicalMode> {
    match input {
        "fts5" => Ok(query::LexicalMode::Fts5),
        "literal" => Ok(query::LexicalMode::Literal),
        _ => anyhow::bail!("unknown lexical mode: {input}"),
    }
}

fn validate_snapshot(snapshot: Option<String>) -> Result<Option<String>> {
    let Some(token) = snapshot else {
        return Ok(None);
    };
    let trimmed = token.trim();
    if trimmed.is_empty() {
        anyhow::bail!("snapshot token cannot be empty");
    }
    OffsetDateTime::parse(trimmed, &Rfc3339).context("parse snapshot token")?;
    Ok(Some(trimmed.to_string()))
}

fn load_filter(filter: Option<String>) -> Result<Option<String>> {
    let Some(value) = filter else {
        return Ok(None);
    };
    if let Some(path) = value.strip_prefix('@') {
        let text =
            std::fs::read_to_string(path).with_context(|| format!("read filter file {path}"))?;
        let trimmed = text.trim();
        if trimmed.is_empty() {
            anyhow::bail!("filter file {path} was empty");
        }
        Ok(Some(trimmed.to_string()))
    } else {
        Ok(Some(value))
    }
}

fn print_jsonl_response(resp: JsonResponse) -> Result<()> {
    let mut meta = serde_json::Map::new();
    meta.insert("type".into(), json!("meta"));
    meta.insert("ok".into(), json!(resp.ok));
    meta.insert("schema_version".into(), json!(resp.schema_version));
    if let Some(query) = resp.query {
        meta.insert("query".into(), json!(query));
    }
    if let Some(stats) = resp.stats {
        meta.insert("stats".into(), json!(stats));
    }
    if let Some(explain) = resp.explain {
        meta.insert("explain".into(), explain);
    }
    if let Some(diagnostics) = resp.diagnostics {
        meta.insert("diagnostics".into(), diagnostics);
    }
    if let Some(actions) = resp.actions {
        meta.insert("actions".into(), json!(actions));
    }
    if !resp.warnings.is_empty() {
        meta.insert("warnings".into(), json!(resp.warnings));
    }
    if let Some(error) = resp.error {
        meta.insert("error".into(), json!(error));
    }
    println!(
        "{}",
        serde_json::to_string(&serde_json::Value::Object(meta))?
    );

    if let Some(results) = resp.results {
        for item in results {
            let line = json!({ "type": "result", "item": item });
            println!("{}", serde_json::to_string(&line)?);
        }
    }
    if let Some(context) = resp.context {
        let line = json!({ "type": "context", "item": context });
        println!("{}", serde_json::to_string(&line)?);
    }
    Ok(())
}

fn memory_stats() -> MemoryStats {
    #[cfg(target_os = "linux")]
    {
        let mut stats = MemoryStats::default();
        if let Ok(text) = std::fs::read_to_string("/proc/self/statm") {
            let mut parts = text.split_whitespace();
            if let (Some(size), Some(rss)) = (parts.next(), parts.next()) {
                if let (Ok(size), Ok(rss)) = (size.parse::<u64>(), rss.parse::<u64>()) {
                    let page_size = 4096u64;
                    stats.virt_bytes = Some(size.saturating_mul(page_size));
                    stats.rss_bytes = Some(rss.saturating_mul(page_size));
                }
            }
        }
        return stats;
    }

    #[cfg(not(target_os = "linux"))]
    {
        MemoryStats::default()
    }
}
