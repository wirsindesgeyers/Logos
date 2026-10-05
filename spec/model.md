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
