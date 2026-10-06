# ADR 0017 — produtor Z3 contextual fora da TCB

Estado: aceito para Etapa 2.4. Data: 2026-10-05.

## Contexto

ROADMAP 2.4 pede tradução de O para SMT-LIB, busca finita limitada e candidato
model/v1. O kernel não foi implementado; ModelChecker 2.3 já confere os bytes
originais/h(O), interpreta modelos contextuais e distingue exaustão de rejeição.
Python/Z3 não pertencem à TCB. Não há integração ao farm nem provas de Verified.

## Decisão

1. `logos_provers.z3_models` recebe bytes canônicos originais de O e V. h(V)
   é SHA-256 da tag vocabulary/v1 e bytes V; O deve referir-se a esse contexto
   exato. A validação Python evita consultas inválidas, mas não emite autoridade.
   Hash esperado de O continua vindo do monitor; auto-hash não autentica O.
2. `refute` consulta Γ ∧ ¬goal. `witness` consulta Γ extraído do envelope O;
   valida goal, mas não inclui sua verdade na consulta. Retorna somente bytes
   de candidato FiniteModel ou None; não retorna CheckOutcome/claim/token.
3. Emitir SMT-LIB real e consumi-lo por `Solver.from_string` dos bindings
   `z3-solver==5.1.0.0`, em contexto Z3 isolado. A mesma versão antes usada
   somente no diferencial dev passa a dependência runtime Python. Nenhuma
   dependência Rust, manifesto TCB, formato, tag ou versão muda.
4. Sorts usam `declare-sort` e representantes posicionais distintos com axioma
   de cobertura universal. Enumerated fixa tamanho e igualdade entre suas
   constantes e ordinais. Open busca todos os vetores de cardinalidade 1..k,
   não somente vetores com tamanhos iguais. Limite também vale para Enumerated.
   Nomes SMT são gerados por posição; nomes AIR não entram na sintaxe SMT.
5. Todo símbolo é declarado, inclusive Defined. Pred Defined equivale ao corpo
   sob parâmetros livres argumento i=Var(i). Const/Func Defined exige, para
   todos argumentos e candidatos de resultado, corpo ↔ resultado=f(args).
   Isso impõe totalidade/unicidade global, inclusive símbolos não usados em O.
   Resultado=Var(0), argumento i=Var(i+1), binders locais deslocam o ambiente.
   Enum Defined também deve coincidir com seu ordinal, sem substituir expansão.
6. Extrair somente primitivos, preservando IDs e ordem de V. Funções densas
   u32 big-endian com último eixo mais rápido; predicados em tuplas lexicográficas
   únicas. Contexto finito exige resultado identificável entre representantes;
   falha de extração é erro explícito, não default/truncamento/certificado parcial.
   Tetos de saída herdados do checker padrão 2.1: 100.000 células e 1.048.576
   bytes; excedê-los retorna None sem certificado parcial.
7. Limites padrão: domínio máximo 4, timeout total 5000 ms. Relógio monotônico
   compartilhado entre consultas; solver recebe tempo restante. Verificações
   cooperativas de tempo cobrem preparação/busca/extração. Unknown/timeout encerra
   chamada sem candidato. Nenhum teto de RSS ou hard timeout físico é prometido.
8. A aceitação reproduzível usa ponte batch não autoritativa em example Rust,
   com V/O/h(O) esperado/M separados e o ModelChecker real. Nenhum booleano
   Python substitui verificação. Cada O do corpus tem modelo conhecido aceito
   como controle antes de testar cobertura do produtor.

## Alternativas

EnumSort Z3 evitaria axiomas de cobertura, mas declare-sort + fechamento explicita
transporte SMT-LIB pedido e mantém tradução independente do modelo de extração.
CLI Z3 exigiria binário extra; bindings já fixados entregam parser SMT-LIB e leitura
estruturada do modelo sem criar parser textual de saída nem dependência TCB.
Expansão textual Defined replicaria termos/corpos; axiomas globais preservam as
relações totais/únicas e permitem que o checker detecte desvios de tradução.

## Limites

None não significa unsat em domínios ilimitados nem prova de Verified. Busca é
finita e incompleta; solver inteiro pode mentir. Modelo falso só ganha aceitação
quando efetivamente satisfaz as condições do checker, não por reputação do Z3.
Ponte e corpus não implementam proveniência, assinaturas, epoch ou ledger real.
CI/ARM/S3/fuzz longo pendentes das etapas anteriores continuam separados.
