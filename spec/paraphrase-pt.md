# Paráfrase determinística em português controlado — paraphrase-pt/v1

Normativo; Etapa 9.1, [ADR 0050](../docs/adr/0050-gramatica-paraphrase-pt.md).
Implementação: `logos-seal` (TCB de fidelidade). Templates por símbolo vêm de
vocabulary/v2 ([render-templates.md](render-templates.md), ADR 0046).

`render_pt(φ, V)` produz o texto que o usuário lê e, na 9.2, assina.
`parse_pt(texto, V)` devolve a fórmula. Contrato: para toda `φ` em que
`render_pt` tem sucesso, `parse_pt(render_pt(φ, V), V) == φ`; e todo texto que
`parse_pt` aceita é exatamente `render_pt` da fórmula lida (uma grafia por fórmula).

## 1. Pré-condições

`V` precisa ser vocabulary/v2 e passar estas checagens, feitas uma vez por
vocabulário (erros entre parênteses):

1. ter `render_pt` (`MissingTemplates`);
2. não haver dois predicados com a mesma lista de segmentos literais, nem dois
   símbolos de termo (constante ou função) com a mesma lista (`AmbiguousTemplates`).
   Predicados e termos vivem em contextos distintos e podem repetir texto entre si;
3. nenhum template de termo sem placeholders ter a forma de variável `x` seguido de
   decimal canônico ≥ 1 (`VariableNameCollision`);
4. nenhum nome de sort conter `⟦` (U+27E6) ou `⟧` (U+27E7) (`UnrenderableSortName`).

Segmentos literais de um template de aridade `n` são os `n+1` trechos ao redor de
`{0}`…`{n-1}`, possivelmente vazios. Ex.: `{0} é dono de {1}` → `["", " é dono de ", ""]`.

`φ` precisa ser fechada e passar o sort-check contra `V` (`Sort`), com
profundidade e nós dentro dos limites (padrão: os de air/v1, profundidade 128 e
100 000 nós; `DepthExceeded`). O texto tem no máximo 1 MiB UTF-8 (`TooLarge`).

## 2. Gramática

Texto entre aspas é literal, byte a byte (UTF-8, NFC). Não há espaço opcional,
maiúsculas alternativas, sinônimos nem normalização.

```ebnf
F     = "verdadeiro" | "falso"
      | "o valor " , SLOT , " é igual a " , SLOT                       (* Eq *)
      | "⟦" , ATOM , "⟧"                                               (* Pred *)
      | "não é o caso que (" , F , ")"                                 (* Not *)
      | "vale cada uma de " , N , COND , LIST                          (* And(N) *)
      | "vale pelo menos uma de " , N , COND , LIST                    (* Or(N) *)
      | "se (" , F , "), então (" , F , ")"                            (* Implies *)
      | "(" , F , ") se e somente se (" , F , ")"                      (* Iff *)
      | "para todo " , VAR , " do tipo ⟦" , SORT , "⟧, vale que (" , F , ")"
      | "existe " , VAR , " do tipo ⟦" , SORT , "⟧ tal que (" , F , ")" ;
COND  = " condição" (* se N = 1 *) | " condições" (* caso contrário *) ;
LIST  = "" (* se N = 0 *) | ": (" , F , ")" , { "; (" , F , ")" } (* N itens *) ;
N     = "0" | dígito não zero , { dígito } ;
VAR   = "x" , dígito não zero , { dígito } ;
SLOT  = "⟦" , TERM , "⟧" ;
TERM  = VAR | template de termo preenchido ;
ATOM  = template de predicado preenchido ;
SORT  = nome do sort, sem "⟦" nem "⟧" ;
```

