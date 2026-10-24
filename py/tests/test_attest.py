"""SDK de atestador em Python (6.3): fato assinado, validação e separação do agente."""

from __future__ import annotations

import hashlib
import os
import re
import stat
from pathlib import Path

import pytest
from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PublicKey

from logos_attest import Attester, AttestError, Ed25519Signer, Observation, attest
from logos_attest.clock import Clock
from logos_attest.sdk import FACT_TAG, SIG_TAG
from logos_client import canon

SEED = b"\x07" * 32
NOW = 1_700_000_000


class Fixed(Attester):
    """Atestador de teste com observação arbitrária."""

    def __init__(self, observation: Observation) -> None:
        self._observation = observation

    @property
    def id(self) -> str:
        return "fixed"

    @property
    def topic(self) -> str:
        return "test"

    @property
    def procedure(self) -> str:
        return "fixed/v1"

    def observe(self, now: int) -> Observation:
        return self._observation


def document(claims: str = "claim c: ok()") -> str:
    return f'vocab v1 {{ pred ok() = anchored("fixed", "fixed/v1") gloss "ok" }}\n{claims}\n'


def test_signed_fact_verifies_and_hashes_with_the_documented_tags() -> None:
    signer = Ed25519Signer.from_seed(SEED)
    attestation = attest(Clock(NOW + 2, 10, 60), signer, NOW)
    fields = canon.decode(attestation.fact)
    assert isinstance(fields, list) and len(fields) == 9
    body, signature = fields[:8], fields[8]
    assert fields[0] == 1 and fields[5] == NOW and fields[6] == [1, NOW + 60]
    assert fields[7] == [["clock", "skew=2s;tolerance=10s"]]
    assert isinstance(signature, canon.CanonMap)
    raw = dict(signature.entries)["sig"]
    assert isinstance(raw, bytes)
    # A assinatura cobre o corpo canônico com a tag de assinatura; qualquer outra tag falha.
    public = Ed25519PublicKey.from_public_bytes(signer.public_key)
    public.verify(raw, SIG_TAG + canon.encode(body))
    with pytest.raises(Exception, match=r".*"):
        public.verify(raw, b"logos/sig/other/v1\0" + canon.encode(body))
    assert attestation.fact_hash == hashlib.sha256(FACT_TAG + attestation.fact).digest()
    # O fato aponta para o vocabulário entregue junto.
    assert fields[2] == attestation.vocabulary_hash


def test_negation_when_the_clock_is_off_and_determinism() -> None:
    signer = Ed25519Signer.from_seed(SEED)
    synced = attest(Clock(NOW, 10, 60), signer, NOW)
    off = attest(Clock(NOW + 11, 10, 60), signer, NOW)
    assert synced.fact != off.fact
    assert synced.vocabulary == off.vocabulary
    assert attest(Clock(NOW, 10, 60), signer, NOW).fact == synced.fact


@pytest.mark.parametrize(
    "observation",
    [
        Observation("not an air document", None, 60),
        Observation(document("claim a: ok() claim b: ok()"), None, 60),
        Observation(document(""), None, 60),
        Observation(document(), None, 0),
        Observation(document(), None, 2**64),
        Observation(document(), ("", "v1"), 60),
        Observation(document(), ("recurso", "v" * 5000), 60),
    ],
)
def test_invalid_observations_raise_instead_of_producing_a_similar_fact(
    observation: Observation,
) -> None:
    with pytest.raises(AttestError):
        attest(Fixed(observation), Ed25519Signer.from_seed(SEED), NOW)


def test_key_file_must_be_private_and_hex(tmp_path: Path) -> None:
    path = tmp_path / "attester.key"
    path.write_text(SEED.hex())
    path.chmod(0o644)
    with pytest.raises(AttestError, match="mode 0600"):
        Ed25519Signer.from_key_file(path)
    path.chmod(0o600)
    assert Ed25519Signer.from_key_file(path).public_key == Ed25519Signer.from_seed(SEED).public_key
    path.write_text("zz")
    with pytest.raises(AttestError, match="64 hex"):
        Ed25519Signer.from_key_file(path)
    link = tmp_path / "link.key"
    link.symlink_to(path)
    assert stat.S_ISLNK(os.lstat(link).st_mode)
    with pytest.raises(AttestError):
        Ed25519Signer.from_key_file(link)
    with pytest.raises(AttestError, match="32 bytes"):
        Ed25519Signer.from_seed(b"short")


def test_write_names_files_by_hash(tmp_path: Path) -> None:
    attestation = attest(Clock(NOW, 10, 60), Ed25519Signer.from_seed(SEED), NOW)
    fact, vocabulary = attestation.write(tmp_path)
    assert fact.name == f"fact-{attestation.fact_hash.hex()}.bin"
    assert vocabulary.name == f"vocabulary-{attestation.vocabulary_hash.hex()}.bin"
    assert fact.read_bytes() == attestation.fact
    assert vocabulary.read_bytes() == attestation.vocabulary


def test_the_agent_never_imports_the_attester_sdk() -> None:
    """A chave do atestador vive fora do agente: nenhum módulo do agente/MCP importa o SDK."""
    root = Path(__file__).resolve().parent.parent
    pattern = re.compile(r"^\s*(?:from|import)\s+logos_attest\b", re.MULTILINE)
    for package in ("logos_agent", "logos_mcp", "logos_provers"):
        for source in (root / package).rglob("*.py"):
            assert not pattern.search(source.read_text(encoding="utf-8")), source
