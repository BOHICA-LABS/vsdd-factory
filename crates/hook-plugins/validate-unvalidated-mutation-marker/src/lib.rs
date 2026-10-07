//! validate-unvalidated-mutation-marker — Two-arm next-advance gate WASM hook plugin.
//!
//! Implements the quarantine gate for the INDETERMINATE outcome class (S-25.01 Layer-1,
//! ADR-047). Two `[[hooks]]` registry entries reference this SINGLE compiled WASM binary:
//!
//! - **Arm 1** (`validate-unvalidated-mutation-marker`, `tool = "^Agent$"`): blocks Agent
//!   dispatches while `.factory/unvalidated-mutation.marker` exists (BC-1.18.002 PC1).
//! - **Arm 2** (`validate-unvalidated-mutation-marker-git`, `tool = "^Bash$"`): blocks
//!   `git commit` and `git push` Bash dispatches while the marker exists (BC-1.18.002 PC2).
//!
//! Both arms use the same WASM binary per BC-1.18.002 invariant 1. The dispatch routing
//! between Arm 1 and Arm 2 is plugin-internal: the plugin reads the `tool` field (or
//! `command` field for Bash) from the PreToolUse payload to determine which arm applies.
//!
//! # Self-lock prevention (BC-1.18.002 invariant 2)
//!
//! Both hooks-registry.toml entries for this plugin MUST have `failure_policy = "fail-open"`.
//! If this plugin itself crashes or fuel-exhausts, it DOES NOT write a new marker — the gate
//! cannot self-lock via a new quarantine (`failure_policy = "fail-open"` governs marker-WRITING
//! only).
//!
//! However, a pre-existing NON-EXPIRED marker STILL blocks the dispatch via the dispatcher's
//! native `on_error = "block_if_marker"` crash/timeout check (ADR-048 §D1 / BC-1.18.002 PC5):
//!
//! - EC-031: gate crash/fuel-exhaustion + non-expired marker present → Block (PC5).
//! - EC-009: gate crash/fuel-exhaustion + marker absent or TTL expired → Allow (PC6).
//! - EC-032: gate crash/fuel-exhaustion + marker TTL expired → Allow (PC6).
//!
//! Recovery: `rm .factory/unvalidated-mutation.marker` (BC-1.18.003 PC3 escape hatch)
//! or wait for the 24h deadman TTL (ADR-048 §D2).
//!
//! # Marker absent → allow (AC-010)
//!
//! When `.factory/unvalidated-mutation.marker` is absent, BOTH arms return exit_code=0 (allow).
//!
//! # Operator escape hatch (BC-1.18.003 postcondition 3)
//!
//! `rm .factory/unvalidated-mutation.marker` is the fully supported escape hatch.
//! After the rm, both arms unblock simultaneously (both check marker presence at runtime).
//!
//! # BC-5.38.001 Red Gate discipline
//!
//! All non-trivial function bodies use `todo!()`. Implementer fills in real logic.

use vsdd_hook_sdk::{HookPayload, HookResult};

/// HOST_ABI_VERSION declares the ABI contract version this plugin was built against.
/// No new host functions introduced by S-25.01 (BC-1.18.002 architecture compliance).
pub const HOST_ABI_VERSION: u32 = 1;

// ---------------------------------------------------------------------------
// Gate decision type
// ---------------------------------------------------------------------------

/// Decision returned by `guard_logic::evaluate_gate` after checking the marker file.
///
/// - `Allow`: marker absent; dispatch may proceed.
/// - `Block`: marker present; block message contains the required 5 fields + recovery instructions.
///
/// BC-1.18.002 postcondition 1/2: block message MUST contain plugin_name, artifact_path,
/// cause, a re-validation command, and the manual escape hatch instruction (`rm .factory/...`).
/// The message MUST be machine-parseable (structured fields, not freeform prose). AC-007/AC-008.
#[derive(Debug)]
pub enum GateDecision {
    /// Marker absent — allow the dispatch to proceed.
    Allow,
    /// Marker present — block the dispatch with the structured block message fields.
    Block {
        /// The `plugin_name` field from the marker TOML.
        plugin_name: String,
        /// The `artifact_path` field from the marker TOML (may be empty string).
        artifact_path: String,
        /// The `cause` field from the marker TOML: "fuel" | "epoch" | "output-too-large".
        cause: String,
        /// The `trace_id` field from the marker TOML.
        trace_id: String,
    },
}

// ---------------------------------------------------------------------------
// guard_logic — pure-core and effectful gate primitives
// ---------------------------------------------------------------------------

/// Gate primitives: pure command filter and effectful marker presence check.
///
/// Both functions are the unit-testable core of the two-arm gate. Tests exercise
/// these without a WASM runtime.
pub mod guard_logic {
    use std::path::Path;

    use super::GateDecision;

    /// Read and parse `.factory/unvalidated-mutation.marker` to determine the gate decision.
    ///
    /// - If the file is absent, returns `GateDecision::Allow`.
    /// - If the file is present and parseable, returns `GateDecision::Block { ... }` with
    ///   the fields extracted from the marker TOML.
    /// - If the file is present but unparseable (corrupt marker), returns `GateDecision::Block`
    ///   with best-effort populated fields (conservative path — still block).
    ///
    /// **Pure presence check (ADR-048 §Decision 4 v1.2):** this function performs NO
    /// `expires_at` / TTL date-math, no marker deletion, and no `marker.cleared` emission.
    /// TTL_EXPIRED detection, auto-clear, and emission are handled entirely dispatcher-native
    /// (`indeterminate_marker::check_and_clear_expired_marker`, invoked from `executor.rs`'s
    /// tier-execution loop BEFORE this plugin ever runs) — a WASM plugin cannot emit an event
    /// carrying the marker's own (foreign) `trace_id`/`plugin_name` through the `emit_event`
    /// host ABI (RESERVED_FIELDS enrichment). By the time this function runs, a present
    /// marker is therefore guaranteed non-expired, so presence alone is sufficient to Block.
    ///
    /// The returned block message fields are used by both Arm 1 and Arm 2 to construct
    /// the structured block message (AC-007/AC-008).
    ///
    /// BC-1.18.002 postcondition 1/2 + invariant 4 (block message completeness).
    ///
    /// # BC-5.38.001
    ///
    /// Effectful (filesystem read + TOML parse). Non-trivial body. Uses `todo!()`.
    pub fn evaluate_gate(marker_path: &Path) -> GateDecision {
        // Absent marker → allow (AC-010; BC-1.18.002 PC4).
        match std::fs::read_to_string(marker_path) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => GateDecision::Allow,
            Err(e) => {
                // EC-030: any non-NotFound read error (EACCES/EPERM/IO/etc.) → fail-open (Allow).
                //
                // Rationale (BC-1.18.002 v1.4 EC-030 + INV2 self-lock prevention):
                // - The marker bytes could not be read, so the marker's quarantine contents
                //   are unknown.
                // - Critically, `rm .factory/unvalidated-mutation.marker` (the operator escape
                //   hatch) would ALSO fail under the same permission/IO fault — meaning a Block
                //   here would create an unclearable self-lock (INV2 violation).
                // - Therefore: unreadable marker → fail-open. The operator escape hatch remains
                //   operable once the underlying filesystem fault is resolved.
                tracing::warn!(
                    error = %e,
                    path = %marker_path.display(),
                    "unvalidated-mutation marker unreadable (fail-open EC-030): \
                     allowing dispatch; resolve filesystem fault and re-run validation"
                );
                GateDecision::Allow
            }
            Ok(content) => {
                // Parse marker TOML via the `toml` crate (Library table mandate; MEDIUM-4).
                // This correctly handles all escape sequences including \n, \r, control chars.
                // If the marker is corrupt (parse error), fall back to conservative block
                // with sentinel values so the operator knows parsing failed.
                let table: toml::Table = match toml::from_str(&content) {
                    Ok(t) => t,
                    Err(_) => {
                        return GateDecision::Block {
                            plugin_name: "<unparseable-marker>".to_string(),
                            artifact_path: String::new(),
                            cause: "<unknown>".to_string(),
                            trace_id: String::new(),
                        };
                    }
                };

                let plugin_name = table
                    .get("plugin_name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<unknown>")
                    .to_string();
                let artifact_path = table
                    .get("artifact_path")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let cause = table
                    .get("cause")
                    .and_then(|v| v.as_str())
                    .unwrap_or("<unknown>")
                    .to_string();
                let trace_id = table
                    .get("trace_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();

                // ADR-048 §Decision 4 v1.2 (S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH):
                // `evaluate_gate` performs NO `expires_at` parsing, no auto-delete, and no
                // `marker.cleared` emission of its own. TTL_EXPIRED detection + clear +
                // emission moved entirely dispatcher-native
                // (`indeterminate_marker::check_and_clear_expired_marker`, called from
                // `executor.rs`'s tier-execution loop BEFORE this plugin ever runs) because
                // the WASM `emit_event` host ABI's RESERVED_FIELDS enrichment unconditionally
                // overwrites plugin-supplied `trace_id`/`plugin_name` with THIS plugin's own
                // dispatch identity — a WASM plugin can never emit an event carrying the
                // marker's foreign trace_id/plugin_name, which the Event 9 wire contract
                // requires. By the time this function runs, the marker is guaranteed
                // already absent-or-non-expired by that pre-check, so a still-present
                // marker here is unconditionally blocked — a pure presence check.
                GateDecision::Block {
                    plugin_name,
                    artifact_path,
                    cause,
                    trace_id,
                }
            }
        }
    }

