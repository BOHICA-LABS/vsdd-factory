# vsdd-factory 1.0 design: project state, artifacts and resume

> Amended by `rewrite/BLUEPRINT.md` on 2026-11-23, and again on 2026-11-24 after the second review pass
> (blueprint section 11). Where this document and the blueprint differ, the blueprint wins.

## Amendments from the blueprint

The text below has been edited in place where a table, a path or a rule changed. This list is the complete set
of changes; where a passage further down still reads differently, this list and section 4 of the blueprint apply.
Items 1 to 21 are the first pass. Where the second pass replaced what an item says, the item is marked and the
second-pass list below it (items 22 to 40) holds.

1. **This design owns the whole pipeline-facing tool.** Survey, checkpoint, gates, jobs, verdicts, delivery and the
   registers live in `<plugin>/py/vsdd/state/`, `gates/` and `delivery/`. Section 2.16 lists every command. The
   pipeline design supplies each gate's pass condition. `vsdd record-test` does not exist; `vsdd gate` is the only
   writer of gate evidence (SEAM-01, COV-02).
2. **`factory.toml` is not writable by any tool call.** `vsdd init` creates it and a person edits it;
   `vsdd config set` is a person-only command. The one exception is `vsdd repos split`, which appends `[[repo]]`
   entries after a person's `spec` decision. A fingerprint of every gate-bearing section (`[commands]`, `[paths]`,
   `[branches]`, `[delivery]`, `[convergence]`, `[holdout]`, `[mutation]`, `[limits]`, `[hooks]`, and since the
   second pass `[[component]]`, `[stops]`, `[release]`, `[spikes]` and `[caps]`) is kept, and a change is
   announced in the next briefing. The second pass added two more sanctioned code writes: `vsdd import`
   creates the file, and the survey fills unset `[commands]` and `[paths]` keys from an approved toolchain file
   (SEAM-08, OPS-14, P2OPS-11, P2COLD-03).
3. **Reference checks.** At a write, a reference is checked for syntax only. Existence is checked by `vsdd gate
   spec`, `vsdd gate plan`, `vsdd validate` and at approval (OPS-04).
4. **Evidence lookup is per gate.** `red`: the latest passing record on any commit between the story's base and
   head, carrying the frozen test identities. `green` and `demo`: the head commit, or a commit with the same
   `git patch-id`. A converged review is an evidence record of its own, `review.<sha12>.json`, computed from the
   stored passes of any run for the unit (SEAM-05, OPS-06, OPS-13).
5. **Digest and settling.** `digest()` leaves out the frontmatter `status` line. `settle_decision` writes the
   decision record first and promotes statuses afterwards; the promotion is idempotent and the next survey
   finishes one that was interrupted (OPS-02).
6. **Layout.** `status.json` moves to `.factory/local/status.json` and is rebuilt on demand; notes move to the
   tracked `.factory/notes.json`. New: `requests/CR-<NNN>.md` (queued change requests, written by code),
   `research/inputs/<source>/<date>.*` (written by `vsdd discover collect`), `cycles/_standing/` (units and
   decisions of work outside a product cycle), `local/jobs/`, `analysis/<scope>/security.md`,
   `specs/architecture/dependencies.toml`. Dropped: `local/cache/research/` and `vsdd cache` (SEAM-21, SEAM-29,
   COV-09, OPS-29).
7. **Reviews.** Pipeline reviewers write no prose report; their findings are the step record, and the closure
   record keeps the dismissed and waived findings in full. `vsdd review show <unit>` renders findings for people.
   `reviews/` holds on-demand reports only. Holdout evaluations are JSON written by `vsdd checkpoint` (OPS-28).
8. **Story kinds.** `feature`, `fix`, `port`, `facade`, `docs`, `chore`, `harness`, plus an optional `method`
   key (SEAM-18, SEAM-28).
9. **Spike ledger.** Verdicts are `confirmed`, `refuted`, `answered`, `inconclusive`, `refused`. An entry is keyed
   by the runner's agent id as well as its worktree and records the asker's agent type. `spike_entry` takes the
   agent id. `spikes.max_per_session` and `max_per_requester` are dropped (SEAM-20, SEAM-36, COV-11, COV-17).
10. **Deletion commands.** `vsdd defer resolve`, `vsdd lesson retire` and `vsdd research retire` delete in the
    classes that allow it. Register caps are reported as status problems and never fail a gate or `vsdd validate`
    in a gate (SEAM-30, OPS-28).
11. **Flush points.** Checkpoint, decide, deliver, status and the pre-compaction hook. Not the survey, and not on
    agent stop or session end (SEAM-17, OPS-28).
12. **The lock.** Taken with a file lock around read, check and write, and only after the survey's preconditions
    pass. Scope: the repository and the branch the run can write. The hook runner owns liveness: it marks the
    holder's session as seen at most once a minute on guarded calls and releases the lock at session end. `vsdd`
    learns the session from the marker the hook runner writes before a `vsdd` call. No cross-machine lease
    (SEAM-22, OPS-07).
13. **Configuration keys.** `[commands] test_report` (required), `[commands.env]`, `limits.agents_per_story`,
    `limits.parallel_gates`, `limits.gate_minutes`, `holdout.batch`, `delivery.dependency_changes`, `[release]`,
    `[[discover.source]]`; `limits.ci_waits` is dropped and `delivery.ci_wait_minutes` is the total wait. The
    survey returns limits under the prelude's names (SEAM-36, COV-03, COV-09, COV-10, OPS-13, OPS-18, OPS-20).
14. **Decisions.** Grammar `<id> <choice> [key=value ...] [-- note]`; a choice-to-status table per kind (section
    2.8); kinds add `review-blocked`. `vsdd checkpoint brief` takes no run and opens the cycle; the `feature`
    survey opens a cycle for a request; the `release` checkpoint closes it (SEAM-23, SEAM-37, OPS-09).
15. **Amendments.** `spec_locked(project, path)` is true when the file is approved and no live run holds an
    amendment. An amendment covers the whole specification set (SEAM-44).
16. **Survey output.** Adds `plugin_root`, and per unit `tree`, `diff_patch`, `test_output`, `dirty`, `open_pr`,
    `rounds_used`; `spikes_used` for the run. "Owned elsewhere" means the remote head is neither the local
    branch's head nor an ancestor of it and no local run record names the branch. `unit_tree(project, workflow,
    subject)` returns the tree a unit's findings are relative to (SEAM-31, SEAM-33, OPS-05, OPS-06).
17. **Exit codes and exemptions.** Outside an initialised project every command exits 2 except `init`, `import`,
    `doctor`, `obs`, `hooks selftest` and `--version`. `vsdd stale` has no `--update`. `vsdd status` takes
    `--fix`, `--json` and `--wave` (SEAM-26).
18. **Driver.** `vsdd driver on`, `pause` and `off` write `local/driver.json` with the session id (SEAM-24).
19. **`next`.** An object `{kind, command, args}` in the status record, as in workflow results (SEAM-40).
20. **Hook needs.** H-1 is withdrawn. H-2 to H-5 are met by the hook design (SEAM-17).
21. **Open questions.** Section 6 is superseded by section 2 of the blueprint (decisions D1 to D3, D22, D23), whose
    recommendations the user adopted on 2026-11-23.

Superseded in part by the second pass: item 2 (code writes `factory.toml` in three stated cases, and the
fingerprint covers more sections); item 4 (no stored verdict record); item 5 (no status promotion, so the
digest has no special case); item 6 (no `status.json`); item 10 (one `vsdd retire`; no lessons register); item
11 (the survey commits pending state; `decide` and plain `status` do not flush); item 12 (caller markers);
item 15 (the lock is by decision over the whole set, not by file status); item 17 (no `vsdd stale`); items 18
and 19 (the driver can be paused; `next` is in `vsdd status --json`).

Second pass (2026-11-24):

22. **Status is read from the deciding record.** An agent sets only the statuses it owns (`draft`, `retired`,
    `dropped`, `stale`, `proposed`, `withdrawn`). `approved`, `ready`, `done`, `proven`, `accepted` and `active`
    are computed from the decision or closure record and shown by `vsdd list`; code never rewrites a tracked
    file to say them. The registry column `code_values`, the hook check on it, the status-line exclusion in
    `digest()`, and the promotion with its repair path are gone (sections 2.4, 2.5, 2.8; P2LEAN-01, P2SEAM-19,
    P2COLD-20, P2OPS-05).
23. **No status record and no verdict record.** The briefing and `vsdd status` are one function computed from
    the records each time. A verdict is computed from step records, or read from the closure after the
    checkpoint. The five conditions of section 2.3 stay, and `partial` now means only that the mount is
    broken (sections 2.3, 2.7, 2.9, 2.10; P2LEAN-04).
24. **No stage registry, no input digests, no `vsdd stale`.** The done test is three parts; stages reopen
    explicitly (sections 2.6, 2.8; P2LEAN-06).
25. **Product path classes.** `[paths]` gains `manifests`, `repo_config` and `harness`. A product path belongs
    to the first class that matches in the order `tests`, `harness`, `ci`, `repo-config`, `manifests`, `docs`,
    `source` (sections 2.4, 2.12; P2SEAM-01, P2COLD-02, P2OPS-03).
26. **The toolchain reaches `factory.toml` by approval.** `specs/architecture/toolchain.toml` is approved with
    `spec`; the next survey writes its values into unset `[commands]` and `[paths]` keys. Missing commands are
    a state, not an error (sections 2.5, 2.12; P2COLD-03, P2REQ-03).
27. **Baselines.** An ingest or an import writes a baseline closure record that the gates compare against
    (section 2.8; P2COLD-12, P2REQ-02).
28. **Cycles and requests.** Every request is `requests/CR-<NNN>.md`; `cycle.json` names it and holds the close
    data; there is no `request.md` or `summary.json`. Cycle openers: `checkpoint brief`, the `discover` assess
    checkpoint, the `feature` survey, and `maintain` when it delivers. An amendment request belongs to the
    open cycle (sections 2.5, 2.8; P2LEAN-11, P2SEAM-05, P2COLD-08, P2REQ-16).
29. **Decisions.** Nine kinds; one `blocked` kind with a reason. Named choice forms. Identifiers qualified
    `<id>@<cycle>` when ambiguous. Settling in a session writes the record only. An approved `merge` decision
    records the dependency it approved (section 2.8; P2LEAN-08, P2SEAM-13, P2COLD-25, P2COLD-29).
30. **Walled findings** are stored in the holdout class; closures and packets keep walled key and severity
    (sections 2.4, 2.5; P2SEAM-02, P2COLD-07).
31. **The lessons register is removed.** A retrospective is a research report with proposals (section 2.5;
    P2LEAN-09).
32. **Commands.** Section 2.16 is rewritten: re-attaching long commands with no job commands, the spike command
    forms, `review prepare`, `retire`, `release`, `-C`, `--include-walled`, the removed commands
    (P2LEAN-05, P2LEAN-07, P2SEAM-04, P2SEAM-12, P2COLD-06, P2COLD-17).
33. **Caller identity.** `vsdd` learns the session, agent id and agent type of its caller from the caller
    marker the hook wrote for that call (section 2.11; P2SEAM-15, P2OPS-01, P2COLD-05, P2COV-08).
34. **`init` and `import`.** `init` converts nothing on pre-1.0 state. The importer writes the baseline, an
    approved `spec` decision marked as imported, queued requests and `factory.toml` (sections 2.3, 2.15;
    P2COLD-14, P2REQ-04).
35. **Story admission.** `vsdd gate plan --story` and `vsdd story new-fix` record a story written after the
    survey, create its worktree and return its unit (section 2.16; P2COLD-01, P2REQ-01).
36. **Remote story branches.** Adoption and "owned elsewhere" are defined by the tool's pull-request trailer
    (section 2.11; P2OPS-07, P2COLD-19).
37. **Multi-repository.** Per-repository wave records and integration worktrees; `[release]` and `[delivery]`
    per `[[repo]]`; `init --hub`; missing members cloned (section 2.14; P2REQ-05).
38. **Configuration.** The full key table, release files in the protected paths, `stops.plan_ceiling_agents`,
    `commands.docs_check`, the port placeholder in `commands.run`, `[hooks]` keys for D4 and D6 (section 2.12;
    P2COLD-22, P2REQ-15, P2OPS-10, P2COV-02, P2COLD-11).
39. **Flush points and warnings.** Survey start, checkpoint, `story deliver`, `status --fix`, pre-compaction.
    Register limits are warnings in `vsdd validate` everywhere (P2OPS-19).
40. **Stale passages corrected**: the scaffold files' home, the research cache, `vsdd worktree ensure`, the
    review path scheme (P2COV-10, P2COLD-32).

Area: project state, artifacts, and resume after compaction or from cold. Written 2026-11-23 on branch
`rewrite/v1`.

How to read the names in this document.

- `<plugin>/` is the plugin root. `.factory/` is the state directory at the root of a project.
- `vsdd` is the plugin's one command-line tool, as named in `packaging.md`. `pipeline.md` calls the same tool
  `factory`. This document writes `vsdd` throughout.
- The four sibling designs were being written at the same time. Where they had already named something
  (agent names, path classes, the `vsdd` command, the policy tables, the survey and checkpoint commands),
  this document uses their name. Section 4 answers their stated needs by their own identifiers, and section
  2.18 lists the places where this design asks a sibling to change an assumption.

## 1. Summary

In 1.0 a project's factory state is still one directory, `.factory/`, kept on its own git branch and
mounted as a worktree. What changes is who does the work. Code mounts it, validates every write before it
lands, commits it and pushes it. No agent keeps state. The lock lease and the state-manager agent are gone,
and the person's own checkout is never switched, pulled or committed to.

Every artifact kind has one path, an identifier scheme, a flat frontmatter schema, one writer role and a
size cap. All of that is data in one registry, checked by code. Each fact is stored once. Indexes, counts,
reverse links, traceability tables, wave schedules and version citations are computed on demand and never
stored.

Progress lives in records that only the `vsdd` command writes: the decisions a person made, and one closure
record per unit of work. The briefing and the status report are computed from those records each time they
are asked for; no copy is stored. Step results and gate evidence stay untracked under `.factory/local/` and
are pruned.

A briefing of at most 1,200 characters is printed at session start, on resume and after compaction. One
entry point, `/vsdd-factory:status`, reports, repairs and names the next command.

## 2. The design

### 2.1 Terms

| Term | Meaning |
|---|---|
| State directory | `.factory/` at the project root. There is exactly one per project. |
| State branch | The git branch that holds the tracked content of the state directory. Its name is `factory-artifacts`. It shares no history with the product's branches. |
| Local store | `.factory/local/`, the untracked part of the state directory. It holds what matters only on this machine: step records, gate evidence, the run lock, the hook runner's records, the event log, caches. |
| Artifact | One file in the tracked part of the state directory. Its path decides its kind. |
| Agent-written artifact | An artifact whose text an agent or a person writes with the Edit or Write tool: a contract, a story, a review report. Code validates it before the write lands. |
| Machine record | A JSON file that only the `vsdd` command or the hook runner writes. No tool call by an agent can write one. |
| Cycle | One pass of the pipeline over a body of work, from a brief or a change request to a release: the first build of a product, one feature, one bug fix. |
| Unit | The thing one workflow run works on: the specification set, the plan, a wave, a story, the release candidate. The term is the pipeline design's. |
| Closure record | The machine record that says how far a unit has got and what closed it. |
| Decision | A question only a person can answer, stored as a record plus a packet a person can read. The term is the pipeline design's. |
| Registry | `artifacts.toml`: one table per artifact kind giving its path pattern, identifier pattern, frontmatter keys, required headings, size cap and frozen statuses. |
| Path class | A named set of paths that the hook runner uses for read walls and write scopes. The names are the agents-skills design's. |
| Flush | Committing whatever is pending in the state directory to the state branch. Code does it. |
| Derived view | Anything computed from artifacts on request and never written back: a catalogue, a count, a traceability table. |

### 2.2 The rules the rest follows

1. One state directory, independent of every product branch and worktree.
2. Each fact is stored in one place. A link is stored on the artifact that depends on the other, and the
   reverse view is derived.
3. There are two kinds of file. Agents write prose artifacts, and code validates them before the write.
   Code writes records, and no agent can write them.
4. Every file has a size cap that is checked before the write. Nothing is compacted afterwards, because
   nothing is allowed to grow.
5. Nothing derived is stored, and nothing stored is updated after the fact. No commit id, count or version
   is ever patched to match something else.
6. Everything needed to resume is on disk at each step boundary. Tracked state is enough to resume on
   another machine; the local store only saves repeating work on this one.
7. The subsystem costs an agent nothing until it is used: one model-visible skill, no index to load, no
   state file an agent must read before working.

### 2.3 The state directory

#### Decision: keep the separate branch, and take every chore away from agents and people

The old design kept state on an orphan branch (`factory-artifacts`) mounted as a git worktree at
`.factory/`. I keep that storage and replace everything around it. Three alternatives were weighed.

| | A. Separate branch mounted as a worktree (chosen) | B. Tracked directory on the integration branch | C. Tracked directory on every work branch, so state travels with code |
|---|---|---|---|
| The person's checkout | Never switched, pulled or committed to | Must stay on the integration branch, and state commits land in it | Free, but every worktree holds its own copy of the state |
| One state directory | Yes, whatever branch any checkout is on | Yes, if story worktrees leave the directory out | No. The right copy depends on which unit the work belongs to |
| Product history and pull requests | No state in either | One state commit per sync on the integration branch | State files in every story pull request |
| Protected branches | Unaffected | The integration branch must accept direct pushes | Works |
| Hidden scenarios | Outside the product tree, and skipped by search tools because the directory is excluded | Inside the product tree and inside packages built from it | In every worktree |
| A fresh clone | One command mounts the state | Works at once | Works at once |
| Specification and code change in one commit | No | No | Yes |

B and C save the mount. In exchange B constrains the person's checkout for as long as the project lives,
and C gives every hook rule and every prompt the job of working out which copy is the right one. The
pipeline design refuses to touch the person's checkout ("nothing in 1.0 pulls or merges in the person's
checkout"), and the hooks and agents-skills designs both assume a single state directory. A is the only
option that satisfies all three without a standing constraint.

The justification against low touch is that the branch was never what cost the effort. The chores around it
were, and each one moves into code.

| What the old design cost | In 1.0 |
|---|---|
| Skills to create the branch, mount the worktree, check its health and repair it | `vsdd init` creates or mounts it. `vsdd status --fix` remounts it. No questions are asked. |
| A lock lease kept in `STATE.md` (two crates, four scripts, two skills, two hooks) | A local run lock taken by the survey command (2.11). Two machines are reconciled by git when state is pushed. |
| One agent allowed to commit, one commit per "burst", commit-message rules, 3,489 state commits made by agents | `flush`, one function. Nobody composes or reads state commit messages. |
| A compare-and-swap push script | `vsdd checkpoint` pushes. On rejection it fetches, rebases the state branch and retries, and only when no run is live. |
| Hooks guarding the branch, the path root and staging (old registry entries 39, 49, 70) | One precondition checked by file reads: `.factory/` is a worktree whose branch is the state branch. |
| `STATE.md` citing product commit ids, kept current by hand | No stored commit id is ever updated. Each state commit carries the integration head as a trailer. |
| A `.gitignore` edit in the product | Two lines in `.git/info/exclude`, written at mount. No product file changes. |
| CI mounting the branch for validators | Optional. A project that wants `vsdd validate` in CI adds two lines. |

What is left for a person: run `/vsdd-factory:setup` once per clone.

#### Mounting

- Location: `<project root>/.factory`. Branch: `factory-artifacts`. The name is kept because the eight
  existing projects, the hook design's precondition and every document already use it.
- `vsdd init` (run by the `setup` skill) does one of three things. With no state branch anywhere it creates
  the orphan branch with a first commit holding `factory.toml`, `notes.json` and `.gitignore`, and mounts
  it. With a state branch on the remote or locally it mounts that. With pre-1.0 state it converts nothing:
  it reports `legacy` and prints the dry-run command and then the `vsdd import` command, which only a person
  may run (2.15). There is no status record to build: the briefing is computed (2.7).
- It writes `.factory/` and `.worktrees/` into `.git/info/exclude`. Search tools that honour git exclusions
  then skip the state directory in a search from the project root, which answers `agents-skills.md`
  N-STATE-8 for every search tool that honours them. The hook design's read walls still cover the rest.
- The state branch's own `.gitignore` contains `local/`.
- Story and spike worktrees are worktrees of product branches, so they contain no `.factory/` at all. A
  `.factory` path inside a linked worktree is always a mistake, and the hook design's `write-scopes` check 5
  blocks it and prints the right path.

How code tells the conditions apart, with file reads only:

| Condition | Test |
|---|---|
| Not a factory project | Neither `.factory/factory.toml` nor `.factory/STATE.md` exists. Hooks are inert and silent. |
| Pre-1.0 state | `.factory/STATE.md` exists and `.factory/factory.toml` does not. Hooks are inert except for one briefing line: "Pre-1.0 factory state found. Run /vsdd-factory:setup to import it." |
| 1.0, ready | `.factory/factory.toml` parses with `schema = 1`, and `.factory/.git` is a file pointing at a worktree whose `HEAD` is `refs/heads/factory-artifacts`. |
| 1.0, partial | The configuration file is there but the mount test fails. All gates run, state writes are blocked, and the briefing names `/vsdd-factory:status --fix`, which remounts. |
| State exists but is not mounted | `.factory/` is absent and `refs/heads/factory-artifacts` or `refs/remotes/origin/factory-artifacts` exists. `vsdd doctor` and `setup` report it. Hooks are silent there (decision D24). |

The function is `vsdd.state.detect(root)` and returns `none`, `legacy`, `ready`, `partial` or `unmounted`.
These are the five conditions every design uses. Since the second pass `partial` has one cause, a broken
mount: the first pass also counted a missing status record, and with no status record there is nothing to
be missing or rebuilt.

#### Layout

```
.factory/                               tracked on the state branch, except local/
├── factory.toml                        project configuration (2.12); a person edits it, no tool call can
├── notes.json                          up to five notes a person asked to keep (written by vsdd note)
├── .gitignore                          one line: local/
├── specs/
│   ├── product/                        brief.md, prd.md, domain/<section>.md, supplements/<name>.md,
│   │                                   contracts/<S>.<SS>/BC-<S>.<SS>.<NNN>.md
│   ├── architecture/                   overview.md, SS-<NN>.md, modules.toml, dependencies.toml, conventions.md, review-rules.md,
│   │                                   decisions/ADR-<NNN>.md, properties/VP-<NNN>.md, assessments/<slug>.md
│   └── ux/                             overview.md, screens/SCR-<NNN>.md, flows/FLOW-<NNN>.md, design-system/**
├── stories/E-<N>/                      epic.md, S-<N>.<NN>.md
├── holdout/                            HS-<NNN>.md, evaluations/<cycle>/<unit>-<n>.json (written by code;
│                                       scores, symptoms, and the full form of walled findings)
├── reviews/<yyyy-mm-dd>-<lens>-<n>.md  on-demand review reports only; pipeline reviews write no file
├── demo/<cycle>/<story>/               AC-<k>.<ext>, AC-<k>.txt, AC-<k>.png, report.md
├── analysis/<scope>/<slug>.md          brownfield and port analysis; security.md, disposition.md, port.md
├── research/<yyyy-mm-dd>-<slug>.md     reports, retrospectives and idea briefs
├── research/inputs/<source>/<date>.*   collected feedback and analytics (written by vsdd discover collect)
├── requests/CR-<NNN>.md                every change request, queued or running (written by code)
├── ideas/ID-<NNN>.md                   the idea registry
├── spikes/SP-<NNN>.md                  the spike ledger
├── deferred/<slug>.md
├── cycles/<cycle>/
│   ├── cycle.json                      what the cycle is: kind, title, route, baseline, request; and, once
│   │                                   closed, the close date, the version and the state commit before pruning
│   ├── units/<workflow>[.<subject>].json     closure records
│   └── decisions/<id>.json, <id>.md    decision records and their packets
├── cycles/_standing/                   the same shape, for work outside a product cycle: ingest, discover,
│                                       maintain and spikes, and the ingest or import baseline. It never closes;
│                                       it keeps the latest closure per workflow and subject, the baseline, the
│                                       latest `spec` decision, and other settled decisions for 90 days
└── local/                              untracked
    ├── runs/<run-id>/run.json, steps/<workflow>/<subject>/<name>.<attempt>.json
    ├── evidence/<subject>/<gate>.<sha12>.json   (and the raw output, JUnit report or diff patch beside it)
    ├── jobs/<key>.json, <key>.log      detached gate and delivery jobs, keyed by command, subject and commit;
    │                                   products started by vsdd env start
    ├── lock.json, lock.flock           the run lock and the file lock that guards it
    ├── driver.json                     whether the driver skill is active or paused, and in which session
    ├── hooks/                          the hook runner's own records (caller markers, spike admissions, the
    │                                   configuration fingerprint)
    ├── events/                         the event log
    ├── media/, index.json
    └── flush.lock
```

Every top-level directory is one path class with one writer role (2.4), which is what lets the hook runner
check write scopes by path prefix.

#### Who commits state, and when

Nobody is asked to commit state. Flush does it. It is the function `vsdd.state.flush(root, reason)`. There is
no `vsdd flush` command: nothing needs one.

| Called from | When |
|---|---|
| `vsdd survey` | At its start, after preconditions pass, for anything pending. In practice this is a decision a person settled since the last checkpoint: settling writes the record and nothing else, so that it fits inside the hook's time limit. |
| `vsdd checkpoint` | After it writes closures and decisions. This is "saves state" in the pipeline design. It then pushes. |
| `vsdd story deliver`, `vsdd status --fix` | After their own writes. |
| The hook runner on `PreCompact` | Always (hook design rule `compaction`, its need S8). |

Flush runs at these five points only. `vsdd decide` and plain `vsdd status` do not flush, and `vsdd status`
writes nothing at all. Flushing on every command or on every agent stop would make hundreds of state commits
per wave. Nothing is lost in between: the files are on disk.

What flush does:

1. Takes `.factory/local/flush.lock`. A second flush waits up to 5 seconds and then leaves the work to the
   holder.
2. Checks the mount. If `.factory/` is not a worktree on the state branch it changes nothing and reports
   `partial`.
3. Validates each changed or new file against the registry (the same check as the write hook). A file that
   fails is left uncommitted and reported, so something written through the shell cannot slip in.
4. Runs `git -C .factory add` on the valid paths and `git -C .factory commit` with the message
   `factory: <kinds and identifiers changed>` and two trailers, `Run: <run id>` and `Code-Head: <head of
   the integration branch>`. The commit runs with an empty hooks path and without signing, because the
   product's git hooks are written for product code and would run in a tree that has none.
5. Never touches a product branch, a product file or the person's index.

It is idempotent, makes no network call, and takes well under 2 seconds on the 2,000-contract corpus of the
old engine. The hook design gives it 20.

Push. `vsdd checkpoint` pushes the state branch when `state.push = true` (the default when the repository
has a remote). If the remote has moved, which only happens when another machine pushed state, it rebases
the state branch onto the remote and retries up to three times, and only while it holds the run lock, so
no agent is writing. A conflict stops the push, changes nothing, and is reported as a problem in
`vsdd status`. Nothing else pushes state, so nothing replaces `factory-cas-push.sh` as a separate tool
(`packaging.md` S6).

### 2.4 Rules every artifact follows

**One registry.** `<plugin>/py/vsdd/policy/artifacts.toml`, in the format of `hooks.md` section 2.11.3, is
the single copy. The hook rule `artifacts` reads it before a write, `vsdd validate` reads it for a whole
tree, and `vsdd new` reads it to stamp a skeleton. This design owns its content, which is the tables in
section 2.5. It needs one column beyond the hook design's list: `template` (the skeleton file under
`<plugin>/templates/`). The first pass also had `code_values`, for statuses only `vsdd` could set; the second
pass removed those statuses from the files (below), so the column is gone. The story kinds, with whether
each has a red gate, are a table in this file too.

```toml
[contract]
glob        = ".factory/specs/product/contracts/*/BC-*.md"
id          = '^BC-(\d+)\.(\d{2})\.(\d{3})$'       # equals the file stem and the frontmatter id
dir         = "{1}.{2}"                             # the parent directory is derived from the identifier
frontmatter = { id = "str", title = "str", status = ["draft", "retired"],
                subsystem = "ref:subsystem", capability = "ref:capability",
                origin = ["new", "recovered"], replaced_by = "ref:contract?", aliases = "list[str]?" }
