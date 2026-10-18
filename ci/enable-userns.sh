#!/usr/bin/env bash
# Garante que o bwrap consegue criar user namespaces para o sandbox do checker (usado pelos jobs
# rust e python do ci.yml). Sem user namespaces o isolamento do cake_lpr não inicia e o projeto
# não tem fallback sem isolamento (AGENTS.md §8).
#
# O Ubuntu 24.04 — inclusive os runners do GitHub — ativa
# kernel.apparmor_restrict_unprivileged_userns=1, e o bubblewrap do sistema não traz perfil
# AppArmor; o bwrap falha com "bwrap: setting up uid map: Permission denied". Estratégia:
# primeiro um perfil AppArmor mínimo só para /usr/bin/bwrap (userns, unconfined); se não
# resolver, desabilita o sysctl (VM efêmera de CI). Ao final o script roda o próprio bwrap como
# controle positivo: se ele não funcionar, o script falha em vez de deixar os testes rodarem.
set -euo pipefail

control() {
  bwrap --unshare-user --unshare-pid --unshare-net --unshare-ipc --unshare-uts \
    --new-session --die-with-parent --ro-bind / / /bin/true 2>&1
}

if control >/dev/null 2>&1; then
  echo "user namespaces do bwrap já funcionam"
  exit 0
fi

if [ ! -x /usr/bin/bwrap ]; then
  echo "erro: /usr/bin/bwrap não existe" >&2
  exit 1
fi

restrict=$(sysctl -n kernel.apparmor_restrict_unprivileged_userns 2>/dev/null || true)
printf 'bwrap bloqueado (kernel.apparmor_restrict_unprivileged_userns=%s); habilitando\n' "${restrict:-ausente}" >&2

if [ "$restrict" = "1" ]; then
  profile=/etc/apparmor.d/logos-bwrap-userns
  if sudo tee "$profile" >/dev/null <<'PROFILE'
abi <abi/4.0>,
include <tunables/global>

profile bwrap /usr/bin/bwrap flags=(unconfined) {
  userns,
  include if exists <local/bwrap>
}
PROFILE
  then
    sudo apparmor_parser -r "$profile" || true
    if control >/dev/null 2>&1; then
      echo "perfil AppArmor aplicado: bwrap cria user namespaces"
      exit 0
    fi
  fi
  echo "perfil AppArmor não bastou; desabilitando o sysctl" >&2
  sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0 || true
fi

if output=$(control); then
  echo "user namespaces do bwrap habilitados"
  exit 0
fi

echo "erro: bwrap continua sem criar user namespaces:" >&2
printf '%s\n' "$output" >&2
sysctl kernel.apparmor_restrict_unprivileged_userns kernel.unprivileged_userns_clone 2>&1 || true
exit 1
