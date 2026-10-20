"""Upload candidato por diretório ancorado; não abre CAS privado nem credenciais."""

from __future__ import annotations

import hashlib
import os
import secrets
import stat
from contextlib import suppress
from pathlib import Path
from typing import Literal

from logos_client.protocol import StagedRef

MAX_OBJECT = 64 * 1024 * 1024


class StagingUploader:
    """Publicação síncrona limitada. Usar somente o diretório provisionado do agente."""

    def __init__(self, directory: Path, *, max_bytes: int = MAX_OBJECT) -> None:
        if type(max_bytes) is not int or not 0 < max_bytes <= MAX_OBJECT:
            raise ValueError("staging byte limit")
        self.directory, self.max_bytes = directory, max_bytes

    def put(self, data: bytes, domain: Literal["certificate", "model"]) -> StagedRef:
        if type(data) is not bytes or len(data) > self.max_bytes:
            raise ValueError("staging bytes/type limit")
        if domain not in ("certificate", "model"):
            raise ValueError("unknown staging domain")
        digest = hashlib.sha256(f"logos/{domain}/v1\0".encode() + data).digest()
        directory = os.open(
            self.directory, os.O_RDONLY | os.O_DIRECTORY | os.O_NOFOLLOW | os.O_CLOEXEC
        )
        temporary: str | None = None
        published: str | None = None
        try:
            meta = os.fstat(directory)
            if meta.st_uid != os.geteuid() or stat.S_IMODE(meta.st_mode) & 0o022:
                raise PermissionError("staging UID/permissions")
            name = secrets.token_hex(32)
            candidate = ".upload-" + secrets.token_hex(32)
            fd = os.open(
                candidate,
                os.O_WRONLY | os.O_CREAT | os.O_EXCL | os.O_NOFOLLOW | os.O_CLOEXEC,
                0o600,
                dir_fd=directory,
            )
            temporary = candidate
            try:
                view = memoryview(data)
                while view:
                    written = os.write(fd, view)
                    if written <= 0:
                        raise OSError("short staging write")
                    view = view[written:]
                os.fsync(fd)
            finally:
                os.close(fd)
            # Atomic no-overwrite publication; return only after unlinking the temporary.
            os.link(
                temporary, name, src_dir_fd=directory, dst_dir_fd=directory, follow_symlinks=False
            )
            published = name
            os.unlink(temporary, dir_fd=directory)
            temporary = None
            os.fsync(directory)
            return StagedRef(name=name, hash=digest)
        except OSError as error:
            if published is not None:
                error.add_note(f"staging publication may exist: {published}; no automatic retry")
            raise
        finally:
            try:
                if temporary is not None:
                    # Only the private temporary name is ours; never remove a published upload.
                    with suppress(FileNotFoundError):
                        os.unlink(temporary, dir_fd=directory)
            finally:
                os.close(directory)
