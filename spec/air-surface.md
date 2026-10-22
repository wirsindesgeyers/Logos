# Sintaxe de superfície AIR — air-text/v1

Contrato normativo da Etapa 1.4. Este é **texto não confiável**, não um novo
formato binário: saídas continuam air/v1 e vocabulary/v1. Parsing não prova
fidelidade, tipagem, fechamento, grounding nem existência de citações.

## 1. Tokens e erros

Whitespace da propriedade Unicode White_Space é ignorado (não U+001C..U+001F);
`#` inicia comentário até LF fora de strings.
Identificadores sem aspas: `[A-Za-z_][A-Za-z0-9_]*`. Palavras reservadas usadas
como nomes devem ser strings. Qualquer nome/texto canônico, inclusive vazio,
Unicode e nomes reservados, pode ser escrito como string JSON entre aspas duplas.
Strings aceitam escapes JSON, incluindo pares surrogate válidos; rejeitam
surrogates isolados, controles sem escape, texto não NFC e code points não
atribuídos em Unicode 16. Não há normalização silenciosa.
Inteiros usam dígitos ASCII decimais unsigned u64 (inclusive 0 e 18446744073709551615).
Pontuação: `{ } ( ) [ ] , : . ; = -> <->`. Tokens não reconhecidos são erros.
Ponto e vírgula entre declarações/claims é opcional. Entrada deve terminar em EOF.
Erros expõem categoria e linha/coluna 1-based (coluna em Unicode scalar values).
Categorias: Syntax, UnknownName, DuplicateName, InvalidVocabulary, InvalidAst,
InvalidText, LimitExceeded, UnknownReference. Mensagens não fazem parte da identidade.

## 2. Documentos e referências

```ebnf
document = "vocab", version, "{", { declaration, [";"] }, "}",
           { "claim", name, ":", formula, ["cites", "[", [name, {",", name}], "]"], [";"] } ;
version = identifier ; (* neste contexto somente: v[0-9]+, decimal u64 *)
declaration = "sort", name, ["=", "enum", "[", [name, {",", name}], "]",
                "from", ("fact" | "policy"), reference]
            | "const", name, ":", name, [grounding], ["gloss", string]
            | "func", name, "(", [name, {",", name}], ")", ":", name,
                [grounding], ["gloss", string]
            | "pred", name, "(", [name, {",", name}], ")",
                [grounding], ["gloss", string]
            | "decl", symbol, grounding, "gloss", string ;
grounding = "=", ("glossed", "(", string, ")"
                 | "anchored", "(", string, ",", string, ")"
                 | "defined", "(", formula, ")") ;
reference = name ;
name = identifier | string ;
```

`v0`/`v1`/`v42` são identificadores interpretados como versão apenas após
`vocab`; continuam nomes válidos nos outros contextos, inclusive `v01` e
`v12name`. Uma versão não pode ter espaço entre `v` e seus dígitos.

`vN` define a versão numérica do Vocabulary, não o encoder. `sort S` declara
Open. `enum [...]` preserva ordem dos membros; nomes devem resolver para
constantes declaradas com a assinatura exigida por vocabulary/v1. A fonte é
um digest de 32 bytes, escrito como string de 64 dígitos hex, ou alias presente
no mapa de referências fornecido pelo chamador. Sem resolver fatos/ledger,
sem extrair/inventar membros do domínio e sem hash de alias como substituto.
Hex não distingue maiúsculas/minúsculas; o printer escreve minúsculas.

Const/Func/Pred introduzem a assinatura usual. Grounding omitido é explicitamente
Glossed("") e gloss omitido é "": abreviações de texto vazio permitido pelo
formato, não significado ou atestação inferida. `decl symbol(...)` permite todas
as assinaturas admitidas por air/v1, inclusive combinações que o sort-check
recusará. Groundings Defined são resolvidos após coletar todas as assinaturas,
permitindo forward references; ciclos e todas as invariantes de vocabulary/v1
são recusados. Ordem de declarações de sorts/símbolos é preservada nas respectivas
listas canônicas. Nomes duplicados em cada namespace são erro.

Claim é uma estrutura de debug em memória: nome, Formula e lista ordenada de
nomes de citações. Nomes de claims são únicos; citações mantêm ordem e repetição.
Não existe envelope binário/hash novo para claim/documento. F3/P2/C5 são nomes
opacos: não viram PremiseRef, Verified, política ou autoridade. Omitir cites
significa lista vazia. Zero claims é admitido.

Exemplo completo (o exemplo resumido do ROADMAP não especifica membros do
enum nem declara Action; aqui ambos são explícitos):

```text
vocab v1 {
  sort User = enum [alice] from fact "0000000000000000000000000000000000000000000000000000000000000001"
  sort Db
  sort Action
  const alice: User = glossed("usuário Alice") gloss "Alice"
  pred Admin(User)
  pred Owns(User, Db)
  pred Allowed(Action)
}
claim C7: forall u:User. Admin(u) -> exists d:Db. Owns(u, d)
  cites [F3, P2, C5]
```

## 3. Fórmulas e termos

Precedência da menor à maior: `<->` (associatividade esquerda), `->`
(associatividade direita), `or` (esquerda), `and` (esquerda), `not`, átomos.
Quantificador nominal `forall x:S. f` / `exists x:S. f` consome uma fórmula
completa até delimitador externo (comma, `)`, `]`, `}`, `cites`, `claim`, `;`, EOF).
Parênteses agrupam fórmulas. Operadores binários and/or criam nós de exatamente
2 filhos; não achatam conectivos. Não se simplifica, reordena ou deduplica.

