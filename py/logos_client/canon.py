"""Implementação de referência da codificação canônica `canon/v1` (spec/encoding.md).

Independente do crate Rust `logos-canon` (fora da TCB): serve ao teste diferencial. Escrita a
partir da especificação, com as mesmas regras e os mesmos nomes de erro.

Modelo de valores: ``None``, ``bool``, ``int``, ``bytes``, ``str``, ``list`` (array) e
:class:`CanonMap` (mapa). ``float`` e qualquer outro tipo não têm codificação.
"""

from __future__ import annotations

import unicodedata
from collections.abc import Iterable
from itertools import pairwise

CANON_VERSION = "canon/v1"
UNICODE_VERSION = "16.0.0"
MAX_DEPTH = 128

if unicodedata.unidata_version != UNICODE_VERSION:  # pragma: no cover - guarda de ambiente
    raise RuntimeError(
        f"logos_client.canon exige Unicode {UNICODE_VERSION} (Python 3.14); "
        f"este Python tem {unicodedata.unidata_version}"
    )

type Value = bool | int | bytes | str | list[Value] | CanonMap | None

_INT_MIN = -(2**64)
_INT_MAX = 2**64 - 1


class CanonError(Exception):
    """Erro de codificação/decodificação; ``kind`` usa os nomes de spec/encoding.md."""

    def __init__(self, kind: str) -> None:
        super().__init__(kind)
        self.kind = kind


class CanonMap:
    """Mapa canônico: entradas ordenadas pelos bytes codificados da chave, sem repetição."""

    entries: tuple[tuple[Value, Value], ...]

    def __init__(self, pairs: Iterable[tuple[Value, Value]]) -> None:
        keyed = sorted(((encode(k), k, v) for k, v in pairs), key=lambda t: t[0])
        for (a, _, _), (b, _, _) in pairwise(keyed):
            if a == b:
                raise CanonError("DuplicateKey")
        self.entries = tuple((k, v) for _, k, v in keyed)
        _check_depth(self)


def _depth(value: Value) -> int:
    if isinstance(value, list):
        return 1 + max((_depth(v) for v in value), default=0)
    if isinstance(value, CanonMap):
        return 1 + max((max(_depth(k), _depth(v)) for k, v in value.entries), default=0)
    return 0


def _check_depth(value: Value) -> None:
    if _depth(value) > MAX_DEPTH:
        raise CanonError("DepthExceeded")


def _head(major: int, arg: int) -> bytes:
    m = major << 5
    if arg < 24:
        return bytes([m | arg])
    if arg < 0x100:
        return bytes([m | 24, arg])
    if arg < 0x10000:
        return bytes([m | 25]) + arg.to_bytes(2, "big")
    if arg < 0x100000000:
        return bytes([m | 26]) + arg.to_bytes(4, "big")
    return bytes([m | 27]) + arg.to_bytes(8, "big")


def check_text(s: str) -> None:
    """Levanta CanonError se ``s`` não é texto canônico (ordem: atribuído, depois NFC)."""
    try:
        s.encode("utf-8")
    except UnicodeEncodeError:
        raise CanonError("InvalidUtf8") from None
    if any(unicodedata.category(c) == "Cn" for c in s):
        raise CanonError("UnassignedCodePoint")
    if not unicodedata.is_normalized("NFC", s):
        raise CanonError("NotNfc")


def encode(value: Value) -> bytes:
    """Codificação canônica de ``value``."""
    _check_depth(value)
    return _encode(value)


def _encode(value: Value) -> bytes:
    if value is None:
        return b"\xf6"
    if isinstance(value, bool):
        return b"\xf5" if value else b"\xf4"
    if isinstance(value, int):
        if not _INT_MIN <= value <= _INT_MAX:
            raise CanonError("IntOutOfRange")
        return _head(0, value) if value >= 0 else _head(1, -1 - value)
    if isinstance(value, bytes):
        return _head(2, len(value)) + value
    if isinstance(value, str):
        check_text(value)
        raw = value.encode("utf-8")
        return _head(3, len(raw)) + raw
    if isinstance(value, list):
        return _head(4, len(value)) + b"".join(_encode(v) for v in value)
    if isinstance(value, CanonMap):
        body = b"".join(_encode(k) + _encode(v) for k, v in value.entries)
        return _head(5, len(value.entries)) + body
    raise TypeError(f"tipo sem codificação canônica: {type(value).__name__}")


