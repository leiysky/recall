# ISSUE-2026-02-02-release-packaging-docs

Status: done
Milestone: M8
Owner: Nexus
Created: 2026-02-02
Updated: 2026-02-02

Context:
- README references docs/scripts that are missing from binary-only distributions.
- Release artifacts should ship the referenced docs/scripts alongside the binary.

Scope:
- Add a release packaging script that bundles the binary with docs/scripts.
- Update release checklist to include packaging step and verification.
- Update README note about packaged artifacts accordingly.
- Ignore packaging output in git.

Acceptance Criteria:
- A release packaging script produces an archive containing `recall` plus docs/scripts.
- Release checklist mentions the packaging step and validation.
- README notes that release archives include docs/scripts; binary-only installs may not.
- Packaging output directory is ignored by git.

Out of Scope:
- Changes to CLI behavior or install mechanisms.
- Publishing or distributing artifacts.

Notes:
- Source: user request to include docs/scripts in release packaging (2026-02-02).
- Completed packaging script, release checklist update, and README note (2026-02-02).

Links:
- docs/progress/2026/2026-02-02.md
