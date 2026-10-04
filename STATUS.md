# STATUS — onde o projeto parou

> Mantido pelo agente (regras em [`AGENTS.md`](AGENTS.md) §6). Para continuar, peça:
> **"implemente o próximo passo do STATUS.md"**.
> Última atualização: 2026-10-04 · referência local `origin/main` registrada em `7c5e8a3`; os commits seguintes
> (`git log origin/main..HEAD`) ainda **não foram enviados**.

## Resumo

Fase 0 (Fundação): **7 de 7 etapas implementadas e verificadas localmente** (0.1–0.7).
Há código real em `logos-canon`, `logos-crypto`, `logos-core` e no CAS de `logos-ledger`;
os outros crates continuam esqueletos. Novos jobs da CI ainda não foram observados.
Verificação local completa desta sessão: `ci/verify.sh --selftest` terminou com
`VERIFY_OK` na árvore de trabalho; smoke público do core e smoke temporário das
primitivas passaram. Etapa 0.7 é documentação, não implementação do kernel/gateway.

## Etapas da Fase 0

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 0.1 Repositório, workspace e CI | ⚠ concluída, falta confirmar na CI | Build/clippy/testes/deny/audit locais passam; selftest cobre regras da TCB e independência direta/transitiva do auditor, incluindo ponte externa opcional com aliases e controle permitido. **Falta:** ver `ci.yml` verde no GitHub e os PRs de teste bloqueados de verdade. |
| 0.2 Manifesto e contagem da TCB | ⚠ concluída, falta confirmar na CI | `cargo xtask tcb check/report` funcionam; diff e comentário de PR escritos mas **nunca executados num PR real**. |
| 0.3 `logos-canon` (canon/v1) | ⚠ concluída, falta 1 h de fuzz | 28 testes unitários + propriedades + 77 vetores + diferencial Rust×Python (100k/direção) passam; 7 mutantes do decoder pegos. Fuzz local: 1,3 M (`canon_decode`) e 0,8 M (`canon_text`) execuções sem crash. **Falta:** ≥ 1 h por alvo no job noturno (`fuzz.yml`). |
| 0.4 `logos-crypto` | ⚠ concluída, falta 1 h de fuzz na CI | 36 testes (RFC 8032, vetores independentes de hash/KeyId/assinatura gerados com `hashlib`/`cryptography`, separação de domínios, chaves fracas, maleabilidade, arquivo de chave 0600/symlink) + 2 doctests `compile_fail,E0308`; 13 mutantes, todos mortos (inclui `verify` frouxo × estrito e `open` sem `O_NOFOLLOW`/`O_NONBLOCK`); fuzz local 0,5 M (`crypto_decode`) e 3,7 M (`crypto_keyfile`) execuções sem crash. **Falta:** ≥ 1 h por alvo na CI e confirmar `O_NOFOLLOW` do aarch64. |
| 0.5 CAS | ⚠ implementada, falta CI/fuzz noturno | 17 testes (6 unitários + 10 públicos + 1 de vetores), smoke real e 6 mutantes mortos; 3 endereços reproduzidos só por `spec/cas.md` + `hashlib`; fuzz final `cas_domain`: 798945 execuções em 61 s sem crash. **Falta:** CI e ≥ 1 h no job noturno. ADR 0005, `spec/cas.md` e [`plano`](docs/plans/etapa-0.5.md) §7. |
| 0.6 `logos-core` (veredictos/orçamentos) | ⚠ implementada, falta CI nativa S3 | 21 testes + 7 fixtures compile-fail e controle positivo; 126 vetores independentes, smoke real e 11 mutantes mortos; fuzz `core_decode`: 224269 execuções em 61 s sem crash. ADR 0006, `spec/core.md` e [`plano`](docs/plans/etapa-0.6.md) §7. **Falta:** observar 1000 execuções por caso em x86_64/aarch64 na CI e confirmar flags crypto em ARM. |
| 0.7 Ameaças e escopo (docs) | ✅ concluída localmente | Três documentos + plano; script reproduzível `DOCS_0_7_OK`: 18 ameaças, 8 atacantes com referências recíprocas, G1–G3 com premissas/TCB/estado, L1–L3 e links locais. Apêndice B distingue mitigação projetada de limite fundamental. Sem nova mitigação/modelo formal. |

