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

use std::path::PathBuf;

use clap::Args;
use clap::Parser;
use clap::Subcommand;
use clap_complete::Shell;

#[derive(Parser, Debug)]
#[command(
    name = "recall",
    version,
    about = "CLI-first hybrid search database",
    long_about = "Recall is a local, single-file database for deterministic retrieval over documents.\nIt supports hybrid semantic + lexical search, exact filters, and stable JSON output.\nCommands discover recall.toml by walking up from the current directory.",
    after_help = "Examples:\n  recall init .\n  recall add . --glob \"**/*.{md,rs}\" --tag code\n  recall search \"retry backoff\" --filter \"doc.path GLOB '**/net/**'\" --json\n  recall query --rql \"FROM chunk USING semantic('vector index') LIMIT 6 SELECT chunk.text;\"\n  recall context \"ordering rules\" --budget-tokens 800 --diversity 2\n  recall search \"foo\" --filter @filters.txt\n  recall query --rql @query.rql --json\n\nNotes:\n  - FILTER is exact; fields must be qualified (doc.* or chunk.*).\n  - RQL supports FROM-first (preferred) and legacy SELECT-first forms.\n  - Snapshot tokens use RFC3339 (e.g. 2026-02-02T00:00:00Z).\n  - Use --json/--jsonl for machine-readable output."
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new Recall store
    #[command(
        long_about = "Create recall.toml and a recall.db store in the target directory.\nAfter init, all commands discover recall.toml by walking up from the current directory.",
        after_help = "Examples:\n  recall init .\n  recall init /path/to/store"
    )]
    Init {
        /// Path to the store directory
        path: Option<PathBuf>,
    },

    /// Add documents to the store
    #[command(
        long_about = "Ingest files or directories into the store.\nUse --glob/--ignore to control scope and --extract-meta for Markdown metadata.",
        after_help = "Examples:\n  recall add . --glob \"**/*.{md,rs}\" --tag code\n  recall add ./docs --glob \"**/*.md\" --extract-meta"
    )]
    Add(AddArgs),

    /// Remove documents from the store
    #[command(
        long_about = "Remove documents by path or ID. Removals are tombstoned unless --purge is set.",
        after_help = "Examples:\n  recall rm ./docs/old.md\n  recall rm 123 456 --purge"
    )]
    Rm(RmArgs),

    /// Hybrid search
    #[command(
        long_about = "Run hybrid search over chunks using semantic + lexical retrieval.\nUse --bm25 or --vector to force a single mode, and --filter for exact constraints.",
        after_help = "Examples:\n  recall search \"rate limit\" --k 12\n  recall search \"429\" --bm25 --filter \"doc.tag = 'docs'\"\n  recall search \"retry\" --vector --snapshot 2026-02-02T00:00:00Z"
    )]
    Search(SearchArgs),

    /// Run an RQL query
    #[command(
        long_about = "Execute a structured RQL query. Use --rql @file or --rql-stdin for long queries.",
        after_help = "Examples:\n  recall query --rql \"FROM doc FILTER doc.tag = 'docs' LIMIT 10 SELECT doc.path;\"\n  recall query --rql @query.rql --json\n  cat query.rql | recall query --rql-stdin --json"
    )]
    Query(QueryArgs),

    /// Assemble a context window
    #[command(
        long_about = "Build a bounded context window from search results with a token budget.\nUse --diversity to cap chunks per doc. Use --format json or --json for structured output.",
        after_help = "Examples:\n  recall context \"ordering rules\" --budget-tokens 800\n  recall context \"retry policy\" --budget-tokens 1200 --diversity 2 --format json"
    )]
    Context(ContextArgs),

    /// Show stats
    Stats {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },

    /// Run integrity checks
    #[command(
        long_about = "Verify store integrity and consistency. Use --fix for safe repair actions.",
        after_help = "Examples:\n  recall doctor\n  recall doctor --fix --json"
    )]
    Doctor {
        /// Output JSON
        #[arg(long)]
        json: bool,
        /// Attempt safe repairs
        #[arg(long)]
        fix: bool,
    },

    /// Compact the database
    #[command(
        long_about = "Compact the database and remove tombstones.",
        after_help = "Examples:\n  recall compact\n  recall compact --json"
    )]
    Compact {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },

    /// Export the database as JSONL
    #[command(
        long_about = "Export the database to JSONL for portability or backups.",
        after_help = "Examples:\n  recall export --out recall.jsonl --json"
    )]
    Export(ExportArgs),

    /// Import a JSONL export
    #[command(
        long_about = "Import a JSONL export into the current store.",
        after_help = "Examples:\n  recall import recall.jsonl --json"
    )]
    Import(ImportArgs),

    /// Generate shell completions
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: Shell,
    },

    /// Generate a man page
    Man,
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Files or directories to add
    pub paths: Vec<PathBuf>,

    /// Glob to include
    #[arg(long, help = "Include glob pattern")]
    pub glob: Option<String>,

    /// Tag for documents
    #[arg(long)]
    pub tag: Option<String>,

    /// Source label
    #[arg(long)]
    pub source: Option<String>,

    /// Skip files if mtime unchanged
    #[arg(long)]
    pub mtime_only: bool,

    /// Ignore globs
    #[arg(long, help = "Exclude glob pattern (repeatable)")]
    pub ignore: Vec<String>,

    /// Parser hint (auto|plain|markdown|code)
    #[arg(long, value_parser = ["auto", "plain", "markdown", "code"])]
    pub parser: Option<String>,

    /// Extract metadata from markdown headers/front matter
    #[arg(long)]
    pub extract_meta: bool,

    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Query text
    pub query: String,

    /// Top-k results
    #[arg(long, default_value_t = 8, help = "Number of results to return")]
    pub k: usize,

    /// Disable semantic search (lexical only)
    #[arg(long)]
    pub bm25: bool,

    /// Disable lexical search (vector only)
    #[arg(long)]
    pub vector: bool,

    /// Exact filter expression
    #[arg(
        long,
        long_help = "Exact filter expression. Use @file to load from a file. Fields must be qualified (doc.* or chunk.*)."
    )]
    pub filter: Option<String>,

    /// Include explain output
    #[arg(long)]
    pub explain: bool,

    /// Lexical parsing mode (fts5|literal)
    #[arg(long, default_value = "fts5", value_parser = ["fts5", "literal"])]
    pub lexical_mode: String,

    /// Snapshot token for reproducible paging
    #[arg(
        long,
        long_help = "Snapshot token in RFC3339 format (e.g. 2026-02-02T00:00:00Z) for reproducible paging."
    )]
    pub snapshot: Option<String>,

    /// Output JSON
    #[arg(long)]
    pub json: bool,

    /// Output JSON Lines
    #[arg(long, conflicts_with = "json", help = "Stream results as JSONL")]
    pub jsonl: bool,
}

