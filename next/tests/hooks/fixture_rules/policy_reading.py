"""A fixture rule that reads the wall row: its handle on the policy tables."""

def check(ctx, cfg):
    row = ctx.row
    return None if row is not None else None
