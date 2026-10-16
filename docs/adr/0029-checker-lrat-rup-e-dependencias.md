# ADR 0029 — checker RUP, limites e dependências históricas

- Estado: aceito para implementação da Etapa4.4
- Data: 2026-10-15
- Base: 37c544e

## Contexto

A4.4 exige adições RUP, deleções, P2 e fecho de cláusulas originais.
A CNF cnf/v1 e Budget já existem. O formato LRAT completo inclui RAT;
o suporte expresso da etapa é seu subconjunto RUP. Referência primária:
[Cruz-Filipe et al., Efficient Certified RAT Verification, §3–4](https://www.cs.utexas.edu/~marijn/publications/lrat.pdf).

## Decisão

1. Perfil textual versionado lrat-rup/v1 descrito em spec/lrat.md; nenhuma
   mudança nos bytes cnf/v1, domínios de hash ou eventos. Não aceitar RAT,
   DRAT, LRAT binário, LPR ou PR. Certificados continuam bytes crus no domínio
   certificate/v1 existente; identificação do perfil é externa, não um cabeçalho
   inventado no LRAT. CNF de entrada deve satisfazer a gramática cnf/v1.
2. LratChecker::new prepara diagnósticos canônicos falivelmente, como o
   ModelChecker existente. check_lrat é método com os quatro argumentos do
   contrato e retorna CheckOutcome; check_with_used acrescenta relatório de
   IDs somente quando Accepted. Sem fallback na preparação de Reason/Text.
3. IDs de adição u64 estritamente crescentes, superiores ao número original;
   deleção usa o último ID e só remove IDs vivos. Armazenamento ordenado por
   adição, busca binária, sem alocar pelo maior ID. Não reutilizar IDs apagados.
4. RUP nega a cláusula proposta, usa somente hints positivos vivos e anteriores,
   na ordem indicada, cada um unitário até o último produzir conflito. Literais
   repetidos contam uma vez; tautologias originais são preservadas. Cláusulas
   propostas tautológicas são recusadas neste perfil. Toda linha, inclusive
   depois da primeira vazia, deve ser válida; exige vazio derivado na prova.
5. Guardar arestas de todos os hints da derivação; deleção desativa a cláusula,
   sem apagar histórico. Fecho iterativo parte da primeira vazia derivada e
   devolve IDs originais crescentes. Não inferir origens do ProvenanceMap nem
   used_premises (4.7). Verificar até derivações irrelevantes para o fecho.
6. Budget.size_bytes limita soma CNF+prova, steps cobre bytes/hash/parsing,
   propagação, consultas e fecho. mem_bytes limita alocações lógicas
   contabilizadas por capacidade com reservas falíveis e crescimento geométrico, sem prometer RSS/rlimit; wall_ms
   é ignorado pelo checker interno. Exaustão é Unknown, entrada inválida é
   Rejected. Erros de alocação são Unknown(Memory); aritmética é checada.
7. Dependências workspace novas: logos-lrat → logos-canon/core/crypto/ground;
   serde_json somente dev. Todas já permitidas em deny.toml, sem pacote externo
   novo ou solver na TCB. Parsers CNF/LRAT têm alvos de fuzz independentes.
8. Corpus histórico SAT-2002 distribuído pela SATLIB, canonicalizado fora da
   TCB antes de gerar provas. CaDiCaL fixado por revisão, exclusivamente na
   preparação dos fixtures; poda de adições fora do fecho e deleções por
   ferramenta Python, seguida de rechecagem independente. XZ determinístico
   reduz o artefato; o harness Rust usa Python padrão para descompressão/JSON.
   CI não baixa corpus nem executa solver. Nenhuma biblioteca de compressão em runtime.

9. O workflow de fuzz descobre os bins em fuzz/Cargo.toml e usa matriz de
   um alvo por job, máximo quatro jobs simultâneos e120min por job. Os16
   alvos ×1h excederiam os720min do job sequencial anterior. O mesmo ci/fuzz.sh
   continua executando cada alvo; logs têm nomes únicos. Não alegar hora remota
   sem observar o job.

10. Perfil test otimiza somente logos-lrat em nível1. O corpus tem223MB de
    certificados; a rodada inicial em debug levou582.77s para200 provas/600
    corrupções. Não alterar overflow-checks ou debug-assertions, nem otimizar
    outros crates. Reservas geométricas evitam crescimento quadrático e cobram
    capacidade; corrupções de hint e vazia sem derivação falham cedo no harness.

## Consequências

Algumas provas válidas no LRAT completo são recusadas pelo perfil explícito;
isto limita completude, nunca permite afirmar falsidade a partir de rejeição.
Não altera escopo nem ordem. cake_lpr, seu wrapper/registry e auditor vêm na4.5;
a concordância fica aberta conforme o critério original.
