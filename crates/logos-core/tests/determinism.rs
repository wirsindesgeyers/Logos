//! Demonstração de determinismo estrito (Critério S3) para `logos-core`.
//!
//! Executa 1000 repetições de cada caso fixo de referência do checker sobre bytes,
//! cobrando 1 passo antes de cada validação e rejeitando o byte 255 (0xFF).
//!
//! Compara tanto o [`CheckOutcome`] quanto os bytes canônicos produzidos (`core/v1`)
//! contra valores e sequências de bytes fixas e versionadas, idênticas em x86_64 e aarch64.
//!
//! Nenhum uso de relógio de sistema, gerador aleatório (RNG), tabelas de dispersão com
//! ordem indeterminada ou tipos dependentes de arquitetura (`usize`) no cálculo do orçamento.
//! O orçamento é fixado com `wall_ms = 0`, com teste adicional comprovando que alterar
//! `wall_ms` não afeta o checker interno.

use logos_canon::{Text, from_canon, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted, Reason, StepMeter};

/// Checker de referência do contrato da Etapa 0.6:
/// Percorre a sequência de bytes, deduzindo 1 passo do medidor antes de inspecionar
/// cada byte. Se encontrar o byte 255, rejeita com razão descritiva.
fn reference_checker(meter: &StepMeter, bytes: &[u8]) -> CheckOutcome {
    for &b in bytes {
        if let Err(e) = meter.charge(1) {
            return CheckOutcome::Unknown(e);
        }
        if b == 255 {
            let reason = Reason::new(Text::new("invalid byte 255").expect("texto estatico valido"));
            return CheckOutcome::Rejected(reason);
        }
    }
    CheckOutcome::Accepted
}

/// Razão padrão emitida para o byte 255.
fn invalid_byte_reason() -> Reason {
    Reason::new(Text::new("invalid byte 255").expect("texto estatico valido"))
}

/// Bytes canônicos esperados para `CheckOutcome::Accepted` em `core/v1`.
/// Array de 2 elementos: `[1, 0]` -> `0x82, 0x01, 0x00`.
const EXPECTED_ACCEPTED_BYTES: &[u8] = &[0x82, 0x01, 0x00];

/// Bytes canônicos esperados para `CheckOutcome::Rejected("invalid byte 255")` em `core/v1`.
/// Array de 3 elementos: `[1, 1, [1, "invalid byte 255"]]`.
/// - `0x83, 0x01, 0x01` (envelope do outcome)
/// - `0x82, 0x01, 0x70` (envelope da Reason: array de 2, versão 1, texto de 16 bytes)
/// - `b"invalid byte 255"` (16 bytes UTF-8/ASCII)
const EXPECTED_REJECTED_BYTES: &[u8] = &[
    0x83, 0x01, 0x01, 0x82, 0x01, 0x70, b'i', b'n', b'v', b'a', b'l', b'i', b'd', b' ', b'b', b'y',
    b't', b'e', b' ', b'2', b'5', b'5',
];

/// Bytes canônicos esperados para `CheckOutcome::Unknown(Exhausted::Steps)` em `core/v1`.
/// Array de 3 elementos: `[1, 2, [1, 1]]`.
/// - `0x83, 0x01, 0x02` (envelope do outcome: tag 2 para Unknown)
/// - `0x82, 0x01, 0x01` (Exhausted::Steps: tag 1)
const EXPECTED_EXHAUSTED_STEPS_BYTES: &[u8] = &[0x83, 0x01, 0x02, 0x82, 0x01, 0x01];

/// Caso fixo de teste determinístico para o checker de referência.
struct DeterminismCase {
    name: &'static str,
    input: &'static [u8],
    budget: Budget,
    expected_outcome: fn() -> CheckOutcome,
    expected_bytes: &'static [u8],
}

