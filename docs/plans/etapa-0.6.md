# Plano de etapa — 0.6: veredictos, orçamentos e erros

- **Data:** 2026-10-02.
- **Estado:** planejada; implementação não iniciada.
- **Escopo:** Etapa 0.6 do `ROADMAP.md`, tamanho **P**, TCB de correção.
- **Referências:** roadmap §0, §3 (S2/S3), §4 e 0.6; especificação conceitual
  §4.4 (rejeição não é refutação), §4 (certificados grandes), §5 (trust boundaries)
  e §10 ameaça 18; ADRs 0001, 0003 e 0004.
- **Local:** `crates/logos-core`, atualmente apenas esqueleto com lints da TCB.

**Fora do escopo:** checkers concretos das Fases 2 e 4; política de tentativas após
`Unknown` (5.4); estados `Pending`/`Invalidated` do claim (5.1). Também não implementar
supervisor de processos, medição de RSS, relógio, eventos, registro de máquina,
registro de checkers ativos ou conversões de erros do CAS. Esta etapa fornece o
vocabulário; esses mecanismos pertencem aos consumidores futuros.

## 1. Interfaces de entrada

| Interface existente | Uso nesta etapa |
| --- | --- |
| Workspace, lints e `ci/verify.sh` | Build, verificação completa e governança da TCB. |
| `logos_canon::{Canon, CanonDecode, Value, Array, Text, CanonError}` | Representação dos tipos e validação estrita da forma. |
| `to_canon`, `from_canon` | Bytes canônicos; decoder rejeita bytes sobrando. |
| `canon/v1`, `spec/encoding.md` | Inteiros mínimos, UTF-8/NFC, Unicode 16 e limite de profundidade. |
| `py/logos_client/canon.py` | Gerar vetores independentemente da implementação Rust. |
| `tcb/manifest.toml`, `deny.toml` | Registrar arquivos e dependências. |
| `ci/fuzz.sh`, workflow de fuzz | Executar o novo decoder no mesmo ambiente local/CI. |

Dependências técnicas 0.1 e 0.3 presentes. `logos-core` ainda não tem dependências;
introduzir apenas `logos-canon` por path em produção. Não depender de crypto, ledger
ou kernel. A 0.5 não é pré-requisito: seu `TooLarge` apenas confirma a semântica futura
`Unknown(Size)`.

**Bloqueio de fechamento remoto:** `.github/workflows/ci.yml` só possui runner
`ubuntu-24.04` no job Rust. Adicionar aarch64 nesta etapa; S3 não fecha com execução
local ou cross-compilation. Disponibilidade/custo do runner ARM no repositório ainda
não confirmados. Pendências remotas anteriores continuam separadas.

Reutilizar traits e erros do canon, vetores TSV/geradores em `tools/` e testes
determinísticos por semente (ADR 0003). Não criar segundo encoder CBOR nem adicionar
`proptest`. A exceção de doctests da 0.4 não substitui o `trybuild` exigido pela 0.6.

## 2. Interfaces de saída

Assinaturas propostas, a registrar em **ADR 0006** antes da implementação:

```rust
pub enum Exhausted { Time, Steps, Memory, Size }

pub struct Reason { /* Text privado */ }
impl Reason {
    pub fn new(text: logos_canon::Text) -> Self;
    pub fn as_str(&self) -> &str;
}

pub enum CheckOutcome {
    Accepted,
    Rejected(Reason),
    Unknown(Exhausted),
}

pub struct Budget {
    pub steps: u64,
    pub mem_bytes: u64,
    pub size_bytes: u64,
    pub wall_ms: u64,
}

pub struct StepMeter { /* saldo e latch de exaustão privados */ }
impl StepMeter {
    pub fn run(
        budget: &Budget,
        checker: impl FnOnce(&StepMeter) -> CheckOutcome,
    ) -> CheckOutcome;
    pub fn charge(&self, steps: u64) -> Result<(), Exhausted>;
}

pub enum Verdict { Verified, Refuted, Unknown }

pub fn verified_from_proof(outcome: CheckOutcome) -> Result<Verdict, CheckOutcome>;
pub fn refuted_from_countermodel(outcome: CheckOutcome) -> Result<Verdict, CheckOutcome>;
```

