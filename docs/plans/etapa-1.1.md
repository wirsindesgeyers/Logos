# Plano da Etapa 1.1 — sintaxe abstrata FOL-ms-eq/v1

Data: 2026-10-04. Escopo exclusivo: ROADMAP Etapa 1.1. Dependências prontas:
canon/v1 e hash com domínio de fórmula; logos-air já listado na TCB.
Fora: vocabulário, sort-check, parser textual, Says e lógicas não clássicas.

## 1. Entradas

- `logos_canon::{Text, Value, Array, Canon, CanonDecode, Decoder, CanonError}`.
- `logos_crypto::{Domain, tags::FORMULA, hash}`; sem dependência externa nova.
- Spec conceitual §9 e ADR 0007 (representação/serialização novas).
- ASTs precisam admitir sintaxe ainda não sort-checada para a futura 1.3.
  Nenhuma checagem de declaração, aridade de assinatura, variável livre ou sorts
  neste estágio; checar somente forma da árvore e limites estruturais.

## 2. Saídas e contrato de integração

Arquivos: `src/{lib,error,shape,syntax,codec}.rs`; APIs reexportadas por lib.

```rust
pub struct Sort { pub name: Text }
pub enum SymbolKind { Const, Func, Pred }
pub enum SymbolResult { Sort(Sort), Bool }
pub struct Symbol {
    pub name: Text,
    pub kind: SymbolKind,
    pub arity: Vec<Sort>,
    pub result: SymbolResult,
}
pub enum LogicId { FolMsEqV1 } // as_str(), from_name(&str) -> Result<Self, AirError>
pub struct Limits { pub max_depth: usize, pub max_nodes: usize }
// Default: max_depth=128, max_nodes=100_000; zero não admite nó algum.
pub enum TermNode {
    Var { index: u64, sort: Sort },
    App { symbol: Symbol, args: usize },
}
pub enum FormulaNode {
    True, False, Eq(Term, Term),
    Pred { symbol: Symbol, args: Vec<Term> },
    Not, And(usize), Or(usize), Implies, Iff,
    Forall(Sort), Exists(Sort),
}
// Vec em pré-ordem: filhos imediatamente após pai, na ordem original.
// Nomes de binders ausentes; índice 0 é binder mais próximo.
pub struct Term { /* privado: nodes: Vec<TermNode>, stats: Stats */ }
pub struct Formula { /* privado: nodes: Vec<FormulaNode>, stats: Stats */ }
// Para ambos:
// new(Vec<Node>, Limits) -> Result<Self, AirError>
// nodes(&self) -> &[Node]; depth(&self) -> usize; node_count(&self) -> usize
// logic_id(&self) -> LogicId
// decode_with_limits(&[u8], Limits) -> Result<Self, AirError>
// from_value_with_limits(&Value, Limits) -> Result<Self, AirError>
// Canon/CanonDecode (decode padrão usa Limits::default()).
// Formula: Domain = tags::FORMULA; nenhum Domain novo para Term/Sort/Symbol.
```

`Sort`/`Symbol`: Canon/CanonDecode. Todos os tipos estruturais têm Debug/Clone/
PartialEq/Eq; Sort/LogicId/kinds têm identidade por conteúdo, não ID de arena.
Formula/Term não guardam limites; limites diferentes não alteram bytes/hash.

Interface interna para codec:

- `Stats { pub depth: usize, pub nodes: usize }`, Copy/Default; zero = sem termos
  embutidos; `combine(self, other) -> Result<Stats, AirError>` soma checked de nós
  e máximo de profundidade.
- `Shape::new(Limits)`, `next_depth(&mut self) -> Result<usize, AirError>`,
  `child_limits(&mut self) -> Result<Limits, AirError>`,
  `push(&mut self, children: usize, embedded: Stats) -> Result<(), AirError>`,
  `finish(self) -> Result<Stats, AirError>`.
- `Term`/`Formula` têm fields `pub(crate) nodes, stats`, para construir o resultado
  validado sem segunda caminhada. `TermNode::children()` e
  `FormulaNode::{children(),embedded_stats()}` são pub(crate).
- `AirError`: `Canon(CanonError)`, `UnsupportedLogic`, `InvalidStructure`,
  `DepthExceeded { limit, actual }`, `NodesExceeded { limit, actual }`;
  Debug/PartialEq/Eq/Display/Error e From<CanonError>. Overflow = InvalidStructure.

