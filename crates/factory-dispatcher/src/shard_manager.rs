//! Native (non-WASM) shard-cap formula and deterministic size/item-count
//! trigger for Layer-2 artifact sharding.
//!
//! Implements BC-1.18.005 (Byte-Size-Denominated Shard-Cap Formula and
//! Native Deterministic Size-Trigger; ADR-051 §Decision 1/2). This module
//! owns BOTH trigger shapes the dispatcher's PreToolUse handling path
//! dispatches on for `Edit`/`Write`/`MultiEdit` tool calls against a
//! `[[shard]]`-registered artifact:
//!
//! - `"flat"` shape — Postconditions 1-7: `stat()`-only byte-size trigger.
//! - `"frontmatter-changelog-array"` shape — Postcondition 8: item-count
//!   trigger over a frontmatter `changelog:` array.
//!
//! Architecturally analogous to the existing native `block_if_marker_check`
//! precedent ([`crate::indeterminate_marker::block_if_marker_check`]),
//! consulted from `executor.rs` BEFORE the registry-driven WASM plugin loop
//! (Invariant 1). The config-match check (Postcondition 1 / Invariant 3)
//! MUST occur before any `stat()`/content read of the TARGET ARTIFACT
//! itself, so the ~99% of `Edit`/`Write`/`MultiEdit` calls that do not
//! target a sharded artifact never pay that cost.
//!
//! **Corrected latency framing (PR #818 fix-burst finding B4, superseding an
//! earlier revision's "pay zero added latency" claim for this ~99% case):**
//! that claim was true only for the target-artifact-specific reads
//! (`stat()`/content) this doc originally meant. It is NOT true of the
//! `[[shard]]` config file itself: `executor.rs::shard_cap_precheck` calls
//! `ShardRegistry::load()`, which performs a real (bounded, but non-zero)
//! TOML-parse of the WHOLE config file on EVERY `Edit`/`Write`/`MultiEdit`
//! dispatch whenever a `[[shard]]` config file exists on disk — including
//! dispatches that end up matching no entry at all. BC-1.18.005 v1.12 itself
//! acknowledges this as "the 'some parse per dispatch is unavoidable' cost"
//! inherent to `Vec<ShardEntry>`'s whole-file TOML grammar (a `Vec` cannot
//! partially deserialize). Today, with no `[[shard]]` config file committed
//! anywhere in this repository, EVERY dispatch short-circuits at the
//! cheaper `Path::exists()` probe in `shard_cap_precheck` and this parse
//! cost is not yet paid at all; once a config file IS committed, every
//! matching-tool dispatch pays one bounded TOML parse, not zero cost. No
//! caching is implemented across dispatcher process invocations: this
//! dispatcher binary is spawned fresh, once, per hook event (confirmed by
//! this crate's own `main.rs` entry point — a single dispatch per process
//! lifetime, not a persistent multi-event loop), so an in-process
//! mtime-checked cache would never see a second dispatch to serve a cache
//! hit to and was deliberately not added — it would add real complexity and
//! genuine staleness-risk surface for zero measurable benefit under this
//! process model.
//!
//! # Scope note (S-25.02 F4 BC-cluster 1 "cap+trigger"; UPDATED by cluster-2)
//!
//! This module originally implemented BC-1.18.005 ONLY (tasks T-1/T-2/T-3;
//! AC-001..AC-005) — fully, not as a stub; see the "BC-5.38.001 Red Gate
//! discipline" section below. **UPDATE (S-25.02 cluster-2, "roll"):**
//! BC-1.18.006 (the observable roll/block outcome once the `"flat"` shape's
//! trigger fires) is now ALSO implemented in this module — it is no longer
//! out of scope; see the "BC-1.18.006 — Roll-Before-Write..." section
//! further below for its full implementation. BC-1.18.009 (the observable
//! rotate/block-and-retry outcome once the item-count trigger fires) and
//! BC-1.18.012 (the one-time changelog backfill migration) remain LATER
//! clusters and are still explicitly OUT OF SCOPE here — the
//! `"frontmatter-changelog-array"` shape's trigger-fired branch still only
//! owns the trigger-boundary decision and hand-off point for THOSE two BCs,
//! per Postcondition 8's "Ownership" bullet.
//!
//! # BC-5.38.001 Red Gate discipline — implemented (S-25.02 F4 BC-cluster 1;
//! EXTENDED by cluster-2)
//!
//! Every function in this module now carries a real implementation driving
//! the test-writer's Red Gate suites green (both BC-1.18.005's cluster-1
//! suite and BC-1.18.006's cluster-2 suite). A fired `"flat"`-shape trigger
//! now DOES construct an observable `HookResult::Block`/`HookResult::Error`
//! outcome via `execute_roll` (BC-1.18.006's roll-before-write mechanism,
//! implemented below) — the withdrawn cluster-1 posture (a non-fatal
//! `tracing::warn!` advisory followed by `Continue`) applies ONLY to the
//! `"frontmatter-changelog-array"` shape's item-count trigger now, whose
//! observable rotate-and-retry outcome remains owned by the still-pending
//! BC-1.18.009 cluster.
//!
//! # Scope note (S-25.02 F4 BC-cluster 3 "retention+backfill" —
//! IMPLEMENTED, BC-5.38.001 Red Gate discipline)
//!
//! This cluster adds BC-1.18.007 (Shard Retention/Compaction —
//! AC-010/AC-011/AC-012) and BC-1.18.008 (Mandatory One-Time Backfill-Split
//! of the Four Pre-Existing Oversized Cycle Append-Logs — AC-013/AC-014),
//! landing near the end of this file, after cluster-2's
//! `reconcile_leading_probe_backstop` and before the test modules.
//! **UPDATE (S-25.02 F4 cluster-3 implementer burst):** like cluster-1/2's
//! functions above, every non-trivial cluster-3 function body is now a REAL,
//! fully implemented body — the stub-architect's original `todo!()`
//! placeholders have all been replaced per BC-1.18.007's and BC-1.18.008's
//! own postconditions, driving test-writer's cluster-3 Red Gate suites
//! (`bc_1_18_007_retention_test.rs`, `bc_1_18_008_backfill_split_test.rs`)
//! to green — see the "BC-1.18.007 — Shard Retention/Compaction" and
//! "BC-1.18.008 — Mandatory One-Time Backfill-Split..." section headers
//! further below for their own per-section implementation notes. Two struct
//! fields were added to already-shipped cluster-1/2 types to carry
//! BC-1.18.007's own schema obligation (`ShardIndex::retention_count`,
//! Postcondition 1) — additive, defaulted, and NOT a behavior change to any
//! cluster-1/2 function; the two pre-existing `ShardIndex { .. }`
//! struct-literal call sites (one production, one test) were mechanically
//! extended with the new field's default value, with no other change to
//! either site. `ShardIndexEntry` itself was deliberately left UNCHANGED (no
//! new field) — BC-1.18.007 Invariant 3 explicitly sanctions either "add an
//! `archived: true` boolean" OR "update the entry's own `path` to reflect
//! the new archived location" as an implementation detail; this module
//! adopts the path-mutation form precisely to avoid a multi-site collateral
//! edit across cluster-1/2's own already-green `ShardIndexEntry { .. }`
//! literals. BC-1.18.009/BC-1.18.010/BC-1.18.011/BC-1.18.012/BC-7.08.001
//! (mechanisms B1/B2 and the Cohort B flip) remain explicitly OUT OF SCOPE
//! for this cluster — later clusters (4-7) own them.

use std::io;
use std::io::Read as _;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use thiserror::Error;

use vsdd_hook_sdk::HookResult;

// ---------------------------------------------------------------------------
// Cross-platform "genuinely missing" disambiguation (PR #824 pr-review
// Finding #1, BLOCKING on Windows CI; REWRITTEN S-25.02 cluster-2 after the
// original `raw_os_error()`-branching design ALSO failed windows-x64 CI —
// see `.factory/code-delivery/S-25.02/windows-path-semantics-research.md`,
// source-verified against Rust std's `decode_error_kind`)
// ---------------------------------------------------------------------------

/// `true` iff `err` represents a path that is genuinely absent — never a
/// path whose TRAVERSAL failed because a non-terminal component exists but
/// is not a directory (e.g. `foo/bar` where `foo` is a plain file).
///
/// On Unix, "a path component is not a directory" (`ENOTDIR`) surfaces as
/// the distinct `io::ErrorKind::NotADirectory` (stable since Rust 1.83,
/// `io_error_more`) — never `io::ErrorKind::NotFound` — so the early
/// `err.kind() != io::ErrorKind::NotFound` return below already correctly
/// rejects it there, with no need to even reach the ancestor walk.
///
/// On WINDOWS, the identical traversal-through-a-file failure collapses to
/// `io::ErrorKind::NotFound` (`ERROR_PATH_NOT_FOUND` = 3) — INDISTINGUISHABLE
/// at the `ErrorKind` level from a genuinely-missing ancestor
/// (`ERROR_FILE_NOT_FOUND` = 2, which ALSO maps to `NotFound`; both codes
/// share one match arm in Rust std's `decode_error_kind`). Windows has no
/// general `ENOTDIR` equivalent surfaced through ordinary `CreateFileW`
/// path resolution. Critically, the raw Win32 code itself is NOT a robust
/// discriminator either — 2-vs-3 is not a normative Microsoft contract, and
/// an ancestor-directory-missing failure and an ancestor-is-a-plain-file
/// failure can BOTH raise code 3 — so an earlier revision of this function
/// that branched on `raw_os_error()` was intrinsically broken on Windows
/// (see the research doc referenced above, §3-4) and is why this function
/// was rewritten.
///
/// Without correct disambiguation, this module's every `NotFound`-relief
/// site (e.g. [`read_canonical_content`] treating a missing canonical as a
/// legitimate zero-byte first-ever-write, BC-1.18.005 EC-004) would
/// silently swallow a genuine path-traversal failure into that same relief
/// — returning `Ok(vec![])`/`Ok(0)` instead of propagating a real I/O
/// error, which `execute_roll` would otherwise map to
/// `ShardRollError::SealWriteFailed` (`E-SHD-001`) exactly as a genuine
/// non-`NotFound` failure already does on Unix.
///
/// The portable fix (research doc §5) is to NOT branch on `raw_os_error()`
/// at all: match `ErrorKind::NotFound` broadly on every platform, then WALK
/// the ancestor chain via [`closest_existing_ancestor_is_directory_or_absent`],
/// which queries `fs::metadata(ancestor).is_dir()` — a property query that
/// is reliable on both Windows and Unix because it directly answers "is
/// traversal blocked by a file?" instead of inferring that from OS-error
/// taxonomy Rust intentionally normalizes away.
fn is_genuinely_missing(err: &io::Error, path: &Path) -> bool {
    if err.kind() != io::ErrorKind::NotFound {
        return false;
    }
    closest_existing_ancestor_is_directory_or_absent(path)
}

/// Resolves `io::ErrorKind::NotFound`'s own internal ambiguity for
/// [`is_genuinely_missing`]. A bare `NotFound` (on Windows; the only
/// platform where this ambiguity is reachable at all, since Unix's
/// `NotADirectory` never gets this far) covers TWO structurally different
/// situations:
///
/// 1. A non-terminal path component EXISTS but is a plain file, not a
///    directory — traversal genuinely cannot continue through it. A real
///    error; must propagate.
/// 2. An ancestor directory simply does not exist YET — a legitimate first
///    write into a freshly-bootstrapped directory tree (e.g. the first
///    `burst-log.md` write into a just-created cycle directory, or a
///    multi-level-deep tree where NONE of the intervening directories have
///    been created yet). Must relieve to `Ok`, exactly like a plain missing
///    file already does.
///
/// Disambiguated by walking UP `path`'s ancestor chain to the FIRST
/// ancestor that actually exists on disk, using `fs::metadata` (which
/// FOLLOWS symlinks/junctions, matching what `File::open`/`fs::read` would
/// have resolved through — NOT `symlink_metadata`), then reporting whether
/// THAT ancestor is a directory: case 1 is a closest-existing-ancestor that
/// is a plain file (blocking further traversal); case 2 is a
/// closest-existing-ancestor that IS a directory (everything below it is
/// simply not created yet), or no existing ancestor at all (the whole tree
/// is unwritten below some real root that does exist). A single-level
/// `path.parent()`-only check would misjudge a MULTI-level-deep traversal
/// failure (a non-terminal component two-or-more levels up blocked by a
/// file, with the immediate parent ALSO consequently absent) as relievable
/// — this walk avoids that by not stopping at the first missing level.
///
/// An empty ancestor component (e.g. `path.parent()` bottoming out at `""`
/// for a relative bare filename) is treated as CWD-relative and therefore
/// non-blocking — returns `true` immediately rather than probing
/// `fs::metadata("")`, which is not a meaningful existence check on any
/// platform.
///
/// Unconditionally compiled and used on EVERY platform (no
/// `#[cfg(windows)]` gate): this is now the WHOLE of
/// [`is_genuinely_missing`]'s discrimination algorithm — there is no
/// remaining `raw_os_error()` branch. On Unix this walk is reached only via
/// `is_genuinely_missing`'s `NotFound` short-circuit for a genuinely-absent
/// path (parent already exists as a directory in the ordinary case), so it
/// resolves in a single `fs::metadata` call there; on Windows it also
/// carries the traversal-through-a-file discrimination.
///
/// NIT-3 (PR #824 pr-review cycle 3; hardened cycle 6 to match the research
/// doc §5 step-2 prescription): the walk distinguishes a `NotFound`
/// ancestor-`metadata` failure (genuine non-existence — strip this level and
/// keep walking up) from any OTHER failure (e.g. `PermissionDenied`/
/// `ERROR_ACCESS_DENIED` on an ancestor) — the latter now STOPS the walk and
/// propagates (`return false`) rather than being conflated with "absent".
/// [`is_genuinely_missing`]'s own early return already filters to
/// `err.kind() == NotFound` before this helper is ever reached, so a
/// `PermissionDenied` on the ORIGINAL failing operation never routes here;
/// this arm only governs an inconclusive `metadata()` call during the
/// ancestor walk itself — which, per the TOCTOU-racy nature of the whole
/// check, is now the strictly more conservative of the two possible
/// resolutions.
fn closest_existing_ancestor_is_directory_or_absent(path: &Path) -> bool {
    let mut cur = path.parent();
    while let Some(candidate) = cur {
        if candidate.as_os_str().is_empty() {
            return true;
        }
        match std::fs::metadata(candidate) {
            Ok(meta) => return meta.is_dir(),
            Err(e) if e.kind() == io::ErrorKind::NotFound => cur = candidate.parent(),
            // A non-NotFound ancestor error (e.g. PermissionDenied) is not
            // genuine absence — stop and propagate rather than claim missing.
            Err(_) => return false,
        }
    }
    // No ancestor exists at all — nothing blocks traversal, the whole tree
    // above `path` simply hasn't been created yet.
    true
}

// ---------------------------------------------------------------------------
// Config surface — `[[shard]]` table (Preconditions 2/3)
// ---------------------------------------------------------------------------

/// Trigger shape a `[[shard]]` config entry declares (Postcondition 8's
/// shape-dispatch field). Read once per entry at config-load time, never
/// inferred from the target path's content or extension (Invariant 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ShardShape {
    /// `stat()`-only byte-size trigger (Postconditions 1-7).
    Flat,
    /// Item-count trigger over a frontmatter `changelog:` array (Postcondition 8).
    FrontmatterChangelogArray,
}

/// The four calibrated cap-formula inputs (Postcondition 4/6). Configuration,
/// never embedded Rust constants (Invariant 4) — this is what makes the F4
/// harness's recalibration a config change, not a code change.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize, Serialize)]
pub struct CapFormulaInputs {
    /// `PRACTICAL_FUEL_CEILING` — today's `DEFAULT_FUEL_CAP`-derived reliably-
    /// completes fuel ceiling (Postcondition 6; PROVISIONAL until F4 lock).
    pub practical_fuel_ceiling: u64,
    /// `WORST_CASE_FUEL_PER_BYTE` — per-plugin measured fuel/byte coefficient,
    /// or the local marginal rate at the largest tested size if superlinear
    /// (Postcondition 7 / EC-006).
    pub worst_case_fuel_per_byte: f64,
    /// `MAX_SINGLE_RECORD_BYTES` — largest single physical line/record margin.
    pub max_single_record_bytes: u64,
    /// `SAFETY_MARGIN` — buffer for shard-index-entry + shard-header overhead.
    pub safety_margin: u64,
}

/// One `[[shard]]` config table entry — one registered sharded artifact.
///
/// `#[serde(deny_unknown_fields)]` is deliberately OMITTED (unlike
/// [`crate::registry::Registry`]'s hooks table) because the `[[shard]]`
/// config location and full field set is explicitly "TBD at F4" per
/// BC-1.18.005's Architecture Anchors — a stricter schema is a follow-up
/// concern once the F4 calibration harness locks the final shape.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ShardEntry {
    /// Artifact stem this entry matches against a tool call's target path
    /// (Precondition 3; Postcondition 1's config-match predicate).
    pub artifact_stem: String,

    /// Path anchor this entry matches against a tool call's target path,
    /// alongside [`ShardEntry::artifact_stem`] (PR #818 cycle-2 review
    /// finding B-1). Required — never `#[serde(default)]` — because a
    /// stem-only match has no directory containment at all: this repository
    /// alone has 426 files sharing the `STATE` stem, 99 sharing `lessons`,
    /// 98 sharing `burst-log`, 79 sharing `BC-INDEX`, and 32 sharing
    /// `decision-log` (finding B-1's own repo-measured collision counts,
    /// mostly under `plugins/vsdd-factory/tests/fixtures/`), so a
    /// stem-only entry would route every one of those unrelated files
    /// through THIS entry's [`validate_entry`]/cap gate as if each were the
    /// one registered artifact — reintroducing, on the path axis, exactly
    /// the blast-radius-scoping defect class the v1.12 MATCH-FIRST
    /// restructure closed on the sibling-entry axis (see
    /// [`find_matching_entry`]'s "Blast-radius scoping" doc reference).
    ///
    /// Repo-root-relative (the idiomatic, clone-portable form config authors
    /// should write) or absolute — [`find_matching_entry`] compares this
    /// field's own path components against the dispatch's target path's
    /// trailing components (a target that ENDS WITH this path, component-
    /// wise, matches — see [`find_matching_entry`]'s own doc comment), so a
    /// relative `artifact_path` naturally matches an absolute target path
    /// arriving from a real tool call without this module ever needing to
    /// join it against a `cwd` (this module performs no I/O and knows no
    /// `cwd` — that stays `executor.rs::shard_cap_precheck`'s concern, which
    /// never needs to resolve one either, precisely because the comparison
    /// is suffix-based). Naming the artifact's own containing directory
    /// (rather than its full path) is also legal — any target nested under
    /// that directory then falls "under" this entry per the same check.
    pub artifact_path: String,

    /// `PRACTICAL_FUEL_CEILING` cap-formula input (Postcondition 4/6).
    pub practical_fuel_ceiling: u64,
    /// `WORST_CASE_FUEL_PER_BYTE` cap-formula input (Postcondition 4/6).
    pub worst_case_fuel_per_byte: f64,
    /// `MAX_SINGLE_RECORD_BYTES` cap-formula input (Postcondition 4/6).
    pub max_single_record_bytes: u64,
    /// `SAFETY_MARGIN` cap-formula input (Postcondition 4/6).
    pub safety_margin: u64,

    /// This entry's own per-plugin `shard_cap_bytes` ceiling. Subject to the
    /// Cross-Validator Minimum Rule (Postcondition 5) across every Cohort B
    /// validator that reads this artifact — this field is the SINGLE
    /// validator's own cap; [`effective_shard_cap_bytes`] combines it with
    /// sibling per-validator caps at call time. Byte-denominated only
    /// (Invariant 2) — never a line-count proxy.
    pub shard_cap_bytes: u64,

    /// Trigger-shape dispatch field (Postcondition 8). `#[serde(default)]`
    /// so an entry that omits it deserializes to `None` at the TOML layer
    /// (rather than a hard parse failure with no artifact-stem context) —
    /// EC-009's fail-loud `HookResult::Error` is enforced by
    /// [`validate_entry`] at entry-match time (BC-1.18.005 v1.12 MATCH-FIRST
    /// restructure), scoped to the single entry [`find_matching_entry`]
    /// resolves for the current dispatch, which can name the offending
    /// `artifact_stem` in the error.
    #[serde(default)]
    pub shape: Option<ShardShape>,

    /// Item-count trigger threshold `N` (`"frontmatter-changelog-array"`
    /// shape only; Postcondition 8). `None` for `"flat"`-shaped entries.
    #[serde(default)]
    pub n: Option<u64>,

    /// Rotation-target config (`"frontmatter-changelog-array"` shape only;
    /// Postcondition 8's rotation-target-config bullet). `None` when
    /// omitted — resolves to `floor(N/2)` via [`resolved_low_water_mark`]
    /// (EC-010; the timing of that resolution is an implementation choice
    /// per BC-1.18.005 v1.8 — `load` itself never eagerly materializes this
    /// default).
    ///
    /// `i64` (not `u64`) so a negative config value round-trips for
    /// EC-011's fail-loud validation instead of failing opaquely at the
    /// TOML/serde layer with no artifact-stem context.
    #[serde(default)]
    pub low_water_mark: Option<i64>,
}

impl ShardEntry {
    /// Extract this entry's four cap-formula inputs (Postcondition 4/6) as a
    /// standalone [`CapFormulaInputs`] value for use with
    /// [`compute_shard_cap_bytes`].
    ///
    /// # GREEN-BY-DESIGN (BC-5.38.002)
    ///
    /// Pure 1:1 field copy into a type constructor — zero branching, no I/O,
    /// no calls to non-trivial helpers, single-expression body. Behavior is
    /// fully determined by the two types' shapes; there is no domain
    /// decision here for a test to exercise non-trivially.
    pub fn cap_formula_inputs(&self) -> CapFormulaInputs {
        CapFormulaInputs {
            practical_fuel_ceiling: self.practical_fuel_ceiling,
            worst_case_fuel_per_byte: self.worst_case_fuel_per_byte,
            max_single_record_bytes: self.max_single_record_bytes,
            safety_margin: self.safety_margin,
        }
    }
}

/// The whole parsed `[[shard]]` config file (Precondition 2).
#[derive(Debug, Clone, Default, PartialEq, Deserialize, Serialize)]
pub struct ShardRegistry {
    #[serde(default, rename = "shard")]
    pub shards: Vec<ShardEntry>,
}

/// Config errors for the `[[shard]]` registry.
///
/// `Io`/`Toml` surface from [`ShardRegistry::load`]'s STRUCTURAL parse step
/// (whole-file, unavoidable — EC-019). `MissingShape` (EC-009),
/// `InvalidLowWaterMark` (EC-011), `CapExceedsFormulaCeiling` (Postcondition
/// 9 / EC-013), `InvalidWorstCaseFuelPerByte` (Postcondition 9's
/// divisor-door closure / EC-015), `MissingN` (Postcondition 8's `N`-presence
/// requirement / EC-016), and `FormulaCeilingSaturated` (Postcondition 9's
/// residual divisor-door closure / EC-017) are the SEMANTIC fail-loud
/// conditions [`validate_entry`] owns — all are NEVER silently defaulted or
/// clamped around.
///
/// **Retimed, BC-1.18.005 v1.12 MATCH-FIRST restructure (F-C1-P6-001):** the
/// six semantic variants above are evaluated at ENTRY-MATCH time (inside
/// [`validate_entry`], called by [`shard_cap_gate_check`] only on the entry
/// [`find_matching_entry`] resolves for the current dispatch), not eagerly
/// for every `[[shard]]` entry at [`ShardRegistry::load`] time — see
/// Postcondition 1's "Blast-radius scoping" ruling and EC-018/EC-019.
#[derive(Debug, Error)]
pub enum ShardConfigError {
    #[error("shard config read failed: {0}")]
    Io(#[from] io::Error),

    #[error("shard config parse failed: {0}")]
    Toml(#[from] toml::de::Error),

    /// EC-009: a `[[shard]]` entry omits `shape` entirely. Fail-loud — this
    /// BC's implementation MUST NOT default silently to either shape.
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" omits the required `shape` \
         field (BC-1.18.005 EC-009). Fail-loud: shape is never defaulted; the dispatch is \
         treated as a config error, never a silent Continue that would leave an oversized \
         artifact unguarded."
    )]
    MissingShape {
        /// The offending entry's `artifact_stem`, so the operator can locate it.
        artifact_stem: String,
    },

    /// EC-022 (PR #818 cycle-4 fresh-context review finding F-1): a
    /// `[[shard]]` entry declares an `artifact_path` that normalizes to ZERO
    /// non-`CurDir` path components — e.g. `"."`, `"./"`, or an empty
    /// string. [`path_falls_under_or_equals`]'s own `CurDir`-filtering (the
    /// B-1/S-2 fix) strips `Component::CurDir` from both the target's and
    /// the registered path's component vectors before comparing; when the
    /// REGISTERED side filters down to an EMPTY vector, its suffix-match leg
    /// (`target[target.len() - 0..] == []`) is vacuously `true` for EVERY
    /// `target_path`, regardless of directory containment. Combined with
    /// [`find_matching_entry`]'s stem-equality filter, this reintroduces
    /// exactly the stem-only, unbounded-blast-radius match B-1's
    /// path-containment leg was built to prevent — any file anywhere in the
    /// repository that merely shares this entry's `artifact_stem` would
    /// silently resolve to this entry. Fail-loud, checked BEFORE any
    /// cap-formula arithmetic (this is a structural config-shape defect, not
    /// a numeric one) — an `artifact_path` must resolve to at least one real
    /// path component; `"."`/`"./"`/empty is a config-authoring mistake, not
    /// a legal "match everything" wildcard.
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" declares artifact_path = \
         \"{artifact_path}\", which normalizes to ZERO non-CurDir path components (BC-1.18.005 \
         EC-022). Fail-loud: an artifact_path of \".\", \"./\", or empty would make \
         path_falls_under_or_equals's suffix-match leg vacuously true for EVERY target path \
         sharing this entry's artifact_stem, reintroducing the exact stem-only unbounded-blast- \
         radius match the B-1 path-containment fix was built to prevent. artifact_path must name \
         at least one real path component (e.g. \".factory/STATE.md\" or \".factory\")."
    )]
    EmptyArtifactPath {
        /// The offending entry's `artifact_stem`, so the operator can locate it.
        artifact_stem: String,
        /// The offending entry's own (degenerate) `artifact_path` value.
        artifact_path: String,
    },

    /// SEC-002 (security review, MEDIUM, CWE-22): a `[[shard]]` entry
    /// declares an `artifact_stem` containing a `/`, `\`, a `..`
    /// path-traversal component, or a NUL byte. Unlike `artifact_path`
    /// (which is only ever used for lexical containment COMPARISON via
    /// [`path_falls_under_or_equals`], never interpolated into a path this
    /// process itself writes), `artifact_stem` IS interpolated unsanitized
    /// into filesystem paths this process constructs and WRITES to — e.g.
    /// the sealed-shard filename (`format!("{artifact_stem}.{seq:04}.md")`)
    /// and the shard-index sibling path
    /// (`format!("{artifact_stem}.shard-index.toml")`, via
    /// [`shard_index_path_for`]). A config-supplied `artifact_stem` of
    /// e.g. `"../../etc/cron.d/evil"` would let a malicious or malformed
    /// `[[shard]]` config steer a sealed-shard write OUTSIDE the canonical
    /// file's own parent directory (`shard_sibling_path`'s `dir.join(...)`
    /// join has no traversal guard of its own — it trusts its `filename`
    /// argument is a single path component, which every OTHER call site
    /// satisfies by construction, but a `[[shard]]` config controls
    /// `artifact_stem` directly). Fail-loud, checked at entry-match time
    /// (mirroring `EmptyArtifactPath`/EC-022's own ordering and posture) —
    /// never silently stripped, escaped, or truncated to "make it safe".
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" contains a `/`, `\\`, `..` \
         path-traversal component, or a NUL byte (security review SEC-002, CWE-22). Fail-loud: \
         artifact_stem is interpolated unsanitized into filesystem paths this process writes \
         (sealed-shard filenames, the shard-index sibling path), so it must never contain a \
         path separator or traversal component."
    )]
    InvalidArtifactStem {
        /// The offending entry's own (unsafe) `artifact_stem` value.
        artifact_stem: String,
    },

    /// EC-011: `low_water_mark >= N` (the `== N` boundary included) or negative.
    /// `low_water_mark == N - 1` is explicitly NOT in this error's scope — see
    /// EC-012 / [`validate_low_water_mark`]'s amortization-advisory path.
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" declares low_water_mark = \
         {low_water_mark}, which violates 0 <= low_water_mark < N (N = {n}) \
         (BC-1.18.005 EC-011). Fail-loud: never silently clamped or defaulted around."
    )]
    InvalidLowWaterMark {
        /// The offending entry's `artifact_stem`, so the operator can locate it.
        artifact_stem: String,
        /// The entry's configured `N` (item-count trigger threshold).
        n: u64,
        /// The entry's configured (invalid) `low_water_mark`.
        low_water_mark: i64,
    },

    /// Postcondition 9 / EC-013: a `[[shard]]` entry declares `shard_cap_bytes`
    /// GREATER than `compute_shard_cap_bytes(entry.cap_formula_inputs())` —
    /// the declared cap exceeds its own formula-derived ceiling. Applies to
    /// EVERY `shape`, not `"flat"`-only (Postcondition 8 already establishes
    /// that `shard_cap_bytes` bounds an artifact's TOTAL byte footprint even
    /// for the `"frontmatter-changelog-array"` shape). Fail-loud: NEVER
    /// silently accepted, NEVER silently clamped down to the computed
    /// ceiling — mirrors EC-009/EC-011's established fail-loud-at-
    /// entry-match-time posture for this same config surface. The `==`
    /// boundary is legal
    /// (Postcondition 4's `<=` comparison is inclusive — EC-002 precedent).
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" declares shard_cap_bytes = \
         {shard_cap_bytes}, which EXCEEDS its own formula-derived ceiling \
         compute_shard_cap_bytes(inputs) = {computed_ceiling} (BC-1.18.005 Postcondition 9 / \
         EC-013). Fail-loud: never silently accepted, never silently clamped down to the \
         computed ceiling."
    )]
    CapExceedsFormulaCeiling {
        /// The offending entry's `artifact_stem`, so the operator can locate it.
        artifact_stem: String,
        /// The entry's own declared `shard_cap_bytes`.
        shard_cap_bytes: u64,
        /// `compute_shard_cap_bytes(entry.cap_formula_inputs())` — the ceiling
        /// the entry's OWN four formula inputs justify.
        computed_ceiling: u64,
    },

    /// EC-015 (BC-1.18.005 v1.10, Postcondition 9's "divisor-door" closure
    /// sub-bullet): a `[[shard]]` entry declares `worst_case_fuel_per_byte`
    /// as non-finite (`NaN`, `inf`, `-inf`) or `<= 0.0` (including `0.0`
    /// itself — the divisor-door case that saturates
    /// `compute_shard_cap_bytes`'s division to near-`u64::MAX`, defeating
    /// the `CapExceedsFormulaCeiling` check above for ANY declared
    /// `shard_cap_bytes`). Fail-loud: rejected BEFORE the cap-vs-formula
    /// comparison is even attempted. `practical_fuel_ceiling`,
    /// `max_single_record_bytes`, and `safety_margin` need no analogous
    /// check — all three are `u64`-typed (TOML excludes non-finite/negative
    /// values for them) and a degenerate `0` on any of them drives the
    /// ceiling toward the SAFE direction (0 via `saturating_sub`), never the
    /// exploitable-widening direction only `worst_case_fuel_per_byte`'s
    /// `f64` type creates.
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" declares \
         worst_case_fuel_per_byte = {worst_case_fuel_per_byte}, which must be finite and > 0.0 \
         (BC-1.18.005 EC-015). Fail-loud: never silently substituted with a default rate, and \
         the cap-vs-formula comparison (Postcondition 9) is never silently skipped for this \
         entry."
    )]
    InvalidWorstCaseFuelPerByte {
        /// The offending entry's `artifact_stem`, so the operator can locate it.
        artifact_stem: String,
        /// The entry's configured (invalid) `worst_case_fuel_per_byte`.
        worst_case_fuel_per_byte: f64,
    },

    /// EC-016 (BC-1.18.005 v1.10, Postcondition 8's presence requirement for
    /// `N`; retimed to entry-match time by the v1.12 MATCH-FIRST restructure):
    /// a `"frontmatter-changelog-array"`-shaped entry omits the required `N`
    /// item-count trigger threshold. Fail-loud, evaluated BEFORE
    /// `low_water_mark` is examined, so an entry that also declares an
    /// invalid `low_water_mark` is never silently accepted merely because
    /// `N` happened to be absent.
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" declares \
         shape = \"frontmatter-changelog-array\" but omits the required `n` item-count trigger \
         threshold (BC-1.18.005 EC-016). Fail-loud: caught at entry-match time (validate_entry), \
         not deferred to the first gate-time write against the artifact."
    )]
    MissingN {
        /// The offending entry's `artifact_stem`, so the operator can locate it.
        artifact_stem: String,
    },

    /// EC-017 (BC-1.18.005 v1.11, Postcondition 9's "Residual divisor-door
    /// closure" sub-paragraph): a `[[shard]]` entry declares a
    /// `worst_case_fuel_per_byte` that is finite and `> 0.0` (passes
    /// EC-015's guard above) but is so small (e.g. `1e-300`) that the RAW,
    /// pre-cast division `practical_fuel_ceiling as f64 /
    /// worst_case_fuel_per_byte` is itself non-finite or `>= u64::MAX as
    /// f64` — i.e. the computed ceiling saturates the subsequent
    /// `.floor() as u64` cast to (or beyond) `u64::MAX`, even though the
    /// divisor itself is legal under EC-015. Distinct from
    /// `InvalidWorstCaseFuelPerByte`, which catches an illegal `<=
    /// 0.0`/non-finite DIVISOR itself, not a saturated RESULT arising from
    /// an otherwise-legal tiny-positive divisor. Fail-loud: rejected BEFORE
    /// the `CapExceedsFormulaCeiling` comparison is even attempted — a
    /// saturated ceiling cannot meaningfully bound any declared
    /// `shard_cap_bytes`, so it is never silently accepted and never
    /// silently substituted with a fallback ceiling.
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" declares \
         practical_fuel_ceiling = {practical_fuel_ceiling} and worst_case_fuel_per_byte = \
         {worst_case_fuel_per_byte}, whose raw division saturates the formula's ceiling \
         computation to (or beyond) u64::MAX (BC-1.18.005 EC-017, residual divisor-door \
         closure). Fail-loud: a saturated ceiling cannot meaningfully bound any declared \
         shard_cap_bytes, so it is never silently accepted and never silently substituted with \
         a fallback ceiling."
    )]
    FormulaCeilingSaturated {
        /// The offending entry's `artifact_stem`, so the operator can locate it.
        artifact_stem: String,
        /// The entry's configured `practical_fuel_ceiling`.
        practical_fuel_ceiling: u64,
        /// The entry's configured (legal-but-degenerate) `worst_case_fuel_per_byte`.
        worst_case_fuel_per_byte: f64,
    },

    /// PR #818 fix-burst finding m2: a `"frontmatter-changelog-array"`-shaped
    /// entry declares `n = 0`. Two compounding problems make this a
    /// fail-loud condition rather than a merely-unusual-but-legal config:
    /// (1) [`item_count_trigger_fires`]'s `current_item_count.saturating_add(1)
    /// > n` is `true` for EVERY `current_item_count >= 0` when `n == 0`
    /// (`0 + 1 > 0`), so the item-count trigger fires unconditionally on
    /// literally the first dispatch against the artifact — permanently, not
    /// just as an edge case; (2) `n = 0` makes ANY EXPLICIT `low_water_mark`
    /// value unsatisfiable, since [`validate_low_water_mark`]'s `0 <=
    /// low_water_mark < N` constraint has no legal value when `N = 0`
    /// (`low_water_mark < 0` is impossible for `low_water_mark: i64 >= 0`,
    /// and `low_water_mark >= 0` always violates `< 0`). Fail-loud, checked
    /// immediately after [`MissingN`]'s presence check and before
    /// `low_water_mark` is examined — mirrors EC-016's own ordering
    /// rationale (a config that is invalid for `n` itself must not be
    /// masked by, or reported as, a downstream `low_water_mark` failure).
    #[error(
        "[[shard]] entry for artifact_stem = \"{artifact_stem}\" declares shape = \
         \"frontmatter-changelog-array\" with n = 0, which makes the item-count trigger fire \
         unconditionally on every dispatch (current_item_count + 1 > 0 is always true for any \
         non-negative current_item_count) and makes ANY explicit low_water_mark value \
         unsatisfiable (0 <= low_water_mark < N requires N >= 1) (PR #818 fix-burst finding m2). \
         Fail-loud: n must be >= 1, never silently accepted at 0."
    )]
    ZeroItemCountThreshold {
        /// The offending entry's `artifact_stem`, so the operator can locate it.
        artifact_stem: String,
    },

    /// PR #818 fix-burst finding N-2: two (or more) `[[shard]]` entries
    /// declare the SAME `artifact_stem` AND both match the current
    /// dispatch's target path per [`path_falls_under_or_equals`] (PR #818
    /// cycle-2 review finding B-1 added the path leg — two entries sharing a
    /// stem but naming DIFFERENT `artifact_path`s that this dispatch's
    /// target does not simultaneously satisfy are legitimate, not a
    /// misconfiguration; only two entries that would BOTH resolve for the
    /// SAME dispatch are). [`find_matching_entry`] previously resolved a
    /// stem-only collision silently to whichever entry appeared first in the
    /// config file, with no diagnostic — an operator adding a second entry
    /// for an already-registered artifact (e.g. a copy-paste typo, or two
    /// independent config fragments merged without deduplication) would
    /// have their SECOND entry's cap-formula inputs / shape / `n` /
    /// `low_water_mark` silently ignored with no error at all, which is
    /// exactly the class of silently-swallowed misconfiguration this BC's
    /// fail-loud posture exists to prevent. Fail-loud, scoped to the current
    /// dispatch's own target (consistent with this BC's v1.12 MATCH-FIRST
    /// blast-radius scoping — an unrelated duplicate elsewhere in the config
    /// for a DIFFERENT artifact this dispatch does not target is never
    /// observed).
    #[error(
        "[[shard]] config declares MULTIPLE entries for artifact_stem = \"{artifact_stem}\" — \
         the second and any subsequent entries would be silently ignored by a plain first-match \
         lookup (PR #818 fix-burst finding N-2). Fail-loud: duplicate artifact_stem entries are \
         never silently resolved to \"whichever appears first\"."
    )]
    DuplicateArtifactStem {
        /// The `artifact_stem` declared by more than one `[[shard]]` entry.
        artifact_stem: String,
    },
}

/// Fail-loud config-load errors surface to the dispatcher's PreToolUse
/// handling path as `HookResult::Error` (EC-009 / EC-011's posture).
///
/// # WIRING-EXEMPT (BC-5.38.003)
///
/// `From<T>` blanket delegation to a single `Display`-forwarding call — the
/// canonical WIRING-EXEMPT example (`Self(value.into())`-shaped). No domain
/// decision: the error's own `Display` impl (via `thiserror`) already
/// carries the full, artifact-stem-scoped diagnostic text.
impl From<ShardConfigError> for HookResult {
    fn from(err: ShardConfigError) -> Self {
        HookResult::Error {
            message: err.to_string(),
        }
    }
}

impl ShardRegistry {
    /// Load a `[[shard]]` config file from disk.
    ///
    /// **STRUCTURAL TOML deserialization ONLY** (BC-1.18.005 v1.12
    /// MATCH-FIRST restructure, F-C1-P6-001, Postcondition 1's "Blast-radius
    /// scoping" ruling) — this function no longer runs any SEMANTIC
    /// per-entry validation loop. It fails (`ShardConfigError::Io`/`Toml`)
    /// ONLY when the file cannot be read, or `toml::from_str` cannot
    /// deserialize the whole `Vec<ShardEntry>` at all — invalid TOML syntax,
    /// or any entry omitting a non-`Option`-typed field required for
    /// `toml::from_str` to succeed (`artifact_stem`, `artifact_path`,
    /// `shard_cap_bytes`, or any of the four `cap_formula_inputs` fields).
    /// This is the ONE
    /// residual, unavoidable whole-file blast-radius case (EC-019) — it is
    /// inherent to TOML's whole-file grammar (a `Vec<ShardEntry>` cannot
    /// partially deserialize), not a validation-eagerness design choice.
    ///
    /// Every SEMANTIC check this function used to run inline (EC-009's
    /// `shape` presence, EC-015/EC-017's `worst_case_fuel_per_byte`
    /// divisor-door guards, Postcondition 9/EC-013's cap-vs-formula
    /// comparison, EC-016's `N` presence, EC-011/EC-012's `low_water_mark`
    /// range/advisory) now lives in [`validate_entry`], called by
    /// [`shard_cap_gate_check`] ONLY on the entry [`find_matching_entry`]
    /// resolves for the current dispatch — never eagerly, for every entry,
    /// regardless of match (see Postcondition 1's "Blast-radius scoping"
    /// sub-paragraph and EC-018).
    pub fn load(path: &Path) -> Result<Self, ShardConfigError> {
        let text = std::fs::read_to_string(path)?;
        let parsed: Self = toml::from_str(&text)?;
        Ok(parsed)
    }
}

/// Validate a single `[[shard]]` entry's SEMANTIC config surface
/// (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001, Postcondition
/// 1's "Blast-radius scoping" ruling).
///
/// Callers (namely [`shard_cap_gate_check`]) MUST call this ONLY on the
/// entry [`find_matching_entry`] resolves for the current dispatch's target
/// path — NEVER eagerly across every entry in a `[[shard]]` config,
/// regardless of match (that eager-whole-config posture is the pre-v1.12 bug
/// this restructure fixes — see EC-018). This function itself performs no
/// I/O and reads no sibling entries — it is pure over the single `entry` it
/// is given.
///
/// Checks, in order (every check after the first assumes the ones before it
/// passed):
/// 1. `shape` MUST be present (fail-loud [`ShardConfigError::MissingShape`]
///    — EC-009).
/// 2. `artifact_path` MUST normalize to at least one non-`CurDir` path
///    component (fail-loud [`ShardConfigError::EmptyArtifactPath`] — EC-022,
///    PR #818 cycle-4 review finding F-1) — checked BEFORE any cap-formula
///    arithmetic, since this is a structural config-shape defect
///    independent of the numeric checks below: an `artifact_path` of `"."`,
///    `"./"`, or empty would make [`path_falls_under_or_equals`]'s
///    suffix-match leg vacuously true for every target sharing this entry's
///    `artifact_stem`.
/// 3. `artifact_stem` MUST NOT contain a `/`, `\`, a `..` path-traversal
///    component, or a NUL byte (fail-loud
///    [`ShardConfigError::InvalidArtifactStem`] — security review SEC-002,
///    CWE-22) — checked alongside check 2 above, since `artifact_stem` is
///    interpolated unsanitized into filesystem paths this process itself
///    WRITES to (sealed-shard filenames, the shard-index sibling path),
///    unlike `artifact_path` which is only ever used for lexical
///    containment comparison.
/// 4. `worst_case_fuel_per_byte` MUST be finite and strictly positive
///    (fail-loud [`ShardConfigError::InvalidWorstCaseFuelPerByte`] —
///    EC-015's "divisor-door" closure) — checked BEFORE
///    `compute_shard_cap_bytes` is ever called for this entry, since a
///    `0.0`/non-finite divisor would otherwise saturate the computed
///    ceiling toward `u64::MAX`, defeating check 6 below for ANY declared
///    `shard_cap_bytes`.
/// 5. The RAW `practical_fuel_ceiling as f64 / worst_case_fuel_per_byte`
///    division result MUST be finite and `< u64::MAX as f64` (fail-loud
///    [`ShardConfigError::FormulaCeilingSaturated`] — EC-017's residual
///    divisor-door closure) — a legal-but-tiny-positive divisor can still
///    saturate the computed ceiling even though it passes check 4.
/// 6. `shard_cap_bytes` MUST NOT exceed
///    `compute_shard_cap_bytes(entry.cap_formula_inputs())` (fail-loud
///    [`ShardConfigError::CapExceedsFormulaCeiling`] — Postcondition 9 /
///    EC-013; the `==` boundary is inclusive, mirroring EC-002's precedent
///    for the per-write trigger) — applies to EVERY `shape`, not
///    `"flat"`-only.
/// 7. For a `"frontmatter-changelog-array"`-shaped entry ONLY: `n` MUST be
///    present (fail-loud [`ShardConfigError::MissingN`] — EC-016), checked
///    BEFORE `low_water_mark` is examined, so an entry missing `n` AND
///    declaring an out-of-range `low_water_mark` is never silently accepted
///    merely because `n` was absent (see EC-016's ordering vector).
/// 8. For a `"frontmatter-changelog-array"`-shaped entry with a PRESENT `n`
///    ONLY: `n` MUST be `>= 1` (fail-loud
///    [`ShardConfigError::ZeroItemCountThreshold`] — PR #818 fix-burst
///    finding m2), checked immediately after check 7 and BEFORE
///    `low_water_mark` is examined — `n = 0` makes the item-count trigger
///    fire unconditionally AND makes any explicit `low_water_mark` value
///    unsatisfiable.
/// 9. For a `"frontmatter-changelog-array"`-shaped entry with an EXPLICIT
///    `low_water_mark` ONLY: `0 <= low_water_mark < N` (fail-loud
///    [`ShardConfigError::InvalidLowWaterMark`] — EC-011; `N-1` is a VALID
///    boundary value, never routed to this error — see EC-012), and a
///    legal-but-poorly-amortizing value in `(floor(N/2), N)` (up to and
///    including `N-1`) succeeds (`Ok(())`) but emits a non-fatal
///    `tracing::warn!` amortization advisory (EC-012) — see
///    [`validate_low_water_mark`]. An OMITTED `low_water_mark` resolves to
///    `floor(N/2)` (EC-010) — see [`resolved_low_water_mark`] — and never
///    runs this check at all (that default is, by construction, never
///    poorly amortizing and never invalid).
pub fn validate_entry(entry: &ShardEntry) -> Result<(), ShardConfigError> {
    // EC-009: `shape` is fail-loud-required, never silently defaulted.
    let shape = entry.shape.ok_or_else(|| ShardConfigError::MissingShape {
        artifact_stem: entry.artifact_stem.clone(),
    })?;

    // EC-022 (PR #818 cycle-4 review finding F-1): `artifact_path` MUST
    // normalize to at least one non-CurDir path component. Checked BEFORE
    // any cap-formula arithmetic — this is a structural config-shape defect,
    // not a numeric one. `path_falls_under_or_equals` filters `CurDir` out
    // of both the target's and the registered path's component vectors
    // (the B-1/S-2 fix); if the REGISTERED side normalizes to an empty
    // vector, its suffix-match leg (`target[target.len() - 0..] ==
    // []`) is vacuously true for every target_path, regardless of
    // directory containment. Combined with find_matching_entry's
    // stem-equality filter, this would let this entry silently match ANY
    // file anywhere that merely shares its artifact_stem, reintroducing the
    // exact stem-only, unbounded-blast-radius match B-1's path-containment
    // leg was built to prevent.
    if !Path::new(&entry.artifact_path)
        .components()
        .any(|c| c != std::path::Component::CurDir)
    {
        return Err(ShardConfigError::EmptyArtifactPath {
            artifact_stem: entry.artifact_stem.clone(),
            artifact_path: entry.artifact_path.clone(),
        });
    }

    // SEC-002 (security review, MEDIUM, CWE-22): `artifact_stem` MUST NOT
    // contain a `/`, `\`, a `..` path-traversal component, or a NUL byte.
    // Unlike `artifact_path` (validated above but only ever used for
    // lexical containment COMPARISON via `path_falls_under_or_equals`),
    // `artifact_stem` IS interpolated unsanitized into filesystem paths
    // this process itself constructs and WRITES to — the sealed-shard
    // filename (`format!("{artifact_stem}.{seq:04}.md")`) and the
    // shard-index sibling path (`shard_index_path_for`,
    // `format!("{artifact_stem}.shard-index.toml")`). A config-supplied
    // `artifact_stem` of e.g. `"../../etc/cron.d/evil"` would let a
    // malicious or malformed `[[shard]]` config steer a sealed-shard write
    // outside the canonical file's own parent directory — `shard_sibling_path`'s
    // `dir.join(filename)` has no traversal guard of its own; it trusts its
    // `filename` argument is a single path component, which every OTHER
    // call site satisfies by construction but a `[[shard]]` config controls
    // directly for `artifact_stem`. Fail-loud: never silently stripped,
    // escaped, or truncated to "make it safe".
    if entry.artifact_stem.contains('/')
        || entry.artifact_stem.contains('\\')
        || entry.artifact_stem.contains("..")
        || entry.artifact_stem.contains('\0')
    {
        return Err(ShardConfigError::InvalidArtifactStem {
            artifact_stem: entry.artifact_stem.clone(),
        });
    }

    // EC-015 (Postcondition 9's "divisor-door" closure): validate BEFORE
    // compute_shard_cap_bytes is called for this entry — a 0.0 divisor
    // makes the internal division yield +inf, whose saturating `as u64`
    // cast is u64::MAX, defeating the cap-vs-formula comparison below for
    // ANY declared shard_cap_bytes. A NaN divisor is likewise non-finite
    // and rejected regardless of which direction its own degenerate
    // arithmetic happens to saturate. practical_fuel_ceiling,
    // max_single_record_bytes, and safety_margin need no analogous check
    // (all u64-typed; a degenerate 0 on any of them drives the ceiling
    // toward the safe direction, never the exploitable one).
    if !entry.worst_case_fuel_per_byte.is_finite() || entry.worst_case_fuel_per_byte <= 0.0 {
        return Err(ShardConfigError::InvalidWorstCaseFuelPerByte {
            artifact_stem: entry.artifact_stem.clone(),
            worst_case_fuel_per_byte: entry.worst_case_fuel_per_byte,
        });
    }

    // EC-017 (Postcondition 9's "Residual divisor-door closure"): the
    // EC-015 guard above is NECESSARY but INSUFFICIENT — a legal
    // tiny-positive divisor (e.g. 1e-300) passes it yet still drives the
    // RAW, pre-cast division result to a value so large that
    // compute_shard_cap_bytes's subsequent `.floor() as u64` cast saturates
    // to u64::MAX, defeating the CapExceedsFormulaCeiling comparison below
    // for ANY declared shard_cap_bytes exactly as EC-015's own
    // divisor-door does. Evaluate the SAME raw f64 quantity
    // compute_shard_cap_bytes computes internally, BEFORE that comparison
    // is attempted, and reject the entry if it is non-finite or at/beyond
    // the saturation boundary of the subsequent lossy cast (u64::MAX as
    // f64).
    let raw_ceiling_division = entry.practical_fuel_ceiling as f64 / entry.worst_case_fuel_per_byte;
    if !raw_ceiling_division.is_finite() || raw_ceiling_division >= u64::MAX as f64 {
        return Err(ShardConfigError::FormulaCeilingSaturated {
            artifact_stem: entry.artifact_stem.clone(),
            practical_fuel_ceiling: entry.practical_fuel_ceiling,
            worst_case_fuel_per_byte: entry.worst_case_fuel_per_byte,
        });
    }

    // Postcondition 9 / EC-013: applies to EVERY entry regardless of
    // `shape` — Postcondition 8 already establishes that shard_cap_bytes
    // bounds an artifact's TOTAL byte footprint as a whole-artifact
    // concern even for the "frontmatter-changelog-array" shape, so this is
    // NOT a "flat"-shape-only check.
    let computed_ceiling = compute_shard_cap_bytes(&entry.cap_formula_inputs());
    if entry.shard_cap_bytes > computed_ceiling {
        return Err(ShardConfigError::CapExceedsFormulaCeiling {
            artifact_stem: entry.artifact_stem.clone(),
            shard_cap_bytes: entry.shard_cap_bytes,
            computed_ceiling,
        });
    }

    // Postcondition 8's rotation-target-config bullet / EC-010/EC-011/
    // EC-012 apply to the item-count shape's OPTIONAL explicit
    // low_water_mark only — an omitted value resolves to floor(N/2)
    // (EC-010) without ever running the fail-loud/advisory check below
    // (that default is, by construction, never poorly amortizing and never
    // invalid).
    if shape == ShardShape::FrontmatterChangelogArray {
        // EC-016 (Postcondition 8's `N`-presence requirement): required for
        // this shape, evaluated BEFORE low_water_mark is examined — a
        // destructure-and-short-circuit pattern that only validates
        // low_water_mark when BOTH `n` and `low_water_mark` are `Some`
        // would silently let an entry that ALSO declares an out-of-range
        // low_water_mark pass validation entirely when `n` is `None`.
        // Requiring `n`'s presence as an unconditional, shape-scoped check
        // first closes that gap structurally: every FrontmatterChangelogArray
        // entry now either (a) fails loud here on a missing `n`, or (b) has
        // `n` present, in which case the existing low_water_mark
        // numeric-range validation below runs exactly as before.
        let Some(n) = entry.n else {
            return Err(ShardConfigError::MissingN {
                artifact_stem: entry.artifact_stem.clone(),
            });
        };

        // PR #818 fix-burst finding m2: n = 0 makes the item-count trigger
        // fire unconditionally AND makes any explicit low_water_mark value
        // unsatisfiable (0 <= low_water_mark < 0 has no solution) — checked
        // immediately after n's presence, before low_water_mark is examined,
        // mirroring EC-016's own ordering rationale.
        if n == 0 {
            return Err(ShardConfigError::ZeroItemCountThreshold {
                artifact_stem: entry.artifact_stem.clone(),
            });
        }

        if let Some(low_water_mark) = entry.low_water_mark {
            let fires_advisory = validate_low_water_mark(&entry.artifact_stem, n, low_water_mark)?;
            if fires_advisory {
                let default_low_water_mark = n / 2;
                tracing::warn!(
                    artifact_stem = %entry.artifact_stem,
                    n,
                    low_water_mark,
                    amortization_factor = n.saturating_sub(low_water_mark as u64),
                    default_low_water_mark,
                    default_amortization_factor = n.saturating_sub(default_low_water_mark),
                    "BC-1.18.005 EC-012: configured low_water_mark amortizes rotation worse \
                     than the recommended default floor(N/2); entry validation still succeeds \
                     (non-fatal advisory only)"
                );
            }
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Postcondition 1 / Invariant 3 — config-match-before-stat() zero-cost bypass
// ---------------------------------------------------------------------------

/// Find the `[[shard]]` config entry (if any) matching `target_path`.
///
/// **Fallible, not a plain lookup** (PR #818 cycle-2 review finding N-2):
/// the outer `Result` is a NORMAL, non-exceptional return path callers MUST
/// handle — an `Err(ShardConfigError::DuplicateArtifactStem)` fires when
/// more than one `[[shard]]` entry would resolve for this dispatch's target
/// (see that variant's own doc comment); this is NOT a `panic!`-only-on-bug
/// escape hatch the way a function named `resolve_matching_entry` or similar
/// might read. Callers pattern-match all three shapes: `Ok(Some(entry))`
/// (matched), `Ok(None)` (no match — the zero-cost-bypass case), and
/// `Err(_)` (ambiguous match, fail loud).
///
/// MUST be called — and return — before any `stat()` call AGAINST THE
/// TARGET ARTIFACT ITSELF (Invariant 3; Postcondition 1's target-artifact-
/// scoped zero-cost bypass). Corrected framing (PR #818 fix-burst finding
/// B4): this function's own cost (a linear scan of the ALREADY-structurally-
/// parsed `ShardRegistry`) is genuinely negligible, but callers should not
/// read this as "the whole dispatch pays zero added latency" —
/// [`ShardRegistry::load`] itself performs a real, unavoidable-per-dispatch
/// structural TOML parse of the config file whenever it exists (BC-1.18.005
/// v1.12's own "some parse per dispatch is unavoidable" acknowledgment),
/// BEFORE this function ever runs. What IS zero-cost for the ~99% of
/// `Edit`/`Write`/`MultiEdit` calls that do not target a sharded artifact is
/// specifically: no `stat()`/content read of the TARGET artifact, and no
/// `[[shard]]` entry semantic validation ([`validate_entry`]) — never the
/// TOML config parse itself.
///
/// PR #818 fix-burst finding N-2: fail-loud
/// [`ShardConfigError::DuplicateArtifactStem`] when MORE THAN ONE `[[shard]]`
/// entry declares the SAME `artifact_stem` AND matches `target_path` on
/// [`path_falls_under_or_equals`] (see that function's doc comment; PR #818
/// cycle-2 review finding B-1 added the path leg to this predicate, so two
/// entries sharing a stem but governing genuinely different artifacts —
/// e.g. two different `STATE.md` files in two different directories — are
/// no longer misreported as a duplicate; only two entries that would BOTH
/// resolve for the SAME dispatch are) — a silent first-match resolution
/// would let a second, differently-configured entry for an
/// already-registered artifact go completely unnoticed. Scoped to the
/// dispatch's own target, consistent with this BC's v1.12 MATCH-FIRST
/// blast-radius doctrine: a duplicate for a DIFFERENT artifact this
/// dispatch does not target is never observed or reported by this call.
pub fn find_matching_entry<'a>(
    registry: &'a ShardRegistry,
    target_path: &Path,
) -> Result<Option<&'a ShardEntry>, ShardConfigError> {
    // `Path::file_stem()` is a pure string operation over the path's own
    // components — it never touches the filesystem, so this comparison is
    // free to run before any stat() call (Invariant 3).
    let Some(stem) = target_path.file_stem().and_then(|s| s.to_str()) else {
        return Ok(None);
    };
    // B-1: stem alone is NOT sufficient — see path_falls_under_or_equals's
    // doc comment for the repo-measured collision counts this additional
    // path-containment leg exists to close.
    let mut matches = registry.shards.iter().filter(|entry| {
        entry.artifact_stem == stem
            && path_falls_under_or_equals(target_path, Path::new(&entry.artifact_path))
    });
    let Some(first) = matches.next() else {
        return Ok(None);
    };
    // N-2: a second entry sharing the same artifact_stem AND matching this
    // dispatch's target path is a fail-loud config defect, not a silent
    // "first one wins" resolution.
    if matches.next().is_some() {
        return Err(ShardConfigError::DuplicateArtifactStem {
            artifact_stem: stem.to_string(),
        });
    }
    Ok(Some(first))
}

/// `true` iff `target_path` IS the artifact `registered_path` denotes, or is
/// lexically nested under it as a directory (PR #818 cycle-2 review finding
/// B-1's path-containment requirement — closes the stem-only collision
/// surface [`find_matching_entry`] previously exposed: this repository alone
/// has 426 files sharing the `STATE` stem, 99 sharing `lessons`, 98 sharing
/// `burst-log`, 79 sharing `BC-INDEX`, and 32 sharing `decision-log`, mostly
/// under `plugins/vsdd-factory/tests/fixtures/`).
///
/// Purely lexical, component-wise comparison — performs NO filesystem I/O
/// (never `canonicalize()`, which requires the path to exist, and would
/// violate Invariant 3's "no stat() before config-match" ordering anyway).
/// Two comparisons, either of which is sufficient:
///
/// 1. **Suffix match** — `registered_path`'s components form a TRAILING run
///    of `target_path`'s own components. This is what makes a
///    repo-root-relative `artifact_path` (the idiomatic, clone-portable form
///    — e.g. `.factory/STATE.md`) match an ABSOLUTE `target_path` arriving
///    from a real tool call (e.g. `/Users/.../repo/.factory/STATE.md`)
///    without this function — or any caller — ever needing to join it
///    against a `cwd`. When `registered_path` names the exact artifact
///    (the common case), this is the ONLY leg that can fire, and at full
///    length it degenerates to an exact-path match.
/// 2. **Prefix match** — `registered_path`'s components form a LEADING run
///    of `target_path`'s own components, with at least one further nested
///    component (a bare prefix-equal-length case is already covered by leg
///    1 above). This is the "falls under" case: `registered_path` names a
///    containing directory, in the SAME coordinate space as `target_path`
///    (both repo-root-relative, or both absolute against the same root),
///    and `target_path` is some file nested inside it.
fn path_falls_under_or_equals(target_path: &Path, registered_path: &Path) -> bool {
    // `Path::components()` retains a leading `CurDir` component (`.`) when the
    // path was written with an explicit `./` prefix (e.g. `./.factory/x.md`),
    // but drops it when the path is written without one (`.factory/x.md`).
    // Both forms name the SAME location, so a raw component-wise comparison
    // would silently fail to match the `./`-prefixed form against an
    // otherwise-identical target/registered path pair — exactly the kind of
    // silently-swallowed misconfiguration this module is designed to fail
    // loud on everywhere else. Filter `CurDir` out of both vectors first so
    // the two spellings normalize to the same component sequence.
    let target: Vec<_> = target_path
        .components()
        .filter(|c| *c != std::path::Component::CurDir)
        .collect();
    let registered: Vec<_> = registered_path
        .components()
        .filter(|c| *c != std::path::Component::CurDir)
        .collect();

    // SEC-003 (security review, LOW, CWE-22): this comparison is purely
    // lexical (never `canonicalize()`, per Invariant 3's "no stat() before
    // config-match" ordering) — a `Component::ParentDir` (`..`) component
    // in EITHER operand is therefore never normalized away the way a real
    // filesystem traversal would resolve it, and a naive component-wise
    // suffix/prefix comparison could be fooled by one into a match that
    // does not correspond to any real containment relationship (e.g.
    // `target = "a/b/../c"` lexically "suffix-matches" `registered = "c"`
    // without actually being nested under whatever `registered_path` names
    // on disk). Currently inert in practice — this dispatcher hook only
    // ever sees a `target_path` already authorized by Claude Code's own
    // tool-approval layer — but this is a latent normalization gap in a
    // security-relevant path-containment primitive, so reject any operand
    // containing a `ParentDir` component outright rather than let it
    // participate in the lexical comparison at all.
    if target
        .iter()
        .chain(registered.iter())
        .any(|c| *c == std::path::Component::ParentDir)
    {
        return false;
    }

    let suffix_match = registered.len() <= target.len()
        && target[target.len() - registered.len()..] == registered[..];
    let prefix_match =
        registered.len() < target.len() && target[..registered.len()] == registered[..];

    suffix_match || prefix_match
}

// ---------------------------------------------------------------------------
// Postcondition 4/6/7 — byte-size-denominated cap formula
// ---------------------------------------------------------------------------

/// Compute the byte-size cap ceiling from the four calibrated inputs
/// (Postcondition 4): `shard_cap_bytes <= (PRACTICAL_FUEL_CEILING /
/// WORST_CASE_FUEL_PER_BYTE) - MAX_SINGLE_RECORD_BYTES - SAFETY_MARGIN`.
///
/// Byte-denominated only (Invariant 2) — never a line-count proxy. VP-116 is
/// ALLOCATED (kani-proof method) to exercise this formula's arithmetic over
/// symbolic inputs for overflow/underflow safety (Postcondition 7 /
/// EC-006/EC-007's superlinear-rate and ceiling-change re-derivation cases
/// feed `worst_case_fuel_per_byte` and `practical_fuel_ceiling` respectively
/// — this function itself is rate-agnostic; the caller supplies the
/// already-corrected inputs). **Corrected tense (PR #818 fix-burst finding
/// n1):** per VP-INDEX.md, VP-116 is `status: draft`,
/// `feasible-pending-harness` — its formal-verification (kani) proof scope
/// is allocated to the Phase 6 / formal-hardening pipeline stage and has NOT
/// yet run against this function in this cluster. This paragraph describes
/// the SCOPED FUTURE obligation, not a completed proof; `cargo test` unit
/// coverage of this function's boundary/overflow cases in this file's own
/// `#[cfg(test)] mod tests` is what is actually verified today.
pub fn compute_shard_cap_bytes(inputs: &CapFormulaInputs) -> u64 {
    let fuel_budget_bytes =
        (inputs.practical_fuel_ceiling as f64 / inputs.worst_case_fuel_per_byte).floor();
    // Defensive saturating arithmetic: the formula's provisional inputs
    // (Postcondition 6) always yield a comfortably positive result, but a
    // pathological config (e.g. an oversized MAX_SINGLE_RECORD_BYTES +
    // SAFETY_MARGIN pair) must never underflow/panic — floor at 0.
    (fuel_budget_bytes as u64)
        .saturating_sub(inputs.max_single_record_bytes)
        .saturating_sub(inputs.safety_margin)
}

// ---------------------------------------------------------------------------
// Postcondition 5 — Cross-Validator Minimum Rule
// ---------------------------------------------------------------------------

/// Effective `shard_cap_bytes` for a multi-reader artifact: the MINIMUM
/// across every Cohort B validator's own per-plugin cap (Postcondition 5).
/// A single global cap across all mechanism-A artifacts MUST NOT be
/// substituted — it would be needlessly conservative for artifacts fewer
/// validators read.
///
/// `per_validator_caps` is the set of `cap_for(validator)` values for every
/// Cohort B validator that reads the target artifact (per ADR-047 §8a's
/// Cohort B table) — NOT every validator in the system.
///
/// **Deliberately has NO live caller in [`shard_cap_gate_check`]** — this is
/// an authoring-time / F4-calibration-harness helper, not a per-write
/// runtime re-derivation, per an explicit BC-1.18.005 v1.10 product-owner
/// adjudication (finding F-C1-P4-003, "Reading (A) CONFIG-TIME/HARNESS-
/// HELPER adjudicated over Reading (B) RUNTIME"): ADR-051 §Decision 2 places
/// the Cross-Validator Minimum Rule inside the F4 calibration harness's own
/// design, `ShardEntry` carries exactly one `shard_cap_bytes` field (no
/// per-validator breakdown for a live gate to combine against), and the
/// live gate consumes the already-MIN'd `shard_cap_bytes` directly. This is
/// NOT dead/unwired code needing a fix (PR #818 fix-burst finding m1 —
/// re-confirmed against the already-recorded adjudication above): it is a
/// correctly-scoped, correctly-tested standalone helper. A hypothetical
/// future RUNTIME re-derivation of the Cross-Validator Minimum is a
/// separate, out-of-scope obligation requiring a new BC postcondition and
/// `architect`/ADR-051 involvement, per that same adjudication.
pub fn effective_shard_cap_bytes(per_validator_caps: &[u64]) -> u64 {
    // A vacuous minimum (no Cohort B validator reads this artifact) imposes
    // no constraint at all — `u64::MAX` rather than `0`, so an
    // unreferenced-but-registered artifact never spuriously self-triggers.
    per_validator_caps.iter().copied().min().unwrap_or(u64::MAX)
}

// ---------------------------------------------------------------------------
// Postcondition 2 — stat()-only current-shard byte-size read ("flat" shape)
// ---------------------------------------------------------------------------

/// Read the current shard's byte size via `stat()`/`metadata()` ONLY — never
/// reading file content into memory for the size determination
/// (Postcondition 2). `"flat"` shape only; see [`read_changelog_item_count`]
/// for the `"frontmatter-changelog-array"` shape's different, more-than-
/// `stat()` read cost.
///
/// EC-004: a shard that does not yet exist on disk (first write ever) is
/// treated as size 0, not an `io::Error`.
pub fn current_shard_bytes_flat(shard_path: &Path) -> io::Result<u64> {
    match std::fs::metadata(shard_path) {
        Ok(meta) => Ok(meta.len()),
        // EC-004: first write ever -> treated as size 0, not an io::Error.
        // TD-VSDD-060 sibling sweep (PR #824 pr-review Finding #1): same
        // cross-platform disambiguation as `read_canonical_content` — see
        // `is_genuinely_missing`'s own doc comment.
        Err(e) if is_genuinely_missing(&e, shard_path) => Ok(0),
        Err(e) => Err(e),
    }
}

// ---------------------------------------------------------------------------
// Postcondition 3 — per-tool-semantics projected-size formula ("flat" shape)
// ---------------------------------------------------------------------------

/// The three tool kinds this BC's trigger discriminates on (Postcondition 3
/// CORRECTED tool-discriminated formula).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolKind {
    /// `content` is the file's complete post-apply state, not a delta.
    Write,
    /// Mutates existing content in place; delta-against-current-size model.
    Edit,
    /// Mutates existing content in place via multiple edit blocks; net delta
    /// is the SUM of every block's own delta (EC-005).
    MultiEdit,
}

impl ToolKind {
    /// Map a Claude Code `tool_name` string onto the discriminated
    /// [`ToolKind`] this BC's formula dispatches on. `None` for any other
    /// tool name — the caller's own zero-cost bypass is expected to have
    /// already excluded non-mutating tools before reaching this point.
    pub fn from_tool_name(tool_name: &str) -> Option<Self> {
        match tool_name {
            "Write" => Some(Self::Write),
            "Edit" => Some(Self::Edit),
            "MultiEdit" => Some(Self::MultiEdit),
            _ => None,
        }
    }
}

/// One `MultiEdit` edit block's raw length inputs, prior to net-delta
/// reduction (EC-005: some blocks net-negative, some net-positive).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EditDelta {
    /// `len(old_string)` for this edit block.
    pub old_len_bytes: u64,
    /// `len(new_string)` for this edit block.
    pub new_len_bytes: u64,
}

/// `Write`'s post-apply projected size: `len(content)` ALONE
/// (Postcondition 3 CORRECTED formula).
///
/// `current_shard_bytes` is deliberately NOT a parameter — the WITHDRAWN
/// uniform `current_shard_bytes + payload_bytes` formula double-counted a
/// `Write`'s own already-complete content on top of the shard's
/// pre-existing bytes, over-triggering rolls on ordinary same-size-or-
/// shrinking full-file `Write` calls.
pub fn projected_size_write(content_len_bytes: u64) -> u64 {
    content_len_bytes
}

/// `Edit`/`MultiEdit`'s post-apply projected size:
/// `current_shard_bytes + net_delta_bytes` (Postcondition 3 — UNCHANGED leg;
/// `Edit`/`MultiEdit` mutate existing content in place, so the delta-
/// against-current-size model was always correct for these two tools).
///
/// `net_delta_bytes` is signed (EC-005: a net-shrinking `MultiEdit` may be
/// negative overall). A large negative delta MUST NOT underflow an unsigned
/// `current_shard_bytes` — this function uses a saturating (floor-at-zero)
/// computation, the production-grade choice VP-116's kani-proof is
/// ALLOCATED to verify for overflow/underflow safety once its Phase 6 /
/// formal-hardening proof scope runs (PR #818 fix-burst finding n1 —
/// corrected from present-tense "verifies"; no kani harness exists for this
/// function on this branch today, per VP-INDEX.md's `feasible-pending-
/// harness` status for VP-116).
pub fn projected_size_edit(current_shard_bytes: u64, net_delta_bytes: i64) -> u64 {
    if net_delta_bytes >= 0 {
        // Safe: net_delta_bytes >= 0, so the cast is lossless for any value
        // that fits an i64 in the first place.
        current_shard_bytes.saturating_add(net_delta_bytes as u64)
    } else {
        current_shard_bytes.saturating_sub(net_delta_bytes.unsigned_abs())
    }
}

/// Net length delta for a single `Edit` call: `len(new_string) - len(old_string)`.
pub fn net_delta_bytes_for_edit(old_len_bytes: u64, new_len_bytes: u64) -> i64 {
    new_len_bytes as i64 - old_len_bytes as i64
}

/// Net length delta for a `MultiEdit` call: the SUM of every edit block's own
/// `len(new_string) - len(old_string)` (EC-005 — may be negative overall even
/// when individual blocks are large).
pub fn net_delta_bytes_for_multi_edit(edits: &[EditDelta]) -> i64 {
    edits
        .iter()
        .map(|e| net_delta_bytes_for_edit(e.old_len_bytes, e.new_len_bytes))
        .sum()
}

/// `true` iff `projected_size > shard_cap_bytes` — the `"flat"` shape's
/// roll-trigger boundary (Postcondition 3's `<=`/`>` comparison; EC-002
/// inclusive boundary at exact equality, EC-003 exactly-one-byte-over).
///
/// This function owns the TRIGGER decision only. BC-1.18.006 owns the
/// observable roll/block outcome once it fires (Postcondition 3's
/// "Ownership" bullet) — implemented separately, by [`execute_roll`] and its
/// [`shard_cap_gate_check`] call site below, not by this function.
pub fn size_trigger_fires(projected_size: u64, shard_cap_bytes: u64) -> bool {
    projected_size > shard_cap_bytes
}

// ---------------------------------------------------------------------------
// Postcondition 8 — item-count trigger ("frontmatter-changelog-array" shape)
// ---------------------------------------------------------------------------

/// Ceiling on the number of bytes [`read_changelog_item_count`] will read
/// from a target file before failing loud, rather than performing an
/// unbounded full-file read (PR #818 fix-burst finding B2). 8 MiB is
/// comfortably larger than any legitimate `"frontmatter-changelog-array"`-
/// shaped artifact should reach even in the pre-BC-1.18.012-migration
/// cold-state (~1,997 items) this BC's own doc comments describe as the
/// worst case on record; this ceiling exists to fail loud against a
/// pathological or adversarial target file rather than read an unbounded
/// amount of data into memory before the frontmatter fence is even located.
const MAX_CHANGELOG_TARGET_READ_BYTES: u64 = 8 * 1024 * 1024;

/// Read the target file's frontmatter far enough to count the existing
/// `changelog:` sequence's items (Postcondition 8's read-cost bullet).
///
/// MORE than a `stat()` call — reads and lightly parses frontmatter content
/// — but remains native, fuel-budget-free dispatcher code, never a WASM
/// plugin invocation (the "why native, not WASM" rationale, Postcondition 2,
/// applies identically to this shape). BOUNDED, not unbounded (PR #818
/// fix-burst finding B2, corrected from an earlier revision's overclaiming
/// doc text): a file exceeding `MAX_CHANGELOG_TARGET_READ_BYTES` fails loud
/// with an `io::ErrorKind::FileTooLarge` error rather than being read into
/// memory in full.
///
/// **PR #818 cycle-2 review finding M-2:** the bound is enforced via a
/// single capped `Read::take(MAX_CHANGELOG_TARGET_READ_BYTES + 1)` read on
/// an already-open file handle, NOT via a separate `metadata()`/`stat()`
/// call followed by an independent, unbounded `read_to_string` (the
/// pre-cycle-2 shape). That two-step shape had two gaps this module's own
/// "pathological or adversarial target file" threat model (see
/// [`MAX_CHANGELOG_TARGET_READ_BYTES`]'s doc comment) makes in-scope: (a) a
/// TOCTOU window — the file could grow past the ceiling between the
/// `stat()` and the later `read_to_string` — and (b) an unbounded read for
/// a non-regular file (e.g. a FIFO) whose `metadata().len()` reports `0`,
/// where `read_to_string` could then block indefinitely or admit unbounded
/// data despite passing the size check trivially. Reading through a single
/// `take()`-limited handle closes both gaps in one mechanism: at most
/// `MAX_CHANGELOG_TARGET_READ_BYTES + 1` bytes are EVER pulled off the
/// handle, regardless of how large the file grows afterward or what kind of
/// file it is, and the `+ 1` lets this function distinguish "exactly at the
/// ceiling" (legal) from "over the ceiling" (rejected) without a separate
/// stat.
///
/// This function's contract is the SAME read regardless of cold-state
/// (pre-BC-1.18.012 migration, ~1,997-item, not-N-relative-bounded) vs.
/// steady-state (post-migration, genuinely `<= N`-item-bounded) — the
/// cold/steady-state split BC-1.18.005 documents is a PERFORMANCE
/// characterization, not a different code path this function branches on.
///
/// A target file that EXISTS but has no well-formed `---` frontmatter fence
/// (missing opening fence, or missing line-anchored closing fence) is
/// treated PERMISSIVELY — `Ok(0)` — rather than a fail-loud `io::Error` (PR
/// #818 fix-burst finding B1, corrected from an earlier revision): a
/// present-but-fenceless file must not hard-block a legitimate write the
/// same way a genuinely missing file does not (EC-014's NotFound->Ok(0)
/// precedent). The closing fence itself is matched LINE-ANCHORED — a line
/// consisting of EXACTLY `---` (optionally with a trailing `\r`), never a
/// bare `"\n---"` substring search (PR #818 fix-burst finding n3, corrected
/// from an earlier revision) — so a line such as `----`, `---foo`, or an
/// in-block-scalar YAML literal line that happens to start with `---` is
/// never mistaken for the closing fence.
pub fn read_changelog_item_count(target_path: &Path) -> io::Result<u64> {
    /// Deserialization target isolating just the `changelog:` sequence —
    /// mirrors `last-amended-migrate/src/yaml_guard.rs`'s `MinimalFrontmatter`
    /// pattern (extra frontmatter fields are ignored by default struct
    /// deserialization; no `deny_unknown_fields`).
    #[derive(Deserialize)]
    struct ChangelogFrontmatter {
        #[serde(default)]
        changelog: Option<Vec<serde_norway::Value>>,
    }

    let file = match std::fs::File::open(target_path) {
        Ok(file) => file,
        // EC-014 (BC-1.18.005 v1.9): a not-yet-existing
        // "frontmatter-changelog-array"-shaped target file is a legitimate
        // first-ever Write CREATING it, not a fail-loud condition — treated
        // as holding 0 existing changelog items, mirroring
        // current_shard_bytes_flat's EC-004 NotFound->Ok(0) precedent above.
        // Any OTHER io::Error kind stays fail-loud. TD-VSDD-060 sibling
        // sweep (PR #824 pr-review Finding #1): same cross-platform
        // disambiguation — see `is_genuinely_missing`'s own doc comment.
        Err(e) if is_genuinely_missing(&e, target_path) => return Ok(0),
        Err(e) => return Err(e),
    };
    // M-2: a single capped read replaces the former separate
    // metadata()-then-read_to_string two-step (see this function's own doc
    // comment for the TOCTOU/non-regular-file gaps that shape had). Reading
    // one byte PAST the ceiling lets the length check below distinguish
    // "exactly at the ceiling" (legal) from "over the ceiling" (rejected)
    // without ever needing a separate stat() call.
    let mut raw = String::new();
    file.take(MAX_CHANGELOG_TARGET_READ_BYTES + 1)
        .read_to_string(&mut raw)?;
    if raw.len() as u64 > MAX_CHANGELOG_TARGET_READ_BYTES {
        return Err(io::Error::new(
            io::ErrorKind::FileTooLarge,
            format!(
                "{}: refusing unbounded frontmatter-changelog-array read — file exceeds the {} \
                 byte ceiling (BC-1.18.005 fix-burst B2 / cycle-2 review finding M-2: bounded \
                 parse via a capped Read::take, never a separate stat()-then-unbounded-read)",
                target_path.display(),
                MAX_CHANGELOG_TARGET_READ_BYTES
            ),
        ));
    }

    // Opportunistic hardening (S-25.02 Phase F4 LOCAL adversary pass-1
    // cluster-1 observation): tolerate a `---\r\n` (CRLF) opening fence in
    // addition to `---\n`, so a CRLF-line-ended frontmatter file is not
    // spuriously treated as having no fence at all.
    let Some(after_open) = raw
        .strip_prefix("---\r\n")
        .or_else(|| raw.strip_prefix("---\n"))
    else {
        // B1: present-but-fenceless (no opening fence at all) -> permissive
        // Ok(0), mirroring EC-014's missing-file precedent — never a
        // fail-loud hard block of a legitimate write.
        return Ok(0);
    };

    // n3: line-anchored closing-fence search. A line consisting of EXACTLY
    // "---" (optionally trailing "\r" for a CRLF-terminated line) closes the
    // frontmatter block — never a bare "\n---" substring match, which would
    // also match "----", "---foo", or an in-block-scalar YAML literal line
    // that happens to start with "---".
    let mut offset = 0usize;
    let mut closing_fence_start = None;
    for line in after_open.split_inclusive('\n') {
        let without_newline = line.strip_suffix('\n').unwrap_or(line);
        let trimmed = without_newline
            .strip_suffix('\r')
            .unwrap_or(without_newline);
        if trimmed == "---" {
            closing_fence_start = Some(offset);
            break;
        }
        offset += line.len();
    }
    let Some(end) = closing_fence_start else {
        // B1: present-but-fenceless (opening fence found, but no
        // line-anchored closing fence) -> permissive Ok(0), same rationale
        // as the missing-opening-fence case above.
        return Ok(0);
    };
    let block = &after_open[..end];

    // EC-021 (BC-1.18.005 v1.13, PR #818 fix-burst finding M-1): this
    // fence-bearing-but-malformed-YAML case MUST remain fail-loud
    // (io::ErrorKind::InvalidData, never coerced to Ok(0) the way EC-014's
    // NotFound or EC-020's fenceless cases are) -- see the "Malformed-YAML
    // case" bullet under Postcondition 8. The message is MANDATORY,
    // load-bearing content, not cosmetic: it MUST (a) name the underlying
    // YAML-parse cause (the `{e}` interpolation below, from
    // serde_norway::from_str's Err) and (b) state that this gate only
    // intercepts Edit/Write/MultiEdit tool calls, so the operator can
    // repair the malformed frontmatter block via any OTHER means (e.g. a
    // Bash-invoked edit) to escape the self-deadlock this fail-loud
    // behavior would otherwise create (every gated Edit/Write/MultiEdit
    // against the target -- including a would-be repair edit -- routes
    // through this same read). An unchanged, generic
    // "{path}: frontmatter YAML parse failed: {e}"-only message is
    // NON-COMPLIANT with this Postcondition from v1.13 forward.
    let parsed: ChangelogFrontmatter = serde_norway::from_str(block).map_err(|e| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!(
                "{}: frontmatter YAML parse failed: {e} -- this gate only intercepts Edit, \
                 Write, and MultiEdit tool calls against this artifact; repair the malformed \
                 frontmatter block by any OTHER means (e.g. a Bash-invoked edit) so this same \
                 gate can parse it successfully, after which Edit, Write, and MultiEdit against \
                 this artifact will proceed normally again",
                target_path.display()
            ),
        )
    })?;

    Ok(parsed
        .changelog
        .map(|items| items.len() as u64)
        .unwrap_or(0))
}

/// `true` iff `current_item_count + 1 > N` — the item-count trigger boundary
/// (Postcondition 8's trigger condition; EC-008 off-by-one: `N-1` items ->
/// `false`, exactly `N` or `N+1` items -> `true`).
///
/// NEVER a byte-size comparison for this shape — `shard_cap_bytes` still
/// bounds the artifact's total byte footprint as a whole-artifact concern,
/// but the rotation decision within this shape is item-count-based only.
pub fn item_count_trigger_fires(current_item_count: u64, n: u64) -> bool {
    current_item_count.saturating_add(1) > n
}

/// Resolve the effective `low_water_mark` for a `"frontmatter-changelog-
/// array"`-shaped entry: the entry's explicit value if present, otherwise
/// `floor(N/2)` (EC-010's default).
///
/// Callers MUST have already validated the entry via [`validate_entry`]
/// (or [`validate_low_water_mark`] directly) before calling this — this
/// function does NOT re-validate `0 <= low_water_mark < N`; it only resolves
/// the omitted-vs-explicit default.
pub fn resolved_low_water_mark(n: u64, low_water_mark: Option<i64>) -> u64 {
    match low_water_mark {
        // Caller-validated: already known non-negative and < N.
        Some(v) => v as u64,
        // EC-010: integer division floors naturally.
        None => n / 2,
    }
}

/// Validate a `"frontmatter-changelog-array"`-shaped entry's `low_water_mark`
/// against `0 <= low_water_mark < N` (EC-011), and determine whether the
/// EC-012 non-fatal amortization advisory applies.
///
/// - `low_water_mark >= N` (the `== N` boundary included) or negative ->
///   `Err(ShardConfigError::InvalidLowWaterMark)` (EC-011). NEVER silently
///   clamped or defaulted around.
/// - Any other legal value (including the boundary value `N - 1`) ->
///   `Ok(fires_advisory)`, where `fires_advisory` is `true` iff
///   `low_water_mark > floor(N/2)` (EC-012) — the caller is responsible for
///   emitting the actual `tracing::warn!` amortization advisory (citing the
///   configured `(N, low_water_mark)` pair and the amortization factor
///   `N - low_water_mark`, compared against the default's `N -
///   floor(N/2)` amortization) when this returns `Ok(true)`. Config load
///   ALWAYS succeeds (`Ok`, never `Err`) for any value satisfying
///   `0 <= low_water_mark < N` — this function never conflates the
///   fail-loud numeric-constraint check with the advisory, non-fatal one.
pub fn validate_low_water_mark(
    artifact_stem: &str,
    n: u64,
    low_water_mark: i64,
) -> Result<bool, ShardConfigError> {
    // i128 comparison sidesteps any u64/i64 range mismatch for the `>= N`
    // check — N is realistically far below i64::MAX/u64::MAX for this BC's
    // item-count trigger, but the wider comparison type costs nothing and
    // removes the need to reason about cast overflow at the boundary.
    if low_water_mark < 0 || i128::from(low_water_mark) >= i128::from(n) {
        return Err(ShardConfigError::InvalidLowWaterMark {
            artifact_stem: artifact_stem.to_string(),
            n,
            low_water_mark,
        });
    }

    let default_low_water_mark = n / 2;
    // Safe: low_water_mark just checked >= 0 above.
    Ok(low_water_mark as u64 > default_low_water_mark)
}

// ---------------------------------------------------------------------------
// Invariant 1 / Precondition 1 — the single native gate dispatch entry point
// ---------------------------------------------------------------------------

/// Extract a REQUIRED string field's byte length from a tool_input payload
/// (or a single `MultiEdit` edit block), fail-loud when the field is absent
/// or not a JSON string (PR #818 fix-burst finding m3/M-1).
///
/// Before this fix, every field extraction in the `Write`/`Edit`/`MultiEdit`
/// arms below used `.and_then(|v| v.as_str()).map(|s| s.len() as
/// u64).unwrap_or(0)` — an absent, non-string, or otherwise malformed
/// required field (`content`/`old_string`/`new_string`) silently computed a
/// 0-byte/0-delta size instead of failing loud. That silent 0 meant the cap
/// trigger could NEVER fire against a malformed payload, directly
/// contradicting this module's own established "never a silent Continue
/// that would leave an oversized artifact unguarded" posture (see
/// [`ShardConfigError::MissingShape`]'s doc text) and CLAUDE.md's forbidden
/// silent-swallowed-failure pattern. A malformed/absent REQUIRED field now
/// fails loud exactly like a stat() I/O error a few lines above already
/// does in these same arms.
fn required_str_len_bytes(
    payload: &serde_json::Value,
    field: &str,
    tool_name: &str,
    artifact_stem: &str,
) -> Result<u64, String> {
    match payload.get(field) {
        Some(serde_json::Value::String(s)) => Ok(s.len() as u64),
        Some(_) => Err(format!(
            "BC-1.18.005: {tool_name} tool_input field \"{field}\" is present but not a JSON \
             string, for artifact_stem \"{artifact_stem}\" — refusing to silently treat a \
             malformed required field as 0 bytes (PR #818 fix-burst finding m3/M-1: a malformed \
             payload MUST fail loud, never let the cap trigger go silently unguarded)"
        )),
        None => Err(format!(
            "BC-1.18.005: {tool_name} tool_input is missing the required field \"{field}\", for \
             artifact_stem \"{artifact_stem}\" — refusing to silently treat an absent required \
             field as 0 bytes (PR #818 fix-burst finding m3/M-1: a malformed payload MUST fail \
             loud, never let the cap trigger go silently unguarded)"
        )),
    }
}

/// Native (non-WASM) shard-cap gate check for a single `Edit`/`Write`/
/// `MultiEdit` PreToolUse tool call.
///
/// This is the single entry point `executor.rs` calls BEFORE the
/// registry-driven WASM plugin loop (Invariant 1; architecturally analogous
/// to [`crate::indeterminate_marker::block_if_marker_check`]).
/// Postcondition 1's zero-cost bypass applies identically regardless of
/// which shape a matched entry declares (Postcondition 8's shape-dispatch
/// bullet: "both shapes share the SAME config-match/no-match entry point").
///
/// Returns:
/// - `HookResult::Continue` — no config match (Postcondition 1 / EC-001), or
///   a match whose projected size / item count does not exceed its trigger
///   threshold (EC-002, happy-path item-count rows).
/// - `HookResult::Block { .. }` — NEVER returned by this function itself.
///   BC-1.18.006 (byte-size roll) and BC-1.18.009 (item-count rotate-then-
///   retry) own the observable Block outcome once THIS function's trigger
///   fires (Postcondition 3 / Postcondition 8 "Ownership" bullets) — this
///   cluster fully implements the trigger boundary and hand-off point (this
///   function's own logic is not a stub), and deliberately never implements
///   the roll/rotation behavior itself — that is later clusters' own scope.
/// - `HookResult::Error { .. }` — fail-loud when [`find_matching_entry`]
///   resolves a MATCHED entry that fails [`validate_entry`] (EC-009 missing
///   `shape`; EC-011 invalid `low_water_mark`; EC-013/EC-015/EC-016/EC-017),
///   via [`ShardConfigError`]'s `From<ShardConfigError> for HookResult`
///   impl. A malformed SIBLING entry the current dispatch's target does NOT
///   match is NEVER validated and NEVER produces this outcome (EC-018,
///   BC-1.18.005 v1.12 MATCH-FIRST restructure, Postcondition 1's
///   "Blast-radius scoping" ruling).
///
/// `tool_input` carries the tool-specific payload (`content` for `Write`;
/// `old_string`/`new_string` for `Edit`; an `edits` array for `MultiEdit`) —
/// kept as an opaque `serde_json::Value` here, matching
/// [`crate::payload::HookPayload::tool_input`]'s own representation, so this
/// function's signature does not have to special-case three different typed
/// tool-input shapes at the call boundary.
pub fn shard_cap_gate_check(
    shard_registry: &ShardRegistry,
    tool_name: &str,
    target_path: &Path,
    tool_input: &serde_json::Value,
) -> HookResult {
    // Postcondition 1 / Invariant 3 / EC-018: unmatched paths return
    // Continue before any stat() call, shape dispatch, OR entry validation —
    // `validate_entry` below is NEVER called for a sibling entry the current
    // dispatch's target does not match (BC-1.18.005 v1.12 MATCH-FIRST
    // restructure, F-C1-P6-001, Postcondition 1's "Blast-radius scoping"
    // ruling). N-2 (PR #818 fix-burst): `find_matching_entry` itself fails
    // loud (`ShardConfigError::DuplicateArtifactStem`) when the target's
    // stem matches MORE THAN ONE `[[shard]]` entry, rather than silently
    // resolving to whichever appears first.
    let entry = match find_matching_entry(shard_registry, target_path) {
        Ok(Some(entry)) => entry,
        Ok(None) => return HookResult::Continue,
        Err(e) => return e.into(),
    };

    // Entry-match-time semantic validation (EC-009/EC-011/EC-012/EC-013/
    // EC-015/EC-016/EC-017), scoped to THIS matched entry only — never a
    // sibling. A malformed entry that the current dispatch's target
    // actually matches still fails loud here, unchanged from the pre-v1.12
    // load()-time posture (only the SCOPE narrowed, not the outcome).
    if let Err(e) = validate_entry(entry) {
        return e.into();
    }

    // `validate_entry` above already rejects a missing `shape` (EC-009), so
    // `entry.shape` is guaranteed `Some` here. Kept as a fail-loud
    // defensive fallback (never an `unwrap()`/`expect()`) rather than an
    // assumption a future refactor could silently invalidate.
    let Some(shape) = entry.shape else {
        return ShardConfigError::MissingShape {
            artifact_stem: entry.artifact_stem.clone(),
        }
        .into();
    };

    match shape {
        ShardShape::Flat => {
            let Some(tool_kind) = ToolKind::from_tool_name(tool_name) else {
                // Not a candidate mutating tool for the "flat" byte-size
                // trigger — zero-cost-bypass spirit of Postcondition 1
                // extended to an unsupported tool kind.
                return HookResult::Continue;
            };

            // BC-1.18.006 Postcondition 1 / ADR-051 §Decision 11: self-heal
            // recovery (E-SHD-006/E-SHD-007) runs on the artifact's NEXT
            // matched dispatch — of ANY tool kind, not just Edit/MultiEdit,
            // since a crash can strand ANY roll regardless of which tool
            // triggered it — BEFORE evaluating this dispatch's own trigger.
            // See `run_self_heal_if_plausible`'s own doc comment for the
            // cheap-detection gate that keeps this a single stat()/small-
            // TOML-read cost on every healthy (non-crashed) dispatch, never
            // a directory-wide scan.
            if let Err(e) = run_self_heal_if_plausible(entry, target_path) {
                return e.into();
            }

            // PR #824 pr-review Finding #3 (MAJOR, TD-VSDD-060 sibling-
            // callsite sweep): FIX-MED-2 was originally installed ONLY on
            // the `Write` arm's own `current_shard_bytes_flat` call site
            // below, leaving the `Edit`/`MultiEdit` arms' IDENTICAL
            // `current_shard_bytes_flat(target_path)` calls unguarded (and
            // untested) — a governed artifact path replaced with a symlink
            // to a sensitive file elsewhere on the filesystem could still
            // have its target's size `stat()`-leaked (CWE-200) through
            // either of those two arms, and — had the leaked size ever
            // tripped the roll trigger — its bytes durably sealed via
            // `execute_roll`'s own (already-guarded) read path. HOISTED
            // here, above the `match tool_kind` dispatch, so this single
            // call covers all three mutation-tool arms uniformly — see
            // `reject_canonical_symlink`'s own doc comment.
            if let Err(e) = reject_canonical_symlink(&entry.artifact_stem, target_path) {
                return e.into();
            }

            // F-002 fix (S-25.02 Phase F4 LOCAL adversary pass-1 cluster-1,
            // MEDIUM), UPDATED by BC-1.18.006 v1.5 (F-C2-P1-002): BC-1.18.005
            // Postcondition 3's CORRECTED `Write` TRIGGER FORMULA
            // (`projected_size = len(content)` alone) never consults
            // `current_shard_bytes` — that half of F-002's original ruling is
            // UNCHANGED. What changed in v1.5: BC-1.18.006 Postcondition 7
            // catch point (ii) now ALSO requires a dedicated, bounded
            // `stat()` inside the `Write` arm itself (a crash-orphan backstop
            // probe, entirely separate from the trigger formula) — see that
            // arm's own doc comment below. So the stat() call is pushed down
            // into EACH arm individually (never called unconditionally
            // before this match, which would fail a `Write` on a stat()
            // error its OWN trigger formula doesn't need), but as of v1.5 it
            // is no longer true that ONLY Edit/MultiEdit perform one — Write
            // now does too, for a different reason (the backstop, not the
            // trigger). EC-004 (a missing shard file treated as size 0) is
            // preserved throughout: `current_shard_bytes_flat` itself still
            // maps `NotFound` to `Ok(0)`, unchanged.
            //
            // BC-1.18.006 v1.9 Postcondition 2's "Double-fire exception"
            // (Invariant 4 Case B1/B2 split, F-C2-P8-004, MINOR; EC-026):
            // dispatch-local tracking of whether
            // `reconcile_leading_probe_backstop` fired (and succeeded)
            // EARLIER in this SAME dispatch — set `true` by whichever tool
            // arm below actually invokes it. Threaded through to the
            // `Ok(None)` short-circuit's `build_empty_roll_retry_block_reason`
            // call further down so it can select Case B1 vs. Case B2.
            let mut preceded_by_backstop_roll = false;
            let projected_size = match tool_kind {
                ToolKind::Write => {
                    // BC-1.18.006 v1.5 Postcondition 7 catch point (ii)
                    // (F-C2-P1-002, MAJOR): `Write`'s own Postcondition 3
                    // trigger formula (`projected_size = len(content)`
                    // alone) performs no `stat()` of the canonical file at
                    // all — F-002's rationale above, UNCHANGED — so, unlike
                    // Edit/MultiEdit (which reuse an existing stat() their
                    // own trigger formula already pays for), `Write` has no
                    // existing stat()-read for the crash-orphan backstop to
                    // reuse. This is therefore a NEW, DEDICATED, bounded
                    // `stat()` specifically for the backstop probe — never
                    // for the trigger formula itself, which stays entirely
                    // stat-free. Without it, a `Write` whose OWN `content`
                    // is under cap would apply directly against a
                    // crash-orphaned, un-sealed, over-cap canonical file
                    // left behind by a missed catch point (i) (EC-015/
                    // EC-017), silently destroying that history — the
                    // exact data-loss gap F-C2-P1-002 identifies.
                    //
                    // BC-1.18.006 v1.6 Invariant 8 / EC-019 (F-C2-P2-003,
                    // MAJOR, cluster-2 LOCAL adversary pass-2 — supersedes
                    // this arm's prior fail-open posture): a non-`NotFound`
                    // failure of THIS backstop probe's OWN `stat()` call now
                    // fails LOUD as `HookResult::Error` naming `E-SHD-008`.
                    // F-002 (BC-1.18.005) is UNCHANGED and still governs a
                    // DIFFERENT question — it scopes ONLY the stat-free
                    // Postcondition 3 TRIGGER FORMULA (`projected_size =
                    // len(content)` alone), which never reads
                    // `current_shard_bytes` and is therefore never affected
                    // by any stat() outcome, success or failure. F-002 never
                    // governed this probe's OWN failure disposition — this
                    // probe (and the Write-arm's very own `stat()` call
                    // altogether) did not exist until BC-1.18.006 v1.5 added
                    // it; see `test_BC_1_18_005_F002_write_under_cap_continues_regardless_of_current_shard_bytes_value`,
                    // retargeted off a stat()-failure fixture onto a
                    // successfully-stat()-able one for exactly this reason.
                    // Rationale (product-owner v1.6 adjudication): `stat()`
                    // follows symlinks and fails on `ELOOP`, but
                    // `write_atomic`'s `rename` need not dereference the
                    // final symlink component and CAN succeed — so
                    // fail-OPEN here would let this probe silently skip
                    // while the `Write` itself proceeds underneath a
                    // symlink the probe could not see through, destroying a
                    // crash-orphaned, un-sealed, over-cap canonical this
                    // dispatch never confirmed was safe to overwrite.
                    // `current_shard_bytes_flat` itself still maps
                    // `NotFound` to `Ok(0)` (EC-004, legitimate
                    // first-write) — Invariant 8 is scoped to every OTHER
                    // `io::ErrorKind`.
                    //
                    // FIX-MED-2 (S-25.02 PR #824 second-security-review,
                    // MEDIUM, CWE-59/CWE-200): the symlinked-canonical guard
                    // for this call site is now HOISTED above the
                    // `match tool_kind` dispatch (PR #824 pr-review Finding
                    // #3) so it also covers the `Edit`/`MultiEdit` arms
                    // below — see the hoisted call's own comment just above
                    // this `match`.
                    match current_shard_bytes_flat(target_path) {
                        Ok(current_bytes) if current_bytes > entry.shard_cap_bytes => {
                            if let Err(e) = reconcile_leading_probe_backstop(entry, target_path) {
                                return e.into();
                            }
                            preceded_by_backstop_roll = true;
                        }
                        Ok(_) => {}
                        Err(source) => {
                            return ShardRollError::BackstopProbeFailed {
                                artifact_stem: entry.artifact_stem.clone(),
                                path: target_path.display().to_string(),
                                source,
                            }
                            .into();
                        }
                    }

                    let content_len = match required_str_len_bytes(
                        tool_input,
                        "content",
                        "Write",
                        &entry.artifact_stem,
                    ) {
                        Ok(len) => len,
                        Err(message) => return HookResult::Error { message },
                    };
                    projected_size_write(content_len)
                }
                ToolKind::Edit => {
                    let mut current_bytes = match current_shard_bytes_flat(target_path) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            return HookResult::Error {
                                message: format!(
                                    "BC-1.18.005: failed to stat() shard '{}' for artifact_stem \
                                     \"{}\": {e}",
                                    target_path.display(),
                                    entry.artifact_stem
                                ),
                            };
                        }
                    };

                    // BC-1.18.006 Postcondition 7 catch point (ii) / story
                    // AC-025 (ADR-051 §Decision 15 point 3): leading-probe
                    // backstop, reusing `current_bytes` (JUST computed above
                    // by this SAME `current_shard_bytes_flat` stat() call —
                    // no NEW stat() call is added here). Real (non-`todo!()`)
                    // comparison: for every one of this file's own already-
                    // shipped cluster-1 fixtures, `current_bytes <=
                    // entry.shard_cap_bytes` holds (none construct an
                    // on-disk shard already over cap), so this branch is
                    // NEVER entered by any pre-existing test — only a NEW
                    // test-writer fixture that deliberately leaves an
                    // on-disk shard already over cap (covering a missed
                    // catch point (i), EC-015) exercises
                    // `reconcile_leading_probe_backstop`.
                    //
                    // Postcondition 7's bounded-window guarantee (AC-025):
                    // once the backstop reconciles, the canonical file is
                    // UNCONDITIONALLY 0 bytes (Invariant 6) — `current_bytes`
                    // is updated in place to reflect that fact so THIS same
                    // dispatch's own BC-1.18.005 trigger, evaluated just
                    // below, is judged against the post-backstop state, never
                    // the stale pre-backstop stat() this arm already paid
                    // for. Without this update, a net-zero-delta Edit against
                    // an already-reconciled (now-empty) canonical would
                    // incorrectly re-fire the trigger a second time for the
                    // SAME already-closed over-cap condition.
                    if current_bytes > entry.shard_cap_bytes {
                        if let Err(e) = reconcile_leading_probe_backstop(entry, target_path) {
                            return e.into();
                        }
                        current_bytes = 0;
                        preceded_by_backstop_roll = true;
                    }

                    let old_len = match required_str_len_bytes(
                        tool_input,
                        "old_string",
                        "Edit",
                        &entry.artifact_stem,
                    ) {
                        Ok(len) => len,
                        Err(message) => return HookResult::Error { message },
                    };
                    let new_len = match required_str_len_bytes(
                        tool_input,
                        "new_string",
                        "Edit",
                        &entry.artifact_stem,
                    ) {
                        Ok(len) => len,
                        Err(message) => return HookResult::Error { message },
                    };
                    let net_delta = net_delta_bytes_for_edit(old_len, new_len);
                    projected_size_edit(current_bytes, net_delta)
                }
                ToolKind::MultiEdit => {
                    let mut current_bytes = match current_shard_bytes_flat(target_path) {
                        Ok(bytes) => bytes,
                        Err(e) => {
                            return HookResult::Error {
                                message: format!(
                                    "BC-1.18.005: failed to stat() shard '{}' for artifact_stem \
                                     \"{}\": {e}",
                                    target_path.display(),
                                    entry.artifact_stem
                                ),
                            };
                        }
                    };

                    // BC-1.18.006 Postcondition 7 catch point (ii) / story
                    // AC-025 — see the identical, more fully commented guard
                    // (including the post-backstop `current_bytes` reset to
                    // 0, Invariant 6) in the `ToolKind::Edit` arm just above.
                    if current_bytes > entry.shard_cap_bytes {
                        if let Err(e) = reconcile_leading_probe_backstop(entry, target_path) {
                            return e.into();
                        }
                        current_bytes = 0;
                        preceded_by_backstop_roll = true;
                    }

                    // m3/M-1: the "edits" field itself must be a present JSON
                    // array — absent or non-array MUST fail loud, never
                    // silently treated as zero edit blocks. An EMPTY array
                    // (`"edits": []`) is legitimate (net_delta = 0) and is
                    // NOT rejected here — only an absent/non-array field is.
                    let edits_value = match tool_input.get("edits").and_then(|v| v.as_array()) {
                        Some(arr) => arr,
                        None => {
                            return HookResult::Error {
                                message: format!(
                                    "BC-1.18.005: MultiEdit tool_input is missing the required \
                                     \"edits\" array (or it is not a JSON array), for \
                                     artifact_stem \"{}\" — refusing to silently treat this as \
                                     zero edit blocks (PR #818 fix-burst finding m3/M-1: a \
                                     malformed payload MUST fail loud, never let the cap trigger \
                                     go silently unguarded)",
                                    entry.artifact_stem
                                ),
                            };
                        }
                    };
                    let mut edits: Vec<EditDelta> = Vec::with_capacity(edits_value.len());
                    for edit in edits_value {
                        let old_len = match required_str_len_bytes(
                            edit,
                            "old_string",
                            "MultiEdit",
                            &entry.artifact_stem,
                        ) {
                            Ok(len) => len,
                            Err(message) => return HookResult::Error { message },
                        };
                        let new_len = match required_str_len_bytes(
                            edit,
                            "new_string",
                            "MultiEdit",
                            &entry.artifact_stem,
                        ) {
                            Ok(len) => len,
                            Err(message) => return HookResult::Error { message },
                        };
                        edits.push(EditDelta {
                            old_len_bytes: old_len,
                            new_len_bytes: new_len,
                        });
                    }
                    let net_delta = net_delta_bytes_for_multi_edit(&edits);
                    projected_size_edit(current_bytes, net_delta)
                }
            };

            if size_trigger_fires(projected_size, entry.shard_cap_bytes) {
                // S-25.02 cluster-2 (BC-1.18.006 "roll", IMPLEMENTED): the
                // trigger-fires branch owns the observable roll/block
                // outcome (this module's own "Scope note" above is UPDATED
                // by cluster-2 — BC-1.18.006 is no longer out of scope).
                // Postcondition 1's staged four-step sequence executes
                // BEFORE any `HookResult` is returned (Invariant 2);
                // `execute_roll` is a real, fully implemented, unit-tested
                // function (`test_BC_1_18_006_AC006_execute_roll_*` et al.,
                // `shard_manager.rs`'s own `bc_1_18_006_roll_tests` module).
                // A fired trigger no longer merely `tracing::warn!`s and
                // Continues (the withdrawn cluster-1 posture) — it MUST
                // resolve to either `HookResult::Block` (Postcondition 2's
                // unified retry message) or `HookResult::Error` (a genuine
                // `E-SHD-001` crash), never a silent `Continue` (Invariant
                // 1).
                return match execute_roll(entry, target_path, false) {
                    Ok(Some(sealed)) => HookResult::Block {
                        reason: build_roll_retry_block_reason(
                            &entry.artifact_stem,
                            entry.shard_cap_bytes,
                            &sealed.path,
                        ),
                    },
                    // F-C2-P2-006: the canonical was already empty (nothing
                    // to seal) — the trigger fired purely because of this
                    // dispatch's OWN oversized payload. Still a Block
                    // (Invariant 1 — never a silent Continue), but with a
                    // dedicated message: there is no sealed shard to name.
                    Ok(None) => HookResult::Block {
                        reason: build_empty_roll_retry_block_reason(
                            &entry.artifact_stem,
                            entry.shard_cap_bytes,
                            projected_size,
                            preceded_by_backstop_roll,
                        ),
                    },
                    Err(e) => e.into(),
                };
            }

            HookResult::Continue
        }
        ShardShape::FrontmatterChangelogArray => {
            // O-C1-P4-001 (LOW, S-25.02 Phase F4 LOCAL adversary cluster-1
            // pass-4 observation): mirror the "flat" arm's
            // ToolKind::from_tool_name early-Continue guard here, so a
            // non-Edit/Write/MultiEdit tool_name never reads the target
            // file. Benign in production today (executor.rs's caller
            // already pre-filters to mutating tools before this function is
            // reached at all) but restores internal symmetry between the
            // two shape arms and avoids surfacing a spurious
            // HookResult::Error for a non-mutating tool call that happens
            // to hit a malformed/unreadable frontmatter fence.
            if ToolKind::from_tool_name(tool_name).is_none() {
                return HookResult::Continue;
            }

            // Postcondition 8's item-count trigger is state-based (current
            // item count in the file on disk), not payload-based — every
            // candidate tool call is evaluated identically regardless of
            // Edit/Write/MultiEdit shape. `validate_entry` above already
            // rejects a missing `n` for this shape (EC-016), so `entry.n` is
            // guaranteed `Some` here — kept as a fail-loud defensive
            // fallback (never an `unwrap()`/`expect()`) rather than an
            // assumption a future refactor could silently invalidate.
            let Some(n) = entry.n else {
                return ShardConfigError::MissingN {
                    artifact_stem: entry.artifact_stem.clone(),
                }
                .into();
            };

            // PR #824 pr-review Finding #3 (MAJOR, TD-VSDD-060 sibling-
            // callsite sweep): `read_changelog_item_count`'s `File::open`
            // below DOES dereference a symlink at `target_path`, and reads
            // its content (unlike the `"flat"` shape's `stat()`-only
            // trigger) — the same CWE-59/CWE-200 exfiltration concern
            // FIX-MED-2 already closes for the `"flat"` shape's three
            // mutation-tool arms applies here too, via a different
            // shape/read-cost path. Refuse loud rather than reading through
            // a symlinked canonical — see `reject_canonical_symlink`'s own
            // doc comment.
            if let Err(e) = reject_canonical_symlink(&entry.artifact_stem, target_path) {
                return e.into();
            }

            let current_item_count = match read_changelog_item_count(target_path) {
                Ok(count) => count,
                Err(e) => {
                    return HookResult::Error {
                        message: format!(
                            "BC-1.18.005: failed to read changelog: item count for \
                             artifact_stem \"{}\" at '{}': {e}",
                            entry.artifact_stem,
                            target_path.display()
                        ),
                    };
                }
            };

            if item_count_trigger_fires(current_item_count, n) {
                // Ownership bullet (Postcondition 8): BC-1.18.009 owns the
                // observable rotate-then-block-and-retry outcome once this
                // trigger fires — still out of scope for this cluster (see
                // this module's own "Scope note" — UPDATED by cluster-2:
                // BC-1.18.006's "flat"-shape roll IS now implemented above,
                // but BC-1.18.009's item-count rotate remains pending). This
                // arm keeps the non-Block, honest hand-off posture
                // (`tracing::warn!` + `Continue`) the "flat" shape's
                // trigger-fired branch above has since WITHDRAWN in favor of
                // a real `execute_roll`-backed outcome.
                tracing::warn!(
                    artifact_stem = %entry.artifact_stem,
                    current_item_count,
                    n,
                    "BC-1.18.005: item-count shard-cap trigger fired; rotate/block outcome is \
                     owned by BC-1.18.009 (not yet implemented in this cluster) — allowing \
                     the call to proceed"
                );
            }

            HookResult::Continue
        }
    }
}

// ===========================================================================
// BC-1.18.006 — Roll-Before-Write via Block-and-Retry (Not Transparent
// Redirection) Plus Same-Invocation Atomic Shard-Index Publication
// (S-25.02 cluster-2 "roll")
// ===========================================================================
//
// # BC-5.38.001 Red Gate discipline — IMPLEMENTED (S-25.02 cluster-2)
//
// Every function below is now a REAL, fully implemented body — the
// stub-architect's original `todo!()` placeholders (commit `e04ab76f`) have
// all been replaced by implementer per BC-1.18.006's postconditions, driving
// test-writer's Red Gate suite (`bc_1_18_006_roll_test.rs`'s integration
// tests plus this file's own `bc_1_18_006_roll_tests` unit-test module) to
// green. `build_roll_retry_block_reason` (GREEN-BY-DESIGN — a pure,
// zero-branching string template) and `From<ShardRollError> for HookResult`
// (WIRING-EXEMPT — `Display`-forwarding delegation, identical in shape to
// this file's already-shipped `From<ShardConfigError> for HookResult`) were
// real from the stub-architect's own initial burst, per their own doc
// comments. `execute_roll`'s call site (this module's `ShardShape::Flat`
// trigger-fire branch, above), Postcondition 7's two catch-point call sites
// (`crate::invoke::reconcile_replace_all_overcap_if_qualifying`, wired
// unconditionally into `main::run`; and the `ShardShape::Flat`
// `Write`/`Edit`/`MultiEdit`-arm leading-probe guards, above), and
// Postcondition 1/ADR-051 §Decision 11's self-heal recovery wiring
// (`run_self_heal_if_plausible`, also called from the `ShardShape::Flat`
// arm above) are all real, wired, and covered by dedicated tests — zero
// regression against cluster-1's already-shipped, already-green
// BC-1.18.005 suite.

/// One `[[shard]]` shard-index table entry — one seal event (BC-1.18.006
/// Postcondition 5, extended v1.4 with `sealed_retroactively`).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ShardIndexEntry {
    /// Monotonically increasing seal sequence number, starting at 1.
    pub seq: u32,
    /// The sealed shard's own filename, e.g. `"decision-log.0001.md"`.
    pub path: String,
    /// UTC ISO-8601 seal timestamp.
    pub sealed_at: String,
    /// The sealed shard's exact final byte count. `<= shard_cap_bytes`
    /// (Postcondition 3) for a normal seal; MAY exceed it when
    /// `sealed_retroactively` is `true` (Postcondition 7's documented,
    /// narrowly-scoped exception — Invariant 6).
    pub bytes_at_seal: u64,
    /// `true` iff this seal was produced by Postcondition 7's retroactive
    /// reconciliation path (catch point (i) or (ii)) rather than
    /// Postcondition 1's normal pre-write block-and-retry path. Optional at
    /// the TOML layer, default `false` — backward compatible with every
    /// `[[shard]]` index entry produced before Postcondition 7 existed.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sealed_retroactively: bool,
    /// P2-002 (S-25.02 F4 cluster-3 adversarial pass-2, HIGH; BC-1.18.008
    /// EC-002's own Canonical Test Vector): `true` iff this seal is the
    /// mechanism-A backfill-split's EC-002 single-oversized-record exception
    /// -- a record that alone exceeds `shard_cap_bytes`, sealed whole rather
    /// than split mid-record (`bytes_at_seal` MAY then exceed
    /// `shard_cap_bytes` for this ONE entry, a documented exception
    /// distinct from `sealed_retroactively`'s Postcondition 7 exception).
    /// `#[serde(default)]` -- backward compatible with every `[[shard]]`
    /// index entry produced before this field existed (mirrors
    /// `sealed_retroactively`'s own additive-field precedent).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub oversized_record: bool,
    /// P3-001 (S-25.02 F4 cluster-3 adversarial pass-3, HIGH; BC-1.18.008
    /// v1.4's Leading-Preamble Handling Rule, EC-007/EC-008): `true` iff this
    /// seal is a **preamble shard** — produced by Postcondition 2's Leading-
    /// Preamble Handling Rule sealing the artifact's leading preamble (YAML
    /// frontmatter + title/intro, plus — for `decision-log.md` — its table
    /// header/separator rows) as its own shard, either because
    /// `preamble_bytes + first_record_bytes > shard_cap_bytes` (EC-007's
    /// overflow case, `oversized_record: false`) or because the preamble
    /// ALONE exceeds `shard_cap_bytes` (EC-008's degenerate case,
    /// `oversized_record: true`). A preamble shard always carries
    /// `records: 0`. Distinguishes it from an ordinary record-bearing shard
    /// for downstream readers (Postcondition 3, Postcondition 4's retention
    /// composition, Postcondition 6(b)'s record-count accounting) without
    /// those readers having to re-derive record count from file content.
    /// `#[serde(default)]` -- backward compatible with every `[[shard]]`
    /// index entry produced before this field existed (mirrors
    /// `oversized_record`'s own additive-field precedent).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub is_preamble_shard: bool,
    /// P3-001 (BC-1.18.008 v1.4 Postcondition 3): this shard's own count of
    /// whole, native-format domain records
    /// ([`MechanismABackfillPartition::record_count`], surfaced through to
    /// the published index) — `0` for a preamble shard (Postcondition 2's
    /// Leading-Preamble Handling Rule) or for a non-mechanism-A entry (the
    /// BC-1.18.006 ongoing per-write roll mechanism has no record-level
    /// concept, mirroring `oversized_record`'s own N/A convention there).
    /// Always serialized (never `skip_serializing_if`) so a preamble shard's
    /// `records: 0` is distinguishable, on the wire, from the field being
    /// absent entirely -- Postcondition 3's own text requires a preamble
    /// shard's PUBLISHED index entry to carry `records: 0` explicitly, not
    /// merely default-deserialize to it. `#[serde(default)]` keeps every
    /// `[[shard]]` index entry produced before this field existed loading
    /// unchanged (mirrors `oversized_record`'s own additive-field
    /// precedent).
    #[serde(default)]
    pub records: u32,
}

/// The whole `<artifact-stem>.shard-index.toml` file (BC-1.18.006
/// Postcondition 5's schema; EXTENDED BC-1.18.007 Postcondition 1 with
/// `retention_count`, S-25.02 F4 BC-cluster 3, stub-only this burst).
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct ShardIndex {
    pub schema_version: u32,
    pub artifact_stem: String,
    pub current_shard: String,
    pub shard_cap_bytes: u64,
    pub max_single_record_bytes: u64,
    pub safety_margin_bytes: u64,
    pub practical_fuel_ceiling: u64,
    pub worst_case_fuel_per_byte: f64,
    /// BC-1.18.007 Postcondition 1: how many of this artifact's most-recent
    /// sealed shards stay ACTIVE (un-archived) — a config value read from
    /// the index, never hardcoded into this module's compaction logic
    /// (Postcondition 1's own "never hardcoding 10" requirement).
    /// `#[serde(default = "default_retention_count")]` keeps every
    /// `[[shard]]` index produced before BC-1.18.007 existed loading
    /// unchanged (backward compatible, mirrors `sealed_retroactively`'s own
    /// additive-field precedent above).
    #[serde(default = "default_retention_count")]
    pub retention_count: u32,
    #[serde(default, rename = "shard")]
    pub shards: Vec<ShardIndexEntry>,
}

/// Named crash-point error codes for BC-1.18.006's staged roll sequence
/// (Postcondition 1's partial-failure postconditions; ADR-051 §Decision 11).
/// `SealWriteFailed`/`TruncateFailedAfterSeal`/`IndexPublishFailedAfterTruncate`
/// (E-SHD-001/006/007) are reused VERBATIM (never a new code) by
/// Postcondition 7's retroactive invocation, catch point (i)/(ii) (ADR-051
/// §Decision 15 point 3). `BackstopProbeFailed` (E-SHD-008, BC-1.18.006 v1.6
/// Invariant 8 / EC-019, F-C2-P2-003) is a DISTINCT addition — it names the
/// `Write` arm's OWN dedicated crash-orphan backstop `stat()` probe failing
/// for a non-`NotFound` reason, which is not a staged-roll-sequence
/// partial-failure at all (no roll has started when this fires).
/// `SealedShardAlreadyExists` (E-SHD-009, BC-1.18.006 v1.8 Postcondition 8,
/// F-C2-P4-002) is ALSO a distinct addition — a write-once/immutability
/// violation detected by `publish_sealed_shard`'s own exclusive-create
/// primitive, refusing to overwrite an already-sealed shard at the
/// destination seq.
#[derive(Debug, Error)]
pub enum ShardRollError {
    /// Steps (a)-(b) fail: the canonical file is left completely untouched
    /// — safe, no data loss, no duplicate. The next dispatch attempt
    /// re-evaluates the trigger and re-attempts the FULL sequence from step
    /// (a).
    #[error(
        "E-SHD-001: shard-seal-write failure for artifact_stem \"{artifact_stem}\" — canonical \
         file left in its exact pre-roll state, still over cap: {source}"
    )]
    SealWriteFailed {
        artifact_stem: String,
        #[source]
        source: io::Error,
    },

    /// Step (c) fails after step (b) succeeded: the sealed shard durably
    /// exists (a byte-for-byte copy of the pre-roll content) AND the
    /// canonical file still holds that same content too (a transient,
    /// DETECTABLE duplicate-content state, not data loss). Self-heals via
    /// resume-from-truncate on the next dispatch
    /// ([`self_heal_resume_from_truncate`]; EC-010).
    #[error(
        "E-SHD-006: canonical-truncate failed after sealed-shard publish succeeded for \
         artifact_stem \"{artifact_stem}\" (sealed shard \"{sealed_path}\" already durable) — \
         resume-from-truncate self-heal required on next dispatch: {source}"
    )]
    TruncateFailedAfterSeal {
        artifact_stem: String,
        sealed_path: String,
        #[source]
        source: io::Error,
    },

    /// Step (d) fails after step (c) succeeded: the canonical file is
    /// CORRECTLY fresh and empty, and the sealed shard exists correctly on
    /// disk, but `<artifact-stem>.shard-index.toml` has not yet recorded
    /// the new `[[shard]]` entry (a discoverability-METADATA gap only — no
    /// reader-visible data loss). Self-heals via index reconciliation on
    /// the next dispatch
    /// ([`self_heal_reconcile_missing_index_entries`]; EC-011).
    #[error(
        "E-SHD-007: shard-index publish failed after canonical-truncate succeeded for \
         artifact_stem \"{artifact_stem}\" (sealed shard \"{sealed_path}\" already durable, \
         canonical already empty) — index reconciliation required on next dispatch: {source}"
    )]
    IndexPublishFailedAfterTruncate {
        artifact_stem: String,
        sealed_path: String,
        #[source]
        source: io::Error,
    },

    /// BC-1.18.006 v1.6 Invariant 8 / EC-019 (F-C2-P2-003, MAJOR): the
    /// `Write`-arm's own dedicated Postcondition 7 catch point (ii)
    /// crash-orphan backstop `stat()` probe (`current_shard_bytes_flat`)
    /// failed for a reason OTHER than `NotFound` (`NotFound` itself remains
    /// EC-004's legitimate first-write case, mapped to `Ok(0)` and never
    /// reaching this variant at all). `stat()` follows symlinks and can fail
    /// (e.g. `ELOOP`) in cases where `write_atomic`'s own `rename` need not
    /// dereference the final symlink component and could still succeed — so
    /// fail-OPEN here would let this probe silently skip while the `Write`
    /// itself proceeds underneath a symlink the probe could not see
    /// through, potentially destroying a crash-orphaned, un-sealed,
    /// over-cap canonical this dispatch never actually confirmed was safe
    /// to overwrite. No roll has started when this fires (unlike
    /// `SealWriteFailed`/`TruncateFailedAfterSeal`/
    /// `IndexPublishFailedAfterTruncate`, all genuine mid-roll partial-
    /// failure states) — the canonical file is left completely untouched,
    /// and no self-heal applies; the caller simply retries once the
    /// underlying I/O condition is resolved.
    #[error(
        "E-SHD-008: Write-arm backstop stat() failed for artifact_stem \"{artifact_stem}\" at \
         '{path}' — cannot confirm whether the canonical file is a crash-orphaned, over-cap \
         shard; refusing to let this Write proceed until the underlying I/O condition is \
         resolved: {source}"
    )]
    BackstopProbeFailed {
        artifact_stem: String,
        path: String,
        #[source]
        source: io::Error,
    },

    /// BC-1.18.006 v1.8 Postcondition 8 (F-C2-P4-002, MINOR, defense-in-
    /// depth): `publish_sealed_shard` is write-once — a sealed shard, once
    /// durably published at a given `<stem>.<seq:04>.md` path, is
    /// IMMUTABLE. This variant reports a detected attempt to publish a NEW
    /// seal at a destination that already exists on disk (e.g. a
    /// `next_seal_seq` collision with an already-sealed file — a
    /// filesystem/index desync, or a self-heal-ordering gap like
    /// F-C2-P4-001) — refused BEFORE any bytes are durably placed at that
    /// path, via an atomic exclusive-create primitive (never a plain
    /// stat()-then-write race, which would leave a TOCTOU window open). No
    /// `#[source]` `io::Error`: this is a detected INVARIANT violation, not
    /// an underlying I/O failure — the exclusive-create call itself
    /// succeeded in determining the destination already exists.
    #[error(
        "E-SHD-009: refusing to overwrite an already-sealed shard at '{sealed_path}' for \
         artifact_stem \"{artifact_stem}\" — sealed shards are write-once/immutable; this seq \
         already has durable content on disk"
    )]
    SealedShardAlreadyExists {
        artifact_stem: String,
        sealed_path: String,
    },

    /// FIX-MED-2 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-59/
    /// CWE-200, defense-in-depth): a governed artifact's canonical path was
    /// found to be a symlink at one of this module's canonical-path
    /// read/stat sites (`read_canonical_content`'s roll-time read; the
    /// hoisted `stat()` guard shared by ALL THREE `"flat"`-shape mutation-
    /// tool arms — `Write`'s `current_shard_bytes_flat` backstop probe, and
    /// `Edit`'s/`MultiEdit`'s IDENTICAL `current_shard_bytes_flat` trigger
    /// reads, per PR #824 pr-review Finding #3's sibling-callsite sweep;
    /// the `"frontmatter-changelog-array"` shape's `read_changelog_item_count`
    /// read (same Finding #3 sweep); `self_heal_resume_from_truncate`'s
    /// duplicate-content comparison read; and
    /// `reconcile_post_write_replace_all_overcap`'s size probe). If a
    /// governed artifact path is replaced with a symlink to a
    /// sensitive file elsewhere on the filesystem, silently reading/
    /// statting through it would let the symlink TARGET's bytes be
    /// durably sealed into a brand-new regular file — a real exfiltration
    /// channel (the sealed shard becomes ordinary repo content that can be
    /// committed/pushed). Detected via `symlink_metadata` (lstat — never
    /// dereferenced), mirroring `publish_sealed_shard`'s own SEC-001
    /// no-follow discipline. No `#[source]` `io::Error`, mirroring
    /// `SealedShardAlreadyExists` (E-SHD-009): this is a detected
    /// INVARIANT violation (the path IS a symlink), not an underlying I/O
    /// failure.
    #[error(
        "E-SHD-010: refusing to read or stat '{path}' for artifact_stem \"{artifact_stem}\" — \
         the canonical path is a symlink, not a regular file; refusing to dereference it \
         (symlink-exfiltration hardening, BC-1.18.006 FIX-MED-2)"
    )]
    CanonicalPathIsSymlink { artifact_stem: String, path: String },
}

/// Fail-loud roll errors surface to the dispatcher's handling path as
/// `HookResult::Error` (Invariant 1: "no version may return `Error` for a
/// normal (non-crash) over-cap condition" — a genuine crash mid-roll IS the
/// one case `Error` is correct for; a normal, uninterrupted roll returns
/// `HookResult::Block`, never routed through this `From` impl at all).
///
/// # WIRING-EXEMPT (BC-5.38.003)
///
/// `From<T>` blanket delegation to a single `Display`-forwarding call —
/// identical in shape to this file's existing, already-shipped
/// `From<ShardConfigError> for HookResult` impl above. No domain decision:
/// `ShardRollError`'s own `Display` impl (via `thiserror`) already carries
/// the full E-SHD-NNN-coded, artifact-stem-scoped diagnostic text.
impl From<ShardRollError> for HookResult {
    fn from(err: ShardRollError) -> Self {
        HookResult::Error {
            message: err.to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// Staged 4-step roll sequence (Postcondition 1; ADR-051 §Decision 11)
// ---------------------------------------------------------------------------

/// Map a [`last_amended_migrate::MigrateError`] (the only error type
/// `write_atomic` returns) down to the `std::io::Error` every
/// [`ShardRollError`] variant's `source` field carries. `write_atomic`
/// itself only ever constructs `MigrateError::Io { source, .. }` (see its
/// own doc comment — every one of its fallible steps wraps a genuine
/// filesystem operation), so the `Io` arm is the expected, common case; the
/// other `MigrateError` variants (none of which `write_atomic` can produce)
/// are handled defensively via `io::Error::other` rather than assumed
/// unreachable, so this mapping stays exhaustive-safe against a future
/// `write_atomic` change without ever panicking here.
fn migrate_err_to_io(err: last_amended_migrate::MigrateError) -> io::Error {
    match err {
        last_amended_migrate::MigrateError::Io { source, .. } => source,
        other => io::Error::other(other.to_string()),
    }
}

/// Best-effort `artifact_stem` derivation from a canonical path (e.g.
/// `"decision-log.md"` -> `"decision-log"`), used ONLY by a low-level step
/// function invoked directly (outside [`execute_roll`]'s own orchestration,
/// which always supplies the caller's authoritative `entry.artifact_stem`
/// instead via [`reattribute_roll_error`]) — this keeps every step function
/// independently unit-testable without requiring a full `ShardEntry` just to
/// report a crash-point error.
fn stem_from_canonical_path(canonical_path: &Path) -> String {
    canonical_path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Best-effort `artifact_stem` derivation from a sealed-shard filename (e.g.
/// `"decision-log.0001.md"` -> `"decision-log"`, stripping both the `.md`
/// extension and the `.<seq:04>` segment) — see
/// [`stem_from_canonical_path`]'s doc comment for why this fallback exists.
fn stem_from_sealed_path(sealed_path: &Path) -> String {
    let file_name = sealed_path.file_name().unwrap_or_default();
    let without_md = Path::new(file_name).file_stem().unwrap_or(file_name);
    Path::new(without_md)
        .file_stem()
        .unwrap_or(without_md)
        .to_string_lossy()
        .into_owned()
}

/// Re-point a [`ShardRollError`] returned by one of the staged step
/// functions at [`execute_roll`]'s own authoritative `artifact_stem`/
/// `sealed_path` context — the step functions themselves only see the
/// narrow slice of context their own signature carries (see
/// [`stem_from_canonical_path`]'s doc comment), so `execute_roll`, which
/// knows the full picture, corrects the diagnostic fields in place while
/// preserving the original `io::Error` source untouched.
fn reattribute_roll_error(
    err: ShardRollError,
    artifact_stem: &str,
    sealed_filename: &str,
) -> ShardRollError {
    match err {
        ShardRollError::SealWriteFailed { source, .. } => ShardRollError::SealWriteFailed {
            artifact_stem: artifact_stem.to_string(),
            source,
        },
        ShardRollError::TruncateFailedAfterSeal { source, .. } => {
            ShardRollError::TruncateFailedAfterSeal {
                artifact_stem: artifact_stem.to_string(),
                sealed_path: sealed_filename.to_string(),
                source,
            }
        }
        ShardRollError::IndexPublishFailedAfterTruncate { source, .. } => {
            ShardRollError::IndexPublishFailedAfterTruncate {
                artifact_stem: artifact_stem.to_string(),
                sealed_path: sealed_filename.to_string(),
                source,
            }
        }
        // `BackstopProbeFailed` (E-SHD-008) is constructed directly at the
        // `Write` arm's own backstop `stat()` call site and converted to a
        // `HookResult` immediately — it never passes through any of the
        // staged step functions this re-attribution exists for, so this
        // arm is unreachable in practice. Handled explicitly (never a
        // wildcard) so the match stays exhaustive-safe against future
        // `ShardRollError` variants: pass it through unchanged rather than
        // guessing at context this function was never given.
        other @ ShardRollError::BackstopProbeFailed { .. } => other,
        ShardRollError::SealedShardAlreadyExists { .. } => {
            ShardRollError::SealedShardAlreadyExists {
                artifact_stem: artifact_stem.to_string(),
                sealed_path: sealed_filename.to_string(),
            }
        }
        // FIX-MED-2 `CanonicalPathIsSymlink` (E-SHD-010) is, like
        // `BackstopProbeFailed` above, constructed directly at each of its
        // own guard call sites (already fully attributed with the correct
        // `artifact_stem`/`path` at construction time) and returned BEFORE
        // any of the staged step functions this re-attribution wraps ever
        // run — never passing through this function in practice. Handled
        // explicitly (never a wildcard) so the match stays exhaustive-safe
        // against future `ShardRollError` variants.
        other @ ShardRollError::CanonicalPathIsSymlink { .. } => other,
    }
}

/// This artifact's `<artifact-stem>.shard-index.toml` path, a sibling of
/// `canonical_path` (Postcondition 5's "one file per sharded mechanism-A
/// artifact" schema).
fn shard_index_path_for(canonical_path: &Path, artifact_stem: &str) -> PathBuf {
    shard_sibling_path(canonical_path, &format!("{artifact_stem}.shard-index.toml"))
}

/// Join `filename` onto `canonical_path`'s own parent directory — every
/// sealed shard and the shard-index TOML are siblings of the canonical file
/// (Postcondition 6's "stable-current-filename addressing" — all of a
/// sharded artifact's files live in the same directory).
fn shard_sibling_path(canonical_path: &Path, filename: &str) -> PathBuf {
    canonical_path
        .parent()
        .map(|dir| dir.join(filename))
        .unwrap_or_else(|| PathBuf::from(filename))
}

/// Load `<artifact-stem>.shard-index.toml` at `index_path`, if it exists.
/// `Ok(None)` (never an error) when the file is simply absent — the caller
/// treats that as "no roll has ever occurred yet for this artifact" and
/// synthesizes a fresh index. A genuine read failure (permission denied,
/// `ELOOP`, etc.) or a malformed-TOML parse failure both surface as `Err`.
fn load_shard_index(index_path: &Path) -> io::Result<Option<ShardIndex>> {
    match std::fs::read_to_string(index_path) {
        Ok(text) => {
            let index: ShardIndex = toml::from_str(&text).map_err(io::Error::other)?;
            Ok(Some(index))
        }
        // TD-VSDD-060 sibling sweep (PR #824 pr-review Finding #1): same
        // cross-platform disambiguation — see `is_genuinely_missing`'s own
        // doc comment.
        Err(e) if is_genuinely_missing(&e, index_path) => Ok(None),
        Err(e) => Err(e),
    }
}

/// The next monotonically-increasing seal `seq` for this artifact
/// (Postcondition 5: "`seq` incrementing monotonically from 1") — the
/// existing index's highest recorded `seq` plus one, or `1` when no index
/// exists yet (first-ever roll).
///
/// PR #824 pr-review Finding #9 (NIT): `checked_add` — not a bare `+ 1` —
/// so a recorded max `seq` already at `u32::MAX` (practically unreachable,
/// but not provably impossible) fails loud with a genuine `io::Error`
/// rather than panicking in debug builds or silently wrapping to `0` in
/// release, which would otherwise collide with (or precede) `seq=1`'s own
/// sealed shard. Both existing call sites already map any `next_seal_seq`
/// `io::Error` to `ShardRollError::SealWriteFailed` (`E-SHD-001`), so this
/// overflow surfaces through the SAME fail-loud path every other
/// `next_seal_seq` I/O failure already uses — no caller change needed.
fn next_seal_seq(index_path: &Path) -> io::Result<u32> {
    match load_shard_index(index_path)? {
        Some(index) => {
            let max_seq = index.shards.iter().map(|s| s.seq).max().unwrap_or(0);
            max_seq.checked_add(1).ok_or_else(|| {
                io::Error::other(format!(
                    "shard-index '{}' already records seq={max_seq} (u32::MAX) — cannot \
                     compute a next seq without overflowing (PR #824 pr-review Finding #9)",
                    index_path.display()
                ))
            })
        }
        None => Ok(1),
    }
}

/// FIX-MED-2 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-59/
/// CWE-200): refuses to let ANY of this module's canonical-path read/stat
/// sites operate on `canonical_path` if it is a symlink, rather than
/// silently dereferencing it. Extends `publish_sealed_shard`'s own
/// SEC-001 no-follow discipline (there applied to the sealed-shard
/// DESTINATION) to every read/stat site that instead operates on the
/// SOURCE canonical path during a roll — a governed artifact path
/// replaced with a symlink to a sensitive file elsewhere on the
/// filesystem must never have its target's bytes read and durably sealed
/// into a brand-new regular file (a real exfiltration channel: the sealed
/// shard becomes ordinary repo content that can be committed/pushed).
///
/// Uses `symlink_metadata` (lstat — never follows the final path
/// component), mirroring SEC-001's own no-follow discipline exactly. A
/// MISSING canonical (`NotFound`) is not this guard's concern — every one
/// of its call sites already has its own legitimate `NotFound` handling
/// (EC-004's first-write zero-case) — so `Ok(())` is returned for
/// `NotFound`, and for any OTHER `symlink_metadata` I/O error (e.g.
/// permission denied), letting the caller's own subsequent read/stat
/// surface that failure through its EXISTING error path, unchanged. Only
/// a CONFIRMED symlink fails loud here.
fn reject_canonical_symlink(
    artifact_stem: &str,
    canonical_path: &Path,
) -> Result<(), ShardRollError> {
    match std::fs::symlink_metadata(canonical_path) {
        Ok(meta) if meta.file_type().is_symlink() => Err(ShardRollError::CanonicalPathIsSymlink {
            artifact_stem: artifact_stem.to_string(),
            path: canonical_path.display().to_string(),
        }),
        Ok(_) | Err(_) => Ok(()),
    }
}

/// Step (a): read the canonical file's current full content — a one-time,
/// roll-only read (BC-1.18.006 Postcondition 1 step (a)). The cheap
/// per-write TRIGGER check (BC-1.18.005 Postcondition 2) remains
/// `stat()`-only; content is read ONLY once a roll is already confirmed
/// necessary. `std::fs::read` (bytes), not `read_to_string` (F-C2-P4-004,
/// ADVISORY, BC-1.18.006 v1.8, cluster-2 LOCAL adversary pass-4): sealing is
/// a byte-for-byte preservation operation with no UTF-8 requirement at all
/// — a canonical file containing non-UTF-8 bytes must still be sealable,
/// never fail with a spurious `E-SHD-001` merely because the content isn't
/// valid UTF-8. Consistent with the self-heal paths' own byte-level reads
/// (`self_heal_resume_from_truncate`, F-C2-P1-001).
///
/// F-C2-P7-001 (MAJOR, cluster-2 LOCAL adversary pass-7): a MISSING
/// canonical file (`io::ErrorKind::NotFound`) is the artifact's first-ever
/// write (BC-1.18.005 EC-004) and MUST be treated as a zero-byte current
/// shard per this BC's own Precondition 2 ("...or is treated as a
/// zero-byte current shard... if this is the artifact's first-ever
/// write") — mirroring the identical `NotFound -> Ok(0)` precedent already
/// established by [`current_shard_bytes_flat`] and
/// [`read_changelog_item_count`]. Any OTHER `io::Error` kind (permission
/// denied, a path component that is not a directory, etc.) still
/// propagates as a genuine `Err`, which `execute_roll` maps to
/// `ShardRollError::SealWriteFailed` (E-SHD-001) — this relief is scoped
/// ONLY to a missing canonical, never to every I/O failure.
pub fn read_canonical_content(canonical_path: &Path) -> io::Result<Vec<u8>> {
    match std::fs::read(canonical_path) {
        Ok(content) => Ok(content),
        // F-C2-P7-001: first-ever write -> treated as zero-byte content,
        // not an io::Error. PR #824 pr-review Finding #1 (BLOCKING,
        // Windows-only): `is_genuinely_missing` — not a bare
        // `e.kind() == NotFound` — so a path traversing through a
        // non-directory component (Windows `ERROR_PATH_NOT_FOUND`, mapped
        // to the SAME `io::ErrorKind::NotFound` as a genuine
        // `ERROR_FILE_NOT_FOUND`) still propagates as `Err` on every
        // platform, never silently relieved to `Ok(vec![])`.
        Err(e) if is_genuinely_missing(&e, canonical_path) => Ok(vec![]),
        Err(e) => Err(e),
    }
}

/// Atomically create a BRAND-NEW file at `path` containing exactly
/// `content`'s bytes — write-once, no-clobber (BC-1.18.006 v1.8
/// Postcondition 8 / F-C2-P4-002, MINOR, defense-in-depth). Fails with
/// `io::ErrorKind::AlreadyExists` if `path` already exists, via an atomic
/// EXCLUSIVE-CREATE primitive rather than a separate stat()-then-write
/// check — closing the TOCTOU race window a plain "does it exist?" probe
/// followed by a possibly-overwriting write would leave open.
///
/// Mechanism: write `content` to a sibling temp file (`fsync`'d for
/// durability, mirroring `last_amended_migrate::atomic_write::write_atomic`'s
/// own durability discipline), then [`std::fs::hard_link`] the temp file
/// ONTO `path`. `link(2)` (Unix) / `CreateHardLinkW` (Windows) atomically
/// fails with `EEXIST` (`ErrorKind::AlreadyExists`) if `path` already
/// exists — unlike `rename(2)`, which unconditionally OVERWRITES an
/// existing destination — so there is no window between "check if `path`
/// exists" and "create it" for a racing writer to land in. The temp file is
/// removed afterward regardless of outcome: on success its content also
/// durably lives at `path` via the hard link; on failure it was never
/// linked to `path` at all.
///
/// FIX-HIGH-1 (S-25.02 PR #824 second-security-review, HIGH, CWE-59/
/// CWE-367/CWE-377): the temp path is created via
/// `OpenOptions::new().write(true).create_new(true)` (`O_EXCL`, Unix /
/// `CREATE_NEW`, Windows) rather than a plain `File::create`
/// (`O_CREAT|O_WRONLY|O_TRUNC`, no `O_EXCL`): per POSIX, `create_new` fails
/// with `ErrorKind::AlreadyExists` if the last path component is a symlink
/// — dangling or not, regardless of its target — WITHOUT ever
/// dereferencing it. Without this, a plain `File::create` would follow a
/// pre-planted symlink and write the sealed content THROUGH it into an
/// arbitrary attacker-chosen file, and the subsequent `hard_link` below
/// (which does NOT dereference a symlink `src` by default) would then
/// hard-link the SYMLINK ITSELF onto `path` — turning the "sealed shard"
/// into a symlink and defeating write-once immutability.
///
/// PR #824 pr-review Finding #2 (MAJOR): the temp path ALSO carries a
/// random nonce (`random_nonce`), not merely `.{basename}.tmp-{pid}` — a
/// fully deterministic path (observable via `ps`/`/proc`, since `pid` is
/// the only variable) meant every retried/repeated call for the SAME
/// destination collided on the exact SAME temp path, so a single stale
/// leftover (or a pre-planted symlink) permanently blocked every
/// subsequent seal attempt for that artifact_stem/seq — and, because
/// [`publish_sealed_shard`] could not tell "the temp path collided" apart
/// from "the destination already exists", it misreported the former as
/// `E-SHD-009` (falsely implying real sealed history exists) and, on its
/// 0-byte-reclaim retry, unlinked a reclaimable destination for a failure
/// that had nothing to do with it. The nonce makes each call's temp path
/// its own, closing the self-perpetuating collision; [`WriteExclusiveError`]
/// closes the misattribution by keeping "temp path occupied" and
/// "destination occupied" as distinct, never-conflated outcomes.
#[derive(Debug)]
enum WriteExclusiveError {
    /// `create_new` on the TEMP path itself failed with `AlreadyExists` —
    /// `path` (the destination) was never even touched.
    TempPathOccupied(io::Error),
    /// The temp file was created and durably written, but `hard_link`
    /// failed because `path` (the destination) already exists.
    DestinationOccupied,
    /// Any other I/O failure at either step.
    Io(io::Error),
}

impl From<StageError> for WriteExclusiveError {
    fn from(err: StageError) -> Self {
        match err {
            StageError::TempPathOccupied(e) => WriteExclusiveError::TempPathOccupied(e),
            StageError::Io(e) => WriteExclusiveError::Io(e),
        }
    }
}

impl From<PublishError> for WriteExclusiveError {
    fn from(err: PublishError) -> Self {
        match err {
            PublishError::DestinationOccupied => WriteExclusiveError::DestinationOccupied,
            PublishError::Io(e) => WriteExclusiveError::Io(e),
        }
    }
}

/// A collision-resistant (never cryptographically-unpredictable — `O_EXCL`
/// is what actually defeats a determined adversary, PR #824 pr-review
/// Finding #2) value mixed into `stage_temp_file`'s temp-file path, so
/// repeated/retried calls for the SAME destination each get their OWN temp
/// path rather than colliding on a single deterministic one. No new
/// dependency (`rand`): mixes a per-process monotonic counter, wall-clock
/// nanoseconds, and a stack address (differs per thread) — sufficient for
/// collision-resistance across concurrent/retried calls, which is the
/// actual failure mode Finding #2 identifies.
fn random_nonce() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let counter = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let stack_addr = std::ptr::addr_of!(counter) as u64;
    nanos ^ counter.wrapping_mul(0x9E37_79B9_7F4A_7C15) ^ stack_addr
}

/// Half of `write_exclusive`'s two-step sequence: create a FRESH temp file
/// (collision-resistant path, [`random_nonce`]) containing `content`,
/// fully durable (`fsync`'d) — NEVER touching `final_path` itself. Returns
/// the temp file's own path on success; the caller publishes it onto
/// `final_path` via [`publish_staged_temp_file`].
///
/// PR #824 pr-review Finding #2 (MAJOR): split out of `write_exclusive` so
/// [`publish_sealed_shard`]'s 0-byte-reclaim retry can stage the RETRY's
/// content BEFORE unlinking the reclaimable destination — a staging
/// failure here (temp-path collision or any other I/O error) therefore
/// NEVER destroys the destination at all, closing the "a failed op
/// destroys a reclaimable 0-byte destination" defect.
///
/// N-1 (PR #824 pr-review cycle 2, MAJOR call-site regression coverage):
/// checks [`FORCE_STAGE_FAILURE`] first, in `#[cfg(test)]` builds only —
/// see that seam's own doc comment for why call-site tests need it (this
/// function's own real per-call randomness makes its actual temp path
/// impossible for a test to predict/pre-occupy). Compiles to nothing
/// outside test builds, so production's own `stage_temp_file` never
/// carries this branch.
fn stage_temp_file(final_path: &Path, content: &[u8]) -> Result<PathBuf, StageError> {
    #[cfg(test)]
    if let Some(kind) = FORCE_STAGE_FAILURE.with(|cell| {
        let mut state = cell.borrow_mut();
        match state.take() {
            Some((0, kind)) => Some(kind),
            Some((remaining, kind)) => {
                *state = Some((remaining - 1, kind));
                None
            }
            None => None,
        }
    }) {
        return Err(match kind {
            ForcedStageFailureKind::TempPathOccupied => {
                StageError::TempPathOccupied(io::Error::new(
                    io::ErrorKind::AlreadyExists,
                    "N-1 test-forced temp-path collision",
                ))
            }
            ForcedStageFailureKind::Io => {
                StageError::Io(io::Error::other("N-1 test-forced staging I/O failure"))
            }
        });
    }
    stage_temp_file_with_nonce(final_path, content, random_nonce())
}

/// N-1 (PR #824 pr-review cycle 2, MAJOR): which synthetic [`StageError`] a
/// forced [`stage_temp_file`] failure should produce — see
/// [`FORCE_STAGE_FAILURE`]'s own doc comment.
#[cfg(test)]
#[derive(Debug, Clone, Copy)]
enum ForcedStageFailureKind {
    /// Mirrors a genuine temp-path collision (`create_new`'s
    /// `AlreadyExists`) — the specific shape `publish_sealed_shard`'s
    /// `TempPathOccupied` match arm distinguishes from `DestinationOccupied`.
    TempPathOccupied,
    /// Mirrors any other I/O failure at the staging step.
    Io,
}

#[cfg(test)]
thread_local! {
    /// N-1 call-site regression coverage (PR #824 pr-review cycle 2): lets
    /// a test force `stage_temp_file` to fail on a specific (0-indexed,
    /// 1-shot) upcoming call on THIS thread, without needing to predict
    /// [`random_nonce`]'s deliberately-unpredictable output — the only way
    /// a test could otherwise pre-occupy the EXACT temp path
    /// `stage_temp_file` will compute. `publish_sealed_shard` calls
    /// `stage_temp_file` at most twice per invocation (the initial attempt
    /// inside `write_exclusive`, call #0, and the 0-byte-reclaim retry's
    /// own staging call, call #1) — set via
    /// [`force_stage_temp_file_failure`], which returns a guard clearing
    /// this state on drop (including on test panic/unwind), so a forced
    /// failure can never leak into another test sharing this worker
    /// thread. `#[cfg(test)]`-gated — compiles to nothing outside test
    /// builds, so production's own `stage_temp_file` never carries this
    /// branch.
    static FORCE_STAGE_FAILURE: std::cell::RefCell<Option<(u32, ForcedStageFailureKind)>> =
        const { std::cell::RefCell::new(None) };
}

/// Clears [`FORCE_STAGE_FAILURE`] on drop — see
/// [`force_stage_temp_file_failure`].
#[cfg(test)]
struct ForceStageFailureGuard;

#[cfg(test)]
impl Drop for ForceStageFailureGuard {
    fn drop(&mut self) {
        FORCE_STAGE_FAILURE.with(|cell| *cell.borrow_mut() = None);
    }
}

/// Forces the call to `stage_temp_file` `calls_to_allow` calls from now
/// (0 = the VERY NEXT call, on THIS thread) to fail with `kind`, letting a
/// test reach `publish_sealed_shard`'s own staging call sites
/// deterministically — see [`FORCE_STAGE_FAILURE`]'s own doc comment.
#[cfg(test)]
fn force_stage_temp_file_failure(
    calls_to_allow: u32,
    kind: ForcedStageFailureKind,
) -> ForceStageFailureGuard {
    FORCE_STAGE_FAILURE.with(|cell| *cell.borrow_mut() = Some((calls_to_allow, kind)));
    ForceStageFailureGuard
}

/// [`stage_temp_file`], parameterized on its own nonce rather than always
/// calling [`random_nonce`] internally — lets tests reproduce a KNOWN temp
/// path deterministically (to plant a fixture symlink/collision at it)
/// without weakening production's own real per-call randomness, which
/// always goes through the [`stage_temp_file`] wrapper above.
fn stage_temp_file_with_nonce(
    final_path: &Path,
    content: &[u8],
    nonce: u64,
) -> Result<PathBuf, StageError> {
    let parent = final_path.parent().unwrap_or_else(|| Path::new("."));
    let basename = final_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "shard".to_string());
    let tmp_path = parent.join(format!(
        ".{basename}.tmp-{}-{nonce:016x}",
        std::process::id()
    ));

    let mut file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp_path)
    {
        Ok(file) => file,
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            return Err(StageError::TempPathOccupied(e));
        }
        Err(e) => return Err(StageError::Io(e)),
    };

    if let Err(e) = file.write_all(content).and_then(|()| file.sync_all()) {
        // We created this temp file ourselves — clean it up on this early
        // return (PR #824 pr-review Finding #2: the prior revision's
        // late/single `remove_file` call, reached only after `hard_link`,
        // never ran for a write_all/sync_all failure, leaking the temp
        // file permanently on ENOSPC/EIO).
        let _ = std::fs::remove_file(&tmp_path);
        return Err(StageError::Io(e));
    }

    Ok(tmp_path)
}

/// The other half of `write_exclusive`'s two-step sequence: hard-link the
/// already-staged `tmp_path` onto `final_path` — an atomic exclusive-
/// create per this module's own write-once discipline — then best-effort
/// remove the temp file regardless of outcome (on success its content
/// also durably lives at `final_path` via the hard link; on failure it
/// was never linked there at all).
fn publish_staged_temp_file(tmp_path: &Path, final_path: &Path) -> Result<(), PublishError> {
    let link_result = std::fs::hard_link(tmp_path, final_path);
    // Best-effort cleanup regardless of outcome — a leftover staged temp
    // file here is a secondary symptom, never the primary error reported.
    let _ = std::fs::remove_file(tmp_path);

    match link_result {
        Ok(()) => {
            // Best-effort directory fsync (Unix-only, mirroring
            // `write_atomic`'s own precedent) so the hard-link's
            // directory-entry update is itself durable across a crash, not
            // just the file's bytes.
            #[cfg(unix)]
            if let Some(parent) = final_path.parent()
                && let Ok(dir) = std::fs::File::open(parent)
            {
                let _ = dir.sync_all();
            }
            Ok(())
        }
        Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
            Err(PublishError::DestinationOccupied)
        }
        Err(e) => Err(PublishError::Io(e)),
    }
}

/// [`stage_temp_file`]'s own narrow error surface — never
/// `DestinationOccupied`, since staging never touches `final_path`.
#[derive(Debug)]
enum StageError {
    TempPathOccupied(io::Error),
    Io(io::Error),
}

/// [`publish_staged_temp_file`]'s own narrow error surface — never
/// `TempPathOccupied`, since the temp file was already staged
/// successfully by the time this runs.
#[derive(Debug)]
enum PublishError {
    DestinationOccupied,
    Io(io::Error),
}

/// Atomically create a BRAND-NEW file at `path` containing exactly
/// `content`'s bytes — write-once, no-clobber (BC-1.18.006 v1.8
/// Postcondition 8 / F-C2-P4-002, MINOR, defense-in-depth). Fails with
/// [`WriteExclusiveError::DestinationOccupied`] if `path` already exists,
/// via an atomic EXCLUSIVE-CREATE primitive rather than a separate
/// stat()-then-write check — closing the TOCTOU race window a plain "does
/// it exist?" probe followed by a possibly-overwriting write would leave
/// open. Composes [`stage_temp_file`] then [`publish_staged_temp_file`];
/// see either's own doc comment for the FIX-HIGH-1/Finding #2 mechanism
/// detail.
fn write_exclusive(path: &Path, content: &[u8]) -> Result<(), WriteExclusiveError> {
    let tmp_path = stage_temp_file(path, content)?;
    publish_staged_temp_file(&tmp_path, path).map_err(Into::into)
}

/// Step (b): publish the sealed shard as a brand-NEW file at
/// `<stem>.<seq:04>.md` (BC-1.18.006 Postcondition 1 step (b)) via
/// [`write_exclusive`] — an exclusive-create that fails loud
/// (`E-SHD-009`, F-C2-P4-002) rather than silently overwriting if the
/// destination already exists, never interrupting any reader of the
/// canonical path (sealed filenames are never read by shard-unaware code).
///
/// # 0-byte-destination exception (BC-1.18.006 v1.9 Postcondition 8,
/// F-C2-P8-002, MEDIUM; EC-024/EC-025)
///
/// A 0-byte file at the destination `seq` path can never be genuine sealed
/// history — this BC's own write paths only ever seal non-empty content
/// (Postcondition 1's empty-canonical short-circuit and Postcondition 3's
/// cap guarantee together ensure this) — so it is structurally always an
/// external anomaly with nothing durable to protect. On an `AlreadyExists`
/// collision, this `lstat()`s (`symlink_metadata` — never follows a symlink
/// at the destination, SEC-001) the destination EXACTLY ONCE; if it is a
/// REGULAR FILE (not a symlink) and exactly 0 bytes, it STAGES the retry's
/// content via [`stage_temp_file`] FIRST, THEN `unlink`s the 0-byte
/// destination, THEN publishes the already-staged temp file via
/// [`publish_staged_temp_file`] — never re-invoking [`write_exclusive`]
/// itself for the retry, and never a loop (bounding a racing concurrent
/// writer to a single extra attempt). N-3 (PR #824 pr-review cycle 2):
/// staging BEFORE unlinking, not after, is the whole point of `ef6ca3b4`'s
/// fix (see the "Finding #2" paragraph immediately below) — a staging
/// failure on the retry therefore leaves the reclaimable 0-byte
/// destination COMPLETELY UNTOUCHED, rather than an unlink-then-stage
/// ordering that would delete it before ever discovering the retry's own
/// staging failure. A successful reclaim emits a `tracing::warn!`
/// diagnostic and does NOT fail the dispatch. If the destination is
/// non-empty, IS a symlink (SEC-001 — never dereferenced/reclaimed
/// through), or the single retry ALSO collides (a genuine race), this
/// fails loud with `E-SHD-009`/[`ShardRollError::SealedShardAlreadyExists`]
/// exactly as the write-once guarantee requires for real sealed content.
///
/// PR #824 pr-review Finding #2 (MAJOR): `write_exclusive`'s
/// [`WriteExclusiveError`] distinguishes a TEMP-path collision from a
/// DESTINATION collision — only the latter enters the 0-byte-reclaim
/// logic below; a temp-path collision (or any other I/O failure) maps
/// straight to `E-SHD-001`/[`ShardRollError::SealWriteFailed`], the same
/// code every other genuine write-side failure already uses, rather than
/// the misleading `E-SHD-009` (which falsely implies real sealed history
/// exists at `sealed_path`) the prior revision reported for BOTH cases
/// alike.
pub fn publish_sealed_shard(sealed_path: &Path, content: &[u8]) -> Result<(), ShardRollError> {
    match write_exclusive(sealed_path, content) {
        Ok(()) => return Ok(()),
        Err(WriteExclusiveError::DestinationOccupied) => {
            // Fall through to the 0-byte-reclaim logic below — the ONLY
            // outcome that logic is entitled to react to.
        }
        Err(WriteExclusiveError::TempPathOccupied(source) | WriteExclusiveError::Io(source)) => {
            return Err(ShardRollError::SealWriteFailed {
                artifact_stem: stem_from_sealed_path(sealed_path),
                source,
            });
        }
    }

    let already_exists_err = || ShardRollError::SealedShardAlreadyExists {
        artifact_stem: stem_from_sealed_path(sealed_path),
        sealed_path: sealed_path.display().to_string(),
    };

    // EC-025: the collision is only reclaimable if the pre-existing
    // destination is exactly 0 bytes. A failed/inconclusive `stat()` (e.g.
    // the file vanished between the collision and this check) is treated
    // as "not reclaimable" — never assumed 0 bytes — falling through to the
    // loud E-SHD-009 refusal below rather than risking an unlink of
    // content this call never confirmed was empty.
    //
    // SEC-001 (security review, MEDIUM, CWE-61/CWE-367): this probe MUST
    // use `symlink_metadata` (lstat — never follows the final path
    // component) rather than `std::fs::metadata` (stat — dereferences
    // symlinks). An attacker with write access to the shard directory
    // could otherwise plant a symlink at the exact seal destination
    // pointing at some unrelated 0-byte-reporting path (e.g. `/dev/null`),
    // and a following `stat()` would judge the destination "reclaimable"
    // by reading THROUGH the symlink rather than the symlink itself. The
    // subsequent `std::fs::remove_file`/`unlink()` call does not dereference
    // symlinks either, so no arbitrary file is ever deleted by that step —
    // but the dereferencing `stat()` read is itself a symlink-follow this
    // write-once guard must never perform, mirroring the no-follow
    // discipline the Write-arm's own crash-orphan backstop probe already
    // applies (`current_shard_bytes_flat`, which fails loud on `ELOOP`
    // rather than silently reading through a symlink). A destination that
    // IS a symlink is therefore never eligible for reclaim — treated
    // exactly like a non-empty pre-existing destination and routed to the
    // same loud E-SHD-009 refusal, never dereferenced or unlinked.
    let is_zero_byte = std::fs::symlink_metadata(sealed_path)
        .map(|meta| !meta.file_type().is_symlink() && meta.len() == 0)
        .unwrap_or(false);

    if !is_zero_byte {
        // EC-024: a non-empty pre-existing destination is real sealed
        // history — write-once immutability is unweakened; refuse loud,
        // leave it byte-identical and untouched.
        return Err(already_exists_err());
    }

    // PR #824 pr-review Finding #2 (MAJOR): STAGE the retry's content
    // BEFORE unlinking the reclaimable 0-byte destination, never after.
    // The prior revision unlinked first and only then attempted
    // `write_exclusive`'s own internal staging — so a staging failure on
    // the retry (e.g. a colliding temp path) left the destination already
    // deleted with nothing published in its place, even though the
    // failure had nothing to do with the destination at all. Staging
    // first means a staging failure here leaves the reclaimable 0-byte
    // destination COMPLETELY UNTOUCHED.
    let staged_tmp_path = match stage_temp_file(sealed_path, content) {
        Ok(tmp_path) => tmp_path,
        Err(StageError::TempPathOccupied(source) | StageError::Io(source)) => {
            return Err(ShardRollError::SealWriteFailed {
                artifact_stem: stem_from_sealed_path(sealed_path),
                source,
            });
        }
    };

    // FIX-MED-1 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-367;
    // MAJOR-1, PR #824 pr-review cycle 3): re-verify identity IMMEDIATELY
    // BEFORE unlinking — the `is_zero_byte` probe above and the
    // `remove_file` below are two SEPARATE syscalls, leaving a TOCTOU
    // window in between. If a second, legitimate concurrent writer
    // replaces this 0-byte placeholder with real sealed content in that
    // window, nothing before this point would notice — `remove_file` would
    // silently discard genuine sealed history. `reclaim_identity_still_safe`
    // re-checks via an ALREADY-OPEN file handle's own metadata, which is
    // immune to a subsequent rename/replace of the PATH (it inspects the
    // inode this call has open, not whatever inode currently occupies the
    // path name) — closing the window as tightly as `std::fs` allows
    // without a new dependency. MAJOR-1 (PR #824 pr-review cycle 3): this
    // re-check MUST run AFTER `stage_temp_file` above, not before it —
    // Finding #2's fix inserted a full staged write (including an
    // `fsync`/`sync_all`) between the re-check and the unlink, which
    // silently re-widened the very window this re-check exists to close
    // (a durable write is a far larger TOCTOU window than "two adjacent
    // syscalls"). Running the re-check here, immediately before
    // `remove_file`, restores adjacency while still preserving Finding #2's
    // own guarantee that a staging failure leaves the destination
    // untouched — the ordering probe pinned by
    // `test_MAJOR1_reclaim_identity_recheck_runs_immediately_before_unlink_not_before_staging`
    // below fails if a future change moves this call back above
    // `stage_temp_file`. If the re-check fails (content changed, no longer
    // 0 bytes, vanished, etc.), abort the reclaim and fail loud via the
    // SAME E-SHD-009 collision error SEC-001's fix already uses, rather
    // than silently proceeding as if the reclaim were still safe — cleaning
    // up the now-orphaned staged temp file first (the same leak class
    // `stage_temp_file` already guards internally).
    if !reclaim_identity_still_safe(sealed_path) {
        let _ = std::fs::remove_file(&staged_tmp_path);
        return Err(already_exists_err());
    }

    // A failed unlink here (e.g. permission denied) leaves the 0-byte file
    // in place with nothing reclaimed — fail loud rather than silently
    // treating an unconfirmed reclaim as success (Invariant 1's "no version
    // may silently proceed past a condition it cannot verify safe",
    // extended to the reclaim step itself). The already-staged temp file
    // is never linked anywhere in this branch — best-effort cleanup so it
    // is never left behind as a leftover.
    if std::fs::remove_file(sealed_path).is_err() {
        let _ = std::fs::remove_file(&staged_tmp_path);
        return Err(already_exists_err());
    }

    match publish_staged_temp_file(&staged_tmp_path, sealed_path) {
        Ok(()) => {
            tracing::warn!(
                sealed_path = %sealed_path.display(),
                "BC-1.18.006 v1.9 Postcondition 8: reclaimed a 0-byte pre-existing file at the \
                 sealed-shard destination path (no durable sealed history to protect — a \
                 structural anomaly, never output of this BC's own write paths) and published \
                 the real seal content there"
            );
            Ok(())
        }
        // Genuine race: a concurrent writer placed real content at the
        // path between the unlink above and this single bounded retry —
        // never retried again (no loop), fail loud exactly as the
        // non-empty case does.
        Err(PublishError::DestinationOccupied) => Err(already_exists_err()),
        Err(PublishError::Io(source)) => Err(ShardRollError::SealWriteFailed {
            artifact_stem: stem_from_sealed_path(sealed_path),
            source,
        }),
    }
}

/// FIX-MED-1 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-367):
/// re-verifies, via an ALREADY-OPEN file handle's own metadata, that
/// `path` is STILL a 0-byte regular file — called by
/// [`publish_sealed_shard`]'s 0-byte reclaim path immediately before the
/// unlink, to close the TOCTOU window between the earlier
/// `symlink_metadata` probe and the `remove_file` call as tightly as
/// `std::fs` allows without a new dependency. MAJOR-1 (PR #824 pr-review
/// cycle 3): "immediately before the unlink" is a load-bearing placement
/// claim, not decorative — Finding #2's fix once inserted a full staged
/// write (`stage_temp_file`, including an `fsync`/`sync_all`) between this
/// call and the unlink, silently re-widening the window this call exists
/// to close. `publish_sealed_shard` now calls [`stage_temp_file`] FIRST
/// and this re-check LAST, immediately before `remove_file`, so the claim
/// in this paragraph is (again) literally true of the call site.
///
/// `File::open` (unlike `symlink_metadata`/`lstat`) DOES dereference a
/// symlink, so if the path was replaced by a symlink in the race window a
/// plain open could not itself detect that substitution. N-4 (PR #824
/// pr-review cycle 2, resolving this paragraph's own PRIOR self-
/// contradiction with the "Finding #4" paragraph below, which correctly
/// states the CURRENT truth): on Unix, this gap is now CLOSED — the open
/// below additionally passes `O_NOFOLLOW` (PR #824 pr-review Finding #4),
/// so a symlink substituted at `path` fails the open outright rather than
/// being silently dereferenced. Only the `#[cfg(not(unix))]` fallback
/// below still has this residual gap: no portable `O_NOFOLLOW`-equivalent
/// is available from `std::fs` alone there, and adding one would require
/// a new dependency (`libc`/`nix`/a platform-specific crate), out of scope
/// for this defense-in-depth hardening. That fallback is still safe in
/// practice for the same reason it always was: the caller's subsequent
/// `remove_file`/`unlink()` never dereferences a symlink either (SEC-001's
/// own no-follow discipline), so a symlink substituted in that narrower
/// window is itself unlinked — never its target — and
/// [`write_exclusive`]'s `O_EXCL` retry (FIX-HIGH-1) still refuses to
/// write through any symlink that manages to reappear at the destination
/// on the retry. What this check DOES close on every platform is the
/// window this fix targets: a second, legitimate concurrent writer
/// replacing the 0-byte placeholder with real (non-empty) sealed content,
/// which `remove_file` would otherwise silently discard.
///
/// Returns `false` — never reclaimable — for: on Unix, ANY symlink at all
/// (rejected outright by `O_NOFOLLOW`, regardless of the target's
/// content — NIT-2, PR #824 pr-review cycle 3, correcting a prior revision
/// of this paragraph that overstated the condition as "a symlink whose
/// target is non-empty", true only of the `#[cfg(not(unix))]` fallback
/// below, which has no `O_NOFOLLOW`-equivalent and so DOES dereference a
/// symlink, judging it solely on the target's own size); for a file whose
/// size changed since the first probe; and for a path that vanished or
/// became inaccessible between the two checks (a failed/inconclusive open
/// is treated as "not confirmed safe", never assumed safe). The caller
/// treats every `false` identically: abort the reclaim, fail loud with the
/// existing E-SHD-009 collision error.
///
/// PR #824 pr-review Finding #4 (MINOR): on Unix, the open is ALSO issued
/// with `O_NONBLOCK`. A plain BLOCKING open of a FIFO for `O_RDONLY` waits
/// for a writer to connect — with no writer ever connecting (the actual
/// shape of the race this check must survive), that wait is INDEFINITE,
/// hanging the PreToolUse dispatch this check gates.
///
/// N-5 (PR #824 pr-review cycle 2, NIT): `O_NONBLOCK` prevents that hang,
/// but NOT by making the open itself fail — correcting this doc comment's
/// own prior claim (and this fix's own commit message) that a FIFO "fails
/// the open immediately". Per POSIX, `O_RDONLY | O_NONBLOCK` on a FIFO
/// with no writer instead SUCCEEDS immediately, returning a valid file
/// descriptor (only `O_WRONLY | O_NONBLOCK` with no reader fails, with
/// `ENXIO` — not the read-only open this function issues). The FIFO is
/// then correctly judged NOT reclaim-safe by the SAME
/// `meta.file_type().is_file()` check every other non-regular-file case
/// already uses: `is_file()` is `false` for a FIFO's metadata, exactly as
/// it already is for a directory. A future reader must not remove
/// `O_NONBLOCK` believing it is load-bearing for the open FAILING — it is
/// load-bearing for the open never HANGING.
///
/// A regular file's open behavior is entirely unaffected by `O_NONBLOCK`
/// (it only changes FIFO/device/socket open semantics), so every existing
/// non-FIFO caller sees no behavior change. Neither flag's raw value is
/// exposed by `std::fs`/`std::os::unix`, so both are hardcoded per-OS
/// below (stable, well-known ABI constants) rather than pulling in a new
/// `libc`/`nix` dependency for two `i32` values.
fn reclaim_identity_still_safe(path: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;

        // N-7 (PR #824 pr-review cycle 2, NIT; corrected NIT-1, PR #824
        // pr-review cycle 3 — the cycle-2 wording was itself inverted and
        // self-contradictory, naming "Linux" on both sides of the
        // distinction): Android is Linux-ABI (these raw fcntl.h values
        // match) but NOT `target_os = "linux"` — a Linux-only cfg would
        // silently hand it the macOS/BSD values below instead. The raw
        // value `0x0100` is `O_NOFOLLOW` on macOS/BSD and `O_NOCTTY` on
        // Linux/Android (Linux's own `O_NOFOLLOW` is `0o400_000`, the
        // Linux/Android constant below) — handing Android the macOS/BSD
        // value would silently substitute `O_NOCTTY` where `O_NOFOLLOW` was
        // intended, the wrong flag entirely. Empirically confirmed correct
        // as written: `test_N4_reclaim_identity_still_safe_rejects_symlink_via_o_nofollow`
        // passes on a macOS host, which it could only do if `0x0100` really
        // is `O_NOFOLLOW` on darwin. Theoretical today (Android is not a
        // shipped release target: darwin-arm64/x86_64, linux-x86_64,
        // linux-musl, windows-x86_64; musl is `target_os = "linux"`), but
        // free to fix correctly.
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const O_NONBLOCK: i32 = 0o4000;
        #[cfg(any(target_os = "linux", target_os = "android"))]
        const O_NOFOLLOW: i32 = 0o400_000;
        // macOS/BSD raw fcntl.h values — distinct from Linux/Android's.
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const O_NONBLOCK: i32 = 0x0004;
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        const O_NOFOLLOW: i32 = 0x0100;

        std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(O_NONBLOCK | O_NOFOLLOW)
            .open(path)
            .and_then(|f| f.metadata())
            .map(|meta| meta.file_type().is_file() && meta.len() == 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        std::fs::File::open(path)
            .and_then(|f| f.metadata())
            .map(|meta| meta.file_type().is_file() && meta.len() == 0)
            .unwrap_or(false)
    }
}

/// Step (c): atomically REPLACE the canonical file's content with empty via
/// the SAME `write_atomic` temp-file-then-rename primitive (BC-1.18.006
/// Postcondition 1 step (c); Invariant 2/3) — never a delete-then-create,
/// never a rename of the canonical path away; the canonical path resolves
/// to SOME valid file at every observable instant.
pub fn truncate_canonical_to_empty(canonical_path: &Path) -> Result<(), ShardRollError> {
    last_amended_migrate::atomic_write::write_atomic(canonical_path, "").map_err(|e| {
        ShardRollError::TruncateFailedAfterSeal {
            artifact_stem: stem_from_canonical_path(canonical_path),
            // Unknown at this narrow call level (this function receives no
            // sealed_path parameter) — execute_roll's own orchestration
            // corrects this field to the real sealed filename via
            // reattribute_roll_error immediately after this call returns.
            sealed_path: String::new(),
            source: migrate_err_to_io(e),
        }
    })
}

/// Step (d): atomically publish the updated shard-index TOML (BC-1.18.006
/// Postcondition 1 step (d); Postcondition 5's schema) — loads the existing
/// `<artifact-stem>.shard-index.toml` sibling to `canonical_path` (if any,
/// else synthesizes a fresh index from `entry`'s cap-formula inputs),
/// appends exactly one new `[[shard]]` entry with `seq` incrementing
/// monotonically from 1, and republishes via `write_atomic`.
pub fn publish_shard_index_update(
    index_path: &Path,
    entry: &ShardEntry,
    new_shard_entry: ShardIndexEntry,
) -> Result<ShardIndex, ShardRollError> {
    let sealed_path = new_shard_entry.path.clone();
    let to_error = |source: io::Error| ShardRollError::IndexPublishFailedAfterTruncate {
        artifact_stem: entry.artifact_stem.clone(),
        sealed_path: sealed_path.clone(),
        source,
    };

    let mut index = load_shard_index(index_path)
        .map_err(to_error)?
        .unwrap_or_else(|| ShardIndex {
            schema_version: 1,
            artifact_stem: entry.artifact_stem.clone(),
            current_shard: entry.artifact_path.clone(),
            shard_cap_bytes: entry.shard_cap_bytes,
            max_single_record_bytes: entry.max_single_record_bytes,
            safety_margin_bytes: entry.safety_margin,
            practical_fuel_ceiling: entry.practical_fuel_ceiling,
            worst_case_fuel_per_byte: entry.worst_case_fuel_per_byte,
            // BC-1.18.007 Postcondition 1 (S-25.02 cluster-3, sibling-site
            // sweep accompanying the new `ShardIndex::retention_count`
            // field): a freshly-synthesized index (first-ever seal for this
            // artifact) starts at the config default — never a bespoke
            // per-call value this narrow constructor has no other source
            // for. No behavior change to BC-1.18.006's own roll sequence.
            retention_count: default_retention_count(),
            shards: Vec::new(),
        });

    index.shards.push(new_shard_entry);

    let serialized =
        toml::to_string(&index).map_err(|e| to_error(io::Error::other(e.to_string())))?;

    last_amended_migrate::atomic_write::write_atomic(index_path, &serialized)
        .map_err(|e| to_error(migrate_err_to_io(e)))?;

    Ok(index)
}

/// Roll-orchestration entry point — BC-1.18.006 Postcondition 1's full
/// staged sequence, steps (a)-(d), executed in order (Invariant 2 — never
/// reordered). Called from [`shard_cap_gate_check`]'s `ShardShape::Flat`
/// trigger-fire branch for a PROSPECTIVE (pre-write) roll
/// (`sealed_retroactively = false`), and from Postcondition 7's two catch
/// points ([`reconcile_post_write_replace_all_overcap`],
/// [`reconcile_leading_probe_backstop`]) for a RETROACTIVE roll
/// (`sealed_retroactively = true`) against content already durably on disk
/// (ADR-051 §Decision 15 point 3 — VERBATIM reuse, no new roll logic, no
/// new error code).
///
/// Returns `Ok(Some(entry))` — the newly published [`ShardIndexEntry`] — on
/// a normal seal; a prospective roll's caller builds the `HookResult::Block`
/// retry message from it via [`build_roll_retry_block_reason`]; a
/// retroactive roll's caller emits no `HookResult` at all (Postcondition 7's
/// no-signal contract, ADR-051 §Decision 15 point 2). Returns `Ok(None)`
/// when the canonical content to seal is EMPTY (F-C2-P2-006, ADVISORY,
/// cluster-2 LOCAL adversary pass-2) — see the empty-content check below for
/// why this is a legitimate, non-error outcome, never a fabricated seal of
/// nothing.
pub fn execute_roll(
    entry: &ShardEntry,
    canonical_path: &Path,
    sealed_retroactively: bool,
) -> Result<Option<ShardIndexEntry>, ShardRollError> {
    // FIX-MED-2: refuse loud rather than reading (and later durably
    // sealing) through a symlinked canonical path. Checked immediately
    // before step (a)'s own read, the earliest point this function could
    // apply the guard.
    reject_canonical_symlink(&entry.artifact_stem, canonical_path)?;

    // Step (a).
    let content = read_canonical_content(canonical_path).map_err(|source| {
        ShardRollError::SealWriteFailed {
            artifact_stem: entry.artifact_stem.clone(),
            source,
        }
    })?;

    // F-C2-P2-006 (ADVISORY, cluster-2 LOCAL adversary pass-2): an EMPTY
    // canonical has nothing to preserve — skip sealing entirely (no sealed
    // shard file published, no `[[shard]]` index row appended) rather than
    // accumulate a useless, permanent 0-byte seal. This is reachable
    // legitimately: `Write`'s trigger formula (`projected_size =
    // len(content)` alone) never depends on the CURRENT canonical size, so
    // the trigger can fire purely because of an oversized incoming payload
    // against a canonical that is ALREADY empty — e.g. immediately after a
    // prior roll, or (the exact sequence this finding was raised against)
    // after THIS SAME dispatch's own Postcondition 7 catch point (ii)
    // Write-arm backstop already retroactively sealed a crash-orphaned
    // canonical and truncated it to 0 bytes moments earlier, only for this
    // dispatch's own trigger to then fire a second time against that
    // now-empty canonical. `truncate_canonical_to_empty` would be a pure
    // no-op write against already-empty content in this case, so this
    // returns before steps (b)-(d) ever run — canonical stays exactly 0
    // bytes either way (Invariant 6 holds trivially).
    if content.is_empty() {
        return Ok(None);
    }

    let index_path = shard_index_path_for(canonical_path, &entry.artifact_stem);
    let next_seq =
        next_seal_seq(&index_path).map_err(|source| ShardRollError::SealWriteFailed {
            artifact_stem: entry.artifact_stem.clone(),
            source,
        })?;
    let sealed_filename = format!("{}.{next_seq:04}.md", entry.artifact_stem);
    let sealed_path = shard_sibling_path(canonical_path, &sealed_filename);

    // Step (b).
    publish_sealed_shard(&sealed_path, &content)
        .map_err(|e| reattribute_roll_error(e, &entry.artifact_stem, &sealed_filename))?;

    // Step (c).
    truncate_canonical_to_empty(canonical_path)
        .map_err(|e| reattribute_roll_error(e, &entry.artifact_stem, &sealed_filename))?;

    let new_entry = ShardIndexEntry {
        seq: next_seq,
        path: sealed_filename.clone(),
        sealed_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        bytes_at_seal: content.len() as u64,
        sealed_retroactively,
        // P2-002/P3-001 sibling-sweep (TD-VSDD-060): the BC-1.18.006 ongoing
        // per-write roll mechanism has no mechanism-A backfill-split
        // record-level concept -- `oversized_record`/`is_preamble_shard` are
        // exclusively mechanism-A backfill-split concepts (see
        // `run_mechanism_a_backfill_split`).
        oversized_record: false,
        is_preamble_shard: false,
        records: 0,
    };

    // Step (d).
    publish_shard_index_update(&index_path, entry, new_entry.clone())
        .map_err(|e| reattribute_roll_error(e, &entry.artifact_stem, &sealed_filename))?;

    Ok(Some(new_entry))
}

/// Unified, single-template retry-instruction `Block` message (BC-1.18.006
/// Postcondition 2, Invariant 4) — the SAME fixed wording regardless of the
/// original tool name, naming the artifact, the cap reached, the fact the
/// current shard is now empty, and per-tool retry guidance embedded within
/// the ONE template (never a per-tool-name divergent choice).
///
/// # GREEN-BY-DESIGN (BC-5.38.002)
///
/// A single `format!()` expression interpolating three already-known
/// arguments into a FIXED literal template (Postcondition 2's own wording,
/// verbatim) — zero branching, no I/O, no calls to non-trivial helpers,
/// single-expression body. There is no domain decision left for a test to
/// exercise non-trivially: this implementation satisfies Postcondition 2's
/// required substrings by construction, not by any logic worth withholding
/// from test-writer's Red Gate suite.
pub fn build_roll_retry_block_reason(
    artifact_stem: &str,
    shard_cap_bytes: u64,
    sealed_path: &str,
) -> String {
    format!(
        "Shard `{artifact_stem}` rotated (cap {shard_cap_bytes} bytes reached); the current \
         shard is now empty. Retry your write against the CURRENT (post-roll, empty) file — do \
         not resubmit your original payload unchanged: if you used `Edit` or `MultiEdit`, your \
         `old_string` will no longer match (the content it targeted is now in `{sealed_path}`) — \
         reissue as a fresh `Write` containing ONLY your new entry; if you used `Write`, \
         recompute `content` to contain ONLY your new entry (not your original full pre-roll \
         payload, which reflects discarded state and will exceed the cap again if resubmitted)."
    )
}

/// Retry-instruction `Block` message for the F-C2-P2-006 "empty roll" case
/// — [`execute_roll`] returned `Ok(None)` because the canonical was already
/// empty when the trigger fired, so there is no sealed shard to name (unlike
/// [`build_roll_retry_block_reason`]'s normal case). The current shard is
/// STILL empty (it always was, in this case) — the caller's own payload is
/// what needs to shrink. `payload_len_bytes` (BC-1.18.006 v1.8, F-C2-P4-003,
/// MINOR) is the incoming payload's own projected size (`projected_size` at
/// the trigger-fire call site — `len(content)` for `Write`, or
/// `current_shard_bytes + net_delta_bytes` for `Edit`/`MultiEdit`) — naming
/// it explicitly tells the caller exactly how far over cap their own
/// payload is, rather than leaving them to recompute it themselves.
///
/// # VERBATIM template (BC-1.18.006 v1.8 Postcondition 2, F-C2-P5-002)
///
/// Emits Postcondition 2's "Empty-canonical retry template" byte-for-byte
/// (modulo the three named substitutions) — a prior revision paraphrased
/// this wording (a spurious "reached" after the cap-bytes parenthetical;
/// a second sentence that never mentioned "no roll was performed" or "the
/// shard remains exactly as it was before this call") and a single-substring
/// test (`.contains("(N bytes)")`) stayed green through that divergence.
/// Backticks around `{artifact_stem}` are literal output characters (the
/// same convention [`build_roll_retry_block_reason`]'s own unified template
/// already establishes); backticks around the numeric placeholders in the
/// spec's own blockquote are doc-only markup — bare numbers are emitted.
///
/// # Case B1/B2 split (BC-1.18.006 v1.9 Postcondition 2's "Double-fire
/// exception", Invariant 4, F-C2-P8-004, MINOR; EC-026)
///
/// `preceded_by_backstop_roll` selects between the two sanctioned Case B
/// sub-templates — a pure function of whether Postcondition 7 catch point
/// (ii)'s leading-probe backstop (`reconcile_leading_probe_backstop`)
/// retroactively rolled a pre-existing orphaned over-cap canonical EARLIER
/// in this SAME dispatch, before this call's own trigger re-evaluated and
/// hit this `Ok(None)` short-circuit a second time:
/// - `false` — Case B1, the "pure" empty-canonical case: no roll of any
///   kind occurred this dispatch, so "no roll was performed... the shard
///   remains exactly as it was before this call" is TRUE.
/// - `true` — Case B2, the "double-fire" case: a roll (the backstop's own
///   retroactive roll) DID occur earlier in this dispatch, so Case B1's
///   wording would be FALSE — this variant drops the "no roll was
///   performed"/"remains exactly as it was" clauses while keeping the
///   identical, actionable split-payload guidance.
fn build_empty_roll_retry_block_reason(
    artifact_stem: &str,
    shard_cap_bytes: u64,
    payload_len_bytes: u64,
    preceded_by_backstop_roll: bool,
) -> String {
    if preceded_by_backstop_roll {
        format!(
            "Shard `{artifact_stem}` is now empty (a prior over-cap shard was retroactively \
             rotated by this same call before your payload was evaluated); your own payload \
             alone ({payload_len_bytes} bytes) exceeds the cap ({shard_cap_bytes} bytes). \
             Recompute or split your payload into multiple smaller calls."
        )
    } else {
        format!(
            "Shard `{artifact_stem}` is already empty; your own payload alone ({payload_len_bytes} \
             bytes) exceeds the cap ({shard_cap_bytes} bytes). Recompute or split your payload into \
             multiple smaller calls — no roll was performed, because there is no existing content \
             to rotate away; the shard remains exactly as it was before this call."
        )
    }
}

// ---------------------------------------------------------------------------
// Self-healing recovery (ADR-051 §Decision 11; EC-010/EC-011). Wired into
// `shard_cap_gate_check`'s `ShardShape::Flat` arm (called unconditionally,
// for every matched tool kind, immediately after the config-match/shape
// dispatch and BEFORE this dispatch's own size-trigger formula is
// evaluated — Postcondition 1's "next dispatch attempt... before evaluating
// any new trigger" ordering requirement).
// ---------------------------------------------------------------------------

/// Cheap crash-state PLAUSIBILITY probe gating both self-heal functions
/// below (ADR-051 §Decision 11's "next dispatch attempt" recovery check).
///
/// **CORRECTED (F-C2-P7-002, MINOR, cluster-2 LOCAL adversary pass-7) —
/// this doc block previously claimed a "SINGLE stat()/existence check...
/// no directory listing," which described the pre-F-C2-P6-003 mechanism,
/// not the mechanism actually shipped below.** As of v1.9 (F-C2-P6-003,
/// see the adjacent `//` note below), this probe performs a `read_dir`
/// SCAN of the canonical file's directory, matching every entry's NAME
/// against the `<stem>.<seq>.md` sealed-shard naming convention (seq at
/// least 4 digits, all digits) and comparing each match's `seq` against
/// the shard-index's already-recorded entries — not a single guessed-path
/// `exists()` check. The scan reads directory NAMES and, since F-C2-P7-004,
/// each unindexed candidate's METADATA (`DirEntry::metadata()`, to apply
/// the same 0-byte-orphan guard Invariant 9 requires of the downstream
/// self-heal functions) — it still never reads any file's CONTENT and
/// never touches the canonical file's own content, so it remains
/// materially cheaper than either self-heal function's own work (a
/// byte-for-byte content comparison; an index-reconciliation pass), but it
/// is a directory-wide scan, not a single stat(). In the overwhelmingly
/// common healthy case (no unindexed, non-empty candidate present) this
/// probe is still the ONLY overhead self-heal adds to a normal dispatch.
/// Only when this probe finds a genuine candidate do the two functions
/// below pay for their own, more expensive checks (byte-for-byte content
/// comparison; a directory-wide orphan-reconciliation scan of their own).
// BC-1.18.006 v1.9 (F-C2-P6-003 sibling-site fix, MAJOR, cluster-2 LOCAL
// adversary pass-6 hardening): this probe previously checked ONLY for a
// sealed-shard file at the single "index max seq + 1" guessed path — but
// that guess is correct ONLY for the `E-SHD-006` resume-from-truncate
// signature (a seal published one seq past the index's current tip). Any
// OTHER unindexed orphan — e.g. a seal at a seq the index doesn't yet
// know about because the index file itself is missing or stale, or an
// orphan sitting at a seq other than exactly "next" — is invisible to a
// single guessed-path `exists()` check, so `run_self_heal_if_plausible`
// short-circuited to `Ok(())` and NEVER invoked
// `self_heal_reconcile_missing_index_entries`'s directory-wide scan at
// all, no matter how many genuine orphans were sitting on disk. Widened
// to a directory scan matching the SAME `<stem>.<seq>.md` naming
// predicate `self_heal_reconcile_missing_index_entries` uses (kept
// consistent with that function's own F-C2-P6-003 digit-width widening:
// "at least 4 digits, and ALL digits" — never a fixed 4, since `{seq:04}`
// is MIN-width, not fixed-width) — cheap relative to either self-heal
// path's own work, since it reads only directory entry names (no file
// content), the same cost class `self_heal_reconcile_missing_index_entries`
// already pays when it actually runs.
fn self_heal_recovery_plausible(entry: &ShardEntry, canonical_path: &Path) -> io::Result<bool> {
    let index_path = shard_index_path_for(canonical_path, &entry.artifact_stem);
    let indexed_seqs: std::collections::BTreeSet<u32> = load_shard_index(&index_path)?
        .map(|index| index.shards.iter().map(|s| s.seq).collect())
        .unwrap_or_default();

    let dir = canonical_path.parent().unwrap_or_else(|| Path::new("."));
    let prefix = format!("{}.", entry.artifact_stem);

    // A missing directory has no candidates by construction — the same
    // "absent means implausible, never an error" semantics the old
    // single-path `sealed_path.exists()` check gave for free (`exists()`
    // returns `false`, never errors, when an ancestor is missing).
    // Preserved here explicitly since `read_dir` itself surfaces `NotFound`
    // as an `Err`, unlike `Path::exists()`.
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        // TD-VSDD-060 sibling sweep (PR #824 pr-review Finding #1): same
        // cross-platform disambiguation — see `is_genuinely_missing`'s own
        // doc comment.
        Err(e) if is_genuinely_missing(&e, dir) => return Ok(false),
        Err(e) => return Err(e),
    };

    for item in entries {
        let item = item?;
        let file_name = item.file_name();
        let file_name = file_name.to_string_lossy();

        let Some(rest) = file_name.strip_prefix(&prefix) else {
            continue;
        };
        let Some(seq_str) = rest.strip_suffix(".md") else {
            continue;
        };
        if seq_str.len() < 4 || !seq_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let Ok(seq) = seq_str.parse::<u32>() else {
            continue;
        };
        if indexed_seqs.contains(&seq) {
            continue;
        }
        // F-C2-P7-004 (ADVISORY, cluster-2 LOCAL adversary pass-7): a
        // candidate whose on-disk size is exactly 0 bytes can NEVER be a
        // genuine E-SHD-006/E-SHD-007 crash orphan (Invariant 9,
        // F-C2-P3-002 — `execute_roll` never seals empty content, and both
        // downstream self-heal functions already refuse to index a 0-byte
        // candidate). Skip it here too, so a PERSISTENT external 0-byte
        // orphan does not keep this cheap probe permanently "plausible"
        // and force needless payment for either self-heal function's more
        // expensive checks on every future dispatch for this artifact.
        let is_zero_byte = item.metadata().map(|m| m.len() == 0).unwrap_or(false);
        if is_zero_byte {
            continue;
        }
        return Ok(true);
    }

    Ok(false)
}

/// Runs self-heal recovery for `entry`/`canonical_path` if — and only if —
/// [`self_heal_recovery_plausible`]'s cheap probe finds a candidate. Tries
/// [`self_heal_resume_from_truncate`] (`E-SHD-006`) first; if it fully
/// reconciles the crash state (`Ok(Some(_))`), that resume ALSO already
/// published the missing index entry (its own step (d)), so there is
/// nothing left for [`self_heal_reconcile_missing_index_entries`]'s more
/// expensive directory-wide scan to find — it is skipped in that case,
/// rather than paid for unconditionally. Otherwise (the byte-identity check
/// found no `E-SHD-006` duplicate — e.g. the canonical was already correctly
/// truncated, the `E-SHD-007` signature), falls through to the index
/// reconciliation scan.
fn run_self_heal_if_plausible(
    entry: &ShardEntry,
    canonical_path: &Path,
) -> Result<(), ShardRollError> {
    let plausible = self_heal_recovery_plausible(entry, canonical_path).map_err(|source| {
        ShardRollError::SealWriteFailed {
            artifact_stem: entry.artifact_stem.clone(),
            source,
        }
    })?;
    if !plausible {
        return Ok(());
    }

    if self_heal_resume_from_truncate(entry, canonical_path)?.is_some() {
        return Ok(());
    }

    self_heal_reconcile_missing_index_entries(entry, canonical_path)?;
    Ok(())
}

/// `E-SHD-006` self-heal: detects "seal published, truncate did not" (a
/// sealed shard exists at the index's next-expected `seq` path whose
/// content is byte-identical to the canonical file's CURRENT content) and,
/// if so, resumes from step (c) alone — re-attempting ONLY the truncate +
/// index publish, never re-writing the already-correct sealed shard
/// (idempotent because step (b) is never re-issued during recovery —
/// self-heal resumes from step (c) [`truncate_canonical_to_empty`] onward;
/// the sealed shard, once written by `write_exclusive`, is immutable and
/// never rewritten). BC-1.18.006 Postcondition 1's `E-SHD-006`
/// partial-failure postcondition; EC-010.
///
/// Returns `Ok(None)` when no `E-SHD-006` duplicate-content state is
/// detected (the common case — no action taken); `Ok(Some(entry))` when the
/// self-heal ran and published the missing index entry.
pub fn self_heal_resume_from_truncate(
    entry: &ShardEntry,
    canonical_path: &Path,
) -> Result<Option<ShardIndexEntry>, ShardRollError> {
    // FIX-MED-2 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-59/
    // CWE-200): refuse loud rather than reading (for the byte-identity
    // comparison below) through a symlinked canonical path — see
    // `reject_canonical_symlink`'s own doc comment. Checked before this
    // function's own `std::fs::read(canonical_path)` call, the earliest
    // point it could apply.
    reject_canonical_symlink(&entry.artifact_stem, canonical_path)?;

    let index_path = shard_index_path_for(canonical_path, &entry.artifact_stem);
    let next_seq =
        next_seal_seq(&index_path).map_err(|source| ShardRollError::SealWriteFailed {
            artifact_stem: entry.artifact_stem.clone(),
            source,
        })?;
    let sealed_filename = format!("{}.{next_seq:04}.md", entry.artifact_stem);
    let sealed_path = shard_sibling_path(canonical_path, &sealed_filename);

    // No sealed shard at the next-expected seq at all — nothing to resume
    // from (the common, healthy case). A read error OTHER than `NotFound`
    // (e.g. a permission error, or — the point of this bytes-level read —
    // content that happens not to be valid UTF-8) must NEVER be silently
    // treated as "absent": the plausibility probe that gated this call
    // already confirmed `sealed_path` exists on disk (F-C2-P1-001, MAJOR).
    // `std::fs::read` (not `read_to_string`) so the byte-identity comparison
    // below is immune to UTF-8 validity entirely — the sealed shard's actual
    // bytes are what must match the canonical's actual bytes, never a
    // lossy/failable `String` decoding of either.
    let sealed_bytes = match std::fs::read(&sealed_path) {
        Ok(bytes) => bytes,
        // TD-VSDD-060 sibling sweep (PR #824 pr-review Finding #1): same
        // cross-platform disambiguation — see `is_genuinely_missing`'s own
        // doc comment.
        Err(e) if is_genuinely_missing(&e, &sealed_path) => return Ok(None),
        Err(source) => {
            return Err(ShardRollError::TruncateFailedAfterSeal {
                artifact_stem: entry.artifact_stem.clone(),
                sealed_path: sealed_filename,
                source,
            });
        }
    };

    // BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002, cluster-2 LOCAL adversary
    // pass-3): `execute_roll` can never itself publish a 0-byte seal
    // (F-C2-P2-006's own empty-canonical short-circuit) — so a 0-byte
    // candidate found sitting at the index's next-expected seq path is, by
    // construction, an EXTERNAL anomaly, never a genuine artifact of this
    // dispatcher's own roll sequence. Without this guard, a 0-byte sealed
    // candidate is byte-identical to an ALSO-0-byte canonical (the vacuous
    // 0-bytes == 0-bytes case), which the comparison below would otherwise
    // misread as a genuine E-SHD-006 duplicate-content match and resume
    // from — fabricating a `bytes_at_seal = 0` index row for a seal that
    // never legitimately happened. Skip it: no truncate, no index publish,
    // never fail the dispatch over it, just a diagnostic.
    if sealed_bytes.is_empty() {
        tracing::warn!(
            artifact_stem = %entry.artifact_stem,
            sealed_path = %sealed_path.display(),
            "BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002): self_heal_resume_from_truncate found a \
             0-byte sealed-shard candidate at the index's next-expected seq — execute_roll can \
             never itself produce a 0-byte seal, so this is an external anomaly, not a \
             resumable E-SHD-006 duplicate-content state. Skipping (no truncate, no index \
             entry published)."
        );
        return Ok(None);
    }

    let current_bytes = match std::fs::read(canonical_path) {
        Ok(bytes) => bytes,
        // TD-VSDD-060 sibling sweep (PR #824 pr-review Finding #1): same
        // cross-platform disambiguation — see `is_genuinely_missing`'s own
        // doc comment.
        Err(e) if is_genuinely_missing(&e, canonical_path) => return Ok(None),
        Err(source) => {
            return Err(ShardRollError::TruncateFailedAfterSeal {
                artifact_stem: entry.artifact_stem.clone(),
                sealed_path: sealed_filename,
                source,
            });
        }
    };

    // The sealed shard exists but its content diverges from the canonical
    // file's CURRENT content — not the `E-SHD-006` duplicate-content
    // signature (which requires byte-identity); take no action rather than
    // fabricate a roll.
    if sealed_bytes != current_bytes {
        return Ok(None);
    }

    // Detected "seal published, truncate did not" — resume from step (c)
    // alone. The already-durable sealed shard is never rewritten.
    truncate_canonical_to_empty(canonical_path)
        .map_err(|e| reattribute_roll_error(e, &entry.artifact_stem, &sealed_filename))?;

    let bytes_at_seal = sealed_bytes.len() as u64;
    let new_entry = ShardIndexEntry {
        seq: next_seq,
        path: sealed_filename.clone(),
        sealed_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        bytes_at_seal,
        // BC-1.18.006 v1.5 Invariant 7 (F-C2-P1-001, MAJOR): no in-flight
        // roll context survives to this self-heal path, so
        // `sealed_retroactively` is deterministically INFERRED from
        // `bytes_at_seal` rather than hardcoded — a prospective roll can
        // never seal over-cap content (Postcondition 3's unconditional
        // guarantee), so `bytes_at_seal > shard_cap_bytes` is
        // proof-by-construction that this seal was retroactive.
        sealed_retroactively: bytes_at_seal > entry.shard_cap_bytes,
        // P2-002/P3-001 sibling-sweep (TD-VSDD-060): not the mechanism-A
        // backfill-split path -- see the roll-mechanism note above.
        oversized_record: false,
        is_preamble_shard: false,
        records: 0,
    };

    publish_shard_index_update(&index_path, entry, new_entry.clone())
        .map_err(|e| reattribute_roll_error(e, &entry.artifact_stem, &sealed_filename))?;

    Ok(Some(new_entry))
}

/// `E-SHD-007` self-heal: scans the filesystem for sealed-shard files
/// matching this artifact's `<stem>.<seq:04>.md` naming convention that are
/// absent from the shard-index, and appends the missing entries. BC-1.18.006
/// Postcondition 1's `E-SHD-007` partial-failure postcondition; EC-011.
///
/// Returns the list of newly appended [`ShardIndexEntry`] rows (empty when
/// the index was already fully reconciled — the common case).
pub fn self_heal_reconcile_missing_index_entries(
    entry: &ShardEntry,
    canonical_path: &Path,
) -> Result<Vec<ShardIndexEntry>, ShardRollError> {
    let index_path = shard_index_path_for(canonical_path, &entry.artifact_stem);
    let to_error = |source: io::Error| ShardRollError::IndexPublishFailedAfterTruncate {
        artifact_stem: entry.artifact_stem.clone(),
        sealed_path: String::new(),
        source,
    };

    let indexed_seqs: std::collections::BTreeSet<u32> = load_shard_index(&index_path)
        .map_err(to_error)?
        .map(|index| index.shards.iter().map(|s| s.seq).collect())
        .unwrap_or_default();

    let dir = canonical_path.parent().unwrap_or_else(|| Path::new("."));
    let prefix = format!("{}.", entry.artifact_stem);

    let mut unindexed: Vec<(u32, String, u64)> = Vec::new();
    for item in std::fs::read_dir(dir).map_err(to_error)? {
        let item = item.map_err(to_error)?;
        let file_name = item.file_name();
        let file_name = file_name.to_string_lossy();

        // Match this artifact's `<stem>.<seq:04>.md` sealed-shard naming
        // convention (Postcondition 5) — anything else (the canonical file
        // itself, the shard-index TOML, an unrelated sibling, or a
        // differently-shaped stem) is skipped.
        let Some(rest) = file_name.strip_prefix(&prefix) else {
            continue;
        };
        let Some(seq_str) = rest.strip_suffix(".md") else {
            continue;
        };
        // BC-1.18.006 v1.9 (F-C2-P6-003, ADVISORY, cluster-2 LOCAL adversary
        // pass-6): `{seq:04}` is Rust's MIN-width formatting spec, NOT a
        // FIXED width — `seq >= 10_000` legitimately produces a 5+ digit
        // filename (e.g. `decision-log.10000.md`), not a truncated or
        // malformed one. A strict `seq_str.len() != 4` guard silently
        // skipped every such orphan forever, indistinguishable (to that
        // guard) from a genuinely unrelated file. Widened to "at least 4
        // digits, and ALL digits" — still rejects anything shorter (which
        // can never be a real `{seq:04}` output) and anything non-numeric
        // (a malformed/unrelated candidate), while accepting any legitimate
        // seq width `{seq:04}` can actually produce.
        if seq_str.len() < 4 || !seq_str.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }
        let Ok(seq) = seq_str.parse::<u32>() else {
            continue;
        };
        if indexed_seqs.contains(&seq) {
            continue;
        }

        let bytes = item.metadata().map_err(to_error)?.len();

        // BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002, cluster-2 LOCAL
        // adversary pass-3): `execute_roll` can never itself publish a
        // 0-byte seal (F-C2-P2-006's own empty-canonical short-circuit),
        // so a 0-byte file sitting at this artifact's exact
        // `<stem>.<seq:04>.md` sealed-shard naming convention is, by
        // construction, an EXTERNAL anomaly — never a genuine orphaned
        // seal this dispatcher's own roll sequence produced. Indexing it
        // would fabricate a false audit-trail `[[shard]]` row (a seal
        // event that never legitimately happened). Skip it (never fail
        // the dispatch over it), leaving the file itself untouched on
        // disk — only emit a diagnostic.
        if bytes == 0 {
            tracing::warn!(
                artifact_stem = %entry.artifact_stem,
                path = %file_name,
                seq,
                "BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002): self_heal_reconcile_missing_index_entries \
                 found a 0-byte candidate at the sealed-shard naming convention — execute_roll can \
                 never itself produce a 0-byte seal, so this is an external anomaly. Skipping (no \
                 [[shard]] index row appended)."
            );
            continue;
        }

        unindexed.push((seq, file_name.into_owned(), bytes));
    }
    unindexed.sort_by_key(|(seq, ..)| *seq);

    let mut appended = Vec::with_capacity(unindexed.len());
    for (seq, path, bytes_at_seal) in unindexed {
        let new_entry = ShardIndexEntry {
            seq,
            path: path.clone(),
            sealed_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            bytes_at_seal,
            // BC-1.18.006 v1.5 Invariant 7 (F-C2-P1-001, MAJOR): this
            // reconciliation scan has no surviving record of which roll
            // (prospective or retroactive) produced an orphaned sealed
            // shard, so `sealed_retroactively` MUST be deterministically
            // INFERRED from `bytes_at_seal` rather than hardcoded — a
            // prospective roll can never seal over-cap content
            // (Postcondition 3's unconditional guarantee), so
            // `bytes_at_seal > shard_cap_bytes` is proof-by-construction of
            // retroactivity (EC-018).
            sealed_retroactively: bytes_at_seal > entry.shard_cap_bytes,
            // P2-002/P3-001 sibling-sweep (TD-VSDD-060): not the mechanism-A
            // backfill-split path -- see the roll-mechanism note above.
            oversized_record: false,
            is_preamble_shard: false,
            records: 0,
        };
        publish_shard_index_update(&index_path, entry, new_entry.clone())
            .map_err(|e| reattribute_roll_error(e, &entry.artifact_stem, &path))?;
        appended.push(new_entry);
    }

    Ok(appended)
}

// ---------------------------------------------------------------------------
// Postcondition 7 — bounded post-write reconciliation for under-projected
// `replace_all: true` writes (BC-1.18.006 v1.4; story AC-024/AC-025;
// ADR-051 §Decision 15)
// ---------------------------------------------------------------------------

/// Postcondition 7 catch point (i) — immediate post-write reconciliation
/// (story AC-024; BC-1.18.006 EC-014; Invariant 6). Performs a fresh
/// `stat()` of `canonical_path` and, if `actual_size >
/// entry.shard_cap_bytes`, executes [`execute_roll`]'s EXACT four-step
/// sequence RETROACTIVELY (`sealed_retroactively = true`) against content
/// ALREADY on disk. Emits NO `HookResult` of its own (ADR-051 §Decision 15
/// point 2 — a silent filesystem side effect, not a Block/Continue/Error
/// decision): the dispatch that triggered this call has already returned
/// `Continue` to the agent before this leg runs.
///
/// Called from [`crate::invoke::reconcile_replace_all_overcap_if_qualifying`]
/// ONLY after that function's own cheap, real, structural qualification
/// filter (event/tool/`replace_all`/config-match — Postcondition 7's own
/// "zero added cost outside the narrow case" requirement) has already
/// confirmed this dispatch is a candidate. This function itself owns ALL of
/// the BC's tested `stat()`-and-retroactive-roll behavior.
///
/// Returns `Ok(None)` when `actual_size <= entry.shard_cap_bytes` (no
/// action — the single-occurrence trigger estimate was conservative or
/// exactly correct) OR when [`execute_roll`] itself finds nothing to seal
/// (F-C2-P2-006 — practically unreachable here, since `actual_size` was
/// just confirmed `> 0`, but the `Option` is threaded through rather than
/// assumed away); `Ok(Some(entry))` when the retroactive roll ran and
/// published a new sealed shard.
pub fn reconcile_post_write_replace_all_overcap(
    entry: &ShardEntry,
    canonical_path: &Path,
) -> Result<Option<ShardIndexEntry>, ShardRollError> {
    // FIX-MED-2 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-59/
    // CWE-200): refuse loud rather than statting (and, via the retroactive
    // `execute_roll` this function may go on to call, reading) through a
    // symlinked canonical path — see `reject_canonical_symlink`'s own doc
    // comment. Checked before this function's own `std::fs::metadata`
    // call, the earliest point it could apply.
    reject_canonical_symlink(&entry.artifact_stem, canonical_path)?;

    let actual_size = std::fs::metadata(canonical_path)
        .map_err(|source| ShardRollError::SealWriteFailed {
            artifact_stem: entry.artifact_stem.clone(),
            source,
        })?
        .len();

    if actual_size <= entry.shard_cap_bytes {
        return Ok(None);
    }

    // BC-1.18.006 v1.8 corrected Invariant 10 (F-C2-P4-001, MAJOR,
    // data-loss, cluster-2 LOCAL adversary pass-4): self-heal a
    // pre-existing E-SHD-006/E-SHD-007 orphan BEFORE calling `execute_roll`
    // for THIS dispatch's own over-cap content. Unlike `shard_cap_gate_check`
    // (whose `ShardShape::Flat` arm already runs `run_self_heal_if_plausible`
    // unconditionally, before ANY trigger evaluation, for every PreToolUse
    // dispatch — which is also why catch point (ii)
    // [`reconcile_leading_probe_backstop`] needs no separate call of its
    // own here), catch point (i) is invoked directly from `invoke.rs`'s
    // PostToolUse qualifying wrapper and never passes through that
    // Pre-ToolUse wiring at all — so without this call, a pre-existing
    // unindexed orphan's own seq would never get reconciled before
    // `execute_roll`'s `next_seal_seq` computation runs. Left unfixed, an
    // un-indexed orphan sealed shard sitting at seq N (index unaware of it)
    // would make `next_seal_seq` ALSO compute N (the index's own max+1,
    // which is still N since the orphan was never recorded), and
    // `execute_roll`'s `publish_sealed_shard` would then collide with —
    // and, prior to F-C2-P4-002's write-once guard, silently overwrite —
    // that durable orphan with THIS dispatch's own new content, permanently
    // destroying the orphan's history. Running self-heal first indexes the
    // orphan at its own seq N, so `next_seal_seq` correctly advances to
    // N+1 for this dispatch's own retroactive seal.
    run_self_heal_if_plausible(entry, canonical_path)?;

    execute_roll(entry, canonical_path, true)
}

/// Postcondition 7 catch point (ii) — next-dispatch leading-probe backstop
/// (story AC-025; BC-1.18.006 EC-015). Covers a dispatcher-process crash
/// between a `replace_all: true` write's completed application and catch
/// point (i)'s own execution. Executes the SAME retroactive four-step roll
/// [`reconcile_post_write_replace_all_overcap`] specifies.
///
/// Called from [`shard_cap_gate_check`]'s `ShardShape::Flat` `Edit`/
/// `MultiEdit` arms ONLY after the caller has ALREADY confirmed
/// `current_bytes > entry.shard_cap_bytes` by reusing that arm's own
/// pre-existing `current_shard_bytes_flat` stat() read (no new `stat()`
/// call is added for this leg) — so no redundant comparison belongs inside
/// this function itself.
///
/// Returns `Ok(Some(entry))` when the backstop reconciled the pre-existing
/// over-cap state left by a missed catch point (i). `Ok(None)` is
/// practically unreachable here (the caller already confirmed
/// `current_bytes > entry.shard_cap_bytes`, so content can never be empty),
/// but [`execute_roll`]'s `Option` (F-C2-P2-006) is threaded through rather
/// than assumed away.
pub fn reconcile_leading_probe_backstop(
    entry: &ShardEntry,
    canonical_path: &Path,
) -> Result<Option<ShardIndexEntry>, ShardRollError> {
    execute_roll(entry, canonical_path, true)
}

// ===========================================================================
// BC-1.18.007 — Shard Retention/Compaction (S-25.02 F4 BC-cluster 3
// "retention+backfill"; AC-010/AC-011/AC-012).
//
// # BC-5.38.001 Red Gate discipline — IMPLEMENTED (S-25.02 F4 cluster-3)
//
// Every function below is now a REAL, fully implemented body — the
// stub-architect's original `todo!()` placeholders have all been replaced by
// implementer per BC-1.18.007's postconditions, driving test-writer's Red
// Gate suite (`bc_1_18_007_retention_test.rs`'s integration tests) to green.
// GREEN-BY-DESIGN/WIRING-EXEMPT exceptions (`default_retention_count`,
// `archived_shard_path`, `archived_shard_index_path_string`,
// `shard_index_entry_is_archived`, `From<ShardRetentionError> for
// HookResult`) were real from the stub-architect's own initial burst, per
// their own doc comments, mirroring cluster-1/2's own precedent
// (`ShardEntry::cap_formula_inputs`, `From<ShardConfigError> for
// HookResult`, `build_roll_retry_block_reason`).
// ===========================================================================

/// BC-1.18.007 Postcondition 1: the config default for
/// [`ShardIndex::retention_count`] — 10 most-recent shards, a round,
/// human-adjustable number per ADR-051 Decision 6. This BC's implementation
/// MUST read `retention_count` from the shard-index; this function exists
/// ONLY to seed a freshly-synthesized index (first-ever seal) and as the
/// `#[serde(default = ...)]` value for a pre-BC-1.18.007 index loading
/// without the field — never called from the ongoing per-write retention
/// CHECK itself, which always reads the already-loaded `index.retention_count`.
///
/// # GREEN-BY-DESIGN (BC-5.38.002)
///
/// A single literal return — zero branching, no I/O, no calls to
/// non-trivial helpers, one-line body. There is no domain decision here for
/// a test to exercise non-trivially.
pub fn default_retention_count() -> u32 {
    10
}

/// BC-1.18.007 Postcondition 2: the OS-native filesystem destination for an
/// archival move — `.factory/cycles/<cycle>/archive/<artifact-stem>/<sealed-
/// filename>`, expressed relative to `cycle_root` (the sharded artifact's
/// own cycle directory, i.e. `canonical_path`'s parent).
///
/// # GREEN-BY-DESIGN (BC-5.38.002)
///
/// Pure path-segment joining — zero branching, no I/O (no filesystem access;
/// this function only computes a path value), no calls to non-trivial
/// helpers, single-expression body. There is no domain decision left for a
/// test to exercise non-trivially: this is the literal, spec-quoted
/// directory-naming convention (Postcondition 2), not a choice.
pub fn archived_shard_path(
    cycle_root: &Path,
    artifact_stem: &str,
    sealed_filename: &str,
) -> PathBuf {
    cycle_root
        .join("archive")
        .join(artifact_stem)
        .join(sealed_filename)
}

/// Portable (always `/`-separated, OS-independent) form of the SAME
/// archived location [`archived_shard_path`] names as an OS-native
/// [`PathBuf`] — this is the string this module records into
/// [`ShardIndexEntry::path`] after archival (BC-1.18.007 Invariant 3's
/// path-mutation implementation choice; see this module's own cluster-3
/// scope note). Deliberately NOT reused for the actual filesystem move
/// (which needs [`archived_shard_path`]'s OS-native form) — TOML-stored
/// index content is portable text, not an OS path.
///
/// # GREEN-BY-DESIGN (BC-5.38.002)
///
/// A single `format!()` expression over a fixed, spec-quoted literal
/// template — zero branching, no I/O, no calls to non-trivial helpers,
/// one-line body.
pub fn archived_shard_index_path_string(artifact_stem: &str, sealed_filename: &str) -> String {
    format!("archive/{artifact_stem}/{sealed_filename}")
}

/// `true` iff `entry` has already been archived — i.e. its own
/// [`ShardIndexEntry::path`] has been rewritten (by
/// [`archive_overflow_shards`]) to an [`archived_shard_index_path_string`]
/// form, rather than still naming a bare sealed-shard filename sibling to
/// the canonical file (BC-1.18.007 Invariant 3).
///
/// # GREEN-BY-DESIGN (BC-5.38.002)
///
/// A single `str::starts_with` predicate against a fixed literal prefix —
/// zero branching (no `if`/`match`/`?`/`unwrap`), no I/O, no calls to
/// non-trivial helpers, one-line body.
pub fn shard_index_entry_is_archived(entry: &ShardIndexEntry) -> bool {
    entry.path.starts_with("archive/")
}

/// BC-1.18.007 EC-005 (E-SHD-002): the shard-index was missing or corrupt at
/// the moment a retention check would run, or an archival move itself
/// failed mid-invocation — either fails the SAME native-gate invocation
/// loud, never silently. Reuses the E-SHD-002 code error-taxonomy.md
/// already allocates for "Shard management errors ... shard-index missing
/// or corrupt" (no new `E-SHD-NNN` code is expected from this story's
/// implementation, per this story's own File Structure Requirements row).
#[derive(Debug, Error)]
pub enum ShardRetentionError {
    /// EC-005: the retention check's own load of
    /// `<artifact-stem>.shard-index.toml` failed (missing in an unexpected
    /// way, or malformed TOML) at the moment a retention/compaction check
    /// would run — retention/compaction MUST NOT silently skip its check
    /// and proceed as if no archival were needed.
    #[error(
        "E-SHD-002: shard-index missing or corrupt for artifact_stem \"{artifact_stem}\": {source}"
    )]
    IndexUnavailable {
        artifact_stem: String,
        #[source]
        source: io::Error,
    },

    /// Postcondition 2: the archival move of an already-identified
    /// overflow shard from the cycle root to `archive/<artifact-stem>/`
    /// failed. No shard-index change is applied for a failed move (the
    /// caller's own atomic-write of the updated index only proceeds once
    /// every archival move this invocation intends to perform has durably
    /// succeeded — same-invocation-atomicity composition with BC-1.18.006
    /// Postcondition 4).
    #[error(
        "E-SHD-002: shard archival move failed for artifact_stem \"{artifact_stem}\" (shard \
         \"{sealed_path}\") — retention/compaction aborted, no shard-index change applied: \
         {source}"
    )]
    ArchivalMoveFailed {
        artifact_stem: String,
        sealed_path: String,
        #[source]
        source: io::Error,
    },
}

/// Fail-loud retention/compaction errors surface to the dispatcher's
/// handling path as `HookResult::Error` (EC-005's posture).
///
/// # WIRING-EXEMPT (BC-5.38.003)
///
/// `From<T>` blanket delegation to a single `Display`-forwarding call —
/// identical in shape to this file's existing, already-shipped
/// `From<ShardConfigError> for HookResult` / `From<ShardRollError> for
/// HookResult` impls. No domain decision: `ShardRetentionError`'s own
/// `Display` impl (via `thiserror`) already carries the full,
/// artifact-stem-scoped diagnostic text.
impl From<ShardRetentionError> for HookResult {
    fn from(err: ShardRetentionError) -> Self {
        HookResult::Error {
            message: err.to_string(),
        }
    }
}

/// BC-1.18.007 EC-005: load `<artifact-stem>.shard-index.toml` for the
/// retention-check path specifically, failing loud with
/// [`ShardRetentionError::IndexUnavailable`] when the index is missing (in
/// a way that is NOT the legitimate "no roll has ever occurred yet"
/// first-artifact case — see [`load_shard_index`]'s own `Ok(None)`
/// contract, which this function must NOT silently treat as "no archival
/// needed" once a real seal is known to have just occurred) or corrupt.
pub fn load_shard_index_for_retention_check(
    canonical_path: &Path,
    artifact_stem: &str,
) -> Result<ShardIndex, ShardRetentionError> {
    let index_path = shard_index_path_for(canonical_path, artifact_stem);
    match load_shard_index(&index_path) {
        Ok(Some(index)) => Ok(index),
        // EC-005: a seal is known to have just occurred (Precondition 1), so
        // an absent index HERE is an anomaly -- never the legitimate
        // "no roll has ever occurred yet" case `load_shard_index`'s own
        // `Ok(None)` contract otherwise carves out for a fresh artifact.
        Ok(None) => Err(ShardRetentionError::IndexUnavailable {
            artifact_stem: artifact_stem.to_string(),
            source: io::Error::new(
                io::ErrorKind::NotFound,
                format!(
                    "shard-index '{}' not found at retention-check time",
                    index_path.display()
                ),
            ),
        }),
        Err(source) => Err(ShardRetentionError::IndexUnavailable {
            artifact_stem: artifact_stem.to_string(),
            source,
        }),
    }
}

/// BC-1.18.007 Postcondition 1/EC-001/EC-002 (AC-010): how many of
/// `index`'s ACTIVE (non-archived, per [`shard_index_entry_is_archived`])
/// shard entries exceed `index.retention_count` right now — the count of
/// the OLDEST active shards [`archive_overflow_shards`] must relocate in
/// this SAME invocation to bring the active count back within the
/// (possibly newly-lowered, EC-002) limit. `0` when the active count is
/// already `<= retention_count` (no archival needed).
pub fn retention_overflow_count(index: &ShardIndex) -> usize {
    let active_count = index
        .shards
        .iter()
        .filter(|entry| !shard_index_entry_is_archived(entry))
        .count();
    active_count.saturating_sub(index.retention_count as usize)
}

/// BC-1.18.007 Postcondition 2/Invariant 1/Invariant 2 (AC-010): archive the
/// oldest `retention_overflow_count(index)` ACTIVE shard(s) for the artifact
/// `index` describes, moving each (never deleting — Invariant 1) from its
/// current sibling location next to `canonical_path` to
/// [`archived_shard_path`], and rewriting the moved entry's own
/// [`ShardIndexEntry::path`] to [`archived_shard_index_path_string`]'s form
/// (Invariant 3's path-mutation choice) IN PLACE within `index` — the SAME
/// `index` value the caller subsequently persists (composing with
/// BC-1.18.006 Postcondition 4's same-invocation atomicity guarantee).
/// `index.retention_count` is this artifact's own, independent value
/// (Invariant 2 — never a global constant). Returns the archived entries,
/// oldest-first.
pub fn archive_overflow_shards(
    index: &mut ShardIndex,
    canonical_path: &Path,
) -> Result<Vec<ShardIndexEntry>, ShardRetentionError> {
    let overflow = retention_overflow_count(index);
    if overflow == 0 {
        return Ok(Vec::new());
    }

    let cycle_root = canonical_path.parent().unwrap_or_else(|| Path::new(""));

    // Oldest-first (lowest seq first) among the ACTIVE (non-archived)
    // entries only -- an already-archived sibling never counts toward, nor
    // is re-selected by, this pass (Invariant 2/3).
    let mut active_seqs: Vec<u32> = index
        .shards
        .iter()
        .filter(|entry| !shard_index_entry_is_archived(entry))
        .map(|entry| entry.seq)
        .collect();
    active_seqs.sort_unstable();

    let mut archived = Vec::with_capacity(overflow);
    for seq in active_seqs.into_iter().take(overflow) {
        let idx = index
            .shards
            .iter()
            .position(|entry| entry.seq == seq)
            .expect("seq collected from index.shards must still be present in index.shards");
        let sealed_filename = index.shards[idx].path.clone();
        let old_path = shard_sibling_path(canonical_path, &sealed_filename);
        let new_path = archived_shard_path(cycle_root, &index.artifact_stem, &sealed_filename);

        let to_error = |source: io::Error| ShardRetentionError::ArchivalMoveFailed {
            artifact_stem: index.artifact_stem.clone(),
            sealed_path: sealed_filename.clone(),
            source,
        };

        if let Some(parent) = new_path.parent() {
            std::fs::create_dir_all(parent).map_err(to_error)?;
        }

        // Invariant 1: move, never delete -- `rename` relocates the file's
        // content byte-for-byte; nothing is read into memory and rewritten.
        std::fs::rename(&old_path, &new_path).map_err(to_error)?;

        // Invariant 3: the moved entry's own index record is rewritten IN
        // PLACE (never removed) to reflect the new archived location.
        index.shards[idx].path =
            archived_shard_index_path_string(&index.artifact_stem, &sealed_filename);
        archived.push(index.shards[idx].clone());
    }

    Ok(archived)
}

/// Whole-corpus shard-glob scope mode (BC-1.18.007 Postcondition 3/6;
/// AC-011/AC-012). `DefaultExcluded` is the general default (honest
/// `O(active shards)` accounting, Postcondition 3/4) — every existing
/// generic whole-corpus validator. `ArchiveInclusive` is POLICY-1's
/// (`append_only_numbering`) MANDATORY carve-out (Postcondition 6, EC-006)
/// — never opt-in for that one audit class (AC-012).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WholeCorpusGlobScope {
    DefaultExcluded,
    ArchiveInclusive,
}

/// BC-1.18.007 Postcondition 3/4/6 (AC-011/AC-012; VP-122/VP-141):
/// enumerate the shard file paths a whole-corpus reader should scan for
/// `artifact_stem` under `cycle_root`, honoring `scope`'s archive-inclusion
/// policy. `DefaultExcluded` returns only the current file plus active
/// (non-archived) sealed shards at the cycle root — honestly
/// `O(active shards)`, bounded by `retention_count`, never
/// `O(all shards ever)` (AC-011). `ArchiveInclusive` additionally globs
/// `archive/<artifact_stem>/<artifact_stem>*.md` — POLICY-1's mandatory
/// carve-out, so an ID whose sole occurrence has aged into the archive
/// remains visible to append-only/gap/uniqueness detection (AC-012, EC-006).
pub fn whole_corpus_shard_paths(
    cycle_root: &Path,
    artifact_stem: &str,
    scope: WholeCorpusGlobScope,
) -> io::Result<Vec<PathBuf>> {
    let mut paths = collect_shard_files_in_dir(cycle_root, artifact_stem)?;

    if scope == WholeCorpusGlobScope::ArchiveInclusive {
        let archive_dir = cycle_root.join("archive").join(artifact_stem);
        match collect_shard_files_in_dir(&archive_dir, artifact_stem) {
            Ok(mut archived) => paths.append(&mut archived),
            // AC-011: an artifact that has never exceeded retention_count
            // has no archive/ directory at all -- not an error, zero
            // archived shards to include.
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e),
        }
    }

    paths.sort();
    Ok(paths)
}

/// Enumerate every regular file directly under `dir` whose filename matches
/// `artifact_stem`'s own shard-naming convention (the bare current filename
/// `<stem>.md`, or a sealed-shard filename `<stem>.<digits>.md`) -- used by
/// [`whole_corpus_shard_paths`] against both the cycle root (active shards)
/// and, under [`WholeCorpusGlobScope::ArchiveInclusive`], the
/// `archive/<stem>/` subdirectory.
fn collect_shard_files_in_dir(dir: &Path, artifact_stem: &str) -> io::Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let file_name = entry.file_name();
        if is_shard_file_for_stem(&file_name.to_string_lossy(), artifact_stem) {
            out.push(entry.path());
        }
    }
    Ok(out)
}

/// `true` iff `file_name` is either `artifact_stem`'s bare current filename
/// (`<stem>.md`) or a sealed-shard filename (`<stem>.<digits>.md`) --
/// deliberately narrow (digits-only middle segment) so an unrelated
/// same-stem-prefixed file (e.g. `<stem>-old.md`, `<stem>.shard-index.toml`)
/// is never mistaken for a shard of this artifact.
fn is_shard_file_for_stem(file_name: &str, artifact_stem: &str) -> bool {
    if file_name == format!("{artifact_stem}.md") {
        return true;
    }
    let prefix = format!("{artifact_stem}.");
    match file_name
        .strip_prefix(prefix.as_str())
        .and_then(|rest| rest.strip_suffix(".md"))
    {
        Some(seq_part) => !seq_part.is_empty() && seq_part.bytes().all(|b| b.is_ascii_digit()),
        None => false,
    }
}

// ===========================================================================
// BC-1.18.008 — Mandatory One-Time Backfill-Split of the Four Pre-Existing
// Oversized Cycle Append-Logs (S-25.02 F4 BC-cluster 3 "retention+backfill";
// AC-013/AC-014). Named `mechanism_a_*`/`MechanismA*` throughout to avoid
// any future naming collision with BC-1.18.011's (B2) and BC-1.18.012's
// (B1) own, structurally distinct one-time migrations, which later clusters
// (6/7) will add to this same module.
//
// # BC-5.38.001 Red Gate discipline — IMPLEMENTED (S-25.02 F4 cluster-3)
//
// Every function below is now a REAL, fully implemented body — the
// stub-architect's original `todo!()` placeholders have all been replaced by
// implementer per BC-1.18.008's postconditions, driving test-writer's Red
// Gate suite (`bc_1_18_008_backfill_split_test.rs`'s integration and unit
// tests) to green, including a fresh-context adversarial-review pattern-based
// rewrite of `mechanism_a_record_boundary_offsets` against BC-1.18.008 v1.2's
// amended Record-Boundary Marker Table.
// ===========================================================================

/// BC-1.18.008 EC-004 (E-SHD-003): the mechanism-A backfill-split's mandatory
/// content-preservation/record-integrity verification gate (Postcondition
/// 6) failed, or a genuine I/O failure occurred while staging the split.
/// Either aborts the WHOLE operation; the original monolithic file is left
/// completely untouched (fail-loud, never partial-and-silent). Reuses the
/// E-SHD-003 code error-taxonomy.md already allocates for "backfill-split
/// content-preservation verification failed" (no new `E-SHD-NNN` code
/// expected from this story's implementation).
#[derive(Debug, Error)]
pub enum MechanismABackfillError {
    /// Postcondition 6(a)/6(b): the staged partitions, concatenated in
    /// order, do not reproduce the original monolithic file byte-for-byte,
    /// or a structural record was found duplicated or dropped across the
    /// staged partitions. `detail` names which of the two checks failed and
    /// how (e.g. a byte offset, or a record identifier).
    #[error(
        "E-SHD-003: backfill-split content-preservation verification failed for artifact_stem \
         \"{artifact_stem}\": {detail}"
    )]
    ContentPreservationFailed {
        artifact_stem: String,
        detail: String,
    },

    /// A genuine I/O failure while staging the split's shard files and
    /// index (Postcondition 5's stage-then-verify-then-atomically-replace
    /// sequence) — the original monolithic file is left untouched; the
    /// operation is safely re-runnable from scratch (Postcondition 5, EC-003).
    #[error(
        "E-SHD-003: backfill-split I/O failure for artifact_stem \"{artifact_stem}\": {source}"
    )]
    Io {
        artifact_stem: String,
        #[source]
        source: io::Error,
    },

    /// F-C3-P6-001 (S-25.02 F4 cluster-3 CROSS-VENDOR (OpenAI Codex)
    /// adversarial pass-6 review, HIGH; BC-1.18.008 v1.6 Postcondition 5's
    /// Recovery-Confirmation Rule, Invariant 3, EC-010; `E-SHD-011`,
    /// `prd-supplements/error-taxonomy.md` v1.10): at recovery-confirmation
    /// time, the canonical file's exact whole-file `(length, SHA-256)`
    /// matches NEITHER the Backfill Recovery Manifest's recorded
    /// `original_bytes`/`original_sha256` pair NOR its
    /// `final_bytes`/`final_sha256` pair — the on-disk state is AMBIGUOUS
    /// (modified by something other than this migration's own two-phase
    /// publish sequence, or corrupted). Fails loud; the canonical file is
    /// NOT written to under any circumstance; recovery halts for operator
    /// investigation. Message format matches the error-taxonomy row
    /// verbatim.
    #[error(
        "E-SHD-011: backfill recovery-confirmation ambiguous for artifact_stem \
         \"{artifact_stem}\" — canonical file bytes ({canonical_bytes} bytes, sha256 \
         {canonical_sha256}) match NEITHER the pre-split original ({original_bytes} bytes, \
         sha256 {original_sha256}) NOR the intended final partition ({final_bytes} bytes, sha256 \
         {final_sha256}) recorded in the Backfill Recovery Manifest — refusing to guess; \
         canonical file left untouched pending operator investigation"
    )]
    AmbiguousRecoveryState {
        artifact_stem: String,
        canonical_bytes: u64,
        canonical_sha256: String,
        original_bytes: u64,
        original_sha256: String,
        final_bytes: u64,
        final_sha256: String,
    },

    /// F-C3-P6-001 companion: the shard-index already exists (so
    /// [`mechanism_a_backfill_already_migrated`] reported this artifact as
    /// migrated) and has at least one sealed shard, but carries NO Backfill
    /// Recovery Manifest at all (Postcondition 3) — Postcondition 5's
    /// Recovery-Confirmation Rule is the SOLE authoritative basis for the
    /// SAFE/DANGEROUS/AMBIGUOUS determination and literally cannot be
    /// applied without it. Rather than falling back to the byte-prefix
    /// heuristic this same finding retires (which is exactly the defect
    /// class F-C3-P6-001 corrects), this also fails loud under the `E-SHD-011`
    /// code — a missing manifest is just as unable to support a safe
    /// disposition as a manifest whose values match neither the canonical
    /// file's original nor final state.
    #[error(
        "E-SHD-011: backfill recovery-confirmation ambiguous for artifact_stem \
         \"{artifact_stem}\" — the published shard-index has no Backfill Recovery Manifest \
         ([backfill_manifest]) to compare the canonical file's current bytes against — refusing \
         to guess; canonical file left untouched pending operator investigation"
    )]
    MissingBackfillManifest { artifact_stem: String },

    /// F-C3-P7-001 (S-25.02 F4 cluster-3 LOCAL adversarial pass-7 review,
    /// HIGH; BC-1.18.008 v1.7 Postcondition 5's Manifest-Authoritative
    /// Slice-and-Verify Rule, EC-011, Invariant 3; Postcondition
    /// 6(c)/Invariant 4's heal-write disk-read-back extension; `E-SHD-012`,
    /// `prd-supplements/error-taxonomy.md` v1.11): at the confirmed
    /// DANGEROUS window (the top-level `(length, hash)` check already
    /// matched the Manifest's `original_bytes`/`original_sha256` pair), the
    /// Manifest-derived candidate slice fails ONE LEVEL DEEPER — either (a)
    /// the mandatory pre-write verification (`sliced.len() == final_bytes
    /// AND sha256(sliced) == final_sha256`) fails before anything is
    /// written, or (b) a FRESH post-hoc disk read-back of the heal's own
    /// just-completed write does not match `(final_bytes, final_sha256)`,
    /// surfacing a write that silently truncated, partially flushed, or
    /// otherwise landed corrupted bytes on disk. `detail` names which of
    /// the two hard gates failed. Distinct from `E-SHD-011`
    /// (`AmbiguousRecoveryState`): that code fires when the TOP-LEVEL check
    /// cannot confirm DANGEROUS at all; this code fires only AFTER
    /// DANGEROUS is already unambiguously confirmed, signaling that the
    /// Manifest's own `final_bytes`/`final_sha256` fields (or the code
    /// deriving/writing the slice) are themselves in an inconsistent state.
    /// On disposition (a) the canonical file is left untouched; on
    /// disposition (b) the destructive write already happened, so this
    /// surfaces the corruption immediately for operator remediation from
    /// git history/backup rather than reporting the heal complete.
    #[error(
        "E-SHD-012: backfill recovery heal slice-verification failed for artifact_stem \
         \"{artifact_stem}\": {detail}"
    )]
    SliceVerificationFailed {
        artifact_stem: String,
        detail: String,
    },

    /// F-C3-P8-002 (S-25.02 F4 cluster-3 LOCAL adversarial pass-8 review,
    /// MEDIUM; BC-1.18.008 v1.8 Postcondition 6(c)'s new
    /// happy-path-canonical-write extension, Invariant 5, EC-013;
    /// `E-SHD-013`, `prd-supplements/error-taxonomy.md` v1.12): after
    /// Postcondition 5 step (ii)'s ORDINARY, non-recovery, first-time/
    /// uninterrupted completion of the canonical-truncate write
    /// (`write_atomic_bytes(canonical_path, &current_partition.bytes, ..)`),
    /// a FRESH post-hoc disk read-back of the canonical file does NOT match
    /// the Backfill Recovery Manifest's own `(final_bytes, final_sha256)`
    /// pair — already durably published in Postcondition 5 step (i), before
    /// step (ii) ever runs, so no new oracle value is computed here; the
    /// SAME pair the heal write already verifies against. Distinct from
    /// `E-SHD-012` (`SliceVerificationFailed`): that code's own read-back
    /// gate covers the DANGEROUS-window HEAL write (a crash-recovery
    /// re-invocation of an INTERRUPTED prior migration); this code covers
    /// the FIRST, uninterrupted happy-path write that the heal exists to
    /// recover FROM. The destructive write has already happened at this
    /// point — unlike Postcondition 6(a)/(b)'s pre-write
    /// content-preservation checks, which gate BEFORE the original file is
    /// ever touched — so this surfaces the corruption immediately for
    /// operator remediation from git history/backup rather than reporting
    /// the migration complete over silently-corrupted content. Per EC-013's
    /// own Canonical Test Vector note, the shard-index and Backfill Recovery
    /// Manifest are already durably published (Postcondition 5 step (i))
    /// strictly BEFORE this failing write, so a follow-on re-invocation
    /// resolves the now-corrupted canonical file via the ordinary
    /// recovery-confirmation path (AMBIGUOUS, `E-SHD-011`, per Invariant 3)
    /// rather than getting stuck or silently re-migrating.
    #[error(
        "E-SHD-013: post-hoc canonical-file verification failed after happy-path \
         final-partition write for artifact_stem \"{artifact_stem}\" — fresh disk read-back \
         ({read_back_bytes} bytes, sha256 {read_back_sha256}) does NOT match the Backfill \
         Recovery Manifest's recorded final partition ({final_bytes} bytes, sha256 \
         {final_sha256}) — the destructive write already completed; original monolithic \
         content is no longer recoverable from the canonical file itself; halting for \
         operator investigation from git history/backup"
    )]
    CanonicalWriteVerificationFailed {
        artifact_stem: String,
        read_back_bytes: u64,
        read_back_sha256: String,
        final_bytes: u64,
        final_sha256: String,
    },
}

/// Fail-loud backfill-split errors surface to the operator/dispatcher
/// handling path as `HookResult::Error`.
///
/// # WIRING-EXEMPT (BC-5.38.003)
///
/// `From<T>` blanket delegation to a single `Display`-forwarding call —
/// identical in shape to this file's existing, already-shipped
/// `From<ShardConfigError> for HookResult` / `From<ShardRollError> for
/// HookResult` / `From<ShardRetentionError> for HookResult` impls. No
/// domain decision: `MechanismABackfillError`'s own `Display` impl (via
/// `thiserror`) already carries the full, artifact-stem-scoped diagnostic
/// text.
impl From<MechanismABackfillError> for HookResult {
    fn from(err: MechanismABackfillError) -> Self {
        HookResult::Error {
            message: err.to_string(),
        }
    }
}

/// One structural partition of a mechanism-A backfill-split (BC-1.18.008
/// Postcondition 2). `record_count` is this partition's own count of
/// whole, native-format records (Postcondition 6(b)'s record-integrity
/// check operates over these counts, summed across all partitions, against
/// the original file's own total). `oversized_record` is `true` only for
/// the rare EC-002 case — a single record alone exceeds `shard_cap_bytes`,
/// which this BC allows for exactly that one record rather than splitting
/// it mid-record.
#[derive(Debug, Clone, PartialEq)]
pub struct MechanismABackfillPartition {
    pub bytes: Vec<u8>,
    pub record_count: usize,
    pub oversized_record: bool,
}

/// BC-1.18.008 v1.6 Postcondition 3's **Backfill Recovery Manifest**
/// (F-C3-P6-001): four fields, each computed exactly once, at split time,
/// from the SAME `original_content` buffer that drives Postcondition 2's
/// partitioning — `original_bytes`/`original_sha256` (the pre-split
/// monolithic file's own exact length + SHA-256 content hash) and
/// `final_bytes`/`final_sha256` (the intended LAST partition's own exact
/// length + SHA-256 content hash, i.e. the content the canonical file is
/// intended to hold once Postcondition 5's canonical-truncate step
/// completes). Postcondition 5's Recovery-Confirmation Rule is the SOLE
/// authoritative basis for the later SAFE/DANGEROUS/AMBIGUOUS
/// recovery-confirmation determination in [`heal_or_confirm_already_migrated`]
/// — an exact whole-file `(length, SHA-256)` comparison against these two
/// recorded pairs, NEVER a structural byte-prefix comparison against a
/// re-concatenation of already-sealed shards (Invariant 3).
///
/// Deliberately NOT a field of [`ShardIndex`] itself: `ShardIndex` is
/// constructed via exhaustive struct-literal syntax at multiple pre-existing
/// call sites this burst's scope does not touch (this module's own
/// `bc_1_18_006_roll_tests` unit-test module, and the separate
/// `bc_1_18_007_retention_test.rs`/`bc_1_18_008_backfill_split_test.rs`
/// integration test files, none of which construct a
/// `backfill_manifest` field) — adding a new required struct field there
/// would be a breaking sibling-site change this burst is explicitly scoped
/// not to make (test files are out of bounds; the pre-existing production
/// literals would need updating for a benefit the manifest itself doesn't
/// need, since it is meaningful ONLY for mechanism-A backfill-split
/// indices, never for the ongoing per-write roll/retention indices those
/// call sites build). Instead, [`write_shard_index_for_backfill`] appends
/// this manifest as an independent `[backfill_manifest]` TOML table in the
/// SAME atomic write as the `ShardIndex`'s own serialized text (valid TOML:
/// a new table header may always follow a preceding `[[shard]]`
/// array-of-tables' final entry), and [`read_backfill_manifest`] parses it
/// back out of the SAME file independently of [`load_shard_index`] — which
/// harmlessly ignores the trailing table as an unrecognized key, exactly
/// like any other forward-compatible additive TOML field this module's own
/// additive-field precedents (`sealed_retroactively`, `oversized_record`,
/// `is_preamble_shard`) already establish for `ShardIndexEntry`.
#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
struct BackfillManifest {
    original_bytes: u64,
    original_sha256: String,
    final_bytes: u64,
    final_sha256: String,
}

/// Serialization-only wrapper producing a `[backfill_manifest]` TOML table
/// header around [`BackfillManifest`]'s own scalar fields — a bare
/// `toml::to_string(&manifest)` call would instead emit its fields as
/// top-level (headerless) keys, which is invalid to append after the
/// `ShardIndex`'s own already-emitted `[[shard]]` array-of-tables.
#[derive(Serialize)]
struct BackfillManifestWrapper<'a> {
    backfill_manifest: &'a BackfillManifest,
}

/// Deserialization-only counterpart to [`BackfillManifestWrapper`]: parses
/// JUST the `[backfill_manifest]` table back out of a
/// `<artifact-stem>.shard-index.toml` file's raw text, ignoring every other
/// key (`schema_version`, `[[shard]]`, ...) the same file also carries —
/// `#[serde(default)]` so a file with no `[backfill_manifest]` table at all
/// (e.g. a genuinely corrupted or hand-edited index) deserializes to `None`
/// rather than failing to parse.
#[derive(Debug, Clone, Deserialize)]
struct BackfillManifestDocument {
    #[serde(default)]
    backfill_manifest: Option<BackfillManifest>,
}

/// SHA-256 content hash of `bytes`, hex-encoded (lowercase, no separator) —
/// the exact encoding [`BackfillManifest`]'s `original_sha256`/`final_sha256`
/// fields and [`MechanismABackfillError::AmbiguousRecoveryState`]'s
/// `canonical_sha256` field use throughout.
fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

/// Reads back the [`BackfillManifest`] published at `index_path`'s own
/// `[backfill_manifest]` TOML table (see [`BackfillManifestWrapper`]'s doc
/// comment for why this is a standalone parse rather than a `ShardIndex`
/// field), or `Ok(None)` if the file carries no such table at all.
fn read_backfill_manifest(index_path: &Path) -> io::Result<Option<BackfillManifest>> {
    let text = std::fs::read_to_string(index_path)?;
    let doc: BackfillManifestDocument =
        toml::from_str(&text).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    Ok(doc.backfill_manifest)
}

/// BC-1.18.008 Postcondition 2/Invariant 2 (v1.2's Record-Boundary Marker
/// Table + its single implementable normalization predicate): locate the
/// structural record-boundary byte offsets native to `artifact_stem`'s own
/// append-log format within `content` — the ONLY points
/// [`mechanism_a_partition_for_backfill`] may split at, never an arbitrary
/// byte offset that could divide a single record across two shard files, and
/// never keyed on heading LEVEL (h2 vs h3) alone.
///
/// Per artifact, per the marker table:
/// - `decision-log.md`: PRIMARY key is the `"| D-"` table-row start
///   (Appendix `### D-NNN (...)` sub-clause blocks are secondary atomic
///   units, never a primary boundary — [`is_lesson_record_heading`]-style
///   filtering is unnecessary here since `"| D-"` never collides with a
///   `#`-prefixed heading line at all).
/// - `burst-log.md`: PRIMARY key is any `"## "` (h2) heading, PLUS the
///   confirmed `### Pass-N Fix Burst` h3-exception form
///   ([`is_pass_fix_burst_heading`]) — a nested `### Block N:` sub-heading
///   (or any other `### ` line) is never a boundary.
/// - `lessons.md`: PRIMARY key is an ID-tagged `"## L-<tag>-NNN"` heading, OR
///   `"## LESSON (D-NNN)"`, OR `"## RECURRENCE NOTE (D-NNN)"`
///   ([`is_lesson_h2_record_heading`]), PLUS the confirmed pre-`L-EDP1-052`
///   `### L-<tag>-NNN` h3-exception form ([`is_lesson_record_heading`]) — a
///   nested `### ` sub-heading lacking the `L-<tag>-NNN` tag is never a
///   boundary, and an untagged `"## "` aside is never a boundary either.
/// - `session-checkpoints.md`: PRIMARY key is any `"## "` (h2) heading, with
///   NO content-based filtering (F-002, BC-1.18.008 v1.3: direct inspection
///   of both real `session-checkpoints.md` files confirmed every h2 heading
///   in both is a genuine checkpoint record with zero legitimate non-record
///   h2 asides) — nested `### ` sub-headings are never a boundary.
pub fn mechanism_a_record_boundary_offsets(artifact_stem: &str, content: &[u8]) -> Vec<usize> {
    const H2: &[u8] = b"## ";
    const H3: &[u8] = b"### ";

    match artifact_stem {
        "decision-log" => decision_log_record_boundary_offsets(content),

        "burst-log" => {
            // PRIMARY: any h2 heading is a genuine burst-log record boundary
            // (the marker table names no content-based h2 filter for this
            // artifact — every confirmed h2 record form qualifies).
            let mut offsets = line_anchored_marker_offsets(content, H2);
            // CONFIRMED EXCEPTION: `### Pass-N Fix Burst` h3 records (e.g.
            // the engine cycle's `### Pass-39/40 Fix Burst` records sitting
            // between two h2 records). Every OTHER `### ` line (e.g. a
            // nested `### Block N:` sub-heading) is excluded.
            offsets.extend(
                line_anchored_marker_offsets(content, H3)
                    .into_iter()
                    .filter(|&offset| {
                        is_pass_fix_burst_heading(heading_line(content, offset, H3.len()))
                    }),
            );
            offsets.sort_unstable();
            offsets
        }

        "lessons" => {
            // PRIMARY: an h2 heading that is either ID-tagged
            // (`## L-<tag>-NNN`, the h2 form adopted starting at
            // `L-EDP1-052`) or one of brownfield's own `## LESSON (D-NNN)` /
            // `## RECURRENCE NOTE (D-NNN)` forms — an untagged h2 aside is
            // never a boundary.
            let mut offsets: Vec<usize> = line_anchored_marker_offsets(content, H2)
                .into_iter()
                .filter(|&offset| {
                    is_lesson_h2_record_heading(heading_line(content, offset, H2.len()))
                })
                .collect();
            // CONFIRMED EXCEPTION: the pre-`L-EDP1-052` `### L-<tag>-NNN` h3
            // records (e.g. `### L-EDP1-050`/`### L-EDP1-051`) — a nested
            // `### ` sub-heading lacking the `L-<tag>-NNN` tag (i.e. part of
            // the preceding lesson's own body) is never a boundary.
            offsets.extend(
                line_anchored_marker_offsets(content, H3)
                    .into_iter()
                    .filter(|&offset| {
                        is_lesson_record_heading(heading_line(content, offset, H3.len()))
                    }),
            );
            offsets.sort_unstable();
            offsets
        }

        "session-checkpoints" => {
            // F-002 (BC-1.18.008 v1.3 fix-burst, HIGH): reverted from the
            // MED-3 content-based `is_checkpoint_record_heading` filter
            // back to bare `^## ` (any h2) detection, per product-owner's
            // DECISION (BC-1.18.008 v1.3 Changelog, finding F-002): direct
            // inspection of BOTH real session-checkpoints.md files (brownfield:
            // 182 h2 records; engine: 12 h2 records) confirmed every h2
            // heading in both is a genuine checkpoint record with ZERO
            // legitimate non-record h2 asides — the marker table's own
            // "any h2 = boundary, no confirmed exception forms" row was
            // already correct. The removed filter was itself the defect:
            // being case-sensitive, it silently dropped real records such
            // as the verbatim all-caps `## ARCHIVED CHECKPOINT: ...` form,
            // which matches neither `starts_with("Archived")` nor
            // `contains("Checkpoint")`.
            line_anchored_marker_offsets(content, H2)
        }

        // No known native record-boundary marker for this artifact stem --
        // callers of `mechanism_a_partition_for_backfill` fall back to
        // treating the whole content as a single record when given no
        // boundaries at all.
        _ => Vec::new(),
    }
}

/// P3-002 (S-25.02 F4 cluster-3 adversarial pass-3, MEDIUM): `true` iff
/// `artifact_stem` is one of the four KNOWN mechanism-A backfill-split
/// artifacts this module's own Record-Boundary Marker Table has a rule for
/// (`decision-log`/`burst-log`/`lessons`/`session-checkpoints`) --
/// [`mechanism_a_record_boundary_offsets`]'s own match arms, named here
/// rather than re-derived from its `_ => Vec::new()` fallthrough so
/// [`run_mechanism_a_backfill_split`] can distinguish "this artifact is
/// recognized but its content simply has no markers" (trust the caller,
/// unaffected) from "this artifact_stem has no marker rule at all" (abort
/// fail-loud, P3-002).
fn is_known_mechanism_a_artifact_stem(artifact_stem: &str) -> bool {
    matches!(
        artifact_stem,
        "decision-log" | "burst-log" | "lessons" | "session-checkpoints"
    )
}

/// The text of the heading line starting at `marker_offset + marker_len`
/// (i.e. immediately after the record-boundary marker itself), up to but
/// not including the next `b'\n'` or the end of `content` -- the substring
/// [`is_lesson_record_heading`], [`is_lesson_h2_record_heading`], and
/// [`is_pass_fix_burst_heading`] apply their own real-format discriminators
/// to. `session-checkpoints.md` no longer applies a content-based
/// discriminator (F-002, BC-1.18.008 v1.3): every bare `^## ` heading is a
/// boundary, so this function is not called for that artifact stem.
fn heading_line(content: &[u8], marker_offset: usize, marker_len: usize) -> &[u8] {
    let start = marker_offset + marker_len;
    let rest = &content[start..];
    let end = rest.iter().position(|&b| b == b'\n').unwrap_or(rest.len());
    &rest[..end]
}

/// `true` iff `heading` (already confirmed UTF-8-decodable prose, i.e. the
/// text right after a `lessons.md` record marker) starts with an
/// `L-<tag>-NNN`-shaped ID (`"L-"` + an alphanumeric cycle-prefix tag + `"-"`
/// + a numeric sequence) — the ID-tag shape shared by BOTH lessons.md's
/// pre-`L-EDP1-052` h3-exception records ([`is_lesson_record_heading`]) and
/// its h2 primary-form records ([`is_lesson_h2_record_heading`]). Factored
/// out so both callers apply the identical tag-detection rule rather than
/// two independently-drifting copies (TD-VSDD-060).
///
/// F-C3-P6-003 (S-25.02 F4 cluster-3 CROSS-VENDOR (OpenAI Codex)
/// adversarial pass-6 review, BC-1.18.008 v1.6 Record-Boundary Marker
/// Table's `^L-<tag>-[0-9]+\b` shape): the marker's own regex requires a
/// `\b` WORD BOUNDARY immediately after the numeric id run — the character
/// following the digits (if any) must be a non-word character (whitespace,
/// punctuation, or end-of-string), never another word character continuing
/// the SAME token. A prior implementation only checked that the byte right
/// after the tag's trailing `-` was a digit, then declared a match without
/// ever inspecting what follows the digit RUN — so `"L-EDP1-050details"`
/// (another letter), `"L-EDP1-050_extra"` (underscore, a word character in
/// `\b` terms), and `"L-EDP1-050x"` (another alnum char) all misdetected as
/// genuine `L-EDP1-050` records, even though each merely SHARES that id as
/// a PREFIX of its own, unrelated heading text — prose inside the real
/// `L-EDP1-050` record's own body, not a new record. Consuming the FULL
/// digit run and requiring a word boundary immediately after it (mirrors
/// [`is_pass_fix_burst_heading`]'s own `\b`-after-"Burst" check, P3-003's
/// sibling fix for burst-log.md) closes this gap for both lessons.md marker
/// forms identically, since they share this one predicate (TD-VSDD-060).
fn is_id_tagged_lesson_heading(heading: &str) -> bool {
    let Some(rest) = heading.strip_prefix("L-") else {
        return false;
    };
    let tag_len = rest
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(rest.len());
    if tag_len == 0 {
        return false;
    }
    let Some(after_dash) = rest[tag_len..].strip_prefix('-') else {
        return false;
    };
    let digit_len = after_dash
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(after_dash.len());
    if digit_len == 0 {
        return false;
    }
    after_dash[digit_len..]
        .chars()
        .next()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_')
}

/// PC2 Record-Boundary Marker Table (`lessons.md` row), CONFIRMED EXCEPTION
/// column: `true` iff `heading` (the text right after a `lessons.md` `### `
/// marker) is the confirmed pre-`L-EDP1-052` h3-exception record form
/// (`### L-<tag>-NNN ...`) rather than a nested, non-record sub-heading
/// inside an existing lesson's own body. Grounded in the real
/// `.factory/cycles/v1.0-feature-engine-discipline-pass-1/lessons.md`
/// convention: every genuine pre-052 lesson record is tagged with its own
/// `L-EDP1-NNN`-shaped ID immediately after the marker; that file's own
/// lesson bodies use bold prose labels (`**Pattern:**`, `**Trend:**`, ...)
/// for internal structure, never a further `### ` sub-heading, so ANY
/// `### ` line lacking this ID tag is necessarily nested body content, not
/// a new record.
fn is_lesson_record_heading(heading: &[u8]) -> bool {
    let Ok(heading) = std::str::from_utf8(heading) else {
        return false;
    };
    is_id_tagged_lesson_heading(heading)
}

/// P3-003 (S-25.02 F4 cluster-3 adversarial pass-3, MINOR): `true` iff
/// `heading` starts with `prefix` immediately followed by one-or-more ASCII
/// digits and a closing `")"` — the full `^<prefix>[0-9]+\)` shape the
/// Record-Boundary Marker Table specifies for `## LESSON (D-NNN)` / `##
/// RECURRENCE NOTE (D-NNN)`, not the bare `<prefix>` alone. A bare-prefix
/// `starts_with` check would misdetect `"LESSON (D-foo)"` (non-digit
/// suffix) or `"LESSON (D-)"` (no suffix at all) — headings that merely
/// SHARE the marker's own leading substring without matching its full
/// documented shape — as genuine record boundaries.
fn is_digit_tagged_paren_marker(heading: &str, prefix: &str) -> bool {
    let Some(rest) = heading.strip_prefix(prefix) else {
        return false;
    };
    let digit_len = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    digit_len > 0 && rest[digit_len..].starts_with(')')
}

/// PC2 Record-Boundary Marker Table (`lessons.md` row), PRIMARY column:
/// `true` iff `heading` (the text right after a `lessons.md` `## ` marker)
/// is a GENUINE lesson-record heading — either the `L-EDP1-052`-onward
/// ID-tagged h2 form (`## L-<tag>-NNN ...`), or one of brownfield's own
/// `## LESSON (D-NNN) ...` / `## RECURRENCE NOTE (D-NNN) ...` forms — rather
/// than an untagged h2 aside nested inside a lesson's own body.
fn is_lesson_h2_record_heading(heading: &[u8]) -> bool {
    let Ok(heading) = std::str::from_utf8(heading) else {
        return false;
    };
    is_id_tagged_lesson_heading(heading)
        || is_digit_tagged_paren_marker(heading, "LESSON (D-")
        || is_digit_tagged_paren_marker(heading, "RECURRENCE NOTE (D-")
}

/// PC2 Record-Boundary Marker Table (`burst-log.md` row), CONFIRMED
/// EXCEPTION column: `true` iff `heading` (the text right after a
/// `burst-log.md` `### ` marker) is the confirmed `### Pass-N Fix Burst`
/// h3-exception record form rather than a nested, non-record sub-heading
/// (e.g. `### Block N: ...`) inside an existing h2 burst record's own body.
/// Grounded in the real
/// `.factory/cycles/v1.0-feature-engine-discipline-pass-1/burst-log.md`
/// convention: `### Pass-39 Fix Burst — ...` / `### Pass-40 Fix Burst — ...`
/// are the only two confirmed real h3-level burst-log records, both shaped
/// `"Pass-"` + digits + `" Fix Burst"`.
fn is_pass_fix_burst_heading(heading: &[u8]) -> bool {
    let Ok(heading) = std::str::from_utf8(heading) else {
        return false;
    };
    let Some(rest) = heading.strip_prefix("Pass-") else {
        return false;
    };
    let digit_len = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    if digit_len == 0 {
        return false;
    }
    // P3-003 (S-25.02 F4 cluster-3 adversarial pass-3, MINOR): the marker
    // table's own regex is `^### Pass-[0-9]+ Fix Burst\b` -- the `\b` word
    // boundary REQUIRES the character immediately after "Burst" (if any) to
    // be a non-word character, never another word character continuing the
    // same word. A bare `starts_with(" Fix Burst")` check would misdetect
    // `"Pass-39 Fix Bursting"` -- sharing the same leading substring, but
    // "Bursting" is a DIFFERENT word than "Burst" -- as a genuine
    // Pass-N-Fix-Burst record.
    let Some(after_burst) = rest[digit_len..].strip_prefix(" Fix Burst") else {
        return false;
    };
    after_burst
        .chars()
        .next()
        .is_none_or(|c| !c.is_alphanumeric() && c != '_')
}

/// F-007 (BC-1.18.008 Record-Boundary Marker Table, `decision-log.md` row,
/// MINOR); regex corrected by F-C3-P7-002 (v1.7, EC-012): every byte offset
/// in `content` where a line matches the table's full
/// `^\| D-[0-9]+(\([a-z0-9/]+\)|-[A-Za-z]+)? \|` marker regex -- the literal
/// `"| D-"` prefix, one-or-more ASCII digits, an OPTIONAL sub-clause suffix
/// (a parenthesized `([a-z0-9/]+)` form or a hyphenated `-[A-Za-z]+` form),
/// and a closing `" |"` -- NOT the bare `"| D-"` prefix alone. The
/// bare-prefix form would misdetect a wrapped prose table cell that merely
/// happens to START a continuation line with the literal text `"| D-"`
/// (e.g. `"| D-something, not a row, continues a multi-line cell..."` with
/// no digits/closing pipe) as a record boundary.
fn decision_log_record_boundary_offsets(content: &[u8]) -> Vec<usize> {
    line_anchored_marker_offsets(content, b"| D-")
        .into_iter()
        .filter(|&offset| is_decision_log_row_marker(content, offset))
        .collect()
}

/// `true` iff the line starting at `marker_offset` (already confirmed to
/// start with the literal `"| D-"` prefix by
/// [`line_anchored_marker_offsets`]) matches the full `^\|
/// D-[0-9]+(\([a-z0-9/]+\)|-[A-Za-z]+)? \|` marker regex the Record-Boundary
/// Marker Table specifies for `decision-log.md` (v1.7, F-C3-P7-002/EC-012)
/// -- one-or-more ASCII digits, an OPTIONAL sub-clause suffix
/// ([`decision_log_subclause_suffix_len`]), then a closing `" |"`. The PRIOR
/// bare-only form (`^\| D-[0-9]+ \|`, with no optional-suffix clause)
/// silently failed to match either confirmed suffix form -- the
/// parenthetical `| D-440(a) |` / combined `| D-446(a/b/c/d/e) |` forms, and
/// the hyphenated `| D-355-AMEND |` form -- under-segmenting the artifact by
/// absorbing each sub-clause row into the preceding record (the defect this
/// amendment corrects).
fn is_decision_log_row_marker(content: &[u8], marker_offset: usize) -> bool {
    const PREFIX: &[u8] = b"| D-";
    let rest = &content[marker_offset + PREFIX.len()..];
    let digit_len = rest.iter().take_while(|b| b.is_ascii_digit()).count();
    if digit_len == 0 {
        return false;
    }
    let after_digits = &rest[digit_len..];
    let suffix_len = decision_log_subclause_suffix_len(after_digits);
    after_digits[suffix_len..].starts_with(b" |")
}

/// The optional sub-clause suffix's own byte length at the START of
/// `after_digits` (`0` if it carries neither confirmed form) -- EITHER a
/// parenthesized `([a-z0-9/]+)` sub-clause suffix (one-or-more
/// lowercase-ASCII-letter/digit/`/` bytes between a literal `(` and `)`,
/// e.g. `(a)` or the combined `(a/b/c/d/e)` form) OR a hyphenated
/// `-[A-Za-z]+` suffix (one-or-more ASCII-alphabetic bytes after a literal
/// `-`, e.g. `-AMEND`). A malformed near-match (an empty `()`, an unclosed
/// paren, or a bare trailing hyphen with no following letters) yields `0`,
/// deferring to [`is_decision_log_row_marker`]'s own closing-`" |"` check on
/// the UNCONSUMED bytes -- which then correctly rejects the line as not a
/// boundary, rather than this helper guessing at a partial match.
fn decision_log_subclause_suffix_len(after_digits: &[u8]) -> usize {
    if let Some(inner) = after_digits.strip_prefix(b"(") {
        let inner_len = inner
            .iter()
            .take_while(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || **b == b'/')
            .count();
        if inner_len > 0 && inner.get(inner_len) == Some(&b')') {
            return 1 + inner_len + 1; // '(' + inner + ')'
        }
        return 0;
    }
    if let Some(inner) = after_digits.strip_prefix(b"-") {
        let alpha_len = inner.iter().take_while(|b| b.is_ascii_alphabetic()).count();
        if alpha_len > 0 {
            return 1 + alpha_len; // '-' + letters
        }
        return 0;
    }
    0
}

/// Every byte offset in `content` where `marker` occurs AT THE START OF A
/// LINE (offset `0`, or immediately preceded by `b'\n'`) -- Invariant 2's
/// "never an arbitrary byte offset" guarantee: a mid-line occurrence of the
/// same marker bytes (e.g. inside prose describing the marker) is never
/// mistaken for a real structural record boundary.
fn line_anchored_marker_offsets(content: &[u8], marker: &[u8]) -> Vec<usize> {
    if marker.is_empty() {
        return Vec::new();
    }
    let mut offsets = Vec::new();
    for idx in 0..content.len() {
        let at_line_start = idx == 0 || content[idx - 1] == b'\n';
        if at_line_start && content[idx..].starts_with(marker) {
            offsets.push(idx);
        }
    }
    offsets
}

/// BC-1.18.008 Postcondition 2 (AC-013): partition `content` at
/// `record_boundary_offsets` (never mid-record, Invariant 2), grouping
/// consecutive whole records into chunks each `<= shard_cap_bytes` — except
/// EC-002's single-oversized-record case, flagged
/// `oversized_record: true` on its own partition rather than split. Returns
/// partitions in original-file chronological order; the caller
/// ([`run_mechanism_a_backfill_split`]) treats the LAST partition as the
/// fresh "current" file and seals every partition before it with
/// sequential `seq` numbers starting at 1 (Postcondition 2).
pub fn mechanism_a_partition_for_backfill(
    content: &[u8],
    record_boundary_offsets: &[usize],
    shard_cap_bytes: u64,
) -> Vec<MechanismABackfillPartition> {
    // F-003 (MINOR): guard against a malformed (non-ascending, duplicate,
    // or out-of-bounds) `record_boundary_offsets` argument BEFORE this
    // function's own partitioning loop below ever computes `rec_end -
    // rec_start` (would underflow for a non-ascending pair) or slices
    // `content[rec_start..rec_end]` (would panic for an out-of-bounds
    // offset). This function is `pub`; callers other than
    // `run_mechanism_a_backfill_split` (which independently validates
    // well-formedness upfront via `record_boundary_offsets_are_well_formed`
    // before ever reaching here, and additionally cross-checks against this
    // module's own detected boundaries per F-001) may call it directly with
    // an unchecked offsets list. Rather than trusting the caller and
    // risking a panic in this critical path, fall back to the SAME safe
    // "treat the whole content as a single record" behavior already used
    // for an empty offsets list -- never panic, never silently fabricate a
    // partial/corrupt partition set from offsets that don't genuinely
    // describe this content's own structure.
    if record_boundary_offsets.is_empty()
        || !record_boundary_offsets_are_well_formed(content.len(), record_boundary_offsets)
    {
        // No known native record-boundary marker for this artifact (or a
        // malformed offsets argument) -- treat the whole (non-empty)
        // content as a single record rather than silently producing zero
        // partitions for real content, or panicking on bad input.
        return if content.is_empty() {
            Vec::new()
        } else {
            vec![MechanismABackfillPartition {
                bytes: content.to_vec(),
                record_count: 1,
                oversized_record: content.len() as u64 > shard_cap_bytes,
            }]
        };
    }

    let mut partitions = Vec::new();
    // BLOCKER-1 (Postcondition 6(a)/Postcondition 2), SUPERSEDED by P3-001's
    // Leading-Preamble Handling Rule (BC-1.18.008 v1.4, F-C3-P3-001): a real
    // artifact's leading preamble (a title/section header, plus --
    // decision-log.md only -- the table header/separator rows) belongs to no
    // record of its own, but every byte of it still MUST round-trip
    // (Postcondition 6(a)). BLOCKER-1's original fix unconditionally folded
    // the preamble into whichever partition ends up holding the first
    // record; a fresh-context adversarial pass-3 review (P3-001, HIGH) found
    // that fold has no overflow check -- when
    // `preamble_bytes + first_record_bytes > shard_cap_bytes`, the fold
    // pushes the FIRST sealed shard over cap without any sanctioning
    // `oversized_record`/`is_preamble_shard` flag, re-creating exactly the
    // unsanctioned Postcondition 2 violation Layer 2 exists to eliminate.
    //
    // The preamble is now resolved ONCE, as a single atomic, indivisible
    // packing unit, BEFORE record-based greedy packing begins:
    //   - normal case (`preamble_bytes + first_record_bytes <=
    //     shard_cap_bytes`): the preamble rides in the SAME shard as the
    //     first record -- unchanged from BLOCKER-1/P2-003's own behavior
    //     (P2-003: `partition_bytes` must be seeded with the preamble bytes
    //     too, so the cap decision governing that first partition actually
    //     sees them).
    //   - overflow case (EC-007): preamble alone is under cap, but
    //     preamble+first-record together are not -- the preamble seals as
    //     its own zero-record partition (`record_count: 0`,
    //     `oversized_record: false`; `run_mechanism_a_backfill_split` maps a
    //     zero-record partition to `is_preamble_shard: true` in the
    //     published index) before record packing starts fresh.
    //   - degenerate case (EC-008): the preamble ALONE exceeds cap -- seals
    //     as its own oversized partition (`record_count: 0`,
    //     `oversized_record: true`), reusing EC-002's oversized-atomic-unit
    //     exception rather than a fail-loud abort (content atomicity for an
    //     indivisible structural unit beats the cap, identically to EC-002).
    // When `record_boundary_offsets[0]` is already `0` (no preamble at all),
    // this reduces to the pre-P3-001 seeding exactly (a no-op).
    let preamble_bytes = record_boundary_offsets[0] as u64;
    let first_record_end = record_boundary_offsets
        .get(1)
        .copied()
        .unwrap_or(content.len());
    let first_record_bytes = (first_record_end - record_boundary_offsets[0]) as u64;

    let (mut partition_start, mut partition_bytes, mut partition_records): (usize, u64, usize) =
        if preamble_bytes == 0 {
            (0, 0, 0)
        } else if preamble_bytes > shard_cap_bytes {
            // EC-008 degenerate case.
            partitions.push(MechanismABackfillPartition {
                bytes: content[0..record_boundary_offsets[0]].to_vec(),
                record_count: 0,
                oversized_record: true,
            });
            (record_boundary_offsets[0], 0, 0)
        } else if preamble_bytes + first_record_bytes > shard_cap_bytes {
            // EC-007 overflow case.
            partitions.push(MechanismABackfillPartition {
                bytes: content[0..record_boundary_offsets[0]].to_vec(),
                record_count: 0,
                oversized_record: false,
            });
            (record_boundary_offsets[0], 0, 0)
        } else {
            // Normal case: fold the preamble into the first record's
            // partition, seeding the cap accumulator with its bytes too
            // (P2-003).
            (0, preamble_bytes, 0)
        };

    let n = record_boundary_offsets.len();
    for i in 0..n {
        let rec_start = record_boundary_offsets[i];
        let rec_end = record_boundary_offsets
            .get(i + 1)
            .copied()
            .unwrap_or(content.len());
        let rec_len = (rec_end - rec_start) as u64;

        if rec_len > shard_cap_bytes {
            // EC-002/EC-017: flush whatever was accumulating BEFORE this
            // record -- possibly zero whole records but still the leading
            // preamble bytes on the very first iteration -- then seal the
            // oversized record on its own -- never merged with a neighbor,
            // never split mid-record. Flushing is keyed on unflushed BYTES
            // (`partition_start < rec_start`), not `partition_records > 0`,
            // so a preamble-only leftover (first record itself oversized)
            // is never silently dropped.
            if partition_start < rec_start {
                partitions.push(MechanismABackfillPartition {
                    bytes: content[partition_start..rec_start].to_vec(),
                    record_count: partition_records,
                    oversized_record: false,
                });
            }
            partitions.push(MechanismABackfillPartition {
                bytes: content[rec_start..rec_end].to_vec(),
                record_count: 1,
                oversized_record: true,
            });
            partition_start = rec_end;
            partition_bytes = 0;
            partition_records = 0;
            continue;
        }

        if partition_records > 0 && partition_bytes + rec_len > shard_cap_bytes {
            // Adding this (normally-sized) record would push the current
            // partition over cap -- flush it now; this record starts a
            // fresh partition instead.
            partitions.push(MechanismABackfillPartition {
                bytes: content[partition_start..rec_start].to_vec(),
                record_count: partition_records,
                oversized_record: false,
            });
            partition_start = rec_start;
            partition_bytes = 0;
            partition_records = 0;
        }

        partition_bytes += rec_len;
        partition_records += 1;
    }

    if partition_start < content.len() {
        partitions.push(MechanismABackfillPartition {
            bytes: content[partition_start..].to_vec(),
            record_count: partition_records,
            oversized_record: false,
        });
    }

    partitions
}

/// BC-1.18.008 Postcondition 6(a): `true` iff the byte-for-byte
/// concatenation of `partitions`' own `bytes`, in order, reproduces
/// `original_content` exactly (modulo the shard/index metadata itself,
/// which is new). Part of the mandatory content-preservation verification
/// gate (AC-014) — a hard gate: `false` here MUST abort the whole backfill
/// operation via [`MechanismABackfillError::ContentPreservationFailed`],
/// leaving the original file untouched.
pub fn mechanism_a_verify_backfill_content_preserved(
    original_content: &[u8],
    partitions: &[MechanismABackfillPartition],
) -> bool {
    let mut reconstructed = Vec::with_capacity(original_content.len());
    for partition in partitions {
        reconstructed.extend_from_slice(&partition.bytes);
    }
    reconstructed == original_content
}

/// BC-1.18.008 Postcondition 6(b): `true` iff the sum of every partition's
/// own `record_count` equals `original_record_count` — every structural
/// record that existed in the original file is present in EXACTLY ONE
/// resulting shard (never zero, never two). The other half of the mandatory
/// content-preservation verification gate (AC-014), alongside
/// [`mechanism_a_verify_backfill_content_preserved`].
pub fn mechanism_a_verify_backfill_record_counts_preserved(
    original_record_count: usize,
    partitions: &[MechanismABackfillPartition],
) -> bool {
    let total: usize = partitions
        .iter()
        .map(|partition| partition.record_count)
        .sum();
    total == original_record_count
}

/// BC-1.18.008 Postcondition 6(c)/Invariant 4 (this amendment, F-C3-P3-001):
/// `true` iff EVERY partition's own byte length is `<= shard_cap_bytes`,
/// UNLESS that partition is flagged `oversized_record: true` (EC-002's
/// single-oversized-record exception, or EC-008's degenerate
/// oversized-preamble exception — this module represents both identically
/// via `oversized_record: true` on the partition). The THIRD sub-clause of
/// the SAME mandatory content-preservation verification gate (AC-014),
/// alongside [`mechanism_a_verify_backfill_content_preserved`] (6(a)) and
/// [`mechanism_a_verify_backfill_record_counts_preserved`] (6(b)) —
/// [`run_mechanism_a_backfill_split`] wires this in as a hard, fail-loud
/// gate BEFORE any durable write occurs, checked explicitly against the
/// actual computed partition bytes rather than merely implied by the
/// packer's own behavior (Invariant 4's own text). An unflagged partition
/// exceeding `shard_cap_bytes` is exactly the unsanctioned Postcondition 2
/// violation Layer 2 exists to eliminate.
pub fn mechanism_a_verify_backfill_per_shard_cap_preserved(
    partitions: &[MechanismABackfillPartition],
    shard_cap_bytes: u64,
) -> bool {
    partitions.iter().all(|partition| {
        partition.oversized_record || partition.bytes.len() as u64 <= shard_cap_bytes
    })
}

/// MED-C: `true` iff `offsets` (a non-empty `record_boundary_offsets` list)
/// is STRUCTURALLY well-formed against a `content_len`-byte original
/// content buffer — strictly ascending (no duplicate or out-of-order
/// offset) and every offset strictly less than `content_len`.
///
/// This predicate validates ORDERING and BOUNDS ONLY — it says nothing
/// about whether `offsets` actually corresponds to `artifact_stem`'s real
/// record structure (a well-formed-but-wrong offsets list, e.g. one that
/// silently omits a genuine boundary present in the content, OR adds a
/// spurious extra offset the content's real structure doesn't have, passes
/// this check trivially either way). [`run_mechanism_a_backfill_split`]
/// validates this BEFORE feeding `record_boundary_offsets` into
/// [`mechanism_a_partition_for_backfill`] at all — rejecting a
/// structurally malformed argument here rather than reaching
/// [`mechanism_a_partition_for_backfill`]'s own `rec_end - rec_start`
/// byte-length subtraction, which assumes ascending order and would
/// otherwise panic on unsigned overflow (debug builds) or compute a bogus
/// huge length (release builds) for a non-ascending offset pair (see
/// [`mechanism_a_partition_for_backfill`]'s own F-003 guard, which reuses
/// this same predicate). The genuinely-failable, record-integrity-aware
/// half of the Postcondition 6 hard gate — catching a well-formed offsets
/// list that is nonetheless WRONG against the content's real structure, in
/// EITHER direction (F-001's UNDER-detection, a missing real boundary; or
/// P2-001's OVER-detection, a spurious extra one) — is
/// [`run_mechanism_a_backfill_split`]'s own independent recompute via
/// [`mechanism_a_record_boundary_offsets`], cross-checked by SET EQUALITY
/// (not just cardinality, and not merely a union) against the
/// caller-supplied offsets whenever that oracle recognizes any genuine
/// boundary at all — not this predicate.
fn record_boundary_offsets_are_well_formed(content_len: usize, offsets: &[usize]) -> bool {
    offsets.windows(2).all(|pair| pair[0] < pair[1])
        && offsets.last().is_some_and(|&last| last < content_len)
}

/// BC-1.18.008 Invariant 3 (AC-014): `true` iff a mechanism-A backfill-split
/// has ALREADY completed for this artifact — a shard-index already exists
/// at this artifact's `<artifact-stem>.shard-index.toml` sibling path and
/// already fully accounts for the artifact's pre-existing history. The
/// idempotency short-circuit [`run_mechanism_a_backfill_split`] MUST
/// consult before doing any split work: re-running the backfill against an
/// already-migrated artifact must never double-split it into redundant
/// shards.
pub fn mechanism_a_backfill_already_migrated(
    canonical_path: &Path,
    artifact_stem: &str,
) -> io::Result<bool> {
    let index_path = shard_index_path_for(canonical_path, artifact_stem);
    match std::fs::metadata(&index_path) {
        Ok(_) => Ok(true),
        Err(e) if is_genuinely_missing(&e, &index_path) => Ok(false),
        Err(e) => Err(e),
    }
}

/// One-time mechanism-A backfill-split outcome (BC-1.18.008 Postcondition
/// 1/3/4).
#[derive(Debug, Clone, PartialEq)]
pub enum MechanismABackfillOutcome {
    /// [`mechanism_a_backfill_already_migrated`] found this artifact
    /// already fully migrated (Invariant 3's idempotency short-circuit) —
    /// no shards were (re-)produced this call.
    AlreadyMigrated,
    /// The backfill-split ran and published `sealed_count` newly-sealed
    /// shards (`ceil(original_bytes / shard_cap_bytes) - 1`) plus a fresh
    /// current file, publishing the full shard index for the complete
    /// pre-existing history in this SAME operation (Postcondition 3).
    /// `archived_count` names how many of the OLDEST of those, if any, were
    /// ALSO archived in this SAME operation because the resulting shard
    /// count already exceeded `retention_count` (Postcondition 4,
    /// composing immediately with BC-1.18.007's retention policy — never
    /// deferred to a later event).
    Migrated {
        sealed_count: u32,
        archived_count: u32,
    },
    /// F4 BC-cluster-3 adversarial-review finding HIGH-2 (BC-1.18.008
    /// Postcondition 5, Invariant 3; VP-124 Property Statement 1): this
    /// re-run landed on the DANGEROUS crash window between a prior run's
    /// index-publish and canonical-truncate writes (the shard-index already
    /// fully and correctly accounted for `sealed_count` sealed shards, but
    /// the canonical file was still holding the complete pre-split
    /// content). The self-heal completed the interrupted truncation this
    /// call — never re-sealing any already-sealed content into new,
    /// redundant shards. Distinct from `AlreadyMigrated` (the SAFE window:
    /// no gap to heal) and from `Migrated` (a genuinely fresh split).
    Healed { sealed_count: u32 },
}

/// BC-1.18.008 Postcondition 1/2/3/4/5/6, Invariant 1/2/3 (AC-013/AC-014):
/// the mechanism-A one-time backfill-split entry point, executed exactly
/// once per artifact, as a one-time migration task at F4 activation — never
/// as an ongoing per-write mechanism (Postcondition 1; distinct from
/// [`execute_roll`]'s reactive, per-write roll).
///
/// Reuses BC-1.18.006's atomic-write / shard-index-schema primitives
/// (Invariant 1) via a stage-then-verify-then-atomically-replace sequence
/// (Postcondition 5): nothing durable about the original monolithic file's
/// role changes until every resulting shard file AND the shard-index have
/// been staged AND BOTH [`mechanism_a_verify_backfill_content_preserved`]
/// and [`mechanism_a_verify_backfill_record_counts_preserved`] have passed
/// (AC-014's hard gate) — a failure at any point aborts with the original
/// file untouched and safely re-runnable from scratch (EC-003/EC-004).
/// Idempotent (Invariant 3) via
/// [`mechanism_a_backfill_already_migrated`]'s upfront check. Composes
/// immediately with BC-1.18.007's retention policy in the SAME operation
/// when the resulting shard count already exceeds `retention_count`
/// (Postcondition 4) — `retention_count` is threaded in explicitly rather
/// than re-derived, since no shard-index (and therefore no
/// `ShardIndex::retention_count`) exists yet for an artifact that has never
/// been backfilled.
pub fn run_mechanism_a_backfill_split(
    entry: &ShardEntry,
    canonical_path: &Path,
    record_boundary_offsets: &[usize],
    retention_count: u32,
) -> Result<MechanismABackfillOutcome, MechanismABackfillError> {
    let index_path = shard_index_path_for(canonical_path, &entry.artifact_stem);

    // Invariant 3: idempotency short-circuit, checked BEFORE any split work
    // or disk read of `canonical_path`'s own content.
    let already_migrated =
        mechanism_a_backfill_already_migrated(canonical_path, &entry.artifact_stem).map_err(
            |source| MechanismABackfillError::Io {
                artifact_stem: entry.artifact_stem.clone(),
                source,
            },
        )?;
    if already_migrated {
        // HIGH-2/VP-124: index-existence alone cannot distinguish a fully
        // committed prior run (SAFE) from a crash landing in the DANGEROUS
        // gap between this function's own index-publish and
        // canonical-truncate writes below (see their ordering comment) --
        // self-heal that gap here rather than silently accepting a
        // permanent whole-corpus duplication.
        return heal_or_confirm_already_migrated(entry, canonical_path, &index_path);
    }

    let original_content =
        std::fs::read(canonical_path).map_err(|source| MechanismABackfillError::Io {
            artifact_stem: entry.artifact_stem.clone(),
            source,
        })?;

    // MED-C (Postcondition 6(b) load-bearing fix): validate
    // `record_boundary_offsets` against the ACTUAL just-read
    // `original_content` bytes BEFORE trusting it to drive partitioning or
    // the record-count expectation at all — see
    // [`record_boundary_offsets_are_well_formed`]'s own doc comment for why
    // this is necessary for the Postcondition 6 hard gate to be genuinely
    // load-bearing rather than tautological.
    if !record_boundary_offsets.is_empty()
        && !record_boundary_offsets_are_well_formed(original_content.len(), record_boundary_offsets)
    {
        return Err(MechanismABackfillError::ContentPreservationFailed {
            artifact_stem: entry.artifact_stem.clone(),
            detail: format!(
                "record_boundary_offsets {record_boundary_offsets:?} is not well-formed \
                 against the {}-byte original content read from disk (offsets must be \
                 strictly ascending, non-duplicate, and in-bounds)",
                original_content.len()
            ),
        });
    }

    let partitions = mechanism_a_partition_for_backfill(
        &original_content,
        record_boundary_offsets,
        entry.shard_cap_bytes,
    );

    // F-001/P2-001 (BLOCKER/HIGH, Postcondition 6(b) load-bearing fix,
    // BOTH-DIRECTIONS): independently recompute this artifact's TRUE record
    // boundaries from the just-read `original_content` via this module's own
    // oracle (`mechanism_a_record_boundary_offsets`) and require the
    // caller-supplied `record_boundary_offsets` to match that oracle set
    // EXACTLY (set equality -- same elements, neither a subset nor a
    // superset) whenever the oracle independently recognizes ANY genuine
    // structural boundary at all. Without this, both sides of the
    // Postcondition 6(b) comparison
    // (`mechanism_a_verify_backfill_record_counts_preserved`, just below)
    // derived from the SAME caller-supplied `record_boundary_offsets` value
    // -- `mechanism_a_partition_for_backfill`'s own `record_count` sum, by
    // construction, always totals exactly `record_boundary_offsets.len()`
    // too -- so a caller-supplied offsets list that is well-formed (MED-C's
    // check, above, passes) but wrong against the content's real structure
    // would tautologically report "preserved" regardless of how wrong it
    // was. A prior fix (F-001) unioned the caller's list with the oracle's,
    // which catches UNDER-detection (a caller list missing a real boundary)
    // but is structurally blind to OVER-detection (a caller list that is a
    // STRICT SUPERSET of the oracle -- every real boundary present, plus one
    // spurious extra landing mid-record): the union contributes nothing new
    // in that shape (`|caller ∪ oracle| == |caller|`), so both sides of the
    // comparison stayed tautologically equal even though the spurious
    // offset would physically split a real record across two shard files. A
    // fresh-context adversarial pass-2 review (P2-001) found this gap; see
    // this function's own module-level doc comment and the F-001/P2-001
    // tests for the full mechanism. When the oracle recognizes NO
    // independently-detectable boundary at all for this artifact_stem/
    // content pair, two shapes are possible, and P3-002 (S-25.02 F4
    // cluster-3 adversarial pass-3, MEDIUM) requires them to be handled
    // DIFFERENTLY:
    //   - `artifact_stem` is one of the four KNOWN mechanism-A artifacts
    //     (`decision-log`/`burst-log`/`lessons`/`session-checkpoints`), but
    //     THIS particular content simply does not match any real marker
    //     form (e.g. a synthetic test fixture) -- there is genuinely
    //     nothing for the oracle to corroborate OR refute against for a
    //     recognized artifact's own content, so the caller-supplied offsets
    //     are trusted at face value, unchanged from pre-P3-002 behavior.
    //   - `artifact_stem` has NO marker rule at all (falls through
    //     `mechanism_a_record_boundary_offsets`'s own `_ => Vec::new()`
    //     arm) -- the oracle has nothing to corroborate the caller's claim
    //     against, full stop, so trusting it at face value is exactly the
    //     silently-mis-partition outcome Postcondition 2's Normalization
    //     rule forbids ("If a future cycle introduces a heading form
    //     outside this enumeration, Postcondition 6's fail-loud
    //     content-preservation gate MUST reject the backfill run rather
    //     than silently mis-partition"). This ABORTS fail-loud.
    let true_boundary_offsets =
        mechanism_a_record_boundary_offsets(&entry.artifact_stem, &original_content);
    let original_record_count = if record_boundary_offsets.is_empty() {
        // F-C3-P5-001 (S-25.02 F4 cluster-3 adversarial pass-5, LOW): the
        // empty-caller-offsets twin of P3-002, above. An empty
        // `record_boundary_offsets` argument must NOT be trusted at face
        // value as "no real records exist" without first consulting the
        // SAME oracle the non-empty branch below already cross-checks
        // against -- otherwise a known stem's real, marker-bearing content
        // silently collapses to a single partition (EC-016's zero-shard
        // no-op) even though the oracle can independently find genuine
        // record boundaries the caller's empty list missed entirely. This
        // is exactly the silently-mis-partition outcome Postcondition 2's
        // Normalization rule forbids. Abort fail-loud ONLY when the oracle
        // finds real structure; a genuinely empty file, or a known stem
        // whose content has zero oracle-detectable markers (e.g. a
        // title-only preamble), has nothing for the oracle to have missed,
        // so both remain valid zero/single-record no-ops.
        if !true_boundary_offsets.is_empty() {
            return Err(MechanismABackfillError::ContentPreservationFailed {
                artifact_stem: entry.artifact_stem.clone(),
                detail: format!(
                    "record_boundary_offsets is empty but the independently-detected oracle \
                     boundary set {true_boundary_offsets:?} finds genuine record boundaries in \
                     this artifact's own on-disk content -- an empty caller-supplied \
                     record_boundary_offsets must never silently no-op a mandated split when \
                     real records exist (Postcondition 6(b)/Invariant 2/EC-004/EC-006, \
                     F-C3-P5-001, twin of P3-002)"
                ),
            });
        }
        usize::from(!original_content.is_empty())
    } else if true_boundary_offsets.is_empty() {
        if !is_known_mechanism_a_artifact_stem(&entry.artifact_stem) {
            return Err(MechanismABackfillError::ContentPreservationFailed {
                artifact_stem: entry.artifact_stem.clone(),
                detail: format!(
                    "artifact_stem \"{}\" has no known BC-1.18.008 Record-Boundary Marker Table \
                     rule at all, so the independently-detected oracle boundary set is empty and \
                     cannot corroborate the caller-supplied record_boundary_offsets \
                     {record_boundary_offsets:?} -- per Postcondition 2's Normalization rule, an \
                     unrecognized artifact's content must never be silently mis-partitioned by \
                     trusting caller-supplied offsets at face value (P3-002)",
                    entry.artifact_stem
                ),
            });
        }
        record_boundary_offsets.len()
    } else {
        let mut sorted_caller_offsets: Vec<usize> = record_boundary_offsets.to_vec();
        sorted_caller_offsets.sort_unstable();
        sorted_caller_offsets.dedup();
        let mut sorted_true_offsets = true_boundary_offsets;
        sorted_true_offsets.sort_unstable();
        if sorted_caller_offsets != sorted_true_offsets {
            return Err(MechanismABackfillError::ContentPreservationFailed {
                artifact_stem: entry.artifact_stem.clone(),
                detail: format!(
                    "record_boundary_offsets {record_boundary_offsets:?} diverges from the \
                     independently-detected true record boundaries {sorted_true_offsets:?} for \
                     this artifact's own on-disk content -- every genuine boundary must be \
                     present and no spurious offset may be added (Postcondition 6(b)/Invariant \
                     2/EC-004/EC-006 -- catches both UNDER-detection, F-001, and \
                     OVER-detection, P2-001)"
                ),
            });
        }
        record_boundary_offsets.len()
    };

    // AC-014/Postcondition 6 hard gate: mandatory content-preservation and
    // record-integrity verification BEFORE any durable write occurs.
    if !mechanism_a_verify_backfill_content_preserved(&original_content, &partitions) {
        return Err(MechanismABackfillError::ContentPreservationFailed {
            artifact_stem: entry.artifact_stem.clone(),
            detail: "concatenation of the computed partitions does not reproduce the original \
                      monolithic content byte-for-byte"
                .to_string(),
        });
    }
    if !mechanism_a_verify_backfill_record_counts_preserved(original_record_count, &partitions) {
        return Err(MechanismABackfillError::ContentPreservationFailed {
            artifact_stem: entry.artifact_stem.clone(),
            detail: "sum of the computed partitions' record counts does not match the original \
                      record count"
                .to_string(),
        });
    }
    // Postcondition 6(c)/Invariant 4 (this amendment, F-C3-P3-001): the
    // THIRD sub-clause of the SAME mandatory content-preservation hard gate
    // -- every computed partition's own byte length must respect
    // `shard_cap_bytes`, EXCEPT one flagged `oversized_record: true`
    // (EC-002/EC-008). Checked against the actual computed partition bytes,
    // BEFORE any durable write occurs, never merely implied by the packer's
    // own behavior.
    if !mechanism_a_verify_backfill_per_shard_cap_preserved(&partitions, entry.shard_cap_bytes) {
        return Err(MechanismABackfillError::ContentPreservationFailed {
            artifact_stem: entry.artifact_stem.clone(),
            detail: format!(
                "at least one computed partition exceeds shard_cap_bytes ({}) without being \
                 flagged oversized_record -- this is exactly the unsanctioned Postcondition 2 \
                 violation Layer 2 exists to eliminate (Postcondition 6(c)/Invariant 4)",
                entry.shard_cap_bytes
            ),
        });
    }

    // EC-016: content already fits within a single partition -- no sealing
    // is structurally necessary. The canonical file is left COMPLETELY
    // UNCHANGED; a shard-index is still created and registered, with zero
    // [[shard]] entries.
    if partitions.len() <= 1 {
        let index = fresh_backfill_shard_index(entry, retention_count, Vec::new());
        // Postcondition 3's Backfill Recovery Manifest: even in the
        // no-split EC-001 case, the manifest is populated from the SAME
        // `original_content` buffer already read above -- with `final_*`
        // equal to `original_*` (the canonical file is left COMPLETELY
        // UNCHANGED, so what it will hold once "done" IS the original
        // content). This artifact will always short-circuit at
        // `mechanism_a_backfill_already_migrated`'s zero-shard branch on
        // any later re-invocation (`heal_or_confirm_already_migrated`'s own
        // `index.shards.is_empty()` guard, below), so the manifest is not
        // load-bearing for THIS artifact's own recovery -- it is populated
        // anyway for consistency with the "computed once, at split time"
        // contract Postcondition 3 states unconditionally.
        let manifest = BackfillManifest {
            original_bytes: original_content.len() as u64,
            original_sha256: sha256_hex(&original_content),
            final_bytes: original_content.len() as u64,
            final_sha256: sha256_hex(&original_content),
        };
        write_shard_index_for_backfill(&index_path, &index, &entry.artifact_stem, Some(&manifest))?;
        return Ok(MechanismABackfillOutcome::Migrated {
            sealed_count: 0,
            archived_count: 0,
        });
    }

    // Postcondition 2: every partition before the last is sealed, in
    // chronological (original-file) order, with sequential `seq` numbers
    // starting at 1; the LAST partition becomes the fresh current file.
    //
    // Deliberately reuses `write_atomic` (rename-based, unconditionally
    // overwriting) rather than `publish_sealed_shard`'s `write_exclusive`
    // write-once primitive: BC-1.18.006's per-write write-once/immutability
    // guarantee governs shards sealed by the ONGOING mechanism, from the
    // moment THIS one-time migration durably completes onward. Before that,
    // a same-named leftover at a destination `seq` path can only be a
    // stale, incomplete artifact of a crashed PRIOR backfill attempt
    // (EC-003/Postcondition 5) -- not real sealed history -- and must be
    // safely overwritten on restart, never refused.
    let sealed_partitions = &partitions[..partitions.len() - 1];
    let current_partition = &partitions[partitions.len() - 1];
    let sealed_count = sealed_partitions.len() as u32;
    let sealed_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

    let mut shard_entries = Vec::with_capacity(sealed_partitions.len());
    for (i, partition) in sealed_partitions.iter().enumerate() {
        let seq = (i + 1) as u32;
        let sealed_filename = format!("{}.{seq:04}.md", entry.artifact_stem);
        let sealed_path = shard_sibling_path(canonical_path, &sealed_filename);
        mechanism_a_write_and_verify_sealed_shard(
            &sealed_path,
            &partition.bytes,
            &entry.artifact_stem,
        )?;

        shard_entries.push(ShardIndexEntry {
            seq,
            path: sealed_filename,
            sealed_at: sealed_at.clone(),
            bytes_at_seal: partition.bytes.len() as u64,
            sealed_retroactively: false,
            // P2-002 (EC-002 Canonical Test Vector): surface
            // `mechanism_a_partition_for_backfill`'s own already-computed
            // `oversized_record` flag through to the published shard-index
            // entry -- previously computed but silently dropped here.
            oversized_record: partition.oversized_record,
            // P3-001 (Leading-Preamble Handling Rule, Postcondition 3): a
            // preamble shard is the ONLY partition shape with zero domain
            // records (every ordinary record-bearing partition has
            // `record_count >= 1` by construction) -- `record_count == 0`
            // is therefore a sound, sufficient discriminator for
            // `is_preamble_shard` without needing a dedicated field on
            // `MechanismABackfillPartition` itself.
            is_preamble_shard: partition.record_count == 0,
            // Postcondition 3: surface this partition's own record count
            // through to the published index entry -- `0` for a preamble
            // shard, `>= 1` for an ordinary record-bearing shard.
            records: partition.record_count as u32,
        });
    }

    let mut index = fresh_backfill_shard_index(entry, retention_count, shard_entries);

    // Postcondition 4: compose immediately with BC-1.18.007's retention
    // policy in this SAME operation when the backfill already produced more
    // shards than `retention_count` -- never deferred to a later event.
    let archived_count = archive_overflow_shards(&mut index, canonical_path)
        .map_err(|source| MechanismABackfillError::Io {
            artifact_stem: entry.artifact_stem.clone(),
            source: io::Error::other(source.to_string()),
        })?
        .len() as u32;

    // Publish the full shard index for the complete pre-existing history in
    // this SAME operation (Postcondition 3) -- deliberately BEFORE the
    // canonical-file rewrite below. A crash between the two leaves a
    // fully-correct, fully-indexed set of sealed shards with the canonical
    // file still (harmlessly, TEMPORARILY) holding the complete pre-split
    // content rather than just the final partition -- HIGH-2/VP-124:
    // `heal_or_confirm_already_migrated` (invoked via this function's own
    // upfront `already_migrated` branch above on the NEXT run) detects
    // exactly this gap and completes the interrupted truncation itself, so
    // the "operator-fixable by re-running" property above is genuinely
    // self-healing, not just self-evident. The alternative ordering
    // (canonical rewritten first) is strictly worse: a crash in ITS gap
    // would leave `mechanism_a_backfill_already_migrated` reporting
    // "not yet migrated" while the canonical file has ALREADY been shrunk
    // to just the final partition, so a naive restart would misread that
    // shrunk remnant as the artifact's true original content and silently
    // orphan every already-sealed shard from the index it (re-)computes.
    //
    // F-C3-P6-001 (BC-1.18.008 v1.6 Postcondition 3): the Backfill Recovery
    // Manifest, computed from the SAME `original_content` buffer that drove
    // Postcondition 2's partitioning above and the SAME `current_partition`
    // that becomes the fresh canonical file below -- published in this SAME
    // atomic index-publish write, durably BEFORE the canonical-truncate
    // write that follows it, so it is guaranteed to exist for
    // `heal_or_confirm_already_migrated`'s Recovery-Confirmation Rule on
    // any later re-invocation, including one landing in the crash window
    // this comment block already describes.
    let backfill_manifest = BackfillManifest {
        original_bytes: original_content.len() as u64,
        original_sha256: sha256_hex(&original_content),
        final_bytes: current_partition.bytes.len() as u64,
        final_sha256: sha256_hex(&current_partition.bytes),
    };
    write_shard_index_for_backfill(
        &index_path,
        &index,
        &entry.artifact_stem,
        Some(&backfill_manifest),
    )?;

    // F-C3-P8-002 (BC-1.18.008 v1.8 Postcondition 6(c)'s "Extension to the
    // happy-path canonical-truncate write", Invariant 5, EC-013): this is
    // the ORDINARY, non-recovery, first-time/uninterrupted completion of
    // Postcondition 5 step (ii)'s destructive, source-overwriting write --
    // structurally identical in kind to the DANGEROUS-window heal write
    // `heal_or_confirm_already_migrated` already verifies (F-C3-P7-001), and
    // to the sealed-shard writes above (F-C3-P6-002). It receives the SAME
    // post-hoc disk read-back discipline via the SAME shared
    // `write_and_read_back` primitive, verified against the SAME Backfill
    // Recovery Manifest `(final_bytes, final_sha256)` pair already durably
    // published in Postcondition 5 step (i) immediately above -- no new
    // oracle value is computed here.
    let read_back = write_and_read_back(
        canonical_path,
        &current_partition.bytes,
        &entry.artifact_stem,
    )?;
    let read_back_bytes = read_back.len() as u64;
    let read_back_sha256 = sha256_hex(&read_back);
    if read_back_bytes != backfill_manifest.final_bytes
        || read_back_sha256 != backfill_manifest.final_sha256
    {
        // The destructive write has already happened at this point -- fail
        // loud with the NEW `E-SHD-013` code (distinct from `E-SHD-012`,
        // which covers the crash-recovery heal's own read-back mismatch)
        // rather than reporting `Migrated` over silently-corrupted content.
        // The shard-index and Backfill Recovery Manifest are already
        // durably published (Postcondition 5 step (i), above), so a
        // follow-on re-invocation resolves the now-corrupted canonical file
        // via the ordinary AMBIGUOUS (`E-SHD-011`) recovery-confirmation
        // path per EC-013's own Canonical Test Vector note -- deliberately
        // not suppressed here.
        return Err(MechanismABackfillError::CanonicalWriteVerificationFailed {
            artifact_stem: entry.artifact_stem.clone(),
            read_back_bytes,
            read_back_sha256,
            final_bytes: backfill_manifest.final_bytes,
            final_sha256: backfill_manifest.final_sha256,
        });
    }

    Ok(MechanismABackfillOutcome::Migrated {
        sealed_count,
        archived_count,
    })
}

/// F4 BC-cluster-3 adversarial-review finding HIGH-2, RESTRUCTURED under
/// F-C3-P6-001 (S-25.02 F4 cluster-3 CROSS-VENDOR (OpenAI Codex)
/// adversarial pass-6 review, HIGH; BC-1.18.008 v1.6 Postcondition 5's
/// Recovery-Confirmation Rule, Invariant 3, EC-009/EC-010): distinguishes
/// the SAFE crash window (a prior [`run_mechanism_a_backfill_split`] call
/// completed in full) from the DANGEROUS one (that call's own index-publish
/// write landed durably but its canonical-truncate write, immediately
/// after, did not) — and self-heals the DANGEROUS case, or fails loud
/// (`E-SHD-011`) on a genuinely AMBIGUOUS one.
///
/// **Manifest-based, never a structural byte-prefix heuristic.** A PRIOR
/// implementation classified the DANGEROUS window structurally: read every
/// already-sealed shard back from disk, concatenate them (`sealed_concat`),
/// and check whether the canonical file's own leading bytes reproduce that
/// concatenation as a PREFIX. BC-1.18.008 v1.6 retires this — it
/// FALSE-POSITIVES whenever the artifact's real content legitimately
/// repeats a sealed shard's exact bytes as a prefix of the SAFE-window
/// final partition (EC-009; realistic for `session-checkpoints.md`, which
/// imposes no uniqueness requirement on checkpoint headings/bodies), and it
/// has NO ambiguous-state disposition at all — it only ever silently
/// resolved SAFE or DANGEROUS, which is the exact class of defect
/// Postcondition 5's Recovery-Confirmation Rule exists to eliminate.
///
/// The determination now reads the canonical file's CURRENT on-disk bytes
/// exactly once and computes their exact whole-file `(length, SHA-256)`,
/// then compares that pair against the Backfill Recovery Manifest's two
/// recorded pairs ([`read_backfill_manifest`]) — the SOLE authoritative
/// basis for this determination (Invariant 3):
///   - matches `(final_bytes, final_sha256)` exactly ⇒ SAFE: the prior
///     run's canonical-truncate write already completed; no action.
///   - matches `(original_bytes, original_sha256)` exactly ⇒ DANGEROUS,
///     unambiguously confirmed: the canonical file still holds the FULL
///     pre-split content byte-for-byte, so step (ii) never ran (or crashed
///     before writing any bytes). Healing completes the interrupted write.
///     The bytes to write are the canonical file's own trailing
///     `original_bytes - final_bytes` bytes — safe to isolate ONLY because
///     the canonical file's FULL contents were just confirmed, by exact
///     SHA-256 match, to be byte-identical to the very `original_content`
///     buffer Postcondition 2's packer partitioned at split time, so this
///     is the SAME final partition already independently established
///     then, never a heuristic re-derivation from an unconfirmed prefix
///     match (the defect this rule corrects).
///   - matches NEITHER pair ⇒ AMBIGUOUS: fails loud with `E-SHD-011`
///     (EC-010); the canonical file is NOT written to under any
///     circumstance.
///
/// Healing never re-derives partitions or re-seals any shard — the
/// shard-index already correctly and completely accounts for the
/// artifact's pre-existing history (Postcondition 3 already happened); it
/// only finishes the single interrupted write.
fn heal_or_confirm_already_migrated(
    entry: &ShardEntry,
    canonical_path: &Path,
    index_path: &Path,
) -> Result<MechanismABackfillOutcome, MechanismABackfillError> {
    let io_err = |source: io::Error| MechanismABackfillError::Io {
        artifact_stem: entry.artifact_stem.clone(),
        source,
    };

    // `mechanism_a_backfill_already_migrated` (this function's only caller)
    // just confirmed the index file exists -- a `None` here would mean it
    // vanished in the interim (a DIFFERENT, worse corruption than the crash
    // window this function heals), so this surfaces loudly rather than
    // silently falling back to a fresh migration.
    let index = load_shard_index(index_path)
        .map_err(io_err)?
        .ok_or_else(|| {
            io_err(io::Error::other(format!(
                "shard-index '{}' reported present by `mechanism_a_backfill_already_migrated` but \
             vanished before this self-heal check could read it back",
                index_path.display()
            )))
        })?;

    if index.shards.is_empty() {
        // EC-016: zero-shard registration -- the original run never
        // rewrote the canonical file at all (nothing was ever sealed), so
        // there is no interrupted truncation to complete.
        return Ok(MechanismABackfillOutcome::AlreadyMigrated);
    }

    // Postcondition 5's Recovery-Confirmation Rule: the Backfill Recovery
    // Manifest is the SOLE authoritative basis for this determination
    // (Invariant 3) -- a shard-index with sealed shards but no manifest at
    // all cannot support ANY safe disposition, so this fails loud under the
    // SAME `E-SHD-011` code rather than falling back to a structural
    // heuristic.
    let manifest = read_backfill_manifest(index_path)
        .map_err(io_err)?
        .ok_or_else(|| MechanismABackfillError::MissingBackfillManifest {
            artifact_stem: entry.artifact_stem.clone(),
        })?;

    let canonical_bytes = std::fs::read(canonical_path).map_err(io_err)?;
    let canonical_len = canonical_bytes.len() as u64;
    let canonical_sha256 = sha256_hex(&canonical_bytes);

    if canonical_len == manifest.final_bytes && canonical_sha256 == manifest.final_sha256 {
        // SAFE window: the canonical file's exact whole-file (length,
        // SHA-256) already matches the manifest's recorded intended-final
        // pair -- the prior run's canonical-truncate write already
        // completed. Nothing to heal, nothing written.
        return Ok(MechanismABackfillOutcome::AlreadyMigrated);
    }

    if canonical_len == manifest.original_bytes && canonical_sha256 == manifest.original_sha256 {
        // DANGEROUS window, unambiguously confirmed: the canonical file's
        // exact whole-file (length, SHA-256) matches the manifest's
        // recorded pre-split-original pair byte-for-byte -- step (ii) never
        // ran. Postcondition 5's Manifest-Authoritative Slice-and-Verify
        // Rule (F-C3-P7-001): the healed content is ALWAYS obtained by
        // slicing the canonical file's own current bytes at an offset
        // derived from the Manifest itself (`original_bytes - final_bytes`)
        // -- NEVER by summing the shard-index's own `bytes_at_seal` fields
        // (a separate, independently-corruptible piece of on-disk state
        // the PRIOR implementation relied on, EC-011) -- and that slice is
        // NEVER written before being confirmed against the Manifest's own
        // `final_bytes`/`final_sha256` pair.
        let offset = manifest
            .original_bytes
            .checked_sub(manifest.final_bytes)
            .ok_or_else(|| MechanismABackfillError::SliceVerificationFailed {
                artifact_stem: entry.artifact_stem.clone(),
                detail: format!(
                    "the Backfill Recovery Manifest's final_bytes ({}) exceeds its own \
                     original_bytes ({}) -- the Manifest is internally inconsistent; refusing to \
                     derive a slice offset from it",
                    manifest.final_bytes, manifest.original_bytes
                ),
            })?;
        let offset = usize::try_from(offset).map_err(|_| {
            MechanismABackfillError::SliceVerificationFailed {
                artifact_stem: entry.artifact_stem.clone(),
                detail: format!(
                    "the Manifest-derived slice offset {offset} does not fit a platform usize"
                ),
            }
        })?;
        let sliced = canonical_bytes.get(offset..).ok_or_else(|| {
            MechanismABackfillError::SliceVerificationFailed {
                artifact_stem: entry.artifact_stem.clone(),
                detail: format!(
                    "the Manifest-derived slice offset {offset} exceeds the canonical file's own \
                     confirmed-original length {canonical_len}"
                ),
            }
        })?;

        // Step 2 (hard gate, pre-write): the candidate slice MUST verify
        // against the Manifest's own recorded final-partition pair before
        // anything is written -- a wrong slice (a future regression, a
        // corrupted Manifest value, or a legacy caller still deriving the
        // offset from the shard index) fails this check and is never
        // written.
        let sliced_len = sliced.len() as u64;
        let sliced_sha256 = sha256_hex(sliced);
        if sliced_len != manifest.final_bytes || sliced_sha256 != manifest.final_sha256 {
            return Err(MechanismABackfillError::SliceVerificationFailed {
                artifact_stem: entry.artifact_stem.clone(),
                detail: format!(
                    "the Manifest-derived candidate slice ({sliced_len} bytes, sha256 \
                     {sliced_sha256}) does not verify against the Backfill Recovery Manifest's \
                     own recorded final partition ({} bytes, sha256 {}) -- refusing to write an \
                     unverified slice; canonical file left untouched pending operator \
                     investigation",
                    manifest.final_bytes, manifest.final_sha256
                ),
            });
        }

        // Step 3: both checks passed -- write the verified slice, then
        // (Postcondition 6(c)/Invariant 4's F-C3-P7-001 extension) perform
        // the SAME post-hoc disk read-back discipline sealed-shard writes
        // already receive (F-C3-P6-002) on the heal's OWN destructive
        // write, since it is the final write of the interrupted migration
        // and the pre-heal content is irretrievably gone the moment it
        // lands.
        let read_back = write_and_read_back(canonical_path, sliced, &entry.artifact_stem)?;
        let read_back_len = read_back.len() as u64;
        let read_back_sha256 = sha256_hex(&read_back);
        if read_back_len != manifest.final_bytes || read_back_sha256 != manifest.final_sha256 {
            return Err(MechanismABackfillError::SliceVerificationFailed {
                artifact_stem: entry.artifact_stem.clone(),
                detail: format!(
                    "post-hoc disk read-back of the heal's own write to '{}' \
                     ({read_back_len} bytes, sha256 {read_back_sha256}) does not match the \
                     Backfill Recovery Manifest's recorded final partition ({} bytes, sha256 \
                     {}) -- the write silently truncated, partially flushed, or otherwise \
                     landed corrupted bytes on disk; the destructive write already happened, so \
                     this surfaces the corruption immediately for operator remediation from git \
                     history/backup rather than reporting the heal complete",
                    canonical_path.display(),
                    manifest.final_bytes,
                    manifest.final_sha256
                ),
            });
        }

        return Ok(MechanismABackfillOutcome::Healed {
            sealed_count: index.shards.len() as u32,
        });
    }

    // AMBIGUOUS (EC-010): the canonical file's exact whole-file
    // (length, SHA-256) matches NEITHER recorded manifest pair -- never
    // silently default to either the SAFE or DANGEROUS disposition. Fail
    // loud; the canonical file is NOT written to.
    Err(MechanismABackfillError::AmbiguousRecoveryState {
        artifact_stem: entry.artifact_stem.clone(),
        canonical_bytes: canonical_len,
        canonical_sha256,
        original_bytes: manifest.original_bytes,
        original_sha256: manifest.original_sha256,
        final_bytes: manifest.final_bytes,
        final_sha256: manifest.final_sha256,
    })
}

/// Build a fresh [`ShardIndex`] for a mechanism-A backfill-split (EC-016's
/// zero-shard registration and the normal sealed-shard case alike) from
/// `entry`'s own cap-formula inputs, threading `retention_count` explicitly
/// (no pre-existing shard-index exists yet for an artifact that has never
/// been backfilled, so there is no `ShardIndex::retention_count` to reuse).
fn fresh_backfill_shard_index(
    entry: &ShardEntry,
    retention_count: u32,
    shards: Vec<ShardIndexEntry>,
) -> ShardIndex {
    ShardIndex {
        schema_version: 1,
        artifact_stem: entry.artifact_stem.clone(),
        current_shard: entry.artifact_path.clone(),
        shard_cap_bytes: entry.shard_cap_bytes,
        max_single_record_bytes: entry.max_single_record_bytes,
        safety_margin_bytes: entry.safety_margin,
        practical_fuel_ceiling: entry.practical_fuel_ceiling,
        worst_case_fuel_per_byte: entry.worst_case_fuel_per_byte,
        retention_count,
        shards,
    }
}

/// Serialize and atomically publish `index` at `index_path` -- the
/// backfill-split's own index-publish primitive, reusing `write_atomic`
/// (Invariant 1: caller of BC-1.18.006's atomic-write primitives, not a
/// reimplementation).
///
/// F-C3-P6-001 (BC-1.18.008 v1.6 Postcondition 3): when `backfill_manifest`
/// is `Some`, its own `[backfill_manifest]` TOML table ([`BackfillManifestWrapper`])
/// is appended to `index`'s own serialized text and the COMBINED text is
/// published in this ONE `write_atomic` call -- satisfying Postcondition
/// 3's "populated in the SAME atomic index-publish write as the `[[shard]]`
/// entries themselves" requirement exactly (one durable write, not two).
fn write_shard_index_for_backfill(
    index_path: &Path,
    index: &ShardIndex,
    artifact_stem: &str,
    backfill_manifest: Option<&BackfillManifest>,
) -> Result<(), MechanismABackfillError> {
    let to_err = |e: toml::ser::Error| MechanismABackfillError::Io {
        artifact_stem: artifact_stem.to_string(),
        source: io::Error::other(e.to_string()),
    };
    let mut serialized = toml::to_string(index).map_err(to_err)?;
    if let Some(manifest) = backfill_manifest {
        let manifest_toml = toml::to_string(&BackfillManifestWrapper {
            backfill_manifest: manifest,
        })
        .map_err(to_err)?;
        serialized.push('\n');
        serialized.push_str(&manifest_toml);
    }
    last_amended_migrate::atomic_write::write_atomic(index_path, &serialized).map_err(|e| {
        MechanismABackfillError::Io {
            artifact_stem: artifact_stem.to_string(),
            source: migrate_err_to_io(e),
        }
    })
}

/// Atomically write `content` (arbitrary bytes) to `path` via `write_atomic`
/// -- `write_atomic` itself is `&str`-typed (S-15.03 N2's permission-
/// preservation + fsync-durability primitive), so this validates `content`
/// is valid UTF-8 first, failing loud (never lossily substituting) if not:
/// a lossy conversion would silently violate Postcondition 6(a)'s
/// byte-for-byte content-preservation guarantee for exactly the corrupted
/// bytes it replaced.
fn write_atomic_bytes(
    path: &Path,
    content: &[u8],
    artifact_stem: &str,
) -> Result<(), MechanismABackfillError> {
    let text = std::str::from_utf8(content).map_err(|e| MechanismABackfillError::Io {
        artifact_stem: artifact_stem.to_string(),
        source: io::Error::new(io::ErrorKind::InvalidData, e.to_string()),
    })?;
    last_amended_migrate::atomic_write::write_atomic(path, text).map_err(|e| {
        MechanismABackfillError::Io {
            artifact_stem: artifact_stem.to_string(),
            source: migrate_err_to_io(e),
        }
    })
}

/// F-C3-P6-002 (S-25.02 F4 cluster-3 CROSS-VENDOR (OpenAI Codex)
/// adversarial pass-6 review, HIGH; BC-1.18.008 v1.6 Postcondition 6(c)'s
/// disk-read-back ruling, Invariant 4): writes `bytes` to `sealed_path` via
/// [`write_atomic_bytes`], then performs a POST-HOC READ-BACK of the
/// JUST-WRITTEN file from disk and verifies the read-back bytes are
/// byte-for-byte identical to `bytes` — the BC's own ruling text: "'actual
/// bytes written to disk' means a POST-HOC READ-BACK of each sealed shard
/// file from disk, via a FRESH file read performed AFTER that shard's
/// write completes, compared against the in-memory partition buffer that
/// was intended to be written." Checking only the in-memory buffer's own
/// length/content before issuing the write (the shipped behavior this
/// finding corrects) cannot detect a write that silently truncated,
/// partially flushed, or otherwise landed corrupted bytes on disk — exactly
/// the failure mode this gate exists to catch BEFORE the original
/// monolithic file is retired and its content becomes unrecoverable except
/// via git history. On a read-back mismatch, aborts fail-loud via
/// [`MechanismABackfillError::ContentPreservationFailed`] — the original
/// monolithic file is untouched and the operation is safely re-runnable
/// from scratch (Postcondition 5, EC-003/EC-004), consistent with
/// Postcondition 6's existing hard-gate discipline.
///
/// **The extracted fault-injection seam (`pub`, not a `#[cfg(test)]` hook):**
/// [`run_mechanism_a_backfill_split`]'s own sealed-shard write loop is one
/// synchronous call with no injectable I/O layer for a test to interpose
/// between a shard's write completing and this gate's own read-back — this
/// function is that seam, extracted as the write-then-read-back-then-verify
/// unit in isolation, addressable directly by a fault-injection test (e.g.
/// one that races a corrupting write against `sealed_path` between this
/// function's own `write_atomic_bytes` call and its read-back) without
/// needing to fabricate a full `run_mechanism_a_backfill_split` invocation.
/// A lower-level extracted function was chosen over a test-only injectable
/// callback parameter threaded through the whole call chain: it keeps
/// production call sites simple (`mechanism_a_write_and_verify_sealed_shard(path,
/// bytes, stem)?` reads identically to the `write_atomic_bytes` call it
/// replaces) and needs no `#[cfg(test)]`-gated parameter on a `pub` function
/// signature, while still giving a fault-injection test a single, real,
/// disk-level operation to drive independently.
pub fn mechanism_a_write_and_verify_sealed_shard(
    sealed_path: &Path,
    bytes: &[u8],
    artifact_stem: &str,
) -> Result<(), MechanismABackfillError> {
    let read_back = write_and_read_back(sealed_path, bytes, artifact_stem)?;

    if read_back != bytes {
        return Err(MechanismABackfillError::ContentPreservationFailed {
            artifact_stem: artifact_stem.to_string(),
            detail: format!(
                "post-hoc disk read-back of sealed shard '{}' ({} bytes) does not match the \
                 in-memory partition that was just written ({} bytes) -- the write silently \
                 truncated, partially flushed, or otherwise landed corrupted bytes on disk \
                 (Postcondition 6(c)/Invariant 4's F-C3-P6-002 disk-read-back ruling)",
                sealed_path.display(),
                read_back.len(),
                bytes.len()
            ),
        });
    }

    Ok(())
}

/// Shared low-level I/O primitive behind BOTH
/// [`mechanism_a_write_and_verify_sealed_shard`]'s sealed-shard write
/// (F-C3-P6-002) AND `heal_or_confirm_already_migrated`'s DANGEROUS-window
/// heal write (F-C3-P7-001's Postcondition 6(c)/Invariant 4 extension):
/// writes `bytes` to `path` via [`write_atomic_bytes`], then performs a
/// POST-HOC READ-BACK of the just-written file from disk, returning the
/// read-back bytes for the CALLER's own verification. The two call sites
/// verify against different "intended content" (a sealed shard's own
/// in-memory partition buffer for the former, the Backfill Recovery
/// Manifest's `final_bytes`/`final_sha256` pair for the latter) and report
/// DIFFERENT error codes on a mismatch (`ContentPreservationFailed` for
/// sealed shards, `SliceVerificationFailed` / `E-SHD-012` for the heal
/// write) -- so the comparison itself stays with each caller rather than
/// being baked into this shared write-then-read-back primitive.
fn write_and_read_back(
    path: &Path,
    bytes: &[u8],
    artifact_stem: &str,
) -> Result<Vec<u8>, MechanismABackfillError> {
    write_atomic_bytes(path, bytes, artifact_stem)?;

    std::fs::read(path).map_err(|source| MechanismABackfillError::Io {
        artifact_stem: artifact_stem.to_string(),
        source,
    })
}

// ---------------------------------------------------------------------------
// Tests — BC-1.18.005 (S-25.02 F4 BC-cluster 1 "cap+trigger")
// ---------------------------------------------------------------------------
//
// Every test below exercises a fully implemented production function (or the
// fully wired `shard_cap_gate_check` dispatch entry point) and is green. The
// two GREEN-BY-DESIGN/WIRING-EXEMPT helpers this file already implements
// (`ShardEntry::cap_formula_inputs`, `From<ShardConfigError> for HookResult`)
// are intentionally NOT covered here — they are trivial field-copy /
// delegation code, not part of this BC's tested trigger/formula logic (see
// their own doc comments), and are outside the enumerated test-writer
// dispatch surface for this cluster.
//
// Scope boundary this suite deliberately respects: `shard_cap_gate_check`
// NEVER returns `HookResult::Block` itself (BC-1.18.006/BC-1.18.009 own the
// observable roll/rotate outcome once a trigger fires — see this BC's
// Postcondition 3/8 "Ownership" bullets and the module's own "Scope note").
// No test below asserts an outcome for the trigger-FIRES branch at the
// `shard_cap_gate_check` level; that branch's observable behavior belongs to
// the later BC-1.18.006/BC-1.18.009 clusters. The trigger-fires DECISION
// itself is fully covered via the lower-level `size_trigger_fires` /
// `item_count_trigger_fires` functions, which this BC does own.
#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------
    // is_genuinely_missing (PR #824 pr-review Finding #1, BLOCKING on
    // Windows CI) — the cross-platform NotFound-vs-path-traversal
    // disambiguation every NotFound-relief site in this module now uses.
    // -----------------------------------------------------------------

    #[test]
    fn test_FINDING1_is_genuinely_missing_true_for_real_not_found() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing = dir.path().join("does-not-exist.md");
        let err = std::fs::read(&missing).expect_err("path must not exist");
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(
            is_genuinely_missing(&err, &missing),
            "a genuine 'file does not exist' error (real syscall, real raw_os_error) must be \
             treated as genuinely missing on every platform"
        );
    }

    #[test]
    fn test_FINDING1_is_genuinely_missing_false_for_non_not_found_kind() {
        let err = io::Error::new(io::ErrorKind::PermissionDenied, "denied");
        assert!(
            !is_genuinely_missing(&err, Path::new("irrelevant-to-this-case.md")),
            "any non-NotFound-kind error must never be treated as genuinely missing"
        );
    }

    /// S-25.02 cluster-2 (windows-x64 CI failure #1): traverse-through-a-file
    /// (`not_a_dir` exists as a plain file, so `child.md` cannot be resolved
    /// through it) is `io::ErrorKind::NotADirectory` on Unix but
    /// `io::ErrorKind::NotFound` (`ERROR_PATH_NOT_FOUND`) on Windows — the
    /// SAME kind a genuinely-missing ancestor produces there (see
    /// `is_genuinely_missing`'s doc comment). A prior revision of this test
    /// asserted `err.kind() != NotFound`, which is Unix-only and FAILS on
    /// Windows for this identical fixture. Per the portable-fix research
    /// (`.factory/code-delivery/S-25.02/windows-path-semantics-research.md`
    /// §"Test-fixture portability recommendation"), assert the SEMANTIC
    /// outcome instead — the error must propagate, never be relieved — which
    /// holds identically on both platforms with no `cfg` gating needed.
    #[test]
    fn test_FINDING1_is_genuinely_missing_false_for_path_traversal_through_non_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let not_a_dir = dir.path().join("plain-file");
        std::fs::write(&not_a_dir, "i am a file").expect("seed a plain file");
        let path = not_a_dir.join("child.md");
        let err = std::fs::read(&path).expect_err("reading through a non-directory must fail");
        assert!(
            !is_genuinely_missing(&err, &path),
            "a path-traversal-through-non-directory failure must never be treated as genuinely \
             missing"
        );
    }

    /// PR #824 pr-review Finding #1's core claim, verified directly against
    /// the disambiguation logic rather than a live Windows syscall (not
    /// available on this CI runner): Windows maps BOTH
    /// `ERROR_FILE_NOT_FOUND` (2, genuinely missing) and
    /// `ERROR_PATH_NOT_FOUND` (3, itself ambiguous — see N-2 below) onto the
    /// SAME `io::ErrorKind::NotFound`. (S-25.02 cluster-2: the raw code
    /// itself is no longer what `is_genuinely_missing` branches on — only
    /// `err.kind()` matters now, so this synthetic `raw_os_error(2)` fixture
    /// exercises the SAME `NotFound` path a real `ERROR_FILE_NOT_FOUND`
    /// would; the ancestor-walk correctly relieves it because the given
    /// path's parent is empty/non-blocking.)
    #[cfg(windows)]
    #[test]
    fn test_FINDING1_is_genuinely_missing_windows_file_not_found_relieved() {
        let err = io::Error::from_raw_os_error(2); // ERROR_FILE_NOT_FOUND
        assert_eq!(err.kind(), io::ErrorKind::NotFound);
        assert!(
            is_genuinely_missing(&err, Path::new("irrelevant-for-error-code-2.md")),
            "Windows ERROR_FILE_NOT_FOUND (2) is the genuine missing-file case and must relieve \
             to Ok regardless of path"
        );
    }

    /// N-2 (PR #824 pr-review cycle 2, MINOR) keeps this cycle-1 Finding #1
    /// test green under the now-`path`-aware disambiguation: a path whose
    /// immediate parent EXISTS but is a plain file (not a directory) is
    /// still a genuine path-traversal failure and must still propagate,
    /// never be relieved. (S-25.02 cluster-2: this fixture's real ancestor —
    /// a plain file — is what makes the ancestor walk return `false` here;
    /// the raw code 3 used to synthesize the `NotFound` kind is otherwise
    /// irrelevant to the now-portable algorithm.)
    #[cfg(windows)]
    #[test]
    fn test_FINDING1_is_genuinely_missing_windows_path_not_found_propagates_through_non_directory()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let not_a_dir = dir.path().join("plain-file");
        std::fs::write(&not_a_dir, "i am a file").expect("seed a plain file");
        let path = not_a_dir.join("child.md");

        let err = io::Error::from_raw_os_error(3); // ERROR_PATH_NOT_FOUND
        assert_eq!(
            err.kind(),
            io::ErrorKind::NotFound,
            "sanity: Windows ERROR_PATH_NOT_FOUND (3) maps to the SAME io::ErrorKind::NotFound \
             as ERROR_FILE_NOT_FOUND (2) — this is exactly the ambiguity Finding #1 identifies"
        );
        assert!(
            !is_genuinely_missing(&err, &path),
            "Windows ERROR_PATH_NOT_FOUND (3) for a path whose immediate parent EXISTS but is a \
             plain file (not a directory) is a genuine path-traversal failure — it must \
             propagate as a real Err, never be silently relieved to Ok"
        );
    }

    /// N-2 (PR #824 pr-review cycle 2, MINOR): the cycle-1 fix over-corrected
    /// `ERROR_PATH_NOT_FOUND` (3) into ALWAYS propagating, which also
    /// blocked a legitimate first Write into a freshly-bootstrapped
    /// directory (e.g. the first `burst-log.md` write into a just-created
    /// cycle directory) — a case Windows raises the SAME raw code 3 for.
    /// This must relieve to `Ok`, exactly like `ERROR_FILE_NOT_FOUND` (2)
    /// already does. (S-25.02 cluster-2: the ancestor walk relieves this
    /// because the closest EXISTING ancestor — the tempdir root — is a real
    /// directory; the raw code is incidental to a `NotFound`-kind error.)
    #[cfg(windows)]
    #[test]
    fn test_N2_is_genuinely_missing_windows_path_not_found_relieved_for_missing_parent_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing_parent = dir.path().join("freshly-bootstrapped-cycle-dir");
        let path = missing_parent.join("burst-log.md");
        assert!(
            !missing_parent.exists(),
            "precondition: the parent directory must genuinely not exist yet"
        );

        let err = io::Error::from_raw_os_error(3); // ERROR_PATH_NOT_FOUND
        assert!(
            is_genuinely_missing(&err, &path),
            "N-2: ERROR_PATH_NOT_FOUND must be relieved to Ok when the reason is a legitimately \
             not-yet-created ancestor directory — over-correcting this (cycle-1's own \
             regression) would block a valid first Write into a brand-new directory on Windows"
        );
    }

    // -----------------------------------------------------------------
    // closest_existing_ancestor_is_directory_or_absent (N-2, PR #824
    // pr-review cycle 2) — the path-aware helper is unconditionally
    // compiled on every platform (no `#[cfg]` gate at all, per the cycle-6
    // portable ancestor-walk rewrite), so its logic is directly, portably
    // unit-testable here without a live Windows runner.
    // -----------------------------------------------------------------

    #[test]
    fn test_N2_closest_existing_ancestor_relieves_missing_parent_directory() {
        let dir = tempfile::tempdir().expect("tempdir");
        let missing_parent = dir.path().join("freshly-bootstrapped-cycle-dir");
        let path = missing_parent.join("burst-log.md");
        assert!(
            !missing_parent.exists(),
            "precondition: the parent directory must genuinely not exist yet"
        );
        assert!(
            closest_existing_ancestor_is_directory_or_absent(&path),
            "N-2: a legitimately not-yet-created parent directory must be treated as \
             relievable — the closest EXISTING ancestor (the tempdir root itself) IS a real \
             directory, so nothing blocks traversal, it just hasn't been created yet"
        );
    }

    #[test]
    fn test_N2_closest_existing_ancestor_relieved_when_whole_chain_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("a").join("b").join("c.md");
        assert!(
            closest_existing_ancestor_is_directory_or_absent(&path),
            "N-2: when NO ancestor above `path` exists yet (a multi-level-deep fresh directory \
             tree), the closest existing ancestor is the tempdir root itself, a real directory \
             — must relieve"
        );
    }

    #[test]
    fn test_N2_closest_existing_ancestor_propagates_when_blocked_by_a_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        let not_a_dir = dir.path().join("plain-file");
        std::fs::write(&not_a_dir, "i am a file").expect("seed a plain file");
        let path = not_a_dir.join("nested").join("child.md");
        assert!(
            !closest_existing_ancestor_is_directory_or_absent(&path),
            "N-2: when the closest EXISTING ancestor is a plain file (not a directory), \
             traversal is genuinely blocked and must propagate as a real error — even though \
             the IMMEDIATE parent ('nested') does not itself exist, a naive parent-only check \
             would misjudge this as relievable"
        );
    }

    /// NIT-B (PR #824 pr-review cycle 6): a non-`NotFound` ancestor
    /// `metadata` error (e.g. `PermissionDenied`) must STOP the walk and
    /// propagate (`false`), not be conflated with genuine absence.
    ///
    /// Deterministic fixture (`#[cfg(unix)]`, same unprivileged-permission-
    /// bit precedent already established in this workspace — e.g.
    /// `internal_log.rs`'s `silently_swallows_errors_on_read_only_dir`):
    /// a directory `blocked/` is chmod'd to `0o000` (no search/execute
    /// bit). Under POSIX, resolving ANY name nested inside a directory
    /// with no search permission fails with `EACCES`
    /// (`io::ErrorKind::PermissionDenied`) regardless of whether that name
    /// actually exists — so `metadata(blocked/inner)` fails with
    /// `PermissionDenied`, never `NotFound`, isolating exactly the arm
    /// NIT-B hardens. The precondition assertion below fails loud (rather
    /// than silently mis-testing) if this workspace's CI ever runs this
    /// suite as a privileged user under which permission bits are not
    /// enforced.
    #[cfg(unix)]
    #[test]
    fn test_NIT_B_closest_existing_ancestor_propagates_non_notfound_ancestor_error() {
        use std::os::unix::fs::PermissionsExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let blocked = dir.path().join("blocked");
        std::fs::create_dir(&blocked).expect("create the to-be-locked-down directory");

        let mut perms = std::fs::metadata(&blocked)
            .expect("stat blocked dir before chmod")
            .permissions();
        perms.set_mode(0o000);
        std::fs::set_permissions(&blocked, perms).expect("chmod blocked dir to 0o000");

        // Always restore permissions before returning (success, assertion
        // failure, or panic) so the tempdir's own Drop cleanup can actually
        // remove the directory tree — a leaked 0o000 directory would fail
        // a subsequent `remove_dir_all` with the very same EACCES.
        struct RestorePermsOnDrop(std::path::PathBuf);
        impl Drop for RestorePermsOnDrop {
            fn drop(&mut self) {
                if let Ok(meta) = std::fs::metadata(&self.0) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o755);
                    let _ = std::fs::set_permissions(&self.0, perms);
                }
            }
        }
        let _restore_guard = RestorePermsOnDrop(blocked.clone());

        let path = blocked.join("inner").join("child.md");

        // Precondition: confirm the fixture actually produces a genuine
        // non-NotFound ancestor error before asserting on the helper under
        // test — if this fails, the fixture (or the CI privilege level)
        // is not exercising NIT-B's branch at all.
        let probe_err = std::fs::metadata(blocked.join("inner")).expect_err(
            "precondition: metadata() on a name nested inside a 0o000-permission directory \
             must fail — if it succeeded, this test is running with elevated privileges that \
             bypass permission bits and cannot exercise NIT-B's branch",
        );
        assert_eq!(
            probe_err.kind(),
            io::ErrorKind::PermissionDenied,
            "precondition: the fixture must produce PermissionDenied specifically (not \
             NotFound) — got {:?}",
            probe_err.kind()
        );

        assert!(
            !closest_existing_ancestor_is_directory_or_absent(&path),
            "NIT-B: a non-NotFound ancestor metadata error (PermissionDenied) must STOP the \
             walk and propagate (return false) — treating it as genuine absence would let a \
             real permission failure during the walk be silently relieved as a legitimate \
             not-yet-created path"
        );
    }

    // -----------------------------------------------------------------
    // Test fixture helpers
    // -----------------------------------------------------------------

    /// A well-formed `"flat"`-shaped entry using the BC's own provisional
    /// calibration constants (Postcondition 6), parameterized only on
    /// `artifact_stem` and `shard_cap_bytes` for per-test readability.
    ///
    /// `artifact_path` defaults to `"{stem}.md"` (PR #818 cycle-2 review
    /// finding B-1) — every existing call site in this module pairs a given
    /// stem with a target path whose final component is exactly
    /// `"{stem}.md"` (a bare filename, a `dir.path().join("{stem}.md")`
    /// tempdir path, or a `"/repo/.../{stem}.md"` literal), so this single-
    /// component default suffix-matches every one of them unchanged (see
    /// [`path_falls_under_or_equals`]) without requiring a signature change
    /// across this file's dozens of call sites. A test that specifically
    /// exercises path-containment (as opposed to merely needing SOME valid
    /// entry) overrides `.artifact_path` explicitly, the same way other
    /// tests already override `.worst_case_fuel_per_byte` etc. post-construction.
    fn flat_entry(stem: &str, shard_cap_bytes: u64) -> ShardEntry {
        ShardEntry {
            artifact_stem: stem.to_string(),
            artifact_path: format!("{stem}.md"),
            practical_fuel_ceiling: 8_000_000,
            worst_case_fuel_per_byte: 106.36,
            max_single_record_bytes: 16_384,
            safety_margin: 8_192,
            shard_cap_bytes,
            shape: Some(ShardShape::Flat),
            n: None,
            low_water_mark: None,
        }
    }

    /// Renders a single `[[shard]]` TOML entry for the
    /// `"frontmatter-changelog-array"` shape, with an optional
    /// `low_water_mark` line (omitted entirely when `None`, exercising
    /// EC-010's config-load-time default path). `artifact_path` defaults to
    /// `"{stem}.md"` — see [`flat_entry`]'s doc comment for the same
    /// rationale (both call sites of this helper only exercise
    /// `ShardRegistry::load`'s structural parse, never path-containment
    /// matching, so the default is never overridden here).
    fn shard_toml_frontmatter_entry(stem: &str, n: u64, low_water_mark: Option<i64>) -> String {
        let lwm_line = match low_water_mark {
            Some(v) => format!("low_water_mark = {v}\n"),
            None => String::new(),
        };
        format!(
            "[[shard]]\n\
             artifact_stem = \"{stem}\"\n\
             artifact_path = \"{stem}.md\"\n\
             practical_fuel_ceiling = 8000000\n\
             worst_case_fuel_per_byte = 106.36\n\
             max_single_record_bytes = 16384\n\
             safety_margin = 8192\n\
             shard_cap_bytes = 49152\n\
             shape = \"frontmatter-changelog-array\"\n\
             n = {n}\n\
             {lwm_line}"
        )
    }

    /// Minimal `tracing::Subscriber` that counts WARN-level events. Used to
    /// assert the EC-012 non-fatal `tracing::warn!` amortization advisory
    /// fires exactly when VP-140's biconditional requires
    /// (`low_water_mark > floor(N/2)`), without pulling in a
    /// `tracing-subscriber` dev-dependency.
    struct WarnCapture {
        warn_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl tracing::Subscriber for WarnCapture {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.warn_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    /// Serializes every test in this module that can execute the
    /// `tracing::warn!` callsite inside `validate_entry` (BC-1.18.005
    /// EC-012's amortization advisory) — i.e. every test that drives
    /// `validate_entry` on a `"frontmatter-changelog-array"`-shaped fixture,
    /// whether or not that particular test's fixture actually crosses the
    /// `low_water_mark > floor(N/2)` threshold that fires the warn (a test
    /// asserting the NON-firing half of VP-140's biconditional still
    /// touches the callsite — it just expects `Interest` to resolve to
    /// "no warn recorded" for ITS fixture, which is exactly the resolution
    /// this discipline protects against being poisoned by callsite-wide
    /// caching).
    ///
    /// `tracing-core`'s per-callsite `Interest` cache
    /// (`tracing_core::callsite`) is a single table shared by the whole
    /// **process**, keyed by source-location callsite — not by which
    /// `Subscriber` happens to be installed on a given thread. The *first*
    /// time this specific callsite is ever executed anywhere in this test
    /// binary, `tracing-core` performs a one-time `Interest` registration
    /// using whatever `Subscriber` is ambient **on the thread that gets
    /// there first**, and cheaply caches that verdict forever after
    /// (`DefaultCallsite`'s registration state machine never revisits a
    /// callsite once it flips from `UNREGISTERED` to `REGISTERED`). Three
    /// tests reach this exact callsite via `validate_entry`, all under
    /// `count_warns`:
    /// `test_BC_1_18_005_EC_012_validate_entry_emits_warn_advisory_when_low_water_mark_exceeds_default`
    /// (fixture `low_water_mark=49`, `n=50` — fires),
    /// `test_BC_1_18_005_VP_140_validate_entry_default_low_water_mark_does_not_emit_warn_advisory`
    /// (fixture `low_water_mark=25`, `n=50` — does not fire), and
    /// `test_BC_1_18_005_EC_010_validate_entry_omitted_low_water_mark_does_not_emit_warn_advisory`
    /// (fixture `low_water_mark=None`, `n=50`, resolving to `floor(N/2)=25`
    /// — does not fire). Confirmed by instrumenting `WarnCapture` and
    /// re-running under `--test-threads>1`: if the FIRST of these tests to
    /// run does not install a `WarnCapture` default before calling
    /// `validate_entry`, the ambient subscriber on its thread is the no-op
    /// default (never interested), so `Interest::never()` gets cached for
    /// the callsite **permanently** — and a later
    /// `tracing::subscriber::with_default` call does not, by itself,
    /// invalidate that cached decision: `with_default`/`set_default` only
    /// swap the thread-local dispatch, they do not call
    /// `tracing::callsite::rebuild_interest_cache()`. Whichever of these
    /// tests the `cargo test` thread pool happens to schedule first
    /// therefore decides, non-deterministically, whether the warn is ever
    /// observable again in this process — this is exactly the intermittent
    /// (~2/5) failure this fixes.
    ///
    /// The fix requires EVERY test that reaches this callsite to be under
    /// the SAME discipline (see the now-locked call sites above); a
    /// serialized capture-only helper does not, by itself, close the race
    /// against an unsynchronized non-capturing sibling. The discipline has
    /// two parts:
    ///
    /// 1. This `Mutex` ensures at most one such call is ever "in flight"
    ///    at once, so no two of these calls can race each other's
    ///    `Interest` recomputation or the callsite's one-time
    ///    registration.
    /// 2. `count_warns` calls `tracing::callsite::rebuild_interest_cache()`
    ///    *after* installing its `WarnCapture` subscriber as the thread's
    ///    default and *immediately before* invoking the code under test.
    ///    This forces `tracing-core` to unconditionally recompute the
    ///    warn! callsite's `Interest` against the currently-installed
    ///    (always-`enabled`) `WarnCapture` subscriber, overwriting any
    ///    `Interest::never()` left behind by an earlier unsynchronized
    ///    touch — regardless of registration history — and, when this is
    ///    the callsite's very first-ever touch, guarantees the ambient
    ///    dispatch at the moment of registration is this call's own
    ///    `WarnCapture` (same thread, same lock-protected scope), rather
    ///    than whatever happened to be ambient on some other thread.
    ///
    /// Since every reachable call site now goes through `count_warns`
    /// under the same lock, and every `Subscriber` ever installed here is
    /// a `WarnCapture` (always `enabled`), the callsite cannot be
    /// re-poisoned after that: any later recomputation triggered by
    /// another call's own `Dispatch::new` (see `tracing_core::callsite`'s
    /// "Rebuilding Cached Interest" docs) can only ever agree that the
    /// callsite is `Interest::always()`.
    static CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    // Generic over the closure's return type `T` (BC-1.18.005 v1.12
    // MATCH-FIRST restructure): the EC-012 amortization-advisory warn fires
    // solely from `validate_entry` (returning `Result<(), ShardConfigError>`)
    // post-restructure — `ShardRegistry::load` is structural-TOML-parse-only
    // and no longer reaches this callsite under any input. This helper stays
    // generic over `T` so any future closure shape (a direct
    // `validate_entry` call today) can route through it unchanged without a
    // signature change.
    fn count_warns<T>(f: impl FnOnce() -> T) -> (usize, T) {
        // A prior capture test panicking mid-assertion (lock still held via
        // its guard's unwind drop) must not cascade into every subsequent
        // capture test failing on a poisoned-lock panic; the poisoning
        // carries no information about *this* test's fixture, so recover
        // and proceed.
        let _guard = CAPTURE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let warn_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let subscriber = WarnCapture {
            warn_count: warn_count.clone(),
        };
        let result = tracing::subscriber::with_default(subscriber, || {
            // See CAPTURE_LOCK's doc comment: force tracing-core to
            // recompute this callsite's cached `Interest` against the
            // `WarnCapture` subscriber just installed above, closing the
            // race against any non-capturing test that touched the same
            // `warn!` callsite first (and thus cached `Interest::never()`
            // against the ambient no-op default).
            tracing::callsite::rebuild_interest_cache();
            f()
        });
        (warn_count.load(std::sync::atomic::Ordering::SeqCst), result)
    }

    // ===================================================================
    // Postcondition 1 / Invariant 3 / EC-001 — zero-cost bypass
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_PC1_find_matching_entry_returns_entry_for_matching_stem() {
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let target = Path::new("/repo/.factory/cycles/pass-1/decision-log.md");
        let result = find_matching_entry(&registry, target)
            .expect("fixture has no duplicate artifact_stem entries");
        assert_eq!(
            result,
            Some(&registry.shards[0]),
            "PC1: a target path whose stem matches a [[shard]] entry's artifact_stem MUST resolve to that entry"
        );
    }

    #[test]
    fn test_BC_1_18_005_AC_001_PC1_EC_001_find_matching_entry_returns_none_for_unmatched_path() {
        let registry = ShardRegistry {
            shards: vec![
                flat_entry("decision-log", 49_152),
                flat_entry("lessons", 49_152),
            ],
        };
        let target = Path::new("/repo/some/unrelated/file.md");
        let result = find_matching_entry(&registry, target)
            .expect("fixture has no duplicate artifact_stem entries");
        assert_eq!(
            result, None,
            "EC-001: a target path matching no [[shard]] entry's artifact_stem MUST return None (zero-cost bypass)"
        );
    }

    // ===================================================================
    // PR #818 cycle-2 review finding B-1 — stem-only matching has no path
    // containment. A [[shard]] entry MUST match on artifact_stem AND
    // artifact_path (path_falls_under_or_equals), never stem alone — this
    // repository alone measured 426 files sharing the STATE stem, 99
    // sharing lessons, 98 sharing burst-log, 79 sharing BC-INDEX, and 32
    // sharing decision-log, mostly under
    // plugins/vsdd-factory/tests/fixtures/.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_B1_find_matching_entry_same_stem_different_directory_does_not_match() {
        // Entry registered for "/registered/dir/decision-log.md"; the
        // dispatch's target shares the SAME stem ("decision-log") but lives
        // under a COMPLETELY DIFFERENT directory — e.g. one of this repo's
        // own 32 unrelated "decision-log" stem-collision fixtures. Stem-only
        // matching (pre-B-1) would have routed this unrelated file through
        // the registered entry's cap gate; path-containment MUST reject it.
        let mut entry = flat_entry("decision-log", 49_152);
        entry.artifact_path = "/registered/dir/decision-log.md".to_string();
        let registry = ShardRegistry {
            shards: vec![entry],
        };
        let target = Path::new("/some/unrelated/fixtures/decision-log.md");

        let result = find_matching_entry(&registry, target)
            .expect("fixture has no duplicate artifact_stem+artifact_path entries");
        assert_eq!(
            result, None,
            "B-1: a target whose STEM matches a [[shard]] entry but whose PATH does not fall \
             under or equal the entry's own artifact_path MUST NOT match — stem alone is not a \
             sufficient config-match predicate (this repository alone has 32 unrelated files \
             sharing the \"decision-log\" stem)"
        );
    }

    #[test]
    fn test_BC_1_18_005_B1_find_matching_entry_same_stem_and_correct_path_matches() {
        // Same entry as above, but this time the target's FULL path (not
        // just its stem) matches the registered artifact_path exactly — the
        // positive control proving the path leg does not merely reject
        // everything.
        let mut entry = flat_entry("decision-log", 49_152);
        entry.artifact_path = "/registered/dir/decision-log.md".to_string();
        let registry = ShardRegistry {
            shards: vec![entry],
        };
        let target = Path::new("/registered/dir/decision-log.md");

        let result = find_matching_entry(&registry, target)
            .expect("fixture has no duplicate artifact_stem+artifact_path entries");
        assert_eq!(
            result,
            Some(&registry.shards[0]),
            "B-1: a target whose stem AND path both match the registered entry MUST resolve to \
             that entry"
        );
    }

    #[test]
    fn test_BC_1_18_005_B1_find_matching_entry_target_nested_under_registered_directory_matches() {
        // `artifact_path` naming a CONTAINING DIRECTORY (rather than the
        // full file path) is also legal per path_falls_under_or_equals's
        // "falls under" leg — any target nested under that directory
        // matches.
        let mut entry = flat_entry("decision-log", 49_152);
        entry.artifact_path = "/registered/dir".to_string();
        let registry = ShardRegistry {
            shards: vec![entry],
        };
        let target = Path::new("/registered/dir/decision-log.md");

        let result = find_matching_entry(&registry, target)
            .expect("fixture has no duplicate artifact_stem+artifact_path entries");
        assert_eq!(
            result,
            Some(&registry.shards[0]),
            "B-1: a target nested under the entry's registered containing directory MUST match \
             (the \"falls under\" leg of path_falls_under_or_equals)"
        );
    }

    #[test]
    fn test_BC_1_18_005_B1_shard_cap_gate_check_same_stem_different_directory_continues() {
        // Full-stack control: the same collision scenario, but driven
        // through the public shard_cap_gate_check gate — an unrelated,
        // differently-directoried file sharing a registered artifact's stem
        // MUST Continue, never be routed through that entry's cap check.
        let mut entry = flat_entry("decision-log", 100);
        entry.artifact_path = "/registered/dir/decision-log.md".to_string();
        let registry = ShardRegistry {
            shards: vec![entry],
        };
        let target = Path::new("/some/unrelated/fixtures/decision-log.md");

        let result = shard_cap_gate_check(
            &registry,
            "Write",
            target,
            // Deliberately oversized relative to the registered entry's
            // shard_cap_bytes=100 — if this fixture's cap gate incorrectly
            // applied, it would still Continue (this cluster never
            // constructs Block), so pair this with EC-018-style asserted
            // Continue plus the find_matching_entry-level test above, which
            // pins the STRUCTURAL non-match directly.
            &serde_json::json!({"content": "x".repeat(1_000)}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "B-1: a Write against a file sharing a registered entry's STEM but not its PATH \
             MUST Continue, exactly as an entirely-unregistered file would"
        );
    }

    #[test]
    fn test_BC_1_18_005_B1_path_falls_under_or_equals_curdir_prefix_matches_same_as_without() {
        // PR #818 fix-burst finding S-2: `Path::components()` retains a
        // leading `CurDir` (`.`) component when `artifact_path` is written
        // with an explicit `./` prefix (the natural, idiomatic way an
        // operator would write a repo-root-relative path), but drops it
        // when written without one. Both spellings name the identical
        // location and MUST match the same targets identically — an
        // operator who writes the `./`-prefixed form must not get an entry
        // that silently matches nothing (leaving the artifact unguarded
        // with no diagnostic).
        let target = Path::new("/repo/.factory/decision-log.md");
        let with_curdir_prefix = Path::new("./.factory/decision-log.md");
        let without_curdir_prefix = Path::new(".factory/decision-log.md");

        assert_eq!(
            path_falls_under_or_equals(target, with_curdir_prefix),
            path_falls_under_or_equals(target, without_curdir_prefix),
            "S-2: a `./`-prefixed artifact_path MUST match exactly the same targets as the same \
             path written without the `./` prefix — CurDir components must be normalized out of \
             both operands before component-wise comparison"
        );
        assert!(
            path_falls_under_or_equals(target, with_curdir_prefix),
            "S-2: the `./`-prefixed form must actually match (not just match-equal to a \
             not-matching baseline) — this is the operator-facing failure mode: a `./`-prefixed \
             artifact_path silently matching nothing would leave the artifact completely \
             unguarded"
        );
    }

    /// SEC-003 (security review, LOW, CWE-22): a `target_path` containing a
    /// literal `..` (`ParentDir`) component must NOT match against a
    /// `registered_path` it would otherwise lexically appear to fall
    /// under — `path_falls_under_or_equals` never canonicalizes, so a
    /// naive component-wise comparison could otherwise be fooled by an
    /// unnormalized `..` segment into a false-positive containment match.
    #[test]
    fn test_SEC003_path_falls_under_or_equals_rejects_parent_dir_component_in_target() {
        // Lexically, stripping CurDir (none present here) leaves
        // ["a", "b", "..", "c"] for the target and ["c"] for the
        // registered path — a naive suffix-match would see the target's
        // trailing component ("c") equal the registered path's sole
        // component and report a match, even though "a/b/../c" does not
        // actually fall under "c" in any real filesystem sense.
        let target = Path::new("a/b/../c");
        let registered = Path::new("c");

        assert!(
            !path_falls_under_or_equals(target, registered),
            "SEC-003: a target_path containing a literal `..` segment MUST NOT match against a \
             registered path it would otherwise lexically appear to fall under"
        );
    }

    #[test]
    fn test_SEC003_path_falls_under_or_equals_rejects_parent_dir_component_in_registered() {
        // Symmetric case: a `..` component in the REGISTERED side must
        // also be rejected outright, never participate in the lexical
        // comparison.
        let target = Path::new("/repo/.factory/decision-log.md");
        let registered = Path::new("../.factory/decision-log.md");

        assert!(
            !path_falls_under_or_equals(target, registered),
            "SEC-003: a registered_path containing a literal `..` segment MUST NOT be allowed to \
             match any target via the lexical comparison"
        );
    }

    /// PR #818 cycle-4 review finding F-1: demonstrates the RAW mechanism the
    /// EC-022 validation guards against, directly at the
    /// `path_falls_under_or_equals` layer (below `validate_entry`). Once
    /// `CurDir` is filtered out of a `"."`-registered path, its component
    /// vector is empty, and the suffix-match leg
    /// (`target[target.len()-0..] == []`) is vacuously true for ANY
    /// target — this is exactly why EC-022 must reject such an
    /// `artifact_path` at entry-match time rather than let it reach this
    /// function at all.
    #[test]
    fn test_BC_1_18_005_F1_path_falls_under_or_equals_dot_artifact_path_is_vacuously_true() {
        let registered = Path::new(".");
        let unrelated_target = Path::new("/completely/unrelated/path/some-other-file.md");

        assert!(
            path_falls_under_or_equals(unrelated_target, registered),
            "F-1: this assertion documents the vacuous-match MECHANISM itself — an empty \
             (post-CurDir-filter) registered component vector suffix-matches EVERY target. \
             This is precisely the raw behavior EC-022's validate_entry check exists to make \
             unreachable via config (see the sibling F-1 validate_entry/shard_cap_gate_check \
             tests below), never a claim that this is safe or desired on its own."
        );
    }

    #[test]
    fn test_BC_1_18_005_F1_validate_entry_rejects_dot_artifact_path() {
        // PR #818 cycle-4 review finding F-1: artifact_path = "." normalizes
        // to zero non-CurDir components, which would make
        // path_falls_under_or_equals vacuously match every target sharing
        // this entry's stem. validate_entry MUST reject this at
        // entry-match time (EC-022), never let it reach the match logic.
        let mut entry = flat_entry("decision-log", 40_000);
        entry.artifact_path = ".".to_string();

        let err = validate_entry(&entry)
            .expect_err("F-1: an artifact_path of \".\" MUST be rejected (EC-022)");
        match err {
            ShardConfigError::EmptyArtifactPath {
                artifact_stem,
                artifact_path,
            } => {
                assert_eq!(artifact_stem, "decision-log");
                assert_eq!(artifact_path, ".");
            }
            other => panic!("expected EmptyArtifactPath, got {other:?}"),
        }
    }

    #[test]
    fn test_BC_1_18_005_F1_validate_entry_rejects_dot_slash_artifact_path() {
        // F-1: "./" is the same degenerate case as "." — a single CurDir
        // component and nothing else.
        let mut entry = flat_entry("lessons", 40_000);
        entry.artifact_path = "./".to_string();

        let err = validate_entry(&entry)
            .expect_err("F-1: an artifact_path of \"./\" MUST be rejected (EC-022)");
        assert!(
            matches!(err, ShardConfigError::EmptyArtifactPath { .. }),
            "expected EmptyArtifactPath, got {err:?}"
        );
    }

    #[test]
    fn test_BC_1_18_005_F1_validate_entry_rejects_empty_artifact_path() {
        // F-1: a literally empty string also normalizes to zero components.
        let mut entry = flat_entry("burst-log", 40_000);
        entry.artifact_path = String::new();

        let err = validate_entry(&entry)
            .expect_err("F-1: an empty-string artifact_path MUST be rejected (EC-022)");
        assert!(
            matches!(err, ShardConfigError::EmptyArtifactPath { .. }),
            "expected EmptyArtifactPath, got {err:?}"
        );
    }

    #[test]
    fn test_BC_1_18_005_F1_validate_entry_accepts_normal_nonempty_artifact_path() {
        // Non-regression: a normal, non-empty artifact_path (the common
        // case exercised throughout this test module via flat_entry's own
        // default `"{stem}.md"`) MUST still validate successfully — EC-022
        // must not reject legitimate configs.
        let entry = flat_entry("decision-log", 40_000);
        validate_entry(&entry)
            .expect("a normal artifact_path like \"decision-log.md\" MUST pass validate_entry");
    }

    #[test]
    fn test_BC_1_18_005_F1_validate_entry_accepts_curdir_prefixed_artifact_path() {
        // Non-regression against the B-1/S-2 CurDir-normalization fix: a
        // "./"-prefixed but otherwise non-empty artifact_path (e.g.
        // "./.factory/decision-log.md") still has a non-empty component
        // vector after CurDir filtering, so it MUST still pass — EC-022
        // only rejects an artifact_path that filters down to NOTHING.
        let mut entry = flat_entry("decision-log", 40_000);
        entry.artifact_path = "./.factory/decision-log.md".to_string();
        validate_entry(&entry).expect(
            "a \"./\"-prefixed non-empty artifact_path MUST still pass validate_entry \
             (S-2 CurDir normalization is orthogonal to EC-022's emptiness check)",
        );
    }

    /// SEC-002 (security review, MEDIUM, CWE-22): `validate_entry` MUST
    /// reject an `artifact_stem` containing a `/`, `\`, or a `..`
    /// path-traversal component — table-driven over the three forbidden
    /// forms the security review names explicitly.
    #[test]
    fn test_SEC002_validate_entry_rejects_path_traversal_artifact_stem() {
        let cases: &[(&str, &str)] = &[
            ("forward slash", "../.factory/decision-log"),
            ("embedded forward slash", "sub/decision-log"),
            ("backslash", "sub\\decision-log"),
            ("bare parent-dir traversal", ".."),
        ];

        for (label, unsafe_stem) in cases {
            let mut entry = flat_entry("decision-log", 40_000);
            entry.artifact_stem = unsafe_stem.to_string();

            let err = validate_entry(&entry).expect_err(&format!(
                "SEC-002 [{label}]: artifact_stem = {unsafe_stem:?} MUST be rejected by \
                 validate_entry"
            ));
            match err {
                ShardConfigError::InvalidArtifactStem { artifact_stem } => {
                    assert_eq!(
                        &artifact_stem, unsafe_stem,
                        "SEC-002 [{label}]: InvalidArtifactStem must echo back the offending \
                         artifact_stem verbatim"
                    );
                }
                other => panic!(
                    "SEC-002 [{label}]: expected ShardConfigError::InvalidArtifactStem — got \
                     {other:?}"
                ),
            }
        }
    }

    #[test]
    fn test_SEC002_validate_entry_accepts_normal_artifact_stem() {
        // Non-regression: a normal artifact_stem (no separators, no `..`)
        // MUST still pass validate_entry.
        let entry = flat_entry("decision-log", 40_000);
        validate_entry(&entry)
            .expect("a normal artifact_stem like \"decision-log\" MUST pass validate_entry");
    }

    #[test]
    fn test_BC_1_18_005_F1_shard_cap_gate_check_surfaces_empty_artifact_path_as_error() {
        // Full-stack: a "." artifact_path MUST surface as HookResult::Error
        // through the public shard_cap_gate_check entry point — exactly
        // like any other match-time config defect (EC-009/EC-011/etc.) —
        // never silently applying this entry's cap to an unrelated file
        // that merely shares its artifact_stem, and never a bare Continue
        // that would leave the misconfiguration undiagnosed.
        let mut entry = flat_entry("decision-log", 100);
        entry.artifact_path = ".".to_string();
        let registry = ShardRegistry {
            shards: vec![entry],
        };
        // An entirely unrelated file that merely shares the registered
        // artifact_stem — under the pre-fix vacuous-match bug this would
        // have silently resolved to the "." entry and applied its cap.
        let target = Path::new("/some/totally/unrelated/fixtures/decision-log.md");

        let result = shard_cap_gate_check(
            &registry,
            "Write",
            target,
            &serde_json::json!({"content": "x".repeat(10)}),
        );
        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("EC-022") && message.contains("decision-log"),
                    "F-1: the surfaced HookResult::Error MUST name EC-022 and the offending \
                     artifact_stem, got: {message}"
                );
            }
            other => panic!(
                "F-1: expected HookResult::Error for a \".\"-artifact_path entry, got {other:?} \
                 — a bare Continue would mean the vacuous-match bug is still reachable, and a \
                 Block would be this module's own scope violation"
            ),
        }
    }

    #[test]
    fn test_BC_1_18_005_N2_find_matching_entry_rejects_duplicate_artifact_stem() {
        // PR #818 fix-burst finding N-2: two [[shard]] entries sharing the
        // same artifact_stem MUST fail loud, never silently resolve to
        // "whichever appears first."
        let registry = ShardRegistry {
            shards: vec![
                flat_entry("decision-log", 40_000),
                flat_entry("decision-log", 49_152),
            ],
        };
        let target = Path::new("/repo/.factory/decision-log.md");
        let err = find_matching_entry(&registry, target).expect_err(
            "N-2: a target path whose stem matches TWO [[shard]] entries MUST fail loud, never \
             silently resolve to the first-declared entry",
        );
        match err {
            ShardConfigError::DuplicateArtifactStem { artifact_stem } => {
                assert_eq!(artifact_stem, "decision-log");
            }
            other => panic!("expected DuplicateArtifactStem, got {other:?}"),
        }
    }

    #[test]
    fn test_BC_1_18_005_N2_shard_cap_gate_check_rejects_duplicate_artifact_stem() {
        // Full-stack: the duplicate-stem fail-loud surfaces all the way
        // through shard_cap_gate_check as a HookResult::Error, exactly like
        // any other match-time config defect (EC-009/EC-011/etc.).
        let registry = ShardRegistry {
            shards: vec![
                flat_entry("decision-log", 40_000),
                flat_entry("decision-log", 49_152),
            ],
        };
        let target = Path::new("/repo/.factory/decision-log.md");
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            target,
            &serde_json::json!({"content": "x"}),
        );
        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("decision-log") && message.contains("MULTIPLE entries"),
                    "N-2: expected the DuplicateArtifactStem diagnostic naming artifact_stem \
                     \"decision-log\", got: {message}"
                );
            }
            other => panic!(
                "N-2: a dispatch whose target matches TWO [[shard]] entries for the same \
                 artifact_stem MUST be a fail-loud HookResult::Error — got {other:?}"
            ),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_BC_1_18_005_INV3_EC_001_shard_cap_gate_check_unmatched_path_never_pays_stat_cost() {
        // A self-referential symlink makes ANY stat()/metadata() call on this
        // path fail with ELOOP. This path's stem does NOT match any [[shard]]
        // entry, so a correct implementation MUST short-circuit (Invariant 3)
        // BEFORE ever attempting to stat() it — the call must cleanly return
        // Continue despite the landmine. A naive "always stat, then check
        // match" implementation would instead surface the ELOOP failure
        // (as an io::Error it has nowhere sound to route, or a panic).
        let dir = tempfile::tempdir().expect("tempdir");
        let looped = dir.path().join("unmatched-canary.md");
        std::os::unix::fs::symlink(&looped, &looped).expect("create self-referential symlink");

        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &looped,
            &serde_json::json!({"content": "hi"}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "INV3/EC-001: an unmatched path MUST return Continue with NO stat() call — a stat() \
             attempt on this self-referential symlink would surface ELOOP, not a clean Continue"
        );
    }

    #[test]
    fn test_BC_1_18_005_INV1_shard_cap_gate_check_unmatched_path_returns_continue() {
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let target = Path::new("/repo/some/unrelated/file.md");
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            target,
            &serde_json::json!({"content": "hello"}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "PC1/EC-001: no [[shard]] match MUST return Continue"
        );
    }

    // ===================================================================
    // Postcondition 4/6/7 — byte-size-denominated cap formula (AC-004)
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_PC4_compute_shard_cap_bytes_clean_round_numbers() {
        let inputs = CapFormulaInputs {
            practical_fuel_ceiling: 1_000,
            worst_case_fuel_per_byte: 10.0,
            max_single_record_bytes: 10,
            safety_margin: 5,
        };
        assert_eq!(
            compute_shard_cap_bytes(&inputs),
            85,
            "PC4: shard_cap_bytes = floor(PRACTICAL_FUEL_CEILING / WORST_CASE_FUEL_PER_BYTE) \
             - MAX_SINGLE_RECORD_BYTES - SAFETY_MARGIN"
        );
    }

    #[test]
    fn test_BC_1_18_005_PC6_compute_shard_cap_bytes_bc_provisional_worked_example() {
        // BC-1.18.005 Postcondition 6 worked example (today's provisional
        // constants): floor(8,000,000 / 106.36) - 16,384 - 8,192
        //           = 75,216 - 24,576 = 50,640.
        // This is the formula's own raw ceiling output — the BC's separate,
        // more-conservative editorial choice of a 49,152-byte (48 KiB)
        // provisional `shard_cap_bytes` CONFIG value is a human-chosen value
        // satisfying Postcondition 4's `shard_cap_bytes <= (formula)`
        // constraint (49,152 <= 50,640), not something this function itself
        // computes or rounds to.
        let inputs = CapFormulaInputs {
            practical_fuel_ceiling: 8_000_000,
            worst_case_fuel_per_byte: 106.36,
            max_single_record_bytes: 16_384,
            safety_margin: 8_192,
        };
        assert_eq!(
            compute_shard_cap_bytes(&inputs),
            50_640,
            "PC6: formula output for today's provisional constants MUST equal 50,640"
        );
    }

    // ===================================================================
    // Postcondition 5 — Cross-Validator Minimum Rule (AC-003)
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_AC_003_PC5_effective_shard_cap_bytes_burst_log_min_of_three_validators() {
        // Canonical vector: burst-log.md read by validate-burst-log(40000),
        // regression-gate(49152), convergence-tracker(52000) -> MIN = 40,000.
        assert_eq!(
            effective_shard_cap_bytes(&[40_000, 49_152, 52_000]),
            40_000,
            "PC5 Cross-Validator Minimum Rule: effective cap = MIN across all Cohort B readers"
        );
    }

    #[test]
    fn test_BC_1_18_005_AC_003_PC5_effective_shard_cap_bytes_decision_log_min_of_two_validators() {
        // Canonical vector: decision-log.md NOT read by validate-burst-log ->
        // MIN(regression-gate=49152, convergence-tracker=52000) = 49,152.
        assert_eq!(
            effective_shard_cap_bytes(&[49_152, 52_000]),
            49_152,
            "PC5: validate-burst-log's (possibly tighter) cap MUST NOT apply to artifacts it doesn't read"
        );
    }

    #[test]
    fn test_BC_1_18_005_effective_shard_cap_bytes_single_validator_returns_itself() {
        assert_eq!(effective_shard_cap_bytes(&[49_152]), 49_152);
    }

    #[test]
    fn test_BC_1_18_005_PC5_effective_shard_cap_bytes_min_is_order_independent() {
        assert_eq!(
            effective_shard_cap_bytes(&[52_000, 40_000, 49_152]),
            40_000,
            "PC5: MIN must be order-independent"
        );
    }

    // ===================================================================
    // Postcondition 2 / EC-004 — stat()-only current-shard byte-size read
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_PC2_current_shard_bytes_flat_reads_real_file_size() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("decision-log.md");
        std::fs::write(&path, vec![b'x'; 1_234]).expect("write fixture");
        let size =
            current_shard_bytes_flat(&path).expect("stat() must succeed for an existing file");
        assert_eq!(
            size, 1_234,
            "PC2: current_shard_bytes_flat MUST report the file's real byte size via stat()/metadata()"
        );
    }

    #[test]
    fn test_BC_1_18_005_EC_004_current_shard_bytes_flat_missing_file_is_ok_zero() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("never-written.md");
        assert!(!path.exists(), "precondition: fixture path must not exist");
        let size = current_shard_bytes_flat(&path)
            .expect("EC-004: a missing shard file MUST be Ok(0), not an io::Error");
        assert_eq!(
            size, 0,
            "EC-004: first write ever -> current_shard_bytes treated as 0"
        );
    }

    // ===================================================================
    // EC-014 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-3
    // finding F-C1-P3-001, MEDIUM, product-owner adjudication, BC-1.18.005
    // v1.9) — item-count shape's missing-file graceful degradation,
    // mirroring EC-004's flat-shape precedent above. NOT fail-loud: a
    // legitimate first-ever Write that CREATES a not-yet-existing
    // "frontmatter-changelog-array"-shaped target artifact MUST be treated
    // as holding 0 existing changelog items, never hard-blocked as
    // HookResult::Error.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_EC_014_read_changelog_item_count_missing_file_is_ok_zero() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("never-written-index.md");
        assert!(!path.exists(), "precondition: fixture path must not exist");
        let count = read_changelog_item_count(&path).expect(
            "EC-014: a missing frontmatter-changelog-array target file MUST be Ok(0), not an \
             io::Error — read_changelog_item_count must map io::ErrorKind::NotFound to Ok(0), \
             mirroring EC-004's current_shard_bytes_flat precedent",
        );
        assert_eq!(
            count, 0,
            "EC-014: first write ever -> current_item_count treated as 0, mirroring EC-004's \
             flat-shape missing-file precedent"
        );
    }

    #[test]
    fn test_BC_1_18_005_EC_014_shard_cap_gate_check_create_path_missing_file_continues() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Deliberately do NOT create the target file — this is the
        // first-ever Write that CREATES a new "frontmatter-changelog-array"-
        // shaped sharded index file, with valid frontmatter (including a
        // changelog: array) present in the content payload.
        let target = dir.path().join("BC-INDEX.md");
        assert!(
            !target.exists(),
            "precondition: fixture path must not exist"
        );

        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        let registry = ShardRegistry {
            shards: vec![entry],
        };

        let content = "---\ntitle: \"BC-INDEX\"\nchangelog:\n  - version: \"1.0\"\n---\n\n# Body\n";
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &target,
            &serde_json::json!({"content": content}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "EC-014: a Write that CREATES a not-yet-existing frontmatter-changelog-array-shaped \
             target MUST Continue — read_changelog_item_count's NotFound->Ok(0) mapping makes \
             current_item_count + 1 = 0 + 1 = 1 <= N=50, so the legitimate create is NEVER \
             hard-blocked as HookResult::Error"
        );
    }

    // ===================================================================
    // Postcondition 3 — per-tool-semantics projected-size formula (AC-002)
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_PC3_tool_kind_from_tool_name_write() {
        assert_eq!(ToolKind::from_tool_name("Write"), Some(ToolKind::Write));
    }

    #[test]
    fn test_BC_1_18_005_PC3_tool_kind_from_tool_name_edit() {
        assert_eq!(ToolKind::from_tool_name("Edit"), Some(ToolKind::Edit));
    }

    #[test]
    fn test_BC_1_18_005_PC3_tool_kind_from_tool_name_multi_edit() {
        assert_eq!(
            ToolKind::from_tool_name("MultiEdit"),
            Some(ToolKind::MultiEdit)
        );
    }

    #[test]
    fn test_BC_1_18_005_tool_kind_from_tool_name_unknown_tool_returns_none() {
        assert_eq!(ToolKind::from_tool_name("Bash"), None);
        assert_eq!(ToolKind::from_tool_name("Read"), None);
        assert_eq!(ToolKind::from_tool_name(""), None);
    }

    #[test]
    fn test_BC_1_18_005_vector_write_5000_bytes_under_cap_continues() {
        // Canonical vector: Write, content=5,000 bytes, cap=49,152 -> Continue
        // (current_shard_bytes is irrelevant to the Write formula).
        let projected = projected_size_write(5_000);
        assert_eq!(
            projected, 5_000,
            "PC3 CORRECTED Write leg: projected_size = len(content) alone"
        );
        assert!(
            !size_trigger_fires(projected, 49_152),
            "5,000 <= 49,152 must NOT trigger a roll"
        );
    }

    #[test]
    fn test_BC_1_18_005_vector_write_50000_bytes_over_cap_triggers() {
        // Canonical vector: Write, content=50,000 bytes, cap=49,152 -> roll triggers.
        let projected = projected_size_write(50_000);
        assert_eq!(projected, 50_000);
        assert!(
            size_trigger_fires(projected, 49_152),
            "50,000 > 49,152 MUST trigger a roll"
        );
    }

    #[test]
    fn test_BC_1_18_005_regression_same_size_write_does_not_double_count_current_shard_bytes() {
        // NEW regression vector (fix-burst pass-2, F-P2-002): current shard
        // 40,000 bytes, content 40,000 bytes (same-size full-file rewrite) ->
        // Continue. The WITHDRAWN formula would have computed
        // 40,000+40,000=80,000>49,152 and wrongly rolled.
        let current_shard_bytes_irrelevant_to_write = 40_000u64;
        let projected = projected_size_write(40_000);
        assert_eq!(
            projected, 40_000,
            "Write projected_size MUST equal len(content) alone, current_shard_bytes \
             ({current_shard_bytes_irrelevant_to_write}) MUST NOT be added"
        );
        assert!(
            !size_trigger_fires(projected, 49_152),
            "regression: same-size full-file Write MUST NOT trigger a roll"
        );
    }

    #[test]
    fn test_BC_1_18_005_net_delta_bytes_for_edit_positive_delta() {
        assert_eq!(
            net_delta_bytes_for_edit(1_000, 6_000),
            5_000,
            "net_delta_bytes_for_edit = len(new_string) - len(old_string)"
        );
    }

    #[test]
    fn test_BC_1_18_005_net_delta_bytes_for_edit_negative_delta_shrinks() {
        assert_eq!(
            net_delta_bytes_for_edit(6_000, 1_000),
            -5_000,
            "net_delta_bytes_for_edit is signed — a shrinking edit MUST be negative"
        );
    }

    #[test]
    fn test_BC_1_18_005_vector_edit_current_45000_plus_5000_net_delta_triggers() {
        // Canonical vector: Edit, current shard 45,000, net +5,000, cap=49,152
        // -> Edit/MultiEdit formula UNCHANGED: projected = 50,000 > 49,152 -> roll triggers.
        let net = net_delta_bytes_for_edit(1_000, 6_000);
        let projected = projected_size_edit(45_000, net);
        assert_eq!(
            projected, 50_000,
            "Edit/MultiEdit leg UNCHANGED: projected_size = current_shard_bytes + net_delta_bytes"
        );
        assert!(
            size_trigger_fires(projected, 49_152),
            "50,000 > 49,152 MUST trigger a roll"
        );
    }

    #[test]
    fn test_BC_1_18_005_EC_005_vector_multi_edit_lessons_mixed_sign_deltas_triggers() {
        // Canonical vector: MultiEdit on lessons.md, edits netting
        // +2,000/-500/+100, current shard 48,000, cap=49,152 -> net=1,600;
        // projected=49,600 > 49,152 -> roll triggers.
        let edits = [
            EditDelta {
                old_len_bytes: 0,
                new_len_bytes: 2_000,
            },
            EditDelta {
                old_len_bytes: 500,
                new_len_bytes: 0,
            },
            EditDelta {
                old_len_bytes: 0,
                new_len_bytes: 100,
            },
        ];
        let net = net_delta_bytes_for_multi_edit(&edits);
        assert_eq!(
            net, 1_600,
            "EC-005: MultiEdit net_delta_bytes = SUM of per-edit-block net deltas"
        );
        let projected = projected_size_edit(48_000, net);
        assert_eq!(projected, 49_600);
        assert!(
            size_trigger_fires(projected, 49_152),
            "49,600 > 49,152 MUST trigger a roll"
        );
    }

    #[test]
    fn test_BC_1_18_005_EC_005_multi_edit_net_shrinking_never_triggers_even_with_large_individual_blocks()
     {
        // "a net-shrinking MultiEdit never triggers a roll even if individual
        // edit blocks are large" — one block is a large +1,000, another a
        // large -5,000 deletion; net shrinks overall.
        let edits = [
            EditDelta {
                old_len_bytes: 0,
                new_len_bytes: 1_000,
            },
            EditDelta {
                old_len_bytes: 5_000,
                new_len_bytes: 0,
            },
        ];
        let net = net_delta_bytes_for_multi_edit(&edits);
        assert_eq!(net, -4_000);
        let projected = projected_size_edit(49_000, net);
        assert_eq!(projected, 45_000);
        assert!(
            !size_trigger_fires(projected, 49_152),
            "EC-005: a net-shrinking MultiEdit MUST NOT trigger a roll even with large individual blocks"
        );
    }

    #[test]
    fn test_BC_1_18_005_projected_size_edit_saturates_at_zero_for_underflowing_negative_delta() {
        // Doc-mandated production-grade behavior (Postcondition 3's Edit/
        // MultiEdit leg doc comment): a net_delta_bytes more negative than
        // current_shard_bytes MUST saturate at 0, never underflow/panic.
        assert_eq!(
            projected_size_edit(100, -500),
            0,
            "projected_size_edit MUST saturate at 0 for a large negative net_delta_bytes, never underflow"
        );
    }

    #[test]
    fn test_BC_1_18_005_EC_002_size_trigger_fires_false_at_exact_boundary() {
        assert!(
            !size_trigger_fires(49_152, 49_152),
            "EC-002: projected_size == shard_cap_bytes MUST NOT trigger (inclusive <=)"
        );
    }

    #[test]
    fn test_BC_1_18_005_EC_003_size_trigger_fires_true_one_byte_over_boundary() {
        assert!(
            size_trigger_fires(49_153, 49_152),
            "EC-003: projected_size == shard_cap_bytes + 1 MUST trigger"
        );
    }

    #[test]
    fn test_BC_1_18_005_size_trigger_fires_false_comfortably_under_cap() {
        assert!(!size_trigger_fires(1, 49_152));
    }

    #[test]
    fn test_BC_1_18_005_regression_shard_cap_gate_check_same_size_write_continues() {
        // Full-stack regression vector via the top-level dispatch entry
        // point: current shard 40,000 bytes on disk, Write content also
        // 40,000 bytes -> Continue (the withdrawn formula would have wrongly
        // rolled here).
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("decision-log.md");
        std::fs::write(&target, "y".repeat(40_000)).expect("write fixture shard");
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let content = "z".repeat(40_000);
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &target,
            &serde_json::json!({"content": content}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "regression: same-size full-file Write MUST NOT trigger a roll"
        );
    }

    #[test]
    fn test_BC_1_18_005_shard_cap_gate_check_write_under_cap_continues() {
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let target = Path::new("/repo/.factory/decision-log.md");
        let content = "x".repeat(5_000);
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            target,
            &serde_json::json!({"content": content}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "matched Write with projected_size 5,000 <= cap 49,152 MUST Continue"
        );
    }

    // ===================================================================
    // Postcondition 8 — item-count trigger (AC-005)
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_PC8_read_changelog_item_count_counts_yaml_list_items() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        std::fs::write(
            &path,
            "---\n\
             title: \"BC-INDEX\"\n\
             changelog:\n\
             \x20\x20- version: \"1.0\"\n\
             \x20\x20\x20\x20date: \"2026-01-01\"\n\
             \x20\x20- version: \"1.1\"\n\
             \x20\x20\x20\x20date: \"2026-01-02\"\n\
             \x20\x20- version: \"1.2\"\n\
             \x20\x20\x20\x20date: \"2026-01-03\"\n\
             ---\n\n# Body\n",
        )
        .expect("write fixture");
        let count = read_changelog_item_count(&path)
            .expect("PC8: read_changelog_item_count must succeed for a well-formed frontmatter changelog array");
        assert_eq!(
            count, 3,
            "PC8: item count MUST equal the number of changelog: array entries"
        );
    }

    #[test]
    fn test_BC_1_18_005_PC8_read_changelog_item_count_empty_array_is_zero() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        std::fs::write(
            &path,
            "---\ntitle: \"BC-INDEX\"\nchangelog: []\n---\n\n# Body\n",
        )
        .expect("write fixture");
        let count = read_changelog_item_count(&path)
            .expect("read must succeed for an empty changelog: array");
        assert_eq!(count, 0);
    }

    #[test]
    fn test_BC_1_18_005_vector_item_count_trigger_10_of_50_continues() {
        // Canonical vector: N=50, current 10 items -> 11 <= 50 -> Continue.
        assert!(!item_count_trigger_fires(10, 50));
    }

    #[test]
    fn test_BC_1_18_005_EC_008_vector_item_count_trigger_fires_at_exactly_n() {
        // Canonical vector: N=50, current 50 items -> 51 > 50 -> fires.
        assert!(item_count_trigger_fires(50, 50));
    }

    #[test]
    fn test_BC_1_18_005_VP_140_item_count_trigger_does_not_fire_at_n_minus_1() {
        assert!(
            !item_count_trigger_fires(49, 50),
            "N-1 items -> current_item_count+1 == N -> must NOT fire"
        );
    }

    #[test]
    fn test_BC_1_18_005_VP_140_item_count_trigger_fires_at_n_plus_1() {
        assert!(item_count_trigger_fires(51, 50), "N+1 items -> must fire");
    }

    #[test]
    fn test_BC_1_18_005_shard_cap_gate_check_item_count_happy_path_continues() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("BC-INDEX.md");
        // 10 changelog items -> 11 <= 50 -> Continue.
        let mut body = String::from("---\ntitle: \"BC-INDEX\"\nchangelog:\n");
        for i in 0..10 {
            body.push_str(&format!("  - version: \"1.{i}\"\n"));
        }
        body.push_str("---\n\n# Body\n");
        std::fs::write(&target, body).expect("write fixture");

        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        let registry = ShardRegistry {
            shards: vec![entry],
        };

        let result = shard_cap_gate_check(
            &registry,
            "Edit",
            &target,
            &serde_json::json!({"old_string": "a", "new_string": "ab"}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "item-count happy path: 10 items -> 11 <= N=50 -> Continue"
        );
    }

    #[test]
    fn test_BC_1_18_005_EC_010_resolved_low_water_mark_defaults_to_floor_n_div_2_when_omitted() {
        assert_eq!(resolved_low_water_mark(50, None), 25);
    }

    #[test]
    fn test_BC_1_18_005_resolved_low_water_mark_floor_of_odd_n_when_omitted() {
        assert_eq!(resolved_low_water_mark(51, None), 25, "floor(51/2) = 25");
    }

    #[test]
    fn test_BC_1_18_005_resolved_low_water_mark_uses_explicit_value_when_present() {
        assert_eq!(resolved_low_water_mark(50, Some(10)), 10);
    }

    #[test]
    fn test_BC_1_18_005_EC_011_validate_low_water_mark_rejects_equal_to_n() {
        let err = validate_low_water_mark("BC-INDEX", 50, 50)
            .expect_err("EC-011: low_water_mark == N (degenerate boundary) MUST fail-loud");
        match err {
            ShardConfigError::InvalidLowWaterMark {
                artifact_stem,
                n,
                low_water_mark,
            } => {
                assert_eq!(artifact_stem, "BC-INDEX");
                assert_eq!(n, 50);
                assert_eq!(low_water_mark, 50);
            }
            other => panic!("expected InvalidLowWaterMark, got {other:?}"),
        }
    }

    #[test]
    fn test_BC_1_18_005_EC_011_validate_low_water_mark_rejects_negative() {
        let err = validate_low_water_mark("BC-INDEX", 50, -1)
            .expect_err("EC-011: negative low_water_mark MUST fail-loud");
        assert!(matches!(err, ShardConfigError::InvalidLowWaterMark { .. }));
    }

    #[test]
    fn test_BC_1_18_005_EC_011_validate_low_water_mark_rejects_greater_than_n() {
        let err = validate_low_water_mark("BC-INDEX", 50, 100)
            .expect_err("low_water_mark > N MUST fail-loud too, not just the == N boundary");
        assert!(matches!(err, ShardConfigError::InvalidLowWaterMark { .. }));
    }

    #[test]
    fn test_BC_1_18_005_EC_012_validate_low_water_mark_n_minus_1_is_valid_and_fires_advisory() {
        // Canonical vector: N=50, low_water_mark=49 (N-1) -> Ok(true) (fires
        // advisory), never Err.
        let fires = validate_low_water_mark("BC-INDEX", 50, 49)
            .expect("EC-012: N-1 MUST load successfully, never HookResult::Error");
        assert!(fires, "EC-012: 49 > floor(50/2)=25 -> advisory MUST fire");
    }

    #[test]
    fn test_BC_1_18_005_VP_140_validate_low_water_mark_default_value_does_not_fire_advisory() {
        // low_water_mark == floor(N/2) exactly (the recommended default) -> Ok(false).
        let fires =
            validate_low_water_mark("BC-INDEX", 50, 25).expect("25 is legal (0 <= 25 < 50)");
        assert!(
            !fires,
            "VP-140: low_water_mark == floor(N/2) MUST NOT fire the advisory (not strictly greater)"
        );
    }

    #[test]
    fn test_BC_1_18_005_VP_140_validate_low_water_mark_floor_plus_one_fires_advisory() {
        let fires =
            validate_low_water_mark("BC-INDEX", 50, 26).expect("26 is legal (0 <= 26 < 50)");
        assert!(
            fires,
            "VP-140: low_water_mark == floor(N/2)+1 MUST fire the advisory"
        );
    }

    #[test]
    fn test_BC_1_18_005_validate_low_water_mark_zero_is_valid_and_does_not_fire_advisory() {
        let fires = validate_low_water_mark("BC-INDEX", 50, 0).expect("0 is legal (0 <= 0 < 50)");
        assert!(!fires);
    }

    #[test]
    fn test_BC_1_18_005_load_valid_flat_shape_entry_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("shard-config.toml");
        std::fs::write(
            &cfg_path,
            "[[shard]]\n\
             artifact_stem = \"decision-log\"\n\
             artifact_path = \"decision-log.md\"\n\
             practical_fuel_ceiling = 8000000\n\
             worst_case_fuel_per_byte = 106.36\n\
             max_single_record_bytes = 16384\n\
             safety_margin = 8192\n\
             shard_cap_bytes = 49152\n\
             shape = \"flat\"\n",
        )
        .expect("write fixture");
        let registry = ShardRegistry::load(&cfg_path)
            .expect("Precondition 2: a well-formed [[shard]] config MUST load");
        assert_eq!(registry.shards.len(), 1);
        assert_eq!(registry.shards[0].artifact_stem, "decision-log");
        assert_eq!(registry.shards[0].shape, Some(ShardShape::Flat));
    }

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001):
    // this test previously asserted EC-009's fail-loud outcome via
    // `ShardRegistry::load(path).is_err()` — the pre-v1.12 eager,
    // whole-file validation loop. Post-restructure, `ShardRegistry::load`
    // becomes structural-TOML-parse-only (an entry omitting `shape`
    // deserializes fine, since `shape: Option<ShardShape>`) and this
    // semantic check moves to entry-MATCH time (`validate_entry`, called
    // only on the entry `find_matching_entry` resolves for the current
    // dispatch — never on sibling entries; see EC-018/EC-019 below). This
    // test now drives the PUBLIC match-time gate path directly
    // (`shard_cap_gate_check`, constructing the `ShardEntry` in-memory
    // rather than round-tripping it through `load()`) to pin the PRESERVED
    // behavior Postcondition 1's "Blast-radius scoping" ruling requires: a
    // dispatch whose target MATCHES a `shape`-omitting entry MUST still
    // fail loud, unchanged — only the entry-SCOPE narrows away from
    // sibling entries, never the matched entry's own outcome.
    // `shard_cap_gate_check` already carries this exact defensive check
    // today: it calls `validate_entry(entry)` on the matched entry
    // immediately after `find_matching_entry` resolves it (before any
    // shape-dispatch), and `validate_entry` rejects a `None` `shape` via
    // its own EC-009 guard, returning `Err(MissingShape)`. So this
    // assertion is GREEN now via that pre-existing entry-match-time
    // validation path, and remains GREEN post-restructure for the same
    // reason — the inline `let Some(shape) = entry.shape else { .. }`
    // fallback further down in `shard_cap_gate_check` is an unreachable
    // defensive backstop, not the mechanism this test actually exercises.
    #[test]
    fn test_BC_1_18_005_EC_009_matched_entry_missing_shape_field_is_fail_loud() {
        let entry = ShardEntry {
            artifact_stem: "decision-log".to_string(),
            artifact_path: "decision-log.md".to_string(),
            practical_fuel_ceiling: 8_000_000,
            worst_case_fuel_per_byte: 106.36,
            max_single_record_bytes: 16_384,
            safety_margin: 8_192,
            shard_cap_bytes: 49_152,
            shape: None, // EC-009: omitted entirely — fail-loud, never defaulted.
            n: None,
            low_water_mark: None,
        };
        let registry = ShardRegistry {
            shards: vec![entry],
        };
        // Target path's stem ("decision-log") MATCHES the malformed entry
        // itself — this is the "matched malformed entry" scenario EC-018's
        // blast-radius ruling explicitly preserves, as opposed to the
        // unmatched/matched-different-entry scenarios EC-018 pins as
        // `Continue` in the integration test suite.
        let target = std::path::Path::new("decision-log.md");

        let result = shard_cap_gate_check(
            &registry,
            "Write",
            target,
            &serde_json::json!({"content": "x"}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("decision-log") && message.contains("EC-009"),
                    "EC-009: expected the MissingShape diagnostic naming artifact_stem \
                     \"decision-log\" and citing EC-009, got: {message}"
                );
            }
            other => panic!(
                "EC-009: a dispatch whose target MATCHES an entry omitting `shape` entirely MUST \
                 be a fail-loud HookResult::Error, never a silent default — got {other:?}"
            ),
        }
    }

    #[test]
    fn test_BC_1_18_005_EC_010_load_frontmatter_changelog_array_omits_low_water_mark_loads_ok() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("shard-config.toml");
        std::fs::write(
            &cfg_path,
            shard_toml_frontmatter_entry("BC-INDEX", 50, None),
        )
        .expect("write fixture");
        let registry = ShardRegistry::load(&cfg_path)
            .expect("EC-010: omitting low_water_mark MUST NOT fail config load");
        assert_eq!(
            registry.shards[0].low_water_mark, None,
            "load itself does not fill in the floor(N/2) default — resolved_low_water_mark is the caller's job"
        );
        assert_eq!(registry.shards[0].n, Some(50));
    }

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001):
    // `low_water_mark` range validation now lives in `validate_entry`
    // (called at entry-MATCH time, never eagerly across every `[[shard]]`
    // entry — see EC-018/EC-019). This test now drives `validate_entry`
    // directly on an in-memory `ShardEntry`, preserving the exact assertion
    // (the same malformed `low_water_mark == N` input still routes to the
    // same specific `InvalidLowWaterMark` variant) — only the call surface
    // moved from `ShardRegistry::load` to `validate_entry`.
    #[test]
    fn test_BC_1_18_005_EC_011_validate_entry_rejects_low_water_mark_equal_to_n() {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        entry.low_water_mark = Some(50);

        let err = validate_entry(&entry)
            .expect_err("EC-011: low_water_mark == N MUST fail-loud at entry-match time");
        assert!(matches!(err, ShardConfigError::InvalidLowWaterMark { .. }));
    }

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001) —
    // same rationale as the `..._rejects_low_water_mark_equal_to_n` test
    // immediately above.
    #[test]
    fn test_BC_1_18_005_EC_011_validate_entry_rejects_negative_low_water_mark() {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        entry.low_water_mark = Some(-1);

        let err = validate_entry(&entry)
            .expect_err("EC-011: negative low_water_mark MUST fail-loud at entry-match time");
        assert!(matches!(err, ShardConfigError::InvalidLowWaterMark { .. }));
    }

    #[test]
    fn test_BC_1_18_005_EC_012_load_accepts_n_minus_1_low_water_mark_and_succeeds() {
        // MIGRATED comment (BC-1.18.005 v1.12 MATCH-FIRST restructure,
        // F-C1-P6-001): `ShardRegistry::load` is now structural-TOML-parse-
        // only and no longer reaches the `tracing::warn!` amortization-
        // advisory callsite (that callsite now lives solely in
        // `validate_entry` — see the emission test immediately below, which
        // exercises it via `validate_entry` under `count_warns`). This test
        // pins the STRUCTURAL claim only: a syntactically-valid
        // `low_water_mark = 49` round-trips through `load()` unchanged.
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("shard-config.toml");
        std::fs::write(
            &cfg_path,
            shard_toml_frontmatter_entry("BC-INDEX", 50, Some(49)),
        )
        .expect("write fixture");

        let registry = ShardRegistry::load(&cfg_path)
            .expect("EC-012: low_water_mark = N-1 = 49 MUST load successfully (structural parse)");
        assert_eq!(registry.shards[0].low_water_mark, Some(49));
    }

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001): the
    // EC-012 amortization-advisory `tracing::warn!` now fires from
    // `validate_entry` (called at entry-MATCH time), not from
    // `ShardRegistry::load`. This test drives `validate_entry` directly via
    // `count_warns`, preserving the exact assertion (exactly one warn for
    // this `(N, low_water_mark)` pair) — `count_warns` is generic over its
    // closure's return type, so `validate_entry`'s `Result<(), _>` and
    // `ShardRegistry::load`'s `Result<ShardRegistry, _>` both route through
    // it unchanged.
    #[test]
    fn test_BC_1_18_005_EC_012_validate_entry_emits_warn_advisory_when_low_water_mark_exceeds_default()
     {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        entry.low_water_mark = Some(49);

        let (warn_count, result) = count_warns(|| validate_entry(&entry));

        assert!(
            result.is_ok(),
            "EC-012: low_water_mark=49 (N-1, N=50) MUST validate successfully: {result:?}"
        );
        assert_eq!(
            warn_count, 1,
            "EC-012/VP-140: validate_entry MUST emit exactly one non-fatal tracing::warn! \
             amortization advisory when low_water_mark(49) > floor(N/2)(25)"
        );
    }

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001,
    // F-C1-P7-003): the EC-012 amortization-advisory `tracing::warn!`
    // callsite this negative control guards now lives solely in
    // `validate_entry` (called at entry-MATCH time), not in
    // `ShardRegistry::load` (structural-TOML-parse-only, post-restructure).
    // Wrapping `ShardRegistry::load` in `count_warns` here would be
    // VACUOUS — `load` can no longer reach that callsite under any input,
    // so `warn_count == 0` would hold even if the "does NOT fire when
    // `low_water_mark <= floor(N/2)`" half of VP-140's biconditional broke.
    // This test drives `validate_entry` directly, mirroring the sibling
    // positive-control test above
    // (`test_BC_1_18_005_EC_012_validate_entry_emits_warn_advisory_when_low_water_mark_exceeds_default`),
    // restoring genuine emission-level coverage of the "does not fire"
    // half.
    #[test]
    fn test_BC_1_18_005_VP_140_validate_entry_default_low_water_mark_does_not_emit_warn_advisory() {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        entry.low_water_mark = Some(25);

        let (warn_count, result) = count_warns(|| validate_entry(&entry));

        assert!(result.is_ok());
        assert_eq!(
            warn_count, 0,
            "low_water_mark == floor(N/2) (the recommended default) MUST NOT fire the amortization advisory"
        );
    }

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001,
    // F-C1-P7-003): same VACUOUSNESS closure as the sibling test
    // immediately above — the EC-012 advisory callsite this negative
    // control guards is reached via `validate_entry`, not
    // `ShardRegistry::load`. This test drives `validate_entry` directly
    // with an OMITTED `low_water_mark`, so `resolved_low_water_mark`
    // resolves it to `floor(N/2)` before the advisory check ever runs.
    #[test]
    fn test_BC_1_18_005_EC_010_validate_entry_omitted_low_water_mark_does_not_emit_warn_advisory() {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        entry.low_water_mark = None;

        let (warn_count, result) = count_warns(|| validate_entry(&entry));

        assert!(result.is_ok());
        assert_eq!(
            warn_count, 0,
            "EC-010: an omitted low_water_mark resolves to floor(N/2) — the default itself — \
             and MUST NOT fire the advisory"
        );
    }

    // ===================================================================
    // F-002 (MED, S-25.02 Phase F4 LOCAL adversary pass-1 cluster-1) — a
    // Write against a [[shard]]-matched path MUST NOT be blocked by the
    // VALUE of current_shard_bytes when the Write's own content length is
    // under cap. Postcondition 3's CORRECTED Write leg computes
    // projected_size = len(content) ALONE — current_shard_bytes is never
    // summed into it, regardless of what value that read produces.
    //
    // RETARGETED (S-25.02 Phase F4 cluster-2 pass-2 finding F-C2-P2-003,
    // product-owner-approved refined Option A): this test ORIGINALLY used
    // a self-referential-symlink ELOOP fixture to prove a *stat() FAILURE*
    // was irrelevant to the Write formula, and its old assertion message
    // claimed "shard_cap_gate_check's ToolKind::Write arm never calls
    // current_shard_bytes_flat() at all" — TRUE when this test was
    // authored (pre-v1.5), but made FALSE by BC-1.18.006 v1.5's own
    // dedicated Write-arm crash-orphan backstop probe (see the
    // `ToolKind::Write` arm's own doc comment above, "a NEW, DEDICATED,
    // bounded stat() specifically for the backstop probe"), which DOES
    // call `current_shard_bytes_flat` for that unrelated purpose. Once
    // BC-1.18.006 v1.6 Invariant 8 / EC-019 / E-SHD-008 (product-owner
    // adjudication, F-C2-P2-003) makes that SAME backstop probe's
    // non-NotFound stat() failures fail LOUD (see
    // `test_BC_1_18_006_F003_write_backstop_stat_failure_fails_loud_e_shd_008`,
    // `bc_1_18_006_roll_tests` below), the ELOOP fixture's expected
    // outcome flips from `Continue` to `Error` — a stat()-FAILURE
    // disposition question BC-1.18.005 Postcondition 3 never governed in
    // the first place (confirmed: only BC-1.18.006 v1.5+ added any Write-
    // arm stat() call at all; BC-1.18.005 itself never required or
    // forbade one). No BC-1.18.005 amendment is needed — only this test's
    // own stale, over-broad fixture.
    //
    // F-002's REAL, still-binding invariant was never about stat()
    // failure — it is Postcondition 3's content-alone formula ignoring
    // current_shard_bytes's *value*. This retargeted fixture proves
    // exactly that with a REAL, successfully-stat()-able on-disk
    // canonical file (45,000 bytes — BC-1.18.006 v1.4's own original,
    // since-corrected Canonical Test Vectors row) and a 5,000-byte Write
    // content: the WITHDRAWN uniform formula would have summed
    // 45,000 + 5,000 = 50,000 > 49,152 and wrongly rolled; the CORRECTED
    // formula (content alone) does not. Because current_shard_bytes here
    // (45,000) is itself UNDER shard_cap_bytes (49,152), this fixture's
    // stat() call succeeds and never reaches the backstop's over-cap
    // reconciliation branch OR its Err(e) leg — it passes identically
    // against BOTH the current fail-open backstop code and the
    // about-to-land E-SHD-008 fail-loud code, so it will not be disturbed
    // by the F-003 flip landing immediately after this commit.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_F002_write_under_cap_continues_regardless_of_current_shard_bytes_value() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir.path().join("decision-log.md");
        // Real on-disk canonical (NOT a symlink) — the Write-arm backstop's
        // stat() succeeds here (Ok(45_000)), and 45_000 < shard_cap_bytes
        // (49_152), so neither the backstop's over-cap reconciliation
        // branch nor its Err(e) leg is ever reached by this fixture.
        std::fs::write(&canonical, "y".repeat(45_000)).expect("seed canonical shard file");

        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &canonical,
            &serde_json::json!({"content": "x".repeat(5_000)}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "F-002: a Write's projected_size = len(content) alone (Postcondition 3 CORRECTED) — \
             the VALUE of current_shard_bytes (45,000 here, successfully read via a real, \
             non-failing stat()) is irrelevant to the Write formula and MUST NOT be summed into \
             it. The withdrawn uniform formula would have computed 45,000 + 5,000 = 50,000 > \
             49,152 and wrongly rolled; the corrected formula (5,000 alone) must Continue."
        );
    }

    // ===================================================================
    // Postcondition 9 / EC-013 (BC-1.18.005 v1.7, S-25.02 Phase F4 LOCAL
    // adversary pass-1 cluster-1 finding) — load-time fail-loud enforcement
    // of shard_cap_bytes <= compute_shard_cap_bytes(entry.cap_formula_inputs()).
    // ===================================================================

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001): the
    // cap-vs-formula-ceiling comparison (Postcondition 9) now lives in
    // `validate_entry`, called at entry-MATCH time only. This test drives
    // `validate_entry` directly, preserving the exact assertion (the same
    // oversized `shard_cap_bytes` input still routes to the same
    // `CapExceedsFormulaCeiling` variant with the same three fields).
    #[test]
    fn test_BC_1_18_005_PC9_EC013_validate_entry_rejects_cap_greater_than_formula_ceiling() {
        let mut entry = flat_entry("decision-log", 100_000);
        entry.practical_fuel_ceiling = 8_000_000;
        entry.worst_case_fuel_per_byte = 106.36;
        entry.max_single_record_bytes = 16_384;
        entry.safety_margin = 8_192;

        // compute_shard_cap_bytes(these four inputs) = 50,640 (BC-1.18.005
        // Postcondition 6's own worked example — see
        // test_BC_1_18_005_PC6_compute_shard_cap_bytes_bc_provisional_worked_example
        // above) — 100,000 is far above its own formula-derived ceiling.
        //
        // TIGHTENED (S-25.02 Phase F4 LOCAL adversary cluster-1 pass-3
        // finding F-C1-P3-004, OBSERVATION): matches the specific
        // ShardConfigError::CapExceedsFormulaCeiling variant and its three
        // fields, aligning with the sibling EC-009/EC-011 unit tests that
        // already match their own specific variant rather than a generic
        // is_err().
        let err = validate_entry(&entry).expect_err(
            "PC9/EC-013: an entry declaring shard_cap_bytes (100,000) GREATER than its own \
             compute_shard_cap_bytes(inputs) ceiling (50,640) MUST fail-loud at entry-match time",
        );
        match err {
            ShardConfigError::CapExceedsFormulaCeiling {
                artifact_stem,
                shard_cap_bytes,
                computed_ceiling,
            } => {
                assert_eq!(artifact_stem, "decision-log");
                assert_eq!(shard_cap_bytes, 100_000);
                assert_eq!(computed_ceiling, 50_640);
            }
            other => panic!("expected CapExceedsFormulaCeiling, got {other:?}"),
        }
    }

    #[test]
    fn test_BC_1_18_005_PC9_load_accepts_cap_exactly_equal_to_formula_ceiling() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("shard-config.toml");
        std::fs::write(
            &cfg_path,
            "[[shard]]\n\
             artifact_stem = \"decision-log\"\n\
             artifact_path = \"decision-log.md\"\n\
             practical_fuel_ceiling = 8000000\n\
             worst_case_fuel_per_byte = 106.36\n\
             max_single_record_bytes = 16384\n\
             safety_margin = 8192\n\
             shard_cap_bytes = 50640\n\
             shape = \"flat\"\n",
        )
        .expect("write fixture");

        // compute_shard_cap_bytes(these four inputs) = 50,640 exactly.
        // POST-v1.12 MATCH-FIRST restructure (F-C1-P6-001): the
        // cap-vs-formula-ceiling comparison (Postcondition 9 / EC-013,
        // inclusive `<=` per Postcondition 4's own text "mirroring EC-002's
        // inclusive-boundary precedent") now lives in `validate_entry`, not
        // `ShardRegistry::load` — this test only pins that an exactly-equal
        // declared cap continues to structurally parse via `load`
        // successfully; it does NOT itself exercise the inclusive `<=`
        // boundary check (see
        // `test_BC_1_18_005_PC9_EC013_validate_entry_rejects_cap_greater_than_formula_ceiling`
        // above for that comparison's rejection-side coverage against
        // `validate_entry`).
        let result = ShardRegistry::load(&cfg_path);
        assert!(
            result.is_ok(),
            "PC9 boundary: shard_cap_bytes EXACTLY EQUAL to compute_shard_cap_bytes(inputs) MUST \
             load successfully (inclusive <=), mirroring EC-002's boundary precedent: {result:?}"
        );
    }

    // NEW (S-25.02 F4 comprehensive-sweep vacuity closure): the `load()`
    // control immediately above stopped exercising PC9's inclusive `<=`
    // boundary once the cap-vs-formula-ceiling comparison migrated to
    // `validate_entry` at v1.12 (its own comment says so) — it would still
    // pass even if `validate_entry` regressed the boundary from `<=` to `<`,
    // because `load()` never calls `validate_entry` at all. This test calls
    // `validate_entry` DIRECTLY with `shard_cap_bytes` set to EXACTLY
    // `compute_shard_cap_bytes(entry.cap_formula_inputs())` and asserts
    // `Ok(())`.
    //
    // Non-vacuousness: PAIRED with
    // `test_BC_1_18_005_PC9_EC013_validate_entry_rejects_cap_greater_than_formula_ceiling`
    // (cap = ceiling + 1 → `Err(CapExceedsFormulaCeiling)`), this test pins
    // BOTH sides of the inclusive `<=` boundary against `validate_entry`
    // itself: if the guard ever regressed from `<=` to `<`, THIS test is the
    // one that would fail (the exact-equal case would wrongly be rejected).
    // Manually verified load-bearing during authoring: temporarily bumping
    // this test's `shard_cap_bytes` to `computed_ceiling + 1` flips the
    // assertion below to a failure (`validate_entry` returns
    // `Err(CapExceedsFormulaCeiling { shard_cap_bytes: 50641, computed_ceiling:
    // 50640, .. })`), confirming the exact-equal `Ok` assertion is
    // meaningful before reverting to the exact ceiling.
    #[test]
    fn test_BC_1_18_005_PC9_validate_entry_accepts_cap_exactly_equal_to_formula_ceiling() {
        // `flat_entry`'s own default calibration inputs (practical_fuel_ceiling
        // = 8_000_000, worst_case_fuel_per_byte = 106.36, max_single_record_bytes
        // = 16_384, safety_margin = 8_192) are the SAME inputs the sibling
        // EC-013 rejection test and the BC's own Postcondition 6 worked
        // example use, whose `compute_shard_cap_bytes` result is the
        // documented 50,640 — computed here from the entry's own
        // `cap_formula_inputs()` (not hardcoded) so this test tracks the
        // formula rather than pinning a second, independently-derived magic
        // number.
        let mut entry = flat_entry("decision-log", 0);
        let computed_ceiling = compute_shard_cap_bytes(&entry.cap_formula_inputs());
        entry.shard_cap_bytes = computed_ceiling;

        let result = validate_entry(&entry);
        assert!(
            result.is_ok(),
            "PC9: shard_cap_bytes EXACTLY EQUAL to compute_shard_cap_bytes(entry.cap_formula_inputs()) \
             ({computed_ceiling}) MUST be accepted by validate_entry itself (inclusive <=) — paired \
             with the EC-013 (>ceiling -> Err) test above, this pins both sides of the boundary \
             against validate_entry directly. Got: {result:?}"
        );
    }

    // ===================================================================
    // EC-015 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4
    // finding F-C1-P4-001, MEDIUM/HIGH, BC-1.18.005 v1.10) — "divisor-door"
    // closure. `compute_shard_cap_bytes` divides `practical_fuel_ceiling` by
    // `worst_case_fuel_per_byte`; the latter is `f64`-typed and therefore
    // NOT type-guaranteed finite or positive (TOML's float grammar accepts
    // `nan`/`inf`/`-inf`/`0.0`/negative literals). `ShardRegistry::load()`
    // MUST validate `worst_case_fuel_per_byte.is_finite() &&
    // worst_case_fuel_per_byte > 0.0` for every entry BEFORE computing
    // `compute_shard_cap_bytes(entry.cap_formula_inputs())`, fail-loud on
    // violation — otherwise a degenerate divisor lets an arbitrarily-large
    // `shard_cap_bytes` sail past Postcondition 9's cap-vs-formula
    // comparison entirely, reintroducing the exact fuel-exhaustion failure
    // mode that comparison exists to prevent.
    //
    // POST-v1.12 MATCH-FIRST restructure (F-C1-P6-001): `validate_entry`
    // validates `worst_case_fuel_per_byte.is_finite() &&
    // worst_case_fuel_per_byte > 0.0` for the matched entry, at entry-match
    // time, BEFORE `compute_shard_cap_bytes` is ever called for it,
    // returning `Err(ShardConfigError::InvalidWorstCaseFuelPerByte {
    // artifact_stem, worst_case_fuel_per_byte })` on violation. The three
    // tests below pin the two rejection cases (zero and NaN) plus the
    // valid-input control.
    // ===================================================================

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001):
    // `worst_case_fuel_per_byte` finiteness/positivity validation now lives
    // in `validate_entry`, called at entry-MATCH time only. This test
    // drives `validate_entry` directly, preserving the exact assertion.
    #[test]
    fn test_BC_1_18_005_EC_015_validate_entry_rejects_zero_worst_case_fuel_per_byte_divisor_door() {
        let mut entry = flat_entry("zero-divisor-log", 100_000_000);
        entry.worst_case_fuel_per_byte = 0.0;

        // Without this guard, worst_case_fuel_per_byte = 0.0 would make
        // compute_shard_cap_bytes's internal division yield +inf, whose
        // saturating `as u64` cast is u64::MAX — the resulting ceiling would
        // comfortably exceed this fixture's already-enormous
        // shard_cap_bytes = 100,000,000, letting it sail past Postcondition
        // 9's `shard_cap_bytes > computed_ceiling` check via the degenerate
        // divisor. The guard rejects the entry BEFORE that comparison is
        // ever attempted.
        let err = validate_entry(&entry).expect_err(
            "EC-015: worst_case_fuel_per_byte = 0.0 (the divisor-door case) MUST fail-loud at \
             entry-match time, BEFORE the cap-vs-formula comparison is even attempted — an \
             arbitrarily-large shard_cap_bytes must never sail through via a degenerate divisor.",
        );
        match err {
            ShardConfigError::InvalidWorstCaseFuelPerByte {
                artifact_stem,
                worst_case_fuel_per_byte,
            } => {
                assert_eq!(artifact_stem, "zero-divisor-log");
                assert_eq!(worst_case_fuel_per_byte, 0.0);
            }
            other => panic!("expected InvalidWorstCaseFuelPerByte, got {other:?}"),
        }
    }

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001) —
    // same rationale as `..._rejects_zero_worst_case_fuel_per_byte_divisor_door`
    // immediately above.
    #[test]
    fn test_BC_1_18_005_EC_015_validate_entry_rejects_nan_worst_case_fuel_per_byte() {
        let mut entry = flat_entry("nan-divisor-log", 0);
        entry.worst_case_fuel_per_byte = f64::NAN;

        // Without this guard, worst_case_fuel_per_byte = NaN would make the
        // internal division yield NaN, whose saturating `as u64` cast is 0
        // (Rust maps a NaN float-to-int cast to 0) — the computed ceiling
        // would then also saturate at 0
        // (0.saturating_sub(16_384).saturating_sub(8_192) = 0), which is NOT
        // LESS than this fixture's own shard_cap_bytes = 0, so Postcondition
        // 9's `shard_cap_bytes > computed_ceiling` check (0 > 0 = false)
        // would never fire either. The guard rejects the entry regardless of
        // which direction the resulting degenerate arithmetic would have
        // saturated — the config is malformed either way and must never be
        // silently accepted.
        let err = validate_entry(&entry).expect_err(
            "EC-015: worst_case_fuel_per_byte = NaN MUST fail-loud at entry-match time \
             regardless of which direction the resulting degenerate arithmetic happens to \
             saturate.",
        );
        match err {
            ShardConfigError::InvalidWorstCaseFuelPerByte {
                artifact_stem,
                worst_case_fuel_per_byte,
            } => {
                assert_eq!(artifact_stem, "nan-divisor-log");
                assert!(worst_case_fuel_per_byte.is_nan());
            }
            other => panic!("expected InvalidWorstCaseFuelPerByte, got {other:?}"),
        }
    }

    #[test]
    fn test_BC_1_18_005_EC_015_load_accepts_valid_positive_worst_case_fuel_per_byte_control() {
        // POST-v1.12 MATCH-FIRST restructure (F-C1-P6-001): EC-015's
        // finiteness/positivity guard now lives in `validate_entry`, not
        // `ShardRegistry::load` — this control only pins that a well-formed,
        // finite, strictly-positive worst_case_fuel_per_byte with
        // shard_cap_bytes AT (not over) its own formula ceiling continues to
        // structurally parse via `load` successfully; it does NOT itself
        // exercise EC-015's guard (see the sibling
        // `validate_entry`-rejecting tests above for that guard's coverage).
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg_path = dir.path().join("shard-config.toml");
        std::fs::write(
            &cfg_path,
            "[[shard]]\n\
             artifact_stem = \"decision-log\"\n\
             artifact_path = \"decision-log.md\"\n\
             practical_fuel_ceiling = 8000000\n\
             worst_case_fuel_per_byte = 106.36\n\
             max_single_record_bytes = 16384\n\
             safety_margin = 8192\n\
             shard_cap_bytes = 49152\n\
             shape = \"flat\"\n",
        )
        .expect("write fixture");

        let result = ShardRegistry::load(&cfg_path);
        assert!(
            result.is_ok(),
            "control: a valid finite positive worst_case_fuel_per_byte with shard_cap_bytes <= \
             its own formula ceiling MUST load successfully. Got: {result:?}"
        );
    }

    // NEW (S-25.02 F4 comprehensive-sweep vacuity closure): the `load()`
    // control immediately above stopped exercising EC-015's finiteness/
    // positivity guard once that guard migrated to `validate_entry` at
    // v1.12 (its own comment says so) — it would still pass even if
    // `validate_entry` regressed the guard to reject a normal valid
    // positive `worst_case_fuel_per_byte`, because `load()` never calls
    // `validate_entry` at all. This test calls `validate_entry` DIRECTLY
    // with a normal valid positive `worst_case_fuel_per_byte` (the BC's own
    // 106.36 provisional calibration value) and an otherwise-valid entry
    // (`shard_cap_bytes` at its own formula ceiling), and asserts `Ok(())`.
    //
    // Non-vacuousness: PAIRED with
    // `test_BC_1_18_005_EC_015_validate_entry_rejects_zero_worst_case_fuel_per_byte_divisor_door`
    // and `..._rejects_nan_worst_case_fuel_per_byte` (0.0/NaN -> `Err`),
    // this test pins the accept side of the finiteness/positivity guard
    // against `validate_entry` itself: if the guard ever regressed to
    // reject finite positive values too (e.g. an inverted comparison), THIS
    // test is the one that would fail.
    #[test]
    fn test_BC_1_18_005_EC_015_validate_entry_accepts_valid_positive_worst_case_fuel_per_byte() {
        // A normal valid positive worst_case_fuel_per_byte (flat_entry's own
        // default, the BC's provisional 106.36 calibration value — see the
        // EC-015 rejection tests above for the 0.0/NaN degenerate cases this
        // pairs against). shard_cap_bytes is set to the entry's own computed
        // formula ceiling (not hardcoded) so this test exercises ONLY the
        // divisor guard, not the separate PC9 cap-vs-ceiling comparison
        // pinned by the sibling PC9 test above.
        let mut entry = flat_entry("valid-fuel-log", 0);
        assert!(
            entry.worst_case_fuel_per_byte.is_finite() && entry.worst_case_fuel_per_byte > 0.0,
            "fixture sanity: flat_entry's default worst_case_fuel_per_byte must itself be the \
             valid-positive case this test exercises"
        );
        entry.shard_cap_bytes = compute_shard_cap_bytes(&entry.cap_formula_inputs());

        let result = validate_entry(&entry);
        assert!(
            result.is_ok(),
            "EC-015: a normal valid positive worst_case_fuel_per_byte ({}) with an otherwise-valid \
             entry MUST be accepted by validate_entry itself — paired with the 0.0/NaN rejection \
             tests above, this pins the accept side of the finiteness/positivity guard against \
             validate_entry directly. Got: {result:?}",
            entry.worst_case_fuel_per_byte
        );
    }

    // ===================================================================
    // EC-017 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-5
    // finding F-C1-P5, MEDIUM/HIGH, BC-1.18.005 v1.11) — residual
    // divisor-door closure: a saturated COMPUTED CEILING, not merely an
    // illegal divisor. EC-015's `worst_case_fuel_per_byte.is_finite() &&
    // > 0.0` guard is necessary but insufficient — a legal tiny-positive
    // divisor (e.g. `1e-300`) satisfies that guard yet still drives
    // `compute_shard_cap_bytes`'s internal division
    // (`practical_fuel_ceiling as f64 / worst_case_fuel_per_byte`) to a
    // value so large that its `.floor() as u64` cast saturates to
    // `u64::MAX`, reproducing the identical failure mode EC-015's guard
    // exists to close: an oversized `shard_cap_bytes` sails past the
    // `CapExceedsFormulaCeiling` comparison because the computed ceiling
    // itself is meaninglessly large.
    //
    // `ShardRegistry::load` MUST, for every entry that passes the EC-015
    // guard and BEFORE the `CapExceedsFormulaCeiling` comparison is
    // evaluated, independently evaluate the RAW, pre-cast division result
    // `practical_fuel_ceiling as f64 / worst_case_fuel_per_byte` and reject
    // the entry if that raw f64 value is non-finite or `>= u64::MAX as
    // f64` — a NEW `ShardConfigError::FormulaCeilingSaturated` variant,
    // distinct from `InvalidWorstCaseFuelPerByte` (which catches an
    // illegal `<= 0.0`/non-finite DIVISOR itself, not a saturated RESULT
    // arising from an otherwise-legal tiny-positive divisor).
    // ===================================================================

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001): the
    // residual divisor-door / saturated-ceiling check now lives in
    // `validate_entry`, called at entry-MATCH time only. This test drives
    // `validate_entry` directly, preserving the exact assertion.
    #[test]
    fn test_BC_1_18_005_EC_017_validate_entry_rejects_saturating_formula_ceiling() {
        let mut entry = flat_entry("saturating-ceiling-log", 100_000_000);
        entry.worst_case_fuel_per_byte = 1e-300;

        // worst_case_fuel_per_byte = 1e-300 is finite and > 0.0, so it
        // PASSES the EC-015 guard. But the raw division
        // 8,000,000 / 1e-300 = ~8e306, which is >= u64::MAX as f64 — the
        // subsequent `.floor() as u64` cast in `compute_shard_cap_bytes`
        // saturates to u64::MAX, so the resulting ceiling would
        // comfortably exceed this fixture's already-enormous
        // shard_cap_bytes = 100,000,000, letting it sail past
        // Postcondition 9's `shard_cap_bytes > computed_ceiling` check via
        // a legal-but-degenerate divisor. The residual closure rejects the
        // entry BEFORE that comparison is ever attempted — and it MUST do
        // so via the specific `FormulaCeilingSaturated` variant (not merely
        // *some* error), mirroring EC-013/EC-015/EC-016's sibling pattern:
        // a generic `.is_err()` would still pass if a future regression
        // made this same input error out through a DIFFERENT variant (e.g.
        // if EC-015's guard were mistakenly widened to also catch
        // tiny-positive divisors), silently no longer proving the residual
        // FormulaCeilingSaturated path is exercised at all.
        let err = validate_entry(&entry).expect_err(
            "EC-017: worst_case_fuel_per_byte = 1e-300 is legal under EC-015's is_finite() && \
             > 0.0 guard, yet the raw division practical_fuel_ceiling as f64 / \
             worst_case_fuel_per_byte saturates the subsequent .floor() as u64 cast to \
             u64::MAX — validate_entry() MUST reject this entry (residual divisor-door \
             closure) BEFORE the CapExceedsFormulaCeiling comparison is even attempted, \
             regardless of the arbitrarily-large declared shard_cap_bytes.",
        );
        match err {
            ShardConfigError::FormulaCeilingSaturated {
                artifact_stem,
                practical_fuel_ceiling,
                worst_case_fuel_per_byte,
            } => {
                assert_eq!(artifact_stem, "saturating-ceiling-log");
                assert_eq!(practical_fuel_ceiling, 8_000_000);
                assert_eq!(worst_case_fuel_per_byte, 1e-300);
            }
            other => panic!("EC-017: expected FormulaCeilingSaturated, got {other:?}"),
        }
    }

    // ===================================================================
    // EC-016 (NEW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4
    // finding F-C1-P4-002, MEDIUM, BC-1.18.005 v1.10) — a
    // "frontmatter-changelog-array"-shaped [[shard]] entry that OMITS the
    // required `n` item-count trigger threshold MUST fail-loud at
    // ShardRegistry::load() time, not merely deferred to the first
    // gate-time write (shard_cap_gate_check's existing
    // HookResult::Error-on-None-`n` check runs too late relative to this
    // BC's own EC-009/EC-011/EC-013 load-time posture).
    //
    // POST-v1.12 MATCH-FIRST restructure (F-C1-P6-001): `validate_entry`
    // requires `n` for every `"frontmatter-changelog-array"`-shaped entry,
    // at entry-match time, via a `let Some(n) = entry.n else { return
    // Err(ShardConfigError::MissingN { artifact_stem }) }` guard, evaluated
    // BEFORE `low_water_mark` is examined — so an entry missing `n` AND
    // declaring an independently invalid `low_water_mark` is rejected on the
    // missing-`n` condition, not silently accepted. The two tests below pin
    // both the plain-omission case and the omitted-`n`-plus-invalid-
    // `low_water_mark` case.
    // ===================================================================

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001):
    // this test previously asserted EC-016's fail-loud outcome via
    // `ShardRegistry::load(path).is_err()` — the pre-v1.12 eager,
    // whole-file validation loop. Post-restructure, `ShardRegistry::load`
    // becomes structural-TOML-parse-only (an entry omitting `n`
    // deserializes fine, since `n: Option<u64>`) and this semantic check
    // moves to entry-MATCH time (`validate_entry`, called only on the
    // entry `find_matching_entry` resolves — never on sibling entries).
    // This test now drives the PUBLIC match-time gate path directly
    // (`shard_cap_gate_check`, constructing the `ShardEntry` in-memory
    // rather than round-tripping it through `load()`): `shard_cap_gate_check`
    // ALREADY carries this exact defensive check today inside its
    // `ShardShape::FrontmatterChangelogArray` arm (`let Some(n) = entry.n
    // else { return HookResult::Error { .. } }`), so this assertion is
    // GREEN now via that pre-existing gate-time path, and remains GREEN
    // post-restructure once the check is (re)homed inside `validate_entry`
    // — the OBSERVABLE outcome of a dispatch matching this entry is
    // unchanged either way.
    #[test]
    fn test_BC_1_18_005_EC_016_matched_entry_missing_n_is_fail_loud() {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = None; // EC-016: omitted entirely — fail-loud, never deferred.
        let registry = ShardRegistry {
            shards: vec![entry],
        };
        // Target path's stem ("BC-INDEX") MATCHES the malformed entry
        // itself — the "matched malformed entry" scenario EC-018's
        // blast-radius ruling explicitly preserves unchanged.
        let target = std::path::Path::new("BC-INDEX.md");

        let result = shard_cap_gate_check(
            &registry,
            "Edit",
            target,
            &serde_json::json!({"old_string": "a", "new_string": "ab"}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("BC-INDEX")
                        && message.contains("item-count trigger threshold"),
                    "EC-016: expected the missing-`n` diagnostic naming artifact_stem \
                     \"BC-INDEX\" and the item-count trigger threshold, got: {message}"
                );
            }
            other => panic!(
                "EC-016: a dispatch whose target MATCHES a \"frontmatter-changelog-array\"-shaped \
                 entry omitting the required `n` MUST be a fail-loud HookResult::Error, evaluated \
                 at match time, never deferred — got {other:?}"
            ),
        }
    }

    // MIGRATED (BC-1.18.005 v1.12 MATCH-FIRST restructure, F-C1-P6-001):
    // unlike the plain missing-`n` case migrated further above, THIS test
    // pins EC-016's ORDERING requirement — that a missing `n` is reported
    // even when the entry ALSO declares an independently-invalid
    // `low_water_mark`. `validate_entry` houses BOTH checks in the required
    // order (EC-016's `n`-presence check BEFORE EC-011's `low_water_mark`
    // range check), so this ordering guarantee is now driven directly
    // against `validate_entry`.
    #[test]
    fn test_BC_1_18_005_EC_016_validate_entry_rejects_missing_n_even_with_invalid_low_water_mark() {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = None;
        entry.low_water_mark = Some(-1);

        // entry.n is None here, so validate_entry's `let Some(n) = entry.n
        // else { ... }` guard fires and reports the missing-`n` condition
        // BEFORE EC-011's negative-low_water_mark check is ever reached for
        // this entry — per Postcondition 8's ordering requirement, MissingN
        // is the reported error even though low_water_mark = -1 is
        // independently invalid against any realistic N.
        let err = validate_entry(&entry).expect_err(
            "EC-016: an entry missing `n` AND declaring an independently-invalid low_water_mark \
             (-1) MUST still be rejected at entry-match time — with EC-016's guard ordered \
             before the low_water_mark check, the reported error is the missing-`n` condition, \
             but either way this entry must never silently pass validation.",
        );
        match err {
            ShardConfigError::MissingN { artifact_stem } => {
                assert_eq!(artifact_stem, "BC-INDEX");
            }
            other => panic!("expected MissingN, got {other:?}"),
        }
    }

    // ===================================================================
    // PR #818 fix-burst finding m2 — n = 0 fail-loud (item-count trigger
    // would fire unconditionally; any explicit low_water_mark unsatisfiable)
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_m2_validate_entry_rejects_zero_item_count_threshold() {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(0);
        entry.low_water_mark = None;

        let err = validate_entry(&entry).expect_err(
            "m2: an entry declaring n = 0 MUST fail loud at entry-match time — before this fix, \
             n = 0 passed validation, making the item-count trigger fire unconditionally on \
             every dispatch and making any explicit low_water_mark config unsatisfiable",
        );
        match err {
            ShardConfigError::ZeroItemCountThreshold { artifact_stem } => {
                assert_eq!(artifact_stem, "BC-INDEX");
            }
            other => panic!("expected ZeroItemCountThreshold, got {other:?}"),
        }
    }

    #[test]
    fn test_BC_1_18_005_m2_matched_entry_zero_n_is_fail_loud() {
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(0);
        let registry = ShardRegistry {
            shards: vec![entry],
        };
        let target = std::path::Path::new("BC-INDEX.md");

        let result = shard_cap_gate_check(
            &registry,
            "Edit",
            target,
            &serde_json::json!({"old_string": "a", "new_string": "ab"}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("BC-INDEX") && message.contains("n = 0"),
                    "m2: expected the ZeroItemCountThreshold diagnostic naming artifact_stem \
                     \"BC-INDEX\" and citing n = 0, got: {message}"
                );
            }
            other => panic!(
                "m2: a dispatch whose target MATCHES a \"frontmatter-changelog-array\"-shaped \
                 entry declaring n = 0 MUST be a fail-loud HookResult::Error — got {other:?}"
            ),
        }
    }

    #[test]
    fn test_BC_1_18_005_m2_validate_entry_accepts_n_equals_one() {
        // Control: n = 1 (the smallest legal positive threshold) MUST still
        // validate successfully — this fix rejects ONLY n = 0, never any
        // positive n.
        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(1);
        entry.low_water_mark = None;
        validate_entry(&entry)
            .expect("m2 control: n = 1 is a legal positive item-count threshold and MUST validate");
    }

    // ===================================================================
    // O-C1-P4-001 (LOW, S-25.02 Phase F4 LOCAL adversary cluster-1 pass-4
    // observation) — symmetry between the two `shard_cap_gate_check` shape
    // arms. The `"flat"` arm guards against a non-mutating `tool_name` via
    // `ToolKind::from_tool_name` BEFORE ever touching the target file
    // (Postcondition 1's zero-cost-bypass spirit, extended to an
    // unsupported tool kind). The `FrontmatterChangelogArray` arm mirrors
    // that same guard: it early-returns `Continue` for a
    // non-Edit/Write/MultiEdit `tool_name` BEFORE reading `entry.n` or
    // calling `read_changelog_item_count(target_path)`.
    //
    // This test drives that symmetry into an observable difference: the
    // target file's content is deliberately malformed (no well-formed `---`
    // frontmatter fence), so `read_changelog_item_count` would fail loud
    // with an `io::Error` if the FrontmatterChangelogArray arm ever reached
    // it for a non-mutating call. The guarded arm returns `Continue` for the
    // non-mutating `"Read"` tool_name WITHOUT ever reading the file, so the
    // malformed content is never observed.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_P4_frontmatter_arm_non_mutating_tool_continues() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("BC-INDEX.md");
        std::fs::write(&target, "no frontmatter fence at all\n").expect("write fixture");

        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        let registry = ShardRegistry {
            shards: vec![entry],
        };

        // The FrontmatterChangelogArray arm's tool_name guard early-returns
        // Continue for the non-mutating "Read" tool_name WITHOUT ever
        // calling read_changelog_item_count(target_path), so the fixture's
        // deliberately malformed (fence-less) frontmatter is never observed.
        let result = shard_cap_gate_check(&registry, "Read", &target, &serde_json::json!({}));

        assert_eq!(
            result,
            HookResult::Continue,
            "O-C1-P4-001: a non-mutating tool_name (\"Read\") reaching the \
             FrontmatterChangelogArray arm MUST Continue WITHOUT reading the target file, \
             mirroring the \"flat\" arm's ToolKind::from_tool_name guard — got {result:?} instead \
             (the ungated arm proceeded to read_changelog_item_count and surfaced the fixture's \
             deliberately malformed frontmatter as a fail-loud HookResult::Error)"
        );
    }

    // ===================================================================
    // PR #818 fix-burst finding m3 / M-1 — malformed/absent REQUIRED
    // payload fields MUST fail loud, never silently compute a 0-byte/
    // 0-delta size that lets the cap trigger go unguarded.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_m3_write_missing_content_field_is_fail_loud() {
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let target = Path::new("/repo/.factory/decision-log.md");

        // Before the fix: an empty tool_input silently computed content_len
        // = 0, so this Write would have Continue'd — a malformed payload
        // silently unguarded. After the fix: fail loud.
        let result = shard_cap_gate_check(&registry, "Write", target, &serde_json::json!({}));

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("decision-log") && message.contains("\"content\""),
                    "m3: expected a diagnostic naming artifact_stem \"decision-log\" and the \
                     missing \"content\" field, got: {message}"
                );
            }
            other => panic!(
                "m3: a Write with tool_input missing the required \"content\" field MUST be a \
                 fail-loud HookResult::Error, never a silent Continue — got {other:?}"
            ),
        }
    }

    #[test]
    fn test_BC_1_18_005_m3_write_non_string_content_field_is_fail_loud() {
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let target = Path::new("/repo/.factory/decision-log.md");

        // "content" present but a JSON number, not a string.
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            target,
            &serde_json::json!({"content": 42}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("decision-log") && message.contains("\"content\""),
                    "m3: expected a diagnostic naming artifact_stem \"decision-log\" and the \
                     non-string \"content\" field, got: {message}"
                );
            }
            other => panic!(
                "m3: a Write with a non-string \"content\" field MUST be a fail-loud \
                 HookResult::Error, never a silent 0-byte Continue — got {other:?}"
            ),
        }
    }

    #[test]
    fn test_BC_1_18_005_m3_edit_missing_fields_is_fail_loud() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("decision-log.md");
        std::fs::write(&target, "x".repeat(40_000)).expect("write fixture shard");
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };

        // Before the fix: an empty tool_input silently computed old_len =
        // new_len = 0, so this Edit would have Continue'd — a malformed
        // payload silently unguarded. After the fix: fail loud.
        let result = shard_cap_gate_check(&registry, "Edit", &target, &serde_json::json!({}));

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("decision-log") && message.contains("old_string"),
                    "m3: expected a diagnostic naming artifact_stem \"decision-log\" and the \
                     missing \"old_string\" field, got: {message}"
                );
            }
            other => panic!(
                "m3: an Edit with tool_input missing the required old_string/new_string fields \
                 MUST be a fail-loud HookResult::Error, never a silent Continue — got {other:?}"
            ),
        }
    }

    #[test]
    fn test_BC_1_18_005_m3_multi_edit_missing_edits_array_is_fail_loud() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("decision-log.md");
        std::fs::write(&target, "x".repeat(40_000)).expect("write fixture shard");
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };

        // Before the fix: an empty tool_input silently treated the absent
        // "edits" array as zero edit blocks (net_delta = 0), so this
        // MultiEdit would have Continue'd. After the fix: fail loud.
        let result = shard_cap_gate_check(&registry, "MultiEdit", &target, &serde_json::json!({}));

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("decision-log") && message.contains("\"edits\""),
                    "m3: expected a diagnostic naming artifact_stem \"decision-log\" and the \
                     missing \"edits\" array, got: {message}"
                );
            }
            other => panic!(
                "m3: a MultiEdit with tool_input missing the required \"edits\" array MUST be a \
                 fail-loud HookResult::Error, never a silent Continue — got {other:?}"
            ),
        }
    }

    #[test]
    fn test_BC_1_18_005_m3_multi_edit_malformed_edit_block_is_fail_loud() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("decision-log.md");
        std::fs::write(&target, "x".repeat(40_000)).expect("write fixture shard");
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };

        // "edits" is present and an array, but its single block is missing
        // "new_string" — this must still fail loud, not silently treat that
        // block's new_len as 0.
        let result = shard_cap_gate_check(
            &registry,
            "MultiEdit",
            &target,
            &serde_json::json!({"edits": [{"old_string": "a"}]}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("decision-log") && message.contains("new_string"),
                    "m3: expected a diagnostic naming artifact_stem \"decision-log\" and the \
                     missing \"new_string\" field, got: {message}"
                );
            }
            other => panic!(
                "m3: a MultiEdit edit block missing \"new_string\" MUST be a fail-loud \
                 HookResult::Error, never a silent 0-byte Continue — got {other:?}"
            ),
        }
    }

    #[test]
    fn test_BC_1_18_005_m3_multi_edit_empty_edits_array_is_not_malformed() {
        // Control: an EMPTY "edits" array is legitimate (net_delta = 0, a
        // no-op MultiEdit), not malformed — it MUST NOT be rejected the same
        // way an ABSENT/non-array "edits" field is.
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("decision-log.md");
        std::fs::write(&target, "x".repeat(5_000)).expect("write fixture shard");
        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };

        let result = shard_cap_gate_check(
            &registry,
            "MultiEdit",
            &target,
            &serde_json::json!({"edits": []}),
        );

        assert_eq!(
            result,
            HookResult::Continue,
            "m3 control: an EMPTY edits array is a legitimate no-op MultiEdit, not a malformed \
             payload — it MUST Continue, not fail loud"
        );
    }

    // ===================================================================
    // PR #818 fix-burst finding B1 — a present-but-fenceless frontmatter
    // target file MUST be permissive (Ok(0)), never a fail-loud io::Error.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_B1_read_changelog_item_count_no_fence_at_all_is_ok_zero() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        std::fs::write(&path, "no frontmatter fence at all\n").expect("write fixture");

        let count = read_changelog_item_count(&path).expect(
            "B1: a present-but-fenceless file MUST be Ok(0), not a fail-loud io::Error — a \
             malformed/absent fence on an EXISTING file must not hard-block a legitimate write",
        );
        assert_eq!(count, 0);
    }

    #[test]
    fn test_BC_1_18_005_B1_read_changelog_item_count_no_closing_fence_is_ok_zero() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        // Opening fence present, but no closing "---" line anywhere.
        std::fs::write(
            &path,
            "---\ntitle: \"BC-INDEX\"\nchangelog: []\n# no closing fence\n",
        )
        .expect("write fixture");

        let count = read_changelog_item_count(&path).expect(
            "B1: an opening fence with no well-formed closing fence MUST be Ok(0), not a \
             fail-loud io::Error",
        );
        assert_eq!(count, 0);
    }

    #[test]
    fn test_BC_1_18_005_B1_shard_cap_gate_check_no_fence_write_continues() {
        // Full-stack: a Write against an EXISTING, present-but-fenceless
        // frontmatter-changelog-array-shaped target MUST Continue, never
        // hard-block as HookResult::Error.
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("BC-INDEX.md");
        std::fs::write(&target, "no frontmatter fence at all\n").expect("write fixture");

        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        let registry = ShardRegistry {
            shards: vec![entry],
        };

        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &target,
            &serde_json::json!({"content": "new content"}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "B1: a Write against an EXISTING, present-but-fenceless target MUST Continue, never \
             hard-block a legitimate write as HookResult::Error — got {result:?}"
        );
    }

    // ===================================================================
    // PR #818 fix-burst finding M-1 (BC-1.18.005 v1.13, EC-021) — a
    // fence-bearing target whose YAML content fails to parse MUST remain
    // fail-loud (never coerced to Ok(0) the way EC-020's fenceless cases
    // are), AND the resulting error message MUST contain both (i) the
    // underlying YAML-parse-cause text and (ii) escape-hatch guidance
    // naming the gate's Edit/Write/MultiEdit-only scope, so the operator
    // is not left in a self-deadlock with no discoverable way out.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_EC_021_read_changelog_item_count_malformed_yaml_is_fail_loud() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        // A well-formed `---`/`---` fence pair, but the YAML content
        // between the fences is malformed (unbalanced quote on the
        // `title` value) — this is NOT the same shape as B1's fenceless
        // case: a real frontmatter block exists here, it just fails to
        // parse.
        std::fs::write(
            &path,
            "---\ntitle: \"unbalanced quote\nchangelog:\n  - version: \"1.0\"\n---\n\n# Body\n",
        )
        .expect("write fixture");

        let err = read_changelog_item_count(&path).expect_err(
            "EC-021: a fence-bearing target whose YAML content fails to parse MUST propagate a \
             fail-loud io::Error — it MUST NEVER be coerced to Ok(0) the way EC-014's NotFound \
             or EC-020's fenceless cases are, since Ok(0) here would silently undercount a \
             changelog: sequence whose true size is unknown and possibly large",
        );
        assert_eq!(
            err.kind(),
            io::ErrorKind::InvalidData,
            "EC-021: the malformed-YAML fail-loud error MUST be io::ErrorKind::InvalidData"
        );

        let message = err.to_string();
        assert!(
            message.contains("parse"),
            "EC-021: the error message MUST name the YAML-parse cause specifically (expected a \
             substring naming the parse failure) — got: {message}"
        );
        assert!(
            message.contains("Edit") && message.contains("Write") && message.contains("MultiEdit"),
            "EC-021: the error message MUST contain escape-hatch guidance naming this gate's \
             Edit/Write/MultiEdit-only scope, so the operator can discover that repairing the \
             frontmatter via any OTHER means (e.g. a Bash-invoked edit) escapes the \
             self-deadlock a fail-loud result on every gated edit would otherwise create — got: \
             {message}"
        );
    }

    #[test]
    fn test_BC_1_18_005_EC_021_shard_cap_gate_check_malformed_yaml_write_is_hook_error() {
        // Full-stack: a Write against an EXISTING, fence-bearing-but
        // -malformed-YAML frontmatter-changelog-array-shaped target MUST
        // surface HookResult::Error (fail-loud), never Continue.
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("BC-INDEX.md");
        std::fs::write(
            &target,
            "---\ntitle: \"unbalanced quote\nchangelog:\n  - version: \"1.0\"\n---\n\n# Body\n",
        )
        .expect("write fixture");

        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        let registry = ShardRegistry {
            shards: vec![entry],
        };

        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &target,
            &serde_json::json!({"content": "new content"}),
        );
        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("parse"),
                    "EC-021: the HookResult::Error message MUST name the YAML-parse cause \
                     specifically — got: {message}"
                );
                assert!(
                    message.contains("Edit")
                        && message.contains("Write")
                        && message.contains("MultiEdit"),
                    "EC-021: the HookResult::Error message MUST contain escape-hatch guidance \
                     naming this gate's Edit/Write/MultiEdit-only scope — got: {message}"
                );
            }
            other => panic!(
                "EC-021: a Write against a fence-bearing-but-malformed-YAML target MUST surface \
                 HookResult::Error (fail-loud), never Continue — got {other:?}"
            ),
        }
    }

    // ===================================================================
    // PR #818 fix-burst finding B2 — bounded read: a target file exceeding
    // MAX_CHANGELOG_TARGET_READ_BYTES MUST fail loud WITHOUT reading its
    // full content.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_B2_read_changelog_item_count_oversized_file_fails_loud() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        // One byte over the ceiling — the M-2 bounded read (a single capped
        // `Read::take(MAX_CHANGELOG_TARGET_READ_BYTES + 1)` on an already-open
        // file handle, not a separate stat()/metadata() call) must be enough
        // to reject this without ever reading the file's full content into
        // memory.
        let oversized_len = MAX_CHANGELOG_TARGET_READ_BYTES + 1;
        let file = std::fs::File::create(&path).expect("create fixture");
        file.set_len(oversized_len).expect("set fixture length");
        drop(file);

        let err = read_changelog_item_count(&path).expect_err(
            "B2: a target file exceeding MAX_CHANGELOG_TARGET_READ_BYTES MUST fail loud rather \
             than being read into memory in full",
        );
        assert_eq!(
            err.kind(),
            io::ErrorKind::FileTooLarge,
            "B2: expected io::ErrorKind::FileTooLarge, got {:?}: {err}",
            err.kind()
        );
    }

    #[test]
    fn test_BC_1_18_005_B2_read_changelog_item_count_at_ceiling_is_not_rejected_by_size_alone() {
        // Control: a file exactly AT the ceiling (well-formed, empty
        // changelog) must NOT be rejected by the size guard — only content
        // strictly GREATER than the ceiling fails loud.
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        let body = "---\ntitle: \"BC-INDEX\"\nchangelog: []\n---\n";
        let padding_len = (MAX_CHANGELOG_TARGET_READ_BYTES as usize).saturating_sub(body.len());
        let mut content = String::with_capacity(body.len() + padding_len + 1);
        // Pad INSIDE a YAML comment line appended after the closing fence so
        // the frontmatter block itself stays well-formed.
        content.push_str(body);
        content.push_str("# ");
        content.push_str(&"x".repeat(padding_len.saturating_sub(2)));
        // Trim/pad to land exactly at the ceiling.
        content.truncate(MAX_CHANGELOG_TARGET_READ_BYTES as usize);
        std::fs::write(&path, &content).expect("write fixture");
        assert_eq!(
            std::fs::metadata(&path).expect("stat fixture").len(),
            MAX_CHANGELOG_TARGET_READ_BYTES,
            "precondition: fixture must be exactly at the ceiling"
        );

        let count = read_changelog_item_count(&path)
            .expect("B2 control: a file exactly AT the ceiling MUST NOT be rejected by size alone");
        assert_eq!(count, 0);
    }

    // ===================================================================
    // PR #818 fix-burst finding n3 — line-anchored closing-fence detection,
    // not a naive "\n---" substring search.
    // ===================================================================

    #[test]
    fn test_BC_1_18_005_n3_block_scalar_line_starting_with_dashes_is_not_mistaken_for_fence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        // A YAML block-scalar value contains a line that STARTS WITH "---"
        // but is NOT itself a bare "---" line — the naive `"\n---"`
        // substring search would truncate the frontmatter block here,
        // silently losing the two real changelog items below it.
        std::fs::write(
            &path,
            "---\n\
             title: \"BC-INDEX\"\n\
             notes: |\n\
             \x20\x20---this line starts with dashes but is not a fence---\n\
             \x20\x20another line\n\
             changelog:\n\
             \x20\x20- version: \"1.0\"\n\
             \x20\x20- version: \"1.1\"\n\
             ---\n\n# Body\n",
        )
        .expect("write fixture");

        let count = read_changelog_item_count(&path).expect(
            "n3: a line-anchored closing-fence search must not be fooled by a block-scalar line \
             that merely starts with \"---\"",
        );
        assert_eq!(
            count, 2,
            "n3: the real changelog: array (2 items) must be counted correctly, not truncated by \
             a false-positive \"\\n---\" substring match inside the notes: block scalar"
        );
    }

    #[test]
    fn test_BC_1_18_005_n3_four_dash_line_is_not_mistaken_for_fence() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("BC-INDEX.md");
        // A frontmatter line that STARTS WITH FOUR DASHES, unindented,
        // immediately after a newline -- here a plain YAML mapping key
        // composed of four dashes -- but is NOT itself the three-dash
        // closing fence. The naive `"\n---"` substring search this
        // line-anchored check replaces would match the newline immediately
        // preceding this line (its first three characters are also
        // "-", "-", "-") and truncate the frontmatter block right there,
        // silently dropping the `changelog:` section (and the item within
        // it) entirely. A correct line-anchored check compares the WHOLE
        // line to "---" and correctly treats this four-dash line as
        // ordinary content, finding the real closing fence below and
        // preserving `changelog:`.
        std::fs::write(
            &path,
            "---\n\
             title: \"BC-INDEX\"\n\
             ----: \"a four-dash mapping key, not the closing fence\"\n\
             changelog:\n\
             \x20\x20- version: \"1.0\"\n\
             ---\n\n# Body\n",
        )
        .expect("write fixture");

        let count = read_changelog_item_count(&path).expect(
            "n3: a \"----\" (four-dash) line must not be mistaken for the three-dash closing \
             fence",
        );
        assert_eq!(
            count, 1,
            "n3: the real changelog: array (1 item) must be counted correctly -- a naive \
             \"\\n---\" substring search would truncate the frontmatter block at the \
             four-dash line above, losing the changelog: section entirely and yielding 0 \
             instead of 1"
        );
    }
}

// ===========================================================================
// BC-1.18.006 — Roll-Before-Write via Block-and-Retry (S-25.02 cluster-2
// "roll") — implementer's unit-test coverage for the Red Gate suite
// test-writer authored.
//
// # BC-5.38.001 Red Gate discipline — GREEN (all functions implemented)
//
// Every function this module exercises (`execute_roll`,
// `read_canonical_content`, `publish_sealed_shard`,
// `truncate_canonical_to_empty`, `publish_shard_index_update`,
// `self_heal_resume_from_truncate`, `self_heal_reconcile_missing_index_entries`,
// `reconcile_post_write_replace_all_overcap`, `reconcile_leading_probe_backstop`)
// is now fully implemented — the stub-architect's original `todo!()`
// placeholders (commit `e04ab76f`) have all been replaced with real logic
// per BC-1.18.006's postconditions (see the "BC-5.38.001 Red Gate
// discipline — IMPLEMENTED" section comment above `ShardIndexEntry`). Every
// test below passes. Each test asserts the REAL, post-implementation
// expected outcome (never `#[should_panic]`) — the same methodology this
// file's own `mod tests` (cluster-1) already establishes.
//
// `build_roll_retry_block_reason` (GREEN-BY-DESIGN) and
// `From<ShardRollError> for HookResult` (WIRING-EXEMPT) are intentionally
// NOT given standalone unit tests here, mirroring cluster-1's own exclusion
// of `ShardEntry::cap_formula_inputs`/`From<ShardConfigError> for HookResult`
// for the identical reason (trivial, already-real code, outside this
// cluster's tested trigger/roll logic) — AC-007's message-content coverage
// instead lives in the integration test file
// (`tests/bc_1_18_006_roll_test.rs`), which drives the FULL dispatch path
// (trigger fires -> execute_roll -> Block) end-to-end, rather than
// exercising build_roll_retry_block_reason in isolation.
#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod bc_1_18_006_roll_tests {
    use super::*;

    /// Same shape as the sibling `mod tests`'s own `flat_entry` helper
    /// (private to that module) — duplicated here rather than shared across
    /// modules, to keep this cluster's test module independent of
    /// cluster-1's internal test-only surface.
    fn flat_entry(stem: &str, shard_cap_bytes: u64) -> ShardEntry {
        ShardEntry {
            artifact_stem: stem.to_string(),
            artifact_path: format!("{stem}.md"),
            practical_fuel_ceiling: 8_000_000,
            worst_case_fuel_per_byte: 106.36,
            max_single_record_bytes: 16_384,
            safety_margin: 8_192,
            shard_cap_bytes,
            shape: Some(ShardShape::Flat),
            n: None,
            low_water_mark: None,
        }
    }

    fn sealed_path_for(dir: &std::path::Path, stem: &str, seq: u32) -> std::path::PathBuf {
        dir.join(format!("{stem}.{seq:04}.md"))
    }

    fn index_path_for(dir: &std::path::Path, stem: &str) -> std::path::PathBuf {
        dir.join(format!("{stem}.shard-index.toml"))
    }

    // -----------------------------------------------------------------
    // Step (a) — read_canonical_content (Postcondition 1 step (a))
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_P1a_read_canonical_content_returns_full_existing_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("decision-log.md");
        std::fs::write(&path, "pre-roll content, 30 bytes ---").expect("seed canonical");

        let content = read_canonical_content(&path).expect(
            "BC-1.18.006 Postcondition 1 step (a): must read the canonical file's full current \
             content",
        );
        assert_eq!(
            content,
            "pre-roll content, 30 bytes ---".as_bytes(),
            "read_canonical_content must return the EXACT pre-roll bytes, not a truncated or \
             re-encoded copy"
        );
    }

    // F-C2-P7-001 (MAJOR, cluster-2 LOCAL adversary pass-7) — REPLACES the
    // withdrawn `test_BC_1_18_006_P1a_read_canonical_content_missing_file_is_io_error`,
    // which MISREAD BC-1.18.006 Precondition 2. Precondition 2 states the
    // current shard "exists (or is treated as a zero-byte current shard per
    // BC-1.18.005 EC-004 if this is the artifact's first-ever write)" — a
    // MISSING canonical at roll time is the artifact's legitimate
    // first-ever-write case, NOT a genuine I/O error. The withdrawn test's
    // own rationale ("a roll can only be triggered against an EXISTING
    // over-cap shard") is exactly the misreading this replacement closes:
    // Postcondition 1's `Ok(None)` empty-canonical short-circuit (F-C2-P3-001)
    // already exists for a 0-byte EXISTING canonical: a MISSING canonical
    // must resolve identically, via `read_canonical_content` treating
    // `NotFound` as `Ok(vec![])`, never propagating it as an `Err` that
    // `execute_roll` would otherwise map to `ShardRollError::SealWriteFailed`
    // (E-SHD-001).
    #[test]
    fn test_BC_1_18_006_P1a_FC2P7_001_read_canonical_content_missing_file_is_treated_as_zero_byte()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("does-not-exist.md");

        let content = read_canonical_content(&path).expect(
            "BC-1.18.006 Precondition 2 (F-C2-P7-001, MAJOR): a MISSING canonical file at roll \
             time — the artifact's first-ever write (BC-1.18.005 EC-004) — MUST be treated as a \
             zero-byte current shard, per Precondition 2's explicit '...or is treated as a \
             zero-byte current shard... if this is the artifact's first-ever write' clause. This \
             is NOT a genuine I/O error condition: read_canonical_content must return Ok(vec![]) \
             for a NotFound canonical, never propagate it as an Err (which execute_roll would \
             otherwise map to ShardRollError::SealWriteFailed / E-SHD-001).",
        );
        assert!(
            content.is_empty(),
            "a missing canonical must read back as exactly zero bytes — never any fabricated \
             content"
        );
    }

    /// Companion to the test above (F-C2-P7-001): Precondition 2's
    /// "treated as a zero-byte current shard" relief is scoped ONLY to a
    /// missing (`NotFound`) canonical — a genuine OTHER I/O failure must
    /// still propagate as a real `Err`, never be silently swallowed into
    /// `Ok(vec![])`.
    ///
    /// S-25.02 cluster-2 (windows-x64 CI failure #2): the fixture producing
    /// this "genuine non-NotFound" failure is platform-specific. On Unix,
    /// reading THROUGH a path component that is a plain file (not a
    /// directory) fails with `NotADirectory`/`ENOTDIR`. On Windows the SAME
    /// layout collapses to `ErrorKind::NotFound` (`ERROR_PATH_NOT_FOUND`) —
    /// this test's own SANITY precondition (the fixture's error must NOT be
    /// `NotFound`) is FALSE there, so the Unix fixture cannot be reused
    /// as-is (see `is_genuinely_missing`'s doc comment for why). Per the
    /// portable-fix research
    /// (`.factory/code-delivery/S-25.02/windows-path-semantics-research.md`
    /// §"Test-fixture portability recommendation" option 2), Windows
    /// instead uses a deterministic, filesystem-independent fixture: a path
    /// containing an illegal filename character (`<`), which `CreateFileW`
    /// rejects with `ERROR_INVALID_NAME` (123) -> `ErrorKind::InvalidFilename`
    /// — a kind that never collapses to `NotFound` on any platform. Both
    /// platforms then assert the SAME semantic outcome: the error
    /// propagates and is genuinely non-`NotFound`.
    #[test]
    fn test_BC_1_18_006_P1a_read_canonical_content_genuine_non_notfound_io_error_still_propagates()
    {
        let dir = tempfile::tempdir().expect("tempdir");

        #[cfg(not(windows))]
        let path = {
            let not_a_dir = dir.path().join("this-is-a-plain-file");
            std::fs::write(&not_a_dir, "i am a file, not a directory").expect("seed a plain file");
            // Reading THROUGH a path component that is a plain file (not a
            // directory) fails with NotADirectory/ENOTDIR on Unix.
            not_a_dir.join("decision-log.md")
        };
        #[cfg(windows)]
        let path = {
            // An illegal filename character deterministically produces
            // ERROR_INVALID_NAME -> ErrorKind::InvalidFilename on Windows,
            // which never collapses to NotFound the way the Unix
            // traverse-through-a-file fixture above does.
            dir.path().join("decision<log.md")
        };

        let result = read_canonical_content(&path);
        let err = result.expect_err(
            "a genuine non-NotFound I/O error must still propagate as Err — Precondition 2's \
             zero-byte-current-shard treatment is scoped ONLY to a missing canonical, never to \
             every I/O failure",
        );
        assert_ne!(
            err.kind(),
            io::ErrorKind::NotFound,
            "sanity: this fixture's I/O failure mode must NOT be NotFound (that case is already \
             covered by the sibling test above) — it must be a genuine other-kind failure"
        );
    }

    /// F-C2-P7-001: the E-SHD-001 mapping itself (`execute_roll`'s own step
    /// (a) error-wrapping) must still fire for a genuine non-NotFound read
    /// failure — only a MISSING (NotFound) canonical gets the zero-byte
    /// short-circuit relief; this is the "Keep a distinct test that a
    /// genuine non-NotFound io error still surfaces E-SHD-001" half of the
    /// Red Gate.
    #[test]
    fn test_BC_1_18_006_ESHD001_FC2P7_001_execute_roll_genuine_non_notfound_read_error_maps_to_seal_write_failed()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let not_a_dir = dir.path().join("this-is-a-plain-file");
        std::fs::write(&not_a_dir, "i am a file, not a directory").expect("seed a plain file");
        let canonical_path = not_a_dir.join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);

        let err = execute_roll(&entry, &canonical_path, false).expect_err(
            "a genuine non-NotFound I/O failure at step (a)'s read must still fail loud, never \
             silently resolve to Ok(None) the way a genuinely-missing (NotFound) canonical does",
        );
        assert!(
            matches!(err, ShardRollError::SealWriteFailed { .. }),
            "F-C2-P7-001: a genuine non-NotFound step (a) read failure MUST still map to \
             ShardRollError::SealWriteFailed (E-SHD-001) — Precondition 2's 'treated as a \
             zero-byte current shard' relief is scoped ONLY to a missing (NotFound) canonical, \
             never to every I/O failure. Got: {err:?}"
        );
    }

    /// F-C2-P7-001 (MAJOR): the observable `execute_roll` outcome for a
    /// MISSING canonical (the artifact's first-ever write) must be
    /// Postcondition 1's `Ok(None)` empty-canonical short-circuit — the SAME
    /// outcome an EXISTING 0-byte canonical already produces (F-C2-P3-001) —
    /// performing ZERO writes (no sealed shard, no canonical file
    /// fabricated, no index entry).
    #[test]
    fn test_BC_1_18_006_P1_FC2P7_001_execute_roll_missing_canonical_short_circuits_to_ok_none() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        assert!(
            !canonical_path.exists(),
            "test setup: the canonical file must NOT exist — this is the artifact's first-ever \
             write"
        );
        let entry = flat_entry("decision-log", 49_152);

        let result = execute_roll(&entry, &canonical_path, false);
        assert!(
            matches!(result, Ok(None)),
            "BC-1.18.006 Precondition 2 / F-C2-P7-001 (MAJOR): a MISSING canonical file (the \
             artifact's first-ever write, BC-1.18.005 EC-004) must be treated as a zero-byte \
             current shard, resolving to Postcondition 1's empty-canonical Ok(None) \
             short-circuit — NEVER a ShardRollError::SealWriteFailed (E-SHD-001) Error. Got: \
             {result:?}"
        );

        assert!(
            !canonical_path.exists(),
            "F-C2-P7-001: the empty-canonical short-circuit performs ZERO writes (Postcondition \
             1's 'skips ALL FOUR steps') — no canonical file may be fabricated as a side effect \
             of a missing-canonical roll attempt"
        );
        let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
        assert!(
            !sealed_path.exists(),
            "F-C2-P7-001: no sealed shard may be published for the missing-canonical \
             short-circuit — there is no pre-existing content to preserve"
        );
        let index_path = index_path_for(dir.path(), "decision-log");
        assert!(
            !index_path.exists(),
            "F-C2-P7-001: no shard-index entry may be published for the missing-canonical \
             short-circuit"
        );
    }

    // -----------------------------------------------------------------
    // Step (b) — publish_sealed_shard (Postcondition 1 step (b))
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_P1b_publish_sealed_shard_creates_new_file_with_exact_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sealed_path = dir.path().join("decision-log.0001.md");

        publish_sealed_shard(
            &sealed_path,
            "sealed content, byte-for-byte copy".as_bytes(),
        )
        .expect(
            "BC-1.18.006 Postcondition 1 step (b): publish_sealed_shard must succeed when the \
             destination does not yet exist",
        );

        let on_disk = std::fs::read_to_string(&sealed_path).expect("read sealed shard back");
        assert_eq!(
            on_disk, "sealed content, byte-for-byte copy",
            "the sealed shard's on-disk content must be an exact byte-for-byte copy of the \
             content passed to publish_sealed_shard"
        );
    }

    #[test]
    fn test_BC_1_18_006_ESHD001_publish_sealed_shard_failure_maps_to_seal_write_failed() {
        let dir = tempfile::tempdir().expect("tempdir");
        // A sealed_path whose PARENT directory does not exist: write_atomic's
        // temp-file-then-rename primitive cannot create either the temp file
        // or the destination — a genuine, unrecoverable-without-retry I/O
        // failure that must map to E-SHD-001 (steps (a)-(b) failure leg),
        // never silently swallowed or misreported as E-SHD-006/E-SHD-007.
        let sealed_path = dir
            .path()
            .join("no-such-subdir")
            .join("decision-log.0001.md");

        let err = publish_sealed_shard(&sealed_path, "content".as_bytes()).expect_err(
            "a write into a non-existent parent directory must fail, not silently succeed",
        );
        assert!(
            matches!(err, ShardRollError::SealWriteFailed { .. }),
            "BC-1.18.006 Postcondition 1 steps (a)-(b) failure MUST map to \
             ShardRollError::SealWriteFailed (E-SHD-001) specifically, not a generic error or a \
             different named variant — got: {err:?}"
        );
    }

    // -----------------------------------------------------------------
    // BC-1.18.006 v1.9 Postcondition 8's 0-byte-destination exception
    // (cluster-2 LOCAL adversary pass-8, F-C2-P8-002, MEDIUM; EC-024/EC-025)
    // — RED GATE (pass-8 fix-burst): `publish_sealed_shard` is currently
    // UNCONDITIONALLY write-once (any pre-existing destination, 0 bytes or
    // not, refuses via `E-SHD-009`/`ShardRollError::SealedShardAlreadyExists`
    // — see `write_exclusive`/`publish_sealed_shard` above, which has NO
    // 0-byte-reclaim branch yet). Per BC v1.9, a 0-byte pre-existing
    // destination MUST instead be reclaimed (`stat()` once, unlink if
    // exactly 0 bytes, retry `write_exclusive` exactly ONCE — never a loop)
    // rather than refused, while a NON-EMPTY pre-existing destination MUST
    // continue to refuse loudly (write-once immutability is unweakened for
    // real, non-empty sealed history). These two tests currently FAIL
    // against HEAD (both collide and both surface `E-SHD-009` today) for the
    // 0-byte case specifically — the non-empty case is the regression guard
    // that must stay green once the 0-byte branch is added.
    // -----------------------------------------------------------------

    /// EC-025 (F-C2-P8-002) reclaim-success test — drives the FULL
    /// `execute_roll` staged sequence (not just `publish_sealed_shard` in
    /// isolation) so this test is load-bearing on the entire roll outcome
    /// BC-1.18.006's own TEST ROUTED text asks for: the roll must SUCCEED
    /// (`Ok(Some(_))`), the sealed shard must durably hold the exact
    /// pre-roll canonical bytes (not left empty, not partially written), and
    /// the shard-index must have advanced — never `E-SHD-009`.
    #[test]
    fn test_BC_1_18_006_EC025_FC2P8_002_execute_roll_reclaims_zero_byte_destination_and_succeeds() {
        let dir = tempfile::tempdir().expect("tempdir");
        let entry = flat_entry("decision-log", 49_152);
        let canonical_path = dir.path().join("decision-log.md");
        let pre_roll_content = "a".repeat(50_000);
        std::fs::write(&canonical_path, &pre_roll_content).expect("seed over-cap canonical");

        // A 0-byte file already occupies the next-expected seq path
        // (seq=1) — e.g. an external anomaly, or a `next_seal_seq` that
        // self-heal's Invariant 9 skip-and-warn guard left permanently
        // un-indexed (BC-1.18.006 v1.9 Postcondition 8's 0-byte-destination
        // exception).
        let sealed_seq1 = sealed_path_for(dir.path(), "decision-log", 1);
        std::fs::write(&sealed_seq1, "").expect("seed 0-byte collision at seq=1");
        assert_eq!(
            std::fs::metadata(&sealed_seq1)
                .expect("stat seeded file")
                .len(),
            0,
            "precondition: the seeded seq=1 file must be exactly 0 bytes"
        );

        let result = execute_roll(&entry, &canonical_path, false);

        let new_entry = match result {
            Ok(Some(new_entry)) => new_entry,
            other => panic!(
                "BC-1.18.006 v1.9 Postcondition 8 0-byte-destination exception (EC-025, \
                 F-C2-P8-002, MEDIUM): a 0-byte pre-existing file at the next-expected seq path \
                 MUST be reclaimed (unlink + a SINGLE write_exclusive retry) rather than \
                 refused — execute_roll must succeed (Ok(Some(_))), NEVER \
                 ShardRollError::SealedShardAlreadyExists (E-SHD-009). Got: {other:?}"
            ),
        };

        assert_eq!(
            new_entry.seq, 1,
            "EC-025: the reclaimed 0-byte file's own seq (1) must be reused, never skipped past"
        );

        let sealed_content = std::fs::read_to_string(&sealed_seq1).expect(
            "EC-025: the reclaimed seq=1 path must now hold the durable sealed content on disk",
        );
        assert_eq!(
            sealed_content, pre_roll_content,
            "EC-025: after reclaiming the 0-byte collision, the sealed shard must contain the \
             EXACT pre-roll canonical bytes — never left empty, never partially written"
        );
        assert_eq!(
            new_entry.bytes_at_seal,
            pre_roll_content.len() as u64,
            "EC-025: the recovered index entry's bytes_at_seal must reflect the real sealed \
             content length"
        );

        let index_path = index_path_for(dir.path(), "decision-log");
        let index_toml = std::fs::read_to_string(&index_path)
            .expect("EC-025: the shard-index must be published after a successful reclaim+roll");
        let index: ShardIndex = toml::from_str(&index_toml).expect("index must be valid TOML");
        assert_eq!(
            index.shards.len(),
            1,
            "EC-025: exactly one [[shard]] entry — the reclaimed seq=1 seal — the index must \
             have ADVANCED, not been left stale or duplicated. Got: {:?}",
            index.shards
        );
        assert_eq!(index.shards[0].seq, 1);
        assert_eq!(index.shards[0].bytes_at_seal, pre_roll_content.len() as u64);

        assert_eq!(
            std::fs::metadata(&canonical_path)
                .expect("canonical must still exist")
                .len(),
            0,
            "Invariant 6: canonical must be exactly 0 bytes after a successful roll"
        );
    }

    /// EC-024/EC-025 boundary regression guard (F-C2-P8-002): the 0-byte
    /// reclaim exception must NOT weaken the write-once guarantee for a
    /// NON-EMPTY pre-existing destination — that case must continue to fail
    /// loud with `E-SHD-009`/`ShardRollError::SealedShardAlreadyExists`,
    /// leaving the pre-existing (non-empty) sealed content byte-identical
    /// and untouched, and performing NEITHER a truncate NOR an index
    /// publish for the refused attempt. This currently already PASSES
    /// against HEAD (the sibling
    /// `test_BC_1_18_006_FC2P4_002_publish_sealed_shard_refuses_to_overwrite_existing_seq`
    /// test in `bc_1_18_006_roll_test.rs` already exercises the same
    /// invariant at the `publish_sealed_shard` unit level) — pinned again
    /// here, driven through the FULL `execute_roll` sequence, as the
    /// pass-8-scoped regression guard that must stay green once the 0-byte
    /// reclaim branch above is implemented (i.e. the fix must not
    /// accidentally widen the reclaim to non-empty destinations too).
    #[test]
    fn test_BC_1_18_006_EC024_FC2P8_002_execute_roll_non_empty_destination_still_fails_loud() {
        let dir = tempfile::tempdir().expect("tempdir");
        let entry = flat_entry("decision-log", 49_152);
        let canonical_path = dir.path().join("decision-log.md");
        let pre_roll_content = "b".repeat(50_000);
        std::fs::write(&canonical_path, &pre_roll_content).expect("seed over-cap canonical");

        // A NON-EMPTY file already occupies the next-expected seq path
        // (seq=1) — genuinely durable sealed history (or an external
        // actor's real content), which write-once immutability must
        // protect.
        let sealed_seq1 = sealed_path_for(dir.path(), "decision-log", 1);
        let preexisting_content = "PRESEED".repeat(200);
        std::fs::write(&sealed_seq1, &preexisting_content)
            .expect("seed non-empty collision at seq=1");

        let err = execute_roll(&entry, &canonical_path, false).expect_err(
            "EC-024 (F-C2-P8-002 boundary regression guard): a NON-EMPTY pre-existing \
             destination at the next-expected seq path MUST continue to refuse — the 0-byte \
             reclaim exception (EC-025) must NEVER widen to a non-empty collision",
        );
        assert!(
            matches!(err, ShardRollError::SealedShardAlreadyExists { .. }),
            "EC-024: the failure MUST be ShardRollError::SealedShardAlreadyExists (E-SHD-009) \
             specifically — got: {err:?}"
        );
        assert!(
            err.to_string().contains("E-SHD-009"),
            "EC-024: the error's Display text must name the E-SHD-009 code — got: {err}"
        );

        let on_disk = std::fs::read_to_string(&sealed_seq1)
            .expect("EC-024: the pre-existing sealed shard must remain on disk");
        assert_eq!(
            on_disk, preexisting_content,
            "EC-024: the pre-existing NON-EMPTY sealed shard must be left byte-for-byte \
             UNCHANGED — publish_sealed_shard must never overwrite real sealed history"
        );

        assert_eq!(
            std::fs::read_to_string(&canonical_path).expect("canonical must still exist"),
            pre_roll_content,
            "EC-024: a refused seal-publish attempt must apply NEITHER the truncate NOR any \
             other roll step — the canonical file must be left in its exact pre-roll state"
        );

        let index_path = index_path_for(dir.path(), "decision-log");
        assert!(
            !index_path.exists(),
            "EC-024: a refused seal-publish attempt must perform NO index publish — no \
             shard-index file may be created as a side effect of a failed roll attempt"
        );
    }

    /// SEC-001 (security review, MEDIUM, CWE-61/CWE-367): a symlink planted
    /// at the exact seal destination — pointing at some OTHER 0-byte-
    /// reporting target (`/dev/null`) — must NEVER be treated as a
    /// reclaimable 0-byte destination. `publish_sealed_shard` must refuse
    /// loud (`E-SHD-009`/`ShardRollError::SealedShardAlreadyExists`) rather
    /// than dereferencing the symlink via a following `stat()` and
    /// reclaiming through it. Regression guard against a `std::fs::metadata`
    /// (follows symlinks) probe where `std::fs::symlink_metadata` (does
    /// not) is required.
    #[cfg(unix)]
    #[test]
    fn test_SEC001_publish_sealed_shard_refuses_to_reclaim_through_symlink() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sealed_path = dir.path().join("decision-log.0001.md");

        // Plant a symlink AT the exact seal destination, pointing at
        // `/dev/null` — a real path that reports 0 bytes under a
        // follow-symlinks `stat()`, but which is NOT the destination
        // itself and must never be reclaimed through.
        std::os::unix::fs::symlink("/dev/null", &sealed_path)
            .expect("create symlink at seal destination pointing at /dev/null");
        assert!(
            std::fs::symlink_metadata(&sealed_path)
                .expect("lstat seeded symlink")
                .file_type()
                .is_symlink(),
            "precondition: the seeded destination must itself be a symlink"
        );

        let err = publish_sealed_shard(&sealed_path, "real sealed content".as_bytes()).expect_err(
            "SEC-001: a symlink occupying the seal destination MUST refuse loud, never be \
             dereferenced and reclaimed through",
        );
        assert!(
            matches!(err, ShardRollError::SealedShardAlreadyExists { .. }),
            "SEC-001: expected ShardRollError::SealedShardAlreadyExists (E-SHD-009) — got: {err:?}"
        );
        assert!(
            err.to_string().contains("E-SHD-009"),
            "SEC-001: the error's Display text must name the E-SHD-009 code — got: {err}"
        );

        assert!(
            std::fs::symlink_metadata(&sealed_path)
                .expect("lstat destination after refusal")
                .file_type()
                .is_symlink(),
            "SEC-001: the symlink at the destination must be left completely untouched — never \
             unlinked, never dereferenced-and-overwritten"
        );
    }

    // -----------------------------------------------------------------
    // cluster-2 LOCAL adversary pass-9, F-C2-P9-002 (TD-VSDD-059
    // weak-substring-assertion finding): the EC-024/F-C2-P4-002 test above
    // (and its sibling in `tests/bc_1_18_006_roll_test.rs`) only assert
    // `err.to_string().contains("E-SHD-009")` — a check so weak it would
    // stay green through wording drift in the surrounding sentence (the
    // exact failure class F-C2-P8-003/P5-002 already flagged once for a
    // DIFFERENT template in this same BC). These two tests pin the FULL
    // `Display` text of `SealedShardAlreadyExists` (E-SHD-009) and
    // `BackstopProbeFailed` (E-SHD-008) VERBATIM, byte-for-byte against the
    // canonical `#[error("...")]` format strings above (confirmed by
    // product-owner), by constructing each variant directly with known
    // field values — never a `.contains(...)` fragment check again for
    // these two formats. The existing `.contains("E-SHD-009")` assertions
    // are left in place (they still serve their own tests, which assert
    // additional real-outcome facts — no truncate, no index publish, etc.
    // — that these two isolated `Display`-only tests deliberately do NOT
    // duplicate).
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_FC2P9_002_seshd009_display_matches_verbatim_format() {
        let err = ShardRollError::SealedShardAlreadyExists {
            artifact_stem: "decision-log".to_string(),
            sealed_path: "/repo/.factory/decision-log.0001.md".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "E-SHD-009: refusing to overwrite an already-sealed shard at \
             '/repo/.factory/decision-log.0001.md' for artifact_stem \"decision-log\" — sealed \
             shards are write-once/immutable; this seq already has durable content on disk",
            "F-C2-P9-002 (TD-VSDD-059): SealedShardAlreadyExists's Display text must match the \
             canonical E-SHD-009 format VERBATIM — a substring-only `.contains(\"E-SHD-009\")` \
             check would stay green through wording drift in the surrounding sentence; this \
             pins the exact byte-for-byte text so drift can no longer happen silently"
        );
    }

    #[test]
    fn test_BC_1_18_006_FC2P9_002_seshd008_display_matches_verbatim_format() {
        let source = io::Error::other("simulated non-NotFound stat() failure");
        let err = ShardRollError::BackstopProbeFailed {
            artifact_stem: "decision-log".to_string(),
            path: "/repo/.factory/decision-log.md".to_string(),
            source,
        };
        assert_eq!(
            err.to_string(),
            "E-SHD-008: Write-arm backstop stat() failed for artifact_stem \"decision-log\" at \
             '/repo/.factory/decision-log.md' — cannot confirm whether the canonical file is a \
             crash-orphaned, over-cap shard; refusing to let this Write proceed until the \
             underlying I/O condition is resolved: simulated non-NotFound stat() failure",
            "F-C2-P9-002 (TD-VSDD-059): BackstopProbeFailed's Display text (including the \
             forwarded `{{source}}` io::Error text) must match the canonical E-SHD-008 format \
             VERBATIM — the existing `test_BC_1_18_006_F003_...` integration-style test only \
             checks `.contains(\"E-SHD-008\")`; this pins the exact byte-for-byte text"
        );
    }

    // -----------------------------------------------------------------
    // cluster-2 LOCAL adversary pass-9, F-C2-P9-003 (TD-VSDD-059 coverage
    // gap): BC-1.18.006 Postcondition 8 routes TWO fail-loud dispositions
    // out of the 0-byte reclaim path (`publish_sealed_shard`'s
    // `stat -> unlink -> retry` sequence, ~lines 2303-2369 above) that had
    // NO test before this pass:
    //
    //   1. The unlink-failure arm — `std::fs::remove_file(sealed_path)`
    //      itself fails (line ~2342's `if
    //      std::fs::remove_file(sealed_path).is_err()`). Covered below by
    //      `test_BC_1_18_006_EC025_FC2P9_003_unlink_failure_fails_loud_e_shd_009`
    //      — deterministic, macOS-only (see that test's own doc comment for
    //      why, including the two fixtures that were tried and rejected
    //      after reading the production code).
    //
    //   2. The concurrent-race retry-collision arm — the SINGLE bounded
    //      `write_exclusive` retry (after a successful unlink) itself
    //      collides again with `AlreadyExists` (line ~2361's `Err(retry_err)
    //      if retry_err.kind() == io::ErrorKind::AlreadyExists`). This
    //      requires a SECOND writer to durably recreate real content at
    //      `sealed_path` in the narrow window between this function's own
    //      `remove_file` succeeding and its own retry `write_exclusive`
    //      call — two statements apart, with no callback/injection seam in
    //      `publish_sealed_shard`'s signature or body for a test to
    //      interpose there deterministically. Fabricating this with a
    //      background thread racing the two calls would be either flaky
    //      (the thread might lose the race on a fast machine) or would
    //      itself require sleeping/synchronizing INSIDE
    //      `publish_sealed_shard`'s two-line gap — not achievable without
    //      modifying the production function to add a test-only seam, which
    //      is out of scope for a coverage-strengthening pass (and would be
    //      a real behavior change to already-shipped, already-reviewed
    //      code). Per this task's own explicit instruction, this arm is
    //      NOT faked with a non-deterministic fixture: it is left as this
    //      documented, structurally-not-deterministically-testable-without-
    //      an-injection-seam gap. The `Err(retry_err) if
    //      retry_err.kind() == io::ErrorKind::AlreadyExists =>
    //      Err(already_exists_err())` arm's routed E-SHD-009 obligation is
    //      NOT silently absent from this record — it is named here so a
    //      future pass that adds a test-only injection seam (e.g. an
    //      `#[cfg(test)]` hook invoked between the unlink and the retry)
    //      can close it for real, rather than the gap being rediscovered
    //      from scratch.
    // -----------------------------------------------------------------

    /// EC-025's routed unlink-failure arm (F-C2-P9-003). Deterministic
    /// fixture: a pre-existing 0-byte file at the seq path is marked BSD
    /// user-immutable (`chflags uchg <path>` — settable by the file's own
    /// OWNER without root, unlike `schg`/Linux's `chattr +i`, which both
    /// require a privileged capability even for the owner). This makes
    /// `std::fs::remove_file` fail with `EPERM` while leaving `stat()` (used
    /// for the `is_zero_byte` check) and `hard_link`-into-a-colliding-path
    /// (used by `write_exclusive`'s first attempt) completely unaffected —
    /// it isolates EXACTLY the `remove_file(sealed_path).is_err()` branch,
    /// nothing upstream or downstream of it.
    ///
    /// Two other fixtures were tried and REJECTED after reading
    /// `publish_sealed_shard` (shard_manager.rs ~lines 2303-2369):
    ///
    /// - A directory (empty OR non-empty) at the seq path: verified
    ///   empirically that a directory's `stat()`-reported size is NEVER
    ///   exactly 0 on either filesystem this workspace's CI runs on (macOS
    ///   APFS reports 64 bytes for an empty directory; Linux ext4 reports
    ///   the block size, e.g. 4096, for any directory) — so this fixture
    ///   trips the EARLIER `if !is_zero_byte` check and returns E-SHD-009
    ///   via the ALREADY-COVERED EC-024 arm (see the sibling test above),
    ///   never reaching `remove_file` at all.
    /// - A read-only PARENT directory: `write_exclusive`'s FIRST attempt
    ///   creates its own `.tmp-<pid>` file in that SAME parent directory
    ///   BEFORE ever touching the destination path — a read-only parent
    ///   makes THAT `File::create` fail with `PermissionDenied` (verified
    ///   empirically), so `first_err.kind() != AlreadyExists` and the call
    ///   fails via the UNRELATED `SealWriteFailed` (E-SHD-001) path instead,
    ///   never reaching the 0-byte-reclaim logic at all.
    ///
    /// Platform scope: standard POSIX directory permissions provide NO
    /// mechanism to allow creating/hard-linking a NEW directory entry while
    /// denying removal of a DIFFERENT, pre-existing entry in the SAME
    /// directory — deletion is gated purely by the parent directory's write
    /// bit (or the sticky-bit same-owner rule, which needs a second UID
    /// this test cannot fabricate unprivileged). This narrows the existing
    /// `#[cfg(unix)]`-only precedent already established in this file
    /// (`test_BC_1_18_005_INV3_EC_001_...`,
    /// `test_BC_1_18_006_F003_write_backstop_stat_failure_fails_loud_e_shd_008`)
    /// one step further, to the one OS this specific mechanism exists on
    /// unprivileged. `cargo-host` (`macos-latest`) and `build-dispatcher`
    /// (`darwin-arm64`/`darwin-x64`) both run `cargo test --workspace` on
    /// every PR, so this test executes for real in CI on every push, not
    /// only on a developer's local macOS machine.
    #[cfg(target_os = "macos")]
    #[test]
    fn test_BC_1_18_006_EC025_FC2P9_003_unlink_failure_fails_loud_e_shd_009() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sealed_path = dir.path().join("decision-log.0001.md");
        std::fs::write(&sealed_path, "").expect("seed 0-byte collision at seq=1");

        let status = std::process::Command::new("chflags")
            .arg("uchg")
            .arg(&sealed_path)
            .status()
            .expect("invoke chflags(1) to mark the seeded file user-immutable");
        assert!(
            status.success(),
            "precondition: `chflags uchg` must succeed against a file this test process owns"
        );

        // Always clear the immutable flag before this function returns
        // (success, assertion failure, or panic) so the tempdir's own Drop
        // cleanup can actually remove the file, and so a panic mid-test
        // never leaks an immutable file into a subsequent run.
        struct ClearImmutableOnDrop(std::path::PathBuf);
        impl Drop for ClearImmutableOnDrop {
            fn drop(&mut self) {
                let _ = std::process::Command::new("chflags")
                    .arg("nouchg")
                    .arg(&self.0)
                    .status();
            }
        }
        let _clear_immutable_guard = ClearImmutableOnDrop(sealed_path.clone());

        assert_eq!(
            std::fs::metadata(&sealed_path)
                .expect("stat seeded file")
                .len(),
            0,
            "precondition: the seeded file must be exactly 0 bytes"
        );

        let new_content = b"z".repeat(3_000);
        let err = publish_sealed_shard(&sealed_path, &new_content).expect_err(
            "F-C2-P9-003: when the 0-byte reclaim's remove_file(sealed_path) call fails (here: \
             EPERM against a user-immutable file), publish_sealed_shard MUST fail loud with \
             E-SHD-009 — never panic, never silently proceed as if the reclaim had succeeded",
        );
        assert!(
            matches!(err, ShardRollError::SealedShardAlreadyExists { .. }),
            "F-C2-P9-003: an unlink failure during the 0-byte reclaim MUST route to \
             ShardRollError::SealedShardAlreadyExists (E-SHD-009) specifically — the same \
             fail-loud disposition as a genuinely non-empty collision — got: {err:?}"
        );
        assert!(
            err.to_string().contains("E-SHD-009"),
            "F-C2-P9-003: the error's Display text must name the E-SHD-009 code — got: {err}"
        );

        // The immutable 0-byte file must still be on disk, untouched — a
        // failed unlink must not have left anything half-reclaimed or
        // corrupted.
        assert_eq!(
            std::fs::metadata(&sealed_path)
                .expect("the seeded 0-byte file must still exist after the failed unlink")
                .len(),
            0,
            "F-C2-P9-003: the pre-existing 0-byte file must be left exactly as it was — a \
             failed unlink must not have partially modified or truncated it further"
        );
    }

    // -----------------------------------------------------------------
    // Step (c) — truncate_canonical_to_empty (Postcondition 1 step (c);
    // Invariant 2/3)
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_P1c_truncate_canonical_to_empty_zeroes_existing_content() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("decision-log.md");
        std::fs::write(&path, "this content must be gone after truncate").expect("seed");

        truncate_canonical_to_empty(&path).expect(
            "BC-1.18.006 Postcondition 1 step (c): truncate must succeed against an existing \
             canonical file",
        );

        let meta = std::fs::metadata(&path).expect(
            "canonical path must still resolve to a real file (Invariant 3) — stat() must \
             succeed, never ENOENT",
        );
        assert_eq!(
            meta.len(),
            0,
            "Invariant 6 / Postcondition 1 step (c): the canonical file MUST be exactly 0 bytes \
             immediately after truncate — no version of the roll may leave residual pre-roll \
             content behind"
        );
    }

    #[test]
    fn test_BC_1_18_006_ESHD006_truncate_canonical_to_empty_failure_maps_to_truncate_failed_after_seal()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("no-such-subdir").join("decision-log.md");

        let err = truncate_canonical_to_empty(&path).expect_err(
            "a truncate against a canonical path whose parent directory does not exist must \
             fail, not silently succeed",
        );
        assert!(
            matches!(err, ShardRollError::TruncateFailedAfterSeal { .. }),
            "BC-1.18.006 Postcondition 1 step (c) failure MUST map to \
             ShardRollError::TruncateFailedAfterSeal (E-SHD-006) specifically — got: {err:?}"
        );
    }

    // -----------------------------------------------------------------
    // Step (d) — publish_shard_index_update (Postcondition 1 step (d);
    // Postcondition 5's schema)
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_P1d_publish_shard_index_update_synthesizes_fresh_index_when_absent() {
        let dir = tempfile::tempdir().expect("tempdir");
        let index_path = index_path_for(dir.path(), "decision-log");
        let entry = flat_entry("decision-log", 49_152);
        let new_entry = ShardIndexEntry {
            seq: 1,
            path: "decision-log.0001.md".to_string(),
            sealed_at: "2026-09-07T00:00:00Z".to_string(),
            bytes_at_seal: 3_000,
            sealed_retroactively: false,
            oversized_record: false,
            is_preamble_shard: false,
            records: 0,
        };

        let index = publish_shard_index_update(&index_path, &entry, new_entry.clone()).expect(
            "BC-1.18.006 Postcondition 1 step (d): must synthesize a fresh index when none \
             exists yet",
        );

        assert_eq!(
            index.schema_version, 1,
            "Postcondition 5: schema_version must be 1"
        );
        assert_eq!(index.artifact_stem, "decision-log");
        assert_eq!(index.shard_cap_bytes, 49_152);
        assert_eq!(index.shards, vec![new_entry.clone()]);

        // Postcondition 1 step (d) also names an ATOMIC PUBLISH — the file
        // must actually be durably on disk, re-loadable via TOML.
        let on_disk = std::fs::read_to_string(&index_path).expect("index file must exist on disk");
        let reparsed: ShardIndex = toml::from_str(&on_disk).expect("index must be valid TOML");
        assert_eq!(reparsed.shards, vec![new_entry]);
    }

    #[test]
    fn test_BC_1_18_006_P1d_AC009_publish_shard_index_update_appends_second_entry_seq_increments() {
        let dir = tempfile::tempdir().expect("tempdir");
        let index_path = index_path_for(dir.path(), "decision-log");
        let entry = flat_entry("decision-log", 49_152);

        let first = ShardIndexEntry {
            seq: 1,
            path: "decision-log.0001.md".to_string(),
            sealed_at: "2026-09-07T00:00:00Z".to_string(),
            bytes_at_seal: 40_000,
            sealed_retroactively: false,
            oversized_record: false,
            is_preamble_shard: false,
            records: 0,
        };
        publish_shard_index_update(&index_path, &entry, first.clone())
            .expect("first publish must succeed");

        let second = ShardIndexEntry {
            seq: 2,
            path: "decision-log.0002.md".to_string(),
            sealed_at: "2026-09-08T00:00:00Z".to_string(),
            bytes_at_seal: 41_000,
            sealed_retroactively: false,
            oversized_record: false,
            is_preamble_shard: false,
            records: 0,
        };
        let index = publish_shard_index_update(&index_path, &entry, second.clone())
            .expect("second publish must succeed");

        assert_eq!(
            index.shards,
            vec![first, second],
            "AC-009: a second roll on the same artifact must APPEND a second [[shard]] entry — \
             the first entry (seq=1) must be left completely untouched, never overwritten or \
             reordered"
        );
    }

    #[test]
    fn test_BC_1_18_006_ESHD007_publish_shard_index_update_failure_maps_to_index_publish_failed() {
        let dir = tempfile::tempdir().expect("tempdir");
        let index_path = dir
            .path()
            .join("no-such-subdir")
            .join("decision-log.shard-index.toml");
        let entry = flat_entry("decision-log", 49_152);
        let new_entry = ShardIndexEntry {
            seq: 1,
            path: "decision-log.0001.md".to_string(),
            sealed_at: "2026-09-07T00:00:00Z".to_string(),
            bytes_at_seal: 3_000,
            sealed_retroactively: false,
            oversized_record: false,
            is_preamble_shard: false,
            records: 0,
        };

        let err = publish_shard_index_update(&index_path, &entry, new_entry)
            .expect_err("a publish into a non-existent parent directory must fail");
        assert!(
            matches!(err, ShardRollError::IndexPublishFailedAfterTruncate { .. }),
            "BC-1.18.006 Postcondition 1 step (d) failure MUST map to \
             ShardRollError::IndexPublishFailedAfterTruncate (E-SHD-007) specifically — got: \
             {err:?}"
        );
    }

    // -----------------------------------------------------------------
    // execute_roll — the full staged 4-step sequence (AC-006/007/008/009)
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_AC006_execute_roll_stages_read_publish_truncate_publish_in_order() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let pre_roll_content = "y".repeat(3_000);
        std::fs::write(&canonical_path, &pre_roll_content).expect("seed pre-roll canonical");
        let entry = flat_entry("decision-log", 49_152);

        let published = execute_roll(&entry, &canonical_path, false)
            .expect(
                "AC-006: a normal (prospective) roll over an existing, readable canonical file \
                 must succeed",
            )
            .expect(
                "AC-006: the pre-roll content is non-empty (3,000 bytes), so a seal is expected",
            );

        // Step (b): sealed shard is a byte-for-byte copy of the PRE-ROLL
        // content, published as a NEW file.
        let sealed_path = sealed_path_for(dir.path(), "decision-log", published.seq);
        let sealed_content = std::fs::read_to_string(&sealed_path)
            .expect("AC-006: the sealed shard file must exist on disk after execute_roll");
        assert_eq!(
            sealed_content, pre_roll_content,
            "AC-006: the sealed shard must be an exact byte-for-byte copy of the canonical \
             file's PRE-ROLL content — never the (nonexistent, in this test) new payload"
        );

        // Step (c): canonical file is truncated to exactly 0 bytes.
        let canonical_len = std::fs::metadata(&canonical_path)
            .expect("canonical path must still resolve to a real file (Invariant 3)")
            .len();
        assert_eq!(
            canonical_len, 0,
            "AC-006 / Invariant 6: the canonical file must be exactly 0 bytes immediately after \
             execute_roll completes"
        );

        // Step (d): the published ShardIndexEntry reflects the seal.
        assert_eq!(
            published.bytes_at_seal, 3_000,
            "AC-009: bytes_at_seal must record the sealed shard's exact final byte count"
        );
        assert!(
            !published.sealed_retroactively,
            "AC-008: a normal (prospective) roll must NOT set sealed_retroactively — the field \
             defaults false"
        );
    }

    #[test]
    fn test_BC_1_18_006_AC006_execute_roll_sealed_filename_matches_stem_seq_pattern() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("burst-log.md");
        std::fs::write(&canonical_path, "z".repeat(1_000)).expect("seed");
        let entry = flat_entry("burst-log", 49_152);

        let published = execute_roll(&entry, &canonical_path, false)
            .expect("roll over an existing canonical file must succeed")
            .expect("the pre-roll content is non-empty (1,000 bytes), so a seal is expected");

        assert_eq!(
            published.path, "burst-log.0001.md",
            "AC-006: the sealed shard's filename MUST follow the exact `<stem>.<seq:04>.md` \
             pattern (e.g. \"burst-log.0001.md\" for the first-ever roll, seq=1 zero-padded to \
             4 digits)"
        );
    }

    #[test]
    fn test_BC_1_18_006_AC009_execute_roll_second_roll_increments_seq_first_entry_untouched() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);

        std::fs::write(&canonical_path, "a".repeat(2_000)).expect("seed roll #1");
        let first = execute_roll(&entry, &canonical_path, false)
            .expect("first roll must succeed")
            .expect("the pre-roll content is non-empty (2,000 bytes), so a seal is expected");
        assert_eq!(
            first.seq, 1,
            "AC-009: the first-ever roll must publish seq=1"
        );

        // Canonical is now empty (post roll #1) — write fresh content to
        // simulate a second cycle-artifact append that itself later exceeds
        // cap again.
        std::fs::write(&canonical_path, "b".repeat(4_000)).expect("seed roll #2");
        let second = execute_roll(&entry, &canonical_path, false)
            .expect("second roll must succeed")
            .expect("the pre-roll content is non-empty (4,000 bytes), so a seal is expected");
        assert_eq!(
            second.seq, 2,
            "AC-009: a SECOND roll on the same artifact must publish seq=2, incrementing \
             monotonically from the first"
        );

        let index_path = index_path_for(dir.path(), "decision-log");
        let on_disk = std::fs::read_to_string(&index_path).expect("index must exist");
        let index: ShardIndex = toml::from_str(&on_disk).expect("index must parse");
        assert_eq!(
            index.shards,
            vec![first, second],
            "AC-009: the index must contain BOTH entries, in order, with seq=1's entry left \
             completely untouched by the second roll (append-only)"
        );
    }

    /// PR #824 pr-review Finding #9 (NIT): `next_seal_seq`'s prior
    /// `max().unwrap_or(0) + 1` panics in debug / silently wraps to 0 in
    /// release once the recorded max `seq` reaches `u32::MAX` — this
    /// fixture plants an index whose sole entry is already at `u32::MAX`
    /// and asserts `execute_roll` fails loud (via the existing E-SHD-001
    /// `SealWriteFailed` path every other `next_seal_seq` I/O failure
    /// already uses) rather than wrapping to a bogus `seq=0` and silently
    /// colliding with (or preceding) `seq=1`'s own sealed shard.
    #[test]
    fn test_FINDING9_next_seal_seq_overflow_at_u32_max_fails_loud() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);

        let index_path = index_path_for(dir.path(), "decision-log");
        let index = ShardIndex {
            schema_version: 1,
            artifact_stem: "decision-log".to_string(),
            current_shard: "decision-log.md".to_string(),
            shard_cap_bytes: entry.shard_cap_bytes,
            max_single_record_bytes: entry.max_single_record_bytes,
            safety_margin_bytes: entry.safety_margin,
            practical_fuel_ceiling: entry.practical_fuel_ceiling,
            worst_case_fuel_per_byte: entry.worst_case_fuel_per_byte,
            // Sibling-site sweep (S-25.02 cluster-3): new
            // `ShardIndex::retention_count` field, default value — this
            // existing (cluster-1/2) test is unconcerned with retention and
            // is otherwise unchanged.
            retention_count: default_retention_count(),
            shards: vec![ShardIndexEntry {
                seq: u32::MAX,
                path: "decision-log.4294967295.md".to_string(),
                sealed_at: "2026-01-01T00:00:00Z".to_string(),
                bytes_at_seal: 1,
                sealed_retroactively: false,
                oversized_record: false,
                is_preamble_shard: false,
                records: 0,
            }],
        };
        std::fs::write(
            &index_path,
            toml::to_string(&index).expect("serialize fixture index"),
        )
        .expect("write fixture index");

        std::fs::write(&canonical_path, "a".repeat(2_000)).expect("seed a roll-triggering write");

        let err = execute_roll(&entry, &canonical_path, false).expect_err(
            "Finding #9: next_seal_seq must fail loud once the recorded max seq is already \
             u32::MAX — never wrap/panic on the '+ 1', never silently compute a bogus seq",
        );
        assert!(
            matches!(err, ShardRollError::SealWriteFailed { .. }),
            "Finding #9: the overflow must surface as the SAME E-SHD-001/SealWriteFailed every \
             other next_seal_seq I/O failure already uses — got: {err:?}"
        );

        // No partial roll may have occurred: canonical untouched, no
        // sealed shard published.
        assert_eq!(
            std::fs::read_to_string(&canonical_path).expect("canonical must still be readable"),
            "a".repeat(2_000),
            "Finding #9: the canonical file must be left in its exact pre-roll state — no \
             partial roll on an overflow failure"
        );
    }

    #[test]
    fn test_BC_1_18_006_AC024_execute_roll_retroactive_sets_sealed_retroactively_true() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        // An over-cap on-disk content, as a `replace_all: true` write would
        // have already produced (EC-014) — the retroactive roll seals
        // content that has ALREADY been written, so bytes_at_seal MAY
        // legitimately exceed shard_cap_bytes (Postcondition 7's documented
        // exception).
        let over_cap_content = "c".repeat(49_500);
        std::fs::write(&canonical_path, &over_cap_content).expect("seed over-cap canonical");
        let entry = flat_entry("decision-log", 49_152);

        let published = execute_roll(&entry, &canonical_path, true)
            .expect("a retroactive roll must succeed identically to a prospective one")
            .expect(
                "the over-cap pre-roll content is non-empty (49,500 bytes), so a seal is expected",
            );

        assert!(
            published.sealed_retroactively,
            "AC-024 / Postcondition 5: a retroactive roll's [[shard]] entry MUST set \
             sealed_retroactively = true — the sole audit trail distinguishing this shard-cap \
             guarantee exception"
        );
        assert_eq!(
            published.bytes_at_seal, 49_500,
            "Postcondition 7's documented exception: a retroactively-sealed shard's \
             bytes_at_seal MAY exceed shard_cap_bytes (49,500 > 49,152 here) — this is legal \
             ONLY because sealed_retroactively is true"
        );

        let canonical_len = std::fs::metadata(&canonical_path)
            .expect("canonical must still exist")
            .len();
        assert_eq!(
            canonical_len, 0,
            "Invariant 6: the canonical file's zero-bytes-after-roll guarantee holds \
             UNCONDITIONALLY even for a retroactive roll — only the SEALED shard's cap \
             guarantee is relaxed, never the canonical file's"
        );
    }

    // -----------------------------------------------------------------
    // Self-healing recovery — E-SHD-006 (EC-010) / E-SHD-007 (EC-011)
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_ESHD006_self_heal_resume_from_truncate_detects_duplicate_and_resumes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        let stuck_content = "d".repeat(2_500);

        // Simulate the E-SHD-006 crash point: the sealed shard is ALREADY
        // durably published (step (b) succeeded) but the canonical file
        // STILL holds that same content (step (c) never ran).
        let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
        std::fs::write(&sealed_path, &stuck_content).expect("seed sealed shard (step b done)");
        std::fs::write(&canonical_path, &stuck_content).expect("seed duplicate canonical");

        let result = self_heal_resume_from_truncate(&entry, &canonical_path).expect(
            "EC-010: the resume-from-truncate self-heal must succeed against a genuine \
             duplicate-content state",
        );
        assert!(
            result.is_some(),
            "EC-010: a detected duplicate-content state (sealed shard byte-identical to the \
             CURRENT canonical content) must resume from step (c) alone and publish the missing \
             index entry — never Ok(None)"
        );

        let canonical_len = std::fs::metadata(&canonical_path)
            .expect("canonical must still exist")
            .len();
        assert_eq!(
            canonical_len, 0,
            "EC-010: resume-from-truncate must complete step (c) — the canonical file must be \
             empty after the self-heal runs"
        );

        // The already-durable sealed shard must NEVER be rewritten (the
        // self-heal resumes from step (c) ALONE).
        let sealed_after =
            std::fs::read_to_string(&sealed_path).expect("sealed shard must remain on disk");
        assert_eq!(
            sealed_after, stuck_content,
            "EC-010: the already-correct sealed shard must be left byte-for-byte untouched — \
             the self-heal never re-publishes step (b)"
        );
    }

    #[test]
    fn test_BC_1_18_006_ESHD006_self_heal_resume_from_truncate_no_action_when_not_duplicated() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        // Normal, healthy state: canonical holds fresh content, no sealed
        // shard exists at the next-expected seq at all.
        std::fs::write(&canonical_path, "e".repeat(500)).expect("seed healthy canonical");

        let result = self_heal_resume_from_truncate(&entry, &canonical_path).expect(
            "self_heal_resume_from_truncate must not error against a healthy (non-crashed) \
             state",
        );
        assert!(
            result.is_none(),
            "EC-010: when no sealed-shard/canonical duplicate-content state exists, the \
             self-heal must take NO action (Ok(None)) — it must never fabricate a roll"
        );
        assert_eq!(
            std::fs::read_to_string(&canonical_path).expect("canonical must be unchanged"),
            "e".repeat(500),
            "no-op self-heal must leave the canonical file completely untouched"
        );
    }

    #[test]
    fn test_BC_1_18_006_ESHD007_self_heal_reconcile_missing_index_entries_appends_unindexed_shard()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        // Simulate the E-SHD-007 crash point: canonical is correctly empty
        // and the sealed shard exists correctly on disk, but NO index file
        // exists at all yet (step (d) never ran).
        std::fs::write(&canonical_path, "").expect("seed empty (post-truncate) canonical");
        let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
        std::fs::write(&sealed_path, "f".repeat(1_200)).expect("seed sealed shard (step c done)");

        let appended = self_heal_reconcile_missing_index_entries(&entry, &canonical_path).expect(
            "EC-011: reconciliation must succeed when an un-indexed sealed shard is discovered \
             on disk",
        );
        assert_eq!(
            appended.len(),
            1,
            "EC-011: exactly one missing [[shard]] entry (seq=1, the un-indexed sealed shard) \
             must be appended"
        );
        assert_eq!(appended[0].seq, 1);
        assert_eq!(appended[0].path, "decision-log.0001.md");
        assert_eq!(
            appended[0].bytes_at_seal, 1_200,
            "the reconciled entry's bytes_at_seal must reflect the sealed shard's ACTUAL \
             on-disk byte count"
        );

        let index_path = index_path_for(dir.path(), "decision-log");
        let on_disk = std::fs::read_to_string(&index_path)
            .expect("EC-011: the reconciliation must durably publish the index file");
        let index: ShardIndex = toml::from_str(&on_disk).expect("index must parse");
        assert_eq!(
            index.shards, appended,
            "the published index must contain exactly the reconciled entry"
        );
    }

    #[test]
    fn test_BC_1_18_006_ESHD007_self_heal_reconcile_missing_index_entries_no_action_when_fully_indexed()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        std::fs::write(&canonical_path, "").expect("seed empty canonical");
        let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
        std::fs::write(&sealed_path, "g".repeat(800)).expect("seed sealed shard");

        // Pre-publish the index entry so the sealed shard is ALREADY fully
        // reconciled — the self-heal must be idempotent and take no action.
        let index_path = index_path_for(dir.path(), "decision-log");
        let already_indexed = ShardIndexEntry {
            seq: 1,
            path: "decision-log.0001.md".to_string(),
            sealed_at: "2026-09-07T00:00:00Z".to_string(),
            bytes_at_seal: 800,
            sealed_retroactively: false,
            oversized_record: false,
            is_preamble_shard: false,
            records: 0,
        };
        publish_shard_index_update(&index_path, &entry, already_indexed)
            .expect("pre-seed the index with the already-correct entry");

        let appended = self_heal_reconcile_missing_index_entries(&entry, &canonical_path)
            .expect("reconciliation over an already-fully-indexed state must not error");
        assert!(
            appended.is_empty(),
            "EC-011: when every on-disk sealed shard is already present in the index, the \
             self-heal must append NOTHING (idempotent on repeat)"
        );
    }

    // -----------------------------------------------------------------
    // BC-1.18.006 v1.7 Invariant 9 (cluster-2 LOCAL adversary pass-3 finding
    // F-C2-P3-002) — `execute_roll` already refuses to publish a 0-byte seal
    // (F-C2-P2-006). Invariant 9 extends the SAME guarantee to BOTH self-heal
    // index-publishing paths: `self_heal_reconcile_missing_index_entries`
    // (which today indexes ANY on-disk `<stem>.<seq:04>.md` sibling via
    // `metadata()?.len()`, with no byte-count floor at all) and
    // `self_heal_resume_from_truncate` (whose byte-identity comparison
    // between the sealed candidate and the CURRENT canonical content is
    // vacuously satisfied when BOTH happen to be 0 bytes — an empty sealed
    // candidate looks identical to an empty, correctly-already-truncated
    // canonical). `execute_roll` can never itself produce a 0-byte seal
    // (Postcondition 3 / F-C2-P2-006's own empty-canonical short-circuit), so
    // ANY 0-byte candidate either self-heal path encounters is by
    // construction an EXTERNAL anomaly (never a genuine artifact of this
    // dispatcher's own roll sequence) — indexing it would fabricate a false
    // audit-trail entry (a `[[shard]]` row claiming a seal event that never
    // legitimately happened). Both paths below must SKIP such a candidate
    // (never append a row with `bytes_at_seal == 0`), emit a `tracing::warn!`
    // diagnostic rather than silently doing nothing, and never fail the
    // dispatch over it.
    //
    // Local duplicate of the sibling `mod tests`'s (cluster-1's) own
    // `WarnCapture`/`count_warns` capture harness — kept independent per this
    // module's own established convention (see `flat_entry`'s doc comment
    // above), rather than reaching into cluster-1's private test-only
    // surface.
    // -----------------------------------------------------------------

    struct InvariantNineWarnCapture {
        warn_count: std::sync::Arc<std::sync::atomic::AtomicUsize>,
    }

    impl tracing::Subscriber for InvariantNineWarnCapture {
        fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
            true
        }
        fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
            tracing::span::Id::from_u64(1)
        }
        fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
        fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
        fn event(&self, event: &tracing::Event<'_>) {
            if *event.metadata().level() == tracing::Level::WARN {
                self.warn_count
                    .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }
        }
        fn enter(&self, _span: &tracing::span::Id) {}
        fn exit(&self, _span: &tracing::span::Id) {}
    }

    static INVARIANT_NINE_CAPTURE_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn count_invariant_nine_warns<T>(f: impl FnOnce() -> T) -> (usize, T) {
        let _guard = INVARIANT_NINE_CAPTURE_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);

        let warn_count = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
        let subscriber = InvariantNineWarnCapture {
            warn_count: warn_count.clone(),
        };
        let result = tracing::subscriber::with_default(subscriber, || {
            tracing::callsite::rebuild_interest_cache();
            f()
        });
        (warn_count.load(std::sync::atomic::Ordering::SeqCst), result)
    }

    #[test]
    fn test_BC_1_18_006_INV9_F_C2_P3_002_self_heal_reconcile_missing_index_entries_skips_zero_byte_orphan()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        std::fs::write(&canonical_path, "").expect("seed empty (post-truncate) canonical");

        // External anomaly: a 0-byte file sitting at the artifact's exact
        // `<stem>.<seq:04>.md` sealed-shard naming convention. `execute_roll`
        // can never itself have produced this (F-C2-P2-006 short-circuits
        // before ever sealing empty content), so this can only be an
        // externally-created (or externally-corrupted) file — never a
        // legitimate orphaned seal.
        let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
        std::fs::write(&sealed_path, "").expect("seed 0-byte orphan sealed-shard candidate");

        let (warn_count, result) = count_invariant_nine_warns(|| {
            self_heal_reconcile_missing_index_entries(&entry, &canonical_path)
        });
        let appended = result.expect(
            "BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002): encountering a 0-byte orphan candidate \
             must NEVER fail the dispatch — it must be skipped, not propagated as an error",
        );

        assert!(
            appended.is_empty(),
            "BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002, cluster-2 LOCAL adversary pass-3): a \
             0-byte candidate at the artifact's sealed-shard naming convention must NEVER be \
             indexed — execute_roll can never itself produce a 0-byte seal, so this is an \
             external anomaly and indexing it would fabricate a false audit-trail entry. Got: \
             {appended:?}"
        );

        let index_path = index_path_for(dir.path(), "decision-log");
        assert!(
            !index_path.exists(),
            "Invariant 9: with nothing legitimate left to reconcile (the ONLY on-disk candidate \
             is the skipped 0-byte orphan), no shard-index file should be published at all"
        );

        assert!(
            warn_count >= 1,
            "BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002): skipping a 0-byte orphan candidate must \
             emit a tracing diagnostic (tracing::warn!) rather than silently doing nothing — got \
             {warn_count} warn-level events"
        );

        // The anomalous 0-byte file itself must be left untouched on disk —
        // the self-heal only ever appends index rows, never mutates or
        // deletes candidate files.
        let sealed_len = std::fs::metadata(&sealed_path)
            .expect("the 0-byte candidate file must remain on disk")
            .len();
        assert_eq!(sealed_len, 0);
    }

    #[test]
    fn test_BC_1_18_006_INV9_F_C2_P3_002_self_heal_resume_from_truncate_skips_zero_byte_sealed_and_canonical()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        // Both the canonical AND the sealed candidate are 0 bytes — under
        // today's byte-identity comparison alone, this is INDISTINGUISHABLE
        // from a genuine E-SHD-006 "seal published, truncate did not" match
        // (0 bytes == 0 bytes), so the self-heal would treat it as a
        // legitimate duplicate and resume from step (c), publishing a
        // `bytes_at_seal = 0` index row. Invariant 9 requires this exact
        // sibling case to ALSO be recognized as the 0-byte anomaly and
        // skipped, never indexed.
        std::fs::write(&canonical_path, "").expect("seed empty canonical");
        let sealed_path = sealed_path_for(dir.path(), "decision-log", 1);
        std::fs::write(&sealed_path, "").expect("seed 0-byte sealed candidate");

        let (warn_count, result) =
            count_invariant_nine_warns(|| self_heal_resume_from_truncate(&entry, &canonical_path));
        let outcome = result.expect(
            "BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002): encountering a 0-byte sealed+canonical \
             anomaly must NEVER fail the dispatch",
        );

        assert!(
            outcome.is_none(),
            "BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002, cluster-2 LOCAL adversary pass-3): a \
             0-byte sealed candidate byte-identical to an also-0-byte canonical must NOT be \
             treated as a genuine E-SHD-006 duplicate-content match — execute_roll can never \
             itself produce a 0-byte seal, so this vacuous 0-byte==0-byte match is an external \
             anomaly, not a resumable crash state. Got Some({outcome:?}) instead of None"
        );

        let index_path = index_path_for(dir.path(), "decision-log");
        assert!(
            !index_path.exists(),
            "Invariant 9: skipping the 0-byte anomaly must never publish a shard-index entry \
             (bytes_at_seal = 0 would fabricate a false audit-trail row)"
        );

        assert!(
            warn_count >= 1,
            "BC-1.18.006 v1.7 Invariant 9 (F-C2-P3-002): skipping the 0-byte sealed+canonical \
             anomaly must emit a tracing diagnostic (tracing::warn!) rather than silently doing \
             nothing — got {warn_count} warn-level events"
        );

        // Neither file's content may be mutated by the skip.
        assert_eq!(
            std::fs::metadata(&canonical_path)
                .expect("canonical must remain on disk")
                .len(),
            0
        );
        assert_eq!(
            std::fs::metadata(&sealed_path)
                .expect("sealed candidate must remain on disk")
                .len(),
            0
        );
    }

    // -----------------------------------------------------------------
    // F-C2-P7-004 (ADVISORY, cluster-2 LOCAL adversary pass-7) —
    // `self_heal_recovery_plausible`'s cheap probe must apply the SAME
    // 0-byte guard Invariant 9 (F-C2-P3-002) already requires of the two
    // downstream self-heal functions. Today the probe matches candidates on
    // filename shape ALONE (`<stem>.<seq>.md`, seq >= 4 digits), ignoring
    // size entirely — so a PERSISTENT external 0-byte orphan (which both
    // downstream self-heal functions will forever refuse to index, per
    // Invariant 9) keeps the probe reporting "plausible" on EVERY future
    // dispatch for this artifact, forcing needless payment for both
    // self-heal functions' more expensive checks (byte-for-byte content
    // comparison; a directory-wide orphan scan) for zero possible benefit.
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_FC2P7_004_self_heal_recovery_plausible_ignores_persistent_zero_byte_orphan()
    {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        std::fs::write(&canonical_path, "k".repeat(500)).expect("seed healthy canonical");
        let entry = flat_entry("decision-log", 49_152);

        // A PERSISTENT 0-byte orphan matching the sealed-shard naming
        // convention (>=4-digit seq), unindexed (no shard-index.toml at all
        // exists). Per Invariant 9 (F-C2-P3-002), `execute_roll` can NEVER
        // itself produce a 0-byte seal, and both
        // `self_heal_resume_from_truncate` and
        // `self_heal_reconcile_missing_index_entries` already refuse to
        // index a 0-byte candidate — they will SKIP this exact file
        // forever, since nothing about it ever changes across dispatches.
        let orphan_path = sealed_path_for(dir.path(), "decision-log", 1);
        std::fs::write(&orphan_path, "").expect("seed persistent 0-byte orphan");

        let plausible = self_heal_recovery_plausible(&entry, &canonical_path).expect(
            "F-C2-P7-004: the plausibility probe must not error merely because a 0-byte \
             filename-shaped candidate exists on disk",
        );
        assert!(
            !plausible,
            "F-C2-P7-004 (ADVISORY): a 0-byte orphan candidate must NEVER register as a \
             plausible self-heal target — consistent with Invariant 9's 0-byte guard, which \
             both downstream self-heal functions already apply and will forever skip this SAME \
             candidate. The plausibility probe currently matches candidates on filename shape \
             ALONE, ignoring size, so it wrongly (and permanently) reports `true` for this \
             artifact on every future dispatch, paying for both self-heal functions' more \
             expensive checks for zero possible benefit."
        );
    }

    // -----------------------------------------------------------------
    // Postcondition 7 catch points — AC-024 / AC-025 (reused via execute_roll)
    // -----------------------------------------------------------------

    #[test]
    fn test_BC_1_18_006_AC024_reconcile_post_write_replace_all_overcap_no_action_under_cap() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        std::fs::write(&canonical_path, "h".repeat(1_000)).expect("well under cap");

        let result = reconcile_post_write_replace_all_overcap(&entry, &canonical_path)
            .expect("catch point (i) must not error when the actual on-disk size is within cap");
        assert!(
            result.is_none(),
            "AC-024: when actual_size <= shard_cap_bytes, catch point (i) must take NO action \
             (Ok(None)) — the single-occurrence trigger estimate was conservative or exactly \
             correct"
        );
    }

    #[test]
    fn test_BC_1_18_006_AC024_EC014_reconcile_post_write_replace_all_overcap_triggers_retroactive_roll()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        // EC-014's worked example: an under-projected replace_all write has
        // ALREADY been applied, leaving the canonical file genuinely over
        // cap on disk.
        std::fs::write(&canonical_path, "i".repeat(49_500)).expect("seed over-cap canonical");

        let result = reconcile_post_write_replace_all_overcap(&entry, &canonical_path)
            .expect("AC-024: catch point (i) must succeed when it detects an over-cap state");
        let published = result.expect(
            "AC-024 / EC-014: actual_size > shard_cap_bytes MUST trigger the retroactive \
             four-step roll (Ok(Some(entry))), never Ok(None)",
        );
        assert!(
            published.sealed_retroactively,
            "AC-024: the resulting seal MUST set sealed_retroactively = true"
        );
        assert_eq!(
            std::fs::metadata(&canonical_path)
                .expect("canonical must exist")
                .len(),
            0,
            "Invariant 6: canonical must be exactly 0 bytes after catch point (i) reconciles"
        );
    }

    #[test]
    fn test_BC_1_18_006_AC025_EC015_reconcile_leading_probe_backstop_triggers_retroactive_roll() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        // EC-015: catch point (i) crashed/never ran — the canonical file is
        // STILL over cap on disk when the artifact's NEXT dispatch arrives.
        std::fs::write(&canonical_path, "j".repeat(50_000)).expect("seed still-over-cap canonical");

        let published = reconcile_leading_probe_backstop(&entry, &canonical_path)
            .expect("AC-025: the backstop must succeed given an already-confirmed over-cap state")
            .expect(
                "AC-025 / EC-015: the leading-probe backstop must ALWAYS reconcile when called \
                 (the caller has already confirmed current_bytes > shard_cap_bytes) — never \
                 Ok(None)",
            );
        assert!(
            published.sealed_retroactively,
            "AC-025: the backstop's seal MUST also set sealed_retroactively = true — it reuses \
             the SAME retroactive roll sequence AC-024's catch point (i) specifies"
        );
        assert_eq!(
            std::fs::metadata(&canonical_path)
                .expect("canonical must exist")
                .len(),
            0,
            "Postcondition 7's bounded-window guarantee: after the backstop fires, the canonical \
             file must be exactly 0 bytes — the over-cap window is closed"
        );
    }

    // ===================================================================
    // F-C2-P2-003 (S-25.02 Phase F4 LOCAL adversary pass-2 cluster-2
    // finding, product-owner-adjudicated BC-1.18.006 v1.6, Invariant 8 /
    // EC-019 / E-SHD-008) — the `ToolKind::Write` arm's OWN dedicated
    // backstop `stat()` (the SAME `current_shard_bytes_flat(target_path)`
    // call that arm performs for its crash-orphan probe, entirely distinct
    // from that arm's stat-free Postcondition 3 trigger formula) MUST fail
    // LOUD — return `HookResult::Error` naming the `E-SHD-008` error code —
    // on ANY non-`NotFound` `stat()` error, matching the existing
    // fail-loud posture the `Edit`/`MultiEdit` arms already apply to their
    // OWN `current_shard_bytes_flat` calls just below in this same match.
    //
    // Rationale (product-owner v1.6 adjudication): `stat()` follows
    // symlinks and fails on `ELOOP`, but `write_atomic`'s `rename` need
    // not dereference the final symlink component and CAN succeed — so
    // fail-OPEN here would let this probe silently skip while the Write
    // itself proceeds underneath a symlink the probe could not see
    // through, destroying a crash-orphaned, un-sealed, over-cap canonical
    // this dispatch never confirmed was safe to overwrite. `NotFound`
    // remains completely unaffected (EC-004, legitimate first-write) —
    // this Invariant 8 gate is scoped to every OTHER `io::ErrorKind`.
    //
    // RED GATE (BC-5.38.001): this test MUST FAIL against the code as of
    // this writing. The `ToolKind::Write` arm's backstop `Err(e)` leg
    // (see the doc comment on that arm's `match current_shard_bytes_flat`
    // above, "F-002 ... still binding: a non-NotFound stat() failure here
    // is fail-OPEN, never fail-loud") only `tracing::warn!`s and falls
    // through to this dispatch's own (stat-free) trigger formula,
    // returning `HookResult::Continue` for an under-cap payload — never
    // `HookResult::Error`. Implementer must flip that specific `Err(e)`
    // arm to return `HookResult::Error` naming `E-SHD-008` for every
    // non-`NotFound` `io::ErrorKind`, leaving `current_shard_bytes_flat`'s
    // own `NotFound` -> `Ok(0)` mapping (EC-004) completely untouched —
    // this fixture never exercises that leg.
    //
    // STATIC CONFLICT WITH CLUSTER-1's F-002 (flagged, NOT resolved here):
    // this fixture deliberately reuses cluster-1's own
    // `test_BC_1_18_005_F002_write_under_cap_continues_despite_irrelevant_stat_failure`
    // (`mod tests`, this same file) technique verbatim — same
    // self-referential-symlink ELOOP construction, same matched
    // "decision-log" stem, same under-cap `content` payload — because it
    // is the EXACT SAME underlying `current_shard_bytes_flat(target_path)`
    // call inside the SAME `ToolKind::Write` arm that F-002 already pins
    // to `HookResult::Continue`. Flipping that arm's `Err(e)` leg to
    // fail-loud per BC-1.18.006 v1.6 Invariant 8 (this test) necessarily
    // flips F-002 (BC-1.18.005) to RED — a genuine BC-1.18.005 vs
    // BC-1.18.006 v1.6 spec-vs-spec conflict on the identical code path,
    // NOT a test-authoring error. Per the Dark Factory Companion
    // Principle, this is routed back to product-owner for adjudication
    // (F-002's own invariant vs. v1.6 Invariant 8) rather than silently
    // edited away here — test-writer does not resolve BC conflicts.
    // ===================================================================

    // RETARGETED (S-25.02 PR #824 second-security-review, FIX-MED-2): this
    // test ORIGINALLY used a self-referential-symlink ELOOP fixture to
    // force a *generic* non-NotFound stat() failure — the SAME "portable
    // technique... mirroring this crate's own established precedent" the
    // sibling F-002 retargeting note above (S-25.02 Phase F4 cluster-2
    // pass-2 finding F-C2-P2-003) describes. FIX-MED-2 now installs
    // `reject_canonical_symlink`'s own `symlink_metadata`-based guard
    // BEFORE this arm's `current_shard_bytes_flat` call, so ANY symlink
    // (self-referential/looped or not) is caught EARLIER and fails loud
    // with the MORE SPECIFIC `E-SHD-010` (`CanonicalPathIsSymlink`) —
    // never reaching E-SHD-008's own `stat()` call at all. Mirroring the
    // exact same precedent (a stale, over-broad symlink fixture retargeted
    // onto a REAL, non-symlink mechanism once a newer guard intercepts it
    // earlier), this fixture is retargeted onto a canonical path whose OWN
    // final filename component exceeds every common filesystem's
    // `NAME_MAX` (255 bytes) — `stat()`/`metadata()` (and `lstat()`/
    // `symlink_metadata()`) on such a path fails with `ENAMETOOLONG`,
    // never `NotFound`, and never touching a symlink at all (empirically
    // confirmed: `reject_canonical_symlink`'s own `symlink_metadata` call
    // ALSO fails with `ENAMETOOLONG` here, which its `Ok(_) | Err(_) =>
    // Ok(())` fall-through correctly treats as "cannot determine, defer to
    // the caller's own read/stat" — exactly as designed, never
    // misclassified as a confirmed symlink).
    //
    // Why the `artifact_stem` stays SHORT ("decision-log") while only the
    // FILENAME'S EXTENSION is overlong: this arm is NOT the first thing
    // `shard_cap_gate_check`'s `ShardShape::Flat` branch runs against
    // `canonical_path` — `run_self_heal_if_plausible`'s own
    // `self_heal_recovery_plausible` probe runs FIRST, and its `index_path
    // = shard_index_path_for(canonical_path, &entry.artifact_stem)` derives
    // the SIBLING `.shard-index.toml` filename from `artifact_stem` alone
    // (never from `canonical_path`'s own filename). An EARLIER version of
    // this fixture made `artifact_stem` itself the 300-byte overlong string
    // (required for `find_matching_entry`'s `file_stem()` equality check
    // when the WHOLE filename, stem included, is overlong) — but that also
    // makes the sibling index filename overlong, so
    // `self_heal_recovery_plausible`'s OWN `load_shard_index` call fails
    // FIRST with the SAME `ENAMETOOLONG`, surfacing `E-SHD-001` instead
    // (confirmed empirically) and never reaching this arm's dedicated probe
    // at all. Keeping `artifact_stem` short and ONLY appending a 300-byte
    // extension after `canonical_path`'s `"decision-log."` stem keeps the
    // sibling index filename short (`self_heal_recovery_plausible` resolves
    // it fine, finds nothing plausible, `Ok(false)` — confirmed
    // empirically) while `canonical_path`'s OWN full filename (stem +
    // extension) still exceeds `NAME_MAX`, isolating the failure to THIS
    // arm's own `current_shard_bytes_flat(target_path)` call exactly as
    // this test requires — the same per-final-component isolation property
    // the original ELOOP-via-symlink fixture relied on, without touching a
    // symlink at all. See
    // `test_FIXMED2_write_backstop_symlink_canonical_fails_loud_e_shd_010`
    // (this same file, just below) for the NEW, explicit symlink-specific
    // `E-SHD-010` coverage this retargeting displaces.
    #[test]
    fn test_BC_1_18_006_F003_write_backstop_stat_failure_fails_loud_e_shd_008() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Full final path component = "decision-log." (13 bytes) + 300
        // bytes = 313 bytes, exceeding NAME_MAX (255 bytes) on every common
        // POSIX filesystem (ext4, APFS, tmpfs, ...) — but `file_stem()`
        // still splits at the LAST '.', so `entry.artifact_stem` stays the
        // short, normal "decision-log" (see the doc comment above for why
        // that matters).
        let filename = format!("decision-log.{}", "a".repeat(300));
        let canonical = dir.path().join(&filename);

        let entry = ShardEntry {
            artifact_stem: "decision-log".to_string(),
            artifact_path: filename,
            practical_fuel_ceiling: 8_000_000,
            worst_case_fuel_per_byte: 106.36,
            max_single_record_bytes: 16_384,
            safety_margin: 8_192,
            shard_cap_bytes: 49_152,
            shape: Some(ShardShape::Flat),
            n: None,
            low_water_mark: None,
        };
        let registry = ShardRegistry {
            shards: vec![entry],
        };

        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &canonical,
            &serde_json::json!({"content": "under-cap content, far below the 49,152-byte cap"}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("E-SHD-008"),
                    "BC-1.18.006 v1.6 Invariant 8 / EC-019: the Write-arm backstop's \
                     non-NotFound stat() failure MUST fail loud naming the E-SHD-008 error \
                     code specifically — got a different HookResult::Error message: {message:?}"
                );
            }
            other => panic!(
                "BC-1.18.006 v1.6 Invariant 8 / EC-019 / F-C2-P2-003: a non-NotFound stat() \
                 failure (ENAMETOOLONG via an overlong final path component) at the Write-arm's \
                 OWN dedicated backstop probe MUST fail LOUD as HookResult::Error naming \
                 E-SHD-008 — NOT silently fail-open into Continue (or resolve to Block). A \
                 probe that cannot confirm-or-deny a crash-orphaned, over-cap canonical MUST \
                 NOT let this Write proceed to destroy that content merely because its OWN \
                 `content` payload is under cap. Got: {other:?}"
            ),
        }
    }

    /// FIX-MED-2 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-59/
    /// CWE-200): the Write-arm's own dedicated backstop probe site
    /// (`current_shard_bytes_flat(target_path)`, the SAME call site
    /// `E-SHD-008`/Invariant 8 above governs for GENERIC stat() failures)
    /// must refuse loud with `E-SHD-010` (`CanonicalPathIsSymlink`) when
    /// the canonical path IS a symlink — never silently `stat()` through
    /// it (which would leak the size of an arbitrary file the dispatcher
    /// process can stat, a CWE-200 info-exposure side channel) nor let a
    /// retroactive roll ever read and durably seal a symlink target's
    /// bytes. Regression guard for the guard installed BEFORE this arm's
    /// `current_shard_bytes_flat` call.
    #[cfg(unix)]
    #[test]
    fn test_FIXMED2_write_backstop_symlink_canonical_fails_loud_e_shd_010() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir.path().join("decision-log.md");
        let sensitive_target = dir.path().join("sensitive-elsewhere.txt");
        std::fs::write(
            &sensitive_target,
            "attacker wants THIS content exfiltrated/sealed",
        )
        .expect("seed the symlink target file");
        std::os::unix::fs::symlink(&sensitive_target, &canonical)
            .expect("plant a symlink at the governed canonical path");

        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };

        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &canonical,
            &serde_json::json!({"content": "under-cap content, far below the 49,152-byte cap"}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("E-SHD-010"),
                    "FIX-MED-2: a symlinked canonical path at the Write-arm's own backstop probe \
                     site MUST fail loud naming the E-SHD-010 error code specifically — got a \
                     different HookResult::Error message: {message:?}"
                );
            }
            other => panic!(
                "FIX-MED-2: a symlinked canonical path MUST fail LOUD as HookResult::Error \
                 naming E-SHD-010 — NOT silently stat() through the symlink (info-exposure side \
                 channel) nor Continue/Block as if it were an ordinary file. Got: {other:?}"
            ),
        }

        assert_eq!(
            std::fs::read_to_string(&sensitive_target).expect("sensitive target must still exist"),
            "attacker wants THIS content exfiltrated/sealed",
            "FIX-MED-2: the symlink target's content must be completely untouched — never read, \
             never sealed into a new file"
        );
        assert!(
            !dir.path().join("decision-log.0001.md").exists(),
            "FIX-MED-2: no sealed shard may ever be published from a refused symlinked \
             canonical — the symlink target's bytes must never end up durably sealed anywhere"
        );
    }

    /// PR #824 pr-review Finding #3 (MAJOR): the `Edit` arm's own
    /// `current_shard_bytes_flat(target_path)` call site — the sibling of
    /// the Write-arm backstop probe covered by the test just above — was
    /// shipped WITHOUT `reject_canonical_symlink`, and untested (the
    /// missed-sibling-callsite pattern TD-VSDD-060 exists to catch).
    /// Regression guard: an `Edit` against a symlinked canonical must
    /// refuse loud with `E-SHD-010`, never `stat()` through the symlink.
    #[cfg(unix)]
    #[test]
    fn test_FINDING3_edit_arm_symlink_canonical_fails_loud_e_shd_010() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir.path().join("decision-log.md");
        let sensitive_target = dir.path().join("sensitive-elsewhere.txt");
        std::fs::write(
            &sensitive_target,
            "attacker wants THIS content exfiltrated/sealed",
        )
        .expect("seed the symlink target file");
        std::os::unix::fs::symlink(&sensitive_target, &canonical)
            .expect("plant a symlink at the governed canonical path");

        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };

        let result = shard_cap_gate_check(
            &registry,
            "Edit",
            &canonical,
            &serde_json::json!({"old_string": "a", "new_string": "ab"}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("E-SHD-010"),
                    "Finding #3: a symlinked canonical path at the Edit-arm's own \
                     current_shard_bytes_flat call site MUST fail loud naming the E-SHD-010 \
                     error code specifically — got a different HookResult::Error message: \
                     {message:?}"
                );
            }
            other => panic!(
                "Finding #3: a symlinked canonical path MUST fail LOUD as HookResult::Error \
                 naming E-SHD-010 — NOT silently stat() through the symlink (info-exposure side \
                 channel) nor Continue/Block as if it were an ordinary file. Got: {other:?}"
            ),
        }

        assert_eq!(
            std::fs::read_to_string(&sensitive_target).expect("sensitive target must still exist"),
            "attacker wants THIS content exfiltrated/sealed",
            "Finding #3: the symlink target's content must be completely untouched"
        );
        assert!(
            !dir.path().join("decision-log.0001.md").exists(),
            "Finding #3: no sealed shard may ever be published from a refused symlinked \
             canonical"
        );
    }

    /// Companion to the test above (Finding #3): the `MultiEdit` arm's own
    /// `current_shard_bytes_flat(target_path)` call site was likewise
    /// shipped without the guard and untested.
    #[cfg(unix)]
    #[test]
    fn test_FINDING3_multiedit_arm_symlink_canonical_fails_loud_e_shd_010() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir.path().join("decision-log.md");
        let sensitive_target = dir.path().join("sensitive-elsewhere.txt");
        std::fs::write(
            &sensitive_target,
            "attacker wants THIS content exfiltrated/sealed",
        )
        .expect("seed the symlink target file");
        std::os::unix::fs::symlink(&sensitive_target, &canonical)
            .expect("plant a symlink at the governed canonical path");

        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };

        let result = shard_cap_gate_check(
            &registry,
            "MultiEdit",
            &canonical,
            &serde_json::json!({"edits": [{"old_string": "a", "new_string": "ab"}]}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("E-SHD-010"),
                    "Finding #3: a symlinked canonical path at the MultiEdit-arm's own \
                     current_shard_bytes_flat call site MUST fail loud naming the E-SHD-010 \
                     error code specifically — got a different HookResult::Error message: \
                     {message:?}"
                );
            }
            other => panic!(
                "Finding #3: a symlinked canonical path MUST fail LOUD as HookResult::Error \
                 naming E-SHD-010 — NOT silently stat() through the symlink (info-exposure side \
                 channel) nor Continue/Block as if it were an ordinary file. Got: {other:?}"
            ),
        }

        assert_eq!(
            std::fs::read_to_string(&sensitive_target).expect("sensitive target must still exist"),
            "attacker wants THIS content exfiltrated/sealed",
            "Finding #3: the symlink target's content must be completely untouched"
        );
        assert!(
            !dir.path().join("decision-log.0001.md").exists(),
            "Finding #3: no sealed shard may ever be published from a refused symlinked \
             canonical"
        );
    }

    /// PR #824 pr-review Finding #3 companion note: `read_changelog_item_count`'s
    /// `File::open` (the `"frontmatter-changelog-array"` shape's own
    /// canonical-path read site) is likewise unguarded against a symlinked
    /// canonical path — same CWE-59/CWE-200 exfiltration concern as the
    /// `"flat"` shape's three mutation-tool arms, just via a different
    /// shape/read-cost path. Regression guard for the guard installed
    /// before `shard_cap_gate_check`'s `ShardShape::FrontmatterChangelogArray`
    /// arm calls `read_changelog_item_count`.
    #[cfg(unix)]
    #[test]
    fn test_FINDING3_frontmatter_changelog_arm_symlink_canonical_fails_loud_e_shd_010() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical = dir.path().join("BC-INDEX.md");
        let sensitive_target = dir.path().join("sensitive-elsewhere.txt");
        std::fs::write(
            &sensitive_target,
            "---\nchangelog:\n  - version: \"1.0\"\n---\n",
        )
        .expect("seed the symlink target file");
        std::os::unix::fs::symlink(&sensitive_target, &canonical)
            .expect("plant a symlink at the governed canonical path");

        let mut entry = flat_entry("BC-INDEX", 49_152);
        entry.shape = Some(ShardShape::FrontmatterChangelogArray);
        entry.n = Some(50);
        let registry = ShardRegistry {
            shards: vec![entry],
        };

        let result = shard_cap_gate_check(
            &registry,
            "Edit",
            &canonical,
            &serde_json::json!({"old_string": "a", "new_string": "ab"}),
        );

        match result {
            HookResult::Error { message } => {
                assert!(
                    message.contains("E-SHD-010"),
                    "Finding #3: a symlinked canonical path at the FrontmatterChangelogArray \
                     arm's read_changelog_item_count call site MUST fail loud naming the \
                     E-SHD-010 error code specifically — got a different HookResult::Error \
                     message: {message:?}"
                );
            }
            other => panic!(
                "Finding #3: a symlinked canonical path MUST fail LOUD as HookResult::Error \
                 naming E-SHD-010 — NOT silently read through the symlink (info-exposure side \
                 channel). Got: {other:?}"
            ),
        }
    }

    /// FIX-HIGH-1 (S-25.02 PR #824 second-security-review, HIGH, CWE-59/
    /// CWE-367/CWE-377): a co-resident local process could pre-plant a
    /// symlink at `write_exclusive`'s temp-file path before it ever runs
    /// (originally exploitable because that path was fully deterministic
    /// — `.{basename}.tmp-{pid}`, `pid` observable via `ps`/`/proc`; PR
    /// #824 pr-review Finding #2 later added a random nonce component,
    /// which this test pins to a KNOWN value via `stage_temp_file_with_nonce`
    /// so the fixture can still predict — and plant a symlink at — the
    /// exact path, without weakening production's own real randomness).
    /// This regression guard confirms staging refuses loud (`create_new`'s
    /// `O_EXCL` semantics) rather than writing the sealed content THROUGH
    /// the symlink into an arbitrary attacker-chosen file.
    #[cfg(unix)]
    #[test]
    fn test_FIXHIGH1_write_exclusive_refuses_to_follow_preplanted_symlink_at_temp_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sealed_path = dir.path().join("decision-log.0001.md");
        let basename = sealed_path
            .file_name()
            .expect("sealed_path has a filename")
            .to_string_lossy()
            .into_owned();
        const KNOWN_NONCE: u64 = 0xDEAD_BEEF_0BAD_F00D;
        // The EXACT temp-file path `stage_temp_file_with_nonce` itself
        // computes for this fixed nonce.
        let tmp_path = dir.path().join(format!(
            ".{basename}.tmp-{}-{KNOWN_NONCE:016x}",
            std::process::id()
        ));

        let attacker_target = dir.path().join("attacker-target.txt");
        std::fs::write(&attacker_target, "pre-existing attacker-owned content")
            .expect("seed the attacker's target file");
        std::os::unix::fs::symlink(&attacker_target, &tmp_path)
            .expect("pre-plant a symlink at the exact temp-file path");

        let err = stage_temp_file_with_nonce(&sealed_path, b"real sealed content", KNOWN_NONCE)
            .expect_err(
                "FIX-HIGH-1: staging MUST refuse loud when a symlink occupies its temp-file \
                 path — never follow it to write through",
            );
        assert!(
            matches!(err, StageError::TempPathOccupied(_)),
            "FIX-HIGH-1: create_new's O_EXCL semantics must surface as TempPathOccupied for a \
             pre-existing symlink at the temp path (dangling or not) — got: {err:?}"
        );

        assert_eq!(
            std::fs::read_to_string(&attacker_target)
                .expect("attacker target must still be readable"),
            "pre-existing attacker-owned content",
            "FIX-HIGH-1: the attacker's target file must be completely untouched — staging \
             must never have written the sealed content THROUGH the pre-planted symlink"
        );
        assert!(
            !sealed_path.exists(),
            "FIX-HIGH-1: the sealed destination must never come into existence when staging \
             itself already failed — the hard-link step is never even reached"
        );
        assert!(
            std::fs::symlink_metadata(&tmp_path)
                .expect("lstat the temp path after refusal")
                .file_type()
                .is_symlink(),
            "FIX-HIGH-1: the pre-planted symlink at the temp path must be left untouched — \
             staging's best-effort cleanup only ever removes a temp file IT created, never a \
             pre-existing symlink it refused to write through"
        );
    }

    /// FIX-MED-1 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-367):
    /// `reclaim_identity_still_safe`'s open-handle re-check must reject a
    /// path whose content changed since a hypothetical earlier probe —
    /// simulating the TOCTOU race window a genuinely concurrent legitimate
    /// writer could otherwise land in between `publish_sealed_shard`'s
    /// `is_zero_byte` probe and its `remove_file` unlink.
    #[test]
    fn test_FIXMED1_reclaim_identity_still_safe_rejects_content_changed_since_first_probe() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("decision-log.0001.md");

        // What an earlier `is_zero_byte` probe would have observed: a
        // genuinely 0-byte file at the destination.
        std::fs::write(&path, []).expect("seed 0-byte placeholder");
        assert!(
            reclaim_identity_still_safe(&path),
            "FIX-MED-1 precondition: a genuinely 0-byte regular file must be judged \
             reclaim-safe by the re-check"
        );

        // SIMULATE the race window: a second, legitimate concurrent writer
        // replaces the 0-byte placeholder with real sealed content between
        // the first probe and the unlink.
        std::fs::write(
            &path,
            b"real sealed content a concurrent writer just published",
        )
        .expect("simulate a concurrent writer replacing the 0-byte placeholder");

        assert!(
            !reclaim_identity_still_safe(&path),
            "FIX-MED-1: the re-check MUST reject reclaiming once the file's content has changed \
             since the first probe — a concurrent writer's genuine sealed content must never be \
             silently unlinked and discarded"
        );
    }

    /// FIX-MED-1 companion coverage: a path that vanished (or never
    /// existed) between the two checks must never be treated as
    /// reclaim-safe — a failed/inconclusive open is "not confirmed safe",
    /// never assumed safe.
    #[test]
    fn test_FIXMED1_reclaim_identity_still_safe_rejects_missing_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("decision-log.0001.md");
        assert!(
            !reclaim_identity_still_safe(&path),
            "FIX-MED-1: a path that vanished (or never existed) must never be treated as \
             reclaim-safe — File::open failing is not evidence of safety"
        );
    }

    /// N-4 (PR #824 pr-review cycle 2, MINOR): the two tests immediately
    /// above pin the content-changed and missing-path arms, but neither
    /// covers the Unix `O_NOFOLLOW` property Finding #4 added — untested
    /// per the reviewer. Plants a symlink AT the checked path pointing at a
    /// genuinely 0-byte, genuinely regular target — the only shape that
    /// could otherwise fool a DEREFERENCING check into wrongly reporting
    /// `true` — and asserts the re-check still refuses it: `O_NOFOLLOW`
    /// makes the open itself fail for the symlink, never silently
    /// dereferencing through to the target's metadata.
    #[cfg(unix)]
    #[test]
    fn test_N4_reclaim_identity_still_safe_rejects_symlink_via_o_nofollow() {
        let dir = tempfile::tempdir().expect("tempdir");
        let target = dir.path().join("zero-byte-target.md");
        std::fs::write(&target, []).expect("seed a genuinely 0-byte regular target");

        let path = dir.path().join("decision-log.0001.md");
        std::os::unix::fs::symlink(&target, &path)
            .expect("plant a symlink at the checked path, pointing at a 0-byte regular file");

        assert!(
            !reclaim_identity_still_safe(&path),
            "N-4: a symlink at the checked path must be rejected via O_NOFOLLOW even when its \
             TARGET is a genuinely 0-byte regular file — the open itself must fail for the \
             symlink, never silently dereference through to report on the target's metadata"
        );
    }

    /// PR #824 pr-review Finding #8 (MINOR): the two tests immediately
    /// above pin `reclaim_identity_still_safe`'s OWN logic in isolation,
    /// but neither drives the re-verify-before-unlink guarantee through the
    /// REAL call site — `publish_sealed_shard`'s 0-byte-reclaim branch
    /// (which calls `reclaim_identity_still_safe` immediately before its
    /// `remove_file` unlink — restored by MAJOR-1's reorder, PR #824
    /// pr-review cycle 3, after Finding #2's staged-write insertion had
    /// briefly widened that adjacency; see
    /// `test_MAJOR1_reclaim_identity_recheck_runs_immediately_before_unlink_not_before_staging`
    /// below for the ordering-pinning regression coverage). A regression in
    /// how the CALLER wires this check in (dropping the
    /// `if !reclaim_identity_still_safe(...)` guard entirely, inverting it,
    /// or ignoring its return value) would not be caught by either
    /// helper-level test above — both call the helper directly and never
    /// touch `publish_sealed_shard`.
    ///
    /// This test deterministically (no thread races, no flakiness)
    /// reproduces the SHAPE of the TOCTOU window `publish_sealed_shard`'s
    /// own doc comment describes: a destination whose EARLIER, coarser
    /// probe (`std::fs::symlink_metadata` — "not a symlink AND exactly 0
    /// bytes") judges reclaimable, but whose LATER, finer re-check
    /// (`reclaim_identity_still_safe`'s open-handle-based "is a REGULAR
    /// file AND exactly 0 bytes") disagrees. A FIFO produces exactly this
    /// disagreement with NO timing dependency at all: `symlink_metadata`
    /// on a FIFO with no writer connected reports `is_symlink() == false`
    /// and `len() == 0` (so the coarse probe says "reclaimable" — verified
    /// as this test's own precondition below), while
    /// `reclaim_identity_still_safe`'s `is_file()` check is `false` for a
    /// FIFO (so the fine re-check correctly disagrees) — the same
    /// "identity changed since the first probe" outcome a genuine
    /// concurrent writer would also produce, reproduced here via a stable
    /// file-type mismatch rather than a race window that would make the
    /// test flaky.
    ///
    /// Asserts the abort surfaces through `publish_sealed_shard` itself
    /// (never the private helper in isolation): the call must return
    /// `Err(ShardRollError::SealedShardAlreadyExists)` (E-SHD-009), the
    /// FIFO at the destination must be left COMPLETELY UNTOUCHED (never
    /// unlinked), and the new content passed in must never be published
    /// anywhere. If the caller's wiring regresses, this test fails: the
    /// reclaim would instead unlink the FIFO and publish the new content,
    /// producing `Ok(())` with the FIFO gone.
    #[cfg(unix)]
    #[test]
    fn test_FIXMED1_publish_sealed_shard_callsite_aborts_reclaim_when_recheck_disagrees_with_initial_probe()
     {
        use std::os::unix::fs::FileTypeExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let sealed_path = dir.path().join("decision-log.0001.md");
        let status = std::process::Command::new("mkfifo")
            .arg(&sealed_path)
            .status()
            .expect("mkfifo must be available on this platform to run this test");
        assert!(
            status.success(),
            "mkfifo must succeed in creating the FIFO fixture at the seal destination"
        );

        // Precondition: confirm the FIFO reproduces the exact disagreement
        // this test depends on — publish_sealed_shard's own coarse
        // is_zero_byte probe must judge it reclaimable (so the call
        // actually reaches the reclaim_identity_still_safe re-check
        // rather than being refused earlier by the `!is_zero_byte`
        // branch), while the fine re-check must disagree.
        let coarse_probe_says_reclaimable = std::fs::symlink_metadata(&sealed_path)
            .map(|meta| !meta.file_type().is_symlink() && meta.len() == 0)
            .unwrap_or(false);
        assert!(
            coarse_probe_says_reclaimable,
            "precondition: publish_sealed_shard's own coarse is_zero_byte probe must see this \
             FIFO as reclaimable (not a symlink, reports 0 bytes) for this test to actually \
             exercise the re-check's disagreement rather than the earlier `!is_zero_byte` \
             refusal"
        );
        assert!(
            !reclaim_identity_still_safe(&sealed_path),
            "precondition: the fine re-check must disagree with the coarse probe for a FIFO — a \
             FIFO is never a 0-byte REGULAR file"
        );

        let new_content = b"NEW-SEAL-CONTENT-MUST-NEVER-BE-PUBLISHED-OVER-THE-FIFO";
        let err = publish_sealed_shard(&sealed_path, new_content).expect_err(
            "FIX-MED-1 call-site regression: publish_sealed_shard must abort the 0-byte reclaim \
             (never proceed to unlink) when the re-check disagrees with the earlier coarse \
             probe — a caller-side regression that stopped invoking, or stopped respecting the \
             return value of, reclaim_identity_still_safe would instead report Ok(()) here",
        );
        assert!(
            matches!(err, ShardRollError::SealedShardAlreadyExists { .. }),
            "FIX-MED-1: the abort must surface as ShardRollError::SealedShardAlreadyExists \
             (E-SHD-009) — got: {err:?}"
        );
        assert!(
            err.to_string().contains("E-SHD-009"),
            "FIX-MED-1: the error's Display text must name the E-SHD-009 code — got: {err}"
        );

        let post_call_meta = std::fs::symlink_metadata(&sealed_path).expect(
            "FIX-MED-1 call-site regression: the destination must still exist — an aborted \
             reclaim must never unlink it",
        );
        assert!(
            post_call_meta.file_type().is_fifo(),
            "FIX-MED-1 call-site regression: the pre-existing FIFO at the seal destination must \
             be left COMPLETELY UNTOUCHED when the reclaim aborts — it must never be unlinked, \
             regardless of what publish_sealed_shard's internal staging did with the new \
             content"
        );
    }

    /// MAJOR-1 (PR #824 pr-review cycle 3): pins `publish_sealed_shard`'s
    /// STAGE-then-RECHECK ordering at the real call site — the property the
    /// FIX-MED-1 FIFO test immediately above does NOT pin. Per the
    /// reviewer's own words: "The Finding #8 FIFO test gates on the
    /// re-check existing, not on where it sits, so it passes under either
    /// ordering." Finding #2's `ef6ca3b4` fix had inserted a full staged
    /// write (including an `fsync`/`sync_all`) BETWEEN FIX-MED-1's re-check
    /// and the unlink, silently re-widening the CWE-367 TOCTOU window
    /// FIX-MED-1 exists to close — a regression neither test above would
    /// catch, since both only observe the FINAL error variant/on-disk
    /// state, which is identical either way in THEIR fixtures.
    ///
    /// Combines a FIFO destination (deterministically disagrees with the
    /// earlier coarse `is_zero_byte` probe, exactly as the FIX-MED-1 test
    /// above establishes) with [`force_stage_temp_file_failure`] armed for
    /// call #1 (the reclaim retry's OWN `stage_temp_file` call — never the
    /// initial `write_exclusive` attempt, call #0, which succeeds normally
    /// here and merely collides at the `hard_link` step). This produces two
    /// OBSERVATIONALLY DISTINCT outcomes depending on ordering:
    ///
    /// - **Correct (current) ordering** — `stage_temp_file` called BEFORE
    ///   `reclaim_identity_still_safe`: call #1 is reached, is forced to
    ///   fail, and `publish_sealed_shard` returns
    ///   `Err(ShardRollError::SealWriteFailed)` (E-SHD-001) WITHOUT ever
    ///   reaching the re-check — the FIFO's identity mismatch is never
    ///   consulted at all.
    /// - **Regressed ordering** — `reclaim_identity_still_safe` called
    ///   BEFORE `stage_temp_file` (the shape MAJOR-1 fixed): the re-check
    ///   runs FIRST, sees the FIFO's identity mismatch immediately, and
    ///   aborts with `Err(ShardRollError::SealedShardAlreadyExists)`
    ///   (E-SHD-009) WITHOUT ever reaching the staging call — the forced
    ///   failure is armed but never consumed.
    ///
    /// Asserting `SealWriteFailed` therefore fails immediately (observing
    /// `SealedShardAlreadyExists` instead) if a future change moves the
    /// re-check back above the staging call. Manually verified
    /// red-without-fix: temporarily swapping the `stage_temp_file` and
    /// `reclaim_identity_still_safe` calls back to the pre-MAJOR-1 order in
    /// `publish_sealed_shard` and re-running this test in isolation
    /// reproduces exactly the `SealedShardAlreadyExists` mismatch this
    /// assertion is written to catch.
    #[cfg(unix)]
    #[test]
    fn test_MAJOR1_reclaim_identity_recheck_runs_immediately_before_unlink_not_before_staging() {
        use std::os::unix::fs::FileTypeExt;

        let dir = tempfile::tempdir().expect("tempdir");
        let sealed_path = dir.path().join("decision-log.0001.md");
        let status = std::process::Command::new("mkfifo")
            .arg(&sealed_path)
            .status()
            .expect("mkfifo must be available on this platform to run this test");
        assert!(
            status.success(),
            "mkfifo must succeed in creating the FIFO fixture at the seal destination"
        );

        // Force ONLY the reclaim retry's OWN staging call (call #1) to
        // fail — never the initial write_exclusive attempt (call #0).
        let _guard = force_stage_temp_file_failure(1, ForcedStageFailureKind::Io);

        let new_content = b"MAJOR1-ORDERING-PROBE-CONTENT-MUST-NEVER-BE-PUBLISHED";
        let err = publish_sealed_shard(&sealed_path, new_content).expect_err(
            "MAJOR-1 ordering regression: publish_sealed_shard must attempt to STAGE the \
             retry's content BEFORE ever re-checking the destination's identity — a forced \
             staging failure at the reclaim retry's own call site must surface here",
        );
        assert!(
            matches!(err, ShardRollError::SealWriteFailed { .. }),
            "MAJOR-1: expected ShardRollError::SealWriteFailed (E-SHD-001) — stage_temp_file \
             must run BEFORE reclaim_identity_still_safe, so a forced staging failure surfaces \
             before the FIFO's identity mismatch is ever consulted. Got {err:?} instead — if \
             this is ShardRollError::SealedShardAlreadyExists, reclaim_identity_still_safe ran \
             BEFORE stage_temp_file, which is exactly the ordering regression MAJOR-1 (PR #824 \
             pr-review cycle 3) fixed: the re-check must sit immediately before the unlink, \
             AFTER staging completes, never before it."
        );

        let post_call_meta = std::fs::symlink_metadata(&sealed_path).expect(
            "MAJOR-1 ordering regression: the destination must still exist — a staging failure \
             must never have unlinked it first",
        );
        assert!(
            post_call_meta.file_type().is_fifo(),
            "MAJOR-1 ordering regression: the pre-existing FIFO at the seal destination must be \
             left COMPLETELY UNTOUCHED by a staging-side failure"
        );
    }

    /// N-1 (PR #824 pr-review cycle 2, MAJOR): Finding #2's `ef6ca3b4` fix
    /// (stage-then-publish, distinguishing a TEMP-path collision from a
    /// DESTINATION collision) had no call-site regression test driving it
    /// through the real `publish_sealed_shard` entrypoint — the only
    /// existing coverage called the private `stage_temp_file_with_nonce`
    /// helper directly (`test_FIXHIGH1_*`), which pins the helper's own
    /// error taxonomy but not how the CALLER reacts to it.
    ///
    /// This half proves the misattribution half of Finding #2: a
    /// temp-path-collision-shaped staging failure (forced via
    /// [`force_stage_temp_file_failure`] — the real per-call random nonce
    /// makes literally pre-occupying the exact temp path impossible for a
    /// test) at `publish_sealed_shard`'s FIRST (and only, for this
    /// scenario) staging attempt must surface as `SealWriteFailed`
    /// (E-SHD-001), never the misleading `SealedShardAlreadyExists`
    /// (E-SHD-009) — even though a pre-existing NON-EMPTY destination sits
    /// right there, which is exactly what a REGRESSED caller (one that
    /// reverted the `TempPathOccupied`/`Io` match arm to fall through into
    /// the 0-byte-reclaim block regardless of `WriteExclusiveError`
    /// variant) would misreport as E-SHD-009 after probing that destination.
    #[test]
    fn test_N1_publish_sealed_shard_callsite_temp_path_collision_never_misattributed_as_already_exists()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let sealed_path = dir.path().join("decision-log.0001.md");
        std::fs::write(&sealed_path, b"real pre-existing sealed history, non-empty")
            .expect("seed a non-empty pre-existing destination");

        // Force the FIRST (call #0) staging attempt to fail exactly as a
        // genuine temp-path collision would.
        let _guard = force_stage_temp_file_failure(0, ForcedStageFailureKind::TempPathOccupied);
        let new_content = b"NEW-CONTENT-MUST-NEVER-BE-PUBLISHED";
        let err = publish_sealed_shard(&sealed_path, new_content).expect_err(
            "N-1 call-site regression: a temp-path collision at the FIRST staging attempt must \
             surface as SealWriteFailed, never SealedShardAlreadyExists — reverting the \
             TempPathOccupied/Io match arm to fall through into the 0-byte-reclaim block would \
             instead probe the (non-empty) destination here and misreport E-SHD-009",
        );
        assert!(
            matches!(err, ShardRollError::SealWriteFailed { .. }),
            "N-1: expected ShardRollError::SealWriteFailed (E-SHD-001) for a temp-path collision \
             — got: {err:?}"
        );
        assert!(
            err.to_string().contains("E-SHD-001"),
            "N-1: the error's Display text must name E-SHD-001, never the misleading E-SHD-009 \
             — got: {err}"
        );
        assert_eq!(
            std::fs::read(&sealed_path).expect("destination must still be readable"),
            b"real pre-existing sealed history, non-empty",
            "N-1: the pre-existing non-empty destination must be completely untouched by a \
             staging-side failure"
        );
    }

    /// N-1 (PR #824 pr-review cycle 2, MAJOR): the other half of Finding #2
    /// — a staging failure on the 0-byte-reclaim RETRY (the SECOND
    /// `stage_temp_file` call within one `publish_sealed_shard` invocation)
    /// must never have unlinked the reclaimable destination first.
    /// `ef6ca3b4` moved staging BEFORE the unlink specifically so a failed
    /// retry-staging attempt leaves the destination COMPLETELY UNTOUCHED;
    /// restoring the pre-fix unlink-then-stage ordering would delete the
    /// destination before ever discovering the retry's own staging
    /// failure.
    ///
    /// Lets the FIRST staging attempt (call #0, inside `write_exclusive`)
    /// succeed for real — it fails at the hard_link step because
    /// `sealed_path` already exists (0 bytes), driving `publish_sealed_shard`
    /// into its 0-byte-reclaim branch — then forces ONLY the retry's own
    /// staging call (call #1) to fail.
    #[test]
    fn test_N1_publish_sealed_shard_callsite_failed_retry_staging_never_destroys_reclaimable_destination()
     {
        let dir = tempfile::tempdir().expect("tempdir");
        let sealed_path = dir.path().join("decision-log.0001.md");
        std::fs::write(&sealed_path, []).expect("seed a reclaimable 0-byte destination");

        let _guard = force_stage_temp_file_failure(1, ForcedStageFailureKind::Io);
        let new_content = b"NEW-CONTENT-MUST-NEVER-BE-PUBLISHED-EITHER";
        let err = publish_sealed_shard(&sealed_path, new_content).expect_err(
            "N-1 call-site regression: a staging failure on the 0-byte-reclaim RETRY must \
             surface as SealWriteFailed and must never have unlinked the reclaimable \
             destination first — restoring the pre-fix unlink-then-stage ordering would delete \
             the destination BEFORE discovering the retry's own staging failure",
        );
        assert!(
            matches!(err, ShardRollError::SealWriteFailed { .. }),
            "N-1: expected ShardRollError::SealWriteFailed (E-SHD-001) for the retry's staging \
             failure — got: {err:?}"
        );

        let meta = std::fs::symlink_metadata(&sealed_path).expect(
            "N-1 call-site regression: the reclaimable 0-byte destination must still exist on \
             disk — a staging failure on the retry must NEVER have unlinked it first",
        );
        assert!(
            meta.file_type().is_file() && meta.len() == 0,
            "N-1: the destination must still be the SAME untouched 0-byte regular file — got: \
             {meta:?}"
        );
    }

    /// PR #824 pr-review Finding #4 (MINOR): a plain blocking
    /// `std::fs::File::open` on a FIFO with NO writer connected blocks
    /// INDEFINITELY — hanging the PreToolUse dispatch this check gates.
    /// This regression guard plants a real FIFO (via the `mkfifo` utility
    /// — no new crate dependency) with no writer ever connected and
    /// asserts `reclaim_identity_still_safe` returns PROMPTLY (the test
    /// process itself would hang forever otherwise, which is the actual
    /// property under test — a bounded external `timeout` wraps this test
    /// run in CI/local verification as an extra safety net, but the
    /// production fix itself must never depend on that wrapper).
    #[cfg(unix)]
    #[test]
    fn test_FINDING4_reclaim_identity_still_safe_does_not_hang_on_fifo() {
        let dir = tempfile::tempdir().expect("tempdir");
        let fifo_path = dir.path().join("decision-log.0001.md");
        let status = std::process::Command::new("mkfifo")
            .arg(&fifo_path)
            .status()
            .expect("mkfifo must be available on this platform to run this test");
        assert!(
            status.success(),
            "mkfifo must succeed in creating the FIFO fixture"
        );

        // No writer is EVER connected — a plain blocking File::open(fifo)
        // would hang here forever. A FIFO can never be a 0-byte REGULAR
        // file either way (is_file() is false for a FIFO), so the correct
        // answer is `false` — but the point of this test is that the call
        // returns AT ALL.
        assert!(
            !reclaim_identity_still_safe(&fifo_path),
            "Finding #4: a FIFO is never a 0-byte regular file, so this must return false — and \
             it must do so promptly, never hang waiting for a writer that will never connect"
        );
    }

    /// FIX-MED-2 (S-25.02 PR #824 second-security-review, MEDIUM, CWE-59/
    /// CWE-200): `execute_roll` — the main roll path, step (a) — must
    /// refuse loud with `E-SHD-010` (`CanonicalPathIsSymlink`) when
    /// `canonical_path` is a symlink, rather than reading the symlink
    /// TARGET's bytes via `read_canonical_content` and durably sealing
    /// them into a brand-new regular file (a real exfiltration channel).
    #[cfg(unix)]
    #[test]
    fn test_FIXMED2_execute_roll_refuses_symlinked_canonical_path() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        let sensitive_target = dir.path().join("sensitive-elsewhere.txt");
        std::fs::write(
            &sensitive_target,
            "attacker wants THIS content exfiltrated/sealed",
        )
        .expect("seed the symlink target file");
        std::os::unix::fs::symlink(&sensitive_target, &canonical_path)
            .expect("plant a symlink at the governed canonical path");

        let err = execute_roll(&entry, &canonical_path, false).expect_err(
            "FIX-MED-2: execute_roll MUST refuse loud against a symlinked canonical path, never \
             read through it and seal the target's content",
        );
        assert!(
            matches!(err, ShardRollError::CanonicalPathIsSymlink { .. }),
            "FIX-MED-2: expected ShardRollError::CanonicalPathIsSymlink (E-SHD-010) — got: \
             {err:?}"
        );
        assert!(
            err.to_string().contains("E-SHD-010"),
            "FIX-MED-2: the error's Display text must name the E-SHD-010 code — got: {err}"
        );

        assert_eq!(
            std::fs::read_to_string(&sensitive_target).expect("sensitive target must still exist"),
            "attacker wants THIS content exfiltrated/sealed",
            "FIX-MED-2: the symlink target's content must be completely untouched"
        );
        assert!(
            !dir.path().join("decision-log.0001.md").exists(),
            "FIX-MED-2: no sealed shard may ever be published from a refused symlinked \
             canonical — the symlink target's bytes must never end up durably sealed anywhere"
        );
    }

    /// FIX-MED-2: `self_heal_resume_from_truncate` must refuse loud with
    /// `E-SHD-010` when `canonical_path` is a symlink, rather than reading
    /// the symlink target's bytes for its byte-identity comparison.
    #[cfg(unix)]
    #[test]
    fn test_FIXMED2_self_heal_resume_from_truncate_refuses_symlinked_canonical() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        let sensitive_target = dir.path().join("sensitive-elsewhere.txt");
        std::fs::write(
            &sensitive_target,
            "attacker wants THIS content exfiltrated/sealed",
        )
        .expect("seed the symlink target file");
        std::os::unix::fs::symlink(&sensitive_target, &canonical_path)
            .expect("plant a symlink at the governed canonical path");

        let err = self_heal_resume_from_truncate(&entry, &canonical_path).expect_err(
            "FIX-MED-2: self_heal_resume_from_truncate MUST refuse loud against a symlinked \
             canonical path, never read through it",
        );
        assert!(
            matches!(err, ShardRollError::CanonicalPathIsSymlink { .. }),
            "FIX-MED-2: expected ShardRollError::CanonicalPathIsSymlink (E-SHD-010) — got: \
             {err:?}"
        );
        assert!(
            err.to_string().contains("E-SHD-010"),
            "FIX-MED-2: the error's Display text must name the E-SHD-010 code — got: {err}"
        );
    }

    /// FIX-MED-2: `reconcile_post_write_replace_all_overcap` must refuse
    /// loud with `E-SHD-010` when `canonical_path` is a symlink, rather
    /// than statting through it (and, via the retroactive roll it would
    /// otherwise trigger, reading and sealing the symlink target's bytes).
    #[cfg(unix)]
    #[test]
    fn test_FIXMED2_reconcile_post_write_replace_all_overcap_refuses_symlinked_canonical() {
        let dir = tempfile::tempdir().expect("tempdir");
        let canonical_path = dir.path().join("decision-log.md");
        let entry = flat_entry("decision-log", 49_152);
        let sensitive_target = dir.path().join("sensitive-elsewhere.txt");
        std::fs::write(&sensitive_target, "y".repeat(49_500))
            .expect("seed an over-cap-sized symlink target file");
        std::os::unix::fs::symlink(&sensitive_target, &canonical_path)
            .expect("plant a symlink at the governed canonical path");

        let err = reconcile_post_write_replace_all_overcap(&entry, &canonical_path).expect_err(
            "FIX-MED-2: reconcile_post_write_replace_all_overcap MUST refuse loud against a \
             symlinked canonical path, never stat/read through it",
        );
        assert!(
            matches!(err, ShardRollError::CanonicalPathIsSymlink { .. }),
            "FIX-MED-2: expected ShardRollError::CanonicalPathIsSymlink (E-SHD-010) — got: \
             {err:?}"
        );
        assert!(
            err.to_string().contains("E-SHD-010"),
            "FIX-MED-2: the error's Display text must name the E-SHD-010 code — got: {err}"
        );
        assert!(
            !dir.path().join("decision-log.0001.md").exists(),
            "FIX-MED-2: no sealed shard may ever be published from a refused symlinked \
             canonical"
        );
    }
}
