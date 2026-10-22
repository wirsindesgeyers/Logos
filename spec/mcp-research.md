# MCP de pesquisa5R.1

Servidor não confiável: `uv run python -m logos_mcp.server`. Usa o SDK oficial
MCP2.3.0 fixado, stdio local e catálogo fechado. Sem HTTP, multiusuário, LLM,
formalizador, ações, políticas, memória, ASSUME ou RPC arbitrário. A camada
Python não carrega chave privada. Formatos canon/AIR/O/event/audit-stream/v1
existentes preservados.

## Configuração local

Obrigatórios: --socket, --staging, --vocabulary (arquivo canônico público V),
--encoder (prover_bridge), --solver (CaDiCaL), --auditor (logos-audit), --cas
(CAS para leitura do auditor), --cake (checker fixado), --public-key
(Ed25519 pública,32 bytes hex minúsculo) e --rust-checker (repetível).
--timeout é o prazo das operações de produção/kernel (padrão30s); auditoria
limitada a120s. Não há argumento de ferramenta para caminhos/checkers/chaves.
O processo supervisor precisa dos executáveis, socket/staging e CAS público
para o auditor; os workers da farm mantêm o sandbox sem ledger/CAS/chaves.
Não há configuração de ledger SQLite ou acesso direto ao banco no Python.

--rust-checker é **hash tipado de Checker**, SHA256 de
`b"logos/checker/v1\0" + bytes_do_executavel_kernel`, previamente permitido
pelo operador. Não é SHA256 cru do executável nem hash escolhido no ledger.
Auditoria LRAT exige Rust e cake_lpr; a CLI agora expõe a allowlist existente.

## Ferramentas e entradas

| Nome | Entrada | Efeito |
| --- | --- | --- |
| propose_claim | text_pt, goal (AIR hex), vocabulary (hV hex), cites opcional (hEvent[]) | PROPOSE; kernel gera O e Pending |
| check_claim | obligation (hO), obligation_bytes (O hex), vocabulary_bytes (V hex), certificate LRAT hex opcional | CERTIFY explícito ou farm10.2 |
| find_countermodel | obligation, obligation_bytes, vocabulary_bytes | Z3/finder isolados, WITNESS e REFUTE pelo kernel |
| query_ledger | vazio, reference (hEvent) ou from_seq/max_events/previous_hash | QueryHead, QueryStatus ou Subscribe finito |
| audit_session | vazio | Snapshot por Subscribe e auditor independente com recheck |
| export_report | vazio | audit-stream/v1 autenticado e rechecado, retornado em hex |

Entradas são objetos diretos de tools/call, strict/extra=forbid. Booleanos
não são inteiros; hO/hV/hEvent são64 dígitos hex minúsculos; bytes exigem
pares hex sem espaço. O/V/AIR até1MiB, LRAT até16MiB. V precisa ser exatamente
o arquivo público configurado e seu h(V) deve corresponder a O; h(O) conferido.
Texto PT preserva UTF-8/NFD e continua tainted. Apenas o kernel constrói O;
nenhuma ferramenta escreve diretamente objetos do ledger/CAS privado.

Respostas estruturadas também aparecem como JSON textual por compatibilidade.
`source=kernel, authenticated=false` são observações do cliente10.1, não
assinaturas autenticadas nem credenciais. Auto check retorna resultado/witness,
latências/cache/diagnósticos da farm. CERTIFY explícito informa que esta
chamada não pediu consistência; não inventa Witness. Busca de modelo devolve
modelo só após REFUTE aceito; ausência/rejeição/Unknown não provam validade
ou inconsistência. Erros têm isError=true; erro de transporte informa que
um append pode ter sido confirmado e proíbe retry automático.

## Snapshot e exportação

Auditor SQLite exige writer.lock exclusivo: não abrir a base do kernel ativo.
Capturar QueryHead e Subscribe desde seq0 em páginas≤256, conferir cursores,
elos e head; codificar exatamente audit-stream/v1 existente em arquivo
temporário privado. Limite agregado2MiB, incluindo header/frames; rejeitar
sufixo incompleto/cabeça divergente/limite sem devolver prefixo como completo.
Auditor autentica assinatura/cadeia com raiz configurada e recheca certificados
com CAS; contagem/head do relatório precisam coincidir com o snapshot.

Operações mutáveis e auditoria/export são serializadas localmente. Outros
clientes podem avançar o ledger: `snapshot_head` identifica o prefixo capturado,
não afirma ser a cabeça mais recente. Não é âncora de transparência13.2.
Export autentica/recheca a fonte, executa export do auditor e recheca seus
bytes; contagens e head precisam coincidir. Retorna audit_stream_hex, sha256
cru do stream, audit e requires_external_cas=true. O stream contém eventos,
não os objetos CAS: export sozinho não é pacote autocontido ou claim portátil.
Prazo/cancelamento recolhem subprocessos; stdout/stderr têm limites explícitos.

## Reprodução e limites

`ci/check-mcp-research.sh`: constrói kernel/auditor release e auxiliares,
provisiona fixtures públicas em Rust, executa dez tarefas por cliente MCP stdio
com cinco provas LRAT/cinco refutações e auditoria completa. Nenhuma proposta
ou certificado das tarefas é submetido fora de tools/call. Catálogo fechado,
entradas adulteradas, certificado rejeitado, raiz/checker errados e preservação
de texto tainted são conferidos. Relatório: target/mcp-research-report.json.
Controles negativos de configuração do auditor são externos às dez tarefas.

`uv run pytest tests/test_mcp_research.py` cobre schemas, despacho MCP,
erros, ausência/rejeição de modelos, snapshots paginados e cabeça/terminal
incorretos. Testes Rust da CLI verificam argumentos e export sem alegação de
recheck. SDK stdio decodifica JSON antes dos limites de campo: não há novo
limite de memória pré-parse de JSON ou sandbox do supervisor nesta etapa.
Fidelidade em português9.1 e UI continuam futuras. O caderno de hipóteses
(5R.3, [notebook.md](notebook.md)) reutiliza `snapshot`/`frames` deste módulo e
os mesmos componentes, sem ampliar o catálogo de ferramentas MCP.
