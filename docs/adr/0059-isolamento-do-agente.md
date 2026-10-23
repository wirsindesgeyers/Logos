# ADR 0059 — Isolamento do processo do agente

- Estado: aceito para implementação da Etapa 5.11 (2026-10-23), por delegação do
  mantenedor ("tome a decisão que for melhor e deixe documentado"). Revisável.
- Relacionadas: §3 da spec conceitual (mecanismo 1: o agente não tem credenciais
  nem a chave do ledger), 0035 (transporte do kernel, SO_PEERCRED), 0030/0044
  (bwrap no cake_lpr e na farm), 0057 (níveis de verificação).

## Contexto

A 5.11 pede usuários de SO separados (`logos-kernel`, `logos-gateway`,
`logos-agent`), chave do kernel legível só pelo kernel, sandbox do agente sem rede
exceto a API do LLM e o socket do kernel/MCP, units de referência, e um pentest
scriptado: a partir do processo do agente, ler a chave do kernel, abrir o SQLite do
ledger em escrita, conectar ao gateway e ler as credenciais das ferramentas —
tudo falha. O gateway real é da 8.4; até lá vale um stub.

Criar usuários de SO exige root, que o ambiente de desenvolvimento e a CI não têm
de forma reproduzível. O `bwrap` sem privilégio já é usado (cake_lpr, farm).

## Decisão

1. **Duas camadas.** (a) Usuários de SO, como deploy de referência: `deploy/systemd`
   com `sysusers.d`, `tmpfiles.d` e três units. A chave fica em
   `/etc/logos/kernel.key` 0600 `logos-kernel`; ledger e área privada em
   `/var/lib/logos-kernel` 0700; credenciais 0600 `logos-gateway`. O socket do
   kernel fica em `/run/logos-kernel` 0750 com grupo `logos-socket` (só kernel e
   agente). O kernel continua autenticando o UID do peer. (b) Sandbox do agente,
   que é o que o pentest exercita: `logos-agent-sandbox` (crate `logos-sandbox`,
   TCB estrita) executa o agente em `bwrap` com namespaces próprios de usuário,
   PID, rede, IPC e UTS, `--cap-drop ALL`, `--clearenv` e `--die-with-parent`.
   Montagens: `/usr` somente leitura, CA e cache do loader, `/proc` e `/dev`
   próprios, `/tmp` tmpfs, o código do agente somente leitura em `/opt/logos`,
   o socket do kernel, o staging e, opcionalmente, o socket do proxy do LLM.
   Nada mais do host entra.
2. **Rede só para o LLM, por proxy.** O sandbox não tem rede. `LlmProxy` escuta num
   socket Unix (diretório privado, socket 0600) e encaminha cada conexão para o
   único `HOST:PORT` fixado pelo operador; os bytes passam sem interpretação (TLS
   fim a fim), e o agente não escolhe destino.
3. **Staging com usuários distintos.** O diretório de origem do agente continua 0700
   (regra do staging da 5.2, inalterada). A unit do kernel recebe só
   `CAP_DAC_OVERRIDE` como capability ambiente, dentro de `ProtectSystem=strict` com
   escrita apenas no ledger, no runtime e no staging. Ela não usa `RestrictNamespaces`
   nem `MemoryDenyWriteExecute`, porque o cake_lpr roda em `bwrap`.
4. **Pentest.** `tools/check_isolation.py` (via `ci/check-isolation.sh`, no verify e
   na CI) sobe o kernel real, um gateway stub (socket Unix e porta TCP), credenciais
   no home do operador e um LLM de eco. O probe `logos_agent.isolation_probe` roda
   duas vezes: fora do sandbox, toda tentativa proibida precisa **conseguir**
   (controle positivo); dentro, todas precisam falhar, e o permitido (head do
   kernel, escrita no staging, LLM pelo proxy) precisa funcionar. Tentativas: ler a
   chave, abrir o ledger e o índice para escrita, listar a área privada do kernel,
   conectar ao gateway por Unix e por TCP, ler as credenciais, abrir rede direta ao
   LLM, ler `/proc/<kernel>/environ` e `/proc/<kernel>/cmdline`. As units são
   conferidas com `systemd-analyze verify` (só se tolera o executável ainda não
   instalado) e com usuários distintos por unit.

## Alternativas consideradas

1. **Exigir os usuários de SO no teste** (rejeitada): pede root; a CI e a máquina de
   desenvolvimento não teriam o critério reproduzível. As units ficam como
   referência validada, e a instalação real fica com o operador.
2. **Landlock/seccomp próprios** (adiado): dependência nova na TCB. Namespaces com
   montagens mínimas já cumprem o critério; um filtro seccomp pode vir depois por
   `bwrap --seccomp`.
3. **Container rootless (podman)** (rejeitada): dependência e superfície maiores que
   o `bwrap`, que já está na base de confiança operacional (cake_lpr).

## Consequências

- O critério é demonstrado com o mesmo UID nos dois lados: o isolamento vem do
  sandbox, e a separação por usuário é uma segunda camada, não exercitada aqui.
- Os itens sobre gateway e credenciais usam o stub e serão repetidos na 8.4.
- O agente real (`logos_agent`) ainda não existe; a unit aponta para ele.
