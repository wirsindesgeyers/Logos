# Etapa5.9 — retratação e invalidação em cascata (ATMS)

Estado: implementação local (2026-10-22), feita em worktree paralelo enquanto o
verify da 9.1 rodava, e rebaseada sobre ela. Decisão em
[ADR 0052](../adr/0052-atms-por-obrigacao-e-retratacao.md).

## 1. Interfaces de entrada

- Catálogo event/v1 (3.1): `Retracted { target, reason, authority }`,
  `Invalidated { claim, causes }`, assinatura de autoridade (events.md §4).
- Índice 3.3 (`deps` = used, 4.7), projeção única `index_project::apply`.
- Typestate 5.1: transição `invalidate(Claim<Verified>, causes)`; replay 5.4.

## 2. Interfaces de saída

```rust
pub struct Retractions;                     // logos_kernel::retract (Linux)
impl Retractions {
    pub fn new(authority: PublicKey) -> Self;
    pub fn retract<S: Signer, A: Signer>(&self, &mut Ledger<S>, &A, Hash<Event>, Text)
        -> Result<Retraction, TransportError>;   // Retracted + Invalidated gravados
    pub fn reconcile<S: Signer>(&self, &mut Ledger<S>) -> Result<Vec<Hash<Event>>, TransportError>;
}
```

Projeção do índice com semântica ATMS por nó `h(O)` (ledger-index.md §3), consumida
por PROPOSE/CERTIFY/WITNESS via citabilidade e pela 5.7/5R.3.

## 3. Tarefas

1. ADR 0052; `spec/ledger-index.md` §3, `spec/state-machine.md` §4, `spec/events.md`.
2. `logos-ledger::index_project`: worklist ATMS no lugar do fechamento SQL.
3. `logos-kernel`: `retract.rs` (autoridade, Retracted, reconcile) e
   `Transitions::recover_verified`; `reconcile` no arranque de `logos-kernel`.
4. Testes: `logos-ledger/tests/index.rs` (rotas independentes, rota retratada,
   rebuild), `logos-kernel/tests/retract.rs` (cadeias, autoridade, crash).
5. Diferencial: `examples/atms_fixture.rs` (ledger real) × `py/tests/test_atms_reference.py`
   (referência ingênua, ponto fixo do zero a cada passo).

## 4. Critério → evidência

| Critério | Evidência |
| --- | --- |
| DAG de 10k nós: retratação produz exatamente o conjunto esperado (referência Python ingênua) | `test_atms_reference.py`: 10 000 claims, várias rotas por h(O), 25 retratações sequenciais; conjunto final e **cada registro** (claim, raiz, premissa) iguais aos da referência |
| Rota independente sobrevive à retratação de uma | `index.rs::independent_routes_keep_node_and_dependents_until_every_route_falls`, `retract.rs::independent_route_survives_and_cascade_is_recorded_when_both_fall`; no diferencial, há rotas vivas com premissa derrubada |

## 5. Riscos e decisões

- Nó é `h(O)`, não a fórmula: rotas sobre citações diferentes são nós diferentes
  (ADR 0052, alternativa 1 adiada).
- Revogação de checker continua derrubando rotas pela mesma propagação, mas sem
  registro Invalidated; o tratamento completo é da 5.10.
- O desempenho da propagação é proporcional à região afetada; retratações são raras.

## 6. Tamanho

`M`, como no roadmap. Mudança na TCB de correção (`logos-ledger`, `logos-kernel`).
