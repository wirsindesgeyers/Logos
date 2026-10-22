# Etapa9.1 — renderizador determinístico AIR → português

Estado: implementação local sobre 48f5cec (2026-10-21/22). Critérios 1–2 em
demonstração; critério 3 (revisão humana) **pendente do mantenedor**.
Decisão em [ADR 0050](../adr/0050-gramatica-paraphrase-pt.md); contrato em
[`spec/paraphrase-pt.md`](../../spec/paraphrase-pt.md).

## Histórico: pré-checagem de 21/out (base ced0cf3)

A primeira leitura constatou que vocabulary/v1 só tinha `gloss_pt` livre, sem
templates `render_pt`; a etapa parou (AGENTS.md §1.3). O pré-requisito 9.1a
(ADR 0046, vocabulary/v2) resolveu o bloqueio e foi concluído localmente em
57c8a03. Nenhum dado histórico foi reinterpretado como template.

## 1. Interfaces de entrada

- `logos_air::{Formula, FormulaNode, Term, TermNode, Vocabulary, check}`; templates
  `Vocabulary::render_pt()` validados pela 9.1a; `Limits` de air/v1.
- `logos_canon::{Canon, Text, Value}`; `logos_crypto::{hash, Hash, tags::PARAPHRASE}`
  (domínio reservado em `spec/domains.md`).

## 2. Interfaces de saída (consumidas por 9.2, 9.3, 15.x e clientes)

```rust
pub struct Paraphraser<'v>;            // new(&V) / with_limits(&V, ParaphraseLimits)
impl Paraphraser<'_> {
    pub fn render(&self, &Formula) -> Result<Paraphrase, ParaphraseError>;
    pub fn parse(&self, &str) -> Result<Formula, ParaphraseError>;
}
pub struct Paraphrase;                 // text(), vocabulary() -> &Hash<Vocabulary>, digest()
pub fn render_pt(&Formula, &Vocabulary) -> Result<Paraphrase, ParaphraseError>;
pub fn parse_pt(&str, &Vocabulary) -> Result<Formula, ParaphraseError>;
pub const GRAMMAR_ID: &str;            // "paraphrase-pt/v1"
pub const RENDERER_VERSION: &str;      // "logos-seal/render_pt/1"
```

## 3. Tarefas

1. ADR 0050 e `spec/paraphrase-pt.md` (gramática, pré-condições, hash).
2. `logos-seal`: `grammar.rs` (frases fixas únicas para render/parse),
   `paraphrase.rs` (tabelas, recusas de vocabulário, registro e hash),
   `render.rs`, `parse.rs`, `error.rs`; manifesto TCB e `deny.toml` (sem dependência
   externa).
3. Testes: `tests/paraphrase.rs` (textos exatos, recusas, limites, templates
   adversariais), `tests/properties.rs` (100k distintas), `tests/spec_vectors.rs`
   (todas as fórmulas dos vetores), `tests/vectors.rs` (Python independente).
4. `tools/gen_paraphrase_vectors.py` + `spec/vectors/paraphrase/vectors.tsv`.
5. Fuzz `paraphrase_parse` com semente dos vetores em `ci/fuzz.sh`.
6. Corpus e planilha de revisão (`etapa-9.1-revisao/`, exemplo `review_sheet`).
7. `ci/verify.sh`: vetores `--check`, mypy estrito do gerador, planilha `--check`.

## 4. Critério → evidência

| Critério | Evidência |
| --- | --- |
| Round-trip em 100k aleatórias até profundidade 6 | `properties.rs`: 100 000 fórmulas **distintas** (canon), 100 000 textos distintos, 75 293 na profundidade 6; mutações aceitas do texto conferidas como grafia única |
| … e em todas as fórmulas de `spec/vectors/` | `spec_vectors.rs`: varre os 40 arquivos, 82 fórmulas distintas; 70 fechadas/bem tipadas voltam idênticas, 12 abertas/mal tipadas recusadas com `Sort`; nenhum `RoundTripMismatch` |
| Gramática documentada | `spec/paraphrase-pt.md` + vetores independentes (17 textos/`h(P)` + 12 recusas) |
| `parse_pt` com fuzz ≥ 1 h | `ci/fuzz.sh 3600 paraphrase_parse`, recibo `target/fuzz-9.1.json` (§7) |
| Revisão humana de 50 paráfrases de políticas reais | **Não feita.** Material em [`etapa-9.1-revisao/`](etapa-9.1-revisao/README.md); origem das políticas e revisão cabem ao mantenedor |

## 5. Riscos e decisões

- Templates repetidos no mesmo contexto tornam o vocabulário não parafraseável
  (erro explícito). Aceito no ADR 0050.
- Texto longo e mecânico; concordância depende do template (P25 da planilha).
- As 50 políticas foram redigidas pelo agente; o critério pede políticas reais.
- A releitura em `render` é equivalente às checagens do construtor (mutação
  sobrevivente registrada): defesa em profundidade, não única garantia.

## 6. Tamanho

`M`, como no roadmap. TCB de correção inalterada; fidelidade +777 LOC próprios.

## 7. Evidência local

- Testes `logos-seal`: grammar (2 unitários), paraphrase (8), properties (1, 100k),
  spec_vectors (1), vectors (1).
- Mutações (`target/mutations-9.1.json`): 11 detectadas por testes que compilam e
  falham por asserção, com controle positivo e restauração por hash; 1 sobrevivente
  esperada (`render_self_check_removed`, equivalente sob as recusas do construtor).
- Smoke de fuzz 60 s: 403 501 execuções sem crash.
- Fuzz 1 h e full verify/selftest: ver STATUS.md e `etapa-9.1-evidencias.json`.
