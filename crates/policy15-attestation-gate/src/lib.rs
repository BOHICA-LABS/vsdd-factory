//! POLICY 15 ATTESTATION-LOCATION GATE — ADR-040 §Decisions 7-10 (v1.18)
//!
//! # Why this exists
//!
//! D-912 added a gate to prevent fix waves from pushing assertion-site changes
//! without co-updating `red-gate-log.md`. Across six bash-in-markdown iterations
//! every defect was found only by manual extraction-and-execution — none by code
//! review. POLICY 21 prohibits new `.sh` files; this crate is the POLICY 21-compliant
//! replacement. Controls are `#[test]` functions, making defect class 5
//! (four outcomes sharing two exit codes → control matches wrong exit code) impossible.
//!
//! # Five outcomes (ADR-040 v1.19)
//!
//! | Outcome | Exit code |
//! |---------|-----------|
//! | `Fail` | 2 |
//! | `PassWithActivations(N ≥ 1)` | 0 |
//! | `PassZeroActivations` | 0 |
//! | `EmptyOrUnreachable(_)` | 2 |
//! | `SkippedEmptyRange` | 0 |
//!
//! # Usage
//!
//! ```no_run
//! use policy15_attestation_gate::run_gate;
//! use std::path::Path;
//!
//! let result = run_gate(Path::new("."), "develop").expect("gate error");
//! println!("{}", result.outcome.identifier());
//! std::process::exit(result.outcome.exit_code());
//! ```

use std::path::Path;
use std::process::Command;

// ── Pinned constants (ADR-040 §Decision 8 Ruling 8(a)) ───────────────────────

/// Pinned crate path for the S-21.07 story.
pub const PLUGIN_CRATE: &str = "crates/hook-plugins/validate-cross-site-correspondence";

/// Pinned evidence log path for the S-21.07 story.
pub const RED_GATE_LOG: &str =
    "crates/hook-plugins/validate-cross-site-correspondence/docs/red-gate-log.md";

// ── Outcome types ─────────────────────────────────────────────────────────────

/// Why a commit failed the obligation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FailReason {
    /// The pinned log (`red-gate-log.md`) was absent at the activating commit.
    LogAbsent,
    /// The log was present but contained no `^### .*assertion-site attestation (<parent>)` heading.
    AttestationMissing,
    /// The pinned log was present and contained more than one
    /// `^### .*assertion-site attestation (<parent>)` heading for this commit's parent —
    /// ambiguous, not absent (ADR-040 §Decision 8 Ruling 8(b) amendment, v1.18).
    AttestationAmbiguous {
        /// Number of matching attestation headings found (always ≥ 2).
        count: usize,
    },
}

/// A commit that violated the POLICY 15 obligation.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct FailedCommit {
    /// Full 40-character commit SHA.
    pub commit: String,
    /// Why this commit failed.
    pub reason: FailReason,
}

/// Why the gate could not reach a measurable result.
///
/// Note: the empty/unresolvable commit-range case is NOT represented here.
/// Per ADR-040 v1.19 Ruling 9(f), that case maps to `GateOutcome::SkippedEmptyRange`
/// (exit 0, inert) rather than `EmptyOrUnreachable` (exit 2, blocking).
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum UnreachableCause {
    /// The pinned crate path is absent from the HEAD git tree (not the filesystem).
    ///
    /// Possible cause: crate renamed or restructured without updating the pin constant.
    StalePin,
    /// A commit in the range produced an empty changed-file set.
    ///
    /// Possible cause: `git commit --allow-empty`, shallow-clone boundary, or
    /// measurement tool failure.
    UnmeasurableDiff {
        /// The commit that produced the empty diff.
        commit: String,
    },
}

/// The five gate outcomes (ADR-040 §Decision 8 Ruling 8(c); fifth outcome added v1.19).
///
/// Mapping to process exit codes happens only in the binary (`src/main.rs`).
/// Tests assert on the variant, which makes defect class 5
/// (five outcomes / two exit codes) structurally impossible.
// NOTE: #[non_exhaustive] is deliberately absent so that exhaustive match at
// every call site is a compile error when a new outcome is added — the same
// property that guards PluginResult in the dispatcher crate.
#[derive(Debug, PartialEq, Eq)]
pub enum GateOutcome {
    /// One or more commits violated the POLICY 15 obligation.
    Fail(Vec<FailedCommit>),
    /// Gate activated for N ≥ 1 commits; all were compliant with the obligation.
    PassWithActivations(usize),
    /// Scope target exists, commit range non-empty, all diffs measurable, zero activations.
    ///
    /// Legitimate for docs-only PRs. Named (not anonymous exit-0) so it is observable
    /// across CI runs — analogous to SystemVerilog `cover property (A)` pairing.
    PassZeroActivations,
    /// Structural prerequisite failed — CI setup defect or stale pin.
    EmptyOrUnreachable(UnreachableCause),
    /// The evaluated commit range was empty or the base branch was unresolvable.
    ///
    /// Per ADR-040 v1.19 Ruling 9(f): this is an INERT SKIP (exit 0). When there is
    /// no PR diff to evaluate (post-merge push to develop, or base ref absent), the gate
    /// has nothing to examine and must not block. Exit code 0 means CI green.
    ///
    /// Guard ordering preserved: `EmptyOrUnreachable(StalePin)` still beats this outcome
    /// (the stale-pin check fires before the commit-range computation).
    SkippedEmptyRange,
}

impl GateOutcome {
    /// Returns `true` for outcomes that map to exit code 0.
    pub fn is_pass(&self) -> bool {
        matches!(
            self,
            GateOutcome::PassWithActivations(_)
                | GateOutcome::PassZeroActivations
                | GateOutcome::SkippedEmptyRange
        )
    }

    /// Process exit code for this outcome.
    ///
    /// | Variant | Exit code |
    /// |---------|-----------|
    /// | `Fail` | 2 |
    /// | `PassWithActivations` | 0 |
    /// | `PassZeroActivations` | 0 |
    /// | `EmptyOrUnreachable` | 2 |
    /// | `SkippedEmptyRange` | 0 |
    pub fn exit_code(&self) -> i32 {
        if self.is_pass() { 0 } else { 2 }
    }

    /// Greppable outcome identifier string emitted to stdout in CI logs.
    ///
    /// The identifier uniquely identifies the outcome within a shared exit code —
    /// controls assert on this string, not the exit code alone (§Decision 10 Req 4).
    pub fn identifier(&self) -> String {
        match self {
            GateOutcome::Fail(_) => "FAIL: obligation violated".to_string(),
            GateOutcome::PassWithActivations(n) => format!("PASS-{n}-activations"),
            GateOutcome::PassZeroActivations => "PASS-zero-activations".to_string(),
            GateOutcome::EmptyOrUnreachable(UnreachableCause::StalePin) => {
                "EMPTY-or-UNREACHABLE: stale pin".to_string()
            }
            GateOutcome::EmptyOrUnreachable(UnreachableCause::UnmeasurableDiff { .. }) => {
                "EMPTY-or-UNREACHABLE: unmeasurable diff".to_string()
            }
            // ADR-040 v1.19 Ruling 9(f): inert skip — exit 0, greppable SKIP prefix.
            GateOutcome::SkippedEmptyRange => {
                "SKIP: empty or unresolvable commit range — inert (no PR diff to evaluate)"
                    .to_string()
            }
        }
    }
}

/// Result of a gate run: the verdict plus non-verdict-affecting diagnostics.
///
/// `skipped_parentless` and `skipped_merge_inert` are both orthogonal to `outcome` —
/// they record commits inside the evaluated range that were skipped without affecting
/// the verdict, for two different reasons:
///
/// - `skipped_parentless`: commits lacking a first parent (root commit or shallow-clone
///   boundary). Skipping such a commit never causes `Fail` (§Decision 9 Ruling 9(c) item 4).
///   `main.rs` prints an operator-facing WARNING for each entry — this is an unusual
///   condition worth operator attention.
/// - `skipped_merge_inert`: merge commits (parent-count > 1) whose COMBINED diff does not
///   touch the pinned crate's `.rs`/`.bats` surface — a routine sync-merge pulling in
///   content already attested on its own originating branch (§Decision 9 Ruling 9(e)).
///   `main.rs` prints NO warning for these — this is the expected, routine case for this
///   repository's branch-sync workflow, not an anomaly.
///
/// Printing operator-facing diagnostics from these fields is `main.rs`'s responsibility,
/// not this library's — mirrors the FAIL/EmptyOrUnreachable detail-line convention already
/// used there.
#[derive(Debug, PartialEq, Eq)]
pub struct GateResult {
    /// The four-way verdict (see `GateOutcome`).
    pub outcome: GateOutcome,
    /// Full 40-character SHAs of commits skipped for lacking a first parent. Empty in
    /// the common case.
    pub skipped_parentless: Vec<String>,
    /// Full 40-character SHAs of merge commits skipped as inert (combined diff does not
    /// touch the pinned crate's `.rs`/`.bats` surface). Empty in the common case.
    pub skipped_merge_inert: Vec<String>,
}

