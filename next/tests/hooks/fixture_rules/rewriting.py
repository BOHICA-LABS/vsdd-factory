"""A fixture rule whose row sets ``rewrites``: it returns ``updatedInput``."""

from vsdd.hooks.model import Rewrite


def check(ctx, cfg):
    tool_input = dict(ctx.tool_input)
    tool_input["fixture"] = True
    return Rewrite(tool_input=tool_input, reason="the fixture rewrite")
