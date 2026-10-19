# Etapa 5.3 — `PROPOSE` e regra de citação

- Estado: concluída localmente; CI/ARM ainda não observados; ADR0038
- Data: 2026-10-19 (plano original18/10)
- Base: 6374755 (5.2 fechada e commitada)
- Depende de: 5.1 (typestate), 5.2 (transporte), 1.5 (`generate`), 1.6 (`proposal/v1`),
  3.3 (`LedgerView`/índice)
- Fora do escopo: `CERTIFY`/`REFUTE` (5.4), especulação (5.8), assumptions com escopo
  (5.7), ações (8.2), autoria criptográfica do agente (5.11)

## 1. Objetivo

Ligar o primeiro handler semântico do kernel: receber `Propose`, aplicar a regra de
citação da §3, gerar a obrigação canônica com `logos-obligation::generate`, publicá-la no
CAS e registrar `Proposed` + `ObligationCreated` no ledger. O agente recebe `h(O)` e os
bytes canônicos de `O` para levar ao prover. Nenhum passo aceita prova, modelo ou
autorização.

Critérios da etapa no `ROADMAP.md`:

- P1: 50 tentativas de um agente malicioso (citar não verificado, mandar obrigação pronta,
  mandar fato, citar evento de outra época, citar claim retratado) — todas recusadas.
- Reproduzir o "STATE 14" da §3 como teste de integração.

## 2. Decisões de implementação

### D1 — Autor do evento `Proposed`

O wire `kernel-protocol/v1` não carrega chave do agente, e o processo só autentica o UID
do SO. Na v1, o evento `Proposed` é assinado pelo kernel e o campo `author` recebe o
`KeyId` do Genesis autenticado do próprio ledger; a conexão é autenticada por
UID via `SO_PEERCRED`. Isso **não** é uma assinatura do agente e não será tratado como tal. A
ligação agente↔chave fica para a 5.11 (isolamento) ou etapa posterior; se ela for
antecipada, este campo e o protocolo precisam de decisão própria.

Consequência: o UID não é persistido no evento; não há atribuição individual
posterior nem distinção de agentes no ledger. Handler sem campos evita chave
de autoria configurada diferente da chave real. Não inventar uma chave de agente no payload.

### D2 — Rota e artefato

`ObligationCreated` recebe `Route::Model` e `artifact = h(O)` (bytes crus). A rota é
metadado de identidade; a seleção real de checker por `format` é da 5.4. Não há
`CnfRecord` na 5.3 (`cnf: None`).

### D3 — Especulação

`speculative_on` não vazio é recusado com `Invalid("speculation requires 5.8")` e nenhum
evento é criado. Não ignorar o campo silenciosamente: a 5.8 é quem define a citação de
`Pending` e a cascata.

### D4 — Escopo de assumptions

`LedgerIndex::snapshot` recebe `scope = ""`. Assumptions com escopo não vazio são
recusadas pela própria snapshot até a 5.7 definir o escopo da proposta. Isso é explícito,
não um `NotCitable` genérico.

### D5 — Resposta de sucesso

Novo `propose-result/v1`: `[1, h(O) bytes32, O bytes canônicos]`. O1 interno é a
versão do resultado novo (ADR0038). O agente usa os bytes para o prover; o ledger guarda
o objeto no CAS e o hash no evento. A resposta não é credencial nem veredicto.

### D6 — Ordem e falhas

1. Validar `speculative_on` e resolver `V` do CAS (sem V não há `generate`).
2. Montar `PremiseRef`s e a snapshot; `generate` aplica a regra de citação completa
   (existência, época, retratação, categoria, compatibilidade de vocabulário, sort-check).
3. Serializar/limitar `O` a1MiB, publicar no CAS e preparar o resultado antes dos appends.
4. `Transitions::propose` (append `Proposed`) e `submit` (append `ObligationCreated`).
5. Responder.

Proposta inválida falha **antes** de qualquer append. Se o append de `ObligationCreated`
falhar depois do `Proposed`, o ledger mantém um `Proposal` não citável; o erro é devolvido
e não há `Pending` fictício. O CAS pode ficar com objeto órfão, coletável pelo `gc`.

### D7 — Resultado novo versionado; históricos preservados

