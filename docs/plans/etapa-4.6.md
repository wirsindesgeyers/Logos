# Etapa 4.6 — produtor SAT fora da TCB

Base: a6d3f37. Estado: concluída localmente em2026-10-17; iniciada em2026-10-16.
Verificação completa com selftest: VERIFY_OK/exit0. CI remota não observada.

## 1. Interfaces de entrada

Obligation/v1 e Vocabulary/v1 canônicos, encoder Python0.3; ground_for_check e
clausify4.3 com tabela de átomos/ground-v1; LratChecker4.4, CakeRegistry4.5
e ModelChecker2.3. Dependências presentes. CaDiCaL3.0.1/revisão
c60730422e758ef1cebe7aeddf2dda31c996bf04 já produziu o corpus4.4; sua fonte
e receita serão fixadas para build local/CI, fora da TCB.

## 2. Interfaces de saída

logos_provers.sat.produce(obligation:bytes, vocabulary:bytes, *, encoder:Path,
solver:Path, limits:Limits) -> Candidate. Candidate distingue UNSAT (LRAT),
SAT (model/v1) e EXHAUSTED, com CNF/identidades e motivo explícito quando
inconclusivo. Nenhum candidato emite Verified/Refuted, evento ou autorização.
O encoder é um executável de apoio Rust, compilado como exemplo, que compõe
as APIs existentes sem acrescentar runtime à TCB. Resultados de checkers
reais só entram na validação/smoke, nunca como certificado gerado pelo solver.

## 3. Tarefas em ordem

1. ADR0032: fixar solver/revisão, formato LRAT textual, eventual adaptação
   de cauda e protocolo de apoio não autoritativo. Sem mudar cnf/v1/IDs.
2. vendor/cadical e tools/build_cadical.py: build offline da fonte fixada,
   licença/hash/receita. Testar binário real SAT/UNSAT e identidade da fonte.
3. Exemplo Rust de apoio: V/O→ground/CNF usando APIs4.3; protocolo JSON
   documentado e limitado. Sem novo parser/decoder no runtime da TCB.
4. py/logos_provers/sat.py: subprocessos sem shell, timeout/tamanho/memória,
   cleanup, saída SAT completa e LRAT textual. Reconstrução do FiniteModel
   via átomos primitivos, representantes Enum, tabelas totais e omissão de
   Defined, com controles de saída inválida. Testes negativos e solver real.
5. Corpus reprodutível de300 obrigações válidas de política, mais pares SAT
   e controles de funções/Defined/Open/singleton/enum vazio. Obrigações
   explícitas de teste, sem alegar Policy/fatos autenticados inexistentes.
6. Smoke/script: cada CNF real→CaDiCaL→LRAT→dois checkers; SAT→ModelChecker.
   Não filtrar casos após observar falhas, não contar EXHAUSTED como sucesso.
   Fixar denominadores/variedade/identidades e registrar observações.
7. Integrar build/smoke/testes à CI e ci/verify.sh/AGENTS; Ruff/mypy/pytest,
   medição TCB e verificação completa com selftest. Atualizar STATUS/ROADMAP/
   diário e commit local somente após VERIFY_OK; sem push.

## 4. Critérios → evidência

| Critério | Teste/script |
| --- | --- |
| ≥95% de300 O→CNF→CaDiCaL→LRAT→cake_lpr | Corpus fixo gerado antes de executar solver; smoke conta todos300, exige ≥285 aceites externos e também confere Rust. Relatório com hashes e resultados. |
| 100% SAT aceitos na camada0 | Pares SAT do corpus e controles de reconstrução conferidos por ModelChecker real, sem fallback Z3/finder. Mutação do modelo/CNF/prova e solver falso devem expor erros. |

## 5. Riscos e decisões

ADR0032 registra decisões antes do código. cake_lpr/Rust diferem em duplicatas
e IDs esparsos; não normalizar CNF para esconder isso. Solver pode produzir
RAT/cauda fora do perfil Rust: adaptação só fora da TCB, preservando CNF/IDs
originais e sempre rechecando bytes finais. SAT não garante modelo completo
ou correto; converter tabelas e exigir checker na validação. Domínios Open
não usados podem ser completados com singleton; Defined é derivado pelo
checker, não armazenado. Enum vazio não tem modelo clássico não vazio.
Sem portfólio/cache10.2, used_premises4.7, kernel5.4 ou fontes autenticadas.
Pendências remotas4.5 são evidência, não APIs ausentes.

## 6. Estimativa

M em vez de P: build offline e reconstrução contextual/SAT exigem controles
de funções/Defined e integração com três checkers. Escopo/ordem preservados.


## 7. Evidência em andamento

- Plano/ADR0032 registrados antes do código. Fonte upstream integral fixada
  por git archive/XZ (577340bytes), SHA25657943c9d7305…; build offline real
  com g++/make passou. Nenhum solver/conversor/dependência entrou na TCB.
- Apoio composto como exemplo logos-audit/prover_bridge, usando dependências
  existentes; nenhum src/ de crate Rust alterado. prover-bridge/v1 documentado
  com vetores derivados do vetor CNF Python independente e versão recusada.
