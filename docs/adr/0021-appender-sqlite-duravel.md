# ADR 0021 — Appender SQLite durável e exclusivo

- Estado: aceito
- Data: 2026-10-15
- Escopo: ROADMAP 3.2 (TCB)

## Decisão

Usar rusqlite =0.40.2, sem features padrão, com bundled e limits. O roteiro
já fixa rusqlite. libsqlite3-sys 0.38.2 compila SQLite incluído no pacote,
sem depender da versão do SO ou bindgen. Dependências runtime novas:
rusqlite, libsqlite3-sys, bitflags, smallvec, fallible-iterator e
fallible-streaming-iterator; cc, find-msvc-tools, shlex, pkg-config e vcpkg
são de compilação. Versões transitivas/checksums fixados no Cargo.lock;
allowlist em deny.toml. Não habilitar extensões carregáveis, funções custom
ou SQL fornecido pelo consumidor. TCB própria continua forbid(unsafe_code);
driver/FFI/SQLite C fazem parte da confiança transitiva.

ledger-sqlite/v1 tem application_id LOGS, user_version 1 e uma única tabela
STRICT events(seq INTEGER PRIMARY KEY, wire BLOB), com checks normativos em
spec/ledger.md. Bytes são o event/v1 existente, sem novo codec/tag/hash.
Conferir schema exato, sem triggers/views/tabelas extras; nunca migrar ou
reparar automaticamente. Não criar índices derivados da 3.3.

Ledger::open recebe diretório e referência ao Signer; captura a chave pública
esperada, verifica todo histórico e mantém lock exclusivo std::fs::File::try_lock
em writer.lock até fechar a conexão. Canonicalizar diretório faz aliases
convergirem; nunca apagar/substituir o arquivo de lock. Criar somente o último
diretório (pai existente), fsync do pai e diretório. SQLite usa NOFOLLOW.
O diretório deve ser controlado pelo dono em filesystem local; lock é
cooperativo, não impede adulteração por quem ignora o protocolo ou troca inodes.

WAL + synchronous=FULL, fullfsync/checkpoint_fullfsync=ON, trusted_schema=OFF;
ler de volta a configuração para detectar fallback/pragma ignorado. BEGIN
IMMEDIATE envolve abertura/replay e cada INSERT explícito. Não expor conexão,
lock ou modo de inserir eventos arbitrários. Seq/prev/epoch são calculados,
assinatura devolvida pelo Signer também é verificada antes da escrita.
Head é atualizado/publicado somente após COMMIT e sync_all do diretório.
Erro de persistência envenena o handle; append/head exigem reabertura, pois
erro depois do commit não prova rollback. Erro de body/assinatura anterior à
persistência não consome seq. Sem retry automático ou duplicação silenciosa.

Replay linear verifica seq contíguo desde Genesis, coluna seq igual ao campo
assinado, prev igual a h(evento completo anterior), assinatura contra chave
externa fixa e continuidade de epoch. EpochAdvanced deve aumentar epoch;
demais bodies conservam a época corrente. Isso só define o cabeçalho do
appender; citabilidade/janelas/revogação ficam na 3.4 e fases posteriores.
Seq de armazenamento limitado a i64::MAX; event/v1 continua u64.

## Alternativas e limites

- SQLite do SO varia entre máquinas e exige dependência nativa no runner.
- NORMAL perde durabilidade em queda de energia; FULL é requerido.
- Lock apenas por transação deixa dois escritores abrirem simultaneamente.
- Checkpoint assinado é otimização futura; v1 sempre confere tudo.
- CAS existe como dependência pronta, mas ordem put→append/refs é da 3.5.
- Não detectar truncamento de sufixo inteiro/substituição por histórico válido
  sem âncora externa (13.2); não implementar HSM/rotação ou política/admin.
- SIGKILL testa processo, não falha física de energia/disco. Durabilidade
  depende do VFS/SO/filesystem honrar sync.
- Medidor atual conta Rust, não SQLite C: reportar separadamente LOC C do
  amalgamation bundled, sem apresentar a soma Rust como TCB completa.
  libsqlite3-sys 0.38.2 inclui SQLite 3.53.2; `tokei --types C` em
  `sqlite3/sqlite3.c` conta 176466 LOC de código (269376 linhas totais).
  É uma cota de fonte, inclui configurações que o build pode excluir;
  não representa prova formal nem contagem de binário.
- Não há novo parser próprio: Event::decode/Canon permanecem exercitados por
  event_decode. Parsing de páginas SQLite pertence à dependência transitiva.

Referências: [rusqlite](https://docs.rs/rusqlite/0.40.2/rusqlite/),
[SQLite synchronous](https://www.sqlite.org/pragma.html#pragma_synchronous),
[locks de File](https://doc.rust-lang.org/std/fs/struct.File.html#method.try_lock).
