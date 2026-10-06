"""Enumeração independente em mapas/sets; corpus distinto de obrigações pequenas."""

from __future__ import annotations

import itertools
from dataclasses import dataclass

from logos_provers import model_context as context
from tests.z3_models_cases import document_case


@dataclass(frozen=True)
class IndependentInterpretation:
    domains: dict[str, int]
    constants: dict[str, int]
    functions: dict[str, dict[tuple[int, ...], int]]
    predicates: dict[str, set[tuple[int, ...]]]


def independent_eval_term(
    term: context.TermAST, env: list[int], model: IndependentInterpretation
) -> int:
    if isinstance(term, context.ASTVar):
        return env[-1 - term.index]
    if isinstance(term, context.ASTApp):
        if term.symbol_kind == 0:
            return model.constants[term.symbol_name]
        arguments = tuple(independent_eval_term(arg, env, model) for arg in term.args)
        return model.functions[term.symbol_name][arguments]
    raise ValueError(f"unsupported term {type(term)}")


def independent_eval_formula(
    node: context.FormulaAST, env: list[int], model: IndependentInterpretation
) -> bool:
    if isinstance(node, context.ASTTrue):
        return True
    if isinstance(node, context.ASTFalse):
        return False
    if isinstance(node, context.ASTEq):
        return independent_eval_term(node.left, env, model) == independent_eval_term(
            node.right, env, model
        )
    if isinstance(node, context.ASTPred):
        arguments = tuple(independent_eval_term(arg, env, model) for arg in node.args)
        return arguments in model.predicates[node.symbol_name]
    if isinstance(node, context.ASTNot):
        return not independent_eval_formula(node.child, env, model)
    if isinstance(node, context.ASTAnd):
        return all(independent_eval_formula(child, env, model) for child in node.children)
    if isinstance(node, context.ASTOr):
        return any(independent_eval_formula(child, env, model) for child in node.children)
    if isinstance(node, context.ASTImplies):
        return not independent_eval_formula(node.left, env, model) or independent_eval_formula(
            node.right, env, model
        )
    if isinstance(node, context.ASTIff):
        return independent_eval_formula(node.left, env, model) == independent_eval_formula(
            node.right, env, model
        )
    if isinstance(node, context.ASTForall):
        return all(
            independent_eval_formula(node.child, [*env, value], model)
            for value in range(model.domains[node.sort])
        )
    if isinstance(node, context.ASTExists):
        return any(
            independent_eval_formula(node.child, [*env, value], model)
            for value in range(model.domains[node.sort])
        )
    raise ValueError(f"unsupported formula {type(node)}")


def enumerate_primitive_models_s_c_f_p() -> list[IndependentInterpretation]:
    """All n*n**n*2**n interpretations: 2+32+648=682 for n=1,2,3."""
    models: list[IndependentInterpretation] = []
    for n in (1, 2, 3):
        domain = range(n)
        for constant in domain:
            for outputs in itertools.product(domain, repeat=n):
                function: dict[tuple[int, ...], int] = {
                    (x,): y for x, y in zip(domain, outputs, strict=True)
                }
                for bits in itertools.product((False, True), repeat=n):
                    predicate: set[tuple[int, ...]] = {
                        (x,) for x, bit in zip(domain, bits, strict=True) if bit
                    }
                    models.append(
                        IndependentInterpretation(
                            {"S": n}, {"c": constant}, {"f": function}, {"P": predicate}
                        )
                    )
    return models


def has_independent_countermodel(
    premises: list[context.FormulaAST],
    goal: context.FormulaAST,
    models: list[IndependentInterpretation],
) -> bool:
    return any(
        all(independent_eval_formula(premise, [], model) for premise in premises)
        and not independent_eval_formula(goal, [], model)
        for model in models
    )


@dataclass(frozen=True)
class CorpusItem:
    name: str
    source: str


VOCAB = "vocab v1 { sort S; const c:S; func f(S):S; pred P(S); }"


