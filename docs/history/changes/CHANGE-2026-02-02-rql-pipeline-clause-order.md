# CHANGE-2026-02-02-rql-pipeline-clause-order

Milestone: v1.0 M6 — Interface Freeze + Compatibility
Summary:
- Added pipeline-style RQL clause order with FROM first and SELECT last.
- Kept legacy SELECT-first syntax for compatibility.
User impact:
- Users can adopt pipeline-style RQL immediately; existing queries continue to work.
Migration:
- None (backward compatible).
References:
- ISSUE-2026-02-02-rql-pipeline-clause-order
- ADR-2026-02-02-rql-pipeline-clause-order
