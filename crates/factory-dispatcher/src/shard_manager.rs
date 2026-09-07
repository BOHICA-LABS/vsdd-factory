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
//! # Scope note (S-25.02 F4 BC-cluster 1 "cap+trigger")
//!
//! This module implements BC-1.18.005 ONLY (tasks T-1/T-2/T-3; AC-001..AC-005)
//! — fully, not as a stub; see the "BC-5.38.001 Red Gate discipline" section
//! below. BC-1.18.006 (the observable roll/block outcome once the `"flat"` trigger
//! fires), BC-1.18.009 (the observable rotate/block-and-retry outcome once
//! the item-count trigger fires), and BC-1.18.012 (the one-time changelog
//! backfill migration) are LATER clusters and are explicitly OUT OF SCOPE
//! here — this module owns the trigger-boundary decision and the hand-off
//! point only, per Postcondition 3's and Postcondition 8's "Ownership"
//! bullets.
//!
//! # BC-5.38.001 Red Gate discipline — implemented (S-25.02 F4 BC-cluster 1)
//!
//! Every function below now carries a real implementation driving the
//! test-writer's Red Gate suite green. A fired trigger (either shape) never
//! constructs `HookResult::Block` from this module — that observable
//! roll/rotate-and-retry outcome is owned by the later BC-1.18.006 /
//! BC-1.18.009 clusters (see the "Scope note" above); this module surfaces a
//! fired trigger as a non-fatal `tracing::warn!` advisory and returns
//! `Continue`, an honest hand-off rather than a fabricated block.

use std::io;
use std::io::Read as _;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

