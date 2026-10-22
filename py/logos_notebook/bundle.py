"""notebook/v1: bundle canônico local do caderno (spec/notebook.md); fora da TCB.

O bundle guarda formulações, proveniência e **referências** a eventos do
ledger. Não guarda status como atual: `status_hint` é histórico rotulado. Um
bundle válido não prova nada; o decoder só confere forma e limites.
"""

from __future__ import annotations

import hashlib
import itertools
from dataclasses import dataclass, field
from enum import IntEnum

from logos_client import air, canon

VERSION = 1
TAG = "logos/notebook"
HASH_DOMAIN = b"logos/notebook/v1\0"
MAX_BUNDLE = 16 * 1024 * 1024
MAX_TEXT = 65_536
MAX_ARTIFACTS = 4096
MAX_RELATIONS = 16_384
MAX_REFS = 1024
MAX_LIMITATIONS = 64
MAX_ATTACHMENT = 4 * 1024 * 1024
SCOPE_PREFIXES = ("session:", "until:", "claim:")


class NotebookError(ValueError):
    """Bundle fora do formato notebook/v1; nunca é corrigido silenciosamente."""


class Kind(IntEnum):
    QUESTION = 0
    HYPOTHESIS = 1
    PREDICTION = 2
    EVIDENCE = 3
    ASSUMPTION = 4
    CONCLUSION = 5
    CANDIDATE_LAW = 6
    DATA_TABLE = 7


class Nature(IntEnum):
    """Como a afirmação entrou no caderno (WS§11.1); não é status lógico."""

    HYPOTHESIS = 0
    EMPIRICAL_EVIDENCE = 1
    SOURCE_SAYS = 2
    MODEL_SUGGESTS = 3
    USER_SAVED = 4
    DERIVATION_REQUEST = 5


class Role(IntEnum):
    ASSUMPTION = 0  # AssumptionDeclared (ASSUME 5.7)
    CHECK = 1  # evento de CERTIFY/REFUTE sobre h(O)


class Relation(IntEnum):
    PREDICTS = 0
    TESTS = 1
    DERIVED_FROM = 2
    ALTERNATIVE_TO = 3
    CONFLICTS_WITH = 4
    OBSERVED_IN = 5


# Tipos que podem carregar cada papel. Evidência só entra como assumption
# explícita (ou como nota sem referência): nunca como resultado de checker.
ALLOWED_ROLES: dict[Kind, frozenset[Role]] = {
    Kind.QUESTION: frozenset(),
    Kind.HYPOTHESIS: frozenset({Role.ASSUMPTION}),
    Kind.PREDICTION: frozenset({Role.CHECK}),
    Kind.EVIDENCE: frozenset({Role.ASSUMPTION}),
    Kind.ASSUMPTION: frozenset({Role.ASSUMPTION}),
    Kind.CONCLUSION: frozenset({Role.CHECK}),
    Kind.CANDIDATE_LAW: frozenset({Role.ASSUMPTION, Role.CHECK}),
    Kind.DATA_TABLE: frozenset(),
}


@dataclass(frozen=True)
class Provenance:
    source: str
    locator: str
    method: str
    actor: str
    recorded_at: int

    def complete(self) -> bool:
        return all((self.source, self.locator, self.method, self.actor)) and self.recorded_at > 0


@dataclass(frozen=True)
class ClaimRef:
    role: Role
    event: bytes
    obligation: bytes | None = None


@dataclass(frozen=True)
class Attachment:
    media_type: str
    data: bytes

    @property
    def sha256(self) -> bytes:
        return hashlib.sha256(self.data).digest()


@dataclass(frozen=True)
class StatusHint:
    """Status visto em `seq`; histórico, nunca exibido como atual (W1)."""

    seq: int
    label: str


@dataclass
class Artifact:
    id: bytes
    kind: Kind
    nature: Nature
    title: str
    statement_pt: str
    provenance: Provenance
    formula: canon.Value = None
    revision: int = 1
    claim_refs: list[ClaimRef] = field(default_factory=list)
    limitations: list[str] = field(default_factory=list)
    attachment: Attachment | None = None
    status_hint: StatusHint | None = None


@dataclass(frozen=True)
class Link:
    source: bytes
    relation: Relation
    target: bytes


@dataclass(frozen=True)
class Project:
    id: bytes
    title: str
    scope: str
    vocabulary: bytes
    revision: int = 1


