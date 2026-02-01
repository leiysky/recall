# Recall Engineering Handbook

Purpose: define how we iterate on Recall, validate changes, and keep the project consistent.
For the consolidated end-to-end workflow (including git steps), see
`WORKFLOWS.md` → "Complete Workflow (Merged Summary)".

## Iteration Workflow
1. Define scope: pick a milestone or a small slice of it.
2. Create a branch: `feat/<milestone>-<topic>` or `fix/<area>-<issue>`.
3. Implement in small, reviewable steps.
4. Validate locally (tests + sample CLI flows).
5. Update docs (`DESIGN.md`, `AGENTS.md`, `ROADMAP.md`) if behavior changes.
6. Commit with a clear message and link to milestone.

## Branching Rules
- Use short‑lived branches; merge quickly.
- Branch names:
  - `feat/m1-rql-parser`
  - `fix/storage-locking`
  - `docs/handbook-update`
- Keep `main` green; never merge without tests.

## Commit Rules (Milestone‑Driven)
- One commit per coherent change; avoid mega‑commits.
- Prefix commits with milestone tags when applicable.
  - Example: `M1: add RQL lexer and parser`
- Include a short “why” in the commit body if non‑obvious.
- If a change alters on‑disk format or JSON schema, mention it explicitly.

## Validation and Testing
- Use `./x` to run builds/tests/benchmarks consistently.
- Always run unit tests for modified areas.
- For storage or query changes, run integration flows:
  - `recall init` → `recall add` → `recall search` → `recall context`.
- Add deterministic tests for ordering and context packing.
- Add migration tests when on‑disk format changes.
- Avoid flaky tests; deterministic output is a requirement.

## Project Structure (Current)
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

## Release Hygiene
- Update version numbers on breaking changes.
- Record migration steps for storage format changes.
- Keep RQL backward compatible whenever possible.
