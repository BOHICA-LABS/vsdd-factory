//! POLICY 15 ATTESTATION-LOCATION GATE — binary entry point.
//!
//! Invocation (for CI):
//!
//! ```text
//! policy15-attestation-gate [BASE_BRANCH]
//! ```
//!
//! Defaults to `develop`; the `BASE_BRANCH` environment variable also sets the default.
//! Prints the outcome identifier to stdout (greppable in CI logs) and exits with
//! the appropriate process code:
//!
//! | Outcome | Exit code |
//! |---------|-----------|
//! | `PASS-N-activations` | 0 |
//! | `PASS-zero-activations` | 0 |
//! | `SKIP: empty or unresolvable commit range — inert (no PR diff to evaluate)` | 0 |
//! | `FAIL: obligation violated` | 2 |
//! | `EMPTY-or-UNREACHABLE: *` | 2 |
//! | Hard error (git not found, I/O) | 1 |
//!
//! GitHub Actions job name: `policy-15-attestation-location`
//! (see ADR-040 §Decision 7 Ruling 7(a)). The gate is ratified (ADR-040 v1.18,
//! active); only CI wiring of this job remains pending (D-969).

use clap::Parser;
use policy15_attestation_gate::{
    FailReason, GateOutcome, GateResult, PLUGIN_CRATE, UnreachableCause, run_gate,
};

/// CR-5: truncate a full commit SHA to its first 12 characters (the convention used for
/// every short-SHA printed by this binary), falling back to the full string if shorter.
fn short_sha(s: &str) -> &str {
    if s.len() >= 12 { &s[..12] } else { s }
}

#[derive(Parser)]
#[command(name = "policy15-attestation-gate")]
#[command(version)]
#[command(about = "POLICY 15 ATTESTATION-LOCATION GATE (ADR-040 §Decisions 8+9)")]
#[command(
    long_about = "Evaluates the per-commit POLICY 15 obligation for the pinned crate.\n\
    Prints a greppable outcome identifier and exits with code 0 (PASS) or 2 (FAIL/EMPTY)."
)]
struct Cli {
    /// Base branch name; merge base is computed as merge-base(HEAD, origin/<base_branch>).
    /// Precedence: explicit CLI argument > `BASE_BRANCH` environment variable > `develop`
    /// default (clap's native derive precedence for an `env`-backed argument).
    #[arg(env = "BASE_BRANCH", default_value = "develop")]
    base_branch: String,
}

fn main() {
    let cli = Cli::parse();
    let base_branch = cli.base_branch;

    let repo = match std::env::current_dir() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("GATE ERROR: cannot determine current directory: {e}");
            std::process::exit(1);
        }
    };

    match run_gate(&repo, &base_branch) {
        Ok(GateResult {
            outcome,
            skipped_parentless,
            skipped_merge_inert: _,
        }) => {
            // CR-2: WARNING lines for skipped parentless commits, printed here (not by
            // `run_gate_inner`) — this is `main.rs`'s diagnostic channel, keyed off the
            // structured `skipped_parentless` list `run_gate` returns. Deliberately does
            // NOT warn on `skipped_merge_inert` — an inert sync-merge is the routine case
            // (ADR-040 §Decision 9 Ruling 9(e)), not an anomaly worth operator attention.
            for commit in &skipped_parentless {
                let short = short_sha(commit);
                eprintln!(
                    "  WARNING: commit {short} has no first parent (root commit or shallow-clone boundary) — skipping POLICY 15 evaluation for this commit"
                );
            }

            // Print detail lines to stderr, identifier to stdout.
            match &outcome {
                GateOutcome::Fail(failed) => {
                    for fc in failed {
                        let reason = match &fc.reason {
                            FailReason::LogAbsent => "log absent at commit".to_string(),
                            FailReason::AttestationMissing => {
                                "attestation heading missing in log".to_string()
                            }
                            FailReason::AttestationAmbiguous { count } => {
                                format!(
                                    "{count} attestation headings found for commit's parent (expected exactly 1)"
                                )
                            }
                        };
                        let short = short_sha(&fc.commit);
                        eprintln!("  FAIL {short}: {reason}");
                    }
                }
                GateOutcome::EmptyOrUnreachable(cause) => match cause {
                    UnreachableCause::StalePin => {
                        eprintln!("  stale pin: \"{PLUGIN_CRATE}\" absent from HEAD git tree")
                    }
                    UnreachableCause::UnmeasurableDiff { commit } => {
                        let short = short_sha(commit);
                        eprintln!("  unmeasurable diff at commit {short}");
                    }
                },
                // ADR-040 v1.19 Ruling 9(f): SkippedEmptyRange is an inert skip (exit 0).
                // No operator-facing detail line needed — the stdout identifier is sufficient.
                GateOutcome::SkippedEmptyRange => {}
                // M-1: explicit arms (not a `_ => {}` wildcard) restore the compile-time
                // exhaustiveness `GateOutcome`'s own doc comment says the deliberately
                // absent `#[non_exhaustive]` is FOR — a 6th variant added later without a
                // corresponding arm here now fails to compile instead of silently falling
                // into a wildcard. No per-commit detail line is needed for a PASS outcome.
                GateOutcome::PassWithActivations(_) => {}
                GateOutcome::PassZeroActivations => {}
            }
            println!("{}", outcome.identifier());
            std::process::exit(outcome.exit_code());
        }
        Err(e) => {
            eprintln!("GATE ERROR: {e}");
            std::process::exit(1);
        }
    }
}
