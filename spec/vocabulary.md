# Vocabulário versionado com grounding — vocabulary/v1

Especificação normativa da Etapa 1.2 para o vocabulário versionado de **FOL-ms-eq/v1**.
As palavras DEVE, NÃO DEVE e PODE expressam requisitos normativos. Uma
implementação independente DEVE produzir os mesmos bytes para o mesmo vocabulário.
Este formato usa [canon/v1](encoding.md), sem modificar suas regras, a sintaxe
[air/v1](air.md) e o domínio de vocabulário já reservado em [domains.md](domains.md).
A decisão de representação e arquitetura está em [ADR 0009](../docs/adr/0009-vocabulario-grounding.md).

## 1. Escopo e identidade

`vocabulary/v1` representa declarações versionadas de sorts e símbolos de
FOL-ms-eq/v1. Grounding e domínios são declarações: seus valores externos e
fidelidade não são demonstrados por este formato.

O vocabulário NÃO realiza sort-check de fórmulas complexas (Etapa 1.3), parsing textual
(Etapa 1.4), resolução de atestações em ledger (Etapa 6.1) nem verificação criptográfica
de políticas (Etapa 7.1). O decoder e o construtor DEVEM validar formato, unicidade de
nomes, integridade de assinaturas, tipos de constantes em domínios enumerados e ausência
estrita de definições circulares.

A identidade de um sort é puramente nominal: o nome canônico em texto NFC.
A identidade de um símbolo é sua assinatura integral em [air/v1](air.md) (`Symbol`):
nome canônico, categoria (*kind*: Const, Func ou Pred), lista ordenada de sorts de entrada
e resultado (Sort ou Bool). Símbolos NÃO possuem campo de nome redundante no vocabulário;
o nome provém exclusivamente da assinatura.

Não há sobrecarga (*overload*) de nomes de símbolos: em um mesmo vocabulário, dois
símbolos NÃO DEVEM possuir o mesmo nome de texto, ainda que divirjam em aridade,
categoria ou tipo de retorno. De igual modo, sorts e símbolos pertencem a espaços de
nomes distintos, e não pode haver dois sorts com o mesmo nome.

## 2. Transporte, versão e envelopes

Um objeto `Vocabulary` completo é **exatamente um item CBOR** codificado conforme
canon/v1. Coleções são arrays de comprimento definido; nenhum mapa CBOR é utilizado.
Tags identificadoras são inteiros unsigned em posições fixas de arrays, não tags CBOR
do major type 6. Strings de texto DEVEM ser UTF-8 válido, normalizadas em NFC, compostas
exclusivamente de code points atribuídos em Unicode 16.0.0. Inteiros DEVEM utilizar a menor
representação canônica sem sinal.

Não há magic bytes, cabeçalhos adicionais nem bytes de padding. Uma API que decodifica
um vocabulário a partir de uma sequência de bytes DEVE rejeitar qualquer byte restante
após o término do item CBOR (`CanonError::TrailingBytes`).

O inteiro inicial `1` no envelope do vocabulário é a versão de formato de
**vocabulary/v1**, independente de canon/v1 ou air/v1. Qualquer versão ou tag diferente
das especificadas DEVE ser rejeitada imediatamente (`CanonError::WrongType`).

### 2.1. Tabela normativa de envelopes

| Tipo / Elemento | Representação CBOR canônica | Aridade |
| --- | --- | --- |
| **Vocabulary** | `[1, version, [SortDecl...], [SymbolDecl...]]` | 4 |
| **SortDecl** | `[name, domain]` | 2 |
| SortDomain **Open** | `[0]` | 1 |
| SortDomain **Enumerated** | `[1, [air/v1 Symbol...], source]` | 3 |
| DomainSource **Fact** | `[0, bytes32]` | 2 |
| DomainSource **Policy** | `[1, bytes32]` | 2 |
| **SymbolDecl** | `[air/v1 Symbol, grounding, gloss_pt]` | 3 |
| Grounding **Defined** | `[0, air/v1 Formula]` | 2 |
| Grounding **Anchored** | `[1, attestor_text, procedure_text]` | 3 |
| Grounding **Glossed** | `[2, text]` | 2 |

