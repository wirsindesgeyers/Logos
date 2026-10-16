# ADR 0027 — Contrato de grounding e determinismo

- Estado: aceito; alternativa A adotada explicitamente pelo mantenedor em2026-10-15
- Etapas: 4.2 e 4.3
- Data: 2026-10-15
- Base inspecionada: 3ce411c0a2d6cb1def4f2e7eae3243ac7291ecd9
- Relacionadas: 0009, 0014, 0015, 0026

## Bloqueio confirmado

ROADMAP4.2 recebe classificação/candidatos4.1 e avaliador2.2, ambos existentes.
Seu segundo critério, porém, exige a mesma O gerar os mesmos bytes DIMACS,
com hash estável em CI. Tseitin/CNF, emissão DIMACS e h(CNF) são entregáveis
da4.3, que depende da saída proposicional4.2. Não existe nenhuma dessas
interfaces no código inspecionado; logos-ground contém apenas classificação
e candidatos de axiomas. logos-lrat ainda é esqueleto.

Reprodução na base inspecionada:

```sh
rtk proxy git show 3ce411c:crates/logos-ground/src/lib.rs
rtk proxy git show 3ce411c:crates/logos-lrat/src/lib.rs
rtk proxy git grep -n -E 'DIMACS|PropFormula|fn ground\(|fn clausify\(|struct Cnf' 3ce411c -- ':(glob)crates/**/*.rs'
```

O último comando termina1 por ausência de ocorrências, não por falha de teste.
A tabela do STATUS registrou o bloqueio antes do código. Nenhum critério foi
marcado na análise. AGENTS.md §1.3 manda:
"pare e registre como bloqueio" quando falta algo de que a etapa depende.
Não implementar um segundo emissor DIMACS provisório, nem chamar uma
representação proposicional de DIMACS.

## Decisão A — testar cada artefato onde é produzido

Preservar a ordem4.2→4.3 e o diferencial semântico de5k obrigações na4.2.
Na4.2, medir determinismo nos bytes canônicos da saída proposicional real,
incluindo tabela de átomos, fórmulas/raízes e vínculos a O/V. Preservar
explicitamente o teste integrado de mesmos bytes DIMACS/hash na4.3,
quando clausify/emissor existirem. O teste de determinismo não é eliminado.

Texto adotado no ROADMAP antes do código após a escolha explícita de A:

| Local | Ajuste concreto |
| --- | --- |
| 4.2, entregáveis | Explicitar ground(O,V) → Result<GroundProgram,GroundError>; GroundProgram contém PropFormula, tabela de átomos ordenada, premissas identificadas e goal, com h(O)/h(V), e codificação canônica versionada. A forma exata será especificada e vetorizada antes da implementação. |
| 4.2, critério semântico | Manter5k obrigações pequenas: validade por enumeração de todos os modelos contextualmente válidos do avaliador2.2 sse fórmula proposicional da obrigação é tautologia por tabela verdade. Casos de exaustão/entrada inválida não contam como equivalências demonstradas. |
| 4.2, critério de determinismo | Mesma O/V e limites suficientes → mesmos bytes canônicos de GroundProgram/tabela de átomos e digest fixado por vetores independentes; execução repetida no job nativo da CI. Só marcar a parte remota após observá-la. |
| 4.3, critério integrado adicional | Mesma O/V → mesma saída proposicional4.2 → mesmos bytes DIMACS/h(CNF); vetores e repetições na CI, preservando expressamente o critério original4.2. |

A4.3 continua responsável por Tseitin, CNF, DIMACS, ProvenanceMap, vínculo
h(O)/h(CNF) e rederivação pelo auditor. A4.2 preserva a identidade das
premissas para essa transformação; não atribui premissa de origem a cláusula
que ainda não existe. Não muda obrigação-v1 nem abre a construção selada.

O novo artefato exige spec/grounding.md, vetores independentes
em spec/vectors/grounding/ e domínio/versionamento documentados. Não se
reaproveita a tag de Formula, Obligation ou CNF. Decoder não é necessário
para a saída em memória; se algum parser/decoder for acrescentado, exige
alvo fuzz próprio ou extensão explícita de alvo que exercite essa entrada.

## Alternativa B — não adotada: manter contrato literal e redesenhar a fronteira

Manter4.2 bloqueada. Preparar outro plano com decisão explícita de quais
partes4.3 seriam antecipadas ou agrupadas, atualizando escopo/ordem e
responsabilidades no ROADMAP. Não assumir que isso está autorizado por
um pedido de continuação. Não fechar4.2 com DIMACS fictício ou stub.

## Semântica a preservar em qualquer alternativa

Estes pontos vêm de contratos existentes, não são autorização para reduzir
o fragmento ou adotar uma interpretação concreta arbitrária:

