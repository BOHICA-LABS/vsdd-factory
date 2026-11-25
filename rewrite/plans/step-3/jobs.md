# Step 3 plan: jobs and the product environment (package G1)

Brief: `rewrite/spec/packages/G1.md`. Design: `rewrite/spec/records/job.md`, `topics/long-commands.md`,
`commands/env-start.md`, `commands/env-stop.md`, `topics/caller-identity.md`, `topics/command-conventions.md`,
`tables/limits.toml`, `config/commands.md` (all under `rewrite/spec/`). Paths below are under `next/`;
`PY` is `plugins/vsdd-factory/py/vsdd`.

## 1. Scope

Built now: a job library in which the whole body of a long command runs in a detached worker process, and
the command only starts it, waits a bounded time and prints either the result or the pending answer; the
semaphore (`limits.parallel_gates`); the run-time limit (`limits.gate_minutes`); the runner for configured
project commands that the red and green gates (G2) will call for `commands.test`; and `vsdd env start` /
`vsdd env stop` with the functions that stop products and list their ports. No gate can depend on a long
foreground call, because no measuring happens in one. The env commands stay in the slice although the
sample starts no product: the step 5 demo method calls `vsdd env start`, and the checkpoint calls
`stop_products`.

Left for when the build widens:

- Hub projects (`--repo`, `.worktrees/_integration/<repo>/`).
- A readiness probe for a started product (decision D15), and the `{port}` check in `init` and `doctor`.
- Wiring by other packages: H9 calls `stop_products` on `SubagentStop`; H6 and the `secrets` rule call
  `product_ports`; S2c shows `list_jobs` and calls `prune` from `status --fix`.
- The survey's measurements as jobs (finding P3REQ-06; S3 second part).
- Raising `MAX_WAIT` to 540 once E2 proves a ten-minute foreground call (D10).
- A cap on the job log, and JSON Schema files for the job record and the env result (D19).

## 2. Files

Owned (only this unit writes them):

- `PY/gates/__init__.py` — a docstring. Later gate packages add modules beside it, not text in it; a
  module that registers a gate is named `gate_<name>.py`, which is what records' gate registry imports.
- `PY/gates/jobs.py` — keys, the record store, `attach`, the worker body, `JobContext`, the command
  environment, the semaphore, the product functions, `list_jobs`, `prune`.
- `PY/gates/job_worker.py` — the worker's entry script, about 15 lines: fork, fix `sys.path`, call
  `jobs.worker_main`.
- `PY/gates/env.py` — the handlers of `env start` and `env stop`.
- `tests/unit/test_jobs.py`, `tests/unit/test_env.py`, `tests/unit/_jobs_helper.py` (a script the tests run
  as "another process": it calls `attach` once and prints the outcome).

Shared: none. The two commands register through `PY/gates/cmd_env.py`, a module of S1's `cmd_` form that
edits no shared file. It needs S1's dispatcher and records' `caller`, so it is the second part of G1 with
its own plan, `jobs-b.md`. This plan needs neither and is built beside S1.

Not touched: `plugins/vsdd-factory/py/cli.py`, `schemas/`, `tools/check.py`, `launch.sh`, the sample
project.

## 3. Interfaces this unit provides

### 3.1 The store (private to G1; others use the functions)

Directory `<root>/.factory/local/jobs/`, created on first use. Per key: `<key>.json` (the record, at most
4,000 bytes, written to a temporary name and moved with `os.replace`), `<key>.log`, `<key>.result.json`
(jobs only, written the same way) and `<key>.lock` (D3). `slots/<n>.lock` are the semaphore, `ports.lock`
serialises port choice.

Record fields. Common: `v` (1), `key`, `kind` (`job` or `product`), `command`, `subject`, `commit`,
`state`, `created`, `started`, `finished` (UTC, whole seconds, or null), `pid`, `nonce` (random hex, new
for every start), `session_id`, `agent_id` (strings or null). Job only: `state` is `queued`, `running` or
`done`; `starts`, `collected`, `work`, `args`, `error_shape`, `limit_seconds`. Product only: `state` is
`starting` or `running`; `port`, `url`, `cwd`.

