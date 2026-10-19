//! Fronteira pública: dados externos não constroem claims nem mudam o estado.

#[test]
fn compile_fail_ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/claim_literal.rs");
    t.compile_fail("tests/ui/receipt_literal.rs");
    t.compile_fail("tests/ui/no_constructor.rs");
    t.compile_fail("tests/ui/refuted_to_verified.rs");
    t.compile_fail("tests/ui/refuted_accept.rs");
    t.compile_fail("tests/ui/unknown_to_verified.rs");
    t.compile_fail("tests/ui/pending_retag.rs");
    t.compile_fail("tests/ui/raw_outcome.rs");
    t.compile_fail("tests/ui/raw_event.rs");
    t.compile_fail("tests/ui/no_clone.rs");
    t.compile_fail("tests/ui/no_default.rs");
    t.compile_fail("tests/ui/consumed_claim.rs");
    t.compile_fail("tests/ui/live_index_blocks_append.rs");
    t.compile_fail("tests/ui/live_index_blocks_drop.rs");
    t.pass("tests/ui/positive_control.rs");
    t.pass("tests/ui/live_index_positive.rs");
}
