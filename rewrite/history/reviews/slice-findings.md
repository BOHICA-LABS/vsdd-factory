# Design findings from the thin slice, layer 1

Builders and reviewers of the foundations under `next/` reported these on 2026-11-24. Each is a place where the
design was wrong, missing, ambiguous or contradicted itself for something that had to be built. The builder did
not edit the design: it chose a reading, built that, and reported it. The amend step applies each finding to the
design, so that the design says what the code now does or the code is brought into line.

Total: 55 findings, 12 major and 43 minor. Section references are to the six original documents.

## SL1-01 (major)

- Reported by: the build step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.4, .mcp.json playwright entry ('<the option that refuses file: addresses>')

The design assumes the pinned @playwright/mcp has a CLI option that refuses file: addresses to add to args. At the version I pinned (0.0.86, checked against its shipped README), the default already blocks navigation to file:// URLs and file access outside workspace roots; the documented flag --allow-unrestricted-file-access LOOSENS this. Adding 'the option that refuses file:' is therefore impossible; the risk is accidentally shipping the loosening flag.

What was built: Pinned @playwright/mcp@0.0.86 with no extra flag (defaults deny file://), and the mcp_configuration lint rejects --allow-unrestricted-file-access and requires exact version pins. Probe 33 (package B8) should confirm on the running server and record the result in rewrite/STATE.md for L-MCP.

## SL1-02 (major)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md section 2.9, rows 'No Python 3.11 or newer (uv exits 2)' and 'uv cannot run (cache not writable, unknown flag on an old uv)'

Both failures reach the launcher as uv exit code 2, but the design requires a different message for each. It does not say how the launcher tells them apart, and the builder's single message gives the wrong diagnosis for an unwritable cache. The design should name the discriminator. For example, on the rc=2 failure path only, run `uv python find --no-python-downloads '>=3.11'`. Alternatively, specify one combined message.

## SL1-03 (major)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md section 2.2 and packaging.md section 4 I3 ('the cli entry may exec') versus BLUEPRINT.md 4.1 and packaging.md 2.10 ('the launcher's message names the terminal command, uv python install 3.12')

A launcher that calls exec on uv cannot print a message after uv fails, so the cli entry, including `vsdd doctor`, cannot meet the message requirement. vsdd's own exit 2 (usage) also collides with uv's exit 2. The design should say that the cli entry does not exec, and how the two exit-2 cases are told apart.

## SL1-04 (major)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.3 (layout during the rewrite; the .gitignore entry) and the repository root .gitignore

The old root .gitignore ignores `.mcp.json` everywhere, so the staged plugin's .mcp.json is ignored during the rewrite. Packaging 2.3 also says the root .mcp.json holds live keys and stays untracked, yet the specified next/.gitignore, which becomes the root one at promotion, carries no `/.mcp.json`. The design should give the contents of next/.gitignore: `/.mcp.json`, a negation for the plugin's file, `.claude/worktrees/` and `/.worktrees/`.

## SL1-05 (major)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md section 2.12 (every event line has v, ts, type, severity, session_id) versus section 2.2 (append_event in the shell launcher)

The launcher writes runner.launch_failed in shell, before any Python runs, and cannot know session_id without parsing stdin. The design does not say which fields a launcher-written line carries, whether the launcher honours VSDD_EVENTS_DIR, or who writes runner.disabled for VSDD_FACTORY_HOOKS=off, since the launcher exits before Python starts. Without these, the launcher's lines and events.py's schema will disagree.

## SL1-06 (major)

- Reported by: the review step of shared core and schemas (B3, C1)
- Where in the design: rewrite/BLUEPRINT.md 4.10 and 9.1 (C1) vs rewrite/design/pipeline.md 2.10.8

Blueprint 4.10 amends Review data, Claim, Holdout data and Intake data as if they were schema files. pipeline.md gives them only as field tables (2.10.8) with no JSON. C1's row says only '<plugin>/schemas/*.json as amended in 4.10'. Neither document says whether step-data shapes are files under schemas/ or which package writes them. That ambiguity allowed C1 to leave them out and B4a to hand-write them in the prelude, against 4.14's single source.

## SL1-07 (major)

- Reported by: the review step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/hooks.md 2.4 and packaging.md (differential test) vs state.md 2.4

The reader is held to PyYAML, which implements YAML 1.1 (yes/no/on/off are booleans, 010 is octal, 12:30 is sexagesimal). The design never says which YAML semantics the subset must agree with, or that plain scalars YAML 1.1 would retype must be refused. The differential test only covers plugin-authored files (agents, skills, templates), never the agent-written artifacts that are the reader's real input. As designed, the test can pass while artifacts are misread.

## SL1-08 (major)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/pipeline.md 2.3 and 2.11.1 (enc), blueprint 4.9 (survey --request-uri)

The design says `encodeURIComponent` makes free text safe on a single-segment shell command. It does not: `'`, `(`, `)`, `*` and `!` stay raw, so a request containing an apostrophe, parentheses or an asterisk breaks or globs the clerk's command. Universal test 11 as specified (spaces, quotes, a semicolon) cannot detect this.

## SL1-09 (major)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/pipeline.md 2.7, 2.10.2; blueprint 4.10 (finding built from scenario scores)

The holdout finding's shape contradicts itself:
- 2.7 and blueprint 4.10 say 'no location', 2.10.2 says `location` is `{"path": ""}`, and S_FINDING requires both `location` and `owner`. No owner is assigned to a holdout finding.
- `spec_ref` is 'set to the contracts the scenario cites (the tool resolves them, because it may read the scenario)', yet the prelude's `holdoutFindings()`, which cannot read scenarios, is to be 'held to it by fixtures'. The two copies cannot agree on `spec_ref`.
- The thresholds live in configuration (`holdout.mean_min`, `must_pass_min`, `stddev_max`), but blueprint 4.12 forbids unknown limit keys and no other input to the prelude is defined for them.

## SL1-10 (major)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/packaging.md 2.6, layer 2 harness table

The table of platform behaviours the harness must copy omits several platform features:
- `log()` (used by the wave.js skeleton) and `budget` (used by step() in 2.11.1).
- The `pipeline(items, ...stages)` signature, with stages receiving (prev, item, index).
- That `workflow()` returns the child's value and throws on failure.
- That the platform counts agents, not workflow() calls.

It also states 'required within properties' where the platform rule is 'required ⊆ properties, required optional'. The builder implemented the omissions and the misreading.

## SL1-11 (major)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/packaging.md 2.6, universal tests 4, 5 and 7; pipeline.md 2.11.1 step()

A stub that answers 'the smallest object its schema allows', or 'the other way', cannot echo the step identity, and step() rejects any result whose step.name or subject differs from the prompt's. Tests 4 and 5 as specified therefore never reach a convergence loop, a spike or a child, so they cannot be the 'mechanical proof that convergence loops and spike limits terminate'. The test definitions must say that stubs echo the prompt's step identity and that the survey stub passes. P3BLD-19 covers only the survey half, for test 7.

## SL1-12 (major)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/pipeline.md 2.10.8 (Close) and 2.5 (the checkpoint output is the workflow's return value)

The checkpoint's output fields are listed without types, although the same object is returned as the workflow result, where `progress` is boolean and `decisions` are objects. The design should state that S_CLOSE is the workflow-result schema plus `ok`. Otherwise a builder chooses string types, as happened here, and the drive loop's `progress: false` test breaks.

## SL1-13 (minor)

- Reported by: the build step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md section 2.2, launch.sh skeleton line here="${0%/*}"

When launch.sh is invoked without a slash in $0 (e.g. 'sh launch.sh' run from inside py/), ${0%/*} yields 'launch.sh' with no directory, so sourcing find-root.sh fails. The real hook command always uses an absolute plugin path, but a bare-name invocation would break the launcher.

What was built: Implemented with a POSIX case guard: case "$0" in */*) here="${0%/*}" ;; *) here=. ;; esac. Proved with a bare-filename invocation of launch.sh beside its sources (gate exit 0).

