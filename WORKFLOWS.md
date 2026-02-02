# Recall Workflows

This document describes practical workflows for using Recall to develop Recall,
with a focus on temporary (volatile) indexes you can throw away after a session.

## Complete Workflow (Merged Summary)
Condensed end-to-end flow derived from `AGENTS.md`, `HANDBOOK.md`, and this
document. Use this as the default operating sequence.

### 1) Plan + Track
- Read `DEVELOPMENT_RULES.md` and pick a milestone from `ROADMAP.md`.
- Create an issue file in `docs/issues/open/` and link it from `ROADMAP.md`.
- Create a short-lived branch (`feat/<milestone>-<topic>` or `fix/<area>-<issue>`).

### 2) Git Workflow (Branch/Checkout/Merge)
- Update your local main, then branch: `git checkout main` → `git pull`.
- Create and switch to a short-lived branch scoped to a single issue:
  `git checkout -b feat/<milestone>-<topic>` or `git checkout -b fix/<area>-<issue>`.
- Keep scope tight: one branch per issue file, merge quickly, and avoid mixing topics.
- Commit in small, reviewable steps; keep `main` green.
- Commit message style (milestone-driven):
  - Prefix with milestone tag when applicable: `M1: <summary>`.
  - One coherent change per commit; avoid mega-commits.
  - Add a short “why” in the body if non-obvious.
  - Mention schema/on-disk/JSON changes explicitly in the body.
- Squash or rebase before merging so each issue lands as a minimal set of commits.
- Integrate document workflow with git:
  - Ensure `docs/issues/open/ISSUE-*.md` exists before branching.
  - Move to `docs/issues/active/` when work starts; log progress and commit SHAs
    in `docs/progress/YYYY/YYYY-MM-DD.md`.
  - Capture ADRs in `docs/history/decisions/` and link them from the issue file.
  - Move to `docs/issues/done/` when merged; add a change summary if user-visible.
- Merge back when validated: `git checkout main` → `git merge <branch>` → `git push`.

### 3) Build Context (Optional but recommended)
- Use a temporary store unless you need persistence.
- `recall init` → `recall add` (narrow globs) → `recall search/query/context`.
- Prefer `recall query --rql` for precise retrieval; use `FILTER` for exact constraints.

### 4) Implement + Record
- Work in small, reviewable steps; keep behavior deterministic.
- Log progress in `docs/progress/YYYY/YYYY-MM-DD.md`.
- Capture design decisions in `docs/history/decisions/` (ADRs).
- If user-visible behavior changes, add a summary in `docs/history/changes/`.

### 5) Validate
- Run unit tests for touched areas and relevant CLI flows:
  `recall init` → `recall add` → `recall search` → `recall context`.
- Use `./x` for consistent builds/tests/benchmarks when applicable.
- Update golden/snapshot tests for JSON output changes.

### 6) Document + Commit
- Update `DESIGN.md`, `AGENTS.md`, and `ROADMAP.md` when behavior or scope changes.
- Commit one coherent change; include milestone tag and any schema/migration notes.

### 7) Agent Output Contract (When emitting JSON)
- Include citations (doc path + chunk offsets) when presenting results.
- `query.limit`/`query.offset` report effective values after defaults (RQL, `--k`, etc.).

## Temporary (Volatile) Workflow
Use a scratch store to avoid polluting the repo and to keep experiments clean.
The scratch store lives in a temp directory; delete it when done.

### Goals
- No permanent `recall.db` in the repo.
- Fast setup for a short session.
- Repeatable, deterministic retrieval within that session.

### Steps
1) Create a temp store directory:
```
tmpdir="$(mktemp -d)"
```

2) Initialize Recall there:
```
recall init "$tmpdir"
```

3) Index the Recall repo from the temp dir:
```
cd "$tmpdir"
recall add /home/leiysky/work/recall \
  --glob "**/*.{md,rs,toml}" \
  --tag code \
  --ignore "**/target/**" \
  --ignore "**/.git/**" \
  --ignore "**/recall.db"
```

4) Query and assemble context:
```
recall search "RQL strict filters" --filter "doc.path GLOB \"**/src/**\"" --json
recall query --rql "SELECT chunk.text FROM chunk USING semantic('ann lsh') LIMIT 6;"
recall context "how ordering works" --budget-tokens 1200 --diversity 2
```

5) Cleanup when done:
```
rm -rf "$tmpdir"
```

## End-to-End Example: One-Shot Investigation
```
tmpdir="$(mktemp -d)"
recall init "$tmpdir"
cd "$tmpdir"

# Index only docs + core Rust
recall add /home/leiysky/work/recall \
  --glob "**/*.{md,rs}" \
  --tag recall \
  --ignore "**/target/**" \
  --ignore "**/.git/**"

# Ask a question and collect context
recall search "snapshot token meaning" --json
recall context "snapshot token meaning" --budget-tokens 800 --diversity 2 --json

# Tear down
rm -rf "$tmpdir"
```

## End-to-End Example: Iterative Session
Use `--mtime-only` to keep the temporary store fresh during active edits.
```
tmpdir="$(mktemp -d)"
recall init "$tmpdir"
cd "$tmpdir"

recall add /home/leiysky/work/recall \
  --glob "**/*.{md,rs,toml}" \
  --tag recall \
  --ignore "**/target/**" \
  --ignore "**/.git/**"

# After edits
recall add /home/leiysky/work/recall \
  --glob "**/*.{md,rs,toml}" \
  --mtime-only \
  --ignore "**/target/**" \
  --ignore "**/.git/**"

recall query --rql "SELECT doc.path FROM doc FILTER doc.tag = \"recall\" LIMIT 10;"
recall context "ordering rules" --budget-tokens 1200 --diversity 2

rm -rf "$tmpdir"
```

## Notes
- Temporary stores are ideal for volatile context (experiments, branches, PRs).
- If you need persistence, initialize a store in the repo root instead.
- Keep `recall.db` out of version control if you do persist it.

## Document Tracking Workflow (Issues/Progress/History)
Use Markdown files to track issue state, daily progress, and decisions in a
path-based, deterministic layout. Status is derived from folder names so it
can be queried with exact filters.

### Layout
- `docs/issues/open/`   — backlog items
- `docs/issues/active/` — in progress
- `docs/issues/done/`   — completed
- `docs/progress/YYYY/` — daily or session logs
- `docs/history/decisions/` — ADRs
- `docs/history/changes/` — change summaries
- `docs/_templates/` — copy templates for new docs

### Steps
1) Create an issue file by copying `docs/_templates/issue.md` into
   `docs/issues/open/` and fill in the header fields.
2) When work starts, move the file to `docs/issues/active/`.
3) Log progress in `docs/progress/YYYY/YYYY-MM-DD.md`, linking the issue and
   any relevant commits.
4) Capture design decisions as ADRs in `docs/history/decisions/`.
5) When done, move the issue file to `docs/issues/done/` and create a change
   summary in `docs/history/changes/` if user-visible behavior changed.

### Recall Queries
- List open issues:
```
recall query --rql "SELECT doc.path FROM doc FILTER doc.path GLOB '**/docs/issues/open/**' ORDER BY doc.mtime DESC LIMIT 20;"
```
- Build context for an issue:
```
recall context "ISSUE-YYYY-MM-DD-<slug>" --filter "doc.path GLOB '**/docs/**'" --budget-tokens 1200 --diversity 2
```
