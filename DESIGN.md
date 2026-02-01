# Recall Design Doc

Date: 2026-02-01
Status: Draft

## Summary
Recall is a CLI-first, hybrid search database designed for AI agents that need to work with large context. It combines fast lexical search with vector similarity, provides deterministic context assembly, and supports local-first workflows. The MVP is SQLite-backed (single-file `recall.db`) with FTS5 for lexical search and a pluggable embedding interface. Optional daemonized services are a future extension.

The product positioning is "SQLite for document data": a local, self-contained store with a stable query language, semantic search, and exact filtering.

## Goals
- CLI-first UX that is fast, composable, and scriptable.
- Hybrid retrieval (BM25-like lexical + vector similarity + metadata filters).
- Deterministic, explainable context assembly for agent pipelines.
- Local-first storage with predictable performance and offline operation.
- Extensible adapters for embeddings, chunking, and ranking.
- Stable, AI-friendly query language with forward-compatible semantics.

## Agent Requirements
- Bounded retrieval with hard token budgets and deterministic packing.
- Exact filters (path/tag/date/source) to tightly scope results.
- Stable query language with strict filters and predictable defaults.
- Explainability (score breakdown and why a chunk was returned).
- Provenance for citations (doc path, offsets, hash, mtime).
- Incremental ingest with reliable updates and no stale results.
- Low-latency local operation; offline by default.
- Clear JSON output contract for tooling.

## Agent Workflow Fit
Recall is designed to map directly onto a typical agent workflow:

1. **Ingest** → `recall add` builds `doc`/`chunk` tables + indexes.
2. **Plan/Scope** → RQL `FILTER` or CLI `--filter` narrows the dataset.
3. **Retrieve** → `USING semantic()` and/or `lexical()` yields candidates.
4. **Assemble** → `recall context` packs a bounded, deterministic window.
5. **Act** → agent consumes results with provenance and citations.
6. **Verify** → `--explain` and `recall stats/doctor` validate trust.

Design choices that strengthen the fit:
- Single-file local store keeps workflows portable and low-ops.
- Stable ordering/tie-breaks provide reproducible agent runs.
- JSON output schema supports toolchain integration.

## Usage Scenarios
Recall is intended for local, deterministic retrieval where a CLI and single
file store are advantages:

- **Per-repo agent retrieval**: maintain a `recall.db` beside a codebase and
  use `recall search/query/context` as tool calls.
- **Curated knowledge bases**: tag sources, preserve provenance, export/import
  JSONL for review or sharing.
- **Incident response**: capture snapshot tokens in stats and export JSONL
  for audits and sharing.
- **Research corpora**: strict filters and deterministic packing for notes and
  papers.

Non-goals still apply: Recall is not a hosted multi-tenant search service or a
general-purpose analytics engine.

## Non-goals
- Full-text analytics engine with complex SQL.
- Managed multi-tenant cloud service (out of scope for v1).
- Real-time collaborative editing of documents.
- OLTP-grade concurrent writes (v1 is single-writer).

## User Experience and Interface

### Primary Interface: CLI
The CLI is the source of truth for all user workflows. Commands are intentionally stable, composable, and easy to pipe.

#### Command Surface (Spec)
- `recall init [path]`:
  - Creates a new local store at `path` (or `.` by default).
  - Writes `recall.toml` with defaults.
- `recall add <path...>`:
  - Ingests files, applies chunking, generates embeddings, builds indexes.
  - Flags: `--glob`, `--tag`, `--source`, `--mtime-only`, `--ignore`, `--parser` (reserved/no-op in MVP).
- `recall rm <doc_id|path...>`:
  - Removes documents and tombstones related chunks.
  - Disambiguation: targets containing a path separator or pointing to an existing path are treated as paths; otherwise treated as doc IDs.
  - Flags: `--purge` (force compaction of tombstones).
- `recall search <query>`:
  - Hybrid search with top-k results, optional filters.
  - Flags: `--k`, `--bm25`, `--vector`, `--filter`, `--explain`, `--json`.
  - Mode rules: `--bm25` = lexical only, `--vector` = semantic only, both or neither = hybrid.
  - Pagination: use RQL (`LIMIT`/`OFFSET`) for stable pagination; `search` is top-k only.
- `recall query --rql <string|@file>`:
  - Executes a Recall Query Language statement.
  - Flags: `--json`, `--explain`.
- `recall context <query>`:
  - Returns assembled context window for an agent.
  - Flags: `--budget-tokens`, `--diversity`, `--format` (reserved/no-op in MVP), `--json`.
- `recall stats`:
  - Shows index sizes, doc counts, embedding model, storage usage.
- `recall doctor`:
  - Validates store integrity and suggests repairs.
- `recall compact`:
  - Runs VACUUM, purges tombstones, and refreshes indexes.
