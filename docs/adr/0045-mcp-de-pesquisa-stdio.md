# ADR0045 — MCP de pesquisa por stdio

Estado: aceito para implementação da5R.1,2026-10-20.

Adotar SDK oficial MCP2.3.0 fixado no uv.lock, fora da TCB, e transporte
stdio local. Referências consultadas: https://py.sdk.modelcontextprotocol.io/
e https://github.com/modelcontextprotocol/python-sdk (versão2.3.0).
Não implementar transporte JSON-RPC próprio, HTTP, autenticação multiusuário,
amostragem de LLM ou raízes escolhidas pelo modelo.

Expor exatamente seis ferramentas, com modelos Pydantic strict/extra=forbid,
bytes hex estritos limitados e validação canônica. Propostas só chegam ao
kernel por PROPOSE; provas/modelos por CERTIFY/REFUTE/WITNESS e uploader10.1.
A restrição do roadmap de “nenhuma ferramenta escreve obrigação” refere-se
à escrita direta: geração de O continua exclusivamente no handler PROPOSE.
Nenhuma ferramenta expõe ASSUME, Fact, Policy, raw RPC, caminhos arbitrários,
comandos ou escolha de chave/checker. Descrições são estáticas; texto do
usuário continua tainted e não gera descrições/instruções de ferramentas.

check_claim usa farm ou certificado LRAT explícito. find_countermodel corre
Z3/finder isolados, submete candidato primeiro a WITNESS e depois REFUTE;
no-model/erro/Unknown não são prova de validade. Sem retry automático.
Serializar operações locais por lock, inclusive auditoria/exportação. O snapshot é um prefixo imutável capturado por Subscribe até QueryHead,
conferido por sequência/elos/head e depois pelo auditor. Outros clientes podem
acrescentar eventos; o relatório identifica o prefixo capturado, sem alegar
que ele continua sendo a cabeça atual.

Auditoria usa logos-audit com raiz pública e lista de hashes Rust fornecidas
pelo operador; CLI expõe a API verify_with_checkers existente sem mudar o
checker ou a TCB. Não inferir allowlist dos hashes no ledger. LRAT exige
Rust e cake_lpr existentes. Exportar o stream autenticado existente, rechecar
o export com CAS público e comparar head/contagens, com limite de saída e
prazo. Nunca abrir/carregar chave privada no Python. Caminhos de auditoria
são exclusivos de configuração local e não são argumentos de ferramenta.

As respostas MCP são observações de software não confiável, não autoridade
criptográfica. Auditoria explicita rechecks e provas não checadas; falhas
aparecem como erros de ferramenta, sem Verified/Refuted sintético. Artefatos
exportados continuam exigindo CAS externo/raiz pública para auditoria.


Descoberta ao integrar: source.sqlite do auditor exige writer.lock exclusivo,
logo não abre a base do kernel ativo. Usar exclusivamente Subscribe e o
formato audit-stream/v1 existente; não remover locks nem criar reader de SQL
no Python. O auditor autentica esse stream e recheca com CAS configurado para
leitura. Paths de ledger e chaves privadas não são configuração do MCP.


Verificação sob carga: primeiro full timeout180s no runner agregado de crash;
reprodução isolada intacta passou em107.56s. Segundo full falhou em EAGAIN no
read_timeout180s do cliente da fixture WITNESS e BrokenPipe subsequente. A
janela testada é lógica (1024 eventos), não SLA wall de replay debug.
Usar600s finitos para cliente dessa fixture e runner agregado de crash,
preservando os limites individuais10/60s do oráculo,24 mortes/10k appends,
asserções e prazos do protocolo/checkers/TCB. Ambos os scripts CI/local são os
mesmos. Isso não altera critérios lógicos/latência de índices nem elimina
checagens; registrar todas as execuções anteriores como falhas.

Reprodução isolada do corpus LRAT confirmou Unknown(Time) na mesma corrupção
Urquhart-s3-b7 (exit101/618.58s). O Budget apenas do teste de concordância
passa de60s para180s; critérios4.4/4.5 não impõem SLA60s. Permanecem200
provas aceitas e600 corrupções explicitamente Rejected nos dois checkers;
Unknown ainda falha. Limites de produção, bytes, memória e passos inalterados.
Quarto full verify/selftest necessário antes de encerrar5R.1.

Fechamento21/out: quarto full verify/selftest passou com VERIFY_OK/exit0
em4681.05s,545 Rust/307 Python/25 TCB, zero testes ignorados. Corpus200/600
passou em308.35s, crash10k/24 passou em85.99s; dez tarefas MCP auditadas.
Recibos das três falhas e da execução aprovada em docs/plans/etapa-5R.1-evidencias.json.
