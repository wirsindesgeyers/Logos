"""CaDiCaL → candidatos LRAT/model-v1; todo este módulo é não confiável.

O consumidor precisa rechecar a prova/modelo e o vínculo com a obrigação.
O status do solver nunca é um veredicto do LOGOS.
"""

from __future__ import annotations

import hashlib
import itertools
import json
import math
import os
import signal
import subprocess
import tempfile
import time
from contextlib import suppress
from dataclasses import dataclass
from enum import StrEnum
from pathlib import Path
from typing import Any

from logos_client import canon
from logos_provers import model_context

ProducerError = model_context.ProducerError


class Status(StrEnum):
    UNSAT = "UNSAT"
    SAT = "SAT"
    EXHAUSTED = "EXHAUSTED"


@dataclass(frozen=True)
class Limits:
    timeout_ms: int = 5000
    mem_bytes: int = 268_435_456
    max_bytes: int = 16_777_216

    def __post_init__(self) -> None:
        for name, value in vars(self).items():
            if type(value) is not int or value <= 0 or value > 4_294_967_295:
                raise ProducerError(f"{name} must be an integer in 1..=u32::MAX")


@dataclass(frozen=True)
class Candidate:
    status: Status
    obligation_hash: bytes
    vocabulary_hash: bytes
    cnf: bytes = b""
    ground: bytes = b""
    certificate: bytes | None = None
    model: bytes | None = None
    reason: str | None = None
    raw_certificate_hash: bytes | None = None

    @property
    def cnf_hash(self) -> bytes:
        return hashlib.sha256(b"logos/cnf/v1\0" + self.cnf).digest()

    @property
    def certificate_hash(self) -> bytes | None:
        if self.certificate is None:
            return None
        return hashlib.sha256(b"logos/certificate/v1\0" + self.certificate).digest()


class _Exhausted(Exception):
    pass


_DEFAULT_LIMITS = Limits()


def _run(
    args: list[str],
    work: Path,
    limits: Limits,
    deadline: float,
    *,
    payload: bytes = b"",
    output_limit: int | None = None,
) -> tuple[int, bytes, bytes]:
    remaining = deadline - time.monotonic()
    if remaining <= 0:
        raise _Exhausted("time")
    cap = limits.max_bytes if output_limit is None else output_limit
    with (
        (work / "stdin").open("w+b") as stdin,
        (work / "stdout").open("w+b") as stdout,
        (work / "stderr").open("w+b") as stderr,
    ):
        stdin.write(payload)
        stdin.seek(0)
        command = [
            "/usr/bin/prlimit",
            f"--as={limits.mem_bytes}:{limits.mem_bytes}",
            f"--fsize={cap}:{cap}",
            "--core=0:0",
            "--",
            *args,
        ]
        process = subprocess.Popen(
            command,
            cwd=work,
            env={"PATH": os.defpath, "LC_ALL": "C"},
            stdin=stdin,
            stdout=stdout,
            stderr=stderr,
            start_new_session=True,
        )
        try:
            try:
                code = process.wait(timeout=remaining)
            except subprocess.TimeoutExpired as error:
                raise _Exhausted("time") from error
        finally:
            with suppress(ProcessLookupError):
                os.killpg(process.pid, signal.SIGKILL)
            process.wait()
        if time.monotonic() >= deadline:
            raise _Exhausted("time")
        if code == -signal.SIGXFSZ:
            raise _Exhausted("size")
        stdout.seek(0)
        stderr.seek(0)
        out, err = stdout.read(cap + 1), stderr.read(cap + 1)
        if len(out) > cap or len(err) > cap:
            raise _Exhausted("size")
        return code, out, err


def _dimacs(cnf: bytes) -> tuple[int, list[list[int]]]:
    try:
        rows = cnf.decode("ascii").splitlines()
        header = rows[0].split()
        if len(header) != 4 or header[:2] != ["p", "cnf"]:
            raise ValueError("header")
        variables, count = map(int, header[2:])
        if not 0 <= variables <= 1_000_000 or len(rows) != count + 1:
            raise ValueError("counts")
        clauses = [list(map(int, row.split())) for row in rows[1:]]
        for clause in clauses:
            if not clause or clause[-1] != 0:
                raise ValueError("terminator")
            if any(lit == 0 or abs(lit) > variables for lit in clause[:-1]):
                raise ValueError("literal")
        return variables, [clause[:-1] for clause in clauses]
    except (UnicodeError, IndexError, ValueError) as error:
        raise ProducerError(f"invalid CNF from encoder: {error}") from error


