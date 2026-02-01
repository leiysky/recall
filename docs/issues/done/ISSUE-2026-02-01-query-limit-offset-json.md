# ISSUE-2026-02-01-query-limit-offset-json

Status: done
Milestone: M1
Owner:
Created: 2026-02-01
Updated: 2026-02-01

Context:
- Emit real query.limit/offset in JSON outputs.
Scope:
- Emit real query.limit/offset in JSON outputs.
Acceptance Criteria:
- JSON output reports the effective limit and offset for search/query/context.
- Values match parsed RQL or CLI defaults for each command.
- Snapshot/golden tests cover the updated JSON fields.
Out of Scope:
- Changing result ordering semantics beyond exposing limit/offset.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-query-limit-offset-json.md
- docs/progress/2026/2026-02-01.md
