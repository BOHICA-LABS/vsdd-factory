// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! F-6 (and F-1) — binary-integration coverage for `src/main.rs`.
//!
//! Prior to this fix-pass, `src/main.rs` had ZERO tests: every outcome-to-
//! exit-code mapping, the identifier printed to stdout, the per-outcome
//! detail lines printed to stderr, and the `>= 12` SHA-truncation formatting
//! logic were entirely unexercised (5/5 binary mutants missed in pre-merge
//! mutation validation). These tests invoke the COMPILED
//! `policy15-attestation-gate` binary (via `CARGO_BIN_EXE_policy15-attestation-gate`,
//! matching the existing workspace pattern in
//! `crates/factory-dispatcher/tests/bc_3_08_001_s19_05.rs`) against real
//! scratch git repositories so the actual `main()` control flow — not a
//! library replica — is what gets exercised.
//!
//! No new dev-dependency was added: `std::process::Command` +
//! `env!("CARGO_BIN_EXE_policy15-attestation-gate")` and the crate's existing
//! `tempfile` dev-dependency are sufficient.
//!
//! ## Fake `origin/<branch>` ref technique
//!
//! `run_gate()` (called from `main()`) computes
//! `git merge-base HEAD origin/<base_branch>`. Git cannot distinguish a
//! remote-tracking ref created locally via `git update-ref
//! refs/remotes/origin/<branch> <sha>` from one populated by a real `git
//! fetch` — so these fixtures never need an actual `origin` remote or network
//! access; they synthesize the ref directly.
//!
//! ## Parentless-commit-in-range technique (F-1)
//!
//! To place a genuinely parentless commit INSIDE a non-empty evaluated range
//! (not just as the range's exclusive lower bound, which every existing test
//! already covers implicitly via the initial commit), a second, historically
//! UNRELATED root commit is synthesized with `git commit-tree <tree> -m ...`
//! (no `-p` parent flag). A disconnected root cannot simply replace the
//! branch tip, though: `git merge-base HEAD origin/<branch>` — which `main()`
//! computes for real — would then have NO common ancestor at all, returning
//! `SkippedEmptyRange` (exit 0, inert per ADR-040 v1.19 Ruling 9(f)) before
//! any commit is even inspected, never reaching the skip guard under test.
//!
//! Instead the unrelated root chain is merged back into the seeded branch
//! with `git merge --allow-unrelated-histories`, producing a merge commit
//! whose FIRST parent is the seeded branch tip (so `git merge-base` still
//! resolves normally to the fake `origin/<branch>` ref) while its history
//! also reaches back through the disconnected root commit. `git log
//! <base>..<merge-commit>` (exactly what `run_gate_inner` computes) then
//! contains the merge commit, the unrelated chain's tip, AND the parentless
//! root commit — placing it genuinely inside a non-empty range while
//! `git merge-base` keeps resolving successfully end-to-end through the real
//! binary. See `Repo::create_range_containing_parentless_commit`.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// `CARGO_BIN_EXE_policy15-attestation-gate` is set by Cargo for integration
/// tests in this crate's `tests/` directory (Cargo builds the crate's own
/// `[[bin]]` targets before running integration tests).
fn binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_policy15-attestation-gate"))
}

// ── Fixture helper (self-contained duplicate of `src/lib.rs`'s private
//    `#[cfg(test)]` `Repo` — integration test files are a separate
//    compilation unit and cannot reach that private module). ────────────────

struct Repo {
    dir: TempDir,
}

impl Repo {
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

    fn head_sha(&self) -> String {
        self.git_stdout(&["rev-parse", "HEAD"])
    }

    fn write(&self, rel_path: &str, content: &str) {
        let full = self.dir.path().join(rel_path);
        if let Some(parent) = full.parent() {
            std::fs::create_dir_all(parent).expect("create_dir_all");
        }
        std::fs::write(&full, content).expect("fs::write");
    }

