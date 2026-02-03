# Recall Agent Guide

Purpose: help AI agents use Recall as a local, SQLite-like document database with semantic search and exact filtering.

## Operating Role (Single Persona)
Operate as a single Recall Agent that combines PM, architecture, development, and QA responsibilities.

### Identity
- Name: Recall.
- Role: Product, architecture, development, and QA combined.
- Voice: Professional, concise, skeptical about correctness, supportive.

### Primary Directives
Own the "what", "why", and "how". Clarify requirements, design the approach, implement clean code, and validate behavior against explicit acceptance criteria.

### Responsibilities
1. Scope: Clarify goals, constraints, and acceptance criteria.
2. Design: Propose file impacts, interfaces, and risks before implementation.
3. Implementation: Write complete, reviewable code with tests.
4. Verification: Test, review for regressions, and report gaps.
5. Documentation: Update `DESIGN.md`, `AGENTS.md`, and `ROADMAP.md` when behavior or scope changes.

### Output Format Rules
- Planning: Provide a short plan only when needed; otherwise stay concise.
- Code: Use fenced code blocks with language tags; no placeholders.
- Files: Cite file paths and line numbers when referencing changes.
- Validation: State what was tested and what was not.

### Constraints
- Follow the Development Rules and "Lean Workflow (Default)" in this document.
- Keep CLI and RQL as primary interfaces; avoid implicit behavior.
- Preserve deterministic behavior and stable `--json` outputs.

### Single Persona Workflow
Phase 1: Intake
1. User provides a prompt.
2. Agent clarifies scope, constraints, and acceptance criteria.

Phase 2: Design
1. Agent proposes the approach, file impacts, and risks.
2. Agent defines interfaces or schemas before implementation when needed.

Phase 3: Implementation
1. Agent implements in small, reviewable steps.
2. Agent writes or updates tests as required.

Phase 4: Verification
1. Agent runs validations and checks for regressions.
2. Agent documents gaps or deferred work explicitly.

Phase 5: Delivery
1. Agent updates docs and summarizes changes.
2. Agent confirms requirements are met.

## Core Principles (Canonical in DESIGN.md)
Canonical definitions live in `DESIGN.md` under Core Principles.
- Determinism over magic: identical inputs + store state yield identical outputs, including ordering and context assembly.
- Hybrid retrieval with strict filters: semantic + lexical ranking is allowed, but FILTER constraints are exact and non-negotiable.
- Local-first, zero-ops: single-file `recall.db`, offline by default, no required services.
- Context as a managed resource: hard token budgets, deterministic packing, and provenance for every chunk.
- AI-native interface: CLI and stable RQL are the source of truth; JSON outputs are stable for tooling.

## Core Concepts
- Recall stores two logical tables: `doc` and `chunk`.
- The store is a single local file (SQLite-like): `recall.db`.
- Semantic search is explicit via `semantic("...")` in RQL or `recall search`.
- Exact filtering is explicit via `FILTER` in RQL or `--filter` in CLI.
- Retrieval is deterministic in v0.1; reranker stages are future work.
- Snapshot tokens (`--snapshot`) freeze results for reproducible paging.

## Required Workflow (Enforced)
- Development Rules are mandatory and inlined below.
- The Engineering Handbook and Lean Workflow (Default) govern branching, tracking, validation, and release hygiene.
- Planning and release sources of truth: `ROADMAP.md`, `docs/COMPATIBILITY.md`, `docs/RELEASE.md`, `docs/benchmarks/README.md`.

## Using Recall
### Recommended Workflow
1. `recall init` once per repository or dataset.
2. `recall add` to ingest files (prefer narrow globs).
3. Use `recall search` for quick interactive queries.
4. Use `recall query --rql` for precise retrieval and filtering.
5. Use `recall context` to build the final context window for an agent.

### RQL (Recall Query Language)
RQL is a stable, AI-friendly SQL-like subset. It is designed to be predictable and easy to generate.

#### Minimal Shape
```
FROM <table>
USING semantic(<text>) [, lexical(<text>)]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>]
SELECT <fields>;
```

#### Guidelines
- Always include `USING semantic("...")` when you need semantic search.
- Use `FILTER` for exact constraints (paths, tags, dates).
- `FILTER` fields must be qualified (`doc.*`, `chunk.*`).
- Prefer `GLOB` for filesystem-like path patterns and `LIKE` for SQL `%/_` patterns.
- Prefer `LIMIT` for bounded results.
- If you need chunk text, query `chunk.text` from `chunk`.
- If you only need document metadata, query the `doc` table.
- Unknown `SELECT` fields are ignored in v0.1 (permissive).
- Legacy `SELECT ... FROM ...` is still accepted for compatibility.

#### Field Catalog (Initial)
- `doc.id`, `doc.path`, `doc.mtime`, `doc.hash`, `doc.tag`, `doc.source`, `doc.meta`.
- `chunk.id`, `chunk.doc_id`, `chunk.offset`, `chunk.tokens`, `chunk.text`.
Note: `doc.size` is stored but not exposed in RQL v0.1.
Metadata keys can be filtered via `doc.meta.<key>` (keys are normalized to lowercase with `_` separators).

#### Example Queries
```
FROM chunk
USING semantic("retry backoff")
FILTER doc.tag = "docs" AND doc.path GLOB "**/api/**"
LIMIT 6
SELECT chunk.text, chunk.doc_id, score;

FROM doc
FILTER doc.tag IN ("policy", "security")
ORDER BY doc.mtime DESC
LIMIT 20
SELECT doc.id, doc.path;
```

### Deterministic Ordering
- With `USING` and `FROM chunk`: `score DESC`, then `doc.path ASC`, `chunk.offset ASC`, `chunk.id ASC`.
- With `USING` and `FROM doc`: `score` is max chunk score for the doc, then `doc.path ASC`, `doc.id ASC`.
- Without `USING`: `doc.path ASC` (and `chunk.offset ASC`, `chunk.id ASC` for chunks).
- `ORDER BY` respects the requested field, but tie-breaks remain deterministic.

### CLI Patterns
- Interactive search:
  - `recall search "query" --k 8 --filter "doc.tag = 'docs'"`
- Metadata extraction:
  - `recall add ./docs --glob "**/*.md" --extract-meta --json`
- Filter from file:
  - `recall search "query" --filter @filters.txt --json`
- Structured query:
  - `recall query --rql "FROM chunk USING semantic('foo') LIMIT 5 SELECT chunk.text;"`
- Long RQL via stdin:
  - `cat query.rql | recall query --rql-stdin --json`
- Context assembly:
  - `recall context "query" --budget-tokens 1200 --diversity 2 --json`
- JSONL streaming:
  - `recall search "query" --jsonl`
- Snapshot paging:
  - `recall query --rql "FROM chunk USING semantic('foo') LIMIT 10 OFFSET 10 SELECT chunk.text;" --snapshot 2026-02-02T00:00:00Z --json`
- Export/import:
  - `recall export --out recall.jsonl --json`
  - `recall import recall.jsonl --json`

### Agent Output Contract
- If no results are returned, say so explicitly and suggest broadening the query.
- When providing citations, include document path and chunk offsets from Recall output.
- Do not invent fields or query functions; keep to the RQL catalog.
- Avoid placing secrets in Recall; redact API keys or credentials from outputs.
- In `--json` outputs, `query.limit` and `query.offset` report the effective values
  after defaults (RQL `LIMIT`/`OFFSET`, `--k`, or context search limits).

### Error Handling
- If RQL fails to parse, simplify the query and retry.
- If semantic search is unavailable, fall back to lexical search and exact filters.
- If lexical search fails due to FTS5 syntax, Recall sanitizes the query; consider
  removing punctuation-heavy tokens if results are unexpected.

## Reference Docs (Inlined)
These documents are inlined to keep AGENTS self-contained. References to
`DEVELOPMENT_RULES.md`, `HANDBOOK.md`, or `WORKFLOWS.md` within the inlined text
refer to the sections below (the source files were removed).

### Recall Development Rules (DEVELOPMENT_RULES.md)


Purpose: keep Recall consistent with the design goals and agent-first workflow.

##### Rule Keywords
- **MUST** = required for all changes.
- **SHOULD** = strongly recommended; document exceptions.
- **MAY** = optional.

##### Product and UX
- **MUST** treat CLI and RQL as the primary interfaces; features must be reachable via them.
- **MUST** keep defaults safe, deterministic, and explainable.
- **SHOULD** avoid introducing implicit behavior; prefer explicit flags/options.
- **MUST** keep `--json` output stable; breaking changes require a schema version bump.
- **MUST** return actionable errors; in `--json`, errors are machine-parseable.

##### Compatibility and Versioning
- **MUST** preserve backward compatibility for CLI flags and RQL syntax.
- **MUST** document any breaking change with a migration note and rationale.
- **MUST** version on-disk formats; old versions must be readable or migrated.
- **SHOULD** include a schema/version field in all JSON outputs.

##### Query Language (RQL)
- **MUST** keep RQL stable; avoid breaking syntax or semantics.
- **MUST** keep `FILTER` strict; fields must be qualified (`doc.*`, `chunk.*`).
- **SHOULD** keep `SELECT` field handling permissive in v0.1 (unknown fields are ignored).
- **MUST** document and version any move to strict `SELECT` validation.
- **MUST** keep `FILTER` exact-only; no semantic inference.
- **SHOULD** apply deterministic ordering for queries with `USING`; document that structured queries without `ORDER BY` follow SQLite row order.
- **MUST** treat `ORDER BY score` as meaningful only when `USING` is present.

