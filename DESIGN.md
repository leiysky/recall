# Recall Design Doc

Date: 2026-02-02
Status: Draft (principles-first)

## Product Summary
Recall is a local, single-file, CLI-first document store for AI agents. It provides deterministic, explainable hybrid retrieval with strict filters and builds token-budgeted context windows via a stable RQL interface.

## Core Principles
1. Determinism over magic: identical inputs and store state produce identical outputs.
2. Hybrid retrieval with strict filters: semantic + lexical, with exact filters as hard constraints.
3. Local-first, zero-ops: one file, offline by default, no required services.
4. Context as a managed resource: hard budgets, deterministic packing, provenance.
5. AI-native interface: stable RQL + CLI as the source of truth, with JSON output for tooling.

## Scope (v0.1)
- Single-file store `recall.db` (SQLite-backed).
- CLI: `init`, `add`, `rm`, `search`, `query`, `context`, plus `stats`, `doctor`, `compact`.
- Hybrid retrieval: lexical (FTS5 BM25) + semantic embeddings with explicit weights.
- Deterministic ordering and tie-breaks; `--explain` for scoring stages.
- Budgeted context assembly with provenance and optional diversity cap.
- Stable `--json` output with schema versioning.
- JSONL export/import for portability.

## Non-goals
- Hosted multi-tenant service.
- Multi-writer OLTP concurrency.
- Complex analytics SQL.
- Real-time collaborative editing.

## Interfaces

### CLI (source of truth)
- `recall init [path]`
- `recall add <path...>`
- `recall rm <doc_id|path...>`
- `recall search <query>`
- `recall query --rql <string|@file>`
- `recall context <query>`
- `recall stats`, `recall doctor`, `recall compact`
- `recall export`, `recall import`

### RQL (AI-native)
```
SELECT <fields> FROM <table>
USING semantic(<text>) [, lexical(<text>)]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>];
```

Notes:
- `USING` enables semantic/lexical search; without it, queries are strict filters only.
- `FILTER` is exact; fields must be qualified (`doc.*`, `chunk.*`).
- `ORDER BY score` is meaningful only when `USING` is present.
- Unknown `SELECT` fields are ignored in v0.1 (permissive).

### Filter Expression Language (FEL)
```
<boolean-expr> := <term> ( (AND|OR) <term> )*
<term> := [NOT] <predicate> | '(' <boolean-expr> ')'
<predicate> := <field> <op> <value> | <field> IN '(' <value-list> ')'
<op> := = | != | < | <= | > | >= | LIKE | GLOB
```
- `LIKE` uses `%` and `_`; `GLOB` uses `*`, `?`, and `**`.
- ISO-8601 dates compare lexicographically; strings are case-sensitive.

## Determinism and Explainability
- Stable IDs: `doc.id` = hash of normalized path + content hash; `chunk.id` = doc id + chunk offset.
- Deterministic ordering is always applied, even when `ORDER BY` is provided; ties are broken by:
  - `doc.path ASC`, then `chunk.offset ASC`, then `chunk.id ASC` (for `FROM chunk`).
  - `doc.path ASC`, then `doc.id ASC` (for `FROM doc`).
- Default ordering (when no `ORDER BY`):
  - With `USING`: `score DESC` then the same deterministic tie-breaks.
  - Without `USING`: `doc.path ASC` (and `chunk.offset ASC` for chunks).
- `FROM doc USING ...`: `score` is the max chunk score for that doc.
- `--explain` returns per-stage scores and any lexical sanitization fallback used.

## Hybrid Retrieval
- Lexical search via SQLite FTS5 (BM25-like); sanitized fallback if parsing fails.
- Semantic search via embeddings (default deterministic hash).
- Scores are normalized and combined with explicit weights from config.
- Filters are strict and never invoke semantic inference.

## Context Assembly
- Hard `budget_tokens`; context never exceeds the budget.
- Deterministic packing order mirrors retrieval ordering.
- De-duplication by chunk id; optional per-doc diversity cap.
- Truncation is deterministic (prefix to fit).
- Provenance for every chunk: path, offset, hash, mtime.

## Storage and Local-first
- Single-file store `recall.db` backed by SQLite.
- Single-writer, multi-reader semantics with a sibling lock file.
- No network calls unless explicitly configured by the user.

## Data Model (Logical)
- `doc`: `id`, `path`, `mtime`, `hash`, `tag`, `source`, `deleted`.
- `chunk`: `id`, `doc_id`, `offset`, `tokens`, `text`, `embedding`, `deleted`.

## Document Metadata (Planned)
- Problem: important fields (e.g., Status, Milestone) are embedded in Markdown
  and cannot be filtered without free-text parsing.
- Add an opt-in ingest flag (e.g., `--extract-meta`) to parse deterministic
  front matter or top-of-file `Key: Value` blocks.
- Store extracted fields as a doc-level metadata map (JSON) and expose them in
  `--json` outputs.
- RQL should allow exact filters on metadata keys (e.g., `doc.meta.milestone`),
  with missing keys treated as null.
- Requires schema versioning and migration support; not part of v0.1.

## JSON Output (Stable)
Top-level fields:
- `ok`, `schema_version`, `query`, `results`, `context`, `stats`, `warnings`, `error`.

Result entries include:
- `score`, `doc{...}`, `chunk{...}`, `explain{lexical, semantic}`.

Context entries include:
- `text`, `budget_tokens`, `used_tokens`, `chunks[{path, hash, mtime, offset, tokens, text}]`.

## Configuration (recall.toml)
- `store_path`
- `chunk_tokens`, `overlap_tokens`
- `embedding`, `embedding_dim`
- `ann_bits`, `ann_seed`
- `bm25_weight`, `vector_weight`
- `max_limit`

## Future (Explicitly Out of MVP Scope)
- Alternative ANN backends (e.g., HNSW).
- Structure-aware chunking and additional parsers.
- Background daemon/service mode.