- `version`: inteiro unsigned de 64 bits (`u64`). O valor zero é admitido.
- `bytes32`: sequência binária de exatamente 32 bytes (CBOR major type 2, comprimento 32).
  Qualquer comprimento diferente de 32 bytes DEVE ser rejeitado com `CanonError::WrongType`.
- `name`, `attestor_text`, `procedure_text`, `gloss_pt`, `text`: strings CBOR (major type 3).
- `air/v1 Symbol` e `air/v1 Formula`: envelopes canônicos completos conforme `spec/air.md`.
  As fórmulas utilizam envelopes `[1, 3, "FOL-ms-eq/v1", [FormulaNode...]]`.

## 3. Declarações e domínios

### 3.1. Declaração de sort (`SortDecl`)
Cada declaração associa um nome de sort (`Text`) a um domínio (`SortDomain`):
- **Open (`[0]`)**: domínio aberto. A interpretação do sort admite indivíduos não
  enumerados a priori pelo vocabulário.
- **Enumerated (`[1, constants, source]`)**: domínio finito e fechado por enumeração.
  - `constants`: lista ordenada de símbolos constantes. Cada símbolo DEVE:
    1. Estar declarado no vocabulário (`symbols`);
    2. Ter categoria `SymbolKind::Const`;
    3. Ter aridade vazia;
    4. Ter resultado `SymbolResult::Sort(s)` onde `s.name == sort.name`.
  - Todas as constantes na lista DEVEM ser mutuamente distintas por nome.
  - `source`: autoridade formal que fixa a enumeração:
    - `Fact(FactRef)`: digest criptográfico SHA-256 de 32 bytes. Desde a 6.1b (ADR0062)
      é o hash AIR (`hash(Formula)`) da conjunção fecho ∧ nomes únicos do próprio sort
      (`spec/finite-fragment.md`), que um fato registrado precisa afirmar; não é `h(Fact)`,
      que criaria um ciclo (o fato carrega `h(V)`).
    - `Policy(PolicyRef)`: digest criptográfico SHA-256 de 32 bytes referenciando uma política.

As referências `FactRef` e `PolicyRef` são identificadores opacos de 32 bytes no fio;
o vocabulário NÃO resolve nem valida a existência dos registros externos referenciados.
A resolução de `Fact` é do kernel (PROPOSE/CERTIFY, `spec/fact.md` §4); `Policy` segue
recusada até a 7.3.

### 3.2. Declaração de símbolo (`SymbolDecl`)
Cada declaração associa uma assinatura `Symbol` ao seu `grounding` e a uma glosa
em linguagem natural (`gloss_pt` em texto UTF-8 NFC):
- A assinatura fixa nome, aridade (lista de sorts de entrada), categoria e resultado.
- Todos os sorts utilizados na aridade e no resultado (quando `SymbolResult::Sort`)
  DEVEM estar previamente ou simultaneamente declarados na lista `sorts` do vocabulário.
- A glosa `gloss_pt` é documentação destinada ao português; o formato só verifica
  texto canônico, não idioma, conteúdo, existência de significado ou não-vazio.

## 4. Grounding e dependências

O *grounding* explicita como o símbolo adquire significado semântico:

1. **Defined (`[0, Formula]`)**: símbolo definido formalmente em termos de outros símbolos.
   - O corpo da definição guarda **exclusivamente a fórmula definidora**, e NÃO o axioma
     completo `P(x) ↔ corpo`.
   - Parâmetros livres e expansão de Pred/Const/Func são fixados em
     [evaluation.md](evaluation.md): Pred usa argumento i no índice i;
     Const/Func usa resultado no índice 0 e argumento i no índice i+1.
     Corpos de valores definem relações totais com resultado único no modelo.
   - Auto-referência é expressamente proibida: o símbolo que está sendo definido
     NÃO DEVE aparecer no corpo definidor (seja como predicado, função ou constante).
   - Todo símbolo e todo sort referenciado no corpo da fórmula DEVE estar declarado
     no vocabulário, e sua assinatura no corpo DEVE coincidir estritamente com a declaração.
   - O orçamento de limites da fórmula (`Limits`) é avaliado independentemente para cada
     definição.

