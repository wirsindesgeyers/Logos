# Etapa 2.5 — model finder alternativo

## 1. Interfaces de entrada

Dependências 2.1/2.3 presentes: model/v1 contextual e checker Rust real.
AIR/canon/obligation Python e ponte checker_diff já existem; Z3 2.4 disponível
para composição. O/V são bytes originais. Proveniência e h(O) esperado são
responsabilidade do monitor; fixtures não autenticam eventos.

## 2. Interfaces de saída

ADR 0018: `model_finder.Limits(max_domain=4, max_steps=1_000_000,
timeout_ms=5000)`, `refute(O,V,limits=...) -> SearchResult(status, model)`.
Status FOUND contém bytes; NO_MODEL_WITHIN_BOUND completa espaço finito;
EXHAUSTED registra interrupção sem conclusão semântica. Nenhum status é
aceitação nem Verdict. `model_fallback.refute` tenta Z3 e, sem candidato,
tenta segundo produtor, retornando bytes/None. Sem integração ao farm.

## 3. Tarefas e arquivos

- Fatorar somente AST/validação/odômetro em model_context; migrar exceção sem shim.
- model_finder: enumeração primitiva e avaliador independente, Defined globais,
  Enum, quantificadores/De Bruijn, codificação existente, limites ≤4.
- model_fallback e smoke público com checker Rust e unknown nativo real.
- Testes independentes: todas as interpretações do corpus ≤3, presença/ausência,
  exemplos mistos/enum/Defined/nullary, orçamento/timeout, contexto inválido.
- Integrar smoke em verify/CI e documentar README/spec/STATUS/plano da Fase 2.

## 4. Critério de conclusão

ROADMAP 2.5: 100% dos casos refutáveis no corpus ≤3 encontrados, comparados
com enumeração independente e aceitos pelo checker real. Casos sem modelo
devem terminar NO_MODEL_WITHIN_BOUND; limite deve terminar EXHAUSTED, nunca
falsa ausência. Fallback exercitado após unknown verdadeiro de Z3 e candidato
aceito pelo checker. Smoke público e `ci/verify.sh --selftest` com exit 0.

## 5. Riscos e mitigação

Explosão combinatória: odômetros lazy e trabalho/tempo explícitos; nunca domínio
>4 no segundo produtor. Avaliação não é confiança: checker contextual confirma
candidatos. Defined não usado pode invalidar modelo: verificar todos globalmente.
Dependências são ordenadas sem expansão recursiva de cadeia. Sem credenciais,
ledger, farm, shims de formato ou promessa de completude fora dos limites.

## 6. Estimativa revisada

P mantido; apenas produtor alternativo, composição pequena e provas existentes.

## 7. Evidência

`uv run pytest -q -s tests/test_model_finder.py tests/test_model_fallback.py
tests/test_z3_models.py`, em `py/`: **41 passed**, 37,13 s, exit 0.
São 22 regressões da 2.5 e 19 da 2.4 preservadas após fatoração.

```text
EXHAUSTIVE_CORPUS_OK cases=296 models=682 refutable=201 no_model=95 accepted=201
Z3_CORPUS_OK controls=200 accepted=200/200
FAKE_Z3_OK attempts=10000 legitimate=2100 rejected=7900 undue=0
INVERTED_NEGATION_OK valid_model=True refutation=Rejected
```

Corpus: todas as 256 combinações de funções booleanas sobre dois átomos, mais
40 obrigações quantificadas/funcionais distintas. Referência enumera todos
os 2+32+648 modelos primitivos n=1,2,3 em mapas/sets, sem usar Z3 ou avaliador
do produtor. 201/201 casos refutáveis encontrados e aceitos pelo checker
Rust; os 95 negativos completaram NO_MODEL_WITHIN_BOUND. Nenhuma obrigação
duplicada por renomeação/event IDs. Completude demonstrada neste corpus, não
prova formal sobre fórmulas arbitrárias ou fora dos limites de recursos.

Regressões: busca que exige 3 e 4 elementos; cardinalidades mistas; Enum
Defined válido e ordinal incompatível; dependência fora de ordem; Defined
global inválido não usado; relação true única em singleton e não única em
domínio ≥2; parâmetros/binders isolados; nulários; função retangular 2×3
com seis resultados assimétricos; V vazio e enum vazio; contexto/tags bool
inválidos. Trabalho/clock determinístico/131072 células geram EXHAUSTED,
sem modelo parcial e sem falsa ausência.

Smokes observados:

```text
NO_Z3_FINDER_OK domain=[2] checker=Accepted bounded=no_model_within_bound budget=exhausted
MODEL_FINDER_SMOKE_OK domain=[2] unknown=max. resource limit exceeded checker=Accepted,Accepted,Rejected
Z3_MODELS_SMOKE_OK domain=[2] h(M)=1caf3aadcf0ea0b7736c06d5770bd133388f705e481fdd161de7965bffe8007c checker=Accepted,Accepted,Rejected,Rejected,Rejected
```

Primeiro: cenário descartável com import de Z3 bloqueado antes de carregar o
finder, fixture do smoke público e checker Rust real. Segundo: solver nativo
com rlimit=1; chama check real, observa unknown, restaura patch e aceita
candidato fallback. Não simula retorno unknown nem aceitação; wrong h(O)
é Rejected. Comandos públicos `python -m logos_provers.model_finder_smoke` e
`python -m logos_provers.z3_models_smoke` via uv, ambos exit 0.

Primeira rodada: 12 passed/1 failed; fixture considerava defined(true) não
único em singleton, mas singleton tem exatamente um resultado. Corrigida
premissa para exigir ≥2 e mantido controle positivo singleton. Timeout físico
de 1ms substituído por clock determinístico; removidos testes de wiring e
texto incidental. Ruff inicial 30 diagnósticos e mypy 17 corrigidos, sem
allow global; binding Z3 não tipado usa import localizado conforme 2.4.
Ruff/format/mypy finais passaram, 29 source files.

`ci/verify.sh --selftest`: **VERIFY_OK**, exit 0, 147,04 s. **217 Python**
(93,66 s) + **25 TCB** (1,34 s); fmt/clippy/tests Rust, guards/manifesto/
independência/supply-chain, vetores e smokes passaram. Selftest bloqueou todos
os controles negativos pelos motivos corretos. Apenas warnings deny anteriores.

`uv run --isolated --no-dev --locked python -m logos_provers.model_finder_smoke`:
dois pacotes instalados, MODEL_FINDER_SMOKE_OK, exit 0, 1,57 s.
Nenhum código/dep/arquivo Rust da TCB alterado; não existe parser/decoder TCB
novo para fuzz nesta etapa. CI/ARM/S3/fuzz longo anteriores continuam pendentes;
execução remota do job Python 2.5 ainda não observada. Sem push.

