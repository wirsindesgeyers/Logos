//! Fuzz target para os tipos do `logos-core` (`Budget`, `Exhausted`, `Reason`, `CheckOutcome`, `Verdict`).
//!
//! Para qualquer entrada de bytes arbitrários: se a decodificação for bem-sucedida, a reencodificação
//! canônica deve ser idêntica byte a byte aos bytes de entrada (rejeição de codificação não canônica)
//! e a redecodificação subsequente deve preservar a igualdade do valor. Erros de decodificação são
//! esperados e permitidos; falhas de reencodificação nunca devem ser silenciadas.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_canon::{from_canon, to_canon};
use logos_core::{Budget, CheckOutcome, Exhausted, Reason, Verdict};

fuzz_target!(|data: &[u8]| {
    if let Ok(budget) = from_canon::<Budget>(data) {
        let bytes =
            to_canon(&budget).expect("budget decodificado com sucesso deve ser codificavel");
        assert_eq!(bytes, data, "aceitou codificacao nao canonica para Budget");
        let roundtrip: Budget =
            from_canon(&bytes).expect("bytes codificados de Budget devem ser decodificaveis");
        assert_eq!(
            roundtrip, budget,
            "roundtrip de Budget deve preservar igualdade"
        );
    }

    if let Ok(exhausted) = from_canon::<Exhausted>(data) {
        let bytes =
            to_canon(&exhausted).expect("exhausted decodificado com sucesso deve ser codificavel");
        assert_eq!(
            bytes, data,
            "aceitou codificacao nao canonica para Exhausted"
        );
        let roundtrip: Exhausted =
            from_canon(&bytes).expect("bytes codificados de Exhausted devem ser decodificaveis");
        assert_eq!(
            roundtrip, exhausted,
            "roundtrip de Exhausted deve preservar igualdade"
        );
    }

    if let Ok(reason) = from_canon::<Reason>(data) {
        let bytes =
            to_canon(&reason).expect("reason decodificado com sucesso deve ser codificavel");
        assert_eq!(bytes, data, "aceitou codificacao nao canonica para Reason");
        let roundtrip: Reason =
            from_canon(&bytes).expect("bytes codificados de Reason devem ser decodificaveis");
        assert_eq!(
            roundtrip, reason,
            "roundtrip de Reason deve preservar igualdade"
        );
    }

    if let Ok(outcome) = from_canon::<CheckOutcome>(data) {
        let bytes =
            to_canon(&outcome).expect("outcome decodificado com sucesso deve ser codificavel");
        assert_eq!(
            bytes, data,
            "aceitou codificacao nao canonica para CheckOutcome"
        );
        let roundtrip: CheckOutcome =
            from_canon(&bytes).expect("bytes codificados de CheckOutcome devem ser decodificaveis");
        assert_eq!(
            roundtrip, outcome,
            "roundtrip de CheckOutcome deve preservar igualdade"
        );
    }

    if let Ok(verdict) = from_canon::<Verdict>(data) {
        let bytes =
            to_canon(&verdict).expect("verdict decodificado com sucesso deve ser codificavel");
        assert_eq!(bytes, data, "aceitou codificacao nao canonica para Verdict");
        let roundtrip: Verdict =
            from_canon(&bytes).expect("bytes codificados de Verdict devem ser decodificaveis");
        assert_eq!(
            roundtrip, verdict,
            "roundtrip de Verdict deve preservar igualdade"
        );
    }
});
