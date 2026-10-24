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
  if [ "$t" = fact_decode ]; then
    # Fatos válidos e inválidos dos vetores independentes (6.1) como sementes.
    python3 - ../spec/vectors/fact/fact.json corpus/fact_decode <<'PY'
import hashlib
import json
import sys
from pathlib import Path

root = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
corpus = Path(sys.argv[2])
corpus.mkdir(parents=True, exist_ok=True)
cases = root["valid"] + root["invalid"] + root["bad_signature"]
for case in cases:
    wire = bytes.fromhex(case["fact_canon_hex"])
    (corpus / hashlib.sha256(wire).hexdigest()).write_bytes(wire)
print(f"fact_decode: {len(cases)} sementes no corpus")
PY
  fi
  if [ "$t" = trust_decode ]; then
    # Configurações válidas e inválidas dos vetores independentes (6.2) como sementes.
    python3 - ../spec/vectors/trust/trust.json corpus/trust_decode <<'PY'
import hashlib
import json
import sys
from pathlib import Path

root = json.loads(Path(sys.argv[1]).read_text(encoding="utf-8"))
corpus = Path(sys.argv[2])
corpus.mkdir(parents=True, exist_ok=True)
cases = root["valid"] + root["invalid"]
for case in cases:
    wire = bytes.fromhex(case["trust_canon_hex"])
    (corpus / hashlib.sha256(wire).hexdigest()).write_bytes(wire)
print(f"trust_decode: {len(cases)} sementes no corpus")
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
  if [ "$t" = render_template ] || [ "$t" = vocabulary_decode ]; then
    python3 - "$t" <<'SEED'
import hashlib
import sys
from pathlib import Path
corpus = Path("corpus") / sys.argv[1]
corpus.mkdir(parents=True, exist_ok=True)
for line in Path("../spec/vectors/vocabulary-v2/vectors.tsv").read_text().splitlines():
    if not line.startswith("#"):
        wire = bytes.fromhex(line.split("\t")[1])
        (corpus / hashlib.sha256(wire).hexdigest()).write_bytes(wire)
if sys.argv[1] == "render_template":
    for arity, text in [(0,"chove"),(1,"{0} administra"),(2,"{0} é dono de {1}"),(3,"{0} liga {1} com {2}")]:
        wire = bytes([arity]) + text.encode()
        (corpus / hashlib.sha256(wire).hexdigest()).write_bytes(wire)
SEED
  fi
  if [ "$t" = paraphrase_parse ]; then
    # Textos válidos dos vetores, prefixados pelo índice do vocabulário que os lê.
    python3 - <<'SEED'
import hashlib
from pathlib import Path
corpus = Path("corpus/paraphrase_parse")
corpus.mkdir(parents=True, exist_ok=True)
vocabularies = []
for line in Path("../spec/vectors/paraphrase/vectors.tsv").read_text().splitlines():
    if line.startswith("#"):
        continue
    _, vocabulary, formula, text, _ = line.split("\t")
    if vocabulary not in vocabularies:
        vocabularies.append(vocabulary)
    selector = bytes([vocabularies.index(vocabulary)])
    for wire in (selector + text.encode(), selector + bytes.fromhex(formula) if formula != "-" else b""):
        if wire:
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