lock        = "spec"                                # frozen while the specification set is locked (below)
# A `ref:` value is checked for syntax when the file is written. Whether the referenced artifact exists is
# checked by `vsdd gate spec`, `vsdd gate plan`, `vsdd validate` and at approval, so a contract may name a
# subsystem that a later stage writes, and parallel story writers may reference each other.
headings    = ["Behavior", "Preconditions", "Postconditions", "Invariants", "Edge cases", "Test vectors"]
max_bytes   = 8000
template    = "contract.md"
```

**Flat frontmatter.** Every agent-written artifact starts with one `---` block of `key: value` lines. A
value is a string, an integer, `true` or `false`, a date, or a flow list such as `[BC-2.03.007, BC-2.03.008]`.
There are no nested maps and no multi-line values, and a line is at most 200 characters. This is inside
the subset the hook design's reader accepts. Only the keys the registry lists for the kind are allowed.
The old `STATE.md` had a frontmatter line of 10,214 characters; a line cap and a closed key list make
that impossible.

Frontmatter carries an identifier, a title, a status and the links to what the artifact depends on. It
carries nothing else. The old standard block had twelve keys and a lifecycle block of eight more; the ones
dropped are in section 3.

**Identifiers.** The path of an identified artifact is a function of its identifier. There are no slugs in
file names, so there is no rule about slugs staying fixed when titles change, and a hook can find a
referenced artifact without scanning a directory.

| Artifact | Identifier | Example | Allocation |
|---|---|---|---|
| Capability, entity, invariant, domain event, domain edge case, assumption, risk, failure mode | `CAP-NNN`, `ENT-NNN`, `DI-NNN`, `EVT-NNN`, `DEC-NNN`, `ASM-NNN`, `R-NNN`, `FM-NNN` | `CAP-012` | Sequential; headings inside a domain file |
| Non-functional requirement, error code | `NFR-NNN`, `E-<AREA>-NNN` | `E-NET-001` | Sequential; headings inside a supplement |
| Subsystem | `SS-NN` | `SS-03` | Sequential |
| Behavioral contract | `BC-S.SS.NNN` | `BC-2.03.007` | Sequential within the group `S.SS` |
| Verification property | `VP-NNN` | `VP-031` | Sequential |
| Decision record | `ADR-NNN` | `ADR-012` | Sequential |
| Screen, flow | `SCR-NNN`, `FLOW-NNN` | `SCR-004` | Sequential |
| Epic, story | `E-N`, `S-N.NN` where `N` is the epic | `E-7`, `S-7.03` | Sequential. `E-0` is the standing epic for fixes and chores |
| Acceptance criterion, edge case | `AC-N`, `EC-N` | `AC-2` | Local to the story or the contract |
| Holdout scenario | `HS-NNN` | `HS-014` | Sequential |
| Idea, spike | `ID-NNN`, `SP-NNN` | `SP-007` | Sequential, by `vsdd` |
| Wave | `W-NN` | `W-02` | Per cycle, by `vsdd plan schedule` |
| Cycle | a slug of 2 to 40 lower-case characters | `auth-sso` | Chosen when the cycle opens |
| On-demand review report; evaluation | the path of section 2.5: `reviews/<yyyy-mm-dd>-<lens>-<n>.md`; `holdout/evaluations/<cycle>/<unit>-<n>.json` | `reviews/2026-11-26-security-2.md` | `vsdd review prepare <lens>` returns the next free report path; the checkpoint numbers evaluations. A pipeline review has no file and no identifier: it is a step record |
| Finding | the key from the pipeline design's finding schema | | The reviewer |
| Research, analysis, deferred item, assessment | the file's slug | `research/2026-11-23-uv-startup` | The writer |

Identifiers are never renumbered and never reused. A retired artifact keeps its file with
`status: retired`, which is what keeps the number occupied. Because there is one state directory, an agent
that picks a number already taken is blocked at the write and told the next free one. `vsdd new <kind>`
allocates for callers that have a shell.

**Links point one way.** A contract names its capability and subsystem. A property names its contracts. A
story names its contracts, properties and the stories it depends on. An acceptance criterion names one
contract in its heading, `### AC-2 (BC-2.03.007)`. A scenario names its contracts. A decision record names
the one it supersedes. Nothing stores the reverse. "Which stories implement this contract" is a query
(`vsdd refs BC-2.03.007`). This removes the old propagation policies, the five-place contract retirement
checklist and the agents that enforced them.

**Size caps.** Caps are in bytes and are enforced before the write (hook rule `artifacts`, reason code
`size_cap`). A write that makes a file smaller is always allowed. Body lines are at most 1,000 characters.
A project may raise a cap in `[caps]` to at most twice the default, and every raised cap is listed in
`vsdd status`. The block message says what to do instead, for example: "S-7.03 would be 17,412 bytes; the
cap is 16,000. Split the story."

**Who may write.** Two mechanisms, both enforced by the hook runner from data and records.

| Mechanism | Data | Effect |
|---|---|---|
| Path class | `classes.toml` (below) and the agents-skills design's write-scope table | Each class has one writer role. A class with `write = "code"` cannot be written by any tool call. |
| The lock | The decision and closure records, through `spec_locked` and `artifact_locked` | The specification set is locked as a whole: while the latest `spec` decision is approved and no live run holds an amendment, no file in the classes `spec-product`, `spec-arch` and `spec-ux` can be written, whether or not the file carries a status. This is what locks the domain files, the architecture overview, `modules.toml`, `dependencies.toml`, the toolchain file, conventions and the UX files, which never had an `approved` status to test. Two files, the class `guidance`, are treated a little differently, because they are guidance and not a statement of what the product does: `conventions.md` and `review-rules.md` are locked with the rest while a cycle is open, so nobody changes the rules in the middle of a build, and are writable by the architect between cycles (`guidance_locked`), which is how a retrospective's proposal is adopted through `/vsdd-factory:author` without an amendment run. They are outside the `spec` decision's digest (2.8), so that edit does not make the approval stale. The `brief` class is outside the lock and outside that digest; it has its own decision. Three kinds stay frozen even under an amendment, because a change to them is a new artifact that supersedes the old one: a decision record that an approved `spec` decision lists as accepted; a property that a `verify` closure lists as proven; a story whose delivery closure exists. |

**Status values an agent may write, and the ones that are computed.** An agent sets only the statuses its
kind lists in section 2.5: `draft`, `retired`, `dropped`, `stale`, `proposed`, `rejected`, `withdrawn`, and
`active` for the living kinds that have no approval step of their own (subsystem, screen, flow, scenario).
The values that mean "a gate or a person approved this" are not written into files at all, so there is one
list and it is per kind. They are read from the record that decides them:

| Computed status | Read from |
|---|---|
| A specification file is `approved` | The latest `spec` decision is approved and its digest covers the file as it stands |
| A decision record is `accepted` | An approved `spec` decision lists its id (the decision record keeps that list) |
| A property is `active`, then `proven` | It is in an approved specification set; a `verify` closure lists it as proven |
| A story is `ready`, then `done` | The plan's closure places it in a wave, or a run's closure records its admission; its delivery closure exists |

`vsdd list <kind>` and `vsdd status` show the computed status, so a person asks the tool instead of opening
the frontmatter. This removes six code write paths into tracked files, the commit after each promotion, the
recovery path for an interrupted promotion, and the case where an amended file kept an `approved` line over
text nobody had approved: an amended file is simply not covered by the old digest.

**Path classes.** This is the content of `classes.toml` (`hooks.md` S6, `agents-skills.md` N-STATE-1).

| Class | Globs, relative to the project root | Write | Writer |
|---|---|---|---|
| `config` | `.factory/factory.toml` | none (no tool call) | A person, in an editor or with `vsdd config set` in their own terminal. Code writes it in three stated cases, each announced in the next briefing: `vsdd init` and `vsdd import` create the file (including the `[hooks]` answers `setup` collected and, for a hub, the `[[repo]]` entries); `vsdd repos split` appends `[[repo]]` entries; and the survey fills `[commands]` and `[paths]` keys that are still unset from the toolchain file an approved `spec` decision covers. |
| `control` | `.factory/notes.json`, `.factory/cycles/**` | code | `vsdd` |
| `decisions` | `.factory/cycles/*/decisions/**` | code | `vsdd checkpoint` opens; `settle_decision` settles |
| `brief` | `.factory/specs/product/brief.md` | tools | `product-owner`; the main session through the `brief` skill |
| `spec-product` | `.factory/specs/product/**` | tools | `product-owner` |
| `spec-arch` | `.factory/specs/architecture/**` | tools | `architect` |
| `guidance` | `.factory/specs/architecture/conventions.md`, `.factory/specs/architecture/review-rules.md` | tools | `architect`. The two guidance files, named apart inside `spec-arch` because three rules treat them apart: their lock (above), the decision digest (2.8), and the `code-reviewer`'s read wall, which denies `spec-arch` and allows these |
| `spec-ux` | `.factory/specs/ux/**` | tools | `ux-designer` |
| `stories` | `.factory/stories/**` | tools | `story-writer` |
| `holdout` | `.factory/holdout/**` | tools | `product-owner` (scenarios) |
| `holdout-evaluations` | `.factory/holdout/evaluations/**` | code | `vsdd checkpoint`, from the evaluator's step records; and the full form of walled findings (a finding from the `scenario-reviewer`, or one located under `.factory/holdout/`). No other tracked class, no decision packet and no closure record holds it; the only other copy is the step record in `runs`, which no caller can read |
| `reviews` | `.factory/reviews/**` | tools, create only | `adversary`, `code-reviewer`, `security-reviewer`, `ui-reviewer`, for on-demand reviews only |
| `demo` | `.factory/demo/**` | tools | `demo-recorder` |
| `analysis` | `.factory/analysis/**` | tools | `codebase-analyzer`; `security-reviewer` may create `security.md` |
| `research` | `.factory/research/**` except `inputs/` | tools | `researcher` |
| `research-inputs` | `.factory/research/inputs/**` | code | `vsdd discover collect` |
| `requests` | `.factory/requests/**` | code | `vsdd survey feature`; `settle_decision` for an amendment request; `vsdd import`; `vsdd story new-fix --maintain` for a maintenance cycle |
| `deferred` | `.factory/deferred/**` | tools | The main session through the `defer` skill |
| `ideas`, `spikes` | `.factory/ideas/**`, `.factory/spikes/**` | code | `vsdd idea register`, `vsdd spike open` and `record` |
| `runs` | `.factory/local/runs/**` | code | The hook runner (step records), `vsdd` (run records). Every caller, the main session included, is denied reading the whole class, which gives the pipeline design's wall W3 and keeps scenario scores and scenario-review findings from builders; what a step needs from an earlier step arrives in its prompt, and people read findings through `vsdd review show`. |
| `evidence` | `.factory/local/evidence/**` | code | `vsdd gate` and the other measuring commands |
| `local` | `.factory/local/**` apart from the two above | code | `vsdd`, the hook runner |
| `reference` | `.reference/**` | none | Nobody. Reference repositories are read-only. |
| `tests`, `harness`, `ci`, `repo-config`, `manifests`, `docs`, `source` | From `paths.tests`, `paths.harness`, `paths.ci`, `paths.repo_config`, `paths.manifests`, `paths.docs`, `paths.source` in the project configuration | tools | Per the agents-skills design: test-writer `tests`, `manifests`, and `source` only before the red gate (stubs); implementer `source` and `manifests`; verifier `harness` and `tests`; technical-writer `docs`; devops `ci`, `repo-config` and `manifests`, and new files in `source` and `tests` in a `repo-setup` chore story |