    fn commit(&self, message: &str) {
        self.git(&["add", "."]);
        let out = self.git(&["commit", "-m", message]);
        assert!(
            out.status.success(),
            "git commit failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    }

    fn write_and_commit(&self, rel_path: &str, content: &str, message: &str) {
        self.write(rel_path, content);
        self.commit(message);
    }

    /// Fake a remote-tracking ref for `origin/<branch>` pointing at `sha`,
    /// without a real `origin` remote (see module doc).
    fn fake_origin_ref(&self, branch: &str, sha: &str) {
        self.git(&["update-ref", &format!("refs/remotes/origin/{branch}"), sha]);
    }

    /// Run the compiled `policy15-attestation-gate` binary with this repo as
    /// its current directory. Returns (exit_code, stdout, stderr).
    fn run_binary(&self, base_branch: &str) -> (i32, String, String) {
        let output = Command::new(binary_path())
            .current_dir(self.path())
            .arg(base_branch)
            .output()
            .expect("spawn policy15-attestation-gate binary");
        (
            output.status.code().expect("process terminated by signal"),
            String::from_utf8_lossy(&output.stdout).into_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    }

    /// Build a genuinely parentless ("root-like") commit and merge it — via
    /// `git merge --allow-unrelated-histories` — back onto the tip of the
    /// current branch, so that:
    ///
    /// - `git merge-base <old-tip> <new-tip>` still resolves to `<old-tip>`
    ///   (it is the merge commit's FIRST parent), keeping the real
    ///   `git merge-base HEAD origin/<branch>` computation in `main()` on the
    ///   SUCCESS path; and
    /// - `git log <old-tip>..<new-tip>` (what `run_gate_inner` evaluates)
    ///   nonetheless contains the parentless root commit, because it is
    ///   reachable from the new tip via the merge's SECOND parent and is not
    ///   reachable from `<old-tip>` at all.
    ///
    /// The only file-level change introduced relative to `<old-tip>` is a
    /// single new file outside the pinned crate (non-activating), so the
    /// only way the fixture can affect the gate's outcome is through the
    /// skip-guard behavior under test.
    ///
    /// Returns the new parentless root commit's full SHA. Leaves the current
    /// branch at the new merge commit.
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

        // 2. A child of the root commit adding one new file (built directly
        //    via plumbing, so the current branch/index/working tree are
        //    never touched by this step).
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

        // 3. Restore the index to match the current branch tip (undo the
        //    `git add` from step 2) and remove the working-tree file so the
        //    upcoming merge starts from a clean, matching state.
        let reset = self.git(&["reset", "--mixed", "HEAD"]);
        assert!(reset.status.success());
        std::fs::remove_dir_all(self.path().join("unrelated")).expect("remove_dir_all");

        // 4. Merge the unrelated chain onto the current branch tip. Because
        //    `git merge` runs from the CURRENT branch, the resulting commit's
        //    first parent is the pre-merge tip and its second parent is
        //    `child_commit` (whose own parent is the parentless root commit).
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

const PC: &str = "crates/hook-plugins/validate-cross-site-correspondence";
const LOG: &str = "crates/hook-plugins/validate-cross-site-correspondence/docs/red-gate-log.md";

// ── F-6 — binary integration tests ──────────────────────────────────────────

/// F-6 branch 1 — FAIL outcome via the compiled binary: exit code 2,
/// `FAIL: obligation violated` on stdout, and the per-commit detail line on
/// stderr. Also covers the `>= 12` SHA-truncation formatting bound for the
/// `FailedCommit` branch of `main.rs`'s match arm: asserts the printed short
/// SHA is exactly the first 12 characters of the real (40-char) commit SHA —
/// this kills operator-swap / negation mutants of the `>= 12` guard (a
/// `< 12` or `== 12` mutant would take the "print full commit" else-arm
/// instead, producing a 40-character string that fails this equality).
///
/// Expected: GREEN.
#[test]
fn test_f6_binary_fail_exit_2_with_sha_truncation() {
    let repo = Repo::new();
    repo.write_and_commit(
        &format!("{PC}/docs/placeholder.md"),
        "placeholder\n",
        "seed: crate in HEAD tree",
    );
    let base = repo.head_sha();
    repo.fake_origin_ref("develop", &base);

    repo.write_and_commit(
        &format!("{PC}/src/lib.rs"),
        "// assertion site\n",
        "activate, no log",
    );
    let activating_sha = repo.head_sha();
    assert_eq!(activating_sha.len(), 40, "fixture sanity: full SHA");

    let (exit_code, stdout, stderr) = repo.run_binary("develop");

    assert_eq!(exit_code, 2, "FAIL must exit 2; stderr: {stderr}");
    assert!(
        stdout.contains("FAIL: obligation violated"),
        "stdout missing FAIL identifier: {stdout}"
    );

    let expected_short = &activating_sha[..12];
    assert!(
        stderr.contains(expected_short),
        "stderr must contain the 12-char truncated SHA {expected_short:?}, got: {stderr}"
    );
    // The full 40-char SHA must NOT appear verbatim on the FAIL detail line —
    // proves truncation actually occurred (kills the "always print full SHA"
    // / negated-guard mutant class).
    assert!(
        !stderr.contains(&activating_sha),
        "stderr should contain the TRUNCATED sha only, not the full 40-char sha: {stderr}"
    );
}

/// F-6 branch 2 — `EmptyOrUnreachable(UnmeasurableDiff)` via the compiled
/// binary: exit code 2, an identifier DISTINCT from the FAIL identifier, and
/// the second `>= 12` SHA-truncation site in `main.rs` (the
/// `UnmeasurableDiff { commit }` match arm) exercised the same way as branch 1.
///
/// Expected: GREEN.
#[test]
fn test_f6_binary_empty_or_unreachable_distinct_identifier_with_sha_truncation() {
    let repo = Repo::new();
    repo.write_and_commit(
        &format!("{PC}/docs/placeholder.md"),
        "placeholder\n",
        "seed: crate in HEAD tree",
    );
    let base = repo.head_sha();
    repo.fake_origin_ref("develop", &base);

    // An --allow-empty commit has a parent but produces an empty changed-file
    // set → UnmeasurableDiff.
    let out = repo.git(&["commit", "--allow-empty", "-m", "empty commit"]);
    assert!(out.status.success());
    let empty_commit_sha = repo.head_sha();
    assert_eq!(empty_commit_sha.len(), 40, "fixture sanity: full SHA");

    let (exit_code, stdout, stderr) = repo.run_binary("develop");

    assert_eq!(
        exit_code, 2,
        "EmptyOrUnreachable must exit 2; stderr: {stderr}"
    );
    assert!(
        stdout.contains("EMPTY-or-UNREACHABLE: unmeasurable diff"),
        "stdout identifier mismatch: {stdout}"
    );
    // Distinct from the FAIL identifier asserted in the sibling test — proves
    // the four-outcome/two-exit-code disambiguation the crate's whole design
    // exists to guarantee (crate-doc "defect class 5") holds at the binary
    // boundary, not just the library boundary.
    assert!(!stdout.contains("FAIL: obligation violated"));

    let expected_short = &empty_commit_sha[..12];
    assert!(
        stderr.contains(expected_short),
        "stderr must contain the 12-char truncated SHA {expected_short:?}, got: {stderr}"
    );
    assert!(
        !stderr.contains(&empty_commit_sha),
        "stderr should contain the TRUNCATED sha only, not the full 40-char sha: {stderr}"
    );
}

/// F-6 branch 3 — `PassWithActivations` (PASS-N) via the compiled binary:
/// exit code 0, `PASS-1-activations` on stdout.
///
/// Expected: GREEN.
#[test]
fn test_f6_binary_pass_n_activations_exit_0() {
    let repo = Repo::new();
    repo.write_and_commit(
        &format!("{PC}/docs/placeholder.md"),
        "placeholder\n",
        "seed: crate in HEAD tree",
    );
    let base = repo.head_sha();
    repo.fake_origin_ref("develop", &base);

    let parent_sha = base.clone();
    repo.write(&format!("{PC}/src/lib.rs"), "// assertion site\n");
    repo.write(
        LOG,
        &format!(
            "# Red Gate Log\n\n### Pass-1 assertion-site attestation ({parent_sha})\n\nBody.\n"
        ),
    );
    repo.commit("assertion site + compliant attestation");

    let (exit_code, stdout, stderr) = repo.run_binary("develop");

    assert_eq!(exit_code, 0, "PASS must exit 0; stderr: {stderr}");
    assert!(
        stdout.contains("PASS-1-activations"),
        "stdout identifier mismatch: {stdout}"
    );
}

// ── F-3 — BASE_BRANCH env-vs-CLI-arg precedence regression ─────────────────

/// F-3 — an explicitly-passed CLI argument must win over an exported
/// `BASE_BRANCH` environment variable (conventional CLI-arg > env > default
/// precedence, via clap's native `env`-backed argument resolution).
///
/// Regression fixture: `origin/cli-branch` is faked to resolve to a real
/// commit (crate seeded, non-empty tree). `BASE_BRANCH` is exported pointing
/// at a branch name with NO faked `origin/` ref at all. If the CLI arg wins
/// (correct), `git merge-base HEAD origin/cli-branch` resolves and the gate
/// reaches `PassZeroActivations` (exit 0). If the env var incorrectly won
/// (the pre-fix bug — `std::env::var("BASE_BRANCH").unwrap_or(cli.base_branch)`
/// consulted env FIRST), `origin/env-branch-does-not-exist` fails to resolve
/// → `SkippedEmptyRange` (exit 0, inert per ADR-040 v1.19 Ruling 9(f)).
///
/// Expected: GREEN (post-fix).
#[test]
fn test_f3_cli_arg_wins_over_base_branch_env_var() {
    let repo = Repo::new();
    repo.write_and_commit(
        &format!("{PC}/docs/placeholder.md"),
        "placeholder\n",
        "seed: crate in HEAD tree",
    );
    let base = repo.head_sha();
    // Faked origin ref ONLY for the branch passed explicitly on the CLI.
    repo.fake_origin_ref("cli-branch", &base);
    // A non-crate commit past `base` so the evaluated range is non-empty
    // (otherwise `merge_base == HEAD` would yield SkippedEmptyRange regardless of
    // which branch name won precedence, masking the assertion this test
    // exists to make — per ADR-040 v1.19 both paths now exit 0).
    repo.write_and_commit("docs/README.md", "# README\n", "docs update outside crate");

    let output = Command::new(binary_path())
        .current_dir(repo.path())
        .arg("cli-branch")
        .env("BASE_BRANCH", "env-branch-does-not-exist")
        .output()
        .expect("spawn policy15-attestation-gate binary");

    let exit_code = output.status.code().expect("process terminated by signal");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(
        exit_code, 0,
        "explicit CLI arg must win over BASE_BRANCH env var; stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("PASS-zero-activations"),
        "expected PASS-zero-activations via the explicit CLI-arg branch \
         (origin/cli-branch resolves), got stdout: {stdout}"
    );
}

// ── F-1 — parentless/root-commit skip guard: WARNING assertion ─────────────

/// F-1 — a parentless (root) commit placed INSIDE a non-empty evaluated
/// range must (a) be SKIPPED without causing a `Fail` or a hard error, and
/// (b) emit a WARNING per ADR-040 §Decision 9 Ruling 9(c) ("emits a WARNING
/// and skips").
///
/// Fixture: `base` (crate-seeded commit, also faked as `origin/develop`) →
/// `create_range_containing_parentless_commit()` (see its doc comment): a
/// disconnected parentless root commit merged back onto `base` via
/// `git merge --allow-unrelated-histories`, so `git merge-base` still
/// resolves to `base` while `git log base..HEAD` contains the merge commit,
/// the unrelated chain's child commit, AND the parentless root commit. None
/// of the three touches the pinned crate's `.rs`/`.bats` surface, so the
/// expected outcome is `PassZeroActivations` (exit 0) IF the skip is
/// harmless.
///
/// ## Assertion (a) — SKIPPED, no Fail / hard error
/// GREEN: `commit_has_parent` returning `None` already just `continue`s
/// (via `skipped_parentless.push(...)`) in `run_gate_inner` (lib.rs); this
/// fixture proves it.
///
/// ## Assertion (b) — WARNING emitted
/// GREEN, delivered. `run_gate_inner` threads skipped-parentless SHAs through
/// the structured `GateResult.skipped_parentless` field (CR-2, v1.18); `main.rs`
/// prints one WARNING line per entry, mirroring the existing FAIL/EmptyOrUnreachable
/// stderr detail-line pattern, before matching on `outcome` (ADR-040 §Decision 9
/// Ruling 9(c)). This is the binary-boundary CLI-contract test — a spawned-process
/// test can only observe stdout/stderr/exit code, so a stderr substring assertion
/// is the correct and sufficient tool at this boundary. A companion structural
/// assertion directly on `GateResult.skipped_parentless` at the library boundary
/// is test-writer scope for a following pass, not duplicated here.
#[test]
fn test_f1_parentless_commit_in_range_is_skipped_with_warning() {
    let repo = Repo::new();
    repo.write_and_commit(
        &format!("{PC}/docs/placeholder.md"),
        "placeholder\n",
        "seed: crate in HEAD tree",
    );
    let base = repo.head_sha();
    repo.fake_origin_ref("develop", &base);

    let root_sha = repo.create_range_containing_parentless_commit();

    let (exit_code, stdout, stderr) = repo.run_binary("develop");

    // (a) SKIPPED — no Fail, no hard error.
    assert_eq!(
        exit_code, 0,
        "a parentless commit in-range must not cause FAIL/hard-error exit; stdout={stdout} stderr={stderr}"
    );
    assert!(
        stdout.contains("PASS-zero-activations"),
        "expected PASS-zero-activations (skip harmless, no activation), got stdout: {stdout}"
    );
    assert!(
        !stdout.contains("FAIL"),
        "a parentless commit in-range must never surface as FAIL, got stdout: {stdout}"
    );

    // (b) WARNING emitted on skip — delivered and GREEN.
    // ADR-040 §Decision 9 Ruling 9(c): "emits a WARNING and skips". `main.rs`
    // prints one WARNING line per `GateResult.skipped_parentless` entry (CR-2, v1.18).
    let root_short = &root_sha[..12];
    assert!(
        stderr.contains("WARNING") && stderr.contains(root_short),
        "expected a WARNING mentioning the skipped parentless commit {root_short:?} \
         on stderr, got stderr: {stderr:?}"
    );
}
