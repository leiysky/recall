#!/usr/bin/env bash
# Build a release archive that includes the binary plus docs/scripts referenced in README.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

OUT_DIR="${1:-dist}"

VERSION="$(sed -n 's/^version = \"\\(.*\\)\"/\\1/p' Cargo.toml | head -n 1)"
if [[ -z "$VERSION" ]]; then
  echo "Unable to determine version from Cargo.toml" >&2
  exit 1
fi

OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
ARCH="$(uname -m)"
NAME="recall-${VERSION}-${OS}-${ARCH}"
STAGE="${OUT_DIR}/${NAME}"

cargo build --release --locked

BIN="${ROOT}/target/release/recall"
if [[ ! -x "$BIN" ]]; then
  echo "Release binary not found at ${BIN}" >&2
  exit 1
fi

rm -rf "$STAGE"
mkdir -p "$STAGE/bin"

cp "$BIN" "$STAGE/bin/"
cp README.md LICENSE DESIGN.md ROADMAP.md AGENTS.md "$STAGE/"
cp x "$STAGE/"
cp -R docs scripts "$STAGE/"

tar -czf "${OUT_DIR}/${NAME}.tar.gz" -C "$OUT_DIR" "$NAME"

echo "Release archive created: ${OUT_DIR}/${NAME}.tar.gz"
