//! event/v1 + corpos v2: preservar bytes canônicos e invariantes.
#![no_main]

use libfuzzer_sys::fuzz_target;
use logos_canon::{Canon, CanonDecode, from_canon, to_canon};
use logos_ledger::{
    Authority, CheckRecord, Consistency, Event, EventBody, EventProposal, MAX_EVENT_BYTES, Route,
    TaintedText,
};

fn roundtrip<T: Canon + CanonDecode + PartialEq + core::fmt::Debug>(data: &[u8]) {
    if let Ok(value) = from_canon::<T>(data) {
        let wire = to_canon(&value).expect("decoded event object must encode");
        assert_eq!(wire, data, "accepted noncanonical event object");
        assert_eq!(
            from_canon::<T>(&wire).expect("encoded object must decode"),
            value
        );
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_EVENT_BYTES {
        return;
    }
    roundtrip::<Event>(data);
    roundtrip::<EventBody>(data);
    roundtrip::<CheckRecord>(data);
    roundtrip::<Consistency>(data);
    roundtrip::<EventProposal>(data);
    roundtrip::<Authority>(data);
    roundtrip::<TaintedText>(data);
    roundtrip::<Route>(data);
    if let Ok(event) = Event::decode(data) {
        // Seeds de envelope também exercitam o decoder isolado do corpo v2.
        roundtrip::<EventBody>(&to_canon(&event.body).expect("body encodes"));
        let refs = event.body.cas_references();
        if let EventBody::CertificateRejected { check } = &event.body {
            assert!(matches!(check.outcome, logos_core::CheckOutcome::Rejected(_)));
            assert_eq!(refs.len(), 3);
        }
        if let EventBody::ObligationCreated {
            cnf: Some(c),
            route,
            ..
        } = &event.body
        {
            assert_eq!(*route, Route::Lrat);
            assert_eq!(c.encoder_version, 1);
            assert!(refs.contains(&logos_ledger::CasReference::Cnf(c.hash)));
        }
        let body: EventBody =
            from_canon(&to_canon(&event.body).expect("body encodes")).expect("body decodes");
        assert_eq!(refs, body.cas_references());
        event
            .body
            .validate()
            .expect("decoded body must satisfy catalog invariants");
        assert_eq!(to_canon(&event).expect("decoded event encodes"), data);
    }
});