- Exploração inicial: sintaxe infixa iff não era válida (usar <->); o controle
  Open com Const/Func de resultado Open é General pelo contrato4.1. Corrigido
  para primitivos com entrada Open/resultado Enum, sem ampliar classificador.
  Primeiros controles reais aceitaram UNSAT nos dois checkers e modelos
  nullary/singleton/Defined/Open no ModelChecker. Enum vazio também UNSAT.
- Corpus fixado antes da execução:300 válidas/300 pares SAT,10 famílias,
  cinco condições de contexto exigidas no goal,30 valorações, domínios2–4.
  Pares SAT conferem também a consistência das300 premissas válidas.
- 23 testes Python passaram inicialmente em4.08s; após controles de tabelas
  de dois sorts/resultado não zero/representante Defined, um teste revelou
  corrida na própria observação de /proc: processo desapareceu entre exists
  e read. Leitura agora trata FileNotFound como processo já encerrado, e
  monitora até1s para entrega do sinal.23 passaram em4.67s. Nenhuma falha
  lógica foi escondida; processo morto/zombie não é contado como executando.
- Ruff/format/mypy do produtor/testes e strict dos dois tools passaram;
  Clippy do apoio passou. Corpus completo em execução; ainda sem full verify.
  O primeiro mypy apontou listas heterogêneas das fórmulas AIR; anotações
  canon.Value corrigiram a tipagem, sem alterar bytes. Ruff inicial exigiu
  suppress para processo já encerrado, default Limits fora da assinatura
  e divisão/formatação de linhas; corrigidos antes da verificação integral.
- Medição TCB target/tcb-4.6.json/.md:11549+747130=758679 Rust de correção,
  build-only64508, exatamente igual à4.5. Nenhum parser novo no runtime TCB,
  portanto nenhum novo alvo fuzz ou mutação de fonte TCB nesta etapa.

- Primeira execução integral do corpus:300/300 provas aceitas por Rust e
  cake_lpr,300/300 contramodelos aceitos e300 conjuntos de premissas
  consistentes;300 CNFs válidas distintas. Cinco controles iniciais também
  passaram (301 provas/304 modelos). Acrescentados depois dois controles de
  tabelas com resultados não zero/dois sorts/func nullary e representante
  Defined: ambos passaram no teste real. Corpus agora tem607 casos e907
  recheques; a verificação completa repetirá o pipeline com todos os sete.
- Testes negativos adicionais recusam CNF substituída e SAT completo que não
  satisfaz CNF; subprocesso real com import Z3 bloqueado ainda produz SAT.
  Rodada final direcionada:25 testes em5.23s. Ruff/mypy passaram; E501 em
  string da fixture foi dividido sem alterar seu programa. Full com selftest
  em target/verify-4.6-final.log, aguardando VERIFY_OK/exit0 antes do commit.
- Revisão do ApêndiceD identificou a métrica bytes/certificado por O/camada.
  Somente o relatório sat_smoke recebeu campos layer/certificate_bytes/
  cnf_bytes; produtor/checkers não mudaram. A full já havia executado o
  pipeline lógico com sete controles (301 provas/306 modelos/300 consistências).
  O corpus está sendo repetido para demonstrar a emissão desses metadados
  finais em target/sat-producer-metrics-report.json, log/exit em
  target/sat-4.6-metrics.log/.exit. Lints/Python da full ainda são posteriores
  a essa alteração. Não alegar que a primeira execução já continha a métrica.
- Revalidação com métricas finalizada:177.15s, exit0;607 registros/907
  recheques,301 provas/306 modelos/300 consistências. Comparação dos campos
  anteriores com o relatório da full: zero diferenças. Relatório final
  publicado também em target/sat-producer-report.json, SHA256
  9c9031fef94e481f042e10ead7f18edde15b6c09462c01bf0bc60a19b554a1ec.
  Campos layer/certificate_bytes/cnf_bytes válidos em todos os registros.
  Incluindo sete controles: camada0,306 modelos de43–121bytes, mediana111,
  total33583; camada1,301 provas de37–6727bytes, mediana2485,total861151.
  Produtor e checkers permaneceram inalterados durante essa revisão do relatório.
- Verificação completa final: ci/verify.sh --selftest terminou em VERIFY_OK,
  exit0 persistido em target/verify-4.6-final.log/.exit;1981.19s. Workspace
  Rust/Clippy/smokes/manifesto/lints/independência/supply-chain/vetores passaram;
  252 Python em92.87s e25 testes TCB em1.35s; autoteste bloqueou todos os
  controles pelos diagnósticos corretos. Cargo-deny manteve avisos conhecidos
  de licença não encontrada/syn duplicado; RTK também avisou sobre ausência
  de hook global, sem falha. Não instalar hook global fora do escopo.
  Corpus legado200/600 concordou em446.75s. Crash10k/24 SIGKILLs/10000 ACKs
  em111.11s sem perdas; rebuild100k/cinco tabelas exatas;1M/20000 consultas
  p99=41740ns, cold rebuild421536ms excluído da latência. TCB inalterada.
  Critérios4.6 demonstrados localmente; próximo4.7. Pendências remotas4.5/
  CI/ARM/fuzz longo anteriores preservadas, sem push.
