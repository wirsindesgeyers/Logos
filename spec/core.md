# Core — core/v1

Especificação normativa da Etapa 0.6. Vocabulário comum de checadores e claims: orçamentos
(`Budget`), medidor de passos lógicos (`StepMeter`), motivos de esgotamento (`Exhausted`),
razões diagnósticas (`Reason`), resultados de checagem (`CheckOutcome`) e veredictos
formais (`Verdict`).

Construído sobre a codificação canônica `canon/v1` ([encoding.md](encoding.md)) sem alterar
sua versão nem seu formato.

---

## 1. Formato de representação `core/v1`

Cada valor serializável de `core/v1` é codificado como um array CBOR canônico de comprimento
fixo, contendo o inteiro de versão `1` na primeira posição (`index 0`).

O versionamento é próprio de `core/v1` e independente de `canon/v1`. Qualquer evolução de
layout exigirá nova versão normativa (ex: `core/v2`), atualização desta especificação e novos
vetores de teste.

### 1.1. Tabela normativa de tipos e variantes

| Tipo / variante | Representação canônica | Descrição |
| --- | --- | --- |
| `Budget` | `[1, steps, mem_bytes, size_bytes, wall_ms]` | Array de 5 elementos; campos são inteiros unsigned de 64 bits (`0..=2^64-1`). |
| `Exhausted::Time` | `[1, 0]` | Limite de tempo esgotado (tag 0). |
| `Exhausted::Steps` | `[1, 1]` | Limite de passos lógicos esgotado (tag 1). |
| `Exhausted::Memory` | `[1, 2]` | Limite de memória esgotado (tag 2). |
| `Exhausted::Size` | `[1, 3]` | Limite de tamanho de dados/certificado esgotado (tag 3). |
| `Reason` | `[1, text]` | Array de 2 elementos; `text` é string de texto canônica (`major 3`, UTF-8 em NFC, code points atribuídos em Unicode 16). Texto vazio `""` é permitido. |
| `CheckOutcome::Accepted` | `[1, 0]` | Checagem aceita (tag 0, aridade 2, sem payload). |
| `CheckOutcome::Rejected(r)` | `[1, 1, canon_value(r)]` | Checagem rejeitada (tag 1, aridade 3, payload é o valor canônico de `Reason`: `[1, text]`). |
| `CheckOutcome::Unknown(e)` | `[1, 2, canon_value(e)]` | Checagem inconclusiva por recurso (tag 2, aridade 3, payload é o valor canônico de `Exhausted`: `[1, tag]`). |
| `Verdict::Verified` | `[1, 0]` | Obrigação verificada por prova válida (tag 0). |
| `Verdict::Refuted` | `[1, 1]` | Obrigação refutada por contramodelo válido (tag 1). |
| `Verdict::Unknown` | `[1, 2]` | Veredicto inconclusivo (tag 2). |

### 1.2. Decodificação orientada pelo tipo

Não existe envelope polimórfico universal nem identificador de tipo na raiz dos bytes. A
decodificação é estritamente orientada pelo tipo esperado fornecido pelo consumidor (ex:
`from_canon::<Budget>(&bytes)`).

Tipos distintos compartilham representações em bytes idênticas quando possuem mesma aridade e
mesmas tags inteiras. Por exemplo, a sequência de bytes correspondente ao array canônico `[1, 0]`
(`0x820100`) é válida para:
- `Exhausted::Time` (quando decodificado como `Exhausted`);
- `CheckOutcome::Accepted` (quando decodificado como `CheckOutcome`);
- `Verdict::Verified` (quando decodificado como `Verdict`).

Qualquer contextualização ou disambiguação criptográfica intertipos é responsabilidade de
domínios externos (como tags de domínio do CAS ou do ledger).

`Reason` é um diagnóstico livre canônico, não um código de controle de máquina nem taxonomia
fechada. Não efetua normalização silenciosa de texto: textos não-NFC são rejeitados na entrada.

---

## 2. Validação estrita e regras de erro

A decodificação estrita valida forma, versão, aridade, tipos de payload e intervalos numéricos,
rejeitando qualquer desvio com erros tipados de `CanonError`:

1. **Forma e versão:**
   - Se o valor decodificado não for um array CBOR: `WrongType`.
   - Se o primeiro elemento do array não for o inteiro sem sinal `1`: `WrongType`.
