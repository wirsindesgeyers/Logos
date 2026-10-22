# STATUS — onde o projeto parou

> Mantido pelo agente (regras em [`AGENTS.md`](AGENTS.md) §6). Para continuar, peça:
> **"implemente o próximo passo do STATUS.md"**.
> Última atualização:2026-10-21 · Pré-requisito9.1a concluído localmente (ADR0046): vocabulary/v2 com render_pt, v1 preservado, fuzz1h/mutações/kernel real e full verify/selftest VERIFY_OK. Próximo:9.1 integral; CI remota pendente.

## Resumo

Fase 0 (Fundação): **7 etapas implementadas; P2 dos controles 0.1/0.2 corrigidos localmente**.
Fase 1 (**1.1–1.6**) demonstrada localmente; CI/ARM/fuzz noturno pendentes.
Há código real em `logos-canon`, `logos-crypto`, `logos-core`, `logos-air`,
`logos-air-text`, `logos-obligation`, `logos-protocol`, `logos-model`, `logos-cli`,
`logos-audit`, `logos-ground`, `logos-lrat` e CAS/eventos/índices de `logos-ledger`.
Kernel possui typestate5.1 e transporte5.2 (socket Unix, staging e Query/Subscribe
reais). PROPOSE5.3 gera O com snapshot autenticado, publica no CAS e persiste
Pending; verificação completa anterior passou. A revisão de19/out confirmou
três P2 no transporte/staging e na abertura de índices por requisição. As
correções passaram no full verify/selftest (2006,93s, exit0/VERIFY_OK), com
treze regressões e17 mutações detectadas; a5.2 voltou a concluída localmente. CERTIFY/REFUTE e registry5.4 implementados e verificados localmente; WITNESS5.5 também implementado e verificado localmente;
demais handlers e gateway seguem pendentes.
Cliente10.1 async do protocolo, modelos Pydantic e staging implementados fora
da TCB; sete mensagens v1 testadas no executável real (ASSUME Unavailable).
Farm10.2 isolada/cache concluída localmente:500 obrigações distintas e veredictos
rechecados no kernel; full verify/selftest passou. MCP5R.1 de pesquisa concluído
localmente: dez tarefas stdio e auditoria/export por snapshots públicos passaram.
2.1–2.3: FiniteModel/model-v1, avaliador contextual e ModelChecker reais na TCB.
Defined com parâmetros De Bruijn isolados, valores totais/únicos e enumeração
conferida pela expansão. P2 confere bytes/h(O); CAS verifica bytes/h(M).
Um Budget cobre contexto/premissas/goal; tamanho de O/M é limitado no checker,
sem hard RSS/wall. Reader não abre construção selada de Obligation.
2.4: logos_provers.z3_models fora da TCB, SMT-LIB/Z3 → candidatos model/v1;
refute/witness, Enum/Open, Defined globais e timeout compartilhado.
200/200 contramodelos aceitos; Z3 falso10k com zero aceitação indevida;
negação invertida rejeitada pelo checker. Z3_MODELS_SMOKE_OK, 19 regressões.
ci/verify.sh --selftest: VERIFY_OK, 195 Python + 25 TCB.
2.5: finder exaustivo independente de Z3, domínios ≤4, status FOUND/
NO_MODEL_WITHIN_BOUND/EXHAUSTED e fallback após unknown nativo.
296 obrigações contra 682 modelos ≤3: 201/201 contramodelos aceitos e 95
ausências limitadas. Smokes público, sem import Z3 e runtime-only passaram;
41 testes dos produtores passaram; ci/verify.sh --selftest: VERIFY_OK, exit 0,
217 Python + 25 TCB. Smoke runtime-only sem dependências dev também passou.
2.6: explain_model(M,V,O) em português fora da TCB: glosas, domínios
ordinais, valores/fatos apenas de O, premissas/objetivo e h(M).
Defined é derivado internamente sem revelar dependências; omissão após
100 entradas é explícita. 20/20 refutações aceitas pelo checker real;
10 regressões e smoke runtime-only passaram. Full verify/selftest:
VERIFY_OK, 227 Python + 25 TCB. CI remota ainda não observada.
TCB atual Rust: 15131+747130=762261 LOC de correção, build-only64508;
logos-core344, logos-ledger3853, logos-obligation572, logos-kernel2547.
Medição da5.5 inclui testes em src/; dependências externas inalteradas.
SQLite 3.53.2 bundled acrescenta 176466 LOC C medidos
separadamente; o contador Rust não inclui esse C (ADR 0021).
2.6 não alterou formatos/dependências Rust; nenhum kernel/gateway/farm implementado.
3.1: catálogo event/v1 de 13 tipos, assinatura externa/KeyId e recibos de
autoridade separados. CheckRecord preserva Budget/outcome; consistência
Witness/Unknown explícita, used ⊆ cited e texto tainted não NFC preservado.
18 vetores independentes, 15 regressões, 599 adulterações e 18 mutantes
capturados. EVENTS_SMOKE_OK; fuzz final 123392/61 s sem crash.
Full ci/verify.sh --selftest: VERIFY_OK, 227 Python + 25 TCB.
Nenhum appender/persistência/índice; assinatura não prova lógica/cadeia/admin.
3.2: appender real em logos-ledger, SQLite WAL/FULL, BEGIN IMMEDIATE,
lock de escritor e replay integral. Seq/prev/epoch calculados; assinatura
conferida antes de persistir. Falha SQL/sync exige reabertura, sem head ambíguo.
48 testes no crate (16 novos, incluindo entrada de subprocesso), 18 vetores
event/v1 persistidos byte a byte e nove mutantes compilados capturados.
10k eventos/24 SIGKILLs: cadeia íntegra, nenhum append reconhecido perdido;
commit sem resposta observado e preservado. Segundo processo recusado.
Full ci/verify.sh --selftest: VERIFY_OK, exit 0; autoteste final isolado
sem avisos. Fuzz final event_decode: 139749 execuções/61 s, exit 0, sem crash.
CI/ARM/fuzz longo não observados; sem índices, integração CAS ou kernel.
3.3: claims/deps/citable e LedgerView reais, derivados de replay autenticado;
cache index.sqlite3 separado, comparado integralmente e publicado por backup/
sync/rename, nunca autoridade. Consultas em SQLite privado na memória, com
lock do writer; snapshot resolve V/O no CAS, tipagem e scope opaco exato.
Used define deps; retratação/invalidação/revogação retiram citabilidade do
fechamento de dependentes. Unknown/Pending/Refuted nunca viram premissa.
CLI logos ledger rebuild-index usa somente chave pública, compara cache.
14 testes índices +1 header/contexto +2 CLI; onze mutantes capturados.
100k eventos: cinco tabelas idênticas após apagar/reconstruir cache.
1M eventos assinados/conferidos, 20000 consultas: p99=36573ns (<1ms),
build252133ms fora da medida. Full verify/selftest VERIFY_OK, exit0;
227 Python +25 TCB, autoteste sem avisos. Fuzz92653/61s, exit0, sem crash.
Nenhum pacote externo novo; CI/ARM/fuzz longo não observados.
3.4: divisão A adotada pelo mantenedor (ADR0023), contrato ajustado antes do
código. EpochRange inclusivo u64 e opcionais de EpochAdvanced com corpo v2;
18 vetores históricos preservados e4 adicionados. Seis regressões novas,
49 testes direcionados/Clippy e oito mutantes passaram. Fuzz core172375/61s
e event107491/61s, exit0 sem crash. Full verify/selftest VERIFY_OK, exit0,
227 Python +25 TCB, autoteste sem avisos. Integração Fact fica na6.1.
3.5: CAS obrigatório em DIRECTORY/cas antes de assinar;
inventário tipado incluindo checks aninhados. CLI gc somente dry-run com
chave pública, candidatos conservadores/temporários e preservação de objetos.
78 testes ledger/CLI passaram, dez novos;28 ausências individuais recusadas
antes do Signer/SQL. Nove mutantes capturados, fontes restauradas; fuzz99066/61s,
exit0 sem crash. Full verify/selftest VERIFY_OK, exit0,227 Python +25 TCB,
autoteste sem avisos. ADR0024/plano/spec registram contrato e sidecars SQLite.
3.6: auditor independente demonstrado localmente, full verify/selftest VERIFY_OK,
exit0;227 Python +25 TCB, autoteste sem avisos.
ModelChecker recebe fonte falível sem ledger; h(O)/h(M), O+M, erros e medidor
preservados. Decoder próprio/SQLite-WAL/CAS, verify/export audit-stream/v1 e
relatório de provas sem LRAT.11 regressões novas,22 vetores históricos e
fixture Python; recheck4/4 e1 Witness antes/depois do export sem banco.
Dez mutantes capturados; fuzz model_eval10486/61s, exit0 sem crash.
Árvore sem ledger/kernel passou localmente; CI remota ainda não observada.
4.1: bloqueio inicial resolvido por escolha explícita da proposta A do
ADR0026. Contrato ROADMAP4.1/4.2/5.4/6.1/7.3/7.5 ajustado antes do código.
Classificador global O/V/Defined e candidatos AIR limitados concluídos localmente;
integração autenticada preservada nas etapas donas. Nenhum candidato vira
premissa ou claim. Dez regressões, oito vetores independentes,288 interpretações
de controle e12 mutantes compilados capturados. Fuzz vocabulary_decode18372/61s,
exit0 sem crash. Full verify/selftest VERIFY_OK, exit0;227 Python +25 TCB,
autoteste sem avisos. CI/ARM/fuzz longo ainda não observados.
4.2: alternativa A do ADR0027 adotada antes do código; determinismo DIMACS
integrado preservado na4.3. Grounder real/ground-v1 com axiomas explícitos,
valores simbólicos/Defined globais, DAG/raízes/refs e limites inclusivos.
11 regressões,5k O distintas/32 interpretações/256 valorações,12 vetores
independentes e13 mutantes compilados capturados. Fuzz final11287/61s,
exit0 sem crash. Full verify/selftest VERIFY_OK, exit0;227 Python +25 TCB.
Fixtures GC serializadas após WriterBusy intermitente,144 repetições passaram.
Determinismo remoto da4.2 aguarda CI; nenhum CNF/DIMACS/LRAT/claim novo.
4.3: Tseitin completo e DIMACS cnf/v1, origens por cláusula e suporte dos
axiomas preservados. Obrigação relida sem construir Obligation selada;
novo namespace cnf-v1 e corpo ObligationCreated/v2 com h(CNF)/encoder=1,
sem alterar os22 vetores históricos. Auditor rederiva3/3 eventos Lrat e
recusa CNF trocada mesmo com assinatura/CAS íntegros; prova LRAT segue pendente.
5k casos distintos contra Z3/tabela verdade, zero divergências;12 vetores,
13 mutantes compilados capturados e fuzz9673/132990 execuções sem crash.
Full verify/selftest VERIFY_OK, exit0;227 Python +25 TCB. Núcleo grounder/
DAG/IR/Tseitin983 LOC, crate inteiro1337. Determinismo remoto aguarda CI.

4.4: checker próprio cnf/v1+lrat-rup/v1, P2/RUP/deleções e fecho histórico
sob Budget compartilhado.15 testes Rust,200 provas SAT-2002/22 famílias e600
corrupções,14 vetores,28160 traces e11 mutantes compilados passaram. Fuzz final
844911/533686 execuções sem crash. Full verify/selftest VERIFY_OK, exit0;
227 Python +25 TCB, autoteste sem avisos. TCB logos-lrat417 LOC; corpus XZ56MB
com manifesto/hashes/reprodução offline. Concordância cake_lpr demonstrada
na4.5; produtor SAT na4.6 e used_premises ainda previsto na4.7.

4.5: A/ADR0030 adotada; teste local de substituição implementado e teste
integrado no kernel preservado na5.4. Cake_lpr upstream fixado com ELF estático
musl/Zig0.15.1 e rebuild x86_64 idêntico no mesmo host. Registry só aceita hash
fixado; P2/CAS/snapshots selados, prlimit/bubblewrap sem rede/host/shell, saída
completa e kill/wait. Auditor3/3 com ambos antes/depois do export sem banco;
1 prova histórica de outra rota continua explicitamente unchecked.10 testes
novos LRAT+1 auditor e11 mutantes passaram; fuzz final2821026/61s sem crash.
Corpus200/600 concordou em380.65s;14 controles manuais têm12 concordâncias
e2 limites de perfil/recursos registrados (duplicates/sparse). Verificação
completa com selftest: VERIFY_OK, exit0;227 Python +25 TCB. Rebuild em segundo
host/CI/ARM não observados. ADR0031/spec/plano registram
TCB Rust758679 como cota superior e componentes nativos separados.

4.6: produtor SAT fora da TCB implementado sob ADR0032, CaDiCaL3.0.1 por
fonte fixada/build offline, ponte às APIs CNF existentes e candidatos LRAT/
FiniteModel com limites explícitos.300/300 provas aceitas nos dois checkers,
300/300 contramodelos e300 testemunhas de consistência no ModelChecker;
300 CNFs distintas/10 famílias.25 testes novos passaram; TCB/dependências
inalteradas. Sete controles de reconstrução, protocolo adulterado/limites e
ausência de Z3 exercitados. Métricas de bytes por O/camada emitidas no relatório.
Verificação completa com selftest: VERIFY_OK, exit0;252 Python +25 TCB.
CI remota ainda não observada.

4.7: used_premises extrai referências do fecho aceito com suporte contextual
por cláusula e vínculo h(O)/h(CNF)/h(certificado).13 regressões novas passaram,
incluindo cinco citadas/duas usadas no índice real e CNF satisfatível quando
falta a premissa necessária.14 mutações compiladas detectadas; fuzz oficial
62502/5447 execuções em61s por alvo, exit0 sem crash. Metadados só em memória,
nenhum formato/parser/pacote externo novo. TCB12004+747130=759134 LOC.
Full ci/verify.sh --selftest: VERIFY_OK/exit0;252 Python +25 TCB.
CI/ARM/fuzz longo continuam pendentes; nenhum kernel/gateway implementado.

5.1: Claim<S> e SignedEvent<S> imutáveis, sem construção/conversão pública;
seis estados e transições consumidoras com append real antes da publicação.
Sessão/época/h(O)/citações conferidos; Refuted/Unknown não promovidos.
Oito regressões com1024 transições intercaladas e replay,12 compile-fail e
controle positivo passaram; nove mutações compiladas detectadas.
Full ci/verify.sh --selftest: VERIFY_OK/exit0 em2415.30s fora do sandbox;
tentativa isolada falhou em bwrap NETLINK_ROUTE e foi preservada.
252 Python +25 TCB; kernel243 LOC, acréscimo próprio241, sem pacote externo,
parser ou wire novo. ADR0034/tabela normativa; handlers5.3/5.4 ficam pendentes.

## Etapas da Fase 0

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 0.1 Repositório, workspace e CI | ⚠ implementada, falta CI remota | Guard corrigido: comentários, literais, nested attrs e cfg_attr não contam como lints do crate. Selftest real bloqueou forbid/deny comentados e unwrap pelo diagnóstico correto; `ci/verify.sh --selftest` passou. **Falta:** ver `ci.yml` verde e PRs deliberados bloqueados na CI. |
| 0.2 Manifesto e contagem da TCB | ⚠ implementada, falta comentário em PR real | Closure inclui workspace fora das raízes da propriedade; roots não duplicam; dev excluído, build/proc-macro separado e uso runtime/build local por propriedade. Regressões Cargo/tokei reais e smoke CLI 3+1=4 passaram. Snapshot limpo `a568d1e` em `tcb/history/`. **Falta:** observar diff/comentário no PR e manter snapshots dos commits seguintes. |
| 0.3 `logos-canon` (canon/v1) | ⚠ concluída, falta 1 h de fuzz | 28 testes unitários + propriedades + 77 vetores + diferencial Rust×Python (100k/direção) passam; 7 mutantes do decoder pegos. Fuzz local: 1,3 M (`canon_decode`) e 0,8 M (`canon_text`) execuções sem crash. **Falta:** ≥ 1 h por alvo no job noturno (`fuzz.yml`). |
| 0.4 `logos-crypto` | ⚠ concluída, falta 1 h de fuzz na CI | 36 testes (RFC 8032, vetores independentes de hash/KeyId/assinatura gerados com `hashlib`/`cryptography`, separação de domínios, chaves fracas, maleabilidade, arquivo de chave 0600/symlink) + 2 doctests `compile_fail,E0308`; 13 mutantes, todos mortos (inclui `verify` frouxo × estrito e `open` sem `O_NOFOLLOW`/`O_NONBLOCK`); fuzz local 0,5 M (`crypto_decode`) e 3,7 M (`crypto_keyfile`) execuções sem crash. **Falta:** ≥ 1 h por alvo na CI e confirmar `O_NOFOLLOW` do aarch64. |
| 0.5 CAS | ⚠ implementada, falta CI/fuzz noturno | 17 testes (6 unitários + 10 públicos + 1 de vetores), smoke real e 6 mutantes mortos; 3 endereços reproduzidos só por `spec/cas.md` + `hashlib`; fuzz final `cas_domain`: 798945 execuções em 61 s sem crash. **Falta:** CI e ≥ 1 h no job noturno. ADR 0005, `spec/cas.md` e [`plano`](docs/plans/etapa-0.5.md) §7. |
| 0.6 `logos-core` (veredictos/orçamentos) | ⚠ implementada, falta CI nativa S3 | 21 testes + 7 fixtures compile-fail e controle positivo; 126 vetores independentes, smoke real e 11 mutantes mortos; fuzz `core_decode`: 224269 execuções em 61 s sem crash. ADR 0006, `spec/core.md` e [`plano`](docs/plans/etapa-0.6.md) §7. **Falta:** observar 1000 execuções por caso em x86_64/aarch64 na CI e confirmar flags crypto em ARM. |
| 0.7 Ameaças e escopo (docs) | ✅ concluída localmente | Três documentos + plano; script reproduzível `DOCS_0_7_OK`: 18 ameaças, 8 atacantes com referências recíprocas, G1–G3 com premissas/TCB/estado, L1–L3 e links locais. Apêndice B distingue mitigação projetada de limite fundamental. Sem nova mitigação/modelo formal. |

## Etapas da Fase 1

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 1.1 Sintaxe abstrata `FOL-ms-eq/v1` | ⚠ implementada, falta CI/fuzz noturno | Prefixo plano/De Bruijn, limites agregados, air/v1, hash Formula; 11 testes, 168 vetores independentes, `AIR_SMOKE_OK`, 4 mutantes mortos, fuzz 182529/61 s sem crash. [Plano](docs/plans/etapa-1.1.md) §7 e ADR 0007. **Falta:** CI, smoke ARM e ≥ 1 h noturna. |
| 1.2 Vocabulário versionado com grounding | ⚠ implementada, falta CI/fuzz noturno | 39 testes, 53 vetores independentes de h(V), smoke público e 4 mutantes mortos; ciclos por Pred/termos, patches revalidados e compatibilidade direcional. Fuzz 236898/61 s; `ci/verify.sh --selftest`: `VERIFY_OK`. [Plano](docs/plans/etapa-1.2.md) §7, ADR 0009 e `spec/vocabulary.md`. **Falta:** CI, smoke ARM e ≥ 1 h noturna. |
| 1.3 Sort-checker | ⚠ implementada, falta CI/fuzz noturno | 24 testes, 2000 fórmulas bem tipadas e 2270 mutações de entradas, quatro mutantes mortos, paths e stacks de 64 KiB; LLVM 36/36 ramos, 100%, sem exclusões. Smoke público e `ci/verify.sh --selftest`: `VERIFY_OK`. Fuzz 206912/61 s. [Plano](docs/plans/etapa-1.3.md) §7, ADR 0010 e `spec/sort-check.md`. **Falta:** jobs Rust/sort-coverage remotos, smoke ARM e ≥ 1 h noturna. |
| 1.4 Sintaxe de superfície/parser | ✅ concluída localmente | Gramática/ADR 0011, parsers independentes fora da TCB, 31 Rust + 71 Python; propriedades de 500 ASTs/lado, diferencial 500 fórmulas/bytes V+Formula e 13 oráculos de precedência/escopo. Smokes reais, limites/erros controlados e `ci/verify.sh --selftest`: `VERIFY_OK`, exit 0. [Plano](docs/plans/etapa-1.4.md) §7. CI/ARM ainda não observados. |
| 1.5 Gerador de obrigações | ✅ concluída localmente | Gerador TCB/obligation-v1 só de saída, snapshot LedgerView abstrato e refs de eventos; 19 Rust (17 behavior, vetores, UI), 20 vetores/hash independentes, 5 negativos compile-fail + controle positivo e seis mutantes mortos. OBLIGATION_SMOKE_OK, full verify/selftest VERIFY_OK, exit 0. [Plano](docs/plans/etapa-1.5.md) §7/ADR 0012. CI/ARM ainda não observados. |
| 1.6 Formato da proposta | ✅ concluída localmente | logos-protocol fora da TCB, proposal/v1 estrito/schema P1, texto tainted sem entrar em O; 17 Rust, seis vetores independentes, dois mutantes mortos. PROTOCOL_SMOKE_OK e full verify/selftest VERIFY_OK, exit 0. [Plano](docs/plans/etapa-1.6.md) §7/ADR 0013. CI/ARM ainda não observados. |

Roteiro em [plano da Fase 1](docs/plans/fase-1.md).

