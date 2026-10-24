#!/usr/bin/env bash
# Verificação local em níveis (AGENTS.md §4, ADR0057). Falha no primeiro erro; sem pipes que
# mascarem o código de saída.
#
#   ci/verify.sh                      completo (padrão): tudo, inclusive testes de escala;
#                                     exigido antes de push e para marcar critérios.
#   ci/verify.sh --tier fast          rápido: fmt, clippy, testes com escala reduzida, vetores,
#                                     lints e Python sem `heavy`. Sem rede. A cada commit.
#   ci/verify.sh --tier affected [--base REV]
#                                     rápido + smokes/scripts das áreas tocadas desde REV
#                                     (padrão: origin/main), escolhidos por tools/verify_plan.py.
#   --selftest                        acrescenta o autoteste da CI (~2 min, usa rede).
#
# Cada etapa grava sua duração em target/verify-times.tsv e imprime STEP_OK; uma falha imprime
# VERIFY_FAILED com o nome da etapa. Termina com VERIFY_OK tier=<nível>.
set -Eeuo pipefail
cd "$(dirname "$0")/.."

tier=full
base=origin/main
selftest=0
while [ $# -gt 0 ]; do
  case "$1" in
    --tier) tier="${2:?--tier precisa de fast|affected|full}"; shift 2 ;;
    --base) base="${2:?--base precisa de uma revisão}"; shift 2 ;;
    --selftest) selftest=1; shift ;;
    *) echo "uso: ci/verify.sh [--tier fast|affected|full] [--base REV] [--selftest]" >&2; exit 2 ;;
  esac
done
case "$tier" in fast | affected | full) ;; *) echo "nível desconhecido: $tier" >&2; exit 2 ;; esac

mkdir -p target
times=target/verify-times.tsv
printf 'step\tseconds\n' >"$times"
current=""
trap '[ -n "${failed:-}" ] || { failed=1; echo "VERIFY_FAILED tier=$tier step=$current" >&2; }' ERR

# step NOME COMANDO...: executa e cronometra; o nome aparece no TSV e na falha.
step() {
  current="$1"
  shift
  local start=${EPOCHREALTIME/,/.} # locale pt_BR usa vírgula decimal
  "$@"
  local end=${EPOCHREALTIME/,/.} seconds
  seconds=$(python3 -c "print(f'{$end - $start:.1f}')")
  printf '%s\t%s\n' "$current" "$seconds" >>"$times"
  echo "STEP_OK $current ${seconds}s"
}
py() { (cd py && "$@"); }

# Testes de escala/corpus demonstram critérios uma vez e rodam inteiros só no completo.
# Nos outros níveis rodam versões pequenas da mesma lógica (LOGOS_TIER=fast) ou são pulados
# por marcador `heavy` (pytest). Ver ADR0057.
if [ "$tier" = full ]; then
  export LOGOS_TIER=full
  pytest_args=(-q)
else
  export LOGOS_TIER=fast
  pytest_args=(-q -m "not heavy")
fi

step build-cake python3 tools/build_cake_lpr.py
step fmt cargo fmt --all --check
step clippy cargo clippy --workspace --all-targets -- -D warnings
step cargo-test cargo test --workspace
step tcb-lints ci/check-tcb-lints.sh
step audit-independence ci/check-audit-independence.sh
step tcb-manifest tcb/count.sh check
step ruff py uv run ruff check . ../tcb ../tools
step ruff-format py uv run ruff format --check . ../tcb ../tools
step mypy py uv run mypy
step mypy-tools py uv run mypy --strict ../tcb/tcb.py ../tcb/check_lints.py ../tcb/tests \
  ../tools/gen_unicode_assigned.py ../tools/gen_crypto_vectors.py ../tools/gen_cas_vectors.py \
  ../tools/gen_core_vectors.py ../tools/gen_air_vectors.py ../tools/gen_vocabulary_vectors.py \
  ../tools/gen_render_template_vectors.py ../tools/gen_notebook_vectors.py ../tools/gen_fact_vectors.py ../tools/gen_trust_vectors.py \
  ../tools/gen_paraphrase_vectors.py ../tools/check_ws_docs.py ../tools/gen_obligation_vectors.py \
  ../tools/gen_protocol_vectors.py ../tools/gen_kernel_protocol_vectors.py \
  ../tools/gen_model_vectors.py ../tools/gen_event_vectors.py ../tools/gen_audit_vectors.py \
  ../tools/gen_fragment_vectors.py ../tools/gen_grounding_vectors.py ../tools/gen_cnf_vectors.py \
  ../tools/gen_lrat_vectors.py ../tools/lrat_corpus.py ../tools/build_cake_lpr.py \
  ../tools/build_cadical.py ../tools/gen_sat_vectors.py ../tools/check_cnf.py \
  ../tools/check_ledger_crash.py ../tools/verify_plan.py ../tools/check_isolation.py
