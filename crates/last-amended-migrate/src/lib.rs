//! `last-amended-migrate` — sanctioned Rust CLI for the one-time
//! `last_amended`/`changelog:` migration plus changelog rotation
//! (BC-10.13.001, POLICY 21-compliant; S-15.03 AC-006).
//!
//! # Behavioral Contract
//!
//! BC-10.13.001 — see
//! `.factory/specs/behavioral-contracts/ss-10/BC-10.13.001.md` for the full
//! postcondition/invariant/edge-case specification this crate implements.
//! Postcondition-to-module map:
//!
//! | PC | Module | Function |
//! |----|--------|----------|
//! | PC1 (`changelog:` coverage completion) | [`changelog`] | `ensure_changelog_field` |
//! | PC2 (current-entry-only confirmation)  | [`eligibility`] | `check_eligibility` |
//! | PC3 (D-1144 escape remediation)        | [`escape`] | `needs_escaping`, `escape_value` |
//! | PC4 (idempotency)                      | [`migrate`] | `migrate_file` (cross-cutting) |
//! | PC5 (lossless rotation)                | [`rotate`] | `rotate_changelog` |
//! | PC6 (frozen-sidecar non-mutation)       | [`registry`] | `register_artifact_paths` (read-only w.r.t. sidecars); cross-cutting elsewhere |
//!
//! # Architecture compliance (POLICY 21)
//!
//! Delivered as a native Rust binary under `crates/last-amended-migrate/`
//! (explicitly NOT a new `.sh` script), satisfying `no_new_shell_scripts`.
//! This is a standalone operator/agent-invoked CLI tool, not a WASM hook
//! plugin — it has no dependency on `vsdd-hook-sdk` and is not built for the
//! `wasm32-wasip1` target (contrast with every crate under
//! `crates/hook-plugins/`).

pub mod atomic_write;
pub mod changelog;
pub mod cli;
pub mod eligibility;
pub mod error;
pub mod escape;
pub mod frontmatter;
pub mod migrate;
pub mod path_guard;
pub mod registry;
pub mod rotate;
pub mod yaml_guard;

pub use cli::{Cli, Command, run};
pub use error::MigrateError;
pub use frontmatter::{FrontmatterDoc, parse_frontmatter};
pub use migrate::{
    FileMigrationReport, MigrationMode, MigrationOptions, MigrationReport, migrate_all,
    migrate_file,
};
pub use rotate::{RotationReport, rotate_changelog, rotate_changelog_at};
