"""A script the tests run as "another process": it attaches once, or runs one worker.

Usage::

    python _jobs_helper.py attach <root> <wait> <spec-json>
    python _jobs_helper.py worker <root> <payload-json>
    python _jobs_helper.py env-start <root> <subject>

``attach`` prints one JSON line ``{"pending": <bool>, "output": <dict>}`` - the
outcome of one :func:`vsdd.gates.jobs.attach` call, with the retry string fixed
to what the helper was given. The tests use it to start a job from a process
that then goes away, which is how a real command call behaves.

``worker`` runs :func:`vsdd.gates.jobs.worker_main` in the foreground with the
given payload (which may carry ``lock_fd`` on a descriptor passed through
``pass_fds`` and an ``alarm_margin`` overriding the constant), and prints one
JSON line afterwards; on the alarm backstop the handler exits first, and the
assertions read the store's files instead.

``env-start`` calls :func:`vsdd.gates.env.start` once and prints the answer;
the tests kill it inside the settle window to leave its product behind.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path

sys.dont_write_bytecode = True

_HERE = Path(__file__).resolve()
_PLUGIN_PY = _HERE.parents[2] / "plugins" / "vsdd-factory" / "py"
sys.path.insert(0, str(_PLUGIN_PY))

from vsdd.gates.jobs import JobSpec, attach  # noqa: E402


def _spec_from(spec_json: dict) -> JobSpec:
    return JobSpec(
        command=spec_json["command"],
        subject=spec_json["subject"],
        commit=spec_json.get("commit", ""),
        work=spec_json.get("work", "vsdd.gates.jobs:run_shell"),
        args=spec_json["args"],
        error_shape=spec_json.get("error_shape", {}),
        session_id=spec_json.get("session_id"),
        limit_seconds=spec_json.get("limit_seconds"),
    )


def main(argv: list[str]) -> int:
    mode = argv[0]
    if mode == "attach":
        root, wait, spec_json = Path(argv[1]), int(argv[2]), json.loads(argv[3])
        outcome = attach(
            root, _spec_from(spec_json), wait=wait, retry=spec_json.get("retry", "retry from the helper")
        )
        print(json.dumps({"pending": outcome.pending, "output": outcome.output}))
        return 0
    if mode == "worker":
        root, payload = Path(argv[1]), json.loads(argv[2])
        payload.setdefault("root", str(root))
        from vsdd.gates.jobs import worker_main

        worker_main(payload)
        fd = payload.get("lock_fd")
        print(json.dumps({"inheritable": os.get_inheritable(fd)} if isinstance(fd, int) else {}))
        return 0
    if mode == "env-start":
        root, subject = Path(argv[1]), argv[2]
        from vsdd.gates import env

        output, code = env.start(root, subject, None)
        print(json.dumps({"output": output, "code": code}))
        return 0
    raise SystemExit(f"unknown mode {mode!r}")


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
