# Sort-check — verificação estrita de tipos e binders em FOL-ms-eq/v1

Especificação normativa da Etapa 1.3 para a verificação de tipos e binders de fórmulas
de **FOL-ms-eq/v1** sobre um vocabulário declarado em **vocabulary/v1**.
As palavras DEVE, NÃO DEVE e PODE expressam requisitos normativos conforme a RFC 2119.
Uma implementação em conformidade DEVE reproduzir exatamente os mesmos resultados,
identificadores de erro e caminhos de localização para os mesmos pares de vocabulário e fórmula.

Este componente não altera a codificação canônica [canon/v1](encoding.md), a sintaxe
[air/v1](air.md) nem os envelopes de [vocabulary/v1](vocabulary.md). A fundamentação de
arquitetura e decisões técnicas está em [ADR 0010](../docs/adr/0010-sort-checker.md).

---

## 1. Escopo e modelo conceitual

O sort-checker é o componente de verificação de tipos e escopo de quantificadores de
fórmulas fechadas na lógica de primeira ordem multi-sortida com igualdade (`FOL-ms-eq/v1`).

Uma fórmula estruturalmente válida segundo `air/v1` é uma árvore prefixa plana de nós
(`FormulaNode`), contendo potencialmente termos embutidos planos (`TermNode`). O sort-checker
assegura que:
1. Todo sort utilizado em quantificadores, variáveis e assinaturas está devidamente declarado
   no vocabulário.
2. Todo símbolo utilizado em predicados ou aplicações de termos coincide integralmente com
   a declaração de mesmo nome no vocabulário (mesmo nome, categoria, sorts de entrada e resultado).
3. A categoria (*kind*) de cada símbolo é apropriada à sua posição sintática: predicados em
   posições de fórmula; constantes e funções em posições de termos.
4. O resultado (*result*) é booleano para predicados e sort individual para termos.
5. Constantes não possuem aridade de entrada não-vazia.
6. A quantidade de argumentos aplicados coincide exatamente com a aridade declarada da assinatura.
7. As variáveis De Bruijn estão devidamente ligadas a quantificadores ativos (*closed formula*)
   e seus sorts anotados coincidem rigorosamente com os sorts dos respectivos quantificadores.
8. Os termos comparados por igualdade (`Eq`) e os argumentos passados a predicados e funções
   possuem sorts idênticos aos esperados pelo contexto.

O sort-checker NÃO realiza inferência de tipos, NÃO expande corpos definidores
(`Defined`) e NÃO emite provas lógicas ou veredictos de satisfatibilidade.
`check` exige fórmula fechada; `check_with_binders` admite parâmetros externos
explícitos para os consumidores da 2.2. Apenas símbolos e sorts efetivamente
empregados na fórmula sob exame são validados; declarações não utilizadas
presentes no vocabulário não são avaliadas.

---

## 2. Interfaces normativas

A interface normativa de entrada e saída é puramente funcional e não produz efeitos colaterais.

### 2.1. Função principal

```rust
pub fn check(vocabulary: &Vocabulary, formula: &Formula) -> Result<(), SortError>;
```

A função recebe referências imutáveis ao vocabulário e à fórmula. Se todos os nós e termos
estiverem em estrita conformidade de tipos e fechamento, a função DEVE retornar `Ok(())`.
Caso contrário, DEVE retornar imediatamente `Err(SortError)` apontando para a primeira
inconformidade encontrada na ordem determinística de percurso.

Consumidores com parâmetros explícitos usam
`check_with_binders(vocabulary, formula, binders: &[&Sort])` com mesmos erros e
percurso. Binders estão em ordem externo → interno; último corresponde a índice
De Bruijn 0. Quantificadores locais acrescentam binders; ao sair do corpo,
restauram contexto inicial. Não se copia AST nem se infere sort de parâmetro.

