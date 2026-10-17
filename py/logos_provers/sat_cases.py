"""Corpus determinístico da4.6: regras explícitas, sem autoridade Policy/Fact."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from typing import Any

from logos_client import air, canon


@dataclass(frozen=True)
class Case:
    name: str
    family: str
    obligation: bytes
    vocabulary: bytes
    sat: bool


def document_case(name: str, text: str, *, sat: bool) -> Case:
    document = air.parse_document(text)
    vocabulary = canon.encode(document.vocabulary)
    h_v = hashlib.sha256(b"logos/vocab/v1\0" + vocabulary).digest()
    raw: Any = document.vocabulary
    formulas: list[canon.Value] = []
    for sort_name, domain in raw[2]:
        if domain[0] != 1:
            continue
        sort: canon.Value = [1, 0, sort_name]
        var: canon.Value = [1, 2, air.LOGIC_ID, [[0, 0, sort]]]
        terms: list[canon.Value] = [[1, 2, air.LOGIC_ID, [[1, s, 0]]] for s in domain[1]]
        formulas.append(
            [
                1,
                3,
                air.LOGIC_ID,
                [[9, sort], [6, len(terms)], *[[2, var, term] for term in terms]],
            ]
        )
        formulas.extend(
            [1, 3, air.LOGIC_ID, [[4], [2, left, right]]]
            for i, left in enumerate(terms)
            for right in terms[i + 1 :]
        )
    formulas.extend(claim.formula for claim in document.claims[:-1])
    premises: list[canon.Value] = [
        [i.to_bytes(4, "big") + bytes(28), formula] for i, formula in enumerate(formulas)
    ]
    obligation = canon.encode(
        [
            1,
            0,
            air.LOGIC_ID,
            1,
            h_v,
            premises,
            document.claims[-1].formula,
        ]
    )
    return Case(name, name.split("/")[0], obligation, vocabulary, sat)


def policy_cases() -> list[Case]:
    """300 válidas em10 famílias;300 pares com uma exigência não estabelecida."""
    cases: list[Case] = []
    for family in range(10):
        for variant in range(30):
            n = 2 + variant % 3
            names = [f"u{i}" for i in range(n)]
            subject = names[(variant // 3) % n]
            source = hashlib.sha256(f"policy-source-{variant}".encode()).hexdigest()
            prefix = (
                f'vocab v1 {{ sort User = enum [{",".join(names)}] from fact "{source}"; '
                + " ".join(f"const {name}:User;" for name in names)
                + " pred Role(User); pred Active(User); pred Allowed(User);"
                + " pred Blocked(User); pred Reviewed(User);"
                + " pred Relation(User,User); func owner(User):User;"
                + " const selected:User; pred Extra(User);"
                + " ".join(f" pred Context{i}();" for i in range(5))
                + " } "
            )
            r = f"Role({subject})"
            a = f"Active({subject})"
            allow = f"Allowed({subject})"
            review = f"Reviewed({subject})"
            rules = [
                (["forall x:User. (Role(x) -> Allowed(x))", r], allow),
                (["forall x:User. ((Role(x) and Active(x)) -> Allowed(x))", r, a], allow),
                (
                    [
                        "forall x:User. (Role(x) -> Active(x))",
                        "forall x:User. (Active(x) -> Allowed(x))",
                        r,
                    ],
                    allow,
                ),
                (
                    [
                        f"{r} or {a}",
                        "forall x:User. (Role(x) -> Allowed(x))",
                        "forall x:User. (Active(x) -> Allowed(x))",
                    ],
                    allow,
                ),
                (["forall x:User. (Allowed(x) -> Active(x))", f"not {a}"], f"not {allow}"),
                (
                    [
                        f"not Blocked({subject})",
                        r,
                        "forall x:User. ((Role(x) and not Blocked(x)) -> Allowed(x))",
                    ],
                    allow,
                ),
                (["forall x:User. (Reviewed(x) <-> Allowed(x))", review], allow),
                (
                    [
                        f"Relation({subject},{names[0]})",
                        "forall x:User. forall y:User. (Relation(x,y) -> Allowed(x))",
                    ],
                    allow,
                ),
                (
                    [
                        f"owner({subject}) = {names[0]}",
                        f"Role({names[0]})",
                        "forall x:User. (Role(owner(x)) -> Allowed(x))",
                    ],
                    allow,
                ),
                (
                    [
                        f"selected = {subject}",
                        "Role(selected)",
                        "forall x:User. (Role(x) -> Allowed(x))",
                    ],
                    allow,
                ),
            ]
            premises, goal = rules[family]
            context = [f"{'not ' if variant & (1 << i) else ''}Context{i}()" for i in range(5)]
            premises = [*premises, *context]
            goal = f"({goal}) and ({' and '.join(context)})"
            claims = " ".join(f"claim H{i}: {p};" for i, p in enumerate(premises))
            name = f"policy{family}/{variant}"
            cases.append(
                document_case(name + "/valid", prefix + claims + f" claim G:{goal};", sat=False)
            )
            # Extra tem interpretação livre: a nova exigência torna a obrigação refutável.
            cases.append(
                document_case(
                    name + "/countermodel",
                    prefix + claims + f" claim G:({goal}) and Extra({subject});",
                    sat=True,
                )
            )
    assert len(cases) == 600
    assert len({c.obligation for c in cases}) == 600
    return cases


def reconstruction_cases() -> list[Case]:
    base = 'sort E = enum [a,b] from fact "' + "11" * 32 + '"; const a:E; const b:E; '
    return [
        document_case("nullary/sat", "vocab v1 { pred Flag(); } claim G:Flag();", sat=True),
        document_case(
            "singleton/sat",
            'vocab v1 { sort E = enum [a] from fact "'
            + "11" * 32
            + '"; const a:E; const c:E; func f(E):E; pred P(E); } '
            "claim G:P(f(c));",
            sat=True,
        ),
        document_case(
            "defined/sat",
            "vocab v1 { "
            + base
            + 'pred P(E); pred D(E) = defined(P(var(0,"E"))); '
            + 'func identity(E):E = defined(eq(var(0,"E"),var(1,"E"))); } '
            + "claim G:D(identity(b));",
            sat=True,
        ),
        document_case(
            "open/sat",
            "vocab v1 { "
            + base
            + "sort Open; func f(Open,E):E; pred Q(Open,E); pred P(E); } "
            + "claim G:P(a);",
            sat=True,
        ),
        document_case(
            "tables/sat",
            "vocab v1 { "
            + base
            + 'sort T = enum [x,y,z] from fact "'
            + "22" * 32
            + '"; const x:T; const y:T; const z:T; const choice:E; func f(E,T):E; '
            + "func nullary():E; pred R(E,T); } "
            + "claim H0:choice=b; claim H1:f(a,x)=b; claim H2:f(a,y)=a; "
            + "claim H3:f(a,z)=b; claim H4:f(b,x)=a; claim H5:f(b,y)=b; "
            + "claim H6:f(b,z)=a; claim H7:nullary()=b; claim H8:R(a,z); "
            + "claim G:R(choice,y);",
            sat=True,
        ),
        document_case(
            "defined-representative/sat",
            'vocab v1 { sort E = enum [a,b] from fact "'
            + "11" * 32
            + '"; const a:E; const b:E = defined(not eq(var(0,"E"),a)); pred P(E); } '
            + "claim G:P(b);",
            sat=True,
        ),
        document_case(
            "empty/unsat",
            'vocab v1 { sort E = enum [] from fact "' + "11" * 32 + '"; } claim G:false;',
            sat=False,
        ),
    ]
