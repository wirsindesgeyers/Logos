# LOGOS — Proof-Carrying Agents

Oct 1, 2026 · @kauan

Um proof-carrying agent é viável e vale a pena, mas como **sistema de efeitos mediado por prova**, não como “agente que raciocina corretamente”. O LLM e o solver podem sair inteiramente da base de confiança. A ligação entre fórmulas e mundo nunca sai. É nesse limite que estão tanto a falha fundamental quanto a contribuição possível.

## 1. Tese central

A unidade de confiança deixa de ser a resposta e passa a ser a **transição de estado**. Um proof-carrying agent é um agente cuja única forma de alterar estado comprometido (registrar um claim utilizável, executar um efeito no mundo, entregar um claim a outro agente) é apresentar a um monitor de referência um certificado que um kernel pequeno consegue checar.

Três ideias antigas, combinadas, dão a forma da arquitetura:

- **Arquitetura LCF** (Milner, anos 1970): teoremas são um tipo abstrato que só o kernel consegue construir. No LOGOS, “claim verificado” e “autorização para agir” são objetos que só o kernel consegue emitir.
- **Proof-carrying code** (Necula e Lee, 1996): quem produz o artefato não é confiável; quem consome checa uma prova contra uma política. O LLM é o produtor não confiável por excelência.
- **Monitor de referência** (Anderson, 1972): mediação completa, à prova de adulteração, pequeno o suficiente para ser verificado. A garantia vem de o agente **não ter credenciais**, e não de um prompt pedindo para ele se comportar.

A formulação precisa da tese:

> Se um efeito ocorreu ou um claim foi marcado como verificado, então existe no ledger uma cadeia de eventos assinados pelo kernel ligando esse efeito ou claim a uma obrigação gerada deterministicamente, a um certificado aceito por um checker identificado por hash, e a premissas cuja origem está registrada. Nada nessa cadeia depende da correção do LLM ou do solver.

### Três limites fundamentais

Estes não são problemas de engenharia a resolver depois. São limites da ideia, e o projeto só é honesto se os declarar.

**L1. A prova não certifica a ligação entre símbolos e mundo.** Um certificado garante `Γ ⊢ φ`. Não garante que `φ` significa o que a frase queria dizer, nem que os fatos em `Γ` são verdadeiros. A confiança não some: ela migra para quem formaliza e para quem atesta fatos. A arquitetura só consegue tornar essa migração explícita, medida e assinada.

**L2. Para políticas decidíveis e pequenas, “proof-carrying” é redundante.** Se a política de ações está num fragmento que o gateway decide sozinho em milissegundos (RBAC, Datalog sem recursão pesada, Cedar), o gateway deve simplesmente avaliar a política. Pedir uma prova ao agente não acrescenta segurança. Prova só se paga quando (a) checar é muito mais barato que decidir, como em lógica de primeira ordem ou aritmética; (b) o checker precisa ser menor que o decisor; ou (c) a justificativa precisa viajar e ser rechecada por terceiros.

**L3. Mediação controla efeitos e ledger, não cognição.** O LLM continua com o texto de um claim não verificado no contexto e pode usá-lo na prosa. O que a arquitetura garante é que nada não verificado vira efeito, entra no ledger como verificado ou é entregue a outro agente com selo. Portanto, “proof-carrying agent” é, mais precisamente, um **sistema de efeitos mediado por prova** com um **ledger epistêmico**. A resposta final precisa ser renderizada a partir do ledger, com o texto livre marcado como não verificado.

Disso sai uma divisão estrutural que atravessa todo o documento: existem dois regimes com garantias muito diferentes.

| Regime | Exemplo | Premissas vêm de | Formalização | Força da garantia |
| --- | --- | --- | --- | --- |
| Operacional, mundo fechado | Pode o agente apagar este banco? | Políticas escritas formalmente por humanos; fatos atestados por ferramentas | Quase nenhuma: ação derivada dos bytes da chamada | Forte |
| Epistêmico, mundo aberto | Este argumento filosófico é válido? | Frases em linguagem natural | Pesada, feita por LLM | Condicional à fidelidade |

O valor do LOGOS está em tratar os dois com a mesma máquina, sem deixar o selo do primeiro emprestar credibilidade ao segundo.

## 2. Arquitetura proposta

A regra que organiza tudo: **o agente pode propor claims, citar premissas e fornecer provas; nunca pode escrever obrigações, fatos ou políticas.** Se o agente escrevesse a obrigação, poderia submeter `⊤` e prová-la. Se escrevesse fatos, a prova seria sobre ficção.

```text
  Linguagem natural / Ambiente
         │                         │
         │ texto                   │ observações
         ▼                         ▼
  ┌───────────────┐        ┌────────────────┐
  │ AGENTE (LLM)   │        │ ATESTADORES     │  ferramentas, sensores,
  │ não confiável  │        │ assinam fatos   │  usuário (passkey), admin
  └──────┬────────┘        └────────┬────────┘
         │ PROPOSE: claim, fórmula,  │ Fact{φ, emissor, época, assinatura}
         │ premissas citadas         │
         ▼                           ▼
  ┌──────────────────────────────────────────────┐
  │ KERNEL (TCB)                                     │
  │  1. gerador de obrigações  (determinístico)        │
  │       O = canon(Γ_citado ⊢ φ)  → hash(O)          │
  │  2. checkers de certificado                      │◄── certificado
  │       modelo finito · LRAT · Alethe · kernel ITP  │    (de provers não
  │  3. appender do ledger (hash chain + assinatura) │     confiáveis)
  │  4. emissor de capabilities                      │
  └───────────┬────────────────────────────────────┘
              │ VERIFIED | REFUTED(M) | UNKNOWN
              ▼
  ┌────────────────────────┐     ┌──────────────────────────┐
  │ LEDGER (append-only)   │────►│ GATEWAY de ferramentas    │
  │ eventos assinados      │ tok │ detém as credenciais;     │
  └───────────┬────────────┘     │ executa só com token     │
              │                  └──────────────────────────┘
              ▼
  continuação do agente: só cita o que está no ledger
```

### Os artefatos e quem pode produzi-los

| Artefato | O que é | Quem produz | Confiança |
| --- | --- | --- | --- |
| Proposal | Claim candidato: texto, fórmula, premissas citadas | Agente | Nenhuma. Não tem status |
| Assumption | Fórmula admitida hipoteticamente, com escopo | Agente ou usuário, registrado com autor | Tudo que depende dela é condicional e carrega a assumption no antecedente |
| Fact | Fórmula atestada: `{φ, emissor, época, assinatura}` | Atestador (ferramenta, sensor, fonte, usuário, admin). **Nunca o LLM** | Tanto quanto o emissor, segundo a política de confiança de quem consome |
| Policy | Regra formal versionada, com raiz Merkle por versão | Autoridade humana, escrita direto em linguagem formal | Tanto quanto quem assinou; sem gap de linguagem natural |
| Derived claim | `φ` ligado a um evento `Verified(Γ ⊢ φ)` | Kernel, ao aceitar um certificado | Válido relativo a `Γ`; o status de `Γ` é herdado |
| Proof obligation | Sequente canônico `Γ ⊢ φ`, hasheado | **Kernel**, deterministicamente, a partir da proposal | É o objeto que o certificado precisa fechar, byte a byte |
| Proof | Qualquer derivação: trace de solver, texto, script | Qualquer um | Nenhuma até virar certificado |
| Proof certificate | Prova num formato que um checker do kernel aceita (modelo, LRAT, Alethe, termo Lean, MM0), ligada a `hash(O)` | Provers não confiáveis (Z3, cvc5, Vampire, LLM) | Confiança só depois do check |
| Countermodel | Estrutura finita `M` com `M ⊨ Γ` e `M ⊭ φ` | Provers / model finders | Checado avaliando fórmulas em `M`: o checker mais simples do sistema |
| Provenance | DAG de eventos e assinaturas: quem introduziu cada nó, quando, de onde | Ledger, automaticamente | Imutável por hash chain |
| Fidelity attestation | `{hash(texto), hash(φ), hash(vocab), método, atestador, assinatura}` | Usuário (assinatura), checker de equivalência, ou LLM (nível mais fraco) | Dimensão separada da validade (seção 8) |
| Authorization to act | Capability assinada pelo kernel, ligada à chamada concreta e ao estado | **Kernel**, só após `Verified(Pol ∪ Fatos ⊢ Allowed(a))` | Aceita pelo gateway sem consultar o LLM |

Duas assimetrias importantes aparecem na tabela. Refutar é mais barato de checar que provar: um contraexemplo se confere avaliando fórmulas num modelo finito, enquanto `VERIFIED` exige um certificado de insatisfatibilidade. E `UNKNOWN` não exige nada, o que é justamente por que ele nunca pode autorizar coisa alguma.

