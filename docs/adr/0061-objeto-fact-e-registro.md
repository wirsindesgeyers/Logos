# ADR 0061 — Objeto Fact, registro por raiz de confiança e janela de validade

- Estado: aceito para implementação da Etapa 6.1 (2026-10-23), por delegação do
  mantenedor. Revisável.
- Relacionadas: 0023 (épocas e fatos), 0026 (fontes de fechamento), 0052 (ATMS),
  0060 (atestador stub da demo); `spec/fact.md`, `spec/events.md` §11,
  `spec/ledger-index.md`, `spec/obligation.md` §3, `spec/audit.md`.

## Contexto

A 6.1 pede `Fact { formula, vocab, issuer, topic, observed_at, valid: EpochRange |
Until(ts), version?, sig }`, `FactRegistered { h(fact) }` só se a assinatura verifica
contra uma raiz de confiança, e quatro critérios: emissor desconhecido recusado;
fato expirado não citável; fato [3,5] citável em 4 e recusado em 6 pelo ledger e
gerador reais (ADR0023); e fonte `Fact` de enumeração com axiomas em O, prova real
e `used`/deps rastreando a fonte (ADR0026). A tag de hash `logos/fact/v1`, a tag de
assinatura `logos/sig/fact/v1` e o namespace CAS `fact-v1` já estavam reservados.

## Decisão

1. **Dividir a etapa.** 6.1a: objeto, registro, janela e citação (critérios 1–3).
   6.1b: fonte de enumeração (critério 4), que mexe no grounder/CERTIFY e vem em
   seguida. Nenhum critério é marcado por partes.
2. **fact/v1** (`spec/fact.md`): array de 9 campos; o emissor é guardado como
   **chave pública** (o `KeyId` do roadmap é `h(PublicKey)`), para que auditor e
   kernel verifiquem a assinatura só com o ledger. Assinatura sobre o corpo sem a
   assinatura; identidade sobre o objeto completo.
3. **Raiz de confiança da 6.1 = configuração do kernel** (`facts::TrustRoots`,
   chaves de emissores). Na 6.2 vira `TrustConfig` assinada e versionada no ledger,
   por tópico. Registro (`facts::register`) é API do kernel; transporte para
   atestadores externos vem com o SDK da 6.3.
4. **Registro** só com: objeto no CAS e decodificável, emissor na raiz, assinatura
   válida, fórmula bem tipada no vocabulário, janela não vencida e fato novo.
5. **Índice ledger-index/v4**: status `Fact`=8 e tabela `facts` (id, h(fact),
   first/last ou until, BLOB8). A projeção lê o fato do CAS (precisa do CAS; a
   leitura confere o hash). Fato conta como premissa viva/ativa para o ATMS; em
   `used`, vale se a época do Verified estiver na janela.
6. **Janela**: épocas inclusivas, ou `Until(t)` = qualquer época desde o registro
   até o instante `t` inclusive. O snapshot resolve a época de aplicabilidade (a
   corrente se aplicável; senão o limite mais próximo → `InvalidEpoch`). `Until`
   vencido marca a entrada `expired` e o gerador recusa com o novo
   `Reject::Expired`, logo após `Retracted`. A replay da projeção não lê relógio
   (determinismo); o instante é conferido pelo kernel ao gerar/regenerar O.
7. **Auditor**: aceita a tag 16, confere o objeto no CAS e reverifica a assinatura
   do emissor com decoder próprio (`facts_verified` no AUDIT_OK). Não decide
   confiança no emissor.

## Alternativas consideradas

1. **Copiar fórmula/janela no corpo do evento** (rejeitada): duplicaria o objeto e
   exigiria conferir coerência; a projeção lê o fato do CAS uma vez.
2. **Representar `Until` como `InvalidEpoch`** (rejeitada): o erro mentiria sobre
   a época; `Expired` diz o motivo certo.
3. **`KeyId` no objeto** (rejeitada): sem a chave, ninguém além do kernel
   verificaria a assinatura.

## Consequências

- Formatos novos: fact/v1, corpo de evento tag 16, ledger-index/v4 (caches v3
  recusados e reconstruídos), `Reject::Expired`, campo `facts_verified` no auditor.
- `LedgerEntry` ganha `expired` (todas as views precisam preenchê-lo).
- Revogação de fatos e de emissores é da 6.4; tópicos, da 6.2.
- 6.1b (fonte de enumeração): ADR0062.