Shape valida prefixo com stack de filhos restantes. Raiz tem profundidade 1;
contagem inclui nós de fórmulas e de termos embutidos, não metadata Sort/Symbol.
Cada Eq/Pred soma profundidade de termo à profundidade do nó da fórmula.
`child_limits` reserva o nó pai e retorna orçamento restante para termos embutidos;
codec reduz max_nodes ao ler cada argumento sucessivo. `push` revalida estatísticas
agregadas de termos construídos sob orçamento diferente. Rejeita vazio, filhos
faltantes e raízes extras. And/Or/App de zero filhos são sintaxe admitida.

Representação plana evita recursão na construção/percurso/Clone/Drop. Canon
continua com Value, mas a profundidade CBOR é limitada por envelopes/metadados,
não pela profundidade lógica. Decoder genérico valida bytes antes de converter;
limites AST não são teto global de memória nem limite de bytes do frame.

## 3. Tarefas e arquivos

1. Plano + `docs/adr/0007-sintaxe-air.md`: contrato acima, decisões normativas
   abaixo, sem inventar normalização/equivalência além de alfa-renomeação.
2. Syntax/error/shape/lib e deps por path em Cargo.toml; testes de árvores
   prefixas inválidas, fronteiras depth/nodes e orçamento agregado.
3. Codec + `spec/air.md`: parser estrito, versões/tags/aridades/LogicId inválidos
   devolvem erro. Envelopes novos `air/v1`; nenhum formato anterior muda.
4. Vetores independentes gerados sem chamar logos-air/logos-canon, testes de
   fixtures e atualização de verify/CI para --check do gerador.
5. `examples/air_smoke.rs`, fuzz `air_decode`, manifesto atualizado e integrações
   no verify/CI/fuzz. Não tocar alterações locais do usuário em outros targets/logs.
6. Checks de etapa, smoke real, mutações manuais e fuzz pelo script oficial;
   verificação completa com selftest, relatório da TCB, STATUS/roadmap e commit.

### Formato normativo air/v1

Arrays CBOR canon/v1; sem mapas. Não ordenar/reduzir conectivos nem tabelas de nós.

| Tipo | Valor canônico |
| --- | --- |
| Sort | `[1, 0, name]` |
| Symbol | `[1, 1, name, kind, [Sort...], result]` |
| kind | `0` Const, `1` Func, `2` Pred |
| result | `[0, Sort]` ou `[1]` Bool |
| Term | `[1, 2, "FOL-ms-eq/v1", [TermNode...]]` |
| Formula | `[1, 3, "FOL-ms-eq/v1", [FormulaNode...]]` |
| Var | `[0, index_u64, Sort]` |
| App | `[1, Symbol, child_count]` |
| True / False | `[0]` / `[1]` |
| Eq | `[2, Term, Term]` |
| Pred | `[3, Symbol, [Term...]]` |
| Not | `[4]` |
| And / Or | `[5, child_count]` / `[6, child_count]` |
| Implies / Iff | `[7]` / `[8]` |
| Forall / Exists | `[9, Sort]` / `[10, Sort]` |

Counts unsigned u64 no fio, conversão checked para usize; binder/index também
unsigned. Sort/Symbol incorporados completos, sem vocabulário oculto ou referências
externas. Const é representável como App de zero filhos; sort-check posterior
confere assinatura. Todos os envelopes de Term/Formula incluem LogicId fixo.
Hash de Formula = SHA-256(`logos/formula/v1\0` || bytes do envelope).

## 4. Critérios → demonstração

| Critério | Verificação |
| --- | --- |
| Alfa-renomeação igual em bytes/hash | Teste de binders x/y com resolução De Bruijn no consumidor de teste, fixtures independentes; índices diferentes/nesting/shadowing mudam bytes |
| Profundidade 10⁶ rejeitada sem stack overflow | Teste em thread de stack pequena com Vec prefixa de 1 milhão de nós e bytes planos equivalentes; erro DepthExceeded e descarte seguro; também Term e limites agregados |
| Spec e vetores | spec/air.md e gerador --check, fixture de cada variante e erros/tags/aridades/versão/LogicId; testes Rust contra fixtures independentes |

