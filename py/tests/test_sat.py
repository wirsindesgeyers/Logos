"""Produtor não confiável: protocolo adversário e candidatos nos checkers reais."""

from __future__ import annotations

import json
import subprocess
import sys
import time
from pathlib import Path

import pytest

from logos_client import canon
from logos_provers import sat, sat_cases
from logos_provers.sat_cases import document_case, reconstruction_cases
from logos_provers.sat_smoke import check_requests, request

ROOT = Path(__file__).resolve().parents[2]
ENCODER = ROOT / "target/debug/examples/prover_bridge"
SOLVER = ROOT / "target/cadical/cadical"
CAKE = ROOT / "target/cake-lpr/cake_lpr"


def script(tmp_path: Path, body: str) -> Path:
    path = tmp_path / "fake-tool"
    path.write_text(f"#!{sys.executable}\n" + body)
    path.chmod(0o700)
    return path


@pytest.mark.parametrize(
    "code,wire",
    [
        (0, b"s UNSATISFIABLE\n"),
        (20, b"s SATISFIABLE\nv 1 0\n"),
        (10, b"s SATISFIABLE\nv 1 -1 0\n"),
        (10, b"s SATISFIABLE\nv 1 0 -2\n"),
        (10, b"s SATISFIABLE\nv 1 0\n"),
        (10, b"s SATISFIABLE\nv 1 3 0\n"),
        (10, b"s SATISFIABLE\nv 1 -2 0\ns UNSATISFIABLE\n"),
        (20, b"s UNSATISFIABLE\nv 0\n"),
        (20, b"s UNSATISFIABLE\ntrailer\n"),
        (-11, b"s UNSATISFIABLE\n"),
        (20, b"s UNSATISFIABLE\xff\n"),
    ],
)
def test_incomplete_conflicting_and_misleading_output_is_error(code: int, wire: bytes) -> None:
    with pytest.raises(sat.ProducerError, match="invalid solver output"):
        sat._answer(code, wire, 2)


def test_status_and_model_terminators_are_explicit() -> None:
    assert sat._answer(10, b"c banner\ns SATISFIABLE\nv 1\nv -2 0\n", 2) == (
        sat.Status.SAT,
        {1: True, 2: False},
    )
    assert sat._answer(20, b"s UNSATISFIABLE\n", 0) == (sat.Status.UNSAT, {})
    assert sat._answer(0, b"s UNKNOWN\n", 2) == (sat.Status.EXHAUSTED, {})


def test_lrat_adaptation_preserves_bytes_ids_and_all_steps_before_empty() -> None:
    prefix = b"5 2 0 2 3 0\n5 d 3 0\n6 0 5 1 4 0\n"
    assert sat._lrat_prefix(prefix + b"7 1 0 0\n") == prefix
    with pytest.raises(sat.ProducerError):
        sat._lrat_prefix(b"5 2 0 2 3 0\n")


@pytest.mark.parametrize(
    "kwargs",
    [
        {"timeout_ms": 0},
        {"timeout_ms": True},
        {"max_bytes": -1},
        {"mem_bytes": 2**32},
    ],
)
def test_invalid_limits_are_rejected(kwargs: dict[str, int]) -> None:
    with pytest.raises(sat.ProducerError):
        sat.Limits(**kwargs)


def test_reconstruction_controls_use_real_checkers() -> None:
    checks = []
    for case in reconstruction_cases():
        result = sat.produce(case.obligation, case.vocabulary, encoder=ENCODER, solver=SOLVER)
        assert result.reason is None
        if case.sat:
            assert result.status == sat.Status.SAT and result.model is not None
            assert result.certificate is None
            checks.append(request(case, mode="model", model=result.model.hex()))
            raw = canon.decode(result.model)
            assert isinstance(raw, list)
            if case.name == "defined/sat":
                # Defined identity/D são rederivados, não tabelas fornecidas pelo solver.
                assert raw[4] == []
            if case.name == "open/sat":
                assert raw[2] == [2, 1]
            if case.name == "tables/sat":
                assert isinstance(raw[4], list) and isinstance(raw[4][0], list)
                assert raw[4][0][1] == b"".join(i.to_bytes(4, "big") for i in [1, 0, 1, 0, 1, 0])
                assert isinstance(raw[4][1], list) and raw[4][1][1] == (1).to_bytes(4, "big")
            if case.name == "defined-representative/sat":
                assert isinstance(raw[3], list) and len(raw[3]) == 1
        else:
            assert result.status == sat.Status.UNSAT and result.certificate is not None
            checks.append(
                request(case, mode="proof", cnf=result.cnf.hex(), proof=result.certificate.hex())
            )
    responses = check_requests(ENCODER, CAKE, checks)
    assert all(r["ok"] for r in responses)
    assert all(r["result"]["accepted"] for r in responses[:-1])
    assert responses[-1]["result"] == {"rust": "Accepted", "cake": "Accepted"}


