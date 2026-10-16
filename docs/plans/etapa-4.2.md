# Plano — Etapa4.2: contrato do grounder

Estado: implementada/verificada localmente; determinismo remoto pendente. Escolha explícita de A no
[ADR0027](../adr/0027-contrato-de-grounding-e-determinismo.md). A alternativa A foi adotada pelo mantenedor em2026-10-15; ROADMAP4.2/4.3
ajustados antes do código. Primeiro critério demonstrado; determinismo remoto continua desmarcado.

## 1. Interfaces de entrada

- Obligation selado/generate1.5: premissas identificadas, goal e h(V);
  spec/obligation.md. Não usar decoder para construir O autoritativo.
- Vocabulary/AIR/sort-check1.1–1.3; classificação e candidatos4.1,
  spec/finite-fragment.md e ADR0026. V explícito deve corresponder a O.
- logos-model2.2 em testes, conforme spec/model.md e spec/evaluation.md:
  enumeração total de interpretações pequenas, com Defined contextualmente
  válido, domínios não vazios e rejeição distinta de exaustão.
- logos-canon/crypto existentes para codificação determinística/identidade,
  conforme a alternativa A adotada. Sem pacote externo novo
  previsto. Qualquer aresta runtime nova deve constar do ADR/allowlist.

Ausente e bloqueante no contrato anterior: CNF/Tseitin/DIMACS da4.3. Inspeção
em3ce411c não encontrou PropFormula/ground/clausify/Cnf/emissor DIMACS.

## 2. Saídas adotadas

```text
ground(&Obligation,&Vocabulary) -> Result<GroundProgram,GroundError>
ground_with_limits(...,GroundLimits) -> Result<GroundProgram,GroundError>
GroundProgram: h(O), h(V), tabela de átomos, restrições semânticas,
               premissas com PremiseRef e goal como PropFormula
```

A fórmula lógica associada será (restrições de interpretação ∧ Γ) → goal.
Preservar raízes separadas para clausify(Γ_ground,¬goal_ground) na4.3 e para
as origens de premissas. Restrição semântica/definição não recebe PremiseRef
inventado. PropFormula usa percurso/armazenamento iterativo, IDs e ordem
determinísticos; o formato e seus limites devem ser fixados na spec antes
do código. Contrato concreto em spec/grounding.md.

Átomos representam predicados instanciados e, quando necessários, escolhas
de valores; existência/unicidade e Defined precisam restringir exatamente
as interpretações admitidas pelo avaliador. Sem expansão de CNF/Tseitin,
SAT/LRAT, kernel, autenticação ou completion de predicado nesta etapa.

## 3. Tarefas em ordem e evidência prevista

1. Fechar ADR0027 com decisão explícita e ajustar ROADMAP4.2/4.3 antes de
   implementar. Se não houver decisão, manter bloqueio no STATUS. Releitura
   das duas etapas deve mostrar o teste DIMACS preservado e o diferencial
   de5k intacto; nenhum checkmark durante a preparação.
2. Especificar IR/versionamento/domínio, ordenação dos átomos, identidade de
   O/V, raiz da obrigação e proveniência preservada em spec/grounding.md;
   registrar escolhas técnicas no ADR. Vetores independentes devem fixar
   bytes/digest. GroundLimits cobre trabalho, nós, átomos, produtos e bytes,
   com testes de limites exatos/um abaixo/overflow antes de alocações.
3. Pré-validação em logos-ground: consumir classify/sort-check; conferir
   O/V; identificar os axiomas explícitos de fecho/nomes únicos utilizados
   e suas referências, recusando ausência sem inserção implícita. Não
   aceitar fonte opaca como autenticação. Testes condicionais constroem
   O pelo generate real com hipóteses explícitas e escopo exato.
4. Grounding iterativo: quantificadores enumerados, conectivos/igualdade,
   valores primitivos, aplicações aninhadas, Defined e validação global
   de definições. Criar somente arquivos da etapa em logos-ground e
   listá-los no manifesto. Asserções pequenas cobrem alternância de
   quantificadores, parâmetros de sorts diferentes, sombreamento e
   isolamento de ambientes; cadeia longa e stack pequena detectam recursão.
5. Funções primitivas exigem todas as tabelas totais possíveis sobre as
   entradas pertinentes; constantes adicionais não são representantes
   novos. Defined usa ordem topológica e semântica relacional existente,
   inclusive validade global de símbolos não usados. Testar congruência,
   totalidade, unicidade, ordinal enumerado e funções nullary.
