# Memória do UniiChat: auditoria de implementação para o LOGOS

Consulta em **2026-10-19**; base LOGOS `d41a76c`. Fontes oficiais, fixadas:

- [UniiChat `0500bf6a8ea4bf0ce047488562022b1b6532cc30`](https://github.com/VictorTaelin/uniichat/tree/0500bf6a8ea4bf0ce047488562022b1b6532cc30), commit de 19/out 16:45:05 UTC.
- [OptMem `1fb164cf39028047781f72ac3bb1e5a691c1dcb0`](https://github.com/VictorTaelin/OptMem/tree/1fb164cf39028047781f72ac3bb1e5a691c1dcb0), commit de 31/jul 02:19:57 UTC.

Revisão de arquivos e execução local de testes com respostas falsas. Não é
auditoria exaustiva do produto nem experimento pago com LLM. O objetivo é
substituir ambiguidades da descrição por contratos conferidos na implementação.
Hashes dos arquivos lidos/resultados em [uniichat-evidence.json](uniichat-evidence.json).
Decisões LOGOS: [ADR 0039](../adr/0039-memoria-baseada-no-codigo-uniichat.md),
[E§5–6/20/22–24](../spec/LOGOS-long-term-memory.md), ROADMAP Apêndice F.

## O que foi conferido

| Assunto | Implementação fixada | Implicação para o LOGOS |
| --- | --- | --- |
| Árvore e Free | [Memory.source/starts/store](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Memory.ts#L575-L658): folha usa `line` fornecida ou mensagem com autor; pai usa dois filhos; fonte que cabe dispensa modelo | geometria/Free continuam; origem/geração/digest/flags são exigências extras nossas |
| Duas views | [load/save](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Memory.ts#L197-L246): `view` e `seen`, parts/batch persistidos | não reconstruir o contexto livremente a cada job ou reboot |
| Histerese | [fit/merge](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Memory.ts#L831-L957): high=128000/32000; lotes até metade | antigo LOGOS dobrava só o suficiente para caber; falta de espaço livre reescreve prefixo cedo |
| Due | [offer/heap](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Memory.ts#L899-L951): `(t+1)/2^l-i`, empate por posição | corrigir para `(H+1-start)/child_count`, com teste que distingue a fórmula antiga |
| Cutoff/contexto | [ctx/first](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Memory.ts#L584-L592), [build](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Memory.ts#L725-L755) | pai pode consultar detalhe já pronto do seu intervalo; folha não consulta a si própria |
| Blocos | [Memory.view](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Memory.ts#L1001-L1019) | quatro linhas por bloco; marcar só o último completo, parcial/fechamento no tail |
| Single flight | [AskAI.keys/wait/lead](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/AskAI.ts#L213-L285), [Agent.request/send](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Agent.ts#L337-L383) | coordenar aquecimento no send; liberar ao começar resposta, não ao terminar |
| Request real | [Server.share](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Server.ts#L335-L342), [call](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Server.ts#L524-L561) | fresh call: view histórica, devices/work e input atual; ferramentas chegam pelo adapter |

## Cache: quatro coisas que não se confundem

O tree store já guarda o resultado de um resumo. O **prompt cache remoto**
reutiliza computação de entrada, não resposta. O `CACHE` de AskAI é uma tabela
local de aquecimento/TTL, não guarda texto de conversa ou resposta do modelo.
Índices de busca, por sua vez, não são nenhum desses caches.

O prefixo precisa coincidir no request serializado: tools/system, modelo,
opções, roles, conteúdo, ordem e limites dos blocos. O tail variável vem depois.
No [adapter Anthropic](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Anthropic.ts#L60-L115)
há marca de system, marca de view, marca de input e cache automático de fim:
o orçamento total do simulador é quatro. Só marcar um grande bloco mutável
não conserva fronteiras antigas de cache. A marca móvel em blocos de quatro
linhas deixa a maior parte do prefixo estável entre lotes.

AskAI usa hash encadeado Bun 64 de vendor/model/effort/speed/system/searches e
blocos até a última marca **antes** do bloco final. Busca o maior prefixo
conhecido: aguarda líder pendente ou aproveita prefixo quente e lidera a
extensão. A tabela é global no processo, TTL estimado de 300s/1800s e limpeza de
expirados; não sobrevive ao restart. Agent só entra nela depois de Bill/fila;
não deixa outras chamadas esperando por usuário sem crédito. O início da
resposta é o ponto de liberação ([Anthropic.wire](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Anthropic.ts#L300-L312),
[OpenAI.wire](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/OpenAI.ts#L215-L227)).

As marcas selecionam fronteiras, mas não entram no hash de conteúdo: mover
uma marca preserva as chaves dos blocos anteriores. Hashear o JSON inteiro,
incluindo diretivas móveis ou teto de output, destruiria essa propriedade.

```text
7 linhas: header | [0–3, marca] | [4–6 + fechamento]
9 linhas: header | [0–3]        | [4–7, marca] | [8 + fechamento]
                    ↑ mesmos bytes e mesma chave
```

**Adaptação LOGOS:** fingerprint inclui tools/schema, endpoint, adapter,
credencial sem segredo e domínio de acesso; SHA-256, cancel/deadline/fencing e
nenhuma coordenação entre domínios. Cache é dispensável para correção. Mesma
instrução system não compartilha cache entre compactor e agente se modelos ou
tools diferem. Os campos de [OpenAI.body](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/OpenAI.ts#L36-L76)
são fatos do source, sem ensaio de compatibilidade com a API externa; não
copiá-los como se fossem um contrato verificado.

## Compactor, folds e turnos

O compactor é outro modelo configurável, sem ferramentas, com preâmbulo
comum ([Tools.system](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Tools.ts#L248-L269)
acrescenta regras configuradas; editá-las muda o prefixo). Recebe uma projeção
da sua view pronta e Target separado. Para um pai,
cutoff é seu end exclusivo; para folha, seu start. Cutoff também é limitado
pela primeira folha pendente. Isso permite referenciar detalhe dos children
e impede esperar o próprio summary para construir seu contexto. O agendador
ordena ctx crescente, nível maior em empate, numa janela limitada pelas 64
primeiras folhas pendentes por lado; chat e import têm filas/holds próprios.

**Retry não é conversa que cresce.** São até cinco respostas candidatas;
requests seguintes preservam blocos iniciais e acrescentam só uma nota de
tamanho/corte, substituída a cada resposta. O texto de corte só é feedback.
O source guarda a menor tentativa, **mesmo se todas excederem 512 bytes**.
Resposta vazia/recusa sem linha usa fonte achatada e recortada. Indisponibilidade
é outro caminho: AskAI patient aguarda 60s e retenta; pausa/crédito seguram o
lado. O LOGOS mantém o input limitado do retry, mas usa limite rígido,
Locator/indisponibilidade explícitos, contadores/deadline e backoff limitado.

Merge exige siblings adjacentes com pai pronto, pode aumentar bytes e usa
heap/lista para não reexaminar a história inteira. Em `fit`, sem append, um
lote candidato só substitui parts se atingir
`size <= max(high/2, old_size-high)`; em append (`next=true`), pode publicar
progresso parcial e manter batch ativo. Settle upstream espera folhas do lado
chat antes do cutoff; **não** garante por si frame inteiro escapado<=128 KB,
nem ausência de resumo acima de 512 bytes. Imports têm um lado próprio e
podem estar pausados enquanto o chat segue; isso não satisfaz automaticamente
a cobertura integral exigida no LOGOS. O LOGOS adiciona esses gates e publica lote
pronto em LOW. Não atribuir as garantias LOGOS ao upstream.

Há uma diferença que cabe em oito eventos: para pares `(0+2,2+2)` e
`(4+1,5+1)`, a fórmula antiga `(H-start)/(4*count)` empata. A fórmula literal
produz 4,5 e 5: ganha o par recente de escala 1. Nosso caso chama Memory.merge
e observa a partição `0+2,2+2,4+2,6+1,7+1` após um merge.

`Server.call` monta chamadas fresh e separa histórico de input atual, mas já
registrou os inputs. Pode mover `from` com novas entradas; `Memory.whole`
limita o tail integral. Nosso receipt/snapshot antes da admissão e replay
durável são uma adaptação, não tradução literal. Navegar por zoom retorna
dados no tail; não é comando para refinar persistentemente a view automática.

## OptMem não é o harness UniiChat

[`memo.cover/cmd_wake`](https://github.com/VictorTaelin/OptMem/blob/1fb164cf39028047781f72ac3bb1e5a691c1dcb0/memo#L82-L130)
recalcula cover por orçamento de 96 linhas, com busca de alpha e refinamento do
presente. Não é a política incremental de cache do UniiChat. O log guarda
**notas já limitadas a 280 bytes**, não a conversa bruta; registros fixos de 320/288 bytes
permitem seek. [`nap_prompt`](https://github.com/VictorTaelin/OptMem/blob/1fb164cf39028047781f72ac3bb1e5a691c1dcb0/memo#L463-L486)
lê raw até blocos de 16 notas; acima disso, dois resumos. O agente fornece a
compressão via CLI; não há o mesmo worker/adapter/single flight do UniiChat.
Não importar RAW_MAX=16 nem cover como se fossem árvore binária pura e folding
da extensão LOGOS. A paginação wake fixa T para evitar mudança entre páginas.

## Retenção e confiança: divergências explícitas

- [Clip](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Clip.ts#L1-L76)
  pagina mensagens armazenadas sem recortá-las; output de comando passa por
  [Machine.shell](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Machine.ts#L161-L190),
  que mantém head/tail e elimina o meio. Logo, “log preservado”
  não significa todos os bytes que o processo externo emitiu. LOGOS guarda
  output integral antes de produzir preview/descriptor.
- [Disk](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/Disk.ts#L1-L81)
  usa group flush; Memory salva view por arquivo temporário/rename, recupera
  índices/cauda e aceita saved ausente. LOGOS exige WAL/FULL, receipts,
  pins/snapshots e erro explícito se estado necessário sumiu. Não inferir
  durabilidade LOGOS de um teste de cache do upstream.
- Fonte, summary, OCR, retrieval e cache permanecem não confiáveis para
  autoridade epistêmica. Nenhum desses resultados habilita Verified/token.
  Python LOGOS continua sem credenciais; provider autorizado é outro processo.

## Reprodução e resultado observado

Clones públicos sem instalação de dependências nem configuração de conta:

```sh
git clone https://github.com/VictorTaelin/uniichat.git /tmp/logos-uniichat-audit
git -C /tmp/logos-uniichat-audit checkout --detach 0500bf6a8ea4bf0ce047488562022b1b6532cc30
git clone https://github.com/VictorTaelin/OptMem.git /tmp/logos-optmem-audit
git -C /tmp/logos-optmem-audit checkout --detach 1fb164cf39028047781f72ac3bb1e5a691c1dcb0
```

Com Bun 1.3.14, no clone UniiChat: `bun test --isolate cache.test.ts failsafe.test.ts`.
**7 pass/0 fail, 9285 asserções, 42,00s**. API falsa, tempo simulado/acelerado.
No clone OptMem: `python3 test.py`: **109099 checks passaram/0 falharam**,
compressor sintético. Não é teste LLM de recall.

O [cache.test.ts](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/cache.test.ts#L91-L123)
simula 20 blocos de lookback, TTL de 300s, disponibilidade após 1500ms e até 4 marcas.
Em 3000 mensagens sintéticas, reportou **1503 requests de turno/98,6% read** e
**3831 compactações/96,4% read**. O acumulador usa comprimento de strings JSON;
o denominador das compactações **exclui a tarefa**. Não são tokens reais,
desconto observado, garantia >=95% em toda carga ou resultado de memória LOGOS.
Os [failsafe tests](https://github.com/VictorTaelin/uniichat/blob/0500bf6a8ea4bf0ce047488562022b1b6532cc30/failsafe.test.ts#L124-L251)
distinguem outage/recusa, holds por crédito, liderança depois do pagamento e
justiça entre imports em rate limit.

Casos adicionais contra funções reais, com hashes de fontes conferidos:

```sh
LOGOS_UNIICHAT_REFERENCE=/tmp/logos-uniichat-audit bun test --isolate docs/research/uniichat-reference.test.ts
```

**8 pass/0 fail, 51 asserções, 0,804s** na execução final. Casos: Free, due divergente, histerese, blocos/marca, cutoff de pai,
retry bounded/oversize soft, recusa versus down e liberação single flight.
Arquivo de teste é apenas instrumento de pesquisa, sem módulo de memória
LOGOS nem dependência nova no projeto. Não entra na CI principal para não
exigir Bun/download externo na verificação da TCB. Resultados/limitações,
incluindo a primeira tentativa com EPERM no sandbox, ficam no JSON de evidência.

Verificação completa do LOGOS: `ci/verify.sh --selftest`, **VERIFY_OK/exit0,
1458,31s**; 252 testes Python + 25 da TCB, Rust e autoteste passaram, nenhum
teste Rust ignorado. Os 424 arquivos acompanhados de runtime/formatos
conservaram seus hashes. Log local `target/verify-memoria-uniichat.log`,
hash/resultados no JSON de evidência. Isso verifica a base existente após
a revisão documental; não demonstra implementação de MEM. CI remota não observada.

Antes de implementar MEM, conferir novamente o commit: seguir main pode
mudar conclusões. Cada etapa exige seus próprios testes do runtime LOGOS,
mutantes/controles, fonte integral e gates de autoridade existentes.
