# LOGOS Spec Extension — Long-Term Memory

Extensão proposta da [especificação LOGOS](LOGOS-proof-carrying-agents.md), a incorporar como §15 sem renumerar §§1–14. `E§N` identifica as seções deste documento; `§N` identifica a especificação existente. Não declara módulos implementados nem modifica os critérios já demonstrados no repositório.

Decisões propostas: [ADR 0008](../adr/0008-memoria-de-longo-prazo.md), estado proposto. Extensão documental; adoção e etapas de código são distintas.

Referência de implementação obrigatória: **UniiChat**, commit
[`0500bf6`](https://github.com/VictorTaelin/uniichat/tree/0500bf6a8ea4bf0ce047488562022b1b6532cc30),
consultado em 2026-10-19. O [relatório de código](../research/uniichat-memory.md)
fixa também OptMem `1fb164c`, funções/linhas, testes e divergências. O
[gist OptChat](https://gist.github.com/VictorTaelin/91837951a5ce5b38f341ec1ba1df6449)
foi a referência descritiva de 04/out; não substitui o código para cache,
compactor ou folding. [ADR 0039](../adr/0039-memoria-baseada-no-codigo-uniichat.md)
registra esta revisão. As etapas propostas estão no ROADMAP, Apêndice F
(MEM.0–MEM.7); não são os invariantes M1–M13 nem etapas já executadas.

## 1. Purpose

**Episodic Memory informa o que foi registrado como tendo acontecido. Knowledge Memory informa onde conteúdo relevante pode existir. A arquitetura epistemológica existente decide o que o LOGOS está justificado em afirmar.**

As memórias melhoram continuidade, navegação e descoberta. Não produzem Fact, Policy, Proof obligation, Claim<Verified>, Seal, PVC nem capability. Não substituem Proof Ledger, AIR, Semantic Contract/ToolContract, atestadores, checkers, kernel ou gateway. Integridade dos bytes não prova significado, autoria alegada ou verdade. Permanecem L1–L3 e os princípios P1–P8 de §12.

Escopo: especificação dos dois módulos e sua integração. Fora do escopo: implementar etapas futuras, novo checker, linguagem lógica, banco de fatos concorrente, nova política de autorização ou sessão LLM com contexto acumulado entre turnos.

## 2. Architectural Integration

```text
harness / logos_agent (não confiável)
  ├─ Memory Router ── Episodic Memory: histórico → árvore → view / zoom
  └─ Memory Router ── Knowledge Memory: registry → índices → fontes
                              │ dados/candidatos, nunca premissas citáveis
                              ▼
         formalizador/extrator já previsto → PROPOSE / Says / Extracted
                              ▼
                kernel → Proof Ledger → gateway
```

A integração pertence à camada de harness/retrieval de §§5/12. Reutiliza canon/v1, SHA-256 separado por domínio, CAS e SQLite. A divisão do [ADR 0002](../adr/0002-divisao-de-linguagens.md) permanece: serviço local de persistência/protocolo em Rust; compactor, parsing, embeddings/retrieval e harness em Python; CLI/TUI Rust. Python continua sem chaves e credenciais. A memória não escreve no ledger nem no CAS privado do kernel.

Runtime Python segue o requisito atual do repo: 3.14 e Unicode 16.0.0. A referência histórica a 3.12+ no ADR 0002 não reduz esse requisito nem permite trocar canon/v1.

A distinção entre confiança de correção e confiança de custódia é explícita: armazenamento/captura precisam funcionar para preservar história; um bug de memória pode omitir/falsificar contexto, mas não deve construir um Verified ou token. Confidencialidade, disponibilidade e fidelidade histórica têm premissas próprias; não se escondem atrás de “fora da TCB de correção”.

## 3. Episodic Memory

### 3.1 Histórico original

Um MemoryStore duradouro pertence a um domínio de autorização. Todos os seus agentes, projetos, sessões e produtores usam **um sequenciador**, não contadores por sessão/dia/agente. `EventId=(store_id,seq)` é global nesse histórico, único, permanente e monotônico. O qualificador store_id torna IDs globalmente únicos entre instalações; não se promete ordem total entre instalações federadas. Separação de permissões exige stores distintos, não esconder eventos dentro de uma árvore compartilhada.

`seq` começa em 0. O intervalo histórico é `[0,H)`, onde H é o próximo seq. SQL usa inteiros não negativos até `MAX_EVENTS <= 2^63-1`; overflow gera SequenceExhausted, nunca wrap/reset/reuso. Timestamps UTC são metadados: relógio atrasado não altera ordem. Uma importação ganha novos IDs e guarda `imported_from`; nunca renumera eventos existentes.

Tipos: user, assistant, tool_call, tool_result, work, note, system_event. Incluem decisões/correções/preferências/requisitos, erros/tentativas/resultados, mudanças observáveis no projeto, relatórios e referências de artefatos. work é atribuído ao subagente real, **não user**, mesmo que o texto imite o usuário. system_event é dado operacional, não instrução system. Não se persiste raciocínio privado do modelo.

Conteúdo original é Bytes, não Text NFC: preservar UTF-8 não NFC, bytes de ferramentas e documentos exatamente. Eventos grandes referenciam o artefato completo; byte_length conta o conteúdo lógico original, não o tamanho do descriptor. Metadata tem schema/limites; texto não escolhe actor, kind ou permissões. Atribuição local de canal não equivale a assinatura de atestador.

### 3.2 Append e recepção

Recepções entram primeiro numa inbox durável com receipt_id/idempotency_key e conteúdo integral. Isso preserva inputs enquanto settle aguarda, sem colocar a mensagem atual na view histórica. Uma chave repetida com bytes diferentes é conflito, não retry válido.

O único capture owner serializa admissão ao histórico. Outras sessões/subagentes entregam envelopes à inbox; não disputam IDs nem escrevem diretamente nas tabelas. Append confirma somente após conteúdo/refs duráveis e commit SQLite WAL com synchronous=FULL. Correção é novo evento com referência ao anterior. A API não tem update/delete de eventos.

Streams de resposta são spoolados antes de exposição; ao concluir, registra-se o conteúdo observável completo. Recuperação de interrupção registra explicitamente uma resposta interrompida e os bytes realmente observados. Spool parcial nunca vira mensagem final ou summary pronto. IDs comprometidos não são reutilizados; seq não comprometido não foi publicado.

## 4. Binary Summary Tree

Para cada geração de política g, `N_g(s,n)` cobre exatamente eventos `[s,s+n)` do mesmo store:

- n é potência de dois, s mod n = 0, soma checked e s+n <= H;
- folha `N_g(i,1)` referencia o evento i;
- pai `N_g(s,2n)` tem exatamente `N_g(s,n)` e `N_g(s+n,n)`;
- pai só é publicado depois dos dois children completos; hashes/geração/ranges são conferidos;
- identidade estrutural é `store/s+n`; a referência persistida inclui geração e digest. `2184+8` cobre eventos 2184–2191, não IDs de sessão nem offsets de arquivo;
- cada node publicado é imutável. Nova política cria outra geração, sem modificar originais ou snapshots existentes.

A árvore pode ser floresta de raízes diádicas quando H não é potência de dois. Todo evento tem folha; descida geométrica termina nela. Não há blocos alternativos de 16 mensagens, children variáveis ou pai que leia um intervalo bruto diferente dos seus dois children.

### Free nodes

Folha: construir `kind + ': ' + conteúdo textual completo`. Usar literalmente se UTF-8 válido/renderizável e cabe em NODE_SIZE. Binário ou texto grande não é cortado: usa artefato completo como fonte e compactor ou Locator explícito.

Pai: testar bytes UTF-8 de `childA.text + '\n' + childB.text`, incluindo separador. Se cabe, esse texto é o pai e não há chamada LLM. Caso contrário, compactar os dois children íntegros. Free é representação derivada sem perda nesse passo; não ganha autoridade. Um free leaf curto pode coincidir literalmente com o original, mas a view sempre referencia um node publicado, nunca injeta raw recente por um caminho especial.

Pais herdam flags de degradation/source_coverage dos children, inclusive quando Free. Concatenação sem chamada não transforma resumo Model/Locator em original literal ou evidência melhor.

## 5. Compactor

Componente lógico `logos-memory-compactor`, separado do modelo principal, sem ferramentas, credenciais, escrita no Proof Ledger ou acesso irrestrito a arquivos/rede. A saída é aproximação para navegação. Os IDs estruturais são acrescentados pelo serviço, não pedidos ao modelo.

Prioridade: (1) decisões explícitas do usuário; (2) correções; (3) requisitos; (4) preferências persistentes; (5) mudanças reais de estado; (6) resultados importantes; (7) erros/causas; (8) perguntas abertas; (9) artefatos/localização; (10) contexto para referências futuras. Descrever tentado/observado sem inflar concluído. Tool noise recebe pouco espaço; “build falhou por X; log integral em artifact ref” é preferível a milhares de linhas.

### Contexto limitado

A entrada distingue Context (view auxiliar só para compreensão) de Target (evento completo ou os dois children). O contexto é congelado no job e não ultrapassa o fim do alvo; para folha, não inclui o próprio evento. Para pai, os children trazem o intervalo-alvo e o contexto pode mostrá-lo com maior resolução. Contexto não vira conteúdo pertencente ao intervalo nem prova de uma correção posterior.

Manter **ContextView persistida própria**, distinta da ConversationView. Sua
fronteira avança até a primeira folha não READY da geração (frontier), sem
esperar o job que a está lendo. Para folha, `ctx=start`; para pai,
`ctx=start+count`: o contexto pode mostrar os children/descendentes já prontos
com mais detalhe que os dois textos do Target. Essa é a regra de
`Memory.ctx/build` do código, corrigindo a exigência antiga de prefixo sempre
anterior a start. Nenhum evento posterior ao alvo é incluído. Context é
auxiliar e pode sobrepor o Target; não muda o intervalo de origem do node.

Congelar a projeção já disponível até `min(ctx,frontier)` quando o job começa.
Incluir somente parts inteiras com end <= cutoff; não refinar parts antigas
para fabricar um contexto novo por job, nem chamar settle recursivamente.
Uma part que cruza o cutoff fica de fora; registrar ranges omitidos. A view
contextual também dobra em lotes, nominalmente 32.000→16.000 bytes. Como a
floresta mínima pode exceder esse orçamento, ela pode omitir parts antigas
inteiras e registrar omitted_prefix, conservando um sufixo. **Essa permissão
não existe para a view histórica da conversa**, que cobre tudo. Metadados de
omissão/referências ficam após o prefixo cacheável. Reply_to/references podem
acrescentar poucas referências anteriores autorizadas no tail com budget
separado. Nenhuma fonte ausente é inventada.

Fila dependency-driven, por store/geração, sem rescans de toda a história.
Window de agendamento limitada pelas primeiras COMPACTION_JOBS folhas ainda
pendentes; entre fontes prontas nessa janela, ordenar por ctx crescente e,
em empate, maior nível primeiro. Pais exigem dois children READY; jobs não
esperam contexto ainda por construir. Free nodes são publicados em lote sem
gastar chamada. Context/render/template/policy/provider e os blocos iniciais
ficam congelados no job, inclusive após retry/restart. Leases/backoff e limite
de concorrência são do LOGOS; 64 jobs **por lado** (chat/import) no upstream
não é o default LOGOS nem autorização para multiplicar concorrência por agente.

### Enforcement e falhas

Defaults configuráveis e experimentais: NODE_SIZE=512 bytes, VIEW_SIZE=128000 bytes, COMPACTOR_CONTEXT_BYTES=32000, CACHE_BLOCK_LINES=4, COMPACTION_JOBS=8, COMPACTION_RETRIES=5 respostas candidatas totais, TOOL_OUTPUT_INLINE_CAP=30000 caracteres Unicode. Não equivalem a limites de tokens. A implementação verifica capacidade do provider antes da chamada. Falhas de transporte têm contador/deadline separado e também limitado; uma tentativa interrompida fica registrada, sem permitir reiniciar contadores indefinidamente.

Medir UTF-8 após trim externo; não confiar no LLM. Summary deve ser não vazio, UTF-8 válido, sem C0 exceto LF/CR/TAB e sem DEL. Originais não passam por essa restrição. Guardar a menor tentativa válida (desempate: primeira) para diagnóstico. Publicar a primeira que cabe; nenhuma tentativa oversized vira READY.

O compactor usa o preâmbulo estável de interpretação de memória, **sem tools
de usuário, busca web ou tool schema ativo**, e tarefa no último bloco não
cacheado. Os IDs são montados pelo serviço. Uma régua sem conteúdo semântico
pode indicar o tamanho; não usar exemplo cuja história possa ser copiada.
Retry por tamanho é uma **nova chamada com os mesmos blocos iniciais + uma
única nota não marcada**, substituindo a nota anterior: tamanho real, limite,
prefixo UTF-8 bounded da tentativa e posição do corte. O corte é feedback,
nunca node nem original. Não reenviar transcript de tentativas/respostas,
raciocínio do modelo ou notas acumuladas; não duplicar Context/Target a cada
retry. Contexto+Target+feedback têm teto de input verificado pelo adapter.

Depois das tentativas/deadline: publicar, quando autorizado pelo perfil, um **Locator** determinístico: “Resumo semântico indisponível; use zoom.” Ele referencia o evento/children completos, tem mode=Locator e degradation explícita, não é placeholder de trabalho ainda pendente nem summary semântico fictício. Se Locator desabilitado, settle bloqueia/cancela. Melhor tentativa oversized pode existir só em diagnóstico restrito, nunca na view.

Inputs integrais que excedem o contexto do provider produzem InputTooLarge/Locator, não compressão de prefixo disfarçada de mensagem inteira. Tool output grande pode usar o descriptor completo do artefato como alvo com source_coverage=ArtifactDescriptorOnly; não se afirma ter resumido bytes não lidos. Retry de transporte usa atraso configurável, lease e deadline, liberando vaga durante backoff. Distinguir interrupção/indisponibilidade, bloqueio de orçamento/egress e recusa semântica do modelo. Nenhum desses vira summary de erro como se fosse memória. São desvios explícitos de `AskAI.patient` (retenta indisponibilidade a cada 60s), `Memory.build` (guarda a menor resposta mesmo oversized ou recorta fonte em recusa) do UniiChat fixado.

TOOL_OUTPUT_INLINE_CAP limita somente a entrega inline ao modelo do turno: até o cap, resultado textual completo; acima dele, descriptor completo FullOutputStored com hash/bytes/ref e leitura explícita. Os bytes originais completos são persistidos antes da entrega. Não se mantém apenas head/tail no histórico, nem se apresenta preview como mensagem completa. O código upstream pagina mensagens guardadas (`Memory.pages/Clip.clip_pages`), mas recorta head/tail do output de comandos **antes** da memória (`Machine.ts/Clip.clip`). Distinguir esses caminhos; o LOGOS não adota o segundo como retenção integral.

## 6. Memory View

Snapshot de parts cronológicas que particionam `[0,H)` sem buracos ou overlap. Cada part referencia um node READY Free, Model ou Locator. Contém summaries, nunca raw message não representada, placeholder “not summarized yet” ou texto arbitrariamente truncado.

Formato renderizado, como dados não confiáveis:

```text
<episodic-memory>
0+1024|resumo derivado...
1024+512|decisão histórica...
...
2046+1|resultado observado...
</episodic-memory>
```

Strings são escapadas; texto contendo tags, aspas ou fake roles não modifica o envelope. O parser/protocolo não é baseado em encontrar essas tags no texto. IDs omitem o store apenas porque a chamada está vinculada ao snapshot/store no servidor. Datas são recuperadas por ferramenta. Estado volátil/snapshot_id/H vai no trailer ou metadata da chamada, não no prefixo system/tools.

Renderer normativo: `start+count|payload\n`. Percent-escape com hex uppercase `%`, `<`, `>`, `&`, `|`, aspas, backslash e LF/CR/TAB; controles C1 e U+2028/U+2029 são escapados byte a byte. Demais bytes UTF-8 válidos permanecem literais. Cada byte custa no máximo três bytes; `%` é escapado para evitar ambiguidade. Header/trailer são constantes ASCII bounded. HTML/JSON escaping com expansão maior não pode substituir esse renderer sob o mesmo piso. O frame é um bloco de dados tipado pelo adapter, não protocolo controlado por tags dentro do texto.

### Folding incremental

Append adiciona folha na cauda; fit substitui apenas siblings adjacentes pelo pai READY. Nunca divide uma part antiga automaticamente. Entre pares elegíveis, maximizar **`(H+1-start)/count`**, onde H é o end exclusivo da view e count é a escala de **um child**. É a tradução literal de `(t+1)/2^l-i` em `Memory.merge`, não uma fórmula reconstruída da descrição. Desempatar menor start. Não depende de score semântico nem reorganiza a cronologia.

Comparar `(H+1-a.start)*b.count` com `(H+1-b.start)*a.count` em u128 checked;
não floats/divisão inteira. Com MAX_EVENTS<=2^63-1, cabe inclusive H+1.
Regressão obrigatória: H=8, pairs `(0+2,2+2)` e `(4+1,5+1)` têm scores 4,5 e
5; escolher o segundo. A fórmula antiga empatava e escolhia o primeiro.

**Histerese persistida:** HIGH=VIEW_SIZE; uma view sem batch e <=HIGH só apenda
cauda. Ao ultrapassar HIGH, abrir batch e dobrar até LOW, não apenas até HIGH.
Guardar batch/parts/revisão, inclusive o progresso privado. Conclusões de jobs
fora de batch não reotimizam a view. No LOGOS, publicar a nova revisão de batch
somente quando pronta e <=LOW; o upstream aceita alguns lotes incompletos em
fronteiras de append (`Memory.fit(next=true)`). Essa diferença dá ao LOGOS um
contrato simples de publicação dentro do teto, podendo aumentar settle.

Pai pode ser maior que a soma textual dos children. Mesmo assim, merge diminui número de parts; não exigir redução imediata de bytes, que poderia causar deadlock. Nada oversized é enviado: fit/settle continuam até caber. Nodes ausentes são agendados; view interna pode exceder budget enquanto aguarda, view publicada não.

Medir **frame renderizado inteiro**: header/trailer, IDs, separadores, escapes e UTF-8. Não só node.text. Renderer usa escapes cujo custo é no máximo 3 vezes os bytes válidos de summary; limite de overhead por linha=44 bytes, header+trailer<=1024. Config deve satisfazer:

`VIEW_SIZE >= 1024 + bit_length(MAX_EVENTS) * (3*NODE_SIZE + 44)`.

Esse piso F cobre a floresta mínima em todos os H permitidos, inclusive summaries adversariais. NODE_SIZE deve caber Locator e ser >=64. Definir **LOW=max(floor(HIGH/2),F)** e exigir F<HIGH. Com MAX_EVENTS=2^63-1, F=100564 e LOW=100564 para HIGH=128000; não prometer faixa 64–128KB com esse renderer/limite. Um perfil com MAX_EVENTS=2^32-1 permite LOW=64000. O UniiChat mede só textos dos nodes e usa LOW=HIGH/2, sem esta garantia de frame escapado. Budgets menores exigem outro perfil compatível, não truncamento. História matematicamente ilimitada não cabe em IDs finitos; sob o limite declarado, a view tem teto fixo. Crescimento do log/árvore/disco não é constante.

Persistir parts, revisão, geração, watermark H e digest em commit atômico. Restart retoma esse snapshot e replay somente da cauda; não calcula livremente outra view de toda a história. Mudança de política/renderer/budget é migração explícita; conserva ranges ou os coarsens, nunca afina história silenciosamente. Cache é otimização de provider, não fonte de persistência.

### Blocos e caches

Renderizar header constante, blocos de **quatro linhas completas** e tail com
0–3 linhas + fechamento. Marcar para cache somente o último bloco completo;
se não existe, marcar o header. Fechamento, IDs de snapshot/cutoff, contexto
documental, devices/work e tarefa atual vão depois dessa fronteira. Marca
móvel não muda os bytes/limites dos blocos anteriores. Escapes, quebras de
linha, roles e schema/order de tools são determinísticos. Não juntar a view
toda num único bloco que muda a cada append. Cada geração fixa renderer e
CACHE_BLOCK_LINES; restart preserva parts e segmentação, mesmo com cache frio.

Há quatro mecanismos distintos:

| Mecanismo | Identidade/vida | Falta ou invalidação |
| --- | --- | --- |
| Nodes já publicados | store/generation/source/context/recipe pinados | não gerar outra resposta sobre o mesmo NodeRef imutável |
| Prompt cache remoto | prefixo **serializado** pelo adapter, modelo/config/tools/roles/blocos | miss/TTL/eviction só muda custo/latência; reenvio completo continua válido |
| Coordenação local de cache | namespace de acesso + fingerprint exato do prefixo/modelo/adapter | efêmera, reconstruível; não é cache de resposta nem confirmação de hit |
| Retrieval/rerank | corpus/profile/generation/scope + query/recipe | invalidar geração e revalidar acesso; não confundir com cache de provas h(O) da10.2 |

Coordenador **single flight de aquecimento**, por prefixo: após autorização
de egress/orçamento e admissão à concorrência, imediatamente antes do send,
uma chamada lidera o prefixo frio. Outra que compartilha o prefixo ainda em
escrita espera até o evento em que o adapter considera o prefixo reutilizável
(upstream: `message_start` ou `response.output_item.added`), não até o fim da
geração. Usar o maior prefixo conhecido; um sufixo novo pode ganhar seu próprio
líder após aproveitar o prefixo quente. Erro/cancel antes desse evento remove
a liderança e acorda seguidores; timeout/cancel do seguidor não mata o líder.
Nenhum await de crédito, fila, lock SQL ou backoff acontece como líder. Usar
deadlines/fencing e relógio monotônico; cache frio ou coordenador offline não
bloqueia a correção do request.

Fingerprint LOGOS: SHA-256 de representação tipada dos campos realmente
enviados, incluindo endpoint/provider, namespace de credencial sem segredo,
domínio de acesso, modelo/revisão, opções que alteram cache, tools/schema em
ordem, system e prefixos de blocos/roles/imagens. Não copiar o hash Bun64 nem
o cache global entre usuários do upstream. Invalidar por mudança desses
campos/ACL; não por tarefa atual que está fora do prefixo. Permissões atuais
sempre são conferidas, inclusive ao reutilizar resultado/local state.

O adapter separa identidade de conteúdo e **diretivas de cache**:
`cache_boundary`/`cache_control` escolhem quais fronteiras oferecem chaves,
mas não entram no hash do conteúdo de um prefixo. Assim, mover a marca para
o bloco seguinte conserva a chave do anterior. Campos fora do input, como
teto de output/streaming, só entram se o contrato do provider disser que
alteram identidade de cache; não hashear o JSON inteiro indiscriminadamente.
Guardar trace do wire e da projeção usada para conferir essa correspondência.

Adapters declaram suporte/minimum tokens/TTL estimado/limite de breakpoints,
lookback e evento de disponibilidade; verificar o wire atual e registrar sua
versão antes de ativar. Valores 300s/1800s e campos de API de `AskAI`/adapters
são observações do commit auditado, não contrato de provider do LOGOS. No
perfil simulado Anthropic do upstream, o total permitido é quatro marcas:
system + view + fim do input/tail + marcador automático. Uma marca só na view
deixa espaço; tools estáveis vêm antes do system. Compactor sem tools e em
outro modelo tem cache próprio: mesmo preâmbulo não garante hits entre ele e
o agente. Adapter sem cache mantém as mesmas entradas e a mesma semântica.

### Settling e turno fresh

1. Receber inputs em inbox durável; terminar/capturar outputs do turno anterior.
2. Capture owner estabelece fronteira H; novas entradas ficam na inbox durante preparação, sem segurar transação SQL longa.
3. `settle(H)` garante coverage completa, nodes READY, geração consistente, frame dentro do budget e snapshot persistido. Cancelamento mantém inbox intacta.
4. Renderizar snapshot **antes** de admitir os inputs atuais no log.
5. Append atômico/idempotente dos inputs atuais; associar IDs ao turno, então iniciar chamada fresh.
6. Request: tools/system estáveis; bloco de dados Memory View; knowledge somente se router pediu; mensagem atual no canal de usuário; passos do turno no tail. Nada de session_id crescente entre turnos.
7. Capturar respostas observáveis, tool calls/results, mudanças e work com atribuição correta. Compactor roda em paralelo; não gera eventos recursivos para cada job.
8. Novos inputs/work durante a execução são entregues em fronteiras de tool como canais próprios; não são promovidos a user por texto. Próximo turno repete settle/render.

O proprietário serializa fronteiras de admissão; job completion pode atualizar nodes sem alterar H. Inputs não consumidos voltam à inbox sem duplicar eventos. Histórico original e inbox confirmada sobrevivem a cancel/crash; delivery é at-least-once com idempotência, não promessa impossível de exactly-once LLM.

A fronteira de admissão usa lease/fencing do capture owner. No commit de preparação, exigir head==H e fence vigente; se mudou, repetir snapshot/settle fora da transação, com deadline. Persistir associação turn_id→snapshot/inputs e estado de delivery no mesmo commit. Views antigas continuam pinadas: nunca reconstruir prefixo H dividindo uma view viva mais nova. Não há await de provider nem de compactor dentro de lock/transação.

Se o crash ocorreu após admissão e antes de delivery confirmado, retomar o
turn_id salvo com o mesmo snapshot histórico e os mesmos inputs; não preparar
snapshot novo que já contenha esses inputs e os injete novamente como atuais.
Somente a associação durável de delivery autoriza essa retomada, nunca uma
busca por eventos user. Revalidar acesso/egress; assinatura histórica continua
sujeita às verificações atuais do gateway.

## 7. Navigation

`zoom(snapshot,node_id)` valida namespace, geração, alinhamento, bounds e permissão. Pai retorna exatamente os dois children completos vinculados no node; folha retorna **o EpisodicEvent original**, não seu summary. Conteúdo grande retorna envelope original + ArtifactReference integral e operação de leitura, sem afirmar que preview é o original. Referências permitem leitura integral ou ranges de artefato explicitamente identificados.

`memory_range(start,end,cursor)` usa intervalo half-open e pagina eventos inteiros; nunca corta um evento para caber num frame. `memory_neighbors(event,before,after)` usa IDs/ranges, não similaridade. `memory_date(node)` devolve datas registradas do primeiro/último evento e sinaliza clock skew; timestamps não determinam sequência.

`episodic_search(query)`/MCP memory_search usa full-text sobre originais/projeções e, opcionalmente, embeddings em geração separada. Retorna candidatos com EventId, node/range, método e versão do índice. Hits sobre summary são marcados derived e precisam de zoom/original para inspeção. Busca semântica não substitui árvore nem transforma decisão histórica em instrução atual.

## 8. Knowledge Memory

Registry de documentos/versões: PDFs, papers, livros, documentação/web, código/repos, datasets, notas/specs, arquivos e outputs persistidos. Fonte pode estar errada ou maliciosa. Disponibilidade/readiness do documento não é estado de claim.

document_id é identidade persistente; version cresce para esse documento. Mesmos bytes de fontes distintas podem deduplicar o blob, **não** sua proveniência. URL igual com bytes novos gera versão nova; não substitui bytes da versão antiga. Import repetido usa idempotency key; document_id fornecido precisa pertencer ao scope autorizado.

Preservar original em CAS da memória ou referência content-addressed externa explícita. Cópia local pinada é padrão. Fonte externa sem garantia de retenção fica external_availability=best_effort e não satisfaz teste de recuperação local garantida. Embedding nunca é representação única.

Knowledge Graph não entra na primeira extensão: registro/locators/chunks já resolvem descoberta e o DAG epistemológico existente resolve dependências justificativas. Sem ganho medido, outra projeção adiciona ambiguidade. Uma extensão futura só será aceita por ablation; edges terão source_ref, extraction recipe, timestamp e status **de extração**, nunca Verified inferido. Não há migração automática de relações para Fact.

## 9. Ingestion

Ingestão é tarefa isolada: parser/OCR sem execução de código importado, scripts/macros, expansão ilimitada ou rede implícita. Arquivos locais entram por staging/handle limitado; fetch remoto e chamadas pagas passam pela política/gateway existentes. Source URI não é permissão para SSRF.

Ordem: capturar bytes → CAS durável → manifest/proveniência/registry version append → extração/chunking derivado → índices. Crash entre CAS e registry deixa órfão, não documento READY sem original. Aprovação de ingestion não atesta significado.

Texto: chunking determinístico por recipe versionada, preservando offsets UTF-8 e limites de caracteres; pode respeitar parágrafos/seções. Overlap é declarado e deduplicado na seleção. Índice normalizado não modifica citação original. PDF/OCR: preservar original binário e extraction artifact com extractor/version; locators usam página/bounding boxes quando disponíveis e offsets na extração. OCR/paráfrase não são chamados byte-original.

Código/repos: snapshot imutável de commit e bytes, path/arquivo/range; nunca resolver silenciosamente no working tree atual. Archives/datasets: locators de membro/linha/coluna com limites de descompressão. Se não houver mapeamento confiável, retornar ExtractionMappingUnavailable, não inventar página ou offset.

## 10. Retrieval

Pipeline: análise de query não confiável → filtros de scope/metadados/versão → full-text/BM25 e dense em paralelo quando habilitados → fusion → rerank opcional → seleção com budget → resolução de source → resultado.

Filtrar autorização **antes** de procurar/rerankar/enviar textos ao provider, e conferir de novo antes de retornar. Não deduzir permissão de filtros sugeridos pelo LLM. Query analysis pode fornecer subqueries, não ampliar ACLs ou trocar document version.

Baseline: lexical obrigatório e vendor-independent; dense opcional. Fusion por Reciprocal Rank Fusion: `sum(weight_m/(60+rank_m))`, rank começa em 1, pesos/config fixados; ordenar por score e desempatar ChunkId. Recuperação repetida fixa snapshot, profiles e versões. Não comparar diretamente BM25, cosine e logits nem chamá-los confiança.

Perfil inicial de fusion: no máximo duas listas (uma lexical, uma dense), ranks 1–4096, pesos inteiros 0–65535 e k=60; ausência contribui zero. Expansões de query são agregadas por método antes da fusion conforme recipe. Frações e comparação usam u128 checked, com desempate ChunkId; qualquer ampliação dos limites tem de demonstrar os bounds. Determinismo refere-se à fusão de listas pinadas; ANN/modelo não determinístico guarda recipe/seed/outputs e não promete rerun idêntico.

Reranker reordena candidatos com source refs preservadas. Timeout/indisponibilidade: conservar ordem fusion e retornar degradation; não fingir que reranking ocorreu. Após resolução, chunks sem original íntegro são excluídos com falha estruturada, never substituídos por source live; buscar suplentes até candidate cap. Resultado inclui failures e incomplete flag, inclusive quando nenhum chunk resta.

Embeddings são projeções de input bytes e profile: provider/model/revision, dimensão, preprocessing e recipe. Mudança de modelo cria nova geração, com alias switch atômico após build; consulta nunca mistura dimensões/profiles. Sem dense, lexical continua. Se lexical também indisponível, oferecer lookup de documento/source ou scan estritamente limitado, não resultado “nenhum documento existe”.

## 11. Provenance

Todo resultado aponta document_id+version, source_id, hashes, chunk/extraction recipe e location. SourceReference não é PremiseRef, FactRef, PolicyRef ou Verified event. retrieval_score não altera status no ledger.

`original_text` tem representação declarada: ExactOriginalSlice para UTF-8 nativo, ExtractedSlice para PDF/OCR etc. Em ambos, hash/offsets conferem contra o artefato indicado; para ExtractedSlice o documento binário continua resolvível e o mapeamento não vira prova de fidelidade. Não converter normalização do índice em citação literal.

Integração argumentativa reutiliza §13/Etapa 6.6: atestador de ingestão atesta `ContentOf(s)=h(c)`/Origin, não verdade; extrator em quarentena propõe Says(s,φ)/Extracted; gerador/ledger conferem citabilidade/época. O snapshot de fonte/extração acompanha a proveniência, mas as dependências justificativas continuam extraídas do certificado (§4), não dos top-k retornados pelo RAG. Uma fonte assinada autentica seu emissor conforme política, não ganha verdade universal.

O termo Source corresponde à versão imutável `(store_id,source_id,DocumentKey,content_hash)`, nunca apenas URL/alias “latest”. Origin vem do atestador/canal autorizado; copiar documento externo para projeto interno ou recebê-lo em mensagem user não altera sua origem nem cria autorização humana assinada.

Refs históricos a Verified/Executed são resolvidos por Query do kernel na época corrente. Memória não revalida, esconde retratação ou restaura token expirado. PVCs seguem §7; documentos recuperados não são PVCs.

## 12. Memory Router

Política pequena no harness: none, episodic, knowledge ou both, com budgets independentes e justificativa de rota registrada. O modelo pode sugerir rota; executor aplica scope/budgets. Não consultar todas as fontes a cada turno.

| Pedido | Rota |
| --- | --- |
| “Como decidimos implementar isso?” | episodic |
| “Você lembra do bug que tivemos?” | episodic |
| “O que aquele PDF dizia?” | knowledge |
| “Procure nos papers que já importamos.” | knowledge |
| “Por que escolhemos esse algoritmo e quais fontes sustentavam a decisão?” | both |

Both liga EventId da decisão a SourceReferences/document versions pertinentes, sem fundir semânticas. Mudança de preferência é localizada no histórico e comparada às instruções atuais. “t=100 PostgreSQL; t=5000 correção SQLite” deve retornar a correção mais recente com ambos os originais acessíveis; não gera Policy nem autorização de ação.

Artefato de tool pode ser referenciado por evento e ingerido explicitamente no registry. Não se duplica blob nem se indexa automaticamente todo log/segredo. Registry guarda originating_event_refs; evento pode guardar document refs. Links bidirecionais não promovem confiança.

## 13. Integration With Existing LOGOS Components

| Componente existente | Extensão mínima |
| --- | --- |
| logos_agent / lifecycle §3 | fresh-turn, captura/inbox, router e budgets; mesmos PROPOSE/CERTIFY/ACTION_INTENT |
| logos_mcp | ferramentas read-only memory_zoom/search/range/neighbors/date, knowledge_search/document/source; sem funções que escrevam Fact/Verified ou executem gateway diretamente |
| logos_client / protocolo | cliente do socket auxiliar memory/v1; kernel protocol v1 e regra de citação intactos |
| logos-ledger::Cas | mesma implementação de bytes/publish/hash/limites, raiz de memória separada; staging existente para ingresso explícito no kernel |
| source/atestador/extrator | reutilizar ContentOf/Origin, Says e Extracted; sem novo atestador que declare texto verdadeiro |
| Proof Ledger / AIR / seal | só refs verificadas pelos fluxos existentes; nenhuma variante lógica ou validade nova |
| gateway / Semantic Contract | contratos/adaptadores estreitos para fetch/provider e egress; mesmas assinatura, α(args), audiência, nonce, épocas e pré-condições |
| subagentes | snapshot settled no spawn e tarefa atual; relatórios kind=work com actor/job, não user; escopo de navegação não maior que o do pai |
| providers | interfaces Generate/Embed/Rerank com budgets/identidades; isolamento/credenciais no adaptador autorizado, nunca no Python/compactor; sem fornecedor obrigatório |
| CLI Rust | comandos auxiliares de memória/documentos abaixo; não reconstruir CLI de operação |
| TUI Rust | painéis Timeline, zoom/original, Sources e saúde; separado do DAG de claims e da fila de autorização |

Protocolos auxiliares: Unix sockets locais autenticados pelo SO, frames `u32 len || canon/v1`, envelope `(version,request_id,operation,payload)`, limites de frame/rate/output e rejeição de campos/tags desconhecidos. namespaces Memory, Knowledge, Artifact; schemas/paginação distintos dos comandos kernel. Mutation interna do log/registry pertence ao capture owner/ingestion autorizado; tools do LLM só leem ou propõem note/ingest via harness. Notes de agente nunca são reclassificadas como user/system.

Comandos adicionais: `logos memory status|view|zoom|search|range|date`, `logos memory export --originals`, `logos knowledge ingest|list|search|show|source|rebuild-index`, `logos memory rebuild-tree --new-generation`. Imports/rebuilds são explícitos, idempotentes, com scope/profile; não há comando padrão para destruir originais. Operações externas propostas pela CLI seguem o mesmo gateway.

TUI mostra Derived/Free/Locator, bytes/range, provenance/location e degradations; não usa selo verde para similarity/readiness. Links ao ledger chamam o renderizador existente com Seal inteiro e estado atual. Não reformata paráfrase assinável nem assina pelo usuário. Navegação de memória é separada da confirmação de efeitos.

## 14. Security / Trust Boundary

Não confiáveis para autoridade epistemológica: summaries, compactor model, embeddings, vector search, reranker, extração KG, retrieved documents e outputs RAG; também router, parsing/OCR e query expansion.

Mecanismos, não apenas aviso no prompt:

- transporte tipado preserva origin/role/taint fora do texto; dados recuperados nunca são carregados como system/developer/current user;
- encoder do provider recebe blocos históricos/documentais como dados escapados/tool results, sem interpolar conteúdo em templates/instruções; fake XML/JSON/roles não mudam envelopes;
- mensagens atuais vêm da inbox de ingresso, não de uma busca por kind=user;
- compactor/parser têm sandbox sem tools, credenciais, execução importada ou rede implícita; headers/URI de documentos não controlam fetch;
- scope/ACL é aplicado pelo broker de acesso independente de ranking/LLM; caches e jobs são segregados por store/scope/profile; egress para provider exige política existente;
- refs/ranges/hashes são validados antes de abrir dados; agente não fornece path arbitrário ao CAS;
- memória não tem chave do kernel, write capability do ledger, acesso ao seu DB/CAS ou via alternativa ao gateway;
- históricos de autorização assinada nunca são replayados como autorização atual: validade/audiência/nonce/precondições continuam checados;
- limites de upload, parsing/decompressão, jobs, duração, frame, tokens/custo e retenção impedem amplificação não limitada.

Retenção/pin de snapshot não concede acesso: consultas, zoom, artifacts, exports, caches e egress revalidam autorização vigente por principal. Revogação posterior invalida acesso mesmo a snapshot antigo. O adaptador autorizado confere destino/modelo e refs exatas do payload antes de enviar; não aceita SourceReference ou labels do LLM como prova de permissão.

A captura conserva restrições de origem, não só texto: registrar em store
menos restritivo é recusado pelo broker. Summaries, views e outputs herdam
restrições de todos os inputs/context refs; compressão não desclassifica
dados. Links cross-store não copiam corpos sem autorização.

“IGNORE ALL PREVIOUS INSTRUCTIONS” permanece citação histórica/documental. L3 continua: um LLM pode ser influenciado e escolher entre ações justificáveis. A extensão não promete imunidade cognitiva; a segurança de efeitos permanece integridade de justificativa (§13).

## 15. Persistence

| Dado | Persistência | Fonte ou projeção |
| --- | --- | --- |
| inbox + receipts/delivery | memory.sqlite + CAS de memória, commit durável | recepção original até admissão |
| EpisodicEvent | linhas append-only com canon bytes/hash + blobs pinados | histórico original |
| SummaryNode | linhas imutáveis por geração + input/context refs | derivado; não descartado se snapshot o referencia |
| MemoryViewPart/snapshots | commit atômico de revisão/parts/watermark | projeção com estado incremental persistido |
| ConversationView/ContextView working state | parts, size, high/low, batch, frontier, omission ranges e geração | estados distintos; cache remoto não é backup de nenhum deles |
| Prefix flights/TTL estimado | somente memória privada do adapter, namespace de acesso | otimização efêmera; restart pode aumentar custo sem perder história |
| CompactionJob/attempts | leases/receitas/result refs no DB | trabalho retomável/diagnóstico |
| KnowledgeDocument versions/source manifests | registry append-only + originais pinados | documentos canônicos para recuperação, não fatos verdadeiros |
| extraction/chunks/manifests | CAS/DB por recipe/generation; pins de resultados/snapshots | derivados reproduzíveis ou snapshots persistidos se não determinísticos |
| EmbeddingRecord/vector bytes | geração/manifest por profile | derivado, reconstruível |
| FTS/vector indexes/rerank caches | namespaces/generations separados | descartáveis e reconstruíveis |
| metadata/index cursors | DB, declarativos ou derivados conforme campo | não confundir live pointer com original |

Tabelas próprias mínimas: `inbox`, `receipts`, `turns`, `episodic_events`,
`admission_journal`, `summary_nodes`, `compaction_jobs`, `memory_views`,
`memory_view_parts`, `document_versions`, `source_records`,
`extraction_manifests`, `chunk_manifests`, `embedding_records`,
`index_generations` e `artifact_pins`. Uniques: receipt por canal/store,
evento por store/seq, node por store/start/count/generation, documento por
store/id/version, chunk por digest e job por target/generation/input_digest.
Foreign keys/pins são validados antes de publicar. Um writer por store,
SQLite WAL+FULL; nenhum consumidor escreve diretamente essas tabelas.

memory.sqlite não é ledger.sqlite. Reusar CAS com namespace/raiz de escrita limitado; logs/documentos podem compartilhar blobs dentro da memória, sem dar write ao CAS do kernel. Novo blob genérico necessita domínio **novo**, não certificate/checker/event. Proposta: `logos/artifact/v1\0` raw e `logos/artifact-manifest/v1\0` canonical; hashes canônicos de eventos/nodes/documents/chunks/snapshots têm tags próprias, nunca `logos/event/v1` do ledger. São adições normativas versionadas, não aceitas hoje por cas/v1.

Objeto cabe no limite CAS configurado; para conteúdo maior, ArtifactReference pode apontar manifest de segmentos completos de tamanho bounded, mantendo digest dos bytes lógicos concatenados e total_length. Resolver valida ordem/completude/digests/limites antes de declarar original íntegro. Manifests são pinados, acíclicos e de profundidade/part-count bounded; biblioteca CAS existente continua operando objetos bounded, sem nova API de leitura ilimitada. Rejeitar antes de ack se não há armazenamento/permissão/orçamento; nunca guardar só o início do original.

Índices são reconstruídos de originals + manifests/recipes versionados. Reexecutar OCR/LLM não promete texto idêntico: conservar extrações pinadas de referências emitidas; extração nova recebe geração nova. Build shadow/verify/switch atômico; queries pinam geração antiga até terminar. GC não apaga roots de evento/documento, segmentos, snapshots ou refs emitidas ainda retidas. Backups incluem DB, WAL/checkpoint consistente e todos os blobs pinados.

GC e append compartilham coordenação de pins: blobs em staging têm lease/pin até o commit ou abandono confirmado. Nunca remover um blob entre put e publicação SQL; se não há coordenação segura, GC destrutivo fica desabilitado. Erro CAS após rename não autoriza commit: pode deixar órfão, mas somente put confirmado satisfaz durabilidade. Restart recupera cauda por journal append-only de admissões, sem refazer folds históricos.

## 16. Failure Recovery

| Falha | Comportamento obrigatório |
| --- | --- |
| Crash durante append | put pode deixar órfão; transação incompleta não publica evento/seq. Receipt comprometido é recuperado/idempotente. Após commit, retry retorna mesmo EventId |
| Compaction interrompida | lease expira; retomar input/context congelados. Publish compare-if-absent evita substituir node existente |
| Node ausente | bloquear inclusão do node, agendar children/leaf na mesma geração; detectar corrupção de ref, não fabricar children |
| Summary inválido/oversized | retry bounded com feedback, diagnóstico e Locator explícito ou cancelamento; nenhum corte publicado |
| Provider embedding indisponível | lexical + source lookup; dense unavailable explícito |
| Index corrompido | retirar geração, usar caminho disponível e reconstruir shadow a partir das fontes; nunca tratar index vazio como corpus vazio |
| Reranker indisponível | retornar fusion não reranked com degradation |
| Original documental faltando | SourceMissing/IntegrityError, exclude com failure de resultado; não servir chunk stale como source recuperado nem buscar URL atual sob hash antigo |
| Original episódico faltando | HistoryIntegrityError; recuperar backup por digest. Navegação não promete original inexistente; modo normal não afirma M6 satisfeita |
| Vector DB indisponível | lexical; fontes/registry intactos, não migrar verdade para cache |
| Embedding model mudou | nova geração/model-space; validar dimensão/recipe/revision antes do switch |
| LLM principal mudou | conservar log/tree/view; chamada fresh pelo novo adapter. Cache pode perder hit. Se contexto não cabe, ContextBudgetMismatch e ajuste explícito ou turno sem memória sinalizado, nunca corte silencioso |
| SQLite/CAS indisponível | não confirmar append/ingest sem durabilidade; leitura de snapshot previamente íntegro só se verificável; inputs ficam pendentes |
| Cancel durante settle/provider | não começar turno histórico incompleto; inbox/originais preservados. Cold turn sem memória requer opção explícita e rótulo, não finge cobertura |

M1/M6 pressupõem armazenamento e backups sob as premissas do CAS. Deleção hostil/hardware loss não é solucionado por summaries. Modo degradado informa perda/indisponibilidade, não transforma aproximação em prova de recuperação.

## 17. Observability

Reusar telemetria da Etapa 10.8: namespace memory.*, métricas/JSONL sem conteúdo privado por padrão.

Episodic: event_count, tree_node_count/depth, pending/leasing jobs, frontier_lag, view_bytes/parts/resolution_distribution, compaction_latency/calls/retries/cost/cache_hit/free_ratio/locator_ratio, settle_latency/cancels, zoom_depth/frequency, search_hit_rate/original_resolution_failures, inbox_lag e recovery incidents.

Cache/folding: high/low/batch e tamanho de cada view, lotes/revisões publicados,
primeiro bloco alterado/prefixo comum em bytes, tokens de input/read/write
**reportados pelo provider** separados por turn/compactor/modelo, tarefa/tail
incluídos e excluídos em métricas distintas, cold/TTL/eviction/config change,
leaders/followers/wait_ms/abort/timeout e custo de escrita/leitura. Contagem de
bytes/UTF-16 de simulador não é token nem desconto monetário garantido.

Knowledge: document/version/chunk counts, original/extraction/index/vector bytes, ingestion/index/rebuild latency, retrieval p50/p95/p99, reranking latency, source/method distribution, source_resolution_failures, degraded_query_ratio, egress/cost e generation/model identifiers.

Não medir “truth score” com cosine ou usar cache-hit como qualidade de memória. Contadores e spans não se autoingerem como eventos a cada job, evitando feedback infinito. Registros de incidente relevantes podem ser admitidos como system_event pelo capture owner.

## 18. Repository Changes

Apenas adições necessárias, sem reorganizar o repo:

```text
crates/logos-memory/          serviço local Rust; fora da TCB de soundness
  src/ event.rs tree.rs view.rs store.rs artifact.rs protocol.rs knowledge.rs
py/logos_agent/memory/
  compactor.py router.py providers.py
  knowledge/ ingestion.py chunking.py lexical.py vectors.py rerank.py retrieval.py
py/logos_client/              cliente do protocolo auxiliar
py/logos_mcp/                 ferramentas read-only adicionais
py/logos_eval/                benchmarks/ablations desta extensão
crates/logos-cli/             subcomandos memory/knowledge
crates/logos-tui/             painéis auxiliares, sem novo renderizador de Seal
spec/memory.md               futuro encoding/protocolo normativo memory/v1
spec/knowledge.md            futuro encoding/protocolo normativo knowledge/v1
spec/vectors/memory/         futuros vetores independentes
spec/vectors/knowledge/      futuros vetores independentes
```

CAS permanece em logos-ledger; dois novos domínios e seus limites requerem ADR, atualização aditiva/versionada de spec/domains.md e spec/cas.md e vetores, sem mudar endereços existentes. Mudança em módulos TCB existentes exige manifesto/fuzz/lints conforme regras atuais. O serviço de memória não entra automaticamente na propriedade correctness. Dependências novas, inclusive fora da TCB, devem respeitar deny.toml e decisões aprovadas.

Protocolos/CLI/TUI/providers e handlers futuros do kernel são dependências de integração, não justificativa para criar substitutos nesta extensão. A trilha proposta está no ROADMAP Apêndice F; adoção/formatos são gate MEM.0. Nenhum checkmark de implementação nasce desta documentação.

## 19. Data Schemas

Tipos de contrato, não código de implementação. Envelope canônico `[format_version, type_tag, record]`; campos ASCII fixos, enums discriminados e unknown fields rejeitados. Aplicar limites de bytes/arrays/metadata/recursão por operação. Bytes originais permanecem Bytes; identificadores, labels e campos textuais canônicos seguem canon/v1. Hash = SHA-256(tag || canon(record)) ou tag || bytes raw conforme domínio, com separação de tipos.

Digest de um record exclui seus campos de identidade derivados: digest/chunk_id e o índice overlap_with não entram no payload hasheado. IDs externos são atribuídos ou derivados de payload acíclico; não se define hash de um objeto contendo seu próprio hash. Overlap usa ordinais/ranges no manifest, e IDs bidirecionais são projeção reconstruível. Os type_tags de encoding serão fixados nos vetores normativos antes de implementação; não se reutilizam tags do protocolo kernel.

```text
EventId           = { store_id: Id128, seq: u64 }
NodeId            = { store_id: Id128, start: u64, count: u64 }
NodeRef           = { node_id, generation: ProfileId, digest: Hash<SummaryNode> }
DocumentKey       = { store_id: Id128, document_id: Id128, version: u64 }
Time              = i64 UTC milliseconds; nunca sequenciador
Metadata          = mapa limitado de valores canon/v1, sem floats/credenciais

ArtifactReference = {
  store_id, content_hash: Hash<ArtifactBytes>, byte_length: u64, mime_type,
  storage_ref: Blob(hash) | Segments(manifest_hash), scope_id,
  source_id?, availability: LocalPinned | ExternalBestEffort
}

EpisodicEvent = {
  format_version, id: EventId, kind, actor, timestamp: Time,
  content: Inline(Bytes) | Artifact(ArtifactReference), byte_length,
  metadata, artifact_refs: [ArtifactReference], correction_of: [EventId],
  related_documents: [DocumentKey], receipt_id, digest
}

SummaryNode = {
  format_version, id: NodeId, generation, digest,
  source: Leaf(event_id,event_digest) | Pair(left:NodeRef,right:NodeRef),
  text_utf8: Bytes, text_byte_length, mode: Free|Model|Locator,
  source_coverage: FullTarget|ArtifactDescriptorOnly,
  context_snapshot_ref?, context_digest?, recipe_ref,
  degradation?, created_at
}

MemoryViewPart = { node_ref: NodeRef }  # start/count derivam da ref
WorkingViewPart = PendingNode(NodeId,generation) | ReadyPart(MemoryViewPart)
MemorySnapshot = {
  snapshot_id, store_id, generation, revision, represented_end: u64,
  parts: [MemoryViewPart], renderer_version, cache_block_lines,
  rendered_bytes, digest,
  degradation: [Code]
}
# WorkingViewPart é estado privado do writer; MemorySnapshot só contém ReadyPart.

WorkingView = {
  store_id, generation, kind: Conversation|Context, revision, represented_end,
  parts: [WorkingViewPart], high, low, batch: bool, renderer_version,
  cache_block_lines, omitted_ranges  # vazio em Conversation; permitido em Context
}
ProviderBlock = { role, text_utf8: Bytes, cache_boundary: bool }
# ProviderBlock é contrato interno; não é novo formato aceito pelo kernel.

CompactionJob = {
  job_id, target: NodeId, generation, immutable_source_refs,
  context_snapshot_ref, input_digest, policy_ref, provider_profile,
  state: Queued|Leased|Retryable|Ready|Failed|Cancelled,
  lease_owner?, lease_until?, attempt_token?, retry_at?, attempts: u32, max_attempts,
  deadline, frozen_block_refs, byte_feedback?, candidate_responses,
  transport_attempts, best_attempt_ref?, result_node_ref?, last_error?
}

KnowledgeDocument = {
  format_version, key: DocumentKey, content_hash, source_id, source,
  title, author?, created_at?, ingested_at: Time, mime_type,
  metadata, provenance: SourceReference, storage_ref: ArtifactReference,
  previous_version?, originating_events: [EventId], receipt_id
}

SourceReference = {
  source_id, document: DocumentKey, original_ref: ArtifactReference,
  original_content_hash, locator,
  extraction_ref?, extraction_recipe?, scope_id,
  representation: OriginalDocument|ExactOriginalSlice|ExtractedSlice,
  observation_provenance_ref?, attestation_ref?
} # attestation_ref é conferida pelo fluxo existente, nunca pelo score

Locator = WholeDocument
        | Utf8Bytes(start,end)
        | Pdf(page,boxes?,extraction_ref,text_start,text_end)
        | Repo(commit,path_bytes,file_ref,start,end)
        | ArchiveMember(path_bytes,member_ref,start,end)
        | Table(member_ref,row_ids,column_ids,extraction_ref?)

KnowledgeChunk = {
  chunk_id: Hash<KnowledgeChunk>, source_ref: SourceReference,
  extraction_generation, chunk_recipe, ordinal: u64,
  text_utf8: Bytes, text_hash, byte_length,
  representation: ExactOriginalSlice|ExtractedSlice,
  overlap_with: [ChunkId], scope_id
}

EmbeddingRecord = {
  chunk_id, input_hash, profile_id, provider_id, model_id, model_revision,
  preprocessing_version, dimension: u32, encoding: IEEE754_f32_le,
  vector_ref: ArtifactReference, generation, created_at
}

Score = { value: i64, scale: u64 } # scale>0; fixed-point, não probabilidade
RetrievalResult = {
  query_id, snapshot_id, document_id, document_version, chunk_id,
  source_ref: SourceReference, source, location: Locator,
  content_hash, chunk_text_hash, original_text: Bytes, representation,
  retrieval_score?: Score, retrieval_method: [Lexical|Dense|RRF|Reranker],
  ranks, profile_refs, degradation: [Code]
}
RetrievalResponse = { results: [RetrievalResult], failures, incomplete, cursor? }

KnowledgeSnapshot = {
  snapshot_id, store_id, registry_revision, membership_manifest_ref,
  extraction_manifests, lexical_generation?, dense_generation?,
  profile_refs, coverage_manifest_ref, created_at
}
IndexManifest = {
  generation, store_id, membership_manifest_ref, profile_refs, input_digest,
  state: Building|Ready|Quarantined, coverage, output_refs, failures
}
```

Embeddings são bytes IEEE754 derivados porque canon/v1 não admite floats. Conferir tamanho=4*dimension, valores finitos, dimensão/profile; vetor zero é recusado para cosine, nunca normalizado a NaN. Scores externos finitos são quantizados round-to-nearest/ties-even com escala explícita e overflow rejeitado; ranking/fusion usam ranks/racionais antes dessa projeção. Campo ausente por falha não é score zero. SourceReference/ArtifactReference não carregam Truth/Verified.

Offsets/ranges são half-open em bytes da representação indicada, com fronteiras
UTF-8 validadas quando se devolve texto. PDF page começa em 1; boxes usam
coordenadas i64 em micropoints, e não floats canônicos. WholeDocument permite
proveniência do registry antes de haver chunks. Toda ref inclui ou deriva um
store inequívoco; referência de outro store exige autorização explícita, não
somente igualdade de hash.

## 20. Algorithms

Primitivas abaixo têm efeitos definidos: `put_original` confirma bytes completos/pinados; `commit` é SQL durável; `publish_once` valida schema/hash/refs e é compare-if-absent; `record_error` não altera status epistêmico. H é o cutoff fixado, não tip móvel. Operações checked falham antes de publicar.

O corpo transacional de append é compartilhado com prepare_turn, sem BEGIN
aninhado. A preparação admite todos os inputs e a associação ao snapshot em
um único commit; CAS/pins provisórios são preparados antes da transação.

### Histórico, árvore e compactor

```text
append_event(receipt):
  require authorized_capture_channel(receipt)
  original = put_original(receipt.full_content)  # pode usar segmentos
  begin immediate transaction
  if receipt.key exists:
    require same canonical receipt input digest (bytes, actor, metadata)
    end transaction without mutation; return committed event id
  require next_seq < MAX_EVENTS
  e = validate_event(id=(store,next_seq), receipt, original)
  insert immutable e; pin all referenced originals
  append_to_view(e.id) within this transaction; journal admission and pending leaf
  associate receipt -> e.id; increment next_seq; commit
  notify tail_available; enqueue recoverable leaf job
  return e.id

build_leaf(event_id,g):
  e = load_and_validate_original_event(event_id)
  source = full_kind_prefixed_text(e) if renderable_text else full_artifact_descriptor(e)
  if source_is_literal_complete(e) and fits_utf8_node(source):
    return publish_once(Leaf(e), mode=Free, text=source, generation=g)
  return enqueue_job(Leaf(e), immutable_source=e, generation=g)

build_parent(left,right,g):
  require READY refs; same store/g; count equal
  require left.start % (2*left.count) == 0
  require right.start == left.start + left.count
  text = left.text + '\n' + right.text
  if fits_utf8_node(text): return publish_once(Pair(left,right), Free, text)
  return enqueue_job(Pair(left,right), generation=g)

compact(job):
  context,target = resolve_frozen_inputs(job)  # fonte íntegra ou descriptor declarado
  if not provider_input_fits(context,target): return explicit_locator_or_failure(job)
  best = load best prior attempt if present
  prior_byte_feedback = load prior feedback if present, else none
  while job.attempts < job.max_attempts and job.candidate_responses < COMPACTION_RETRIES:
    if cancelled_or_deadline(): break
    persist incremented attempts + claimed attempt_token before provider call
    reply = generate_without_tools(context, target, budget, prior_byte_feedback)
    if provider_error(reply):
      increment transport_attempts; persist retry_at/error
      release execution slot; return Retryable if under caps else explicit_failure
    persist incremented candidate_responses  # inválida/vazia também consome candidato
    text = trim_outer_whitespace(reply)
    if not valid_summary_text(text): record invalid attempt; continue
    best = smaller_utf8_then_earlier(best,text); persist best diagnostic ref
    if utf8_bytes(text) <= NODE_SIZE: return text with mode=Model
    prior_byte_feedback = {actual:utf8_bytes(text), limit:NODE_SIZE,
                           diagnostic_prefix:utf8_safe_bounded_prefix(text)}
  persist best as restricted diagnostic if present
  return explicit_locator_or_failure(job)

pump_compactor():
  while active_jobs < COMPACTION_JOBS:
    f = first committed event without READY leaf in this generation, else H
    window_end = COMPACTION_JOBS-th pending leaf's start, else H
    candidates = unpublished source-ready targets with ctx(target) <= window_end
                 and no live lease/backoff/blocked access
    j = minimum (ctx(target), -level(target)) among candidates
    if no j: return
    ctx = reuse frozen context or whole_ready_parts(context_view,
          cutoff=min(ctx(j),f), no_refinement=true, omissions_explicit=true)
    lease j with unique attempt_token, frozen inputs/context and deadline
    start/resume compact(j) outside locks
  on completion:
    require current generation/job/attempt_token and valid lease/fence
    validate result; publish_once or persist Retryable/Failed
    release execution slot even during backoff
    enqueue source-ready sibling parent, idempotently
    increment notification_version; notify fit/settle; pump_compactor()
```

Folhas futuras Free podem ser publicadas cedo; a frontier continua no primeiro gap. Merges independem da prontidão de eventos posteriores. Jobs não aguardam o próprio target para construir contexto. Cada parent é enqueued uma vez por geração. Contexto limitado nunca inicia novos jobs recursivamente enquanto segura lease.

### View, settle e navegação

```text
append_to_view(event_id):  # dentro da transação do append, sem espera/model call
  require event_id.seq == working_view.end
  append part for leaf(event_id,current_generation)  # interna, talvez pending
  working_view.end += 1
  persist admission journal/working parts revision

fit_view(view,H):  # Conversation; Context tem projeção/omissões próprias de E§5
  require view partitions [0,H)
  if any part is not READY:
    schedule required leaf/ancestors; return Pending
  if exact_rendered_bytes(view) > view.high: persist view.batch = true
  target = view.low if view.batch else view.high
  while exact_rendered_bytes(view) > target:
    eligible = adjacent siblings with READY parent pinning these exact child refs
    if no eligible:
      schedule missing parents of sibling pairs
      return Pending if such parents exist else ConfigurationOrIntegrityError
    pair = max rational (H+1-start)/child_count, tie smallest start
    replace pair with parent; persist private owner revision atomically
    # parent pode aumentar bytes, mas reduz parts; não exigir economia imediata
  view.batch = false
  validate geometry/generation; atomically persist pinned snapshot + working state
  return Ready(snapshot)

settle(H,cancel):
  candidate = immutable owner/revision with exact persisted coverage [0,H)
  repeat:
    v = notification_version  # observar antes do check evita lost wakeup
    if cancel/deadline: return CancelledOrUnavailable
    r = fit_view(candidate,H)
    if r is Ready: return r.snapshot
    if r is integrity/configuration error or required job terminal Failed: return error
    pump_compactor()
    await notification_version != v or next retry/lease expiry/deadline/cancel

prepare_turn(receipts):
  persist receipts/inbox before acknowledging reception
  old = lookup admitted turn by exact ordered receipt group/digests
  if old and old.delivery == Delivered: return authorized saved delivery result/ref
  if old:
    atomically claim fenced delivery lease; revalidate current read/egress
    frame = render(old.pinned_snapshot)
    return start fresh call(frame,authorized knowledge,old.current_input_refs)
    # no append, no new historical snapshot; delivery remains at-least-once
  acquire capture-owner admission lease; freeze H and revision
  snapshot = settle(H,cancel); frame = render(snapshot)
  begin immediate transaction
  require current head==H and capture-owner fence still current
  append current receipts idempotently; persist turn->snapshot/event_ids/delivery
  commit; release admission barrier
  start fresh model call with frame, authorized routed knowledge, current inputs
  # head/fence mismatch: rollback/retry snapshot with deadline, never reuse stale frame

render(snapshot):
  validate pinned snapshot digest, geometry, renderer profile and current permission
  frame = one bounded output buffer(VIEW_SIZE); write constant header into frame
  for part in chronological order:
    node = load READY node by exact pinned ref; validate this record hash/child-ref geometry
    require utf8_bytes(node.text) <= NODE_SIZE and valid_summary_text(node.text)
    write_ascii_range_into(frame,part.start,part.count)
    percent_escape_into(frame,node.text); write newline into frame
  write bounded trailer into frame  # volatile metadata never prepends history
  require exact utf8_bytes(frame) <= VIEW_SIZE else ContextBudgetMismatch
  return owned immutable frame bytes without a second full-frame copy

provider_blocks(snapshot):
  frame = render(snapshot)
  # dividir sem alterar bytes; quatro linhas inteiras por bloco, sem reflow
  blocks = [constant_header] + full_four_line_blocks + [remaining_lines_and_close]
  mark only last full block, else header; never mark memory's partial tail
  require concatenation(blocks.text) == frame
  return immutable blocks

send_provider(request,cancel):
  revalidate current read + destination/egress + budget; obtain execution slot
  wire = adapter.serialize(request)  # stable tools/system/view, then volatile tail
  flight = coordinator.claim_or_wait(exact_prefix_keys(wire), deadline, cancel)
  try:
    send wire; on adapter.cache_available_event: flight.begun()
    capture response and actual usage normally  # every task generates its own answer
  finally: flight.end(); release slot
  # no task result reuse; cold/miss/cancel cannot change node/claim authority

get_event(snapshot,event_id):
  require same store, 0 <= seq < snapshot.H and current read permission
  e = load immutable event; validate canonical event digest and original refs
  resolve logical original digest/length or return HistoryIntegrityError
  return original envelope with complete inline bytes or full ArtifactReference
  # transport may use an exact-content artifact handle; stored e is not rewritten
  # bounded range reads are explicit; preview is never labelled complete original

zoom(snapshot,node_id):
  require snapshot scope/generation and aligned in-bounds node_id
  n = load validated immutable node for that snapshot generation
  if n.count == 1: return get_event(snapshot,n.source.event_id)
  require exactly two valid pinned children
  return both child descriptors/summaries, with original ranges

episodic_search(query,scope,snapshot,budget):
  enforce scope before index access
  candidates = lexical original hits + optional same-profile dense hits
  fuse ranks; filter committed in-bounds EventIds
  return bounded candidates with origin kind, hit_kind, refs and degradation
  # summary hit requires zoom; score never changes authority
```

Prontidão e budget são portas distintas: nenhuma redução de resolução gratuita para esconder pending. Leases expirados/callbacks antigos não publicam no lugar de uma tentativa nova. Wakeups são versionados; waiters não dependem de polling ou de um H móvel.

### Knowledge pipeline

```text
ingest_document(input,scope,receipt,recipe):
  require authorized ingress; enforce upload/parser/egress limits
  original = put_original(input.complete_bytes_or_verified_content_ref)
  source = capture provenance, source URI and observed metadata
  begin transaction
  if receipt already committed:
    require identical capture input digest (content,metadata,scope,recipe)
    end transaction; return prior document key and processing state
  allocate document_id/version with checked arithmetic
  append registry version + original/source manifest; pin original
  increment registry_revision; enqueue recoverable ingestion job with frozen recipe
  commit
  worker chunk_document(document_version,recipe), preserving failures as processing state
  persist immutable extraction/chunk manifests; enqueue index_document(chunks,profiles)
  return document key with availability/index state, not Verified

chunk_document(document,recipe):
  original = resolve pinned original and verify logical digest
  if native UTF8: segments = deterministic boundaries on original bytes
  else: extraction = sandbox_extract(original,recipe)
        persist extraction artifact and location map
        segments = deterministic boundaries on extraction bytes
  for complete segment in order:
    require character boundaries, locator validity and recipe limits
    emit immutable chunk with source ref, exact text hash and representation
  return manifest/chunks  # não normalizar original nem inventar PDF offsets

index_document(chunks,profiles):
  validate chunk/source manifests
  build lexical projection in shadow generation
  if dense enabled:
    for chunk: authorize embedding egress; generate finite compatible vector
               persist EmbeddingRecord or explicit provider failure
    build vector generation from valid records only
  verify counts, source keys and profile consistency
  publish available generations only with current build attempt/fence and pinned input digest
  atomically switch generation aliases; publish exact coverage/degradation manifest

retrieve(query,filters,budget,snapshot):
  allowed = authorization_scope intersect permitted filters
  subqueries = bounded_query_analysis(query)  # não amplia allowed
  lexical = lexical_search(subqueries,allowed,snapshot) or explicit failure
  dense = compatible_dense_search(subqueries,allowed,snapshot) or explicit failure
  candidates = deterministic_RRF(available lists, configured weights, k=60)
  candidates = rerank(query,candidates,allowed,budget) if enabled else candidates
  output=[]; failures=[]
  for candidate until candidate cap/result byte budget:
    recheck permission
    chunk = load canonical chunk by id from snapshot membership manifest
    validate chunk digest, source ref and representation; ignore index-supplied replacements
    r = resolve_source(chunk.source_ref,snapshot)
    if r failed or slice hash != chunk.text_hash: record failure; continue
    append complete RetrievalResult from resolved source/chunk bytes
  return output + failures + incomplete/degradation, never hidden empty success

rerank(query,candidates,scope,budget):
  freeze refs; enforce candidate/text/time/cost/egress budget
  scores = optional_reranker(query,authorized_candidate_texts)
  if unavailable/invalid: return original fusion order with degradation
  validate scores finite and refs unchanged
  return stable order(scores descending, chunk_id tie), preserving provenance

resolve_source(ref,snapshot):
  require authorized ref and exact document version in registry snapshot
  original = open pinned original or explicit external content-addressed source
  verify digest/length; failure -> SourceMissing/IntegrityError
  validate locator belongs to original/extraction manifest and byte bounds
  if OriginalDocument: return pinned original document handle with hash/length
  if ExactOriginalSlice: return exact original bytes at locator
  if ExtractedSlice: verify extraction bytes/hash; return exact extracted slice
                    plus original document handle + declared mapping
  never substitute latest URL/working tree for pinned version
```

Busca/index provider failures são estados de discovery, não Refuted/Unknown de obrigação. Serviços impõem budgets também às funções abstratas; oversize não é truncado sem tipo/offset/flag explícitos.

Eligibility deve ser indexada por escala/adjacência, não por rescan do log:
por escala, o menor start elegível maximiza due. Comparar esses heads mantém
a regra exata; invalidar heads após fold. O benchmark inclui custo do scheduler
e não apenas custo do LLM.

## 21. Invariants

**M1.** Eventos episódicos originais comprometidos são imutáveis; correção adiciona evento.

**M2.** Cada node mapeia um intervalo contíguo exato no mesmo store; alinhamento/aritmética/geração validados.

**M3.** Cada non-leaf tem exatamente dois children prontos, adjacentes, iguais em escala e vinculados por digest.

**M4.** Memory View cobre `[0,H)` do snapshot sem overlap/buraco, cronologicamente, com frame dentro do budget.

**M5.** Folding automático só apenda cauda ou diminui resolução; restart não afina regiões antigas.

**M6.** Todo evento comprometido é alcançável estruturalmente até sua folha/original sob as premissas de armazenamento/acesso; perda gera erro, não original inventado.

**M7.** Summary, incluindo Free/Locator, não cria autoridade epistêmica.

**M8.** Score, embedding ou rerank não cria autoridade epistêmica.

**M9.** Cada chunk vincula fonte original/versionada e locator verificável; extração é declarada como extração.

**M10.** Índices são projeções reconstruíveis; deletá-los não apaga originais/registry/manifests necessários.

**M11.** Memória episódica e documental têm semânticas distintas; links não as fundem.

**M12.** Instruções históricas/recuperadas não herdam autoridade de system/developer/current user.

**M13.** Entre lotes, blocos completos do prefixo conservam bytes/ordem/roles;
cache miss, restart ou coordenador indisponível não altera histórico nem
resultado epistêmico. Cache-hit real é medido, nunca inferido da tabela de TTL.

Adicionalmente: commit pressupõe refs duráveis; cancel mantém inputs; permissões não derivam de modelo; todos os passos que criam status/efeito continuam sob os invariantes existentes de §§3/4/6/12/13.

## 22. Acceptance Tests

Casos de implementação futura, não resultados alegados desta extensão:

1. **1M eventos:** após settle, frame UTF-8 inteiro <= VIEW_SIZE; union de ranges=[0,1000000); event #42 recuperável por descida e bytes iguais ao original. Validar todos os IDs estruturalmente, amostra de payloads e full sweep separado.
2. **Todos os originais:** append de texto não NFC, Unicode multibyte, binário e artefato segmentado; folhas/zoom devolvem bytes/digest exatos antes/depois de reboot. Nenhuma conversão NFC do original.
3. **Free:** texto literal que cabe e pai cuja concatenação+newline cabe fazem zero calls; um byte acima dispara compactor/Locator, sem corte. Verificar children ordenados.
4. **Folding:** snapshots sucessivos; cada old part só permanece ou está contida em part maior alinhada. Par de maior due e tie menor start escolhido; incluir o caso H=8 de E§6. Parent maior em bytes não causa loop/stall. Entre LOW e HIGH sem batch, nenhuma revisão histórica muda; exceder HIGH inicia batch até LOW e restart retoma seu estado. Perfil de 63 bits satisfaz F/LOW, sem exigir 64KB impossíveis.
5. **Byte enforcement:** modelo scriptado responde oversized cinco vezes, incluindo multibyte/control; nenhum oversized é publicado. Locator explicitamente marcado ou settle cancela. IDs/escapes/trailer contam no budget.
6. **Settling:** última folha/modelo lento; nenhuma chamada principal/subagente recebe placeholder/partial. Cancel preserva input e retry idempotente. Context do compactor nunca inclui pending ou o próprio target como pré-condição; pai pode ler children/descendentes já prontos até seu end, folha só vê anteriores. ContextView e ConversationView têm fronteiras independentes, persistidas; import/backoff não cria dependência circular.
7. **Crash append:** SIGKILL antes/depois de CAS put, begin, insert e commit; acknowledged receipt conserva evento exato/mesmo ID, sem buraco/reuso committed. Retry com conteúdo diferente falha.
8. **Crash jobs/view:** matar worker com lease, duplicar completion e reiniciar entre replace/persist; retomar sem node sobrescrito, coverage parcial ou expansão de view anterior.
9. **Sem autoridade:** summary inventa X/Verified/assinatura; proposta que cita NodeId/SourceRef como PremiseRef é recusada; nenhum evento Verified/token aparece sem o fluxo existente.
10. **Injection:** original/chunk contém fake role, XML closing tag, “IGNORE ALL PREVIOUS INSTRUCTIONS”, falso user/subagent report e autorização histórica expirada; origin/role do request e gate de efeitos permanecem intactos. Registrar influência residual, não prometer LLM imune.
11. **Fonte:** recuperar chunk UTF8 e PDF/OCR; resolve retorna versão/hashes/location corretos e rótulo ExactOriginalSlice/ExtractedSlice; missing/altered original produz erro, não texto de URL atual.
12. **Rebuild:** apagar FTS/vector indexes; originais/registry/pins não mudam. Rebuild determinístico recupera mesmos ChunkIds quando recipe/extraction pinada é a mesma; OCR novo recebe geração nova.
13. **Provider failures:** embed/rerank/vector DB offline separadamente; lexical/source lookup continua com degradation. Todos índices offline retorna indisponibilidade, não “corpus sem documentos”.
14. **Model spaces:** mudar embedding model/dimensão e LLM principal; sem mistura de profiles, sem perder eventos/view, alias atômico; ContextBudgetMismatch não resulta em corte.
15. **Correção de preferência:** t=100 PostgreSQL, t=5000 SQLite; responder decisão mais recente com refs aos dois originais; nenhuma preference vira Policy ou assinatura de ação.
16. **Continuidade referencial:** “faz daquele jeito”, “usa o anterior”, “volta pra solução que funcionou”; summary inclui antecedente quando disponível ou ambiguity explícita; zoom localiza o original, sem inventar implementação.
17. **Cross-memory:** decisão episódica ligada ao paper versionado; resposta distingue “decidimos” de “fonte afirma”, resolve ambas e usa fluxo Says/atestação para qualquer claim selada.
18. **Scope/egress:** tentativa de acessar outro store, falsificar actor, trocar URI por file path/SSRF ou mandar documento proibido a provider é recusada antes de search/rerank/egress; cache não vaza resultados entre scopes.
19. **Recuperação longa:** inserir dado cedo, aumentar H por ordens de magnitude, consultar depois; medir precisão/recall/zoom/custo, sem assumir que summary preservou todas as palavras.
20. **Fencing/backoff/turn replay:** job com uma vaga; transporte falha e libera vaga, tentativa antiga retorna após lease nova, callback é recusado. Ao atingir max_attempts não reinicia contador. Head/fence mudou durante settle: inputs continuam na inbox e nenhum turno recebe snapshot stale. Crash após admissão retoma snapshot salvo sem incluir o input atual no histórico; receipt já entregue retorna resultado salvo, sem novo append/model call.

21. **Retries bounded:** capturar cinco requests do provider scriptado. Cada um
    conserva exatamente os blocos iniciais; após o primeiro só uma nota
    substituída, de tamanho bounded, sem transcript/respostas/raciocínio
    acumulados. Recusa, vazio, oversized, outage e crédito/egress negado têm
    saídas distintas; nenhuma vira clipping silencioso. Free testa 512/513
    bytes, newline e UTF-8 multibyte, não número de caracteres.
22. **Cache wire:** 3/4/7/8/9 linhas, append, passagem de ID9→10, zoom e restart;
    conferir conteúdo/ordem dos blocos e posição única da marca da view.
    Mover a marca conserva chaves de prefixos anteriores; tail parcial e
    knowledge/tarefa não alteram o prefixo. Teto de saída segue o contrato de
    identidade declarado pelo adapter. Mutar tools,
    role, system, endpoint/modelo/adapter ou ACL muda namespace/fingerprint.
    Adapter sem cache envia o mesmo conteúdo. Simular lookback finito,
    TTL, limite de marcas, prefixo mínimo e eviction; não só igualdade de hash.
23. **Cache concorrente:** N compactações com prefixo frio compartilham um
    aquecimento; cada uma ainda recebe sua própria resposta. Liberar seguidores
    no evento de cache disponível, antes do fim da geração. Cobrir extensão
    de prefixo, falha do líder, cancel/timeout do seguidor, crédito/fila/yield
    antes da liderança, mudança de store/credencial/ACL e restart frio.
24. **Replay de custo:** dataset/seed/wire/relógio fixados; separar input/read/
    write/tarefa/tail e turn/compactor. Baseline sem lotes, bloco único,
    contexto refeito por job e sem single flight devem piorar as métricas
    correspondentes; falha de controle é reportada. >=95% é só referência do
    simulador upstream, não gate universal ou medição do LOGOS real.

Verificação estrutural desta proposta: runner descartável de árvore/view com
1.000.000 eventos, 1.999.993 nodes, 161 parts; frame máximo 8.192/8.192 bytes;
event #42 recuperado em 16 descidas com bytes não NFC preservados. Outro caso
teve 236 folds que aumentaram bytes privados e terminou com frame 8.123;
floresta adversarial de 63 raízes/512 bytes ficou em 98.794/128.000 bytes.
Comparação do chooser indexado com enumeração exata: 1.000 partições, H<128,
**para a fórmula antiga**; não demonstra o chooser corrigido nesta revisão.
Fixture SQLite WAL/FULL: SIGKILL antes de insert, após insert e após commit;
inbox preservada e retry com mesmo ID. Resolver nativo recusou alteração e
ausência do original. São referências estruturais, não módulos LOGOS, testes
do kernel, implementação de ACL/provider, PDF/OCR ou benchmark de recall.
Dois runners iniciais atingiram deadlines de 300s/90s; versões indexada e
menor de comparação concluíram. São resultados históricos de 04/out, sem
full verify naquela entrega, e não verificam a nova política de lotes/cache.
A revisão de 19/out tem evidências próprias no relatório de código; nenhum
dos 24 critérios de implementação foi fechado por testar o upstream.

Runner de 1M: NODE_SIZE=64, VIEW_SIZE=8192; evento i com bytes `event#i`,
exceto #42 não NFC. Free quando concatenação cabe; demais nodes Locator,
sem calls LLM e com fontes prontas. A comparação de chooser usa seed 1701.
Fixture adicional matou processo após commit atômico event+turn: replay
retomou snapshot salvo sem duplicar input atual no histórico; Delivered
retornou resposta capturada sem nova admissão/chamada.

## 23. Benchmarks

Long-term recall: informação rara/decisão cedo, distração em 10^3/10^4/10^5/10^6 eventos; ground truth em originais. Exact recovery: hashes/bytes alcançáveis e p50/p99 passos/latência. Preference correction: versões, fontes conflitantes e erro de stale preference. Referential continuity: antecedentes próximos/distantes/ambíguos. Context size: bytes completos, distribuição de resolução, prefixo comum/cache-hit por provider, custo amortizado/free/Locator e settling.

Knowledge: mesmos corpus/queries/relevance labels para lexical only, dense only, hybrid e hybrid+reranker; Recall@k, MRR/nDCG, cobertura/resolução de fonte, duplicate overlap, latência/custo, versões obsoletas e unavailable paths. Cross-memory: tarefas exigindo decisão+fonte; acerto dos dois IDs, localização original, status/taint preservado e claim/ação corretamente mediada.

Baselines episódicas obrigatórias: full context (só onde cabe; reportar onde não cabe), recent sliding window, rolling summary, periodic compaction, vector-only memory, recent window+vector RAG e hierarchical binary memory. Igualar budgets de tokens/custo/latência quando comparável; declarar original retention/recovery em cada baseline. Fazer ablations de free nodes, ContextView persistida versus contexto reconstruído, due antigo versus literal do código, lotes HIGH/LOW versus fold mínimo, blocos de quatro versus bloco único, single flight, fresh turns e semantic episodic search. Variar defaults Node/View/jobs/retries, datasets, seeds e modelos; reportar intervalos, falhas e qualidade separada de acessibilidade estrutural. Não assumir superioridade OptChat-like ou segurança cognitiva. Em pelo menos um provider real autorizado, medir read/write/input tokens e custo com cache frio/quente/TTL; divergência do simulador não pode ser escondida removendo tarefa do denominador.

## 24. Implementation Order

Somente destes módulos, condicionado à infraestrutura existente:

1. Aprovar extensão/ADR; definir encoding, novos domínios CAS, acesso/staging e vetores. Não antecipar kernel/gateway faltantes.
2. Artifact refs/originais pinados, inbox, event append idempotente e recuperação.
3. Binary tree/free nodes, validação geométrica e custódia de generations.
4. Compactor contextual, leases, budgets/diagnósticos/Locator explícito.
5. Duas views incrementais persistidas, lotes HIGH/LOW/due corrigido, settle/cancel e blocos estáveis.
6. Adapters e single flight, deadlines, fingerprints/namespace, testes de wire e custo.
7. Zoom/range/neighbors/date e original resolution.
8. Fresh-turn harness/subagents/MCP read-only; testes de mediação/taint existentes ampliados.
9. Episodic lexical search; dense episódico opcional após baseline.
10. Knowledge registry versionado, source/provenance e sandbox ingestion/chunking.
11. Lexical retrieval e resolve_source; rebuild/ACL/failure paths antes de dense.
12. Embedding profiles, vector generations e hybrid fusion.
13. Reranker opcional, budgets/egress e degradação.
14. Memory Router e cross-memory refs; CLI/TUI auxiliares sem mudar Seal.
15. Observabilidade, benchmarks, ablations e fault injection de todos os itens.

Knowledge Graph fica fora desta ordem inicial; só uma extensão futura apoiada por ganho medido. O ROADMAP Apêndice F organiza estes entregáveis em MEM.0–MEM.7, com gates explícitos; este índice não cria uma segunda ordem concorrente. 5.2 está pronta localmente, mas 6.6/8.x/10.x continuam futuras. Esta proposta não desloca a próxima etapa principal 5.4 nem a ordem das Fases 0–23.

# Changes Required Elsewhere in the Existing Spec

Alterações propostas, não reescrita das seções originais:

- **§2 — Arquitetura proposta:** depois da tabela de artefatos, adicionar: “Episodic Memory e Knowledge Memory são auxiliares não confiáveis de história/descoberta (§15). Seus refs não são premissas citáveis nem artefatos autorizadores.”
- **§3 — Agent lifecycle:** depois dos cinco mecanismos, adicionar referência ao fresh-turn/settle/inbox de E§6; manter sem alteração regra de citação, speculative claims e proibição de efeitos especulativos.
- **§4 — Proof lifecycle, busca/proveniência:** adicionar: “Retrieval auxilia descoberta; SourceReference preserva versão/bytes/location. Ingresso de premissa permanece atribuído/atestado e as dependências são extraídas do certificado; hits RAG não entram automaticamente em Γ.”
- **§5 — Trust boundaries:** acrescentar memória/compactor/embeddings/reranker às componentes não confiáveis e declarar custódia/confidencialidade/progresso separadamente; não inserir LLM ou índice na TCB de correção.
- **§6 — Proof-carrying actions:** adicionar que memória histórica de UserAuthorization/Executed não substitui época/nonce/pré-condições atuais; fetch e provider egress usam contratos/adaptadores existentes, nunca credenciais no harness.
- **§7 — Multi-agent:** acrescentar snapshot/scoped navigation e actor=work para relatórios; resultados de memória não são PVCs. Sem mudar pacote PVC ou confiança em Γ.
- **§8 — Semantic fidelity:** acrescentar que resumo/OCR/RAG/score não aumenta fidelity/grounding/validity; TUI de memória não renderiza Seal parcial.
- **§10 — Ameaças:** acrescentar envenenamento histórico, fonte stale, summary fabricated, namespace/ACL/cache leakage, index loss e source loss, remetendo E§14–16; preservar caráter fundamental de L1/L3/ameaça 13.
- **§12 — Arquitetura final:** adicionar os dois módulos e Router na faixa de processos não confiáveis; referenciar protocolo auxiliar/CAS scopes. Manter kernel/gateway/checkers/renderizador. Tokens seguem ADR 0001 vigente, não introduzir Biscuit por esta extensão.
- **§13 — Integridade de justificativa:** adicionar: “Memória pode influenciar a busca; nunca satisfaz por si a exigência de origem de Γ. Conteúdo recuperado passa pelos mecanismos existentes de atribuição/extração/atestação, inclusive Says/Extracted.”
- **Após §14, antes de Fontes:** adicionar §15 apontando para esta extensão; em Fontes acrescentar referência OptChat técnica. Não renumerar referências §N.k existentes.
- **ROADMAP:** Apêndice F/MEM.0–MEM.7 e referências nas6.6/10.3/10.5–10.8/23.1/23.4 adicionados documentalmente em19/out. Critérios de implementação continuam abertos; memória não vira dependência dos checkers nem desloca fases. Aprovação da extensão base/formatos fica em MEM.0.
- **Specs normativas futuras:** memory/knowledge encodings e vetores próprios; atualização aditiva/versionada de domains/CAS para blobs/manifests. Canon/v1 e AIR não mudam. Cas/v1 continua válido para objetos/domínios existentes.
