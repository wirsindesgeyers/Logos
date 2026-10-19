# Plano da Etapa5.2 — servidor do protocolo

2026-10-19; base451e1ff. Concluída localmente; escopo ROADMAP5.2/ADR0035/ADR0036.

## 1. Entradas

Proposal1.6/spec e vetores, canon0.3, CAS0.5 e appender3.2 presentes.
EventProposal compartilha os campos candidatos sem ligar JSON à TCB.
rustix1.1.5 instalado/fixado; APIs seguras de credenciais e openat/renameat.
Nenhuma dependência ausente. Não implementar semântica das etapas5.3–5.10.

## 2. Saídas

Request/Message/ObjectRef, encode/decode estritos, framing limitado;
Server Linux autenticado por UID, ConnectionLimits e handlers por comando;
Staging::import para snapshots conferidos e CAS; Query/Subscribe sobre ledger.
Contrato kernel-protocol/v1 em spec/kernel-protocol.md e vetores independentes.

## 3. Tarefas em ordem

1. Plano/ADR/spec antes do runtime; definir schema, framing, quotas e erros.
2. Codec em kernel, manifesto/dependências locais; vetores reproduzidos por
   Python independente, rejeição de extras/tipos/versões/limites/UTF-8/AIR.
3. Staging com captura ancorada e snapshot privado; testes de hash/limites,
   symlink/FIFO/hardlink/traversal/mutação do original e erro sem publicação.
4. Socket, peer credentials, deadlines absolutos e quotas por conexão;
   handlers e leitura paginada de eventos no appender existente. Testes reais
   fragmentando/coalescendo frames, EOF parcial, UID errado, taxa/prazos,
   eventos e comandos indisponíveis sem append.
5. Fuzz oficial/kernel_message, CI14400s, mutações compiladas e full
   ci/verify.sh --selftest; relatório TCB, STATUS/roadmap e commit local.

## 4. Critérios → evidências

- Extras/tipo desconhecido → vetores negativos, suíte adversária de codec e
  socket; erro explícito, nenhum append/handler executado em entrada inválida.
- Decoder≥4h sem crash → ci/fuzz.sh 14400 kernel_message com nightly fixada
  localmente ou logs da matriz noturna; smoke curto não marca esse critério.
- Regras TCB → manifesto, mutações manuais compiladas e verificação completa.

## 5. Riscos/decisões

ADR0035: Linux/UID configurado; JSON permanece fora da TCB. Same-UID não é
isolamento; ambiente/concessão de diretórios é do operador/5.11. Captura do
staging pode consumir arquivo mesmo em erro; não repetir automaticamente.
Handlers de prova/fatos não existem ainda e devolvem erro, nunca sucesso falso.
ADR0036: o fuzz longo roda em lotes de1800s com processo novo por lote, porque
o runtime ASAN acumula RSS por execução mesmo em alvo trivial; o teto padrão
de2048MiB segue armado em cada lote e o corpus acumula entre eles.
Não alegar4h de fuzz ou CI sem evidência; preservar os ZIPs preexistentes.

## 6. Estimativa

M mantida: decoder novo na TCB, socket, staging adversário e testes reais.

## 7. Evidências

Parciais: cinco testes de codec (14 vetores positivos e20 negativos; limites
reais de AIR1MiB/texto64KiB/referências1024), quatro de staging e dez de
transporte passaram. Os testes exercitam upload3MiB pelo socket, todos os
handlers, consulta de status no índice, assinatura viva com append concorrente
e o executável real com Genesis cujo hash corresponde ao binário.

Treze mutantes compilados foram detectados por falha de asserção, conforme
`etapa-5.2-mutacoes.json`: aridade, tag desconhecida, hash anterior ao CAS,
hardlink, symlink, UID, taxa/frames/bytes e assinatura/predecessor/head/linha
ausente na paginação. Fontes restauradas e SHA-256 conferido; resultados em
`target/kernel-mutations/results.json`.

Tentativas iniciais: sockets bloqueados pelo sandbox com Operation not
permitted (1 passou/8 falharam); execução externa dos mesmos testes passou.
UV teve cache bloqueado; na execução externa Ruff passou e mypy apontou
inferência da lista heterogênea do gerador, corrigida com tipo explícito.
Clippy detectou condicionais/map_identity/drop_non_drop e detalhes das
fixtures, corrigidos antes da verificação final.

Primeiro fuzz longo foi interrompido manualmente: um smoke concorrente usou
o mesmo arquivo de tee. Essa execução não fecha o critério. O reinício
exclusivo de14400s terminou com OOM (RSS2049MiB acima do teto2048MiB) após
7778.07s/43473055 execuções (exit1); `target/fuzz-5.2-final.{log,json}`,
artefato `fuzz/artifacts/kernel_message/oom-4675fee5…`. O input salvo executa
sozinho em2ms sem falha, então não era defeito de lógica do decoder.

