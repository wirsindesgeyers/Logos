# CAS antes do append e GC de diagnóstico (Etapa 3.5)

Normativo para a integração; ADR0024. Os formatos cas/v1, event/v1,
epoch-body/v2 e ledger-sqlite/v1 permanecem iguais.

## 1. Associação e ordem

`Ledger::open(DIRECTORY, signer)` abre um CAS obrigatório em `DIRECTORY/cas`,
com Limits padrão. `ledger.cas() -> &Cas` fornece put/get no mesmo store.
Não existe appender público com verificação desativada. Ordem do caller:
`ledger.cas().put::<T>(bytes)` durável → `ledger.append(body)`.

Após conferir estado do handle e estrutura do body, append MUST verificar
todos os objetos abaixo antes de chamar Signer::sign ou iniciar SQL. Ausência
retorna `LedgerError::MissingObject(CasReference)`; I/O, integridade e limite
retornam CasError, sem se converter em ausência/valor padrão. Falha nessa
pré-condição não consome seq/head, não envenena o handle e admite nova tentativa
após put. Falhas de persistência continuam exigindo reabertura, como ledger/v1.

Presença exige diretório/shard/arquivo regular, sem symlinks nessas posições,
limite do domínio e hash recalculado dos bytes. Root CAS symlink é recusada na
abertura do appender. Não é proteção contra troca hostil de paths pelo operador.
CAS continua opaco: objeto presente não equivale a certificado aceito ou O/M
semanticamente válido; não rechecar prova nem inferir Verified desta etapa.

## 2. Inventário do catálogo

`EventBody::cas_references() -> BTreeSet<CasReference>` enumera domínios
explícitos, deduplicados por domínio+digest. Variantes de CasReference:
Vocabulary, Obligation, Certificate, Checker e Model (ModelObject).

| Evento | Objetos exigidos |
| --- | --- |
| VocabPublished | vocabulary no domínio vocab-v1 |
| Proposed | proposal.vocabulary em vocab-v1 |
| ObligationCreated | obligation em obligation-v1 |
| Verified | três hashes do check principal e de cada check de consistência; Witness.model em model-v1 |
| Refuted | três hashes de check e model em model-v1 |
| UnknownRecorded | três hashes de check, sem exceção por Unknown |
| AssumptionDeclared | vocabulary em vocab-v1 |
| EpochAdvanced | vocab em vocab-v1, quando presente |
| CheckerRegistered / CheckerRevoked | checker em checker-v1 |
| Genesis / Retracted / Invalidated | nenhum objeto CAS direto |

Todo CheckRecord exige obligation/certificate/checker nos respectivos domínios,
inclusive o check opcional de Consistency::Unknown e o de Witness.
Digest em outro domínio não satisfaz a referência, mesmo com bytes iguais.

prev/cited/used/target/causes são referências do ledger SQLite. KeyIds, IDs
especulativos, escopo e raiz Merkle não são objetos CAS. kernel_binary/artifact
são digests crus de artefatos externos, sem domínio de objeto atribuído.
Não inventar Fact/Policy ou objetos de preenchimento para essas identidades.

Reabertura/replay preserva o histórico anterior sem exigir retroativamente
os objetos não obrigatórios antes da3.5. Novo append sempre aplica o contrato,
mesmo num ledger antigo. Não migrar nem reassinar os bytes armazenados.

## 3. Diagnóstico somente dry-run

```text
logos ledger gc --dry-run DIRECTORY --public-key ED25519_HEX
```

Sem --dry-run, flag destrutiva ou campos extras: erro antes do diagnóstico.
Nenhuma chave privada é recebida. `ledger_references(directory, public_key)`
confere toda a cadeia/schema/assinatura em conexão read-only e mantém lock
existente até Drop de LedgerReferences. Não cria lock, índice ou eventos.
Seu resultado contém head e raízes tipadas. `Cas::open_existing` abre raiz
existente sem criação/fsync; a CLI nunca chama put. SQLite pode criar/manter
sidecars WAL/SHM durante leitura; dados do banco principal/objetos ficam iguais.
Leitura inclui WAL confirmado, não usa immutable para ignorar histórico.

Inventariar os16 diretórios conhecidos de cas/v1, shards hex minúsculos de2
e nomes de objetos de62 caracteres. Recusar symlinks/arquivos especiais em
qualquer nível, layout desconhecido, nome não UTF-8 e erros I/O. Arquivos
`.tmp.PID.COUNTER` regulares são separados como temporários, nunca removidos.
Conferir limite/hash de todos os objetos publicados, inclusive candidatos
órfãos; raiz direta ausente aborta. Compor o resultado antes de imprimir,
sem lista parcial quando um objeto/ledger inválido for encontrado.

A marcação é conservadora e fica fora da TCB: partir das raízes tipadas dos
eventos, decodificar objetos canônicos alcançáveis com canon/v1 e tratar cada
bytes32 (inclusive chave de mapa) como possível link para todos os objetos
existentes com esse digest. Raw certificate/checker são folhas. Malformação
canônica alcançável aborta em vez de supor zero links. Pode reter bytes32 que
eram apenas metadados; isso não autentica o objeto nem seu significado.

Saída identifica head, contadores e candidatos fora do fechamento observado:
`LEDGER_GC_DRY_RUN ... orphan_candidates=N temporaries=M`, seguido por linhas
`ORPHAN domínio/shard/sufixo` e `TEMP caminho_escapado`. Ordem determinística.
São candidatos relativos àquele histórico/inventário, sem prova sobre objetos
transitivos ausentes, uso externo ou put futuro de outro writer CAS.
**Não há remoção de objetos, temporários ou referenciados.**

## 4. Evidência

logos-ledger/tests/ledger_cas.rs omite28 referências individuais cobrindo o
catálogo/checks aninhados, confere Signer não chamado e seq/head/SQL inalterados,
put e tentativa posterior aceita, domínio errado/corrupção/limite/tipo/symlinks,
lock mantido, WAL confirmado e abertura existente sem criação.
logos-cli/tests/gc.rs executa processos reais, marcação transitiva incluindo
map keys/arrays/raw, candidatos/temporários, falhas completas, argumentos
estritos e conteúdo/mtime/nomes de CAS/banco principal preservados.
Vetores históricos continuam exercitando replay; fixtures de novos appends
usam put real. Worker de crash e oráculo independente conferem objetos por seq.
