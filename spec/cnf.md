# CNF canônica — cnf/v1 (Etapa 4.3)

Normativo: [ADR0028](../docs/adr/0028-tseitin-canonico-e-rederivacao.md).
Entrada: GroundProgram validado da [4.2](grounding.md). Saída de clausify;
não é prova de insatisfatibilidade nem autorização.

## Variáveis e cláusulas

Átomo i recebe variável i+1. Percorrer todos os nós do DAG por ID crescente;
Atom reutiliza sua variável primitiva, demais nós recebem auxiliares crescentes
após os átomos. Variáveis são inteiros positivos até i32::MAX. Para nó v:

```text
False: [-v]
True:  [v]
Not(a): [-v,-a], [v,a]
And(a1,...,ak): [-v,ai] para cada i; [v,-a1,...,-ak]
Or(a1,...,ak):  [v,-ai] para cada i; [-v,a1,...,ak]
```

Assertar [constraints], [premise_root] para cada premissa na ordem de O e
[-goal]. Todas as definições entram, inclusive nós não alcançados por essas
assertivas. Não assertar a implicação root de GroundProgram. As equivalências
garantem extensão única das variáveis auxiliares para cada valoração primitiva.

Ordenar literais de cada cláusula pelo valor inteiro assinado crescente.
Ordenar pares (cláusula, origem) lexicograficamente pelos literais, depois
pela origem. Preservar multiplicidade de literais/cláusulas e tautologias.
Nenhum pré-processamento, subsunção, resolução ou minimização.

## Proveniência

ID de cláusula = posição ordenada+1, alinhado a origins. Ordem das origens:
Premise(ref)=0, NegatedGoal=1, DomainAxiom(ref)=2, Definitional=3; refs por
bytes crescentes. Definições e assertiva de constraints são Definitional.
Assertiva de premissa é DomainAxiom se sua ref aparece em domain_axioms,
senão Premise; assertiva de goal negado é NegatedGoal.

domain_support conserva a lista domain_axioms do grounder, inclusive para
premissas transformadas em tautologia. É suporte da transformação, não cláusula
SAT inventada. É uma lista conservadora de requisitos de contexto, não
used. Etapa4.7 deve conservá-la e analisar seu alcance junto com as origens
alcançadas pelo certificado, testando precisão. Não unir automaticamente
todas as referências a used, inclusive em provas independentes do domínio.
Fontes opacas não se tornam autenticadas.
ProvenanceMap é somente saída em memória, sem decoder/wire nesta etapa.

## Bytes DIMACS e identidade

ASCII, decimais sem zeros à esquerda ou sinal '+', somente espaço ASCII e LF:

```text
p cnf <variables> <clauses>\n
<literal> <literal> ... 0\n
```

Cláusula vazia seria `0\n`; nenhum comentário ou linha extra. Espaço separa
cada literal do próximo token, inclusive o zero final. Número de variáveis
é o maior ID alocado, inclusive se alguma variável não aparece em cláusula.
encoder_version=1. h(CNF)=SHA256("logos/cnf/v1\0" || bytes_dimacs).
Cnf implementa Domain; dimacs() é a representação canônica para hash_raw.
Não usar CBOR/Canon de Cnf nem tag CERTIFICATE. CAS: cnf-v1, 16 MiB padrão.

## Limites e leitura para auditoria

CnfLimits inclusivos: max_variables=1000000, max_clauses=1000000,
max_literals=5000000, max_bytes=16777216. Conferir somas/produtos/IDs e
reservas antes de materializar saída. Unknown(Size/Memory) ou erro explícito;
nenhum truncamento. Limites lógicos não limitam rigidamente RSS/tempo.
ground_for_check confere bytes/hash/versão/tipagem com decode_for_check,
reutiliza a análise e tradução, sem conversão para Obligation selada.

## Registro no evento e rederivação

```text
ObligationCreated corpo v2 = [2,3,hO,1,artifact_raw,[hCNF,1]]
```

Último 1 é encoder_version; rota 1=Lrat. Some(CnfRecord) somente nessa rota,
versão 1 obrigatória. artifact_raw continua SHA-256 cru de bytes_dimacs.
Corpo v1 `[1,3,hO,route,artifact]` tem cnf=None e conserva bytes históricos.
CNF referenciada deve estar no CAS antes de assinar; inventário/GC inclui cnf.

Auditor verify rederiva cada ObligationCreated/Lrat de O/V do CAS usando
encoder conhecido e limites padrão, compara bytes, hCNF e artifact_raw. Recusa
Lrat v1 sem metadados, versão desconhecida, objeto ausente/adulterado ou erro
de tradução. Outras rotas não são CNF. Authenticate/export só autenticam cadeia.
Provas Verified permanecem proofs_unchecked até 4.5; conferir tradução não
checa certificado LRAT, used, registry ou autoridade de produção de claims.

## Evidência

Vetores independentes em spec/vectors/cnf/, produtor Python próprio;
diferencial 5k contra Z3 fora da TCB e enumeração pequena, mutações compiladas,
histórico real com rederivação de todos os eventos e export. Repetições de
DIMACS/h(CNF) entram na CI; critério remoto só fecha após observação.
