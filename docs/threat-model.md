# Modelo de ameaças

Etapa 0.7 · 2026-10-04. Fontes: [spec §1, §5, §6, §10 e §13](spec/LOGOS-proof-carrying-agents.md),
[ROADMAP, Apêndice B](../ROADMAP.md#apêndice-b--matriz-de-ameaças--etapas),
[manifesto da TCB](../tcb/manifest.toml).
Este documento mapeia riscos; não implementa mitigação.
As [garantias](guarantees.md) são condicionais e projetadas, não funcionalidades
já disponíveis. Os [limites](limits.md) não desaparecem com o kernel.

## Ativos e fronteiras

| Ativo | Propriedade em risco | Fronteira de confiança |
| --- | --- | --- |
| Chave do kernel | Autenticidade de eventos/tokens; comprometimento permite forjá-los | Signer/processo do kernel; FileSigner atual depende de SO e permissões, não é HSM |
| Credenciais do gateway/adaptadores | Mediação de todo efeito; vazamento permite execução fora do monitor | Só adaptador detém credencial; agente/Python/prover não têm acesso no desenho futuro |
| Ledger, cabeças, épocas e DAG de dependências | Integridade, proveniência, revogação e visão consistente entre leitores | Appender assinado e auditor futuro; hash chain isolada não impede bifurcação |
| Objetos CAS e certificados | Integridade dos bytes, separação de domínios e disponibilidade | CAS atual verifica digest ao ler; filesystem e ancestrais confiáveis; não constitui ledger |
| Obrigações, chamadas e contratos | Checker prova exatamente obrigação e ação realmente executadas | Gerador/encoder/grounder e ligação aos bytes; agente só propõe e cita |
| Políticas, fatos, vocabulário e autorização humana | Fidelidade, origem aceitável e escopo de autorização | Autoridades, atestadores, paráfrase determinística, interface exibida e usuário |
| Orçamentos e capacidade de concluir | Resistência a excesso de trabalho/tamanho; progresso | Medição cooperativa de passos atual; consumidores/supervisores futuros |
| Dados privados do usuário e das ferramentas | Confidencialidade | Operador curioso e outros agentes podem observar o que lhes for disponibilizado; nenhuma garantia geral de sigilo nesta etapa |

A classificação é **por propriedade**: LLM/prover não entram na TCB de correção
quando o certificado é rechecado contra a obrigação correta; autores de políticas
e atestadores entram na de fidelidade. Um tradutor não certificado cuja saída é a
entrada efetiva do checker entra na de correção. A ausência de `unsafe` no código
próprio não implica dependências nem plataforma verificadas formalmente.

## Atacantes

IDs estáveis para rastreabilidade. “Atacante” inclui participantes comprometidos;
erro honesto com o mesmo efeito também aparece no modelo.

| ID | Participante e capacidade | Ameaças da §10 |
| --- | --- | --- |
| A1 | LLM comprometido por injeção: conteúdo externo controla propostas, extrações, escolha de ações e prosa; sem chave/credencial própria no desenho mediado | 1, 12, 13, 14, 16 |
| A2 | Agente malicioso: ignora o cliente oficial, escolhe bytes, premissas, claims e chamadas, insiste/reordena/submete propostas concorrentes | 1, 4, 5, 6, 10, 11, 12, 13, 14, 15, 17 |
| A3 | Prover malicioso: envia prova inválida, resposta falsa, certificado gigante ou trabalho dependente do tempo; tenta explorar bugs do decoder/checker | 6, 7, 15, 17, 18 |
| A4 | Atestador comprometido ou errado: assina fato falso, obsoleto ou conflitante; pode omitir mudança de versão | 2, 4, 10 |
| A5 | Operador ou kernel malicioso/comprometido: controla processo, armazenamento ou distribuição de cabeças e mostra histórias diferentes a partes diferentes; inclui operador curioso que observa dados sob seu controle | 2, 3, 6, 7, 8, 9, 11, 15, 18 |
| A6 | Outro agente na rede: envia PVC/premissas enganosas, provas inválidas ou gigantes; observa/rouba/retransmite mensagens se acesso permitir; não recebe confiança automática por assinatura remota | 1, 4, 5, 6, 7, 8, 10, 11, 12, 13, 15, 16, 17 |
| A7 | Autor/administrador de política malicioso ou equivocado: publica regras permissivas, contraditórias ou incompletas sobre trajetórias/fontes | 3, 4, 13 |
| A8 | Apresentador de interface enganosa ou usuário induzido a erro: troca explicação, omite condições do selo, confirma formalização errada ou contorna verificações frustrantes | 1, 15, 16 |

Acesso a chave/credencial por A5/A6 é cenário de comprometimento, não capacidade
normal de cliente. Operador curioso não é automaticamente falsificador, mas não
é excluído do modelo por ser o dono do servidor. Bugs honestos de encoder/checker
(6/7) e pressão operacional de `UNKNOWN` (15) não exigem adversário ativo;
a tabela identifica quem pode provocá-los, explorá-los ou amplificar o dano.

## Ameaças → atacantes e etapas

Numeração e natureza preservadas da §10. Etapas citadas são **destinos de
mitigação**, não evidência de que já estejam completas. Nos limites fundamentais,
mitigação reduz risco ou torna dependências visíveis; não resolve o limite.

| # | Ameaça | Atacantes | Etapas do Apêndice B / risco residual |
| --- | --- | --- | --- |
| 1 | Formalização infiel | A1, A2, A6, A8 | 9.1, 9.3, 15.1–15.6; fundamental, declarada em 0.7/11.10 |
| 2 | Atestador comprometido ou errado | A4, A5 | 6.2, 6.4; fundamental, declarada em 0.7 |
| 3 | Política errada | A5, A7 | 7.6; fundamental, declarada em 0.7 |
| 4 | Premissas inconsistentes | A2, A4, A6, A7 | 5.5, 14.6; exigir testemunha, não supor consistência decidível em toda lógica |
| 5 | Agente escreve a própria obrigação | A2, A6 | 1.5, 1.6, 5.3 |
| 6 | Bug em encoder ou grounder | A2, A3, A5, A6 | 4.2, 4.3, 13.3, 22.2; canon existente não prova traduções futuras |
| 7 | Bug de correção num checker | A3, A5, A6 | 4.4, 4.5, 5.10, 14.5, 20.2 |
| 8 | Chave do kernel comprometida | A5, A6 | 13.1, 8.3; FileSigner não oferece isolamento de HSM |
| 9 | Histórias diferentes para partes diferentes | A5 | 13.2; assinatura/hash chain não garantem ausência de forks |
| 10 | TOCTOU | A2, A4, A6 | 8.5; parte fundamental declarada em 0.7/8.3 (`unprotected_pre`) |
| 11 | Replay ou roubo de token | A2, A5, A6 | 8.3, 8.4 |
| 12 | Decomposição de ação proibida | A1, A2, A6 | 7.5, 11.6; depende de política sobre trajetórias adequada |
| 13 | Injeção escolhe entre ações permitidas | A1, A2, A6, A7 | 6.6, 9.2, 11.1, 11.2; fundamental, declarada em 0.7/11.10 |
| 14 | Enfraquecimento até trivialidade | A1, A2 | 10.5 (`Weakened`), 16.2 |
| 15 | Excesso de `UNKNOWN` e contorno pelo usuário | A2, A3, A5, A6, A8 | 8.7, 10.8; override registrado não é prova |
| 16 | Confiança excessiva no selo | A1, A6, A8 | 15.1, 16.3; fundamental, declarada em 0.7 |
| 17 | Certificados gigantes | A2, A3, A6 | 0.5 (`TooLarge`), 0.6 (`Unknown(Size)`), 17.3; sem teto global de RSS |
| 18 | Check dependente da máquina/timeout | A3, A5 | 0.6; passos determinísticos, timeout externo é `Unknown(Time)` |

## Premissas de plataforma e fora do escopo

- Compilador/toolchain, SO, hardware e filesystem executam seus contratos.
  Ataques à implementação desses componentes estão fora da revisão/mitigação
  da 0.7, **não** fora das premissas de validade das garantias.
- SO deverá isolar usuários/processos, chave, ledger e credenciais. Root malicioso
  pode quebrar essas premissas; `forbid(unsafe_code)` não impede isso.
- Hash/assinatura pressupõem segurança criptográfica e gestão correta das chaves.
  Assinatura atesta autoria dos bytes, não verdade, sigilo ou história única.
- A confidencialidade contra operador curioso, canais laterais e segurança física
  não são garantias enunciadas aqui. A5 continua atacante de integridade na ameaça 9.
- Disponibilidade da rede, plataforma e serviços não é garantida. Orçamentos
  delimitam trabalho contabilizado; não obrigam LLM/prover a produzir certificado.
- Mitigações de kernel, gateway, transparência, HSM, políticas e interface,
  bem como prova mecanizada, pertencem às etapas indicadas; não são implementadas
  pela presença destes documentos.
