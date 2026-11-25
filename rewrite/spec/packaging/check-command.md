---
kind: packaging
id: check-command
package: B0
sources: [packaging.md#2.6]
---

# `tools/check.py`

One command runs everything that does not cost money:

```
uv run python tools/check.py            # static + unit + workflows, about a minute
uv run python tools/check.py static     # build --check, ruff, lint suite, shellcheck, gen_reference --check
uv run python tools/check.py e2e-smoke  # starts real model sessions; needs credentials
```

The test layers it draws on:

- Layer 1, pytest for Python: [[packaging:tests-python]]
- Layer 2, the Node workflow harness: [[packaging:tests-workflows]]
- Layer 2b, conformance fixtures: [[packaging:tests-conformance]]
- Layer 3, lint over every shipped file: [[packaging:tests-lint]] and [[table:lint-checks]]
- Layer 4, headless end to end: [[packaging:tests-e2e]] (its smoke tier is what `check.py e2e-smoke` starts)

Package B0 delivers a first `tools/check.py` that runs and passes with nothing to check, so the command exists from
the start of the build ([[package:B0]]).