Implementar `Canon`/`CanonDecode` para `Budget`, `Exhausted`, `Reason`, `CheckOutcome`
e `Verdict`. `StepMeter` é estado transitório, não objeto de evento nem serializável.
`Budget`, `Exhausted` e `Verdict` podem ser `Copy`; `Reason`/`CheckOutcome` não são,
pois a razão possui texto. Não oferecer `Default` que fabrique sucesso ou orçamento.

### 2.1. Contrato do medidor: excesso vence resultado do checker

- `StepMeter::run` cria medidor privado com saldo `budget.steps`, executa closure e
  finaliza resultado. Se o latch disparou, retorna **sempre** `Unknown(Steps)`, mesmo
  que a closure ignore o erro e retorne `Accepted`, `Rejected` ou outro `Unknown`.
- `charge(n)` consome saldo apenas quando `n <= saldo`. Exceder saldo dispara latch
  irreversível e retorna `Err(Steps)`. Depois disso, toda cobrança retorna esse erro,
  inclusive `charge(0)`. Nenhum reset, setter, `Clone`, construtor público ou método
  público de finalização que permita descartar o latch.
- Closure recebe referência compartilhada, não `&mut StepMeter`: impede trocar
  medidor esgotado por outro de uma chamada aninhada usando `mem::swap`. Usar
  `Cell<u64>`/`Cell<bool>` privados para saldo/latch, sem heap, locks ou atomics;
  medidor local à execução, não compartilhável entre threads. Helpers internos
  recebem a mesma referência, sem criar orçamento novo.
- Limite exato aceita; primeira cobrança além dele esgota. Orçamento zero permite
  trabalho de zero passos, não uma cobrança positiva. Antes da exaustão, cobrar zero
  não altera estado. Usar comparação/subtração de saldo, sem soma que transborde em
  `u64::MAX`; sem alocação, relógio ou dispatch dinâmico no medidor.
- Checker de referência cobra **antes** de cada operação contabilizada e interrompe
  no primeiro erro. Unidade do passo pertence ao algoritmo e deverá ser documentada
  por cada checker futuro; medidor não consegue detectar trabalho não contabilizado.
- Internos ignoram `wall_ms`; todos os campos do orçamento são preservados na
  serialização. `mem_bytes`/`size_bytes` não são limites de passos nem monitor de RSS.
  Excesso externo de tempo será `Unknown(Time)` com máquina registrada pelo consumidor,
  não `Rejected`. Não construir supervisão externa nesta etapa.

### 2.2. Construção de veredictos e fronteira da garantia

As duas funções nomeadas aceitam somente `CheckOutcome::Accepted`: uma produz
`Ok(Verified)` no contexto de prova; outra, `Ok(Refuted)` no contexto de contramodelo.
`Rejected(reason)` e `Unknown(exhausted)` retornam `Err` com o outcome original,
sem perder razão nem transformar rejeição em `Verdict::Unknown`. Este último pode
ser representado diretamente quando a política futura decidir o estado do claim.

Não implementar `From`, `Into`, `TryFrom` nem método genérico de conversão de
`CheckOutcome` para `Verdict`. Revisar todos os matches de produção para impedir
promoção de rejeição/exaustão. Contexto prova × contramodelo é explícito no nome;
um `Accepted` isolado não autentica checker ativo nem obrigação.

**Limite importante do requisito compile-fail:** os enums públicos pedidos pelo
roadmap permitem escrever `Verdict::Refuted` diretamente e decodificar um veredicto.
Rust também não distingue variantes de `CheckOutcome` como tipos de entrada.
`trybuild` pode provar ausência de APIs de conversão, mas não impedir qualquer match
escrito por um consumidor. Registrar essa fronteira no ADR e complementar com testes
das funções e revisão; não anunciar prova de autorização. As garantias de checker
ativo/claim pertencem à 5.4. Se se exigir proibição absoluta de construção direta,
será necessária mudança explícita do contrato do roadmap e ADR, não um teste fictício.

