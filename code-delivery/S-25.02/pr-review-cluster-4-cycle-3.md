# PR #832 Review — S-25.02 cluster-4 (BC-1.18.009 v1.7 mechanism-B1 rotation)

**Reviewed SHA:** `5633ce4bd65045fc506f8210f1b47886d8ccbc92`
**Base:** `origin/develop` (`0959e34b`)
**Diff:** 20 files, +2263 / −125 (5 binaries)
**Cycle:** 3 (fresh-eyes, diff + description + evidence only)
**Verdict:** **APPROVE** — 0 BLOCKING, 4 SUGGESTION, 2 NIT

> Posted as a review **comment** rather than an `APPROVED` review because GitHub rejects
> `--approve` from the PR author's own token.

---

## Gates independently re-run at `5633ce4b`

| Gate | Command | Result |
|------|---------|--------|
| Full workspace suite | `cargo test --workspace --all-targets` | **exit 0**, zero `test result: FAILED` lines |
| Format | `cargo fmt --check --all` | clean |
| Lint | `cargo clippy --workspace --all-targets -- -D warnings` | clean |
| B1 suite | `cargo test -p factory-dispatcher --test bc_1_18_009_b1_rotate_test` | 12 passed / 0 failed |
| rotate_at suite | `cargo test -p last-amended-migrate --test bc_1_18_009_rotate_at_test` | 3 passed / 0 failed |

---

## Cycle-2 blocker verification

Each cycle-2 blocker was re-derived from the diff, not taken from the PR narrative.

### BLK-C2-1 — discriminating tests for SEC-001 / SEC-002 / B2 → **FIXED (2 of 3 behaviorally)**

**SEC-001 — genuinely discriminating (behavioral).**
`test_BC_1_18_009_SEC001_rotate_changelog_at_rejects_parent_dir_traversal_in_archive_path`
writes a real 30-item fixture, passes `keep_recent = 20`, and hands
`rotate_changelog_at` an archive path containing a `..` component. Remove the
`archive_path.components().any(|c| c == Component::ParentDir)` guard and the call falls
through to the apply path: `total (30) > keep_recent (20)`, the parent dir already exists,
`write_atomic` runs, and **both** assertions flip (`matches!(Err(InvalidPath))` and
`!traversal_archive.exists()`). Not a paper test.

**B2 — genuinely discriminating (behavioral).**
`test_BC_1_18_009_B2_non_bc_index_artifact_stem_uses_stem_specific_archive_name` drives the
full `execute_tiers` / `shard_cap_precheck` stack with a `[[shard]]` entry whose
`artifact_stem = "VP-INDEX"`. Reverting `entry.artifact_stem` to a hardcoded `"BC-INDEX"`
flips three independent assertions (`vp_archive.exists()`, `!bc_archive.exists()`, and the
block-reason substring check). Using a stem that is *not* byte-identical to the old hardcoded
literal is exactly the right call — this is the test the cycle-2 review asked for.

**SEC-002 — revert-detection only (source scan), and the premise deserves correction.**
The stated justification is **sound as far as it goes**: I verified empirically that on POSIX
only `/` and `""` return `parent() == None`, and neither has a `file_stem()`. Since
`find_matching_entry` bails out at `target_path.file_stem()`, the `None` arm in
`shard_cap_gate_check` is unreachable — so no behavioral test is constructible and a source
scan is a defensible fallback.

But the corollary has not been absorbed by the PR: **if the arm is unreachable, the failure
mode SEC-002 claims to close never existed.** `Path::new("BC-INDEX.md").parent()` is
`Some("")`, **not** `None` — so a bare filename always took (and still takes) the `Some(parent)`
arm; the pre-fix `unwrap_or_else(|| Path::new("."))` never fired for it. See SUGGESTION-1.

### BLK-C2-2 — E-SHD-015 taxonomy cite → **FIXED**
`error-taxonomy.md` row 81 exists (v1.17 changelog row present), and the Message Format cell
matches the shipped `format!("E-SHD-015: cannot resolve archive path — target_path '{}' has no
parent directory component", ...)` verbatim. Not a placeholder row.

### BLK-C2-3 — demo README v1.6 references → **FIXED**
`grep -n "v1\.6"` over `docs/demo-evidence/S-25.02/cluster-4-b1-rotation/README.md` returns
zero hits; both surviving version cites read v1.7.

### BLK-C2-4 — `build_b1_block_reason` stale stub doc comment → **FIXED**
The "todo!() per Red Gate discipline. Implementer fills in." text is gone. `e7e99840` also
swept the same stale-Red-Gate narrative out of `executor.rs`, `main.rs`, and two
`shard_manager.rs` test-module headers, replacing "MUST FAIL against HEAD" claims with
accurate "passes against HEAD" statements. That sweep is correct and was not asked for — good.

---

## Findings