## 3. Agent lifecycle

O verificador só é parte do loop de verdade se certas transições forem **impossíveis** sem ele. “Impossível” aqui tem um sentido operacional preciso, garantido por cinco mecanismos, nenhum deles baseado em prompt:

1. **Confinamento de capabilities.** O processo do agente não tem credenciais de ferramentas nem a chave de escrita do ledger. Ele só fala com o kernel. Não existe chamada que ele possa fazer para pular a verificação.
2. **Regra de citação.** O gerador de obrigações rejeita qualquer proposal que cite algo fora do ledger da época corrente. Um claim não verificado não pode ser premissa de outro claim verificado. É isso que impede o erro de se propagar no grafo.
3. **Eventos assinados.** “C7 é utilizável” significa: existe um evento `Verified` assinado pelo kernel para `hash(O)` de C7, não retratado. Qualquer componente confere isso sem perguntar a ninguém.
4. **Typestate dentro do kernel.** Em Rust, `Claim<Verified>` só tem construtor privado no crate do kernel. Isso protege a correção interna; entre processos, quem garante são as assinaturas.
5. **Resposta renderizada do ledger.** A resposta final é uma visão do ledger. Trechos de texto livre do LLM entram marcados como não verificados, como dados contaminados num sistema de information-flow control.

### Máquina de estados de um claim

```text
             PROPOSE
  [Draft] ─────────────► [Pending O] ──── cert aceito ────► [Verified] ──► citável
                            │   │                              │
                            │   └── contraexemplo M checado ──► [Refuted]
                            │                                       │
                            └─── timeout / sem cert ───► [Unknown]  │
                                                         │         │
               agente precisa: enfraquecer φ · declarar assumption · descartar · pedir humano

  [Verified] ── premissa retratada / época nova / checker revogado ──► [Invalidated]
                                   (cascata pelo DAG, exceto rotas independentes)
```

### Um trecho do protocolo

```text
STATE 14   agent  → PROPOSE C7 { text, φ7, cites: [F3, P2, C5] }
           kernel → citações ∈ ledger(época 9)?            ok
           kernel → O19 = canon({F3, P2, C5} ⊢ φ7)       h(O19) = 9f…
STATE 15   agent  → prover (não confiável): resolve O19   → π (Alethe)
           agent  → CERTIFY O19 π
           kernel → checker alethe@h(c3…) aceita π para h(O19)?  ok
           kernel → testemunha de consistência M0 ⊨ {F3,P2,C5}?   ok
           ledger → E212 { Verified, obl: 9f…, cert: h(π), checker: c3…,
                           consist: h(M0), prev: h(E211), sig_kernel }
STATE 16   C7 citável na época 9

variante:  kernel → M ⊨ {F3,P2,C5} e M ⊭ φ7?   ok
           ledger → E212 { Refuted, obl: 9f…, model: h(M) }
           agent  ← M traduzido pelo vocabulário
           kernel bloqueia nova PROPOSE com o mesmo h(O19)
```

O bloqueio da última linha é importante: sem ele, o agente pode reenviar a mesma proposta até algum prover dar timeout e transformar uma refutação num `UNKNOWN` mais confortável.

### Especulação: permitida para claims, proibida para efeitos

Esperar o verificador a cada passo deixa o agente lento. A solução é a mesma das CPUs: especular e reverter. O agente pode propor C8 citando C7 ainda `Pending`; C8 entra como condicional a C7 e, se C7 for refutado, C8 cai junto. A regra inviolável é que **nenhum efeito é especulativo**: um token de ação só é emitido sobre premissas todas `Verified` ou atestadas.

### O que “intercalado” significa, de forma mensurável

Na verificação pós-hoc, um erro no passo 3 contamina os passos 4 a 10 e só aparece no fim. Na intercalada, a regra de citação impede o passo 4 de usar o passo 3 não verificado. A métrica natural é a **profundidade de propagação de erro**: quantos nós do DAG dependem de um nó inválido no momento em que ele é detectado. No modo intercalado sem especulação ela é zero por construção; a pergunta empírica é quanto isso custa em latência e em tarefas abandonadas.

## 4. Proof lifecycle

Uma prova no LOGOS tem vida própria: nasce ligada a uma obrigação, é checada, registrada, citada, e pode morrer depois, por motivos que não têm a ver com ela.

```text
 proposal ──► [1 obrigação] ──► [2 busca] ──► [3 certificado] ──► [4 check]
               kernel            não confiável   não confiável     kernel
                                                                  │
         ┌───────────────────────────────────────────────────────────┘
         ▼
 [5 não-vacuidade] ──► [6 registro] ──► [7 citação] ──► [8 invalidação?] ──► [9 recheck]
  modelo de Γ         evento assinado  por claims e    premissa, época,     qualquer um,
                                       tokens          checker revogado    a qualquer hora
```

**1. Geração da obrigação.** O kernel monta `Γ ⊢ φ` a partir da proposal e serializa de forma canônica: premissas ordenadas, nomes de variáveis normalizados, identificador da lógica (por exemplo, FOL clássica multi-sortida com igualdade), hash do vocabulário e versão do encoder. O hash disso é a identidade da obrigação. **Princípio: o checker consome exatamente os bytes que o ledger hasheia.** Se houver tradução entre o objeto hasheado e o objeto checado, o tradutor entra na TCB.

**2. Busca.** Qualquer prover, em qualquer lugar: Z3, cvc5, Vampire, um SAT solver, um LLM que escreve termos Lean. Nada aqui é confiável e nada precisa ser.

**3. Certificado.** O prover entrega uma prova num formato que algum checker do kernel aceita. Aqui mora uma armadilha técnica: provers de primeira ordem quase sempre trabalham sobre a forma clausal, após skolemização e CNF. Essa transformação precisa estar dentro da cadeia checada, ou porque o formato de prova a registra (o Alethe tem regras para isso), ou porque o kernel faz a própria clausificação com código pequeno e auditado.

**4. Check.** Checker identificado pelo hash do binário, determinístico, com limite de tempo e memória. Resultado binário: aceita ou rejeita. Certificado rejeitado não é refutação; a obrigação volta para `Pending`.

**5. Não-vacuidade.** Um `Verified` sobre premissas inconsistentes é inválido na prática, porque qualquer coisa segue. Todo evento `Verified` carrega o status de consistência de `Γ`: uma testemunha (modelo finito, barata de checar) ou `UNKNOWN`, que fica visível no selo como “possivelmente vazio”. Além disso, **as dependências registradas no DAG são extraídas do certificado, não da citação do agente**: um certificado diz exatamente quais premissas usou. Isso dá o unsat core de graça e impede o agente de inflar ou esconder dependências.

**6. Registro.** Evento `{tipo, h(obrigação), h(certificado), h(checker), h(testemunha), época, h(evento anterior)}` assinado pelo kernel. O certificado fica em armazenamento endereçado por conteúdo; o ledger guarda o hash.

**7. Citação.** Claims e tokens de ação referenciam eventos, não textos.

**8. Invalidação.** Quatro causas, todas registradas como eventos (o ledger nunca apaga):

- uma premissa é retratada (fato revogado, assumption rejeitada): cascata pelo DAG, como num ATMS;
- muda a época da política: provas sobre a versão anterior deixam de autorizar ações;
- **um checker é revogado**: se aparece um bug de correção no checker de hash X, o ledger lista todos os eventos aceitos por X e eles são rechecados com outro checker;
- o vocabulário muda de significado: todas as fórmulas com o hash de vocabulário antigo perdem o atestado de fidelidade.

**9. Recheck.** Este é o ganho real de certificados sobre solvers confiáveis. Um veredicto do Z3 só pode ser **re-executado**, e a nova execução tem os mesmos bugs. Um certificado pode ser **rechecado** por um checker diferente, escrito por outra equipe, a qualquer momento, por qualquer pessoa. Revogação de checker só é possível porque existe recheck.

Custo a declarar: certificados podem ser enormes (provas LRAT de problemas difíceis chegam a gigabytes). O sistema precisa de limites de tamanho e trata certificado grande demais como `UNKNOWN`, nunca como aceite.

## 5. Trust boundaries

### “Semi-trusted” não existe numa cadeia de correção

A divisão em UNTRUSTED / SEMI-TRUSTED / TRUSTED tem um problema conceitual: para uma propriedade de segurança, um componente ou está na TCB (um bug nele pode produzir um `Verified` falso) ou não está. Não há meio-termo. Um tradutor AIR → SMT, por exemplo, é **totalmente confiável** se o checker confere a prova contra a saída do tradutor, e **não confiável** se o checker confere contra a fórmula AIR hasheada. A decisão de arquitetura é empurrar cada componente para um lado ou outro, nunca deixá-lo no meio.

