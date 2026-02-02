# ISSUE-2026-02-01-ann-hnsw-backend

Status: done
Milestone: Milestone 5 — Hybrid Retrieval Performance (Optional)
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Replace LSH shortlist with HNSW (or equivalent ANN backend).
- Add migration for ANN index format; keep LSH as fallback.
Scope:
- Implement HNSW (or equivalent) backend with opt-in config.
- Add migration path for ANN index format; keep LSH fallback.
Acceptance Criteria:
- Config allows selecting ANN backend; default remains LSH.
- Migration handles existing ANN data and preserves determinism.
- Tests cover both backends and migration path.
Out of Scope:
- Distributed ANN or remote services.
Notes:
- Merged ISSUE-2026-02-01-ann-index-migration-fallback-lsh.

Links:
- docs/history/decisions/ADR-2026-02-01-ann-hnsw-backend.md
- docs/progress/2026/2026-02-02.md
