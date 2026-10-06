# event/v1 — catálogo e assinaturas de eventos

Normativo para ROADMAP 3.1; ADR 0020. MUST/MUST NOT conforme RFC 2119.
Consome canon/v1, air/v1, core/v1 e domínios existentes. Novo formato começa
em v1; formatos anteriores não mudam. Persistência/appender/índices/kernel
não fazem parte desta etapa.

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
`causes` é caminho ordenado e não vazio, preservado, não conjunto.

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
| 10 | EpochAdvanced | new_epoch_u64, reason_Text |
| 11 | CheckerRegistered | hChecker, format_Text, authority |
| 12 | CheckerRevoked | hChecker, reason_Text, authority |

Genesis ledger_version MUST ser 1. Verified/Refuted check.outcome MUST ser
Accepted; UnknownRecorded check.outcome MUST ser Unknown, nunca Rejected.
Rejected não vira Refuted; evento CertificateRejected só entra na 5.4.
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
Vetores independentes: spec/vectors/events/vectors.json, 18 fixtures públicas
para 13 tags e variantes de consistência/outcome/texto não NFC.

```sh
(cd py && uv run python ../tools/gen_event_vectors.py --check)
cargo test -p logos-ledger --test events
cargo run -p logos-ledger --example events_smoke
FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 event_decode
```

Vetores e smoke não provam lógica, autoridade de produção ou cadeia histórica.
