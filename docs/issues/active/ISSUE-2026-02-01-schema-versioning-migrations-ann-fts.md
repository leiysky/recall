# ISSUE-2026-02-01-schema-versioning-migrations-ann-fts

Status: active
Milestone: Milestone 2 — Local-first Reliability
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Add on-disk schema versioning + migrations (incl. ANN + FTS).
Scope:
- Add on-disk schema versioning + migrations (incl. ANN + FTS).
Acceptance Criteria:
- Store records a schema version in the database.
- Opening a store checks and migrates to the current version.
- ANN/FTS versioning is tracked for future migrations.
- Migration tests cover upgrade from unversioned stores.
Out of Scope:
- Implementing new ANN/FTS backends beyond version tracking.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-schema-versioning-migrations-ann-fts.md
- docs/progress/2026/2026-02-01.md
