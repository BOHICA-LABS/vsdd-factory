# Step 3 plan: the state core (package S1) at slice depth

`P/` is `next/plugins/vsdd-factory/`, `T/` is `next/tests/`, "spec" is `rewrite/spec/`. Brief: spec
`packages/S1.md`. Findings: `rewrite/findings/by-package/S1.md`.

## 1. Scope

Built now: the command dispatch every unit registers with (`-C`, `--json`, `--wait`, exit codes, the
outside-a-project rule); the `vsdd.state` facade; `detect` and the mount; the key-table gaps of the
configuration loader; `vsdd.project.path_class`; the artifact registry reader with every check a story,
its contract and its epic need; and the commands `init` (create, mount, report legacy, `--auto-allow`;
enough for the sample in local or pr mode), `config --json`, `validate`, `path` and `--version`.

Left for when the build widens (S1 unless another package is named):

- `new` with its skeleton templates and `next_id`, and `list`: nothing on the story path calls them
  (`path` and `config --json` stay: the step 5 method files call them).
- `init --hub`, `init --fresh`; command detection (per toolchain, trial runs, the JUnit report, the
  `{port}` check); the lists of unclassed tracked files and of files a trial run left behind.
- `config set`, `refs`, `coverage`; `P/schemas/config.schema.json`; in `validate` the lock check,
  register-limit warnings and existence of references to kinds whose row has no `id`.
- Filling unset keys from a toolchain file (S3, second part). `story_kind` and `kind_has_red_gate`
  are the survey unit's (survey.md).

## 2. Files

Owned, new:

- `P/py/vsdd/cli/__init__.py` (docstring only) and `dispatch.py` (3.1).
- `P/py/vsdd/state/__init__.py`: the facade of 3.2 only; no other unit creates or edits it.
- `P/py/vsdd/globs.py`: `glob_regex` (3.4).
- In `P/py/vsdd/state/`: `gitrun.py` (the tool's one git subprocess helper); `mount.py` (`detect`, the
  mount test, creating and mounting the branch, the exclusions); `detection.py` (the values init
  detects, and `render_config`); `artifacts.py` (3.5); `cmd_init.py`, `cmd_config.py`,
  `cmd_validate.py`, `cmd_path.py` (one command each).
- `T/unit/`: the seven test files of section 7; `fixtures/artifacts/` (a valid story, contract and
  epic; one invalid file per reason code); `_repo.py`, the project builder every unit's tests start
  from: `make_project(tmp_path, *, remote=False) -> Path` (a scratch repository with one commit on
  `develop`, then the real `init`, in process) and `set_keys(root, section, **values)` (rewrites keys
  of its `factory.toml`).

Shared files; in step 3 only S1 edits them:

