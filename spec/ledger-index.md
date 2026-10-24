# ledger-index/v1 — índices derivados e LedgerView

Normativo para ROADMAP 3.3; ADR 0022/0040. MUST/MUST NOT conforme RFC 2119.
Novo formato de cache v1; event/v1, ledger-sqlite/v1 e obligation/v1 não mudam.

## 1. Fonte e snapshot

MUST conferir a cadeia integral da 3.2 contra a chave pública externa antes
de disponibilizar a projeção. MUST manter writer.lock enquanto o Index existir.
Ledger::index empresta o escritor; appends ficam impedidos por esse empréstimo.
Projeção SQLite privada em memória, imutável enquanto o snapshot existir. index.sqlite3
é cópia derivada para comparação/reconstrução, MUST NOT autorizar premissas.
Nunca abrir o cache como fonte das consultas. Nenhum índice novo na base
ledger.sqlite3; o schema estrito da 3.2 permanece idêntico.

`Ledger::live_index` oferece a mesma semântica de consulta, com projeção privada
mantida pelo writer (ADR0040). Na primeira chamada MUST autenticar a cadeia
inteira e conferir a cabeça contra o writer antes de publicar qualquer cache.
A varredura inicial MUST incluir todas as linhas, inclusive sequências
negativas inválidas; o limite inferior de sequência só exclui prefixo quando
esse prefixo já foi autenticado. Linhas inválidas MUST gerar erro, não sumir
silenciosamente por filtro SQL.
Nas chamadas seguintes sem append MUST reutilizar a projeção em memória sem
replay, comparação ou publicação em disco. Com appends MUST autenticar somente
o sufixo novo, encadeado à cabeça privada anterior, e exigir a cabeça final
exata do writer (seq/época/hash). A mesma projeção de §3 e metadata avançam em
transação; falha MUST retornar erro e descartar a projeção afetada, nunca uma
view antiga ou parcialmente atualizada. Poisoned MUST bloquear a API viva.
Cada view empresta o Ledger, impedindo append/Drop enquanto existir; a view é
imutável. Reabrir o writer perde esse estado e exige replay integral.

O relatório/comparação/publicação abaixo pertence às APIs explícitas de
reconstrução `Ledger::index`/`LedgerIndex::open`; a API viva não oferece um
relatório fictício por consulta. index.sqlite3 pode ficar antigo entre rebuilds,
sem afetar as views privadas. Alteração do disco não revoga fatos já autenticados
em memória; sufixos novos e reaberturas continuam conferidos. Nenhum checkpoint
persistido ou confiança no cache foi acrescentado.

Nas APIs explícitas de reconstrução, comparar todas as colunas/linhas ordenadas do
cache anterior e classificar Missing/Equal/Different/Invalid. Erro de I/O
MUST ser erro, não Missing. SQLite corrompido/versão/schema desconhecido é
Invalid explícito. Publicar nova cópia por backup para arquivo temporário
exclusivo, sync do arquivo, rename atômico e sync do diretório. Nunca apagar
ou reparar os eventos assinados. A consulta continua usando a cópia privada.

## 2. Layout

application_id=0x4c494458 (LIDX), user_version=6 (ledger-index/v6, Etapa 6.4: tabela
`revoked_issuers`; v5 (6.2): emissor e
tópico em `facts`, tabelas `trust` e `trust_configs`; v4 (6.1): status
Fact=8 e tabela `facts` com a janela de cada fato; v3 (5.10) acrescentou Suspended=7 e
`claims_h_o` sobre `(h_o,status)`; v2 acrescentou `claims_h_o` na 5.6; caches anteriores são recusados e
reconstruídos, nunca migrados). Schema SQL exato e codebook
em [vectors/ledger-index/layout.json](vectors/ledger-index/layout.json),
conferidos por teste contra arquivo real. Catorze objetos SQL, sem triggers/views:

| Tabela | Campos | Chave/ordem de comparação |
| --- | --- | --- |
| claims | id, h_o?, status, epoch, retracted, vocab?, formula?, scope?, checker? | id |
| deps | claim, premise | claim,premise; índice reverso premise,claim |
| citable | ref, epoch | ref,epoch |
| revoked | checker | checker |
| facts | id, fact, issuer, topic, first?, last?, until? | id; fact único |
| trust | issuer, topic, config | issuer,topic (concessões vigentes; config = h(TrustConfigPublished)) |
| trust_configs | id, version | id; version única (BLOB8) |
| revoked_issuers | issuer, since, event | issuer (uma revogação por emissor) |
| metadata | singleton=1, seq?, epoch?, hash? | singleton |

