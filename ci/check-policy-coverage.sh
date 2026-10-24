#!/usr/bin/env bash
# Etapa 7.1: 100% dos ramos LLVM da validação de políticas (logos-policy/src), sem exclusões.
# Mesmo método de ci/check-sort-coverage.sh: cargo-llvm-cov sob nightly datado, sem exclusões.
# Cobertura instrumentada é evidência empírica, não prova de correção.
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTUP_TOOLCHAIN="${COVERAGE_TOOLCHAIN:-nightly-2026-09-28}"
output_json="${1:-target/policy-coverage.json}"
mkdir -p "$(dirname "$output_json")"

echo "== cargo llvm-cov --branch para logos-policy (toolchain: $RUSTUP_TOOLCHAIN) =="
cargo llvm-cov --branch --json --output-path "$output_json" -p logos-policy --tests

python3 - "$output_json" << 'PY'
import json
import os
import sys

with open(sys.argv[1], encoding="utf-8") as handle:
    data = json.load(handle)

root = os.path.abspath("crates/logos-policy/src")
files = [
    f
    for entry in data.get("data", [])
    for f in entry.get("files", [])
    if os.path.abspath(f.get("filename", "")).startswith(root + os.sep)
]
if not files:
    sys.exit("ERRO: nenhum arquivo de crates/logos-policy/src no relatório")
failed = False
for f in sorted(files, key=lambda f: f["filename"]):
    b = f["summary"]["branches"]
    r = f["summary"]["regions"]
    print(
        f"{os.path.relpath(f['filename'])}: ramos {b['covered']}/{b['count']}, "
        f"regiões {r['covered']}/{r['count']}"
    )
    # Critério: 100% dos ramos. Regiões são informativas: restam apenas os `?` de operações que
    # não falham (hash/codificação de objetos já validados), sem caminho de erro alcançável.
    if b["covered"] != b["count"]:
        failed = True
        for br in f.get("branches", []):
            if len(br) >= 6 and (br[4] == 0 or br[5] == 0):
                print(f"  ramo L{br[0]}:{br[1]} true={br[4]} false={br[5]}", file=sys.stderr)
if failed:
    sys.exit("FALHA: cobertura de ramos incompleta em logos-policy/src")
print("SUCESSO: 100% dos ramos de logos-policy/src")
PY