### 3.2 `vsdd.gates.jobs` — jobs

Constants: `DEFAULT_WAIT = 100`, `MAX_WAIT = 240`, `POLL = 0.2`, `RECORD_CAP = 4000`, `RETAIN_DAYS = 7`.

- `job_key(command: str, subject: str, commit: str) -> str` — the first 16 hex digits of the SHA-256 of
  the three joined by newlines. `command` is the command's words plus every option that changes the work
  (`gate red --refreeze`); it never holds `--run`, `--wait`, `--json` or `-C`. `commit` is the full SHA
  the job measures, or `""` for a job keyed by its subject alone (`story deliver`).
- `JobSpec` (frozen dataclass): `command`, `subject`, `commit`, `work: str`, `args: dict`,
  `error_shape: dict`, `session_id: str | None = None`, `limit_seconds: int | None = None`. `work` is
  `module:function` and must match `^vsdd(\.[a-z_][a-z0-9_]*)+:[a-z_][a-z0-9_]*$`. `limit_seconds` unset
  means `limits.gate_minutes * 60`; delivery passes its CI wait.
- `attach(root: Path, spec: JobSpec, *, wait: int | None = None, retry: str) -> Outcome`, where
  `Outcome` is a frozen dataclass with `pending: bool` and `output: dict`. Pending output is exactly
  `{"ok": True, "state": "pending", "retry": retry}` (the shipped `pending-output` schema). Otherwise
  `output` is the dict the work function returned, or a failure the job layer made:
  `{**spec.error_shape, "ok": False, "error": "<one line naming the log>"}`. The caller prints `output`
  and picks its own exit code; pending exits 0. Raises `ValueError` and writes nothing when the spec is
  malformed (bad `work`, empty `command` or `subject`, `args` not JSON, record over the cap) or `wait` is
  negative; `OSError` from an unwritable store passes up. It never raises for a failed job.
- `clamp_wait(seconds: int | None) -> int` — `None` gives 100; above `MAX_WAIT` gives `MAX_WAIT`;
  negative raises `ValueError`. `attach` applies it.
- The `retry` string is not built here: a command module passes S1's `inv.retry_command()`, the one
  place the command line is rebuilt.
- `list_jobs(root) -> list[dict]` — every record plus `alive: bool` (its lock is held) and, for a job,
  `has_result: bool`. Never raises on a torn or foreign file; it skips it.
- `prune(root, *, days: int = RETAIN_DAYS) -> list[str]` — deletes every file of each key whose lock is
  free and whose newest file is older than `days`; returns the keys. `attach` and `env.start` call it
  before they write a new record.

How `attach` behaves (the builder implements exactly this loop; `mine` is a nonce this call may collect):

1. Read the record. If a result file exists and `collected` is false, or the record's nonce is `mine`:
   set `collected`, return the result.
2. Try `flock(LOCK_EX | LOCK_NB)` on a fresh open of `<key>.lock`.
   - Held by another: a worker is alive. If no result file exists, `mine = record.nonce`.
   - Acquired: no worker is alive. Re-check step 1. If the record says `queued` or `running` and there is
     no result, the worker was lost: log `job.lost`, and if this call has already restarted once, write
     the failure result itself, release, return it. Otherwise delete the old result file, truncate the
     log, write a fresh record (`state: queued`, new nonce, `starts` + 1 after a loss, else 1), start the
     worker with the lock handed over (D2, D3), and set `mine`.
3. If the wait is used up, return pending. Sleep `POLL`, go to 1.

### 3.3 The work function and `JobContext`

A work function is `def f(ctx: JobContext) -> dict`; it returns the command's ordinary result, a JSON
object with a boolean `ok`. It runs in the worker. `ctx` has `root: Path`, `key`, `args: dict`,
`config` (the loaded `Config`), `log: Path`, `remaining() -> float`, and:

- `ctx.run(command: str, *, cwd: Path, env: dict[str, str] | None = None, output: Path | None = None)
  -> CommandResult(returncode: int, seconds: float, output: Path)`. Runs `/bin/sh -c <command>` in its
  own process group, stdin from `/dev/null`, in `command_env(root)` plus `env`. Standard output and error
  go to `output` (truncated first) when given, else are appended to the job log; the log always gets one
  line naming the command and one with its exit code. At the deadline it signals the group (TERM, 5
  seconds, KILL) and raises `JobTimeout`. An empty `command` raises `ValueError`.
- `command_env(root: Path, config: Config | None = None) -> dict[str, str]` (module level, also used by
  `env.start`) — D12.
- `run_shell(ctx)` — a built-in work function (`vsdd.gates.jobs:run_shell`): runs `ctx.args["command"]`
  in `ctx.args["cwd"]` and returns `{"ok": True, "returncode", "seconds"}`. The tests use it.

The worker: forks and lets its parent exit; makes the lock descriptor non-inheritable; takes a slot (D8);
writes `running`, `started`, `pid`; sets the deadline and `signal.alarm(limit + 30)`; resolves and calls
the function. `JobTimeout` gives the failure "ran longer than <n> minutes (limits.gate_minutes)"; any
other exception, or a return value that is not an object with a boolean `ok`, gives "the job crashed:
<Type>: <message>" with the traceback in the log. It writes the result file, then `done` and `finished`.
On SIGTERM it kills the running command's group and exits with no result.

### 3.4 `vsdd.gates.jobs` — products

- `stop_products(root, *, agent_id=None, session_id=None, subject=None, grace: float = 5.0)
  -> list[dict]` — a filter that is given must equal the record's field. For each matching product whose
  lock is held: `killpg(pid, SIGTERM)`, wait up to `grace` for the lock to come free, then `SIGKILL`;
  delete its record and lock file. Returns `{subject, port, pid}` for each one stopped. A matching record
  whose lock is already free is deleted and not returned. Never raises for a process that is gone.
  Callers: `env.stop`; S3's checkpoint, once per story of the run, with `subject=<story id>`; H9 with
  `agent_id` and a `grace` below its 4 s `SubagentStop` deadline (2 s).
- `product_ports(root, *, session_id=None) -> set[int]` — ports of products whose lock is held.

In every function of 3.2 to 3.5 the parameter written `root` takes a `vsdd.project.Project`, any object
with `.root`, or a `str` or `Path`, as records' functions do (a local helper: `jobs.py` imports no state
code). `jobs.py` imports the standard library, `vsdd.config`, `vsdd.project` and `vsdd.events` only, so
state or hook code may import it with no cycle.

### 3.5 The handlers of the two commands

Both handlers return `(output: dict, code: int)` with code 0 or 1; `jobs-b.md` wraps them in `Command`s.
`caller` is the caller marker as a mapping with `session_id`, `agent_id`, `agent_type`, `cwd`, or `None`
when the call has no marker.

`vsdd env start --subject <id>` — `env.start(root, subject: str, caller) -> (dict, int)`.

- Success, exit 0: `{"ok": true, "url": "http://127.0.0.1:<port>", "port": <int>, "pid": <int>}`.
- Exit 1 with `{"ok": false, "error", "log"}`: `commands.run` is empty; the subject has no worktree
  (D17); the process exited within `SETTLE_SECONDS` on three ports (the error carries the exit code and
  the last log lines); no port is free.
- A subject that does not match `^[A-Za-z0-9][A-Za-z0-9._-]{0,63}$` raises `ValueError`, which the
  command module turns into exit 2.
