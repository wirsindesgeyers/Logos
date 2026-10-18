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
| [`spec/events.md`](spec/events.md) | event/v1: catálogo, assinatura, checks e consistência explícita |
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
cargo run -p logos-ledger --example checker_smoke   # refutação/consistência, bytes/h(O) e CAS real
ci/check-audit.sh                                # auditor independente,4 refutações e export sem banco
cargo run -p logos-ground --example fragment_smoke # classificação/candidatos, fontes ainda opacas
cargo run -p logos-ground --example ground_smoke   # ground/v1 e hashes independentes, hipóteses explícitas
cargo run -p logos-ground --example cnf_smoke      # DIMACS/h(CNF) e vetores independentes
ci/check-cnf.sh                                  # 5 mil O contra SAT Z3 e tabela verdade
cargo run -p logos-ledger --example events_smoke   # eventos/autoridade reais, adulteração rejeitada
(cd py && uv run python -m logos_provers.z3_models_smoke)   # produtor Z3 → checker Rust real
(cd py && uv run python -m logos_provers.model_explain_smoke)   # 20 refutações → português + h(M)
ci/check-sort-coverage.sh   # 100% ramos LLVM; cargo-llvm-cov 0.9.1 + nightly/llvm-tools
(cd py && uv sync && uv run ruff check . && uv run mypy && uv run pytest)
(cd py && uv run pytest -q tests/test_air_differential.py)   # 500 fórmulas com AST esperada
(cd py && uv run pytest -q -s tests/test_model_differential.py)   # 10k pares (M, φ) × Z3
(cd py && uv run pytest -q -s tests/test_z3_models.py)   # corpus 200, Z3 falso 10k, negação invertida
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
continuam antecedentes explícitos. A interface LedgerView recebe implementação
autenticada na 3.3. Contrato em [`spec/obligation.md`](spec/obligation.md),
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
[`ADR 0015`](docs/adr/0015-avaliacao-finita-defined.md). Avaliador não emite Verdict.

Etapa 2.3 oferece `ModelChecker`: refutação exige Γ verdadeira e goal falso;
consistência exige Γ verdadeira, mas valida goal. Um orçamento cobre bytes,
contexto e avaliação. Reader contextual de O não abre o tipo gerado pelo monitor;
h(O) esperado confere os bytes originais. CAS verifica h(M) e preserva erros de
integridade/I/O. Contrato em [`spec/evaluation.md`](spec/evaluation.md), decisão em
[`ADR 0016`](docs/adr/0016-checkers-contextuais-de-modelos.md).
Desde3.6, métodos *_from_source recebem leitura falível por closure; o checker
confere h(M) novamente e logos-model independe de logos-ledger (ADR0025).

Etapa 2.4 oferece `logos_provers.z3_models`, **fora da TCB**: traduz O para
SMT-LIB, busca estruturas finitas limitadas e extrai candidatos model/v1.
`refute(o_bytes, v_bytes)` busca Γ ∧ ¬goal; `witness(o_bytes, v_bytes)` busca Γ.
`Limits(max_domain=4, timeout_ms=5000)` limita domínio e tempo compartilhado.
Retorno `None` não prova Verified/unsat geral; bytes retornados ainda precisam
passar pelo checker real contra h(O) vindo do monitor. Sem farm/claims/ledger.
Decisão/API em [`ADR 0017`](docs/adr/0017-produtor-z3-de-modelos.md) e
[`plano 2.4`](docs/plans/etapa-2.4.md). Uso:

```python
from logos_provers.z3_models import Limits, refute, witness

# o_bytes e v_bytes vêm do contexto do monitor; não são texto livre do agente.
countermodel = refute(o_bytes, v_bytes, limits=Limits(max_domain=3, timeout_ms=2000))
consistent_model = witness(o_bytes, v_bytes)
# Nenhum dos retornos é um veredicto; rejeição/Unknown continuam no checker Rust.
```

Etapa 2.5 oferece `logos_provers.model_finder`, independente de Z3 e **fora
da TCB**. Enumera interpretações primitivas e cardinalidades até 4; deriva
Defined globalmente antes de procurar Γ verdadeira e goal falso.
`Limits(max_domain=4, max_steps=1_000_000, timeout_ms=5000)` limita trabalho
e tempo cooperativamente. Resultado distingue `FOUND`, `NO_MODEL_WITHIN_BOUND`
(busca finita completa) e `EXHAUSTED` (sem conclusão). Ausência limitada nunca
prova validade geral; mesmo `FOUND` precisa do checker Rust.

