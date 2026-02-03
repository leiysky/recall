# Change Log: sqlite-vec Migration (2026-02-03)

## Summary
- Replaced vector search backends (LSH/HNSW/linear) with sqlite-vec `vec0` KNN using cosine distance.
- Bumped on-disk schema to v2 and removed compatibility with older stores.
- Removed ANN config knobs; vector search now uses sqlite-vec only.

## Impact
- **Breaking**: existing `recall.db` files must be re-initialized and re-ingested.
- JSON `schema_version` is now `"2"`.
- `doctor` and `explain` report sqlite-vec diagnostics and configuration.

## Notes
- The vector index is stored in the `chunk_vec` virtual table and maintained during ingest and delete operations.
- Embeddings are still stored in `chunk.embedding` for export/import and rebuilds.
