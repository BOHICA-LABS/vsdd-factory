//! dispatch.rs — File-path classification for validate-cross-site-correspondence.
//!
//! Pure-core module (ADR-035 §Decision 1): path-matching only; no I/O.
//!
//! All guards use `Path::new(file_path).components()` — raw `ends_with` or
//! `contains` on the path string is FORBIDDEN per BC-5.39.010 invariant 3
//! (path-component-strict matching, same requirement as BC-5.39.008 precedent).
//!
//! # BC trace
//! BC-5.39.010 preconditions 1, 9, 16, 22, 28, 34 — file-path trigger conditions.
//! BC-5.39.010 invariant 3 — path-component-strict matching.
//! ADR-035 §Decision 1 — Tier 2A read-only PostToolUse validator.

/// Which category of cycle artifact a triggered write maps to.
///
/// **[DEFERRED v1.6 — Class D]**: `is_cycle_artifact` always returns `None` per
/// BC-5.39.010 v1.6 / D-953. Retained-but-unconstructed pending Class D
/// re-enablement in S-21.08: the `#[ignore]`d tests that once constructed these
/// variants directly were removed (ADV-RECON-005 — they asserted the opposite of
/// current normative behavior, dead and misleading test content; see the removal
/// note above `test_BC_5_39_010_dispatch_non_cycle_index_md_rejected` below), so
/// no test or production code constructs `CycleArtifactKind` anywhere in this crate
/// today.
///
/// Used by Arm D to determine the scope-limited extraction strategy
/// (BC-5.39.010 precondition 30).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CycleArtifactKind {
    /// `burst-log.md`: scope = last H2 section (from last `^## ` heading through EOF).
    BurstLog,
    /// `lessons.md`: scope = last `^L-EDP1-[0-9]+-[0-9]+:` anchor block; fallback last 200 lines.
    Lessons,
    /// `INDEX.md` (under `.factory/cycles/`): scope = `## Adversarial Reviews` section only.
    CycleIndex,
}

