#!/usr/bin/env bash
# Mesma fixture assinada e API pública na CI e na verificação local.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo run --locked --release -p logos-ledger --example ledger_index_scale -- target/ledger-index-scale-v1 target/ledger-index-scale.json
