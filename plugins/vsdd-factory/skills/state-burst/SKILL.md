---
name: state-burst
description: Execute the Single-Commit Burst Protocol (TD-VSDD-053) for state-manager remediation bursts. One atomic commit per burst — no Stage 2 backfill, no SHA placeholder, no chain. Refuses in-progress narrative voice and reintroduction of the retired two-commit pattern.
disable-model-invocation: false
allowed-tools: Read, Write, Edit, Bash
---

# State-Burst Protocol (Single-Commit)

This skill executes the canonical state-manager remediation burst safely.

## History

This skill previously implemented the two-commit "Single Canonical SHA +
Stage 2 Backfill" protocol. That pattern was self-referential: STATE.md
sits ON the factory-artifacts branch, so committing STATE.md changes
HEAD, instantly staling any HEAD-SHA cite inside the same content. The
two-commit workaround (Stage 1 placeholder → commit → Stage 2 backfill
SHA → commit) created "fix-the-fix" loops when any of 8 cite locations
was missed, manifesting **6 consecutive recurrences in one session
costing 5+ force-pushes** (see
`docs/lessons-learned/wave-gate-bookkeeping.md`).

TD-VSDD-053 (2026-05-04) retired the two-commit protocol by removing
the self-referential cite altogether: STATE.md and SESSION-HANDOFF.md
no longer claim the current factory-artifacts HEAD SHA in their
"current state" sections. Git itself owns that data — run
`git -C .factory log -1 --format='%h %s'` for it. Historical SHA
references in changelog rows, decisions log, and cycle manifests remain
valid (immutable PAST burst SHAs).

## Announce at Start

Before any other action, say verbatim:

> I'm using the state-burst skill to execute the Single-Commit Burst
> Protocol for this remediation burst. One atomic commit; no Stage 2
> backfill; no SHA placeholder. The current factory-artifacts HEAD SHA
> is not cited in STATE.md/HANDOFF.md "current state" sections.

## When to use

- You are remediating findings from an adversarial pass (Phase 3
  wave-gate convergence, or any analogous gate that produces a per-pass
  review).
- You need to update STATE.md, SESSION-HANDOFF.md, and wave-state.yaml
  in lockstep with a single commit.
- You're committing to the `factory-artifacts` branch (not `develop`).

If you only need to update one of those files for non-burst bookkeeping
(e.g., a session-checkpoint refresh), use the regular state-manager
update protocol — this skill is overkill.

## Pre-burst hygiene

Run before applying any changes:

```bash
git -C .factory status
```

If there are unrelated modifications (sidecar logs, etc.):
- Commit them separately first, OR
- Stash them with `git -C .factory stash push -u`.

Pre-existing modifications **must not** contaminate the burst commit.

## Apply changes (single atomic commit)

Apply every change required by the
[State-Manager Checklist](../../templates/state-manager-checklist-template.md):

1. **Remediation deltas** to source/spec files closing the adversarial
   findings.
