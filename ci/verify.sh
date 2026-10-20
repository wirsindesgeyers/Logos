#!/usr/bin/env bash
# Verificação completa local (AGENTS.md §4). Falha no primeiro erro; sem pipes que mascarem o
# código de saída. Use --selftest para incluir o autoteste da CI (~1-2 min, usa rede).
set -euo pipefail
cd "$(dirname "$0")/.."

python3 tools/build_cake_lpr.py
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p logos-core --example core_smoke
cargo run -p logos-air --example air_smoke
cargo run -p logos-air --example vocabulary_smoke
cargo run -p logos-air --example sort_check_smoke
cargo run -p logos-air-text --example air_text_smoke
cargo run -p logos-obligation --example obligation_smoke
cargo run -p logos-protocol --example protocol_smoke
cargo run -p logos-model --example model_smoke
cargo run -p logos-model --example eval_smoke
cargo run -p logos-ledger --example checker_smoke
cargo run -p logos-ledger --example events_smoke
cargo run -p logos-ground --example fragment_smoke
cargo run -p logos-ground --example ground_smoke
cargo run -p logos-ground --example cnf_smoke
cargo run -p logos-lrat --example lrat_smoke
ci/check-cnf.sh
ci/check-audit.sh
ci/check-sat.sh
ci/check-ledger-crash.sh
ci/check-ledger-index.sh
ci/check-tcb-lints.sh
ci/check-audit-independence.sh
tcb/count.sh check
cargo deny check
cargo audit
(
  cd py
  uv run ruff check . ../tcb ../tools
  uv run ruff format --check . ../tcb ../tools
  uv run mypy
  uv run mypy --strict ../tcb/tcb.py ../tcb/check_lints.py ../tcb/tests ../tools/gen_unicode_assigned.py ../tools/gen_crypto_vectors.py ../tools/gen_cas_vectors.py ../tools/gen_core_vectors.py ../tools/gen_air_vectors.py ../tools/gen_vocabulary_vectors.py ../tools/gen_obligation_vectors.py ../tools/gen_protocol_vectors.py ../tools/gen_kernel_protocol_vectors.py ../tools/gen_model_vectors.py ../tools/gen_event_vectors.py ../tools/gen_audit_vectors.py ../tools/gen_fragment_vectors.py ../tools/gen_grounding_vectors.py ../tools/gen_cnf_vectors.py ../tools/gen_lrat_vectors.py ../tools/lrat_corpus.py ../tools/build_cake_lpr.py ../tools/build_cadical.py ../tools/gen_sat_vectors.py ../tools/check_cnf.py ../tools/check_ledger_crash.py
  uv run python ../tools/gen_unicode_assigned.py --check
  uv run python ../tools/gen_crypto_vectors.py --check
  uv run python ../tools/gen_cas_vectors.py --check
  uv run python ../tools/gen_core_vectors.py --check
  uv run python ../tools/gen_air_vectors.py --check
  uv run python ../tools/gen_vocabulary_vectors.py --check
  uv run python ../tools/gen_obligation_vectors.py --check
  uv run python ../tools/gen_protocol_vectors.py --check
  uv run python ../tools/gen_kernel_protocol_vectors.py --check # requests v1 + propose-result/v1 + check-result/v1 + witness-result/v1
  uv run python ../tools/gen_model_vectors.py --check
  uv run python ../tools/gen_event_vectors.py --check
  uv run python ../tools/gen_audit_vectors.py --check
  uv run python ../tools/gen_fragment_vectors.py --check
  uv run python ../tools/gen_grounding_vectors.py --check
  uv run python ../tools/gen_cnf_vectors.py --check
  uv run python ../tools/gen_lrat_vectors.py --check
  uv run python ../tools/lrat_corpus.py --check
  uv run python -m logos_provers.z3_models_smoke
  uv run python -m logos_provers.model_finder_smoke
  uv run python -m logos_provers.model_explain_smoke
  uv run pytest -q # inclui diferenciais canon e AIR-text (500 fórmulas)
  uv run pytest -q ../tcb/tests
)
if [ "${1:-}" = "--selftest" ]; then
  ci/selftest.sh
fi
echo "VERIFY_OK"