def test_false_solver_unsat_and_mutated_model_never_pass_checkers(tmp_path: Path) -> None:
    case = reconstruction_cases()[0]
    liar = script(
        tmp_path,
        "import pathlib,sys\npathlib.Path(sys.argv[-1]).write_text('9 0 1 0\\n')\n"
        "print('s UNSATISFIABLE')\nraise SystemExit(20)\n",
    )
    forged = sat.produce(case.obligation, case.vocabulary, encoder=ENCODER, solver=liar)
    assert forged.status == sat.Status.UNSAT and forged.certificate is not None
    response = check_requests(
        ENCODER,
        CAKE,
        [
            request(
                case,
                mode="proof",
                cnf=forged.cnf.hex(),
                proof=forged.certificate.hex(),
            )
        ],
    )[0]
    assert not response["ok"] or response["result"] != {"rust": "Accepted", "cake": "Accepted"}
    good = sat.produce(case.obligation, case.vocabulary, encoder=ENCODER, solver=SOLVER)
    assert good.model is not None
    model = canon.decode(good.model)
    assert isinstance(model, list) and isinstance(model[5], list)
    table = model[5][0]
    assert isinstance(table, list)
    table[1] = [[]]  # Flag passa a true, portanto não refuta o objetivo.
    response = check_requests(
        ENCODER,
        CAKE,
        [
            request(
                case,
                mode="model",
                model=canon.encode(model).hex(),
            )
        ],
    )[0]
    assert response["ok"] and response["result"]["accepted"] is False


def test_actual_deadline_kills_descendants_and_size_limit_is_inconclusive(tmp_path: Path) -> None:
    child_pid = tmp_path / "descendant.pid"
    slow = script(
        tmp_path,
        "import os,time,pathlib\npid=os.fork()\n"
        f"pathlib.Path({str(child_pid)!r}).write_text(str(pid or os.getpid()))\n"
        "time.sleep(30)\n",
    )
    work = tmp_path / "work"
    work.mkdir()
    start = time.monotonic()
    with pytest.raises(sat._Exhausted, match="time"):
        sat._run([str(slow)], work, sat.Limits(), start + 0.5)
    assert time.monotonic() - start < 5 and child_pid.exists()
    pid = int(child_pid.read_text())
    state = Path(f"/proc/{pid}/stat")

    # Um órfão morto pode aguardar coleta pelo init; nunca continua executando.
    def stopped() -> bool:
        try:
            return state.read_text().split()[2] == "Z"
        except FileNotFoundError:
            return True

    until = time.monotonic() + 1
    while not stopped() and time.monotonic() < until:
        time.sleep(0.01)
    assert stopped()
    case = reconstruction_cases()[0]
    result = sat.produce(
        case.obligation,
        case.vocabulary,
        encoder=ENCODER,
        solver=SOLVER,
        limits=sat.Limits(max_bytes=1),
    )
    assert result.status == sat.Status.EXHAUSTED and result.reason == "input size"
    assert result.model is None and result.certificate is None


def test_forged_encoder_context_and_false_sat_are_errors(tmp_path: Path) -> None:
    case = document_case("simple/sat", "vocab v1 { pred P(); } claim G:P();", sat=True)
    fake = script(
        tmp_path, 'print(\'{"version":1,"ok":true,"result":{"ground":"80","cnf":""}}\')\n'
    )
    with pytest.raises(sat.ProducerError, match="ground context mismatch"):
        sat.produce(case.obligation, case.vocabulary, encoder=fake, solver=SOLVER)
    fake = script(tmp_path, "print('s SATISFIABLE\\nv 1 0')\nraise SystemExit(10)\n")
    with pytest.raises(sat.ProducerError, match="incomplete assignment"):
        sat.produce(case.obligation, case.vocabulary, encoder=ENCODER, solver=fake)
    fake = script(
        tmp_path,
        "import pathlib,sys\n"
        "n=int(pathlib.Path(sys.argv[-2]).read_text().splitlines()[0].split()[2])\n"
        "print('s SATISFIABLE')\nprint('v '+ ' '.join(str(-i) for i in range(1,n+1))+' 0')\n"
        "raise SystemExit(10)\n",
    )
    with pytest.raises(sat.ProducerError, match="does not satisfy CNF"):
        sat.produce(case.obligation, case.vocabulary, encoder=ENCODER, solver=fake)


def test_transport_version_is_required() -> None:
    case = reconstruction_cases()[0]
    bad = request(case, mode="encode")
    bad["version"] = 2
    response = check_requests(ENCODER, CAKE, [bad])[0]
    assert response == {"version": 1, "ok": False, "error": "transport version"}
    assert json.dumps(response)


