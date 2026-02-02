# ISSUE-2026-02-02-v1-0-m8-release-readiness

Status: active
Milestone: v1.0 Milestone 8 — Release Readiness
Owner: Nexus
Created: 2026-02-02
Updated: 2026-02-02

Context:
- v1.0 release requires docs completeness and formal sign-off gates.
- Release notes and tagging must be prepared once gates are met.

Scope:
- Complete documentation updates (README, DESIGN, AGENTS, WORKFLOWS).
- Produce a release candidate checklist with sign-off criteria and zero P0/P1 issues.
- Cut v1.0 release notes and tag the release.

Acceptance Criteria:
- Documentation audit complete with updates recorded.
- RC checklist published with explicit sign-off criteria and issue severity definitions.
- Zero open P0/P1 issues at RC sign-off.
- v1.0 release notes prepared and release tag recorded.

Out of Scope:
- Benchmark creation or performance tuning.
- Interface freeze or migration guarantees.

Notes:
- Ensure release notes reference compatibility matrix and baselines.
- RC gate definitions live in docs/RELEASE.md; release notes draft in docs/history/changes/CHANGE-2026-02-02-v1-0-release.md.
- Release tag is pending review and approval.
- Core principle severity mapping: strict FILTER exactness is P0; context budget/provenance is P1.
- Core principle verification checklist now references tests/commands in docs/RELEASE.md.

Links:
- ROADMAP.md
- docs/PRD-v1.0.md
- docs/RELEASE.md
- docs/progress/2026/2026-02-02.md
