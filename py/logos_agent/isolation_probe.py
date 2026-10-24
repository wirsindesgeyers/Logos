"""Probe de isolamento da 5.11 (ADR0059): roda como o agente e tenta o proibido.

Recebe em JSON os caminhos/portas que um atacante conheceria (chave do kernel,
arquivos do ledger, gateway, credenciais, chave do usuário, PID do kernel) e imprime em JSON, para
cada tentativa, se ela conseguiu. Fora do sandbox serve de controle positivo:
as tentativas proibidas precisam conseguir, senão o teste não detectaria nada.
"""

from __future__ import annotations

import asyncio
import json
import os
import socket
import sqlite3
import sys
from collections.abc import Callable
from pathlib import Path

from logos_client.client import KernelClient

TIMEOUT = 3.0


def _read(path: str) -> str:
    with Path(path).open("rb") as handle:
        handle.read(1)
    return "read"


def _ledger_write(paths: list[str]) -> str:
    # Abrir para escrita já é a violação; nada é alterado no controle positivo.
    for path in paths:
        with Path(path).open("r+b"):
            pass
    database = next(p for p in paths if p.endswith("ledger.sqlite3"))
    connection = sqlite3.connect(f"file:{database}?mode=rw", uri=True, timeout=TIMEOUT)
    try:
        connection.execute("BEGIN IMMEDIATE")
        connection.execute("ROLLBACK")
    finally:
        connection.close()
    return "opened for writing"


def _unix(path: str, payload: bytes = b"ping") -> str:
    with socket.socket(socket.AF_UNIX, socket.SOCK_STREAM) as client:
        client.settimeout(TIMEOUT)
        client.connect(path)
        client.sendall(payload)
        client.shutdown(socket.SHUT_WR)
        return client.recv(64).decode(errors="replace")


def _tcp(port: int) -> str:
    with socket.create_connection(("127.0.0.1", port), timeout=TIMEOUT) as client:
        client.sendall(b"ping")
        client.shutdown(socket.SHUT_WR)
        return client.recv(64).decode(errors="replace")


def _sign_as_user(path: str) -> str:
    """Produz uma assinatura do usuário: só possível se a chave for legível pelo agente."""
    from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

    seed = bytes.fromhex(Path(path).read_text(encoding="ascii").strip())
    key = Ed25519PrivateKey.from_private_bytes(seed)
    key.sign(b"authorize: drop table users")
    return "signed as the user"


def _kernel_head(path: str) -> str:
    head = asyncio.run(KernelClient(Path(path), timeout=TIMEOUT * 10).head())
    if head is None:
        raise RuntimeError("empty ledger")
    return f"seq={head.seq}"


def _staging_write(directory: str) -> str:
    target = Path(directory) / "probe.tmp"
    target.write_bytes(b"candidate")
    target.unlink()
    return "written"


def attempt(action: Callable[[], str]) -> dict[str, object]:
    try:
        return {"ok": True, "detail": action()}
    except Exception as error:  # o resultado é o próprio tipo de falha
        return {"ok": False, "detail": f"{type(error).__name__}: {error}"}


def main() -> None:
    target = json.loads(sys.argv[1])
    forbidden: dict[str, Callable[[], str]] = {
        "read_kernel_key": lambda: _read(target["key"]),
        "write_ledger_sqlite": lambda: _ledger_write(target["ledger_files"]),
        "list_kernel_private": lambda: str(os.listdir(target["private"])),
        "connect_gateway_unix": lambda: _unix(target["gateway_socket"]),
        "connect_gateway_tcp": lambda: _tcp(target["gateway_port"]),
        "read_tool_credentials": lambda: _read(target["credentials"]),
        # 6.5: o agente não lê a chave do usuário, logo não produz assinatura dele.
        "read_user_key": lambda: _read(target["user_key"]),
        "sign_as_user": lambda: _sign_as_user(target["user_key"]),
        "direct_network_to_llm": lambda: _tcp(target["llm_port"]),
        "read_kernel_process": lambda: _read(f"/proc/{target['kernel_pid']}/environ"),
        # cmdline não passa pela checagem de ptrace: só o namespace de PID o esconde.
        "read_kernel_cmdline": lambda: _read(f"/proc/{target['kernel_pid']}/cmdline"),
    }
    allowed: dict[str, Callable[[], str]] = {
        "kernel_head": lambda: _kernel_head(os.environ["LOGOS_KERNEL_SOCKET"]),
        "staging_write": lambda: _staging_write(os.environ["LOGOS_STAGING"]),
    }
    if "LOGOS_LLM_SOCKET" in os.environ:
        allowed["llm_via_proxy"] = lambda: _unix(os.environ["LOGOS_LLM_SOCKET"])
    report = {
        "forbidden": {name: attempt(action) for name, action in forbidden.items()},
        "allowed": {name: attempt(action) for name, action in allowed.items()},
    }
    print(json.dumps(report, sort_keys=True))


if __name__ == "__main__":
    main()
