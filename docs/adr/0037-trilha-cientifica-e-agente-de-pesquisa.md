# ADR 0037 — Trilha científica e agente de pesquisa

- Estado: proposto (aguarda revisão do mantenedor)
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
ações e sem políticas. O kernel e suas garantias não mudam; a trilha é uma camada não
confiável sobre o protocolo e o ledger existentes.

## Decisão

1. Adotar **LOGOS Research** como primeira vertical de produto: um agente de pesquisa
   conversacional, com lógica formal interna e saída em linguagem natural, voltado a
   alunos/pesquisadores. Não é um produto empresarial nem substitui o kernel.
2. Acrescentar ao `ROADMAP.md` a **Fase 5R ∥ — Bancada científica**, executável a partir
   da 5.5 e em paralelo com 5.6–5.12 (não bloqueia a Fase 5). Ela não substitui nenhuma
   etapa existente.
3. A 5R não altera a TCB. Todo código novo (MCP, formalizador, caderno, TUI) fica fora da
   TCB. Se algum formato serializado mudar, valem as regras normais: spec, vetores, bump e
   ADR.
4. A 5R reutiliza os componentes da Fase 10. A 10.1/10.2/10.4/10.6/10.7 passam a ser
   compartilhadas; a 5R entrega versões de pesquisa sem ações. A 10.3 (MCP completo) e a
   10.5 (loop com ações) continuam na Fase 10.
5. O renderizador determinístico 9.1 é dependência da 5R.4, porque a resposta autoritativa
   em português precisa vir da TCB de fidelidade. O selo completo (15.1) continua na Fase
   15; o MVP usa cartões de status e uma versão simples de selo.
6. Evidência empírica entra como `Says(fonte, φ)`/atestação explícita ou como assumption do
   usuário, nunca como fato automático. A 5R não cria um tipo novo de fato na TCB.
7. A avaliação inicial é um piloto técnico com 20–30 tarefas de pesquisa, comparando com e
   sem o harness, com métricas de formalização, detecção de erros, tempo e auditabilidade.
   Estudo com participantes humanos exige aprovação de ética e fica para a Fase 11/16.

## Diff proposto no `ROADMAP.md`

Inserir, depois da Fase 5 (após a 5.12), a seção abaixo. A ordem no documento é posterior
à Fase 5, mas a dependência real é a 5.5; a fase pode começar assim que 5.3–5.5 estiverem
concluídas.