2. **Anchored (`[1, AttestorId, procedure]`)**: símbolo primitivo cuja denotação é ancorada
   em procedimento empírico ou institucional executado por um atestador externo.
   - `AttestorId`: identificador textual canônico opaco do atestador.
   - `procedure`: identificador textual do procedimento de medição, observação ou verificação.
   - Símbolos ancorados constituem bases primitivas e NÃO dependem de outros símbolos.

3. **Glossed (`[2, Text]`)**: símbolo primitivo fundamentado conceitualmente por texto descritivo.
   - Símbolos glosados constituem bases primitivas e NÃO dependem de outros símbolos.

### 4.1. Grafo de dependências e ordenação topológica

Apenas símbolos com grounding `Defined` participam como vértices com arestas de dependência
no grafo do vocabulário. Símbolos `Anchored` e `Glossed` são nós-folha (bases primitivas).

As dependências de uma definição $A \in \text{Defined}$ são todos os símbolos presentes no
corpo de sua fórmula:
- Símbolos em nós de predicado (`FormulaNode::Pred`);
- Símbolos de predicados ou funções aplicados dentro de termos de igualdade (`FormulaNode::Eq`);
- Símbolos aplicados em termos de argumentos de predicados.

O grafo de dependências DEVE ser acíclico (DAG). A API Rust usa Kahn iterativo,
sem recursão proporcional ao grafo. Uma implementação independente PODE usar
algoritmo equivalente. `definition_order` contém somente os índices de símbolos
Defined; dependências Defined precedem seus usuários. Bases não constam da lista.
A existência de qualquer ciclo direto ($A \to A$) ou indireto ($A \to B \to A$) DEVE
resultar na rejeição imediata com o erro `VocabularyError::CircularDefinition`.

## 5. Compatibilidade e evolução

A evolução de um vocabulário é modelada por compatibilidade direcional: um novo vocabulário
$V_{\text{novo}}$ é avaliado em relação a um vocabulário anterior $V_{\text{antigo}}$ através de
`V_novo.compatibility_with(&V_antigo)`:

- **Compatible**: $V_{\text{novo}}$ preserva **integralmente** todas as declarações de sorts
  e símbolos presentes em $V_{\text{antigo}}$:
  1. Todo sort de $V_{\text{antigo}}$ está presente em $V_{\text{novo}}$ com o mesmo nome,
     mesmo domínio (`Open` ou `Enumerated`) e, se enumerado, mesmas constantes na mesma ordem
     e mesma fonte de domínio.
  2. Todo símbolo de $V_{\text{antigo}}$ está presente em $V_{\text{novo}}$ com a mesma assinatura
     (nome, kind, aridade e resultado), mesmo grounding (mesma fórmula idêntica em `Defined`,
     mesmo atestador/procedimento em `Anchored`, mesmo texto em `Glossed`) e mesmo `gloss_pt`.
  3. $V_{\text{novo}}$ pode introduzir novos sorts e novos símbolos.
- **Incompatible**: qualquer alteração em sort ou símbolo pré-existente (mudança de glosa,
  redefinição de fórmula, alteração de assinatura, mudança de autoridade de domínio ou
  remoção de declaração).

A versão numérica (`version`) e a ordem física das declarações nas listas não afetam
a compatibilidade semântica, mas alteram a identidade criptográfica do vocabulário.

### 5.1. Operação de extensão (`extend`)

A operação `V_antigo.extend(patch_sorts, patch_symbols)` constrói deterministicamente uma
evolução do vocabulário:
1. Nomes em `patch_sorts` e `patch_symbols` que já existiam em $V_{\text{antigo}}$ substituem
   a declaração na posição ordinal original.
