# Auditor independente e audit-stream/v1

Normativo para ROADMAP3.6; ADR0025. Não emite claims/tokens e não depende de
logos-ledger/kernel, inclusive via dependências dev/features. Usa canon/crypto,
checkers e rusqlite fixados no workspace. Não usa cache/index.sqlite3.

## Comandos e confiança

```sh
logos-audit verify LEDGER --cas CAS_ROOT --public-key ED25519_HEX [--cake-lpr BIN] [--rust-checker HASH_HEX]
logos-audit export LEDGER --public-key ED25519_HEX > events.logos
logos-audit verify events.logos --cas CAS_ROOT --public-key ED25519_HEX
ci/check-audit.sh
```

LEDGER pode ser diretório com ledger.sqlite3, arquivo SQLite ou stream desta
spec. Chave externa obrigatória: 64 dígitos hexadecimais/32 bytes Ed25519.
KeyId embutido não é âncora. Uma chave fixa por histórico; sem rotação13.1.

SQLite MUST ser aberto read-only/query_only com trusted_schema=OFF, sob lock
exclusivo sem espera de writer.lock **existente**, mantido durante leitura.
MUST validar application_id/user_version/schema de ledger-sqlite/v1 e ler
snapshot incluindo WAL confirmado. Não criar base/lock nem inicializar schema.
SQLite pode criar/manter seus sidecars WAL/SHM; isso não é escrita de eventos.
Diretórios/ancestrais controlados pelo operador e inodes estáveis são premissas,
como no appender. Não prometer proteção contra troca hostil concorrente do SO.

## Verificação

Decoder próprio MUST conferir canonicidade, limites/aridades/versões de
[events.md](events.md), todos os13 corpos e epoch-body/v2. Conferir assinatura
externa Ed25519 estrita, KeyId contra chave externa, Genesis, seq contíguo
0..i64::MAX, coluna SQL igual ao seq assinado, prev=h(Event integral anterior)
e épocas estritamente avançadas apenas por EpochAdvanced. Sem reparação.
Assinaturas administrativas têm estrutura conferida, mas política/chaves dos
administradores não estão configuradas nesta etapa; não alegar sua autoridade.

verify MUST buscar cada referência CAS direta e aninhada do catálogo, validar
tipo de arquivo regular (recusar links estáticos na raiz/domínio/shard/objeto),
limite cas/v1 e digest original sob domínio correto. Ausência/erro é falha,
nunca objeto padrão. Raw artifact/kernel_binary/policy_root e IDs/referências
de eventos não são objetos CAS. V de uma obrigação rechecada também é buscado
e conferido, ainda que não publicado separadamente no ledger.

Cada Refuted MUST ser rechecado pelo ModelChecker independente do transporte:
bytes de O/h(O) conferidos, V correspondente decodificado, certificate/model
obtidos nos dois domínios e bytes exatamente iguais, h(M) conferido novamente
pelo checker, Budget original com O+M e medidor compartilhado. Exigir Accepted.
Rejected/Unknown são falhas de recheck, não refutação nem autorização.
Cada Consistency::Witness explícito recebe a mesma checagem em modo consistência.
Unknown não recebe promoção, reprodução forçada ou limite maior.

O conteúdo de h(checker) é conferido por integridade e **não executado**; recheck
usa o checker compilado no auditor, não confiança no rótulo/hash do produtor.
Provas Verified ficam em proofs_unchecked até4.5; assinatura/objetos presentes
não provam sua validade lógica. Nenhum certificado LRAT/Alethe/Lean executado.
Autenticação não confere proveniência/citabilidade das premissas, fidelidade
ou política administrativa do kernel futuro.

Sucesso: exit0, AUDIT_OK com events, refutations_rechecked,
consistencies_rechecked, proofs_unchecked, failures=0, head, cnf_rederived,
lrat_rust_rechecked, lrat_cake_rechecked, invalidated_by_recheck (5.10) e
facts_verified (6.1: FactRegistered com fact/v1 no CAS e assinatura do emissor
reverificada) e trust_configs (6.2: TrustConfigPublished reaplicados na ordem, com
versão crescente; cada fato precisa de `Trusted(emissor, tópico)` vigente no ponto
do registro, trust.md §4; 6.4: fato de emissor já revogado e revogação repetida
são falhas).
Rota LRAT cujo certificado falha no recheck só é aceita se um Rechecked Rejected
posterior a invalidar (incidente contado, não escondido); Rechecked precisa casar
com h(O)/h(certificado) da rota, usar checker permitido e ter Accepted/Rejected
reproduzidos pelo auditor. Falha: exit1,
AUDIT_FAILED failures=1 com posição/diagnóstico; para na primeira falha.
Não publica contagens de sucesso ou export parcial por erro de autenticação.

