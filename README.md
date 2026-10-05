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
| [`spec/air.md`](spec/air.md) | Sintaxe FOL-ms-eq/v1: formato canônico, De Bruijn e limites estruturais |
| [`spec/vocabulary.md`](spec/vocabulary.md) | Vocabulário versionado, grounding, dependências acíclicas e compatibilidade |
| [`spec/sort-check.md`](spec/sort-check.md) | Tipagem, escopo De Bruijn, precedência e caminhos de erros |
| [`spec/air-surface.md`](spec/air-surface.md) | Texto AIR não confiável: gramática, documentos, nomes e debug reversível |
| [`spec/obligation.md`](spec/obligation.md) | Geração TCB, citações de eventos, snapshot e obligation/v1 só de saída |
| [`spec/model.md`](spec/model.md) | model/v1: estruturas finitas, funções totais, enumeração e limites |
| [`spec/evaluation.md`](spec/evaluation.md) | M ⊨ φ contextual, expansão Defined, erros e passos determinísticos |
| `crates/` | Rust: TCB (`⚠`) e componentes de apoio |
| `py/` | Python: agente, MCP, prover farm e avaliação (tudo não confiável) |
| `ci/` | Verificações de CI reproduzíveis localmente |
| `tcb/` | Manifesto da TCB (`manifest.toml`) e ferramenta de contagem (`cargo xtask tcb`) |
| `fuzz/` | Alvos de `cargo fuzz` |

## Desenvolvimento

```sh
cargo build --workspace && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace
cargo run -p logos-air --example air_smoke   # fixture/hash independentes, limites e erros
cargo run -p logos-air --example vocabulary_smoke   # grounding, extensão, codec e ciclos
cargo run -p logos-air --example sort_check_smoke   # fórmula fechada e erros com path exato
cargo run -p logos-air-text --example air_text_smoke   # texto → AIR, bytes e reparse
cargo run -p logos-obligation --example obligation_smoke   # Γ ⊢ φ, citações, rejeições e h(O)
cargo run -p logos-protocol --example protocol_smoke   # JSON estrito → candidato → O; sem autoridade
cargo run -p logos-model --example model_smoke   # modelo canônico, totalidade/domínio e Unknown(Size)
cargo run -p logos-model --example eval_smoke   # true/false/Defined/Steps/entradas inválidas
ci/check-sort-coverage.sh   # 100% ramos LLVM; cargo-llvm-cov 0.9.1 + nightly/llvm-tools
(cd py && uv sync && uv run ruff check . && uv run mypy && uv run pytest)
(cd py && uv run pytest -q tests/test_air_differential.py)   # 500 fórmulas com AST esperada
(cd py && uv run pytest -q -s tests/test_model_differential.py)   # 10k pares (M, φ) × Z3
ci/check-tcb-lints.sh && ci/check-audit-independence.sh
cargo xtask tcb check && cargo xtask tcb report   # manifesto e contagem da TCB (tcb/)
ci/selftest.sh   # bloqueios reais: unsafe, unwrap, lints comentados e dependências fora da allowlist
```

Regras que valem para todo crate de TCB estão na §4 do roadmap. Licença: Apache-2.0 ou MIT, à sua escolha.

O guard de lints lê atributos incondicionais do preâmbulo do crate; comentários,
literais, atributos de módulo e `cfg_attr` não substituem `forbid`/`deny`.
Rustc e Clippy continuam responsáveis pela validade e pelo enforcement.
A contagem segue a closure Cargo de cada propriedade, incluindo dependências
workspace fora das raízes declaradas, sem duplicar raízes já contadas como próprias.
Uso runtime/build é calculado por propriedade; a tabela consolidada marca runtime
se alguma propriedade o exigir. Snapshots JSON de commits estão em `tcb/history/`.

## Sintaxe disponível

`logos-air` oferece `Sort`, `Symbol`, `Term` e `Formula`: árvores prefixas planas,
com variáveis De Bruijn e limites configuráveis. `Formula` tem identidade por
hash no domínio reservado e todo envelope de fórmula inclui `FOL-ms-eq/v1`.
Exemplo público: `crates/logos-air/examples/air_smoke.rs`; contrato em
[`spec/air.md`](spec/air.md), decisão em
[`ADR 0007`](docs/adr/0007-sintaxe-air.md).

