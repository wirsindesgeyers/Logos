# model/v1 — estruturas finitas

Normativo para ROADMAP 2.1; ADR [0014](../docs/adr/0014-estruturas-finitas.md).
`MUST`/`MUST NOT` no sentido RFC 2119. Pré-requisitos: [canon/v1](encoding.md),
[vocabulary/v1](vocabulary.md) e [air/v1](air.md).

## 1. Contexto e identidade

Modelo MUST referir-se a Vocabulary validado por digest h(V) exato, calculado
como SHA-256(`logos/vocab/v1\0` || canon(V)). Decoder MUST receber V e comparar
esse digest; não é suficiente confiar em hash fornecido separadamente pelo agente.

Sort ID = posição zero-based em V.sorts; Symbol ID = posição zero-based em
V.symbols. IDs MUST ser unsigned u32. Não são nomes nem identidades globais;
a mesma posição em outro V não é intercambiável. Ordem de V participa de h(V).

Modelo válido é estrutura finita, não prova de `M ⊨ φ`, consistência ou refutação.
Não autentica atestadores, domínio fechado ou qualquer referência de V.

## 2. Domínios e interpretações

Domains MUST conter exatamente uma cardinalidade u32 para cada sort, na ordem
original. Cardinalidades MUST ser positivas; elementos são 0..n−1. Open aceita
qualquer cardinalidade positiva dentro do limite.

Enumerated MUST ter cardinalidade exatamente igual ao número de constantes
listadas. Constante na posição i da enumeração denota elemento i; uma constante
primitiva listada MUST ter interpretação i. Isso fixa representantes distintos e
cobre todo domínio. Outras constantes desse sort podem denotar qualquer elemento.
Enumeração vazia não tem modelo clássico não vazio. Constantes Defined listadas
não ganham tabela: têm rótulo ordinal intrínseco; a avaliação contextual
([evaluation.md](evaluation.md), 2.2) verifica se a expansão respeita esse
rótulo. Estrutura isolada não demonstra tal compatibilidade.

Assinaturas MUST ser semanticamente válidas, inclusive Defined:

- Const: aridade vazia e resultado Sort declarado.
- Func: resultado Sort declarado; argumentos são sorts declarados.
- Pred: resultado Bool; argumentos são sorts declarados.

Cada símbolo primitivo (Anchored/Glossed) MUST ter exatamente uma interpretação
na categoria correta. Defined MUST NOT ter entradas de interpretação. Em cada
categoria IDs MUST ser estritamente crescentes: repetição, ordem invertida,
ID inexistente, categoria errada e interpretação ausente são erros; nada é
ordenado, truncado ou deduplicado silenciosamente.

Função MUST fornecer um output para cada tupla do produto cartesiano de seus
sorts de entrada. Tabela densa em ordem lexicográfica dos inputs, último eixo
variando mais rápido. Para tamanhos (a,b), índice de (i,j) = i*b+j. Todos outputs
MUST ser menores que cardinalidade do sort de resultado. Func nullary tem um
output (produto vazio = 1), não zero outputs.

Predicado é conjunto de tuplas verdadeiras, cada uma com comprimento exatamente
igual à aridade. Cada coordenada MUST pertencer ao respectivo sort de entrada.
Tuplas MUST estar em ordem lexicográfica estritamente crescente. Ausência de uma
tupla significa falso. Pred nullary: `[]` = falso, `[[]]` = verdadeiro.

## 3. Envelope e hash

CBOR canon/v1 estrito, um array de comprimento 6:

```text
[1, bytes32_hV, domains, consts, funcs, preds]
domains = [unsigned_u32, ...]
consts = [[symbol_id_u32, element_u32], ...]
funcs = [[symbol_id_u32, outputs_bytes], ...]
preds = [[symbol_id_u32, [[element_u32, ...], ...]], ...]
```

`outputs_bytes` MUST ser concatenação de outputs u32 big-endian de largura fixa
4 bytes. Comprimento MUST ser 4 vezes produto de cardinalidades de entrada.
Exemplo: outputs `[1,0]` são bytes `0000000100000000`, não array CBOR de inteiros.
Bytes vazios não são função total de nenhum domínio admitido.

