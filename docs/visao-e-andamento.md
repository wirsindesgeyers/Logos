# LOGOS — visão, ideais e andamento

> Documento de contexto conceitual para leitura por LLM. Estado em **2026-10-21**.
> Fontes de verdade: **estado** em [`STATUS.md`](../STATUS.md); **escopo e ordem** em
> [`ROADMAP.md`](../ROADMAP.md); **conceito** em
> [`docs/spec/LOGOS-proof-carrying-agents.md`](spec/LOGOS-proof-carrying-agents.md).
> Este texto resume os três. Em caso de conflito, prevalecem eles, e este arquivo deve ser atualizado.

## 1. O que é o LOGOS

LOGOS é um **sistema de efeitos mediado por prova**: um agente de IA só consegue alterar estado
comprometido (registrar um claim como verificado, executar uma ação no mundo, entregar um claim a
outro agente) apresentando um **certificado** que um **kernel pequeno** consegue checar.

O LLM e os solvers (Z3, CaDiCaL, checkers LRAT) são **produtores não confiáveis**: podem errar ou
ser manipulados sem que a garantia caia, porque quem decide é o kernel, não eles.

A unidade de confiança é a **transição de estado**, não a resposta do modelo.

A forma da arquitetura vem de três ideias antigas:
- **LCF** (Milner): só o kernel constrói objetos do tipo "claim verificado" e "autorização".
- **Proof-carrying code** (Necula e Lee): quem produz não é confiável; quem consome checa a prova.
- **Monitor de referência** (Anderson): mediação completa, à prova de adulteração, pequeno o bastante
  para ser verificado. O agente **não tem credenciais**; a garantia não depende de pedir "por favor,
  se comporte".

Tese em uma frase: se um efeito ocorreu ou um claim foi marcado verificado, existe no ledger uma
cadeia de eventos assinados pelo kernel ligando isso a uma obrigação gerada deterministicamente, a um
certificado aceito por um checker identificado por hash, e a premissas com origem registrada.

## 2. Ideais e princípios (o que não se negocia)

1. **O agente propõe, cita e prova; nunca escreve obrigações, fatos ou políticas.** Se escrevesse a
   obrigação, poderia submetê-la como `⊤`. Se escrevesse os fatos, a prova seria sobre ficção.
2. **TCB pequena e explícita.** Crates listados em `tcb/manifest.toml`, com `#![forbid(unsafe_code)]`,
   sem `unwrap`/`expect`/`panic!` fora de testes, dependências novas só com ADR. Tudo o mais (Python,
   MCP, farm de provadores, cliente) é **não confiável por construção**.
3. **Nada de silêncio em erro na TCB.** Objeto inválido devolve erro, nunca um valor "parecido".
   `Unknown` nunca vira `Verified`/`Refuted`; `Rejected` nunca vira `Refuted`.
4. **Teste que não detecta nada não vale.** Código de TCB precisa de mutações com controles positivos
   que mostrem que o teste falha quando o código está errado.
5. **Reprodutibilidade.** Vetores independentes (gerados com ferramentas de fora, como `hashlib`),
   hashes canônicos, `ci/verify.sh` que roda tudo localmente.
6. **Honestidade sobre limites.** Ver seção 6. O projeto declara o que a prova *não* garante.
7. **Não marcar como feito o que não foi demonstrado.** Critério de conclusão é contrato; "passou na
   minha máquina" não basta quando o critério exige CI.

## 3. Arquitetura em uma página

```text
 texto natural ──► AGENTE (LLM, fora da TCB) ──► proposta: fórmula AIR + citações
                                                        │
                       vocabulário versionado + fatos   ▼
                      (atestados, eventos do ledger)  GERADOR DE OBRIGAÇÕES (TCB, só saída)
                                                        │  O = Γ ⊢ φ, com h(O)
                              produtores não confiáveis ▼
                       Z3 / finder / CaDiCaL / LRAT ──► certificado
                                                        │
                                  CHECKER identificado por hash (TCB)
                                                        ▼
                       KERNEL (TCB): máquina de estados + ledger com hash chain assinada
                                                        ▼
                  claims Verified / Refuted / Witness · (futuro) tokens de ação e gateway
```

Componentes principais:
- **AIR** (`FOL-ms-eq/v1`): sintaxe canônica, De Bruijn, sort-checker. Texto não confiável é
  reparseado.
- **Vocabulário** versionado com grounding; a partir da 9.1a, templates `render_pt` explícitos (ADR 0046).
- **Modelos finitos** (`model/v1`) e avaliador `M ⊨ φ`; checker de refutação e de consistência.
- **Grounding e CNF** (`ground/v1`, Tseitin canônico) e **checker LRAT próprio**, com o `cake_lpr`
  verificado como segundo checker, isolado em sandbox (bwrap).