- V deve corresponder a O.vocab; a classificação e a tipagem abrangem
  premissas, goal e todas as definições. General/erro/limite não viram
  tautologia, certificado ou claim.
- Grounding da obrigação significa consequência: Γ implica goal, com as
  restrições semânticas de V. Não testar apenas a satisfatibilidade do goal
  nem conjunção de Γ com goal. A4.3 usa Γ e negação do goal para refutação.
- Axiomas de fecho e nomes únicos usados devem estar explícitos em O,
  preservando suas referências. Ausência não autoriza inserção silenciosa.
  Hipóteses explícitas permitem testes lógicos condicionais; não demonstram
  fontes Fact/Policy autenticadas ou used/deps das6.1/7.3/7.5.
- Constantes enumeradas têm ordinais distintos conforme model/v1. Outras
  constantes do mesmo sort não recebem ordinal pela ordem de declaração;
  podem coincidir entre si ou com um representante. Função primitiva com
  resultado enumerado admite todas as tabelas totais possíveis; não escolher
  uma tabela fixa nem tratar cada aplicação como predicado independente.
- Representar escolhas de valores por átomos/restrições proposicionais de
  existência/unicidade ou outra construção com equivalência demonstrada.
  Congruência, igualdade e aplicações aninhadas devem respeitar os valores.
  Essas restrições não são cláusulas de Tseitin ou prova de fonte.
- Defined Pred usa argumento i no índice De Bruijn i. Defined Const/Func
  usa resultado no índice0 e argumento i no índice i+1, com ambiente isolado.
  Preservar totalidade/unicidade e compatibilidade dos representantes
  enumerados, inclusive definições não usadas por O (evaluation.md §2).
  Interpretação rejeitada por InvalidDefinition/EnumerationMismatch não é
  modelo admissível e não pode falsificar artificialmente a equivalência.
- A semântica é FOL clássica com sorts não vazios. Enum vazio não permite
  instanciar Exists para false e declarar Forall true sob um modelo vazio;
  é contexto sem modelo clássico, a tratar explicitamente no contrato.
- Produtos cartesianos, expansão Defined/termos, nós/átomos e trabalho de
  construção precisam de limites inclusivos/aritmética checada. Reservas
  falíveis e Unknown(Size/Memory) não produzem saída parcial. Sem promessa
  de teto físico de RSS/tempo nem recursão nativa proporcional à entrada.

Contraprovas mínimas para o diferencial: constante adicional c:S, S={a,b},
goal c=a não é válido; função f:S→S, goal f(a)=a não é válido; corpos Defined
sem resultado ou com dois resultados devem ser excluídos dos modelos
admissíveis mesmo com goal que não menciona a definição. São requisitos
da implementação, não resultados obtidos por esta documentação.

## Consequências e estado

A estimativa continua M; tratamento de valores/Defined pode exigir revisão
ao fechar a IR e medir o diferencial. A meta de LOC grounder+Tseitin<1000
continua na4.3, sem alegação antecipada de cumprimento.

O [plano4.2](../plans/etapa-4.2.md) registra interfaces, testes e decisões.
A escolha explícita de A foi registrada e o ROADMAP foi ajustado antes do
código. DIMACS/Tseitin permanecem na4.3. Contrato concreto da IR/semântica
em spec/grounding.md; fontes continuam condicionais conforme ADR0026.

## Decisões técnicas concretas da4.2

GroundProgram privado, DAG topológico internado por conteúdo exato e tabela
ordenada por índice de símbolo/tupla/resultado. Codificação nova ground/v1,
tag logos/ground/v1\0 em logos-crypto e vetores independentes; nenhum decoder
novo, namespace CAS ou alteração de um wire anterior. Não confundir a nova
tag de hash com suporte a persistir esse tipo no CAS existente.

Tabelas simbólicas mantêm todas as escolhas de valores e impõem exatamente
um resultado; Defined é compilado topologicamente com guardas globais de
totalidade/unicidade/ordinal. Não há interpretação escolhida pelo grounder.
Exigir os candidatos4.1 exatos como premissas para todos os Enumerated de
V; registrar as referências encontradas, sem alegar resolução de fonte.

Limites lógicos por átomos/nós/arestas/células/passos/bytes e entrada4.1.
Células cobram entradas×max(1,outputs)×(aridade+1) antes de materializar
tuplas, incluindo coordenadas repetidas por átomo, também conservadoramente
para Defined. Revisão encontrou que contar apenas outputs omitia esse custo;
regressão e mutação específicas exigem a recusa antes da alocação.
Hash/canon e BTreeMap existentes não oferecem hard RSS/wall; restrições e
guardas não são certificado nem autenticação. Nenhuma dependência nova.
