# ISSUE-2026-02-02-squash-main-history

Status: done
Milestone: Milestone 0 — Workflow Hygiene
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
Main accumulated too many small commits; squash via rebase before merge.

Scope:
- Rebase to squash workflow/determinism commits into fewer commits.
- Preserve unrelated existing commits.
- Update progress log and roadmap accordingly.

Acceptance Criteria:
- Main history contains squashed commits for recent work.
- Progress log references the new commit SHAs.
- Issue is moved to done and linked from ROADMAP.

Out of Scope:
- Changing commit content beyond squashing.

Notes:
- Keep local history consistent; no remote push required.

Links:
- docs/progress/2026/2026-02-02.md
