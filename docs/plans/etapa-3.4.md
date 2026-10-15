# Plano — Etapa 3.4: épocas genéricas

Estado: demonstrada localmente. Mantenedor adotou explicitamente a
divisão A do [ADR 0023](../adr/0023-contrato-de-epocas-e-fatos.md).
ROADMAP 3.4/6.1 atualizado antes do código; bloqueio inicial resolvido.

## 1. Interfaces de entrada

- 3.1: `EventBody::EpochAdvanced { new_epoch: u64, reason: Text }`, envelope
  event/v1 assinado, catálogo fechado de 13 variantes (`spec/events.md`).
- 3.2: append/replay verificam crescimento estrito de epoch; lock de writer,
  assinatura e persistência SQLite já implementados (`spec/ledger.md`).
- 3.3: `LedgerIndex::is_citable`, `status`, `snapshot`; cache reconstruível,
  `LedgerView` real e CAS/escopo conferidos (`spec/ledger-index.md`).
- 1.5: `LedgerEntry.epoch` representa aplicabilidade resolvida pelo snapshot,
  `generate` exige época corrente e kind citável (`spec/obligation.md`).
- Ausentes no início: `Fact`, `FactRegistered`, `EpochRange` e ingestão de
  fatos. EpochRange genérico passa a ser saída da 3.4; os demais ficam na6.1.

## 2. Interfaces de saída

API genérica em logos-core, sem formato serializado:

```text
EpochRange::new(first: u64, last: u64) -> Result<EpochRange, EpochError>
EpochRange::contains(epoch: u64) -> bool
```

Getters first/last completam a API. Intervalo inclusivo não autentica entradas
nem muda EntryKind ou hash lógico de O. EpochAdvanced recebe policy_root
Option<bytes32> e vocab Option<Hash<Vocabulary>>; epoch-body/v2 somente com
algum opcional presente, v1 histórico quando ambos ausentes (ADR0023).

## 3. Tarefas em ordem e arquivos

1. **Concluído: diagnóstico documental.** Localizar dependências reais;
   `docs/adr/0023-contrato-de-epocas-e-fatos.md`, este plano, `STATUS.md`,
   `docs/plans/fase-3.md` e nota em `ROADMAP.md`. Sem código da TCB.
2. **Concluído: contrato.** Mantenedor adotou A; ADR/ROADMAP atualizados.
   O teste de fato real fica explicitamente na6.1, sem mock substituto.
3. Implementar EpochRange em `logos-core/src/epoch.rs`, export/manifesto e
   testes de limites/enumeração. Estender codec EpochAdvanced e consumidores,
   vetores independentes preservando os18 anteriores e fuzz core/event.
4. Implementar intervalo com limites, testes do ledger/
   snapshot/gerador reais e preservação de estados não citáveis. Reutilizar
   testes da 3.3; não acrescentar Fact/FactRegistered nem política/ações.
5. Demonstrar testes sob mutações compiladas, fuzz no
   script oficial, full verify/selftest e medição TCB; atualizar docs/STATUS
   e commit. Não marcar critérios que ainda dependam da integração da 6.1.

## 4. Critérios adotados → testes

| Critério de ROADMAP 3.4 | Teste necessário | Estado |
| --- | --- | --- |
| Intervalo [3,5], limites inclusivos, inválidos e u64 | logos-core/tests/epochs.rs, enumeração independente de81 pares | Passou localmente |
| Premissas antigas recusadas e estados preservados | advanced_epoch_rejects_old_verified_and_assumption_in_real_generation; advancing_epoch_never_promotes_unknown_or_invalidated_records | Passou localmente |
| Opcionais estritos, vetores/assinatura/histórico | 22 vetores + extended_epoch_body_has_one_strict_encoding_and_preserves_legacy_bytes + epoch_commitments_are_bound_to_signature_without_conferring_authority; replay de ledger | Passou localmente |

`epochs_do_not_truncate_u64_or_promote_old_premises`, já na 3.3, demonstra
igualdade de época e preservação de u64, mas não janela de fatos. Um teste
de pertinência de intervalo isolado também não fecha o critério integrado.
A divisão adotada preserva esse teste integrado como obrigação explícita da6.1.

## 5. Riscos e decisões em aberto

- Atribuir `EntryKind::Fact` numa view falsa contornaria o contrato sem
  demonstrar autenticidade/validade. Não usar esse expediente.
