#!/usr/bin/env bash
# Etapa4.6: solver real,300 obrigações válidas e300 contramodelos; sem claims.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 tools/build_cadical.py
python3 tools/build_cake_lpr.py
cargo build -p logos-audit --example prover_bridge
cd py
uv run python ../tools/gen_sat_vectors.py --check
uv run python -m logos_provers.sat_smoke
