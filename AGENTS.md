# AGENTS.md — instruções para agentes de IA (e para quem trabalha com eles)

LOGOS é um sistema de **agentes mediados por prova**: o LLM e os solvers ficam fora da base de
confiança (TCB); só um kernel pequeno, em Rust, checa certificados e emite claims/tokens. Tese em
5 linhas no [`README.md`](README.md); especificação conceitual em
[`docs/spec/`](docs/spec/LOGOS-proof-carrying-agents.md) (citada como `§N`; `§N.k` = k-ésimo item
numerado da seção N); plano completo em [`ROADMAP.md`](ROADMAP.md).

## 1. Antes de qualquer coisa

1. **Leia [`STATUS.md`](STATUS.md)**: onde o projeto parou, o que está verificado, o que está
   pendente e qual é o próximo passo. Ele é a fonte de verdade sobre o *estado*; o roadmap é a
   fonte de verdade sobre o *escopo*.
2. Leia a etapa correspondente no `ROADMAP.md`: `Depende de`, `Fora do escopo`, `Entregáveis` e
   `Critério de conclusão`. **O que está em "Fora do escopo" não entra**, nem "já que estou aqui".
3. Se algo de que a etapa depende não existe no repositório, **pare e registre como bloqueio** em
   `STATUS.md` em vez de implementar.

## 2. Fluxo de uma etapa

1. Plano curto da etapa (formato do prompt em `ROADMAP.md` §0) em `docs/plans/etapa-X.Y.md` quando a
   etapa for `M` ou maior, ou tiver decisões em aberto. Para etapas `P` simples, vale um resumo
   no próprio `STATUS.md`.
2. Implementar com testes. **Critério de conclusão é contrato**: cada item vira um teste, script
   ou artefato reproduzível. Só marque `- [x]` no `ROADMAP.md` o que foi *demonstrado*
   (localmente basta se o item não depende da CI; se depende, marque só depois de ver a CI).
3. Rodar `ci/verify.sh --tier fast` (ou `--tier affected`) a cada commit e corrigir tudo. Etapas
   independentes podem andar em `git worktree` paralelos; a verificação **completa** roda na ponta
   da pilha antes de qualquer push e antes de marcar critérios (ADR 0051, ADR 0057).
4. Atualizar `STATUS.md` (§6) **no mesmo commit** do trabalho.
5. Commitar (§5).

## 3. Regras duras

- **TCB (`⚠ TCB`)**, crates listados em `tcb/manifest.toml` com `strict = true`:
  `#![forbid(unsafe_code)]`, sem `unwrap()`/`expect()`/`panic!` fora de testes, dependências novas
  só com ADR + entrada em `deny.toml` (`[bans].allow`), fuzz target para todo parser/decoder novo,
  arquivo novo listado em `tcb/manifest.toml`.
- **Formato serializado mudou?** Vetores em `spec/vectors/`, bump de versão, `spec/*.md` atualizado.
- **Decisão técnica nova ou divergência da spec/roadmap?** ADR em `docs/adr/` (numeração
  sequencial). Mudança de escopo ou ordem: atualizar o `ROADMAP.md` e registrar em ADR.
- **Nada de silêncio em erro na TCB**: um objeto que não tem codificação/resultado válido devolve
  erro; nunca um valor "parecido" (`null`, truncado, padrão). `Unknown` nunca vira
  `Verified`/`Refuted`; `Rejected` nunca vira `Refuted`.
- **Fidelidade ao relatório**: se um teste falhou, foi pulado ou não rodou, diga isso. Não marque
  como feito o que só passou "na minha máquina" se o critério exige CI.
- **Testes que não detectam nada não valem**: para código de TCB, prove que o teste falha quando o
  código está errado (mutação manual de alguns bugs, ou o teste diferencial quebrando).
- `logos-audit` **não** depende de `logos-kernel` nem de `logos-ledger` (`ci/check-audit-independence.sh`).
- Python em `py/` é **não confiável** por construção: nunca guarda chaves nem credenciais.

## 4. Verificação (níveis, ADR 0057)

