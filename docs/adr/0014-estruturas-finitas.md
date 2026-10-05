# ADR 0014 — estruturas finitas e model/v1

- Estado: aceito.
- Data: 2026-10-05.
- Escopo: ROADMAP 2.1; não implementa avaliação/expansão de fórmulas da 2.2.

## Contexto

Vocabulary preserva ordem de declarações e h(V) identifica todo esse conteúdo.
AIR permite assinaturas sintaticamente representáveis mas semanticamente inválidas.
Modelos precisam impedir funções parciais, tabelas injetadas para Defined,
valores fora de sort e produtos grandes antes de alocar tabelas tipadas.

## Decisão

1. `FiniteModel` imutável, construído somente com `ModelData`, Vocabulary real e
   limites, ou decoder contextual. Conferir h(V) calculado do Vocabulary recebido.
   Não implementar CanonDecode sem contexto, nem emitir Accepted/Refuted/Verified.
2. Índices u32 referem-se às posições em `sorts()`/`symbols()` de V exato.
   Cardinalidades seguem ordem de sorts; interpretações seguem IDs de símbolos
   estritamente crescentes em cada categoria. Sem deduplicação/normalização.
3. Domínios são não vazios, elementos inteiros de 0 até n−1. Enumerated tem n
   igual ao número de constantes declaradas; constante na posição i denota i.
   Constantes primitivas devem respeitar esse índice, sem permutação. Constantes
   Defined enumeradas têm rótulo intrínseco, sem tabela: compatibilidade da
   expansão com esse rótulo é obrigação futura da 2.2, não prova feita na 2.1.
   Enumeração vazia é Vocabulary representável, mas não admite modelo clássico.
4. Exatamente uma interpretação por símbolo primitivo, nenhuma para Defined.
   Conferir kind/resultado/aridade de todas as assinaturas. Func sem argumentos
   tem um resultado; Pred sem argumentos é falso com conjunto vazio e verdadeiro
   com conjunto contendo uma tupla vazia.
5. Funções densas, último eixo mais rápido; wire contém bytes de u32 big-endian.
   Predicados contêm tuplas lexicograficamente crescentes, sem repetições.
   Packed functions evitam objeto Value por célula durante decoder canônico.
6. `model/v1`: `[1, hV_bytes32, domains, consts, funcs, preds]`; domínio de hash
   reservado `logos/model/v1\0`. Nenhum formato antigo muda; qualquer mudança
   incompatível desse envelope exige model/v2 e vetores novos.
7. Limites inclusivos: 1 MiB wire, 65.536 elementos por sort, 100.000 células
   agregadas por padrão. Células: constantes + outputs funcionais +
   `sum(max(1, aridade))` das tuplas de predicados. Produtos funcionais esperados
   contam mesmo se input estiver parcial. Overflow e excesso => Unknown(Size),
   reserva falível de buffers tipados => Unknown(Memory), nunca modelo truncado.
   Decoder canon existente é limitado em bytes antes de ler Value; não é limite
   rígido de RSS. Produtos/células são conferidos antes de alocar tabelas tipadas.
8. Dependências runtime workspace logos-air, logos-canon, logos-crypto e
   logos-core, já allowlisted em deny.toml. Sem pacote externo novo runtime.
   serde_json só dev para vetores, já no lockfile/allowlist.

## Alternativas e consequências

Nomes/assinaturas completas por entrada duplicariam V e exigiriam reconciliação.
Mapas permitem ordenação adicional desnecessária; vetores posicionais vinculados
por h(V) são menores. Funções esparsas exigiriam verificar cobertura de todas as
chaves; tabelas densas tornam totalidade igualdade de tamanho com produto.
Tuplas de predicados permanecem explícitas para conferir sorts por eixo.

Usar Decoder/Canon existente evita um segundo parser CBOR confiável. O custo de
Value para tuplas é limitado por bytes, não pelo max_cells antes do parse genérico;
max_cells é limite de estrutura tipada/trabalho funcional. Limites não entram em
hash. Encode contextual preserva motivo Unknown; Canon usa CanonError quando não
há codificação válida. h(V) não autentica grounding/fontes, e modelo estrutural
válido não demonstra consistência ou refutação de nenhuma obrigação.
