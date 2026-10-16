#!/usr/bin/env bash
# Interoperabilidade real, binários separados, consumidor sem banco SQLite.
set -euo pipefail
cd "$(dirname "$0")/.."
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
python3 tools/build_cake_lpr.py
cake="$(pwd)/target/cake-lpr/cake_lpr"
cargo build -p logos-audit
key=$(cargo run --quiet -p logos-ledger --example audit_fixture -- "$work/ledger" "$cake")
audit="${CARGO_TARGET_DIR:-target}/debug/logos-audit"
report=$("$audit" verify "$work/ledger" --cas "$work/ledger/cas" --public-key "$key" --cake-lpr "$cake")
printf '%s\n' "$report"
[[ "$report" == *"cnf_rederived=6"* ]]
[[ "$report" == *"lrat_rust_rechecked=3 lrat_cake_rechecked=3"* ]]
"$audit" export "$work/ledger" --public-key "$key" > "$work/export.logos"
# Mover o CAS e remover o SQLite impede uso acidental do produtor pelo consumidor.
mv "$work/ledger/cas" "$work/cas"
rm -rf "$work/ledger"
report=$("$audit" verify "$work/export.logos" --cas "$work/cas" --public-key "$key" --cake-lpr "$cake")
printf '%s\n' "$report"
[[ "$report" == *"cnf_rederived=6"* ]]
[[ "$report" == *"lrat_rust_rechecked=3 lrat_cake_rechecked=3"* ]]
ci/check-audit-independence.sh
echo "AUDIT_SMOKE_OK refutations=4/4 consistency=1 cnf=6/6 lrat=3/3-two-checkers export_roundtrip=true"