## SL1-14 (minor)

- Reported by: the build step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md section 2.8, find-root walk termination

'Stop at the user's home directory or the filesystem root' is ambiguous about whether $HOME and / are themselves examined as candidate project roots; the stated rationale ('a stray file there can never turn every project into a factory project') only holds if they are not.

What was built: Implemented the walk exclusively: $HOME and / terminate the walk before their .factory is tested. Consequence: a project rooted directly at $HOME is never detected; matches the stated intent and decision D24.

## SL1-15 (minor)

- Reported by: the build step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: packaging.md section 2.3, canonical check command 'cd next && uv run python tools/check.py'

The first run syncs pytest/ruff/pyyaml from PyPI; in this sandbox (and per the spike, in assistant shells generally) PyPI is unreachable (TLS UnknownIssuer), so the canonical command fails before check.py starts even though check.py itself is standard-library only.

What was built: Verified 'uv run --project next --no-sync python tools/check.py' passes fully offline. Recommend documenting the --no-sync variant for disconnected environments and committing a generated uv.lock from a connected machine (B7/CI).

## SL1-16 (minor)

- Reported by: the build step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.4 omitted-fields list (privacyPolicyUrl) vs claude plugin validate

'claude plugin validate' emits PRIVACY_URL_MISSING (a warning, verdict still passes) because the plugin declares remote MCP servers but no privacy policy URL. The design deliberately omits the field, so the warning is inherent to the manifest as specified.

