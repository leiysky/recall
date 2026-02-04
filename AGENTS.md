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
- Retrieval is deterministic; reranker stages are future work.
- Snapshot tokens (`--snapshot`) freeze results for reproducible paging.

## Required Workflow (Enforced)
- Development Rules are mandatory and inlined below.
- The Engineering Handbook and Lean Workflow (Default) govern branching and validation.
- Planning sources of truth: `README.md`.

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
- Unknown `SELECT` fields are ignored (permissive).
- `SELECT ... FROM ...` is still accepted.

#### Field Catalog (Initial)
- `doc.id`, `doc.path`, `doc.mtime`, `doc.hash`, `doc.tag`, `doc.source`, `doc.meta`.
- `chunk.id`, `chunk.doc_id`, `chunk.offset`, `chunk.tokens`, `chunk.text`.
Note: `doc.size` is stored but not exposed in RQL.
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
- `recall add ./data --glob "**/*.md" --extract-meta --json`
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
- **MUST** keep `--json` output stable.
- **MUST** return actionable errors; in `--json`, errors are machine-parseable.

##### Query Language (RQL)
- **MUST** keep RQL stable; avoid breaking syntax or semantics.
- **MUST** keep `FILTER` strict; fields must be qualified (`doc.*`, `chunk.*`).
- **SHOULD** keep `SELECT` field handling permissive (unknown fields are ignored).
- **MUST** document any move to strict `SELECT` validation.
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
- **SHOULD** update `ROADMAP.md` if scope changes.

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

- Scope: define the work item.
- Branch: short‑lived; `feat/<topic>` or `fix/<topic>`.
- Implement: small, reviewable steps; keep behavior deterministic.
- Docs: update `DESIGN.md`, `AGENTS.md`, `ROADMAP.md` when behavior/scope changes; add ADRs for non‑trivial decisions.
- Validate: unit tests for touched areas; run `recall init` → `recall add` → `recall search` → `recall context`; use `./x` where applicable; add determinism/migration tests when needed.
- Commit/merge: one coherent change per commit; mention schema/on‑disk/JSON changes; keep `main` green; rebase/squash to minimal commits before merge.

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
- Prefer a body file (no shell-escaping pitfalls):
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
- `ROADMAP.md` — roadmap

### Recall Workflows (WORKFLOWS.md)


Purpose: minimal default workflow for developing Recall while keeping docs and tests
consistent. This is the canonical sequence referenced by
`DEVELOPMENT_RULES.md`.

##### Lean Workflow (Default)
###### 1) Scope + branch
- Read `DEVELOPMENT_RULES.md`; pick a scope in `ROADMAP.md`.
- `git checkout main && git pull`
- `git checkout -b feat/<topic>` or `git checkout -b fix/<topic>`

###### 2) Implement + track
- Small, reviewable steps; keep behavior deterministic.
- Update `DESIGN.md`, `AGENTS.md`, and `ROADMAP.md` when behavior or scope changes.

###### 3) Validate
- Run unit tests for touched areas.
- Run the CLI flow: `recall init` -> `recall add` -> `recall search` -> `recall context`.
- Use `./x` for builds/tests/bench when applicable.
- Run benchmarks for perf-sensitive changes.

###### 4) Commit + merge
- One coherent change per commit.
- Mention schema/on-disk/JSON changes in the commit body.
- Rebase/squash to a minimal commit set.
- `git checkout main && git merge <branch> && git push`

Documentation system is being rebuilt.

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

##### Scope
- Single-file store `recall.db` (SQLite-backed).
- CLI: `init`, `add`, `rm`, `search`, `query`, `context`, plus `stats`, `doctor`, `compact`.
- Hybrid retrieval: lexical (FTS5 BM25) + semantic embeddings with explicit weights.
- Deterministic ordering and tie-breaks; `--explain` for scoring stages.
- Budgeted context assembly with provenance and optional diversity cap.
- Stable `--json` output and JSONL streaming for large results.
- JSONL export/import for portability.
- Snapshot tokens for reproducible paging.
- On-disk schema migrations.
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
- Unknown `SELECT` fields are ignored (permissive).
- `SELECT ... FROM ...` syntax is still accepted.

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
- Semantic search via embeddings (default embedded model2vec) using sqlite-vec `vec0` with cosine distance.
- Scores are normalized and combined with explicit weights from config.
- Filters are strict and never invoke semantic inference.

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
- On-disk schema metadata is stored in a `meta` table; unsupported schema versions are rejected (re-init required).

##### Data Model (Logical)
- `doc`: `id`, `path`, `mtime`, `hash`, `tag`, `source`, `meta`, `deleted`.
- `chunk`: `id`, `doc_id`, `offset`, `tokens`, `text`, `embedding`, `deleted`.
- `chunk_vec`: sqlite-vec virtual table keyed by `chunk_rowid` with `embedding` for KNN.
- `meta`: key/value schema metadata.

##### Document Metadata
- Opt-in ingest flag `--extract-meta` parses deterministic Markdown front matter
  or top-of-file `Key: Value` blocks.
- Extracted fields are stored as a doc-level metadata map (JSON) and exposed in
  `--json` outputs.
- RQL allows exact filters on metadata keys (e.g., `doc.meta.category`), with
  missing keys treated as null.
- Metadata keys are normalized to lowercase with `_` separators.

##### JSON Output (Stable)
Top-level fields:
- `ok`, `query`, `results`, `context`, `stats`, `warnings`, `error`, `explain`.

Result entries include:
- `score`, `doc{...}`, `chunk{...}`, `explain{lexical, semantic}`.

Context entries include:
- `text`, `budget_tokens`, `used_tokens`, `chunks[{path, hash, mtime, offset, tokens, text}]`.

##### Configuration (Global recall.toml)
Recall uses an optional global config file in the OS config directory:
`<config_dir>/recall/recall.toml`. On Unix (including macOS), this follows XDG
(`$XDG_CONFIG_HOME` or `$HOME/.config`).
- `store_path`
- `chunk_tokens`, `overlap_tokens`
- `embedding`, `embedding_dim`
- `bm25_weight`, `vector_weight`
- `max_limit`

##### Future (Explicitly Out of MVP Scope)
- Additional parsers (PDF deferred).
- Background daemon/service mode.