```python
from logos_provers import model_fallback, model_finder
from logos_provers.model_context import ProducerError

limits = model_finder.Limits(max_domain=3, max_steps=1_000_000, timeout_ms=2000)
result = model_finder.refute(o_bytes, v_bytes, limits=limits)
if result.status is model_finder.SearchStatus.FOUND:
    countermodel = result.model  # ainda não aceito; conferir no checker contextual
fallback_candidate = model_fallback.refute(o_bytes, v_bytes, limits=limits)
# Z3 primeiro; None (inclusive unknown) aciona enumeração. Erros são ProducerError.
# Timeout é por produtor; nenhum dos dois recebe credenciais ou emite Verdict.
```

AST/contexto compartilhado e `ProducerError` ficam em `model_context`, não
reexportados pelo módulo Z3. Decisão em
[`ADR 0018`](docs/adr/0018-model-finder-exaustivo.md), provas no
[`plano 2.5`](docs/plans/etapa-2.5.md). Smoke, em `py/`:
`uv run python -m logos_provers.model_finder_smoke`.

Etapa 2.6 oferece apresentação portuguesa **fora da TCB**, sem garantia de
fidelidade ou aceitação. Glosas descrevem somente símbolos presentes em O;
dependências Defined são avaliadas sem aparecer no texto. Domínios usam
ordinais; fatos/valores limitados a 100 entradas com omissão explícita.
Premissas falsas e objetivo verdadeiro não são mascarados.

```python
from logos_provers.model_explain import explain_model

text = explain_model(m_bytes, v_bytes, o_bytes)  # inclui h(M), não emite Verdict
print(text)
```

