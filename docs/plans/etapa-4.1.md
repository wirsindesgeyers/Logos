# Plano — Etapa 4.1: classificador e candidatos de axiomas

Contrato: proposta A do [ADR0026](../adr/0026-contrato-de-fragmentos-e-fontes-de-fechamento.md),
adotada explicitamente pelo mantenedor em2026-10-15. ROADMAP4.1/4.2/5.4/6.1/
7.3/7.5 ajustados antes do código. O bloqueio inicial foi resolvido por divisão
explícita de responsabilidades, sem fabricar dependências futuras.

## 1. Entradas

Obligation selado/generate da1.5; Vocabulary/DomainSource/AIR da1.2 e sort-check
1.3. Modelos/avaliador2.2 somente para testes semânticos. Fonte de enumeração
continua opaca; nenhum vínculo payload→evento ou fonte autenticada é assumido.

## 2. Saídas e assinaturas

Em logos-ground:

```text
classify(&Obligation,&Vocabulary) -> Result<Fragment,GroundError>
classify_with_limits(&Obligation,&Vocabulary,AnalysisLimits) -> Result<Fragment,GroundError>
domain_axioms(&Vocabulary,&Sort) -> Result<AxiomCandidates,GroundError>
domain_axioms_with_limits(&Vocabulary,&Sort,AxiomLimits) -> Result<AxiomCandidates,GroundError>
Fragment = FiniteDomain | General
```

AxiomCandidates tem campos privados/getters para h(V), sort, DomainSource,
closure e distinct. Sem Canon/Domain/Obligation/PremiseRef/claim/conversão
para premissa. Saída lógica tipada não demonstra autenticação da fonte.

## 3. Tarefas e fechamento

1. Contrato/ADR/ROADMAP: preservar os critérios integrados nos destinos6.1/
   7.3/7.5 e exigir recusa de fontes não resolvidas no uso autorizador5.4.
2. Classificação TCB em classify.rs: conferir h(V), limites de entrada,
   todas as assinaturas/Defined (binders como no avaliador), premissas e goal.
   Todo V é analisado; Const/Func com resultado Open e parâmetros Defined
   Open ou quantificadores Open exigem General. Erro não é fallback.
   Testes: matriz de primitivos/Defined, trânsito por definições não usadas,
   escopos/sorts mistos, identidade, invariância entre premissas/goal, limites.
3. Emissor TCB em axioms.rs: enumeração vazia/singleton/múltipla, desigualdades
   i<j, ordem original, contagem checada antes de alocar a expansão quadrática.
   Limites de constantes/axiomas/nós agregados e AIR por fórmula; reservas
   explícitas falíveis e erro Unknown(Size/Memory), sem teto físico de RSS.
   Testes: AST/vetores exatos, fontes Fact/Policy/h(V), não mutação de O,
   caps inclusivos, sort desconhecido/Open, limites de profundidade.
4. Semântica por avaliador independente: controle Open com mesmas assinaturas,
   enumeração de cardinalidades/interpretações de constantes. Conferir fecho
   sse imagens cobrem domínio, nomes únicos sse imagens são distintas. Contra
   Enumerated, modelos já têm esses invariantes e servem de controles positivos.
5. Fuzz vocabulary_decode existente exercita candidatos e classificação sobre
   V arbitrário/estruturado, sem parser novo. Manifesto/allowlist/locks atualizados.
6. Mutantes compilados para hash/contexto/quantificadores/Defined/constantes
   e gerador de axiomas/limites/fonte. Fontes restauradas; reprovação por teste.
7. Smoke público e vetores AIR reproduzíveis, CI/verify/AGENTS, spec/README,
   full verify/selftest, relatório TCB, STATUS e commit coerente.

## 4. Critérios → evidência

- Classificador: testes com generate real e entradas de V erradas/limitadas;
  não construir O por decoder ou função pública nova.
- Candidatos: vetores AST/bytes/hash e enumeração semântica com logos-model;
  fonte preservada e caps antes de expansão. Nenhum teste finge fonte real.
- Integrações: itens originais mantidos desmarcados em6.1/7.3/7.5 e com
  dependências explícitas. Verificação documental; não simular used/claim.

## 5. Riscos/decisões