2. **Aridade:**
   - Se o array de `Budget` não contiver exatamente 5 elementos: `WrongType`.
   - Se o array de `Exhausted`, `Reason` ou `Verdict` não contiver exatamente 2 elementos: `WrongType`.
   - Se `CheckOutcome::Accepted` contiver aridade diferente de 2 (incluindo qualquer payload extra): `WrongType`.
   - Se `CheckOutcome::Rejected` ou `CheckOutcome::Unknown` contiver aridade diferente de 3: `WrongType`.
3. **Tags de variantes:**
   - Inteiros fora do conjunto discriminador válido para a variante (`Exhausted`: 0..=3; `CheckOutcome`: 0..=2; `Verdict`: 0..=2): `WrongType`.
   - Valores não-inteiros na posição de tag: `WrongType`.
4. **Payloads aninhados:**
   - Payload de `CheckOutcome::Rejected` que não decodifica como `Reason` (`[1, text]`): `WrongType`.
   - Payload de `CheckOutcome::Unknown` que não decodifica como `Exhausted` (`[1, tag]`): `WrongType`.
   - Payload de `Reason` que não é string de texto canônica: `WrongType`.
5. **Campos numéricos (`Budget`):**
   - Campos `steps`, `mem_bytes`, `size_bytes`, `wall_ms` devem ser inteiros unsigned no intervalo `0..=2^64-1`.
   - Inteiros negativos (CBOR `major 1`): `IntOutOfRange`. Inteiros positivos acima de `2^64-1` não são representáveis em `canon/v1`; bignums CBOR usam tags proibidas e retornam `Tag`, não são truncados para `u64`.
   - Tipos não inteiros em campos numéricos: `WrongType`.
6. **Violações canônicas subjacentes (`canon/v1`):**
   - Cabeçalhos CBOR não mínimos: `NonMinimalHead`.
   - Bytes extras após o valor principal: `TrailingBytes`.
   - String de texto que não está em Unicode Normalization Form C: `NotNfc`.
   - String de texto contendo UTF-8 inválido: `InvalidUtf8`.
   - String de texto contendo code point não atribuído em Unicode 16: `UnassignedCodePoint`.
   - Entrada vazia ou terminada prematuramente: `Eof`.
   - Tags CBOR (major 6), valores simples indefinidos/floats (major 7) ou coleções indefinidas: `Tag`, `UnsupportedSimple`, `Indefinite`.

Erros de decodificação **nunca** são interpretados como `CheckOutcome::Unknown` ou
`Verdict::Unknown`. Falha de representação é erro de entrada/formato, não resultado de checagem.

---

## 3. Semântica de execução e contratos de API

### 3.1. Orçamentos (`Budget`)

`Budget` define os limites declarados para execução de checadores:
- `steps`: passos lógicos discretos do algoritmo. Único recurso contabilizado e limitado internamente nesta etapa.
- `mem_bytes`: limite superior declarado de memória em bytes para consumo do validador. Não monitora RSS de processo nesta etapa.
- `size_bytes`: limite superior declarado para tamanho de dados ou certificados manipulados.
- `wall_ms`: limite de tempo de relógio em milissegundos. Checkers internos ignoram `wall_ms` para manter determinismo estrito (S3) entre máquinas e arquiteturas. Supervisores externos que constatem timeout convertem o resultado em `CheckOutcome::Unknown(Exhausted::Time)` com máquina registrada pelo consumidor.

### 3.2. Medidor de passos (`StepMeter`)

- **Estado interno:** mantido em `Cell<u64>` (saldo restante) e `Cell<bool>` (latch de exaustão). Sem alocação em heap, sem sincronização multithread (não `Sync`), sem locks, sem atomics e sem relógio de sistema.
- **Ponto único de execução:** construído e executado exclusivamente por `StepMeter::run(budget, checker)`. Não possui construtor público, `Default`, `Clone` ou métodos de reinicialização.
- **Referência compartilhada:** a closure do checker recebe `&StepMeter` (nunca `&mut`), impedindo que chamadas aninhadas substituam o medidor esgotado por outro via `std::mem::swap`.
- **Cobrança (`charge`):**
  - Consome `steps` do saldo apenas quando `steps <= saldo`.
  - Limite exato consome todo o saldo e tem sucesso.
  - Orçamento zero permite zero passos de trabalho; primeira cobrança estritamente positiva falha.
  - Se `steps > saldo`, dispara irreversivelmente o latch de exaustão e retorna `Err(Exhausted::Steps)`.
  - Uma vez disparado o latch, toda e qualquer chamada subsequente a `charge` retorna `Err(Exhausted::Steps)`, inclusive `charge(0)`.
