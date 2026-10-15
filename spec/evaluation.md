# Avaliação finita e checkers — Etapas 2.2/2.3

Normativo: MUST/MUST NOT conforme RFC 2119. ADR [0015](../docs/adr/0015-avaliacao-finita-defined.md).
Consome [air/v1](air.md), [vocabulary/v1](vocabulary.md), [model/v1](model.md),
Budget e StepMeter de [core](core.md). Não altera bytes ou hashes desses formatos.

## 1. Entrada e resultado

`eval(&M,&V,&φ,&Budget) -> Result<bool,EvalError>` MUST comparar h(V) calculado
contra M.vocab, sort-check de φ fechada e tipagem de todos os corpos Defined.
Modelo estruturalmente válido não demonstra interpretação válida de Defined.
Fórmula mal tipada, vocabulário divergente e definição não funcional MUST ser
erros de entrada; MUST NOT retornar booleano, Unknown genérico ou Verdict.

Unknown(Exhausted) MUST preservar causa de exaustão, nunca true/false. Nenhum
resultado desta API autentica fatos, glosas, autoridades ou eventos no ledger.

## 2. Denotação e parâmetros

Primitivos usam tabelas/conjuntos validados de M; funções densas têm último eixo
mais rápido. Pred nullary é true sse conjunto contém tupla vazia. Const/Func
nullary denota seu único output. Eq compara valores do mesmo sort validado.
And vazio=true; Or vazio=false. Not/And/Or/Implies/Iff são clássicos. Forall/Exists
percorrem todos os elementos do domínio finito não vazio do sort.

Defined Pred: corpo sob parâmetros livres, argumento i no índice De Bruijn i.
Defined Const/Func: relação definidora, resultado no índice 0 e argumento i no
índice i+1. Quantificadores no corpo deslocam parâmetros; ambientes de expansão
MUST ser isolados do chamador. Exemplo `f:S,T→S`: `Eq(Var(0,S), a(Var(1,S),
Var(2,T)))` define f a partir de função primitiva a. Pred `D(S,T)` pode usar
Var(0,S) e Var(1,T); sob Exists(T) passam a Var(1,S) e Var(2,T).

Todos os corpos MUST ser tipados sob esses parâmetros antes de avaliar φ.
Cada Const/Func Defined MUST ter exatamente um resultado em seu domínio para
cada tupla de argumentos, inclusive símbolos não usados por φ. Nenhum resultado
ou dois resultados produz InvalidDefinition{symbol}. Const Defined de Enumerated
MUST coincidir com seu ordinal, ou EnumerationMismatch{symbol}. MUST NOT usar
rótulo no lugar da expansão. Pred Defined não exige resultado único de indivíduo.

## 3. Trabalho e orçamento

Um único StepMeter MUST cobrir preparação, validação e fórmula. Antes do trabalho:

- 1 passo para conferência contextual de h(V);
- node_count(φ) passos para sort-check da fórmula (inclui termos);
- 1 passo por declaração de símbolo inspecionada;
- node_count(corpo) passos para sort-check de cada definição;
- 1 passo por nó de fórmula/termo realmente avaliado;
- 1 passo por iteração de quantificador, candidato de resultado Defined e
  tupla de entrada na validação global de Const/Func Defined.

Conectivos e quantificadores MUST avaliar integralmente em ordem prefixa/ordem
crescente, sem curto-circuito. Definições não são cacheadas: expansão repetida é
cobrada repetidamente. Índices/buscas/tabelas são trabalho interno de cada passo,
não instruções de CPU; passos não são promessa de tempo físico constante.

`budget.steps` é inclusivo: consumir exatamente limite permite resultado;
próxima cobrança falha com Unknown(Steps). Latch MUST prevalecer sobre resultado.
wall_ms é ignorado por checker interno; mem_bytes/size_bytes não são tetos de
RSS/entrada nesta API. Limites de construção AIR/Model permanecem necessários.
Reservas falíveis das pilhas do interpretador MUST retornar Unknown(Memory).
Sem recursão nativa proporcional à AST/cadeia Defined; sem expansão materializada.

Sem wall, mesmas entradas e limites MUST gerar mesmo resultado (S3). Esgotamento
MUST NOT virar aceitação/rejeição/falsidade (S2). Fronteira não escolhe Refuted.

## 4. Reprodução

- `cargo test -p logos-model --test evaluation`
- `cargo run -p logos-model --example eval_smoke` → EVAL_SMOKE_OK
- Em py/: `uv run pytest -q tests/test_model_differential.py` (10k pares, Z3 dev).
- `cargo xtask tcb report`: LOC próprio e closure da TCB, não prova formal.
- `FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 model_eval`.

