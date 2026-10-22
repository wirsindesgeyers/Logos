# Revisão humana das paráfrases — Etapa 9.1

Terceiro critério da 9.1: *"Revisão humana de 50 paráfrases de políticas reais:
nenhuma lida de forma errada (registrar a revisão)."* Este diretório prepara o
material; **a revisão não foi feita** e nenhum script a substitui.

| Arquivo | Conteúdo |
| --- | --- |
| `politicas.air` | Vocabulário vocabulary/v2 (templates `render_pt`) e 50 claims P01–P50 em air-text/v2 |
| `intencoes.tsv` | Para cada claim, a intenção em português livre, escrita antes da formalização |
| `planilha.md` | Gerada: intenção, paráfrase exata de `render_pt` e `h(P)` de cada claim |

Regerar ou conferir a planilha (o `verify.sh` roda `--check`):

```sh
cargo run -p logos-seal --example review_sheet            # grava planilha.md
cargo run -p logos-seal --example review_sheet -- --check # confere
```

## Pendência antes de revisar: origem das políticas

As 50 políticas foram **redigidas pelo agente** no molde de políticas comuns de
controle de acesso e governança de dados (papéis, retenção legal, dados pessoais
sensíveis, colaboradores externos, incidentes). O critério pede políticas
**reais**. O mantenedor decide uma das opções e registra abaixo:

1. aceitar este corpus como representativo (registrar a justificativa); ou
2. substituir parte ou todo o corpus por políticas reais (de uma instituição,
   repositório ou norma), mantendo 50 claims e intenções escritas por quem
   conhece a política, e regerar a planilha.

## Protocolo

1. Ler a **intenção** e depois a **paráfrase exatamente como exibida**, sem
   consultar a AIR. A paráfrase é o que um usuário assinaria na 9.2.
2. Marcar "sim" só se a paráfrase disser a mesma coisa que a intenção, inclusive
   quantificadores e seu escopo, negações, condições e "exatamente quando".
3. Marcar "não" se qualquer leitura plausível divergir. Anotar o motivo:
   gramática (frases fixas), template de símbolo, formalização (a AIR não diz o
   que a intenção diz) ou intenção ambígua.
4. Uma leitura errada reprova o critério; a correção (template, formalização ou
   gramática) exige nova planilha e nova revisão das claims afetadas.

Regra de redação adotada (22/out, decisão do agente por delegação): um template
por símbolo não flexiona gênero conforme o argumento, então usa construções
neutras em relação a ele. A primeira versão tinha `{0} é público`, que com "a
pasta que contém …" gerava concordância errada (P25); passou a `{0} tem acesso
público`. Pelo mesmo motivo: `{0} está em arquivo`, `{0} tem vínculo externo`,
`{0} participa de {1}`. O revisor ainda avalia se cada frase lê corretamente.

## Registro da revisão

| Campo | Valor |
| --- | --- |
| Origem das políticas (opção 1 ou 2 acima, com justificativa) | *pendente* |
| Revisor(es) | *pendente* |
| Data | *pendente* |
| `h(V)` revisado (cabeçalho da planilha) | *pendente* |
| Claims lidas corretamente / total | *pendente* |
| Claims com leitura errada e motivo | *pendente* |
| Resultado do critério | *pendente* |
