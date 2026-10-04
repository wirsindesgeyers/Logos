# Sintaxe abstrata — air/v1

Especificação normativa da Etapa 1.1 para a sintaxe **FOL-ms-eq/v1**.
As palavras DEVE, NÃO DEVE e PODE expressam requisitos normativos. Uma
implementação independente DEVE produzir os mesmos bytes para a mesma sintaxe.
Este formato usa [canon/v1](encoding.md), sem modificar suas regras, e o domínio
de fórmula já reservado em [domains.md](domains.md). A decisão de representação
está em [ADR 0007](../docs/adr/0007-sintaxe-air.md).

## 1. Escopo e identidade

`air/v1` representa sorts, assinaturas de símbolos, termos e fórmulas. Não é
parser textual, vocabulário, certificado de boa formação semântica ou sort-check.
O decoder DEVE verificar formato, árvore prefixa e limites estruturais. NÃO DEVE
rejeitar uma árvore somente porque os símbolos não foram declarados, um índice
não encontra binder, os sorts divergem, o kind é inadequado ao uso, a assinatura
tem aridade diferente da aplicação ou o resultado não é o esperado. Esses
requisitos pertencem a etapas posteriores.

Sorts são identificados pelo conteúdo do nome. Símbolos são identificados por
**todos** os campos da assinatura: nome, kind, lista ordenada de sorts de entrada
e resultado. Nomes são texto canônico; nomes vazios são permitidos. Não existem
IDs de arena, tabela de símbolos oculta, referências externas ou interning no fio.
Um Sort ou Symbol embutido DEVE carregar sua representação completa.

Igualdade é estrutural, não equivalência lógica. O encoder NÃO DEVE ordenar
argumentos, aplicar associatividade/comutatividade, eliminar duplicatas, reduzir
conectivos, simplificar constantes ou eliminar quantificadores. Limites e
estatísticas auxiliares NÃO fazem parte da identidade nem são serializados.

## 2. Transporte, versão e envelopes

Um objeto completo é **exatamente um item CBOR** conforme canon/v1. Toda coleção
abaixo é um array de comprimento definido, em ordem; nenhum mapa é usado. Tags
AIR são inteiros em arrays, não tags CBOR de major type 6. Texto DEVE ser UTF-8
válido, NFC, composto de code points atribuídos em Unicode 16.0.0. Inteiros e
comprimentos DEVEM ter cabeçalhos mínimos. Não se admite normalização silenciosa,
inteiros negativos em campos unsigned, floats, tags CBOR ou arrays indefinidos.

Não há magic bytes, tamanho de frame, hash, limites ou bytes de padding no
objeto AIR. O consumidor estabelece o framing externo e seu teto de bytes antes
de decodificar. Uma API que recebe bytes de um objeto completo DEVE rejeitar
qualquer byte restante após o item, inclusive outro item CBOR válido. Envelopes
aninhados são valores completos, sem framing externo adicional.

O inteiro inicial `1` é a versão de **air/v1**, independente de canon/v1. O
segundo inteiro distingue os quatro tipos. Versão ou tipo inesperado DEVE ser
rejeitado, nunca inferido, ignorado ou traduzido para versão anterior.

| Tipo | Array exato | Aridade do envelope |
| --- | --- | --- |
| Sort | `[1, 0, name]` | 3 |
| Symbol | `[1, 1, name, kind, [Sort...], result]` | 6 |
| Term | `[1, 2, "FOL-ms-eq/v1", [TermNode...]]` | 4 |
| Formula | `[1, 3, "FOL-ms-eq/v1", [FormulaNode...]]` | 4 |

`name` e LogicId são strings de texto CBOR (major type 3), não bytes. A única
lógica suportada é o texto **exato e sensível a caixa** `FOL-ms-eq/v1`. Termos e
fórmulas, inclusive termos embutidos, DEVEM repetir esse LogicId. Outro texto é
lógica não suportada; campo de outro tipo é erro de formato. Alterar layout ou
regras de identidade exige nova versão normativa, não extensão tolerante de v1.

