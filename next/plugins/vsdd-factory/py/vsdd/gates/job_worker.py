"""The detached worker's entry script: fork, fix ``sys.path``, run the job.

Started by ``vsdd.gates.jobs`` as ``python [-B] job_worker.py <package_root>
<payload-json>`` in its own session (D2). It forks at once and lets the
intermediate parent exit, so the worker's parent is process 1 and no kill that
walks child processes reaches it. It removes its own directory from
``sys.path`` and puts the package root there instead (D22): the ``vsdd``
package must resolve once, as ``vsdd.gates.jobs``, never as a bare ``jobs``.
"""

from __future__ import annotations

import json
import os
import sys
from pathlib import Path


def main(argv: list[str]) -> None:
    package_root, payload = argv[0], argv[1]
    here = str(Path(__file__).resolve().parent)
    sys.path[:] = [p for p in sys.path if p and str(Path(p).resolve()) != here]
    sys.path.insert(0, package_root)
    if os.fork():
        os._exit(0)  # the intermediate exits; the worker is re-parented
    from vsdd.gates.jobs import worker_main

    worker_main(json.loads(payload))


if __name__ == "__main__":
    main(sys.argv[1:])