def _boolean_function(mask: int) -> str:
    if mask == 0:
        return "false"
    if mask == 15:
        return "true"
    atoms = ("P(c)", "(c=f(c))")
    minterms = [
        "("
        + " and ".join(
            atom if valuation & (1 << bit) else f"not {atom}" for bit, atom in enumerate(atoms)
        )
        + ")"
        for valuation in range(4)
        if mask & (1 << valuation)
    ]
    return "(" + " or ".join(minterms) + ")"


def get_deterministic_corpus() -> list[CorpusItem]:
    """Every pair of 16 Boolean truth functions, plus 40 quantified/function cases."""
    claims: list[tuple[str | None, str]] = [
        (_boolean_function(premise), _boolean_function(goal))
        for premise in range(16)
        for goal in range(16)
    ]
    claims.extend(
        [
            ("P(c)", "forall x:S. P(x)"),
            ("forall x:S. P(f(x))", "P(c)"),
            ("not(c=f(c))", "forall x:S. not(x=f(x))"),
            ("P(c) and not P(f(c))", "c=f(c)"),
            (None, "forall x:S. exists y:S. f(y)=x"),
            ("forall x:S. forall y:S. (f(x)=f(y) -> x=y)", "forall x:S. exists y:S. f(y)=x"),
            ("forall x:S. f(f(x))=x", "forall x:S. f(x)=x"),
            ("exists x:S. P(x)", "forall x:S. P(x)"),
            ("P(c) and not P(f(c))", "forall x:S. P(x) or not P(x)"),
            ("c=f(c) and not(c=f(c))", "P(c)"),
            ("forall x:S. (P(x) -> P(f(x)))", "forall x:S. (P(f(x)) -> P(x))"),
            ("forall x:S. f(f(f(x)))=x", "forall x:S. f(x)=x"),
            ("forall x:S. f(f(x))=f(x)", "forall x:S. f(x)=x"),
            ("forall x:S. f(x)=x", "forall x:S. f(f(x))=x"),
            ("not P(c)", "forall x:S. not P(x)"),
            ("exists x:S. not P(x)", "not(forall x:S. P(x))"),
            ("not(forall x:S. P(x))", "forall x:S. not P(x)"),
            ("forall x:S. exists y:S. f(x)=y", "exists y:S. forall x:S. f(x)=y"),
            ("exists y:S. forall x:S. f(x)=y", "forall x:S. exists y:S. f(x)=y"),
            ("P(c)", "P(f(c))"),
            ("P(f(c))", "P(c)"),
            ("forall x:S. P(x)", "P(f(f(c)))"),
            ("forall x:S. (P(x) <-> x=c)", "P(f(c))"),
            ("(forall x:S. (P(x) <-> x=c)) and f(c)=c", "P(f(c))"),
            (None, "forall x:S. forall y:S. x=y"),
            (
                "exists x:S. exists y:S. not(x=y)",
                "forall x:S. forall y:S. forall z:S. x=y or y=z or x=z",
            ),
            ("c=f(c) or P(c)", "P(f(c))"),
            ("P(c) and (P(c) -> P(f(c)))", "P(f(c))"),
            ("P(c) <-> P(f(c))", "c=f(c)"),
            ("(forall x:S. P(x)) or (forall x:S. not P(x))", "forall x:S. (P(x) <-> P(c))"),
            ("forall x:S. P(x) or P(f(x))", "forall x:S. P(x)"),
            ("exists x:S. P(x) and f(x)=c", "P(c)"),
            ("(exists x:S. P(x) and f(x)=c) and (forall y:S. f(y)=y)", "P(c)"),
            ("not(P(c) <-> P(f(c)))", "not(c=f(c))"),
            ("not(c=f(c))", "not(P(c) <-> P(f(c)))"),
            (
                "forall x:S. not(f(x)=x)",
                "exists x:S. exists y:S. exists z:S. not(x=y) and not(y=z) and not(x=z)",
            ),
            ("forall x:S. forall y:S. (P(x) <-> P(y))", "P(c) <-> P(f(c))"),
            ("P(f(f(c))) and not P(c)", "f(f(c))=c"),
            ("(forall x:S. P(f(x))) and (exists y:S. not P(y))", "exists x:S. f(x)=c"),
            ("(forall x:S. P(x)) and (forall x:S. not P(x))", "forall x:S. forall y:S. x=y"),
        ]
    )
    return [
        CorpusItem(
            f"case_{index}",
            f"{VOCAB} "
            + (f"claim H:{premise}; " if premise is not None else "")
            + f"claim G:{goal};",
        )
        for index, (premise, goal) in enumerate(claims)
    ]


