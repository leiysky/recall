# Recall Usage Guide

This guide complements `recall --help` with a narrative walkthrough of the CLI.
It covers the full lifecycle: create a store, ingest content, retrieve results,
assemble context, and maintain/export the database.

## Quickstart
```
recall init .
recall add . --glob "**/*.{md,rs,ts,py}" --tag code
recall search "retry backoff" --k 8 --filter "doc.path GLOB '**/net/**'" --json
recall context "retry backoff" --budget-tokens 1200 --diversity 2
```

## Stores and Discovery
- `recall init <path>` creates `recall.db` in the target directory. The
  directory is created if it does not exist.
- All commands search for `recall.db` by walking up from the current directory.
  This lets you run Recall from nested folders without extra flags.
- Configuration is global (no per-project `recall.toml`). The file is optional
  and lives in the OS config directory. `recall init` prints the path.

## Ingesting Documents (`recall add`)
`recall add` accepts files or directories and builds chunks + embeddings.

Common flags:
- `--glob` include pattern (e.g. `"**/*.md"`).
- `--ignore` exclude pattern (repeatable).
- `--tag` logical tag (e.g. `docs`, `code`, `policy`).
- `--source` source label for audits or grouping.
- `--parser` hint for chunking: `auto|plain|markdown|code`.
- `--extract-meta` parse Markdown front matter and headers into `doc.meta.*`.
- `--mtime-only` skip unchanged files (fast re-indexing).
- `--json` emit stats and warnings in stable JSON.

Example:
```
recall add ./docs --glob "**/*.md" --tag docs --extract-meta --json
```

## Hybrid Search (`recall search`)
Search runs hybrid retrieval by default (semantic + lexical). You can force one
mode with `--bm25` (lexical) or `--vector` (semantic).

Key options:
- `--k` number of results (default: 8).
- `--filter` exact predicate; supports `@file` to load filters.
- `--lexical-mode` `fts5` (default) or `literal` for punctuation-heavy queries.
- `--snapshot` RFC3339 token for reproducible paging.
- `--explain` include scoring details and warnings.
- `--json` / `--jsonl` for machine-readable output.

Example:
```
recall search "rate limit" --k 12 --filter "doc.tag = 'docs'" --json
```

## Structured Queries (`recall query` + RQL)
Use RQL when you need structured filters, field selection, or doc-level results.
RQL can be passed inline (`--rql "..."`), from a file (`--rql @file`), or via
stdin (`--rql-stdin`).

Minimal shape:
```
FROM <table>
USING semantic("text") [, lexical("text")]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>]
SELECT <fields>;
```

Notes:
- `USING` is optional; without it queries are strict filters only.
- `FILTER` is exact and fields must be qualified (`doc.*` or `chunk.*`).
- `ORDER BY score` is meaningful only with `USING`.
- Legacy `SELECT ... FROM ...` is still accepted.

Useful fields to `SELECT`:
- Doc fields: `doc.id`, `doc.path`, `doc.mtime`, `doc.hash`, `doc.tag`,
  `doc.source`, `doc.meta.<key>`
- Chunk fields: `chunk.id`, `chunk.doc_id`, `chunk.offset`, `chunk.tokens`,
  `chunk.text`
- `score` (when `USING` is present)

Example:
```
recall query --rql "FROM chunk USING semantic('rate limit') \
  FILTER doc.path GLOB '**/api/**' LIMIT 6 SELECT chunk.text, doc.path, score;" \
  --json
```

## Context Assembly (`recall context`)
`recall context` builds a bounded context window from search results.

Options:
- `--budget-tokens` hard cap for output size.
- `--diversity` maximum chunks per doc (optional).
- `--format` `text` (default) or `json` (equivalent to `--json`).
- `--filter`, `--lexical-mode`, `--snapshot`, `--explain` behave like `search`.

Example:
```
recall context "deployment steps" --budget-tokens 1000 --diversity 2 --format json
```

## Filters (FEL)
- Fields must be qualified: `doc.*` or `chunk.*`.
- Operators: `=`, `!=`, `<`, `<=`, `>`, `>=`, `LIKE`, `GLOB`, `IN`.
- `LIKE` uses `%` and `_`; `GLOB` uses `*`, `?`, and `**`.
- Use `--filter @file` for complex predicates.

Examples:
```
--filter "doc.tag = 'docs' AND doc.path GLOB '**/api/**'"
--filter @filters.txt
```

## Determinism and Snapshots
- Ordering is deterministic for identical inputs and store state.
- JSON outputs include `stats.snapshot` for reproducible paging.
- Pass `--snapshot <token>` to freeze results across runs.

## Maintenance Commands
- `recall rm <id|path...>` tombstones documents (use `--purge` to compact).
- `recall stats` shows corpus and database stats.
- `recall doctor` checks integrity; `--fix` applies safe repairs.
- `recall compact` removes tombstones and vacuums the database.
- `recall export --out FILE` and `recall import FILE` for portability.
- `recall completions <shell>` generates shell completions.
- `recall guide` prints this guide.

## JSON Output
Most commands support `--json` with a stable schema version. Errors are also
JSON and include `error.code` and `error.message`. Use `--jsonl` for streaming
large result sets.

## Configuration (Optional)
Recall reads a single optional config file from the OS config directory. The
path is printed by `recall init`. Example:
```
store_path = "recall.db"
chunk_tokens = 256
overlap_tokens = 32
embedding = "hash"
embedding_dim = 256
ann_backend = "lsh"
ann_bits = 16
ann_seed = 42
bm25_weight = 0.5
vector_weight = 0.5
max_limit = 1000
```