O que existe de fato são **propriedades diferentes**, cada uma com sua TCB:

| Propriedade | O que garante | TCB |
| --- | --- | --- |
| Correção (soundness) | Nenhum `Verified` ou token falso | Gerador de obrigações, checkers, ledger, emissor de tokens, gateway, criptografia |
| Fidelidade | A fórmula diz o que se quis dizer; os fatos refletem o mundo | Atestadores, autores de políticas, **renderizador de paráfrases**, quem confirma |
| Progresso (liveness) | O sistema consegue concluir coisas | Provers, LLM, formalizador. Um bug aqui gera `UNKNOWN`, nunca um aceite falso |

Um detalhe que costuma passar: o renderizador que transforma `φ` em português para o usuário confirmar está na TCB de fidelidade. Se ele renderiza errado, o usuário assina a coisa errada. Por isso ele é determinístico, mora do lado do kernel, e a TUI só exibe a string que ele produziu. O usuário assina `h(paráfrase, φ)`, não o que a TUI mostrou.

```text
┌──────────────────────────────────────────────────────────────────────┐
│ NÃO CONFIÁVEL (só afeta progresso)                                    │
│  LLM · agente · planner · formalizador · Z3 · cvc5 · Vampire · TUI     │
│  transporte MCP · retrieval · encoders AIR→SMT (se checados contra AIR) │
│  ┌───────────────────────────────────────────────────────────────────┐  │
│  │ TCB DE FIDELIDADE                                               │  │
│  │  atestadores · autores de política · renderizador de paráfrase   │  │
│  │  usuário que confirma (com chave própria)                        │  │
│  │  ┌───────────────────────────────────────────────────────────┐   │  │
│  │  │ TCB DE CORREÇÃO                                         │   │  │
│  │  │  gerador de obrigações + serialização canônica        │   │  │
│  │  │  checkers: modelo finito · LRAT · Alethe · (kernel ITP) │   │  │
│  │  │  clausificador (se não for certificado)                │   │  │
│  │  │  appender do ledger · emissor de tokens · gateway      │   │  │
│  │  │  hash + assinatura · chave do kernel (HSM / TEE)        │   │  │
│  │  └───────────────────────────────────────────────────────────┘   │  │
│  └──────────────────────────────────────────────────────────────────┘  │
└──────────────────────────────────────────────────────────────────────┘
   abaixo de tudo: compilador Rust, SO, hardware (declarados, fora do escopo)
```

### Z3 deve estar na TCB?

Não, e por três razões independentes:

1. **Tamanho.** O Z3 tem centenas de milhares de linhas de C++. Nenhuma revisão humana cobre isso.
2. **Histórico.** Fuzzing sistemático de solvers SMT (por exemplo, o trabalho de semantic fusion de Winterer, Zhang e Su, PLDI 2020) encontrou bugs de correção no Z3 e no CVC4, isto é, casos em que o solver respondia `unsat` para fórmulas satisfazíveis. Para o LOGOS, um `unsat` falso é exatamente um `Verified` falso.
3. **Recheck.** Enquanto o Z3 estiver na TCB, o único jeito de conferir um veredicto é rodar o Z3 de novo.

O papel certo do Z3 é de **produtor não confiável**, e ele continua muito útil nos dois sentidos. Para `REFUTED` e consistência, ele produz modelos, e modelos são triviais de checar: o Z3 pode errar à vontade ali. Para `VERIFIED`, ele é um prover rápido cujas respostas são reconfirmadas por um produtor que emite certificados (cvc5 com saída Alethe, por exemplo) ou são reconstruídas como prova checada. O formato de prova nativo do Z3 é de granularidade grossa e difícil de checar de forma independente; por isso, para certificados SMT, o cvc5 é o produtor mais natural hoje.

Regra resultante: **todo veredicto que autoriza algo passa por um checker; solvers só aceleram a busca.**

### Reduzindo a TCB de correção, em ordem de impacto

1. Tirar solvers da TCB (acima).
2. Fazer o checker consumir a obrigação hasheada direto, eliminando tradutores da TCB.
3. Escrever gerador de obrigações, appender e gateway em Rust sem `unsafe`, com dependências mínimas, e verificar seus invariantes com uma ferramenta de verificação de Rust (Verus, Kani ou Creusot).
4. Usar checkers verificados onde existem: o `cake_lpr`, checker de provas LRAT verificado em CakeML até o código de máquina, para o fragmento proposicional.
5. A longo prazo, um único checker de framework lógico (seção 9) no lugar de vários checkers específicos.

## 6. Proof-carrying actions

Este é o regime em que a ideia é mais forte, porque quase todo o gap de linguagem natural pode ser eliminado: políticas são escritas formalmente por humanos, fatos vêm de atestadores, e a ação é derivada dos bytes da chamada, não da descrição do LLM.

### Fluxo

```text
agente ── ActionIntent{ tool: "db.drop", args: {name:"orders_v1"} } ──► GATEWAY

GATEWAY
 1. a := α(tool, args)            termo formal calculado pelo contrato da ferramenta,
                                  NUNCA pelo texto do agente
 2. Pol_v := política vigente     com raiz Merkle r_v assinada pelo admin
 3. Obs := atestadores consultam o estado AGORA, cada fato com versão:
          Backup(orders_v1, t=12:01)  @ backup-svc, ver 88
          Connections(orders_v1) = 0  @ db-monitor, ver 4411
 4. Auth := autorização do usuário, assinada sobre h(a) e sua paráfrase
 5. O := canon(Pol_v ∪ Obs ∪ Auth ⊢ Allowed(a))
        │
        ▼
prova: avaliação direta (política decidível)  OU  certificado de um prover
        │
        ▼
KERNEL: checa · testemunha de consistência · evento E901 no ledger
        │
        ▼
TOKEN τ = sign_k{ h(tool,args), h(a), E901, r_v,
                  pre: [(backup-svc, 88), (db-monitor, 4411)],
                  aud: "db-adapter", nonce, exp: +30s }
        │
        ▼
ADAPTADOR DA FERRAMENTA (único detentor da credencial do banco)
 6. confere assinatura, aud, exp, nonce não usado, h(tool,args) == chamada real
 7. executa CONDICIONADO a pre (transação / compare-and-swap / If-Match)
 8. efeito observado → atestado → ledger E902 { Executed, τ, resultado }
```

A garantia, enunciada de forma verificável:

> Se o adaptador executou a chamada `c`, então existe um token `τ` assinado pelo kernel com `h(c)`, ligado a um evento que certifica `Pol_v ∪ Obs ∪ Auth ⊢ Allowed(α(c))`, com `Pol_v` assinada pela autoridade, `Obs` assinadas por atestadores confiáveis, `Γ` consistente, e as pré-condições de versão válidas no instante da execução.

Isso é um teorema sobre o sistema, que pode ser formalizado e provado sobre o modelo da máquina de estados (seção 11).

### Por que o termo da ação vem dos bytes

Se o agente descreve a ação (“vou limpar registros antigos”) e o sistema prova sobre a descrição, há um gap de fidelidade entre descrição e chamada. Calculando `a = α(tool, args)` a partir do contrato da ferramenta, esse gap desaparece. Sobra uma única suposição: **o contrato descreve corretamente o efeito da ferramenta** (que `db.drop` realmente apaga só o banco nomeado). Ela é da TCB de fidelidade e fica escrita no contrato, revisável por humanos.

### TOCTOU: o mundo muda entre a prova e a ação

A prova vale para um retrato do estado no instante t; a ação ocorre em t'. A solução geral é a da concorrência otimista: **a prova não é rechecada na execução, mas os fatos são**, atomicamente com a ação. Isso só é possível até certo ponto, e o token precisa dizer até onde.

| Classe de pré-condição | Exemplo | Mecanismo | Risco residual |
| --- | --- | --- | --- |
| Recurso transacional | Linha de banco, saldo | Executar na mesma transação que verifica a versão | Eliminado |
| Recurso versionado | Objeto com ETag, arquivo em git | Requisição condicional (`If-Match`, CAS) | Quase eliminado |
| Fato atestado sem versão atômica | “Nenhum usuário conectado” | Janela de validade curta no token | Declarado no token como `unprotected_pre` |
| Fato do mundo físico | “A sala está vazia” | Nenhum mecanismo técnico resolve | A política pode exigir confirmação humana no instante |

O ponto honesto: a terceira e a quarta linhas não têm solução criptográfica. O máximo que a arquitetura faz é torná-las visíveis e deixar a política decidir.

### Fatos negativos e mundo fechado

