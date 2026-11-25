"""The tool's one git subprocess helper (state-core.md 3.3).

Every git call the product makes goes through :func:`git`: it sets ``LC_ALL=C``
so messages and paths never depend on the person's locale, and
``GIT_TERMINAL_PROMPT=0`` so a credential prompt can never hang a hook. The
repository-locating ``GIT_*`` variables are stripped from the inherited
environment, so ``cwd`` decides the repository even under a hook or script
that exports ``GIT_DIR``.
"""

from __future__ import annotations

import os
import subprocess

__all__ = []


class GitError(Exception):
    """A git command failed (or timed out). ``returncode`` and ``stderr``."""

    def __init__(self, message: str, returncode: int, stderr: str):
        super().__init__(message)
        self.returncode = returncode
        self.stderr = stderr


#: Environment variables that tell git where its repository lives. They are
#: stripped, so ``git(cwd, ...)`` always addresses ``cwd``; a caller that
#: really wants one passes it in ``env`` and wins over the stripping.
_REPO_LOCATING_KEYS = (
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
)


def git(
    cwd,
    *args: str,
    check: bool = True,
    timeout: int = 60,
    input: str | None = None,  # noqa: A002 - the subprocess spelling
    env: dict | None = None,
) -> str:
    """Run ``git <args>`` in ``cwd``; stdout without its trailing newline.

    Raises :class:`GitError` with ``.returncode`` and ``.stderr`` on a non-zero
    exit when ``check``, on a timeout, and when ``git`` itself cannot run.
    """
    full = {key: value for key, value in os.environ.items() if key not in _REPO_LOCATING_KEYS}
    full["LC_ALL"] = "C"
    full["GIT_TERMINAL_PROMPT"] = "0"
    if env:
        full.update(env)
    try:
        proc = subprocess.run(
            ["git", *(str(a) for a in args)],
            cwd=str(cwd),
            capture_output=True,
            text=True,
            input=input,
            timeout=timeout,
            env=full,
        )
    except subprocess.TimeoutExpired as exc:
        # TimeoutExpired carries raw bytes even under text=True.
        detail = exc.stderr or b""
        if isinstance(detail, bytes):
            detail = detail.decode("utf-8", "replace")
        raise GitError(
            f"git {' '.join(map(str, args))} timed out after {timeout} s: {str(detail).strip()}",
            124,
            str(detail),
        ) from exc
    except FileNotFoundError as exc:
        raise GitError(
            f"git is not available: {exc}", 127, f"git is not available: {exc}"
        ) from exc
    if proc.returncode != 0 and check:
        raise GitError(
            f"git {' '.join(map(str, args))} exited {proc.returncode}: {proc.stderr.strip()}",
            proc.returncode,
            proc.stderr,
        )
    return proc.stdout[:-1] if proc.stdout.endswith("\n") else proc.stdout