Etapa 1.2 acrescenta `Vocabulary`, `SortDecl`, `SymbolDecl` e grounding
`Defined`/`Anchored`/`Glossed`. `hash(&v)` identifica versão, declarações e glosas;
`extend` revalida patches e marca mudanças incompatíveis. Corpos definidores
não incluem a própria cabeça; dependências também são extraídas dos termos.
Contrato em [`spec/vocabulary.md`](spec/vocabulary.md), decisão em
[`ADR 0009`](docs/adr/0009-vocabulario-grounding.md).

Etapa 1.3 oferece `check(&vocabulary, &formula) -> Result<(), SortError>`:
aridade, sorts, kind/resultado, igualdade e variáveis ligadas. Percurso iterativo,
sem clonar ASTs; erros apontam record de fórmula e posição/nó do termo embutido.
Contrato em [`spec/sort-check.md`](spec/sort-check.md), decisão em
[`ADR 0010`](docs/adr/0010-sort-checker.md).

Etapa 1.4 acrescenta `logos-air-text` e `logos_client.air`, **fora da TCB**:
parsers independentes de fórmulas/documentos e pretty-printers reversíveis.
Nomes de binders viram De Bruijn; formas explícitas preservam assinaturas,
índices livres e sintaxe mal tipada. Parsing não invoca sort-check nem resolve ledger.
Grounding, enum/fonte, versão e citações são preservados; texto/claims não têm
novo envelope binário ou autoridade. Contrato em [`spec/air-surface.md`](spec/air-surface.md),
decisão em [`ADR 0011`](docs/adr/0011-sintaxe-de-superficie.md).

```python
from logos_client import air, canon

document = air.parse_document(
    "vocab v1 { sort User pred Admin(User) } "
    "claim C7: forall u:User. Admin(u) cites [F3]"
)
formula = document.claims[0].formula
print(canon.encode(formula).hex())  # air/v1, não hash do texto
print(air.print_document(document))  # metadados explícitos; texto de debug
```

Etapa 1.5 oferece `logos-obligation::generate(Proposal, &LedgerView)`: valida
objetivo/premissas, citabilidade/época e compatibilidade; ordena/deduplica digests
de eventos, não fórmulas. `Obligation` privado expõe somente leitura e codificação
canônica; não tem decoder/construtor público. Época não entra em h(O); assumptions
continuam antecedentes explícitos. LedgerView é snapshot abstrato confiável,
não ledger/autenticação implementados. Contrato em [`spec/obligation.md`](spec/obligation.md),
decisão em [`ADR 0012`](docs/adr/0012-geracao-de-obrigacoes.md).

Etapa 1.6 oferece `logos-protocol::{decode, encode}` fora da TCB, JSON versionado
com um único payload `Proposal`. Campos extras/duplicados e tipos inválidos são
recusados; objetivo contém AIR canônico, referências têm tipos distintos e texto
é `UntrustedText`, preservado mas excluído de O. `speculative_on` não habilita
Pending; h(V) ainda precisa de resolução confiável. Contrato/schema em
[`spec/protocol.md`](spec/protocol.md), decisão em
[`ADR 0013`](docs/adr/0013-protocolo-de-proposta.md).

Etapa 2.1 oferece `logos-model::FiniteModel`: construtor/decoder contextual
vinculado a Vocabulary real por h(V), domínios finitos, constantes, funções
densas totais e conjuntos ordenados de tuplas. Defined não recebe tabelas;
excesso de tamanho é Unknown(Size), nunca modelo truncado. Contrato em
[`spec/model.md`](spec/model.md), decisão em
[`ADR 0014`](docs/adr/0014-estruturas-finitas.md). Estrutura válida não prova refutação.

Etapa 2.2 oferece `logos-model::eval(&model, &vocabulary, &formula, &budget)`:
termos, conectivos e quantificadores integrais, expansão Defined com parâmetros
De Bruijn isolados e relações Const/Func totais com resultado único. Conferência
de h(V), tipagem de todos os corpos e compatibilidade ordinal antecedem fórmula.
Erros de entrada e Unknown(Steps) nunca viram booleano; somente steps é imposto,
wall ignorado e mem_bytes/size_bytes não são tetos físicos nesta API.
Contrato em [`spec/evaluation.md`](spec/evaluation.md), decisão em
[`ADR 0015`](docs/adr/0015-avaliacao-finita-defined.md). Checkers 2.3 ainda não
implementados; avaliador não emite Verdict.


Grounding é declarado, não demonstrado; parsing e tipagem não são prova lógica.
Kernel e gateway continuam sem implementação; estado em [`STATUS.md`](STATUS.md).
