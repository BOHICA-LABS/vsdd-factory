"""A fixture gate rule that always blocks: at advise strength the runner
relaxes its block to advice and logs ``rule.would_block``."""

from vsdd.hooks.model import Block


def check(ctx, cfg):
    return Block("strength_block", "blocked at full strength", "lower the strength")
