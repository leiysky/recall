# ISSUE-2026-02-02-sqlite-sql-builder-refactor

Status: done
Milestone: v1.0 M7 — Quality + Performance Baselines
Owner: codex
Created: 2026-02-02
Updated: 2026-02-02

Context:
- Dynamic SQLite queries are built with string formatting and ad-hoc placeholder lists.
- A typed builder would centralize column/table definitions and reduce stringly-typed SQL.

Scope:
- Add a type-safe SQL builder for SELECT queries and predicates.
- Refactor query filtering/order/joins in src/query.rs to use the builder.

Acceptance Criteria:
- Dynamic query construction uses the builder (no ad-hoc format! SQL for SELECT paths).
- Query behavior and ordering remain unchanged.
- Tests/build pass for touched areas when run.

Out of Scope:
- DDL schema strings and static INSERT/UPDATE statements.
- Changes to RQL syntax or CLI behavior.

Notes:
- Keep parameter ordering deterministic and preserve tie-breaks.

Links:
- docs/progress/2026/2026-02-02.md
