# Etapa 1.3 — Sort-checker

Somente ROADMAP 1.3. Dependências 1.1/1.2 implementadas; sem bloqueio de código.
Sem inferência de sorts, parser, expansão/tipagem de corpos Defined, obrigações,
vereditos ou mensagens em português. TCB: sem novas dependências externas e sem
mudança de air/v1 ou vocabulary/v1. Decisão técnica em ADR 0010.

## 1. Interfaces de entrada

`Vocabulary::sorts()/symbols()`, `Sort`, `Symbol`, `SymbolKind`, `SymbolResult`,
`Formula::nodes()`, `FormulaNode`, `Term::nodes()`, `TermNode`.
As árvores são prefixos planos estruturalmente válidos; não repetir validação
estrutural nem fabricar fallback para invariantes garantidas pelos construtores.

## 2. Interfaces de saída

```rust
pub fn check(vocabulary: &Vocabulary, formula: &Formula) -> Result<(), SortError>;
pub struct SortError { pub path: SortPath, pub kind: SortErrorKind }
pub struct SortPath { pub formula_node: usize, pub term: Option<TermPath> }
pub struct TermPath { pub position: TermPosition, pub node: usize }
pub enum TermPosition { EqualityLeft, EqualityRight, PredicateArgument(usize) }
pub enum SortErrorKind {
    UnknownSort,
    UnknownSymbol,
    SignatureMismatch,
    InvalidSymbolKind,
    InvalidSymbolResult,
    InvalidConstantArity { actual: usize },
    ArityMismatch { expected: usize, actual: usize },
    UnboundVariable { index: u64, binders: usize },
    SortMismatch { expected: Sort, actual: Sort },
}
```

Todos os tipos de erro/path derivam Debug, Clone, PartialEq, Eq. Path e TermPosition
podem derivar Copy. SortError implementa Display/Error (diagnóstico inglês; texto
não é contrato). Caminho usa índice zero-based do record Formula, mais posição
Eq/argumento e índice zero-based do record Term: localização exata no prefixo,
sem alocar vetores de ancestrais por nó.

### Regras e precedência

- True/False e conectivos são fórmulas; And/Or de zero filhos permitidos.
- Quantificadores exigem sort declarado. Forall/Exists empilham binder somente
  para o corpo; pop ao sair do subtree, nunca vaza entre irmãos.
- Pred: nome declarado, assinatura integral exatamente igual à declaração,
  kind Pred, resultado Bool, quantidade de argumentos igual à arity, argumentos
  com sorts esperados. Func/Const em posição de Pred são InvalidSymbolKind.
- Term App: nome declarado, assinatura exata, kind Const ou Func, resultado Sort.
  Pred em termo é InvalidSymbolKind. Const com arity não vazia é
  InvalidConstantArity; Func pode ser nulo-ária. Child count precisa coincidir
  com signature.arity.len(). Cada filho tem sort esperado da posição respectiva.
- Var: sort declarado; índice De Bruijn encontra binder ativo (0 = mais próximo),
  depois sort anotado deve ser idêntico ao sort do binder. u64::MAX é Unbound,
  sem truncamento. Não inferir nem normalizar sort.
- Eq: conferir termo esquerdo completo, depois conferir raiz direita e exigir
  seu sort igual ao esquerdo antes de descer nos filhos direitos. Erro intrínseco
  da raiz direita precede SortMismatch, que aponta essa mesma raiz.
- Nó de termo: erro intrínseco (declaração/kind/result/aridade ou var) precede
  mismatch com pai/argumento. Conferir sort esperado antes de descer nos filhos.
- Erros de argumentos reportam nó raiz do argumento/subtermo divergente;
  erros do predicado/aridade ficam no record Formula. UnknownSort em binder
  fica no record Formula. Ordem de visita pré-ordem, esquerda para direita.
- Apenas símbolos usados na fórmula são sort-checados. Vocabulário pode conter
  assinatura inadequada não usada; check não concede credencial ao vocabulário
  nem tipa suas definições (parâmetros livres/expansão pertencem aos consumidores).

