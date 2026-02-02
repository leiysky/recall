# ISSUE-2026-02-02-cli-flag-hygiene-context-explain

Status: open
Milestone: Milestone 1 — Determinism + Explainability
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
- CLI exposes --parser and --format flags that are no-ops.
- context --explain currently produces no visible explain output.
Scope:
- Decide for --parser and --format: implement, deprecate, or error out explicitly.
- Ensure context --explain returns explain payload or fails with actionable error.
- Update CLI help text and docs to match behavior.
Acceptance Criteria:
- No-op flags have explicit behavior (implemented or clear error/warning).
- context --explain produces documented output in text/JSON or returns a clear error.
- CLI help and README describe the behavior.
Out of Scope:
- Implementing new parsers or output formats beyond documented scope.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-02-cli-flag-hygiene-context-explain.md
- docs/progress/2026/2026-02-02.md
