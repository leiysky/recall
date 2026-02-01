# ISSUE-2026-02-01-file-locking-busy-timeout

Status: done
Milestone: M1
Owner:
Created: 2026-02-01
Updated: 2026-02-01

Context:
- Implement file locking + busy timeout for single-writer, multi-reader.
Scope:
- Implement file locking + busy timeout for single-writer, multi-reader.
Acceptance Criteria:
- Store enforces single-writer, multi-reader semantics via explicit file locking.
- SQLite busy timeout configured to avoid immediate lock failures.
- Errors for lock contention are actionable.
- Tests cover locking behavior where feasible.
Out of Scope:
- Changes to on-disk schema or query semantics.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-file-locking-busy-timeout.md
- docs/progress/2026/2026-02-01.md
