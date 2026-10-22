# event/v1 — catálogo, assinaturas e corpos v2/v3

Normativo para ROADMAP 3.1/3.4/4.3; ADRs 0020/0023/0028. MUST/MUST NOT conforme RFC 2119.
Consome canon/v1, air/v1, core/v1 e domínios existentes. A extensão de corpo
mantém os formatos anteriores. O catálogo valida estrutura; verificação de
histórico pertence ao appender/replay e não prova execução lógica do kernel.

## 1. Envelope, identidade e assinatura

```text
Event = [1, 0, seq_u64, epoch_u64, prev, body, kernel_key_bytes32, signature]
prev = [] | [bytes32_hEvent]
unsigned = [1, 0, seq, epoch, prev, body, kernel_key]
h(Event) = SHA-256("logos/event/v1\0" || canon(Event))
message = "logos/sig/event/v1\0" || canon(unsigned)
signature = {"alg":"ed25519", "sig":bytes64}
```

Assinar todos os campos exceto sig; h(Event) inclui sig. Verificar com chave
pública fornecida pelo consumidor e MUST comparar KeyId calculado a kernel_key.
Chave embutida/KeyId não é raiz de confiança. Decoder não verifica assinatura;
Event::verify não demonstra cadeia, autoridade do emissor ou validade lógica.

Genesis MUST ter seq=0, epoch=0, prev=[] e chave do body igual a kernel_key.
Demais eventos MUST ter seq>0 e prev=[digest]. Isso não confere elo/continuidade
com outro evento; somente a etapa 3.2 confere cadeia. EpochAdvanced registra
new_epoch igual a epoch do envelope, sem inferir validade da transição histórica.
Tags/versão/tamanhos/tipos desconhecidos, campos extras e bytes sobrando são erros.

## 2. Domínios e tipos compartilhados

Hashes são bytes32, contextualizados por tipos Rust. h(V), h(O) e h(Formula)
usam tipos existentes; h(cert), h(checker), h(model) usam tags CERTIFICATE,
CHECKER, MODEL existentes, sem dependência de logos-model (evitar ciclo).
Referências cited/used/claim/cause/target são h(Event), não h(Formula).
`kernel_binary` e `artifact` são bytes32 de SHA-256 cru de binários/artefatos;
não recebem tags de objetos inexistentes. Rota é enum fechado: 0 Model,
1 Lrat, 2 Alethe, 3 Lean; somente identidade, sem executar rotas futuras.

```text
CheckRecord = [hO, hCert, hChecker, Budget, CheckOutcome]
Budget, CheckOutcome = envelopes core/v1 existentes
Consistency = [0, optional_check] | [1, hModel, check]
optional_check = [] | [CheckRecord]
Authority = [KeyId, Signature]
Proposal = [text_pt_utf8_bytes, Formula, cites, speculative_on, hV]
```

Text_pt preserva UTF-8 original como bytes, inclusive não NFC; continua tainted.
Não converter para Text canônico, normalizar, interpretar ou promover a política.
Formula é AIR integral; não aplicar sort-check sem V. speculative_on são IDs
opacos bytes32, não referências autenticadas; preservam ordem e duplicatas.
Cites da proposta preservam ordem/duplicatas, tal como proposal/v1.
Demais conjuntos de refs MUST estar estritamente ordenados e sem duplicatas;
`causes` é caminho ordenado e não vazio, preservado, não conjunto. Para a
cascata de retratação da 5.9, o kernel grava exatamente `[h(Retracted raiz),
premissa morta imediata]` (state-machine.md §4); o decoder continua aceitando
qualquer caminho não vazio do catálogo v1.

## 3. Catálogo: body = [1, tag, campos...]

| tag | Rust EventBody | Campos, nesta ordem |
| --- | --- | --- |
| 0 | Genesis | ledger_version_u64, kernel_binary_bytes32, kernel_key |
| 1 | VocabPublished | hV, compatible_bool, authority |
| 2 | Proposed | proposal, author_KeyId |
| 3 | ObligationCreated | hO, route, artifact_bytes32 |
| 4 | Verified | check, cited_refs, used_refs, consistency |
| 5 | Refuted | check, hModel |
| 6 | UnknownRecorded | check, machine_Text |
| 7 | AssumptionDeclared | hV, Formula, scope_Text, author_KeyId |
| 8 | Retracted | target_hEvent, reason_Text, authority |
| 9 | Invalidated | claim_hEvent, causes_refs |
| 10 | EpochAdvanced (corpo v1 sem opcionais) | new_epoch_u64, reason_Text |
| 11 | CheckerRegistered | hChecker, format_Text, authority |
| 12 | CheckerRevoked | hChecker, reason_Text, authority |
| 13 | CertificateRejected | check (corpo v2) |

