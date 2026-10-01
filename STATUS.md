# STATUS — onde o projeto parou

> Mantido pelo agente (regras em [`AGENTS.md`](AGENTS.md) §6). Para continuar, peça:
> **"implemente o próximo passo do STATUS.md"**.
> Última atualização: 2026-10-01 · `origin/main` está em `ce37670`; os commits seguintes (`git log origin/main..HEAD`) ainda **não foram enviados**.

## Resumo

Fase 0 (Fundação): **3 de 7 etapas concluídas** (0.1, 0.2, 0.3). Nenhum código ainda além da
codificação canônica; o resto dos crates são esqueletos vazios. A CI do GitHub **ainda não foi
observada** (o `gh` não está instalado aqui): tudo abaixo foi verificado *localmente*.

## Etapas da Fase 0

| Etapa | Estado | Evidência / pendência |
| --- | --- | --- |
| 0.1 Repositório, workspace e CI | ⚠ concluída, falta confirmar na CI | Build/clippy/testes/deny/audit locais passam; `ci/selftest.sh` prova que `unsafe`, `unwrap`, remoção do lint, dependência fora da allowlist, arquivo fora do manifesto e `logos-audit→kernel` são bloqueados pelo motivo certo. **Falta:** ver `ci.yml` verde no GitHub e os PRs de teste bloqueados de verdade. |
| 0.2 Manifesto e contagem da TCB | ⚠ concluída, falta confirmar na CI | `cargo xtask tcb check/report` funcionam; diff e comentário de PR escritos mas **nunca executados num PR real**. |
| 0.3 `logos-canon` (canon/v1) | ⚠ concluída, falta 1 h de fuzz | 28 testes unitários + propriedades + 77 vetores + diferencial Rust×Python (100k/direção) passam; 7 mutantes do decoder pegos. Fuzz local: 1,3 M (`canon_decode`) e 0,8 M (`canon_text`) execuções sem crash. **Falta:** ≥ 1 h por alvo no job noturno (`fuzz.yml`). |
| 0.4 `logos-crypto` | ⏳ próxima | ver "Próximo passo" |
| 0.5 CAS | ⏳ | depende da 0.4 |
| 0.6 `logos-core` (veredictos/orçamentos) | ⏳ | depende da 0.3 (feita); runner aarch64 na CI |
| 0.7 Ameaças e escopo (docs) | ⏳ | independente; só documentação |

Fases 1–23: não iniciadas.

## Pendências de verificação (nada disso depende de código novo)

1. `git push` dos commits locais (achados da revisão externa, docs) e conferir **Actions**:
   `ci.yml` (jobs `rust`, `tcb-rules`, `tcb-report`, `supply-chain`, `python`, `selftest`) e
   `fuzz.yml` (smoke no PR / noturno).
2. Abrir PRs de teste deliberados e confirmar o bloqueio (`unsafe {}` em crate de TCB; dependência
   fora da allowlist; arquivo de TCB fora do manifesto) e o comentário automático de diff da TCB.
3. Deixar o fuzz noturno completar ≥ 1 h por alvo e arquivar os logs (critério da 0.3).
4. Repositório é público ou privado? Runner ARM (`ubuntu-24.04-arm`) é gratuito só em público
   (necessário para S3 na 0.6).

## Próximo passo: Etapa 0.4 — `logos-crypto` (`P`, ⚠ TCB)

Já pronto de que ela depende: `logos-canon` (`Canon::to_value/encode` **falíveis**, `Value`,
`Bytes`, `[u8; N]`, `Option<T>` com presença explícita), `spec/encoding.md`, manifesto da TCB.

Escopo (ver `ROADMAP.md` Etapa 0.4): `trait Domain`, `Hash<T>` fantasma, `hash` (canônico) e
`hash_raw` (bytes crus), `ExternalDigest`, `SigAlg` (Ed25519 na v1; ES256 só reservado),
`KeyId`/`PublicKey`/`Signature` com `alg`, `trait Signer` + `FileSigner` (exige permissão `0600`),
`sign`/`verify` com domínio de assinatura, `spec/domains.md` com as tags reservadas, vetores
(`spec/vectors/domains/`), vetores RFC 8032.

Decisões em aberto / riscos (devem virar ADR 0004):
- Dependências novas de TCB: `sha2` e `ed25519-dalek` (e transitivas: `curve25519-dalek`,
  `zeroize`, `subtle`, ...). Verificar quantas entram e o LOC (`cargo xtask tcb report`); usar
  `default-features = false`. Atualizar `deny.toml` e `tcb/manifest.toml`.
- **`hash` devolve `Result`**, porque `Canon::encode` é falível (ADR 0003).
- **Teste de tipo compile-fail:** o roadmap pede `trybuild`, que puxa dezenas de crates. Alternativa
  proposta: *doctests* ```` ```compile_fail,E0308 ````, que exigem o código de erro e evitam a
  allowlist inflada. Decidir e registrar.
- `ExternalDigest` (SHA-256 sem tag) só serve para conferir binários de terceiros; nunca é aceito
  onde se espera `Hash<T>`. O ledger sempre registra `hash_raw::<Checker>`.
- Chaves em arquivo: recusar permissão mais aberta que `0600` (teste com `0644`).
- Não implementar ES256, HSM/TEE, rotação de chaves (fora do escopo: 9.2 e 13.1).

## Decisões em aberto para o mantenedor

- **Tamanho da TCB por causa da NFC:** hoje 1089 LOC próprios vs. ~27,5 k LOC de dependências,
  dos quais 23,9 k são `unicode-normalization`. Se isso incomodar para a avaliação (11.9), uma
  alternativa é restringir o texto canônico (p.ex. identificadores ASCII). Não alterado.
- Planos por etapa em `docs/plans/etapa-X.Y.md`: para 0.1–0.3 só existe o plano de fase
  (`docs/plans/fase-0.md`); a partir da 0.4 vale seguir `AGENTS.md` §2.

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
