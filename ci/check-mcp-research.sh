#!/usr/bin/env bash
# Etapa5R.1: dez tarefas por MCP stdio e auditoria completa no kernel real.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 tools/build_cake_lpr.py
python3 tools/build_cadical.py
cargo build -p logos-kernel --example client_fixture
cargo build -p logos-audit --example prover_bridge
cargo build --release -p logos-kernel --bin logos-kernel -p logos-audit --bin logos-audit
cd py
uv run python -m logos_mcp.smoke
