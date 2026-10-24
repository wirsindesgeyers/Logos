# Atestador `ingest`

Atesta a **proveniência dos bytes** de um conteúdo externo (e-mail, web, documento), nunca o seu significado, e fornece o levantamento `Says` usado pelas extrações (6.6).

- **AttestorId:** `ingest`
- **Procedimentos:** `ingest/content-origin/v1` (fatos de ingestão) e `says-lift/v1` (símbolos `says_*`)
- **Tópico (trust/v1):** `ingestion` (fatos); `extraction` é o tópico do extrator que registra `Extracted`

## Símbolos ancorados

- `content_of(fonte, digest)`: o conteúdo da fonte tem este SHA-256.
- `origin_is(fonte, origem)`: a fonte é `origin_external`, `origin_internal` ou `origin_user`.
- Constantes: a fonte (sort `Source`), o digest (`sha256:<hex>`, sort `Digest`) e as três origens.
- `says_P(fonte, a…)`: para cada predicado `P(a…)` do vocabulário, o símbolo distinto "a fonte diz P(a…)". Não implica `P`.

## Procedimento

1. Lê o arquivo de conteúdo (arquivo regular até 64 MiB; link simbólico é recusado) e calcula o SHA-256.
2. Afirma `content_of(fonte, digest) ∧ origin_is(fonte, origem)`; a versão registrada é `fonte` → `sha256=<hex>;size=<n>`.
3. `says_lift` declara `says_P` com a assinatura de `P` precedida do sort `Source` e grounding `Anchored(ingest, says-lift/v1)`.
4. O extrator (Fase 11) propõe literais `says_P(fonte, …)` que o kernel registra como `Extracted { fonte, h(conteúdo), φ, extrator }`: premissa de confiança, assumption de escopo `session:extracted`, nunca `Fact`.

## Limites

O atestador não lê o conteúdo: não diz se o e-mail é verdadeiro nem o que ele pede. `Says_P(s, x)` só vira `P(x)` por uma regra de política explícita (7.x). O vínculo entre o `h(conteúdo)` do `Extracted` e o fato de ingestão é conferido pelo consumidor/auditor, não pelo kernel nesta etapa.

O operador precisa conceder `Trusted(emissor, ingestion)` ao atestador de ingestão e `Trusted(extrator, extraction)` ao extrator (6.2).
