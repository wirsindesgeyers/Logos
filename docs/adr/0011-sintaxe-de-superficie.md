# ADR 0011 — Sintaxe de superfície reversível fora da TCB

**Estado:** aceito. **Data:** 2026-10-05. **Etapa:** 1.4.

## Contexto

air/v1 admite árvores sintaticamente válidas mas mal tipadas: índices livres,
sorts não declarados, assinaturas completas inconsistentes com Vocabulary e
connectivos de zero/n filhos. Texto só com nomes resolvidos perderia essas
informações. vocabulary/v1 inclui versão, enum/fonte e três groundings que
precisam sobreviver ao debug sem alteração de h(V).

Roadmap 1.4 exige parser Python/Rust, pretty-printer reverso, propriedades e
500 fórmulas diferenciais; exclui parser da TCB, políticas, editor/LSP.

## Decisão

1. `logos-air-text` é crate separado **não listado em tcb/manifest.toml**;
   `logos_client.air` é implementação Python independente. Sem novas bibliotecas
   de parser/JSON/CBOR, sem migrar parsing para logos-air ou canon.
2. Gramática única em `spec/air-surface.md`. Uso nominal de símbolos consulta
   Vocabulary; variáveis consultam stack de binders e viram De Bruijn. Escopo
   restaurado entre irmãos, shadowing permitido; alfa-renomeação não é identidade.
3. Formas explícitas `var`, `symbol`, `app`, `pred` e constructors de conectivos
   representam toda AST sintática. Printer as usa com nomes em strings JSON;
   preserva aridade real, kind/result, sorts e ordens sem corrigir erros de tipo.
4. Printer não faz type-check. Parser valida estrutura/codificação e Vocabulary,
   mas caller invoca `logos_air::check` separadamente para tipagem/fechamento.
   Texto mal tipado não vira fórmula bem tipada por normalização/rewrite.
5. Declarações são recolhidas antes de resolver bodies Defined: forward reference
   é válida, ciclo/assinatura/sort/domínio inválido não é. Em Python, validação
   independente implementa mesmos invariantes normativos, não chama Rust.
6. Grounding omitido abrevia `Glossed("")`; gloss omitida abrevia `""`, textos
   já permitidos por vocabulary/v1. Printer expõe os campos, não presume sentido
   ou atestação. Forma `decl symbol(...)` preserva assinaturas não usuais.
7. Enum traz membros explícitos. Referência é digest hex32 ou alias fornecido
   pelo chamador; sem ledger, fetching, hash de nome ou fallback vazio. Refs,
   claims e citações permanecem opacas e não constituem eventos/propostas.
8. Document/Claim são estruturas em memória para debug. Não definem novo formato
   binário, tag de hash, autoridade de citação ou obrigatoriedade de premissa;
   air/v1/vocabulary/v1/encoder versions permanecem inalterados.
9. Erros estruturados carregam categoria e posição Unicode 1-based. Strings
   seguem JSON mais canon/v1 (NFC/Unicode16), não normalizam silenciosamente.
   Parsing de fórmula e termo usa pilhas explícitas; nenhuma cadeia de entrada
   aumenta a pilha nativa proporcionalmente. Limites lógicos air/v1 (128/100000)
   permanecem distintos do limite textual de 4096 tarefas/contextos ativos.
   Grupos transparentes não mudam profundidade lógica nem achatam operadores.
   Traversal do printer é iterativo sobre prefixo.
10. Propriedades locais nos dois lados e diferencial com AST/bytes esperados
    independentemente para 500 fórmulas; checks de byte não provam fidelidade.

## Alternativas rejeitadas

- Parser dentro da TCB: viola isolamento e escopo da etapa.
- Um parser chamando o outro: não satisfaz implementação/diferencial independente.
- Printer só nominal ou apenas bem tipado: perde assinatura/índices/erro observado.
- Dump genérico CBOR como texto de usuário: não fornece sintaxe humana pedida.
- Resolver fontes no ledger ou extrair enum de um fato: dependência futura e
  autoridade não disponível nesta etapa.
- Envelope binário próprio para claim/cites: antecipa obrigações/proposta 1.5/1.6.

## Consequências

Texto canônico de debug é verboso, mas sem ambiguidade de identidade.
Identidade/hash continuam exclusivamente nos objetos binários antigos.
Round-trip e diferencial detectam discrepância de implementação; não fazem
promessas de interpretação humana, consistência, prova ou autorização.