What was built: Left the manifest per the design and recorded the warning. A later package (D3 or a maintainer) can add privacyPolicyUrl once a human supplies the organisational URL; any MIT-project GitHub URL would also satisfy it.

## SL1-17 (minor)

- Reported by: the build step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/hooks.md 2.4 vs rewrite/design/state.md 2.4

hooks.md defines the reader to accept one level of nested mapping, while state.md's flat artifact frontmatter forbids nested maps and caps line length; the two constraints are stated against different artifacts and a builder could read either as the reader's contract.

What was built: Built the reader to the hooks.md superset (it is the owning design) and left flatness and the 200-character line cap to the artifacts registry (C2); both module docstrings state the subset relation.

## SL1-18 (minor)

- Reported by: the build step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/hooks.md 2.4 scalar enumeration vs rewrite/design/state.md 2.4 value list

state.md lists a date among flat frontmatter values but hooks.md's scalar list (string, integer, boolean, null) omits it.

What was built: The reader parses bare YYYY-MM-DD scalars to datetime.date (YAML-consistent, stdlib), documented in the module docstring; the differential PyYAML test compares parsed objects so dates agree.

## SL1-19 (minor)

- Reported by: the build step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/pipeline.md 2.10 preamble

It says scripts hold the schemas 'composed without $ref, because the platform only promises a root object with properties', while the single-source schema files themselves use $ref between files.

What was built: Kept $ref in the shipped files (blueprint 4.14: the files are the single source); vsdd.schema resolves refs through a file-name registry, and the B4a build inlines them when it generates the prelude literals.

## SL1-20 (minor)

- Reported by: the build step of shared core and schemas (B3, C1)
- Where in the design: next/tests/hooks/test_sibling_import.py (unit 1 file)

The probe asserted py/ carries no vsdd package ('that arrives with B3') and unconditionally mkdir'd one, so B3's arrival raised FileExistsError and broke the hooks lint tier.

What was built: Updated build_probe_case to import the real shipped py/vsdd package when present and keep the minimal stand-in only for the pre-B3 case; the probe now proves the real package imports under PYTHONSAFEPATH.

## SL1-21 (minor)

- Reported by: the build step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/hooks.md 2.12 hygiene rules

'Commands are cut to 200 characters and passed through a redactor' is ambiguous about order; cutting first would leave secret material in the 200-char prefix.

What was built: Implemented redact-then-cut in events.py (no secret material survives, then the budget applies) and pinned the order in a test.

## SL1-22 (minor)

- Reported by: the build step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/pipeline.md 2.11.1 (step() budget check) and blueprint 4.12

The skeleton's step() consults an undeclared `budget.total` / `budget.remaining()`; no `budget` object, survey key, or blueprint-4.12 limit key defines a token budget