Requests, envelope externo kernel-protocol/v1 e catálogo de eventos não mudam.
O `value` antes livre recebe um formato novo, `propose-result/v1`, cuja versão
começa em1. Três vetores independentes de resultado são gerados/conferidos
pelo gerador já existente, sem alterar os34 vetores históricos de entrada.
Não existe decoder novo de entrada na TCB; fuzz usa kernel_message existente.

## 3. Implementação

- Novo módulo `crates/logos-kernel/src/propose.rs` (TCB):
  - `pub struct ProposalHandlers` sem estado; autor do Genesis autenticado.
  - `impl<S: Signer> Handlers<S> for ProposalHandlers` sobrescrevendo `propose`; `query` e
    `subscribe` ficam com o default (leitura real).
  - Carregar `Vocabulary` do CAS (`Cas::get` + `decode_with_limits`), montar snapshot,
    chamar `generate`, mapear `Reject` para `TransportError::Invalid` sem perder a causa
    no log interno.
  - Publicar `O` com `to_canon` + `Cas::put::<Obligation>`.
  - Usar `Transitions::new(ledger)`, `draft = propose(...)`, `pending = submit(...)`.
  - Retornar `Value::Array([1, Bytes(hO), Bytes(O)])`.
- `lib.rs`: expor `mod propose` e o tipo.
- `main.rs`: usar `ProposalHandlers` no lugar de
  `ReadOnlyHandlers`; `ReadOnlyHandlers` continua para testes de leitura.
- `tcb/manifest.toml`: listar `propose.rs`; rodar `cargo xtask tcb check`.
- `spec/kernel-protocol.md`: documentar o resultado versionado `[1, hO, O]`, a autoria D1 e que
  `CERTIFY`/`REFUTE`/`WITNESS`/`ASSUME` seguem indisponíveis.
- Sem nova dependência.

## 4. Testes

Novo `crates/logos-kernel/tests/propose.rs` (integração com ledger/socket reais):

1. **STATE 14**: vocabulário real, três premissas citáveis (assumptions de escopo vazio),
   `PROPOSE C7`; resposta `hO/O`; eventos `Proposed` + `ObligationCreated`; `O` com
   premissas ordenadas/deduplicadas; `Query::Status(h_Event_Pending)` = `Pending`; `Proposed` preserva
   ordem/duplicatas das citações.
2. **Caminho vazio**: `goal = True` sem citações gera `O` e `Pending`.
3. **P1 (50 casos)**, cada um via frame real, erro `Invalid` e cabeça do ledger
   inalterada (nenhum evento novo):
   - 10 citações não verificadas (`Pending`/`Proposal`/`Refuted`/`Unknown`);
   - 10 tentativas de mandar obrigação pronta (campo extra, tag trocada, bytes de `O` no
     lugar do goal), recusadas já pelo decoder estrito da 5.2;
   - 10 tentativas de contrabandear fato/política (campo extra `fact`/`policy`, tag de
     evento no payload), recusadas pelo decoder;
   - 10 citações de evento de época anterior;
   - 10 citações de claim retratado (assumption de escopo vazio + `Retracted`).
4. **Regras específicas**: citação inexistente; vocabulário ausente do CAS; vocabulário
   incompatível; goal mal tipado; premissa mal tipada; `speculative_on` não vazio.
5. **Duplicatas**: `cites = [A, A, B]` no `Proposed`; `O` usa `[A, B]`; a resposta e o
   status continuam `Pending`.
6. **Autoria**: `Proposed.author == Genesis.kernel_key == signer.key_id()`; o evento é assinado pelo kernel.
7. **Sem credencial**: nenhuma resposta de `PROPOSE` cria `Verified`, `Fact`, `Policy` ou
   token; `Query::Status` de `h_Event` é `Pending`, enquanto `hO` não é referência de evento.

Para os fixtures de `Refuted`/`Unknown`/retratação, reaproveitar os construtores de
`tests/support/` e os vetores de `spec/vectors/events/`; não inventar `CheckRecord` sem
`outcome` coerente.

Os50 ataques de P1 retornam Invalid. Erros de snapshot/contexto persistido,
incluindo scope incompatível e premissa mal tipada, conservam Storage no wire
e causa tipada internamente; não são transformados em ausência/resultado lógico.

## 5. Verificação e evidência

