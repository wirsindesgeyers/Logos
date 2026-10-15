# Plano — Etapa 3.6: auditor independente

Escopo: ROADMAP3.6, S1; ADR0025. Sem LRAT4.5/transparência13.2.

## 1. Entradas

Canon/crypto0.3/0.4, checkers2.3, spec/events.md (22 vetores), cas/v1,
ledger-sqlite/v1 e epochs. Checker puro existe; acoplamento de transporte
model -> ledger será removido sem substituir dependência ausente.

## 2. Saídas

ModelChecker::check_refutation_from_source/check_consistency_from_source
recebem bytes/h(O), closure FnOnce(&Hash<FiniteModel>)->Result<Option<Vec<u8>>,E>,
h(M), Budget e retornam Result<CheckOutcome,E>. API transporta erro sem
autenticar fonte, conferindo hashes de O/M dentro do checker.

logos-audit verify LEDGER --cas DIR --public-key ED25519_HEX;
logos-audit export LEDGER --public-key ED25519_HEX.
LEDGER é diretório SQLite, arquivo SQLite ou audit-stream/v1. Export em stdout
binário; relatórios em stderr no export. Biblioteca expõe verificação/export
para consumo não autoritativo. Nenhum Verified/token é emitido.

## 3. Tarefas e testes

1. Separar leitura/checker (model, testes/smoke ledger, chamadas CI/docs).
   Testes de CAS reais preservados; fonte enganosa e limites exercitados;
   mutações de hash/budget detectadas. Guard all-features/dev permanece forte.
2. Decoder independente de catálogo/envelope/épocas em audit;22 vetores e
   fixture independente. Rejeição de desconhecidos, limites, resultados,
   conjuntos e consistência; não confiar em assinatura para estrutura.
3. Leitores próprios SQLite/CAS e export framing. Testar schema/WAL/lock,
   arquivos regulares, corrupção/ausência/limites, EOF/truncamento e replay.
4. Recheck100% dos Refuted e Witness com bytes autenticados e Budget
   registrado; teste aceita assinatura mas recusa modelo logicamente falso.
5. Comandos/relatórios, ≥10 adulterações, interoperabilidade real com appender,
   independência cargo tree e guard CI; export roundtrip sem banco original.
6. Spec/vetor export, README/ROADMAP/STATUS, full verify/selftest e commit.

## 4. Critérios → evidência

- 100% Refuted: fixture com múltiplas refutações distintas, contagem exata,
  uma refutação inválida em cada posição causa falha.
- ≥10 adulterações: remoção intermediária/reordenação/seq/prev/epoch/chave/
  assinatura/formato/resultado/CAS/modelo/certificado e limites no processo real.
- Independência: cargo tree -p logos-audit e guard metadata all-features/dev
  na verificação completa; testes sem importar ledger no auditor.

## 5. Riscos/decisões

ADR0025 registra fronteira, novo transporte e migração de API. Sufixo válido
sem âncora não é adulteração detectável. Assinatura não prova execução/autoridade;
Verified sem LRAT é contado como não rechecado. Erros/Unknown permanecem
falhas de recheck, não Refuted. SQLite read-only pode manter WAL/SHM.

## 6. Estimativa

M preservada. Refatoração TCB pequena, mas catálogo independente e regressões
adversariais exigem revisão. Nenhuma fase futura antecipada.

## 7. Evidência

Nove testes novos no auditor (1 de catálogo/22 vetores,8 de processos/leitores)
e dois testes novos de fonte no checker.21 regressões anteriores com CAS real
e smoke foram transferidos para ledger, sem enfraquecer o guard dev/features.
Alvos audit/model_checker/lib passaram40 testes; Clippy workspace passou.
Fixture Python independente/--check/mypy estrito/Ruff passaram.
16 adulterações de histórico/estrutura, mais corrupção/ausência/links CAS,
schema/coluna/lock, limites e truncamento; rejeição sem saída parcial.
As quatro posições Refuted recebem testes de orçamento zero e de goal True
assinado com objetos íntegros, para exigir recheck lógico em todas elas.

