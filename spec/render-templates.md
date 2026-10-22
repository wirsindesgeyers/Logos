# Templates explícitos de português — vocabulary/v2

Normativo; ADR0046. Este contrato fornece os dados da9.1, não a gramática
completa de paráfrases nem a confirmação humana.

## Envelope

`vocabulary/v1 = [1, version, sorts, symbols]` permanece inalterado.
`vocabulary/v2 = [2, version, sorts, symbols, render_pt]`.
`version` é conteúdo u64; sorts/symbols têm exatamente os layouts v1.
render_pt é array de Text NFC/Unicode16; seu tamanho é igual ao número de
símbolos, e a posição i pertence à declaração symbols[i]. Zero símbolos
exige array vazio; Some([])/v2 é distinto de ausência/v1. Tags/layouts
cruzados, campos extras e templates inválidos são recusados.

`h(V) = SHA256(b"logos/vocab/v1\0" || canon(V))`, domínio histórico mantido.
O tag de formato e todos os templates fazem parte dos bytes. Não retirar
render_pt antes de calcular hash, criar O, resolver CAS ou checar modelo.
CAS mantém categoria/hash de vocabulário; não há novo formato de O/evento.

## Templates

Limite individual inclusivo1–4096 bytes UTF-8; soma inclusiva≤1048576 bytes.
Literal deve conter ao menos um caractere não branco fora dos placeholders.
Um símbolo de aridade n exige exatamente `{0}`, `{1}`, …, `{n-1}` nessa
ordem, cada um uma vez. Não aceitar zeros à esquerda, sinal, nomes, espaços,
repetição, omissão ou índices extras; chaves não são literais escapáveis.
Símbolos nularios têm somente texto literal. Exemplo de aridade2:
`{0} é dono de {1}`. Parâmetros são posições da assinatura, não nomes livres.

Rejeitar controles Unicode Cc; delimitadores reservados U+27E6/U+27E7
(⟦/⟧); U+061C; U+200B–200F; U+2028–202E; U+2060–206F; U+FEFF.
Esses pontos são reservados/restritos para exibição e parsing; isso não é
uma prova geral contra confusáveis Unicode ou significado enganoso.
NFC/Unicode16 são exigidos antes da análise; não normalizar silenciosamente.
Glosas, grounding e nomes não viram templates por inferência.

## Compatibilidade/API

Além das regras v1, compatibilidade v2 preserva o template de cada símbolo
anterior por nome, independentemente da ordem de declarações. Acrescentar
símbolo/template pode ser compatível; trocar/remover template anterior é
incompatível. v1↔v2 é incompatível, mesmo com zero símbolos. Não invalidar
claims automaticamente aqui: a integração15.7 continua dona dessa tarefa.

Rust: Vocabulary::new continua construindo v1; new_with_render_pt recebe
lista completa e constrói v2. render_pt() devolve None para v1 ou lista
validada para v2. extend de v1 permanece; em v2 exige extend_with_render_pt
com lista completa do resultado. A operação incrementa versão de conteúdo
e compara compatibilidade; nunca inventa templates para símbolos novos.

## Superfície air-text/v2

Extensão retrocompatível da gramática v1: no corpo de vocab, no máximo uma
cláusula `render_pt ["template0", "template1", ...]`, com semicolon opcional.
Strings JSON canônicas; sem trailing comma. Pode aparecer antes/depois das
declarações, mas refere-se à ordem final dos símbolos. Presença seleciona
vocabulary/v2; ausência mantém vocabulary/v1. `vocab vN` continua sendo versão
de conteúdo, não formato. render_pt é contextual; ainda pode ser nome de
símbolo/sort sem aspas nas posições de nomes da gramática anterior.
Impressoras Rust/Python emitem a cláusula após declarações; jamais descartam
metadados v2. A impressão/parse de fórmulas AIR não muda.

## Evidências reproduzíveis

Vetores em spec/vectors/vocabulary-v2/vectors.tsv, gerador independente
`python3 tools/gen_render_template_vectors.py --check`. Testes Rust/Python
conferem bytes, h(V), rejeições, limites, compatibilidade, impressão e
parsing cruzados. `ci/check-render-templates.sh` exercita10 obrigações com
v2 em produtores isolados/kernel real. Fuzz: render_template e vocabulary_decode.