2. Nomes novos são anexados ao final das listas na ordem dada nos patches.
3. Duplicatas internas dentro de `patch_sorts` ou `patch_symbols` são proibidas.
4. O campo `version` é incrementado de forma segura via `checked_add(1)` (produzindo erro
   `VersionOverflow` em caso de overflow).
5. O vocabulário resultante é integralmente revalidado (checagem de unicidade, assinaturas,
   domínios e ausência de ciclos).
6. Retorna a nova instância acompanhada de sua marca de compatibilidade (`Compatible` ou
   `Incompatible`).

## 6. Separação de domínio e hash

O vocabulário possui domínio criptográfico tipado exclusivo reservado em `spec/domains.md`:
- Tag de domínio: `b"logos/vocab/v1\0"` (com byte nulo terminador `0x00`).
- Hash de vocabulário:
  $$h(V) = \text{SHA-256}(\mathtt{"logos/vocab/v1\textbackslash 0"} \mathbin{\Vert} \text{canon}(V))$$

Versão, ordem de declarações/constantes, glosas e fórmulas participam dos bytes
hasheados. Bytes diferentes produzem identidades diferentes salvo colisão SHA-256.

## 7. Invariantes e validação

Todo vocabulário construído via `Vocabulary::new` ou decodificado de bytes DEVE satisfazer
os seguintes invariantes:

1. **Unicidade de sorts**: não há dois sorts com o mesmo nome (`DuplicateSort`).
2. **Unicidade de símbolos**: não há dois símbolos com o mesmo nome na assinatura (`DuplicateSymbol`).
3. **Sorts declarados**: todo sort referenciado em assinaturas de símbolos ou em fórmulas
   está presente na lista de sorts (`UnknownSort`).
4. **Símbolos declarados**: todo símbolo referenciado em corpos de fórmulas ou constantes de
   domínio está presente na lista de símbolos (`UnknownSymbol`).
5. **Assinaturas exatas**: qualquer menção a um símbolo em uma fórmula ou domínio enumerado
   coincide integralmente com a assinatura declarada do símbolo (`SignatureMismatch`).
6. **Constantes de domínio**: constantes de domínios enumerados são símbolos do tipo `Const`,
   sem argumentos, com tipo de retorno igual ao sort, e sem duplicatas (`InvalidDomainConstant`).
7. **Aciclicidade**: o grafo de dependências entre símbolos definidos é um DAG finito
   (`CircularDefinition`).
8. **Limites estruturais**: cada Formula foi validada estruturalmente; o decoder
   de Vocabulary aplica `Limits` independentemente a cada definição.
   `Vocabulary::new` aceita fórmulas já validadas sob seus limites de construção,
   sem reaplicar os limites padrão.

## 8. Limitações e não-objetivos

1. **Sem verificação semântica profunda**: o vocabulário não garante consistência lógica
   teórica (ausência de contradições nas definições) nem modelo de satisfatibilidade.
2. **Sem sort-check completo**: a verificação de tipos termo-a-termo em fórmulas arbitrárias
   é responsabilidade da Etapa 1.3.
3. **Sem validação externa**: digests de `FactRef` e `PolicyRef` são verificados apenas em
   tamanho (32 bytes), não contra ledgers ativos ou sistemas externos.
4. **Sem interning**: representações canônicas contêm os objetos completos sem ponteiros ou
   arenas de memória.

## Extensão vocabulary/v2 (ADR0046, pré-requisito9.1a)

O formato histórico deste documento permanece aceito byte a byte. A extensão
normativa em [render-templates.md](render-templates.md) acrescenta render_pt ao
envelope v2, com validação/limites e hash sobre o conteúdo integral. Não altera
gloss_pt nem transforma v1 em v2 implicitamente. Compatibilidade passa também
a conferir templates quando v2 é usado; formatos diferentes são incompatíveis.
