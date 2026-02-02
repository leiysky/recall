# ISSUE-2026-02-01-doctor-fix-safer-compact

Status: open
Milestone: Milestone 2 — Local-first Reliability
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Add recall doctor --fix and safer compact flows.
- Strengthen recall doctor with FTS/ANN consistency checks and repair hints.
Scope:
- Add FTS/ANN consistency checks to doctor output.
- Add --fix to attempt safe repairs and report actions.
- Make compact flow safer with pre-checks and clear warnings.
Acceptance Criteria:
- doctor reports consistency status for FTS and ANN with actionable hints.
- doctor --fix performs safe repairs and records actions in JSON.
- compact refuses unsafe operations and reports what was done.
Out of Scope:
- Full rebuilds of ANN/FTS beyond documented repair steps.
Notes:
- Merged ISSUE-2026-02-01-doctor-consistency-checks-repair-hints.

Links:
- docs/history/decisions/ADR-2026-02-01-doctor-fix-safer-compact.md
- docs/progress/2026/2026-02-02.md