def _answer(code: int, output: bytes, variables: int) -> tuple[Status, dict[int, bool]]:
    try:
        rows = output.decode("ascii").splitlines()
        statuses: list[str] = []
        literals: list[int] = []
        for row in rows:
            if not row or row.startswith("c ") or row == "c":
                continue
            if row.startswith("s "):
                statuses.append(row)
            elif row.startswith("v "):
                literals.extend(map(int, row[2:].split()))
            else:
                raise ValueError("unexpected stdout line")
        if code == 20 and statuses == ["s UNSATISFIABLE"] and not literals:
            return Status.UNSAT, {}
        if code == 0 and statuses == ["s UNKNOWN"] and not literals:
            return Status.EXHAUSTED, {}
        if code != 10 or statuses != ["s SATISFIABLE"]:
            raise ValueError("exit/status disagreement")
        if not literals or literals[-1] != 0 or 0 in literals[:-1]:
            raise ValueError("witness terminator")
        valuation: dict[int, bool] = {}
        for literal in literals[:-1]:
            variable = abs(literal)
            if not 1 <= variable <= variables or variable in valuation:
                raise ValueError("duplicate/out-of-range assignment")
            valuation[variable] = literal > 0
        if len(valuation) != variables:
            raise ValueError("incomplete assignment")
        return Status.SAT, valuation
    except (UnicodeError, ValueError) as error:
        raise ProducerError(f"invalid solver output: {error}") from error


def _lrat_prefix(proof: bytes) -> bytes:
    """Descarta somente a cauda depois da primeira cláusula vazia; IDs intactos."""
    try:
        rows = proof.decode("ascii").splitlines(keepends=True)
        prefix: list[str] = []
        for row in rows:
            fields = row.split()
            prefix.append(row if row.endswith("\n") else row + "\n")
            if len(fields) >= 4 and fields[1] == "0" and fields[-1] == "0":
                return "".join(prefix).encode("ascii")
        raise ValueError("no empty clause")
    except (UnicodeError, ValueError) as error:
        raise ProducerError(f"invalid/incomplete LRAT candidate: {error}") from error


def _model(
    vocabulary: bytes,
    ground: bytes,
    valuation: dict[int, bool],
    deadline: float,
) -> bytes:
    vocab = model_context._validate_vocabulary(vocabulary)
    data: Any = canon.decode(ground)
    if not isinstance(data, list) or len(data) != 10 or data[0] != 1 or data[2] != vocab.h_v:
        raise ProducerError("invalid ground envelope")
    domains = tuple(len(sort.enum_constants) if sort.is_enum else 1 for sort in vocab.sorts)
    if any(size == 0 for size in domains):
        raise ProducerError("SAT cannot describe an empty classical domain")
    atoms: dict[tuple[int, tuple[int, ...], int | None], bool] = {}
    for index, atom in enumerate(data[3], 1):
        if not isinstance(atom, list) or len(atom) != 3:
            raise ProducerError("invalid atom")
        symbol, args, output = atom
        if (
            type(symbol) is not int
            or not 0 <= symbol < len(vocab.symbols)
            or not isinstance(args, list)
            or any(type(a) is not int or a < 0 for a in args)
            or not isinstance(output, list)
            or len(output) > 1
            or any(type(a) is not int or a < 0 for a in output)
        ):
            raise ProducerError("invalid atom coordinates")
        key = (symbol, tuple(args), output[0] if output else None)
        if key in atoms or index not in valuation:
            raise ProducerError("duplicate/missing primitive assignment")
        atoms[key] = valuation[index]

    consts: list[canon.Value] = []
    funcs: list[canon.Value] = []
    preds: list[canon.Value] = []
    cells = 0
    for symbol in vocab.symbols:
        if time.monotonic() >= deadline:
            raise _Exhausted("time")
        if symbol.grounding_kind == 0:
            continue
        inputs = [vocab.sort_by_name[name] for name in symbol.arity]
        rows = math.prod(domains[sort.index] for sort in inputs)
        cells += rows * max(1, len(inputs))
        if cells > model_context.MAX_MODEL_CELLS:
            raise _Exhausted("model cells")
        output_sort = vocab.sort_by_name[symbol.result] if symbol.result is not None else None
        representative = (
            output_sort.enum_constants.index(symbol.name)
            if output_sort is not None and symbol.name in output_sort.enum_constants
            else None
        )
        outputs: list[int] = []
        true_tuples: list[canon.Value] = []
        for args in itertools.product(*(range(domains[sort.index]) for sort in inputs)):
            if time.monotonic() >= deadline:
                raise _Exhausted("time")
            if symbol.kind == 2:
                key = (symbol.index, args, None)
                if all(sort.is_enum for sort in inputs) and key not in atoms:
                    raise ProducerError("missing finite predicate atom")
                if atoms.get(key, False):
                    true_tuples.append(list(args))
            else:
                assert output_sort is not None
                count = domains[output_sort.index]
                if representative is not None:
                    selected = representative
                elif count == 1 or not output_sort.is_enum or any(not s.is_enum for s in inputs):
                    selected = 0
                else:
                    keys = [(symbol.index, args, output) for output in range(count)]
                    if any(key not in atoms for key in keys):
                        raise ProducerError("missing finite value atom")
                    values = [output for output, key in enumerate(keys) if atoms[key]]
                    if len(values) != 1:
                        raise ProducerError("function/constant does not have a unique value")
                    selected = values[0]
                outputs.append(selected)
        if symbol.kind == 0:
            consts.append([symbol.index, outputs[0]])
        elif symbol.kind == 1:
            funcs.append([symbol.index, b"".join(value.to_bytes(4, "big") for value in outputs)])
        else:
            preds.append([symbol.index, true_tuples])
    wire = canon.encode([1, vocab.h_v, list(domains), consts, funcs, preds])
    if len(wire) > model_context.MAX_MODEL_BYTES:
        raise _Exhausted("model encoding size")
    return wire


