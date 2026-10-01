#!/usr/bin/env bash
# logos-audit não pode depender (nem transitivamente) de logos-kernel ou logos-ledger (roadmap §2, S1).
set -euo pipefail
cd "$(dirname "$0")/.."
deps=$(cargo metadata --format-version 1 --no-deps | python3 -c '
import json, sys
meta = json.load(sys.stdin)
pk = {p["name"]: p for p in meta["packages"]}
seen, stack = set(), ["logos-audit"]
while stack:
    n = stack.pop()
    if n in seen or n not in pk:
        continue
    seen.add(n)
    stack += [d["name"] for d in pk[n]["dependencies"]]
print("\n".join(sorted(seen)))')
for forbidden in logos-kernel logos-ledger; do
  if grep -qx "$forbidden" <<<"$deps"; then
    echo "ERRO: logos-audit depende de $forbidden" >&2
    exit 1
  fi
done
echo "ok: logos-audit independente de logos-kernel e logos-ledger"
