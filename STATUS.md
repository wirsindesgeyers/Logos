# STATUS — onde o projeto parou

> Mantido pelo agente (regras em [`AGENTS.md`](AGENTS.md) §6). Para continuar, peça:
> **"implemente o próximo passo do STATUS.md"**.
> Última atualização: 2026-10-05 · referência local `origin/main` registrada em `7c5e8a3`; os commits seguintes
> (`git log origin/main..HEAD`) ainda **não foram enviados**.

## Resumo

Fase 0 (Fundação): **7 etapas implementadas; P2 dos controles 0.1/0.2 corrigidos localmente**.
Fase 1 (**1.1–1.6**) demonstrada localmente; CI/ARM/fuzz noturno pendentes.
Há código real em `logos-canon`, `logos-crypto`, `logos-core`, `logos-air`,
`logos-air-text`, `logos-obligation`, `logos-protocol`, `logos-model` e CAS de `logos-ledger`;
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
TCB correção: 6677+78287=84964, build-only51581; logos-model1845 LOC, inalterada.
Nenhum Verdict/kernel/gateway/farm. Formatos e dependências Rust inalterados.

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

Roteiro em [plano da Fase 2](docs/plans/fase-2.md). Fases 3–23 não iniciadas.

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

## Próximo passo: Etapa 3.1 — catálogo de eventos v1 (`M`, TCB)

Dependências prontas: canon/v1 (0.3), hash/assinatura/KeyId/Signer (0.4),
Budget/CheckOutcome (0.6) e obrigação/h(O) (1.5). Fase 2 possui modelos,
avaliador/checkers, produtores e apresentação não autoritativa.
Ler ROADMAP 3.1, Apêndice C e spec conceitual; registrar plano de etapa
e ADR antes de definir wire. Criar `spec/events.md` normativo.

Implementar em logos-ledger o catálogo mínimo: Genesis, VocabPublished,
Proposed, ObligationCreated, Verified, Refuted, UnknownRecorded,
AssumptionDeclared, Retracted, Invalidated, EpochAdvanced,
CheckerRegistered e CheckerRevoked. Event contém seq/epoch/prev/body/
kernel_key/sig; checks carregam h(O)/h(cert)/h(checker)/budget/outcome.
Não implementar persistência/appender (3.2), índices, kernel/gateway
ou eventos introduzidos nas fases 5–17.

Critério: vetores canônicos de cada tipo e teste de adulteração de todos
os campos invalidando assinatura. Aplicar regras TCB, manifesto e fuzz
para cada decoder novo; demonstrar mutantes negativos e smoke público.

Decisões abertas: envelope/tags e payloads mínimos de cada evento,
domínio de hash/assinatura e representação de checks por rota, sem
inferir Refuted de Rejected/Unknown. Reutilizar formatos existentes.
Pendências remotas 0.1–2.6 separadas: local não fecha S3/ARM/CI/fuzz longo.
h(O) esperado deve vir do monitor; fixtures/readers não autenticam proveniência.

## Decisões em aberto para o mantenedor

- **Tamanho da TCB de correção (execução):** 6677 LOC próprios vs. 78287 LOC de dependências de execução
  (+ 51581 só de compilação), após 2.3; logos-air: 1556 LOC, logos-obligation: 550 LOC, logos-model: 1845 LOC.
  Maiores: `curve25519-dalek` 26 k, `unicode-normalization` 23,9 k (NFC),
  `typenum` 16 k. Se isso incomodar para a avaliação (11.9): restringir o texto canônico (p.ex.
  identificadores ASCII) tira a NFC; trocar a curva exigiria outra biblioteca. Não alterado. A
  medição agora filtra pela plataforma do host e separa execução × compilação.
- Planos por etapa em `docs/plans/etapa-X.Y.md`: para 0.1–0.4 só existe o plano de fase
  (`docs/plans/fase-0.md`); a partir da 0.5 vale seguir `AGENTS.md` §2.

## Armadilhas conhecidas

Ver `AGENTS.md` §8 (toolchain, `cp -i`, `pkill -f`, Unicode 16, `gh` ausente, `gen` reservado).
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