fn test_cases() -> [DeterminismCase; 8] {
    [
        // 1. Accepted: entrada vazia com orçamento de zero passos (limite exato zero).
        DeterminismCase {
            name: "accepted_empty_zero_budget",
            input: &[],
            budget: Budget {
                steps: 0,
                mem_bytes: 0,
                size_bytes: 0,
                wall_ms: 0,
            },
            expected_outcome: || CheckOutcome::Accepted,
            expected_bytes: EXPECTED_ACCEPTED_BYTES,
        },
        // 2. Accepted: 3 bytes válidos com limite exato de 3 passos.
        DeterminismCase {
            name: "accepted_exact_boundary",
            input: &[1, 2, 3],
            budget: Budget {
                steps: 3,
                mem_bytes: 0,
                size_bytes: 0,
                wall_ms: 0,
            },
            expected_outcome: || CheckOutcome::Accepted,
            expected_bytes: EXPECTED_ACCEPTED_BYTES,
        },
        // 3. Accepted: 4 bytes válidos com orçamento superavitário de passos e limites de memória/tamanho.
        DeterminismCase {
            name: "accepted_surplus_budget",
            input: &[10, 20, 30, 40],
            budget: Budget {
                steps: 100,
                mem_bytes: 1024,
                size_bytes: 2048,
                wall_ms: 0,
            },
            expected_outcome: || CheckOutcome::Accepted,
            expected_bytes: EXPECTED_ACCEPTED_BYTES,
        },
        // 4. Rejected: byte inválido 255 na primeira posição, limite exato de 1 passo.
        DeterminismCase {
            name: "rejected_immediate_first_byte",
            input: &[255],
            budget: Budget {
                steps: 1,
                mem_bytes: 0,
                size_bytes: 0,
                wall_ms: 0,
            },
            expected_outcome: || CheckOutcome::Rejected(invalid_byte_reason()),
            expected_bytes: EXPECTED_REJECTED_BYTES,
        },
        // 5. Rejected: byte inválido 255 após 2 bytes válidos, com orçamento suficiente.
        DeterminismCase {
            name: "rejected_after_valid_steps",
            input: &[1, 2, 255, 4],
            budget: Budget {
                steps: 10,
                mem_bytes: 512,
                size_bytes: 1024,
                wall_ms: 0,
            },
            expected_outcome: || CheckOutcome::Rejected(invalid_byte_reason()),
            expected_bytes: EXPECTED_REJECTED_BYTES,
        },
        // 6. Exhausted: 1 byte a processar, mas orçamento de passos é zero.
        DeterminismCase {
            name: "exhausted_zero_budget_with_work",
            input: &[1],
            budget: Budget {
                steps: 0,
                mem_bytes: 0,
                size_bytes: 0,
                wall_ms: 0,
            },
            expected_outcome: || CheckOutcome::Unknown(Exhausted::Steps),
            expected_bytes: EXPECTED_EXHAUSTED_STEPS_BYTES,
        },
        // 7. Exhausted: 3 bytes a processar com orçamento de 2 passos (esgota exatamente no 3º passo).
        DeterminismCase {
            name: "exhausted_exact_boundary_short_by_one",
            input: &[1, 2, 3],
            budget: Budget {
                steps: 2,
                mem_bytes: 0,
                size_bytes: 0,
                wall_ms: 0,
            },
            expected_outcome: || CheckOutcome::Unknown(Exhausted::Steps),
            expected_bytes: EXPECTED_EXHAUSTED_STEPS_BYTES,
        },
        // 8. Exhausted: byte 255 está na posição 2 (3º passo), mas orçamento é de 2 passos.
        // A cobrança falha ANTES de inspecionar o byte 255, gerando Unknown(Steps) e NUNCA Rejected.
        DeterminismCase {
            name: "exhausted_before_reaching_invalid_byte",
            input: &[1, 2, 255],
            budget: Budget {
                steps: 2,
                mem_bytes: 0,
                size_bytes: 0,
                wall_ms: 0,
            },
            expected_outcome: || CheckOutcome::Unknown(Exhausted::Steps),
            expected_bytes: EXPECTED_EXHAUSTED_STEPS_BYTES,
        },
    ]
}

