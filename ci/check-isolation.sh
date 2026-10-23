#!/usr/bin/env bash
# Etapa5.11 (ADR0059): pentest scriptado do processo do agente isolado (bwrap) e
# verificação das units systemd de referência.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 tools/check_isolation.py
