//! Orchestration: isolated worktree per mutant, one substitution, run the
//! killer, classify, always clean up.

use crate::spec::{Killer, Mutant};
use crate::subst::apply_single;
use crate::verdict::{Verdict, classify_kani, classify_test};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

#[derive(Debug, thiserror::Error)]
pub enum RunError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("git {args}: {detail}")]
    Git { args: String, detail: String },
}

#[derive(Debug, Clone)]
pub struct RunConfig {
    /// Repository (or worktree) whose `base_rev` is mutated. Never modified.
    pub repo: PathBuf,
    /// Revision the mutants are applied to; resolved to a full SHA.
    pub base_rev: String,
    /// Shared CARGO_TARGET_DIR so third-party dependency builds are reused
    /// across mutants (workspace crates still rebuild: their path differs).
    pub target_dir: PathBuf,
    /// Per-mutant full killer logs are written here as `<id>.log`.
    pub log_dir: PathBuf,
}

/// Outcome of one mutant.
#[derive(Debug, Clone)]
pub struct MutantResult {
    pub id: String,
    pub verdict: Verdict,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Summary {
    pub total: usize,
    pub killed: usize,
    pub survived: usize,
    pub invalid: usize,
}

impl Summary {
    pub fn record(&mut self, v: &Verdict) {
        self.total += 1;
        match v {
            Verdict::Killed { .. } => self.killed += 1,
            Verdict::Survived => self.survived += 1,
            Verdict::Invalid { .. } => self.invalid += 1,
        }
    }

    /// Every mutant must be KILLED (VP-147 expected verdict).
    pub fn passed(&self) -> bool {
        self.total > 0 && self.killed == self.total
    }

    pub fn line(&self) -> String {
        format!(
            "VP147-SUMMARY total={} killed={} survived={} invalid={} result={}",
            self.total,
            self.killed,
            self.survived,
            self.invalid,
            if self.passed() { "PASS" } else { "FAIL" }
        )
    }
}

fn git(repo: &Path, args: &[&str]) -> Result<String, RunError> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    } else {
        Err(RunError::Git {
            args: args.join(" "),
            detail: String::from_utf8_lossy(&out.stderr).trim().to_string(),
        })
    }
}

/// Resolve `rev` to a full commit SHA.
pub fn resolve_rev(repo: &Path, rev: &str) -> Result<String, RunError> {
    let spec = format!("{rev}^{{commit}}");
    Ok(git(repo, &["rev-parse", "--verify", &spec])?
        .trim()
        .to_string())
}

/// True if tracked files in `repo` differ from HEAD.
pub fn tracked_tree_dirty(repo: &Path) -> Result<bool, RunError> {
    Ok(
        !git(repo, &["status", "--porcelain", "--untracked-files=no"])?
            .trim()
            .is_empty(),
    )
}

/// The program and arguments for a killer (pure; unit-tested).
pub fn killer_command(killer: &Killer) -> (&'static str, Vec<String>) {
    match killer {
        Killer::Kani { package, harnesses } => {
            let mut a = vec![
                "kani".to_string(),
                "-p".into(),
                package.clone(),
                "--exact".into(),
            ];
            for h in harnesses {
                a.push("--harness".into());
                a.push(h.clone());
            }
            ("cargo", a)
        }
        Killer::Test { args } => {
            let mut a = vec!["test".to_string()];
            a.extend(args.iter().cloned());
            ("cargo", a)
        }
    }
}

/// Owns a scratch `git worktree`; removes it on drop (normal exit, early
/// return, or panic unwind).
struct WorktreeGuard {
    repo: PathBuf,
    path: PathBuf,
    // Dropped after `drop()` below has run, deleting the parent directory.
    _tmp: tempfile::TempDir,
}

impl WorktreeGuard {
    fn create(repo: &Path, sha: &str) -> Result<Self, RunError> {
        let tmp = tempfile::Builder::new().prefix("vp147-mutant-").tempdir()?;
        let path = tmp.path().join("wt");
        let p = path.to_string_lossy().into_owned();
        git(repo, &["worktree", "add", "--detach", &p, sha])?;
        Ok(Self {
            repo: repo.to_path_buf(),
            path,
            _tmp: tmp,
        })
    }
}

impl Drop for WorktreeGuard {
    fn drop(&mut self) {
        let p = self.path.to_string_lossy().into_owned();
        if git(&self.repo, &["worktree", "remove", "--force", &p]).is_err() {
            // Fall back: delete the directory, then drop the dangling
            // registration so no stale entry survives in the main repo.
            let _ = std::fs::remove_dir_all(&self.path);
            let _ = git(&self.repo, &["worktree", "prune"]);
        }
    }
}

/// Validate (without building) that each mutant's `find` text matches exactly
/// once in `base` (read via `git show`). Returns the number of bad mutants.
pub fn check_spec(
    cfg: &RunConfig,
    mutants: &[&Mutant],
    out: &mut dyn Write,
) -> Result<usize, RunError> {
    let sha = resolve_rev(&cfg.repo, &cfg.base_rev)?;
    writeln!(out, "VP147-CHECK base={sha}")?;
    let mut bad = 0;
    for m in mutants {
        let blob = git(&cfg.repo, &["show", &format!("{sha}:{}", m.file)]);
        let status = match blob.map(|c| apply_single(&c, &m.find, &m.replace)) {
            Ok(Ok(_)) => "matches=1 OK".to_string(),
            Ok(Err(e)) => {
                bad += 1;
                format!("INVALID {e}")
            }
            Err(e) => {
                bad += 1;
                format!("INVALID {e}")
            }
        };
        writeln!(out, "VP147-CHECK {} file={} {status}", m.id, m.file)?;
    }
    writeln!(
        out,
        "VP147-CHECK-SUMMARY total={} invalid={bad}",
        mutants.len()
    )?;
    Ok(bad)
}

