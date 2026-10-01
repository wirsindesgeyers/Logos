# ADR 0002 — Divisão de linguagens

- **Status:** aceito
- **Data:** 2026-10-01

## Decisão

- **Rust**: toda a TCB (`logos-canon`, `-crypto`, `-core`, `-air`, `-obligation`, `-model`, `-ground`, `-lrat`, `-ledger`, `-seal`, `-policy`, `-token`, `-kernel`, `-gateway`), o auditor independente, a CLI de operação e a **TUI** (Ratatui, fora da TCB: só exibe, não assina).
- **Python 3.12+**: agente, servidor MCP, prover farm e avaliação (`py/`). Tudo isso é **não confiável** por construção: só fala com o kernel pelo protocolo e não tem chaves nem credenciais.

## Motivo

AgentDojo, CaMeL, FIDES, SDKs de LLM e a maioria dos solvers (Z3, cvc5, CaDiCaL) têm bindings e ecossistema em Python; a avaliação da Fase 11 fica muito mais barata assim. A TCB em Rust dá typestate, ausência de `unsafe` por lint e acesso a Kani/Verus. A interface de usuário segue a decisão original de produto em Rust.

## Alternativa considerada: tudo em Rust

Elimina a fronteira de linguagem e o teste diferencial de codificação canônica. Custo: reescrever ou embrulhar o harness de avaliação, o SDK dos LLMs e o prover farm. **O que mudaria se for adotada:** apenas as Fases 10 e 11 (`logos_client`, `logos_mcp`, `logos_agent`, `logos_eval`); a TCB e o protocolo ficam iguais.

## Consequências

- O protocolo é especificado por bytes (`spec/`), não por bibliotecas: o cliente Python reimplementa o encoder canônico e é testado contra o Rust (Etapa 0.3).
- `mypy --strict` e `ruff` valem no Python; `clippy -D warnings` no Rust.
