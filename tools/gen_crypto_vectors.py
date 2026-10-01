#!/usr/bin/env python3
"""Gera os vetores de hash/KeyId/assinatura com domínio, de forma independente do Rust.

As tags vêm da tabela de spec/domains.md; o SHA-256 vem de `hashlib`, o Ed25519 de
`cryptography` (OpenSSL) e a codificação canônica de py/logos_client/canon.py. Nada aqui usa o
crate Rust, então os vetores testam o Rust contra uma segunda implementação.

Uso:  uv run python tools/gen_crypto_vectors.py          # reescreve os arquivos
      uv run python tools/gen_crypto_vectors.py --check  # falha se estiverem desatualizados
"""

from __future__ import annotations

import hashlib
import re
import sys
from pathlib import Path

from cryptography.hazmat.primitives.asymmetric.ed25519 import Ed25519PrivateKey

from logos_client import canon

ROOT = Path(__file__).resolve().parent.parent
SPEC = ROOT / "spec/domains.md"
RFC = ROOT / "spec/vectors/crypto/ed25519-rfc8032.tsv"
OUT_HASH = ROOT / "spec/vectors/domains/hash.tsv"
OUT_KEYID = ROOT / "spec/vectors/domains/keyid.tsv"
OUT_SIG = ROOT / "spec/vectors/crypto/sig-domain.tsv"

MESSAGES = {
    "empty": b"",
    "abc": b"abc",
    "canon-map": canon.encode(canon.CanonMap([("a", 1)])),
    "pattern-100": bytes(range(100)),
}


def spec_tags() -> tuple[dict[str, bytes], dict[str, bytes]]:
    """(tags de hash por nome, tags de assinatura por nome), lidas da tabela do spec."""
    hash_tags: dict[str, bytes] = {}
    sig_tags: dict[str, bytes] = {}
    for line in SPEC.read_text().splitlines():
        m = re.match(r"\| ([a-z]+) \| `(logos/[a-z0-9/]+)` \|", line)
        if not m:
            continue
        name, tag = m.group(1), m.group(2).encode() + b"\x00"
        (sig_tags if tag.startswith(b"logos/sig/") else hash_tags)[name] = tag
    return hash_tags, sig_tags


def render_hash() -> str:
    hash_tags, _ = spec_tags()
    rows = [
        "# domínio<TAB>mensagem(nome)<TAB>mensagem(hex)<TAB>SHA-256(tag ‖ mensagem)"
        " — spec/domains.md §1"
    ]
    for name, tag in hash_tags.items():
        for mname, msg in MESSAGES.items():
            digest = hashlib.sha256(tag + msg).hexdigest()
            rows.append(f"{name}\t{mname}\t{msg.hex()}\t{digest}")
    return "\n".join(rows) + "\n"


def rfc_vectors() -> list[tuple[str, bytes, bytes]]:
    out = []
    for line in RFC.read_text().splitlines():
        if line and not line.startswith("#"):
            name, seed, pub, *_ = line.split("\t")
            out.append((name, bytes.fromhex(seed), bytes.fromhex(pub)))
    return out


def render_keyid() -> str:
    hash_tags, _ = spec_tags()
    rows = [
        "# nome<TAB>chave pública(hex)<TAB>canon(PublicKey)(hex)<TAB>KeyId(hex)"
        " — spec/domains.md §2"
    ]
    for name, _, pub in rfc_vectors():
        encoded = canon.encode(canon.CanonMap([("alg", "ed25519"), ("key", pub)]))
        key_id = hashlib.sha256(hash_tags["key"] + encoded).hexdigest()
        rows.append(f"{name}\t{pub.hex()}\t{encoded.hex()}\t{key_id}")
    return "\n".join(rows) + "\n"


def render_sig() -> str:
    _, sig_tags = spec_tags()
    rows = [
        "# semente(hex)<TAB>domínio<TAB>mensagem(hex)<TAB>chave pública(hex)<TAB>assinatura(hex)"
        " — spec/domains.md §3"
    ]
    for _, seed, pub in rfc_vectors()[:3]:
        key = Ed25519PrivateKey.from_private_bytes(seed)
        for dom, tag in sig_tags.items():
            for msg in (b"", b"abc"):
                sig = key.sign(tag + msg)
                rows.append(f"{seed.hex()}\t{dom}\t{msg.hex()}\t{pub.hex()}\t{sig.hex()}")
    return "\n".join(rows) + "\n"


def main() -> int:
    outputs = {OUT_HASH: render_hash(), OUT_KEYID: render_keyid(), OUT_SIG: render_sig()}
    if "--check" in sys.argv:
        stale = [p.name for p, text in outputs.items() if not p.exists() or p.read_text() != text]
        if stale:
            print(
                f"erro: desatualizados: {', '.join(stale)}; rode tools/gen_crypto_vectors.py",
                file=sys.stderr,
            )
            return 1
        print("ok: vetores de domínio/KeyId/assinatura atualizados")
        return 0
    for path, text in outputs.items():
        path.write_text(text)
        print(f"escrito {path.relative_to(ROOT)} ({text.count(chr(10)) - 1} vetores)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