- **Precedência absoluta de exaustão (S2):**
  - Na finalização de `StepMeter::run`, se o latch de exaustão foi disparado, o retorno é **sempre** `CheckOutcome::Unknown(Exhausted::Steps)`.
  - Esta regra sobrepõe qualquer resultado retornado pela closure do checker: mesmo que o checker ignore o erro retornado por `charge` e devolva `Accepted`, `Rejected` ou outro `Unknown`, o resultado final é compulsoriamente `Unknown(Steps)`.
  - O checker deve cobrar antes de executar o trabalho e interromper no primeiro erro.

### 3.3. Construção de veredictos e fronteira de garantia

- `verified_from_proof(outcome: CheckOutcome) -> Result<Verdict, CheckOutcome>`:
  - Aceita exclusivamente `CheckOutcome::Accepted`, produzindo `Ok(Verdict::Verified)`.
  - `Rejected` e `Unknown` retornam `Err(outcome)` com o outcome original intacto (preservando `Reason` e `Exhausted`).
- `refuted_from_countermodel(outcome: CheckOutcome) -> Result<Verdict, CheckOutcome>`:
  - Aceita exclusivamente `CheckOutcome::Accepted`, produzindo `Ok(Verdict::Refuted)`.
  - `Rejected` e `Unknown` retornam `Err(outcome)` com o outcome original intacto.
- **Rejeição não é refutação (§4.4):**
  - Um certificado rejeitado (`Rejected`) indica prova inválida ou malformada; a obrigação deve retornar para o estado pendente, não ser refutada. Apenas a validação bem-sucedida de um contramodelo formal autentica `Refuted`.
- **Fronteira dos enums públicos:**
  - `Verdict` é um enum público e decodificável. Não constitui credencial criptográfica por si só.
  - A biblioteca proíbe conversões genéricas de outcome para veredicto (`From`, `Into`, `TryFrom`). A vinculação de checadores ativos e autorização de claims é responsabilidade do pipeline de verificação do kernel (Etapa 5.4).

---

## 4. Vetores de teste normativos

Os vetores de teste oficiais vivem em `spec/vectors/core/vectors.tsv`.

O arquivo utiliza formato TSV com 4 colunas separadas por caractere TAB (`\t`):

```tsv
# name	type	hex	expected
```

- `name`: identificador único do caso de teste.
- `type`: tipo de destino a ser decodificado (`Budget`, `Exhausted`, `Reason`, `CheckOutcome`, `Verdict`).
- `hex`: bytes de entrada codificados em hexadecimal minúsculo (entrada vazia é representada por string vazia).
- `expected`:
  - Casos válidos: prefixo `ok:` seguido da representação JSON canônica do valor decodificado:
    - `Budget`: `ok:{"steps":<u64>,"mem_bytes":<u64>,"size_bytes":<u64>,"wall_ms":<u64>}`
    - `Exhausted`: `ok:"Time"` | `ok:"Steps"` | `ok:"Memory"` | `ok:"Size"`
    - `Reason`: `ok:{"text":"<string>"}`
    - `CheckOutcome`:
      - `ok:"Accepted"`
      - `ok:{"Rejected":{"text":"<string>"}}`
      - `ok:{"Unknown":"Time"}` | `ok:{"Unknown":"Steps"}` | `ok:{"Unknown":"Memory"}` | `ok:{"Unknown":"Size"}`
    - `Verdict`: `ok:"Verified"` | `ok:"Refuted"` | `ok:"Unknown"`
  - Casos inválidos: prefixo `err:` seguido do nome do erro canônico:
    - `err:WrongType`
    - `err:IntOutOfRange`
    - `err:TrailingBytes`
    - `err:NonMinimalHead`
    - `err:NotNfc`
    - `err:InvalidUtf8`
    - `err:UnassignedCodePoint`
    - `err:Eof`

O gerador independente `tools/gen_core_vectors.py` monta os casos utilizando a biblioteca de referência Python `logos_client.canon` sem depender do código Rust.
O comando `python tools/gen_core_vectors.py --check` confere os vetores versionados contra o gerador; a tabela normativa acima deve ser revisada junto de qualquer mudança no gerador.