// ── Error type ────────────────────────────────────────────────────────────────

/// Hard failure: git not installed, repository inaccessible, or unexpected I/O error.
///
/// All semantic gate outcomes (including "stale pin" and "unresolvable base")
/// are returned as `Ok(GateOutcome)`, not `Err(GateError)`.
#[derive(Debug, thiserror::Error)]
pub enum GateError {
    /// A git command exited non-zero in a context where success was required.
    #[error("git command failed: {0}")]
    GitCommand(String),
    /// An I/O error occurred when spawning git.
    #[error("I/O error spawning git: {0}")]
    Io(#[from] std::io::Error),
    /// The file did not exist at the given commit.
    ///
    /// Used internally to distinguish "file absent at commit" from hard errors.
    #[error("file absent at {commit}:{path}")]
    FileAbsent { commit: String, path: String },
}

// ── Public API ────────────────────────────────────────────────────────────────

/// Run the POLICY 15 ATTESTATION-LOCATION GATE.
///
/// Computes `git merge-base HEAD origin/<base_branch>` as the range start.
/// If the merge base cannot be resolved (no `origin` remote, unknown branch),
/// returns `Ok(SkippedEmptyRange)` — inert skip, exit 0 (ADR-040 v1.19 Ruling 9(f)).
///
/// The stale-pin guard runs **before** the merge-base computation, so when both
/// the crate is absent *and* the base is unresolvable, `StalePin` is returned.
pub fn run_gate(repo: &Path, base_branch: &str) -> Result<GateResult, GateError> {
    // Guard 1 — stale-pin check BEFORE the origin/<branch> merge-base lookup.
    //
    // This guard is intentionally duplicated from the one in `run_gate_from_merge_base`
    // (guard 2). The duplication establishes a documented ordering invariant: when the
    // pinned crate is absent *and* the base branch is unresolvable, the gate reports
    // `StalePin` (not `SkippedEmptyRange`). Without guard 1, a repo where the crate was
    // renamed/deleted *and* the remote was removed would report `SkippedEmptyRange`
    // (inert, exit 0 per ADR-040 v1.19), masking the more actionable stale-pin diagnosis
    // with a silent pass instead of a CI-visible `EMPTY-or-UNREACHABLE: stale pin`.
    //
    // Pinned by: `test_run_gate_guard1_stale_pin_beats_unresolvable_base`.
    if !tree_path_exists(repo, "HEAD", PLUGIN_CRATE)? {
        return Ok(GateResult {
            outcome: GateOutcome::EmptyOrUnreachable(UnreachableCause::StalePin),
            skipped_parentless: Vec::new(),
            skipped_merge_inert: Vec::new(),
        });
    }

    let remote_ref = format!("origin/{base_branch}");
    let merge_base = match git_merge_base(repo, "HEAD", &remote_ref) {
        Ok(sha) => sha,
        Err(_) => {
            // Unresolvable base → inert skip per ADR-040 v1.19 Ruling 9(f).
            // The gate cannot evaluate any PR diff; exit 0 (not a blocking failure).
            return Ok(GateResult {
                outcome: GateOutcome::SkippedEmptyRange,
                skipped_parentless: Vec::new(),
                skipped_merge_inert: Vec::new(),
            });
        }
    };

    run_gate_inner(repo, &merge_base)
}

/// Run the gate with a pre-computed merge-base SHA.
///
/// This is the testable core: test fixtures construct scratch repos and pass the
/// initial commit SHA as `merge_base` (bypassing the `origin/` remote lookup).
///
/// The stale-pin guard is still applied — the `merge_base` parameter only
/// replaces the `git merge-base HEAD origin/<branch>` computation.
pub fn run_gate_from_merge_base(repo: &Path, merge_base: &str) -> Result<GateResult, GateError> {
    if !tree_path_exists(repo, "HEAD", PLUGIN_CRATE)? {
        return Ok(GateResult {
            outcome: GateOutcome::EmptyOrUnreachable(UnreachableCause::StalePin),
            skipped_parentless: Vec::new(),
            skipped_merge_inert: Vec::new(),
        });
    }
    run_gate_inner(repo, merge_base)
}

// ── Gate core ─────────────────────────────────────────────────────────────────

/// Core gate logic (stale-pin guard applied by callers).
fn run_gate_inner(repo: &Path, merge_base: &str) -> Result<GateResult, GateError> {
    let commits = git_log_range(repo, merge_base, "HEAD")?;
    if commits.is_empty() {
        // Empty range → inert skip per ADR-040 v1.19 Ruling 9(f).
        // Triggered when merge_base == HEAD (post-merge push, base==HEAD, etc.).
        // The gate has no PR diff to evaluate; exit 0.
        return Ok(GateResult {
            outcome: GateOutcome::SkippedEmptyRange,
            skipped_parentless: Vec::new(),
            skipped_merge_inert: Vec::new(),
        });
    }

    let mut fail_list: Vec<FailedCommit> = Vec::new();
    let mut empty_diff_commit: Option<String> = None;
    let mut activations: usize = 0;
    let mut skipped_parentless: Vec<String> = Vec::new();
    let mut skipped_merge_inert: Vec<String> = Vec::new();

    for commit_sha in &commits {
        // Skip parentless commits (root commit or shallow-clone boundary). CR-4:
        // `commit_has_parent` returns the resolved parent SHA so this loop avoids a
        // second `git rev-parse {commit}^1` spawn later for the attestation lookup.
        let parent_sha = match commit_has_parent(repo, commit_sha)? {
            Some(p) => p,
            None => {
                skipped_parentless.push(commit_sha.clone());
                continue;
            }
        };

        // H-1 (ADR-040 §Decision 9 Ruling 9(e)): merge commits (parent-count > 1) are
        // evaluated with git's COMBINED diff, which by construction shows only content
        // differing from EVERY parent — excluding pass-through content already attested
        // on its own originating branch. Ordinary commits (parent-count == 1) keep the
        // two-dot endpoint diff, unchanged.
        let parent_count = commit_parent_count(repo, commit_sha)?;
        let all_changed = if parent_count > 1 {
            let combined = git_diff_tree_combined_name_only(repo, commit_sha)?;
            if combined.is_empty() {
                // Inert sync-merge — the routine, expected case. Not an anomaly, so no
                // WARNING and no `empty_diff_commit` (that branch stays reserved for
                // parent-count == 1 commits — see the check below).
                skipped_merge_inert.push(commit_sha.clone());
                continue;
            }
            combined
        } else {
            git_diff_name_only(repo, &format!("{commit_sha}^1"), commit_sha)?
        };

        // Empty diff → unmeasurable scope (§Decision 8 trigger 3). Reserved for
        // parent-count == 1 commits; an empty combined diff on a merge commit is the
        // NORMAL case, already handled above as `skipped_merge_inert`.
        if all_changed.is_empty() {
            empty_diff_commit = Some(commit_sha.clone());
            continue;
        }

        // Does this commit touch the pinned crate's *.rs or *.bats?
        let crate_prefix = format!("{PLUGIN_CRATE}/");
        let activating = all_changed
            .iter()
            .any(|f| f.starts_with(&crate_prefix) && (f.ends_with(".rs") || f.ends_with(".bats")));

        if !activating {
            continue;
        }

        // Unconditional obligation: gate activates for this commit. Attestation lookup
        // keys on the FIRST parent (`{commit_sha}^1`) in both the ordinary and
        // merge-commit (Ruling 9(e)) cases — unchanged.
        activations += 1;

        // FAIL-when-absent: pinned log must exist at this commit (§Decision 8 Ruling 8(b)).
        let log_content = match git_show_file(repo, commit_sha, RED_GATE_LOG) {
            Ok(content) => content,
            Err(GateError::FileAbsent { .. }) => {
                fail_list.push(FailedCommit {
                    commit: commit_sha.clone(),
                    reason: FailReason::LogAbsent,
                });
                continue;
            }
            Err(e) => return Err(e),
        };

        // Count `^### .*assertion-site attestation (<parent_sha>)` headings. Ruling 8(b)
        // amendment (v1.18, M-3): absence (0) and ambiguity (>= 2) are distinct
        // `FailReason` variants — the `count != 1` FAIL boundary itself is unchanged.
        let count = count_attestation_headings(&log_content, &parent_sha);
        match count {
            1 => {}
            0 => fail_list.push(FailedCommit {
                commit: commit_sha.clone(),
                reason: FailReason::AttestationMissing,
            }),
            n => fail_list.push(FailedCommit {
                commit: commit_sha.clone(),
                reason: FailReason::AttestationAmbiguous { count: n },
            }),
        }
    }

    // Priority order: FAIL > EMPTY-or-UNREACHABLE > PASS.
    let outcome = if !fail_list.is_empty() {
        GateOutcome::Fail(fail_list)
    } else if let Some(commit) = empty_diff_commit {
        GateOutcome::EmptyOrUnreachable(UnreachableCause::UnmeasurableDiff { commit })
    } else if activations == 0 {
        GateOutcome::PassZeroActivations
    } else {
        GateOutcome::PassWithActivations(activations)
    };

    Ok(GateResult {
        outcome,
        skipped_parentless,
        skipped_merge_inert,
    })
}

// ── Attestation-heading matcher ───────────────────────────────────────────────

/// Count lines in `log_content` matching `^### .*assertion-site attestation (<parent_sha>)`.
///
/// The `^### ` anchor (via `starts_with`) prevents prose lines from contributing
/// a false count (ADR-040 §Decision 6 Ruling 6(b)).
fn count_attestation_headings(log_content: &str, parent_sha: &str) -> usize {
    let needle = format!("assertion-site attestation ({parent_sha})");
    log_content
        .lines()
        .filter(|line| line.starts_with("### ") && line.contains(&needle))
        .count()
}

// ── Git helpers ───────────────────────────────────────────────────────────────

fn run_git(repo: &Path, args: &[&str]) -> Result<std::process::Output, GateError> {
    Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .map_err(GateError::Io)
}

/// CR-3: run `git <args>` and return its trimmed single-line stdout.
///
/// Shared by `git_merge_base` and `commit_parent_count` — both invocations produce
/// exactly one line of stdout on success.
fn run_git_line(repo: &Path, args: &[&str]) -> Result<String, GateError> {
    let output = run_git(repo, args)?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(GateError::GitCommand(format!(
            "git {} exited {}",
            args.join(" "),
            output.status
        )))
    }
}

