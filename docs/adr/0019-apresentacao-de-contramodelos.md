# ADR 0019 — Apresentação portuguesa de contramodelos

- Estado: aceito
- Data: 2026-10-05
- Escopo: ROADMAP 2.6, fora da TCB

## Contexto

Modelos finitos têm IDs posicionais ligados a h(V), tabelas densas e símbolos
Defined sem interpretação serializada. Apresentação deve ser legível sem
introduzir símbolos das dependências de definições ou afirmar fidelidade.

## Decisão

API `logos_provers.model_explain.explain_model(M_bytes, V_bytes, O_bytes) -> str`.
Consumir formatos existentes e validação AIR/contexto; validar model/v1
contextualmente antes de interpretar. Reutilizar intérprete/derivação do finder
não confiável, sem duplicar semântica e sem importar Z3. Colaboração interna
entre módulos da mesma camada, sem novo contrato na TCB.

Relevância é ocorrência sintática nas premissas ou objetivo de O, incluindo
termos e sorts das assinaturas/quantificadores. Não expandir Defined nem listar
sorts/símbolos ocultos. Derivar Defined globalmente para avaliar, mas mostrar
apenas suas interpretações relevantes. Elementos `Sort#ordinal` não usam nomes
de constantes enumeradas ausentes de O. Domínios em intervalo compacto,
constantes como valores, funções como tupla → valor, predicados como tuplas
verdadeiras; predicado sem tuplas explicitamente sem fatos verdadeiros.

Glosas não vazias são dados entre aspas escapadas junto do identificador; glosa
vazia usa só identificador. Não interpretar glosa como fórmula/template ou
identidade de símbolo. Restrição de símbolos aplica-se às referências formais,
não às palavras arbitrárias de uma glosa. Não há promessa de fidelidade.

Premissas e objetivo são identificados pelo impressor AIR já existente, com
valoração calculada no modelo. Objetivo verdadeiro não vira falsificado;
premissa falsa é indicada, não fabricada como justificativa de refutação.
Não implementar paráfrase determinística portuguesa de fórmulas da etapa 9.1.
Texto sempre informa caráter não autoritativo e inclui
SHA-256(`logos/model/v1\0` || M_bytes). Não chama checker nem emite Verdict.

Caps de bytes/domínios de modelo (1 MiB, domínio até 65536), e trabalho
cooperativo do finder (1M passos/5s) na avaliação. Tabelas densas agregadas,
inclusive predicados/Defined, limitadas a 100k células: certificado esparso
válido pode não caber nesta apresentação; isso não muda resultado do checker.
Limites insuficientes ou estrutura inválida produzem ProducerError.
Até 100 linhas de fatos/valores na
apresentação, com omissão explícita; não truncar fórmulas, hash ou valorações.
Nenhum limite rígido de RSS/wall-clock. Saída depende somente dos inputs quando
avaliação termina dentro do orçamento.

## Alternativas

- Expandir Defined no texto: vazaria dependências ausentes de O.
- Nomear elementos por constantes Enum: vazaria símbolos não usados.
- Chamar checker automaticamente: acoplaria apresentação à autoridade e ao
  hash esperado de O, que deve vir do monitor, não desta API.
- Nova semântica/evaluador português: duplicaria regras e entraria na 9.1.

## Consequências

Sem formato novo, bump, dependência ou alteração da TCB. Corpus de 20
refutações é conferido pelo checker Rust real na demonstração, separadamente
da explicação. CI remota continua pendência distinta da execução local.
