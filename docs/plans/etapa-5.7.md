# Etapa5.7 — assumptions com escopo

Estado: implementação local (2026-10-22) em worktree paralelo, sobre a 5.9.
Decisão em [ADR 0054](../adr/0054-assumptions-com-escopo.md).

1. Entradas: catálogo 3.1 (AssumptionDeclared), PROPOSE 5.3, índice 3.3, retratação 5.9.
2. Saídas: handler ASSUME (assume-result/v1), `Query::Open` (open-result/v1),
   `LedgerIndex::open_assumptions`, escopo derivado em PROPOSE/CERTIFY/WITNESS,
   cliente Python `assume`/`open_assumptions`.
3. Tarefas: `scope.rs`, `assume.rs`, wire/servidor, índice, cliente, vetores, specs.
4. Critério → evidência:
   - claim derivado de assumption aparece como condicional no Query:
     `certify.rs::assumption_derived_claim_is_conditional_in_query_and_falls_when_retracted`
     (Query open devolve a assumption e o escopo; claim sem assumption devolve lista vazia);
   - rejeitar a assumption (5.9) invalida os dependentes: o mesmo teste retrata a
     assumption do agente e confere o dependente Invalidated e o incondicional Verified.
   - Regras de escopo: `assume_scopes_are_normative_and_proposals_derive_a_single_scope`.
5. Riscos: `until` sem efeito prático antes da 6.1; origem agente/usuário só na 8.2.
6. Tamanho: `P`/`M` (protocolo e cliente além do previsto).
