# ISSUE-2026-02-01-deterministic-structured-queries

Status: done
Milestone: M1
Owner:
Created: 2026-02-01
Updated: 2026-02-01

Context:
- Make structured queries deterministic or require explicit ORDER BY.
Scope:
- Make structured queries deterministic or require explicit ORDER BY.
Acceptance Criteria:
- Structured queries without `USING` have deterministic default ordering.
- `ORDER BY` overrides the default ordering.
- Documentation updated to reflect deterministic ordering rules.
- Tests cover structured-query ordering.
Out of Scope:
- Changing ordering rules for `USING` queries beyond existing behavior.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-deterministic-structured-queries.md
- docs/progress/2026/2026-02-01.md
