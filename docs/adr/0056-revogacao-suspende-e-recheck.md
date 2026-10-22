# ADR 0056 — Revogação suspende; recheck reativa ou invalida

- Estado: aceito para implementação da Etapa 5.10 (2026-10-22), por delegação do
  mantenedor ("tome a decisão que for melhor e deixe documentado").
- Relacionadas: 0052 (ATMS/retratação), 0041 (CERTIFY/registry), 0030 (cake_lpr),
  0055 (retratação de operador); `spec/events.md` §10, `spec/ledger-index.md` §3,
  `spec/state-machine.md` §5, `spec/audit.md`.

## Contexto

A 5.10 pede `CheckerRevoked` (só admin), `logos recheck --checker h --with h2`
(Accepted → `Rechecked`, rota segue válida pelo checker novo; falha →
`Invalidated` + cascata) e estado `Suspended` para claims sustentados só por
checker revogado ainda não rechecados. Até a 5.9 a projeção **matava** as rotas
do checker revogado: a morte é terminal e monótona, então nenhum recheck
posterior recuperaria os dependentes. O critério ("nenhum claim perdido se os
certificados forem bons") é impossível nessa semântica.

## Decisão

1. **Revogação suspende, não mata.** Novo status de índice `Suspended` (código 7,
   layout ledger-index/v3) e `EntryKind::Suspended` (Query status kind 10,
   acrescentado ao fim do codebook; os anteriores não mudam). Suspenso não é
   citável (nem por PROPOSE nem por ações) e não é morto.
2. **Dois pontos fixos.** Morte segue o ADR0052, contando rota suspensa como
   viva. Atividade: um nó h(O) vivo é *ativo* se alguma rota sua de checker
   efetivo não revogado tem todas as premissas used ativas (premissa ativa =
   assumption viva ou nó ativo); senão suas rotas vivas ficam Suspended, e a
   suspensão é transitiva. Rotas refletem o estado do nó, como no ADR0052.
3. **Checker efetivo** de uma rota é o do Verified, substituído pelo do último
   Rechecked aceito. Revogar h suspende as rotas de checker efetivo h.
4. **Evento `Rechecked { route, CheckRecord }`** (tag 15, rechecked-body/v1),
   assinado pelo kernel, sem autoridade administrativa: mesmos h(O) e
   h(certificado) da rota, checker novo e desfecho. Accepted troca o checker
   efetivo e reativa (se o checker novo não estiver revogado); Rejected mata a
   rota e abre a cascata; Unknown mantém a suspensão. Rota alvo precisa estar viva.
   Uma rota nova de mesma h(O) por checker ativo também reativa o nó (reproposta).
5. **Registro da cascata** reutiliza `Retractions::reconcile`: um Rechecked
   rejeitado é raiz como um Retracted; `causes = [h(Rechecked), premissa morta]`.
   O alvo do recheck não recebe Invalidated (seu registro é o Rechecked).
   Verified emitida por checker já revogado nasce morta e não tem registro.
6. **Operação de operador** (`logos_kernel::recheck::Rechecks`), fora do protocolo:
   `revoke` exige a autoridade, checker registrado e não revogado; `recheck`
   exige checker novo ativo e diferente, roda o certificado original no
   formato pedido e grava um Rechecked por rota viva. Falha operacional do
   checker é erro, nunca Rejected. CLI: `logos ledger revoke-checker` e
   `logos ledger recheck --with <h2> --cake BIN`; a CLI só executa o cake_lpr
   fixado (a identidade Rust é o executável do kernel, não o da CLI).
7. **Auditor independente**: uma rota cujo certificado falha no recheck do
   auditor só é aceita se um Rechecked Rejected posterior a invalidar; Rechecked
   precisa casar com a rota (h(O)/h(certificado)) e com checker permitido, e
   Accepted/Rejected precisam ser reproduzidos. Relatório ganha
   `invalidated_by_recheck` (incidente visível, não escondido).

## Alternativas consideradas

1. **Manter revogação como morte** (rejeitada): torna o critério impossível.
2. **Só reproposta + CERTIFY com o checker novo** (insuficiente): cake no CERTIFY
   depende do Rust ativo para `used`; e a morte já teria levado os dependentes.
3. **Suspended como flag separada** (rejeitada): duplicaria o estado; o status
   único mantém `is_citable` e Query simples.
4. **Recalcular `used` no recheck** (adiada): `used` vem do fecho LRAT calculado
   pelo checker Rust; cake só aceita/rejeita. Rechecked confirma a validade da
   prova contra O inteira e conserva o `used` registrado (⊆ cited). Um bug do
   checker revogado *na extração de used* exigiria um extrator independente.
5. **Recheck pelo socket do agente** (rejeitada): P1, como no ADR0052.

## Consequências

- TCB: `logos-ledger` (catálogo de eventos, projeção, layout v3), `logos-kernel`
  (`recheck.rs`, reconcile, mapeamento kind 10), `logos-obligation` (EntryKind).
  Vetores de evento (30), layout de índice e schema de obrigação atualizados.
- Caches ledger-index/v2 são recusados e reconstruídos (sem migração).
- O cliente Python aceita kind 10; o caderno (5R.3) trata revogação como
  suspensão e Rechecked rejeitado como raiz de corte.
- Limite: recheck Rust de rotas cake exige um caminho de operador dentro do
  próprio kernel (não implementado); o cenário da 5.10 é Rust → cake.
