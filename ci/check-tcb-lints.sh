#!/usr/bin/env bash
# Falha se algum crate de TCB perdeu as proibições da §4 do roadmap.
# A lista vive em ci/tcb-crates.txt (substituída por tcb/manifest.toml na Etapa 0.2).
set -euo pipefail
cd "$(dirname "$0")/.."
status=0
while read -r crate; do
  [ -z "$crate" ] && continue
  lib="crates/$crate/src/lib.rs"
  if ! grep -Fq '#![forbid(unsafe_code)]' "$lib"; then
    echo "ERRO: $lib não contém #![forbid(unsafe_code)]" >&2
    status=1
  fi
  if ! grep -Eq '#!\[deny\(.*clippy::unwrap_used.*clippy::expect_used.*clippy::panic.*\)\]' "$lib"; then
    echo "ERRO: $lib não nega clippy::unwrap_used, expect_used e panic" >&2
    status=1
  fi
  if grep -Rnw 'unsafe' "crates/$crate" --include='*.rs' | grep -v 'forbid(unsafe_code)' | grep -vE '^\S+:\s*[0-9]+:\s*//' ; then
    echo "ERRO: uso de 'unsafe' em crates/$crate" >&2
    status=1
  fi
done < ci/tcb-crates.txt
exit $status
