# Recall Roadmap

Date: 2026-02-02
Status: Principles-aligned plan (post-MVP)

## Done
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

## Next Milestones (Ordered by principles)

### Milestone 1 — Determinism + Explainability
- Add explicit `--snapshot` for search/query/context.
  (issue: docs/issues/open/ISSUE-2026-02-01-snapshot-flag-search-query-context.md)
- Accept snapshots for reproducible pagination with `OFFSET`.
  (issue: docs/issues/open/ISSUE-2026-02-01-snapshot-pagination-offset.md)
- Add detailed timing breakdowns (`took_ms` per stage).
  (issue: docs/issues/open/ISSUE-2026-02-01-timing-breakdowns-per-stage.md)
- Add cache hints and candidate counts in `--explain`.
  (issue: docs/issues/open/ISSUE-2026-02-01-explain-cache-hints-candidates.md)
- Add `--explain` diagnostics for effective search mode and resolved config.
  (issue: docs/issues/open/ISSUE-2026-02-01-explain-search-mode-resolved-config.md)
- Include lexical sanitization details in `--json` when FTS5 fallback is used.
  (issue: docs/issues/open/ISSUE-2026-02-01-json-lexical-sanitization-details.md)

### Milestone 2 — Local-first Reliability
- Add on-disk schema versioning + migrations (incl. ANN + FTS).
  (issue: docs/issues/active/ISSUE-2026-02-01-schema-versioning-migrations-ann-fts.md)
- Strengthen `recall doctor` (FTS/ANN consistency checks + repair hints).
  (issue: docs/issues/open/ISSUE-2026-02-01-doctor-consistency-checks-repair-hints.md)
- Add `recall doctor --fix` and safer compact flows.
  (issue: docs/issues/open/ISSUE-2026-02-01-doctor-fix-safer-compact.md)
- Publish a release checklist and versioning policy.
  (issue: docs/issues/open/ISSUE-2026-02-01-release-checklist-versioning-policy.md)

### Milestone 3 — Context as Managed Resource
- Add structure-aware chunking (headings, code blocks).
  (issue: docs/issues/open/ISSUE-2026-02-01-structure-aware-chunking.md)
- Add markdown and code parsers (then PDF).
  (issue: docs/issues/open/ISSUE-2026-02-01-markdown-code-parsers.md)
- Improve JSON stats for corpus and memory usage.
  (issue: docs/issues/open/ISSUE-2026-02-01-json-stats-corpus-memory.md)
- Offer optional JSONL output for large result sets.
  (issue: docs/issues/open/ISSUE-2026-02-01-jsonl-output-large-results.md)

### Milestone 4 — AI-native Interface
- Support `--rql-stdin` / `--filter @file` for long agent queries.
  (issue: docs/issues/open/ISSUE-2026-02-01-rql-stdin-filter-file.md)
- Add CLI controls for lexical query parsing (literal vs FTS5 syntax).
  (issue: docs/issues/open/ISSUE-2026-02-01-cli-lexical-parsing-controls.md)
- Shell completions, man page, and `cargo install` guidance.
  (issue: docs/issues/open/ISSUE-2026-02-01-shell-completions-manpage-install.md)

### Milestone 5 — Hybrid Retrieval Performance (Optional)
- Replace LSH shortlist with HNSW (or equivalent ANN backend).
  (issue: docs/issues/open/ISSUE-2026-02-01-ann-hnsw-backend.md)
- Add migration for ANN index format; keep LSH as fallback.
  (issue: docs/issues/open/ISSUE-2026-02-01-ann-index-migration-fallback-lsh.md)
