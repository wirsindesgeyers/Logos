# kernel-protocol/v1

Etapa5.2/ADR0035. Novo formato; proposal/v1 JSON não muda. Protocolo de
candidatos e leitura: não é certificado, token, Seal ou construtor de Claim.

## Framing e requisições

Cada frame é u32 big-endian length seguido por exatamente length bytes de
canon/v1. Length zero, excesso, EOF parcial ou bytes sobrando em um frame
são erros. Conexões podem carregar vários frames; fragmentação/coalescência
do socket não altera mensagens. Envelope exato: [1, request_id_u64, tag, payload].

| tag | Mensagem | payload exato |
| --- | --- | --- |
| 0 | Propose | [text_pt_bytes, goal_air_bytes, cites_hashes, speculative_ids, vocab_hash] |
| 1 | Certify | [h_O, format_Text, certificate_ref] |
| 2 | Refute | [h_O, model_ref] |
| 3 | Witness | [h_O, model_ref] |
| 4 | Assume | [vocab_hash, formula_air_bytes, scope_Text] |
| 5 | Query | [0] para head, [1,h_Event] para status |
| 6 | Subscribe | [from_seq_u64, max_events_u64] |

Hash/ID é bstr32. ObjectRef é [0,h_object] (CAS) ou [1,name_Text,h_object]
(staging). Name tem exatamente64 caracteres ASCII hex minúsculos, sem caminho.
Hash usa o domínio Certificate ou ModelObject conforme o comando. Format é
Text não vazio ≤256bytes; nomes de formatos ainda não reconhecidos continuam
candidatos e são recusados pelo handler/registry, nunca aceitos por default.
Text_pt preserva UTF-8 inclusive não NFC; texto, cites e speculative têm os
limites da1.6 (64KiB,1024,1024). AIR completo ≤1MiB,128 níveis/100000 nós.
Scope é Text ≤64KiB. Aridades/tipos/versões/tags extras MUST ser recusados.

## Respostas e eventos

Resposta [1,request_id,0,error_code] ou [1,request_id,1,value]. Codes:
0 Invalid,1 Limit,2 Unauthorized,3 Unavailable,4 Storage. Não há sucesso
implícito em EOF/erro. Query head retorna [] ou [seq,epoch,h_Event]; status
retorna [] ou [kind,epoch,retracted]. Kind: Fact0, Policy1, Verified2,
Assumption3, Pending4, Refuted5, Unknown6, Rejected7, Invalidated8, Proposal9.
Esses dados não criam credenciais.

Etapa5.3/ADR0038 acrescenta o resultado **propose-result/v1**, primeiro formato
de sucesso do PROPOSE: `value = [1,h_O_bytes32,O_canonical_bytes]`.
Envelope externo completo: `[1,request_id,1,[1,h_O,O_bytes]]`. O primeiro1
interno é a versão deste resultado novo. Requisições/envelope/eventos
históricos não mudam. Vetores independentes em
`vectors/kernel-protocol/propose-result-v1.json`; o gerador existente os
confere junto com os vetores de requisição.

PROPOSE MUST resolver V no CAS e montar a snapshot autenticada do ledger
antes de generate. Especulação não vazia é inválida; scope opaco é vazio
até5.7. O kernel MUST serializar/limitar O a1MiB e publicar os bytes no CAS
antes de registrar Proposed+ObligationCreated pelas transições5.1.
Resposta só após ambos os appends confirmados. Erro entre eles pode deixar
Proposed não citável e O órfão; nunca sucesso/Pending fictício ou retry
automático. O limite de bytes de saída não é limite de RSS/wall da geração.

Proposed.author é a chave do kernel lida do Genesis autenticado. UID é
autenticado somente na conexão; não é persistido no evento. Não atribuir
essa assinatura ao agente. Query::Status continua recebendo **h(Event)**,
nunca h(O); o evento Pending pode ser obtido em Subscribe. Nenhum PROPOSE
emite Verified, Fact, Policy ou autorização. WITNESS está disponível na5.5;
ASSUME continua explicitamente indisponível até5.7.

