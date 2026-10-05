# Etapa 1.4 — Sintaxe de superfície e parsers

Escopo exclusivo do [ROADMAP 1.4](../../ROADMAP.md#etapa-14--sintaxe-de-superfície-e-parser-fora-da-tcb--m).
Parser Python e crate Rust separado **fora da TCB**. Nenhuma política, editor,
LSP de produto, obrigação, kernel ou expansão de Defined.

## 1. Interfaces de entrada

- Etapa 0.3: canon/v1, `logos_canon::{Text, to_canon, from_canon}` e
  `logos_client.canon::{Value, check_text, encode, decode}`, Unicode 16/Python 3.14.
- Etapa 1.1: `Sort`, `Symbol`, `Term`, `Formula`, prefixo plano De Bruijn,
  `Limits::default()` (128/100000), `spec/air.md` e ADR 0007.
- Etapa 1.2: `Vocabulary`, declarações/domínios, grounding e refs opacas,
  `spec/vocabulary.md`, ADR 0009. Validação de símbolos/sorts/enums/DAG disponível.
- Etapa 1.3: `check(&Vocabulary, &Formula)`, erros/caminhos; composição explícita
  no smoke, não chamada implícita do parser.

Dependências existem. Nenhum bloqueio de código. Rust LSP tentou inicializar e
falhou: `Unknown binary 'rust-analyzer' in official toolchain 'stable-x86_64-unknown-linux-gnu'`.
Usar leitura das APIs e compilação, sem alegar navegação LSP verificada.

## 2. Interfaces de saída

Contrato em [air-surface.md](../../spec/air-surface.md), decisão em
[ADR 0011](../adr/0011-sintaxe-de-superficie.md).

Rust `logos-air-text`:

```rust
pub fn parse_formula(text: &str, vocabulary: &Vocabulary) -> Result<Formula, SurfaceError>;
pub fn print_formula(formula: &Formula) -> String;
pub type References = BTreeMap<String, [u8; 32]>;
pub fn parse_document(text: &str, references: &References) -> Result<Document, SurfaceError>;
pub fn print_document(document: &Document) -> String;
```

Document contém Vocabulary e claims (nome, Formula, citações opacas ordenadas).
Python `logos_client.air` oferece operações equivalentes, com valores canon/v1
e dataclasses. Não há novo encoder, formato/hash de claim ou arquivo na TCB.
Consumidor futuro deve receber bytes AIR e verificar contratos próprios;
texto/glosa/citações não são autoridade.

## 3. Tarefas em ordem

1. Fixar gramática/escaping/precedência e representação sem perda em
   `spec/air-surface.md`, ADR 0011 e este plano. Testes de precedência,
   alfa-renomeação e formas explícitas verificam decisões.
2. Criar `crates/logos-air-text/{Cargo.toml,src/*,tests/*}`: lexer, parser,
   printer iterativo, fonte de erros. Inserir crate no workspace, não no manifesto.
   Testes comportamentais e property round-trip independentes fecham Rust.
3. Criar `py/logos_client/air.py` e testes: parser independente, modelo de Value,
   validação das invariantes de vocabulary/v1, printer e erros controlados.
   Testes comportamentais e property round-trip fecham Python.
4. Criar `py/tests/air_gen.py`, `test_air_differential.py` e bridge Rust
   `examples/air_text_diff.rs`. Gerador determinístico cria texto e AST esperada
   independentemente dos parsers. Comparar os bytes V/Formula e reparse de ambos
   os printers para 500 fórmulas distintas.
5. Exercitar API real com `examples/air_text_smoke.rs`: parse nominal, type-check
   separado, bytes reais decodificados, impressão/reparse, erros de superfície
   e demonstração de tipagem não implícita. Integrar smoke em ci/verify/CI nativa.
6. Rodar fmt/lints/testes/smoke/diferencial e `ci/verify.sh --selftest`. Atualizar
   estado e roadmap somente com evidência observada, commit sem push.

## 4. Critérios → prova reproduzível

| Critério ROADMAP | Teste/artefato |
| --- | --- |
| Round-trip nos dois parsers | propriedades em `crates/logos-air-text/tests/` e `py/tests/test_air.py`/helpers; todos os nós, n-aridades, metadados explícitos e limites |
| Diferencial de 500 fórmulas | `cd py && uv run pytest -q tests/test_air_differential.py`; compara com AST/bytes esperados, não somente concordância entre parsers |

Complementos: documentos com Open/Enumerated Fact/Policy e três groundings;
forward references/ciclos, fontes inexistentes, assinaturas mal tipadas,
Unicode/JSON, shadowing e scope entre irmãos, nulos/n-ários/free indices,
EOF e nesting com erro controlado. Testes não fixam texto de diagnóstico.

## 5. Riscos e decisões

- Gramática humana não representa sozinha todas as assinaturas sintáticas:
  explicit `symbol`, `var`, `app`, `pred` e constructors de conectivos preservam
  toda identidade inclusive erros de tipo. Não reescrever And/Or ou índices.
- Nomes livres não têm sort inferível: usuário escreve `var(index, sort)`.
  Binders nominais resolvem o mais interno; nome de símbolo com `()` não vira var.
- Declarações são recolhidas antes de resolver Defined; ciclos/ref inválida
  continuam erros, sem recuperar com glosa/default/valor parecido.
- Enum exige lista explícita. Fonte é digest/alias fornecido, nunca conteúdo
  buscado/inventado. Exemplo de roadmap é esboço sem membros/Action, não corpus
  completo; spec apresenta versão integral.
- Grounding/gloss omitidos abreviam textos vazios admitidos no formato, não
  atribuem significado empírico; printer imprime campos explicitamente.
- Dois parsers podem errar igual: corpus tem AST esperada independente e
  negativos locais, além de impressão/bytes cruzados.
- UTF-8/NFC/Unicode 16, u64 e limites precisam concordar. Parser e printer usam
  pilhas explícitas; nesting textual (4096 tarefas/contextos ativos) é distinto
  de profundidade lógica 128/nós 100000. Grupos transparentes não achatam
  operadores; cadeias excessivas retornam erro, nunca stack overflow/RecursionError.

## 6. Estimativa

M, mantida: dois parsers independentes, documentos/metadados e gramática debug
sem perda exigem mais que sintaxe de fórmula bem tipada. Não amplia TCB.

## 7. Evidência local — 2026-10-05

- Rust: `cargo test -p logos-air-text`, 31 testes (15 documentos, 14 fórmulas,
  2 propriedades). Propriedade Formula percorreu 500 ASTs; documentos cobrem
  Open/Enumerated Fact/Policy, Glossed/Anchored/Defined e metadados completos.
- Python: 71 testes novos; propriedade com 500 ASTs esperadas, independentemente
  do parser/printer. Diferencial: 500 fórmulas distintas, metade nominal/metade
  explícita, bytes Formula/Vocabulary comparados com AST independente; também
  reparse do documento impresso em Rust. Corpus adicional de 13 fórmulas fixa
  precedência, grupos aninhados, shadowing e restauração de escopo.
- `AIR_TEXT_SMOKE_OK` e `PY_AIR_TEXT_SMOKE_OK`, exit 0: APIs públicas reais,
  Defined adiantado, enum/fonte, alfa-renomeação, texto/bytes, citações repetidas,
  tipagem separada e erros. Not/quantificadores/App atingem profundidade lógica
  128; 129 é rejeitada. Grupos transparentes 200/500 preservam identidade;
  cadeias textuais de 10000 terminam em LimitExceeded.
- Integração inicial revelou stack overflow Rust e RecursionError Python;
  reduzir recursão só por guard não foi suficiente. Fastpath Python de grupos
  adjacentes perdeu operadores. Pilhas explícitas substituíram ambas as causas.
  Digest não ASCII de 64 bytes causava pânico em slicing UTF-8 Rust; reproduzido
  na bridge e corrigido com pares de bytes. Mesmo input passou a retornar
  UnknownReference (exit 1), e regressão de documento passou.
- Falhas intermediárias de compilação, tipagem/indentação/lint e fixture com
  semicolons inválidos foram corrigidas; nenhum resultado inicial foi contado
  como sucesso. Cargo apontou colisão de `diff` entre crates; bridge renomeada
  para `air_text_diff`, sem alterar protocolo ou deixar compatibilidade obsoleta.
- `ci/verify.sh --selftest`: VERIFY_OK, exit 0 observado diretamente; 175 testes
  Python, 25 do contador TCB, workspace Rust, smokes públicos, vetores,
  cargo deny/audit e autotestes dos guards. Avisos conhecidos de licenças
  não encontradas não são falhas. CI/runner ARM não observados.
- `cargo xtask tcb report`: correção 4277+78287=82564, fidelidade
  1550+80034=81584, build-only 51581. Nenhum LOC/dependência da TCB ou formato
  binário mudou; parser textual não é prova de fidelidade/correção.
