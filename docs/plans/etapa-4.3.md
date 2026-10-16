# Plano — Etapa 4.3: Tseitin canônico e proveniência

Estado: implementada/verificada localmente; determinismo remoto pendente. Base inspecionada: e738f78. Decisões em
[ADR0028](../adr/0028-tseitin-canonico-e-rederivacao.md); contrato de bytes
em [spec/cnf.md](../../spec/cnf.md). Não altera a ordem do roadmap.

## 1. Interfaces de entrada

- GroundProgram privado da 4.2: átomos ordenados, DAG topológico, raízes de
  restrições/premissas/goal e domain_axioms. Canon/Domain e limites existentes.
- Obligation selada de generate; leitor decode_for_check da 2.3 para auditoria
  não autoritativa. Acrescentar entrada por bytes/hash sem converter CheckedObligation
  em Obligation selada; compartilhar apenas a tradução lógica.
- CAS 0.5/3.5 e ObligationCreated 3.1: artifact é SHA-256 cru, ainda sem
  h(CNF)/versão. A ampliação versionada desses campos é entregável desta etapa.
- Auditor 3.6 independente: decoder próprio, leitor CAS e export. Acrescentar
  ground à sua árvore, sem ledger/kernel, para refazer a derivação.
- Z3 já instalado fora da TCB como solver de referência; nenhuma biblioteca
  SAT nova em Rust. Não depende de LRAT/checker/registry/kernel futuros.

## 2. Interfaces de saída

```text
clausify(&GroundProgram) -> Result<(Cnf, ProvenanceMap), GroundError>
clausify_with_limits(&GroundProgram, CnfLimits) -> Result<...>
Cnf: variables(), clauses(), dimacs(); Domain no domínio logos/cnf/v1\0
ProvenanceMap: origins() alinhadas aos IDs de cláusulas 1-based,
               domain_support() com referências requeridas pela transformação
ground_for_check(bytes, hO, V, DecodeLimits, GroundLimits) -> Result<GroundProgram,...>
ObligationCreated.cnf: Option<CnfRecord { hash, encoder_version }>
```

O artefato do evento mantém seu SHA-256 cru; CnfRecord acrescenta hash com
domínio e versão. Os bytes v1 históricos ficam idênticos quando cnf=None.
Auditor confere todo ObligationCreated/Lrat, recusa falta de metadados e
versão desconhecida; rederivação de CNF não é rechecagem de prova LRAT.

## 3. Tarefas em ordem e testes

1. Fixar este plano, ADR0028 e spec/cnf.md antes do código. Registrar 4.3
   em andamento no STATUS; nenhum critério marcado por planejamento.
2. Implementar encoder em logos-ground, listando fontes no manifesto:
   variáveis primitivas e auxiliares, equivalências completas de Tseitin,
   assertivas separadas, ordenação conjunta cláusula/origem, suporte global
   dos axiomas e limites inclusivos. Regressões pequenas exaustivas e mutantes.
3. Compartilhar entrada lógica do classificador/grounder com leitor contextual
   existente. Conferir h(O)/h(V), bytes e limites; teste compara artefato selado
   e releitura, e recusa hash/tipagem/bytes adulterados. Sem construtor selado novo.
4. Registrar domínio CNF, CAS, vetores independentes e limites. Atualizar todos
   os consumidores do catálogo (Rust/Python/geradores), preservando vetores antigos.
5. Versionar somente corpo ObligationCreated com CnfRecord; atualizar inventário
   CAS e decoder próprio do auditor. Testar assinatura, roundtrip, versões,
   ausência CAS antes de assinar, GC e histórico. Estender event_decode existente.
6. Auditor refaz O→ground→CNF com encoder conhecido, compara bytes/hash/artefato;
   testar todos os eventos de um histórico real e export, inclusive adulterações
   assinadas com objetos CAS íntegros. Preservar contador de proofs_unchecked.
7. Diferencial 5k: worker Rust gera casos, tabela verdade independente fornece
   resultado; script Python usa Z3 apenas como referência SAT, enumera casos
   pequenos e confere todas as cláusulas/proveniência. Sem sampling/Unknown aceito.
8. Integrar smoke/vetores/script na verificação e CI, estender fuzz existente,
   demonstrar mutações compiladas, medir LOC e executar ci/verify.sh --selftest.
9. Atualizar STATUS/diário e somente critérios demonstrados; commit local.

## 4. Critérios → evidência

| Critério | Teste/artefato |
| --- | --- |
| CNF unsat sse ground(O) válida, 5k | worker Rust + tools/check_cnf.py/Z3 e enumeração pequena |
| Auditor refaz h(CNF) em todos os eventos pertinentes | histórico com múltiplos ObligationCreated/Lrat, SQLite/export/CAS; mutação de cada posição |
| LOC grounder+Tseitin, meta <1000 | cargo xtask tcb report e contagem dos módulos; reportar valor real mesmo acima da meta |
| Ground→DIMACS/hash determinístico, vetores/CI | produtor Python independente, repetições Rust/smoke nas duas arquiteturas; CI permanece desmarcada até observação |

## 5. Riscos e decisões

domain_axioms é suporte da transformação, inclusive quando sua fórmula vira
tautologia: preservá-lo explicitamente no mapa para a 4.7. A lista é conservadora; precisão/alcance devem ser testados
na extração do certificado, sem união automática de todo o contexto a used.
Não inventar cláusula
de fato nem autenticação. Guardas de valores/Defined entram na CNF. Duplicatas e
tautologias são preservadas; nenhuma minimização/pré-processamento SAT.

Eventos antigos Lrat sem versão continuam decodificáveis para autenticação/export,
mas auditoria de derivação recusa a ausência em vez de assumir uma versão.
Outras rotas não têm CNF atribuída. Verified permanece sem recheck de prova até 4.5.
Limites lógicos e reservas falíveis não são garantia de RSS/wall-clock.

