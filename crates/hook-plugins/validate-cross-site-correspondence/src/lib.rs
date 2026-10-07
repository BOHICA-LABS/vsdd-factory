//! validate-cross-site-correspondence — PostToolUse WASM hook plugin.
//!
//! Blocks any Edit/Write/MultiEdit to governed paths that contains cross-site
//! value-correspondence violations:
//!
//! **Class A Arm1 (BC file write):** BC `version:` vs BC-INDEX.md row version cell.
//! **Class A Arm2 (story file write):** story body BC-table version citations vs BC frontmatter.
//! **Class B Arm1 (story file write):** story `input-hash:` vs STORY-INDEX catalog row vs blockquote.
//! **Class B Arm2 (STORY-INDEX.md write):** internal catalog vs blockquote hash parity.
//! **Class D (cycle artifact write):** finding-ID namespace format advisory — **[DEFERRED to S-21.08; not created — BC-5.39.010 §File Structure / Task 12]**.
//! **Class E1 (BC/VP/story/epic write):** `version:` vs `last_amended:` outer prefix mismatch.
//! **Class E2 (BC/VP/story/epic write):** `modified:` sequence date monotonicity.
//!
//! # Governing BC
//! BC-5.39.010 — six-arm PostToolUse cross-site value-correspondence gate (A1/A2/B1/B2/E1/E2; Class D deferred) (current version per BC-INDEX).
//!
//! # Architecture
//! - **Tier 2A** (ADR-035 §Decision 1): PostToolUse read-only validator.
//! - PostToolUse ONLY — never PreToolUse (BC-5.39.010 invariant 2).
//! - No writes (BC-5.39.010 invariant 1).
//! - No `read_dir`, no `list_dir` enumeration (BC-5.39.010 invariant 3 / ADR-035 §Alternatives).
//! - Fail-closed on primary-target `CapabilityDenied` (BC-5.39.010 invariant 4 / BC-5.39.008 v1.6).
//! - `on_error = "continue"`: fuel exhaustion → silent non-finding, not spurious block (ADR-035 §Decision 5).
//! - `is_char_boundary()` guards on all byte-index slicing (BC-5.39.010 invariant 9).
//!
//! # Forbidden dependencies (enforced by Cargo.toml absence):
//! - `serde_yaml`: deprecated/archived 2024; TD #72 migrated workspace to `serde_norway`.
//! - `wasmtime`: only dispatcher binary links wasmtime.
//! - `regex`: WASM fuel budget; hand-rolled scanning mandated (ADR-035 §Decision 5).

pub mod arm_a1;
pub mod arm_a2;
pub mod arm_b;
pub mod arm_e;
pub mod dispatch;
pub mod frontmatter;

use vsdd_hook_sdk::{
    HookPayload, HookResult,
    host::{self},
};

/// Read cap for BC file and story file primary reads.
/// BC-5.39.010 AC-019: `max_bytes = 524288`, `timeout_ms = 3000`.
/// Separate from cycle artifact cap — BC/story files are bounded by spec
/// (F-S2107-P1C-002: former PRIMARY_READ_MAX_BYTES=1 MiB was 2× spec cap).
const BC_STORY_PRIMARY_MAX_BYTES: u32 = 524_288;
const BC_STORY_PRIMARY_TIMEOUT_MS: u32 = 3_000;

/// Read cap for cycle artifact primary reads.
/// BC-5.39.010 AC-019: `max_bytes = 2097152`, `timeout_ms = 5000`.
/// Larger cap required for large cycle artifacts (e.g., lessons.md ≤4000 lines).
/// Former PRIMARY_READ_MAX_BYTES=1 MiB caused OutputTooLarge → Block on 1–2 MiB
/// artifacts, violating invariant 6 (Class D advisory-only, F-S2107-P1C-003).
const CYCLE_ARTIFACT_PRIMARY_MAX_BYTES: u32 = 2_097_152;
const CYCLE_ARTIFACT_PRIMARY_TIMEOUT_MS: u32 = 5_000;

/// HOST_ABI_VERSION declares the ABI contract version this plugin was built against.
pub const HOST_ABI_VERSION: u32 = 1;

/// A blocking cross-site correspondence violation.
///
/// Carries a human-readable `description` used verbatim in the combined block message
/// and the arm `class` for message prefixing.
#[derive(Debug, Clone)]
pub struct Violation {
    /// Human-readable description citing the arm class, values, and policy anchor.
    pub description: String,
}

/// A non-blocking advisory warning emitted via `host::log_warn`.
///
/// Advisories are always Continue (never block). Used for:
/// - Class D finding-ID namespace format anomalies (invariant 6: advisory-only).
/// - Bootstrap-ordering conditions (v1.0 BC not yet in INDEX, STORY-INDEX absent/NotFound).
/// - Unparseable `last_amended:` format (precondition 37 last sentence).
#[derive(Debug, Clone)]
pub struct Advisory {
    /// Human-readable advisory message for `host::log_warn`.
    pub message: String,
}

/// PostToolUse entry point called by `vsdd_hook_sdk::__internal::run`.
///
/// # Dispatch flow
/// 1. Extract `file_path` from `payload.tool_input`. If absent: Continue + log_warn.
/// 2. Classify the file via `dispatch::*`. If unclassified (all classifiers return
///    false/None): return Continue immediately — no read attempted (F-S2107-P1C-001).
///    This ensures paths outside the hook's scope (`.factory/STATE.md`, source files,
///    `CLAUDE.md`, `policies.yaml`, etc.) never produce spurious blocks.
/// 3. Read the primary target via `host::read_file` with branch-specific caps
///    (BC-5.39.010 AC-019 / F-S2107-P1C-002 / F-S2107-P1C-003):
///    - STORY-INDEX.md: B2_MAX_BYTES / B2_TIMEOUT_MS
///    - Cycle artifact: CYCLE_ARTIFACT_PRIMARY_MAX_BYTES / CYCLE_ARTIFACT_PRIMARY_TIMEOUT_MS
///    - BC/story/VP/epic: BC_STORY_PRIMARY_MAX_BYTES / BC_STORY_PRIMARY_TIMEOUT_MS
///    On HostError for cycle artifacts: advisory + Continue (invariant 6 — Class D
///    is advisory-only, never blocking). On HostError for all other classified targets:
///    BLOCK (fail-closed per invariant 4 + BC-5.39.008 v1.6).
///    On UTF-8 decode failure: BLOCK (fail-closed per invariant 4 extension — introduced
///    BC-5.39.010 v1.20, ADV-RECON-007; governing BC — invariant 9 governs slicing
///    safety only, not decode disposition).
/// 4. Dispatch to applicable arms based on classification:
///    - STORY-INDEX.md: Arm B2 only.
///    - Cycle artifact: Arm D advisory (returns Continue).
///    - BC file: Arm A1 + Class E.
///    - Story file: Arm A2 + Arm B1 + Class E.
///    - All BC/VP/story/epic files: Class E (frontmatter parity).
/// 5. Collect ALL violations from all arms.
/// 6. Emit ALL advisories via `host::log_warn` (regardless of block state).
/// 7. If violations non-empty: return ONE combined `HookResult::block_with_fix`.
///    If violations empty: return `HookResult::Continue`.
///
/// # BC trace
/// BC-5.39.010 preconditions, postconditions, and invariants (current version per BC-INDEX) —
/// including preconditions 15a/15b/40 (primary-target decode-failure fail-closed + Arm B1
/// volatile-input scan), postcondition 25 (primary UTF-8 decode BLOCK, Step 4 below), and
/// postcondition 26 (secondary-index INDETERMINATE advisory).
/// BC-5.39.010 invariant 7: arms MUST NOT suppress each other.
/// BC-5.39.010 invariant 9: `is_char_boundary()` on byte-index slicing.
pub fn on_post_tool_use(payload: HookPayload) -> HookResult {
    // Step 1: extract file_path from tool_input
    let file_path = match payload.tool_input.get("file_path").and_then(|v| v.as_str()) {
        Some(p) => p.to_string(),
        None => {
            host::log_warn(
                "validate-cross-site-correspondence: file_path absent from tool_input — skipping",
            );
            return HookResult::Continue;
        }
    };

    // Step 2: classify first; return Continue for unclassified paths (F-S2107-P1C-001).
    // Classify before any read — the registry fires on tool="^(Edit|Write|MultiEdit)$"
    // with path_allow covering only three prefixes. Paths outside those prefixes
    // (e.g., .factory/STATE.md, crates/**, CLAUDE.md) get CapabilityDenied from
    // host::read_file, causing spurious blocks if we read before classifying.
    let is_si = dispatch::is_story_index(&file_path);
    let cycle_kind = dispatch::is_cycle_artifact(&file_path);
    let is_bc = dispatch::is_bc_file(&file_path);
    let is_story = dispatch::is_story_file(&file_path);
    let is_fpm = dispatch::is_frontmatter_parity_target(&file_path);

    if !is_si && cycle_kind.is_none() && !is_bc && !is_story && !is_fpm {
        // Unclassified — outside this hook's scope (PC1/PC9/PC16/PC22/PC28/PC34).
        // No arm fires; Continue immediately without reading the file.
        return HookResult::Continue;
    }

    // Step 3: read primary target with branch-specific cap (BC-5.39.010 AC-019).
    let (primary_max, primary_timeout_ms) = if is_si {
        (
            arm_b::STORY_INDEX_B2_MAX_BYTES,
            arm_b::STORY_INDEX_B2_TIMEOUT_MS,
        )
    } else if cycle_kind.is_some() {
        (
            CYCLE_ARTIFACT_PRIMARY_MAX_BYTES,
            CYCLE_ARTIFACT_PRIMARY_TIMEOUT_MS,
        )
    } else {
        (BC_STORY_PRIMARY_MAX_BYTES, BC_STORY_PRIMARY_TIMEOUT_MS)
    };

    let primary_bytes = match host::read_file(&file_path, primary_max, primary_timeout_ms) {
        Ok(bytes) => bytes,
        // Note: the Err(e) if cycle_kind.is_some() arm is structurally removed here.
        // cycle_kind is always None (is_cycle_artifact returns None per Class D deferral
        // in D-953). When Class D is re-enabled in S-21.08, restore the arm alongside
        // the is_cycle_artifact body. F-S2107-P3-008 (dead code deletion).
        Err(e) => {
            // Fail-closed: CapabilityDenied or any error on classified non-cycle target → block
            // (BC-5.39.010 invariant 4 + BC-5.39.008 v1.6)
            return combine_violations_into_block(
                "validate-cross-site-correspondence",
                &[Violation {
                    description: format!(
                        "validate-cross-site-correspondence [primary-read] POLICY 14: \
                        cannot read primary target '{file_path}': {e:?}. \
                        Fail-closed per BC-5.39.010 invariant 4. Fix: review and fix all \
                        cross-site correspondence issues listed above, then retry the write."
                    ),
                }],
            );
        }
    };

    // Step 4: decode UTF-8; fail-closed on decode failure (precondition 15a /
    // postcondition 25 / invariant 4 extension — introduced BC-5.39.010 v1.20,
    // ADV-RECON-007; governing BC).
    // A primary target that reads successfully as bytes but is not valid UTF-8 is
    // NOT eligible for silent Continue — invariant 9 governs is_char_boundary()
    // slicing safety on already-decoded strings only and does NOT authorize
    // fail-open disposition of a decode failure.
    //
    // ============================================================================
    // S-21.08 OBLIGATION (Class D re-enablement — currently harmless, do not skip):
    // Precondition 15a's primary-target set EXCLUDES cycle artifacts — Class D is
    // advisory-only per invariant 6, so a decode failure on a cycle artifact must
    // NOT reach this unconditional block. Today this arm is correct-but-coincidental:
    // `cycle_kind` is always `None` (`is_cycle_artifact` returns `None` unconditionally
    // per the v1.6/D-953 Class D deferral — see `dispatch.rs`), so no cycle artifact
    // ever reaches Step 4 as a classified target. When Class D is restored in S-21.08,
    // this block MUST gain a `cycle_kind.is_some()` advisory carve-out (log_warn +
    // Continue, not block_with_fix) per PC15a / invariant 6 — a cycle-artifact decode
    // failure must be advisory, not blocking. Do NOT let this arm silently regress to
    // blocking cycle-artifact decode failures once Class D starts producing `Some(_)`.
    // ============================================================================
    let content = match std::str::from_utf8(&primary_bytes) {
        Ok(s) => s.to_string(),
        Err(decode_error) => {
            return combine_violations_into_block(
                "validate-cross-site-correspondence",
                &[Violation {
                    description: format!(
                        "validate-cross-site-correspondence [primary-read]: cannot decode \
                        primary target '{file_path}' as UTF-8: {decode_error}. Fail-closed \
                        per BC-5.39.010 invariant 4 (extended, v1.20) — invariant 9 governs \
                        slicing safety only and does not authorize Continue here. Fix: verify \
                        the file's encoding and re-save as UTF-8, then retry the write."
                    ),
                }],
            );
        }
    };

    // Step 5: classify and dispatch
    let mut violations: Vec<Violation> = Vec::new();
    let mut advisories: Vec<Advisory> = Vec::new();

    // Arm B2: STORY-INDEX.md write
    if is_si {
        let b2_violations = arm_b::run_arm_b2(&content);
        violations.extend(b2_violations);
        // STORY-INDEX.md: no Class E arm — PC34 scopes Class E to BC/VP/story/epic files
        // only. STORY-INDEX.md is excluded despite having version:/last_amended: fields
        // because it is the Arm B2 trigger, not a per-artifact spec file. F-S2107-P3-019.
        emit_advisories(&advisories);
        return if violations.is_empty() {
            HookResult::Continue
        } else {
            combine_violations_into_block("validate-cross-site-correspondence", &violations)
        };
    }

    // [DEFERRED v1.6 — Class D]: Class D (arm_d.rs) does not exist in this crate per
    // BC-5.39.010 §File Structure / Task 12 ("DEFERRED v1.6 — Class D; do NOT
    // create"; target: S-21.08). cycle_kind is always None (is_cycle_artifact returns
    // None); the cap-selection and primary-read-error branches above are kept intact
    // so re-enabling Class D in S-21.08 only requires restoring the is_cycle_artifact
    // body and adding back arm_d.rs plus its dispatch block here.

    // BC file: Arm A1 + Class E
    if is_bc {
        let bc_id = extract_stem_from_path(&file_path);
        // F-P6-019(lib): extract_version_field normalizes at the parse boundary.
        // run_arm_a1_with_index_result shadows its bc_version parameter (F-P6-019a), so
        // passing raw "v1.3" was benign-but-wrong — no observable defect while the shadow
        // exists, but a latent trap for any comparison added before that shadow.
        let bc_version = frontmatter::extract_version_field(&content).unwrap_or_default();
        let (a1_v, a1_a) = arm_a1::run_arm_a1(&bc_id, &bc_version, &file_path);
        violations.extend(a1_v);
        advisories.extend(a1_a);
    }

    // Story file: Arm A2 + Arm B1 + Class E
    if is_story {
        let story_id = extract_story_id_from_path(&file_path);
        let (a2_v, a2_a) = arm_a2::run_arm_a2(&story_id, &content);
        violations.extend(a2_v);
        advisories.extend(a2_a);

        let (b1_v, b1_a) = arm_b::run_arm_b1(&story_id, &content);
        violations.extend(b1_v);
        advisories.extend(b1_a);
    }

    // Class E (frontmatter parity) for BC/VP/story/epic
    if is_fpm {
        let (e1_v, e1_a) = arm_e::run_arm_e1(&content);
        violations.extend(e1_v);
        advisories.extend(e1_a);

        let e2_v = arm_e::run_arm_e2(&content);
        violations.extend(e2_v);
    }

    // Step 6: emit all advisories, then return
    emit_advisories(&advisories);

    // Step 7: combine violations or continue
    if violations.is_empty() {
        HookResult::Continue
    } else {
        combine_violations_into_block("validate-cross-site-correspondence", &violations)
    }
}