/// CR-3: run `git <args>` and return its non-blank stdout lines.
///
/// Shared by `git_log_range`, `git_diff_name_only`, and
/// `git_diff_tree_combined_name_only` — all three invocations produce a newline-
/// delimited list on stdout on success.
fn run_git_lines(repo: &Path, args: &[&str]) -> Result<Vec<String>, GateError> {
    let output = run_git(repo, args)?;
    if !output.status.success() {
        return Err(GateError::GitCommand(format!(
            "git {} exited {}",
            args.join(" "),
            output.status
        )));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect())
}

/// Returns `true` if `<treeish>:<path>` exists in the git object tree.
///
/// Uses `git cat-file -e` — a tree query, NOT a filesystem `is_dir()` check.
/// Prevents defect-3: a directory can exist on disk but be absent from the tree
/// if it was never committed (ADR-040 §Decision 8 stale-pin discussion).
fn tree_path_exists(repo: &Path, treeish: &str, path: &str) -> Result<bool, GateError> {
    let spec = format!("{treeish}:{path}");
    let output = run_git(repo, &["cat-file", "-e", &spec])?;
    Ok(output.status.success())
}

/// Returns the full 40-character SHA of `git merge-base a b`.
fn git_merge_base(repo: &Path, a: &str, b: &str) -> Result<String, GateError> {
    run_git_line(repo, &["merge-base", a, b])
}

/// Returns full SHAs of commits in `base..head`, newest-first.
fn git_log_range(repo: &Path, base: &str, head: &str) -> Result<Vec<String>, GateError> {
    let range = format!("{base}..{head}");
    run_git_lines(repo, &["log", &range, "--format=%H"])
}

/// Returns `Some(<resolved first-parent SHA>)` if `commit` has a first parent, or
/// `None` if it is a root or parentless commit (shallow-clone boundary).
///
/// CR-4: returns the resolved parent SHA (not just a bool) so callers that need the
/// parent SHA (the attestation-heading lookup key) reuse this result instead of
/// re-spawning `git rev-parse {commit}^1`.
fn commit_has_parent(repo: &Path, commit: &str) -> Result<Option<String>, GateError> {
    let parent_rev = format!("{commit}^1");
    let output = run_git(repo, &["rev-parse", "--verify", &parent_rev])?;
    if output.status.success() {
        Ok(Some(
            String::from_utf8_lossy(&output.stdout).trim().to_string(),
        ))
    } else {
        Ok(None)
    }
}

/// Returns the number of parents of `commit` (1 for an ordinary commit, > 1 for a
/// merge commit, 0 for a root commit — though root commits are filtered out by
/// `commit_has_parent` before this is called).
///
/// H-1 (ADR-040 §Decision 9 Ruling 9(e)): `git rev-list --parents -n1 <commit>` prints
/// `<commit> <parent1> <parent2> ...` on one line; the parent count is the token count
/// minus one (the commit itself).
fn commit_parent_count(repo: &Path, commit: &str) -> Result<usize, GateError> {
    let line = run_git_line(repo, &["rev-list", "--parents", "-n1", commit])?;
    let token_count = line.split_whitespace().count();
    Ok(token_count.saturating_sub(1))
}

/// Returns the list of files changed between `from` and `to` (one per line, no blanks).
///
/// H-2: `-c core.quotePath=false` disables git's default C-quoting of non-ASCII paths,
/// which would otherwise cause a non-ASCII `.rs`/`.bats` path to be missed by the
/// activating-path suffix/prefix match below (false PASS-zero).
fn git_diff_name_only(repo: &Path, from: &str, to: &str) -> Result<Vec<String>, GateError> {
    run_git_lines(
        repo,
        &[
            "-c",
            "core.quotePath=false",
            "diff",
            "--name-only",
            from,
            to,
        ],
    )
}

/// Returns the list of files in `commit`'s COMBINED diff — files whose content differs
/// from EVERY parent (H-1, ADR-040 §Decision 9 Ruling 9(e)).
///
/// H-2: `-c core.quotePath=false` applied for the same reason as `git_diff_name_only`.
fn git_diff_tree_combined_name_only(repo: &Path, commit: &str) -> Result<Vec<String>, GateError> {
    run_git_lines(
        repo,
        &[
            "-c",
            "core.quotePath=false",
            "diff-tree",
            "-c",
            "--name-only",
            "--no-commit-id",
            "-r",
            commit,
        ],
    )
}