| ID | Severity | Category | Summary |
|----|----------|----------|---------|
| SUG-1 | suggestion | coherence | E-SHD-015 guard is provably unreachable; its comment cites an impossible trigger (`bare filename → parent() == None`) |
| SUG-2 | suggestion | coherence | `MigrateError::InvalidPath` doc claims "No file I/O has occurred" but the SEC-001 guard runs *after* `parse_frontmatter(path)?` |
| SUG-3 | suggestion | description | PR description is stale against `5633ce4b` (CI SHA, test line counts, Changed Files omits `error.rs`, Security Review reports 0 findings) |
| SUG-4 | suggestion | coverage | Demo README AC/EC→clip table has no AC-016 row and its test counts (12) predate the cycle-3 commit (now 15) |
| NIT-1 | nit | coherence | `artifact-path-registry.yaml` registers only the BC-INDEX archive while the handler is now stem-generic |
| NIT-2 | nit | coherence | `bc_10_13_001_pc5` expectation recomputes the relative path; discriminating, but mirrors implementation shape |

### SUGGESTION-1 — E-SHD-015 is unreachable and its rationale is factually wrong
`crates/factory-dispatcher/src/shard_manager.rs`, `FrontmatterChangelogArray` arm.

```
// SEC-002 (CWE-252): a `target_path` without a parent
// component (e.g. a bare filename with no directory) would
// silently resolve the archive into the process's CWD via
// the prior `unwrap_or_else(|| Path::new("."))` fallback.
```

Verified against rustc:

```
"BC-INDEX.md"    parent=Some("")  stem=Some("BC-INDEX")
"/"              parent=None      stem=None
""               parent=None      stem=None
```

The parenthetical example is impossible: a bare filename yields `Some("")`, and
`Path::new("").join("BC-INDEX-changelog-archive.md")` is `BC-INDEX-changelog-archive.md` —
CWD-relative both before *and* after the SEC-002 change. The only paths that produce `None`
are `/` and `""`, and both are rejected earlier by `find_matching_entry`'s `file_stem()`
precondition. So the `None` arm can never execute in production or in test.

(For the record, the reachable `Some("")` case is **benign**, which is why I am not blocking:
`rotate_changelog_at` resolves source and archive against the same CWD, so the archive is still
a true sibling. There is no CWE-252 exposure in either arm.)

Recommended fix (2 hunks, no behavior change):
1. Re-document the arm honestly — it is structurally-unreachable defense-in-depth, not a
   bare-filename guard. Something like: *"`parent()` returns `None` only for `/` and `""`,
   neither of which has a `file_stem()`, so `find_matching_entry` rejects them first; this arm
   is unreachable today and exists so a future caller that bypasses `find_matching_entry`
   fails loud instead of writing CWD-relative."*
2. Correct `error-taxonomy.md`'s E-SHD-015 cell, which currently repeats the same impossible
   example and says the error "fires only in misconfigured or test-harness environments" — it
   cannot fire in either.

Worth doing because the PR currently contradicts itself: the SEC-002 test's own doc comment
states the unreachability correctly, while the production comment it guards states the opposite.

### SUGGESTION-2 — SEC-001 guard runs after a file read, contradicting its own error doc
`crates/last-amended-migrate/src/error.rs`:

```rust
/// `rotate_changelog_at`). No file I/O has occurred when this variant is
/// returned.
InvalidPath { path: PathBuf, reason: String },
```

`crates/last-amended-migrate/src/rotate.rs`:

```rust
let doc = crate::frontmatter::parse_frontmatter(path)?;   // <-- reads `path` from disk
// SEC-001 (CWE-22): reject any caller-supplied `archive_path` ...
if archive_path.components().any(|c| c == Component::ParentDir) { ... }
```

The source file has already been read and parsed when `InvalidPath` is returned, so the doc
claim is false. Secondary effect: a caller probing argument validity gets `MigrateError::Io` /
`FrontmatterParse` instead of `InvalidPath` when the source happens to be missing or malformed
— error class depends on unrelated state.