Oráculo Z3 recebe interpretação finita completa; não integra caminho confiável,
nem produz modelos de obrigação. O evaluator 2.2 não registra ledger, otimiza
ou avalia incrementalmente. Execução local não fecha CI/ARM/fuzz ≥1 h.

## 5. Checkers de obrigação (Etapa 2.3)

ADR [0016](../docs/adr/0016-checkers-contextuais-de-modelos.md). API:

```rust,ignore
let checker = ModelChecker::new(&vocabulary)?;
checker.check_refutation(&bytes_o, hash_o, &model, &budget);
checker.check_consistency(&bytes_o, hash_o, &model, &budget);
checker.check_refutation_from_source(&bytes_o, hash_o, |h| cas.get(h), hash_model, &budget)?;
checker.check_consistency_from_source(&bytes_o, hash_o, |h| cas.get(h), hash_model, &budget)?;
```

`ModelChecker::new` MUST preparar Reasons canônicos de forma falível, sem
fallback ou panic. Métodos em memória retornam CheckOutcome; métodos de fonte
retornam Result<CheckOutcome,E> (CasError ao usar cas.get). Nenhum emite Verdict/claim/evento.

Refutação MUST aceitar sse todas as premissas são verdadeiras e goal falso.
Consistência MUST aceitar sse todas as premissas são verdadeiras: não avalia
goal, mas MUST exigir sintaxe/tipagem válida dele. Mesmo com Γ vazio, conferir
h(V) do modelo e Defined total/único/ordinal globalmente antes do aceite.
Premissa falsa, goal verdadeiro na refutação e entrada inválida são Rejected;
exaustão MUST ser Unknown com causa preservada, nunca Refuted/Rejected.

Ambos MUST chamar decoder contextual de [obligation/v1](obligation.md) com
os mesmos bytes e hash esperado, sem AST fornecida à parte ou reencoding antes
do hash. h(O) MUST vir do monitor: hash autoconsistente não demonstra origem.
O reader não verifica proveniência/época das referências; generate permanece
única construção do Obligation autoritativo (P1).

### 5.1. Orçamento compartilhado

Um StepMeter por execução MUST cobrir:

- comprimento de O antes de hash/parsing/contexto;
- comprimento de M antes de decoding contextual no caminho CAS;
- contexto/Defined global uma única vez, cobrados como §3;
- node_count(F) por fórmula usada, mais interpretação integral conforme §3.

Decoder já confere todas as fórmulas; checker MUST NOT repetir sort-check nem
reiniciar medidor por premissa. Pilhas do interpretador são reutilizadas entre
fórmulas; Defined continua expandido/cobrado a cada aplicação. Refutação pode
recusar quando encontra premissa falsa; consistência não interpreta goal.
Para V sem símbolos e fórmulas folha: bytes_O + 1 + 2 * (premissas + 1)
passos para refutação; bytes_O + 1 + 2 * premissas para consistência; caminho
CAS soma bytes_M. Consumir exatamente o teto permite resultado. Latch prevalece.

Budget.size_bytes MUST limitar O (e O+M no caminho CAS), além dos DecodeLimits/
ModelLimits padrão. Modelo em memória já foi validado sob ModelLimits; MUST NOT
reencodá-lo apenas para medir entrada. CAS impõe seus limites próprios antes
da leitura: não é hard RSS do Budget. mem_bytes não mede RSS; wall_ms ignorado.
Reservas falíveis tipadas MUST preservar Unknown(Memory).

### 5.2. Artefatos e erros

CAS MUST carregar M por Hash<FiniteModel>, verificando integridade dos bytes.
O checker MUST recalcular h(M) dos mesmos bytes independentemente da fonte.
Modelo ausente é Rejected. CasError::Integrity/TooLarge/Io MUST permanecer Err,
nunca falsidade lógica nem Unknown arbitrário. Depois da leitura, decoder de
modelo MUST usar o mesmo V do checker. Envelope inválido produz Rejected;
limites estruturais do modelo produzem Unknown(Size).

Desde3.6 (ADR0025), métodos *_from_source recebem closure falível de leitura;
o checker recalcula h(M) e preserva soma O+M, medidor e erros da fonte. A crate
logos-model não depende do ledger, inclusive em dev; os testes com CAS real
vivem em logos-ledger. Sem mudança de wire ou resultados lógicos.

Reprodução: `cargo test -p logos-ledger --test model_checker` e
`cargo run -p logos-ledger --example checker_smoke` → CHECKER_SMOKE_OK.
Mutação completa do arquivo checker.rs: comandos/resultados no
[plano 2.3](../docs/plans/etapa-2.3.md). Sem ledger, roteamento ou produtor Z3.