```markdown
## Fase 5R ∥ — Bancada científica (trilha vertical)

Objetivo da fase: um agente de pesquisa conversacional, com lógica formal por baixo e
saída em linguagem natural, utilizável por alunos e pesquisadores antes da Fase 10
completa. A fase não substitui o kernel nem as ações: é uma camada não confiável sobre o
protocolo (5.2), as claims (5.3–5.5) e o ledger (3.x). Pode rodar em paralelo com 5.6–5.12.

### Etapa 5R.1 — Ferramentas MCP de pesquisa · `M`

**Depende de (recebe pronto).** 5.3, 5.4, 5.5; 10.1 (cliente, antecipado aqui se preciso).

**Fora do escopo.** ações, políticas, multiusuário, MCP completo da 10.3.

**Entregáveis.**
- Servidor MCP de pesquisa com ferramentas: propor claim, checar, buscar contramodelo,
  consultar ledger, auditar e exportar relatório.
- Frames canônicos do protocolo; nenhuma ferramenta escreve fato, obrigação ou política.
- Testes de integração contra o kernel real; auditoria de uma sessão completa.

**Critério de conclusão.**
- [ ] 10 tarefas de pesquisa executadas só via MCP, com claims e certificados reais.
- [ ] Nenhuma ferramenta expõe caminho para `Verified` sem checker ativo ou para escrita
  de fato/política.

### Etapa 5R.2 — Formalizador NL → AIR com confirmação · `M`

**Depende de (recebe pronto).** 5R.1; 1.2, 1.4.

**Fora do escopo.** aprovação automática de símbolos novos (sempre humana).

**Entregáveis.**
- Texto em português → proposta AIR no vocabulário vigente; erros de sort-check e
  contramodelos voltam ao formalizador.
- Paráfrase determinística (9.1) para confirmação humana; símbolos novos viram pedido de
  extensão, com status explícito.
- O formalizador é não confiável: nunca registra a claim sem confirmação.

**Critério de conclusão.**
- [ ] Em 100 frases de um domínio piloto, ≥ 80% viram propostas bem tipadas sem
  intervenção (métrica de progresso).
- [ ] Nenhum símbolo novo é aprovado sem confirmação humana registrada.

### Etapa 5R.3 — Caderno de claims, hipóteses e evidências · `M`

**Depende de (recebe pronto).** 5R.1; 3.3; 5.7 (assumptions); 5.9 (retratação, quando
disponível).

**Fora do escopo.** `Fact`/atestadores reais (Fase 6); estatística e desenho experimental.

**Entregáveis.**
- Grafo por projeto: hipóteses, previsões, evidências, assumptions e conclusões, com status
  e dependências.
- Evidência como `Says`/atestação explícita ou assumption do usuário; nunca fato
  automático.
- Exportação de relatório e auditoria; retratação atualiza a citabilidade.

**Critério de conclusão.**
- [ ] 20 hipóteses de um domínio piloto com proveniência completa e nenhuma evidência
  apresentada como teorema.
- [ ] Retirar uma premissa invalida exatamente os dependentes e a auditoria mostra o corte.

### Etapa 5R.4 — Resposta natural com cartões de status · `M`

**Depende de (recebe pronto).** 5R.2, 5R.3; 9.1; 10.6 (versão simples).

**Fora do escopo.** selo completo (15.1/15.8).

**Entregáveis.**
- Resposta em blocos `ledger_ref(h)` (renderizados do ledger, com status simples) e
  `free_text` (LLM, marcado como não verificado).
- Cartões de claim com status, premissas, contramodelo e lacunas; expansão para auditoria.
- Nenhuma afirmação não verificada sem marcação.

**Critério de conclusão.**
- [ ] 50 respostas de teste sem conteúdo não verificado sem marcação.
- [ ] Trocar a paráfrase ou o vocabulário invalida o cartão, como na 15.7.

### Etapa 5R.5 — Chat/TUI de pesquisa · `M`

**Depende de (recebe pronto).** 5R.4; 10.7 (padrões); 3.3.

**Fora do escopo.** assinatura dentro da TUI; ações.

**Entregáveis.**
- Chat com histórico, DAG de claims, visualizador de contramodelo, fila de confirmações e
  auditoria ao vivo.
- Primeira integração via MCP com OpenCode e afins; TUI própria mínima (Ratatui ou web)
  depois.
- Nenhuma reformatação de texto que o usuário venha a assinar.

**Critério de conclusão.**
- [ ] Demo gravada: pergunta científica → claims → veredicto → auditoria, sem editar AIR.
- [ ] Um aluno completa a tarefa sem conhecer a sintaxe formal.

### Etapa 5R.6 — Piloto UEA e avaliação · `G`

**Depende de (recebe pronto).** 5R.1–5R.5; 11.8 apenas se houver participantes humanos.

**Fora do escopo.** paper (Fase 11); estudo com usuários em escala.

**Entregáveis.**
- 20–30 tarefas de pesquisa de um domínio formalizável, com baseline (agente sem harness).
- Métricas: taxa de formalização, erros detectados, tempo até veredicto, tempo de
  auditoria, reproducibilidade e percepção de utilidade.
- Relatório e artefato reproduzível; aprovação de ética se houver humanos.

**Critério de conclusão.**
- [ ] Resultados com e sem harness, análise estatística e nenhuma conclusão sem marcação.
- [ ] Um terceiro reproduz o piloto a partir do README.
```

Além disso, ajustar a Fase 10:

- Objetivo: "harness completo, incluindo ações", reutilizando a 5R.
- Nota em 10.1, 10.2, 10.4, 10.6 e 10.7: "compartilhada com a 5R; a 5R antecipa a versão
  de pesquisa sem ações".
- 10.3 continua sendo o MCP completo (com `ActionIntent`); 10.5 continua sendo o loop com
  ações; 10.8 permanece.

## Consequências

**Positivas.**
- Primeiro artefato utilizável por pesquisadores muito antes da Fase 10 completa.
- Feedback cedo sobre formalização, latência e UX, quando ainda é barato mudar.
- Caso de uso concreto para a UEA e para projetos de IC, sem prometer descoberta
  automática.
- Nenhuma mudança na TCB nem nas garantias de segurança.

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

O diff do `ROADMAP.md` e a mudança de estado deste ADR para "aceito" devem ser aplicados
**depois** do commit de fechamento da 5.2, para manter um commit coerente por etapa. A
ordem sugerida é: 5.3–5.5, 5R.1–5R.5, e 5R.6 quando houver artefato estável. A 5R é
independente de 5.6–5.12 e pode rodar em paralelo; com uma equipe pequena, porém, a ordem
prática é terminar 5.5, fazer a 5R e então retomar 5.6–5.12 (ou intercalar por
disponibilidade), em vez de tratar "paralelo" como duas frentes simultâneas obrigatórias.
