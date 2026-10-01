#!/usr/bin/env bash
# Roda os alvos de fuzz (fuzz/fuzz_targets). Usado pela CI (.github/workflows/fuzz.yml) e local.
#
#   ci/fuzz.sh SEGUNDOS [alvo...]      # sem alvos: todos os de `cargo fuzz list`
#
# O cargo-fuzz exige nightly, mas o rust-toolchain.toml da raiz fixa `stable` (e vale dentro de
# fuzz/). Por isso a toolchain é escolhida aqui, explicitamente: FUZZ_TOOLCHAIN (padrão: nightly).
# Localmente o `nightly` da máquina pode ser mais velho que o rust-version do projeto; use um
# nightly datado, ex.: FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 canon_decode
set -euo pipefail
cd "$(dirname "$0")/../fuzz"

export RUSTUP_TOOLCHAIN="${FUZZ_TOOLCHAIN:-nightly}"
seconds="${1:-3600}"
shift || true
if [ "$#" -gt 0 ]; then targets=("$@"); else mapfile -t targets < <(cargo fuzz list); fi
[ "${#targets[@]}" -gt 0 ] || { echo "erro: nenhum alvo de fuzz" >&2; exit 1; }

echo "toolchain: $(rustc --version) (RUSTUP_TOOLCHAIN=$RUSTUP_TOOLCHAIN); alvos: ${targets[*]}"
mkdir -p ../fuzz-logs
for t in "${targets[@]}"; do
  echo "== $t (${seconds}s) =="
  cargo fuzz run "$t" -- -max_total_time="$seconds" 2>&1 | tee "../fuzz-logs/$t.log"
done
