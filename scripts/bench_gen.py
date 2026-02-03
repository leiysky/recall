#!/usr/bin/env python3
"""Generate a deterministic benchmark dataset for Recall."""

from __future__ import annotations

import argparse
from pathlib import Path
import random
import sys

VOCAB = [
    "alpha", "beta", "gamma", "delta", "epsilon", "zeta", "eta", "theta",
    "lambda", "omega", "sigma", "kappa", "tau", "pi", "rho", "mu",
    "vector", "matrix", "query", "search", "context", "chunk", "doc",
    "index", "token", "filter", "schema", "snapshot", "migrate", "store",
    "deterministic", "semantic", "lexical", "hybrid", "budget", "provenance",
    "ingest", "export", "import", "fts5", "sqlite",
    "memory", "latency", "throughput", "benchmark", "dataset", "baseline",
    "window", "offset", "order", "score", "ranking", "weight", "config",
]


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--out", required=True, help="Output directory")
    parser.add_argument("--docs", type=int, default=10000, help="Number of docs")
    parser.add_argument(
        "--tokens",
        type=int,
        default=1000000,
        help="Approximate total tokens across all docs",
    )
    parser.add_argument("--seed", type=int, default=42, help="PRNG seed")
    parser.add_argument(
        "--shard-size",
        type=int,
        default=1000,
        help="Docs per shard directory",
    )
    parser.add_argument(
        "--topics",
        type=int,
        default=50,
        help="Number of rotating topic labels",
    )
    return parser.parse_args()


def write_doc(path: Path, text: str) -> None:
    path.write_text(text + "\n", encoding="utf-8")


def main() -> int:
    args = parse_args()
    if args.docs <= 0:
        raise SystemExit("--docs must be > 0")
    if args.tokens <= 0:
        raise SystemExit("--tokens must be > 0")

    out_dir = Path(args.out)
    out_dir.mkdir(parents=True, exist_ok=True)

    tokens_per_doc = max(1, args.tokens // args.docs)
    for doc_id in range(args.docs):
        shard = out_dir / f"shard-{doc_id // args.shard_size:04d}"
        shard.mkdir(parents=True, exist_ok=True)

        rng = random.Random(args.seed + doc_id)
        topic = f"topic-{doc_id % args.topics:02d}"
        keywords = ["recall", "benchmark", topic, f"doc-{doc_id:05d}"]
        if doc_id % 100 == 0:
            keywords.append("needle")

        body_len = max(1, tokens_per_doc - len(keywords))
        body = [rng.choice(VOCAB) for _ in range(body_len)]
        text = " ".join(keywords + body)

        filename = shard / f"doc-{doc_id:05d}.txt"
        write_doc(filename, text)

    print(
        f"Generated {args.docs} docs with ~{tokens_per_doc} tokens per doc in {out_dir}",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