class _Reader:
    def __init__(self, data: bytes) -> None:
        self.data = data
        self.pos = 0

    def take(self, n: int) -> bytes:
        end = self.pos + n
        if end > len(self.data):
            raise CanonError("Eof")
        chunk = self.data[self.pos : end]
        self.pos = end
        return chunk

    def argument(self, info: int) -> int:
        if info < 24:
            return info
        size, minimum = {24: (1, 24), 25: (2, 0x100), 26: (4, 0x10000), 27: (8, 0x100000000)}[info]
        arg = int.from_bytes(self.take(size), "big")
        if arg < minimum:
            raise CanonError("NonMinimalHead")
        return arg

    def value(self, depth: int) -> Value:
        initial = self.take(1)[0]
        major, info = initial >> 5, initial & 0x1F
        if major == 6:
            raise CanonError("Tag")
        if major == 7:
            if info == 20:
                return False
            if info == 21:
                return True
            if info == 22:
                return None
            if 28 <= info <= 30:
                raise CanonError("ReservedInfo")
            if info == 31:
                raise CanonError("Indefinite")
            raise CanonError("UnsupportedSimple")
        if 28 <= info <= 30:
            raise CanonError("ReservedInfo")
        if info == 31:
            raise CanonError("Indefinite" if major >= 2 else "ReservedInfo")
        arg = self.argument(info)
        if major == 0:
            return arg
        if major == 1:
            return -1 - arg
        if major == 2:
            return self.take(arg)
        if major == 3:
            raw = self.take(arg)
            try:
                s = raw.decode("utf-8")
            except UnicodeDecodeError:
                raise CanonError("InvalidUtf8") from None
            check_text(s)
            return s
        if depth + 1 > MAX_DEPTH:
            raise CanonError("DepthExceeded")
        if major == 4:
            items: list[Value] = []
            for _ in range(arg):
                items.append(self.value(depth + 1))
            return items
        pairs: list[tuple[Value, Value]] = []
        previous: bytes | None = None
        for _ in range(arg):
            start = self.pos
            key = self.value(depth + 1)
            key_bytes = self.data[start : self.pos]
            if previous is not None:
                if previous == key_bytes:
                    raise CanonError("DuplicateKey")
                if previous > key_bytes:
                    raise CanonError("MapKeyOrder")
            previous = key_bytes
            pairs.append((key, self.value(depth + 1)))
        return CanonMap(pairs)


def decode(data: bytes) -> Value:
    """Decodificação estrita: levanta CanonError para qualquer coisa fora de ``canon/v1``."""
    reader = _Reader(data)
    value = reader.value(0)
    if reader.pos != len(data):
        raise CanonError("TrailingBytes")
    return value


def dump(value: Value) -> str:
    """Renderização textual determinística, idêntica à de ``impl Display for Value`` em Rust."""
    if value is None:
        return "null"
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, bytes):
        return "h'" + value.hex() + "'"
    if isinstance(value, str):
        out = ['t"']
        for c in value:
            if c == '"':
                out.append('\\"')
            elif c == "\\":
                out.append("\\\\")
            elif " " <= c <= "~":
                out.append(c)
            else:
                out.append(f"\\u{{{ord(c):x}}}")
        out.append('"')
        return "".join(out)
    if isinstance(value, list):
        return "[" + ",".join(dump(v) for v in value) + "]"
    return "{" + ",".join(f"{dump(k)}:{dump(v)}" for k, v in value.entries) + "}"


def verdict(data: bytes) -> str:
    """``ok:<dump>`` se ``data`` é canônico, ``err:<Kind>`` caso contrário (formato do diff)."""
    try:
        return "ok:" + dump(decode(data))
    except CanonError as e:
        return "err:" + e.kind