- `recall export`:
  - Exports docs/chunks as JSONL (stdout or `--out` file).
- `recall import <file>`:
  - Imports a JSONL export into the current store.

#### CLI Examples
```
recall init .
recall add ./docs --glob "**/*.md" --tag docs
recall search "zero-downtime deploy" --k 8 --filter "doc.tag = 'docs'" --json
recall query --rql "SELECT chunk.text, score FROM chunk USING semantic('S3 auth') FILTER doc.tag = 'docs' LIMIT 5;"
recall context "how we handle retries" --budget-tokens 1200 --diversity 2
```

### Output Formats
- Default: human-readable tables for interactive use.
- `--json` for machine-readable integration.
- `--explain` returns scoring and ranking diagnostics.

### JSON Output Schema (Draft)
All JSON responses are stable and machine-friendly. Fields not relevant to a command are omitted.

```
{
  "ok": true,
  "schema_version": "1",
  "query": {
    "text": "string",
    "rql": "string|null",
    "filters": "string|null",
    "limit": 10,
    "offset": 0
  },
  "results": [
    {
      "score": 0.82,
      "doc": {
        "id": "string",
        "path": "string",
        "mtime": "RFC3339",
        "hash": "string",
        "tag": "string",
        "source": "string"
      },
      "chunk": {
        "id": "string",
        "doc_id": "string",
        "offset": 120,
        "tokens": 220,
        "text": "string"
      },
      "explain": {
        "lexical": 0.31,
        "semantic": 0.76
      }
    }
  ],
  "context": {
    "text": "string",
    "budget_tokens": 1200,
    "used_tokens": 1087,
    "chunks": [
      {
        "id": "string",
        "doc_id": "string",
        "offset": 120,
        "tokens": 220,
        "text": "string",
        "path": "string",
        "hash": "string",
        "mtime": "RFC3339"
      }
    ]
  },
  "stats": {
    "took_ms": 23,
    "total_hits": 148,
    "snapshot": "RFC3339"
  },
  "warnings": [ "string" ],
  "next_offset": 10,
  "error": {
    "code": "string",
    "message": "string",
    "details": "string|null",
    "hint": "string|null"
  }
}
```
Notes:
- For success responses: `ok=true` and `error` is omitted.
- For failures: `ok=false`, `error` is present, and `results/context` may be omitted.
- `stats.snapshot` is the max `doc.mtime` string at query time; it may be empty for empty stores.
- `query.limit` and `query.offset` reflect the effective values after defaults are applied
  (e.g., `--k` for search/context, `max_limit` or RQL `LIMIT` for queries).
- `explain` currently includes `lexical` and `semantic` only; additional stages are future work.

### Configuration
`recall.toml` controls:
- `store_path` for the single-file database.
- Chunking (`chunk_tokens`, `overlap_tokens`).
- Embedding (`embedding`, `embedding_dim`), default is deterministic `hash`.
- ANN LSH parameters (`ann_bits`, `ann_seed`).
- Hybrid weighting (`bm25_weight`, `vector_weight`).
- `max_limit` default limit for RQL when `LIMIT` is omitted.

### Optional Service Mode (Future)
A background `recalld` process can provide:
- Faster repeated searches (warm caches).
- Lightweight API over a local socket.
- File watcher for automatic reindexing.

The CLI remains fully functional without the daemon; service mode is not part of the MVP.

## Recall Query Language (RQL)

### Design Principles
- Stable, minimal SQL-like subset to be easy for AI and humans to generate.
- Deterministic semantics (no hidden re-ranking unless specified).
- Explicit separation between semantic search, lexical search, and exact filters.

### Core Concepts
- Tables: `doc`, `chunk`.
- Functions: `semantic(text)` and `lexical(text)` define search inputs.
- Filters: strict boolean expressions over doc/chunk fields.
- Output: explicit `SELECT` list.

### Minimal Grammar (Subset)
```
SELECT <fields> FROM <table>
USING semantic(<text>) [, lexical(<text>)]
FILTER <boolean-expr>
ORDER BY <field|score> [ASC|DESC]
LIMIT <n> [OFFSET <m>];
```

### Notes
- `USING` is optional; if absent, RQL acts like a structured filter query.
- `semantic()` and `lexical()` can be used together. The engine normalizes
  scores and combines them using configured weights.
- `FILTER` is exact matching; it never invokes semantic search.
- `FILTER` fields must be qualified (`doc.*`, `chunk.*`).
- `SELECT` fields may be unqualified; unknown `SELECT` fields are ignored in v0.1.
- If `ORDER BY` is omitted and `USING` exists, chunk results are scored
  (`score DESC`); doc results are grouped by `doc.id` unless `ORDER BY` is specified.