#[derive(Args, Debug)]
pub struct RmArgs {
    /// Document IDs or paths to remove
    pub targets: Vec<String>,

    /// Force compaction after removal
    #[arg(long)]
    pub purge: bool,

    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct QueryArgs {
    /// RQL string or @file
    #[arg(
        long,
        required_unless_present = "rql_stdin",
        long_help = "RQL string or @file path. Use --rql-stdin to read from stdin."
    )]
    pub rql: Option<String>,

    /// Read RQL from stdin
    #[arg(long, conflicts_with = "rql")]
    pub rql_stdin: bool,

    /// Include explain output
    #[arg(long)]
    pub explain: bool,

    /// Lexical parsing mode (fts5|literal)
    #[arg(long, default_value = "fts5", value_parser = ["fts5", "literal"])]
    pub lexical_mode: String,

    /// Snapshot token for reproducible paging
    #[arg(
        long,
        long_help = "Snapshot token in RFC3339 format (e.g. 2026-02-02T00:00:00Z) for reproducible paging."
    )]
    pub snapshot: Option<String>,

    /// Output JSON
    #[arg(long)]
    pub json: bool,

    /// Output JSON Lines
    #[arg(long, conflicts_with = "json", help = "Stream results as JSONL")]
    pub jsonl: bool,
}

#[derive(Args, Debug)]
pub struct ContextArgs {
    /// Query text
    pub query: String,

    /// Token budget
    #[arg(long, default_value_t = 1200)]
    pub budget_tokens: usize,

    /// Max chunks per doc
    #[arg(long, help = "Cap chunks per doc (default: unlimited)")]
    pub diversity: Option<usize>,

    /// Output format (text|json)
    #[arg(
        long,
        value_parser = ["text", "json"],
        long_help = "Output format. --format json is equivalent to --json; --format text disables JSON output."
    )]
    pub format: Option<String>,

    /// Exact filter expression
    #[arg(
        long,
        long_help = "Exact filter expression. Use @file to load from a file. Fields must be qualified (doc.* or chunk.*)."
    )]
    pub filter: Option<String>,

    /// Include explain output
    #[arg(long)]
    pub explain: bool,

    /// Lexical parsing mode (fts5|literal)
    #[arg(long, default_value = "fts5", value_parser = ["fts5", "literal"])]
    pub lexical_mode: String,

    /// Snapshot token for reproducible paging
    #[arg(
        long,
        long_help = "Snapshot token in RFC3339 format (e.g. 2026-02-02T00:00:00Z) for reproducible paging."
    )]
    pub snapshot: Option<String>,

    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ExportArgs {
    /// Output file (defaults to stdout)
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Output JSON stats (requires --out)
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ImportArgs {
    /// Input file (JSONL)
    pub path: PathBuf,

    /// Output JSON stats
    #[arg(long)]
    pub json: bool,
}
