//! Orchestration: isolated worktree per mutant, one substitution, run the
//! killer, classify, always clean up.

use crate::spec::{Killer, Mutant};
use crate::subst::apply_single;
use crate::verdict::{Verdict, classify_kani, classify_test};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

/// Default per-mutant killer wall-clock budget. A Kani mutant takes ~20 min
/// of CBMC solve time, so 45 min is a >2x margin that still bounds a hung
/// solver far below the workflow's job-level timeout.
pub const DEFAULT_MUTANT_TIMEOUT: Duration = Duration::from_secs(45 * 60);

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
    /// Wall-clock budget for ONE mutant's killer. On expiry the killer's whole
    /// process group is killed and the mutant is INVALID (a timeout is not
    /// evidence that the mutant was killed).
    pub mutant_timeout: Duration,
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

/// Captured result of a killer process.
#[derive(Debug)]
pub struct Captured {
    /// Exit code (`None` if terminated by a signal or killed on timeout).
    pub code: Option<i32>,
    /// stdout followed by stderr.
    pub combined: String,
    /// True if the wall-clock budget expired and the process group was killed.
    pub timed_out: bool,
}

/// Upper bound on how long [`run_with_timeout`] waits for a pipe reader to
/// reach EOF after the child has exited or been killed. A descendant that
/// escaped the killed process group (own session, or a failed/absent group
/// kill on non-unix) can hold the pipe's write end open indefinitely; the
/// output captured up to that point is kept and the reader is detached.
const DRAIN_GRACE: Duration = Duration::from_secs(2);

/// Spawn a reader thread that streams the pipe into a shared buffer and
/// signals completion on a channel, so the caller can wait with a deadline
/// (a plain `JoinHandle::join` has no timeout) and still keep the partial
/// output read so far.
struct Drain {
    buf: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    done: std::sync::mpsc::Receiver<()>,
}

fn drain<R: Read + Send + 'static>(mut r: R) -> Drain {
    let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let (tx, done) = std::sync::mpsc::channel();
    let shared = std::sync::Arc::clone(&buf);
    std::thread::spawn(move || {
        let mut chunk = [0u8; 8192];
        loop {
            match r.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => {
                    if let Ok(mut g) = shared.lock() {
                        g.extend_from_slice(&chunk[..n]);
                    }
                }
            }
        }
        let _ = tx.send(());
    });
    Drain { buf, done }
}

impl Drain {
    /// Wait up to `grace` (shared deadline) for EOF, then return whatever
    /// was captured; never blocks past the deadline.
    fn collect(self, deadline: Instant) -> Vec<u8> {
        let wait = deadline.saturating_duration_since(Instant::now());
        let _ = self.done.recv_timeout(wait);
        self.buf.lock().map(|g| g.clone()).unwrap_or_default()
    }
}

