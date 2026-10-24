#!/usr/bin/env python3
"""Pentest scriptado da Etapa 5.11 (ADR0059): o agente isolado não alcança o proibido.

Sobe o kernel real (chave 0600, ledger, staging), um gateway stub (socket Unix e
porta TCP), um arquivo de credenciais de ferramentas e um "LLM" de eco em TCP.
Roda `logos_agent.isolation_probe` duas vezes:

1. Fora do sandbox (controle positivo): toda tentativa proibida precisa conseguir;
   senão o teste não detectaria nada.
2. Dentro de `logos-agent-sandbox`: toda tentativa proibida precisa falhar e o
   permitido (kernel, staging, LLM pelo proxy) precisa funcionar.

Imprime ISOLATION_OK com os contadores; qualquer desvio é erro.
"""

from __future__ import annotations

import json
import os
import shutil
import socket
import subprocess
import tempfile
import threading
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
PY = ROOT / "py"


def build() -> tuple[Path, Path, Path, Path]:
    subprocess.run(
        [
            "cargo",
            "build",
            "-q",
            "-p",
            "logos-kernel",
            "--bin",
            "logos-kernel",
            "--example",
            "client_fixture",
            "-p",
            "logos-sandbox",
            "--bin",
            "logos-agent-sandbox",
            "-p",
            "logos-cli",
            "--bin",
            "logos",
        ],
        cwd=ROOT,
        check=True,
        timeout=900,
    )
    debug = ROOT / "target/debug"
    return (
        debug / "logos-kernel",
        debug / "examples/client_fixture",
        debug / "logos-agent-sandbox",
        debug / "logos",
    )


def serve(listener: socket.socket, reply: bytes) -> None:
    """Responde `reply` + eco a cada conexão; roda até o processo terminar."""

    def loop() -> None:
        while True:
            try:
                connection, _ = listener.accept()
            except OSError:
                return
            with connection:
                data = connection.recv(64)
                connection.sendall(reply + data)

    threading.Thread(target=loop, daemon=True).start()


def tcp_server(reply: bytes) -> int:
    listener = socket.socket(socket.AF_INET, socket.SOCK_STREAM)
    listener.bind(("127.0.0.1", 0))
    listener.listen()
    serve(listener, reply)
    return int(listener.getsockname()[1])


def unix_server(path: Path, reply: bytes) -> None:
    listener = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)
    listener.bind(str(path))
    listener.listen()
    serve(listener, reply)


Report = dict[str, dict[str, dict[str, object]]]


def probe(command: list[str], target: dict[str, object], env: dict[str, str]) -> Report:
    result = subprocess.run(
        [*command, json.dumps(target)],
        capture_output=True,
        text=True,
        timeout=180,
        env=env,
        check=False,
    )
    if result.returncode != 0:
        raise SystemExit(f"probe exit {result.returncode}: {result.stderr[-2000:]}")
    report: Report = json.loads(result.stdout)
    return report


def check_units() -> int:
    """Units de referência: `systemd-analyze verify` sem erro além dos executáveis
    ainda não instalados em /usr/local/bin; separação de usuários declarada."""
    units = sorted((ROOT / "deploy/systemd").glob("*.service"))
    result = subprocess.run(
        ["systemd-analyze", "verify", "--man=no", *map(str, units)],
        capture_output=True,
        text=True,
        timeout=60,
        check=False,
    )
    problems = [
        line
        for line in (result.stdout + result.stderr).splitlines()
        if line.strip() and "Command /usr/local/bin/logos-" not in line
    ]
    if problems:
        raise SystemExit("ISOLATION_FAILED units:\n" + "\n".join(problems))
    users = {}
    for unit in units:
        text = unit.read_text()
        users[unit.stem] = next(
            line.split("=", 1)[1] for line in text.splitlines() if line.startswith("User=")
        )
    agent = (ROOT / "deploy/systemd/logos-agent.service").read_text()
    if len(set(users.values())) != len(users) or "/etc/logos/kernel.key" not in agent:
        raise SystemExit(f"ISOLATION_FAILED unit users/paths: {users}")
    return len(units)


