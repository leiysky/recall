# ISSUE-2026-02-02-rql-pipeline-clause-order

Status: done
Milestone: v1.0 M6 — Interface Freeze + Compatibility
Owner: codex
Created: 2026-02-02
Updated: 2026-02-02

Context:
- RQL is currently SELECT-first; request to adopt a pipeline-style order.
- Interface rules require backward-compatible syntax changes.

Scope:
- Extend the RQL parser to accept FROM-first / SELECT-last form.
- Update docs/examples to present pipeline-style as canonical and note legacy support.
- Update CLI help and tests to cover the new syntax.

Acceptance Criteria:
- Parser accepts both SELECT-first and FROM-first forms without semantic changes.
- README/DESIGN/AGENTS reflect the pipeline-style order and mention legacy syntax.
- CLI help/examples and golden tests use the pipeline-style form.
- Tests pass for touched areas.

Out of Scope:
- Changing filter semantics or adding new RQL features.
- Removing legacy SELECT-first support.

Notes:
- Preserve deterministic ordering rules and output stability.

Links:
- docs/history/decisions/ADR-2026-02-02-rql-pipeline-clause-order.md
- docs/progress/2026/2026-02-02.md
