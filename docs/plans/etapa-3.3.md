# Plano — Etapa 3.3: índices e consultas do ledger

## 1. Entradas

Ledger persistido e verificação integral da 3.2, Event/EventBody da 3.1,
LedgerView/PremiseRef/LedgerEntry da 1.5 e reader contextual da 2.3.
CAS/Vocabulary/Formula já existem. Nenhuma dependência ausente.
ROADMAP 3.3 e spec conceitual §3–4; ADR 0022 fixa a projeção.

## 2. Saídas

`LedgerIndex::open(directory, public_key)` e `Ledger::index()` conferem a
cadeia e constroem um snapshot SQLite privado, com claims/deps/citable.
`status`, `is_citable(reference, epoch)`, `formula_of(reference, cas)` e
`dependents` consultam esse snapshot; referências são h(Event), não h(O).
`snapshot(cas, references, scope)` produz LedgerView com fórmulas/vocabulários
autenticados e escopo explícito. `logos ledger rebuild-index DIRECTORY
--public-key HEX` reconstrói e compara o índice persistido.
O comando usa somente chave pública; nenhuma chave privada é necessária.

## 3. Tarefas e arquivos

1. ADR 0022/spec/ledger-index.md/vetor de layout: índices derivados separados
   em index.sqlite3; ledger-sqlite/v1 e event/v1 continuam iguais.
2. src/index*.rs: projeção determinística, fechamento de invalidação por deps
   usados, consultas, comparação e publicação atômica do cache; manifesto TCB.
3. Adapter de leitura de h(V) de O em logos-obligation, reutilizando envelope
   estrito; fuzz obligation_decode e regressões. Resolver CAS valida bytes,
   hash, AIR/sorts e O completo; nunca constrói Obligation selada de entrada.
4. LedgerIndex/novo snapshot LedgerView, consultas reais e integração com
   generate. Ledger::index empresta o writer; snapshot não acompanha appends.
5. logos-cli: binário logos/rebuild-index, argumentos estritos e erros claros;
   nenhuma dependência externa nova. rusqlite habilita somente backup.
6. Testes de comportamento/mutações, smoke público e CLI; script release de
   escala com cadeias reais assinadas de 100k/1M eventos. Fixture de escala
   agrupa INSERTs para preparar dados, sem medir throughput/durabilidade do
   appender. Comparar todas as tabelas/consultas após apagar/recriar cache.
7. Fuzz oficial, medição TCB, full verify/selftest sem execução concorrente,
   STATUS/ROADMAP/README e commit sem push.

## 4. Critérios → evidência

- Reconstrução com 100k eventos: ci/check-ledger-index.sh, igualdade integral
  de tabelas ordenadas e consultas antes/depois da remoção do cache.
- is_citable p99 <1ms com 1M eventos: mesmo script, API pública real sobre
  snapshot derivado da cadeia assinada, consultas positivas/negativas/épocas
  erradas, amostragem individual e artefato JSON com tamanho e distribuição.
- LedgerView: geração real a partir de Assumption/Verified, CAS inválido/
  ausente, hashes/tipos/tipagem/escopos recusados, Unknown/Refuted/Pending
  não citáveis, snapshot estável, deps usados distintos de cited.
- CLI: teste de processo com chave pública, rebuild igual/diferente/ausente,
  erro de chave/argumentos/falha de armazenamento com saída não zero.
- TCB: testes falham sob mutações compiladas de status/deps/época/hash/escopo;
  manifesto/lints/fuzz e verificação completa.

## 5. Riscos e decisões

ADR 0022: replay linear e índices em memória privados; index.sqlite3 é cópia
reconstruível para comparação, nunca raiz de confiança. Lock impede outro
writer enquanto Index estiver vivo. A cópia derivada usa SQLite backup,
arquivo temporário, fsync/rename/sync do diretório. Abertura não reutiliza cache
como autoridade. Memória cresce com os eventos; p99 não inclui reconstrução.
V/O são conferidos ao resolver fórmulas, sem antecipar put→append/GC da 3.5.
Época limita-se à igualdade da 1.5; janelas de fatos ficam na 3.4.
Invalidação projetada não escreve eventos: cascata registrada/kernel continuam
futuros. Sem UI, ações, certificados rechecados, HSM ou transparência.

## 6. Tamanho

M conforme roteiro. Implementação da LedgerView é fronteira confiável e fica
no crate TCB existente, apesar de 3.3 não carregar marca TCB no título.

## 7. Evidência local

