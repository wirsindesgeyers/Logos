# Máquina de estados de claims — Etapa5.1

Tabela normativa para logos-kernel e a futura formalização12.1. Versão da
tabela: 1 (somente memória; não é novo formato wire). Referências: spec
conceitual §3–4, ROADMAP5.1/5.3/5.4/5.9/5.10, ADR0034 e events.md.

## 1. Estados e transições

Claim<S> MUST ter estado fantasma S e construção restrita ao módulo de
transições. O claim e seu SignedEvent<S> MUST NOT ser clonáveis, mutáveis ou
decodificáveis. O recibo é produzido exclusivamente pelo append confirmado;
um Event público e uma assinatura válida não bastam para construí-lo.

| ID | Origem | Operação interna | Destino | Evento único | Precondições |
| --- | --- | --- | --- | --- | --- |
| propose | ∅ | propose | Draft | Proposed | Catálogo válido; writer iniciado |
| submit | Draft | submit | Pending | ObligationCreated | Mesma sessão/época; h(O) atribuído pelo kernel |
| accept | Pending | accept | Verified | Verified | Mesma sessão/época/h(O); Accepted de prova; cited exatas da proposta; used⊆cited |
| refute | Pending | refute | Refuted | Refuted | Mesma sessão/época/h(O); Accepted de contramodelo |
| unknown | Pending | unknown | Unknown | UnknownRecorded | Mesma sessão/época/h(O); outcome Unknown com limite explícito |
| invalidate | Verified | invalidate | Invalidated | Invalidated | Mesma sessão; alvo é h(Event Verified); causas não vazias |

Não há outras transições nesta tabela. Refuted, Unknown e Invalidated são
terminais para esta instância. Enfraquecer a fórmula/declarar hipótese cria
outra proposta, sujeito às regras dos handlers. Refuted→Verified e
Unknown→Verified MUST NOT existir. Rejected MUST NOT virar Refuted.

Draft é a proposta já registrada, ainda sem obrigação. Entrada não registrada
é dado candidato, não Claim<Draft>. PROPOSE5.3 compõe propose e submit,
gerando dois eventos; cada operação da tabela gera exatamente um.

## 2. Vínculos e persistência

O claim conserva h(Proposed), época de origem, citações normalizadas e h(O)
quando atribuído. Cada passo MUST consumir o claim anterior. Intercalar
claims da mesma sessão é permitido. Trocar a sessão/writer MUST falhar antes
de escrever, mesmo com mesma chave ou época. Invalidação permite época
posterior; decisões lógicas exigem a época original ainda corrente.

O novo estado MUST ser publicado somente após Ledger::append retornar com
sucesso. Validação falha não acrescenta evento. Falha de escrita/hash não
devolve novo claim e não permite repetir automaticamente com o anterior:
reabertura/replay resolve commit sem resposta. Evento único por sucesso não
é uma garantia de entrega exactly-once em presença de crash.

## 3. Fronteiras de autorização e extensão

Nenhum construtor/transição é exposto ao agente. Verdict/CheckOutcome brutos
ou eventos externos MUST NOT habilitar uma API pública de emissão.
Os futuros handlers5.3/5.4 são responsáveis por generate/citações/contexto,
checker ativo por hash, bytes/CAS/certificado e used conferidos; os guardas
de h(O)/outcome desta etapa são necessários, mas não substituem esses checks.
Claim<Verified> isolado não prova citabilidade na época presente; consultar
o ledger e as regras de autorização apropriadas.

Rejected mantém Pending e não é uma transição desta tabela. Registro de
tentativas, Unknown mantendo Pending e política de novas tentativas são
da5.4; não se inventa CertificateRejected usando outro evento do catálogo.
Consistência usa o campo existente, sem implementar WITNESS5.5. Recheck/
revogação pertencem à5.10; atualizar esta tabela explicitamente quando essa
etapa acrescentar transições.

## 4. Retratação e cascata (Etapa 5.9, ADR0052)

Retratação é operação de operador (`logos_kernel::retract::Retractions`), nunca
mensagem do agente. Alvo MUST estar Verified (somente autoridade administrativa)
ou Assumption (autoridade ou o KeyId `author` registrado), não retratado. O
evento Retracted carrega a assinatura de autoridade de events.md §4. A cascata é
derivada pela projeção do índice (ledger-index.md §3). Em seguida o kernel usa a
transição `invalidate` desta tabela para cada rota Verified derrubada que não é o
próprio alvo, recuperando o Claim<Verified> da instância registrada. `causes` =
`[h(Retracted raiz), premissa morta imediata]`: a premissa é a de menor h(Event)
entre as used cujo nó já está morto, e a raiz é a da premissa (a do alvo é o seu
Retracted). Raiz já gravada não é recalculada. `reconcile` é idempotente e roda no
arranque do processo: completa registros após crash entre appends. Rotas que caem
só por Verified de checker já revogado nascem mortas e não recebem registro; revogação
suspende e Rechecked rejeitado é raiz como Retracted (§5).

## 5. Revogação e recheck (Etapa 5.10, ADR0056)

Operação de operador (`logos_kernel::recheck::Rechecks`), nunca do agente.
`revoke` exige a autoridade administrativa, checker registrado e não revogado, e
grava CheckerRevoked assinado: rotas desse checker efetivo ficam Suspended (não
citáveis, não mortas), com suspensão transitiva aos dependentes. `recheck` roda o
certificado original de cada rota viva desse checker num checker ativo diferente
e grava Rechecked: Accepted reativa, Rejected invalida e o `reconcile` da §4
registra a cascata com a raiz no Rechecked, Unknown mantém a suspensão. Falha
operacional do checker é erro sem append. `used` registrado não é recalculado.

## 6. Especulação (Etapa 5.8, ADR0058)

Um Pending pode especular sobre outros Pending (`speculative_on`): fica Pending
condicional, nunca citável (P5: nem por PROPOSE nem por ação). Só vira Verified pelo
CERTIFY depois que toda base tem rota Verified ativa (kernel-protocol.md); a partir
daí vale o ATMS das §4–5 pelas arestas `used`.

Base falha: linha invalidada (especulação dela falhou), h(O) refutado, ou nó h(O)
com rotas e nenhuma viva. Todo Pending especulativo não concluído com base falha
recebe `Invalidated { claim: h(Pending), causes: [raiz, h(base)] }` —
`SpeculationFailed(base)` —, onde a raiz é o Refuted da base, a raiz registrada da
base ou da rota morta, ou a rota nascida morta. A varredura é idempotente, em ordem
de seq (a menor base falha decide a causa), e roda após todo Refuted e no fim do
`reconcile` (retratação, recheck, boot). Pending concluído (com Verified ou Refuted
do seu h(O)) não é varrido.