Políticas reais têm negação: “permitido, salvo se existir uma regra de bloqueio”. Provar `Allowed` exige provar a **ausência** de bloqueios, e ausência num mundo aberto não tem certificado finito. A saída é transformar ausência em afirmação checada: a autoridade assina a raiz de uma árvore Merkle ordenada com **todas** as regras da versão `v`. Uma prova de não-pertinência na árvore vira premissa checada (“não existe regra de bloqueio para `a` na versão `v`”). É a mesma técnica de estruturas de dados autenticadas e transparência de certificados. Sem isso, qualquer política com negação tem um buraco silencioso.

### Autorização do usuário

`UserAuthorization12` não pode ser “o usuário disse ok no chat”. Ela é uma assinatura do usuário (passkey / WebAuthn) sobre `h(a)` e sobre a paráfrase determinística de `a`, mostrada a ele. O usuário autoriza o termo formal, não a narrativa do agente. Isso reaproveita o mesmo princípio de fidelidade da seção 8.

### Revogação

Três mecanismos, do mais simples ao mais caro:

1. **Validade curta.** Tokens de segundos. É o mecanismo principal; revogação explícita é exceção.
2. **Épocas.** O adaptador recusa tokens com `r_v` anterior à política mínima vigente. Mudar a política invalida todos os tokens antigos de uma vez.
3. **Lista de revogação** por evento ou por checker, para o caso de bug descoberto num checker (seção 4).

### Políticas sobre trajetórias, não só sobre ações

Um agente pode decompor uma ação proibida em várias permitidas (apagar tabela por tabela em vez do banco inteiro). Políticas sobre ações isoladas não pegam isso. Como o ledger é a história assinada de tudo que aconteceu, ele pode ser **premissa** de políticas: “`Allowed(drop(x))` exige um evento `Executed(backup(x))` no ledger na última hora”, ou “no máximo N deleções por sessão”. Isso é algo que policy engines sem ledger não conseguem expressar diretamente, e é um dos pontos mais interessantes da combinação.

### Relação com o que já existe

Seria desonesto apresentar isto como novo sem estas comparações:

- **Proof-carrying authorization** (Appel e Felten, 1999) e o sistema Grey (Bauer et al., 2005) já faziam o cliente apresentar uma prova de autorização numa lógica de “A says φ”, checada pelo servidor. Proof-carrying actions é, em essência, isso.
- **Policy engines** como Cedar (com avaliador formalizado e verificado) e OPA decidem autorização diretamente. Para políticas nesse fragmento, o LOGOS deve **usar avaliação direta** (limite L2) e reservar certificados para políticas fora dele.
- **Tokens com caveats**: macaroons (Birgisson et al., 2014) e Biscuit (tokens com regras Datalog, verificáveis offline). O token `τ` acima é um caso particular; usar Biscuit como formato evita inventar criptografia.
- **CaMeL** (Debenedetti et al., 2025) já separa um LLM privilegiado que escreve o plano de um LLM em quarentena que lê dados não confiáveis, rastreia capabilities por fluxo de dados e aplica políticas antes de cada chamada de ferramenta. É o vizinho mais próximo de “proof-gated tool execution” e o baseline obrigatório de qualquer avaliação.

O que sobra de diferente no LOGOS: lógica de política mais expressiva com certificados rechecáveis, políticas sobre o ledger (trajetórias), TOCTOU tratado no token, e o mesmo ledger servindo a claims epistêmicos e a efeitos.

## 7. Multi-agent verification

“B não deveria precisar confiar em A” é verdade só pela metade, e a metade que falta é a parte interessante. B nunca precisa confiar no **raciocínio** de A, porque pode rechecar o certificado. Mas B sempre precisa decidir se aceita as **premissas** de A, porque nenhuma prova transforma um fato afirmado por A em fato.

### A saída formal: claims portáveis são sequentes

O que A envia não é “C”. É o sequente `Γ ⊢ C` com certificado. Pelo teorema da dedução, isso equivale a `⊢ (⋀Γ → C)`: um teorema sem premissas. **B pode aceitar esse condicional sem confiar em ninguém.** O que B decide, segundo sua própria política de confiança, é quais elementos de `Γ` consegue descarregar.

Premissas vindas de outros agentes entram na lógica com o operador de atribuição das lógicas de autorização: `A says φ` (Abadi, Burrows, Lampson e Plotkin, 1993). B tem regras de delegação do tipo `A says φ ∧ Confiável(A, tópico(φ)) → φ`. Confiança vira premissa explícita, e não um pressuposto do protocolo.

### O pacote: Portable Verified Claim (PVC)

```text
PVC {
  claim        : φ, h(φ), vocab: h(V), lógica: "FOL-ms-eq/v1"
  sequent      : Γ = [ h(p1), h(p2), h(p3) ]      ← extraído do certificado
  premises     : p1 = Fact{ …, emissor: tool:db-monitor, época, sig }
                 p2 = PVC{ … }                       ← recursivo: DAG Merkle
                 p3 = Assumption{ autor: agent:A }
  certificate  : h(π), formato: alethe, onde obter: content-addressed
  consistency  : h(M0) | UNKNOWN
  fidelity     : [ FidAttest{ método: user-confirmed, sig: user:K } ]
  provenance   : eventos do ledger de A (h, sig_kernel_A)
}
```

### Algoritmo de verificação em B

1. Recalcula `h(φ)` e a obrigação canônica `Γ ⊢ φ` **localmente**, com o seu próprio gerador.
2. Recheca `π` com o **seu** checker. A assinatura do kernel de A é irrelevante para a validade; serve só de proveniência.
3. Checa a testemunha de consistência, se houver.
4. Para cada premissa: se é um PVC, recursão; se é um fato, confere a assinatura do emissor contra as **suas** raízes de confiança e a **sua** janela de validade; se é assumption, mantém como hipótese.
5. Avalia os atestados de fidelidade segundo a **sua** política (B pode exigir confirmação humana e recusar revisão por LLM).
6. Registra no **seu** ledger: `Imported(φ, descarregadas: [p1], pendentes: [p3])`. O status de `φ` em B é condicional às premissas não descarregadas.

Como tudo é endereçado por conteúdo, B guarda em cache `(h(π), h(checker)) → aceito` e nunca recheca o mesmo certificado duas vezes. DAGs grandes compartilham subprovas.

### Desacordo entre agentes vira diff de premissas

Se A envia `Γ_A ⊢ C` e um terceiro agente envia `Γ_D ⊢ ¬C`, os dois sequentes podem ser ambos válidos. O conflito está em `Γ_A ∪ Γ_D` ser inconsistente, e o kernel calcula o subconjunto mínimo inconsistente. Isso unifica o caso multi-agente com o objetivo filosófico original do LOGOS: **localizar o desacordo** em vez de escolher um vencedor. Debate entre agentes deixa de ser retórica e vira uma operação sobre conjuntos de premissas.

### Problemas do protocolo

- **Negação de serviço.** A pode enviar certificados gigantes. B precisa de orçamento por PVC e trata excesso como `UNKNOWN`.
- **Envenenamento por premissas plausíveis.** A pode montar um argumento válido sobre assumptions sutilmente falsas. O protocolo garante que elas aparecem marcadas como assumptions de A; não garante que B vai notá-las. A interface precisa destacar premissas não descarregadas.
- **Obsolescência.** Fatos têm época. Um PVC válido ontem pode depender de fatos que B considera velhos hoje.
- **Privacidade.** A pode não querer revelar `Γ`. Compromissos criptográficos escondem premissas, mas aí B não consegue recheck; só computação verificável resolveria (seção 9), a um custo alto.

### Precedentes

O conceito mais próximo é **proof-carrying data** (Chiesa e Tromer, 2010): em computação distribuída, cada mensagem carrega uma prova de que é consistente com toda a história que a produziu. Na segurança de cadeia de suprimentos, **in-toto** e SLSA fazem algo estruturalmente parecido com atestados assinados por etapa. O PVC é a versão disso para claims lógicos produzidos por agentes, com fidelidade como campo de primeira classe. A contribuição está na combinação e na semântica de importação, não no transporte.

## 8. Semantic fidelity

Uma prova pode estar perfeitamente correta para uma formalização errada. Nenhum checker resolve isso, porque o erro não está dentro do sistema formal. O que a arquitetura pode fazer é garantir três coisas: que a fidelidade seja **avaliada separadamente**, **atestada por alguém identificável**, e **impossível de esconder** atrás do selo de validade.

### O selo tem cinco campos, e nenhum pode ser omitido

```text
Seal {
  validity  : Verified(checker, h(cert)) | Refuted(h(M)) | Unknown
  vacuity   : Consistent(h(M0)) | ConsistencyUnknown
  fidelity  : nível mais fraco entre as fórmulas usadas (premissas + conclusão)
  grounding : nível mais fraco entre os símbolos usados
  open      : premissas não descarregadas (assumptions, fatos de terceiros)
}
```

