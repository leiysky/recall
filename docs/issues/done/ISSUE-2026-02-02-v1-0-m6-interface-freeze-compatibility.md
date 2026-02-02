# ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility

Status: done
Milestone: v1.0 Milestone 6 — Interface Freeze + Compatibility
Owner: Nexus
Created: 2026-02-02
Updated: 2026-02-02

Context:
- v1.0 requires stable interfaces and explicit compatibility guarantees.
- CLI, RQL, and JSON schema must be frozen before release gating.

Scope:
- Freeze CLI flags, RQL syntax, and JSON schema for v1.0.
- Define the supported upgrade path and migration guarantees for on-disk schema.
- Publish a compatibility matrix (OS/toolchain + embedding provider expectations).
- Document any known constraints or limitations.

Acceptance Criteria:
- Interface freeze documented (no breaking changes after Milestone 6).
- Upgrade/migration guarantees documented with supported schema versions.
- Compatibility matrix published in docs.
- PRD and ROADMAP reference the freeze and compatibility commitments.

Out of Scope:
- Performance baselines or benchmark work.
- Release candidate checklist or release notes.

Notes:
- Ensure documentation stays consistent across README, DESIGN, and RELEASE.
- docs/COMPATIBILITY.md captures the interface freeze, upgrade guarantees, and matrix.

Links:
- ROADMAP.md
- docs/PRD-v1.0.md
- docs/progress/2026/2026-02-02.md
