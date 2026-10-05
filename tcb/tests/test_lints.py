"""Regressões do guard: conteúdo comentado/literal não é atributo de crate."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import check_lints
import pytest

FORBID = "#![forbid(unsafe_code)]\n"
DENY = "#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)]\n"


@pytest.mark.parametrize(
    "source",
    [
        "// " + FORBID + "// " + DENY,
        "/*\n" + FORBID + DENY + "*/\n",
        "/* externo /* interno */\n" + FORBID + DENY + "*/\n",
        '#![doc = r##"' + FORBID + DENY + '"##]\n',
        '#![doc = "#![forbid(unsafe_code)]\\n' + DENY.rstrip() + '"]\n',
        'pub const FAKE: &[u8] = br#"' + FORBID + DENY + '"#;\n',
        'pub const FAKE: &core::ffi::CStr = cr#"' + FORBID + DENY + '"#;\n',
        "mod inner {\n" + FORBID + DENY + "}\n",
        "#![cfg_attr(any(), forbid(unsafe_code))]\n"
        "#![cfg_attr(any(), deny(clippy::unwrap_used, clippy::expect_used, clippy::panic))]\n",
    ],
    ids=[
        "line",
        "block",
        "nested-block",
        "raw-doc",
        "string-doc",
        "byte-doc",
        "c-doc",
        "module",
        "conditional",
    ],
)
def test_fake_attributes_do_not_satisfy_crate_lints(source: str) -> None:
    assert check_lints.missing_lints(source) == (True, check_lints.CLIPPY_LINTS)


def test_nested_deny_does_not_complete_root_forbid() -> None:
    source = FORBID + "#[cfg(test)] mod tests {\n" + DENY + "}\n"
    assert check_lints.missing_lints(source) == (False, check_lints.CLIPPY_LINTS)


def test_active_multiline_split_attributes_and_trivia() -> None:
    source = "\ufeff//! Documentação de crate\n/*\n" + DENY + "*/\n"
    source += "# ! [ forbid ( unsafe_code ) ]\n"
    source += "#![deny(\nclippy::panic,\nclippy::expect_used,\n)]\n"
    source += '#![deny(clippy::unwrap_used, reason = "texto /* opaco */ #[deny(fake)]")]\n'
    source += "pub fn item() {}\n"
    assert check_lints.missing_lints(source) == (False, frozenset())


def test_forbid_clippy_is_stronger_than_deny() -> None:
    assert check_lints.missing_lints(FORBID + DENY.replace("deny", "forbid")) == (
        False,
        frozenset(),
    )


def test_later_allow_cancels_deny() -> None:
    source = FORBID + DENY + "#![allow(clippy::unwrap_used)]\n"
    assert check_lints.missing_lints(source) == (False, frozenset({"clippy::unwrap_used"}))


def test_related_lint_name_is_not_required_lint() -> None:
    source = FORBID + DENY.replace("clippy::panic)", "clippy::panic_in_result_fn)")
    assert check_lints.missing_lints(source) == (False, frozenset({"clippy::panic"}))


def test_guard_command_uses_strict_manifest_entries(tmp_path: Path) -> None:
    (tmp_path / "tcb").mkdir()
    (tmp_path / "tcb" / "manifest.toml").write_text(
        '[[crate]]\nname="strict"\nstrict=true\n[[crate]]\nname="helper"\nstrict=false\n',
        encoding="utf-8",
    )
    strict = tmp_path / "crates" / "strict" / "src" / "lib.rs"
    strict.parent.mkdir(parents=True)
    strict.write_text(FORBID + "// " + DENY, encoding="utf-8")
    command = [sys.executable, check_lints.__file__, "--root", str(tmp_path)]
    blocked = subprocess.run(command, capture_output=True, check=False)
    assert blocked.returncode == 1
    strict.write_text(FORBID + DENY + "pub fn item() {}\n", encoding="utf-8")
    allowed = subprocess.run(command, capture_output=True, check=False)
    assert allowed.returncode == 0, allowed.stderr.decode()
