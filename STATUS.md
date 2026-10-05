# STATUS — onde o projeto parou

> Mantido pelo agente (regras em [`AGENTS.md`](AGENTS.md) §6). Para continuar, peça:
> **"implemente o próximo passo do STATUS.md"**.
> Última atualização: 2026-10-05 · referência local `origin/main` registrada em `7c5e8a3`; os commits seguintes
> (`git log origin/main..HEAD`) ainda **não foram enviados**.

## Resumo

Fase 0 (Fundação): **7 etapas implementadas; P2 dos controles 0.1/0.2 corrigidos localmente**.
Fase 1 (**1.1–1.6**) demonstrada localmente; CI/ARM/fuzz noturno pendentes.
Há código real em `logos-canon`, `logos-crypto`, `logos-core`, `logos-air`,
`logos-air-text`, `logos-obligation`, `logos-protocol` e CAS de `logos-ledger`;
demais crates continuam esqueletos. Nenhum kernel/gateway implementado.
1.6: JSON proposal/v1 estrito fora da TCB, somente Proposal, objetivo air/v1,
refs de eventos/ClaimId distintos e UntrustedText preservado, excluído de O.
Extras/duplicatas/tipos/versões/limites recusados; schema sem campos de autoridade.
17 testes Rust, seis vetores independentes e dois mutantes mortos.
PROTOCOL_SMOKE_OK: JSON → candidato → gerador real com snapshot em memória;
texto/ordem não alteram h(O), speculative_on não torna Pending citável.
ci/verify.sh --selftest: VERIFY_OK (exit 0), 175 Python + 25 TCB.
TCB inalterada: correção 4610+78287=82897, build-only 51581. Sem pacote externo
novo no lockfile; serde/JSON runtime somente fora da TCB. O não concede prova/token.

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

Roteiro em [plano da Fase 1](docs/plans/fase-1.md). Fases 2–23 não iniciadas.

## Pendências de verificação (nada disso depende de código novo)

1. `git push` dos commits locais (0.4 etc.) e conferir **Actions**:
   `ci.yml` (jobs `rust`, `sort-coverage`, `tcb-rules`, `tcb-report`, `supply-chain`, `python`, `selftest`) e
   `fuzz.yml` (smoke no PR / noturno).
2. Abrir PRs de teste deliberados e confirmar o bloqueio (`unsafe {}` em crate de TCB; dependência
   fora da allowlist; arquivo de TCB fora do manifesto) e o comentário automático de diff da TCB.
3. Confirmar que o job de fuzz **realmente executa** os alvos na CI e completar ≥ 1 h por alvo
   (`canon_decode`, `canon_text`, `crypto_decode`, `crypto_keyfile`, `cas_domain`), com logs
   arquivados (critérios da 0.3–0.5). Smoke local não fecha essa pendência.
   Alvos `core_decode` (0.6), `air_decode` (1.1), `vocabulary_decode` (1.2) e
   `sort_check` (1.3) integrados ao smoke/noturno; logs locais não provam CI nem ≥ 1 h.
   Smokes AIR/vocabulário/sort-check/AIR textual/obrigações/protocolo ARM ainda não observados.
4. S3 da 0.6 exige execução nativa em `ubuntu-24.04` e `ubuntu-24.04-arm`.
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

## Próximo passo: Etapa 2.1 — estruturas finitas (`P`, TCB)

Fase 1 demonstrada localmente. Dependências da 2.1 prontas: sintaxe Formula/Term/
Sort/Symbol (1.1) e Vocabulary com domínios/grounding versionados (1.2).
Ler ROADMAP 2.1, preparar plano curto de fase 2 e plano 2.1 conforme decisões.

Implementar logos-model: FiniteModel com h(V), tamanhos por sort, constantes,
tabelas totais de funções e conjuntos de tuplas de predicados. Validar aridades,
sorts, totalidade e valores no domínio; Enumerated deve refletir exatamente
elementos declarados. Definir spec/model.md, formato/versionamento, limites e
vetores canônicos em spec/vectors/model/; excesso de tamanho é Unknown(Size),
nunca modelo parcial aceito ou truncado.

Decisões em aberto: representação de tabelas/ordem dos eixos, identificação de
símbolos/sorts no formato, domínios vazios, produto de cardinalidades/overflow e
limites antes de alocar. Resolver em ADR sem antecipar semântica de 2.2.
Fora do escopo: modelos infinitos, tabelas de símbolos Defined (expansão 2.2),
avaliador M ⊨ φ (2.2), busca Z3 (2.4), ledger real ou kernel.

Critérios: função parcial e valor fora do domínio rejeitados; vetores canônicos
fixos reproduzíveis. Sendo TCB, atualizar manifesto/lints, dependências só via
ADR/deny, fuzz de parser novo, mutantes e smoke público além de full verify.
P1 preliminar 1.6 está demonstrado, não autentica refs nem executa especulação.

Pendências remotas 0.1–1.6 separadas: execução local não fecha CI/ARM/fuzz longo.
Cobertura 1.3 em target/sort-check-coverage.json é métrica instrumentada, não prova formal.

## Decisões em aberto para o mantenedor

- **Tamanho da TCB de correção (execução):** 4610 LOC próprios vs. 78287 LOC de dependências de execução
  (+ 51581 só de compilação), após 1.5; logos-air: 1547 LOC, logos-obligation: 335 LOC.
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
