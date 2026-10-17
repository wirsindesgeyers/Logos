# ADR 0033 — Dependências do certificado e suporte finito

- Estado: aceito para implementação da Etapa4.7
- Data: 2026-10-17
- Base: 709dc45
- Relacionadas: 0028, 0029, 0031, 0032

## Contexto

ProvenanceMap4.3 e o fecho histórico4.4 existem. O ledger3.3 já aceita
cited/used e projeta somente used em deps: o teste pode usar essas interfaces
existentes, sem implementar handler, evento novo ou cascata. Não falta
nenhuma dependência da etapa; não há mudança de escopo/ordem.

A lista domain_support global não é used. Igualdades entre representantes e
quantificação finita podem desaparecer da CNF, mas dependem dos axiomas
explícitos de O. Por outro lado, uma prova booleana não precisa herdar todas
as fontes de domínios não usados. Só mapear ClauseOrigin perde hipóteses;
união global indiscriminada inclui dependências sem relação com a prova.

## Decisão

1. Guardar suporte contextual em memória na tradução de cada premissa e do
   objetivo. Percorrer sorts de quantificadores, termos e assinaturas, seguindo
   corpos Defined transitivamente com pilha explícita. Para cada sort usado,
   conservar as referências de fecho e nomes únicos já encontradas em O.
   Não buscar/autenticar fontes Fact/Policy nem criar premissas novas.
2. Cada assertiva original da CNF recebe o suporte de sua tradução. Definições
   de Tseitin são equivalências proposicionais, sem premissas próprias. A
   assertiva de constraints conserva suporte global se não for True: inclui
   as hipóteses de todas as guardas globais, inclusive Defined e enum vazio.
   É aproximação sintática conservadora, não núcleo mínimo nem análise de
   necessidade semântica; constraints permanece uma assertiva indivisível.
   Provas sem assertivas dependentes do domínio não herdam domain_support.
3. ProvenanceMap liga h(O) e h(CNF) ao mapa privado, com suporte por cláusula;
   LratReport privado liga IDs realmente aceitos a h(CNF) e h(certificado).
   used_premises recebe relatório aceito, mapa, h(O) esperado e Budget.
   Confere identidades, IDs originais positivos/crescentes/no intervalo;
   une somente origens Premise/DomainAxiom e suporte das cláusulas usadas.
   NegatedGoal/Definitional não inventam PremiseRef. Saída é conjunto ordenado
   privado com identidades O/CNF/certificado para consumo futuro.
4. Inconclusivo/rejeitado, mistura de contextos, ID inválido e limites devolvem
   erro explícito; nenhuma lista parcial. A checagem LRAT e a projeção têm
   orçamentos separados, explicitamente passados; cada execução usa um único
   StepMeter. Reservas falíveis e limites por passos/memória/tamanho; sem RSS
   ou wall rígidos no código interno. Não confundir o resultado com Verified.
5. Metadados novos são somente memória. Não muda nenhum byte ground/v1,
   cnf/v1, LRAT, evento, domínio de hash ou encoder_version; vetores antigos
   continuam obrigatórios. Não há parser/decoder novo. Estender fuzz existente
   e demonstrar mutações compiladas; listar novos fontes no manifesto TCB.
6. logos-lrat acrescenta aresta runtime ao logos-obligation já permitido;
   somente testes acrescentam logos-lrat→logos-air e
   logos-ledger→logos-ground/logos-lrat. Nenhum pacote externo
   novo. Independência do auditor continua sem ledger/kernel, inclusive dev.

## Consequências

used é a união do fecho do certificado com suporte da transformação que esse
fecho alcançou; pode conter mais hipóteses que um núcleo semântico mínimo.
O teste 5 citadas/2 usadas usa política booleana e deve produzir exatamente
as duas no índice real. Testes separados cobrem domínio irrelevante, axioma
que vira tautologia, sorts separados, Defined transitivo e guardas globais.
O gerador continua incapaz de incluir uma premissa omitida. Registro e
validação autorizadora no kernel5.4 e cascata5.9 permanecem fora do escopo.
