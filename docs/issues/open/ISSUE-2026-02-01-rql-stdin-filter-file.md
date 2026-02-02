# ISSUE-2026-02-01-rql-stdin-filter-file

Status: open
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
