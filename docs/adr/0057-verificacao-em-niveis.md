# ADR 0057 — Verificação em níveis (rápido, afetado, completo)

- Estado: aceito em 2026-10-22, por delegação do mantenedor, depois de ele perguntar
  se o desenvolvimento não ficaria insustentável com a demora dos testes ("Pode
  fazer essas alterações"). Revisa o ADR0051 (itens 2 e 3). Revisável a pedido dele.
- Relacionadas: 0051 (verificação por pilha), AGENTS.md §2, §4 e §5.

## Contexto

`ci/verify.sh --selftest` leva de 50 a 80 min nesta máquina, e cresce a cada etapa:
cada critério de escala (corpus LRAT inteiro, 50k fórmulas por vocabulário no
selo, 100k casos do diferencial canon, 10k obligations × Z3, farm 500, índice 1M)
vira teste permanente e roda por inteiro em toda verificação. Medições na ponta
ae32873: `certify.rs` 391 s, `corpus.rs` (LRAT) 230 s, `properties.rs` (seal) 92 s,
86 binários Rust somando ~890 s, pytest ~8 min. A verificação "direcionada" do
ADR0051 era ad hoc (cada agente escolhia o que rodar) e já deixou passar uma falha
óbvia (vetor de transporte SAT da 5R.3, pego só pelo completo uma hora depois).

Em 2026-10-22 o completo da pilha 5R.3+5.10 ficou mais de 4 h parado no benchmark
de índice com 1M eventos: a 5.10 tinha introduzido um `refresh()` O(n) por
Verified (O(n²) no total). Nenhum teste pequeno pegaria isso; o nível completo
continua indispensável, mas não pode ser o laço de desenvolvimento.

## Decisão

1. **Três níveis no mesmo script** (`ci/verify.sh --tier fast|affected|full`,
   padrão `full`), para que o atalho local nunca divirja do que a CI roda:
   - `fast`: build do cake_lpr, fmt, clippy, `cargo test --workspace` com
     `LOGOS_TIER=fast`, lints/manifesto da TCB, Ruff/mypy, todos os vetores
     `--check`, pytest sem o marcador `heavy`. Sem smokes de integração, sem rede.
     Exigido para todo commit local (substitui a "direcionada" do ADR0051).
   - `affected`: `fast` + os smokes e `ci/check-*.sh` das áreas tocadas desde
     `--base` (padrão `origin/main`), escolhidos por `tools/verify_plan.py`.
   - `full`: tudo, em escala de critério, + cargo deny/audit; `--selftest`
     acrescenta o autoteste. Exigido antes de push e para marcar critérios
     (ADR0051 itens 3–4 seguem valendo).
2. **Escala por nível, mesma lógica.** Testes de escala leem `LOGOS_TIER`
   (Rust: `std::env::var`; Python: `tests/tier.py::scaled(full, fast)`) e rodam
   uma amostra da mesma lógica no rápido: corpus LRAT 1 a cada 10, selo 2k
   fórmulas por vocabulário, corrupções de certificado do `certify.rs` 1 a cada 16
   posições, diferencial canon 5k, diferencial de modelos 1k, ATMS de referência
   1k claims. Testes cuja asserção depende do tamanho exato (Z3 falso 10k) levam
   `@pytest.mark.heavy` e só rodam no completo. Sem `LOGOS_TIER` o padrão é a
   escala completa: `cargo test` puro e a CI continuam provando os critérios.
3. **Seleção conservadora no afetado.** `tools/verify_plan.py` mapeia caminhos para
   smokes pelo fecho reverso de dependências do `cargo metadata` e por dependências
   Python declaradas. Docs/spec/testes Python/geradores não selecionam smokes (o
   rápido já os cobre). Caminho sem regra (Cargo.lock, toolchain, `ci/verify.sh`,
   `tools/build_*`, pyproject), base inválida ou desconhecida: seleciona tudo.
4. **Cronometragem.** Cada etapa grava sua duração em `target/verify-times.tsv` e
   imprime `STEP_OK nome segundos`; uma falha imprime `VERIFY_FAILED tier= step=`;
   o fim mostra as 10 etapas mais lentas e `VERIFY_OK tier=<nível>`. Uma etapa que
   cresce aparece na hora, não depois de horas sem saída.
5. **Criptografia otimizada nos perfis de teste/dev.** A medição por teste mostrou
   `witness_deadline_is_inclusive…` com 332 s dos 393 s do `certify.rs`: cada
   append assina e verifica Ed25519 com `curve25519-dalek` em opt-level 0 (~0,3 s
   por evento; a janela de testemunha tem 1024 eventos). `curve25519-dalek` e
   `ed25519-dalek` ganham `opt-level = 1` em `profile.test` (com overflow/debug
   mantidos, como `sha2`/`logos-crypto` nos ADRs 0029/0031) e em `profile.dev`.
   O perfil de produção não muda. Esse teste caiu para 19 s; o `certify.rs`
   inteiro para 64 s. `tools/lrat_corpus.py --check` confere hashes em todos os
   casos e a releitura RUP em Python em 1 de cada 10 no rápido (145 s → 30 s).
6. **Sem paralelização interna por enquanto.** Os scripts sensíveis a carga (crash
   do ledger com limites de 10/60 s, testemunhas com prazo, farm) dão falso
   negativo sob CPU disputada; paralelizá-los exigiria afrouxar limites que são
   critério. Fica para depois de medir com `verify-times.tsv`.

## Alternativas consideradas

1. **Só reduzir os testes de escala** (rejeitada): perderia a prova contínua dos
   critérios; o completo segue com a escala original.
2. **Pular testes pesados por `#[ignore]`** (rejeitada para Rust): o código deixaria
   de ser exercitado no laço rápido; a amostra pega erros de lógica com custo baixo.
3. **Mover o completo só para a CI** (rejeitada por ora): `gh` não está instalado e
   a CI não roda corpus/índice 1M inteiros; o completo local é o recibo.

## Consequências

- Laço de desenvolvimento em minutos: `--tier fast` passou de 23 min (só com a
  escala reduzida) para 8 min 15 s com a criptografia otimizada, incluindo a
  recompilação das dependências (`cargo test` 285 s, pytest 151 s, corpus 30 s).
  O completo também encolhe, porque usa os mesmos perfis.
- Regressões de desempenho só aparecem no completo: ele continua obrigatório antes
  do push, e a cronometragem por etapa torna o crescimento visível.
- Um commit local verificado só pelo rápido pode quebrar um smoke; o afetado reduz
  isso e o completo na ponta da pilha pega o resto antes de sair da máquina.

## Adendo (2026-10-23, Etapa 5.8)

O teste de profundidade de propagação (1000 cadeias pelo kernel real) mostrou outro
custo fixo: o checker Rust é o próprio executável, e o registro e cada evento que o
referencia rehasheiam o objeto no CAS (`verified_present`, regra de integridade que
não muda). Com debuginfo completo o binário de teste tinha ~43 MB. O perfil `dev`
(herdado por `test`) passa a `debug = "line-tables-only"` (~22 MB, backtraces com
linha) e `sha2` sobe para `opt-level = 3` no perfil de teste, com checks mantidos.
O teste também agrupa 25 cadeias por ledger e roda os lotes em paralelo (até 8
threads): 1000 cadeias em ~7 min no completo, 40 em ~36 s no rápido.
