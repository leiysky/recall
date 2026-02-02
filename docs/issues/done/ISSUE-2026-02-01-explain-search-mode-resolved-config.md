# ISSUE-2026-02-01-explain-search-mode-resolved-config

Status: done
Milestone: Milestone 1 — Determinism + Explainability
Owner:
Created: 2026-02-01
Updated: 2026-02-02

Context:
- Add --explain diagnostics for effective search mode and resolved config.
- Add cache hints and candidate counts in --explain.
- Include lexical sanitization details in --json when FTS5 fallback is used.
- Add detailed per-stage timing breakdowns in stats.
- Ensure context --explain produces output consistent with search/query.
Scope:
- Expand --explain payload to include resolved config, mode, candidate counts, and cache hints.
- Add per-stage timing fields to stats in JSON output.
- Surface lexical sanitization details in JSON (and warnings in text mode).
- Ensure context --explain returns explain data or errors explicitly.
Acceptance Criteria:
- --explain reports mode (lexical/semantic/both) and resolved config values.
- JSON includes per-stage timing breakdowns with a stable schema.
- Lexical sanitization details are surfaced when fallback occurs.
- context --explain yields documented explain fields or a clear error.
Out of Scope:
- New ranking stages or rerankers.
Notes:
- Merged ISSUE-2026-02-01-explain-cache-hints-candidates.
- Merged ISSUE-2026-02-01-json-lexical-sanitization-details.
- Merged ISSUE-2026-02-01-timing-breakdowns-per-stage.

Links:
- docs/history/decisions/ADR-2026-02-01-explain-search-mode-resolved-config.md
- docs/progress/2026/2026-02-02.md
