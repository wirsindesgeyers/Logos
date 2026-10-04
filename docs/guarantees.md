# Garantias e suas bases de confiança

Etapa 0.7 · 2026-10-04. Fontes: [spec §1, §5, §6 e §13](spec/LOGOS-proof-carrying-agents.md),
[manifesto](../tcb/manifest.toml), [ameaças](threat-model.md) e [limites](limits.md).
**G1–G3 são contratos do sistema projetado, não garantias end-to-end já
implementadas ou teoremas mecanizados.** A 0.7 apenas os documenta.

TCB é relativa à propriedade: bug em componente de correção pode aceitar
inferência/autorização inválida; bug em componente de fidelidade pode ligar
prova válida à coisa errada; falha nos produtores afeta progresso.
As dependências transitivas usadas na execução também entram na confiança,
não apenas LOC próprios. Compilador, SO/hardware e filesystem são premissas
subjacentes, fora do escopo de mitigação desta etapa.

## G1 — Transição comprometida acompanhada de evidência (§1)

**Enunciado.** Se um efeito mediado ocorreu ou um claim foi marcado como
verificado pelo sistema, existe no ledger uma cadeia de eventos assinados pelo
kernel ligando-o a obrigação gerada deterministicamente, certificado aceito por
checker identificado por hash e premissas com origem registrada. A validade da
cadeia não depende da correção do LLM ou do solver.
No fragmento decidível, a avaliação direta confiável ocupa a rota de autorização:
não se exige prova externa redundante (L2); o resultado e suas dependências
precisam permanecer auditáveis.

**Premissas.**

- Toda transição comprometida passa pelo monitor; agente, LLM, prover e cliente
  não escrevem o ledger, não emitem credenciais e não executam efeito por fora.
  SO e operador preservam isolamento e acesso exclusivo às credenciais.
- Gerador produz a obrigação a partir da proposta/citações admitidas; agente
  não fornece obrigação autoritativa. Checker confere exatamente os bytes
  comprometidos, na lógica/versão corretas, sem confiar em `unsat` do solver.
- Checker ativo é correto para o fragmento e identificado pelo binário efetivo;
  dependências usadas são extraídas do certificado/avaliação, não declaradas
  livremente pelo agente. Estado, escopo, revogações e épocas são respeitados.
- Eventos são persistidos/autenticados e ligados ao efeito ou claim correto;
  chaves permanecem secretas e hash/assinatura seguros.
- `Unknown` e `Rejected` não viram aceite; contramodelo aceito, não certificado
  rejeitado, é evidência de refutação. Claim especulativo não autoriza efeito.
- A propriedade é sobre a história autenticada examinada. Unicidade global
  entre leitores exige a transparência da 13.2; kernel/operador que bifurca
  histórias é ameaça 9, não algo excluído do modelo.

**TCB de correção.** Serialização canônica (`logos-canon`), hash/assinatura e
Signer (`logos-crypto`); representações e gerador (`logos-air`,
`logos-obligation`); checkers e traduções não certificadas da rota usada
(`logos-model`, `logos-ground`, `logos-lrat` e binário externo quando registrado);
CAS/appender/estado do ledger (`logos-ledger`), máquina de estados/registry
(`logos-kernel`), avaliador de política quando usado (`logos-policy`), emissor
(`logos-token`), gateway/adaptador (`logos-gateway`) e tratamento de resultados/
orçamentos (`logos-core`). Rotas futuras Alethe/ITP acrescentam seus checkers e
tradutores efetivamente confiados, sem inferir confiança pela linguagem usada.

**TCB de fidelidade.** Para interpretar a cadeia como verdade/intenção no mundo:
atestadores e autoridades de confiança, autores de políticas e contratos,
vocabulário/grounding, renderizador determinístico, interface de confirmação
fiel aos bytes e pessoa que confirma. Não são necessários para verificar apenas
a derivabilidade formal; são necessários para atribuir significado à conclusão.

**Dependências de progresso.** LLM, formalizador, prover farm, solvers,
transporte e serviços disponíveis. Podem não fornecer proposta/prova útil;
não lhes é permitido transformar ausência de prova em aceite.

**Estado.** Primitivas 0.3–0.6 e sintaxe AIR 1.1 implementadas; ledger de eventos, kernel,
checkers lógicos e emissão/execução de tokens ainda não. G1 será exercitada nos
marcos 5.12/8.9; prova formal nas etapas 12.1–12.4. Nenhuma evidência atual
comprova mediação completa.

