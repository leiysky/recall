# Recall Benchmarks

Date: 2026-02-02
Status: Draft (Milestone 7)

## Dataset Spec
- 10,000 documents, ~1,000,000 tokens total.
- Plain text (`.txt`) files, sharded into subdirectories for filesystem realism.
- Deterministic generation from a fixed seed.

## Dataset Generator
Generate the benchmark dataset with:
```
python3 scripts/bench_gen.py --out /tmp/recall-bench --docs 10000 --tokens 1000000 --seed 42
```

The generator shards files into `shard-XXXX/` directories and adds a periodic
"needle" token to enable stable search queries.

## Benchmark Runs
Preferred: release build (`target/release/recall`). Debug builds are acceptable
for smoke baselines, but release numbers should be used for gating.

Run the benchmark harness:
```
python3 scripts/bench_run.py --recall-bin target/debug/recall --dataset /tmp/recall-bench --docs 10000 --runs 20
```

The script reports p50/p95 latencies (ms) for search/query/context and ingest
throughput (docs/min). Capture results in a dated baseline file in this folder.
Current baseline: docs/benchmarks/baseline-2026-02-02.md.

## Metrics
- Search/query/context latency: report p50 and p95 over 20 runs.
- Ingest throughput: docs/min for the full dataset ingest.

## Regression Thresholds
Hard gates (from PRD NFRs):
- Search p95 <= 250ms
- Query p95 <= 300ms
- Context p95 <= 400ms
- Ingest throughput >= 1,000 docs/min

Relative regression gates:
- p95 latency regression > 15% versus the most recent baseline fails review.
- Ingest throughput regression > 10% versus baseline fails review.

## Determinism Checks
Run the determinism tests to ensure repeated runs are identical:
```
cargo test deterministic_outputs
```

These tests validate repeated search/query/context output equality across 20
runs with a fixed snapshot token.

## Migration Tests
Supported prior schema version: unversioned stores (pre-`schema_version`).
Run:
```
cargo test migrates_unversioned_store
```

This confirms migrations to schema version 1 for all supported prior stores.
