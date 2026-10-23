#!/usr/bin/env bash
# Etapa5.12 (ADR0060): kernel mínimo ponta a ponta + auditoria independente.
# vocabulário → fatos do atestador de teste → Verified/Refuted/Unknown → retratação
# → cascata → logos-audit verifica cadeia/assinaturas e recheca LRAT com Rust e cake_lpr.
set -euo pipefail
cd "$(dirname "$0")/.."
python3 tools/build_cake_lpr.py
cake="$(pwd)/target/cake-lpr/cake_lpr"
cargo build -q --release -p logos-kernel --example demo_kernel -p logos-audit --bin logos-audit
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
chmod 700 "$work"
out=$(target/release/examples/demo_kernel "$work/demo")
printf '%s\n' "$out"
field() { sed -n "s/.* $1=\([^ ]*\).*/\1/p" <<<"$out"; }
[[ "$(field invalidated)" == 1 ]]
report=$(target/release/logos-audit verify "$work/demo/ledger" --cas "$work/demo/ledger/cas" \
  --public-key "$(field public_key)" --cake-lpr "$cake" --rust-checker "$(field rust_checker)")
printf '%s\n' "$report"
[[ "$report" == AUDIT_OK* ]]
[[ "$report" == *"refutations_rechecked=1"* ]]
[[ "$report" == *"lrat_rust_rechecked=1 lrat_cake_rechecked=1"* ]]
echo "DEMO_KERNEL_AUDIT_OK verified=1 refuted=1 unknown=1 invalidated=1 audit=complete"
