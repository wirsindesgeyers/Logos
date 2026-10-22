# Etapa9.1a — templates explícitos no vocabulário

1. Entradas: Vocabulary/AIR1.1–1.3, parsers1.4 e formatos existentes.
2. Saídas: vocabulary/v2 com render_pt; v1 preservado; validação limitada;
   compatibilidade por símbolo; superfície Rust/Python preserva os dois formatos.
3. Tarefas: ADR0046/spec; dados/codec/validador; consumidores/impressoras;
   vetores independentes; regressões/diferencial; mutações e fuzz; full verify.
4. Critérios→ testes Rust/Python e vetores; mutações de validação/hash/compat;
   fuzz render_template/vocabulary_decode com controles; recibo full verify.
5. Decisões: ADR0046; lista alinhada a símbolos; placeholders numerados ordenados;
   formatos distintos e nenhum fallback para gloss_pt. Sem nova dependência TCB.
6. M; renderizador/parser da língua PT e revisão humana permanecem9.1.

Concluída localmente em2026-10-21 (ver §Fechamento). CI remota/ARM/noturno pendentes.

## Revisão antes do fechamento

Primeiro full e fuzz interrompidos voluntariamente (exit-15), em446.79s e
565.68s respectivamente, antes de completar. Sem VERIFY_OK/1h de fuzz.
Revisão identificou comparação de templates por buscas lineares repetidas,
quadrática; substituída por índice BTreeMap por nome. Sem alterar contrato
ou critérios. Refazer mutações/controles, fuzz e full com as fontes finais.
Logs/recibos anteriores em target/*9.1a-attempt1.*.

Segundo full: exit1 em4628.09s, Ruff do novo gerador (I001/RUF005).
Rust/farm500/MCP v1+v2/crash/índices passaram;491 entradas inalteradas.
Gerador corrigido com bytes de vetores idênticos e incluído em mypy estrito;
preflight completo de py/tools/tcb antes do terceiro full. Fuzz anterior
3600s+60s passou e fontes TCB/fuzz não mudaram. Não contar full parcial como sucesso.

Preflight Python completo detectou uma falha anterior da fixture10.2:
313/314 passaram; timeout/cancelamento lia worker.pid antes da escrita
(arquivo ausente na suíte, vazio na reprodução isolada). Capturar o Process
real e aguardar criação antes do cancelamento elimina a corrida/stale PID;
assertar retorno SIGKILL e ausência do PID. Mantidos timeout100ms e produção.
Repetir o preflight após essa correção; fuzz/TCB não mudaram.

Terceiro full: exit101/1145.85s; processo do kernel vivo, porém sem socket
no teto15s da fixture binary_serves, antes de qualquer RPC.13/14 transport
passaram. Reprodução isolada intacta passou em15.22s (tempo total do teste,
não medição exclusiva de startup). A inicialização confere/hash do ELF debug
(34820472 bytes) e registra checkers. Teto de espera da fixture120s finitos,
alinhado à fixture cliente10.1; nenhum SLA/timeout de protocolo/checker mudou.
Novo full obrigatório. Preflight Python corrigido passou314+25 testes;
fuzz1h permanece válido porque as fontes de parser/TCB não mudaram.

Quarto full: interrompido sem veredito às16:30 (farm400/500), quando o
processo e o wrapper deixaram de existir; nenhum erro no log. Não contado como
falha nem sucesso; log preservado em target/verify-9.1a-attempt4-interrupted.*.

## Fechamento

Quinto full verify/selftest, lançado desacoplado da sessão sobre as fontes
congeladas: VERIFY_OK/exit0 em3566.59s,491 entradas inalteradas;551 Rust/0
ignorados, Python314+25, RENDER_TEMPLATE_VECTORS_OK (14 casos),
RENDER_TEMPLATES_SMOKE_OK (10 tarefas, vocabulary/v2, auditoria completa),
MCP v1, farm500 (p50=1.258s), crash e índices. Fuzz render_template3600s e
vocabulary_decode60s sem crash; quatro mutações detectadas com controles
positivos e fontes restauradas por hash. TCB de correção15262+747130=762392
(+131 próprios), build-only64508, sem dependência nova. Recibos versionados em
[evidências](etapa-9.1a-evidencias.json) e [inputs](etapa-9.1a-inputs.json).
A9.1 (render_pt/parse_pt, gramática, round-trip100k, fuzz≥1h e revisão humana
de50 paráfrases) continua integralmente aberta.
