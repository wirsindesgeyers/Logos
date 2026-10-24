"""Observação → fato fact/v1 assinado (spec/fact.md), com canon e Ed25519 de referência."""

from __future__ import annotations

import hashlib
import os
import stat
from abc import ABC, abstractmethod
from dataclasses import dataclass
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from logos_client import air, canon
from logos_client.canon import Value

SIG_TAG = b"logos/sig/fact/v1\0"
FACT_TAG = b"logos/fact/v1\0"
VOCABULARY_TAG = b"logos/vocab/v1\0"
MAX_TEXT = 4096


class AttestError(Exception):
    """Observação inválida ou chave inutilizável; nunca vira um fato "parecido"."""


@dataclass(frozen=True)
class Observation:
    """O que o atestador observou, em sintaxe de superfície AIR (air-text/v1).

    ``document`` tem um vocabulário e exatamente um ``claim``; a fórmula do claim é a
    afirmação do fato. ``version`` é o par recurso/versão observado, se houver.
    """

    document: str
    version: tuple[str, str] | None
    ttl_secs: int


class Attester(ABC):
    """Contrato de um atestador: identidade, tópico, procedimento e ``observe``."""

    @property
    @abstractmethod
    def id(self) -> str:
        """``AttestorId`` dos símbolos ``anchored``."""

    @property
    @abstractmethod
    def topic(self) -> str:
        """Tópico atestado (a confiança é por tópico)."""

    @property
    @abstractmethod
    def procedure(self) -> str:
        """Identificador do procedimento, descrito em ``docs/attesters/<id>.md``."""

    @abstractmethod
    def observe(self, now: int) -> Observation:
        """Observa agora (``now`` = instante Unix em segundos); falha em vez de adivinhar."""


class Ed25519Signer:
    """Chave Ed25519 do atestador. Fica no processo do atestador, nunca no agente."""

    def __init__(self, private: Ed25519PrivateKey) -> None:
        self._private = private

    @classmethod
    def from_seed(cls, seed: bytes) -> Ed25519Signer:
        if len(seed) != 32:
            raise AttestError("seed must have 32 bytes")
        return cls(Ed25519PrivateKey.from_private_bytes(seed))

    @classmethod
    def from_key_file(cls, path: Path) -> Ed25519Signer:
        """Mesmo formato do kernel: 64 dígitos hex, arquivo regular modo 0600 do próprio usuário."""
        info = os.lstat(path)
        if not stat.S_ISREG(info.st_mode) or info.st_mode & 0o077 or info.st_uid != os.geteuid():
            raise AttestError("key file must be a regular file of mode 0600 owned by the user")
        text = path.read_text(encoding="ascii").strip()
        try:
            return cls.from_seed(bytes.fromhex(text))
        except ValueError as error:
            raise AttestError("key file must hold 64 hex digits") from error

    @property
    def public_key(self) -> bytes:
        return self._private.public_key().public_bytes_raw()

    def sign(self, message: bytes) -> bytes:
        return self._private.sign(message)


@dataclass(frozen=True)
class Attestation:
    """Fato assinado e vocabulário, ambos em canon, mais os hashes de identidade."""

    fact: bytes
    vocabulary: bytes

    @property
    def fact_hash(self) -> bytes:
        return hashlib.sha256(FACT_TAG + self.fact).digest()

    @property
    def vocabulary_hash(self) -> bytes:
        return hashlib.sha256(VOCABULARY_TAG + self.vocabulary).digest()

    def write(self, directory: Path) -> tuple[Path, Path]:
        """Grava ``fact-<hash>.bin`` e ``vocabulary-<hash>.bin`` para o operador registrar."""
        fact = directory / f"fact-{self.fact_hash.hex()}.bin"
        vocabulary = directory / f"vocabulary-{self.vocabulary_hash.hex()}.bin"
        fact.write_bytes(self.fact)
        vocabulary.write_bytes(self.vocabulary)
        return fact, vocabulary


def _text(value: str, what: str) -> str:
    if not value or len(value.encode()) > MAX_TEXT:
        raise AttestError(f"{what} must be non-empty text up to {MAX_TEXT} bytes")
    return value


def attest(attester: Attester, signer: Ed25519Signer, now: int) -> Attestation:
    """Observa e assina. A chave do atestador é a única autoridade do fato."""
    observation = attester.observe(now)
    try:
        document = air.parse_document(observation.document)
    except air.SurfaceError as error:
        raise AttestError(f"observation is not valid AIR: {error}") from error
    if len(document.claims) != 1:
        raise AttestError("an observation must contain exactly one claim")
    if observation.ttl_secs <= 0 or now + observation.ttl_secs >= 2**64:
        raise AttestError("ttl must be positive and keep the validity inside u64")
    version: list[Value] = []
    if observation.version is not None:
        version = [
            [_text(observation.version[0], "resource"), _text(observation.version[1], "version")]
        ]
    vocabulary = canon.encode(document.vocabulary)
    issuer = canon.CanonMap([("alg", "ed25519"), ("key", signer.public_key)])
    body: list[Value] = [
        1,
        document.claims[0].formula,
        hashlib.sha256(VOCABULARY_TAG + vocabulary).digest(),
        issuer,
        _text(attester.topic, "topic"),
        now,
        [1, now + observation.ttl_secs],
        version,
    ]
    signature = canon.CanonMap(
        [("alg", "ed25519"), ("sig", signer.sign(SIG_TAG + canon.encode(body)))]
    )
    return Attestation(fact=canon.encode([*body, signature]), vocabulary=vocabulary)
