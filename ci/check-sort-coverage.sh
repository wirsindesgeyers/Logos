#!/usr/bin/env bash
# Verificação estrita de 100% de cobertura de ramos (branch coverage) em src/sort_check.rs.
# Executa cargo-llvm-cov sob a toolchain configurada e exige ambos os resultados
# de todos os ramos instrumentados de src/sort_check.rs. Não exclui regiões.
# Cobertura LLVM é evidência empírica da instrumentação, não prova de correção
# nem garantia de que todo comportamento semântico foi enumerado.
set -euo pipefail
cd "$(dirname "$0")/.."

export RUSTUP_TOOLCHAIN="${COVERAGE_TOOLCHAIN:-nightly-2026-09-28}"
output_json="${1:-target/sort-check-coverage.json}"
mkdir -p "$(dirname "$output_json")"

echo "== Executando cargo llvm-cov --branch para logos-air (toolchain: $RUSTUP_TOOLCHAIN) =="
cargo llvm-cov --branch --json --output-path "$output_json" -p logos-air --tests

python3 - "$output_json" << 'EOF'
import os
import json
import sys

report_path = sys.argv[1]

try:
    with open(report_path, "r", encoding="utf-8") as f:
        data = json.load(f)
except Exception as e:
    print(f"ERRO: Falha ao ler relatório JSON de cobertura '{report_path}': {e}", file=sys.stderr)
    sys.exit(1)

target_filename = os.path.abspath("crates/logos-air/src/sort_check.rs")
matched_file = None

for entry in data.get("data", []):
    for file_entry in entry.get("files", []):
        filename = file_entry.get("filename", "")
        if os.path.abspath(filename) == target_filename:
            matched_file = file_entry
            break
    if matched_file is not None:
        break

if matched_file is None:
    print(f"ERRO: '{target_filename}' não foi encontrado no relatório.", file=sys.stderr)
    sys.exit(1)

summary = matched_file.get("summary", {})
branch_summary = summary.get("branches", {})
region_summary = summary.get("regions", {})
line_summary = summary.get("lines", {})

b_count = branch_summary.get("count", 0)
b_covered = branch_summary.get("covered", 0)
b_notcovered = branch_summary.get("notcovered", 0)
b_percent = branch_summary.get("percent", 0.0)

r_count = region_summary.get("count", 0)
r_covered = region_summary.get("covered", 0)
r_notcovered = region_summary.get("notcovered", 0)

l_count = line_summary.get("count", 0)
l_covered = line_summary.get("covered", 0)

print(f"Arquivo analisado: {matched_file.get('filename')}")
print(f"  Linhas:     {l_covered}/{l_count}")
print(f"  Regiões:    {r_covered}/{r_count} (descobertas: {r_notcovered})")
print(f"  Ramos:      {b_covered}/{b_count} (descobertos: {b_notcovered}, {b_percent:.2f}%)")

if b_count == 0:
    print("ERRO: Nenhum ramo instrumentado encontrado para src/sort_check.rs. Verifique a flag --branch.", file=sys.stderr)
    sys.exit(1)

# Inspecionar detalhes de ramos com contagem zero, se presentes no payload de branches
uncovered_branches = []
for b in matched_file.get("branches", []):
    if len(b) >= 6:
        line_start, col_start, line_end, col_end, true_cnt, false_cnt = b[:6]
        if true_cnt == 0 or false_cnt == 0:
            uncovered_branches.append((line_start, col_start, line_end, col_end, true_cnt, false_cnt))

if b_notcovered > 0 or b_covered < b_count:
    print(f"\nFALHA: Cobertura de ramos incompleta em src/sort_check.rs ({b_covered}/{b_count} cobertos).", file=sys.stderr)
    if uncovered_branches:
        print("Ramos com execuções zero (linha:coluna):", file=sys.stderr)
        for l_s, c_s, l_e, c_e, t_c, f_c in uncovered_branches:
            print(f"  - L{l_s}:{c_s}..L{l_e}:{c_e} -> true={t_c}, false={f_c}", file=sys.stderr)
    sys.exit(1)

print(f"\nSUCESSO: 100% dos ramos instrumentados de src/sort_check.rs foram cobertos ({b_covered}/{b_count}).")
EOF
