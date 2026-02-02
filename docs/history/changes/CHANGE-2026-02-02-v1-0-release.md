# CHANGE-2026-02-02-v1-0-release

Milestone: v1.0
Summary:
- Interface freeze for CLI/RQL/JSON schema and published compatibility matrix.
- Benchmark dataset, baselines, regression thresholds, and determinism checks.
- Release readiness checklist with RC gate definitions.
- Release tag: v1.0.0

User impact:
- Stable interfaces for v1.0 tooling and agent integrations.
- Reproducible performance baselines and deterministic outputs.
- Clear upgrade guidance and compatibility expectations.

Migration:
- Automatic migration from unversioned stores to schema version 1 on open.
- Newer schema versions are rejected with a machine-parseable error.
- Back up `recall.db` before upgrading.

References:
- ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility
- ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines
- ISSUE-2026-02-02-v1-0-m8-release-readiness