Versão MUST ser 1. Tipo/layout incorreto, fields extras, versão desconhecida,
inteiros fora de u32, CBOR não mínimo, indefinite, float/tag, truncamento e bytes
sobrando MUST ser recusados. Nenhuma versão anterior de formato muda.

`h(M) = SHA-256("logos/model/v1\0" || canon(M))`.
Modelos que diferem em qualquer campo têm bytes distintos; limites locais não
participam do wire/hash. Vetores fixos incluem bytes de V, bytes de M e digest
independente, para reproduzir sem depender da implementação Rust.

## 4. Limites e erros

`ModelLimits` inclusivos, padrão:

| Campo | Padrão | Mede |
| --- | ---: | --- |
| max_bytes | 1.048.576 | Tamanho exato do envelope model/v1 |
| max_domain_size | 65.536 | Cardinalidade de cada sort |
| max_cells | 100.000 | Constantes + outputs funcionais + células de tuplas |

Cada tupla de predicado conta `max(1, aridade)` células; tupla nullary também
custa 1. Constante custa 1; função custa produto esperado de entrada, mesmo se
tabela recebida for parcial. Domínios não são materializados em arrays de elementos.
Produtos/somas MUST usar aritmética checada. Excesso ou overflow MUST produzir
`ModelError::Unknown(Exhausted::Size)`, nunca modelo parcial ou falso lógico.
Falha de reserva tipada MUST produzir Unknown(Memory).

Decoder MUST conferir bytes antes de chamar Decoder canônico existente. Esse
parser genérico materializa Value: seu custo é limitado por bytes e MAX_DEPTH,
não é teto rígido de RSS. Produtos funcionais/células MUST ser verificados antes
de alocar tabelas tipadas. Encoder e construtor MUST conferir tamanho exato do
wire e limites de células/domínios. Limite zero aceita apenas objetos que não
consomem aquele recurso; modelo sem sorts/símbolos ainda custa bytes de envelope.

Erros estruturais são ModelError: VocabularyMismatch, InvalidSignature,
InvalidDomains, InvalidInterpretation, InvalidEnumeration, NonTotalFunction,
ValueOutOfDomain, InvalidTuple ou Canon. Unknown preserva causa Size/Memory nas
APIs contextualizadas; Canon não produz bytes válidos no lugar de erro.
Não existe conversão implícita para CheckOutcome::Accepted ou Verdict.

## 5. API e reprodução

```rust,ignore
FiniteModel::new(data, &vocabulary, limits)?;
FiniteModel::decode_with_limits(&wire, &vocabulary, limits)?;
model.encode_with_limits(limits)?;
model.data(); // somente leitura
```

Não implementar CanonDecode sem Vocabulary contextual. Canon/Domain permitem
codificar/hashear estrutura já validada; construtor mantém campos privados.

Vetores: [vectors/model/vectors.json](vectors/model/vectors.json).
Gerador independente stdlib: `tools/gen_model_vectors.py --check`.
Smoke público: `cargo run -p logos-model --example model_smoke`.
Fuzz: `FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 model_decode`.
Avaliação/expansão Defined é especificada em [evaluation.md](evaluation.md);
solver e ledger permanecem fora desta etapa de estruturas.

## 6. Produtor não confiável (Etapa 2.4)

`logos_provers.z3_models.refute(O_bytes, V_bytes)` traduz Γ ∧ ¬goal para
SMT-LIB; `witness(O_bytes, V_bytes)` traduz somente Γ. Ambos retornam bytes
canônicos model/v1 ou None. Nenhum retorno é CheckOutcome, Refuted ou Verified.
`Limits(max_domain=4, timeout_ms=5000)` limita cada domínio e compartilha tempo
entre consultas; None inclui busca finita sem sat, unknown e timeout, não prova
insatisfatibilidade geral. Sem hard RSS/isolamento físico de timeout.

