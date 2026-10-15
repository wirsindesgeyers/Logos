# Plano de fase — Fase 3: ledger assinado

Fonte: ROADMAP Fase 3 e Apêndice C; dependências locais implementadas até a3.6 (CI remota pendente).

1. **3.1**: catálogo event/v1, assinatura/envelope e vetores; não escreve ledger.
2. **3.2**: appender único e SQLite transacional/durável, cadeia/recuperação/crash.
3. **3.3**: índices reconstruíveis e LedgerView concreta; citabilidade/consultas.
4. **3.4**: semântica genérica de épocas, sem política da Fase 7.
5. **3.5**: CAS antes do append, GC somente dry-run, sem apagar referenciados.
6. **3.6**: auditor independente usa formatos/checkers, nunca depende de ledger
   ou kernel em código; recheca refutações e detecta adulterações.

Interfaces: canon/Event assinado → hash integral com sig → prev/referências.
Assinatura válida não prova cadeia/autoridade/lógica. CheckRecord preserva
Budget/CheckOutcome e relação com O; Rejected/Unknown não produzem Refuted.
Proposta preserva texto tainted como UTF-8 bytes, não política/fórmula confiável.

3.1 demonstrada localmente: [plano](etapa-3.1.md),
[ADR 0020](../adr/0020-catalogo-assinado-de-eventos.md),
[spec](../../spec/events.md). 3.2 demonstrada localmente:
[plano](etapa-3.2.md)/[spec](../../spec/ledger.md). 3.3 demonstrada localmente:
[plano](etapa-3.3.md)/[ADR 0022](../adr/0022-indices-e-ledgerview.md).
3.4 demonstrada localmente: mantenedor adotou divisão A do
[ADR0023](../adr/0023-contrato-de-epocas-e-fatos.md), mecanismo genérico agora
e teste integrado de fatos preservado na6.1 com dependência explícita de3.4.
[Plano](etapa-3.4.md). 3.5 demonstrada localmente: CAS obrigatório antes de assinar,
inventário do catálogo e GC somente dry-run, [plano](etapa-3.5.md)/
[ADR0024](../adr/0024-cas-antes-do-append-e-gc-dry-run.md). 3.6 demonstrada localmente, com independência remota ainda pendente:
checker independente do transporte (ADR0025), auditor/catálogo/leitores próprios,
recheck de modelos e export audit-stream/v1. [Plano](etapa-3.6.md).
CI remota/S3/ARM/fuzz longo anteriores continuam pendências separadas.
