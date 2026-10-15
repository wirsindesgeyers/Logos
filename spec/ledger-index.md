# ledger-index/v1 — índices derivados e LedgerView

Normativo para ROADMAP 3.3; ADR 0022. MUST/MUST NOT conforme RFC 2119.
Novo formato de cache v1; event/v1, ledger-sqlite/v1 e obligation/v1 não mudam.

## 1. Fonte e snapshot

MUST conferir a cadeia integral da 3.2 contra a chave pública externa antes
de disponibilizar a projeção. MUST manter writer.lock enquanto o Index existir.
Ledger::index empresta o escritor; appends ficam impedidos por esse empréstimo.
Projeção SQLite privada em memória, imutável após construção. index.sqlite3
é cópia derivada para comparação/reconstrução, MUST NOT autorizar premissas.
Nunca abrir o cache como fonte das consultas. Nenhum índice novo na base
ledger.sqlite3; o schema estrito da 3.2 permanece idêntico.

Após construir, comparar todas as colunas de todas as linhas ordenadas do
cache anterior e classificar Missing/Equal/Different/Invalid. Erro de I/O
MUST ser erro, não Missing. SQLite corrompido/versão/schema desconhecido é
Invalid explícito. Publicar nova cópia por backup para arquivo temporário
exclusivo, sync do arquivo, rename atômico e sync do diretório. Nunca apagar
ou reparar os eventos assinados. A consulta continua usando a cópia privada.

## 2. Layout

application_id=0x4c494458 (LIDX), user_version=1. Schema SQL exato e codebook
em [vectors/ledger-index/layout.json](vectors/ledger-index/layout.json),
conferidos por teste contra arquivo real. Sete objetos SQL, sem triggers/views:

| Tabela | Campos | Chave/ordem de comparação |
| --- | --- | --- |
| claims | id, h_o?, status, epoch, retracted, vocab?, formula?, scope?, checker? | id |
| deps | claim, premise | claim,premise; índice reverso premise,claim |
| citable | ref, epoch | ref,epoch |
| revoked | checker | checker |
| metadata | singleton=1, seq?, epoch?, hash? | singleton |

Digests são BLOB32; epoch é BLOB8 big-endian unsigned (inclusive u64::MAX).
seq usa INTEGER não negativo, como na fonte. formula é canon(Formula), não
texto. scope conserva Text do evento. Metadata sem cabeça tem três NULL;
cache vazio não inventa evento Genesis. Demais tabelas sem linhas é válido.
Codebook: 0 Proposal, 1 Pending, 2 Verified, 3 Refuted, 4 Unknown,
5 Assumption, 6 Invalidated. Fact/Policy/Rejected não têm evento neste catálogo
e MUST NOT ser sintetizados. Índices `claims_checker`, `deps_premise`.

## 3. Projeção

Uma linha por evento lógico, identificada por **h(Event)**, não h(O), texto ou
ID da proposta. Proposed→Proposal; ObligationCreated→Pending; Verified→Verified;
Refuted→Refuted; UnknownRecorded→Unknown; AssumptionDeclared→Assumption.
Um evento Verified não transforma o ID do Pending anterior em Verified.
Fórmula/V embutidos só em Proposed/Assumption; demais armazenam h(O).

Para Verified, deps MUST conter exatamente used, nunca todo cited. Cada used
MUST apontar para claim anterior citável na época desse evento; caso contrário
reconstrução falha. Citável em estado: Assumption/Verified, não retratado nem
invalidado, época da emissão igual à época corrente do snapshot. Unknown/
Refuted/Pending/Proposal MUST NOT ser promovidos. is_citable só responde sobre
a época corrente, não oferece consultas históricas nem janelas da 3.4.

Retracted/Invalidated MUST ter alvo lógico anterior. Invalidar alvo e fechamento
transitivo por deps, remover citable; Retracted marca retracted só no alvo.
CheckerRevoked invalida Verified desse checker e dependentes, registra revogação;
Verified posteriores desse checker são Invalidated. CheckerRegistered não
restaura um hash revogado. Isso é projeção conservadora de consulta, MUST NOT
criar eventos ou interpretar autorização administrativa. Registro da cascata
e políticas do kernel ficam para as etapas correspondentes.

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
índices de ações, kernel ou autorização administrativa. Assinatura do registro
não demonstra que a prova foi checada; auditor independente é a 3.6.
