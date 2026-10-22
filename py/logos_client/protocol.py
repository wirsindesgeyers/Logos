"""kernel-protocol/v1: candidatos e observações não confiáveis, nunca credenciais."""

from __future__ import annotations

import hashlib
from typing import Annotated, Literal, Self

from pydantic import AfterValidator, BaseModel, ConfigDict, Field, model_validator

from logos_client import air, canon

MAX_REQUEST = 2 * 1024 * 1024
MAX_RESPONSE = 16 * 1024 * 1024 + 1024
MAX_AIR = 1024 * 1024
U64_MAX = 2**64 - 1
Digest = Annotated[bytes, Field(min_length=32, max_length=32)]
U64 = Annotated[int, Field(ge=0, le=U64_MAX)]
References = Annotated[tuple[Digest, ...], Field(max_length=1024)]


class ProtocolError(ValueError):
    """Resposta/framing inválido; não indica verdict lógico."""


class Model(BaseModel):
    model_config = ConfigDict(
        strict=True, frozen=True, extra="forbid", revalidate_instances="always"
    )


def _air(data: bytes) -> bytes:
    if len(data) > MAX_AIR:
        raise ValueError("AIR exceeds 1 MiB")
    try:
        value = canon.decode(data)
        air.print_formula(value)
    except (canon.CanonError, air.SurfaceError) as error:
        raise ValueError("invalid AIR") from error
    return data


def _text(value: str, limit: int, *, tainted: bool = False) -> str:
    if len(value.encode("utf-8")) > limit:
        raise ValueError("text exceeds byte limit")
    if not tainted:
        try:
            canon.check_text(value)
        except canon.CanonError as error:
            raise ValueError("noncanonical text") from error
    return value


AirBytes = Annotated[bytes, AfterValidator(_air)]
Scope = Annotated[str, AfterValidator(lambda s: _text(s, 65536))]
Tainted = Annotated[str, AfterValidator(lambda s: _text(s, 65536, tainted=True))]
Format = Annotated[str, Field(min_length=1), AfterValidator(lambda s: _text(s, 256))]


class CasRef(Model):
    kind: Literal["cas"] = "cas"
    hash: Digest


class StagedRef(Model):
    kind: Literal["staged"] = "staged"
    hash: Digest
    name: Annotated[str, Field(min_length=64, max_length=64, pattern=r"^[0-9a-f]{64}$")]


type ObjectRef = CasRef | StagedRef


class Propose(Model):
    kind: Literal["propose"] = "propose"
    text_pt: Tainted
    goal: AirBytes
    vocabulary: Digest
    cites: References = ()
    speculative_on: References = ()


class Certify(Model):
    kind: Literal["certify"] = "certify"
    obligation: Digest
    format: Format
    certificate: ObjectRef


class Refute(Model):
    kind: Literal["refute"] = "refute"
    obligation: Digest
    model: ObjectRef


class Witness(Model):
    kind: Literal["witness"] = "witness"
    obligation: Digest
    model: ObjectRef


class Assume(Model):
    kind: Literal["assume"] = "assume"
    vocabulary: Digest
    formula: AirBytes
    scope: Scope


class QueryHead(Model):
    kind: Literal["head"] = "head"
    pass


class QueryStatus(Model):
    kind: Literal["status"] = "status"
    reference: Digest


class QueryOpen(Model):
    """Assumptions abertas no fecho used de um claim (5.7)."""

    kind: Literal["open"] = "open"
    reference: Digest


class Subscribe(Model):
    kind: Literal["subscribe"] = "subscribe"
    from_seq: U64
    max_events: Annotated[int, Field(ge=1, le=256)] = 256


type Message = Annotated[
    Propose | Certify | Refute | Witness | Assume | QueryHead | QueryStatus | QueryOpen | Subscribe,
    Field(discriminator="kind"),
]


class Request(Model):
    id: U64
    message: Message


def array(value: canon.Value, length: int | None = None) -> list[canon.Value]:
    if not isinstance(value, list) or (length is not None and len(value) != length):
        raise ProtocolError("array type/arity")
    return value


def uint(value: canon.Value, maximum: int = U64_MAX) -> int:
    if type(value) is not int or not 0 <= value <= maximum:
        raise ProtocolError("unsigned integer type/range")
    return value