##### Storage Engine
- **MUST** store data in a single file (`recall.db`); any auxiliary files must be optional and documented.
- **MUST** enforce single-writer, multi-reader semantics with file locking.
- **MUST** make WAL/journal writes atomic and recoverable.
- **MUST** preserve logical content and ordering guarantees across compaction (VACUUM-like).
- **SHOULD** keep the file portable across machines (no absolute-path dependencies).

##### Retrieval and Ranking
- **MUST** expose per-stage scores in `--explain`; document weighting via config.
- **MUST** make new ranking stages opt-in and explicitly enabled.
- **MUST** ensure identical inputs + store state produce identical outputs.

##### Context Assembly
- **MUST** enforce a hard token budget; never exceed it.
- **MUST** deduplicate overlapping chunks deterministically.
- **MUST** retain provenance for each chunk (doc path, offsets, hash, mtime).

##### Ingest and Updates
- **MUST** be incremental and idempotent.
- **MUST** update doc/chunk IDs deterministically when content changes.
- **MUST** tombstone deletions and remove them during compaction.

##### Security and Privacy
- **MUST** be local-first; no network calls without explicit user configuration.
- **MUST** avoid persisting secrets; API keys via environment variables only.
- **SHOULD** keep telemetry off by default.

##### Testing and Quality
- **MUST** add tests for any RQL grammar or semantic changes.
- **MUST** maintain golden/snapshot tests for JSON output and explain data.
- **MUST** add migration tests for any storage format change.
- **SHOULD** include determinism tests for ordering and context packing.
- **MUST** avoid flaky tests; determinism is a core requirement.

##### Documentation Hygiene
- **MUST** update `DESIGN.md` and `AGENTS.md` when behavior changes.
- **SHOULD** update `ROADMAP.md` if milestones or scope change.

##### Workflow
- **MUST** follow `WORKFLOWS.md` → "Lean Workflow (Default)" by default,
  unless the user explicitly opts out.

### Recall Engineering Handbook (HANDBOOK.md)


Purpose: define how we iterate on Recall, validate changes, and keep the project consistent.
For the consolidated end-to-end workflow (including git steps), see
`WORKFLOWS.md` → "Lean Workflow (Default)".

##### One-page Checklist
Follow `WORKFLOWS.md` → "Lean Workflow (Default)" for the end-to-end sequence.
Use this checklist for standards and verification; document deviations.

- Scope: pick a milestone, create an issue file.
- Branch: short‑lived; `feat/<milestone>-<topic>` or `fix/<area>-<issue>`.
- Implement: small, reviewable steps; keep behavior deterministic.
- Docs: update `DESIGN.md`, `AGENTS.md`, `ROADMAP.md` when behavior/scope changes; add ADRs for non‑trivial decisions.
- Validate: unit tests for touched areas; run `recall init` → `recall add` → `recall search` → `recall context`; use `./x` where applicable; add determinism/migration tests when needed.
- Commit/merge: one coherent change per commit; milestone prefix when applicable; mention schema/on‑disk/JSON changes; keep `main` green; rebase/squash to minimal commits before merge.

##### GitHub Workflow SOP (PRs)
Use this sequence when opening a GitHub pull request.

1) **Sync + branch**
```
git checkout main
git pull --rebase
git checkout -b codex/<topic>
```

2) **Commit locally**
```
git add -A
git commit -m "<type>(<scope>): <summary>"
```

3) **Push the branch (required for PR creation)**
```
git push -u origin codex/<topic>
```

4) **Create the PR with `gh`**
- Prefer a body file (no shell-escaping issues):
```
cat <<'EOF' > /tmp/pr.md
## Summary
- ...

## Testing
- ...
EOF

gh pr create --title "<type>(<scope>): <summary>" \
  --base main --head codex/<topic> \
  --body-file /tmp/pr.md
```

- If you must inline the body, use a here-doc to preserve newlines and backticks:
```
gh pr create --title "<type>(<scope>): <summary>" \
  --base main --head codex/<topic> \
  --body "$(cat <<'EOF'
## Summary
- ...

## Testing
- ...
EOF
)"
```

5) **Troubleshooting `gh pr create`**
- Error: `Head sha can't be blank` / `Head ref must be a branch` / `No commits between main and <branch>` →
  the branch is not pushed or GitHub cannot see it. Push the branch (`git push -u origin <branch>`)
  and re-run `gh pr create`.

##### Project Structure (Current)
- `src/`
  - `cli.rs` — command parsing and flags
  - `config.rs` — config load/write
  - `context.rs` — packing policy
  - `ann.rs` — LSH signature helpers
  - `embed.rs` — embedding provider(s)
  - `ingest.rs` — file ingest + chunking
  - `model.rs` — shared domain types
  - `output.rs` — JSON response helpers
  - `query.rs` — retrieval, ranking, explain
  - `rql.rs` — parser, AST, validator
  - `store.rs` — single‑file engine schema + integrity
  - `transfer.rs` — export/import helpers
- `tests/`
  - unit and integration tests
- `DESIGN.md` — architecture and interfaces
- `DEVELOPMENT_RULES.md` — rules and invariants
- `AGENTS.md` — agent usage guide
- `ROADMAP.md` — milestones

##### Release Hygiene
- Update version numbers on breaking changes.
- Record migration steps for storage format changes.
- Keep RQL backward compatible whenever possible.

### Recall Workflows (WORKFLOWS.md)


Purpose: minimal default workflow for developing Recall while keeping docs, tests,
and history consistent. This is the canonical sequence referenced by
`DEVELOPMENT_RULES.md`.

##### Lean Workflow (Default)
###### 1) Scope + issue
- Read `DEVELOPMENT_RULES.md`; pick a milestone in `ROADMAP.md`.
- Create an issue file in `docs/issues/open/` from `docs/_templates/issue.md`.
- `git checkout main && git pull`
- `git checkout -b feat/<milestone>-<topic>` or `git checkout -b fix/<area>-<issue>`
- Move the issue to `docs/issues/active/` when work starts.

###### 2) Implement + track
- Small, reviewable steps; keep behavior deterministic.
- Log progress in `docs/progress/YYYY/YYYY-MM-DD.md`.
- Record ADRs in `docs/history/decisions/` for non-trivial decisions.
- Update `DESIGN.md`, `AGENTS.md`, and `ROADMAP.md` when behavior or scope changes.

###### 3) Validate
- Run unit tests for touched areas.
- Run the CLI flow: `recall init` -> `recall add` -> `recall search` -> `recall context`.
- Use `./x` for builds/tests/bench when applicable.
- Run benchmarks only for RCs or perf-sensitive changes.

###### 4) Commit + merge
- One coherent change per commit; milestone prefix when applicable.
- Mention schema/on-disk/JSON changes in the commit body.
- Rebase/squash to a minimal commit set.
- `git checkout main && git merge <branch> && git push`
- Move the issue to `docs/issues/done/`; add `docs/history/changes/` if user-visible.

Tracking docs layout and templates live in `docs/README.md`.

##### Scratch Context (Optional)
Use a temporary store for disposable context; keep `recall.db` out of VCS.

```
repo="$(pwd)"
tmpdir="$(mktemp -d)"
recall init "$tmpdir"
cd "$tmpdir"
recall add "$repo" --glob "**/*.{md,rs,toml}" --tag recall \
  --ignore "**/target/**" --ignore "**/.git/**"
recall search "query" --json
rm -rf "$tmpdir"
```

For iterative updates, re-run `recall add` with `--mtime-only`.
If you need persistence, initialize a store in the repo root and ignore
`recall.db` in `.gitignore`.

### Recall Roadmap (ROADMAP.md)


Date: 2026-02-02
Status: Planning v1.0; milestone issues created and ready to start.

##### v1.0 Plan (Proposed)
Target window: 2026-03-15 to 2026-04-30.

###### Milestone 6 — Interface Freeze + Compatibility
- Freeze CLI, RQL, and JSON schema for 1.0 (no breaking changes after this point).
- Define the supported upgrade path and migration guarantees for on-disk schema.
- Publish a compatibility matrix (OS/toolchain + embedding provider expectations).
  (issue: docs/issues/done/ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility.md)
- Add pipeline-style RQL clause order (FROM first, SELECT last) with legacy SELECT-first support.
  (issue: docs/issues/done/ISSUE-2026-02-02-rql-pipeline-clause-order.md)

###### Milestone 7 — Quality + Performance Baselines
- Establish a reproducible benchmark dataset or generator and publish baseline numbers.
- Define regression thresholds and add run-to-run determinism checks.
- Validate upgrades across prior schema versions with explicit migration tests.
- Refactor SQLite SQL to type-safe builder style.
  (issue: docs/issues/done/ISSUE-2026-02-02-sqlite-sql-builder-refactor.md)
  (issue: docs/issues/done/ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines.md)

###### Milestone 8 — Release Readiness
- Complete documentation updates (README, DESIGN, AGENTS, WORKFLOWS).
- README accuracy fixes from user feedback.
  (issue: docs/issues/done/ISSUE-2026-02-02-readme-doc-fixes.md)
- Package release artifacts with docs/scripts referenced in README.
  (issue: docs/issues/done/ISSUE-2026-02-02-release-packaging-docs.md)
- Produce a release candidate checklist with sign-off criteria and zero P0/P1 open issues.
- Cut v1.0 release notes and tag the release.
  (issue: docs/issues/active/ISSUE-2026-02-02-v1-0-m8-release-readiness.md)

