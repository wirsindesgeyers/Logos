# Etapa 2.4 — produtor Z3 de modelos

## 1. Interfaces de entrada

ROADMAP 2.4: somente produtor não confiável; sem provas de Verified ou integração ao farm.
Dependências presentes: canon/v1 Python; AIR/vocabulary/v1; obligation/v1 selado pelo
monitor; model/v1 e ModelChecker Rust contextual. Consumir bytes V e O originais;
autenticidade e hash esperado de O continuam responsabilidade do monitor.

## 2. Interfaces de saída

Módulo `logos_provers.z3_models`, fora da TCB:

```python
@dataclass(frozen=True)
class Limits:
    max_domain: int = 4
    timeout_ms: int = 5000

def refute(obligation: bytes, vocabulary: bytes, *, limits: Limits = Limits()) -> bytes | None: ...
def witness(obligation: bytes, vocabulary: bytes, *, limits: Limits = Limits()) -> bytes | None: ...
def to_smtlib(obligation: bytes, vocabulary: bytes, domains: tuple[int, ...], *, mode: str = "refute") -> str: ...
```

Retorno `bytes` é exclusivamente candidato model/v1, não aceitação/veredicto.
`None` significa nenhum candidato dentro dos limites (unsat finito, unknown ou timeout);
nunca prova ausência de contramodelo geral. Entrada inválida gera erro explícito.
`witness` consome Γ do envelope O: goal não entra na consulta de satisfatibilidade.

Transporte: SMT-LIB real via `z3.Solver.from_string`, bindings z3-solver fixados
5.1.0.0 como dependência Python de runtime, fora da TCB. Busca vetores de tamanhos
Open em 1..k; Enumerated fixa cardinalidade e ordinais. Nomes SMT posicionais, não
interpolação de nomes AIR. Defined: axiomas globais com ambientes De Bruijn isolados;
relações de valores equivalentes à igualdade ao resultado exigem totalidade/unicidade.
Não serializar tabelas Defined. Não mudar formatos, versões ou dependências Rust.

## 3. Tarefas e arquivos

1. Fixar decisões em ADR 0017 e este plano.
2. Implementar `py/logos_provers/z3_models.py`, limites/busca/extração; ajustar pyproject/lock.
3. Criar example `crates/logos-model/examples/checker_diff.rs`: batch de bytes V/O/M,
   hash esperado separado e modo; conferir h(O) original no checker real 2.3.
4. Criar testes `py/tests/test_z3_models.py` e fixtures `py/tests/z3_models_cases.py`,
   reutilizando ASTs/oráculo do diferencial 2.2; nenhum booleano Python representa checker.
5. Smoke público `py/logos_provers/z3_models_smoke.py` executa produtor real e ponte Rust.
6. Integrar smoke/pytest em verify e CI; full verify/selftest. Atualizar STATUS/ROADMAP.

Cada responsável por um arquivo é único; agentes não executam checks durante edição.

## 4. Critérios reproduzíveis

- `test_200_refutable_obligations_at_least_95_percent`: 200 O/contextos distintos,
  sem templates repetidos mudando só event IDs. Modelos conhecidos passam primeiro
  pelo checker; candidatos reais devem alcançar pelo menos 190 aceitações.
  Conectivos/quantificadores, múltiplos sorts, Enum/Open e Defined.
- `test_10k_fake_z3_zero_undue_accepted`: Z3 falso simulado na fronteira de bytes
  candidato → checker. 1000 controles positivos, 7000 valorações aleatórias e
  2000 defeitos estruturais isolados. Oráculo inteiro/sets de P[f(c,d)] ∧ c≠d ∧
  ¬Q[c,f(d,c)], independente de tradução/checker. Exigir decisões exatas.
- `test_inverted_negation_in_translator_rejected`: inverter sinal do goal no
  tradutor sem matching de texto SMT; modelo passa consistência, mas o checker
  recusa refutação contra O original. Controle sem mutação aceito.
