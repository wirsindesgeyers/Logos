# ADR 0049 — Artefatos, visualizações declarativas e documentos reproduzíveis

- Estado: **aceito** em 2026-10-22, por delegação do mantenedor, com as escolhas
  abaixo; revisável a pedido dele. Proposto em 2026-10-21.
- Base: 57c8a03
- Relacionadas: 0008/0039 (memória), 0031/0044 (sandbox bwrap/prlimit), 0037, 0047,
  0048; `WS§7`, `WS§9`, `WS§10`; Etapas 2.6, 4.7, 5R.3, 5R.4, 9.1, 15.1/15.8.

## Contexto

O produto precisa de gráficos, tabelas, grafos de claims, fórmulas, contramodelos e
documentos exportáveis (LaTeX/PDF), salvos em projetos e recuperáveis depois. Uma
imagem decorativa não é reproduzível; um componente que executa código do LLM é uma
porta de XSS e exfiltração; um PDF com "selo" desenhado pela interface contradiz P7.

## Decisão proposta

1. **Dados + especificação + receita (com execução isolada).** Um artefato persistido
   guarda referências por hash aos dados originais, à transformação/receita, à
   especificação declarativa e à versão do renderer. Quando há receita/transformação
   sobre `Dataset`, ela roda num executor isolado em sandbox (`bwrap`/`prlimit` sem
   rede, padrão 4.5/10.2), vinculando `(h(dados), h(receita)) → h(saída)` fora da TCB.
   SVG/PNG/PDF são **snapshots derivados**, nunca a única cópia.
2. **Lista fechada de componentes.** O contrato de visualização (`spec/visualization.md`,
   a criar em WS.5) enumera componentes suportados; a especificação carrega dados e
   parâmetros, não código. JavaScript vindo do LLM ou de documentos nunca roda como
   componente confiável.
3. **Sem floats canônicos.** Registros canônicos (canon/v1) não ganham floats: dados
   numéricos ficam em blobs originais; parâmetros usam inteiros ou decimais textuais
   com escala declarada, até que uma spec específica decida outra coisa com vetores.
4. **Status consultado, obsolescência explícita e extensão monotônica.** Projeções
   ligadas a claims, dados ou `h(V)` exibem congelada/atualizada/stale/irrecuperável
   (`WS§9.4`), como os cartões da 5R.4. Nada é recalculado silenciosamente. Para não
   invalidar cartões e projeções a cada símbolo novo adicionado numa sessão exploratória
   (5R.2), distingue-se extensão monotônica de vocabulário (apenas novos símbolos, sem
   alterar os `used_symbols` da fórmula) de mudança incompatível (coerente com 15.7).
5. **Documentos.** Fonte estruturada e editável → formato de composição → PDF
   derivado. Enunciados de claims vêm do renderizador da 9.1 (quando existir) ou da
   AIR impressa, com `ledger_ref`; texto do LLM marcado. O PDF registra o status no
   momento da geração; retratação posterior marca o relatório como desatualizado no
   projeto sem alterar bytes antigos. Nenhum selo visual; o nome `Seal` é reservado
   à 15.1.
6. **Compilação isolada.** Compilador em sandbox no padrão bwrap/prlimit já usado
   (4.5/10.2): sem rede, sem chaves, sem shell-escape, só inputs declarados, com
   limites de tempo/RSS/saída.

## Escolhas pendentes (WS.0), com critérios

| Escolha | Candidatos | Critérios |
| --- | --- | --- |
| Gráficos | Vega-Lite (spec JSON declarativa), ECharts, outras | spec declarativa sem funções embutidas; compatibilidade com CSP sem `unsafe-eval` (a verificar para cada candidato, incluindo modos/intérpretes CSP); dados separados da spec; acessibilidade; licença |
| Grafos | Cytoscape.js, alternativas | limites de nós/arestas, layout determinístico ou registrado, teclado |
| Matemática | KaTeX, MathJax | sem execução de macros arbitrárias; cobertura; CSP; desempenho |
| Documento | Typst, LaTeX (TeX Live), outros | sandbox, ausência de shell-escape, tamanho da distribuição, reprodutibilidade, qualidade tipográfica, bibliografia |

## Consequências

- TCB: inalterada; renderers e compiladores são não confiáveis.
- Persistência depende de WS.3/WS.4 (e, portanto, de MEM.0/MEM.1).
- Árvores de prova legíveis só existem para formatos com estrutura (Alethe 14.x,
  Lean 20.x); para LRAT, a UI mostra premissas `used` e o DAG de claims.

## Limites

Uma visualização fiel aos dados não prova que os dados representam o fenômeno;
hash/assinatura garante identidade dos bytes, não metodologia (L1). Nada implementado.

## Decisão de bibliotecas e formato (2026-10-22, por delegação)

| Escolha | Decisão | Motivo | Reverter se |
| --- | --- | --- | --- |
| Gráficos | **Vega-Lite**, num subconjunto fechado de marks/encodings definido em `spec/visualization.md` (WS.5), renderizado com o interpretador de expressões compatível com CSP do Vega | A spec é JSON declarativa com dados separados: casa com "dados + especificação + receita". ECharts aceita funções JavaScript em opções (formatters), o que reabre TW1 | o modo compatível com CSP não cobrir o subconjunto necessário (verificar na WS.5) |
| Grafos | **Cytoscape.js** com layout determinístico (`breadthfirst`/`dagre`) e limites de nós/arestas | Maduro, sem `eval`, navegação por teclado possível; layout registrável | desempenho com os limites da WS.5 |
| Matemática | **KaTeX** com `trust: false`, `strict` e `maxExpand` limitado | Rápido, sem macros que executem código; MathJax é mais pesado. Leitura apenas: nunca substitui `render_pt` em assinatura (W3) | faltar cobertura de notação necessária |
| Documentos | **Typst** para PDF, em sandbox bwrap/prlimit, sem rede e sem pacotes baixados; exportação da **fonte LaTeX** como texto, sem compilá-la no LOGOS | Typst é um binário só, sem shell-escape e mais fácil de isolar e fixar por hash; TeX Live é grande e tem `\write18`. Quem precisar de LaTeX recebe a fonte | a qualidade tipográfica ou a bibliografia não atenderem os templates da WS.7 |

Licenças (MIT, BSD-3, Apache-2.0) cabem na allowlist do `deny.toml`. Versões e
hashes fixados entram nos planos da WS.5/WS.7.