- **Ledger**: eventos assinados em SQLite (WAL/FULL), hash chain, épocas, CAS antes do append,
  índices derivados (sem autoridade própria).
- **Auditor independente** (`logos-audit`): verifica o ledger ou um export sem depender do kernel.
- **Kernel**: typestate de claims, protocolo por socket Unix autenticado por UID, PROPOSE, CERTIFY,
  REFUTE, WITNESS.
- **Harness fora da TCB**: cliente async do protocolo, farm isolada de provadores com cache que
  sempre rechecka, servidor MCP de pesquisa (stdio).

## 4. Onde o projeto está

### 4.1 Concluído localmente (demonstrado na máquina do mantenedor, sem CI remota)

- **Fase 0 — Fundação:** workspace, manifesto e medição da TCB, `logos-canon`, `logos-crypto`, CAS,
  `logos-core`. Documentos de ameaças e escopo (0.7).
- **Fase 1 — AIR e obrigações (1.1–1.6):** sintaxe, vocabulário com grounding, sort-check com 100% de
  cobertura de ramos, parser de superfície, gerador de obrigações, formato da proposta.
- **Fase 2 — Modelos (2.1–2.6):** estruturas finitas, avaliador, checker de refutação, produtor Z3
  (200/200 contramodelos aceitos; Z3 falso não é aceito indevidamente), finder exaustivo
  independente de Z3, explicação de contramodelos em português (20/20 aceitos pelo checker).
- **Fase 3 — Ledger (3.1–3.5):** catálogo de 13 tipos de evento, appender SQLite durável (10k eventos
  com 24 SIGKILLs, sem perda de append reconhecido), índices com p99 < 1 ms em 1M eventos, épocas,
  CAS integrado sem bypass.
- **Fase 4 — Grounding e SAT (4.1–4.7):** fragmento finito, grounding, CNF/Tseitin, checker LRAT
  próprio (200/200 provas e 600/600 corrupções concordam com o `cake_lpr`), produtor CaDiCaL (300/300),
  dependências extraídas do certificado.
- **Fase 5 — Kernel (5.1 concluída; 5.2–5.5 implementadas localmente):** typestate, transporte e
  staging reais, PROPOSE com snapshot autenticado, CERTIFY/REFUTE com registry de checkers, WITNESS e
  não-vacuidade (Γ inconsistente prova `False` mas não autoriza ação).
- **Fase 10 — Harness (10.1–10.2, locais):** cliente async do protocolo, sete mensagens v1 contra o
  executável real; farm isolada com 500 obrigações distintas, todas rechecadas no kernel.
- **Trilha 5R (5R.1, local):** servidor MCP de pesquisa com seis ferramentas stdio; dez tarefas
  (5 provas, 5 refutações) auditadas com Rust e `cake_lpr`.
- **9.1a — templates versionados no vocabulário (ADR 0046, local):** `vocabulary/v2` com
  `render_pt` explícito por símbolo, v1 preservado, fuzz de 1 h, mutações e full verify/selftest
  com `VERIFY_OK`. É só o pré-requisito: o renderizador/parser da 9.1 ainda não existe.

### 4.2 Em andamento

- Nada em andamento no momento desta revisão; o próximo passo é a 9.1 (seção 4.4).
- **Proposto, não iniciado:** trilha de interface e workspaces (Apêndice G, seção 5.1).

### 4.3 Pendências transversais

- **CI remota nunca observada** para várias etapas (`gh` não está instalado nesta máquina). Nada
  marcado como "CI" está demonstrado remotamente.
- **ARM/aarch64:** flags de `logos-crypto` e `O_NOFOLLOW` ainda precisam de confirmação.
- **Fuzz noturno:** a meta é ≥ 1 h por alvo na CI; até agora rodou localmente.
- **Rebuild do `cake_lpr` em segundo host** (critério da 4.5).
- **Fuzz e verificação de uso** de algumas etapas 5.x ainda pendentes em CI.

### 4.4 Próximo passo

1. **9.1 — renderizador/parser PT na TCB de fidelidade:** `render_pt` e `parse_pt` com round-trip
   `parse_pt(render_pt(φ)) == φ` em 100k fórmulas (profundidade 6), gramática da língua controlada em
   `spec/paraphrase-pt.md`, fuzz do `parse_pt` (≥ 1 h), e **revisão humana de 50 paráfrases de
   políticas reais**. Essa revisão é feita por uma pessoa; script não substitui.
2. Depois: **5R.2** (formalizador com confirmação), que depende de 9.1 completa.

## 5. Futuro (o que ainda não existe)

O roadmap tem 23 fases numeradas, mais a trilha 5R e a trilha de memória. Resumo dos títulos:

