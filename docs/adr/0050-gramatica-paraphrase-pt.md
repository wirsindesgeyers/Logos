# ADR 0050 — Gramática paraphrase-pt/v1 e injetividade por releitura

- Estado: aceito para implementação da Etapa 9.1 (2026-10-21), na sequência da
  autorização do mantenedor para seguir o próximo passo.
- Base: 48f5cec
- Relacionadas: 0007 (AIR), 0011 (superfície), 0046 (templates vocabulary/v2);
  [`spec/paraphrase-pt.md`](../../spec/paraphrase-pt.md).

## Contexto

A 9.1 exige `render_pt(φ, V)` e `parse_pt(String, V)` na TCB de fidelidade, com
`parse_pt(render_pt(φ)) == φ` para toda `φ`, sem ambiguidade de escopo e com
versão no hash. A 9.1a forneceu templates por símbolo; o ADR 0046 deixou dois
limites para esta etapa: templates de símbolos distintos podem coincidir, e o
texto livre dos templates pode imitar a estrutura (conectivos, parênteses,
"verdadeiro").

## Decisão

1. **Onde.** `logos-seal`, propriedade fidelidade, como o §2 do roadmap previa
   ("selo de cinco campos + renderizador"). Dependências só do workspace (air,
   canon, crypto); nenhuma dependência externa.
2. **Estrutura fora dos templates.** Toda subfórmula de conectivo fica entre
   parênteses; todo átomo e todo argumento de termo fica entre `⟦ ⟧`, que os
   templates não podem conter (ADR 0046). O texto dos templates nunca é lido
   como estrutura, e as frases fixas (conectivos, `verdadeiro`, `o valor …`)
   nunca começam com `⟦`. Prefere-se redundância a elegância, como pede o roadmap.
3. **Símbolo identificado pelo texto.** O parser identifica o símbolo pela lista
   de segmentos literais do template. O construtor `Paraphraser::new` recusa
   vocabulários em que dois símbolos do mesmo contexto têm a mesma lista, em que
   uma constante tem forma de variável (`x7`) ou em que um sort contém `⟦`/`⟧`.
   Não se infere símbolo por sorts nem por semelhança de texto.
4. **Variáveis por nível.** Binder no nível `k` é `xk`; a forma é redundante com o
   De Bruijn e conferida na leitura.
5. **Releitura obrigatória.** `render_pt` só devolve o texto se `parse_pt` o ler de
   volta como a mesma fórmula; caso contrário, erro `RoundTripMismatch`. Isso dá
   injetividade por construção (render e parse são funções), sem depender de
   provar a ausência de ambiguidade para todo vocabulário.
6. **Escopo de entrada.** Só fórmulas fechadas e bem tipadas são renderizadas; o
   parser devolve fórmula sort-checada. Limites: os de air/v1 e 1 MiB de texto.
7. **Hash.** `paraphrase/v1 = [1, gramática, versão do renderizador, h(V), texto]`
   no domínio reservado `logos/paraphrase/v1`.

## Alternativas consideradas

1. **Átomos sem delimitador, desambiguação por precedência** (rejeitada): templates
   como `verdadeiro` ou `se {0}, então {1}` colidiriam com conectivos; a
   correção dependeria do conteúdo de cada vocabulário.
2. **Desambiguar templates iguais pelos sorts dos argumentos** (rejeitada): torna a
   leitura dependente de inferência de tipos e quebra com variáveis; recusa
   explícita é mais simples e auditável.
3. **Só checagem estática, sem releitura** (rejeitada): a análise do §1 da spec
   não é prova mecanizada; a releitura custa uma passada e fecha a lacuna.
4. **Gramática em formato de dados executado** (rejeitada): mais código de TCB e
   um interpretador para fuzzar; as frases fixas são poucas.

## Consequências

- TCB de fidelidade cresce (`logos-seal`); o manifesto lista os novos arquivos e o
  fuzz `paraphrase_parse` cobre o parser.
- Vocabulários com templates repetidos no mesmo contexto não podem ser
  parafraseados até serem corrigidos (erro explícito, nunca texto ambíguo).
- O texto é longo e mecânico; legibilidade é avaliada na revisão humana.
- 9.2 pode reler a string assinada com `parse_pt` e compará-la com `a`.

## Limites

Ida e volta e injetividade não garantem que o template diga o que o símbolo
significa nem que o leitor entenda a frase: isso é revisão humana (critério 3 da
9.1) e permanece L1. Confusáveis Unicode dentro de templates não são tratados.
