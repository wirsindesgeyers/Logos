# ADR 0009 — Vocabulário versionado e grounding

- Status: aceito
- Data: 2026-10-05
- Etapa: 1.2

## Contexto

A Etapa 1.1 (ADR 0007) estabeleceu a sintaxe abstrata plana FOL-ms-eq/v1 (`air/v1`),
na qual sorts e símbolos possuem identidades estruturais canônicas. Para que fórmulas
e termos possam expressar contratos, proposições e obrigações verificáveis nas etapas
subsequentes, é imperativo fixar o universo do discurso: quais sorts e símbolos existem,
como são interpretados seus domínios e de onde provém a ancoragem semântica (*grounding*)
de cada símbolo.

Vocabulários precisam de identidade imutável e classificação explícita de
mudanças. Esta etapa verifica dependências acíclicas, não consistência lógica,
fidelidade ou segurança de migrações.

## Decisão

1. **Versão como `u64`**: o vocabulário carrega uma versão inteira unsigned de 64 bits.
   O valor inicial zero é admitido. Operações de extensão realizam incremento verificado
   via `checked_add(1)`, prevenindo overflow (`VersionOverflow`).
2. **Identidade nominal e espaço de nomes sem sobrecarga**:
   - Sorts são identificados pelo nome (`Text` canônico NFC).
   - Símbolos são identificados por sua assinatura integral [air/v1](../../spec/air.md) (`Symbol`).
     Não há campo redundante de nome em `SymbolDecl`; o nome provém exclusivamente da
     assinatura.
   - Não se admite sobrecarga (*overload*) de nomes de símbolos: cada nome de símbolo
     é único em todo o vocabulário, mesmo com aridades ou categorias distintas.
   - Sorts e símbolos habitam namespaces separados.
3. **Três variantes de grounding**:
   - `Defined(Formula)`: símbolo derivado formalmente através de uma fórmula.
   - `Anchored { attestor: AttestorId, procedure: Text }`: símbolo primitivo ancorado em
     procedimento empírico ou institucional executado por autoridade externa.
   - `Glossed(Text)`: símbolo primitivo fundamentado informalmente em linguagem natural
     (português normativo em UTF-8 NFC).
4. **Corpos definidores e ausência de auto-referência**:
   - `Defined` armazena **exclusivamente o corpo da fórmula definidora**, e não a equivalência
     completa `P(x) ↔ corpo`.
   - Para predicados com parâmetros, variáveis De Bruijn livres representam os argumentos
     formais da definição; convenção de substituição e tipagem não são verificadas aqui.
     `Formula` também é representável para Const/Func, sem fixar sua expansão nesta etapa.
   - A fórmula definidora NÃO PODE referenciar o próprio símbolo que está sendo definido
     (auto-referência direta é ciclo imediato).
   - Toda referência a sorts e símbolos no corpo definidor deve coincidir estritamente com
     declarações presentes no vocabulário.
5. **Grafo de dependências e ordenação topológica de Kahn**:
   - Símbolos com grounding `Defined` formam vértices em um grafo direcionado de dependências.
     Bases (`Anchored` e `Glossed`) são folhas no grafo.
   - As dependências são extraídas dos predicados e dos termos embutidos em igualdades ou
     argumentos.
   - O grafo DEVE ser estritamente acíclico (DAG). A validação executa o algoritmo iterativo
     de Kahn (sem recursão em stack) para produzir `definition_order` (*dependencies-first*).
   - Qualquer ciclo direto ou indireto é rejeitado com `VocabularyError::CircularDefinition`.
6. **Domínios de sort e autoridades opacas**:
   - `SortDomain::Open`: domínio aberto.
   - `SortDomain::Enumerated`: domínio fechado enumerando constantes declaradas do tipo
     `Const`, aridade zero e com resultado coincidente com o sort, sem duplicatas.
   - Enumerações carregam `DomainSource`: `Fact(FactRef)` ou `Policy(PolicyRef)`.
   - `FactRef` e `PolicyRef` são digests SHA-256 opacos de 32 bytes (`bytes32`), não resolvidos
     nem consultados nesta etapa.
7. **Compatibilidade direcional estrutural**:
   - A compatibilidade de um vocabulário candidato com relação a um anterior
     (`candidato.compatibility_with(anterior)`) exige que o candidato preserve integralmente
     toda declaração pré-existente (mesmo nome, domínio/fonte, assinatura, grounding e glosa).
   - Adições de novos sorts e símbolos preservando os anteriores são marcadas como `Compatible`.
   - Modificações em declarações existentes quebram compatibilidade (`Incompatible`).
   - Versão numérica e ordem física das listas entram na identidade criptográfica $h(V)$,
     mas não afetam a compatibilidade semântica.
8. **Extensão determinística (`extend`)**:
   - Substitui declarações pré-existentes na posição ordinal original e anexa novas ao fim.
   - Incrementa a versão, revalida integralmente a estrutura e aciclicidade, e retorna a nova
     instância acompanhada da classificação de compatibilidade.
9. **Formato `vocabulary/v1`**:
   - Envelope canônico em array CBOR fixo `[1, version, [SortDecl...], [SymbolDecl...]]`.
   - Sem mapas CBOR, sem floats, sem comprimentos indefinidos.
   - Rejeição estrita de bytes restantes no decodificador (`CanonError::TrailingBytes`).
   - Sem novas dependências externas no TCB além de `logos-canon` e `logos-crypto`.
10. **Separação de domínio**:
    - Tag exclusiva reservada `b"logos/vocab/v1\0"` em `spec/domains.md`.
    - Hash tipado: $h(V) = \text{SHA-256}(\mathtt{"logos/vocab/v1\textbackslash 0"} \mathbin{\Vert} \text{canon}(V))$.

## Consequências

- Construção/decodificação recusam ciclos de dependência, inclusive nos termos.
- O percurso e a ordenação topológica não usam recursão proporcional ao grafo.
- O hash tipado identifica os bytes exatos, incluindo versão e glosas; não prova
  grounding, existência de autoridade ou equivalência lógica.
- `Compatible` significa preservação estrutural, não certificação de migração.
- Sem novas dependências externas.

## Alternativas rejeitadas

- **Axiomas completos `P ↔ φ` no vocabulário**: permitiria que definições ocultassem
  axiomatizações arbitrárias que violassem a conservatividade de extensões e dificultaria
  a análise puramente sintática de ciclos.
- **Sobrecarga de nomes por aridade/sort**: enfraqueceria a unicidade conceitual de termos
  e aumentaria a complexidade do sort-check e do parser em etapas futuras.
- **Resolução e atestação imediata de `FactRef`/`PolicyRef`**: violaria o princípio de
  independência e divisão em camadas do roadmap (fatos pertencem à Etapa 6.1 e políticas à 7.1).
- **Algoritmo de DFS recursivo para ordenação topológica**: descartado pelo risco de
  esgotamento de stack em vocabulários gerados automaticamente com milhares de nós.
- **Mapas CBOR**: permitidos por canon/v1, mas arrays posicionais seguem o padrão
  air/v1 e evitam um segundo esquema de campos nesta etapa.
