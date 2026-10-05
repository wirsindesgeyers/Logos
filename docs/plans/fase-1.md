# Plano de fase — Fase 1: AIR

Fonte: [ROADMAP](../../ROADMAP.md), Fase 1. Estado detalhado em
[STATUS](../../STATUS.md); decisões da 1.1 em [ADR 0007](../adr/0007-sintaxe-air.md).
Este roteiro não implementa etapas futuras nem substitui seus planos próprios.

## Ordem e caminho crítico

`1.1 sintaxe → 1.2 vocabulário → 1.3 sort-check → 1.5 obrigações → 1.6 proposta`.
Após 1.3, a 1.4 (parsers fora da TCB) pode avançar em paralelo com 1.5/1.6.
A 1.5 recebe a sintaxe canônica, não depende do parser textual da 1.4.

## Interfaces entre etapas

| Interface | Origem → consumidor |
| --- | --- |
| Sort, Symbol, Term, Formula, LogicId; prefixo plano/De Bruijn/limites air/v1 | 1.1 → 1.2–1.6, Fase 2 |
| Vocabulary/SortDecl/SymbolDecl, grounding, h(V), compatibilidade entre versões | 1.2 → 1.3/1.5 e grounding futuro |
| Conferência contra Vocabulary | 1.3 → 1.4/1.5 |
| Texto → sintaxe canônica; pretty-printer de debug, fora da TCB | 1.4 → consumidores humanos/LLM; não concede autoridade |
| PremiseRef, Obligation, generate(proposal, LedgerView), h(O) | 1.5 → 1.6, Fases 2/3/5 |
| logos-protocol::Proposal/UntrustedText, JSON proposal/v1/schema P1 fora da TCB | 1.6 → servidor futuro 5.2; não resolve V nem autoriza especulação |

Na 1.2, `domain_source` é referência opaca, não implementação de fatos/políticas.
Na 1.5, `LedgerView` é interface abstrata; armazenamento real somente na 3.3.
Nenhuma etapa desta fase implementa kernel, gateway ou certificado de prova.

## Demonstração do marco

A fase fecha quando o objetivo/citações geram obrigação determinística sort-checada
com bytes/hash reproduzíveis; ordem das citações não altera h(O), citação inválida
é recusada e agente não fornece obrigação autoritativa. Critérios por etapa no
roadmap, sem antecipar checkmarks por estes planos.

1.1–1.6 demonstradas localmente por testes adversariais, propriedades,
vetores/diferenciais e smokes públicos; CI/ARM/fuzz noturno permanecem pendências.
1.6 fechou decoder estrito/P1 preliminar e taint sem alterar TCB; schema e seis
vetores independentes preservam formato. PROTOCOL_SMOKE_OK exercitou candidato
transmitido → gerador real, sem aceitar autoridade ou Pending.
Próxima implementação: Fase 2, Etapa 2.1, estruturas finitas; não há checker de
modelo, ledger assinado ou kernel implementado nesta fase.
