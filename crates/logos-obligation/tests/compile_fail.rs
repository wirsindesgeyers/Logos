//! Testes de fronteira de compilação via trybuild para Obligation.
//!
//! Garante que a única construção pública de `Obligation` é via `generate`,
//! impedindo instanciação literal direta, decodificação canônica arbitrária,
//! valores Default e conversões From/TryFrom a partir de dados externos.

#[test]
fn compile_fail_ui() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/struct_literal.rs");
    t.compile_fail("tests/ui/canon_decode.rs");
    t.compile_fail("tests/ui/no_default.rs");
    t.compile_fail("tests/ui/from_conversion.rs");
    t.compile_fail("tests/ui/try_from_conversion.rs");
    t.pass("tests/ui/positive_control.rs");
}