### 2.1. Assinaturas de símbolos

| Campo / variante | Representação |
| --- | --- |
| kind Const | `0` |
| kind Func | `1` |
| kind Pred | `2` |
| resultado Sort | `[0, Sort]` (aridade 2) |
| resultado Bool | `[1]` (aridade 1) |

A lista de entrada PODE ser vazia. Todas as combinações desses kinds, listas e
resultados são sintaxe representável: por exemplo, kind Pred com resultado Sort,
ou Const com lista não vazia, NÃO são rejeitados por este formato. Tags de kind
ou resultado diferentes das listadas são inválidas, assim como payload extra.

## 3. Termos e fórmulas em prefixo plano

Cada envelope Term/Formula contém um array **não vazio** de records em pré-ordem.
A raiz vem primeiro; os subtrees dos filhos vêm imediatamente após o pai, na
ordem original. Cada subtree ocupa um intervalo contíguo de records. Counts
informam filhos **diretos**, não quantidade de descendentes. Não há offsets,
back-references nem envelopes de subfórmulas.

### 3.1. Records de Term

| Variante | Record exato | Aridade do record | Filhos no prefixo |
| --- | --- | --- | --- |
| Var | `[0, index, Sort]` | 3 | 0 |
| App | `[1, Symbol, child_count]` | 3 | `child_count` |

`index` e `child_count` são inteiros unsigned CBOR de 64 bits, no intervalo
`0..=2^64-1`. A conversão de `child_count` para o tamanho usado pela implementação
DEVE ser verificada, sem truncamento. Uma constante PODE ser representada por
App de zero filhos, mas nenhum kind/assinatura obriga nem proíbe esse count na
etapa sintática.

### 3.2. Records de Formula

| Variante | Record exato | Aridade do record | Filhos de fórmula no prefixo |
| --- | --- | --- | --- |
| True | `[0]` | 1 | 0 |
| False | `[1]` | 1 | 0 |
| Eq | `[2, Term, Term]` | 3 | 0 |
| Pred | `[3, Symbol, [Term...]]` | 3 | 0 |
| Not | `[4]` | 1 | 1 |
| And | `[5, child_count]` | 2 | `child_count` |
| Or | `[6, child_count]` | 2 | `child_count` |
| Implies | `[7]` | 1 | 2 |
| Iff | `[8]` | 1 | 2 |
| Forall | `[9, Sort]` | 2 | 1 |
| Exists | `[10, Sort]` | 2 | 1 |

Counts de And/Or seguem a mesma regra unsigned de App. And/Or de zero filhos e
Pred com lista vazia são sintaxe válida; não são reescritos para True/False. Eq
contém exatamente dois envelopes Term, na ordem esquerda/direita. Pred contém
uma lista ordenada de envelopes Term. Esses termos são filhos lógicos para os
limites, mas seus records NÃO são inseridos no array de records da fórmula.

### 3.3. Validação da árvore

Um decoder DEVE rejeitar prefixo vazio, árvore incompleta ou raízes extras. Um
algoritmo independente equivalente usa uma stack de counts de filhos restantes:

1. Antes de ler o próximo record, retirar frames esgotados do topo. Sem frames,
   somente o primeiro record PODE começar uma raiz; outro record é raiz extra.
2. A profundidade do próximo record é `1 + comprimento_da_stack`. Conferir o
   limite de profundidade e o orçamento do próximo nó **antes** de converter seu
   record ou copiar metadados para a AST.
3. Ler estritamente o record e, quando houver, seus termos embutidos sob o
   orçamento reduzido descrito na seção 5.
4. Consumir um filho do frame pai, se houver; empilhar o count do record quando
   não zero; acumular estatísticas verificadas.
5. Ao fim, retirar frames esgotados. Exigir ao menos uma raiz e stack vazia.

Cada record DEVE casar uma das formas exatas da tabela: campo ausente, campo
extra, tag desconhecida ou payload de outro tipo não é aceito. O algoritmo usa
stack explícita, não recursão proporcional à profundidade lógica. Construção,
percurso, cópia e descarte de uma árvore plana NÃO devem exigir stack recursiva
proporcional ao número de records.

