# ADR 0018 — model finder exaustivo fora da TCB

Estado: aceito (Etapa 2.5). Data: 2026-10-05.

## Contexto

ROADMAP 2.5 exige segundo produtor para domínios ≤4 e fallback quando Z3
retorna unknown. 2.1/2.3 fornecem model/v1 e checker contextual; 2.4 retorna
bytes ou None, sem distinguir unknown/timeout/unsat limitado. Nenhum deles
transforma ausência de candidato em prova geral de validade.

## Decisão

Implementar `logos_provers.model_finder`, sem importar Z3. Compartilhar somente
AST/validação/contexto AIR e odômetro de cardinalidades em `model_context`;
tradutor SMT, enumeração e avaliação permanecem independentes. Mover
`ProducerError` para esse contexto e migrar consumidores, sem alias legado.

`Limits(max_domain=4, max_steps=1_000_000, timeout_ms=5000)` valida inteiros
estritos, domínio 1..4, trabalho positivo e timeout positivo. `refute(O,V)`
retorna `SearchResult(status, model)` com status `FOUND`,
`NO_MODEL_WITHIN_BOUND` ou `EXHAUSTED`. Apenas FOUND contém bytes model/v1;
os outros nunca significam Verified, Accepted ou ausência de modelo geral.
NO_MODEL_WITHIN_BOUND exige completar todas as interpretações e cardinalidades
autorizadas. Trabalho/tempo/tamanho insuficiente retorna EXHAUSTED.

Enumerar todos os vetores Open em 1..k; Enumerated tem cardinalidade/ordinais
fixos. Enumerar interpretações primitivas por odômetro misto, sem materializar
produto de modelos. Funções densas variam último eixo mais rápido; predicados
enumeram cada decisão booleana. Derivar Defined em ordem de dependência,
ambientes De Bruijn isolados, e verificar totalidade/unicidade e ordinal de
Enum inclusive para símbolos não usados na obrigação. Não serializar Defined.

Contar trabalho durante preparação, interpretação, avaliação e codificação;
timeout monotônico cooperativo inicia antes da validação. Não prometer limite
físico de RSS/wall-time da validação ou de bibliotecas nativas. Não devolver
modelo parcial. Manter caps model/v1 padrão do checker (100k células, 1 MiB).

`logos_provers.model_fallback.refute` chama produtor Z3 primeiro e, se recebe
None, chama enumeração. Padrão Z3 usa domínio/timeout do fallback; limites Z3
explícitos continuam possíveis. Timeout por produtor, não SLA total único.
Unknown real portanto aciona fallback; None por timeout/unsat limitado também.
Erros de entrada/solver propagam, não viram ausência. Retorno externo bytes/None
preserva semântica não autoritativa da 2.4. A aceitação cabe ao checker Rust,
com h(O) esperado vindo do monitor. Não integrar farm nem aumentar domínios.

## Consequências

Busca completa dentro do limite é exponencial; orçamento insuficiente é
EXHAUSTED, não falsa conclusão de ausência. Testes com enumeração independente
de todas as interpretações para domínios ≤3 e checker Rust real demonstram
completude do corpus. Testar também cardinalidades distintas, Defined, enum,
nullary, nenhum modelo, esgotamento e unknown real. Sem dependências novas,
formatos/vetores/TCB/Rust runtime alterados.