### 2.3. Formato proposto: `core/v1` sobre `canon/v1`

Usar arrays de comprimento exato, com inteiro de versão `1` na primeira posição.
Formato novo, sem alterar `CANON_VERSION` ou formatos já existentes. Mudança posterior
deste layout exige versão nova, spec e vetores atualizados.

| Tipo/variante | Valor canônico |
| --- | --- |
| `Budget` | `[1, steps, mem_bytes, size_bytes, wall_ms]` |
| `Exhausted` | `[1, tag]`, tags `Time=0`, `Steps=1`, `Memory=2`, `Size=3` |
| `Reason` | `[1, texto]`, texto validado por `Text`, sem normalização silenciosa |
| `CheckOutcome::Accepted` | `[1, 0]` |
| `CheckOutcome::Rejected(r)` | `[1, 1, canon_value(r)]` |
| `CheckOutcome::Unknown(e)` | `[1, 2, canon_value(e)]` |
| `Verdict` | `[1, tag]`, tags `Verified=0`, `Refuted=1`, `Unknown=2` |

Decodificação orientada pelo tipo: alguns tipos compartilham bytes; não há envelope
universal nem identidade criptográfica intertipos nesta etapa. Consumidor informa tipo
esperado; eventual hash requer domínio/contexto próprio. `Reason` é diagnóstico livre
canônico, não código de controle nem classificação extensível de erros dos checkers.
Permitir texto vazio e não inventar catálogo antes de existir checker concreto.

Rejeitar versão/tag desconhecida, aridade errada, tipo de payload errado, campos
negativos ou acima de `u64`, e payload em `Accepted`. Usar `CanonError::WrongType` para
forma/versão/tag e `IntOutOfRange` para inteiro fora do intervalo, reutilizando erros
estruturais do decoder. Nunca ignorar campos, truncar, usar default ou interpretar
falha de decode como `Unknown`: erro de representação não é resultado de checagem.

## 3. Tarefas em ordem

### 3.1. Fixar decisões e especificação

**Arquivos:** criar `docs/adr/0006-veredictos-e-orcamentos.md` e `spec/core.md`.

Registrar APIs, unidade/precedência de passos, política de zero/overflow, funções de
veredicto, fronteira dos enums públicos e tabela completa de `core/v1`. Documentar
inteiros em bytes/ms/passos, distinção de tempo externo e limite de memória, e exemplos
suficientes para consumidor independente. Avaliar `trybuild` como dev-dependency:
resolver versão compatível, registrar licença e transitivas realmente novas em
`deny.toml`/ADR e atualizar `Cargo.lock`; não adicionar dependência de execução.

**Fecha com:** contrato revisado contra todos os entregáveis 0.6; nenhuma mudança
silenciosa no roadmap, em `canon/v1` ou na semântica de `Rejected`.

### 3.2. Implementar tipos e medidor

**Arquivos:** `crates/logos-core/Cargo.toml`, `src/lib.rs`; criar `src/outcome.rs`,
`src/budget.rs`, `src/verdict.rs`; atualizar `tcb/manifest.toml`.

Implementar tipos, funções nomeadas e medidor com finalização única por `run`.
Campos internos privados; preservar lints existentes. Criar testes públicos em
`crates/logos-core/tests/budget.rs` e `tests/verdict.rs`.

**Fecha com:** limite zero/exato/excedido, cobranças em lote e fracionadas, fronteira
`u64::MAX`, latch após erro ignorado, precedência sobre rejeição e outros `Unknown`;
matriz das duas funções com `Accepted`, `Rejected` e quatro motivos de exaustão.
Razão/exaustão devem sair intactas no erro. Checker de referência apenas nos testes:
percorre bytes, cobra um passo por byte e rejeita byte inválido depois da cobrança.
Cenários distintos demonstram aceitação, rejeição com orçamento suficiente e
exaustão antes de alcançar byte inválido.

