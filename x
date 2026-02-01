#!/usr/bin/env bash
# Copyright 2026 Recall Authors
#
# Licensed under the Apache License, Version 2.0 (the "License");
# you may not use this file except in compliance with the License.
# You may obtain a copy of the License at
#
#     http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS,
# WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
# See the License for the specific language governing permissions and
# limitations under the License.

set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$ROOT"

cmd="${1:-help}"
shift || true

usage() {
  cat <<'USAGE'
Usage: ./x <command> [args...]

Commands:
  build      cargo build
  check      cargo check
  test       cargo test
  bench      cargo bench
  run        cargo run [-- <args>]
  fmt        cargo fmt
  clippy     cargo clippy
  clean      cargo clean
  help       show this help

Examples:
  ./x build --release
  ./x test
  ./x run -- --json
USAGE
}

case "$cmd" in
  build)  cargo build "$@" ;;
  check)  cargo check "$@" ;;
  test)   cargo test "$@" ;;
  bench)  cargo bench "$@" ;;
  run)    cargo run "$@" ;;
  fmt)    cargo fmt "$@" ;;
  clippy) cargo clippy --workspace --all-targets --all-features "$@" ;;
  clean)  cargo clean ;;
  help|-h|--help) usage ;;
  *)
    echo "Unknown command: $cmd" >&2
    usage >&2
    exit 1
    ;;
 esac