- 14 testes de índices, um do novo acesso ao contexto e dois processos CLI.
  Gerador real usa Assumption/Verified e objetos CAS; Unknown/Pending,
  referência ausente/não preparada, escopo direto/transitivo, tipagem,
  hash/integridade, V/O malformado/ausente, epoch u64::MAX, locks, cache
  adulterado/ausente/inválido e invalidação por deps usados cobertos.
- Primeira escala: reconstrução de 100000 eventos preservou todas as colunas/
  linhas ordenadas das cinco tabelas após remover index.sqlite3. Um milhão de
  eventos assinados e conferidos: 20000 consultas, 5142 positivas/14858
  negativas, p50=10210ns, p99=27042ns, máximo571307ns. Build frio255921ms;
  p99 se refere ao SQLite privado em memória, não à abertura. Fixture assinada
  agrupa INSERTs em transação única; não é medição de throughput/durabilidade
  do appender. O script oficial repete tudo na verificação completa e CI.
- Onze mutantes manuais compilados capturados por asserções, baseline passou,
  fontes restauradas. Logs/resultado locais em target/index-mutations/.
  Reproduzir cada alteração isoladamente e executar o filtro abaixo com
  `cargo test -p logos-ledger FILTRO` (reader usa logos-obligation); restaurar
  a fonte antes da próxima alteração/verificação/fuzz.

| Mutação | Arquivo | Filtro que falhou |
| --- | --- | --- |
| Tornar Unknown citável | index_project.rs | statuses_dependencies |
| Registrar cited em lugar de used | index_project.rs | statuses_dependencies |
| Remover fechamento transitivo da invalidação | index_project.rs | retracting_unused |
| Inverter condição de checker revogado | index_project.rs | explicit_invalidation |
| Ignorar época corrente | index.rs | epochs_do_not |
| Mapear Invalidated para Verified | index.rs | retracting_unused |
| Ignorar scope divergente | index.rs | scope_is_exact |
| Usar True em lugar do goal real de O | index_resolve.rs | statuses_dependencies |
| Omitir conferência de h(O) no header | decode.rs (obligation) | context_hash_matches |
| Relatar Equal para cache diferente | index_store.rs | cache_tampering |
| Omitir sort-check da fórmula embutida | index_resolve.rs | ill_sorted_embedded |

- Fuzz oficial obligation_decode, nightly-2026-09-28, 92653 execuções/61s,
  exit0, sem crash, sanitizers ativos. Fora do sandbox por ptrace já identificado
  em 3.2. Target cobre o header novo e decoder completo, hashes divergentes,
  fixtures válidas e perturbações. max_len4096, sem alegação de ≥1h remota.
  Log desta sessão em target/obligation-decode-3.3.log; log anterior restaurado.
- TCB Rust medida: 8974 próprios +105199 runtime=114173, build-only64508;
  ledger2937, obligation572. +739 próprios, nenhum pacote externo novo.
  SQLite C continua176466 LOC medidos separadamente (não incluídos no Rust).
- Erros iniciais corrigidos: API DatabaseName inexistente/FromSql u64,
  fixture usando canon_hex em vez de canonical_hex e três needless_borrows
  em testes/exemplo. Feature cache descartada por exigir hashlink; somente
  backup habilitado. Nenhum hashlink foi instalado ou entrou no lockfile.
- Full `ci/verify.sh --selftest`: VERIFY_OK, exit0; workspace Rust, todos os
  smokes, crash10k/24 SIGKILLs (9998 ACKs, zero perda), manifesto/independência/
  lints/supply-chain/vetores e controles negativos pelo diagnóstico correto.
  227 Python em89,58s +25 TCB em1,34s; autoteste sem avisos de tar. Avisos deny
  anteriores de licenças não encontradas e syn2/3, sem falha.
- Escala repetida dentro dessa verificação: 100k/tabelas5 exatas, build24588ms;
  1M/20000 consultas, p50=15580ns, p99=36573ns, máximo81136ns, build252133ms.
  Artefato target/ledger-index-scale.json, ligado a upload por arquitetura na CI.
  Dataset reutilizado passou novamente pelo replay integral, sem confiar no
  contador de linhas usado somente para evitar gerar a fixture de novo.
- CI/ARM/fuzz longo não observados. Sem fatos/políticas/tokens/kernel/auditor
  novo ou alteração dos wires anteriores. Memória, fuzz-src alheios, logs e
  mutants.out preexistentes preservados fora do commit.