/// Run `prog args` in `cwd` with `envs`, capturing stdout+stderr, killing the
/// whole process group (cargo -> kani -> cbmc/solver children) if it exceeds
/// `timeout`.
pub fn run_with_timeout(
    prog: &str,
    args: &[String],
    cwd: &Path,
    envs: &[(&str, &Path)],
    timeout: Duration,
) -> Result<Captured, RunError> {
    let mut cmd = Command::new(prog);
    cmd.args(args)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (k, v) in envs {
        cmd.env(k, v);
    }
    // Own process group so a timeout can reap the solver grandchildren too.
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        cmd.process_group(0);
    }
    let mut child = cmd.spawn()?;
    let stdout = child.stdout.take().map(drain);
    let stderr = child.stderr.take().map(drain);

    let deadline = Instant::now() + timeout;
    let (status, timed_out) = loop {
        if let Some(st) = child.try_wait()? {
            break (Some(st), false);
        }
        if Instant::now() >= deadline {
            kill_group(&mut child);
            let _ = child.wait();
            break (None, true);
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let drain_deadline = Instant::now() + DRAIN_GRACE;
    let out_bytes = stdout
        .map(|d| d.collect(drain_deadline))
        .unwrap_or_default();
    let err_bytes = stderr
        .map(|d| d.collect(drain_deadline))
        .unwrap_or_default();
    let mut combined = String::from_utf8_lossy(&out_bytes).into_owned();
    combined.push_str(&String::from_utf8_lossy(&err_bytes));
    Ok(Captured {
        code: status.and_then(|s| s.code()),
        combined,
        timed_out,
    })
}

#[cfg(unix)]
fn kill_group(child: &mut std::process::Child) {
    // `kill -KILL -- -<pgid>`: the child is its own group leader
    // (process_group(0)), so pgid == pid. Falls back to killing just the child.
    let group = format!("-{}", child.id());
    let ok = Command::new("kill")
        .args(["-KILL", "--", &group])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !ok {
        let _ = child.kill();
    }
}

#[cfg(not(unix))]
fn kill_group(child: &mut std::process::Child) {
    let _ = child.kill();
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
    let captured = run_with_timeout(
        prog,
        &args,
        &wt.path,
        &[("CARGO_TARGET_DIR", cfg.target_dir.as_path())],
        cfg.mutant_timeout,
    )?;
    let combined = captured.combined;
    std::fs::create_dir_all(&cfg.log_dir)?;
    let log = cfg.log_dir.join(format!("{}.log", m.id));
    std::fs::write(&log, &combined)?;

    if captured.timed_out {
        writeln!(
            out,
            "VP147-MUTANT {} killer-TIMEOUT after {}s (budget {}s) log={}",
            m.id,
            started.elapsed().as_secs(),
            cfg.mutant_timeout.as_secs(),
            log.display()
        )?;
        return Ok(Verdict::Invalid {
            reason: format!(
                "killer timed out after {}s (per-mutant budget)",
                cfg.mutant_timeout.as_secs()
            ),
        });
    }

    let code = captured.code;
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

    #[cfg(unix)]
    #[test]
    fn run_with_timeout_captures_output_and_exit_code() {
        let args = vec!["-c".to_string(), "echo out; echo err >&2; exit 3".into()];
        let c =
            run_with_timeout("sh", &args, Path::new("."), &[], Duration::from_secs(30)).unwrap();
        assert!(!c.timed_out);
        assert_eq!(c.code, Some(3));
        assert!(c.combined.contains("out") && c.combined.contains("err"));
    }

    #[cfg(unix)]
    #[test]
    fn run_with_timeout_kills_a_hung_killer_and_its_children() {
        let args = vec!["-c".to_string(), "sleep 60 & sleep 60".into()];
        let started = Instant::now();
        let c =
            run_with_timeout("sh", &args, Path::new("."), &[], Duration::from_millis(300)).unwrap();
        assert!(c.timed_out, "must report the timeout");
        assert_eq!(c.code, None);
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "must not wait for the hung killer"
        );
    }

    /// NB-4: a descendant that escaped the killed process group (own session)
    /// keeps the stdout/stderr pipe write-end open; the drain-thread joins
    /// must be bounded so a timeout always returns.
    #[cfg(unix)]
    #[test]
    fn run_with_timeout_returns_even_if_an_escaped_child_holds_the_pipe_open() {
        // This scenario needs python3 to spawn a session-escaped grandchild.
        // Skip LOUDLY (never silently pass) when it is absent.
        let python_ok = Command::new("python3")
            .arg("--version")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        if !python_ok {
            eprintln!(
                "SKIPPED (NOT PASSED): run_with_timeout_returns_even_if_an_escaped_child_holds_the_pipe_open \
                 requires `python3` on PATH and it is absent; the NB-4 bounded-drain-join \
                 behaviour was NOT exercised in this environment"
            );
            return;
        }

        // The escaped `sleep 60` is in its own session, so run_with_timeout's
        // process-group kill cannot reach it. It records its pid so this test
        // can reap it at the end (RAII guard: runs even if an assert panics).
        struct ReapGuard(PathBuf);
        impl Drop for ReapGuard {
            fn drop(&mut self) {
                if let Ok(txt) = std::fs::read_to_string(&self.0) {
                    if let Ok(pid) = txt.trim().parse::<u32>() {
                        let _ = Command::new("kill")
                            .args(["-KILL", &pid.to_string()])
                            .stdin(Stdio::null())
                            .stdout(Stdio::null())
                            .stderr(Stdio::null())
                            .status();
                    }
                }
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let pid_file = std::env::temp_dir().join(format!(
            "vp147-escaped-child-{}-{:?}.pid",
            std::process::id(),
            std::thread::current().id()
        ));
        let _ = std::fs::remove_file(&pid_file);
        let _reap = ReapGuard(pid_file.clone());

        let script = format!(
            "import subprocess,time; \
             p=subprocess.Popen(['sleep','60'], start_new_session=True); \
             open({:?},'w').write(str(p.pid)); time.sleep(60)",
            pid_file.to_string_lossy()
        );
        let args = vec!["-c".to_string(), script];
        let started = Instant::now();
        let c = run_with_timeout(
            "python3",
            &args,
            Path::new("."),
            &[],
            Duration::from_millis(500),
        )
        .unwrap();
        assert!(c.timed_out, "must report the timeout");
        assert!(
            started.elapsed() < Duration::from_secs(20),
            "drain joins must be bounded, took {:?}",
            started.elapsed()
        );
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
