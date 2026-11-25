"""A fixture rule that advises."""

from vsdd.hooks.model import Advise


def check(ctx, cfg):
    return Advise("fixture_advice", "consider the fixture alternative")
