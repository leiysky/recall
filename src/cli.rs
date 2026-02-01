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

#[derive(Parser, Debug)]
#[command(name = "recall", version, about = "CLI-first hybrid search database")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Initialize a new Recall store
    Init {
        /// Path to the store directory
        path: Option<PathBuf>,
    },

    /// Add documents to the store
    Add(AddArgs),

    /// Remove documents from the store
    Rm(RmArgs),

    /// Hybrid search
    Search(SearchArgs),

    /// Run an RQL query
    Query(QueryArgs),

    /// Assemble a context window
    Context(ContextArgs),

    /// Show stats
    Stats {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },

    /// Run integrity checks
    Doctor {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },

    /// Compact the database
    Compact {
        /// Output JSON
        #[arg(long)]
        json: bool,
    },

    /// Export the database as JSONL
    Export(ExportArgs),

    /// Import a JSONL export
    Import(ImportArgs),
}

#[derive(Args, Debug)]
pub struct AddArgs {
    /// Files or directories to add
    pub paths: Vec<PathBuf>,

    /// Glob to include
    #[arg(long)]
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
    #[arg(long)]
    pub ignore: Vec<String>,

    /// Parser hint (not yet used)
    #[arg(long)]
    pub parser: Option<String>,

    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct SearchArgs {
    /// Query text
    pub query: String,

    /// Top-k results
    #[arg(long, default_value_t = 8)]
    pub k: usize,

    /// Lexical search only
    #[arg(long)]
    pub bm25: bool,

    /// Vector search only
    #[arg(long)]
    pub vector: bool,

    /// Exact filter expression
    #[arg(long)]
    pub filter: Option<String>,

    /// Include explain output
    #[arg(long)]
    pub explain: bool,

    /// Output JSON
    #[arg(long)]
    pub json: bool,
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
    #[arg(long)]
    pub rql: String,

    /// Include explain output
    #[arg(long)]
    pub explain: bool,

    /// Output JSON
    #[arg(long)]
    pub json: bool,
}

#[derive(Args, Debug)]
pub struct ContextArgs {
    /// Query text
    pub query: String,

    /// Token budget
    #[arg(long, default_value_t = 1200)]
    pub budget_tokens: usize,

    /// Max chunks per doc
    #[arg(long)]
    pub diversity: Option<usize>,

    /// Output format (reserved)
    #[arg(long)]
    pub format: Option<String>,

    /// Exact filter expression
    #[arg(long)]
    pub filter: Option<String>,

    /// Include explain output
    #[arg(long)]
    pub explain: bool,

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
