//! Testes de fronteira de compilação via trybuild.

#[test]
fn compile_fail_ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/direct_assignment.rs");
    t.compile_fail("tests/ui/into_conversion_rejected.rs");
    t.compile_fail("tests/ui/into_conversion_unknown.rs");
    t.compile_fail("tests/ui/try_from_rejected.rs");
    t.compile_fail("tests/ui/try_from_unknown.rs");
    t.compile_fail("tests/ui/result_not_verdict.rs");
    t.compile_fail("tests/ui/meter_swap.rs");
    t.pass("tests/ui/positive_control.rs");
}
