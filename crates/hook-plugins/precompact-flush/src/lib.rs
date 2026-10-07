//! precompact-flush — pure logic for the native WASM PreCompact hook plugin.
//!
//! This module contains the testable pure state machine. All effectful
//! operations (host `read_file`, `write_file`, `exec_subprocess`) are injected
//! via closures so the logic can be unit-tested without a WASM runtime.
//!
//! The effectful entry point lives in `src/main.rs` (vsdd-hook-sdk trampoline).
//!
//! # Architecture mapping (S-18.04a)
//!
//! | Component | Module | Pure/Effectful |
//! |-----------|--------|---------------|
//! | Pure state machine | `src/lib.rs` (this file) | Pure |
//! | Effectful entry point | `src/main.rs` | Effectful |
//!
//! # Canonical execution order (BC-7.07.001 INV3 / AC-011)
//!
//! 1.  Discover factory-artifacts worktree path via `git worktree list --porcelain`
//!     (AC-017); fail-open with DURABILITY DEGRADED advisory if not found.
//! 2.  AC-017 canonicalize mount guard (F-R3-001): verify discovered path ==
//!     `<cwd>/.factory` (Tier-1 suffix check + Tier-2 canonicalize comparison);
//!     fail-open with DURABILITY DEGRADED if mismatch. Runs BEFORE any I/O.
//! 3.  Read STATE.md via host `read_file`; exit 0 + warn if unreadable (AC-002).
//!     The AC-002 fail-open occurs AFTER steps 1–2.
//! 4.  Identity-gated renewal via `step4_renewal_gate` / `renew_lock_if_holder` (AC-003, AC-018,
//!     S-17.07 / ADR-046 Decision 3):
//!     - `Renewed(content)` → write STATE.md (step 5) and proceed with new content
//!     - `AlreadyExpired` / `NotHolder` / absent block → skip step 5; flush with original
//!     - `IdentityResolutionFailed` → emit `factory.lock.renewal_indeterminate` event +
//!       `host::log_warn`; flush with original content
//!     - `Err(Malformed)` → MANDATORY `host::log_warn` (SHALL); skip step 5; proceed to step 6
//! 5.  If `Renewed`: call `host::write_file(".factory/STATE.md", content)` (AC-018).
//! 6.  `git -C <wt> add -A` — stage ALL changes including new untracked files (AC-004).
//!     After staging, check `git -C <wt> diff --cached`: if NoOp AND diff empty →
//!     INV5 clean-state → exit 0 silently; no commit (AC-005 / BC-7.07.001 INV5).
//! 7.  `git -C <wt> commit -m <msg>` (LOCAL; no network); exit 2 on failure (AC-005b).
//! 8.  `SHA_B = git -C <wt> rev-parse HEAD` — IMMEDIATELY after commit, BEFORE
//!     append (AC-006 / BC-7.07.001 PC8).
//! 9.  Append 4-field `\n`-terminated line to precompact-flush-log via host
//!     `write_file`; treat absent-log read error as empty baseline (AC-007).
//! 10. If append fails: compare CURRENT_HEAD to SHA_B; if equal → `git reset --soft
//!     SHA_B^`; if not equal → no reset + human-intervention message; exit 2 (AC-008).
//! 11. If append succeeds: `git -C <wt> push origin factory-artifacts` (NETWORK) (AC-009).
//! 12. If push fails: exit 2 with retry message (AC-009).
//! 13. If push succeeds: exit 0 (AC-009).

#![cfg_attr(not(kani), allow(unexpected_cfgs))]

use chrono::Utc;
use factory_lock::{
    IdentityResolution, LockError, RenewOutcome, SkipReason, classify_identity_resolution,
    renew_lock_if_holder,
};
use vsdd_hook_sdk::{HookPayload, HookResult};

/// Precompact-flush log path (relative to `.factory/` host write_file root).
///
/// BC-7.07.001 Architecture Anchors / AC-015: exact path, no extension.
pub const LOG_PATH: &str = ".factory/hooks/precompact-flush-log";

/// STATE.md path (relative to `.factory/` host write_file root).
pub const STATE_MD_PATH: &str = ".factory/STATE.md";

/// Commit message prefix. MUST begin with exactly `PreCompact flush ` (capital P,
/// capital C, single space between words, trailing space) per BC-5.41.003 INV3 and
/// BC-7.07.001 INV4. Used as exemption key by validate-burst-log.
pub const COMMIT_PREFIX: &str = "PreCompact flush ";

/// Factory-artifacts branch name used in `git worktree list --porcelain` parsing.
pub const FACTORY_ARTIFACTS_BRANCH: &str = "refs/heads/factory-artifacts";

/// Remote and branch for `git push` (AC-009).
pub const PUSH_REMOTE: &str = "origin";
pub const PUSH_BRANCH: &str = "factory-artifacts";

// ---------------------------------------------------------------------------
// Data types used by the pure state machine
// ---------------------------------------------------------------------------

/// Outcome of worktree discovery (AC-017).
#[derive(Debug, Clone, PartialEq)]
pub enum WorktreeDiscovery {
    /// Worktree found at the given absolute path.
    Found(String),
    /// `git worktree list --porcelain` subprocess exited non-zero.
    CommandFailed { exit_code: i32 },
    /// Command succeeded but no factory-artifacts stanza found.
    BranchNotFound,
    /// Canonicalize assertion failed: discovered path != `<cwd>/.factory`.
    PathMismatch {
        discovered: String,
        expected: String,
    },
}

/// Parsed STATE.md context fields (BC-7.07.001 PC1 / AC-005).
///
/// Derived from `current_cycle:` and `current_step:` YAML frontmatter fields.
/// NOT from `current_wave:` (phantom field — must NOT be used).
#[derive(Debug, Clone, PartialEq)]
pub struct StateContext {
    /// Value of `current_cycle:` frontmatter field.
    pub current_cycle: String,
    /// Value of `current_step:` frontmatter field.
    pub current_step: String,
}

/// Decision of the append-failure concurrent-commit guard (AC-008).
#[derive(Debug, Clone, PartialEq)]
pub enum AppendFailureAction {
    /// CURRENT_HEAD == SHA_B — safe to reset. Run `git reset --soft SHA_B^`.
    ResetSafe { sha_b: String },
    /// CURRENT_HEAD != SHA_B — intervening commit; do NOT reset.
    NoResetHumanIntervention { sha_b: String, current_head: String },
}

// ---------------------------------------------------------------------------
// Pure functions (pure state machine logic — no I/O)
// ---------------------------------------------------------------------------

/// Parse the factory-artifacts worktree path from `git worktree list --porcelain`
/// output.
///
/// Finds the stanza whose `branch` line is `refs/heads/factory-artifacts` and
/// returns the corresponding `worktree` line as the absolute path.
///
/// Returns `Some(absolute_path)` if found, `None` if no matching stanza.
///
/// Used by the effectful entry point in `src/main.rs` (AC-017).
pub fn parse_worktree_list(porcelain_output: &str) -> Option<String> {
    let mut current_worktree: Option<String> = None;

    for line in porcelain_output.lines() {
        if let Some(path) = line.strip_prefix("worktree ") {
            current_worktree = Some(path.to_string());
        } else if let Some(branch) = line.strip_prefix("branch ") {
            if branch == FACTORY_ARTIFACTS_BRANCH {
                return current_worktree;
            }
        } else if line.is_empty() {
            // Blank line separates stanzas; reset current worktree.
            current_worktree = None;
        }
    }
    None
}