## 4. Variáveis e alfa-renomeação

Variáveis carregam **índices De Bruijn u64**, não nomes de binders. `0` identifica
o quantificador mais próximo no contexto de binders; `1` o próximo externo, e
assim sucessivamente. Cada Forall/Exists introduz um binder e carrega somente seu
Sort. Os termos embutidos em Eq/Pred referem-se ao contexto de quantificadores da
fórmula que os contém. Alterar nomes escritos pelo consumidor não altera os
índices, bytes nem hash; os nomes sequer aparecem na AST canônica.

Por exemplo, `forall x:S. x = x` e `forall y:S. y = y` são representados pelo
mesmo prefixo de fórmula: um record `[9, S]`, seguido por Eq de dois termos Var
com índice `0`. Sob dois quantificadores, índices `0` e `1` distinguem o binder
interno e externo. Shadowing é resolvido pelo consumidor antes de construir os
índices. Quantificadores não são removidos nem reordenados; mudar nesting, Sort
ou índice PODE mudar bytes e identidade.

Um termo isolado PODE conter quaisquer índices u64. A etapa sintática NÃO
verifica fechamento, quantidade de binders disponíveis ou sort da variável em
relação ao binder; um índice fora de contexto permanece representável.

## 5. Limites estruturais e orçamento agregado

Limites configuráveis são `max_depth` e `max_nodes`, inclusivos. Os padrões são
**128** e **100_000**. Qualquer limite zero não admite nó algum; prefixo vazio
continua inválido e não é uma árvore admitida por zero. Limites NÃO aparecem no
fio e não mudam bytes/hash de uma árvore que seja aceita.

- A raiz de um Term/Formula tem profundidade **1**. Um filho tem profundidade
  do pai mais 1.
- Termos em Eq/Pred são filhos lógicos do nó de fórmula: se esse nó está na
  profundidade `d` e um termo tem profundidade local `t`, sua profundidade máxima
  na fórmula é `d + t`.
- `max_nodes` conta cada record de fórmula e cada record de termo, inclusive
  todos os termos embutidos. Sorts, assinaturas, nomes, counts e envelopes não
  contam como nós. Metadata não contribui para a profundidade lógica.
- Um Eq tem contagem `1 + nodes(left) + nodes(right)` para seu próprio subtree;
  um Pred tem `1 + soma(nodes(arg))`. Mesmo argumentos idênticos são contados
  uma vez por ocorrência, não deduplicados.

Antes de ler termos embutidos num nó de profundidade `d`, sendo `used` a contagem
já consumida na fórmula, o decoder DEVE reservar o nó pai e usar limites locais:

`child_max_depth = max_depth - d`

`child_max_nodes = max_nodes - used - 1`

Para cada termo sucessivo, o decoder DEVE subtrair seus nós do orçamento local
**antes** de ler o próximo argumento. Nenhum filho PODE ser primeiro construído
sob orçamento maior para só depois conferir o agregado. Ao aceitar o record pai,
o validador DEVE também conferir `used + 1 + soma(child_nodes)` e
`d + max(child_depth)`, inclusive para ASTs cujos termos foram construídos
separadamente sob outros limites. Soma/conversão de estatísticas DEVE ser checked;
overflow não é wraparound, saturação nem aprovação.

Uma implementação PODE interromper a conversão no primeiro limite excedido sem
ler os records seguintes; não precisa calcular a profundidade total da entrada
rejeitada. Na API Rust de Formula, erros de termos embutidos reportam os limites
da chamada pública; o valor `actual` local é acrescido do offset
`parent_limit - child_limit` (incluindo pai e argumentos anteriores para nós),
com aritmética checked. Não é necessário descobrir o total da entrada rejeitada.

### 5.1. O que estes limites não prometem

