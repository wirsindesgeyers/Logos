# ADR 0068 — `Says` por levantamento de predicados e o evento `Extracted`

- Estado: aceito para a Etapa 6.6 (2026-10-24), por delegação do mantenedor. Revisável.
- Relacionadas: 0061 (fact/v1), 0063 (confiança por tópico), 0054 (escopos de assumption),
  0065 (SDK de atestador); `spec/says.md`, `spec/events.md` §14, `docs/attesters/ingest.md`.
- Diverge do roadmap 6.6, que pede "operador `Says(source, φ)` no AIR"; ver Decisão 1.

## Contexto

A 6.6 prepara a Fase 11: conteúdo de e-mail, web e documentos entra como premissa atribuída,
nunca como fato. O roadmap pede (a) o operador `Says(source, φ)` com `source` de sort `Source`,
(b) um atestador de ingestão e (c) o evento `Extracted { source, h(c), φ, extractor }`. O AIR
(FOL de várias sortes) não tem fórmula como termo, e um operador novo mudaria `air/v1`, o
sort-checker, o grounder, o CNF, o avaliador de modelos, o parser/printer e os geradores
Python — cinco crates da TCB e todos os vetores — para um benefício que a Fase 11 ainda não usa.

## Decisão

1. **`Says` por levantamento.** Para cada predicado `P(a…)` o vocabulário pode declarar
   `says_P(Source, a…)`, `Anchored(ingest, says-lift/v1)`. É um símbolo distinto de `P`, então
   a separação "dizer ≠ ser" vale sem nenhuma regra nova na TCB: nenhuma obrigação que exige
   `P(x)` é satisfeita por `says_P(s, x)`. Só literais (átomo ou negação) se levantam. O operador
   geral (fórmulas compostas, aninhamento, `Says(A, φ)` entre agentes da Fase 17) fica para quando
   houver consumidor; a migração é aditiva (novo nó de fórmula, `air/v2`).
2. **`Extracted` é um evento próprio (tag 19)**, `[1,19,hV,source,content,Formula,extractor]`, e
   não uma `AssumptionDeclared` com escopo mágico: o ledger registra fonte, hash do conteúdo e
   extrator de forma auditável (o auditor independente o decodifica).
3. **Projeção como assumption de escopo fixo `session:extracted`.** Assim `Extracted` não é
   `Fact` (não serve de fonte de sort enumerado, não passa por `Trusted` de fato), é citável,
   aparece em `used`, torna condicional toda conclusão que a usa (`open_assumptions`), é
   retratável pelo extrator ou pelo admin e a cascata ATMS derruba o dependente. Todas as
   extrações compartilham o escopo; misturá-lo com outro escopo de assumption é recusado pelo
   gerador ("mixed assumption scopes"), o que é o comportamento conservador desejado.
4. **Quem pode registrar.** `extracted::register` exige `Trusted(extrator, "extraction")` vigente
   e extrator não revogado, vocabulário no CAS, fórmula bem tipada e literal `Says`-lifted
   fechado cujo primeiro argumento é a constante de sort `Source` de nome `source`. Uma
   extração não consegue, portanto, introduzir `P(x)` puro.
5. **A identidade do extrator é a que o kernel autenticou** (conexão/UID, na Fase 11); `Extracted`
   não carrega assinatura própria do extrator, diferente do `Fact`.
6. **Atestador de ingestão (`ingest`, 7º de referência, tópico `ingestion`)** atesta
   `content_of(s, h(c)) ∧ origin_is(s, origem)`: proveniência dos bytes, não significado.

## Alternativas consideradas

1. **Novo nó `Says` em AIR** (adiada, ver acima).
2. **`Says` como predicado sobre hash da fórmula** (`says(s, h(φ))`): perde as variáveis livres
   que as regras de política usam (`Says(e, Pedido(x))`) e impede o grounder de relacionar
   `φ` com os átomos.
3. **Novo `EntryKind::Extracted` com tabela própria** (rejeitada por ora): exigiria índice v7 e
   tocaria geração, servidor e política só para obter o que o escopo fixo já dá; reavaliar na 7.x
   se a política precisar distinguir a origem sem olhar o evento.

## Consequências

- Formato: um corpo de evento novo (tag 19), vetor independente em Python
  (`spec/vectors/events`), decoder do auditor atualizado; nenhuma mudança de esquema do índice.
- Riscos: o vínculo entre o `h(conteúdo)` da extração e o fato de ingestão não é conferido pelo
  kernel (cabe ao consumidor/auditor, e à Fase 11 ligar os dois); extrações de fontes diferentes
  compartilham escopo; `Says` composto não existe até a migração para o operador geral.