## G2 — Execução pelo gateway vinculada à chamada (§6)

**Enunciado.** Se o adaptador executou chamada `c`, existe token `τ` assinado
pelo kernel, ligado a `h(c)` e a evento que certifica
`Pol_v ∪ Obs ∪ Auth ⊢ Allowed(α(c))`. `Pol_v` foi assinada pela autoridade,
`Obs` por atestadores aceitos pela política, e as premissas têm testemunha de
consistência. Pré-condições **protegidas por versão atômica** valem no instante
da execução. Pré-condições não protegidas são explicitamente `unprotected_pre`,
sem promessa de validade instantânea (ameaça 10).

**Premissas.**

- `α(c)` deriva dos bytes reais de ferramenta/argumentos pelo contrato, nunca
  da descrição textual do agente; adaptador executa exatamente a chamada
  cujo hash/audiência constam no token.
- Política vigente/época/raiz autenticada e regras negativas usam fechamento
  explícito, não ausência de informação tratada como prova. Fatos têm fonte,
  escopo e versão verificáveis e trust policy por tópico.
- Checker ou avaliador direto correto autoriza `Allowed(α(c))`; testemunha
  de consistência é aceita para as premissas usadas. Falta de testemunha
  não é prova de consistência; efeito não é especulativo.
- Quando exigida, autorização humana é assinatura sobre termo formal e
  paráfrase determinística; “ok” no chat não é autorização criptográfica.
  A pessoa vê o mesmo conteúdo que assina, com chave/interface confiáveis.
- Adaptador é único detentor de credencial, verifica assinatura, hash da
  chamada, audiência, expiração, época/revogação e consome nonce sem corrida.
  Relógio e armazenamento de nonces atendem ao contrato.
- Comparação de versões e efeito protegido são uma operação atômica no recurso
  correto. TTL curto, consulta antes da chamada ou assinatura do fato não
  substituem transação/CAS/`If-Match`. Efeito observado produz atestado e vínculo
  correto com `Executed`; falha de registro não desfaz efeito já ocorrido.
- Isolamento, disponibilidade de evidência persistente e segredo das chaves
  satisfazem G1. Não há promessa de sucesso da ferramenta nem de rollback físico.

**TCB de correção.** `logos-canon`, `logos-crypto` e Signer; representação e
gerador (`logos-air`, `logos-obligation`); checkers/tradutores da rota usada ou
avaliador direto (`logos-model`, `logos-ground`, `logos-lrat`, `logos-policy` e
binário registrado quando usado); `logos-core`, CAS/appender do `logos-ledger`,
estado/registry de `logos-kernel`, emissor `logos-token`, verificador e adaptador
`logos-gateway`. Também protocolo transacional/condicional do serviço alvo,
relógio e estado anti-replay usados pelo adaptador.

**TCB de fidelidade.** Autor de política, contrato da ferramenta (efeito real
corresponde a `α`), atestadores e política de confiança, vocabulário/grounding,
renderizador, interface de autorização e usuário quando a classe de ação exigir
confirmação. Uma assinatura válida de fato falso não satisfaz verdade no mundo.

**Dependências de progresso.** LLM, prover/solver, transporte e serviço da
ferramenta. Indecidibilidade ou ausência de prova/testemunha pode impedir ação;
nenhuma garantia de que toda ação legítima conseguirá autorização.

**Estado.** Contratos/intent/mint/gateway/execução condicional são Fase 8;
fatos e autorização são Fases 6/9; ainda não implementados. G2 não é demonstrada
por CAS nem pela variante `Verdict::Verified`. Marco 8.9 e modelo 12.2 são futuros.

## G3 — Integridade de justificativa, não de influência (§13)

**Enunciado.** Para toda ação executada `a`, existe evidência aceita para
`Pol_v ∪ Γ ⊢ Allowed(a)`, com `Γ` formado pelas premissas efetivamente usadas
pelo certificado (ou avaliação direta na rota L2). Cada `p ∈ Γ` tem origem cuja
confiança satisfaz o requisito de `Pol_v` para a classe de `a`. Na rota de prova,
a evidência é o certificado aceito enunciado na §13.

**Premissas.**

