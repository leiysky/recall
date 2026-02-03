# Recall

Recall is a CLI-first, hybrid search database for AI agents working with large context. It is designed as “SQLite for document data” with deterministic retrieval, exact filtering, and semantic search.

## Highlights
- CLI and RQL are the stable, top-level interfaces.
- Single-file local data store (`recall.db`) backed by SQLite + FTS5; config (`recall.toml`) and lock files are separate.
- Hybrid retrieval: lexical (FTS5 bm25) + semantic embeddings.
- Deterministic ordering and context assembly with token budgets and provenance.
- JSON outputs with schema validation and golden tests.
- Export/import for reproducible datasets.

## Core Principles
Canonical definitions live in `DESIGN.md` under Core Principles.
- Determinism over magic: identical inputs + store state yield identical outputs, including ordering and context assembly.
- Hybrid retrieval with strict filters: semantic + lexical ranking is allowed, but FILTER constraints are exact and non-negotiable.
- Local-first, zero-ops: data store is a single file (`recall.db`); config/lock are separate, offline by default, no required services.
- Context as a managed resource: hard token budgets, deterministic packing, and provenance for every chunk.
- AI-native interface: CLI and stable RQL are the source of truth; JSON outputs are stable for tooling.

## What Recall Is For
Recall is a local, deterministic retrieval layer for agents and tools that need
repeatable access to large, evolving corpora. Think of it as “SQLite for
document data” with semantic search and exact filtering.

Use Recall when you want:
- A portable index where the data store is a single file (`recall.db`) you can move with a repo or dataset.
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
recall query --rql "FROM doc FILTER doc.tag = \"policy\" LIMIT 20 SELECT doc.path;"
```

### 3) Incident Response / Runbooks
Record the snapshot token for audit trails and reproducible paging.
```
recall search "rollback steps" --k 12 --json
# Copy stats.snapshot from the JSON output, then re-run deterministically:
recall search "rollback steps" --k 12 --snapshot TOKEN --json
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
recall query --rql "FROM chunk USING semantic('SLO') LIMIT 6 SELECT chunk.text, score;" --json
```

## Install (Local)
```
cargo build --release
```
The binary will be at `target/release/recall`.

### Install (Cargo)
```
cargo install --path .
```
This installs the `recall` binary into your Cargo bin directory.

## Quickstart
```
recall init .
recall add ./docs --glob "**/*.md" --tag docs
recall search "retry policy" --k 8 --filter "doc.tag = 'docs'" --json
recall context "how we handle retries" --budget-tokens 1200 --diversity 2
```

## Shell Completions and Man Page
Generate completions:
```
recall completions bash > /tmp/recall.bash
recall completions zsh > /tmp/_recall
recall completions fish > /tmp/recall.fish
```

Generate a man page:
```
recall man > /tmp/recall.1
```

## CLI Commands
```
recall init [path]
recall add <path...> [--glob ...] [--tag ...] [--source ...] [--mtime-only] [--ignore ...] [--parser auto|plain|markdown|code] [--extract-meta] [--json]
recall rm <doc_id|path...> [--purge] [--json]
recall search <query> [--k N] [--bm25] [--vector] [--filter ...|@file] [--lexical-mode fts5|literal] [--snapshot TOKEN] [--explain] [--json|--jsonl]
recall query --rql <string|@file> [--rql-stdin] [--lexical-mode fts5|literal] [--snapshot TOKEN] [--explain] [--json|--jsonl]
recall context <query> [--budget-tokens N] [--diversity N] [--format text|json] [--filter ...|@file] [--lexical-mode fts5|literal] [--snapshot TOKEN] [--explain] [--json]
recall stats [--json]
recall doctor [--json] [--fix]
recall compact [--json]
recall export [--out FILE] [--json]
recall import <FILE> [--json]
recall completions <shell>
recall man
```

## Metadata Extraction (Optional)
Use `--extract-meta` to parse deterministic header metadata from Markdown files
and filter on it in RQL:
```
recall add ./docs --glob "**/*.md" --extract-meta
recall search "migration" --filter "doc.meta.status = 'active'" --json
```

## RQL (Recall Query Language)
Minimal shape:
```
FROM <table>
USING semantic(<text>) [, lexical(<text>)]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>]
SELECT <fields>;
```

Notes:
- `USING` enables semantic/lexical search; without it, queries are strict filters only.
- `FILTER` is exact and fields must be qualified (`doc.*`, `chunk.*`).
- Unknown `SELECT` fields are ignored in v0.1 (permissive).
- Legacy `SELECT ... FROM ...` syntax is still accepted for compatibility.

Example:
```
FROM chunk
USING semantic("rate limit"), lexical("429")
FILTER doc.path GLOB "**/api/**" AND chunk.tokens <= 256
ORDER BY score DESC
LIMIT 8
SELECT chunk.text, chunk.doc_id, score;
```

## Filter Expression Language (FEL)
- `FILTER` fields must be qualified (`doc.*`, `chunk.*`).
- Metadata keys are available via `doc.meta.<key>` (keys are normalized to lowercase).
- `LIKE` uses SQL patterns (`%`, `_`).
- `GLOB` uses glob patterns (`*`, `?`, `**`).

Example:
```
FILTER doc.tag = "docs" AND doc.path GLOB "**/api/**"
```

## JSON Output
Most commands support `--json` with a stable schema (including `schema_version`); `recall init`, `recall completions`, and `recall man` are plain text only. Errors are machine-parseable and include `code` and `message`. A `stats.snapshot` token is provided as a reproducibility hint, and `--snapshot` accepts tokens for deterministic pagination. Use `--jsonl` for streaming large result sets from `recall search` and `recall query`.

## Export / Import
Use JSONL for portability:
```
recall export --out recall.jsonl --json
recall import recall.jsonl --json
```

## Development
Note: the files referenced below (including `./x`, `AGENTS.md`, `ROADMAP.md`, and `docs/`) live in the source checkout. Release archives built via `scripts/package_release.sh` include them; binary-only installs may not.

Use the `./x` helper for consistent workflows:
```
./x fmt
./x test
./x clippy -- -D warnings
```

## Benchmarks
See `docs/benchmarks/README.md` for the benchmark dataset, baseline numbers,
and regression thresholds.

## Workflows
See `AGENTS.md` → "Inlined Reference Documents" → `WORKFLOWS.md` for temporary
(volatile) workflows and end-to-end examples of using Recall to develop Recall.

## Roadmap
See `ROADMAP.md`.

## Compatibility
See `docs/COMPATIBILITY.md` for the v1.0 interface freeze, upgrade guarantees,
and compatibility matrix.

## Releases
See `docs/RELEASE.md` for the release checklist and versioning policy. v1.0
release notes draft lives in `docs/history/changes/CHANGE-2026-02-02-v1-0-release.md`.

## License
Apache-2.0. See `LICENSE`.