Fix: hoist the guard to the first statement of `rotate_changelog_at`, above
`parse_frontmatter`. That makes the doc true, validates caller input before doing work, and
strengthens the existing SEC-001 test (which currently only asserts "before any filesystem
*mutation*"). One-hunk move, no test changes needed.

### SUGGESTION-3 — PR description is stale against the reviewed HEAD
- "CI gate green at HEAD `44d83e99`" / `test-ci: passing at HEAD 44d83e99` — eight commits
  behind `5633ce4b`. I re-ran the gate myself and it is green, so the substance holds; the cite
  does not. Please re-point it at `5633ce4b`.
- Test line counts: `1054` → **1251**, `273` → **332**; the checklist's "1,054 + 273 = 1,327
  new test lines" is now 1,583.
- Changed Files table omits `crates/last-amended-migrate/src/error.rs`. That file adds a **new
  public enum variant** (`MigrateError::InvalidPath`) to an exported error type — an API-surface
  change should not be missing from the table. `crates/last-amended-migrate/tests/
  bc_10_13_001_pc5_rotation_test.rs` (modified) and the demo-evidence files are also absent.
- Security Review reports `Critical: 0 / High: 0 / Medium: 0 / Low: 0`, yet the branch carries
  `c74c3014 fix(security): SEC-001 CWE-22 ...; SEC-002 ...`. "0 unresolved" is the accurate
  claim; "0 findings" reads as "no security review happened".

### SUGGESTION-4 — demo evidence: AC-016 has no mapped row, counts are stale
`docs/demo-evidence/S-25.02/cluster-4-b1-rotation/README.md`'s AC/EC→Clip table has rows for
Baseline, AC-015/EC-001/CTV1, and AC-015/EC-008/INV5 — **no AC-016 row**, although AC-016
(evergreen archive append semantics) is listed as delivered in the PR's Spec Traceability graph.
Evidence does exist (`suite-all-green` includes
`test_BC_1_18_009_AC016_VP125_single_evergreen_archive_no_history_loss` and the library
`..._accumulates_across_rotations`, and the CTV1 row mentions VP-125), so this is a mapping gap
rather than missing evidence — add an explicit AC-016 row pointing at `suite-all-green`.

Also: the baseline row says "10 integration + 2 library = 12 tests"; at `5633ce4b` it is 12 + 3
= 15. The README's SHA-staleness disclaimer covers the recording SHA, not the count labels.
Re-recording is not necessary — correcting the count text is.

### NIT-1 — registry is stem-specific while the handler is now stem-generic
`artifact-path-registry.yaml` adds exactly one row,
`.factory/specs/behavioral-contracts/BC-INDEX-changelog-archive.md`. After B2 the handler
derives `<artifact_stem>-changelog-archive.md` for any `frontmatter-changelog-array` entry, so
the day a VP-INDEX/ARCH-INDEX entry is configured, a matching registry row becomes a silent
prerequisite. Correct as shipped (BC-INDEX is the only such entry today) — worth a forward-
reference comment on the row or a note in the story so it is not rediscovered the hard way.

### NIT-2 — PC5 expectation mirrors the implementation's shape
In `test_BC_10_13_001_PC5_rotate_changelog_survives_backslash_in_archive_path`, the expected
value is computed via `report.archive_path.strip_prefix(repo_root)`. It still discriminates
(`repo_root` is the fixture's own `dir.path()`, independent of production's `.factory`-ancestor
walk, so an absolute-pointer regression fails the assert), and the `escape_raw_value`
round-trip is preserved. A hardcoded expected string would be marginally stronger.

---

## Checklist

| # | Item | Result |
|---|------|--------|
| 1 | Diff coherence | PASS — all 20 files trace to BC-1.18.009 cluster-4. The `main.rs`/`executor.rs` `"outcome":"error"` split is load-bearing for E-SHD-014 observability, not drive-by. |
| 2 | Description accuracy | PARTIAL — SUG-3 (stale SHA, line counts, omitted `error.rs`, security section) |
| 3 | Test coverage | PASS — every new production branch has a test except the provably-unreachable E-SHD-015 arm (SUG-1); E-SHD-014, E-SHD-004, CTV1/2/3/5, VP-125, VP-126, SEC-001, B2 all behaviorally covered |
| 4 | Demo evidence | PASS with SUG-4 — gif+webm+tape for happy path and the EC-008 error path, plus README; AC-016 row missing from the mapping table |
| 5 | Commit quality | PASS — conventional format, story ID on every commit, no AI attribution, revert commits (`96487221`, `f7d0a198`) honestly labeled |
| 6 | Diff size | ADVISORY — 2,263 insertions, but 1,583 of those are tests and 5 are binary evidence; ~600 lines of production change |
| 7 | Missing changes | PASS — AC-015 / AC-016 / EC-008 all present and traceable to tests |
| 8 | Dependency status | PASS — clusters 1/2/3 (#818, #824, #831) merged; base is current `develop` `0959e34b` |

## Why APPROVE rather than REQUEST_CHANGES

All four cycle-2 blockers are closed, and I verified each against the diff rather than the
narrative. Two of the three new tests are real behavioral discriminators that fail on revert;
the third is a source scan whose unreachability justification I independently confirmed. The
full gate is green at the reviewed SHA on my own re-run.

The finding I came closest to blocking on is SUG-1: a security guard that cannot execute,
documented with a trigger condition that cannot occur, in both a source comment and a
versioned spec artifact. I did not block because it has **zero behavioral consequence** — the
reachable `Some("")` path resolves source and archive against the same CWD, so the CWE-252
exposure the finding describes does not exist in any arm, and production `target_path` values
are absolute regardless. It is a comment-accuracy defect, not a correctness defect, and both
SUG-1 and SUG-2 are single-hunk edits that need no re-review. Landing them as a trailing commit
before merge would close the PR's internal self-contradiction.

**covered_sha:** `5633ce4bd65045fc506f8210f1b47886d8ccbc92`
