# Etapa5.5 — Não-vacuidade/WITNESS

Estado: concluída localmente; CI remota pendente; base12826f4; ADR0042.

## 1. Entradas

CertificationHandlers/registry/replay selado5.4 e ModelChecker2.3 presentes.
Consistência event/v1, CAS, índices e auditor independente existentes.

## 2. Saídas

Handler WITNESS; ConsistencyRecorded/v3; consulta efetiva de consistência
por referência Verified e elegibilidade de premissa para uso lógico/ação.
Não emite token nem implementa gateway. ModelChecker continua autoridade.

## 3. Tarefas e testes

1. Spec/codec/CAS/auditor: corpo v3/tag14, vetores e negativos; fuzz event_decode.
2. Kernel witness.rs, manifesto e handler: antes/depois, reabertura, contexto,
   limites/rejeição, prazo lógico inclusivo, ausência, revogação e instâncias.
3. CERTIFY incorpora testemunha rechecada; índice não promove eventos anexos.
4. Explosão Γ={P,¬P}: prova real aceita, modelos candidatos recusados,
   Verified Unknown e consulta de ação negativa; uso lógico continua possível.
5. Mutações com controles positivos, fuzz oficial e full verify/selftest;
   medir TCB, atualizar STATUS/ROADMAP, commit e push autorizado.

## 4. Critério → demonstração

Integrações pelo socket real para WITNESS/CERTIFY e fixture autenticada
para Γ inconsistente; consulta de elegibilidade real do kernel recusa ação.
Gateway8.2 permanece futuro, com contrato normativo explícito.

## 5. Decisões e riscos

ADR0042 fixa evento imutável e prazo lógico1024 eventos, sem relógio.
Nenhuma dependência ausente é simulada como pronta. Assumptions de teste
usam o catálogo existente; não implementam handler5.7. Auditor sem dependência
kernel/ledger. Não confundir modelo das premissas com prova do objetivo.

## 6. Tamanho

M por exigir decisão de formato, replay e auditor; escopo funcional só5.5.

## Verificação

Trajetória das verificações (fechamento final abaixo):17 integrações passaram na
suíte (443.63s), mais os dois novos controles de contexto/auditor executados
separadamente. Modelos de P e ¬P cobrem as duas interpretações possíveis do
vocabulário proposicional da fixture; a prova real de False foi aceita por LRAT.

Rodada inicial: nove mutações compiladas detectadas com controle positivo e
fontes restaurados por SHA-256. A rodada final tem dez; runner/recibo anexos.
Os24 vetores de evento históricos permanecem idênticos; três novos eventos
v3 e três resultados witness-result/v1 independentes adicionados.

Falhas e ajustes preservados:
- Fixture inicial de prova só conhecia par unitário oposto; substituída por
  gerador de refutação por propagação unitária (não é checker/produtor runtime).
- Primeira integração da janela excedeu30s de leitura do cliente em debug;
  timeout somente da fixture aumentado para180s, contrato do servidor intacto.
- Um filtro abreviado com --exact executou zero testes; não contado, repetido
  com nome completo e teste passou.
- Runner de mutações parou na última âncora após rustfmt; oito casos estavam
  detectados e fontes restaurados. Âncora corrigida, último caso detectado;
  recibo daquela rodada reuniu nove casos com controles.
- Primeira full verify exit101/41.23s: fixture do auditor ainda esperava24
  vetores. Corrigida para27 e a sequência final da fixture do ledger para26;
  ambos testes direcionados passaram. Log/recibo em target/verify-5.5-failed-1.*.

TCB final medida:15131 próprios +747130 de dependências =762261 Rust de correção;
kernel2547/ledger3853, build-only64508. +343 próprios, nenhum pacote externo
novo. Inclui testes em src/; componentes C/nativos conservam limites anteriores.

Limites: janela lógica1024 eventos não equivale a relógio; replay síncrono
limitado a1 milhão de eventos; consulta efetiva pode falhar se checker/CAS/
contexto não estiver elegível. CI/ARM/noturno ainda pendentes. Gateway futuro8.2
consumirá a consulta de uso e seus demais controles, sem falsa integração aqui.

- Segunda full interrompida deliberadamente na revisão (245.18s,exit-15):
  consulta efetiva usava a última instância de h(O), impedindo a consulta da
  testemunha de um Verified anterior ainda citável. Corrigida para seleção por
  referência exata e acrescentada regressão com repróposta idêntica. Nenhuma
  execução interrompida é contada como verificação concluída.

- Repetição final das mutações após a correção da seleção:10/10 compiladas e
  detectadas com controle positivo; fontes restaurados por hash. O primeiro
  recibo de nove mutações é anterior a essa correção, não é a evidência final.
- Terceira full interrompida cedo (36.29s,exit-15) para corrigir somente a
  formatação do gerador de vetores Python. Ruff check/format de todos os64
  arquivos passaram após o ajuste. Logs em target/verify-5.5-interrupted-format.*.

## Fechamento local

`ci/verify.sh --selftest`: VERIFY_OK/exit0 em1845.75s;
Suíte final CERTIFY/WITNESS:19 integrações passaram em409.88s.
543 testes Rust, 0 ignorados; Python: 252 passed in 92.75s (0:01:32), 25 passed in 1.37s.
As457 entradas monitoradas permaneceram idênticas. Dez mutações
com controle positivo e fontes restaurados; fuzz oficial97591/513712 execuções,
61s por alvo, exit0. Vetores históricos24 preservados; seis vetores novos.
Recibos/hashes em etapa-5.5-evidencias.json; inputs e mutações anexos.
Logs completos locais em target/verify-5.5.log e target/fuzz-5.5-final/.

CI/ARM/noturno não consultados. Consulta de ação demonstrada no kernel;
integração obrigatória do gateway8.2 continua futura. Próximo:10.1 no perfil
de pesquisa, antes da farm10.2 e do MCP5R.1, conforme ADR0037.