/// Returns `true` if `filename` matches the canonical BC filename shape:
/// `^BC-[0-9]+\.[0-9]+\.[0-9]+\.md$` (three dot-separated numeric groups).
///
/// Excludes BC-INDEX.md and any other non-contract files that merely start with "BC-".
///
/// # BC trace
/// BC-5.39.010 PC1 — bc_id shape guard; F-S2107-P1B-005 (BC-INDEX.md exclusion).
fn is_canonical_bc_filename(filename: &str) -> bool {
    // Must start with "BC-" and end with ".md"
    let inner = match filename
        .strip_prefix("BC-")
        .and_then(|s| s.strip_suffix(".md"))
    {
        Some(s) => s,
        None => return false,
    };
    // inner must be N.N.NNN format: exactly three dot-separated non-empty digit groups
    let parts: Vec<&str> = inner.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

/// Returns `true` if `components` contains `chain` — a sequence of expected
/// `Component::Normal` path-segment names — as a CONTIGUOUS, correctly-ordered
/// window that is followed by exactly `gap` further `Component::Normal`
/// directory segments and then the final component (the file name).
///
/// The window's start position is DERIVED from the total component count
/// (`components.len() - (chain.len() + gap + 1)`), never searched — so the
/// chain can occupy only the one position immediately (modulo `gap` plain
/// directory segments) before the file name. Components before that position
/// are otherwise unconstrained (any `Normal` name is allowed), so real
/// absolute/worktree paths (where `.factory` is not the first component)
/// still classify correctly — EXCEPT for `Component::ParentDir` (`..`),
/// which this function rejects unconditionally wherever it appears (prefix,
/// chain, gap, or filename position); see the definitive-close guard below.
///
/// # Why this exists (SEC-001 / CWE-697)
/// The prior implementation tested for each required component with an
/// independent `.any()` presence check. That admits a decoy path where the
/// required names are present but non-contiguous, out of order, or separated
/// by `..` traversal segments — e.g. `x/.factory/y/specs/z/behavioral-contracts/BC-1.2.3.md`
/// or `.factory/specs/behavioral-contracts/../../../../tmp/evil/BC-1.2.3.md` —
/// as a governed path. This function anchors the chain to a fixed position so
/// only the real governed directory sequence, immediately preceding the file,
/// classifies as a match.
///
/// # Definitive close (CWE-697 residual)
/// Anchoring alone left one bounded over-acceptance residual: a leading `..`
/// BEFORE the anchored chain — e.g.
/// `../.factory/specs/behavioral-contracts/ss-05/BC-1.2.3.md` — sits entirely
/// in the unconstrained prefix and still classified true. Since no legitimate
/// governed corpus path ever contains `..`, this function now rejects the
/// whole path outright if ANY component is `Component::ParentDir`, closing
/// the residual at the classification layer rather than leaving prefix
/// components unconstrained to `..`.
///
/// This is a classification-correctness fix only: `..` sequences cannot lexically
/// cancel a `Normal` component via `Path::components()`, and the actual
/// traversal defense (`factory-dispatcher/src/host/path_util.rs::check_path_allowed`,
/// Kani-proven VP-097) canonicalizes + allow-lists downstream and fails closed
/// on any real escape. A wrong classification here can at most cause a spurious
/// read attempt that path_util then blocks — never a bypass.
fn chain_immediately_precedes_filename(
    components: &[std::path::Component<'_>],
    chain: &[&str],
    gap: usize,
) -> bool {
    use std::path::Component;
    // SEC-001 / CWE-697 DEFINITIVE CLOSE: reject the whole path if ANY component
    // is a `ParentDir` (`..`), wherever it appears — including in the prefix
    // BEFORE `chain_start`, which is otherwise intentionally unconstrained so
    // real absolute/worktree paths (where `.factory` is not component 0) still
    // classify correctly. No legitimate governed corpus path ever contains
    // `..` — AI agents write real `.factory/...` paths, and the actual
    // traversal defense (`factory-dispatcher/src/host/path_util.rs::check_path_allowed`,
    // Kani-proven VP-097) canonicalizes + allow-lists downstream and fails
    // closed on any real escape regardless of this classifier's answer. This
    // guard only removes the last bounded spurious-classification residual
    // (a leading `..` before the anchored chain, e.g.
    // `../.factory/specs/behavioral-contracts/ss-05/BC-1.2.3.md`) — it does
    // not change containment. `CurDir` (`.`) / `RootDir` (`/`) / `Prefix`
    // (Windows drive letters) are deliberately NOT rejected: those are
    // legitimate for `./`-relative and absolute/worktree paths.
    if components.iter().any(|c| matches!(c, Component::ParentDir)) {
        return false;
    }
    let needed = chain.len() + gap + 1;
    if components.len() < needed {
        return false;
    }
    let chain_start = components.len() - needed;
    let chain_matches = components[chain_start..chain_start + chain.len()]
        .iter()
        .zip(chain.iter())
        .all(|(c, name)| matches!(c, Component::Normal(s) if *s == *name));
    if !chain_matches {
        return false;
    }
    let gap_start = chain_start + chain.len();
    components[gap_start..gap_start + gap]
        .iter()
        .all(|c| matches!(c, Component::Normal(_)))
}

/// Returns `true` if `file_path` names a BC file under
/// `.factory/specs/behavioral-contracts/` using path-component-strict,
/// contiguity-anchored matching.
///
/// A BC file has:
/// - The `.factory` / `specs` / `behavioral-contracts` chain as a CONTIGUOUS,
///   correctly-ordered window (SEC-001 / CWE-697 — not independent presence
///   checks), followed by exactly one further directory component (the
///   `ss-NN` subdirectory), AND
/// - A filename matching `BC-*.md`
///
/// # BC trace
/// BC-5.39.010 precondition 1 (Class A Arm1 trigger condition).
/// BC-5.39.010 invariant 3 (path-component-strict — NOT `ends_with` / `contains`).
pub fn is_bc_file(file_path: &str) -> bool {
    use std::path::Path;
    let path = Path::new(file_path);
    let components: Vec<_> = path.components().collect();
    // BC-5.39.010 PC1: filename must match ^BC-[0-9]+\.[0-9]+\.[0-9]+\.md$ — three
    // dot-separated numeric groups. This excludes BC-INDEX.md (F-S2107-P1B-005)
    // and any other non-contract files that merely start with "BC-".
    let filename_ok = path
        .file_name()
        .and_then(|f| f.to_str())
        .map(is_canonical_bc_filename)
        .unwrap_or(false);
    filename_ok
        && chain_immediately_precedes_filename(
            &components,
            &[".factory", "specs", "behavioral-contracts"],
            1,
        )
}

/// Returns `true` if `filename` is a canonical story basename: `^S-[0-9]+\.[0-9]+`.
///
/// Admits `S-21.07-name.md`, `S-1.1.md`, etc. Rejects `S-README.md` (no digit before
/// the dot) and `S-ARCH.md` (no dot at all).
///
/// # BC trace
/// BC-5.39.010 PC9/PC16: story basename pattern `^S-[0-9]+\.[0-9]+`.
/// F-P2-011: prior `starts_with("S-")` admitted S-README.md, S-ARCH.md, etc.
fn is_canonical_story_basename(filename: &str) -> bool {
    // Strip "S-" prefix
    let rest = match filename.strip_prefix("S-") {
        Some(r) => r,
        None => return false,
    };
    // Find the first dot; there must be at least one digit before it
    let dot_pos = match rest.find('.') {
        Some(p) if p > 0 => p,
        _ => return false,
    };
    // All characters before the dot must be ASCII digits
    if !rest[..dot_pos].chars().all(|c| c.is_ascii_digit()) {
        return false;
    }
    // The character immediately after the dot must be an ASCII digit
    matches!(rest[dot_pos + 1..].chars().next(), Some(c) if c.is_ascii_digit())
}

/// Returns `true` if `file_path` names a story file under `.factory/stories/`
/// using path-component-strict, contiguity-anchored matching.
///
/// A story file has:
/// - The `.factory` / `stories` chain as a CONTIGUOUS, correctly-ordered window
///   immediately preceding the filename (SEC-001 / CWE-697 — not independent
///   presence checks) AND
/// - Filename ending in `.md` (but NOT `STORY-INDEX.md` — that is Arm B2's trigger)
/// - Basename matching `^S-[0-9]+\.[0-9]+` (canonical story ID, excluding S-README etc.)
///
/// # BC trace
/// BC-5.39.010 precondition 9 (Class A Arm2 trigger condition).
/// BC-5.39.010 precondition 16 (Class B Arm1 trigger condition).
/// BC-5.39.010 invariant 3.
/// F-P2-011: canonical story basename requires numeric section.subsection.
pub fn is_story_file(file_path: &str) -> bool {
    use std::path::Path;
    let path = Path::new(file_path);
    let components: Vec<_> = path.components().collect();
    let filename = path.file_name().and_then(|f| f.to_str()).unwrap_or("");
    let is_md = filename.ends_with(".md");
    // STORY-INDEX.md is Arm B2's trigger, not a story file
    let not_index = filename != "STORY-INDEX.md";
    // PC9/PC16+F-P2-011: canonical story basename (^S-[0-9]+\.[0-9]+) excludes
    // epics (E-XX-*), index files, and non-story S-README/S-ARCH etc.
    let is_story_basename = is_canonical_story_basename(filename);
    is_md
        && not_index
        && is_story_basename
        && chain_immediately_precedes_filename(&components, &[".factory", "stories"], 0)
}

/// Returns `true` if `file_path` names `STORY-INDEX.md` under `.factory/stories/`.
///
/// Uses path-component-strict, contiguity-anchored matching:
/// `file_name == "STORY-INDEX.md"` AND the `.factory` / `stories` chain as a
/// CONTIGUOUS, correctly-ordered window immediately preceding the filename
/// (SEC-001 / CWE-697 — not independent presence checks).
///
/// # BC trace
/// BC-5.39.010 precondition 22 (Class B Arm2 trigger condition).
/// BC-5.39.010 invariant 3.
pub fn is_story_index(file_path: &str) -> bool {
    use std::path::Path;
    let path = Path::new(file_path);
    let components: Vec<_> = path.components().collect();
    let is_story_index = path
        .file_name()
        .and_then(|f| f.to_str())
        .map(|f| f == "STORY-INDEX.md")
        .unwrap_or(false);
    is_story_index && chain_immediately_precedes_filename(&components, &[".factory", "stories"], 0)
}

/// Returns `Some(CycleArtifactKind)` if `file_path` names a cycle artifact
/// under `.factory/cycles/` that Arm D should scan.
///
/// **[DEFERRED v1.6 — Class D]**: BC-5.39.010 v1.6 descopes Class D (D-953,
/// human-approved). This function always returns `None` so cycle artifact writes
/// are unclassified → `HookResult::Continue` (no primary read attempted).
/// `.factory/cycles/` is removed from `path_allow` as part of this deferral.
///
/// When Class D is re-enabled, restore the body from git history and add
/// `.factory/cycles/` back to path_allow in hooks-registry.toml.
///
/// # BC trace
/// BC-5.39.010 precondition 28 (Class D trigger condition — DEFERRED).
/// BC-5.39.010 invariant 3.
pub fn is_cycle_artifact(_file_path: &str) -> Option<CycleArtifactKind> {
    // [DEFERRED v1.6 — Class D]: cycle artifact dispatch removed per BC-5.39.010 v1.6.
    // Cycle artifact writes are unclassified → Continue (no primary read attempted).
    None
}

/// Returns `true` if `filename` is a canonical VP basename: `^VP-[0-9]+\.md$`.
///
/// Admits `VP-039.md`, `VP-100.md`, etc. Rejects `VP-INDEX.md` (explicit basename guard
/// per PC34 REQUIRED defence-in-depth) and `VP-9999-test.md` (inner "9999-test" contains
/// a non-digit).
///
/// # BC trace
/// BC-5.39.010 PC34: VP canonical filename pattern `^VP-[0-9]+\.md$`.
/// F-P2-003+F-P2-008: prior `starts_with("VP-") && ends_with(".md")` admitted VP-INDEX.md.
/// F-S2107-P3-011: explicit `filename == "VP-INDEX.md"` guard REQUIRED for defence-in-depth
/// per PC34 bullet 2 (normative "REQUIRED") — digit predicate excludes VP-INDEX.md today
/// but a future relaxation of the digit predicate would silently re-admit it without this guard.
fn is_canonical_vp_filename(filename: &str) -> bool {
    // Defence-in-depth guard: explicitly reject VP-INDEX.md regardless of digit predicate.
    // PC34 bullet 2 (normative "REQUIRED"): "an explicit basename guard
    // `file_name() != "VP-INDEX.md"` is REQUIRED for defence-in-depth".
    if filename == "VP-INDEX.md" {
        return false;
    }
    let inner = filename
        .strip_prefix("VP-")
        .and_then(|s| s.strip_suffix(".md"))
        .unwrap_or("");
    !inner.is_empty() && inner.chars().all(|c| c.is_ascii_digit())
}

/// Returns `true` if `file_path` is a BC, VP, story, or epic file (Class E trigger).
///
/// Class E (frontmatter parity) fires on any file with `version:` and `last_amended:`
/// frontmatter fields: BC files, VP files, story files, and epic files.
///
/// # BC trace
/// BC-5.39.010 precondition 34 (Class E trigger condition).
pub fn is_frontmatter_parity_target(file_path: &str) -> bool {
    use std::path::Path;
    // BC files
    if is_bc_file(file_path) {
        return true;
    }
    // Story files (but not STORY-INDEX.md)
    if is_story_file(file_path) {
        return true;
    }
    // VP files under .factory/specs/verification-properties/ — the
    // `.factory` / `specs` / `verification-properties` chain must be a
    // CONTIGUOUS, correctly-ordered window immediately preceding the filename
    // (SEC-001 / CWE-697 — not independent presence checks).
    let path = Path::new(file_path);
    let components: Vec<_> = path.components().collect();
    let vp_filename_ok = path
        .file_name()
        .and_then(|f| f.to_str())
        .map(is_canonical_vp_filename)
        .unwrap_or(false);
    if vp_filename_ok
        && chain_immediately_precedes_filename(
            &components,
            &[".factory", "specs", "verification-properties"],
            0,
        )
    {
        return true;
    }
    // Epic files under .factory/stories/epics/ — PC34 bullet 4 (F-S2107-P3-009):
    // requires the `.factory` / `stories` / `epics` chain as a CONTIGUOUS,
    // correctly-ordered window immediately preceding the filename (SEC-001 /
    // CWE-697), AND basename matching `^E-[0-9]+-.*\.md$`. Absent contiguity
    // or wrong basename → false.
    let epic_filename_ok = path
        .file_name()
        .and_then(|f| f.to_str())
        .map(is_canonical_epic_basename)
        .unwrap_or(false);
    epic_filename_ok
        && chain_immediately_precedes_filename(&components, &[".factory", "stories", "epics"], 0)
}

/// Returns `true` if `filename` is a canonical epic basename: `^E-[0-9]+-.*\.md$`.
///
/// Admits `E-21-name.md`, `E-1-foo.md`. Rejects `README.md`, `E-.md`, `E-21.md`
/// (missing `-` separator after digits), `E-21-name.txt`.
///
/// # BC trace
/// BC-5.39.010 PC34 bullet 4: epic basename pattern `^E-[0-9]+-.*\.md$`.
/// F-S2107-P3-009: prior `ends_with(".md")` admitted README.md and any .md file.
fn is_canonical_epic_basename(filename: &str) -> bool {
    let Some(rest) = filename.strip_prefix("E-") else {
        return false;
    };
    let bytes = rest.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return false;
    }
    let mut i = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    // Digits must be followed by '-'
    if i >= bytes.len() || bytes[i] != b'-' {
        return false;
    }
    // Remainder must end with ".md"
    filename.ends_with(".md")
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::*;

    // -----------------------------------------------------------------------
    // is_bc_file — BC-5.39.010 precondition 1 + invariant 3
    // -----------------------------------------------------------------------

    #[test]
    fn test_BC_5_39_010_dispatch_bc_file_factory_path_detected() {
        // BC-5.39.010 PC1: canonical BC path under .factory/specs/behavioral-contracts/
        let result = is_bc_file(".factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md");
        assert!(result, "canonical BC path must be detected");
    }

    #[test]
    fn test_BC_5_39_010_dispatch_bc_file_non_factory_path_rejected() {
        // Invariant 3: path-component-strict — missing .factory component must reject
        let result = is_bc_file("specs/behavioral-contracts/ss-05/BC-5.39.010.md");
        assert!(!result, "path without .factory component must not match");
    }

    #[test]
    fn test_BC_5_39_010_dispatch_story_file_detected() {
        // BC-5.39.010 PC9: story file under .factory/stories/
        let result =
            is_story_file(".factory/stories/S-21.07-validate-cross-site-correspondence.md");
        assert!(result, "canonical story path must be detected");
    }

    #[test]
    fn test_BC_5_39_010_dispatch_story_index_not_story_file() {
        // STORY-INDEX.md must NOT match is_story_file (it's Arm B2's trigger)
        let result = is_story_file(".factory/stories/STORY-INDEX.md");
        assert!(
            !result,
            "STORY-INDEX.md must not be classified as a story file"
        );
    }

    #[test]
    fn test_BC_5_39_010_dispatch_story_index_detected() {
        // BC-5.39.010 PC22: STORY-INDEX.md trigger
        let result = is_story_index(".factory/stories/STORY-INDEX.md");
        assert!(result, "STORY-INDEX.md must be detected as story index");
    }

    // ADV-RECON-005: the former test_BC_5_39_010_dispatch_burst_log_detected and
    // test_BC_5_39_010_dispatch_lessons_md_detected `#[ignore]`d tests asserted
    // `Some(CycleArtifactKind::BurstLog/Lessons)` — directly contradicting the active
    // Class-D-deferred `..._returns_none` tests below (which assert `is_cycle_artifact`
    // returns `None` for the SAME two paths, per BC-5.39.010 v1.6 / D-953 Class D
    // deferral). An `#[ignore]`d test that asserts the OPPOSITE of current normative
    // behavior is dead, misleading test content — removed rather than retained
    // ignored. The `returns_none` assertions below are the sole active source of
    // truth for `is_cycle_artifact` until S-21.08 re-enables Class D.

    #[test]
    fn test_BC_5_39_010_dispatch_non_cycle_index_md_rejected() {
        // .factory/stories/STORY-INDEX.md must not trigger the cycle artifact arm
        let result = is_cycle_artifact(".factory/stories/STORY-INDEX.md");
        assert!(
            result.is_none(),
            "stories/STORY-INDEX.md must not classify as cycle artifact"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-005: BC-INDEX.md must not be classified as a BC file.
    // is_bc_file uses `starts_with("BC-") && ends_with(".md")` which admits
    // "BC-INDEX.md" — but BC-INDEX.md is the index file, not a behavioral contract.
    // When classified as a BC file, arm A1 runs with BC-INDEX.md as primary target,
    // treating "BC-INDEX.md" as the bc_id, producing spurious violations.
    // BC-5.39.010 v1.19 §Classification invariant: bc_id guard excludes index.
    // -----------------------------------------------------------------------

    /// T-035: BC-INDEX.md must NOT be classified as a BC file (F-S2107-P1B-005).
    ///
    /// RED GATE: current `starts_with("BC-") && ends_with(".md")` matches "BC-INDEX.md".
    /// `is_bc_file` returns true → assert!(!result) FAILS → RED gate.
    /// After fix (exclude index file by name): returns false → PASSES.
    #[test]
    fn test_BC_5_39_010_dispatch_bc_index_not_bc_file() {
        let result = is_bc_file(".factory/specs/behavioral-contracts/BC-INDEX.md");
        assert!(
            !result,
            "BC-INDEX.md must NOT be classified as a BC file — it is the index, \
            not a behavioral contract (F-S2107-P1B-005)"
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P1B-010: Epic file under .factory/stories/epics/ must not be
    // classified as a story file. is_story_file has no basename guard against
    // the epics/ subdirectory, so any .md file under .factory/stories/ passes.
    // BC-5.39.010 v1.19 §Classification invariant: story_id must match S-XX.YY format.
    // -----------------------------------------------------------------------

    /// F-S2107-P1B-010: epic file in stories/epics/ must NOT be classified as story file.
    ///
    /// RED GATE: current code has no basename guard for `S-` prefix; any `.md` not named
    /// STORY-INDEX.md under `.factory/stories/` matches. `is_story_file` returns true for
    /// "E-21-W4.md" → assert!(!result) FAILS → RED gate.
    /// After fix (basename must start with "S-"): returns false → PASSES.
    #[test]
    fn test_BC_5_39_010_dispatch_epic_file_not_story_file() {
        let result = is_story_file(".factory/stories/epics/E-21-W4.md");
        assert!(
            !result,
            "epic file under .factory/stories/epics/ must NOT be classified as a story file \
            (F-S2107-P1B-010)"
        );
    }

    /// F-S2107-P1B-010: canonical story basename IS classified as story file.
    ///
    /// Complement test: verifies the story guard still fires for real story basenames
    /// after the basename guard is applied.
    #[test]
    fn test_BC_5_39_010_dispatch_story_file_s_prefix_basename_accepted() {
        let result =
            is_story_file(".factory/stories/S-21.07-validate-cross-site-correspondence.md");
        assert!(
            result,
            "canonical story file with S-XX.YY basename must be classified as story file"
        );
    }

    // -----------------------------------------------------------------------
    // F-P2-003 + F-P2-008 (BLOCKER): VP-INDEX.md admitted by is_frontmatter_parity_target.
    //
    // Bug: vp_filename_ok uses `f.starts_with("VP-") && f.ends_with(".md")` which
    // admits "VP-INDEX.md". BC-5.39.010 PC34 requires the basename to match
    // `^VP-[0-9]+\.md$` and mandates an explicit `file_name() != "VP-INDEX.md"` guard.
    //
    // When VP-INDEX.md is classified as a frontmatter parity target, Class E1 runs
    // against it. VP-INDEX.md has no `version:` or `last_amended:` frontmatter →
    // precondition 37 fires → advisory + Continue (not blocking). But the primary-read
    // still fires and the spurious arm call wastes cycles and may emit noise.
    //
    // F-P2-003 and F-P2-008 both trace to the same root (flat VP path + INDEX guard).
    // -----------------------------------------------------------------------

    /// F-P2-003+F-P2-008 (BLOCKER): VP-INDEX.md must NOT be a frontmatter parity target.
    ///
    /// RED GATE: `f.starts_with("VP-") && f.ends_with(".md")` → true for "VP-INDEX.md" →
    /// is_frontmatter_parity_target returns true → `assert!(!result)` FAILS.
    /// After fix (guard `file_name() != "VP-INDEX.md"` OR digit-anchored regex): returns false.
    #[test]
    fn test_BC_5_39_010_dispatch_vp_index_excluded_from_class_e() {
        let result =
            is_frontmatter_parity_target(".factory/specs/verification-properties/VP-INDEX.md");
        assert!(
            !result,
            "VP-INDEX.md must NOT be classified as a frontmatter parity target. \
            BC-5.39.010 PC34: explicit VP-INDEX.md guard required. \
            F-P2-003+F-P2-008: starts_with('VP-')&&ends_with('.md') admits VP-INDEX.md. \
            RED GATE: current check returns true."
        );
    }

    /// F-P2-003+F-P2-008 complement: canonical VP file IS classified as parity target.
    ///
    /// Regression guard: verifies the guard does not over-exclude real VP files.
    /// This test PASSES in Red Gate (is_frontmatter_parity_target already returns true for
    /// canonical VP files).
    #[test]
    fn test_BC_5_39_010_dispatch_vp_canonical_file_accepted_as_class_e_target() {
        let result =
            is_frontmatter_parity_target(".factory/specs/verification-properties/VP-039.md");
        assert!(
            result,
            "canonical VP file VP-039.md must be classified as a frontmatter parity target \
            (Class E regression guard after VP-INDEX.md exclusion fix)"
        );
    }

    // -----------------------------------------------------------------------
    // F-P2-011 (HIGH): is_story_file must require numeric section.subsection ID.
    //
    // Bug: `filename.starts_with("S-")` is too broad — admits S-README.md, S-ARCH.md,
    // S-TODO.md, S-NOTES.md, and any other `.md` file starting with "S-" that is not
    // a story. BC-5.39.010 PC9: story basename must match `^S-[0-9]+\.[0-9]+.*\.md$`.
    // -----------------------------------------------------------------------

    /// F-P2-011 (HIGH): S-README.md must not be classified as a story file.
    ///
    /// RED GATE: `"S-README.md".starts_with("S-")` → true → is_story_file returns true
    /// → `assert!(!result)` FAILS.
    /// After fix (regex guard `^S-[0-9]+\.[0-9]+`): "S-README.md" has no numeric part
    /// → returns false → PASSES.
    #[test]
    fn test_BC_5_39_010_dispatch_story_file_s_readme_rejected_requires_numeric_id() {
        let result = is_story_file(".factory/stories/S-README.md");
        assert!(
            !result,
            "S-README.md must NOT be classified as a story file. \
            PC9: basename must match ^S-[0-9]+\\.[0-9]+.*\\.md$ (has numeric section+subsection). \
            F-P2-011: starts_with('S-') too broad — admits S-README, S-ARCH, etc. \
            RED GATE: current check returns true."
        );
    }

    // -----------------------------------------------------------------------
    // Class D deferral (BC-5.39.010 v1.6): is_cycle_artifact MUST return None
    // for all paths once the Class D dispatch arm is removed.
    //
    // F-P2-007: invariant-6 I/O-BLOCK vs content-advisory split.
    // Resolution per BC-5.39.010 v1.6: Class D is DEFERRED. The implementer
    // removes the is_cycle_artifact dispatch, run_arm_d, and .factory/cycles/
    // from path_allow. After removal, cycle artifact writes are unclassified →
    // Continue (no primary read attempted).
    //
    // The EXISTING tests test_BC_5_39_010_dispatch_burst_log_detected and
    // test_BC_5_39_010_dispatch_lessons_md_detected assert Some(BurstLog/Lessons).
    // The implementer must update those tests to assert None (or DEFERRED-skip them)
    // when removing the Class D arm. Both cannot coexist with these new tests.
    // -----------------------------------------------------------------------

    /// [DEFERRED v1.6 — Class D] burst-log.md must return None after Class D removal.
    ///
    /// Pre-fix Red Gate (now closed): is_cycle_artifact used to return Some(BurstLog) →
    /// `result.is_none()` was false → assertion failed.
    /// After fix (remove is_cycle_artifact cycle-artifact dispatch): returns None.
    #[test]
    fn test_BC_5_39_010_dispatch_class_d_deferred_burst_log_returns_none() {
        // BC-5.39.010 v1.6 Class D DEFERRED: is_cycle_artifact must return None.
        // Cycle artifact writes become unclassified → no primary read → Continue.
        // F-P2-007 resolution: Class D arm removed → is_cycle_artifact → None for all paths.
        let result =
            is_cycle_artifact(".factory/cycles/v1.0-feature-engine-discipline-pass-1/burst-log.md");
        assert!(
            result.is_none(),
            "burst-log.md must NOT classify as cycle artifact after Class D deferral \
            (BC-5.39.010 v1.6 Class D DEFERRED). \
            F-P2-007 resolution: is_cycle_artifact dispatch removed. \
            Pre-fix Red Gate (now closed): used to return Some(BurstLog)."
        );
    }

    /// [DEFERRED v1.6 — Class D] lessons.md must return None after Class D removal.
    ///
    /// Pre-fix Red Gate (now closed): is_cycle_artifact used to return Some(Lessons) →
    /// `result.is_none()` was false → assertion failed.
    /// After fix: returns None → PASSES.
    #[test]
    fn test_BC_5_39_010_dispatch_class_d_deferred_lessons_returns_none() {
        let result =
            is_cycle_artifact(".factory/cycles/v1.0-feature-engine-discipline-pass-1/lessons.md");
        assert!(
            result.is_none(),
            "lessons.md must NOT classify as cycle artifact after Class D deferral \
            (BC-5.39.010 v1.6 Class D DEFERRED). \
            Pre-fix Red Gate (now closed): used to return Some(Lessons)."
        );
    }

    // -----------------------------------------------------------------------
    // F-S2107-P3-009 (HIGH): is_frontmatter_parity_target epic arm defects
    //
    // PC34 bullet 4 requires epic files to satisfy THREE conditions:
    //   1. `.factory` component present
    //   2. `stories` component present  ← MISSING from current impl
    //   3. `epics` component present
    //   4. Basename matches `^E-[0-9]+-`  ← MISSING from current impl
    //
    // Current impl checks only `.factory` + `epics` + ends-with-`.md`.
    // Consequence: `.factory/epics/README.md` → true (should be false).
    // Blast radius: any non-story .md under a path with "epics" component fires Class E.
    // -----------------------------------------------------------------------

    /// F-S2107-P3-009 RED GATE: epic file without `stories` component must NOT be
    /// classified as a frontmatter parity target.
    ///
    /// `.factory/epics/E-21-test.md` lacks the `stories` path component (correct path
    /// is `.factory/stories/epics/E-21-test.md`). Current impl: has `.factory` + `epics`
    /// + ends `.md` → true. After fix (requires `stories`): false.
    ///
    /// RED GATE: current impl returns true → assert!(!result) FAILS.
    #[test]
    fn test_BC_5_39_010_dispatch_epic_missing_stories_component_rejected() {
        let result = is_frontmatter_parity_target(".factory/epics/E-21-test.md");
        assert!(
            !result,
            "'.factory/epics/E-21-test.md' lacks the `stories` path component and must \
            NOT be classified as a frontmatter parity target. \
            PC34 bullet 4: epic path requires both `stories` AND `epics` components. \
            F-S2107-P3-009: current impl checks only `.factory`+`epics`+`.md` → true. \
            RED GATE."
        );
    }

    /// F-S2107-P3-009 RED GATE: epic file without numeric basename must NOT be classified
    /// as a frontmatter parity target.
    ///
    /// `.factory/stories/epics/README.md` has the correct path components but
    /// `README.md` does not match `^E-[0-9]+-`. Current impl: any `.md` under
    /// `.factory/...epics/` → true. After fix (requires `^E-[0-9]+-` basename): false.
    ///
    /// RED GATE: current impl returns true → assert!(!result) FAILS.
    #[test]
    fn test_BC_5_39_010_dispatch_epic_non_numeric_basename_rejected() {
        let result = is_frontmatter_parity_target(".factory/stories/epics/README.md");
        assert!(
            !result,
            "'.factory/stories/epics/README.md' has basename 'README.md' which does not \
            match `^E-[0-9]+-`. Must NOT be classified as frontmatter parity target. \
            PC34 bullet 4: epic basename must match `^E-[0-9]+-.*\\.md$`. \
            F-S2107-P3-009: current `ends_with('.md')` admits all .md files under epics/. \
            RED GATE."
        );
    }

    /// F-S2107-P3-009 GREEN: canonical epic file at correct path IS a parity target.
    ///
    /// Complement test: after fix, `.factory/stories/epics/E-21-name.md` must still
    /// be classified correctly. Verifies the guard does not over-exclude real epics.
    #[test]
    fn test_BC_5_39_010_dispatch_epic_correct_path_accepted() {
        let result = is_frontmatter_parity_target(
            ".factory/stories/epics/E-21-factory-state-data-loss-hardening.md",
        );
        assert!(
            result,
            "canonical epic path '.factory/stories/epics/E-21-*.md' must be a \
            frontmatter parity target (PC34 bullet 4). GREEN guard test."
        );
    }

    // -----------------------------------------------------------------------
    // SEC-001 (CWE-697 component-anchoring over-inclusion, security triage
    // reclassified LOW — real defect is spurious-block, not bypass; the actual
    // traversal defense is factory-dispatcher/src/host/path_util.rs::check_path_allowed,
    // Kani-proven VP-097, and is untouched here).
    //
    // Root cause: is_bc_file / is_story_file / is_story_index /
    // is_frontmatter_parity_target tested for required path COMPONENTS with
    // independent `.any()`-style presence checks that did not require the
    // components to be CONTIGUOUS and correctly ORDERED immediately before the
    // matched filename. A decoy path with the required component names present
    // but non-contiguous / out-of-order / separated by `..` traversal segments
    // was wrongly classified as a governed path.
    //
    // Fix: `chain_immediately_precedes_filename` anchors the required directory
    // chain to a position derived from the total component count (not searched),
    // so the chain must appear as a contiguous, ordered window immediately
    // (modulo an explicit `gap` of plain directory components, e.g. the BC
    // ss-NN subdirectory) before the file name. Arbitrary prefix components
    // BEFORE the chain remain unconstrained so real absolute/worktree paths
    // (where `.factory` is not the first component) still classify correctly.
    // -----------------------------------------------------------------------

    /// SEC-001 RED GATE: is_bc_file — non-contiguous decoy (no `..`) must be
    /// rejected. Pre-fix `.any()` logic finds ".factory", "specs",
    /// "behavioral-contracts" present anywhere (out of order / non-contiguous)
    /// plus a canonical BC filename, and wrongly returns true.
    #[test]
    fn test_SEC_001_dispatch_bc_file_non_contiguous_decoy_rejected() {
        let result = is_bc_file("x/.factory/y/specs/z/behavioral-contracts/BC-1.2.3.md");
        assert!(
            !result,
            "decoy path with non-contiguous, interspersed governed component names \
            must NOT classify as a BC file (SEC-001 CWE-697 component-anchoring gap)."
        );
    }

    /// SEC-001 RED GATE: is_bc_file — contiguous real chain followed by `..`
    /// traversal segments before the decoy destination must be rejected. Pre-fix
    /// `.any()` logic ignores what comes AFTER the required components and
    /// wrongly returns true because ".factory"/"specs"/"behavioral-contracts"
    /// are all present (contiguous, even) and the filename matches the BC
    /// pattern.
    #[test]
    fn test_SEC_001_dispatch_bc_file_dotdot_after_chain_decoy_rejected() {
        let result =
            is_bc_file(".factory/specs/behavioral-contracts/../../../../tmp/evil/BC-1.2.3.md");
        assert!(
            !result,
            "decoy path where '..' segments follow the real governed chain before \
            reaching the destination must NOT classify as a BC file \
            (SEC-001 CWE-697 component-anchoring gap)."
        );
    }

    /// SEC-001 POSITIVE control: canonical BC path in ABSOLUTE/worktree form
    /// (`.factory` is not the first component) must still classify true.
    #[test]
    fn test_SEC_001_dispatch_bc_file_absolute_form_accepted() {
        let result = is_bc_file(
            "/Users/dev/vsdd-factory/.factory/specs/behavioral-contracts/ss-05/BC-5.39.010.md",
        );
        assert!(
            result,
            "canonical BC path in absolute/worktree form must still classify as a \
            BC file — the governed chain need not start at component index 0."
        );
    }

    /// SEC-001 RED GATE: is_story_file — non-contiguous decoy (no `..`) must be
    /// rejected.
    #[test]
    fn test_SEC_001_dispatch_story_file_non_contiguous_decoy_rejected() {
        let result = is_story_file("x/.factory/y/stories/S-21.07-decoy.md");
        assert!(
            !result,
            "decoy path with non-contiguous governed component names must NOT \
            classify as a story file (SEC-001 CWE-697 component-anchoring gap)."
        );
    }

    /// SEC-001 RED GATE: is_story_file — real chain followed by `..` segments
    /// before the decoy destination must be rejected.
    #[test]
    fn test_SEC_001_dispatch_story_file_dotdot_after_chain_decoy_rejected() {
        let result = is_story_file(".factory/stories/../../../../tmp/evil/S-1.1-decoy.md");
        assert!(
            !result,
            "decoy path where '..' segments follow the real governed chain must \
            NOT classify as a story file (SEC-001 CWE-697 component-anchoring gap)."
        );
    }

    /// SEC-001 POSITIVE control: canonical story path in absolute/worktree form.
    #[test]
    fn test_SEC_001_dispatch_story_file_absolute_form_accepted() {
        let result = is_story_file(
            "/Users/dev/vsdd-factory/.factory/stories/S-21.07-validate-cross-site-correspondence.md",
        );
        assert!(
            result,
            "canonical story path in absolute/worktree form must still classify as \
            a story file."
        );
    }

    /// SEC-001 RED GATE: is_story_index — non-contiguous decoy (no `..`) must
    /// be rejected.
    #[test]
    fn test_SEC_001_dispatch_story_index_non_contiguous_decoy_rejected() {
        let result = is_story_index("a/.factory/b/stories/STORY-INDEX.md");
        assert!(
            !result,
            "decoy path with non-contiguous governed component names must NOT \
            classify as the story index (SEC-001 CWE-697 component-anchoring gap)."
        );
    }

    /// SEC-001 POSITIVE control: STORY-INDEX.md in absolute/worktree form.
    #[test]
    fn test_SEC_001_dispatch_story_index_absolute_form_accepted() {
        let result = is_story_index("/Users/dev/vsdd-factory/.factory/stories/STORY-INDEX.md");
        assert!(
            result,
            "STORY-INDEX.md in absolute/worktree form must still classify as the \
            story index."
        );
    }

    /// SEC-001 RED GATE: is_frontmatter_parity_target (VP arm) — non-contiguous
    /// decoy (no `..`) must be rejected.
    #[test]
    fn test_SEC_001_dispatch_fpm_vp_non_contiguous_decoy_rejected() {
        let result =
            is_frontmatter_parity_target("a/.factory/b/specs/c/verification-properties/VP-039.md");
        assert!(
            !result,
            "decoy VP path with non-contiguous governed component names must NOT \
            classify as a frontmatter parity target (SEC-001 CWE-697 gap)."
        );
    }

    /// SEC-001 RED GATE: is_frontmatter_parity_target (VP arm) — real chain
    /// followed by `..` segments before the decoy destination must be rejected.
    #[test]
    fn test_SEC_001_dispatch_fpm_vp_dotdot_after_chain_decoy_rejected() {
        let result = is_frontmatter_parity_target(
            ".factory/specs/verification-properties/../../../../tmp/evil/VP-039.md",
        );
        assert!(
            !result,
            "decoy VP path where '..' segments follow the real governed chain must \
            NOT classify as a frontmatter parity target (SEC-001 CWE-697 gap)."
        );
    }

    /// SEC-001 POSITIVE control: canonical VP path in absolute/worktree form.
    #[test]
    fn test_SEC_001_dispatch_fpm_vp_absolute_form_accepted() {
        let result = is_frontmatter_parity_target(
            "/Users/dev/vsdd-factory/.factory/specs/verification-properties/VP-039.md",
        );
        assert!(
            result,
            "canonical VP path in absolute/worktree form must still classify as a \
            frontmatter parity target."
        );
    }

    /// SEC-001 RED GATE: is_frontmatter_parity_target (epic arm) — non-contiguous
    /// decoy (no `..`) must be rejected.
    #[test]
    fn test_SEC_001_dispatch_fpm_epic_non_contiguous_decoy_rejected() {
        let result = is_frontmatter_parity_target("a/.factory/b/stories/c/epics/E-21-decoy.md");
        assert!(
            !result,
            "decoy epic path with non-contiguous governed component names must NOT \
            classify as a frontmatter parity target (SEC-001 CWE-697 gap)."
        );
    }

    /// SEC-001 POSITIVE control: canonical epic path in absolute/worktree form.
    #[test]
    fn test_SEC_001_dispatch_fpm_epic_absolute_form_accepted() {
        let result = is_frontmatter_parity_target(
            "/Users/dev/vsdd-factory/.factory/stories/epics/E-21-factory-state-data-loss-hardening.md",
        );
        assert!(
            result,
            "canonical epic path in absolute/worktree form must still classify as \
            a frontmatter parity target."
        );
    }

    // -----------------------------------------------------------------------
    // SEC-001 DEFINITIVE CLOSE (CWE-697 residual): a leading `..` BEFORE the
    // anchored chain still classifies TRUE today, because
    // `chain_immediately_precedes_filename` derives the chain's start position
    // from the total component count and leaves components BEFORE that
    // position entirely unconstrained (that unconstrained-prefix behavior is
    // intentional and correct for real absolute/worktree paths, where
    // `.factory` legitimately isn't component 0 — see the
    // `*_absolute_form_accepted` positive controls above). A `../.factory/...`
    // decoy exploits that same unconstrained prefix by placing `ParentDir`
    // there instead of ordinary directory names.
    //
    // Research-confirmed impact: bounded spurious block (availability only) —
    // the actual traversal defense is the Kani-proven
    // `path_util::check_path_allowed` downstream, which canonicalizes and
    // fails closed on any real escape. This is a classification-correctness
    // close, not a bypass close. No legitimate governed corpus path ever
    // contains `..`; the fix rejects the whole path if ANY component is
    // `Component::ParentDir`, wherever it appears (before, within, or after
    // the anchored chain) — while leaving `CurDir` / `RootDir` / `Prefix`
    // unconstrained, since those are legitimate for `./` relative and
    // absolute/worktree paths (the existing positive-control tests above
    // must still pass unchanged).
    //
    // RED GATE (pre-fix): `chain_immediately_precedes_filename` only
    // examines the chain/gap window; a `ParentDir` sitting in the prefix
    // (before `chain_start`) is invisible to it, so these paths wrongly
    // classify true today. `assert!(!result)` FAILS pre-fix.
    // -----------------------------------------------------------------------

    /// SEC-001 definitive-close RED GATE: is_bc_file — leading `..` before the
    /// anchored `.factory`/specs/behavioral-contracts chain must be rejected.
    #[test]
    fn test_SEC_001_dispatch_bc_file_leading_dotdot_rejected() {
        let result = is_bc_file("../.factory/specs/behavioral-contracts/ss-05/BC-1.2.3.md");
        assert!(
            !result,
            "leading '..' before the anchored governed chain must NOT classify as \
            a BC file (SEC-001 CWE-697 definitive close — bounded over-acceptance \
            residual: components before chain_start were previously unconstrained)."
        );
    }

    /// SEC-001 definitive-close RED GATE: is_story_file — leading `..` before
    /// the anchored `.factory`/stories chain must be rejected.
    #[test]
    fn test_SEC_001_dispatch_story_file_leading_dotdot_rejected() {
        let result = is_story_file("../.factory/stories/S-1.1-decoy.md");
        assert!(
            !result,
            "leading '..' before the anchored governed chain must NOT classify as \
            a story file (SEC-001 CWE-697 definitive close)."
        );
    }

    /// SEC-001 definitive-close RED GATE: is_story_index — leading `..` before
    /// the anchored `.factory`/stories chain must be rejected.
    #[test]
    fn test_SEC_001_dispatch_story_index_leading_dotdot_rejected() {
        let result = is_story_index("../.factory/stories/STORY-INDEX.md");
        assert!(
            !result,
            "leading '..' before the anchored governed chain must NOT classify as \
            the story index (SEC-001 CWE-697 definitive close)."
        );
    }

    /// SEC-001 definitive-close RED GATE: is_frontmatter_parity_target (VP arm)
    /// — leading `..` before the anchored verification-properties chain must be
    /// rejected.
    #[test]
    fn test_SEC_001_dispatch_fpm_vp_leading_dotdot_rejected() {
        let result =
            is_frontmatter_parity_target("../.factory/specs/verification-properties/VP-039.md");
        assert!(
            !result,
            "leading '..' before the anchored governed chain must NOT classify as \
            a frontmatter parity target (SEC-001 CWE-697 definitive close, VP arm)."
        );
    }

    /// SEC-001 definitive-close RED GATE: is_frontmatter_parity_target (epic
    /// arm) — leading `..` before the anchored stories/epics chain must be
    /// rejected.
    #[test]
    fn test_SEC_001_dispatch_fpm_epic_leading_dotdot_rejected() {
        let result = is_frontmatter_parity_target(
            "../.factory/stories/epics/E-21-factory-state-data-loss-hardening.md",
        );
        assert!(
            !result,
            "leading '..' before the anchored governed chain must NOT classify as \
            a frontmatter parity target (SEC-001 CWE-697 definitive close, epic arm)."
        );
    }
}
