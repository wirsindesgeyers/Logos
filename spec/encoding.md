# Codificação canônica — `canon/v1`

Especificação normativa (roadmap, Etapa 0.3). Quem ler só este documento e os vetores em
[`vectors/canon/vectors.tsv`](vectors/canon/vectors.tsv) deve conseguir reimplementar a
codificação em outra linguagem e produzir exatamente os mesmos bytes. As palavras DEVE, NÃO DEVE
e PODE têm o sentido usual de especificação.

Implementações neste repositório: `crates/logos-canon` (Rust, TCB) e `py/logos_client/canon.py`
(Python, referência independente, fora da TCB). Todo objeto que o LOGOS hasheia ou assina é
serializado assim (princípio P2: o que é hasheado é exatamente o que é checado).

## 1. Identificação e versão

- Identificador: `canon/v1` (constante `CANON_VERSION`). Entra nas obrigações (spec conceitual §4,
  passo 1). Qualquer mudança de regra deste documento é uma nova versão (`canon/v2`) e exige
  novos vetores.
- Versão de Unicode fixada: **16.0.0** (constante `UNICODE_VERSION`), ver §5.
- Profundidade máxima: **128** (`MAX_DEPTH`), ver §6.

## 2. Modelo de valores

Um valor é exatamente um de:

| Valor | CBOR | Observação |
| --- | --- | --- |
| `null` | `f6` | |
| `false` / `true` | `f4` / `f5` | |
| inteiro `n` com `-2^64 ≤ n ≤ 2^64-1` | tipo maior 0 (`n ≥ 0`) ou 1 (`n < 0`, argumento `-1-n`) | fora da faixa: não tem codificação |
| string de bytes | tipo maior 2 | |
| string de texto | tipo maior 3 | UTF-8, ver §5 |
| array | tipo maior 4 | |
| mapa | tipo maior 5 | chaves são valores quaisquer, ver §4 |

É o subconjunto determinístico de CBOR (RFC 8949 §4.2.1) **sem**: floats (tipo 7, info 25–27),
`undefined` (`f7`), outros valores simples (0–19 e `f8 xx`), tags (tipo 6, inclusive bignums),
comprimentos indefinidos e `break`.

## 3. Cabeçalhos

Cada item começa por um byte inicial `MMMIIIII` (tipo maior `M`, informação adicional `I`).

- `I < 24`: o argumento é `I`.
- `I = 24, 25, 26, 27`: o argumento está nos 1, 2, 4 ou 8 bytes seguintes, em big-endian.
- **Menor forma obrigatória:** o argumento DEVE usar a menor forma possível. Ou seja, com `I = 24`
  o valor DEVE ser ≥ 24; com `I = 25`, ≥ 256; com `I = 26`, ≥ 65536; com `I = 27`, ≥ 2^32.
- Para strings (tipos 2 e 3) o argumento é o tamanho em **bytes**; para arrays, o número de
  elementos; para mapas, o número de entradas.

O encoder escolhe sempre a menor forma. O decoder rejeita qualquer outra (`NonMinimalHead`).

## 4. Mapas

- Cada entrada é `chave valor`, ambas codificadas conforme esta especificação.
- As entradas DEVEM estar em ordem **estritamente crescente** pela comparação lexicográfica,
  byte a byte, das **codificações** das chaves (RFC 8949 §4.2.1; não é a ordem “por tamanho
  primeiro” da RFC 7049). Um prefixo é menor que a cadeia mais longa.
- Duas chaves com a mesma codificação são a mesma chave: é proibido repetir (`DuplicateKey`).
  Chaves em ordem decrescente: `MapKeyOrder`.
- Exemplo: as chaves `10`, `-1`, `"b"`, `"aa"`, `false` codificam como `0a`, `20`, `61 62`,
  `62 61 61`, `f4` e é essa a ordem canônica.

O encoder recebe as entradas em qualquer ordem e as ordena; a ordem de inserção nunca afeta os
bytes.

## 5. Texto

O conteúdo de uma string de texto DEVE ser:

1. **UTF-8 válido** (sem sequências sobrelongas, sem substitutos U+D800–DFFF, sem code points
   acima de U+10FFFF);
2. composto só de **code points atribuídos** em Unicode 16.0.0, isto é, com categoria geral
   diferente de `Cn`. Uso privado (`Co`) conta como atribuído; não-caracteres (U+FFFE, U+FFFF,
   U+FDD0–U+FDEF…) não;
3. estar em **NFC** (forma de normalização C) segundo Unicode 16.0.0.

Motivo da regra 2: a estabilidade de normalização garante que texto NFC numa versão continua NFC
nas seguintes, **exceto** code points que eram não atribuídos e passaram a ter decomposição.
Rejeitar não atribuídos elimina a única fonte de divergência entre implementações com tabelas de
versões diferentes.

