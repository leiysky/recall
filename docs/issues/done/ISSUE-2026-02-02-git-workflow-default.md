# ISSUE-2026-02-02-git-workflow-default

Status: done
Milestone: Milestone 0 — Workflow Hygiene
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
Agents need an explicit, enforced default to follow the full git workflow
captured in AGENTS.md (inlined WORKFLOWS.md).

Scope:
- Add a MUST rule in AGENTS.md (inlined DEVELOPMENT_RULES.md) to follow
  AGENTS.md (inlined WORKFLOWS.md) "Lean Workflow (Default)" unless the user opts out.
- Keep the change documentation-only; no product behavior changes.

Acceptance Criteria:
- AGENTS.md (inlined DEVELOPMENT_RULES.md) contains the default workflow requirement.
- ROADMAP.md links to this issue.

Out of Scope:
- Changes to the workflow steps themselves.
- Tooling automation for git workflows.

Notes:
- This is a process enforcement update only.

Links:
- docs/progress/2026/2026-02-02.md
