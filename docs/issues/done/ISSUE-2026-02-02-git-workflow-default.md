# ISSUE-2026-02-02-git-workflow-default

Status: done
Milestone: Milestone 0 — Workflow Hygiene
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
Agents need an explicit, enforced default to follow the full git workflow
captured in WORKFLOWS.md.

Scope:
- Add a MUST rule in DEVELOPMENT_RULES.md to follow WORKFLOWS.md
  "Complete Workflow (Merged Summary)" unless the user opts out.
- Keep the change documentation-only; no product behavior changes.

Acceptance Criteria:
- DEVELOPMENT_RULES.md contains the default workflow requirement.
- ROADMAP.md links to this issue.

Out of Scope:
- Changes to the workflow steps themselves.
- Tooling automation for git workflows.

Notes:
- This is a process enforcement update only.

Links:
- docs/progress/2026/2026-02-02.md
