---
kind: command
id: survey
package: S3
sources: [BLUEPRINT.md#4.9, state.md#2.16, pipeline.md#2.5]
---

# `vsdd survey`

First step of every workflow: checks the caller marker and preconditions, opens a run, takes the lock,
makes worktrees and returns what is done.

## Syntax

`vsdd survey <workflow> [--subject] [--repo] [--cycle] [--request-uri] [--guidance-uri] [--question-uri]`

## What it does

"Checks that the caller marker for this call exists (the hooks are alive) and the workflow's preconditions;
then commits pending state, opens a run, takes the lock, makes worktrees and diff patches, applies the done
test, applies an approved toolchain file to unset `[commands]` and `[paths]` keys, and returns the survey
object." ([[topic:caller-identity]], [[topic:workflow-shape]])

Per workflow: "For `feature` with a request it writes the request file and opens a cycle, or leaves the
request queued when a cycle is active; with `--amend` it runs inside the open cycle. For `spec` it marks
the run as holding an amendment when the latest `spec` decision is approved. For `spike` it stores the
question in the run record and takes no lock." Free text arrives percent-encoded in the `-uri` options,
"because a script cannot write a file and a clerk's command takes no standard input".

Order of the checks: "The command first checks that the hook runner is alive for
this very call (the caller marker the hook wrote for it) and that every precondition is met.
Only then does it commit any pending state, open a run record, take the run lock, and make the worktrees
and diff patches the units need." "If a precondition is unmet no lock is taken and the script returns at
once with the reason and `next`."

Adoption. A remote story branch whose pull request carries the story trailer and whose tracked state shows
the story undelivered in the open cycle is adoptable: "the survey fetches it and continues the delivery from
the open pull request"; otherwise the unit is reported blocked with kind `conflict`.

## Who may call it

`clerk` — "The clerk step that opens every workflow".

## Writes

The run record ([[record:run]]; each story unit the survey returns is recorded in the run's `stories`),
the lock ([[record:lock]]), `cycle.json` when a request starts a cycle
([[record:cycle]]), worktrees and diff patches via the worktree library ([[topic:worktrees]]), the request
file for a new `feature`, the toolchain application to unset keys; and, for `import` baselines, the
first-survey measurements.

## Output and failure cases

The survey object ([[schema:survey]]): "project directory, plugin root, the run id, preconditions with
whether each is met, the units and their positions, each stage name recorded as `done` (a stage not listed
is `todo`), the limits and thresholds from project configuration, open and settled decisions, and `next`".
Failure: no caller marker ("the survey refuses to start a run when its call has no marker, because that
means the hooks are not active for this session"), an unmet precondition, or a lock held by a live run
(`ok: false` with the holder). A lock the caller's own session holds for the same workflow and subject is
not a failure: the survey re-enters or takes over, as [[topic:run-lock]] sets out, so running the same
command again after an interruption continues the work.

## Feature flags (decision D31; built when the build widens)

Not on the thin slice path. The survey gains one step, F-FILL of [[table:flag-checks]]: it keeps the
approved flag settings in `factory.toml` and records the flags the code already reads.

## Team use (decision D33; built when the build widens)

Not on the thin slice path. On a team the survey gains two steps: a sync before any precondition is
read, without which no run starts (T-FRESH), and a claim for each scope the run needs once the lock is
taken (T-CLAIM). Wherever the target branch has an upstream it lists the hand commits on it (T-NOTICE),
and for a story branch that T-BRANCH's `runs` cell has it look for on the remote, it brings in what
arrived (T-BRANCH); there the adoption rule above gives way to the claim ([[topic:run-lock]]). Its
object gains `elsewhere` and `hand_commits`. The checks are [[table:sync-checks]], [[table:work-claim-checks]] and
[[table:team-delivery-checks]].