**One class per product path.** The seven product classes cover the product tree: `manifests` is build
manifests and lock files; `repo-config` is toolchain and repository configuration; `harness` is proofs, fuzz
targets and benchmarks. Globs overlap in real projects (the Rust defaults `src/**` and `**/tests.rs` both
match `src/parser/tests.rs`), so a path belongs to the first class that matches in this order: `tests`,
`harness`, `ci`, `repo-config`, `manifests`, `docs`, `source`. The rule is stated here and in `classes.toml`
(`precedence = [...]`), and the write scopes, the tests-first rule and the red gate all use it. So a test in
`src/parser/tests.rs` is counted by the red gate, the test-writer may still fix it after red, and the
implementer may not edit it. Proof harnesses that live beside the code (Kani-style, under `src/`) get the
same treatment through a `harness` glob such as `**/proofs.rs`, so the verifier, which may not write
`source`, can write them. A tracked file that matches no class is reported by `vsdd doctor`, and a write to
it is blocked with the `[paths]` line to add, because a file nobody may write stops a story.

`brief` lies inside `spec-product`, `guidance` inside `spec-arch`, `holdout-evaluations` inside `holdout`,
`research-inputs` inside `research`, `decisions` inside `control`, and `runs` and `evidence` inside `local`.
This table is the only content of `classes.toml`; the agents-skills and hooks designs use these names. Every
other class is a directory no other class shares. Read walls are the agents-skills design's table over these
names.

**Deletion.** Identified specification artifacts, stories and scenarios are never deleted; they are
retired. Deferred items and research may be deleted when they are resolved or superseded, because their names
are slugs and git keeps the history. The hook runner blocks every shell deletion inside the state directory,
so deletion goes through one command, `vsdd retire <path>`, which deletes only in the classes `deferred` and
`research` and refuses anywhere else. The main session runs it in the `defer` and `research` skills. Inside a
workflow nobody has to: a research step returns the paths it superseded in `data.superseded`, and the
checkpoint retires them. Evaluations and demo files are write-once and are removed only when the cycle
closes (2.5, last table).

### 2.5 The artifacts

Caps are in bytes. "Living" means edited in place with history in git.

**The specification chain.**

| Kind | Path under `.factory/` | Frontmatter keys | Writer | Cap | Lifecycle |
|---|---|---|---|---|---|
| Product brief | `specs/product/brief.md` | `title`, `status` (draft) | The main session in the `brief` skill, which saves the draft after each section; the `product-owner`, which writes the final text | 12,000 | Living. "Approved" is computed from decision `brief`. A brief that exists with no `brief` decision is "in draft", which is a route state: the next command is `/vsdd-factory:brief`. |
| Requirements | `specs/product/prd.md` | `title`, `status` | `product-owner` | 16,000 | Living. Goals, scope, non-goals, release slicing, success measures. It lists no contracts; that list is derived. |
| Domain spec | `specs/product/domain/<section>.md`, one file per section: `capabilities`, `entities`, `invariants`, `events`, `edge-cases`, `assumptions`, `risks`, `failure-modes`, `glossary`. A section over its cap splits by area: `capabilities-billing.md`. | `title` | `product-owner` | 32,000 | Living. Items are `## CAP-012: Title` headings. A retired item keeps its heading and gains a line `Retired: replaced by CAP-031`. |
| Supplements | `specs/product/supplements/nfr.md`, `errors.md`, `interfaces.md` | `title` | `product-owner` | 24,000 | Living. Items are headings, as above. |
| Behavioral contract | `specs/product/contracts/<S>.<SS>/BC-<S>.<SS>.<NNN>.md` | `id`, `title`, `status` (draft, retired), `subsystem`, `capability`, `origin` (new, recovered), `replaced_by`, `aliases` | `product-owner` | 8,000 | draft, then retired. "Approved" is computed: the latest approved `spec` decision covers the file. "Implemented" is derived: a done story names it. Test vectors live in the contract. |
| Architecture overview | `specs/architecture/overview.md` | `title` | `architect` | 24,000 | Living. |
| Subsystem | `specs/architecture/SS-<NN>.md` | `id`, `title`, `status` (active, retired), `depends_on`, `repo` | `architect` | 16,000 | Living. The subsystem list is these files; there is no separate registry. |
| Module map | `specs/architecture/modules.toml` | TOML: `[[module]]` with `name`, `paths`, `subsystem`, `criticality`, `pure` | `architect` | 16,000 | Living. Gates read it for criticality thresholds; the hook rule `purity-advice` may read it for pure paths. |
| Conventions | `specs/architecture/conventions.md` | `title` | `architect` | 12,000 | Living. How code is written in this project. Given to builders. |
| Review rules | `specs/architecture/review-rules.md` | `title` | `architect` | 8,000 | Living. The project's own rules for reviewers. It replaces the policy registry (`agents-skills.md` N-STATE-5). |
| Decision record | `specs/architecture/decisions/ADR-<NNN>.md` | `id`, `title`, `status` (proposed, rejected), `subsystems`, `supersedes` | `architect` | 12,000 | proposed. "Accepted" is computed: an approved `spec` decision lists the record's id, and from then it is frozen, even under a later amendment. "Superseded" is derived. The old ADR-052 reached 685 KB at version 1.33 by being amended in place; here an amendment is a new record. |
| Verification property | `specs/architecture/properties/VP-<NNN>.md` | `id`, `title`, `status` (draft, withdrawn, retired), `contracts`, `method` (unit, property, fuzz, model-check, proof, manual), `module`, `harness`, `replaced_by` | `architect` | 6,000 | draft. "Active" is computed from the approved specification set, and "proven" from the `verify` closure that lists it after the `formal` check of `vsdd gate release` passed for it. Frozen once proven, which the old `protect-vp` hook never actually enforced. |
| Toolchain | `specs/architecture/toolchain.toml` | TOML: `[commands]`, `test_report`, `[paths]`, in the shapes of section 2.12 | `architect` | 4,000 | Living. The commands and path globs the project uses. For an empty repository the architect chooses them in `spec`; for an existing one `ingest` recovers them. It is part of the specification set, so the `spec` decision approves it, and the next survey writes its values into the `factory.toml` keys that are still unset (2.12). |
| Assessment | `specs/architecture/assessments/<slug>.md` | `title`, `kind` (verification, service-clones, clone, port, impact, data-model, feasibility), `cycle` | `architect` | 16,000 | Living. One per topic: the verification strategy, the service-clone assessment, one clone specification per service, a port assessment, the impact of a change. |
| UX overview, screen, flow | `specs/ux/overview.md`, `specs/ux/screens/SCR-<NNN>.md`, `specs/ux/flows/FLOW-<NNN>.md` | `id`, `title`, `status` (active, retired); a flow also has `screens` | `ux-designer` | 12,000; 8,000; 8,000 | Living. UI projects only. |
| Design system | `specs/ux/design-system/**` (JSON, YAML, Markdown) | none | `ux-designer` | 64,000 per file | Living. |
| Diagrams and images | `specs/*/assets/<name>.(svg, png, jpg, mmd, excalidraw, drawio)` | none | The owner of the directory | 512,000 per file | Living. |

**Work items and hidden scenarios.**

