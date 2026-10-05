# ADR 0010 — Sort-checker de primeira ordem multi-sortida com igualdade

- Status: aceito
- Data: 2026-10-05
- Etapa: 1.3

## Contexto

A Etapa 1.1 ([ADR 0007](0007-sintaxe-air.md)) fixou a sintaxe abstrata plana FOL-ms-eq/v1 (`air/v1`),
na qual sorts e símbolos possuem identidades estruturais canônicas e variáveis utilizam índices
De Bruijn `u64`. A Etapa 1.2 ([ADR 0009](0009-vocabulario-grounding.md)) estabeleceu o vocabulário
versionado (`vocabulary/v1`), garantindo unicidade de nomes, aciclicidade em definições e integridade
de declarações nominais.

Para que fórmulas possam ser consumidas com segurança pelas etapas posteriores (obrigações em 1.5,
modelos finitos na camada 0 e verificadores formais), é indispensável verificar se uma fórmula é
bem sortada (*well-sorted*), se suas variáveis estão estritamente fechadas (*closed formula*) e se o
uso de cada símbolo respeita sua categoria (*kind*), aridade e tipos de entrada/saída declarados.

Esta etapa realiza estritamente a checagem de tipos e binders. Ela não introduz inferência de tipos,
não realiza expansão de corpos `Defined`, não altera a representação `air/v1` nem o formato `vocabulary/v1`,
e não adiciona dependências externas ao TCB.

## Decisão

1. **API pura e sem efeitos colaterais**:
   - A verificação de tipos é exposta exclusivamente pela função:
     `pub fn check(vocabulary: &Vocabulary, formula: &Formula) -> Result<(), SortError>;`
   - Uma fórmula válida, fechada e aderente ao vocabulário retorna `Ok(())`.
   - Qualquer desconformidade aborta imediatamente na primeira violação encontrada pela ordem
     canônica de visitação, retornando `Err(SortError)`.

2. **Percurso iterativo sem recursão de pilha (*stack-safe*)**:
   - A árvore plana de records em pré-ordem é percorrida sequencialmente, sem recursão proporcional
     à profundidade lógica.
   - São mantidas pilhas explícitas para controle de contagem de filhos restantes e contexto
     de quantificadores (`FormulaFrame`), e para aridades de aplicações em termos (`TermFrame`).
   - A verificação é estritamente segura para execução em ambientes de pilha restrita (como threads
     com pilha de 64 KiB), mesmo para fórmulas e termos com centenas ou milhares de nós.

3. **Localização exata sem alocação dinâmica proporcional**:
   - O caminho de erro `SortPath` identifica com precisão zero-based o índice do record no prefixo
     plano da fórmula (`formula_node: usize`) e, quando o erro reside dentro de um termo embutido
     (em igualdade ou argumento de predicado), a coordenada `TermPath`:
     - `position`: `EqualityLeft`, `EqualityRight` ou `PredicateArgument(usize)`.
     - `node`: índice zero-based do record `TermNode` no prefixo do termo.
   - Não são alocados vetores de ancestrais (*breadcrumbs*) nem strings de diagnóstico durante o
     percurso ou no caminho bem-sucedido. `SortPath`, `TermPath` e `TermPosition` derivam `Copy`.

