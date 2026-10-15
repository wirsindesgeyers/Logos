# Épocas — mecanismo genérico (Etapa 3.4)

Contrato adotado pelo mantenedor no ADR0023. Isto define um intervalo em
memória e o uso de épocas já autenticadas pelo ledger; não define Fact.

`logos_core::EpochRange::new(first: u64, last: u64)` retorna
`Result<EpochRange, EpochError>`. Limites privados inclusivos; first > last
retorna erro com ambos os valores. Getters first()/last() preservam u64
inteiro; contains(epoch) indica pertinência, incluindo os extremos.
Nenhuma soma de1, estreitamento para i64, relógio ou valor padrão é usado.

Exemplos normativos:

| Intervalo | Contém | Recusa |
| --- | --- | --- |
| [3,5] | 3,4,5 | 0,2,6,u64::MAX |
| [0,0] | 0 | 1,u64::MAX |
| [u64::MAX,u64::MAX] | u64::MAX | 0,u64::MAX−1 |
| [0,u64::MAX] | todo u64 | nenhum u64 |
| [5,3] | construção inválida | erro, sem intervalo substituto |

EpochRange sozinho não autentica fórmula/emissor, não torna entrada citável
nem altera estado lógico. Não implementa Canon/CanonDecode/Domain e não
introduz formato serializado ou domínio novo; Fact/valid da6.1 especificará
seu uso serializado e sua verificação. O teste integrado de fato [3,5] citável
em4/recusado em6 permanece explicitamente no ROADMAP6.1.

EpochAdvanced é descrito em [events.md](events.md); new_epoch precisa ser
maior que a cabeça anterior, pode saltar épocas e vai até u64::MAX. Igualdade,
regressão e tentativa de avançar além do máximo são recusadas sem consumir
seq ou atualizar head. Genesis permanece época0.

O snapshot real atual só permite Verified/Assumption emitidos na época
corrente. Após avanço, não promove eventos antigos, Unknown ou Invalidated;
histórico/fórmula e época de emissão continuam consultáveis. LedgerEntry.epoch
é aplicabilidade decidida pela view confiável; o gerador exige época corrente.
Não escrever a época atual no lugar da antiga para contornar a regra.
Um opcional policy_root/vocab não prolonga premissas nem altera h(O).

Testes: logos-core/tests/epochs.rs compara81 pares com enumeração explícita
e limites u64. logos-ledger/tests/index.rs usa Ledger/CAS/snapshot/generate
reais nas épocas3/4/6, com rejeição de premissas antigas e sem promoção de
estados. Vetores históricos e estendidos recebem replay/assinatura/codec
estrito; core_decode e event_decode exercitam a TCB no script oficial.
Políticas, fatos, relógio de expiração e ações continuam nas fases futuras.