Subscribe envia [0,Event] por evento e termina com [1,next_seq] ao atingir
max_events (1..256) ou o prazo da assinatura. O cursor é explícito; nenhum
evento é truncado ou descartado silenciosamente. Novos eventos podem entrar
por outra conexão do mesmo UID. Frames de resposta têm teto independente.
MUST verificar o prazo também antes de cada evento da página e limitar cada
escrita pelo menor prazo entre frame e assinatura. O cursor final não renova
o prazo: se a assinatura já expirou entre frames, tentar enviá-lo sem bloqueio;
socket cheio ou expiração durante frame fecha com erro. Um frame parcial nunca
é reconhecido por cursor; o cliente retoma do último evento completo observado.
O prazo de I/O não preempta o handler/replay síncrono. Status e PROPOSE usam a
projeção viva autenticada do writer (ADR0040/spec ledger-index), sem reconstruir
e publicar o índice inteiro por requisição.
Não manter lock do ledger enquanto espera/escreve no socket.
Cada página relê e verifica no máximo257 eventos (até256 mais o predecessor),
conferindo assinatura, sequência, links e a cabeça quando alcançada. A abertura
do Ledger já autenticou o prefixo inteiro; paginação não substitui esse replay.
O servidor busca um evento por página (e no máximo seu predecessor), para
reavaliar o prazo e liberar o writer entre eventos; não materializa256 eventos
grandes de uma vez. max_events continua1..256 para a assinatura inteira.

## Autenticação, limites e staging

Linux SO_PEERCRED, UID configurado localmente, nunca no payload. Socket não
substitui caminho existente; pai controlado pelo kernel sem escrita de grupo/
outros. O processo autentica antes de ler o primeiro frame. Same-UID pode
compartilhar descritores; isolamento de processos/chaves é da5.11.

Padrões: request2MiB; resposta16MiB+1024;32frames/s;1024frames/conexão;
64MiB de frames/conexão; objeto staging64MiB; importações128MiB/conexão;
prazo absoluto por frame5s, assinatura5s; no máximo8 conexões do mesmo UID.
Excesso fecha a sessão com erro; não dormir segurando o ledger.

Staging é diretório do UID do agente sem escrita por grupo/outros. Área privada
é diretório0700 do kernel, no mesmo filesystem. Ambos são abertos sem seguir
symlink e mantidos por descritor. Rename captura a entrada; openat sem seguir
link e com NONBLOCK impede FIFO de bloquear. Exigir arquivo regular, UID
esperado e nlink1. Snapshot privado em memória limitado, hash conferido antes do CAS;
caller recebe somente o hash dos bytes publicados. Nome/hash não autorizam
prova; alterações por descritor antigo não alteram o snapshot conferido.
Diretórios inválidos capturados MUST ser devolvidos intactos ao staging do
agente por rename ancorado/NOREPLACE. Se o nome original já contiver um upload
novo, preservá-lo e devolver sob nome `rejected-*` com nonce de128bits do SO, com até
oito tentativas totais. Não percorrer/excluir filhos nem seguir symlinks.
Falha de devolução/sync MUST ser erro explícito com a causa e nome capturado,
nunca limpeza declarada como concluída. Outros tipos são removidos somente pelo
nome capturado privado. Devolução de diretório não consome quota de bytes.
Erros não produzem claim; captura pode consumir o arquivo, sem retry automático.

ASSUME retorna Unavailable até5.7.
PROPOSE/CERTIFY/REFUTE/WITNESS têm implementação real (5.3–5.5).

## Processo

`cargo run -p logos-kernel -- SOCKET LEDGER KEYFILE STAGING PRIVATE AGENT_UID [CAKE_BINARY]`
abre a chave0600 existente, o ledger e os diretórios provisionados pelo operador.
Ledger vazio recebe Genesis com o hash do executável real; ledger existente é
reaberto por replay. Antes de servir, o operador provisiona CheckerRegistered
para os checkers Rust do executável atual (lrat-rup/v1, model/v1) e, se passado,
o cake-lpr/v1 conhecido. Autoridade local é a chave do kernel. Reinstalar
nunca reativa um hash revogado. Query/Subscribe continuam somente de leitura.
O socket existente precisa ser removido pelo operador após encerrar o processo;
o servidor nunca o substitui automaticamente.

