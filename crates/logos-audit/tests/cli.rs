//! Integração da allowlist existente na CLI não confiável.
use std::process::Command;

#[test]
fn checker_argument_is_validated_before_opening_any_ledger() {
    for bad in ["00", &"z".repeat(64), &"é".repeat(32)] {
        let output = Command::new(env!("CARGO_BIN_EXE_logos-audit"))
            .args([
                "verify",
                "/does-not-exist",
                "--public-key",
                &"11".repeat(32),
                "--rust-checker",
                bad,
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        let error = String::from_utf8_lossy(&output.stderr);
        assert!(
            error.contains("hash de checker") || error.contains("invalid digit"),
            "{error}"
        );
        assert!(!error.contains("No such file"));
    }
}

#[test]
fn export_does_not_pretend_that_allowlist_authenticates_a_proof() {
    let output = Command::new(env!("CARGO_BIN_EXE_logos-audit"))
        .args([
            "export",
            "/does-not-exist",
            "--public-key",
            &"11".repeat(32),
            "--rust-checker",
            &"22".repeat(32),
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("uso:"));
}
