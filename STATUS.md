# STATUS — onde o projeto parou

> Mantido pelo agente (regras em [`AGENTS.md`](AGENTS.md) §6). Para continuar, peça:
> **"implemente o próximo passo do STATUS.md"**.
> Última atualização: 2026-10-02 · referência local `origin/main` em `7c5e8a3`; os commits seguintes
> (`git log origin/main..HEAD`) ainda **não foram enviados**.

## Resumo

Fase 0 (Fundação): **4 de 7 etapas concluídas** (0.1–0.4). Há código real em `logos-canon` (codificação
canônica) e `logos-crypto` (hash com domínio, Ed25519, `KeyId`, `FileSigner`); os outros crates são
esqueletos. A CI do GitHub **ainda não foi observada** (o `gh` não está instalado aqui): tudo foi
verificado *localmente* com `ci/verify.sh --selftest`.

## Etapas da Fase 0

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 0.1 Repositório, workspace e CI | ⚠ concluída, falta confirmar na CI | Build/clippy/testes/deny/audit locais passam; `ci/selftest.sh` prova que `unsafe`, `unwrap`, remoção do lint, dependência fora da allowlist, arquivo fora do manifesto e `logos-audit→kernel` são bloqueados pelo motivo certo. **Falta:** ver `ci.yml` verde no GitHub e os PRs de teste bloqueados de verdade. |
| 0.2 Manifesto e contagem da TCB | ⚠ concluída, falta confirmar na CI | `cargo xtask tcb check/report` funcionam; diff e comentário de PR escritos mas **nunca executados num PR real**. |
| 0.3 `logos-canon` (canon/v1) | ⚠ concluída, falta 1 h de fuzz | 28 testes unitários + propriedades + 77 vetores + diferencial Rust×Python (100k/direção) passam; 7 mutantes do decoder pegos. Fuzz local: 1,3 M (`canon_decode`) e 0,8 M (`canon_text`) execuções sem crash. **Falta:** ≥ 1 h por alvo no job noturno (`fuzz.yml`). |
| 0.4 `logos-crypto` | ⚠ concluída, falta 1 h de fuzz na CI | 36 testes (RFC 8032, vetores independentes de hash/KeyId/assinatura gerados com `hashlib`/`cryptography`, separação de domínios, chaves fracas, maleabilidade, arquivo de chave 0600/symlink) + 2 doctests `compile_fail,E0308`; 13 mutantes, todos mortos (inclui `verify` frouxo × estrito e `open` sem `O_NOFOLLOW`/`O_NONBLOCK`); fuzz local 0,5 M (`crypto_decode`) e 3,7 M (`crypto_keyfile`) execuções sem crash. **Falta:** ≥ 1 h por alvo na CI e confirmar `O_NOFOLLOW` do aarch64. |
| 0.5 CAS | ⏳ próxima; plano pronto | [`docs/plans/etapa-0.5.md`](docs/plans/etapa-0.5.md); implementação não iniciada |
| 0.6 `logos-core` (veredictos/orçamentos) | ⏳ | depende da 0.3 (feita); runner aarch64 na CI |
| 0.7 Ameaças e escopo (docs) | ⏳ | independente; só documentação |

Fases 1–23: não iniciadas.

## Pendências de verificação (nada disso depende de código novo)

1. `git push` dos commits locais (0.4 etc.) e conferir **Actions**:
   `ci.yml` (jobs `rust`, `tcb-rules`, `tcb-report`, `supply-chain`, `python`, `selftest`) e
   `fuzz.yml` (smoke no PR / noturno).
2. Abrir PRs de teste deliberados e confirmar o bloqueio (`unsafe {}` em crate de TCB; dependência
   fora da allowlist; arquivo de TCB fora do manifesto) e o comentário automático de diff da TCB.
3. Confirmar que o job de fuzz **realmente executa** os alvos na CI (o workflow tinha um bug de toolchain, corrigido; nunca foi visto rodando) e deixá-lo completar ≥ 1 h por alvo (`canon_decode`, `canon_text`, `crypto_decode`, `crypto_keyfile`) e arquivar os logs (critérios da 0.3 e 0.4).
4. Repositório é público ou privado? Runner ARM (`ubuntu-24.04-arm`) é gratuito só em público
   (necessário para S3 na 0.6).
