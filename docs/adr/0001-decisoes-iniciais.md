# ADR 0001 — Decisões técnicas iniciais

- **Status:** aceito
- **Data:** 2026-10-01
- **Contexto:** ROADMAP.md §1 fixa as decisões abaixo para que as etapas sejam acionáveis. Mudá-las exige novo ADR.

## Decisões (transcrição da §1 do roadmap)

| Tema | Decisão | Motivo |
| --- | --- | --- |
| Linguagem da TCB | Rust (edição 2024), `#![forbid(unsafe_code)]` em todo crate de TCB | spec §5, §12; typestate; Verus/Kani depois |
| Linguagem fora da TCB | Python 3.12+ (agente, MCP, prover farm, avaliação); TUI em Rust (Ratatui). Ver ADR 0002 | AgentDojo, CaMeL, FIDES, SDKs de LLM e solvers são Python |
| Nome da representação | AIR = *Argument Intermediate Representation* | Consistência com a especificação |
| Hash | SHA-256 com separação de domínio (`H(tag ‖ bytes)`) | Interoperável, padrão em logs de transparência |
| Assinatura | Ed25519 (`ed25519-dalek`); `Signature` e `KeyId` carregam o identificador do algoritmo; ES256 entra na 9.2 | Passkeys/WebAuthn assinam quase sempre com ES256 (spec §6) |
| Serialização canônica | Subconjunto de CBOR determinístico (RFC 8949 §4.2.1), implementado em `logos-canon`, com decoder estrito | O que é hasheado é exatamente o que é checado (P2) |
| Lógica base | `FOL-ms-eq/v1` | spec §9 |
| Armazenamento | SQLite (`rusqlite`) para ledger e índices; CAS em disco endereçado por hash | spec §12 |
| Transporte agente↔kernel | Unix domain socket, frames `u32 len ‖ CBOR canônico` | Sem rede por padrão; isolamento por SO |
| Formato de token | Struct canônica assinada pelo kernel (v1) | Ver divergência abaixo |
| Modelo formal | Lean 4 | Mesmo ecossistema da camada 3 |
| Verificação de Rust | Kani (invariantes locais); Verus (módulos críticos) | spec §5 |
| Licença | Apache-2.0 **ou** MIT, à escolha de quem usa | Padrão do ecossistema Rust; evita atrito de adoção |
| Gerenciador Python | `uv` | Lockfile reprodutível, rápido, sem estado global |

## Divergências deliberadas em relação à especificação

1. **Token próprio em vez de Biscuit.** A spec (§6, §12) sugere Biscuit “para evitar inventar criptografia”. Na v1 o token é uma struct canônica assinada pelo kernel: só há assinatura sobre bytes canônicos, nenhuma criptografia nova. A v1 não precisa de atenuação offline, e o Biscuit traria um Datalog próprio e mais dependências para a TCB. Biscuit permanece opção futura.
2. **Crate `logos-core`.** A spec não diz onde ficam `CheckOutcome`, `Budget` e `Verdict`; criamos um crate de TCB pequeno para eles, em vez de acoplá-los a `logos-canon` ou `logos-air`.
3. **Assinatura agnóstica de algoritmo.** Ver tabela: evita quebrar o tipo `Signature` quando a 9.2 introduzir ES256.
4. **Tese do primeiro paper.** O README adota a tese da spec §13 (integridade de justificativa), que superou a recomendação da §11.

## Consequências

- Dependências novas em crate de TCB exigem ADR próprio e entrada em `deny.toml` (`[bans].allow`).
- Python e Rust compartilham o formato canônico por especificação (`spec/`) e teste diferencial, não por código.
