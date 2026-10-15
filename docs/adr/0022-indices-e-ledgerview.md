# ADR 0022 — Projeção reconstruível e LedgerView concreta

- Estado: aceito
- Data: 2026-10-15
- Escopo: ROADMAP 3.3

## Decisão

Manter ledger-sqlite/v1 imutável. Novo ledger-index/v1 em index.sqlite3,
com claims/deps/citable, checker revogado e metadata da cabeça. A autoridade
continua nos eventos assinados. Cada abertura reconstrói a projeção em SQLite
privado na memória a partir do replay integral; nunca importa confiança de um
cache. A cópia em disco é comparada integralmente e substituída atomicamente.
Ausente/igual/diferente/inválida são resultados distintos no relatório.
Falhas de I/O não são ausência nem resultado padrão. Memória/custo de abertura
crescem com o histórico; latência da consulta mede snapshot já construído.

Lock de escritor da 3.2 protege abertura e toda a vida do Index. Ledger::index
empresta o writer e impede append enquanto esse Index existir. Snapshot
LedgerView empresta o Index e captura apenas as referências solicitadas;
lookup de referência não preparada é Unavailable, não ausência inventada.
O CLI usa chave pública Ed25519, não FileSigner/chave privada.

ID de claim é h(Event) conforme PremiseRef. Proposed/ObligationCreated/
Verified/Refuted/UnknownRecorded/AssumptionDeclared têm linhas distintas;
uma verificação posterior não promove o evento Pending anterior. Verified
usa somente used para deps, nunca todas as citações. Assumption e Verified
podem ser citáveis; demais categorias não. Isso projeta o registro assinado,
não recheca certificados nem prova que o kernel executou o checker (3.6/5).

Retracted/Invalidated removem citabilidade do alvo e do fechamento de
dependentes; CheckerRevoked faz o mesmo para claims desse checker. A projeção
conservadora não cria eventos Invalidated: registro/autorização pelo kernel
continuam futuros. Revogação de hash não é desfeita por CheckerRegistered.
Dependência usada deve apontar para claim anterior e citável na época do
evento. Um Verified após revogação é projetado como Invalidated.

Epoch é blob unsigned de 8 bytes; não estreitar u64 a i64. Sem janela temporal
de fatos: is_citable só aceita a época corrente do snapshot, e a emissão deve
ser dessa época. Sem promoção de época/Unknown/Rejected. Scope de Assumption
é rótulo opaco: a LedgerView exige igualdade exata com o scope escolhido pelo
caller, inclusive nas assumptions alcançáveis por deps. is_citable é consulta
de estado/época; a seleção de escopo pertence ao snapshot usado por generate.

Formula/Vocabulary não vêm de strings ou de cache persistido. Fórmulas
embutidas são associadas a h(V) assinado, carregado do CAS e sort-checkado.
Para eventos com h(O), ler somente o h(V) do envelope estrito, carregar V do
CAS e chamar decode_for_check completo com h(O) esperado do evento. Acrescentar
vocabulary_for_check ao reader existente não abre construção selada de O.
Objetos ausentes/corrompidos/limitados são erros explícitos. Isto não implementa
exigência de presença no CAS antes do append (3.5).

rusqlite continua =0.40.2; habilitar somente backup, sem novo pacote runtime.
Não habilitar cache de statements: isso acrescentaria hashlink à TCB;
usar prepare e medir o custo real. Dependências diretas do CLI são ledger/crypto,
com canon transitivo pelo ledger.
Código próprio do ledger/reader permanece TCB estrito, manifesto e fuzz.

## Consequências

- Cache adulterado não altera consulta nem gera premissa; reconstrução reporta
  diferença e a fonte continua sendo a cadeia conferida.
- Snapshot não é streaming de estado mutável, não substitui âncora externa
  contra rollback, nem implementa políticas/admin/fatos de fases futuras.
- Medição de 1M inclui eventos realmente assinados e replay real. Preparação
  de fixture em transação única não mede durabilidade do appender, já testada
  em 3.2. p99 em memória não promete p99 de cold start/disco.

Referências: [SQLite WITHOUT ROWID](https://www.sqlite.org/withoutrowid.html),
[rusqlite backup](https://docs.rs/rusqlite/0.40.2/rusqlite/struct.Connection.html).