- CLI `--filter` uses the same Filter Expression Language and requires
  qualified fields (`doc.*`, `chunk.*`).
- `ORDER BY score` only applies when `USING` is present; otherwise it is ignored.
- For `SELECT ... FROM doc USING ...`, `score` is the max chunk score for that doc.

### Filter Expression Language (FEL)
`FILTER` and CLI `--filter` share the same expression language.

```
<boolean-expr> := <term> ( (AND|OR) <term> )*
<term> := [NOT] <predicate> | '(' <boolean-expr> ')'
<predicate> := <field> <op> <value>
             | <field> IN '(' <value-list> ')'
<op> := = | != | < | <= | > | >= | LIKE | GLOB
```

Semantics:
- Fields must be qualified as `doc.*` or `chunk.*`.
- `LIKE` uses SQL patterns: `%` (any string) and `_` (single char).
- `GLOB` uses glob patterns: `*`, `?`, and `**` for recursive paths.
- Comparisons are type-aware; ISO-8601 dates compare lexicographically.
- Strings are case-sensitive by default.

### Ordering and Tie-breaks
- If `ORDER BY` is provided, it is respected (ties are not further ordered in v0.1).
- With `USING` and `FROM chunk`, ordering is deterministic by default:
  - `score DESC`, then `doc.path ASC`, then `chunk.offset ASC`, then `chunk.id ASC`.
- With `USING` and `FROM doc`, results are grouped by `doc.id` unless `ORDER BY` is specified.
- Without `USING`, ordering follows SQLite row order unless `ORDER BY` is specified.
- `OFFSET` and `LIMIT` operate on the resulting ordering.

### Example Queries
```
SELECT doc.id, doc.path, score FROM doc
USING semantic("recovery runbook")
FILTER doc.tag = "ops" AND doc.mtime >= "2026-01-01"
LIMIT 10;

SELECT chunk.text, chunk.doc_id, score FROM chunk
USING semantic("rate limit") , lexical("429")
FILTER doc.path GLOB "**/api/**" AND chunk.tokens <= 256
ORDER BY score DESC
LIMIT 8;

SELECT doc.id, doc.path FROM doc
FILTER doc.tag IN ("policy", "security")
ORDER BY doc.mtime DESC
LIMIT 20;
```

### Field Catalog (Initial)
- `doc.id`, `doc.path`, `doc.mtime`, `doc.hash`, `doc.tag`, `doc.source`.
- `chunk.id`, `chunk.doc_id`, `chunk.offset`, `chunk.tokens`, `chunk.text`.
Note: `doc.size` is stored but not exposed in RQL v0.1.

### Strictness (v0.1)
RQL and filters apply strictness as follows:
- `FILTER` fields must be qualified (`doc.*`, `chunk.*`).
- Unknown `SELECT` fields are ignored (no error) in v0.1.
- `USING` is required to enable semantic or lexical search.

## Architecture Overview

### High-Level Components
1. **CLI**
   - Parses commands, loads config, orchestrates workflows.
2. **Ingest Pipeline**
   - File discovery, parsing, chunking, embedding, metadata extraction.
3. **Indexing Layer**
   - Lexical index (BM25-like) and vector index (ANN).
4. **Storage Engine**
   - SQLite-backed single-file row store for document data and indexes.
5. **Query Engine**
   - RQL parser, hybrid retrieval, filtering, ranking, explainability.
6. **Context Assembler**
   - Budgets tokens, deduplicates, returns final payload.

### Data Flow (Ingest)
1. Read files and metadata.
2. Parse into text (MVP: UTF-8 text files only; binary is skipped).
3. Chunk text with stable boundaries (MVP: whitespace token counts).
4. Generate embeddings for each chunk.
5. Update lexical and vector indexes.
6. Persist to single-file store with SQLite transaction semantics.

### Data Flow (Query)
1. Parse query or RQL.
2. Retrieve lexical hits and vector neighbors.
3. Normalize scores and merge candidate lists.
4. Combine and sort by weighted scores (v0.1; no learned ranker).
5. Assemble context according to token budget and policy.
6. Return results and optional explain data.

## Data Model

### Entities
- **Document**: file-level metadata (path, hash, mtime, tags, source).
- **Chunk**: text span with embedding and metadata references.
- **Index Entry**: mapping from term or vector to chunk IDs.

### Identifiers
- Document IDs derived from stable hash of normalized path + content hash.
- Chunk IDs derived from document ID + chunk offset.

