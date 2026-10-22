# ADR 0055 — Caderno de pesquisa, retratação de operador e operações científicas

- Estado: aceito para implementação da Etapa 5R.3 (2026-10-22), por delegação do
  mantenedor ("pode prosseguir o desenvolvimento"; decisões competentes documentadas).
- Relacionadas: 0045 (MCP de pesquisa), 0044 (farm), 0052 (ATMS/retratação),
  0054 (assumptions com escopo), 0047/0049 (artefatos e projetos);
  `spec/notebook.md`, `spec/sat-producer.md`, `spec/prover-farm.md`.

## Contexto

A 5R.3 pede um caderno por projeto (hipóteses, previsões, evidências,
assumptions, conclusões), evidência externa só como nota não verificada ou
assumption explícita, exportação com retratação atualizando a citabilidade,
três operações formais (experimento crucial, MUS checado, leis contra
tabelas) e um bundle local `notebook/v1` alinhado a `ProjectArtifact` (WS§7.3).

Três lacunas apareceram no código existente:

1. Retratação (5.9) é API de operador sem caminho executável: o ledger tem
   `writer.lock` exclusivo e o protocolo do agente não tem RETRACT (ADR0052, P1).
2. Rotas independentes exigem duas provas da **mesma** h(O) com `used`
   diferentes; o produtor SAT sempre entrega ao solver todas as premissas.
3. Encolher um conflito até um MUS exige provas de ⊥ que não usem certas
   premissas, de novo sobre a mesma O.

## Decisão

1. **Caderno fora da TCB** (`py/logos_notebook`): bundle canônico `notebook/v1`
   com IDs de 16 bytes, proveniência `[fonte, localizador, método, ator, tempo]`,
   referências `[papel, h(Event), h(O)?]` e relações tipadas ordenadas. O bundle
   **nunca** guarda status como atual: `status_hint` é histórico rotulado.
   Evidência e hipótese só aceitam referência de papel ASSUMPTION (estrutural:
   o decoder recusa evidência com referência a resultado de checker).
2. **Status sempre consultado**: o relatório chama Query status/open no kernel na
   hora de gerar; "provável sob Γ" só aparece para Verified, com as assumptions
   abertas listadas. Rótulos de WS§11.1 (hipótese, evidência empírica, fonte
   afirma, modelo sugere) descrevem a natureza do item, não seu status lógico.
3. **Retratação de operador**: `logos ledger retract DIR --key KEYFILE --target
   EVENT --reason TEXT` (CLI não TCB) abre o ledger com a chave do kernel, com o
   kernel parado, e chama `Retractions::retract` com a mesma autoridade que o
   `logos-kernel` configura (sua chave pública). A TCB decide quem pode retratar;
   ao reiniciar, o kernel roda `reconcile`. O agente continua sem RETRACT.
4. **Premissas escondidas no produtor SAT** (não confiável): `hide` lista
   h(Event) citados em O; o encoder de apoio expõe os IDs das assertivas
   unitárias de premissa e o produtor as troca por uma tautologia na mesma
   posição, preservando IDs e a CNF de O. Provas que citem um ID escondido são
   descartadas; SAT nesse modo nunca vira modelo de O. O kernel recalcula `used`
   da prova; nada do produtor é confiado. O worker isolado da farm aceita `hide`
   só no modo SAT.
5. **Operações científicas** compõem o protocolo existente: (a) REFUTE de
   `H1 → H2` e `H2 → H1` sob Γ dá os dois cenários; previsões discriminantes só
   contam quando Γ,H1 ⊢ L e Γ,H2 ⊢ ¬L são ambos Verified; (b) MUS: Verified de
   Γ ⊢ ⊥ com `used = S` e, para cada p ∈ S, Refuted de `S − {p} ⊢ ⊥`; (c) cada
   linha de tabela vira `model/v1` proposicional, REFUTE certifica a linha que
   viola a lei; redundância entre sobreviventes (declaradas como assumptions de
   escopo próprio) por SAT/LRAT, independência por Refuted.
6. **Auditoria do corte**: além do auditor independente (assinaturas, cadeia,
   recheck LRAT/modelos), o caderno recalcula o ATMS do zero sobre o stream
   capturado e confere o conjunto morto contra o kernel e cada `Invalidated`
   como caminho `[Retracted raiz, premissa used morta]`.

## Alternativas consideradas

1. **RETRACT no socket do agente** (rejeitada): viola P1 e o ADR0052.
2. **Canal de operador no kernel vivo** (adiada): exigiria autenticação de
   operador no transporte; parar o kernel é suficiente para a 5R.3.
3. **Rotas independentes com obrigações diferentes** (rejeitada): ADR0052 define
   o nó como h(O); citações diferentes são nós diferentes e não demonstram o
   critério de rotas independentes.
4. **Extrair o MUS do `used` de uma prova só** (insuficiente): `used` é um
   núcleo, não necessariamente mínimo; minimalidade precisa dos modelos de S − {p}.
5. **Persistir o caderno em SQLite próprio** (rejeitada): WS§7.1 proíbe um
   store paralelo; o bundle local é importável pela WS.4.

## Consequências

- Formatos da TCB, eventos e protocolo inalterados. Novo formato fora da TCB
  (`notebook/v1`, vetores independentes) e novo campo opcional `premise_clauses`
  na resposta `encode` do encoder de apoio.
- Pendings não certificados podem ficar no ledger quando uma tentativa não acha
  prova/modelo; o `Lab` reaproveita a instância Pending da mesma h(O) quando pode.
- `until:` e `claim:` seguem aceitos no bundle como no ADR0054, mas o piloto usa
  só `session:`.
- O piloto é fixture sintética: nenhuma fonte real, nenhuma medição; o relatório
  diz isso em cada item. Não é avaliação científica (5R.6) nem estatística.
