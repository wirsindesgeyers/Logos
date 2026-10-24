# Etapa7.1 — linguagem de política

Estado: implementada localmente (2026-10-24). Decisão em [ADR 0069](../adr/0069-linguagem-de-politica.md);
especificação em [spec/policy.md](../../spec/policy.md).

1. **Entradas:** AIR, vocabulário e sort-check (1.1–1.3), parser de fórmulas (1.4), `says_P` (6.6).
2. **Saídas:** `logos-policy` (`Policy`, `Rule`, `Class`, `PolicyError`, codec `policy/v1`, forma de
   Horn, estratificação); `logos-policy-text` (`parse_policy`, `print_policy`, fora da TCB);
   vetores `spec/vectors/policy/{policy.json,vocabulary.bin}` (`tools/gen_policy_vectors.py`);
   fuzz `policy_decode`; `ci/check-policy-coverage.sh` e job `policy-coverage`.
3. **Critério → evidência:**
   - *Parser + sort-check de políticas com 100% de cobertura de ramos*:
     `ci/check-policy-coverage.sh` → `logos-policy/src` ramos 36/36 + 4/4 + 22/22 (regiões
     informativas 413/431: restam os `?` de hash/codificação sem caminho de erro). O parser de
     superfície é fora da TCB (ADR 0069 §5) e tem `logos-policy-text/tests/surface.rs`
     (5 testes: estrutura, posições, escapes, Unicode, splitter, round-trip, erros com o `id`).
   - *Política com ciclo pela negação é recusada*: `negation_cycles_are_refused` (autonegação,
     `p`↔`q`, `Allowed` negado no corpo `allow`, corpo `deny` que depende de `Allowed`, ciclo que
     começa no deny) e o vetor `negation-cycle`/`deny-depends-on-allowed`.
   - Forma e classes: `rules_outside_the_restricted_form_or_with_the_wrong_head_are_refused`;
     codec estrito: `decode_is_strict_and_revalidates_against_the_vocabulary`; vetores Python↔Rust
     byte a byte (`rust_reads_the_python_vocabulary_and_policies_byte_for_byte`,
     `surface_text_reproduces_the_python_vector_bytes`).
   - Mutações: 19 mortas, 1 equivalente por defesa em profundidade (P16, o teto de regras também é
     conferido em `validate`) — `etapa-7.1-mutacoes.json`. Fuzz `policy_decode` 60 s, 15 422
     execuções, sem crash (a meta de ≥ 1 h roda na matriz noturna).
4. **Riscos:** segurança por faixa e ordem de avaliação ficam para a 7.3; `require` só é checada
   sintaticamente; o parser de superfície depende do delimitador `rule ID CLASSE:`/`comment "…"`.
5. **Tamanho:** `M`, TCB.