def test_transport_vectors_preserve_independent_cnf_bytes() -> None:
    data = json.loads((ROOT / "spec/vectors/sat/transport.json").read_text())
    rows = check_requests(ENCODER, CAKE, [v["request"] for v in data["vectors"]])
    assert rows == [v["response"] for v in data["vectors"]]


def test_cnf_substitution_is_rejected_before_proof_recheck() -> None:
    case = reconstruction_cases()[0]
    response = check_requests(
        ENCODER,
        CAKE,
        [
            request(
                case,
                mode="proof",
                cnf=b"p cnf 0 1\n0\n".hex(),
                proof=b"2 0 1 0\n".hex(),
            )
        ],
    )[0]
    assert response == {"version": 1, "ok": False, "error": "CNF differs from obligation"}


def test_runtime_producer_works_when_z3_import_is_blocked() -> None:
    program = """
import sys
class BlockZ3:
    def find_spec(self, fullname, path=None, target=None):
        if fullname == 'z3' or fullname.startswith('z3.'):
            raise RuntimeError('SAT producer must not use Z3')
sys.meta_path.insert(0,BlockZ3())
from pathlib import Path
from logos_provers.sat import produce, Status
from logos_provers.sat_cases import reconstruction_cases
case = reconstruction_cases()[0]
result = produce(case.obligation,case.vocabulary,encoder=Path(sys.argv[1]),solver=Path(sys.argv[2]))
assert result.status is Status.SAT and result.model is not None
print('SAT_WITHOUT_Z3_OK')
"""
    result = subprocess.run(
        [sys.executable, "-c", program, str(ENCODER), str(SOLVER)],
        capture_output=True,
        text=True,
        check=True,
        timeout=30,
    )
    assert result.stdout.strip() == "SAT_WITHOUT_Z3_OK" and not result.stderr


def _hidden_case() -> tuple[bytes, bytes]:
    from tests.z3_models_cases import document_case as premises_case

    return premises_case(
        "vocab v1 { pred P(); pred Q(); pred R(); } claim A: P(); claim B: P() -> Q(); "
        "claim C: R(); claim D: R() -> Q(); claim G: Q();"
    )


def _ref(index: int) -> bytes:
    return index.to_bytes(32, "big")


def test_hidden_premises_give_independent_proofs_accepted_by_both_checkers() -> None:
    """5R.3: rotas com `used` disjuntos para a mesma O; IDs da CNF de O preservados."""
    obligation, vocabulary = _hidden_case()
    case = sat_cases_case(obligation, vocabulary)
    proofs: list[sat.Candidate] = []
    for hide in (frozenset({_ref(1), _ref(2)}), frozenset({_ref(3), _ref(4)})):
        result = sat.produce(obligation, vocabulary, encoder=ENCODER, solver=SOLVER, hide=hide)
        assert result.status == sat.Status.UNSAT and result.certificate is not None
        proofs.append(result)
    plain = sat.produce(obligation, vocabulary, encoder=ENCODER, solver=SOLVER)
    assert {p.cnf for p in proofs} == {plain.cnf}, "hidden mode must keep O's own CNF"
    responses = check_requests(
        ENCODER,
        CAKE,
        [
            request(case, mode="proof", cnf=p.cnf.hex(), proof=p.certificate.hex())
            for p in proofs
            if p.certificate is not None
        ],
    )
    assert [r["result"] for r in responses] == [{"rust": "Accepted", "cake": "Accepted"}] * 2
    assert proofs[0].certificate != proofs[1].certificate


def test_hidden_mode_never_turns_sat_into_a_model_or_hides_unknown_refs() -> None:
    obligation, vocabulary = _hidden_case()
    every = frozenset({_ref(1), _ref(3)})
    result = sat.produce(obligation, vocabulary, encoder=ENCODER, solver=SOLVER, hide=every)
    assert result.status == sat.Status.EXHAUSTED and result.model is None
    assert result.reason == "sat_without_hidden_premises"
    with pytest.raises(sat.ProducerError, match="not cited"):
        sat.produce(
            obligation, vocabulary, encoder=ENCODER, solver=SOLVER, hide=frozenset({_ref(9)})
        )
    with pytest.raises(sat.ProducerError, match="32-byte"):
        sat.produce(obligation, vocabulary, encoder=ENCODER, solver=SOLVER, hide=frozenset({b"x"}))


def test_hint_scan_rejects_proofs_that_cite_hidden_assertions() -> None:
    assert sat._hint_ids(b"9 -1 0 3 4 0\n9 d 2 0\n10 0 9 7 0\n") == {3, 4, 9, 7}
    with pytest.raises(sat.ProducerError):
        sat._hint_ids(b"9 x 0 1 0\n")


def sat_cases_case(obligation: bytes, vocabulary: bytes) -> sat_cases.Case:
    return sat_cases.Case("hidden", "hidden-premises", obligation, vocabulary, sat=False)