- Regressões: busca que exige cardinalidade >1, fechamento Enumerated, diferenças
  entre witness/refute, pred/func nulários, tuplas/dense order, Defined não total/não
  único/ordinal incompatível, unknown/timeout sem certificado e entrada inválida.

## 5. Riscos e decisões

SMT solver inteiro não confiável; confiança permanece somente na aceitação Rust.
Busca finita incompleta; timeout total compartilhado entre consultas Z3 e checagens
cooperativas de tempo, não isolamento/hard RSS ou garantia física de parede.
Todos os vetores de cardinalidade Open precisam ser cobertos, não apenas tamanhos
iguais entre sorts. Enums maiores que limite também não produzem candidato.
Z3 unknown/timeout encerra chamada sem candidato. Extração exige elementos finitos
identificáveis, nunca truncamento/default em erro. ADR 0017 detalha escolhas.

## 6. Tamanho

M, conforme ROADMAP: tradução completa, extração contextual e provas adversariais.

## 7. Evidência

Execução seletiva observada:

```sh
cd py
uv run python -m logos_provers.z3_models_smoke
uv run pytest -q -s tests/test_z3_models.py
```

`Z3_MODELS_SMOKE_OK`: contramodelo exige domínio [2]; witness aceito como
consistência e rejeitado como refutação; h(O)/h(V) divergentes rejeitados.
`Z3_CORPUS_OK controls=200 accepted=200/200` (100%, acima dos 95% exigidos).
`FAKE_Z3_OK attempts=10000 legitimate=2100 rejected=7900 undue=0`.
`INVERTED_NEGATION_OK valid_model=True refutation=Rejected`.
19 regressões passaram em 31,87 s. Ruff/mypy estrito (22 arquivos) e clippy
workspace/all-targets passaram.

Antes/depois em runtime: V com versão bool gerava certificado, agora ProducerError;
1500 Not planos geravam RecursionError, agora ProducerError antes da recursão;
extração após deadline gerava certificado, agora None. Regressões permanentes
determinísticas. Removidos ASTs/encoders duplicados nas fixtures, templates
repetidos e teste de timeout que permitia bytes ou None sem observar validade.
Produção reusa validação estrutural AIR e axiomas Defined universais; não
materializa expansão cartesiana das definições. Busca é odômetro iterativo.
Hash esperado da ponte decodificado direto no array de 32 bytes, sem Vec/cópia.

Primeiro `ci/verify.sh --selftest` completo: VERIFY_OK, 195 Python + 25 TCB,
selftest com todos bloqueios pelos diagnósticos corretos (144,36 s).
Verificação final após ajuste de alocação da ponte: `ci/verify.sh --selftest`,
VERIFY_OK e código de saída 0 conferido por subprocess; 195 Python (86,81 s) +
25 TCB, todos smokes/vetores/guards/supply-chain/selftest passaram. Avisos
cargo-deny preexistentes (duas licenças não encontradas e syn 2/3), sem erro.
`CHECKER_BRIDGE_HASH_SMOKE_OK`: digest lower/uppercase aceito; digest curto,
longo, não hex e campos extras rejeitados. Sem alocação intermediária de digest.
Ruff inicial detectou defaults construídos/linhas/if; mypy detectou variáveis de
tipos distintos e acesso a importações privadas nos testes; corrigidos sem allow.
Smoke adicional de instalação runtime: `uv run --isolated --no-dev --locked python
-m logos_provers.z3_models_smoke` instalou somente dois pacotes e emitiu
Z3_MODELS_SMOKE_OK; nenhum pacote dev ou estado da venv de desenvolvimento exigido.
TCB correção permanece 6677+78287=84964; fidelidade1559+80034=81593;
build-only51581. Nenhum arquivo/dependência/formato/domínio TCB novo.
CI/ARM/S3/fuzz longo anteriores continuam não observados; não houve fuzz novo
nesta etapa Python fora da TCB. Rust-analyzer continua ausente na stable.

