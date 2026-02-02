# ISSUE-2026-02-01-snapshot-flag-search-query-context

Status: done
Milestone: Milestone 1 — Determinism + Explainability
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Add explicit --snapshot for search/query/context.
- Accept snapshots for reproducible pagination with OFFSET.
Scope:
- Add --snapshot to search/query/context CLI commands.
- Accept snapshot tokens to make OFFSET paging reproducible.
- Ensure JSON stats/reporting exposes snapshot used or generated.
Acceptance Criteria:
- search/query/context accept --snapshot <token> and include it in query metadata.
- OFFSET + --snapshot returns stable ordering across runs for identical inputs and store state.
- Invalid snapshot tokens produce actionable, JSON-parseable errors.
Out of Scope:
- New snapshot formats beyond current snapshot token semantics.
Notes:
- Merged ISSUE-2026-02-01-snapshot-pagination-offset.

Links:
- docs/history/decisions/ADR-2026-02-01-snapshot-flag-search-query-context.md
- docs/progress/2026/2026-02-02.md
