"""A fixture rule that crashes with a divide-by-zero, every time."""


def check(ctx, cfg):
    return 1 / 0
