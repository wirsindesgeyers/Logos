# ADR 0028 — Tseitin canônico e rederivação pelo auditor

- Estado: aceito para implementação da Etapa 4.3
- Data: 2026-10-15
- Base: e738f78
- Relacionadas: 0020, 0024, 0025, 0027

## Contexto

A 4.2 entrega DAG/raízes/refs e a 4.3 exige CNF determinística, proveniência,
h(O)/h(CNF)/encoder_version no evento e rederivação pelo auditor. Todas as
dependências de etapa existem. ObligationCreated guarda artifact SHA-256 cru,
sem versão de CNF; o auditor lê O como CheckedObligation não autoritativa.
Acrescentar essas interfaces faz parte dos entregáveis expressos da 4.3.

## Decisão

1. Clausificar GroundProgram inteiro com equivalências completas de Tseitin.
   Átomos usam variáveis 1..N; nós não Atom recebem variáveis posteriores em
   ordem topológica. Assertar constraints, cada premissa e negação do goal.
   Não assertar a raiz de implicação. Não podar nós, eliminar cláusulas,
   duplicatas ou tautologias. Ordenar literais por inteiro assinado e cláusulas
   lexicograficamente, desempate pela origem; IDs de cláusula são 1-based.
2. ProvenanceMap separa origens Premise(ref), NegatedGoal, DomainAxiom(ref) e
   Definitional. Ref de axioma explícito classifica sua assertiva como DomainAxiom.
   Conservar também domain_support global da transformação: simplificar uma
   igualdade para tautologia não elimina a fonte exigida para usar enumeração.
   Essa lista é conservadora: registra requisitos de contexto do grounder,
   não o conjunto mínimo usado por uma prova. A4.7 deve analisar seu alcance
   junto às origens do certificado e testar precisão; não convertê-la por
   união automática em used. Este mapa ainda não calcula used nem demonstra
   autenticação.
3. DIMACS ASCII estrito de saída cnf/v1; h(CNF)=SHA256(logos/cnf/v1\0 || bytes).
   Nova tag e namespace cnf-v1 do CAS, default 16 MiB. Nenhum decoder DIMACS
   nesta etapa: auditor compara os bytes canônicos que reemite. Limites
   inclusivos por variáveis/cláusulas/literais/bytes, aritmética checada e
   reservas falíveis. Sem resultado parcial, RSS/wall rígidos ou fallback.
4. ground_for_check usa decode_for_check existente, compartilha a análise e
   tradução por referências de fórmulas e h(O), sem construir Obligation selada.
   Não altera bytes ground/v1 nem o contrato de proveniência do gerador.
5. ObligationCreated ganha cnf: Option<CnfRecord>. None preserva corpo v1.
   Some em rota Lrat usa corpo v2 com h(CNF) e encoder_version=1, mantendo
   artifact como SHA-256 cru dos mesmos bytes. Versões desconhecidas, rota
   diferente ou formas alternativas são erros. O inventário exige CNF no CAS
   antes de assinar, e o GC conserva a referência. Envelope/assinatura v1 e
   corpos históricos permanecem byte a byte, sem migração ou reassinatura.
6. Auditor acrescenta dependência workspace logos-ground (já permitida), sem
   logos-ledger/kernel em qualquer aresta. Recalcula O/V e emite CNF novamente;
   confere hash, bytes e artifact cru de cada ObligationCreated/Lrat. Falta de
   metadata, encoder desconhecido ou erro/limite é falha explícita. Autenticação
   e export continuam distinguíveis de rederivação e prova lógica. A contagem
   de provas Verified sem checker permanece até 4.5; não executar LRAT agora.

## Dependências e compatibilidade

Nenhum pacote externo novo. logos-audit→logos-ground é a única aresta runtime
nova; constar em deny.toml. Testes interoperáveis ficam no ledger, invocando
binário separado, preservando a independência inclusive dev/all-features.
O oráculo SAT é Z3 já disponível em Python fora da TCB. Nenhum SAT solver em
runtime da TCB. Os catálogos de tags de hash e namespaces CAS permanecem
distintos (ground/v1 segue somente em memória).

## Consequências

Leitores antigos recusam o novo corpo; leitores novos conservam todos os bytes
antigos. Lrat histórico sem encoder não é auditável como derivação versionada:
verify falha explicitamente; autenticar/exportar não supõe uma versão.
Corpos v2 de época continuam independentes, identificados por sua tag.
Não muda escopo/ordem, critérios ou meta de LOC do roadmap. Critério remoto de
determinismo continua aberto até observar a CI; relatórios locais não o fecham.