Implementação iterativa, com maps emprestados por nome, stacks explícitas de
counts/assinaturas e binders. Sem cópia de AST/Text/Sort em caminho bem-sucedido;
SortMismatch possui cópias somente para devolver erro com ownership.

## 3. Tarefas em ordem

1. Plano, ADR 0010 e spec/sort-check.md definem regras/precedência e path.
2. src/sort_check.rs implementa check e erros; lib.rs exporta novos tipos;
   tcb/manifest.toml lista arquivo. Não modificar contratos dos decoders existentes.
3. tests/sort_check.rs: casos positivos/negativos para cada ramo, path exato,
   escopo entre irmãos/nesting, argumentos aninhados, assinatura/kind/result,
   sorts desconhecidos, aridade, Const inválido, variáveis livres/u64::MAX,
   cadeia profunda em stack 64 KiB e fórmulas vazias And/Or.
4. tests/sort_check_properties.rs: gerador determinístico pseudoaleatório sem
   novas deps, milhares de fórmulas fechadas bem tipadas em vocabulário multi-sort;
   mutações válidas estruturalmente quebram tipo e devem ser recusadas. Usar
   constantes/funções/predicados, múltiplos argumentos, Eq/conectivos/quantificadores
   e shadowing; mutação nunca é apenas erro estrutural do construtor.
5. examples/sort_check_smoke.rs: API pública constrói/decodifica fórmula e V,
   aceita fórmula fechada e rejeita binder/sort/aridade com path observado.
   Fuzz sort_check usa decoders existentes: não há parser/decoder novo.
6. Cobertura LLVM branch com nightly datado, runner ci/check-sort-coverage.sh,
   report JSON/artefato reproduzível: 100% dos ramos instrumentados de
   crates/logos-air/src/sort_check.rs, sem excluir regiões ou branches do checker.
   Integrar job de cobertura CI (não colocar custo/toolchain nightly no verify
   stable) e smoke em ci/verify.sh/job Rust; fuzz descoberto automaticamente.
7. Controles de mutação em cópias isoladas: ignorar sort de argumento, igualdade,
   índice binder/escopo ou assinatura; testes devem falhar na asserção behavior.
8. Full verify --selftest, estado/roadmap, medição TCB e commit; sem push.

## 4. Critérios → evidência

| Critério | Evidência |
| --- | --- |
| 100% ramos checker | ci/check-sort-coverage.sh, JSON LLVM com todos branches cobertos; CI remota separada |
| bem tipadas passam / mutações de tipo falham | tests/sort_check_properties.rs com seed fixo e casos reproduzíveis |
| erros com caminho | tests/sort_check.rs com SortPath/TermPath exatos, não texto incidental |

## 5. Riscos e decisões

Escopo De Bruijn e posição de irmãos não podem compartilhar binder fora do corpo.
Kind/result de Symbol sintaticamente representável não implica uso permitido.
Não validar declarações não usadas nem corpos Defined como fórmulas fechadas.
Não usar recursão no checker nem path de tamanho proporcional ao prefixo.
Cobertura branch exige cargo-llvm-cov e llvm-tools-preview em nightly compatível;
ferramentas de verificação não são dependências TCB. LSP Rust indisponível no
ambiente (falhou na inicialização na 1.2); usar compilação/Clippy.

## 6. Estimativa

P, mantida: checker isolado sobre AST/vocabulário prontos. Cobertura de ramos e
propriedades são critérios obrigatórios, não pendências silenciosas.

## 7. Evidência local — 2026-10-05

- `cargo test -p logos-air --test sort_check --test sort_check_properties`:
  24 testes passaram: 22 comportamentais e 2 propriedades.
