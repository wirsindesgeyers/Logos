# Etapa5.6 — bloqueio de re-proposta refutada (S4)

Estado: implementação local (2026-10-22) em worktree paralelo. Decisão em
[ADR 0053](../adr/0053-s4-bloqueio-de-reproposta-refutada.md).

1. Entradas: índice 3.3 (`claims.h_o`, status Refuted), PROPOSE 5.3, REFUTE 5.4,
   cliente 10.1.
2. Saídas: `LedgerIndex::refuted`/`LedgerLiveIndex::refuted`; `TransportError::AlreadyRefuted`;
   erro already-refuted/v1; `logos_client.client.AlreadyRefuted`.
3. Tarefas: layout ledger-index/v2 (`claims_h_o`); consulta; checagem no PROPOSE antes
   do CAS; codificação do erro; cliente Python; vetor independente; specs.
4. Critério → evidência:
   - refutar e re-propor com citações em outra ordem → recusado:
     `certify.rs::s4_reproposal_with_reordered_citations_is_refused_with_the_countermodel`
     (detalhe com h(O)/h(Refuted)/h(M), cabeça e CAS inalterados);
   - prover com timeout na segunda tentativa não converte em Unknown:
     `certify.rs::s4_timeout_on_a_second_attempt_cannot_turn_refutation_into_unknown`
     (PROPOSE/CERTIFY/REFUTE recusados com orçamento de 1 passo; cabeça inalterada).
   - Vetor: `already-refuted-v1.json` conferido por Rust (`wire.rs`) e Python.
5. Riscos: cache v1 existente é reconstruído na primeira abertura (custo único).
6. Tamanho: `P`, como no roadmap.