/// Returns the content of `path` at `commit`.
///
/// Returns `GateError::FileAbsent` if the path does not exist at that commit.
fn git_show_file(repo: &Path, commit: &str, path: &str) -> Result<String, GateError> {
    let spec = format!("{commit}:{path}");
    let output = run_git(repo, &["show", &spec])?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(GateError::FileAbsent {
            commit: commit.to_string(),
            path: path.to_string(),
        })
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::*;
    use std::fs;
    use std::process::Command;
    use tempfile::TempDir;

    // ── Fixture helpers ───────────────────────────────────────────────────────

    struct Repo {
        dir: TempDir,
    }

    impl Repo {
        /// Create a scratch git repository with one initial empty commit.
        fn new() -> Self {
            let dir = TempDir::new().expect("TempDir::new");
            let r = Repo { dir };
            r.git(&["init"]);
            r.git(&["config", "user.email", "ci@vsdd-test"]);
            r.git(&["config", "user.name", "CI Fixture"]);
            r.git(&["commit", "--allow-empty", "-m", "initial"]);
            r
        }

        fn path(&self) -> &Path {
            self.dir.path()
        }

        fn git(&self, args: &[&str]) -> std::process::Output {
            Command::new("git")
                .current_dir(self.dir.path())
                .args(args)
                .output()
                .expect("git")
        }

        fn head_sha(&self) -> String {
            let out = self.git(&["rev-parse", "HEAD"]);
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        }

        /// CR-2 (v1.18): run `git <args>` and return its trimmed single-line stdout,
        /// asserting the command succeeded. Ported from
        /// `tests/binary_integration_test.rs`'s `Repo::git_stdout` — that file's module
        /// doc documents the established precedent for this exact duplication
        /// (integration test files are a separate compilation unit and cannot reach
        /// this private `#[cfg(test)]` module).
        fn git_stdout(&self, args: &[&str]) -> String {
            let out = self.git(args);
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        }

        /// Write `content` to `rel_path` (relative to repo root), creating parent dirs.
        fn write(&self, rel_path: &str, content: &str) {
            let full = self.dir.path().join(rel_path);
            if let Some(parent) = full.parent() {
                fs::create_dir_all(parent).expect("create_dir_all");
            }
            fs::write(&full, content).expect("fs::write");
        }

        /// Stage all changes and commit.
        fn commit(&self, message: &str) {
            self.git(&["add", "."]);
            self.git(&["commit", "-m", message]);
        }

        /// Write a file and commit it in one step.
        fn write_and_commit(&self, rel_path: &str, content: &str, message: &str) {
            self.write(rel_path, content);
            self.commit(message);
        }

        /// CR-2 (v1.18): port of `tests/binary_integration_test.rs`'s
        /// `create_range_containing_parentless_commit` — builds a genuinely parentless
        /// ("root-like") commit and merges it, via `git merge --allow-unrelated-histories`,
        /// back onto the tip of the current branch, so that:
        ///
        /// - `git merge-base <old-tip> <new-tip>` still resolves to `<old-tip>` (it is the
        ///   merge commit's FIRST parent), keeping `run_gate_from_merge_base`'s caller-supplied
        ///   merge-base on the expected path; and
        /// - `git log <old-tip>..<new-tip>` (what `run_gate_inner` evaluates) nonetheless
        ///   contains the parentless root commit, because it is reachable from the new tip via
        ///   the merge's SECOND parent and is not reachable from `<old-tip>` at all.
        ///
        /// The only file-level change introduced relative to `<old-tip>` is a single new file
        /// outside the pinned crate (non-activating), so the only way the fixture can affect
        /// the gate's outcome is through the skip-guard behavior under test.
        ///
        /// Returns the new parentless root commit's full SHA. Leaves the current branch at the
        /// new merge commit.
        fn create_range_containing_parentless_commit(&self) -> String {
            // 1. A parentless commit reusing the CURRENT (clean) tree verbatim.
            let base_tree = self.git_stdout(&["write-tree"]);
            let root_commit =
                self.git_stdout(&["commit-tree", &base_tree, "-m", "synthetic root commit"]);
            let parent_check = self.git(&["rev-parse", "--verify", &format!("{root_commit}^1")]);
            assert!(
                !parent_check.status.success(),
                "fixture bug: synthetic root commit unexpectedly has a parent"
            );

            // 2. A child of the root commit adding one new file (built directly via
            //    plumbing, so the current branch/index/working tree are never touched by
            //    this step).
            self.write("unrelated/extra.md", "extra\n");
            self.git(&["add", "unrelated/extra.md"]);
            let child_tree = self.git_stdout(&["write-tree"]);
            let child_commit = self.git_stdout(&[
                "commit-tree",
                &child_tree,
                "-p",
                &root_commit,
                "-m",
                "docs on unrelated root chain",
            ]);

            // 3. Restore the index to match the current branch tip (undo the `git add`
            //    from step 2) and remove the working-tree file so the upcoming merge
            //    starts from a clean, matching state.
            let reset = self.git(&["reset", "--mixed", "HEAD"]);
            assert!(reset.status.success());
            fs::remove_dir_all(self.path().join("unrelated")).expect("remove_dir_all");

            // 4. Merge the unrelated chain onto the current branch tip. Because
            //    `git merge` runs from the CURRENT branch, the resulting commit's first
            //    parent is the pre-merge tip and its second parent is `child_commit`
            //    (whose own parent is the parentless root commit).
            let merge = self.git(&[
                "merge",
                "--allow-unrelated-histories",
                "-m",
                "merge unrelated parentless-root chain",
                &child_commit,
            ]);
            assert!(
                merge.status.success(),
                "merge --allow-unrelated-histories failed: {}",
                String::from_utf8_lossy(&merge.stderr)
            );

            root_commit
        }
    }

    // Shared path constants used across all fixtures.
    const PC: &str = "crates/hook-plugins/validate-cross-site-correspondence";
    const LOG: &str = "crates/hook-plugins/validate-cross-site-correspondence/docs/red-gate-log.md";

    // ── Tests ─────────────────────────────────────────────────────────────────

    /// Positive control 1 — crate `.rs` change; evidence file absent → `Fail(LogAbsent)`.
    #[test]
    fn test_positive_1_absent_log() {
        let repo = Repo::new();
        let base = repo.head_sha();

        repo.write_and_commit(
            &format!("{PC}/src/lib.rs"),
            "// assertion site\n",
            "add assertion site, no log",
        );

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(
                &outcome,
                GateOutcome::Fail(v) if v.len() == 1 && v[0].reason == FailReason::LogAbsent
            ),
            "expected Fail(LogAbsent), got: {:?}",
            outcome
        );
        assert_eq!(outcome.identifier(), "FAIL: obligation violated");
        assert_eq!(outcome.exit_code(), 2);
    }

    /// Positive control 2 — crate `.rs` change; log present, no attestation heading
    /// → `Fail(AttestationMissing)`.
    #[test]
    fn test_positive_2_no_attestation_heading() {
        let repo = Repo::new();
        let base = repo.head_sha();

        // Single commit: .rs file + log present but no matching heading.
        repo.write(&format!("{PC}/src/lib.rs"), "// assertion site\n");
        repo.write(
            LOG,
            "# Red Gate Log\n\n## Observations\n\n(no attestation section)\n",
        );
        repo.commit("assertion site + log, no attestation heading");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(
                &outcome,
                GateOutcome::Fail(v) if !v.is_empty() && v[0].reason == FailReason::AttestationMissing
            ),
            "expected Fail(AttestationMissing), got: {:?}",
            outcome
        );
        assert_eq!(outcome.identifier(), "FAIL: obligation violated");
        assert_eq!(outcome.exit_code(), 2);
    }

    /// Negative control — crate `.rs` change; correct attestation heading for the parent SHA
    /// → `PassWithActivations(1)`.
    #[test]
    fn test_negative_compliant_attestation() {
        let repo = Repo::new();
        let base = repo.head_sha();
        // The commit we are about to make has HEAD (= base) as its parent.
        let parent_sha = base.clone();

        repo.write(&format!("{PC}/src/lib.rs"), "// assertion site\n");
        repo.write(
            LOG,
            &format!(
                "# Red Gate Log\n\n### Pass-1 assertion-site attestation ({parent_sha})\n\nAttestation body.\n"
            ),
        );
        repo.commit("assertion site + correct attestation heading");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(outcome, GateOutcome::PassWithActivations(1)),
            "expected PassWithActivations(1), got: {:?}",
            outcome
        );
        assert_eq!(outcome.identifier(), "PASS-1-activations");
        assert_eq!(outcome.exit_code(), 0);
        assert!(outcome.is_pass());
    }

    /// Pass-zero control — crate path seeded into tree; docs-only commit outside
    /// the crate → `PassZeroActivations`.
    ///
    /// Note: `mkdir -p` alone is insufficient — git never tracks empty directories.
    /// A non-`*.rs`/`*.bats` file must be committed inside the crate path for
    /// `git cat-file -e HEAD:<CRATE>` to succeed (ADR-040 §Decision 10 note on
    /// fixture construction).
    #[test]
    fn test_pass_zero_activations() {
        let repo = Repo::new();
        let base = repo.head_sha();

        // Seed the crate into the HEAD git tree via a placeholder file.
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate placeholder in tree",
        );
        // A docs-only commit outside the crate (no *.rs/*.bats changes).
        repo.write_and_commit("docs/README.md", "# README\n", "docs update outside crate");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(outcome, GateOutcome::PassZeroActivations),
            "expected PassZeroActivations, got: {:?}",
            outcome
        );
        assert_eq!(outcome.identifier(), "PASS-zero-activations");
        assert_eq!(outcome.exit_code(), 0);
        assert!(outcome.is_pass());
    }

    /// Empty-range control — crate in HEAD tree; `MERGE_BASE == HEAD` (no commits in range)
    /// → `SkippedEmptyRange` (exit 0, inert) per ADR-040 v1.19 Ruling 9(f).
    ///
    /// Updated from old behavior (EmptyOrUnreachable(EmptyRange), exit 2) — ADR-040 v1.19
    /// rules this an inert skip because there is no PR diff to evaluate.
    ///
    /// The crate placeholder must be committed FIRST so the stale-pin guard passes;
    /// HEAD is then captured AFTER the placeholder commit so `merge_base == HEAD`
    /// produces a genuinely empty range.
    #[test]
    fn test_empty_range() {
        let repo = Repo::new();

        // Seed the crate into the tree before capturing HEAD.
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );
        let head = repo.head_sha(); // captured AFTER seeding

        // MERGE_BASE == HEAD → git log head..HEAD returns zero commits → SkippedEmptyRange.
        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &head).expect("no gate error");
        assert!(
            matches!(outcome, GateOutcome::SkippedEmptyRange),
            "expected SkippedEmptyRange (ADR-040 v1.19 Ruling 9(f)), got: {:?}",
            outcome
        );
        assert_eq!(
            outcome.identifier(),
            "SKIP: empty or unresolvable commit range — inert (no PR diff to evaluate)"
        );
        assert_eq!(outcome.exit_code(), 0);
        assert!(outcome.is_pass());
    }

    /// Stale-pin control — crate path absent from HEAD tree → `EmptyOrUnreachable(StalePin)`.
    #[test]
    fn test_stale_pin() {
        let repo = Repo::new();
        let base = repo.head_sha();

        // Non-empty commit range, but nothing inside the pinned crate.
        repo.write_and_commit("docs/CHANGELOG.md", "# Changelog\n", "changelog only");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(
                outcome,
                GateOutcome::EmptyOrUnreachable(UnreachableCause::StalePin)
            ),
            "expected EmptyOrUnreachable(StalePin), got: {:?}",
            outcome
        );
        assert_eq!(outcome.identifier(), "EMPTY-or-UNREACHABLE: stale pin");
        assert_eq!(outcome.exit_code(), 2);
    }

    /// Unmeasurable-diff control — `--allow-empty` commit in range →
    /// `EmptyOrUnreachable(UnmeasurableDiff)`.
    #[test]
    fn test_unmeasurable_diff() {
        let repo = Repo::new();

        // Seed the crate so stale-pin guard passes.
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );
        let base = repo.head_sha();

        // An --allow-empty commit has a parent but an empty changed-file set.
        repo.git(&[
            "commit",
            "--allow-empty",
            "-m",
            "empty commit (allow-empty)",
        ]);

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(
                outcome,
                GateOutcome::EmptyOrUnreachable(UnreachableCause::UnmeasurableDiff { .. })
            ),
            "expected EmptyOrUnreachable(UnmeasurableDiff), got: {:?}",
            outcome
        );
        assert_eq!(
            outcome.identifier(),
            "EMPTY-or-UNREACHABLE: unmeasurable diff"
        );
        assert_eq!(outcome.exit_code(), 2);
    }

    /// Defect-3 regression test — crate directory created on disk but never committed
    /// → `EmptyOrUnreachable(StalePin)`, NOT `PassZeroActivations`.
    ///
    /// This is the key regression for ADR-040 §Decision 8: `is_dir()` on the filesystem
    /// returns `true`, but `git cat-file -e HEAD:<path>` returns NOT FOUND because the
    /// directory was never committed.  The gate must use the git-tree check.
    #[test]
    fn test_disk_present_tree_absent_is_stale_pin() {
        let repo = Repo::new();
        let base = repo.head_sha();

        // Create the crate directory on disk — but do NOT commit it.
        let crate_dir = repo.path().join(PC);
        fs::create_dir_all(&crate_dir).expect("create_dir_all");

        // A non-crate commit to give a non-empty range.
        repo.write_and_commit("docs/README.md", "# README\n", "docs only, no crate commit");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(
                outcome,
                GateOutcome::EmptyOrUnreachable(UnreachableCause::StalePin)
            ),
            "expected StalePin (crate on disk but not in git tree), got: {:?}",
            outcome
        );
    }

    /// Unresolvable-base test — bogus base branch ref → `SkippedEmptyRange` (exit 0, inert)
    /// per ADR-040 v1.19 Ruling 9(f).
    ///
    /// Updated from old behavior (EmptyOrUnreachable(EmptyRange), exit 2, "fail closed") —
    /// ADR-040 v1.19 rules that when `git merge-base HEAD origin/<branch>` fails (e.g.,
    /// no `origin` remote, or branch not found), the gate cannot evaluate any PR diff and
    /// must exit 0 (inert skip), not block CI.
    ///
    /// Guard ordering is preserved: `StalePin` still wins when the crate is ALSO absent
    /// (see `test_run_gate_guard1_stale_pin_beats_unresolvable_base` — UNCHANGED).
    ///
    /// Assert the specific variant, not merely is_pass(): a coarse boolean check would
    /// survive a mutation that changed the outcome identifier (level-5 defect).
    #[test]
    fn test_unresolvable_base_is_skipped_inert() {
        let repo = Repo::new();

        // Seed the crate so guard 1 passes (we reach the merge-base step).
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );

        // No `origin` remote — merge-base fails → SkippedEmptyRange (inert, exit 0).
        let GateResult { outcome, .. } =
            run_gate(repo.path(), "nonexistent-branch-xyz").expect("no hard I/O error");
        assert!(
            matches!(outcome, GateOutcome::SkippedEmptyRange),
            "expected SkippedEmptyRange for unresolvable base (ADR-040 v1.19 Ruling 9(f)), got: {:?}",
            outcome
        );
        assert_eq!(
            outcome.identifier(),
            "SKIP: empty or unresolvable commit range — inert (no PR diff to evaluate)"
        );
        assert_eq!(outcome.exit_code(), 0);
        assert!(outcome.is_pass());
    }

    /// Guard-ordering test — crate absent AND range empty → `StalePin` wins over `SkippedEmptyRange`.
    ///
    /// ADR-040 §Decision 8/9: the stale-pin guard runs BEFORE the commit-range check.
    /// When both preconditions fail, `StalePin` must be the reported cause.
    #[test]
    fn test_guard_ordering_stale_pin_beats_empty_range() {
        let repo = Repo::new();
        // HEAD is the initial empty commit; crate was never committed.
        let head = repo.head_sha(); // MERGE_BASE == HEAD → would be empty range

        // Neither condition holds: crate absent (→ StalePin) AND range empty (→ SkippedEmptyRange).
        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &head).expect("no gate error");
        assert!(
            matches!(
                outcome,
                GateOutcome::EmptyOrUnreachable(UnreachableCause::StalePin)
            ),
            "expected StalePin to win over SkippedEmptyRange, got: {:?}",
            outcome
        );
    }

    /// Guard-1 ordering test (through `run_gate`) — crate absent AND base unresolvable
    /// → `StalePin`, not `SkippedEmptyRange`.
    ///
    /// This is the mutation-killing companion to `test_guard_ordering_stale_pin_beats_empty_range`.
    /// That test exercises guard ordering through `run_gate_from_merge_base` (guard 2).
    /// This test exercises it through `run_gate` (guard 1), which fires before the
    /// `git merge-base HEAD origin/<branch>` call. Neutralising guard 1 would cause
    /// the merge-base failure to be reached first, returning `SkippedEmptyRange`
    /// (inert, exit 0 per ADR-040 v1.19) rather than `StalePin` — failing this assertion
    /// and silently masking a stale-pin CI setup defect with a false-pass.
    #[test]
    fn test_run_gate_guard1_stale_pin_beats_unresolvable_base() {
        let repo = Repo::new();
        // Crate is NOT seeded — absent from HEAD tree (StalePin trigger).
        // No origin remote — merge-base would fail (SkippedEmptyRange per ADR-040 v1.19).
        // Guard 1 must fire first → outcome must be StalePin, not SkippedEmptyRange.
        let GateResult { outcome, .. } =
            run_gate(repo.path(), "nonexistent-branch-xyz").expect("no hard I/O error");
        assert!(
            matches!(
                outcome,
                GateOutcome::EmptyOrUnreachable(UnreachableCause::StalePin)
            ),
            "expected StalePin (guard 1 fires before merge-base), got: {:?}",
            outcome
        );
    }

    // ── Additional edge-case tests ────────────────────────────────────────────

    /// A `.bats` file in the crate also activates the gate.
    #[test]
    fn test_bats_file_activates_gate() {
        let repo = Repo::new();
        let base = repo.head_sha();
        let parent_sha = base.clone();

        repo.write(
            &format!("{PC}/tests/it.bats"),
            "#!/usr/bin/env bats\n@test 'x' { true; }\n",
        );
        repo.write(
            LOG,
            &format!(
                "# Red Gate Log\n\n### Pass-1 assertion-site attestation ({parent_sha})\n\nAttestation.\n"
            ),
        );
        repo.commit("add .bats assertion + attestation");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(outcome, GateOutcome::PassWithActivations(1)),
            "expected PassWithActivations(1) for .bats activation, got: {:?}",
            outcome
        );
    }

    /// A `.rs` file *outside* the pinned crate does not activate the gate.
    #[test]
    fn test_rs_outside_crate_does_not_activate() {
        let repo = Repo::new();
        // Seed the crate so stale-pin passes.
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate placeholder",
        );
        let base = repo.head_sha();

        // A .rs file in a completely different crate.
        repo.write_and_commit(
            "crates/some-other-crate/src/lib.rs",
            "// other crate\n",
            "add other crate .rs file",
        );

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(outcome, GateOutcome::PassZeroActivations),
            "expected PassZeroActivations (.rs outside crate), got: {:?}",
            outcome
        );
    }

    /// Prose quoting a prior attestation SHA does not cause count > 1 (line-anchor test).
    ///
    /// This verifies the fix for ADR-040 §Decision 6 Ruling 6(b): the grep predicate
    /// is anchored to `^### ` so prose lines that merely contain the SHA do not increment
    /// the count.
    #[test]
    fn test_prose_sha_does_not_cause_false_count() {
        let repo = Repo::new();
        let base = repo.head_sha();
        let parent_sha = base.clone();

        repo.write(&format!("{PC}/src/lib.rs"), "// assertion site\n");
        // Log contains the correct heading PLUS a prose paragraph quoting the same SHA.
        repo.write(
            LOG,
            &format!(
                "# Red Gate Log\n\n### Pass-1 assertion-site attestation ({parent_sha})\n\nSee also `assertion-site attestation ({parent_sha})` in a prior pass.\n"
            ),
        );
        repo.commit("assertion site + attestation + prose quote");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        // Should be PASS-1-activations, not FAIL due to count > 1.
        assert!(
            matches!(outcome, GateOutcome::PassWithActivations(1)),
            "expected PassWithActivations(1) (prose quote must not add to count), got: {:?}",
            outcome
        );
    }

    /// Multiple activating commits, all compliant → `PassWithActivations(2)`.
    #[test]
    fn test_multiple_activations_all_compliant() {
        let repo = Repo::new();
        let base = repo.head_sha();

        // First activating commit.
        let parent1 = base.clone();
        repo.write(&format!("{PC}/src/a.rs"), "// a\n");
        repo.write(
            LOG,
            &format!("# Red Gate Log\n\n### Pass-1 assertion-site attestation ({parent1})\n"),
        );
        repo.commit("commit 1: a.rs + attestation 1");

        // Second activating commit.
        let parent2 = repo.head_sha();
        repo.write(&format!("{PC}/src/b.rs"), "// b\n");
        // Overwrite log with both attestation headings.
        repo.write(
            LOG,
            &format!(
                "# Red Gate Log\n\n### Pass-1 assertion-site attestation ({parent1})\n\n### Pass-2 assertion-site attestation ({parent2})\n"
            ),
        );
        repo.commit("commit 2: b.rs + attestation 2");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(outcome, GateOutcome::PassWithActivations(2)),
            "expected PassWithActivations(2), got: {:?}",
            outcome
        );
    }

    // ── Coverage-gap fix-pass tests (pre-merge mutant-survival closure) ────────
    //
    // The four groups below (F-1, F-2, F-5, F-6) close per-guard mutant-survival
    // gaps identified in pre-merge validation. F-1's WARNING assertion is delivered
    // and GREEN (ADR-040 §Decision 9 Ruling 9(c) — see `tests/binary_integration_test.rs`).
    // F-2 and F-5 below are GREEN.

    /// F-2 — exactly-once upper bound (`count != 1` upper arm, lib.rs `run_gate_inner`).
    ///
    /// Existing controls (`test_positive_2_no_attestation_heading`,
    /// `test_negative_compliant_attestation`) only exercise `count == 0` and
    /// `count == 1`. This test drives `count == 2`: a single activating commit
    /// whose pinned `red-gate-log.md` contains TWO
    /// `### <Pass-N> assertion-site attestation (<PARENT-SHA>)` headings for the
    /// SAME parent SHA. The obligation is "exactly one", not "at least one", so
    /// this must still fail — with `AttestationAmbiguous { count: 2 }` (ADR-040
    /// §Decision 8 Ruling 8(b) amendment, v1.18: absence and ambiguity are distinct
    /// `FailReason` variants; the `count != 1` FAIL boundary itself is unchanged). A
    /// plain mutation run on `count != 1` (e.g. a mutant that flips the comparison to
    /// `count == 0`) cannot be generated from existing fixtures alone — this
    /// hand-authored control is required to kill it.
    ///
    /// Expected: GREEN.
    #[test]
    fn test_f2_two_attestation_headings_same_parent_fails() {
        let repo = Repo::new();
        let base = repo.head_sha();
        let parent_sha = base.clone();

        repo.write(&format!("{PC}/src/lib.rs"), "// assertion site\n");
        // TWO headings for the SAME parent SHA within the SAME commit's log.
        repo.write(
            LOG,
            &format!(
                "# Red Gate Log\n\n\
                 ### Pass-1 assertion-site attestation ({parent_sha})\n\n\
                 Attestation body 1.\n\n\
                 ### Pass-2 assertion-site attestation ({parent_sha})\n\n\
                 Attestation body 2 (duplicate heading for the same parent SHA).\n"
            ),
        );
        repo.commit("assertion site + duplicate attestation headings for same parent");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");
        assert!(
            matches!(
                &outcome,
                GateOutcome::Fail(v) if v.len() == 1 && v[0].reason == FailReason::AttestationAmbiguous { count: 2 }
            ),
            "expected Fail(AttestationAmbiguous {{ count: 2 }}) for count==2 (exactly-once upper bound), got: {:?}",
            outcome
        );
        assert_eq!(outcome.identifier(), "FAIL: obligation violated");
        assert_eq!(outcome.exit_code(), 2);
    }

    /// F-5 — `git_merge_base` SUCCESS path, driven through `run_gate()`.
    ///
    /// Existing `run_gate()`-level controls (`test_unresolvable_base_is_skipped_inert`,
    /// `test_run_gate_guard1_stale_pin_beats_unresolvable_base`) only exercise the
    /// ERROR arm of `git_merge_base` (no `origin` remote → merge-base command
    /// fails → `SkippedEmptyRange` per ADR-040 v1.19 Ruling 9(f)). Neither test proves
    /// the SUCCESS arm (`Ok(sha)`) is wired correctly.
    ///
    /// This test fakes a `refs/remotes/origin/<branch>` ref via `git update-ref`
    /// — no real `origin` remote or network access required; `git merge-base`
    /// cannot distinguish a synthesized remote-tracking ref from a fetched one —
    /// so `git merge-base HEAD origin/develop` genuinely succeeds. The downstream
    /// outcome (`Fail(LogAbsent)`) is reachable ONLY if the merge-base SHA was
    /// correctly threaded into `run_gate_inner`; the ERROR arm would instead
    /// short-circuit to `SkippedEmptyRange` (exit 0) before any commit is
    /// ever inspected.
    ///
    /// Expected: GREEN.
    #[test]
    fn test_f5_run_gate_resolves_real_merge_base_success_path() {
        let repo = Repo::new();

        // Seed the crate so guard 1 (stale-pin) passes.
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );
        let base = repo.head_sha();

        // Fake the remote-tracking ref WITHOUT a real `origin` remote.
        repo.git(&["update-ref", "refs/remotes/origin/develop", &base]);

        // Activating commit with no red-gate-log.md → Fail(LogAbsent). This
        // outcome is reachable only via the merge-base SUCCESS arm: if
        // `git_merge_base` had errored, the gate would report
        // SkippedEmptyRange (exit 0, inert per ADR-040 v1.19) instead, before
        // ever inspecting commits.
        repo.write_and_commit(
            &format!("{PC}/src/lib.rs"),
            "// assertion site\n",
            "activate, no log (drives merge-base success path)",
        );

        let GateResult { outcome, .. } = run_gate(repo.path(), "develop").expect("no gate error");
        assert!(
            matches!(
                &outcome,
                GateOutcome::Fail(v) if v.len() == 1 && v[0].reason == FailReason::LogAbsent
            ),
            "expected Fail(LogAbsent) via resolved real merge-base (success path), got: {:?}",
            outcome
        );
        assert_eq!(outcome.identifier(), "FAIL: obligation violated");
        assert_eq!(outcome.exit_code(), 2);
    }

    // ── v1.18 formal regression controls (PR #777 review: CR-2, H-1, H-2) ──────

    /// CR-2 — structural (not stderr-substring) proof that a parentless/root commit
    /// placed INSIDE a non-empty evaluated range is recorded in
    /// `GateResult.skipped_parentless`, not merely inferred from a WARNING string.
    ///
    /// This is the still-outstanding v1.17 test directive
    /// (`test_root_commit_is_skipped_not_activating`), delivered per ADR-040 §Decision
    /// 9 Ruling 9(c) item 4 + v1.18 CR-2 adjudication (`.factory/specs/architecture/
    /// decisions/ADR-040-...md`, "v1.18 replacement routing — test-writer" item 1). The
    /// companion binary-boundary stderr-substring test
    /// (`test_f1_parentless_commit_in_range_is_skipped_with_warning` in
    /// `tests/binary_integration_test.rs`) is UNCHANGED and remains the CLI-contract
    /// proof; this test is the library-level structural proof CR-2 asked for. A mutant
    /// that drops the `skipped_parentless.push(...)` call, pushes the wrong SHA, or
    /// pushes a duplicate now fails this `assert_eq!` regardless of any message-text
    /// formatting.
    ///
    /// Expected: GREEN.
    #[test]
    fn test_root_commit_is_skipped_not_activating() {
        let repo = Repo::new();
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );
        let base = repo.head_sha();

        let root_sha = repo.create_range_containing_parentless_commit();

        let GateResult {
            outcome,
            skipped_parentless,
            ..
        } = run_gate_from_merge_base(repo.path(), &base).expect("no gate error");

        assert_eq!(
            skipped_parentless,
            vec![root_sha.clone()],
            "expected skipped_parentless to contain exactly the root commit's SHA"
        );
        assert_eq!(
            outcome,
            GateOutcome::PassZeroActivations,
            "the parentless-commit skip must be harmless to the verdict"
        );
    }

    /// H-1 control 1 (false-FAIL-gone) — a real, non-fast-forward merge commit whose
    /// FIRST-PARENT (two-dot) diff would re-flag an already-attested crate change
    /// pulled in from the second parent, but whose COMBINED diff is INERT for the
    /// crate (the file matches the parent that introduced it). Mirrors PR #777's own
    /// `010e6140` shape (ADR-040 §Decision 9 Ruling 9(e), H-1 pr-reviewer finding).
    ///
    /// Fixture: a `feature` lineage adds a pinned-crate `.rs` file WITH a compliant
    /// attestation (mirrors `test_negative_compliant_attestation` — "already attested
    /// elsewhere"); `main` independently diverges with an unrelated docs commit so the
    /// merge cannot fast-forward; a real `git merge --no-ff` brings `feature` back onto
    /// `main`. Pre-fix (two-dot endpoint diff on the merge commit), the crate file
    /// would appear "new" at the merge and false-FAIL for missing an attestation keyed
    /// to the merge's OWN first parent. Post-fix, the combined diff excludes it (it
    /// matches the `feature` parent) and the merge commit is recorded as
    /// `skipped_merge_inert`.
    ///
    /// Asserts: `result.outcome` is NOT `Fail` AND the merge commit's SHA is in
    /// `result.skipped_merge_inert`.
    ///
    /// Expected: GREEN.
    #[test]
    fn test_h1_merge_pass_through_content_is_skipped_not_failed() {
        let repo = Repo::new();
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );
        let base = repo.head_sha();
        let main_branch = repo.git_stdout(&["rev-parse", "--abbrev-ref", "HEAD"]);
        let attestation_parent = base.clone();

        // `feature` lineage: adds a pinned-crate `.rs` file WITH a compliant
        // attestation — this is the "already attested on its own originating branch"
        // content Ruling 9(e) says must not be re-flagged at the merge.
        repo.git(&["checkout", "-b", "feature"]);
        repo.write(&format!("{PC}/src/lib.rs"), "// assertion site (feature)\n");
        repo.write(
            LOG,
            &format!(
                "# Red Gate Log\n\n### Pass-1 assertion-site attestation ({attestation_parent})\n\nAttestation body.\n"
            ),
        );
        repo.commit("feature: assertion site + compliant attestation");

        // `main` diverges independently (unrelated docs change) so the upcoming merge
        // cannot fast-forward and must produce a genuine two-parent merge commit.
        repo.git(&["checkout", &main_branch]);
        repo.write_and_commit(
            "docs/unrelated.md",
            "unrelated\n",
            "main: unrelated docs change",
        );

        let merge_out = repo.git(&[
            "merge",
            "--no-ff",
            "-m",
            "merge feature into main (sync)",
            "feature",
        ]);
        assert!(
            merge_out.status.success(),
            "expected a clean, non-conflicting merge: {}",
            String::from_utf8_lossy(&merge_out.stderr)
        );
        let merge_sha = repo.head_sha();

        let GateResult {
            outcome,
            skipped_merge_inert,
            ..
        } = run_gate_from_merge_base(repo.path(), &base).expect("no gate error");

        assert!(
            !matches!(&outcome, GateOutcome::Fail(_)),
            "H-1 false-FAIL regression: a merge pulling in already-attested content \
             must not FAIL, got: {:?}",
            outcome
        );
        assert!(
            skipped_merge_inert.contains(&merge_sha),
            "expected the merge commit {merge_sha} in skipped_merge_inert (combined \
             diff must be empty for pass-through crate content), got: {:?}",
            skipped_merge_inert
        );
    }

    /// H-1 control 2 (bypass-stays-closed) — a merge commit whose COMBINED diff
    /// genuinely contains merge-authored crate content (introduced ONLY at the merge
    /// point, nowhere else in the DAG — the exact bypass H-1 asked to be closed,
    /// ADR-040 §Decision 9 Ruling 9(e) "Closing the bypass" paragraph), with NO
    /// attestation heading for the merge commit's first parent.
    ///
    /// Fixture: two independent lineages (`parent1`, `parent2`) off `base`, NEITHER of
    /// which touches the pinned crate (so neither generates its own, unrelated Fail).
    /// A merge commit is fabricated directly via `commit-tree` plumbing (same
    /// technique `create_range_containing_parentless_commit` uses) whose tree adds a
    /// pinned-crate `.rs` file present in NEITHER parent — content that, by
    /// definition, is exactly what git's combined diff surfaces as merge-authored. No
    /// `red-gate-log.md` is ever committed, so the attestation lookup keyed on the
    /// merge commit's first parent (`parent1`) finds nothing.
    ///
    /// This proves the H-1 skip does not open a hole: attestation is still enforced on
    /// genuinely merge-authored content.
    ///
    /// Asserts: `result.outcome` IS `Fail(v)` with exactly one entry, whose `commit`
    /// is the merge commit's SHA and whose `reason` is `FailReason::LogAbsent`.
    ///
    /// Expected: GREEN.
    #[test]
    fn test_h1_merge_authored_content_with_no_attestation_still_fails() {
        let repo = Repo::new();
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );
        let base = repo.head_sha();
        let main_branch = repo.git_stdout(&["rev-parse", "--abbrev-ref", "HEAD"]);

        // Parent 1: unrelated docs change off `base` — does NOT touch the pinned
        // crate, so it carries no obligation of its own.
        repo.write_and_commit("docs/p1.md", "p1\n", "parent1: unrelated docs change");
        let parent1 = repo.head_sha();

        // Parent 2: a second, independent lineage off `base` — also does NOT touch
        // the pinned crate.
        repo.git(&["checkout", "-b", "lineage-p2", &base]);
        repo.write_and_commit("docs/p2.md", "p2\n", "parent2: unrelated docs change");
        let parent2 = repo.head_sha();

        // Back to `main_branch`'s tip (== parent1) so the working tree/index reflect
        // parent1's committed state before staging the merge-only content.
        repo.git(&["checkout", &main_branch]);

        // Fabricate a merge commit whose tree adds a pinned-crate `.rs` file that
        // exists NOWHERE else in the DAG — content introduced ONLY at the merge point
        // itself, mirroring a conflict-resolution edit authored during the merge.
        repo.write(
            &format!("{PC}/src/merge_authored.rs"),
            "// authored only at the merge commit, no attestation\n",
        );
        repo.git(&["add", "-A"]);
        let merge_tree = repo.git_stdout(&["write-tree"]);
        let merge_sha = repo.git_stdout(&[
            "commit-tree",
            &merge_tree,
            "-p",
            &parent1,
            "-p",
            &parent2,
            "-m",
            "merge: introduce crate .rs only at the merge point (no attestation)",
        ]);
        // Advance the branch ref to the fabricated merge commit. No separate checkout
        // is needed: the index/working tree already match `merge_tree`, since that is
        // exactly what was just staged on top of `parent1`'s checkout.
        repo.git(&[
            "update-ref",
            &format!("refs/heads/{main_branch}"),
            &merge_sha,
        ]);
        assert_eq!(
            repo.head_sha(),
            merge_sha,
            "fixture bug: HEAD did not advance"
        );

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");

        assert!(
            matches!(
                &outcome,
                GateOutcome::Fail(v) if v.len() == 1
                    && v[0].commit == merge_sha
                    && v[0].reason == FailReason::LogAbsent
            ),
            "expected Fail(LogAbsent) for the sole merge-authored, unattested commit \
             {merge_sha}, got: {:?}",
            outcome
        );
    }

    /// H-2 control — a non-ASCII path component under the pinned crate must still
    /// activate the gate. Regression for `git diff --name-only`'s default
    /// `core.quotePath=true`, which C-quotes (surrounds with `"` + octal escapes) any
    /// path containing non-ASCII bytes — breaking both the `starts_with(&crate_prefix)`
    /// and `ends_with(".rs")` predicates and silently producing `PassZeroActivations`
    /// for a genuinely unattested crate change (pr-reviewer H-2, PR #777).
    ///
    /// This test FAILS against the pre-fix code (which invoked plain
    /// `git diff --name-only` without `-c core.quotePath=false`) and PASSES now that
    /// `git_diff_name_only` pins `core.quotePath=false`.
    ///
    /// Expected: GREEN.
    #[test]
    fn test_h2_non_ascii_path_activates_gate() {
        let repo = Repo::new();
        let base = repo.head_sha();
        let parent_sha = base.clone();

        let non_ascii_path = format!("{PC}/src/café.rs");
        repo.write(&non_ascii_path, "// non-ASCII filename assertion site\n");
        repo.write(
            LOG,
            &format!(
                "# Red Gate Log\n\n### Pass-1 assertion-site attestation ({parent_sha})\n\nAttestation body.\n"
            ),
        );
        repo.commit("add non-ASCII-named .rs file + compliant attestation");

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &base).expect("no gate error");

        assert!(
            matches!(outcome, GateOutcome::PassWithActivations(1)),
            "H-2 regression: a non-ASCII activating path must be detected (not \
             silently PASS-zero-activations due to core.quotePath C-quoting), got: {:?}",
            outcome
        );
    }

    /// `GateOutcome::identifier` produces strings that are greppable — spot-check.
    #[test]
    fn test_identifier_strings_are_greppable() {
        assert!(
            GateOutcome::PassWithActivations(3)
                .identifier()
                .contains("PASS-3-activations")
        );
        assert!(
            GateOutcome::PassZeroActivations
                .identifier()
                .contains("PASS-zero-activations")
        );
        assert!(GateOutcome::Fail(vec![]).identifier().contains("FAIL"));
        assert!(
            GateOutcome::EmptyOrUnreachable(UnreachableCause::StalePin)
                .identifier()
                .contains("stale pin")
        );
        // ADR-040 v1.19 Ruling 9(f): empty/unresolvable range is now SkippedEmptyRange.
        // The old EmptyOrUnreachable(EmptyRange) variant has been retired; its callsites
        // now emit SkippedEmptyRange with a SKIP-prefixed identifier (exit 0, inert).
        assert!(
            GateOutcome::SkippedEmptyRange
                .identifier()
                .contains("SKIP:")
        );
        assert!(
            GateOutcome::SkippedEmptyRange
                .identifier()
                .contains("inert")
        );
        assert!(
            GateOutcome::EmptyOrUnreachable(UnreachableCause::UnmeasurableDiff {
                commit: "abc".into()
            })
            .identifier()
            .contains("unmeasurable diff")
        );
    }

    // ── ADR-040 v1.19 Ruling 9(f) — SkippedEmptyRange inert-skip tests ───────

    /// ADR-040 v1.19 Ruling 9(f): empty commit range → `SkippedEmptyRange` (exit 0, inert).
    ///
    /// Regression test for the defect: when `merge_base == HEAD` (no commits in range),
    /// the gate previously returned `GateOutcome::EmptyOrUnreachable(UnreachableCause::EmptyRange)`
    /// (exit 2), false-failing every post-merge push to develop. ADR-040 v1.19 rules
    /// this case INERT SKIP — the gate cannot evaluate any PR diff, so it must exit 0.
    ///
    /// This is the PRIMARY RED-GATE test for the fix (written first, run red, then green).
    ///
    /// Expected: GREEN after implementing `GateOutcome::SkippedEmptyRange`.
    #[test]
    fn test_adr040_v119_empty_range_is_skipped_inert() {
        let repo = Repo::new();

        // Seed the crate so stale-pin guard passes (guard fires before range check).
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );
        let head = repo.head_sha(); // MERGE_BASE == HEAD → empty range

        let GateResult { outcome, .. } =
            run_gate_from_merge_base(repo.path(), &head).expect("no gate error");
        assert!(
            matches!(outcome, GateOutcome::SkippedEmptyRange),
            "ADR-040 v1.19 Ruling 9(f): empty range must return SkippedEmptyRange (exit 0), got: {:?}",
            outcome
        );
        assert_eq!(
            outcome.identifier(),
            "SKIP: empty or unresolvable commit range — inert (no PR diff to evaluate)"
        );
        assert_eq!(outcome.exit_code(), 0);
        assert!(
            outcome.is_pass(),
            "SkippedEmptyRange must be a pass (is_pass() == true)"
        );
    }

    /// ADR-040 v1.19 Ruling 9(f): unresolvable base → `SkippedEmptyRange` (exit 0, inert).
    ///
    /// When `git merge-base HEAD origin/<branch>` fails (no `origin` remote, branch not
    /// found), the gate can evaluate no PR diff. ADR-040 v1.19 rules this INERT SKIP
    /// (exit 0), not a hard fail-closed (exit 2).
    ///
    /// Guard ordering is preserved: `StalePin` still wins when the crate is ALSO absent
    /// (see `test_run_gate_guard1_stale_pin_beats_unresolvable_base` — UNCHANGED).
    ///
    /// Expected: GREEN after implementing `GateOutcome::SkippedEmptyRange`.
    #[test]
    fn test_adr040_v119_unresolvable_base_is_skipped_inert() {
        let repo = Repo::new();

        // Seed the crate so guard 1 (stale-pin) passes — we want to reach the merge-base step.
        repo.write_and_commit(
            &format!("{PC}/docs/placeholder.md"),
            "placeholder\n",
            "seed: crate in HEAD tree",
        );

        // No `origin` remote — merge-base fails → SkippedEmptyRange (inert, exit 0).
        let GateResult { outcome, .. } =
            run_gate(repo.path(), "nonexistent-branch-xyz").expect("no hard I/O error");
        assert!(
            matches!(outcome, GateOutcome::SkippedEmptyRange),
            "ADR-040 v1.19 Ruling 9(f): unresolvable base must return SkippedEmptyRange (exit 0), got: {:?}",
            outcome
        );
        assert_eq!(
            outcome.identifier(),
            "SKIP: empty or unresolvable commit range — inert (no PR diff to evaluate)"
        );
        assert_eq!(outcome.exit_code(), 0);
        assert!(
            outcome.is_pass(),
            "SkippedEmptyRange must be a pass (is_pass() == true)"
        );
    }
}
