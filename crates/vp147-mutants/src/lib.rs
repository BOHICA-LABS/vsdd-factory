//! Durable VP-147 mutation runner (S-25.08 T-12, D-1252-OWED).
//!
//! VP-147 claims that the OBL-1 Kani suite and the VP-133 black-box suite
//! *kill* a fixed list of mutants (M1..M5). A mutant is a one-site source
//! edit; "killed" means the named Kani harness(es) report
//! `VERIFICATION:- FAILED`, or the named `cargo test` targets report a failing
//! test, against the mutated tree. This crate re-runs that experiment
//! reproducibly from a checked-in spec (`mutants.toml`).
//!
//! # Running
//!
//! ```text
//! # validate that every mutant's `find` text still matches exactly once at
//! # HEAD (seconds; no build, no worktree)
//! cargo run -p vp147-mutants -- --check-spec
//!
//! # run everything (Kani solves cost ~20 min per Kani mutant)
//! cargo run --release -p vp147-mutants
//!
//! # run selected mutants against an explicit revision
//! cargo run --release -p vp147-mutants -- --only M3 --only M5 --base <rev>
//! ```
//!
//! Requirements: `git`, `cargo`, and (for Kani mutants) `cargo-kani`
//! (`kani-verifier` 0.68.0, matching `.github/workflows/kani.yml`).
//!
//! Exit status: 0 only when every selected mutant is `KILLED`; non-zero when
//! any mutant `SURVIVED`, was `INVALID`, or the run could not be set up.
//!
//! # Verdict classification
//!
//! Verdicts come from the killer's exit status and *parsed result lines*
//! (`VERIFICATION:- ...`, `test ... FAILED`, `test result: ...`,
//! line-anchored cargo compile diagnostics). Free-text substring matching on
//! "error" is deliberately avoided: cargo prints `error: test failed, to
//! rerun pass ...` for ordinary test failures, which the first scratchpad
//! runner mistook for a compile error and reported as a false INVALID.

pub mod runner;
pub mod spec;
pub mod subst;
pub mod verdict;

pub use runner::{RunConfig, RunError, run_all};
pub use spec::{Killer, Mutant, Spec, SpecError};
pub use subst::{SubstError, apply_single};
pub use verdict::{Verdict, classify_kani, classify_test};
