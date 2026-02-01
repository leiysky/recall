# Recall Roadmap

Date: 2026-02-01
Status: MVP complete; roadmap refreshed

## Done
- MVP implementation: CLI, RQL/FEL, SQLite store, ingest, hybrid search, context assembly.
- Golden tests with `insta` + JSON schema validation.
- LSH ANN shortlist stored in SQLite.
- Export/import with snapshot hints in JSON stats.
- Docs aligned with implemented behavior.
- Hardened lexical search for FTS5 syntax errors with sanitization + warnings, plus tests for
  sanitized queries and structured filters.

## Next Milestones (Ordered)

### Milestone 1 — Release Hardening + Compatibility
- Add on-disk schema versioning + migrations (incl. ANN + FTS). (issue: docs/issues/active/ISSUE-2026-02-01-schema-versioning-migrations-ann-fts.md)
- Implement file locking + busy timeout for single-writer, multi-reader. (issue: docs/issues/done/ISSUE-2026-02-01-file-locking-busy-timeout.md)
- Make structured queries deterministic or require explicit `ORDER BY`. (issue: docs/issues/done/ISSUE-2026-02-01-deterministic-structured-queries.md)
- Strengthen `recall doctor` (FTS/ANN consistency checks + repair hints). (issue: docs/issues/open/ISSUE-2026-02-01-doctor-consistency-checks-repair-hints.md)
- Emit real `query.limit/offset` in JSON outputs. (issue: docs/issues/done/ISSUE-2026-02-01-query-limit-offset-json.md)

### Milestone 2 — Snapshot Pagination API
- Add explicit `--snapshot` for search/query/context. (issue: docs/issues/open/ISSUE-2026-02-01-snapshot-flag-search-query-context.md)
- Accept snapshots for reproducible pagination with `OFFSET`. (issue: docs/issues/open/ISSUE-2026-02-01-snapshot-pagination-offset.md)

### Milestone 3 — Performance + Explainability
- Add detailed timing breakdowns (`took_ms` per stage). (issue: docs/issues/open/ISSUE-2026-02-01-timing-breakdowns-per-stage.md)
- Add cache hints and candidate counts in `--explain`. (issue: docs/issues/open/ISSUE-2026-02-01-explain-cache-hints-candidates.md)
- Improve JSON stats for corpus and memory usage. (issue: docs/issues/open/ISSUE-2026-02-01-json-stats-corpus-memory.md)
- Add `--explain` diagnostics for effective search mode and resolved config (weights, ANN bits). (issue: docs/issues/open/ISSUE-2026-02-01-explain-search-mode-resolved-config.md)
- Include lexical sanitization details in `--json` when FTS5 fallback is used. (issue: docs/issues/open/ISSUE-2026-02-01-json-lexical-sanitization-details.md)

### Milestone 4 — ANN Backend Upgrade
- Replace LSH shortlist with HNSW (or equivalent ANN backend). (issue: docs/issues/open/ISSUE-2026-02-01-ann-hnsw-backend.md)
- Add migration for ANN index format; keep LSH as fallback. (issue: docs/issues/open/ISSUE-2026-02-01-ann-index-migration-fallback-lsh.md)

### Milestone 5 — Parser Adapters + Chunking
- Add markdown and code parsers (then PDF). (issue: docs/issues/open/ISSUE-2026-02-01-markdown-code-parsers.md)
- Add structure-aware chunking (headings, code blocks). (issue: docs/issues/open/ISSUE-2026-02-01-structure-aware-chunking.md)

### Milestone 6 — Distribution + UX Polish
- Shell completions, man page, and `cargo install` guidance. (issue: docs/issues/open/ISSUE-2026-02-01-shell-completions-manpage-install.md)
- Add `recall doctor --fix` and safer compact flows. (issue: docs/issues/open/ISSUE-2026-02-01-doctor-fix-safer-compact.md)
- Publish a release checklist and versioning policy. (issue: docs/issues/open/ISSUE-2026-02-01-release-checklist-versioning-policy.md)
- Add CLI controls for lexical query parsing (literal vs FTS5 syntax). (issue: docs/issues/open/ISSUE-2026-02-01-cli-lexical-parsing-controls.md)
- Support `--rql-stdin` / `--filter @file` for long agent queries. (issue: docs/issues/open/ISSUE-2026-02-01-rql-stdin-filter-file.md)
- Offer optional JSONL output for large result sets. (issue: docs/issues/open/ISSUE-2026-02-01-jsonl-output-large-results.md)
