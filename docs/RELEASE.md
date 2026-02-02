# Recall Release Checklist & Versioning Policy

## Versioning Policy
- Use semantic versioning (MAJOR.MINOR.PATCH).
- MAJOR: breaking CLI/RQL changes, JSON schema version bumps, or on-disk format changes.
- MINOR: backward-compatible features, new flags, or new JSON fields.
- PATCH: bug fixes and internal improvements with no interface changes.
- On-disk schema changes must include:
  - Migration logic.
  - A migration test.
  - A note in `docs/history/changes/`.
- JSON schema version changes must be documented and migrated in tooling.

## Release Candidate Gate (v1.0)
Sign-off criteria:
- Zero open P0/P1 issues.
- All tests pass (`./x test`, determinism, and migration tests).
- Benchmarks within thresholds (see `docs/benchmarks/README.md`).
- Compatibility matrix and upgrade guidance up to date (`docs/COMPATIBILITY.md`).
- Release notes prepared (`docs/history/changes/CHANGE-2026-02-02-v1-0-release.md`).
- Verify core principles:
  - Strict filters are exact and enforced (no semantic inference).
  - Context budget/provenance guarantees hold.
  - Local-first/no-network behavior is intact.

Severity definitions:
- P0: data loss/corruption, security issue, crash in core flows, failed migrations,
  nondeterministic search/query/context outputs, or strict FILTER exactness violations.
- P1: CLI/RQL/JSON compatibility regression, performance outside thresholds,
  incorrect results in common flows, benchmark regressions beyond limits, or
  context budget/provenance violations.

## Release Checklist
- [ ] Update `Cargo.toml` version.
- [ ] Run `./x fmt` and `./x clippy -- -D warnings`.
- [ ] Run `./x test` and review snapshot updates.
- [ ] Run core flows:
  - `recall init` → `recall add` → `recall search` → `recall context`.
- [ ] Run `scripts/bench_run.py` and compare against baseline thresholds.
- [ ] Verify migrations on an older store (schema version bump).
- [ ] Core principle checks (v1.0):
  - Strict FILTER exactness: run `cargo test golden_cli_outputs` and spot-check a negative filter
    (expect empty results when the filter cannot match).
  - Context budget/provenance: confirm `context.used_tokens <= context.budget_tokens` and that
    each context chunk includes `path`, `hash`, `mtime`, `offset`, and `tokens` (see
    `tests/cli_golden.rs` snapshots).
  - Local-first/no-network: verify no network configuration is set and confirm no network
    dependencies are introduced in the release diff.
- [ ] Update `docs/history/changes/` for user-visible changes.
- [ ] Update `ROADMAP.md` milestones and issue status.
- [ ] Tag the release and publish notes.