Testes de TCB precisam matar bugs: desligar limite de profundidade, desligar limite
agregado de nós e alterar índice de variável/LogicId no encoder/decoder. Registrar
falhas por asserção (não erro de compilação) e restaurar conteúdo/mtime.
Smoke público verifica bytes/hash independentes, encode/decode, erros e limites
pela API, além da suíte. Fuzz garante round-trip exato dos bytes aceitos e dos
limites aceitos; smoke local não fecha ≥1 h noturna exigida na CI.

## 5. Riscos/decisões

ADR 0007 fixa prefixo plano e air/v1, identidade por Text e falta intencional de
sort-check. Evita Drop recursivo de milhão de Box e explosão do CBOR aninhado.
Sem vocabulário/interning/builder de IDs: eliminar ambiguidade de referências
locais e evitar abstração prematura. Quantificadores não guardam nomes.
Não confundir igualdade estrutural com equivalência lógica/normalização AC.
Metadata de assinaturas/texto exige limite de bytes do consumidor futuro, não
monitoramento global nesta etapa. Decoder canon ainda materializa Value; isso
será declarado, não escondido atrás de limites AST.

## 6. Tamanho

`M`, como roadmap: representação e formato novos na TCB, vetores independentes,
limites adversariais e contrato futuro para vocabulário/sort-check.

## 7. Evidência local — 2026-10-04

- `cargo test -p logos-air`: 11 testes passaram (10 de comportamento + harness
  de 168 vetores). Alfa-renomeação **∀x.P(x)/∀y.P(y)** usa resolução De Bruijn
  no consumidor de teste e hash independente; índices/nesting distintos não
  são colapsados. Argumentos e conectivos mantêm sua ordem/estrutura.
- `million_formula_and_4096_deep_terms_are_stack_safe`: thread de 64 KiB,
  construção e bytes planos independentes com 1.000.000 records de Formula,
  ambos recusados com DepthExceeded e descarte seguro. Term de profundidade
  4096 construído/encodado/descartado sem recursão lógica; decode limitado recusado.
- `cargo run -p logos-air --example air_smoke`: `AIR_SMOKE_OK` em x86_64;
  fixture/hash de ∀x:person.P(x) independentes, round-trip, limites agregados,
  LogicId desconhecido, prefixo incompleto e raízes extras.
- `tools/gen_air_vectors.py --check`: 168 entradas distintas, 39 positivas
  semeando fuzz; cobertura de todas as variantes, campos, erros e hashes Formula.
  Gerador somente stdlib, sem encoder de produção. Duplicatas de entradas
  removidas; teste isolado de valores default substituído por fronteiras reais.

### Mutações manuais (todas exit 101 por asserção)

Comando: `cargo test -p logos-air --test <suite> <test> -- --exact`.
Cada mutação foi temporária; conteúdo original restaurado e mtime atualizado.

| Mutação | Suite / teste que a matou |
| --- | --- |
| Desligar ambos os guards de profundidade em Shape | syntax / exact_and_zero_limits_apply_to_both_syntax_types |
| Ignorar guard de nós agregados no push de Shape | syntax / embedded_terms_are_revalidated_under_aggregate_parent_limits |
| Encoder colapsar todos os índices de variável para 0 | syntax / nested_binders_shadowing_and_distinct_indices_are_not_conflated |
| Decoder não conferir texto LogicId | vectors / independent_air_vectors_cover_syntax_bytes_hashes_and_rejection |

Após restaurar todos: verificação completa `ci/verify.sh --selftest` terminou
com `VERIFY_OK`, incluindo suíte Rust, smokes core/AIR, lints/manifesto/independência,
deny/audit, geradores, ruff/mypy, 104 testes Python + 7 da TCB e selftest.
Avisos de cargo deny: duas licenças permitidas não encontradas no grafo;
sem erro de supply chain. Clippy/mypy/ruff iniciais falharam em estilo/tipagem
dos testes/gerador e foram corrigidos antes do run completo.

Fuzz oficial: `FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 air_decode`;
182529 execuções em 61 s, sem crash. Target integrado à descoberta automática
da CI; **≥1 h noturna e CI remota ainda não observadas**. Log local não fecha
essas pendências. Não foi executado novo fuzz de outros targets nesta etapa.

TCB de correção: 3424 LOC próprios + 78287 de dependências de execução,
51581 só de compilação; logos-air adiciona 690 LOC próprios. Sem dependência
externa nova. Manifesto atualizado com os cinco módulos.
