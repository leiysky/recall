# Recall v1.0 Compatibility and Freeze

Date: 2026-02-03
Status: Draft (Milestone 6)

## Interface Freeze (v1.0)
Beginning at Milestone 6, the following interfaces are frozen for v1.0:
- CLI flags and command behavior.
- RQL syntax and semantics.
- JSON output schema (including `schema_version`).

No breaking changes to these interfaces are permitted after this point. Any
breaking changes require a major version bump and a new schema version.

## Upgrade and Migration Guarantees
- Recall stores schema metadata in `recall.db` and migrates on open when the
  stored version is older than the supported schema version.
- v1.0 guarantees migration from unversioned stores (pre-`schema_version`) to
  schema version 1.
- Stores with a newer schema than the running binary are rejected with a
  machine-parseable error, and the user is instructed to upgrade Recall.

Recommended upgrade flow:
1) Back up `recall.db` before upgrading.
2) Run `recall doctor --json` to validate the store.
3) Open the store with the new Recall binary; migrations run automatically.
4) Run `recall stats --json` to confirm the new schema version.

## Breaking Changes (2026-02-03)
- Per-directory `recall.toml` is no longer supported. Move settings to the global config at
  `<config_dir>/recall/recall.toml`.
- Store discovery now walks up for `recall.db` (or the configured `store_path`).
- Lock files moved to the OS temp directory and are cleaned up on writer exit.

## Compatibility Matrix
Status legend: Validated (tested locally), Targeted (expected to work),
Unsupported (known incompatible).

| Component | Status | Notes |
| --- | --- | --- |
| macOS 13+ (arm64/x86_64) | Validated | Primary dev/test environment. |
| Linux (x86_64, glibc 2.31+) | Targeted | Expected to work with bundled SQLite. |
| Windows 11 (x86_64) | Targeted | Expected to work; path handling differs. |
| Rust toolchain | Validated | `nightly-2026-02-01` per rust-toolchain.toml. |
| SQLite (FTS5) | Validated | Bundled via rusqlite; FTS5 enabled. |
| Embedding provider | Validated | Built-in `hash` embedder only. |
| ANN backend | Validated | `lsh` default; `hnsw` and `linear` supported. |

## Known Constraints
- Only the built-in `hash` embedder is supported in v1.0; external providers
  are not bundled.
- Cross-platform path normalization is OS-native; path separators differ.
