# Etapa5.11 — isolamento de processos

Estado: implementação local (2026-10-23). Decisão em
[ADR 0059](../adr/0059-isolamento-do-agente.md).

1. **Entradas:** socket do kernel com SO_PEERCRED e staging (5.2), ledger (3.2),
   bwrap (4.5/10.2). Gateway real é da 8.4: aqui, stub.
2. **Saídas:** crate `logos-sandbox` (TCB; `Plan`, `LlmProxy`, binário
   `logos-agent-sandbox`); `deploy/systemd` (sysusers, tmpfiles, units do kernel,
   gateway e agente); probe `logos_agent.isolation_probe`; `tools/check_isolation.py`
   e `ci/check-isolation.sh` (verify e CI).
3. **Critério → evidência:** *pentest scriptado a partir do processo do agente —
   chave do kernel, SQLite do ledger em escrita, gateway direto, credenciais das
   ferramentas — todas falham*: `ci/check-isolation.sh` →
   `ISOLATION_OK forbidden=9 blocked=all control_reached=9 allowed=3 units=3`. As
   9 tentativas conseguem fora do sandbox (controle positivo) e falham dentro;
   kernel, staging e LLM pelo proxy funcionam dentro. Testes do crate:
   `crates/logos-sandbox/tests/sandbox.rs`. Mutações: 9/9
   (`etapa-5.11-mutacoes.json`).
4. **Pendente de ambiente:** a instalação real com usuários de SO precisa de root
   (`systemd-sysusers`, `systemd-tmpfiles --create`, units em
   `/etc/systemd/system`); ver `deploy/README.md`. Repetir os itens de gateway e
   credenciais na 8.4.
5. **Riscos:** sem seccomp/landlock próprios; `CAP_DAC_OVERRIDE` no kernel em
   produção (limitada por `ProtectSystem=strict`).
6. **Tamanho:** `M`, TCB.
