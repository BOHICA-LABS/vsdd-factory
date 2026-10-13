//! `vp147-mutants` CLI. See the crate-level docs in `lib.rs`.

use clap::Parser;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, ExitCode};
use vp147_mutants::runner::{self, RunConfig, tracked_tree_dirty};
use vp147_mutants::spec::{DEFAULT_SPEC_PATH, Spec};

#[derive(Parser, Debug)]
#[command(
    name = "vp147-mutants",
    about = "Run the VP-147 mutants (M1..M5) against isolated git worktrees and report KILLED/SURVIVED/INVALID. Kani mutants take ~20 min each.",
    version
)]
struct Cli {
    /// Mutant spec (default: <repo>/crates/vp147-mutants/mutants.toml).
    #[arg(long)]
    spec: Option<PathBuf>,
    /// Repository root (default: `git rev-parse --show-toplevel` of the cwd).
    #[arg(long)]
    repo: Option<PathBuf>,
    /// Revision to mutate (default: HEAD).
    #[arg(long, default_value = "HEAD")]
    base: String,
    /// Run only this mutant id (repeatable).
    #[arg(long = "only")]
    only: Vec<String>,
    /// Shared CARGO_TARGET_DIR (default: <repo>/target/vp147-mutants).
    #[arg(long)]
    target_dir: Option<PathBuf>,
    /// Directory for full per-mutant killer logs (default: <target-dir>/logs).
    #[arg(long)]
    log_dir: Option<PathBuf>,
    /// Only verify that every `find` text matches exactly once at --base; build nothing.
    #[arg(long)]
    check_spec: bool,
    /// Allow running when tracked files differ from HEAD (the mutants are
    /// applied to the committed --base, not to working-tree edits).
    #[arg(long)]
    allow_dirty: bool,
}

fn toplevel() -> Result<PathBuf, String> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err("not inside a git repository (pass --repo)".into());
    }
    Ok(PathBuf::from(String::from_utf8_lossy(&out.stdout).trim()))
}

fn real_main() -> Result<bool, String> {
    let cli = Cli::parse();
    let repo = match cli.repo {
        Some(r) => r,
        None => toplevel()?,
    };
    let spec_path = cli.spec.unwrap_or_else(|| repo.join(DEFAULT_SPEC_PATH));
    let text = std::fs::read_to_string(&spec_path)
        .map_err(|e| format!("cannot read spec {}: {e}", spec_path.display()))?;
    let spec = Spec::parse(&text).map_err(|e| e.to_string())?;
    let selected = spec.select(&cli.only).map_err(|e| e.to_string())?;

    let target_dir = cli
        .target_dir
        .unwrap_or_else(|| repo.join("target/vp147-mutants"));
    let log_dir = cli.log_dir.unwrap_or_else(|| target_dir.join("logs"));
    let cfg = RunConfig {
        repo: repo.clone(),
        base_rev: cli.base,
        target_dir,
        log_dir,
    };

    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    if cli.check_spec {
        let bad = runner::check_spec(&cfg, &selected, &mut out).map_err(|e| e.to_string())?;
        return Ok(bad == 0);
    }
    if !cli.allow_dirty && tracked_tree_dirty(&repo).map_err(|e| e.to_string())? {
        return Err(
            "tracked files differ from HEAD; mutants apply to the committed --base. Commit, or pass --allow-dirty."
                .into(),
        );
    }
    let (summary, _) = runner::run_all(&cfg, &selected, &mut out).map_err(|e| e.to_string())?;
    out.flush().map_err(|e| e.to_string())?;
    Ok(summary.passed())
}

fn main() -> ExitCode {
    match real_main() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(msg) => {
            eprintln!("vp147-mutants: {msg}");
            ExitCode::from(2)
        }
    }
}