# Vetores normativos: todo gerador confere os arquivos versionados (--check).
for generator in unicode_assigned crypto_vectors cas_vectors core_vectors air_vectors \
  vocabulary_vectors paraphrase_vectors notebook_vectors fact_vectors trust_vectors obligation_vectors protocol_vectors \
  kernel_protocol_vectors model_vectors event_vectors audit_vectors fragment_vectors \
  grounding_vectors cnf_vectors lrat_vectors; do
  step "vectors-$generator" py uv run python "../tools/gen_$generator.py" --check
done
step lrat-corpus-manifest py uv run python ../tools/lrat_corpus.py --check
step ws-docs py uv run python ../tools/check_ws_docs.py # trilha WS: links/âncoras, IDs, grafo
step pytest py uv run pytest "${pytest_args[@]}" # inclui diferenciais canon e AIR-text
step pytest-tcb py uv run pytest -q ../tcb/tests

# Smokes e scripts de integração: todos no completo; no afetado, os das áreas tocadas
# (tools/verify_plan.py mapeia caminhos alterados para estes nomes).
smokes=(
  "core-smoke:cargo run -p logos-core --example core_smoke"
  "air-smoke:cargo run -p logos-air --example air_smoke"
  "vocabulary-smoke:cargo run -p logos-air --example vocabulary_smoke"
  "sort-check-smoke:cargo run -p logos-air --example sort_check_smoke"
  "air-text-smoke:cargo run -p logos-air-text --example air_text_smoke"
  "review-sheet:cargo run -p logos-seal --example review_sheet -- --check"
  "obligation-smoke:cargo run -p logos-obligation --example obligation_smoke"
  "protocol-smoke:cargo run -p logos-protocol --example protocol_smoke"
  "model-smoke:cargo run -p logos-model --example model_smoke"
  "eval-smoke:cargo run -p logos-model --example eval_smoke"
  "checker-smoke:cargo run -p logos-ledger --example checker_smoke"
  "events-smoke:cargo run -p logos-ledger --example events_smoke"
  "fragment-smoke:cargo run -p logos-ground --example fragment_smoke"
  "ground-smoke:cargo run -p logos-ground --example ground_smoke"
  "cnf-smoke:cargo run -p logos-ground --example cnf_smoke"
  "lrat-smoke:cargo run -p logos-lrat --example lrat_smoke"
  "z3-models-smoke:py uv run python -m logos_provers.z3_models_smoke"
  "model-finder-smoke:py uv run python -m logos_provers.model_finder_smoke"
  "model-explain-smoke:py uv run python -m logos_provers.model_explain_smoke"
  "check-cnf:ci/check-cnf.sh"
  "check-audit:ci/check-audit.sh"
  "check-sat:ci/check-sat.sh"
  "check-farm:ci/check-farm.sh"
  "check-mcp-research:ci/check-mcp-research.sh"
  "check-notebook:ci/check-notebook.sh"
  "check-isolation:ci/check-isolation.sh"
  "demo-kernel:ci/demo-kernel.sh"
  "check-render-templates:ci/check-render-templates.sh"
  "check-ledger-crash:ci/check-ledger-crash.sh"
  "check-ledger-index:ci/check-ledger-index.sh"
)
case "$tier" in
  full) selected=all ;;
  affected) selected=$(python3 tools/verify_plan.py --base "$base") ;;
  fast) selected="" ;;
esac
for entry in "${smokes[@]}"; do
  name="${entry%%:*}"
  read -r -a command <<<"${entry#*:}"
  if [ "$selected" = all ] || [[ " $selected " == *" $name "* ]]; then
    step "$name" "${command[@]}"
  fi
done

if [ "$tier" = full ]; then
  step cargo-deny cargo deny check
  step cargo-audit cargo audit
fi
if [ "$selftest" = 1 ]; then
  step selftest ci/selftest.sh
fi
echo "== etapas mais lentas (target/verify-times.tsv) =="
tail -n +2 "$times" | sort -t $'\t' -k2,2 -g -r | head -n 10
echo "VERIFY_OK tier=$tier"
