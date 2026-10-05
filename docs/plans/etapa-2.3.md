# Plano — Etapa 2.3: refutação e consistência

## 1. Entradas e dependências

FiniteModel/model-v1 e eval (2.1/2.2), Obligation gerado/obligation-v1 (1.5),
CAS tipado (0.5), Budget/StepMeter (0.6). Implementações existem. Nenhum ledger
real ou produtor requerido. cargo-mutants 27.1.0 será instalado como ferramenta
dev; comando não estava instalado. Rust LSP continua indisponível.

## 2. Interfaces e decisões (ADR 0016)

Preservar Obligation privado, somente generate; não adicionar CanonDecode nem
conversão de bytes. Novo CheckedObligation é objeto não autoritativo, privado,
sem Canon/Domain/conversão para Obligation. decode_for_check(bytes, Hash<Obligation>,
&Vocabulary, DecodeLimits) confere hash dos bytes originais antes de parsing,
CBOR/envelope exatos, h(V), refs estritamente crescentes e fórmulas fechadas.
DecodeLimits: max_bytes=1MiB, max_premises=4096, air=Limits padrão; max_nodes
agregado entre todas as fórmulas, max_depth por fórmula. Excessos são Unknown(Size).
DecodeError: Unknown(Exhausted), HashMismatch, VocabularyMismatch, Canon, Air,
Sort, InvalidPremises. Sem inferência de proveniência/época citável pelo decoder.

ModelChecker::new(&Vocabulary) -> Result<ModelChecker, CanonError> prepara quatro
Reason canônicos (entrada inválida, premissa falsa, goal verdadeiro, modelo ausente).
Razões são validadas uma vez antes de checar: sem unwrap/fallback para erros de
codificação. Métodos check_refutation(bytes,h,&FiniteModel,&Budget) e
check_consistency(bytes,h,&FiniteModel,&Budget) -> CheckOutcome. Ambos conferem
mesma obrigação; consistência ignora verdade do goal, não sua validade estrutural.
Métodos check_refutation_from_cas/check_consistency_from_cas recebem bytes/h da
obrigação, &Cas, Hash<FiniteModel>, Budget e retornam Result<CheckOutcome,CasError>;
I/O/integridade não são falsidade. Modelo ausente é Rejected. CAS não registra ledger.

Adicionar eval_on_meter(M,V,φ,&StepMeter), reusando exatamente evaluator 2.2;
eval público de Budget continua wrapper com latch. Um medidor cobre input,
preparação e todas as avaliações do checker. Não resetar por premissa.
Budget size_bytes limita bytes da obrigação (e soma O+M no caminho CAS), com
limites estruturais padrão adicionais; cobrar bytes antes de parsing/hash;
mem_bytes não promete RSS e wall_ms permanece ignorado. Reservas falíveis
preservam Unknown(Memory). Error Unknown nunca vira Rejected/Accepted.

## 3. Sequência

1. Parser/contexto e regressões/vetores/fuzz de obligation/v1 separado.
2. Checkers/métodos CAS + shared-meter, testes semânticos/P2 e smoke real CAS.
3. cargo mutants somente código de checker (arquivo checker.rs), suíte real;
   corrigir todos os sobreviventes semanticamente viáveis; não excluir lógica.
4. Smoke, fuzz, full verify/selftest, contagem TCB e documentação/commit.

## 4. Contrato de conclusão

- P2: bytes de outra obrigação/hash original sempre recusados; CBOR não
  canônico/trailing/formato inválido recusado mesmo com hash recalculado.
- Tabelas de verdade: premissas falsas recusam ambos; goal falso só requisito
  de refutação; consistência com goal true/false e premissas vazias.
- Orçamento compartilhado: cada fórmula isolada cabe, soma excede => Steps.
  Defined/modelo/V inválidos nunca produzem aceite; 1000 repetições S3 local.
- CAS real: put/read/reopen M, absent/corrupt/size e bytes/hash O trocados.
- Vetores existentes positivos reaproveitados; negativos independentes
  adicionais para parser contextual, versão do wire inalterada.
- cargo-mutants: 100% dos mutantes semanticamente viáveis do checker mortos;
  registrar total, inviáveis e eventuais equivalentes com evidência, sem silêncio.
- Fuzz parser novo; manifesto/lints, full verify/selftest e LOC.

## 5. Fora do escopo e riscos

Sem ledger/roteamento, producer Z3 2.4, Verdict ou autenticação de referências.
Hash dado ao checker deve vir de obrigação gerada pelo monitor: hash recalculado
por atacante não prova origem. CheckedObligation não altera fronteira P1.
CAS tem limites próprios além do Budget; leitura não impõe hard RSS/wall.
Decoder necessário nesta etapa não é dependência ausente da 1.5: consumo dos
bytes originais é entregável da 2.3; ADR 0012 permanece sobre geração selada.

## 6. Tamanho

P com decisões explícitas: decoder e orçamento compartilhado exigem plano/ADR.
Nenhum formato/domínio/versionamento novo; dependências runtime workspace
core para obligation, obligation/ledger para model já permitidas em deny.toml.

## 7. Evidência — 2026-10-05

