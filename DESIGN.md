# Recall Design Doc

Date: 2026-02-03
Status: Draft (principles-first)

## Product Summary
Recall is a local, single-file, CLI-first document store for AI agents. It provides deterministic, explainable hybrid retrieval with strict filters and builds token-budgeted context windows via a stable RQL interface.

## Core Principles
Canonical source: this section defines the core principles and terms; other docs should link here.
1. Determinism over magic: identical inputs + store state yield identical outputs, including ordering and context assembly.
2. Hybrid retrieval with strict filters: semantic + lexical ranking is allowed, but FILTER constraints are exact and non-negotiable.
3. Local-first, zero-ops: single-file `recall.db`, offline by default, no required services.
4. Context as a managed resource: hard token budgets, deterministic packing, and provenance for every chunk.
5. AI-native interface: CLI and stable RQL are the source of truth; JSON outputs are stable for tooling.

### Core Terms (Glossary)
- Strict filters: FILTER predicates are exact; no semantic inference, and any result must satisfy them.
- Deterministic packing: context assembly selects, orders, and truncates chunks in a fixed, documented way under a hard token budget.
- Provenance: each chunk retains path, offsets, hash, and mtime for traceability.

## Scope (v0.1)
- Single-file store `recall.db` (SQLite-backed).
- CLI: `init`, `add`, `rm`, `search`, `query`, `context`, plus `stats`, `doctor`, `compact`.
- Hybrid retrieval: lexical (FTS5 BM25) + semantic embeddings with explicit weights.
- Deterministic ordering and tie-breaks; `--explain` for scoring stages.
- Budgeted context assembly with provenance and optional diversity cap.
- Stable `--json` output with schema versioning and JSONL streaming for large results.
- JSONL export/import for portability.
- Snapshot tokens for reproducible paging.
- On-disk schema versioning + migrations.
- Optional metadata extraction from Markdown headers/front matter.
- Structure-aware chunking (markdown headings and code blocks).

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
- `recall completions`, `recall man`

### RQL (AI-native)
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
- `FILTER` is exact; fields must be qualified (`doc.*`, `chunk.*`).
- `ORDER BY score` is meaningful only when `USING` is present.
- Unknown `SELECT` fields are ignored in v0.1 (permissive).
- Legacy `SELECT ... FROM ...` syntax is still accepted for compatibility.

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
- `--explain` returns per-stage scores, resolved config, candidate counts, and lexical sanitization details.
- Per-stage timing breakdowns are included in JSON stats.
- Snapshot tokens (`--snapshot`) freeze results for reproducible pagination.

## Hybrid Retrieval
- Lexical search via SQLite FTS5 (BM25-like); sanitized fallback if parsing fails.
- Semantic search via embeddings (default deterministic hash).
- Scores are normalized and combined with explicit weights from config.
- Filters are strict and never invoke semantic inference.
- ANN backend is configurable (`lsh`, `hnsw`, or `linear`) with LSH fallback.

## Context Assembly
- Hard `budget_tokens`; context never exceeds the budget.
- Deterministic packing order mirrors retrieval ordering.
- De-duplication by chunk id; optional per-doc diversity cap.
- Truncation is deterministic (prefix to fit).
- Provenance for every chunk: path, offset, hash, mtime.

## Storage and Local-first
- Single-file store `recall.db` backed by SQLite.
- Single-writer, multi-reader semantics with a temporary lock file in the OS temp dir.
- No network calls unless explicitly configured by the user.
- On-disk schema versions are stored in a `meta` table and migrated on open.

## Compatibility and Upgrade Guarantees (v1.0)
- CLI, RQL, and JSON schema are frozen after Milestone 6.
- Breaking changes require a major version bump and schema version change.
- Unversioned stores are migrated to schema version 1 on open; newer schemas are rejected.
- See `docs/COMPATIBILITY.md` for the compatibility matrix and upgrade guidance.

## Release Readiness (v1.0)
- Release checklist and RC gate definitions: `docs/RELEASE.md`.
- Performance baselines and regression thresholds: `docs/benchmarks/README.md`.

## Data Model (Logical)
- `doc`: `id`, `path`, `mtime`, `hash`, `tag`, `source`, `meta`, `deleted`.
- `chunk`: `id`, `doc_id`, `offset`, `tokens`, `text`, `embedding`, `deleted`.
- `meta`: key/value schema metadata.

## Document Metadata
- Opt-in ingest flag `--extract-meta` parses deterministic Markdown front matter
  or top-of-file `Key: Value` blocks.
- Extracted fields are stored as a doc-level metadata map (JSON) and exposed in
  `--json` outputs.
- RQL allows exact filters on metadata keys (e.g., `doc.meta.milestone`), with
  missing keys treated as null.
- Metadata keys are normalized to lowercase with `_` separators.

## JSON Output (Stable)
Top-level fields:
- `ok`, `schema_version`, `query`, `results`, `context`, `stats`, `warnings`, `error`, `explain`.

Result entries include:
- `score`, `doc{...}`, `chunk{...}`, `explain{lexical, semantic}`.

Context entries include:
- `text`, `budget_tokens`, `used_tokens`, `chunks[{path, hash, mtime, offset, tokens, text}]`.

## Error Contract
- With `--json`, failures return `ok=false` and an `error{code,message}` object.
- Non-JSON mode returns a non-zero exit status and prints a human-readable error.

## Configuration (Global recall.toml)
Recall uses an optional global config file in the OS config directory:
`<config_dir>/recall/recall.toml`.
- `store_path`
- `chunk_tokens`, `overlap_tokens`
- `embedding`, `embedding_dim`
- `ann_backend` (`lsh`, `hnsw`, `linear`)
- `ann_bits`, `ann_seed`
- `bm25_weight`, `vector_weight`
- `max_limit`

## Defaults and Precedence
- Global config (if present) overrides built-in defaults.
- No per-directory config files are supported.

## Store Discovery
- Commands walk up from the current directory to locate `config.store_path`
  (default `recall.db`).
- If `store_path` is absolute, it is used directly.

## Future (Explicitly Out of MVP Scope)
- Additional parsers (PDF deferred).
- Background daemon/service mode.