/// Parse `current_cycle` and `current_step` from STATE.md content.
///
/// Scans the YAML frontmatter block (between `---` fences) for `current_cycle:`
/// and `current_step:` lines. Returns `None` if either field is missing.
///
/// # Note on phantom field
///
/// MUST NOT read `current_wave:` — that is a phantom field (BC-7.07.001 PC1).
///
/// Used by the effectful entry point to derive commit message context (AC-005).
pub fn parse_state_context(state_md_content: &str) -> Option<StateContext> {
    // Normalize CRLF.
    let normalized;
    let content = if state_md_content.contains('\r') {
        normalized = state_md_content.replace("\r\n", "\n");
        normalized.as_str()
    } else {
        state_md_content
    };

    // Find frontmatter region (between first `---\n` and closing `---\n`).
    let after_open = content.strip_prefix("---\n")?;
    let frontmatter_end = after_open.find("\n---\n").or_else(|| {
        if after_open.ends_with("\n---") {
            Some(after_open.len() - 4)
        } else {
            None
        }
    });

    let frontmatter = if let Some(end) = frontmatter_end {
        &after_open[..end]
    } else {
        // No closing fence — scan what we have (fail-open; partial content).
        after_open
    };

    let mut current_cycle: Option<String> = None;
    let mut current_step: Option<String> = None;

    for line in frontmatter.lines() {
        if let Some(val) = line.strip_prefix("current_cycle: ") {
            current_cycle = Some(val.trim().to_string());
        } else if let Some(val) = line.strip_prefix("current_step: ") {
            current_step = Some(val.trim().to_string());
        }
    }

    // SEC-001: strip embedded CR/LF from cycle and step so they cannot corrupt
    // the 4-field log line or the commit subject (injection guard).
    // SEC-003: cap each value to 512 chars to bound commit/log size.
    fn sanitize(s: String) -> String {
        s.replace('\n', " ")
            .replace('\r', "")
            .chars()
            .take(512)
            .collect()
    }

    Some(StateContext {
        current_cycle: sanitize(current_cycle?),
        current_step: sanitize(current_step?),
    })
}

/// Build the commit message for the flush commit (BC-7.07.001 INV4 / AC-005).
///
/// Format: `PreCompact flush <cycle>/<step> <ISO-8601-timestamp>`
///
/// The subject MUST begin with `PreCompact flush ` (capital P, capital C, single
/// space between words, trailing space — case-sensitive per BC-5.41.003 INV3).
///
/// `timestamp` must be in YYYY-MM-DDTHH:MM:SSZ format (UTC, second precision,
/// uppercase Z suffix).
pub fn build_commit_message(ctx: &StateContext, timestamp: &str) -> String {
    format!(
        "{}{}/{} {}",
        COMMIT_PREFIX, ctx.current_cycle, ctx.current_step, timestamp
    )
}

/// Build the log entry line for the precompact-flush-log (AC-007).
///
/// Format: `<ISO-timestamp> <SHA_B> <cycle>/<step> commit\n`
///
/// The trailing `\n` is embedded in the returned string (mandatory per AC-007).
/// Field-4 is the literal string `commit` (never a variable).
pub fn build_log_entry(timestamp: &str, sha_b: &str, ctx: &StateContext) -> String {
    format!(
        "{} {} {}/{} commit\n",
        timestamp, sha_b, ctx.current_cycle, ctx.current_step
    )
}

/// Decide the append-failure action based on CURRENT_HEAD vs SHA_B (AC-008).
///
/// # Semantics
///
/// - If `current_head == sha_b`: safe to reset (`git reset --soft SHA_B^`).
/// - If `current_head != sha_b`: concurrent commit advanced HEAD; do NOT reset.
pub fn decide_append_failure_action(sha_b: &str, current_head: &str) -> AppendFailureAction {
    if current_head == sha_b {
        AppendFailureAction::ResetSafe {
            sha_b: sha_b.to_string(),
        }
    } else {
        AppendFailureAction::NoResetHumanIntervention {
            sha_b: sha_b.to_string(),
            current_head: current_head.to_string(),
        }
    }
}

/// Check whether the staged diff is empty (INV5 clean-state guard, AC-005 / AC-011).
///
/// Returns `true` if the `git diff --cached` output is empty (no staged changes),
/// `false` otherwise.
///
/// When `true` AND `step4_renewal_gate` returned the original un-renewed content, the
/// plugin MUST exit 0 silently without creating an empty commit (BC-7.07.001 INV5).
pub fn is_diff_empty(git_diff_cached_output: &str) -> bool {
    git_diff_cached_output.is_empty()
}

