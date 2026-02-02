# CHANGE-2026-02-02-backlog-flush

Milestone: Milestone 1–5 (backlog flush)
Summary:
- Added schema versioning + migrations and doc metadata storage.
- Added snapshot support, explain diagnostics, JSONL streaming, and corpus/memory stats.
- Added structure-aware chunking, HNSW backend option, and safer doctor/compact flows.
- Added CLI conveniences: `--rql-stdin`, `--filter @file`, completions, and man page output.

User impact:
- New CLI flags (`--snapshot`, `--lexical-mode`, `--jsonl`, `--rql-stdin`, `--extract-meta`, `--parser`).
- New `recall completions` and `recall man` commands.
- `recall doctor --fix` and safer `recall compact` pre-checks.
- Metadata filtering via `doc.meta.<key>` and `--extract-meta`.

Migration:
- Stores are auto-migrated on open to schema v1; HNSW index is rebuilt during migration.

References:
- ISSUE-2026-02-01-schema-versioning-migrations-ann-fts
- ISSUE-2026-02-02-markdown-metadata-extraction
- ISSUE-2026-02-01-explain-search-mode-resolved-config
- ISSUE-2026-02-01-snapshot-flag-search-query-context
- ISSUE-2026-02-01-structure-aware-chunking
- ISSUE-2026-02-01-rql-stdin-filter-file
- ISSUE-2026-02-01-ann-hnsw-backend
- ISSUE-2026-02-01-doctor-fix-safer-compact
- ISSUE-2026-02-02-cli-store-mode-safety
- ISSUE-2026-02-02-cli-flag-hygiene-context-explain
- ISSUE-2026-02-01-jsonl-output-large-results
- ISSUE-2026-02-01-json-stats-corpus-memory
- ISSUE-2026-02-01-release-checklist-versioning-policy
- ISSUE-2026-02-01-shell-completions-manpage-install
