# Plano — Etapa 2.6: contramodelos em português

## 1. Entradas

`model/v1` contextual e `vocabulary/v1` com `gloss_pt`; `obligation/v1`
contém premissas e objetivo. Consumir bytes canônicos, validação AIR/contexto
existente e intérprete não confiável do finder. Dependências 2.1/1.2 prontas;
checker 2.3 disponível para demonstração, sem integração autoritativa.

## 2. Saída

`logos_provers.model_explain.explain_model(model: bytes, vocabulary: bytes,
obligation: bytes) -> str`. Português com h(M), domínios, valores/fatos relevantes,
valoração de cada premissa e objetivo AIR identificado. Erros estruturais ou
limites insuficientes: `model_context.ProducerError`, nunca texto parcial
apresentado como conclusão. Nenhum Verdict, aceitação ou fidelidade.

## 3. Tarefas

1. ADR 0019: bytes/contexto, nomes/ordinais, relevância sintática, glosas e limites.
2. `py/logos_provers/model_explain.py`: validar estrutura contextual, reutilizar
   avaliação/derivação Defined, filtrar apresentação pela sintaxe de O.
3. `py/logos_provers/model_explain_examples.py`, `py/tests/test_model_explain.py`: 20 casos distintos,
   checker Rust real; regressões de dependências ocultas, tabelas, glosas,
   premissas/goal, entradas inválidas e omissão explícita.
4. `model_explain_smoke.py`: corpus e saída legível, executar checker real;
   integrar `ci/verify.sh` e job Python existente.
5. Verificação completa/selftest; atualizar spec de consumo, plano da fase,
   ROADMAP e STATUS; commit coerente sem push.

## 4. Critério → evidência

20 refutações com objetivo falsificado e apenas símbolos de O → corpus público
reproduzível, regressão com checker Rust `Accepted` para cada modelo e smoke
`MODEL_EXPLAIN_SMOKE_OK`. Cada caso mostra objetivo AIR exato, hash independente,
glosas pertinentes e nenhuma declaração/dependência excluída. Smoke imprime
exemplo PT; full verify confirma integração sem alterar TCB.

## 5. Riscos e decisões

ADR 0019 resolve representação e limites. Glosa é dado livre, não referência
formal; aspas escapadas não conferem autenticidade. O/readers/fixtures não
provam proveniência: hash esperado de O deve vir do monitor no uso autoritativo.
Defined pode depender de símbolos ocultos: avaliar internamente, nunca expandir
na explicação. Premissa falsa não é refutação aceita; apresentar valoração real.
Não implementar paráfrase 9.1, kernel, ledger, farm ou memória.

## 6. Tamanho

P, conforme roadmap; sem mudanças de formatos, dependências ou TCB.

## 7. Evidência local

- Smoke público `MODEL_EXPLAIN_SMOKE_OK accepted=20/20 false_goals=20 hashes=20`;
  todos os certificados conferidos pelo checker Rust real antes da apresentação.
- 10 regressões: corpus, relevância/Defined, tabelas densas, valoração,
  entradas inválidas, omissão e representantes Enum/glosas com caracteres de controle.
- Smoke `uv run --isolated --no-dev --locked` passou sem dependências dev.
- `ci/verify.sh --selftest`: `VERIFY_OK`, exit 0, 150,71s;
  227 Python + 25 TCB. CI remota ainda não observada.
- TCB, formato model/v1 e dependências Rust inalterados.