##### Done
- MVP implementation: CLI, RQL/FEL, SQLite store, ingest, hybrid search, context assembly.
- Deterministic structured queries (default ordering + tests).
- File locking + busy timeout for single-writer, multi-reader.
- JSON `query.limit/offset` accuracy.
- LSH ANN shortlist stored in SQLite.
- Golden tests with `insta` + JSON schema validation.
- Hardened lexical search for FTS5 syntax errors with sanitization + warnings, plus tests.
- Export/import with snapshot hints in JSON stats.
- Enforced default git workflow in DEVELOPMENT_RULES.
- Squashed local main history before merge.
- Enforced deterministic tie-breaks for ORDER BY and search results.
- Added squash/rebase rule to WORKFLOWS.
- Backlog milestone alignment across issue metadata.
  (issue: docs/issues/done/ISSUE-2026-02-02-backlog-milestone-alignment.md)
- Documentation tidy-up pass for post-backlog clarity.
  (issue: docs/issues/done/ISSUE-2026-02-02-docs-tidyup.md)

##### Completed Milestones (Backlog Flush, ordered by principles)

###### Milestone 1 — Determinism + Explainability
- Canonicalize core principles across docs and add a glossary.
  (issue: docs/issues/done/ISSUE-2026-02-02-core-principles-canonical-glossary.md)
- Add `--snapshot` for search/query/context and support snapshot-based pagination with `OFFSET`.
  (issue: docs/issues/done/ISSUE-2026-02-01-snapshot-flag-search-query-context.md)
- Expand `--explain` and JSON diagnostics (resolved config, cache hints, candidate counts, lexical sanitization) and add per-stage timing breakdowns.
  (issue: docs/issues/done/ISSUE-2026-02-01-explain-search-mode-resolved-config.md)
- Fix CLI no-op flags and make `context --explain` effective.
  (issue: docs/issues/done/ISSUE-2026-02-02-cli-flag-hygiene-context-explain.md)

###### Milestone 2 — Local-first Reliability
- Add on-disk schema versioning + migrations (incl. ANN + FTS).
  (issue: docs/issues/done/ISSUE-2026-02-01-schema-versioning-migrations-ann-fts.md)
- Strengthen `recall doctor` with FTS/ANN checks, repair hints, `--fix`, and safer compact flows.
  (issue: docs/issues/done/ISSUE-2026-02-01-doctor-fix-safer-compact.md)
- Fix CLI store mode safety for stats/compact.
  (issue: docs/issues/done/ISSUE-2026-02-02-cli-store-mode-safety.md)
- Publish a release checklist and versioning policy.
  (issue: docs/issues/done/ISSUE-2026-02-01-release-checklist-versioning-policy.md)

###### Milestone 3 — Context as Managed Resource
- Add structure-aware chunking with markdown/code parsers (PDF deferred).
  (issue: docs/issues/done/ISSUE-2026-02-01-structure-aware-chunking.md)
- Improve JSON stats for corpus and memory usage.
  (issue: docs/issues/done/ISSUE-2026-02-01-json-stats-corpus-memory.md)
- Offer optional JSONL output for large result sets.
  (issue: docs/issues/done/ISSUE-2026-02-01-jsonl-output-large-results.md)

###### Milestone 4 — AI-native Interface
- Support `--rql-stdin` / `--filter @file` and lexical parsing controls.
  (issue: docs/issues/done/ISSUE-2026-02-01-rql-stdin-filter-file.md)
- Shell completions and `cargo install` guidance.
  (issue: docs/issues/done/ISSUE-2026-02-01-shell-completions-manpage-install.md)
- Add optional Markdown metadata extraction for doc-level filtering.
  (issue: docs/issues/done/ISSUE-2026-02-02-markdown-metadata-extraction.md)

###### Milestone 5 — Hybrid Retrieval Performance (Optional)
- Replace LSH shortlist with HNSW and add ANN migration/fallback.
  (issue: docs/issues/done/ISSUE-2026-02-01-ann-hnsw-backend.md)

##### Next
- Move the v1.0 plan issue to done when confirmed.
- Start Milestone 6 work and move its issue to active.

### Recall v1.0 Compatibility and Freeze (docs/COMPATIBILITY.md)


Date: 2026-02-02
Status: Draft (Milestone 6)

##### Interface Freeze (v1.0)
Beginning at Milestone 6, the following interfaces are frozen for v1.0:
- CLI flags and command behavior.
- RQL syntax and semantics.
- JSON output schema (including `schema_version`).

No breaking changes to these interfaces are permitted after this point. Any
breaking changes require a major version bump and a new schema version.

##### Upgrade and Migration Guarantees
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

##### Compatibility Matrix
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

##### Known Constraints
- Only the built-in `hash` embedder is supported in v1.0; external providers
  are not bundled.
- Cross-platform path normalization is OS-native; path separators differ.

### Recall Release Checklist & Versioning Policy (docs/RELEASE.md)


##### Versioning Policy
- Use semantic versioning (MAJOR.MINOR.PATCH).
- MAJOR: breaking CLI/RQL changes, JSON schema version bumps, or on-disk format changes.
- MINOR: backward-compatible features, new flags, or new JSON fields.
- PATCH: bug fixes and internal improvements with no interface changes.
- On-disk schema changes must include:
  - Migration logic.
  - A migration test.
  - A note in `docs/history/changes/`.
- JSON schema version changes must be documented and migrated in tooling.

##### Release Candidate Gate (v1.0)
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

##### Release Checklist
- [ ] Update `Cargo.toml` version.
- [ ] Run `./x fmt` and `./x clippy -- -D warnings`.
- [ ] Run `./x test` and review snapshot updates.
- [ ] Run core flows:
  - `recall init` → `recall add` → `recall search` → `recall context`.
- [ ] Run `scripts/bench_run.py` and compare against baseline thresholds.
- [ ] Verify migrations on an older store (schema version bump).
- [ ] Build release archive with `scripts/package_release.sh` and verify docs/scripts are included.
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

### Recall Benchmarks (docs/benchmarks/README.md)


Date: 2026-02-02
Status: Draft (Milestone 7)

##### Dataset Spec
- 10,000 documents, ~1,000,000 tokens total.
- Plain text (`.txt`) files, sharded into subdirectories for filesystem realism.
- Deterministic generation from a fixed seed.

##### Dataset Generator
Generate the benchmark dataset with:
```
python3 scripts/bench_gen.py --out /tmp/recall-bench --docs 10000 --tokens 1000000 --seed 42
```

The generator shards files into `shard-XXXX/` directories and adds a periodic
"needle" token to enable stable search queries.

##### Benchmark Runs
Preferred: release build (`target/release/recall`). Debug builds are acceptable
for smoke baselines, but release numbers should be used for gating.

Run the benchmark harness:
```
python3 scripts/bench_run.py --recall-bin target/debug/recall --dataset /tmp/recall-bench --docs 10000 --runs 20
```

The script reports p50/p95 latencies (ms) for search/query/context and ingest
throughput (docs/min). Capture results in a dated baseline file in this folder.
Current baseline: docs/benchmarks/baseline-2026-02-02.md.

##### Metrics
- Search/query/context latency: report p50 and p95 over 20 runs.
- Ingest throughput: docs/min for the full dataset ingest.

##### Regression Thresholds
Hard gates (from PRD NFRs):
- Search p95 <= 250ms
- Query p95 <= 300ms
- Context p95 <= 400ms
- Ingest throughput >= 1,000 docs/min

Relative regression gates:
- p95 latency regression > 15% versus the most recent baseline fails review.
- Ingest throughput regression > 10% versus baseline fails review.

##### Determinism Checks
Run the determinism tests to ensure repeated runs are identical:
```
cargo test deterministic_outputs
```

These tests validate repeated search/query/context output equality across 20
runs with a fixed snapshot token.

##### Migration Tests
Supported prior schema version: unversioned stores (pre-`schema_version`).
Run:
```
cargo test migrates_unversioned_store
```

This confirms migrations to schema version 1 for all supported prior stores.

### Recall Design Doc (DESIGN.md)


Date: 2026-02-02
Status: Draft (principles-first)

##### Product Summary
Recall is a local, single-file, CLI-first document store for AI agents. It provides deterministic, explainable hybrid retrieval with strict filters and builds token-budgeted context windows via a stable RQL interface.

##### Core Principles
Canonical source: this section defines the core principles and terms; other docs should link here.
1. Determinism over magic: identical inputs + store state yield identical outputs, including ordering and context assembly.
2. Hybrid retrieval with strict filters: semantic + lexical ranking is allowed, but FILTER constraints are exact and non-negotiable.
3. Local-first, zero-ops: single-file `recall.db`, offline by default, no required services.
4. Context as a managed resource: hard token budgets, deterministic packing, and provenance for every chunk.
5. AI-native interface: CLI and stable RQL are the source of truth; JSON outputs are stable for tooling.

###### Core Terms (Glossary)
- Strict filters: FILTER predicates are exact; no semantic inference, and any result must satisfy them.
- Deterministic packing: context assembly selects, orders, and truncates chunks in a fixed, documented way under a hard token budget.
- Provenance: each chunk retains path, offsets, hash, and mtime for traceability.

##### Scope (v0.1)
- Single-file store `recall.db` (SQLite-backed).
- CLI: `init`, `add`, `rm`, `search`, `query`, `context`, plus `stats`, `doctor`, `compact`.
- Hybrid retrieval: lexical (FTS5 BM25) + semantic embeddings with explicit weights.
- Deterministic ordering and tie-breaks; `--explain` for scoring stages.
- Budgeted context assembly with provenance and optional diversity cap.
- Stable `--json` output with schema versioning and JSONL streaming for large results.
- JSONL export/import for portability.
- Snapshot tokens for reproducible paging.
- On-disk schema versioning + migrations.
- Optional metadata extraction from Markdown headers/front matter.
- Structure-aware chunking (markdown headings and code blocks).