### 3.3. Implementar codificação estrita e vetores independentes

**Arquivos:** criar `src/canon.rs`, `tests/vectors.rs`, `spec/vectors/core/` e
`tools/gen_core_vectors.py`; atualizar manifesto, `ci/verify.sh` e job Python da CI.

Implementar traits por `Value`/`Array` usando o decoder existente. Gerador Python
monta valores da tabela normativa, não chama Rust, e oferece `--check`, seguindo os
outros geradores. Incluir no ruff/mypy e verificação de vetores locais/CI.

**Fecha com:** comparação de bytes de todas as variantes, budgets zero/mistos/máximos,
razões ASCII/NFC e vetores negativos de versão/tag, aridade, payload, inteiros,
texto não NFC, encoding não mínimo e bytes sobrando. Round-trip isolado não substitui
bytes independentes nem rejeição de representações alternativas.

### 3.4. Provar contratos de tipos e sensibilidade dos testes

**Arquivos:** criar `tests/compile_fail.rs`, `tests/ui/` com fixtures e diagnósticos
`.stderr`; atualizar dev-dependencies, lockfile e allowlist conforme 3.1.

`trybuild` deve rejeitar atribuição direta de `CheckOutcome` a `Verdict`, `.into()` e
`Verdict::try_from` para outcomes construídos como `Rejected` e `Unknown`. Rejeitar
uso do `Result` das funções nomeadas como veredicto sem tratamento. Incluir controle
compilável exercitando APIs legítimas; verificar que fixtures falham pelo erro de
contrato esperado, não import quebrado. Não fixar strings de diagnóstico em testes
runtime; `.stderr` é o artefato do contrato compile-fail pedido.

**Fecha com:** `cargo test -p logos-core` passando e mutações manuais mortas: limite
exato recusado; wrap/erro em cobrança máxima; remoção do latch/finalização; promoção
de `Rejected` ou `Unknown` pelas funções nomeadas; decoder aceitando tag/versão/aridade
inválida. Adicionar temporariamente conversão `From<CheckOutcome> for Verdict` deve
fazer fixture `.into()` compilar e teste `trybuild` falhar. Restaurar por conteúdo e
recompilar; arquivar comandos, resultados e teste que matou cada mutante.

### 3.5. Demonstrar S3 nas duas arquiteturas e fuzz do decoder

**Arquivos:** `.github/workflows/ci.yml`; criar `tests/determinism.rs` e
`examples/core_smoke.rs`; atualizar `fuzz/Cargo.toml`, lockfile do fuzz e workflow de
fuzz; criar `fuzz/fuzz_targets/core_decode.rs`.

Transformar job Rust em matriz x86_64 (`ubuntu-24.04`) e aarch64
(`ubuntu-24.04-arm`), preservando build/fmt/clippy/testes existentes. Rodar nativamente,
registrar `uname -m`. Teste S3 repete **1000 vezes cada caso fixo** do checker de
referência, comparando resultado e bytes com expectativa versionada comum às duas
arquiteturas. Usar casos de aceitação, rejeição e exaustão; `wall_ms=0` para critério
S3, com teste adicional mostrando que alterar só `wall_ms` não muda checker interno.
Não usar tempo, RNG do SO, hash de ordem aleatória nem `usize` no cálculo do orçamento.

Smoke executável exercita API pública com mesmas entradas: orçamento suficiente,
exato e insuficiente; erro ignorado seguido de `Accepted`; funções de veredicto;
decode de vetores positivos/negativos. Falha de expectativa deve sair com código
não zero. CI roda `cargo run -p logos-core --example core_smoke` nas duas arquiteturas
junto dos testes. Manter exemplo reproduzível, sem lógica de produção de checker.

Novo fuzz target tenta os cinco tipos serializáveis; para cada decode bem-sucedido,
encode deve reproduzir exatamente input e decode subsequente deve conservar valor.
Sem tratar erro como sucesso. Alimentar corpus com vetores e rodar
`FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 core_decode` localmente; manter alvo
no smoke/noturno da CI conforme convenção existente. Não usar `RUSTC_BOOTSTRAP`.

