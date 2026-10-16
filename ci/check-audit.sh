#!/usr/bin/env bash
# Interoperabilidade real, binários separados, consumidor sem banco SQLite.
set -euo pipefail
cd "$(dirname "$0")/.."
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
cargo build -p logos-audit
key=$(cargo run --quiet -p logos-ledger --example audit_fixture -- "$work/ledger")
audit="${CARGO_TARGET_DIR:-target}/debug/logos-audit"
report=$("$audit" verify "$work/ledger" --cas "$work/ledger/cas" --public-key "$key")
printf '%s\n' "$report"
[[ "$report" == *"cnf_rederived=3"* ]]
"$audit" export "$work/ledger" --public-key "$key" > "$work/export.logos"
# Mover o CAS e remover o SQLite impede uso acidental do produtor pelo consumidor.
mv "$work/ledger/cas" "$work/cas"
rm -rf "$work/ledger"
report=$("$audit" verify "$work/export.logos" --cas "$work/cas" --public-key "$key")
printf '%s\n' "$report"
[[ "$report" == *"cnf_rederived=3"* ]]
ci/check-audit-independence.sh
echo "AUDIT_SMOKE_OK refutations=4/4 consistency=1 cnf=3/3 export_roundtrip=true"
