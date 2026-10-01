# LOGOS — Roadmap completo

> Proof-carrying agents como **sistema de efeitos mediado por prova** com **ledger epistêmico**.
> Este documento vai das primeiras funções (hash, serialização canônica) até as fases finais (framework lógico único, produto). Cada etapa tem objetivo, entregáveis, o que fica fora, e um **critério de conclusão testável**. Uma etapa só fecha quando todos os itens do critério estiverem marcados.

Referência conceitual: `LOGOS — Proof-Carrying Agents` (Oct 1, 2026), em [`docs/spec/LOGOS-proof-carrying-agents.md`](docs/spec/LOGOS-proof-carrying-agents.md). As seções da especificação são citadas como `§N`; `§N.k` designa o **k-ésimo item numerado** da seção N (a especificação não tem subseções numeradas). Ex.: `§4.9` é o passo 9 (“Recheck”) do *Proof lifecycle*; `§12.1` é o item 1 da *Ordem de construção*. Citações às seções 1–5 deste roadmap são escritas “§N do roadmap” quando houver ambiguidade.

> **Revisão 2 (2026-10-01).** Fase 0 revisada contra a especificação. Corrigidos: 0.6 passa a depender da 0.3 (`Budget` e `CheckOutcome` são registrados nos eventos, §10 ameaça 18) e proíbe também `Rejected → Refuted` (§4.4); 0.4 com assinatura e `KeyId` agnósticos de algoritmo (passkeys usam ES256, §6) atrás de uma trait `Signer` (HSM, 13.1), hash de bytes crus (`h(checker)`, certificados de prover) e mecanismo `Domain` com tags reservadas; 0.3 com identificador de versão `canon/v1` (§4.1) e versão de Unicode fixada para NFC; 0.5 com `spec/cas.md` e API preparada para streaming (§4, §7); 0.7 com atacante que bifurca o ledger (ameaça 9) e TOCTOU (ameaça 10) em `limits.md`; 0.1 com ADR registrando as divergências em relação à especificação (token próprio em vez de Biscuit; tese da §13). Novo crate de TCB `logos-core` para os tipos da 0.6. Fuzz longo roda em job noturno. Especificação movida para `docs/spec/`; convenção `§N.k` explicitada.

> **Revisão 1 (2026-10-01).** Toda etapa ganhou os campos *Depende de (recebe pronto)* e *Fora do escopo*. Corrigidos: semântica de negação entre a rota Datalog e a rota FOL (4.1, 7.3); critério de injetividade da paráfrase (9.1); recheck LRAT no auditor (4.5). Decidida a divisão de linguagens (§1, ADR 0002): TCB e TUI em Rust, agente/farm/avaliação em Python. AIR = *Argument Intermediate Representation*. Fases 19–22 marcadas fora do horizonte do primeiro ciclo (§5). Acrescentado o procedimento de geração de planos (§0).

---

## Sumário