## Transporte audit-stream/v1

Novo transporte v1, independente de SQL; event/v1 e todos os hashes/assinaturas
existentes permanecem idênticos. Cada arquivo contém:

```text
ASCII("LOGOS-AUDIT/v1") || 0x0a || frame*
frame = u32be(len(canon(Event))) || canon(Event)
1 <= len <= 16777216
```

Header sozinho representa histórico vazio. EOF é válido somente entre frames;
header/frame incompleto, tamanho0/acima do teto, lixo extra ou evento inválido
MUST falhar. Sequência/hashes do evento autenticam a ordem, sem renumeração,
normalização ou reassinatura. Remover sufixo **inteiro** deixa prefixo válido:
exige âncora externa da13.2, não coberta nesta etapa.

export MUST autenticar histórico completo antes de escrever no stdout. Spool
temporário exclusivo0600 permite uma passagem da fonte e memória por evento;
relatório em stderr declara authenticated_only=true e zero rechecks. Export
sem CAS não implica disponibilidade ou validade lógica dos certificados.
Erro de escrita no consumidor pode deixar saída parcial: exit não zero exige
descartar essa saída. Saída exportada pode ser verificada diretamente com --cas,
sem ter o banco SQLite original. O binário usa a biblioteca SQLite apenas quando
a entrada for SQLite; a representação exportada não exige esse backend.

## APIs, vetores e reprodução

logos_audit::{authenticate, verify, export, Report, AuditError}. authenticate
confere apenas estrutura/cadeia/assinatura; não contar como recheck lógico.
Decoder independente aceita os22 vetores históricos de events. Fixture Python
em vectors/audit/vectors.json tem8 eventos,4 refutações distintas e1 testemunha;
contém os bytes de objetos e stream para reprodução por outra linguagem.

```sh
(cd py && uv run python ../tools/gen_audit_vectors.py --check)
cargo test -p logos-audit
cargo test -p logos-ledger --test model_checker
ci/check-audit.sh
ci/check-audit-independence.sh
```

O guard registra cargo tree em target/audit-dependencies.txt e percorre metadata
--all-features com todas as arestas inclusive dev/aliases/ponte externa. Não mover
ledger/kernel para dev ou feature para contorná-lo. Leitor de modelos refatorado
não introduz parser TCB; usa decoders contextuais existentes e mesmo wire.


## Derivação CNF (Etapa4.3)

Para todo ObligationCreated/Lrat, verify requer CnfRecord/encoder_version=1,
resolve O/V/CNF no CAS, confere hashes, faz leitura contextual não autoritativa
de O e reemite ground→Tseitin→DIMACS. Compara bytes, h(CNF) com domínio e
artifact SHA-256 cru. Falta de metadados em evento Lrat v1, versão desconhecida,
limite/erro de tradução ou divergência falham sem saída de sucesso parcial.
Outras rotas não são reinterpretadas como CNF. Campo cnf_rederived conta
apenas derivações efetivamente reemitidas; authenticate/export mantêm zero.
O auditor depende de logos-ground, sem ledger/kernel inclusive em dev.
Conferir tradução não valida prova LRAT, autoridade ou used; proofs_unchecked
continua explícito até4.5. Contrato em [cnf.md](cnf.md)/ADR0028.


## Anexos de consistência (5.5)

O decoder independente aceita ConsistencyRecorded/v3/tag14 e exige target
anterior do tipo ObligationCreated ou Verified, mesma época e h(O). Accepted
recheca os bytes O/M/certificado pelo ModelChecker; Rejected/Unknown não
contam como testemunha nem como Refuted. O relatório conta verificações de
consistência aceitas (eventos, não modelos únicos), inclusive anexos tardios.
O auditor não depende de logos-kernel/logos-ledger nem executa código do CAS.


## Integração MCP5R.1 (ADR0045)

CLI `--rust-checker HASH_HEX` pode repetir; encaminha lista explícita para
verify_with_checkers já existente. A lista vem do operador, nunca dos hashes
no ledger. Cada hash são32 bytes/64 dígitos hexadecimais. Export recusa essa
opção, pois autenticação de stream não é recheck lógico. LRAT rederivado
continua exigindo Rust e cake_lpr. Sem novo formato, autoridade ou dependência.

O lock exclusivo de SQLite continua obrigatório; kernel ativo mantém seu
writer.lock. O MCP captura um prefixo via QueryHead/Subscribe, codifica o
audit-stream/v1 existente e fornece esse arquivo ao auditor, sem abrir o
SQLite em Python. O relatório precisa corresponder ao head e número de
eventos capturados; export é rechecado com o CAS antes da entrega.
