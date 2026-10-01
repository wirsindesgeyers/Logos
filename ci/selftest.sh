#!/usr/bin/env bash
# Autoteste da CI (Etapa 0.1): cada regra precisa realmente falhar, *pelo motivo certo*, quando
# violada, e passar numa cópia limpa (controle). Trabalha numa cópia temporária.
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

# Insere `linha` dentro da seção [dependencies] de um Cargo.toml (cria a seção se faltar).
add_dependency() { # arquivo, linha
  if grep -q '^\[dependencies\]' "$1"; then
    sed -i "/^\[dependencies\]/a $2" "$1"
  else
    printf '\n[dependencies]\n%s\n' "$2" >> "$1"
  fi
}

# O comando precisa FALHAR e a saída precisa conter o diagnóstico esperado.
expect_blocked() { # descrição, padrão-do-diagnóstico, comando...
  local desc="$1" pattern="$2"; shift 2
  if "$@" >"$work/out.log" 2>&1; then
    echo "FALHOU: '$desc' deveria ser bloqueado, mas passou"; fail=1
  elif ! grep -Eq "$pattern" "$work/out.log"; then
    echo "FALHOU: '$desc' falhou, mas NÃO pelo motivo esperado (/$pattern/):"
    sed 's/^/    | /' "$work/out.log" | head -8; fail=1
  else
    echo "ok: $desc é bloqueado (diagnóstico: $pattern)"
  fi
}

expect_passes() { # descrição, comando...
  local desc="$1"; shift
  if "$@" >"$work/out.log" 2>&1; then
    echo "ok: controle — $desc passa na cópia limpa"
  else
    echo "FALHOU: controle — $desc deveria passar na cópia limpa:"
    sed 's/^/    | /' "$work/out.log" | head -8; fail=1
  fi
}

# --- controle: o repositório limpo passa em todas as verificações usadas abaixo ---
fresh_copy
expect_passes "cargo build -p logos-canon" cargo build -p logos-canon
expect_passes "clippy -D warnings" cargo clippy -p logos-canon -- -D warnings
expect_passes "ci/check-tcb-lints.sh" ci/check-tcb-lints.sh
expect_passes "ci/check-audit-independence.sh" ci/check-audit-independence.sh
expect_passes "tcb/count.sh check" tcb/count.sh check
expect_passes "cargo deny check bans" cargo deny check bans

# --- violações ---
fresh_copy
printf '\npub fn violacao() { unsafe { core::hint::unreachable_unchecked() } }\n' >> crates/logos-canon/src/lib.rs
expect_blocked "unsafe {} em crate de TCB" 'usage of an `unsafe` block|forbid\(unsafe_code\)' cargo build -p logos-canon

fresh_copy
sed -i '/#!\[forbid(unsafe_code)\]/d' crates/logos-canon/src/lib.rs
expect_blocked "remover forbid(unsafe_code)" 'logos-canon/src/lib.rs não contém #!\[forbid\(unsafe_code\)\]' ci/check-tcb-lints.sh

fresh_copy
printf '\npub fn violacao() -> u8 { Some(1u8).unwrap() }\n' >> crates/logos-canon/src/lib.rs
expect_blocked "unwrap() em crate de TCB" 'clippy::unwrap_used|used `unwrap\(\)`' cargo clippy -p logos-canon -- -D warnings

fresh_copy
add_dependency crates/logos-canon/Cargo.toml 'itoa = "1"'
expect_blocked "dependência fora da allowlist" "crate 'itoa[^']*' is not explicitly allowed" cargo deny check bans

fresh_copy
add_dependency crates/logos-audit/Cargo.toml 'logos-kernel = { path = "../logos-kernel" }'
expect_blocked "logos-audit dependendo de logos-kernel" 'logos-audit depende de logos-kernel' ci/check-audit-independence.sh

fresh_copy
printf '\n' > crates/logos-canon/src/novo_arquivo.rs
expect_blocked "arquivo novo em crate de TCB sem manifesto" 'logos-canon: arquivo fora do manifesto: src/novo_arquivo.rs' tcb/count.sh check

[ "$fail" = 0 ] && echo "selftest: todas as regras bloqueiam o que deveriam, pelo motivo certo" || exit 1