- Mediação e ligação chamada/obrigação/evento/token/efeito de G1 e G2;
  políticas corretas expressam as fontes aceitáveis por conteúdo e classe.
- Conteúdo externo é representado como `fonte diz φ` com origem autenticada,
  não promovido silenciosamente a `φ`. Extração não confiável não recebe
  identidade/chave/autoridade de atestador ou usuário.
- Dependências efetivas são extraídas e verificadas pelo lado confiável;
  agente/prover não pode ocultar origem em `Γ` nem trocar a política vigente.
- Premissas usadas satisfazem trust policy, escopo/época e consistência;
  assumptions remotas não viram fatos aceitos por assinatura de outro kernel.
- Extrator/formalizador pode errar conteúdo; rótulo de fonte e política
  permanecem intactos. Para implicar intenção do usuário, a política precisa
  cobrir essa intenção; derivabilidade isolada não prova isso.

**TCB de correção.** `logos-canon`, `logos-crypto`, `logos-core`, representação/
gerador (`logos-air`, `logos-obligation`), checker e extrator confiável de
premissas usadas (incluindo `logos-ground`/`logos-lrat` e checker externo na rota
finita), avaliador e verificador de origem/requisitos (`logos-policy`), estado/
registry (`logos-kernel`), CAS/appender e proveniência (`logos-ledger`), emissor
(`logos-token`) e gateway/adaptador (`logos-gateway`). Checkers e tradutores
adicionais entram quando a rota efetivamente os consumir.

**TCB de fidelidade.** Autores da política de endosso semântico, atestadores/
raízes por tópico, contratos/classificação de ações e vocabulário; renderizador,
interface e usuário para ações que exigem autorização humana. Política incompleta
pode aceitar origem imprópria à intenção embora a derivação esteja correta.

**Dependências de progresso.** LLM/planner, extrator em quarentena,
formalizador, prover/solver, MCP e rede. Não são autoridades de endosso;
erros podem causar `Unknown`, rejeição ou escolha ruim entre ações permitidas.

**Estado.** `Says`/endosso, dependências de prova, política e execução ainda
futuros (6.6, 4.7/14.4, Fases 7/8 e 11.1–11.2). Teorema 12.3 e avaliação da
Fase 11 não executados. Não implica não-interferência: dado não confiável pode
influenciar qual ação permitida é escolhida (ameaça fundamental 13).

## O que existe hoje, e o que isso não autoriza concluir

| Primitiva implementada | Contrato local / fronteira |
| --- | --- |
| `logos-canon`, canon/v1 | Bytes canônicos/decoder segundo spec; não fornece AIR nem fidelidade de formalização |
| `logos-crypto` | Hash com domínio, assinatura Ed25519 estrita, FileSigner; não atesta verdade, não é HSM, ES256 está reservado e não implementado |
| CAS em `logos-ledger` | Endereço/integridade/limite por objeto e publicação tmp+rename/fsync sob premissas locais; não é ledger assinado, anti-fork ou prova experimental de power loss |
| `logos-core` | Tipos e latch de exaustão de passos; consumidor deve cobrar antes do trabalho; `mem_bytes`/`size_bytes` não monitoram RSS; sem supervisor de tempo nesta etapa |
| `logos-air`, air/v1 | Sorts/assinaturas e sintaxe FOL-ms-eq/v1 prefixa com De Bruijn e limites agregados; não é vocabulário, sort-check, prova nem credencial; decoder canon ainda materializa Value |

[ADR 0005](adr/0005-escolhas-do-cas.md) fixa filesystem local Unix e ancestrais
confiáveis. [ADR 0006](adr/0006-veredictos-e-orcamentos.md) fixa a fronteira dos
enums: `Verdict`/`CheckOutcome` são públicos e decodificáveis, **não credenciais**.
Criar `Accepted` ou `Verified` localmente não prova nada nem concede autorização.
`Rejected` significa certificado recusado, não `Refuted`; `Unknown` nunca é aceite
nem refutação. O medidor não interrompe closure nem detecta trabalho não cobrado.

O manifesto lista também esqueletos; `verified = "none"` não é prova formal.
O auditor futuro poderá rechecar evidências sem depender de kernel/ledger em
código; o gate de independência atual só confere dependências, não implementa
auditoria. CI remota, fuzz noturno e execução ARM continuam pendências no
[STATUS](../STATUS.md), distintas da documentação local da 0.7.
