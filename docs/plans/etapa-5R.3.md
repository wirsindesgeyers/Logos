# Etapa5R.3 — caderno de claims, hipóteses e evidências

Estado: implementação local (2026-10-22) sobre c882c43 (pilha 9.1/5.9/5.6/5.7
verificada). Decisão em [ADR 0055](../adr/0055-caderno-de-pesquisa-e-operacoes-cientificas.md);
formato em [`spec/notebook.md`](../../spec/notebook.md).

1. **Entradas (prontas):** MCP/serviço 5R.1 (cliente 10.1, farm 10.2, staging,
   auditor independente), ASSUME/Query open 5.7, retratação/ATMS 5.9, índice 3.3,
   produtor SAT 4.6, modelos 2.4/2.5, explicação 2.6.
2. **Saídas:** pacote `py/logos_notebook` (bundle notebook/v1, `Lab`, `science`,
   `atms`, `report`, piloto e smoke); `logos ledger retract` (CLI não TCB);
   premissas escondidas no produtor SAT e no worker da farm; `snapshot`/`frames`
   reutilizáveis em `logos_mcp.service`.
3. **Tarefas:** ADR0055; spec notebook/v1 e vetores independentes; produtor SAT
   com `hide` + `premise_clauses` no encoder de apoio; CLI de retratação e testes;
   caderno/operações/relatório; testes unitários; `ci/check-notebook.sh` no verify.
4. **Critério → evidência** (`ci/check-notebook.sh`, relatório
   `target/notebook-report.{json,md,bundle}`):
   - *20 hipóteses com proveniência completa, nenhuma evidência como teorema*:
     piloto com 20 hipóteses e 6 evidências (5 assumptions, 1 nota), todas com
     fonte/localizador/método/ator/tempo; o smoke exige `provenance_complete` em
     todos os itens e `theorem_violations == []`. Estruturalmente, o decoder recusa
     evidência/hipótese com referência CHECK (`test_notebook.py`, vetores).
   - *Retirar uma premissa invalida exatamente os claims que perderam todas as
     rotas, preservando rotas independentes; a auditoria mostra o corte e `used`*:
     P1 tem duas rotas LRAT da mesma h(O) com `used` disjuntos (premissas
     escondidas). Retratar E3 derruba {E3, C4}; P1B (premissa morta) segue citável
     pela rota de P1; C3 cita E3 sem usá-la e sobrevive. Retratar E1 derruba
     {E1, P1, P1B, C1, C2, C3}. Conjuntos esperados fixos no smoke, iguais ao
     kernel, à recomputação ATMS independente sobre o stream autenticado e aos
     registros Invalidated `[raiz, premissa]`; relatório lista cita × usa.
   - Operações WS§11.1: experimento crucial (H12 × H13 sob E5: dois Refuted e
     previsões discriminantes Algas/Turbidez com quatro Verified), MUS checado
     ({E5, H12, H13} de seis premissas; três Refuted de S − {p}), leis × tabela
     (L2/L5 refutadas por linhas, L4/L7 redundantes, L1/L3/L6 independentes).
5. **Riscos/decisões:** retratação exige parar o kernel (writer.lock);
   tentativas sem prova podem deixar Pending; piloto é fixture sintética
   proposicional (CERTIFY recusa sorts enumerados sem Fact, 4.1/5.4); MCP não
   ganha ferramentas de caderno nesta etapa.
6. **Tamanho:** `M`. Sem mudança de runtime, formato ou dependência da TCB.
