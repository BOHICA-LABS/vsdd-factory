# Demo Evidence Report — S-15.03

**Story:** S-15.03 — ARCH-INDEX Cite-Refresh Hook + Lessons Retroactive-Sweep Verification (Scope Extension: `last_amended` Write-Path Durable Fix)
**Branch:** feature/S-15.03
**BC gates:** BC-10.13.001 v1.1 (`last-amended-migrate` CLI), BC-5.45.001 (write-path invariant), BC-4.18.001 (fuel-budget relief)
**Demo strategy:** VHS terminal recordings of the `last-amended-migrate` Rust CLI (`migrate` subcommand, `--check`/apply) against synthetic fixture files
**Product type:** Non-UI Rust CLI binary — VHS used (no browser/GUI)
**POLICY 10:** All files are under `docs/demo-evidence/S-15.03/` (story-scoped subfolder)

**Fixture safety:** every recording operates ONLY on synthetic fixture files under a throwaway `/tmp/lam-demo-*` scratch root created fresh by each tape's hidden setup. No file under `.factory/` or `plugins/vsdd-factory/config/` is read or written by any recording in this report.

**CLI hardening note:** at recording time the `last-amended-migrate` working tree carried in-progress, uncommitted SEC-001/SEC-002/SEC-003 hardening (YAML-injection guard, `--path`/`--registry` allowlisting, atomic writes) beyond what the committed history / story text describe. The SEC-002 `--path` allowlist requires `--path` to canonicalize to exactly one of the 5 BC-10.13.001 target files (`stories/STORY-INDEX.md`, `specs/behavioral-contracts/BC-INDEX.md`, `specs/architecture/ARCH-INDEX.md`, `specs/verification-properties/VP-INDEX.md`, `STATE.md`) under the supplied `--factory-root`. Every tape's fixture is therefore laid out at the matching relative path under a synthetic `--factory-root` (e.g. `$ROOT/stories/STORY-INDEX.md` with `--factory-root $ROOT`) rather than a bare file directly under `/tmp`, so the demonstrated commands reflect the CLI's actual current behavior. This is recording-only adaptation to the binary's real interface — no source or test file was modified to produce these recordings.

**VHS timing note:** the installed `vhs` (0.11.0) does not reliably support `Wait`/`Wait+Line` pattern-matching in this environment — verified empirically, including on a trivial `echo` + `Wait+Line` case, which timed out. All three tapes therefore use fixed `Sleep` durations instead, consistent with this repo's other pre-existing CLI demo tapes (e.g. `docs/demo-evidence/S-12.02/*.tape`).

---

## Coverage Map

| Recording | AC | BC-10.13.001 clause | Outcome demonstrated | GIF | WebM | Tape |
|-----------|----|--------------------|------------------------|-----|------|------|
| AC-006-migrate-full-recovery-split | AC-006 | PC7 — full-recovery split | `--check` detects an inline `[Prior: ...]` chain (dry-run, exit 1, fixture untouched); apply performs the split — current entry stays in `last_amended`, both chained entries relocate into `changelog:` newest-first | [gif](AC-006-migrate-full-recovery-split.gif) | [webm](AC-006-migrate-full-recovery-split.webm) | [tape](AC-006-migrate-full-recovery-split.tape) |
| AC-008-yaml-escape-hardening | AC-008 | PC3 — D-1144 escape hardening | `--check` detects an unescaped literal `"` in a `last_amended` entry (exit 1); apply rewrites the value with the quote escaped as `\"`, `changelog:` left untouched, result parses cleanly as YAML | [gif](AC-008-yaml-escape-hardening.gif) | [webm](AC-008-yaml-escape-hardening.webm) | [tape](AC-008-yaml-escape-hardening.tape) |
| AC-009-AC-010-idempotency-recheck | AC-009, AC-010 | PC4 / Invariant 2 — idempotency | After both fixtures above have already been migrated once, re-running `--check` on each reports `mutated=false` and exits 0 — zero further violations on either the split-recovered file or the escape-fixed file | [gif](AC-009-AC-010-idempotency-recheck.gif) | [webm](AC-009-AC-010-idempotency-recheck.webm) | [tape](AC-009-AC-010-idempotency-recheck.tape) |

---

## AC-006 — Sanctioned migrate tool: `--check` dry-run + apply full-recovery split

**BC-10.13.001 clause:** PC7 (full-recovery split, BC-10.13.001 v1.1)
**Fixture:** `$ROOT/stories/STORY-INDEX.md` — a synthetic `STORY-INDEX.md`-shaped file whose `last_amended` holds an old-style inline `[Prior: ...]` bracket chain nesting two historical entries (mirroring the pre-D-1149 unbounded-growth shape).

