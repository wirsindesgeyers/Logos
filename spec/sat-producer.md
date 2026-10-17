# Produtor SAT — Etapa4.6

API Python logos_provers.sat, fora da TCB, conforme ADR0032. Não altera
canon/v1, AIR/v1, obligation/v1, ground/v1, cnf/v1, model/v1 ou lrat-rup/v1.

## API e confiança

produce(O:bytes,V:bytes,*,encoder:Path,solver:Path,limits:Limits)->Candidate.
Paths são executáveis explícitos. O encoder de apoio é compilado por
cargo build -p logos-audit --example prover_bridge; compõe as APIs4.3 sem
abrir o construtor selado de O nem acrescentar runtime à TCB. Sem ledger/kernel.

Candidate contém status UNSAT/SAT/EXHAUSTED, bytes CNF/ground e h(O)/h(V).
UNSAT contém certificate LRAT e seu hash tipado, além do SHA256 cru da prova
original. SAT contém model/v1. EXHAUSTED contém motivo, sem certificado/modelo.
Todos são candidatos não confiáveis, inclusive depois de conferir a
valoração SAT contra a CNF. Só os checkers reais podem aceitá-los; não há
Verified/Refuted/claim ou chamada ao kernel neste produtor.

CaDiCaL3.0.1 fixado em vendor/cadical/lock.json emite LRAT textual com
--lrat --no-binary. Não converter CNF, remover duplicatas ou renumerar seus
IDs. A adaptação não confiável conserva as linhas da prova até a primeira
adição de cláusula vazia, completa LF final se ausente e descarta a cauda;
a prova final inteira
precisa ser aceita pelos checkers. Nenhum DRAT/conversor é necessário.

## Reconstrução SAT

Exit10 exige uma única linha s SATISFIABLE e valoração completa dos IDs1..N,
sem conflitos/duplicatas e com um único0 final nas linhas v. A valoração deve
satisfazer todas as cláusulas da CNF exata. Exit20 exige s UNSATISFIABLE e
prova presente; exit0/s UNKNOWN é EXHAUSTED. Outros status/saídas são erro.

Átomos primitivos de ground/v1 ocupam IDs1..atoms.len; auxiliares são
descartados. Pred recebe as tuplas verdadeiras; Const/Func recebe resultado
único por tupla em ordem lexicográfica (último eixo varia mais rápido).
Representantes Enum denotam seu ordinal. Resultado singleton dispensa escolha.
Defined é omitido de model/v1 e rederivado pelo ModelChecker. Partes Open
não denotadas recebem domínio singleton e tabelas totais com resultado0 ou
predicados falsos. O classificador existente ainda recusa todo resultado de
valor Open e parâmetros Defined Open, mesmo não usados; não ampliar o fragmento.
Enum vazio não possui modelo clássico não vazio; resposta SAT seria erro.

## Limites

Limits usa timeout_ms5000, mem_bytes256MiB e max_bytes16MiB por padrão,
todos inteiros positivos até u32::MAX. Prazo monotônico compartilhado pela
validação/encoder/solver/conversão; subprocessos em grupo próprio, sem shell,
ambiente limitado a PATH padrão/LC_ALL, sem opções CADICAL_* herdadas.
prlimit impõe AS/FSIZE/CORE; deadline encerra e recolhe o grupo, inclusive
descendentes. Não é sandbox de rede nem limite físico do RSS Python.
Entrada/codificação/prova/modelo e stdout/stderr têm limites explícitos;
modelo respeita também os limites de células/bytes existentes do produtor.
Exaustão reconhecida é inconclusiva; saída malformada/configuração/I/O são
erro explícito. Sem fallback para outro solver, cache ou portfólio.

## prover-bridge/v1 — transporte de apoio

Uma requisição JSON por linha, campos version=1, mode e bytes hexadecimais
vocabulary/obligation. encode retorna ground/cnf em hexadecimal. proof recebe
cnf/proof, rederiva a CNF de O, exige bytes iguais e executa os dois checkers;
model/witness recebe model e chama ModelChecker de refutação/consistência.
--cake-lpr BIN vem do operador, não do JSON. Resposta tem version=1 e
ok=true/result ou ok=false/error. Sem autoridade no protocolo de transporte.
Limite de linha100663296bytes; campos hex até33554432caracteres. Usar
check_requests do smoke para validação; não confundir seu JSON com certificado.

## Corpus e reprodução

ci/check-sat.sh prepara CaDiCaL offline, cake_lpr por hash e bridge; fixa
spec/vectors/sat/corpus.json antes de gerar provas e executa sat_smoke.
300 obrigações válidas em10 famílias, cada uma com30 contextos: implicação,
conjunção, cadeia, casos, contraposição, bloqueio, bicondicional, relação
binária, função e constante não representante. Domínios2–4 e cinco fatos de
contexto com30 valorações distintas, exigidos também no objetivo.
300 pares acrescentam uma exigência Extra sem premissa; seus modelos são
checados como refutações e como testemunhas da consistência das300 premissas
válidas. Sete controles extras: nullary/singleton/Defined/Open/tabelas de dois sorts/
representante Defined/enum vazio.
O corpus não usa objetos Policy/Fact autenticados, que ainda não existem.

Critério: ≥285/300 aceites cake_lpr, além do recheck Rust;100% dos SAT aceitos
no ModelChecker. O script exige300/300,300 CNFs distintas e todas as
testemunhas de consistência. Não excluir falhas/inconclusivos do denominador.
Relatórios com revisão/compilador/hashes dos executáveis, h(O)/h(CNF)/h(certificado)/h(M)
e resultados em target/sat-producer-report.json. CI/ARM são evidência remota
separada dos resultados locais. Limites duplicates/sparse da4.5 permanecem.
O relatório também emite layer e certificate_bytes por h(O), conforme
ROADMAP ApêndiceD: bytes LRAT na camada1 ou model/v1 na camada0; cnf_bytes
é contabilizado separadamente. Medir o certificado final realmente rechecado.