Regra de tipos: `Seal` só é construído pelo kernel, e a única função que o transforma em texto ou ícone recebe o selo inteiro. **Não existe API para renderizar “verificado” sozinho.** O rótulo-resumo é o mínimo entre as dimensões: o ícone verde só aparece quando todas estão no topo. Isso transforma o princípio “verificado não é verdadeiro” de aviso em restrição do compilador.

Como um claim apareceria:

```text
C7  SEGUE · condicional
    validade    verificado (alethe @ c3…), premissas consistentes
    fidelidade  P1, P2 confirmadas pelo usuário · P3 revisada só por LLM
    grounding   Causa/2 e Pessoal/1 apenas glosados em português
    em aberto   P3 (assumption do agente)
    resumo      válido sobre uma formalização não totalmente confirmada
```

### Hierarquia de evidência de fidelidade

| Nível | Método | O que de fato garante |
| --- | --- | --- |
| F4 | Autoria formal direta | Não há texto de origem: políticas e contratos escritos em lógica por humanos. Não há tradução a errar |
| F3 | Confirmação humana sobre paráfrase determinística, assinada | Uma pessoa identificada reconheceu a leitura mecânica de `φ` como o que quis dizer |
| F2 | Dupla formalização independente com equivalência **provada** | Duas formalizações de produtores diferentes são logicamente equivalentes. A equivalência é uma obrigação checada; a independência é uma suposição |
| F1 | Retrotradução julgada por LLM | Um modelo não achou diferença. Evidência fraca |
| F0 | Nenhuma | — |

O nível F2 merece cuidado. Ele é o único em que parte da fidelidade vira algo mecanicamente checado, o que é atraente. Mas dois LLMs, mesmo de famílias diferentes, compartilham vieses de leitura; concordância entre eles é evidência mais fraca do que parece. Já a **discordância** é informativa e barata: quando as duas formalizações não são equivalentes, o kernel produz um modelo que distingue as duas leituras, e isso mostra ao usuário exatamente onde a frase é ambígua.

### Grounding: fidelidade por símbolo, não só por fórmula

Uma fórmula pode estar fielmente traduzida e mesmo assim usar um predicado cujo significado ninguém fixou. Por isso cada símbolo do vocabulário tem um status de ancoragem:

- **Definido:** tem definição formal em termos de outros símbolos (`Solteiro(x) ↔ Homem(x) ∧ ¬Casado(x)`).
- **Ancorado:** seu valor é dado por um atestador com procedimento conhecido (`Conexoes(db) = n` segundo o `db-monitor`).
- **Glosado:** só tem uma explicação em português (`Pessoal(x)`: “x é pessoal”).

Aqui a diferença entre os dois regimes da seção 1 fica precisa. No regime operacional, quase todos os símbolos são definidos ou ancorados, e a fidelidade é forte. No regime epistêmico, quase todos são glosados, e o melhor que se consegue é F3. **Nenhuma engenharia muda isso; só é possível deixar visível.**

### Fidelidade calibrada empiricamente

Para os níveis F1 e F2, o sistema pode manter uma estimativa medida: periodicamente, uma amostra de fórmulas aceitas por cada método passa por auditoria humana, e a taxa de infidelidade observada (com intervalo de confiança) acompanha o selo. O selo deixa de dizer “revisado por LLM” e passa a dizer “revisado por um método cuja taxa de erro medida é X”. Isso não é garantia, mas é a forma honesta de tratar um componente probabilístico.

## 9. Kernels e provers

Lean não é a melhor escolha em geral. É a melhor para matemática. Para o regime operacional, existe uma rota com TCB muito menor e checker formalmente verificado; para argumentos, certificados de primeira ordem servem melhor. A recomendação é uma arquitetura em camadas com **uma única semântica** (FOL clássica multi-sortida com igualdade como base) e checkers diferentes por fragmento.

### As opções

```text
A  LLM → AIR → SMT → Z3 → "unsat"                               (solver na TCB)
B  LLM → AIR → grounding → SAT → CaDiCaL → LRAT → cake_lpr      (checker verificado)
C  LLM → AIR → SMT → cvc5 → Alethe → Carcara (→ Isabelle)        (checker de SMT)
D  LLM → AIR → FOL → Vampire/E → TSTP → reconstrução em kernel   (ATP + replay)
E  LLM → teorema Lean → elaborador → termo → kernel Lean / nanoda / lean4lean
F  LLM → HOL (embedding de lógicas modais) → kernel HOL / Candle
G  qualquer produtor → tradução → Metamath Zero ou Dedukti → checker único
H  qualquer checker acima rodando num zkVM → recibo sucinto
```

| Opção | Fragmento | O que fica na TCB | Tamanho relativo da TCB | Checker verificado? | Maturidade | Papel no LOGOS |
| --- | --- | --- | --- | --- | --- | --- |
| A | SMT amplo | O solver inteiro | Enorme | Não | Alta | Só protótipo |
| B | Proposicional; domínios finitos via grounding | Checker LRAT + grounder | Mínimo | **Sim** (CakeML, até código de máquina) | Alta | Regime operacional |
| C | Teorias SMT, quantificadores via instanciação | Checker Alethe + semântica das regras | Médio | Não | Média, crescendo | Argumentos de 1ª ordem decidíveis na prática |
| D | FOL completa | Kernel de replay + clausificação | Médio, com lacunas | Não | Média | Reserva para FOL difícil |
| E | Teoria de tipos dependentes | Kernel Lean (cruzável com kernels independentes) | Médio | Parcial (há trabalho de verificar o kernel) | Alta | Matemática |
| F | HOL e lógicas embutidas | Kernel HOL | Pequeno (HOL Light); Candle verificado | Sim, no caso Candle | Média | Lógica modal e deôntica |
| G | Qualquer lógica codificável | Um verificador mínimo | Mínimo e único | MM0 foi desenhado para isso | Baixa | Convergência de longo prazo |
| H | O do checker | Circuito + criptografia + checker | Grande e exótica | — | Baixa | Só privacidade entre agentes |

### Observações que decidem a escolha

**A rota B é subestimada.** Políticas operacionais quase sempre quantificam sobre domínios finitos e conhecidos (usuários, recursos, ações). Com grounding, viram SAT. Aí a prova de `Allowed(a)` termina num certificado LRAT checado por um programa formalmente verificado até o binário. É difícil imaginar uma TCB menor para a parte que autoriza efeitos. O custo é o grounder entrar na TCB, mas ele é código pequeno e puramente sintático.

**Lean resolve o problema certo para matemática e o errado para argumentos.** Formalizar “toda causa do universo é pessoal” em teoria de tipos dependentes é trabalho pesado sem ganho: a lógica necessária é de primeira ordem. Onde Lean entra, o elaborador fica fora da TCB, o kernel fica dentro, e o checker deve também recusar provas que usem axiomas fora de uma lista permitida (o equivalente de checar `#print axioms`). Kernels independentes de Lean, como o nanoda (em Rust) e o lean4lean, permitem checar o mesmo termo duas vezes com implementações diferentes.

**Para lógicas modais e deônticas**, a abordagem do LogiKEy (embedding semântico em HOL) é a mais madura. O kernel do Isabelle é bem maior que o do HOL Light, então, se a prioridade é TCB mínima, a rota F com HOL Light ou Candle é preferível, ao custo de menos automação.

**Raciocínio derrotável também é certificável, em parte.** Na argumentação abstrata, aceitação pela semântica grounded é polinomial: um checker pequeno simplesmente recalcula. Aceitação crédula pela semântica estável tem testemunha (a própria extensão), fácil de checar. Aceitação cética exige mostrar que *nenhuma* extensão exclui o argumento, o que volta a ser um certificado de insatisfatibilidade via codificação SAT. Ou seja, a rota B serve também para o backend ASPIC+ do roadmap.

**Contraexemplos não precisam de nada disso.** Em todas as rotas, `REFUTED` é checado por um avaliador de modelos finitos com algumas centenas de linhas. É o componente mais simples e mais confiável do sistema.

**O ponto fraco que sobra é a multiplicidade de checkers.** Cada rota acrescenta um checker e um encoder à TCB. A rota G existe para eliminar isso: o gerador de obrigações emite a obrigação direto na linguagem de um framework lógico, e todos os produtores traduzem suas provas para lá. A tradução de provas fica fora da TCB, porque o checker confere contra a obrigação hasheada. É a arquitetura mais limpa e a menos madura: o esforço de tradução de provas para Metamath Zero ou Dedukti é um projeto de pesquisa em si.

### Recomendação