fn run_one(
    cfg: &RunConfig,
    sha: &str,
    m: &Mutant,
    out: &mut dyn Write,
) -> Result<Verdict, RunError> {
    let started = Instant::now();
    let wt = WorktreeGuard::create(&cfg.repo, sha)?;
    writeln!(out, "VP147-MUTANT {} worktree=created base={sha}", m.id)?;

    let target = wt.path.join(&m.file);
    let original = std::fs::read_to_string(&target)?;
    let mutated = match apply_single(&original, &m.find, &m.replace) {
        Ok(s) => s,
        Err(e) => {
            writeln!(
                out,
                "VP147-MUTANT {} file={} substitution=FAILED ({e})",
                m.id, m.file
            )?;
            return Ok(Verdict::Invalid {
                reason: format!("spec mismatch: {e}"),
            });
        }
    };
    std::fs::write(&target, mutated)?;
    writeln!(out, "VP147-MUTANT {} file={} substitution=1", m.id, m.file)?;
    let diff = git(&wt.path, &["--no-pager", "diff", "--no-color", "-U2"])?;
    writeln!(
        out,
        "--- mutant diff ({}) ---\n{}-------------------",
        m.id, diff
    )?;

    let (prog, args) = killer_command(&m.killer);
    writeln!(
        out,
        "VP147-MUTANT {} killer={prog} {}",
        m.id,
        args.join(" ")
    )?;
    out.flush()?;
    let output = Command::new(prog)
        .args(&args)
        .current_dir(&wt.path)
        .envs([("CARGO_TARGET_DIR", &cfg.target_dir)])
        .output()?;
    let mut combined = String::from_utf8_lossy(&output.stdout).into_owned();
    combined.push_str(&String::from_utf8_lossy(&output.stderr));
    std::fs::create_dir_all(&cfg.log_dir)?;
    let log = cfg.log_dir.join(format!("{}.log", m.id));
    std::fs::write(&log, &combined)?;

    let code = output.status.code();
    let verdict = match &m.killer {
        Killer::Kani { harnesses, .. } => classify_kani(code, &combined, harnesses.len()),
        Killer::Test { .. } => classify_test(code, &combined),
    };
    writeln!(
        out,
        "VP147-MUTANT {} killer-exit={} elapsed={}s log={}",
        m.id,
        code.map_or_else(|| "signal".to_string(), |c| c.to_string()),
        started.elapsed().as_secs(),
        log.display()
    )?;
    Ok(verdict)
}

/// Run each mutant in its own scratch worktree; print literal per-mutant and
/// summary lines to `out`.
pub fn run_all(
    cfg: &RunConfig,
    mutants: &[&Mutant],
    out: &mut dyn Write,
) -> Result<(Summary, Vec<MutantResult>), RunError> {
    let sha = resolve_rev(&cfg.repo, &cfg.base_rev)?;
    std::fs::create_dir_all(&cfg.target_dir)?;
    let mut summary = Summary::default();
    let mut results = Vec::new();
    for m in mutants {
        let verdict = match run_one(cfg, &sha, m, out) {
            Ok(v) => v,
            Err(e) => Verdict::Invalid {
                reason: format!("runner error: {e}"),
            },
        };
        let detail = match &verdict {
            Verdict::Killed { evidence } => format!(" evidence=[{}]", evidence.join("; ")),
            Verdict::Survived => String::new(),
            Verdict::Invalid { reason } => format!(" reason={reason}"),
        };
        writeln!(
            out,
            "VP147-MUTANT {} VERDICT={} expected=KILLED{detail}",
            m.id, verdict
        )?;
        out.flush()?;
        summary.record(&verdict);
        results.push(MutantResult {
            id: m.id.clone(),
            verdict,
        });
    }
    writeln!(out, "{}", summary.line())?;
    Ok((summary, results))
}

#[cfg(test)]
#[allow(clippy::unwrap_used, clippy::expect_used, clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn summary_requires_all_killed() {
        let mut s = Summary::default();
        assert!(!s.passed(), "empty run must not pass");
        s.record(&Verdict::Killed { evidence: vec![] });
        assert!(s.passed());
        s.record(&Verdict::Survived);
        assert!(!s.passed());
        assert_eq!(
            s.line(),
            "VP147-SUMMARY total=2 killed=1 survived=1 invalid=0 result=FAIL"
        );
        let mut t = Summary::default();
        t.record(&Verdict::Killed { evidence: vec![] });
        t.record(&Verdict::Invalid { reason: "x".into() });
        assert!(!t.passed());
        assert_eq!(t.invalid, 1);
    }

    #[test]
    fn killer_command_shapes() {
        let (p, a) = killer_command(&Killer::Kani {
            package: "factory-dispatcher".into(),
            harnesses: vec!["m::a".into(), "m::b".into()],
        });
        assert_eq!(p, "cargo");
        assert_eq!(
            a,
            [
                "kani",
                "-p",
                "factory-dispatcher",
                "--exact",
                "--harness",
                "m::a",
                "--harness",
                "m::b"
            ]
        );
        let (p, a) = killer_command(&Killer::Test {
            args: vec!["-p".into(), "x".into(), "--no-fail-fast".into()],
        });
        assert_eq!(
            (p, a),
            (
                "cargo",
                vec![
                    "test".into(),
                    "-p".into(),
                    "x".into(),
                    "--no-fail-fast".into()
                ]
            )
        );
    }
}