**What the recording shows:**
1. `cat` of the fixture — the nested `[Prior: 2026-08-15 (v4.20) — previous change entry [Prior: 2026-07-01 (v4.10) — earlier change entry]]` chain is visible in `last_amended`.
2. `migrate --path ... --factory-root ... --check` — reports `eligibility=PriorChainSplit changelog=Added escape_fixed=false entries_recovered=2 mutated=true` and exits `1` (drift found, mirroring `compute-input-hash --check`'s convention); the fixture is confirmed byte-for-byte unchanged after this call (dry-run, no write).
3. `migrate --path ... --factory-root ...` (apply, no `--check`) — performs the split and exits `0`.
4. `cat` of the resulting file — `last_amended` now holds ONLY the current entry (`"2026-09-02 (v4.430) — some entry text"`); a new `changelog:` sequence has been added with both prior entries as separate items, newest-first (`2026-08-15`/`v4.20` first, `2026-07-01`/`v4.10` second).

This directly demonstrates AC-006(ii)/BC-10.13.001 PC7: the tool is the sanctioned, complete replacement for a D-1149-style POL-3 exception — it recovers an inline-chain file in one run rather than refusing it.

---

## AC-008 — D-1144 YAML-escape hardening

**BC-10.13.001 clause:** PC3 (escape hardening); BC-5.45.001 Invariant 3 (strict-YAML-clean output)
**Fixture:** `$ROOT/specs/behavioral-contracts/BC-INDEX.md` — a synthetic `BC-INDEX.md`-shaped file whose `last_amended` entry contains an unescaped literal `"` (`fixed the "quoted term" defect`), reproducing the D-1144 defect class found on the real `BC-INDEX.md`/`ARCH-INDEX.md`/`STATE.md`. A pre-existing `changelog:` item is present to confirm it is left untouched.

**What the recording shows:**
1. `cat` of the fixture — the literal, unescaped `"quoted term"` is visible inside the double-quoted `last_amended` scalar (a genuinely YAML-invalid file as constructed).
2. `migrate --path ... --factory-root ... --check` — reports `eligibility=CurrentEntryOnly changelog=AlreadyPresent escape_fixed=true entries_recovered=0 mutated=true` and exits `1`.
3. `migrate --path ... --factory-root ...` (apply) — exits `0`.
4. `cat` of the resulting file — `last_amended` now reads `"2026-09-02 (v5.41) — fixed the \"quoted term\" defect"` (embedded quotes escaped as `\"`); the pre-existing `changelog:` item (`an older entry`) is unchanged.

---

## AC-009 / AC-010 — Idempotency: zero further violations on re-check

**BC-10.13.001 clause:** PC4 / Invariant 2 (idempotency across every subcommand)
**Fixtures:** both the AC-006 split fixture and the AC-008 escape fixture, each already migrated once (apply run silently in the tape's hidden setup) before the recording begins.

**What the recording shows:**
1. `migrate --path <split-fixture> --factory-root ... --check` — reports `eligibility=CurrentEntryOnly changelog=AlreadyPresent escape_fixed=false entries_recovered=0 mutated=false` and exits `0` (the post-split file reclassifies as `CurrentEntryOnly` — no chain left to re-detect, no re-split).
2. `migrate --path <escape-fixture> --factory-root ... --check` — reports the same `mutated=false` shape and exits `0` (the post-fix value is already fully escaped — no re-flagging).

Both re-checks report **zero further violations**, directly demonstrating AC-009's fuel-relief acceptance (no repeated mutation/re-write on an already-compliant file) and AC-010's "no future POL-3/TD-FACTORY-HOOK-BYPASS-001 exception ever needed for this class" guarantee — the tool converges to a stable, verified-clean state after one apply run, for both the full-recovery-split path and the escape-fix path.

---

## N/A — Documentation/Audit-Only Acceptance Criteria

The following ACs have no runtime behavior to demo (they are documentation or audit deliverables, not executable CLI behavior) and are recorded here as N/A per this task's scope:

| AC | Reason N/A |
|----|------------|
| AC-002 | Validator-compatibility audit (arm_e, naive line-based extractors, other frontmatter readers) — a written audit finding (ADR-049 Phase A), not a runtime demo. |
| AC-003 | Alternatives documented, not implemented — a documentation-only AC by its own text ("neither ships as part of this story's delivery"). |
| AC-004 | `plugins/vsdd-factory/skills/state-burst/SKILL.md` prose update — a skill-file text change, not CLI-demonstrable. |
| AC-005 | `plugins/vsdd-factory/agents/state-manager.md` prompt-text update — an agent-prompt text change, not CLI-demonstrable. |

---

## Test Suite Summary

The `last-amended-migrate` crate's own Red Gate test suite (`crates/last-amended-migrate/tests/bc_10_13_001_*.rs`) exercises `migrate_file`/`migrate_all`/`rotate_changelog` directly at the library level and passed before this recording session began; these VHS recordings are CLI-surface acceptance evidence layered on top of that suite, not a replacement for it.

---

## File Index

```
docs/demo-evidence/S-15.03/
├── evidence-report.md                                  (this file)
├── AC-006-migrate-full-recovery-split.tape
├── AC-006-migrate-full-recovery-split.gif
├── AC-006-migrate-full-recovery-split.webm
├── AC-008-yaml-escape-hardening.tape
├── AC-008-yaml-escape-hardening.gif
├── AC-008-yaml-escape-hardening.webm
├── AC-009-AC-010-idempotency-recheck.tape
├── AC-009-AC-010-idempotency-recheck.gif
└── AC-009-AC-010-idempotency-recheck.webm
```

Total: 10 files (1 report + 3 tapes + 3 gifs + 3 webms)
POLICY 10 compliance: all files under `docs/demo-evidence/S-15.03/` — no flat files at `docs/demo-evidence/*.md`