Fases 1–23: não iniciadas.

## Pendências de verificação (nada disso depende de código novo)

1. `git push` dos commits locais (0.4 etc.) e conferir **Actions**:
   `ci.yml` (jobs `rust`, `tcb-rules`, `tcb-report`, `supply-chain`, `python`, `selftest`) e
   `fuzz.yml` (smoke no PR / noturno).
2. Abrir PRs de teste deliberados e confirmar o bloqueio (`unsafe {}` em crate de TCB; dependência
   fora da allowlist; arquivo de TCB fora do manifesto) e o comentário automático de diff da TCB.
3. Confirmar que o job de fuzz **realmente executa** os alvos na CI e completar ≥ 1 h por alvo
   (`canon_decode`, `canon_text`, `crypto_decode`, `crypto_keyfile`, `cas_domain`), com logs
   arquivados (critérios da 0.3–0.5). Smoke local não fecha essa pendência.
   Alvo `core_decode` da 0.6 também integrado ao smoke/noturno; log local não prova CI.
4. S3 da 0.6 exige execução nativa em `ubuntu-24.04` e `ubuntu-24.04-arm`.
   [API do GitHub](https://api.github.com/repos/wirsindesgeyers/Logos) confirmou
   repositório público (`private=false`); disponibilidade/execução do runner ARM
   ainda depende de observar o job remoto.
5. Arquivar relatório JSON por commit em `tcb/history/` (entregável da 0.2): diretório
   encontrado vazio na revisão de 2026-10-02; ferramentas existem, artefato versionado falta.

## Próximo passo: Etapa 1.1 — sintaxe abstrata da lógica (`M`, TCB)

Criar `docs/plans/etapa-1.1.md` no formato do ROADMAP §0 e implementar
`FOL-ms-eq/v1` em `logos-air`, conforme Etapa 1.1: `Sort`, `Symbol`, `Term`,
`Formula`, variáveis De Bruijn, `LogicId`, limites estruturais de profundidade/nós
e codificação em `spec/air.md` com vetores. `logos-air` ainda é esqueleto, mas já
consta no manifesto. Dependências prontas: canon/v1 (0.3), `Domain`/`Hash<T>` e
tag reservada `logos/formula/v1` (0.4); documentação de fronteiras 0.7 disponível.

Critérios: alfa-equivalência gera bytes/hash idênticos, profundidade 10⁶ é recusada
sem estourar pilha, formato normativo com vetores. Não incluir vocabulário (1.2),
sort-check (1.3), parser textual (1.4), `Says` (6.6) nem lógicas não clássicas.
Novo decoder requer fuzz target/manifesto; formato novo e decisões técnicas
requerem spec/vetores e ADR conforme regras do projeto.

Decisões para o plano/ADR: representação/identidade de sorts e símbolos,
codificação de binders/LogicId e cobrança de limites configuráveis.
Risco principal: rejeição de profundidade deve cobrir construção, percurso,
codificação e descarte de entrada adversarial sem recursão descontrolada.
Não atribuir autoridade a tipos públicos nem assumir fidelidade pela canonicidade.

Pendências remotas 0.1–0.6 e histórico JSON da 0.2 permanecem separados;
documentação 0.7 não os fecha. Revisão direta desta sessão cobriu decoder/valores
canon Rust e referência Python, domínios/assinatura/FileSigner, medidor/veredictos/
codificação core e publicação/leitura CAS. Nenhum bug novo identificado nesse
recorte; não é auditoria exaustiva nem prova formal.

## Decisões em aberto para o mantenedor

- **Tamanho da TCB (execução):** 2736 LOC próprios vs. 78287 LOC de dependências de execução
  (+ 51581 só de compilação), medição local após 0.6; inclui testes em `src/`.
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