##### Non-goals
- Hosted multi-tenant service.
- Multi-writer OLTP concurrency.
- Complex analytics SQL.
- Real-time collaborative editing.

##### Interfaces

###### CLI (source of truth)
- `recall init [path]`
- `recall add <path...>`
- `recall rm <doc_id|path...>`
- `recall search <query>`
- `recall query --rql <string|@file>`
- `recall context <query>`
- `recall stats`, `recall doctor`, `recall compact`
- `recall export`, `recall import`
- `recall completions`, `recall guide`

###### RQL (AI-native)
```
FROM <table>
USING semantic(<text>) [, lexical(<text>)]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>]
SELECT <fields>;
```

Notes:
- `USING` enables semantic/lexical search; without it, queries are strict filters only.
- `FILTER` is exact; fields must be qualified (`doc.*`, `chunk.*`).
- `ORDER BY score` is meaningful only when `USING` is present.
- Unknown `SELECT` fields are ignored in v0.1 (permissive).
- Legacy `SELECT ... FROM ...` syntax is still accepted for compatibility.

###### Filter Expression Language (FEL)
```
<boolean-expr> := <term> ( (AND|OR) <term> )*
<term> := [NOT] <predicate> | '(' <boolean-expr> ')'
<predicate> := <field> <op> <value> | <field> IN '(' <value-list> ')'
<op> := = | != | < | <= | > | >= | LIKE | GLOB
```
- `LIKE` uses `%` and `_`; `GLOB` uses `*`, `?`, and `**`.
- ISO-8601 dates compare lexicographically; strings are case-sensitive.

##### Determinism and Explainability
- Stable IDs: `doc.id` = hash of normalized path + content hash; `chunk.id` = doc id + chunk offset.
- Deterministic ordering is always applied, even when `ORDER BY` is provided; ties are broken by:
  - `doc.path ASC`, then `chunk.offset ASC`, then `chunk.id ASC` (for `FROM chunk`).
  - `doc.path ASC`, then `doc.id ASC` (for `FROM doc`).
- Default ordering (when no `ORDER BY`):
  - With `USING`: `score DESC` then the same deterministic tie-breaks.
  - Without `USING`: `doc.path ASC` (and `chunk.offset ASC` for chunks).
- `FROM doc USING ...`: `score` is the max chunk score for that doc.
- `--explain` returns per-stage scores, resolved config, candidate counts, and lexical sanitization details.
- Per-stage timing breakdowns are included in JSON stats.
- Snapshot tokens (`--snapshot`) freeze results for reproducible pagination.

##### Hybrid Retrieval
- Lexical search via SQLite FTS5 (BM25-like); sanitized fallback if parsing fails.
- Semantic search via embeddings (default deterministic hash).
- Scores are normalized and combined with explicit weights from config.
- Filters are strict and never invoke semantic inference.
- ANN backend is configurable (`lsh`, `hnsw`, or `linear`) with LSH fallback.

##### Context Assembly
- Hard `budget_tokens`; context never exceeds the budget.
- Deterministic packing order mirrors retrieval ordering.
- De-duplication by chunk id; optional per-doc diversity cap.
- Truncation is deterministic (prefix to fit).
- Provenance for every chunk: path, offset, hash, mtime.

##### Storage and Local-first
- Single-file store `recall.db` backed by SQLite.
- Single-writer, multi-reader semantics with a sibling lock file.
- No network calls unless explicitly configured by the user.
- On-disk schema versions are stored in a `meta` table and migrated on open.

##### Compatibility and Upgrade Guarantees (v1.0)
- CLI, RQL, and JSON schema are frozen after Milestone 6.
- Breaking changes require a major version bump and schema version change.
- Unversioned stores are migrated to schema version 1 on open; newer schemas are rejected.
- See `docs/COMPATIBILITY.md` for the compatibility matrix and upgrade guidance.

##### Release Readiness (v1.0)
- Release checklist and RC gate definitions: `docs/RELEASE.md`.
- Performance baselines and regression thresholds: `docs/benchmarks/README.md`.

##### Data Model (Logical)
- `doc`: `id`, `path`, `mtime`, `hash`, `tag`, `source`, `meta`, `deleted`.
- `chunk`: `id`, `doc_id`, `offset`, `tokens`, `text`, `embedding`, `deleted`.
- `meta`: key/value schema metadata.

##### Document Metadata
- Opt-in ingest flag `--extract-meta` parses deterministic Markdown front matter
  or top-of-file `Key: Value` blocks.
- Extracted fields are stored as a doc-level metadata map (JSON) and exposed in
  `--json` outputs.
- RQL allows exact filters on metadata keys (e.g., `doc.meta.milestone`), with
  missing keys treated as null.
- Metadata keys are normalized to lowercase with `_` separators.

##### JSON Output (Stable)
Top-level fields:
- `ok`, `schema_version`, `query`, `results`, `context`, `stats`, `warnings`, `error`, `explain`.

Result entries include:
- `score`, `doc{...}`, `chunk{...}`, `explain{lexical, semantic}`.

Context entries include:
- `text`, `budget_tokens`, `used_tokens`, `chunks[{path, hash, mtime, offset, tokens, text}]`.

##### Configuration (Global recall.toml)
Recall uses an optional global config file in the OS config directory:
`<config_dir>/recall/recall.toml`.
- `store_path`
- `chunk_tokens`, `overlap_tokens`
- `embedding`, `embedding_dim`
- `ann_backend` (`lsh`, `hnsw`, `linear`)
- `ann_bits`, `ann_seed`
- `bm25_weight`, `vector_weight`
- `max_limit`

##### Future (Explicitly Out of MVP Scope)
- Additional parsers (PDF deferred).
- Background daemon/service mode.

### ISSUE-YYYY-MM-DD-<slug> (docs/_templates/issue.md)


Status: open | active | done
Milestone:
Owner:
Created: YYYY-MM-DD
Updated: YYYY-MM-DD

Context:
Scope:
Acceptance Criteria:
Out of Scope:
Notes:
Links:
- docs/history/decisions/ADR-YYYY-MM-DD-<slug>.md
- docs/progress/YYYY/YYYY-MM-DD.md

### Recall Tracking Docs (docs/README.md)


Purpose: track issue, progress, and history context for Recall using simple
Markdown files and path-based status.

##### Layout
- docs/issues/
  - open/   # backlog items
  - active/ # in progress
  - done/   # completed
- docs/progress/
  - YYYY/   # daily or session notes
- docs/history/
  - decisions/ # ADRs
  - changes/   # milestone or release summaries
- docs/_templates/ # copy these when creating new docs

##### Workflow
1) Create a new issue file in docs/issues/open/ using the issue template.
2) When work starts, move the file to docs/issues/active/.
3) Log progress in docs/progress/YYYY/YYYY-MM-DD.md and link the issue.
4) Record design decisions in docs/history/decisions/ and link back.
5) When done, move the issue file to docs/issues/done/ and add a change summary
   if user-visible behavior changed.

##### Templates
Copy from docs/_templates/ and adjust dates, slugs, and references.

### ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility (docs/issues/done/ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility.md)


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

### ISSUE-2026-02-02-rql-pipeline-clause-order (docs/issues/done/ISSUE-2026-02-02-rql-pipeline-clause-order.md)


Status: done
Milestone: v1.0 M6 — Interface Freeze + Compatibility
Owner: codex
Created: 2026-02-02
Updated: 2026-02-02

Context:
- RQL is currently SELECT-first; request to adopt a pipeline-style order.
- Interface rules require backward-compatible syntax changes.

Scope:
- Extend the RQL parser to accept FROM-first / SELECT-last form.
- Update docs/examples to present pipeline-style as canonical and note legacy support.
- Update CLI help and tests to cover the new syntax.

Acceptance Criteria:
- Parser accepts both SELECT-first and FROM-first forms without semantic changes.
- README/DESIGN/AGENTS reflect the pipeline-style order and mention legacy syntax.
- CLI help/examples and golden tests use the pipeline-style form.
- Tests pass for touched areas.

Out of Scope:
- Changing filter semantics or adding new RQL features.
- Removing legacy SELECT-first support.

Notes:
- Preserve deterministic ordering rules and output stability.

Links:
- docs/history/decisions/ADR-2026-02-02-rql-pipeline-clause-order.md
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-02-sqlite-sql-builder-refactor (docs/issues/done/ISSUE-2026-02-02-sqlite-sql-builder-refactor.md)


Status: done
Milestone: v1.0 M7 — Quality + Performance Baselines
Owner: codex
Created: 2026-02-02
Updated: 2026-02-02

Context:
- Dynamic SQLite queries are built with string formatting and ad-hoc placeholder lists.
- A typed builder would centralize column/table definitions and reduce stringly-typed SQL.

Scope:
- Add a type-safe SQL builder for SELECT queries and predicates.
- Refactor query filtering/order/joins in src/query.rs to use the builder.

Acceptance Criteria:
- Dynamic query construction uses the builder (no ad-hoc format! SQL for SELECT paths).
- Query behavior and ordering remain unchanged.
- Tests/build pass for touched areas when run.

Out of Scope:
- DDL schema strings and static INSERT/UPDATE statements.
- Changes to RQL syntax or CLI behavior.

Notes:
- Keep parameter ordering deterministic and preserve tie-breaks.

Links:
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines (docs/issues/done/ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines.md)


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

### ISSUE-2026-02-02-readme-doc-fixes (docs/issues/done/ISSUE-2026-02-02-readme-doc-fixes.md)


