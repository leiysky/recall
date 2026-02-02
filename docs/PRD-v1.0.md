# PRD: Recall v1.0

## Context
Recall is post‑MVP with the backlog flush complete. The next step is a v1.0
release focused on interface stability, deterministic behavior, and documented
upgrade guarantees, with a clear target window and release gates.

## User Flow
1. User upgrades an existing store to v1.0 using documented steps and verifies
   compatibility.
2. User ingests documents and validates counts/metadata via `--json` outputs.
3. User runs `search`, `query`, and `context` with consistent ordering and
   snapshot‑stable results.
4. User exports/imports data and references v1.0 release notes for auditability.

## Functional Requirements
- FR1: Interface freeze after Milestone 6; no breaking changes to CLI flags,
  RQL syntax, or JSON schema for v1.0.
- FR2: Upgrade path from any supported pre‑1.0 schema with no data loss;
  incompatible stores must fail with machine‑parseable errors and explicit
  remediation steps.
- FR3: Deterministic retrieval; identical store state + snapshot token yields
  identical ordering and JSON outputs across 20 repeated runs for `search`,
  `query`, and `context`.
- FR4: Compatibility matrix published for supported OS/toolchain and embedding
  provider expectations.
- FR5: Documentation complete for v1.0 (README, DESIGN, AGENTS, WORKFLOWS,
  and a v1.0 change summary).
- FR6: Release checklist completed and v1.0 tagged with release notes.

## Non-Functional Requirements
- NFR1: Performance baselines on a documented reference dataset (min 10k docs,
  1M tokens) and reference hardware: p95 latency <= 250ms (`search`),
  <= 300ms (`query`), <= 400ms (`context`); ingest throughput >= 1,000
  docs/min.
- NFR2: Reliability verified over 10 cycles of ingest -> search/query/context
  -> compact/doctor with zero corruption and preserved determinism.
- NFR3: Stability gate: zero open P0/P1 issues at release; all tests pass
  without flakes.
- NFR4: Local‑first guarantee: no network access without explicit
  configuration, documented in release notes.

## Tasks
- [x] T1: Break v1.0 plan into milestone issues (Acceptance Criteria: separate
  M6–M8 issue files with scope + acceptance criteria, linked from `ROADMAP.md`;
  see ISSUE-2026-02-02-v1-0-m6-interface-freeze-compatibility,
  ISSUE-2026-02-02-v1-0-m7-quality-performance-baselines,
  ISSUE-2026-02-02-v1-0-m8-release-readiness)
- [x] T2: Define benchmark dataset/generator and baseline metrics
  (Acceptance Criteria: benchmark spec in docs, baseline numbers recorded,
  regression thresholds defined; see docs/benchmarks/README.md)
- [x] T3: Draft interface freeze + compatibility matrix (Acceptance Criteria:
  published matrix and upgrade guidance documented; see docs/COMPATIBILITY.md)
- [ ] T4: Define release candidate gate (Acceptance Criteria: v1.0 gate and
  P0/P1 definitions documented in `docs/RELEASE.md`)
- [ ] T5: Prepare v1.0 change summary (Acceptance Criteria: v1.0 change log
  in `docs/history/changes/`)
- [ ] T6: Execute release checklist and tag v1.0 (Acceptance Criteria:
  checklist complete and tag recorded)