| Camada | Uso | Produtor | Certificado | Checker |
| --- | --- | --- | --- | --- |
| 0 | Contraexemplos e consistência, em todo lugar | Z3, model finders | Modelo finito | Avaliador próprio em Rust |
| 1 | Autorização de ações; aceitação cética em argumentação | Grounder + CaDiCaL/Kissat | LRAT | cake\_lpr |
| 2 | Argumentos de primeira ordem | cvc5 (Z3 como acelerador) | Alethe | Carcara; Isabelle como segundo checker |
| 3 | Matemática | LLM + táticas Lean | Termo Lean | Kernel Lean + nanoda, axiomas restritos |
| 4 | Modal e deôntica | LLM + embedding em HOL | Prova HOL | HOL Light / Candle |
| Longo prazo | Tudo | Todos | Tradução | Metamath Zero ou Dedukti |

## 10. Ameaças e failure modes

A coluna “Natureza” separa o que é limite da ideia (fundamental) do que é resolvível com engenharia. As fundamentais são as que o paper precisa declarar como limitações, não como trabalho futuro.

| # | Ameaça | Consequência | Mitigação | Natureza |
| --- | --- | --- | --- | --- |
| 1 | Formalização infiel | Prova correta da coisa errada | Selo em cinco campos; F3/F2; auditoria amostral | **Fundamental** |
| 2 | Atestador comprometido ou errado | Prova sobre fatos falsos | Múltiplos atestadores; política de confiança por tópico; atestados com época | **Fundamental** (a confiança só se desloca) |
| 3 | Política errada | Ação “permitida” que não deveria ser | Análise de políticas por SMT (diferença entre versões, ações que passam a ser permitidas); revisão humana | **Fundamental** |
| 4 | Premissas inconsistentes | Qualquer coisa “segue” | Testemunha de consistência obrigatória para autorizar ações | Engenharia (exceto quando a consistência é indecidível) |
| 5 | Agente escreve a própria obrigação | Prova trivial de uma obrigação vazia | Obrigações geradas só pelo kernel | Engenharia |
| 6 | Bug em encoder ou grounder | `Verified` falso | Código mínimo, round-trip, fuzzing; longo prazo: framework único | Engenharia |
| 7 | Bug de correção num checker | `Verified` falso, possivelmente em massa | Checkers verificados onde existem; dois checkers independentes; revogação por hash de checker | Engenharia |
| 8 | Chave do kernel comprometida | Eventos e tokens forjados | HSM ou TEE; rotação de chaves; tokens de vida curta | Engenharia |
| 9 | Kernel mostra histórias diferentes a partes diferentes | Ledger “imutável” bifurcado | Publicar as cabeças do ledger num log de transparência com testemunhas externas, como no Certificate Transparency | Engenharia |
| 10 | TOCTOU | Prova vale para um estado que já mudou | Pré-condições versionadas e execução condicional; `unprotected_pre` explícito | Parcialmente **fundamental** (fatos sem versão) |
| 11 | Replay ou roubo de token | Ação repetida ou desviada | Nonce, audiência, validade curta, ligação a `h(args)` | Engenharia |
| 12 | Decomposição de ação proibida em permitidas | Política contornada | Políticas sobre trajetórias com o ledger como premissa | Engenharia, mas exige políticas melhores |
| 13 | Injeção de prompt escolhendo entre ações permitidas | Ação conforme a política e contrária à intenção do usuário | Proveniência dos argumentos como premissa (argumento derivado de dado não confiável é marcado), no estilo CaMeL; autorização do usuário sobre o termo formal | **Fundamental**: conformidade não é intenção |
| 14 | Agente enfraquece claims até virarem triviais | Tudo verificado, nada útil | Medir informatividade; registrar e exibir eventos de enfraquecimento | Engenharia e avaliação |
| 15 | Excesso de `UNKNOWN` | Usuários contornam o sistema | Override humano assinado e registrado; métrica de taxa de `UNKNOWN` | Engenharia; é o modo de falha mais provável na prática |
| 16 | Confiança excessiva no selo | “Verificado” lido como “verdadeiro” | Renderização obrigatória do selo inteiro; estudo com usuários | **Fundamental** (só mitigável) |
| 17 | Certificados gigantes | Negação de serviço | Orçamento por obrigação; excesso vira `UNKNOWN` | Engenharia |
| 18 | Resultado de check depende da máquina (timeout) | Ledgers divergentes | Registrar limites no evento; timeout nunca é rejeição nem aceite | Engenharia |

### A falha fundamental, dita explicitamente

Se a pergunta é “esta arquitetura garante que as conclusões e ações de um agente são corretas?”, a resposta é **não, e nenhuma arquitetura desse tipo garante**. As ameaças 1, 2, 3, 13 e 16 mostram por quê: a correção depende de formalização, fatos, políticas e intenção, e nada disso é objeto de prova.

O que a arquitetura garante é mais estreito e ainda muito valioso: **nenhuma inferência inválida e nenhuma ação não autorizada passam sem que o sistema saiba exatamente de qual suposição não verificada elas dependem.** Isso troca “confie no agente” por “confie nestas premissas, nestes atestadores e nesta política, que estão listados e assinados”. A ideia sobrevive se for vendida assim. Se for vendida como correção, é falsa.

## 11. Contribuição científica possível

**Atualização:** a análise abaixo foi superada pela seção 13. A tese central e o primeiro paper passaram a ser *integridade de justificativa contra integridade de influência*, que diferencia o LOGOS do CaMeL e do FIDES.

Cada candidata abaixo foi julgada contra a pergunta: um revisor que conhece a área diria “isso já existe”?

| Candidata | O que já existe | O que sobraria de novo | Veredicto |
| --- | --- | --- | --- |
| Proof-carrying agent protocol | Proof-carrying code (1996); proof-carrying authorization (1999); proof-carrying data (2010) | O produtor é um modelo estocástico que também formaliza a partir de linguagem natural; loop de refutação interativo | **Fraca** como ideia isolada. O protocolo é PCC aplicado; o revisor vai dizer isso |
| Verificação intercalada | Logic-LM, LINC; loops com kernel no AlphaProof e em copilotos de Lean | Medição fora da matemática, com a métrica de profundidade de propagação de erro | **Moderada**, só como resultado empírico |
| Ledger epistêmico | TMS/ATMS; W3C PROV; in-toto; logs de transparência | Invalidação por dependência (ATMS) + atestados assinados + dependências extraídas do certificado + revogação de checker | **Moderada**; precisa de formalização com propriedades provadas para não parecer engenharia |
| Proof-gated tool execution | CaMeL; FIDES e outros trabalhos de information-flow control para agentes; Cedar, OPA | Políticas sobre trajetórias usando o ledger; TOCTOU no token; certificados rechecáveis | **Fraca a moderada**; CaMeL é um vizinho muito próximo, e para políticas decidíveis o limite L2 se aplica |
| Fidelity-aware verification | Medições do gap entre compilação e fidelidade (Beyond Compilation, 2026); retrotradução; hermenêutica computacional | Fidelidade como **atestado tipado e obrigatório** no selo; grounding por símbolo; equivalência de dupla formalização como obrigação checada; calibração empírica por método | **A mais forte.** A literatura mede o gap; não encontrei uma arquitetura de verificação que o torne parte do tipo do resultado |
| Portable verified claims | Proof-carrying data; lógicas de “says” (ABLP, DKAL, SecPAL); atestados de supply chain | Semântica de importação com descarregamento parcial de premissas; desacordo entre agentes como subconjunto inconsistente mínimo | **Moderada**; interessante como aplicação, fraca como teoria nova |

### A contribuição que eu defenderia

Nenhuma das peças sozinha sustenta um paper forte. O que sustenta é uma tese única com três partes que se apoiam:

> **Agentes mediados por prova com TCB sem LLM e sem solver, e com fidelidade como dimensão tipada do resultado.**

1. **Modelo formal.** A máquina de estados do kernel (seções 3, 4 e 6) mecanizada num assistente de provas, com dois teoremas: *mediação completa* (todo efeito executado tem uma cadeia válida até uma obrigação checada) e *honestidade do selo* (nenhum selo renderizado afirma um nível de fidelidade ou validade maior do que o ledger sustenta). Isso transforma a arquitetura de diagrama em afirmação verificável.
2. **Medição da TCB.** Linhas de código confiável e proporção verificada, comparando A (solver confiável), a arquitetura em camadas e o custo em latência e tamanho de certificado. É um resultado quantitativo que ninguém publicou para agentes.
3. **Avaliação empírica nos dois regimes.**
   - Operacional: AgentDojo (o benchmark de injeção de prompt usado pelo CaMeL), medindo taxa de sucesso de ataque e utilidade, com CaMeL como baseline. Hipótese: igual segurança com políticas mais expressivas, ou maior segurança contra ataques de decomposição graças às políticas de trajetória.
   - Epistêmico: o benchmark de argumentos da especificação, medindo profundidade de propagação de erro (intercalado contra pós-hoc) e, num estudo com usuários, se o selo em cinco campos reduz a aceitação de conclusões válidas sobre formalizações infiéis.