def digest(value: canon.Value) -> bytes:
    if not isinstance(value, bytes) or len(value) != 32:
        raise ProtocolError("expected digest bytes32")
    return value


def object_value(reference: ObjectRef) -> canon.Value:
    if isinstance(reference, StagedRef):
        return [1, reference.name, reference.hash]
    return [0, reference.hash]


def encode_request(request: Request) -> bytes:
    # Revalidate even model_construct/model_copy inputs; no coercion or normalization.
    request = Request.model_validate(request.model_dump())
    m = request.message
    fields: list[canon.Value]
    if isinstance(m, Propose):
        tag, fields = (
            0,
            [m.text_pt.encode(), m.goal, list(m.cites), list(m.speculative_on), m.vocabulary],
        )
    elif isinstance(m, Certify):
        tag, fields = 1, [m.obligation, m.format, object_value(m.certificate)]
    elif isinstance(m, Refute | Witness):
        tag = 2 if isinstance(m, Refute) else 3
        fields = [m.obligation, object_value(m.model)]
    elif isinstance(m, Assume):
        tag, fields = 4, [m.vocabulary, m.formula, m.scope]
    elif isinstance(m, QueryHead):
        tag, fields = 5, [0]
    elif isinstance(m, QueryStatus):
        tag, fields = 5, [1, m.reference]
    elif isinstance(m, QueryOpen):
        tag, fields = 5, [2, m.reference]
    else:
        tag, fields = 6, [m.from_seq, m.max_events]
    payload = canon.encode([1, request.id, tag, fields])
    if len(payload) > MAX_REQUEST:
        raise ProtocolError("request exceeds frame limit")
    return len(payload).to_bytes(4, "big") + payload


def _object(value: canon.Value) -> ObjectRef:
    fields = array(value)
    if len(fields) == 2 and uint(fields[0]) == 0:
        return CasRef(hash=digest(fields[1]))
    if len(fields) == 3 and uint(fields[0]) == 1 and isinstance(fields[1], str):
        return StagedRef(hash=digest(fields[2]), name=fields[1])
    raise ProtocolError("object reference")


def decode_request(payload: bytes) -> Request:
    if not 0 < len(payload) <= MAX_REQUEST:
        raise ProtocolError("request frame size")
    v = array(canon.decode(payload), 4)
    if uint(v[0]) != 1:
        raise ProtocolError("request version")
    request_id, tag, f = uint(v[1]), uint(v[2]), array(v[3])
    message: Message
    if tag == 0 and len(f) == 5 and isinstance(f[0], bytes) and isinstance(f[1], bytes):
        message = Propose(
            text_pt=f[0].decode("utf-8"),
            goal=f[1],
            cites=tuple(digest(x) for x in array(f[2])),
            speculative_on=tuple(digest(x) for x in array(f[3])),
            vocabulary=digest(f[4]),
        )
    elif tag == 1 and len(f) == 3 and isinstance(f[1], str):
        message = Certify(obligation=digest(f[0]), format=f[1], certificate=_object(f[2]))
    elif tag in (2, 3) and len(f) == 2:
        cls = Refute if tag == 2 else Witness
        message = cls(obligation=digest(f[0]), model=_object(f[1]))
    elif tag == 4 and len(f) == 3 and isinstance(f[1], bytes) and isinstance(f[2], str):
        message = Assume(vocabulary=digest(f[0]), formula=f[1], scope=f[2])
    elif tag == 5 and len(f) == 1 and uint(f[0]) == 0:
        message = QueryHead()
    elif tag == 5 and len(f) == 2 and uint(f[0]) == 1:
        message = QueryStatus(reference=digest(f[1]))
    elif tag == 5 and len(f) == 2 and uint(f[0]) == 2:
        message = QueryOpen(reference=digest(f[1]))
    elif tag == 6 and len(f) == 2:
        message = Subscribe(from_seq=uint(f[0]), max_events=uint(f[1], 256))
    else:
        raise ProtocolError("unknown message/type/arity")
    return Request(id=request_id, message=message)


class ProposeResult(Model):
    obligation: Digest
    obligation_bytes: Annotated[bytes, Field(max_length=MAX_AIR)]

    @model_validator(mode="after")
    def check_hash(self) -> Self:
        if (
            hashlib.sha256(b"logos/obligation/v1\0" + self.obligation_bytes).digest()
            != self.obligation
        ):
            raise ValueError("obligation hash mismatch")
        canon.decode(self.obligation_bytes)
        return self