def main() -> None:
    units = check_units()
    kernel_binary, fixture, sandbox, logos = build()
    with tempfile.TemporaryDirectory(prefix="logos-isolation-") as temporary:
        root = Path(temporary)
        root.chmod(0o700)
        subprocess.run([str(fixture), str(root)], check=True, capture_output=True, timeout=60)
        for name in ["staging", "private", "proxy"]:
            (root / name).mkdir(mode=0o700)
        # Credenciais de ferramentas no home do operador, onde costumam ficar.
        home = Path(tempfile.mkdtemp(prefix="logos-isolation-", dir=Path.home() / ".cache"))
        credentials = home / "credentials.json"
        credentials.write_text('{"tool": "token-secreto-de-teste"}')
        credentials.chmod(0o600)
        # 6.5: chave do usuário criada pela CLI, no home do usuário, fora do alcance do agente.
        user_key = home / "user.key"
        subprocess.run(
            [str(logos), "user", "init", "--key", str(user_key)],
            check=True,
            capture_output=True,
            timeout=60,
        )
        unix_server(root / "gateway.sock", b"gateway:")
        gateway_port = tcp_server(b"gateway:")
        llm_port = tcp_server(b"llm:")
        log = (root / "kernel.log").open("wb")
        kernel = subprocess.Popen(
            [
                str(kernel_binary),
                str(root / "s"),
                str(root / "ledger"),
                str(root / "operator.key"),
                str(root / "staging"),
                str(root / "private"),
                str(os.geteuid()),
            ],
            stdout=log,
            stderr=subprocess.STDOUT,
        )
        try:
            until = time.monotonic() + 120
            while not (root / "s").exists():
                if kernel.poll() is not None or time.monotonic() > until:
                    raise SystemExit("kernel failed: " + (root / "kernel.log").read_text())
                time.sleep(0.05)
            target: dict[str, object] = {
                "key": str(root / "operator.key"),
                "ledger_files": [
                    str(root / "ledger/ledger.sqlite3"),
                    str(root / "ledger/index.sqlite3"),
                ],
                "private": str(root / "private"),
                "gateway_socket": str(root / "gateway.sock"),
                "gateway_port": gateway_port,
                "credentials": str(credentials),
                "user_key": str(user_key),
                "llm_port": llm_port,
                "kernel_pid": kernel.pid,
            }
            python = str(PY / ".venv/bin/python")
            control = probe(
                [python, "-m", "logos_agent.isolation_probe"],
                target,
                {
                    "PATH": "/usr/bin",
                    "PYTHONPATH": str(PY),
                    "LOGOS_KERNEL_SOCKET": str(root / "s"),
                    "LOGOS_STAGING": str(root / "staging"),
                },
            )
            isolated = probe(
                [
                    str(sandbox),
                    "--kernel-socket",
                    str(root / "s"),
                    "--staging",
                    str(root / "staging"),
                    "--code",
                    str(PY),
                    "--llm",
                    f"127.0.0.1:{llm_port}",
                    "--proxy-dir",
                    str(root / "proxy"),
                    "--",
                    "/usr/bin/env",
                    "PYTHONPATH=/opt/logos",
                    "/opt/logos/.venv/bin/python",
                    "-m",
                    "logos_agent.isolation_probe",
                ],
                target,
                {"PATH": "/usr/bin"},
            )
        finally:
            kernel.terminate()
            try:
                kernel.wait(timeout=10)
            except subprocess.TimeoutExpired:
                kernel.kill()
                kernel.wait(timeout=10)
            log.close()
            shutil.rmtree(home)
    failures = []
    for name, outcome in control["forbidden"].items():
        if not outcome["ok"]:
            failures.append(f"control: {name} should succeed outside the sandbox: {outcome}")
    for name, outcome in control["allowed"].items():
        if not outcome["ok"]:
            failures.append(f"control: {name} failed: {outcome}")
    for name, outcome in isolated["forbidden"].items():
        if outcome["ok"]:
            failures.append(f"sandbox: {name} must fail: {outcome}")
    expected = {"kernel_head", "staging_write", "llm_via_proxy"}
    if set(isolated["allowed"]) != expected:
        failures.append(f"sandbox: allowed checks {sorted(isolated['allowed'])}")
    for name, outcome in isolated["allowed"].items():
        if not outcome["ok"]:
            failures.append(f"sandbox: {name} must work: {outcome}")
    if failures:
        raise SystemExit("ISOLATION_FAILED\n" + "\n".join(failures))
    for name, outcome in isolated["forbidden"].items():
        print(f"  blocked {name}: {outcome['detail']}")
    print(
        f"ISOLATION_OK forbidden={len(isolated['forbidden'])} blocked=all "
        f"control_reached={len(control['forbidden'])} allowed={len(isolated['allowed'])} "
        f"units={units}"
    )


if __name__ == "__main__":
    main()
