# Plano de fase — Fase 0: Fundação

Gerado a partir do `ROADMAP.md` (Revisão 2) e de `docs/spec/LOGOS-proof-carrying-agents.md`, no formato de plano de fase do §0 do roadmap. Os planos detalhados de cada etapa ficam em `docs/plans/etapa-0.X.md`.

## 1. Ordem das etapas e caminho crítico

```text
0.1 Repo/CI ──► 0.2 TCB ──► 0.3 canon (M) ──► 0.4 crypto ──► 0.5 CAS
     │                          │
     └──► 0.6 core (início) ────┴──► 0.6 core (fecha: Canon dos tipos)

0.7 ameaças ∥ desde o dia 1 (só documentação)
```

- **Caminho crítico: 0.1 → 0.2 → 0.3 → 0.4 → 0.5** (≈ 3–4 semanas). A 0.3 é a etapa grande e a de maior risco.
- **0.2 antes da 0.3**: a 0.3 é o primeiro código de TCB; com manifesto e contagem já ativos, a TCB é medida desde o primeiro commit, sem adaptação posterior.
- **0.6** começa logo após a 0.1 (tipos, `StepMeter`, regras de conversão, runner aarch64) e só fecha depois da 0.3, porque `Budget` e `CheckOutcome` precisam de `Canon` (ficam registrados nos eventos, §10 ameaça 18).
- **0.7** é independente; precisa estar pronta antes da Fase 5, mas fecha nesta fase.

## 2. Interfaces entre etapas

| De → Para | Interface (esboço; a forma final é do plano de etapa) |
| --- | --- |
| 0.1 → todas | Workspace com os crates da §2 do roadmap (inclui `logos-core`), lints `forbid(unsafe_code)`, `clippy::unwrap_used`/`panic`, `deny.toml`, CI de PR + job noturno de fuzz |
| 0.2 → todas ⚠ TCB | `tcb/manifest.toml` por propriedade (correção, fidelidade, progresso) e job que falha se um arquivo de TCB não está listado |
| 0.3 → 0.4, 0.6, 1.x, 10.1 | `const CANON_VERSION: &str = "canon/v1"`; `trait Canon { fn encode(&self, out: &mut Vec<u8>); }`; `trait CanonDecode: Sized { fn decode(d: &mut Decoder) -> Result<Self, CanonError>; }`; `fn from_canon<T: CanonDecode>(b: &[u8]) -> Result<T, CanonError>` (recusa bytes sobrando); `py/logos_client/canon.py` com `encode`/`decode` |
| 0.4 → 0.5, 1.1, 3.x, 8.x, 9.2, 13.1 | `trait Domain { const TAG: &'static [u8]; }`; `Hash<T: Domain>`; `hash<T: Domain + Canon>(&T)`; `hash_raw<T: Domain>(&[u8])`; `ExternalDigest`; `enum SigAlg { Ed25519 /* Es256 reservado */ }`; `KeyId`, `PublicKey`, `Signature` com `alg`; `trait SigDomain`; `trait Signer`; `FileSigner::load(path)` (exige 0600); `verify::<D>`; `spec/domains.md` |
| 0.5 → 2.3, 3.2, 3.5, 3.6, 4.x, 17.x | `Cas::open(root, Limits)`; `put::<T>(&[u8])`, `put_reader::<T>(impl Read)`; `get::<T>(&Hash<T>) -> Result<Option<Vec<u8>>, CasError>`; `CasError { Integrity, TooLarge { limit, actual }, Io }`; `spec/cas.md` |
| 0.6 → 2.2, 3.1, 4.4, 5.x | `CheckOutcome`, `Reason`, `Exhausted`, `Budget`, `StepMeter`, `Verdict`, todos `Canon`; sem caminho `Unknown → Verified/Refuted` nem `Rejected → Refuted` |
| 0.7 → 11.10, Apêndice B | `docs/threat-model.md`, `docs/guarantees.md`, `docs/limits.md` |

## 3. Paralelismo

- **Trilha A (crítica):** 0.1 → 0.2 → 0.3 → 0.4 → 0.5.
- **Trilha B:** 0.6, a partir da 0.1; fecha após a 0.3.
- **Trilha C:** 0.7, a qualquer momento.
- **Dentro da 0.3:** referência Python e `spec/encoding.md` avançam junto com o encoder Rust — é isso que dá valor ao teste diferencial.

## 4. Marco da fase e como demonstrá-lo

A Fase 0 não tem `★`; o marco da §5 do roadmap é “repositório, canon, crypto, CAS”. Demonstração, sem nada fora do escopo:

1. CI verde a partir de clone limpo, em x86_64 e aarch64.
2. Todos os critérios de 0.1–0.7 marcados, cada um com link para teste ou job.
3. PRs de teste deliberados bloqueados (`unsafe`, dependência fora da allowlist, arquivo de TCB sem manifesto), com links.
4. Primeiro relatório `cargo xtask tcb` em `tcb/history/` com o LOC confiável de canon, crypto, core e CAS.
5. Logs do job noturno de fuzz (≥ 1h por target) arquivados.

## 5. Bloqueios e decisões em aberto

**Bloqueios**
- Remoto GitHub ainda não conectado: os critérios da 0.1/0.2 que envolvem PR e comentário de CI só fecham depois disso.
- Runner aarch64: GitHub oferece runners ARM (`ubuntu-24.04-arm`) gratuitos para repositórios públicos; em repositório privado, confirmar disponibilidade e custo.

**ADRs a abrir**
- 0001 — decisões iniciais e divergências em relação à especificação (token próprio × Biscuit; `logos-core`; assinatura com algoritmo identificado); licença (Apache-2.0 / MIT); `uv`.
- Dependências de TCB: `sha2`, `ed25519-dalek`, `unicode-normalization` (e a versão de Unicode que ele fixa); de desenvolvimento: `proptest`, `trybuild`, `cargo-fuzz`; ferramenta: `tokei`.
- Hash de binário externo: confirmar que `ExternalDigest` (SHA-256 puro) é usado só para conferência contra hashes publicados, e que o ledger sempre registra `hash_raw::<Checker>`.

## 6. Estimativa

Mantida em **M** (≈ 3–4 semanas para uma pessoa), puxada pela 0.3. Riscos de estouro: concordância Rust × Python em ordenação de chaves e NFC (versão de Unicode); configuração da CI multi-arquitetura.
