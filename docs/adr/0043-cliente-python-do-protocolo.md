# ADR0043 — Cliente Python do protocolo

- Estado: aceito para implementação10.1 em2026-10-20; base2c22e1e.
- Relacionadas:0035,0037,0038,0041,0042.

## Decisão

Cliente async Unix fora da TCB, com modelos Pydantic2.14.1 estritos/congelados,
canon/v1 existente e conexão independente por operação. Nenhum formato Rust
ou mecanismo de autoridade muda. Os sete requests v1 são representados,
incluindo ASSUME: integração deve observar Unavailable do kernel atual, sem
implementar5.7. Sucessos conhecidos recebem parser próprio (propose/check/
witness/head/status/assinatura); sucesso ASSUME sem contrato é erro explícito.

Prazo absoluto da operação, tamanho de frame antes da leitura e correlação
exata do request_id. Timeout/cancelamento/EOF não viram sucesso nem retry.
Subscribe expõe eventos completos não autenticados e cursor final explícito;
interrupção informa cursor derivado somente dos eventos completos observados.
Conferir sequência e encadeamento dentro do fluxo não autentica assinatura.
Nenhum resultado do cliente é Claim, token ou autorização.

Upload síncrono de bytes limitados, por descritor de diretório sem symlink,
UID/permissões conferidos, temporário0600 e publicação sem sobrescrever.
Hash com domínio certificate/model explícito; caller não fornece caminhos de
arquivo de certificado. O cliente não acessa ledger, CAS privado ou chaves.
Não repetir upload/pedido após resultado ambíguo; arquivos órfãos ficam visíveis.

A fixture Rust de testes provisiona chave pública de exemplo/arquivo0600,
vocabulário e diretórios, sem entregar material privado ao Python. O teste
Python apenas inicia o executável real do kernel com caminhos provisionados.
Pydantic/dependências Python não entram na TCB Rust; uv.lock fixa resolução.

## Validação

Vetores independentes de todos os requests e resultados; transporte adversário
fragmentado/truncado, IDs, limites, cancelamento/retomada e staging hostil.
Integração no executável real cobre as sete mensagens, incluindo erro ASSUME,
prova/modelo aceitos pelos checkers e leitura das assinaturas no ledger.
Não adiciona farm, MCP, gateway, produtor, memória ou novo handler Rust.

Referência da validação estrita:
https://github.com/pydantic/pydantic/blob/main/docs/concepts/strict_mode.md
