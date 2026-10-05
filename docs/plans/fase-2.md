# Plano de fase — Fase 2: modelos finitos

Fonte: ROADMAP Fase 2; dependências da Fase 1 demonstradas localmente.

1. **2.1**: `logos-model` valida estruturas finitas e fixa `model/v1` com h(V),
   limites e vetores. Não avalia fórmulas nem emite veredictos.
2. **2.2**: avaliador deliberadamente simples sobre FiniteModel e Formula,
   expansão de Defined e Budget. Diferencial Z3 e propriedades S2/S3.
3. **2.3**: checkers de refutação/consistência recebem bytes e hash da obrigação;
   usam avaliador e CAS. Não registram ledger nem fazem roteamento.
4. **2.4**: produtor Z3 não confiável consome formatos O/model já fixados;
   contramodelos precisam passar pelo checker real da 2.3.
5. **2.5**: finder alternativo, após 2.1/2.3; pode ocorrer em paralelo à 2.4.
6. **2.6**: apresentação do contramodelo, sem garantia de fidelidade; consome
   modelo e glosas, não integra checker.

Interfaces: posições de sorts/símbolos referem-se ao vocabulário exato hasheado;
funções densas usam último eixo mais rápido; predicados são conjuntos de tuplas.
Unknown(Size/Memory) permanece exaustão, nunca Refuted ou aceitação parcial.

2.1 implementada nesta sessão; 2.2–2.6 não iniciadas. Memória de longo prazo
continua proposta separada, sem alteração de escopo ou dependências desta fase.