4. **Regras e precedência determinística de erros**:
   - **Conectivos**: `True`, `False`, `Not`, `And`, `Or`, `Implies`, `Iff` são fórmulas canônicas.
     `And(0)` e `Or(0)` são fórmulas vazias válidas e não são reescritas.
   - **Quantificadores**: `Forall(sort)` e `Exists(sort)` exigem que o sort do binder esteja
     declarado no vocabulário (`UnknownSort` na raiz do quantificador). O sort é empilhado no contexto
     de binders ativos exclusivamente durante a descida no corpo do quantificador; ao término do
     subtree do corpo, o sort é desempilhado e nunca vaza para nós irmãos.
   - **Predicados**:
     - O nome do símbolo deve estar declarado no vocabulário (`UnknownSymbol`).
     - A assinatura integral do nó deve coincidir identicamente com a declaração (`SignatureMismatch`).
     - A categoria do símbolo deve ser `SymbolKind::Pred` (`InvalidSymbolKind`).
     - O resultado do símbolo deve ser `SymbolResult::Bool` (`InvalidSymbolResult`).
     - A quantidade de termos argumentos deve coincidir com a aridade da assinatura (`ArityMismatch`).
     - Argumentos são verificados da esquerda para a direita na posição `PredicateArgument(i)`.
   - **Termos em Aplicação (`App`)**:
     - O nome do símbolo deve estar declarado no vocabulário (`UnknownSymbol`).
     - A assinatura integral deve coincidir com a declaração (`SignatureMismatch`).
     - A categoria deve ser `Const` ou `Func` (`InvalidSymbolKind` caso seja `Pred`).
     - O resultado deve ser `SymbolResult::Sort` (`InvalidSymbolResult` caso seja `Bool`).
     - Constantes declaradas com aridade não-vazia reportam `InvalidConstantArity`.
     - O número de filhos no prefixo (`args`) deve coincidir com a aridade da assinatura (`ArityMismatch`).
   - **Variáveis (`Var`)**:
     - O sort anotado na variável deve estar declarado no vocabulário (`UnknownSort`).
     - O índice De Bruijn deve apontar para um binder ativo no contexto atual (índice `0` denota o
       quantificador mais interno). Índices maiores ou iguais à quantidade de binders ativos reportam
       `UnboundVariable { index, binders }`. O valor `u64::MAX` e índices arbitrários são verificados
       sem risco de truncamento via conversão sem perda (`u128`).
     - O sort da variável deve coincidir exatamente com o sort do binder encontrado (`SortMismatch`).
   - **Igualdade (`Eq`) e precedência na raiz**:
     - O termo esquerdo é verificado integralmente, determinando seu sort resultante $S_{esq}$.
     - O termo direito é verificado passando $S_{esq}$ como sort esperado da sua raiz.
     - **Precedência na igualdade**: qualquer erro intrínseco no nó raiz do termo direito (declaração,
       assinatura, kind, result, aridade ou variável livre) precede a checagem de sort da igualdade.
     - Se o nó raiz do termo direito for intrinsecamente válido mas seu sort divergir de $S_{esq}$,
       o erro `SortMismatch { expected: S_{esq}, actual: S_{dir} }` é reportado imediatamente na raiz
       do termo direito (`EqualityRight`, nó 0), **antes** de percorrer os subtermos filhos do termo direito.
   - **Precedência em termos gerais**:
     - Em todo nó de termo, erros intrínsecos precedem divergência de sort com o pai ou parâmetro.
     - A verificação do sort esperado no nó é realizada antes de descer nos filhos daquele termo.

5. **Isolamento de corpos `Defined` e economia de verificação**:
   - Apenas os sorts e símbolos **efetivamente utilizados** na fórmula em verificação são consultados.
     O vocabulário pode conter símbolos declarados não usados sem que sua suficiência seja avaliada aqui.
   - Símbolos com grounding `Defined` no vocabulário **não são expandidos** e seus corpos **não são
     checados como fórmulas fechadas** pelo sort-checker. Definições podem conter variáveis livres
     correspondentes aos parâmetros formais; sua expansão pertence à Etapa 2.2.

6. **Sem mutação, sem cópias e sem novo formato**:
   - `check` opera exclusivamente sobre referências imutáveis `&Vocabulary` e `&Formula`.
   - O percurso bem-sucedido não clona ASTs, identificadores ou sorts. Mapas de busca temporários
     emprestam referências `&Text` e `&Symbol`. Apenas o erro `SortMismatch` instancia cópias dos
     dois sorts discrepantes para fornecer ownership estruturado ao chamador.
   - Nenhum novo formato CBOR, domínio criptográfico ou hash é introduzido.

## Consequências

- O percurso é iterativo. A busca por declaração custa O(log V) por ocorrência,
  além do custo de comparar assinaturas completas; índices custam O(V log V).
- Aceitação verifica fechamento, tipos de aplicação e igualdade, não validade lógica.
- Localizações de erros são reprodutíveis e independentes de mensagens textuais ou nomes de variáveis.
- Não há aumento no Trusted Computing Base (TCB) em termos de dependências de terceiros.
- O critério de 100% dos ramos LLVM será medido pela suíte comportamental.
  Fuzz local/noturno é evidência adicional, não substitui a medição.

## Alternativas rejeitadas

- **Expansão inline de corpos `Defined` durante o sort-check**: rejeitada porque corpos definidores
  possuem variáveis De Bruijn livres representando parâmetros de predicados/funções; sort-checar
  uma definição como fórmula fechada causaria falsos positivos de `UnboundVariable`.
- **Vetores de caminho ancestrais (`Vec<usize>`) em cada nó**: rejeitada por exigir alocações
  dinâmicas frequentes na heap no caminho crítico de verificação.
- **Recursão de função na checagem da AST**: rejeitada pelo perigo iminente de exaustão de pilha
  (stack overflow) em fórmulas adversariais com centenas de níveis de aninhamento.
- **Mensagens diagnósticas em português como contrato**: rejeitado; o contrato normativo é a
  estrutura de dados `SortError` (`SortPath` e `SortErrorKind`). O método `Display` provê diagnósticos
  em inglês para observabilidade sem constituir contrato de formato.
