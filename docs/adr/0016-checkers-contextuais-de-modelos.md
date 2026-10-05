# ADR 0016 — Leitura contextual e checkers de modelos finitos

- Estado: aceita
- Etapa: 2.3
- Data: 2026-10-05
- Relacionadas: 0012, 0014, 0015

## Contexto

P2 exige ler a obrigação dos mesmos bytes cujo hash foi apresentado. A 1.5
implementou apenas geração selada de Obligation (P1), não sua desserialização.
CheckOutcome::Rejected exige Reason com Text canônico; construir diagnósticos
é falível. O avaliador 2.2 recebe Budget e cria seu próprio StepMeter, que não
pode ser reiniciado entre premissas sem violar S2. CAS oferece bytes tipados
verificados por hash, mas suas falhas de I/O não são falsidade lógica.

## Decisão

1. Criar CheckedObligation, distinto do Obligation gerado: leitura contextual
   de obligation/v1, campos privados/accessors, sem Canon/Domain/conversão para
   Obligation. decode_for_check compara hash_raw dos bytes originais, verifica
   o envelope e h(V), referências estritamente crescentes e sort-check de todas
   as fórmulas. Limites de bytes/premissas e nós agregados impedem o orçamento
   por fórmula de multiplicar silenciosamente o total. Não valida proveniência,
   consenso de época ou que o digest esperado foi emitido pelo monitor.
2. Criar ModelChecker::new(&Vocabulary) -> Result<_,CanonError>, preparando
   Reasons canônicos uma única vez por processo em LazyLock<Result<...>>.
   Métodos check_refutation/check_consistency retornam CheckOutcome, sem capturar
   falhas canônicas como valores padrão, sem panic/unwrap ou wall clock.
   Preparação falível é explícita e reutilizável; não emite claims nem Verdicts.
3. Expor eval_on_meter para composição; eval mantém sua API de Budget e latch.
   Cada execução do checker usa um medidor cobrindo bytes, preparação contextual
   e todos os evals. Unknown permanece Unknown; entrada inválida ou premissa
   falsa é Rejected, nunca refutação. Consistência valida goal mas não sua verdade.
   A máquina/pilhas são reutilizadas; identidade de M usa h(V) já conferido no
   reader, sem rehash ou sort-check duplicado na fronteira do evaluator.
4. Métodos *_from_cas carregam/verificam modelo por Hash<FiniteModel>, fazem
   decoder contextual contra V e impõem soma O+M para Budget.size_bytes.
   Ausência é Rejected; erros de CAS são Result::Err(CasError), não falsidade,
   nem Unknown(Time/Size) inventado. Sem interação com ledger/eventos.
5. A dependência runtime obligation -> core fornece Exhausted para limite do
   reader; model -> obligation/ledger usa as APIs existentes. Todas são crates
   workspace já allowlisted em deny.toml. Sem dependência externa runtime nova,
   formato serializado, domínio ou bump de versão. cargo-mutants 27.1.0 é
   ferramenta dev independente da TCB.

## Alternativas rejeitadas

- CanonDecode/From bytes para Obligation: abre a autoridade selada da ADR0012.
- Decodificar/reencodar antes do hash: abandona vínculo aos bytes originais (P2).
- eval com Budget por premissa: restaura passos em vez de consumir saldo (S2).
- Converter erro de Text em Unknown arbitrário ou razão padrão: erro mascarado.
- Prometer hard RSS/wall dentro do checker: sem supervisor físico é falso.

## Consequências e limites

O chamador deve obter h(O) de obrigação gerada no monitor; hash autoconsistente
não é autenticação. Decoder estrutural é entregável da 2.3, não substitui generate.
Metadados de origem/epoch só serão ligados no ledger real da Fase3. Métodos CAS
preservam I/O no tipo público, não implementam retry. Limites de AIR/model são
independentes do budget lógico; mem_bytes não mede RSS, wall_ms é ignorado.
Mutação é medida no arquivo com toda a lógica checker, não em construção de
mensagens/codec já cobertos nas etapas anteriores. Evidência e limitações dos
mutantes/CI remota serão registradas no STATUS, sem alegar prova formal.
