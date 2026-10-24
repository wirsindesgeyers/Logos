"""Atestador ``clock`` em Python: espelho byte a byte do de referência em Rust.

A mesma entrada (chave, instante, referência, tolerância, ttl) produz exatamente os mesmos
bytes de fato e de vocabulário que ``logos_attest::reference::clock`` (vetor
``spec/vectors/attest/clock.json``, conferido pelos dois lados).
"""

from __future__ import annotations

from logos_attest.sdk import Attester, Observation

ID = "clock"
PROCEDURE = "clock/skew-vs-reference/v1"
TOPIC = "time"
GLOSS = "o relógio local está dentro da tolerância da referência"


class Clock(Attester):
    """Atesta ``clock_synced`` (ou a negação) pelo desvio entre ``now`` e a referência."""

    def __init__(self, reference_unix: int, tolerance_secs: int, ttl_secs: int) -> None:
        self._reference = reference_unix
        self._tolerance = tolerance_secs
        self._ttl = ttl_secs

    @property
    def id(self) -> str:
        return ID

    @property
    def topic(self) -> str:
        return TOPIC

    @property
    def procedure(self) -> str:
        return PROCEDURE

    def observe(self, now: int) -> Observation:
        skew = abs(now - self._reference)
        claim = "clock_synced()" if skew <= self._tolerance else "not(clock_synced())"
        document = (
            "vocab v1 {\n"
            f'  pred clock_synced() = anchored("{ID}", "{PROCEDURE}") gloss "{GLOSS}"\n'
            "}\n"
            f"claim observed: {claim}\n"
        )
        return Observation(
            document=document,
            version=("clock", f"skew={skew}s;tolerance={self._tolerance}s"),
            ttl_secs=self._ttl,
        )
