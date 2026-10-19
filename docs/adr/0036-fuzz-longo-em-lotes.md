# ADR 0036 — Fuzz longo em lotes com processo novo por lote

- Estado: aceito no fechamento da5.2
- Data: 2026-10-18
- Base: 451e1ff
- Relacionadas: 0035

## Contexto

A execução oficial de14400s do alvo kernel_message terminou com
`SUMMARY: libFuzzer: out-of-memory`: RSS2049MiB excedeu o teto padrão2048MiB
do libFuzzer depois de7778.07s e43473055 execuções (exit1). Alvo e decoder não
mudaram (SHA-256 de `crates/logos-kernel/src/wire.rs` igual antes/depois). O
input salvo no OOM (`fuzz/artifacts/kernel_message/oom-4675fee5…`) executa
sozinho em2ms e não falha: não é defeito de lógica do decoder.

Diagnóstico: o crescimento é do runtime ASAN do build de fuzz, não do decoder.
Com o alvo trivial `smoke` (só `str::from_utf8`), teto de8.4M execuções por
variante e medição `stat::peak_rss_mb` (`target/smoke-asan-matrix.log`):

| Variante | Pico de RSS |
| --- | --- |
| padrão | 616MiB |
| `quarantine_size_mb=0` | 35MiB (plano até4.2M execuções) |
| `quarantine_size_mb=32` | 110MiB (platô) |
| `use_cmp=0` / `malloc_context_size=1` / `detect_leaks=0` / `allocator_release_to_os_interval_ms=1000` | 615-616MiB (mesma curva do padrão) |

Encurtar redzones (`redzone=16:max_redzone=16`) reduz o platô para557MiB, mas
ao fim da corrida o LeakSanitizer passa a reportar ~6KB de alocações vivas do
próprio libFuzzer (exit1); não serve como correção.

O mesmo perfil aparece no alvo real (578MiB@1.0M execuções;1013MiB@8.4M;
1419MiB@33.5M;2049MiB@43.5M) e num laço de decode/encode sobre o corpus, sem
libFuzzer (experimento `target/km-loop-exp/`): o RSS fica em392MiB do passo200
ao passo2000 (813800→8138000 tentativas; 401444→402244KiB, +800KiB), ou seja,
o decoder não retém nada e o platô é do alocador/quarentena do ASAN (memória
liberada que não volta ao SO).

Logo, em processo único não dá para garantir o teto padrão2048MiB numa
corrida de4h — e reduzir a quarentena na corrida oficial enfraqueceria o ASAN
(a quarentena é o que atrasa a reutilização de memória liberada e expõe
use-after-free).

## Decisão

1. `ci/fuzz.sh` divide SEGUNDOS em lotes de `FUZZ_BATCH_SECONDS` (padrão1800s);
   cada lote é um processo novo de `cargo fuzz run <alvo> -- -max_total_time=<lote>`.
   O corpus é cumulativo (a cobertura acumula entre lotes) e o teto de RSS
   padrão fica armado em cada lote. A soma dos lotes é o tempo pedido; lote
   que falha aborta o script (exit≠0), então crash/OOM real continua visível.
2. O padrão1800s vem da medição: um lote de1800s faz ~10.4M execuções e o
   pico do alvo real fica em1160MiB (`target/fuzz-5.2-batch-1800s.log`) —
   folga de ~2× sobre2048MiB. Um processo único de4h chegaria a ~43M
   execuções e ultrapassa o teto (foi o OOM).
3. Não há mudança de código no decoder nem no alvo, logo não há regressão de
   código a acrescentar; a regressão operacional é o próprio laço de lotes,
   exercitado com `FUZZ_BATCH_SECONDS` curto (lotes múltiplos, cabeçalhos no
   log, exit0) antes da corrida oficial.
4. A CI noturna segue chamando `ci/fuzz.sh` com14400s para kernel_message; o
   timeout do job (330min) cobre os8 lotes mais a partida de cada um.

## Limites

- O tempo de fuzzing é a soma dos lotes; a partida de cada lote (releitura e
  calibração do corpus) fica fora da conta de14400s e aparece no log.
- O teto por lote detecta crescimento acima de ≈90bytes/execução (folga de
 888MiB em10.4M execuções); vazamento mais lento que isso não se distingue do
  próprio ruído do ASAN nem numa corrida única de4h (o OOM observado crescia
  ~45bytes/execução). A defesa contra isso continua sendo o corpus cumulativo
  entre lotes e a revisão do decoder, não o teto de RSS.

## Verificação do fechamento em2026-10-19

O fechamento anunciado anteriormente era prematuro: o log original tem seis
lotes completos e um interrompido, sem exit final. Preservados os originais,
dois lotes oficiais adicionais de1800s terminaram com exit0; decoder, alvo e
executável conservaram seus hashes. Somente os oito lotes completos contam:
14400s pedidos/15457s reportados,96634872 execuções sem crash, pico1331MiB.
Resumo e log versionados em `docs/plans/etapa-5.2-fuzz.json` e
`fuzz-logs/kernel_message.log`. Nova verificação completa com selftest:
VERIFY_OK/exit0 em1760.07s. Execução noturna/ARM continua pendente.
- `FUZZ_BATCH_SECONDS` é parâmetro operacional (não é formato nem spec):
  mudar o padrão exige refazer a medição, sem bump de versão.
