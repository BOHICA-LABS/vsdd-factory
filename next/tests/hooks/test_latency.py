"""Gate latency: the median of twenty real launches stays under 120 ms
(hooks.md 2.4; the plan's test_gate_latency).

The measurement is sensitive to machine load: a failure while other builders
run is not a finding until it repeats on a quiet machine (ORDER.md 2)."""

from __future__ import annotations

import json
import statistics
import time

from tests.hooks import runner_helpers as helpers
from tests.hooks.test_runner import base_env, bash_call

LIMIT_MS = 120.0
RUNS = 20


def test_gate_latency(tmp_path):
    project = helpers.make_project(tmp_path)
    payload = json.dumps(bash_call(project, "echo hi"))

    from tests.hooks._helpers import LAUNCH_SH

    durations = []
    for _ in range(RUNS):
        started = time.perf_counter()
        result = helpers.launch(
            "sh", LAUNCH_SH, ["hook", "gate"], project,
            stdin_text=payload, env={"VSDD_EVENTS_DIR": str(project / "events")},
        )
        durations.append((time.perf_counter() - started) * 1000)
        assert result.returncode == 0, result.stderr

    median = statistics.median(durations)
    assert median < LIMIT_MS, f"median gate launch {median:.0f} ms over {RUNS} runs"
