"""The rules (H2 to H9): one module per rule, ``vsdd/rules/<id with
underscores>.py``, each exposing ``check(ctx, cfg)``.

At this depth the package ships no rule; the fixture rules under the test
suite prove the runner in process and through the real launcher. A rule's one
declaration is its row in :data:`vsdd.hooks.registry.RULES` (HD1) — there is
no decorator and no directory scanning.
"""
