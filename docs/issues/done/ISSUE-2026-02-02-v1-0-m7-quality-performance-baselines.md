# ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines

Status: done
Milestone: v1.0 Milestone 7 — Quality + Performance Baselines
Owner: Nexus
Created: 2026-02-02
Updated: 2026-02-02

Context:
- v1.0 needs reproducible performance and determinism baselines.
- Upgrades must be validated across prior schema versions.

Scope:
- Establish a reproducible benchmark dataset or generator.
- Publish baseline numbers for search/query/context and ingest.
- Define regression thresholds and add run-to-run determinism checks.
- Validate upgrades across prior schema versions with explicit migration tests.

Acceptance Criteria:
- Benchmark spec and dataset/generator documented.
- Baseline numbers recorded with hardware/software notes.
- Regression thresholds documented and wired into test/CI guidance.
- Determinism checks added and repeatable over multiple runs.
- Migration tests cover all supported prior schema versions.

Out of Scope:
- Interface freeze or compatibility matrix updates.
- Release candidate checklist and tagging.

Notes:
- Align metrics with PRD non-functional requirements.
- Benchmark spec and baseline: docs/benchmarks/README.md and docs/benchmarks/baseline-2026-02-02.md.
- Determinism checks: tests/determinism.rs (cargo test deterministic_outputs).

Links:
- ROADMAP.md
- docs/PRD-v1.0.md
- docs/progress/2026/2026-02-02.md