- `cargo test -p logos-kernel --test propose`
- `cargo test --workspace && cargo clippy --workspace --all-targets -- -D warnings`
- `ci/verify.sh` (completo; anexar `target/verify-5.3.*`)
- `cargo xtask tcb check` e `tcb report` (novo arquivo na TCB)
- `ci/fuzz.sh 60 kernel_message` (o decoder não mudou, mas o alvo continua verde)
- Revisão de mutação manual: remover a checagem de categoria/época/retratação e conferir
  que o P1 falha; restaurar.

## 6. Riscos

- **Autor D1**: se a revisão exigir identidade criptográfica do agente já na 5.3, o
  protocolo v1 precisa de campo/assinatura e a etapa cresce; a alternativa é adiar a
  autoria para 5.11 e manter o UID como identidade de transporte.
- **Escopo de assumptions D4**: a 5.7 pode exigir um campo de escopo na proposta; hoje a
  recusa é segura, mas limita o uso de assumptions na 5.3.
- **Falha entre `Proposed` e `ObligationCreated`**: não há transação única; o `Proposal`
  órfão é não citável, mas o teste deve documentar o comportamento.
- **Resposta grande**: AIR limita a fórmula, não a soma das premissas de O. O handler
  confere explicitamente O≤1MiB após serializar e antes de publicar; testa exatamente
  1MiB e1MiB+1. Esse teto de saída não é limite de RSS/wall da resolução/geração.

## 7. Evidências

- Oito testes de integração: todos passaram, incluindo50 ataques distintos P1,
  STATE14/época9, falhas entre appends, reabertura, escopo transitivo, limite
  exato de1MiB e três vetores independentes;37.39s. Nenhum handler de prova implementado.
- Primeira compilação da fixture apontou Signer não Sync ao mover o ledger para
  thread. Corrigido movendo somente o cliente, sem adicionar Sync ao signer
  de falhas nem alterar o runtime. Clippy apontou vec desnecessário em fixture;
  corrigido com array. Naquela primeira rodada, verificação completa,
  mutações e fuzz ainda estavam pendentes.
- As14 mutações da receita `etapa-5.3-mutacoes.json` compilaram e foram
  detectadas por falhas de teste; zero sobreviventes/erros de build contados
  como êxito. Incluem categoria/época/retratação/escopo, especulação, limite,
  bytes CAS, hash/versão/autor da resposta/evento, omissão de Pending,
  descarte de citações, campos extras e dispatch do executável. SHA-256 de
  todos os fontes restaurados conferidos; `target/propose-mutations/results.json`.
- Fuzz oficial `ci/fuzz.sh 60 kernel_message`, nightly-2026-09-28:560503
  execuções/61s, exit0, pico555MiB. Tempo total116.34s inclui rebuild;
  fontes inalterados. Log novo em `fuzz-logs/kernel_message-5.3.log`;
  log histórico5.2 restaurado byte a byte, sem alterar seu resumo/hash.
  Metadados em `target/fuzz-5.3-final.json`.
- TCB repetida:13573+747130=760703 (+72 próprios), kernel1427;
  build-only64508. Nenhum pacote externo novo. Relatórios em
  `target/tcb-5.3.{json,md}`.
- Full `ci/verify.sh --selftest`: VERIFY_OK/exit0 em1568.40s, logs e hashes
  em `target/verify-5.3-final.{log,json,exit,seconds}`. Rust/Clippy/smokes/
  manifesto/independência/deny/audit/vetores/autoteste passaram.252 Python
  em101.41s e25 TCB em1.53s; novos testes PROPOSE repetidos na rodada completa
  passaram em31.41s. Avisos conhecidos deny de licença/syn, sem falha.
- Produtor SAT300/300,300 pares SAT/300 consistências, sete controles e907
  rechecks; corpus LRAT200/200. Crash ledger10k/24 SIGKILLs/9997 ACKs/66.97s
  sem perda reconhecida; rebuild100k/cinco tabelas exatas;1M/20000 consultas
  p99=36617ns, cold rebuild274590ms fora da latência.
- `etapa-5.3-evidencias.json` liga resultados aos hashes restaurados, aos34
  vetores históricos de request inalterados e aos três vetores novos. Diff
  de fontes/documentos sem espaços finais; log smoke/science/ZIPs anteriores
  preservados fora do commit. Critérios locais5.3 demonstrados; nenhuma
  certificação, ingestão de fatos/políticas, especulação ou ação implementada.
