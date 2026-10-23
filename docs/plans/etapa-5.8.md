# Etapa5.8 — especulação de claims

Estado: implementação local (2026-10-23) sobre a855c11. Decisão em
[ADR 0058](../adr/0058-especulacao-de-claims.md).

1. **Entradas:** PROPOSE/CERTIFY/REFUTE (5.3/5.4), escopos (5.7), ATMS e
   `Retractions::reconcile` (5.9), suspensão por revogação (5.10). O campo
   `speculative_on` já existia em Proposed, no protocolo e no cliente Python.
2. **Saídas:** `logos_obligation::generate_speculative`; consultas
   `active_route`/`dead_routes` no índice; módulo `logos_kernel::speculation`
   (`History`, `settle_bases`, `sweep`); transições `recover_pending_at`, `settle`,
   `abandon`. Nenhum formato serializado muda.
3. **Tarefas:** ADR; gerador; consultas do índice; PROPOSE com bases e escopo
   transitivo; CERTIFY com bases resolvidas (rota em `cited`, base trocada em
   `used`); REFUTE e reconcile com varredura `SpeculationFailed`; WITNESS recusa;
   specs (kernel-protocol, obligation, events, state-machine §6); testes e
   mutações.
4. **Critério → evidência:**
   - *C8 especula sobre C7; C7 refutado → C8 `Invalidated` com causa
     `SpeculationFailed(C7)`*:
     `logos-kernel/tests/speculation.rs::criterion_c8_speculates_on_c7_and_refuting_c7_invalidates_c8_with_speculation_failed`
     (C9 sobre C8 cai transitivamente com `[Refuted, C8]`; CERTIFY antes da base
     é recusado sem evento; Pending não é citável por PROPOSE nem por ação;
     reconcile idempotente e índice reconstruído igual).
   - *Profundidade de propagação de erro = 0 sem especulação, 1000 cadeias*:
     `criterion_error_propagation_depth_is_zero_without_speculation_over_random_chains`
     — `ERROR_PROPAGATION_DEPTH chains=1000 interleaved_depth=0
     speculative_chains=100 speculative_depth=138` (controle positivo: com
     especulação a métrica não é trivialmente zero). 40 cadeias no nível rápido.
   - Demais: base Verified libera o especulativo com `used` mapeado para a rota e
     ATMS comum depois; base que morre após Verified derruba o especulativo ainda
     Pending (`[Retracted, C7]`); recusas de base desconhecida, assumption, rota,
     base concluída, escopos misturados e WITNESS especulativo; gerador
     (`logos-obligation/tests/generation.rs`) e consultas do índice
     (`logos-ledger/tests/index.rs`). Mutações em `etapa-5.8-mutacoes.json`.
5. **Riscos:** a varredura e a resolução de bases custam um replay completo
   (mesma ordem do CERTIFY atual); o auditor independente não confere a semântica
   da cascata (como na 5.9); rota escolhida para a base é a menor ativa, então
   uma segunda rota posterior da base não muda o Verified já emitido.
6. **Tamanho:** `M`, TCB.