What was built: Modelled it as RUN.tokens_left + TOKEN_FLOOR, filled from an optional survey `config.tokens_left`; the unknown-limit-key test still enforces the key table. The design should add tokens_left to 4.12 or drop the token-budget clause

## SL1-23 (minor)

- Reported by: the build step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/packaging.md 2.6, universal tests 3 and 6

Test 3 (every agentType exists in <plugin>/agents/) and test 6 (the method file exists and the agent owns it) cannot pass before A1 builds the agent and method files, but B5 ships the tests

What was built: Test 3 checks each observed agentType against the generated ROLE table (roster), switching to file existence once agents/ is populated; test 6 asserts the Method-prefix always and defers file ownership. Should be stated in packaging.md 2.6 or B5's dependency row

## SL1-24 (minor)

- Reported by: the build step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/pipeline.md 2.5, parseArgs grammar

Grammar ^([a-z_]+)= cannot parse decision keys containing hyphens (e.g. the blocked choice `open-spike name=...`) that section 2.6 of the same design documents

What was built: Implemented ^([a-z_-]+)= — a superset: every documented input parses identically and hyphenated keys resolve. Amend the design or restrict the key table

## SL1-25 (minor)

- Reported by: the build step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/packaging.md 2.5, marker guard (no file outside <plugin>/workflows/ contains a marker)

Taken literally, the guard fires on tools/build.py and tools/check.py themselves, which must quote the marker strings to define and check them

What was built: Guard scans plugin, workflow-lib and tools, skipping the .py tool sources; noted as an interpretation for the amend step

## SL1-26 (minor)

- Reported by: the build step of the sample project
- Where in the design: rewrite/design/packaging.md 2.6, Layer 4 (sample project test command)

The command is fixed literally as `uv run python tools/junit_runner.py`. `uv run` discovers ancestor pyprojects, so run in place (or inside any monorepo) it picks up next/pyproject.toml and tries to sync its dev dependencies from PyPI - observed failing with a TLS/network error (exit 2).

What was built: Used `uv run --no-project python ...` for all three commands in factory.toml and README.md. Identical behaviour in the fresh copy (the e2e deployment), immune to ancestor discovery, and no network ever needed; proven both in place and in the fresh repo.

## SL1-27 (minor)

- Reported by: the build step of the sample project
- Where in the design: rewrite/design/state.md 2.12 and 2.15 vs packaging.md 2.6 and the unit task

state.md says factory.toml 'exists if and only if the project is initialised' and is created by vsdd init, while the unit requires shipping it filled in inside the sample. Committed naively, a fresh copy would look initialised with a broken .factory mount.

What was built: Shipped .factory/factory.toml in the sample and added .factory/ and .worktrees/ to the sample's .gitignore (the two exclusion lines init writes), so the product commit stays clean; make_sample_repo.sh -n drops .factory entirely for uninitialised scenarios.

## SL1-28 (minor)

- Reported by: the build step of the sample project
- Where in the design: rewrite/design/packaging.md 2.6 (JUnit runner)

The XML envelope is unspecified: bare <testsuite> root (Ant-style) or <testsuites> wrapper (pytest-style) both parse differently.

What was built: Wrote the pytest-style <testsuites><testsuite .../> wrapper; per-test identity is classname.dotted.module.Class + name, recoverable as the stable id gates freeze. Later gate packages should parse the wrapper form.

## SL1-29 (minor)

- Reported by: the build step of the sample project
- Where in the design: rewrite/design/packaging.md 2.6 (sample project) - fresh-copy script and path classes

