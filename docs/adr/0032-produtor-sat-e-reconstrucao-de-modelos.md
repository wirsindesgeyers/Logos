# ADR 0032 — Produtor SAT e reconstrução de modelos

- Estado: aceito para implementação da4.6
- Data: 2026-10-16
- Base: a6d3f37

## Decisões

1. Usar CaDiCaL3.0.1 na revisão c60730422e758ef1cebe7aeddf2dda31c996bf04,
   já usada no corpus4.4. Conservar fonte/licença/hash e build offline com
   compilador C++ do host. Prover não integra a TCB; hash do solver é
   proveniência, nunca substitui o recheck. Não exigir binários idênticos
   entre hosts/compiladores nem ampliar os checkers para aceitar seu output.
2. Solicitar --lrat --no-binary; não é preciso conversor DRAT. Qualquer
   adaptação de LRAT (como remover cauda após derivação da cláusula vazia)
   acontece em Python fora da TCB, mantém CNF/IDs e é rechecada integralmente.
   Não normalizar CNF, renumerar suas cláusulas nem fabricar provas em Python.
3. Compor ground_for_check/clausify existentes em um executável de apoio
   (exemplo Rust); Python recebe bytes canônicos ground/v1 e CNF exata.
   O protocolo de transporte JSON não é formato normativo nem abre o
   construtor selado de Obligation. Nenhum código runtime da TCB precisa
   mudar. A configuração dos caminhos de encoder/solver é explícita.
4. SAT→FiniteModel via variáveis primitivas da tabela de átomos, que ocupam
   IDs1..N. Auxiliares não são símbolos. Exigir valoração completa/consistente
   que satisfaça a CNF antes da conversão; representantes enumerados têm
   ordinais fixos, funções/constantes valores únicos e predicados tuplas.
   Defined é omitido e rederivado pelo checker; completar partes Open não
   denotadas com domínio singleton/tabelas totais. Sem buscar modelo com Z3.
5. API distingue UNSAT/SAT/EXHAUSTED; candidatos continuam não confiáveis.
   Saída malformada/configuração inválida é erro explícito, recurso excedido
   é inconclusivo. Processos sem shell, ambiente sem CADICAL_*, arquivos
   privados, limites de tempo/memória/tamanho e encerramento do grupo.
6. Corpus:300 obrigações válidas explícitas com premissas consistentes,
   regras finitas de política e variações determinísticas, além de pares
   SAT/controles de reconstrução. Identidades distintas e famílias/denominadores
   publicados antes dos resultados. Não implementar o objeto Policy7.1
   nem atribuir autoridade a fontes opacas.

Estimativa4.6 revisada P→M: build offline, transporte e reconstrução contextual
de tabelas exigem controles adicionais; entregáveis/ordem/critério preservados.

## Validação

UNSAT: mesmos bytes CNF/prova nos dois checkers reais. SAT: modelo no
ModelChecker real. Controles negativos de protocolo/limites/hash/candidato;
≥285/300 aceites cake_lpr e100% dos casos SAT válidos no checker camada0.
O produtor nunca emite claim ou resultado Accepted em nome da TCB.

Fonte primária: [CaDiCaL fixado](https://github.com/arminbiere/cadical/tree/c60730422e758ef1cebe7aeddf2dda31c996bf04),
BUILD.md/src/options.hpp e receitas do corpus4.4.