- Propriedade positiva: 2000 fórmulas fechadas, seed `0x5eedc0de12345678`.
  Propriedade negativa: 1000 contextos gerados, seed `0x5eedc0de123456a2`,
  com 2270 mutações estruturais válidas recusadas pelo motivo de tipo esperado:
  variável/sort 331, igualdade 829, argumento 312, aridade 312,
  variável livre 174, assinatura 312. Mensagem `MUTATION_CASES` reproduzível
  com `cargo test -p logos-air --test sort_check_properties -- --nocapture`.
- `cargo run -p logos-air --example sort_check_smoke`: `SORT_CHECK_SMOKE_OK`;
  bytes reais de V/Formula decodificados, fórmula fechada aceita e cinco erros
  (sort, binder, aridade, kind, declaração) com paths exatos e diagnóstico real.
- Fórmula de 1000 Nots, 500 binders e termo com 500 funções validados em
  thread com stack de 64 KiB.
- `ci/check-sort-coverage.sh`: cargo-llvm-cov 0.9.1 com nightly-2026-09-28,
  **36/36 ramos LLVM (100%)**, sem exclusões. Relatório reproduzível em
  `target/sort-check-coverage.json`; linhas 179/182, regiões 186/192.
  Não confundir cobertura de ramos com 100% de linhas ou prova formal.
- `FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 sort_check`:
  206912 execuções em 61 s, sem crash, semeado com 17 pares de valores canônicos
  V/Formula. Pares são apenas entrada do fuzz, não novo formato normativo.
- `ci/verify.sh --selftest`: `VERIFY_OK` (exit 0), incluindo smokes públicos,
  guards/manifesto/deny/audit, fmt/Clippy, testes workspace, Ruff/mypy,
  geradores --check, 104 testes Python e 25 TCB, mais controles negativos da CI.
  Dois avisos `license-not-encountered` conhecidos, sem falha.
- `cargo xtask tcb report`: correção 4277+78287=82564 LOC; fidelidade
  1550+80034=81584 LOC; build-only 51581; logos-air 1547 LOC.

### Controles de mutação da implementação

Quatro cópias isoladas do crate, com dependências reais canon/crypto e
`tests/sort_check.rs`, compilaram e falharam nas asserções de comportamento.
Rodar separadamente `cargo test --offline --test sort_check NOME -- --exact`
após aplicar a mudança numa cópia do crate:

| Mutação em src/sort_check.rs | Regressão que a matou (exit 101) |
| --- | --- |
| omitir expect_sort(actual, expected, path) dos filhos App | test_nested_app_child_sort_mismatch |
| passar None em vez de Some(sort) à raiz EqualityRight | test_eq_well_typed_and_sort_mismatch |
| omitir binders.truncate(before) ao sair do frame | test_scope_and_sibling_isolation |
| trocar comparação da assinatura declarada por if false | test_pred_unknown_symbol_and_signature_mismatch |

Todos aceitaram indevidamente entrada que precisava de recusa; os testes
detectaram Ok onde esperavam SortError. Workspaces temporários removidos;
implementação original nunca foi alterada pelas mutações.
Primeira integração falhou em dois E0382 de fixtures (ownership de Sort/Term);
primeiro full verify falhou em collapsible_if nas propriedades. Correções
aplicadas, sem alteração das regras de tipagem.

Reexecução final do full verify falhou (exit 101) no assert preexistente
`std::ptr::eq` sobre dois `dyn Error` do CAS: igualdade de fat pointers inclui
vtables, que podem ser duplicadas em diferentes unidades de codegen.
Removido esse assert incidental, não substituído por outro teste de identidade.
As verificações do erro primário/secundário e diagnóstico continuam; produção
do CAS não mudou. Teste específico
`cargo test -p logos-ledger cleanup_preserves_primary_and_secondary_errors`
passou (exit 0); full verify final terminou `VERIFY_OK`, exit 0 observado
diretamente por subprocess. Medição final acima inclui essa remoção.

CI remota (incluindo sort-coverage), smoke nativo ARM e fuzz ≥ 1 h continuam
não observados; nenhum checkmark remoto foi antecipado.
