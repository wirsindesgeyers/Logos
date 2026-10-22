# ADR 0052 — ATMS por obrigação e retratação assinada

- Estado: aceito para implementação da Etapa 5.9 (2026-10-22), por delegação do
  mantenedor ("tome decisões competentes e deixe tudo documentado").
- Base: 48f5cec (implementado em worktree paralelo, rebaseado sobre a 9.1)
- Relacionadas: 0020 (eventos), 0022 (índices), 0033 (used), 0034 (typestate),
  0041 (certify); `spec/ledger-index.md` §3, `spec/state-machine.md`, `spec/events.md`.

## Contexto

A 5.9 pede retratação `Retract { ref, reason, authority_sig }` (emissor ou admin),
cascata pelas dependências normativas `used` e **rotas independentes**: um claim
com dois `Verified` sobre conjuntos `used` diferentes só cai se todas as rotas
caírem, como os labels de um ATMS. Eventos `Invalidated { claim, causes }`; nada é
apagado.

A projeção 3.3 já retirava citabilidade por fechamento transitivo **por evento**:
qualquer dependente de um evento morto caía, mesmo com outra rota viva. O
`ObligationCreated` não aponta para o `Proposed`, então o índice não chega à
fórmula sem o CAS. O protocolo v1 não tem mensagem de retratação.

## Decisão

1. **Nó = h(O).** Cada `Verified` com a mesma `h(O)` é uma rota (justificativa).
   Rotas novas surgem pelos fluxos existentes: propor de novo o mesmo objetivo com
   as mesmas citações gera a mesma `h(O)` (geração determinística) e uma nova
   instância Pending, certificável com outra prova. Assumptions e demais linhas são
   nós de si mesmas.
2. **Vivacidade.** Um nó h(O) está vivo se alguma rota sua ainda `Verified` tem
   todas as premissas `used` vivas; uma premissa é viva se for assumption não morta
   ou rota de nó h(O) vivo. Retratação (do alvo), `Invalidated` (do alvo) e
   revogação de checker (das rotas daquele checker) **matam** linhas; a projeção
   propaga por worklist até o ponto fixo. Quando um nó cai, todas as suas rotas
   viram `Invalidated` (terminal, monótono). Sem ciclos: premissas são anteriores.
3. **Estado por rota reflete o nó.** Uma rota cuja premissa morreu, mas cujo nó
   segue vivo por outra rota, continua `Verified` e citável; a rota retratada
   diretamente fica `Invalidated` e `retracted`. Com uma rota por nó, o resultado é
   idêntico ao fechamento transitivo anterior (testes 3.3 inalterados).
4. **Retratação é operação de operador, fora do protocolo do agente.**
   `Retractions::retract` aceita alvo `Verified` (só a autoridade administrativa) ou
   `Assumption` (autoridade ou o autor registrado). Assinatura de autoridade no
   formato já existente de `Retracted`. Alvo já retratado ou de outro tipo é erro.
5. **Registro da cascata.** Após o `Retracted`, o kernel grava um `Invalidated` por
   rota derrubada (exceto o próprio alvo, cujo registro é o `Retracted`), com
   `causes = [h(Retracted raiz), premissa morta imediata]`; a premissa é a de menor
   hash entre as que já estavam mortas, e a raiz é a da premissa. O caminho
   completo se reconstrói seguindo os registros. Tamanho fixo: um DAG profundo não
   gera eventos enormes nem custo quadrático.
6. **Crash-safe.** O índice deriva a cascata do próprio `Retracted`, então um crash
   antes dos `Invalidated` não deixa dependente citável. `reconcile` (idempotente,
   também chamado no arranque do `logos-kernel`) grava os registros faltantes,
   reaproveitando raízes já gravadas. Rotas que caíram só por revogação de checker
   não recebem registro aqui: são da 5.10.

## Alternativas consideradas

1. **Nó = fórmula (h(V), h(φ), scope)** (adiada): mais geral, mas exige o objetivo
   na projeção; sem campo de ligação nos eventos, o índice precisaria ler O do CAS
   na reconstrução. Pode vir numa versão futura do catálogo.
2. **Permitir segundo CERTIFY sobre O já Verified** (rejeitada): exigiria transição
   nova Verified→Verified e mudança no recover de testemunha; a reproposta já dá a
   mesma `h(O)` sem tocar no typestate.
3. **Cadeia de causas completa** (rejeitada): tamanho proporcional à profundidade
   do DAG, quadrático no total.
4. **Cascata só nos Invalidated escritos pelo kernel** (rejeitada): um crash entre
   appends deixaria dependentes citáveis.
5. **Mensagem RETRACT no protocolo** (rejeitada): o agente não retrata; P1.

## Consequências

- `logos-ledger` (TCB) troca o fechamento SQL por propagação ATMS; `logos-kernel`
  ganha `retract` e `recover_verified`. Formatos de evento e de índice inalterados;
  a semântica de §3 de `spec/ledger-index.md` muda e é documentada.
- Retratar continua não revalidando o mundo: L1. Não há "desretratação".
