# Plano — Etapa 2.1: estruturas finitas

Fonte: ROADMAP 2.1; decisão ADR 0014. Tamanho P; decisões de formato exigem plano.

## 1. Entradas

- Sort/Symbol/Vocabulary da 1.1/1.2; declarações, domínios e Grounding.
- Canon/Decoder estrito canon/v1 da 0.3; hash tipado e tags::MODEL da 0.4.
- Exhausted da 0.6 para distinguir Size/Memory de estrutura inválida.
- Nenhuma dependência faltante; evaluator, Z3 e ledger não são pré-requisitos.

## 2. Saídas

- `ModelData { vocab: Hash<Vocabulary>, domains: Vec<u32>,
  consts: Vec<(u32,u32)>, funcs: Vec<(u32,Vec<u32>)>,
  preds: Vec<(u32,Vec<Vec<u32>>)> }`, candidato não confiável.
- `FiniteModel::new(ModelData, &Vocabulary, ModelLimits) -> Result<Self, ModelError>`.
- `FiniteModel::decode_with_limits(&[u8], &Vocabulary, ModelLimits) -> Result<Self, ModelError>`.
- `encode_with_limits(&self, ModelLimits) -> Result<Vec<u8>, ModelError>` e
  `data(&self) -> &ModelData`, sem mutação de modelo validado.
- Canon + Domain; sem decoder sem V e sem veredicto lógico.
- `spec/model.md`, `spec/vectors/model/vectors.json` e gerador independente.

## 3. Tarefas em ordem

1. Fixar formato/model limites/ordem/enumeração no ADR e spec; plano de fase 2.
2. Implementar src do logos-model: validação completa e codec contextual.
   Dependências workspace existentes; atualizar manifesto e allowlist documentada.
3. Testes em tests/behavior.rs e tests/vectors.rs; vetores fixos via Python stdlib:
   totalidade, domínio, enum, Defined, assinaturas, IDs, tuplas e limites.
4. Smoke público em examples/model_smoke.rs; fuzz model_decode e mutações manuais
   com testes específicos que falham pelo comportamento, não compilação.
5. Integrar smoke/vetores/fuzz à verificação/CI, executar full verify/selftest,
   medir TCB e atualizar STATUS/ROADMAP. Commit somente com verificação verde.

Slices independentes: implementação src/Cargo; testes/smoke/fuzz; integração,
vetores e documentação. Interface compartilhada fixada antes de paralelizar.

## 4. Critérios → evidência

- Função parcial/valor fora do domínio rejeitados: testes específicos + vetores
  adversariais + smoke usando API pública; mutantes relaxam as duas checagens.
- Vetores canônicos: arquivos fixos, teste Rust decodifica/recodifica, hash
  independente hashlib, script `gen_model_vectors.py --check`.
- Excesso => Unknown(Size): limites inclusivos, agregado, produto/overflow,
  limites bytes antes de Decoder; smoke nunca aceita estrutura parcial.
- TCB: lints/manifesto/deny/full selftest; fuzz com nightly datado da CI.

## 5. Riscos e decisões

ADR 0014 fixa domínios não vazios, enumeração ordinal, IDs posicionais vinculados
por h(V), tabelas densas packed e conjuntos ordenados. Decoder Value é bounded
por bytes, não RSS. Defined não tem tabela; consistência de expansão e rótulos
enumerados só na 2.2. Não criar avaliador, parser CBOR alternativo ou autenticação.
Rust-analyzer falhou: `Unknown binary 'rust-analyzer'` na toolchain stable;
compilação/clippy/testes serão evidência, não LSP. Preservar docs de memória e
logs preexistentes fora do commit. CI/ARM/hora de fuzz exigem observação remota.

## 6. Tamanho

P mantido: estruturas e codec reutilizam V/canon/hash; sem busca ou avaliação.

## 7. Evidência observada

- 15 testes Rust passaram: 14 comportamentos + consumidor de 32 vetores fixos.
  Smoke público terminou `MODEL_SMOKE_OK`, exit 0.
- Regressão de encode confirmou limites de células/cardinalidade ignorados:
  falhou por retornar Ok onde exigia Unknown(Size); corrigida, suíte passou.
- Quatro mutantes compilados mortos, exit 101 por asserção comportamental:
  função parcial admitida, output igual à cardinalidade admitido, h(V) ignorado,
  decoder permitindo domínio enumerado maior que lista. Fontes restauradas.
- Integração inicial dos testes falhou por Array::from inexistente; removido
  caso packed duplicado dos vetores. Fixture de endianness corrigida para função
  nullary, evitando produto incorreto. Contagem incidental de vetores removida.
- Formatador nightly não instalado; formatado com stable, sem mudar toolchain
  do fuzz. Primeiro full verify parou em dois `collapsible_if` do Clippy.
- Clippy também detectou helper de teste obsoleto depois da remoção do caso
  packed duplicado; helper removido, sem allow para esconder diagnóstico.
- Medição final: logos-model 930 LOC; correção 5538 próprios + 78287 runtime
  = 83825, build-only 51581. Sem dependência externa runtime nova.
- Fuzz final `model_decode` no nightly datado: 212019 execuções em 61 s,
  sem crash, exit 0; corpus contextual começou com 32 vetores independentes.
  Smoke de fuzz anterior: 391395 execuções/61 s sem crash. Não demonstra
  ≥ 1 h noturna nem CI/ARM.
- `ci/verify.sh --selftest` final terminou `VERIFY_OK`, exit 0: fmt/clippy,
  Rust workspace, todos smokes, manifesto/guards, cargo deny/audit, ruff/mypy,
  geradores --check, 175 Python + 25 TCB e controles selftest pelo motivo certo.
  Avisos deny: duas licenças não encontradas e syn 2/3; nenhuma falha.
  Sem push ou observação de CI/ARM.