O decoder Rust de bytes primeiro materializa um Value canônico e confere fim do
frame, depois converte a AST incrementalmente. Os limites da AST NÃO são teto de
memória total, tamanho de assinatura, texto, array CBOR ou bytes de rede. O
consumidor DEVE impor seu próprio teto de frame/CAS. A validação de canon/v1
continua impondo seu limite independente de aninhamento CBOR de 128.

A profundidade lógica NÃO aninha records CBOR: App/And/Or guardam counts e os
filhos permanecem no mesmo array plano. Em um valor air/v1 bem formado, o
aninhamento de coleções é limitado a no máximo 10 (Formula/Pred com Term/App e
Symbol com Sort), independentemente da profundidade lógica. Um milhão de Not
não produz um milhão de envelopes aninhados. O limite lógico DEVE ser conferido
sem recursão proporcional a esse milhão, inclusive ao descartar entrada rejeitada.

## 6. APIs e erros da implementação Rust

`Sort` e `Symbol` implementam `Canon`/`CanonDecode`. `Term`/`Formula` também, além
de `decode_with_limits(bytes, limits)` e `from_value_with_limits(value, limits)`.
`CanonDecode` usa os limites padrão; `new(nodes, limits)` valida a representação
prefixa construída pelo consumidor. Estatísticas públicas `depth()` e
`node_count()` têm as definições da seção 5; `logic_id()` retorna FolMsEqV1.

| Falha | Erro da API explícita Term/Formula |
| --- | --- |
| Violação canon/v1, entrada truncada ou bytes restantes | `AirError::Canon(c)` preserva o CanonError |
| Envelope/record de forma, versão, type tag, tag, aridade ou payload inválidos | `AirError::Canon(WrongType)` |
| Texto LogicId não suportado | `AirError::UnsupportedLogic` |
| Índice/count inteiro negativo ou count não representável em usize | `AirError::Canon(IntOutOfRange)` |
| Prefixo vazio, filho faltante, raiz extra ou overflow de estatísticas | `AirError::InvalidStructure` |
| Profundidade lógica excedida | `AirError::DepthExceeded { limit, actual }` |
| Número de nós excedido | `AirError::NodesExceeded { limit, actual }` |

Sort/Symbol reportam diretamente CanonError; formato desconhecido é WrongType.
Em `CanonDecode` de Term/Formula, `AirError::Canon(c)` DEVE ser convertido de volta
em `c`; todos os outros AirError DEVEM ser convertidos em `CanonError::WrongType`.
Não existe fallback para outra lógica ou objeto parcialmente válido. Erro de
formato NÃO é resultado de checagem ou credencial lógica.

Quando a entrada viola múltiplas condições, a validação canônica precede a
conversão AST; `decode_with_limits` verifica bytes restantes antes de converter.
Na conversão, `next_depth` precede a leitura do record. Não há requisito de
percorrer uma entrada já rejeitada para procurar outro erro.

## 7. Hash de Formula

Se `b` são os bytes canônicos do envelope **completo** de Formula, a identidade
criptográfica é:

`SHA-256(b"logos/formula/v1\0" || b)`

O byte NUL final (`00`) faz parte do domínio; não se adiciona tamanho, hash de
metadata, JSON, framing externo ou outra versão entre domínio e bytes. Não se
hasheia somente a lista de records. `Formula` implementa `logos_crypto::Domain`
com `tags::FORMULA`. Esta etapa não reserva domínios novos para Sort, Symbol ou
Term; tais objetos não recebem hash tipado de fórmula.

Exemplo mínimo: True é o valor `[1, 3, "FOL-ms-eq/v1", [[0]]]`, cujos bytes são
`8401036c464f4c2d6d732d65712f7631818100`. O hash é SHA-256 do domínio acima seguido
por esses bytes, não da grafia hexadecimal deles.

## 8. Vetores independentes

Os vetores de conformidade ficam em [vectors/air](vectors/air), produzidos pelo
gerador independente `tools/gen_air_vectors.py`, sem importar logos-air ou
logos-canon. Incluem variantes, fronteiras, alfa-renomeação e entradas rejeitadas.
A especificação é normativa: fixtures não autorizam forma alternativa, tags
adicionais, normalização ou omissão de LogicId.
