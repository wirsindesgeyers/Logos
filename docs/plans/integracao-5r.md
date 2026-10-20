# Incorporação documental da trilha5R

- Data:2026-10-20; base:cd41b17.
- Estado: integração documental concluída e verificada localmente; runtime5R não iniciado.
- Escopo autorizado: aceitar ADR0037 e incorporar5R ao roadmap. Implementação
  de5.5/5R fica para as etapas próprias; nenhum critério novo está demonstrado.

## 1. Entradas

Proposta ADR0037/visão científica, contratos5.5/5.7/5.9/9.1/10.x e STATUS
com5.4 concluída localmente. Dependências futuras ficam explicitadas, sem
fingir que cliente, farm, renderizador ou handlers já existem.

## 2. Saídas

Fase5R.1–5R.6 normativa no ROADMAP, ADR0037 aceito, visão alinhada e tabela
de etapas/Próximo passo no STATUS. A próxima implementação permanece5.5.

## 3. Tarefas

1. Incorporar etapas, sumário e visão geral; preservar todos os critérios
   anteriores e deixar todos os critérios5R abertos.
2. Resolver dependências: cliente/farm antes do MCP,9.1 antes da confirmação,
   5.9/5.7 antes do caderno. Perfis de pesquisa10.6/10.7 nascem na5R.4/5R.5;
   versões completas mantêm dependências operacionais, evitando ciclos.
3. Distinguir notas de evidência de fatos; cartões de status de Seal/F3;
   demo scriptada de avaliação com participantes sujeita a11.8.
4. Conferir referências, critérios e coerência entre os quatro documentos;
   rodar ci/verify.sh --selftest conforme AGENTS.md antes de commit.

## 4. Critérios documentais → conferência

- Seis etapas5R, com dependências/fora do escopo/entregáveis/critérios e
  nenhum checkbox concluído; critérios originais do roadmap preservados.
- 5.5 continua próximo passo; cronograma aponta9.1/5.9/5.7 obrigatórias.
- ADR/visão deixam de dizer proposta; ROADMAP é a única lista normativa.
- Nenhum fonte, formato serializado, vetor ou dependência alterado.

## 5. Decisões e riscos

Aceite do mantenedor nesta conversa. A proposta original escondia a dependência
9.1 já na5R.2,5.7/5.9 no caderno e ciclos com10.6/10.7 completas. Correções
registradas no ADR0037 aceito. A camada5R fica fora da TCB; etapas antecipadas
da TCB mantêm seus contratos. Não prometer5R inteira imediatamente após5.5.

## 6. Tamanho

P documental com decisões registradas; não é execução das etapas M/G da5R.

## Verificação

Conferência documental:277 critérios anteriores preservados,13 novos abertos,
seis etapas e Diário apenas acrescido. Uma primeira conferência auxiliar usou
um padrão de cabeçalho incorreto; corrigido para `### Etapa 5R.`, passou.

`ci/verify.sh --selftest`: VERIFY_OK/exit0 em1496.5s;252 testes Python e25
testes TCB passaram. Os455 arquivos monitorados permaneceram idênticos.
Recibo e hashes em `integracao-5r-evidencias.json`; log local em
`target/verify-5r-docs.log`. CI remota, ARM e noturno continuam pendentes;
esta alteração não encerra implementação alguma.