Enumerated fixa cardinalidade/ordinais; Open busca todos vetores 1..k. Defined
é globalmente restringido como predicado ou relação total com resultado único,
inclusive símbolos não usados; somente primitivos recebem tabelas model/v1.
Contexto, posições e ordens continuam §§1–3; nenhum formato/domínio/versionamento
é alterado. Cada candidato MUST passar pelo checker contextual real contra os
bytes/hash esperado de O fornecidos pelo monitor antes de ganhar autoridade.

Reprodução: em `py/`, `uv run python -m logos_provers.z3_models_smoke` e
`uv run pytest -q -s tests/test_z3_models.py`. Decisões em
[ADR 0017](../docs/adr/0017-produtor-z3-de-modelos.md).

## 7. Produtor exaustivo não confiável (Etapa 2.5)

`logos_provers.model_finder.refute(O_bytes, V_bytes, limits=...)` enumera
cardinalidades Open 1..k e interpretações primitivas, com k entre 1 e 4.
Enum fixa cardinalidade/ordinais. Funções e predicados seguem §§2–3; Defined
é derivado em ordem de dependência e checado globalmente conforme evaluation.md,
inclusive totalidade/unicidade e ordinal, mas não recebe tabelas no certificado.

`Limits(max_domain=4, max_steps=1_000_000, timeout_ms=5000)` impõe limite
cooperativo de trabalho/tempo e preserva caps padrão de saída do checker.
`SearchResult.status` distingue FOUND (model bytes), NO_MODEL_WITHIN_BOUND
(enumerou completamente o espaço autorizado), EXHAUSTED (tempo/trabalho/tamanho
insuficiente). Apenas FOUND contém bytes. Nenhum status é Verdict ou prova de
validade; NO_MODEL_WITHIN_BOUND não significa ausência em domínios maiores.

`logos_provers.model_fallback.refute` tenta Z3 e, se recebe None, tenta
enumeração; retorna somente candidato bytes/None. Portanto unknown nativo
aciona fallback, mas timeout/unsat finito também podem acioná-lo: API 2.4 não
distingue causa. Timeout por produtor, não SLA conjunto. Erros não viram None.
`model_context.ProducerError` e validação AIR são compartilhados, não reexportados
por Z3; avaliação/busca não usa Z3. Sem novo formato, dependência ou autoridade.

Reprodução em `py/`: `uv run python -m logos_provers.model_finder_smoke`,
`uv run pytest -q -s tests/test_model_finder.py tests/test_model_fallback.py`.
[ADR 0018](../docs/adr/0018-model-finder-exaustivo.md).


## 8. Apresentação não autoritativa (Etapa 2.6)

`logos_provers.model_explain.explain_model(M_bytes, V_bytes, O_bytes) -> str`
apresenta domínio, valores/fatos relevantes, valoração das premissas e objetivo
AIR, junto de h(M). Usa `gloss_pt` entre aspas escapadas; sem glosa, usa nome.
Elementos são `Sort#ordinal`, sem introduzir representantes Enum ausentes de O.

Somente símbolos/sorts sintáticos de O e suas assinaturas são apresentados;
Defined é avaliado internamente sem exibir dependências ou expandir fórmulas.
Objetivo verdadeiro e premissa falsa permanecem visíveis, não são convertidos
em refutação. Texto não atesta aceitação, proveniência ou fidelidade.

Entradas inválidas e exaustão da avaliação produzem `ProducerError`. Apresentação
usa até 100 entradas de fatos/valores com omissão explícita, domínios compactos
e orçamento cooperativo 1M passos/5s. Além dos caps de model/v1, a representação
densa do intérprete é limitada a 100k células agregadas, inclusive predicados e
Defined; um certificado esparso válido pode exceder esse limite de apresentação.
Limite de apresentação não muda validade do modelo ou resultado do checker.

Reprodução em `py/`: `uv run python -m logos_provers.model_explain_smoke`
confere 20 fixtures de refutação pelo checker Rust real e imprime exemplo PT;
`uv run pytest -q -s tests/test_model_explain.py` cobre relevância e valoração.
Corpus público em `logos_provers.model_explain_examples.corpus_20`.
[ADR 0019](../docs/adr/0019-apresentacao-de-contramodelos.md) e
[plano 2.6](../docs/plans/etapa-2.6.md). Nenhuma mudança no formato model/v1.
