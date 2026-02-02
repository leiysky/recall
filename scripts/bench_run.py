#!/usr/bin/env python3
"""Run Recall benchmarks against a generated dataset."""

from __future__ import annotations

import argparse
import json
from pathlib import Path
import statistics
import subprocess
import tempfile
import time


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--recall-bin", default="target/debug/recall", help="Recall binary")
    parser.add_argument("--dataset", required=True, help="Dataset directory")
    parser.add_argument("--docs", type=int, default=10000, help="Doc count for throughput")
    parser.add_argument("--runs", type=int, default=20, help="Runs per command")
    parser.add_argument("--k", type=int, default=8, help="Search k")
    parser.add_argument(
        "--snapshot",
        default="2100-01-01T00:00:00Z",
        help="Snapshot token (RFC3339)",
    )
    return parser.parse_args()


def run_cmd(cmd: list[str], cwd: Path) -> float:
    start = time.perf_counter()
    subprocess.run(cmd, cwd=cwd, check=True, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
    end = time.perf_counter()
    return (end - start) * 1000.0


def percentile(values: list[float], pct: float) -> float:
    if not values:
        return 0.0
    values = sorted(values)
    idx = int(round((pct / 100.0) * (len(values) - 1)))
    return values[idx]


def summarize(values: list[float]) -> dict[str, float]:
    return {
        "p50_ms": percentile(values, 50),
        "p95_ms": percentile(values, 95),
        "mean_ms": statistics.mean(values),
        "runs": len(values),
    }


def main() -> int:
    args = parse_args()
    recall_bin = Path(args.recall_bin)
    dataset = Path(args.dataset)

    if not recall_bin.exists():
        raise SystemExit(f"Recall binary not found: {recall_bin}")
    if not dataset.exists():
        raise SystemExit(f"Dataset not found: {dataset}")

    with tempfile.TemporaryDirectory() as tmp:
        root = Path(tmp)
        run_cmd([str(recall_bin), "init", "."], root)

        ingest_ms = run_cmd(
            [
                str(recall_bin),
                "add",
                str(dataset),
                "--glob",
                "**/*.txt",
                "--tag",
                "bench",
                "--json",
            ],
            root,
        )
        ingest_docs_per_min = 0.0
        if ingest_ms > 0:
            ingest_docs_per_min = args.docs / (ingest_ms / 1000.0) * 60.0

        search_times = []
        query_times = []
        context_times = []

        search_cmd = [
            str(recall_bin),
            "search",
            "needle",
            "--k",
            str(args.k),
            "--snapshot",
            args.snapshot,
            "--json",
        ]
        query_cmd = [
            str(recall_bin),
            "query",
            "--rql",
            "FROM chunk USING semantic('needle') LIMIT 8 SELECT chunk.text, doc.path;",
            "--snapshot",
            args.snapshot,
            "--json",
        ]
        context_cmd = [
            str(recall_bin),
            "context",
            "needle",
            "--budget-tokens",
            "512",
            "--snapshot",
            args.snapshot,
            "--json",
        ]

        for _ in range(args.runs):
            search_times.append(run_cmd(search_cmd, root))
        for _ in range(args.runs):
            query_times.append(run_cmd(query_cmd, root))
        for _ in range(args.runs):
            context_times.append(run_cmd(context_cmd, root))

    report = {
        "ingest": {
            "time_ms": ingest_ms,
            "docs_per_min": ingest_docs_per_min,
        },
        "search": summarize(search_times),
        "query": summarize(query_times),
        "context": summarize(context_times),
    }

    print(json.dumps(report, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