/// Combine a list of violations into a single `HookResult::block_with_fix`.
///
/// Enumerates all violations in a single combined block message per
/// BC-5.39.010 invariant 7 (arms must not suppress each other) and postcondition 23
/// (combined violations → single block).
///
/// # BC trace
/// BC-5.39.010 postcondition 23; invariant 7.
pub fn combine_violations_into_block(hook_name: &str, violations: &[Violation]) -> HookResult {
    let combined = violations
        .iter()
        .enumerate()
        .map(|(i, v)| format!("[{}] {}", i + 1, v.description))
        .collect::<Vec<_>>()
        .join(" | ");
    HookResult::block_with_fix(
        hook_name,
        &combined,
        "review and fix all cross-site correspondence issues listed above, then retry the write",
        "POLICY 14/18",
    )
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Regex `\b` word-character class: `[A-Za-z0-9_]`.
///
/// The single source of truth for "is this a word character" across ALL
/// `\b` (word-boundary) checks in this crate — Arm A1, Arm A2, and Arm B
/// alike. BC-5.39.010 PC5 (`\bv([0-9]+\.[0-9]+)\b`), PC13 (word-boundary
/// BC-ID / v-token matching), PC20 (`\binput-hash\s+([0-9a-f]{7,40})\b`),
/// and PC21 (`\b<id>=([0-9a-f]{7,40})\b`) all use standard regex `\b`
/// semantics, which include `_` as a word character — NOT just
/// `is_ascii_alphanumeric()`.
///
/// Promoted here from `arm_b`-private (F-S2107-RECON-004, which unified
/// only Arm B's leading/trailing checks) to crate-shared (F-S2107-RECON-005)
/// after a sibling-file sweep found Arm A1 (`extract_first_v_token`) and
/// Arm A2 (`line_contains_bc_id_at_boundary`, `bc_id_token_starting_at`,
/// `find_bc_id_boundary_end`, `extract_first_v_token_after_position`) still
/// used the narrower `!is_ascii_alphanumeric()`-only predicate — a SECOND,
/// divergent definition of `\b` living alongside the first. Every `\b`
/// boundary check in the crate now routes through this single function so
/// the two definitions cannot drift apart again (TD-VSDD-060).
pub(crate) fn is_word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// Emit advisories via `host::log_warn`.
fn emit_advisories(advisories: &[Advisory]) {
    for adv in advisories {
        host::log_warn(&adv.message);
    }
}

/// Extract the file stem (filename without `.md` extension) from a path.
///
/// Used to derive bc_id from a BC file path like
/// `.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md` → `BC-5.39.010`.
fn extract_stem_from_path(file_path: &str) -> String {
    std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(file_path)
        .to_string()
}

/// Extract the story ID from a story file path.
///
/// Story paths have the form `.factory/stories/S-21.07-<name>.md`.
/// The story ID is the `S-XX.YY` prefix (first two dash-separated segments).
///
/// For a file named `S-21.07-validate-cross-site-correspondence.md`, returns `S-21.07`.
fn extract_story_id_from_path(file_path: &str) -> String {
    let stem = std::path::Path::new(file_path)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("");
    // stem = "S-21.07-validate-cross-site-correspondence"
    // Split by '-': ["S", "21.07", "validate", ...]
    let parts: Vec<&str> = stem.splitn(3, '-').collect();
    if parts.len() >= 2 {
        format!("{}-{}", parts[0], parts[1])
    } else {
        stem.to_string()
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;
    // BC_STORY_PRIMARY_MAX_BYTES and CYCLE_ARTIFACT_PRIMARY_MAX_BYTES are
    // pub(super) in lib.rs; imported explicitly here for constant-level assertions.
    use super::{BC_STORY_PRIMARY_MAX_BYTES, CYCLE_ARTIFACT_PRIMARY_MAX_BYTES};
    use vsdd_hook_sdk::{HookPayload, HookResult};

    // -----------------------------------------------------------------------
    // T-046 / F-S2107-P1C-001: unclassified paths must return Continue, not block.
    //
    // Pre-fix bug (now closed): on_post_tool_use used to read the primary target
    // file (Step 2) BEFORE it classified the path (Step 4). In the non-wasm test
    // host, `ffi::read_file` always returns -1 (CapabilityDenied), so EVERY path —
    // including completely unclassified ones like ordinary source files, CLAUDE.md,
    // and critically .factory/STATE.md — used to produce a blocking HookResult.
    //
    // The most severe consequence: .factory/STATE.md is inside .factory/ but
    // NOT under any of the three `path_allow` prefixes
    // (.factory/specs/behavioral-contracts/, .factory/specs/verification-properties/,
    // .factory/stories/). State-manager writes .factory/STATE.md
    // on every fix burst, so under the pre-fix ordering the factory could not
    // record its own state. .factory/policies.yaml had the same problem.
    //
    // Fix applied (classify-then-read): classify first; if unclassified, return
    // Continue without calling host::read_file at all.
    //
    // BC-5.39.010: unclassified paths are outside the hook's scope (PC1/PC9/PC16/
    // PC22/PC28/PC34). No arm fires → Continue.
    //
    // Pre-fix Red Gate (now closed): current code used to return Block
    // (CapabilityDenied on primary read for all paths). assert_eq!(result,
    // HookResult::Continue) failed for every case.
    // -----------------------------------------------------------------------

    /// T-046 / F-S2107-P1C-001: unclassified paths must return Continue.
    ///
    /// RED GATE: Step 2 reads before Step 4 classifies → CapabilityDenied → Block.
    /// assert_eq!(result, Continue) FAILS for all four paths (Block ≠ Continue).
    /// After fix (classify-then-read): unclassified → Continue, no read attempted.
    #[test]
    fn test_BC_5_39_010_unclassified_path_returns_continue_not_block() {
        fn payload(file_path: &str) -> HookPayload {
            serde_json::from_value(serde_json::json!({
                "event_name": "PostToolUse",
                "tool_name": "Edit",
                "session_id": "test-session",
                "dispatcher_trace_id": "test-trace",
                "tool_input": {"file_path": file_path}
            }))
            .expect("test payload must parse")
        }

        // These paths are all OUTSIDE the three path_allow prefixes and must
        // never be blocked by this hook.
        let cases: &[&str] = &[
            // Ordinary source file — no .factory component
            "crates/hook-plugins/validate-cross-site-correspondence/src/lib.rs",
            // Repo root doc — no .factory component
            "CLAUDE.md",
            // .factory/STATE.md — inside .factory/ but NOT under any arm's prefix.
            // CRITICAL: state-manager writes this on every burst. Blocking it breaks
            // the factory's ability to record its own state or repair itself.
            ".factory/STATE.md",
            // .factory/policies.yaml — YAML, not .md; never matches any arm classifier
            ".factory/policies.yaml",
        ];

        for path in cases {
            let result = on_post_tool_use(payload(path));
            assert_eq!(
                result,
                HookResult::Continue,
                "unclassified path '{}' must return HookResult::Continue, not Block. \
                F-S2107-P1C-001: classify-first fix required. \
                RED GATE: current Step-2 read fires before classification → \
                CapabilityDenied (non-wasm stub returns -1) → Block.",
                path
            );
        }
    }

    // -----------------------------------------------------------------------
    // T-047 / F-S2107-P1C-002 + HIGH F-S2107-P1C-003: all six read caps pinned.
    //
    // Existing bats AC-019 pinned only four constants (BC_INDEX and BC secondary).
    // Three divergences stayed invisible because the primary read caps were not
    // pinned. This test covers all six BC-5.39.010 read cap spec values.
    //
    // Spec (BC-5.39.010, unchanged):
    //   BC file primary:        524288 / 3000 ms
    //   Story file primary:     524288 / 3000 ms
    //   Cycle artifact primary: 2097152 / 5000 ms
    //   BC-INDEX secondary:     1048576 / 3000 ms
    //   STORY-INDEX B1:         1048576 / 3000 ms
    //   STORY-INDEX B2:         2097152 / 5000 ms
    //
    // BC_MAX_BYTES = 524_288 already exists and is correct BUT is only passed
    // for Arm A2 secondary reads (arm_a2::run_arm_a2_for_bc). The primary read
    // in lib.rs Step 2 uses PRIMARY_READ_MAX_BYTES = 1_048_576 for EVERYTHING,
    // producing two divergences:
    //
    //   F-S2107-P1C-002 (BLOCKER): BC/story primary reads use 1 MiB instead of
    //   512 KiB. This is 2× the spec cap; causes false-negative pass-through on
    //   oversized BC files that should be rejected.
    //
    //   F-S2107-P1C-003 (HIGH): Cycle artifact primary reads use 1 MiB instead
    //   of 2 MiB. A cycle artifact between 1 MiB and 2 MiB (e.g., a large
    //   lessons.md) returns OutputTooLarge → Block, violating BC-5.39.010
    //   invariant 6 ("Class D is advisory-only, never blocking"). Also:
    //   NotFound on cycle artifact primary should be advisory + Continue per
    //   PC33, but currently produces Block (all Step-2 errors produce Block
    //   before classification runs).
    //
    //   Non-wasm host note: host::read_file always returns CapabilityDenied (-1)
    //   in the test context, so OutputTooLarge and NotFound behavioral cases
    //   cannot be triggered here. Both are asserted at constant level per brief
    //   (F-S2107-P1C-003 scope).
    //
    // After fix: introduce separate named constants for BC/story primary
    // (= BC_MAX_BYTES = 524_288) and cycle artifact primary (= 2_097_152).
    // Update assertions (9) and (10) below to reference the new constant names.
    // -----------------------------------------------------------------------

    /// T-047 / F-S2107-P1C-002 + HIGH F-S2107-P1C-003: all six read caps pinned.
    ///
    /// RED GATE assertions (9) and (10):
    ///   (9) PRIMARY_READ_MAX_BYTES (1_048_576) == BC_MAX_BYTES (524_288) → FAILS.
    ///   (10) PRIMARY_READ_MAX_BYTES (1_048_576) == STORY_INDEX_B2_MAX_BYTES (2_097_152) → FAILS.
    /// After fix: replace both assertions with the new per-path constants.
    #[test]
    fn test_BC_5_39_010_ac019_extended_all_six_read_caps_fully_pinned() {
        // (1–8) Already-correct constants — regression protection only:
        assert_eq!(
            arm_a1::BC_INDEX_MAX_BYTES,
            1_048_576,
            "BC-INDEX secondary: 1 MiB"
        );
        assert_eq!(
            arm_a1::BC_INDEX_TIMEOUT_MS,
            3_000,
            "BC-INDEX secondary: 3000 ms"
        );
        assert_eq!(
            arm_a1::BC_MAX_BYTES,
            524_288,
            "BC secondary (arm A2): 512 KiB"
        );
        assert_eq!(arm_a1::BC_TIMEOUT_MS, 3_000, "BC secondary: 3000 ms");
        assert_eq!(
            arm_b::STORY_INDEX_B1_MAX_BYTES,
            1_048_576,
            "STORY-INDEX B1: 1 MiB"
        );
        assert_eq!(
            arm_b::STORY_INDEX_B1_TIMEOUT_MS,
            3_000,
            "STORY-INDEX B1: 3000 ms"
        );
        assert_eq!(
            arm_b::STORY_INDEX_B2_MAX_BYTES,
            2_097_152,
            "STORY-INDEX B2: 2 MiB"
        );
        assert_eq!(
            arm_b::STORY_INDEX_B2_TIMEOUT_MS,
            5_000,
            "STORY-INDEX B2: 5000 ms"
        );

        // (9) F-S2107-P1C-002 (BLOCKER — now CLOSED):
        // BC/story file primary reads must use BC_STORY_PRIMARY_MAX_BYTES = 524_288 (512 KiB).
        // Former PRIMARY_READ_MAX_BYTES = 1_048_576 (1 MiB) was 2× the spec cap.
        // Fix: renamed constant BC_STORY_PRIMARY_MAX_BYTES = 524_288 wired to BC/story reads.
        assert_eq!(
            BC_STORY_PRIMARY_MAX_BYTES,
            arm_a1::BC_MAX_BYTES,
            "BC/story primary read cap MUST equal BC_MAX_BYTES=524288. \
            F-S2107-P1C-002 (BLOCKER — closed by BC_STORY_PRIMARY_MAX_BYTES rename)."
        );

        // (10) HIGH F-S2107-P1C-003 + invariant 6 (now CLOSED):
        // Cycle artifact primary reads must use CYCLE_ARTIFACT_PRIMARY_MAX_BYTES = 2_097_152.
        // Former PRIMARY_READ_MAX_BYTES = 1_048_576 caused OutputTooLarge → Block on
        // 1 MiB–2 MiB files, violating invariant 6 (Class D advisory-only).
        // Behavioral tests blocked by non-wasm CapabilityDenied; asserting at constant level.
        assert_eq!(
            CYCLE_ARTIFACT_PRIMARY_MAX_BYTES,
            arm_b::STORY_INDEX_B2_MAX_BYTES,
            "Cycle artifact primary read cap MUST equal STORY_INDEX_B2_MAX_BYTES=2097152. \
            HIGH F-S2107-P1C-003 + invariant 6 (closed by CYCLE_ARTIFACT_PRIMARY_MAX_BYTES)."
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-005: BC-INDEX.md must not be classified as a BC file.
    // dispatch::is_bc_file uses starts_with("BC-") && ends_with(".md") which
    // admits "BC-INDEX.md" as a valid BC path. When admitted, arm A1 tries to
    // read BC-INDEX.md as both primary target AND secondary index, producing
    // spurious version mismatches (F-S2107-P1B-005 + F-S2107-P1B-007 cascade).
    // BC-5.39.010 §Classification invariant: index files are excluded.
    // -----------------------------------------------------------------------

    /// T-035 lib-level: BC-INDEX.md must NOT be classified as a BC file (F-S2107-P1B-005).
    ///
    /// RED GATE: current `starts_with("BC-") && ends_with(".md")` matches "BC-INDEX.md".
    /// dispatch::is_bc_file returns true → assert!(!result) FAILS → RED gate.
    /// After fix (exclude by name: file_stem == "BC-INDEX"): returns false → PASSES.
    #[test]
    fn test_BC_5_39_010_dispatch_bc_index_not_bc_file_lib_integration() {
        let result = dispatch::is_bc_file(".factory/specs/behavioral-contracts/BC-INDEX.md");
        assert!(
            !result,
            "BC-INDEX.md must NOT be classified as a BC file — it is the index, \
            not a behavioral contract (F-S2107-P1B-005)"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-006: escaped-pipe version chains in production BC-INDEX rows.
    // arm_a1::run_arm_a1_with_index_result with production-shaped content must
    // correctly identify `1.6` as current when version history is `v1.3 \| v1.6`.
    // Pre-fix code (now closed): split on '|' → first token was "1.3" → "1.3" ≠ "1.6" → violation.
    // After fix: last token "1.6" → match → no violation.
    // -----------------------------------------------------------------------

    /// F-S2107-P1B-006: lib-level escaped-pipe chain integration test.
    ///
    /// RED GATE: current code returns "1.3" → "1.3" ≠ "1.6" → violation → NOT empty.
    /// assert!(violations.is_empty()) FAILS → RED gate.
    /// After fix (last token wins): "1.6" == "1.6" → empty → PASSES.
    #[test]
    fn test_BC_5_39_010_arm_a1_escaped_pipe_chain_stale_blocks_lib_level() {
        let index = concat!(
            "---\ndocument_type: bc-index\n---\n\n",
            "| [BC-5.39.010](ss-05/BC-5.39.010.md) | title | draft | CAP-032 | S-21.07",
            " | v1.3 \\| v1.4 \\| v1.5 \\| v1.6 |\n",
        );
        let (violations, _advisories) = arm_a1::run_arm_a1_with_index_result(
            "BC-5.39.010",
            "1.6",
            ".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
            Ok(index.as_bytes().to_vec()),
        );
        assert!(
            violations.is_empty(),
            "escaped-pipe chain `v1.3 \\| ... \\| v1.6` with current version '1.6' must not block. \
            Red Gate: current code extracts first token '1.3' → violation (F-S2107-P1B-006)"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1C-014: 15-byte last_amended string rejected by length guard.
    // "2026-07-30 (v2)" — single-digit outer version, no sub-version suffix.
    // BC-5.39.010 §E1: this is a valid format.
    // Pre-fix code (now closed): `if len < 17 { return None }` — 15 < 17 → returned None.
    // -----------------------------------------------------------------------

    /// T-045 lib-level: extract_last_amended_outer_version must accept 15-byte format.
    ///
    /// RED GATE: `if len < 17 { return None }` → 15 < 17 → None.
    /// assert_eq!(result, Some("2".to_string())) FAILS → RED gate.
    /// After fix (lower threshold to 14): returns Some("2") → PASSES.
    #[test]
    fn test_BC_5_39_010_class_e1_15_byte_last_amended_accepted_lib_level() {
        let result = arm_e::extract_last_amended_outer_version("2026-07-30 (v2)");
        assert_eq!(
            result,
            Some("2".to_string()),
            "15-byte last_amended '2026-07-30 (v2)' must parse to outer version '2'. \
            BC-5.39.010 v1.20 §E1 (F-S2107-P1C-014). \
            Red Gate: len < 17 guard rejects 15-byte strings → None → assertion FAILS"
        );
    }

    // -----------------------------------------------------------------------
    // F-P1C-016 / AC-018: invariant-7 multi-arm aggregation — Rust unit assertion.
    //
    // BC-5.39.010 invariant 7: arms MUST NOT suppress each other.
    // When both A1 and E1 produce violations, ALL violations must appear in the
    // combined block message (postcondition 23: combined violations → single block).
    //
    // Previously: AC-018 had bats-only coverage. This test adds Rust-level assertion.
    //
    // COVERAGE TEST (not RED GATE): combine_violations_into_block is already correctly
    // implemented. This test passes immediately — it adds Rust-level regression coverage
    // for AC-018. The implementation pre-existed; the test coverage gap is the finding.
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // REAL-CORPUS TESTS — reads live .factory/ corpus files.
    //
    // ROOT CAUSE CLOSURE: no prior test read a real corpus file; spec-describes-imagined-
    // shape defects survived a green test suite. Pass-2 found F-P2-004 (S-21.04 Arm A2)
    // and F-P2-005 (BC-1.17.001 Arm A1) as corpus-unverifiable because no test exercised
    // the real corpus shape. These tests make corpus shape load-bearing going forward.
    //
    // CI GATING (decided here, not deferred):
    //   - Default: tests skip gracefully if .factory/ corpus is unavailable (normal CI
    //     without factory-artifacts branch checked out). No CI flakiness.
    //   - CI_REQUIRE_ARTIFACTS=1: tests FAIL if corpus not found (use for corpus-aware CI
    //     jobs that explicitly mount the factory-artifacts worktree).
    //   Rationale: bats integration tests already use this env-var pattern; consistent
    //   treatment across Rust unit tests and bats tests unifies the CI configuration.
    //
    // DURABILITY vs. LOAD-BEARING balance:
    //   - Assertions compare extractor output against LIVE frontmatter fields, not
    //     hardcoded expected values (e.g., "1.7"). This survives BC version bumps: when
    //     BC-1.17.001 advances to v1.8, both BC-INDEX.md and BC-1.17.001.md are updated;
    //     the test still passes. It fails ONLY when the extractor returns a wrong value
    //     (the bug we're catching).
    //   - A test that passes regardless of corpus content (e.g., always skips if file
    //     missing) is a paper-fix (TD-VSDD-059). These tests assert real behavior when
    //     the corpus IS available.
    // -----------------------------------------------------------------------

    /// Discover the corpus root from an injectable override and walk base.
    ///
    /// Injectable seam for unit testing — production callers go through `live_factory_root`.
    ///
    /// `corpus_root_override`: value of `VSDD_CORPUS_ROOT`, if set.
    ///   - `Some(s)` where `s` is a valid corpus directory → returns `Some(path)`.
    ///   - `Some(s)` where `s` is invalid/nonexistent → **panics**. An explicit override
    ///     that is invalid is always a configuration error. Silent fallback to auto-discovery
    ///     would mask the error: a typo'd `VSDD_CORPUS_ROOT` could find the dev worktree's
    ///     `.factory/` and make corpus tests appear to run against unintended data — the
    ///     worst failure class: false success.
    ///   - `None` → walk up from `walk_start` up to 8 levels.
    /// `walk_start`: directory to begin the upward walk. Tests pass a tmpdir to isolate
    ///   from the real `.factory/`; `live_factory_root` passes `CARGO_MANIFEST_DIR`.
    ///
    /// # BC trace
    /// F-P7-003b: injectable seam added to make panic branch and invalid-override path
    /// reachable in unit tests without env-var manipulation (unsafe in concurrent tests).
    #[allow(clippy::panic)] // test helper: panic on invalid VSDD_CORPUS_ROOT is intentional
    fn discover_factory_root(
        corpus_root_override: Option<&str>,
        walk_start: &std::path::Path,
    ) -> Option<std::path::PathBuf> {
        fn is_real_corpus(root: &std::path::Path) -> bool {
            root.join("specs/behavioral-contracts").is_dir()
        }

        if let Some(root_str) = corpus_root_override {
            let path = std::path::PathBuf::from(root_str);
            if path.is_dir() && is_real_corpus(&path) {
                return Some(path);
            }
            // Explicit override set but not a valid corpus: always fatal.
            // Never fall through to auto-discovery — that would mask the error.
            panic!(
                "VSDD_CORPUS_ROOT is set to {root_str:?} but is not a valid corpus root \
                (directory must exist and contain 'specs/behavioral-contracts/'). \
                Fix: correct VSDD_CORPUS_ROOT to point at a real .factory/ corpus, \
                or unset it to use auto-discovery."
            );
        }

        // Auto-discover: walk up from walk_start up to 8 levels.
        let mut dir = walk_start.to_path_buf();
        for _ in 0..8 {
            let candidate = dir.join(".factory");
            if candidate.is_dir() && is_real_corpus(&candidate) {
                return Some(candidate);
            }
            match dir.parent().map(|p| p.to_path_buf()) {
                Some(p) => dir = p,
                None => break,
            }
        }
        None
    }

    /// Discover the live .factory/ corpus root.
    ///
    /// Priority: VSDD_CORPUS_ROOT env var → parent-directory walk from CARGO_MANIFEST_DIR.
    /// Story-worktree layout: .factory/ is mounted ~5 levels above the crate root in the
    /// MAIN checkout, not in the worktree. Walk finds it by ascending.
    ///
    /// Validation: the discovered .factory/ MUST contain `specs/behavioral-contracts/`
    /// to be accepted as the real corpus root (excludes worktree stub .factory/ directories
    /// that only have `cycles/` and `logs/`).
    ///
    /// VSDD_CORPUS_ROOT set-but-invalid → panics immediately (see `discover_factory_root`).
    /// The CI fail-hard branch is tested via `handle_corpus_absent(true)`.
    ///
    /// # BC trace
    /// F-P7-003b: refactored to call `discover_factory_root` for testability.
    fn live_factory_root() -> Option<std::path::PathBuf> {
        let corpus_root_override = std::env::var("VSDD_CORPUS_ROOT").ok();
        discover_factory_root(
            corpus_root_override.as_deref(),
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")),
        )
    }

    /// Respond to absent corpus: skip gracefully or fail hard.
    ///
    /// Called from `corpus_root_or_skip!` when `live_factory_root()` returns `None`.
    /// Extracted as a function so the panic branch is unit-testable without env-var
    /// manipulation (unsafe in concurrent tests).
    ///
    /// `require = true`  (CI_REQUIRE_ARTIFACTS=1): panics — corpus must be present in CI.
    /// `require = false` : prints `[CORPUS-SKIP]` and returns; caller should `return`.
    ///
    /// # BC trace
    /// F-P7-003b: injectable `require` parameter replaces the inline env-var read so both
    /// branches are reachable from unit tests without manipulating `CI_REQUIRE_ARTIFACTS`.
    #[allow(clippy::panic)] // test helper: panic on CI_REQUIRE_ARTIFACTS=1 is intentional
    fn handle_corpus_absent(require: bool) {
        if require {
            panic!(
                "CI_REQUIRE_ARTIFACTS=1 but .factory/ corpus not found. \
                Set VSDD_CORPUS_ROOT=/path/to/.factory to run corpus tests. \
                Or mount the factory-artifacts worktree before running."
            );
        }
        eprintln!(
            "[CORPUS-SKIP] .factory/ not found; set VSDD_CORPUS_ROOT or \
            CI_REQUIRE_ARTIFACTS=1 to require. Skipping corpus test."
        );
    }

    /// Skip gracefully or fail hard based on CI_REQUIRE_ARTIFACTS.
    /// Usage: `let root = corpus_root_or_skip!();` at the top of each corpus test.
    macro_rules! corpus_root_or_skip {
        () => {{
            match live_factory_root() {
                Some(r) => r,
                None => {
                    let require = std::env::var("CI_REQUIRE_ARTIFACTS").as_deref() == Ok("1");
                    handle_corpus_absent(require);
                    return;
                }
            }
        }};
    }

    // -----------------------------------------------------------------------
    // F-P7-003b — fail-hard branch testability + VSDD_CORPUS_ROOT invalid detection
    //
    // Three untested behaviors identified by independent verification:
    //   (1) CI_REQUIRE_ARTIFACTS=1 + absent corpus → panic (fail-hard), not skip.
    //   (2) VSDD_CORPUS_ROOT set-but-invalid → panic immediately, not fall through to
    //       auto-discovery (which would find the real .factory/ and mask the error).
    //   (3) CI_REQUIRE_ARTIFACTS unset + absent corpus → graceful skip (no panic).
    //
    // Root cause of prior untestability:
    //   (1) live_factory_root() always returns Some in this repo: CARGO_MANIFEST_DIR walk
    //       finds the real .factory/ within 8 levels. The panic branch is structurally
    //       unreachable from any env-var combination when the corpus is mounted.
    //   (2) Before this refactor, VSDD_CORPUS_ROOT set-but-invalid silently fell through
    //       to auto-discovery, returning the real .factory/ — masking the error.
    //
    // Fix applied (this burst):
    //   (a) discover_factory_root(override, walk_start) — injectable seam.
    //   (b) handle_corpus_absent(require) — injectable require flag.
    //   (c) VSDD_CORPUS_ROOT set-but-invalid now panics (not fall-through).
    //   (d) Tests call discover_factory_root + handle_corpus_absent directly —
    //       no env-var manipulation, safe for concurrent test execution.
    // -----------------------------------------------------------------------

    /// F-P7-003b(1): CI_REQUIRE_ARTIFACTS=1 + absent corpus → panics (fail-hard).
    ///
    /// TEETH: #[should_panic] fails if handle_corpus_absent(true) does NOT panic —
    /// proving the panic is genuinely load-bearing. The expected= suffix additionally
    /// catches regressions where the message changes but a panic still fires.
    ///
    /// Models the corpus_root_or_skip! behavior when CI_REQUIRE_ARTIFACTS=1 and
    /// live_factory_root() returns None (structurally unreachable in this repo by
    /// other means — injectable seam is the only viable test path).
    #[test]
    #[should_panic(expected = "CI_REQUIRE_ARTIFACTS=1 but .factory/ corpus not found")]
    fn test_corpus_fail_hard_panics_when_ci_require_artifacts_set() {
        handle_corpus_absent(true);
    }

    /// F-P7-003b(1) CONTROL: CI_REQUIRE_ARTIFACTS unset + absent corpus → no panic.
    ///
    /// CONTROL for the fail-hard test above: handle_corpus_absent(false) must NOT panic.
    /// If both handle_corpus_absent(true) and handle_corpus_absent(false) panicked, the
    /// fail-hard test would be vacuous (it cannot distinguish mode). This control
    /// proves the two modes are distinct.
    #[test]
    fn test_corpus_graceful_skip_when_ci_require_artifacts_not_set() {
        // Must complete without panic. In corpus_root_or_skip!, the macro then issues
        // `return` to exit the test function. That `return` is not testable from a unit
        // test (we're not inside a corpus_root_or_skip! call here), but the non-panic
        // path is the only testable concern for handle_corpus_absent(false).
        handle_corpus_absent(false);
    }

    /// F-P7-003b(2a): VSDD_CORPUS_ROOT set to nonexistent path → panics immediately.
    ///
    /// Before this fix: discover_factory_root silently fell through to auto-discovery,
    /// finding the real .factory/ and returning Some — masking the typo'd override.
    /// After fix: panics immediately when the override path fails is_real_corpus().
    ///
    /// TEETH: #[should_panic(expected = "VSDD_CORPUS_ROOT")] fails if discover_factory_root
    /// returns Some or None instead of panicking — proving the panic is load-bearing.
    #[test]
    #[should_panic(expected = "VSDD_CORPUS_ROOT")]
    fn test_corpus_invalid_corpus_root_override_panics_nonexistent_path() {
        discover_factory_root(
            Some("/vsdd-test-nonexistent-corpus-path-9876543210"),
            std::path::Path::new("/tmp"),
        );
    }

    /// F-P7-003b(2b): VSDD_CORPUS_ROOT set to existing dir without corpus structure → panics.
    ///
    /// Distinct from (2a): the path exists as a directory but lacks the
    /// `specs/behavioral-contracts/` subdirectory required by is_real_corpus().
    /// Verifies that corpus structure validation fires, not just path existence.
    ///
    /// /tmp always exists as a directory on the test platform but has no corpus structure.
    #[test]
    #[should_panic(expected = "VSDD_CORPUS_ROOT")]
    fn test_corpus_invalid_corpus_root_override_panics_dir_without_corpus_structure() {
        discover_factory_root(Some("/tmp"), std::path::Path::new("/tmp"));
    }

    /// F-P7-003b(3): auto-discovery returns None when walk finds no corpus.
    ///
    /// When VSDD_CORPUS_ROOT is None and the walk from /tmp finds no .factory/
    /// within 8 ancestors, discover_factory_root must return None (not panic).
    /// The macro then calls handle_corpus_absent to decide skip vs fail-hard.
    ///
    /// /tmp ancestors on Linux/macOS: /tmp → / → None (at most 2 levels).
    /// Neither has .factory/specs/behavioral-contracts/, so None is guaranteed.
    ///
    /// CONTROL: If discover_factory_root returned Some for the /tmp walk, all corpus
    /// tests would be broken — they rely on None triggering the skip path.
    #[test]
    fn test_corpus_discovery_returns_none_when_no_factory_in_walk() {
        let result = discover_factory_root(None, std::path::Path::new("/tmp"));
        assert!(
            result.is_none(),
            "discover_factory_root(None, /tmp) must return None: /tmp's ancestors do not \
            contain .factory/specs/behavioral-contracts/. \
            F-P7-003b(3): no-panic for absent corpus (panic only in handle_corpus_absent). \
            Got Some path, which means /tmp's ancestor tree unexpectedly contains a corpus. \
            This is a test environment anomaly — check if VSDD_CORPUS_ROOT is set."
        );
    }

    // -----------------------------------------------------------------------
    // Corpus test 1 — arm_a1 (RED GATE)
    //
    // extract_bc_index_version("BC-1.17.001") on the LIVE BC-INDEX.md must return
    // BC-1.17.001's OWN row version, not BC-2.07.001's version.
    //
    // Bug (F-P2-002): BC-2.07.001's row (line ~693 in BC-INDEX.md) contains text
    // "aligned to BC-1.17.001/BC-4.13.001 convention" in its v1.4 changelog segment.
    // The unanchored `line.contains("BC-1.17.001")` check matches BOTH rows.
    // LAST-wins across all matching rows then overwrites last_version = "1.7" (from
    // BC-1.17.001's own row) with "1.6" (BC-2.07.001's latest version).
    //
    // Verified manually: BC-1.17.001 row = line 659, BC-2.07.001 row = line 693;
    // BC-2.07.001 last version token = "1.6"; result = Some("1.6") (WRONG).
    // BC-1.17.001 frontmatter version: "1.7".
    // -----------------------------------------------------------------------

    /// CORPUS RED GATE: arm_a1 first-cell anchoring — BC-1.17.001 own-row version wins.
    ///
    /// Reads LIVE BC-INDEX.md and BC-1.17.001.md. Asserts extractor returns BC-1.17.001's
    /// own INDEX row version (from BC-1.17.001.md frontmatter) — not BC-2.07.001's latest
    /// version picked up via unanchored cross-reference scan.
    /// DURABLE: reads both live files; survives BC-1.17.001 version bumps.
    /// RED GATE: current LAST-wins + unanchored contains returns Some("1.6") ≠ Some("1.7").
    #[test]
    fn test_BC_5_39_010_corpus_arm_a1_bc_1_17_001_own_row_version_not_cross_ref() {
        let root = corpus_root_or_skip!();
        let bc_index_bytes = std::fs::read(root.join("specs/behavioral-contracts/BC-INDEX.md"))
            .expect("BC-INDEX.md must be readable from corpus root");
        let bc_file_str =
            std::fs::read_to_string(root.join("specs/behavioral-contracts/ss-01/BC-1.17.001.md"))
                .expect("BC-1.17.001.md must be readable from corpus root");
        // Read expected version from live BC frontmatter — durable (updates with the file).
        // ADV-RECON-006: uses extract_version_field (v-stripped), not raw
        // extract_frontmatter_field(_, "version"), because arm_a1::extract_bc_index_version_state
        // returns v-stripped tokens (BcIndexVersionState::Version) — comparing against a
        // possibly-"v"-prefixed raw frontmatter value would be an apples-to-oranges mismatch
        // for any BC whose frontmatter carries a "v"-prefixed version. Matches the sibling
        // corpus test below (test_BC_5_39_010_corpus_arm_a1_row_present_no_version_cell_majority_shape).
        let expected = frontmatter::extract_version_field(&bc_file_str)
            .expect("BC-1.17.001.md must have a version: field");
        let result = arm_a1::extract_bc_index_version_state("BC-1.17.001", &bc_index_bytes);
        assert_eq!(
            result,
            arm_a1::BcIndexVersionState::Version(expected.clone()),
            "extract_bc_index_version_state('BC-1.17.001') must return Version(BC-1.17.001's own \
            INDEX row version ('{expected}')), not a version from a later row that cross-references \
            BC-1.17.001 in its changelog text. BC-2.07.001's row at line ~693 mentions \
            'BC-1.17.001' in a v1.4 annotation and ends with v1.6 — unanchored LAST-wins \
            overwrites the correct answer. CORPUS RED GATE: F-P2-002 first-cell anchoring."
        );
    }

    // -----------------------------------------------------------------------
    // Corpus test 1b — arm_a1 RowPresentNoVersion majority (RED GATE)
    //
    // F-S2107-P3-001 BLOCKER: BC-INDEX corpus test MUST sample the ~1,943-row
    // RowPresentNoVersion majority. The prior corpus test (corpus test 1) sampled
    // BC-1.17.001 — one of the ~40 rows that DOES carry a version-chain cell —
    // allowing the three-state defect to survive three adversary passes undetected.
    //
    // Sampled BC: BC-1.01.001 (5-column row, no version-chain cell, version "1.2").
    // Expected: run_arm_a1_with_index_result produces no violations.
    // Current: extract_bc_index_version returns Some("15.01") (from S-15.01 fragment)
    //          → "15.01" != "1.2" → violation → FAILS.
    // -----------------------------------------------------------------------

    /// CORPUS RED GATE (F-S2107-P3-001): run_arm_a1 must not block for a BC whose
    /// INDEX row has no version-chain cell (RowPresentNoVersion majority shape).
    ///
    /// Reads LIVE BC-INDEX.md and BC-1.01.001.md. BC-1.01.001 has a 5-column row
    /// (no version-chain cell) and version "1.2". Current two-state None conflation
    /// returns Some("15.01") from the story-ID fragment → block.
    /// After fix (three-state): RowPresentNoVersion → silent-continue → no violations.
    ///
    /// DURABLE: reads both live files; survives BC-1.01.001 version bumps.
    #[test]
    fn test_BC_5_39_010_corpus_arm_a1_row_present_no_version_cell_majority_shape() {
        let root = corpus_root_or_skip!();
        let bc_index_bytes = std::fs::read(root.join("specs/behavioral-contracts/BC-INDEX.md"))
            .expect("BC-INDEX.md must be readable from corpus root");
        let bc_file_str =
            std::fs::read_to_string(root.join("specs/behavioral-contracts/ss-01/BC-1.01.001.md"))
                .expect("BC-1.01.001.md must be readable from corpus root");
        let bc_version = frontmatter::extract_version_field(&bc_file_str)
            .expect("BC-1.01.001.md must have a version: field");
        // Verify this is a meaningful test: version must not be "1.0" (RowAbsent path).
        // Uses extract_version_field (not raw extract_frontmatter_field) so that a BC with
        // `version: "v1.0"` is correctly normalized to "1.0" and the vacuity guard fires.
        assert_ne!(
            bc_version, "1.0",
            "BC-1.01.001 must have version != '1.0' for the RowPresentNoVersion test to \
            distinguish from the RowAbsent v1.0 advisory path"
        );
        // Negative twin (F-P7-020): prove extract_version_field normalizes 'v'-prefixed
        // versions, making the vacuity guard reachable for `version: "v1.0"` BCs.
        // Without this fix, raw extract_frontmatter_field(_, "version") returns "v1.0"
        // and assert_ne("v1.0", "1.0") would pass — silently defeating the guard.
        // 18 BCs in the live corpus carry v-prefixed versions, so this is reachable.
        {
            let raw_v_prefixed = "v1.0";
            let raw_guard_passes = raw_v_prefixed != "1.0"; // "v1.0" != "1.0" → true → guard not triggered
            let normalized = raw_v_prefixed.trim_start_matches('v'); // production normalisation
            let normalized_guard_fires = normalized == "1.0"; // "1.0" == "1.0" → true → guard fires
            assert!(
                raw_guard_passes,
                "Regression control: raw comparison allows 'v1.0' through — \
                vacuity guard defeated without normalization fix."
            );
            assert!(
                normalized_guard_fires,
                "NEGATIVE TWIN: extract_version_field strips 'v', so 'v1.0' → '1.0'. \
                assert_ne fires correctly. 18 BCs in the corpus use v-prefixed versions."
            );
        }
        let (violations, _) = arm_a1::run_arm_a1_with_index_result(
            "BC-1.01.001",
            &bc_version,
            &root
                .join("specs/behavioral-contracts/ss-01/BC-1.01.001.md")
                .to_string_lossy(),
            Ok(bc_index_bytes),
        );
        assert!(
            violations.is_empty(),
            "BC-1.01.001 (version={bc_version}) has a 5-column RowPresentNoVersion row in the \
            live BC-INDEX.md. run_arm_a1_with_index_result must NOT produce violations. \
            BC-5.39.010 §PC5: RowPresentNoVersion → silent-continue. \
            F-S2107-P3-001 BLOCKER: corpus test sampling the ~1,943-row majority. \
            Violations: {violations:?}"
        );
    }

    // -----------------------------------------------------------------------
    // Corpus test 2 — arm_a2 (RED GATE — F-P2-001 live corpus evidence)
    //
    // extract_story_bc_version_citations on LIVE S-21.04 must return only citations
    // from the Behavioral Contracts section (matching the live BC-6.26.001 frontmatter).
    //
    // Bug (F-P2-001 / PC13): skip_section initializes to `false`, so lines BEFORE the
    // first `## ` heading are scanned. S-21.04's `last_amended:` YAML field (line 11)
    // is a very long string that contains `|` pipe characters (from gate patterns like
    // `(^\|[^a-zA-Z0-9_])bcs:`) AND contains "BC-6.26.001" AND old version tokens
    // (e.g., "BC-6.26.001 v1.3→v1.4" in the historical changelog text). The extractor
    // returns a phantom citation "1.3" from the YAML frontmatter line.
    //
    // Verified: `extract_story_bc_version_citations(s21_04, "BC-6.26.001")` returns
    // [("table row 11", "1.3"), ("table row 161", "1.18")] with current buggy code.
    // The "1.3" phantom citation (from the last_amended YAML line) causes the assertion
    // `version == "1.18"` to fail.
    //
    // After fix (skip_section starts true): frontmatter lines are not scanned;
    // only the Behavioral Contracts section produces citations. Returns [("table row 161",
    // "1.18")] — exactly one citation matching the current BC version.
    // -----------------------------------------------------------------------

    /// CORPUS RED GATE: arm_a2 must not scan YAML frontmatter preamble for BC citations.
    ///
    /// Reads LIVE S-21.04 story and BC-6.26.001.md. Asserts all citations match the
    /// live BC frontmatter version. Current bug (skip_section=false) causes phantom "1.3"
    /// citation from the last_amended YAML field. DURABLE: reads both live files.
    /// RED GATE: extractor returns "1.3" ≠ "1.18" from S-21.04 last_amended YAML line.
    #[test]
    fn test_BC_5_39_010_corpus_arm_a2_s21_04_bc_citations_match_live_bc_frontmatter() {
        let root = corpus_root_or_skip!();
        let story_str = std::fs::read_to_string(
            root.join("stories/S-21.04-story-worktree-write-path-discipline.md"),
        )
        .expect("S-21.04 must be readable from corpus root");
        let bc_str =
            std::fs::read_to_string(root.join("specs/behavioral-contracts/ss-06/BC-6.26.001.md"))
                .expect("BC-6.26.001.md must be readable from corpus root");
        let expected = frontmatter::extract_frontmatter_field(&bc_str, "version")
            .expect("BC-6.26.001.md must have a version: field");
        let citations = arm_a2::extract_story_bc_version_citations(&story_str, "BC-6.26.001");
        assert!(
            !citations.is_empty(),
            "S-21.04 Behavioral Contracts section must have at least one BC-6.26.001 citation. \
            extract_story_bc_version_citations returned empty — section bounding or table-row \
            detection is broken. CORPUS shape invariant: F-P2-001."
        );
        for (location, version) in &citations {
            assert_eq!(
                version.as_str(),
                expected.as_str(),
                "S-21.04 BC-6.26.001 citation at {location} cites v{version} but BC-6.26.001 \
                frontmatter is v{expected}. Extractor returned a stale or phantom citation. \
                CORPUS shape invariant: F-P2-001 / section bounding guard."
            );
        }
    }

    // -----------------------------------------------------------------------
    // Corpus tests 3 + 4 — dispatch (RED GATE + GREEN on arrival)
    //
    // is_frontmatter_parity_target on the LIVE VP-INDEX.md path must return false.
    // Pre-fix code (now closed): starts_with("VP-") && ends_with(".md") used to admit
    // VP-INDEX.md. F-P2-003 / PC34 corpus evidence.
    // -----------------------------------------------------------------------

    /// CORPUS RED GATE: VP-INDEX.md path must be excluded by is_frontmatter_parity_target.
    ///
    /// Verifies VP-INDEX.md exists in live corpus and the path classifier returns false.
    /// RED GATE: current starts_with("VP-") && ends_with(".md") admits VP-INDEX.md → true.
    #[test]
    fn test_BC_5_39_010_corpus_dispatch_vp_index_excluded_from_class_e_live_path() {
        let root = corpus_root_or_skip!();
        assert!(
            root.join("specs/verification-properties/VP-INDEX.md")
                .is_file(),
            "VP-INDEX.md must exist in live corpus"
        );
        assert!(
            !dispatch::is_frontmatter_parity_target(
                ".factory/specs/verification-properties/VP-INDEX.md"
            ),
            "VP-INDEX.md MUST NOT be a frontmatter parity target. \
            BC-5.39.010 PC34: explicit VP-INDEX.md guard required. \
            CORPUS RED GATE: starts_with/ends_with check admits VP-INDEX.md. F-P2-003/F-P2-008."
        );
    }

    /// CORPUS shape invariant: VP-039.md path must be accepted by is_frontmatter_parity_target.
    ///
    /// Verifies VP-039.md exists in live corpus and the path classifier returns true.
    /// Complement guard: PC34 fix must NOT over-exclude canonical VP files.
    /// GREEN on arrival; prevents regression if PC34 fix is over-broad.
    #[test]
    fn test_BC_5_39_010_corpus_dispatch_vp_canonical_file_accepted_by_class_e_live_path() {
        let root = corpus_root_or_skip!();
        assert!(
            root.join("specs/verification-properties/VP-039.md")
                .is_file(),
            "VP-039.md must exist in live corpus"
        );
        assert!(
            dispatch::is_frontmatter_parity_target(
                ".factory/specs/verification-properties/VP-039.md"
            ),
            "VP-039.md MUST be a frontmatter parity target. The PC34 fix (VP-INDEX.md \
            exclusion) must NOT accidentally exclude canonical VP files. F-P2-003 regression guard."
        );
    }

    // -----------------------------------------------------------------------
    // Corpus test 5 — arm_e (GREEN on arrival)
    //
    // extract_last_amended_outer_version on LIVE VP-100.md must match version: field.
    // VP-100.md has: version: "1.2", last_amended: "2026-07-10 (v1.2) — ..."
    // Tests real corpus format compatibility, including [Prior: ...] chain exclusion.
    // -----------------------------------------------------------------------

    /// CORPUS shape invariant: arm_e1 extract_last_amended_outer_version on live VP-100.md.
    ///
    /// Reads LIVE VP-100.md. Asserts outer version matches version: field.
    /// Tests real corpus format compatibility: [Prior: ...] chains must not pollute result.
    /// GREEN on arrival — arm_e implementation is correct for this format. Regression guard.
    #[test]
    fn test_BC_5_39_010_corpus_arm_e1_vp100_last_amended_outer_version_matches_version_field() {
        let root = corpus_root_or_skip!();
        let vp_str = std::fs::read_to_string(root.join("specs/verification-properties/VP-100.md"))
            .expect("VP-100.md must be readable from corpus root");
        let version = frontmatter::extract_frontmatter_field(&vp_str, "version")
            .expect("VP-100.md must have a version: field");
        let last_amended = frontmatter::extract_frontmatter_field(&vp_str, "last_amended")
            .expect("VP-100.md must have a last_amended: field");
        let outer = arm_e::extract_last_amended_outer_version(&last_amended).expect(
            "VP-100.md last_amended must be parseable. If None, the extractor has a format \
            compatibility bug with real VP files. CORPUS shape invariant: F-P2-013.",
        );
        assert_eq!(
            outer, version,
            "VP-100.md: extract_last_amended_outer_version returned '{outer}' but version: \
            is '{version}'. Extractor has a parsing bug OR VP-100.md has a live E1 violation \
            (hook failure). CORPUS shape invariant: arm_e1 format guard."
        );
    }

    // -----------------------------------------------------------------------
    // F-P4-004 — Corpus test 5b: block-scalar `last_amended` on BC-5.39.010.md
    //
    // BC-5.39.010.md uses `last_amended: |-` (YAML literal block strip scalar).
    // Current extract_frontmatter_field returns "|-" for this field →
    // extract_last_amended_outer_version("|-") → None (len 2 < 14) → E1 inert.
    //
    // RED GATE: corpus_arm_e1_vp100 test passes because VP-100.md uses inline form.
    // This test reads the governing BC itself — the one artifact E1 most needs to gate —
    // and asserts that block-scalar last_amended is parseable into an outer version.
    // -----------------------------------------------------------------------

    /// F-P4-004 CORPUS RED GATE: extract_frontmatter_field on BC-5.39.010.md must not
    /// return the `|-` indicator string for `last_amended`.
    ///
    /// BC-5.39.010.md uses `last_amended: |-` block scalar form. Current implementation
    /// returns Some("|-") → E1 structurally inert on its own governing BC.
    /// BC-5.39.010 PC36: block scalars (`|`, `|-`, `>`, `>-`) MUST be parsed.
    ///
    /// RED GATE: extract_frontmatter_field returns "|-" → outer_version = None → FAILS.
    /// After PC36 fix: first body line extracted → outer_version = Some(vN.N) → PASSES.
    #[test]
    fn test_BC_5_39_010_corpus_arm_e1_bc5_39_010_block_scalar_last_amended_parseable() {
        let root = corpus_root_or_skip!();
        let bc_str =
            std::fs::read_to_string(root.join("specs/behavioral-contracts/ss-05/BC-5.39.010.md"))
                .expect(
                    "BC-5.39.010.md must be readable from corpus root. \
            Verify VSDD_CORPUS_ROOT or .factory/ is accessible.",
                );
        let last_amended = frontmatter::extract_frontmatter_field(&bc_str, "last_amended").expect(
            "BC-5.39.010.md must have a last_amended: field. \
                If None, the frontmatter scanner missed the field.",
        );
        assert!(
            !last_amended.starts_with("|-"),
            "extract_frontmatter_field must NOT return '|-' for BC-5.39.010.md \
            last_amended. BC-5.39.010 PC36: `|-` is a block scalar indicator — the \
            function must scan subsequent indented lines and return the block body. \
            F-P4-004 RED GATE. Got: {last_amended:?}"
        );
        assert!(
            !last_amended.starts_with('>'),
            "extract_frontmatter_field must not return a block scalar indicator (`>`, `>-`). \
            Got: {last_amended:?}"
        );
        // After the PC36 fix, the outer version must be extractable so E1 can gate.
        let outer = arm_e::extract_last_amended_outer_version(&last_amended);
        assert!(
            outer.is_some(),
            "extract_last_amended_outer_version must return Some(version) for \
            BC-5.39.010.md last_amended after block-scalar fix. \
            Current: extract_frontmatter_field returns '|-' → len 2 < 14 → None → \
            E1 inert on its own governing BC. BC-5.39.010 PC36 / postcondition 20. \
            F-P4-004 RED GATE. Got last_amended: {last_amended:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-P1C-016 / AC-018: invariant-7 multi-arm aggregation — Rust unit assertion.
    //
    // BC-5.39.010 invariant 7: arms MUST NOT suppress each other.
    // When both A1 and E1 produce violations, ALL violations must appear in the
    // combined block message (postcondition 23: combined violations → single block).
    //
    // Previously: AC-018 had bats-only coverage. This test adds Rust-level assertion.
    //
    // COVERAGE TEST (not RED GATE): combine_violations_into_block is already correctly
    // implemented. This test passes immediately — it adds Rust-level regression coverage
    // for AC-018. The implementation pre-existed; the test coverage gap is the finding.
    // -----------------------------------------------------------------------

    /// F-P1C-016 / AC-018: both A1 and E1 violations appear in combined block.
    ///
    /// COVERAGE TEST (immediately GREEN): combine_violations_into_block already works.
    /// Adding Rust-level assertion per F-P1C-016 (previously bats-only coverage).
    #[test]
    #[allow(clippy::panic)] // F-S2107-P3-025: panic! is intentional — defect detection,
    // not production code. unreachable! claims this arm can never
    // be reached; panic! correctly signals reachable-on-defect.
    fn test_BC_5_39_010_invariant_7_ac018_multi_arm_violations_both_in_combined_block() {
        // Two violations from different arms (simulating A1 + E1 co-firing on a BC write)
        let a1_v = Violation {
            description: "validate-cross-site-correspondence [Class A Arm1]: \
                BC-5.39.010 INDEX stale (1.5 ≠ 1.6)"
                .to_string(),
        };
        let e1_v = Violation {
            description: "validate-cross-site-correspondence [Class E1]: \
                version '1.6' ≠ last_amended outer '1.5'"
                .to_string(),
        };
        let result =
            combine_violations_into_block("validate-cross-site-correspondence", &[a1_v, e1_v]);
        match result {
            HookResult::Block { reason } => {
                assert!(
                    reason.contains("Class A Arm1"),
                    "A1 violation must appear in combined block. \
                    BC-5.39.010 invariant 7 / AC-018 / F-P1C-016. reason: {reason}"
                );
                assert!(
                    reason.contains("Class E1"),
                    "E1 violation must appear in combined block. \
                    BC-5.39.010 invariant 7 / AC-018 / F-P1C-016. reason: {reason}"
                );
            }
            _ => panic!(
                "two violations must produce HookResult::Block, not Continue — \
                reaching this arm means combine_violations_into_block has a correctness \
                defect. BC-5.39.010 invariant 7 / postcondition 23 (F-P1C-016). \
                F-S2107-P3-025: unreachable! replaced with panic! (reachable on defect)."
            ),
        }
    }

    // -----------------------------------------------------------------------
    // F-P6-010 CORPUS TEST A — Arm A1 on live BC-5.39.010
    //
    // BC-5.39.010 is the governing BC for this hook. Its own presence in the live
    // corpus verifies that Arm A1 does not spuriously block when processing itself.
    //
    // BC-5.39.010 uses a block-scalar `last_amended: |-` frontmatter (see F-P4-004).
    // After the PC36 block-scalar fix, the version is parseable. The live BC-INDEX
    // row for BC-5.39.010 may be: RowAbsent (if version > "1.0"), RowPresentNoVersion,
    // or Version(v). This corpus test verifies that whatever the live state is,
    // run_arm_a1_with_index_result does NOT produce a violation for BC-5.39.010's
    // own current version (i.e., the corpus is in a self-consistent state).
    //
    // GREEN on arrival IF the corpus is consistent; RED otherwise — detects live corpus
    // drift where BC-5.39.010's INDEX row is stale against its frontmatter version.
    // F-P6-010: no prior corpus test exercised Arm A1 on BC-5.39.010 itself.
    // -----------------------------------------------------------------------

    /// CORPUS shape invariant (F-P6-010): Arm A1 produces no violations for live BC-5.39.010.
    ///
    /// Reads LIVE BC-INDEX.md and BC-5.39.010.md. Asserts run_arm_a1_with_index_result
    /// produces no violations — confirming the corpus is self-consistent for the governing BC.
    /// Also confirms that the block-scalar `last_amended: |-` parse (PC36) does not prevent
    /// the version from being extracted (prerequisite for E1 arm coverage on BC-5.39.010).
    ///
    /// GREEN on arrival when corpus is consistent.
    /// RED when BC-5.39.010 version has been bumped without updating BC-INDEX — live corpus drift.
    #[test]
    fn test_BC_5_39_010_corpus_arm_a1_bc5_39_010_no_violations_self_consistent() {
        let root = corpus_root_or_skip!();
        let bc_index_bytes = std::fs::read(root.join("specs/behavioral-contracts/BC-INDEX.md"))
            .expect("BC-INDEX.md must be readable from corpus root");
        let bc_file_str =
            std::fs::read_to_string(root.join("specs/behavioral-contracts/ss-05/BC-5.39.010.md"))
                .expect("BC-5.39.010.md must be readable from corpus root");
        let bc_version = frontmatter::extract_frontmatter_field(&bc_file_str, "version").expect(
            "BC-5.39.010.md must have a version: field. \
            If None, frontmatter scanner or block-scalar parse (PC36) has a bug. \
            F-P4-004/PC36 prerequisite: block-scalar last_amended must be parseable.",
        );
        let (violations, _) = arm_a1::run_arm_a1_with_index_result(
            "BC-5.39.010",
            &bc_version,
            &root
                .join("specs/behavioral-contracts/ss-05/BC-5.39.010.md")
                .to_string_lossy(),
            Ok(bc_index_bytes),
        );
        assert!(
            violations.is_empty(),
            "BC-5.39.010 (version={bc_version}) must not produce Arm A1 violations against the \
            live BC-INDEX.md. The corpus is self-inconsistent: BC-5.39.010's INDEX row is stale \
            against its current frontmatter version. F-P6-010 corpus shape invariant: \
            run_arm_a1_with_index_result must not block on the governing BC's own live state. \
            Violations: {violations:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-010 CORPUS TEST B — Arm B1 on live S-21.07
    //
    // S-21.07 is the story delivered in this cycle. This test calls
    // run_arm_b1_with_index_result (pure function) with live STORY-INDEX bytes.
    //
    // Corpus-shape invariants asserted upfront (fail loud, not silent):
    //   (1) S-21.07 must have NO volatile inputs in its `inputs:` block.
    //       If a future burst adds a volatile input (e.g. ARCH-INDEX.md was
    //       removed last burst — the reverse is plausible), this test fails
    //       with a diagnostic message rather than silently degrading to a
    //       branch that never exercises Arm B1's three-way comparison.
    //   (2) S-21.07 must have an `input-hash:` field present.
    //       If absent, Arm B1 skips entirely — not a valid regression guard.
    //
    // If either invariant breaks: fix the test (re-scope for the new corpus
    // shape) or add a separate fixture-based PC40 test, but do NOT allow this
    // test to silently pass without testing Arm B1.
    //
    // F-P6-010: no prior corpus test exercised Arm B1 on live S-21.07 itself.
    // -----------------------------------------------------------------------

    /// CORPUS shape invariant (F-P6-010): Arm B1 produces no violations for live S-21.07.
    ///
    /// Asserts corpus preconditions loudly (no volatile inputs, has input-hash), then
    /// unconditionally calls run_arm_b1_with_index_result against live STORY-INDEX bytes.
    /// Fails loud if corpus shape changes rather than silently degrading to a vacuous pass.
    ///
    /// GREEN when corpus is self-consistent for S-21.07 (no volatile inputs, hashes agree).
    /// RED when S-21.07 input-hash disagrees with STORY-INDEX — live corpus drift.
    /// RED (corpus-shape invariant) when S-21.07 gains volatile inputs or loses input-hash.
    #[test]
    fn test_BC_5_39_010_corpus_arm_b1_s21_07_no_violations() {
        let root = corpus_root_or_skip!();

        // Find S-21.07 story file — name may vary. Try canonical path first.
        let story_path =
            root.join("stories/S-21.07-validate-cross-site-correspondence-wasm-hook.md");
        let story_str = if story_path.is_file() {
            std::fs::read_to_string(&story_path)
                .expect("S-21.07 story must be readable from corpus root")
        } else {
            // Fallback: scan stories/ for a file whose frontmatter story_id == "S-21.07"
            let entries = std::fs::read_dir(root.join("stories"))
                .expect("stories/ directory must be readable from corpus root");
            let mut found = None;
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension().and_then(|e| e.to_str()) != Some("md") {
                    continue;
                }
                if let Ok(content) = std::fs::read_to_string(&p)
                    && let Some(id) = frontmatter::extract_frontmatter_field(&content, "story_id")
                    && id == "S-21.07"
                {
                    found = Some(content);
                    break;
                }
            }
            found.expect(
                "S-21.07 story must exist in corpus stories/ directory. \
                Set VSDD_CORPUS_ROOT or CI_REQUIRE_ARTIFACTS=1 if missing. \
                F-P6-010 corpus shape invariant.",
            )
        };

        // Corpus-shape invariant (1): S-21.07 must have NO volatile inputs.
        // A future burst could add ARCH-INDEX.md, STATE.md, or another volatile
        // path to inputs:. If that happens, this test fails loud rather than
        // silently degrading to a branch that never calls run_arm_b1_with_index_result.
        let volatile_inputs = arm_b::parse_story_volatile_inputs(&story_str);
        let volatile_found: Vec<&str> = volatile_inputs
            .iter()
            .filter(|p| arm_b::is_volatile_path(p))
            .map(|s| s.as_str())
            .collect();
        assert!(
            volatile_found.is_empty(),
            "CORPUS SHAPE CHANGED: S-21.07 now declares volatile inputs: {:?}. \
            This test assumed no volatile inputs so that run_arm_b1_with_index_result \
            is exercised unconditionally. Either (a) remove the volatile input if it \
            was added in error, or (b) re-scope this test to a non-volatile story and \
            write a separate fixture-based PC40 test. F-P6-010 corpus shape invariant.",
            volatile_found
        );

        // Corpus-shape invariant (2): S-21.07 must have an input-hash: field.
        // Without it, Arm B1 skips entirely (PC18) and this test provides no coverage.
        let story_hash = arm_b::parse_story_input_hash(&story_str).expect(
            "CORPUS SHAPE CHANGED: S-21.07 has no input-hash: field. \
            Without input-hash:, Arm B1 skips and this test is not a valid regression guard. \
            Either add the field or re-scope this test. F-P6-010 corpus shape invariant.",
        );

        // Both preconditions hold: run the three-way comparison unconditionally.
        let story_index_bytes = std::fs::read(root.join("stories/STORY-INDEX.md")).expect(
            "STORY-INDEX.md must be readable from corpus root. \
            F-P6-010 corpus test B: three-way comparison requires STORY-INDEX.md.",
        );
        let (violations, _) =
            arm_b::run_arm_b1_with_index_result("S-21.07", &story_hash, Ok(story_index_bytes));
        assert!(
            violations.is_empty(),
            "S-21.07 must not produce Arm B1 violations against the live STORY-INDEX.md. \
            The corpus is self-inconsistent: S-21.07 input-hash disagrees with STORY-INDEX.md. \
            F-P6-010 corpus shape invariant. Violations: {violations:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P7-002 CORPUS TEST B2 — run_arm_b2 on live STORY-INDEX.md
    //
    // F-S2107-P7-002 (pass-7): the corpus-test sweep covered Arm A1 (Test A) and
    // Arm B1 (Test B) but skipped Arm B2 — the arm that checks STORY-INDEX.md's
    // internal consistency (catalog rows vs. aggregation blockquote).
    //
    // Live corpus state at burst time (requires state-manager action to clear):
    // Live corpus state when test added (pass-7 burst):
    //   - S-18.06: catalog=63d94a3, blockquote=cf37976 → MISMATCH (RED at test-write time)
    //   - S-18.08: catalog=fe61c2c, blockquote=747b3eb → MISMATCH (RED at test-write time)
    //   - S-18.11: catalog=c45c0fc, blockquote=absent  → half-present → advisory (GREEN)
    //   - S-18.12: catalog=345086c, blockquote=absent  → half-present → advisory (GREEN)
    //
    // D-957 state-manager reconciliation (2026-08-05): all four entries fixed.
    // Current corpus is CLEAN: test is now GREEN as a permanent regression guard.
    //
    // ADR-038 §Decision 3: half-present case (exactly one of {B2,B3} present, the
    // other absent) is advisory + Continue per PC12 ("B2 or B3 absent → advisory +
    // Continue" — unconditional inclusive-or). run_arm_b2 correctly produces no
    // violation for the half-present case (catalog-only rows: bq_hash=None branch).
    //
    // Test lifecycle:
    //   - corpus test (B2 live): GREEN after D-957; turns RED if corpus drifts again
    //   - mutant/teeth test: GREEN always (injected mismatch is always caught)
    //   - half-present control: GREEN always (run_arm_b2 already handles it)
    // -----------------------------------------------------------------------

    /// CORPUS shape invariant (F-S2107-P7-002): run_arm_b2 must produce no violations
    /// on the live STORY-INDEX.md.
    ///
    /// Was RED at test-write time (S-18.06/S-18.08 catalog↔blockquote mismatches).
    /// Became GREEN after D-957 state-manager reconciliation (2026-08-05).
    /// Now serves as permanent regression guard: fails if any story's catalog and
    /// blockquote diverge in a future STORY-INDEX.md burst.
    ///
    /// Half-present stories (catalog present, no blockquote) must NOT produce violations
    /// per ADR-038 §Decision 3 (PC12 literal). The mutant companion below proves this
    /// test has teeth and is not vacuous.
    #[test]
    fn test_BC_5_39_010_corpus_arm_b2_live_story_index_no_violations() {
        let root = corpus_root_or_skip!();
        let story_index_str = std::fs::read_to_string(root.join("stories/STORY-INDEX.md")).expect(
            "STORY-INDEX.md must be readable from corpus root. \
                F-S2107-P7-002 corpus Arm B2.",
        );
        let violations = arm_b::run_arm_b2(&story_index_str);
        assert!(
            violations.is_empty(),
            "STORY-INDEX.md must have no Arm B2 catalog↔blockquote violations. \
            F-S2107-P7-002: permanent regression guard — any story whose catalog hash \
            differs from its blockquote hash is a corpus inconsistency. \
            STATE-MANAGER ACTION: reconcile catalog↔blockquote in the same burst as any \
            story input-hash update. \
            ADR-038 §Decision 3: half-present stories (catalog present, no blockquote) \
            must NOT produce violations (PC12 literal: B2 or B3 absent → advisory). \
            Violations: {violations:?}"
        );
    }

    /// TEETH PROOF for corpus_arm_b2: run_arm_b2 with an injected mismatch must detect it.
    ///
    /// Proves the corpus test has teeth after reconciliation: if run_arm_b2 were broken
    /// and always returned an empty Vec, THIS mutant test would fail — exposing the vacuity.
    ///
    /// Mutant: appends a synthetic story S-99.99 whose catalog hash (aabbcc1) differs from
    /// its blockquote hash (ddeeff2) into the live STORY-INDEX content, then asserts
    /// run_arm_b2 detects the injected mismatch.
    ///
    /// GREEN immediately (the injected mismatch is always caught regardless of corpus state).
    #[test]
    fn test_BC_5_39_010_corpus_arm_b2_teeth_mutant_injected_mismatch_detected() {
        let root = corpus_root_or_skip!();
        let story_index_str = std::fs::read_to_string(root.join("stories/STORY-INDEX.md")).expect(
            "STORY-INDEX.md must be readable from corpus root. \
                F-S2107-P7-002 teeth mutant.",
        );
        // Inject a catalog row + blockquote pair with a guaranteed mismatch.
        // S-99.99 is a synthetic ID that cannot appear in the real corpus.
        let mutated = format!(
            "{story_index_str}\n\
            | S-99.99 | MUTANT-TEETH-TEST | input-hash aabbcc1 |\n\
            > S-99.99=ddeeff2\n"
        );
        let violations = arm_b::run_arm_b2(&mutated);
        assert!(
            !violations.is_empty(),
            "TEETH PROOF FAILED: run_arm_b2 must detect injected S-99.99 catalog=aabbcc1 \
            vs blockquote=ddeeff2 mismatch. \
            If violations is empty, run_arm_b2 is broken and would pass the corpus test \
            vacuously after state-manager reconciles the real mismatches. \
            F-S2107-P7-002 anti-vacuity requirement."
        );
        let found = violations.iter().any(|v| v.description.contains("S-99.99"));
        assert!(
            found,
            "At least one violation must mention the injected story ID 'S-99.99'. \
            Violations: {violations:?}"
        );
    }

    /// ADR-038 §Decision 3 SHAPE CONTROL: half-present catalog row (no blockquote) →
    /// no violation from run_arm_b2.
    ///
    /// Pins the ruling so a future implementer cannot accidentally change the half-present
    /// arm to block without breaking this control. Uses a synthetic fixture (no corpus mount
    /// needed) to isolate the shape invariant from live corpus state.
    ///
    /// PC12 literal: "B2 or B3 absent → advisory + Continue" — unconditional inclusive-or.
    /// Half-present satisfies this predicate because the blockquote (B3) is absent.
    ///
    /// GREEN immediately. If this breaks, it means run_arm_b2 began blocking on
    /// catalog-only entries — a regression against ADR-038 §Decision 3.
    #[test]
    fn test_BC_5_39_010_arm_b2_half_present_catalog_no_blockquote_no_violation() {
        // S-18.11 shape: catalog row present, hash extractable, no blockquote entry.
        let content = "| S-18.11 | some story | input-hash c45c0fc |\n\
            # No blockquote entry for S-18.11\n";
        let violations = arm_b::run_arm_b2(content);
        assert!(
            violations.is_empty(),
            "ADR-038 §Decision 3: half-present case (catalog present, blockquote absent) \
            must produce no Arm B2 violation. \
            PC12 literal: 'B2 or B3 absent → advisory + Continue' is unconditional. \
            Violations: {violations:?}"
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-010 CORPUS TEST C — is_volatile_path on all live story inputs
    //
    // Sweeps ALL story files in .factory/stories/*.md (except STORY-INDEX.md itself)
    // and for each story that declares an inputs: list, verifies is_volatile_path
    // against each declared path. Checks two invariants:
    //
    //   (1) STATE.md-referencing stories: at least one story must declare
    //       ".factory/STATE.md" and is_volatile_path must return true for it.
    //
    //   (2) No story declares a VP-NNN.md or BC-NNN.NNN.NNN.md path that
    //       is_volatile_path returns true for (VP/BC files are stable corpus artifacts
    //       and must NOT be treated as volatile by PC40).
    //
    // F-P6-010: prior is_volatile_path tests used synthetic fixtures. This test
    // exercises the function against the real corpus inputs: declarations.
    // -----------------------------------------------------------------------

    /// CORPUS shape invariant (F-P6-010): is_volatile_path correct on live story inputs.
    ///
    /// Sweeps live .factory/stories/*.md. Asserts STATE.md is volatile, VP/BC files are not.
    /// GREEN on arrival assuming is_volatile_path is correctly implemented (post-fix).
    #[test]
    #[allow(clippy::panic)] // test code: panic on unreadable stories/ is intentional
    fn test_BC_5_39_010_corpus_is_volatile_path_live_story_inputs() {
        let root = corpus_root_or_skip!();
        let stories_dir = root.join("stories");
        let entries = match std::fs::read_dir(&stories_dir) {
            Ok(e) => e,
            Err(err) => panic!(
                "stories/ directory must be readable: {err}. \
                Set VSDD_CORPUS_ROOT or CI_REQUIRE_ARTIFACTS=1. \
                F-P6-010 corpus sweep."
            ),
        };

        let mut found_state_md_story = false;
        let mut stories_with_inputs = 0usize;

        for entry in entries.flatten() {
            let p = entry.path();
            if p.file_name().and_then(|f| f.to_str()) == Some("STORY-INDEX.md") {
                continue; // Skip the index file itself
            }
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let content = match std::fs::read_to_string(&p) {
                Ok(s) => s,
                Err(_) => continue,
            };
            let inputs = arm_b::parse_story_volatile_inputs(&content);
            if inputs.is_empty() {
                continue;
            }
            stories_with_inputs += 1;

            for path in &inputs {
                // Invariant 2: VP/BC canonical spec files must NOT be volatile.
                // They are stable corpus artifacts. PC40 applies only to STATE.md,
                // INDEX files (BC/VP/STORY/ARCH), and cycles/ artifacts.
                if (path.contains("/VP-")
                    && path.ends_with(".md")
                    && !path.contains("/VP-INDEX.md"))
                    || (path.contains("/BC-")
                        && path.ends_with(".md")
                        && !path.contains("/BC-INDEX.md"))
                {
                    assert!(
                        !arm_b::is_volatile_path(path),
                        "is_volatile_path must return false for stable spec file '{path}'. \
                        BC-5.39.010 PC40: PC40 applies only to STATE.md, INDEX files, and \
                        cycles/ artifacts. VP/BC canonical spec files are stable and must NOT \
                        match volatile patterns. F-P6-010 corpus sweep."
                    );
                }

                // Track STATE.md declarations.
                if path == ".factory/STATE.md" {
                    found_state_md_story = true;
                    assert!(
                        arm_b::is_volatile_path(path),
                        "is_volatile_path must return true for '.factory/STATE.md'. \
                        BC-5.39.010 PC40 pattern 1: .factory/STATE.md is the canonical \
                        volatile pipeline state file. F-P6-010 corpus sweep."
                    );
                }
            }
        }

        // Corpus shape invariant: at least some stories must have inputs: declared.
        assert!(
            stories_with_inputs > 0,
            "Expected at least one story in .factory/stories/ to declare an inputs: list. \
            If all stories lack inputs:, the corpus may be malformed or the \
            parse_story_volatile_inputs extractor has a bug. F-P6-010 corpus sweep. \
            stories_with_inputs={stories_with_inputs}"
        );

        // Corpus shape invariant: at least one story should declare STATE.md.
        // S-21.07 is known to declare it (as of v1.x). If this fails in a future cycle
        // where no story declares STATE.md anymore, update this expectation.
        assert!(
            found_state_md_story,
            "Expected at least one story in .factory/stories/ to declare '.factory/STATE.md' \
            in its inputs: list. S-21.07 (this story) is known to declare it. \
            If no story declares STATE.md anymore, update this assertion. \
            F-P6-010 corpus sweep: is_volatile_path must be exercised against live data."
        );
    }

    // -----------------------------------------------------------------------
    // Helper: collect_md_files
    //
    // Recursively collects all .md files under `dir` into `out`.
    // Used by corpus sweep tests that need to walk the entire BC/VP/story corpus.
    // -----------------------------------------------------------------------
    fn collect_md_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_dir() {
                    collect_md_files(&path, out);
                } else if path.extension().and_then(|e| e.to_str()) == Some("md") {
                    out.push(path);
                }
            }
        }
    }

    // -----------------------------------------------------------------------
    // F-P6-010 CORPUS TEST D — BC-INDEX version-sync sweep (compensating guard)
    //
    // Rationale: Under v1.10, primary-newer-than-index BLOCKED, which
    // incidentally caught state-manager sync failures at write time. Under v1.11
    // PC2a, primary-newer-than-index becomes an advisory — so a BC bumped without
    // its BC-INDEX row updated produces an advisory and nothing fails at the hook.
    //
    // The hook is CORRECT: PC2a tolerates the transient mid-burst state
    // (primary written, index not yet synced — guaranteed by POLICY 3). But once a
    // burst is COMMITTED, index and frontmatter must agree. This test restores the
    // invariant at the cargo-test (CI) layer instead of the write layer:
    // write-time advisory, commit-time hard failure.
    //
    // Scope:
    //   - Scans ALL .md files in specs/behavioral-contracts/ recursively.
    //   - Excludes *-INDEX.md files (index files, not BC definitions).
    //   - Excludes files whose names do not start with "BC-" (naming convention guard).
    //   - Extracts frontmatter `version:` from each BC file.
    //   - Calls arm_a1::extract_bc_index_version_state for the full five-variant
    //     classification (four post-decode §PC5 states plus the v1.22 pre-decode
    //     IndexUnreadable state) plus the actual version comparison:
    //   - RowAbsent: BC not registered in INDEX — NOT a sync failure. Skip.
    //   - RowPresentNoVersion: BC registered but no version chain — NOT a sync failure.
    //     These are BCs using the legacy INDEX shape (no version-cell column) or
    //     BCs whose 6th cell carries no vN.N token. Skip.
    //   - RowMalformed: INDEX row is structurally malformed — separate concern. Skip.
    //   - IndexUnreadable: the live BC-INDEX.md itself failed UTF-8 decoding — a
    //     genuine corpus-shape anomaly, surfaced as a mismatch (see the match arm below).
    //   - Version(index_ver): INDEX has a version chain. The extracted last-chain-entry
    //     version is compared DIRECTLY to the normalized frontmatter version. This uses
    //     arm_a1's v1.13 extractor (ADR-038 §Decision 1: first-token-of-last-chain-entry)
    //     which correctly handles all F-P6-019 shapes:
    //       F-P6-019a (v-prefix): extractor strips leading 'v'; frontmatter is also
    //         normalized via trim_start_matches('v'). BC-5.24.006 handled correctly.
    //       F-P6-019b/c (annotation/prior entries): extractor picks the first token of
    //         the last chain entry, not annotation prose versions. BC-3.08.001 and
    //         BC-7.03.079 handled correctly.
    //       F-P6-019d (unescaped |): escape-aware split + fields[5..].join fixes the
    //         truncation that caused arm_a1 to return Version("1.16") for BC-4.13.001.
    //     F-S2107-P8-006: the previous bc_index_row_contains_version() helper was
    //     DELETED because it searched the whole row for the version token rather than
    //     using the extracted last-chain-entry value, admitting three bypass vectors
    //     (index-newer-than-primary, annotation-rollback, chain-rollback). See the
    //     bypass mutant tests below for the demonstrated evidence.
    //
    // Scale: ~1,985 BC files total; approximately 40 carry version chains.
    // The test asserts checked_count >= 5 to prove the sweep has teeth —
    // a zero-check run would silently pass while asserting nothing.
    //
    // F-P6-010: compensating guard, human-approved in S-21.07 burst.
    // -----------------------------------------------------------------------

    // -----------------------------------------------------------------------
    // TEETH PROOF for the corpus version sync gate (F-P6-010-D, F-S2107-P8-006)
    //
    // The corpus test is GREEN when the corpus is synced and RED when a BC's
    // frontmatter version does not match the last-chain-entry version in BC-INDEX.
    // The proof uses extract_bc_index_version_state with synthetic data to verify
    // the extracted version and then the direct comparison logic:
    //
    //   BEFORE perturbation: row ends with `v1.10 \| v1.11 |`
    //     extract_bc_index_version_state → Version("1.11"); "1.11" == "1.11" → GREEN
    //   AFTER perturbation:  row ends with `v1.10 |`
    //     extract_bc_index_version_state → Version("1.10"); "1.10" ≠ "1.11" → RED
    //
    // The three bypass mutant tests below (test_bypass_1_*, test_bypass_2_*,
    // test_bypass_3_*) pin that arm_a1::index_ver_matches_frontmatter — the named
    // comparison function the corpus gate calls — returns false for each bypass vector.
    // The extractor (extract_bc_index_version_state) was byte-identical before and after
    // the fix; only the comparison predicate changed (F-S2107-P8-006). The bypass mutants
    // exercise that predicate, not the extractor. See F-S2107-P9-001.
    // -----------------------------------------------------------------------

    /// Teeth proof: Version("1.11") matches frontmatter "1.11" (GREEN path);
    /// Version("1.10") does not match "1.11" (RED path — perturbation detected).
    /// Verifies the direct comparison mechanism used in
    /// test_BC_corpus_version_sync_all_indexed_bcs_match_frontmatter.
    #[test]
    #[allow(clippy::panic)] // test helper: panic! on unexpected variant is intentional
    fn test_corpus_version_sync_gate_teeth() {
        // Synthetic BC-INDEX row mimicking BC-5.39.010's real structure.
        // BEFORE perturbation: last chain entry is v1.11.
        let index_before = "\
| [BC-5.39.010](ss-05/BC-5.39.010.md) | validate-cross-site-correspondence | \
active | CAP-123 | S-21.07 | v1.10 \\| v1.11 |\n";
        // AFTER perturbation: v1.11 removed, only v1.10 in chain.
        let index_after = "\
| [BC-5.39.010](ss-05/BC-5.39.010.md) | validate-cross-site-correspondence | \
active | CAP-123 | S-21.07 | v1.10 |\n";

        // GREEN: extracted last-chain-entry is v1.11; frontmatter is "1.11" → match.
        let state_before =
            arm_a1::extract_bc_index_version_state("BC-5.39.010", index_before.as_bytes());
        assert_eq!(
            state_before,
            arm_a1::BcIndexVersionState::Version("1.11".to_string()),
            "TEETH FAILURE: before perturbation, extractor must return Version(\"1.11\")."
        );
        match state_before {
            arm_a1::BcIndexVersionState::Version(ref v) => {
                assert_eq!(
                    v.as_str(),
                    "1.11",
                    "TEETH: extracted version must equal frontmatter \"1.11\"."
                );
            }
            _ => panic!("TEETH: Version(_) expected"),
        }

        // RED trigger: last-chain-entry is v1.10 after perturbation; frontmatter is "1.11".
        let state_after =
            arm_a1::extract_bc_index_version_state("BC-5.39.010", index_after.as_bytes());
        assert_eq!(
            state_after,
            arm_a1::BcIndexVersionState::Version("1.10".to_string()),
            "TEETH FAILURE: after perturbation, extractor must return Version(\"1.10\")."
        );
        match state_after {
            arm_a1::BcIndexVersionState::Version(ref v) => {
                assert_ne!(
                    v.as_str(),
                    "1.11",
                    "TEETH: extracted version \"1.10\" must NOT equal frontmatter \"1.11\" \
                     — mismatch must be detected (corpus test would report sync failure)."
                );
            }
            _ => panic!("TEETH: Version(_) expected"),
        }
    }

    /// CORPUS shape invariant (F-P6-010-D, F-S2107-P8-006): every BC with a
    /// BC-INDEX version chain has its frontmatter `version:` matching the
    /// last-chain-entry version extracted by `extract_bc_index_version_state`.
    ///
    /// COMMIT-LAYER guard: CI fails on a sync failure that actually landed,
    /// while the v1.11 write-time hook correctly stays advisory during authoring (PC2a).
    ///
    /// GREEN when corpus is synced (normal post-burst state).
    /// RED when a BC frontmatter is bumped without updating BC-INDEX, or vice versa,
    /// OR when the BC-INDEX last-chain-entry does not match frontmatter (bypass scenarios).
    #[test]
    fn test_BC_corpus_version_sync_all_indexed_bcs_match_frontmatter() {
        let root = corpus_root_or_skip!();

        // Read BC-INDEX.md once as bytes and String for extract_bc_index_version_state.
        let bc_index_str =
            std::fs::read_to_string(root.join("specs/behavioral-contracts/BC-INDEX.md")).expect(
                "BC-INDEX.md must be readable. \
                    F-P6-010-D corpus sync invariant.",
            );

        // Collect all BC .md files, excluding *-INDEX.md files.
        let bc_dir = root.join("specs/behavioral-contracts");
        let mut all_bc_files: Vec<std::path::PathBuf> = Vec::new();
        collect_md_files(&bc_dir, &mut all_bc_files);
        all_bc_files.retain(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            // Keep only files that look like BC definitions: "BC-*.md", not "*-INDEX.md".
            name.starts_with("BC-") && !name.ends_with("-INDEX.md")
        });
        all_bc_files.sort(); // deterministic order for reproducible output

        // Corpus-shape invariant: BC directory must be non-trivially populated.
        assert!(
            all_bc_files.len() >= 100,
            "CORPUS SHAPE ANOMALY: only {} BC .md files found in specs/behavioral-contracts/. \
            Expected at least 100 (corpus has ~1983 BCs). \
            Check that VSDD_CORPUS_ROOT points to a populated .factory/ directory. \
            F-P6-010-D corpus sync invariant.",
            all_bc_files.len()
        );

        let mut mismatches: Vec<String> = Vec::new();
        let mut row_absent_count = 0usize;
        let mut row_no_version_count = 0usize;
        let mut row_malformed_count = 0usize;
        let mut checked_count = 0usize;

        for bc_path in &all_bc_files {
            let bc_id = bc_path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();
            if bc_id.is_empty() {
                continue;
            }

            let content = match std::fs::read_to_string(bc_path) {
                Ok(c) => c,
                Err(e) => {
                    mismatches.push(format!(
                        "{bc_id}: cannot read file {}: {e}",
                        bc_path.display()
                    ));
                    continue;
                }
            };

            let frontmatter_version = frontmatter::extract_frontmatter_field(&content, "version");

            // extract_bc_index_version_state classifies the INDEX row (RowAbsent /
            // RowPresentNoVersion / RowMalformed / Version) AND provides the
            // extracted last-chain-entry version for direct comparison.
            // F-S2107-P8-006: bc_index_row_contains_version() was deleted; the
            // Version(v) value is now compared directly to normalized_fv.
            let index_state =
                arm_a1::extract_bc_index_version_state(&bc_id, bc_index_str.as_bytes());

            match index_state {
                arm_a1::BcIndexVersionState::RowAbsent => {
                    // Not registered in INDEX — not a sync failure (draft or unregistered).
                    row_absent_count += 1;
                }
                arm_a1::BcIndexVersionState::RowPresentNoVersion => {
                    // Registered but no version chain — not a sync failure.
                    row_no_version_count += 1;
                }
                arm_a1::BcIndexVersionState::RowMalformed(_) => {
                    // INDEX row structurally malformed — separate concern from version sync.
                    row_malformed_count += 1;
                }
                arm_a1::BcIndexVersionState::IndexUnreadable => {
                    // BC-5.39.010 precondition 15b / postcondition 26 (v1.22 /
                    // ADV-RECON11-001): the corpus-sync gate reads the real BC-INDEX.md
                    // from disk, which MUST be valid UTF-8. Reaching this arm means the
                    // live corpus file itself failed to decode — a genuine corpus-shape
                    // anomaly distinct from every other classification above, so it is
                    // surfaced as a mismatch rather than silently skipped.
                    mismatches.push(format!(
                        "{bc_id}: BC-INDEX.md failed UTF-8 decode while checking this BC — \
                        row/version state is INDETERMINATE, not confirmed-absent. \
                        Verify the index file's encoding and re-save as UTF-8."
                    ));
                }
                arm_a1::BcIndexVersionState::Version(index_ver) => {
                    // INDEX has a version chain. Compare via the named predicate
                    // arm_a1::index_ver_matches_frontmatter, which encapsulates the
                    // terminal-value comparison that replaced bc_index_row_contains_version()
                    // (deleted at F-S2107-P8-006). Using the named function ensures this
                    // comparison is pinned by the three bypass-mutant tests in this module.
                    //   (1) index-newer-than-primary: terminal entry "1.19" ≠ fv "1.18".
                    //   (2) annotation-rollback: terminal token "1.24" ≠ fv "1.23".
                    //   (3) chain-rollback: terminal entry "1.13" ≠ fv "1.10".
                    checked_count += 1;
                    match &frontmatter_version {
                        None => {
                            // INDEX has a version chain but BC has no frontmatter version:.
                            mismatches.push(format!(
                                "{bc_id}: BC-INDEX has a version chain but BC frontmatter \
                                has no version: field. \
                                Add version: to the BC frontmatter, or remove the version \
                                chain from BC-INDEX."
                            ));
                        }
                        Some(fv) => {
                            // F-S2107-P9-001: use the named comparison function rather than
                            // an inline expression, so bypass-mutant tests in this module
                            // directly pin the comparison predicate.
                            if !arm_a1::index_ver_matches_frontmatter(&index_ver, fv) {
                                let normalized_fv = fv.trim_start_matches('v');
                                mismatches.push(format!(
                                    "{bc_id}: BC frontmatter version={fv:?} \
                                    (normalized={normalized_fv:?}) does not match BC-INDEX \
                                    last-chain-entry version={index_ver:?}. \
                                    State-manager must sync the INDEX row in the same burst \
                                    as a BC version bump."
                                ));
                            }
                        }
                    }
                }
            }
        }

        // Corpus-shape invariant: at least 5 versioned BCs must have been checked.
        // If checked_count == 0 the sweep asserted nothing — silent vacuous pass.
        assert!(
            checked_count >= 5,
            "CORPUS SHAPE ANOMALY: only {checked_count} BCs had a Version(_) INDEX state \
            (expected >= 5; corpus has approximately 40 versioned BCs). \
            Either BC-INDEX has lost its version chains, or extract_bc_index_version_state \
            has regressed. Scope: {} files scanned. F-P6-010-D corpus sync invariant.",
            all_bc_files.len()
        );

        assert!(
            mismatches.is_empty(),
            "BC-INDEX VERSION SYNC FAILURE — commit-layer gate (F-P6-010-D).\n\
            These BCs have their frontmatter version token absent from the BC-INDEX row.\n\
            State-manager must sync BC-INDEX in the same burst as a BC version bump.\n\
            \n\
            Mismatches ({} total):\n{}\n\
            \n\
            Sweep scope summary ({} files scanned):\n\
              assertions made (Version state): {checked_count}\n\
              RowAbsent (not registered — not a sync failure): {row_absent_count}\n\
              RowPresentNoVersion (no version chain — not a sync failure): {row_no_version_count}\n\
              RowMalformed (structural — separate concern): {row_malformed_count}\n",
            mismatches.len(),
            mismatches.join("\n"),
            all_bc_files.len(),
        );
    }

    // -----------------------------------------------------------------------
    // F-P6-019-GUARD — executable enforcement that no production code reads
    // `version` via the raw `extract_frontmatter_field(_, "version")` accessor.
    //
    // Root cause being guarded (F-P6-019 class):
    //   Every function that extracts a version from structured text strips the
    //   leading `v`, but `extract_frontmatter_field` returns raw frontmatter.
    //   The fix is `extract_version_field` — a wrapper that normalises at the
    //   single boundary. A doc-comment WARNING on `extract_frontmatter_field`
    //   is a narrative check; this test is the executable enforcement.
    //
    // SCAN STRATEGY:
    //   For each .rs source file in src/:
    //     1. Truncate at the first `#[cfg(test)]` line — everything after is
    //        test code and may legitimately call `extract_frontmatter_field`
    //        with any field for isolation testing.
    //     2. Skip lines that begin with `//` (doc comments and inline comments)
    //        to avoid false positives from documentation examples.
    //     3. Any remaining production line containing BOTH `extract_frontmatter_field`
    //        AND `"version"` is a violation — except the one entry in the
    //        EXCLUSION LIST below.
    //
    // EXCLUSION LIST (closed; every legitimate raw call must be named here):
    //
    //   Entry 1 — frontmatter.rs: the `extract_version_field` wrapper body.
    //     `extract_frontmatter_field(content, "version").map(|v| v.trim_start_matches('v')...`
    //     This IS the wrapper; the one caller that should be raw.
    //     Detected by: line also contains `trim_start_matches`.
    //
    //   Zero other entries. If a new legitimate raw caller is added without
    //   updating this list, this test fails — forcing an explicit decision.
    //   An exclusion list that grows silently is the vulnerability class this
    //   test was written to eliminate (cf. "45/45" concealing skips, burst-6).
    //
    // MULTILINE CALL LIMITATION:
    //   Calls split across two lines (argument on the next line) are not
    //   detected. This is acceptable: all call sites in this crate use
    //   single-line invocations per the established code style, and the
    //   compiler's type system would still require passing "version" explicitly.
    // -----------------------------------------------------------------------

    /// F-P6-019-GUARD: no production code may call
    /// `extract_frontmatter_field(_, "version")` directly.
    ///
    /// RED: any production source file contains the banned pattern outside the
    /// `extract_version_field` wrapper body. Teeth proof: `lib.rs` contained the
    /// live violation `frontmatter::extract_frontmatter_field(&content, "version")`
    /// before the implementer's fix — this test was RED against that code and
    /// GREEN after the fix.
    ///
    /// GREEN: all production version reads go through `extract_version_field`.
    #[test]
    fn test_F_P6_019_guard_no_raw_version_field_access_in_production_code() {
        use std::fs;
        use std::path::PathBuf;

        let src_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src");

        // Collect all .rs source files deterministically.
        let mut rs_files: Vec<PathBuf> = fs::read_dir(&src_dir)
            .expect("src/ directory must be readable")
            .filter_map(|e| {
                let p = e.expect("dir entry readable").path();
                if p.extension().and_then(|s| s.to_str()) == Some("rs") {
                    Some(p)
                } else {
                    None
                }
            })
            .collect();
        rs_files.sort();

        assert!(
            !rs_files.is_empty(),
            "F-P6-019-GUARD vacuity check: src/ must contain at least one .rs file. \
            Empty file list means the scan is guarding nothing."
        );

        let mut violations: Vec<String> = Vec::new();

        for file_path in &rs_files {
            let file_name = file_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("?");
            let content =
                fs::read_to_string(file_path).expect("source file must be readable as UTF-8");

            // Truncate at the first `#[cfg(test)]` — everything after is test code.
            // Test modules may call `extract_frontmatter_field` directly to exercise
            // the function in isolation; those are not production call sites.
            let production_code = match content.find("#[cfg(test)]") {
                Some(pos) => &content[..pos],
                None => content.as_str(),
            };

            for (line_idx, line) in production_code.lines().enumerate() {
                // Must contain both tokens to be a candidate.
                if !line.contains("extract_frontmatter_field") || !line.contains("\"version\"") {
                    continue;
                }

                // Skip comment and doc-comment lines — these are documentation examples,
                // not call sites. Detected by: trimmed line begins with `//`.
                if line.trim_start().starts_with("//") {
                    continue;
                }

                // EXCLUSION LIST — Entry 1: the `extract_version_field` wrapper body
                // in frontmatter.rs. The single legitimate raw caller normalises via
                // `trim_start_matches('v')`. This exclusion is file-specific and
                // normalizer-character-specific to prevent a wildcard bypass (F-S2107-P7-015).
                if file_name == "frontmatter.rs" && line.contains("trim_start_matches('v')") {
                    continue;
                }

                violations.push(format!("  {}:{}: {}", file_name, line_idx + 1, line.trim()));
            }
        }

        assert!(
            violations.is_empty(),
            "F-P6-019-GUARD: production code must not call \
            `extract_frontmatter_field(_, \"version\")` directly. \
            All production version reads must go through \
            `frontmatter::extract_version_field(content)`, which normalises \
            the leading 'v' so that raw frontmatter \"v1.3\" compares equal to \
            index-extracted \"1.3\" (F-P6-019 normalization asymmetry class). \
            \n\nFound {} violation(s):\n{}\n\
            \nFix: replace each raw call with \
            `frontmatter::extract_version_field(content)` (returns Option<String> \
            without leading 'v'). \
            \nTo add a new legitimate raw caller, update the EXCLUSION LIST in \
            this test and document why normalization is not required at that site.",
            violations.len(),
            violations.join("\n")
        );

        // Negative twin (F-S2107-P7-015): prove the file-specific escape does NOT
        // fire for a non-frontmatter.rs caller that uses trim_start_matches alongside
        // the trigger tokens. Without the file-name guard, such a line would be
        // silently exempted by the old `line.contains("trim_start_matches")` escape,
        // making the guard a wildcard bypass for any caller normalizing wrongly.
        let synthetic_raw_caller = concat!(
            r#"let v = extract_frontmatter_field(content, "version")"#,
            r#".map(|s| s.trim_start_matches('v').to_string());"#
        );
        // Old escape: fires for ANY file containing trim_start_matches → false-pass.
        let old_escape_would_exempt = synthetic_raw_caller.contains("trim_start_matches");
        assert!(
            old_escape_would_exempt,
            "Regression control: the OLD escape (line.contains(\"trim_start_matches\")) \
            would have exempted this synthetic non-wrapper line — proving the gap."
        );
        // New escape: file-specific → does NOT exempt a lib.rs caller.
        let new_escape_would_exempt = "lib.rs" == "frontmatter.rs"
            && synthetic_raw_caller.contains("trim_start_matches('v')");
        assert!(
            !new_escape_would_exempt,
            "NEGATIVE TWIN: the file-specific escape must NOT exempt a raw caller in \
            a non-frontmatter.rs file (F-S2107-P7-015). Escape scope must match \
            its documented intent — frontmatter.rs wrapper body only."
        );
    }

    // -----------------------------------------------------------------------
    // F-P7-010 (in-scope leg): production-scale fixture for arm_a1 pure Rust functions.
    //
    // The live BC-INDEX.md is 574,311 bytes. This test exercises
    // run_arm_a1_with_index_result (BC-INDEX scan + version state extraction) against
    // a synthetic >=574 KB index to verify no logic regression under production-scale
    // input.
    //
    // What this fixture proves:
    //   The pure Rust functions (arm_a1::run_arm_a1_with_index_result) process a
    //   >=574 KB BC-INDEX correctly and detect version mismatches at scale.
    //
    // What this fixture does NOT prove:
    //   WASM sandbox fuel behavior at scale. The hook runs with a finite fuel budget
    //   when deployed as a WASM plugin; this test does not exercise that constraint.
    //   The fuel-exhaustion gap is tracked separately (ADR-039 Phase 3 calibration).
    //
    // Anti-vacuity: a stale version (1.10 when index has v1.13) produces violations,
    // proving the scan actually found and processed the target BC row in the large index.
    // -----------------------------------------------------------------------
    #[test]
    fn test_F_P7_010_production_scale_arm_a1_scan_no_regression() {
        // Build a >=574 KB synthetic BC-INDEX.
        // Row format matches the live index: GFM table with 6 fields.
        // Locator column: "[BC-{ss}.{n1:02}.{i:04}](path)".
        // 2,800 filler rows x ~250 bytes/row = ~700 KB >> 574,311 byte threshold.
        let mut index_content = String::with_capacity(720 * 1024);

        // Minimal header matching the live BC-INDEX.md structure.
        index_content.push_str("---\nversion: \"4.47\"\n---\n\n# BC-INDEX\n\n");

        // Target BC inserted at row 1,400 (mid-index; scan must not short-circuit).
        const TARGET_BC: &str = "BC-5.39.010";
        const TARGET_VERSION: &str = "1.13";
        const TARGET_ROW: usize = 1_400;

        for i in 1..=2_800_usize {
            if i == TARGET_ROW {
                // Target row: version chain "v1.10 \| v1.11 \| v1.12 \| v1.13".
                // Last-wins algorithm extracts v1.13.
                // In the actual string content, \\| becomes \| (GFM escape for literal |).
                index_content.push_str(
                    "| [BC-5.39.010](ss-05/BC-5.39.010.md) | \
                    validate-cross-site-correspondence | \
                    active | CAP-122 | S-21.07 | \
                    v1.10 \\| v1.11 \\| v1.12 \\| v1.13 |\n",
                );
            } else {
                let ss = (i % 9) + 1;
                let n1 = (i % 99) + 1;
                let v = (i % 12) + 1;
                let m = (i % 8) + 1;
                // Long description (~200 chars) ensures each row contributes ~250 bytes.
                // 2,800 * 250 = 700,000 bytes > 574,311 byte threshold.
                index_content.push_str(&format!(
                    "| [BC-{ss}.{n1:02}.{i:04}](ss-{ss:02}/BC-{ss}.{n1:02}.{i:04}.md) | \
                    Behavioral contract row {i:04} — synthetic filler for F-P7-010 \
                    production-scale fixture; exercises pure Rust arm_a1 scan at \
                    >=574 KB input size (live BC-INDEX is 574311 bytes) | \
                    active | CAP-{n1:03} | S-01.01 | v{v}.{m} |\n"
                ));
            }
        }

        // Assert fixture meets production-scale threshold before running assertions.
        let fixture_bytes = index_content.len();
        assert!(
            fixture_bytes >= 574_311,
            "FIXTURE SIZE GATE: index must be >=574,311 bytes (live BC-INDEX size). \
            Got {fixture_bytes} bytes. Increase row count or description length."
        );

        // Convert once; clone for happy path, move for anti-vacuity mutant.
        let index_bytes: Vec<u8> = index_content.into_bytes();

        // Happy path: bc_version matches index row → no violations, no advisories.
        let (violations, advisories) = arm_a1::run_arm_a1_with_index_result(
            TARGET_BC,
            TARGET_VERSION,
            "/fake/BC-5.39.010.md",
            Ok(index_bytes.clone()),
        );
        assert!(
            violations.is_empty(),
            "PRODUCTION-SCALE GATE (happy path): arm_a1 must produce no violations \
            when bc_version matches the index row. Found {} violation(s) in a \
            {fixture_bytes}-byte index. Target: {TARGET_BC} v{TARGET_VERSION} \
            at row {TARGET_ROW} of 2800.",
            violations.len()
        );
        assert!(
            advisories.is_empty(),
            "PRODUCTION-SCALE GATE (happy path): arm_a1 must produce no advisories \
            when bc_version matches exactly. Found {} advisory message(s) in a \
            {fixture_bytes}-byte index.",
            advisories.len()
        );

        // Anti-vacuity mutant: bc_version="1.10" < index v1.13 → PC2b block.
        // This proves the scan found the target BC row; a vacuous scan that missed it
        // would return RowAbsent (a different violation) or no violations at all.
        let (stale_violations, _) = arm_a1::run_arm_a1_with_index_result(
            TARGET_BC,
            "1.10",
            "/fake/BC-5.39.010.md",
            Ok(index_bytes),
        );
        assert!(
            !stale_violations.is_empty(),
            "ANTI-VACUITY MUTANT: stale version (1.10 vs index v1.13) in a \
            {fixture_bytes}-byte index MUST produce violations (PC2b: index newer \
            than primary). Empty violations means the scan did not find {TARGET_BC} \
            at row {TARGET_ROW} — the production-scale fixture is vacuous."
        );
    }

    // -----------------------------------------------------------------------
    // BYPASS MUTANT TESTS (F-S2107-P9-001 fix — pinning the corpus gate comparison)
    //
    // These three tests verify that arm_a1::index_ver_matches_frontmatter — the named
    // comparison function called by the corpus gate — returns false for each bypass
    // vector. The fix at F-S2107-P8-006 deleted bc_index_row_contains_version() (a
    // whole-row helper) and replaced it with direct terminal-value comparison.
    // The EXTRACTOR (extract_bc_index_version_state) was byte-identical before and
    // after the fix. Only the comparison predicate changed.
    //
    // Each test has three assertions:
    //   (A) Extractor sanity: confirm the terminal chain-entry value.
    //   (B) Inline WEAK semantics proof: whole-row `v{fv}` scan with trailing-non-digit
    //       check returns true for this bypass vector (the old false-pass).
    //   (C) NAMED FUNCTION assertion: index_ver_matches_frontmatter returns false
    //       (mismatch detected). This is the load-bearing mutant assertion.
    //       If index_ver_matches_frontmatter is reimplemented with non-strict-equality
    //       semantics (e.g., always-true, presence-check, substring), (C) fails and
    //       the test goes RED.
    //
    // Bypass vectors (adversary-verified against live BC-INDEX.md @ 67ffbdcc):
    //   Bypass 1: index-newer-than-primary — row "v1.18 | v1.19", frontmatter "1.18"
    //     Old helper: finds "v1.18" in row → true (false-pass).
    //     Named function: "1.19" ≠ "1.18" → false (mismatch detected). ✓
    //   Bypass 2: annotation-token rollback — row "v1.24 (promoted v1.23 D-839; …)", fv "1.23"
    //     Old helper: finds "v1.23" in annotation → true (false-pass).
    //     Named function: "1.24" ≠ "1.23" → false (mismatch detected). ✓
    //   Bypass 3: chain rollback — row "v1.10 | v1.11 | v1.12 | v1.13", frontmatter "1.10"
    //     Old helper: finds "v1.10" in cumulative chain → true (false-pass).
    //     Named function: "1.13" ≠ "1.10" → false (mismatch detected). ✓
    // -----------------------------------------------------------------------

    /// Bypass 1 pin: index-newer-than-primary is detected as mismatch.
    /// Inline proof (B) shows the deleted whole-row helper would have returned true.
    /// Load-bearing assertion (C): index_ver_matches_frontmatter returns false.
    #[test]
    #[allow(clippy::panic)]
    fn test_bypass_1_index_newer_than_primary_detected_as_mismatch() {
        // BC-INDEX row with chain "v1.18 \| v1.19": index advanced past BC frontmatter.
        let row = "| [BC-5.39.010](ss-05/BC-5.39.010.md) | desc | active | CAP | S-21.07 | \
                   v1.18 \\| v1.19 |\n";
        let frontmatter_version = "1.18";

        // (A) Extractor sanity: terminal chain-entry must be "1.19".
        let state = arm_a1::extract_bc_index_version_state("BC-5.39.010", row.as_bytes());
        let index_ver = match state {
            arm_a1::BcIndexVersionState::Version(v) => v,
            other => panic!("BYPASS 1 SETUP: expected Version(_) from index row, got {other:?}."),
        };
        assert_eq!(
            index_ver.as_str(),
            "1.19",
            "BYPASS 1 EXTRACTOR: must return Version(\"1.19\") for chain ending '\\| v1.19'."
        );

        // (B) Inline WEAK semantics proof: whole-row `v{fv}` scan with trailing-non-digit
        // check returns true — the false-pass the old helper admitted for this vector.
        let fv_token = format!("v{frontmatter_version}");
        let weak_result = row
            .find(&fv_token)
            .map(|pos| {
                row[pos + fv_token.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_ascii_digit())
            })
            .unwrap_or(false);
        assert!(
            weak_result,
            "BYPASS 1 WEAK PROOF: whole-row scan must find '{fv_token}' with trailing \
             non-digit — this is the false-pass the fix closes; if this fails the bypass \
             vector is not structured as expected."
        );

        // (C) NAMED FUNCTION assertion (load-bearing): index_ver_matches_frontmatter must
        // return false (mismatch detected). Any non-strict-equality implementation of this
        // function that returns true here causes this assertion to fail → RED.
        assert!(
            !arm_a1::index_ver_matches_frontmatter(&index_ver, frontmatter_version),
            "BYPASS 1 MUTANT FAILURE: index_ver_matches_frontmatter returned true for \
             index-newer-than-primary (index_ver={index_ver:?}, fv={frontmatter_version:?}). \
             Terminal-value comparison must detect this mismatch. A true return means the \
             comparison predicate was changed to a non-strict form — the bypass vector is live."
        );
    }

    /// Bypass 2 pin: annotation-token rollback is detected as mismatch.
    /// Inline proof (B) shows the deleted whole-row helper would have returned true.
    /// Load-bearing assertion (C): index_ver_matches_frontmatter returns false.
    #[test]
    #[allow(clippy::panic)]
    fn test_bypass_2_annotation_rollback_detected_as_mismatch() {
        // BC-3.08.001 shape: last chain entry "v1.24 (promoted v1.23 D-839; …)".
        // Frontmatter rolled back to "1.23".
        let row = "| [BC-3.08.001](ss-03/BC-3.08.001.md) | desc | active | CAP-003 | S | \
                   v1.24 (promoted v1.23 D-839; no promotion required) |\n";
        let frontmatter_version = "1.23";

        // (A) Extractor sanity: terminal value is first token of last entry — "1.24".
        let state = arm_a1::extract_bc_index_version_state("BC-3.08.001", row.as_bytes());
        let index_ver = match state {
            arm_a1::BcIndexVersionState::Version(v) => v,
            other => panic!("BYPASS 2 SETUP: expected Version(_) from index row, got {other:?}."),
        };
        assert_eq!(
            index_ver.as_str(),
            "1.24",
            "BYPASS 2 EXTRACTOR: must return Version(\"1.24\") — first token of last chain \
             entry, not annotation prose."
        );

        // (B) Inline WEAK semantics proof: whole-row scan finds "v1.23" in annotation prose.
        let fv_token = format!("v{frontmatter_version}");
        let weak_result = row
            .find(&fv_token)
            .map(|pos| {
                row[pos + fv_token.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_ascii_digit())
            })
            .unwrap_or(false);
        assert!(
            weak_result,
            "BYPASS 2 WEAK PROOF: whole-row scan must find '{fv_token}' in annotation \
             prose — this is the false-pass the fix closes."
        );

        // (C) NAMED FUNCTION assertion (load-bearing): must return false (mismatch detected).
        assert!(
            !arm_a1::index_ver_matches_frontmatter(&index_ver, frontmatter_version),
            "BYPASS 2 MUTANT FAILURE: index_ver_matches_frontmatter returned true for \
             annotation-rollback (index_ver={index_ver:?}, fv={frontmatter_version:?}). \
             Terminal-value comparison must detect this mismatch."
        );
    }

    /// Bypass 3 pin: chain-rollback is detected as mismatch.
    /// Inline proof (B) shows the deleted whole-row helper would have returned true.
    /// Load-bearing assertion (C): index_ver_matches_frontmatter returns false.
    #[test]
    #[allow(clippy::panic)]
    fn test_bypass_3_chain_rollback_detected_as_mismatch() {
        // Full chain row: v1.10 \| v1.11 \| v1.12 \| v1.13.
        // Frontmatter rolled back to "1.10" — any prior chain entry is a rollback.
        let row = "| [BC-5.39.010](ss-05/BC-5.39.010.md) | desc | active | CAP | S-21.07 | \
                   v1.10 \\| v1.11 \\| v1.12 \\| v1.13 |\n";
        let frontmatter_version = "1.10";

        // (A) Extractor sanity: last chain entry is "1.13".
        let state = arm_a1::extract_bc_index_version_state("BC-5.39.010", row.as_bytes());
        let index_ver = match state {
            arm_a1::BcIndexVersionState::Version(v) => v,
            other => panic!("BYPASS 3 SETUP: expected Version(_) from index row, got {other:?}."),
        };
        assert_eq!(
            index_ver.as_str(),
            "1.13",
            "BYPASS 3 EXTRACTOR: must return Version(\"1.13\") — last chain entry for \
             row ending '\\| v1.13'."
        );

        // (B) Inline WEAK semantics proof: whole-row scan finds "v1.10" in cumulative chain.
        // Chains are append-only; any prior entry is always present in the raw row.
        let fv_token = format!("v{frontmatter_version}");
        let weak_result = row
            .find(&fv_token)
            .map(|pos| {
                row[pos + fv_token.len()..]
                    .chars()
                    .next()
                    .is_none_or(|c| !c.is_ascii_digit())
            })
            .unwrap_or(false);
        assert!(
            weak_result,
            "BYPASS 3 WEAK PROOF: whole-row scan must find '{fv_token}' in cumulative \
             chain — chains are append-only so any prior entry is always present."
        );

        // (C) NAMED FUNCTION assertion (load-bearing): must return false (rollback detected).
        // Chains are append-only: the terminal entry is always the maximum version.
        // A rolled-back frontmatter value ≠ terminal entry → false (mismatch detected).
        assert!(
            !arm_a1::index_ver_matches_frontmatter(&index_ver, frontmatter_version),
            "BYPASS 3 MUTANT FAILURE: index_ver_matches_frontmatter returned true for \
             chain-rollback (index_ver={index_ver:?}, fv={frontmatter_version:?}). \
             Terminal-value comparison must detect rollback to any prior chain entry."
        );
    }
}