### SQLite-like Schema (Logical)
```
CREATE TABLE doc (
  id TEXT PRIMARY KEY,
  path TEXT,
  mtime TEXT,
  size INTEGER,
  hash TEXT,
  tag TEXT,
  source TEXT,
  deleted INTEGER DEFAULT 0
);

CREATE TABLE chunk (
  rowid INTEGER PRIMARY KEY,
  id TEXT UNIQUE,
  doc_id TEXT,
  offset INTEGER,
  tokens INTEGER,
  text TEXT,
  embedding BLOB,
  deleted INTEGER DEFAULT 0
);

CREATE INDEX idx_doc_path ON doc(path);
CREATE INDEX idx_doc_tag ON doc(tag);
CREATE INDEX idx_chunk_doc ON chunk(doc_id);

CREATE VIRTUAL TABLE chunk_fts USING fts5(text, content='chunk', content_rowid='rowid');

CREATE TABLE ann_lsh (
  signature INTEGER,
  chunk_id TEXT,
  doc_id TEXT
);

CREATE INDEX idx_ann_sig ON ann_lsh(signature);
CREATE INDEX idx_ann_doc ON ann_lsh(doc_id);
```
- FTS5 is kept in sync via triggers on `chunk` inserts/updates/deletes.

## Storage Engine Details

### Layout (Local)
```
recall.db
```

### SQLite Layout (MVP)
- `recall.db` is a standard SQLite file.
- Tables: `doc`, `chunk`, plus FTS5 virtual table `chunk_fts`.
- Embeddings are stored as BLOBs on `chunk.embedding`.
- Lexical scoring uses SQLite FTS5 `bm25`.
- ANN is an LSH signature table (`ann_lsh`) used to shortlist candidates.

### Write Path
- Single-writer, multi-reader (SQLite connection semantics).
- SQLite journaling (MVP uses `journal_mode=DELETE`, `synchronous=NORMAL`).
- Commit is atomic via SQLite transaction boundaries.

### Compaction
- Tombstoned docs/chunks are dropped during compaction.
- Page defragmentation and index rebuilds happen in a VACUUM-like pass.
- Compaction runs on demand (`recall compact`); auto thresholds are a future optimization.

### Durability and Integrity
- `recall doctor` runs `PRAGMA integrity_check`.
- Compaction uses `VACUUM` and tombstone purging.

## Indexing and Retrieval

### Lexical Index
- SQLite FTS5 with `bm25` scoring (BM25-like).
- Tokenization follows SQLite FTS5 defaults (configurable later).
- If an FTS5 query fails to parse, Recall retries with a sanitized literal
  query (non-word characters replaced by spaces) and emits a warning in
  `--explain` output.

### Vector Index
- MVP: embeddings stored as BLOBs in `chunk` and searched by LSH shortlist + linear rerank.
- Future: swap in HNSW or other ANN backends.

### Hybrid Ranking
- Weighted combination of lexical and vector scores.
- `--explain` outputs per-stage scoring.
- Optional ranker stages are future work.

## Context Assembly
### Packing Policy
- Candidate chunks are ranked by the query ordering rules.
- Hard `budget_tokens` is enforced; the packer never exceeds it.
- MVP: de-duplication is by chunk ID; overlap merging is planned.
- Token counts are based on whitespace tokenization (MVP).
- Optional diversity cap limits chunks per document (e.g., `--diversity 2`).
- Truncation is deterministic: if a chunk exceeds remaining budget, it is
  prefix-truncated to fit.
### Guarantees
- Stable ordering and deterministic packing for identical inputs.
- Provenance retained for each packed chunk (doc path, offsets, hash, mtime).

## Extensibility
- **Parser adapters**: add file types (md, pdf, code).
- **Embedding adapters**: local or remote model providers.
- **Ranker plugins**: custom scoring or reranking stages.
- **Storage backends**: future support for remote object stores.

## Reliability and Operations
- Single-file storage with periodic compaction (VACUUM-like).
- `recall doctor` uses SQLite integrity checks to validate file health.

## Security and Privacy
- Local-first by default, no network required.
- Embedding providers are explicit and configurable.
- Secrets (API keys) stored in environment variables, not config files.

## Testing Strategy
- Unit tests for chunking, scoring, and storage invariants.
- Integration tests for ingest + search + context assembly.
- Golden tests for explainability output stability.
- Parser tests for RQL grammar and filter strictness.

## MVP Scope (v0.1)
- `recall init/add/search/query/context` with `--json`.
- RQL parser with `SELECT/FROM/USING/FILTER/LIMIT`.
- Lexical index via SQLite FTS5 and LSH-assisted semantic search.
- Single-writer, single-file SQLite store.
- Deterministic context assembly.
- JSONL export/import.

## Milestones
1. CLI scaffolding with init/add/search/query.
2. RQL parser + field catalog + strict filters.
3. Lexical index and storage engine.
4. Vector embeddings and ANN search.
5. Hybrid ranking and context assembly.
6. Compaction + doctor + integrity checks.
7. Daemon mode and local socket API.
