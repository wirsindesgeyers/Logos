# Etapa 1.2 — Vocabulário versionado com grounding

Escopo exclusivo do ROADMAP 1.2. Dependências 1.1, canon/v1 e hashes tipados
implementadas. Sem sort-check, parser, atestação, resolução de referências,
tradução ou invalidação. TCB: sem novas dependências externas, decoder estrito,
manifesto e fuzz. Decisões normativas: ADR 0009 e spec/vocabulary.md.

## 1. Interfaces de entrada

`Sort`, `Symbol`, `SymbolKind`, `SymbolResult`, `Formula`, `FormulaNode`, `TermNode`,
`Limits`; `Text`, `Canon`, `CanonDecode`, `Decoder`; `Domain`, `hash`, tag VOCAB.
Fórmulas são prefixos planos imutáveis. AIR e canon permanecem v1.

## 2. Interfaces de saída

```rust
pub struct FactRef(pub [u8; 32]);
pub struct PolicyRef(pub [u8; 32]);
pub struct AttestorId(pub Text);
pub enum DomainSource { Fact(FactRef), Policy(PolicyRef) }
pub enum SortDomain { Open, Enumerated { constants: Vec<Symbol>, domain_source: DomainSource } }
pub struct SortDecl { pub name: Text, pub domain: SortDomain }
pub enum Grounding { Defined(Formula), Anchored { attestor: AttestorId, procedure: Text }, Glossed(Text) }
pub struct SymbolDecl { pub signature: Symbol, pub grounding: Grounding, pub gloss_pt: Text }
pub enum Compatibility { Compatible, Incompatible }
pub struct Vocabulary { /* private validated version, sorts, symbols, definition_order */ }
impl Vocabulary {
    pub fn new(version: u64, sorts: Vec<SortDecl>, symbols: Vec<SymbolDecl>) -> Result<Self, VocabularyError>;
    pub fn version(&self) -> u64;
    pub fn sorts(&self) -> &[SortDecl];
    pub fn symbols(&self) -> &[SymbolDecl];
    pub fn definition_order(&self) -> &[usize];
    pub fn compatibility_with(&self, previous: &Self) -> Compatibility;
    pub fn extend(&self, sorts: Vec<SortDecl>, symbols: Vec<SymbolDecl>) -> Result<(Self, Compatibility), VocabularyError>;
    pub fn decode_with_limits(bytes: &[u8], limits: Limits) -> Result<Self, VocabularyError>;
    pub fn from_value_with_limits(value: &Value, limits: Limits) -> Result<Self, VocabularyError>;
}
```

`Vocabulary: Canon + CanonDecode + Domain`; `hash(&v)` produz `Hash<Vocabulary>`.
`VocabularyError`: `Canon(CanonError)`, `Air(AirError)`, `DuplicateSort`,
`DuplicateSymbol`, `UnknownSort`, `UnknownSymbol`, `SignatureMismatch`,
`InvalidDomainConstant`, `CircularDefinition`, `VersionOverflow`.
Nomes de símbolos vêm exclusivamente da assinatura, sem campo redundante.
Identidades de declaração: nome único por namespace, sem overload. Assinaturas
referenciadas devem coincidir integralmente com a declaração. Domínio enumerado
contém constantes declaradas, distintas, sem argumentos e com resultado do sort.

`Defined` guarda **corpo definidor**, não axioma completo `A ↔ corpo`: referência
a A no corpo é ciclo. Para predicado com argumentos, variáveis De Bruijn livres
representam parâmetros; tipo/fechamento e forma da definição ficam para etapas
posteriores. Referências a todos os símbolos/sorts no corpo são declaradas.
Grafo inclui aplicações nos termos de Eq e argumentos de Pred, além de Pred.
Somente símbolos Defined entram no grafo topológico; bases Anchored/Glossed não
são definições. Kahn iterativo produz índices dependencies-first, sem recursão.

Compatibilidade é direcional: candidato preserva exatamente toda declaração
anterior por nome, incluindo domínio/fonte, assinatura, grounding e gloss_pt.
Versão e ordem das listas não alteram compatibilidade, mas entram em h(V).
`extend` recebe patches completos: substitui nomes existentes na posição original,
adiciona novos ao fim em ordem de entrada; duplicatas em patches são erro;
versão incrementa checked (u64, zero permitido). Revalida tudo e retorna marca
Compatible/Incompatible; não muta anterior nem aceita ciclo numa redefinição.

### Layout vocabulary/v1

Um item CBOR canon/v1, arrays exatos; sem campos extras ou bytes restantes:

- Vocabulary: `[1, version, [SortDecl...], [SymbolDecl...]]`.
- SortDecl: `[name, domain]`; Open `[0]`; Enumerated `[1, [air/v1 Symbol...], source]`.
- Source Fact `[0, bytes32]`; Policy `[1, bytes32]`.
- SymbolDecl: `[air/v1 Symbol, grounding, gloss_pt]`.
- Defined `[0, air/v1 Formula]`; Anchored `[1, attestor_text, procedure_text]`;
  Glossed `[2, text]`.
- h(V): SHA-256(`b"logos/vocab/v1\0" || canon(V)`).

Declarações e enumerações preservam ordem, sem ordenação/normalização silenciosa.
Refs opacas são digests de 32 bytes, não tokens resolvidos. AttestorId é nome
canônico opaco. Vazio não prova grounding. Texto segue Unicode 16/NFC.
Limites AIR valem por definição; frame deve ser limitado pelo consumidor.
CanonDecode usa Limits::default e converte erros semânticos em WrongType,
como AIR; API específica preserva VocabularyError/AirError.