## Etapas da Fase 2

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 2.1 Estruturas finitas | ✅ concluída localmente | FiniteModel/model-v1 contextual, totalidade/domínios/enumeração/Defined/tuplas/limites; 15 testes Rust, 32 vetores/hash independentes e quatro mutantes mortos. MODEL_SMOKE_OK, fuzz final 212019/61 s sem crash e full verify/selftest VERIFY_OK, exit 0. [Plano](docs/plans/etapa-2.1.md) §7, ADR 0014 e spec/model.md. CI/ARM/fuzz noturno ainda não observados. |
| 2.2 Avaliador M ⊨ φ | ⚠ implementada, falta S3 remota | 30 testes Rust, 10k pares Z3 (5783 true/4217 false), zero divergências; EVAL_SMOKE_OK, cinco mutantes mortos, fuzz final 13591/61 s e full verify/selftest VERIFY_OK. Núcleo 537 LOC (alvo <400 excedido); logos-model 1590. [Plano](docs/plans/etapa-2.2.md) §7, ADR 0015 e spec/evaluation.md. **Falta:** observar 1000 repetições S3 x86_64/aarch64 na CI e fuzz longo. |
| 2.3 Checkers de refutação/consistência | ✅ concluída localmente | ModelChecker/P2, reader não autoritativo de obligation/v1 e CAS real; 21 testes de checker + 15 de decoder, 20 vetores positivos/10 negativos. cargo-mutants: 12/12 viáveis capturados no checker (8 inviáveis), 17/17 no reader (4 inviáveis), zero sobreviventes. CHECKER_SMOKE_OK; fuzz final 10297/3909 execuções por alvo em 61 s, full verify/selftest VERIFY_OK, exit 0. [Plano](docs/plans/etapa-2.3.md) §7/ADR 0016. CI/ARM/fuzz longo ainda não observados. |
| 2.4 Produtor Z3 de modelos | ✅ concluída localmente | refute/witness por SMT-LIB real, extração model/v1 contextual e limites; 200/200 contramodelos aceitos, Z3 falso10k (2100 legítimos/7900 rejeitados/zero indevidos), negação invertida rejeitada. Z3_MODELS_SMOKE_OK; 19 regressões, full verify/selftest VERIFY_OK. [Plano](docs/plans/etapa-2.4.md) §7/ADR 0017. CI ainda não observada; TCB inalterada. |
| 2.5 Model finder alternativo | ✅ concluída localmente | 296 obrigações distintas contra enumeração independente dos 682 modelos ≤3; 201/201 refutáveis encontrados e aceitos, 95 negativos. Limites/trabalho/timeout distinguem EXHAUSTED. MODEL_FINDER_SMOKE_OK com unknown nativo/fallback/checker; runtime-only e sem import Z3 passaram. 22 regressões novas + 19 da 2.4 passaram; full verify/selftest VERIFY_OK, exit 0. [Plano](docs/plans/etapa-2.5.md) §7/ADR 0018. CI ainda não observada; TCB inalterada. |
| 2.6 Tradução de contramodelos | ✅ concluída localmente | explain_model com h(M)/gloss_pt/ordinais e relevância sintática de O; Defined sem vazamento, valorações verdadeiras/falsas preservadas. Checker Rust aceitou 20/20; 10 regressões, MODEL_EXPLAIN_SMOKE_OK e runtime-only passaram. Full verify/selftest VERIFY_OK, exit 0, 227 Python + 25 TCB. [Plano](docs/plans/etapa-2.6.md)/ADR 0019. CI ainda não observada; TCB e formatos inalterados. |

Roteiro em [plano da Fase 2](docs/plans/fase-2.md). Fase4 tem classificação, grounder e CNF/auditor4.1–4.3 implementados localmente; Fases5–23 não iniciadas.

## Etapas da Fase 3

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 3.1 Catálogo de eventos v1 | ✅ concluída localmente | 13 tipos/event-v1, 18 vetores independentes, 15 regressões/599 adulterações e 18 mutantes capturados. EVENTS_SMOKE_OK, fuzz final 123392/61 s sem crash e full verify/selftest VERIFY_OK, exit 0. [Plano](docs/plans/etapa-3.1.md) §7, ADR 0020 e spec/events.md. CI/ARM/fuzz longo não observados; sem persistência/appender. |
| 3.2 Appender com hash chain | ✅ concluída localmente | SQLite WAL/FULL, lock de escritor, replay integral, 16 testes novos/48 no crate e nove mutantes capturados. LEDGER_CRASH_OK: 10k eventos/24 SIGKILLs, zero perdas de appends reconhecidos; segundo processo recusado. Full verify/selftest VERIFY_OK, exit 0; autoteste isolado sem avisos; fuzz final 139749/61 s, exit 0. [Plano](docs/plans/etapa-3.2.md) §7, ADR 0021 e spec/ledger.md. CI/ARM/fuzz longo não observados. |
| 3.3 Índices e consultas | ✅ concluída localmente | LedgerView/CAS/escopo e claims/deps/citable reais, cache derivado sem autoridade, CLI com chave pública. 17 testes novos e onze mutantes capturados; reconstrução100k preservou todas as cinco tabelas. 1M/20000 consultas: p99=36573ns, build252133ms fora da latência. Full verify/selftest VERIFY_OK, exit0; fuzz92653/61s, exit0, sem crash. [Plano](docs/plans/etapa-3.3.md) §7, ADR 0022 e spec/ledger-index.md. CI/ARM/fuzz longo não observados. |
| 3.4 Épocas | ✅ concluída localmente | Divisão A adotada pelo mantenedor (ADR0023): EpochRange genérico e opcionais com epoch-body/v2; teste integrado Fact preservado na6.1. Seis regressões novas,22 vetores (18 idênticos), oito mutantes e fuzz core/event172375/107491 execuções sem crash. Full verify/selftest VERIFY_OK, exit0. [Plano](docs/plans/etapa-3.4.md)/[spec](spec/epochs.md). CI/ARM/fuzz longo pendentes. |
| 3.5 CAS integrado ao ledger | ✅ concluída localmente | Appender sem bypass, inventário completo do catálogo, CLI gc somente dry-run. Dez testes novos,28 omissões individuais,78 testes ledger/CLI e nove mutantes passaram; fuzz99066/61s sem crash. Full verify/selftest VERIFY_OK, exit0,227 Python +25 TCB. [Plano](docs/plans/etapa-3.5.md)/[spec](spec/ledger-cas.md)/ADR0024. CI/ARM/fuzz longo pendentes. |
| 3.6 Auditor independente | ⚠ implementada, falta CI remota de independência | verify/export, decoder e leitores próprios,4/4 Refuted e1 Witness rechecada com Budget original;11 regressões novas e dez mutantes capturados. Smoke SQLite/export sem banco e independência passaram localmente; full verify/selftest VERIFY_OK, exit0. Terceiro critério aguarda CI remota. [Plano](docs/plans/etapa-3.6.md)/[spec](spec/audit.md)/ADR0025. |

## Etapas da Fase 4

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 4.1 Fragmento de domínio finito | ✅ concluída localmente | Proposta A do ADR0026: classificador global/limites e candidatos sem autenticação; fontes/O/used/deps preservados em6.1/7.3/7.5. Dez regressões, oito vetores,288 interpretações,12 mutantes e fuzz18372/61s passaram. Full verify/selftest VERIFY_OK, exit0. [Plano](docs/plans/etapa-4.1.md)/[spec](spec/finite-fragment.md). CI/ARM/fuzz longo pendentes. |
| 4.2 Grounder | ⚠ implementada, falta CI de determinismo | ADR0027/A: ground/v1 e referências/axiomas explícitos;11 regressões,5k O/32 interpretações/256 valorações,12 vetores e13 mutantes passaram. Fuzz11287/61s sem crash; full verify/selftest VERIFY_OK, exit0. Primeiro critério demonstrado; segundo aguarda CI. [Plano](docs/plans/etapa-4.2.md)/[spec](spec/grounding.md). |
| 4.3 Tseitin/CNF e proveniência | ⚠ implementada, falta CI de determinismo | 5k O/Z3/tabela verdade sem divergência,12 vetores e13 mutantes; auditor3/3 antes/depois do export, bytes/hash/versão conferidos. Fuzz9673/132990 sem crash; full verify/selftest VERIFY_OK, exit0. Núcleo983 LOC/crate1337; primeiro a terceiro critérios demonstrados, quarto aguarda CI. [Plano](docs/plans/etapa-4.3.md)/[spec](spec/cnf.md)/[ADR0028](docs/adr/0028-tseitin-canonico-e-rederivacao.md). |
| 4.4 Checker LRAT próprio | ✅ concluída localmente no corpus exigido | P2/RUP/deleções/fecho;200/200 provas e600/600 corrupções concordam com cake_lpr na4.5 (380.65s), sem filtro Rust.14 vetores Rust preservados; dois controles de perfil/recursos diferem no cake_lpr (duplicates/sparse), explicitados no ADR0031. [Plano](docs/plans/etapa-4.4.md). CI/ARM/fuzz longo pendentes. |
| 4.5 Integração do cake_lpr | ⚠ implementada, falta rebuild em segundo host/CI | A/ADR0030 adotada; build estático fixado e rebuild x86_64 idêntico localmente; wrapper/registry/P2/isolamento e auditor3/3 com dois checkers.10 testes LRAT+1 auditor,11 mutantes e fuzz final2821026/61s passaram. Corpus200/600 concordou; dois controles manuais de perfil diferem. Verificação completa com selftest: VERIFY_OK, exit0. Primeiro critério aguarda segundo host; execução CI/ARM e fuzz longo pendentes. [Plano](docs/plans/etapa-4.5.md). |
| 4.6 Produtor SAT | ✅ concluída localmente | [Plano](docs/plans/etapa-4.6.md)/ADR0032 e fonte CaDiCaL fixados.300/300 válidas,300/300 pares SAT e300 consistências aceitas;300 CNFs distintas/10 famílias e sete controles.25 testes Python novos; métricas por O/camada emitidas. Verificação completa com selftest: VERIFY_OK, exit0;252 Python +25 TCB. Nenhum runtime/dependência TCB alterado; CI remota pendente. |
| 4.7 Dependências extraídas do certificado | ✅ concluída localmente | ADR0033;13 regressões,14 mutações e fuzz oficial passaram. Cinco citadas/duas usadas no índice real; vínculo O/CNF/prova e suporte finito preservados. Full verify/selftest: VERIFY_OK/exit0. |

Roteiro em [plano da Fase4](docs/plans/fase-4.md). Grounder/CNF e rederivação implementados;
Checker LRAT4.4 e recheck duplo4.5 demonstrados no corpus exigido. Kernel/políticas ainda ausentes,
fontes opacas não demonstram autenticação.

## Etapas da Fase 5

| Etapa | Estado | Evidência / próximo trabalho |
| --- | --- | --- |
| 5.1 Typestate de claims | ✅ concluída localmente | Claim<S>/SignedEvent<S> selados; seis operações com evento único após append confirmado. Oito testes/1024 transições,12 compile-fail+controle positivo, nove mutações compiladas; full verify/selftest VERIFY_OK/exit0. [Plano](docs/plans/etapa-5.1.md), ADR0034 e spec/state-machine.md. CI desta etapa ainda não observada. |
| 5.2 Servidor do protocolo | ⚠ concluída localmente após correções; CI/ARM/noturno pendentes | Correções19/out (ADR0040): prazo por evento/escrita/cursor, diretórios devolvidos sem sobrescrever upload novo e Status com projeção viva.13 regressões/17 mutações; novo full verify/selftest VERIFY_OK/exit0 em2006,93s,520 Rust/252 Python/25 TCB; [reproduções](docs/reviews/2026-10-19-kernel.md). Evidências anteriores preservadas: codec/socket/staging/Query/Subscribe reais;14 vetores positivos/20 negativos e13 mutantes compilados detectados. ADR0035/0036. Seis lotes completos anteriores mais dois de retomada oficial com exit0, excluindo o parcial:14400s pedidos/15457s reportados,96634872 execuções sem crash, pico1331MiB; hashes inalterados. [Resumo versionado](docs/plans/etapa-5.2-fuzz.json). Full verify/selftest anterior VERIFY_OK/exit0 em1760.07s,252 Python+25 TCB; [plano](docs/plans/etapa-5.2.md) §7. |
| 5.3 PROPOSE e regra de citação | ⚠ concluída localmente após correção do índice; CI/ARM pendentes | Correção19/out (ADR0040): PROPOSE usa projeção viva e sufixos autenticados. Quatro propostas sucessivas passam com cache de disco indisponível; com5mil eventos, propostas após abertura custaram3,32/3,31ms. Novo full verify/selftest VERIFY_OK/exit0 em2006,93s. Handler generate/snapshot/CAS/transições, autoria do Genesis e resultado novo propose-result/v1 (ADR0038). Oito testes novos, executável real,50 ataques P1/STATE14/época9 passaram;14 mutações compiladas detectadas, fontes restauradas. Fuzz560503/61s/exit0; log5.2 preservado byte a byte. Full verify/selftest VERIFY_OK/exit0 em1568.40s,252 Python+25 TCB. [Plano](docs/plans/etapa-5.3.md)/[evidências](docs/plans/etapa-5.3-evidencias.json). |
| 5.4 CERTIFY e registry de checkers | ⚠ concluída localmente; CI pendente | ADR0041; CERTIFY/REFUTE reais, registry por formato/hash e retomada privada de Pending. Onze integrações novas, dez mutações com controles positivos e full verify/selftest VERIFY_OK. |
| 5.5 Não-vacuidade/WITNESS | ⚠ concluída localmente; CI pendente | ADR0042; WITNESS real, antes/depois, prazo lógico, replay e consulta de uso. Γ inconsistente prova False mas não autoriza ação;10 mutações, fuzz e full verify/selftest passaram. |

## Etapas compartilhadas com a Fase10

| Etapa | Estado | Evidência / próximo trabalho |
| --- | --- | --- |
| 10.1 Cliente do protocolo | ⚠ concluída localmente; CI pendente | ADR0043; sete mensagens v1 no kernel real,21 testes adversários/vetores/staging, ambiente sem dev e full verify/selftest passaram. Cliente fora da TCB; ASSUME Unavailable. |
| 10.2 Prover farm de pesquisa | ⚠ concluída localmente; CI pendente | ADR0044; quatro rotas isoladas/cache sempre rechecado;500/500 distintas,250 Verified/250 Refuted, p50=1.274575s. Nove testes, runtime sem dev e full verify/selftest passaram. |

## Etapas da Fase5R — bancada científica

ADR0037 aceito em20/out. Incorporação documental não é implementação.

| Etapa | Estado | Dependências / próximo trabalho |
| --- | --- | --- |
| 5R.1 MCP de pesquisa | ⚠ concluída localmente; CI pendente | ADR0045; seis ferramentas stdio,10 tarefas (5 provas/5 refutações), auditoria Rust+cake_lpr e export rechecado;24 testes Python+2 CLI/full verify passaram. |
| 5R.2 Formalizador com confirmação | ⏳ não iniciada | 5R.1 e9.1 completa (TCB de fidelidade). |
| 5R.3 Caderno de hipóteses | ⏳ não iniciada | 5R.1;5.9 e5.7 completas, incluindo retirada de premissas. |
| 5R.4 Resposta com cartões | ⏳ não iniciada | 5R.2/5R.3; perfil de pesquisa10.6 entregue aqui. |
| 5R.5 Chat/TUI de pesquisa | ⏳ não iniciada | 5R.4; perfil de pesquisa10.7 entregue aqui, sem ações. |
| 5R.6 Piloto técnico/UEA | ⏳ não iniciada | 5R.1–5R.5;11.8 antes de coleta com participantes. |

## Etapa9.1 — pré-checagem

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 9.1a Templates versionados | ⚠ concluída localmente; CI pendente | ADR0046; vocabulary/v2, vetores14, diferencial Rust/Python, quatro mutações, fuzz3600s+60s, cenário v2 no kernel/MCP/auditor e full verify/selftest VERIFY_OK em3566.59s. [Evidências](docs/plans/etapa-9.1a-evidencias.json). |
| 9.1 Renderizador/parser PT | ⏳ não iniciada | 9.1a pronta; implementar com todos os critérios e revisão humana. |

## Trilha de interface e workspaces (Apêndice G, proposta)

| Etapa | Estado | Gate / entrega |
| --- | --- | --- |
| WS.0 Decisões de produto/stack | ⏳ não iniciada | documental; ADRs 0047–0049 propostos aguardam o mantenedor |
| WS.1 Contrato de sessão/apresentação | ⏳ não iniciada | 5R.4 e WS.0 |
| WS.2 Servidor web local + chat | ⏳ não iniciada | WS.0, WS.1, 5R.5 |
| WS.3 Registry de projetos | ⏳ não iniciada | MEM.0/MEM.1 (bloqueio se ADR 0008 não for adotado) |
| WS.4 Artefatos e anexos | ⏳ não iniciada | WS.3, MEM.1, 5R.3 |
| WS.5 Visualizações | ⏳ não iniciada | WS.1, WS.4 |
| WS.6 Contexto de projeto | ⏳ não iniciada | MEM.5/MEM.6 (dependem de 8.9/9.2) |
| WS.7 Documentos/PDF | ⏳ não iniciada | WS.4, WS.5, 9.1 |
| WS.8 Workspace científico | ⏳ não iniciada | 5R.3, 5R.4, WS.4–WS.7 |
| WS.9 Operação/release | ⏳ não iniciada | WS.2–WS.8, MEM.7, 10.8, 23.1 |

## Pendências de verificação (execução remota)

Etapa9.1a: full verify/selftest local passou; CI remota/ARM/noturno do novo
commit ainda não consultados. Evidências em docs/plans/etapa-9.1a-*.json.

Etapa5R.1: full verify/selftest local passou; CI remota do novo commit ainda
não consultada. Evidências em docs/plans/etapa-5R.1-*.json.

Etapa10.2: full verify/selftest local passou; CI remota do novo commit ainda não
consultada. Corpus e recibos em docs/plans/etapa-10.2-*.json.

Etapa10.1: verificação completa local passou; CI remota do commit ainda não
consultada. Evidências em docs/plans/etapa-10.1-evidencias.json.

Etapa5.5: full verify/selftest local passou; CI remota/ARM/noturno deste
novo commit ainda não consultados. Evidências em docs/plans/etapa-5.5-evidencias.json.

Integração documental5R (2026-10-20): verificação completa local passou;
CI remota do commit documental ainda não consultada. Evidências em
`docs/plans/integracao-5r-evidencias.json`.

Etapa5.4: verificação completa local passou; execução remota deste novo
commit, ARM e duração noturna dos alvos ainda não observadas.

1. **Actions**: ✅ verde na run `39085765358` (`3f902d8`) — todos os jobs passaram,
   incluindo os dois `rust` e o `python`. Os BOs dos primeiros envios ficaram
   fechados: userns/AppArmor (runners Ubuntu 24.04; `ci/enable-userns.sh` nos dois
   jobs) e `tokei` ausente no job `python` (agora via `taiki-e/install-action`;
   4 testes de `tcb.build_report` falhavam com "tokei não encontrado"). Resta só o
   `fuzz.yml` noturno: vinha vermelho desde 4/out com a definição antiga (cargo
   fuzz sob stable); a nova (`ci/fuzz.sh`) vale desde o push de 17/out e, a partir
   da5.2, roda `kernel_message` por14400s em8 lotes — confirmar na próxima run
   agendada.
2. Abrir PRs de teste deliberados e confirmar o bloqueio (`unsafe {}` em crate de TCB; dependência
   fora da allowlist; arquivo de TCB fora do manifesto) e o comentário automático de diff da TCB.
3. Confirmar que o job de fuzz **realmente executa** os alvos na CI e completar ≥ 1 h por alvo
   (`canon_decode`, `canon_text`, `crypto_decode`, `crypto_keyfile`, `cas_domain`), com logs
   arquivados (critérios da 0.3–0.5). Smoke local não fecha essa pendência.
   Alvos `core_decode` (0.6), `air_decode` (1.1), `vocabulary_decode` (1.2),
   `sort_check` (1.3), `model_decode` (2.1), `model_eval` (2.2) e
   `obligation_decode` (2.3) integrados ao smoke/noturno; logs locais não provam
   CI nem ≥ 1 h. Smokes AIR/vocabulário/sort-check/AIR textual/obrigações/protocolo/
   modelo/avaliador/checker ARM ainda não observados.
   Produtor 2.4 tem smoke/corpus/adversário/mutação ligados ao job Python existente;
   execução remota ainda não observada, sem exigir checker ARM novo nesta etapa.
   Finder/fallback 2.5 e enumeração independente também ligados ao job Python;
   execução remota ainda não observada; sem novo requisito ARM nesta etapa.
   Apresentação 2.6: corpus de 20 refutações/checker e smoke PT ligados ao mesmo
   job Python; execução remota ainda não observada. Sem alteração da TCB.
   Eventos 3.1: smoke nativo, vetores --check e event_decode ligados aos
   workflows; runner de fuzz semeia 18 wires válidos também na CI limpa.
   Execução remota/ARM e ≥ 1 h de event_decode ainda não observados.
   Appender 3.2: testes e ci/check-ledger-crash.sh ligados aos dois runners
   Rust nativos, com worker release/oráculo Python/10k eventos/24 SIGKILLs.
   Demonstração local passou; execução remota/ARM ainda não observada.
   Índices 3.3: check-ledger-index.sh ligado ao mesmo job nativo; compara
   reconstrução100k e p99 de20000 consultas com1M, artefato JSON por arquitetura.
   Execução local passou; remoto/ARM ainda não observado. Novo acesso ao h(V)
   de O é exercitado pelo alvo obligation_decode existente, sem desativar
   sanitizers; ≥1h remota continua pendente.
   Épocas3.4: testes de intervalo e ledger/snapshot reais entram no workspace;
   event_decode recebe os22 vetores (incluindo corpo v2) pelo runner existente,
   core_decode exercita limites do intervalo. Smokes locais de61s passaram;
   remoto/ARM e ≥1h continuam pendentes, sem integração de Fact antecipada.
   Integração3.5: dez testes novos no workspace nativo, worker/oráculo de
   crash confere objetos CAS e inventário do body exercitado em event_decode.
   Demonstração local passou; CI/ARM e fuzz longo ainda não observados.
   Auditor3.6: novos testes do workspace e ci/check-audit.sh ligados aos dois
   runners Rust; vetores independentes --check/lints ligados ao job Python.
   cargo tree e guard metadata all-features/dev passaram localmente. Falta
   observar a independência na CI para marcar o terceiro critério da3.6.
   API de leitura de modelos exercitada em model_eval; ≥1h remota pendente.
   Fragmento4.1: testes/smoke e vetores independentes ligados aos jobs nativos
   e Python. vocabulary_decode estende o alvo existente para candidatos e
   classificação; smoke local passou, CI/ARM/fuzz longo ainda não observados.
   Grounder4.2: diferencial5k, regressões/vetores/repetições/smoke ligados
   aos jobs nativos/Python. vocabulary_decode exercita O condicional e DAG;
   fuzz final11287/61s passou. Determinismo remoto e ≥1h ainda não observados.
   CNF4.3: smoke DIMACS/vetores nas duas arquiteturas; script SAT5k no job
   Python, rederivação3/3 em check-audit e testes nativos. Fuzz de vocabulário/
   eventos exercita CNF/releitura e corpo v2;61s por alvo passou localmente.
   Determinismo integrado remoto4.3/4.2 e ≥1h de fuzz ainda não observados.
   LRAT4.4: quinze testes nativos/smoke,14 vetores e corpus200/600 ligados
   aos jobs Rust/Python. cnf_decode/lrat_decode integrados à matriz que descobre
   os16 alvos; fuzz local final61s por alvo passou. Execução remota/ARM,
   matriz e ≥1h por alvo ainda não observados. Concordância cake_lpr no
   corpus200/600 demonstrada localmente na4.5.
   Cake_lpr4.5: build offline estático por hash e recheck duplo entram nos
   testes/smoke nativos; workflow cake-rebuild compara o hash fixado em jobs
   separados Ubuntu24.04/22.04, mas nenhum resultado remoto foi observado.
   Rebuild x86_64 local idêntico não fecha primeiro critério4.5. ARM só teve
   cross-build; execução nativa, bwrap/namespaces do runner e fuzz longo seguem
   pendentes. Dois controles manuais diferem e não são equivalência universal;
   corpus exigido200/600 concordou. Wrapper nunca normaliza CNF/IDs para ocultar
   diferenças. Certificados futuros precisam de Accepted nos dois checkers.
   cake_output é o17º alvo da matriz; fuzz local2821026/61s sem crash,
   mas ≥1h e execução remota permanecem pendentes.
   SAT4.6: ci/check-sat.sh no job Python prepara solver offline/encoder/
   cake_lpr e exige300/300 provas,300/300 contramodelos e premissas consistentes.
   Testes de protocolo/limites/reconstrução e25 regressões também no pytest;
   nenhum resultado remoto observado. Etapa não acrescenta parser na TCB.
   Used4.7:13 regressões no workspace, inclusive ledger/CAS/índice reais;
   fuzz lrat_decode/vocabulary_decode ampliado.14 mutações detectadas e
   full verify/selftest passaram localmente; CI/ARM/≥1h não observados.
   Typestate5.1: oito testes/1024 transições e12 compile-fail no workspace,
   nove mutações compiladas e full verify/selftest passaram localmente.
   CI/ARM desta etapa ainda não observados; sem parser/decoder novo.
   Transporte5.2: kernel_message é o18º alvo,14400s no noturno em8 lotes
   (FUZZ_BATCH_SECONDS; ADR0036). Cinco testes de codec/14 vetores positivos/
   20 negativos, quatro de staging, dez de socket/binário e treze mutações
   passaram. Diagnóstico anterior aponta acúmulo do runtime ASAN. Fechamento
   local conferido em19/10: seis lotes completos mais dois de retomada
   oficial com exit0; lote parcial excluído. Soma14400s pedidos/15457s
   reportados,96634872 execuções sem crash, pico1331MiB, hashes inalterados.
   Novo full verify/selftest VERIFY_OK/exit0 em1760.07s,252 Python+25 TCB.
   CI/ARM e fuzz noturno pendentes.
   PROPOSE5.3: oito testes novos e executável real no workspace nativo;
   cinquenta recusas P1/STATE14,14 mutações e full verify/selftest passaram
   localmente. propose-result/v1 tem três vetores novos no gerador/--check
   já usado pela CI;34 requests históricos preservados. Decoder não mudou;
   kernel_message560503/61s sem crash, log5.2 restaurado pelo hash. Falta
   observar esta etapa nos runners/CI; não alegar execução remota.
