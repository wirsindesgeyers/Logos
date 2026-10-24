# ADR 0065 — SDK de atestador e atestadores de referência

- Estado: aceito para implementação da Etapa 6.3 (2026-10-24), por delegação do
  mantenedor. Revisável.
- Relacionadas: 0061 (fact/v1), 0062 (fonte Fact de sort enumerado), 0063 (confiança
  por tópico), 0064 (revogação), 0009 (vocabulário e grounding); `spec/fact.md`,
  `spec/vocabulary.md` §3 (Anchored), `docs/attesters/`.

## Contexto

A 6.3 pede `logos-attest` (Rust) e `logos_attest` (Python) com `observe`, assinatura e
envio ao kernel; seis atestadores de referência (`clock`, `fs-stat`, `db-monitor`
com mock de `pg-monitor`, `backup-svc`, `enum-source`, `sandbox-run`); e um documento
de procedimento por atestador para o status `Anchored`. Duas lacunas apareceram:
AIR não tem literais numéricos nem de texto (só constantes declaradas no vocabulário),
e o binário do kernel não expõe `publish_trust`/`register`/`revoke_issuer`.

## Decisão

1. **Fórmulas qualitativas, números na `version`.** Os símbolos são predicados sobre
   constantes nominais (`file_present(config_file)`, `clock_synced`, `backup_recent(db)`).
   Valores medidos (hash e mtime do arquivo, desvio do relógio, instante do backup,
   fingerprint da linha) vão no par `version` do fato (`recurso`, `texto`), que já é
   assinado. Observação falsa vira a **negação** (`¬file_present(x)`), fato também
   citável; falha de observação é erro, nunca um fato padrão.
2. **Um vocabulário por observação quando o conteúdo é dinâmico.** `sandbox-run` declara
   um sort `Digest` com as constantes `sha256:<hex>` do script, da entrada e da saída e o
   predicado `computed/3`; o `h(V)` do fato é o desse vocabulário. `enum-source` monta o
   vocabulário do sort enumerado e afirma a conjunção fecho ∧ nomes distintos, com
   `FactRef` = hash da fórmula (ADR 0062). Os demais usam vocabulários fixos pequenos.
3. **Vocabulário de exemplo** (`logos_attest::example_vocabulary`): união dos símbolos dos
   seis atestadores, todos `Anchored(<atestador>, <procedimento>)`. O teste confere
   que nenhum símbolo é `Glossed`, que o `AttestorId` e o procedimento existem em
   `REFERENCES` e que o documento `docs/attesters/<id>.md` cita id, procedimento e tópico.
4. **Envio ao kernel.** Em Rust, `submit` põe vocabulário e fato no CAS e chama
   `facts::register` (kernel como biblioteca, ledger aberto pelo escritor). Para o
   kernel parado, a CLI de operador ganha `logos ledger trust-publish|fact-register|
   revoke-issuer` (mesma autoridade do kernel, como `retract`, ADR 0052). Esta CLI é a
   entrega de fatos do Python: `logos_attest` grava `fact-<hash>.bin` e
   `vocabulary-<hash>.bin`. **Não** há mensagem de protocolo para registrar fato num
   kernel em execução: continua lacuna (STATUS), a resolver com a Fase 8.
5. **Python em paridade byte a byte.** `logos_attest` usa o canon e o parser AIR de
   `logos_client` e Ed25519 de `cryptography`. O `clock` em Python gera os mesmos bytes que
   o de Rust (vetor `spec/vectors/attest/clock.json`, conferido pelos dois lados). A chave
   vive no processo do atestador; nenhum módulo de `logos_agent`/`logos_mcp`/`logos_provers`
   pode importar `logos_attest` (teste). `cryptography` sobe de dev para dependência
   de runtime do projeto Python.
6. **`sandbox-run` sem `--nproc`.** `RLIMIT_NPROC` conta os processos do usuário inteiro e
   faz o `bwrap` falhar em máquinas ocupadas; o isolamento de PID é o limite. CPU,
   memória, tamanho de arquivo e tempo de parede continuam. Executa duas vezes e exige
   saída idêntica; script/entrada alterados durante a execução invalidam o fato.
7. **`pg-monitor` é mock** (`MockPgProbe`), como o roadmap permite. `SqliteProbe` abre
   o arquivo só para leitura e não inventa contagem de conexões (erro).

## Alternativas consideradas

1. **Estender AIR com literais** (rejeitada): muda formato e TCB; fora do escopo.
2. **Um vocabulário monolítico para todas as observações** (rejeitada): exigiria
   conhecer de antemão cada hash de saída do `sandbox-run` e cada caminho vigiado.
3. **Protocolo de registro de fatos agora** (adiada): decisão de autenticação do
   atestador junto ao kernel, melhor na Fase 8 com o gateway.
4. **Atestadores de referência também em Python** (rejeitada): só o `clock` é espelhado,
   para provar a paridade do SDK; o resto seria duplicação sem novo risco coberto.

## Consequências

- Sem formato novo na TCB; `logos-attest` segue fora da TCB de correção (fidelidade,
  `strict = false`). Dependências: `rusqlite` (já na allowlist) e `logos-kernel`.
- A confiança continua sendo do operador (6.2): sem `Trusted(emissor, tópico)` o fato
  não registra. Riscos: o atestador só é tão confiável quanto o processo e a chave dele;
  fatos de arquivo/banco descrevem o instante da leitura (TOCTOU, mitigado por validade
  curta e hash na versão); `sandbox-run` prova a cadeia de hashes, não a verdade empírica
  do dado (L1); determinismo que dependa do host e se repita nas duas execuções passa.
