# Plano — Etapa 3.5: CAS integrado ao ledger

## 1. Entradas

Ledger/appender/replay/lock da3.2 e Cas put/get duráveis da0.5 existem.
Catálogo atual, incluindo epoch-body/v2, e CLI com chave pública existem.
Contratos: spec/cas.md, spec/events.md, spec/ledger.md, ADRs0021–0024.
Nenhuma dependência de fatos, kernel ou auditor a implementar nesta etapa.

## 2. Saídas

- `Ledger::open` mantém API e passa a possuir CAS obrigatório em DIRECTORY/cas.
- `Ledger::cas(&self) -> &Cas` e validação obrigatória antes de cada append.
- `EventBody::cas_references() -> BTreeSet<CasReference>` com leitura tipada.
- `ledger_references(directory, public_key) -> Result<LedgerReferences, LedgerError>`
  confere histórico sob lock sem escrever, para a CLI de diagnóstico.
- `Cas::open_existing` sem criação; verificação regular/integridade/limites
  no caminho obrigatório do appender. Nenhum formato serializado novo.
- `logos ledger gc --dry-run DIRECTORY --public-key HEX`, relatório
  determinístico de candidatos a órfãos e temporários, sem remover arquivos.

## 3. Tarefas e arquivos

1. ADR0024/plano/spec fixam inventário de referências e fronteira de confiança.
2. ledger_cas.rs, cas.rs, ledger.rs/error/lib e manifesto: CasReference, checagem
   obrigatória, helper de leitura autenticada e erro de referência ausente.
3. Migrar support/testes/worker de crash/CLI para objetos realmente publicados;
   vetores históricos passam pelo import/replay de fixture sem bypass público.
4. CLI gc.rs/main.rs: inventário cas/v1, hashes, marcação conservadora com
   decoder canon existente, relatório composto antes de imprimir; CLItests.
5. Regressões/integração, mutantes, fuzz oficial de referências no event_decode,
   medição TCB e full verify/selftest. Atualizar STATUS/ROADMAP e commit sem push.

## 4. Critérios → evidência

- Append com hash ausente: suíte pública de integração verifica toda variante
  e referências aninhadas; contador de Signer, seq/head/linhas preservados;
  put seguido de tentativa válida passa. Corrupção/domínio/symlink/I/O recusados.
- GC: processos reais demonstram órfão listado, raízes/links mantidos,
  temporários separados, argumentos sem --dry-run recusados, falhas explícitas
  e objetos CAS/banco principal idênticos antes/depois, incluindo conteúdo/
  nomes/mtime. Excluir somente sidecars WAL/SHM mantidos pelo próprio SQLite
  read-only; não ignorar mudanças em objetos ou no arquivo principal.
- Histórico: vetores22 preservados no replay, crash10k com objetos no CAS e
  oráculo independente atualizado, escala100k/1M ainda conferida integralmente.
- TCB: mutações de omissão/bypass devem falhar em testes compilados; manifesto,
  lints, independência do auditor, dependências e fuzz no runner oficial.

## 5. Riscos e decisões

ADR0024: CAS interno obrigatório evita API antiga sem conferência; não
revalidar disponibilidade de todos os objetos históricos a cada abertura.
GC é diagnóstico fora da TCB e conserva excessivamente links bytes32;
não é prova de ausência de uso por histórico externo ou objeto ausente.
Operador controla paths; não prometer snapshot atômico contra writers CAS
independentes, mudança hostil de diretórios ou exclusão manual. Falha após
put e antes de append pode deixar órfão legítimo. Nenhum GC destrutivo.

## 6. Tamanho

M revisado a partir de P: inventário completo e migração das fixtures/worker,
mais GC conservador e checagem sem bypass, exigem alterações em vários
consumidores. Escopo e ordem mantidos; sem parser de formato de fase futura.

## 7. Evidência local

