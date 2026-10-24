# ADR 0069 — Linguagem de política (policy/v1)

- Estado: aceito para a Etapa 7.1 (2026-10-24), por delegação do mantenedor. Revisável.
- Relacionadas: 0068 (`Says` levantado), 0011 (sintaxe de superfície fora da TCB), 0010 (sort-check),
  0065; `spec/policy.md`, `ROADMAP.md` 7.1–7.3.

## Contexto

A 7.1 pede política como conjunto de regras AIR numa forma restrita (Horn com negação
estratificada) e FOL geral para o que exige certificado; cada regra com `id`, `class` e
comentário; sintaxe de superfície; semântica deny-overrides; parser + sort-check com 100% de
cobertura de ramos; recusa de ciclo pela negação.

## Decisão

1. **Objeto `policy/v1`** `[1, version, hV, rules]`, regras ordenadas por `id`,
   `Rule = [id, class, Formula, comment]`; tag de hash `logos/policy/v1\0` (já reservada). O
   kernel consome só os bytes canônicos, revalidados contra o vocabulário (`Policy::decode`).
2. **Quatro classes**: `allow`/`deny` (cabeça `Allowed` / `¬Allowed`), `require` (FOL geral sobre
   `Allowed`, para a rota L2) e `derive` (predicados auxiliares, acréscimo ao roadmap: sem ele a
   estratificação teria um único predicado e o critério "ciclo pela negação" seria trivial).
3. **Horn restrito** inclui blocos negados `¬∃ȳ.(conjunção positiva)`, porque o exemplo r9 do
   roadmap os usa; negação dupla, `∨`, `↔`, `→` aninhado e `∀` no corpo ficam fora.
4. **Deny-overrides na estratificação**: nós `AllowFires`/`DenyFires` e arestas
   `Allowed → AllowFires` (+) e `Allowed → DenyFires` (−); componentes por Kosaraju iterativo
   (sem recursão na TCB).
5. **Sintaxe de superfície em `logos-policy-text`, fora da TCB**, reutilizando o parser de fórmulas
   `air-text`. O roadmap lista "parser" como TCB; divergência deliberada, como na 1.4: o texto
   nunca é autoridade e o kernel nunca o lê. O critério de cobertura é aplicado à validação
   (`logos-policy/src`: sort-check por regra, forma, estratificação, codec), por
   `ci/check-policy-coverage.sh`; o parser tem testes comportamentais, round-trip e diferencial
   contra os bytes do gerador Python.
6. **Cobertura**: 100% dos ramos LLVM (exigido); as regiões restantes são `?` de operações que não
   falham (hash e codificação de objetos já validados), registradas como informativas.
7. **Fora do escopo**, explicitamente: segurança por faixa (range restriction) e ordem de
   avaliação (7.3), árvore Merkle (7.2).

## Alternativas consideradas

1. **Parser dentro de `logos-policy`**: aumentaria a TCB (air-text, ~3 kLOC) para um benefício que
   o kernel não usa.
2. **Duas classes só (allow/deny)**: estratificação trivial e sem predicados auxiliares.
3. **Verificar segurança por faixa já na 7.1**: depende de como a 7.3 avalia (índices De Bruijn,
   ordem dos literais); melhor decidir lá, com o avaliador.

## Consequências

- Formato novo `policy/v1` com vetores independentes, fuzz `policy_decode` e job de cobertura na CI.
- `Says` entra nas regras como `says_P(s, x)` (símbolo ancorado ao levantamento), então uma regra de
  endosso é uma `allow`/`derive` comum cujo corpo cita `says_P`; nada de especial na TCB.
- Risco: regras `require` só são conferidas sintaticamente aqui; sua semântica depende da 7.3 e do
  certificado L2.
