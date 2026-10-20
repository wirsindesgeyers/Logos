# Cliente Python10.1 — protocolo Unix, fora da TCB

API: `logos_client.client.KernelClient`, `logos_client.protocol` (modelos
Pydantic estritos) e `logos_client.staging.StagingUploader`.
Consome kernel-protocol/v1 e os resultados propose/check/witness existentes;
nenhum formato de wire novo. ASSUME tem modelo/request completo, mas o kernel
atual retorna `KernelError(code=3)`; não há handler5.7 ou sucesso inventado.

```python
from pathlib import Path
from logos_client.client import KernelClient
from logos_client.protocol import Certify
from logos_client.staging import StagingUploader

client = KernelClient(Path("/run/logos/kernel.sock"), timeout=120)
uploader = StagingUploader(Path("/run/logos/agent-staging"))

async def submit_proof(h_o: bytes, proof_bytes: bytes):
    candidate = uploader.put(proof_bytes, "certificate")
    return await client.certify(Certify(
        obligation=h_o, format="lrat-rup/v1", certificate=candidate,
    ))
```

O operador provisiona socket/staging/vocabulário; o cliente não publica
vocabulário nem abre ledger/CAS privado/chaves. `Propose` recebe goal em bytes
AIR canônicos, vocab hash e citações por h(Event). Seu resultado traz O/h(O)
para os produtores existentes. Um modelo usa `put(model_bytes,"model")` e
`Witness`/`Refute`; referências CAS usam `CasRef(hash=...)`.

`head()` e `status(h_event)` são consultas. Status não recebe h(O).
`CheckResult.state` e `WitnessResult.outcome` são observações de resposta,
sem construtor de Claim/token/autorização. O cliente não valida assinaturas:
EventRecord guarda bytes canônicos e hash; use o auditor independente para
verificação criptográfica/lógica. A string "verified" não concede autoridade.

## Transporte

Uma conexão independente por operação. Deadline absoluto de I/O, sem renovar
entre pedaços do frame; tamanho validado antes de ler o corpo, request_id
conferido também nos erros. Limites fixos do protocolo, modelos sem coerção,
normalização silenciosa ou campos extras. Texto tainted preserva NFD;
campos Text exigem canon/v1. Validação síncrona limitada por bytes não é
preemptada pelo relógio do event loop.

Erro remoto levanta KernelError; framing/resultado inválido, ProtocolError;
EOF parcial e TimeoutError nunca significam sucesso. Cancelamento propaga e
fecha a conexão. Não há retry automático: um append pode ter sido confirmado
antes de a resposta se perder. IDs não têm semântica de idempotência.

## Assinatura de eventos

`subscribe(from_seq,max_events=256,previous_hash=None)` é gerador async com
EventRecord e SubscriptionEnd(next_seq) explícito. Confere sequência, links
entre eventos da página e, se passado, o hash anterior conhecido. Isso não
autentica assinatura nem substitui replay/auditoria. Prazo inclui connect,
write/read e tempo decorrido de consumo, mas não cancela o consumidor enquanto
o gerador está suspenso em yield. Expiração é observada na próxima leitura.

Use `contextlib.aclosing` ao encerrar antecipadamente:

```python
from contextlib import aclosing
from logos_client.protocol import SubscriptionEnd

async def read_page(cursor: int):
    async with aclosing(client.subscribe(cursor, max_events=20)) as stream:
        async for item in stream:
            if isinstance(item, SubscriptionEnd):
                return item.next_seq
            # Encaminhar item.canonical_bytes ao consumidor/auditor.
```

Interrupção levanta SubscriptionInterrupted com next_seq/previous_hash apenas
dos eventos completos entregues. Um frame parcial não avança o cursor. O
caller decide retomar; não se fabrica cursor terminal nem se descarta evento.
Conexões finitas preservam quotas do servidor; Subscribe não vira polling
infinito implícito.

## Upload

`put` é síncrono, recebe bytes limitados (padrão64MiB) e domínio explícito
certificate/model. Abre diretório sem seguir symlink, confere UID/permissões,
escreve arquivo0600 privado, fsync e publica nome opaco sem sobrescrever.
A referência só retorna após remover o link temporário e sincronizar o diretório.
Falha de sync depois da publicação é explícita e pode deixar upload órfão;
a exceção identifica o nome. Nenhuma limpeza automática remove arquivo publicado.
O kernel captura/confere/importa os bytes; nome/hash não autorizam evidência.

## Verificação reproduzível

`cd py` e `uv run pytest -q tests/test_kernel_client.py tests/test_kernel_integration.py`.
A fixture constrói logos-kernel e client_fixture em Rust. Só Rust provisiona
material privado de teste; Python recebe dados públicos e inicia o binário.
As sete mensagens são exercitadas, incluindo ASSUME Unavailable e checkers reais.
Os vetores normativos fixam bytes de todos os requests/resultados. O teste
não implementa farm10.2, MCP5R.1/10.3, gateway ou memória.
