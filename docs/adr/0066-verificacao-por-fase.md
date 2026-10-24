# ADR 0066 — Verificação pesada no fim da fase, não a cada etapa

- Estado: aceito em 2026-10-24, por delegação do mantenedor, depois de ele perguntar
  se não seria melhor rodar a suíte grande só ao finalizar uma fase inteira, porque
  rodá-la a cada etapa "está lesando demais o desenvolvimento". Revisa o ADR 0057
  (itens de frequência) e o 0051. Revisável a pedido dele.
- Relacionadas: 0051, 0057, AGENTS.md §2, §4 e §5.

## Contexto

O ADR 0057 criou os níveis `fast` (~8 min), `affected` e `full` (45–65 min), mas
manteve `fast` em todo commit e `full` a cada pilha de no máximo três etapas. Na
Fase 6 isso virou um `full` a cada 2–3 sub-etapas (6.1, 6.2+6.4, …) mais um `fast` por
commit. Nos fulls recentes as falhas vieram de efeitos entre etapas (índice O(n²),
vetor de transporte, allowlist do `cargo-deny`), que o `full` pega e o `fast` não;
um teste direcionado do crate tocado pega o resto durante o desenvolvimento.

## Decisão

1. **Durante a etapa:** só testes direcionados (`cargo test -p <crates tocados>`,
   `cargo clippy -p …`, `cargo fmt`, pytest dos arquivos novos, `--check` do gerador de
   vetores tocado). Nenhum `ci/verify.sh`.
2. **Commit de etapa:** `fmt` + `clippy` do workspace + testes dos crates tocados e dos
   que dependem deles + mutações quando a etapa toca TCB. `ci/verify.sh --tier fast`
   deixa de ser exigido por commit; continua o atalho recomendado quando a etapa
   altera crate compartilhado (`logos-core`, `logos-canon`, `logos-crypto`, `logos-air`,
   `logos-ledger`, `logos-kernel`) ou formato serializado.
3. **Fim da fase (e antes de qualquer push):** `ci/verify.sh --selftest` completo na
   ponta, com recibo listando os commits cobertos. O limite de três commits de etapa sem
   `full` cai; vale "uma fase, ou o que o mantenedor mandar enviar antes".
4. **Critérios do ROADMAP** só são marcados `- [x]` quando o `full` do fim da fase passar;
   até lá a etapa fica como "implementada localmente, verificação completa pendente".
5. Uma etapa que mexa em desempenho/escala (índice, farm, corpus) roda o teste de escala
   correspondente na própria etapa, porque o `fast` o reduz e o `full` só viria depois.

## Consequências

- Menos horas de máquina por fase; o risco é achar uma regressão entre etapas só no fim
  da fase. Mitigação: o recibo do `full` aponta o commit que quebrou (`git bisect` sobre
  os commits da fase) e as mutações da TCB continuam por etapa.
- Fases longas acumulam commits sem `full`; o mantenedor pode pedir um `full`
  intermediário a qualquer momento.