def make_strictly_domain_3_case() -> tuple[bytes, bytes]:
    return document_case(
        "vocab v1 { sort S; } claim H:exists x:S. exists y:S. exists z:S. "
        "not(x=y) and not(y=z) and not(x=z); claim G:false;"
    )


def make_mixed_sorts_case() -> tuple[bytes, bytes]:
    return document_case(
        "vocab v1 { sort A; sort B; func g(B):A; } "
        "claim H1:exists a:A. exists b:A. not(a=b); "
        "claim H2:forall a:B. forall b:B. a=b; "
        "claim G:exists b:B. forall a:A. g(b)=a;"
    )


def make_enum_fixed_valid_case() -> tuple[bytes, bytes]:
    return document_case(
        'vocab v1 { sort State = enum [s0,s1] from fact "' + "11" * 32 + '"; '
        'const s0:State; const s1:State = defined(eq(var(0,"State"),'
        'app(symbol("base",const,[],"State"),[]))); const base:State; } claim G:s1=s0;'
    )


def make_enum_fixed_invalid_ordinal_case() -> tuple[bytes, bytes]:
    return document_case(
        'vocab v1 { sort State = enum [s0,s1] from fact "' + "11" * 32 + '"; '
        'const s0:State = defined(eq(var(0,"State"),'
        'app(symbol("s1",const,[],"State"),[]))); const s1:State; } claim G:false;'
    )


def make_out_of_order_defined_case() -> tuple[bytes, bytes]:
    return document_case(
        "vocab v1 { sort S; const c:S; pred A(S) = defined(forall s:S. B(s)); "
        "pred B(S) = defined(true); } claim G:not A(c);"
    )


def make_unused_invalid_defined_case() -> tuple[bytes, bytes]:
    return document_case(
        "vocab v1 { sort S; const unused_bad:S = defined(false); const c:S; pred P(S); } "
        "claim H:P(c); claim G:false;"
    )


def make_non_unique_defined_func_case() -> tuple[bytes, bytes]:
    # true has exactly one result in a singleton, so require at least two elements.
    return document_case(
        "vocab v1 { sort S; func bad_f(S):S = defined(true); const c:S; const d:S; } "
        "claim H:not(c=d); claim G:false;"
    )


def make_nullary_case() -> tuple[bytes, bytes]:
    return document_case(
        "vocab v1 { sort S; const c:S; func fnul():S; pred P0(); } "
        "claim H:P0() and fnul()=c; claim G:not P0();"
    )


def make_dense_axis_case() -> tuple[bytes, bytes]:
    return document_case(
        'vocab v1 { sort S = enum [a,b] from fact "' + "11" * 32 + '"; '
        'sort T = enum [x,y,z] from fact "' + "22" * 32 + '"; '
        "const a:S; const b:S; const x:T; const y:T; const z:T; func f(S,T):S; } "
        "claim H:f(a,x)=a and f(a,y)=b and f(a,z)=a and "
        "f(b,x)=b and f(b,y)=a and f(b,z)=b; claim G:false;"
    )


def make_empty_vocab_case() -> tuple[bytes, bytes]:
    return document_case("vocab v1 {} claim G:false;")


def make_empty_enum_case() -> tuple[bytes, bytes]:
    return document_case(
        'vocab v1 { sort Empty = enum [] from fact "' + "11" * 32 + '"; } claim G:false;'
    )
