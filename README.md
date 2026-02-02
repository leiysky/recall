# Recall

Recall is a CLI-first, hybrid search database for AI agents working with large context. It is designed as “SQLite for document data” with deterministic retrieval, exact filtering, and semantic search.

## Highlights
- CLI and RQL are the stable, top-level interfaces.
- Single-file local store (`recall.db`) backed by SQLite + FTS5.
- Hybrid retrieval: lexical (FTS5 bm25) + semantic embeddings.
- Deterministic ordering and context assembly with token budgets and provenance.
- JSON outputs with schema validation and golden tests.
- Export/import for reproducible datasets.

## Core Principles
- Determinism over magic: identical inputs + store state yield identical outputs.
- Hybrid retrieval with strict filters: semantic + lexical, filters are hard constraints.
- Local-first, zero-ops: one file, offline by default, no required services.
- Context as a managed resource: hard budgets, deterministic packing, provenance.
- AI-native interface: stable RQL + CLI + JSON outputs.

## What Recall Is For
Recall is a local, deterministic retrieval layer for agents and tools that need
repeatable access to large, evolving corpora. Think of it as “SQLite for
document data” with semantic search and exact filtering.

Use Recall when you want:
- A single-file, portable index you can move with a repo or dataset.
- Deterministic results across runs for agent workflows.
- A CLI-first surface you can script and automate.
- Hybrid search (semantic + lexical) without a hosted service.

## Usage Scenarios
### 1) Codebase Retrieval for Agents
Keep a `recall.db` per repo and use it for tool calls.
```
recall init .
recall add . --glob "**/*.{md,rs,ts,py}" --tag code
recall search "retry backoff" --filter "doc.path GLOB \"**/net/**\"" --json
```

### 2) Product or Policy Knowledge Base
Maintain a curated corpus with tags and sources for audits.
```
recall init ./kb
recall add ./policies --glob "**/*.md" --tag policy --source "handbook"
recall query --rql "SELECT doc.path FROM doc FILTER doc.tag = \"policy\" LIMIT 20;"
```

### 3) Incident Response / Runbooks
Record the snapshot token for audit trails and export for sharing.
```
recall search "rollback steps" --k 12 --json
recall export --out incident-2026-02-01.jsonl --json
```

### 4) Research Notes and Papers
Use tags and filters to keep retrieval scoped and deterministic.
```
recall add ./papers --glob "**/*.txt" --tag research
recall context "evaluation methodology" --budget-tokens 1200 --diversity 2
```

### 5) Agent Tooling Pipelines
Integrate `recall query --json` into pipelines for reproducible retrieval.
```
recall query --rql "SELECT chunk.text, score FROM chunk USING semantic('SLO') LIMIT 6;" --json
```

## Install (Local)
```
cargo build --release
```
The binary will be at `target/release/recall`.

## Quickstart
```
recall init .
recall add ./docs --glob "**/*.md" --tag docs
recall search "retry policy" --k 8 --filter "doc.tag = 'docs'" --json
recall context "how we handle retries" --budget-tokens 1200 --diversity 2
```

## CLI Commands
```
recall init [path]
recall add <path...> [--glob ...] [--tag ...] [--source ...] [--mtime-only] [--ignore ...]
recall rm <doc_id|path...> [--purge]
recall search <query> [--k N] [--bm25] [--vector] [--filter ...] [--explain] [--json]
recall query --rql <string|@file> [--explain] [--json]
recall context <query> [--budget-tokens N] [--diversity N] [--json]
recall stats [--json]
recall doctor [--json]
recall compact [--json]
recall export [--out FILE] [--json]
recall import <FILE> [--json]
```

## RQL (Recall Query Language)
Minimal shape:
```
SELECT <fields> FROM <table>
USING semantic(<text>) [, lexical(<text>)]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>];
```

Notes:
- `USING` enables semantic/lexical search; without it, queries are strict filters only.
- `FILTER` is exact and fields must be qualified (`doc.*`, `chunk.*`).
- Unknown `SELECT` fields are ignored in v0.1 (permissive).

Example:
```
SELECT chunk.text, chunk.doc_id, score FROM chunk
USING semantic("rate limit") , lexical("429")
FILTER doc.path GLOB "**/api/**" AND chunk.tokens <= 256
ORDER BY score DESC
LIMIT 8;
```

## Filter Expression Language (FEL)
- `FILTER` fields must be qualified (`doc.*`, `chunk.*`).
- `LIKE` uses SQL patterns (`%`, `_`).
- `GLOB` uses glob patterns (`*`, `?`, `**`).

Example:
```
FILTER doc.tag = "docs" AND doc.path GLOB "**/api/**"
```

## JSON Output
All commands support `--json` with a stable schema (including `schema_version`). Errors are machine-parseable and include `code` and `message`. A `stats.snapshot` token is provided as a reproducibility hint; snapshot paging is planned.

## Export / Import
Use JSONL for portability:
```
recall export --out recall.jsonl --json
recall import recall.jsonl --json
```

## Development
Use the `./x` helper for consistent workflows:
```
./x fmt
./x test
./x clippy -- -D warnings
```

## Workflows
See `WORKFLOWS.md` for temporary (volatile) workflows and end-to-end examples
of using Recall to develop Recall.

## Roadmap
See `ROADMAP.md`.

## License
Apache-2.0. See `LICENSE`.
