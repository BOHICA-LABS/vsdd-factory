"""A fixture rule that sleeps past the deadline, catching Exception — the
deadline must still land (HD4)."""

import time


def check(ctx, cfg):
    seconds = float(ctx.tool_input.get("sleep", "30"))
    try:
        time.sleep(seconds)
    except Exception:
        pass  # the alarm is a BaseException; this catch cannot swallow it
    return None
