# ADR: sqlite-vec Migration with Hard Schema Break

Date: 2026-02-03
Status: Accepted

## Context
Recall previously supported multiple ANN backends (LSH/HNSW/linear) with
backend-specific tables and configuration knobs. This increased maintenance
cost and complexity, and it diverged from the goal of a single, deterministic
storage engine. sqlite-vec provides a maintained, SQLite-native vector index
with KNN queries and cosine distance support across platforms.

## Decision
- Adopt sqlite-vec `vec0` as the sole vector index.
- Register sqlite-vec via `sqlite3_auto_extension(sqlite3_vec_init)` before any
  SQLite connection is opened.
- Store vectors in a `chunk_vec` virtual table with cosine distance.
- Remove ANN configuration options and backend selection.
- Bump on-disk schema to v2 and reject older schemas (no migration).

## Consequences
- Existing stores must be re-initialized and re-ingested.
- Vector index maintenance is handled by `chunk_vec` insert/delete and rebuild
  operations (used by `doctor --fix` and import flows).
- `doctor`/`explain` now report sqlite-vec diagnostics and configuration.
