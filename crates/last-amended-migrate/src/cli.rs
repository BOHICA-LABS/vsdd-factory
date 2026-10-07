//! Command-line surface for `last-amended-migrate` (BC-10.13.001
//! §Description).
//!
//! Three subcommands: `migrate` (PC1-PC4, PC6), `rotate` (PC5, PC6), and
//! `register` (PC6 / §Architecture Anchors / S-15.03 AC-006). `migrate` and
//! `rotate` support `--check` (report violations without writing, mirroring
//! `compute-input-hash`'s `--check` convention) and default to apply mode
//! when `--check` is absent; `register` has no check mode — appending
//! registry entries is itself idempotent (re-running is always safe), so
//! there is no drift state to report.

use crate::rotate::DEFAULT_KEEP_RECENT;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;

/// `last-amended-migrate` top-level CLI (POLICY 21-compliant Rust tool;
/// BC-10.13.001).
#[derive(Parser, Debug)]
#[command(name = "last-amended-migrate")]
#[command(version)]
#[command(
    about = "Sanctioned Rust CLI for one-time last_amended/changelog: migration \
    plus changelog rotation (BC-10.13.001, POLICY 21-compliant)"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Subcommands mapping 1:1 to BC-10.13.001's two functions (§Description).
#[derive(Subcommand, Debug)]
pub enum Command {
    /// One-time migration of one file, or all 5 ADR-049-governed files,
    /// into the ADR-049 shape (PC1-PC4, PC6).
    Migrate {
        /// Target file path. Omit to migrate all 5 D-1149 files under
        /// `--factory-root`.
        #[arg(long)]
        path: Option<PathBuf>,
        /// Root of the `.factory/` tree; used only when `--path` is
        /// omitted.
        #[arg(long, default_value = ".factory")]
        factory_root: PathBuf,
        /// Report violations without writing (mirrors `compute-input-hash`
        /// `--check`).
        #[arg(long)]
        check: bool,
        /// Explicit opt-in required before a PC7 full-recovery split is
        /// allowed to PERMANENTLY DISCARD `STATE.md`'s chained historical
        /// `last_amended` entries (S-15.03 pr-reviewer B2-R; BC-10.13.001
        /// EC-006). `STATE.md` has no `changelog:` field to relocate them
        /// into, and this tool never writes to the frozen
        /// `STATE-amendment-history.md` sidecar (PC6) — so, unlike the
        /// other 4 governed files, this operation has no real destination
        /// for the recovered text. Without this flag, encountering such a
        /// chain on `STATE.md` refuses with a clear error instead of
        /// silently dropping the entries. Has no effect on the other 4
        /// files, which always relocate their chained entries into
        /// `changelog:` regardless of this flag.
        #[arg(long)]
        discard_state_chain: bool,
    },
    /// Rotate an over-long `changelog:` sequence into a per-cycle archive
    /// (PC5, PC6).
    Rotate {
        /// Target file path.
        #[arg(long)]
        path: PathBuf,
        /// Destination cycle name under `.factory/cycles/<cycle_name>/`.
        #[arg(long)]
        cycle_name: String,
        /// Number of most-recent items to retain in the source file.
        #[arg(long, default_value_t = DEFAULT_KEEP_RECENT)]
        keep_recent: usize,
        /// Report violations without writing.
        #[arg(long)]
        check: bool,
    },
    /// Register this tool's own output paths and the 5 pre-existing D-1149
    /// `*-amendment-history.md` sidecar paths in the artifact path registry
    /// (PC6, §Architecture Anchors, S-15.03 AC-006).
    Register {
        /// Path to `plugins/vsdd-factory/config/artifact-path-registry.yaml`
        /// (or an equivalent registry file in tests).
        #[arg(long)]
        registry: PathBuf,
    },
}

