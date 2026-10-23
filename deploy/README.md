# Deploy de referência (Etapa 5.11, ADR0059)

Três usuários de SO, um por processo: `logos-kernel` (chave e ledger),
`logos-gateway` (credenciais de ferramentas; stub até a 8.4) e `logos-agent`
(sem chave nem credencial, rodando dentro de `logos-agent-sandbox`).

Instalação (como root):

```sh
install -Dm644 deploy/systemd/logos.sysusers.conf /usr/lib/sysusers.d/logos.conf
install -Dm644 deploy/systemd/logos.tmpfiles.conf /usr/lib/tmpfiles.d/logos.conf
systemd-sysusers && systemd-tmpfiles --create
install -m755 target/release/logos-kernel target/release/logos-agent-sandbox /usr/local/bin/
install -Dm600 -o logos-kernel -g logos-kernel <chave> /etc/logos/kernel.key
echo "AGENT_UID=$(id -u logos-agent)" > /etc/logos/kernel.env
echo "LLM_ENDPOINT=api.exemplo:443" > /etc/logos/agent.env
install -m644 deploy/systemd/*.service /etc/systemd/system/
systemctl daemon-reload && systemctl enable --now logos-kernel logos-agent
```

O pentest sem root (`ci/check-isolation.sh`) exercita o sandbox do agente e valida
estas units com `systemd-analyze verify`; a separação por usuário é a segunda camada.
