# ADR 0003 — Dependências e escolhas do `logos-canon`

- **Status:** aceito
- **Data:** 2026-10-01
- **Etapa:** 0.3 (primeiro código de TCB)

## Dependências novas na TCB

| Crate | Versão | Para quê | Licença |
| --- | --- | --- | --- |
| `unicode-normalization` | `=0.1.24` (exata) | Checar NFC (`is_nfc`) | MIT OR Apache-2.0 |
| `tinyvec` | transitiva | Buffers pequenos da normalização | Zlib OR Apache-2.0 OR MIT |

**Por que a versão exata.** O formato fixa Unicode 16.0.0 (`spec/encoding.md` §5). A 0.1.24 traz
as tabelas de Unicode 16.0.0; a 0.1.25 já é Unicode 17. Um teste (`table_version_matches_normalization_crate`)
quebra se a versão da tabela e a do crate divergirem; atualizar o crate exige `canon/v2`.

**Alternativa recusada: escrever a NFC à mão.** Seriam milhares de linhas de tabelas geradas
dentro da TCB. Optamos por reusar um crate pequeno e muito usado, medido pela contagem da TCB
(`cargo xtask tcb report`), e cercá-lo por teste diferencial contra o `unicodedata` do Python e
por fuzz.

## Escolhas do formato

- **Rejeitar code points não atribuídos.** NFC é estável entre versões *exceto* para code points
  que passam de não atribuídos a atribuídos com decomposição. Rejeitá-los elimina a divergência
  entre implementações com tabelas de versões distintas. A tabela de atribuídos (731 intervalos)
  é **gerada** por `tools/gen_unicode_assigned.py` a partir do Python 3.14 e conferida na CI.
- **Python fixado em 3.14** (`py/.python-version`): o `unicodedata` do 3.12 é Unicode 15.0 e
  divergiria da referência. `logos_client.canon` falha na importação se a versão não for 16.0.0.
- **`MAX_DEPTH = 128`.** Sem limite, um decoder recursivo estoura a pilha com entrada hostil.
  128 acomoda árvores de fórmulas razoáveis; a AIR (Fase 1) deve preferir conectivos n-ários a
  cadeias binárias aninhadas.
- **`encode` falível.** `Canon::to_value`/`encode` devolvem `Result`: objeto fundo demais ou com
  chaves repetidas não tem codificação. A alternativa (codificar como `null` ou truncar) produziria
  bytes hasheados de algo diferente do objeto, violando P2. O `Value` canônico em si é total.
- **Sem `proptest`.** Os testes de propriedade usam um gerador próprio (SplitMix64, determinístico
  por semente, S3) e mutação de bytes. Evita ~30 crates na allowlist de `cargo deny` e dá os mesmos
  casos em qualquer máquina. Perde-se o *shrinking*; a semente impressa reproduz a falha.
- **Ordem de chaves = bytes codificados** (RFC 8949 §4.2.1), não “tamanho primeiro”.

## Consequências

- `deny.toml` passa a permitir `unicode-normalization` e `tinyvec`.
- `tcb/manifest.toml` lista os arquivos de `logos-canon`; `unicode_assigned.rs` é dado gerado,
  mas conta como TCB.
- A 0.4 (`logos-crypto`) consome `Canon::to_value`/`encode` já com `Result`.
