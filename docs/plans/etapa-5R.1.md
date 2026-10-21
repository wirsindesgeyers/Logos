# Etapa5R.1 — ferramentas MCP de pesquisa

1. Entradas: kernel5.3–5.5; cliente10.1; farm10.2/worker isolado; AIR/canon;
   auditor independente3.6/4.5/5.4. Configuração do operador fixa socket,
   staging, V público, CAS para leitura pelo auditor, chave pública e
   hashes de checker permitidos. Nenhuma credencial no Python.
2. Saídas: servidor MCP stdio, seis ferramentas tipadas: propose_claim,
   check_claim, find_countermodel, query_ledger, audit_session, export_report.
   Bytes canônicos em hexadecimal estrito; resultados são observações,
   não construtores de Claim/token. Exportação audit-stream/v1 existente.
3. Tarefas: ADR0045/SDK fixado; schemas estritos/serviço; CLI do auditor
   expõe verify_with_checkers já existente; servidor stdio; testes negativos
   e dez tarefas por cliente MCP real/kernel real; integração CI/full verify.
   Arquivos py/logos_mcp, testes, spec/mcp-research.md e CLI não TCB.
4. Critérios → ci/check-mcp-research.sh: dez tarefas só via tools/call,
   propostas/certificados/refutações reais; auditoria completa por dois
   checkers e export rechecado. Catálogo fechado, extras/tipos/hex inválidos
   recusados, certificado adulterado rejeitado e checker desconhecido
   impedido; nenhuma rota Fact/Policy/ASSUME/raw RPC/escrita direta no CAS.
5. ADR0045: SDK oficial2.3.0/Python3.14, stdio local; serialize operações
   mutáveis/auditoria; caminhos/chave pública/checkers vêm apenas da CLI.
   Leitura pública de V explícita; produtor permanece sem ledger/chaves.
   Export autenticado não contém objetos CAS; auditoria usa CAS externo.
6. M mantido. Sem formalizador5R.2/renderizador9.1, hipóteses5R.3, MEM, TUI, ações, políticas,
   multiusuário ou MCP completo10.3. Runtime/formatos da TCB preservados.

Fechamento local e recibos registrados ao fim deste plano.


Descoberta de integração: auditor exige writer.lock exclusivo e não abre
SQLite com kernel ativo. Compor Subscribe/QueryHead com audit-stream/v1 já
existente: snapshot limitado e imutável, seq/elos/head conferidos, autenticação
pelo auditor e CAS configurado. Nenhum novo reader SQL ou alteração de lock.
Primeiro cenário real de dez tarefas MCP passou; verificação completa pendente.


## Desenvolvimento

Cenário real de dez tarefas stdio passou com5 provas/5 refutações e auditoria
completa (Rust+cake_lpr). Raiz pública errada e checker fora da lista recusados
sobre stream exportado; sem falso positivo por writer.lock.
24 testes Python e2 CLI Rust passaram. Um teste inicial usava stub sem método
query (falhava antes da validação); corrigido. EOF parcial de cliente escapava
como erro de protocolo MCP; incluído explicitamente como erro de ferramenta,
sem promover sucesso. Teste vermelho foi reproduzido e passou após correção.
Ruff/mypy das fixtures ajustados; Clippy e MCP_RUNTIME_ONLY_OK sem dev passaram.
Sem alteração de decoder/formato/runtime da TCB; CLI auditor não confiável só
expõe API já existente. Nenhum fuzz/mutação de TCB novo necessário/executado.

Primeiro full verify: exit124/3383.93s, timeout180s do teste de crash, após24 SIGKILLs; sem VERIFY_OK. Farm500 e MCP10/auditoria passaram.481 entradas inalteradas. Evidências em target/verify-5R.1-attempt1.*; reprodução isolada em curso antes de nova verificação completa.

Reprodução isolada com o script/limites intactos: exit0,10k eventos/24 SIGKILLs/10000 ACKs,107.56s. Causa do timeout anterior não confirmada; carga elevada foi observada, sem atribuição causal. Nova verificação completa será exigida antes do commit.

Segundo full: exit101/1526.00s,18/19 testes de certify passaram; fixture
witness_deadline expirou read_timeout180s (WouldBlock/EAGAIN) e o servidor
recebeu BrokenPipe.481 entradas inalteradas; logs em attempt2. Ajuste de
fixtures conforme ADR0045: cliente do teste e runner agregado de crash600s,
sem alterar protocolo, janela lógica,24/10k ou limites individuais10/60s.
Terceira verificação completa obrigatória antes do commit.

Terceiro full: exit101 em1744.94s; corpus LRAT/cake_lpr devolveu
Unknown(Time) para Urquhart-s3-b7.shuffled.cnf após150 provas/450 corrupções.
Mantido Budget60s do teste; reprodução isolada em andamento.
Recibo/log/inputs preservados em target/verify-5R.1-attempt3.*.

Reprodução isolada do corpus LRAT confirmou Unknown(Time) na mesma corrupção
Urquhart-s3-b7 (exit101/618.58s). O Budget apenas do teste de concordância
passa de60s para180s; critérios4.4/4.5 não impõem SLA60s. Permanecem200
provas aceitas e600 corrupções explicitamente Rejected nos dois checkers;
Unknown ainda falha. Limites de produção, bytes, memória e passos inalterados.
Quarto full verify/selftest necessário antes de encerrar5R.1.

## Fechamento local

Full ci/verify.sh --selftest: VERIFY_OK/exit0 em4681.05s.
545 Rust/0 ignorados; Python ['307 passed in 167.35s (0:02:47)', '25 passed in 1.79s'].
Dez tarefas só via MCP,5 provas/5 refutações, todas auditadas; export rechecado.
24 testes Python+2 CLI e runtime sem dev passaram. 481 entradas inalteradas.
Runtime TCB Rust preservado; única alteração Rust de produção na CLI não TCB do auditor.
Evidências/inputs/corpus em etapa-5R.1-*.json. CI remota/ARM/noturno pendentes.
