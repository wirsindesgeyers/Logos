# Etapa 4.4 — checker LRAT próprio

Base: 37c544e. Escopo: somente RUP, deleções e cláusulas originais usadas.

## 1. Interfaces de entrada

CNF cnf/v1 e Hash<Cnf> da4.3, IDs originais 1-based do ProvenanceMap;
Budget/StepMeter/CheckOutcome da0.6; Text/Reason e hash_raw existentes.
As dependências existem. Concordância cake_lpr depende explicitamente da4.5.

## 2. Interfaces de saída

LratChecker::new() -> Result<LratChecker, CanonError> prepara razões falivelmente,
como ModelChecker. O método check_lrat(cnf_bytes, h_cnf, proof_bytes, &Budget)
-> CheckOutcome implementa o contrato; check_with_used com os mesmos argumentos
-> LratReport também fornece Option<&[u64]> de IDs originais, somente se Accepted.
Nenhum Verdict/claim ou used_premises é emitido.

## 3. Tarefas em ordem

1. ADR0029 e spec/lrat.md: perfil textual lrat-rup/v1, parser cnf/v1,
   P2, ordenação/IDs, deleções, limites e fecho; vetores independentes.
2. crates/logos-lrat: parser ASCII estrito, banco de cláusulas com IDs esparsos,
   RUP sequencial e deleções, medidor único e reservas falíveis. Manifesto TCB.
   Testes de gramática, identidade, passos/tamanho/memória, duplicatas/tautologias.
3. Fecho iterativo a partir da cláusula vazia; preservar histórico de dependências
   mesmo após deleção. Testar cadeia, ramos irrelevantes e IDs grandes.
4. Corpus reproduzível com >=200 benchmarks pequenos/médios e provas RUP;
   documentar fontes/parâmetros e digests, sem confundir benchmarks gerados com
   instâncias históricas da competição. Corromper hint/cláusula/vazia por caso.
5. Fuzz dos dois parsers, mutações compiladas do checker, smoke/CI/vetores;
   cargo xtask tcb report e ci/verify.sh --selftest antes do commit.
6. STATUS/roadmap/diário com evidência real; commit local, sem push.

## 4. Mapeamento do critério

- >=200 instâncias: teste corpus, manifesto com origem/identidade e comando
  reproduzível; cardinalidade e unicidade dos bytes conferidas.
- 100% corrompidas: três mutações semânticas por certificado, mais gramática,
  P2, deleção/reuso/forward refs e SAT negativo. Testes têm mutações compiladas.
- Concordância cake_lpr: permanece aberta até4.5, sem wrapper nesta etapa.

## 5. Riscos e decisões

ADR0029 fixa perfil RUP e rejeição explícita de RAT negativo/binário; não
apresentar suporte LRAT completo. CNF preserva duplicatas e tautologias;
propagação conta literais distintos. Não parar no primeiro vazio ignorando
uma cauda inválida. Deleção não apaga as arestas de dependência históricas.
Orçamento lógico não é RSS nem tempo rígido. IDs esparsos não dimensionam arrays.
Nenhum produtor público SAT/integração CAS/auditor/cake_lpr nesta etapa.

## 6. Estimativa

M, conforme roadmap: parser e checker pequenos, mas corpus, adversários,
fuzz e comprovação por mutação são parte necessária da entrega.

## 7. Evidência de execução

- LratChecker prepara Reason falivelmente e recebe os quatro argumentos do
  contrato nos métodos check_lrat/check_with_used. Nenhum Verdict/claim é emitido.
  Perfil lrat-rup/v1 e parser cnf/v1 documentados; RAT/DRAT/binário/LPR/PR recusados.
- 15 testes Rust:12 regressões, vetores, diferencial exaustivo e corpus. Cobrem
  P2/domínio, IDs esparsos/u64::MAX, sintaxe, deleções/reuso/hints futuros,
  caudas inválidas após vazia, duplicatas/tautologias, limites inclusivos e
  resultados sem used em rejeição/exaustão. Cadeia de5000 deps em pilha64KiB.