class CheckResult(Model):
    state: Literal["verified", "refuted", "rejected_pending", "unknown"]
    event: Digest


class WitnessResult(Model):
    outcome: Literal["accepted", "rejected", "unknown"]
    event: Digest


class AssumeResult(Model):
    """assume-result/v1: evento AssumptionDeclared (autor = kernel, origem agente)."""

    event: Digest


class OpenAssumption(Model):
    event: Digest
    scope: str


class OpenResult(Model):
    """open-result/v1: lista vazia = claim incondicional."""

    assumptions: list[OpenAssumption]


class Head(Model):
    seq: U64
    epoch: U64
    hash: Digest


class Status(Model):
    kind: Annotated[int, Field(ge=0, le=9)]
    epoch: U64
    retracted: bool


class EventRecord(Model):
    """Envelope observado; hash/links conferidos não autenticam a assinatura."""

    seq: U64
    epoch: U64
    previous: Digest | None
    hash: Digest
    canonical_bytes: bytes


class SubscriptionEnd(Model):
    next_seq: U64


def parse_event(value: canon.Value) -> EventRecord:
    v = array(value, 8)
    if uint(v[0]) != 1 or uint(v[1]) != 0:
        raise ProtocolError("event envelope version/tag")
    previous = array(v[4])
    if len(previous) > 1:
        raise ProtocolError("event previous")
    array(v[5])
    digest(v[6])
    if not isinstance(v[7], canon.CanonMap):
        raise ProtocolError("event signature envelope")
    wire = canon.encode(value)
    return EventRecord(
        seq=uint(v[2]),
        epoch=uint(v[3]),
        previous=None if not previous else digest(previous[0]),
        hash=hashlib.sha256(b"logos/event/v1\0" + wire).digest(),
        canonical_bytes=wire,
    )


type Result = (
    ProposeResult | CheckResult | WitnessResult | AssumeResult | OpenResult | Head | Status | None
)


def parse_result(message: Message, value: canon.Value) -> Result:
    f = array(value)
    if isinstance(message, QueryHead):
        if not f:
            return None
        f = array(value, 3)
        return Head(seq=uint(f[0]), epoch=uint(f[1]), hash=digest(f[2]))
    if isinstance(message, QueryStatus):
        if not f:
            return None
        f = array(value, 3)
        if type(f[2]) is not bool:
            raise ProtocolError("status boolean")
        return Status(kind=uint(f[0], 9), epoch=uint(f[1]), retracted=f[2])
    if isinstance(message, QueryOpen):
        if not f:
            return None
        f = array(value, 2)
        if uint(f[0]) != 1:
            raise ProtocolError("open result version")
        entries = []
        for entry in array(f[1]):
            pair = array(entry, 2)
            if not isinstance(pair[1], str):
                raise ProtocolError("open result scope")
            entries.append(OpenAssumption(event=digest(pair[0]), scope=pair[1]))
        if [e.event for e in entries] != sorted(e.event for e in entries):
            raise ProtocolError("open result order")
        return OpenResult(assumptions=entries)
    if isinstance(message, Assume):
        f = array(value, 2)
        if uint(f[0]) != 1:
            raise ProtocolError("assume result version")
        return AssumeResult(event=digest(f[1]))
    f = array(value, 3)
    if uint(f[0]) != 1:
        raise ProtocolError("result version")
    if isinstance(message, Propose) and isinstance(f[2], bytes):
        return ProposeResult(obligation=digest(f[1]), obligation_bytes=f[2])
    if isinstance(message, Certify | Refute):
        tag = uint(f[1], 3)
        if (isinstance(message, Certify) and tag == 1) or (
            isinstance(message, Refute) and tag == 0
        ):
            raise ProtocolError("result incompatible with request")
        return CheckResult(
            state=("verified", "refuted", "rejected_pending", "unknown")[tag], event=digest(f[2])
        )
    if isinstance(message, Witness):
        tag = uint(f[1], 2)
        return WitnessResult(outcome=("accepted", "rejected", "unknown")[tag], event=digest(f[2]))
    raise ProtocolError("unsupported success result")