Status: done
Milestone: M8
Owner: Nexus
Created: 2026-02-02
Updated: 2026-02-02

Context:
- User feedback reports README inaccuracies around --json support, CLI synopsis, snapshot token usage, and single-file store wording.
- README references to docs/scripts may be missing in distributed packages; clarify source expectations.

Scope:
- Correct README JSON support language and CLI synopsis for add/rm.
- Make snapshot token usage reproducible with explicit capture + --snapshot example.
- Clarify single-file store vs config/lock files.
- Triage README references to docs/scripts and add guidance if they are absent in packaged distributions.

Acceptance Criteria:
- README no longer claims all commands support --json; coverage is accurate and explicit.
- CLI synopsis lists --json where supported for add/rm.
- Snapshot example shows how to capture stats.snapshot and re-use with --snapshot.
- Single-file store wording clarified to distinguish data vs config/lock.
- README references to docs/scripts include a note about source checkout vs packaged binaries.

Out of Scope:
- No CLI behavior changes or new flags.
- No packaging changes for release artifacts.

Notes:
- Source: user feedback on README inaccuracies (2026-02-02).
- Completed README corrections and roadmap link (2026-02-02).

Links:
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-02-release-packaging-docs (docs/issues/done/ISSUE-2026-02-02-release-packaging-docs.md)


Status: done
Milestone: M8
Owner: Nexus
Created: 2026-02-02
Updated: 2026-02-02

Context:
- README references docs/scripts that are missing from binary-only distributions.
- Release artifacts should ship the referenced docs/scripts alongside the binary.

Scope:
- Add a release packaging script that bundles the binary with docs/scripts.
- Update release checklist to include packaging step and verification.
- Update README note about packaged artifacts accordingly.
- Ignore packaging output in git.

Acceptance Criteria:
- A release packaging script produces an archive containing `recall` plus docs/scripts.
- Release checklist mentions the packaging step and validation.
- README notes that release archives include docs/scripts; binary-only installs may not.
- Packaging output directory is ignored by git.

Out of Scope:
- Changes to CLI behavior or install mechanisms.
- Publishing or distributing artifacts.

Notes:
- Source: user request to include docs/scripts in release packaging (2026-02-02).
- Completed packaging script, release checklist update, and README note (2026-02-02).

Links:
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-02-v1-0-m8-release-readiness (docs/issues/active/ISSUE-2026-02-02-v1-0-m8-release-readiness.md)


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

### ISSUE-2026-02-02-backlog-milestone-alignment (docs/issues/done/ISSUE-2026-02-02-backlog-milestone-alignment.md)


Status: done
Milestone: Milestone 0 — Workflow Hygiene
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
Backlog issues use shorthand milestone labels (M1–M6) that no longer align with
ROADMAP.md, which now names milestones explicitly. Aligning them makes the
backlog status quo easier to scan and keeps issue metadata consistent.

Scope:
- Update open and active issue milestone fields to match ROADMAP.md.
- Refresh Updated dates for edited issues.
- Record this hygiene task in ROADMAP.md and the daily progress log.

Acceptance Criteria:
- Open and active issues list milestone names that match ROADMAP.md.
- ROADMAP.md references this backlog hygiene issue.
- Progress log links this issue and notes the alignment work.

Out of Scope:
- Reprioritizing milestones or changing ROADMAP ordering.
- Editing done issues.

Notes:
- Documentation-only metadata alignment.

Links:
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-02-docs-tidyup (docs/issues/done/ISSUE-2026-02-02-docs-tidyup.md)


Status: done
Milestone: Maintenance
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
- Recent backlog-flush doc updates need a consistency and clarity pass.
Scope:
- Review recent documentation changes for clarity and alignment.
- Tidy README and ROADMAP wording for accuracy.
- Create/update repo-level todo list for doc hygiene.
Acceptance Criteria:
- Review findings are recorded and resolved or tracked.
- README and ROADMAP wording is consistent and unambiguous.
- todo.md exists and reflects the doc hygiene work.
Out of Scope:
- Behavior changes or new features.
Notes:

Links:
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-02-core-principles-canonical-glossary (docs/issues/done/ISSUE-2026-02-02-core-principles-canonical-glossary.md)


Status: done
Milestone: Milestone 1 — Determinism + Explainability
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
- Core principles appear in multiple docs with drift and undefined terms.
Scope:
- Define a canonical core principles section.
- Link other docs to the canonical source.
- Add a short glossary for key terms.
Acceptance Criteria:
- DESIGN.md declares the canonical core principles and includes a glossary.
- README.md and AGENTS.md reference the canonical source.
- Core principles wording is consistent across docs.
Out of Scope:
- Behavior changes or new features.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-02-core-principles-canonical-glossary.md
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-01-snapshot-flag-search-query-context (docs/issues/done/ISSUE-2026-02-01-snapshot-flag-search-query-context.md)


Status: done
Milestone: Milestone 1 — Determinism + Explainability
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Add explicit --snapshot for search/query/context.
- Accept snapshots for reproducible pagination with OFFSET.
Scope:
- Add --snapshot to search/query/context CLI commands.
- Accept snapshot tokens to make OFFSET paging reproducible.
- Ensure JSON stats/reporting exposes snapshot used or generated.
Acceptance Criteria:
- search/query/context accept --snapshot <token> and include it in query metadata.
- OFFSET + --snapshot returns stable ordering across runs for identical inputs and store state.
- Invalid snapshot tokens produce actionable, JSON-parseable errors.
Out of Scope:
- New snapshot formats beyond current snapshot token semantics.
Notes:
- Merged ISSUE-2026-02-01-snapshot-pagination-offset.

Links:
- docs/history/decisions/ADR-2026-02-01-snapshot-flag-search-query-context.md
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-01-explain-search-mode-resolved-config (docs/issues/done/ISSUE-2026-02-01-explain-search-mode-resolved-config.md)


Status: done
Milestone: Milestone 1 — Determinism + Explainability
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Add --explain diagnostics for effective search mode and resolved config.
- Add cache hints and candidate counts in --explain.
- Include lexical sanitization details in --json when FTS5 fallback is used.
- Add detailed per-stage timing breakdowns in stats.
- Ensure context --explain produces output consistent with search/query.
Scope:
- Expand --explain payload to include resolved config, mode, candidate counts, and cache hints.
- Add per-stage timing fields to stats in JSON output.
- Surface lexical sanitization details in JSON (and warnings in text mode).
- Ensure context --explain returns explain data or errors explicitly.
Acceptance Criteria:
- --explain reports mode (lexical/semantic/both) and resolved config values.
- JSON includes per-stage timing breakdowns with a stable schema.
- Lexical sanitization details are surfaced when fallback occurs.
- context --explain yields documented explain fields or a clear error.
Out of Scope:
- New ranking stages or rerankers.
Notes:
- Merged ISSUE-2026-02-01-explain-cache-hints-candidates.
- Merged ISSUE-2026-02-01-json-lexical-sanitization-details.
- Merged ISSUE-2026-02-01-timing-breakdowns-per-stage.

Links:
- docs/history/decisions/ADR-2026-02-01-explain-search-mode-resolved-config.md
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-02-cli-flag-hygiene-context-explain (docs/issues/done/ISSUE-2026-02-02-cli-flag-hygiene-context-explain.md)


Status: done
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

### ISSUE-2026-02-01-schema-versioning-migrations-ann-fts (docs/issues/done/ISSUE-2026-02-01-schema-versioning-migrations-ann-fts.md)


Status: done
Milestone: Milestone 2 — Local-first Reliability
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Add on-disk schema versioning + migrations (incl. ANN + FTS).
Scope:
- Add on-disk schema versioning + migrations (incl. ANN + FTS).
Acceptance Criteria:
- Store records a schema version in the database.
- Opening a store checks and migrates to the current version.
- ANN/FTS versioning is tracked for future migrations.
- Migration tests cover upgrade from unversioned stores.
Out of Scope:
- Implementing new ANN/FTS backends beyond version tracking.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-schema-versioning-migrations-ann-fts.md
- docs/progress/2026/2026-02-01.md

### ISSUE-2026-02-01-doctor-fix-safer-compact (docs/issues/done/ISSUE-2026-02-01-doctor-fix-safer-compact.md)


Status: done
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

### ISSUE-2026-02-02-cli-store-mode-safety (docs/issues/done/ISSUE-2026-02-02-cli-store-mode-safety.md)


Status: done
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

### ISSUE-2026-02-01-release-checklist-versioning-policy (docs/issues/done/ISSUE-2026-02-01-release-checklist-versioning-policy.md)


Status: done
Milestone: Milestone 2 — Local-first Reliability
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Publish a release checklist and versioning policy.
Scope:
- Publish a release checklist and versioning policy.
Acceptance Criteria:
- TBD
Out of Scope:
- TBD
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-release-checklist-versioning-policy.md
- docs/progress/2026/2026-02-01.md

### ISSUE-2026-02-01-structure-aware-chunking (docs/issues/done/ISSUE-2026-02-01-structure-aware-chunking.md)


Status: done
Milestone: Milestone 3 — Context as Managed Resource
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Add structure-aware chunking (headings, code blocks).
- Add markdown and code parsers (then PDF).
Scope:
- Implement structure-aware chunk boundaries using markdown/code parser output.
- Add markdown and code parsers to ingestion (PDF deferred).
Acceptance Criteria:
- Markdown headings and code fences influence chunk boundaries.
- Parser selection is deterministic and documented.
- Tests cover chunking behavior on sample markdown/code inputs.
Out of Scope:
- PDF parsing (deferred).
Notes:
- Merged ISSUE-2026-02-01-markdown-code-parsers.

