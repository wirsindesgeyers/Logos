# Plano — Etapa 2.2: avaliador M ⊨ φ

## 1. Entradas

FiniteModel/model-v1 validado (2.1), Vocabulary/AIR imutáveis (1.1–1.3),
Budget e StepMeter com latch (0.6). Sem dependência ausente de implementação.
Z3 será dependência de desenvolvimento Python, fora da TCB, para o diferencial.
LSP Rust indisponível no ambiente conforme STATUS; compilador valida interfaces.

## 2. Saídas e contrato

`logos_model::eval(&FiniteModel, &Vocabulary, &Formula, &Budget)` retorna
`Result<bool, EvalError>`. EvalError distingue Unknown(Exhausted), erro de
sort-check, identidade canônica, vínculo de Vocabulary, definição não funcional
e enumeração incompatível. A assinatura abstrata do roadmap é alargada somente
para representar entradas semanticamente inválidas, que não são exaustão nem
falsidade; ADR 0015 fixa isso. Nenhum verdict/checker/ledger nesta etapa.

Pred Defined: parâmetros livres em ordem de assinatura, primeiro argumento no
índice De Bruijn 0. Const/Func Defined: corpo é relação, resultado no índice 0,
argumento i no índice i+1. Sob k quantificadores esses índices aumentam em k.
Const/Func devem ter exatamente um resultado para toda tupla de argumentos;
validar inclusive definições não utilizadas pela fórmula. Const Defined de
Enumerated deve coincidir com seu ordinal, sem substituir expansão pelo rótulo.

`check_with_binders` reutiliza sort-check existente; binders explícitos em ordem
externo → interno. Não copiar AST para envolver parâmetros em quantificadores.

## 3. Implementação

1. Fixar ADR/semântica; adicionar contexto inicial ao sort-check existente.
2. Fronteira pública confere h(V), fórmula fechada e tipagem de todos os corpos.
3. Interpretador ingênuo com pilhas explícitas, sem recursão proporcional a AST,
   termos, quantificadores ou cadeia de definições; sem materializar expansão.
4. Conectivos avaliados integralmente em ordem prefixa, sem curto-circuito;
   quantificadores percorrem domínio inteiro crescente. Passos por nó visitado,
   iteração de domínio, candidato de resultado e tupla de validação definida.
5. Mesmo StepMeter cobre fronteira, validação semântica e fórmula final. Budget
   wall_ms ignorado como no core. Esta etapa impõe steps; mem_bytes/size_bytes
   não são tetos de RSS/entrada: estruturas já validadas usam ModelLimits/AIR
   Limits, falhas de reserva do interpretador retornam Unknown(Memory).
6. Integrar manifesto, smoke e diferencial na verificação local/CI existente.

## 4. Testes e artefatos

- Testes Rust: todos nós/termos, sorts múltiplos, ordem de tabela, De Bruijn,
  Defined aninhado/nullary, relações sem/múltiplos resultados, erro de tipagem,
  vínculo contextual e enumeração incompatível inclusive não utilizada.
- Orçamento: limites exatos, quantificadores multiplicam trabalho, Steps nunca
  vira bool; 1000 repetições sem relógio (S3). CI nativa em duas arquiteturas
  permanece pendência até observada.
- Diferencial Python independente: 10k pares (M, φ) aleatórios, modelos finitos
  exportados explicitamente para Z3, fórmulas quantificadas e Defined; zero
  divergências. Não é produtor de modelos 2.4.
- Smoke público real, mutações semânticas detectadas pelos testes, LOC via
  cargo xtask tcb report; alvo <400 LOC do núcleo, reportar medição real.
- Nenhum parser/formato serializado novo; model/v1/vocabulary/v1 permanecem.
  Fuzz existente model_decode continua cobrindo fronteira de modelo; novo alvo
  de avaliação exercita formatos existentes e propagação de orçamento.
- ci/verify.sh e ci/verify.sh --selftest, com exit/VERIFY_OK registrados.

## 5. Riscos e fora do escopo

Busca exaustiva de unicidade pode consumir orçamento antes da fórmula; Unknown
é correto. Avaliação integral intencional mantém custo previsível. Tabelas
Defined não são cacheadas; otimizações/incremental fora. Não implementar
check_refutation/check_consistency (2.3), produtor (2.4), kernel ou ledger.

## 6. Tamanho

M, sem alteração de escopo; interpretação de definições e oráculo independente
são os pontos de risco. Evidências finais serão acrescentadas após execução.

## 7. Evidências locais

- 30 regressões Rust passaram após correção. Integração inicial reproduziu
  dois bugs: função nulária primitiva procurada na tabela de constantes e
  `f(g(c))` retornando valor de g sem terminar aplicação de f. Ambos têm
  regressões failing-before/passing-after mantidas.
- Máquina única com continuações/valores/ambientes reutilizados; sem cópia de
  AST nem Vec/máquina por termo/candidato. Termos com 500 níveis e cadeia de
  250 Defined passaram em thread de 64 KiB; fórmula com 500 Not também.
- EVAL_SMOKE_OK: true, false, Pred Defined, Unknown(Steps), definição inválida
  inclusive não utilizada e vínculo h(V) divergente.
- Z3 5.1.0.0 dev, modelos EnumSort completos e quantificadores nativos
  ForAll/Exists. Seed 20261005: 10.000 pares, 5.783 true e 4.217 false,
  zero divergências. Model.eval é usado; quantificador residual é decidido
  pelo solver sob a mesma interpretação finita explicitamente fixada.
- Cinco mutantes compilados mortos por asserções, um de cada vez, fontes
  restauradas: Eq/Iff invertido; índice funcional usando primeiro eixo
  em todas as posições; aceitar relação multivalorada; cobrança de nó
  omitida; ignorar h(V). Não é alegação de cobertura de todos os mutantes.
- S2: Steps preservado antes/durante expansão/quantificação, limites exatos;
  S3: 1000 repetições de true/false/Unknown sem wall. CI nativa ARM ainda
  não observada; checkbox de S3 remoto permanece aberto.
- Cobertura sort-check após parâmetro contextual: 36/36 ramos LLVM,
  100%, sem exclusões; instrumento não prova semântica.
- Fuzz final model_eval: 13.591 execuções/61 s sem crash, nightly-2026-09-28.
  Run anterior: 39.145/61 s. Harness complementa limites ao acrescentar Not;
  limite original não pode rejeitar a negação de AST válida na fronteira.
- tokei: núcleo evaluate.rs 537 LOC, fronteira/erros 118; alvo <400 excedido
  e explicado em ADR 0015. logos-model 1590 LOC, correção 6207+78287=84494,
  fidelidade 1559+80034=81593; 51581 só de compilação. Sem runtime externo novo.

ci/verify.sh --selftest final: VERIFY_OK, exit 0. Workspace Rust/smokes/guards,
manifesto, supply-chain e vetores, 176 Python + 25 TCB e selftest completo.
Tentativas iniciais falharam em Clippy (collapsible_if/chunks_exact/
is_multiple_of) e ruff/mypy (cast obsoleto/tuplas Canon invariantes);
corrigidas sem allow. Avisos cargo-deny já existentes de duas licenças não
encontradas e syn 2/3 permaneceram avisos, sem erro. CI/ARM não observados.
