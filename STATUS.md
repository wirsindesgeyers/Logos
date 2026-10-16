# STATUS — onde o projeto parou

> Mantido pelo agente (regras em [`AGENTS.md`](AGENTS.md) §6). Para continuar, peça:
> **"implemente o próximo passo do STATUS.md"**.
> Última atualização: 2026-10-16 · referência local `origin/main` registrada em `7c5e8a3`; os commits seguintes
> (`git log origin/main..HEAD`) ainda **não foram enviados**.

## Resumo

Fase 0 (Fundação): **7 etapas implementadas; P2 dos controles 0.1/0.2 corrigidos localmente**.
Fase 1 (**1.1–1.6**) demonstrada localmente; CI/ARM/fuzz noturno pendentes.
Há código real em `logos-canon`, `logos-crypto`, `logos-core`, `logos-air`,
`logos-air-text`, `logos-obligation`, `logos-protocol`, `logos-model`, `logos-cli`,
`logos-audit`, `logos-ground`, `logos-lrat` e CAS/eventos/índices de `logos-ledger`;
demais crates continuam esqueletos. Nenhum kernel/gateway implementado.
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
TCB atual Rust: 11549+747130=758679 LOC de correção, build-only64508;
logos-core344, logos-ledger3271, logos-obligation572. SQLite 3.53.2 bundled acrescenta 176466 LOC C medidos
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
na4.5; produtor SAT público e used_premises continuam nas4.6/4.7.

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
| 4.6 Produtor SAT | ⏳ não iniciada | CNF4.3, checkers4.4/4.5, ModelChecker2.3 e encoder Python0.3 prontos. Próximo: produtor fora da TCB e corpus de300 obrigações, respeitando os limites de perfil documentados. |

Roteiro em [plano da Fase4](docs/plans/fase-4.md). Grounder/CNF e rederivação implementados;
Checker LRAT4.4 e recheck duplo4.5 demonstrados no corpus exigido. Kernel/políticas ainda ausentes,
fontes opacas não demonstram autenticação.

## Pendências de verificação (nada disso depende de código novo)

1. `git push` dos commits locais (0.4 etc.) e conferir **Actions**:
   `ci.yml` (jobs `rust`, `sort-coverage`, `tcb-rules`, `tcb-report`, `supply-chain`, `python`, `selftest`) e
   `fuzz.yml` (smoke no PR / noturno).
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
pseudocódigo, M1–M12, 20 critérios concretos, benchmarks e mudanças cirúrgicas
para a spec original. [ADR 0008](docs/adr/0008-memoria-de-longo-prazo.md)
**proposto**, não aceito/implementado. Kernel/gateway/harness continuam ausentes;
nenhum checkmark do ROADMAP ou etapa de implementação foi criado.

Smoke estrutural descartável: 1M eventos, 1.999.993 nodes, 161 parts,
frame máximo 8.192/8.192 bytes, original #42 não NFC em 16 descidas.
Folding adversarial terminou em 8.123 bytes apesar de 236 merges que
aumentaram bytes privados; floresta extrema de 63 raízes ficou em 98.794 bytes.
Fixture SQLite WAL/FULL com três SIGKILLs preservou inbox/ID; resolver nativo
recusou fonte alterada/ausente. Isso não verifica módulos reais, LLM/recall,
PDF/OCR, ACL/providers ou kernel. Dois runners iniciais atingiram 300s/90s;
versões indexada e de comparação menor passaram; temporários removidos.

Adoção exige aprovação, formatos/domínios/vetores normativos e etapas com
dependências explícitas. Próximo passo de código: Etapa 2.1; memória não implementada.
Sem full verify/CI/commit/push nesta entrega documental.

## Próximo passo: Etapa4.6 — produtor SAT fora da TCB

Implementar logos_provers.sat conforme ROADMAP4.6. Estão prontos o encoder
Python0.3, a CNF/tabela de átomos4.3, o checker LRAT4.4 e o ModelChecker2.3;
cake_lpr4.5 fornece a validação externa exigida pelo critério ponta a ponta.
Ler o contrato e registrar o plano/decisões antes de implementar.

O produtor executa CaDiCaL ou Kissat e, se necessário, conversão DRAT→LRAT,
tudo fora da TCB. Demonstrar obrigação finita→CNF→solver→LRAT→cake_lpr em
≥95% de300 obrigações de política; para SAT, reconstruir FiniteModel pela
tabela de átomos e demonstrar100% dos contramodelos aceitos pelo ModelChecker.
Políticas autenticadas ainda não existem: usar obrigações explícitas no
corpus, sem antecipar Policy/fatos/kernel. Portfólio/cache10.2,
used_premises4.7 e kernel5.4 estão fora do escopo.

Riscos: perfis LRAT diferem em duplicates e sparse; não normalizar bytes/IDs
silenciosamente. Fixar solver/conversor e preservar h(CNF)/h(certificado),
limites positivos e resultados inconclusivos sem convertê-los em aceitação.
Decisões em aberto: solver, formato de prova e eventual conversor; resolver
com ADR conforme a evidência, sem acrescentar dependências à TCB.

4.5 passou integralmente em ci/verify.sh --selftest: VERIFY_OK/exit0 em
target/verify-4.5-final.log/.exit;200/600 em380.65s, onze mutantes e fuzz final
2821026/61s. Primeiro critério permanece desmarcado até rebuild em segundo
host; CI/ARM/fuzz longo também pendentes. O teste integrado de substituição
no kernel está preservado na5.4 (ADR0030/A). Alterações preexistentes de
memória/fontes de fuzz/logs/mutants.out preservadas; nenhum push autorizado.

## Decisões em aberto para o mantenedor

- **Tamanho da TCB de correção (execução):** 11549 LOC Rust próprios + 747130
  de dependências Rust = 758679 (+64508 só de compilação), após4.5;
  logos-core344, logos-air1556, logos-obligation572, logos-model1847, logos-ground1337, logos-lrat865, logos-ledger3271.
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
