# lrat-rup/v1 — checagem textual e cláusulas usadas

Versão1 (ADR0029). Perfil do LRAT definido por
[Cruz-Filipe et al. §3–4](https://www.cs.utexas.edu/~marijn/publications/lrat.pdf).
Nenhum cabeçalho adicional na prova. Hash de certificado é o domínio raw
certificate/v1 existente. Não muda cnf/v1 nem inventa uma tag para o perfil.

## Entradas e identidade

LratChecker::new() prepara Reason/Text falivelmente. Seus métodos recebem
cnf_bytes, Hash<Cnf>, proof_bytes, &Budget. Antes do parser, comparar
hash_raw::<Cnf>(cnf_bytes) com o esperado: SHA256(logos/cnf/v1\0 || bytes).
Não normalizar, não aceitar hash SHA-256 puro. O hash esperado não autentica
sua própria origem; a ligação O→CNF continua responsabilidade da4.3.

CNF: exatamente p cnf V C\n e C linhas de literais separados por um espaço,
terminadas em 0\n (vazia: 0\n). Decimais sem sinal +, zeros iniciais ou -0;
literais i32 com abs≤V, V≤1000000. Literais de cada cláusula não decrescentes;
cláusulas lexicograficamente não decrescentes. Duplicatas e tautologias são
preservadas, IDs originais são posições1..C, incluindo duplicatas.
C/literais/bytes respeitam limites cnf/v1 (1M/5M/16MiB).

## Prova

Cada linha ASCII termina em LF; espaços/tabs separam tokens. Linhas vazias e
comentários iniciados por c (após espaços/tabs) são permitidos. CR/NUL e bytes
não ASCII são recusados, inclusive em comentários. Sem continuação de linha.
Adição: ID LIT* 0 HINT* 0. Deleção: LAST_ID d ID* 0.
ID positivo u64, literal i32 não nulo com abs≤V. Mesma gramática decimal da
CNF. Adições têm ID maior que todos os originais e adições anteriores;
lacunas e u64::MAX são permitidos. Deleção usa o último ID de adição (ou C
antes da primeira), e cada alvo deve existir e estar vivo. Lista vazia vale.
Hints positivos devem existir e estar vivos; referências futuras/apagadas,
RAT negativo, tokens extras e ausência de terminador são Rejected.

## RUP

Negar todos os literais da cláusula proposta para criar atribuição temporária;
recusar proposta tautológica. Para cada hint, avaliar a cláusula referida:
satisfeita ou com ≥2 literais distintos não atribuídos é inválida; exatamente
um propaga esse literal; nenhum produz conflito e deve ser o último hint.
Duplicatas não criam unidades falsas; tautologias nunca produzem conflito.
Exigir conflito; vazio sem hints não é aceito, mesmo se a CNF já contiver vazio
(o hint dessa cláusula original pode justificar uma nova vazia).

Checar todas as linhas, sem aceitar prefixo antes de cauda inválida. Ao menos
uma adição vazia deve ser validada. Não converter Rejected/Unknown em veredicto.

## Fecho e orçamento

Arestas de cada adição apontam aos hints validados. Deleções só desativam
cláusulas para novos passos. Fecho iterativo da primeira vazia derivada produz
IDs originais únicos e crescentes; inclui somente o ramo alcançável. Relatório
expõe IDs somente quando Accepted, junto a h(CNF) e h(certificado inteiro).
Não é um núcleo mínimo; projeção4.7 em [used-premises.md](used-premises.md).

size_bytes limita soma CNF+prova antes do hash; steps cobra esses bytes mais
CNF pelo hash, operações por literal/hint/consulta, fecho e hash do certificado
aceito. Uma única execução
StepMeter impede reinicialização por cláusula. mem_bytes contabiliza capacidades reservadas de
cláusulas, literais, arestas, atribuição, marcação/pilha e saída; reservas são
falíveis, contabilização monotônica e conservadora (sem restituição por deleção, crescimento geométrico).
Não inclui buffers emprestados nem overhead do alocador/RSS. wall_ms ignorado.
Limites inclusivos; exceder recurso é Unknown correspondente, nunca aceite.

Vetores independentes em spec/vectors/lrat/vectors.json. Corpus e evidência
na etapa4.4; comparação cake_lpr fica para4.5.