Implementação: ModelChecker sobre V real, CheckedObligation sem autoridade,
leitura/verificação de certificado de modelo no CAS. Sem ledger/roteamento,
producer ou alteração de formato/hash. Reason validado uma vez por processo
(LazyLock<Result<...>>), emprestado pelos contextos; clone apenas para Rejected.
VocabHash já conferido pelo reader é reutilizado na identidade do modelo.
Uma máquina/pilhas por execução, incluindo todos os Defined e fórmulas.

### Comportamento e reprodução

```sh
cargo test -p logos-model -p logos-obligation
cargo run -p logos-model --example checker_smoke
cargo clippy --workspace --all-targets -- -D warnings
cargo xtask tcb report
ci/verify.sh --selftest
```

- Suíte seletiva: 100 testes Rust em 12 suítes; 21 novos de checker e 15 de
  decoder. Reusa 20 vetores positivos; 10 negativos adicionais com SHA-256
  conferido também por hashlib stdlib. UI geração selada permanece passando.
- P2 bytes/hash cruzados para ambos os checkers; modelo/V divergentes,
  formatos/trailing/tipagem de goal mesmo na consistência, Defined inválido
  não usado, orçamento agregado e fronteiras exatas O/O+M.
- Corpus determinístico de 1000 repetições Accepted/Rejected/Unknown local.
  Predicado Defined quantificado sobre domínios 1/2 exercita contexto/máquina
  reutilizados entre premissas e goal, com mudança real da refutação.
- CAS real: put/reopen, goal verdadeiro aceito só na consistência, ausência,
  corrupção Integrity, teto do CAS Err(TooLarge), modelo mal codificado
  Rejected e domínio excedido Unknown(Size); consumo cumulativo O+M.
- Smoke público: CHECKER_SMOKE_OK. Workflow Rust nativo x86_64/ARM e verify.sh
  incluem o smoke; fuzz descobre obligation_decode e observa seus specs/vetores.
- Full verify/selftest: VERIFY_OK, exit 0; workspace Rust, smokes, guards,
  independência, manifesto, deny/audit, vetores, 176 Python + 25 TCB; todos os
  negativos de proteção bloqueados pelo motivo certo. Avisos deny preexistentes
  de licenças não encontradas e syn 2/3, sem erro.

Falhas intermediárias registradas: fixtures SortDecl/Array incompatíveis,
predicado ausente no modelo da regressão de goal mal tipado; corrigidos antes
de contar testes. Também corrigida lógica lowercase que mascararia o sort-check.
Clippy encontrou collapsible_if, is_multiple_of e onze clones desnecessários
para slices; corrigidos sem allow. Não houve falha de runtime no smoke.

### Mutação automática

```sh
cargo install cargo-mutants --version 27.1.0 --locked
cargo mutants --no-config --package logos-model \
  --file crates/logos-model/src/checker.rs --test-workspace false \
  --cargo-test-arg=--test --cargo-test-arg=checker \
  --timeout 90 --jobs 2 --caught --unviable
cargo mutants --no-config --package logos-obligation \
  --file crates/logos-obligation/src/decode.rs --re decode_for_check \
  --test-workspace false --cargo-test-arg=--test --cargo-test-arg=decoding \
  --timeout 90 --jobs 1 --caught --unviable
```

Resultado final, repetido após integração:

| Escopo | Gerados | Capturados | Inviáveis | Sobreviventes/timeouts |
| --- | ---: | ---: | ---: | ---: |
| checker.rs inteiro, todas as funções | 20 | 12 | 8 | 0 |
| decoder: decode_for_check | 21 | 17 | 4 | 0 |

Pontuação do checker: **12/12 viáveis mortos (100%)**, sem excluir função,
operador ou mutante compilável e sem config/skip. Oito substituições artificiais
por Default não compilam porque CheckOutcome/CheckedObligation não têm Default.
Não são mortos por testes e não demonstram cobertura; mantidos no relatório.
No reader: Default ausente, dois match arms removidos inviabilizam inferência,
e `if let ... || ...` é sintaxe proibida; quatro inviáveis, explicitamente
inconclusivos conforme [cargo-mutants](https://mutants.rs/using-results.html#mutant-outcomes).
Mutantes executáveis incluem ambas as desigualdades de bytes, fronteiras O+M,
subtração trocada por soma/divisão, troca refutação/consistência, true/false
forçados para premissas, remoção de Not e inversões de hash/envelope/ordenação.

### Fuzz e tamanho

```sh
FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 obligation_decode model_eval
```

Final: obligation_decode 10297 execuções/61 s; model_eval 3909/61 s, sem crash.
Primeiro smoke do parser: 23582/61 s, sem crash. Nightly real 1.101, sem
RUSTC_BOOTSTRAP. Corpus/logs gerados não entram no commit.

TCB correção: **6677 próprios + 78287 runtime = 84964** (+470 próprios sobre
2.2), build-only 51581. logos-obligation 550; logos-model 1845. Dependências
externas runtime inalteradas. Não é prova formal.

CI/ARM/S3 remota e fuzz ≥1 h ainda não observados; local não fecha pendências
anteriores. Sem push. STATUS preserva propostas de memória/diário e logs
preexistentes fora do commit desta etapa. Próximo: produtor não confiável 2.4.

