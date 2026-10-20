# ADR 0037 — Trilha científica e agente de pesquisa

- Estado: aceito pelo mantenedor em2026-10-20; incorporado ao roadmap
- Data: 2026-10-18
- Base: 451e1ff
- Relacionadas: 0035, 0036; Fases 5, 9, 10 e 15 do `ROADMAP.md`;
  [`docs/plans/visao-agente-cientifico.md`](../plans/visao-agente-cientifico.md)

## Contexto

O objetivo original do LOGOS inclui um agente de conversa em linguagem natural, com lógica
formal por baixo: o usuário pergunta como num chat, mas as afirmações relevantes carregam
status, premissas e certificado. O `ROADMAP.md` já prevê essa experiência na Fase 10
(cliente, prover farm, MCP, formalizador, loop, resposta renderizada e TUI), na Fase 9
(paráfrase determinística e autorização) e na Fase 15 (selo).

O fechamento da 5.2 torna possível a primeira interface externa real. Ao mesmo tempo, o
uso científico — IC e pesquisa universitária, com piloto potencial na UEA — pede um
artefato utilizável **antes** da Fase 10 inteira e antes de ações/gateway (Fase 8). Esperar
a 5.12 + Fases 6–10 adiaria qualquer validação com usuários por muitas etapas.

A proposta é antecipar uma **trilha vertical de pesquisa**: conversa em linguagem natural,
claims formalizadas, certificados/contramodelos, caderno de hipóteses e auditoria, sem
ações e sem políticas. As garantias do kernel são preservadas; a camada do agente fica fora da
TCB e usa o protocolo e o ledger existentes.

## Decisão

1. Adotar **LOGOS Research** como primeira vertical de produto: um agente de pesquisa
   conversacional, com lógica formal interna e saída em linguagem natural, voltado a
   alunos/pesquisadores. Não é um produto empresarial nem substitui o kernel.
2. Acrescentar ao `ROADMAP.md` a **Fase 5R ∥ — Bancada científica**, executável a partir
   da 5.5 e em paralelo com 5.6–5.12 (não bloqueia a Fase 5). Ela não substitui nenhuma
   etapa existente.
3. A camada5R (MCP, formalizador, caderno, TUI) fica fora da TCB. As etapas
   compartilhadas9.1/5.7/5.9 mantêm suas TCBs e critérios integrais. Se algum formato serializado mudar, valem as regras normais: spec, vetores, bump e
   ADR.
4. A 5R reutiliza os componentes da Fase 10. A 10.1/10.2/10.4/10.6/10.7 passam a ser
   compartilhadas; a 5R entrega versões de pesquisa sem ações. A 10.3 (MCP completo) e a
   10.5 (loop com ações) continuam na Fase 10.
5. O renderizador determinístico9.1 é dependência já da5R.2 e da5R.4, porque a resposta autoritativa
   em português precisa vir da TCB de fidelidade. O selo completo (15.1) continua na Fase
   15; o MVP usa cartões de status, sem Seal ou atestado F3.
6. Evidência empírica entra como nota de proveniência não verificada ou
   assumption explícita5.7. Says não cria fato/atestação na TCB; ingestão de
   atestações reais depende dos mecanismos das Fases6/7.
7. A avaliação inicial é um piloto técnico com 20–30 tarefas de pesquisa, comparando com e
   sem o harness, com métricas de formalização, detecção de erros, tempo e auditabilidade.
   Estudo com participantes humanos exige aprovação conforme11.8 antes da coleta;
   a avaliação técnica scriptada pode ocorrer sem esse estudo.

## Integração normativa (2026-10-20)

O mantenedor autorizou incorporar a trilha ao roadmap. A definição normativa
das etapas está na [Fase5R do ROADMAP](../../ROADMAP.md#fase-5r),
evitando manter uma segunda lista divergente neste ADR. Nenhuma etapa5R foi
implementada ou teve critério encerrado com esta alteração documental.

Correções de dependências ao aceitar a proposta:

- 5.5 continua primeiro;5R.1 exige cliente10.1 e farm10.2 no perfil de pesquisa.
- 9.1 vem antes de5R.2, que já exige confirmação por paráfrase. A9.1 conserva
  TCB de fidelidade, testes e revisão humana; não é um renderizador do LLM.
- 5R.3 exige5.7 e5.9, inclusive o critério de retirada de premissas. Logo a
  trilha inteira não é independente de5.6–5.12; antecipar5.9 antes da5.7
  para demonstrar também o critério de retratação da5.7.
- Perfis de pesquisa10.6/10.7 são entregues dentro de5R.4/5R.5, sem depender
  de10.5/9.2 e sem ciclo entre etapas. Versões completas mantêm critérios e
  dependências originais; nenhum subconjunto as encerra automaticamente.
- Evidência externa pode ser nota não verificada ou assumption explícita5.7.
  A expressão Says não cria fato/atestação nem construtor lógico novo.
- Cartões não são Seal/F3; invalidar contexto do cartão não encerra15.7.
- Demo técnica usa roteiro scriptado. Coleta com participantes, inclusive
  alunos, depende de11.8; simulação não é avaliação com usuários.

## Consequências

**Positivas.**
- Primeiro artefato utilizável por pesquisadores muito antes da Fase 10 completa.
- Feedback cedo sobre formalização, latência e UX, quando ainda é barato mudar.
- Caso de uso concreto para a UEA e para projetos de IC, sem prometer descoberta
  automática.
- Código do agente permanece fora da TCB; dependências antecipadas mantêm
  suas TCBs e garantias, sem reduzir critérios.

**Negativas.**
- Aumenta o escopo paralelo; exige disciplina para não atrasar 5.6–5.12.
- A qualidade do formalizador vira gargalo de adoção; precisa de avaliação por domínio.
- A TUI e a resposta natural têm custo de engenharia e de design.
- O MVP não satisfaz os critérios da Fase 10 (ações, políticas, selo completo).

**Neutras.**
- A 5R é uma vertical de pesquisa; o kernel completo e as ações continuam necessários
  para o regime operacional.
- O selo completo continua na Fase 15; cartões de status não são o selo final.

## Alternativas rejeitadas

1. **Manter a Fase 10 como está.** Chega tarde para o piloto e não prioriza a experiência
   científica; a 5R antecipa só o necessário.
2. **Construir um produto separado.** Duplicaria protocolo/ledger e divergiria das
   garantias; a vertical deve usar o mesmo kernel.
3. **Forkar o OpenCode como único front-end.** É um bom primeiro passo via MCP, mas não
   cobre o caderno de claims, a auditoria e o selo; a integração não substitui a UX própria.
4. **Fazer a 5R depender da Fase 10 inteira.** Contradiz o objetivo de validar cedo e
   adia o piloto UEA.

## Limites

- Não há verdade empírica automática nem descoberta científica garantida; L1 continua.
- Não há ações, gateway, políticas ou multiusuário na 5R.
- Não há selo completo nem atestados F0–F4 na 5R; há cartões de status.
- O formalizador é não confiável e exige confirmação humana.
- O piloto técnico não substitui estudo com pessoas nem aprovação de ética.

## Aplicação

Aplicada documentalmente em2026-10-20 sobre cd41b17, após o fechamento local
da5.4. Próximo principal:5.5. Depois, seguir a trilha5R com suas dependências
por etapa: cliente/farm de pesquisa →5R.1;9.1 →5R.2;5.7/5.9 →5R.3;
5R.4 →5R.5 →5R.6. Intercalar as demais5.6–5.12 conforme necessário.
O roteiro não obriga frentes simultâneas; preserva todas as etapas originais.