| Fase | Tema |
| --- | --- |
| 6 | Atestadores e fatos (ligação entre símbolos e mundo, com assinatura) |
| 7 | Políticas (regras formais escritas por humanos) |
| 8 | Ações: contratos, tokens e gateway (efeito só com token do kernel) |
| 9 | Paráfrase determinística e autorização do usuário (9.1a em curso; 9.2 passkey/WebAuthn; 9.3 fidelidade) |
| 10 | Harness do agente (LLM, MCP, prover farm) — parte já feita |
| 11 | Integridade de justificativa — **primeiro paper** |
| 12 | Modelo formal mecanizado |
| 13 | Hardening, chaves e transparência |
| 14 | Camada 2: primeira ordem via Alethe |
| 15 | Selo de cinco campos e fidelidade |
| 16 | Regime epistêmico: benchmark e estudo com usuários |
| 17 | Portable Verified Claims (multi-agente) |
| 18 | Debate checkável — **segundo paper** |
| 19 | Argumentação derrotável (ASPIC+) |
| 20 | Camada 3: Lean |
| 21 | Camada 4: HOL, modal e deôntica |
| 22 | Framework lógico único |
| 23 | Produto |
| 5R | Trilha científica: agente de pesquisa com cartões, caderno de hipóteses e piloto (ADR 0037) |
| MEM | Memória persistente, baseada no código do uniichat (ADR 0039, Apêndice F do roadmap) |
| WS | Interface web, projetos, artefatos, visualizações e documentos (Apêndice G, proposta) |

### 5.1 O produto que se quer (trilha WS, proposta)

Um LOGOS só: **um chat universal e um harness**, com verificadores nativos em qualquer conversa,
sessões separadas e reabríveis, TUI/CLI e uma interface web local opcional, **projetos opcionais**
persistentes, artefatos com proveniência (hipóteses, evidências, referências a claims, datasets,
gráficos, relatórios), visualizações interativas, exportação LaTeX/PDF e um **workspace científico**
que organiza painéis sem trocar de agente. Não há "LOGOS Reason" e "LOGOS Research" separados: a
trilha 5R é o perfil de pesquisa do mesmo agente.

Regras que não mudam: a interface só mostra e pede; status vem do kernel; salvar não torna nada
verdadeiro; a assinatura acontece na interface do lado do kernel (9.2), nunca na web; a memória é a
da trilha MEM, sem segunda memória. Tudo fica fora da TCB. Projetos dependem de MEM.0/MEM.1, e o
contexto compartilhado automático, de MEM.5/MEM.6, que vêm depois da Fase 8/9.2. Especificação em
[`docs/spec/LOGOS-workspace-and-web-ui.md`](spec/LOGOS-workspace-and-web-ui.md); ADRs 0047–0049
propostos aguardam decisão do mantenedor (WS.0).

Ordem e dependências: cada etapa lista "Depende de" e "Fora do escopo" no `ROADMAP.md`. O que está
fora do escopo de uma etapa **não entra nela**.

## 6. Limites e riscos (o que o projeto não promete)

- **L1 — a prova não liga símbolos ao mundo.** Certifica `Γ ⊢ φ`, não que `φ` signifique o que a frase
  queria dizer nem que os fatos em `Γ` sejam verdadeiros. A confiança migra para quem formaliza e
  quem atesta; o projeto torna essa migração explícita, assinada e visível no selo.
- **L2 — para políticas pequenas e decidíveis, prova é redundante.** Um gateway que decide RBAC ou
  Cedar sozinho não precisa de prova. Prova só se paga quando checar é muito mais barato que decidir.
- **L3 — a mediação controla efeitos e ledger, não a cognição.** O LLM pode usar texto não verificado
  na prosa; o que a arquitetura garante é que nada não verificado vira efeito ou claim verificado.
- **Risco de fidelidade:** a paráfrase da 9.1 precisa ser lida por humanos; sem isso, o selo mente.
- **Tamanho da TCB:** medição de 21/10 com ~15 mil LOC próprios e ~747 mil LOC de dependências Rust
  (medição exata em `STATUS.md` e `cargo xtask tcb report`). Crescimento é monitorado; a TCB ainda
  não foi reduzida a propósito.
- **Dois regimes:** operacional (mundo fechado, políticas formais, forte) e epistêmico (mundo aberto,
  linguagem natural, garantia condicional à fidelidade). A mesma máquina serve aos dois sem que o
  selo de um empreste credibilidade ao outro.

## 7. Como usar este documento com um LLM

- Para **estado atual** ("o que está pronto?", "o que falta?"): use `STATUS.md`; este documento
  pode estar um pouco atrás.
- Para **escopo e critérios** de uma etapa: use `ROADMAP.md`.
- Para **o porquê** das decisões e a tese: use a spec em `docs/spec/` e os ADRs em `docs/adr/`.
- Nunca diga que algo está "verificado em CI" se o `STATUS.md` marcar "CI pendente".
- Este arquivo deve ser atualizado ao fechar cada etapa (regra de `AGENTS.md` §6).
