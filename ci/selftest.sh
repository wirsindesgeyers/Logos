#!/usr/bin/env bash
# Autoteste da CI (Etapa 0.1): cada regra precisa realmente falhar quando violada.
# Trabalha numa cópia temporária; o repositório não é alterado.
set -uo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
work="$(mktemp -d)"
trap 'rm -rf "$work"' EXIT
export CARGO_TARGET_DIR="$root/target/selftest"
fail=0

fresh_copy() {
  rm -rf "$work/repo" && mkdir "$work/repo"
  (cd "$root" && tar --exclude=./target --exclude=./.git --exclude=./py/.venv --exclude=./fuzz/target -cf - .) \
    | tar -x -C "$work/repo"
  cd "$work/repo" || exit 1
}

expect_failure() { # descrição, comando...
  local desc="$1"; shift
  if "$@" >"$work/out.log" 2>&1; then
    echo "FALHOU: '$desc' deveria ser bloqueado, mas passou"; fail=1
  else
    echo "ok: $desc é bloqueado"
  fi
}

fresh_copy
printf '\npub fn violacao() { unsafe { core::hint::unreachable_unchecked() } }\n' >> crates/logos-canon/src/lib.rs
expect_failure "unsafe {} em crate de TCB" cargo build -p logos-canon

fresh_copy
sed -i '/#!\[forbid(unsafe_code)\]/d' crates/logos-canon/src/lib.rs
expect_failure "remover forbid(unsafe_code) de crate de TCB" ci/check-tcb-lints.sh

fresh_copy
printf '\npub fn violacao() -> u8 { Some(1u8).unwrap() }\n' >> crates/logos-canon/src/lib.rs
expect_failure "unwrap() em crate de TCB" cargo clippy -p logos-canon -- -D warnings

fresh_copy
printf '\n[dependencies]\nitoa = "1"\n' >> crates/logos-canon/Cargo.toml
expect_failure "dependência fora da allowlist" cargo deny check bans

fresh_copy
printf '\n[dependencies]\nlogos-kernel = { path = "../logos-kernel" }\n' >> crates/logos-audit/Cargo.toml
expect_failure "logos-audit dependendo de logos-kernel" ci/check-audit-independence.sh

[ "$fail" = 0 ] && echo "selftest: todas as regras bloqueiam o que deveriam" || exit 1
