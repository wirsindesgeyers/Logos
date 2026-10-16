#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/../py"
uv run python ../tools/check_cnf.py