2. **STATE.md** updates:
   - Frontmatter `adversary_<wave>_pass_N_<gate>` entry (with
     `remediation_sha:` if your project still uses that field for
     historical record — that field is OK because once written it
     points at THIS burst's commit and never gets re-cited; future
     bursts add new entries, never modify this one).
   - Frontmatter `convergence_status` advanced to the
     `*_REMEDIATED_AWAITING_PASS_N+1` form (or `_CLEAN_WINDOW_K_OF_3`,
     `_CONVERGED`).
   - `awaiting:` field uses outcome-neutral language ("if CLEAN…if
     BLOCKED…").
   - Body table rows updated.
   - Session Resume Checkpoint replaced with current snapshot.
   - Version bumped (X.Y → X.Y+1).
   - **DO NOT** cite the current factory-artifacts HEAD SHA anywhere in
     "current state" prose. It's `git -C .factory log -1` — git owns it.
3. **SESSION-HANDOFF.md** updates (if your project uses it):
   - `develop HEAD` set to the actual current develop SHA (cross-branch
     cite — fine, no loop).
   - PR / story / test counts current.
   - Next-session priority outcome-neutral.
   - **DO NOT** cite the current factory-artifacts HEAD anywhere.
4. **wave-state.yaml** updates (write the eventual SHA AFTER the commit
   exists is no longer needed — see Stage-2-retirement note below):
   - `<wave>.gate_pass_N` record (the `remediation_sha:` field, if
     present, gets the SHA of the commit you're about to make; you
     can't know this in advance under the single-commit model. Two
     options:
     (a) Omit the SHA field for this burst — wave-state.yaml records
         only that pass N happened, and the historical lookup uses
         `git log` to map pass→commit by date/message.
     (b) Pre-compute the SHA via `git commit-tree` dry-run and write
         it before the actual commit. Most projects pick (a) — it
         avoids the loop AND the pre-compute complexity.
   - `<wave>.gate_status` updated.
   - `<wave>.notes` extended.
   - `next_gate_required` advanced.

**Tense rule** (mandatory):
Write all narrative as if the burst has already completed. ❌ Never
"REMEDIATION IN PROGRESS" or "this burst remediates…". ✅ Always
"REMEDIATED — Awaiting Pass N+1".

## `last_amended` Write-Path Discipline (BC-5.45.001 / ADR-049)

This discipline governs exactly 5 files — the D-1149 files:
`STORY-INDEX.md`, `BC-INDEX.md`, `ARCH-INDEX.md`, `VP-INDEX.md`, and
`STATE.md`. It does NOT extend to any other `.factory/` artifact's own
`last_amended` field (BC-5.45.001 §Out of scope — those remain governed,
where checked at all, by the pre-existing position-0 `(vX.Y)` parity
check, unaffected by this discipline).

Whenever this burst writes a new history entry to one of the 5 files'
`last_amended:` frontmatter field:

1. **Overwrite, never wrap.** Write `last_amended:` as a single-line,
   double-quoted YAML scalar holding ONLY the new entry
   (`"YYYY-MM-DD (vX.Y) — <summary>"`, D-1144-escaped — an embedded
   literal `"` becomes `\"`). NEVER read the existing value and
   concatenate it into the new value as a `[Prior: ...]` bracket or any
   other nested form. This read-wrap-rewrite pattern is exactly what
   produced the 323,499-char `STORY-INDEX.md` mega-line (D-1149) and the
   743 fuel-timeouts/day WASM validator symptom it caused.
2. **Prepend the displaced entry to `changelog:`.** For `ARCH-INDEX.md`,
   `BC-INDEX.md`, `VP-INDEX.md`, and `STORY-INDEX.md` (all four carry a
   frontmatter `changelog:` sequence), PREPEND exactly ONE new list item
   — the entry `last_amended` held immediately before this write,
   verbatim (including any trailing `[Prior history → ...]` pointer
   note) — to the top of `changelog:`. Every existing `changelog:` item
   is left byte-for-byte untouched: this is a list-item prepend, never a
   rewrite-in-place of the sequence or of any existing item.
3. **`STATE.md` has no `changelog:` counterpart.** Apply step 1 only; do
   NOT add a frontmatter `changelog:` field to STATE.md. The displaced
   entry is superseded by STATE.md's own already-append-only body-level
   `## Decisions Log`/`## Phase Progress` sections, which are the durable
   historical record for this file.
4. **Never emit an inline chain.** No write under this discipline ever
   produces a `last_amended` value containing a nested
   `[Prior: <date> (vX.Y) — ...]` bracket referring to a DIFFERENT dated
   entry than the current one. (The static, non-growing
   `[Prior history → <file>-amendment-history.md]` pointer note is NOT
   this pattern — it never grows and carries no dated entry of its own —
   and MAY be retained/repeated verbatim across writes.)
5. **Every emitted value is strictly-valid YAML.** Escape embedded
   literal `"` per D-1144 in both the `last_amended` scalar and any
   `changelog:` item's text field so the frontmatter parses cleanly under
   strict YAML `safe_load`.

**Pre-push guard.** Before pushing any burst that edits one or more of
the 5 governed files, run:

```bash
cargo run -p last-amended-migrate -- migrate --check
```

Exit 0 means every governed file is already in the current-entry-only +
`changelog:` shape — no drift. A non-zero exit means at least one of the
5 files still needs migrating or D-1144 escape remediation; do NOT
hand-patch it — re-run the same subcommand without `--check` to apply, or
see Recovery below if the drift is an inline chain.

**Recovery — if a mega-line/inline `[Prior: ...]` chain is ever
detected** on one of the 5 files, at any scale: this is BC-10.13.001's
sanctioned **full-recovery-split** remedy (PC7), NOT a fresh
POL-3/TD-FACTORY-HOOK-BYPASS-001 exception request. Run:

```bash
cargo run -p last-amended-migrate -- migrate --path <file>
```

For `STORY-INDEX.md`, `BC-INDEX.md`, `ARCH-INDEX.md`, and `VP-INDEX.md`,
the tool splits the chain in place — the current entry stays in
`last_amended`; every chained historical entry is RELOCATED into
`changelog:` as a new item, newest-first, verbatim (D-1144-escaped) — via
a bounded/streaming linear scan safe on arbitrarily long input, up to and
beyond the D-1149 323K-350K-char calibration scale. Invoking the tool is
the sanctioned path for this failure class going forward; no human
POL-3 exception is needed or should be requested (BC-10.13.001 PC7,
S-15.03 AC-010).

**`STATE.md` is different (S-15.03 pr-reviewer B2-R) — the tool REFUSES
by default, it does not silently relocate.** `STATE.md` has no
`changelog:` field to relocate the chained entries into (ADR-049
Decision 4), and PC6 forbids this tool from ever writing them into the
frozen `STATE-amendment-history.md` sidecar — so, unlike the other 4
files, there is no real destination for the recovered text. Running
`migrate --path <file>` (or a bare `migrate`) against a `STATE.md` chain
therefore returns `Err(MigrateError::StateChainDiscardNotAuthorized)` and
mutates nothing (in both `--check` and apply mode) UNLESS you explicitly
pass `--discard-state-chain`:

```bash
# Refuses (default) — leaves STATE.md untouched, prints the entry count
# that would be lost and where to look instead:
cargo run -p last-amended-migrate -- migrate --path .factory/STATE.md

# Explicit, human-directed acknowledgment that the chained entries will be
# PERMANENTLY DISCARDED (only after confirming their substantive content
# already lives in STATE.md's own body-level ## Decisions Log — see the
# Write-Path Discipline above):
cargo run -p last-amended-migrate -- migrate --path .factory/STATE.md --discard-state-chain
```

Before passing `--discard-state-chain`, manually verify the chained
entries' substantive text is already recorded in `## Decisions Log`/
`## Phase Progress` — a surviving inline chain is itself evidence this
discipline may not have been followed for those specific entries, so do
not assume it without checking. If it is NOT already recorded, copy the
substantive content into the body first, THEN run the discard.

## Apply changes — mandatory renew step

Before staging the commit, renew the factory lock (if one is held) to advance
`factory_lock.expires_at` and `timestamp:` in STATE.md:

```bash
bash plugins/vsdd-factory/bin/factory-lock-write.sh renew .factory/STATE.md
```

No-op when factory is unlocked (absent `factory_lock:` key) — exits 0 with
'no factory_lock block present — renew is a no-op'. Safe to call
unconditionally.

## Commit

When all changes are staged:

```bash
git -C .factory add -A
git -C .factory commit -m "fix(<wave>): close pass N findings — REMEDIATED awaiting pass N+1"
```

The commit message must NOT contain the word `backfill` (that token is
reserved for the retired Stage 2 pattern; using it would trigger
`MULTI_COMMIT_CHAIN_NOT_ALLOWED` if any subsequent commit also uses it).

## Verification

Run the hook:

```bash
bash .factory/hooks/verify-sha-currency.sh
```

Must report `PASS`. The hook now checks:
- `develop` SHA cited in STATE.md/HANDOFF.md matches actual develop HEAD
  (cross-branch cite — no loop)
- No `MULTI_COMMIT_CHAIN_NOT_ALLOWED` (the chain-shape regression guard)
- Cross-record agreement between wave-state.yaml `gate_pass_N`
  remediation_sha entries and STATE.md frontmatter (if both record the
  same SHA, they must agree)
- No tense-flip in active-pass narrative (advisory)

If FAIL:
- Inspect the failure message.
- DO NOT add a second commit. Instead:
  ```bash
  git -C .factory reset --soft HEAD
  ```
  then re-edit and re-commit.

## Push

Use the `factory-cas-push.sh` helper (BC-5.40.001 PC5 / S-17.01 D6). This replaces
the former blind `git push origin factory-artifacts` with a fetch-then-`--force-with-lease`
CAS sequence that detects concurrent writes rather than silently clobbering them.

```bash
bash plugins/vsdd-factory/bin/factory-cas-push.sh
```

The helper internally runs:
1. `git -C .factory fetch origin factory-artifacts` — synchronize remote ref
2. `EXPECTED_SHA=$(git -C .factory rev-parse origin/factory-artifacts)` — capture tip
3. `git -C .factory push --force-with-lease=factory-artifacts:"${EXPECTED_SHA}" origin factory-artifacts`

On push rejection (concurrent write detected), the helper exits non-zero with a
human-readable `CASPushRejected` message. The local `.factory/` commit is preserved;
fetch and retry after resolving the divergence.

After push, run the hook one more time to catch any push-side issues:

```bash
bash .factory/hooks/verify-sha-currency.sh
```

## Anti-patterns this skill blocks

| Anti-pattern | Detection | Recovery |
|--------------|-----------|----------|
| Reintroducing two-commit chain (HEAD and HEAD^ both contain `backfill`) | `verify-sha-currency.sh` reports `MULTI_COMMIT_CHAIN_NOT_ALLOWED` | `git reset --soft HEAD~2` + re-author as one commit |
| Citing current factory-artifacts HEAD SHA in STATE.md/HANDOFF.md "current state" sections | Code review / per-burst editor discipline (the hook no longer enforces this since the cite is gone) | Edit out the cite; replace with "see `git -C .factory log -1`" if guidance is needed |
| In-progress voice in narrative | Hook tense-flip WARN | Edit narrative to past-tense before push |
| Cross-record SHA drift between STATE.md and wave-state.yaml | Hook DRIFT report | Fix the disagreeing record (per Schema Semantics in checklist) |
| Develop SHA in STATE.md does not match actual develop HEAD | Hook FAIL | Update the develop cite to the current develop HEAD |
| Skipping renew before `git add` while lock is held | `verify-state-timestamp-refresh` is retired (registry entry removed per ADR-046 Decision 2; no longer invoked, does not block). The `stamp-state-timestamp` PostToolUse hook now auto-stamps `timestamp:` and renews `factory_lock.expires_at` after every tool-mediated Edit/Write/MultiEdit to STATE.md (fail-open). | The mandatory `factory-lock-write.sh renew` step (see "Apply changes — mandatory renew step" above) remains required before `git add`. The `stamp-state-timestamp` PostToolUse hook mechanizes timestamp re-stamp and lock renewal ONLY for tool-mediated STATE.md edits (Edit/Write/MultiEdit). Non-tool-mediated writes — specifically the `factory-lock-write.sh` bash writer itself and git-layer state-burst pushes — do NOT trigger the PostToolUse hook, so the manual renew step before `git add` is the authoritative mechanism on those paths. |
| Read-wrap-rewrite of `last_amended` as an inline `[Prior: ...]` bracket on one of the 5 D-1149 files (`STORY-INDEX.md`/`BC-INDEX.md`/`ARCH-INDEX.md`/`VP-INDEX.md`/`STATE.md`) | `cargo run -p last-amended-migrate -- migrate --check` reports drift, or an inline chain is found on inspection | Never hand-patch. Run `cargo run -p last-amended-migrate -- migrate --path <file>` — full-recovery split (BC-10.13.001 PC7) — not a POL-3 exception (see "`last_amended` Write-Path Discipline" above). **STATE.md is special-cased:** this refuses by default (`StateChainDiscardNotAuthorized`, no mutation) — pass `--discard-state-chain` to opt in; see above |

## When to bypass

Bypassing this skill is acceptable for:
- The first state-manager burst on a brand-new project (no
  `wave-state.yaml` yet).
- Manual recovery after a force-push event (the protocol assumes a clean
  starting tree).

In both cases, document the bypass reason in
`SESSION-HANDOFF.md → Recent Burst Episodes`.

## Reference

- Checklist: `templates/state-manager-checklist-template.md`
- Hook: `templates/verify-sha-currency.sh`
- Case study: `docs/lessons-learned/wave-gate-bookkeeping.md`
- TD: TD-VSDD-053 (single-commit protocol replacing two-commit; resolves
  TD-VSDD-044 self-referential-cite loop)
