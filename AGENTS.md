# Recall Agent Guide

Purpose: help AI agents use Recall as a local, SQLite-like document database with semantic search and exact filtering.

## Development Rules (Enforced)
When working on Recall, agents must follow `DEVELOPMENT_RULES.md`.

## Engineering Handbook
Iteration and validation guidance lives in `HANDBOOK.md`. Follow it for
branching, commits, testing, and project structure.
For the consolidated end-to-end workflow (including git steps), see
`WORKFLOWS.md` → "Complete Workflow (Merged Summary)".

## Roadmap
Current roadmap and milestones live in `ROADMAP.md`.

## Core Concepts
- Recall stores two logical tables: `doc` and `chunk`.
- The store is a single local file (SQLite-like): `recall.db`.
- Semantic search is explicit via `semantic("...")` in RQL or `recall search`.
- Exact filtering is explicit via `FILTER` in RQL or `--filter` in CLI.
- Retrieval is deterministic in v0.1; reranker stages are future work.

## Recommended Workflow
1. `recall init` once per repository or dataset.
2. `recall add` to ingest files (prefer narrow globs).
3. Use `recall search` for quick interactive queries.
4. Use `recall query --rql` for precise retrieval and filtering.
5. Use `recall context` to build the final context window for an agent.

## RQL (Recall Query Language)
RQL is a stable, AI-friendly SQL-like subset. It is designed to be predictable and easy to generate.

### Minimal Shape
```
SELECT <fields> FROM <table>
USING semantic(<text>) [, lexical(<text>)]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>];
```

### Guidelines
- Always include `USING semantic("...")` when you need semantic search.
- Use `FILTER` for exact constraints (paths, tags, dates).
- `FILTER` fields must be qualified (`doc.*`, `chunk.*`).
- Prefer `GLOB` for filesystem-like path patterns and `LIKE` for SQL `%/_` patterns.
- Prefer `LIMIT` for bounded results.
- If you need chunk text, query `chunk.text` from `chunk`.
- If you only need document metadata, query the `doc` table.

### Field Catalog (Initial)
- `doc.id`, `doc.path`, `doc.mtime`, `doc.hash`, `doc.tag`, `doc.source`.
- `chunk.id`, `chunk.doc_id`, `chunk.offset`, `chunk.tokens`, `chunk.text`.
Note: `doc.size` is stored but not exposed in RQL v0.1.

### Example Queries
```
SELECT chunk.text, chunk.doc_id, score FROM chunk
USING semantic("retry backoff")
FILTER doc.tag = "docs" AND doc.path GLOB "**/api/**"
LIMIT 6;

SELECT doc.id, doc.path FROM doc
FILTER doc.tag IN ("policy", "security")
ORDER BY doc.mtime DESC
LIMIT 20;
```

## CLI Patterns
- Interactive search:
  - `recall search "query" --k 8 --filter "doc.tag = 'docs'"`
- Structured query:
  - `recall query --rql "SELECT chunk.text FROM chunk USING semantic('foo') LIMIT 5;"`
- Context assembly:
  - `recall context "query" --budget-tokens 1200 --diversity 2 --json`
- Export/import:
  - `recall export --out recall.jsonl --json`
  - `recall import recall.jsonl --json`

## Agent Output Contract
- If no results are returned, say so explicitly and suggest broadening the query.
- When providing citations, include document path and chunk offsets from Recall output.
- Do not invent fields or query functions; keep to the RQL catalog.
- Avoid placing secrets in Recall; redact API keys or credentials from outputs.
- In `--json` outputs, `query.limit` and `query.offset` report the effective values
  after defaults (RQL `LIMIT`/`OFFSET`, `--k`, or context search limits).

## Error Handling
- If RQL fails to parse, simplify the query and retry.
- If semantic search is unavailable, fall back to lexical search and exact filters.
- If lexical search fails due to FTS5 syntax, Recall sanitizes the query; consider
  removing punctuation-heavy tokens if results are unexpected.