### Extensão epoch-body/v2 (Etapa 3.4)

```text
body = [2, 10, new_epoch_u64, reason_Text, policy_root, vocab]
policy_root = [] | [bytes32_Merkle_root]
vocab = [] | [bytes32_hV]
```

Pelo menos um opcional MUST estar presente. Quando ambos estão ausentes,
somente `[1,10,new_epoch,reason]` é válido: não existem duas codificações para
o mesmo corpo. Decoder v2 MUST recusar outra tag, aridade, null, digest fora
de 32 bytes, listas de opção com mais de um item e texto além dos limites.
Encoder/decode do corpo v1 preenchem None somente porque campos não existem
nessa versão, sem descartar bytes/campos de entrada.

`policy_root` é compromisso opaco, não assinatura administrativa nem hash
de um objeto Policy. `vocab` identifica um vocabulário, sem implicar tipagem,
compatibilidade ou presença no CAS. Essa transição não autoriza efeitos nem
implementa política da 7.2. New_epoch continua igual ao envelope; append/replay
exigem avanço estrito. Sem ambos opcionais, sign/encode preservam os bytes
históricos, incluindo assinaturas e h(Event).

O bump pertence ao corpo versionado; envelope event/v1, domínios de hash/
assinatura e ledger-sqlite/v1 permanecem iguais. A versão do corpo também
está nos bytes autenticados. Leitor antigo recusa v2 em vez de ignorar seus
campos. Nenhum evento armazenado é migrado ou reassinado implicitamente.
API Rust acrescenta `policy_root: Option<[u8;32]>` e
`vocab: Option<Hash<Vocabulary>>` à variante EpochAdvanced.

Genesis ledger_version MUST ser 1. Verified/Refuted check.outcome MUST ser
Accepted; UnknownRecorded check.outcome MUST ser Unknown, nunca Rejected.
Rejected não vira Refuted. CertificateRejected usa exclusivamente
`[2,13,CheckRecord]` (certificate-rejected-body/v2) e MUST ter outcome Rejected.
Accepted/Unknown, versão1 e campos extras MUST ser recusados. Mantém a claim
Pending e referencia O/certificado/checker no CAS. Nenhum byte histórico muda.
Verified used MUST ser subconjunto de cited (não inferir dependências a partir
texto). Conjuntos são evidência registrada, não checagem do certificado.
Consistency Witness MUST ter check Accepted e mesmo hO do check principal.
Consistency Unknown pode não ter check (nenhuma tentativa registrada), ou ter
check Unknown/Rejected com mesmo hO; Accepted não cabe em Unknown. Unknown
permanece visível, não passa a Witness. Todos os checks efetivamente registrados
contêm hO/hCert/hChecker/budget/outcome. ModelHash não substitui CertificateHash:
ambos identificam mesmos bytes sob tags distintas, verificação dessa relação
exige obter certificado, fora deste decoder.

## 4. Assinaturas de autoridade

VocabPublished, Retracted, CheckerRegistered e CheckerRevoked carregam
assinatura do admin/emissor além da assinatura externa do kernel.
O payload de autoridade é `canon([1, 1, body_without_authority_signature])`;
body conserva seu envelope/tag/campos e substitui Authority por somente KeyId.
Assinatura usa domínio existente `logos/sig/event/v1\0`. Discriminador 1 evita
reutilizar assinatura externa (discriminador 0) ou em outro tipo de body.
`EventBody::authority_message` fornece bytes; consumidor assina via Signer.
`EventBody::verify_authority` confere assinatura e KeyId contra chave pública
esperada. Event::verify confere somente assinatura externa; política de quem
é admin/emissor e revogação das chaves ficam nas etapas futuras.

## 5. Limites e erros

Limites inclusivos fixos: Event wire até 16 MiB; texto tainted ou Text até
64 KiB UTF-8 por campo; até 1024 refs por lista; Formula usa limites AIR padrão.
Caps valem para construção, codificação e decodificação; sem truncamento,
ordenação, deduplicação ou preenchimento silencioso. Conferir max_bytes antes
de parsing na API de bytes. Caps não são garantia rígida de RSS/wall-clock.
Erro estrutural, limite ou criptográfico é erro tipado; não devolve Verified,
Refuted, null ou evento padrão. Nenhuma API deste catálogo emite Verdict.

