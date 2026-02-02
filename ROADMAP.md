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
- Canonicalize core principles across docs and add a glossary.
  (issue: docs/issues/done/ISSUE-2026-02-02-core-principles-canonical-glossary.md)
- Add `--snapshot` for search/query/context and support snapshot-based pagination with `OFFSET`.
  (issue: docs/issues/open/ISSUE-2026-02-01-snapshot-flag-search-query-context.md)
- Expand `--explain` and JSON diagnostics (resolved config, cache hints, candidate counts, lexical sanitization) and add per-stage timing breakdowns.
  (issue: docs/issues/open/ISSUE-2026-02-01-explain-search-mode-resolved-config.md)
- Fix CLI no-op flags and make `context --explain` effective.
  (issue: docs/issues/open/ISSUE-2026-02-02-cli-flag-hygiene-context-explain.md)

### Milestone 2 — Local-first Reliability
- Add on-disk schema versioning + migrations (incl. ANN + FTS).
  (issue: docs/issues/active/ISSUE-2026-02-01-schema-versioning-migrations-ann-fts.md)
- Strengthen `recall doctor` with FTS/ANN checks, repair hints, `--fix`, and safer compact flows.
  (issue: docs/issues/open/ISSUE-2026-02-01-doctor-fix-safer-compact.md)
- Fix CLI store mode safety for stats/compact.
  (issue: docs/issues/open/ISSUE-2026-02-02-cli-store-mode-safety.md)
- Publish a release checklist and versioning policy.
  (issue: docs/issues/open/ISSUE-2026-02-01-release-checklist-versioning-policy.md)

### Milestone 3 — Context as Managed Resource
- Add structure-aware chunking with markdown/code parsers (PDF deferred).
  (issue: docs/issues/open/ISSUE-2026-02-01-structure-aware-chunking.md)
- Improve JSON stats for corpus and memory usage.
  (issue: docs/issues/open/ISSUE-2026-02-01-json-stats-corpus-memory.md)
- Offer optional JSONL output for large result sets.
  (issue: docs/issues/open/ISSUE-2026-02-01-jsonl-output-large-results.md)

### Milestone 4 — AI-native Interface
- Support `--rql-stdin` / `--filter @file` and lexical parsing controls.
  (issue: docs/issues/open/ISSUE-2026-02-01-rql-stdin-filter-file.md)
- Shell completions, man page, and `cargo install` guidance.
  (issue: docs/issues/open/ISSUE-2026-02-01-shell-completions-manpage-install.md)
- Add optional Markdown metadata extraction for doc-level filtering.
  (issue: docs/issues/active/ISSUE-2026-02-02-markdown-metadata-extraction.md)

### Milestone 5 — Hybrid Retrieval Performance (Optional)
- Replace LSH shortlist with HNSW and add ANN migration/fallback.
  (issue: docs/issues/open/ISSUE-2026-02-01-ann-hnsw-backend.md)
