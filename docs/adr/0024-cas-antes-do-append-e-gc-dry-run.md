# ADR 0024 — CAS obrigatório antes do append e GC somente dry-run

- Estado: aceito para Etapa 3.5
- Data: 2026-10-15
- Escopo: ROADMAP 3.5; não implementa auditor, fatos ou GC destrutivo

Estimativa P → M: integração sem bypass, migração de fixtures/worker e
inventário/diagnóstico conservador do GC. Escopo e ordem preservados.

## Decisão

Ledger possui um Cas em `DIRECTORY/cas`, aberto com os limites padrão.
`Ledger::open(directory, signer)` conserva sua assinatura, mas todo append
passa pelo novo contrato. `Ledger::cas()` permite put/get no mesmo store.
Não criar flag de bypass, appender legado ou variante unchecked pública.
Reabertura confere a cadeia histórica sem exigir retroativamente objetos
que etapas anteriores não obrigavam a guardar; bytes/hashes/assinaturas e
ledger-sqlite/v1 permanecem iguais. O novo contrato vale para todo novo append.

Antes de assinar, validar body e conferir todas as referências CAS tipadas:
VocabPublished/Proposed/Assumption e EpochAdvanced.vocab → Vocabulary;
ObligationCreated → Obligation; cada CheckRecord, inclusive os opcionais de
consistência → Obligation/Certificate/Checker; Witness/Refuted → ModelObject;
CheckerRegistered/CheckerRevoked → Checker. Deduplicar por domínio+digest.
CasReference enumera esses cinco tipos sem criar formato wire ou domínio.

Presença significa arquivo regular no caminho CAS, domínio correto, bytes
dentro do limite e hash íntegro, não apenas exists(). Erro ou ausência falha
antes de Signer::sign/SQL, preservando seq/head e permitindo put e nova tentativa.
CAS continua opaco: isso não recheca certificados nem valida semanticamente O/M.
Não converter falha de leitura em ausência nem criar objetos de preenchimento.

prev/cited/used/target/causes são referências de eventos no SQLite, não objetos
que precisam de cópia no CAS. KeyId/autor/escopo são identidade/metadados.
kernel_binary/artifact são SHA256 crus de artefatos externos; policy_root é
compromisso Merkle opaco. Não inventar objetos/domínios para essas identidades.

## GC de diagnóstico

`logos ledger gc --dry-run DIRECTORY --public-key ED25519_HEX` é a única
forma aceita. Sem chave privada. Um helper de leitura no ledger mantém o
lock existente, confere schema/cadeia/assinaturas e coleta referências; não
reconstrói índice nem escreve ledger. `Cas::open_existing` não cria diretórios.

Inventário e relatório do GC ficam fora da TCB, na CLI. Percorrer somente os
16 domínios e o layout cas/v1, sem seguir symlinks. Nomes desconhecidos,
arquivos especiais, I/O, objetos corrompidos ou raízes diretas ausentes
abortam o relatório. Temporários `.tmp.PID.COUNTER` são reportados à parte,
sem tratar arquivo de writer em andamento como objeto publicado.

Marcação conservadora: raízes tipadas vêm dos eventos. Para objetos canônicos
alcançáveis, usar o decoder canon existente e preservar todos os objetos cujo
digest coincida com qualquer bytes32 embutido, inclusive em mapas/arrays.
Isto evita introduzir parsers de Fact/Policy ou dependência cíclica com model;
pode reter objetos em excesso quando bytes32 é apenas metadado. Domínios raw
certificate/checker são folhas. Forma canônica alcançável inválida aborta,
sem converter erro em ausência de arestas. A lista é de candidatos a órfãos
no inventário observado; não prova disponibilidade de referências transitivas
ausentes nem autoriza remoção. Não há método de apagar nem flag destrutiva.

Arquivo de lock já precisa existir; GC o abre sem create. Índices, dados do
SQLite e conteúdo/nomes/mtime dos objetos não são escritos. SQLite read-only
pode criar/manter seus sidecars WAL/SHM; teste inicial detectou essa criação,
que não é escrita de eventos ou objetos CAS. Leituras também podem alterar
atime conforme o filesystem. Store/ancestrais continuam sob controle
do operador, como cas/v1; não prometer isolamento contra troca hostil de paths.

## Testes e consequências

Migrar fixtures de novos appends para put real. Vetores históricos com hashes
públicos sem preimagem continuam fixtures de importação SQL/conferência de
replay, sem criar bypass de append em produção. Testes de objetos ausentes
no reader passam a remover objetos depois de um append válido.

Novos testes cobrem cada referência direta/aninhada, domínio incorreto,
ausência/corrupção/symlink/erro, Signer não chamado e seq/head preservados;
CLI cobre dry-run obrigatório, órfãos/referenciados/links conservadores,
temporários, erro de leitura/cadeia e preservação dos arquivos.
Mutantes compilados demonstram os controles, fuzz event_decode exercita
inventário de referências do body e full verify/selftest permanece obrigatório.
Sem dependências novas na TCB ou alteração de formatos/vetores existentes.