## 6. API e reprodução

`logos_ledger::{Event, EventBody, CheckRecord, Consistency, Authority,
EventProposal, TaintedText, Route, Certificate, Checker, ModelObject, EventError}`.
EventBody/CheckRecord/Consistency/EventProposal implementam Canon/CanonDecode.
Event fornece `sign(seq, epoch, prev, body, signer)`, `verify(public_key)`,
`decode(bytes)`, Canon/CanonDecode e campos públicos para leitores; sign e verify
validam estrutura. Decodificar não produz autoridade.
Vetores independentes: spec/vectors/events/vectors.json, 22 fixtures públicas
para 13 tags e variantes de consistência/outcome/texto não NFC/opcionais de
época. Os 18 vetores anteriores permanecem byte a byte; quatro novos exercitam
raiz isolada, vocabulário isolado, ambos e época u64::MAX/digests zero.

```sh
(cd py && uv run python ../tools/gen_event_vectors.py --check)
cargo test -p logos-ledger --test events
cargo run -p logos-ledger --example events_smoke
FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 event_decode
```

Vetores e smoke não provam lógica, autoridade de produção ou cadeia histórica.
Desde a3.5, o appender exige objetos CAS antes de assinar; isso não altera
o decoder/assinaturas ou bytes históricos. Ver [ledger-cas.md](ledger-cas.md).


## 7. Extensão obligation-body/v2 (Etapa 4.3)

```text
body = [2,3,hO,1,artifact_raw,[hCNF,encoder_version]]
encoder_version = 1
```

A variante Rust ObligationCreated ganha cnf: Option<CnfRecord>. Some somente
na rota Lrat, com encoder conhecido 1, produz o corpo acima. None usa corpo
v1 sem metadados, preservando exatamente sua representação histórica.
artifact_raw continua SHA-256 cru; hCNF usa logos/cnf/v1\0 e bytes DIMACS
canônicos, conforme [cnf.md](cnf.md). Digest de outro tamanho, aridade, rota,
versão ou metadados vazios são erros; não preencher ou ignorar campos.
A versão do corpo está nos bytes assinados. Envelope/domínio event/v1,
corpos de época e ledger-sqlite/v1 permanecem iguais; sem migração histórica.

O inventário do appender inclui hCNF em cnf-v1 antes de assinar. Presença não
prova derivação; o auditor reemite e confere todos os eventos Lrat, recusando
v1 sem versão em verify. Authenticate/export conservam a distinção entre
cadeia autenticada e tradução/prova checada. Recheck LRAT continua na4.5.
Vinte e três vetores:22 anteriores preservados byte a byte e um corpo novo.

## 9. Consistência anexa — consistency-body/v3 (Etapa5.5, ADR0042)

`[3,14,target,CheckRecord,model]` é ConsistencyRecorded. target/model são
bytes32, com domínios Event/ModelObject. Aridade exata; versões1/2 recusam tag14.
Todos os resultados são tentativas: só Accepted é testemunha; Rejected não é
Refuted, Unknown não é ausência de tentativa. CAS exige O/certificado/checker/M;
certificado tem bytes idênticos ao modelo, domínios distintos.

O kernel liga target ao Pending/Verified da proposta corrente, mesma época/O,
regenera O e usa check_consistency com checker model/v1 ativo. A tentativa deve
ser anexada até seq(Pending)+1024 inclusive. Prazo lógico, sem promessa temporal
em ledger parado; reboot não renova. Testemunha já aceita não expira nessa janela.
Antes de CERTIFY a consistência é incorporada no Verified; depois a consulta
efetiva usa o anexo, sem alterar bytes/assinatura do Verified histórico.
Falha posterior não apaga Witness anterior. Sem tentativa é Unknown{None}.

O índice lógico ignora este corpo: nenhum novo claim/citable/deps. Elegibilidade
por uso no kernel exige Verified citável e Witness contextual rechecada para
Action; uso Logical conserva citabilidade comum. A Fase8 deve consumir essa
consulta, além de seus demais controles. Não há gateway/token nesta etapa.
Auditor independente confere target anterior/O/época e recheca Accepted.

A consulta de consistência por referência Verified preserva a instância exata,
mesmo após uma repróposta de mesmo h(O); isso não transfere evidência para a
nova instância. Requests WITNESS por h(O) dirigem-se à instância mais recente.
