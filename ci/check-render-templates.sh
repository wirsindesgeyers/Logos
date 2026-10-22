#!/usr/bin/env bash
# Vocabulary/v2 por MCP: produtores/checkers e auditor independente reais.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 tools/gen_render_template_vectors.py --check
python3 tools/build_cadical.py
python3 tools/build_cake_lpr.py
cargo build -p logos-audit --example prover_bridge
cargo build -p logos-kernel --example client_fixture
cargo build --release -p logos-kernel --bin logos-kernel -p logos-audit --bin logos-audit
cd py
uv run python -m logos_provers.render_templates_smoke