No design text names the fresh-copy script or says where project-local tool scripts fall in the seven path classes; tools/** would be classified by no glob.

What was built: Placed the script at next/tests/e2e/make_sample_repo.sh (POSIX sh, optional DEST target, -n/--no-factory, deterministic local git identity, asserts exactly one commit); in the sample's [paths], put tools/** under source.

## SL1-30 (minor)

- Reported by: the fix step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md 2.9 (table row 'uv cannot run') vs the proposed fix in U1-R3/U1-R4

The proposed discriminator (`uv python find --no-python-downloads '>=3.11'`) inherits the session's UV_CACHE_DIR, so when the cache itself is broken the probe also fails and a perfectly healthy interpreter is misreported as 'no Python' — the exact conflation the finding was about. Found by running the real uv against a read-only cache while implementing R3.

What was built: The probe in launch.sh's interpreter_ok runs with its own writable UV_CACHE_DIR (${XDG_CACHE_HOME:-$HOME/.cache}/vsdd-factory/probe-cache), so 'is a Python at the floor installed?' is answered independently of 'is uv's cache usable?'. Documented in the launcher comment; guarded by a real-uv TestReadOnlyCache test asserting the uv_unusable message (not py_floor) with a read-only cache.

## SL1-31 (minor)

- Reported by: the fix step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md 2.2 illustrative skeleton (the rc case) vs U1-R5's expectation table

The skeleton's `case 0) exit 0;; 10) exit 2;; *) fail ...` is mode-uniform, so a brief-mode launch failure with rc=10 exits 2 and rc=1 exits 0 with the down briefing — while a reader of R5 ('assert exit code and stderr for 0,1,2,10,13,137 in each of gate, brief and notify') might expect 2 for every failure in every mode. The two readings differ for brief/notify.

What was built: Kept the design's uniform mapping (0->0, 10->2, other codes through fail(), whose brief/notify arms absorb the failure with exit 0 and the down briefing) and encoded exactly that in the test matrix with a comment citing the skeleton; no launcher change was needed.

## SL1-32 (minor)

- Reported by: the fix step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: next/tools/check.py shipped_files_are_not_gitignored

The check can only answer 'would this file vanish on a fresh clone' where git itself runs; on a scratch copy outside a repository (how review-style negative tests tend to run) git check-ignore has nothing to consult.

What was built: The check returns early (passes) when `git rev-parse --show-toplevel` fails, with a comment; in a real checkout it enumerates every file under plugins/vsdd-factory/ and fails listing any ignored paths.

## SL1-33 (minor)

- Reported by: the fix step of units 2 and 3
- Where in the design: rewrite/BLUEPRINT.md 4.10 vs rewrite/design/pipeline.md 2.11.1 (judge skeleton)

The documents contradict each other on holdout findings: blueprint 4.10 exempts category 'holdout' from BOTH the location and grounding conditions; pipeline.md 2.11.1's judge() exempts only the location condition (its 'hard' still requires spec_ref || repro).

What was built: Followed blueprint 4.10 (authority per the task brief): the prelude's judge() keeps the exemption from both conditions, matching the pre-existing code and the walled-finding rationale; the test-10 fixture carries spec_ref so it is meaningful under either reading.

## SL1-34 (minor)

- Reported by: the fix step of units 2 and 3
- Where in the design: rewrite/design/packaging.md 2.5 (400-line prelude cap) vs required additions from U3-R16 and U3-R20 (S_CLOSE composition, S_WAVE_GATE)

The cap predates those required schemas; the prelude stood at 445 non-generated lines with every mandated composite present, and deleting design-mandated content was not an option.

What was built: Compacted composites and doc comments, and fixed the measuring tool: check.py's _strip_region left 5 phantom blank lines at the splice seams which the cap counted. After a fair seam-trim and compaction the prelude is at or under 400 real lines with all mandated content.

## SL1-35 (minor)

- Reported by: the fix step of units 2 and 3
- Where in the design: next/tools/check.py prelude_respects_platform_constraints annotation (U3-R28)

Annotated '-> bool' but returned None.

What was built: Corrected the annotation to -> None.

## SL1-36 (minor)

- Reported by: the fix step of units 2 and 3
- Where in the design: next/tests/workflows/universal.test.mjs relay-prompt assertions

relay() prefixes 'vsdd -C <dir>', so no prompt contains the contiguous substring 'vsdd checkpoint'; the pre-existing test 7 passed only because its survey stub returned an empty project_dir.

What was built: New assertions match 'checkpoint' rather than 'vsdd checkpoint', correct whichever project_dir a stub supplies.

## SL1-37 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.10 (bin/vsdd snippet) and 2.6 L-SH

The specified `root=$(CDPATH= cd -- ...)` fails shellcheck with SC1007, and L-SH requires shellcheck to pass, so the design's own snippet violates its own lint. `CDPATH='' cd` passes. L-SH also names only launch.sh and bin/vsdd; find-root.sh is shipped shell too and should be covered.

## SL1-38 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md sections 2.3 and 2.12 (B0 skeleton, B1 'empty component folders') and BLUEPRINT.md 9.1

Git cannot track empty directories, so the skeleton and component folders vanish on commit, and a layout check over them fails on any fresh checkout. The design should specify a placeholder such as .gitkeep, or make layout lint check only populated directories.

## SL1-39 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.3 (the check command `cd next && uv run python tools/check.py`) and STATE.md's note that PyPI fails in the sandbox

The first run syncs from PyPI. In the assistant sandbox, plain uv fails with UnknownIssuer, but `--system-certs` (UV_SYSTEM_CERTS=1) works: uv.lock was generated and the dependencies installed. The design should state the working form for assistant shells, so builders do not defer uv.lock or work around a check command that fails.

## SL1-40 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md section 2.8 step 3 and the inline guard in section 2.3

Both assume an absolute `gitdir:` path. git 2.48 and later can write relative worktree paths (worktree.useRelativePaths). Both also compare against $HOME as a raw string, which a trailing slash or a symlinked spelling defeats. The design should say that relative gitdirs are resolved against the worktree, and that HOME is normalised.

## SL1-41 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/hooks.md sections 2.2 and 2.9

Section 2.2 says hook.py exits 13 below the Python floor, but no row of the failure table maps launcher rc 13 to a message. The generic 'could not run (exit N)' row has no fix, although 2.2 says every message carries one. Also, the skeleton's fail() sends any mode other than gate or brief to exit 0, so an unknown or mistyped mode turns failures silent. The design should require unknown modes to fail closed.

## SL1-42 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.4 (network.hosts and its hosts table)

`*.githubusercontent.com` is justified only by uv downloading a Python interpreter. Blueprint 4.1 and the launcher's `--no-python-downloads` forbid that, so the entry has no stated reason. Separately, Playwright's browser-binary download hosts are not considered, should @playwright/mcp need to install a browser.

## SL1-43 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.11 (the rewrite-mode CLAUDE.md: 'work only under next/; everything else is the system being replaced, to be read and not edited')

rewrite/ is not the system being replaced. STATE.md says rewrite/STATE.md is updated at every step and that slice findings go to rewrite/design/slice-findings.md. The specified text forbids that to every agent that loads it, and should carve out rewrite/ for the orchestrating session.

## SL1-44 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.4 and BLUEPRINT.md 5.4 ('the bundled browser server's entry carries the option that refuses file: addresses, and lint checks it')

The builder's finding is confirmed against the @playwright/mcp 0.0.86 README. There is no option that refuses file: addresses: refusal is the default, and the only documented option, --allow-unrestricted-file-access, loosens it. The README also says this restriction is 'not a secure boundary', and the environment variable PLAYWRIGHT_MCP_ALLOW_UNRESTRICTED_FILE_ACCESS loosens it from the environment as well. L-MCP should be restated as: no loosening flag in args, no loosening variable in env. The wall rests on the hook check of 2.11.2.

## SL1-45 (minor)

- Reported by: the review step of skeleton, manifest, launcher and entries (B0, B1, B2)
- Where in the design: rewrite/design/packaging.md section 2.4 (privacyPolicyUrl omitted)

`claude plugin validate` raises PRIVACY_URL_MISSING because the plugin bundles remote MCP servers. The validator says a 'Privacy' link in the README also satisfies it, so no organisational URL is needed. A README link to a privacy statement in the repository would clear the directory-profile warning while keeping the field omitted.

## SL1-46 (minor)

- Reported by: the review step of shared core and schemas (B3, C1)
- Where in the design: rewrite/BLUEPRINT.md 4.10 (Gate output) vs rewrite/design/pipeline.md 2.10.6 and 2.10.8

4.10 says gate output is 'either the gate result, or {ok, state: "pending"}'. pipeline.md's gate-result schema requires gate, subject, passed, sha, evidence and failures, yet also lists state 'pending'. 2.10.8 adds a 'retry' field that is not among the gate-result properties. The design does not say whether this is one schema with a union, which the stdlib checker cannot express, or two schemas.

## SL1-47 (minor)

- Reported by: the review step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/hooks.md 2.6 vs hooks.md 2.9 and state.md 2.12

hooks.md 2.6 says an unknown key or a bad value under [hooks] makes only that rule fall back to its defaults. hooks.md 2.9 says an invalid configuration means built-in defaults overall, and state.md 2.12 says unknown keys and wrong types are errors. The designs do not say whether a [hooks] fault invalidates the whole file or only the rule, or which module (config.py or H1) owns the per-rule check.

## SL1-48 (minor)

- Reported by: the review step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/hooks.md 2.5 (Ctx.project) vs hooks.md 2.8

2.8 gives exact discovery rules for the shell: stop at $HOME and '/', follow a linked worktree's .git file to the main checkout, no symlink resolution. 2.5's per-path Python resolution says only 'walking up to the nearest state directory, falls back to the session's project'. Leaving the Python rules unstated invites the divergence in U2-R8. The Python resolver should be specified as the same algorithm.

## SL1-49 (minor)

- Reported by: the review step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/state.md 2.12 ([caps]) vs the build plan (B3, C2)

'Raise a cap to at most twice its default' needs the cap names and defaults, which live in artifacts.toml (C2). config.py (B3) validates [caps] and cannot know them, and no package is told to enforce the bound.

## SL1-50 (minor)

- Reported by: the review step of shared core and schemas (B3, C1)
- Where in the design: rewrite/design/hooks.md 2.12 event example vs its field lists

The example rule.block line carries 'story', but neither the required field list nor the type table's extra fields for rule.block name 'story'. A schema that the writer validates against must either list it or allow additional properties, and the design does not say which.

## SL1-51 (minor)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/pipeline.md 2.11.1 gated() and step()

gated() renames the second attempt to `<name>-retryN`, and step() looks up OVERRIDE by `${wf}/${o.stage || name}`. In the design's own skeleton, a retried `ingest/map` or `ingest/analyze` therefore runs on the planner tier, which contradicts D5 and blueprint 4.2.

## SL1-52 (minor)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/packaging.md 2.5, the --check list ('no file outside <plugin>/workflows/ contains a marker')

The rule cannot hold as written. The harness toys under tests/workflows/toys must be stamped for the universal tests to run anything before W1, and tools/build.py and tools/check.py must contain the marker strings to stamp and check them. The rule should name the exact allowed files (this extends the builder's design finding 4). The layout rule also shows trailing text on the opening marker line while requiring the region to equal prelude.js byte for byte, without saying whether that text belongs to the region.

## SL1-53 (minor)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/agents-skills.md 2.7 versus packaging.md 2.5 and L-AGENT

Reviewer files must carry 'the finding key rule' byte for byte, but no single source file for it is named; severity.md holds only the table. packaging.md 2.5 says spike-runner and clerk take 'the variants the source file defines', while agents-skills.md 2.7 says spike-runner omits one bullet and the clerk carries a two-line block of its own. Neither document defines how one source file holds three variants.

## SL1-54 (minor)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/BLUEPRINT.md 9.1, the B5 row; packaging.md 2.6, universal tests 3 and 6

The builder's design finding 2 is confirmed. Tests 3 (every agentType exists in <plugin>/agents/) and 6 (the method file exists and the agent owns it) cannot pass until A1, yet B5 depends only on B4a and no interim rule is stated. The interim check should be exact roster membership read from roster.toml; the builder's interim regex accepts non-agents (U3-R14).

## SL1-55 (minor)

- Reported by: the review step of workflow prelude, build step and harness (B4a, B5)
- Where in the design: rewrite/design/pipeline.md 2.11.1 safe()

`safe()` is the design's guard for identifiers on command lines, but its regex admits a leading '-'. A typed identifier can therefore become an option of the vsdd command.
