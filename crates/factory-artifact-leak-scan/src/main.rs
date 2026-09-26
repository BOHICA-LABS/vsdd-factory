//! CLI for the factory-artifact leak scanner (#515). Output, modes, and exit
//! codes are contract-identical to the retired shell implementation so the
//! bats suite and factory-health check #10 port without behavior change.
//!
//! Usage:
//!   factory-artifact-leak-scan                 # table of leaks
//!   factory-artifact-leak-scan --list          # one leaked path per line
//!   factory-artifact-leak-scan --count         # number of leaks
//!
//! Exit codes:
//!   0 — no leaks found (registry-clean)
//!   1 — one or more leaked factory artifacts detected
//!   2 — usage / environment error (not a git repo, missing templates dir)
//!
//! Deterministic, no network, no LLM. Advisory by design (see #515
//! discussion): it REPORTS leaks for a human/orchestrator to relocate; it
//! does not mutate.

use std::env;
use std::path::PathBuf;
use std::process::{Command, exit};

use factory_artifact_leak_scan::{Leak, collect_factory_doctypes, scan};

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Table,
    List,
    Count,
}

fn die(msg: &str) -> ! {
    eprintln!("factory-artifact-leak-scan: {msg}");
    exit(2);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let mode = match args.first().map(String::as_str) {
        None => Mode::Table,
        Some("--list") => Mode::List,
        Some("--count") => Mode::Count,
        Some(other) => die(&format!(
            "unknown mode '{other}' (expected --list | --count | or no argument)"
        )),
    };

    // Repo root — scan tracked files here. Honour an explicit override for
    // testing (same contract as the shell version).
    let repo_root: PathBuf = match env::var("VSDD_REPO_ROOT") {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => {
            let out = Command::new("git")
                .args(["rev-parse", "--show-toplevel"])
                .output();
            match out {
                Ok(o) if o.status.success() => {
                    PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string())
                }
                _ => die("not inside a git repository"),
            }
        }
    };

    // Plugin root: env override, else the engine-repo layout. (The shell
    // version defaulted relative to its own file location inside bin/; a
    // compiled binary lives in target/, so the in-repo default anchors on
    // the repo root instead.)
    let plugin_root: PathBuf = match env::var("CLAUDE_PLUGIN_ROOT") {
        Ok(v) if !v.is_empty() => PathBuf::from(v),
        _ => repo_root.join("plugins/vsdd-factory"),
    };
    let templates = plugin_root.join("templates");
    if !templates.is_dir() {
        die(&format!(
            "templates dir not found at {}",
            templates.display()
        ));
    }

    let doctypes = match collect_factory_doctypes(&templates) {
        Ok(d) => d,
        Err(e) => die(&format!(
            "failed to read templates under {}: {e}",
            templates.display()
        )),
    };
    if doctypes.is_empty() {
        die(&format!(
            "no factory document_types found in {}",
            templates.display()
        ));
    }

    let out = Command::new("git")
        .args(["ls-files", "*.md"])
        .current_dir(&repo_root)
        .output();
    let tracked: Vec<String> = match out {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        _ => die(&format!("git ls-files failed in {}", repo_root.display())),
    };

    let leaks = scan(&repo_root, &tracked, &doctypes);
    emit(mode, &leaks);
}

fn emit(mode: Mode, leaks: &[Leak]) -> ! {
    if mode == Mode::Count {
        println!("{}", leaks.len());
        exit(if leaks.is_empty() { 0 } else { 1 });
    }

    if leaks.is_empty() {
        if mode == Mode::Table {
            println!("0 leaked factory artifacts found. Product tree is clean.");
        }
        exit(0);
    }

    if mode == Mode::List {
        for l in leaks {
            println!("{}", l.path);
        }
        exit(1);
    }

    // table mode
    eprintln!("Leaked factory artifacts detected on the product branch (#515):");
    eprintln!();
    eprintln!("{:<24}  tracked path", "document_type");
    eprintln!("{:<24}  ------------", "------------------------");
    for l in leaks {
        eprintln!("{:<24}  {}", l.doctype, l.path);
    }
    eprintln!();
    eprintln!("These carry factory-artifact frontmatter but live outside .factory/.");
    eprintln!(
        "Relocate to their canonical .factory/ home (see config/artifact-path-registry.yaml)"
    );
    eprintln!("or, if a genuine product deliverable, add its (document_type -> home");
    eprintln!(
        "directory) pair to PRODUCT_TRACKED_HOMES in crates/factory-artifact-leak-scan with justification."
    );
    exit(1);
}
