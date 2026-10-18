# Plano da Fase 5 — kernel

2026-10-18. Escopo e ordem: ROADMAP5.1–5.12; não antecipar handlers ao
implementar os tipos. O caminho crítico é 5.1/5.2 → 5.3 → 5.4 → 5.5–5.11 → 5.12.

- 5.1 entrega Claim<S>, recibos tipados de append e a tabela normativa;
  eventos, Verdict e appender existentes são suficientes.
- 5.2 entrega transporte/staging; 5.3 une Proposal, generate e LedgerView real
  aos tipos, emitindo Proposed e ObligationCreated em duas transições.
- 5.4 integra checkers/registry/used e é dona da autorização de CERTIFY/Refute;
  5.5 integra consistência. Os tipos não tornam CheckOutcome uma credencial.
- 5.6–5.10 integram bloqueio de reproposta, assumptions, especulação,
  invalidação/cascata e revogação; 5.11 isola processos/chaves.
- 5.12 demonstra proposta → prova/modelo → ledger → claim no kernel mínimo.

5.1 e 5.2 não compartilham implementação; convergem em 5.3. A formalização
da Fase 12 usa spec/state-machine.md depois da Fase 5. Nenhum trabalho
paralelo de agentes é necessário neste plano.