Links:
- docs/history/decisions/ADR-2026-02-01-structure-aware-chunking.md
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-01-json-stats-corpus-memory (docs/issues/done/ISSUE-2026-02-01-json-stats-corpus-memory.md)


Status: done
Milestone: Milestone 3 — Context as Managed Resource
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Improve JSON stats for corpus and memory usage.
Scope:
- Improve JSON stats for corpus and memory usage.
Acceptance Criteria:
- TBD
Out of Scope:
- TBD
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-json-stats-corpus-memory.md
- docs/progress/2026/2026-02-01.md

### ISSUE-2026-02-01-jsonl-output-large-results (docs/issues/done/ISSUE-2026-02-01-jsonl-output-large-results.md)


Status: done
Milestone: Milestone 3 — Context as Managed Resource
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Offer optional JSONL output for large result sets.
Scope:
- Offer optional JSONL output for large result sets.
Acceptance Criteria:
- TBD
Out of Scope:
- TBD
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-jsonl-output-large-results.md
- docs/progress/2026/2026-02-01.md

### ISSUE-2026-02-01-rql-stdin-filter-file (docs/issues/done/ISSUE-2026-02-01-rql-stdin-filter-file.md)


Status: done
Milestone: Milestone 4 — AI-native Interface
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Support --rql-stdin / --filter @file for long agent queries.
- Add CLI controls for lexical query parsing (literal vs FTS5 syntax).
Scope:
- Add --rql-stdin to read RQL from stdin.
- Support --filter @file in CLI commands that accept filters.
- Add explicit lexical parsing controls for search.
Acceptance Criteria:
- recall query --rql-stdin reads RQL from stdin and validates empty input.
- --filter @file loads filter expressions from file for search/context.
- Lexical parsing mode is explicit, documented, and surfaced in JSON output.
Out of Scope:
- New query languages beyond RQL/FEL.
Notes:
- Merged ISSUE-2026-02-01-cli-lexical-parsing-controls.

Links:
- docs/history/decisions/ADR-2026-02-01-rql-stdin-filter-file.md
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-01-shell-completions-manpage-install (docs/issues/done/ISSUE-2026-02-01-shell-completions-manpage-install.md)


Status: done
Milestone: Milestone 4 — AI-native Interface
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Shell completions and cargo install guidance.
Scope:
- Shell completions and cargo install guidance.
Acceptance Criteria:
- TBD
Out of Scope:
- TBD
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-shell-completions-manpage-install.md
- docs/progress/2026/2026-02-01.md

### ISSUE-2026-02-02-markdown-metadata-extraction (docs/issues/done/ISSUE-2026-02-02-markdown-metadata-extraction.md)


Status: done
Milestone: Milestone 4 — AI-native Interface
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
During backlog hygiene, Recall could not filter or group issues by structured
fields like Status or Milestone because those values only exist as plain
Markdown text. We had to use external scripting to aggregate milestones.

Scope:
- Add optional metadata extraction for Markdown issue-style headers or front
  matter so values like Status/Milestone can be filtered in RQL.
- Expose extracted metadata in `--json` outputs for `doc` entries.
- Keep extraction deterministic and opt-in to avoid implicit behavior.

Acceptance Criteria:
- `recall add` can optionally extract metadata into a deterministic doc-level
  structure.
- RQL can FILTER on extracted metadata without free-text parsing.
- JSON outputs include the extracted metadata when present.

Out of Scope:
- Automatic schema inference for arbitrary text.
- Changing default ingestion behavior without an explicit flag.

Notes:
- This is a product hygiene/ergonomics gap identified during backlog work.

Links:
- docs/progress/2026/2026-02-02.md

### ISSUE-2026-02-01-ann-hnsw-backend (docs/issues/done/ISSUE-2026-02-01-ann-hnsw-backend.md)


Status: done
Milestone: Milestone 5 — Hybrid Retrieval Performance (Optional)
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Replace LSH shortlist with HNSW (or equivalent ANN backend).
- Add migration for ANN index format; keep LSH as fallback.
Scope:
- Implement HNSW (or equivalent) backend with opt-in config.
- Add migration path for ANN index format; keep LSH fallback.
Acceptance Criteria:
- Config allows selecting ANN backend; default remains LSH.
- Migration handles existing ANN data and preserves determinism.
- Tests cover both backends and migration path.
Out of Scope:
- Distributed ANN or remote services.
Notes:
- Merged ISSUE-2026-02-01-ann-index-migration-fallback-lsh.

Links:
- docs/history/decisions/ADR-2026-02-01-ann-hnsw-backend.md
- docs/progress/2026/2026-02-02.md

### CHANGE-2026-02-02-v1-0-release (docs/history/changes/CHANGE-2026-02-02-v1-0-release.md)


Status: draft (release pending)

Milestone: v1.0
Summary:
- Interface freeze for CLI/RQL/JSON schema and published compatibility matrix.
- Benchmark dataset, baselines, regression thresholds, and determinism checks.
- Release readiness checklist with RC gate definitions (including core principles).
- Release archives include docs/scripts referenced in README.
- Release tag: pending review

User impact:
- Stable interfaces for v1.0 tooling and agent integrations.
- Reproducible performance baselines and deterministic outputs.
- Clear upgrade guidance and compatibility expectations.
- Release archives ship docs/scripts; binary-only installs may not include them.
- Strict filters remain exact, and context budget/provenance guarantees remain enforced.

Migration:
- Automatic migration from unversioned stores to schema version 1 on open.
- Newer schema versions are rejected with a machine-parseable error.
- Back up `recall.db` before upgrading.

References:
- ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility
- ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines
- ISSUE-2026-02-02-v1-0-m8-release-readiness

### Benchmark Baseline (2026-02-02) (docs/benchmarks/baseline-2026-02-02.md)


##### Environment
- OS: macOS 26.2 (Build 25C56)
- CPU: Apple M4 Max
- Memory: 64 GB
- Storage: /dev/disk3s1s1 (926Gi, APFS)
- Rust toolchain: nightly-2026-02-01
- Recall build: debug (`target/debug/recall`)

##### Dataset
- Generator: scripts/bench_gen.py
- Docs: 10,000
- Tokens: ~1,000,000
- Seed: 42
- Shard size: 1,000 docs

##### Results (20 runs)
- Search latency: p50 97.3 ms, p95 101.4 ms
- Query latency: p50 97.4 ms, p95 99.0 ms
- Context latency: p50 101.1 ms, p95 108.0 ms
- Ingest throughput: 44,398 docs/min (13,514 ms total ingest)

##### Notes
- Dataset generated at /tmp/recall-bench with seed 42.
- Debug build baseline; capture a release baseline before v1.0 RC.

### PRD: Recall v1.0 (docs/PRD-v1.0.md)


##### Context
Recall is post‑MVP with the backlog flush complete. The next step is a v1.0
release focused on interface stability, deterministic behavior, and documented
upgrade guarantees, with a clear target window and release gates.

##### User Flow
1. User upgrades an existing store to v1.0 using documented steps and verifies
   compatibility.
2. User ingests documents and validates counts/metadata via `--json` outputs.
3. User runs `search`, `query`, and `context` with consistent ordering and
   snapshot‑stable results.
4. User exports/imports data and references v1.0 release notes for auditability.

##### Functional Requirements
- FR1: Interface freeze after Milestone 6; no breaking changes to CLI flags,
  RQL syntax, or JSON schema for v1.0.
- FR2: Upgrade path from any supported pre‑1.0 schema with no data loss;
  incompatible stores must fail with machine‑parseable errors and explicit
  remediation steps.
- FR3: Deterministic retrieval; identical store state + snapshot token yields
  identical ordering and JSON outputs across 20 repeated runs for `search`,
  `query`, and `context`.
- FR4: Compatibility matrix published for supported OS/toolchain and embedding
  provider expectations.
- FR5: Documentation complete for v1.0 (README, DESIGN, AGENTS, WORKFLOWS,
  and a v1.0 change summary).
- FR6: Release checklist completed and v1.0 tagged with release notes.

##### Non-Functional Requirements
- NFR1: Performance baselines on a documented reference dataset (min 10k docs,
  1M tokens) and reference hardware: p95 latency <= 250ms (`search`),
  <= 300ms (`query`), <= 400ms (`context`); ingest throughput >= 1,000
  docs/min.
- NFR2: Reliability verified over 10 cycles of ingest -> search/query/context
  -> compact/doctor with zero corruption and preserved determinism.
- NFR3: Stability gate: zero open P0/P1 issues at release; all tests pass
  without flakes.
- NFR4: Local‑first guarantee: no network access without explicit
  configuration, documented in release notes.

##### Tasks
- [x] T1: Break v1.0 plan into milestone issues (Acceptance Criteria: separate
  M6–M8 issue files with scope + acceptance criteria, linked from `ROADMAP.md`;
  see ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility,
  ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines,
  ISSUE-2026-02-02-v1-0-m8-release-readiness)
- [x] T2: Define benchmark dataset/generator and baseline metrics
  (Acceptance Criteria: benchmark spec in docs, baseline numbers recorded,
  regression thresholds defined; see docs/benchmarks/README.md)
- [x] T3: Draft interface freeze + compatibility matrix (Acceptance Criteria:
  published matrix and upgrade guidance documented; see docs/COMPATIBILITY.md)
- [x] T4: Define release candidate gate (Acceptance Criteria: v1.0 gate and
  P0/P1 definitions documented in `docs/RELEASE.md`)
- [x] T5: Prepare v1.0 change summary (Acceptance Criteria: v1.0 change log
  in `docs/history/changes/`)