6. Diferencial independente em testes: gerar pelo menos5k obrigações
   pequenas distintas com sorts enumerados, enumerar todos os modelos dos
   domínios declarados e todas as valorações da saída proposicional. Usar avaliador
   2.2 como oráculo FOL e um interpretador de testes para PropFormula;
   não reutilizar o algoritmo do grounder para produzir o esperado.
   Guardas limitam previamente o tamanho dos casos, sem amostragem de
   modelos/valorações. Reportar os5k casos e a partição de construtores,
   com Γ verdadeiro/falso e goal verdadeiro/falso; entradas inválidas ficam
   em regressões separadas e não compõem os5k. Erros/Unknown não são false
   nem casos aprovados; definições sem modelo
   exigem controles explícitos de validade vacuamente verdadeira.
7. Determinismo: gerador Python independente/vetores em
   spec/vectors/grounding/; repetições com mesmos bytes O/V e limites
   suficientes. Ordem baseada no contrato canônico, nunca HashMap/horário.
   Integrar smoke/vetores/semântica nos jobs existentes, mantendo o teste
   remoto aberto até observá-lo. Não escrever DIMACS nesta etapa sob A.
8. Mutantes compilados: trocar Forall/Exists, ignorar premissa, comparar
   nomes de constantes adicionais, fixar arbitrariamente tabela de função,
   omitir unicidade/Defined global, trocar índices De Bruijn, pular limite
   ou perder PremiseRef. Cada bug deve compilar e causar falha de asserção
   ou divergência do oráculo; restaurar fontes antes da verificação final.
9. Exercitar o caminho novo por fuzz com entradas reais O/V via APIs
   existentes; decidir extensão do alvo pertinente na implementação.
   Parser/decoder novo exige fuzz específico, se for necessário. Atualizar
   manifesto/allowlist/locks/CI/AGENTS/README conforme arquivos tocados.
10. Rodar verificação completa/selftest e fuzz oficial, medir TCB; somente
    então atualizar STATUS/diário/critério demonstrado e criar commit local.

## 4. Critérios → teste/artefato

| Critério vigente | Evidência prevista | Estado |
| --- | --- | --- |
|5k obrigações: validade FOL sse tautologia proposicional | Enumeração exaustiva dos dois lados, modelos contextualmente válidos do avaliador2.2; relatório e mutantes | Demonstrado localmente:3140 válidas/1860 inválidas |
|Critério original: mesma O → DIMACS/hash estável em CI | Teste integrado ground→clausify→DIMACS | Preservado, desmarcado, na4.3 (ADR0027) |
|Critério adotado: mesmos bytes da saída4.2 | Doze vetores independentes e repetições nativas | Local passou; CI ainda não observada |

## 5. Riscos e decisões abertas

Fronteira4.2/4.3 adotada por A. Funções são escolhas simbólicas totais,
sem interpretação fixa. Exigir os candidatos exatos4.1 como premissas para
todos os sorts Enumerated de V, preservando referências. Enum vazio é sintaxe aceita sem modelo clássico,
não domínio vazio do avaliador. Global Defined pode restringir bases mesmo
quando não alcançado pelo goal; erro de definição não é contramodelo.

Evitar expansão combinatória antes do pré-cálculo de limites; nomes/glosas
e codificação também têm custo. Axiomas lógicos condicionais não fecham
integrações Fact/Policy/used das6.1/7.3/7.5. Sem wall/RSS rígidos prometidos.
API/bytes/tag e semântica foram fixados em spec/grounding.md antes do código.

## 6. Estimativa e estado

M, como no ROADMAP. Implementação pronta em ground.rs/dag.rs/prop.rs,
manifesto e tag próprios atualizados; sem CNF/DIMACS ou novas dependências.
Semântica, bytes e verificação completa/selftest demonstrados localmente;
critério de determinismo remoto ainda pendente.
Determinismo remoto e fuzz ≥1h continuam pendentes.

## 7. Evidência e histórico de verificação

Inspeção em3ce411c: APIs4.1 e avaliador2.2 presentes; nenhuma ocorrência
de PropFormula/ground/clausify/Cnf/DIMACS nos fontes Rust. Auditoria local
target/contract-4.2-audit.json: CONTRACT_4_2_AUDIT_OK, links conferidos e
ROADMAP sem alterações naquele momento. Nenhum teste do grounder tinha
rodado antes da escolha de A; a implementação ainda não existia.

A primeira verificação completa desta preparação terminou101 em gc.rs:
reachable_noncanonical_payload_is_error_instead_of_empty_graph recebeu
WriterBusy ao reabrir a fixture. Isolado passou; suíte sem alteração voltou
a falhar na sexta repetição. Experimento local reproduziu flock herdado
por fork mesmo com CLOEXEC: fechamento do descritor no pai não libera o
lock enquanto o filho o mantém. Isso explica uma janela entre fork e exec
nos testes paralelos que criam subprocessos e reabrem ledgers.

