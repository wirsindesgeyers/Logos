# ADR 0040 — Projeção viva autenticada e correções do transporte

- Estado: aceito
- Data: 2026-10-19
- Escopo: correções da revisão do kernel, Etapas3.3/5.2/5.3

## Contexto

A revisão em `docs/reviews/2026-10-19-kernel.md` reproduziu três P2:
Subscribe verifica o prazo somente entre páginas; Status/PROPOSE reconstroem
todo o índice por requisição; diretórios rejeitados ficam na área privada.
As dependências necessárias estão implementadas. Corrigir antes de iniciar5.4
não acrescenta handlers, formatos ou etapas de produto.

## Decisão

Preservar `Ledger::index`/`LedgerIndex::open` para reconstrução integral,
comparação/publicação explícita de index.sqlite3 conforme ADR0022. Acrescentar
`Ledger::live_index`, usado pelos handlers Status/PROPOSE: primeira abertura
autentica e projeta a cadeia inteira, confere a cabeça contra o writer e
mantém a projeção SQLite privada na vida daquele Ledger. Nenhum snapshot
persistido é importado como autoridade.
A varredura inicial inclui todas as linhas, inclusive sequências negativas
inválidas que devem ser recusadas. Só a varredura de sufixo exclui um prefixo
já autenticado; `WHERE seq>=0` não substitui a validação do replay integral.

Depois de appends, conferir somente o sufixo após a cabeça já autenticada,
incluindo sequência, prev, época, assinatura e cabeça final exata do writer.
Aplicar a mesma projeção em transação privada; falha não publica prefixo
parcial nem devolve snapshot antigo. Head, metadados e tabelas avançam juntos.
Handle Poisoned não fornece projeção. Reabrir o escritor perde o estado
privado e exige replay integral. A API de leitura empresta o Ledger e impede
append/Drop enquanto um snapshot existir. Empréstimos internos usam operações
falíveis de RefCell; sem panic no runtime. A projeção viva não oferece um
relatório fictício de reconstrução/publicação a cada consulta.

Não publicar nem comparar o cache em disco por requisição da API viva.
Corrupção desse cache continua irrelevante à autorização; a ferramenta de
rebuild mantém detecção/reparo explícitos. O snapshot já autenticado pode
continuar sendo consultado se o disco for adulterado; abertura subsequente
e leitura de novos eventos continuam conferidas. Sem checkpoint persistido,
âncora anti-rollback, dependência nova ou mudança do schema ledger-index/v1.

Subscribe verifica prazo antes de cada evento e usa o menor prazo entre
frame e assinatura durante cada escrita. Não iniciar outro evento após o
prazo. Buscar um evento por página limita replay síncrono/memória antes da
próxima checagem, sem materializar256 eventos grandes sob o lock do writer.
O cursor final também respeita o prazo restante; depois de expirado,
tentar enviá-lo sem bloqueio e fechar em erro se o socket estiver cheio.
Expiração durante um frame encerra a conexão com erro, sem cursor que reconheça
um evento parcial. O cliente retoma do último evento completo observado.
O prazo não interrompe sincronamente handlers/replay;
nenhuma escrita de evento recebe uma renovação do prazo da assinatura.

Diretórios capturados inválidos são devolvidos intactos ao diretório do agente
por rename ancorado/NOREPLACE. Preferir o nome original; se já houver um upload
novo, preservar esse upload e devolver sob nome `rejected-*` com nonce de128bits
lido de `/dev/urandom`, sem nomes previsíveis que o agente possa ocupar antes.
Falha de entropia é I/O explícito; nenhuma dependência Rust nova.
Tentativas de colisão são limitadas, sem exclusão recursiva, seguir symlinks ou
sobrescrever uma entrada do agente. Falhas de devolução/sync são erros explícitos
com causa de validação e limpeza; não declarar sucesso de limpeza em falha de I/O.
Outros tipos continuam removidos somente pelo nome capturado privado.

## Verificação exigida

Regressões com socket lento/página pronta, deadline durante frame bloqueado,
cursor entre frames e liberação de lock; diretórios vazios/com filhos e colisão
com upload novo; equivalência da projeção viva com replay completo após appends,
época/retratação/revogação e falhas; cache adulterado, Poisoned e reabertura.
Demonstrar que regressões detectam os defeitos/mutações, medir Status e PROPOSE
pelo socket, rodar `ci/verify.sh --selftest` antes do commit. CI remota permanece
pendência separada. Sem novo parser/decoder ou alteração de formato/vetor.
