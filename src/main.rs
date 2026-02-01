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
mod store;
mod transfer;

use std::path::PathBuf;

use anyhow::Context as _;
use anyhow::Result;
use clap::Parser;

use crate::cli::Cli;
use crate::cli::Commands;
use crate::config::Config;
use crate::config::ConfigCtx;
use crate::output::JsonResponse;
use crate::output::StatsOut;
use crate::output::print_json;

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
                args.json,
            ),
            args.json,
        ),
        Commands::Rm(args) => handle_result(cmd_rm(args.targets, args.purge, args.json), args.json),
        Commands::Query(args) => {
            handle_result(cmd_query(args.rql, args.json, args.explain), args.json)
        }
        Commands::Context(args) => handle_result(
            cmd_context(
                args.query,
                args.budget_tokens,
                args.diversity,
                args.filter,
                args.explain,
                args.json,
            ),
            args.json,
        ),
        Commands::Stats { json } => handle_result(cmd_stats(json), json),
        Commands::Doctor { json } => handle_result(cmd_doctor(json), json),
        Commands::Compact { json } => handle_result(cmd_compact(json), json),
        Commands::Export(args) => handle_result(cmd_export(args.out, args.json), args.json),
        Commands::Import(args) => handle_result(cmd_import(args.path, args.json), args.json),
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

    let config_path = root.join("recall.toml");
    if config_path.exists() {
        anyhow::bail!("recall.toml already exists at {}", config_path.display());
    }

    let config = Config::default();
    config::write_config(&config_path, &config)?;

    let store_path = root.join(&config.store_path);
    store::Store::init(&store_path)?;

    println!("Initialized Recall store at {}", store_path.display());
    Ok(())
}

fn cmd_add(
    paths: Vec<PathBuf>,
    glob: Option<String>,
    tag: Option<String>,
    source: Option<String>,
    mtime_only: bool,
    ignore: Vec<String>,
    json: bool,
) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;

    let opts = ingest::IngestOptions {
        glob,
        tag,
        source,
        mtime_only,
        ignore,
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
    let store = store::Store::open(&ctx.store_path())?;
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
        });
        print_json(&resp)?;
    } else {
        println!("Removed {removed} docs (tombstoned)");
    }
    Ok(())
}

fn cmd_search(
    query: String,
    k: usize,
    bm25: bool,
    vector: bool,
    filter: Option<String>,
    explain: bool,
    json: bool,
) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;

    let opts = query::SearchOptions {
        k,
        use_lexical: bm25 || !vector,
        use_semantic: vector || !bm25,
        filter,
        explain,
    };

    let result = query::search_chunks(&store, &ctx.config, &query, opts)?;

    if json {
        let mut resp = JsonResponse::ok().with_query(
            &query,
            None,
            result.filter_string(),
            result.limit,
            result.offset,
        );
        resp = resp.with_results(result.to_json_results());
        resp = resp.with_stats(result.stats);
        if explain {
            resp = resp.with_warnings(result.explain_warnings);
        }
        print_json(&resp)?;
    } else {
        query::print_table(&result, explain);
    }

    Ok(())
}

fn cmd_query(rql_input: String, json: bool, explain: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;

    let rql_text = if let Some(path) = rql_input.strip_prefix('@') {
        std::fs::read_to_string(path).with_context(|| format!("read RQL file {path}"))?
    } else {
        rql_input
    };

    let result = query::run_rql(&store, &ctx.config, &rql_text, explain)?;

    if json {
        let mut resp = JsonResponse::ok().with_query(
            "",
            Some(&rql_text),
            result.filter_string(),
            result.limit,
            result.offset,
        );
        resp = resp.with_results(result.to_json_results());
        resp = resp.with_stats(result.stats);
        if explain {
            resp = resp.with_warnings(result.explain_warnings);
        }
        print_json(&resp)?;
    } else {
        query::print_table(&result, explain);
    }

    Ok(())
}

fn cmd_export(out: Option<PathBuf>, json: bool) -> Result<()> {
    if json && out.is_none() {
        anyhow::bail!("--json requires --out for export");
    }
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;

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
        });
        print_json(&resp)?;
    }

    Ok(())
}

fn cmd_import(path: PathBuf, json: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;
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
        });
        print_json(&resp)?;
    }
    Ok(())
}

fn cmd_context(
    query: String,
    budget_tokens: usize,
    diversity: Option<usize>,
    filter: Option<String>,
    explain: bool,
    json: bool,
) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;

    let opts = query::SearchOptions {
        k: 100,
        use_lexical: true,
        use_semantic: true,
        filter,
        explain,
    };

    let search = query::search_chunks(&store, &ctx.config, &query, opts)?;
    let assembled = context::assemble(&search, budget_tokens, diversity);

    if json {
        let mut resp = JsonResponse::ok().with_query(
            &query,
            None,
            search.filter_string(),
            search.limit,
            search.offset,
        );
        resp = resp.with_context(assembled.to_json());
        resp = resp.with_stats(search.stats);
        print_json(&resp)?;
    } else {
        println!("{}", assembled.text);
    }

    Ok(())
}

fn cmd_stats(json: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;
    let stats = store.stats()?;
    let stats_out = StatsOut {
        took_ms: 0,
        total_hits: 0,
        doc_count: Some(stats.doc_count),
        chunk_count: Some(stats.chunk_count),
        db_size_bytes: Some(stats.db_size_bytes),
        snapshot: store.snapshot_token().ok(),
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

fn cmd_doctor(json: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;
    let report = store.integrity_check()?;
    let stats_out = StatsOut {
        took_ms: 0,
        total_hits: 0,
        doc_count: Some(report.stats.doc_count),
        chunk_count: Some(report.stats.chunk_count),
        db_size_bytes: Some(report.stats.db_size_bytes),
        snapshot: store.snapshot_token().ok(),
    };

    if json {
        let resp = JsonResponse::ok().with_stats(stats_out);
        print_json(&resp)?;
    } else {
        println!("Integrity: {}", report.status);
    }

    Ok(())
}

fn cmd_compact(json: bool) -> Result<()> {
    let ctx = ConfigCtx::load_from_cwd()?;
    let store = store::Store::open(&ctx.store_path())?;
    store.compact()?;

    if json {
        let resp = JsonResponse::ok();
        print_json(&resp)?;
    } else {
        println!("Compaction complete");
    }

    Ok(())
}
