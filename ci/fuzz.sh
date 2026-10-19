#!/usr/bin/env bash
# Roda os alvos de fuzz (fuzz/fuzz_targets). Usado pela CI (.github/workflows/fuzz.yml) e local.
#
#   ci/fuzz.sh SEGUNDOS [alvo...]      # sem alvos: todos os de `cargo fuzz list`
#
# O cargo-fuzz exige nightly, mas o rust-toolchain.toml da raiz fixa `stable` (e vale dentro de
# fuzz/). Por isso a toolchain é escolhida aqui, explicitamente: FUZZ_TOOLCHAIN (padrão: nightly).
# Localmente o `nightly` da máquina pode ser mais velho que o rust-version do projeto; use um
# nightly datado, ex.: FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 canon_decode
#
# O SEGUNDOS é dividido em lotes de FUZZ_BATCH_SECONDS (padrão1800) com processo novo por lote:
# o runtime do fuzz (ASAN) acumula RSS por execução mesmo com alvo trivial e não é possível
# garantir o teto padrão de2048MiB numa execução única longa (medido no fechamento da5.2; ver
# docs/plans/etapa-5.2.md §7 e ADR0036). O corpus persiste entre lotes, então a cobertura
# acumula; o teto de RSS padrão do libFuzzer segue armado em cada lote.
set -euo pipefail
cd "$(dirname "$0")/../fuzz"

export RUSTUP_TOOLCHAIN="${FUZZ_TOOLCHAIN:-nightly}"
seconds="${1:-3600}"
shift || true
batch_seconds="${FUZZ_BATCH_SECONDS:-1800}"
[ "$batch_seconds" -ge 1 ] || { echo "erro: FUZZ_BATCH_SECONDS inválido: $batch_seconds" >&2; exit 1; }
if [ "$#" -gt 0 ]; then targets=("$@"); else mapfile -t targets < <(cargo fuzz list); fi
[ "${#targets[@]}" -gt 0 ] || { echo "erro: nenhum alvo de fuzz" >&2; exit 1; }

echo "toolchain: $(rustc --version) (RUSTUP_TOOLCHAIN=$RUSTUP_TOOLCHAIN); alvos: ${targets[*]}"
mkdir -p ../fuzz-logs
for t in "${targets[@]}"; do
  if [ "$t" = kernel_message ]; then
    python3 - <<'SEED'
import hashlib
import json
from pathlib import Path
corpus = Path("corpus/kernel_message")
corpus.mkdir(parents=True, exist_ok=True)
for vector in json.loads(Path("../spec/vectors/kernel-protocol/vectors.json").read_text())["vectors"]:
    for field in ("wire_hex", "frame_hex"):
        wire = bytes.fromhex(vector[field])
        (corpus / hashlib.sha256(wire).hexdigest()).write_bytes(wire)
SEED
  fi
  if [ "$t" = event_decode ]; then
    # O catálogo tem muitos campos/tag strings: começar com wire válido exercita
    # os decoders internos também no runner limpo da CI, sem biblioteca Python.
    python3 - ../spec/vectors/events/vectors.json corpus/event_decode <<'PY'
import hashlib
import json
import sys
from pathlib import Path

vectors = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))["vectors"]
corpus = Path(sys.argv[2])
corpus.mkdir(parents=True, exist_ok=True)
for vector in vectors:
    wire = bytes.fromhex(vector["event_wire_hex"])
    (corpus / hashlib.sha256(wire).hexdigest()).write_bytes(wire)
print(f"event_decode: {len(vectors)} vetores canônicos no corpus")
PY
  fi
  if [ "$t" = cnf_decode ] || [ "$t" = lrat_decode ]; then
    python3 - "$t" <<'SEED'
import hashlib
import json
import sys
from pathlib import Path

field = "cnf" if sys.argv[1] == "cnf_decode" else "proof"
corpus = Path("corpus") / sys.argv[1]
corpus.mkdir(parents=True, exist_ok=True)
for vector in json.loads(Path("../spec/vectors/lrat/vectors.json").read_text())["vectors"]:
    wire = vector[field].encode()
    (corpus / hashlib.sha256(wire).hexdigest()).write_bytes(wire)
SEED
  fi
  if [ "$t" = cake_output ]; then
    python3 - <<'SEED'
import hashlib
import json
from pathlib import Path
corpus = Path("corpus/cake_output")
corpus.mkdir(parents=True, exist_ok=True)
for v in json.loads(Path("../spec/vectors/cake-lpr/output.json").read_text())["vectors"]:
    wire = bytes([v["status"]]) + v["stdout"].encode() + b"\0" + v["stderr"].encode()
    (corpus / hashlib.sha256(wire).hexdigest()).write_bytes(wire)
SEED
  fi
  : > "../fuzz-logs/$t.log"
  remaining="$seconds"
  batch=1
  while [ "$remaining" -gt 0 ]; do
    chunk=$(( remaining < batch_seconds ? remaining : batch_seconds ))
    echo "== $t (${chunk}s de ${seconds}s; lote ${batch}) ==" | tee -a "../fuzz-logs/$t.log"
    cargo fuzz run "$t" -- -max_total_time="$chunk" 2>&1 | tee -a "../fuzz-logs/$t.log"
    remaining=$(( remaining - chunk ))
    batch=$(( batch + 1 ))
  done
  echo "== $t concluído: ${seconds}s em $((batch - 1)) lote(s) ==" | tee -a "../fuzz-logs/$t.log"
done