- [ ] T6: Execute release checklist and tag v1.0 (Acceptance Criteria:
  checklist complete and tag recorded)

### 2026-02-02 (docs/progress/2026/2026-02-02.md)


Focus:
- Enforce default git workflow requirements in project rules.
- Apply deterministic ordering tie-breaks across query/search results.
- Squash local main history after rebase.
- Add squash/rebase rule to the workflow.

Progress:
- Created workflow enforcement issue and linked it from ROADMAP.
- Added default workflow requirement to DEVELOPMENT_RULES.
- Marked the workflow issue done and updated the roadmap.
- Opened the deterministic ordering tie-breaks issue and started patching.
- Completed deterministic ordering changes and updated the roadmap.
- Merged fix/deterministic-tiebreaks into main.
- Squashed recent workflow/determinism commits into two commits.
- Merged fix/squash-main-history into main.
- Added squash/rebase guidance to WORKFLOWS.
- Closed the workflow squash-rule issue and updated the roadmap.
- Merged fix/workflow-squash-rule into main.
- Logged metadata-extraction flaw from backlog hygiene and drafted design notes.
- Aligned backlog issue milestone labels with ROADMAP and refreshed Updated dates.
- Closed ISSUE-2026-02-02-backlog-milestone-alignment.
- Opened ISSUE-2026-02-02-core-principles-canonical-glossary and moved it to active.
- Clarified core principles wording, added canonical source note, and added a glossary in DESIGN.
- Merged fix/docs-core-principles into main.
- Moved ISSUE-2026-02-02-core-principles-canonical-glossary to done and updated ROADMAP link.
- Compacted backlog by merging related milestone issues and moving duplicates to done.
- Updated merged issue scopes and acceptance criteria to reflect consolidated work.
- Added CLI correctness issues for store mode safety and no-op flags/context explain.
- Updated ROADMAP to reflect the compacted backlog.
- Implemented schema versioning + migrations with meta table and migration tests.
- Added optional Markdown metadata extraction and `doc.meta` filtering/output.
- Added snapshot support, lexical parsing controls, `--rql-stdin`, and `--filter @file`.
- Expanded `--explain` payloads and per-stage timing breakdowns.
- Added JSONL streaming output and corpus/memory stats in JSON.
- Implemented structure-aware chunking with parser hints and tests.
- Added HNSW backend option with ann_hnsw index build and tests.
- Strengthened `doctor` with `--fix` and safe compact pre-checks.
- Added shell completions and guide output plus release checklist docs.
- Moved all milestone issues to done and updated ROADMAP/README/DESIGN/AGENTS.
- Opened ISSUE-2026-02-02-docs-tidyup for a documentation consistency pass.
- Completed doc tidy-up pass (README/ROADMAP/todo).
- Drafted the v1.0 plan and updated ROADMAP milestones.
- Opened ISSUE-2026-02-02-v1-0-plan and updated todo follow-ups.
- Persisted the v1.0 PRD and task checklist in docs.
- Broke v1.0 milestones into separate issues (M6–M8) and linked them from ROADMAP.
- Updated PRD task checklist to reflect milestone issue breakdown.
- Added todo.md for v1.0 follow-ups and closed ISSUE-2026-02-02-v1-0-plan.
- Started ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility (moved to active).
- Drafted v1.0 interface freeze + compatibility matrix in docs/COMPATIBILITY.md.
- Closed ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility and updated ROADMAP.
- Started ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines (moved to active).
- Added benchmark docs/scripts and determinism test scaffolding for Milestone 7.
- Captured benchmark baseline numbers and documented regression thresholds.
- Closed ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines and updated ROADMAP.
- Started ISSUE-2026-02-02-v1-0-m8-release-readiness (moved to active).
- Updated release readiness docs, RC gates, and v1.0 release notes.
- Release hold requested; ISSUE-2026-02-02-v1-0-m8-release-readiness remains active.
- Clarified RC gates and severity definitions for core principles (strict filters P0, context/provenance P1).
- Added explicit core principle verification references in docs/RELEASE.md.
- Opened ISSUE-2026-02-02-sqlite-sql-builder-refactor and moved it to active.
- Added a typed SQL builder module and refactored dynamic query construction in src/query.rs.
- Ran `cargo fmt` and `cargo check`.
- Closed ISSUE-2026-02-02-sqlite-sql-builder-refactor and updated ROADMAP.
- Opened ISSUE-2026-02-02-rql-pipeline-clause-order and recorded the ADR decision.
- Updated RQL parsing to accept FROM-first/SELECT-last and refreshed docs/examples/tests.
- Ran `./x fmt` and `./x test`.
- Closed ISSUE-2026-02-02-rql-pipeline-clause-order and updated ROADMAP.
- Opened ISSUE-2026-02-02-readme-doc-fixes and linked it from ROADMAP.
- Updated README for JSON coverage, snapshot token usage, and single-file store wording; added note about source-only docs/scripts.
- Moved ISSUE-2026-02-02-readme-doc-fixes to done and updated the ROADMAP link.
- Opened ISSUE-2026-02-02-release-packaging-docs and linked it from ROADMAP.
- Added release packaging script to bundle docs/scripts with binaries and updated README/RELEASE/CHANGE notes.
- Moved ISSUE-2026-02-02-release-packaging-docs to done and updated ROADMAP.

Decisions:
- Enforce the workflow in DEVELOPMENT_RULES for strongest default.
- Use doc.path + chunk.offset + chunk.id as universal tie-breaks.

Next:
- Push main to origin when ready.
- Review release readiness (M8) and approve tagging when ready.

References:
- ISSUE-2026-02-02-git-workflow-default
- ISSUE-2026-02-02-deterministic-ordering-tiebreaks
- ISSUE-2026-02-02-squash-main-history
- ISSUE-2026-02-02-workflow-squash-rule
- ISSUE-2026-02-02-markdown-metadata-extraction
- ISSUE-2026-02-02-backlog-milestone-alignment
- ISSUE-2026-02-02-core-principles-canonical-glossary
- ISSUE-2026-02-01-snapshot-flag-search-query-context
- ISSUE-2026-02-01-explain-search-mode-resolved-config
- ISSUE-2026-02-01-doctor-fix-safer-compact
- ISSUE-2026-02-01-structure-aware-chunking
- ISSUE-2026-02-01-rql-stdin-filter-file
- ISSUE-2026-02-01-ann-hnsw-backend
- ISSUE-2026-02-02-cli-store-mode-safety
- ISSUE-2026-02-02-cli-flag-hygiene-context-explain
- ISSUE-2026-02-02-v1-0-plan
- ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility
- ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines
- ISSUE-2026-02-02-v1-0-m8-release-readiness
- ISSUE-2026-02-02-sqlite-sql-builder-refactor
- ISSUE-2026-02-02-rql-pipeline-clause-order
- ISSUE-2026-02-02-readme-doc-fixes
- ISSUE-2026-02-02-release-packaging-docs
- ADR-2026-02-02-rql-pipeline-clause-order
- CHANGE-2026-02-02-rql-pipeline-clause-order
- Commits: 81ec30a, 4914894, e1b0f6d, b2c332a

### ADR-2026-02-02-rql-pipeline-clause-order (docs/history/decisions/ADR-2026-02-02-rql-pipeline-clause-order.md)


Status: accepted
Context:
- Users asked for a pipeline-friendly RQL clause order with FROM first and SELECT last.
- Interface rules require backward-compatible RQL syntax changes.
Decision:
- Accept FROM-first / SELECT-last as the canonical RQL order.
- Keep SELECT-first syntax supported for compatibility.
- Update docs/examples to present pipeline style and note legacy support.
Consequences:
- New pipeline style is documented and covered by tests.
- Legacy queries continue to parse without changes.
Alternatives:
- Break compatibility by enforcing a single new order (rejected).
- Keep SELECT-first as the only order (rejected).
Links:
- ISSUE-2026-02-02-rql-pipeline-clause-order

### Todo (todo.md)


Last updated: 2026-02-02

##### v1.0 planning follow-ups
- [x] Create milestone issues M6-M8 and link them from ROADMAP.
- [x] Capture v1.0 requirements in docs/PRD-v1.0.md.
- [x] Draft interface freeze + compatibility matrix (M6).
- [x] Define benchmark dataset/generator and baseline metrics (M7).
- [ ] Publish release readiness checklist + v1.0 release notes (M8).