@dataclass
class Notebook:
    project: Project
    artifacts: list[Artifact] = field(default_factory=list)
    links: set[Link] = field(default_factory=set)

    def get(self, artifact_id: bytes) -> Artifact:
        for artifact in self.artifacts:
            if artifact.id == artifact_id:
                return artifact
        raise KeyError(artifact_id.hex())

    def encode(self) -> bytes:
        value = _encode_notebook(self)
        data = canon.encode(value)
        if len(data) > MAX_BUNDLE:
            raise NotebookError("bundle size limit")
        # Releitura estrita: o encoder não emite o que o decoder recusaria.
        decode(data)
        return data

    def digest(self) -> bytes:
        return hashlib.sha256(HASH_DOMAIN + self.encode()).digest()


def new_id(*parts: bytes | str) -> bytes:
    """ID de 16 bytes derivado de partes explícitas (determinístico, não secreto)."""
    hasher = hashlib.sha256(b"logos/notebook-id/v1\0")
    for part in parts:
        data = part.encode() if isinstance(part, str) else part
        hasher.update(len(data).to_bytes(8, "big") + data)
    return hasher.digest()[:16]


def _text(value: str, what: str, *, empty: bool = True) -> str:
    if not isinstance(value, str):
        raise NotebookError(f"{what} must be text")
    if len(value.encode()) > MAX_TEXT or (not empty and not value):
        raise NotebookError(f"{what} length")
    try:
        canon.check_text(value)
    except canon.CanonError as error:
        raise NotebookError(f"{what}: {error}") from error
    return value


def _uint(value: canon.Value, what: str, maximum: int = 2**64 - 1) -> int:
    if type(value) is not int or not 0 <= value <= maximum:
        raise NotebookError(f"{what} must be an unsigned integer")
    return value


def _bytes(value: canon.Value, size: int, what: str) -> bytes:
    if not isinstance(value, bytes) or len(value) != size:
        raise NotebookError(f"{what} must have {size} bytes")
    return value


def _list(value: canon.Value, what: str, length: int | None = None) -> list[canon.Value]:
    if not isinstance(value, list) or (length is not None and len(value) != length):
        raise NotebookError(f"{what} arity")
    return value


def _formula(value: canon.Value) -> canon.Value:
    if value is None:
        return None
    try:
        air.print_formula(value)
    except (air.SurfaceError, ValueError, TypeError, IndexError) as error:
        raise NotebookError(f"invalid AIR formula: {error}") from error
    return value


def _encode_notebook(notebook: Notebook) -> canon.Value:
    project = notebook.project
    artifacts: list[canon.Value] = []
    for item in notebook.artifacts:
        artifacts.append(
            [
                item.id,
                int(item.kind),
                int(item.nature),
                item.revision,
                item.title,
                item.statement_pt,
                item.formula,
                [
                    item.provenance.source,
                    item.provenance.locator,
                    item.provenance.method,
                    item.provenance.actor,
                    item.provenance.recorded_at,
                ],
                [[int(r.role), r.event, r.obligation] for r in item.claim_refs],
                list(item.limitations),
                None
                if item.attachment is None
                else [item.attachment.media_type, item.attachment.data],
                None
                if item.status_hint is None
                else [item.status_hint.seq, item.status_hint.label],
            ]
        )
    links: list[canon.Value] = [
        [link.source, int(link.relation), link.target]
        for link in sorted(notebook.links, key=lambda x: (x.source, int(x.relation), x.target))
    ]
    return [
        VERSION,
        TAG,
        [project.id, project.title, project.scope, project.vocabulary, project.revision],
        artifacts,
        links,
    ]