    /// Returns `true` iff the given Bash `command` string identifies `commit` or `push`
    /// as the git subcommand. Implements the BC-1.18.002 v1.4 algorithm.
    ///
    /// Used by Arm 2 to determine whether a `^Bash$` PreToolUse dispatch should be
    /// checked against the marker file. Non-advancing commands (`git status`, `git log`,
    /// `cargo test`, etc.) return `false`. EC-001..EC-005 MUST return false.
    /// EC-006..EC-023 and EC-012/EC-013/EC-014/EC-015..EC-018 MUST return true.
    ///
    /// The five phases:
    ///
    /// - **Phase 1** — compound split on `&&`, `||`, `;`, `|`, `&`, `\n` →
    ///   any segment returning `true` ⇒ `true`.
    /// - **Phase 1b** — shell-words quote-aware POSIX tokenization of each segment
    ///   (`shell_words::split`): removes quotes and handles backslash escapes so that
    ///   `git "commit"`, `git 'push' origin`, and `g'i't commit` tokenize correctly.
    ///   On `Err` (unmatched quote) → conservative fail-safe: return `true`.
    /// - **Phase 2** — executable basename identification: strip a leading `env` token,
    ///   then strip leading `VAR=value` tokens; take the first remaining token as the
    ///   executable; strip to its basename via `rfind('/')`. basename ≠ `"git"` ⇒ `false`.
    /// - **Phase 3** — skip recognized git global options: complete arg-taking set
    ///   (`-C`, `-c`, `--namespace`, `--git-dir`, `--work-tree`, `--super-prefix`,
    ///   `--config-env`) + recognized no-arg flags + inline `--opt=value` forms.
    ///   Unrecognized `-`-prefixed flag → fail-safe (return `true`; arity unknown).
    /// - **Phase 4** — exact subcommand match: return `true` iff token equals
    ///   `"commit"` or `"push"`.
    ///
    /// BC-1.18.002 v1.4 PC2. VP-105 unit-test property row. EC-001..EC-023.
    pub fn is_git_commit_or_push(command: &str) -> bool {
        // BC-1.18.002 v1.4 algorithm (Phase 1 compound split → Phase 1b quote-aware
        // tokenization → Phase 2 basename → Phase 3 global-option skip / fail-safe →
        // Phase 4 exact subcommand match).
        //
        // Phase 1 — compound split on &&, ||, ;, |, &, \n → any segment true ⇒ true.
        // Phase 1b — shell_words::split() per evaluate_git_segment (quote-aware POSIX).
        // Phase 2 — basename identification: strip leading env/VAR=x tokens; take
        //           basename of first remaining token; basename != "git" ⇒ false.
        // Phase 3 — skip global options (complete arg-taking + no-arg sets; FAIL-SAFE
        //           on unrecognized -flags).
        // Phase 4 — exact subcommand: true iff "commit" or "push".
        for segment in split_shell_segments(command) {
            let segment = segment.trim();
            if !segment.is_empty() && evaluate_git_segment(segment) {
                return true;
            }
        }
        false
    }

    // ── Phase 1: compound splitting ────────────────────────────────────────────

    /// Splits `s` on shell operators (`&&`, `||`, `;`, `|`, `&`, `\n`) and returns
    /// a `Vec<&str>` of segments (borrows of the original string). Two-character
    /// operators (`&&`, `||`) are consumed atomically. Segments may be empty or
    /// whitespace-only; callers must trim and skip blanks.
    ///
    /// All operator characters are single-byte ASCII, so all split indices are valid
    /// UTF-8 character boundaries for any well-formed input.
    fn split_shell_segments(s: &str) -> Vec<&str> {
        let bytes = s.as_bytes();
        let n = bytes.len();
        let mut segments = Vec::new();
        let mut seg_start = 0_usize;
        let mut i = 0_usize;

        while i < n {
            let (advance, is_sep): (usize, bool) = match bytes[i] {
                b'&' if i + 1 < n && bytes[i + 1] == b'&' => (2, true),
                b'|' if i + 1 < n && bytes[i + 1] == b'|' => (2, true),
                b'&' | b'|' | b';' | b'\n' => (1, true),
                _ => (1, false),
            };

            if is_sep {
                segments.push(&s[seg_start..i]);
                seg_start = i + advance;
                i = seg_start;
            } else {
                i += advance;
            }
        }

        segments.push(&s[seg_start..]);
        segments
    }

    // ── Phases 2–4: single-segment evaluation ──────────────────────────────────

    /// Applies Phases 2–4 per BC-1.18.002 §PC2 Phase-2/3/4 algorithm to a single pre-trimmed segment.
    ///
    /// MEDIUM-2 fix (S-25.01): uses `shell_words::split()` for quote-aware POSIX
    /// tokenization so that `git "commit"`, `git 'push' origin`, and `g'i't commit`
    /// correctly produce token `["git", "commit"]` / `["git", "push", "origin"]` /
    /// `["git", "commit"]` rather than literal-quoted strings.
    ///
    /// If `shell_words::split` returns `Err` (unmatched quote) the segment is
    /// UNPARSEABLE — conservative posture: return `true` (uncertain = block).
    fn evaluate_git_segment(segment: &str) -> bool {
        // MEDIUM-2 fix: quote-aware tokenization via shell_words (BC-1.18.002 v1.3).
        // split() removes quotes and handles backslash escapes per POSIX sh word-splitting.
        // On Err (mismatched quotes) → fail-safe: uncertain input = block.
        let owned: Vec<String> = match shell_words::split(segment) {
            Ok(v) => v,
            Err(_) => return true,
        };
        let tokens: Vec<&str> = owned.iter().map(String::as_str).collect();
        let n = tokens.len();
        if n == 0 {
            return false;
        }

        let mut i = 0_usize;

        // ── Phase 2a: strip leading `env` literal (first token only) ──────────
        if tokens[i] == "env" {
            i += 1;
            if i >= n {
                return false;
            }
        }

        // ── Phase 2a (cont.): strip consecutive leading VAR=value tokens ──────
        while i < n && is_env_assignment(tokens[i]) {
            i += 1;
        }
        if i >= n {
            return false;
        }

        // ── Phase 2b/c: first remaining token is the executable; basename check ─
        let executable = tokens[i];
        i += 1;
        let basename = executable
            .rfind('/')
            .map_or(executable, |pos| &executable[pos + 1..]);
        if basename != "git" {
            return false;
        }

        // ── Phase 3: skip recognized global options ────────────────────────────
        //
        // Complete arg-taking set (BC-1.18.002 §PC2 Phase-3 arg-taking-set table): each option
        // consumes itself PLUS the immediately following token as its separate-token argument.
        const OPTS_ARG_TAKING: &[&str] = &[
            "-C",
            "-c",
            "--namespace",
            "--git-dir",
            "--work-tree",
            "--super-prefix",
            "--config-env",
        ];
        // Recognized no-arg options: each option consumes only itself.
        const OPTS_NO_ARG: &[&str] = &[
            "--no-pager",
            "--paginate",
            "-p",
            "--bare",
            "--literal-pathspecs",
            "--no-literal-pathspecs",
            "--glob-pathspecs",
            "--noglob-pathspecs",
            "--icase-pathspecs",
            "--no-replace-objects",
            "--no-optional-locks",
            "--exec-path",
            "--html-path",
            "--man-path",
            "--info-path",
            "--version",
            "--help",
            "-v",
        ];

        while i < n {
            let token = tokens[i];

            if token == "--" {
                // End-of-options: next positional is the subcommand.
                i += 1;
                break;
            }

            if !token.starts_with('-') {
                // First non-option positional: this is the candidate subcommand.
                break;
            }

            // Inline option with embedded value (e.g., `--git-dir=.git`): skip only.
            if token.contains('=') {
                i += 1;
                continue;
            }

            // Recognized arg-taking option: skip token + its following argument.
            if OPTS_ARG_TAKING.contains(&token) {
                i += 2;
                continue;
            }

            // Recognized no-arg option: skip token only.
            if OPTS_NO_ARG.contains(&token) {
                i += 1;
                continue;
            }

            // Unrecognized -prefixed option: arity unknown → fail-safe (block).
            // Under-blocking is the dangerous failure mode for this security gate.
            return true;
        }

        // ── Phase 4: exact subcommand match ──────────────────────────────────────
        matches!(tokens.get(i), Some(&"commit") | Some(&"push"))
    }

    // ── Phase 2 helper: detect env-var assignment token ───────────────────────

    /// Returns `true` iff `token` matches the pattern `^[A-Za-z_][A-Za-z0-9_]*=`.
    /// Used by Phase 2 to strip leading environment variable assignments
    /// (e.g., `GIT_DIR=.git`, `HOME=/tmp`) from a shell command segment.
    fn is_env_assignment(token: &str) -> bool {
        let mut chars = token.chars();
        // First character must be ASCII letter or underscore.
        match chars.next() {
            Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
            _ => return false,
        }
        // Subsequent characters up to `=`: ASCII alphanumeric or underscore.
        // A `=` terminates the scan as a successful match.
        for c in chars {
            if c == '=' {
                return true;
            }
            if !c.is_ascii_alphanumeric() && c != '_' {
                return false;
            }
        }
        false // no `=` found — not an env-var assignment
    }
}

// ---------------------------------------------------------------------------
// Entry point: single function handling BOTH Arm 1 (Agent) and Arm 2 (Bash)
// ---------------------------------------------------------------------------

/// Testable inner implementation — takes an explicit `marker_path` so unit tests
/// can inject a tempdir marker without coupling to the WASI preopened CWD.
///
/// Called by `on_pre_tool_use` with the hardcoded production path; called directly
/// by tests with a tempdir-based path. This extraction ensures tests exercise the
/// real dispatch-routing logic and real block-message assembly, not a reconstruction.
///
/// # BC-5.38.001
///
/// Effectful (reads tool payload, reads marker file). Non-trivial body.
pub(crate) fn on_pre_tool_use_impl(
    payload: HookPayload,
    marker_path: &std::path::Path,
) -> HookResult {
    // Single entry point for BOTH Arm 1 (^Agent$) and Arm 2 (^Bash$) dispatches.
    // Both hooks-registry.toml entries reference this IDENTICAL WASM binary (AC-019).

    // Arm 2 (^Bash$): apply the git commit/push filter FIRST.
    // If this is a Bash dispatch that does NOT match is_git_commit_or_push
    // (BC-1.18.002 v1.3 tokenized subcommand match),
    // pass immediately without checking the marker (AC-009; BC-1.18.002 PC3).
    if payload.tool_name == "Bash" {
        let command = payload
            .tool_input
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if !guard_logic::is_git_commit_or_push(&command) {
            return HookResult::Continue;
        }
        // Command IS a git commit/push — fall through to marker check (AC-008).
    }

    // Arm 1 (^Agent$) or Arm 2 (git commit/push match): check marker presence.
    match guard_logic::evaluate_gate(marker_path) {
        GateDecision::Allow => HookResult::Continue,
        GateDecision::Block {
            plugin_name,
            artifact_path,
            cause,
            trace_id,
        } => {
            // Build structured block message (AC-007/AC-008 + BC-1.18.002 v1.6 INV4).
            // BC-1.18.002 v1.6 PC5: three-tier recovery guidance.
            // Tier-1 directs the AGENT to re-validate the artifact via Edit/Write.
            // Tier-2 notes the 24 h TTL auto-expiry.
            // Tier-3 mentions human operator rm as the break-glass escape — this agent
            // MUST NOT be instructed to perform rm to bypass the gate.
            let reason = serde_json::json!({
                "blocked_by": "validate-unvalidated-mutation-marker",
                "marker_plugin_name": plugin_name,
                "marker_artifact_path": artifact_path,
                "marker_cause": cause,
                "marker_trace_id": trace_id,
                "recovery": {
                    "tier_1_revalidate": format!(
                        "Edit or re-write the artifact at '{}' so that '{}' produces \
                         exit_code=0 on the next dispatch; this clears the quarantine \
                         via the REVALIDATED path",
                        artifact_path, plugin_name
                    ),
                    "tier_2_ttl_expiry": "If immediate re-validation is not possible, \
                        the quarantine auto-expires after 24 h via TTL_EXPIRED auto-clear",
                    "tier_3_operator_break_glass": "Human operator only: \
                        rm .factory/unvalidated-mutation.marker — \
                        this agent MUST NOT perform rm operations to bypass the gate"
                }
            })
            .to_string();
            HookResult::Block { reason }
        }
    }
}

