# ADR 0062 — Fonte Fact de sort enumerado

- Estado: aceito para implementação da Etapa 6.1b (2026-10-23), por delegação do
  mantenedor. Revisável.
- Relacionadas: 0026 (fontes de fechamento), 0061 (objeto Fact); `spec/fact.md` §4,
  `spec/finite-fragment.md`, `spec/vocabulary.md` §3.1.

## Contexto

O último critério da 6.1 (herdado da 4.1 pelo ADR0026) pede: fonte `Fact` de
enumeração autenticada e resolvida pelo evento correto → axiomas explícitos em O;
certificado real usa os axiomas; `used`/deps rastreiam a fonte; fonte ausente ou
adulterada é recusada; sem copiar hash de payload para `PremiseRef`.

Até aqui o kernel recusava qualquer vocabulário com sort `Enumerated` no CERTIFY
(`unresolved enumeration source`) e o PROPOSE não fazia grounding dele. O grounder
(4.2) já exigia os axiomas de fecho e nomes únicos como premissas explícitas de O.

Problema de identidade: se `FactRef` fosse `h(Fact)`, haveria ciclo — o vocabulário
contém `FactRef`, e o fato contém `h(V)` (e precisa ser bem tipado num V compatível,
que declara o mesmo sort com a mesma fonte).

## Decisão

1. **`FactRef` = `hash(Formula)` da conjunção** `And(fecho, distinct…)` do sort, na
   ordem dos candidatos da 4.1 (`AxiomCandidates::conjunction`). A fórmula depende só
   de nomes de sort/constantes, não de `h(V)`: sem ciclo. O vocabulário passa a
   comprometer-se com o conteúdo exato que a fonte precisa atestar.
2. **Um fato carrega a conjunção inteira.** O grounder aceita uma premissa igual à
   conjunção como suporte único do sort (preferida aos axiomas separados, que
   continuam aceitos para uso condicional). Uma referência por sort mantém `used`
   pequeno e evita registrar n(n−1)/2 fatos.
3. **Autoridade no kernel** (`logos-kernel/src/enumeration.rs`, TCB), no PROPOSE e
   de novo em todo contexto de CERTIFY/REFUTE/WITNESS sobre O regenerado:
   `FactRef` confere com a conjunção de V; alguma premissa de O tem a conjunção;
   toda premissa com a conjunção resolve para entrada `Fact` no snapshot. A premissa
   chega a O pelo gerador comum, pela citação do evento `FactRegistered`.
4. **`Policy` continua recusada**, agora já no PROPOSE, até a 7.3.
5. **PROPOSE faz grounding de V enumerado** (antes pulava): sem fonte, o grounder
   recusa (`MissingDomainAxiom`) ou o kernel recusa; com fonte, a rota é LRAT.

## Alternativas consideradas

1. **`FactRef` = `h(Fact)`** (impossível): ciclo V ↔ fato.
2. **`FactRef` = tópico ou emissor** (rejeitada): pertence à confiança por tópico da
   6.2; não prende o conteúdo atestado, e um fato do tópico com enumeração diferente
   precisaria de outra conferência.
3. **Um fato por axioma** (rejeitada): n(n−1)/2+1 registros e citações por sort;
   `FactRef` não identificaria um único objeto.
4. **Kernel injeta os axiomas em O a partir do fato** (rejeitada): abriria o
   construtor selado de O (ADR0026 proíbe) e duplicaria o gerador.

## Consequências

- Sem formato serializado novo: `FactRef` mantém os 32 bytes; muda a semântica.
  Vocabulários com `FactRef` arbitrário continuam decodificando, mas são recusados
  no uso.
- A checagem no CERTIFY é redundante para O gerado pelo PROPOSE atual; ela cobre
  Pending gravado sem essa checagem (ledger anterior) e tem teste próprio.
- Revogação de fatos (6.4) derruba o claim pelo ATMS, como para qualquer premissa.
- Fonte `Policy` (7.3) e projeção (7.5) devem seguir o mesmo desenho: compromisso
  com a conjunção no vocabulário, premissa citada, kernel confere o tipo da entrada.
