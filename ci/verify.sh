#!/usr/bin/env bash
# Verificação completa local (AGENTS.md §4). Falha no primeiro erro; sem pipes que mascarem o
# código de saída. Use --selftest para incluir o autoteste da CI (~1-2 min, usa rede).
set -euo pipefail
cd "$(dirname "$0")/.."

cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p logos-core --example core_smoke
cargo run -p logos-air --example air_smoke
cargo run -p logos-air --example vocabulary_smoke
cargo run -p logos-air --example sort_check_smoke
cargo run -p logos-air-text --example air_text_smoke
cargo run -p logos-obligation --example obligation_smoke
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
  uv run mypy --strict ../tcb/tcb.py ../tcb/check_lints.py ../tcb/tests ../tools/gen_unicode_assigned.py ../tools/gen_crypto_vectors.py ../tools/gen_cas_vectors.py ../tools/gen_core_vectors.py ../tools/gen_air_vectors.py ../tools/gen_vocabulary_vectors.py ../tools/gen_obligation_vectors.py
  uv run python ../tools/gen_unicode_assigned.py --check
  uv run python ../tools/gen_crypto_vectors.py --check
  uv run python ../tools/gen_cas_vectors.py --check
  uv run python ../tools/gen_core_vectors.py --check
  uv run python ../tools/gen_air_vectors.py --check
  uv run python ../tools/gen_vocabulary_vectors.py --check
  uv run python ../tools/gen_obligation_vectors.py --check
  uv run pytest -q # inclui diferenciais canon e AIR-text (500 fórmulas)
  uv run pytest -q ../tcb/tests
)
if [ "${1:-}" = "--selftest" ]; then
  ci/selftest.sh
fi
echo "VERIFY_OK"