**Fecha com:** logs/artifacts dos dois runners comprovam S3 e smoke; testes de chave
symlink/FIFO/corrida do crypto também passam nativamente em ARM, confirmando as flags
pendentes da 0.4. Fuzz sem crash local é evidência local, não substitui job remoto.
A 0.6 não exige por si uma hora de fuzz como critério, mas o alvo entra no job noturno.

## 4. Mapeamento dos critérios de conclusão

| Critério literal da 0.6 | Demonstração concreta |
| --- | --- |
| Exceder passos no checker de referência sempre dá `Unknown(Steps)` (S2) | `tests/budget.rs`: casos zero/exato/excesso, rejeição após ponto de exaustão, erro ignorado e latch; smoke público e mutantes da 3.4. Reaplicar essa regressão aos checkers reais em 2.2/4.4. |
| Mesmo input/Budget sem wall time, 1000 execuções em x86_64/aarch64 (S3) | `tests/determinism.rs`: 1000 repetições por caso contra expectativa fixa; matriz CI nativa com arquitetura registrada e smoke. Só marcar após observar ambos os jobs. |
| Compile-fail por `trybuild`, sem conversão indevida fora das funções nomeadas | `tests/compile_fail.rs` + fixtures de 3.4; controle positivo; matriz runtime das funções e revisão de matches. Limite dos enums públicos explicitado em 2.2; não prometer impedir código arbitrário do consumidor. |

Entregáveis adicionais obrigatórios: `spec/core.md`, vetores independentes com
`--check`, parser coberto por fuzz, dependências aprovadas no ADR/allowlist, arquivos
no manifesto e medição da TCB. Nenhum checkbox do roadmap será marcado no planejamento.

## 5. Riscos e decisões para ADR 0006

1. **Exaustão ignorada:** retornar só `Result` em `charge` não garante S2; finalização
   central com latch protege outcome, mas não interrompe closure mal escrita nem mede
   trabalho sem cobrança. Essa limitação deve ficar explícita para os futuros checkers.
2. **Veredicto não é credencial:** enum público/decodificável não prova origem. Manter
   escopo 0.6 e revisar caminhos; checker ativo e autorização serão garantidos no kernel.
3. **`trybuild`:** custo de transitivas/allowlist e diagnósticos dependentes de rustc.
   Manter requisito; inventariar versões/licenças na implementação, sem reutilizar
   silenciosamente a exceção de doctests da 0.4.
4. **Representação:** arrays versionados e `Reason(Text)` são propostas, não decisões
   já aceitas. Não criar catálogo prematuro de motivos nem modificar `canon/v1`.
5. **Runner ARM:** indisponibilidade/custo bloqueia fechamento S3, não autoriza marcar
   etapa concluída só com runner x86. Registrar bloqueio em `STATUS.md` se ocorrer.
6. **Campos de orçamento:** declarar mem/size não impõe limite global de memória;
   esta etapa só impõe passos. Não confundir erro de formato com `Unknown` nem limite
   de objeto com rejeição de certificado.

## 6. Estimativa e fechamento

Mantida **P**: crate pequeno, canon pronto, sem checkers concretos. Trabalho principal
é contrato do medidor, compile-fail, vetores e integração multi-arquitetura. Espera por
CI/runner ARM é risco de calendário, não razão para ampliar escopo.

Antes de commit da implementação: executar smoke público, testes/mutações descritos,
fuzz local, `ci/verify.sh --selftest` até `VERIFY_OK` e `cargo xtask tcb report`.
Atualizar este plano com evidências, `STATUS.md` (tabela, pendências, próximo passo e
Diário) e somente checkboxes demonstrados do roadmap. Sem evidência remota de S3,
registrar implementação local com pendência, não conclusão. Próxima etapa após 0.6:
0.7, documentação de ameaças/garantias/limites, ainda dentro da Fase 0.
