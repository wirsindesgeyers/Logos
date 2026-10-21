"""Entradas MCP estritas; hexadecimal transporta formatos canônicos existentes."""

from __future__ import annotations

import hashlib
from typing import Annotated, Self

from pydantic import BaseModel, ConfigDict, Field, model_validator

from logos_client import canon
from logos_provers import model_context

type DigestHex = Annotated[str, Field(min_length=64, max_length=64, pattern=r"^[0-9a-f]{64}$")]
type PublicHex = Annotated[
    str, Field(min_length=2, max_length=2_097_152, pattern=r"^(?:[0-9a-f]{2})+$")
]
type ProofHex = Annotated[
    str, Field(min_length=2, max_length=33_554_432, pattern=r"^(?:[0-9a-f]{2})+$")
]


class StrictInput(BaseModel):
    model_config = ConfigDict(strict=True, extra="forbid", frozen=True)


class ProposeInput(StrictInput):
    text_pt: Annotated[str, Field(max_length=65_536)]
    goal: PublicHex
    vocabulary: DigestHex
    cites: Annotated[list[DigestHex], Field(max_length=256)] = Field(default_factory=list)


class ObligationInput(StrictInput):
    obligation: DigestHex
    obligation_bytes: PublicHex
    vocabulary_bytes: PublicHex

    @model_validator(mode="after")
    def check_context(self) -> Self:
        obligation = bytes.fromhex(self.obligation_bytes)
        if hashlib.sha256(b"logos/obligation/v1\0" + obligation).hexdigest() != self.obligation:
            raise ValueError("obligation hash mismatch")
        vocabulary = model_context._validate_vocabulary(bytes.fromhex(self.vocabulary_bytes))
        model_context._validate_obligation(obligation, vocabulary)
        return self


class CheckInput(ObligationInput):
    certificate: ProofHex | None = None


class QueryInput(StrictInput):
    reference: DigestHex | None = None
    from_seq: Annotated[int, Field(ge=0, le=18_446_744_073_709_551_615)] | None = None
    max_events: Annotated[int, Field(ge=1, le=256)] = 64
    previous_hash: DigestHex | None = None

    @model_validator(mode="after")
    def one_query(self) -> Self:
        if self.reference is not None and self.from_seq is not None:
            raise ValueError("choose reference or from_seq")
        if self.previous_hash is not None and self.from_seq is None:
            raise ValueError("previous_hash requires from_seq")
        return self


class EmptyInput(StrictInput):
    pass


def canonical(hexadecimal: str) -> bytes:
    wire = bytes.fromhex(hexadecimal)
    canon.decode(wire)
    return wire
