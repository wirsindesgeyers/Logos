# Vetores e corpus LRAT RUP

`vectors.json`:14 vetores manuais independentes, gerados por
`rtk proxy python tools/gen_lrat_vectors.py --check`. Fixam hashes separados
CNF/certificate e fecho de IDs, positivos e negativos.

`sat2002.json.xz`: corpus histórico offline, JSON comprimido com XZ/LZMA2 preset6.
`corpus-manifest.json`: identidade do arquivo, fontes individuais, estatísticas
por caso. Os bytes de entrada seguem cnf/v1. O harness Rust usa Python padrão
somente para descompressão/JSON; o checker não depende de Python/lzma/solver.

Fonte primária: [SATLIB, SAT-02 Challenge Set](https://www.cs.ubc.ca/~hoos/SATLIB/),
contribuído por Laurent Simon e Daniel Le Berre. Citação SATLIB: Holger H. Hoos
e Thomas Stützle, SATLIB: An Online Resource for Research on SAT, SAT2000,
pp.283–292. Nomes completos dos contribuidores/famílias estão nos caminhos.
Não são instâncias geradas pelo LOGOS nem variantes contadas de uma mesma CNF.

Arquivo original:
https://www.cs.ubc.ca/~hoos/SATLIB/Benchmarks/SAT/New/Competition-02/sat-2002-beta.tgz
(146991367 bytes, SHA256
`90be16ce81b54ccfc3b93b5633d746dbf21dc7dcedde5211a13793615a0f87bc`).
Cada caso preserva SHA256 dos bytes originais antes de canonicalizar.

Preparação fora da TCB: remover comentários, ordenar literais/cláusulas sem
remover duplicatas/tautologias, emitir cnf/v1. Gerar LRAT textual com CaDiCaL
revisão `c60730422e758ef1cebe7aeddf2dda31c996bf04` (build `./configure --quiet`,
`make -j4`); opções exatas por caso no fixture. Não há limite de tempo na prova
salva: só certificados completos que terminaram UNSAT e foram rechecados.
A opção --plain aparece em alguns casos; todos usam --lrat --no-binary.
Guardar hash da prova bruta, podar adições fora do fecho da primeira vazia e
remover deleções por `trim` em tools/lrat_corpus.py; revalidar RUP e used com
oráculo Python de conjuntos/dicionários. CNF original inteira é preservada.
Essa poda prepara fixtures, sem expor produtor SAT ou integrar AIR/modelos.

Comandos de verificação offline:

```sh
rtk proxy python tools/lrat_corpus.py --check
rtk proxy cargo test -p logos-lrat --test corpus -- --nocapture
```

Reprodução das provas usando arquivo original e binário compilado da revisão
acima (confere originais, canonicalização, hash da prova bruta e poda exata):

```sh
rtk proxy python tools/lrat_corpus.py --archive target/lrat-corpus-source/sat-2002-beta.tgz --solver target/lrat-corpus-source/cadical/build/cadical
```

A reprodução extrai somente por leitura de cada membro, sem descompactar
caminhos fornecidos pelo arquivo. Nenhum binário externo é confiável para o
resultado Rust. As corrupções substituem o primeiro hint por referência futura,
alteram a cláusula vazia final para unitária e substituem a prova por uma
vazia sem derivação. Há ainda
regressões semânticas de falsa unidade, polaridade, deleções e cláusulas alteradas,
e28.160 traces de hints contra enumeração independente. cake_lpr fica na4.5.