/// PreToolUse entry point for both Arm 1 (`^Agent$`) and Arm 2 (`^Bash$` git filter).
///
/// This single function handles both dispatch arms since both hooks-registry.toml entries
/// reference the IDENTICAL WASM binary (BC-1.18.002 invariant 1; AC-019).
///
/// **Arm 1 (Agent):** fires on `tool = "^Agent$"` PreToolUse dispatches.
/// Checks marker presence; blocks if marker exists (AC-007; EC-004).
///
/// **Arm 2 (Bash git filter):** fires on `tool = "^Bash$"` PreToolUse dispatches.
/// First applies `is_git_commit_or_push(command)` filter; if the command is NOT
/// a git commit/push, passes immediately without checking the marker (AC-009).
/// If the command IS a git commit/push, checks marker presence; blocks if marker exists (AC-008).
///
/// **Both arms absent-marker path:** when marker is absent, both arms return exit_code=0
/// (allow) unconditionally (AC-010).
///
/// Delegates to `on_pre_tool_use_impl` with the production marker path. The production
/// path is relative to the WASI preopened CWD (`"."`), which the dispatcher sets to
/// the project root — so `.factory/unvalidated-mutation.marker` resolves correctly
/// without requiring an absolute path (which WASI rejects when no root preopen is
/// configured).
pub fn on_pre_tool_use(payload: HookPayload) -> HookResult {
    // Relative path: the dispatcher preopens host_ctx.cwd as WASI `"."`, so
    // `.factory/unvalidated-mutation.marker` resolves to the project root's
    // .factory directory without needing an absolute path (which WASI rejects
    // when no root preopen is configured). Do NOT use host::cwd() + absolute
    // PathBuf here — wasmtime WASI returns ENOENT for absolute paths that
    // aren't under a preopened directory prefix.
    let marker_path = std::path::Path::new(".factory").join("unvalidated-mutation.marker");
    on_pre_tool_use_impl(payload, &marker_path)
}

// ---------------------------------------------------------------------------
// S-25.01 Red Gate tests — BC-1.18.002 (3 required stubs + 4 additional)
// ---------------------------------------------------------------------------
//
// All tests call todo!() production functions (evaluate_gate, is_git_commit_or_push)
// and MUST FAIL at runtime until the implementer fills in the production logic (T-4).
// BC-5.38.005 self-check: "If I include this real implementation, will the test
// pass trivially without implementer work?" — YES for all tests below.

#[cfg(test)]
#[allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
mod tests {
    use super::GateDecision;
    use super::guard_logic::{evaluate_gate, is_git_commit_or_push};

    // ── S-25.01 Red Gate stub 13 ─────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_002_blocks_agent_dispatch_when_marker_exists() {
        // AC-007 / BC-1.18.002 postcondition 1 + invariant 4:
        // Marker present → evaluate_gate returns GateDecision::Block.
        // Block message MUST contain: plugin_name, artifact_path, cause,
        // a re-validation command, and the manual escape hatch (`rm .factory/...`).
        // Message MUST be machine-parseable (structured fields, not freeform prose).
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        // Write a valid 5-field TOML marker (BC-1.18.001 postcondition 4)
        std::fs::write(
            &marker_path,
            concat!(
                "timestamp = \"2026-08-30T00:00:00Z\"\n",
                "plugin_name = \"validate-factory-path-staging\"\n",
                "artifact_path = \"/tmp/.factory/STATE.md\"\n",
                "cause = \"fuel\"\n",
                "trace_id = \"deadbeef-0001-0001-0001-000000000001\"\n"
            ),
        )
        .expect("test setup: write marker");

