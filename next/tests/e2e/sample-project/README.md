# tally

The sample project for the vsdd-factory end-to-end test tiers: a Python
package of one module with standard-library `unittest` tests. It needs
nothing from the network and nothing that is not on a stock macOS or Linux
developer machine with `uv`.

## Layout

```
tally/stats.py        the product: mean(), variance()
tests/test_stats.py   the unittest suite
tools/junit_runner.py runs the suite, writes build/junit.xml (JUnit XML)
tools/lint.py         lint and format checks
.factory/factory.toml the project configuration for a factory project
```

## Commands

Declared in `.factory/factory.toml` under `[commands]`; each runs from the
project root.

```
uv run --no-project python tools/junit_runner.py     # test  -> build/junit.xml
uv run --no-project python tools/lint.py             # lint
uv run --no-project python tools/lint.py --format    # format_check
```

There is no build step, no runtime, no UI and no third-party dependency.

The runner collects every `unittest.TestCase` in `tests/` and in the
`test_*.py` files beside the `tally` package, and every module-level `test_*`
function of those modules; a test function with required parameters — or one
that is a coroutine, an async generator or a generator, which the runner
would call without awaiting or iterating — is reported as an error case, and
a test module that fails to import (whatever the failure: a missing module,
a syntax error, a name error) as an ImportError case, so `build/junit.xml`
holds every test of a run.

## Planned story: add `median()`

Specified, not yet implemented; no test covers it. This is the standing
one-story-room the end-to-end story tier delivers ("add `median()`"),
from the `story-ready` state of this project.

`median(values) -> float`, in `tally/stats.py` beside `mean` and
`variance`:

- requires at least one value; raises `ValueError` for an empty sequence,
  exactly as `mean` and `variance` do;
- accepts any iterable and copies it before sorting;
- with an odd count, returns the middle element of the sorted values;
- with an even count, returns the mean of the two middle elements;
- is exported from `tally/__init__.py` next to `mean` and `variance`.

The story that implements it adds the tests for it in the same run; until
then `tests/` deliberately contains nothing for `median()`.
