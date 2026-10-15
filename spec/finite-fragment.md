# Fragmentos e candidatos de axiomas — Etapa4.1

Contrato normativo de [ADR0026](../docs/adr/0026-contrato-de-fragmentos-e-fontes-de-fechamento.md),
proposta A adotada pelo mantenedor. Implementação TCB em logos-ground.
Não implementa grounding, certificado, autenticação de fonte ou rota de kernel.
MUST/MUST NOT conforme RFC2119.

## Classificação conservadora

```text
classify(&Obligation,&Vocabulary) -> Result<Fragment,GroundError>
classify_with_limits(&Obligation,&Vocabulary,AnalysisLimits) -> Result<Fragment,GroundError>
Fragment = FiniteDomain | General
```

Recebe somente O selado existente, sem novo construtor/decoder. MUST conferir
h(V)==O.vocab. MUST validar assinaturas de todos os símbolos, mesmo não usados,
sort-check de premissas/goal e todos os corpos Defined. Binders Defined são
os mesmos do avaliador: argumentos em ordem reversa, resultado de Const/Func
por último (índice De Bruijn0); Pred não tem parâmetro de resultado.

A análise percorre todo V, incluindo definições não usadas. General se houver:

- quantificador Forall/Exists de sort Open em O ou em qualquer Defined;
- Const/Func de resultado Open (Const é função de aridade zero);
- parâmetro Defined de sort Open, pois a definição é interpretada globalmente.

Demais contextos válidos são FiniteDomain. Um sort Open não utilizado em
quantificador/valor/parâmetro Defined não basta para General. Funções sobre
sorts enumerados são representáveis, sem executar grounding. Nenhuma decisão
é baseada em glosa ou em relevância inferida pelo agente. Não expande Defined
recursivamente: visita cada corpo uma vez e confere suas assinaturas.

MUST validar todo o contexto antes de devolver General. Entrada inválida,
identidade divergente ou limite insuficiente retorna erro, não fragmento de
fallback. A classificação não afirma que exista modelo, prova, fonte citável
ou permissão de ação. Enum vazio é sintaxe válida, sem modelo clássico não vazio.

AnalysisLimits padrão/inclusivos:

| Campo | Padrão | Contagem |
| --- | ---: | --- |
| max_declarations | 10000 | sorts + símbolos |
| max_nodes | 100000 | nós lógicos de goal/premissas/Defined + aridades + constantes das enumerações |

Conferir contagens com aritmética checada antes do hash/validação contextual.
Estes limites não são orçamento de passos, bytes, RSS ou relógio. Hash de V
pode materializar bytes canônicos; nomes/glosas participam da identidade.
Erro de limite MUST ser GroundError::Unknown(Size), não FiniteDomain/General.

## Candidatos lógicos de enumeração

```text
domain_axioms(&Vocabulary,&Sort) -> Result<AxiomCandidates,GroundError>
domain_axioms_with_limits(&Vocabulary,&Sort,AxiomLimits) -> Result<AxiomCandidates,GroundError>
```

Sort MUST existir e ser Enumerated. Saída tem campos privados/getters para
h(V), sort, DomainSource opaco, closure e distinct; sem Canon/Domain/setters
ou conversão para Obligation/PremiseRef/claim. Retorna fórmulas AIR tipadas,
não prova de validade das definições ou autenticação de fonte. Não altera O.

Para constantes c0…c(n−1) na ordem original da enumeração:

```text
closure = forall x:S. Or([x=c0, …, x=c(n−1)])
distinct = [Not(ci=cj) | 0 <= i < j < n]
```

Var0 denota x. Closure usa prefixo Forall, Or(n), Eq(Var0,ci) na ordem original,
sem ordenar por nome ou simplificar. Distinct usa ordem lexicográfica de
índices (i,j). Nenhuma normalização lógica/reordenação silenciosa.

No vazio: Forall(S),Or(0), sem desigualdades. É falso em domínio clássico não
vazio, não autorização vacuosa. Singleton conserva Or(1), sem desigualdades.

| AxiomLimits | Padrão | Mede |
| --- | ---: | --- |
| max_constants | 256 | n antes de expansão |
| max_axioms | 10000 | 1+n(n−1)/2 |
| max_nodes | 100000 | nós lógicos agregados:2+3n+4·n(n−1)/2 |
| air | Limits padrão | profundidade128 e100000 nós por fórmula, incluindo termos |

MUST calcular quantidades com aritmética checada e recusar antes de alocar
saída quadrática. Tetos inclusivos; excesso/overflow é Unknown(Size). Reserva
explícita falha com Unknown(Memory). Limites AIR de tamanho/profundidade também
preservam Unknown(Size). Sem lista parcial, truncamento ou fecho padrão.
Limites não garantem RSS/tempo e não limitam tamanho dos nomes/bytes de V.

## Fronteira de proveniência e fases futuras

DomainSource::Fact/Policy guarda hash de payload opaco da1.2, não digest de
evento citável. MUST NOT copiar esses bytes para PremiseRef e tratá-los como
evento conferido. Candidatos não demonstram existência/autenticidade da fonte.

A decisão do mantenedor preserva integração factual em6.1 e política/
completion em7.3, repetida para projeções com as_of:h(head) em7.5. Só esses
caminhos com fontes/checkers reais podem ligar candidatos a O/used/deps.
Grounder4.2 MUST exigir axiomas explícitos em O; não ganha fecho implícito.
CERTIFY5.4 MUST recusar uso autorizador de fontes ainda não resolvidas.
Não completar predicado a partir de mera ausência de fatos. Declaração de
fragmento ou objeto candidato nunca emite Verified/token.

## Vetores, semântica e reprodução

Nenhum formato serializado/domínio novo. Fórmulas usam AIR/v1 e seu hash
existentes; metadados de candidatos não são serializados. O/vocabulário/
eventos/vetores anteriores permanecem iguais. O fixture JSON em
vectors/fragment/vectors.json é artefato de teste, não wire de candidatos.

Oito casos independentes (Fact/Policy ×0/1/2/3 constantes), produzidos só por
Python canon/hashlib, fixam bytes/hash de V/Formula, fonte e ordem de pares.
Testes semânticos enumeram288 interpretações em V de controle Open com mesmas
assinaturas: fecho sse imagens cobrem o domínio, nomes únicos sse imagens são
distintas. Esse controle permite violar os candidatos; em V Enumerated os
modelos já obedecem às restrições por construção. Não confundir controle
lógico com fonte autenticada ou certificado de prova.

```sh
cargo test -p logos-ground
cargo run -p logos-ground --example fragment_smoke
(cd py && uv run python ../tools/gen_fragment_vectors.py --check)
FUZZ_TOOLCHAIN=nightly-2026-09-28 ci/fuzz.sh 60 vocabulary_decode
```

Fuzz estende alvo existente para V arbitrário e estruturado, determinismo,
limites, fonte/h(V) e tipagem de candidatos. Não introduz decoder TCB novo.
