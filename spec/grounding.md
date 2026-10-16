# Grounding finito — ground/v1 (Etapa4.2)

Contrato adotado: [ADR0027](../docs/adr/0027-contrato-de-grounding-e-determinismo.md).
Consome Obligation selado, V, AIR e avaliação2.2; não emite CNF/DIMACS/prova.

## Entrada e fontes

ground(O,V) e ground_with_limits(O,V,GroundLimits) retornam Result. Conferir
classify(O,V)==FiniteDomain, incluindo Defined global. General é erro explícito.
Para cada sort Enumerated de V, exigir as fórmulas exatas de fecho e nomes
únicos de4.1 como premissas de O; registrar a referência de cada uma.
Não adicionar axiomas ausentes. Hipóteses explícitas permitem consequência
condicional, não demonstram fonte autenticada (ADR0026). Nenhuma mutação de O.

## Semântica

GroundProgram contém DAG proposicional, raízes das premissas com referências,
goal, restrições de interpretação e raiz (restrições ∧ Γ) → goal. Átomos
primitivos são Pred(symbol,tuple) ou Value(symbol,tuple,output). Ordem:
índice original do símbolo em V, tupla lexicográfica (último eixo mais rápido),
output crescente. Nenhuma variável SAT ou cláusula de Tseitin nesta etapa.

Representantes enumerados primitivos denotam seu ordinal; outras constantes
e funções primitivas são escolhas de valores com existência/unicidade por
tupla. Resultado singleton é fixo e dispensa átomos de escolha. Predicados
primitivos recebem um átomo por tupla, inclusive nullary. Entradas Open em
primitivos não podem ser denotadas no fragmento aceito: essas tabelas são
omitidas e qualquer interpretação finita não vazia das partes Open estende
a projeção. Defined sobre Open/valores Open já é General pelo classificador.

Defined é processado na ordem topológica de V. Pred resulta do corpo sob
argumentos De Bruijn i; Const/Func usa resultado no0 e argumento i no i+1,
com ambiente isolado. Toda relação definidora de valor exige exatamente um
resultado por entrada, inclusive se não usada por O; representante Defined
deve coincidir com o ordinal. Guardas excluem valorações incompatíveis,
assim como o avaliador exclui InvalidDefinition/EnumerationMismatch dos
modelos admissíveis. Nunca escolher arbitrariamente um resultado.

Termos denotam vetores proposicionais de possíveis valores; aplicação combina
vetores dos argumentos com a tabela do símbolo. Eq é disjunção das conjunções
por valor igual. Quantificadores instanciam em ordem crescente; ambientes
de binder são isolados dos parâmetros Defined. Percursos, termos e DAG são
iterativos, sem recursão nativa proporcional à entrada.

Enum vazio produz guarda de contexto false: não há modelo clássico não vazio.
O fecho correspondente continua exigido em O. Validade nesse contexto é
vacuamente verdadeira por ausência de modelos, nunca autorização de fonte.

## DAG e wire

IDs de nós são índices topológicos, filhos anteriores ao pai. Começar com
False=0 e True=1. Internar por conteúdo exato, em ordem de construção. Not
é conservado; And/Or vazios dão True/False, singleton dá seu filho; demais
listas preservam ordem e multiplicidade. Implies(a,b)=Or(Not(a),b),
Iff(a,b)=And(Implies(a,b),Implies(b,a)). Não reordenar por hash.

Envelope CBOR canon/v1, somente saída (sem decoder novo):

```text
[1,h(O),h(V),atoms,nodes,constraints,premises,goal,root,domain_axioms]
atoms = [[symbol_id,[input_ordinals],[output_ordinal]?], ...]
output option = [] para Pred; [ordinal] para Value
nodes = [[0],[1],[2,atom_id],[3,child],[4,[children]],[5,[children]]]
premises = [[bytes32_event_ref,node_id], ...] na ordem de O
domain_axioms = [bytes32_event_ref, ...] por sort de V/fecho/pares i<j
```

Hashes são bytes32. h(G)=SHA256("logos/ground/v1\0" || canon(G)); tag própria
reservada em logos-crypto. Armazenar h(O)/h(V) não autentica a proveniência.
PropFormula é ID no DAG de GroundProgram; getters não permitem mutação.
O canônico de O/V, seu wire e seus domínios anteriores permanecem intactos.

domain_axioms também preserva dependências da transformação. Instanciação
e igualdade podem tornar esses axiomas tautológicos na saída proposicional;
isso não permite perder suas referências ao derivar CNF/used/deps. A4.3/4.7
deve preservar esse suporte, inclusive se nenhuma cláusula SAT corresponder
diretamente à fórmula original. Este artefato não demonstra used extraído
de certificado nem autenticação das fontes (integrações futuras).

## Limites

GroundLimits inclusivos: max_atoms10000, max_nodes100000, max_edges500000,
max_cells100000, max_steps1000000, max_bytes16777216 e AnalysisLimits padrão.
Cada tabela cobra entradas×max(1,outputs)×(aridade+1), incluindo coordenadas
de tuplas antes de materializar os argumentos dos átomos; Defined usa a
mesma cota conservadora. Não contar somente a quantidade de outputs.
Cobrar trabalho de construção, enumeração e combinação antes de cada passo;
produtos/tamanhos checados antes de expansão. max_cells limita tabelas e
vetores temporários; nós e arestas limitam o DAG mesmo com internamento.
Excesso/overflow → Unknown(Size); reserva falível → Unknown(Memory).
Não há resultado parcial nem General como fallback para entrada inválida.

O limite de bytes confere a codificação final, após construir Value de
profundidade constante sob limites lógicos; não é teto prévio de RSS.
Hash de O/V e APIs canônicas existentes podem alocar; sem hard RSS/wall.
Candidatos exigidos usam os limites4.1 existentes (até256 constantes por
enumeração/10000 axiomas/100000 nós); exaustão preserva Unknown(Size).

## Evidência exigida

5k obrigações distintas com modelos/valorações exaustivos e regressões de
funções/Defined/limites/axiomas ausentes; vetores independentes com bytes e
digest; mutantes compilados; fuzz pelos caminhos reais; smoke público.
Determinismo remoto só é marcado após observar CI. DIMACS/h(CNF) integrado
permanece desmarcado na4.3; fonte/used/deps reais permanecem nas6.1/7.3/7.5.
