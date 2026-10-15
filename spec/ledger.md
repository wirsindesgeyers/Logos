# ledger-sqlite/v1 — persistência da cadeia assinada

Normativo para ROADMAP 3.2 e ADR 0021. MUST/MUST NOT conforme RFC 2119.
Este formato de armazenamento novo começa em v1. event/v1, canon/v1 e
domínios/assinaturas anteriores permanecem iguais.

## 1. Layout

Diretório local exclusivo: ledger.sqlite3, seus arquivos SQLite -wal/-shm e
writer.lock. MUST manter o mesmo arquivo de lock; nunca removê-lo ao fechar
ou ao recuperar. Pai do diretório MUST existir. Diretório/ancestrais MUST
estar sob controle do dono durante o uso. Alias de diretório são canonicalizados.

SQLite application_id = 0x4c4f4753 (LOGS), user_version = 1. Schema exato:

```sql
CREATE TABLE events (seq INTEGER PRIMARY KEY CHECK (seq >= 0), wire BLOB NOT NULL CHECK (length(wire) BETWEEN 1 AND 16777216)) STRICT
```

Nenhuma outra tabela/view/trigger/índice nesta versão. Base sem objetos,
application_id=0 e user_version=0 pode ser inicializada transacionalmente.
Outra versão/schema MUST ser recusada. Não converter base desconhecida.
wire contém exatamente canon(Event) segundo spec/events.md, até 16 MiB.
Coluna seq MUST coincidir com event.seq, no intervalo 0..i64::MAX.
Sem hash/assinatura duplicado no banco: a fonte são os bytes de wire.

## 2. Abertura e replay

MUST obter lock exclusivo de arquivo sem espera antes de abrir SQLite;
segundo escritor MUST falhar mesmo sem transação ativa. Lock dura toda a
conexão. SQLite WAL, synchronous=FULL, fullfsync=ON,
checkpoint_fullfsync=ON, trusted_schema=OFF; conferir valores efetivos.
O processo MUST sincronizar pai/diretório quando cria arquivos.

BEGIN IMMEDIATE protege schema/replay. Ordenar eventos por seq; verificar:
primeiro Genesis seq0/epoch0/prev ausente, seq contíguo, coluna seq igual ao
seq assinado, assinatura externa com chave pública do Signer fornecido pelo
caller, prev igual a h(Event anterior) incluindo assinatura. Event::decode
confere canonicidade/limites; Event::verify confere invariantes do catálogo.
Ledger vazio é válido, mas só aceita Genesis como primeiro evento.

Época inicial 0; EpochAdvanced MUST ter new_epoch maior que época anterior,
copiada no envelope; outros eventos MUST conservar época. Nenhuma regra de
citabilidade/política é inferida. Não aceitar append antes do replay completo.
Falhas MUST ser erros; MUST NOT truncar/reparar eventos para continuar.

## 3. Append

Única API de escrita: Ledger::append(body). Seq é 0 no vazio ou seq anterior+1;
prev é ausente no vazio ou h(Event completo anterior); epoch segue §2.
MUST assinar e verificar contra a chave pública capturada na abertura.
INSERT explícito dentro de BEGIN IMMEDIATE, seguido de COMMIT e sync_all do
diretório. Somente depois devolver Event e atualizar head em memória.
Ninguém fornece cabeçalho/assinatura pronta ao appender.

Erros de construção/assinatura antes da persistência não consomem seq.
Qualquer falha de persistência MUST tornar append/head indisponíveis até
reabrir; o resultado do commit pode ser ambíguo. Evento confirmado sem resposta
pode estar presente após crash. Reabertura MUST recuperar prefixo válido e
conservar todos os eventos cujo append já devolveu sucesso. Não prometer
exactly-once para retries do chamador.

## 4. Limites de garantia e reprodução

Uma chave fixa por ledger. Verificação de assinatura não demonstra validade
lógica, autoridade administrativa ou existência de objetos no CAS (3.5).
Remover um sufixo inteiro deixa um prefixo válido: detecção exige âncora
externa (13.2). Lock pressupõe escritores cooperativos e inodes estáveis.
Durabilidade depende de sync honrado pelo SO/VFS/disco. SIGKILL não simula
perda de energia. Replay é linear, sem checkpoint/índice derivado.

Layout versionado em spec/vectors/ledger/layout.json; vetores de eventos
independentes existentes em spec/vectors/events são consumidos byte a byte
também pelo teste de armazenamento. Testes reproduzíveis:

```sh
cargo test -p logos-ledger --test ledger
ci/check-ledger-crash.sh
```