Diagnóstico do OOM (evidências em `target/smoke-asan-matrix.log`,
`target/km-loop-exp/` e `target/fuzz-5.2-batch-1800s.log`; decisão no
ADR0036): o alvo trivial `smoke` (só `str::from_utf8`) acumula RSS no mesmo
build — padrão:32→616MiB em8.4M execuções, `quarantine_size_mb=0`: plano em
35MiB, `=32`: platô de110MiB — e um laço de decode/encode sobre o corpus, sem
libFuzzer, estabiliza em392MiB (passos200–2000, 0.8M–8.1M tentativas). O
acúmulo é do alocador/quarentena do ASAN, não do decoder. Correção: `ci/fuzz.sh`
divide o tempo pedido em lotes de `FUZZ_BATCH_SECONDS` (padrão1800s) com
processo novo por lote, corpus cumulativo e teto de RSS armado em cada lote;
um lote mediu10364977 execuções com pico de1160MiB (exit0).

Conferência em19/10: `target/fuzz-5.2-batches.log` contém seis lotes completos
de1800s e o sétimo interrompido; `target/fuzz-5.2-batches.json` não existe.
A afirmação anterior de14400s/exit0 era prematura. Os seis lotes completos
somam10800s pedidos; o lote interrompido não conta. Retomada oficial de3600s
pelo mesmo `ci/fuzz.sh` concluída com exit0 em3604.78s, com corpus cumulativo,
sem mudar decoder/alvo;
BuildId do binário existente3b46da11997ced9c153248fdb34aea39a1437f41
confere com o log anterior. SHA-256 do decoder, alvo e executável conferidos
antes/depois, todos inalterados: `target/fuzz-5.2-resume.json`.
Logs originais preservados em `target/fuzz-5.2-batches.log` e
`target/fuzz-5.2-resume-prior.log`; nova corrida em
`target/fuzz-5.2-resume.log`. Log combinado versionado em
`fuzz-logs/kernel_message.log`, com somente espaços finais removidos;
o lote interrompido fica apenas no original, explicitamente excluído.

Oito lotes completos:14400s pedidos,15457s reportados pelo libFuzzer,
96634872 execuções sem crash; picos por lote1026/1026/1028/1069/1256/1331/
1092/1213MiB, todos abaixo do teto2048MiB. Resumo por lote e hashes no
artefato versionado `docs/plans/etapa-5.2-fuzz.json`. Não houve conclusão
nem exit0 do script original de14400s; seus seis lotes completos são
demonstrados pelo avanço do laço sob pipefail, e a retomada de3600s tem
exit0 registrado. Essa soma de lotes completos satisfaz o critério≥4h
do ROADMAP sob ADR0036, sem contar o lote parcial nem o OOM anterior.
Full ci/verify.sh --selftest repetido depois da mudança do script: VERIFY_OK,
exit0 em2396.56s sob carga concorrente (o de1316.73s segue em
`target/verify-5.2-final.*`; a medição nova está em
`target/verify-5.2-fuzzscript.{log,exit,seconds}`). TCB13501+747130=760631 LOC,
build-only64508, kernel1355; relatório local em target/tcb-5.2.{json,md}.

Nova verificação em19/10: `ci/verify.sh --selftest` terminou com VERIFY_OK,
exit0 em1760.07s (`target/verify-5.2-resume.{log,json,exit,seconds}`).
252 testes Python/122.34s e25 TCB/1.93s; Rust/Clippy/smokes/manifesto/
independência/deny/audit/vetores/autoteste passaram. Avisos conhecidos do
deny: licença não encontrada e duas versões de syn, sem falha. Produtor SAT:
300/300 provas,300/300 pares SAT,300 consistências, sete controles e907
rechecks; corpus LRAT200/200. Crash ledger:10k eventos/24 SIGKILLs/9998 ACKs,
83.54s, nenhum append reconhecido perdido. Rebuild100k/cinco tabelas exatas;
1M/20000 consultas p99=37332ns, cold rebuild340010ms fora da latência.
Hashes do decoder, alvo e executável de fuzz inalterados durante a verificação.
Medição TCB repetida e inalterada:13501+747130=760631; kernel1355 e
build-only64508 (`target/tcb-5.2-resume.{json,md}`).

`git diff --check` global apontou espaços finais em `fuzz-logs/smoke.log`
preexistente, preservado fora do commit; o log combinado desta etapa remove
somente espaços finais, conservando originais em target/. ZIPs, plano5.3 e
proposta científica/ADR0037 preexistentes também ficam fora deste commit.
