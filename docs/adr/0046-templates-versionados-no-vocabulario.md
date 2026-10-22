# ADR0046 — templates versionados no vocabulário

Estado: aceito para implementação do pré-requisito da9.1,21/out/2026.

A autorização de prosseguir resolve o bloqueio registrado na pré-checagem.
Adicionar `render_pt` ao próprio Vocabulary, como lista de templates alinhada
à ordem dos símbolos, sem modificar SymbolDecl nem reinterpretar gloss_pt.
`vocabulary/v2 = [2, version, sorts, symbols, render_pt]`; v1 permanece com
seus quatro campos, bytes e hashes históricos. v2 tem exatamente um template
por símbolo; ausência/erro é rejeição, não fallback. A versão de conteúdo
continua distinta da versão de formato. Domínio h(V) existente é preservado:
o tag2 no conteúdo canônico distingue o formato e todos os templates entram
no hash. Trocar formato ou template de símbolo anterior é incompatível.

Templates: Text NFC/Unicode16,1–4096 bytes cada, agregado≤1MiB. Placeholders
numéricos canônicos `{0}`…`{n-1}`, cada um exatamente uma vez e nessa ordem.
Deve existir texto literal não branco. Não admitir chaves literais, controles e os pontos reservados de exibição
listados em spec/render-templates.md (incluindo marcadores bidirecionais/zero-width),
nem delimitadores ⟦ e ⟧.
Essas regras permitem expansão e parsing determinísticos posteriores; não
provam que o texto natural corresponde ao significado, tarefa da revisão humana.
Nenhuma glosa serve implicitamente como template. Templates nularios não
contêm placeholders. Conteúdo não é executado como código ou instrução LLM.

Migrar codecs/validação e round-trip documental Rust/Python em conjunto:
cláusula opcional `render_pt ["template0", ...]` no corpo vocab opta pelo v2;
ausência preserva v1. Formato de superfície atualizado para air-text/v2,
com aceitação integral da gramática v1. Impressão nunca descarta templates.
Produtores/modelos continuam consumindo os mesmos sorts/símbolos; validar o
novo envelope antes de usá-lo e manter h(V) dos bytes completos.

Introduzir pré-requisito9.1a no roadmap, preservando todos os critérios9.1.
Este trabalho só fornece dados/templates versionados: não entrega render_pt,
parse_pt, autorização9.2 ou revisão humana. Nova decisão da gramática PT e
hash de paráfrase pertence à9.1 após essa base estar demonstrada.

Limite explícito: templates de símbolos diferentes podem coincidir. A9.1
precisa preservar identidade dos símbolos/construtores na gramática; não
inferir símbolo apenas pelo texto da frase nem atribuir injetividade a essa
validação de templates. O round-trip/injetividade continua contrato da9.1.

Verificação: corrigir a corrida da fixture Python que lia PID em arquivo
sob timeout100ms, capturando o Process real e sincronizando criação antes
de cancelar, com assert SIGKILL/reap. Na fixture binária Rust, aguardar120s
finitos de arranque debug (hash/registro de executável), preservando todos
os prazos/asserts do protocolo/checker. Nenhuma mudança de produção nessas
correções; falhas e repetições registradas no plano, sem contar parciais como sucesso.
