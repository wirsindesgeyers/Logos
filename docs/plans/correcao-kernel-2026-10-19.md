# Correções da revisão do kernel — 2026-10-19

## Objetivo e dependências

Corrigir integralmente os três P2 reproduzidos na revisão de19/out, sobre
c647418. Dependências3.3/5.2/5.3 presentes; escopo conforme ADR0040.
Fora do escopo: CERTIFY5.4, memória, handlers novos, formatos e dependências.

## Plano

1. Registrar ADR0040 e contratos da projeção viva, prazo/cursor e devolução de
   diretórios. Preservar API de reconstrução/cache e formatos existentes.
2. Adicionar regressões adversárias e confirmar falha nos caminhos antigos.
3. Implementar prazo compartilhado de Subscribe, devolução ancorada de
   diretórios e projeção incremental privada, com falhas explícitas.
4. Comparar projeção viva com reconstrução; testar integridade, Poisoned,
   múltiplos snapshots, reabertura, colisões e I/O. Compilar/matar mutantes.
5. Repetir reproduções, medir caminho real Status/PROPOSE, executar Clippy,
   verificação completa/selftest e medição da TCB. Não alterar decoder.
6. Atualizar STATUS/relatório/evidências no mesmo commit. Sem push.

## Critérios e evidências

- [x] Subscribe interrompe a página no deadline e limita escrita bloqueada;
  cursor correto quando o encerramento ocorre entre frames.
- [x] Diretórios vazios/com filhos não deixam entradas privadas; colisão
  preserva upload novo e devolve o rejeitado, sem seguir links.
- [x] Status repetido não reconstrói índice/copia cache por requisição;
  PROPOSE usa sufixos autenticados após seus appends.
- [x] Projeção viva equivale ao replay completo e não aceita corrupção de
  cache, assinatura, prev, época, sequência ou cabeça; erro não vaza parcial.
- [x] Regressões falham antes das correções e/ou em mutações correspondentes.
- [x] `ci/verify.sh --selftest`: VERIFY_OK, exit0; manifesto/TCB e formatos
  conferidos; nenhuma verificação remota presumida.

## Riscos

Um frame parcial no timeout exige reconexão. Handler/replay não é preemptível;
não confundir deadline de escrita com limite de CPU. Devolução de diretório
pode falhar por I/O ou colisões provocadas: retornar causa e caminho capturado,
sem destruir outro upload. Projeção incremental precisa conferir cabeça exata,
preservar a transação e bloquear consulta em handle Poisoned. Cache em disco
permanece uma cópia não confiável, possivelmente antiga até rebuild explícito.

## Registro de desenvolvimento e verificação

- Regressões antes da correção: diretório rejeitado retornava EISDIR e a página
  atrasada ainda enviava evento após expirar. Ambos falharam por asserção.
- Tentativa inicial da fixture de frame bloqueado excedeu o teto real de texto
  tainted64KiB; corrigida. O teste com prazo100ms também confundia preparação
  da página com espera de socket. A fixture final prepara a página antes e usa
  assinatura1s/frame5s, buffer4096bytes e guarda de3s; exige frame de evento.
- A primeira execução ampla no sandbox teve EPERM em sockets; repetição nativa
  de ledger/kernel terminou exit0. Clippy apontou módulo de testes antes de
  itens; módulo movido para o fim, sem relaxar lint.
- Ajustes de fixtures: canal movido para fora do scope após E0373; evento de
  época inválida usa corpo que permite assinatura estrutural válida, pois
  Event::sign já recusa EpochAdvanced com header/body divergentes. Isso isola
  o guard de continuidade da cadeia, sem contar recusa do construtor como teste.
- A primeira bateria de mutações revelou sobrevivência do no-op de metadata:
  o teste só conferia depois de reconstruir. Acrescentado sufixo válido antes
  da checagem; a bateria seguinte detectou16/16 mutantes compilados.
- Dois compile-fail novos demonstram E0502/E0505 para append/Drop com view
  ativa; controle positivo compila com duas views e append depois de Drop.
- A primeira verificação completa foi interrompida deliberadamente em317,35s
  (exit-15, sem VERIFY_OK) para trocar nomes previsíveis de devolução por nonce
  do SO. Log/relatório preservados em target/verify-kernel-review-interrupted.*.
  As tentativas anteriores de mutações ficam em target/kernel-review-mutations/.
- A versão final testa colisões forçadas com injeção privada, oito tentativas
  limitadas, falha de entropia e falha de devolução. A produção usa128bits de
  `/dev/urandom`; não permite ao agente escolher o nome de retorno.
- A conferência do replay compartilhado encontrou filtro inicial `seq>=0`
  escondendo linhas negativas inseridas com CHECK desabilitado por uma fonte
  adulterada. Regressão nova falhou antes da correção; replay inicial agora
  varre desde i64::MIN e rejeita a linha, enquanto só o sufixo exclui prefixo
  autenticado. Segunda verificação interrompida em972,66s (exit-15, sem
  VERIFY_OK) para corrigir; preservada em target/verify-kernel-review-negative-row.*.

O resultado final abaixo foi conferido após término real, com hashes e código de saída.

Versão com replay inicial corrigido:17/17 mutações compiladas detectadas,
arquivos restaurados por hash. Treze regressões runtime novas, dois
compile-fail e um controle positivo novo. Reproduções finais em
[JSON](correcao-kernel-2026-10-19-probes.json), mutações em
[JSON](correcao-kernel-2026-10-19-mutacoes.json); full verify final concluído conforme recibo abaixo.

## Fechamento local

`ci/verify.sh --selftest`: **VERIFY_OK, exit0,2006,93s**. As430 entradas de
runtime/spec/CI/TCB conservaram SHA-256 antes/depois e na conferência final;
[manifesto de entradas](correcao-kernel-2026-10-19-inputs.json) e
[evidências](correcao-kernel-2026-10-19-evidencias.json).
Rust:100 resumos de suítes,520 testes/0 falhas/0 ignorados; Python252/107,52s
e TCB25/1,67s. Smokes/CNF/auditor independente/SAT, formatos/vetores,
manifesto/lints/deny/audit e autoteste passaram. Crash10k/24 SIGKILLs/9999 ACKs
sem perda reconhecida; rebuild100k/cinco tabelas exatas;1M/20000 consultas,
p99=38874ns e cold285496ms fora da latência. TCB14116+747130=761246 Rust,
kernel1594/ledger3791, build-only64508; inclui testes em src/, sem pacote novo.

O autoteste emitiu `tar: .: file changed as we read it` ao copiar a raiz
para fixture temporária. Concluiu todos os controles e recusas com os
diagnósticos exigidos; nenhum dos430 arquivos acompanhados mudou e exit0/
VERIFY_OK foram conferidos. O aviso e os demais avisos do log constam no
recibo; não se afirma execução sem avisos.

Não houve novo fuzz: wire/event decoders e vetores permaneceram inalterados.
Os testes de4h da5.2 são evidência histórica, não uma nova corrida desta correção.
CI/ARM/noturno ainda não observados; nenhum push autorizado/realizado.
Arquivos científicos, ZIPs e smoke.log preexistentes ficam fora do commit.
Próximo passo principal: planejar a5.4, mantendo a API viva nas consultas
frequentes e a reconstrução explícita separada conforme ADR0040.
