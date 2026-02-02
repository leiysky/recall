# ISSUE-2026-02-02-markdown-metadata-extraction

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