- Steps: key `job_key("env", subject, "")`. Lock held and the record `running` at the worktree's head
  commit: return its answer, start nothing. Held at another commit: `stop_products(subject=...)`, then
  start. Held and still `starting`: poll 5 seconds, then answer from the record or fail. Otherwise:
  choose a port (D15), write the record with the caller's `agent_id` and `session_id`, start
  `/bin/sh -c` with `{port}` replaced and `VSDD_PORT` set, in its own session, the lock descriptor passed
  to it, output to the log; watch for `SETTLE_SECONDS = 2`; write `running`.

`vsdd env stop` (no option, D13) — `env.stop(root, caller, *, terminal: bool) -> (dict, int)`. With a
marker it calls `stop_products(root, agent_id=..., session_id=...)` from it. With none it stops every
product only when `terminal` is true (a person); otherwise it stops nothing and answers exit 1 (D16).
Exit 0: `{"ok": true, "stopped": [{subject, port, pid}]}`. Exit 1: `{"ok": false, "stopped": [], "error"}`
when nothing matched or the call was refused.

## 4. Interfaces this unit needs

- B3, already under `next/`: `vsdd.config.load`, `Config.get("limits", "parallel_gates")`,
  `Config.get("limits", "gate_minutes")`, `Config.command("run")`, `Config.get("commands")["env"]`;
  `vsdd.project.FACTORY_DIR` and `STORY_GRAMMAR`; `vsdd.events.EventLog` and `events_dir_for`.
- B3's `vsdd.project.classify_worktree`, the one place the worktree layout is written (D17).
- S1 and records: nothing in this part. The command frame (`-C`, `--json`, the global `--wait`, exit 2
  outside a project) and `caller` are used by the second part (`jobs-b.md`), never by `jobs.py` or
  `env.py`. A gate that attaches to a job passes `retry=inv.retry_command()` and `wait=inv.wait`.
- The hook runner writes the marker; without one `caller` is `None` and D16 applies.
- B6's lint as it stands: standard library only, none of the banned network modules, ruff clean, nothing
  generated under the plugin tree.
- Nothing from S3, S4, C2 or B4b.

## 5. Design decisions

- **D1. The work runs in the worker.** The spec says the command "runs its work detached"; it does not
  say what the work is. The whole body, evidence write included, is a function the worker calls. A killed
  or short foreground call then loses nothing, and a gate never depends on a foreground call's length.
- **D2. Detachment.** `Popen(start_new_session=True)` of `job_worker.py`, which forks at once and lets
  its parent exit, so the worker's parent is process 1. It survives a kill of the caller's process group
  and a kill that walks child processes. Checked by prototype on macOS.
- **D3. Liveness is a file lock.** The starter takes `flock` on `<key>.lock` and passes the descriptor to
  the worker (or product). "Alive" means the lock cannot be taken. No process-id reuse, no heartbeat, and
  a kill or a reboot frees it. The worker closes it to its own child commands, so a leftover child never
  makes a dead job look alive.
- **D4. The result is its own file.** A gate result can exceed the record's 4,000-byte cap.
- **D5. A result is served once.** The spec keeps a job for 7 days but says a failing gate is never
  returned without measuring again, and `story deliver` must run again after a repair with the same key.
  So a call that finds a result already collected starts a new job. The nonce lets every call that was
  waiting on the same job still receive its result.
- **D6. A lost worker is restarted once per call**, then reported as `ok: false`. A reboot in the middle
  of a gate costs the run nothing; a job that dies every time is loud.
- **D7. Failures keep the command's shape.** `error_shape` carries the fields the command's schema
  requires (for a gate: `gate`, `subject`, `passed: false`, `sha`, `evidence`, `failures`).
- **D8. The semaphore** is `parallel_gates` slot files, each a `flock` the worker holds while it runs. A
  dead worker's slot frees itself. Time in the queue does not count against the run-time limit. Products
  hold no slot: a demo's product must run beside its gate.
- **D9. The time limit is the worker's.** Each configured command gets the remaining time; an alarm 30
  seconds past the limit is the backstop.