Digests são BLOB32; epoch é BLOB8 big-endian unsigned (inclusive u64::MAX).
seq usa INTEGER não negativo, como na fonte. formula é canon(Formula), não
texto. scope conserva Text do evento. Metadata sem cabeça tem três NULL;
cache vazio não inventa evento Genesis. Demais tabelas sem linhas é válido.
Codebook: 0 Proposal, 1 Pending, 2 Verified, 3 Refuted, 4 Unknown,
5 Assumption, 6 Invalidated, 7 Suspended, 8 Fact. Fact só nasce de FactRegistered
(6.1; janela em `facts`: épocas first/last BLOB8 ou instante until BLOB8);
Policy/Rejected não têm evento neste catálogo e MUST NOT ser sintetizados. Índices `claims_checker`, `claims_h_o`, `deps_premise`, `facts_fact` (único), `trust_configs_version` (único).

## 3. Projeção

Uma linha por evento lógico, identificada por **h(Event)**, não h(O), texto ou
ID da proposta. Proposed→Proposal; ObligationCreated→Pending; Verified→Verified;
Refuted→Refuted; UnknownRecorded→Unknown; AssumptionDeclared→Assumption.
CertificateRejected→Pending, não citável; uma rejeição não é refutação.
UnknownRecorded descreve a tentativa inconclusiva; o handler5.4 permite retry
privado de Pending enquanto não houver conclusão Verified/Refuted nem invalidação.
Um evento Verified não transforma o ID do Pending anterior em Verified.
Fórmula/V embutidos só em Proposed/Assumption; demais armazenam h(O).

Para Verified, deps MUST conter exatamente used, nunca todo cited. Cada used
MUST apontar para claim anterior citável na época desse evento; caso contrário
reconstrução falha. Citável em estado: Assumption/Verified, não retratado nem
invalidado, época da emissão igual à época corrente do snapshot. Unknown/
Refuted/Pending/Proposal MUST NOT ser promovidos. is_citable só responde sobre
a época corrente, não oferece citabilidade histórica. EpochRange genérico
da 3.4 não prolonga essas entradas. Fact (6.1/6.2) é citável enquanto vivo, com
a janela aplicável e `Trusted(issuer, topic)` na tabela `trust`; `used` de fato
exige o mesmo na época do Verified. TrustConfigPublished (6.2) lê trust/v1 do CAS,
exige versão maior que todas as anteriores, grava `trust_configs` e substitui
`trust`. FactRegistered sem concessão vigente é erro de reconstrução.
Extracted (6.6) projeta a fórmula como status 5 (Assumption) com o escopo fixo
`session:extracted`, sem mudar o esquema; a origem fica no evento.
IssuerRevoked (6.4) grava `revoked_issuers` (repetida é erro) e mata, como um
Retracted, todo Fact vivo do emissor com época de registro ≥ since; FactRegistered
posterior de emissor revogado é erro de reconstrução. Retracted aceita Fact.

Retracted/Invalidated MUST ter alvo lógico anterior; Retracted marca retracted
só no alvo. Cascata ATMS (Etapa 5.9, ADR0052): o **nó** de uma linha Verified é
seu h(O) (todas as rotas Verified com o mesmo h(O)); qualquer outra linha é nó de
si mesma. Retracted, Invalidated e Rechecked com Rejected matam o alvo;
CheckerRevoked **suspende** (5.10, ADR0056) e registra a revogação; Verified
posteriores desse checker nascem Invalidated. Morta = status Invalidated e fora de citable. Depois de cada
morte, a projeção MUST propagar até o ponto fixo: um nó h(O) está vivo se alguma
rota ainda Verified tem todas as premissas used vivas; premissa viva é assumption
não morta ou rota de nó h(O) vivo. Nó sem rota sustentada cai: todas as suas rotas
Verified viram Invalidated (terminal) e os dependentes são reavaliados. Rota cujo
nó segue vivo por outra rota continua Verified/citável, mesmo com premissa morta.
Com uma rota por nó, coincide com o fechamento transitivo por deps.
Para a morte, rota Suspended conta como viva.

