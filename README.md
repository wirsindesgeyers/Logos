# LOGOS

Agentes mediados por prova: um sistema de efeitos com **ledger epistêmico**, em que o LLM e o solver estão fora da base de confiança.

1. A unidade de confiança é a **transição de estado**, não a resposta: efeito, claim verificado ou entrega a outro agente só ocorrem com um certificado que um kernel pequeno consegue checar.
2. A segurança é definida pela **justificativa checada de cada ação**, e não pela influência que dados não confiáveis tiveram sobre ela (*integridade de justificativa*, spec §13).
3. O agente propõe, cita e prova; nunca escreve obrigações, fatos ou políticas, e não tem credenciais nem chaves.
4. Solvers (Z3, cvc5, CaDiCaL…) são produtores não confiáveis; todo veredicto que autoriza algo passa por um checker identificado por hash e é rechecável por terceiros.
5. A ligação entre fórmulas e mundo não sai da base de confiança (L1): a arquitetura a torna explícita, assinada e visível no selo, não a elimina.

## Onde está o quê

| | |
| --- | --- |
| [`ROADMAP.md`](ROADMAP.md) | Fases e etapas, cada uma com critério de conclusão testável |
| [`docs/spec/`](docs/spec/LOGOS-proof-carrying-agents.md) | Especificação conceitual (citada como `§N` no roadmap) |
| [`docs/plans/`](docs/plans/) | Planos de fase e de etapa |
| [`docs/adr/`](docs/adr/) | Decisões de arquitetura |
| `crates/` | Rust: TCB (`⚠`) e componentes de apoio |
| `py/` | Python: agente, MCP, prover farm e avaliação (tudo não confiável) |
| `ci/` | Verificações de CI reproduzíveis localmente |
| `fuzz/` | Alvos de `cargo fuzz` |

## Desenvolvimento

```sh
cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
(cd py && uv sync && uv run ruff check . && uv run mypy && uv run pytest)
ci/check-tcb-lints.sh && ci/check-audit-independence.sh
ci/selftest.sh   # prova que a CI bloqueia unsafe, unwrap e dependências fora da allowlist
```

Regras que valem para todo crate de TCB estão na §4 do roadmap. Licença: Apache-2.0 ou MIT, à sua escolha.