## CERTIFY e REFUTE (5.4/ADR0041)

O registry MUST combinar implementação local conhecida, hash exato e registro
administrativo ativo autenticado pela chave local. Registro no ledger nunca
carrega código do CAS. Format desconhecido, checker não registrado/revogado,
assinatura administrativa inválida ou falha operacional MUST recusar sem Verified.
Revogação é terminal para o hash nesta etapa; mudança de build Rust requer novo
registro. A identidade Rust é h(bytes do executável em execução); cake conserva
hash fixado e revalidação de snapshot por execução. Nenhum caminho vem do cliente.

CertificationHandlers.PROPOSE registra rota Lrat e obrigação-body/v2 com CNF
rederivada no CAS quando o fragmento permite; General/enumeração opaca mantém
rota Model. O handler mínimo ProposalHandlers5.3 conserva a rota Model histórica.
Os appends continuam Proposed+ObligationCreated adjacentes. CERTIFY MUST
rederivar grounding/CNF dos bytes de O, nunca usar CNF indicada pelo agente.

Recuperação privada de Pending MUST conferir replay autenticado até a cabeça,
continuidade entre páginas, par adjacente Proposed/ObligationCreated, h(O),
época atual, proposta/Pending não invalidados e ausência de conclusão posterior.
Regerar O com snapshot atual e exigir bytes idênticos; citações que perderam
elegibilidade recusam a operação. UnknownRecorded/CertificateRejected permitem
nova tentativa; não há política automática de esgotamento. Replay tem teto de
1 milhão de eventos e erro explícito ao exceder, sem resultado parcial.

CERTIFY admite lrat-rup/v1 (Rust e extração autenticada de used) e cake-lpr/v1
(Rust ativo para used mais cake ativo). Ambos os resultados exigidos MUST ser
Accepted antes de Verified. Used vem do fecho/mapa da4.7 e MUST ser subconjunto
das citações; não copiar todas as citações. DomainSource Fact/Policy ainda opaco
MUST ser recusado, inclusive na recuperação para REFUTE; nenhum cast o autentica.
Na5.4 Verified usa ConsistencyUnknown sem tentativa; a5.5 incorpora testemunha
ou tentativa de consistência conforme a seção WITNESS abaixo.

REFUTE usa ModelChecker ativo sobre O/M exatos do CAS. M é publicado também
como Certificate, preservando separação de domínio e igualdade dos bytes.
Accepted emite Refuted, nunca Verified. Rejected em qualquer rota emite
CertificateRejected e preserva Pending. Unknown emite UnknownRecorded com
Budget/limite, sem autorização. Falha de I/O/identidade/isolamento não recebe
um outcome lógico inventado. Pré-processamento tem limites próprios de AIR,
grounding/CNF/CAS; Budget do checker e Budget de used são alocações separadas,
sem teto de RSS/wall rígido no código interno.

Novo sucesso check-result/v1: `[1,status,h_event]`, status0 Verified,1 Refuted,
2 CertificateRejected/Pending,3 UnknownRecorded. Retornar só após append
confirmado. Esses valores não são credenciais nem autorização. Vetores em
`vectors/kernel-protocol/check-result-v1.json`, conferidos pelo gerador existente.

O auditor independente rederiva a CNF e recheca com Rust/cake. A API
verify_with_checkers permite identidades Rust explicitamente configuradas pelo
operador, nunca derivadas automaticamente do ledger/CAS. Nenhum binário CAS
é executado; a API existente verify_with_cake conserva sua lista restrita.


## WITNESS — Etapa5.5

Request existente tag3 continua kernel-protocol/v1. Resultado witness-result/v1:
`[1,outcome,event]`, outcome0=Accepted,1=Rejected,2=Unknown. O evento é sempre
ConsistencyRecorded, nunca Verified/Refuted. Erro operacional, instância
inelegível, registro inativo ou prazo expirado usa resposta de erro existente.
Prazo lógico inclusivo1024 eventos desde Pending; ver events.md/ADR0042.

WITNESS seleciona a instância mais recente de h(O). Consultar a referência
Verified anterior continua possível e não herda anexos da nova proposta.

Cliente Python10.1: [contrato e exemplos](python-client.md), fora da TCB.