Fixtures de GC agora são serializadas por Mutex durante sua vida inteira;
teste explícito de writer ocupado continua criando seu segundo processo.
Não mudou o lock da TCB nem o comportamento do GC. Repetição corrigida:
24 suítes/144 testes, todos passaram. Logs em target/gc-repeat-4.2/ e
target/gc-repeat-fixed-4.2/; experimento target/gc-lock-inheritance-4.2.json.
strace não está instalado; não houve captura de syscalls da falha original.
Fmt apontou ordem dos imports adicionados, corrigida por rustfmt.
Nova execução documental interrompida com exit143 após escolha de A, antes
de Python/selftest; não contada como sucesso. Uma execução final deve cobrir
a implementação completa, ainda sem VERIFY_OK desta sessão.


Após a escolha explícita de A, ADR/ROADMAP ajustados antes do código e
spec/grounding.md fixou ground/v1/tag logos/ground/v1\0. GroundProgram
preserva h(O)/h(V), DAG/tabela, restrições, premissas/goal e referências dos
axiomas explícitos, sem autenticação/claim. Não há decoder novo ou alteração
de wire anterior. Os três fontes novos estão no manifesto; dependências
runtime/dev já existiam, nenhum pacote novo.

Diferencial local:5000 O distintas (unicidade dos bytes conferida),32
interpretações por caso,16 admissíveis/16 excluídas por InvalidDefinition
em definição global não usada; todas as256 valorações da saída avaliadas
por tabela verdade integral. Resultado3140 válidas/1860 inválidas, nenhuma
divergência. Γ/validade de modelos comuns são calculados uma vez pelo
avaliador2.2; cada goal é avaliado por2.2 em todos os16 modelos admissíveis,
sem curto-circuito. Bitsets de64 valorações aceleram o interpretador de
DAG dos testes sem amostrar valorações ou reutilizar o grounder. Rodada
inicial redundante interrompida por SIGTERM (cargo101), não contada; versão
completa passou em70,31s. Erros não esperados/Unknown nunca são descartados.

Onze regressões novas (incluindo o diferencial) cobrem limites inclusivos
com busca do mínimo de passos/células, produtos em overflow, predicados e
funções nullary, constantes adicionais, congruência, mistura de sorts,
De Bruijn/quantificadores/escopos, Defined total/único/ordinal global, cadeia
200/stack64KiB, axiomas ausentes, h(V), refs e não mutação de O. Doze vetores
booleanos independentes Python/canon/hashlib conferem O/ground bytes/hash;
fontes factuais não são fingidas nesses controles condicionais. Smoke:
GROUND_SMOKE_OK cases=12 wire=ground/v1 claims=0. Repetição de100 construções
idênticas, maior teto sem alterar wire e oito repetições por vetor passaram.

Compilação inicial exigiu Vec<bool> explícito no interpretador de testes;
Clippy apontou documentação do crate de teste. Corrigidos. Ruff detectou
captura de variável de loop no gerador, fixada por argumento ligado à
instância; mypy exigiu anotação das listas heterogêneas. Workspace Clippy,
Ruff/format/mypy e vetores --check finais passaram, sem suprimir os diagnósticos encontrados.

Fuzz oficial vocabulary_decode/nightly-2026-09-28:11348 execuções/61s,
exit0 sem crash, sanitizers ativos/max_len4096. Alvo existente recebe V
arbitrário e estruturado (função/Defined), constrói O condicional real,
confere repetição/identidade/ordem/topologia/limites. Log preexistente
restaurado byte a byte; novo log target/vocabulary-decode-4.2.log.

Primeira rodada de12 mutantes compilados detectada. Após o ajuste de células,
rodada final de13 mutantes compilados detectados por asserções; nenhuma falha de
compilação contou como captura. Fontes restauradas em finally. Reproduzir
substituições em target/grounding-mutations/results.json e executar
cargo test -p logos-ground --test grounding NOME -- --exact --nocapture;
runner local /tmp/logos-grounding-mutations.py, logs no mesmo diretório.

