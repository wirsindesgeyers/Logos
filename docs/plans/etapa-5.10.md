# Etapa5.10 — revogação de checker e recheck

Estado: implementação local (2026-10-22) em worktree paralelo sobre c50cd93
(5R.3). Decisão em [ADR 0056](../adr/0056-revogacao-suspende-e-recheck.md).

1. **Entradas:** registry/CERTIFY 5.4, cake_lpr 4.5, checker Rust 4.4, ATMS e
   retratação 5.9 (`index_project`, `Retractions::reconcile`), auditor 3.6/4.5.
2. **Saídas:** `EventBody::Rechecked` (tag 15), status Suspended (índice v3,
   kind 10), `logos_kernel::recheck::{Rechecks, RecheckReport}`, CLI
   `logos ledger revoke-checker|recheck`, auditor com `invalidated_by_recheck`.
3. **Tarefas:** ADR; catálogo/codec/CAS do evento; projeção com dois pontos
   fixos; reconcile generalizado; módulo de recheck; CLI; auditor; vetores;
   Python (cliente, MCP auditor, caderno); testes, mutações e fuzz.
4. **Critério → evidência:**
   - *Revogar o checker LRAT próprio; rechecar tudo com cake_lpr; nenhum claim
     perdido se os certificados forem bons; certificado plantado como
     falso-aceito é invalidado*:
     `certify.rs::criterion_revoke_rust_recheck_with_cake_keeps_good_claims_and_invalidates_planted`
     (provas LRAT reais; revogar o Rust suspende c1, c2, planted e d; recheck
     com cake: c1/c2 voltam Verified e citáveis, planted é Rejected e
     Invalidated, d cai com `causes = [Rechecked, planted]`; idempotente; índice
     reconstruído do zero igual) e
     `independent_auditor_accepts_planted_route_only_after_rejected_recheck`.
   - Semântica do índice: 6 testes em `logos-ledger/tests/index.rs`
     (suspensão transitiva, Unknown mantém, Rejected cascata, rota ativa
     independente, reativação por rota nova, recusas da projeção).
   - CLI: `logos-cli/tests/recheck.rs`. Mutações: 12/12 mortas
     (`etapa-5.10-mutacoes.json`). Fuzz `event_decode` com a tag nova.
5. **Riscos:** `used` não é recalculado no recheck (ADR0056 alt. 4); recheck
   com Rust só por caminho interno do kernel; custo da worklist proporcional à
   região afetada.
6. **Tamanho:** `M`, TCB.
