# Handoff da Etapa5.2 — 2026-10-18

> Fechado em2026-10-19: a conferência corrigiu o fechamento anunciado
> prematuramente em18/10. Seis lotes completos anteriores mais dois de
> retomada, excluindo o sétimo interrompido:14400s pedidos/96634872 execuções
> sem crash, pico1331MiB. Retomada oficial3600s com exit0 e hashes inalterados;
> nova verificação completa VERIFY_OK/exit0 em1760.07s. Evidências em
> `etapa-5.2-fuzz.json` e no §7 de `etapa-5.2.md`; o texto abaixo registra
> o estado original do handoff.

Pedido do mantenedor: informar o restante para outro agente encerrar a etapa.
Base local451e1fffbc9e50cc0ba0d6dac108e983500b5622; alterações da5.2 ainda
sem commit. Nenhum push nesta sessão. Não reimplementar o que já está pronto.

## Pronto e verificado

- ADR0035, plano, spec/kernel-protocol.md e14 vetores positivos/20 negativos.
- logos-kernel: codec canônico/framing, socket Linux/SO_PEERCRED, limites de
  frame/taxa/bytes/prazos/conexões, dispatch e staging com snapshot conferido
  antes do CAS. Query/Subscribe reais; handlers semânticos futuros indisponíveis.
- Ledger::events_from: paginação limitada com assinaturas/links/seq/head.
- Cinco testes de wire, quatro de staging, dez de transporte/binário e teste
  interno da paginação; upload3MiB, arquivo alterado pelo agente, symlink/
  hardlink/FIFO, UID errado, deadlines absolutos e assinatura viva concorrente.
- Treze mutantes compilados detectados. Receita: etapa-5.2-mutacoes.json;
  resultados: target/kernel-mutations/results.json, fontes restauradas.
- `ci/verify.sh --selftest`: **VERIFY_OK, exit0,1316.73s**, executado fora do
  sandbox porque sockets/bwrap são bloqueados dentro dele. Logs:
  target/verify-5.2-final.{log,exit,seconds}. Nenhum teste ficou ignorado para
  contornar esse bloqueio. TCB:13501+747130=760631; build-only64508.
- Alvo kernel_message integrado ao script oficial/matriz noturna14400s;
  manifesto TCB/allowlist/lockfiles/CI/AGENTS atualizados.

## Falta para fechar

1. Diagnosticar o OOM do fuzz. A execução oficial terminou após7778.07s
   (cerca de2h09m38s), RSS2049MiB excedendo limite2048MiB, exit1.
   Último progresso43473055 execuções; heap vivo34785659bytes,8009chunks;
   isso ainda não determina se a causa é decoder ou acúmulo no fuzzer/ASAN.
   Não alegar crash de lógica nem inocentar o decoder sem reproduzir/analisar.
2. Corrigir a causa, acrescentando regressão significativa se for código.
   Reproduzir primeiro o artefato; um input salvo durante OOM acumulado pode
   não falhar sozinho. Nesse caso investigar crescimento ao longo da execução.
3. Completar **14400s reais com exit0** pelo mesmo ci/fuzz.sh da CI.
4. Se houver alteração de código/script, repetir ci/verify.sh --selftest.
   Atualizar STATUS/plano/ROADMAP com resultados observados; só marcar fuzz
   concluído após evidência. Commit local `Etapa 5.2: ...`, sem push.

## Arquivos e comandos

Logs do fuzz encerrado: target/fuzz-5.2-final.log e .json; o segundo contém
duração, exit1 e SHA-256 de wire.rs antes/depois (fonte inalterada).
Log oficial: fuzz-logs/kernel_message.log. Artefato salvo:
`fuzz/artifacts/kernel_message/oom-4675fee509ad98e3a2fb56e43d35997aaff6e836`.

Reprodução, com diretório de trabalho `fuzz/`:

```sh
rtk proxy env RUSTUP_TOOLCHAIN=nightly-2026-09-28 CARGO_NET_OFFLINE=true cargo fuzz run kernel_message artifacts/kernel_message/oom-4675fee509ad98e3a2fb56e43d35997aaff6e836 -- -runs=1
```

Execução longa, com diretório de trabalho na raiz do projeto:

```sh
rtk proxy env FUZZ_TOOLCHAIN=nightly-2026-09-28 CARGO_NET_OFFLINE=true ci/fuzz.sh 14400 kernel_message
rtk proxy ci/verify.sh --selftest
```

Não usar RUSTC_BOOTSTRAP. Não rodar dois kernel_message simultaneamente:
o script escreve no mesmo fuzz-logs/kernel_message.log. A primeira tentativa
da sessão foi interrompida manualmente por colisão de log com um smoke;
preservada em target/fuzz-5.2-shared-tee.log e target/fuzz-5.2-captured/.
Também não fecha o critério. A execução posterior exclusiva é a que teve OOM.

Preservar ZIPs preexistentes logos-code.zip/logos-docs.zip e artefatos locais.
TCB/runtime ainda não implementa PROPOSE5.3, CERTIFY5.4, ASSUME5.7 nem MCP.
Não avançar de etapa durante o fechamento da5.2.