Atividade (5.10): checker efetivo de uma rota é o do Verified, trocado pelo do
último Rechecked Accepted. Um nó h(O) vivo é ativo se alguma rota sua de checker
efetivo não revogado tem todas as premissas used ativas (assumption viva ou nó
ativo); suas rotas vivas são então Verified e citáveis na época de emissão. Nó vivo
inativo: rotas vivas Suspended, fora de citable, e a reavaliação segue para os
dependentes até o ponto fixo. Rota nova Verified de checker ativo reativa o nó.
Rechecked exige alvo rota viva e mesmo h(O); Accepted de checker revogado é erro.
CheckerRegistered não restaura hash revogado. A projeção MUST NOT criar eventos
nem interpretar autorização administrativa; o registro Invalidated da cascata é
do kernel (state-machine.md §4).

## 4. Fórmulas e interface confiável

status/dependents/is_citable consultam o snapshot; inexistência de linha lógica
é None/false/vetor vazio, enquanto falhas SQL são erro. is_citable consulta
estado/época; não afirma presença no CAS, scope contextual ou validade de prova.

formula_of e snapshot MUST resolver V/O pelo CAS que confere hash dos bytes
originais. Formula embutida usa h(V) assinado; decodificar V e sort-check de
Formula fechada. Para h(O), `vocabulary_for_check` confere teto/hash/canon e
envelope, extrai somente h(V); carregar V e chamar decode_for_check completo
contra O original e h(O) do evento. Nunca construir Obligation selada de bytes.
Erro/limite/ausência/integridade MUST NOT virar fórmula padrão ou None.

`snapshot(cas, references, scope)` prepara exatamente as referências pedidas.
O scope é rótulo opaco, comparação exata, sem interpretar texto. Todas as
assumptions alcançáveis por deps usados, inclusive a própria entrada, MUST
ter esse scope. Snapshot empresta Index, epoch permanece constante. LedgerView
lookup retorna referência à Formula/Vocabulary já conferidos, kind/epoch/
retracted reais. Referência preparada inexistente retorna None; referência
não preparada retorna Unavailable. generate continua conferindo citabilidade,
época, sort e compatibilidade direcional de vocabulário. Não emitir Verdict.

## 5. CLI, escala e limites

```sh
cargo run -p logos-cli --bin logos -- ledger rebuild-index DIR --public-key ED25519_HEX
cargo test -p logos-ledger --test index
ci/check-ledger-index.sh
```

CLI usa somente chave pública Ed25519 (64 hex), exige argumentos exatos e
devolve saída não zero em falha. Relatório distingue cache ausente/igual/
diferente/inválido. Rebuild não exige CAS: resolve objetos apenas nas APIs de
fórmulas/snapshot, sem antecipar put→append/GC da 3.5.

Escala usa cadeias reais assinadas com 100k/1M eventos; preparação em uma
transação agrupa INSERTs só como fixture. Todos os eventos são rechecados no
replay real. Teste de 100k apaga cache e compara as cinco tabelas completas;
1M mede chamadas individuais de is_citable da API pública, positivas/negativas/
época errada, p99<1ms. Artefato target/ledger-index-scale.json; p99 exclui
reconstrução e refere-se ao snapshot SQLite privado em memória.

Custo de memória e abertura cresce com o histórico. Ainda sem checkpoint,
anti-rollback externo, rechecagem de certificados, janelas de fatos, UI,
índices de ações ou autorização administrativa. A integração com o kernel usa
a API viva (Etapas5.2/5.3, ADR0040). Assinatura do registro
não demonstra que a prova foi checada; auditor independente é a 3.6.


## ConsistencyRecorded (5.5)

O corpo v3/tag14 é anexo de evidência, não claim. A projeção mantém as cinco
tabelas e o schema1: não cria linha claims, citable ou deps para este evento.
Verified histórico mantém sua consistência original. O kernel consulta replay
mais recheck para a consistência efetiva e a elegibilidade `PremiseUse::Action`;
`is_citable` isolado é apenas lógico e não autoriza ação.