**Template preenchido.** Para o símbolo `s` de aridade `n`: segmento 0, `SLOT` do
argumento 0, segmento 1, …, `SLOT` do argumento `n-1`, segmento `n`. O símbolo é
identificado pela lista de segmentos (única no contexto, §1); o parser lê o
conteúdo de `⟦ ⟧` como texto até o próximo `⟦` ou `⟧`, e cada `⟦` interno abre
um `SLOT`. Templates não contêm `⟦`, `⟧`, `{` nem `}` literais (ADR 0046), então
todo `⟦ ⟧` do texto é estrutura.

**Variáveis.** O binder no nível `k` (1 = mais externo) é `xk`; o quantificador
só aceita o próximo nível. A variável De Bruijn de índice `i` sob `d` binders é
`x(d-i)`; ao ler, `xk` com `1 ≤ k ≤ d` vira índice `d-k` com o sort do binder `k`
(`UnboundVariable` fora disso).

**Contagem.** `And`/`Or` com `N` filhos escrevem `N` em decimal canônico; o
número e a forma singular/plural precisam bater com a lista (`Syntax`).

## 3. Leitura

`parse_pt` faz descida recursiva estrita com profundidade limitada, recusa
qualquer byte fora da gramática (`Syntax { offset }`), template desconhecido
(`UnknownTemplate`), sort não declarado (`UnknownSort`) e bytes após a fórmula.
O resultado passa por `Formula::new` e pelo sort-check; não há valor padrão nem
recuperação de erro.

`render_pt` lê de volta o próprio texto e recusa com `RoundTripMismatch` se não
obtiver `φ`. Com as pré-condições do §1 isso não deve ocorrer; a releitura torna a
injetividade independente dessa análise: render e parse são funções, logo
`render(φ1) = render(φ2)` implicaria `φ1 = parse(render(φ1)) = φ2`.

## 4. Hash da paráfrase

```text
paraphrase/v1 = [1, "paraphrase-pt/v1", "logos-seal/render_pt/1", h(V) bytes(32), texto]
h(P)          = SHA256("logos/paraphrase/v1\0" || canon(paraphrase/v1))
```

O domínio `logos/paraphrase/v1` já estava reservado ([domains.md](domains.md)).
O hash fixa gramática, versão do renderizador, vocabulário (templates inclusos em
`h(V)`) e texto exato. Mesmo texto com outro `V` tem outro `h(P)`. Mudança de
frase fixa, regra ou forma do registro é `paraphrase-pt/v2`, nunca edição da v1.

## 5. Exemplo

`Owns(u,d)` com template `{0} é dono de {1}` e `CanRead` com `{0} pode ler {1}`:

```text
∀u:Usuário. ∀d:Documento. Owns(u,d) → CanRead(u,d)
para todo x1 do tipo ⟦Usuário⟧, vale que (para todo x2 do tipo ⟦Documento⟧, vale que (se (⟦⟦x1⟧ é dono de ⟦x2⟧⟧), então (⟦⟦x1⟧ pode ler ⟦x2⟧⟧)))
```

## 6. O que a paráfrase não garante

Injetividade e ida e volta são propriedades da gramática, não do significado: um
template pode estar mal escrito, ter concordância errada ou não corresponder à
intenção de quem escreveu a política. Por isso o critério da 9.1 inclui revisão
humana registrada ([docs/plans/etapa-9.1-revisao](../docs/plans/etapa-9.1-revisao/README.md)).
Confusáveis Unicode dentro de templates continuam possíveis (render-templates.md).
O usuário pode assinar texto que não leu com atenção (L1, ameaça 16).

## 7. Evidências reproduzíveis

- `tools/gen_paraphrase_vectors.py --check`: renderizador Python independente, 17
  textos/`h(P)` e 12 recusas em `spec/vectors/paraphrase/vectors.tsv`.
- `cargo test -p logos-seal`: vetores, 100 000 fórmulas distintas até profundidade 6,
  todas as fórmulas de `spec/vectors/`, recusas e limites.
- Fuzz `paraphrase_parse` (`ci/fuzz.sh`): todo texto aceito é o render da fórmula lida.