def _decode_artifact(value: canon.Value) -> Artifact:
    f = _list(value, "artifact", 12)
    try:
        kind, nature = Kind(_uint(f[1], "kind")), Nature(_uint(f[2], "nature"))
    except ValueError as error:
        raise NotebookError(f"unknown artifact kind/nature: {error}") from error
    p = _list(f[7], "provenance", 5)
    provenance = Provenance(
        _text(p[0], "source"),  # type: ignore[arg-type]
        _text(p[1], "locator"),  # type: ignore[arg-type]
        _text(p[2], "method"),  # type: ignore[arg-type]
        _text(p[3], "actor"),  # type: ignore[arg-type]
        _uint(p[4], "recorded_at"),
    )
    refs: list[ClaimRef] = []
    raw_refs = _list(f[8], "claim refs")
    if len(raw_refs) > MAX_REFS:
        raise NotebookError("claim refs limit")
    for raw in raw_refs:
        r = _list(raw, "claim ref", 3)
        try:
            role = Role(_uint(r[0], "role"))
        except ValueError as error:
            raise NotebookError("unknown claim ref role") from error
        if role not in ALLOWED_ROLES[kind]:
            raise NotebookError(f"{kind.name} cannot carry a {role.name} reference")
        obligation = None if r[2] is None else _bytes(r[2], 32, "obligation")
        if (role == Role.CHECK) != (obligation is not None):
            raise NotebookError("CHECK refs need h(O); ASSUMPTION refs have none")
        refs.append(ClaimRef(role, _bytes(r[1], 32, "event"), obligation))
    limitations = _list(f[9], "limitations")
    if len(limitations) > MAX_LIMITATIONS:
        raise NotebookError("limitations limit")
    attachment = None
    if f[10] is not None:
        a = _list(f[10], "attachment", 2)
        if kind != Kind.DATA_TABLE:
            raise NotebookError("only DATA_TABLE carries an attachment")
        data = a[1]
        if not isinstance(data, bytes) or len(data) > MAX_ATTACHMENT:
            raise NotebookError("attachment bytes")
        attachment = Attachment(_text(a[0], "media type", empty=False), data)  # type: ignore[arg-type]
    hint = None
    if f[11] is not None:
        h = _list(f[11], "status hint", 2)
        hint = StatusHint(_uint(h[0], "hint seq"), _text(h[1], "hint label", empty=False))  # type: ignore[arg-type]
    formula = _formula(f[6])
    if kind in (Kind.PREDICTION, Kind.CONCLUSION, Kind.CANDIDATE_LAW) and formula is None:
        raise NotebookError(f"{kind.name} needs an AIR formula")
    if any(ref.role == Role.ASSUMPTION for ref in refs) and formula is None:
        raise NotebookError("assumption reference needs the declared formula")
    return Artifact(
        id=_bytes(f[0], 16, "artifact id"),
        kind=kind,
        nature=nature,
        revision=_uint(f[3], "revision"),
        title=_text(f[4], "title", empty=False),  # type: ignore[arg-type]
        statement_pt=_text(f[5], "statement"),  # type: ignore[arg-type]
        formula=formula,
        provenance=provenance,
        claim_refs=refs,
        limitations=[_text(item, "limitation", empty=False) for item in limitations],  # type: ignore[arg-type]
        attachment=attachment,
        status_hint=hint,
    )


def decode(data: bytes) -> Notebook:
    """Decoder estrito: aridade, tipos, limites, IDs únicos e relações ordenadas."""
    if not isinstance(data, bytes) or len(data) > MAX_BUNDLE:
        raise NotebookError("bundle size limit")
    try:
        value = canon.decode(data)
    except canon.CanonError as error:
        raise NotebookError(f"not canonical: {error}") from error
    f = _list(value, "bundle", 5)
    if _uint(f[0], "version") != VERSION or f[1] != TAG:
        raise NotebookError("bundle version/tag")
    p = _list(f[2], "project", 5)
    scope = _text(p[2], "scope", empty=False)  # type: ignore[arg-type]
    if not scope.startswith(SCOPE_PREFIXES):
        raise NotebookError("scope must be session:/until:/claim: (ADR0054)")
    project = Project(
        _bytes(p[0], 16, "project id"),
        _text(p[1], "project title", empty=False),  # type: ignore[arg-type]
        scope,
        _bytes(p[3], 32, "vocabulary"),
        _uint(p[4], "project revision"),
    )
    raw_artifacts = _list(f[3], "artifacts")
    if len(raw_artifacts) > MAX_ARTIFACTS:
        raise NotebookError("artifacts limit")
    artifacts = [_decode_artifact(item) for item in raw_artifacts]
    ids = {item.id for item in artifacts}
    if len(ids) != len(artifacts):
        raise NotebookError("duplicate artifact id")
    raw_links = _list(f[4], "relations")
    if len(raw_links) > MAX_RELATIONS:
        raise NotebookError("relations limit")
    links: list[Link] = []
    for raw in raw_links:
        r = _list(raw, "relation", 3)
        try:
            relation = Relation(_uint(r[1], "relation type"))
        except ValueError as error:
            raise NotebookError("unknown relation type") from error
        link = Link(_bytes(r[0], 16, "relation source"), relation, _bytes(r[2], 16, "target"))
        if link.source not in ids or link.target not in ids or link.source == link.target:
            raise NotebookError("relation endpoint")
        links.append(link)
    keys = [(x.source, int(x.relation), x.target) for x in links]
    if any(a >= b for a, b in itertools.pairwise(keys)):
        raise NotebookError("relations must be strictly sorted and unique")
    return Notebook(project, artifacts, set(links))


def digest(data: bytes) -> bytes:
    decode(data)
    return hashlib.sha256(HASH_DOMAIN + data).digest()