ci/check-audit.sh passou: appender/CAS reais reproduzem8 eventos da fixture,
auditor recheca4/4 refutações e1 Witness; export preserva todos os bytes,
banco original removido e verificação do stream passa com a mesma cabeça.
proofs_unchecked=1 permanece explícito. Árvore metadata/all-features/dev e
cargo tree passaram localmente; execução remota ainda não observada.

Fuzz oficial: FUZZ_TOOLCHAIN=nightly-2026-09-28 CARGO_NET_OFFLINE=true
ci/fuzz.sh60 model_eval,10486 execuções/61s, exit0 sem crash, sanitizers ativos,
max_len4096. Target existente passou a exercitar *_from_source e h(M) errado.
Log target/model-eval-3.6.log; log preexistente restaurado byte a byte. Nenhum
parser novo na TCB (o decoder independente de eventos fica fora dela).

Mutações manuais compiladas, uma por vez; teste indicado deve falhar por
asserção, nunca contar falha de compilação. Fontes restauradas no finally.
Comandos/logs/resultados em target/audit-mutations/results.json; runner local
/tmp/logos-audit-mutations.py.10/10 capturadas:

| Bug inserido | Teste que detectou |
| --- | --- |
| Não conferir h(M) da fonte | source_hash_is_checked_even_when_transport_lies |
| Recalcular h(O) no lugar do esperado | source_errors_are_preserved_and_invalid_obligation_does_not_call_source |
| Ignorar teto O+M | test_cas_too_large_error_vs_budget_unknown_size |
| Não cobrar bytes de M | test_cas_shared_budget_model_bytes_step_accounting |
| Converter erro da fonte em Accepted | source_errors_are_preserved_and_invalid_obligation_does_not_call_source |
| Modelo ausente vira Accepted | test_cas_missing_model_hash_is_rejected |
| Ignorar resultado do recheck | signed_semantically_invalid_refutations_are_not_accepted_by_integrity_alone |
| Ignorar prev | detects_more_than_ten_independent_history_and_structure_tampers |
| Não comparar certificate/model | cas_missing_corrupt_wrong_certificate_and_symlinks_are_explicit_failures |
| Ignorar hash CAS | cas_missing_corrupt_wrong_certificate_and_symlinks_are_explicit_failures |

Os primeiros seis rodam com cargo test -p logos-ledger --test model_checker
NOME -- --exact; os demais com cargo test -p logos-audit --test audit NOME
-- --exact. TCB report:9287+105199=114486 LOC Rust (+2 próprios), model1847,
ledger3209, build-only64508; SQLite C176466 separado. Nenhuma dependência
externa nova. Verificação completa/selftest final: VERIFY_OK, exit0 (log target/verify-3.6.log).

Falhas iniciais, corrigidas: API vocabulary_for_check exigia DecodeLimits;
example chamava Ledger::open com String em vez de &Path. Clippy exigiu
as_chunks para grupos de tamanho constante. Teste de refutação inválida
trocava certificado sem trocar h(M), caindo antes na igualdade de bytes;
fixture corrigida para realmente testar goal True com objetos íntegros.
Mypy exigiu anotar listas heterogêneas do gerador; a anotação deixou linha
acima de100 caracteres. Primeira verificação completa terminou exit1 no Ruff
(E501), após Rust/smokes/crash/escala/supply-chain verdes; Python/selftest ainda
não tinham rodado nessa tentativa. Ruff format/check corrigidos; verificação
completa reiniciada. Log anterior: target/verify-3.6-first.log.

Rodada final completa:227 Python90,01s +25 TCB1,38s, autoteste sem avisos e
controles pelo diagnóstico correto. Crash10k/24 SIGKILLs/9999 ACKs/62,40s,
zero perda; rebuild100k exato nas cinco tabelas;1M/20000 consultas com
p99=19003ns (<1ms), cold rebuild264109ms fora da medida. Primeira tentativa
p99=18896ns/build252870ms, mas falhou depois no Ruff e não conta como full
verify bem-sucedida. Só avisos deny preexistentes. CI remota/ARM/fuzz≥1h
pendentes; terceiro critério3.6 permanece desmarcado. Novos smokes/testes/vetores
ligados aos jobs nativos/Python; nenhum arquivo TCB novo a listar no manifesto.
