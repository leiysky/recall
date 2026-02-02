# ISSUE-2026-02-02-deterministic-ordering-tiebreaks

Status: done
Milestone: Milestone 1 — Determinism + Explainability
Owner:
Created: 2026-02-02
Updated: 2026-02-02

Context:
Ensure deterministic ordering and tie-breaks are applied consistently across
RQL ordering, hybrid search, and single-mode search results.

Scope:
- Apply deterministic tie-breaks when ordering by score or fields.
- Ensure lexical/semantic-only search paths use deterministic ordering.
- Update docs (AGENTS/README) to describe deterministic ordering.
- Add regression tests for ORDER BY tie-breaks.

Acceptance Criteria:
- All query/search paths resolve ties deterministically.
- ORDER BY preserves tie-breaks.
- Tests cover ORDER BY tie-break behavior.

Out of Scope:
- Ranking model changes or new scoring stages.

Notes:
- This aligns implementation with the principles-first design.

Links:
- docs/progress/2026/2026-02-02.md