```sh
ci/verify.sh --tier fast      # a cada commit (minutos): build do cake_lpr, fmt, clippy -D warnings,
                              # cargo test --workspace com LOGOS_TIER=fast (escala reduzida, mesma lógica),
                              # lints/manifesto/independência da TCB, ruff/mypy, vetores --check,
                              # pytest -m "not heavy"; sem smokes de integração, sem rede
ci/verify.sh --tier affected  # fast + smokes/ci/check-*.sh das áreas tocadas desde --base (origin/main),
                              # escolhidos por tools/verify_plan.py (caminho desconhecido => tudo)
ci/verify.sh                  # completo (padrão, ~1 h): tudo em escala de critério — corpus LRAT SAT-2002 ×
                              # cake_lpr, selo 50k/vocabulário, diferenciais canon 100k/modelo 10k, Z3 falso 10k,
                              # farm ×500, MCP, caderno, crash do ledger (SIGKILL, 10k appends), índices
                              # (rebuild 100k, p99 com 1M), todos os smokes — + cargo deny/audit.
                              # Exigido antes de push e para marcar critérios no ROADMAP.
ci/verify.sh --selftest       # completo + autoteste: a CI bloqueia o que deveria, pelo motivo certo (~2 min)
```

Cada etapa grava a duração em `target/verify-times.tsv` e imprime `STEP_OK`; falha imprime
`VERIFY_FAILED tier=… step=…`. Teste de escala novo: leia `LOGOS_TIER` (Rust) ou use
`tests/tier.py::scaled(full, fast)` (Python); se a asserção depende do tamanho exato, marque
`@pytest.mark.heavy`. Sem `LOGOS_TIER` vale a escala completa.

O script termina com `VERIFY_OK tier=<nível>` e usa `set -euo pipefail`. Rode-o e confira o código de saída.
Se uma etapa de `ci/verify.sh` mudar, mude o script (e este trecho), não só o seu atalho local.

- **Não coloque `| tail`/`| head` numa cadeia `&&` de verificação**: o pipe esconde o código de
  saída do comando que falhou (isso já deixou passar um clippy quebrado). Use `set -o pipefail`
  ou rode sem pipe e olhe o `exit=`.
- Fuzz (alvos em `fuzz/fuzz_targets/`): `ci/fuzz.sh SEGUNDOS [alvo...]` — **o mesmo script da CI**
  (`.github/workflows/fuzz.yml`). Ele fixa `RUSTUP_TOOLCHAIN` (padrão `nightly`; localmente use um
  nightly datado: `FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 canon_decode`). **Não use
  `RUSTC_BOOTSTRAP=1`**: ele faz o fuzz rodar na stable e esconde exatamente o erro que a CI teria
  (o `rust-toolchain.toml` da raiz fixa `stable` e vale dentro de `fuzz/`). A meta de ≥ 1 h por alvo
  roda na matriz noturna (um alvo por job, descoberta de `fuzz/Cargo.toml`);
  `kernel_message` (5.2) exige ≥4h e usa14400s/timeout330min no noturno.
  O script divide o tempo em lotes de `FUZZ_BATCH_SECONDS` (padrão1800s) com
  processo novo por lote, corpus cumulativo e teto de RSS armado em cada lote:
  o runtime ASAN acumula RSS por execução mesmo em alvo trivial, então uma
  execução única longa não garante o teto padrão de2048MiB (5.2; ADR0036).
- Cobertura de ramos da 1.3: `ci/check-sort-coverage.sh` — runner separado (também
  no job `sort-coverage`), exige 100% dos ramos LLVM de `src/sort_check.rs`. Instale
  `cargo install cargo-llvm-cov --version 0.9.1 --locked` e
  `rustup component add llvm-tools-preview --toolchain nightly-2026-09-28`.
  `COVERAGE_TOOLCHAIN` permite selecionar outra nightly; o padrão datado mantém
  a medição reproduzível. Relatório: `target/sort-check-coverage.json`; sem
  `RUSTC_BOOTSTRAP`. Cobertura instrumentada não é prova formal.
- Medição da TCB: `cargo xtask tcb report` (LOC próprio + dependências transitivas por propriedade).

## 5. Git

- Mensagens em português, no formato `Etapa X.Y: resumo` (ou `Corrige ...`, `Docs: ...`), corpo
  explicando o porquê. Um commit coerente por etapa/correção.
- **Não adicione `Co-Authored-By` nem qualquer atribuição ao Claude/IA** em commits ou PRs
  (decisão do mantenedor). Vale para qualquer agente.
- Só faça `git push` quando o mantenedor pedir (ou já tiver pedido nesta sessão). Nunca use
  `--force` sem pedido explícito. A branch é `main`; o remoto é `origin` (GitHub).
- Commit local exige `ci/verify.sh --tier fast` (ou `affected`) passando, mais mutações quando a
  etapa toca TCB (ADR 0057, revisa o 0051); **push** exige a verificação
  completa passando na ponta da pilha (no máximo três commits de etapa sem ela), com o recibo
  listando os commits cobertos. Critérios que dependem dela só são marcados depois.
  Se um commit local ainda não foi enviado e
  precisa de correção, `--amend` é aceitável; depois de enviado, faça um commit novo.

