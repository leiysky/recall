# ISSUE-2026-02-02-readme-doc-fixes

Status: done
Milestone: M8
Owner: Nexus
Created: 2026-02-02
Updated: 2026-02-02

Context:
- User feedback reports README inaccuracies around --json support, CLI synopsis, snapshot token usage, and single-file store wording.
- README references to docs/scripts may be missing in distributed packages; clarify source expectations.

Scope:
- Correct README JSON support language and CLI synopsis for add/rm.
- Make snapshot token usage reproducible with explicit capture + --snapshot example.
- Clarify single-file store vs config/lock files.
- Triage README references to docs/scripts and add guidance if they are absent in packaged distributions.

Acceptance Criteria:
- README no longer claims all commands support --json; coverage is accurate and explicit.
- CLI synopsis lists --json where supported for add/rm.
- Snapshot example shows how to capture stats.snapshot and re-use with --snapshot.
- Single-file store wording clarified to distinguish data vs config/lock.
- README references to docs/scripts include a note about source checkout vs packaged binaries.

Out of Scope:
- No CLI behavior changes or new flags.
- No packaging changes for release artifacts.

Notes:
- Source: user feedback on README inaccuracies (2026-02-02).
- Completed README corrections and roadmap link (2026-02-02).

Links:
- docs/progress/2026/2026-02-02.md
