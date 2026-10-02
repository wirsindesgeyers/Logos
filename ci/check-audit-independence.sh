#!/usr/bin/env bash
# logos-audit não pode depender (nem transitivamente) de logos-kernel ou logos-ledger (roadmap §2, S1).
set -euo pipefail
cd "$(dirname "$0")/.."
deps=$(cargo metadata --format-version 1 --all-features | python3 -c '
import json, sys
meta = json.load(sys.stdin)
pk = {p["id"]: p["name"] for p in meta["packages"]}
nodes = {n["id"]: n for n in meta["resolve"]["nodes"]}
workspace = set(meta["workspace_members"])
audit = next(pid for pid in workspace if pk[pid] == "logos-audit")
seen, stack = set(), [audit]
while stack:
    pid = stack.pop()
    if pid in seen:
        continue
    seen.add(pid)
    stack += [d["pkg"] for d in nodes[pid]["deps"]]
print("\n".join(sorted({pk[pid] for pid in seen})))')
for forbidden in logos-kernel logos-ledger; do
  if grep -qx "$forbidden" <<<"$deps"; then
    echo "ERRO: logos-audit depende de $forbidden" >&2
    exit 1
  fi
done
echo "ok: logos-audit independente de logos-kernel e logos-ledger"