- Dez testes novos (quatro ledger e seis CLI),78 testes ledger/CLI passaram.
  Matriz remove28 referências uma por vez, exige MissingObject correspondente,
  Signer::sign não chamado, contagem SQL/head/seq estáveis e nova tentativa
  válida após put. Casos principais, Witness e Unknown de consistência, checker
  registrado/revogado e vocab opcional cobertos; domínio não é inferido dos bytes.
- Testes de corrupção/16MiB+1/caminho não regular/symlinks e lock, incluindo
  leitura de evento confirmado só no WAL, sem reescrever banco/WAL, passaram.
- GC real:2 candidatos,3 retidos por fechamento,1 temporário separado;
  mapa/array/hash de raw exercitados, erros sem lista parcial, flags sem
  dry-run ou destrutivas recusadas, objetos CAS/banco principal preservados.
  Quatro testes iniciais detectaram sidecars WAL/SHM criados pelo SQLite
  read-only; a regra de preservação cobre dados/objetos e documenta a
  coordenação do SQLite, sem desativar leitura de WAL confirmado.
- Compilação inicial do teste usou update syntax de struct numa enum;
  corrigida com campos explícitos. Clippy inicial apontou Vec desnecessário
  em fixture; substituído por array. Clippy workspace/all-targets final passou.
- Nove mutantes compilados capturados, nenhum erro de compilação contado
  como detecção. Fontes restauradas; logs/comandos em
  target/cas-integration-mutations/results.json. Reproduzir isoladamente,
  executar `cargo test -p PACOTE --test SUITE FILTRO` e restaurar antes do próximo.

| Mutação | Arquivo | Pacote/suite/filtro que falhou |
| --- | --- | --- |
| Remover conferência antes de sign | ledger.rs | logos-ledger / ledger_cas / every_direct |
| Omitir checker de CheckRecord | ledger_cas.rs | logos-ledger / ledger_cas / every_direct |
| Omitir check opcional de consistência Unknown | ledger_cas.rs | logos-ledger / ledger_cas / every_direct |
| Omitir modelo Witness | ledger_cas.rs | logos-ledger / ledger_cas / every_direct |
| Omitir vocab de EpochAdvanced | ledger_cas.rs | logos-ledger / ledger_cas / every_direct |
| Aceitar presença sem conferir hash | cas.rs | logos-ledger / ledger_cas / corruption_limits |
| Não coletar raízes do histórico | ledger_cas.rs | logos-ledger / ledger_cas / every_direct |
| GC não confere integridade de órfãos | CLI gc.rs | logos-cli / gc / missing_or_corrupt |
| GC ignora links em chaves de mapa | CLI gc.rs | logos-cli / gc / dry_run_lists_candidates |

- Fuzz oficial event_decode/nightly-2026-09-28:99066 execuções/61s, exit0,
  sem crash, sanitizers ativos, max_len4096. Fora do sandbox por ptrace conhecido.
  Inventário também é comparado após roundtrip de body;22 seeds da spec no
  runner existente. Log novo target/event-decode-3.5.log; anterior restaurado.
- TCB Rust9285 próprios +105199 runtime=114484, build-only64508; ledger3209,
  +229 próprios, sem dependência nova na TCB. CLI acrescenta somente canon
  local já existente; nenhum pacote externo novo. SQLite C176466 separado.
- Full `ci/verify.sh --selftest`: VERIFY_OK, exit0; Rust/smokes, manifesto/
  independência/lints/supply-chain/vetores,227 Python90,61s +25 TCB1,77s.
  Autoteste sem avisos; controles bloqueiam pelos diagnósticos corretos.
  Só avisos deny preexistentes. Crash10k/24 SIGKILLs/9999 ACKs sem perda,
  oráculo independente confere objetos CAS;64,90s. Rebuild100k/tabelas5 exatas;
  1M/20000 consultas p99=21866ns (cold build255599ms fora da medida).
- Escala histórica continua fixture de preparação SQL em lote, não teste de
  throughput do appender; novos appends reais cumprem put→append. Vetores22
  anteriores não mudaram; importar histórico não cria API de bypass.
- CI/ARM/fuzz longo não observados. Sem auditor/fatos/políticas/ações novos;
  alterações preexistentes preservadas, sem push.