| Kind | Path | Frontmatter keys | Writer | Cap | Lifecycle |
|---|---|---|---|---|---|
| Epic | `stories/E-<N>/epic.md` | `id`, `title`, `cycle`, `capabilities`, `status` (open, dropped) | `story-writer` | 6,000 | "Done" is derived: every story in the directory is done or dropped. |
| Story | `stories/E-<N>/S-<N>.<NN>.md` | `id`, `title`, `status` (draft, dropped), `cycle`, `kind` (feature, fix, port, facade, docs, chore, harness), `method` (optional: another method of the kind's implement agent), `depends_on`, `contracts`, `properties`, `points`, `priority` (P0, P1, P2), `criticality` (critical, high, medium, low), `security`, `ui`, `demo`, `screens`, `repo`, `source` (the finding a fix story answers) | `story-writer` | 16,000 | draft. "Ready" is computed: the plan's closure places the story in a wave, or a run's closure records its admission (a fast-route story, a readiness chore, a fix story). "Done" is computed: its delivery closure exists. Frozen once done. The old story files grew to 480 KB by collecting review history; that history now lives in records. |
| Holdout scenario | `holdout/HS-<NNN>.md` | `id`, `title`, `status` (active, stale, retired), `contracts`, `category`, `must_pass` | `product-owner` | 6,000 | Living. There is no separate wave scenario kind: the scenarios for a wave are those whose contracts the wave's stories implement, and the rotation sample is drawn by `vsdd` from a recorded seed. |

These keys give the tool every story fact in `pipeline.md` S10. The wave is the one fact not in the story
file: `vsdd plan schedule` computes it and stores it in the plan's closure record (2.8).

**Reports written by agents during a cycle.**

| Kind | Path | Frontmatter keys | Writer | Cap | Lifecycle |
|---|---|---|---|---|---|
| Review report | `reviews/<yyyy-mm-dd>-<lens>-<n>.md` | `commit` (what was reviewed) | The reviewer for the lens, in an on-demand review started by the `review` skill | 16,000; at most 50 files, reported by `vsdd status` when exceeded | Write-once. The reviewer is handed the path, which `vsdd review prepare <lens>` returns (`agents-skills.md` N-STATE-3). A review inside the pipeline writes no file: its structured result is the review. |
| Holdout evaluation | `holdout/evaluations/<cycle>/<unit>-<n>.json` | JSON: `commit`, `seed`, `scores`, `symptoms`, and `walled_findings` (the full form of scenario-review findings and of any finding located under `holdout/`) | `vsdd checkpoint`, from the evaluator's and the scenario-reviewer's step records | 24,000 | Write-once. Inside the holdout wall. The product owner reads it when refreshing scenarios and when fixing scenario-review findings. |
| Demo evidence | `demo/<cycle>/<story>/AC-<k>.(tape, spec.ts, sh)`, `AC-<k>.txt`, `AC-<k>.png`, `report.md` | none | `demo-recorder` | script 8,000; transcript 20,000; screenshot 300,000; 1,500,000 per story | The script and the text transcript are the evidence, because they can be run again. Rendered video and GIF files go to `.factory/local/media/` and are never committed (`pipeline.md` S11). The old repository carries 110 MB of committed demo media. |
| Analysis | `analysis/<scope>/<slug>.md`, where the scope is `codebase` or a reference repository's name | `title`, `pass` | `codebase-analyzer` | 24,000; 40 files per scope | Living during ingestion, then kept as the source of recovered contracts. The old pass files ran to 198 KB. |
| Research report | `research/<yyyy-mm-dd>-<slug>.md` | `title`, `kind` (domain, technical, market, competitive, feedback, opportunity, retrospective), `question` | `researcher` | 32,000 | Living. A superseded report is deleted with `vsdd retire`, by the checkpoint when a step names it. A report of kind `retrospective` carries the cycle's proposals, each naming the line it would add to `conventions.md` or `review-rules.md`; this replaces the lessons register. |

The findings the pipeline computes with are the reviewer's structured result, stored as a step record
(2.8), and only that feeds a verdict. A person reads them with `vsdd review show <unit>`, which renders the
stored findings; the closure record keeps the dismissed and waived findings in full after the step records
are pruned. The exception is a walled finding: the closure record, the decision packet and `vsdd review
show` keep its walled key and severity only, with the line "details are inside the holdout wall", and the
full form is in the evaluation file. `vsdd review show <unit> --include-walled` prints it for a caller whose
wall allows `holdout`, and for a person in a terminal. The walled key is `walled:` and the first 12 hex
characters of the SHA-256 of the finding's key, because the key itself holds the scenario's path
(`pipeline.md` 2.7). A policy test asserts that no record outside the holdout class and `runs` contains a
scenario path or a quote from one.

**Registers.**

| Kind | Path | Content | Writer | Cap | Lifecycle |
|---|---|---|---|---|---|
| Spike ledger entry | `spikes/SP-<NNN>.md` | Frontmatter: `id`, `question`, `asker`, `run`, `ref`, `opened`, `minutes`, `verdict` (confirmed, refuted, answered, inconclusive, refused: the pipeline design's list), `method`, `agent` (the runner's agent id, from the caller marker), `asker_type` (the asker's agent type, whose walls the runner inherits; taken from the asking step's stored record or from the hook's admission record, never from the runner's argument), `read_deny` (the classes this runner was denied: its own and the asker's), `question_hash`, `worktree`, `branch`, `name` (for a named spike). Body: the answer and up to ten evidence lines. These are the fields of `pipeline.md` S6. | `vsdd spike open` creates it; `vsdd spike record` completes it | 4,000 | Permanent. `vsdd spike open` searches it first, so a question already answered is not run again; an earlier finding is returned only to an asker whose own deny set is covered by the entry's `read_deny`, so a finding produced under looser walls never reaches a stricter asker. |
| Idea | `ideas/ID-<NNN>.md` | Frontmatter: `id`, `title`, `status` (proposed, pursue, research, deferred, rejected), `score`, `decided`. Body: a summary. | `vsdd idea register` | 3,000 | Permanent, so rejected and deferred ideas are not proposed again. |
| Deferred item | `deferred/<slug>.md` | Frontmatter: `title`, `target` (a story, an epic or `unscheduled`), `source`, `severity`, `decided` (date) | The main session through the `defer` skill, which only a person can invoke | 2,000; at most 40, reported by `vsdd status` when exceeded | Deleted by `vsdd retire <path>`. Waivers and advisory findings are not stored here: `vsdd report debt` derives them from closure records, together with the debt a baseline recorded. |
| Change request | `requests/CR-<NNN>.md` | Frontmatter: `id`, `title`, `status` (queued, active, done, dropped), `received`, `cycle` (the cycle it runs in, once it has one), `amendment` (true for a request written when a `spec-gap` decision is settled `amend`). Body: the request as the person gave it, or for an amendment the gaps it covers. | `vsdd survey feature`, for every request; `settle_decision`, for an amendment; `vsdd import`, for epics that were in flight; `vsdd story new-fix --maintain`, for a maintenance cycle | 8,000 | This is the only copy of a request: `cycle.json` names it. The briefing shows how many are queued. When the cycle closes the route starts `feature` with the oldest. An amendment request is not queued: it runs inside the open cycle (`pipeline.md` 2.6). |

There is no lessons register. The first pass kept `lessons/<slug>.md`, capped at 30, to be folded into the
conventions by the architect. Nothing in the pipeline did the folding and no prompt carried a lesson to an
agent, so it would have been a smaller copy of the old 1.4 MB lessons files that changed nobody's behaviour.
The capability stays where it is used: the retrospective (a research report of kind `retrospective`) lists
its proposals, and a person adopts one with `/vsdd-factory:author conventions` or `author review-rules`,
which has the architect edit the file every builder or every reviewer is already given.

Named spikes also keep the user's existing convention inside their own worktree: `.spike/STATE.md` and
`.spike/TASKS.md` on branch `spike/<name>` in `.worktrees/spike-<name>/`. Those two files belong to the
spike and are never read by the pipeline. The project ledger entry is the only thing that crosses over.

**Where the remaining artifact types of the brief live.** Three of them are not files an agent writes.

| Type | Where it is | Written by |
|---|---|---|
| Wave | Membership: the `waves` map in `cycles/<cycle>/units/plan.json`. Progress, gate rounds and gate status: `cycles/<cycle>/units/wave.<W-NN>.json` (in a hub, `wave.<W-NN>.<repo>.json`, one per repository with stories in the wave; the wave is closed when every one of them is). | `vsdd plan schedule`, `vsdd gate wave`, `vsdd checkpoint` |
| Convergence record | The `verdict` in the unit's closure record: perimeter, commit, passes, outcome, and the counts of advisory, dismissed and waived findings (2.8). The trajectory across passes is derived from the cycle's step records while they exist, and from the verdicts afterwards. | `vsdd checkpoint`, by the pipeline design's rule |
| Finding | In the reviewer's step record, in the pipeline design's finding schema. Findings still open when a loop ends are copied into the decision packet. Findings a person waived are listed in the closure's verdict. A walled finding is copied as walled key and severity only; its full form goes to the evaluation file. | The hook runner (step record); `vsdd checkpoint` (packet, verdict, evaluation file) |

**What closing a cycle does.** The `release` checkpoint closes the cycle (a person can also run
`vsdd cycle close` in a terminal to repair one). It adds to `cycles/<cycle>/cycle.json` the close date, the
release version and the commit of the state branch just before pruning (totals and pass counts are derived
from the closure records, so there is no separate summary file), then removes
`demo/<cycle>/` and `holdout/evaluations/<cycle>/` from the tree and the cycle's runs
and evidence from the local store. The removed files stay in the state branch's history at the recorded
commit. The living tree therefore holds full detail for the open cycle only, and its size is bounded by
the size of the specification, not by how long the project has run. `state.prune_closed_cycles = false`
keeps the detail (open question 5).

### 2.6 Stored and derived

| Fact | Old system | 1.0 |
|---|---|---|
| Catalogue of contracts, properties, stories, scenarios, subsystems, research | Eleven hand-kept index files, four of them over 400 KB | `vsdd list <kind>`, from a cache keyed by path, size and modification time |
| Counts | Stored in `STATE.md`, the requirements document and every index | Never stored |
| Titles in other files | Copied, with a policy that the heading wins | Stored in the artifact's frontmatter only |
| Reverse links, traceability matrix, coverage matrix, UI traceability | Stored in several files and kept in step by policy | `vsdd refs <id>`, `vsdd coverage` |
| Subsystem registry, epic list, dependency graph | `ARCH-INDEX.md`, `epics.md`, `dependency-graph.md` | Derived from the subsystem files, the epic directories and `depends_on` |
| Wave schedule | `wave-schedule.md`, `sprint-state.yaml`, `wave-state.yaml` and a `wave` key in every story | Computed by `vsdd plan schedule` from dependencies and limits; stored once in the plan's closure record |
| Whether a story is in progress | A status an agent set | Derived: its wave is open and it is not done. Its position comes from evidence. |
| Whether a contract is implemented, an epic done, a decision record superseded | Status values promoted by an agent after merge | Derived |
| Version of an artifact, and citations of that version elsewhere | A `version` key, bumped by hand, cited in stories and indexes | Git history. A decision records the digest of what it approved (below). |
| Whether a file is approved, a story ready or done, a property proven | A status an agent or the state-manager wrote into the file | Computed from the decision or closure record that made it so (2.4); shown by `vsdd list` |
| Staleness | `inputs` and `input-hash` in every artifact | Not tracked per stage. An approval is bound to a digest of what was approved, so a changed specification makes its decision stale and the stop returns. After approval a contract changes only through an amendment, whose change analysis names the stories it affects, and `plan` in delta scope re-traces them. A stage is run again only when a person's `revise` or `hold`, or an amendment, marks it not done. There is no `vsdd stale` report and no `built_against` field. |
| Product commit ids | Cited in `STATE.md` and patched after each commit | Recorded once in evidence and closure records; never updated |
| Convergence trajectory, cycle manifest, pull request description, debt report | Written and maintained | Derived from closure records |
| Changelogs, amendment histories, burst logs, decision logs | Hand-kept files of up to 2 MB | Git history of the state branch |

The digest is `vsdd.state.digest(paths)`: SHA-256 over the sorted list of each file's path and content
hash, cut to 16 hex characters. It is over the whole file: approval changes no file, so there is nothing to
leave out of a file. Decision records use it; section 2.8 says which files a `spec` decision covers.

### 2.7 The status report: computed, not stored

There is no status file. The first pass kept `.factory/local/status.json`, a derived copy that nine commands
each had to rewrite, whose absence was a failure condition with its own repair, and which could disagree with
the records it was copied from. The second pass removed it. `vsdd.state.status(root)` computes the object
below from the records each time it is asked: `factory.toml`, the lock, the driver flag, the open cycle's
`cycle.json`, its closure records and open decisions, the spike ledger's open named entries, `notes.json`,
and then the route. That is tens of small file reads and no subprocess, a few milliseconds, well inside the
eight seconds the briefing is allowed. `vsdd status --json` prints it, and the briefing (2.9) renders it, so
the two cannot disagree, and nothing can be stale or missing. Notes, the one part that is not derived, live
in the tracked `.factory/notes.json`. `next` is the object `{kind, command, args}` of the pipeline design's
workflow result. The object holds only what the briefing prints and what the hook rules test.

```json
{
  "schema": 1,
  "updated": "2026-11-26T16:40:00Z",
  "project": "acme-api",
  "cycle": {
    "id": "auth-sso",
    "kind": "feature",
    "title": "Single sign-on for the admin console",
    "opened": "2026-11-23",
    "units": { "spec": "closed", "plan": "closed", "verify": "open" },
    "waves": { "W-01": "passed", "W-02": "open" },
    "wave": { "id": "W-02", "stories": 5, "delivered": 3, "blocked": 1 }
  },
  "decisions": [
    { "id": "finding.S-7.04", "kind": "finding", "question": "Waive or direct: lockout counter race",
      "packet": ".factory/cycles/auth-sso/decisions/finding.S-7.04.md" }
  ],
  "spikes": ["uv-startup"],
  "others": [],
  "requests_queued": 0,
  "driver": { "state": "on", "at": null },
  "next": { "kind": "workflow", "command": "/vsdd-factory:wave", "args": {} },
  "notes": [ { "at": "2026-11-26T16:40:00Z", "text": "PR 41 needs the staging secret before it can merge." } ]
}
```

| Field | Meaning | Limit |
|---|---|---|
| `schema`, `updated`, `project` | Format version, when it was computed, project name | |
| `cycle` | The active cycle, or `null` | One |
| `cycle.units` | State of each non-story unit: `open`, `closed`, `blocked`, `failed` | One entry per workflow |
| `cycle.waves` | Gate status of each wave: `open`, `gating`, `passed`, `failed`. `passed` is the only passing value. | 60 waves |
| `cycle.wave` | Counts for the open wave | |
| `decisions` | Open decisions: identifier, kind, one line, packet path | 12 listed, then a count |
| `spikes` | Open named spikes | 8 |
| `others` | One line each for a second cycle in flight (a critical fix), with its qualified decision identifiers | 3 |
| `requests_queued` | How many change requests wait for the cycle to close | |
| `driver` | Whether the driver is `on`, `paused` (and at which decision) or `off`, and in which session | |
| `next` | The object `{kind, command, args}` from the route table. `kind` is `workflow`, `human` or `done`. For the second of two open cycles `args` carries `cycle`. The briefing renders it as text | The rendered text is at most 200 characters |
| `notes` | Free text a person asked to keep, through `vsdd note` | 5 entries of 200 characters; the oldest is dropped |

The lists are shortened so that the rendered object stays under 8,192 bytes.

Who writes what the report is computed from. Nothing writes the report. The records it reads are written
only by `vsdd`, at these points.

| Moment | Command | What it writes | Run by |
|---|---|---|---|
| A workflow starts | `vsdd survey` | The run record and the lock; a cycle record when a request starts a cycle | The clerk step that opens every workflow |
| A workflow ends | `vsdd checkpoint` | Closure records, decision records and packets | The clerk step that closes every workflow |
| A story merges, a wave gate is measured | `vsdd story deliver`, `vsdd gate wave` | The story's or the wave's closure record | A clerk step |
| A person settles a decision | `vsdd decide`, called by the hook runner when the person types `/vsdd-factory:decide`, or by the person in a terminal | The decision record, and nothing else | The hook runner or the person; never an agent |
| A cycle opens | `vsdd checkpoint brief`; the checkpoint of `discover` in assess mode (the same code); `vsdd survey feature`; `vsdd story new-fix --maintain` when it is about to write fix stories and no cycle is open | `cycle.json` | A clerk step, or the main session for the brief |
| A change request's route is derived | `vsdd feature route` | The `route` field of `cycle.json` | The architect, as the last act of its delta step |
| A cycle closes | The `release` checkpoint | The close fields of `cycle.json` | A clerk step |
| A named spike opens or closes | `vsdd spike open --name`, `vsdd spike close` | The ledger entry | The spike runner |
| A person leaves a note | `vsdd note "<text>"` | `notes.json` | The main session, on request |

The pipeline design runs one cycle at a time; a critical fix is the only second line of work, and the report
has room for it in `others`.

### 2.8 Records written by code

These are the stores the pipeline design asks for (its S3 to S8). This design fixes where each lives, its
size, and whether it is tracked. Their field lists are the pipeline design's schemas in
`<plugin>/schemas/`.

| Store | Path | Tracked | Written by | Cap | Kept until |
|---|---|---|---|---|---|
| Cycle record | `cycles/<cycle>/cycle.json`: `id`, `kind` (product, feature, hotfix, maintenance), `title`, `route` (`standard` from the opening when a brief opens the cycle; written by `vsdd feature route` when a change request opened it; empty for a maintenance cycle), `opened`, `baseline` (a release tag, an ingest or an import; empty for a first release), `request` (`brief`, or the id of the one request file, `CR-<NNN>`); and once closed, `closed`, `version`, `archive_commit` | Yes | Whichever command opens the cycle: `vsdd checkpoint brief`, the `discover` assess checkpoint, `vsdd survey feature`, `vsdd story new-fix --maintain`. `vsdd feature route` writes `route`. The `release` checkpoint adds the close fields | 2,000 | Permanent |
| Closure record | `cycles/<cycle>/units/<workflow>[.<subject>].json`, for example `spec.json`, `plan.json`, `wave.W-02.json`, `story.S-7.03.json`, `verify.json`; and in `_standing/`, `ingest.json` or `import.json`, which hold the baseline | Yes | `vsdd checkpoint`, `vsdd story deliver`, `vsdd plan schedule`, `vsdd gate plan --story` and `vsdd story new-fix` (admission), `vsdd import` | 16,000 | Permanent |
| Decision record and packet | `cycles/<cycle>/decisions/<id>.json` and `<id>.md` | Yes | `vsdd checkpoint` opens; `vsdd decide` settles; `vsdd import` writes one approved `spec` decision | 4,000 and 12,000 | Permanent |
| Run record | `local/runs/<run-id>/run.json`: workflow, subject, repository, cycle, session, start, agent count, parent run, the stories admitted to it, the request or question it was given, whether it holds an amendment | No | `vsdd survey`; the admission commands | 2,000 | 30 days, or until the cycle closes |
| Step record | `local/runs/<run-id>/steps/<workflow>/<subject>/<name>.<attempt>.json`, the key in `pipeline.md` S3. It carries the calling agent's type and the worktree head, both added by the hook | No | The hook runner, when the step returns (`pipeline.md` H1) | 64,000 | As the run |
| Evidence record | `local/evidence/<subject>/<gate>.<sha12>.json`, with the raw output (`.log`, cut to 256 KB), the JUnit report, or the diff (`diff.<sha12>.patch`) beside it. One record per gate and commit. Each record also carries the `git patch-id` of the subject's diff. A re-freeze is a `red` record of its own kind | No | `vsdd gate`, `vsdd survey` (diff patches), `vsdd story deliver` | 8,000 | Until the subject's unit closes |
| Job record | `local/jobs/<key>.json` and `<key>.log`, where the key is a hash of the command, its subject and the commit; a product started by `vsdd env start` is a job record with its port, process id and the caller's agent id | No | Any long command, the first time it is run for that key; `vsdd env start` | 4,000 plus the log | 7 days |
| Run lock | `local/lock.json` | No | `vsdd survey`, `vsdd checkpoint` | 2,000 | 2.11 |
| Driver flag | `local/driver.json`: `on`, `paused` at a decision id, or absent; and the session | No | `vsdd driver on`, `pause`, `off` | 500 | Until it is turned off |

There is no stored verdict record. The first pass wrote `review.<sha12>.json` beside the evidence; it was a
second copy of a verdict that could disagree with the first. The verdict is a function,
`vsdd.state.verdict(project, subject)`, computed from the stored passes of any run for the unit, so a
converged review still outlives the run that produced it. After the checkpoint the verdict is in the unit's
closure record, which is what a fresh clone reads. The hook runner's own records (caller markers, spike
admissions, the configuration fingerprint) are under `local/hooks/` and belong to the hook design.

What a closure record holds. One schema serves every unit.

```json
{
  "schema": 1, "workflow": "story", "subject": "S-7.03", "cycle": "auth-sso",
  "state": "closed",
  "position": "delivered",
  "stages": { "tests":  { "state": "done", "sha": "1a2b3c4", "artifacts": ["tests/test_lockout.py"], "at": "2026-11-26T10:02:00Z" },
              "review": { "state": "done", "sha": "5d6e7f8", "artifacts": [], "at": "2026-11-26T12:40:00Z" } },
  "verdict": { "perimeter": "story", "head": "5d6e7f8", "passes": 2, "rounds_used": 2, "outcome": "converged",
               "advisory": 1, "dismissed": 0, "waived": [] },
  "delivery": { "branch": "story/S-7.03", "pr": 41, "merged_sha": "9c1d2e3", "base": "develop" },
  "admitted_by": "plan",
  "decision": null,
  "closed": "2026-11-26T13:05:00Z"
}
```

The plan's closure record also holds `waves`, the map from wave to stories that `vsdd plan schedule`
computed. A wave's record holds its stories and its gate rounds. `verify.json` holds the checks per round and
the properties proven. A story's record, a wave's and `verify.json` also hold `flaky`, the identities of
inherited tests a gate recorded as flaky (below); the command that writes the record copies them from the
gate's evidence, which is not kept. A story's `admitted_by` says how it became ready: `plan`, or the run that admitted it
through `vsdd gate plan --story` or `vsdd story new-fix`. The stage entries are in the order the stages
finished, which is what lets `revise` clear "from the review stage onward" without a list of stages.

The baseline record. `cycles/_standing/units/ingest.json` (or `import.json`) is a closure record with a
`baseline` object: the integration head it was measured on; any earlier release tag; the suite result with
the identities of the tests that failed; the lint and format findings; the mutation score per module; the
scanner findings in existing dependencies; the recovered or imported contract ids; and whether the project
has a suite, a JUnit report and CI. `vsdd gate ingest` measures it; for an import, the first survey that
needs the measurements takes them. A cycle's `cycle.json` names its baseline. The green, wave and release
gates read it and fail only on what it does not list, and they apply mutation and coverage thresholds to the
modules the cycle touched. An inherited test that passed at the baseline and fails now is run once more
before it counts as new; one that passes in either run does not fail the gate, and is recorded as flaky in
the gate's evidence and then in the unit's closure record (`pipeline.md` 2.9.8). A cycle whose baseline is
empty is a first release and runs `verify` in full scope; every other cycle runs in delta scope.
`vsdd report debt` lists what the baseline recorded and the flaky tests the closure records hold.

Why step records and evidence are not tracked. They are per run and per commit, they are large in total
(hundreds of step records in a cycle), and they can be regenerated: a gate is a command, and a step can be
run again. The closure record is the tracked statement of what they proved. This matters for the done test
in `pipeline.md` section 2.5, which this design refines for a machine that has no local store:

- A stage is done when its closure entry says `done`, or, within a run that has not yet reached its
  checkpoint, when a done step record exists; and the artifacts that entry lists still exist and validate;
  and the gate it must pass has passing evidence for the current commit. There is no input digest: nothing
  reopens because an upstream file changed.
- When a closure names a gate that passed at the current commit and the evidence record is missing, the
  survey measures the gate again before deciding. It does not send the stage back to an agent.
- A stage is marked not done only explicitly. Settling `revise` or `hold` clears the entries from the stage
  the person named with `stage=` or, by default, from the entry named `review` onward. Opening an amendment
  clears the specification unit's entries the same way. An intake step's `covers` list is recorded as done
  entries for the stages it names, once their artifacts validate.
- The tool keeps no list of stages. It records the stage names the scripts report and returns the names
  that are done; the script decides what to skip.

So a fresh clone resumes from the last checkpoint, and loses only the steps of a run that never reached its
checkpoint.

How a decision is protected. The record holds `id`, `kind`, `status` (open, approved, revise, rejected,
stale), `choice`, `values` (the `key=value` pairs), `note`, `digest`, `opened`, `settled`,
`settled_by` (`session`, `terminal` or `import`), `settled_by_session`, and for a `blocked` decision its
`reason`. A `spec` decision also holds `files` (the paths the digest covers) and `accepted` (the ids of the
decision records it accepted). A `merge` decision holds `dependencies` (the names and versions it approved).
`vsdd checkpoint` computes the digest of what is being decided (the specification file set for `spec`, the
commit for `release`), over the files exactly as they are. The specification file set is every file in the
three specification classes except the classes `brief` and `guidance`: the brief has its own decision, and
the two guidance files have their own lock (2.4) and may change between cycles, so drafting a brief or
editing the guidance never makes a `spec` approval stale. The survey recomputes the
digest, and a decision whose digest no longer matches reads as `stale` and cannot be settled.

Settling changes no artifact. `settle_decision` writes the decision record and nothing else (with one
small exception: for `spec-gap amend` it also writes the amendment request file): it does not
promote any file, it does not run git, and it returns in a few milliseconds, which is what lets the hook
that captures a typed decision stay inside its four-second deadline on a specification of any size. The
next survey commits the record. What follows from an approval is read from the record afterwards: the
specification set is locked because the latest `spec` decision is approved (2.4); a story is ready because
a closure places it. One fixture test holds the sequence that used to fail: approve `spec`, run the `plan`
survey, and the approval is still current.

The command grammar is `<id> <choice> [key=value ...] [-- note]`. Values are named, a key may repeat, and
free text follows `--`; there are no positional values. Where two cycles are open and both hold a decision
with the same id, the id is written `<id>@<cycle>`, and a bare id that is ambiguous is refused with the
choices. Nine kinds; choices map to statuses as follows.

| Kind | Choices that set `approved` | `revise` | `rejected` |
|---|---|---|---|
| `brief`, `plan` | `approve` | `revise -- <guidance>` | `drop` (`brief` only) |
| `spec` | `approve [variant=<id>] [topology=single or split]` | `revise [stage=<name>] -- <guidance>` | `drop` |
| `release` | `approve` | `hold -- <guidance>` | none |
| `merge` | `approve`: the next `vsdd story deliver` merges, and the dependencies the decision names are recorded as approved | none | `reject`: the next survey closes the pull request and the story is descoped |
| `ideas` | `select` with `pursue=`, `research=`, `defer=`, `reject=` lists of idea ids; each idea's own status is set from its list | none | none |
| `finding` | `waive key=<key> [key=<key> ...] -- <reason>`, `waive-coverage item=<item> [item=<item> ...] -- <reason>` (the keys or items are recorded as waived) | `direct -- <guidance>` | `descope` |
| `spec-gap` | `amend` (writes an amendment request for the open cycle) | `not-a-gap -- <reason>` | `descope` |
| `blocked` (reason `spike_limit`, `environment`, `stuck` or `reviewer`) | `retry`; `open-spike name=<name>` for `spike_limit`; `skip` for an optional environment check | `direct -- <guidance>` | `descope` |

Dependencies approved between specifications. `dependencies.toml` is part of the locked specification set,
so a dependency a person approves in a `merge` decision cannot be written into it during a build.
Dependency detection in `vsdd story deliver` therefore reads `dependencies.toml` and the `dependencies` of
every approved `merge` decision that is newer than the latest approved `spec` decision. The next `spec` or
amendment run has the architect fold them into `dependencies.toml`.

**Evidence lookup.** `evidence(project, gate, subject)` depends on the gate. For `red` it returns the latest
passing record, or re-freeze record, on any commit between the story's base and its head; that record lists
the frozen test identities and stays valid as the implementer commits. For `green` and `demo` it returns the
record for the head commit, or one whose patch id equals the head's after a clean rebase. For `review` it
returns the verdict the tool computes from the stored passes, or the one in the closure record.

### 2.9 The session briefing

The hook rule `session-briefing` prints it on every `SessionStart` (startup, resume, clear, compact and
fork). This design owns the text and the limit; the function is `vsdd.state.briefing(root, session_id,
source)`, which `vsdd status` also calls, so the two cannot disagree (`packaging.md` S5, `hooks.md` S4).

How it is computed. `briefing()` renders the status object of section 2.7, which is computed from the records
each time: `factory.toml`, `local/lock.json`, `local/driver.json`, the open cycle's `cycle.json`, its closure
records and open decision records, the open named spikes, `notes.json`, and then the route. That is tens of
small file reads and no subprocess. The mount test of section 2.3 adds two more reads. It returns in
milliseconds; the hook design gives the briefing eight seconds.

Content, in this order. Lines are dropped from the bottom if the limit would be passed.

```
vsdd-factory: acme-api. Cycle auth-sso (feature).
Build: wave W-02, 3 of 5 stories delivered, 1 blocked. Specification and plan are closed.
Waiting on you (1): finding.S-7.04  Waive or direct: lockout counter race
  Read .factory/cycles/auth-sso/decisions/finding.S-7.04.md, then /vsdd-factory:decide finding.S-7.04 <choice>
Running: wave W-02, started 14:02 in this session. Do not start it again. The driver is active: continue it.
Next: /vsdd-factory:wave
Named spikes open: uv-startup
Note: PR 41 needs the staging secret before it can merge.
Full report: /vsdd-factory:status
```

| Line | Shown when | Source |
|---|---|---|
| Project and cycle | Always. With no cycle: "No cycle is open." | `cycle.json` |
| Position | A cycle is open | The cycle's closure records |
| Waiting on you | A decision is open. At most three, then a count. Identifiers are qualified with `@<cycle>` when two cycles are open. A `blocked` decision's line states its reason. | The open decision records |
| Running | The run lock is held by a live run. It says whether that run belongs to this session, and whether the driver is active or paused (`pipeline.md` S9). | `local/lock.json`, the run record, `local/driver.json` |
| Next | Always | The route, rendered from the `next` object |
| Named spikes | Any are open | The spike ledger |
| Note | Any exist. At most two. | `notes.json` |
| State problems | The condition is `partial` (the mount is broken); the last push failed; `[commands]` is not configured | The mount test; the push result the checkpoint recorded in the run record; `factory.toml` |
| Full report | Always | |

When `local/driver.json` names this session, the "Running" line is followed by the instruction that lets a
compacted main session carry on without the person. When the driver is on: "The driver is active. When the
running workflow returns, start the command in `next` with the Workflow tool and repeat; at a decision for
a person run `vsdd driver pause <id>` and stop; at done, or after two runs with no progress, run `vsdd
driver off`." When the driver is paused at a decision: "The driver is paused at `<id>`. When that decision
is settled, resume the driver loop." The context the `decide` hook returns carries the same instruction, so
after an approval the person does not type `/vsdd-factory:drive` again. `vsdd driver on`, `pause` and `off`
write that file. Queued change requests are shown as a count on the position line.

After a compaction the first line is preceded by: "Context was compacted. This briefing is computed from
disk and is current." The running workflow, if any, is named by the "Running" line from the lock and the
run record; there is no compaction snapshot to read. The hook rule may then add its own lines (relaxed
gates, a changed gate-bearing section).

Limit: 1,200 characters and 12 lines for this design's part, inside the hook rule's own ceiling of 2,000.
A subagent never receives it. It contains no file contents and nothing from the holdout class.

### 2.10 Flush before compaction, and the one status entry point

**What is flushed before compaction.** The hook rule `compaction` calls flush with reason `precompact`.
There is usually little to commit, because flush already ran at the last flush point (2.3): the start of a
survey, a checkpoint, a delivery. Nothing else needs saving at that moment, by construction:

| What a main session knows | Where it already is |
|---|---|
| Which workflow is running and for what | The run lock and the run record, written by the survey |
| What each finished step returned | Step records, written by the hook runner as each step returned |
| What a person decided | Decision records, written when the person typed the command |
| Something a person said that must survive | Only if it was saved with `vsdd note`. The `status` skill tells the main session to do that when a person gives a standing instruction. |

**The status entry point.** `/vsdd-factory:status [--fix]` is the `status` skill of the agents-skills
design. It tells the main session to run `vsdd status [--fix]` and relay the output. It is safe at any
time, in any session, and running it twice changes nothing.

`vsdd status` does three things in order, and writes nothing.

1. Checks: the mount; that `factory.toml` and the open closure and decision records parse;
   `vsdd validate` on files changed since the last flush; uncommitted and unpushed state; the run lock and
   whether its holder is alive; jobs left running by an earlier session; spike worktrees against the ledger;
   raised caps and waived files; tracked product files that fall in no path class; a member repository that
   is not cloned.
2. Reports: the cycle; each unit with its state; the open wave's stories with their positions and computed
   statuses, taken from closure records and evidence; open decisions with their packets; the lock and its
   holder; open spikes; problems, each with the command that fixes it. `--json` gives the same as data
   (`pipeline.md` S9).
3. Names the next command.

With `--fix` it also repairs what is safe to repair, and only then does it write: it remounts the state
directory; releases a lock whose holder is dead; removes spike worktrees past retention and worktrees of
delivered stories; prunes runs, jobs and evidence past retention; clones a member repository that is missing
from the `url` in its `[[repo]]` entry; rebuilds the catalogue cache; and flushes. It never deletes a
tracked file and never changes a decision. Because it releases locks and removes worktrees, `status --fix`
is not in the read group that every agent holds: the main session and the clerk may run it (blueprint 4.9).

It replaces six old skills.

| Old skill | What it did | Why it is not needed |
|---|---|---|
| `wrap` | Paused the pipeline, had the state-manager write a checkpoint, released the lock, and verified fifteen postconditions before saying it was safe to clear | There is nothing to pause. State is on disk after every step. To stop, stop. |
| `wave-handoff` | Wrote `HANDOFF.md` and `wave-state.yaml` at wave close | The wave's closure record and the briefing carry it. |
| `wave-reset` | A ritual for clearing the session at a wave boundary | The main session holds a few hundred bytes per run. Clearing it is always safe. |
| `rehydrate-wave` | Re-read a listed set of spec files into a new session | The main session does not need them. Each step is handed the paths of its unit by the survey. |
| `recover-state` | Rebuilt `STATE.md` from artifacts by an agent's reasoning | There is no status file to rebuild: the status is computed from the decision and closure records each time. |
| `compact-state` | Moved history out of `STATE.md` when it grew | Caps are checked before a write. Nothing grows. |

**What happens in each resume case.**

| Case | What is on disk | What happens |
|---|---|---|
| The context compacts while a workflow runs | Everything; flush ran in `PreCompact` | The workflow keeps running. The briefing after compaction says which run is live and that the driver was active. |
| The session dies in the middle of a wave | Step records and evidence up to the last finished step; closures up to the last checkpoint; story branches and worktrees | The next briefing says a run started and did not finish. If the old session ended normally its lock is already released. After a hard crash the briefing names `/vsdd-factory:status --fix` (and, for a terminal, `vsdd lock release`), or the lock lapses after 20 minutes. The same command then starts a new run; its survey skips what is done, continues each story from its position, and re-attaches to any gate or delivery job the dead session left running, because a job is keyed by its command, subject and commit and so is never started twice. |
| A new session, days later | Tracked state and the local store | The briefing gives the position and the next command. |
| A fresh clone on another machine | Tracked state only, after `setup` mounts it | The survey resumes from the last checkpoint and measures gates again where evidence is missing. A story branch that was pushed has a pull request the tool opened, with the story trailer, and is adopted (2.11). |
| The mount is broken | The state branch | The condition is `partial`. `vsdd status --fix` remounts. Uncommitted files in the old directory are kept and reported. |

### 2.11 Two sessions at once

The run lock is `.factory/local/lock.json`, the answer to `pipeline.md` S8 and `packaging.md` S4.

```json
{ "locks": [ { "scope": "acme-api:develop", "run": "r-20261126-1402-7q", "workflow": "wave", "subject": "W-02",
               "session": "0c1d…", "taken": "2026-11-26T14:02:00Z", "seen": "2026-11-26T14:31:10Z" } ] }
```

- `vsdd survey` takes it once its preconditions pass, scoped to the repository and the branch the run can
  write. Every run that can merge into the integration branch takes the same scope, so a standalone `story`
  run and its wave exclude each other. A second run on the same scope gets `ok: false` with the holder. The
  take is atomic: a file lock on `local/lock.flock` is held around the read, the check and the write.
- `vsdd` learns who is calling from the caller marker the hook runner writes for that one call. Before it
  lets a `vsdd` call through, the hook writes `local/hooks/calls/<key>.json` (session id, agent id, agent
  type, working directory, time) and rewrites the command, through the hook's `updatedInput`, to carry the
  key in the environment (`VSDD_CALLER=<key>`). The tool reads the marker with its own key and no other. A
  per-session marker could not do this: a `vsdd` process does not know its own session id, so with two
  sessions open it could have taken the other session's marker, recorded the lock under the wrong session
  and passed the hooks-alive check while its own hooks were off. The survey refuses to start a run when its
  call has no marker, because that means the hooks are not active for this session. The same marker gives
  `vsdd spike open` the runner's agent id, `vsdd env start` the agent to clean up after, and `vsdd doctor`
  the proof that hooks are live. A caller cannot forge it: `gate-guard` blocks a command that sets a
  `VSDD_*` variable itself, and the marker directory is in a class only code writes. Blueprint section 10,
  item 27 tests the rewrite. If it fails, the marker is keyed by the Claude Code process the hook and the
  tool both descend from (item 29), and matched to an agent by working directory, which is unique for every
  isolated agent.
- `vsdd checkpoint` releases it. So does the hook runner when the holder's session ends (section 5, H-2),
  which covers a person closing a session in the middle of a run.
- The holder counts as alive while `seen` is recent. The hook runner owns liveness: on any guarded tool call
  from the holder's session, or from a working directory inside a worktree the locked run owns, it refreshes
  `seen`, at most once a minute. The second test covers the case, not yet probed, in which tool calls inside
  a workflow step carry a different session id from the main session (blueprint section 10, item 28). After
  `state.lock_stale_minutes` (default 20) without one, the lock is stale and the next survey takes it over.
  A foreground command can run for ten minutes at most, so a live run is not mistaken for a dead one.
- After a hard crash there is no session end, so the lock stands until it goes stale. The briefing and
  `vsdd status` then say which session holds it, when it was last seen, and what a person runs if they know
  that session is gone: `/vsdd-factory:status --fix` in a session, which releases a lock whose holder is
  dead, or `vsdd lock release` in a terminal once the shim is installed. `vsdd status` shows the lock at any
  time; there is no separate `lock status` command.
- The `spike` workflow, `discover` and a report-only `maintain` take no lock.
- It is local, so it does not stop two machines. They meet at the state push: the second fetches, rebases
  and retries, and a real conflict in the same record is reported to a person.
- A story branch on the remote. `vsdd story deliver` pushes the story branch and opens the pull request with
  a trailer in its body naming the story (`Factory-Story: S-7.03`). The survey treats a remote story branch
  in one of two ways. It is adoptable when its pull request carries that trailer and the tracked state shows
  the story undelivered in the open cycle: the survey fetches it and continues the delivery from the open
  pull request. That is this machine's own branch after a crash, and equally a fresh clone picking up where
  another left off, which is safe because the merge is one idempotent, head-matched act. Otherwise it is
  owned elsewhere: the remote head is neither the local branch's head nor an ancestor of it, no local run
  record names the branch, and it has no pull request the tool opened. The survey then reports the story as
  a blocked unit with kind `conflict`, which the checkpoint turns into a `blocked` decision if it persists.

Writes to machine records are atomic (write a temporary file, then rename) under `flush.lock`, so two
sessions that legitimately run at once (a wave and a report-only sweep) cannot corrupt a record.

### 2.12 The project configuration file

`.factory/factory.toml`. It exists if and only if the project is initialised for 1.0. `vsdd init` creates it
by detection and a person edits it afterwards; no agent tool call can write it, so an agent blocked by a gate
cannot relax the gate. Code writes it in three stated cases, by appending or by filling a key that is unset,
never by rewriting a value a person set: `vsdd init` and `vsdd import` when they create it; `vsdd repos
split`; and the survey, when an approved `spec` decision covers a toolchain file and `[commands]` or
`[paths]` keys are still unset. Each is announced in the next briefing. Otherwise code only reads it,
because the standard library can read TOML and cannot write it back with comments intact. Every other
machine-written file is JSON. Unknown keys and wrong types are errors that the briefing reports, and the
hook design's rule applies: an invalid value never relaxes a gate.

```toml
schema = 1

[project]
name = "acme-api"
languages = ["rust"]
ui = false
layout = "single"                    # single | hub (2.14)

[branches]
integration = "develop"              # stories merge here
release = "main"
protected = ["main"]
story = "story/{id}"                 # branch patterns the hook runner maps back to stories and spikes
spike = "spike/{name}"

[commands]                           # each is one shell string, run from the root of the worktree under test
build = "cargo build --workspace"
test = "cargo nextest run --workspace --profile ci"
test_report = "junit:target/nextest/ci/junit.xml"   # required: the red gate needs per-test results
lint = "cargo clippy --workspace --all-targets -- -D warnings"
format_check = "cargo fmt --check --all"
run = ""                             # how to start the product, for demos and hidden scenarios. It must take
                                     # its port from {port} or $VSDD_PORT; vsdd init and vsdd doctor check that
ui_check = ""
docs_check = ""                      # the documentation check the green gate runs for a docs story
mutation = ""
scan = ""
audit = ""
bench = ""
proofs = ""

[commands.env]                       # optional: environment for every configured command, e.g. a shared build cache
# SCCACHE_DIR = "~/.cache/sccache"

[paths]                              # globs relative to a worktree root. Seven classes cover the product tree.
                                     # A path that matches more than one belongs to the first in this order:
                                     # tests, harness, ci, repo_config, manifests, docs, source (2.4)
tests = ["tests/**", "crates/*/tests/**", "**/tests.rs"]   # so src/parser/tests.rs is a test file
harness = ["fuzz/**", "benches/**", "proofs/**", "**/proofs.rs"]   # proofs, fuzz targets, benchmarks, in-source proof files
ci = [".github/**"]
repo_config = ["rust-toolchain.toml", ".gitignore", ".editorconfig", "clippy.toml", "rustfmt.toml", "deny.toml"]
manifests = ["Cargo.toml", "Cargo.lock", "crates/*/Cargo.toml"]   # build manifests and lock files
docs = ["docs/**", "README.md"]
source = ["src/**", "crates/*/src/**", "build.rs"]

[delivery]
mode = "pr"                          # pr | local. vsdd init sets local when the repository has no remote
ci = "required"                      # required | none. vsdd init sets none when the repository has no CI; only
                                     # then does "no checks" count as a pass, and a CI chore story is scheduled
merge = "auto"                       # auto | human
merge_strategy = "squash"
protected_paths = [".github/**", ".env*", "CODEOWNERS", "CHANGELOG.md"]   # always wait for a person. vsdd init
                                     # adds the files [release] names: version files, the change log, the
                                     # release workflow
dependency_changes = "human"         # human | auto: a new dependency, or an upgrade beyond dependencies.toml
                                     # and the dependencies approved merge decisions list
ci_wait_minutes = 30                 # the total wait for CI, taken as one job in slices of nine minutes

[convergence]                        # loops; the defaults are the pipeline design's
passes = 3                           # review passes at the story and spec perimeters
rounds = 3                           # review rounds at the wave and release perimeters

[limits]
agents_per_run = 900
agents_per_story = 40                # the budget each child story receives
stories_per_wave_run = 24
step_attempts = 2
delivery_rounds = 3
gate_minutes = 60                    # the longest a gate job may run
parallel_gates = 4                   # how many gate jobs run at once
failed_runs_before_stuck = 2

[spikes]
per_step = 2
per_run = 6
per_day = 20
minutes = 20                         # the time box
max_open = 3                         # named spikes open at once; enforced by vsdd spike open --name
retain_days = 14                     # how long a finished spike's worktree is kept

[holdout]
mean_min = 0.85
must_pass_min = 0.6
stddev_max = 0.15
rotation = 0.8
batch = 12                           # scenarios per evaluator step

[mutation]
kill_rate = { critical = 0.95, high = 0.90, medium = 0.80, low = 0.70 }   # by module criticality

[stops]
plan = "auto"                        # auto | always: when the plan decision opens
plan_ceiling_agents = 600            # with auto, the plan decision opens when the estimate passes this

[evidence]
wave_demo = false                    # a demo at the wave gate
final_demo = false                   # a demo in the release UI review

[maintain]
sweeps = []                          # empty means every sweep that applies; or a list of sweep names
fixes_per_run = 10

[release]                            # detected by vsdd init; run by vsdd release
version_files = []                   # e.g. [{ path = "Cargo.toml", key = "package.version" }]
changelog = "CHANGELOG.md"
tag_format = "v{version}"
publish = []                         # commands, e.g. ["cargo publish"]
github_release = true

# [[discover.source]]                # optional: feedback and analytics pulled by vsdd discover collect
# name = "github-issues"
# command = "gh issue list --state open --json number,title,body,labels --limit 200"

[state]
push = true
lock_stale_minutes = 20
retain_runs_days = 30
prune_closed_cycles = true

[caps]                               # optional: raise a cap to at most twice its default
# story = 20000
# waived = ["specs/architecture/decisions/ADR-052.md"]   # files over their cap after an import (2.15)

[notify]
channel = "none"                     # none | github-issue | command
command = ""

# [[component]]                      # optional: a repository with more than one toolchain
# name = "web"
# paths = ["web/**"]
# commands = { test = "pnpm -C web test", lint = "pnpm -C web lint" }

# [[reference]]                      # optional: repositories studied during ingestion, cloned under .reference/
# name = "superpowers"
# url = "https://github.com/obra/superpowers"

# [[repo]]                           # hub layout only (2.14); written by vsdd init --hub or vsdd repos split
# name = "api-server"
# path = "repos/api-server"
# url = "git@github.com:acme/api-server.git"
# # optional per repository, with the same keys as the top level: branches, commands, paths, release, delivery

[hooks]                              # owned by the hook design; only a person edits it after init
main_session_wall = true             # decision D6. false relaxes the main session's row only, and is announced

[hooks.rules.auto-allow]
scope = "off"                        # decision D4: off | core | extended. vsdd init writes the answer setup collected
```

Sections this design defines and owns: `schema`, `[project]`, `[branches]`, `[commands]`, `[paths]`,
`[state]`, `[caps]`, `[[component]]`, `[[reference]]`, `[[repo]]`. Sections whose keys and defaults belong
to another design, which this file only hosts and validates:

| Section | Owner | Keys as that design lists them |
|---|---|---|
| `[delivery]`, `[convergence]`, `[holdout]`, `[mutation]`, `[limits]`, `[stops]`, `[evidence]`, `[notify]`, `[maintain]`, `[discover]` | pipeline | As shown above: `delivery.{mode, ci, merge, merge_strategy, protected_paths, dependency_changes, ci_wait_minutes}`; `convergence.{passes, rounds}`; `holdout.{mean_min, must_pass_min, stddev_max, rotation, batch}`; `mutation.kill_rate` by criticality; `limits.*`; `stops.{plan, plan_ceiling_agents}`; `evidence.{wave_demo, final_demo}`; `maintain.{sweeps, fixes_per_run}` |
| `[spikes]` | pipeline and hooks | `per_step`, `per_run`, `per_day`, `minutes`, `retain_days`, `max_open`. `per_day` and `max_open` are enforced by `vsdd spike open`; the time box by the hook runner |
| `[release]`, `[[discover.source]]` | pipeline | As shown above |
| `[hooks]` | hooks | `main_session_wall`, `log_retention_days`, and `[hooks.rules.<rule>]` tables (`hooks.md` 2.6). `vsdd init` and `vsdd import` write the `auto-allow` scope once, from `--auto-allow`; after that no `vsdd` command an agent can run changes it. |

The example above is the whole key list: every section and every key the loader accepts appears in it, with
its default. `pipeline.md` S1 and blueprint 4.12 cite this section and keep no second list, because an
unknown key is an error and two lists would reject each other's keys.

A workflow script cannot read this file. The survey returns the values a run needs in its `config` and
`limits` fields. `vsdd config --json` prints the whole file as data. The survey returns limits under the
names the workflow prelude uses, and a harness test fails on an unknown name:

| Prelude name | Key in this file |
|---|---|
| `agents`, `story_agents`, `stories_per_run` | `limits.agents_per_run`, `limits.agents_per_story`, `limits.stories_per_wave_run` |
| `attempts`, `delivery_rounds` | `limits.step_attempts`, `limits.delivery_rounds` |
| `passes`, `rounds` | `convergence.passes`, `convergence.rounds` |
| `gate_minutes`, `ci_minutes` | `limits.gate_minutes`, `delivery.ci_wait_minutes` |
| `spikes_per_step`, `spikes_per_run`, `spike_minutes` | `spikes.per_step`, `spikes.per_run`, `spikes.minutes` |

`vsdd init` writes the first version by detection (`Cargo.toml`, `package.json` scripts, `pyproject.toml`,
`go.mod`, a `Makefile` or `justfile`, the current branch names, the release configuration, whether there is
a remote and whether there is CI) and then runs each detected command once to see that it starts. It detects
the globs of the seven product classes and lists any tracked file that falls in none. It detects a JUnit
report for the test command (nextest, `pytest --junitxml`, jest-junit, go-junit-report). It checks that
`commands.run`, when set, takes its port from `{port}` or `$VSDD_PORT`. A command it cannot detect is left
empty and listed in the briefing. The `setup` skill shows the detected values to the person, who edits the
file where they are wrong. `--auto-allow <scope>` and `--hub` carry the answers `setup` collected before it
ran `init`, because `init` is the only moment the plugin writes those keys.

Missing commands are a state, not an error. An empty repository has no build or test command to detect, and
a codebase with no tests has no `test_report`. The first pass made that a configuration error, which left a
person to edit `factory.toml` at a moment no briefing named. Now:

- `vsdd status` reports "commands not configured". A gate with no command still never reads as a pass:
  the workflows that measure (`wave`, `story`, `verify`, `maintain`) have "`[commands]` is configured" as a
  precondition and refuse to start.
- The values arrive by approval. The architect writes `specs/architecture/toolchain.toml` in `spec`
  (greenfield) or in the `synthesize` stage of `ingest` (existing code). The `spec` decision approves it with
  the rest of the specification, its packet shows the exact lines, and the first survey after the approval
  writes them into the unset keys. The person typed one decision and edited nothing.
- Where the commands cannot work yet, a chore story makes them work. For an empty repository that is the
  scaffold story, alone in the first wave. For a codebase with no suite or no JUnit report it is a "test
  harness" chore story in `E-0`, scheduled first on every route, fast routes included. Its green gate runs
  the approved commands from its own worktree. For a repository with a remote and no CI, a CI chore story is
  scheduled, and until it merges `delivery.ci = "none"` records that there are no checks to wait for.
- If the approved toolchain file did not supply a command, the route says so and gives the line to add.

### 2.13 What becomes of the 136 templates

A template in the old system was a full sample document that an agent copied, and that a hook then compared
the output against. In 1.0 the registry is the schema and code enforces it, so what remains of a template
is a skeleton: the frontmatter keys with their starting values and the required headings, each with one
line saying what belongs under it. Skeletons live in `<plugin>/templates/<kind>.md`, one per agent-written
kind in section 2.5, about 25 files of at most 40 lines. A lint test checks each skeleton against its
registry entry, so the two cannot drift. Method files cite them by that path (`agents-skills.md`
N-STATE-7), and `vsdd new <kind>` stamps one with the next free identifier.

| What the old template was | Count | In 1.0 |
|---|---|---|
| A sample of an agent-written artifact (brief, domain spec, contract, property, architecture, decision record, UX files, epic, story, scenario, evaluation, research, lesson, deferred item, assessments, conventions, release notes) | 38 | About 25 skeletons, because several old templates described one kind |
| A report format (adversarial, code, security and consistency reviews, findings, drift, fuzz, formal, performance, fidelity, readiness, sweep, session review, and the four UI quality reports) | 25 | One skeleton, `review.md`. The structure of findings is the pipeline design's result schema. |
| An index, a matrix, a trajectory, a manifest, a schedule or a pull request description | 18 | Not stored. Each is a derived view (2.6). |
| A file code now writes (state, wave state, red-gate log, findings tracker, demo evidence report) | 5 | Machine records (2.7, 2.8) |
| A configuration file (policies, merge, autonomy, discovery, project manifest, reference manifest) | 6 | Sections of `factory.toml`, written by `vsdd init` |
| Something copied into the product (design-system starter of 23 files, demo tape, demo browser test, demo CI job, justfile) | 27 | Files under `<plugin>/assets/`: `design-system/`, `demo/` and `scaffold/`. No skill copies them. Method files cite them by path, and the lint rule L-TEMPLATE checks that every cited path exists |
| A prompt or an authoring guide (three adversary prompts, agent and skill authoring templates) | 7 | Method files and contributor documentation, owned by the agents-skills design |
| Bookkeeping with nothing left to describe (burst log, session checkpoints, state-manager checklist, resolved blockers, spec changelog, test-vector supplement, property withdrawal, the two multi-repo state templates, the commit-id currency script) | 10 | Dropped |
| Total | 136 | |

### 2.14 Multi-repo projects

All state for a multi-repo project, shared or not, lives in one state directory, in the hub repository.

- The hub is the git repository whose `factory.toml` has `layout = "hub"` and `[[repo]]` entries. It may be
  a repository made for the purpose or one of the code repositories. Member repositories are cloned at the
  `path` of their entry (by default `repos/<name>/`, excluded from the hub through `.git/info/exclude`).
- Member repositories hold code and nothing from the factory. A session runs from the hub root, and the
  hook runner resolves the project by walking up from the path being touched (`pipeline.md` H7), so work
  inside `repos/api-server/` is governed by the hub's state.
- There is one identifier space for the whole project. A subsystem and a story carry `repo`. A story
  belongs to one repository; work that spans two is two stories joined by `depends_on`. Cross-repository
  contracts are ordinary contracts, and interface definitions sit in `specs/product/supplements/`.
- Each `[[repo]]` entry may carry its own `branches`, `commands`, `paths`, `release` and `delivery`, with the
  same keys as the top level, so the hub release can ship each repository by its own rules.
- The order in which repositories are built is not stored. It follows from story dependencies, as waves do.
- Waves are project-wide, and a wave is built and gated one repository at a time, because a `wave` run
  takes the lock of one repository and branch. So a wave has one closure record per repository that has
  stories in it (`units/wave.<W-NN>.<repo>.json`), each with its own gate rounds, and one integration
  worktree per repository (`.worktrees/_integration/<repo>/`). The wave is closed when every one of those
  records is closed.
- A hub is set up deliberately. `vsdd init` does not guess one. `setup --hub` shows what `vsdd repos scan`
  found and passes the confirmed list to `vsdd init --hub`, which writes the `[[repo]]` entries when it
  creates `factory.toml`. On a project that already has a `factory.toml`, setup shows the block for the
  person to paste. `vsdd repos split` remains the case in which a `spec` decision creates the members.
- On a fresh clone of the hub the member repositories are missing. `vsdd status` reports each one, and
  `vsdd status --fix` clones it from the `url` in its entry.
- A change that crosses repositories is one story per repository, joined by `depends_on`, on every route.

I chose one state directory over the old arrangement (a `.factory/` in every repository plus a
`.factory-project/` above them) because it keeps the single-directory rule the hook and pipeline designs
rely on, and because it is what the user's own nine-repository project already does in practice: its state
sits in one directory with a list of repositories. The cost is that a member repository cloned on its own
has no factory state. That is accepted.

### 2.15 Importing pre-1.0 state

Recommendation to the user: yes, provide an importer (open question 1). The eight projects on this machine
hold real specifications. Read on 2026-11-23: five of them have between 86 and 278 contracts; `prism` has
352 story files, 95 decision records and about 100 hidden scenarios across nine repositories; `pregolya`
and `flowloom` each carry a `STATE.md` of about 300 KB.

Because storage is unchanged, the import is a transformation in place. When `setup` finds pre-1.0 state it
runs `vsdd import --dry-run`, shows the report, and gives the person the command `vsdd import`, which only a
person may run (in their own terminal, through the shim). The import works in the existing `.factory`
worktree on the existing branch:

1. Refuses to start if the old state has uncommitted changes or a story worktree has uncommitted work.
2. Tags the current tip of the state branch `factory-pre-1.0`. That tag is the archive; nothing is deleted
   from history.
3. Rewrites the tree mechanically and commits once as "factory: import to the 1.0 layout".
4. Writes what a 1.0 project needs in order to take a feature. Without these an imported project could not
   start: `feature` would refuse for lack of a baseline, and the route would send a project with hundreds of
   approved contracts back to `ingest`.
   - `factory.toml`, by the same detection as `vsdd init` (and with `--auto-allow`, the answer `setup`
     collected).
   - `cycles/_standing/units/import.json`, a baseline closure record with the integration head and the
     latest old release tag. Its measurements are taken by the first survey that needs them.
   - One `spec` decision record in `cycles/_standing/decisions/`, status `approved`, `settled_by: import`,
     with the digest of the imported specification set. This is what makes the imported contracts the
     approved, locked specification, and it is the only approval the importer writes.
   - One queued change request, `requests/CR-<NNN>.md`, for each epic that had stories in flight, naming
     those stories. The stories are imported as `draft`; the request is what brings them back through
     `feature` and `plan`, so no story is "ready" with no plan or wave behind it.
5. Writes a report: what was converted, what was left out, which files are over their cap, and the requests
   it queued.

`vsdd init` never runs the importer. On pre-1.0 state it reports `legacy` and prints the dry-run command and
the import command. A fixture test holds the whole path: import, then one fast-route `feature`.

| Old content | Import |
|---|---|
| Product brief, domain spec sections, requirements document | Moved under `specs/product/`. Index files dropped. Lists of contracts stripped from the requirements document. |
| Contracts, properties, decision records, subsystem documents | Moved to the new paths. Frontmatter reduced to the registry's keys. Old statuses mapped to the statuses a file may hold: active contracts and properties become `draft` files covered by the imported `spec` decision, which is what makes them approved; deprecated, retired and removed become `retired`; accepted decision records are listed in the imported decision's `accepted`; properties verified with a lock are listed as proven in the import closure. Reverse-link sections and changelogs removed. Identifiers kept. |
| Stories and epics | Moved under `stories/E-<N>/`. A merged story gets a delivery closure in the import record, which is what makes it done. A story in flight is imported as `draft` and named in a queued change request for its epic. An identifier that does not fit `S-N.NN` gets a new one, and the old one goes into `aliases`, so references in code and commit messages still resolve through `vsdd refs`. |
| Hidden scenarios | Moved to `holdout/`. Wave scenarios become ordinary scenarios. |
| Research, ingestion and port analysis | Moved to `research/` and `analysis/`. |
| Technical debt register | Open entries become deferred items with `target: unscheduled`. |
| `STATE.md`, decision logs, burst logs, session checkpoints, lessons, indexes, amendment histories, cycle directories, wave and sprint state, policies, pull request and review archives, demo evidence, logs | Not imported. They remain at the tag. |

A file over its cap is imported as it is and listed under `[caps] waived` with its size. A waived file can
be made smaller but not larger, and `vsdd status` counts them. Bringing one under the cap is ordinary work
for the owning agent: `/vsdd-factory:author conform <path>` for a file that is not locked (a draft story,
research, analysis), and the next amendment run for a file in the specification set, which the imported
`spec` decision locks. A frozen file (an accepted decision record, a done story) stays waived. The import
does not use a model, so it is deterministic, testable against fixtures of the eight real layouts, and has a
`--dry-run` that writes only the report.

A project that prefers a clean start runs `vsdd init --fresh`, which renames the old branch to
`factory-artifacts-rc` and starts a new one.

### 2.16 State commands

All are subcommands of `vsdd`, implemented in `<plugin>/py/vsdd/state/`, `gates/` and `delivery/` on the
shared core. This design owns the whole pipeline-facing tool. Commands marked P have their pass conditions
or behaviour specified in `pipeline.md`; this design fixes their arguments, their output and what they
write. Every command prints one JSON object with `ok` when given `--json`. Exit codes: 0 for success or
"yes", 1 for a negative answer, 2 for a usage or environment error. Outside an initialised project every
command exits 2 except `init`, `import`, `doctor`, `obs`, `hooks selftest` and `--version`. Every command
takes the global option `-C <dir>`, the project directory, so a clerk's command is one segment with no `cd`
in front. The last column of the second table says who may run a command; the hook runner enforces it from
the `[callers]` table in `walls.toml`.

| Command | What it does |
|---|---|
| `vsdd init [--fresh] [--auto-allow <scope>] [--hub]` | Creates or mounts the state branch, writes `factory.toml` by detection (with the D4 answer and, for a hub, the `[[repo]]` entries that `setup` collected), `notes.json`, `.gitignore` and the two exclusion lines. On pre-1.0 state it converts nothing: it reports `legacy` and prints the dry-run command and the import command. `--fresh` needs the person. |
| `vsdd import [--dry-run] [--auto-allow <scope>]` | Section 2.15. `--dry-run` may be run by the `setup` skill; the real import is person-only, like `init --fresh`, because it rewrites the state tree. |
| `vsdd status [--json] [--wave W]` | The status entry point (2.10). Read-only: it computes the report and writes nothing. |
| `vsdd status --fix` | The repairs of 2.10, then a flush. Main session and clerk only. |
| `vsdd survey <workflow> [--subject] [--repo] [--cycle] [--request-uri] [--guidance-uri] [--question-uri]` (P) | Checks that the caller marker for this call exists (the hooks are alive) and the workflow's preconditions; then commits pending state, opens a run, takes the lock, makes worktrees and diff patches, applies the done test, applies an approved toolchain file to unset `[commands]` and `[paths]` keys, and returns the survey object. Free text arrives percent-encoded in the `-uri` options, because a script cannot write a file and a clerk's command takes no standard input. For `feature` with a request it writes the request file and opens a cycle, or leaves the request queued when a cycle is active; with `--amend` it runs inside the open cycle. For `spec` it marks the run as holding an amendment when the latest `spec` decision is approved. For `spike` it stores the question in the run record and takes no lock. |
| `vsdd checkpoint <workflow> --run <id> [--outcome failed]` (P) | Turns the run's step records and evidence into closures and decisions, writes holdout evaluations (with any walled findings), retires research reports a step superseded, stops products the run started, flushes, pushes, releases the lock, and returns `next`. The `release` checkpoint closes the cycle. The `discover` assess checkpoint opens the cycle with the `brief` decision. |
| `vsdd checkpoint brief` | Takes no run. Opens the cycle and the `brief` decision when the person says the draft is ready. |
| `vsdd decide <id> <choice> [key=value ...] [-- note]` (P) | Settles a decision: writes the decision record and nothing else. Refused when called from an agent's tool call. |
| `vsdd cycle close` | Repairs a cycle that did not close; person only. Opening and closing otherwise happen inside the checkpoint and the survey. |
| `vsdd config set <key> <value>` | Changes one key of `factory.toml`; person only. |
| `vsdd driver on`, `pause <decision>`, `off` | Writes `local/driver.json` with the session id, for the `drive` skill. |
| `vsdd plan schedule` (P) | One job: computes waves from dependencies and limits and stores them in the plan's closure record, assigns scenarios, runs the plan gate, records the baseline check, and estimates. |
| `vsdd spike open`, `record`, `close` (P) | The spike ledger; forms below. |
| `vsdd idea register`, `vsdd report debt` (P) | The idea registry; the derived report of waivers, advisory findings and flaky inherited tests, all read from closure records, and the debt a baseline recorded. |
| `vsdd discover collect` (P) | Runs each `[[discover.source]]` command and stores its output under `research/inputs/`. |
| `vsdd defer add`, `list`; `vsdd retire <path>` | The deferred-work register; deletion of one file in a class that allows it (`deferred`, `research`), refused anywhere else (2.4). |
| `vsdd note "<text>"` | Adds a note to `notes.json`. |
| `vsdd lock release [--force]` | Releases the run lock (2.11). `vsdd status` shows the lock. |
| `vsdd validate [--changed] [paths]` | Checks artifacts against the registry: path, identifier, frontmatter, references, caps, the lock, acceptance criteria against contracts. For workflows, CI and `vsdd status`. It also reports register limits (deferred items, on-demand reports), always as warnings with exit 0, so a 41st deferred item never turns a project's CI red. It is the hook design's batch check and `packaging.md`'s `vsdd validate`: one command. |
| `vsdd list <kind>`, `refs <id>`, `coverage`, `path <id>` | Derived views (2.6). `<kind>` is an artifact kind from `artifacts.toml` (`contract`, `story`, `scenario`, `property` and the rest), and an unknown kind is refused for any caller with a wall, as if it revealed every walled class. `list` shows computed statuses. These and `review show` leave walled content out by default; `--include-walled` prints it, and the hook allows that flag only for a caller whose wall permits the class. |
| `vsdd new <kind> ...` | Stamps a skeleton with the next free identifier. |
| `vsdd review prepare <lens> [base]` | For an on-demand review: writes the diff patch and returns its path and the next free report path. |
| `vsdd config --json` | The configuration as data. |

Not built, and why: `vsdd flush` (flush is a function called at the five points of 2.3); `vsdd lock status`
(`vsdd status` shows it); `vsdd stale` (2.6); `vsdd review verdict`, `review diff` and `review path` (a
function, and one `review prepare`); `vsdd defer resolve`, `lesson retire` and `research retire` (one
`retire`); `vsdd job wait` and `job status` (below); `vsdd step record` (the probe it was a fallback for
passed).

**The spike commands.** Every input is an option or a file the runner writes in the temporary directory;
none reads standard input, so each is one command the hook can inspect and approve.

| Command | Arguments | What it does |
|---|---|---|
| `vsdd spike open` | One of: `--run <id> --step <workflow>/<subject>/<name>.<attempt>` (inside a workflow); `--run <id>` (the `spike` workflow); `--question-file <path>` (a dispatch with the Agent tool). Optional: `--name <name>` for a named spike, `--ref <commit>`, `--asker-type <type>` | Takes the runner's agent id and working directory from the caller marker. Takes the question and the asker's agent type from a hook-written record: the stored `needs_spike` step record of the named step; the run record; or the admission the hook recorded for the dispatch. `--asker-type` is a cross-check only, and a mismatch is refused. Refuses when no asker can be established. Returns an earlier finding for the same question in the same unit only if that entry's `read_deny` covers this asker's. Enforces `per_run`, `per_day` and `max_open`. Writes the ledger entry. With `--name`: creates the branch, the worktree and the two ledger files; or, if the named entry is already open, attaches this runner's agent id to it, with no search for an earlier finding and no count against `max_open`. |
| `vsdd spike record` | `--id <ledger id> --json-file <path> [--increment]` | Validates the finding against the schema and completes the entry. `--increment`, for a named spike, records one session's progress without a verdict; the stop check accepts it. |
| `vsdd spike close` | `--name <name> --json-file <path>` | Records the final finding of a named spike and closes the entry. |

**The measuring and delivery commands.** These are the commands `pipeline.md` asked for in its need P1. A
command marked "job" runs its work detached, and the command itself re-attaches. The job is keyed by the
command, its subject and the commit (`story deliver` by the story). The first call starts it; every call
waits for it, up to `--wait <seconds>` (default 100, which is inside the Bash default of two minutes; at
most 540), and returns either the command's ordinary result or `{ok: true, state: "pending", retry: "<the
same command>"}`. The caller runs the same command again. There is no `--start`, no job id to carry from
one tool call to the next, and no second output shape to parse. A call killed by a timeout loses nothing,
and a new session re-attaches to a job an earlier one left running. At most `limits.parallel_gates` jobs run
at once.

| Command | Arguments | Output (schemas in `<plugin>/schemas/`) | Writes | Who may run it |
|---|---|---|---|---|
| `vsdd gate red` (job) | `--story <id> --run <id>`; `--refreeze` | Gate result | Evidence `red.<sha12>.json` with the frozen test identities and hashes, read from the JUnit report. A test counts when its file is in the `tests` class by the precedence of 2.4. With `--refreeze`: a re-freeze record with the new hashes of frozen tests the test-writer changed, written only if each still exists and either passes or fails on an assertion, as at the red gate (the implementer then makes a failing one pass) | `test-writer`, `clerk`; `--refreeze` the `test-writer` only |
| `vsdd gate green` (job) | `--story <id> --run <id>`; `--since <commit>` | Gate result with `diff_patch` | Evidence `green.<sha12>.json`; the diff patch `diff.<sha12>.patch` (from the story's base, or from `--since`); the test output. Frozen tests are compared with the latest red or re-freeze record. On a project with a baseline it fails only on what the baseline does not list | The story's builder, `clerk` |
| `vsdd gate demo` | `--story <id> --run <id>` | Gate result | Evidence `demo.<sha12>.json`; the criterion-to-test table when no recording is required | `demo-recorder`, `clerk` |
| `vsdd gate spec` | `[--full]`; `[--only <part>]` | Gate result whose findings carry `owner` | Evidence | `clerk` for the cross-set check; the owning planner for `--only <part>` after its own fix |
| `vsdd gate plan` | `[--story <id> --run <id>]` | Gate result whose findings carry `owner`; with `--story`, also `unit` when it passes | Evidence. With `--story` and a pass it admits the story: records it in the run record and in a closure (`admitted_by` this run), creates `story/<id>` from the cycle's base and the worktree with the worktree library, under the run's lock. Run again for a story it has admitted, it changes nothing and returns the same unit | `clerk`; the `story-writer` for `--story` |
| `vsdd gate wave`, `release`, `ingest`, `sweep`, `contracts`, `integration` (jobs) | `--wave <id> [--settle]`; `[--only <check>]`; `[--repo <name>]`; `--run <id>` | Gate result; wave gate data for `wave` | Evidence; the integration worktree (per repository in a hub); closure rounds for `wave`; the baseline record for `ingest`. Each runs every check that is due as one job, the checks in parallel inside the tool: `wave` (suite, lint, build, mutation, clone fidelity, UI), `release` (`baseline`, `formal`, `perf`, `ui`), `ingest` (coverage, baseline), `sweep` (audit, benchmarks, clone fidelity, spec coherence, the debt report, collection). A failing check is reported with the others | `clerk` |
| `vsdd feature route` | `--run <id>` | `{ok, route, amendment, stories, readiness}` | The `route` field of the cycle's `cycle.json`, which is tracked, so the route table and the `verify` survey can read it in any clone; the amendment flag in the run record | `architect`, `clerk` |
| `vsdd review show` | `<unit> [--include-walled]` | Rendered findings; a walled finding as walled key and severity unless the flag is given and allowed | Nothing | Any caller, subject to its wall |
| `vsdd story deliver` (job) | `--story <id> --run <id>` | Delivery data with `diff_patch` | The pull request (with the story trailer), the merge with the head-commit match, the `pr.created` and `pr.merged` events, the story's delivery closure, which is what makes the story done | `clerk` |
| `vsdd story new-fix` | `--wave <id>`, `--verify` or `--maintain`, `--round <k>`, `--run <id>` | Units | Fix stories in `E-0` from blocking findings, each admitted with its worktree as `gate plan --story` does. With `--maintain` and no open cycle, a maintenance cycle and its request | `clerk` |
| `vsdd release` (job) | `--run <id>` | `{ok, state, version, tag}` | Idempotent: each run does the next missing part. Prepare: version files and change log, ending with the check that only release files changed. Publish: the tag, the publish commands, the merge back of a hotfix, the new baseline | `clerk` |
| `vsdd env start`, `env stop` | `--subject <id>` | `{ok, url, port, pid}` | A product process on its own port (`{port}` in `commands.run`, or `$VSDD_PORT`), recorded as a job record with the caller's agent id from the caller marker. It is stopped by `env stop`, by the hook runner when that agent stops, and by the checkpoint for anything left | `demo-recorder`, `ui-reviewer`, `holdout-evaluator`, `clerk` |
| `vsdd repos split` | none | `{ok, repos}` | Member repositories; `[[repo]]` entries appended to `factory.toml` | `clerk` |

`vsdd pr-merge`, `vsdd worktree`, `vsdd cache` and `vsdd record-test` do not exist. Merging is a library
function inside `story deliver`. Worktree creation is one library used by `survey` and by the two admission
commands, so there is one merge route and one creator of worktrees. `vsdd gate baseline`, `ui`, `formal`,
`perf` and `audit` are no longer commands of their own: they are checks inside `wave`, `release`, `ingest`
and `sweep`, selectable with `--only`. The gate names are eleven: `red`, `green`, `demo`, `spec`, `plan`,
`wave`, `release`, `ingest`, `contracts`, `integration`, `sweep`.

### 2.17 Where this design departs from the readings in `rewrite/STATE.md`

- STATE.md says a pre-compaction hook "flushes anything pending". I kept the hook call and made it nearly
  empty: state is committed at the five flush points of section 2.3 (the start of a survey, the checkpoint,
  a delivery, `status --fix`, and this hook), so compaction is not the moment anything depends on.
- It reads the spike ledger as `.spike/STATE.md` and `.spike/TASKS.md`. I kept those two files for named,
  human-led spikes, where they are the user's convention, and added one project ledger outside any spike
  worktree, because a ledger that lives in a throwaway worktree cannot stop a question being asked twice.
- It says kept spike worktrees "need a retention rule". The rule is: the ledger entry lists the worktree
  and branch; `vsdd checkpoint` and `vsdd status --fix` remove spike worktrees older than
  `spikes.retain_days` (14) and those whose asking unit is delivered; the branch `spike/<name>` of a named
  spike is kept until the spike is closed.
- It says "state files have size caps". Every artifact kind and every machine record has one, and the
  cap is checked before the write instead of being repaired afterwards.
- It says the briefing reports "project initialised or not". Following the hook design, a project that is
  not a factory project gets nothing at all, and a project with pre-1.0 state gets one line.

### 2.18 Seams with the sibling designs that need settling

| With | Their assumption | This design |
|---|---|---|
| hooks | `find-root.sh` tests a directory for `.factory/factory.toml` before checking whether it is a linked worktree | Unaffected, because a linked worktree never contains `.factory/`. No change is needed if storage stays as chosen here. |
| hooks | A story's `wave` is a frontmatter key, and stories match `.factory/stories/*.md` | The wave is in the plan's closure record; stories are under `stories/E-<N>/`. The example table in `hooks.md` 2.11.3 changes; the rule does not. |
| hooks | An earlier draft tested conditions such as `phases.<name>.status` in the status record and read `evidence/<story>/red.json` | There is no status record. If any such test remains, unit and wave state are read from the closure records (or from `vsdd.state.status`), a settled decision is read from its decision record, and evidence is one record per gate and commit at `local/evidence/<subject>/<gate>.<sha12>.json`, reached through the `evidence` function. |
| agents-skills | `evidence` is at `.factory/evidence/**`; `holdout` at `.factory/holdout/**` | `evidence` is `.factory/local/evidence/**`, untracked. `holdout` is as assumed. |
| agents-skills | The story-writer writes a wave plan | It writes `depends_on` and `priority`. `vsdd plan schedule` computes the waves. |
| agents-skills | The spike runner writes its ledger entry | It does, by running `vsdd spike open` and `vsdd spike record`, not with the Write tool. |
| pipeline | The tool is named `factory` | `vsdd`, per the packaging design. |
| pipeline | Step records decide whether a stage is done | Closures decide across sessions and machines; step records decide only inside a run (2.8). |
| packaging | `vsdd stale` reads stored input hashes | There is no `vsdd stale` and there are no stored input hashes (2.6). `vsdd lock release` operates the run lock. Templates are in `<plugin>/templates/`, and scaffold files in `<plugin>/assets/`. There is no research cache: the dated reports under `research/` are what a researcher reads first. |

## 3. Mapping from the old system

"Replaced" means the capability is kept and the mechanism is new. Each dropped item has its reason.

**Storage and protocol.**

| Old item | Fate | In 1.0 |
|---|---|---|
| Orphan branch `factory-artifacts`, mounted as a worktree at `.factory/` | Ported | Same branch, same mount; created, mounted and repaired by code (2.3). |
| `.gitignore` entry for `.factory/` | Replaced | `.git/info/exclude`. |
| Commit protocol: only the state-manager commits, `factory(<phase>): ...` messages, one commit per burst | Replaced | Flush. |
| `factory-cas-push.sh` | Replaced | The push inside `vsdd checkpoint`. |
| Factory lock: lease in `STATE.md`, `factory-lock` and `factory-lock-parse` crates, four `factory-lock-*` scripts, `factory-lock` and `factory-unlock` skills, two `verify-factory-lock` hooks | Replaced | The local run lock (2.11). |
| Hooks `factory-branch-guard`, `validate-factory-path-root`, `validate-factory-path-staging` and `-staged`, `check-factory-commit` | Replaced | The mount precondition, and the hook design's `write-scopes` check 5. |
| CI step that mounts the branch | Dropped | Nothing in the plugin's CI needs it. A project may add it. |
| `.factory-project/` on `factory-project-artifacts` for multi-repo | Replaced | The hub's `.factory/` (2.14). |
| Lifecycle classes: living, accumulating, cycle-scoped | Merged | Living artifacts, and cycle records that are pruned at close. |
| `current-cycle` file | Replaced | Derived: the cycle whose `cycle.json` has no close date. |
| `.factory/logs/` (2.6 GB in one project) | Replaced | `.factory/local/events/`, pruned by the hook runner. |
| `research-cache/` | Dropped | Research reports under `research/` are the cache: the researcher reads earlier reports first. A query-level cache needs a shell, which no researching agent has. |

**`STATE.md` and the bookkeeping files.**

| Old item | Fate | In 1.0 |
|---|---|---|
| `STATE.md` (231 KB against a 200-line target) | Replaced | The status report, computed from the records each time and never stored (2.7). |
| Its sections: project metadata; phase progress; current phase steps; artifact counts; story status; active branches; concurrent cycles; skip log; blocking issues; session resume checkpoint | Replaced | In order: `factory.toml`; `cycle.units`; step records; derived; closure records; derived from git; `others`; verdict waivers; decisions; the briefing. |
| Its decisions log, and `decision-log.md` (D-001 to D-1269, over 4 MB with archives) | Dropped | Architectural decisions are decision records. A person's decisions are decision records with packets. The rest narrated bookkeeping that no longer exists. |
| `burst-log.md`, `session-checkpoints.md`, `SESSION-HANDOFF.md`, `HANDOFF.md`, phase-progress and state archives, amendment-history sidecars, changelog archives | Dropped | They recorded how the state files were edited. Git records that. |
| `lessons.md` per cycle, `sidecar-learning.md` | Replaced | The retrospective report's proposals, adopted into `conventions.md` or `review-rules.md` through `/vsdd-factory:author`. |
| `tech-debt-register.md` | Replaced | `deferred/` for what a person deferred; `vsdd report debt` for waivers and advisories. |
| `cost-summary.md` | Dropped | The file was never written on Claude Code. Cost is shown by the observability dashboards (packaging design). |
| Cycle `INDEX.md`, `convergence-report.md`, convergence trajectory, cycle manifest | Replaced | Derived from closure records; the close fields of `cycle.json`. |
| `policies.yaml`, `policy-add`, `policy-registry` | Replaced | Mechanical policies are registry checks. Project review rules are `review-rules.md`. |
| `planning/` (16 registered file types), `plans/`, `proposals/`, `feature-delta/`, `po-obligations.md`, `open-questions.md`, `legacy-design-docs/`, `measurements/` | Merged | Assessments, research, the change request file (`requests/CR-<NNN>.md`), decision packets. Validation and routing reports are step results now. |
| `phase-0-ingestion/`, `semport/`, `reference-manifest.yaml` | Merged | `analysis/`, and `[[reference]]` in `factory.toml`. |
| `code-delivery/<story>/` (pull request descriptions and review rounds) | Replaced | Review reports, step records, and a derived pull request description. |
| `demo-evidence/`, and `docs/demo-evidence/` in the product | Replaced | `demo/`, with caps and no committed video. |
| `dtu-clones/` | Replaced | Clone code is product code. The clone specification is an assessment. |
| `holdout-scenarios/wave-scenarios/`, `holdout-evaluations/` | Merged | `holdout/` and `holdout/evaluations/`. |
| `wave-state.yaml`, `sprint-state.yaml` | Replaced | Closure records and `cycle.waves`. |
| `regression-state.json`, `adversary-convergence-state.json`, `scenario-selection.json` | Replaced | Evidence records, the verdict in a closure, the seed in an evaluation. |
| `unvalidated-mutation.marker` | Dropped | It covered a failure mode of the old dispatcher. Validation now runs before the write. |
| `shard-config.toml` and shard rotation | Dropped | Rotation managed files that outgrew a budget. Caps stop them growing. |

**Indexes and stored views.**

| Old item | Fate | In 1.0 |
|---|---|---|
| `BC-INDEX.md`, `VP-INDEX.md`, `STORY-INDEX.md`, `ARCH-INDEX.md`, `HS-INDEX.md`, `L2-INDEX.md`, `UX-INDEX.md`, `RESEARCH-INDEX.md`, `EVAL-INDEX.md`, review indexes, `epics.md` | Replaced | `vsdd list`. |
| `dependency-graph.md`, `wave-schedule.md` | Replaced | Derived; the schedule is stored once by code. |
| Traceability matrices, `verification-coverage-matrix.md`, the catalogue in `verification-architecture.md`, `ui-traceability.yaml` | Replaced | `vsdd coverage`. |
| `bc-id-mapping.md`, `legacy-id-mapping.md` | Replaced | The `aliases` key. |
| `input-hash`, `compute-input-hash`, `check-input-drift` | Replaced | The decision digest, which makes an approval stale when the approved content changes, and explicit reopening of stages (2.6). No hash is stored per artifact or per stage. |

**Conventions.**

| Old item | Fate | In 1.0 |
|---|---|---|
| Canonical frontmatter: `document_type`, `level`, `version`, `status`, `producer`, `timestamp`, `phase`, `inputs`, `input-hash`, `traces_to`, `pass`, `previous_review` | Replaced | `id`, `title`, `status` and typed links. Type and level follow from the path; producer and time from git; pass from the file name. |
| Lifecycle block: `introduced`, `modified`, `deprecated`, `deprecated_by`, `replacement`, `retired`, `removed`, `removal_reason` | Replaced | `status` and `replaced_by`. The rest is git history. |
| Version keys, version citations, the spec versioning strategy, `spec-changelog` | Dropped | A version an agent bumps by hand was the largest single source of the old bookkeeping. Git has the history, and digests record what was built against what. |
| The 30 identifier formats | Ported, 20 of them | Dropped, 10: wave scenario ids (merged into scenarios); the four finding and fix formats `ADV-`, `CR-`, `SEC-`, `FIX-` (finding keys and fix stories); `TD-` (slugs); `EVAL-` (the path); `GAP-` and `EAC-` (deferred items; epic text); `AD-` (a duplicate of `ADR-`). The three UI labels local to a screen (`CMP-`, `ELM-`, `INT-`) stay as plain labels inside the screen file and are not checked. Old forms `STORY-NNN` and `EPIC-NNN` import as aliases. New in 1.0: `ENT-`, `EVT-`, `FLOW-`, `ID-`, `SP-`, `W-`. |
| Iterative naming `-pass[N]`, `-final` | Ported | `<lens>-<n>.md`. There is no `-final`. |
| Sharding: an index file plus detail files | Ported without the index | One file per item, in directories derived from identifiers. |
| Append-only numbering; numbering that continues across cycles | Ported | Enforced at the write. |
| Single source of truth rule and the propagation policies (`bc_array_changes_propagate`, `bc_h1_is_title_source_of_truth`, `architecture_is_subsystem_name_source_of_truth`, `vp_index_is_vp_catalog_source_of_truth`) | Replaced | Nothing is stored twice, so nothing propagates. The check that a story's criteria and its contract list agree is a registry check. |
| `lift_invariants_to_bcs` | Ported | `vsdd coverage` lists invariants no contract cites. |
| `semantic_anchoring_integrity`, `creators_justify_anchors` | Ported as review | Whether a link is the right link is a judgment, so it belongs to the adversary's spec review (agents-skills design). |
| Contract retirement checklist (five places) | Replaced | Set `status: retired` and `replaced_by`. `vsdd validate` reports stories that still cite it. |
| "Immutable once green" for properties and contracts | Ported | The lock (`spec_locked`, `artifact_locked`), read from the decision and closure records and enforced before the write. The old hooks printed a denial and exited 0. |
| `artifact-path-registry.yaml` (477 lines) | Merged | The `glob` column of the registry. |
| Artifact path migration table of 2026 | Dropped | Historical. |
| Priority scale P0 to P2 | Ported | The story key `priority`. |
| Agent permission model (who writes, who commits) | Replaced | Path classes; code commits. |

**Skills, tools, hooks and agents that worked on state.** The other designs hold the authoritative
mappings for these; this table says where the state-facing function went.

| Old item | Fate | In 1.0 |
|---|---|---|
| Skills `wrap`, `wave-handoff`, `wave-reset`, `rehydrate-wave`, `recover-state`, `compact-state` | Replaced | `status` (2.10). |
| Skills `state-update`, `state-burst`; the state-manager agent | Replaced | `vsdd` commands and flush. |
| Skills `check-state-health`, `factory-health`, `factory-worktree-health` | Replaced | `vsdd status --fix`, `vsdd doctor`. |
| Skill `factory-cycles-bootstrap` | Dropped | It migrated a layout older than the one being replaced. |
| Skills `next-step`, `wave-status`, `factory-dashboard`; tools `wave-state`, `factory-dashboard` | Replaced | `next` in the computed status; `vsdd status`. |
| Skill `register-artifact` | Dropped | There is no index to register in. |
| Skills `relocate-artifact`, `conform-to-template`, `validate-template-compliance` | Replaced | Validation before the write; `vsdd validate`; `author conform`. |
| Skill `track-debt` | Replaced | The `defer` skill. |
| Skill `spec-versioning`; the spec-steward agent | Dropped | Versions are not stored. |
| Hooks that policed the old formats: `validate-state-size`, `validate-state-structure`, `validate-state-pin-freshness`, `validate-state-index-status-coherence`, `validate-index-cite-refresh`, `validate-index-self-reference`, `validate-count-propagation`, `validate-changelog-monotonicity`, `validate-burst-log`, `validate-closes-completeness`, `validate-dispatch-advance`, `validate-cross-site-correspondence`, `validate-input-hash`, `validate-wave-handoff-completeness`, `stamp-state-timestamp`, `verify-state-timestamp-advisory` | Dropped | The formats they checked no longer exist. |
| Hooks with a rule that survives: `validate-artifact-path`, `validate-template-compliance`, `validate-bc-title`, `validate-story-bc-sync`, `validate-subsystem-names`, `validate-vp-consistency`, `validate-finding-format`, `protect-bc`, `protect-vp` | Merged | Registry checks run by the hook rule `artifacts`. |
| `precompact-flush`, `postcompact-reanchor`, `session-start-telemetry`, `warn-pending-wave-gate`, `update-wave-state-on-merge` | Replaced | Flush on `PreCompact`; the briefing; `vsdd story deliver`. |
| The mechanical checks of the consistency-validator agent (about 80 criteria) | Replaced | `vsdd validate` and `vsdd coverage`. |
| The 136 templates | Replaced | Section 2.13. |

## 4. Interfaces I provide

**To every design.**

| # | Interface |
|---|---|
| 1 | The state directory: `.factory/`, a worktree of branch `factory-artifacts`, one per project, never present inside a linked worktree; the layout of 2.3; the untracked `.factory/local/`. |
| 2 | `vsdd.state.detect(root)` and the five conditions of 2.3. |
| 3 | The registry content (2.4, 2.5), the path classes (2.4), the identifier schemes, and the flat frontmatter subset. |
| 4 | The record formats: the status object (2.7), which is computed and not stored; closure, decision, run, step, evidence, job and driver stores (2.8); the run lock (2.11). |
| 5 | `factory.toml`: its path, its schema, the sections it hosts for other designs (2.12). |
| 6 | The state commands of 2.16, and these functions in `<plugin>/py/vsdd/state/`, each returning within one second except `flush`: `detect`, `status`, `briefing`, `flush`, `digest`, `verdict`, `caller`, `lock_seen`, `lock_release`, and the functions the hook design names in its S7 (`store_step`, `settle_decision`, `evidence`, `spike_entry`, `spike_admit`, `story_kind`, `spec_locked`, `artifact_locked`, `unit_tree`). |
| 7 | Skeletons in `<plugin>/templates/<kind>.md`. |

**Answers to the needs the sibling designs stated.**

| Their need | Answer |
|---|---|
| `agents-skills.md` N-STATE-1 | The class table in 2.4; configuration keys `paths.tests`, `paths.harness`, `paths.ci`, `paths.repo_config`, `paths.manifests`, `paths.docs`, `paths.source`, with the precedence rule. |
| N-STATE-2 | One state directory at `<project root>/.factory`, the same absolute path from every worktree, and no copy inside a story worktree. |
| N-STATE-3 | On-demand reviews: `reviews/<yyyy-mm-dd>-<lens>-<n>.md`, with `vsdd review prepare <lens>`. Pipeline reviews write no report. Evaluations are `holdout/evaluations/<cycle>/<unit>-<n>.json`, written by code. |
| N-STATE-4 | Spike ledger: `spikes/SP-<NNN>.md`, written through `vsdd spike`. Deferred items: the register table in 2.5. There is no lessons register; a retrospective is a research report. |
| N-STATE-5 | `.factory/specs/architecture/review-rules.md`. |
| N-STATE-6 | Confirmed. No artifact asks an agent to keep an index, a count or a version citation. Evidence and diffs are written by code, and the status report is computed. |
| N-STATE-7 | `status` behaviour: 2.10. A project is initialised when `vsdd init` has run: the mount, `factory.toml` with `schema = 1`, `.gitignore`, the two exclusion lines. Templates: `<plugin>/templates/<kind>.md`. |
| N-STATE-8 | The state directory is excluded from the product checkout, so search tools that honour git exclusions do not descend into it from the project root. Story worktrees do not contain it. Direct paths are covered by the hook design's read walls. |
| `hooks.md` S1 | `.factory/factory.toml`, present if and only if initialised, integer `schema`. Pre-1.0 test: `.factory/STATE.md` without `factory.toml`. |
| S2 | `.factory/local/`, with `hooks/` and `events/` for the runner. Classes `local`, `runs`, `evidence`, all `write = "code"`. |
| S3 | `paths.tests`, `paths.harness`, `paths.ci`, `paths.repo_config`, `paths.manifests`, `paths.docs`, `paths.source`, resolved by `vsdd.project.path_class(path)` with the precedence of 2.4; `branches.integration`, `branches.release`, `branches.protected`; the spike time box as `spikes.minutes`; `commands.test`, with `[[component]]` selecting a different command by the paths a story touches. The event log goes to the fixed path `.factory/local/events/`; there is no key for it. |
| S4 | `vsdd.state.briefing`, which renders the status object of 2.7 computed from the records; 1,200 characters. |
| S5 | Section 2.5 as the one copy of `artifacts.toml`. Indexes, counts and input hashes are derived. |
| S6 | The class table, including `runs` (the step-record store), `evidence`, `decisions` and `spikes`. Evidence is not committed. N-STATE-8 as above: the whole state directory is excluded from the product checkout, which is the first of the three options in `hooks.md` 2.11.2 if the Grep tool honours git exclusions, and that must be confirmed by a recorded run (section 7, risk 10). |
| S7 | These functions, which are the list both designs build to. `store_step(project, result, agent_type, head)` writes the step record at the path of 2.8 with the calling agent's type and the worktree head, refuses one over its cap, and writes the full form of any walled finding to the evaluation file. `settle_decision(project, id, choice, values, note, session_id)` writes the decision record and nothing else (and, for `spec-gap amend`, the amendment request); it raises with a reason when the decision is not open, the id is ambiguous, the choice is not one of its kind's, or the digest of what is being approved no longer matches. `evidence(project, gate, subject)` returns the passing record by the per-gate rule of section 2.8 (for `red`, the latest passing or re-freeze record on any commit of the story branch since its base), or nothing. `spike_entry(project, agent_id, worktree)` returns the open ledger entry for that runner, matched by agent id first, with `opened`, `verdict`, `asker_type` and `read_deny`. `spike_admit(project, session_id, dispatcher_type)` records an admission, and `spike_attach(project, agent_id, cwd)` pairs a starting runner with it. `story_kind(project, story)` reads the story's `kind`, and `kind_has_red_gate(kind)` reads the story-kind table. `spec_locked(project)` is true when the latest `spec` decision is approved and no live run record holds an amendment; it takes no path, because the lock covers the whole specification set. `artifact_locked(project, path)` is true for an accepted decision record, a proven property and a done story. `guidance_locked(project)` is true while a cycle is open and no live run holds an amendment; it governs `conventions.md` and `review-rules.md`. `unit_tree(project, workflow, subject)` returns the root that a unit's finding paths are relative to. `caller(key)` reads the caller marker for a `vsdd` call. `lock_seen` and `lock_release` serve the hook runner's liveness duty. The story's wave is in `cycles/<cycle>/units/plan.json`, and a wave's gate status is in its closure record, `units/wave.<W-NN>.json`. |
| S8 | `vsdd.state.flush(root, "precompact")`. Idempotent, no network, within 20 seconds. |
| S9 | A single instance at the project root, on a separate branch named `factory-artifacts`, mounted as a worktree. Old entries 39, 49 and 70 become the mount precondition. |
| `pipeline.md` S1 | The sections hosted in `factory.toml` (2.12). Two names differ: `branches.protected` is here and `delivery.protected_paths` is theirs. |
| S2, S7 | `vsdd survey` and `vsdd checkpoint` live in the state package and use the stores of 2.8 and the refinement of the done test stated there. |
| S3, S4, S5 | Step, evidence and decision stores: paths, caps and tracking in 2.8. |
| S6 | The spike ledger entry of 2.5, and `.spike/STATE.md` and `.spike/TASKS.md` for named spikes. |
| S8 | The run lock of 2.11. |
| S9 | `vsdd status --json` and the briefing of 2.9, including `next`, open decisions, named spikes and the driver flag. |
| S10 | The story frontmatter keys of 2.5, with the wave in the plan's closure record. |
| S11 | `demo/` on the state branch for scripts, transcripts and small screenshots; rendered media in `.factory/local/media/`. Nothing on a product branch. |
| `packaging.md` S1 | `vsdd.state.detect`. |
| S2 | `.factory/factory.toml`; test and build commands under `[commands]`; base branch `branches.integration`; merge strategy `delivery.merge_strategy`; the event log at the fixed path `.factory/local/events/`. |
| S3 | The registry as data. There is no staleness report (2.6). |
| S4 | The run lock; `vsdd lock release` operates it and `vsdd status` shows it. |
| S5 | `vsdd.state.briefing`. |
| S6 | State is pushed only by `vsdd checkpoint`. No separate push tool. |
| S7 | Yes, an importer. After `cutover.py apply`, the person runs `/vsdd-factory:setup`. |
| S8 | `<plugin>/templates/`. There is no research cache. |

## 5. Interfaces I need

**From the hooks design.**

| # | Need | A mismatch shows as |
|---|---|---|
| H-1 | Withdrawn by the blueprint. Flush runs on `PreCompact` and inside `vsdd` commands only. | |
| H-2 | On any guarded tool call from a session that holds the run lock, or from a working directory inside a worktree the locked run owns, update `seen` in `local/lock.json` at most once a minute (through `vsdd.state.lock_seen`). Before letting a `vsdd` call through, write the caller marker for that call and rewrite the command to carry its key (2.11). On `SessionEnd`, release any lock that session holds (`vsdd.state.lock_release`). On `SubagentStop`, stop a product that agent started and left running. | Without the first, a long step with no `vsdd` call makes the lock look stale and a second session could take over a live run. Without the marker bound to the call, the lock can be recorded under the wrong session. Without the third, a closed session blocks the next one for 20 minutes. |
| H-3 | The `artifacts` rule honours the registry column `dir` (the parent directory derived from the identifier) and the lock (`spec_locked`, `artifact_locked`). When an identifier is already taken, the block message names the next free one. The frontmatter status list of each kind no longer contains a value that only code may set, so there is nothing for a `code_values` check to do. | An agent without a shell could not find a free identifier; an architect could edit the approved architecture during a build. |
| H-4 | A state write is blocked when the condition is `partial` (the mount test fails), with the message naming `/vsdd-factory:status --fix`. | State could be written into a directory that is not on the state branch. |
| H-5 | A read or write of any `.factory` path inside a linked worktree is blocked with the correct path (check 5 already does this for writes). | An agent could read a stray copy. |
| H-6 | Step records are written only through `store_step`, so the path of 2.8 and the cap have one owner. | The survey would not find them. |
| H-7 | The person's `/vsdd-factory:decide` is recorded only through `settle_decision`, which writes the decision record and nothing else. The briefing is computed from that record, so a settled decision cannot be listed as open. | A second writer could leave a record the survey does not understand. |
| H-9 | The hook stores the calling agent's type with every step record, records a spike admission when an agent dispatches the spike runner, and pairs it with the runner when it starts. | The tool could not check who wrote a review pass, or which walls a spike runner inherits. |
| H-8 | The `[hooks]` table, `hooks/` and `events/` under `.factory/local/` are the runner's. This design does not read or write them, apart from pruning by `vsdd status --fix` if the hook design asks for it. | |

**From the pipeline design.**

| # | Need | A mismatch shows as |
|---|---|---|
| P-1 | Withdrawn. There is no stage registry: each step result lists the artifacts it wrote, and the done test checks those. The scripts report stage names and the tool records them. | |
| P-2 | The route table and the preconditions as specified rows (`pipeline.md` 2.4 and 2.5), implemented once in this package as ordered lists beside their condition functions, so `vsdd status`, `vsdd survey` and `vsdd checkpoint` compute `next` and "could run" with one implementation. | The briefing and a workflow's result would name different commands, or the route would name a command the survey refuses. |
| P-3 | Acceptance of the refinement in 2.8: closures decide whether a stage is done across sessions; a missing evidence record for a closed gate is measured again by the survey. | A fresh clone would redo finished stages. |
| P-4 | Every workflow ends with `vsdd checkpoint`, including on failure paths, so the lock is released and state is pushed. | A dead lock for 20 minutes; unpushed state. |
| P-5 | Story worktrees are worktrees of product branches made by `vsdd survey`, or by the two admission commands (`vsdd gate plan --story`, `vsdd story new-fix`) through the same worktree library, under `.worktrees/`, with branch names matching `branches.story`. No agent and no script makes one. | The hook runner could not map a path to a story, and the exclusion lines would not cover the directory. |
| P-6 | One active cycle at a time, with a critical fix as the only second one. | The status object has room for one `cycle` and short `others` lines, and decision identifiers need qualifying only then. |
| P-7 | The schemas for step results, verdicts and decisions in `<plugin>/schemas/` stay within the caps of 2.8 (a step record of 64 KB, a packet of 12 KB). | A write is refused. |
| P-8 | Settled by the blueprint: `vsdd checkpoint brief`, the `discover` assess checkpoint, the `feature` survey and `vsdd story new-fix --maintain` open a cycle, and the `release` checkpoint closes it. Work outside a cycle uses `cycles/_standing/`. | Demos and decisions would have no cycle directory to go under. |

**From the agents-skills design.**

| # | Need | A mismatch shows as |
|---|---|---|
| A-1 | The `status` skill's body is: run `vsdd status` with the given arguments and relay the output; when a person states a standing instruction, save it with `vsdd note`. | |
| A-2 | The `setup` skill runs `vsdd init`, shows the detected configuration, and runs `vsdd doctor`. It is also the command a person runs on a fresh clone and after the cutover tool. | The packaging design's cutover step 4 would print the wrong command. |
| A-3 | Method files tell agents that write artifacts to start from `${CLAUDE_PLUGIN_ROOT}/templates/<kind>.md`, to give new files `status: draft`, and never to write an index, a count or a reverse link. | Blocks at the write. |
| A-4 | The `spike-runner` writes its ledger entry with `vsdd spike open` and `vsdd spike record`. | Two ledger formats. |
| A-5 | The retrospective method writes one research report of kind `retrospective`, with its proposals, and returns the paths of earlier reports it supersedes. There is no `lessons/` directory. | Blocks at the write. |
| A-7 | The four planners have a `vsdd`-only shell, so that the architect can end with `vsdd feature route`, the story-writer with `vsdd gate plan --story`, and a planner's fix step with `vsdd gate spec --only <part>`. | Each of those would need a clerk step. |
| A-6 | The walls table uses the class names of 2.4, including `runs` (reviewers may not read it) and `holdout-evaluations`. | A wall that matches no path. |

**From the packaging design.**

| # | Need | A mismatch shows as |
|---|---|---|
| K-1 | `<plugin>/py/vsdd/state/` and `<plugin>/py/vsdd/policy/artifacts.toml` and `classes.toml` on the shared core, importable by the hook runner in process. Python 3.11 or later for `tomllib`. | The briefing would need a subprocess. |
| K-2 | Test fixtures: a small state tree per condition of 2.3, and the eight real pre-1.0 layouts reduced to fixtures for the importer. | The importer cannot be tested. |
| K-3 | The cutover tool leaves the old `.factory` worktree mounted, and prints `/vsdd-factory:setup` as the next step. | The import would have nothing to transform. |
| K-4 | A documented two-line recipe for a project's own CI to mount the state branch and run `vsdd validate`. | |
| K-5 | The lint test that compares each skeleton in `<plugin>/templates/` with its registry entry. | Skeletons drift from the schema. |

## 6. Open questions for the user

These questions are carried into section 2 of the blueprint as decisions D3, D1, D2, D22 and D23.

1. **Should projects on the old plugin get an importer?** Recommendation: yes. It is mechanical, it works in
   place on the existing state branch after tagging the old tip `factory-pre-1.0`, it keeps contract, story
   and decision identifiers, and it leaves `STATE.md`, the logs and the indexes behind at the tag. Without
   it, eight projects with up to 278 contracts and 352 stories either stay on rc.25 or start again from
   nothing. If you would rather start some of them clean, `vsdd init --fresh` does that per project.

2. **Keep state on its own branch?** This design keeps the separate branch and removes the chores around
   it. The alternative is a tracked directory on the integration branch: no mount on a fresh clone, and
   specifications visible beside the code, at the price of state commits in the product history, hidden
   scenarios in the product tree and in packages built from it, and a checkout that must stay on the
   integration branch. Recommendation: keep the separate branch.

3. **May the plugin commit and push state on its own?** Flush commits to the state branch without asking,
   and `vsdd checkpoint` pushes that branch when the repository has a remote. Neither touches a product
   branch. State commits skip the product's git hooks and are not signed, because those hooks are written
   for product code. `rewrite/STATE.md` lists "commit or push only when asked" as an unconfirmed rule;
   applied to the state branch it would put a person back into every step. Recommendation: allow both for
   the state branch only, with `state.push = false` for a project that wants pushes by hand.

4. **Is text enough as demo evidence?** As designed, the demo script and its text transcript are committed,
   screenshots up to 300 KB each are allowed, and video or GIF renderings stay on the local machine. The
   old repository carries 110 MB of committed recordings. Recommendation: yes. If you want recordings
   attached to pull requests, that is an upload step for the pipeline design, not a reason to commit them.

5. **Remove review and demo detail from the tree when a cycle closes?** The files stay in the state
   branch's history at a commit recorded in the cycle summary; the working tree keeps full detail for the
   open cycle only. Recommendation: yes. A project with audit duties sets `state.prune_closed_cycles =
   false`.

## 7. Risks

1. **The mount is one more thing that can break.** A deleted directory, a pruned worktree or a person
   checking out another branch inside `.factory/` leaves the project `partial`. Mitigation: the mount test
   is five file reads and runs on every hook call and every `vsdd` command; in that condition state writes
   are blocked instead of landing in the wrong place; `vsdd status --fix` remounts and keeps any
   uncommitted files.

2. **A fresh clone is silent until `setup` runs.** With no `.factory/`, the hooks are inert, so nothing
   announces that the project is a factory project. No pipeline work can start there, because every
   workflow's survey fails, but direct agent work would be ungated. Mitigation: `vsdd doctor` and `setup`
   detect the unmounted state branch; a one-line briefing hint is possible at the cost of the hook design's
   "silent when not initialised" rule, and is left to that design's open question 6.

3. **Specification and code have separate histories.** Nobody can check out one commit and get both.
   Mitigation: each state commit carries the integration head as a trailer, and closure records carry the
   product commit each gate passed on. This is recorded once and never updated, so it cannot become the
   citation bookkeeping of the old system.

4. **Resume across machines is only as good as the last checkpoint.** Step records and evidence are local.
   A run that dies before its checkpoint is repeated on another machine, and gates are measured again.
   Mitigation: every workflow ends in a checkpoint, a wave run checkpoints per wave, and repeating is safe
   because the survey starts from disk. If this proves costly, the fix is an extra checkpoint inside long
   workflows, not tracking step records.

5. **The run lock depends on liveness signals.** If the hook runner does not refresh `seen` (need H-2), a
   quiet 20 minutes inside one long tool call makes a live run look dead. Mitigation: the default is
   longer than the longest foreground command Claude Code allows; the take-over is logged; and a second run
   on the same story would still be stopped at delivery by the evidence check.

6. **Caps will sometimes block legitimate content.** A contract with a large table of test vectors or an
   architecture overview of a large system can exceed its cap. Mitigation: the message says how to split;
   a project may raise a cap to twice its default, visibly; imported files over the cap are waived by
   name. The numbers should be checked against the end-to-end proof and the first import.

7. **The importer meets eight different layouts.** The projects drifted: `prism` has flat contract files
   with slugs and free-form story identifiers, others have subsystem directories whose names do not follow
   from the identifiers. Mitigation: a dry run that only reports; fixtures cut from the real projects; a
   tag before anything changes; aliases for identifiers that cannot be kept.

8. **Records are only as true as the commands that write them.** A closure says a gate passed because
   `vsdd` measured it, which is sound. A verdict says findings were resolved because a reviewer's
   structured result said so, which is judgment. Mitigation: that line is the pipeline design's rule
   ("scripts steer, tools measure, hooks enforce"); this design keeps the two kinds of fact in different
   stores, so a reader can tell which is which.

9. **A shared tool, four owners.** The survey, the checkpoint and the stores are specified here and in
   `pipeline.md`; validation is specified here and in `hooks.md`. If the two descriptions of one command
   differ, builders will implement two. Mitigation: section 2.18 lists every difference found while
   writing, and the build plan should give the state package and the policy tables one owner.

10. **Platform behaviour this design relies on and has not seen.** Each needs a recorded run before the
    build depends on it.

    | Assumption | If it fails |
    |---|---|
    | A hook may run `git` in the state worktree within its time limit on `PreCompact`, and in the background on `SubagentStop`. | Flush runs only inside `vsdd` commands, which already covers every step boundary. |
    | The Grep tool honours `.git/info/exclude`. | The state design adds a `.ignore` file at the project root naming `.factory/`, which the hook design already anticipated. |
    | Agents can read files under `.factory/` by absolute path from a story worktree without a permission prompt. | The hook design's `auto-allow` list covers the state directory. |
    | `git worktree` commands by Claude Code's own isolation do not prune or disturb a worktree mounted at `.factory`. | `vsdd status --fix` remounts; the mount test catches it at the next call. |