##### Doc fixes from user feedback (2026-02-02)
- [x] README: correct "All commands support --json" (init lacks --json).
- [x] README: CLI synopsis should list --json for `recall add` and `recall rm`.
- [x] README: audit references to missing docs/scripts (DESIGN.md, AGENTS.md, ROADMAP.md, docs/*, ./x) and fix links or packaging.
- [x] README: make snapshot token workflow reproducible (show `stats.snapshot` capture and `--snapshot` usage).
- [x] README: clarify single-file store claim vs config/lock file presence.

### Recall (README.md)


Recall is a CLI-first, hybrid search database for AI agents working with large context. It is designed as “SQLite for document data” with deterministic retrieval, exact filtering, and semantic search.

##### Highlights
- CLI and RQL are the stable, top-level interfaces.
- Single-file local data store (`recall.db`) backed by SQLite + FTS5; optional global config in the OS config dir; lock file is temporary.
- Hybrid retrieval: lexical (FTS5 bm25) + semantic embeddings.
- Deterministic ordering and context assembly with token budgets and provenance.
- JSON outputs with schema validation and golden tests.
- Export/import for reproducible datasets.

##### Core Principles
Canonical definitions live in `DESIGN.md` under Core Principles.
- Determinism over magic: identical inputs + store state yield identical outputs, including ordering and context assembly.
- Hybrid retrieval with strict filters: semantic + lexical ranking is allowed, but FILTER constraints are exact and non-negotiable.
- Local-first, zero-ops: data store is a single file (`recall.db`); optional global config in the OS config dir; temporary lock file; offline by default, no required services.
- Context as a managed resource: hard token budgets, deterministic packing, and provenance for every chunk.
- AI-native interface: CLI and stable RQL are the source of truth; JSON outputs are stable for tooling.

##### What Recall Is For
Recall is a local, deterministic retrieval layer for agents and tools that need
repeatable access to large, evolving corpora. Think of it as “SQLite for
document data” with semantic search and exact filtering.

Use Recall when you want:
- A portable index where the data store is a single file (`recall.db`) you can move with a repo or dataset.
- Deterministic results across runs for agent workflows.
- A CLI-first surface you can script and automate.
- Hybrid search (semantic + lexical) without a hosted service.

##### Usage Scenarios
###### 1) Codebase Retrieval for Agents
Keep a `recall.db` per repo and use it for tool calls.
```
recall init .
recall add . --glob "**/*.{md,rs,ts,py}" --tag code
recall search "retry backoff" --filter "doc.path GLOB \"**/net/**\"" --json
```

###### 2) Product or Policy Knowledge Base
Maintain a curated corpus with tags and sources for audits.
```
recall init ./kb
recall add ./policies --glob "**/*.md" --tag policy --source "handbook"
recall query --rql "FROM doc FILTER doc.tag = \"policy\" LIMIT 20 SELECT doc.path;"
```

###### 3) Incident Response / Runbooks
Record the snapshot token for audit trails and reproducible paging.
```
recall search "rollback steps" --k 12 --json
# Copy stats.snapshot from the JSON output, then re-run deterministically:
recall search "rollback steps" --k 12 --snapshot TOKEN --json
recall export --out incident-2026-02-01.jsonl --json
```

###### 4) Research Notes and Papers
Use tags and filters to keep retrieval scoped and deterministic.
```
recall add ./papers --glob "**/*.txt" --tag research
recall context "evaluation methodology" --budget-tokens 1200 --diversity 2
```

###### 5) Agent Tooling Pipelines
Integrate `recall query --json` into pipelines for reproducible retrieval.
```
recall query --rql "FROM chunk USING semantic('SLO') LIMIT 6 SELECT chunk.text, score;" --json
```

##### Install (Local)
```
cargo build --release
```
The binary will be at `target/release/recall`.

###### Install (Cargo)
```
cargo install --path .
```
This installs the `recall` binary into your Cargo bin directory.

##### Quickstart
```
recall init .
recall add ./docs --glob "**/*.md" --tag docs
recall search "retry policy" --k 8 --filter "doc.tag = 'docs'" --json
recall context "how we handle retries" --budget-tokens 1200 --diversity 2
```

##### Shell Completions and Guide
Generate completions:
```
recall completions bash > /tmp/recall.bash
recall completions zsh > /tmp/_recall
recall completions fish > /tmp/recall.fish
```

Print the full usage guide:
```
recall guide
```

##### CLI Commands
```
recall init [path]
recall add <path...> [--glob ...] [--tag ...] [--source ...] [--mtime-only] [--ignore ...] [--parser auto|plain|markdown|code] [--extract-meta] [--json]
recall rm <doc_id|path...> [--purge] [--json]
recall search <query> [--k N] [--bm25] [--vector] [--filter ...|@file] [--lexical-mode fts5|literal] [--snapshot TOKEN] [--explain] [--json|--jsonl]
recall query --rql <string|@file> [--rql-stdin] [--lexical-mode fts5|literal] [--snapshot TOKEN] [--explain] [--json|--jsonl]
recall context <query> [--budget-tokens N] [--diversity N] [--format text|json] [--filter ...|@file] [--lexical-mode fts5|literal] [--snapshot TOKEN] [--explain] [--json]
recall stats [--json]
recall doctor [--json] [--fix]
recall compact [--json]
recall export [--out FILE] [--json]
recall import <FILE> [--json]
recall completions <shell>
recall guide
```

##### Metadata Extraction (Optional)
Use `--extract-meta` to parse deterministic header metadata from Markdown files
and filter on it in RQL:
```
recall add ./docs --glob "**/*.md" --extract-meta
recall search "migration" --filter "doc.meta.status = 'active'" --json
```

##### RQL (Recall Query Language)
Minimal shape:
```
FROM <table>
USING semantic(<text>) [, lexical(<text>)]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>]
SELECT <fields>;
```

Notes:
- `USING` enables semantic/lexical search; without it, queries are strict filters only.
- `FILTER` is exact and fields must be qualified (`doc.*`, `chunk.*`).
- Unknown `SELECT` fields are ignored in v0.1 (permissive).
- Legacy `SELECT ... FROM ...` syntax is still accepted for compatibility.

Example:
```
FROM chunk
USING semantic("rate limit"), lexical("429")
FILTER doc.path GLOB "**/api/**" AND chunk.tokens <= 256
ORDER BY score DESC
LIMIT 8
SELECT chunk.text, chunk.doc_id, score;
```

##### Filter Expression Language (FEL)
- `FILTER` fields must be qualified (`doc.*`, `chunk.*`).
- Metadata keys are available via `doc.meta.<key>` (keys are normalized to lowercase).
- `LIKE` uses SQL patterns (`%`, `_`).
- `GLOB` uses glob patterns (`*`, `?`, `**`).

Example:
```
FILTER doc.tag = "docs" AND doc.path GLOB "**/api/**"
```

##### JSON Output
Most commands support `--json` with a stable schema (including `schema_version`); `recall init`, `recall completions`, and `recall guide` are plain text only. Errors are machine-parseable and include `code` and `message`. A `stats.snapshot` token is provided as a reproducibility hint, and `--snapshot` accepts tokens for deterministic pagination. Use `--jsonl` for streaming large result sets from `recall search` and `recall query`.

##### Export / Import
Use JSONL for portability:
```
recall export --out recall.jsonl --json
recall import recall.jsonl --json
```

##### Development
Note: the files referenced below (including `./x`, `AGENTS.md`, `ROADMAP.md`, and `docs/`) live in the source checkout. Release archives built via `scripts/package_release.sh` include them; binary-only installs may not.

Use the `./x` helper for consistent workflows:
```
./x fmt
./x test
./x clippy -- -D warnings
```

##### Benchmarks
See `docs/benchmarks/README.md` for the benchmark dataset, baseline numbers,
and regression thresholds.

##### Workflows
See `AGENTS.md` → "Inlined Reference Documents" → `WORKFLOWS.md` for temporary
(volatile) workflows and end-to-end examples of using Recall to develop Recall.

##### Roadmap
See `ROADMAP.md`.

##### Compatibility
See `docs/COMPATIBILITY.md` for the v1.0 interface freeze, upgrade guarantees,
and compatibility matrix.

##### Releases
See `docs/RELEASE.md` for the release checklist and versioning policy. v1.0
release notes draft lives in `docs/history/changes/CHANGE-2026-02-02-v1-0-release.md`.

##### License
Apache-2.0. See `LICENSE`.

### 2026-02-01 (docs/progress/2026/2026-02-01.md)


Focus:
- ISSUE-2026-02-01-query-limit-offset-json: emit real query.limit/offset in JSON output.
- ISSUE-2026-02-01-deterministic-structured-queries: default deterministic ordering for structured queries.
- ISSUE-2026-02-01-file-locking-busy-timeout: single-writer, multi-reader locking + busy timeout.
- ISSUE-2026-02-01-schema-versioning-migrations-ann-fts: add schema versioning + migrations.

Progress:
- Opened issue and linked roadmap item.
- Sketched JSON output changes to expose limit/offset from parsed query and CLI defaults.
- Implemented limit/offset propagation from search/RQL into JSON output.
- Updated goldens and docs to reflect effective limit/offset values.
- Ran `cargo test --test cli_golden`.
- Moved query-limit-offset issue to done and added a change summary.
- Activated deterministic structured-query ordering issue.
- Implemented deterministic default ordering for structured RQL queries.
- Updated DESIGN notes and added a structured-ordering test.
- Ran `cargo test --test cli_golden`.
- Moved deterministic-structured-queries issue to done and added a change summary.
- Activated file-locking/busy-timeout issue.
- Implemented lock file handling, busy timeout, and read/write store modes.
- Added a shared-lock unit test and ran `cargo test`.
- Moved file-locking issue to done and added a change summary.
- Activated schema-versioning/migrations issue.

Decisions:
- None yet.

Next:
- Define schema version table and migration flow.
- Identify ANN/FTS version markers to track.
- Add migration tests for unversioned stores.

References:
- docs/issues/done/ISSUE-2026-02-01-query-limit-offset-json.md
- Commit: <sha>

### ISSUE-2026-02-01-query-limit-offset-json (docs/issues/done/ISSUE-2026-02-01-query-limit-offset-json.md)


Status: done
Milestone: M1
Owner:
Created: 2026-02-01
Updated: 2026-02-01

Context:
- Emit real query.limit/offset in JSON outputs.
Scope:
- Emit real query.limit/offset in JSON outputs.
Acceptance Criteria:
- JSON output reports the effective limit and offset for search/query/context.
- Values match parsed RQL or CLI defaults for each command.
- Snapshot/golden tests cover the updated JSON fields.
Out of Scope:
- Changing result ordering semantics beyond exposing limit/offset.
Notes:

Links:
- docs/history/decisions/ADR-2026-02-01-query-limit-offset-json.md
- docs/progress/2026/2026-02-01.md