use vsdd_hook_sdk::HookResult;

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
/// 2. `worst_case_fuel_per_byte` MUST be finite and strictly positive
///    (fail-loud [`ShardConfigError::InvalidWorstCaseFuelPerByte`] —
///    EC-015's "divisor-door" closure) — checked BEFORE
///    `compute_shard_cap_bytes` is ever called for this entry, since a
///    `0.0`/non-finite divisor would otherwise saturate the computed
///    ceiling toward `u64::MAX`, defeating check 4 below for ANY declared
///    `shard_cap_bytes`.
/// 3. The RAW `practical_fuel_ceiling as f64 / worst_case_fuel_per_byte`
///    division result MUST be finite and `< u64::MAX as f64` (fail-loud
///    [`ShardConfigError::FormulaCeilingSaturated`] — EC-017's residual
///    divisor-door closure) — a legal-but-tiny-positive divisor can still
///    saturate the computed ceiling even though it passes check 2.
/// 4. `shard_cap_bytes` MUST NOT exceed
///    `compute_shard_cap_bytes(entry.cap_formula_inputs())` (fail-loud
///    [`ShardConfigError::CapExceedsFormulaCeiling`] — Postcondition 9 /
///    EC-013; the `==` boundary is inclusive, mirroring EC-002's precedent
///    for the per-write trigger) — applies to EVERY `shape`, not
///    `"flat"`-only.
/// 5. For a `"frontmatter-changelog-array"`-shaped entry ONLY: `n` MUST be
///    present (fail-loud [`ShardConfigError::MissingN`] — EC-016), checked
///    BEFORE `low_water_mark` is examined, so an entry missing `n` AND
///    declaring an out-of-range `low_water_mark` is never silently accepted
///    merely because `n` was absent (see EC-016's ordering vector).
/// 6. For a `"frontmatter-changelog-array"`-shaped entry with a PRESENT `n`
///    ONLY: `n` MUST be `>= 1` (fail-loud
///    [`ShardConfigError::ZeroItemCountThreshold`] — PR #818 fix-burst
///    finding m2), checked immediately after check 5 and BEFORE
///    `low_water_mark` is examined — `n = 0` makes the item-count trigger
///    fire unconditionally AND makes any explicit `low_water_mark` value
///    unsatisfiable.
/// 7. For a `"frontmatter-changelog-array"`-shaped entry with an EXPLICIT
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
    let target: Vec<_> = target_path.components().collect();
    let registered: Vec<_> = registered_path.components().collect();

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
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(0),
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
/// "Ownership" bullet) — out of scope for this cluster.
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
        // Any OTHER io::Error kind stays fail-loud.
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(0),
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

            // F-002 fix (S-25.02 Phase F4 LOCAL adversary pass-1 cluster-1,
            // MEDIUM): `current_shard_bytes_flat` (a stat()/metadata() call)
            // is ONLY needed by the Edit/MultiEdit legs of Postcondition 3's
            // CORRECTED formula — `Write`'s `projected_size = len(content)`
            // alone never consults `current_shard_bytes` at all. Calling it
            // unconditionally here (i.e. before this match) would fail a
            // `Write` on a stat() error the Write formula doesn't even need
            // (e.g. a non-`NotFound` I/O error such as `ELOOP`), which is
            // never sound: `Write` must never be blocked by a read it
            // doesn't perform. So the stat() call is pushed down into ONLY
            // the Edit/MultiEdit arms below, each of which does need
            // `current_shard_bytes` for the current+delta formula.
            // EC-004 (a missing shard file treated as size 0) is preserved:
            // `current_shard_bytes_flat` itself still maps `NotFound` to
            // `Ok(0)`, unchanged.
            let projected_size = match tool_kind {
                ToolKind::Write => {
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
                    let current_bytes = match current_shard_bytes_flat(target_path) {
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
                    let current_bytes = match current_shard_bytes_flat(target_path) {
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
                // Postcondition 3's "Ownership" bullet: this BC owns the
                // trigger-boundary DECISION only. BC-1.18.006 owns the
                // observable roll/block outcome once it fires — that BC is
                // explicitly out of scope for this cluster (see this
                // module's own "Scope note"), so this function itself NEVER
                // constructs a `HookResult::Block` for a fired trigger. The
                // fired decision is still surfaced (non-fatally) via
                // `tracing::warn!` so it is visible in telemetry rather than
                // silently swallowed while the roll implementation lands.
                tracing::warn!(
                    artifact_stem = %entry.artifact_stem,
                    projected_size,
                    shard_cap_bytes = entry.shard_cap_bytes,
                    "BC-1.18.005: byte-size shard-cap trigger fired; roll/block outcome is \
                     owned by BC-1.18.006 (not yet implemented in this cluster) — allowing \
                     the call to proceed"
                );
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
                // trigger fires — out of scope for this cluster (see this
                // module's own "Scope note"). Same non-Block, honest
                // hand-off posture as the "flat" shape's trigger-fired
                // branch above.
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
    // Write against a [[shard]]-matched path whose current shard cannot be
    // stat()-ed for a reason OTHER than NotFound (e.g. EC-004's already-
    // covered missing-file case) MUST NOT be blocked when the Write's own
    // content length is under cap. Postcondition 3's CORRECTED Write leg
    // computes projected_size = len(content) ALONE — current_shard_bytes
    // (and, by extension, any failure reading it) is irrelevant to Write.
    // ===================================================================

    #[cfg(unix)]
    #[test]
    fn test_BC_1_18_005_F002_write_under_cap_continues_despite_irrelevant_stat_failure() {
        // Same portable technique as the existing unmatched-path zero-cost
        // canary test above (a self-referential symlink makes ANY
        // stat()/metadata() call on this path fail with ELOOP) — but here the
        // path's STEM MATCHES a [[shard]] entry, so the gate does NOT
        // short-circuit before stat(); it must instead recognize that a
        // Write's formula never needs current_shard_bytes at all.
        //
        // Portability caveat: ELOOP via a self-referential symlink is a
        // Unix-only technique (`std::os::unix::fs::symlink`), hence
        // `#[cfg(unix)]` — mirroring this file's own existing precedent for
        // the same landmine (`test_BC_1_18_005_INV3_EC_001_...`). No portable
        // cross-platform non-NotFound stat() failure was substituted because
        // this exact technique is already the codebase's established pattern
        // for this class of test.
        let dir = tempfile::tempdir().expect("tempdir");
        let looped = dir.path().join("decision-log.md");
        std::os::unix::fs::symlink(&looped, &looped).expect("create self-referential symlink");

        let registry = ShardRegistry {
            shards: vec![flat_entry("decision-log", 49_152)],
        };
        let result = shard_cap_gate_check(
            &registry,
            "Write",
            &looped,
            &serde_json::json!({"content": "x".repeat(5_000)}),
        );
        assert_eq!(
            result,
            HookResult::Continue,
            "F-002: a Write's projected_size = len(content) alone (Postcondition 3 CORRECTED) — \
             current_shard_bytes, and any stat() failure reading it (here ELOOP on a \
             self-referential symlink), is irrelevant to the Write formula and MUST NOT block it. \
             shard_cap_gate_check's ToolKind::Write arm never calls current_shard_bytes_flat() at \
             all — that stat() call is pushed down into ONLY the Edit/MultiEdit arms (F-002 fix), \
             so this non-NotFound stat error on a Write's own target path is never observed."
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
        // One byte over the ceiling — stat() alone must be enough to reject
        // this without ever reading the file's content into memory.
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