4. S3 da 0.6 e do avaliador 2.2 exige execução nativa em `ubuntu-24.04` e `ubuntu-24.04-arm`.
   Checker 2.3 também tem 1000 repetições locais; execução do teste/smoke nos
   dois runners nativos foi ligada ao workflow, ainda sem observação remota.
   [API do GitHub](https://api.github.com/repos/wirsindesgeyers/Logos) confirmou
   repositório público (`private=false`); disponibilidade/execução do runner ARM
   ainda depende de observar o job remoto.
5. Snapshot JSON limpo de `a568d1e` arquivado em
   [`tcb/history/a568d1e06eb8f2aee6b4b897f24caf038d3508c6.json`](tcb/history/a568d1e06eb8f2aee6b4b897f24caf038d3508c6.json).
   Foi medido pelo contador corrigido num worktree isolado (dirty=false):
   correção 3424 LOC próprios + 78287 runtime, 51581 só compilação.
   Manter snapshots por commit; commits históricos anteriores não foram reconstituídos.
6. **Revisão documental de memória (19/out):** full verify/selftest local
   `VERIFY_OK`, exit0,1458.31s; CI remota desta revisão não observada.
   Testes UniiChat/OptMem e oito casos extras são pesquisa local opcional,
   sem job novo na CI e sem fechar MEM.0–MEM.7. Provider real/custo/recall e
   critérios do runtime de memória continuam futuros, conforme Apêndice F.
7. **Correções da revisão da codebase (19/out):** três P2 corrigidos e
   demonstrados localmente; full verify/selftest VERIFY_OK/exit0 em2006,93s,
   520 Rust/0 ignorados,252 Python+25 TCB e430 entradas com hashes inalterados.
   [Evidências](docs/plans/correcao-kernel-2026-10-19-evidencias.json).
   CI/ARM/noturno desta correção ainda não observados; sem novo fuzz/parser.

## Achados da revisão — 2026-10-19 (corrigidos localmente)

[Relatório/reprodutor](docs/reviews/2026-10-19-kernel.md), basec647418,
[ADR0040](docs/adr/0040-projecao-viva-e-limites-do-transporte.md) e
[plano/evidências](docs/plans/correcao-kernel-2026-10-19.md):

1. **Subscribe:** prazo conferido por evento e compartilhado nas escritas;
   cursor final não renova prazo. Uma página por evento limita trabalho
   síncrono. Reprodução20ms:3 eventos completos/24,54ms de servidor, em vez
   dos32 eventos/1262,77ms; CPU síncrona não é preemptível.
2. **Status/PROPOSE:** projeção privada por writer, primeiro replay integral
   e somente sufixos autenticados depois. Status5mil após abertura0,18/0,20ms
   (antes2527,80–4035,84ms); PROPOSE3,32/3,31ms. Rebuild/cache em disco
   continua explícito; nunca se usa o cache persistido como autoridade.
3. **Staging:** diretórios rejeitados voltam intactos ao agente, por rename
   ancorado/NOREPLACE e nonce128bits no nome alternativo. Três tentativas:
   três devoluções/zero resíduos; erro de tipo preservado, sem percorrer links.

Treze regressões, dois compile-fail+controle positivo,17 mutações compiladas
detectadas e fontes restauradas por hash. Incluem linhas negativas no replay
inicial, assinatura/prev/época/cabeça, Poisoned, falha de entropia/devolução,
colisões e comparação com reconstrução. Full verify/selftest VERIFY_OK/exit0,
2006,93s;430 entradas inalteradas. Autoteste concluiu os diagnósticos certos,
com aviso de metadata da raiz na cópia tar registrado no plano. Sem formato/
pacote novo, novo fuzz ou observação remota. Evidências históricas preservadas.

## Achados da revisão — 2026-10-04 (corrigidos localmente)

Evidências abaixo descrevem `a568d1e` **antes** da correção. Agora o guard
descarta comentários/literais e módulos internos; a closure subtrai somente
raízes da propriedade. Também corrigida consolidação runtime/build que
classificava build-only como runtime em outra propriedade. Formatos e código
Rust da TCB não mudaram; regressões usam Cargo/tokei reais, sem mocks.

1. **P2 — atributo de lint comentado aceito.** `ci/check-tcb-lints.sh:14` procura
   texto sem distinguir comentário de atributo ativo. Fixture com apenas
   `// #![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]` e
   `pub fn unchecked(value: Option<u8>) -> u8 { value.unwrap() }` passou no
   guard **e** em `cargo clippy --workspace --all-targets -- -D warnings`
   (ambos exit 0). Controle com atributo ativo: Clippy exit 101,
   diagnóstico `clippy::unwrap_used`. A proteção anunciada pode desaparecer
   sem o guard perceber. Não é vulnerabilidade runtime encontrada nos crates atuais.
2. **P2 — dependência workspace omitida do medidor.** `tcb/tcb.py:191` exclui
   todos os membros do workspace do resultado transitivo, enquanto LOC próprio
   usa só crates declarados para a propriedade. Fixture real com logos-canon
   chamando logos-cli, dependência normal (`kind=None`), passou no manifesto,
   guard e Clippy; relatório de correção retornou próprio/total=3, deps=0,
   ignorando 1 LOC Rust da dependência. Não houve mock de cargo metadata/tokei.
   Deve contabilizar a closure workspace por propriedade, ou recusar closure
   que não esteja declarada na TCB; nunca excluir silenciosamente.

Quatro revisores cobriram canon Rust/Python, crypto/CAS, core e AIR com specs,
testes/vetores e fronteiras: nenhum bug runtime acionável identificado nesse
recorte. Revisão estática não é prova formal nem auditoria exaustiva.
Smokes executados: `CORE_SMOKE_OK`, `AIR_SMOKE_OK` e programa temporário com
NFC/minimalidade CBOR, limites AIR entre irmãos/termos, arquivo de chave real,
assinatura/domínio/mensagem, permissões/symlink, CAS reaberto/corrompido/reparado
e stream acima do limite. Dependências externas do smoke conferidas contra
Cargo.lock do projeto. Primeira tentativa do smoke falhou por esperar Io em
symlink; contrato retorna KeyFileNotRegular. Asserção temporária corrigida,
run final passou. Fixtures/programa removidos; sem nova suíte/fuzz/CI remota.

## Extensão arquitetural proposta — Long-Term Memory

[Extensão](docs/spec/LOGOS-long-term-memory.md) com 24 seções, schemas,
pseudocódigo, M1–M13, 24 critérios concretos, benchmarks e mudanças cirúrgicas
para a spec original. [ADR 0008](docs/adr/0008-memoria-de-longo-prazo.md)
**proposto**, memória não implementada. Kernel até 5.3 pronto localmente;
gateway/harness futuros. A revisão solicitada em 19/out acrescenta trilha
proposta MEM.0–MEM.7 no ROADMAP, sem fechar critérios de implementação.

| Etapa de memória | Estado | Gate / entrega |
| --- | --- | --- |
| MEM.0 Adoção/contratos | ⏳ não iniciada | decidir adoção ADR 0008, formatos/vetores/ACL/egress |
| MEM.1 Originais/inbox | ⏳ não iniciada | MEM.0; custódia e recuperação integral |
| MEM.2 Árvore/compactor | ⏳ não iniciada | MEM.1 + interface/test double de provider |
| MEM.3 Views/lotes/blocos | ⏳ não iniciada | MEM.2; due e high/low/frame/settle |
| MEM.4 Adapters/cache | ⏳ não iniciada | MEM.3, 8.9/9.2; fake separado de medição real |
| MEM.5 Harness/navegação | ⏳ não iniciada | MEM.1–4, 6.6, 8.9, 9.2, 10.3/5/6 |
| MEM.6 Knowledge/Router | ⏳ não iniciada | MEM.1/5, 6.6; lexical antes de dense/rerank |
| MEM.7 Avaliação/produto | ⏳ não iniciada | MEM.5/6, 10.7/8 e 23.1 para empacotar |

[Auditoria UniiChat/OptMem](docs/research/uniichat-memory.md), commits
`0500bf6`/`1fb164c` e ADR 0039: corrigidos due, duas views/cutoffs, lotes,
blocos de quatro linhas e single flight; retries mantêm input limitado.
Mantidos originais integrais/Locator/limite rígido no LOGOS, com divergências
upstream explícitas. Sete testes upstream passaram: replay simulado de 1503
requests do loop/98,6% e 3831 requests de compactação/96,4%, sem tokens/custo
reais. Oito casos extras/51 asserções chamaram funções reais e passaram;
OptMem: 109099 checks/0 falhas. Primeira execução Bun no sandbox teve EPERM
no flush e foi interrompida; repetição fora do sandbox passou. Esses testes
não fecham MEM nem recall/ACL/durabilidade do LOGOS.

Smoke estrutural descartável: 1M eventos, 1.999.993 nodes, 161 parts,
frame máximo 8.192/8.192 bytes, original #42 não NFC em 16 descidas.
Folding adversarial terminou em 8.123 bytes apesar de 236 merges que
aumentaram bytes privados; floresta extrema de 63 raízes ficou em 98.794 bytes.
Fixture SQLite WAL/FULL com três SIGKILLs preservou inbox/ID; resolver nativo
recusou fonte alterada/ausente. Isso não verifica módulos reais, LLM/recall,
PDF/OCR, ACL/providers ou kernel. Dois runners iniciais atingiram 300s/90s;
versões indexada e de comparação menor passaram; temporários removidos.

Adoção/formatos/domínios/vetores são MEM.0; dependências ausentes bloqueiam
implementação, sem substitutos. O smoke acima é histórico de 04/out, com
chooser antigo e sem lotes/cache: não demonstra os contratos corrigidos.
A revisão de 19/out foi verificada: `ci/verify.sh --selftest`, `VERIFY_OK`,
exit0 em1458.31s;252 testes Python+25 da TCB passaram. Os424 arquivos de
runtime/formatos acompanhados conservaram os hashes; CI remota não observada.
Esse trabalho manteve a5.4 como próximo passo principal; a revisão da codebase
posterior acrescentou as correções abaixo. Se o mantenedor priorizar memória,
o primeiro trabalho é o plano documental MEM.0, sem antecipar runtime.

## Próximo passo: Etapa9.1 — renderizador/parser PT

Pronto de que depende: AIR1.1, vocabulário1.2 e parser1.4; vocabulary/v2 com
render_pt explícito por símbolo, validado e vinculado a h(V) (9.1a/ADR0046,
spec/render-templates.md). Não usar gloss_pt como template.

Implementar render_pt(φ,V) e parse_pt(String,V) na TCB de fidelidade, gramática
da língua controlada em spec/paraphrase-pt.md, versão do renderizador/gramática
no hash da paráfrase, round-trip em100k fórmulas até profundidade6 e em todos
os vetores, fuzz de parse_pt≥1h e revisão humana registrada de50 paráfrases de
políticas reais. Não substituir a revisão por script. Plano em
docs/plans/etapa-9.1.md (hoje só pré-checagem) precisa ser completado.

Decisões em aberto: crate/propriedade da TCB de fidelidade que recebe o código
(manifesto), forma da gramática e tratamento de templates coincidentes de
símbolos distintos (ADR0046, limite explícito: identidade do símbolo precisa
ficar na gramática). Riscos: injetividade, ambiguidade de escopo e custo da
revisão humana.9.2/5R.2 continuam fora. CI remota/ARM/noturno pendentes.

## Decisões em aberto para o mantenedor

- **Trilha de interface (Apêndice G):** aceitar, alterar ou rejeitar os ADRs
  0047 (um harness, clientes e projetos), 0048 (servidor web local) e 0049
  (artefatos, visualizações e documentos), todos propostos. Stack de servidor,
  frontend, gráficos, matemática e formato de documento fica para a WS.0, com
  alternativas e critérios. Não altera o próximo passo (9.1).
- **Tamanho da TCB de correção (execução):** 15262 LOC Rust próprios + 747130
  de dependências Rust = 762392 (+64508 só de compilação), após a9.1a (+131
  próprios, sem dependência nova). Medição anterior, após a5.5: 15131+747130=762261;
  logos-core344, logos-air1556, logos-obligation572, logos-model1847, logos-ground1534, logos-lrat1123, logos-ledger3853, logos-kernel2547.
  Acréscimo de343 LOC próprios sobre12826f4, incluindo testes em src/; pacotes externos inalterados.
  Resumo atual em docs/plans/etapa-5.5-evidencias.json (TCB).
  SQLite bundled 3.53.2 soma 176466 LOC C em sqlite3.c, medidos separadamente
  por tokei: o relatório Rust sozinho não representa toda a TCB transitiva.
  Rustix49394 + linux-raw-sys478851 + libc113368 + errno318 acrescentam641931
  LOC runtime de cota superior (fontes completos/todas as plataformas/módulos,
  não LOC efetivamente compilados). O custo não é omitido por serem bindings.
  C/FFI/musl/assembly e ferramentas de isolamento ficam fora desse contador Rust;
  hashes dos dois binários cake_lpr constam no manifesto, sem estender a prova
  CakeML ao wrapper/C/OS. Relatório target/tcb-4.5-final.json/.md.
  Maiores dependências Rust: linux-raw-sys478851, libc113368 e rustix49394;
  seguem `curve25519-dalek` 26 k, `unicode-normalization` 23,9 k (NFC) e
  `typenum` 16 k. Se isso incomodar para a avaliação (11.9): restringir o texto canônico (p.ex.
  identificadores ASCII) tira a NFC; trocar a curva exigiria outra biblioteca. Não alterado. A
  medição agora filtra pela plataforma do host e separa execução × compilação.
- Planos por etapa em `docs/plans/etapa-X.Y.md`: para 0.1–0.4 só existe o plano de fase
  (`docs/plans/fase-0.md`); a partir da 0.5 vale seguir `AGENTS.md` §2.

## Armadilhas conhecidas

- PROPOSE5.3 retorna propose-result/v1 `[1,h(O),O_bytes]`, sem credencial.
  Query::Status usa h(Event), não h(O); obter ObligationCreated por Subscribe.
  O autor registrado é a chave do Genesis/kernel; UID autenticado não é
  persistido no evento. O teto de saída1MiB não é teto de RSS/wall da geração.
- Fuzz5.2/ASAN: o runtime do fuzz (build com ASAN) acumula RSS por execução
  mesmo em alvo trivial — platô de~616MiB em `smoke` com a quarentena padrão,
  35MiB com `quarantine_size_mb=0` — então corrida única de4h estoura o teto
  padrão2048MiB sem vazamento do decoder. `ci/fuzz.sh` roda em lotes de
  `FUZZ_BATCH_SECONDS` (padrão1800s) com processo novo por lote e corpus
  cumulativo; um lote de1800s pica em~1160MiB (ADR0036). Não simplificar o
  laço de lotes de volta para corrida única, e não usar quarentena0 no fuzz
  oficial (esconderia use-after-free).
- Typestate5.1: Transitions empresta exclusivamente o writer e não permite
  reutilizar claims em outra sessão. API interna até handlers5.3/5.4;
  CheckOutcome/Event público não é autorização. CertificateRejected/v2 foi
  incorporado na5.4 (ADR0041); nunca reinterpretar UnknownRecorded nem
  promover rejeição a refutação.
- No sandbox do Codex, o bwrap falha ao criar socket NETLINK_ROUTE
  (`Operation not permitted`). A verificação completa precisa rodar com
  autorização externa ao sandbox; não remover o isolamento do checker.

Ver `AGENTS.md` §8 (toolchain, `cp -i`, `pkill -f`, Unicode 16, `gh` ausente, `gen` reservado).
- Executar Ruff de tools/ a partir de py/, como ci/verify.sh. Selecionar
  somente o ambiente com uv --project py a partir da raiz usou outra configuração
  de formatação; isso causou a primeira falha completa da4.4. O corpus LRAT
  é offline e usa Python padrão/lzma no harness Rust; a prova continua checada
  pelo Rust e pelo oráculo independente, sem solver instalado na CI.
- Uma tag nova exige atualizar spec/domains.md, constante e catálogo RESERVED,
  regenerar tools/gen_crypto_vectors.py, incluir o tipo/caso no helper Rust
  de vetores e atualizar as contagens em py/tests/test_crypto_vectors.py.
  Rodar os14 geradores --check e a suíte Python antes do full verify evita
  descobrir essa sincronização depois do benchmark de1M. Os17 namespaces
  do CAS são catálogo separado: ground/v1 não é persistível implicitamente.
  Atualizar também a contagem em gen_cas_vectors.py e o despacho de domínios
  do GC; regressão da4.3 cobre CNF raw e o último domínio reservado.
- SQLite read-only pode criar/manter sidecars WAL/SHM; GC3.5 preserva objetos
  CAS/banco principal e lê WAL confirmado. Não usar immutable para esconder
  sidecars e acabar omitindo eventos. Testes de preservação distinguem esses arquivos.
- `rust-analyzer` não existe na toolchain stable do ambiente; consulta LSP falhou na inicialização.
  Rust foi validado por compilação/clippy/testes, não por diagnósticos LSP.
- Formatar ferramentas Python com `uv run ruff format ../tools/...` dentro de `py/`:
  a configuração usada por `ci/verify.sh` difere da descoberta ao rodar na raiz.
- Revisores paralelos não executaram a revisão: backend retornou
  `Named models unavailable: Free plans can only use Auto`. Revisão feita
  diretamente; não atribuir resultados aos jobs falhos.
- Z3 binário não estava disponível; usa-se z3-solver 5.1.0.0 Python, fora da TCB.
  Desde 2.4 é dependência runtime, não apenas diferencial dev. Transporte SMT-LIB
  passa por Solver.from_string; quantificadores nativos e contextos isolados.
- Python não tem servidor LSP configurado: consulta de referências retornou
  No language server found. Fatoração validada por AST, mypy e regressões reais.
- Desde 2.5, ProducerError/AST/validação dos produtores ficam em model_context;
  z3_models não reexporta os símbolos movidos. Fallback usa ausência de candidato
  (None), pois API Z3 não distingue unknown/timeout/unsat finito.
- Fuzz com LeakSanitizer dentro do sandbox terminou com erro de ptrace no
  encerramento, mesmo após completar o tempo. Repetição fora do sandbox
  passou; não contar a primeira execução como fuzz bem-sucedido nem desativar
  sanitizers. Evitar fuzz/comandos sandbox concorrentes durante o autoteste:
  corpus e montagem dos diretórios protegidos podem gerar avisos do tar.
- Runners do GitHub (Ubuntu 24.04) mantêm `kernel.apparmor_restrict_unprivileged_userns=1`
  e o bubblewrap do sistema não traz perfil AppArmor: `bwrap` falha com
  "setting up uid map: Permission denied" e todo job que usa o sandbox do checker morre
  junto (testes de logos-lrat/audit/ledger, `ci/check-audit.sh`, `ci/check-sat.sh`).
  Os mantenedores do runner não desativam isso por padrão (actions/runner-images#10015);
  cada workflow precisa habilitar. `ci/enable-userns.sh` aplica perfil AppArmor mínimo
  para /usr/bin/bwrap e, se preciso, desliga o sysctl; termina com controle positivo do
  próprio bwrap e falha alto se não conseguiu habilitar (nunca falso verde).
- `pytest ../tcb/tests` mede LOC real com `tokei` (via `tcb.build_report`); sem ele no
  PATH os quatro testes de grafo Cargo falham com "tokei não encontrado". O job
  `python` do CI instala via `taiki-e/install-action`, como o `tcb-report`; local já
  está instalado.



## Diário (apenas acrescentar)

- **2026-10-01** — Roadmap Revisão 2 e especificação movida para `docs/spec/` (`6453908`). Etapas 0.1
  (`e5b6f77`) e 0.2 (`05e038b`). Etapa 0.3 (`ce37670`): codificação canônica, spec, vetores,
  diferencial, fuzz, ADR 0003.
- **2026-10-01** — Revisão externa (GPT) achou 4 problemas, todos reproduzidos e corrigidos
  (`074fa00`): `Option<T>` colidia `None`/`Some(null)`; `CanonMap` Python mutável gerava bytes
  não canônicos; `RecursionError` antes do limite de profundidade; `selftest` da allowlist passava
  por "duplicate key". Lição: nunca mascarar o código de saída com `| tail` (ver `AGENTS.md` §4).
  Criados `AGENTS.md`, `CLAUDE.md` e este arquivo.
- **2026-10-01** — Etapa 0.4 (`logos-crypto`): hash com domínio (16 tags reservadas), `hash_raw`/`RawHasher`,
  `ExternalDigest`, `SigAlg` (ES256 só reservado), `PublicKey`/`Signature`/`KeyId` com `alg`, `Signer`/
  `FileSigner` (0600, sem symlink, fstat), `verify` estrito; `spec/domains.md`, vetores (RFC 8032 + gerados
  independentemente), ADR 0004. Achados no caminho: o `PUBLIC KEY` do TEST 3 que eu lembrava estava errado
  (só descoberto ao ler o RFC); um mutante "sobreviveu" por eu ter confiado num build obsoleto (mtime) e
  por um `git checkout` falhar num arquivo não versionado — o runner de mutação agora ajusta mtime e
  restaura por conteúdo; a verificação de algoritmo em `verify` era inalcançável pela API pública e ganhou
  teste unitário interno. `tcb.py` passou a medir só dependências do host e a separar execução × compilação.
  Criado `ci/verify.sh`.
- **2026-10-01** — Segunda revisão externa (4 achados, todos reproduzidos): (1) `FileSigner::load` tinha corrida
  `lstat`→`open` que permitia seguir symlink: agora abre com `O_NOFOLLOW|O_NONBLOCK` e só então faz `fstat`
  (testes de symlink, FIFO e de corrida; constantes por plataforma, aarch64 a confirmar); (2) `fuzz.yml`
  rodava `cargo fuzz` na stable por causa do `rust-toolchain.toml`: criado `ci/fuzz.sh` (mesmo script na CI e
  local, toolchain explícita; validado com um nightly datado). Meu conselho `RUSTC_BOOTSTRAP=1` no
  `AGENTS.md` foi o que escondeu o bug: removido; (3) o mutante `verify` frouxo **não** era equivalente — quem
  conhece a chave forja `R` = identidade; teste com controles e ADR 0004 corrigido; (4) `requires-python`
  agora `>=3.14,<3.15`. Lição: "equivalente" só vale com prova; e ambiente local com atalhos (`RUSTC_BOOTSTRAP`)
  não valida a CI.
- **2026-10-02** — Revisão confirmou parada após 0.4; `ci/verify.sh --selftest` passou
  localmente (`VERIFY_OK`). Criado plano da 0.5 em `docs/plans/etapa-0.5.md`, com APIs,
  sequência, quatro critérios rastreados e decisões propostas para ADR 0005. Implementação
  não iniciada; CI remota/fuzz longo continuam pendentes. Descoberta: `tcb/history/` vazio,
  apesar do entregável de histórico JSON versionado da 0.2; não corrigido nesta etapa de planejamento.
- **2026-10-02** — Ajustado plano da 0.5 após revisão: durabilidade independente entre
  writers, teste determinístico de reader durante publicação (mutante sem tmp+rename),
  injeção de falhas de fsync/reserva e byte de excesso fora do `Vec`. Testes descritos
  são critérios da futura implementação, não evidência de CAS já implementado.
- **2026-10-02** — Planejamento solicitado para a próxima etapa: confirmada 0.5 (CAS),
  ainda não implementada. Mantido plano existente e completadas propostas de representação
  de `Limits` e propagação conjunta de erro primário/limpeza, sem ampliar escopo.
  Links locais válidos; `ci/verify.sh --selftest` terminou com `VERIFY_OK` localmente.
  CI remota e fuzz longo permanecem pendentes; nenhuma implementação do CAS nesta sessão.
- **2026-10-02** — Implementada Etapa 0.5: CAS em `logos-ledger`, limites por tag,
  leitura bounded com reserva falível/probe externo, tmp+rename e fsync independentes por
  writer, integridade em `get`, erro composto de limpeza; ADR 0005 e `spec/cas.md`.
  17 testes passaram; smoke real confirmou três vetores, reabertura, corrupção e excesso.
  Seis mutantes mortos por asserções e restaurados. Fuzz final: 798945 execuções em 61 s
  sem crash. `ci/verify.sh --selftest` terminou com `VERIFY_OK`; relatório local da TCB:
  2433 LOC próprios + 78287 de dependências de execução (51581 só compilação).
  Primeiras verificações falharam em compilação/lints e na formatação Python; corrigidas
  antes da verificação completa final. Próximo passo: 0.6; CI remota/hora de fuzz pendentes.
- **2026-10-02** — Criado plano da Etapa 0.6 em `docs/plans/etapa-0.6.md`: APIs,
  latch de exaustão com finalização central, proposta `core/v1`, vetores independentes,
  `trybuild`, mutações e matriz CI nativa x86_64/aarch64. Identificado limite dos enums
  públicos: ausência de conversões não impede construção direta de veredictos.
  Implementação não iniciada; propostas aguardam ADR 0006. Runner ARM ainda ausente,
  fechamento S3 e demais verificações remotas continuam pendentes.
- **2026-10-02** — Verificação do planejamento 0.6: links locais, blocos de código e
  seis seções do formato do roadmap validados; `ci/verify.sh --selftest` terminou
  com `VERIFY_OK` localmente. Nenhum teste de comportamento da 0.6 executado:
  o crate continua esqueleto; os testes descritos no plano são trabalho futuro.
- **2026-10-02** — Implementada Etapa 0.6: tipos/medidor `logos-core`, finalização
  com latch irreversível, funções nomeadas sem promoção de rejeição/exaustão,
  `core/v1`, ADR 0006, 126 vetores independentes e CI nativa x86_64/aarch64.
  21 testes (inclui harness com 7 compile-fail e controle positivo), smoke público
  e 11 mutantes mortos/restaurados. Fuzz `core_decode`: 224269 execuções em 61 s,
  sem crash. `ci/verify.sh --selftest` terminou com `VERIFY_OK` localmente.
  Ruff/mypy/clippy iniciais falharam e foram corrigidos antes do run completo final.
  API GitHub confirmou repositório público; sem push/execução ARM/CI nova observada.
  `trybuild` trouxe `itoa` à allowlist; selftest agora usa `either` e passou pelo
  diagnóstico certo. TCB: 2736 LOC próprios + 78287 execução (51581 compilação).
  Alterações recebidas no manifesto CAS e log `cas_domain` preservadas fora do commit;
  verificação completa usou a árvore com esse hunk CAS. Próximo passo: 0.7.
- **2026-10-02** — Corrigidos os dois achados da revisão: manifesto de `logos-ledger`
  lista `src/cas.rs`, `src/cas_domain.rs` e `src/lib.rs`; gate do auditor usa metadata
  completo e grafo resolvido por `PackageId` com `--all-features`, sem ignorar pacotes externos.
  Smoke real com ponte externa opcional e aliases retornou erro por dependência de
  `logos-ledger`. Selftest cobre ponte permitida para `logos-core` e bloqueios transitivos
  para `logos-ledger` e `logos-kernel`; todos passaram pelo diagnóstico esperado.
  `bash -n`, conferência do manifesto e `ci/verify.sh --selftest` passaram em árvore
  limpa exportada do conteúdo staged; run completo terminou com `VERIFY_OK`.
  Alterações locais de fuzz e logs ficaram fora dessa árvore e do commit.
  Sem push nem nova verificação remota. Próximo passo permanece 0.7.

- **2026-10-04** — Revisadas primitivas canon/crypto/core/CAS e suas fronteiras
  contra spec/ADRs; sem bug novo identificado no recorte. Implementada 0.7:
  plano, threat-model (8 atacantes/18 ameaças), guarantees (G1–G3 condicionais
  com premissas/TCB/estado), limits (L1–L3 e ameaças fundamentais/TOCTOU).
  Apêndice B explicita que mitigação futura não resolve limites fundamentais;
  três critérios documentais demonstrados por revisão e `DOCS_0_7_OK`.
  `ci/verify.sh --selftest` terminou com `VERIFY_OK`, incluindo `CORE_SMOKE_OK`;
  Python: 104 testes + 7 da TCB passaram. `cargo deny` emitiu dois avisos
  `license-not-encountered`, sem falha. Smoke Rust temporário com arquivos reais:
  canon estrito, separação de assinatura, latch/enums públicos e CAS com
  reabertura, corrupção e excesso passaram (`REVIEW_SMOKE_OK`); temporários removidos.
  Dois jobs de revisão falharam por modelo indisponível; revisão direta substituiu
  a investigação, sem substituir modelo. Alterações locais em fuzz/logs preservadas;
  sem push/CI remota/novo fuzz nesta sessão. Próximo passo: 1.1.

- **2026-10-04** — Etapa 1.1 implementada: Sort/Symbol/Term/Formula/LogicId
  `FOL-ms-eq/v1`, De Bruijn, AST de prefixo plano e limites estruturais agregados.
  ADR 0007, spec air/v1, plano de etapa/fase, 168 vetores stdlib independentes,
  exemplo público, target air_decode e integração de CI/manifesto/documentação.
  11 testes AIR passaram; ∀x.P(x)/∀y.P(y) com bytes/hash idênticos e milhões de
  records/bytes recusados em stack de 64 KiB. Quatro mutantes mortos por
  asserções, restaurados antes de `ci/verify.sh --selftest`: `VERIFY_OK`,
  `CORE_SMOKE_OK`/`AIR_SMOKE_OK`, 104 testes Python + 7 TCB.
  Fuzz oficial nightly-2026-09-28: 182529 execuções em 61 s sem crash.
  Clippy/mypy/ruff iniciais corrigidos; dois avisos de licenças não encontradas
  em cargo deny, sem erro. TCB de correção: 3424 LOC próprios/78287 runtime
  (+51581 build-only); sem dependência externa nova. CI remota/ARM e ≥1 h
  noturna ainda não observados; sem push. Próximo passo: 1.2.

- **2026-10-04** — Revisão da codebase implementada com quatro revisores
  (canon, crypto/CAS, core, AIR) e inspeção direta de CI/manifesto/medidor.
  Dois P2 reproduzidos sem alterar produção: comentário satisfaz guard de
  deny e permite unwrap fora de testes (guard/Clippy exit 0; controle ativo
  exit 101); dependência workspace normal fora da propriedade desaparece do
  relatório transitivo (3 LOC reportados, 1 LOC omitido). Smokes core/AIR e
  programa temporário com arquivos reais passaram; erro inicial da asserção
  de symlink corrigido para KeyFileNotRegular, conforme contrato. Nenhum
  achado runtime novo no recorte; sem full verify/testes/fuzz/CI remota
  nesta revisão. Temporários removidos, código não corrigido nem commitado.
  Próximo passo: corrigir guards/medição, depois retomar 1.2.

- **2026-10-04** — Especificada extensão proposta de memória de longo prazo
  após leitura integral da spec e referência OptChat: Episodic/Knowledge/Router,
  24 seções, schemas/algoritmos, M1–M12, 20 acceptance cases, benchmarks/ordem
  e alterações cirúrgicas. ADR 0008 proposto; sem módulos, novos formatos
  implementados, checkmarks ou correção dos P2 anteriores. Smoke estrutural
  de 1M eventos passou; #42 original não NFC recuperado, frame <=8192,
  folding com aumento privado terminou bounded. Fixture real SQLite/SIGKILL
  e fonte nativa corrompida/ausente passaram. Tentativas iniciais de runner
  atingiram deadlines 300s/90s; runners ajustados passaram, temporários removidos.
  Sem full verify/CI/commit/push; próximo passo de código continua 0.1/0.2.
- **2026-10-04** — Smoke adicional da proposta de memória: SIGKILL real após
  commit event+turn; retomada usou snapshot anterior à entrada atual.
  Receipt Delivered retornou resposta salva sem novo evento/chamada.
  Fixture removida; continua verificação de referência, não módulo implementado.

- **2026-10-04** — Corrigidos os P2 pendentes dos controles 0.1/0.2:
  scanner do preâmbulo Rust ignora comentários/literais/nested attrs/cfg_attr;
  closure inclui dependências workspace por propriedade sem duplicar roots.
  Classificação runtime/build deixou de vazar entre propriedades. CLI real
  da fixture confirmou own=3/deps=1/total=4; 25 testes TCB + 104 Python e
  selftest completo passaram (`VERIFY_OK`), incluindo bloqueios de lints
  comentados e unwrap ativo. Primeiro run parou em E501 de literal na fixture;
  literal repartido sem alterar os bytes e run completo final passou.
  Dois avisos cargo-deny de licenças não encontradas, sem falha. Agente do
  guard falhou com PROHIBITED_CONTENT; correção concluída diretamente.
  Snapshot limpo de `a568d1e` medido/arquivado; worktree/fixtures removidos.
  Fuzz e proposta de memória preservados fora da correção; sem push/CI remota.
  Próximo passo: 1.2.

- **2026-10-05** — Etapa 1.2 implementada em logos-air: Vocabulary versionado
  imutável, SortDecl/SymbolDecl, Defined/Anchored/Glossed, refs opacas,
  enumerações/fonte, Kahn iterativo e extensão com compatibilidade direcional.
  Novo formato vocabulary/v1 e h(V) com tag reservada; ADR 0009/plano/spec,
  manifesto correção+fidelidade, decoder/fuzz e CI integrados. 39 testes
  (38 behavior + 53 vetores em um leitor), cadeia de 1000 em stack 64 KiB,
  smoke público e quatro mutantes mortos. Fuzz: 236898 execuções em 61 s.
  Primeiro run de vetores falhou por nome incorreto NonCanonicalInt; fixture
  corrigida para NonMinimalHead do canon/v1. Primeiro full verify falhou em
  op_ref/manual_is_multiple_of dos artefatos novos; corrigidos e
  `ci/verify.sh --selftest` terminou VERIFY_OK (exit 0), 104 Python + 25 TCB.
  Dois avisos cargo-deny de licenças não encontradas, sem falha. LSP novamente
  falhou ao inicializar; compilação/Clippy/testes usados. Medição: correção
  4006+78287=82293 LOC; fidelidade 1275+80034=81309; build-only 51581.
  Workspaces de mutação removidos; proposta de memória preexistente preservada,
  fora do commit da implementação. Sem push/CI remota; próximo passo: 1.3.

- **2026-10-05** — Etapa 1.3 implementada: checker iterativo de fórmulas
  fechadas em logos-air, kind/result/aridades, sorts, igualdade e De Bruijn;
  erros estruturados com record Formula/posição Term/record Term. Sem expansão
  de Defined, inferência, formatos novos ou deps externas TCB. ADR 0010,
  plano/spec, manifesto, smoke público, fuzz e job sort-coverage integrados.
  24 testes (22 behavior + 2 propriedades), 2000 fórmulas bem tipadas e 2270
  mutações de entradas rejeitadas; contagens por classe
  [331,829,312,312,174,312], seed 0x5eedc0de123456a2. LLVM 36/36 ramos (100%),
  179/182 linhas e 186/192 regiões, sem exclusões; quatro mutantes compilados
  mortos pelos testes. Fuzz com 17 pares canônicos: 206912 execuções em 61 s.
  Primeira integração falhou em dois moves E0382 de fixtures; corrigidos.
  Primeiro full verify falhou em dois collapsible_if das propriedades;
  corrigidos e `ci/verify.sh --selftest` terminou VERIFY_OK (exit 0), 104 Python
  + 25 TCB. Dois avisos cargo-deny conhecidos, sem falha. Instalados
  cargo-llvm-cov 0.9.1/llvm-tools da nightly datada para medição real.
  Shell comprimiu stdout de --nocapture; contagens observadas por subprocess
  direto (problema da ferramenta registrado). TCB: correção 4281+78287=82568,
  fidelidade 1550+80034=81584, build-only 51581. Workspaces temporários removidos;
  proposta de memória preexistente preservada fora do commit; sem push/CI remota.
  Nova execução final expôs teste frágil preexistente no CAS (exit 101):
  ptr::eq de dyn Error compara vtables, cuja identidade não é garantida.
  Removido apenas esse assert incidental; verificações dos dois erros/diagnóstico
  mantidas, sem mudar produção. Teste específico passou (exit 0) e nova
  verificação completa terminou VERIFY_OK, exit 0 observado por subprocess.
  Medição final após remoção: correção 4277+78287=82564; demais valores iguais.
  Próximo passo: 1.4.
- **2026-10-05** — Etapa 1.4 implementada fora da TCB: gramática air-text/v1,
  ADR 0011, parsers/printers independentes Python e logos-air-text, metadata/
  grounding completos, binders → De Bruijn e citações opacas sem autoridade.
  31 Rust + 71 Python novos; propriedades 500 ASTs/lado e diferencial de 500
  fórmulas contra AST/bytes independentes, mais 13 oráculos de precedência/escopo.
  Integração inicial encontrou stack overflow Rust, RecursionError e perda de
  operadores em grupos Python: pilhas explícitas substituíram recursão/fastpath.
  Profundidade lógica 128 aceita/129 rejeitada, grupos 200/500 preservados,
  input adversarial de 10000 controlado por LimitExceeded. Digest Unicode de
  64 bytes causava pânico Rust; reproduzido, corrigido por byte pairs e regressão
  passou. Erros intermediários de compilação/indentação/mypy/clippy e fixture
  inválida corrigidos; colisão Cargo do exemplo diff eliminada por air_text_diff.
  AIR_TEXT_SMOKE_OK e PY_AIR_TEXT_SMOKE_OK; ci/verify.sh --selftest terminou
  VERIFY_OK (exit 0), 175 Python + 25 TCB, vetores/supply-chain/guards completos.
  TCB: correção 4277+78287=82564, fidelidade 1550+80034=81584, build-only 51581;
  nenhum formato binário/dependência externa TCB novo. Memória preexistente
  preservada fora do commit; sem push/CI remota. Próximo passo: 1.5.
- **2026-10-05** — Etapa 1.5 implementada na TCB: generate resolve refs de
  eventos em LedgerView snapshot abstrato, recusa desconhecidos/retratados/época
  inválida/não citáveis, confere sorts e compatibilidade direcional, ordena/dedup
  e produz obligation/v1 privado/imutável, canon/h(O) só de saída e sem época.
  ADR 0012/spec/manifesto integrados; 19 testes Rust, 20 vetores Python/hashlib
  independentes, cinco negativos compile-fail E0451/E0277 e controle positivo.
  Seis mutantes compilados mortos: Pending aceito, época ignorada, dedup ausente,
  compatibilidade invertida, check goal ausente e goal omitido do codec.
  Integração inicial falhou em APIs AIR inexistentes/Result<Hash>/move parcial,
  controle positivo UI e stderr ainda não observado; corrigidos, negativos
  capturados do compilador real. Asserts incidentais/tautologias removidos.
  Clone_on_copy/ruff corrigidos; smoke duplicado removido. OBLIGATION_SMOKE_OK;
  ci/verify.sh --selftest terminou VERIFY_OK (exit 0), 175 Python + 25 TCB,
  Rust workspace, guards, smokes e vetores completos. TCB correção:
  4610+78287=82897 (+333 próprios), fidelidade 1550+80034=81584, build-only 51581.
  Sem nova dependência externa runtime ou decoder de obrigação; AIR/Vocabulary
  anteriores inalterados. Não implementa ledger real/Verified/kernel.
  Memória/logs preexistentes preservados fora do commit; sem push/CI remota.
  Próximo passo: 1.6.
- **2026-10-05** — Etapa 1.6 implementada sem subagentes: logos-protocol fora
  da TCB, JSON proposal/v1 versionado com campos obrigatórios, extras/duplicatas/
  tipos/hex recusados; Formula por AIR canônico, PremiseRef/ClaimId distintos,
  texto UntrustedText preservado inclusive NFD/controles e excluído de O.
  spec/protocol.md/schema, ADR 0013, seis vetores independentes e smoke integrados.
  17 regressões Rust passaram. Teste antes da correção revelou derive serde
  aceitando structs como arrays; decoder agora exige mapas em ambos os níveis,
  regressão falhou antes/passou depois. Clippy chunks_exact/ruff corrigidos;
  exemplo normativo de hex alinhado ao envelope AIR real. Dois mutantes
  compilados mortos (decoder permissivo e Fact no schema), fontes restauradas.
  PROTOCOL_SMOKE_OK: JSON → candidato → geração real com snapshot em memória,
  h(O) invariável frente a texto/ordem, Pending recusado mesmo marcado especulativo.
  ci/verify.sh --selftest terminou VERIFY_OK, exit 0: workspace Rust, smokes,
  guards, vetores e supply-chain, 175 Python + 25 TCB. Avisos cargo-deny:
  duas licenças não encontradas e versões syn 2/3, sem falha; versões já estavam
  no lockfile, nenhum pacote externo novo. TCB correção 4610+78287=82897,
  fidelidade 1550+80034=81584, build-only 51581, inalterada. Sem parser TCB novo,
  sem transporte/ledger/especulação/kernel. Memória/logs preexistentes preservados
  fora do commit; sem push/CI remota. Fase 1 demonstrada localmente; próximo: 2.1.
- **2026-10-05** — Etapa 2.1 implementada: logos-model TCB com FiniteModel
  imutável, h(V) contextual, domínios clássicos não vazios/enumeração ordinal,
  constantes, funções densas totais packed big-endian e predicados ordenados.
  Sem tabelas Defined, avaliador, Z3, checker ou kernel. ADR 0014, model/v1,
  planos fase/etapa, manifesto, CI e gerador stdlib integrados.
  15 testes Rust e 32 vetores independentes passaram; MODEL_SMOKE_OK.
  Regressão de encoder ignorando limites de células/cardinalidade falhou antes
  e passou após correção. Integração dos testes inicialmente falhou por API
  Array::from inexistente; fixture endian era unary com apenas um output:
  corrigida para nullary. Caso packed duplicado/contagem incidental removidos.
  Quatro mutantes compilados mortos por asserções: parcialidade, fronteira de
  domínio, h(V) ignorado e cardinalidade enumerada frouxa no decoder.
  Full verify inicialmente parou em collapsible_if e helper obsoleto; corrigidos
  sem allow. ci/verify.sh --selftest final: VERIFY_OK, exit 0, workspace Rust,
  smokes, guards, supply-chain/vetores, 175 Python + 25 TCB. Avisos deny de duas
  licenças não encontradas e versões syn 2/3, sem erro.
  Fuzz final nightly datado: 212019 execuções/61 s sem crash; anterior 391395/61 s.
  TCB correção 5538+78287=83825 (+928 próprios), build-only 51581; logos-model 930.
  Memória/logs/adições preexistentes ao STATUS preservados fora do commit.
  Sem push/CI remota; próximo: 2.2, com expansão Defined/Const/Func ainda a especificar.
- **2026-10-05** — Etapa 2.2 implementada: eval(M,V,φ,Budget) contextual
  na TCB, pilhas explícitas compartilhadas, sem recursão/cópia de AST/tabelas
  Defined. ADR 0015 fixa Pred argumento i no índice i; Const/Func resultado 0,
  argumentos 1+; todos corpos tipados, valores globais totais/únicos inclusive
  não usados, constantes enumeradas compatíveis. Unknown(Steps)/erros tipados
  nunca viram bool; mem_bytes/size_bytes não são tetos físicos desta API.
  30 regressões Rust; 500 níveis de termos/fórmulas e 250 Defined em 64 KiB.
  Dois bugs de integração reproduzidos e corrigidos com regressões: função
  nulária buscando consts e f(g(c)) pulando f após expansão. Máquina única
  removeu Vec/máquina por termo/candidato. Cinco mutantes compilados mortos:
  Eq/Iff invertido, eixo funcional errado, múltiplos resultados aceitos,
  passos de nó omitidos, h(V) ignorado. Fontes restauradas.
  Z3 5.1.0.0 dev: 100 modelos, 10k pares com quantificadores nativos,
  5783 true/4217 false, zero divergências; oráculo não é produtor 2.4.
  EVAL_SMOKE_OK. Fuzz final model_eval 13591/61 s sem crash; anterior39145/61 s.
  Cobertura sort-check 36/36 ramos LLVM, 100%, sem exclusões.
  Full verify parou primeiro em collapsible_if/chunks_exact/is_multiple_of;
  ruff/mypy detectaram import obsoleto e tuplas canon invariantes. Corrigidos,
  sem allow. ci/verify.sh --selftest final: VERIFY_OK, exit 0; workspace,
  smokes/guards/vetores/supply-chain/selftest, 176 Python + 25 TCB.
  Avisos deny já presentes (duas licenças não encontradas e syn 2/3), sem erro.
  TCB correção6207+78287=84494, fidelidade1559+80034=81593, build-only51581.
  logos-model1590; núcleo537 vs. alvo<400, desvio explícito em ADR.
  Nenhum formato/domínio/dependência runtime novo. Memória/logs/adições prévias
  ao STATUS preservados fora do commit. Sem push/CI remota; S3 ARM pendente.
  Próximo: 2.3, fixar decoder contextual da obrigação e checkers P2.
- **2026-10-05** — Etapa 2.3 implementada: ModelChecker com refutação/consistência,
  decoder contextual de obrigação dos mesmos bytes/h(O), CheckedObligation
  separado de Obligation gerado (P1), refs estritas e limite agregado de nós.
  Budget compartilhado: bytes/contexto/Defined globais/premissas/goal; uma máquina
  e pilhas por execução, sem reset nem sort-check/rehash de V no evaluator.
  Consistência valida goal mas ignora verdade; Contexto inválido nunca aceita
  mesmo com Γ vazio. CAS real verifica bytes/h(M); ausência Rejected, I/O/
  integridade/teto do CAS Err explícito, exaustão preserva Unknown.
  Reasons canônicos preparados uma vez por processo, sem fallback de encoding.
  ADR 0016/plano/specs, manifesto, deps workspace/deny e workflows atualizados;
  nenhum formato/domain/dependência externa runtime novo. Sem ledger/roteamento.
  21 novos testes de checker + 15 decoder, 20 vetores positivos + 10 negativos;
  100 Rust seletivos passaram, incluindo controle de geração selada. 1000
  repetições S3 locais e predicado Defined quantificado em domínios 1/2.
  Fixtures inicialmente tinham SortDecl/Array incompatíveis e predicado ausente;
  corrigidos, inclusive lowercase que mascarava erro de tipagem. Clippy detectou
  collapsible_if/is_multiple_of/onze clones para slices; corrigidos sem allow.
  CHECKER_SMOKE_OK. cargo-mutants 27.1.0 final: checker 20 gerados, 12/12 viáveis
  capturados e 8 inviáveis; reader 21 gerados, 17/17 capturados e 4 inviáveis.
  Zero sobreviventes/timeouts, sem exclusões no checker. Inviáveis não contam
  como mortos/cobertura; não foi inventado Default para tipos selados/resultados.
  Fuzz final nightly datado: obligation_decode 10297/61 s, model_eval 3909/61 s,
  sem crash; primeiro parser23582/61 s. Sem RUSTC_BOOTSTRAP.
  ci/verify.sh --selftest: VERIFY_OK, exit 0, workspace Rust, smokes/guards/
  manifesto/independência/supply-chain/vetores, 176 Python + 25 TCB, selftest
  negativo completo; avisos deny preexistentes de licenças e syn 2/3, sem erro.
  TCB correção6677+78287=84964 (+470 próprios), build-only51581; obrigação550,
  modelo1845. Memória/diário/logs preexistentes preservados fora do commit.
  Sem push/CI remota; S3 ARM e fuzz ≥1h pendentes. Próximo: produtor Z3 2.4.
- **2026-10-05** — Etapa 2.4 implementada: logos_provers.z3_models fora da TCB,
  refute Γ∧¬goal/witness Γ via SMT-LIB real e Solver.from_string, contextos Z3
  isolados, nomes posicionais; Open em todos vetores1..k e Enum fechado/ordinal.
  Defined globais universais exigem totalidade/unicidade, inclusive não usados
  e membros Defined enumerados; extração serializa só primitivos contra V exato.
  Limites domínio4/timeout5000ms compartilhado; unknown/timeout sem certificado,
  sem hard RSS/wall. z3-solver5.1.0.0 passou de dev para runtime Python; nenhum
  formato/domínio/dependência Rust/TCB mudou. API não emite Verdict nem usa farm.
  Duas fatias paralelas integradas; revisão local removeu ASTs/encoders de testes
  duplicados, corpus com templates repetidos e timeout que aceitava bytes ou None.
  Reusados AST/oráculo independente 2.2, com 200 contextos/obrigações distintos;
  controles200/200 e candidatos200/200 aceitos pelo ModelChecker real.
  Z3 falso na fronteira de bytes: 10k tentativas, 2100 aceitos legítimos,
  7900 rejeitados, zero aceitação indevida; defeitos estruturais isolados.
  Mutação de negação no tradutor produziu modelo aceito como consistência,
  mas rejeitado como refutação contra O original. Sem matching de texto SMT.
  Três falhas reproduzidas antes/corrigidas depois: versão bool emitia candidato;
  1500 Not planos causavam RecursionError; extração expirada devolvia certificado.
  Agora ProducerError/ProducerError/None, com regressões determinísticas.
  19 regressões passaram em 31,87s; Z3_MODELS_SMOKE_OK e smoke de digest direto
  lower/uppercase/curto/longo/não-hex/extras. Ponte Rust sem Vec por campos/hash.
  Ruff/mypy iniciais detectaram defaults/linhas/if, tipos de locals e imports
  privados; corrigidos sem allow. Rust-analyzer continua ausente na stable.
  Full verify/selftest passou duas vezes; final VERIFY_OK com exit0 explícito,
  195 Python +25 TCB, smokes/guards/manifesto/independência/supply-chain/vetores
  e bloqueios pelos motivos corretos. Avisos deny preexistentes sem erro.
  TCB correção6677+78287=84964, fidelidade1559+80034=81593, build-only51581,
  inalterada. Sem fuzz novo nesta etapa não-TCB. ADR0017/plano/API/README,
  STATUS/ROADMAP e smoke no job Python/verify atualizados.
  Memória/diário/fuzz-src/logs preexistentes preservados fora do commit.
  Sem push/CI remota; S3 ARM/fuzz longo anteriores pendentes. Próximo: 2.5.
  Smoke runtime adicional: uv run --isolated --no-dev --locked executou o produtor
  e checker real com somente dois pacotes instalados; Z3_MODELS_SMOKE_OK.

- **2026-10-05** — Etapa 2.5 implementada: segundo produtor exaustivo Python,
  domínio 1..4, sem importar Z3; enumeração lazy das interpretações primitivas
  e cardinalidades Open, Enum ordinal fixo e Defined global em ordem de
  dependência. Resultado FOUND/NO_MODEL_WITHIN_BOUND/EXHAUSTED distingue
  busca completa e recursos, sem autoridade ou conclusão geral de validade.
  API model_fallback tenta Z3 e, sem candidato, usa finder; timeout separado
  por produtor e erros propagados. AST/validação/ProducerError migrados para
  model_context sem alias/reexport legado; consumidores da 2.4 atualizados.
  Corpus 296 obrigações distintas (256 pares booleanos + 40 quantificadas/
  funcionais) contra enumeração independente de 682 modelos ≤3: 201/201
  contramodelos encontrados e aceitos pelo checker Rust real, 95 negativos
  completados. Smokes sem import Z3 e público passaram: native rlimit=1
  produziu unknown real, fallback Accepted e wrong h(O) Rejected.
  uv run --isolated --no-dev --locked executou smoke com dois pacotes;
  MODEL_FINDER_SMOKE_OK, exit 0. 22 regressões novas cobrem limites 3/4,
  sorts mistos, Enum Defined ordinal, nulários, tabela retangular 2×3,
  binder isolado, definições globais inválidas e orçamento/clock/cap de saída.
  Primeiro alvo: 12 passed/1 failed por fixture que supunha não unicidade de
  defined(true) em singleton; produtor estava correto. Fixture exige ≥2 agora
  e singleton tem controle positivo. Removidos wiring/texto incidental e
  timeout não determinístico; Ruff inicial 30/mypy 17 corrigidos, sem allow
  global. Ruff/format/mypy finais passaram (29 arquivos); alvos 41 passed
  em 37,13s, preservando corpus Z3 200/200, adversário 10k sem indevidos e
  mutação da negação rejeitada. Full ci/verify.sh --selftest: VERIFY_OK, exit 0,
  147,04s; 217 Python (93,66s) + 25 TCB (1,34s), vetores/smokes e controles
  de CI bloqueando pelos motivos corretos. Avisos deny preexistentes apenas.
  Formatos/dependências/TCB Rust inalterados; sem novo fuzz nesta etapa
  não-TCB. ADR0018/plano/API/README/STATUS/ROADMAP e verify/job Python
  atualizados. Memória/diário/fuzz-src/logs preexistentes preservados fora
  do commit. Sem push/CI remota; S3/ARM/fuzz longo anteriores pendentes.
  Próximo: 2.6, apresentação portuguesa sem garantia de fidelidade.
- **2026-10-05** — Etapa 2.6 implementada: explain_model(M_bytes,V_bytes,O_bytes)
  fora da TCB, com glosas escapadas/fallback nome, elementos Sort#ordinal,
  valores funcionais e tuplas verdadeiras só de símbolos sintáticos de O.
  Defined é derivado globalmente via intérprete não confiável existente,
  sem mostrar dependências; representantes Enum ausentes de O ficam ocultos.
  Cada premissa e objetivo AIR têm valoração real; texto inclui h(M) e aviso
  não autoritativo. Até 100 entradas com omissão explícita; domínios compactos,
  tabelas densas ≤100k agregadas e orçamento cooperativo 1M passos/5s.
  Certificado válido esparso pode não caber na apresentação; ProducerError
  não muda validade ou resultado do checker. Sem paráfrase da 9.1.
  Corpus público reproduzível de 20 refutações distintas, todas Accepted pelo
  checker Rust real; 10 regressões passaram em 0,72s. Smoke público e
  uv run --isolated --no-dev --locked passaram (2 pacotes), com
  MODEL_EXPLAIN_SMOKE_OK accepted=20/20 false_goals=20 hashes=20.
  Primeiro alvo: 8 passed/1 failed por asserção incidental proibindo sintaxe
  AIR legítima app(symbol(...)); teste redundante removido, sem re-pin.
  Fixture de tuplas invertidas inicialmente não exercitava caminho porque
  selecionava caso funcional sem predicados; substituída por tuplas explícitas.
  Ruff inicial 11 linhas longas/mypy 1 erro de lista corrigidos;
  Ruff/format/mypy finais passaram. Full ci/verify.sh --selftest:
  VERIFY_OK, exit 0, 150,71s; 227 Python (95,39s) + 25 TCB (1,48s).
  Smokes/vetores/controles de CI passaram; avisos deny preexistentes apenas.
  ADR 0019, plano, API/spec/README, ROADMAP/STATUS e verify/job Python
  atualizados. TCB/formato/dependências Rust inalterados. Memória/diário,
  alterações fuzz-src e logs preexistentes preservados fora do commit.
  Sem push/CI remota; pendências S3/ARM/fuzz longo continuam separadas.
  Próximo: 3.1, catálogo de eventos v1, sem persistência da 3.2.
- **2026-10-05** — Etapa 3.1 implementada: catálogo assinado event/v1 (13
  tipos), Envelope/CheckRecord/Consistency/Authority/Route/EventProposal,
  texto tainted UTF-8 sem normalização. Signature/KeyId externos vinculados;
  recibos administrativos verificáveis separadamente, sem inferir privilégio.
  Verified/Refuted exigem Accepted, UnknownRecorded exige Unknown; used
  subconjunto ordenado de cited, testemunha com mesmo h(O), Unknown explícito.
  Caps wire 16 MiB, texto 64 KiB, refs 1024 e AIR padrão, erros explícitos.
  Python canon/hashlib/cryptography produziu 18 vetores para todas as tags
  e variantes; Rust decode/encode/hash/verify e Signer real coincidiram.
  15 regressões passaram; EVENT_FIELDS_TAMPER_OK vectors=18 mutations=599.
  EVENTS_SMOKE_OK, autoridade real, chave errada/adulteração rejeitadas,
  consistência Unknown preservada. Fixtures não demonstram validade lógica.
  Cargo-mutants: 18/18 viáveis capturados em assinatura/KeyId/outcome/
  consistência/validação, zero sobreviventes; limiares de tamanho excluídos.
  Fuzz event_decode cobre oito decoders: 55 seeds iniciais, 98154/61 s;
  runner permanente passou a semear 18 wires válidos da spec também na CI,
  execução final 123392/61 s sem crash. max_len padrão 4096 neste smoke;
  limites maiores têm regressões, não prova de fuzz longo.
  Compilação inicial falhou em acesso privado a Reason e helper sombreado;
  corrigidos. Ruff/mypy/clippy iniciais tiveram erros, corrigidos;
  verificações finais passaram. Full ci/verify.sh --selftest: VERIFY_OK,
  exit 0, 249,58s; 227 Python (130,40s) + 25 TCB (1,52s).
  Autoteste concorrente avisou tar por corpus alterado pelo fuzz; execução
  isolada final passou sem esses avisos (14,26s). Avisos deny preexistentes.
  TCB: 7892+78287=86179 correção, build-only51581; ledger1877, +1215 próprios;
  dependências novas diretas só crates TCB locais, serde_json apenas dev.
  ADR 0020/spec/plano/vetores/manifesto/allowlist e CI/docs/STATUS atualizados.
  Sem appender/persistência/índices/kernel; formatos antigos inalterados.
  Memória, fuzz-src e logs preexistentes preservados fora do commit.
  Sem push/CI remota; próximo: 3.2, appender SQLite durável e recuperação.
- **2026-10-15** — Etapa 3.2 implementada sem subagentes: Ledger::open/append/
  head, conexão privada rusqlite0.40.2 bundled/limits, SQLite3.53.2 WAL/FULL,
  BEGIN IMMEDIATE e lock std::fs::File mantido até fechar a conexão. Replay
  completo confere wire/assinaturas com chave externa fixa, seq/prev/epoch e
  schema ledger-sqlite/v1; sem reparação silenciosa. Signer defeituoso não
  persiste evento, erro de SQL/sync envenena append/head até reabrir.
  ADR0021/spec/plano/vetor layout/manifesto/allowlist/locks e CI atualizados;
  event/v1 e demais wires/domínios permanecem iguais. 48 testes no crate,
  16 novos (14 integração, incluindo entrada de subprocesso, +2 unitários).
  Vetores independentes 18/18 reproduzidos byte a byte pelo appender;
  adulterações isolam assinatura, prev, seq/coluna/epoch e schema. Nove
  mutantes compilados capturados, sem erro de compilação contado como morto;
  fontes restauradas. Logs locais em target/ledger-mutations, reprodução
  descrita no plano. Primeiro alvo teve fixture JSON tratada como array em
  vez de objeto com vectors; corrigida. Clippy detectou import obsoleto e
  chunks_exact constantes; corrigidos sem allows novos.
  Worker release/oráculo canon/hashlib/Ed25519 Python público independente:
  10k eventos/24 SIGKILLs, zero perdas de appends reconhecidos e commits sem
  resposta preservados; segundo escritor real recusado antes da transação.
  Primeiro smoke62,57s/9999 reconhecidos; na verificação64,50s/9996;
  todos10000 persistidos e conferidos por ambos os leitores ao final.
  Full ci/verify.sh --selftest: VERIFY_OK, exit0; workspace/smokes/guards/
  supply-chain/vetores, 227 Python113,64s +25 TCB1,84s. Avisos deny anteriores
  apenas. Autoteste concorrente avisou tar por corpus alterado; repetição
  com comando sandbox concorrente avisou diretório raiz. Execução final
  isolada passou sem avisos e com todos os controles pelo diagnóstico certo.
  Fuzz nightly-2026-09-28 no script oficial: primeira execução132337/61s
  acabou exit1 por LeakSanitizer/ptrace do sandbox, não foi contada como
  sucesso. Repetição fora do sandbox139749/61s, exit0, sem crash, max_len4096;
  sem desativar sanitizers/RUSTC_BOOTSTRAP. Log preexistente restaurado;
  logs desta sessão em target/event-decode-3.2*.log. TCB Rust8235+105199=
  113434, build-only64508, ledger2220; SQLite C176466 contado separadamente,
  fora do medidor Rust. Sem índices/citabilidade/CAS-integrado/kernel/HSM/
  transparência. Memória, fuzz-src, mutants.out e logs preexistentes fora
  do commit. Sem push/CI remota; próximo: 3.3, índices e LedgerView reais.
- **2026-10-15** — Etapa 3.3 implementada sem subagentes: LedgerIndex privado
  em memória reconstruído de cadeia conferida, claims/deps/citable/metadata/
  revoked em cache index.sqlite3 derivado, comparação integral e backup/sync/
  rename atômicos. Cache adulterado não é autoridade. Borrow de Ledger::index
  mantém writer emprestado; Index standalone usa somente chave pública e
  mantém lock. Snapshots preparados resolvem V/O pelo CAS e reader completo,
  sort-check e scope opaco exato inclusive ancestors usados. Deps só de used;
  invalidação conservadora fecha dependentes, sem criar eventos de kernel.
  CLI logos ledger rebuild-index compara Missing/Equal/Different/Invalid e
  não lê chave privada. Novo vocabulary_for_check compartilha envelope/hash/
  caps do reader, sem abrir Obligation selada; wire anterior inalterado.
  ADR0022/plano/spec/novo vetor layout/manifesto/CLI/CI/README atualizados.
  Só feature rusqlite backup; cache de statements descartado por depender de
  hashlink. Nenhum pacote externo novo ou mudança da base ledger-sqlite/v1.
  14 regressões índices +1 header independente20 vetores +2 processos CLI
  passaram. Onze mutantes compilados capturados por asserções; fontes
  restauradas, nenhum erro de compilação contado como morto. Logs locais
  em target/index-mutations; receita no plano. Erros iniciais de API
  DatabaseName/FromSql u64, fixture canon_hex/canonical_hex e três Clippy
  needless_borrows corrigidos antes do full verify.
  Dataset realmente assinado/conferido: rebuild100k apagou cache e comparou
  todas as colunas/linhas ordenadas das cinco tabelas, cabeça preservada.
  1M/20000 consultas (5142 positivas/14858 negativas), p99 inicial27042ns;
  full verify36573ns, p5015580ns, máximo81136ns, build252133ms excluído da
  latência. Snapshot privado em memória, não cold start; fixture agrupa
  INSERTs somente para preparar dados, sem alegação de throughput do appender.
  Artefato target/ledger-index-scale.json ligado a upload por arquitetura na CI.
  Fuzz oficial obligation_decode/nightly-2026-09-28:92653 execuções/61s,
  exit0, sem crash, sanitizers ativos, max_len4096; fora do sandbox por ptrace
  já conhecido. Log anterior restaurado, novo em target/obligation-decode-3.3.log.
  ci/verify.sh --selftest: VERIFY_OK, exit0; workspace Rust, smokes/crash10k
  (24 SIGKILLs/9998 ACKs/zero perda), escala, manifesto/independência/lints/
  supply-chain/vetores,227 Python89,58s +25 TCB1,34s e autoteste sem avisos de
  tar, com bloqueios pelos diagnósticos corretos. Só avisos deny preexistentes.
  TCB Rust8974+105199=114173 (+739 próprios), build-only64508; ledger2937 e
  obrigação572. SQLite C176466 continua separado do contador Rust.
  Sem kernel/auditor/fatos/políticas/janelas de épocas/UI/ações novos.
  Descoberta para 3.4: critério de janela de fato antecede o objeto Fact da
  6.1; exige contrato/plano e bloqueio se depender desse objeto ausente,
  sem antecipar 6.1. Memória, fuzz-src alheios, logs e mutants.out preexistentes
  fora do commit. Sem push/CI remota; próximo: contrato da 3.4.
- **2026-10-15** — Contrato da Etapa 3.4 examinado. Confirmada ausência de
  Fact/FactRegistered/EpochRange em fontes reais; FactRef e EntryKind::Fact
  são abstratos. O critério de fato válido em3–5/citável em4/recusado em6
  depende da 6.1, que também recebe citação da5.3 ainda ausente. Implementação
  parada por AGENTS.md §1.3; nenhuma view falsa usada para fechar o critério.
  Plano3.4 e ADR0023 proposto registram evidência e opção A: mecanismo genérico
  na3.4, teste integrado preservado na6.1 com dependência explícita. Opção B
  conserva critério bloqueado e exige plano de ordem. Decisão do mantenedor
  pendente; ROADMAP recebeu apenas nota de bloqueio, sem mudar entregáveis/
  ordem/checkbox. Plano da fase e próximo passo atualizados. Nenhuma mudança
  de código, formato, dependência, manifesto ou vetor; sem novo fuzz/mutação.
  Links locais dos planos/ADR conferidos. Full ci/verify.sh: VERIFY_OK, exit0;
  Rust/smokes/guards/manifesto/independência/auditorias/vetores,227 Python88,90s
  +25 TCB1,28s. Crash10k/24 SIGKILLs/9998 ACKs sem perda; rebuild100k exato;
  1M/20000 consultas p99=29711ns, cold build253313ms excluído da latência.
  Só avisos deny anteriores, sem falha. --selftest não repetido nesta entrega
  documental; última execução verde permanece a da3.3. Alterações fuzz-src,
  memória, logs e mutants.out preexistentes preservados fora do commit.
  Sem push/CI remota; próximo: decidir contrato do ADR0023 antes de codificar.
- **2026-10-15** — Mantenedor adotou explicitamente proposta A do ADR0023;
  bloqueio inicial da3.4 resolvido antes do código por ajuste dos critérios
  ROADMAP3.4/6.1. Teste integrado Fact3–5/4/6 e dependência explícita de3.4
  preservados na6.1; nenhuma antecipação de fatos, políticas ou ações.
  EpochRange genérico em logos-core, limites privados inclusivos u64,
  construtor falível/getters/contains; sem Canon/Domain ou novo wire de intervalo.
  EpochAdvanced recebe policy_root opaco Option<bytes32> e vocab Option<h(V)>.
  Novo epoch-body/v2 exige algum opcional; ambos ausentes usam somente v1,
  rejeitando forma duplicada v2. Envelope/domínios/schema anteriores iguais.
  22 vetores Python/canon/hashlib/Ed25519 independentes; comparação com e46ebed
  comprovou18 anteriores idênticos em todos os campos. Quatro novos cobrem
  opcionais isolados/ambos e u64::MAX/digests zero; replay preserva22 wires.
  Seis regressões novas (intervalo×enumeração81 pares, codec/assinatura e
  Ledger/CAS/snapshot/generate reais nas épocas3/4/6);49 alvos passaram.
  Oito mutantes compilados capturados, fontes restauradas; nenhum erro de
  compilação contado como detecção. Logs/comandos target/epoch-mutations.
  Clippy passou. Ruff avulso inicial usou configuração errada fora de py/;
  na pasta correta apontou linha longa nova, formatada; Ruff final passou.
  Fuzz oficial nightly-2026-09-28: core_decode172375/61s e event_decode107491/61s,
  exit0 sem crash, sanitizers ativos/max_len4096. Fora do sandbox por ptrace
  conhecido; logs anteriores restaurados byte a byte, novos em target/*-3.4.log.
  Full ci/verify.sh --selftest: VERIFY_OK, exit0, Rust/smokes/guards/manifesto/
  independência/supply-chain/vetores,227 Python85,88s +25 TCB1,80s; autoteste
  sem avisos, bloqueios pelos diagnósticos corretos. Só avisos deny anteriores.
  Crash10k/24 SIGKILLs/9999 ACKs sem perda; rebuild100k/tabelas5 exatas;
  1M/20000 consultas p99=17756ns, cold build250150ms excluído da medida.
  TCB Rust9056+105199=114255 (+82 próprios), build-only64508; core344 e
  ledger2980, sem dependências novas. SQLite C176466 continua separado.
  ADR/plano/spec/README/ROADMAP/manifesto/STATUS atualizados. Memória, fontes
  fuzz alheias, logs e mutants.out preexistentes preservados fora do commit.
  Sem push/CI remota; próximo:3.5, integração CAS/append e GC só dry-run.
- **2026-10-15** — Etapa3.5 implementada: Ledger possui CAS obrigatório em
  DIRECTORY/cas, getter cas() para put e conferência de todas as referências
  antes de Signer::sign/SQL. CasReference deduplica cinco domínios e inclui
  todos os CheckRecords de consistência, Witness.model e EpochAdvanced.vocab.
  Arquivo regular/limites/hash exigidos; ausência/erro não consome seq/head
  ou envenena handle. Sem bypass público; replay antigo permanece legível.
  Helper ledger_references confere cadeia/schema/assinatura sob lock existente
  mantido até Drop; open_existing não cria raiz CAS. GC fora da TCB, somente
  --dry-run/chave pública, inventário dos16 domínios, rejeição de symlinks/
  layout/nomes inválidos, integridade de candidatos e raízes diretas completas.
  Marcação canônica conservadora por bytes32 em mapas/arrays, raw como folha;
  pode reter em excesso e não prova links ausentes/uso externo. Temporários
  separados, nenhum objeto removido; falha não emite lista parcial.
  Dez regressões novas (4 ledger/6 CLI);28 omissões individuais com contador
  de assinatura/SQL/head e put/retry, integridade/domínio/limites/tipos/links,
  WAL confirmado/locks/abertura sem criação e preservação de objetos/dados.
  78 testes ledger/CLI passaram. Compilação inicial de teste tentou update
  de enum como struct; campos explícitos corrigiram. Clippy apontou Vec
  desnecessário em fixture, trocado por array. Quatro testes CLI iniciais
  detectaram sidecars criados pelo SQLite read-only; contrato/testes passaram
  a preservar objetos e dados, documentando coordenação sem ignorar WAL.
  Nove mutantes compilados capturados por testes, fontes restauradas; nenhum
  erro de compilação contado como detecção. Logs target/cas-integration-mutations.
  Fuzz oficial event_decode/nightly-2026-09-28:99066/61s, exit0 sem crash,
  sanitizers ativos/max_len4096, fora do sandbox por ptrace conhecido.
  Log preexistente restaurado byte a byte; novo target/event-decode-3.5.log.
  Full ci/verify.sh --selftest: VERIFY_OK, exit0; Rust/smokes/lints/manifesto/
  independência/supply-chain/vetores,227 Python90,61s +25 TCB1,77s; autoteste
  sem avisos e bloqueios pelos diagnósticos corretos. Só avisos deny anteriores.
  Crash10k/24 SIGKILLs/9999 ACKs/64,90s, zero perdas; oráculo independente
  confere todos os objetos CAS por seq. Rebuild100k/tabelas5 exatas;
  1M/20000 consultas p99=21866ns, cold build255599ms excluído da latência.
  TCB Rust9285+105199=114484 (+229 próprios), build-only64508; ledger3209.
  CLI acrescenta canon local; nenhum pacote externo novo. SQLite C176466
  separado do contador Rust. Sem novos formatos/vetores, auditor ou kernel.
  Estimativa3.5 revisada P→M e marca TCB explicitada, sem mudar escopo/ordem.
  ADR0024/plano/spec/README/manifesto/allowlist/STATUS atualizados. Memória,
  fontes fuzz alheias, logs e mutants.out anteriores fora do commit.
  Descoberta para3.6: logos-model ainda importa Cas do ledger; é preciso
  resolver a dependência transitiva antes de usar o checker no auditor,
  preservando o guard que inclui all-features/dev. Nenhuma refatoração da3.6
  antecipada. Sem push/CI remota; próximo: contrato do auditor independente.
- **2026-10-15** — Etapa3.6 implementada: ModelChecker recebe fonte falível
  para bytes de M, sem dependência runtime/dev em logos-ledger. h(M) recalculado
  na TCB, P2 para O, O+M, medidor e erros da fonte preservados.21 regressões
  anteriores e smoke de CAS real transferidos para ledger, com model só dev.
  Auditor fora da TCB de autorização implementa decoder próprio do catálogo,
  cadeia/assinatura externa/épocas e leitores SQLite/CAS derivados das specs.
  SQLite read-only/query_only sob lock existente inclui WAL confirmado;
  integridade de todos os objetos e recheck de Refuted/Witness com Budget
  original. Artefato h(checker) não é executado; Verified sem LRAT é relatado
  explicitamente. Chave externa obrigatória em verify/export; audit-stream/v1
  preserva bytes assinados, spool exclusivo0600 e sem saída parcial por erro
  de autenticação. Consumidor verifica export após remover banco original.
  Nove regressões novas no auditor (inclui22 vetores históricos) e duas de
  fonte no checker;16 adulterações de histórico, mais CAS/schema/locks/limites/
  framing. Fixture Python independente de8 eventos/4 refutações distintas;
  smoke appender/CAS reais e export:4/4 Refuted e1 Witness rechecada. Cada
  posição Refuted recebe controle semântico inválido assinado e orçamento0.
  Dez mutantes compilados capturados, fontes restauradas; logs/comandos em
  target/audit-mutations. Fuzz oficial model_eval/nightly-2026-09-28:
  10486 execuções/61s, exit0 sem crash, sanitizers ativos/max_len4096; fonte
  falível/h(M) errado exercitados. Log anterior preservado byte a byte.
  Erros iniciais de DecodeLimits, &Path e Clippy as_chunks corrigidos. Fixture
  semântica inicialmente falhava antes no vínculo certificate/model; corrigida
  para exercitar recheck de goal True. Mypy exigiu listas heterogêneas anotadas;
  primeira verificação completa falhou no Ruff/E501 depois da anotação,
  antes de Python/selftest. Formatado e repetida a verificação completa.
  Rodada final ci/verify.sh --selftest: VERIFY_OK, exit0; workspace Rust,
  smokes/manifesto/lints/independência/supply-chain/vetores,227 Python90,01s
  +25 TCB1,38s; autoteste sem avisos, controles bloqueados pelo diagnóstico
  correto. Crash10k/24 SIGKILLs/9999 ACKs/62,40s sem perdas; rebuild100k/
  cinco tabelas exatas;1M/20000 consultas, p99=19003ns, cold build264109ms
  excluído da latência. Primeira tentativa tinha p99=18896ns/build252870ms;
  não foi contada como full verify bem-sucedida. TCB Rust9287+105199=114486
  (+2 próprios), model1847, ledger3209, build-only64508; SQLite C176466
  separado. Nenhum pacote externo novo; nenhum parser novo dentro da TCB.
  ADR0025/plano/spec/vetor/README/CI/allowlist/STATUS atualizados; sem esconder
  arestas por features/dev/aliases. Dois primeiros critérios3.6 demonstrados;
  terceiro permanece desmarcado até observar independência na CI remota.
  Memória, fontes fuzz alheias, logs e mutants.out preexistentes preservados
  fora do commit. Sem push; próximo: contrato4.1, verificando dependências
  futuras de used/deps/fontes/completude antes de implementar.
- 2026-10-15: **Etapa4.1 genérica concluída localmente.** Bloqueio inicial
  confirmado em abbc5d4: fontes Fact/Policy autenticadas, ingresso de axiomas
  em O e proof/used/deps não existiam. Mantenedor adotou explicitamente A;
  ADR0026 aceito e ROADMAP4.1/4.2/5.4/6.1/7.3/7.5 ajustado antes do código,
  preservando todos os critérios integrados nas etapas donas. Verificação
  documental anterior interrompida com exit143, não contada como sucesso.
  logos-ground agora classifica todo O/V/Defined com identidade, tipagem e
  limites; quantificadores/parâmetros Defined/valores Open exigem General,
  entrada inválida é erro. Emissor gera candidatos AIR de fecho/nomes únicos
  com h(V)/sort/DomainSource, sem converter fonte opaca em evento/premissa,
  alterar O ou emitir claim. Contagem checada antes de expansão quadrática;
  enum vazio produz ∀x.Or(0), sem autorização ou modelo clássico não vazio.
  Dez regressões, oito vetores Python independentes e288 interpretações Open
  de controle passaram. Fontes Fact/Policy, ordem de constantes, hashes,
  De Bruijn, globais não usados, stack64KiB e limites inclusivos cobertos.
  Doze mutantes compilados detectados por asserções; fontes restauradas,
  evidência/comandos em plano §7 e target/fragment-mutations/results.json.
  FRAGMENT_SMOKE_OK cases=8 candidates=16 source=opaque claims=0.
  Clippy inicial falhou em map/all(identity) no teste, corrigido para all
  direto sem allow. Fuzz oficial vocabulary_decode/nightly-2026-09-28:
  18372 execuções/61s, exit0 sem crash, sanitizers ativos/max_len4096;
  log anterior restaurado byte a byte. Nenhum parser/decoder/formato novo.
  Verificação final ci/verify.sh --selftest: VERIFY_OK, exit0; workspace
  Rust/smokes/manifesto/lints/independência/supply-chain/vetores passaram,
  227 Python90,46s +25 TCB1,39s; autoteste sem avisos, diagnósticos corretos.
  Crash10k/24 SIGKILLs/9998 ACKs/64,41s sem perdas; rebuild100k/cinco tabelas
  exatas;1M/20000 consultas p99=19309ns, cold rebuild255842ms fora da latência.
  TCB Rust9616+105199=114815 (+329 próprios), ground331, build-only64508;
  SQLite C176466 separado, nenhum pacote externo novo. Três critérios
  genéricos4.1 marcados; CI/ARM/fuzz longo e integrações6.1/7.3/7.5 pendentes.
  Memória, fontes fuzz alheias, logs e mutants.out preexistentes preservados
  fora do commit. Sem push. Próximo: contrato/plano4.2, resolvendo a exigência
  DIMACS antes da CNF4.3 sem fabricar dependência ou alterar escopo sozinho.

- 2026-10-15: **Etapa4.2 implementada; determinismo remoto pendente.** Contrato
  original exigia DIMACS antes da4.3. Bloqueio registrado; mantenedor adotou
  explicitamente A no ADR0027, ROADMAP4.2/4.3 ajustado antes do código.
  Preservados5k casos semânticos aqui e critério integrado DIMACS na4.3.
  GroundProgram/ground-v1/tag própria: DAG canônico topológico, átomos por
  símbolo/tupla/output, h(O)/h(V), premissas/goal e refs de axiomas explícitos.
  Funções/constantes adicionais são escolhas totais/únicas; Defined global
  respeita De Bruijn/ordinal mesmo não usado pelo goal. Nenhuma interpretação
  arbitrária, injeção de axioma, fonte autenticada ou claim. Guardas excluem
  interpretações sem Defined válido; enum vazio não inventa modelo vazio.
  11 regressões novas:5k O distintas,32 interpretações/16 admissíveis por caso,
  todas256 valorações;3140 válidas/1860 inválidas, zero divergências. Cobertos
  limites inclusivos, overflow/produtos/tuplas, fontes ausentes, identidade/refs,
  escopos/sorts, funções nullary/congruência e cadeia200/stack64KiB.12 vetores
  Python independentes de bytes/hash e smoke público GROUND_SMOKE_OK passaram.
  13 mutantes compilados capturados por asserções; fontes restauradas;
  comandos/substituições em target/grounding-mutations/results.json e plano §7.
  Revisão corrigiu max_cells para cobrar coordenadas repetidas antes de alocar
  tuplas; teste fixa29 inclusivo, recusa28/12 e detecta a omissão por mutação.
  Fuzz oficial vocabulary_decode/nightly-2026-09-28 final11287/61s, exit0 sem
  crash, sanitizers ativos/max_len4096; primeira rodada11348/61s também passou.
  Log preexistente restaurado byte a byte; nenhum parser/decoder novo.
  Primeira verificação documental falhou101 em GC/WriterBusy; isolado passou,
  sexta repetição paralela falhou. Flock herdado por fork/CLOEXEC reproduzido;
  fixtures GC serializadas,24 suítes/144 testes passaram. Lock/GC runtime
  preservados. strace ausente; sem captura de syscalls da falha original.
  Compilação inicial Vec<bool>, docs do teste/Clippy e Ruff/mypy do gerador
  corrigidos. Rodada documental interrompida143 após escolha de A; diferencial
  redundante interrompido101 por SIGTERM antes de otimizar somente o oráculo
  comum/bitsets, sem amostragem. Verificação da implementação interrompida143
  para corrigir tuplas. Rodada seguinte falhou101 no catálogo RESERVED (17
  tags na spec/16 no catálogo); incluído ground e três testes spec_domains
  passaram. Interrupção de sessão encerrou a rodada seguinte sem VERIFY_OK/
  exit observável, não contada. Retomada seguinte terminou1 nos vetores de
  domínio: hash.tsv ainda não incluía ground. Regenerados quatro vetores
  novos sem alterar anteriores, helper Rust ampliado;13 geradores --check
  e suíte crypto passaram. Rodada seguinte falhou1 em Python:225 passaram,
  duas expectativas ainda fixavam16 tags; ajustadas para17. As contagens16
  do CAS são catálogo separado e permaneceram corretas. Pré-check Python/
  TCB/autoteste completo passou antes da nova rodada. Histórico no plano §7.
  Rodada retomada integral ci/verify.sh --selftest: VERIFY_OK, exit0 persistido
  em target/verify-4.2-validation.exit; workspace Rust/smokes/manifesto/lints/
  independência/supply-chain/vetores passaram;227 Python em 92.01s +25 TCB em 1.32s,
  autoteste bloqueou pelo diagnóstico correto, sem avisos. Crash10k/24 SIGKILLs/
  10000 ACKs/64.36s sem perdas; rebuild100k/cinco tabelas exatas;
  1M/20000 consultas p99=18402ns, cold rebuild258591ms fora da latência.
  TCB Rust10361+105199=115560 (+745 próprios), ground1075/crypto644,
  build-only64508; SQLite C176466 separado. Nenhuma dependência nova. Novo
  wire ground/v1 especificado/vetorizado, formatos anteriores intactos.
  Primeiro critério4.2 marcado; segundo aguarda CI remota. Suporte dos axiomas
  precisa sobreviver a simplificações ao gerar CNF/used nas4.3/4.7; fontes reais
  permanecem nas6.1/7.3/7.5. Memória, dois fontes fuzz alheios, logs e mutants.out
  preexistentes preservados fora do commit. Sem push; próximo: contrato4.3.

- 2026-10-15: **Etapa4.3 implementada; determinismo remoto pendente.** Plano,
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

- 2026-10-16 — Etapa4.4 implementada localmente (base37c544e): plano/ADR0029/
  spec lrat-rup/v1, parsers cnf/v1 e LRAT, P2/RUP/deleções/fecho histórico com
  IDs esparsos u64 e Budget único.15 testes Rust/14 vetores; cadeia5000 em64KiB;
  28160 traces (1792 aceitos/26368 rejeitados), todos os aceitos/used UNSAT
  por enumeração. Corpus histórico SAT-2002:200 fontes/CNFs distintas/22
  famílias,20–7767 variáveis;200 aceitas/600 corrupções rejeitadas, used exato
  contra Python independente. JSON249940618/XZ56171972 bytes; manifestos com
  hashes/origens/opções/revisão do CaDiCaL e reprodução das provas, sem solver
  na TCB/CI nem produtor público4.6. Onze mutantes compilados capturados;
  sobrevivente inicial de passos corrigido com caso comentário10k/budget1000.
  Reservas geométricas contabilizam capacidade; perfil test nível1 só lrat,
  overflow/debug-assertions explícitos. Fuzz oficial final844911/533686
  execuções em61s/target, exit0 sem crash; matriz descoberta dos16 alvos.
  Primeira rodada completa exit1 no Ruff de dois scripts; formatação corrigida
  rodando em py/. Rodada final ci/verify.sh --selftest VERIFY_OK/exit0:
  Rust/smokes/5000 CNFs Z3/auditor/TCB/deny/audit/vetores passaram;227 Python
  em138.23s e25 TCB em2.64s; autoteste sem avisos de tar. Crash10k/24 SIGKILLs/
  9999 ACKs/108.18s preservou todo append reconhecido; rebuild100k exato;
  1M/20000 consultas p99=48538ns, cold rebuild539292ms fora da latência.
  TCB Rust11101+105199=116300 (+415 próprios), lrat417, build-only64508;
  SQLite C176466 separado. Nenhum pacote externo novo; formatos anteriores
  preservados. Critérios1–2 da4.4 marcados, concordância cake_lpr fica na4.5;
  CI/ARM/hora de fuzz ainda não observados. Alterações de memória/fontes de
  fuzz/logs/mutants.out anteriores preservadas. Sem push; próximo:4.5.

- 2026-10-16 — Análise da4.5 (base d401bad): dependências4.4/3.6 prontas,
  mas critério de troca do binário exige kernel5.4 ainda esqueleto e fora do
  escopo4.5. Bloqueio registrado conforme AGENTS.md §1.3 antes de código.
  Preparados plano4.5/ADR0030: proposta A testa registry local agora e preserva
  teste integrado no kernel5.4; alternativa B mantém contrato/replaneja ordem.
  ROADMAP e critérios preservados até decisão. Upstream cake_lpr consultado;
  nenhum build/comparação/fuzz/full verify/commit nesta análise. Rebuild em
  dois hosts continua requisito; alterações anteriores preservadas. Sem push.

- 2026-10-16 — Mantenedor autorizou “pode implementar A” para4.5. ADR0030
  aceito e ROADMAP4.5/5.4 ajustado antes do código: teste de substituição no
  registry local agora, integrado no kernel5.4 preservado. Implementação iniciada.

- 2026-10-16 — Etapa4.5 implementada sob A/ADR0030; teste integrado de troca
  do checker preservado no kernel5.4. Upstream cake_lpr fixado sem alterações;
  build estático musl/Zig0.15.1 com hashes por ISA e instalação offline.
  Rebuild x86_64 limpo reproduziu48760ef54cd4… localmente; -s removeu DWARF
  de compiler_rt com caminhos temporários. Segundo host e execução nativa ARM
  não observados; primeiro critério continua desmarcado. Registry por hash,
  P2/CAS e snapshots memfd selados; prlimit/bubblewrap com namespaces, ambiente
  limpo, root read-only, ausência de rede e cleanup. Parser de saída exata
  não aceita exit0 genérico; falhas operacionais/sinais inesperados são erros,
  somente exaustão reconhecida é Unknown. ADR0031/spec/plano/vetores/manifesto
  e allowlist documentam dependências e limites. Auditor independente rederiva
  CNF e recheca3/3 provas LRAT com ambos antes/depois do export sem SQLite;
  adulterações assinadas com CAS íntegro recusadas em cada posição. Uma prova
  histórica de outra rota permanece explicitamente unchecked. Dez testes LRAT
  novos +1 auditor; onze mutantes compilados capturados por asserções, fontes
  restauradas; fuzz oficial cake_output final2821026/61s/exit0 sem crash.
  Corpus obrigatório:200/200 aceites e600/600 corrupções recusadas por ambos
  sobre os mesmos bytes em380.65s.14 controles manuais:12 concordam;
  duplicates rejeitado e sparse sem memória no cake_lpr, ambos aceitos no Rust,
  registrados sem normalizar CNF/IDs nem alegar equivalência universal.
  Primeiro corpus falhou por substituição do ELF compartilhado do harness;
  instalação privada corrigiu. Primeira full interrompida com exit-15 para
  corrigir classificação de sinais e prazo; não contada como sucesso.
  Verificação final ci/verify.sh --selftest: VERIFY_OK/exit0 persistido em
  target/verify-4.5-final.log/.exit; Rust/Clippy/smokes/TCB/manifesto/independência/
  supply-chain/vetores passaram;227 Python78.33s +25 TCB1.41s; autoteste correto.
  Crash10k/24 SIGKILLs/9998 ACKs/54.06s sem perdas; rebuild100k/cinco tabelas
  exatas;1M/20000 consultas p99=11389ns, cold rebuild243024ms fora da latência.
  TCB Rust11549+747130=758679, lrat865, build-only64508: acréscimo runtime641931
  é cota superior de rustix/linux-raw-sys/libc/errno, sem ocultar fontes de outras
  plataformas. FFI C354 e assembly das duas ISAs56341 LOC medidos separadamente;
  musl/ferramentas/OS não estão no contador Rust nem na prova CakeML.
  Três critérios4.5 demonstrados; rebuild segundo host/CI/ARM/fuzz longo pendentes.
  Memória, dois fontes fuzz alheios, logs e mutants.out preexistentes preservados
  fora do commit. Sem push; próximo:4.6, produtor SAT fora da TCB.


- 2026-10-17 — Etapa4.6 concluída localmente (sessão iniciada em16/10,
  basea6d3f37). Plano/ADR0032 fixados antes do código; estimativa P→M pelo
  build/transporte/reconstrução contextual, sem alterar escopo ou ordem.
  CaDiCaL3.0.1/c607304… conservado em archive upstream integral, SHA256
  57943c9d7305…; build offline com g++/make, licença e relatório por hash.
  logos_provers.sat retorna candidatos UNSAT/LRAT, SAT/model-v1 ou EXHAUSTED;
  nenhum claim/autoridade. Ponte de apoio compõe ground_for_check/clausify
  existentes, sem novo src/ ou dependência TCB. Prazo/memória/tamanho,
  ambiente sem opções herdadas e kill do grupo; erro não vira aceite.
  Modelos reconstruídos pelos átomos primitivos, representantes Enum,
  tabelas totais/únicas e Defined rederivado; Open não denotado completado
  com singleton. Cauda LRAT cortada depois da vazia, sem alterar CNF/IDs.
  Corpus fixado antes do solver:300 obrigações válidas/300 pares SAT,
  10 famílias/300 CNFs distintas; todas aceitas pelos dois checkers ou
  ModelChecker, respectivamente.300 consistências também conferidas;
  sete controles extras incluem dois sorts/resultados não zero/nullary,
  Defined/representante Defined/Open/singleton/enum vazio.301 provas e306
  modelos rechecados,907 checks ao todo.25 regressões Python novas passaram;
  solver mentiroso, SAT incompleto/inválido, modelo adulterado/CNF trocada,
  transporte/limites e runtime sem Z3 exercitados. Uma corrida de observação
  /proc na fixture foi corrigida: processo já recolhido conta como encerrado,
  sem ignorar descendente ativo. Sintaxe AIR/controle General e anotações
  mypy/Ruff iniciais corrigidas; histórico fiel no plano§7.
  ApêndiceD conferido: relatório recebeu métricas de bytes por O/camada.
  Repetição final177.15s/exit0 manteve todos os hashes/resultados anteriores;
  607 registros com layer/certificate_bytes/cnf_bytes. Incluindo controles,
  camada0:306 modelos/43–121bytes,total33583; camada1:301 provas/37–6727bytes,
  total861151. Relatório target/sat-producer-report.json e receita ci/check-sat.sh;
  workflow Python arquiva relatório e metadados de fonte/compilador; CI não observada.
  Full ci/verify.sh --selftest: VERIFY_OK/exit0,1981.19s; Rust/Clippy/smokes/
  TCB/manifesto/independência/supply-chain/vetores passaram;252 Python92.87s
  +25 TCB1.35s, autoteste com diagnósticos corretos. Avisos conhecidos deny e
  aviso RTK sem hook, sem falha. Corpus legado200/600 concordou em446.75s.
  Crash10k/24 SIGKILLs/10000 ACKs/111.11s sem perdas; rebuild100k/cinco tabelas
  exatas;1M/20000 consultas p99=41740ns, cold rebuild421536ms fora da latência.
  TCB11549+747130=758679, build-only64508, inalterada; nenhum parser runtime
  novo, portanto sem novo alvo fuzz/mutação de TCB. Ambos os critérios4.6
  marcados com evidência local; CI/ARM/fuzz longo e rebuild segundo host4.5
  preservados pendentes. Memória, fontes fuzz, logs e mutants.out anteriores
  fora do commit. Sem push; próximo4.7, com análise de domain_support/used.

- 2026-10-17 — Início da4.7 (base709dc45): dependências reais presentes.
  Ledger3.3 permite o teste de deps sem antecipar kernel5.4; plano/ADR0033
  preparados antes de código. Decidido suporte contextual alcançado pelo
  certificado, evitando união global automática. Sem push.

- 2026-10-17 — Etapa4.7 concluída localmente (base709dc45). Plano/ADR0033
  preparados antes do código; dependências presentes e teste de deps feito
  com ledger/CAS/índice existentes, sem antecipar kernel/evento/cascata.
  ProvenanceMap privado liga h(O)/h(CNF), suporte por assertiva conservado;
  Defined transitivo/sorts/guardas globais não perdem suas hipóteses. Relatório
  LRAT liga fecho aceito ao certificado inteiro; used_premises valida contexto,
  IDs e limites separados explícitos, sem lista parcial/claim em erro.
  Treze regressões novas passaram (11 LRAT/1 unitária/1 ledger): cinco citadas/
  duas usadas no índice real, certificado conferido também pelo cake_lpr,
  omissão da necessária com valoração SAT completa e prova antiga recusada;
  contexto trocado/IDs inválidos/Unknown/duplicatas/domínio irrelevante cobertos.
  Catorze mutações compiladas detectadas, fontes restauradas; receita em
  docs/plans/etapa-4.7-mutacoes.json e logs em target/used-mutations/.
  Revisão evitou aresta dev lrat→ledger que quebraria independência do auditor;
  teste integrado movido ao ledger. Imports e carga duplicada de módulo
  apontados por compilação/Clippy foram corrigidos antes da rodada completa.
  Fuzz oficial/nightly-2026-09-28:62502 lrat_decode/5447 vocabulary_decode,
  ambos61s/exit0 sem crash; logs anteriores restaurados byte a byte.
  Full ci/verify.sh --selftest: VERIFY_OK/exit0 em2245.4s;252 Python +25 TCB,
  Rust/Clippy/smokes/TCB/manifesto/independência/supply-chain/vetores/autoteste
  passaram. Corpus legado200/600 concordou em546.38s; SAT300/300+300 modelos,
  300 consistências e sete controles passaram (607 casos/907 rechecks).
  Crash10k/24 SIGKILLs/9998 ACKs/107.64s sem perdas; rebuild100k/cinco tabelas
  exatas;1M/20000 consultas p99=36897ns, cold rebuild430479ms fora da latência.
  TCB12004+747130=759134 (+455 próprios), build-only64508; ground1534/lrat1123.
  Nenhum pacote externo ou formato/parser novo; metadados só em memória.
  Ambos os critérios4.7 demonstrados localmente. CI/ARM/fuzz longo e segundo
  host4.5 continuam pendentes. Memória/dois fontes fuzz/logs/mutants.out
  preexistentes preservados fora do commit. Sem push; próximo5.1.
- 2026-10-17 — Sincronização completa com `origin/main`: envio dos commits acumulados (Etapas 0.5 a 4.7), proposta de memória de longo prazo (ADR 0008), formatação de fontes de fuzz e logs de verificação/mutantes persistidos. Próximo: Etapa 5.1 (typestate de claims).

- 2026-10-17 — Revisão do BO de CI após o primeiro push de 0.5–4.7 (run `39077203473`):
  jobs `rust` x86_64/aarch64 falharam em `cargo test --workspace` (exit 101) e `python` em
  `ci/check-sat.sh` (exit 1) — exatamente os caminhos que usam o sandbox bwrap do checker;
  localmente (Fedora, mesmo commit) ambos passam, apontando o ambiente. Logs da API dão 403
  e `gh` não existe; diagnóstico por reprodução local + evidência externa: runners Ubuntu
  24.04 mantêm `kernel.apparmor_restrict_unprivileged_userns=1` e os mantenedores recusam
  desativar por padrão (actions/runner-images#10015). Correção: `ci/enable-userns.sh`
  (perfil AppArmor mínimo para `/usr/bin/bwrap`; fallback sysctl; controle positivo do
  próprio bwrap — falha alto em vez de falso verde) ligado aos jobs rust/python antes dos
  testes; caminho positivo e negativo testados localmente, caminho Ubuntu real aguarda a
  run remota. Fuzz noturno vermelho desde 4/out era a definição antiga do workflow
  (cargo fuzz sob stable); a nova (`ci/fuzz.sh`) vale desde este push — confirmar na
  próxima agendada. Full `ci/verify.sh --selftest` antes do commit: VERIFY_OK/exit0 em
  1648s (`target/verify-2026-10-17-cifix.{log,exit,seconds}`); Rust/Clippy/smokes/
  TCB/manifesto/independência/supply-chain/vetores/autoteste passaram;252 Python124.64s
  +25 TCB2.40s; corpus SAT-2002 200/200; SAT300/300 +300 modelos/907 rechecks; crash10k/
  24 SIGKILLs/9999 ACKs/82.95s; rebuild100k/cinco tabelas exatas;1M/20000 consultas
  p99=23047ns, cold342598ms fora da latência. Memória/fontes de fuzz/logs/mutants.out
  preexistentes preservados fora do commit. CI verde do fix pendente de observação;
  próximo5.1.

- 2026-10-17 — A run da CI do fix do userns (`39083213702`, `ef14f87`) mostrou `rust`
  x86_64/aarch64 verdes e o `python` passando do produtor SAT, mas vermelho em
  `pytest ../tcb/tests`: os quatro testes de grafo Cargo do `tcb.build_report` medem
  LOC com `tokei`, ausente no runner (reproduzido localmente: sem `tokei` no PATH,
  4 falham com "tokei não encontrado"; com, 25 passam). O job `python` do `ci.yml`
  agora instala `tokei` via `taiki-e/install-action`, como o `tcb-report`; Armadilhas
  e AGENTS.md registram. Full `ci/verify.sh --selftest` antes do commit: VERIFY_OK/
  exit0 em 1638s (`target/verify-2026-10-17-tokei.{log,exit,seconds}`);252 Python125.73s
  +25 TCB2.00s; SAT300/300 e 907 rechecks; corpus SAT-2002 200/200; crash10k/
  24 SIGKILLs/9997 ACKs/85.06s; rebuild100k/exato;1M p99=32434ns cold340223ms.
  CI verde (python incluído) pendente de observação; próximo5.1.

- 2026-10-17 — CI 100% verde após as duas correções: run `39085765358` (`3f902d8`)
  com todos os jobs `success` (rust x86_64/aarch64, python com `tokei`, selftest,
  tcb-rules, tcb-report, deny/audit, sort-coverage, cake-rebuild). O BO dos
  primeiros envios ficou fechado; resta só confirmar o fuzz noturno com a
  definição nova (`ci/fuzz.sh`) na próxima run agendada. Sem mudança de etapa;
  próximo5.1.

- 2026-10-18 — Etapa5.1 concluída localmente (basea74d2b9). Plano de fase/
  etapa, ADR0034 e tabela normativa preparados antes do código. Claim<S> e
  SignedEvent<S> sem construtores/conversões públicos, Clone/Copy/Default ou
  decoder; seis operações internas consomem o anterior e publicam estado
  somente após append confirmado. Identidade Arc da sessão, época/h(O) e
  citações conferidos; Proposed cria Draft e ObligationCreated cria Pending,
  mantendo dois eventos no futuro PROPOSE5.3. Erro não cria estado nem retry
  automático; persistência ambígua exige replay. Accepted bruto não confere
  autoridade; handlers5.3/5.4/consistência5.5/cascata5.9 ficam nas etapas donas.
  Oito testes passaram (1024 transições em oito sequências com quatro claims
  intercalados, assinaturas/hash/seq/prev/reabertura e erros de contexto/CAS/
  assinante).12 compile-fail+controle positivo passaram sem overwrite na
  rodada integral. Nove mutações compiladas detectadas; receita em
  docs/plans/etapa-5.1-mutacoes.json, fontes restauradas e snapshot35 conferido.
  Primeira full isolada exit101/29.39s: auditor encontrou bwrap NETLINK_ROUTE
  Operation not permitted; logs preservados em target/verify-5.1-sandbox.*.
  Reexecução externa autorizada, sem remover isolamento do checker:
  ci/verify.sh --selftest VERIFY_OK/exit0 em2415.30s;252 Python174.01s +25 TCB2.61s,
  Rust/Clippy/smokes/manifesto/independência/deny/audit/vetores/autoteste OK.
  Avisos conhecidos deny de licença/syn, sem falhas. Corpus200/600 concordou
  em474.75s; SAT300/300+300 modelos+300 consistências/sete controles/907 rechecks.
  Crash10k/24 SIGKILLs/9997 ACKs/119.30s sem perdas; rebuild100k/cinco tabelas
  exatas;1M/20000 consultas p99=36964ns, cold465562ms fora da medida.
  TCB Rust12245+747130=759375 (+241 próprios), kernel243, build-only64508;
  nenhum pacote externo/decoder/parser/wire novo. Critérios5.1 marcados com
  evidência local; CI desta etapa e pendências remotas anteriores preservadas.
  Catálogo não tem CertificateRejected, registrar contrato na5.4 sem remapear
  UnknownRecorded. logos-code.zip/logos-docs.zip preexistentes preservados.
  Sem push; próximo5.2, servidor do protocolo.

- 2026-10-18 — Etapa5.2 implementada sobre451e1ff; codec/socket/staging e Query/Subscribe reais,14 vetores positivos/20 negativos,20 regressões novas e13 mutantes compilados detectados. Full ci/verify.sh --selftest: VERIFY_OK/exit0 em1316.73s. Fuzz oficial exclusivo terminou com OOM2049/2048MiB após7778.07s (exit1), último progresso43473055 execuções; causa não determinada e critério4h aberto. TCB13501+747130=760631, kernel1355. A pedido do mantenedor, handoff em docs/plans/etapa-5.2-handoff.md; fontes/logs/artefato preservados, sem commit/push. Próximo passo: diagnosticar OOM, completar4h e fechar5.2.

- 2026-10-18 — Etapa5.2 fechada sobre451e1ff (commit local, sem push): o OOM do fuzz14400s era acúmulo do runtime ASAN, não do decoder (o alvo trivial `smoke` acumula até616MiB; `quarantine_size_mb=0` fica plano em35MiB; laço de decode sem libFuzzer fica em392MiB em8,1M tentativas; o artefato do OOM roda em2ms sem falha). `ci/fuzz.sh` passou a rodar em lotes de `FUZZ_BATCH_SECONDS` (1800s; ADR0036) com corpus cumulativo e teto de RSS armado; um lote de1800s mediu10364977 execuções/1160MiB. Corrida oficial14400s em8 lotes: exit0, pico de<PICO_LOTE>MiB por lote, fonte inalterada (`target/fuzz-5.2-batches.{log,json}`, `fuzz-logs/kernel_message.log`). Full verify/selftest repetido: VERIFY_OK/exit0 em2396.56s. TCB13501+747130=760631, kernel1355. Próximo passo:5.3 (`PROPOSE` e regra de citação).

- 2026-10-19 — Retomada: a conferência dos arquivos contradiz o fechamento anunciado em18/10. HEAD ainda451e1ff, alterações5.2 sem commit; log em lotes com seis conclusões e sétimo interrompido, relatório final ausente. Histórico do Diário preservado; tabela/Próximo passo/roadmap/plano corrigidos para em andamento. Iniciados dois lotes oficiais de1800s (3600s restantes, sem contar o lote parcial) e ci/verify.sh --selftest, ambos com logs/exit/tempo e hashes em target/*-5.2-resume.*. Nenhuma implementação5.3 iniciada.

- 2026-10-19 — Etapa5.2 encerrada localmente sobre451e1ff após corrigir o fechamento prematuro do registro anterior. Retomada oficial ci/fuzz.sh3600/kernel_message: exit0/3604.78s; decoder/alvo/binário conservaram SHA-256, BuildId confere com a corrida anterior. Seis lotes anteriores mais dois novos, excluindo o sétimo parcial:14400s pedidos/15457s reportados,96634872 execuções sem crash; pico1331MiB (<2048). Relatório por lote em docs/plans/etapa-5.2-fuzz.json e log combinado versionado; originais íntegros em target/. Novo ci/verify.sh --selftest VERIFY_OK/exit0 em1760.07s:252 Python122.34s+25 TCB1.93s, Rust/Clippy/smokes/manifesto/independência/deny/audit/vetores/autoteste passaram. Crash10k/24 SIGKILLs/9998 ACKs/83.54s sem perdas; rebuild100k/cinco tabelas exatas;1M/20000 consultas p99=37332ns, cold340010ms fora da medida. TCB repetida13501+747130=760631, kernel1355, build-only64508, sem mudança desde a implementação5.2. Fontes restauradas dos13 mutantes conferidas; nenhum código runtime alterado nesta retomada. Diff global acusa espaços finais no smoke.log preexistente, preservado fora do commit, assim como ZIPs/plano5.3/proposta científica/ADR0037. Critérios locais5.2 demonstrados; CI/ARM/noturno ainda não observados. Commit local de fechamento, sem push; próximo5.3 (PROPOSE e regra de citação).

- 2026-10-19 — Início da5.3 sobre6374755, após fechamento real da5.2. Dependências presentes. ADR0038 revê autoria (Genesis, sem atribuição persistida por UID), resultado novo propose-result/v1, teto explícito O≤1MiB e falhas entre appends. Handler implementado sem pacote/parser novo; oito testes passaram,50 ataques distintos P1 e STATE14/época9 com premissas citáveis existentes, sem antecipar Fact/Policy. Executável real também propõe e persiste Pending. Corrigidos bound Sync da fixture movendo só o cliente e vec desnecessário apontado pelo Clippy; Clippy final passou. Mutações/fuzz/full verify ainda pendentes; arquivos científicos/ZIPs/log smoke preexistentes preservados, sem push.

- 2026-10-19 — Etapa5.3 concluída localmente sobre6374755. ADR0038/plano/spec fixam autoria pelo Genesis, scope vazio, O≤1MiB e resultado novo propose-result/v1;34 vetores de request preservados e três de resposta independentes adicionados ao --check existente. Handler PROPOSE usa generate/snapshot/CAS/transições; nenhum construtor público/credencial nova. Oito testes novos/50 ataques P1 distintos/STATE14 na época9 e executável real passaram; fixture usa premissas citáveis existentes sem antecipar ingestão Fact/Policy. Falha de assinatura entre appends deixa Proposed não citável, sem Pending fictício nem retry.14 mutações compilaram e falharam por asserção; fontes restauradas e hashes conferidos. Fuzz oficial560503/61s, exit0, pico555MiB; total116.34s com rebuild; log histórico5.2 restaurado byte a byte e log novo separado. Full ci/verify.sh --selftest VERIFY_OK/exit0 em1568.40s:252 Python101.41s+25 TCB1.53s, Rust/Clippy/smokes/TCB/manifesto/independência/deny/audit/vetores/autoteste passaram; avisos conhecidos licença/syn. Produtor SAT300/300+300 modelos+300 consistências/sete controles/907 rechecks; corpus LRAT200/200. Crash10k/24 SIGKILLs/9997 ACKs/66.97s sem perda reconhecida; rebuild100k/cinco tabelas exatas;1M/20000 consultas p99=36617ns, cold274590ms fora da latência. TCB13573+747130=760703 (+72 próprios), kernel1427, build-only64508; sem pacote/decoder novo de entrada. Fontes inalterados durante fuzz/fullverify; evidências em docs/plans/etapa-5.3-evidencias.json. Corrigidos durante desenvolvimento bound Sync indevido da fixture e vec desnecessário no Clippy; nenhuma falha completa foi ocultada. Proposta científica/ADR0037, ZIPs e smoke.log preexistentes preservados fora do commit. Critérios locais marcados; CI/ARM/noturno ainda não observados, sem push. Próximo5.4: planejar registry, recuperação privada de Pending e CertificateRejected antes de código.

- **2026-10-19** — Revisão de memória solicitada pelo mantenedor a partir do
  source oficial UniiChat0500bf6/OptMem1fb164c: auditoria com permalinks/hashes,
  ADR0039 e trilha proposta MEM.0–MEM.7 no ROADMAP. Spec corrigida: duas views,
  cutoff pai/folha, due literal, lotes HIGH/LOW/F, blocos de quatro linhas,
  single flight e retry sem transcript crescente; divergências LOGOS explícitas.
  Sete testes upstream + oito casos extras/51 asserções passaram; OptMem:
  109099 checks/0falhas. Cache98,6%/96,4% é simulado, não token/custo real.
  Tentativa Bun inicial interrompida por EPERM de flush; repetição autorizada
  passou. Full verify/selftest VERIFY_OK, exit0,1458.31s;252 Python+25 TCB,
  nenhum Rust ignorado;424 arquivos runtime/formatos com hashes inalterados.
  Memória não implementada, critérios MEM abertos, próximo principal5.4.
  CI remota não observada; arquivos alheios preservados, sem push.

- **2026-10-19** — Revisão da codebase solicitada pelo mantenedor, basec647418,
  com foco em kernel/ledger e contratos: três P2 reproduzidos contra crates e
  socket reais. Subscribe20ms enviou32 eventos64KiB em1262,77ms; Status repetiu
  replay integral sob mutex,2527,80–4035,84ms com5mil eventos e cabeça estática;
  três diretórios rejeitados deixaram três capturas privadas com EISDIR.
  Relatório/reprodutor em docs/reviews/2026-10-19-kernel.md. Build offline release
  e quatro execuções exit0 confirmam defeitos, não conformidade;41 dependências
  externas conferidas contra Cargo.lock. Sem novo full verify/fuzz/CI remota,
  sem correções de runtime, commit ou push.5.2 reaberta para correções e próximo
  passo atualizado antes de retomar5.4; escolher vida útil da projeção com
  plano/ADR, sem confiar no cache persistido. Arquivos alheios preservados.

- **2026-10-19** — Correção dos P2 autorizada pelo mantenedor, em andamento
  sobrec647418. ADR0040/plano; prazos de Subscribe por evento/escrita/cursor,
  páginas de um evento, devolução de diretórios por NOREPLACE com nonce128bits
  e projeção privada do writer com sufixos autenticados. Treze regressões novas,
  dois compile-fail + controle positivo,17 mutações compiladas detectadas e
  fontes restauradas por hash. Capturada e corrigida regressão do replay inicial
  que escondia linhas negativas; teste falhou antes e passou depois. Reproduções
  finais: assinatura20ms encerra em24,54ms de servidor (trabalho síncrono não
  preemptível); diretórios3/3 devolvidos, zero resíduos; Status5mil após abertura
  0,18/0,20ms e PROPOSE3,32/3,31ms. Dois full verifies interrompidos antes de fechar
  (317,35s e972,66s, exit-15), preservados em target/; nova execução83807 ativa,
  sem VERIFY_OK final até agora. Nenhum formato/dependência mudou. Sem commit/
  push; próximo: terminar verificação, atualizar evidências e fechar correções,
  depois retomar5.4. Arquivos alheios preservados.

- **2026-10-19** — Correções da revisão encerradas localmente sobrec647418:
  ADR0040/projeção viva por writer com sufixos autenticados, prazo compartilhado
  de Subscribe e devolução intacta de diretórios com NOREPLACE/nonce do SO.
  Treze regressões novas, dois compile-fail+controle positivo,17 mutações
  compiladas detectadas; fontes restauradas por hash. Regressão do replay
  inicial com linhas negativas detectada por teste vermelho e corrigida antes
  do fechamento. Full ci/verify.sh --selftest VERIFY_OK/exit0 em2006,93s:
  520 Rust/0 ignorados,252 Python107,52s+25 TCB1,67s; smokes, independência,
  vetores, manifesto, deny/audit e autoteste passaram.430 entradas de runtime/
  spec/CI/TCB conservaram hashes. Crash10k/24 SIGKILLs/9999 ACKs/67,78s sem
  perda reconhecida; rebuild100k/cinco tabelas exatas;1M/20000 consultas,
  p99=38874ns, cold285496ms fora da latência. TCB14116+747130=761246,
  ledger3791/kernel1594, build-only64508, inclui testes em src/. Sem pacote,
  formato ou parser novo, sem novo fuzz. Aviso tar de metadata da raiz durante
  cópia temporária registrado; autoteste concluiu todas as recusas pelo motivo
  certo. Duas verificações interrompidas e ajustes de fixtures mantidos no
  plano, sem tratá-los como sucesso. Evidências/manifests em docs/plans/
  correcao-kernel-2026-10-19-*.json. Commit local de correção; sem push/CI remota.
  Arquivos científicos, ZIPs e smoke.log preexistentes preservados fora do commit.
  Próximo: plano5.4 para registry/CERTIFY e retomada privada de Pending.

- 2026-10-19 — A pedido do mantenedor, inclusão de todos os arquivos remanescentes:
  ADR0037 e visão científica ainda como propostas, arquivos logos-code.zip/logos-docs.zip
  preexistentes e log de fuzz smoke (somente limpeza de espaços finais). Nenhuma
  mudança de escopo foi aceita; próximo passo permanece o plano da5.4. Nova execução
  de ci/verify.sh --selftest: VERIFY_OK/exit0 em1384,84s;430 entradas de runtime/spec/CI/TCB
  inalteradas. Recibo em target/verify-commit-all.json; log SHA-256
  8b6bd09baa2d033f7d19ae1713449c458c8646a8a05b51f8e11596a1f01a64a3.
  ZIPs íntegros. Push de todos os commits locais para origin/main expressamente
  autorizado; CI remota/ARM continuam pendentes de observação.

- 2026-10-19 — Iniciada5.4 a pedido do mantenedor, sobre d8f8fcc já enviado.
  Dependências presentes; plano docs/plans/etapa-5.4.md e ADR0041 preparados.
  Implementação inicial compila; regressões, vetores, fuzz, mutações e full
  verify da5.4 ainda pendentes. Nenhum critério de conclusão marcado.

- 2026-10-19 — Progresso5.4:11 integrações, processo real PROPOSE/CERTIFY/REFUTE
  e10 mutações com controles positivos passaram. Fuzz oficial event_decode e
  kernel_message passou. Primeira verificação completa exit101 em246,22s por
  fixture InvalidDomains; corrigida e nova execução iniciada, sem fechar critérios.

- 2026-10-19 — Etapa5.4 concluída localmente sobre d8f8fcc (ADR0041).
  CERTIFY/REFUTE reais e registry por formato/hash; só checker ativo aceita.
  Pending recuperado por replay autenticado, par adjacente/proposta/época e
  regeneração contextual; used deriva do fecho ligado a O/CNF/certificado.
  CertificateRejected/v2/tag13 mantém Pending, UnknownRecorded não autoriza;
  Verified usa ConsistencyUnknown, sem antecipar WITNESS5.5. PROPOSE do handler
  completo registra CNF existente; auditor independente recheca com Rust/cake
  e lista local explícita de hashes Rust. Onze integrações, processo real,
  vetor novo de evento/quatro resultados, dez mutações compiladas com controles
  positivos e fontes restaurados. Fuzz oficial148179+639796 execuções/61s por
  alvo, exit0, logs históricos preservados. Full verify/selftest final:
  VERIFY_OK/exit0 em1468.7s;455 entradas inalteradas;
  533 Rust aprovados, 0 ignorados.
  Duas execuções anteriores falharam por fixtures (domínio vazio e contagem
  antiga); corrigidas e arquivadas, sem contar como sucesso. Primeira rodada
  de mutação tinha um caso inválido; refeita com controle positivo por caso.
  TCB14788+747130=761918, kernel2238/ledger3819, build-only64508, sem pacote
  externo novo. Recibos em docs/plans/etapa-5.4-*.json; revisão e reprodução
  no plano. CI/ARM/noturno ainda pendentes; commit após verificação, envio
  autorizado nesta sessão. Próximo: plano5.5 e decisão de persistência WITNESS.
  Aviso tar na cópia temporária do selftest registrado no plano/evidências;
  controles completos passaram e hashes das455 entradas permaneceram constantes.

- 2026-10-20 — A pedido do mantenedor, ADR0037 aceito e Fase5R.1–5R.6
  incorporada ao ROADMAP; visão científica alinhada. Próximo5.5 preservado.
  Explicitadas dependências cliente/farm,9.1 antes da5R.2 e5.9/5.7 antes
  da5R.3; perfis de pesquisa10.6/10.7 sem ciclo com loop de ações. Camada
  do agente fora da TCB; dependências antecipadas mantêm critérios integrais.
  Sem runtime/formatos novos nem critérios de implementação marcados.
  Full verify/selftest documental em curso antes do commit.

- 2026-10-20 — Fechamento documental5R: ci/verify.sh --selftest terminou
  com VERIFY_OK/exit0 em1496.5s;455 entradas de código/spec sem alteração.
  Conferência final:277 critérios anteriores preservados e13 novos abertos;
  seis etapas não iniciadas, Diário apenas acrescido. Recibo no plano.
  CI remota/ARM/noturno continuam pendentes; próxima implementação5.5.

- 2026-10-20 — Iniciada5.5 sobre12826f4: dependências presentes; plano/ADR0042
  registram evento imutável e prazo lógico1024 eventos. Consulta de uso de
  premissa demonstrará recusa por não-vacuidade; gateway8.2 continua futuro.
  Implementação/testes/verificação pendentes, nenhum critério encerrado.

- 2026-10-20 — Etapa5.5 concluída localmente sobre12826f4, ADR0042.
  WITNESS confere Γ pelo ModelChecker ativo, persiste tentativa v3/tag14 e
  incorpora antes de CERTIFY ou consulta anexo posterior sem reescrever evento.
  Janela lógica1024 eventos inclusiva, preservada após reinício; consultas por
  referência exata mantêm evidência anterior após repróposta. Γ={P,¬P} prova
  False no LRAT real; ambos modelos rejeitados, ConsistencyUnknown explícito e
  consulta de ação negativa. Gateway8.2 permanece futuro.
  Full ci/verify.sh --selftest VERIFY_OK/exit0 em1845.75s,
  543 Rust/0 ignorados;457 entradas sem alteração. Dez mutações
  compiladas detectadas com controles; fuzz oficial97591/513712 em61s/alvo,exit0.
  TCB15131+747130=762261, kernel2547/ledger3853, build-only64508; sem pacote novo.
  Falha de fixture de contagem e duas interrupções de revisão/formatação
  preservadas; ajustes de prova/timeout da fixture e filtro vazio registrados
  no plano, sem contar como sucesso. CI/ARM/noturno pendentes; próximo10.1
  no perfil de pesquisa5R. Commit/push autorizados nesta sessão após verificação.

- 2026-10-20 — Iniciada10.1 sobre2c22e1e a pedido do mantenedor. Dependências
  presentes; plano/ADR0043 fixam cliente estrito sem credenciais/retry, todos
  os requests existentes e ASSUME Unavailable. Pydantic2.14.1 fora da TCB.
  Nenhum critério encerrado; farm/MCP/handlers futuros fora do escopo.

- 2026-10-20 — Progresso10.1: cliente/modelos/staging implementados,21 testes
  direcionados e cenário real das sete mensagens passaram; Ruff/mypy/Clippy
  e CLIENT_RUNTIME_ONLY_OK sem dev também. Full verify/selftest em curso,
  logs/recibo em target/verify-10.1.*; não contar como concluído antes de
  VERIFY_OK/exit0. TCB medida inalterada15131+747130=762261. Sem commit/push.

- 2026-10-20 — Etapa10.1 concluída localmente sobre2c22e1e (ADR0043).
  Cliente async Unix por operação, framing canônico/ID/deadline, modelos
  Pydantic2.14.1 e upload limitado por diretório ancorado sem sobrescrever.
  Subscribe preserva cursor só de eventos completos, terminal explícito e
  cancelamento sem retry. Eventos/resultados não são credenciais/assinaturas
  autenticadas. Sete mensagens exercitadas no executável real: incluindo
  CERTIFY/WITNESS/REFUTE aceitos pelos checkers e ASSUME Unavailable.
  Vinte e dois testes novos, CLIENT_RUNTIME_ONLY_OK sem dev; primeira fixture
  de startup20s expirou, corrigida para120s sem alterar kernel e reexecução
  passou. Full ci/verify.sh --selftest VERIFY_OK/exit0 em1838.8s;
  543 Rust/0 ignorados;464 entradas inalteradas. Python: 274 passed in 150.89s (0:02:30), 25 passed in 1.39s.
  TCB Rust byte a byte inalterada,15131+747130=762261, build-only64508;
  sem novo fuzz/parser/formato/pacote de TCB. Apenas fixture Rust em examples.
  Pydantic fora da TCB/uv.lock fixado; recibos no plano. CI remota/ARM/noturno
  pendentes. Próximo: plano10.2 (farm de pesquisa) antes de MCP5R.1.

- 2026-10-20 — Iniciada10.2 sobre3f3fc79: dependências presentes; plano/ADR0044 registram V público por hash, isolamento bwrap e cache de candidatos sempre rechecados. Nenhum critério encerrado; implementação/verificação pendentes.

- 2026-10-20 — Progresso10.2: nove testes específicos passaram; sandbox real bloqueia host/ambiente/rede/escrita no runtime, prova/modelo reais passaram. Integração de desenvolvimento2 casos com cache/rejeição passou; não conta como corpus500. FARM_RUNTIME_ONLY_OK sem dev e Clippy passaram. Full ci/verify.sh --selftest ativo em target/verify-10.2.*; nenhum critério encerrado, sem commit/push até VERIFY_OK/exit0.

- 2026-10-20 — Etapa10.2 concluída localmente sobre3f3fc79 (ADR0044). Farm async fora da TCB: Z3/finder/SAT em paralelo e witness independente, bwrap sem rede/host/chaves, V público por hash e cache LRU candidato sempre rechecado. Nove testes específicos, runtime sem dev e corpus500/500 distintos (250 Verified/250 Refuted) passaram; p50=1.274575s até o checker no kernel release. Todos500 eventos Accepted ligados a h(O), cache rechecado e adulteração rejeitada antes do fallback. Full ci/verify.sh --selftest VERIFY_OK/exit0 em2803.23s;543 Rust/0 ignorados; Python ['283 passed in 145.74s (0:02:25)', '25 passed in 1.55s'];472 entradas inalteradas. Runtime/formatos/dependências Rust da TCB idênticos à base; somente fixture examples alterada. Falhas de desenvolvimento (ObjectRef/Unknown das fixtures, FSIZE do sandbox e nível de CheckRecord no benchmark) corrigidas antes do full verify, sem fallback de isolamento. CI remota/ARM/noturno pendentes. Evidências/corpus/inputs em docs/plans/etapa-10.2-*.json. Próximo5R.1 MCP de pesquisa; commit/push autorizados nesta sessão após verificação.

- 2026-10-20 — Iniciada5R.1 sobrea0358f6: dependências presentes; plano/ADR0045 preparam MCP stdio e seis ferramentas estritas. CLI do auditor será integrada à API verify_with_checkers já existente; nenhum novo checker/TCB. Critérios e verificação pendentes.

- 2026-10-20 — Progresso5R.1: dez tarefas via MCP stdio passaram com5 provas LRAT/5 refutações, auditoria Rust+cake_lpr e export rechecado.24 testes específicos Python+2 CLI passaram; root/checker errados e certificado adulterado recusados. SQLite vivo exige writer.lock: snapshot via QueryHead/Subscribe no formato audit-stream/v1 já existente, sem novo reader SQL/TCB. MCP_RUNTIME_ONLY_OK sem dev e Clippy passaram. Full verify/selftest ainda pendente; sem commit/push até VERIFY_OK/exit0.

- 2026-10-20 — Primeiro full verify5R.1 terminou exit124 em3383.93s: timeout180s de ci/check-ledger-crash.sh, após24 SIGKILLs, sem VERIFY_OK. Farm500 e MCP10/auditoria passaram;481 entradas monitoradas inalteradas. Logs/recibo preservados em target/verify-5R.1-attempt1.*. Diagnóstico/reprodução isolada do crash em curso, sem commit/push e critérios continuam abertos.

- 2026-10-20 — Reprodução isolada do mesmo ci/check-ledger-crash.sh passou sem alteração: LEDGER_CRASH_OK events=10000 sigkill=24 acknowledged=10000 elapsed=107.56s, exit0. Primeiro timeout não teve causa confirmada; carga observada9–14, memória disponível9GiB, sem prova causal. Iniciada nova verificação completa; nenhuma checagem pulada/relaxada e sem commit antes de VERIFY_OK.

- 2026-10-20 — Segundo full5R.1 falhou exit101/1526.00s na fixture witness_deadline: read_timeout180s do cliente→EAGAIN/servidor BrokenPipe;18/19 certify passaram,481 entradas inalteradas. Logs attempt2 preservados. ADR0045 registra teto600s para cliente da fixture e runner agregado de crash, preservando24/10k, limites individuais10/60s, protocolo e janela lógica1024. Terceiro full obrigatório; sem commit/push.

- 2026-10-20 — Terceiro full5R.1 falhou exit101/1744.94s: corpus LRAT retornou Unknown(Time) no cake_lpr para Urquhart-s3-b7.shuffled.cnf, após150 provas/450 corrupções. Log/recibo attempt3 preservados; reprodução isolada do corpus com o mesmo Budget60s iniciada. Nenhum aceite inferido de Unknown, nenhum critério encerrado; sem commit/push.

- 2026-10-20 — Reprodução isolada do corpus confirmou mesma corrupção Unknown(Time), exit101/618.58s. ADR0045 registra Budget do teste60→180s, sem mudar produção nem considerar Unknown como Rejected; concordância200/600 continua obrigatória. Quarto full verify/selftest iniciado, sem commit/push antes de VERIFY_OK.

- 2026-10-21 — Etapa5R.1 concluída localmente sobrea0358f6 (ADR0045). Seis ferramentas MCP stdio por SDK oficial2.3.0 fixado, strict/extra=forbid, sem rotas Fact/Policy/ASSUME/raw RPC; O somente PROPOSE e aceites somente checkers ativos. Dez tarefas via MCP:5 provas LRAT e5 refutações, vínculos h(O)/eventos Accepted, texto tainted preservado, certificado adulterado recusado; auditoria Rust+cake_lpr e export rechecado. Snapshot via QueryHead/Subscribe evita abrir SQLite com writer.lock ativo, sem relaxar locks ou criar formato. CLI não TCB expõe allowlist existente; raiz pública errada e checker desconhecido recusados sobre export.24 testes Python+2 CLI, MCP_RUNTIME_ONLY_OK sem dev e full ci/verify.sh --selftest VERIFY_OK/exit0 em4681.05s;545 Rust/0 ignorados; Python ['307 passed in 167.35s (0:02:47)', '25 passed in 1.79s'];481 entradas inalteradas. TCB Rust15131+747130=762261, build-only64508 preservada; sem novo fuzz/parser/runtime TCB. Falhas de desenvolvimento de stub/query, EOF parcial e anotações mypy corrigidas antes do fechamento. CI remota/ARM/noturno pendentes. Evidências/inputs/corpus em docs/plans/etapa-5R.1-*.json. Próximo9.1 integral antes de5R.2, inclusive revisão humana de50 paráfrases. Commit/push autorizados nesta sessão após verificação.

- 2026-10-21 — Pré-checagem9.1 sobreced0cf3 a pedido do mantenedor. AIR1.1, vocabulário1.2 e parser1.4 existem, mas SymbolDecl/codec vocabulary/v1 só têm gloss_pt livre (spec/vocabulary.md §3.2), sem render_pt exigido pelo contrato9.1. Aplicada parada do AGENTS.md §1.3: nenhuma implementação, migração de formato ou reutilização silenciosa da glosa. Plano de pré-checagem em docs/plans/etapa-9.1.md e próximo passo reescrito para resolver contrato/templates versionados por ADR antes de código. Nenhum critério marcado; full verify não rodou nesta pré-checagem; sem novo commit/push.

- 2026-10-21 — Mantenedor autorizou prosseguir na resolução do contrato. ADR0046 adotado e pré-requisito9.1a explicitado antes da9.1 no ROADMAP, preservando os critérios integrais posteriores. Templates explícitos em vocabulary/v2, validação limitada, compatibilidade e superfície Rust/Python implementados;5 testes Rust de contrato+1 superfície+7 Python passaram (inclui diferencial102 documentos). Lint inicial detectou doc/string longa; correção em curso. Fuzz/mutações/kernel real/full verify ainda pendentes; sem commit/push e sem marcar conclusão.

- 2026-10-21 — Revisão9.1a detectou busca quadrática na compatibilidade de templates; substituída por índice por nome, preservando contrato. Primeiro full/fuzz interrompidos voluntariamente antes de terminar (exit-15;446.79s/565.68s), não contados como sucesso; recibos attempt1 preservados. Quatro mutações/controles serão repetidos nas fontes finais, seguidos de fuzz/full completos. Cenário v2 via MCP/kernel/auditor completo passou; sem commit/push até fechamento.

- 2026-10-21 — Após índice por nome, quatro mutações repetidas com controles positivos e restauração por hash; fmt/Clippy passaram. Medição final: TCB própria15262 (+131), deps747130, total762392/build-only64508, sem dependência nova. Segunda execução full verify/selftest e nova rodada completa de fuzz3600s+60s ativas em target/verify-9.1a.* e target/fuzz-9.1a.*. Nenhum critério encerrado até os dois recibos finais; sem commit/push.

- 2026-10-21 — Fuzz9.1a encerrado: render_template3600s em dois lotes (5928617+5937627 execuções/1801s cada) e vocabulary_decode60s (4895 execuções/61s), exit0, sem crash;3773.12s de parede e fontes inalteradas. Recibo target/fuzz-9.1a.json. Segundo full verify ainda ativo; farm500 passou p50=2.18480858s. Aguardando MCP/auditoria, crash/índices e verificações finais antes de encerrar critérios/commitar.

- 2026-10-21 — Segundo full9.1a terminou exit1/4628.09s no Ruff do novo gerador: I001(imports) e RUF005(concatenação de listas); não houve VERIFY_OK. Rust, farm500, MCP v1/v2, crash e índices passaram;491 entradas inalteradas. Recibo/log/inputs attempt2 preservados. Gerador corrigido sem mudar vetores/TCB; ampliado preflight para tools/tcb e incluído gerador no mypy estrito. Fuzz1h permanece válido, fontes cobertas inalteradas. Nova verificação completa necessária; sem commit/push.

- 2026-10-21 — Preflight Python:313 passaram/1 falhou na fixture antiga de timeout/cancelamento da farm, que lia worker.pid sob prazo100ms (ausente; reprodução isolada vazio). Fixture passa a capturar Process/PID real, sincronizar criação antes de cancelar e exigir SIGKILL/reap, preservando timeout100ms e produção. Novo preflight necessário; fuzz1h/TCB continuam inalterados. Sem commit/push.

- 2026-10-21 — Preflight Python completo corrigido passou: Ruff/formatação py/tools/tcb, mypy54+28 fontes, vetores/corpus/smokes,314 testes Python em343.19s e25 TCB em2.69s. Fixture de timeout captura Process real e exige SIGKILL/reap; produção/timeout100ms preservados. Terceiro full verify/selftest em andamento, sem novo VERIFY_OK até agora; fuzz1h válido/inalterado.

- 2026-10-21 — Terceiro full9.1a falhou exit101/1145.85s no arranque da fixture binary_serves: processo vivo sem socket após teto15s, antes de RPC;13/14 testes transport passaram. Logs/recibo attempt3 preservados. Reprodução intacta isolada passou (15.22s de teste total). Fixture passa a aguardar até120s finitos para hash/registro do executável debug, sem alterar prazos de protocolo/checkers ou produção. Repetir transport e full; preflight Python/fuzz1h continuam válidos. Sem commit/push.

- 2026-10-21 — Transport inteiro passou após teto de startup120s:14/14 em14.48s, sem mudar limites de RPC/checker. Quarto full verify/selftest iniciado com fontes congeladas; Python preflight314+25 e fuzz1h já demonstrados. Aguardar VERIFY_OK/exit0 antes de fechar9.1a e commitar/push.

- 2026-10-21 — Etapa9.1a concluída localmente sobreced0cf3 (ADR0046). Quarto full interrompido sem veredito às16:30 (farm400/500; processo e wrapper sumiram sem erro no log), preservado como attempt4 e não contado. Quinto full verify/selftest, desacoplado da sessão: VERIFY_OK/exit0 em3566.59s,491 entradas inalteradas,551 Rust/0 ignorados, Python314+25, RENDER_TEMPLATE_VECTORS_OK14, RENDER_TEMPLATES_SMOKE_OK10 tarefas v2 com auditoria, farm500 p50=1.258s. Fuzz3600s+60s e quatro mutações já demonstrados nas fontes finais. TCB15262+747130=762392 (+131), build-only64508, sem dependência nova. Recibos em docs/plans/etapa-9.1a-*.json. Próximo:9.1 integral. CI remota/ARM/noturno pendentes.

- 2026-10-21 — Docs: revisão de produto a pedido do mantenedor. Trilha transversal proposta no Apêndice G do ROADMAP (WS.0–WS.9: contrato de sessão, servidor web local, registry de projetos, artefatos, visualizações, contexto de projeto, documentos, workspace científico e operação), extensão docs/spec/LOGOS-workspace-and-web-ui.md (WS§1–16), ADRs 0047–0049 propostos, invariantes W1–W9, ameaças TW1–TW13/AW1–AW3, métricas e riscos. Integrações como notas em 5R.3–5R.5, 9.1/9.2, 10, 15, 16, 23 e Apêndice F, sem novas dependências para 5R/10/MEM, sem eventos de ledger e sem mudança de TCB/critérios. Nenhuma etapa implementada ou marcada. Referências de próximo passo desatualizadas no §5 e no Apêndice F foram datadas. Validação documental: DOCS_0_7_OK, links/âncoras e grafo WS acíclico. Próximo passo continua9.1.
