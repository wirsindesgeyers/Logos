# Etapa6.1 — objeto Fact

Estado: 6.1a e 6.1b implementadas localmente (2026-10-23). Decisões em
[ADR 0061](../adr/0061-objeto-fact-e-registro.md) e
[ADR 0062](../adr/0062-fonte-fact-de-enumeracao.md).

1. **Entradas:** assinatura (0.4, tag `logos/sig/fact/v1` reservada), eventos (3.1),
   EpochRange (3.4, ADR0023), citação/snapshot (5.3), CAS `fact-v1` reservado.
2. **Saídas (6.1a):** `logos_ledger::{Fact, Validity, ResourceVersion}` (fact/v1);
   `EventBody::FactRegistered` (tag16); índice ledger-index/v4 (status Fact=8,
   tabela `facts`); `LedgerEntry::expired` e `Reject::Expired`;
   `logos_kernel::facts::{TrustRoots, register}` (`TrustRoots` substituído pela
   configuração no ledger na 6.2, ADR0063); auditor com `facts_verified`;
   fuzz `fact_decode`; vetores `spec/vectors/fact/fact.json` (Python independente).
3. **Critério → evidência (6.1a):**
   - *Emissor desconhecido → recusado*:
     `logos-kernel/tests/facts.rs::unknown_issuer_bad_signature_and_duplicates_are_refused_without_events`
     (também assinatura que não cobre o corpo, bytes fora de fact/v1, fato ausente,
     duplicado; nenhum evento gravado).
   - *Fato expirado → não citável*: `criterion_expired_fact_is_not_citable`
     (`Until` vencido durante a sessão → `Expired`; janela de épocas vencida →
     `InvalidEpoch`; registro já vencido recusado).
   - *[3,5] citável em 4 e recusado em 6 pelo ledger/gerador reais*:
     `criterion_fact_valid_3_to_5_is_citable_in_4_and_refused_in_6` (PROPOSE e
     CERTIFY LRAT reais na época 4 com `used = [fato]` e dependente no índice;
     PROPOSE recusado na 6 sem evento; índice reconstruído do zero igual).
   - Projeção recusa `used` de fato fora da janela; auditor recusa fato mal
     assinado gravado sem o kernel. Mutações 10/10 (`etapa-6.1-mutacoes.json`);
     fuzz `fact_decode` 301s/1.817.093 execuções sem crash.
4. **6.1b — fonte de enumeração (ADR0062):** `FactRef = hash(Formula)` da conjunção
   fecho ∧ nomes únicos (`AxiomCandidates::conjunction`); o grounder aceita essa
   premissa como suporte único do sort; `logos-kernel/src/enumeration.rs` confere no
   PROPOSE e no CERTIFY/REFUTE/WITNESS que ela vem de entrada `Fact`. Evidência em
   `logos-kernel/tests/facts.rs`:
   - `criterion_enumeration_source_fact_puts_axioms_in_o_and_is_traced_by_used`:
     sem a fonte → recusado; fato registrado com fecho errado → recusado; a
     conjunção exata como hipótese → recusada; axiomas separados como hipóteses
     (o grounder aceitaria) → recusados; com o fato da fonte, a conjunção está em
     O com o `PremiseRef` do evento `FactRegistered`, a prova LRAT real é aceita, o
     `used` contém o fato e o índice o lista em deps;
   - `enumeration_source_must_match_the_vocabulary_and_policy_stays_refused`:
     `FactRef` que não é o hash da conjunção → recusado;
   - `certify_rechecks_the_source_of_a_pending_written_without_it`: Pending gravado
     sem a checagem (ledger anterior) com axiomas por hipótese → CERTIFY recusa;
   - `certify.rs::enumeration_sources_are_not_promoted_by_hash_or_cast`: Policy
     recusada no PROPOSE, inclusive com o fecho citado como hipótese;
   - `logos-ground/tests/grounding.rs::enumeration_conjunction_is_exact_and_covers_the_sort_as_one_premise`.
   Mutações 9/9 (`etapa-6.1b-mutacoes.json`).
5. **Riscos:** projeção agora lê o CAS (fato ausente invalida o índice); a
   fonte prende o conteúdo do fecho, não o emissor (confiança por tópico é da 6.2);
   `Until` depende do relógio do kernel; raiz de confiança é configuração até a 6.2.
6. **Tamanho:** `M` (o critério de enumeração do ADR0026 aumentou a etapa), TCB.
