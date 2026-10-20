# ADR 0041 — Certificação e recuperação privada de Pending

- Estado: aceito para implementação da5.4
- Data: 2026-10-19
- Relacionadas: 0026, 0030, 0033, 0038, 0040

## Decisão

1. CERTIFY/REFUTE só operam obrigações criadas por PROPOSE e recuperadas
   privadamente do replay autenticado. Conferir par de eventos, proposta,
   época, ausência de conclusão e rederivação com citações ainda elegíveis.
   Nunca aceitar um Pending/CheckOutcome recebido do cliente.
2. Registry combina implementação local conhecida com eventos de registro
   assinados por autoridade local. Hash Rust identifica o executável em
   execução; cake mantém identidade fixada e revalidação por execução.
   Um registro no ledger não carrega nem executa código do CAS. Revogação
   não pode ser revertida por novo registro nesta etapa.
3. CERTIFY rederiva CNF e proveniência de O. Rust LRAT confere o certificado
   inteiro e fornece used; cake, quando selecionado, também deve aceitar.
   Fontes opacas de enumeração continuam recusadas conforme ADR0026.
4. Accepted habilita Verified com `Consistency::Unknown { check: None }`.
   Isso é ConsistencyUnknown explícito, não testemunha nem autorização de
   ação. WITNESS e tratamento de não-vacuidade ficam na5.5.
5. Rejected usa tag13 com corpo `[2,13,CheckRecord]`
   (`certificate-rejected-body/v2`) e exige outcome Rejected. O envelope,
   assinatura e domínio event/v1 permanecem; nenhum vetor histórico muda.
   O evento preserva Pending e não é refutação. UnknownRecorded exige Unknown;
   tentativa inconclusiva não autoriza, mas pode ser repetida sem política
   automática de esgotamento nesta etapa.
6. REFUTE só emite Refuted com Accepted do ModelChecker ativo sobre bytes
   exatos O/M. Modelos são também armazenados no domínio certificate/v1 para
   CheckRecord, sem confundir os dois hashes/domínios.
7. Nenhuma dependência externa nova. Kernel passa a consumir os crates
   internos ground/lrat/model já existentes e permitidos. Novos fontes são
   listados no manifesto; decoder ampliado usa fuzz event_decode existente.
   Auditor é dependência somente de testes do kernel; a independência inversa
   (auditor sem kernel/ledger, inclusive dev) continua obrigatória.

8. PROPOSE no handler5.4 registra CNF/obligation-body/v2 já existentes quando
   elegível para LRAT; o handler mínimo5.3 conserva sua rota histórica. Auditor
   independente aceita hashes Rust somente em lista explícita do operador e
   continua rechecando com Rust/cake. Nenhum binário CAS é executado.
9. STATE15–16 usa camada1/ConsistencyUnknown nesta etapa, preservando as
   dependências de Alethe14 e WITNESS5.5; sem antecipá-las por causa do exemplo.

## Consequências

Replay e regeneração recusam históricos incompletos, épocas antigas e
citações que perderam elegibilidade. Rejeição nunca altera a verdade da
claim. Auditor independente aprende o novo corpo sem depender de kernel ou
ledger. Identidade do executável Rust é específica do build; um novo build
requer registro explícito da nova identidade.
