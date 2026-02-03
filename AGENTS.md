# AGENTS

## Project Snapshot
Recall is a local, single-file, CLI-first document store for AI agents with
strict filters, deterministic hybrid retrieval, and stable JSON output. Core
principles live in `DESIGN.md` and must be preserved.

## Repo Map
- `src/main.rs`: CLI entry + command dispatch.
- `src/cli.rs`: Clap CLI definitions.
- `src/query.rs`, `src/rql.rs`, `src/sql.rs`: RQL parsing + SQL generation.
- `src/store.rs`: SQLite schema, migrations, and storage.
- `src/ingest.rs`, `src/context.rs`, `src/ann.rs`, `src/embed.rs`: ingest,
  context assembly, ANN, and embeddings.
- `src/output.rs`: JSON/text output formatting.
- `schemas/response.schema.json`: JSON schema for `--json` output.
- `tests/cli_golden.rs` + `tests/snapshots/`: golden JSON snapshots (insta).
- `tests/determinism.rs`: determinism checks.
- `scripts/bench_gen.py`, `scripts/bench_run.py`: benchmark dataset + harness.

## Development Commands
Prefer the helper script:
```
./x build
./x test
./x fmt
./x clippy -- -D warnings
```
Targeted tests:
```
cargo test golden_cli_outputs
cargo test deterministic_outputs
cargo test migrates_unversioned_store
```
Snapshot updates (insta):
```
cargo insta accept
# or
INSTA_UPDATE=always cargo test
```

## Behavior Constraints
- Preserve determinism, strict filter exactness, and local-first/no-network
  behavior (see `DESIGN.md`).
- CLI/RQL/JSON changes require updating `README.md`, `DESIGN.md`, and
  `schemas/response.schema.json`, plus golden snapshots in `tests/snapshots/`.
- On-disk schema changes require migrations, migration tests, and an entry in
  `docs/history/changes/`, plus updates to `docs/COMPATIBILITY.md`.

## Inlined Reference Documents

### DEVELOPMENT_RULES.md
- MUST follow `WORKFLOWS.md` → "Lean Workflow (Default)" unless the user opts
  out explicitly.
- Keep changes minimal, deterministic, and fully tested for touched areas.
- Document user-facing behavior changes in README + design docs.

### WORKFLOWS.md
#### Lean Workflow (Default)
1. Sync with `main`, then create a topic branch (use `codex/<topic>`).
2. Make focused changes and update tests/docs alongside code.
3. Run relevant checks (`./x fmt`, `./x test`, targeted tests as needed).
4. Rebase onto latest `main` and squash/rewrite commits before merge.
5. Open a PR or provide a clear change summary + test results.
