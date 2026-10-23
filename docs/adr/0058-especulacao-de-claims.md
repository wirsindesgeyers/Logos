# ADR 0058 — Especulação de claims sobre Pending

- Estado: aceito para implementação da Etapa 5.8 (2026-10-23), por delegação do
  mantenedor ("tome a decisão que for melhor e deixe documentado"). Revisável.
- Relacionadas: §3 da spec conceitual ("Especulação: permitida para claims,
  proibida para efeitos"), 0012 (geração de obrigações), 0052 (ATMS/retratação),
  0054 (escopos), 0056 (revogação/recheck); `spec/kernel-protocol.md`,
  `spec/obligation.md`, `spec/state-machine.md`.

## Contexto

A 5.8 pede `Proposal.speculative_on: [ClaimId]` para citar `Pending`: o claim
resultante fica `Pending(conditional_on = …)`; quando a base é Verified os
especulativos são checados normalmente; quando a base é Refuted/Invalidated, eles
caem em cascata; claim especulativo nunca é citável por ação (P5). Critérios: C8
especula sobre C7, C7 refutado → C8 `Invalidated` com causa `SpeculationFailed(C7)`;
profundidade de propagação de erro = 0 sem especulação, em 1000 cadeias aleatórias.

O campo já existe no evento `Proposed` (events.md §2), no protocolo do kernel e no
cliente Python, mas o kernel recusava qualquer especulação não vazia.

## Decisão

1. **Identidade da base.** `speculative_on` contém hashes de eventos
   `ObligationCreated` (o claim `Pending`; o agente o obtém por Subscribe, como já
   diz kernel-protocol). A base precisa ser uma instância Proposed+ObligationCreated
   do ledger, na época corrente, não invalidada e **não concluída** (sem Verified
   nem Refuted para o seu h(O)). Base já Verified deve ser citada em `cites`, não
   especulada. Não há formato novo: evento, protocolo e obligation/v1 ficam iguais.
2. **Obrigação especulativa.** `logos_obligation::generate_speculative` monta
   Γ = citações ∪ bases; cada base entra como premissa com a meta da sua obrigação
   (o índice já resolve Pending pela meta do O no CAS) e precisa ter kind
   `Pending` — qualquer outro kind é `NotCitable`. Mesmas regras de época,
   retratação, vocabulário e sort. O agente pode provar O8 enquanto C7 está
   pendente: é esse o ganho de latência.
3. **Escopo.** O escopo derivado (ADR0054) considera as citações da proposta e as
   citações das bases, fechadas transitivamente sobre bases especulativas. A
   regeneração no CERTIFY/REFUTE repete o mesmo cálculo.
4. **CERTIFY só com base Verified.** Enquanto alguma base não tiver rota Verified
   ativa e citável na época, CERTIFY do especulativo é recusado sem evento (o claim
   continua Pending condicional). Com todas as bases ativas, para cada base o kernel
   escolhe a menor rota Verified ativa do nó h(O) da base; essas rotas entram em
   `cited` do Verified e substituem a base em `used` quando a prova a usa. O
   Verified resultante é um claim comum: a partir daí vale o ATMS da 5.9/5.10.
5. **REFUTE permitido; WITNESS não.** Um contramodelo de O8 refuta O8 qualquer que
   seja o estado das bases. WITNESS (consistência das premissas) continua recusado
   para proposta especulativa: consistência de Γ com premissa não verificada não
   vira autoridade.
6. **Cascata `SpeculationFailed`.** Uma base falha quando: sua linha foi
   invalidada (especulação dela falhou), seu h(O) foi refutado, ou seu nó h(O) teve
   rotas e nenhuma está viva. Todo Pending especulativo não concluído com base
   falha recebe `Invalidated { claim: h(Pending), causes: [raiz, h(base)] }`; a raiz
   é o Refuted da base, a raiz já registrada da base invalidada ou da rota morta,
   ou a própria rota nascida morta. `SpeculationFailed(C7)` é exatamente esse
   caminho cujo segundo elemento é a base listada em `speculative_on`. A varredura
   é idempotente, roda em ordem de seq (bases sempre anteriores), e é chamada após
   todo Refuted e no fim de `Retractions::reconcile` (retratação, recheck e boot do
   kernel, que já reconciliava crashes da 5.9).
7. **P5.** Pending (especulativo ou não) nunca é citável: nem por `cites` do
   PROPOSE nem por ação. O único caminho para um especulativo virar citável é o
   CERTIFY do item 4, que exige bases Verified.

## Alternativas consideradas

1. **`speculative_on` com h(O) da base** (rejeitada): o agente já tem h(O), mas a
   premissa de O8 precisaria de uma referência que não é evento, quebrando o
   namespace de PremiseRef, e várias instâncias Pending do mesmo O seriam ambíguas.
2. **Verificar o especulativo antes da base e guardar um "Verified condicional"**
   (rejeitada): estado novo citável-em-parte, mais superfície na TCB e no
   auditor; o roadmap pede checagem normal depois da base.
3. **Arestas especulativas no índice** (rejeitada por ora): exigiria layout novo
   (v4) e ligar Proposed ao ObligationCreated na projeção; o kernel reconstrói as
   relações por replay, como já faz no CERTIFY e no reconcile.
4. **Gerar O8 só quando a base for Verified** (rejeitada): o agente não teria O8
   para provar enquanto espera, que é o objetivo da especulação.

## Consequências

- Nenhum formato serializado muda; `kernel-protocol.md` deixa de dizer que
  especulação é inválida e documenta as regras acima.
- O varredor custa um replay completo por refutação/reconcile (mesma ordem do
  CERTIFY atual).
- O auditor independente não valida a semântica de `Invalidated` (como na 5.9):
  ele rechecka provas e modelos; a cascata é conferida no índice e nos testes.
