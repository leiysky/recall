# ISSUE-2026-02-02-cli-store-mode-safety

Status: open
Milestone: Milestone 2 — Local-first Reliability
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
- recall compact opens the store read-only but performs writes.
- recall stats opens the store read-write, blocking readers unnecessarily.
Scope:
- Open the store in read-write mode for compact.
- Use read-only mode for stats.
- Add or adjust tests to validate lock behavior.
Acceptance Criteria:
- recall compact succeeds without read-only errors and uses exclusive lock.
- recall stats can run concurrently with other readers.
- Tests or diagnostics validate lock modes.
Out of Scope:
- Changing compaction algorithm or schema.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-02-cli-store-mode-safety.md
- docs/progress/2026/2026-02-02.md