## 3. Tarefas em ordem

1. Plano/ADR/spec: fixar representação, corpos definidores e compatibilidade.
2. `src/vocabulary.rs`: tipos, identidade, validação declarativa, grafo iterativo,
   extensão. `src/vocabulary_codec.rs`: formato e decoder. Exports em lib.rs;
   manifesto TCB lista ambos, AIR participa de correção e fidelidade.
3. `tests/vocabulary.rs`, `tests/vocabulary_vectors.rs`, gerador stdlib em
   `tools/gen_vocabulary_vectors.py`, vetores em `spec/vectors/vocabulary/`:
   casos positivos, negativos, todos os grounding, referências e hash.
4. Exemplo público `vocabulary_smoke`, fuzz `vocabulary_decode`; integrar em
   `ci/verify.sh` e CI pública existente. Mutação manual: apagar ciclo,
   ignorar gloss, ignorar definição, ignorar símbolos nos termos.
5. Smoke real, fuzz local, `ci/verify.sh --selftest`, estado/roadmap e commit.
   CI/ARM/fuzz noturno só fecham quando observados remotamente.

## 4. Critérios → evidência

| Critério | Demonstração |
| --- | --- |
| A↔B/B↔A rejeitado | teste de corpos mutuamente dependentes, self-cycle, ordem invertida e ciclo por termos |
| adição compatível, gloss/definição incompatíveis | testes extend/compatibility_with, preservação do anterior e hash/version |
| h(V) fixo | vetores independentes Python stdlib + testes Rust e gerador --check |

Smoke público constrói domínio enumerado/fonte, grounding dos três tipos,
decodifica bytes, estende, altera gloss e rejeita ciclo. Mutações devem ser
mortas pelos testes que cobrem comportamento, não por texto do código.

## 5. Riscos e decisões

ADR 0009 decide versão u64, refs opacas, namespace sem overload, corpos
(definição não pode incluir própria cabeça) e compatibilidade estrutural.
Não há prova de fidelidade, existência do atestador, autorização de política,
fecho de domínio ou equivalência lógica. Ordem de declaração não pode ocultar
ciclos; substituição de assinatura revalida todas as referências.
LSP local falhou ao inicializar; validação Rust por compilação/Clippy.

## 6. Estimativa

M, mantida: novo formato TCB, validação de dependências e evidência independente.

## 7. Evidência local — 2026-10-05

- `cargo test -p logos-air --test vocabulary --test vocabulary_vectors`:
  39 testes passaram (38 comportamentais e um leitor de 53 vetores independentes).
- `cargo run -p logos-air --example vocabulary_smoke`:
  `VOCABULARY_SMOKE_OK`, h(V) de exemplo
  `5a55ce97e8e1a5aa15662fdb73cc6c53972139607e39bc8e1b6590bb9cfdedd5`.
  API pública exercitou três groundings, domínio/fonte, codec, adição compatível,
  glosa incompatível e rejeição de ciclo.
- Cadeia de 1000 definições validada/copiada/descartada em stack de 64 KiB.
- Ruff/mypy estrito do gerador e `gen_vocabulary_vectors.py --check` passaram.
- `FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 vocabulary_decode`:
  236898 execuções em 61 s, sem crash; 53 fixtures semearam o corpus.
  Descoberta automática integra o alvo ao job noturno; não comprova execução remota.

### Controles de mutação

Quatro cópias isoladas do crate usaram dependências reais canon/crypto, a suíte
`tests/vocabulary.rs` e `cargo test --offline --test vocabulary NOME -- --exact`.
Cada mutante compilou e falhou na asserção de comportamento (exit 101);
fontes originais não foram alteradas. Workspaces temporários removidos.
Para reproduzir, aplicar cada mudança separadamente numa cópia do crate:

| Mutação em `src/vocabulary.rs` | Regressão que a matou |
| --- | --- |
| trocar `if order.len() != count` por `if false` | `cycle_mutually_dependent_rejected`: aceitou ciclo em vez de CircularDefinition |
| comparar símbolos apenas por signature e grounding em compatibility_with | `compatibility_gloss_change_is_incompatible`: devolveu Compatible |
| comparar símbolos apenas por signature e gloss_pt em compatibility_with | `compatibility_definition_change_is_incompatible`: devolveu Compatible |
| trocar ramo TermNode::App por ramo vazio, sem dependency | `cycle_through_eq_terms_rejected`: aceitou ciclo via termos |

Primeiro run dos vetores falhou: fixture usava nome de erro inexistente
`NonCanonicalInt`; corrigida para o contrato canon/v1 `NonMinimalHead`.
Primeiro full verify parou em Clippy `op_ref`/`manual_is_multiple_of` nos novos
smoke/vetores; correções sem mudança de comportamento.

Verificação completa final: `ci/verify.sh --selftest` terminou com `VERIFY_OK`
(exit 0). Passaram fmt, Clippy -D warnings, testes workspace, smokes públicos,
guards/manifesto/independência, deny/audit, Ruff/mypy, geradores --check,
104 testes Python, 25 TCB e selftest de bloqueios. Cargo-deny emitiu dois
avisos `license-not-encountered` já conhecidos, sem falha.
`cargo xtask tcb report`: correção próprio=4006/runtime=78287/total=82293;
fidelidade próprio=1275/runtime=80034/total=81309; build-only=51581.
CI, smoke ARM e fuzz ≥ 1 h continuam não observados remotamente.
