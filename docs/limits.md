# Limites do LOGOS

Etapa 0.7 · 2026-10-04. Referência: [spec §1, §6, §10 e §13](spec/LOGOS-proof-carrying-agents.md).
Leia junto com [garantias condicionais](guarantees.md) e
[modelo de ameaças](threat-model.md).
Limite fundamental não é promessa de correção numa fase futura.

## L1 — A prova não liga símbolos ao mundo

Um certificado aceito garante derivabilidade formal `Γ ⊢ φ` sob a semântica
identificada e a correção da TCB. Não garante que `φ` traduz a intenção da frase,
que o predicado está ancorado corretamente ou que os fatos de `Γ` são verdadeiros.
Assinatura identifica quem afirmou bytes; não converte mentira em verdade.

Fidelidade tem TCB própria: formalização/grounding, atestadores, autores de
políticas/contratos, renderizador e quem confirma. Dupla formalização equivalente
pode repetir o mesmo erro. Usuário pode assinar uma paráfrase que não compreende;
F3 registra confirmação, não prova objetivamente o significado pretendido.
Exemplo: prova de `Allowed(drop(db))` sobre fato falso de backup continua podendo
ser uma derivação válida, mas não torna seguro apagar o banco.

## L2 — Política pequena decidível dispensa prova externa

Se avaliador confiável decide a política diretamente com custo adequado, exigir
certificado de um prover não acrescenta segurança. Usar avaliação direta na rota
L2 (7.3/7.4) preserva mediação, origem e registro, sem protocolo de prova redundante.
Isso é orientação de arquitetura; avaliador ainda não está implementado.

Certificados se justificam quando checar custa muito menos que decidir, quando o
checker é menor que o decisor ou quando a justificativa precisa viajar e ser
rechecada. Não há promessa de rapidez geral: grounding finito pode explodir,
prova pode faltar e consistência de lógica aberta pode ser indecidível.
Ausência de certificado/testemunha é `Unknown` ou bloqueio, nunca evidência de
verdade/falsidade. Override humano futuro é exceção assinada e registrada, não
certificado lógico e não conversão de `Unknown` em `Verified`.

## L3 — Mediação não governa cognição nem texto livre

No desenho completo, monitor controla efeitos, transições do ledger e claims
entregues com selo, não o contexto do LLM. Claim pendente/rejeitado/refutado ainda
pode influenciar pensamento, plano e prosa. Integridade de justificativa não é
não-interferência nem prova de quais tokens de contexto causaram a escolha.

Resposta final deve renderizar claims do ledger com condições/selo e marcar
texto livre como não verificado (10.6/15.8, futuros). Não há garantia de que toda
frase gerada será verdadeira nem que todo usuário distinguirá os campos. Hoje
não há kernel/gateway ou renderização final mediada implementados.

## Ameaças fundamentais e parte fundamental da 10

Numeração da §10; destinos de mitigação no
[Apêndice B](../ROADMAP.md#apêndice-b--matriz-de-ameaças--etapas).
“Mitigar” aqui reduz exposição ou explicita premissas; não elimina o limite.

| # | Limite | O que a engenharia pode fazer / o que não promete |
| --- | --- | --- |
| 1 | Formalização infiel | Paráfrase, F2/F3 e auditoria tornam erro mais visível; nenhuma prova liga automaticamente linguagem natural à intenção |
| 2 | Atestador comprometido ou errado | Confiança por tópico, pluralidade e revogação reduzem exposição; assinaturas não provam fatos do mundo |
| 3 | Política errada | Revisão/análise compara consequências; provar conformidade não prova que a política é adequada ou completa |
| 10 | TOCTOU sem versão atômica e fatos físicos | Declarar `unprotected_pre`, janela curta e possível confirmação humana; não assegurar validade do fato no instante do efeito |
| 13 | Injeção escolhe entre ações permitidas | Origem e autorização restringem espaço de ações; dentro dele a escolha ainda pode contrariar intenção do usuário |
| 16 | Confiança excessiva no selo | Campos completos e estudo de compreensão reduzem erro; “verificado” não significa verdade, fidelidade alta ou intenção satisfeita |

As ameaças 1/2/3/13/16 são fundamentais. A 10 tem uma parte tratável por
concorrência e outra fundamental; não se deve marcar toda a classe como resolvida.

## TOCTOU — proteção depende do recurso

| Classe (§6) | Condição para proteção | Risco residual |
| --- | --- | --- |
| Recurso transacional: linha/saldo | Verificar versão e executar dentro da mesma transação | Sob atomicidade/isolamento corretos, elimina corrida sobre as pré-condições cobertas; não sobre fatos externos |
| Recurso versionado: ETag/revisão | Serviço compara versão correta e realiza efeito condicional (`If-Match`/CAS) | Quase eliminado na spec; depende de cobertura da versão e do contrato real do serviço |
| Fato atestado sem versão atômica: nenhum usuário conectado | Token de vida curta e `unprotected_pre` explícito | Fato pode mudar entre observação e efeito, mesmo dentro do TTL |
| Fato físico: sala vazia | Política pode exigir confirmação humana no instante | Nem confirmação nem criptografia garantem permanência/verdade do estado físico |

Consultar versão e depois executar em requisição separada não é execução
condicional atômica. Tampouco revalidar assinatura é revalidar o mundo.
G2 limita validade instantânea às pré-condições protegidas; registrar as demais
não as torna protegidas. Contrato errado da ferramenta permanece risco de L1.

## Limites de implementação e de evidência

- CAS garante integridade dos bytes lidos sob premissas de filesystem local e
  ancestrais confiáveis; não protege contra operador hostil, forks de ledger,
  falta de espaço ou indisponibilidade. Limite por objeto não é teto de RSS.
- Core só contabiliza passos cobrados. `mem_bytes`/`size_bytes` são campos para
  consumidores futuros; não há supervisão global de alocação nem timeout interno.
  [ADR 0006](adr/0006-veredictos-e-orcamentos.md): closure pode omitir cobrança;
  enums públicos podem ser construídos sem prova e não conferem autoridade.
- `Rejected` é rejeição de certificado; `Refuted` exige contramodelo aceito.
  `Unknown` por passos/tamanho/memória/tempo não é nenhum dos dois. Decoder
  inválido devolve erro, não um valor padrão “parecido”.
- Hash chain assinada comprova encadeamento da visão apresentada, não ausência
  de outra visão. Operador/kernel bifurcando histórias é A5/ameaça 9; transparência
  futura depende de testemunhas externas e comparação de cabeças.
- Não há promessa geral de confidencialidade contra operador curioso, progresso,
  utilidade de claims, política completa, reversibilidade física nem segurança
  contra compilador/SO/hardware comprometidos.
- Testes locais, fuzz curto e código sem `unsafe` próprio não são prova formal,
  evidência de execução ARM ou job noturno. Pendências remotas permanecem no
  [STATUS](../STATUS.md); documentação não as fecha.

## Sintaxe AIR implementada (Etapa 1.1)

[air/v1](../spec/air.md) valida árvore prefixa e limites lógicos agregados.
Sintaxe mal sortada, variáveis livres e assinaturas incongruentes permanecem
representáveis até o sort-check da 1.3. Igualdade de bytes/hash não é equivalência
lógica nem prova de verdade. Nomes de binders não participam da identidade.

Prefixo plano evita recursão proporcional à profundidade lógica na construção e
no descarte. O decoder canônico ainda materializa Value antes de converter a AST;
limites de nós/profundidade não limitam bytes de strings/assinaturas nem RSS.
Consumidor continua responsável por teto de frame/CAS e orçamento de execução.
