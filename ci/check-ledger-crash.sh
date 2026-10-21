#!/usr/bin/env bash
# Mesmo worker/oráculo nos runners nativos e na verificação local.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo build --locked --release -p logos-ledger --example ledger_writer
binary="$(pwd)/target/release/examples/ledger_writer"
cd py
# Teto agregado das24 mortes/replays; não é critério de latência.
# Os timeouts individuais do worker/oráculo permanecem em10/60s.
timeout 600 uv run --locked python ../tools/check_ledger_crash.py "$binary"