/// Dispatch a parsed CLI invocation to the corresponding subcommand handler
/// (BC-10.13.001 §Description) — routes to `migrate_file`/`migrate_all`
/// (PC1-PC4, PC6) or `rotate_changelog` (PC5, PC6) depending on which
/// subcommand was selected, prints the resulting report, and maps any
/// `Err` to a non-zero `ExitCode`.
///
/// `MigrationMode::Check`/`MigrationMode::Apply` are derived here from each
/// subcommand's `check: bool` flag. This is the ONLY branching point in the
/// CLI surface — `main()` itself is a single-line delegate to
/// `run(Cli::parse())`.
///
/// `--check` never exits nonzero on success and never writes; it exits
/// nonzero exactly when the underlying report would have recorded a
/// mutation (drift found), mirroring `compute-input-hash`'s own `--check`
/// convention.
pub fn run(cli: Cli) -> ExitCode {
    use crate::migrate::{MigrationMode, MigrationReport, migrate_all, migrate_file};
    use crate::registry::register_artifact_paths;
    use crate::rotate::rotate_changelog;

    match cli.command {
        Command::Migrate {
            path,
            factory_root,
            check,
            discard_state_chain,
        } => {
            // S-15.03 SEC-002 (CWE-73): allowlist the user-supplied
            // single-file `--path` BEFORE any read/write of its content.
            // `migrate_all`'s own `--path`-omitted form already only ever
            // touches the hardcoded `TARGET_FILES` set, so it needs no
            // separate check here.
            if let Some(p) = &path
                && let Err(e) = crate::path_guard::validate_migrate_path(p, &factory_root)
            {
                eprintln!("last-amended-migrate: error: {e}");
                return ExitCode::FAILURE;
            }
            let mode = if check {
                MigrationMode::Check
            } else {
                MigrationMode::Apply
            };
            let options = crate::migrate::MigrationOptions {
                discard_state_chain,
            };
            let result = match path {
                Some(p) => migrate_file(&p, mode, options).map(|report| MigrationReport {
                    files: vec![report],
                }),
                None => migrate_all(&factory_root, mode, options),
            };
            match result {
                Ok(report) => {
                    for file in &report.files {
                        println!(
                            "{}: eligibility={:?} changelog={:?} escape_fixed={} entries_relocated={} entries_discarded={} mutated={}",
                            file.path.display(),
                            file.eligibility,
                            file.changelog_mutation,
                            file.escape_fixed,
                            file.entries_relocated,
                            file.entries_discarded,
                            file.mutated
                        );
                    }
                    if mode == MigrationMode::Check && report.total_mutated() > 0 {
                        ExitCode::FAILURE
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("last-amended-migrate: error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Rotate {
            path,
            cycle_name,
            keep_recent,
            check,
        } => {
            // S-15.03 SEC-002 (CWE-73): allowlist `--path` before any I/O.
            if let Err(e) = crate::path_guard::validate_rotate_path(&path) {
                eprintln!("last-amended-migrate: error: {e}");
                return ExitCode::FAILURE;
            }
            let mode = if check {
                MigrationMode::Check
            } else {
                MigrationMode::Apply
            };
            match rotate_changelog(&path, &cycle_name, keep_recent, mode) {
                Ok(report) => {
                    println!(
                        "{}: items_moved={} archive={} mutated={}",
                        report.path.display(),
                        report.items_moved,
                        report.archive_path.display(),
                        report.mutated
                    );
                    if mode == MigrationMode::Check && report.items_moved > 0 {
                        ExitCode::FAILURE
                    } else {
                        ExitCode::SUCCESS
                    }
                }
                Err(e) => {
                    eprintln!("last-amended-migrate: error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
        Command::Register { registry } => {
            // S-15.03 SEC-002 (CWE-73): allowlist `--registry` before any
            // read/append/write of its content.
            if let Err(e) = crate::path_guard::validate_registry_path(&registry) {
                eprintln!("last-amended-migrate: error: {e}");
                return ExitCode::FAILURE;
            }
            match register_artifact_paths(&registry) {
                Ok(()) => {
                    println!("{}: registration complete", registry.display());
                    ExitCode::SUCCESS
                }
                Err(e) => {
                    eprintln!("last-amended-migrate: error: {e}");
                    ExitCode::FAILURE
                }
            }
        }
    }
}
