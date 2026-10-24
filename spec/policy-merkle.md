# Árvore Merkle de política e provas (Etapa 7.2, ADR 0070)

A política publicada é comprometida por uma raiz `r_v`. A árvore é **ordenada** por chave de
indexação, o que permite provar tanto que uma regra existe quanto que **nenhuma regra de uma
classe casa com uma ação**.

## 1. Chave de indexação e folhas

`key(regra) = (classe, símbolo)`, com `classe ∈ {0 allow, 1 deny, 2 require, 3 derive}`:

| classe | símbolo |
| --- | --- |
| `allow`, `deny` | símbolo de função do termo de ação da cabeça (`Allowed(drop(x))` → `drop`); vazio se o argumento é variável (casa com qualquer ação) |
| `derive` | nome do predicado definido |
| `require` | sempre vazio |

`h(regra) = H("logos/rule/v1\0" ‖ canon(Rule))`. Folha: `Leaf = [classe_u64, símbolo_Text, h(regra)]`,
ordenada por `(classe, bytes do símbolo, h(regra))`; `digest(folha) = H("logos/merkle-leaf/v1\0" ‖
canon(Leaf))`.

## 2. Árvore

RFC 9162 §2.1: `MTH([]) = H("logos/merkle-empty/v1\0")`, `MTH([d]) = d`, e para `n ≥ 2`,
`MTH(D) = H("logos/merkle-node/v1\0" ‖ MTH(D[0:k]) ‖ MTH(D[k:n]))` com `k` a maior potência de 2
estritamente menor que `n`. A raiz é `r_v`. Os quatro domínios de hash são reservados em
`spec/domains.md`.

## 3. Provas

- **Inclusão** `InclusionProof = [índice, tamanho, [irmãos…]]` (até 64 irmãos): caminho de
  auditoria RFC 9162 §2.1.3; verificação pelo algoritmo do §2.1.3.2, recusando `índice ≥ tamanho`
  (sem essa guarda, em 2 folhas o índice 2 imitaria o 0). O tamanho não é autenticado sozinho (uma
  prova de 5 folhas pode passar como de 6 se a cadeia de hashes coincide), mas isso nunca esconde
  uma folha existente: a cadeia precisa reproduzir `r_v`.
- **Não-pertinência de uma chave** `NonMembership = [tamanho, esquerda, direita]`, cada lado
  `[] | [[Leaf, InclusionProof]]`: mostra as duas folhas **adjacentes** que cercam a chave (ou a
  borda da árvore). Vale se ambas incluem sob `r_v` com o mesmo tamanho, os índices são
  consecutivos (ou `0` / `tamanho-1` na borda), `esquerda.chave < chave < direita.chave`
  (comparação estrita, só pela chave). Folhas com a mesma chave ficam juntas na ordem, então
  nenhum par adjacente pode cercar uma chave que existe. Árvore vazia: `[0, [], []]` contra
  `H(merkle-empty)`.
- **`NoRule(classe, a, v)`** `NoRuleProof = [específica, curinga]`: não-pertinência de
  `(classe, símbolo(a))` e de `(classe, vazio)`; para ação sem símbolo, só a do curinga. É a
  premissa que a 7.3 consome para "não existe regra `deny` que case com `a`".

## 4. Publicação

`PolicyPublished` e o `EpochAdvanced` associado: `spec/events.md` §15. A raiz vigente é consultada
no índice (`current_policy`). Uma política nova (versão maior) muda `r_v`; provas antigas deixam de
verificar contra a raiz nova.

## 5. Vetores

`spec/vectors/policy/policy.json` (`merkle`): para 4 políticas, a raiz, as folhas ordenadas, o
caminho de cada folha e, para cada chave ausente de um universo fixo, os índices das vizinhas e a
prova canônica; tudo calculado pelo gerador Python independente.
