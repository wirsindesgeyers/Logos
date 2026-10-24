# Etapa6.6 — conteúdo não confiável como `fonte diz φ`

Estado: implementada localmente (2026-10-24). Decisão em
[ADR 0068](../adr/0068-says-levantado-e-extracted.md); spec em [spec/says.md](../../spec/says.md).

1. **Entradas:** AIR (1.1), fatos e confiança (6.1, 6.2), SDK de atestador (6.3), assumptions com
   escopo (5.7), cascata ATMS (5.9).
2. **Saídas:** `EventBody::Extracted` (tag 19) com codec, projeção e `cas_references`;
   `logos_kernel::extracted::{register, SAYS_ATTESTOR, SAYS_PROCEDURE, SOURCE_SORT, EXTRACTION_TOPIC}`;
   `Retractions` aceita o extrator; auditor decodifica a tag 19; `logos_attest::reference::ingest`
   (`Ingest`, `says_lift`, `source_decl`) e `docs/attesters/ingest.md` (7º atestador de referência);
   vetor `Extracted` em `spec/vectors/events`; `spec/says.md`, `spec/events.md` §14.
3. **Critério → evidência:**
   - *`Says(email1, Pedido(x))` nunca satisfaz uma premissa que exige `Pedido(x)` sem a regra que o
     endossa*: `logos-kernel/tests/extracted.rs::criterion_says_never_satisfies_the_plain_premise_without_the_endorsing_rule`
     (a CNF de O com só a extração é satisfatível por DPLL, logo sem prova LRAT; com a regra de
     endosso como assumption há prova real, checada pelo auditor com cake_lpr).
   - *O certificado mostra a premissa `Says(...)` em `used`*: o mesmo teste lê `used` do `Verified`
     (extração e regra), `dependents(extração)` e `open_assumptions` (escopo `session:extracted`).
   - Registro: `extraction_needs_a_trusted_extractor_and_a_closed_says_literal_of_the_named_source`
     (sem confiança, tópico errado, extrator revogado; predicado simples, grounding de outro
     atestador, procedimento errado, mal tipada, fonte trocada, sort do 1º argumento, composta,
     termo aberto — todos sem evento; a negação do literal passa).
   - Retratação: `only_the_extractor_or_the_admin_retracts_and_dependents_fall`.
   - Ingestão: `logos-attest/tests/reference.rs::ingest_attests_content_hash_and_origin_but_not_meaning`
     e `says_lift_declares_a_distinct_anchored_symbol_with_a_source_first`.
   - Mutações em `etapa-6.6-mutacoes.json`: 12 mortas, 2 equivalentes por defesa em
     profundidade (E6, E7), 1 sobrevivente conhecida do auditor (E15, ver arquivo).
4. **Riscos:** ver ADR 0068 (vínculo ingestão↔extração fora do kernel; escopo compartilhado;
   `Says` só atômico). Fuzz: o decoder novo é o `EventBody` já coberto por `event_decode`;
   corpus de fuzz da tag 19 fica para o fim da fase.
5. **Tamanho:** `M`, TCB (fidelidade).
