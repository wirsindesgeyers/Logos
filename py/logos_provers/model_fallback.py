"""Fallback não autoritativo: Z3 primário, busca exaustiva finita em contingência.

Roadmap Etapa 2.5: Fora da TCB de correção. Chama `z3_models.refute` primeiro; se não
obtiver modelo (unknown, timeout ou unsat dentro do limite), recorre a
`model_finder.refute`.
"""

from __future__ import annotations

from logos_provers import model_context, model_finder, z3_models

__all__ = ["refute"]

_DEFAULT_LIMITS = model_finder.Limits()


def refute(
    obligation: bytes,
    vocabulary: bytes,
    *,
    limits: model_finder.Limits = _DEFAULT_LIMITS,
    z3_limits: z3_models.Limits | None = None,
) -> bytes | None:
    """Busca um contra-modelo tentando primariamente Z3 e recorrendo ao model finder.

    Valida o tipo de `limits` mesmo se o produtor primário obtiver sucesso.
    Se `z3_limits` não for fornecido, constrói `z3_models.Limits` a partir de
    `limits.max_domain` e `limits.timeout_ms`.
    O timeout é independente por produtor (não conjunto).
    Erros de validação e execução propagam sem conversão para ausência de modelo.
    """
    if not isinstance(limits, model_finder.Limits):
        raise model_context.ProducerError(
            f"limits must be a model_finder.Limits instance, got {type(limits).__name__}"
        )
    if z3_limits is not None and not isinstance(z3_limits, z3_models.Limits):
        raise model_context.ProducerError(
            f"z3_limits must be a z3_models.Limits instance, got {type(z3_limits).__name__}"
        )

    resolved_z3_limits = (
        z3_limits
        if z3_limits is not None
        else z3_models.Limits(max_domain=limits.max_domain, timeout_ms=limits.timeout_ms)
    )

    primary_candidate = z3_models.refute(obligation, vocabulary, limits=resolved_z3_limits)
    if primary_candidate is not None:
        return primary_candidate

    finder_result = model_finder.refute(obligation, vocabulary, limits=limits)
    if finder_result.status == model_finder.SearchStatus.FOUND:
        return finder_result.model
    return None