#[test]
fn s3_reference_checker_1000_repetitions_deterministic() {
    let cases = test_cases();

    for case in &cases {
        let expected = (case.expected_outcome)();

        for rep in 0..1000 {
            let outcome =
                StepMeter::run(&case.budget, |meter| reference_checker(meter, case.input));

            assert_eq!(
                outcome, expected,
                "caso '{}' na repetição {}: outcome divergente da expectativa",
                case.name, rep
            );

            let canonical_bytes =
                to_canon(&outcome).expect("serialização canônica deve ser bem-sucedida");

            assert_eq!(
                canonical_bytes.as_slice(),
                case.expected_bytes,
                "caso '{}' na repetição {}: bytes canônicos divergentes da especificação normativa",
                case.name,
                rep
            );

            let roundtrip: CheckOutcome = from_canon(&canonical_bytes)
                .expect("decodificação canônica dos bytes deve ter sucesso");

            assert_eq!(
                roundtrip, expected,
                "caso '{}' na repetição {}: round-trip de decodificação divergiu",
                case.name, rep
            );
        }
    }
}

#[test]
fn s3_ignored_error_latch_1000_repetitions_deterministic() {
    // Cenário onde o checker propositalmente ignora o erro de cobrança e tenta retornar Accepted.
    // O latch permanente do StepMeter deve forçar Unknown(Steps) em 100% das 1000 repetições.
    let budget = Budget {
        steps: 1,
        mem_bytes: 0,
        size_bytes: 0,
        wall_ms: 0,
    };
    let expected = CheckOutcome::Unknown(Exhausted::Steps);

    for rep in 0..1000 {
        let outcome = StepMeter::run(&budget, |meter| {
            let _ = meter.charge(1); // Consome 1/1
            let _ = meter.charge(1); // Excede saldo; latch irreversível disparado
            // Ignora o erro e tenta declarar sucesso:
            CheckOutcome::Accepted
        });

        assert_eq!(
            outcome, expected,
            "repetição {}: latch não prevaleceu sobre Accepted retornado",
            rep
        );

        let canonical_bytes =
            to_canon(&outcome).expect("serialização de Unknown(Steps) deve ter sucesso");
        assert_eq!(
            canonical_bytes.as_slice(),
            EXPECTED_EXHAUSTED_STEPS_BYTES,
            "repetição {}: bytes de Unknown(Steps) divergentes",
            rep
        );
    }
}

#[test]
fn s3_wall_ms_is_irrelevant_to_internal_checker() {
    // Demonstração explícita de que alterar `wall_ms` não altera o resultado do checker interno.
    // Testa variações de `wall_ms` (0, 1, 42, 10_000, u64::MAX) em 1000 iterações.
    let wall_variations: [u64; 5] = [0, 1, 42, 10_000, u64::MAX];
    let cases = test_cases();

    for case in &cases {
        let expected = (case.expected_outcome)();

        for &wall in &wall_variations {
            let budget_with_wall = Budget {
                steps: case.budget.steps,
                mem_bytes: case.budget.mem_bytes,
                size_bytes: case.budget.size_bytes,
                wall_ms: wall,
            };

            for rep in 0..200 {
                let outcome = StepMeter::run(&budget_with_wall, |meter| {
                    reference_checker(meter, case.input)
                });

                assert_eq!(
                    outcome, expected,
                    "caso '{}' com wall_ms={} na repetição {}: outcome divergiu do baseline wall_ms=0",
                    case.name, wall, rep
                );

                let canonical_bytes =
                    to_canon(&outcome).expect("serialização canônica deve ter sucesso");

                assert_eq!(
                    canonical_bytes.as_slice(),
                    case.expected_bytes,
                    "caso '{}' com wall_ms={} na repetição {}: bytes canônicos divergiram",
                    case.name,
                    wall,
                    rep
                );
            }
        }
    }
}