Critério conservador global é deliberado: símbolos/Defined não usados também
podem exigir General; não há decisão por relevância nem expansão recursiva.
Enum vazio gera ∀x.Or(0), candidato inconsistente em domínio clássico não vazio,
nunca autorização ou Witness. Bound de saída não mede RSS/tempo e hash de V
pode alocar bytes canônicos. Fórmulas seguem AIR/v1; nenhum wire mudou.
Sem grounding, completion de predicado, fato/política/autenticação ou kernel.

## 6. Estimativa

M (antes P): dois núcleos TCB, validação global, limites e semântica/mutações.
Sem reordenação de fases e sem antecipar testes integrados.

## 7. Evidência

Análise inicial documentada em ADR0026 e snapshot target/contract-4.1-audit.json.
Verificação somente documental iniciada antes da decisão foi interrompida
(exit143), não contada como sucesso; a execução final cobriu a implementação.
Dez testes novos passaram. Oito vetores independentes AIR/h(V)/h(Formula)
passaram; Ruff/format/mypy estrito do gerador passaram. Smoke público passou:
FRAGMENT_SMOKE_OK cases=8 candidates=16 source=opaque claims=0. Semântica:
288 interpretações Open de controle, além de controles Enumerated positivos;
fecho e nomes únicos conferidos separadamente, sem alegar fonte autenticada.
Clippy inicial pediu .all direto no teste semântico; simplificado, sem allow.
Clippy workspace e fmt finais passaram antes da verificação completa.

Fuzz oficial vocabulary_decode/nightly-2026-09-28, CARGO_NET_OFFLINE=true:
18372 execuções/61s, exit0, sem crash, sanitizers ativos/max_len4096. Alvo
existente estendido com V arbitrário/estruturado, candidatos e classificação.
Log novo target/vocabulary-decode-4.1.log; log preexistente restaurado byte a
byte. Não há parser/decoder novo na TCB, nenhum formato/domínio novo.

Doze mutantes compilados capturados por testes, um de cada vez; fontes
restauradas em finally. Reproduzir cada substituição e executar cargo test
-p logos-ground --test fragment NOME -- --exact. Comandos/logs/resultados em
target/fragment-mutations/results.json; runner local /tmp/logos-fragment-mutations.py.

| Bug inserido | Teste que falhou |
| --- | --- |
| Ignorar h(V) | vocabulary_identity_and_input_limits_are_errors_with_inclusive_boundaries |
| Ignorar Exists | open_quantifiers_in_goal_or_premise_are_general_but_unused_open_sort_is_allowed |
| Analisar só goal | open_quantifiers_in_goal_or_premise_are_general_but_unused_open_sort_is_allowed |
| Ignorar Defined | defined_bodies_and_implicit_parameters_are_examined_globally_without_recursion |
| Ignorar parâmetros Defined Open | defined_bodies_and_implicit_parameters_are_examined_globally_without_recursion |
| Ignorar Const Open | all_value_symbols_are_checked_including_unused_constants_and_nullary_functions |
| Não cobrar aridades | input_budget_counts_enumerations_and_signature_arities |
| Fecho existencial em lugar de universal | exhaustive_open_control_models_check_closure_and_names_independently |
| Igualdade positiva em lugar de desigualdade | exhaustive_open_control_models_check_closure_and_names_independently |
| Perder tipo/bytes de fonte | independent_air_vectors_preserve_order_source_identity_and_exact_hashes |
| Omitir pares finais | independent_air_vectors_preserve_order_source_identity_and_exact_hashes |
| Ignorar teto agregado de saída | candidate_limits_are_inclusive_and_refuse_before_quadratic_expansion |

TCB report target/tcb-4.1.json:9616+105199=114815 LOC Rust de correção;
ground331 (antes2), +329 próprios, build-only64508. SQLite C176466 separado.
Novas arestas somente locais; nenhum pacote externo novo.

Verificação final ci/verify.sh --selftest: VERIFY_OK, exit0. Workspace Rust,
smokes, manifesto/lints/independência, supply-chain e vetores passaram;
227 Python em90,46s e25 TCB em1,39s. Autoteste sem avisos, controles
bloqueados pelo diagnóstico correto. Crash10k/24 SIGKILLs/9998 ACKs/64,41s
sem perda reconhecida; rebuild100k preservou as cinco tabelas;1M eventos/
20000 consultas, p99=19309ns, cold rebuild255842ms fora da latência.
Os três critérios genéricos4.1 foram demonstrados localmente. CI remota/ARM
e ≥1h de fuzz não foram observados; integrações6.1/7.3/7.5 continuam abertas.