- [0. Como usar este roadmap](#0-como-usar-este-roadmap)
- [1. Decisões técnicas fixadas](#1-decisões-técnicas-fixadas)
- [2. Mapa de componentes e crates](#2-mapa-de-componentes-e-crates)
- [3. Invariantes globais (testadas desde o início)](#3-invariantes-globais-testadas-desde-o-início)
- [4. Definition of Done global](#4-definition-of-done-global)
- [5. Visão geral das fases](#5-visão-geral-das-fases)
- [Fase 0 — Fundação](#fase-0--fundação)
- [Fase 1 — AIR: lógica, vocabulário e obrigações](#fase-1--air-lógica-vocabulário-e-obrigações)
- [Fase 2 — Camada 0: modelos finitos, refutação e consistência](#fase-2--camada-0-modelos-finitos-refutação-e-consistência)
- [Fase 3 — Ledger assinado](#fase-3--ledger-assinado)
- [Fase 4 — Camada 1: grounding, CNF e LRAT](#fase-4--camada-1-grounding-cnf-e-lrat)
- [Fase 5 — Kernel: máquina de estados e protocolo](#fase-5--kernel-máquina-de-estados-e-protocolo)
- [Fase 6 — Atestadores e fatos](#fase-6--atestadores-e-fatos)
- [Fase 7 — Políticas](#fase-7--políticas)
- [Fase 8 — Ações: contratos, tokens e gateway](#fase-8--ações-contratos-tokens-e-gateway)
- [Fase 9 — Paráfrase determinística e autorização do usuário](#fase-9--paráfrase-determinística-e-autorização-do-usuário)
- [Fase 10 — Harness do agente (LLM, MCP, prover farm)](#fase-10--harness-do-agente-llm-mcp-prover-farm)
- [Fase 11 — Integridade de justificativa (primeiro paper)](#fase-11--integridade-de-justificativa-primeiro-paper)
- [Fase 12 — Modelo formal mecanizado](#fase-12--modelo-formal-mecanizado)
- [Fase 13 — Hardening, chaves e transparência](#fase-13--hardening-chaves-e-transparência)
- [Fase 14 — Camada 2: primeira ordem via Alethe](#fase-14--camada-2-primeira-ordem-via-alethe)
- [Fase 15 — Selo de cinco campos e fidelidade](#fase-15--selo-de-cinco-campos-e-fidelidade)
- [Fase 16 — Regime epistêmico: benchmark e estudo com usuários](#fase-16--regime-epistêmico-benchmark-e-estudo-com-usuários)
- [Fase 17 — Portable Verified Claims (multi-agente)](#fase-17--portable-verified-claims-multi-agente)
- [Fase 18 — Debate checkável (segundo paper)](#fase-18--debate-checkável-segundo-paper)
- [Fase 19 — Argumentação derrotável (ASPIC+)](#fase-19--argumentação-derrotável-aspic)
- [Fase 20 — Camada 3: Lean](#fase-20--camada-3-lean)
- [Fase 21 — Camada 4: HOL, modal e deôntica](#fase-21--camada-4-hol-modal-e-deôntica)
- [Fase 22 — Framework lógico único](#fase-22--framework-lógico-único)
- [Fase 23 — Produto](#fase-23--produto)
- [Apêndice A — Matriz de rastreabilidade: princípios → etapas](#apêndice-a--matriz-de-rastreabilidade-princípios--etapas)
- [Apêndice B — Matriz de ameaças → etapas](#apêndice-b--matriz-de-ameaças--etapas)
- [Apêndice C — Catálogo de eventos do ledger](#apêndice-c--catálogo-de-eventos-do-ledger)
- [Apêndice D — Métricas que o sistema precisa emitir](#apêndice-d--métricas-que-o-sistema-precisa-emitir)
- [Apêndice E — Riscos do projeto e pontos de decisão](#apêndice-e--riscos-do-projeto-e-pontos-de-decisão)

---

## 0. Como usar este roadmap

1. **Uma etapa = um plano de desenvolvimento.** Cada `Etapa X.Y` foi escrita para virar, sozinha, o input de um plano detalhado. Ela diz *o quê* e *como saber que acabou*; o plano diz *como*.
2. **Critério de conclusão é contrato.** Se um item não pode ser demonstrado por teste automatizado, script reproduzível ou artefato revisável, a etapa não fechou. "Funciona na minha máquina" não conta.
3. **Ordem.** As fases seguem a ordem da §12: primeiro onde as garantias são mais fortes (regime operacional), depois o regime epistêmico. Dentro de uma fase, a ordem das etapas é a ordem recomendada; dependências explícitas estão em `Depende de`.
4. **Paralelismo permitido.** A Fase 12 (modelo formal) roda em paralelo a partir do fim da Fase 5. Etapas marcadas com `∥` podem ser feitas em paralelo com a anterior.
5. **Tamanho.** `P` (dias), `M` (1–2 semanas), `G` (3–6 semanas), `GG` (meses / pesquisa). Estimativa para uma pessoa; serve para ordenar, não para prometer.
6. **TCB.** Toda etapa que adiciona código à TCB de correção diz isso explicitamente (`⚠ TCB`). Esse código tem regras mais duras (§4 abaixo).
7. **Marcos.** `★` marca etapas que fecham um marco demonstrável (demo, paper, release).
8. **Dois níveis de plano.** Cada fase recebe um **plano de fase** curto (ordem das etapas, interfaces entre elas, o que pode ser feito em paralelo). Cada etapa recebe um **plano de etapa** detalhado, gerado a partir dela. Não gerar um plano detalhado único para uma fase inteira.
9. **Campos de cada etapa.** *Depende de (recebe pronto)* lista as etapas anteriores e o que cada uma entrega para esta (tipos, specs, APIs). *Fora do escopo* é contrato tão forte quanto o critério de conclusão: um plano que implemente algo listado ali está errado.

### Prompt de geração de plano de etapa

Entradas: este roadmap, a especificação `docs/spec/LOGOS-proof-carrying-agents.md` (para as referências `§N`), `docs/adr/` e o estado atual do repositório.

```text
Você vai escrever o plano de desenvolvimento da Etapa {X.Y} do ROADMAP.md anexo.
A especificação conceitual anexa é referenciada no roadmap como §N.

Regras:
- O plano cobre SÓ esta etapa. O que estiver em "Fora do escopo" não entra.
- Assuma prontas apenas as etapas listadas em "Depende de (recebe pronto)" e o que
  já existe no repositório. Se algo necessário não existir, liste como bloqueio
  em vez de implementar.
- Se a etapa é ⚠ TCB, aplique as regras da §4 do roadmap (sem unsafe, sem unwrap,
  fuzz target, manifesto da TCB).
- Respeite as decisões da §1 e os ADRs existentes; uma mudança vira proposta de ADR.

Formato:
1. Interfaces de entrada (tipos, specs, APIs que a etapa consome)
2. Interfaces de saída (o que etapas futuras vão consumir), com assinaturas
3. Tarefas em ordem, cada uma com arquivos tocados e o teste que a fecha
4. Mapeamento: cada item do "Critério de conclusão" → teste ou script concreto
5. Riscos e decisões em aberto (viram ADR se forem decididas)
6. Estimativa revisada (P/M/G) e o motivo, se diferente do roadmap
```

O plano de fase usa o mesmo cabeçalho de regras, com o formato: ordem das etapas e caminho crítico interno; interfaces que passam de uma etapa para outra; etapas paralelizáveis; o marco da fase e como demonstrá-lo.

---

## 1. Decisões técnicas fixadas

Decisões tomadas aqui para que as etapas sejam acionáveis. Mudá-las é possível, mas deve ser registrado em `docs/adr/` (Architecture Decision Record).

| Tema | Decisão | Motivo |
| --- | --- | --- |
| Linguagem da TCB | Rust (edição 2024), `#![forbid(unsafe_code)]` em todo crate de TCB | §5, §12; typestate; Verus/Kani depois |
| Linguagem fora da TCB | Python 3.12+ para agente, MCP, prover farm e avaliação. **TUI em Rust (Ratatui)**, como crate fora da TCB. Registrado em `docs/adr/0002-divisao-de-linguagens.md` | AgentDojo, CaMeL, FIDES, SDKs de LLM e solvers são Python: a avaliação da Fase 11 fica muito mais barata. A interface de usuário segue a decisão original de produto em Rust. Revisável: se a decisão for "tudo em Rust", só as Fases 10–11 mudam |
| Nome da representação | AIR = *Argument Intermediate Representation* | Consistência com a especificação |
| Hash | SHA-256 com **separação de domínio** (`H(tag ‖ bytes)`, tag por tipo de objeto) | Interoperável, padrão em logs de transparência |
| Assinatura | Ed25519 (`ed25519-dalek`) para kernel, gateway, atestadores e admin. `Signature` e `KeyId` carregam o **identificador do algoritmo** desde a v1; ES256 (P-256) entra na 9.2 para passkeys do usuário | Determinística, pequena, sem parâmetros. Passkeys/WebAuthn assinam quase sempre com ES256 (§6): fixar Ed25519 no tipo quebraria a 9.2 |
| Serialização canônica | Subconjunto de CBOR determinístico (RFC 8949 §4.2.1), implementado no próprio crate `logos-canon`, com **decoder estrito** (rejeita qualquer codificação não canônica) | O que é hasheado é exatamente o que é checado (P2) |
| Lógica base | `FOL-ms-eq/v1`: primeira ordem clássica, multi-sortida, com igualdade | §9 |
| Armazenamento | SQLite (via `rusqlite`) para ledger e índices; CAS em disco endereçado por hash | §12 |
| Transporte agente↔kernel | Unix domain socket, frames `u32 len ‖ CBOR canônico`; JSON só em ferramentas de debug | Sem rede por padrão; isolamento por SO |
| Formato de token | Struct canônica assinada pelo kernel (v1). Biscuit é opção futura para atenuação offline. **Diverge da especificação** (§6, §12 sugerem Biscuit); a divergência fica registrada no ADR 0001 | Não inventar criptografia: só assinatura sobre bytes canônicos. A v1 não precisa de atenuação offline, e o Biscuit traria Datalog próprio e mais dependências para a TCB |
| Modelo formal | Lean 4 | Mesmo ecossistema da camada 3; comunidade ativa |
| Verificação de Rust | Kani para invariantes locais; Verus para módulos críticos (gerador de obrigações, appender, gateway) | §5 |
| Licença | Definir na Etapa 0.1 (sugestão: Apache-2.0 / MIT dual) | — |

---

## 2. Mapa de componentes e crates

```text
logos/
├── crates/                         # Rust
│   ├── logos-canon      ⚠ TCB     CBOR canônico, encode/decode estrito
│   ├── logos-crypto     ⚠ TCB     hash com domínio, assinatura com algoritmo identificado, KeyId
│   ├── logos-core       ⚠ TCB     CheckOutcome, Verdict, Budget, StepMeter (vocabulário comum dos checkers)
│   ├── logos-air        ⚠ TCB     sorts, termos, fórmulas, vocabulário, sort-check, normalização
│   ├── logos-obligation ⚠ TCB     gerador de obrigações canônicas
│   ├── logos-model      ⚠ TCB     modelos finitos + avaliador (camada 0)
│   ├── logos-ground     ⚠ TCB     grounder + Tseitin + mapa cláusula→premissa
│   ├── logos-lrat       ⚠ TCB     checker LRAT próprio + wrapper do cake_lpr
│   ├── logos-alethe       (wrap)   integração Carcara (camada 2)
│   ├── logos-ledger     ⚠ TCB     eventos, hash chain, assinatura, SQLite, CAS
│   ├── logos-seal       ⚠ TCB     selo de cinco campos + renderizador (TCB de fidelidade)
│   ├── logos-policy     ⚠ TCB     linguagem de política, Merkle, avaliador direto
│   ├── logos-token      ⚠ TCB     mint/verify de tokens de ação
│   ├── logos-kernel     ⚠ TCB     binário: máquina de estados, protocolo, registry de checkers
│   ├── logos-gateway    ⚠ TCB     binário: adaptadores, credenciais, execução condicional
│   ├── logos-attest       (fid.)   SDK de atestadores + atestadores de referência
│   ├── logos-audit        (indep.) verificador independente do ledger (não linka logos-kernel)
│   ├── logos-cli                   CLI de operação (retract, inspect, override…)
│   └── logos-tui        (fora)     interface de terminal (Ratatui); só exibe, não assina
├── py/                             # Python, tudo NÃO confiável
│   ├── logos_client                cliente do protocolo do kernel
│   ├── logos_provers               prover farm: z3, cvc5, cadical, vampire, model finders
│   ├── logos_mcp                   servidor MCP: propose / certify / action_intent / ledger
│   ├── logos_agent                 harness do agente (loop LLM, formalizador, planner)
│   └── logos_eval                  AgentDojo, benchmarks, estudos
├── formal/                         # Lean 4: modelo da máquina de estados + teoremas
├── spec/                           # especificações normativas (encoding, domínios, CAS, eventos, protocolo, AIR)
├── docs/spec/                      # especificação conceitual (citada como §N)
├── docs/adr/                       # decisões de arquitetura
├── tcb/                            # manifesto da TCB + script de contagem
└── bench/                          # benchmarks de desempenho e tamanho de certificado
```

`logos-audit` **não pode depender** de `logos-kernel` nem de `logos-ledger`: ele reimplementa a verificação a partir de `spec/`. Pode depender de `logos-canon`, `logos-crypto` e `logos-core` (primitivas sem estado), mas o hash de CAS, os domínios e o formato de eventos precisam estar em `spec/` para que uma reimplementação em outra linguagem seja possível. Isso é o que torna o recheck por terceiros real (§4.9).

---

## 3. Invariantes globais (testadas desde o início)

Os oito princípios da §12, numerados para rastreio. Cada um ganha um teste de regressão permanente (`tests/invariants/`) na primeira etapa em que se torna testável.

| ID | Princípio | Primeiro teste em |
| --- | --- | --- |
| **P1** | O agente propõe, cita e prova; nunca escreve obrigações, fatos ou políticas | 5.3 |
| **P2** | O checker consome exatamente os bytes que o ledger hasheia | 2.3 / 4.4 |
| **P3** | Todo veredicto que autoriza algo passa por um checker; solvers só aceleram | 5.4 |
| **P4** | Refutações são testemunhas; provas são certificados; `UNKNOWN` não autoriza nada | 5.4 |
| **P5** | Nenhum efeito é especulativo | 8.5 |
| **P6** | O termo de uma ação vem dos bytes da chamada, nunca da descrição | 8.2 |
| **P7** | Nenhum selo de validade é renderizável sem fidelidade, consistência e premissas em aberto | 15.1 |
| **P8** | Confiança em outro agente é premissa explícita | 17.4 |

Mais quatro invariantes de sistema:

| ID | Invariante | Primeiro teste em |
| --- | --- | --- |
| **S1** | Toda entrada no ledger é verificável por `logos-audit` sem acesso ao kernel | 3.6 |
| **S2** | Nenhuma resposta de timeout/limite é aceite ou rejeição: é `UNKNOWN` com limites registrados | 2.2 |
| **S3** | O mesmo input com os mesmos limites produz o mesmo resultado em qualquer máquina (determinismo) | 2.2 |
| **S4** | Uma obrigação refutada não pode ser re-proposta (mesmo `h(O)`) | 5.6 |

---

## 4. Definition of Done global

Vale para **toda** etapa, além do critério específico dela:

- [ ] Código compila sem warnings (`cargo clippy -- -D warnings`; `ruff` + `mypy --strict` no Python).
- [ ] Testes da etapa passam em CI; nenhum teste anterior regrediu.
- [ ] Documentação: comentários de módulo + atualização em `spec/` se o formato ou o protocolo mudou.
- [ ] Se tocou a TCB (`⚠ TCB`):
  - [ ] `#![forbid(unsafe_code)]`, sem `unwrap()`/`expect()`/`panic!` fora de testes (lint `clippy::unwrap_used`, `clippy::panic`);
  - [ ] nova dependência justificada em ADR e adicionada a `deny.toml`;
  - [ ] fuzz target (`cargo fuzz`) para todo parser/decoder novo, rodado ≥ 1h sem crash (no job noturno de fuzz da CI, com o log arquivado como artefato e linkado no PR; no PR roda um smoke de poucos minutos);
  - [ ] `tcb/manifest.toml` atualizado e `tcb/count.sh` rodado (LOC confiável reportado no PR).
- [ ] Se mudou um formato serializado: vetores de teste (golden files) em `spec/vectors/` e bump de versão do formato.
- [ ] Se mudou semântica de evento: entrada no Apêndice C e teste em `logos-audit`.

---

## 5. Visão geral das fases

| Fase | Nome | Regime | Marco | Tamanho |
| --- | --- | --- | --- | --- |
| 0 | Fundação | — | Repositório, canon, crypto, CAS | M |
| 1 | AIR e obrigações | ambos | Obrigação canônica hasheada | G |
| 2 | Camada 0: modelos finitos | ambos | ★ `REFUTED` checado | M |
| 3 | Ledger assinado | ambos | ★ Ledger auditável por terceiros | M |
| 4 | Camada 1: LRAT | operacional | ★ `VERIFIED` com checker verificado | G |
| 5 | Kernel | ambos | ★ Kernel mínimo (fim da ordem de construção §12.1) | G |
| 6 | Atestadores | operacional | Fatos assinados | M |
| 7 | Políticas | operacional | Políticas com Merkle e trajetórias | G |
| 8 | Ações e gateway | operacional | ★ Primeiro efeito mediado por prova | G |
| 9 | Paráfrase e autorização do usuário | operacional | Usuário assina o termo formal | M |
| 10 | Harness do agente | ambos | ★ Agente LLM ponta a ponta | G |
| 11 | Integridade de justificativa | operacional | ★★ Primeiro paper (segurança) | GG |
| 12 | Modelo formal | — | Teoremas de mediação e justificativa | GG (∥) |
| 13 | Hardening | — | Chaves, transparência, Verus | G |
| 14 | Camada 2: Alethe | epistêmico | FOL certificada | G |
| 15 | Selo e fidelidade | epistêmico | ★ Selo de cinco campos | G |
| 16 | Benchmark epistêmico e estudo | epistêmico | Estudo com usuários | GG |
| 17 | PVCs | multi-agente | Claims portáveis | G |
| 18 | Debate checkável | epistêmico | ★★ Segundo paper | GG |
| 19 | ASPIC+ *(fora do horizonte)* | epistêmico | Argumentação certificada | G |
| 20 | Lean *(fora do horizonte)* | matemática | Camada 3 | G |
| 21 | HOL / modal *(fora do horizonte)* | modal, deôntica | Camada 4 | GG |
| 22 | Framework único *(fora do horizonte)* | todos | Pesquisa | GG |
| 23 | Produto | — | Binário único, release | G |

Caminho crítico até o primeiro paper: **0 → 1 → 2 → 3 → 4 → 5 → 6 → 7 → 8 → 9 → 10 → 11**, com a 12 em paralelo a partir da 5 e a 13 parcialmente antes da 11 (o mínimo para a avaliação ser crível).

**Horizonte do primeiro ciclo.** As Fases 19–22 ficam fora do horizonte até o fim da Fase 16 (decisão antecipada do último risco do Apêndice E). Não gerar planos de etapa para elas antes disso; os planos de fase podem existir só como esboço.

---

## Fase 0 — Fundação

Objetivo da fase: ter as primitivas sobre as quais tudo é hasheado e assinado, e um repositório onde a TCB é medida desde o primeiro commit.

Ordem recomendada: 0.1 → 0.2 → 0.3 → 0.4 → 0.5, com 0.6 iniciada após a 0.1 e fechada após a 0.3, e 0.7 em paralelo desde o início. Plano de fase em `docs/plans/fase-0.md`.

### Etapa 0.1 — Repositório, workspace e CI · `P`

**Depende de (recebe pronto).** nenhuma (primeira etapa).

**Fora do escopo.** código funcional em qualquer crate; publicação de pacotes; CI multi-arquitetura (entra na 0.6).

**Objetivo.** Esqueleto do monorepo com as regras de qualidade já ativas.

**Entregáveis.**
- Cargo workspace com os crates vazios da §2 do roadmap (só `lib.rs` com `#![forbid(unsafe_code)]` nos de TCB), incluindo `logos-core`.
- Projeto Python (`py/`, `pyproject.toml`, `uv`), `ruff`, `mypy --strict`, `pytest`.
- CI (GitHub Actions): build, clippy, testes Rust e Python, `cargo deny`, `cargo audit`. Job **noturno** separado para fuzz longo (≥ 1h por target, log arquivado como artefato); no PR, só smoke de fuzz.
- `deny.toml` com allowlist de licenças e de crates.
- `docs/adr/0001-decisoes-iniciais.md` registrando a tabela da §1 do roadmap **e as divergências deliberadas em relação à especificação**: formato de token próprio na v1 em vez de Biscuit (§6, §12); crate `logos-core` (ausente da especificação); assinatura com algoritmo identificado.
- `docs/adr/0002-divisao-de-linguagens.md`: TCB e TUI em Rust; agente, MCP, prover farm e avaliação em Python; alternativa considerada ("tudo em Rust") e o que mudaria.
- Workspace Rust já com `logos-tui` (vazio, fora da TCB).
- `LICENSE` (Apache-2.0 / MIT dual, salvo decisão contrária no ADR 0001), `README.md` com a tese em 5 linhas — **a da §13** (integridade de justificativa), apoiada na formulação da §1 — e links para este roadmap e para `docs/spec/`.

**Critério de conclusão.**
- [ ] `cargo build --workspace` e `pytest` passam em CI limpo.
- [ ] Um PR de teste com `unsafe {}` num crate de TCB é bloqueado pela CI.
- [ ] Um PR de teste adicionando uma dependência fora da allowlist é bloqueado.
- [ ] O job noturno de fuzz existe e roda (mesmo sem targets reais ainda: um target trivial de exemplo).

---

### Etapa 0.2 — Manifesto e contagem da TCB · `P`

**Depende de (recebe pronto).** 0.1 — workspace e CI.

**Fora do escopo.** contagem de binários externos (entram quando registrados, ex.: 4.5); verificação formal (12.4).

**Objetivo.** Medir a TCB desde o dia zero. É insumo da avaliação do primeiro paper (11.9; §11, item 2 da tese em três partes, mantido após a reorientação da §13). Fazer antes da 0.3, que é o primeiro código de TCB.

**Entregáveis.**
- `tcb/manifest.toml`: para cada propriedade da §5 (correção, fidelidade, progresso), a lista de crates/arquivos/binários externos que pertencem a ela, com o hash do binário quando externo (hash de bytes crus, ver 0.4).
- `tcb/count.sh` (ou `cargo xtask tcb`): LOC por crate (via `tokei`), LOC das dependências transitivas da TCB, e quais são verificadas (campo `verified = "kani" | "verus" | "cakeml" | none`).
- Saída em JSON versionada por commit (`tcb/history/`).

**Critério de conclusão.**
- [ ] `cargo xtask tcb` produz um relatório JSON e uma tabela Markdown.
- [ ] CI comenta no PR a diferença de LOC da TCB em relação a `main`.
- [ ] Adicionar um arquivo novo num crate de TCB sem atualizar o manifesto falha a CI.

---

### Etapa 0.3 — Codificação canônica (`logos-canon`) · `M` · ⚠ TCB

**Depende de (recebe pronto).** 0.1 — workspace, CI, projeto Python; 0.2 — manifesto (o crate nasce listado).

**Fora do escopo.** floats, tags CBOR (inclusive bignums), comprimentos indefinidos, streaming; otimização de desempenho.

**Objetivo.** Uma única forma de transformar qualquer objeto do LOGOS em bytes, e de recusar qualquer outra.

**Entregáveis.**
- Subconjunto de CBOR determinístico: inteiros (menor codificação; faixa dos tipos maiores 0 e 1, `[-2^64, 2^64-1]`), bytes, strings UTF-8 NFC, arrays, mapas com chaves ordenadas pela ordem lexicográfica dos bytes codificados (RFC 8949 §4.2.1), `bool`, `null`. **Sem** floats, tags arbitrárias, comprimentos indefinidos.
- **Identificador de versão da codificação** (`canon/v1`), definido em `spec/encoding.md` e exposto como constante. É o “versão do encoder” que a obrigação carrega (§4.1); qualquer mudança de regra é `canon/v2`.
- **Versão de Unicode fixada** em `spec/encoding.md` para a checagem NFC, com a regra para code points não atribuídos nessa versão (padrão: rejeitar). Rust e Python precisam usar a mesma tabela; o teste diferencial confere a versão.
- Traits `Canon` (encode) e `CanonDecode` (decode estrito) + derive macro **ou** implementação manual (preferir manual na TCB; macro só se pequena e auditável).
- Decoder estrito: rejeita mapas fora de ordem, chaves duplicadas, inteiros não mínimos, UTF-8 inválido ou não NFC, code points não atribuídos, bytes sobrando.
- `spec/encoding.md`: especificação normativa (alguém deve poder reimplementar em outra linguagem só lendo).
- Implementação de referência em Python (`py/logos_client/canon.py`) — fora da TCB, usada para teste diferencial.

**Critério de conclusão.**
- [ ] Property test: `decode(encode(x)) == x` para objetos aleatórios.
- [ ] Property test: para toda sequência de bytes `b` aceita, `encode(decode(b)) == b` (unicidade da codificação).
- [ ] Teste diferencial Rust × Python sobre 100k objetos aleatórios (incluindo strings com caracteres fora do ASCII e combinantes): bytes idênticos e mesmas rejeições.
- [ ] Teste: as duas implementações reportam a mesma versão de Unicode, igual à de `spec/encoding.md`.
- [ ] Fuzz do decoder ≥ 1h sem crash nem aceitação de codificação não canônica.
- [ ] ≥ 30 vetores de teste em `spec/vectors/canon/` incluindo casos de rejeição.

---

### Etapa 0.4 — Criptografia (`logos-crypto`) · `P` · ⚠ TCB

**Depende de (recebe pronto).** 0.3 — traits `Canon` / `CanonDecode`.

**Fora do escopo.** HSM/TEE e rotação de chaves (13.1); passkey/WebAuthn e a implementação de ES256 (9.2); tipos concretos de domínio (`Formula`, `Obligation`… nascem nas fases 1, 3 e seguintes).

**Objetivo.** Hash com separação de domínio e assinaturas, com tipos que impeçam misturar domínios, sem fechar a porta para outros algoritmos de assinatura e para chaves fora de arquivo.

**Entregáveis.**
- Mecanismo de domínio: `trait Domain { const TAG: &'static [u8]; }` e `Hash<T: Domain>`, tipo fantasma. `H_T(x) = SHA-256(tag_T ‖ canon(x))`. As tags de todos os domínios previstos (`formula`, `obligation`, `event`, `model`, `cert`…) são **reservadas** em `spec/domains.md` já aqui; os tipos que as usam nascem nas etapas que os definem.
- **Hash de bytes crus**: `hash_raw::<T>(bytes) -> Hash<T> = SHA-256(tag_T ‖ bytes)`, para objetos que não são CBOR (binários de checker `h(checker)` §4.4/§4.6, certificados LRAT/Alethe, modelos vindos de provers). `spec/domains.md` diz, por domínio, se o objeto é canônico ou cru.
- **Digest externo** sem tag (`ExternalDigest = SHA-256(bytes)`), usado só para conferir binários de terceiros contra o hash que eles publicam (ex.: `cake_lpr`, 4.5) e no manifesto da TCB. Nunca é aceito onde se espera `Hash<T>`.
- `KeyId = H("key" ‖ alg ‖ pubkey)`; `PublicKey` e `Signature` carregam `alg` (`ed25519` na v1; o enum já reserva `es256`). Verificar com `alg` desconhecido é erro tipado, nunca sucesso.
- `trait Signer { fn key_id(&self) -> KeyId; fn sign<D: SigDomain>(&self, bytes: &[u8]) -> Result<Signature, SignError>; }`, com uma implementação por arquivo (`FileSigner`). O HSM da 13.1 é outra implementação da mesma trait.
- `verify<D: SigDomain>(bytes, sig, pubkey)` — a assinatura também tem domínio (`"logos/event/v1"`, `"logos/token/v1"`…), para que uma assinatura de evento nunca valha como token.
- Carregamento de chave a partir de arquivo com permissão `0600` (recusa se mais aberto).

**Critério de conclusão.**
- [ ] Teste de tipo (compile-fail com `trybuild`, sobre dois domínios marcadores de teste): passar `Hash<A>` onde se espera `Hash<B>` não compila; passar `ExternalDigest` onde se espera `Hash<T>` não compila.
- [ ] Teste: assinatura com domínio `event` não verifica com domínio `token`.
- [ ] Teste: `hash::<T>(x)` e `hash_raw::<T>(canon(x))` coincidem, e `hash_raw` com tags diferentes nunca coincide (vetores em `spec/vectors/domains/`).
- [ ] Teste: assinatura com `alg` trocado no envelope não verifica; `alg` desconhecido dá erro tipado.
- [ ] Vetores de teste Ed25519 do RFC 8032 passam.
- [ ] Chave com permissão `0644` é recusada.

---

### Etapa 0.5 — Armazenamento endereçado por conteúdo (CAS) · `P` · ⚠ TCB

**Depende de (recebe pronto).** 0.4 — `Hash<T>`, `hash_raw`, `spec/domains.md`.

**Fora do escopo.** coleta de lixo (3.5); replicação; compressão; implementação de leitura/escrita em streaming (só a forma da API).

**Objetivo.** Guardar certificados, modelos e obrigações por hash; o ledger só guarda hashes. Terceiros (auditor, outros kernels via PVC, §7) precisam conseguir recalcular o endereço sem o código do LOGOS.

**Entregáveis.**
- `Cas::put::<T>(bytes) -> Result<Hash<T>, CasError>`, `Cas::get::<T>(&Hash<T>) -> Result<Option<Vec<u8>>, CasError>`, verificação do hash na leitura (arquivo corrompido = erro, nunca dado errado). A API aceita também `impl Read` (`put_reader`), mesmo que a v1 leia tudo em memória: certificados podem chegar a gigabytes (§4) e a troca para streaming não pode mudar a assinatura.
- `CasError { Integrity, TooLarge { limit, actual }, Io }`. `TooLarge` é o que o kernel converte em `Unknown(Size)` (0.6), nunca em rejeição.
- Layout em disco `cas/<domínio>/ab/cdef…`, escrita atômica (tmp + rename + fsync).
- `spec/cas.md`: regra de endereçamento (domínio, tag, canônico × cru), layout e limites, normativa para o `logos-audit` (S1) e para PVCs.
- Limite de tamanho por domínio (configurável; padrão: certificado ≤ 256 MiB).

**Critério de conclusão.**
- [ ] Corromper 1 byte de um objeto faz `get` retornar erro de integridade.
- [ ] `put` concorrente do mesmo objeto por 16 threads não corrompe nada.
- [ ] Objeto acima do limite é recusado com erro tipado `TooLarge`.
- [ ] Teste: o endereço de 3 objetos de exemplo calculado só a partir de `spec/cas.md` (script Python independente) coincide com o do `Cas`.

---

### Etapa 0.6 — Veredictos, orçamentos e erros (`logos-core`) · `P` · ⚠ TCB

**Depende de (recebe pronto).** 0.1 — workspace; 0.3 — `Canon` (os tipos desta etapa entram nos eventos: §10 ameaça 18, Apêndice C `Verified.budget`). CI com runner aarch64 (adicionar aqui se ainda não existir). Pode começar logo após a 0.1; fecha depois da 0.3.

**Fora do escopo.** checkers concretos (Fases 2 e 4); política de tentativas após `Unknown` (5.4); estados `Pending`/`Invalidated` do claim (5.1).

**Objetivo.** Vocabulário comum de resultados para todos os checkers, com S2 e S3 embutidos no tipo.

**Entregáveis.**
- `enum CheckOutcome { Accepted, Rejected(Reason), Unknown(Exhausted) }` onde `Exhausted ∈ {Time, Steps, Memory, Size}`.
- `Budget { steps: u64, mem_bytes: u64, size_bytes: u64, wall_ms: u64 }`. **Determinismo:** checkers internos contam *passos* (determinístico); `wall_ms` só é usado para checkers externos e, quando dispara, produz `Unknown(Time)` registrado com a máquina.
- `StepMeter`: contador de passos compartilhado que todo checker interno usa; estourar o orçamento devolve `Unknown(Steps)` por construção, sem depender de cada checker tratar o caso.
- `enum Verdict { Verified, Refuted, Unknown }` no nível do claim.
- `Canon`/`CanonDecode` para `CheckOutcome`, `Exhausted`, `Budget` e `Verdict`, com vetores em `spec/vectors/core/`.
- Regras codificadas (sem `From`, sem `match` que as viole — revisado):
  - `Unknown` não tem conversão para `Verified` ou `Refuted`;
  - **`Rejected` não tem conversão para `Refuted`**: certificado rejeitado não é refutação, a obrigação volta para `Pending` (§4.4). Só um contramodelo checado produz `Refuted`. Sem isso, um prover que envia lixo dispararia o bloqueio S4 sobre uma obrigação possivelmente verdadeira.

**Critério de conclusão.**
- [ ] Teste: estourar `steps` num checker de referência de teste (construído sobre `StepMeter`) sempre dá `Unknown(Steps)`, nunca `Rejected` nem `Accepted` (S2). O teste vira regressão obrigatória dos checkers reais (2.2, 4.4).
- [ ] Teste: o mesmo input com o mesmo `Budget` (sem `wall_ms`) dá o mesmo outcome em 1000 execuções e em 2 arquiteturas da CI (x86_64, aarch64) (S3).
- [ ] Teste compile-fail (`trybuild`): não existe caminho de `CheckOutcome::Rejected` ou `Unknown` para `Verdict::Refuted`/`Verified` fora das funções nomeadas de construção.

---

### Etapa 0.7 — Documento de ameaças e escopo · `P`

**Depende de (recebe pronto).** nenhuma técnica (∥ com 0.1–0.6); usa a especificação conceitual (§1, §5, §6, §10, §13).

**Fora do escopo.** mitigação das ameaças (só o mapeamento); modelo formal (Fase 12).

**Objetivo.** Fixar por escrito o que o sistema promete, o que não promete, e os limites L1–L3 (§1), antes de escrever o kernel.

**Entregáveis.**
- `docs/threat-model.md`: atacantes (LLM comprometido por injeção, agente malicioso, prover malicioso — inclusive enviando certificados inválidos ou gigantes —, atestador comprometido, **operador ou kernel que mostra histórias diferentes a partes diferentes** (ameaça 9; inclui o operador curioso), outro agente na rede), ativos (chave do kernel, credenciais do gateway, ledger), o que está fora do escopo (compilador, SO, hardware).
- `docs/guarantees.md`: o enunciado da tese (§1), da garantia do gateway (§6) e da integridade de justificativa (§13), cada um com a lista de premissas de que depende e a TCB por propriedade (§5).
- `docs/limits.md`: L1, L2, L3, as ameaças fundamentais (§10: 1, 2, 3, 13, 16) e a parcialmente fundamental (§10: 10, TOCTOU para fatos sem versão atômica e fatos do mundo físico, tabela da §6).

**Critério de conclusão.**
- [ ] Os três documentos existem e cada garantia lista explicitamente sua TCB.
- [ ] Cada ameaça da tabela da §10 aparece no Apêndice B deste roadmap com a etapa que a mitiga (ou "fundamental — só declarada").
- [ ] Cada atacante de `threat-model.md` aponta para ao menos uma ameaça da §10, e cada ameaça aponta para ao menos um atacante.

---

## Fase 1 — AIR: lógica, vocabulário e obrigações

Objetivo da fase: o kernel consegue receber uma proposta, checar que ela é bem formada, e produzir `O = canon(Γ ⊢ φ)` e `h(O)` de forma determinística. AIR (*Argument Intermediate Representation*) é a linguagem formal do LOGOS.

### Etapa 1.1 — Sintaxe abstrata da lógica · `M` · ⚠ TCB

**Depende de (recebe pronto).** 0.3 — `Canon`; 0.4 — `Hash<Formula>`.

**Fora do escopo.** vocabulário (1.2); sort-check (1.3); sintaxe textual (1.4); operador `Says` (6.6); lógicas não clássicas (Fases 19–21).

**Objetivo.** Tipos Rust para `FOL-ms-eq/v1`.

**Entregáveis.**
- `Sort`, `Symbol { name, kind: Const|Func|Pred, arity: [Sort], result: Sort|Bool }`.
- `Term ::= Var(idx, sort) | App(sym, [Term])`.
- `Formula ::= True | False | Eq(t,t) | Pred(sym,[t]) | Not | And([..]) | Or([..]) | Implies | Iff | Forall(sort, body) | Exists(sort, body)`.
- **Variáveis em índices de De Bruijn** na representação canônica: alfa-equivalência vira igualdade de bytes.
- `LogicId("FOL-ms-eq/v1")` presente em todo objeto que carrega fórmulas.
- Limites estruturais: profundidade máxima, número de nós (configurável; rejeita acima).

**Critério de conclusão.**
- [ ] `∀x.P(x)` e `∀y.P(y)` têm a mesma codificação canônica e o mesmo hash.
- [ ] Fórmula com profundidade acima do limite é rejeitada sem estourar a pilha (teste com profundidade 10⁶).
- [ ] Codificação canônica de fórmulas especificada em `spec/air.md` com vetores.

---

### Etapa 1.2 — Vocabulário versionado com grounding · `M` · ⚠ TCB (fidelidade parcial)

**Depende de (recebe pronto).** 1.1 — `Sort`, `Symbol`, `Formula`.

**Fora do escopo.** fatos que fecham domínios (6.3): aqui `domain_source` é uma referência opaca; invalidação por mudança de vocabulário (15.7); tradução entre vocabulários.

**Objetivo.** Todo símbolo usado numa fórmula está declarado num vocabulário com hash, e cada símbolo tem um status de ancoragem (§8).

**Entregáveis.**
- `Vocabulary { version, sorts: [SortDecl], symbols: [SymbolDecl] }` com `h(V)`.
- `SortDecl { name, domain: Open | Enumerated([const]) }` — `Enumerated` é o que habilita grounding (Fase 4). **A enumeração do domínio é ela mesma uma premissa** (fecho de domínio) e precisa vir de um fato atestado ou de uma política (registrar isso no tipo: `domain_source: FactRef | PolicyRef`).
- `SymbolDecl { name, signature, grounding: Defined(Formula) | Anchored(AttestorId, procedure) | Glossed(String), gloss_pt: String }`.
- Símbolos `Defined` têm definição verificada como não circular (ordem topológica).
- Operação `Vocabulary::extend` (gera nova versão com novo hash); mudança de significado de símbolo existente é **nova versão incompatível** (dispara invalidação na Etapa 15.7).

**Critério de conclusão.**
- [ ] Definição circular (`A ↔ B`, `B ↔ A`) é rejeitada.
- [ ] Extensão que só adiciona símbolos preserva a compatibilidade; alterar a glosa ou a definição de símbolo existente gera versão marcada como incompatível.
- [ ] Vetores de teste com `h(V)` fixo para um vocabulário de exemplo.

---

### Etapa 1.3 — Sort-checker · `P` · ⚠ TCB

**Depende de (recebe pronto).** 1.1 — sintaxe abstrata; 1.2 — `Vocabulary`.

**Fora do escopo.** inferência de sorts; mensagens em português (só o caminho até o nó).

**Objetivo.** Recusar fórmula mal tipada antes de gerar obrigação.

**Entregáveis.**
- `check(V, formula) -> Result<(), SortError>`: aridade, sorts dos argumentos, igualdade só entre termos do mesmo sort, variáveis ligadas, símbolos declarados em `V`.
- Erros com caminho até o nó problemático (para o agente corrigir).

**Critério de conclusão.**
- [ ] 100% de cobertura de ramos no checker.
- [ ] Property test: fórmulas geradas aleatoriamente *bem tipadas* sempre passam; mutações que quebram tipo sempre falham.

---

### Etapa 1.4 — Sintaxe de superfície e parser (fora da TCB) · `M`

**Depende de (recebe pronto).** 1.1–1.3; 0.3 — encoder canônico Python.

**Fora do escopo.** parser na TCB (por decisão, ele fica fora); sintaxe de políticas (7.1); editor ou LSP.

**Objetivo.** Humanos e o LLM escrevem AIR em texto; o parser produz a forma abstrata. **O parser não está na TCB de correção**: o kernel recebe a forma canônica binária e o hash é sobre ela. Um erro de parser produz uma fórmula *diferente*, o que é problema de fidelidade, não de correção — e por isso a Etapa 9.1 mostra a paráfrase da forma canônica, não o texto digitado.

**Entregáveis.**
- Gramática em `spec/air-surface.md`. Exemplo:
  ```text
  vocab v1 {
    sort User = enum from fact F_users
    sort Db
    pred Admin(User)
    pred Owns(User, Db)
    pred Allowed(Action)
  }
  claim C7: forall u:User. Admin(u) -> exists d:Db. Owns(u, d)
    cites [F3, P2, C5]
  ```
- Parser em Python (`logos_client.air`) e em Rust (crate separado `logos-air-text`, fora da TCB).
- Pretty-printer reverso (forma canônica → texto) para debug.

**Critério de conclusão.**
- [ ] Round-trip `parse(print(f)) == f` em property test, nos dois parsers.
- [ ] Teste diferencial: parser Python e Rust produzem bytes canônicos idênticos para um corpus de 500 fórmulas.

---

### Etapa 1.5 — Gerador de obrigações (`logos-obligation`) · `M` · ⚠ TCB

**Depende de (recebe pronto).** 1.1–1.3. Define aqui a interface abstrata `LedgerView` (implementação real na 3.3; nos testes, um fake em memória).

**Fora do escopo.** emissão para CNF/SMT/Lean (4.3, 14.1, 20.1); especulação (5.8) — aqui só a recusa de `Pending`.

**Objetivo.** O coração de P1: só o kernel produz obrigações, e de forma determinística.

**Entregáveis.**
- `Obligation { logic, encoder_version, vocab: h(V), premises: [(PremiseRef, Formula)], goal: Formula }`.
  - `premises` ordenadas por `h(PremiseRef)`, sem duplicatas.
  - `PremiseRef` é o hash do evento do ledger que torna a premissa citável (Fact, Policy, Verified, Assumption). A fórmula vai junto para o checker não precisar resolver referências (P2).
  - Época **não** faz parte da obrigação: a obrigação é um objeto lógico puro; a época vai no evento.
- `generate(proposal, ledger_view) -> Result<Obligation, Reject>`:
  - resolve cada citação no `ledger_view` (interface abstrata nesta etapa; implementação real na Fase 3);
  - recusa citação inexistente, retratada, de época inválida ou não citável (regra de citação, §3);
  - sort-check do objetivo e das premissas contra `V`;
  - recusa vocabulário diferente entre premissas sem tradução declarada (v1: exige o mesmo `h(V)` ou vocabulário estendido compatível).
- `h(O) = H_obligation(canon(O))`.

**Critério de conclusão.**
- [ ] Mesma proposta com citações em ordem diferente → mesmo `h(O)`.
- [ ] Citar um hash inexistente → `Reject::UnknownCitation`; citar algo `Pending` sem marcar especulação → `Reject::NotCitable`.
- [ ] A API pública não tem nenhuma função que aceite um `Obligation` vindo de fora (só `generate`); teste compile-fail.
- [ ] Vetores de teste: 20 obrigações com `h(O)` fixo em `spec/vectors/obligation/`.

---

### Etapa 1.6 — Formato da proposta (protocolo) · `P`

**Depende de (recebe pronto).** 1.1 — `Formula`; 1.5 — `PremiseRef`.

**Fora do escopo.** transporte e framing (5.2); mensagens além de `Proposal` (5.2, 8.2).

**Objetivo.** Definir o que o agente pode mandar — e, por construção, o que ele **não** pode.

**Entregáveis.**
- `Proposal { text_pt: String, goal: Formula, cites: [PremiseRef], speculative_on: [ClaimId], vocab: h(V) }`.
- **Não existe** campo para obrigação, fato, política, dependência ou status. `spec/protocol.md` lista isso explicitamente.
- `text_pt` é armazenado como dado não confiável (taint), nunca entra na obrigação.

**Critério de conclusão.**
- [ ] Decoder da proposta rejeita campos desconhecidos (um agente não pode "contrabandear" uma obrigação).
- [ ] Teste de invariante P1 preliminar: nenhum tipo de mensagem do agente contém `Obligation`, `Fact`, `Policy` ou `Signature` do kernel (verificado por teste que inspeciona o schema).

---

## Fase 2 — Camada 0: modelos finitos, refutação e consistência

Objetivo da fase: o checker mais simples do sistema (§9). Com ele, `REFUTED` e testemunhas de consistência já funcionam, e Z3 entra como produtor não confiável.

### Etapa 2.1 — Estruturas finitas · `P` · ⚠ TCB

**Depende de (recebe pronto).** 1.1, 1.2 — sintaxe e vocabulário.

**Fora do escopo.** modelos infinitos; tabelas para símbolos `Defined` (são expandidos na 2.2).

**Entregáveis.**
- `FiniteModel { vocab: h(V), domains: Map<Sort, u32 /*tamanho*/>, consts, funcs: tabelas totais, preds: conjuntos de tuplas }`.
- Validação: funções totais, valores dentro do domínio, sorts `Enumerated` têm exatamente os elementos declarados.
- Codificação canônica e limite de tamanho (tabelas grandes = `Unknown(Size)`).

**Critério de conclusão.**
- [ ] Modelo com função parcial ou valor fora do domínio é rejeitado.
- [ ] Vetores canônicos em `spec/vectors/model/`.

---

### Etapa 2.2 — Avaliador `M ⊨ φ` · `M` · ⚠ TCB

**Depende de (recebe pronto).** 2.1 — `FiniteModel`; 0.6 — `Budget`, `Exhausted`.

**Fora do escopo.** otimizações; avaliação incremental.

**Entregáveis.**
- `eval(M, φ, budget) -> Result<bool, Exhausted>`, com contador de passos (quantificadores multiplicam passos pelo tamanho do domínio).
- Implementação deliberadamente ingênua e curta (alvo: < 400 LOC). Clareza > desempenho.
- Avaliação de `Defined` símbolos por expansão da definição.

**Critério de conclusão.**
- [ ] Teste diferencial contra Z3 (`(eval φ)` num modelo exportado) em 10k pares (M, φ) aleatórios: 0 divergências.
- [ ] Teste de orçamento: S2 e S3 (da Etapa 0.6) valem para o avaliador.
- [ ] LOC do crate reportado no manifesto da TCB.

---

### Etapa 2.3 — Checker de refutação e de consistência · `P` · ⚠ TCB

**Depende de (recebe pronto).** 2.2 — `eval`; 1.5 — `Obligation`; 0.5 — CAS.

**Fora do escopo.** registro no ledger (Fase 3); roteamento por formato (5.4).

**Entregáveis.**
- `check_refutation(O, M) -> CheckOutcome`: aceita sse `M ⊨ p` para toda premissa de `O` e `M ⊭ goal`.
- `check_consistency(premises, M0) -> CheckOutcome`: aceita sse `M0 ⊨ p` para todo `p`.
- Ambos consomem o `Obligation` decodificado **dos mesmos bytes** que foram hasheados (P2): a API recebe `(bytes, h)` e recusa se `H(bytes) ≠ h`.

**Critério de conclusão.**
- [ ] Teste de P2: passar bytes de uma obrigação com hash de outra → recusa.
- [ ] Mutation testing (`cargo mutants`) no checker: 100% dos mutantes mortos.

---

### Etapa 2.4 — Produtor Z3 de modelos (fora da TCB) · `M`

**Depende de (recebe pronto).** 2.1 — formato de `FiniteModel`; 1.5 — formato de `O`; 0.3 — encoder canônico Python.

**Fora do escopo.** provas de `VERIFIED`; integração ao prover farm (10.2).

**Entregáveis.**
- `logos_provers.z3_models`: traduz `O` para SMT-LIB (com `declare-sort` + cardinalidade finita quando `Enumerated`, ou busca incremental de tamanho 1..k para `Open`), pede `sat`, lê o modelo e o converte em `FiniteModel` canônico.
- Modos: `refute(O)` busca `Γ ∧ ¬φ`; `witness(Γ)` busca modelo de `Γ`.
- Limite de tamanho de domínio e timeout; `unknown`/timeout → nada (não produz certificado).

**Critério de conclusão.**
- [ ] Em 200 obrigações refutáveis de um corpus de teste, ≥ 95% recebem contramodelo aceito pelo checker.
- [ ] **Teste adversarial**: um "Z3 falso" que devolve modelos aleatórios nunca consegue um `Accepted` indevido em 10k tentativas.
- [ ] Um bug introduzido de propósito no tradutor (ex.: inverte uma negação) produz modelos *rejeitados*, nunca refutações falsas.

---

### Etapa 2.5 ∥ — Model finder alternativo (fora da TCB) · `P`

**Depende de (recebe pronto).** 2.1, 2.3.

**Fora do escopo.** model finders para domínios grandes; integração ao farm (10.2).

**Entregáveis.**
- Segundo produtor: busca exaustiva em Python/Rust para domínios minúsculos (≤ 4 elementos), ou integração com um model finder (Mace4/Paradox, se disponível).
- Usado como fallback quando Z3 dá `unknown`.

**Critério de conclusão.**
- [ ] Para domínios ≤ 3, encontra contramodelo em 100% dos casos em que existe (teste contra enumeração).

---

### Etapa 2.6 — Tradução de contramodelos para português · `P`

**Depende de (recebe pronto).** 2.1 — `FiniteModel`; 1.2 — `gloss_pt`.

**Fora do escopo.** paráfrase determinística de fórmulas (9.1); qualquer garantia de fidelidade (é só apresentação).

**Objetivo.** O agente e o usuário recebem `M` legível (§3, "M traduzido pelo vocabulário").

**Entregáveis.**
- `explain_model(M, V, O) -> String` usando `gloss_pt` dos símbolos: lista do domínio, fatos verdadeiros relevantes, e qual premissa/objetivo é falso.
- Fora da TCB de correção (é só apresentação); o hash do modelo vai junto da explicação.

**Critério de conclusão.**
- [ ] Para 20 refutações de exemplo, a explicação menciona o objetivo falsificado e só símbolos que aparecem em `O`.

---

## Fase 3 — Ledger assinado

Objetivo da fase: um log append-only, encadeado por hash, assinado pelo kernel, que qualquer um verifica sem perguntar ao kernel.

### Etapa 3.1 — Catálogo de eventos v1 · `M` · ⚠ TCB

**Depende de (recebe pronto).** 0.3, 0.4; 0.6 — `CheckOutcome`, `Budget`; 1.5 — `h(O)`.

**Fora do escopo.** eventos introduzidos nas Fases 5–17 (cada um entra na etapa que o introduz, ver Apêndice C); persistência (3.2).

**Entregáveis.**
- `enum EventBody` (detalhes no Apêndice C). Mínimo nesta fase: `Genesis`, `VocabPublished`, `Proposed`, `ObligationCreated`, `Verified`, `Refuted`, `UnknownRecorded`, `AssumptionDeclared`, `Retracted`, `Invalidated`, `EpochAdvanced`, `CheckerRegistered`, `CheckerRevoked`.
- `Event { seq, epoch, prev: h(Event_{seq-1}), body, kernel_key: KeyId, sig }`.
- Todo evento que registra um check carrega `{h(O), h(cert), h(checker), budget, outcome}`.
- `spec/events.md` normativo.

**Critério de conclusão.**
- [ ] Vetores canônicos para cada tipo de evento.
- [ ] Teste: alterar qualquer campo de um evento invalida a assinatura.

---

### Etapa 3.2 — Appender com hash chain · `M` · ⚠ TCB

**Depende de (recebe pronto).** 3.1 — `EventBody`, `Event`; 0.5 — CAS.

**Fora do escopo.** índices (3.3); HSM (13.1); log de transparência (13.2).

**Entregáveis.**
- `Ledger::append(body) -> SignedEvent`: única função que escreve; calcula `prev`, `seq`, assina.
- Persistência SQLite em transação (`BEGIN IMMEDIATE`), WAL, `fsync`. Um único escritor (lock de arquivo).
- Recuperação após crash: na abertura, verifica a cadeia inteira (ou desde o último checkpoint assinado) antes de aceitar novos eventos.

**Critério de conclusão.**
- [ ] Teste de crash: matar o processo (`SIGKILL`) em pontos aleatórios durante 10k appends; ao reabrir, a cadeia é sempre válida e sem buracos.
- [ ] Dois processos tentando ser escritores: o segundo falha ao abrir.

---

### Etapa 3.3 — Índices e consultas · `M`

**Depende de (recebe pronto).** 3.2 — ledger persistido; implementa a `LedgerView` definida na 1.5.

**Fora do escopo.** consultas específicas de UI; índices de ações (Fase 8).

**Entregáveis.**
- Tabelas derivadas (reconstruíveis a partir dos eventos): `claims(id, h_O, status, epoch)`, `deps(claim, premise)`, `citable(ref, epoch)`.
- `LedgerView` (interface usada pelo gerador de obrigações da Etapa 1.5): `is_citable(ref, epoch)`, `formula_of(ref)`, `status(ref)`, `dependents(ref)`.
- Comando `logos ledger rebuild-index` que recria tudo a partir dos eventos e compara.

**Critério de conclusão.**
- [ ] Apagar os índices e reconstruir produz exatamente o mesmo estado (teste com ledger de 100k eventos).
- [ ] `is_citable` em < 1 ms p99 com 1M eventos.

---

### Etapa 3.4 — Épocas · `P` · ⚠ TCB

**Depende de (recebe pronto).** 3.1, 3.3.

**Fora do escopo.** épocas de política (7.2): aqui só o mecanismo genérico.

**Entregáveis.**
- `EpochAdvanced { new_epoch, reason, policy_root?, vocab? }`.
- Semântica: citabilidade é por época; fatos têm janela de validade em épocas; provas sobre política anterior deixam de autorizar ações (uso real na Fase 7).

**Critério de conclusão.**
- [ ] Teste: fato válido nas épocas 3–5 é citável em 4 e recusado em 6.

---

### Etapa 3.5 — CAS integrado ao ledger · `P`

**Depende de (recebe pronto).** 3.2, 0.5.

**Fora do escopo.** GC destrutivo (só `--dry-run`).

**Entregáveis.**
- Todo hash referenciado por evento (`h(cert)`, `h(M)`, `h(O)`) tem o objeto no CAS **antes** do append (ordem: put → append).
- `logos ledger gc --dry-run` lista objetos órfãos (nunca apaga referenciados).

**Critério de conclusão.**
- [ ] Teste: append de evento referenciando hash ausente do CAS falha.

---

### Etapa 3.6 ★ — Auditor independente (`logos-audit`) · `M`

**Depende de (recebe pronto).** 0.3, 0.4, 2.3 e `spec/events.md` (3.1). **Não** depende de 3.2 em código (só do formato).

**Fora do escopo.** recheck de LRAT (acrescentado na 4.5); conferência com log de transparência (13.2).

**Objetivo.** Invariante S1: o ledger é verificável por terceiros.

**Entregáveis.**
- Binário que **não depende** de `logos-kernel`/`logos-ledger` (CI verifica com `cargo tree`). Usa só `logos-canon`, `logos-crypto` e os checkers.
- `logos-audit verify <ledger> --cas <dir>`: verifica cadeia, assinaturas, e **rechecagem** de todos os certificados (refutações agora; LRAT quando a Fase 4 chegar).
- `logos-audit export <ledger>`: exporta eventos em formato canônico para quem não tem SQLite.
- Relatório: eventos verificados, certificados rechecados, falhas.

**Critério de conclusão.**
- [ ] Em um ledger de exemplo, o auditor rechecou 100% dos `Refuted`.
- [ ] Ledgers adulterados (evento removido, reordenado, assinatura trocada, certificado trocado no CAS) são todos detectados — suíte com ≥ 10 adulterações.
- [ ] `cargo tree -p logos-audit` não contém `logos-kernel` nem `logos-ledger` (teste de CI).

---

## Fase 4 — Camada 1: grounding, CNF e LRAT

Objetivo da fase: o primeiro `VERIFIED` do sistema, sobre domínios finitos, terminando num certificado LRAT checado por um programa formalmente verificado (cake_lpr). É a rota B da §9, a que autoriza efeitos.

### Etapa 4.1 — Fragmento "domínio finito" · `P` · ⚠ TCB

**Depende de (recebe pronto).** 1.2 — `Enumerated`, `domain_source`; 1.5 — `Obligation`.

**Fora do escopo.** domínios `Open` (rota para a camada 2); grounding lazy.

**Entregáveis.**
- `classify(O, V) -> Fragment`: `FiniteDomain` se todo sort quantificado é `Enumerated` e não há símbolos de função com contradomínio `Open`; caso contrário, `General`.
- Os axiomas de fecho de domínio (`∀u:User. u = alice ∨ u = bob`) e de nomes únicos (`alice ≠ bob`) entram **como premissas explícitas** derivadas da fonte da enumeração (fato ou política), não como pressupostos implícitos do grounder.
- **Fechamento de predicados** (necessário para negação, ver 7.3): quando uma obrigação usa `¬P(…)` sobre um predicado cuja extensão vem de fatos ou regras, o mesmo mecanismo gera o axioma de fechamento (completion de Clark) de `P` — por exemplo, `∀x. Executed(backup(x)) ↔ (x = db1 ∨ x = db2)` — como premissa explícita, cuja fonte é um compromisso de completude (raiz Merkle da política na 7.2, ou projeção do ledger com `as_of: h(head)` na 7.5). Sem esse axioma, a negação não é derivável na rota clássica, e isso é o comportamento correto.

**Critério de conclusão.**
- [ ] Obrigação com sort `Open` quantificado é classificada `General` e roteada para fora da camada 1.
- [ ] Os axiomas de fecho aparecem em `O` e no `deps` do claim (rastreáveis à fonte da enumeração).
- [ ] Teste: `¬P(c)` não é provável sem o axioma de fechamento de `P` e passa a ser com ele; o axioma aparece em `used` com a fonte do compromisso de completude.

---

### Etapa 4.2 — Grounder · `M` · ⚠ TCB

**Depende de (recebe pronto).** 4.1; 2.2 — avaliador (usado no teste semântico).

**Fora do escopo.** grounding incremental/lazy (Apêndice E); funções com contradomínio `Open`.

**Entregáveis.**
- `ground(O) -> PropFormula`: instancia `∀`/`∃` sobre os domínios enumerados, avalia igualdade entre constantes distintas (usando os axiomas de nomes únicos), cria átomos proposicionais `Pred(c1,…,cn)`.
- Tabela de átomos canônica (ordenada), para que a numeração das variáveis SAT seja determinística.
- Limite de explosão: número de átomos e de nós; acima → `Unknown(Size)`.

**Critério de conclusão.**
- [ ] Teste semântico: para 5k obrigações pequenas, `O` é válida (verificado por enumeração de todos os modelos com o avaliador da Etapa 2.2) sse `ground(O)` é tautologia (verificado por tabela verdade).
- [ ] Determinismo: mesma `O` → mesmos bytes DIMACS (hash estável em CI).

---

### Etapa 4.3 — Tseitin canônico e mapa cláusula → premissa · `M` · ⚠ TCB

**Depende de (recebe pronto).** 4.2 — `PropFormula` e tabela de átomos; 0.5 — CAS.

**Fora do escopo.** pré-processamento SAT; minimização da CNF.

**Entregáveis.**
- `clausify(Γ_ground, ¬φ_ground) -> (Cnf, ProvenanceMap)`: Tseitin determinístico; cada cláusula original recebe a origem `Premise(ref) | NegatedGoal | DomainAxiom(ref) | Definitional`.
- Saída DIMACS canônica (cláusulas e literais ordenados); `h(CNF)` registrado no evento junto com `h(O)`.
- Decisão documentada em ADR: **a derivação O → CNF é determinística e está na TCB** (é o "clausificador, se não for certificado" da §5). O evento guarda `h(O)`, `h(CNF)` e `encoder_version`, e o auditor refaz a derivação.

**Critério de conclusão.**
- [ ] `CNF` é insatisfazível sse `ground(O)` é válida (teste com SAT solver de referência em 5k casos + enumeração nos pequenos).
- [ ] O auditor (`logos-audit`) refaz `O → CNF` e confere `h(CNF)` em 100% dos eventos.
- [ ] LOC de grounder + Tseitin reportado na TCB (meta: < 1.000 LOC somados).

---

### Etapa 4.4 — Checker LRAT próprio · `M` · ⚠ TCB

**Depende de (recebe pronto).** 4.3 — DIMACS canônico e `ProvenanceMap`; 0.6 — `Budget`.

**Fora do escopo.** DRAT (convertido fora da TCB na 4.6); formatos LPR/PR.

**Objetivo.** Um checker em Rust para desenvolvimento e para ter **dois checkers** independentes (com o cake_lpr).

**Entregáveis.**
- `check_lrat(cnf_bytes, h_cnf, proof_bytes, budget) -> CheckOutcome` (P2: confere `H(cnf_bytes) == h_cnf`).
- Suporte a adição de cláusula com hints RUP e deleção; recusa formatos LRAT malformados.
- Extração do conjunto de **cláusulas originais usadas** (fecho para trás a partir da cláusula vazia).

**Critério de conclusão.**
- [ ] Aceita as provas LRAT de um conjunto de ≥ 200 instâncias SAT-competition pequenas/médias (pigeonhole, etc.).
- [ ] Rejeita 100% de uma suíte de provas corrompidas (hint trocado, cláusula alterada, cláusula vazia sem derivação).
- [ ] Concordância total com `cake_lpr` sobre a mesma suíte (após a Etapa 4.5).

---

### Etapa 4.5 — Integração do `cake_lpr` · `M` · ⚠ TCB (binário verificado)

**Depende de (recebe pronto).** 4.4 — suíte de instâncias e provas; 3.6 — auditor (para estender o recheck).

**Fora do escopo.** modificar o `cake_lpr`; registro no kernel (5.4) — aqui só wrapper, hash e registry local.

**Entregáveis.**
- Build reprodutível do `cake_lpr` (script + hash do binário esperado); o binário é identificado por `h(binário)` no `CheckerRegistered`.
- Wrapper: executa em subprocesso com limite de memória/tempo (rlimit), sem rede, com CNF e prova passados por arquivo do CAS.
- Política de registry: um checker externo só é registrado com hash conhecido; hash diferente = recusa.
- `logos-audit` estendido: rechecagem de todo `Verified` de camada 1 com os dois checkers LRAT (4.4 e `cake_lpr`), refazendo também `O → CNF` (4.3).

**Critério de conclusão.**
- [ ] Binário de `cake_lpr` reconstruído do zero tem o mesmo hash em duas máquinas.
- [ ] Trocar o binário por outro (mesmo nome) faz o kernel recusar o check.
- [ ] Mesma suíte da Etapa 4.4 passa com resultados idênticos.
- [ ] `logos-audit` recheca 100% dos `Verified` de camada 1 de um ledger de exemplo, com os dois checkers, e detecta um certificado LRAT trocado no CAS.

---

### Etapa 4.6 — Produtor SAT (fora da TCB) · `P`

**Depende de (recebe pronto).** 4.3, 4.4; 2.3 — checker de refutação (para o caso `sat`); 0.3 — encoder Python.

**Fora do escopo.** portfólio e cache (10.2).

**Entregáveis.**
- `logos_provers.sat`: roda CaDiCaL (ou Kissat) com saída DRAT/LRAT; se só DRAT, converte com `drat-trim` para LRAT. Tudo fora da TCB.
- Se o solver diz `sat`, devolve o modelo como **contramodelo** (convertido de volta para `FiniteModel` via a tabela de átomos) → vai para o checker da camada 0.

**Critério de conclusão.**
- [ ] Ponta a ponta: obrigação de domínio finito → CNF → CaDiCaL → LRAT → `cake_lpr` aceita, em ≥ 95% de um corpus de 300 obrigações de política.
- [ ] Caso `sat`: contramodelo aceito pela camada 0 em 100% dos casos.

---

### Etapa 4.7 ★ — Dependências extraídas do certificado · `P` · ⚠ TCB

**Depende de (recebe pronto).** 4.3 — `ProvenanceMap`; 4.4 — cláusulas usadas.

**Fora do escopo.** registro em evento (feito na 5.4); cascata (5.9).

**Objetivo.** §4.5: o DAG usa as premissas **que o certificado usou**, não as citadas pelo agente.

**Entregáveis.**
- `used_premises(lrat_used_clauses, provenance_map) -> Set<PremiseRef>`.
- O evento `Verified` registra `cited` (do agente, informativo) e `used` (do certificado, normativo). Só `used` entra no DAG de dependências.

**Critério de conclusão.**
- [ ] Teste: agente cita 5 premissas, prova usa 2 → `deps` do claim contém exatamente as 2.
- [ ] Teste: agente não cita uma premissa necessária → obrigação não é provável (o certificado não pode usar o que não está em `O`) — o agente não consegue "esconder" dependências.

---

## Fase 5 — Kernel: máquina de estados e protocolo

Objetivo da fase: `logos-kernel` como processo isolado, implementando a máquina de estados da §3 e o ciclo de vida de provas da §4. Fecha o item 1 da ordem de construção da §12.

### Etapa 5.1 — Typestate de claims · `M` · ⚠ TCB

**Depende de (recebe pronto).** 3.1 — eventos; 0.6 — `Verdict`.

**Fora do escopo.** transporte (5.2); formalização (12.1).

**Entregáveis.**
- `Claim<S>` com `S ∈ {Draft, Pending, Verified, Refuted, Unknown, Invalidated}` como tipos fantasma.
- Construtores `pub(crate)`: só o módulo de transições cria `Claim<Verified>`. Transições como funções que consomem o estado anterior: `fn accept(c: Claim<Pending>, ev: &SignedEvent<Verified>) -> Claim<Verified>`.
- Tabela de transições permitidas em `spec/state-machine.md` (é a mesma que a Fase 12 formaliza).

**Critério de conclusão.**
- [ ] Testes compile-fail: construir `Claim<Verified>` fora do crate não compila; transição `Refuted → Verified` não existe.
- [ ] Toda transição gera exatamente um evento do ledger (teste por propriedade sobre sequências aleatórias de comandos).

---

### Etapa 5.2 — Servidor do protocolo · `M` · ⚠ TCB

**Depende de (recebe pronto).** 1.6 — `Proposal`; 0.3 — canon; 0.5 — CAS (staging).

**Fora do escopo.** vários agentes por socket (v1: um agente por conexão autenticada pelo SO); MCP (10.3).

**Entregáveis.**
- Unix socket com frames canônicos. Mensagens v1: `Propose`, `Certify{h_O, format, cert_ref}`, `Refute{h_O, model_ref}`, `Witness{h_O, model_ref}`, `Assume`, `Query`, `Subscribe(events)`.
- Upload de certificado grande via CAS (o agente escreve num diretório de staging; o kernel recalcula o hash e move).
- Um handler por mensagem, sem estado compartilhado mutável fora do ledger.
- Limites por conexão (tamanho de frame, taxa) para negação de serviço.

**Critério de conclusão.**
- [ ] Fuzz do decoder de mensagens ≥ 4h sem crash.
- [ ] Mensagem com campo extra ou tipo desconhecido → erro, nunca ignorada silenciosamente.

---

### Etapa 5.3 — `PROPOSE` e regra de citação · `P` · ⚠ TCB

**Depende de (recebe pronto).** 5.1, 5.2; 1.5 — `generate`; 3.3 — `LedgerView` real.

**Fora do escopo.** especulação (5.8); ações (8.2).

**Entregáveis.**
- Handler: valida a proposta → `generate` (Etapa 1.5) → CAS `put(O)` → eventos `Proposed` + `ObligationCreated` → claim `Pending`.
- Retorna `h(O)` e os bytes de `O` para o agente levar ao prover.

**Critério de conclusão.**
- [ ] **Teste de invariante P1**: suíte de 50 tentativas de um agente malicioso (citar o não verificado, mandar obrigação pronta, mandar fato, citar evento de outra época, citar claim retratado) — todas recusadas.
- [ ] Reproduz o "STATE 14" da §3 como teste de integração.

---

### Etapa 5.4 — `CERTIFY` e registry de checkers · `M` · ⚠ TCB

**Depende de (recebe pronto).** 5.3; 2.3 — checker de refutação; 4.4, 4.5 — checkers LRAT; 4.7 — `used`; 3.2 — appender.

**Fora do escopo.** camada 2 (Fase 14); avaliação direta (7.3).

**Entregáveis.**
- `CheckerRegistry`: `format → [checker por h(binário|crate)]`, com status `active | revoked`.
- Handler: busca `O` pelo hash, obtém bytes do CAS, roteia pelo fragmento (camada 1 nesta fase), roda checker, interpreta `CheckOutcome`:
  - `Accepted` → (Etapa 5.5) → `Verified`;
  - `Rejected` → claim volta a `Pending`, evento `CertificateRejected` (não é refutação);
  - `Unknown(_)` → evento `UnknownRecorded` com limites; claim fica `Pending` ou vai a `Unknown` após política de tentativas.
- `Refute` → checker da camada 0 → `Refuted`.

**Critério de conclusão.**
- [ ] **Invariantes P3 e P4**: não existe caminho de código que leve a `Verified` sem `CheckOutcome::Accepted` de um checker ativo (teste de propriedade + revisão; formalizado na Fase 12).
- [ ] Reproduz o "STATE 15–16" e a variante de refutação da §3 como testes de integração.

---

### Etapa 5.5 — Não-vacuidade (testemunha de consistência) · `P` · ⚠ TCB

**Depende de (recebe pronto).** 5.4; 2.3 — `check_consistency`.

**Fora do escopo.** consistência em FOL aberta (14.6).

**Entregáveis.**
- Todo `Verified` carrega `consistency: Witness(h(M0)) | ConsistencyUnknown`.
- O kernel aceita `Witness{h_O, model}` antes ou depois do `CERTIFY` (até um prazo); sem testemunha → `ConsistencyUnknown` explícito.
- Flag por uso: claims `ConsistencyUnknown` **não podem** ser premissa de autorização de ação (regra aplicada na Fase 8).

**Critério de conclusão.**
- [ ] Teste: `Γ = {P, ¬P}` prova qualquer coisa; o `Verified` resultante fica `ConsistencyUnknown` (nenhum modelo existe) e é recusado como premissa de ação.

---

### Etapa 5.6 — Bloqueio de re-proposta refutada (S4) · `P` · ⚠ TCB

**Depende de (recebe pronto).** 5.4; 3.3 — índices.

**Fora do escopo.** detecção de propostas semanticamente equivalentes mas sintaticamente diferentes.

**Entregáveis.**
- Conjunto `refuted_obligations` no índice; `PROPOSE` cujo `h(O)` está refutado → `Reject::AlreadyRefuted(h(M))`.
- Variações triviais (mesma fórmula, premissas reordenadas) já colapsam no mesmo `h(O)` pela canonicalização.

**Critério de conclusão.**
- [ ] Teste: refutar, re-propor a mesma coisa com citações em outra ordem → recusado.
- [ ] Teste: um prover que dá timeout na segunda tentativa não converte refutação em `UNKNOWN`.

---

### Etapa 5.7 — Assumptions com escopo · `P` · ⚠ TCB

**Depende de (recebe pronto).** 5.3; 3.1.

**Fora do escopo.** assumptions assinadas pelo usuário (entram depois da 6.5; aqui o autor usuário vem de CLI local); uso em ações (bloqueado na 8.2).

**Entregáveis.**
- `Assume { formula, scope: Session | Claim(id) | Until(epoch), author }` → evento `AssumptionDeclared` com autor (agente ou usuário assinado).
- Claims que dependem de assumption carregam a assumption no conjunto `open` (premissas não descarregadas); a forma exportável é `⋀assumptions → φ`.
- Assumption do agente **nunca** satisfaz requisito de confiança de política para ações.

**Critério de conclusão.**
- [ ] Claim derivado de assumption aparece como condicional no `Query`.
- [ ] Rejeitar uma assumption (Etapa 5.9) invalida os dependentes.

---

### Etapa 5.8 — Especulação de claims · `M` · ⚠ TCB

**Depende de (recebe pronto).** 5.3, 5.4.

**Fora do escopo.** uso em ações (bloqueado na 8.2).

**Entregáveis.**
- `Proposal.speculative_on: [ClaimId]` permite citar `Pending`. O claim resultante fica `Pending(conditional_on = …)`.
- Quando a base é `Verified`, os especulativos podem ser checados normalmente; quando a base é `Refuted`/`Invalidated`, os especulativos caem em cascata.
- Claim especulativo **nunca** é citável por ação (P5, reforçado na Fase 8).

**Critério de conclusão.**
- [ ] Teste: C8 especula sobre C7; C7 refutado → C8 `Invalidated` com causa `SpeculationFailed(C7)`.
- [ ] Métrica: profundidade de propagação de erro (Apêndice D) = 0 sem especulação, em teste com 1000 cadeias aleatórias.

---

### Etapa 5.9 — Retratação e invalidação em cascata (ATMS) · `M` · ⚠ TCB

**Depende de (recebe pronto).** 5.4; 4.7 — `used`; 3.3 — `dependents`.

**Fora do escopo.** revogação de atestadores (6.4); reversão de ações executadas (11.7).

**Entregáveis.**
- `Retract { ref, reason, authority_sig }` (só o emissor do fato/assumption ou um admin pode retratar).
- Cascata pelo DAG de dependências **normativas** (`used`, Etapa 4.7). Um claim com **rotas independentes** (dois `Verified` diferentes para a mesma fórmula, com conjuntos `used` diferentes) só cai se todas as rotas caírem — como labels de ATMS.
- Eventos `Invalidated { claim, cause_chain }`; nada é apagado.

**Critério de conclusão.**
- [ ] Teste com DAG de 10k nós: retratação produz exatamente o conjunto esperado (comparado com implementação de referência ingênua em Python).
- [ ] Teste de rota independente: claim com duas provas sobre premissas disjuntas sobrevive à retratação de uma.

---

### Etapa 5.10 — Revogação de checker e recheck · `M` · ⚠ TCB

**Depende de (recebe pronto).** 5.4, 5.9; 4.4, 4.5 — dois checkers LRAT.

**Fora do escopo.** revogação de tokens no gateway (8.8).

**Entregáveis.**
- `CheckerRevoked { h(checker), reason }` (só admin). O índice lista todos os eventos aceitos por esse checker.
- `logos recheck --checker <h> --with <h2>`: recheca cada certificado com outro checker; sucesso → evento `Rechecked` (claim continua válido, agora sustentado pelo novo checker); falha → `Invalidated` + cascata.
- Claims sustentados só por um checker revogado e ainda não rechecados ficam em estado `Suspended` (não citáveis por ações).

**Critério de conclusão.**
- [ ] Cenário: revogar o checker LRAT próprio; rechecar tudo com `cake_lpr`; nenhum claim perdido se os certificados forem bons; certificado plantado como falso-aceito é invalidado.

---

### Etapa 5.11 — Isolamento de processos · `M` · ⚠ TCB

**Depende de (recebe pronto).** 5.2 — socket do kernel; 3.2 — arquivo do ledger.

**Fora do escopo.** isolamento do gateway real (8.4): os itens do critério sobre gateway e credenciais usam um gateway stub até lá e são repetidos na 8.4.

**Objetivo.** Mecanismo 1 da §3: o agente não tem credenciais, nem a chave do ledger.

**Entregáveis.**
- Usuários de SO separados: `logos-kernel`, `logos-gateway`, `logos-agent`. Chave do kernel legível só por `logos-kernel`.
- Sandbox do agente: sem rede exceto para a API do LLM e o socket do kernel/MCP (namespaces, `landlock`, `seccomp` ou container rootless).
- Units `systemd` (ou `docker-compose`) de referência.

**Critério de conclusão.**
- [ ] Teste de penetração scriptado: a partir do processo do agente, tentar ler a chave do kernel, abrir o SQLite do ledger em escrita, conectar ao gateway diretamente, ler as credenciais das ferramentas — todas falham.

---

### Etapa 5.12 ★ — Kernel mínimo ponta a ponta · `P`

**Depende de (recebe pronto).** 5.1–5.11; um atestador de teste (stub — atestadores reais na Fase 6).

**Fora do escopo.** ações e gateway (Fase 8); LLM (Fase 10): a demo é scriptada.

**Objetivo.** Marco: fim do item 1 da ordem de construção da §12.

**Critério de conclusão.**
- [ ] Script de demo: vocabulário → fatos de teste (inseridos por um atestador de teste) → 3 propostas → uma `Verified` via LRAT, uma `Refuted` via modelo, uma `Unknown` por timeout → retratação de um fato → cascata → `logos-audit` verifica e recheca tudo.
- [ ] Toda a demo reproduzível com um comando (`make demo-kernel`).

---

## Fase 6 — Atestadores e fatos

Objetivo da fase: fatos entram no sistema só como objetos assinados por emissores identificados. O LLM não tem chave para assinar nada.

### Etapa 6.1 — Objeto `Fact` · `P` · ⚠ TCB

**Depende de (recebe pronto).** 1.1, 1.2; 0.4 — assinatura; 3.1 — eventos; 5.3 — citação.

**Fora do escopo.** confiança por tópico (6.2).

**Entregáveis.**
- `Fact { formula, vocab: h(V), issuer: KeyId, topic, observed_at, valid: EpochRange | Until(ts), version?: (resource, ver), sig }`.
- `version` é o gancho para TOCTOU (Fase 8): fatos sobre recursos versionados carregam a versão observada.
- Kernel registra `FactRegistered { h(fact) }` só se a assinatura verifica contra uma raiz de confiança.

**Critério de conclusão.**
- [ ] Fato com assinatura de emissor desconhecido → recusado.
- [ ] Fato expirado → não citável.

---

### Etapa 6.2 — Raízes e política de confiança por tópico · `M` · ⚠ TCB (fidelidade)

**Depende de (recebe pronto).** 6.1.

**Fora do escopo.** delegação entre agentes (17.2).

**Entregáveis.**
- `TrustConfig { roots: [KeyId → {name, topics: [Topic], level}] }`, ela mesma assinada pelo admin e versionada no ledger.
- Regra: um fato só é citável se o emissor é confiável **para o tópico** do fato (`db-monitor` não atesta backups).
- Na lógica, a confiança aparece como premissa explícita (prepara P8): `Trusted(issuer, topic)`.

**Critério de conclusão.**
- [ ] Teste: `db-monitor` assinando fato de tópico `backup` → não citável.

---

### Etapa 6.3 — SDK de atestador e atestadores de referência · `M`

**Depende de (recebe pronto).** 6.1, 6.2; 1.2 — status `Anchored`.

**Fora do escopo.** atestadores de produção: `pg-monitor` pode ficar como mock nesta etapa.

**Entregáveis.**
- `logos-attest` (Rust) + `logos_attest` (Python): `observe() -> Formula`, assinatura, envio ao kernel.
- Atestadores de referência:
  - `clock` (tempo atual, com tolerância);
  - `fs-stat` (existência, hash e mtime de arquivos — versionado por hash);
  - `sqlite-monitor` / `pg-monitor` (contagem de conexões, existência de tabela, versão de linha);
  - `backup-svc` (mock: registra backups com timestamp);
  - `enum-source` (atesta o fecho de domínio: "os usuários são exatamente {…}").
- Cada atestador tem um documento "procedimento" (para o status `Anchored` dos símbolos — Etapa 1.2).

**Critério de conclusão.**
- [ ] Cada atestador de referência tem teste de integração produzindo um fato citável.
- [ ] O vocabulário de exemplo tem todos os símbolos operacionais `Anchored` ou `Defined`.

---

### Etapa 6.4 — Revogação de fatos e de atestadores · `P` · ⚠ TCB

**Depende de (recebe pronto).** 6.1; 5.9 — cascata.

**Fora do escopo.** rotação de chaves de atestador.

**Entregáveis.**
- `Retract` (Etapa 5.9) para fatos; `IssuerRevoked { KeyId, since }` para atestador comprometido → retrata em massa todos os fatos dele após `since` (cascata).

**Critério de conclusão.**
- [ ] Revogar `backup-svc` invalida todos os claims que dependiam de fatos dele, e só esses.

---

### Etapa 6.5 — Chave do usuário (v1) · `P`

**Depende de (recebe pronto).** 0.4; 6.2 — raízes; 5.11 — usuários de SO separados.

**Fora do escopo.** passkey/WebAuthn (9.2).

**Entregáveis.**
- Usuário tem par Ed25519 local (`logos user init`), registrado como raiz de confiança com tópicos `authorize`, `confirm`, `assume`.
- Assinaturas do usuário são feitas **fora do processo do agente** (CLI separada ou TUI do lado do kernel). Passkey/WebAuthn na Fase 9.

**Critério de conclusão.**
- [ ] O processo do agente não consegue produzir assinatura do usuário (a chave não é legível por ele — teste do 5.11 estendido).

---

### Etapa 6.6 — Conteúdo não confiável como `fonte diz φ` · `M` · ⚠ TCB (fidelidade)

**Depende de (recebe pronto).** 1.1 — AIR; 6.1, 6.2.

**Fora do escopo.** extrator LLM em quarentena (11.1): aqui só o operador `Says`, o atestador de ingestão e o evento `Extracted`.

**Objetivo.** Preparar a Fase 11 (§13): dados de e-mail, web, documentos entram como premissas atribuídas, nunca como fatos.

**Entregáveis.**
- Operador `Says(source, φ)` no AIR (`source` é um termo de sort `Source`).
- Atestador de ingestão: dado um conteúdo bruto `c` de uma fonte `s`, atesta `ContentOf(s) = h(c)` e `Origin(s) = external|internal|user`. **O atestador atesta a proveniência dos bytes, não o significado.**
- Extração (LLM em quarentena, Fase 11) produz `Says(s, φ)` como **proposta** que o kernel registra como `Extracted { source: s, h(c), φ, extractor: id }` — premissa de confiança "extração por X", nunca fato.

**Critério de conclusão.**
- [ ] Um `Says(email1, Pedido(x))` nunca satisfaz uma premissa que exige `Pedido(x)` sem a regra de política que o endossa.
- [ ] O certificado de qualquer prova que use conteúdo externo mostra a premissa `Says(...)` em `used`.

---

## Fase 7 — Políticas

Objetivo da fase: políticas formais, escritas por humanos direto em lógica (fidelidade F4), versionadas, com raiz Merkle assinada, prova de não-pertinência para negação, e capazes de falar sobre o próprio ledger (trajetórias).

### Etapa 7.1 — Linguagem de política · `M` · ⚠ TCB

**Depende de (recebe pronto).** 1.1–1.4; 6.6 — `Says`.

**Fora do escopo.** árvore Merkle (7.2); avaliação (7.3).

**Entregáveis.**
- Política = conjunto de regras em AIR, numa forma restrita (cláusulas de Horn com negação estratificada) para o fragmento decidível **e** regras FOL gerais para o fragmento que exige certificado.
- Toda regra tem `id`, `class` (ex.: `allow`, `deny`, `require`), e comentário em português (glosa, não normativa).
- Sintaxe de superfície em `spec/policy.md`. Exemplo:
  ```text
  policy v12 {
    rule r1 allow: Classe(x) = responder ∧ Says(e, Pedido(x)) ∧ Externo(e) -> Allowed(x)
    rule r7 require: Classe(x) = destrutiva -> (Allowed(x) <-> ∃u. Usuario(u) ∧ Signs(u, Autoriza(x)))
    rule r9 deny: Drop(x) ∧ ¬∃t. Executed(backup(x), t) ∧ Recent(t, 1h) -> ¬Allowed(Drop(x))
  }
  ```
- Semântica: `Allowed(a)` só é derivável se alguma regra `allow` dispara e nenhuma `deny` aplicável dispara (semântica deny-overrides, documentada).

**Critério de conclusão.**
- [ ] Parser + sort-check de políticas com 100% de cobertura de ramos.
- [ ] Estratificação: política com ciclo pela negação é recusada.

---

### Etapa 7.2 — Árvore Merkle ordenada e raiz assinada · `M` · ⚠ TCB

**Depende de (recebe pronto).** 7.1; 0.4; 3.4 — épocas.

**Fora do escopo.** atualização incremental da árvore.

**Entregáveis.**
- Árvore Merkle sobre as regras ordenadas por `(chave de indexação, h(regra))`; a chave de indexação permite provar "não existe regra `deny` que case com a ação `a`".
- `PolicyPublished { version, root r_v, admin_sig }` no ledger; `EpochAdvanced` associado.
- `prove_inclusion(rule)` e `prove_non_membership(index_key)`; verificadores na TCB.
- Não-pertinência vira premissa checada: `NoRule(deny, key(a), v)`.

**Critério de conclusão.**
- [ ] Property test: para toda chave, exatamente uma das provas (inclusão ou não-pertinência) verifica.
- [ ] Adulterar uma regra muda `r_v`; prova antiga deixa de verificar.

---

### Etapa 7.3 — Avaliador direto (rota L2) · `M` · ⚠ TCB

**Depende de (recebe pronto).** 7.1, 7.2; 6.1 — fatos; 4.1 — axiomas de fechamento; 4.6 — rota LRAT (para o teste diferencial).

**Fora do escopo.** recursão não estratificada; agregações.

**Objetivo.** Respeitar L2: para o fragmento decidível, não pedir prova ao agente — o kernel avalia.

**Entregáveis.**
- Avaliador Datalog estratificado, semi-naive, pequeno, que produz uma **árvore de derivação** de `Allowed(a)` com folhas = fatos, regras (com prova de inclusão) e ausências (com prova de não-pertinência / falha finita no fragmento fechado).
- Checker da árvore de derivação (ainda menor que o avaliador) — o evento registra a árvore no CAS, para que terceiros rechequem sem reavaliar.
- Evento `Verified { route: DirectEval, … }` (mesmo formato dos outros).
- **Ponte semântica com a rota FOL.** O Datalog estratificado usa semântica de modelo mínimo (mundo fechado: o que não é derivado é falso); a camada 1 usa consequência clássica (o que não é derivado é só desconhecido). As duas só coincidem quando os fechamentos de predicado entram como premissas. Por isso, ao montar a obrigação de uma ação para a rota FOL, o kernel inclui os axiomas de completion (Etapa 4.1) de todo predicado negado em regras aplicáveis, com a fonte do compromisso de completude. A árvore de derivação do avaliador direto referencia os mesmos compromissos nas folhas de ausência. Documentar em `spec/policy.md` (seção "Semântica").

**Critério de conclusão.**
- [ ] Teste diferencial avaliador × grounding+LRAT (Fase 4) sobre 2k pares (política, ação), **com axiomas de completion na rota FOL**: mesmo veredicto. Pelo menos 30% dos pares usam regras com negação.
- [ ] Teste de controle: nos pares com negação, a rota FOL **sem** os axiomas de completion não prova `Allowed` (mostra que a ponte é necessária, e não um acaso do corpus).
- [ ] Benchmark: p99 < 5 ms para políticas de 500 regras e 10k fatos.

---

### Etapa 7.4 — Roteamento por fragmento · `P` · ⚠ TCB

**Depende de (recebe pronto).** 7.3; 4.1 — `classify`.

**Fora do escopo.** camada 2 (Fase 14): até lá, a rota FOL geral devolve `Unknown`.

**Entregáveis.**
- Para cada obrigação de ação: se o fragmento é Datalog → 7.3; se é FOL com domínios finitos → camada 1; senão → camada 2 (Fase 14) ou `Unknown`.
- A rota escolhida é registrada no evento e é **determinística** a partir de `O`.

**Critério de conclusão.**
- [ ] Teste: a mesma obrigação sempre recebe a mesma rota; o auditor reproduz a escolha.

---

### Etapa 7.5 — Fatos do próprio ledger (políticas de trajetória) · `M` · ⚠ TCB

**Depende de (recebe pronto).** 7.3; 3.3 — índices; 6.1 — formato de fato.

**Fora do escopo.** TOCTOU das projeções (tratado na 8.5).

**Objetivo.** §6, "Políticas sobre trajetórias": o ledger é premissa.

**Entregáveis.**
- O kernel atua como atestador do próprio ledger: projeções assinadas `Executed(action_term, t)`, `CountExecuted(class, session) = n`, `ExecutedSince(x, t)`.
- Fecho: "não houve backup de x na última hora" é atestado pelo kernel com a cabeça do ledger em que a consulta foi feita (`as_of: h(head)`), sujeito a TOCTOU (Fase 8).

**Critério de conclusão.**
- [ ] Cenário de decomposição (§10, ameaça 12): política "no máximo 3 deleções por sessão"; o agente tenta 4 deleções de tabelas separadas → a 4ª é recusada.
- [ ] Cenário: `drop(x)` exige `Executed(backup(x))` na última hora → sem backup, recusado; após backup executado via gateway, permitido.

---

### Etapa 7.6 ∥ — Análise de políticas (fora da TCB, com resultados checados) · `M`

**Depende de (recebe pronto).** 7.1, 7.3; 2.3 — checker de modelos.

**Fora do escopo.** sugestão automática de correções de política.

**Objetivo.** Mitigar a ameaça 3 (política errada).

**Entregáveis.**
- `logos policy diff v11 v12`: usa Z3 para achar ações que passaram a ser permitidas/proibidas; cada exemplo encontrado é um **modelo** checado pela camada 0 (então o relatório é confiável mesmo com Z3 não confiável).
- `logos policy explain <action>`: árvore de derivação legível.
- Lints: regra `allow` que nunca dispara, `deny` sombreado, classes de ação sem regra.

**Critério de conclusão.**
- [ ] Em 10 pares de versões construídos com diferenças conhecidas, o `diff` encontra todas e cada exemplo é checado.

---

## Fase 8 — Ações: contratos, tokens e gateway

Objetivo da fase: o regime operacional completo da §6. Item 2 da ordem de construção da §12.

### Etapa 8.1 — Contratos de ferramenta · `M` · ⚠ TCB (fidelidade)

**Depende de (recebe pronto).** 1.1, 1.2; 3.1; 7.1 — classes de ação.

**Fora do escopo.** compensação automática (11.7); contratos de ferramentas de produção (23.2).

**Entregáveis.**
- `ToolContract { tool, version, args_schema, alpha: função determinística args → termo AIR, class, effect_pt, preconditions: [PreSpec], compensation?: ContractRef }`, assinado pelo admin, registrado no ledger.
- `alpha` é escrita numa mini-linguagem declarativa de mapeamento (não código arbitrário) para ser auditável. Ex.: `db.drop {name: s} ↦ Drop(Db(s))`.
- `PreSpec { kind: Transactional | Versioned | Unversioned | Physical, fact_pattern }` (tabela TOCTOU da §6).
- A suposição "o contrato descreve corretamente o efeito" fica escrita em `effect_pt` (TCB de fidelidade).

**Critério de conclusão.**
- [ ] `alpha` é total sobre `args_schema` (property test) e determinística (mesmos bytes → mesmo termo).
- [ ] Contrato não assinado ou de versão revogada → ferramenta indisponível.

---

### Etapa 8.2 — `ACTION_INTENT` e obrigação de ação · `M` · ⚠ TCB

**Depende de (recebe pronto).** 8.1; 7.3, 7.4 — avaliação e roteamento; 6.1 — fatos; 5.3, 5.5, 5.8.

**Fora do escopo.** mint (8.3); autorização do usuário via passkey (9.2) — aqui `Auth` vem assinado pela chave local da 6.5.

**Entregáveis.**
- Mensagem `ActionIntent { tool, args_bytes, cites: [ref], justification_hint? }` — **sem** campo de descrição que entre na lógica.
- Kernel: `a := alpha(tool, args)` (P6); busca `Pol_v` vigente; pede aos atestadores os fatos de pré-condição **agora** (ou aceita fatos citados recentes dentro da janela); monta `O := canon(Pol_v ∪ Obs ∪ Auth ∪ cites ⊢ Allowed(a))`.
- Regras extras de citação para ações: nenhuma premissa `Pending`, especulativa, `ConsistencyUnknown`, `Suspended` ou assumption do agente (P5).

**Critério de conclusão.**
- [ ] **Invariante P6**: teste em que o agente descreve "limpar registros antigos" mas os args são `db.drop{orders}` → a obrigação é sobre `Drop(Db(orders))`.
- [ ] **Invariante P5**: ação citando claim especulativo → recusada antes de qualquer check.

---

### Etapa 8.3 — Mint de tokens · `M` · ⚠ TCB

**Depende de (recebe pronto).** 8.2; 5.5 — testemunha; 0.4.

**Fora do escopo.** Biscuit/atenuação offline; revogação (8.8).

**Entregáveis.**
- `ActionToken { h(tool,args), h(a), event: h(E_verified), policy_root: r_v, pre: [(issuer, resource, version)], unprotected_pre: [fact_ref], aud: adapter_id, nonce, exp }`, assinado pelo kernel com domínio `logos/token/v1`.
- Só mintado após `Verified(Pol ∪ Obs ∪ Auth ⊢ Allowed(a))` com testemunha de consistência (§6: "Γ consistente").
- Evento `TokenMinted { h(token) }` no ledger.
- Validade padrão: 30 s (configurável por classe de ação).

**Critério de conclusão.**
- [ ] Não existe função pública de mint que não receba um `SignedEvent<Verified>` do tipo ação com consistência `Witness` (teste compile-fail + revisão).
- [ ] `unprotected_pre` é não vazio sempre que alguma pré-condição é `Unversioned`.

---

### Etapa 8.4 — Gateway e verificação de token · `M` · ⚠ TCB

**Depende de (recebe pronto).** 8.3; 5.11 — isolamento.

**Fora do escopo.** adaptadores concretos (8.6): aqui um adaptador de teste.

**Entregáveis.**
- Processo `logos-gateway` (usuário de SO próprio), único detentor das credenciais.
- Verifica: assinatura do kernel, `aud`, `exp`, nonce não usado (store persistente), `r_v ≥ política mínima`, `h(tool, args) == chamada real` (o gateway recebe os bytes da chamada, não confia em nada do token além do hash), evento referenciado não revogado (lista de revogação, Etapa 8.8).
- Interface para adaptadores: `Adapter::execute(call, pre) -> Result<Observed, Err>`.

**Critério de conclusão.**
- [ ] Suíte de replay/roubo (§10, ameaça 11): token reusado, token de outra audiência, token expirado, args trocados, token de política antiga → todos recusados.

---

### Etapa 8.5 ★ — Execução condicional e evento `Executed` · `G` · ⚠ TCB

**Depende de (recebe pronto).** 8.4; 6.1 — versões nos fatos; 6.3 — atestadores.

**Fora do escopo.** adaptadores além de um transacional e um versionado de referência (resto na 8.6).

**Entregáveis.**
- Mecanismos TOCTOU (tabela da §6):
  - `Transactional`: adaptador SQL abre transação, confere versão das linhas/estado, executa, commit — tudo na mesma transação;
  - `Versioned`: requisição condicional (`If-Match`, CAS em git/objeto);
  - `Unversioned`: só a janela `exp`; registrado em `unprotected_pre`;
  - `Physical`: a política pode exigir `Auth` do usuário com timestamp dentro de N segundos.
- Após execução: adaptador produz `Observed` → atestado pelo gateway → `Executed { h(token), result, observed_facts }` no ledger.
- Falha de pré-condição → `ExecutionAborted { h(token), pre_failed }`, nada executado.

**Critério de conclusão.**
- [ ] **Invariante P5** completa: teste de propriedade — para toda execução registrada, existe `TokenMinted` cujo `Verified` só usa premissas não especulativas.
- [ ] Teste TOCTOU: mudar a versão do recurso entre mint e execução → `ExecutionAborted`.
- [ ] **Garantia do gateway (§6)** como teste de propriedade: em 10k execuções aleatórias com agente adversarial, todo `Executed` tem a cadeia completa (token → evento → obrigação → certificado → política assinada → fatos assinados → testemunha). Verificada por `logos-audit`.

---

### Etapa 8.6 — Adaptadores de referência · `G`

**Depende de (recebe pronto).** 8.5; 8.1.

**Fora do escopo.** ferramentas de produção (GitHub, S3, …: 23.2).

**Entregáveis.**
- `fs` (escrever/apagar/mover arquivo dentro de uma raiz; versionado por hash do conteúdo);
- `sqlite` e `postgres` (DDL/DML parametrizadas; transacional);
- `http` (GET/PUT/DELETE com `If-Match`);
- `git` (commit/push com CAS no ref);
- `email` (mock SMTP/IMAP em memória — usado na Fase 11);
- `shell-restricted` (lista fechada de comandos com contratos; **sem** shell arbitrário).
- Cada adaptador com contrato (Etapa 8.1) e compensação quando existir (`restore`, `revert`, `notify`).

**Critério de conclusão.**
- [ ] Cada adaptador tem teste de integração ponta a ponta (intent → token → execução → `Executed`).
- [ ] Cada adaptador tem teste de TOCTOU para seu tipo de pré-condição.

---

### Etapa 8.7 — Override humano assinado · `P` · ⚠ TCB

**Depende de (recebe pronto).** 8.2; 7.1; 6.5.

**Fora do escopo.** quórum de assinaturas além de 2.

**Objetivo.** Mitigar a ameaça 15 (excesso de `UNKNOWN`) sem criar porta dos fundos silenciosa.

**Entregáveis.**
- `Override { h(a), reason, user_sig, scope: once }` → premissa `Overridden(a)`; a política decide se `Overridden` vale para cada classe (ex.: nunca para `destrutiva` sem 2 assinaturas).
- Overrides são destacados em toda renderização e contados na métrica de overrides.

**Critério de conclusão.**
- [ ] Override sem regra de política que o aceite não autoriza nada.
- [ ] Overrides aparecem no relatório de auditoria.

---

### Etapa 8.8 — Revogação de tokens · `P` · ⚠ TCB

**Depende de (recebe pronto).** 8.4; 5.10.

**Fora do escopo.** verificação online por consulta a cada execução.

**Entregáveis.**
- Validade curta (padrão), recusa por `r_v` antigo, lista de revogação por evento/checker publicada pelo kernel e puxada pelo gateway (com intervalo máximo de atraso documentado).

**Critério de conclusão.**
- [ ] Revogar um checker invalida, no gateway, tokens cujo `Verified` dependia só dele (dentro do atraso declarado).

---

### Etapa 8.9 ★ — Demo do regime operacional · `P`

**Depende de (recebe pronto).** 8.1–8.8; 6.3.

**Fora do escopo.** LLM (Fase 10): a demo é scriptada.

**Critério de conclusão.**
- [ ] Reproduz o fluxo completo da §6 (`db.drop(orders_v1)` com backup, conexões = 0, autorização do usuário) com `make demo-actions`.
- [ ] Mesma demo com cada premissa faltando (sem backup, com conexões, sem autorização) → recusa com explicação de qual premissa faltou.
- [ ] `logos-audit` verifica toda a demo.

---

## Fase 9 — Paráfrase determinística e autorização do usuário

### Etapa 9.1 — Renderizador determinístico AIR → português · `M` · ⚠ TCB (fidelidade)

**Depende de (recebe pronto).** 1.1, 1.2; 1.4 — parser (usado como base do parser da língua controlada).

**Fora do escopo.** outras línguas; elegância de redação.

**Objetivo.** §5: o renderizador está na TCB de fidelidade, mora do lado do kernel, e o usuário assina a string que ele produz.

**Entregáveis.**
- `render_pt(φ, V) -> String`: templates por símbolo (campo `render_pt` no vocabulário, ex.: `Owns(u,d)` ↦ "{u} é dono de {d}"), conectivos com regras fixas de parênteses/escopo ("para todo … vale que …"), sem ambiguidade de escopo (prefere redundância a elegância).
- **Língua controlada com parser.** A saída do renderizador pertence a uma gramática fixa (`spec/paraphrase-pt.md`), e existe `parse_pt(String, V) -> Formula`, do lado do kernel, na mesma TCB de fidelidade. **Injetividade por construção:** se `parse_pt(render_pt(φ)) == φ` para toda `φ`, duas fórmulas diferentes não podem ter a mesma paráfrase. Um teste que só procura colisões não garante isso.
- Bônus de segurança: o kernel pode reler a string que o usuário assinou e conferir que ela volta exatamente para `a` antes de aceitar a autorização (usado na 9.2).
- Versão do renderizador e da gramática no hash da paráfrase.

**Critério de conclusão.**
- [ ] Round-trip `parse_pt(render_pt(φ)) == φ` em property test com 100k fórmulas aleatórias até profundidade 6, e em todas as fórmulas de `spec/vectors/`.
- [ ] Gramática da língua controlada documentada; `parse_pt` com fuzz target (≥ 1h, como todo parser de TCB).
- [ ] Revisão humana de 50 paráfrases de políticas reais: nenhuma lida de forma errada (registrar a revisão).

---

### Etapa 9.2 — Autorização do usuário sobre o termo formal · `M`

**Depende de (recebe pronto).** 9.1; 6.5 — chave do usuário; 8.2 — termo `a`.

**Fora do escopo.** autenticação multiusuário.

**Entregáveis.**
- Fluxo: kernel produz `(a, render_pt(a))` → interface de confirmação (CLI/TUI do lado do kernel, nunca a do agente) → usuário assina `h(a ‖ paráfrase)` → `UserAuthorization { h(a), h(paráfrase), user_sig, at }` como premissa.
- WebAuthn/passkey: servidor local mínimo que mostra a paráfrase numa página e coleta a assinatura do autenticador.
- Verificação ES256 (P-256) em `logos-crypto`, no `alg` já reservado na 0.4 (nova dependência via ADR). A assinatura WebAuthn cobre `authenticatorData ‖ h(clientData)`; o `challenge` é `h(a ‖ paráfrase)` com o domínio de autorização, e o formato fica em `spec/`.
- Antes de registrar a autorização, o kernel confere `parse_pt(paráfrase assinada) == a` (Etapa 9.1).

**Critério de conclusão.**
- [ ] O agente não consegue alterar a string exibida (vem do kernel; teste de integração com agente que tenta injetar texto no fluxo).
- [ ] Autorização assinada para `a` não serve para `a'` ≠ `a`.
- [ ] Passkey funciona com pelo menos um autenticador real (ou emulador WebAuthn em CI).

---

### Etapa 9.3 — Confirmação de fidelidade (F3) · `P`

**Depende de (recebe pronto).** 9.1, 9.2.

**Fora do escopo.** selo (15.1); atestados F1/F2 (15.2, 15.4).

**Entregáveis.**
- Mesmo fluxo da 9.2, para fórmulas de claims: `FidelityAttestation { h(text), h(φ), h(V), method: UserConfirmed, attestor, sig }` sobre a paráfrase determinística.
- Registrado no ledger; consumido pelo selo na Fase 15.

**Critério de conclusão.**
- [ ] Mudar o vocabulário (versão incompatível) faz o atestado deixar de valer (teste; integra com 15.7).

---

## Fase 10 — Harness do agente (LLM, MCP, prover farm)

Objetivo da fase: um agente LLM real operando **só** através do kernel. Todo o código desta fase é não confiável: bugs aqui afetam progresso (liveness), nunca correção.

### Etapa 10.1 — Cliente do protocolo (`logos_client`) · `P`

**Depende de (recebe pronto).** 0.3 — canon Python; 5.2 — protocolo; 1.6.

**Fora do escopo.** MCP (10.3).

**Entregáveis.**
- Cliente Python async do socket do kernel, com encoder canônico (Etapa 0.3), tipos `pydantic` para mensagens, upload ao staging do CAS.

**Critério de conclusão.**
- [ ] Testes de integração contra o kernel real cobrindo todas as mensagens.

---

### Etapa 10.2 — Prover farm · `M`

**Depende de (recebe pronto).** 10.1; 2.4, 2.5, 4.6 — produtores.

**Fora do escopo.** camada 2 (adicionada na 14.2).

**Entregáveis.**
- Serviço que recebe `O` (bytes + hash) e corre em paralelo: camada 0 (Z3 modelos, model finder), camada 1 (grounder **não confiável** espelhado + CaDiCaL → LRAT); depois camada 2 (Fase 14).
- Política de portfólio: primeiro a terminar ganha; refutação e prova correm juntas; testemunha de consistência sempre pedida.
- Cache por `h(O)`.
- Isolado: sem acesso ao ledger nem a credenciais; só lê `O` e escreve no staging do CAS.

**Critério de conclusão.**
- [ ] Em 500 obrigações mistas, p50 de tempo até veredicto reportado; nenhuma resposta do farm é aceita sem checker (verificável no ledger).

---

### Etapa 10.3 — Servidor MCP · `M`

**Depende de (recebe pronto).** 10.1, 10.2; 8.2 — `ActionIntent`.

**Fora do escopo.** autenticação de clientes MCP remotos (v1 é local).

**Objetivo.** §12: o MCP expõe `propose`, `certify`, `action_intent`; ferramentas reais só atrás do gateway.

**Entregáveis.**
- Ferramentas MCP: `logos_vocab`, `logos_propose`, `logos_prove` (chama o farm e faz `CERTIFY`), `logos_refute`, `logos_assume`, `logos_action`, `logos_ledger_query`, `logos_explain`.
- Nenhuma ferramenta que execute efeitos diretamente.
- Respostas sempre incluem o status do ledger (não o que o LLM "acha").

**Critério de conclusão.**
- [ ] Um cliente MCP genérico consegue fazer o ciclo propose → prove → verified, e action → executed.
- [ ] Inventário: a lista de ferramentas MCP não contém nada que fale com o gateway diretamente.

---

### Etapa 10.4 — Formalizador (LLM → AIR) · `M`

**Depende de (recebe pronto).** 10.1; 1.2, 1.4.

**Fora do escopo.** aprovação automática de símbolos novos (sempre humana).

**Entregáveis.**
- Prompting + parsing: texto em português → proposta AIR usando o vocabulário vigente; proposta de novos símbolos (que viram pedido de extensão de vocabulário, aprovado por humano, com status `Glossed` por padrão).
- Loop de correção: erros de sort-check e contramodelos traduzidos voltam ao LLM.

**Critério de conclusão.**
- [ ] Em 100 frases de teste do domínio operacional, ≥ 80% viram propostas bem tipadas sem intervenção (métrica de progresso, não de correção).

---

### Etapa 10.5 — Loop do agente · `G`

**Depende de (recebe pronto).** 10.2–10.4; 8.9 — regime operacional; 9.2 — autorização.

**Fora do escopo.** suítes do AgentDojo (Fase 11).

**Entregáveis.**
- Harness principal: recebe tarefa → planeja → para cada passo: propõe claims, pede provas, reage a `REFUTED` (enfraquecer φ, declarar assumption, descartar, pedir humano — §3) e a `UNKNOWN`, submete `ActionIntent`s.
- Modos: intercalado (padrão), intercalado com especulação, pós-hoc (para comparação — Fase 16).
- Registro de enfraquecimentos (`Weakened { from: h(φ), to: h(φ') }`) para a métrica de informatividade (ameaça 14).
- Uso da API do Claude (modelo configurável; padrão: o mais capaz disponível).

**Critério de conclusão.**
- [ ] Agente completa 10 tarefas operacionais de exemplo (fs, db, http) ponta a ponta, com todo efeito mediado.
- [ ] Com um LLM "adversarial" scriptado (tenta pular o kernel, citar o não verificado, descrever ação falsa), nenhum efeito não autorizado ocorre.

---

### Etapa 10.6 — Resposta renderizada do ledger (taint) · `M`

**Depende de (recebe pronto).** 10.5; 3.3.

**Fora do escopo.** selo completo (15.8).

**Objetivo.** L3 e mecanismo 5 da §3.

**Entregáveis.**
- A resposta final é um documento estruturado: blocos `ledger_ref(h)` (renderizados a partir do ledger, com selo — versão simples agora, completa na Fase 15) e blocos `free_text` (do LLM) marcados visualmente como **não verificado**.
- O renderizador recebe o texto do LLM com marcações de citação; qualquer frase que afirme algo sem citação fica no bloco `free_text`.

**Critério de conclusão.**
- [ ] Nenhuma saída do agente mostra conteúdo não verificado sem marcação (teste com 50 respostas).

---

### Etapa 10.7 — TUI · `M`

**Depende de (recebe pronto).** 10.5, 10.6; 9.2 — fila de autorizações; 3.3 — consultas.

**Fora do escopo.** assinaturas dentro da TUI (a TUI só exibe; quem assina é o componente do lado do kernel).

**Entregáveis.**
- Crate `logos-tui` em Rust (Ratatui + crossterm), **fora da TCB**: conversa, DAG de claims com status, fila de autorizações (assinatura feita pela parte do kernel, não pela TUI), ledger ao vivo, refutações com contramodelo legível.
- Fontes de dados: ledger via `Query`/`Subscribe` do kernel (somente leitura); conversa e passos do agente via um socket de eventos exposto pelo harness Python (10.5), em frames canônicos.
- Strings de paráfrase e selos são exibidas exatamente como o kernel as produziu (com o hash ao lado); a TUI não reformata texto que o usuário vai assinar.

**Critério de conclusão.**
- [ ] Demo gravada da TUI executando a demo da Etapa 8.9 conduzida pelo LLM.

---

### Etapa 10.8 ★ — Telemetria e métricas · `P`

**Depende de (recebe pronto).** 10.5; métricas emitidas pelo kernel (5.x, 8.x).

**Fora do escopo.** dashboards de produção.

**Entregáveis.**
- Emissão das métricas do Apêndice D (OpenTelemetry ou JSONL), dashboards simples.

**Critério de conclusão.**
- [ ] Todas as métricas do Apêndice D marcadas como "Fase ≤ 10" aparecem numa execução da demo.

---

## Fase 11 — Integridade de justificativa (primeiro paper)

Objetivo da fase: a tese da §13 — **integridade de justificativa contra integridade de influência** — avaliada no AgentDojo contra CaMeL, FIDES e agente sem defesa, com a suíte de ataques de bit decisivo e a reversão de ações como resultado secundário.

### Etapa 11.1 — Extrator em quarentena · `M`

**Depende de (recebe pronto).** 6.6 — `Says`/`Extracted`; 10.5.

**Fora do escopo.** vocabulário das suítes (11.2).

**Entregáveis.**
- LLM em quarentena (sem ferramentas, sem acesso ao kernel) que lê conteúdo não confiável e devolve **proposições estruturadas** no vocabulário (`Pedido(x)`, `Menciona(y)`, …), nunca texto livre ao planejador.
- Saída vira `Extracted { source, h(content), φ, extractor }` (Etapa 6.6) — premissa `Says(source, φ)`.
- O planejador vê o `h` e a fórmula extraída, não o conteúdo bruto (ou vê o conteúdo marcado como tainted, conforme modo de experimento).

**Critério de conclusão.**
- [ ] Em 200 e-mails de teste com injeções, nenhuma instrução injetada aparece como `Fact`; todas ficam como `Says(email, …)`.

---

### Etapa 11.2 — Políticas de endosso semântico para as suítes do AgentDojo · `G`

**Depende de (recebe pronto).** 7.1–7.5; 8.1; 11.1.

**Fora do escopo.** suítes fora do AgentDojo.

**Entregáveis.**
- Vocabulário e contratos para as suítes `workspace`, `banking`, `slack`, `travel`.
- Classes de ação por suíte (`responder`, `criar_tarefa`, `transferir`, `destrutiva`, `exfiltrar`…) e políticas no estilo da §13: o que cada fonte pode justificar para cada classe.
- Documento de custo de autoria (horas, linhas de política por suíte) — dado do paper (§13, "Custo de autoria").

**Critério de conclusão.**
- [ ] Toda ferramenta do AgentDojo tem contrato e classe.
- [ ] Revisão cruzada das políticas por outra pessoa (registrada).

---

### Etapa 11.3 — Integração com o AgentDojo · `G`

**Depende de (recebe pronto).** 11.2; 8.4–8.6; 10.5.

**Fora do escopo.** baselines (11.4).

**Entregáveis.**
- O ambiente do AgentDojo encapsulado como **servidor de ferramentas** que só aceita chamadas vindas do socket do gateway (o gateway em Rust verifica o token e repassa).
- Pipeline LOGOS como "defense" do AgentDojo: planejador + extrator em quarentena + kernel + gateway.
- Execução reprodutível: seeds, versão do modelo, versão das políticas, todos registrados.

**Critério de conclusão.**
- [ ] Roda o benchmark completo (utilidade sem ataque, utilidade sob ataque, ASR) com um comando.
- [ ] Cada execução produz um ledger auditável por `logos-audit`.

---

### Etapa 11.4 — Baselines · `G`

**Depende de (recebe pronto).** 11.3 — harness comum.

**Fora do escopo.** defesas além das listadas.

**Entregáveis.**
- Sem defesa; CaMeL (implementação oficial, se disponível; senão reimplementação documentada); FIDES (idem); opcionalmente defesas de prompt (spotlighting, sandwich) como referência.
- Mesmo modelo de LLM em todos.

**Critério de conclusão.**
- [ ] Números dos baselines reproduzem os publicados dentro de margem documentada (ou a diferença é explicada).

---

### Etapa 11.5 — Suíte de ataques de bit decisivo · `G`

**Depende de (recebe pronto).** 11.3, 11.4.

**Fora do escopo.** ataques fora do modelo de bit decisivo (decomposição é a 11.6).

**Objetivo.** §13, pergunta 3: injeções que só precisam controlar uma resposta booleana/enum.

**Entregáveis.**
- ≥ 50 cenários em que a decisão de ação depende de um bit extraído de conteúdo não confiável e o dano está nesse bit (ex.: "o e-mail confirma o pagamento?" → transferir).
- Para cada cenário: versão para endosso por capacidade (FIDES/CaMeL) e para endosso semântico (LOGOS).
- Métricas: ASR por defesa, e para o LOGOS, qual premissa sustentou cada ação.

**Critério de conclusão.**
- [ ] Suíte publicada no repositório com harness e resultados reproduzíveis.
- [ ] Resultado (positivo **ou negativo**) escrito com intervalo de confiança.

---

### Etapa 11.6 — Ataques de decomposição · `M`

**Depende de (recebe pronto).** 11.3; 7.5 — trajetórias.

**Fora do escopo.** ataques de bit decisivo (11.5).

**Entregáveis.**
- Cenários em que a ação proibida é decomposta em permitidas (§6, §10 ameaça 12), com políticas de trajetória (Etapa 7.5).

**Critério de conclusão.**
- [ ] ASR de decomposição medido para todas as defesas.

---

### Etapa 11.7 — Reversão de ações por dependência · `G`

**Depende de (recebe pronto).** 5.9 — cascata; 8.5 — `Executed`; 8.6 — compensações por contrato.

**Fora do escopo.** UI de incidente para operadores (23.3).

**Objetivo.** §14: "o que o agente fez **porque** acreditou em F37?".

**Entregáveis.**
- `logos retract F37 --reason …` → lista conclusões invalidadas, ações executadas que dependiam de F37 (via `Executed → token → Verified.used → …`), ações com rota independente.
- Proposta de compensação por contrato; compensações passam pelo gateway como qualquer ação (precisam de política, token, etc.).
- Saída exatamente no formato do exemplo da §14.

**Critério de conclusão.**
- [ ] Cenário da §14 (backup falso → `db.drop` + `notify`) reproduzido: as duas ações listadas, a independente excluída, compensações executadas via gateway.
- [ ] Teste com 1000 ações aleatórias: conjunto de afetadas = conjunto de referência calculado por busca ingênua.

---

### Etapa 11.8 — Estudo de auditabilidade · `M`

**Depende de (recebe pronto).** 11.7; 10.7 — TUI.

**Fora do escopo.** estudo sobre o selo (16.3).

**Entregáveis.**
- Protocolo: analistas recebem incidentes (ação danosa) com e sem certificado/ledger; mede-se tempo até achar a premissa causadora (§13, pergunta 4).
- Aprovação ética se envolver participantes externos.

**Critério de conclusão.**
- [ ] ≥ 10 participantes, resultados com análise estatística.

---

### Etapa 11.9 — Medição da TCB e de custo · `M`

**Depende de (recebe pronto).** 0.2 — contagem da TCB; 12.4 — proporção verificada; `bench/`.

**Fora do escopo.** camadas 2–4.

**Entregáveis.**
- Tabela final: LOC da TCB por propriedade e proporção verificada (Kani/Verus/CakeML), comparando a rota A (solver confiável) com a arquitetura em camadas (§11, item 2 da tese em três partes).
- Latência por etapa (propose, prova, check, mint, execução), tamanho de certificados, taxa de `UNKNOWN`.

**Critério de conclusão.**
- [ ] Números gerados por script a partir de `tcb/` e `bench/`; reproduzíveis.

---

### Etapa 11.10 ★★ — Paper 1 · `G`

**Depende de (recebe pronto).** 11.3–11.9; 12.2, 12.3 — teoremas; 13.1–13.3 — hardening mínimo.

**Fora do escopo.** regime epistêmico (paper 2).

**Entregáveis.**
- Artigo (alvo: USENIX Security / CCS / SaTML) com: tese de integridade de justificativa, arquitetura, garantias (com os teoremas da Fase 12), avaliação 11.3–11.9, limitações fundamentais (L1–L3, ameaças 1, 2, 3, 13, 16) como limitações e não trabalho futuro.
- Artefato reprodutível (container + scripts).

**Critério de conclusão.**
- [ ] Artefato passa por avaliação de artefato simulada (alguém de fora roda tudo seguindo o README).
- [ ] Submetido.

---

## Fase 12 — Modelo formal mecanizado

Roda em paralelo a partir do fim da Fase 5. Objetivo: transformar a arquitetura de diagrama em afirmação verificável (§11.1).

### Etapa 12.1 — Modelo da máquina de estados em Lean 4 · `G`

**Depende de (recebe pronto).** 5.1 — `spec/state-machine.md`.

**Fora do escopo.** ligação com o código Rust (12.4).

**Entregáveis.**
- Tipos: eventos, ledger como lista, estado do kernel, mensagens do agente (como entradas arbitrárias — agente adversarial), checkers como funções abstratas com hipótese de correção (`checker_sound : accepts c O → valid O`).
- Função de passo `step : State → Input → State × Output` espelhando `spec/state-machine.md`.

**Critério de conclusão.**
- [ ] Modelo compila; todas as transições da tabela da Etapa 5.1 estão representadas (checado por script que compara a tabela com o modelo).

---

### Etapa 12.2 — Teorema de mediação completa · `G`

**Depende de (recebe pronto).** 12.1; especificação do gateway e dos tokens (8.3–8.5).

**Fora do escopo.** camadas 2–4 (checkers abstraídos por hipótese de correção).

**Enunciado (§11.1).** Para toda sequência de entradas, todo `Executed` no ledger tem uma cadeia válida: token assinado pelo kernel → `Verified` → obrigação gerada pelo kernel → certificado aceito por checker não revogado → premissas com origem registrada (política assinada, fatos de emissores confiáveis, testemunha de consistência).

**Critério de conclusão.**
- [ ] Prova sem `sorry` e sem axiomas além dos declarados (hipóteses de correção dos checkers e de não forjabilidade da assinatura, explícitas).

---

### Etapa 12.3 — Teorema de integridade de justificativa · `G`

**Depende de (recebe pronto).** 12.2; 6.6 — `Says`; 7.1 — semântica de política.

**Fora do escopo.** propriedades de confidencialidade.

**Enunciado (§13).** Para toda ação executada `a`, existe certificado aceito para `Pol_v ∪ Γ ⊢ Allowed(a)` onde `Γ` = premissas efetivamente usadas, e cada `p ∈ Γ` tem origem cujo nível de confiança satisfaz o requisito de `Pol_v` para a classe de `a`.

**Critério de conclusão.**
- [ ] Prova sem `sorry`.
- [ ] Contraexemplo formal (no modelo) de que **não-interferência não vale** — o teorema é sobre justificativa, não influência (deixa a distinção explícita no paper).

---

### Etapa 12.4 — Ligação modelo ↔ código (Kani / Verus) · `GG`

**Depende de (recebe pronto).** 12.1; código de 1.5, 3.2 e 8.4.

**Fora do escopo.** verificação de checkers externos (cake_lpr já é verificado; Carcara não entra).

**Entregáveis.**
- Kani: harnesses para invariantes locais (decoder canônico total e estrito; `generate` nunca aceita citação não citável; appender sempre encadeia; gateway nunca executa com token inválido).
- Verus: especificação e prova dos módulos `logos-obligation`, `Ledger::append`, verificação de token.
- Documento de correspondência: cada função do modelo Lean ↔ função Rust, com o tipo de evidência (prova, teste diferencial, revisão).

**Critério de conclusão.**
- [ ] Harnesses Kani passam em CI.
- [ ] Pelo menos o gerador de obrigações verificado em Verus.
- [ ] Proporção verificada da TCB reportada pelo `tcb/count.sh`.

---

### Etapa 12.5 — Teorema de honestidade do selo · `M`

**Depende de (recebe pronto).** **Fase 15** (15.1 — `Seal` e renderização); 12.1.

**Fora do escopo.** propriedades de calibração (15.6).

**Enunciado.** Nenhum selo renderizado afirma nível de validade, consistência, fidelidade ou grounding maior que o sustentado pelo ledger, nem omite premissas em aberto.

**Critério de conclusão.**
- [ ] Prova sem `sorry` sobre o modelo do selo e da função de renderização.

---

## Fase 13 — Hardening, chaves e transparência

O mínimo de 13.1–13.3 deve estar pronto antes da submissão do paper 1.

### Etapa 13.1 — Chaves do kernel em HSM/TEE e rotação · `M` · ⚠ TCB

**Depende de (recebe pronto).** 0.4 — trait `Signer`; 3.2 — appender; 8.3 — mint.

**Fora do escopo.** TEE obrigatório (opcional).

**Entregáveis.**
- Backend de assinatura plugável (novas implementações da trait `Signer` da 0.4): arquivo (dev, `FileSigner`), PKCS#11 (HSM/YubiHSM/SoftHSM em CI), TPM ou TEE (opcional).
- Rotação: `KeyRotated { old, new, sig_old, sig_new }`; auditor aceita cadeia com rotações.

**Critério de conclusão.**
- [ ] Kernel rodando com SoftHSM em CI; a chave privada nunca existe em memória do processo.
- [ ] Rotação no meio de um ledger verifica no auditor.

---

### Etapa 13.2 — Log de transparência das cabeças do ledger · `M`

**Depende de (recebe pronto).** 3.6 — auditor; 13.1.

**Fora do escopo.** rede pública de testemunhas (v1 usa testemunhas configuradas).

**Objetivo.** Ameaça 9 (kernel mostra histórias diferentes a partes diferentes).

**Entregáveis.**
- Publicação periódica de `(seq, h(head))` assinada num log de transparência (implementação própria compatível com o modelo do Certificate Transparency / Trillian-Tessera, ou Sigstore Rekor).
- Testemunhas externas (cossignatura); `logos-audit` confere consistência entre o ledger local e o log.

**Critério de conclusão.**
- [ ] Teste de bifurcação: kernel adulterado mostrando duas histórias → detectado pelo auditor via log.

---

### Etapa 13.3 — Campanha de fuzzing e revisão da TCB · `M`

**Depende de (recebe pronto).** crates de TCB até a Fase 8.

**Fora do escopo.** auditoria externa paga.

**Entregáveis.**
- Fuzzing contínuo (OSS-Fuzz style ou cluster local) de todos os decoders e checkers; diferencial checker próprio × cake_lpr; diferencial avaliador × LRAT.
- Revisão de código externa da TCB (outra pessoa, checklist escrito).

**Critério de conclusão.**
- [ ] 24h de fuzzing por alvo sem achados abertos.
- [ ] Relatório de revisão com todos os itens resolvidos ou aceitos com justificativa.

---

### Etapa 13.4 — Reprodutibilidade de build · `P`

**Depende de (recebe pronto).** 5.12 — kernel; 8.4 — gateway.

**Fora do escopo.** builds reprodutíveis do Python (fora da TCB).

**Entregáveis.**
- Builds reprodutíveis do kernel e do gateway (Nix ou Docker com toolchain fixa); hash dos binários publicado; o próprio kernel se identifica pelo hash no `Genesis`.

**Critério de conclusão.**
- [ ] Duas máquinas produzem binários idênticos.

---

## Fase 14 — Camada 2: primeira ordem via Alethe

Objetivo da fase: `VERIFIED` para argumentos de primeira ordem fora de domínios finitos, com cvc5 produzindo provas Alethe e Carcara checando. Início do item 3 da §12.

### Etapa 14.1 — Obrigação emitida diretamente em SMT-LIB canônico · `M` · ⚠ TCB

**Depende de (recebe pronto).** 1.5 — `Obligation`; 4.3 — padrão de encoder na TCB.

**Fora do escopo.** teorias aritméticas (v1: UF + quantificadores).

**Objetivo.** P2 para a camada 2: o Carcara confere contra um problema SMT-LIB; para não ter tradutor fora do hash, **o gerador de obrigações emite o problema SMT-LIB canônico como parte da obrigação**, e é esse texto que é hasheado e checado.

**Entregáveis.**
- `emit_smtlib(O) -> bytes` determinístico (sorts → `declare-sort`, símbolos → `declare-fun`, premissas → `assert` nomeados com o hash da premissa, objetivo negado → `assert`), na TCB junto do gerador.
- Evento registra `h(O)` e `h(smt)`; auditor refaz a emissão.

**Critério de conclusão.**
- [ ] Emissão determinística (hash estável em CI, 2 arquiteturas).
- [ ] Teste semântico: nas obrigações de domínio finito, `emit_smtlib` e a camada 1 concordam em 100% dos 2k casos.

---

### Etapa 14.2 — cvc5 com saída Alethe (fora da TCB) · `P`

**Depende de (recebe pronto).** 14.1; 10.2 — farm.

**Fora do escopo.** outros produtores (Vampire: rota D, Apêndice E).

**Entregáveis.**
- Produtor no farm; Z3 como acelerador (se Z3 diz `unsat` rápido, pede-se ao cvc5 a prova; se Z3 diz `sat`, o modelo vai à camada 0).

**Critério de conclusão.**
- [ ] Em 300 argumentos de primeira ordem de teste, taxa de certificado obtido reportada.

---

### Etapa 14.3 — Integração do Carcara · `M` · ⚠ TCB

**Depende de (recebe pronto).** 14.1; 5.4 — registry de checkers.

**Fora do escopo.** regras Alethe confiadas (`hole`, `trust`): sempre recusadas.

**Entregáveis.**
- Carcara identificado por hash de build reprodutível; execução isolada com limites; política de regras permitidas (recusar provas com `hole`/regras confiadas).
- Clausificação/skolemização dentro da prova (regras Alethe correspondentes); obrigações cuja prova depende de pré-processamento não certificado → recusadas.

**Critério de conclusão.**
- [ ] Prova contendo regra não checada (`hole`, `trust`) → `Rejected`.
- [ ] Suíte de provas corrompidas → 100% rejeitadas.

---

### Etapa 14.4 — Dependências a partir das provas Alethe · `P` · ⚠ TCB

**Depende de (recebe pronto).** 14.3; 4.7 — padrão de `used`.

**Fora do escopo.** dependências semânticas (só sintáticas, pela prova).

**Entregáveis.**
- `used` = conjunto de `assume` alcançáveis a partir da cláusula vazia, mapeados pelos nomes dos `assert` (hash da premissa).

**Critério de conclusão.**
- [ ] Mesmo teste da Etapa 4.7, para a camada 2.

---

### Etapa 14.5 ∥ — Segundo checker (reconstrução em Isabelle/HOL) · `G`

**Depende de (recebe pronto).** 14.3; 5.10 — revogação e recheck.

**Fora do escopo.** Isabelle como checker principal (só segundo checker, salvo o Apêndice E).

**Entregáveis.**
- Pipeline opcional que reconstrói provas Alethe em Isabelle (trabalho ITP 2025), registrado como segundo checker do formato `alethe`, para recheck e revogação.

**Critério de conclusão.**
- [ ] Revogar o Carcara e rechecar tudo com o Isabelle funciona no cenário da Etapa 5.10.

---

### Etapa 14.6 — Testemunhas de consistência para teorias abertas · `P`

**Depende de (recebe pronto).** 14.1; 2.4 — produtor de modelos.

**Fora do escopo.** modelos infinitos.

**Entregáveis.**
- Busca de modelos finitos (camada 0) para `Γ` de primeira ordem; se não houver modelo finito pequeno, `ConsistencyUnknown` (visível no selo como "possivelmente vazio").

**Critério de conclusão.**
- [ ] Taxa de `ConsistencyUnknown` reportada no benchmark de argumentos.

---

## Fase 15 — Selo de cinco campos e fidelidade

Objetivo da fase: fidelidade como dimensão **tipada** do resultado (§8), a contribuição mais forte do projeto segundo a §11.

### Etapa 15.1 ★ — Tipo `Seal` e renderização total · `M` · ⚠ TCB (fidelidade)

**Depende de (recebe pronto).** 5.5 — consistência; 9.3 — F3; 3.3 — consultas.

**Fora do escopo.** atestados F2/F1 (15.2+).

**Entregáveis.**
- `Seal { validity, vacuity, fidelity, grounding, open }` construído só pelo kernel a partir do ledger.
- Única API de renderização: `render(seal: &Seal, …)` — recebe o selo inteiro. **Não existe** função que renderize só a validade (P7).
- Rótulo-resumo = mínimo entre as dimensões; ícone verde só quando todas no topo.
- Formato de exibição como no exemplo da §8 (`C7 SEGUE · condicional …`).

**Critério de conclusão.**
- [ ] **Invariante P7**: teste compile-fail + inspeção de API pública: nenhum caminho expõe `validity` como string/ícone isolado.
- [ ] Teste: claim `Verified` com uma premissa F1 → resumo nunca "verde".

---

### Etapa 15.2 — Atestados de fidelidade F0–F4 · `M` · ⚠ TCB (fidelidade)

**Depende de (recebe pronto).** 15.1; 9.3.

**Fora do escopo.** calibração (15.6).

**Entregáveis.**
- `FidelityAttestation { h(text), h(φ), h(V), method: F4|F3|F2|F1, attestor, sig, calibration? }`.
- F4 automático para políticas e contratos (autoria formal); F3 da Etapa 9.3; F2 da 15.4; F1 do LLM revisor (assinado pela chave do **serviço** de revisão, rotulado como LLM).
- Nível de fidelidade de um claim = mínimo sobre premissas `used` + conclusão.

**Critério de conclusão.**
- [ ] Teste: claim derivado de 3 premissas F3 e 1 F1 → fidelidade F1.

---

### Etapa 15.3 — Grounding agregado por símbolo · `P`

**Depende de (recebe pronto).** 15.1; 1.2 — status de grounding.

**Fora do escopo.** grounding de símbolos de outros agentes (17.x).

**Entregáveis.**
- `grounding` do selo = mínimo do status (`Defined > Anchored > Glossed`) entre os símbolos usados nas fórmulas `used`, expandindo definições.

**Critério de conclusão.**
- [ ] Exemplo da §8 (`Causa/2`, `Pessoal/1` glosados) reproduzido.

---

### Etapa 15.4 — F2: dupla formalização com equivalência provada · `M`

**Depende de (recebe pronto).** 15.2; 4.6 / 14.2 — provas de equivalência (domínio finito ou FOL).

**Fora do escopo.** escolha automática entre formalizações.

**Entregáveis.**
- Dois formalizadores independentes (modelos/prompts de famílias diferentes) → `φ1`, `φ2`; obrigação `⊢ φ1 ↔ φ2` gerada pelo kernel e certificada.
- Se não equivalentes: contramodelo distintivo → explicado em português ("as leituras divergem quando …") → base da 15.5.
- Rótulo honesto: F2 registra que a independência é suposição.

**Critério de conclusão.**
- [ ] Em 100 frases ambíguas de teste, as não equivalências produzem cenários distintivos checados.

---

### Etapa 15.5 — Desambiguação por cenários distintivos · `M`

**Depende de (recebe pronto).** 15.4; 10.7 — TUI; 9.3 — fluxo de assinatura.

**Fora do escopo.** geração de cenários sem par de formalizações (sempre parte de φ1 ≠ φ2).

**Entregáveis.**
- Interface que mostra ao usuário o cenário concreto em que as leituras divergem e pergunta qual ele quis (estilo TiCoder/Monty); resposta assinada vira F3 para a leitura escolhida.

**Critério de conclusão.**
- [ ] Fluxo completo na TUI para 10 frases de exemplo.

---

### Etapa 15.6 — Calibração empírica de F1/F2 · `M`

**Depende de (recebe pronto).** 15.2.

**Fora do escopo.** auditoria por pessoas externas ao projeto (opcional).

**Entregáveis.**
- Amostragem periódica de fórmulas aceitas por método para auditoria humana; taxa de infidelidade com intervalo de Wilson, versionada; o selo exibe "método com taxa de erro medida X% [IC]".

**Critério de conclusão.**
- [ ] Primeira rodada de calibração com ≥ 200 itens auditados por método.

---

### Etapa 15.7 — Invalidação por mudança de vocabulário · `P` · ⚠ TCB

**Depende de (recebe pronto).** 15.2; 1.2 — versionamento de vocabulário.

**Fora do escopo.** migração automática de atestados entre versões.

**Entregáveis.**
- Nova versão incompatível de `V` → evento `VocabChanged` → todos os atestados de fidelidade com o `h(V)` antigo perdem efeito; selos recalculados.

**Critério de conclusão.**
- [ ] Teste: mudar a glosa de `Pessoal/1` rebaixa a fidelidade de todos os claims que o usam.

---

### Etapa 15.8 — Resposta final com selos completos · `P`

**Depende de (recebe pronto).** 15.1; 10.6.

**Fora do escopo.** novos formatos de saída.

**Entregáveis.**
- A renderização da Etapa 10.6 passa a usar o `Seal` completo para cada bloco `ledger_ref`.

**Critério de conclusão.**
- [ ] Nenhum bloco verificado aparece sem os cinco campos.

---

## Fase 16 — Regime epistêmico: benchmark e estudo com usuários

### Etapa 16.1 — Benchmark de argumentos · `G`

**Depende de (recebe pronto).** nenhuma técnica (∥, pode começar cedo); 1.4 — sintaxe para as formalizações de referência.

**Fora do escopo.** argumentos em outras línguas.

**Entregáveis.**
- Corpus de argumentos em português (filosóficos, jurídicos, científicos), com: gabarito de validade, formalizações de referência, armadilhas de formalização (ambiguidade de escopo, quantificadores implícitos), e argumentos longos com erro plantado em passo intermediário.

**Critério de conclusão.**
- [ ] ≥ 300 argumentos, cada um revisado por duas pessoas.

---

### Etapa 16.2 — Intercalado × pós-hoc · `M`

**Depende de (recebe pronto).** 16.1; 10.5 — modos do agente; 14.x — FOL certificada.

**Fora do escopo.** estudo com usuários (16.3).

**Entregáveis.**
- Experimento com o agente nos três modos (Etapa 10.5), medindo profundidade de propagação de erro, latência, tarefas abandonadas, taxa de `UNKNOWN`, enfraquecimentos (informatividade).

**Critério de conclusão.**
- [ ] Resultados com IC e análise do custo de latência (§3, "pergunta empírica").

---

### Etapa 16.3 — Estudo com usuários sobre o selo · `GG`

**Depende de (recebe pronto).** 15.1–15.6; 16.1.

**Fora do escopo.** estudo de auditabilidade operacional (11.8).

**Objetivo.** O resultado "mais publicável e mais arriscado" da §11: o selo de cinco campos reduz a aceitação de conclusões válidas sobre formalizações infiéis?

**Entregáveis.**
- Protocolo pré-registrado, aprovação ética, condições: selo binário "verificado" × selo de cinco campos × sem selo; tarefas com formalizações fiéis e infiéis.
- Análise estatística pré-especificada.

**Critério de conclusão.**
- [ ] Pré-registro publicado; estudo executado; resultado (positivo ou negativo) analisado e escrito.

---

## Fase 17 — Portable Verified Claims (multi-agente)

### Etapa 17.1 — Formato PVC · `M` · ⚠ TCB

**Depende de (recebe pronto).** 4.7, 14.4 — `used`; 15.2 — fidelidade; 0.5 — CAS.

**Fora do escopo.** premissas ocultas por compromisso (17.6, só nota de design).

**Entregáveis.**
- `PVC` exatamente como a §7 (claim, sequent extraído do certificado, premissas recursivas como DAG Merkle, certificado endereçado por conteúdo, consistência, fidelidade, proveniência).
- `logos export <claim>` gera o PVC e o pacote de objetos do CAS.

**Critério de conclusão.**
- [ ] Vetores canônicos; PVC com sub-PVCs compartilhados não duplica objetos.

---

### Etapa 17.2 — Lógica de atribuição `says` e delegação · `M` · ⚠ TCB

**Depende de (recebe pronto).** 6.2, 6.6; 17.1.

**Fora do escopo.** cadeias de delegação longas.

**Entregáveis.**
- `Says(A, φ)` estendido para agentes (KeyId do kernel de A); regras de delegação na política local de B: `Says(A, φ) ∧ Trusted(A, topic(φ)) → φ`.
- `topic` como função do vocabulário (anotação por símbolo).

**Critério de conclusão.**
- [ ] Teste: B só descarrega premissa de A quando a política local de B tem a regra de confiança correspondente.

---

### Etapa 17.3 — Algoritmo de importação em B · `M` · ⚠ TCB

**Depende de (recebe pronto).** 17.1, 17.2; 5.4 — checkers.

**Fora do escopo.** transporte de rede (v1: arquivos/bundles).

**Entregáveis.**
- Os seis passos da §7: recálculo local da obrigação; recheck com o checker **de B**; checagem da testemunha; recursão nas premissas (PVC → recursão; fato → raízes e janela de B; assumption → hipótese); avaliação de fidelidade pela política de B; evento `Imported { φ, discharged, pending }`.
- Cache `(h(π), h(checker)) → aceito`.
- Orçamento por PVC (DoS); excesso → `Unknown`.

**Critério de conclusão.**
- [ ] Dois kernels independentes (chaves, ledgers, configs diferentes) trocam PVCs; B recusa PVC com certificado adulterado mesmo com assinatura válida de A.
- [ ] Assinatura do kernel de A irrelevante para validade (teste: PVC re-assinado por terceiro continua importável se o certificado é bom).

---

### Etapa 17.4 — Premissas não descarregadas na interface (P8) · `P`

**Depende de (recebe pronto).** 17.3; 15.1.

**Fora do escopo.** UI dedicada.

**Entregáveis.**
- Selo de claims importados destaca `open` = premissas de A não descarregadas por B; nenhuma confiança implícita no protocolo.

**Critério de conclusão.**
- [ ] **Invariante P8**: teste — remover a regra `Trusted(A, …)` da política de B rebaixa todos os importados de A a condicionais.

---

### Etapa 17.5 — Desacordo como subconjunto inconsistente mínimo · `M`

**Depende de (recebe pronto).** 17.3; 4.4 — LRAT; 2.3 — modelos.

**Fora do escopo.** MUS em FOL aberta.

**Entregáveis.**
- Dado `Γ_A ⊢ C` e `Γ_D ⊢ ¬C`, kernel calcula um MUS de `Γ_A ∪ Γ_D` (produtor não confiável — ex.: MUS extractor sobre a CNF — com o resultado checado: insatisfatibilidade via LRAT e minimalidade via modelos de cada subconjunto com um elemento removido).
- Saída: "o desacordo está nas premissas P4 e P19".

**Critério de conclusão.**
- [ ] MUS checado (inconsistência + minimalidade) em 100 pares de teste.

---

### Etapa 17.6 — Obsolescência e privacidade · `M`

**Depende de (recebe pronto).** 17.3; 6.1.

**Fora do escopo.** computação verificável (rota H): só nota de design.

**Entregáveis.**
- Política de B para idade máxima de fatos importados; reimportação quando fatos expiram.
- Nota de design (sem implementação completa) sobre premissas comprometidas e computação verificável (rota H), com a decisão de deixar para pesquisa futura.

**Critério de conclusão.**
- [ ] PVC com fato expirado segundo B importa como condicional.

---

## Fase 18 — Debate checkável (segundo paper)

### Etapa 18.1 — Protocolo de debate · `G`

**Depende de (recebe pronto).** 17.1–17.5.

**Fora do escopo.** juiz automático.

**Entregáveis.**
- Debatedores publicam argumentos como PVCs; ataques válidos = contramodelo checado, MUS checado, ou ataque a premissa específica (nova evidência atestada / pedido de descarregamento).
- Kernel decide todas as inferências; o juiz humano recebe só premissas-folha em disputa.

**Critério de conclusão.**
- [ ] Debate entre dois agentes sobre 20 tópicos produz, para cada um, a lista de premissas em disputa (sem inferências para o juiz).

---

### Etapa 18.2 — Interface do juiz · `M`

**Depende de (recebe pronto).** 18.1; 9.1; 15.1; 10.7.

**Fora do escopo.** interface web.

**Entregáveis.**
- Tela "o desacordo está em P4 e P19; qual você aceita?" com paráfrases determinísticas e selos.

**Critério de conclusão.**
- [ ] Tempo médio de julgamento medido num piloto.

---

### Etapa 18.3 ★★ — Experimentos e paper 2 · `GG`

**Depende de (recebe pronto).** 18.1, 18.2; 16.1.

**Fora do escopo.** regime operacional.

**Entregáveis.**
- Comparação com debate em texto livre: acurácia do juiz, tempo, robustez a debatedor desonesto; selo de cinco campos e desambiguação por cenários como componentes.
- Revisão da literatura de debate com verificação formal (pendência declarada na §14).

**Critério de conclusão.**
- [ ] Paper submetido (comunidade de IA / supervisão escalável).

---

## Fase 19 — Argumentação derrotável (ASPIC+)

### Etapa 19.1 — Representação ASPIC+ e argumentação abstrata · `M`

**Depende de (recebe pronto).** 1.1; 7.1.

**Fora do escopo.** semânticas além de grounded e estável.

**Entregáveis.**
- Regras estritas e derrotáveis, preferências, construção do grafo de ataques (produtor não confiável), grafo serializado canonicamente como obrigação.

**Critério de conclusão.**
- [ ] Exemplos clássicos (Tweety, Nixon diamond) produzem os grafos esperados.

---

### Etapa 19.2 — Checkers por semântica · `M` · ⚠ TCB

**Depende de (recebe pronto).** 19.1; 4.4 — LRAT.

**Fora do escopo.** semântica preferida cética.

**Entregáveis.**
- Grounded: checker recalcula (polinomial).
- Estável crédula: testemunha = extensão, checada.
- Estável cética: codificação SAT canônica → LRAT (camada 1).

**Critério de conclusão.**
- [ ] Teste diferencial contra um solver de argumentação de referência (ex.: µ-toksia, ASPARTIX) em 1k frameworks.

---

### Etapa 19.3 — Integração com o selo · `P`

**Depende de (recebe pronto).** 19.2; 15.1.

**Fora do escopo.** —

**Entregáveis.**
- Validade passa a ter variantes `Accepted(grounded|credulous|skeptical)`, renderizadas com a semântica explícita.

**Critério de conclusão.**
- [ ] Nenhuma aceitação derrotável é renderizada como "segue" dedutivo.

---

## Fase 20 — Camada 3: Lean

### Etapa 20.1 — Obrigação emitida como enunciado Lean canônico · `M` · ⚠ TCB

**Depende de (recebe pronto).** 1.5 — `Obligation`.

**Fora do escopo.** tradução de teorias aritméticas além do necessário para os exemplos.

**Entregáveis.**
- Gerador emite o enunciado Lean (tipo do teorema) a partir de `O`, em formato de export do kernel Lean; esse é o objeto hasheado (P2).

**Critério de conclusão.**
- [ ] Emissão determinística; correspondência semântica testada contra a camada 1 em obrigações finitas.

---

### Etapa 20.2 — Checkers: kernel Lean + nanoda (+ lean4lean) · `M` · ⚠ TCB

**Depende de (recebe pronto).** 20.1; 5.4.

**Fora do escopo.** kernels Lean além de dois.

**Entregáveis.**
- Checagem do termo exportado por dois kernels independentes; verificação de que o tipo do termo é **exatamente** o enunciado hasheado; allowlist de axiomas (`propext`, `Quot.sound`, `Classical.choice` conforme política); recusa de `sorryAx` e axiomas novos.

**Critério de conclusão.**
- [ ] Prova que usa axioma fora da allowlist → `Rejected`.
- [ ] Discordância entre os dois kernels → `Unknown` + alerta.

---

### Etapa 20.3 — Produtor LLM + táticas (fora da TCB) · `M`

**Depende de (recebe pronto).** 20.2; 10.2.

**Fora do escopo.** treino de modelos.

**Entregáveis.**
- Agente que escreve provas Lean com feedback do elaborador.

**Critério de conclusão.**
- [ ] Benchmark de matemática pequeno (ex.: subconjunto do miniF2F) com taxa de sucesso reportada.

---

## Fase 21 — Camada 4: HOL, modal e deôntica

### Etapa 21.1 — Embedding semântico (estilo LogiKEy) · `GG`

**Depende de (recebe pronto).** 1.1.

**Fora do escopo.** lógicas além de K e SDL na primeira versão.

**Entregáveis.**
- Embedding de lógicas modais/deônticas em HOL; identificador de lógica versionado (`HOL-modal-K/v1`, `HOL-SDL/v1`, …).

**Critério de conclusão.**
- [ ] Paradoxos deônticos clássicos (Chisholm) formalizados e checados.

---

### Etapa 21.2 — Checker HOL Light / Candle · `G` · ⚠ TCB

**Depende de (recebe pronto).** 21.1; 5.4.

**Fora do escopo.** Isabelle (kernel maior; só se Candle/HOL Light inviável).

**Entregáveis.**
- Integração do kernel HOL Light (ou Candle, verificado) como checker identificado por hash; obrigação emitida como termo HOL canônico.

**Critério de conclusão.**
- [ ] Provas modais de exemplo checadas; suíte de provas corrompidas rejeitada.

---

## Fase 22 — Framework lógico único

### Etapa 22.1 — Estudo de viabilidade: Metamath Zero × Dedukti · `G`

**Depende de (recebe pronto).** 4.5, 14.3, 20.2, 21.2 (conhecimento dos formatos).

**Fora do escopo.** implementação.

**Entregáveis.**
- Relatório: custo de tradução de LRAT, Alethe, Lean e HOL para cada framework; tamanho do verificador; maturidade.

**Critério de conclusão.**
- [ ] ADR escolhendo (ou adiando, com justificativa) o framework.

---

### Etapa 22.2 — Obrigações emitidas no framework · `GG` · ⚠ TCB

**Depende de (recebe pronto).** 22.1.

**Fora do escopo.** camadas além de duas na primeira versão.

**Entregáveis.**
- Gerador emite a obrigação direto na linguagem do framework; um checker único; tradutores de prova fora da TCB.

**Critério de conclusão.**
- [ ] Pelo menos duas camadas (ex.: LRAT e Alethe) checadas pelo checker único.
- [ ] Redução de LOC da TCB medida e reportada.

---

### Etapa 22.3 — Aposentadoria gradual de checkers específicos · `M`

**Depende de (recebe pronto).** 22.2; 5.10.

**Fora do escopo.** —

**Entregáveis.**
- Recheck de todo o histórico do ledger com o checker único; checkers antigos revogados quando redundantes.

**Critério de conclusão.**
- [ ] Ledger histórico completo rechecado pelo checker único.

---

## Fase 23 — Produto

### Etapa 23.1 — Binário único e instalação · `M`

**Depende de (recebe pronto).** 8.9; 10.7.

**Fora do escopo.** Windows; macOS opcional.

**Entregáveis.**
- `logos` com subcomandos (`kernel`, `gateway`, `audit`, `agent`, `tui`), mantendo processos e usuários de SO separados em runtime; pacotes (deb/rpm, container, Nix).

**Critério de conclusão.**
- [ ] Instalação limpa em Fedora e Ubuntu até a demo da Etapa 8.9 em < 15 minutos seguindo o README.

---

### Etapa 23.2 — SDK de contratos e políticas · `M`

**Depende de (recebe pronto).** 8.1; 7.1.

**Fora do escopo.** marketplace de contratos.

**Entregáveis.**
- Guia e ferramentas para escrever contratos de ferramenta, vocabulários e políticas; templates para ferramentas comuns (GitHub, Postgres, S3, e-mail, calendário).

**Critério de conclusão.**
- [ ] Uma pessoa de fora integra uma ferramenta nova só com a documentação.

---

### Etapa 23.3 — Reversão de ações como recurso de operação · `M`

**Depende de (recebe pronto).** 11.7; 10.7.

**Fora do escopo.** integração com ferramentas de incidente de terceiros.

**Entregáveis.**
- `logos incident` (UI + CLI) sobre a Etapa 11.7: "o que o agente fez porque acreditou em X?", compensações guiadas.

**Critério de conclusão.**
- [ ] Exercício de incidente simulado concluído por operador que não participou do desenvolvimento.

---

### Etapa 23.4 — Documentação e release 1.0 · `M`

**Depende de (recebe pronto).** 23.1–23.3; Fase 12 — teoremas.

**Fora do escopo.** —

**Entregáveis.**
- Docs de usuário, de operador, de auditor e de pesquisador; `docs/guarantees.md` atualizado com os teoremas provados e suas hipóteses; changelog; política de versão dos formatos.

**Critério de conclusão.**
- [ ] Release 1.0 com builds reprodutíveis, hashes publicados e cabeça do ledger de demo publicada no log de transparência.

---

## Apêndice A — Matriz de rastreabilidade: princípios → etapas

| Princípio | Implementado em | Testado em | Formalizado em |
| --- | --- | --- | --- |
| P1 Agente nunca escreve obrigações/fatos/políticas | 1.5, 1.6, 5.3, 6.1, 7.2 | 5.3 | 12.2 |
| P2 Checker consome os bytes hasheados | 2.3, 4.3, 4.4, 14.1, 20.1 | 2.3, 4.3 | 12.1 |
| P3 Todo veredicto autorizador passa por checker | 5.4, 7.3, 8.3 | 5.4, 8.5 | 12.2 |
| P4 Refutação = testemunha; prova = certificado; `UNKNOWN` não autoriza | 0.6, 2.3, 5.4 | 0.6, 5.4 | 12.2 |
| P5 Nenhum efeito especulativo | 5.8, 8.2, 8.3 | 8.2, 8.5 | 12.2 |
| P6 Termo da ação vem dos bytes | 8.1, 8.2 | 8.2 | 12.2 |
| P7 Selo completo obrigatório | 15.1 | 15.1 | 12.5 |
| P8 Confiança entre agentes é premissa | 6.2, 17.2 | 17.4 | — |
| Mecanismo 1: confinamento | 5.11, 6.5 | 5.11 | 12.2 |
| Mecanismo 2: regra de citação | 1.5, 5.3 | 5.3 | 12.2 |
| Mecanismo 3: eventos assinados | 3.1, 3.2 | 3.6 | 12.1 |
| Mecanismo 4: typestate | 5.1 | 5.1 | 12.4 |
| Mecanismo 5: resposta do ledger | 10.6, 15.8 | 10.6 | 12.5 |
| Dependências do certificado | 4.7, 14.4 | 4.7 | 12.3 |
| Não-vacuidade | 5.5, 14.6 | 5.5 | 12.2 |
| Recheck / revogação de checker | 5.10, 14.5 | 5.10 | — |
| Garantia do gateway (§6) | 8.3–8.5 | 8.5 | 12.2 |
| Integridade de justificativa (§13) | 6.6, 7.1, 11.2 | 11.5 | 12.3 |

## Apêndice B — Matriz de ameaças → etapas

Numeração da tabela da §10.

| # | Ameaça | Natureza | Mitigada em | Declarada em |
| --- | --- | --- | --- | --- |
| 1 | Formalização infiel | Fundamental | 9.1, 9.3, 15.1–15.6 | 0.7, 11.10 |
| 2 | Atestador comprometido | Fundamental | 6.2, 6.4 | 0.7 |
| 3 | Política errada | Fundamental | 7.6 | 0.7 |
| 4 | Premissas inconsistentes | Engenharia | 5.5, 14.6 | — |
| 5 | Agente escreve obrigação | Engenharia | 1.5, 1.6, 5.3 | — |
| 6 | Bug em encoder/grounder | Engenharia | 4.2, 4.3, 13.3, 22.2 | — |
| 7 | Bug em checker | Engenharia | 4.4, 4.5, 5.10, 14.5, 20.2 | — |
| 8 | Chave do kernel comprometida | Engenharia | 13.1, 8.3 (vida curta) | — |
| 9 | Ledger bifurcado | Engenharia | 13.2 | 0.7 (atacante) |
| 10 | TOCTOU | Parcialmente fundamental | 8.5 | 0.7, 8.3 (`unprotected_pre`) |
| 11 | Replay/roubo de token | Engenharia | 8.3, 8.4 | — |
| 12 | Decomposição | Engenharia | 7.5, 11.6 | — |
| 13 | Injeção escolhendo entre ações permitidas | Fundamental | 6.6, 9.2, 11.1, 11.2 | 0.7, 11.10 |
| 14 | Enfraquecimento até trivialidade | Engenharia/avaliação | 10.5 (`Weakened`), 16.2 | — |
| 15 | Excesso de `UNKNOWN` | Engenharia | 8.7, 10.8 | — |
| 16 | Confiança excessiva no selo | Fundamental | 15.1, 16.3 | 0.7 |
| 17 | Certificados gigantes | Engenharia | 0.5 (`TooLarge`), 0.6 (`Unknown(Size)`), 17.3 | — |
| 18 | Check dependente de máquina | Engenharia | 0.6 (passos determinísticos) | — |

## Apêndice C — Catálogo de eventos do ledger

| Evento | Introduzido em | Campos essenciais | Quem pode causar |
| --- | --- | --- | --- |
| `Genesis` | 3.1 | versão, `h(binário do kernel)`, chave | kernel |
| `VocabPublished` / `VocabChanged` | 3.1 / 15.7 | `h(V)`, compatibilidade, admin_sig | admin |
| `Proposed` | 3.1 | proposta (texto como dado tainted), autor | agente (via kernel) |
| `ObligationCreated` | 3.1 | `h(O)`, rota, `h(CNF|smt|lean)` | kernel |
| `Verified` | 3.1 | `h(O)`, `h(cert)`, `h(checker)`, `cited`, `used`, consistência, budget | kernel |
| `Refuted` | 3.1 | `h(O)`, `h(M)` | kernel |
| `CertificateRejected` | 5.4 | `h(O)`, `h(cert)`, motivo | kernel |
| `UnknownRecorded` | 3.1 | `h(O)`, limites, máquina | kernel |
| `AssumptionDeclared` | 3.1 | fórmula, escopo, autor | agente / usuário |
| `Retracted` | 3.1 | ref, motivo, assinatura da autoridade | emissor / admin |
| `Invalidated` | 3.1 | claim, cadeia de causas | kernel |
| `Suspended` / `Rechecked` | 5.10 | claim, checker novo | kernel |
| `EpochAdvanced` | 3.4 | época, motivo | kernel / admin |
| `CheckerRegistered` / `CheckerRevoked` | 3.1 | `h(checker)`, formato, motivo | admin |
| `FactRegistered` | 6.1 | `h(fact)` | atestador |
| `IssuerRevoked` | 6.4 | KeyId, desde | admin |
| `Extracted` | 6.6 | fonte, `h(conteúdo)`, φ, extrator | ingestão |
| `TrustConfigPublished` | 6.2 | raízes, tópicos, admin_sig | admin |
| `PolicyPublished` | 7.2 | versão, `r_v`, admin_sig | admin |
| `ContractPublished` | 8.1 | contrato, admin_sig | admin |
| `TokenMinted` | 8.3 | `h(token)` | kernel |
| `Executed` / `ExecutionAborted` | 8.5 | `h(token)`, resultado / pré-condição falha | gateway |
| `Override` | 8.7 | `h(a)`, motivo, user_sig | usuário |
| `UserAuthorization` | 9.2 | `h(a)`, `h(paráfrase)`, user_sig | usuário |
| `FidelityAttested` | 9.3 / 15.2 | atestado F1–F4 | usuário / serviço |
| `Weakened` | 10.5 | `h(φ)` → `h(φ')` | agente (via kernel) |
| `KeyRotated` | 13.1 | chaves velha/nova, assinaturas | kernel / admin |
| `HeadPublished` | 13.2 | seq, `h(head)`, prova de inclusão no log | kernel |
| `Imported` | 17.3 | φ, descarregadas, pendentes | kernel de B |

## Apêndice D — Métricas que o sistema precisa emitir

| Métrica | Definição | A partir de |
| --- | --- | --- |
| Profundidade de propagação de erro | nº de nós do DAG que dependem de um nó inválido quando ele é detectado | 5.8 |
| Taxa de `UNKNOWN` | `UNKNOWN` / obrigações, por camada e por classe de ação | 5.4 |
| Taxa de overrides | overrides / ações, por classe | 8.7 |
| Latência por etapa | propose, prova, check, mint, execução (p50/p95/p99) | 5.12, 8.9 |
| Tamanho de certificado | bytes por obrigação, por camada | 4.6 |
| Taxa de consistência desconhecida | `ConsistencyUnknown` / `Verified` | 5.5 |
| Enfraquecimentos | `Weakened` por tarefa; informatividade (nº de modelos excluídos) | 10.5 |
| LOC da TCB | por propriedade; proporção verificada | 0.2 |
| ASR / utilidade | AgentDojo e suítes próprias | 11.3 |
| Tempo até premissa causadora | estudo de auditabilidade | 11.8 |
| Fidelidade calibrada | taxa de infidelidade por método, com IC | 15.6 |
| Aceitação indevida | % de conclusões válidas sobre formalizações infiéis aceitas | 16.3 |

## Apêndice E — Riscos do projeto e pontos de decisão

| Risco | Sinal de alerta | Decisão / plano B | Quando decidir |
| --- | --- | --- | --- |
| Grounding explode em políticas reais | Etapa 4.2: muitas obrigações `Unknown(Size)` | Priorizar avaliador direto (7.3) e árvores de derivação; grounding incremental/lazy | Fim da Fase 7 |
| `cake_lpr` difícil de compilar/integrar | Etapa 4.5 trava > 2 semanas | Usar o checker próprio + outro checker LRAT independente (ex.: `lrat-check` do drat-trim) como segundo checker; registrar que nenhum é verificado | Etapa 4.5 |
| Custo de autoria de políticas inviável no AgentDojo | Etapa 11.2 > 6 semanas | Reduzir às suítes com classes de ação mais claras; reportar custo como resultado | Etapa 11.2 |
| CaMeL/FIDES não reproduzíveis | Etapa 11.4 | Reimplementação documentada + comparação só com números publicados, explicitando a limitação | Etapa 11.4 |
| Taxa de `UNKNOWN` alta demais (ameaça 15) | Métrica > 20% em tarefas reais | Ampliar rota L2, melhorar o farm, override assinado; tratar como resultado | Fim da Fase 10 |
| Carcara não cobre regras necessárias | Etapa 14.3: muitas provas com regras não checadas | Isabelle como checker principal da camada 2 ou reconstrução via Vampire/TSTP (rota D) | Etapa 14.3 |
| Bit decisivo não quebra endosso por capacidade | Etapa 11.5 sem diferença | Publicar resultado negativo; deslocar ênfase do paper para trajetórias e reversão | Etapa 11.5 |
| Estudo com usuários sem efeito | Etapa 16.3 | Resultado negativo é publicável (§11); investigar formatos alternativos de selo | Etapa 16.3 |
| Escopo grande demais para uma pessoa | Caminho crítico até 11 > 12 meses | Cortar fases 19–22 do horizonte; o núcleo 0–13 é a contribuição | Revisão trimestral |

---

*Fim do roadmap. Mudanças de escopo, ordem ou decisão técnica: registrar em `docs/adr/` e atualizar a seção correspondente aqui.*