```ebnf
atom = "true" | "false" | "(", formula, ")" | term, "=", term
     | name, ["(", [term, {",", term}], ")"]
     | "eq", "(", term, ",", term, ")"
     | "pred", "(", symbol, ",", "[", [term, {",", term}], "]", ")"
     | "not", "(", formula, ")"
     | ("and" | "or"), "(", "[", [formula, {",", formula}], "]", ")"
     | ("implies" | "iff"), "(", formula, ",", formula, ")"
     | ("forall" | "exists"), name, ":", name, ".", formula ;
term = name, ["(", [term, {",", term}], ")"]
     | "var", "(", uint, ",", name, ")"
     | "app", "(", symbol, ",", "[", [term, {",", term}], "]", ")" ;
symbol = "symbol", "(", name, ",", ("const" | "func" | "pred"), ",",
         "[", [name, {",", name}], "]", ",", ("bool" | name), ")" ;
```

Nomes de aplicação resolvem assinatura integral no Vocabulary. Variável nominal
resolve binder mais próximo, com sort desse binder e índice De Bruijn; shadowing
é permitido, escopo não vaza entre irmãos. Um nome de variável sem binder nem
símbolo declarado é erro. Bindings só interceptam nomes sem `(...)`: `x()` é
aplicação do símbolo x, mesmo se existir binder x. Aplicação usada como átomo sem
`=` vira Pred; não se faz type-check implícito de kind/result/aridade.

`var(index, sort)` permite todos os índices/anotações de air/v1, inclusive livres
ou incoerentes. `app`/`pred` com `symbol` explícito não consultam Vocabulary e
preservam assinaturas não declaradas, counts diferentes da aridade e combinações
sintáticas mal tipadas. Sort explícito não exige declaração. Em result de symbol,
`bool` sem aspas significa Bool, enquanto `"bool"` significa Sort de nome bool.
Sort nominal de quantificador também constrói seu nome sem exigir declaração;
isso permite imprimir/reler um Forall com sort desconhecido. Referências em
Defined continuam sujeitas à validação integral do vocabulário.
Normal parse usa as declarações; forma explícita garante debug reversível de
**toda** AST admitida pelos limites, sem esconder erro de tipagem.

Construção valida prefixo e limites agregados air/v1 (profundidade 128,
100000 nós por fórmula, inclusive termos), também dentro de Defined. Fórmulas
e termos são analisados por pilhas explícitas, não recursão proporcional à entrada.
O teto adicional de nesting textual é 4096 tarefas/contextos ativos de parsing;
excesso produz LimitExceeded, não stack overflow/RecursionError. Parênteses
transparentes não aumentam profundidade lógica nem achatam operações entre grupos.
Printer percorre a AST plana e não faz sort-check. Não normaliza a árvore.

## 4. Pretty-print e APIs

Pretty-printer determinístico usa strings JSON para todos os nomes/metadados,
`var`, assinaturas `symbol`, `app`/`pred`, conectivos por constructor e
quantificadores nominais gerados `_q<índice do record>`. Valores True/False usam
`true`/`false`. Declarações usam `decl symbol(...) = ... gloss ...`; enums usam
membros explícitos e fontes hex. Namespaces, grounding, gloss, version, citações
e ordens são preservados. Print não consulta ledger nem usa glosa como assinatura.

Rust (`logos-air-text`, fora da TCB):
- `parse_formula(text: &str, vocabulary: &Vocabulary) -> Result<Formula, SurfaceError>`.
- `print_formula(formula: &Formula) -> String`.
- `References = BTreeMap<String, [u8; 32]>`.
- `parse_document(text: &str, references: &References) -> Result<Document, SurfaceError>`.
- `print_document(document: &Document) -> String`.
- `Document { vocabulary: Vocabulary, claims: Vec<Claim> }`.
- `Claim { name: Text, formula: Formula, cites: Vec<Text> }`.
- `SurfaceError { kind: ErrorKind, line: usize, column: usize, message: String }`.

Python (`logos_client.air`, independente de Rust): mesmas operações snake_case,
`parse_formula(text, vocabulary=None)` retorna Value canon/v1 (envelope Formula).
Contexto None não declara símbolos. Nomes nominais de símbolos exigem Vocabulary;
binders e assinaturas explícitas independem dele. None não é fallback para
um Vocabulary inválido.
`print_formula(value)` aceita Value. Document/Claim dataclasses com vocabulary/
formula como Value e nomes/cites como str/list[str]. `parse_document(text,
references=None)` aceita aliases str → bytes32. SurfaceError tem kind (str),
line/column/message. Usar logos_client.canon para texto e bytes, sem novo encoder.

## 5. Contratos verificáveis

Dentro dos limites suportados, `parse_formula(print_formula(f), V) == f`,
inclusive fórmulas sintaticamente válidas mas mal tipadas; também round-trip de
documento com todas as variantes de grounding/domínio. Alfa-renomeação de binder
não muda bytes. Diferencial obrigatório de 500 fórmulas com bytes Formula e
Vocabulary idênticos em Python/Rust; Python verifica também texto impresso em Rust.

## Extensão air-text/v2 — templates (ADR0046)

A gramática v1 permanece aceita. A cláusula contextual opcional render_pt no
corpo de vocab, impressão e seleção de vocabulary/v2 são definidas em
[render-templates.md](render-templates.md). Sua ausência mantém bytes v1.
Não há mudança na gramática de fórmulas nem na identidade AIR.