Contrato/limites em [`spec/model.md`](spec/model.md#8-apresentação-não-autoritativa-etapa-26)
e [`ADR 0019`](docs/adr/0019-apresentacao-de-contramodelos.md).
`uv run python -m logos_provers.model_explain_smoke` em `py/` confere
20 refutações pelo checker Rust real e imprime exemplo legível.

## Eventos assinados disponíveis

Etapa 3.1 acrescenta `logos_ledger::Event`/`EventBody`: 13 tipos, codificação
canônica e `Hash<Event>` que inclui assinatura. `Event::sign` recebe
seq/epoch/prev/body e um `Signer`; `Event::decode` rejeita wire inválido;
`Event::verify` vincula assinatura e KeyId à chave pública fornecida pelo consumidor.
Checks preservam h(O)/h(cert)/h(checker)/Budget/CheckOutcome;
Verified registra used ⊆ cited e consistência Witness ou Unknown.

Recibos administrativos são verificados separadamente por
`EventBody::verify_authority`; chave assinante não implica privilégio.
Propostas preservam texto tainted UTF-8, inclusive não NFC, sem normalização.
Assinatura não demonstra lógica, continuidade da cadeia ou autorização.
Contrato em [`spec/events.md`](spec/events.md), decisão em
[`ADR 0020`](docs/adr/0020-catalogo-assinado-de-eventos.md).

Etapa 3.2 acrescenta `Ledger::open(directory, &signer)` e `append(body)`:
SQLite WAL/FULL, lock exclusivo de escritor e replay integral de
seq/prev/assinaturas antes de aceitar novas entradas. O appender calcula
cabeçalho e só devolve o evento após commit/sync. Falha de persistência
exige reabertura. Teste `ci/check-ledger-crash.sh`: SIGKILL durante 10k
appends, com oráculo Python independente. Integração CAS antes do append é da
3.5; proteção contra truncamento de sufixo válido é da 13.2.
Contrato em [`spec/ledger.md`](spec/ledger.md), decisão em
[`ADR 0021`](docs/adr/0021-appender-sqlite-duravel.md).

Etapa 3.3 acrescenta `LedgerIndex::open(directory, public_key)` e
`Ledger::index()`: claims/deps/citable derivados da cadeia conferida,
consultas de estado/época/dependentes e `snapshot(cas, refs, scope)` para
gerar novas obrigações com Formula/Vocabulary autenticados. Deps vêm de used;
retratação/invalidação/revogação retiram citabilidade dos dependentes.
Cache index.sqlite3 é comparado e reconstruído, sem servir como autoridade;
consultas usam snapshot SQLite privado em memória, com lock de escritor.
`logos ledger rebuild-index DIR --public-key HEX` usa somente chave pública.
`ci/check-ledger-index.sh` demonstra reconstrução de 100k eventos e p99<1ms
para is_citable com 1M, excluindo o custo de reconstrução.
Contrato em [`spec/ledger-index.md`](spec/ledger-index.md), decisão em
[`ADR 0022`](docs/adr/0022-indices-e-ledgerview.md).

Etapa 3.4 oferece `logos_core::EpochRange`, intervalo inclusivo u64 com
construtor falível, sem autenticar entradas. `EpochAdvanced` acrescenta
`policy_root`/`vocab` opcionais; o corpo estendido usa epoch-body/v2 e os bytes
históricos continuam iguais. LedgerView/gerador reais recusam premissas
anteriores após avanço da época. Fatos com janela serão integrados na 6.1.
Contrato em [`spec/epochs.md`](spec/epochs.md) e
[`spec/events.md`](spec/events.md), decisão em
[`ADR0023`](docs/adr/0023-contrato-de-epocas-e-fatos.md).

Etapa 3.5 associa o appender ao CAS em `DIRECTORY/cas`: `ledger.cas().put(...)`
precede `append`, que confere todas as referências, inclusive as de checks
de consistência, antes de assinar. Hash ausente, domínio errado, corrupção,
limite ou caminho irregular falham sem consumir seq/head. Histórico anterior
continua legível, sem migração dos bytes assinados.
`logos ledger gc --dry-run DIR --public-key HEX` usa somente chave pública,
lista candidatos a órfãos e temporários e conserva links canônicos de forma
conservadora; não remove arquivos. SQLite pode manter sidecars de coordenação
durante leitura. Contrato em [`spec/ledger-cas.md`](spec/ledger-cas.md), decisão
em [`ADR 0024`](docs/adr/0024-cas-antes-do-append-e-gc-dry-run.md).

Etapa3.6 oferece `logos-audit verify DIR --cas CAS --public-key HEX`:
decoder independente, cadeia/assinaturas/épocas, integridade dos objetos e
recheck de todas as refutações e testemunhas de consistência. Relatório
separa provas Verified ainda sem checker LRAT. `logos-audit export DIR
--public-key HEX` exporta audit-stream/v1 autenticado; o mesmo verify aceita
esse arquivo sem banco original. Chave externa obrigatória, sem kernel/ledger
na árvore de dependências; contrato em [`spec/audit.md`](spec/audit.md).

O status semântico dos símbolos é declarado; parsing e tipagem não são prova lógica.
Etapa4.1 oferece classificação conservadora de O/V e candidatos de fecho/
nomes únicos com fonte declarada, sem modificar O ou emitir claims. Integrações
autenticadas permanecem em6.1/7.3/7.5 por decisão do mantenedor (ADR0026).
Contrato e limites em [`spec/finite-fragment.md`](spec/finite-fragment.md).
Etapa4.2 traduz obrigações finitas com axiomas explícitos para DAG proposicional
canônico (`ground/v1`), preservando premissas e referências. Funções/Defined
mantêm suas restrições semânticas e a construção tem limites explícitos.
Contrato em [`spec/grounding.md`](spec/grounding.md); DIMACS permanece na4.3
conforme [`ADR0027`](docs/adr/0027-contrato-de-grounding-e-determinismo.md).
Etapa4.3 acrescenta Tseitin canônico, mapa de proveniência com suporte dos
axiomas, cnf-v1 no CAS e rederivação O→CNF pelo auditor. Contrato em
[`spec/cnf.md`](spec/cnf.md); provas LRAT ainda aguardam as4.4/4.5.

Etapa5.1 inicia `logos-kernel` com `Claim<S>` e recibos `SignedEvent<S>`
imutáveis. Transições internas consomem o claim anterior e só publicam o novo
estado após append confirmado; não existe emissão pública a partir de
`Event` ou `CheckOutcome`. Estados, vínculos e tabela em
[`spec/state-machine.md`](spec/state-machine.md). Servidor, handlers de
certificação e gateway permanecem nas próximas etapas; estado em
[`STATUS.md`](STATUS.md).