- `P/py/cli.py` (B2's). S1 replaces `build_parser` and `main`. `main` imports `vsdd.cli.dispatch` in a
  `try` that turns any `BaseException` but `SystemExit` into `vsdd: <type>: <text>` and exit 2: an
  import error must not leave as exit 1, the launcher's negative answer. The `sys.path` line stays
  verbatim at module level, no `vsdd` import beside it (`T/hooks/test_sibling_import.py`).
- `P/py/vsdd/config.py` (B3's, after C2's change). S1 changes `_apply_hook_rules` and the
  `delivery.protected_paths` default, and adds `Config.hook_rules()`.
- `P/py/vsdd/project.py` (B3's). S1 adds `path_class` (3.4) and, handed over from the hook-runner plan,
  makes a `home` of `None` in `find_project_root` mean `os.environ.get("HOME")`.

Other units add their own modules to `vsdd/state/`. S1 creates neither `vsdd/gates/` nor
`vsdd/delivery/`, and adds nothing to `next/tools/check.py`, `T/unit/test_config.py` or `P/schemas/`.

## 3. Interfaces this unit provides

### 3.1 Registering a command (`vsdd.cli.dispatch`)

A unit adds a command by adding one file and editing none: a module named `cmd_<family>.py` or
`cmd_<family>_<anything>.py` in `vsdd/state/`, `vsdd/gates/`, `vsdd/delivery/` or `vsdd/cli/`, holding
`COMMANDS = (Command(...), ...)`. `<family>` is the command's first word with `-` written `_`.

```python
@dataclass(frozen=True)
class Command:
    words: tuple[str, ...]  # ("gate", "red"); words[0] is the family
    summary: str  # one line, for --help
    run: Callable[[Invocation, argparse.Namespace], Reply]
    configure: Callable[[argparse.ArgumentParser], None] | None = None  # its own arguments
    needs: str = "initialised"  # "none" | "initialised" | "mounted"
    raw: bool = False  # True: nothing is parsed; run reads args.tokens
    output: str | None = None  # a relayed command's schema name, e.g. "survey"
    failure: Mapping[str, object] = field(default_factory=dict)  # see "Relayed" below
```

Resolution: the leading tokens up to the first that starts with `-` are the candidate words. The
dispatcher lists, without importing, the `cmd_` modules of that family in the four packages (a missing
package is skipped), imports only those, and picks the `Command` whose `words` is the longest prefix of
the candidates. The rest goes to `argparse` (`prog` is `vsdd <words>`), to which the dispatcher has
added `--json` and `--wait <seconds>` (a non-negative integer); a command defines neither, and with no
`configure` takes no argument of its own. With `raw=True` (no `configure`; `decide`'s grammar) the
dispatcher takes `--json` and `--wait <n>` out of the tokens before the first lone `--` and passes the
rest, in order, as `args.tokens`.

`needs`: `none` runs anywhere (init). `initialised` requires a project root, else exit 2 naming the
condition and `/vsdd-factory:setup`. `mounted` also requires `detect == "ready"`, else exit 2 naming
`/vsdd-factory:status --fix`: for `decide` and any writer of tracked state that may simply refuse. A
command that must still run on a broken mount (`survey`, `checkpoint`, `lock release`, `env`) declares
`initialised`.

Relayed. A command a script relays sets `output`. With `--json`, each exit 2 after the command is
resolved prints `{**empty_of(output), **failure, "ok": false, "error": "<message>"}`:
`empty_of(name) -> dict` gives every required key of `P/schemas/<name>.schema.json` its empty value by
type (`0`, `false`, `[]`, `{}`, `""`; the prelude's `emptyOf`), and `failure` holds the fields an empty
value would make invalid (`status`, `next`). `--json` is read from the raw tokens, so a usage error has
this shape too. On exit 0 and 1 the command prints every field itself.

```python
@dataclass
class Invocation:
    words: tuple[str, ...]
    argv: tuple[str, ...]  # every token after the program name, as received
    start: Path  # absolute: the -C directory, else the working directory
    root: Path | None  # vsdd.project.find_project_root(start, home=$HOME)
    json: bool
    wait: int | None  # --wait; None when absent
    caller_key: str | None  # $VSDD_CALLER, set by the hook's rewrite
    stdin_tty: bool  # standard input is a terminal
    plugin_root: Path
    environ: Mapping[str, str]
    # properties: state_dir (root/".factory"); config (cached vsdd.config.load); condition (detect)
    def retry_command(self) -> str: ...  # "vsdd " + shlex.join(argv), for a pending answer
    def require_person(self, what: str) -> None:  # Refuse if caller_key is set or stdin_tty is false

@dataclass
class Reply:
    ok: bool = True
    data: dict = field(default_factory=dict)  # merged after "ok"; must not hold "ok"
    text: str | None = None  # printed without --json; default: the JSON, indented
    code: int | None = None  # 0 or 1 only; default 0 when ok, else 1
    @classmethod
    def of(cls, output: dict, code: int | None = None) -> "Reply": ...  # from an object holding "ok"
```

- Output: with `--json`, stdout is exactly one line, `{"ok": ..., **data}`.
- `raise Refuse(message)` is the usage or environment error and the only way to exit 2: stderr
  `vsdd <words>: <message>`, and with `--json` also `{"ok": false, "error": "<message>"}` on stdout. A
  `ConfigError` from `Invocation.config` and any other exception from `run`
  (`internal error: <type>: <text>`) are reported the same way: a crash never exits 1.
- `require_person` is the tool's one person-only check (decision 2). A command learns its caller from
  `vsdd.state.callers.caller(inv.caller_key, project=inv.root)` (records.md), never from `os.environ`.
- `vsdd -C <dir> <words...>`: `-C` is accepted only as the first token with its value as the second
  (the hook runner reads the directory from that text). `--version` prints `vsdd <version>`. No
  command: exit 2.
- Exit 2 also for: no module for the family or no matching `Command`; two `Command`s with the same
  `words` (both modules named); a `cmd_` module that fails to import or has no `COMMANDS` (named, with
  the exception; other families still run).
- `commands(packages=COMMAND_PACKAGES) -> dict[tuple[str, ...], Command]` imports every `cmd_` module and
  raises `RegistrationError` on a duplicate or malformed one. For `--help`, B6's lints and tests.
- `decode_text(value: str) -> str` decodes the prelude's `enc()` (percent-encoded UTF-8), for every
  free-text argument; `Refuse` on a malformed escape.
- `main(argv, *, cwd=None, environ=None, stdout=None, stderr=None, stdin_tty=None,
  packages=COMMAND_PACKAGES) -> int`.

### 3.2 The `vsdd.state` facade

A unit publishes a flat `vsdd.state.<name>` by listing it in a literal top-level `__all__ = [...]` (one
list of string literals) of its own module in `vsdd/state/`. Three rules for every unit:

- `__all__` holds the names of spec `topics/state-api.md` that the module implements and no other
  (`[]` for none). Any other name is imported from its submodule
  (`from vsdd.state.runs import run_begin`), so a helper named `check`, `head` or `git` cannot collide.
- **A module is never named after a name any module exports** (`flush` cannot live in `flush.py`):
  Python would bind the submodule over the function. `test_state_facade.py` fails on it.
- Code on the hook path imports the submodule itself (`from vsdd.state.locks import lock_seen`).

`import vsdd.state` imports no submodule. The first `vsdd.state.<name>` builds the name table once per
process, reading each sibling's `__all__` with a regular expression (no import, no `ast`; modules
starting `_` or `cmd_` are skipped), then imports only the owner and caches the function. No owner, or
two, raises `AttributeError` naming the modules. `mount.py` lists `detect`; `artifacts.py` and
`gitrun.py` list nothing.

### 3.3 Detection, mount and git

- `vsdd.state.detect(root: Path | str) -> str`: `"none"`, `"legacy"`, `"ready"`, `"partial"` or
  `"unmounted"`, by the tests of spec `topics/state-directory.md`. File reads only (a ref is its loose
  file or a line of `packed-refs`); never raises.
- `vsdd.state.mount`: `STATE_BRANCH = "factory-artifacts"`; `mounted(root) -> bool` (flush step 2);
  `ensure_excludes(root) -> None`; `mount_state(root) -> None` (worktree add of the existing local or
  `origin` branch; raises `GitError`).
- `vsdd.state.gitrun.git(cwd, *args, check=True, timeout=60, input=None, env=None) -> str` returns stdout
  without its trailing newline; raises `GitError` (`.returncode`, `.stderr`) on a non-zero exit when
  `check`, and on a timeout. Sets `LC_ALL=C` and `GIT_TERMINAL_PROMPT=0`.

### 3.4 Configuration, path classes and globs

- `vsdd.project.path_class(path, *, root=None, config=None) -> str | None`: for an absolute path, the
  first class in `[_product].precedence` of `classes.toml` whose `from_config` key has a glob matching
  the path relative to the root of the git worktree holding it (the nearest ancestor with `.git`).
  `None` when no glob matches, in no project, or under `.factory/`. `root` and `config` spare a caller
  that has them the search and the load (the hook passes both); when `path_class` loads the file
  itself, an invalid one raises `ConfigError`.
- `vsdd.globs.glob_regex(glob) -> re.Pattern`, cached: the tool's one glob semantics (`*` and `?` do
  not cross `/`; `**` does; Python 3.11 has no such matcher), for `path_class`, the registry, the hook
  runner's classes and the digests' file sets.
- `Config.hook_rules() -> dict[str, dict]`: the validated `[hooks.rules.*]` tables;
  `Config.hook_problems` as today. The keys under a rule are closed, as spec `config/hooks.md` lists
  them: `strength`, the keys five rules take of their own, and for `auto-allow` only `scope`. Whether
  a rule id exists is the hook runner's check.
- `Config.get("delivery", "protected_paths")` when the key is absent: `.github/**`, `.env*`, `CODEOWNERS`,
  then `release.changelog` and each `release.version_files[].path`.

### 3.5 Artifacts (`vsdd.state.artifacts`)

Paths are POSIX, relative to the project root, starting `.factory/`.

- `registry() -> Registry`: built once from `vsdd.policy.load("artifacts")`. `.kinds[name]` is a `Kind`
  (the row's cells, compiled); `.raw` is the parsed table (the `story-kind` rows). Raises
  `RegistryError` naming the row.
- `kind_of(rel) -> str | None`: the kind; `"_unknown"` under an artifact directory with no matching kind;
  `None` outside artifact directories (records, `factory.toml`, `local/`).
- `check_text(rel, text, *, config, references=None, old_size=None) -> list[Problem]`: the write-time
  check of one file. `references` is a project root: give it to also report `dangling_reference`.
- `check_paths(root, rels=None, *, config, references=True) -> list[Problem]`: `rels=None` is the whole
  tree. Flush (records.md) calls `check_paths(root, rels, config=cfg, references=False)` on its changed
  files and leaves out each file that has a `Problem` of severity `error`.
- `changed_paths(root) -> list[str]`: modified and untracked files of the state worktree.
- `load_artifact(root, ident_or_rel) -> Artifact` (`kind`, `id`, `rel`, `frontmatter`, `body`); raises
  `ArtifactError` when absent or unreadable.
- `path_for(ident) -> tuple[str, str] | None` (kind, rel).
- `Problem(rel, code, message, severity)`, severity `"error"` or `"warning"`. Codes: those of spec
  `rules/artifacts.md` except `artifact_frozen`, plus `criterion_contract`.

### 3.6 Commands

- `vsdd init [--auto-allow off|core|extended]`, needs `none`. Fields: `state`, `created`, `mounted`,
  `root`, `config` (path), `detected` (key to value), `undetected` (keys), `next`. Exit 0 when `state` is
  `ready` (a second run changes nothing). Exit 1 with `state` `legacy` (`next` holds the dry-run and
  import commands; nothing converted or mounted) or `partial` (`next` names
  `/vsdd-factory:status --fix`). Exit 2: not a git repository, a linked worktree, a non-empty unmounted
  `.factory/`, no git identity, a state branch holding neither `factory.toml` nor `STATE.md`.
- `vsdd config --json`, needs `initialised`. Fields: `config` (the merged file), `hook_problems`.
- `vsdd validate [--changed] [PATH...]`, needs `initialised`. Fields: `checked`, `problems`
  (`path`, `code`, `message`, `severity`), `warnings`. Exit 0 with no error, 1 otherwise.
- `vsdd path <id>`, needs `initialised`. Fields: `id`, `kind`, `path`, `exists`. Exit 1 when no kind's
  pattern matches or the file is absent. Without `--json` it prints the path alone.

## 4. Interfaces this unit needs

- B3: `vsdd.project`, `vsdd.config` (`load`, `Config`, `ConfigError`, `SPECS`), `vsdd.frontmatter`. B2:
  `P/py/launch.sh` passing exit 0 and 1 through silently, and `P/bin/vsdd`.
- C2: `vsdd.policy.load("artifacts")` as shipped (rows `contract`, `story`, `epic`; `[_unknown]`;
  `[[story-kind]]`), `load("classes")` for `[_product].precedence`, and `config._apply_caps`.
- B6: L-PY on the new files (`urllib` is banned, hence `decode_text`). The sample:
  `T/e2e/make_sample_repo.sh -n` (a copy with no `.factory/`).
- From the other step 3 units: nothing.

## 5. Design decisions

1. **A crash exits 2** (1 would read as the negative answer), and a relayed command still prints its
   whole object, or the clerk must invent fields. `failure` exists because an empty `status` or `next`
   fails its own schema.
2. **`require_person` needs no `VSDD_CALLER` and a terminal on standard input.** The hook marks only a
   single-segment `vsdd` call and `gate-guard` (H6) is not in the slice, so an agent's
   `true && vsdd decide …` carries no marker; but a tool call has no terminal. Exit 2.
3. **Init builds the first commit with plumbing** (`hash-object`, `mktree`, `commit-tree --no-gpg-sign`,
   `update-ref` with an empty old value), then `worktree add`: the person's checkout, index and hooks
   are untouched, and `worktree add --orphan` needs git 2.42. Before creating, init asks
   `git show-ref`, so `none` read from files is never trusted alone. Interrupted after the branch
   exists, the next run finds `unmounted` and mounts. `partial` and an unmounted legacy branch are
   refused: repair is `status --fix` (S5), conversion is import (S6). The first `notes.json` is
   `{"notes": []}`, which the note store (left for the widening in records.md) reads as no notes.
4. **Detection at slice depth** writes name, languages, branches (`develop` if it exists, else the
   current one; release `main`, else `master`, else current), `delivery.mode` and `state.push` from
   whether a remote exists, `delivery.ci` from CI files, paths from directories that exist, and
   `README.md` and `CLAUDE.md` in `paths.docs`. Commands stay empty, listed in `undetected`. Every key
   of every fixed section is written.
5. **`config.SPECS` is the one key list**, and no schema file ships. Where the two key lists disagree
   the loader's default is the strict one: `delivery.mode` `"pr"`, `delivery.ci` `"required"`,
   auto-allow `"off"`; init writes what is detected or set at setup.
6. **A fault under `[hooks.rules.<rule>]` costs that rule its settings only** (strict defaults, reported
   in `hook_problems`); a fault in `[hooks]` itself is a `ConfigError`. Neither relaxes anything.
7. **Reading the registry as shipped.** A `glob` is alternatives separated by `, ` outside parentheses;
   `<word>` is `*`; a trailing `.(a, b)` expands; an alternative with no `/` takes the first one's
   directory. A type is `str`, `int`, `bool`, `date`, `ref:<kind>` or `list[<type>]`, with `?` for
   optional, or a list of allowed values; `""` or a string holding a space is an untyped optional key;
   anything else is a `RegistryError`. Typed keys without `?` are required; null counts as absent. A
   string `frontmatter` cell means no frontmatter to check.
8. **Reference patterns**: the row's `id`, else a constant for `capability` (`CAP-\d{3}`), `subsystem`
   (`SS-\d{2}`) and `screen` (`SCR-\d{3}`); a `ref:` to a kind with neither is a `RegistryError`.
9. **Artifact territory** is the set of first directories under `.factory/` that the registry's globs
   name. Existence is checked for references to kinds with an `id` row (a contract, a story).
10. **Acceptance criteria**: `AC-N` unique; when `contracts` is not empty each `### AC-N (...)` names
    one contract from it. Required headings are `## <text>` lines outside code fences. Line limits are
    those of spec `topics/artifact-rules.md`; an over-long body line is `size_cap`.

## 6. Findings

- **AU-S2-03, AU-T2-13**: one key list, `config.SPECS` (decision 5). `test_key_table_parity`.
- **AU-T2-15**: computed `protected_paths` default. `test_protected_paths_default_follows_release`.
- **P3SEAM-06**: closed per-rule keys. `test_hook_rule_keys_closed_per_rule`.
- **SL1-47**: decision 6; key shapes in `config.py`, rule ids in H1. `test_hook_fault_scope`.
- **SL1-48** (the Python half): the `home` default. `test_find_root_stops_below_home_by_default`.
- **SL1-49** (closed by C2): the validated value is the cap. `test_raised_cap_applies_and_waived_skips`.
- **VSA-17**: detection always writes `CLAUDE.md` into `paths.docs`. `test_init_detection`.
- **SL1-27**: init refuses a plain `.factory/` holding a `factory.toml` (`partial`); moving the
  sample's file into the overlay is E1's. `test_init_partial_refuses`.
- **AU-S2-12**: one `validate` syntax. `test_validate_exit_codes`.
- **AU-P2-17**: `flush`, `stale`, `job` and `step` are not commands. `test_unknown_command_exits_2`.
- **AU-P2-03, AU-S2-09**: limits and defaults exist once, in `SPECS`
  (`test_prelude_limits_equal_prelude_keys`); the other duplicates are G2's, S4's, B6's and H's.
- Not at slice depth, with the package that takes each: **AU-S2-11**, **AU-P2-16** S1 (prose);
  **P3COV-04** S1, G5, G7; **P3COV-06** S3; **P3OPS-11** G2 (S1 adds the report keys); **P3REQ-03**
  G4a, S3; **P3REQ-12** S3, S5; **SL1-22** S1, W1.

## 7. Tests

`test_dispatch.py`: `test_registers_by_file_alone`; `test_longest_words_win`;
`test_duplicate_words_exit_2_naming_both_modules` and `test_broken_command_module_is_loud` (another
family still runs); `test_unknown_command_exits_2`; `test_C_names_the_project` (also from a story
worktree; `-C` later is refused); `test_needs_levels`; `test_json_is_one_object_with_ok`;
`test_crash_exits_2_never_1`; `test_invalid_config_exits_2_naming_key`;
`test_wait_and_json_on_every_command`; `test_decode_text`; `test_retry_command`;
`test_version_and_no_command`; `test_require_person` (a marker, or no terminal: exit 2);
`test_raw_command_gets_tokens` (`--json` goes before `--` and stays after it);
`test_relayed_failure_prints_whole_object` (a `Refuse`, an invalid file, a crash and a usage error each
print a schema-valid object); `test_shipped_commands_register_cleanly` (the real packages; each command
with `output` has a schema-valid failure object); `test_import_error_exits_2` (a copy of `py/` with a
broken `dispatch.py`, through the launcher).

`test_state_facade.py`: `test_flat_name_resolves_to_owner`, `test_no_owner_or_two_is_loud` (temporary
packages); `test_no_module_shadows_an_exported_name`, `test_flat_names_are_state_api_names` (the real
package); `test_import_loads_no_submodule` (a fresh interpreter: `import vsdd.state` leaves no
`vsdd.state.*` in `sys.modules`; a lookup adds only the owner; a second reads no file).

`test_project_paths.py`: `test_path_class_precedence`, `test_path_class_in_a_story_worktree`,
`test_path_class_none_outside_and_under_state`, `test_glob_semantics`,
`test_find_root_stops_below_home_by_default`.

`test_mount.py`: `test_detect_five_conditions` (an invalid file is still `ready`),
`test_detect_reads_files_only` (no `git` on `PATH`; packed refs), `test_git_raises_with_stderr`,
`test_ensure_excludes_is_idempotent`.

`test_init.py`, on `make_sample_repo.sh -n` copies: `test_init_creates_and_mounts` (the branch, the
mount and the exclusions exist; the person's HEAD, index and tree are unchanged);
`test_init_is_idempotent`; `test_init_mounts_existing_branch` (local and `origin`);
`test_init_resumes_after_interruption`; `test_init_auto_allow_default_off`;
`test_init_legacy_converts_nothing`; `test_init_partial_refuses`; `test_init_not_a_repository_exits_2`;
`test_init_detection`; `test_init_through_bin_vsdd`.

`test_config_keys.py`: the five tests named in section 6, and `test_config_command_prints_merged_data`.

`test_artifacts.py`: `test_shipped_registry_compiles_every_row`, `test_kind_of_territory`,
`test_valid_story_contract_epic_pass`, `test_each_reason_code`, `test_smaller_write_always_allowed`,
`test_raised_cap_applies_and_waived_skips`, `test_dangling_contract_only_with_references`,
`test_criterion_names_a_listed_contract`, `test_path_for_round_trip`, `test_path_command`,
`test_validate_exit_codes` (also `--changed`), `test_changed_paths`.

## 8. Order of work

Before the builder starts: layer 2a has reported (C2's tables and `_apply_caps`, B6's lint) and the
checks pass under `next/`. `records`, `survey`, `hook-runner-b` and `jobs-b` wait for this unit
(`ORDER.md`).

1. `dispatch.py`, `cli/__init__.py`, the `cli.py` change, `test_dispatch.py`.
2. `state/__init__.py`, `gitrun.py`, `mount.py`, their tests.
3. `globs.py`, the `project.py` and `config.py` changes, `cmd_config.py`, their two test files.
4. `artifacts.py`, `cmd_validate.py`, `cmd_path.py`, `test_artifacts.py`.
5. `detection.py`, `cmd_init.py`, `_repo.py`, `test_init.py`.
6. Run the checks again; record outcomes in `rewrite/findings/outcomes/`.
