"""Geradores determinísticos de casos para o teste diferencial da codificação canônica."""

from __future__ import annotations

import random
import unicodedata

from logos_client.canon import CanonError, CanonMap, Value, check_text, encode

POOL: list[tuple[int, int]] = [
    (0x20, 0x7E), (0x00, 0x1F), (0x7F, 0x7F), (0xA0, 0xFF), (0x300, 0x36F), (0x391, 0x3C9),
    (0x378, 0x379), (0x1100, 0x1112), (0x1161, 0x1175), (0x11A8, 0x11C2), (0xAC00, 0xD7A3),
    (0x4E00, 0x4E50), (0x958, 0x95F), (0x2126, 0x2126), (0x212A, 0x212B), (0xFB00, 0xFB06),
    (0x1E00, 0x1EFF), (0x1C89, 0x1C8A), (0x20C0, 0x20C1), (0xE000, 0xE010), (0xFDD0, 0xFDEF),
    (0xFFFE, 0xFFFF), (0x1F300, 0x1F340), (0x2FFFE, 0x2FFFF), (0x10FFFE, 0x10FFFF),
]  # fmt: skip

INTERESTING = [0x00, 0x17, 0x18, 0x19, 0x1A, 0x1B, 0x1C, 0x1F, 0x5F, 0xC0, 0xF4, 0xF7, 0xF9, 0xFF]
EDGES = [0, 1, 23, 24, 255, 256, 65535, 65536, 0xFFFFFFFF, 0x100000000, 2**64 - 2, 2**64 - 1]


def raw_string(rng: random.Random) -> str:
    chars = []
    for _ in range(rng.randrange(10)):
        lo, hi = rng.choice(POOL)
        chars.append(chr(rng.randint(lo, hi)))
    return "".join(chars)


def valid_text(rng: random.Random) -> str:
    kept = "".join(c for c in raw_string(rng) if unicodedata.category(c) != "Cn")
    s = unicodedata.normalize("NFC", kept)
    check_text(s)  # garante que o gerador só produz texto canônico
    return s


def random_u64(rng: random.Random) -> int:
    if rng.randrange(3) == 0:
        return rng.choice(EDGES)
    return rng.getrandbits(rng.randint(1, 64))


def random_value(rng: random.Random, depth_left: int) -> Value:
    kind = rng.randrange(6 if depth_left == 0 else 8)
    if kind == 0:
        return None
    if kind == 1:
        return rng.random() < 0.5
    if kind == 2:
        return random_u64(rng)
    if kind == 3:
        return -1 - random_u64(rng)
    if kind == 4:
        return bytes(rng.randrange(256) for _ in range(rng.randrange(20)))
    if kind == 5:
        return valid_text(rng)
    if kind == 6:
        return [random_value(rng, depth_left - 1) for _ in range(rng.randrange(5))]
    pairs: list[tuple[Value, Value]] = []
    for _ in range(rng.randrange(5)):
        pair = (random_value(rng, depth_left - 1), random_value(rng, depth_left - 1))
        try:
            CanonMap([*pairs, pair])
        except CanonError:
            continue  # chave repetida: descarta
        pairs.append(pair)
    return CanonMap(pairs)


def mutate(rng: random.Random, data: bytes) -> bytes:
    b = bytearray(data)
    for _ in range(rng.randint(1, 3)):
        op = rng.randrange(6)
        if op == 0 and b:
            b[rng.randrange(len(b))] ^= 1 << rng.randrange(8)
        elif op == 1 and b:
            b[rng.randrange(len(b))] = rng.choice(INTERESTING)
        elif op == 2:
            b.insert(rng.randint(0, len(b)), rng.randrange(256))
        elif op == 3 and b:
            del b[rng.randrange(len(b))]
        elif op == 4 and b:
            del b[rng.randrange(len(b)) :]
        elif op == 5 and b:
            i = rng.randrange(len(b))
            j = i + rng.randrange(len(b) - i) + 1
            b[j:j] = b[i:j]
    return bytes(b)


def raw_text_item(rng: random.Random) -> bytes:
    """Cabeçalho de texto + UTF-8 de uma string possivelmente não canônica."""
    raw = raw_string(rng).encode("utf-8")
    if len(raw) < 24:
        return bytes([0x60 | len(raw)]) + raw
    return bytes([0x78, len(raw)]) + raw


def cases(n: int, seed: int) -> list[bytes]:
    rng = random.Random(seed)
    out: list[bytes] = []
    for i in range(n):
        if i % 3 == 0:
            out.append(encode(random_value(rng, 4)))
        elif i % 3 == 1:
            out.append(mutate(rng, encode(random_value(rng, 4))))
        else:
            out.append(raw_text_item(rng))
    return out