Ordem de verificação (determina o erro quando há mais de um problema): `InvalidUtf8`, depois
`UnassignedCodePoint`, depois `NotNfc`.

Ao mudar a versão de Unicode, `canon/v2` é necessário (textos hoje rejeitados passariam a ser
aceitos, e vice-versa para NFC de caracteres reclassificados).

`tools/gen_unicode_assigned.py` gera a tabela de code points atribuídos usada pelo crate Rust a
partir do `unicodedata` do Python 3.14 (Unicode 16.0.0); a CI confere que o arquivo está
atualizado. A NFC em Rust usa `unicode-normalization =0.1.24` (Unicode 16.0.0), e um teste afirma
que a versão coincide com a da tabela.

## 6. Profundidade

Um escalar tem profundidade 0; um array ou mapa tem profundidade `1 + máximo` entre seus
elementos (e, para mapas, chaves e valores). Profundidade > 128 é inválida (`DepthExceeded`):
o encoder recusa o valor e o decoder rejeita a entrada ao **entrar** no 129º nível de
aninhamento, antes de ler seus elementos. Isso limita a pilha de qualquer implementação.

## 7. Decodificação estrita

O decoder lê exatamente um item e DEVE então estar no fim da entrada (`TrailingBytes`).
Aceita somente bytes que o encoder produziria; portanto, para toda entrada aceita `b`,
`encode(decode(b)) = b`, e para todo valor `v`, `decode(encode(v)) = v`.

Para um byte inicial `MMMIIIII`, nesta ordem:

1. entrada vazia: `Eof`;
2. `M = 6` (tag): `Tag`;
3. `M = 7`: `I = 20, 21, 22` → `false`, `true`, `null`; `I = 28–30` → `ReservedInfo`;
   `I = 31` → `Indefinite` (`break`); qualquer outro → `UnsupportedSimple`;
4. `I = 28–30`: `ReservedInfo`; `I = 31`: `Indefinite` se `M ≥ 2`, senão `ReservedInfo`;
5. ler o argumento (`Eof` se faltar byte; depois `NonMinimalHead`);
6. strings: faltando bytes → `Eof`; texto: regras de §5;
7. array/mapa: profundidade (§6), depois os elementos em ordem. Num mapa, a ordem das chaves é
   conferida assim que cada chave termina de ser lida, **antes** de ler seu valor
   (`DuplicateKey` se igual à anterior, `MapKeyOrder` se menor).

O **primeiro** erro encontrado, lendo da esquerda para a direita, é o reportado. Um decoder NÃO
DEVE alocar memória proporcional a um comprimento declarado antes de confirmar que os bytes
existem.

## 8. Erros

Nomes estáveis, usados pelos vetores e por ambas as implementações: `Eof`, `TrailingBytes`,
`NonMinimalHead`, `ReservedInfo`, `Indefinite`, `Tag`, `UnsupportedSimple`, `InvalidUtf8`,
`UnassignedCodePoint`, `NotNfc`, `MapKeyOrder`, `DuplicateKey`, `DepthExceeded`.
Só na construção de valores (não na decodificação): `IntOutOfRange`; só na conversão para tipos
tipados: `WrongType`.

## 9. Renderização de referência (`dump`)

Texto determinístico de um valor, usado pelos vetores e pelo teste diferencial para comparar
**valores** (não só bytes) entre implementações:

- `null`, `true`, `false`; inteiros em decimal (`-1`, `18446744073709551615`);
- bytes: `h'` + hexadecimal minúsculo + `'`;
- texto: `t"` … `"`, onde cada code point é: `"` → `\"`; `\` → `\\`; ASCII imprimível
  U+0020–U+007E → o próprio caractere; qualquer outro → `\u{hex}` com hexadecimal minúsculo e
  sem zeros à esquerda;
- array: `[a,b,c]`; mapa: `{k:v,k:v}` na ordem canônica; sem espaços.

## 10. API Rust

- `Canon::to_value` e `Canon::encode` retornam `Result`: um objeto que aninha mais que
  `MAX_DEPTH` ou tem chaves repetidas **não** tem codificação, e nunca é codificado como outra
  coisa. (O `Value` em si é total: texto, ordem de chaves e profundidade são validados na
  construção.)
- `CanonDecode::decode` e `from_canon` usam o decoder estrito; `from_canon` exige o fim da
  entrada.
- `Vec<u8>` é um **array de inteiros**; strings de bytes são `Bytes` ou `[u8; N]`.

## 11. Vetores e testes

- `vectors/canon/vectors.tsv`: `nome<TAB>hex<TAB>esperado`, com `esperado` = `ok:<dump>` ou
  `err:<Kind>`. As expectativas foram escritas à mão a partir desta especificação, não geradas
  por uma das implementações.
- Teste diferencial: `py/tests/test_canon_differential.py` (100k casos em cada direção).
- Fuzz: `fuzz/fuzz_targets/canon_decode.rs` e `canon_text.rs`.
