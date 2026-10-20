# Visão — LOGOS como agente de pesquisa

- Estado: visão adotada pelo ADR0037 em2026-10-20; execução e critérios no ROADMAP
- Data: 2026-10-18
- Base: 451e1ff
- Relacionadas: ADR0035, ADR0036, Fases 9, 10 e 15 do `ROADMAP.md`

## 1. Tese

O LOGOS deve ser um **agente de conversa em linguagem natural** com lógica formal por
baixo. O usuário pergunta, discute e pede ajuda como faria com qualquer agente; o que muda
é que toda afirmação relevante carrega status formal, premissas e certificado, e nenhum
efeito acontece sem mediação do kernel.

O diferencial não é "ter provas" escondidas num backend. É a **experiência**: conversa
simples na superfície, rigor visível quando importa. O aluno não precisa escrever AIR para
usar o sistema; ele precisa saber ler um cartão de claim e pedir auditoria.

## 2. Usuário e cenário inicial

Usuário inicial: aluno de IC e pesquisador universitário (piloto UEA). Cenário:
transformar uma pergunta científica em um conjunto de hipóteses, previsões e evidências
explicitamente ligadas, com o que foi formalmente verificado separado do que é conjectura
ou evidência empírica.

Exemplo de pergunta:

> "A política P de acesso a dados sensíveis impede o fluxo F nas condições C?"

O agente não responde só em texto. Ele propõe uma formalização, mostra as premissas que
está usando, pede certificado ou contramodelo, e devolve uma resposta em português com o
status de cada parte.

## 3. Princípios de experiência

1. **Conversa primeiro.** A interface padrão é chat (TUI ou web), com histórico, planos,
   ferramentas e streaming. Nada de obrigar o usuário a editar fórmulas.
2. **Formalização progressiva.** Só as afirmações que importam viram obrigação. Conversa
   trivial continua barata; claims sensíveis são formalizadas e checadas.
3. **Nunca fingir verificação.** Todo bloco tem um status honesto: `Verified`, `Refuted`,
   `Unknown`, `Consistente`, `Suportado por evidência` ou `Não formalizado`. O selo existe
   para mostrar as dimensões, não para esconder lacunas.
4. **Texto autoritativo vem do ledger.** A resposta pode ter texto livre do LLM, mas ele é
   marcado como não verificado; os blocos verificados são renderizados a partir do ledger,
   com hash e paráfrase determinística.
5. **Auditoria a um clique.** Qualquer claim expande para certificado, contramodelo,
   premissas, assumptions e lacunas. O relatório final é rechecável por terceiros.
6. **Camada do agente descartável.** Chat, planejamento e redação ficam fora da TCB. A
   correção não depende do LLM; depende do kernel pequeno.
7. **Simples por padrão, formal quando necessário.** O usuário pode conversar sem saber o
   que é AIR; o sistema pede confirmação humana quando a formalização muda o sentido.
8. **Local-first quando possível.** O núcleo roda offline; LLM e solvers são plugáveis.

## 4. Modos de uso

- **Explorar.** Conversa livre, hipóteses informais, busca de literatura. Tudo que não foi
  formalizado aparece marcado como não verificado.
- **Verificar.** O agente formaliza a claim, mostra a proposta em português controlado,
  pede confirmação, roda checker/prover e devolve `Verified`, `Refuted` com contramodelo ou
  `Unknown` com motivo. O usuário pode enfraquecer, declarar assumption ou descartar.
- **Auditar.** Reconstrói a cadeia, recheca certificados, mostra dependências e exporta um
  relatório com as lacunas. Não confia no agente nem no histórico da conversa.

## 5. Fluxo ponta a ponta (exemplo)

1. O usuário pergunta em português.
2. O agente separa a pergunta em claims e hipóteses; o que vier da literatura entra como
   `Says(fonte, φ)` ou como assumption explícita, nunca como fato automático.
3. O formalizador propõe AIR; o usuário confirma a paráfrase e os símbolos novos.
4. O kernel gera a obrigação, o prover devolve certificado/modelo e o checker decide.
5. Se `Refuted`, o agente mostra o contramodelo e propõe uma revisão; se `Unknown`, mostra
   o limite e o que faltaria.
6. A resposta final combina blocos do ledger (com selo/status) e texto livre marcado.
7. O usuário pede `auditar`; o relatório mostra o que sustenta cada conclusão e o que é
   conjectura.

