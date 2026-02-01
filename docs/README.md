# Recall Tracking Docs

Purpose: track issue, progress, and history context for Recall using simple
Markdown files and path-based status.

## Layout
- docs/issues/
  - open/   # backlog items
  - active/ # in progress
  - done/   # completed
- docs/progress/
  - YYYY/   # daily or session notes
- docs/history/
  - decisions/ # ADRs
  - changes/   # milestone or release summaries
- docs/_templates/ # copy these when creating new docs

## Workflow
1) Create a new issue file in docs/issues/open/ using the issue template.
2) When work starts, move the file to docs/issues/active/.
3) Log progress in docs/progress/YYYY/YYYY-MM-DD.md and link the issue.
4) Record design decisions in docs/history/decisions/ and link back.
5) When done, move the issue file to docs/issues/done/ and add a change summary
   if user-visible behavior changed.

## Templates
Copy from docs/_templates/ and adjust dates, slugs, and references.
