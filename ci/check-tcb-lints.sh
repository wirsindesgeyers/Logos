#!/usr/bin/env bash
# Falha se algum crate de TCB perdeu as proibições da §4 do roadmap.
# A lista vem de tcb/manifest.toml (crates com `strict = true`).
set -euo pipefail
cd "$(dirname "$0")/.."
status=0
python3 tcb/check_lints.py || status=1
while read -r crate; do
  [ -z "$crate" ] && continue
  if grep -Rnw 'unsafe' "crates/$crate" --include='*.rs' | grep -v 'forbid(unsafe_code)' | grep -vE '^\S+:\s*[0-9]+:\s*//' ; then
    echo "ERRO: uso de 'unsafe' em crates/$crate" >&2
    status=1
  fi
done < <(python3 tcb/tcb.py strict-crates)
exit $status