O resultado mais publicável, e o mais arriscado, é o estudo com usuários: ele testa diretamente se “verificado” engana menos quando a fidelidade é obrigatória no selo. Um resultado negativo também seria interessante.

### Onde publicar

As três partes puxam para comunidades diferentes: o modelo formal para métodos formais (CAV, ITP), a mediação de ações para segurança (USENIX Security, CCS, SaTML, onde o CaMeL saiu), o estudo de fidelidade para IA e IHC. Um primeiro paper deve escolher uma. A recomendação é **segurança**, com o regime operacional: é onde as garantias são fortes, o baseline é claro e a avaliação tem benchmark público. O regime epistêmico vira o segundo paper, apoiado na mesma infraestrutura.

## 12. Arquitetura final recomendada

```text
 ════════════════════ PROCESSOS NÃO CONFIÁVEIS ════════════════════════════
  agentes LLM · formalizador · planner · TUI · MCP · retrieval
  prover farm: Z3 · cvc5 · Vampire · CaDiCaL · Lean (elaborador) · model finders
        │ PROPOSE / CERTIFY / ACTION_INTENT          (sem credenciais, sem chaves)
 ═══════╪══════════════════════════════════════════════════════════════
        ▼
  logos-kernel   (processo isolado; Rust sem unsafe; invariantes verificados)
  ├─ obligations   AIR canônica → sequente → hash      (única fonte de obrigações)
  ├─ checkers      modelo finito · LRAT(cake_lpr) · Alethe · Lean(nanoda) · HOL
  ├─ seal          constrói e renderiza o selo de cinco campos; paráfrase determinística
  ├─ ledger        hash chain assinada; cabeças publicadas em log de transparência
  └─ minter        tokens Biscuit-like ligados a h(args), pre, r_v, exp
        │ tokens                         ▲ fatos assinados
        ▼                                │
  logos-gateway (TCB)            ATESTADORES (TCB de fidelidade)
  adaptadores finos por ferramenta;      ferramentas · monitores · fontes
  únicos donos das credenciais;           usuário (passkey) · admin (políticas)
  execução condicional a versões
        │
        ▼
  mundo  ─── efeito observado ───► atestado ───► ledger
```

### O que muda em relação à especificação atual

| Hoje na especificação | Proposto | Motivo |
| --- | --- | --- |
| `logos-check` roda Z3/Vampire e confia no veredicto | `logos-kernel` só checa certificados; solvers viram produtores externos | Tirar o solver da TCB; permitir recheck e revogação de checker |
| Dependências declaradas na AIR (`from`) | Dependências extraídas do certificado | O agente não pode inflar nem esconder dependências |
| Status em vetor, calculado pelo ledger | Selo de cinco campos construído só pelo kernel, sem API para renderizar validade isolada | Fidelidade vira restrição de tipo, não aviso |
| Proveniência como rótulo (`USER`, `MODEL`…) | Fatos e políticas são objetos assinados; o LLM não tem chave para assinar nada | Proveniência verificável, não declarada |
| Ledger SQLite | SQLite como armazenamento, mas com hash chain, assinatura e publicação de cabeças | Imutabilidade verificável por terceiros |
| Servidor MCP expõe `logos_check` | MCP expõe `propose`, `certify`, `action_intent`; ferramentas reais só atrás do gateway | Mediação completa: o agente não tem outro caminho |
| Vocabulário com glosa | Vocabulário com status de grounding por símbolo | Fidelidade por símbolo, não só por fórmula |
| Lógica implícita | Identificador de lógica versionado em toda obrigação | Ledgers e PVCs interpretáveis ao longo do tempo |

### Princípios que eu preservaria mesmo que tudo o mais mude

1. O agente propõe, cita e prova; nunca escreve obrigações, fatos ou políticas.
2. O checker consome exatamente os bytes que o ledger hasheia.
3. Todo veredicto que autoriza algo passa por um checker; solvers só aceleram a busca.
4. Refutações são testemunhas; provas são certificados; `UNKNOWN` não autoriza nada.
5. Nenhum efeito é especulativo.
6. O termo de uma ação vem dos bytes da chamada, nunca da descrição do agente.
7. Nenhum selo de validade é renderizável sem fidelidade, consistência e premissas em aberto.
8. Confiança em outro agente é premissa explícita, nunca pressuposto do protocolo.

### Ordem de construção de longo prazo

A ordem segue onde as garantias são mais fortes, não onde a demo é mais bonita:

1. **Kernel mínimo:** obrigações, avaliador de modelos, LRAT, ledger encadeado. Modelo formal da máquina de estados em paralelo.
2. **Gateway e tokens:** regime operacional completo, avaliação no AgentDojo contra CaMeL. Primeiro paper.
3. **Alethe e o selo de cinco campos:** regime epistêmico, benchmark de argumentos, estudo com usuários. Segundo paper.
4. **PVCs entre agentes.**
5. **Lean e HOL** para matemática e lógicas modais.
6. **Framework lógico único** para colapsar os checkers num só.

## 13. Diferenciação: da influência à justificativa

A tese central do LOGOS passa a ser esta: **a segurança de um agente deve ser definida pela justificativa checada de cada ação, e não pela influência que dados não confiáveis tiveram sobre ela.** É isso que separa o projeto dos dois vizinhos mais próximos, o CaMeL e o FIDES.

### O que os vizinhos fazem