- A época de emissão assinada não é uma janela de aplicabilidade; não
  sobrescrevê-la para promover premissas antigas.
- `policy_root` é compromisso opaco nesta etapa. Publicação de política,
  confiança administrativa e autorização de efeitos pertencem a fases futuras.
- Campos serializados novos exigem bump/vetores/spec e estratégia explícita
  de leitura histórica; evento antigo continua com seus bytes/hash originais.
- Snapshot real mantém lock e replay integral; custo frio/CI/ARM pendentes
  da 3.3 não desaparecem com este planejamento.

## 6. Estimativa revisada

P para mecanismo genérico após resolver o contrato; análise de dependências
não autoriza implementar 6.1/5.3 dentro dessa estimativa. Nenhuma estimativa
de entrega do critério integrado enquanto suas entradas estiverem ausentes.

## 7. Evidência da implementação genérica

- Seis regressões novas: dois testes de intervalo (incluindo enumeração
  independente dos81 pares pequenos), dois de codec/assinatura e dois de
  Ledger/CAS/snapshot/generate reais. Os49 testes direcionados passaram;
  Clippy workspace/all-targets passou. Referências a Verified de fixture
  demonstram elegibilidade/geração, não que um kernel já checou prova lógica.
- Vetores independentes de canon/hashlib/Ed25519:22 eventos. Comparação com
  `e46ebed` confirmou que todos os18 anteriores permanecem idênticos,
  incluindo bytes, assinatura e hash. Replay armazena os22 byte a byte.
- Oito mutantes compilados capturados por falhas de testes; fontes
  restauradas. Logs e comandos em target/epoch-mutations/results.json.
  Reprodução: aplicar cada alteração isoladamente, executar
  `cargo test -p PACOTE --test SUITE FILTRO` e restaurar antes da próxima.

| Mutação | Arquivo | Pacote/suite/filtro que falhou |
| --- | --- | --- |
| Primeiro limite exclusivo | core/src/epoch.rs | logos-core / epochs / inclusive_epoch |
| Último limite exclusivo | core/src/epoch.rs | logos-core / epochs / inclusive_epoch |
| Aceitar intervalo invertido | core/src/epoch.rs | logos-core / epochs / inclusive_epoch |
| Omitir policy_root ao codificar | ledger/src/event_body_codec.rs | logos-ledger / events / test_all_canonical |
| Omitir vocab ao codificar | ledger/src/event_body_codec.rs | logos-ledger / events / test_all_canonical |
| Aceitar v2 com ambos ausentes | ledger/src/event_body_codec.rs | logos-ledger / events / extended_epoch_body |
| Ignorar época de emissão na consulta | ledger/src/index.rs | logos-ledger / index / advanced_epoch_rejects |
| Ignorar época no gerador | obligation/src/generate.rs | logos-ledger / index / advanced_epoch_rejects |

- TCB Rust9056 próprios +105199 runtime=114255, build-only64508;
  core344, ledger2980. +82 próprios; nenhum pacote externo novo. SQLite C
  permanece separado do contador Rust (176466 LOC, ADR0021).
- Primeiro Ruff avulso foi invocado fora de py/ e usou configuração errada
  (noqa E402 preexistente); execução na pasta correta identificou uma linha
  nova longa, corrigida por Ruff format. Ruff final passou. Nenhum teste
  direcionado ou Clippy falhou.
- Fuzz oficial com nightly-2026-09-28 e sanitizers ativos: core_decode172375
  execuções/61s e event_decode107491/61s, exit0 sem crash. Seeds dos22 eventos
  são carregadas pelo runner existente. Fora do sandbox por ptrace já conhecido;
  max_len4096, sem alegação de ≥1h remota. Logs desta sessão em
  target/core_decode-3.4.log e target/event_decode-3.4.log; anteriores restaurados.
- Full `ci/verify.sh --selftest`: VERIFY_OK, exit0; Rust/smokes, guards,
  manifesto/independência/supply-chain/vetores,227 Python85,88s +25 TCB1,80s;
  autoteste sem avisos e bloqueios pelos diagnósticos corretos. Só avisos deny
  preexistentes. Crash10k/24 SIGKILLs/9999 ACKs sem perda; rebuild100k/tabelas5
  exatas, 1M/20000 consultas p99=17756ns (build250150ms fora da latência).
- CI/ARM/fuzz longo não observados. Nenhuma integração de fatos/políticas/
  ações/kernel nova. Alterações preexistentes preservadas; sem push.