## 6. Como manter o `STATUS.md`

Ao fim de cada sessão de trabalho (e sempre que uma etapa mudar de estado):

1. Atualize a **tabela de etapas** (⏳ não iniciada, 🚧 em andamento, ✅ concluída, ⚠ concluída com
   pendência de verificação remota) e os itens "Pendências de verificação".
2. Reescreva **"Próximo passo"** para que o mantenedor possa simplesmente pedir: *"implemente o
   próximo passo do STATUS.md"*. Deve dizer qual etapa, o que já está pronto de que ela depende,
   decisões em aberto e riscos conhecidos.
3. Acrescente uma linha ao **Diário** (apenas acrescente; nunca reescreva o passado).
4. Registre **decisões e descobertas** que não estão no código (ex.: "a ferramenta X não está
   instalada") em "Armadilhas conhecidas" abaixo ou no STATUS.

## 7. Estrutura

```text
crates/     Rust: TCB (⚠, ver tcb/manifest.toml) e apoio (alethe, attest, audit, cli, tui, xtask em xtask/)
py/         Python 3.14: logos_client (canon.py = referência), provers, mcp, agent, eval; tudo não confiável
spec/       especificações normativas (encoding.md, ...) e vetores de teste (spec/vectors/)
docs/       spec conceitual (docs/spec/), ADRs (docs/adr/), planos (docs/plans/), ameaças (0.7, a criar)
tcb/        manifest.toml + tcb.py (check/report/diff) + history/
ci/         verificações reproduzíveis localmente (selftest.sh, check-*.sh)
tools/      geradores (gen_unicode_assigned.py)
fuzz/       alvos de cargo fuzz (workspace próprio)
.github/    CI (ci.yml) e fuzz noturno (fuzz.yml)
```

## 8. Armadilhas conhecidas do ambiente

- `rust-toolchain.toml` fixa `stable` (1.99) e vale também dentro de `fuzz/`; o `nightly` padrão da
  máquina (1.98) é **mais velho** que o `rust-version` do projeto. Para fuzz local instale um nightly
  datado (`rustup toolchain install nightly-2026-09-28 --profile minimal`) e use `ci/fuzz.sh`.
- Constantes de `O_NOFOLLOW`/`O_NONBLOCK` em `logos-crypto` são fixas por plataforma (sem `libc`); o
  valor do aarch64 ainda precisa ser confirmado no runner aarch64 (Etapa 0.6).
- O Python precisa ser **3.14** (`requires-python = ">=3.14,<3.15"`).
- `cp` está com alias interativo (`cp -i`) no shell do mantenedor: use `command cp -f` em scripts.
- `pkill -f <padrão>` e `pgrep -f` casam com a *própria* linha de comando do shell que os chama;
  use PIDs ou nomes exatos de processo.
- `gen` é palavra reservada na edição 2024 do Rust.
- O `unicodedata` do Python **precisa ser 16.0.0** (Python 3.14); o do 3.12
  é 15.0 e o `unicode-normalization` 0.1.25 é Unicode 17 (por isso está fixado em `=0.1.24`).
- `gh` (GitHub CLI) não está instalado: não dá para consultar a CI daqui; peça ao mantenedor.
- Ferramentas instaladas: `cargo-deny`, `cargo-audit`, `cargo-fuzz`, `tokei`, `uv`.

- cake_lpr4.5 exige Linux, `/usr/bin/bwrap` e `/usr/bin/prlimit`, com namespaces
  de usuário/rede habilitados. Ausência/falha não tem fallback sem isolamento.
  Build offline: `python3 tools/build_cake_lpr.py`; rebuild do assembly/FFI
  fixados: `python3 tools/build_cake_lpr.py --rebuild` (Zig0.15.1 por hash).
- Runners do GitHub (Ubuntu 24.04) mantêm o AppArmor restringindo user namespaces
  (`kernel.apparmor_restrict_unprivileged_userns=1`) e não vão desativar por padrão
  (actions/runner-images#10015). Os jobs `rust` e `python` do `ci.yml` rodam
  `ci/enable-userns.sh` antes dos testes; sem isso o sandbox bwrap do checker não inicia.
- `pytest ../tcb/tests` exige `tokei` no PATH (medição real de LOC via `tcb.build_report`);
  o job `python` do `ci.yml` instala com `taiki-e/install-action`, como o `tcb-report`.
- Produtor SAT4.6: `python3 tools/build_cadical.py` compila a fonte fixada
  offline com g++/GNU make; cache por hash e log em target/cadical-build.log.
  `ci/check-sat.sh` prepara o encoder de apoio e executa o corpus real.