### 2.2. Tipos de erro e localização

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortError {
    pub path: SortPath,
    pub kind: SortErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SortPath {
    pub formula_node: usize,
    pub term: Option<TermPath>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TermPath {
    pub position: TermPosition,
    pub node: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TermPosition {
    EqualityLeft,
    EqualityRight,
    PredicateArgument(usize),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SortErrorKind {
    UnknownSort,
    UnknownSymbol,
    SignatureMismatch,
    InvalidSymbolKind,
    InvalidSymbolResult,
    InvalidConstantArity { actual: usize },
    ArityMismatch { expected: usize, actual: usize },
    UnboundVariable { index: u64, binders: usize },
    SortMismatch { expected: Sort, actual: Sort },
}
```

- `formula_node`: índice base zero do record no array `formula.nodes()` da fórmula.
- `term`: quando o erro ocorre dentro de um termo embutido, identifica o termo e a posição do record
  base zero dentro de `term.nodes()`. Caso o erro ocorra no record da fórmula (por exemplo, símbolo
  de predicado inválido, aridade incorreta de predicado ou sort desconhecido em quantificador),
  `term` DEVE ser `None`.
- O método `Display` de `SortError` fornece diagnóstico textual descritivo em inglês. O texto exato
  não é contrato normativo; o contrato reside exclusivamente nos tipos estruturados `SortPath`
  e `SortErrorKind`.

---

## 3. Regras de tipagem e precedência de erros

A inspeção é realizada em pré-ordem da esquerda para a direita sobre a sequência plana de records.

### 3.1. Conectivos lógicos

1. `True` e `False` não possuem filhos nem parâmetros. São aceitos incondicionalmente.
2. `Not`, `Implies` e `Iff` validam recursivamente seus subtrees de fórmula filhos na ordem de aparição.
3. `And(k)` e `Or(k)` com $k \ge 0$ validam sequencialmente seus $k$ subtrees de fórmula filhos.
   Casos vazios ($k = 0$) são fórmulas válidas e não são simplificados nem rejeitados.

### 3.2. Quantificadores e escopo De Bruijn

1. Em `Forall(sort)` e `Exists(sort)`:
   - O sort do binder DEVE estar presente em `vocabulary.sorts()`. Se ausente, o verificador DEVE
     retornar `SortErrorKind::UnknownSort` no nó do quantificador (`term: None`).
   - O sort é empilhado no contexto local de binders ativos.
   - O corpo do quantificador (próximo subtree de fórmula) é verificado sob esse contexto estendido.
   - Ao término do percurso do corpo, o sort do binder DEVE ser desempilhado. Um binder NUNCA vaza
     para subtrees irmãos.

### 3.3. Predicados (`FormulaNode::Pred`)

Para um nó `FormulaNode::Pred { symbol, args }` no índice `formula_node`:
1. **Símbolo declarado**: `symbol.name` DEVE existir em `vocabulary.symbols()`. Caso contrário:
   `UnknownSymbol` em `SortPath { formula_node, term: None }`.
2. **Assinatura integral**: a assinatura `symbol` DEVE coincidir em todos os campos com a declaração
   do vocabulário de mesmo nome (mesmo `kind`, mesma lista `arity`, mesmo `result`). Caso contrário:
   `SignatureMismatch` em `SortPath { formula_node, term: None }`.
3. **Categoria de símbolo**: `symbol.kind` DEVE ser `SymbolKind::Pred`. Caso seja `Const` ou `Func`:
   `InvalidSymbolKind` em `SortPath { formula_node, term: None }`.
4. **Resultado de predicado**: `symbol.result` DEVE ser `SymbolResult::Bool`. Caso seja `Sort(_)`:
   `InvalidSymbolResult` em `SortPath { formula_node, term: None }`.
5. **Aridade de predicado**: a quantidade de termos em `args.len()` DEVE ser igual a `symbol.arity.len()`.
   Caso contrário: `ArityMismatch { expected: symbol.arity.len(), actual: args.len() }` em
   `SortPath { formula_node, term: None }`.
6. **Argumentos**: os termos em `args` são verificados em ordem sequencial $i \in 0..\text{args.len()}$:
   - Cada argumento na posição $i$ é verificado com o sort esperado `symbol.arity[i]`.
   - As coordenadas dentro do termo utilizam `TermPosition::PredicateArgument(i)`.

### 3.4. Termos e aplicações

Para cada nó de termo `TermNode` no índice `node` do termo embutido:

#### 3.4.1. Variáveis (`TermNode::Var { index, sort }`)
1. **Sort declarado**: `sort` DEVE estar declarado em `vocabulary.sorts()`. Caso contrário:
   `UnknownSort`.
2. **Ligação de binder**: o índice De Bruijn `index` DEVE satisfazer `index < binders.len()`.
   Caso contrário, o verificador DEVE retornar `UnboundVariable { index, binders: binders.len() }`.
   A comparação DEVE ser realizada sem truncamento numérico (por exemplo, via conversão para `u128`).
   O valor `u64::MAX` e índices grandes fora de escopo são estritamente rejeitados.
3. **Sort do binder**: o sort anotado na variável DEVE ser idêntico ao sort do binder encontrado
   na posição `binders[binders.len() - 1 - index]`. Caso divirja:
   `SortMismatch { expected: sort_do_binder, actual: sort.clone() }`.
4. O sort resultante da variável é `sort`.

#### 3.4.2. Aplicação (`TermNode::App { symbol, args }`)
1. **Símbolo declarado**: `symbol.name` DEVE existir em `vocabulary.symbols()`. Caso contrário:
   `UnknownSymbol`.
2. **Assinatura integral**: a assinatura `symbol` DEVE coincidir em todos os campos com a declaração
   do vocabulário de mesmo nome. Caso contrário: `SignatureMismatch`.
3. **Categoria de termo**: `symbol.kind` DEVE ser `SymbolKind::Const` ou `SymbolKind::Func`.
   Se for `SymbolKind::Pred`: `InvalidSymbolKind`.
4. **Resultado de termo**: `symbol.result` DEVE ser `SymbolResult::Sort(s)`.
   Se for `SymbolResult::Bool`: `InvalidSymbolResult`.
5. **Aridade de constante**: se `symbol.kind == SymbolKind::Const`, a assinatura DEVE ter aridade
   vazia (`symbol.arity.is_empty()`). Se `symbol.arity` não for vazia:
   `InvalidConstantArity { actual: symbol.arity.len() }`.
6. **Aridade de aplicação**: o número de records filhos no prefixo `args` DEVE ser igual a
   `symbol.arity.len()`. Caso contrário:
   `ArityMismatch { expected: symbol.arity.len(), actual: *args }`.
7. O sort resultante da aplicação é o sort retornado por `symbol.result`.

#### 3.4.3. Precedência em nós de termos
- **Erro intrínseco precede mismatch**: qualquer erro de declaração, assinatura, kind, result,
  aridade ou variável livre no próprio nó é reportado antes de verificar se o sort resultante do nó
  coincide com o sort esperado pelo pai ou pelo contexto.
- **Sort esperado precede descida nos filhos**: após validar os requisitos intrínsecos do nó,
  se o sort resultante do nó divergir do sort esperado pelo contexto, o erro
  `SortMismatch { expected, actual }` DEVE ser reportado imediatamente na coordenada desse nó,
  **antes** de descer nos subtermos filhos.

### 3.5. Igualdade (`FormulaNode::Eq(left, right)`)

Para um nó de igualdade no índice `formula_node`:
1. O termo esquerdo `left` é verificado integralmente na posição `TermPosition::EqualityLeft`
   sem sort prévio esperado. O sort resultante da sua raiz é determinado como $S_{esq}$.
2. O termo direito `right` é verificado na posição `TermPosition::EqualityRight` tendo $S_{esq}$
   como sort esperado.
3. **Precedência na igualdade**:
   - Erros intrínsecos no nó raiz do termo direito (símbolo desconhecido, assinatura divergente,
     kind inválido, resultado inválido, aridade incorreta ou variável livre) são reportados
     em `TermPath { position: TermPosition::EqualityRight, node: 0 }` antes da verificação
     de compatibilidade com $S_{esq}$.
   - Se o nó raiz do termo direito for válido mas seu sort resultante divergir de $S_{esq}$,
     o erro `SortMismatch { expected: S_{esq}, actual: S_{dir} }` DEVE ser reportado
     imediatamente no nó raiz do termo direito (`node: 0`), **antes** de percorrer os subtermos
     filhos do termo direito.
   - Caso o sort da raiz coincida, os subtermos filhos do termo direito são verificados na ordem
     prefixa correspondente às suas respectivas posições.

---

## 4. Localização de erros e coordenadas de caminho

A coordenada do erro é representada pela estrutura `SortPath`:

| Ponto do erro | `formula_node` | `term` |
| --- | --- | --- |
| Erro em quantificador (`Forall`/`Exists`) | Índice do record do quantificador | `None` |
| Erro em predicado (`Pred` símbolo ou aridade) | Índice do record `Pred` | `None` |
| Erro no termo esquerdo de `Eq` | Índice do record `Eq` | `Some(TermPath { position: EqualityLeft, node })` |
| Erro no termo direito de `Eq` | Índice do record `Eq` | `Some(TermPath { position: EqualityRight, node })` |
| Erro em argumento de `Pred` | Índice do record `Pred` | `Some(TermPath { position: PredicateArgument(i), node })` |

O índice `node` em `TermPath` refere-se estritamente à posição do record `TermNode` no vetor
de records do termo correspondente, iniciando em `0` para o nó raiz do termo.

---

## 5. Garantias de complexidade e limites de conformidade

1. **Complexidade temporal e espacial**:
   - O percurso dos prefixos é linear no número de records. A implementação Rust
     constrói índices por nome em O(V log V), consulta declarações em O(log V)
     por ocorrência e compara assinaturas completas. Nomes/assinaturas também
     contribuem ao custo; não há promessa de tempo linear somente em records.
   - Memória adicional inclui índices O(V), stacks O(profundidade) e contexto de
     binders. Não copia ASTs ou nomes no caminho bem-sucedido.
2. **Segurança de pilha**:
   - A implementação DEVE ser iterativa e não utilizar chamadas recursivas de função para percorrer
     a árvore sintática, prevenindo exaustão de pilha em ambientes restritos (como pilhas de 64 KiB).
3. **Natureza da cobertura de ramos**:
   - A exigência de 100% de cobertura de ramos instrumentados (`branch coverage`) em `src/sort_check.rs`
     é uma métrica de rigor de teste e exercício de código sob o compilador de referência.
   - Ela comprova que todas as bifurcações instrumentadas na implementação foram exercitadas pela suíte
     de testes e geradores, mas **não constitui prova matemática formal** da ausência de incompletudes
     na especificação ou de falhas de hardware.
