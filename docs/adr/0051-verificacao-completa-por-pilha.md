# ADR 0051 — Verificação completa por pilha de commits locais

- Estado: aceito em 2026-10-22, por delegação do mantenedor, depois de ele apontar
  que o desenvolvimento estava parado 2–3 h por etapa à espera de verificação
  ("tem algum trabalho que seja paralelizável pra você não ficar toda hora empacado").
  Revisável a pedido dele.
- Relacionadas: AGENTS.md §2, §4 e §5.

## Contexto

`ci/verify.sh --selftest` leva cerca de 1 h nesta máquina (corpus LRAT, farm 500,
crash do ledger, índice 1M, smokes, testes Python). A regra antiga exigia a
verificação completa antes de **cada** commit. Com uma pessoa e um agente, isso
serializava as etapas: fuzz de 1 h + full de 1 h + correções = 2–3 h de espera por
etapa, mesmo quando a etapa seguinte não dependia da anterior.

## Decisão

1. **Etapas independentes em worktrees paralelos.** Cada uma num `git worktree` com
   branch local temporária (`etapa-X.Y`), processos pesados com `nice`. Arquivos que
   uma verificação em curso confere por hash não são editados no diretório dela.
2. **Commit local com verificação direcionada.** Um commit de etapa pode ser criado
   quando passaram: `cargo fmt --check`, `clippy -D warnings` e testes dos crates
   tocados, vetores/geradores `--check` afetados, Ruff/mypy dos arquivos Python
   tocados, mutações com controle positivo e (TCB) `tcb check`/lints. O commit diz
   no corpo que a verificação completa está pendente na pilha.
3. **Verificação completa na ponta da pilha.** Antes de qualquer push, e no máximo a
   cada três commits de etapa empilhados, roda `ci/verify.sh --selftest` sobre a
   ponta. O recibo (`target/verify-*.json`, versionado em `docs/plans/`) lista os
   commits cobertos. Só então os critérios que exigem a verificação completa são
   marcados no ROADMAP, e o STATUS registra a cobertura.
4. **Falha na ponta.** Corrige-se com novo commit (ou `--amend`/rebase local se nada
   foi enviado) e repete-se a verificação completa; nenhum critério é marcado antes.
5. **Fuzz longo em paralelo.** Fuzz de ≥1 h roda desacoplado (`setsid`), em paralelo
   ao desenvolvimento, sobre fontes congeladas registradas por hash no recibo.

## Alternativas consideradas

1. **Manter a regra antiga** (rejeitada): seguro, mas 2–3 h ociosas por etapa.
2. **Dispensar a verificação completa** (rejeitada): perderia as regressões
   cruzadas que só o conjunto pega (já pegou falhas reais, ex.: Ruff, fixture de
   startup, cancelamento da farm).
3. **Paralelizar o próprio verify** (complementar): possível depois de medir quais
   etapas dominam; não resolve a serialização entre etapas.

## Consequências

- Um commit local pode existir sem verificação completa própria; o push nunca.
- Bisseção local pode cair num commit não verificado sozinho; o recibo da ponta diz
  quais commits cobriu.
- Nenhuma garantia do sistema muda: é regra de processo, não de TCB.
