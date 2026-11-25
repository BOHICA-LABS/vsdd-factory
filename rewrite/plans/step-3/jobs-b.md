# Step 3 plan: jobs, second part: the `env` commands (G1)

Paths are under `next/`; `PY` is `plugins/vsdd-factory/py/vsdd`. Spec files are under `rewrite/spec/`.
This plan continues `jobs.md`, the first part, which is built and reported before it. Names it does not
define (`env.start`, `env.stop`, the D decisions) are that plan's.

## 1. Scope

The command module that puts `vsdd env start` and `vsdd env stop` on the command line, and their two
command-line tests. The first part built the handlers. This part could not be built with it, because a
command registers through S1's dispatcher and learns its caller from records' marker reader.

Left for when the build widens: what `jobs.md` lists.

## 2. Files

Owned, new:

- `PY/gates/cmd_env.py`: `COMMANDS`, two `configure` functions, two `run` adapters.
- `tests/unit/test_cmd_env.py`.

Edited: none. Not touched: `plugins/vsdd-factory/py/cli.py`, `PY/cli/`, `PY/state/`, `PY/gates/jobs.py`
and `env.py` (a fault found in them is reported, and fixed there only if the fix is a line this part
needs), `schemas/`, `tools/`.

## 3. Interfaces this unit provides

`PY/gates/cmd_env.py` holds, in S1's form (state-core.md 3.1):

```python
COMMANDS = (
    Command(words=("env", "start"), summary=..., run=run_start, configure=configure_start,
            needs="initialised"),
    Command(words=("env", "stop"), summary=..., run=run_stop, configure=configure_stop,
            needs="initialised"),
)
```

- `configure_start` adds the required option `--subject <id>`. `configure_stop` adds nothing, so
  `vsdd env stop --subject x` is a usage error, exit 2.
- Both adapters read the caller once: `caller(inv.caller_key, project=inv.root)`, imported from
  `vsdd.state.callers` inside the function. `None` (no marker) is passed on as `None`.
- `run_start(inv, args) -> Reply`: `Reply.of(*env.start(inv.root, args.subject, caller))`. A `ValueError`
  from the handler (a malformed subject) becomes `raise Refuse(str(error))`, exit 2.
- `run_stop(inv, args) -> Reply`: `Reply.of(*env.stop(inv.root, caller, terminal=inv.stdin_tty))`.
- `needs` is `initialised`, not `mounted`: a product must be stoppable when the state mount is broken, and
  neither command writes tracked state.
- `--json` and `--wait <seconds>` are the dispatcher's; both commands accept `--wait` and ignore it.
- Neither command is relayed by a script, so neither sets `output`.

Output and exit codes are the handlers' (`jobs.md` 3.5): `env start` prints `{ok, url, port, pid}` or
`{ok: false, error, log}`; `env stop` prints `{ok, stopped}` or `{ok: false, stopped: [], error}`.

## 4. Interfaces this unit needs

- The first part (`jobs.md`): `env.start(root, subject, caller) -> (dict, int)`, which raises `ValueError`
  for a malformed subject, and `env.stop(root, caller, *, terminal: bool) -> (dict, int)`.
- S1 (state-core.md 3.1): `Command`, `Invocation` (`root`, `caller_key`, `stdin_tty`), `Reply.of`,
  `Refuse`; `main` for tests; `tests/unit/_repo.make_project`.
- Records (records.md): `vsdd.state.callers.caller(key, *, project) -> dict | None`, which returns
  `{session_id, agent_id, agent_type, cwd}` and deletes the marker.
- The hook runner writes the marker; the tests write that file themselves.

## 5. Design decisions

- D1. The module is `gates/cmd_env.py` and not a hook in `env.py`: S1 finds commands by the file name
  `cmd_<family>.py`, and a module named `env.py` is never listed.
- D2. The adapters hold no logic. Every refusal that is an answer (no run command, no worktree, nothing to
  stop, a stop without a marker from a tool call) is the handler's exit 1; only a malformed argument is
  exit 2.
- D3. `terminal` comes from `inv.stdin_tty`, the same fact S1's `require_person` reads, so a test injects
  it through `main(..., stdin_tty=...)`.

## 6. Findings

- **AU-S2-12** (`env-stop` gives two syntaxes): closed with the first part's handler:
  `test_cli_env_stop_rejects_subject`.
- **P3SEAM-20** (off the list; `--wait` is global): `test_cli_env_commands` passes `--wait 540`.

## 7. Tests

`tests/unit/test_cmd_env.py`, on a project from `_repo.make_project` with a story worktree and a
`commands.run` set by `_repo.set_keys`:

- `test_cli_env_commands`: through `sh py/launch.sh cli -C <dir> env start --subject S-1.01 --json --wait
  540`, with a marker file and `VSDD_CALLER`: one JSON object, exit 0, and the product record holds the
  marker's `agent_id` and `session_id`; then `env stop --json` the same way stops it. Skipped without `uv`.
- `test_cli_env_stop_rejects_subject`: exit 2.
- `test_start_bad_subject_exits_2`: in process through `main`.
- `test_stop_without_marker`: in process: `stdin_tty=True` stops every product; `stdin_tty=False` stops
  nothing, exits 1 and names the single-segment form.
- `test_env_outside_a_project_exits_2`.
- `test_env_commands_register`: `commands()` holds both words.

## 8. Order of work

Before the builder starts (`ORDER.md`): the first part, S1 and records have reported. It runs beside
survey and `hook-runner-b`, and writes two files of its own.

1. `cmd_env.py` and `test_cmd_env.py`.
2. `cd next && uv run --no-sync pytest tests/unit/test_cmd_env.py tests/unit/test_env.py
   tests/unit/test_jobs.py`, then every check.