- **D10. One call waits at most 240 seconds.** The spec allows 540, but only a four-minute call is
  proven. A larger `--wait` is clamped without error, so prompts need no change, and `MAX_WAIT` is the
  one line to raise.
- **D11. The key** leaves out `--run`, so a new run re-attaches to a job an earlier run left.
- **D12. Configured commands run in the person's environment, not the tool's.** `uv run --script` puts
  its own environment first on `PATH` and sets `VIRTUAL_ENV`, and the launcher sets Python variables.
  `command_env` removes `PYTHONSAFEPATH`, `PYTHONUTF8`, `PYTHONPYCACHEPREFIX`, `UV_RUN_RECURSION_DEPTH`,
  every `VSDD_*` name, `VIRTUAL_ENV` when it equals `sys.prefix`, and the `PATH` entry `<sys.prefix>/bin`;
  then adds `[commands.env]`.
- **D13. `env stop` takes no argument**, as the blueprint has it; the caller selects what stops.
- **D14. One product per subject.** Keyed by subject, with the commit in the record: a second start at
  the same commit returns the first, at a new commit replaces it. Nothing leaks and no demo runs old code.
- **D15. The port is chosen without a socket.** The lint bans `socket` in shipped Python. The port is the
  lowest in 28100 to 28199 that no live product record holds, chosen under `ports.lock`. A product that
  exits within 2 seconds (a taken port, a broken command) is retried on the next port, three in all.
  `ok: true` therefore means started, not yet listening; the caller's first request retries.
- **D16. No marker is not a refusal for `env start`.** A person's terminal has none: the product is
  recorded with null ids. `env stop` without a marker stops every product only from a terminal. From a
  tool call it stops nothing and exits 1, naming the form that works, `vsdd -C <dir> env stop` as the
  whole command: the hook marks only a single-segment call, so an agent's `cd x && vsdd env stop` would
  otherwise stop other agents' products in silence.
- **D17. Where the product runs.** A story id runs in its story worktree, anything else in the
  integration worktree; both paths come from `vsdd.project.classify_worktree(...).path` under the root,
  as the survey's worktree library takes them, so `.worktrees/<id>` is written once. A missing directory
  is the negative answer. The head is read with `git -C <dir> rev-parse HEAD`, empty when that fails.
- **D18. Exit codes.** Every refusal of `env start` and `env stop` is a negative answer, exit 1.
- **D19. No schema file.** The amendment left the job record as prose because only G1 reads it; the
  fields are fixed in 3.1 and checked in code.
- **D20. Placement.** `py/vsdd/gates/` (`packaging/python-package.md`: "gate commands and jobs").
- **D21. Events.** `job.started`, `job.done`, `job.lost`, `env.started`, `env.stopped`, with `id` (the
  key), `command` and `detail`, written only when a session id is known, because the writer requires one.
- **D22. No bytecode in the plugin tree.** The worker is started with `-B` when the parent has
  `sys.dont_write_bytecode` set, and removes its own directory from `sys.path`.

## 6. Findings

- **P3REQ-06** (survey measurements are not jobs): not at slice depth; S3, second part. `attach` takes no
  command-line input and returns pending to any caller, so the survey can use it unchanged.
- **AU-S2-12** (`env-stop` gives two syntaxes): the code has the one syntax, D13: the handler takes no
  subject, and `jobs-b.md` closes the finding with `test_cli_env_stop_rejects_subject`. Removing the
  commentary from `rewrite/spec/commands/env-stop.md` is the orchestrator's.

P3SEAM-20 left the list while this plan was written (the topic now makes `--wait` global); the code side
is S1's frame, and `jobs-b.md`'s `test_cli_env_commands` covers it.

## 7. Tests

`tests/unit/test_jobs.py` (jobs use `run_shell`; a project is a temporary directory with a `factory.toml`):

- `test_job_key_is_stable_and_distinct` — same parts, same 16-hex key; any part changed, another key.
- `test_first_call_starts_and_returns_result` — `attach` returns the work function's object; record `done`.
- `test_same_command_twice_starts_one_job` — `wait=0` gives exactly the pending object (valid against
  `pending-output`); the next call returns the result; the command ran once. The named fixture test.
