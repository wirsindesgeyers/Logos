# ADR0042 — WITNESS e não-vacuidade

- Estado: aceito para implementação da5.5,2026-10-20.
- Base:12826f4; relacionados:0034,0041.

## Decisão

Evento novo `ConsistencyRecorded` usa corpo v3/tag14:
`[3,14,target,check,model]`. target é o evento Pending (ObligationCreated)
ou Verified da instância mais recente da obrigação na época corrente.
O envelope event/v1 e todos os corpos anteriores são preservados.
Accepted registra testemunha; Rejected/Unknown registra tentativa, nunca
refutação. Certificado e modelo têm bytes iguais, em domínios CAS distintos.
O kernel recompõe proposta/O e exige checker model/v1 ativo.

Prazo determinístico: evento da tentativa deve ter seq ≤ seq(Pending)+1024.
É prazo lógico, não relógio de parede; não promete expirar um ledger ocioso.
Após esse prazo não entram novas tentativas; testemunhas já aceitas persistem.
Reinício não renova a janela. Uma nova proposta é uma nova instância.

Antes de CERTIFY, a testemunha aceita é rechecada e incorporada ao Verified.
Depois de CERTIFY, o evento original permanece byte a byte; consulta efetiva
combina seu status com os eventos anexos e recheca o modelo. Tentativa falha
não apaga testemunha válida anterior. Sem tentativa: Unknown{check:None}.
Índices de citabilidade lógica não mudam: ConsistencyRecorded não é claim,
não cria dependências nem Verified. Consulta de uso no kernel confere
citabilidade atual, consistência e checker; nenhum cache é autoridade.

O critério de ação da5.5 é demonstrado na consulta de elegibilidade por uso,
que a Fase8 deverá consumir. Gateway/token/autorizações8.2 não existem e não
são implementados aqui. ROADMAP distingue essa unidade da integração futura.
FOL aberta, novo produtor de modelos e busca automática ficam fora.

## Consequências

Replay limitado já existente continua síncrono; consulta não é O(1).
Revogação, época, retratação ou perda do contexto impedem uso. Auditor
independente decodifica o corpo e recheca testemunhas Accepted sem importar
kernel/ledger. Spec, vetores independentes, fuzz e mutações são obrigatórios.

A consulta por h(Event) seleciona a instância exata, inclusive quando uma nova
proposta de mesmo h(O) já existe. Uma repróposta não apaga a evidência anterior.
O request WITNESS, que só identifica h(O), continua destinado à instância mais
recente; não transfere evidência entre propostas.