## 6. Estimativa

M, conforme roadmap. Integração com CAS/eventos/auditor é necessária ao contrato,
sem adicionar execução de solvers/checkers confiáveis ou dependência futura.

## 7. Evidência

Inspeção inicial: APIs 4.2/CAS/auditor presentes; h(CNF)/encoder_version ausentes
no evento atual. São saídas explicitamente exigidas pela 4.3, não dependências
futuras antecipadas. Nenhum teste da 4.3 executado durante esta preparação.


Receitas reproduzíveis das mutações: [etapa-4.3-mutacoes.json](etapa-4.3-mutacoes.json).
Cada entrada fixa fonte, substituição, comando e resultado; a primeira sobrevivente
também está registrada. Aplicar uma por vez e restaurar antes da próxima.


2026-10-15: **Etapa4.3 implementada; determinismo remoto pendente.** Plano,
  ADR0028 e spec/cnf.md fixados antes do código; dependências reais presentes.
  Tseitin completo sobre DAG inteiro, assertivas de constraints/Γ/¬goal,
  literais/cláusulas/origens ordenados juntos; duplicatas/tautologias preservadas.
  domain_support mantém fontes exigidas pela transformação sem inventar used.
  Leitura contextual para auditor reutiliza decoder sem criar Obligation selada;
  revisão manteve hash da entrada selada depois dos limites de classificação.
  Novo domínio/raw cnf-v1 (16MiB), h(CNF) com tag própria; artifact preserva
  SHA-256 cru. ObligationCreated usa corpo v2 somente com registro CNF/Lrat/
  encoder1.22 vetores históricos intactos,23 ao todo; CAS tem4 vetores (3 antigos
  preservados) e hash.tsv72 (68 antigos preservados). Auditor sem ledger/kernel
  rederiva3/3 CNFs no ledger real e após export sem SQLite; provas seguem
  proofs_unchecked. Adulterações assinadas/CAS íntegro em cada posição recusadas,
  assim como ausência/versão/metadata/artifact. CAS exige CNF antes do Signer;
  inventário/GC conserva raw sem ler como CBOR. Revisão corrigiu despacho GC
  fixo em16 índices, com regressão do último domínio e CNF retida.
  5k O distintas/Z3×tabela verdade completa:2247 válidas/2753 inválidas,
  zero divergências; Unknown não contado.12 vetores independentes de DIMACS/
  digest/origens, oito repetições por vetor;5 novas regressões do encoder.
  Primeiras compilações apontaram match do novo domínio e borrow em teste;
  Clippy apontou if aninhado; Ruff/mypy exigiram captura/tipagem explícitas e
  declaração de biblioteca Z3 sem stubs. Corrigidos. Pré-check inicial usou
  caminho relativo incorreto, corrigido; gerador CAS ainda esperava16 domínios,
  ajustado para17 e4 exemplos antes da verificação integral.
  Primeiro lote de mutações:4 capturadas, Or com sinal errado sobreviveu ao
  teste de unicidade. Oráculo reforçado para conferir valores de cada auxiliar
  contra avaliação independente do DAG; rodada final13/13 compiladas capturadas
  por asserções, fontes restauradas. Casos/comandos/substituições em
  target/cnf-mutations/results.json, primeiro lote preservado em
  target/cnf-mutations-first/; runner /tmp/logos-cnf-mutations.py. Enumeração
  integral dos vetores pequenos e DPLL de teste cruzado com enumeração cobrem
  contexto maior sem substituir o oráculo SAT5k. Teto inicial de enumeração
  do teste recusou um contexto maior; oráculo de teste ampliado, sem reduzir casos.
  Fuzz oficial/nightly-2026-09-28: vocabulary_decode9673 e event_decode132990,
  ambos61s/exit0 sem crash,23 seeds de evento; logs preexistentes restaurados.
  Nenhum parser DIMACS novo; extensão de decoder de evento exercitada no alvo.
  Primeira verificação integral falhou101 em replay histórico: a nova fixture
  CNF usava época7 após vetor u64::MAX. Época nova corrigida para a corrente,
  preservando os22 vetores anteriores. Replay já passou pela cadeia; asserção
  de head ainda fixava21 (22 eventos), ajustada22 para23 eventos e repetida. Fuzz de evento
  repetido sobre o corpus atualizado: 132990/61s sem crash; primeira
  rodada140616/61s também passou.
  Verificação integral ci/verify.sh --selftest: VERIFY_OK, exit0 persistido em
  target/verify-4.3-final.exit; Rust/Clippy/smokes/TCB/supply-chain/vetores passaram,
  227 Python em92.02s e25 TCB em1.46s; autoteste sem avisos.
  Crash10k/24 SIGKILLs/9997 ACKs/63.32s sem perder append reconhecido; rebuild100k/cinco
  tabelas exatas;1M/20000 consultas p99=21392ns, cold rebuild263042ms
  fora da latência. TCB Rust10686+105199=115885 (+325 próprios), build-only64508;
  ground1337/ledger3271/crypto645; SQLite C176466 separado. Núcleo ground/dag/
  prop/cnf983 LOC;354 LOC restantes de classificação/axiomas/infraestrutura
  continuam no relatório do crate, sem alegar TCB total abaixo de1000.
  Nenhum pacote externo novo; única aresta runtime nova audit→ground registrada
  no ADR/deny/lock. Critérios1–3 da4.3 marcados; determinismo remoto4.3/4.2,
  ARM e fuzz longo pendentes. Fontes/memória/logs alheios e mutants.out preservados.
  Sem push; próximo:4.4, checker LRAT, concordância cake_lpr após4.5.
