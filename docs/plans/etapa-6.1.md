# Etapa6.1 — objeto Fact

Estado: 6.1a implementada localmente (2026-10-23); 6.1b pendente. Decisão em
[ADR 0061](../adr/0061-objeto-fact-e-registro.md).

1. **Entradas:** assinatura (0.4, tag `logos/sig/fact/v1` reservada), eventos (3.1),
   EpochRange (3.4, ADR0023), citação/snapshot (5.3), CAS `fact-v1` reservado.
2. **Saídas (6.1a):** `logos_ledger::{Fact, Validity, ResourceVersion}` (fact/v1);
   `EventBody::FactRegistered` (tag16); índice ledger-index/v4 (status Fact=8,
   tabela `facts`); `LedgerEntry::expired` e `Reject::Expired`;
   `logos_kernel::facts::{TrustRoots, register}`; auditor com `facts_verified`;
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
4. **6.1b (pendente):** fonte `Fact` de enumeração autenticada e resolvida pelo
   evento correto → axiomas explícitos em O; certificado real usa os axiomas;
   `used`/deps rastreiam a fonte; fonte ausente/adulterada recusada (ADR0026).
5. **Riscos:** projeção agora lê o CAS (fato ausente invalida o índice);
   `Until` depende do relógio do kernel; raiz de confiança é configuração até a 6.2.
6. **Tamanho:** `M` (o critério de enumeração do ADR0026 aumentou a etapa), TCB.
