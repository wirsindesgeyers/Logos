# ADR 0015 — avaliação finita e relações Defined

- Estado: aceito para Etapa 2.2.
- Data: 2026-10-05.
- Escopo: ROADMAP 2.2; sem solver produtor, checker de refutação ou ledger.

## Contexto

vocabulary/v1 admite Formula como corpo de Pred/Const/Func Defined, mas 1.2 não
fixou parâmetros nem denotação de valores. model/v1 não contém tabelas Defined;
Enumerated fixa rótulos ordinais também para constantes definidas. Converter
corpo arbitrário em um valor, selecionar primeiro resultado ou usar ordinal em
vez da definição esconderia entrada inválida na TCB.

## Decisão

1. Pred Defined é corpo booleano sob parâmetros livres: argumento i tem índice
   De Bruijn i; o primeiro argumento é o binder externo mais interno. Binder
   introduzido no corpo desloca parâmetros normalmente.
2. Const/Func Defined é relação sobre resultado e argumentos: resultado tem
   índice 0; argumento i tem índice i+1. Const não tem argumentos. A relação deve
   ter exatamente um resultado para cada entrada no produto cartesiano.
3. Antes da fórmula final, conferir tipagem de todos os corpos e totalidade/
   unicidade de todos os símbolos de valor Defined, inclusive não utilizados.
   Domínios e tuplas são percorridos crescentemente, último eixo mais rápido.
   Não criar nem persistir tabelas para Defined, nem materializar AST expandida.
4. Const Defined enumerada deve expandir para seu ordinal intrínseco. Discordar
   é erro de entrada, não falsidade e não autorização para trocar a denotação.
5. API contextual `eval(M,V,φ,Budget) -> Result<bool,EvalError>`. A assinatura
   abstrata `Result<bool,Exhausted>` do roadmap descreve apenas entradas válidas;
   `EvalError::Unknown(Exhausted)` conserva esse resultado. Outras variantes são
   necessárias para h(V) errado, fórmulas mal tipadas e relações não funcionais.
   Não mapear erros de entrada para Unknown nem bool. Não emitir Verdict.
6. Reusar sort-check iterativo com binders iniciais emprestados, sem criar uma
   segunda convenção de tipagem nem copiar AST para envolver quantificadores.
7. Interpretador com pilhas explícitas; conectivos e quantificadores integrais
   sem curto-circuito. Fórmula/cadeia de definições não usa stack nativo recursivo.
8. Um StepMeter com latch cobre preparação e execução. Etapa impõe apenas steps
   como o contrato do core; wall_ms ignorado, mem_bytes/size_bytes não prometem
   RSS/tamanho de entrada. AIR Limits/ModelLimits são fronteiras de construção;
   falhas de reserva do interpretador são Unknown(Memory). Nenhum timeout vira
   false. Política precisa de passos em spec/evaluation.md.
9. Nenhum formato/domínio/hash novo: bytes model/v1 e vocabulary/v1 intactos.
   Nenhuma dependência runtime nova. Z3-solver é oráculo dev Python independente,
   fora da TCB, não produtor de modelos da 2.4.

## Alternativas recusadas

- Recusar Const/Func Defined: não cumpre expansão de todos os símbolos 2.2.
- Tratar Formula como termo: categoria distinta, sem denotação normativa.
- Escolher primeiro resultado: aceita relação parcial/ambígua por acidente.
- Validar só símbolos usados: permite estruturas sem interpretação total passar
  por modelo mesmo numa fórmula true.
- Definir ordem pelo stack do chamador: captura variável e inverte argumentos.
- Recursão com limite artificial de profundidade: restringe AIR já aceito;
  pilhas explícitas preservam contrato sem stack overflow.

## Consequências

Preparação pode esgotar orçamento antes da fórmula. Unknown é resultado correto.
Não há garantia de fidelidade de grounding/autoridade, nem prova sobre modelos
infinitos. Validação exaustiva pode ser cara; otimização/incremental fora de 2.2.
Resultados sem wall são determinísticos para mesmo input e mesmo orçamento;
confirmação remota x86_64/aarch64 permanece separada da evidência local.

Medição local do núcleo formatado: 537 LOC de código Rust (tokei), acima do alvo
<400 do roadmap; fronteira/erros somam mais 118 LOC. Mantidas continuações
explícitas e reservas falíveis, sem compactar sintaxe para reduzir contagem.
Uma única máquina reutiliza pilhas entre termos, definições e candidatos:
não aloca máquina/Vec por termo nem por candidato. O alvo é transparência de
tamanho, não exclusão silenciosa de semântica ou troca por recursão insegura.
