# Plano de fase — Fase4: camada1/LRAT

Fonte: ROADMAP Fase4; estado atual em STATUS.md. Etapa4.1 genérica concluída localmente;
full verify/selftest VERIFY_OK. CI/ARM/fuzz longo ainda não observados.
Contrato4.1 ajustado pelo mantenedor (ADR0026, opção A): [plano](etapa-4.1.md) e
[ADR0026 aceito](../adr/0026-contrato-de-fragmentos-e-fontes-de-fechamento.md).
Contrato4.2 ajustado antes do código por escolha explícita de A no
[ADR0027 aceito](../adr/0027-contrato-de-grounding-e-determinismo.md):
bytes ground/v1 na4.2, determinismo DIMACS integrado preservado na4.3.
[Plano4.2](etapa-4.2.md); ordem preservada; implementação4.2/full verify/selftest demonstrados localmente,
critério de determinismo ainda aguarda CI remota.

## Ordem e caminho crítico vigentes

4.1 (fragmento/axiomas) →4.2 (grounding) →4.3 (CNF/proveniência) →4.4
(checker LRAT) →4.5 (cake_lpr/auditor) →4.6 (produtor SAT) →4.7 (used).
Recebe AIR/Vocabulary/Obligation/checkers de modelos, CAS e auditor3.6 prontos
localmente; o critério de independência remota da3.6 continua pendente.

A decisão A separou mecanismos genéricos e integrações.4.1 entrega
classificador e candidatos; fontes reais e axiomas em O/used/deps continuam
obrigatórios na6.1 (Fact),7.3 (Policy/completion) e7.5 (projeções).4.2 exige
axiomas explícitos;5.4 recusa fontes não resolvidas no uso autorizador.
Não há ciclo7.3→7.5→7.3: o teste de política roda antes da integração de projeção.

## Interfaces previstas

- 4.1: classificação e axiomas; a fronteira entre candidato lógico e premissa
  autenticada precisa respeitar source payload ≠ evento citável.
- 4.2: PropFormula/tabela de átomos; sem fecho implícito fora de O.
- 4.3: CNF e ProvenanceMap determinísticos, vinculados a h(O).
- 4.3/4.7: preservar suporte de domain_axioms da transformação, mesmo quando
  fecho/igualdade virarem tautologias; não perder fontes porque suas fórmulas
  não geraram cláusulas SAT úteis. Used real continua pendente nessas etapas.
- 4.4/4.5: CheckOutcome de prova, sem converter Rejected em Refuted.
- 4.6: candidato LRAT/modelo; produtor permanece fora da TCB.
- 4.7: used extraído de certificado/proveniência; kernel5.4 registra depois.

## Paralelismo e marco

4.1 genérica foi autorizada explicitamente. Não antecipar integrações das
fases seguintes; cada etapa recebe plano próprio. Marco: Verified com certificado
LRAT rechecado por checkers independentes e pelo auditor, bytes de O/CNF
vinculados e dependências normativas extraídas. Documentação de bloqueio
não demonstra esse marco nem encerra etapas da Fase4.


4.3 implementada/verificada localmente; CI de determinismo pendente: [plano](etapa-4.3.md)/[ADR0028](../adr/0028-tseitin-canonico-e-rederivacao.md).
CNF, registro versionado/CAS e rederivação são entregáveis da própria etapa;
nenhum checker LRAT antecipado. Determinismo remoto continua pendente.
