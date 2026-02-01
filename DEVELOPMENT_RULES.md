# Recall Development Rules

Purpose: keep Recall consistent with the design goals and agent-first workflow.

## Rule Keywords
- **MUST** = required for all changes.
- **SHOULD** = strongly recommended; document exceptions.
- **MAY** = optional.

## Product and UX
- **MUST** treat CLI and RQL as the primary interfaces; features must be reachable via them.
- **MUST** keep defaults safe, deterministic, and explainable.
- **SHOULD** avoid introducing implicit behavior; prefer explicit flags/options.
- **MUST** keep `--json` output stable; breaking changes require a schema version bump.
- **MUST** return actionable errors; in `--json`, errors are machine-parseable.

## Compatibility and Versioning
- **MUST** preserve backward compatibility for CLI flags and RQL syntax.
- **MUST** document any breaking change with a migration note and rationale.
- **MUST** version on-disk formats; old versions must be readable or migrated.
- **SHOULD** include a schema/version field in all JSON outputs.

## Query Language (RQL)
- **MUST** keep RQL stable; avoid breaking syntax or semantics.
- **MUST** keep `FILTER` strict; fields must be qualified (`doc.*`, `chunk.*`).
- **SHOULD** keep `SELECT` field handling permissive in v0.1 (unknown fields are ignored).
- **MUST** document and version any move to strict `SELECT` validation.
- **MUST** keep `FILTER` exact-only; no semantic inference.
- **SHOULD** apply deterministic ordering for queries with `USING`; document that structured queries without `ORDER BY` follow SQLite row order.
- **MUST** treat `ORDER BY score` as meaningful only when `USING` is present.

## Storage Engine
- **MUST** store data in a single file (`recall.db`); any auxiliary files must be optional and documented.
- **MUST** enforce single-writer, multi-reader semantics with file locking.
- **MUST** make WAL/journal writes atomic and recoverable.
- **MUST** preserve logical content and ordering guarantees across compaction (VACUUM-like).
- **SHOULD** keep the file portable across machines (no absolute-path dependencies).

## Retrieval and Ranking
- **MUST** expose per-stage scores in `--explain`; document weighting via config.
- **MUST** make new ranking stages opt-in and explicitly enabled.
- **MUST** ensure identical inputs + store state produce identical outputs.

## Context Assembly
- **MUST** enforce a hard token budget; never exceed it.
- **MUST** deduplicate overlapping chunks deterministically.
- **MUST** retain provenance for each chunk (doc path, offsets, hash, mtime).

## Ingest and Updates
- **MUST** be incremental and idempotent.
- **MUST** update doc/chunk IDs deterministically when content changes.
- **MUST** tombstone deletions and remove them during compaction.

## Security and Privacy
- **MUST** be local-first; no network calls without explicit user configuration.
- **MUST** avoid persisting secrets; API keys via environment variables only.
- **SHOULD** keep telemetry off by default.

## Testing and Quality
- **MUST** add tests for any RQL grammar or semantic changes.
- **MUST** maintain golden/snapshot tests for JSON output and explain data.
- **MUST** add migration tests for any storage format change.
- **SHOULD** include determinism tests for ordering and context packing.
- **MUST** avoid flaky tests; determinism is a core requirement.

## Documentation Hygiene
- **MUST** update `DESIGN.md` and `AGENTS.md` when behavior changes.
- **SHOULD** update `ROADMAP.md` if milestones or scope change.
