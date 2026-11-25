"""A second rewriting fixture: another rewrite, with a reason of its own."""

from vsdd.hooks.model import Rewrite


def check(ctx, cfg):
    tool_input = dict(ctx.tool_input)
    tool_input["other"] = True
    return Rewrite(tool_input=tool_input, reason="the second rewrite")
