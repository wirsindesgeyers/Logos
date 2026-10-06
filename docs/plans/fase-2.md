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

2.1–2.5 implementadas localmente; detalhes dos checkers em
[etapa-2.3.md](etapa-2.3.md)/ADR 0016 e produtor Z3 em
[etapa-2.4.md](etapa-2.4.md)/ADR 0017: 200/200 contramodelos aceitos, adversário
10k sem aceitação indevida e negação invertida rejeitada. Finder independente
[etapa-2.5.md](etapa-2.5.md)/ADR 0018: 296 obrigações contra enumeração completa
de 682 modelos ≤3, 201/201 contramodelos aceitos, 95 ausências limitadas;
fallback após unknown nativo. TCB inalterada na 2.4/2.5.
S3 do avaliador em duas arquiteturas da CI ainda precisa ser observado.
2.6 demonstrada localmente: [plano](etapa-2.6.md)/ADR 0019,
explain_model com glosas/h(M)/ordinais e símbolos sintáticos de O;
20/20 refutações aceitas pelo checker real, 10 regressões, smoke público
e runtime-only passaram. Full verify/selftest VERIFY_OK, 227 Python + 25 TCB.
Nenhuma garantia de fidelidade ou autoridade; TCB/formatos inalterados.
Próximo: 3.1, catálogo de eventos. CI remota 2.6 não observada.
Memória de longo prazo continua proposta separada, sem alteração
de escopo ou dependências desta fase.