/// Step-4 identity-gate: resolve caller identity, call
/// [`factory_lock::renew_lock_if_holder`], and map the result to the 6
/// BC-7.07.001 outcomes, plus a defensive exhaustiveness wildcard
/// (structurally unreachable per BC-5.40.001) — returning the STATE.md
/// content to use for the flush.
///
/// Pure-core (callback-injectable) per S-17.07 Purity Classification. All
/// effectful operations are injected via closures, enabling all 5 Rust unit
/// tests (AC-001 through AC-005) to run without a WASM runtime or a real
/// subprocess.
///
/// # Parameters
///
/// - `state_md_content` — full STATE.md content (pure content-in; no I/O).
/// - `resolve_identity` — lazy identity resolver (`FnOnce`). Called AT MOST
///   ONCE: only when the lock is present, valid, and not yet expired
///   (AC-002 lazy-call invariant). In production: wraps
///   `host::exec_subprocess("git", &["config", "user.email"])` through
///   `factory_lock::classify_identity_resolution`. In tests: a mock closure
///   with an invocation counter.
/// - `write_state_md` — injectable write for `.factory/STATE.md`. Called ONLY
///   on `Ok((RenewOutcome::Renewed(new_content), None))` to persist the
///   updated `expires_at`. NOT called on any other outcome.
/// - `log_warn_fn` — injectable `host::log_warn`. **MANDATORY** (SHALL, not
///   advisory-optional) on `Err(LockError::Malformed(msg))` per BC-7.07.001
///   PC3 case 1 / EC-004 / Invariant 3 step 3. Also called on
///   `SkipReason::IdentityResolutionFailed`. In tests: a counter closure.
/// - `emit_event_fn` — injectable `host::emit_event`. Called ONLY on
///   `SkipReason::IdentityResolutionFailed` with event type
///   `factory.lock.renewal_indeterminate` and exactly 5 payload fields:
///   `plugin`, `holder`, `locked_at`, `expires_at`, `resolution_error`
///   (ADR-046 Decision 4 / AC-004). In tests: a counter closure.
/// - `now_fn` — injectable clock (`FnOnce() -> DateTime<Utc>`); passed
///   through to `factory_lock::renew_lock_if_holder`. Use `|| Utc::now()`
///   in production.
///
/// # Returns
///
/// The STATE.md content to flush:
/// - `Ok((Renewed(new_content), None))` → write new_content, return
///   new_content (`expires_at` advanced by `TTL_SECONDS`).
/// - All other outcomes → return `state_md_content` unchanged (flush
///   proceeds unblocked per BC-7.07.001 Invariant 3 /
///   flush-proceeds-unblocked invariant).
///
/// # Self-check (BC-5.38.005 invariant 1)
///
/// "If I include this real implementation, will the test for this function
/// pass trivially without any implementer work?" YES — the 6-branch decision
/// tree with conditional I/O callbacks makes this non-trivial.
pub fn step4_renewal_gate<RI, WS, LW, EE, NF>(
    state_md_content: &str,
    resolve_identity: RI,
    write_state_md: WS,
    mut log_warn_fn: LW,
    mut emit_event_fn: EE,
    now_fn: NF,
) -> String
where
    RI: FnOnce() -> IdentityResolution,
    WS: FnOnce(&str) -> Result<(), String>,
    LW: FnMut(&str),
    EE: FnMut(&str, &[(&str, &str)]),
    NF: FnOnce() -> chrono::DateTime<Utc>,
{
    match renew_lock_if_holder(state_md_content, resolve_identity, now_fn) {
        // Case 5 (Success): identity matched and lock unexpired → write renewed content.
        Ok((RenewOutcome::Renewed(new_content), None)) => {
            if let Err(e) = write_state_md(&new_content) {
                // Write failure is advisory — flush proceeds with un-renewed content.
                // Route through log_warn_fn (injectable advisory channel) so the failure
                // is visible to the dispatcher telemetry pipeline. No eprintln! in
                // production paths per CLAUDE.md conventions.
                log_warn_fn(&format!(
                    "precompact-flush: advisory: STATE.md renewal write failed: {}; \
                    proceeding with flush commit using un-renewed content.",
                    e
                ));
                state_md_content.to_string()
            } else {
                new_content
            }
        }

        // Case 4 (IdentityResolutionFailed): emit event + MANDATORY log_warn;
        // flush proceeds unblocked with original content (ADR-046 Decision 4 / AC-004 /
        // BC-7.07.001 Invariant 3b).
        Ok((
            RenewOutcome::NoOp,
            Some(SkipReason::IdentityResolutionFailed {
                reason,
                holder,
                locked_at,
                expires_at,
            }),
        )) => {
            emit_event_fn(
                "factory.lock.renewal_indeterminate",
                &[
                    ("plugin", "precompact-flush"),
                    ("holder", &holder),
                    ("locked_at", &locked_at),
                    ("expires_at", &expires_at),
                    ("resolution_error", &reason),
                ],
            );
            log_warn_fn(&format!(
                "precompact-flush: factory_lock renewal indeterminate: \
                identity resolution failed ({}); holder={}; lock expires {}; \
                flush proceeds with un-renewed content.",
                reason, holder, expires_at
            ));
            state_md_content.to_string()
        }

        // Case 2 (AlreadyExpired): no renewal, flush proceeds.
        Ok((RenewOutcome::NoOp, Some(SkipReason::AlreadyExpired))) => state_md_content.to_string(),

        // Case 3 (NotHolder): no renewal, flush proceeds.
        Ok((RenewOutcome::NoOp, Some(SkipReason::NotHolder))) => state_md_content.to_string(),

        // Case 0 (absent/null block): no renewal, no event, flush proceeds.
        Ok((RenewOutcome::NoOp, None)) => state_md_content.to_string(),

        // Case 1 (Malformed): MANDATORY log_warn (SHALL) per BC-7.07.001 PC3 case 1 /
        // EC-004 / Invariant 3 step 3. resolve_identity is NOT called. expires_at
        // byte-identical. Flush proceeds unblocked.
        Err(LockError::Malformed(msg)) => {
            log_warn_fn(&format!(
                "precompact-flush: advisory: factory_lock block malformed: {}; \
                proceeding with flush commit.",
                msg
            ));
            state_md_content.to_string()
        }

        // Structurally impossible: Renewed always pairs with None per BC-5.40.001.
        // Defensive wildcard required for Rust exhaustiveness.
        Ok((RenewOutcome::Renewed(_), Some(_))) => state_md_content.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Effectful plugin entry point (called from src/main.rs)
// ---------------------------------------------------------------------------

/// Run the precompact-flush plugin logic given a hook payload.
///
/// This is the effectful entry point invoked by the vsdd-hook-sdk trampoline via
/// `src/main.rs`. All host I/O (`read_file`, `write_file`, `exec_subprocess`) is
/// routed through the SDK's host bindings.
///
/// Implements the canonical execution order (BC-7.07.001 INV3 / AC-011).
///
/// Returns `HookResult::Continue` on success or fail-open.
/// Returns `HookResult::Block` on commit/push/append failure (exit 2).
/// Maximum bytes to read per host::read_file call (1 MiB).
const MAX_READ_BYTES: u32 = 1024 * 1024;
/// Maximum bytes to write per host::write_file call (1 MiB, per BC-2.02.011).
const MAX_WRITE_BYTES: u32 = 1024 * 1024;
/// Host I/O timeout in milliseconds for read/write calls.
const IO_TIMEOUT_MS: u32 = 10_000;
/// Timeout in milliseconds for git subprocess calls.
const EXEC_TIMEOUT_MS: u32 = 30_000;
/// Maximum output bytes captured from a git subprocess.
const EXEC_MAX_OUTPUT_BYTES: u32 = 512 * 1024;

pub fn run_plugin(payload: HookPayload) -> HookResult {
    use vsdd_hook_sdk::host;

    let cwd = host::cwd();

    run_plugin_with_mock_and_cwd(
        payload,
        |path| {
            host::read_file(path, MAX_READ_BYTES, IO_TIMEOUT_MS)
                .map_err(|e| format!("read_file error: {e:?}"))
                .and_then(|bytes| {
                    String::from_utf8(bytes).map_err(|e| format!("utf8 decode error: {e}"))
                })
        },
        |path, content| {
            host::write_file(path, content.as_bytes(), MAX_WRITE_BYTES, IO_TIMEOUT_MS)
                .map_err(|e| format!("write_file error: {e:?}"))
        },
        |bin, args| {
            host::exec_subprocess(bin, args, &[], EXEC_TIMEOUT_MS, EXEC_MAX_OUTPUT_BYTES)
                .map(|out| {
                    (
                        out.exit_code,
                        String::from_utf8_lossy(&out.stdout).into_owned(),
                        String::from_utf8_lossy(&out.stderr).into_owned(),
                    )
                })
                .map_err(|e| format!("exec_subprocess error: {e:?}"))
        },
        Some(cwd),
        // Step-4 log_warn: use host::log_warn for structured dispatcher telemetry.
        host::log_warn,
        // Step-4 emit_event: use host::emit_event for factory.lock.renewal_indeterminate.
        host::emit_event,
    )
}

/// Injectable variant of `run_plugin` for unit testing (TDD — mock host I/O).
///
/// Accepts mock closures for host I/O so that the effectful plugin logic can be
/// exercised without a WASM runtime. The implementer MUST implement this function
/// as the testable core of the plugin, with `run_plugin` delegating to it via
/// the real host bindings.
///
/// # Note on path-mismatch check (AC-017)
///
/// In unit tests, `host::cwd()` is not available (returns empty string on non-WASM
/// targets). The path-mismatch check (AC-017 canonicalize assertion) uses
/// `std::env::current_dir()` as the CWD approximation in mock context.
///
/// The mismatch check applies to ALL discovered paths — including paths that do not
/// end with "/.factory". Any path that does not canonicalize to `<cwd>/.factory`
/// triggers DURABILITY DEGRADED + exit 0 (fail-open), so tests that exercise the
/// normal flush flow must provide a worktree path that matches `<cwd>/.factory`
/// (i.e., `<std::env::current_dir()>/.factory`). Use `worktree_path_for_test_cwd()`
/// or the `worktree_list_for(wt)` helper in the integration test suite.
pub fn run_plugin_with_mock<RF, WF, ES>(
    payload: HookPayload,
    read_file: RF,
    write_file: WF,
    exec_subprocess: ES,
) -> HookResult
where
    RF: Fn(&str) -> Result<String, String>,
    WF: Fn(&str, &str) -> Result<(), String>,
    ES: Fn(&str, &[&str]) -> Result<(i32, String, String), String>,
{
    // Determine mock CWD from std::env::current_dir.
    let mock_cwd = std::env::current_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    run_plugin_with_mock_and_cwd(
        payload,
        read_file,
        write_file,
        exec_subprocess,
        Some(mock_cwd),
        // Step-4 log_warn: eprintln in mock context (no WASM host available).
        |msg| eprintln!("precompact-flush: log_warn: {msg}"),
        // Step-4 emit_event: no-op in mock context.
        |_event_type, _fields| {},
    )
}

/// Core implementation shared by `run_plugin` and `run_plugin_with_mock`.
///
/// `cwd` is the project directory (CLAUDE_PROJECT_DIR) used for the
/// canonicalize assertion in AC-017. `None` skips the check.
///
/// `log_warn_fn` and `emit_event_fn` are injectable I/O callbacks for the
/// Step-4 identity-gate (S-17.07). In production: `host::log_warn` and
/// `host::emit_event`. In mock context: eprintln-based / no-op defaults.
fn run_plugin_with_mock_and_cwd<RF, WF, ES, LW, EE>(
    // PreCompact fires unconditionally on every compaction event — no payload dispatch
    // is needed because there is no tool_name or input_content to route on.
    _payload: HookPayload,
    read_file: RF,
    write_file: WF,
    exec_subprocess: ES,
    cwd: Option<String>,
    mut log_warn_fn: LW,
    mut emit_event_fn: EE,
) -> HookResult
where
    RF: Fn(&str) -> Result<String, String>,
    WF: Fn(&str, &str) -> Result<(), String>,
    ES: Fn(&str, &[&str]) -> Result<(i32, String, String), String>,
    LW: FnMut(&str),
    EE: FnMut(&str, &[(&str, &str)]),
{
    // -------------------------------------------------------------------------
    // Step 1: Discover factory-artifacts worktree path via git worktree list --porcelain.
    // (AC-017 / BC-7.07.001 INV3 step 1)
    //
    // This MUST run before STATE.md read per BC-7.07.001 INV3 + ADR-028 §Decision 5.
    // "No step may be reordered." Discovery always runs first; AC-002 (STATE.md
    // unreadable → exit 0) occurs AFTER discovery has completed.
    // -------------------------------------------------------------------------
    let wt_result = exec_subprocess("git", &["worktree", "list", "--porcelain"]);

    let wt_path = match wt_result {
        Err(e) => {
            eprintln!(
                "precompact-flush: DURABILITY DEGRADED — git worktree list command failed ({}); \
                factory-artifacts worktree cannot be discovered; flush SKIPPED this compaction \
                event. Check PATH/git configuration.",
                e
            );
            return HookResult::Continue;
        }
        Ok((exit_code, _, _)) if exit_code != 0 => {
            eprintln!(
                "precompact-flush: DURABILITY DEGRADED — git worktree list command failed \
                (exit {}); factory-artifacts worktree cannot be discovered; flush SKIPPED \
                this compaction event. Check PATH/git configuration.",
                exit_code
            );
            return HookResult::Continue;
        }
        Ok((_, stdout, _)) => match parse_worktree_list(&stdout) {
            Some(path) => path,
            None => {
                eprintln!(
                    "precompact-flush: DURABILITY DEGRADED — factory-artifacts branch not \
                    found in git worktree list output; flush SKIPPED this compaction event. \
                    Ensure the factory-artifacts worktree is mounted at .factory/ (run: \
                    git worktree add .factory factory-artifacts)."
                );
                return HookResult::Continue;
            }
        },
    };

    // -------------------------------------------------------------------------
    // Step 2: AC-017 startup canonicalize assertion (F-R3-001).
    //
    // Per AC-017 / BC-7.07.001 Precondition 4, the mount guard MUST run AFTER
    // worktree discovery (step 1) and BEFORE any I/O or git operations. The
    // Tier-1 structural suffix check gates all paths not ending with "/.factory";
    // Tier-2 canonicalize comparison gates paths that end with "/.factory" but
    // resolve to the wrong physical directory.
    // -------------------------------------------------------------------------

    // Tier 1: structural suffix check (F-002 / AC-017).
    // A discovered path NOT ending with "/.factory" is categorically wrong — it
    // would commit factory-artifacts to a non-standard location. Fail-open.
    if !wt_path.ends_with("/.factory") {
        let expected_raw = cwd
            .as_deref()
            .map(|c| format!("{}/.factory", c.trim_end_matches('/')))
            .unwrap_or_else(|| "<cwd>/.factory".to_string());
        eprintln!(
            "precompact-flush: DURABILITY DEGRADED — factory-artifacts worktree path \
            mismatch: discovered {} but expected {}; flush SKIPPED to prevent split-tree \
            data loss. Ensure factory-artifacts is mounted at .factory/ (run: git worktree \
            add .factory factory-artifacts).",
            wt_path, expected_raw
        );
        return HookResult::Continue;
    }

    // Tier 2: canonicalize comparison (only for paths ending with "/.factory").
    if let Some(ref cwd_str) = cwd {
        let expected_raw = format!("{}/.factory", cwd_str.trim_end_matches('/'));

        // Try canonicalize; fall back to raw string comparison if paths don't exist.
        //
        // The dispatcher canonicalizes CLAUDE_PROJECT_DIR (→ host::cwd()) before
        // passing it to the plugin, so on macOS the symlink /var→/private/var is
        // resolved at the dispatcher level. Both wt_path (from git worktree list,
        // already canonical) and cwd_str (canonicalized by dispatcher) use the same
        // physical path representation, so the raw-string fallback is correct when
        // std::path::Path::canonicalize is unavailable inside the WASM sandbox.
        let mismatch = match (
            std::path::Path::new(&wt_path).canonicalize(),
            std::path::Path::new(cwd_str)
                .join(".factory")
                .canonicalize(),
        ) {
            (Ok(disc), Ok(exp)) => disc != exp,
            (Err(_), _) | (_, Err(_)) => {
                // Canonicalize failed (path doesn't exist on this host, or WASM
                // sandbox doesn't expose the filesystem for absolute path resolution).
                // Fall back to raw string comparison — correct when the dispatcher
                // has already canonicalized cwd_str.
                wt_path != expected_raw
            }
        };

        if mismatch {
            eprintln!(
                "precompact-flush: DURABILITY DEGRADED — factory-artifacts worktree path \
                mismatch: discovered {} but expected {}; flush SKIPPED to prevent split-tree \
                data loss. Ensure factory-artifacts is mounted at .factory/ (run: git worktree \
                add .factory factory-artifacts).",
                wt_path, expected_raw
            );
            return HookResult::Continue;
        }
    }

    // -------------------------------------------------------------------------
    // Step 3: Read STATE.md via host read_file. Exit 0 + warn if unreadable. (AC-002)
    //
    // Per BC-7.07.001 INV3 + ADR-028 §Decision 5, this runs AFTER step 1
    // (worktree discovery) and AFTER step 2 (AC-017 mount guard). The AC-002
    // fail-open path (STATE.md absent/unreadable → exit 0) occurs here.
    // -------------------------------------------------------------------------
    let state_md_content = match read_file(STATE_MD_PATH) {
        Ok(content) => content,
        Err(_) => {
            eprintln!("precompact-flush: STATE.md unreadable; flush skipped.");
            return HookResult::Continue;
        }
    };

    // -------------------------------------------------------------------------
    // Step 4: identity-gated renewal via step4_renewal_gate / renew_lock_if_holder
    // (S-17.07 / AC-003, AC-018 / ADR-046 Decision 3).
    //
    // resolve_identity is lazy: called at most once, only when lock is present,
    // valid, and not expired (AC-002 lazy-call invariant).
    // -------------------------------------------------------------------------
    let resolve_identity = || {
        classify_identity_resolution(
            exec_subprocess("git", &["config", "user.email"])
                .map(|(exit, stdout, _stderr)| (exit, stdout)),
        )
    };
    let write_state_md_fn = |new_content: &str| write_file(STATE_MD_PATH, new_content);

    let flush_content = step4_renewal_gate(
        &state_md_content,
        resolve_identity,
        write_state_md_fn,
        &mut log_warn_fn,
        &mut emit_event_fn,
        Utc::now,
    );

    // Track whether renewal produced new content (for INV5 decision in step 6).
    let was_renewed = flush_content != state_md_content;

    // -------------------------------------------------------------------------
    // Step 6a: git -C <wt> add -A — stage ALL changes including new untracked files.
    // (AC-004 / ADR-028 §Decision 15 F-R3-003)
    //
    // A non-zero exit code (e.g., index lock, pathspec error) is a hard local
    // failure that MUST block compaction — staging is a prerequisite for the
    // durability commit (AC-005b local-failure-exit-2 policy / AC-004).
    // -------------------------------------------------------------------------
    match exec_subprocess("git", &["-C", &wt_path, "add", "-A"]) {
        Ok((exit_code, _, stderr)) if exit_code != 0 => {
            eprintln!(
                "precompact-flush: git add -A failed (exit {}): {}; blocking compaction.",
                exit_code, stderr
            );
            return HookResult::Block {
                reason: format!(
                    "precompact-flush: git add -A failed (exit {}): {}",
                    exit_code, stderr
                ),
            };
        }
        Err(e) => {
            eprintln!(
                "precompact-flush: git add -A failed: {}; blocking compaction.",
                e
            );
            return HookResult::Block {
                reason: format!("precompact-flush: git add -A failed: {}", e),
            };
        }
        Ok(_) => {
            // Staging succeeded.
        }
    }

    // -------------------------------------------------------------------------
    // Step 6b: check git diff --cached. If RenewOutcome::NoOp AND diff empty →
    // INV5 clean-state → exit 0 silently. (AC-005 / BC-7.07.001 INV5)
    // -------------------------------------------------------------------------
    let diff_output = match exec_subprocess("git", &["-C", &wt_path, "diff", "--cached"]) {
        Ok((_, stdout, _)) => stdout,
        Err(e) => {
            // diff --cached subprocess failure (e.g., binary not in sandbox allow-list,
            // permission denied). Fail-open: emit the error to stderr but do NOT fabricate
            // a "non-empty" sentinel that would force a spurious commit (INV5 violation).
            // A potentially-clean worktree must not be committed without verified staged
            // changes. (AC-005 / BC-7.07.001 INV5 / F-004)
            eprintln!(
                "precompact-flush: git diff --cached failed: {}; flush skipped (fail-open).",
                e
            );
            return HookResult::Continue;
        }
    };

    if !was_renewed && is_diff_empty(&diff_output) {
        // INV5: no renewal + no staged changes → clean state → exit 0.
        return HookResult::Continue;
    }

    // -------------------------------------------------------------------------
    // Step 7: git -C <wt> commit -m <msg>. Exit 2 on LOCAL failure. (AC-005b)
    // -------------------------------------------------------------------------
    let timestamp = Utc::now().format("%Y-%m-%dT%H:%M:%SZ").to_string();
    let ctx = parse_state_context(&state_md_content).unwrap_or_else(|| StateContext {
        current_cycle: "unknown-cycle".to_string(),
        current_step: "unknown-step".to_string(),
    });
    let commit_msg = build_commit_message(&ctx, &timestamp);

    match exec_subprocess("git", &["-C", &wt_path, "commit", "-m", &commit_msg]) {
        Ok((exit_code, _, stderr)) if exit_code != 0 => {
            eprintln!(
                "precompact-flush: git commit failed (exit {}): {}; blocking compaction.",
                exit_code, stderr
            );
            return HookResult::Block {
                reason: format!(
                    "precompact-flush: git commit failed (exit {}): {}",
                    exit_code, stderr
                ),
            };
        }
        Err(e) => {
            eprintln!(
                "precompact-flush: git commit failed: {}; blocking compaction.",
                e
            );
            return HookResult::Block {
                reason: format!("precompact-flush: git commit failed: {}", e),
            };
        }
        Ok(_) => {
            // Commit succeeded.
        }
    }

    // -------------------------------------------------------------------------
    // Step 8: SHA_B = git -C <wt> rev-parse HEAD — IMMEDIATELY after commit,
    // BEFORE append. (AC-006 / BC-7.07.001 PC8)
    // -------------------------------------------------------------------------
    let sha_b = match exec_subprocess("git", &["-C", &wt_path, "rev-parse", "HEAD"]) {
        Ok((0, stdout, _)) => {
            let sha = stdout.trim().to_string();
            debug_assert!(
                sha.chars().all(|c| c.is_ascii_hexdigit()),
                "rev-parse HEAD produced non-hex output: {sha:?}"
            );
            sha
        }
        Ok((code, _, stderr)) => {
            eprintln!(
                "precompact-flush: rev-parse HEAD failed (exit {}): {}; blocking compaction.",
                code, stderr
            );
            return HookResult::Block {
                reason: format!(
                    "precompact-flush: rev-parse HEAD failed (exit {}): {}",
                    code, stderr
                ),
            };
        }
        Err(e) => {
            eprintln!(
                "precompact-flush: rev-parse HEAD failed: {}; blocking compaction.",
                e
            );
            return HookResult::Block {
                reason: format!("precompact-flush: rev-parse HEAD failed: {}", e),
            };
        }
    };

    // -------------------------------------------------------------------------
    // Step 9: Append 4-field \n-terminated line to precompact-flush-log.
    // Treat absent-log read error as empty baseline. (AC-007 / ADR-028 §Decision 12)
    // -------------------------------------------------------------------------
    let log_entry = build_log_entry(&timestamp, &sha_b, &ctx);

    // Read existing log content; absent file → empty baseline (F-NW2-008).
    let existing_log = read_file(LOG_PATH).unwrap_or_default();
    let new_log = format!("{}{}", existing_log, log_entry);

    if let Err(e) = write_file(LOG_PATH, &new_log) {
        // Append failed — SHA-pinned concurrent-commit guard. (AC-008)
        eprintln!("precompact-flush: log append to {} failed: {}", LOG_PATH, e);

        // Check CURRENT_HEAD vs SHA_B.
        let current_head = exec_subprocess("git", &["-C", &wt_path, "rev-parse", "HEAD"])
            .map(|(_, stdout, _)| stdout.trim().to_string())
            .unwrap_or_default();

        return match decide_append_failure_action(&sha_b, &current_head) {
            AppendFailureAction::ResetSafe { sha_b: ref sb } => {
                let reset_ref = format!("{}^", sb);
                match exec_subprocess("git", &["-C", &wt_path, "reset", "--soft", &reset_ref]) {
                    Ok((0, _, _)) => {
                        eprintln!(
                            "precompact-flush: SHA append to precompact-flush-log failed; \
                            orphan commit reverted (SHA_B={}); blocking compaction.",
                            sb
                        );
                        HookResult::Block {
                            reason: format!(
                                "precompact-flush: SHA append to precompact-flush-log failed; \
                                orphan commit reverted (SHA_B={}); blocking compaction.",
                                sb
                            ),
                        }
                    }
                    _ => {
                        eprintln!(
                            "precompact-flush: SHA append failed AND reset failed; \
                            human intervention required (SHA_B={}).",
                            sb
                        );
                        HookResult::Block {
                            reason: format!(
                                "precompact-flush: SHA append to precompact-flush-log failed \
                                AND orphan-commit reset failed (SHA_B={}); \
                                manual intervention required; compaction blocked.",
                                sb
                            ),
                        }
                    }
                }
            }
            AppendFailureAction::NoResetHumanIntervention {
                sha_b: ref sb,
                current_head: ref ch,
            } => {
                eprintln!(
                    "precompact-flush: append failed; concurrent commit advanced HEAD; \
                    SHA_B={}; current HEAD={}; human intervention required.",
                    sb, ch
                );
                HookResult::Block {
                    reason: format!(
                        "precompact-flush: append failed; concurrent commit advanced HEAD; \
                        SHA_B={}; current HEAD={}; human intervention required.",
                        sb, ch
                    ),
                }
            }
        };
    }

    // -------------------------------------------------------------------------
    // Step 11: git -C <wt> push origin factory-artifacts. (AC-009)
    // -------------------------------------------------------------------------
    match exec_subprocess("git", &["-C", &wt_path, "push", PUSH_REMOTE, PUSH_BRANCH]) {
        Ok((exit_code, _, stderr)) if exit_code != 0 => {
            eprintln!(
                "precompact-flush: git push failed (exit {}): {}; \
                local commit {} and log entry intact; retry is push-only.",
                exit_code, stderr, sha_b
            );
            HookResult::Block {
                reason: format!(
                    "precompact-flush: git push failed; local commit {} and log entry intact; \
                    retry is push-only.",
                    sha_b
                ),
            }
        }
        Err(e) => {
            eprintln!(
                "precompact-flush: git push failed: {}; \
                local commit {} and log entry intact; retry is push-only.",
                e, sha_b
            );
            HookResult::Block {
                reason: format!(
                    "precompact-flush: git push failed; local commit {} and log entry intact; \
                    retry is push-only.",
                    sha_b
                ),
            }
        }
        Ok(_) => {
            // Push succeeded → exit 0. (AC-009 / BC-7.07.001 PC5)
            HookResult::Continue
        }
    }
}

// ---------------------------------------------------------------------------
// S-17.07 unit tests — step4_renewal_gate (BC-7.07.001 PC3 / Invariants 3/3b)
//
// All 5 tests exercise the implemented step4_renewal_gate function (S-17.07
// complete). Test naming follows the Red Gate Test Table in S-17.07 v1.2
// (authoritative). Each test uses injected counter/capture closures — no WASM
// runtime needed.
// ---------------------------------------------------------------------------
#[cfg(test)]
mod step4_tests {
    #![allow(
        clippy::expect_used,
        clippy::unwrap_used,
        clippy::panic,
        clippy::type_complexity
    )]

    use super::step4_renewal_gate;
    use chrono::{DateTime, Utc};
    use factory_lock::IdentityResolution;
    use std::sync::{Arc, Mutex};

    // -----------------------------------------------------------------------
    // Fixtures — portable STATE.md content strings (all \n; no platform-specific
    // terminators, paths, or separators — PG-CI-2 cross-platform portability).
    // -----------------------------------------------------------------------

    /// STATE.md with expired factory_lock (expires_at 2020, now_2026 > 2020 → AlreadyExpired).
    fn fixture_expired_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "factory_lock:\n",
            "  holder: \"holder@example.com\"\n",
            "  locked_at: \"2020-01-01T10:00:00Z\"\n",
            "  expires_at: \"2020-01-01T10:45:00Z\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// STATE.md with valid unexpired factory_lock (expires_at 2099, now_2026 < 2099 → identity step).
    /// holder = "holder@example.com"
    fn fixture_valid_unexpired_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "factory_lock:\n",
            "  holder: \"holder@example.com\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"2099-01-01T10:45:00Z\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// STATE.md with malformed factory_lock (holder = "" → Err(LockError::Malformed)).
    /// factory_lock: key IS present; block is structurally malformed.
    fn fixture_malformed_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "factory_lock:\n",
            "  holder: \"\"\n",
            "  locked_at: \"2026-01-01T10:00:00Z\"\n",
            "  expires_at: \"2026-01-01T10:45:00Z\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// STATE.md with NO factory_lock key (absent lock — 0th case, Ok((NoOp, None))).
    fn fixture_no_lock() -> &'static str {
        concat!(
            "---\n",
            "document_type: state\n",
            "version: \"test\"\n",
            "---\n\n# STATE\n",
        )
    }

    /// Injectable clock returning 2026-08-27T12:00:00Z.
    /// After expired fixture (2020) but before valid fixture (2099): discriminates cases.
    /// now_2026() + 2700s = 2026-08-27T12:45:00Z (expected renewed expires_at for AC-002).
    fn now_2026() -> DateTime<Utc> {
        "2026-08-27T12:00:00Z"
            .parse::<DateTime<Utc>>()
            .expect("fixture timestamp must parse")
    }

    // -----------------------------------------------------------------------
    // AC-001: AlreadyExpired → NO resolve_identity call (count==0), flush returns
    //         ORIGINAL content, no write_state_md call.
    //
    // Verifies: step4_renewal_gate returns original content and does NOT invoke
    // resolve_identity or write_state_md on the AlreadyExpired path
    // (BC-7.07.001 PC3 Case 2).
    // -----------------------------------------------------------------------

    /// test_BC_7_07_001_AC001 — AlreadyExpired path: no exec subprocess, original content returned.
    #[test]
    fn test_precompact_flush_step4_already_expired_no_exec_subprocess() {
        let content = fixture_expired_lock();

        let resolve_identity_calls = Arc::new(Mutex::new(0u32));
        let ri_count = resolve_identity_calls.clone();

        let write_state_md_calls = Arc::new(Mutex::new(0u32));
        let ws_count = write_state_md_calls.clone();

        let result = step4_renewal_gate(
            content,
            // resolve_identity: MUST NOT be called on AlreadyExpired path.
            move || -> IdentityResolution {
                *ri_count.lock().unwrap() += 1;
                IdentityResolution::Resolved("holder@example.com".to_string())
            },
            // write_state_md: MUST NOT be called (no renewal on AlreadyExpired).
            move |_new_content: &str| -> Result<(), String> {
                *ws_count.lock().unwrap() += 1;
                Ok(())
            },
            |_msg: &str| {},
            |_event: &str, _fields: &[(&str, &str)]| {},
            now_2026,
        );

        // AC-001: resolve_identity MUST NOT be called — no exec subprocess on AlreadyExpired.
        assert_eq!(
            *resolve_identity_calls.lock().unwrap(),
            0,
            "AC-001: resolve_identity must NOT be called for AlreadyExpired \
            (no git config user.email exec subprocess)"
        );
        // AC-001: write_state_md MUST NOT be called (no renewal).
        assert_eq!(
            *write_state_md_calls.lock().unwrap(),
            0,
            "AC-001: write_state_md must NOT be called on AlreadyExpired path"
        );
        // AC-001: flush returns the original un-renewed content.
        assert_eq!(
            result, content,
            "AC-001: flush content must be the original un-renewed STATE.md on AlreadyExpired"
        );
    }

    // -----------------------------------------------------------------------
    // AC-002: identity match → write_state_md called with RENEWED content,
    //         expires_at advanced by TTL, flush returns renewed content.
    //
    // Verifies: step4_renewal_gate calls write_state_md once with expires_at
    // advanced by TTL and returns the renewed content (BC-7.07.001 PC3 Case 5).
    // -----------------------------------------------------------------------

    /// test_BC_7_07_001_AC002 — identity match: write_state_md called, expires_at advanced.
    #[test]
    fn test_precompact_flush_step4_identity_match_renews_content() {
        let content = fixture_valid_unexpired_lock();

        let write_state_md_calls = Arc::new(Mutex::new(0u32));
        let ws_count = write_state_md_calls.clone();
        let written_content: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
        let wct = written_content.clone();

        let result = step4_renewal_gate(
            content,
            // resolve_identity: identity matches holder → Renewed outcome.
            || IdentityResolution::Resolved("holder@example.com".to_string()),
            // write_state_md: MUST be called once with renewed content.
            move |new_content: &str| -> Result<(), String> {
                *ws_count.lock().unwrap() += 1;
                *wct.lock().unwrap() = Some(new_content.to_string());
                Ok(())
            },
            |_msg: &str| {},
            |_event: &str, _fields: &[(&str, &str)]| {},
            now_2026,
        );

        // AC-002: write_state_md MUST be called exactly once with renewed content.
        assert_eq!(
            *write_state_md_calls.lock().unwrap(),
            1,
            "AC-002: write_state_md must be called exactly once on identity match"
        );
        // AC-002: written content must contain expires_at advanced by TTL.
        // now_2026() = 2026-08-27T12:00:00Z; + 2700s = 2026-08-27T12:45:00Z.
        {
            let guard = written_content.lock().unwrap();
            let written_str = guard
                .as_ref()
                .expect("AC-002: write_state_md must have been called with renewed content");
            assert!(
                written_str.contains("2026-08-27T12:45:00Z"),
                "AC-002: written content must contain expires_at advanced by TTL \
                (2026-08-27T12:45:00Z), content: {}",
                written_str
            );
            // AC-002: flush returns the same renewed content.
            assert_eq!(
                result, *written_str,
                "AC-002: step4_renewal_gate must return the renewed content \
                (same value as passed to write_state_md)"
            );
        }
        // AC-002: original stale expires_at must not survive in renewed output.
        assert!(
            !result.contains("2099-01-01T10:45:00Z"),
            "AC-002: renewed content must NOT still contain the stale fixture \
            expires_at (2099-01-01T10:45:00Z)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-003: NotHolder → no renewal, expires_at byte-identical, flush returns
    //         original content, no abort.
    //
    // Verifies: step4_renewal_gate returns original content with expires_at
    // byte-identical and does NOT call write_state_md (BC-7.07.001 PC3 Case 3).
    // -----------------------------------------------------------------------

    /// test_BC_7_07_001_AC003 — not holder: expires_at unchanged, flush proceeds.
    #[test]
    fn test_precompact_flush_step4_not_holder_no_renewal() {
        let content = fixture_valid_unexpired_lock();

        let write_state_md_calls = Arc::new(Mutex::new(0u32));
        let ws_count = write_state_md_calls.clone();

        let result = step4_renewal_gate(
            content,
            // resolve_identity: resolves to a different identity — not the holder.
            || IdentityResolution::Resolved("other@example.com".to_string()),
            // write_state_md: MUST NOT be called (not holder → no renewal).
            move |_new_content: &str| -> Result<(), String> {
                *ws_count.lock().unwrap() += 1;
                Ok(())
            },
            |_msg: &str| {},
            |_event: &str, _fields: &[(&str, &str)]| {},
            now_2026,
        );

        // AC-003: write_state_md MUST NOT be called on NotHolder path.
        assert_eq!(
            *write_state_md_calls.lock().unwrap(),
            0,
            "AC-003: write_state_md must NOT be called on NotHolder path"
        );
        // AC-003: flush returns original content — expires_at byte-identical.
        assert_eq!(
            result, content,
            "AC-003: flush content must be the original un-renewed STATE.md on NotHolder"
        );
        // AC-003: expires_at must be byte-identical — original value still present.
        assert!(
            result.contains("2099-01-01T10:45:00Z"),
            "AC-003: expires_at must be byte-identical (original 2099-01-01T10:45:00Z \
            must still appear in the flushed content)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-004: IdentityResolutionFailed → emit_event_fn called with
    //         factory.lock.renewal_indeterminate + 5-field payload,
    //         log_warn_fn called, flush proceeds with original content.
    //
    // Verifies: step4_renewal_gate emits event + log_warn and returns original
    // content without renewal on IdentityResolutionFailed
    // (BC-7.07.001 PC3 Case 4 / ADR-046 Decision 4).
    // -----------------------------------------------------------------------

    /// test_BC_7_07_001_AC004 — IdentityResolutionFailed: event emitted, log_warn called.
    #[test]
    fn test_precompact_flush_step4_resolution_failed_emits_event_and_logs() {
        let content = fixture_valid_unexpired_lock();

        let emit_event_calls = Arc::new(Mutex::new(0u32));
        let ec = emit_event_calls.clone();
        let emitted_events: Arc<Mutex<Vec<(String, Vec<(String, String)>)>>> =
            Arc::new(Mutex::new(Vec::new()));
        let ev = emitted_events.clone();

        let log_warn_calls = Arc::new(Mutex::new(0u32));
        let wc = log_warn_calls.clone();

        let write_state_md_calls = Arc::new(Mutex::new(0u32));
        let wrt = write_state_md_calls.clone();

        let result = step4_renewal_gate(
            content,
            // resolve_identity: resolution fails → IdentityResolutionFailed outcome.
            || IdentityResolution::Failed("git config user.email failed (exit 1)".to_string()),
            // write_state_md: MUST NOT be called (no renewal on IdentityResolutionFailed).
            move |_new_content: &str| -> Result<(), String> {
                *wrt.lock().unwrap() += 1;
                Ok(())
            },
            // log_warn_fn: MUST be called on IdentityResolutionFailed.
            move |_msg: &str| {
                *wc.lock().unwrap() += 1;
            },
            // emit_event_fn: MUST be called with factory.lock.renewal_indeterminate + 5 fields.
            move |event_type: &str, fields: &[(&str, &str)]| {
                *ec.lock().unwrap() += 1;
                ev.lock().unwrap().push((
                    event_type.to_string(),
                    fields
                        .iter()
                        .map(|(k, v)| (k.to_string(), v.to_string()))
                        .collect(),
                ));
            },
            now_2026,
        );

        // AC-004: emit_event_fn must be called exactly once.
        assert_eq!(
            *emit_event_calls.lock().unwrap(),
            1,
            "AC-004: emit_event_fn must be called exactly once on IdentityResolutionFailed"
        );

        // AC-004: verify event type and 5-field payload.
        // (use a nested block so the MutexGuard is released before further lock calls)
        {
            let events = emitted_events.lock().unwrap();
            let (event_type, fields) = events
                .first()
                .expect("AC-004: at least one event must have been emitted");
            assert_eq!(
                event_type, "factory.lock.renewal_indeterminate",
                "AC-004: event type must be factory.lock.renewal_indeterminate, got: {:?}",
                event_type
            );
            // AC-004: exactly 5 fields required (ADR-046 Decision 4 — fewer is a BC violation).
            assert_eq!(
                fields.len(),
                5,
                "AC-004: event payload must have exactly 5 fields (ADR-046 Decision 4), \
                got {:?}: {:?}",
                fields.len(),
                fields
            );
            // AC-004: assert ORDER + VALUES — POLICY 12 makes field order and
            // value-sourcing contractual (F-P3-001). The fixture is
            // fixture_valid_unexpired_lock(); the injected reason string is
            // "git config user.email failed (exit 1)".
            assert_eq!(
                fields[0],
                ("plugin".to_string(), "precompact-flush".to_string()),
                "AC-004: fields[0] must be (\"plugin\", \"precompact-flush\") — \
                ORDER+VALUE contractual per POLICY 12 (F-P3-001); got: {:?}",
                fields[0]
            );
            assert_eq!(
                fields[1],
                ("holder".to_string(), "holder@example.com".to_string()),
                "AC-004: fields[1] must be (\"holder\", \"holder@example.com\") \
                from fixture_valid_unexpired_lock (F-P3-001); got: {:?}",
                fields[1]
            );
            assert_eq!(
                fields[2],
                ("locked_at".to_string(), "2026-01-01T10:00:00Z".to_string()),
                "AC-004: fields[2] must be (\"locked_at\", \"2026-01-01T10:00:00Z\") \
                from fixture_valid_unexpired_lock (F-P3-001); got: {:?}",
                fields[2]
            );
            assert_eq!(
                fields[3],
                ("expires_at".to_string(), "2099-01-01T10:45:00Z".to_string()),
                "AC-004: fields[3] must be (\"expires_at\", \"2099-01-01T10:45:00Z\") \
                from fixture_valid_unexpired_lock (F-P3-001); got: {:?}",
                fields[3]
            );
            assert_eq!(
                fields[4],
                (
                    "resolution_error".to_string(),
                    "git config user.email failed (exit 1)".to_string()
                ),
                "AC-004: fields[4] must be (\"resolution_error\", \
                \"git config user.email failed (exit 1)\") — injected reason string \
                (F-P3-001); got: {:?}",
                fields[4]
            );
        } // MutexGuard on emitted_events released here

        // AC-004: log_warn_fn must be called.
        assert!(
            *log_warn_calls.lock().unwrap() >= 1,
            "AC-004: log_warn_fn must be called on IdentityResolutionFailed"
        );
        // AC-004: write_state_md must NOT be called (no renewal on IdentityResolutionFailed).
        assert_eq!(
            *write_state_md_calls.lock().unwrap(),
            0,
            "AC-004: write_state_md must NOT be called on IdentityResolutionFailed"
        );
        // AC-004: flush proceeds with original un-renewed content.
        assert_eq!(
            result, content,
            "AC-004: flush content must be the original un-renewed STATE.md \
            on IdentityResolutionFailed"
        );
    }

    // -----------------------------------------------------------------------
    // AC-005: PRIMARY — Err(LockError::Malformed(msg)):
    //           log_warn_fn called exactly once (MANDATORY SHALL),
    //           resolve_identity NOT called (count==0),
    //           write_state_md NOT called (expires_at byte-identical),
    //           flush proceeds.
    //
    //         SECONDARY — Ok((RenewOutcome::NoOp, None)) absent lock:
    //           resolve_identity NOT called, no event emitted, flush proceeds.
    //
    // Verifies: step4_renewal_gate emits MANDATORY log_warn (and no event) on
    // Malformed; and neither calls resolve_identity nor write_state_md on
    // absent lock (BC-7.07.001 PC3 Case 1 / EC-004 / Case 0).
    // -----------------------------------------------------------------------

    /// test_BC_7_07_001_AC005 — Malformed + absent-lock: MANDATORY log_warn, no exec, no write.
    #[test]
    fn test_precompact_flush_step4_malformed_lock_emits_log_warn_no_exec() {
        // ------------------------------------------------------------------
        // PRIMARY sub-case: Err(LockError::Malformed(msg))
        // factory_lock: key present, holder="" → parse_factory_lock returns
        // Err(MalformedLockBlock) → renew_lock_if_holder returns Err(LockError::Malformed).
        // ------------------------------------------------------------------
        {
            let content = fixture_malformed_lock();

            let resolve_identity_calls = Arc::new(Mutex::new(0u32));
            let ri_count = resolve_identity_calls.clone();

            let log_warn_calls = Arc::new(Mutex::new(0u32));
            let lw_count = log_warn_calls.clone();

            let write_state_md_calls = Arc::new(Mutex::new(0u32));
            let ws_count = write_state_md_calls.clone();

            let emit_event_calls = Arc::new(Mutex::new(0u32));
            let ec = emit_event_calls.clone();

            let result = step4_renewal_gate(
                content,
                // resolve_identity: MUST NOT be called on Malformed path.
                move || -> IdentityResolution {
                    *ri_count.lock().unwrap() += 1;
                    IdentityResolution::Resolved("holder@example.com".to_string())
                },
                // write_state_md: MUST NOT be called (expires_at byte-identical on Malformed).
                move |_new_content: &str| -> Result<(), String> {
                    *ws_count.lock().unwrap() += 1;
                    Ok(())
                },
                // log_warn_fn: MANDATORY (SHALL) — must be called exactly once.
                // BC-7.07.001 PC3 case 1 / EC-004 / Invariant 3 step 3.
                move |_msg: &str| {
                    *lw_count.lock().unwrap() += 1;
                },
                move |_event: &str, _fields: &[(&str, &str)]| {
                    *ec.lock().unwrap() += 1;
                },
                now_2026,
            );

            // AC-005 PRIMARY: log_warn_fn MUST be called exactly once.
            // This is MANDATORY (SHALL) per BC-7.07.001 PC3 case 1 / EC-004 / Invariant 3 step 3.
            // Failing this assertion is the blocker that F2 identified as missing.
            assert_eq!(
                *log_warn_calls.lock().unwrap(),
                1,
                "AC-005 PRIMARY: log_warn_fn must be called exactly once on \
                Err(LockError::Malformed) — this is MANDATORY (SHALL), not optional \
                (BC-7.07.001 PC3 case 1 / EC-004 / Invariant 3 step 3)"
            );
            // AC-005 PRIMARY: resolve_identity MUST NOT be called.
            assert_eq!(
                *resolve_identity_calls.lock().unwrap(),
                0,
                "AC-005 PRIMARY: resolve_identity must NOT be called on Malformed path \
                (no exec subprocess; AC-002 lazy-call invariant)"
            );
            // AC-005 PRIMARY: write_state_md MUST NOT be called (expires_at byte-identical).
            assert_eq!(
                *write_state_md_calls.lock().unwrap(),
                0,
                "AC-005 PRIMARY: write_state_md must NOT be called on Malformed path \
                (expires_at must remain byte-identical)"
            );
            // AC-005 PRIMARY: flush proceeds with original content.
            assert_eq!(
                result, content,
                "AC-005 PRIMARY: flush must proceed with original content on Malformed path \
                (must not abort or exit 2)"
            );
            // AC-005 PRIMARY: NO factory.lock.renewal_indeterminate event must be emitted
            // on Malformed arm (BC-7.07.001 Invariant 3b / PC3 case 1 mandate NO event
            // on Malformed; event is ONLY for IdentityResolutionFailed — O-1).
            assert_eq!(
                *emit_event_calls.lock().unwrap(),
                0,
                "AC-005 PRIMARY: no factory.lock.renewal_indeterminate event must be emitted \
                on Malformed arm (BC-7.07.001 Invariant 3b / PC3 case 1 — event is exclusive \
                to IdentityResolutionFailed path; O-1)"
            );
        }

        // ------------------------------------------------------------------
        // SECONDARY sub-case: Ok((RenewOutcome::NoOp, None)) — absent lock.
        // factory_lock: key absent → renew_lock_if_holder returns Ok((NoOp, None)).
        // ------------------------------------------------------------------
        {
            let content = fixture_no_lock();

            let resolve_identity_calls = Arc::new(Mutex::new(0u32));
            let ri_count = resolve_identity_calls.clone();

            let emit_event_calls = Arc::new(Mutex::new(0u32));
            let ec = emit_event_calls.clone();

            let write_state_md_calls = Arc::new(Mutex::new(0u32));
            let ws_count = write_state_md_calls.clone();

            let result = step4_renewal_gate(
                content,
                // resolve_identity: MUST NOT be called on absent lock (0th case).
                move || -> IdentityResolution {
                    *ri_count.lock().unwrap() += 1;
                    IdentityResolution::Resolved("holder@example.com".to_string())
                },
                move |_new_content: &str| -> Result<(), String> {
                    *ws_count.lock().unwrap() += 1;
                    Ok(())
                },
                |_msg: &str| {},
                // emit_event_fn: MUST NOT be called on absent lock.
                move |_event: &str, _fields: &[(&str, &str)]| {
                    *ec.lock().unwrap() += 1;
                },
                now_2026,
            );

            // AC-005 SECONDARY: resolve_identity MUST NOT be called on absent lock.
            assert_eq!(
                *resolve_identity_calls.lock().unwrap(),
                0,
                "AC-005 SECONDARY: resolve_identity must NOT be called on absent lock \
                (0th case; BC-7.07.001 PC3 0th case / EC-009)"
            );
            // AC-005 SECONDARY: no event must be emitted.
            assert_eq!(
                *emit_event_calls.lock().unwrap(),
                0,
                "AC-005 SECONDARY: no event must be emitted on absent lock path"
            );
            // AC-005 SECONDARY: no write to expires_at.
            assert_eq!(
                *write_state_md_calls.lock().unwrap(),
                0,
                "AC-005 SECONDARY: write_state_md must NOT be called on absent lock path"
            );
            // AC-005 SECONDARY: flush proceeds with original content.
            assert_eq!(
                result, content,
                "AC-005 SECONDARY: flush must proceed with original content on absent lock"
            );
        }
    }

    // -----------------------------------------------------------------------
    // O-2: Renewed arm write-failure routes through log_warn_fn (advisory channel)
    //      and returns un-renewed content (fail-open semantics preserved).
    //
    // Traces to: step4_renewal_gate Renewed arm write-failure branch (O-2 finding).
    // BC-7.07.001 does not mandate a specific behavior for renewal-write-failure —
    // fail-open-with-advisory is the production-grade choice.
    // -----------------------------------------------------------------------

    /// test_step4_renewed_write_failure_routes_log_warn_and_returns_original
    ///
    /// When `write_state_md` returns `Err(...)` on the `Renewed` arm:
    /// - `log_warn_fn` MUST be called (advisory emitted to dispatcher telemetry).
    /// - The returned content MUST be the original un-renewed STATE.md.
    /// - Flush proceeds (fail-open — this test exercises the branch;
    ///   caller tests verify the flush continues to commit/push).
    ///
    /// This closes the untested branch identified in the O-2 finding.
    #[test]
    fn test_step4_renewed_write_failure_routes_log_warn_and_returns_original() {
        let content = fixture_valid_unexpired_lock();

        let log_warn_calls = Arc::new(Mutex::new(0u32));
        let lw_count = log_warn_calls.clone();
        let logged_messages: Arc<Mutex<Vec<String>>> = Arc::new(Mutex::new(Vec::new()));
        let lm = logged_messages.clone();

        let result = step4_renewal_gate(
            content,
            // resolve_identity: identity matches holder → Renewed outcome attempted.
            || IdentityResolution::Resolved("holder@example.com".to_string()),
            // write_state_md: returns Err — simulates a filesystem write failure.
            |_new_content: &str| -> Result<(), String> {
                Err("write_file: CAPABILITY_DENIED: disk full".to_string())
            },
            // log_warn_fn: MUST be called when write_state_md fails on Renewed arm.
            move |msg: &str| {
                *lw_count.lock().unwrap() += 1;
                lm.lock().unwrap().push(msg.to_string());
            },
            |_event: &str, _fields: &[(&str, &str)]| {},
            now_2026,
        );

        // O-2: log_warn_fn MUST be called exactly once (advisory channel, not eprintln!).
        assert_eq!(
            *log_warn_calls.lock().unwrap(),
            1,
            "O-2: log_warn_fn must be called exactly once when write_state_md fails on \
            Renewed arm (advisory channel — NOT eprintln! / raw stderr)"
        );

        // O-2: the advisory message must mention the failure.
        {
            let msgs = logged_messages.lock().unwrap();
            let msg = msgs
                .first()
                .expect("O-2: log_warn_fn must have received a message");
            assert!(
                msg.contains("advisory") || msg.contains("write failed") || msg.contains("renewal"),
                "O-2: advisory message must describe the write failure; got: {msg:?}"
            );
        }

        // O-2: fail-open — return original (un-renewed) content, not the new content.
        assert_eq!(
            result, content,
            "O-2: step4_renewal_gate must return original un-renewed content when \
            write_state_md fails (fail-open semantics preserved)"
        );
    }
}
