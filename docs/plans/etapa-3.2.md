# Plano — Etapa 3.2: appender persistente com hash chain

## 1. Entradas

Event/EventBody e event/v1 (3.1), Signer/PublicKey/Hash (0.4), CAS já
implementado (0.5). Dependências presentes. ROADMAP 3.2 e spec conceitual
§3–5. A checagem de referências no CAS pertence à 3.5.

## 2. Saídas

`Ledger::open(directory, &signer) -> Result<Ledger, LedgerError>` adquire lock
exclusivo sem espera e verifica todos os eventos antes de devolver o escritor.
`Ledger::append(body) -> Result<Event, LedgerError>` calcula seq/prev/epoch,
assina e confirma uma transação durável. `Ledger::head()` expõe somente
seq/epoch/hash já confirmados. Event é o SignedEvent existente, sem novo wire.

## 3. Tarefas e arquivos

1. ADR 0021 e spec/ledger.md: rusqlite bundled, schema ledger-sqlite/v1,
   locks, WAL/FULL, recuperação estrita e falhas ambíguas; Cargo/deny/manifesto.
2. src/ledger.rs, ledger_store.rs, ledger_error.rs: abertura, replay integral,
   appender único, head privado e conexão inacessível ao consumidor.
3. tests/ledger.rs: retomada real, assinatura/chave/seq/prev/corrupção/schema,
   falha de assinante, rollback e recusa de escritor concorrente.
4. examples/ledger_writer.rs, tools/check_ledger_crash.py e
   ci/check-ledger-crash.sh: subprocessos reais, SIGKILL em pontos aleatórios,
   10k appends confirmados e comparação independente de todos os bytes/hashes;
   recuperação não perde eventos reconhecidos, admite commit sem resposta.
5. Integrar verificação/CI; mutações manuais compiladas e capturadas, fuzz
   event_decode existente (nenhum codec próprio novo), full verify/selftest,
   medição TCB, STATUS/ROADMAP/README e commit sem push.

## 4. Critérios → evidência

- Crash durante 10k appends: ci/check-ledger-crash.sh, semente fixa reproduzível,
  SIGKILL enquanto writer está ativo; replay após cada morte e oráculo externo
  para seq/prev/assinaturas e prefixo dos eventos já reconhecidos.
- Segundo escritor em outro processo falha na abertura: teste subprocesso
  com handshake; também mesmo processo, alias de diretório e liberação após kill.
- Entregáveis de durabilidade/recuperação: testes de persistência/configuração,
  corrupção rejeitada e falhas SQL sem reutilizar head ambíguo; revisão de
  WAL/FULL e fsync explícito do diretório. SIGKILL não simula perda de energia.

## 5. Riscos e decisões

ADR 0021 fixa dependências transitivas e formato SQLite versionado. SQLite C e
rusqlite contêm unsafe internamente; o código próprio permanece forbid(unsafe).
Disco/SO devem honrar sync; filesystem local e diretório controlado pelo dono.
Lock é cooperativo, arquivo estável nunca removido. Não detecta remoção de
sufixo inteiro ou substituição por outro histórico válido sem âncora externa
(13.2). Mesma chave por ledger, sem rotação/HSM; replay linear na abertura.
Sequenciamento de epoch somente pelo evento EpochAdvanced; citabilidade/
política/validade temporal ficam na 3.4 e posteriores. Índices ficam na 3.3.

## 6. Tamanho

M, conforme roteiro. Sem mudança de ordem ou escopo.

## 7. Evidência local

- 48 testes no crate, 16 novos: 14 integrações (uma é entrada de subprocesso)
  e duas unidades. Bytes/head/reabertura, 18 vetores independentes de eventos,
  layout versionado, limites seq, configuração efetiva da conexão, rollback,
  handle Poisoned, Signer defeituoso, corrupção/schema e lock entre processos.
- `ci/check-ledger-crash.sh`: `LEDGER_CRASH_OK events=10000 sigkill=24
  seed=320026`. Oráculo Python canon/hashlib/Ed25519 confere integralmente
  após cada morte; replay Rust confere cada reabertura e os 10k finais.
  Smoke62,57s/9999 ACKs; full verify64,50s/9996 ACKs, 10000 linhas válidas
  em ambos. Diferença são commits sem resposta, preservados; zero ACK perdido.
  Segundo processo devolveu WRITER_BUSY/23 com primeiro fora de transação.
  SIGKILL não simula perda física de energia.
- Nove mutantes manuais compilados capturados por asserções; baseline passou,
  fontes restauradas, nenhum erro de compilação contado como mutante morto.
  Logs/resultado locais: target/ledger-mutations/. Para reproduzir, aplicar
  cada substituição isoladamente, executar `cargo test -p logos-ledger FILTRO`
  e restaurar a fonte; não executar verificação/fuzz contra fonte mutante.

| Mutação | Arquivo | Filtro que falhou |
| --- | --- | --- |
| Remover Event::verify em advance | src/ledger.rs | recovery_rejects_holes |
| Omitir comparação de prev | src/ledger.rs | recovery_rejects_holes |
| Omitir comparação seq vs next_seq | src/ledger.rs | recovery_rejects_holes |
| Omitir comparação seq da coluna vs evento | src/ledger.rs | recovery_rejects_holes |
| Ignorar continuidade de epoch | src/ledger.rs | recovery_rejects_holes |
| Trocar último commit por rollback (append) | src/ledger.rs | persists_exact_signed_bytes |
| Substituir try_lock por Ok | src/ledger.rs | lock_is_held_between_transactions |
| Exigir versão 2 ao reabrir v1 | src/ledger_store.rs | every_catalog_vector |
| FULL→NORMAL e aceitar synchronous=1 | src/ledger_store.rs | actual_writer_uses_wal_full |

- Full `ci/verify.sh --selftest`: VERIFY_OK, exit0, 227 Python (113,64s)
  +25 TCB (1,84s), todos smokes/vetores/guards/supply-chain. Avisos deny
  preexistentes apenas. Tar avisou corpus alterado por fuzz concorrente;
  primeiro autoteste isolado ainda avisou raiz alterada por comando sandbox
  concorrente. Autoteste final sem comandos paralelos: exit0, sem avisos,
  todos bloqueios pelo diagnóstico correto.
- Fuzz oficial event_decode/nightly-2026-09-28: final139749/61s, exit0,
  sem crash. Inicial sandbox132337/61s terminou exit1 porque LeakSanitizer
  não pôde usar ptrace ao encerrar; repetido fora do sandbox, sem desativar
  sanitizers. max_len4096; não prova fuzz de entradas16MiB nem ≥1h remota.
  Codec de eventos existente; nenhum parser próprio novo nesta etapa.
- TCB Rust: 8235 próprios +105199 runtime=113434; só build64508, ledger2220.
  +343 próprios e +26912 runtime Rust. SQLite3.53.2 bundled conta176466 LOC
  C em sqlite3.c por tokei, separadamente do contador Rust. Ambos são cotas
  de fonte; sem alegação de prova formal ou TCB total limitada à soma Rust.
- Fixture JSON inicial incorreta e lints de imports/chunks_exact corrigidos
  antes da verificação final. Logs de fuzz preexistentes preservados;
  alterações preexistentes de memória/fuzz-src/mutants.out fora do commit.
- CI/ARM/fuzz longo ainda não observados. Sem índices, autorização lógica,
  integração CAS da 3.5 ou âncora externa contra sufixo truncado.