def produce(
    obligation: bytes,
    vocabulary: bytes,
    *,
    encoder: Path,
    solver: Path,
    limits: Limits = _DEFAULT_LIMITS,
) -> Candidate:
    """Produz candidato, sem conferir autoridade nem emitir resultado de checker."""
    if not isinstance(limits, Limits):
        raise ProducerError("limits must be Limits")
    if not isinstance(obligation, bytes) or not isinstance(vocabulary, bytes):
        raise ProducerError("obligation/vocabulary must be bytes")
    deadline = time.monotonic() + limits.timeout_ms / 1000
    h_o = hashlib.sha256(b"logos/obligation/v1\0" + obligation).digest()
    h_v = hashlib.sha256(b"logos/vocab/v1\0" + vocabulary).digest()
    if len(obligation) + len(vocabulary) > limits.max_bytes:
        return Candidate(Status.EXHAUSTED, h_o, h_v, reason="input size")
    vocab = model_context._validate_vocabulary(vocabulary)
    model_context._validate_obligation(obligation, vocab)
    cnf = ground = b""
    encoder_path, solver_path = str(encoder.resolve()), str(solver.resolve())
    try:
        with tempfile.TemporaryDirectory(prefix="logos-sat-") as directory:
            work = Path(directory)
            request = (
                json.dumps(
                    {
                        "version": 1,
                        "mode": "encode",
                        "vocabulary": vocabulary.hex(),
                        "obligation": obligation.hex(),
                    }
                ).encode()
                + b"\n"
            )
            code, out, err = _run(
                [encoder_path],
                work,
                limits,
                deadline,
                payload=request,
                output_limit=limits.max_bytes * 4 + 4096,
            )
            if code != 0 or err:
                raise ProducerError(f"encoder failed: exit={code} stderr={err[:200]!r}")
            try:
                response = json.loads(out)
                if not isinstance(response, dict):
                    raise ValueError("encoder response must be an object")
                if type(response.get("version")) is not int or response.get("version") != 1:
                    raise ValueError("encoder transport version")
                if response.get("ok") is not True:
                    raise ValueError(response.get("error", "encoder failure"))
                ground = bytes.fromhex(response["result"]["ground"])
                cnf = bytes.fromhex(response["result"]["cnf"])
                data = canon.decode(ground)
                if (
                    not isinstance(data, list)
                    or len(data) != 10
                    or data[0] != 1
                    or data[1:3] != [h_o, h_v]
                ):
                    raise ValueError("ground context mismatch")
            except (ValueError, KeyError, TypeError, canon.CanonError) as error:
                raise ProducerError(f"invalid encoder response: {error}") from error
            if len(cnf) + len(ground) > limits.max_bytes:
                raise _Exhausted("encoded size")
            variables, clauses = _dimacs(cnf)
            input_path, proof_path = work / "input.cnf", work / "proof.lrat"
            input_path.write_bytes(cnf)
            code, out, err = _run(
                [solver_path, "--lrat", "--no-binary", str(input_path), str(proof_path)],
                work,
                limits,
                deadline,
            )
            if err:
                raise ProducerError(f"solver stderr: {err[:200]!r}")
            status, valuation = _answer(code, out, variables)
            if status == Status.EXHAUSTED:
                raise _Exhausted("solver unknown")
            if status == Status.UNSAT:
                if not proof_path.is_file():
                    raise ProducerError("solver omitted LRAT candidate")
                with proof_path.open("rb") as file:
                    proof = file.read(limits.max_bytes + 1)
                if len(proof) > limits.max_bytes:
                    raise _Exhausted("certificate size")
                prefix = _lrat_prefix(proof)
                result = Candidate(
                    status,
                    h_o,
                    h_v,
                    cnf,
                    ground,
                    certificate=prefix,
                    raw_certificate_hash=hashlib.sha256(proof).digest(),
                )
            else:
                if any(not any(valuation[abs(lit)] == (lit > 0) for lit in c) for c in clauses):
                    raise ProducerError("SAT witness does not satisfy CNF")
                model = _model(vocabulary, ground, valuation, deadline)
                if len(model) > limits.max_bytes:
                    raise _Exhausted("model size")
                result = Candidate(status, h_o, h_v, cnf, ground, model=model)
            if time.monotonic() >= deadline:
                raise _Exhausted("time")
            return result
    except _Exhausted as error:
        return Candidate(Status.EXHAUSTED, h_o, h_v, cnf, ground, reason=str(error))
