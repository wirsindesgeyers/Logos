# ADR 0063 — Raízes e política de confiança por tópico

- Estado: aceito para implementação da Etapa 6.2 (2026-10-23), por delegação do
  mantenedor. Revisável.
- Relacionadas: 0061 (objeto Fact; raiz de confiança provisória do kernel), 0062,
  0056 (revogação suspende), 0052 (ATMS); `spec/trust.md`, `spec/events.md` §12,
  `spec/ledger-index.md`, `spec/obligation.md` §3, `spec/audit.md`.

## Contexto

A 6.2 pede `TrustConfig { roots: [KeyId → {name, topics, level}] }` assinada pelo
admin e versionada no ledger; fato citável só se o emissor é confiável **para o
tópico** do fato; e, "na lógica", a confiança como premissa explícita
`Trusted(issuer, topic)` (prepara P8). Critério: `db-monitor` assinando fato de
tópico `backup` → não citável. Na 6.1 a raiz era `facts::TrustRoots`, configuração
do operador, válida para qualquer tópico.

## Decisão

1. **Objeto trust/v1 no CAS** (`spec/trust.md`): `[1, version, roots]`, raízes em
   ordem estrita de `KeyId`, cada uma `[PublicKey, name, topics, level]` com
   tópicos em ordem estrita. Tag de hash nova `logos/trust/v1` e namespace CAS
   `trust-v1`. A chave pública (não só o `KeyId`) fica no objeto, como no fact/v1.
2. **Assinatura do admin no evento**, pelo mecanismo `Authority` já usado por
   `CheckerRegistered`/`Retracted`: `TrustConfigPublished { h(config), authority }`,
   tag 17. O admin é a chave administrativa configurada pelo operador (como na
   retratação); o objeto não carrega assinatura própria.
3. **Versão estritamente crescente**; a vigente é a de maior versão. Configuração
   nova **substitui** a anterior inteira (não é delta).
4. **Regra no registro e na citação.** `facts::register` exige
   `Trusted(KeyId, tópico)` vigente (sem configuração publicada, ninguém atesta);
   `TrustRoots` foi removido. Na citação, o snapshot marca o fato `untrusted` e o
   gerador recusa com o novo `Reject::Untrusted` (depois de `Expired`). Tópico casa
   por igualdade exata.
5. **Sem efeito retroativo nesta etapa.** Retirar um tópico impede citações novas,
   mas não derruba claims Verified; isso é revogação de emissor (6.4), que usará a
   cascata ATMS. `used` gravado depois da retirada é erro de projeção.
6. **`Trusted(issuer, topic)` como premissa explícita no ledger, não em AIR.** O
   índice guarda, para cada concessão vigente, o evento `TrustConfigPublished` que a
   concedeu (`trust_grant`, `fact_trust`). Isso diverge da letra do roadmap ("na
   lógica"): pôr `Trusted` em O exigiria um vocabulário de confiança compatível com
   todo V e mudaria obligation/v1. Fica para a 17.4 (P8), que já é a etapa da
   invariante; o ROADMAP da 6.2 registra o ajuste.
7. **Projeção e auditor conferem de novo.** ledger-index/v5 (colunas emissor/tópico
   em `facts`, tabelas `trust` e `trust_configs`); `FactRegistered` sem concessão e
   versão que não cresce são erros de reconstrução. O auditor independente reaplica
   as configurações com decoder próprio e confere cada fato (`trust_configs=`).
8. **`level` informativo.** Guardado e auditado, sem efeito na regra até que uma
   etapa defina o uso (ex.: limiares por ação na Fase 8).

## Alternativas consideradas

1. **Configuração só local (arquivo do operador)** (rejeitada): o roadmap pede
   versionamento no ledger, e o auditor não teria como reconferir.
2. **Assinatura do admin dentro do objeto** (rejeitada): duplicaria o mecanismo
   `Authority` e o caminho de verificação.
3. **Tópicos hierárquicos por prefixo** (rejeitada por ora): `backup` cobrir
   `backup/x` é conveniente, mas abre ambiguidade de normalização; igualdade exata
   é o mínimo seguro e o prefixo pode vir como versão nova do formato.
4. **Retirar confiança derruba claims** (adiada para a 6.4): exige decidir `since`,
   cascata e auditoria da revogação, que são o escopo da 6.4.
5. **`Trusted` como fórmula AIR em O** (adiada para a 17.4): ver decisão 6.

## Consequências

- Formatos novos: trust/v1, corpo de evento tag 17, ledger-index/v5 (caches v4
  recusados e reconstruídos), `Reject::Untrusted`, `LedgerEntry::untrusted`,
  `trust_configs=` no AUDIT_OK (o parser Python acompanha).
- A API `facts::register` perde o parâmetro de raiz; quem registra fatos precisa de
  uma configuração publicada antes (testes da 6.1 migrados).
- A chave do admin continua fora do ledger: o auditor não autentica o admin. Uma
  etapa futura pode ancorá-la no Genesis ou numa configuração inicial.
- Lacuna conhecida (revisão de 24/out): o binário do kernel não expõe
  `publish_trust` nem `facts::register` (nem mensagem de protocolo nem CLI). O
  envio de fatos virá com o SDK da 6.3; a publicação pelo admin ainda não tem etapa
  dona e precisa ser atribuída (sugestão: CLI de operador junto da 6.3).
- Todo sort enumerado de V exige o fato da fonte (6.1b), mesmo sem uso no objetivo;
  o grounder exige o mesmo. Coerente, mas obriga citar fontes irrelevantes.