Esboço da interação:

```text
> A política P impede o fluxo F nas condições C?

[agente] Vou separar a pergunta em três claims verificáveis.

  C1  P ∧ C → ¬F                      [Verified]
      premissas: P, C · certificado · auditar
  C2  ¬F → sem vazamento de dados     [Refuted]
      contramodelo: M3 · ver modelo
  C3  vale para todo domínio          [Unknown: limite]
      falta: prova no caso ilimitado

Resposta: para o domínio finito coberto por P, F é bloqueado; a generalização
irrestrita falha em M3. A parte não formalizada está marcada abaixo.

[Auditar] [Exportar relatório] [Enfraquecer C3]
```

## 6. O que o harness verifica — e o que não verifica

| Dimensão | O que significa | Quem sustenta |
| --- | --- | --- |
| Consequência formal | `Γ ⊢ φ` segundo um checker | Kernel + certificado |
| Consistência | existe modelo para as premissas | Kernel + testemunha |
| Refutação | existe contramodelo | Kernel + checker |
| Evidência empírica | dados/observações apontam para φ | Atestador/usuário, explícito |
| Verdade no mundo | φ vale de fato | Fora do sistema (L1) |
| Fidelidade | a paráfrase corresponde à fórmula | Renderizador + atestado F0–F4 |

O LOGOS não transforma evidência em teorema nem conjectura em fato. Ele torna explícito
**o que** foi assumido, **quem** atestou e **qual** foi o caminho de checagem.

## 7. MVP proposto

1. Ferramentas de pesquisa via MCP: propor claim, checar, buscar contramodelo, consultar
   ledger, auditar e exportar.
2. Formalizador NL → AIR com confirmação humana e tratamento de símbolos novos.
3. Caderno de claims/hipóteses/evidências por projeto, com status e dependências.
4. Resposta em linguagem natural com cartões de status (selo completo fica para a Fase 15).
5. CLI/TUI mínima; integração com OpenCode e afins via MCP antes de uma TUI própria.
6. Piloto com 20–30 tarefas de pesquisa e comparação com/sem o harness.

Fora do MVP: ações e gateway (Fase 8), políticas (Fase 7), multiusuário, selo completo
(Fase 15), estudo com pessoas (Fase 11/16) e descoberta científica automática.

## 8. Métricas de sucesso

- Taxa de afirmações relevantes que viram claims formalizadas e checadas.
- Taxa de erros/contradições detectadas antes de virarem conclusão.
- Taxa de `Unknown` e tempo até um veredicto útil.
- Tempo para auditar uma conclusão e reprodutibilidade por terceiros.
- Facilidade percebida (questionário) e tempo para a primeira resposta auditável.
- Nenhuma resposta mostra conteúdo não verificado sem marcação.

## 9. Riscos

- **Gargalo de formalização.** Se o formalizador errar muito, a experiência morre. O
  caminho é confirmação humana + vocabulário assistido, não AIR manual.
- **Latência.** Provar/refutar pode demorar; a UX precisa de estados parciais e `Unknown`
  útil, não de spinner infinito.
- **Expectativa.** "Hipótese revolucionária" não é garantia do harness; o valor é rigor e
  auditabilidade.
- **Escopo.** Construir um IDE novo é inviável; a aposta é MCP + TUI enxuta e reutilização
  do ecossistema de agentes.
- **Fidelidade.** A paráfrase pode enganar; por isso o selo e a confirmação humana são
  parte do produto, não um detalhe.

## 10. Relação com o roadmap

A proposta é antecipar uma **trilha vertical de pesquisa** depois da 5.5, sem esperar a
Fase 10 inteira: cliente/MCP, formalizador, caderno de claims, resposta natural e TUI
mínima. As etapas 5.6–5.12 e as Fases 6–9 continuam necessárias para o kernel completo e
para ações; a trilha de pesquisa não as substitui. O ADR0037 registra a decisão aceita e a Fase5R está incorporada ao
`ROADMAP.md`. A ordem completa inclui9.1 antes de5R.2 e5.7/5.9 antes
de5R.3; os perfis de pesquisa10.6/10.7 não dependem do loop com ações10.5.
As etapas5R ainda não foram implementadas. Notas de evidência/Says não criam
fatos na TCB; avaliação com participantes depende de11.8.
