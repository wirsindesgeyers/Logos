#!/usr/bin/env bash
# Atalho para tcb/tcb.py (manifesto e contagem da TCB). Equivalente a `cargo xtask tcb`.
exec python3 "$(dirname "$0")/tcb.py" "$@"