- Diferencial:256 CNFs sobre duas variáveis/28160 traces de hints,1792
  aceitações e26368 rejeições. Toda aceitação e seu subconjunto usado são
  UNSAT pela enumeração independente de quatro valorações.
- Corpus histórico SAT-2002:200 fontes e CNFs distintas,22 famílias,
  20–7767 variáveis/70–91307 cláusulas. Entradas20770401 bytes e provas223146820
  bytes; JSON249940618 bytes em XZ56171972 bytes. Nomes, hashes originais,
  CNF/certificado, opções/revisão do CaDiCaL e hashes das provas brutas estão
  em spec/vectors/lrat/corpus-manifest.json. O produtor externo só prepara
  fixtures; não há logos_provers.sat nem dependência de solver na TCB.
- 200/200 aceitas e600/600 corrupções rejeitadas, used exato contra oráculo
  Python independente. Execução inicial582.77s; reservas geométricas e erros
  antecipados no harness253.70s; perfil test nível1 com overflow-checks e
  debug-assertions ativos98.60s isoladamente. Full não usa esse tempo como
  limiar: custo depende do host.14 vetores manuais/hash/used independentes.
- Onze mutações compiladas capturadas no código final/perfil test: P2,
  hints apagados, omissão de RUP, falsa unidade, polaridade, cauda ignorada,
  fecho excessivo, história apagada, tamanho inclusivo, cobrança de bytes e
  memória. A de passos sobreviveu inicialmente; comentário10k/budget1000
  reforçou o teste. Receitas em etapa-4.4-mutacoes.json, logs em
  target/lrat-mutations/*-final.log; compilação e assertion failure exigidas.
- Fuzz final oficial ci/fuzz.sh/nightly-2026-09-28: cnf_decode844911 e
  lrat_decode533686 execuções em61s cada, exit0 sem crash. Logs
  target/fuzz-4.4-final.log e target/fuzz-4.4-*-final.log. A rodada inicial
  anterior às reservas geométricas teve327537/275844 execuções sem crash.
  Matriz remota descobre os16 bins, um por job/max-parallel4;720min em
  sequência não cobririam16h. Hora remota por alvo permanece não observada.
- TCB: logos-lrat417 LOC, quatro fontes listadas no manifesto; acréscimo415
  próprios sobre o esqueleto. Correção11101+105199=116300 Rust, build-only64508,
  SQLite C176466 medido à parte. Relatórios target/tcb-4.4.json/.md, base37c544e
  com árvore modificada. Nenhum pacote externo novo; serde_json só dev.
- Primeira verificação completa: exit1 em Ruff, depois de Rust/smokes/CNF/
  auditor/crash/índices/TCB/deny/audit passarem. Duas quebras de linha dos
  scripts novos divergiam da configuração de py/. Corrigidas rodando Ruff
  em py/; check/format globais e mypy dos scripts passaram. Log preservado
  em target/verify-4.4-attempt1.log/.exit. Rodada final completa: ci/verify.sh --selftest terminou VERIFY_OK/exit0;
  227 Python em138.23s e25 TCB em2.64s, autoteste sem avisos de tar. Rust/smokes/
  CNF5k/auditor/manifesto/independência/deny/audit/vetores passaram. Crash10k/
  24 SIGKILLs/9999 ACKs/108.18s sem perda reconhecida; rebuild100k exato;
  1M/20k consultas p99=48538ns, cold build539292ms. Log/exit finais em
  target/verify-4.4-final.log/.exit. Primeiro e segundo critérios demonstrados.
- Concordância cake_lpr fica aberta até4.5 conforme o contrato original.
  Nenhum wrapper/registro/auditor LRAT, produtor público4.6 ou used_premises4.7.
