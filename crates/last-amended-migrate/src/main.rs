//! `last-amended-migrate` binary entry point.
//!
//! Native Rust CLI (POLICY 21-compliant — explicitly NOT a new `.sh`
//! script) invoked by an operator or an orchestrator-dispatched agent
//! against one or all of the 5 ADR-049-governed files (BC-10.13.001
//! Precondition 1).
//!
//! All real dispatch/orchestration logic lives in
//! `last_amended_migrate::cli::run`; this file is intentionally a
//! single-line delegate.
//!
//! # WIRING-EXEMPT (BC-5.38.003)
//!
//! `main()` parses argv via `clap::Parser::parse()` and delegates to a
//! single call, `run(..)` — the framework-standard binary entry-point
//! shape (analogous to the `Self(value.into())` `From<T>` delegation
//! example), with no branching of its own. See the stub commit report's
//! WIRING-EXEMPT table.

use clap::Parser;
use last_amended_migrate::{Cli, run};
use std::process::ExitCode;

fn main() -> ExitCode {
    run(Cli::parse())
}
