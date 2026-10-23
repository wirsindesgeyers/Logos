# ADR 0060 — Demo do kernel mínimo ponta a ponta

- Estado: aceito para implementação da Etapa 5.12 (2026-10-23), por delegação do
  mantenedor. Revisável.
- Relacionadas: 0052 (retratação/ATMS), 0054 (escopos), 0058 (especulação),
  0059 (isolamento); Fase 6 (objeto `Fact` e atestadores reais).

## Contexto

A 5.12 pede uma demo scriptada: vocabulário → fatos de teste inseridos por um
atestador de teste → 3 propostas → uma Verified via LRAT, uma Refuted via modelo,
uma Unknown por timeout → retratação de um fato → cascata → `logos-audit` verifica e
recheca tudo, com um comando (`make demo-kernel`). O objeto `Fact` e os atestadores
reais são da Fase 6 (6.1/6.3): o catálogo de eventos ainda não tem fato.

## Decisão

1. **Atestador stub = assumption assinada.** Até a 6.1, o atestador de teste tem
   chave própria e grava `AssumptionDeclared { scope: "session:fatos-de-teste",
   author: atestador }`. A retratação é feita pelo próprio atestador (autor), pelo
   caminho da 5.9; a cascata é a do ATMS. Nenhum formato novo, e a 6.1 substitui o
   stub por `Fact` sem mudar a demo além desse passo.
2. **Transporte real em processo.** A demo (`crates/logos-kernel/examples/demo_kernel.rs`)
   fala com os handlers do kernel pelo socket e frame reais (`Server::serve_stream`),
   como os testes de integração, para fixar um orçamento próprio sem mudar o
   binário `logos-kernel`.
3. **"Timeout" determinístico.** Checkers internos ignoram tempo de relógio
   (`Budget.wall_ms`); o timeout deles é o teto de passos. A demo usa 50 000 passos e,
   para C, uma prova LRAT válida com 100 000 cópias RUP de uma cláusula antes da
   cláusula vazia: o checker esgota o orçamento e o kernel grava `UnknownRecorded`.
   Tempo de relógio deixaria a demo instável sob carga.
4. **Auditoria independente.** `ci/demo-kernel.sh` (alvo `make demo-kernel`, no
   verify) roda o `logos-audit` sobre o ledger da demo, com `cake_lpr` e a identidade
   do checker Rust impressa pela demo, e exige `AUDIT_OK`, a refutação rechecada e a
   prova LRAT rechecada pelos dois checkers.

## Consequências

- A demo é o marco do item 1 da §12 com fatos de teste, não fatos atestados de
  verdade; a Fase 6 troca o passo 2.
- Certificados entram por CAS (como nos testes); o caminho por staging já é coberto
  pelos testes da 5.2 e pelo cliente Python.