| Bug inserido | Teste que falhou |
| --- | --- |
| tuple-preflight | tuple_coordinates_are_charged_before_creating_atom_arguments |
| missing-axiom | explicit_axioms_identity_general_and_provenance_are_checked |
| vocab-identity | explicit_axioms_identity_general_and_provenance_are_checked |
| quantifier-polarity | quantifier_polarity_outer_binders_and_false_premises_keep_consequence |
| ignore-premise | quantifier_polarity_outer_binders_and_false_premises_keep_consequence |
| fixed-values | values_are_total_congruent_and_additional_constants_are_not_distinct_names |
| primitive-uniqueness | values_are_total_congruent_and_additional_constants_are_not_distinct_names |
| defined-uniqueness | unused_defined_values_restrict_models_and_enumerated_ordinals |
| defined-parameters | different_sorts_and_de_bruijn_parameter_order_are_preserved |
| binder-index | quantifier_polarity_outer_binders_and_false_premises_keep_consequence |
| bytes-limit | resource_limits_are_inclusive_and_no_partial_program_escapes |
| premise-identity | explicit_axioms_identity_general_and_provenance_are_checked |
| equality-or | values_are_total_congruent_and_additional_constants_are_not_distinct_names |

TCB report target/tcb-4.2.json:10361+105199=115560 LOC Rust de correção,
+745 próprios; logos-ground1075 (antes331), build-only64508; SQLite C176466
separado. O total do crate inclui a classificação/candidatos4.1; a meta
específica grounder+Tseitin<1000 ainda será avaliada na4.3, sem promessa
antecipada de cumprimento. Verificação completa/selftest em execução,
ainda sem alegação de VERIFY_OK desta etapa.


Revisão posterior detectou que coordenadas repetidas nos argumentos de átomos
não eram cobradas em max_cells. Corrigido o pré-cálculo ponderado por aridade
antes da alocação; regressão fixa29 células inclusivas e recusa28/12, com
mutante compilado que restaura a contagem anterior capturado. A primeira
verificação da implementação foi interrompida143 para incorporar o ajuste,
sem ser contada como sucesso. Fuzz final repetido sobre o código corrigido:
11287 execuções/61s, exit0 sem crash, sanitizers ativos; log anterior novamente
restaurado. Log novo target/vocabulary-decode-4.2-final.log. Verificação
completa/selftest final em target/verify-4.2-final.log, ainda em andamento.


A rodada target/verify-4.2-final.log falhou101 na conferência spec/domains:
17 linhas na tabela,16 no catálogo RESERVED. A tag já existia, mas faltava
registrá-la também nesse catálogo. Incluída a entrada ground; os três testes
spec_domains passaram. CAS continua usando sua tabela própria, sem novo
namespace. TCB final10361+105199=115560 (+745 próprios); crypto644,
ground1075. Rodada completa seguinte em target/verify-4.2-complete.log.


A retomada após interrupção encontrou target/verify-4.2-complete.log sem
VERIFY_OK e nenhum processo verificador ativo; sessão anterior não pôde
fornecer exit. Não contada como sucesso. Nova execução integral supervisionada
em target/verify-4.2-resumed.log, com exit persistido em .exit ao terminar.

A4.3/4.7 deve preservar domain_axioms como suporte da transformação:
fecho/igualdade podem simplificar a premissa para tautologia no DAG; isso
não elimina a dependência da fonte. O artefato4.2 conserva todas as referências,
mas ainda não há Cnf/used/certificado ou claim que demonstre essa integração.


A retomada target/verify-4.2-resumed.log terminou1 na conferência global de
vetores: faltavam quatro linhas ground em spec/vectors/domains/hash.tsv.
Regeneradas pelo produtor independente;64 linhas anteriores preservadas
byte a byte,68 no total. Helper Rust de hash recebeu DGround/caso ground;
suíte crypto inteira e todos13 geradores --check passaram.

A rodada target/verify-4.2-final-all.log passou Rust/smokes/crash/escala e
vetores, mas falhou na suíte Python:225 passaram e2 falharam em91,72s.
Os testes de separação/prefixos fixavam16 tags de hash; expectativas e
comentário atualizados para17, preservando as verificações de unicidade,
NUL e prefixos. Contagem16 do CAS continua correta; comentários esclarecem
que o catálogo de namespaces é separado. Nova pré-verificação Python/TCB
antes de repetir a verificação completa.


Rodada retomada final ci/verify.sh --selftest: VERIFY_OK, exit0, persistido em
target/verify-4.2-validation.exit. Onze testes novos passaram no workspace,
227 Python em92.01s e25 TCB em1.32s. Smokes, manifesto/lints/independência,
supply-chain e vetores passaram; autoteste sem avisos, diagnósticos corretos.
Crash10k/24 SIGKILLs/10000 ACKs/64.36s sem perdas; reconstrução100k
preservou as cinco tabelas.1M/20000 consultas: p99=18402ns, cold rebuild
258591ms fora da latência. Primeiro critério4.2 demonstrado; determinismo
local demonstrado, critério remoto permanece desmarcado até observar CI.
