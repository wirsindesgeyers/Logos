# Plano da Etapa4.7 — dependências extraídas do certificado

Estado: concluída localmente; CI remota/ARM/fuzz longo pendentes. Base709dc45; 2026-10-17.
Escopo ROADMAP4.7; decisões em ADR0033. Nenhum kernel/handler/cascata novo.

## 1. Entradas

- ground/ground_for_check, GroundProgram e domain_axioms4.2;
  clausify/Cnf/ProvenanceMap e origens4.3.
- LratChecker::check_with_used e IDs do fecho histórico4.4;
  CakeRegistry4.5 para conferir os certificados de integração.
- PremiseRef, Obligation selada/generate, Budget/StepMeter;
  ledger/CAS/índice3.3 e EventBody::Verified existentes.
- Nenhuma dependência ausente. O teste de deps usa o ledger já implementado.

## 2. Saídas

- ProvenanceMap com h(O)/h(CNF) e suporte por cláusula, privado e imutável.
- LratReport com identidade da CNF/certificado aceitos.
- used_premises(&LratReport, &ProvenanceMap, Hash<Obligation>, &Budget)
  -> Result<UsedPremises, UsedError> em logos-lrat. Conjunto ordenado por
  PremiseRef; getters de referências e identidades. Não emite claim/veredicto.
- Metadados de suporte apenas em memória; bytes e versões existentes intactos.

## 3. Tarefas e testes

1. Suporte contextual de traduções em logos-ground/src/support.rs, ground.rs,
   prop.rs e cnf.rs; manifesto. Sorts/termos/Defined transitivo sem recursão,
   limites antes de alocar. Testes de domínio irrelevante, enum tautológico,
   sorts independentes, Defined e constraints; vetores antigos idênticos.
2. Identidades em LratReport e projeção em logos-lrat/src/used.rs, lib.rs,
   checker.rs; Cargo/deny/lock/manifesto. Testes de contextos misturados, IDs
   zero/fora/duplicados, rejeição/Unknown e limites inclusivos sem parcial.
3. Teste integrado logos-ledger/tests/used.rs com ledger/CAS/índice reais: cinco
   assumptions citadas, LRAT usa duas, cake_lpr também aceita; deps contém
   exatamente duas. Sem a necessária, não há prova: replay do certificado
   rejeitado e valoração independente satisfaz CNF/premissas e nega objetivo.
   Harness monta evento existente; não antecipa handler autorizador.
4. Mutações manuais compiladas para origens, suporte, identidades, fecho e
   orçamento; resultado reproduzível em docs/plans/etapa-4.7-mutacoes.json.
   Ampliar fuzz lrat_decode/vocabulary_decode; script oficial/nightly datado.
5. Specs/plano/STATUS, verificação completa ci/verify.sh --selftest, medição
   TCB. Marcar só critérios demonstrados; commit português, sem push.

## 4. Critérios → evidência

- Cinco citadas/duas usadas → teste de integração com dependents/índice real
  e consulta deps, fecho da prova conferida e conjunto extraído.
- Premissa necessária omitida → obrigação realmente gerada sem ela, rejeição
  do certificado antigo/tentativa de ID escondido e contraexemplo independente.
- Regra global TCB → vetores preservados, testes negativos, mutações detectadas,
  fuzz oficial e full verify/selftest. CI/ARM/fuzz longo só após observar.

## 5. Riscos e decisões

ADR0033 separa suporte contextual do conjunto global. A projeção é conservadora
nas fórmulas e na assertiva indivisível de constraints; não promete minimização.
Falsificação de mapa é impedida por campos privados/rederivação; o consumidor
confere h(O) esperado. Hash CNF sozinho não identifica nomes das premissas.
Recursos da checagem e projeção são separados e explícitos, sem reset oculto.
Preservar arquivos sujos de memória/fuzz/logs/mutants.out anteriores à sessão.

## 6. Estimativa

P mantida: suporte contextual e projeção usam interfaces existentes, sem wire,
parser ou evento novos. A verificação completa inclui corpus/índices demorados.

## 7. Diário e evidências

- 2026-10-17: dependências e contrato examinados. Ledger existente permite
  testar deps sem kernel; ADR0033/plano preparados antes de código runtime.
- Revisão de dependências encontrou que pôr o teste de ledger no logos-lrat
  criaria aresta dev transitiva proibida para o auditor. Teste real passou
  inicialmente, mas foi movido ao crate logos-ledger; guard não é relaxado.
- Testes direcionados: CNF5/5, checker12/12 e grounding26/26 (diferencial5k
  incluído) passaram; casos novos10 LRAT+1 ledger+1 unitário passaram. A
  mudança de crate exigiu corrigir imports; Clippy detectou carga duplicada
  do módulo de suporte, corrigida por import único compartilhado, sem allow.
- Estado após correções:13 regressões novas (11 LRAT/1 unitária/1 ledger),
  Clippy -D warnings e guard de independência passaram.14/14 mutações
  compiladas detectadas, fontes restauradas; logs em target/used-mutations/.
  TCB medida:12004 próprios+747130 runtime=759134 (+455 próprios),
  build-only64508; ground1534/lrat1123. Nenhum pacote externo novo.
- Fuzz oficial FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 lrat_decode
  vocabulary_decode: ambos61s/exit0, sem crash; logs anteriores restaurados
  byte a byte, resultados em target/fuzz-4.7/. Iniciada verificação integral
  ci/verify.sh --selftest; ainda não contada como sucesso.
- Revisão final conferiu §4.5 da spec: extração e suporte preservam a cadeia
  O→CNF→certificado; registro autorizador continua na5.4. Snapshot de19
  entradas de código/configuração permaneceu idêntico durante a verificação.
  Na rodada integral, diferencial5k do grounding passou em102.88s; corpus
  histórico LRAT ainda em execução, sem resultado final contado.
- Verificação integral final ci/verify.sh --selftest: VERIFY_OK/exit0 em
  2245.4s;252 Python +25 TCB, Rust/Clippy/smokes/manifesto/independência/
  supply-chain/vetores/autoteste passaram. Log/exit em target/verify-4.7-final.*.
  Corpus200/600:546.38s; SAT300/300 provas+300/300 modelos+300 consistências,
  sete controles,607 casos/907 rechecks. Crash10k/24 SIGKILLs/9998 ACKs em
  107.64s sem perdas;100k/cinco tabelas exatas;1M/20000 consultas p99=36897ns,
  cold rebuild430479ms fora da latência. Critérios demonstrados, próximos
  contrato/plano5.1; CI remota/ARM/≥1h e segundo host4.5 não observados.
