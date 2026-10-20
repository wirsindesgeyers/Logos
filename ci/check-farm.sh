#!/usr/bin/env bash
# Etapa10.2: portfólio isolado e500 veredictos rechecados no kernel real.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 tools/build_cadical.py
cargo build -p logos-audit --example prover_bridge
cargo build -p logos-kernel --example client_fixture
cargo build --release -p logos-kernel --bin logos-kernel
cd py
uv run python -m logos_provers.farm_smoke