5. Arquivar relatório JSON por commit em `tcb/history/` (entregável da 0.2): diretório
   encontrado vazio na revisão de 2026-10-02; ferramentas existem, artefato versionado falta.

## Próximo passo: Etapa 0.5 — CAS (`P`, ⚠ TCB)

Implementar conforme [`docs/plans/etapa-0.5.md`](docs/plans/etapa-0.5.md), no módulo `cas`
de `logos-ledger` (sem crate novo). Plano pronto; nenhum código da 0.5 implementado.

Já pronto de que ela depende: `logos-crypto` (`Hash<T>`, `hash_raw`, `RawHasher` incremental,
`tags::RESERVED`, `Domain`), `spec/domains.md`. `logos-canon` não é necessário (o CAS guarda bytes).

Escopo (ver `ROADMAP.md` Etapa 0.5): `Cas::open(root, Limits)`, `put::<T>(&[u8])`,
`put_reader::<T>(impl Read)` (a v1 pode ler em memória, mas a assinatura já aceita fluxo; use
`RawHasher`), `get::<T>(&Hash<T>) -> Result<Option<Vec<u8>>, CasError>` com **verificação do hash
na leitura**, `CasError { Integrity, TooLarge { limit, actual }, Io }`, escrita atômica
(tmp + rename + fsync do arquivo **e** do diretório), layout `cas/<domínio>/ab/cdef…`, limite de
tamanho por domínio (padrão: certificado ≤ 256 MiB), `spec/cas.md`.

Critérios a provar: corromper 1 byte ⇒ erro de integridade; 16 threads fazendo `put` do mesmo objeto
sem corromper; objeto acima do limite ⇒ `TooLarge`; **endereço de 3 objetos recalculado por um script
Python independente a partir só de `spec/cas.md`** (idem aos vetores da 0.4: gerar com `hashlib`).

Decisões propostas no plano (registrar em ADR 0005 antes do código) / riscos:
- **Diretório por domínio:** tabela de tags reservadas exatas → `certificate-v1` etc.;
  manter `Domain` inalterado. Tag desconhecida retorna `Io(InvalidInput)`, nunca caminho arbitrário.
- **Limites por tag:** canônicos 16 MiB; certificate/checker crus 256 MiB; overrides configuráveis,
  aplicados também em `get`. Excesso vira `TooLarge`, não rejeição de certificado.
- **Bytes opacos:** CAS não valida `canon/v1`; consumidor valida codificação e semântica.
- **Durabilidade:** tmp exclusivo + rename + fsync do arquivo, shard e pais novos;
  erro após publicação não promete rollback. Raiz/ancestrais confiáveis; filesystem local Unix.
- Resolver representação de `Limits` e propagação conjunta de erro primário/limpeza no ADR.
- Sem dependências externas novas esperadas; apenas `logos-crypto` e `std`.
- Teste de concorrência determinístico (sem `sleep`); criar diretórios temporários sem crate `tempfile`
  (ver `tests/security.rs` do crypto para o padrão usado).
- **GC, replicação, compressão e streaming de verdade** estão fora do escopo (3.5 etc.).

## Decisões em aberto para o mantenedor

- **Tamanho da TCB (execução):** 1721 LOC próprios vs. ≈ 78 k LOC de dependências de execução
  (+ ≈ 52 k só de compilação). Maiores: `curve25519-dalek` 26 k, `unicode-normalization` 23,9 k (NFC),
  `typenum` 16 k. Se isso incomodar para a avaliação (11.9): restringir o texto canônico (p.ex.
  identificadores ASCII) tira a NFC; trocar a curva exigiria outra biblioteca. Não alterado. A
  medição agora filtra pela plataforma do host e separa execução × compilação.
- Planos por etapa em `docs/plans/etapa-X.Y.md`: para 0.1–0.4 só existe o plano de fase
  (`docs/plans/fase-0.md`); a partir da 0.5 vale seguir `AGENTS.md` §2.

## Armadilhas conhecidas

Ver `AGENTS.md` §8 (toolchain, `cp -i`, `pkill -f`, Unicode 16, `gh` ausente, `gen` reservado).

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