        let decision = evaluate_gate(&marker_path);
        // Marker present → must block
        assert!(
            matches!(decision, GateDecision::Block { .. }),
            "AC-007: marker present MUST yield GateDecision::Block — got {:?}",
            decision
        );
        // Block message must carry the plugin_name and artifact_path from the marker
        if let GateDecision::Block {
            plugin_name,
            artifact_path,
            cause,
            ..
        } = decision
        {
            assert_eq!(
                plugin_name, "validate-factory-path-staging",
                "AC-007 / BC-1.18.002 INV4: block message MUST carry plugin_name from marker"
            );
            assert_eq!(
                artifact_path, "/tmp/.factory/STATE.md",
                "AC-007 / BC-1.18.002 INV4: block message MUST carry artifact_path from marker"
            );
            assert_eq!(
                cause, "fuel",
                "AC-007 / BC-1.18.002 INV4: block message MUST carry cause from marker"
            );
        }
    }

    // ── S-25.01 Red Gate stub 14 ─────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_002_passes_agent_dispatch_when_no_marker() {
        // AC-010 / BC-1.18.002 postcondition 4:
        // Marker absent → evaluate_gate returns GateDecision::Allow.
        // Both Arm 1 (Agent) and Arm 2 (Bash git) MUST allow when marker is absent.
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        // Verify no marker exists (do NOT create one)
        assert!(
            !marker_path.exists(),
            "pre-condition: marker must be absent for this test"
        );

        let decision = evaluate_gate(&marker_path);
        assert!(
            matches!(decision, GateDecision::Allow),
            "AC-010: marker absent MUST yield GateDecision::Allow — got {:?}",
            decision
        );
    }

    // ── S-25.01 Red Gate stub 15 ─────────────────────────────────────────────
    #[test]
    fn test_BC_1_18_002_block_message_names_plugin_and_artifact() {
        // AC-007 / AC-008 / BC-1.18.002 invariant 4:
        // The block message MUST be machine-parseable (structured fields, not freeform prose).
        // Both Arm 1 (Agent) and Arm 2 (Bash git) produce identical block message format.
        // Required fields: plugin_name, artifact_path, cause, trace_id (from marker TOML).
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        let plugin = "validate-factory-path-staging";
        let artifact = "/path/to/.factory/STATE.md";
        let cause_val = "output-too-large";
        let trace = "aaaabbbb-cccc-dddd-eeee-ffffffffffff";
        std::fs::write(
            &marker_path,
            format!(
                "timestamp = \"2026-08-30T12:00:00Z\"\nplugin_name = \"{plugin}\"\n\
                 artifact_path = \"{artifact}\"\ncause = \"{cause_val}\"\ntrace_id = \"{trace}\"\n"
            ),
        )
        .expect("test setup: write marker");

        let decision = evaluate_gate(&marker_path);
        match decision {
            GateDecision::Block {
                plugin_name,
                artifact_path,
                cause,
                ..
            } => {
                assert_eq!(
                    plugin_name, plugin,
                    "AC-007/AC-008: block plugin_name MUST equal marker plugin_name (machine-parseable)"
                );
                assert_eq!(
                    artifact_path, artifact,
                    "AC-007/AC-008: block artifact_path MUST equal marker artifact_path"
                );
                assert_eq!(
                    cause, cause_val,
                    "AC-007/AC-008: block cause MUST equal marker cause"
                );
            }
            GateDecision::Allow => {
                panic!("AC-007/AC-008: marker present MUST produce Block, not Allow");
            }
        }
    }

    // ── S-25.01 additional Red Gate test for AC-008 ───────────────────────────

    /// AC-008 / BC-1.18.002 postcondition 2 + EC-006 + EC-007:
    /// Bash arm (Arm 2): marker present + git commit/push command → block.
    /// Both Arm 1 and Arm 2 use the same evaluate_gate — marker presence is checked.
    /// The is_git_commit_or_push filter (Arm 2) is tested separately below.
    #[test]
    fn test_BC_1_18_002_bash_arm_blocks_git_commit_when_marker_exists() {
        // AC-008: marker present + command matches \bgit\b.*\b(commit|push)\b → block.
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        std::fs::write(
            &marker_path,
            concat!(
                "timestamp = \"2026-08-30T00:00:00Z\"\n",
                "plugin_name = \"regression-gate\"\n",
                "artifact_path = \"\"\n",
                "cause = \"epoch\"\n",
                "trace_id = \"test-trace-bash-arm\"\n"
            ),
        )
        .expect("test setup");

        // EC-006: git commit --amend MUST match the filter
        assert!(
            is_git_commit_or_push("git commit --amend --no-edit"),
            "AC-008 / EC-006: 'git commit --amend' MUST match is_git_commit_or_push (BC-1.18.002 v1.3 tokenized subcommand match)"
        );
        // EC-007: git push --force-with-lease MUST match the filter
        assert!(
            is_git_commit_or_push("git push --force-with-lease"),
            "AC-008 / EC-007: 'git push --force-with-lease' MUST match is_git_commit_or_push (BC-1.18.002 v1.3 tokenized subcommand match)"
        );

        // When marker exists and command matches, evaluate_gate MUST block
        let decision = evaluate_gate(&marker_path);
        assert!(
            matches!(decision, GateDecision::Block { .. }),
            "AC-008: marker present → evaluate_gate MUST return Block (Arm 2 then blocks) — got {:?}",
            decision
        );
    }

    // ── S-25.01 additional Red Gate tests for AC-009 (is_git_commit_or_push) ──

    /// AC-009 / BC-1.18.002 postcondition 3 + EC-001..EC-005:
    /// Non-advancing Bash commands MUST NOT be gated.
    /// is_git_commit_or_push(command) MUST return false for non-commit/push git subcommands
    /// and for non-git commands entirely.
    #[test]
    fn test_BC_1_18_002_is_git_commit_or_push_rejects_non_advancing_commands() {
        // AC-009 / BC-1.18.002 postcondition 3 (EC-001..EC-005 + non-git):
        // These commands MUST NOT match the pattern — Arm 2 should NOT gate them.

        // EC-001: git status
        assert!(
            !is_git_commit_or_push("git status --porcelain"),
            "AC-009 / EC-001: 'git status' MUST NOT match (read-only; never gates)"
        );
        // EC-002: git log
        assert!(
            !is_git_commit_or_push("git log --oneline -5"),
            "AC-009 / EC-002: 'git log' MUST NOT match (read-only)"
        );
        // EC-003: git diff
        assert!(
            !is_git_commit_or_push("git diff HEAD~1"),
            "AC-009 / EC-003: 'git diff' MUST NOT match (read-only)"
        );
        // EC-004: git fetch
        assert!(
            !is_git_commit_or_push("git fetch origin"),
            "AC-009 / EC-004: 'git fetch' MUST NOT match (does not advance local state)"
        );
        // EC-005: cargo test (non-git)
        assert!(
            !is_git_commit_or_push("cargo test --workspace --all-targets"),
            "AC-009 / EC-005: non-git commands MUST NOT match"
        );
        // Additional: git add (staging only, not advancing)
        assert!(
            !is_git_commit_or_push("git add -A"),
            "AC-009: 'git add' MUST NOT match (staging, not advancing)"
        );
        // Additional: git stash
        assert!(
            !is_git_commit_or_push("git stash"),
            "AC-009: 'git stash' MUST NOT match"
        );
    }

    /// AC-009 / BC-1.18.002 postcondition 2 + EC-006 + EC-007:
    /// Advancing commands: git commit and git push variants MUST match.
    #[test]
    fn test_BC_1_18_002_is_git_commit_or_push_matches_commit_and_push_variants() {
        // AC-008 / AC-009 / EC-006 / EC-007:
        // git commit (all variants) and git push (all variants) MUST match.

        // git commit variants
        assert!(
            is_git_commit_or_push("git commit -m 'test message'"),
            "AC-009: 'git commit -m ...' MUST match"
        );
        assert!(
            is_git_commit_or_push("git commit --amend --no-edit"),
            "AC-009 / EC-006: 'git commit --amend' MUST match (BC-1.18.002 EC-006)"
        );
        assert!(
            is_git_commit_or_push("git commit -F /tmp/msg.txt"),
            "AC-009: 'git commit -F ...' MUST match"
        );
        // git push variants
        assert!(
            is_git_commit_or_push("git push origin factory-artifacts"),
            "AC-009: 'git push origin ...' MUST match"
        );
        assert!(
            is_git_commit_or_push("git push --force-with-lease"),
            "AC-009 / EC-007: 'git push --force-with-lease' MUST match (BC-1.18.002 EC-007)"
        );
        assert!(
            is_git_commit_or_push("git push"),
            "AC-009: bare 'git push' MUST match"
        );
    }

    // ── BLOCKER-3 (RED): global-option forms ─────────────────────────────────

    /// BLOCKER-3 (RED): `is_git_commit_or_push` MUST return `true` when `git` is
    /// followed by global options that take arguments (`-C <path>`, `-c <key>=<val>`,
    /// `--namespace <ns>`) before the `commit` or `push` subcommand. AC-008 / AC-009 /
    /// BC-1.18.002 PC2.
    ///
    /// Currently FAILS (RED) for the `-C`, `-c`, and `--namespace` forms because the
    /// current scanner stops at the first non-flag positional token after `git`,
    /// treating it as the subcommand. With `git -C /path commit`, the scanner sees
    /// `/path` (the argument to `-C`) as the positional token and returns `false`.
    ///
    /// The Phase 1/1b/2/3/4 multi-stage algorithm in `is_git_commit_or_push` handles all
    /// these forms because Phase 3 explicitly skips recognized global options (and their
    /// arguments for arg-taking options) before the subcommand is checked in Phase 4.
    /// Note: BC-1.18.002 §PC2 previously cited a regex as illustrative; the authoritative
    /// specification is the Phase 1/1b/2/3/4 algorithm, not any regex pattern.
    #[test]
    fn test_BC_1_18_002_is_git_commit_or_push_global_option_forms() {
        // AC-008 / AC-009 / BC-1.18.002 PC2
        // git global options with arguments:
        //   -C <path>         : change working directory
        //   -c <key>=<value>  : set a config option
        //   --namespace <ns>  : operate in namespace
        // All are valid before the subcommand and occur in real dispatcher commits.

        // RED cases (currently fail with current scanner):

        // git -C /path commit -m x → TRUE
        assert!(
            is_git_commit_or_push("git -C /path commit -m x"),
            "BLOCKER-3 / AC-008 / BC-1.18.002 PC2: \
             'git -C /path commit' MUST match \\bgit\\b.*\\b(commit|push)\\b. \
             FAILS: current scanner treats '/path' (arg to -C) as subcommand, returns false."
        );

        // git -C .factory push → TRUE
        assert!(
            is_git_commit_or_push("git -C .factory push"),
            "BLOCKER-3 / AC-008 / BC-1.18.002 PC2: \
             'git -C .factory push' MUST match. \
             FAILS: current scanner treats '.factory' (arg to -C) as subcommand, returns false."
        );

        // git -c user.email=x commit → TRUE
        assert!(
            is_git_commit_or_push("git -c user.email=x commit"),
            "BLOCKER-3 / AC-008 / BC-1.18.002 PC2: \
             'git -c user.email=x commit' MUST match. \
             FAILS: current scanner treats 'user.email=x' (arg to -c) as subcommand, returns false."
        );

        // git --namespace foo push → TRUE
        assert!(
            is_git_commit_or_push("git --namespace foo push"),
            "BLOCKER-3 / AC-008 / BC-1.18.002 PC2: \
             'git --namespace foo push' MUST match. \
             FAILS: current scanner treats 'foo' (arg to --namespace) as subcommand, returns false."
        );

        // git   commit (extra whitespace) → TRUE (split_whitespace normalizes)
        // NOTE: This case likely already passes; included for completeness.
        assert!(
            is_git_commit_or_push("git   commit"),
            "AC-008: 'git   commit' with extra spaces MUST match (split_whitespace normalizes)"
        );

        // Non-matching forms MUST remain FALSE after the fix:

        // gitk is a different program entirely (no word boundary match)
        assert!(
            !is_git_commit_or_push("gitk"),
            "AC-009: 'gitk' is NOT 'git'; MUST NOT match"
        );

        // git commit-graph write: 'commit-graph' contains 'commit' as a prefix but the
        // regex \bcommit\b does NOT match it because '-' is a word boundary.
        // However, the *current* split_whitespace scanner also correctly returns false here
        // (first token after git is 'commit-graph' ≠ 'commit'). Both current and correct
        // implementations agree on this case. Verify the correct semantic is preserved.
        // BC-1.18.002 v1.1 EC-011: commit-graph → false.
        assert!(
            !is_git_commit_or_push("git commit-graph write"),
            "AC-009 / EC-011 / BC-1.18.002 v1.1: 'git commit-graph write' MUST NOT match — \
             'commit-graph' is NOT the 'commit' subcommand (exact subcommand matching; EC-011)"
        );

        // Read-only commands MUST NOT match
        assert!(
            !is_git_commit_or_push("git status"),
            "AC-009 / EC-001: 'git status' MUST NOT match"
        );
        assert!(
            !is_git_commit_or_push("git log --oneline"),
            "AC-009 / EC-002: 'git log' MUST NOT match"
        );

        // ── BC-1.18.002 v1.1 PO canonical vector set — all 12 exact forms ──────
        //
        // PO ruling (BC-1.18.002 v1.1): is_git_commit_or_push uses EXACT-SUBCOMMAND
        // matching. The following 12 vectors are the authoritative golden set.
        //
        // TRUE (advancing — MUST be gated):
        //   git commit -m "fix"
        //   git push origin main
        //   git -C .factory commit -m "state"    [EC-012: git -C commit → true]
        //   git -c user.email=x push origin main
        //   git commit --amend --no-edit
        //   git push --force-with-lease
        //
        // FALSE (non-advancing — MUST NOT be gated):
        //   git commit-graph write               [EC-011: commit-graph → false]
        //   git status --porcelain
        //   git log --oneline
        //   git diff HEAD~1
        //   git fetch origin
        //   cargo test --workspace

        // TRUE canonical vectors ------------------------------------------------

        assert!(
            is_git_commit_or_push("git commit -m \"fix\""),
            "BC-1.18.002 v1.1 canonical TRUE: 'git commit -m \"fix\"' MUST match"
        );
        assert!(
            is_git_commit_or_push("git push origin main"),
            "BC-1.18.002 v1.1 canonical TRUE: 'git push origin main' MUST match"
        );
        // EC-012: git -C <path> commit → true.
        // The -C global option takes a separate argument; the subcommand is 'commit'.
        assert!(
            is_git_commit_or_push("git -C .factory commit -m \"state\""),
            "BC-1.18.002 v1.1 EC-012: 'git -C .factory commit -m \"state\"' MUST match — \
             -C takes the path argument; 'commit' is the subcommand (EC-012: git -C commit → true)"
        );
        assert!(
            is_git_commit_or_push("git -c user.email=x push origin main"),
            "BC-1.18.002 v1.1 canonical TRUE: 'git -c user.email=x push origin main' MUST match \
             (-c takes the key=value argument; 'push' is the subcommand)"
        );
        assert!(
            is_git_commit_or_push("git commit --amend --no-edit"),
            "BC-1.18.002 v1.1 canonical TRUE: 'git commit --amend --no-edit' MUST match"
        );
        assert!(
            is_git_commit_or_push("git push --force-with-lease"),
            "BC-1.18.002 v1.1 canonical TRUE: 'git push --force-with-lease' MUST match"
        );

        // FALSE canonical vectors -----------------------------------------------

        // EC-011: commit-graph → false (also asserted above; canonical coverage).
        assert!(
            !is_git_commit_or_push("git commit-graph write"),
            "BC-1.18.002 v1.1 EC-011 canonical FALSE: 'git commit-graph write' MUST NOT match \
             ('commit-graph' is a plumbing subcommand, not 'commit'; EC-011)"
        );
        assert!(
            !is_git_commit_or_push("git status --porcelain"),
            "BC-1.18.002 v1.1 canonical FALSE: 'git status --porcelain' MUST NOT match"
        );
        assert!(
            !is_git_commit_or_push("git log --oneline"),
            "BC-1.18.002 v1.1 canonical FALSE: 'git log --oneline' MUST NOT match"
        );
        assert!(
            !is_git_commit_or_push("git diff HEAD~1"),
            "BC-1.18.002 v1.1 canonical FALSE: 'git diff HEAD~1' MUST NOT match"
        );
        assert!(
            !is_git_commit_or_push("git fetch origin"),
            "BC-1.18.002 v1.1 canonical FALSE: 'git fetch origin' MUST NOT match"
        );
        assert!(
            !is_git_commit_or_push("cargo test --workspace"),
            "BC-1.18.002 v1.1 canonical FALSE: 'cargo test --workspace' MUST NOT match \
             (non-git command)"
        );
    }

    // ── BC-1.18.002 v1.2 — complete 23-EC canonical-vector test ─────────────
    //
    // Covers all EC-001..EC-023 for which `is_git_commit_or_push` has a
    // defined return value.
    //
    // v1.2 four-phase algorithm under test:
    //   Phase 1 — compound splitting: &&, ||, ;, |, &, newline → any segment
    //             true ⇒ true.
    //   Phase 2 — basename identification: strip leading `env` / `VAR=x`
    //             tokens; executable = first remaining token; basename = strip
    //             through last '/'.  basename ≠ "git" ⇒ false for this segment.
    //   Phase 3 — skip global options: complete arg-taking set
    //             {-C, -c, --namespace, --git-dir, --work-tree, --super-prefix,
    //             --config-env} + recognized no-arg flags + inline --opt=value;
    //             UNRECOGNIZED flag ⇒ fail-safe true.
    //   Phase 4 — exact subcommand: true iff "commit" or "push".
    //
    // GREEN = passes with current (v1.1-only) implementation.
    // RED   = fails with current implementation; must go red before implementation.
    //
    // RED assertions (7): EC-013, EC-014, EC-015, EC-016, EC-017, EC-018,
    //                     EC-023.
    //
    // VP-105 unit-test property row.
    #[test]
    #[allow(clippy::cognitive_complexity)]
    fn test_BC_1_18_002_is_git_commit_or_push_v1_2_canonical_vectors() {
        // ── Phase 4: basic advancing subcommands ─────────────────────────────

        // [GREEN] Basic commit form.  Also exercises EC-010 (marker-absent path
        // in the full gate: is_git still returns true; marker-absent arm allows).
        assert!(
            is_git_commit_or_push(r#"git commit -m "test""#),
            "BC-1.18.002 v1.2 / VP-105: 'git commit -m \"test\"' MUST return true \
             (Phase 4: subcommand is 'commit'; covers EC-010 is_git aspect)"
        );
        // [GREEN] Basic push form.
        assert!(
            is_git_commit_or_push("git push origin factory-artifacts"),
            "BC-1.18.002 v1.2 / VP-105: 'git push origin factory-artifacts' MUST return true \
             (Phase 4: subcommand is 'push')"
        );
        // [GREEN] EC-006: commit --amend variant.
        assert!(
            is_git_commit_or_push("git commit --amend --no-edit"),
            "BC-1.18.002 v1.2 EC-006 / VP-105: 'git commit --amend --no-edit' MUST return true \
             (Phase 4: subcommand is 'commit'; --amend and --no-edit are post-subcommand flags)"
        );
        // [GREEN] EC-007: push --force-with-lease variant.
        assert!(
            is_git_commit_or_push("git push --force-with-lease"),
            "BC-1.18.002 v1.2 EC-007 / VP-105: 'git push --force-with-lease' MUST return true \
             (Phase 4: subcommand is 'push')"
        );

        // ── Phase 3: complete arg-taking global option set ────────────────────

        // [GREEN] EC-012: -C <path> — already in current OPTS_TAKING_ARG.
        assert!(
            is_git_commit_or_push(r#"git -C .factory commit -m "state""#),
            "BC-1.18.002 v1.2 EC-012 / VP-105: 'git -C .factory commit -m \"state\"' MUST return true \
             (Phase 3: -C consumes .factory as arg; first positional is 'commit')"
        );
        // [RED] EC-013: --git-dir <path> — NOT in current OPTS_TAKING_ARG.
        // Current: skips --git-dir flag only; treats '.git' as subcommand → false.
        // Correct: --git-dir consumes '.git'; first positional is 'commit' → true.
        assert!(
            is_git_commit_or_push(r#"git --git-dir .git commit -m "state""#),
            "BC-1.18.002 v1.2 EC-013 / VP-105: 'git --git-dir .git commit -m \"state\"' MUST return true \
             (Phase 3: --git-dir arg-taking; .git consumed; first positional is 'commit') \
             [RED: --git-dir not in current OPTS_TAKING_ARG; '.git' treated as subcommand → false]"
        );
        // [RED] EC-014: --work-tree <path> — NOT in current OPTS_TAKING_ARG.
        // Current: skips --work-tree flag only; treats '/tmp' as subcommand → false.
        // Correct: --work-tree consumes /tmp; first positional is 'push' → true.
        assert!(
            is_git_commit_or_push("git --work-tree /tmp push"),
            "BC-1.18.002 v1.2 EC-014 / VP-105: 'git --work-tree /tmp push' MUST return true \
             (Phase 3: --work-tree arg-taking; /tmp consumed; first positional is 'push') \
             [RED: --work-tree not in current OPTS_TAKING_ARG; '/tmp' treated as subcommand → false]"
        );
        // [RED] EC-023: --config-env <name=envvar> — NOT in current OPTS_TAKING_ARG.
        // Current: skips --config-env flag only; treats 'FOO=BAR' as subcommand → false.
        // Correct: --config-env consumes FOO=BAR; first positional is 'commit' → true.
        assert!(
            is_git_commit_or_push("git --config-env FOO=BAR commit"),
            "BC-1.18.002 v1.2 EC-023 / VP-105: 'git --config-env FOO=BAR commit' MUST return true \
             (Phase 3: --config-env arg-taking; FOO=BAR consumed; first positional is 'commit') \
             [RED: --config-env not in current OPTS_TAKING_ARG; 'FOO=BAR' treated as subcommand → false]"
        );
        // [GREEN] EC-022: fail-safe on unrecognized option.
        // Current: accidentally returns true (skips --unknown-flag, finds 'commit').
        // v1.2: fail-safe posture — unrecognized option arity unknown → return true immediately.
        // Observable behavior is the same; mechanism changes in implementation.
        assert!(
            is_git_commit_or_push("git --unknown-flag commit"),
            "BC-1.18.002 v1.2 EC-022 / VP-105: 'git --unknown-flag commit' MUST return true \
             (Phase 3 fail-safe: --unknown-flag not in any recognized set; \
             subcommand position uncertain; conservative posture blocks)"
        );

        // ── Phase 1: compound command splitting ───────────────────────────────

        // [RED] EC-015: && operator.
        // Current: split_whitespace gives tokens [git, status, &&, git, commit, ...];
        //   first non-option positional after 'git' is 'status' → false.
        // Correct: Phase 1 splits on &&; segment 2 subcommand is 'commit' → true.
        assert!(
            is_git_commit_or_push(r#"git status && git commit -m "x""#),
            "BC-1.18.002 v1.2 EC-015 / VP-105: 'git status && git commit -m \"x\"' MUST return true \
             (Phase 1: && splits; segment 1 = 'git status' → false; \
             segment 2 = 'git commit -m \"x\"' → true; any-segment true ⇒ true) \
             [RED: current split_whitespace; first positional after 'git' is 'status' → false]"
        );
        // [RED] EC-016: ; operator.
        // Current: first positional after 'git' is 'diff' → false.
        // Correct: Phase 1 splits on ;; segment 2 subcommand is 'push' → true.
        assert!(
            is_git_commit_or_push("git diff ; git push"),
            "BC-1.18.002 v1.2 EC-016 / VP-105: 'git diff ; git push' MUST return true \
             (Phase 1: ; splits; segment 1 = 'git diff' → false; \
             segment 2 = 'git push' → true; any-segment true ⇒ true) \
             [RED: current split_whitespace; first positional after 'git' is 'diff' → false]"
        );

        // ── Phase 2: basename identification ──────────────────────────────────

        // [RED] EC-017: absolute-path git executable.
        // Current: tokens[i] == "git" exact match; '/usr/bin/git' ≠ 'git' → scans past it,
        //   never finds a 'git' token → false.
        // Correct: basename('/usr/bin/git') = 'git'; subcommand is 'commit' → true.
        assert!(
            is_git_commit_or_push(r#"/usr/bin/git commit -m "init""#),
            "BC-1.18.002 v1.2 EC-017 / VP-105: '/usr/bin/git commit -m \"init\"' MUST return true \
             (Phase 2: basename('/usr/bin/git') = 'git'; subcommand is 'commit') \
             [RED: current impl uses exact token == \"git\"; misses path-prefixed executable]"
        );
        // [RED] EC-018: relative-path git executable.
        // Current: './git' ≠ 'git' → never finds git token → false.
        // Correct: basename('./git') = 'git'; subcommand is 'push' → true.
        assert!(
            is_git_commit_or_push("./git push origin main"),
            "BC-1.18.002 v1.2 EC-018 / VP-105: './git push origin main' MUST return true \
             (Phase 2: basename('./git') = 'git'; subcommand is 'push') \
             [RED: current impl uses exact token == \"git\"; misses ./git]"
        );
        // [GREEN] EC-021: env + VAR=x prefix stripping.
        // Current: split_whitespace gives [env, GIT_DIR=.git, git, commit, ...];
        //   scans all tokens, finds 'git' at index 2 → true. Accidentally correct.
        // v1.2: Phase 2 strips 'env' + 'GIT_DIR=.git'; executable is 'git' → same result.
        assert!(
            is_git_commit_or_push(r#"env GIT_DIR=.git git commit -m "x""#),
            "BC-1.18.002 v1.2 EC-021 / VP-105: 'env GIT_DIR=.git git commit -m \"x\"' MUST return true \
             (Phase 2: strip leading 'env' token + 'GIT_DIR=.git' VAR= token; \
             executable is 'git'; subcommand is 'commit')"
        );

        // ── FALSE: non-advancing commands MUST NOT be gated ───────────────────

        // [GREEN] EC-001: read-only git subcommand.
        assert!(
            !is_git_commit_or_push("git status --porcelain"),
            "BC-1.18.002 v1.2 EC-001 / VP-105: 'git status --porcelain' MUST return false \
             (Phase 4: subcommand 'status' ≠ 'commit'/'push'; read-only)"
        );
        // [GREEN] EC-002: read-only git subcommand.
        assert!(
            !is_git_commit_or_push("git log --oneline -5"),
            "BC-1.18.002 v1.2 EC-002 / VP-105: 'git log --oneline -5' MUST return false \
             (Phase 4: subcommand 'log' ≠ 'commit'/'push'; read-only)"
        );
        // [GREEN] EC-003: read-only git subcommand.
        assert!(
            !is_git_commit_or_push("git diff HEAD~1"),
            "BC-1.18.002 v1.2 EC-003 / VP-105: 'git diff HEAD~1' MUST return false \
             (Phase 4: subcommand 'diff' ≠ 'commit'/'push'; read-only)"
        );
        // [GREEN] EC-004: non-advancing git subcommand.
        assert!(
            !is_git_commit_or_push("git fetch origin"),
            "BC-1.18.002 v1.2 EC-004 / VP-105: 'git fetch origin' MUST return false \
             (Phase 4: subcommand 'fetch' ≠ 'commit'/'push')"
        );
        // [GREEN] EC-005: non-git command entirely.
        assert!(
            !is_git_commit_or_push("cargo test --workspace"),
            "BC-1.18.002 v1.2 EC-005 / VP-105: 'cargo test --workspace' MUST return false \
             (Phase 2: basename('cargo') ≠ 'git'; not a git invocation)"
        );
        // [GREEN] EC-011: exact subcommand mismatch — 'commit-graph' ≠ 'commit'.
        // BC-1.18.002 v1.1 clarification: the illustrative regex \bcommit\b would
        // false-positive here because '-' is a word boundary making \bcommit\b match
        // inside 'commit-graph'.  Phase 4 exact matching correctly rejects this.
        assert!(
            !is_git_commit_or_push("git commit-graph write"),
            "BC-1.18.002 v1.2 EC-011 / VP-105: 'git commit-graph write' MUST return false \
             (Phase 4 exact match: 'commit-graph' ≠ 'commit'; \
             BC-1.18.002 v1.1 clarification; illustrative regex would false-positive here)"
        );
        // [GREEN] EC-019: compound with no advancing segment in either part.
        // Current: accidentally returns false (first segment 'status' → false, stops there).
        // v1.2: Phase 1 splits on &&; segment 1 subcommand = 'status' → false;
        //   segment 2 subcommand = 'log' → false; all-false ⇒ false.
        assert!(
            !is_git_commit_or_push("git status && git log --oneline"),
            "BC-1.18.002 v1.2 EC-019 / VP-105: 'git status && git log --oneline' MUST return false \
             (Phase 1: && splits; both segments are non-advancing; \
             no segment returns true; all-false ⇒ false)"
        );
        // [GREEN] EC-020: non-git executable whose name contains 'git' as substring.
        assert!(
            !is_git_commit_or_push("cat gitfile"),
            "BC-1.18.002 v1.2 EC-020 / VP-105: 'cat gitfile' MUST return false \
             (Phase 2: basename('cat') ≠ 'git'; 'gitfile' is an argument, not the executable)"
        );
    }

    // ── BC-1.18.002 v1.3 — quoting-in-scope (EC-024/025/026) ────────────────
    //
    // MEDIUM-2 fix (S-25.01): Phase 1b POSIX quote-aware tokenization via the
    // `shell_words` crate. Quoted-literal git invocations are now IN SCOPE.
    // These assertions MUST PASS with the v1.3 implementation (green tests).
    // VP-105 unit-test property row (v1.3 quoting clause).
    #[test]
    fn test_BC_1_18_002_is_git_commit_or_push_v1_3_quoting_in_scope_vectors() {
        // EC-024: git "commit" → true (BLOCKED).
        // BC-1.18.002 v1.3 Phase 1b: shell_words tokenizes `git "commit"` →
        // ["git", "commit"]. POSIX quote removal: "commit" → commit.
        // Phase 4: subcommand "commit" matches → blocked.
        // Newly in scope as of v1.3.
        assert!(
            is_git_commit_or_push(r#"git "commit""#),
            "EC-024 / BC-1.18.002 v1.3: 'git \"commit\"' MUST return true \
             (Phase 1b: double-quoted 'commit' → token 'commit' after POSIX quote removal; \
             Phase 4: subcommand 'commit' matches; in scope via shell_words tokenization)"
        );

        // EC-025: git 'push' origin → true (BLOCKED).
        // BC-1.18.002 v1.3 Phase 1b: shell_words tokenizes `git 'push' origin` →
        // ["git", "push", "origin"]. POSIX quote removal: 'push' → push.
        // Phase 4: subcommand "push" matches → blocked.
        assert!(
            is_git_commit_or_push("git 'push' origin"),
            "EC-025 / BC-1.18.002 v1.3: \"git 'push' origin\" MUST return true \
             (Phase 1b: single-quoted 'push' → token 'push' after POSIX quote removal; \
             Phase 4: subcommand 'push' matches; in scope via shell_words tokenization)"
        );

        // EC-026: g'i't commit → true (BLOCKED).
        // BC-1.18.002 v1.3 Phase 1b: shell_words tokenizes `g'i't commit` →
        // ["git", "commit"]. POSIX concatenation: unquoted 'g' + single-quoted 'i' +
        // unquoted 't' character runs concatenate into token "git".
        // Phase 2: basename("git") = "git". Phase 4: subcommand "commit" matches.
        assert!(
            is_git_commit_or_push("g'i't commit"),
            "EC-026 / BC-1.18.002 v1.3: \"g'i't commit\" MUST return true \
             (Phase 1b: POSIX concatenation — unquoted 'g' + single-quoted 'i' + \
             unquoted 't' → token 'git'; Phase 2: basename 'git' matches; \
             Phase 4: subcommand 'commit' matches; in scope via shell_words tokenization)"
        );
    }

    // ── BC-1.18.002 v1.3 — out-of-scope boundary + fail-safe (EC-027/028/029) ─
    //
    // These assertions document the intentional gate boundary per the v1.3 Threat Model.
    // Reliably blocking the forms below is UNDECIDABLE for a static string analysis
    // (Rice's theorem — see BC-1.18.002 v1.3 Threat Model §Out-of-scope).
    //
    // The gate is a fast accidental-misuse interlock for a cooperating agent —
    // NOT an adversary-resistant authorization boundary. These are DOCUMENTED
    // LIMITATIONS, not defects. They are caught by the durable marker (PostToolUse),
    // the ^Agent$ next-advance gate, and GitHub server-side branch protection.
    //
    // DO NOT change these to MUST-block without first revising the BC's Threat Model
    // and obtaining human approval. These tests document the accepted boundary.
    //
    // VP-105 unit-test property row (v1.3 out-of-scope clause).
    #[test]
    fn test_BC_1_18_002_is_git_commit_or_push_v1_3_out_of_scope_boundary_and_fail_safe() {
        // EC-027: $(git commit -m "x") → false (NOT blocked).
        // OUT-OF-SCOPE by design per BC-1.18.002 v1.3 Threat Model — documented limitation, not a bug.
        // The top-level token is `$(git` (command substitution syntax: `$` + `(` are literal
        // characters in shell_words' tokenizer, not expanded). Phase 2: basename("$(git") ≠ "git"
        // → false for this segment. Even if the substitution were expanded, static analysis cannot
        // resolve dynamic command substitution (UNDECIDABLE). Allowed under fail-open posture;
        // caught by the durable marker and other controls.
        assert!(
            !is_git_commit_or_push(r#"$(git commit -m "x")"#),
            "EC-027 / BC-1.18.002 v1.3 OUT-OF-SCOPE by design — documented limitation, not a bug: \
             '$(git commit -m \"x\")' MUST return false (command substitution — top-level token \
             basename is '$(git' ≠ 'git'; dynamic expansion is UNDECIDABLE via static analysis; \
             allowed under fail-open; caught by durable marker and other controls)"
        );

        // EC-028: echo x | xargs git commit → false (NOT blocked).
        // OUT-OF-SCOPE by design per BC-1.18.002 v1.3 Threat Model — documented limitation, not a bug.
        // Phase 1 splits on `|`: segment 1 = "echo x" (exec "echo" ≠ "git" → false);
        // segment 2 = "xargs git commit" (exec "xargs" ≠ "git" → false).
        // `git` is a string argument to xargs, not the statically-literal top-level executable.
        // Indirection via xargs/bash -c is UNDECIDABLE for static analysis.
        assert!(
            !is_git_commit_or_push("echo x | xargs git commit"),
            "EC-028 / BC-1.18.002 v1.3 OUT-OF-SCOPE by design — documented limitation, not a bug: \
             'echo x | xargs git commit' MUST return false (Phase 1 splits on |; \
             segment 1 exec 'echo' ≠ 'git'; segment 2 exec 'xargs' ≠ 'git'; \
             xargs indirection is UNDECIDABLE via static analysis; allowed under fail-open; \
             caught by durable marker and other controls)"
        );

        // EC-029: eval "git push origin main" → false (NOT blocked).
        // OUT-OF-SCOPE by design per BC-1.18.002 v1.3 Threat Model — documented limitation, not a bug.
        // shell_words tokenizes `eval "git push origin main"` → ["eval", "git push origin main"]
        // (the double-quoted string is a single token; spaces preserved inside quotes).
        // Phase 2: executable is "eval", basename "eval" ≠ "git" → false.
        // Dynamic eval is UNDECIDABLE via static string analysis (Rice's theorem).
        assert!(
            !is_git_commit_or_push(r#"eval "git push origin main""#),
            "EC-029 / BC-1.18.002 v1.3 OUT-OF-SCOPE by design — documented limitation, not a bug: \
             'eval \"git push origin main\"' MUST return false (executable is 'eval' not 'git'; \
             dynamic evaluation is UNDECIDABLE via static analysis; allowed under fail-open; \
             caught by durable marker and other controls)"
        );

        // ── Fail-safe: unterminated quote → shell_words Err → true (BLOCKED) ─
        //
        // When shell_words::split returns Err (mismatched/unterminated quote), the segment
        // is unparseable. The conservative posture is to BLOCK (return true) — an
        // undecidable/malformed command must not be silently allowed through the gate.
        // Per BC-1.18.002 v1.3 evaluate_git_segment: Err branch returns true.
        assert!(
            is_git_commit_or_push(r#"git "commit"#),
            "BC-1.18.002 v1.3 fail-safe: 'git \"commit' (unterminated double-quote) MUST return true \
             (shell_words::split returns Err on unmatched quote; evaluate_git_segment Err arm = true; \
             conservative posture: undecidable input treated as gated to avoid silent pass-through)"
        );
    }

    // ── EC-030: unreadable marker → Allow (fail-open, INV2 self-lock prevention) ─

    /// EC-030 / BC-1.18.002 v1.4 + INV2 (self-lock avoidance):
    /// A non-NotFound read error on the marker file MUST yield `GateDecision::Allow`
    /// (fail-open).
    ///
    /// Mechanism: passing a directory path to `evaluate_gate` triggers
    /// `std::fs::read_to_string(dir_path)` → `Err` with kind `IsADirectory` (not
    /// `NotFound`), reliably exercising the `Err(other) => Allow` arm on all platforms
    /// without permission games or root-bypass flakiness (chmod 0o000 would make the
    /// test falsely pass/fail when running as root; the directory trick is unconditional).
    ///
    /// Rationale (BC-1.18.002 v1.4 EC-030 + INV2): the marker bytes could not be read,
    /// so the quarantine state is unknown. Critically,
    /// `rm .factory/unvalidated-mutation.marker` (the operator escape hatch) would ALSO
    /// fail under the same filesystem fault — meaning a Block here would create an
    /// unclearable self-lock (INV2 violation). Therefore: unreadable marker → fail-open.
    #[test]
    fn test_BC_1_18_002_evaluate_gate_unreadable_marker_allow_ec030() {
        // Pass a directory path as the marker path.
        // read_to_string on a directory returns Err whose kind is NOT NotFound
        // (it is IsADirectory on Unix / PermissionDenied on Windows),
        // reliably hitting the Err(other) fail-open arm of evaluate_gate.
        let dir = tempfile::tempdir().expect("tempdir");
        // The tempdir root IS a directory — use its path directly as the marker path.
        let dir_path = dir.path();

        // Pre-condition: path exists and is a directory (not a regular file).
        assert!(
            dir_path.exists(),
            "pre-condition: directory path must exist for this test"
        );
        assert!(
            dir_path.is_dir(),
            "pre-condition: path must be a directory, not a regular file"
        );

        let decision = evaluate_gate(dir_path);

        // EC-030 + INV2: unreadable marker → fail-open (Allow).
        // Blocking on an unreadable marker would create an unclearable self-lock because
        // the same filesystem fault that prevents reading also prevents `rm` from clearing
        // the marker (INV2 violation: self-lock is unclearable by the operator escape hatch).
        assert!(
            matches!(decision, GateDecision::Allow),
            "EC-030 / BC-1.18.002 v1.4 + INV2 self-lock prevention: \
             non-NotFound read error MUST yield GateDecision::Allow (fail-open) — \
             got {decision:?}"
        );
    }

    // ── EC-008: readable-but-malformed marker → Block (conservative posture) ───

    /// EC-008 / BC-1.18.002:
    /// A marker file that exists but contains invalid TOML MUST yield
    /// `GateDecision::Block` with sentinel values in the required fields (conservative
    /// posture — still block, surface parse failure).
    ///
    /// The logic: `toml::from_str` returns `Err` on malformed content; the fallback path
    /// returns `Block` with `plugin_name = "<unparseable-marker>"`,
    /// `cause = "<unknown>"`, and empty `artifact_path` / `trace_id`.
    ///
    /// Rationale: a corrupt marker is evidence that something wrote to the quarantine
    /// path. Fail-open here would silently allow all dispatches through a potentially
    /// quarantined session. Conservative posture: block and surface the parse failure
    /// via sentinel fields so the operator can diagnose the corrupt marker.
    #[test]
    fn test_BC_1_18_002_evaluate_gate_malformed_toml_marker_block_ec008() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");

        // Write invalid TOML to the marker file (EC-008: malformed / corrupt marker).
        // Content is syntactically invalid TOML; also lacks the required fields.
        std::fs::write(&marker_path, b"this is not valid toml {{{")
            .expect("test setup: write malformed marker");

        let decision = evaluate_gate(&marker_path);

        // EC-008: readable-but-malformed marker MUST block (conservative posture).
        assert!(
            matches!(decision, GateDecision::Block { .. }),
            "EC-008 / BC-1.18.002: malformed TOML marker MUST yield GateDecision::Block \
             (conservative posture — file exists but is corrupt; sentinel fields surface \
             parse failure) — got {decision:?}"
        );

        // Sentinel fields MUST be present so the operator can diagnose the corrupt marker.
        if let GateDecision::Block {
            plugin_name, cause, ..
        } = decision
        {
            assert_eq!(
                plugin_name, "<unparseable-marker>",
                "EC-008: malformed marker block MUST use sentinel plugin_name \
                 '<unparseable-marker>' — got: {plugin_name}"
            );
            assert_eq!(
                cause, "<unknown>",
                "EC-008: malformed marker block MUST use sentinel cause '<unknown>' \
                 — got: {cause}"
            );
        }
    }

    // ── LOW-6: block message content (AC-007 / AC-008) ───────────────────────

    /// LOW-6: `on_pre_tool_use_impl` produces a JSON block message whose ACTUAL
    /// emitted structure (not a reconstruction) MUST contain:
    ///   - `marker_plugin_name` — the plugin_name from the marker TOML
    ///   - `recovery.tier_1_revalidate` — references the blocking plugin name (BC-1.18.002 v1.6 PC5 Tier-1)
    ///   - `recovery.tier_3_operator_break_glass` — human-only rm reference, MUST NOT instruct agent to rm (PC5 Tier-3)
    ///   - be machine-parseable (valid JSON)
    ///
    /// AC-007 / AC-008 / BC-1.18.002 INV4.
    ///
    /// **Adversary OBS fix (S-25.01):** the previous version of this test
    /// *reconstructed* the block-message JSON using a copied `serde_json::json!`
    /// shape, then asserted on that copy — validating a duplicate, not the real
    /// production output. A future change to `on_pre_tool_use_impl`'s actual JSON
    /// shape would NOT have been caught. This rewrite calls `on_pre_tool_use_impl`
    /// directly with a real `HookPayload` (Agent arm AND Bash git commit arm) and
    /// asserts on the actual returned `HookResult::Block { reason }`.
    ///
    /// The `marker_path` parameter on `on_pre_tool_use_impl` is the injection point
    /// that makes native unit tests viable without writing to the real `.factory/`
    /// directory or depending on WASI preopened-directory semantics.
    #[test]
    fn test_BC_1_18_002_block_message_contains_required_fields_and_escape_hatch() {
        use super::on_pre_tool_use_impl;
        use vsdd_hook_sdk::HookResult;

        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");

        let expected_plugin = "validate-factory-path-staging";
        let expected_artifact = "/path/to/.factory/STATE.md";
        let expected_cause = "fuel";
        let expected_trace = "trace-low6-test";

        std::fs::write(
            &marker_path,
            format!(
                "timestamp = \"2026-08-31T00:00:00Z\"\n\
                 plugin_name = \"{expected_plugin}\"\n\
                 artifact_path = \"{expected_artifact}\"\n\
                 cause = \"{expected_cause}\"\n\
                 trace_id = \"{expected_trace}\"\n"
            ),
        )
        .expect("test setup: write marker");

        // ── Arm 1: Agent dispatch ──────────────────────────────────────────────
        // AC-007: when marker is present, an Agent PreToolUse dispatch MUST produce
        // HookResult::Block with a machine-parseable JSON reason.
        let agent_payload: vsdd_hook_sdk::HookPayload = serde_json::from_str(
            r#"{
                "event_name": "PreToolUse",
                "tool_name": "Agent",
                "session_id": "test-sess-low6",
                "dispatcher_trace_id": "test-trace-low6",
                "tool_input": {
                    "subagent_type": "vsdd-factory:state-manager",
                    "prompt": "advance state"
                }
            }"#,
        )
        .expect("agent payload should deserialize");

        let agent_result = on_pre_tool_use_impl(agent_payload, &marker_path);
        let agent_reason = match agent_result {
            HookResult::Block { ref reason } => reason.clone(),
            other => panic!(
                "AC-007: Agent arm MUST produce HookResult::Block when marker present — got {other:?}"
            ),
        };

        // Assert on the ACTUAL emitted reason (not a reconstruction).
        // AC-007: block reason MUST be valid JSON
        let agent_parsed: serde_json::Value = serde_json::from_str(&agent_reason)
            .expect("AC-007 / BC-1.18.002 INV4: Agent arm block reason MUST be valid JSON");

        // AC-007: marker plugin_name MUST appear in the block message
        assert_eq!(
            agent_parsed
                .get("marker_plugin_name")
                .and_then(|v| v.as_str()),
            Some(expected_plugin),
            "AC-007 / BC-1.18.002 INV4: 'marker_plugin_name' MUST equal marker plugin_name '{expected_plugin}'"
        );

        // AC-007: recovery object MUST be present with all three PC5 tier subfields
        let agent_recovery = agent_parsed.get("recovery").expect(
            "AC-007 / BC-1.18.002 INV4: block reason MUST have a structured 'recovery' field",
        );

        // PC5 Tier-1: must reference the blocking plugin name so the agent knows which
        // artifact/validator to re-run.
        let tier_1 = agent_recovery
            .get("tier_1_revalidate")
            .and_then(|v| v.as_str())
            .expect("AC-007 (BC-1.18.002 v1.6 PC5 Tier-1): recovery MUST have 'tier_1_revalidate' subfield");
        assert!(
            tier_1.contains(expected_plugin),
            "AC-007 (PC5 Tier-1): tier_1_revalidate MUST reference the marker plugin_name \
             '{expected_plugin}' — got: {tier_1}"
        );

        // PC5 Tier-3: must reference the marker path for the human operator, but MUST NOT
        // instruct this agent to run rm (BC-1.18.002 v1.6 PC5 Tier-3).
        let tier_3 = agent_recovery
            .get("tier_3_operator_break_glass")
            .and_then(|v| v.as_str())
            .expect("AC-007 (BC-1.18.002 v1.6 PC5 Tier-3): recovery MUST have 'tier_3_operator_break_glass' subfield");
        assert!(
            tier_3.contains(".factory/unvalidated-mutation.marker"),
            "AC-007 (PC5 Tier-3): tier_3_operator_break_glass MUST reference the marker path \
             '.factory/unvalidated-mutation.marker' — got: {tier_3}"
        );
        // PC5 key invariant: the break-glass message MUST be framed as a human-only operation.
        // The agent MUST NOT be directed to rm. Check for "MUST NOT" or "Human operator" framing.
        assert!(
            tier_3.to_ascii_uppercase().contains("MUST NOT")
                || tier_3.contains("Human operator")
                || tier_3.contains("human operator"),
            "AC-007 (BC-1.18.002 v1.6 PC5): tier_3_operator_break_glass MUST frame rm as \
             human-only (contains 'MUST NOT' or 'Human operator') — got: {tier_3}"
        );

        // ── Arm 2: Bash git commit dispatch ───────────────────────────────────
        // AC-008: when marker is present, a Bash git commit PreToolUse dispatch
        // MUST also produce HookResult::Block with the same structured JSON reason.
        let bash_payload: vsdd_hook_sdk::HookPayload = serde_json::from_str(
            r#"{
                "event_name": "PreToolUse",
                "tool_name": "Bash",
                "session_id": "test-sess-low6",
                "dispatcher_trace_id": "test-trace-low6",
                "tool_input": {
                    "command": "git commit -m 'fix: S-25.01 block-message test invokes production path'"
                }
            }"#,
        )
        .expect("bash payload should deserialize");

        let bash_result = on_pre_tool_use_impl(bash_payload, &marker_path);
        let bash_reason = match bash_result {
            HookResult::Block { ref reason } => reason.clone(),
            other => panic!(
                "AC-008: Bash git commit arm MUST produce HookResult::Block when marker present \
                 — got {other:?}"
            ),
        };

        // Assert on the ACTUAL emitted reason from the Bash arm.
        let bash_parsed: serde_json::Value = serde_json::from_str(&bash_reason)
            .expect("AC-008 / BC-1.18.002 INV4: Bash arm block reason MUST be valid JSON");

        assert_eq!(
            bash_parsed
                .get("marker_plugin_name")
                .and_then(|v| v.as_str()),
            Some(expected_plugin),
            "AC-008 / BC-1.18.002 INV4: Bash arm 'marker_plugin_name' MUST equal \
             marker plugin_name '{expected_plugin}'"
        );
        // PC5 Tier-3: Bash arm must also have human-only operator break-glass reference.
        let bash_tier_3 = bash_parsed
            .get("recovery")
            .and_then(|r| r.get("tier_3_operator_break_glass"))
            .and_then(|v| v.as_str())
            .expect(
                "AC-008 (BC-1.18.002 v1.6 PC5 Tier-3): Bash arm recovery MUST have \
                     'tier_3_operator_break_glass' subfield",
            );
        assert!(
            bash_tier_3.contains(".factory/unvalidated-mutation.marker"),
            "AC-008 (PC5 Tier-3): Bash arm tier_3_operator_break_glass MUST reference the \
             marker path '.factory/unvalidated-mutation.marker' — got: {bash_tier_3}"
        );
    }

    // ── ADR-048 §Decision 4 v1.2: evaluate_gate is a pure presence check ─────

    /// ADR-048 §D4 v1.2 (S-25.01 LOCAL adversary pass 2 F-P2-002 HIGH): a marker
    /// with an ELAPSED `expires_at` still yields `GateDecision::Block` from
    /// `evaluate_gate` — TTL determination is no longer this function's job.
    /// It is performed dispatcher-native, BEFORE this plugin ever runs
    /// (`indeterminate_marker::check_and_clear_expired_marker`), so in normal
    /// operation `evaluate_gate` never actually observes an expired marker;
    /// this test asserts the DEFENSE-IN-DEPTH behavior directly — presence
    /// alone blocks, unconditionally, with no TTL awareness of its own.
    #[test]
    fn test_ADR_048_D4_evaluate_gate_ignores_expires_at_blocks_on_presence_alone() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        std::fs::write(
            &marker_path,
            "timestamp = \"2020-01-01T00:00:00Z\"\n\
             plugin_name = \"p\"\n\
             artifact_path = \"\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-expired\"\n\
             expires_at = \"2020-01-02T00:00:00Z\"\n",
        )
        .expect("write expired marker");
        let decision = evaluate_gate(&marker_path);
        assert!(
            matches!(decision, GateDecision::Block { .. }),
            "ADR-048 §D4 v1.2: evaluate_gate MUST Block on marker presence alone, \
             regardless of expires_at — got {decision:?}"
        );
        assert!(
            marker_path.exists(),
            "ADR-048 §D4 v1.2: evaluate_gate MUST NOT delete the marker — deletion is \
             dispatcher-native (check_and_clear_expired_marker), not this function's job"
        );
    }

    /// BC-1.18.003 PC4/INV5: future expires_at → evaluate_gate returns Block.
    ///
    /// Non-expired marker (expires_at >> now): the gate MUST block the dispatch.
    #[test]
    fn test_BC_1_18_003_evaluate_gate_future_expires_at_blocks() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        std::fs::write(
            &marker_path,
            "timestamp = \"2026-08-31T00:00:00Z\"\n\
             plugin_name = \"gate-plugin\"\n\
             artifact_path = \"/some/artifact.md\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-future\"\n\
             expires_at = \"2099-01-01T00:00:00Z\"\n",
        )
        .expect("write future marker");
        let decision = evaluate_gate(&marker_path);
        assert!(
            matches!(decision, GateDecision::Block { .. }),
            "BC-1.18.003 INV5: non-expired marker MUST return Block"
        );
    }

    /// BC-1.18.003: legacy marker (missing expires_at) → evaluate_gate returns Block (conservative).
    ///
    /// Pre-ADR-048 markers have no expires_at. evaluate_gate treats missing expires_at as
    /// non-expired (conservative block) per ADR-048 §D2 spec.
    #[test]
    fn test_BC_1_18_003_evaluate_gate_missing_expires_at_blocks_legacy() {
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        std::fs::write(
            &marker_path,
            "timestamp = \"2026-08-31T00:00:00Z\"\n\
             plugin_name = \"legacy\"\n\
             artifact_path = \"\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-legacy\"\n",
        )
        .expect("write legacy marker (no expires_at)");
        let decision = evaluate_gate(&marker_path);
        assert!(
            matches!(decision, GateDecision::Block { .. }),
            "BC-1.18.003: legacy marker (missing expires_at) MUST return Block (conservative)"
        );
    }

    // ── BC-1.18.002 INV6/VP-107: ungated-escape invariant ──────────────────────

    /// BC-1.18.002 INV6/VP-107: `is_git_commit_or_push("rm ...")` returns false.
    ///
    /// The operator escape hatch (`rm .factory/unvalidated-mutation.marker`) MUST NOT be
    /// matched by Arm 2 (is_git_commit_or_push). rm is not git commit or push.
    #[test]
    fn test_BC_1_18_002_INV6_rm_escape_hatch_is_not_gated_by_arm2() {
        assert!(
            !is_git_commit_or_push("rm .factory/unvalidated-mutation.marker"),
            "BC-1.18.002 INV6/VP-107: 'rm .factory/unvalidated-mutation.marker' MUST NOT be \
             matched by is_git_commit_or_push — the rm escape hatch must always be ungated"
        );
    }

    /// BC-1.18.002 INV6/VP-107: Bash dispatch `rm .factory/unvalidated-mutation.marker`
    /// + active marker → on_pre_tool_use_impl returns Continue (Arm 2 must not gate rm).
    #[test]
    fn test_BC_1_18_002_INV6_bash_rm_dispatch_allowed_with_active_marker() {
        use super::on_pre_tool_use_impl;
        use vsdd_hook_sdk::HookResult;
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        std::fs::write(
            &marker_path,
            "timestamp = \"2026-08-31T00:00:00Z\"\n\
             plugin_name = \"p\"\n\
             artifact_path = \"\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-escape-bash\"\n\
             expires_at = \"2099-01-01T00:00:00Z\"\n",
        )
        .expect("write active marker");
        let payload: vsdd_hook_sdk::HookPayload = serde_json::from_value(serde_json::json!({
            "event_name": "PreToolUse",
            "tool_name": "Bash",
            "session_id": "test-escape-1",
            "dispatcher_trace_id": "trace-escape-bash",
            "tool_input": {
                "command": "rm .factory/unvalidated-mutation.marker"
            }
        }))
        .expect("deserialize Bash rm payload");
        let result = on_pre_tool_use_impl(payload, &marker_path);
        assert!(
            matches!(result, HookResult::Continue),
            "BC-1.18.002 INV6/VP-107: Bash 'rm .factory/unvalidated-mutation.marker' MUST \
             produce Continue — rm is the operator escape hatch and must never be gated by Arm 2"
        );
    }

    /// BC-1.18.002 INV6: Bash dispatch with a non-advancing command (`cargo clippy`) +
    /// active marker → on_pre_tool_use_impl returns Continue (Arm 2 only gates git commit/push).
    ///
    /// `on_pre_tool_use_impl` is only invoked by the registry for Agent and Bash tool calls.
    /// For Bash dispatches, only git commit/push subcommands fall through to evaluate_gate.
    /// All other Bash commands — including `cargo clippy`, `cargo test`, `ls`, etc. — are
    /// ungated regardless of marker presence.
    #[test]
    fn test_BC_1_18_002_INV6_non_git_bash_command_not_gated() {
        use super::on_pre_tool_use_impl;
        use vsdd_hook_sdk::HookResult;
        let dir = tempfile::tempdir().expect("tempdir");
        let marker_path = dir.path().join("unvalidated-mutation.marker");
        std::fs::write(
            &marker_path,
            "timestamp = \"2026-08-31T00:00:00Z\"\n\
             plugin_name = \"p\"\n\
             artifact_path = \"\"\n\
             cause = \"fuel\"\n\
             trace_id = \"trace-clippy\"\n\
             expires_at = \"2099-01-01T00:00:00Z\"\n",
        )
        .expect("write active marker");
        let payload: vsdd_hook_sdk::HookPayload = serde_json::from_value(serde_json::json!({
            "event_name": "PreToolUse",
            "tool_name": "Bash",
            "session_id": "test-clippy",
            "dispatcher_trace_id": "trace-clippy",
            "tool_input": {
                "command": "cargo clippy --workspace --all-targets -- -D warnings"
            }
        }))
        .expect("deserialize Bash cargo clippy payload");
        let result = on_pre_tool_use_impl(payload, &marker_path);
        assert!(
            matches!(result, HookResult::Continue),
            "BC-1.18.002 INV6: Bash 'cargo clippy' (non-git command) MUST produce Continue \
             — Arm 2 only gates git commit/push, not arbitrary Bash commands"
        );
    }
}
