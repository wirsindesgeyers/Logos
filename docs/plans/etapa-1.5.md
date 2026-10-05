# Etapa 1.5 — Gerador de obrigações

## 1. Dependências e escopo

1.1 AST FOL-ms-eq/v1, 1.2 Vocabulary/hash/compatibilidade, 1.3 check e
Canon/crypto 0.3/0.4 prontos. LedgerView abstrato definido aqui; ledger real 3.3
não é dependência. Proposal wire 1.6 vem depois. Sem CNF/SMT/Lean, especulação,
kernel/gateway, assinatura de eventos ou memória proposta.

## 2. Interfaces

Contrato completo em spec/obligation.md, ADR 0012. Proposal<'a> { vocabulary,
goal, cites }; refs por digest de evento, LedgerEntry emprestada com kind,
época, retratação, Vocabulary e Formula. LedgerView snapshot estável com
current_epoch/lookup falível. Obligation campos privados e acesso de leitura;
Canon/Domain só saída, única construção generate, sem decoder/conversões externas.

## 3. Tarefas em ordem e arquivos

1. Fixar contrato/precedência/envelope e fronteira de confiança nos docs.
2. Implementar crates/logos-obligation/src/{lib,types,error,generate,codec}.rs;
   objetivo/premissas tipados, refs ordenadas/dedup, rejeições determinísticas,
   compatibilidade direcional, ownership sem cópias intermediárias. Manifesto
   e dependências de workspace atualizados; sem parser/decoder novo.
3. Testes públicos behavior, compile-fail trybuild com controle positivo,
   tools/gen_obligation_vectors.py e 20 vetores independentes em
   spec/vectors/obligation/. Smoke crates/logos-obligation/examples/obligation_smoke.rs.
4. Integrar smoke/vetores em ci/verify.sh e CI nativa, README/AGENTS.
5. Exercitar API, matar mutantes de regras críticas, medir TCB, full verify/selftest;
   registrar evidências, atualizar STATUS/ROADMAP e commit sem push.

## 4. Critérios → prova

| ROADMAP | Evidência |
| --- | --- |
| Mesma proposta, ordem diferente, mesmo h(O) | property/permutations públicas com oráculo bytes; dedup e referências iguais/diferentes |
| UnknownCitation e Pending NotCitable | testes exatos mais demais kinds, retratação/época/lookup/sorts |
| Nenhuma ingestão pública O externo | compile-fail struct literal/CanonDecode/Default/conversão, controle generate |
| 20 obrigações/h(O) fixos | Python/hashlib independente --check + teste Rust via generate |

Smokes reais com refs/época/status/Vocabulary, hash/domínio, mutantes que devem
falhar e regressões consumer-visible. Não testar mera wiring/wording/default.

## 5. Riscos/decisões

Refs são eventos, não fórmulas. Época excluída de O mas obrigatória na view.
Assumption preserva antecedente/provenance; não concede fato. Lookup confia no
snapshot e no binding evento/fórmula/Vocabulary, não autentica backend ausente.
Compatibilidade é alvo preservando origem, sem requisito de versão crescente
inventado além da 1.2. Rejeições determinadas pela ordem canônica. Nenhum decoder
público ou geração de artefato inválido em silêncio. Rust-analyzer indisponível
(toolchain stable reporta Unknown binary); compilar/clippy/testar substituem LSP.
Alterações preexistentes de memória/logs ficam fora do commit.

## 6. Estimativa

M: gerador TCB e fronteira de construção, formato/vetores, rejeições e invariantes
requerem demonstração comportamental e mutação; sem expandir para ledger/protocolo.

## 7. Evidência local — 2026-10-05

- `cargo test -p logos-obligation`: 19 testes passaram, sendo 17 behavior,
  1 teste dos 20 vetores e 1 suite compile-fail (5 negativos + controle positivo).
  Variantes citáveis/não citáveis, desconhecidos, retratação, época, falhas da
  view, sorts com paths, direção compatível, refs distintas/fórmula igual,
  ordem/dedup e precedência determinística exercitadas.
- `OBLIGATION_SMOKE_OK`, exit 0: API pública real com Vocabulary/Formula,
  snapshot em memória, geração/canon/hash, invariâncias e rejeições. Não prova
  autenticidade de ledger ausente nem concede Verified/token.
- `uv run python ../tools/gen_obligation_vectors.py --check`: 20 casos atualizados.
  Python stdlib/hashlib/CBOR independente de LOGOS; fórmulas/vocabulários AST
  authored independentemente, casos quantificados/De Bruijn/funções/igualdade.
- Seis mutantes compilaram e falharam pelos asserts certos (exit 101, não erro
  de build): Pending citável; época ignorada; dedup removida; compatibilidade
  invertida; sort-check de goal removido; goal substituído por uint no codec.
  Fontes restauradas antes da execução completa. Testes não vazios demonstrados.
- Integração inicial falhou com Sort::new/Formula::from_nodes inexistentes,
  Result<Hash> sem tratamento e move parcial de SortError nas fixtures.
  Corrigidos usando APIs existentes. Controle positivo UI também tinha APIs
  inexistentes; corrigido antes de capturar stderr real. Negativos bloqueados
  por E0451 (campos privados) e E0277 (decoder/default/conversões ausentes),
  não por erros irrelevantes. Asserts incidentais/tautológicos removidos.
- Clippy inicial encontrou clone_on_copy no smoke; ruff seis linhas >100;
  corrigidos. Formatter/mypy/ruff/clippy passaram. Smoke duplicado fora do
  crate criado durante a integração foi removido.
- `ci/verify.sh --selftest`: VERIFY_OK, exit 0 observado por subprocess direto;
  Rust workspace, 175 Python + 25 TCB, smokes, geradores --check, deny/audit,
  manifesto/independência e guards passaram. Dois avisos conhecidos de allowance
  de licença não encontrada; nenhuma falha final. CI/ARM remotos não observados.
- TCB medida: correção 4610+78287=82897; logos-obligation 335 LOC próprios
  (5 arquivos, +333 sobre stub). Fidelidade 1550+80034=81584; build-only 51581.
  Sem nova dependência externa runtime; obligation/v1 usa tag reservada e não
  altera formatos antigos. Sem decoder novo de obrigação, portanto sem novo
  alvo de fuzz de parser; AIR permanece sob alvos existentes.
