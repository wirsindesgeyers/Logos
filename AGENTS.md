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
3. Rodar a **verificação completa** (§4) e corrigir tudo.
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

## 4. Verificação completa (rodar antes de todo commit)

```sh
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
ci/check-tcb-lints.sh && ci/check-audit-independence.sh
tcb/count.sh check                 # ou: cargo xtask tcb check
cargo deny check && cargo audit
( cd py && uv run ruff check . ../tcb ../tools && uv run ruff format --check . ../tcb ../tools \
    && uv run mypy && uv run mypy --strict ../tcb/tcb.py ../tcb/tests ../tools/gen_unicode_assigned.py \
    && uv run python ../tools/gen_unicode_assigned.py --check && uv run pytest -q && uv run pytest -q ../tcb/tests )
ci/selftest.sh                     # a CI bloqueia o que deveria, pelo motivo certo (~1 min, usa rede)
```

- **Não coloque `| tail`/`| head` numa cadeia `&&` de verificação**: o pipe esconde o código de
  saída do comando que falhou (isso já deixou passar um clippy quebrado). Use `set -o pipefail`
  ou rode sem pipe e olhe o `exit=`.
- Fuzz (alvos em `fuzz/fuzz_targets/`): `cd fuzz && RUSTC_BOOTSTRAP=1 cargo +stable fuzz run <alvo> -- -max_total_time=150`.
  A meta de ≥ 1 h por alvo roda no job noturno da CI (`.github/workflows/fuzz.yml`).
- Medição da TCB: `cargo xtask tcb report` (LOC próprio + dependências transitivas por propriedade).

## 5. Git

- Mensagens em português, no formato `Etapa X.Y: resumo` (ou `Corrige ...`, `Docs: ...`), corpo
  explicando o porquê. Um commit coerente por etapa/correção.
- **Não adicione `Co-Authored-By` nem qualquer atribuição ao Claude/IA** em commits ou PRs
  (decisão do mantenedor). Vale para qualquer agente.
- Só faça `git push` quando o mantenedor pedir (ou já tiver pedido nesta sessão). Nunca use
  `--force` sem pedido explícito. A branch é `main`; o remoto é `origin` (GitHub).
- Commit só depois da verificação completa passar. Se um commit local ainda não foi enviado e
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

- `rust-toolchain.toml` fixa `stable` (1.99); o `nightly` padrão da máquina (1.98) é **mais velho**
  que o `rust-version` do projeto. Para fuzz local: `RUSTC_BOOTSTRAP=1 cargo +stable fuzz ...`.
- `cp` está com alias interativo (`cp -i`) no shell do mantenedor: use `command cp -f` em scripts.
- `pkill -f <padrão>` e `pgrep -f` casam com a *própria* linha de comando do shell que os chama;
  use PIDs ou nomes exatos de processo.
- `gen` é palavra reservada na edição 2024 do Rust.
- O `unicodedata` do Python **precisa ser 16.0.0** (Python 3.14, via `py/.python-version`); o do 3.12
  é 15.0 e o `unicode-normalization` 0.1.25 é Unicode 17 (por isso está fixado em `=0.1.24`).
- `gh` (GitHub CLI) não está instalado: não dá para consultar a CI daqui; peça ao mantenedor.
- Ferramentas instaladas: `cargo-deny`, `cargo-audit`, `cargo-fuzz`, `tokei`, `uv`.
