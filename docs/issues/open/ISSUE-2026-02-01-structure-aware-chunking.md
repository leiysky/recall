# ISSUE-2026-02-01-structure-aware-chunking

Status: open
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
