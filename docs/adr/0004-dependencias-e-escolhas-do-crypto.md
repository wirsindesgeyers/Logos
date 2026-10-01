# ADR 0004 — Dependências e escolhas do `logos-crypto`

- **Status:** aceito
- **Data:** 2026-10-01
- **Etapa:** 0.4

## Dependências novas na TCB

| Crate | Papel | Licença |
| --- | --- | --- |
| `sha2` 0.10 | SHA-256 | MIT OR Apache-2.0 |
| `ed25519-dalek` 2 (`default-features = false`) | Ed25519, verificação estrita | BSD-3-Clause |
| transitivas de execução | `curve25519-dalek`, `ed25519`, `signature`, `digest`, `crypto-common`, `block-buffer`, `generic-array`, `typenum`, `subtle`, `cfg-if`, `cpufeatures` | MIT / Apache-2.0 / BSD-3-Clause |
| só de compilação | `curve25519-dalek-derive` (macro procedural), `syn`, `quote`, `proc-macro2`, `unicode-ident`, `rustc_version`, `semver`, `version_check` | MIT OR Apache-2.0 |

Medição (`cargo xtask tcb report`, host x86_64-linux): +632 LOC próprios no crate e **≈ 51 k LOC de
dependências de execução** adicionais (a maior parte é `curve25519-dalek`, 26 k, e `typenum`, 16 k),
mais ≈ 52 k LOC só de compilação (`syn` 42 k). O total de execução da TCB de correção passa de
≈ 28 k para ≈ 80 k LOC; a aritmética de curva é, de longe, a maior parte. É o custo conhecido de não
escrever criptografia à mão: o roadmap proíbe inventar criptografia, e ambos são crates muito usados.

**Alternativas recusadas.** `ring` (código C/asm em TCB, build mais pesado); `ed25519-compact`
(menos auditado); implementar Ed25519 próprio (proibido pelo princípio de “não inventar
criptografia”). Revisitar na Fase 13 (verificação formal parcial) se a proporção de LOC verificado
exigir.

`libc` e `fiat-crypto` aparecem na allowlist porque o `cargo deny` avalia todas as plataformas e
backends, mas **não** entram na medição do host: o `tcb.py` passou a usar `--filter-platform`.

## Escolhas do formato

- **`KeyId = H_key(canon(PublicKey))`**, em vez de `H("key" ‖ alg ‖ pubkey)` do roadmap: é a mesma
  ideia (domínio + algoritmo + chave), mas reaproveita a codificação canônica e o mecanismo de domínio.
  Como `alg` está dentro da chave canônica, mesmos bytes sob outro algoritmo são outra identidade.
- **Tags prefixo-livres** `logos/<nome>/v<k>\0`: `tag ‖ bytes` nunca é ambíguo. Um teste confere que
  `spec/domains.md` e as constantes do crate coincidem e que nenhuma tag é prefixo de outra.
- **`Option`/ES256:** `SigAlg` já reserva `Es256`, mas `PublicKey::new`/`Signature::new` o recusam
  (`UnsupportedAlgorithm`), então nenhum valor ES256 existe até a 9.2. A checagem de algoritmo em
  `verify` é inalcançável pela API pública; é testada por testes unitários internos.
- **Verificação estrita** (`verify_strict`): rejeita assinaturas maleáveis e componentes de ordem
  pequena. Como `PublicKey::new` já recusa chaves fracas, a diferença para o `verify` frouxo não é
  observável por testes com chaves honestas (**mutante equivalente**, documentado aqui); fica como
  defesa em profundidade.
- **Teste de tipos sem `trybuild`:** o roadmap pedia `trybuild`, que puxa dezenas de crates. Usamos
  *doctests* `compile_fail,E0308` (o rustdoc exige o código de erro, então a falha é a de tipos e
  não qualquer erro).
- **Arquivo de chave:** 64 hex minúsculos (+ `\n` opcional); regular, sem link simbólico, sem
  permissões para grupo/outros; permissão conferida no arquivo **já aberto** (fstat). Não há
  geração de chave nesta etapa (exigiria RNG): é da CLI (`logos user init`, 6.5) e do kernel.
- **Chave em memória:** o `FileSigner` mantém a semente no processo; zeragem de memória e HSM/TEE
  ficam para a Etapa 13.1. `Debug` nunca imprime a chave privada.
- **`hash` devolve `Result`**, herdando a falibilidade de `Canon::encode` (ADR 0003).
- **Hash incremental** (`RawHasher`) já aqui, para a Etapa 0.5 (certificados de gigabytes).

## Testes que sustentam estas decisões

Vetores do RFC 8032 (Ed25519 puro, via um domínio de assinatura de tag vazia só nos testes); vetores
de hash/KeyId/assinatura com domínio gerados de forma **independente** (`hashlib`, `cryptography`,
`canon.py`) por `tools/gen_crypto_vectors.py`; mutação manual de 12 bugs (11 mortos, 1 equivalente);
fuzz de `PublicKey`/`Signature`/`Hash` e do parser do arquivo de chave.
