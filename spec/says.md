# Says e Extracted — conteúdo externo atribuído (Etapa 6.6, ADR 0068)

Dados de e-mail, web e documentos entram no sistema como **premissas atribuídas**, nunca como
fatos (§13). Esta especificação fixa a forma lógica e o registro; extrair `φ` do texto é da Fase 11.

## 1. Forma lógica: `Says` levantado

AIR/v1 (FOL de várias sortes) não tem fórmula como termo, então `Says(s, φ)` é realizado pelo
**levantamento de predicados**: para cada predicado `P(a₁…aₙ)` do vocabulário, o vocabulário pode
declarar `says_P(Source, a₁…aₙ)`, com:

- assinatura `Source × a₁ … aₙ → Bool`, sort `Source` como primeiro argumento;
- grounding `Anchored(ingest, says-lift/v1)`;
- nenhuma relação lógica com `P`: `says_P(s, x)` não implica `P(x)` e vice-versa.

Como `says_P` e `P` são símbolos distintos, nenhuma obrigação que exige `P(x)` é satisfeita por
`says_P(s, x)`. A ponte é sempre uma premissa explícita, a regra de política (7.x), por exemplo
`says_Pedido(e, x) ∧ Externo(e) → Pedido(x)`, que aparece em `used` ao lado da extração.

Limite: só literais atômicos (átomo ou negação) são levantados; `Says` sobre fórmulas compostas ou
aninhadas, e `Says(A, φ)` entre agentes (Fase 17), exigem um operador próprio e ficam para quando
forem necessários (ADR 0068).

## 2. Ingestão (`ingest`, tópico `ingestion`)

Fatos fact/v1 que atestam **bytes**, não significado: `content_of(s, h(c))` (digest SHA-256 em
constante `sha256:<hex>` do sort `Digest`) e `origin_is(s, origem)` com `origem` em
`origin_external | origin_internal | origin_user`. Procedimento em `docs/attesters/ingest.md`.

## 3. `Extracted` (evento tag 19)

Corpo em `spec/events.md` §14. Regras do kernel (`extracted::register`):

1. extrator com `Trusted(extrator, "extraction")` vigente e não revogado (`IssuerRevoked`);
2. vocabulário no CAS e fórmula bem tipada nele;
3. fórmula é `says_P(fonte, t…)` ou `¬says_P(fonte, t…)`: predicado declarado `Anchored(ingest,
   says-lift/v1)`, primeiro argumento a constante de sort `Source` cujo nome é `source`, demais
   termos fechados;
4. recusa não grava evento.

Efeitos: a entrada é projetada como assumption (status 5) de escopo `session:extracted`; a conclusão
que a cita fica condicional a ela (`open_assumptions`), `used` a mostra e a retratação (do extrator
ou do admin) invalida o que dependia dela pela cascata ATMS. O vínculo entre o `h(conteúdo)` do
evento e o fato de ingestão é conferido por quem consome (auditor, política), não pelo kernel.

## 4. Fora do escopo

Extração por LLM em quarentena (11.1); `Says` composto ou aninhado; semântica de endosso (7.1).
