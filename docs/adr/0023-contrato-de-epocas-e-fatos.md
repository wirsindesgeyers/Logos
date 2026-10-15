# ADR 0023 — Contrato de épocas e dependência de fatos

- Estado: aceito; proposta A adotada explicitamente pelo mantenedor
- Data: 2026-10-15
- Escopo: ROADMAP 3.4/6.1; decisão do mantenedor em 2026-10-15

## Contexto e evidência anterior à implementação (e46ebed)

As dependências declaradas da 3.4, eventos da 3.1 e índices da 3.3, existem.
Entretanto, seu único critério exige um **fato** válido nas épocas 3–5,
citável em 4 e recusado em 6. O caminho real de ingestão/citação desse fato
depende de tipos e eventos ainda inexistentes, previstos na 6.1:

- `logos-air::FactRef` é apenas um digest em `src/vocabulary.rs`.
- `logos-obligation::EntryKind::Fact` é categoria abstrata em `src/types.rs`;
  os testes dessa categoria usam implementações falsas de `LedgerView`.
- `logos-ledger::EventBody` tem 13 variantes, sem `FactRegistered`.
- Não há definição de `Fact` nem `EpochRange` nos fontes Rust/Python.
- `logos-attest` e `logos-kernel` continuam esqueletos. A 6.1 ainda depende
  da citação da 5.3, portanto antecipá-la não seria apenas criar um tipo.

Reprodução da inspeção, a partir da raiz (o primeiro comando não encontra
definições nem registro, exit 1; o segundo localiza apenas os pontos abstratos):

```sh
rtk proxy git grep -n -E '(struct|enum|class) (Fact|EpochRange)([^A-Za-z0-9_]|$)|FactRegistered' e46ebed -- crates py
rtk rg -n 'FactRef|EntryKind::Fact' crates
```

O snapshot real da 3.3 autentica o ledger e resolve CAS/escopo, mas só projeta
Assumption/Verified como citáveis na própria época de emissão. Uma fixture
que devolvesse `EntryKind::Fact` e escolhesse manualmente sua época não
demonstraria a integração exigida pela 3.4.

AGENTS.md §1.3 exige parar e registrar dependência ausente; §2.2 e ROADMAP §0
tratam critério de conclusão como contrato. A 3.1 também exclui eventos de
fases posteriores. O bloqueio foi registrado antes do código; a resposta
explícita do mantenedor adotou a divisão a seguir.

## Decisão A — separar mecanismo genérico da integração

Texto adotado para os entregáveis da **3.4**, preservando `EpochAdvanced`
com `new_epoch`, `reason`, `policy_root?` e `vocab?`:

> Definir `EpochRange` genérico, inclusivo, com construtor que rejeita
> início maior que fim e consulta de pertinência sobre u64 inteiro.
> Citabilidade permanece condicionada ao estado, à época corrente e à
> aplicabilidade da entrada; intervalo sozinho nunca confere autenticidade
> ou transforma uma entrada em fato. Fatos serão integrados na 6.1, e o uso
> de políticas para autorizar ações continua na Fase 7.

Substituição adotada para o critério da **3.4**:

- Intervalo genérico [3,5] contém 3, 4 e 5 e recusa 2 e 6; intervalos
  invertidos são recusados, com testes de limites 0/u64::MAX.
- No ledger/snapshot reais, avançar a época torna Verified/Assumption
  anteriores não citáveis e a geração recusa seu uso; Unknown/Invalidated
  continuam não citáveis. Esse teste não usa um fato fictício.
- Campos opcionais de EpochAdvanced têm vetores independentes, hash/
  assinatura vinculados e leitura estrita; formatos alterados recebem bump
  explícito, spec e fuzz, mantendo testes dos formatos anteriores conforme
  a estratégia de compatibilidade decidida no plano.

Acrescentar à **6.1** a dependência `3.4 — EpochRange e mecanismo de épocas`
e o critério integrado original:

- Fato assinado por emissor reconhecido, registrado no ledger real com
  validade [3,5], é citável em 4 e recusado em 6 pela LedgerView/gerador reais;
  sua janela não é fornecida por uma view falsa nem por cache adulterável.

O mantenedor adotou explicitamente esta divisão nesta sessão. Atualizar o
ROADMAP antes do código; não declarar integração de fatos na entrega genérica.

### Contrato técnico da 3.4

- `logos-core::EpochRange` possui limites privados inclusivos u64, construtor
  falível, getters first/last e contains. Tipo somente em memória nesta etapa,
  sem Canon/Domain/decoder; seu wire pertence ao formato que o usar na 6.1.
- EpochAdvanced ganha `policy_root: Option<[u8;32]>` (compromisso Merkle opaco,
  não hash de um objeto Policy ainda inexistente) e
  `vocab: Option<Hash<Vocabulary>>`. Não confere política/confiança/ações;
  presença no CAS antes do append continua na 3.5.
- Bump **do corpo** para `epoch-body/v2`: `[2,10,new_epoch,reason,root?,vocab?]`,
  opcionais canon `[]`/`[bytes32]`. Pelo menos um deve estar presente. Sem
  ambos, corpo histórico `[1,10,new_epoch,reason]` é a única codificação.
  Outros corpos continuam v1; versões/tags/aridades alternativas são erros.
- Envelope event/v1, domínios criptográficos e schema do ledger permanecem
  iguais: só o corpo versionado muda. A versão interna está coberta pelos
  bytes assinados e hasheados. Decode/encode/sign de eventos anteriores
  preservam bytes e assinatura; nenhum rewrite de histórico.
- Índices/snapshot mantêm regra conservadora de época corrente; janela de
  Fact não é simulada e EpochRange sozinho não torna nada citável. Provas e
  assumptions antigas permanecem consultáveis como histórico, mas não
  servem de premissa corrente. Hash lógico de O continua sem época.

## Alternativa B — não adotada: preservar o critério original

Manter a 3.4 bloqueada até que o caminho real de fatos esteja disponível.
Antes de mudar a ordem de execução, explicitar dependências e verificar se
5.3/6.1 podem ser alcançadas sem ciclo. Não pular automaticamente para 3.5
ou antecipar fases inteiras apenas para obter um teste verde.

## Consequências

- Bloqueio foi registrado antes do código. A decisão explícita permite
  atualizar critérios 3.4/6.1 sem antecipar fatos nem mudar a ordem das fases.
- EpochRange entra na TCB/manifesto; não acrescenta dependências. O decoder
  de corpo estendido é exercitado no alvo event_decode já existente, com
  seeds e vetores independentes. Construtor também recebe fuzz em core_decode.
- Mutantes compilados devem demonstrar detecção de limites, preservação dos
  opcionais e rejeição de premissas antigas, antes do full verify/commit.
