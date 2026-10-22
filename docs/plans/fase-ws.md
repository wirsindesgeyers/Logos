# Plano de fase — trilha WS (interface web, workspaces e artefatos)

Estado em 2026-10-22: WS.0 (documental) concluída por delegação do mantenedor;
WS.1–WS.9 não iniciadas. Escopo e critérios no
[Apêndice G](../../ROADMAP.md#apêndice-g--interface-web-workspaces-e-artefatos-trilha-proposta);
contratos em [`WS§`](../spec/LOGOS-workspace-and-web-ui.md); decisões em
[ADR 0047](../adr/0047-produto-unico-com-clientes-e-workspaces.md),
[0048](../adr/0048-servidor-web-local-nao-confiavel.md) e
[0049](../adr/0049-artefatos-visualizacoes-e-documentos.md).

## Ordem e caminho crítico interno

```text
            5R.4 ──► WS.1 ──► WS.2 ★ (precisa também de 5R.5)
                       │
MEM.0 ► MEM.1 ──► WS.3 ──► WS.4 ──► WS.5 ──► WS.7 ──► WS.8 ★ ──► WS.9
                             │                 ▲         ▲
5R.3 ────────────────────────┘          9.1 ──┘   5R.3/5R.4
MEM.5 + MEM.6 (após 8.9/9.2) ──► WS.6 (contexto automático) ──► WS.8 (painéis cross-session)
```

Caminho crítico até o primeiro marco (WS.2, chat web local real): 9.1 → 5R.2 →
5.9/5.7 → 5R.3 → 5R.4 → WS.1 → 5R.5 → WS.2. Ou seja, **a web só começa depois do
caminho da 5R**; nada aqui antecipa trabalho dela.

## Interfaces entre etapas

| De → para | Interface |
| --- | --- |
| 5R.4 → WS.1 | blocos `ledger_ref`/`free_text`, adaptação progressiva (`WS§2.2`) e regra de cartão stale (com extensão monotônica de vocabulário por `used_symbols(φ)`) |
| WS.1 → WS.2, 5R.5/10.7 | `spec/client-session.md`: sessão, eventos com cursor, componentes inline, projeção JSON (consumida por TUI, CLI one-shot e web) |
| MEM.0/MEM.1 → WS.3 | custódia, ACL/scope, idempotência e CAS da memória (namespace `Artifact`/`Project`) |
| WS.3 → WS.4 | `Project`, `ProjectSessionLink`, revisões, `scope_id` |
| 5R.3 → WS.4 | caderno de hipóteses/evidências/claims (bundle `notebook/v1`, mais operações de experimento crucial, MUS checado e tabelas `model/v1`) a importar sem perda |
| WS.4 → WS.5/WS.7 | `ProjectArtifact`, `Visualization` persistida com executor isolado de receitas (`bwrap`/`prlimit`), fonte de `Report` |
| 9.1 → WS.7 | enunciados por `render_pt` com `h(P)` |
| MEM.5/MEM.6 → WS.6 | rota `project` do Memory Router e snapshots |

## Paralelismo

- WS.0 é documental e não bloqueia nada.
- WS.3 pode avançar em paralelo com WS.1/WS.2 assim que MEM.1 existir: projetos
  também funcionam pela CLI/TUI.
- WS.5 e WS.7 compartilham WS.4 e podem andar juntas.
- WS.6 é tardia por construção; até ela, a continuidade entre sessões é só por
  anexos explícitos (WS.4).

## Marcos e como demonstrá-los

- **WS.2 ★:** o roteiro scriptado da 5R.5 completo no navegador e na TUI contra o
  mesmo harness, com os testes adversariais de origem/CSRF/rebinding passando.
- **WS.8 ★:** um projeto piloto do começo ao fim (pergunta → evidência → hipótese →
  visualização → relatório), com estados honestos, retratação e inventário de que
  tudo também é alcançável pelo chat.

## Decisões já tomadas (WS.0)

Um harness e um ledger; projetos no store da memória; servidor Rust (`axum`) fora
da TCB; SSE + POST; segredo de sessão de uso único; Preact/TypeScript com assets
embutidos; Vega-Lite (subconjunto fechado, CSP), Cytoscape.js, KaTeX; Typst para PDF
em sandbox, com exportação da fonte LaTeX. Motivos e critérios de reversão nos ADRs.