- `test_second_process_reattaches` — the helper process starts the job and exits; this process collects.
- `test_job_outlives_killed_starter` — the helper's process group is killed while it waits; the result
  still arrives, and the worker's parent is not the helper.
- `test_collected_result_is_not_served_again` — after a result was returned the same call runs the work
  again; a result nobody collected is served to the next call.
- `test_two_waiters_get_one_result` — two calls waiting on one key both return it; one run.
- `test_lost_worker_restarts_once` — the worker is killed with SIGKILL; the next call finishes the job,
  `starts` is 2.
- `test_job_that_keeps_dying_fails_loudly` — a job that kills its own worker returns `ok: false` with the
  `error_shape` fields and an error naming the log. The loud-failure test.
- `test_semaphore_caps_running_jobs` — `parallel_gates = 1`: the second job is `queued` while the first
  runs, and starts after it finishes.
- `test_time_limit_stops_the_job` — `limit_seconds=1` on `sleep 30`: `ok: false`, the error names the
  limit, the sleep is gone.
- `test_work_crash_keeps_command_shape` — a missing argument: `ok: false`, the exception type in the
  error, the traceback in the log.
- `test_bad_spec_is_refused` — `work` outside `vsdd.`, unserialisable `args`, an oversize record:
  `ValueError`, nothing written.
- `test_context_run_writes_output_and_log` — `output=` receives the command's output; exit code returned.
- `test_command_env_scrubs_tool_variables` — D12, and `[commands.env]` is added.
- `test_clamp_wait` — `None` is 100, 540 is 240, 0 is 0, negative raises.
- `test_list_jobs_and_prune` — a live job is listed `alive` and kept; an old finished key is removed.

`tests/unit/test_env.py`:

- `test_start_records_caller_and_port` — the four output fields; the record holds `agent_id` and
  `session_id`; `{port}` and `$VSDD_PORT` both reached the command.
- `test_start_is_idempotent_at_same_commit`; `test_start_replaces_product_of_older_commit`.
- `test_two_subjects_get_different_ports`.
- `test_start_fails_loudly_when_command_exits` — exit 1, the exit code and log tail, no record left.
- `test_start_without_run_command_is_negative`; `test_start_without_worktree_is_negative`;
  `test_start_bad_subject_raises`.
- `test_stop_stops_only_the_callers_products` — agent A's product is gone, B's lives.
- `test_stop_without_marker_needs_a_terminal` — from a terminal every product stops; without one nothing
  stops, exit 1, and the error names the single-segment form. The loud-failure test of D16.
- `test_stop_with_nothing_running_is_negative`.
- `test_stop_products_by_subject_session_and_ports` — what the checkpoint and the hooks will call, with
  a `Project`-like object as well as a path; a `grace` of 2 s is honoured.

The command-line tests are in `jobs-b.md`.

## 8. Order of work

Before the builder starts: the amended foundations (B3's `config` and `events`), and B6's lint in place so
`tools/check.py` is the bar. Nothing else: this unit runs beside S1 and the hook runner (`ORDER.md`), and
the survey waits for it because the checkpoint calls `stop_products`.

1. `gates/__init__.py`; in `jobs.py` the key, the record store, the lock helpers, `command_env`,
   `clamp_wait`, with their tests.
2. `job_worker.py`, `worker_main`, `JobContext`, `run_shell`, then `attach`; tests up to
   `test_two_waiters_get_one_result`.
3. Lost workers, the semaphore, the time limit, the crash path, events; their tests.
4. `list_jobs`, `prune`, `stop_products`, `product_ports`.
5. `env.py` and `test_env.py` at handler level.
6. `cd next && uv run --no-sync pytest tests/unit/test_jobs.py tests/unit/test_env.py`, then
   `uv run --no-sync python tools/check.py`. Both suites together run in under a minute.