| Sistema | Como trata dados não confiáveis | Garantia | Limite |
| --- | --- | --- | --- |
| CaMeL (DeepMind, 2025) | O LLM que planeja nunca lê dados não confiáveis; um LLM em quarentena extrai valores; capabilities por valor; políticas antes de cada chamada | Integridade do fluxo de controle: o plano é fixado antes de ver os dados | O plano não pode depender do que o planejador não lê (“The P-LLM cannot write a plan based on data it can't read”). No AgentDojo, cerca de 77% das tarefas com garantia, contra 84% sem defesa |
| FIDES (Microsoft, 2025) | Rótulos por variável; ocultação seletiva para evitar label creep; LLM em quarentena com saída de tipo restrito (booleano, enum) | Não-interferência para integridade, com endossos explícitos | O endosso é por **capacidade**: um booleano carrega só 1 bit de influência não confiável. Os próprios autores avisam que essas políticas “effectively endorse untrusted values” |
| LOGOS (proposto) | Conteúdo não confiável entra no ledger como `fonte diz φ`; toda ação exige `Pol ∪ Γ ⊢ Allowed(a)` checado; a política raciocina sobre fonte, **conteúdo** da premissa e classe da ação | Integridade de justificativa (abaixo) | Não impede a escolha entre ações justificadas; depende de políticas completas |

Correção em relação a uma versão anterior desta análise: o FIDES **já mitiga** label creep (oculta dados em variáveis em vez de contaminar o contexto inteiro) e **já permite** decisões sobre dados não confiáveis via saídas de baixa capacidade. A diferença do LOGOS não está em permitir isso, e sim em **como** o endosso é decidido.

### Endosso semântico contra endosso por capacidade

O problema do endosso por capacidade: 1 bit pode ser o bit decisivo. Se o agente pergunta ao LLM em quarentena “o e-mail pede para apagar o banco?” e a resposta é um booleano, a injeção controla exatamente o bit que importa. Capacidade baixa limita exfiltração e payloads, mas não limita **dano**.

No LOGOS, a premissa extraída é `email(e) diz PedidoApagar(db)`, e a política fala sobre o **conteúdo e a origem** dela:

```text
# premissas vindas de e-mail externo podem justificar ações de baixo impacto
email(e) diz Pedido(x) ∧ Externo(e) ∧ Classe(x) = responder   → Allowed(x)
email(e) diz Pedido(x) ∧ Externo(e) ∧ Classe(x) = criar_tarefa → Allowed(x)

# ações destrutivas exigem premissa assinada pelo usuário, nunca "diz" de terceiros
Classe(x) = destrutiva → ( Allowed(x) ↔ ∃u. Usuario(u) ∧ u assina Autoriza(x) )
```

Com isso, uma injeção pode no máximo fazer o agente executar ações que a política já aceitava justificar a partir daquela fonte. E o certificado mostra exatamente qual premissa não confiável sustentou cada ação.

### A propriedade, enunciada

> **Integridade de justificativa.** Para toda ação executada `a`, existe um certificado aceito para `Pol_v ∪ Γ ⊢ Allowed(a)`, em que `Γ` é o conjunto de premissas efetivamente usadas pelo certificado (não tudo o que o agente leu), e cada `p ∈ Γ` tem uma origem cuja confiança satisfaz o requisito que `Pol_v` impõe para a classe de `a`.

Comparação com não-interferência: não-interferência pergunta se o dado não confiável **causou** a ação, e por isso precisa sobre-aproximar a influência. Integridade de justificativa pergunta se a ação é **sustentada** por premissas aceitáveis, e é indiferente ao que mais o agente leu.

### O que a tese não resolve

- **Escolha entre ações justificadas.** Se várias ações são justificáveis, a injeção pode escolher qual. A segurança fica igual à completude da política. Isso é um limite, não um bug.
- **Extração manipulada.** O LLM em quarentena pode extrair um `φ` errado. O rótulo de origem continua sendo “e-mail”, então o pior caso fica dentro do que a política permite para e-mails; mas, dentro desse espaço, a extração é a superfície de ataque.
- **Custo de autoria.** Políticas sobre conteúdo são mais ricas e mais caras de escrever que rótulos. A pergunta prática é se um conjunto pequeno de classes de ação e de fontes cobre os casos reais.

### Perguntas de pesquisa e experimento

1. **Quando integridade de justificativa basta?** Caracterizar formalmente as condições (completude da política em relação a uma especificação de intenção) sob as quais ela implica a propriedade que o usuário quer.
2. **Ela recupera utilidade?** AgentDojo com CaMeL, FIDES e agente sem defesa como baselines, separando as tarefas em que a decisão depende de dado não confiável. Métricas: taxa de sucesso de ataque, utilidade, latência.
3. **Uma nova classe de ataque.** Construir uma suíte de ataques de **bit decisivo**: injeções que só precisam controlar uma resposta booleana ou enum para causar dano. Se ela quebrar o endosso por capacidade e não quebrar o endosso semântico, isso é um resultado publicável por si só.
4. **Auditabilidade.** Num incidente, quanto tempo um analista leva para achar a premissa que causou a ação, com e sem o certificado?

Essa seção substitui a recomendação de primeiro paper da seção 11: o primeiro paper deixa de ser “proof-gated actions contra CaMeL” e passa a ser **integridade de justificativa contra integridade de influência**.

## 14. Outras apostas

Três ideias que complementam a tese da seção 13. Nenhuma sustenta o projeto sozinha; cada uma tem um papel definido.

| Aposta | Ideia | O que já existe | Papel no LOGOS |
| --- | --- | --- | --- |
| Reversão de ações por dependência | Quando uma premissa se revela falsa, o ledger lista todas as conclusões **e ações executadas** que dependiam dela e propõe como compensar cada uma | Invalidação por dependência em TMS/ATMS; não encontrei aplicação a ações de agentes | Diferencial de produto e resultado secundário do primeiro paper |
| Debate checkável | Debatedores se comprometem com sequentes; refutações são checadas por máquina; o humano julga só as premissas folha, nunca inferências | Debate como supervisão escalável em segurança de IA; **literatura de debate com verificação formal ainda não checada** | Segundo paper, no regime epistêmico |
| Desambiguação por cenários distintivos | Em vez de confirmar uma paráfrase, o usuário vê um cenário concreto em que duas leituras possíveis divergem e diz qual quis | Já existe: TiCoder (2022) para código; Monty (2026) para especificações JML | Componente de fidelidade, não contribuição principal |

### Reversão de ações, em mais detalhe

Todo evento `Executed` no ledger aponta para o certificado que o autorizou, e o certificado aponta para as premissas que usou. Se a premissa `p` é retratada, uma busca no DAG devolve:

```text
$ logos retract F37 --reason "backup reportado não existia"

conclusões invalidadas:  C12, C19
ações executadas que dependiam de F37:
  E902  db.drop(orders_v1)        12:04   compensação: restore(orders_v1, snapshot 11:58)
  E915  notify(team, "migrado")    12:06   compensação: notify(team, correção)
ações com rota independente (não afetadas): E917
```

Compensações são propostas a partir dos contratos das ferramentas e passam pelo mesmo gateway. Em segurança, isso funciona como resposta a incidentes guiada por dependência: em vez de perguntar “o que o agente fez depois das 12h?”, pergunta-se “o que o agente fez **porque** acreditou em F37?”.

### Debate checkável, em mais detalhe

O gargalo das propostas de debate é o juiz humano, que precisa avaliar argumentos longos. No LOGOS, cada debatedor publica seus argumentos como PVCs (seção 7). Se o debatedor B quer atacar A, ele precisa apresentar um contraexemplo checado, um subconjunto inconsistente mínimo, ou um ataque a uma premissa específica. As inferências nunca chegam ao juiz, porque o kernel já decidiu. O juiz recebe só uma lista curta: “o desacordo está nas premissas P4 e P19; qual você aceita?”. Isso une o objetivo filosófico original (localizar o desacordo) a uma pergunta aberta de segurança de IA (supervisionar sistemas mais capazes que o supervisor).

### Como as peças se encaixam

1. **Primeiro paper (segurança):** integridade de justificativa contra integridade de influência, com a suíte de ataques de bit decisivo e a reversão de ações como resultado secundário.
2. **Segundo paper (IA e supervisão):** debate checkável no regime epistêmico, com o selo de cinco campos e a desambiguação por cenários como componentes de fidelidade.
3. **Produto:** os dois regimes no mesmo binário, com a reversão de ações como o recurso mais fácil de explicar a quem opera agentes em produção.

## Fontes

- [Defeating Prompt Injections by Design (CaMeL)](https://arxiv.org/abs/2503.18813), Debenedetti et al., 2025 · [AgentDojo](https://github.com/ethz-spylab/agentdojo)
- [Carcara: proof checker e elaborador para Alethe](https://team.inria.fr/veridis/files/2023/05/carcara.pdf) · [Especificação do Alethe](https://verit.loria.fr/documentation/alethe-spec.pdf) · [Saída Alethe do cvc5](https://cvc5.github.io/docs/latest/proofs/output_alethe.html) · [Reconstrução de provas SMT em Isabelle/HOL (ITP 2025)](https://drops.dagstuhl.de/storage/00lipics/lipics-vol352-itp2025/LIPIcs.ITP.2025.26/LIPIcs.ITP.2025.26.pdf)
- [Beyond Compilation: Evaluating Faithful Natural-Language-to-Lean Statement Formalization](https://arxiv.org/abs/2606.31002), 2026
- Checkers e kernels: [cake\_lpr](https://github.com/tanyongkiam/cake_lpr) · [nanoda\_lib](https://github.com/ammkrn/nanoda_lib) · [lean4lean](https://github.com/digama0/lean4lean) · [Metamath Zero](https://github.com/digama0/mm0) · [Lambdapi / Dedukti](https://github.com/Deducteam/lambdapi)
- Autorização e atestados: [Cedar](https://www.cedarpolicy.com/) · [Biscuit](https://www.biscuitsec.org/) · [in-toto](https://in-toto.io/)
- Verificação de Rust: [Verus](https://github.com/verus-lang/verus)
- Referências clássicas sem link: Milner, arquitetura LCF (anos 1970); Anderson, *Computer Security Technology Planning Study* (1972, monitor de referência); Necula e Lee, *Safe Kernel Extensions Without Run-Time Checking* (OSDI 1996); Appel e Felten, *Proof-Carrying Authentication* (CCS 1999); Bauer et al., sistema Grey (2005); Abadi, Burrows, Lampson e Plotkin, *A Calculus for Access Control in Distributed Systems* (TOPLAS 1993); Chiesa e Tromer, *Proof-Carrying Data* (ICS 2010); Birgisson et al., *Macaroons* (NDSS 2014); de Kleer, *An Assumption-based TMS* (1986); Winterer, Zhang e Su, *Validating SMT Solvers via Semantic Fusion* (PLDI 2020); Tan, Heule e Myreen, *cake\_lpr: Verified Propagation Redundancy Checking in CakeML* (TACAS 2021).

* [Securing AI Agents with Information-Flow Control (FIDES)](https://arxiv.org/abs/2505.23643), Costa, Köpf et al., 2025 · [Agent Security with FIDES](https://learn.microsoft.com/en-us/agent-framework/agents/security)
* [Monty: Faithful Autoformalization of Natural Language Assertions](https://arxiv.org/html/2607.13303) · [Intent Formalization: A Grand Challenge (cita o TiCoder)](https://arxiv.org/html/2603.17150v1)
* Lahiri et al., *Interactive Code Generation via Test-Driven User-Intent Formalization* (TiCoder, 2022)
