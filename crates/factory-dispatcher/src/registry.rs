//! `hooks-registry.toml` parser and type declarations.
//!
//! The registry is the dispatcher's source of truth for *which* plugins
//! exist, *what* events they react to, and *what capabilities* they
//! hold. S-1.2 defines the types; S-1.5 consumes capabilities during
//! plugin instantiation; S-2.2 auto-generates the file from the existing
//! v0.79.x hooks.json.
//!
//! ## S-18.00: PreCompact / PostCompact event-string parsing
//!
//! The [`parse_event_type`] function parses the `event` string field from a
//! `[[hooks]]` entry in hooks-registry.toml into the typed [`crate::invoke::EventType`]
//! enum (BC-1.15.001 INV1). `"PreCompact"` and `"PostCompact"` are valid event strings
//! that MUST NOT produce `RegistryError::UnknownEvent`.
//!
//! `parse_event_type` is implemented: it delegates to
//! [`crate::invoke::EventType::from_event_str`], the single source of truth for
//! event-string → enum mapping (BC-1.15.001 INV1).

use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use thiserror::Error;

/// Current registry schema version. The loader refuses anything else
/// so an unreleased schema change can't silently mis-parse.
///
/// S-15.01 (AC-001, BC-7.06.001 postcondition 1): bumped to 2 for per-plugin
/// `async` field support and partition semantics (ADR-019). A registry with
/// `schema_version != 2` produces E-REG-001 at load time (fail-closed).
pub const REGISTRY_SCHEMA_VERSION: u32 = 2;

#[derive(Debug, Error)]
pub enum RegistryError {
    #[error("registry file not found: {0}")]
    NotFound(PathBuf),
    #[error("registry read failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("registry parse failed: {0}")]
    Toml(#[from] toml::de::Error),
    #[error(
        "registry schema_version = {got}, dispatcher expects {expected}. \
         Regenerate hooks-registry.toml or upgrade the dispatcher. \
         [E-REG-001]"
    )]
    SchemaVersion { got: u32, expected: u32 },
    #[error("registry entry '{name}': invalid tool regex `{pattern}`: {source}")]
    ToolRegex {
        name: String,
        pattern: String,
        #[source]
        source: regex::Error,
    },
    /// E-REG-002: entry has an `on_error` value that can block (`"block"` or
    /// `"block_if_marker"`) AND `async = true`.
    /// Enforced at registry-load time; dispatcher refuses to start.
    /// (BC-1.14.001 Invariant 4, BC-7.06.001 Invariant 1, ADR-048 §Decision 1)
    ///
    /// `on_error` carries the ACTUAL offending value (e.g. `"block"` or
    /// `"block_if_marker"`) so the Display message never misreports which
    /// variant triggered the invariant (LOW-1).
    #[error(
        "registry entry '{name}': on_error = \"{on_error}\" combined with async = true is forbidden \
         (BC-7.06.001 Invariant 1). Classify this plugin async = false or remove on_error = \"{on_error}\". \
         [E-REG-002]"
    )]
    AsyncBlockConflict { name: String, on_error: String },
    /// E-REG-003: duplicate (name, event, tool) tuple across [[hooks]] entries.
    /// Enforced at registry-load time; dispatcher refuses to start.
    /// (BC-7.06.001 Invariant 7, F-P8-001)
    #[error(
        "[E-REG-003] Duplicate hook entry: name={name}, event={event}, tool={tool:?} \
         (BC-7.06.001 Invariant 7). Each (name, event, tool) tuple must be unique \
         across all [[hooks]] entries."
    )]
    DuplicateEntry {
        name: String,
        event: String,
        tool: Option<String>,
    },
}

/// Outcome for a plugin that returns `Error` or crashes. `Continue` is
/// the default; operators opt into hard-stop behavior per plugin.
///
/// # ADR-048 §Decision 1 — block_if_marker
///
/// `BlockIfMarker` is the third value, added by S-25.01. When a plugin registered
/// with `on_error = "block_if_marker"` crashes, fuel-exhausts, or times out, the
/// dispatcher executes a NATIVE (non-WASM) filesystem check:
/// - Non-expired `.factory/unvalidated-mutation.marker` exists → Block (exit 2).
/// - Marker absent, expired, or unreadable → Allow (exit 0).
///
/// This closes the CWE-636 gap where crash + valid-quarantine-signal → silent allow
/// under the old `"continue"` policy (D-1135 reversed).
///
/// `BlockIfMarker` MUST NOT be combined with `async = true` (same invariant as
/// `Block` — async hooks never affect gate decisions, E-REG-002).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum OnError {
    #[default]
    Continue,
    Block,
    /// Conditionally fail-closed on crash: block iff a non-expired
    /// `.factory/unvalidated-mutation.marker` exists (ADR-048 §Decision 1).
    BlockIfMarker,
}

/// Resource-exhaustion failure policy for a plugin entry.
///
/// Governs dispatcher behavior when a plugin exhausts its WASM fuel budget
/// (`TimeoutCause::Fuel`) or epoch budget (`TimeoutCause::Epoch`). This axis
/// is independent of `on_error`, which governs plugin crashes and host-side
/// invocation errors (ADR-039 §Decision 1 axes-separation requirement;
/// BC-1.01.016 PC5).
///
/// Serializes and deserializes as kebab-case (`"fail-closed"`, `"fail-open"`)
/// per ADR-039 §Decision 2 value format. Snake-case variants (`"fail_closed"`)
/// are NOT accepted — serde rejects them at parse time, satisfying
/// BC-1.01.016 EC-003 (underscored variants must be `Err`).
///
/// Defaults to `FailOpen` for backward-compatibility with existing registry
/// entries that predate this field (ADR-039 §Decision 1 backward-compat clause;
/// BC-1.01.016 PC4). Phase 1 (S-21.10): schema extension only; enforcement
/// flip in S-21.11.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum FailurePolicy {
    /// Plugin resource-exhaustion causes a hard block; the same terminal outcome
    /// as `OnError::Block` but for the orthogonal fuel/epoch failure class.
    FailClosed,
    /// Plugin resource-exhaustion is advisory (fail-open). Default for all
    /// existing plugins that predate this field (ADR-039 §Decision 1).
    #[default]
    FailOpen,
}

/// Capability declaration for a plugin entry. Deny-by-default — a
/// missing block means the plugin cannot use the corresponding host
/// function at all.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default)]
    pub exec_subprocess: Option<ExecSubprocessCaps>,
    #[serde(default)]
    pub read_file: Option<ReadFileCaps>,
    /// Write-file capability declaration (BC-2.02.011).
    /// Deny-by-default: absence of this block causes every `write_file`
    /// call to return `CAPABILITY_DENIED (-1)`.
    #[serde(default)]
    pub write_file: Option<WriteFileCaps>,
    /// Bounded prefix-read capability (BC-1.17.001, S-19.06).
    /// Independent of `read_file` — a plugin with only `read_file` capability
    /// cannot call `read_prefix` (BC-1.17.001 Invariant 3).
    #[serde(default)]
    pub read_prefix: Option<ReadPrefixCaps>,
    /// Environment variable names the plugin is allowed to read.
    #[serde(default)]
    pub env_allow: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ExecSubprocessCaps {
    /// Binary basename allow-list. The dispatcher resolves each entry
    /// to a full path at registry load time (S-1.5 enforces).
    pub binary_allow: Vec<String>,
    /// Opt-in to invoking shell interpreters (bash/sh/zsh/pwsh). The
    /// field name is deliberate: the operator has acknowledged the
    /// blast-radius increase.
    #[serde(default)]
    pub shell_bypass_acknowledged: Option<String>,
    /// Working-directory allow-list, relative to `CLAUDE_PROJECT_DIR`.
    /// Empty means "no cwd override permitted".
    #[serde(default)]
    pub cwd_allow: Vec<String>,
    /// Environment variable names permitted to be forwarded to the
    /// subprocess.
    #[serde(default)]
    pub env_allow: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadFileCaps {
    /// Path prefixes the plugin is allowed to read, rooted at
    /// `CLAUDE_PROJECT_DIR` unless absolute.
    pub path_allow: Vec<String>,
}

/// Capability declaration for `host::read_prefix` (BC-1.17.001, S-19.06).
/// Independent of [`ReadFileCaps`] — a plugin with `capabilities.read_file`
/// does NOT automatically receive `capabilities.read_prefix` (defense-in-depth,
/// BC-1.17.001 Invariant 3). Deny-by-default: absence of this block causes
/// every `vsdd::read_prefix` host call to return `CAPABILITY_DENIED (-1)`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ReadPrefixCaps {
    /// Path prefixes the plugin is allowed to read via `read_prefix`, rooted at
    /// `CLAUDE_PROJECT_DIR` unless absolute. Same path-allow semantics as
    /// `ReadFileCaps::path_allow`.
    pub path_allow: Vec<String>,
}

/// Capability declaration for `host::write_file` (BC-2.02.011).
/// Parallel to [`ReadFileCaps`]. Deny-by-default: absence of this block
/// causes every `vsdd::write_file` host call to return `CAPABILITY_DENIED`.
#[derive(Debug, Clone, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WriteFileCaps {
    /// Path prefixes the plugin is allowed to write, rooted at
    /// `CLAUDE_PROJECT_DIR` unless absolute.
    pub path_allow: Vec<String>,
    /// Optional per-call byte cap. When set, overrides the `max_bytes`
    /// argument if the argument exceeds this value. If `None`, the
    /// `max_bytes` argument is used as-is (BC-2.02.011 invariant 2:
    /// `max_bytes` is mandatory; no opt-out is permitted).
    #[serde(default)]
    pub max_bytes_per_call: Option<u32>,
}

/// Registry-wide defaults, applied when a per-entry field is missing.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields, default)]
pub struct RegistryDefaults {
    pub timeout_ms: u32,
    pub fuel_cap: u64,
    pub on_error: OnError,
    pub priority: u32,
}

impl Default for RegistryDefaults {
    fn default() -> Self {
        Self {
            timeout_ms: 5_000,
            // ADR-042 §Decision 1: raised 10M → 20M (measurement-validated).
            // Uses DEFAULT_FUEL_CAP — the single source of truth — so any future
            // cap change propagates atomically to both Default impls.
            fuel_cap: crate::invoke::DEFAULT_FUEL_CAP,
            on_error: OnError::Continue,
            priority: 500,
        }
    }
}

/// A single plugin registration.
///
/// `Eq` was deliberately dropped from the derives when the `config`
/// field landed: `toml::Value` carries `Float`, which is `PartialEq` but
/// not `Eq`. Tests use `assert_eq!`, which only needs `PartialEq`.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct RegistryEntry {
    /// Stable identifier, e.g. `"capture-commit-activity"`. Unique
    /// within a registry.
    pub name: String,

    /// Claude Code event this entry reacts to, e.g. `"PostToolUse"`.
    pub event: String,

    /// Regex matched against the tool name. `None` (the default) means
    /// "all tools".
    #[serde(default)]
    pub tool: Option<String>,

    /// Relative or absolute path to the plugin `.wasm` file.
    pub plugin: PathBuf,

    /// Ordering tier. Lower fires first. Same-priority plugins execute
    /// in parallel (S-1.6). Default inherited from `RegistryDefaults`.
    #[serde(default)]
    pub priority: Option<u32>,

    /// `false` skips the entry entirely.
    #[serde(default = "default_enabled")]
    pub enabled: bool,

    /// Per-call wall-clock budget. Default inherited from
    /// `RegistryDefaults.timeout_ms`.
    #[serde(default)]
    pub timeout_ms: Option<u32>,

    /// Per-call wasmtime fuel budget. Default inherited from
    /// `RegistryDefaults.fuel_cap`.
    #[serde(default)]
    pub fuel_cap: Option<u64>,

    /// What to do if the plugin errors. Default inherited from
    /// `RegistryDefaults.on_error`.
    #[serde(default)]
    pub on_error: Option<OnError>,

    /// Deny-by-default capability declaration. `None` means the plugin
    /// gets no host-function access beyond the always-on APIs
    /// (`log`, `emit_event`, `session_id`, etc.).
    #[serde(default)]
    pub capabilities: Option<Capabilities>,

    /// Per-plugin configuration. The dispatcher forwards this verbatim
    /// as `plugin_config` on the [`HookPayload`] handed to the plugin.
    /// Schema is plugin-defined; the registry only carries it through.
    /// Default is an empty TOML table — distinguishable from "missing"
    /// by plugin code that wants to require keys (the legacy-bash-
    /// adapter, S-2.1, demands `script_path`).
    ///
    /// [`HookPayload`]: vsdd_hook_sdk::HookPayload
    #[serde(default = "default_config")]
    pub config: toml::Value,

    /// Per-plugin async classification (S-15.01, BC-7.06.001 postcondition 2).
    ///
    /// - `async = true`: plugin is fire-and-forget (async_group). Its verdict
    ///   never affects the dispatcher exit code. Suitable for telemetry-only
    ///   plugins. MUST NOT be combined with `on_error = "block"` (E-REG-002).
    /// - `async = false` (default, including absent field): plugin is in the
    ///   sync_group. The dispatcher awaits its completion; a block verdict gates
    ///   Claude Code. The serde-default semantics (absent = false) ensure all
    ///   existing registry entries are treated as sync-group plugins without
    ///   any TOML file migration (BC-7.06.001 postcondition 3).
    ///
    /// Renamed to `async_flag` in Rust source because `async` is a reserved
    /// keyword. The TOML key remains `async`.
    ///
    /// ASYNC_DRAIN_WINDOW_MS for async group tasks is defined in DI-019 —
    /// cite by reference, do NOT hardcode the value (Decision 4).
    #[serde(default, rename = "async")]
    pub async_flag: bool,

    /// Resolver names this hook requires for context injection.
    ///
    /// Each name must match a `name` entry in `resolvers-registry.toml`.
    /// Defaults to `[]` (empty) so existing hooks-registry.toml entries that
    /// omit this field parse correctly (BC-1.13.001 INV3). An empty list
    /// means the dispatcher skips resolver invocation entirely — zero overhead
    /// on the dispatch hot path (BC-1.13.001 PC3).
    ///
    /// `#[serde(default)]` is mandatory here per BC-1.13.001 invariant 3.
    /// Do NOT use `Option<Vec<String>>` — use `Vec<String>` with `default` so
    /// the absent field deserializes to `[]`, not `None`.
    #[serde(default)]
    pub needs_context: Vec<String>,

    /// Per-plugin resource-exhaustion failure policy (S-21.10; ADR-039 §Decision 1+2).
    ///
    /// Governs what happens when this plugin exhausts its WASM fuel budget
    /// (`TimeoutCause::Fuel`) or epoch budget (`TimeoutCause::Epoch`). Independent
    /// of `on_error` — the two fields govern distinct failure classes:
    /// `on_error` handles plugin crashes and host-side invocation errors;
    /// `failure_policy` handles resource exhaustion (ADR-039 §Decision 1
    /// axes-separation; BC-1.01.016 PC5).
    ///
    /// Absent field defaults to `FailurePolicy::FailOpen` via `#[serde(default)]`
    /// (ADR-039 §Decision 1 backward-compat clause; BC-1.01.016 PC4). This is
    /// deliberate per-plugin granularity — NOT a global `RegistryDefaults` key
    /// (ADR-039 §Decision 2 per-plugin-granularity note; SR-003). Do NOT refactor
    /// to `Option<FailurePolicy>` — the field-level `#[serde(default)]` is the
    /// intentional design.
    ///
    /// Phase 1 (S-21.10): schema extension only. `plugin_fail_closed` behavior
    /// is UNCHANGED in this phase; enforcement flip in S-21.11 (AC-006).
    #[serde(default)]
    pub failure_policy: FailurePolicy,
}

fn default_enabled() -> bool {
    true
}

fn default_config() -> toml::Value {
    toml::Value::Table(toml::Table::new())
}

impl RegistryEntry {
    pub fn priority(&self, defaults: &RegistryDefaults) -> u32 {
        self.priority.unwrap_or(defaults.priority)
    }

    pub fn timeout_ms(&self, defaults: &RegistryDefaults) -> u32 {
        self.timeout_ms.unwrap_or(defaults.timeout_ms)
    }

    pub fn fuel_cap(&self, defaults: &RegistryDefaults) -> u64 {
        self.fuel_cap.unwrap_or(defaults.fuel_cap)
    }

    pub fn on_error(&self, defaults: &RegistryDefaults) -> OnError {
        self.on_error.unwrap_or(defaults.on_error)
    }

    /// Convert the registry-side `config` (TOML) into the JSON shape
    /// that lands on `HookPayload.plugin_config`. JSON-incompatible
    /// TOML scalars (datetime, NaN/inf floats) flatten to strings or
    /// null respectively; in practice the registry only carries
    /// strings, ints, bools, arrays, and tables.
    pub fn config_as_json(&self) -> serde_json::Value {
        toml_to_json(&self.config)
    }
}

fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(d) => serde_json::Value::String(d.to_string()),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(tab) => serde_json::Value::Object(
            tab.iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect(),
        ),
    }
}

/// The whole parsed registry.
///
/// See [`RegistryEntry`] for why `Eq` is not derived.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Registry {
    pub schema_version: u32,
    #[serde(default)]
    pub defaults: RegistryDefaults,
    #[serde(default, rename = "hooks")]
    pub hooks: Vec<RegistryEntry>,
}

impl Registry {
    /// Load + validate from a filesystem path.
    ///
    /// Relative `plugin` paths in entries are resolved against the
    /// registry file's parent directory — i.e. plugins live under the
    /// same `${CLAUDE_PLUGIN_ROOT}` as the registry itself. Absolute
    /// `plugin` paths pass through unchanged for tests / packaging
    /// flows that produce them deliberately.
    pub fn load(path: &Path) -> Result<Self, RegistryError> {
        if !path.exists() {
            return Err(RegistryError::NotFound(path.to_path_buf()));
        }
        let text = std::fs::read_to_string(path)?;
        let mut parsed: Self = toml::from_str(&text)?;
        parsed.validate()?;
        if let Some(base) = path.parent() {
            parsed.resolve_plugin_paths(base);
        }
        Ok(parsed)
    }

    /// Resolve every entry's relative `plugin` path against `base`.
    /// Idempotent — absolute paths pass through unchanged.
    pub fn resolve_plugin_paths(&mut self, base: &Path) {
        for entry in &mut self.hooks {
            if entry.plugin.is_relative() {
                entry.plugin = base.join(&entry.plugin);
            }
        }
    }

    /// Parse + validate from a TOML string buffer. Useful for tests.
    ///
    /// Named `parse_str` rather than `from_str` to avoid shadowing the
    /// `std::str::FromStr` convention, which operators (correctly)
    /// expect to be infallible-returning `Result<Self, Self::Err>`
    /// with a single associated error type.
    pub fn parse_str(s: &str) -> Result<Self, RegistryError> {
        let parsed: Self = toml::from_str(s)?;
        parsed.validate()?;
        Ok(parsed)
    }

    fn validate(&self) -> Result<(), RegistryError> {
        if self.schema_version != REGISTRY_SCHEMA_VERSION {
            return Err(RegistryError::SchemaVersion {
                got: self.schema_version,
                expected: REGISTRY_SCHEMA_VERSION,
            });
        }
        for entry in &self.hooks {
            if let Some(pattern) = &entry.tool {
                regex::Regex::new(pattern).map_err(|e| RegistryError::ToolRegex {
                    name: entry.name.clone(),
                    pattern: pattern.clone(),
                    source: e,
                })?;
            }
        }
        // S-15.01 T-3f: check BC-7.06.001 Invariant 1 — on_error=block implies async=false.
        // Any entry with on_error=block AND async=true is E-REG-002 (fail-closed).
        self.validate_async_block_invariant()?;
        // F-P2-011: BC-7.06.001 Invariant 7 — (name, event, tool) tuple must be unique.
        // Two entries MAY share name+event if they bind different tool regex values.
        self.validate_name_event_tool_uniqueness()?;
        Ok(())
    }

    /// Lint invariant (S-15.01 T-3f, BC-7.06.001 Invariant 1, BC-1.14.001 Invariant 4):
    /// No entry may combine `on_error = "block"` with `async = true`.
    ///
    /// This is a hard load-time error (E-REG-002). The dispatcher refuses to start
    /// if any entry violates this invariant. Emits `dispatcher.registry_invalid`.
    ///
    /// ASYNC_DRAIN_WINDOW_MS is defined in DI-019 — cite by reference only.
    fn validate_async_block_invariant(&self) -> Result<(), RegistryError> {
        for entry in &self.hooks {
            // Both `Block` and `BlockIfMarker` can conditionally gate a dispatch; neither
            // is compatible with async-advisory semantics (E-REG-002, ADR-048 §Decision 1).
            // Matching directly to `Option<&str>` (rather than a separate bool guard)
            // threads the ACTUAL offending wire-format value through to the error
            // constructor, so `RegistryError::AsyncBlockConflict`'s Display never
            // misreports which variant triggered the invariant (LOW-1).
            let blocking_on_error: Option<&str> = match entry.on_error {
                Some(OnError::Block) => Some("block"),
                Some(OnError::BlockIfMarker) => Some("block_if_marker"),
                Some(OnError::Continue) | None => None,
            };
            if let Some(on_error) = blocking_on_error
                && entry.async_flag
            {
                return Err(RegistryError::AsyncBlockConflict {
                    name: entry.name.clone(),
                    on_error: on_error.to_string(),
                });
            }
        }
        Ok(())
    }

    /// Uniqueness invariant (F-P2-011, BC-7.06.001 Invariant 7):
    /// The (name, event, tool) tuple must be unique across all [[hooks]] entries.
    ///
    /// Two entries MAY share `name` and `event` if they bind to different `tool`
    /// regex values — this permits a single named plugin to enforce against multiple
    /// tool surfaces (e.g. `protect-secrets` on `Bash` and `Read` PreToolUse events).
    /// Two entries with `tool = None` and the same `name`+`event` are duplicates.
    ///
    /// Hard load-time error; dispatcher refuses to start on violation.
    fn validate_name_event_tool_uniqueness(&self) -> Result<(), RegistryError> {
        let mut seen: HashSet<(String, String, Option<String>)> = HashSet::new();
        for entry in &self.hooks {
            let key = (entry.name.clone(), entry.event.clone(), entry.tool.clone());
            if !seen.insert(key) {
                return Err(RegistryError::DuplicateEntry {
                    name: entry.name.clone(),
                    event: entry.event.clone(),
                    tool: entry.tool.clone(),
                });
            }
        }
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// S-18.00: event-string → EventType parsing (BC-1.15.001 INV1)
//
// `parse_event_type` is the registry-side bridge from the TOML `event` string
// field to the typed `EventType` enum in `invoke.rs`. `"PreCompact"` and
// `"PostCompact"` MUST parse without error (BC-1.15.001 Architecture Anchors).
//
// Implemented: delegates to `EventType::from_event_str` — single source of
// truth for event-string → enum mapping. Unknown event strings return
// `EventType::Other` (fail-open for novel harness event types).
// ---------------------------------------------------------------------------

/// Parse the `event` string from a hooks-registry.toml `[[hooks]]` entry into
/// the typed [`crate::invoke::EventType`] enum.
///
/// `"PreCompact"` and `"PostCompact"` are valid event strings and MUST NOT produce
/// an error (BC-1.15.001 Architecture Anchors). Unknown event strings return
/// `EventType::Other` rather than an error so future harness event types do not
/// cause registry-load failures (fail-open for novel event types).
///
/// # BC-1.15.001 INV1
///
/// Delegates to [`crate::invoke::EventType::from_event_str`] — single source of
/// truth for event-string → enum mapping. The registry side bridges the typed
/// enum into the TOML `event` string field without introducing a second dispatch table.
pub fn parse_event_type(event: &str) -> crate::invoke::EventType {
    crate::invoke::EventType::from_event_str(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_toml() -> &'static str {
        r#"
schema_version = 2

[[hooks]]
name = "commit"
event = "PostToolUse"
tool = "Bash"
plugin = "plugins/commit.wasm"
"#
    }

    #[test]
    fn parses_minimal_registry() {
        let reg = Registry::parse_str(minimal_toml()).unwrap();
        assert_eq!(reg.schema_version, 2);
        assert_eq!(reg.hooks.len(), 1);
        assert_eq!(reg.hooks[0].name, "commit");
        assert_eq!(reg.hooks[0].event, "PostToolUse");
        assert_eq!(reg.hooks[0].tool.as_deref(), Some("Bash"));
        assert!(reg.hooks[0].enabled);
    }

    #[test]
    fn config_defaults_to_empty_table_when_absent() {
        let reg = Registry::parse_str(minimal_toml()).unwrap();
        assert!(reg.hooks[0].config.is_table());
        assert_eq!(reg.hooks[0].config.as_table().unwrap().len(), 0);
        let as_json = reg.hooks[0].config_as_json();
        assert!(as_json.is_object());
        assert!(as_json.as_object().unwrap().is_empty());
    }

    #[test]
    fn config_block_parses_into_entry() {
        // Real-shape registry as the legacy-bash-adapter operator
        // would write — string + nested table.
        let toml = r#"
schema_version = 2

[[hooks]]
name = "validate-template"
event = "PostToolUse"
plugin = "hook-plugins/legacy-bash-adapter.wasm"

[hooks.config]
script_path = "legacy-hooks/validate-template.sh"
extra = { key = "value" }
"#;
        let reg = Registry::parse_str(toml).unwrap();
        let cfg = reg.hooks[0].config_as_json();
        assert_eq!(
            cfg.get("script_path").and_then(|v| v.as_str()),
            Some("legacy-hooks/validate-template.sh"),
        );
        assert_eq!(
            cfg.get("extra")
                .and_then(|v| v.get("key"))
                .and_then(|v| v.as_str()),
            Some("value"),
        );
    }

    #[test]
    fn defaults_applied_when_missing() {
        use crate::invoke::DEFAULT_FUEL_CAP;
        let reg = Registry::parse_str(minimal_toml()).unwrap();
        assert_eq!(reg.defaults.timeout_ms, 5_000);
        assert_eq!(reg.defaults.fuel_cap, DEFAULT_FUEL_CAP);
        assert_eq!(reg.defaults.priority, 500);
        assert_eq!(reg.defaults.on_error, OnError::Continue);
        assert_eq!(reg.hooks[0].priority(&reg.defaults), 500);
        assert_eq!(reg.hooks[0].timeout_ms(&reg.defaults), 5_000);
        // Close the chain: accessor resolves to DEFAULT_FUEL_CAP for entries that
        // don't override fuel_cap in their registry entry (production path via executor.rs).
        assert_eq!(reg.hooks[0].fuel_cap(&reg.defaults), DEFAULT_FUEL_CAP);
    }

    // Cross-field sync guard for the ADR-042 §Decision 1 fuel cap raise (10M → 20M).
    //
    // `RegistryDefaults::default().fuel_cap` is the global fallback applied to every
    // hook plugin that does not override `fuel_cap` in its registry entry.
    // `InvokeLimits::default().fuel_cap` is the hard limit used by `invoke_plugin`
    // when the caller supplies no explicit limits.
    //
    // Both Default impls now reference `DEFAULT_FUEL_CAP` — the single source of truth
    // in invoke.rs — making drift structurally impossible (a re-introduced literal would
    // not compile unless it also matched DEFAULT_FUEL_CAP). This test is retained as an
    // explicit cross-module guard: if either Default impl is refactored to bypass
    // DEFAULT_FUEL_CAP, this test fails with a message naming which one drifted.
    #[test]
    fn fuel_cap_defaults_stay_in_sync() {
        use crate::invoke::{DEFAULT_FUEL_CAP, InvokeLimits};

        assert_eq!(
            InvokeLimits::default().fuel_cap,
            DEFAULT_FUEL_CAP,
            "InvokeLimits::default().fuel_cap drifted from DEFAULT_FUEL_CAP; \
             update InvokeLimits::default() to reference invoke::DEFAULT_FUEL_CAP",
        );
        assert_eq!(
            RegistryDefaults::default().fuel_cap,
            DEFAULT_FUEL_CAP,
            "RegistryDefaults::default().fuel_cap drifted from DEFAULT_FUEL_CAP; \
             update RegistryDefaults::default() to reference crate::invoke::DEFAULT_FUEL_CAP",
        );
    }

    // Mutation-audit hardening SURV-05 (out-of-gate production accessor,
    // S-21.09 mutation-hardening burst): cargo-mutants replaced
    // `RegistryEntry::on_error`'s body (`self.on_error.unwrap_or(defaults.on_error)`)
    // with `Default::default()` and the mutant survived — no existing test
    // distinguished the defaults-supplied fallback value from `OnError::Continue`,
    // which happens to be BOTH the entry's expected eventual value in every prior
    // fixture AND `OnError::default()`'s value (see `OnError`'s `#[default]`
    // variant), so `Default::default()` produced an outcome indistinguishable
    // from the correct one under every prior test.
    //
    // This test parses a hook entry that OMITS `on_error` (so
    // `entry.on_error` is `None`) and pairs it with a `RegistryDefaults` whose
    // `on_error` is explicitly set to `OnError::Block` — the NON-default variant
    // (`RegistryDefaults::default().on_error` is `OnError::Continue`) — so the
    // live accessor and the `Default::default()` mutant diverge observably.
    //
    // Under live code: `self.on_error` (`None`) `.unwrap_or(defaults.on_error)`
    // (`OnError::Block`) → `OnError::Block`.
    //
    // Under the `Default::default()` body mutant: the method ignores both `self`
    // and `defaults` entirely and returns `OnError::default()` → `OnError::Continue`
    // — `assert_eq!` fails (`Continue` != `Block`) → this test goes RED.
    //
    // Mutation-proof (empirically verified, mutation-hardening burst): replacing
    // `RegistryEntry::on_error`'s body with `Default::default()` locally and
    // running `cargo test -p factory-dispatcher` turns this test RED while all
    // other registry.rs tests remain GREEN. Reverting the mutant restores GREEN.
    #[test]
    fn on_error_falls_back_to_registry_defaults_when_entry_omits_it() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "no-on-error-override"
event = "PostToolUse"
plugin = "hook-plugins/x.wasm"
"#;
        let reg = Registry::parse_str(toml).unwrap();
        assert_eq!(
            reg.hooks[0].on_error, None,
            "fixture must omit on_error so the entry-level Option is None, forcing the \
             fallback-to-defaults code path under test"
        );

        let mut defaults = RegistryDefaults::default();
        assert_eq!(
            defaults.on_error,
            OnError::Continue,
            "RegistryDefaults::default().on_error must be OnError::Continue (its #[default] \
             variant) — this sub-assertion pins the premise that OnError::Block, used below, \
             is genuinely the NON-default variant"
        );
        defaults.on_error = OnError::Block;

        assert_eq!(
            reg.hooks[0].on_error(&defaults),
            OnError::Block,
            "RegistryEntry::on_error must return defaults.on_error (Block) when the entry's \
             own on_error field is None; under the Default::default() body mutant this would \
             incorrectly return OnError::Continue (OnError's #[default] variant) regardless \
             of what `defaults` specifies"
        );
    }

    #[test]
    fn rejects_unknown_schema_version() {
        // schema_version=3 is unknown — dispatcher expects 2 (REGISTRY_SCHEMA_VERSION).
        let toml = r#"
schema_version = 3

[[hooks]]
name = "x"
event = "PreToolUse"
plugin = "x.wasm"
"#;
        let err = Registry::parse_str(toml).unwrap_err();
        match err {
            RegistryError::SchemaVersion { got, expected } => {
                assert_eq!(got, 3);
                assert_eq!(expected, 2);
            }
            other => panic!("expected SchemaVersion, got {other:?}"),
        }
    }

    #[test]
    fn rejects_invalid_tool_regex() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "bad"
event = "PreToolUse"
tool = "[unclosed"
plugin = "x.wasm"
"#;
        let err = Registry::parse_str(toml).unwrap_err();
        match err {
            RegistryError::ToolRegex { name, .. } => assert_eq!(name, "bad"),
            other => panic!("expected ToolRegex, got {other:?}"),
        }
    }

    #[test]
    fn rejects_unknown_entry_field() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "typo"
event = "PreToolUse"
plugin = "x.wasm"
priorty = 100
"#;
        assert!(Registry::parse_str(toml).is_err());
    }

    #[test]
    fn rejects_unknown_on_error_value() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "x"
event = "PreToolUse"
plugin = "x.wasm"
on_error = "shout"
"#;
        assert!(Registry::parse_str(toml).is_err());
    }

    #[test]
    fn accepts_capabilities_block() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "git"
event = "PostToolUse"
plugin = "git.wasm"

[hooks.capabilities]
env_allow = ["CLAUDE_SESSION_ID"]

[hooks.capabilities.exec_subprocess]
binary_allow = ["git"]
cwd_allow = ["."]
env_allow = ["HOME"]

[hooks.capabilities.read_file]
path_allow = [".factory/STATE.md"]
"#;
        let reg = Registry::parse_str(toml).unwrap();
        let caps = reg.hooks[0].capabilities.as_ref().unwrap();
        let exec = caps.exec_subprocess.as_ref().unwrap();
        assert_eq!(exec.binary_allow, vec!["git"]);
        assert_eq!(caps.env_allow, vec!["CLAUDE_SESSION_ID"]);
    }

    #[test]
    fn overrides_priority_per_entry() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "fast"
event = "PreToolUse"
plugin = "f.wasm"
priority = 10

[[hooks]]
name = "slow"
event = "PreToolUse"
plugin = "s.wasm"
priority = 900
"#;
        let reg = Registry::parse_str(toml).unwrap();
        assert_eq!(reg.hooks[0].priority(&reg.defaults), 10);
        assert_eq!(reg.hooks[1].priority(&reg.defaults), 900);
    }

    #[test]
    fn load_returns_not_found_for_missing_path() {
        let err = Registry::load(Path::new("/nonexistent/registry.toml")).unwrap_err();
        assert!(matches!(err, RegistryError::NotFound(_)));
    }

    #[test]
    fn load_resolves_relative_plugin_paths_against_registry_dir() {
        // Operators write `plugin = "x.wasm"` in hooks-registry.toml
        // expecting it to resolve under ${CLAUDE_PLUGIN_ROOT}, not cwd.
        // Regression for a smoke-test bug where the dispatcher reported
        // "plugin file not found" for a perfectly valid relative path.
        // The "absolute" fixture uses tempdir() to get a platform-native
        // absolute path (Path::is_absolute is platform-defined: POSIX
        // /paths/ are not absolute on Windows).
        let dir = tempfile::tempdir().unwrap();
        let reg_path = dir.path().join("hooks-registry.toml");
        let abs_dir = tempfile::tempdir().unwrap();
        let abs_plugin = abs_dir.path().join("explicit.wasm");
        assert!(abs_plugin.is_absolute());
        let abs_str = abs_plugin.to_str().unwrap().replace('\\', "/");
        std::fs::write(
            &reg_path,
            format!(
                r#"
schema_version = 2

[[hooks]]
name = "rel"
event = "PreToolUse"
plugin = "rel-plugin.wasm"

[[hooks]]
name = "abs"
event = "PreToolUse"
plugin = "{abs_str}"
"#
            ),
        )
        .unwrap();
        let reg = Registry::load(&reg_path).unwrap();
        assert_eq!(reg.hooks[0].plugin, dir.path().join("rel-plugin.wasm"));
        assert_eq!(reg.hooks[1].plugin, PathBuf::from(&abs_str));
    }

    #[test]
    fn resolve_plugin_paths_is_idempotent_for_absolute_paths() {
        let dir = tempfile::tempdir().unwrap();
        let abs_dir = tempfile::tempdir().unwrap();
        let abs_plugin = abs_dir.path().join("x.wasm");
        assert!(abs_plugin.is_absolute());
        let abs_str = abs_plugin.to_str().unwrap().replace('\\', "/");
        let mut reg = Registry::parse_str(&format!(
            r#"
schema_version = 2

[[hooks]]
name = "x"
event = "PreToolUse"
plugin = "{abs_str}"
"#
        ))
        .unwrap();
        let expected = reg.hooks[0].plugin.clone();
        reg.resolve_plugin_paths(dir.path());
        // Absolute path stays absolute.
        assert_eq!(reg.hooks[0].plugin, expected);
        reg.resolve_plugin_paths(dir.path());
        assert_eq!(reg.hooks[0].plugin, expected);
    }
}

// ---------------------------------------------------------------------------
// S-15.01 F4 test additions — VP-078 Harnesses 1 + 4 (registry-side tests)
//
// Harness 1 — lint_invariant: schema_version=2 required; v1 rejected; block+async rejected.
// Harness 4 — serde_default: absent `async` field → false; string `async` → parse error.
//
// These tests exercise Registry::parse_str() directly (no I/O).
// validate_async_block_invariant() is fully implemented (S-15.01 T-3f delivered).
// All tests in this module are GREEN.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod s15_01_vp078_harness_1_lint_invariant {
    use super::*;

    // -----------------------------------------------------------------------
    // Harness 1a — schema_version = 2 required
    // AC-001: BC-7.06.001 postcondition 1
    // -----------------------------------------------------------------------

    /// VP-078 H1: v1 registry rejected with E-REG-001 (SchemaVersion error).
    ///
    /// validate() returns Err(SchemaVersion{got:1,expected:2}).
    /// Enforced by REGISTRY_SCHEMA_VERSION = 2 constant.
    #[test]
    fn test_BC_7_06_001_schema_v1_rejected_with_e_reg_001() {
        let toml = r#"
schema_version = 1

[[hooks]]
name = "some-validator"
event = "PreToolUse"
plugin = "hook-plugins/some-validator.wasm"
"#;
        let err = Registry::parse_str(toml).unwrap_err();
        match err {
            RegistryError::SchemaVersion { got, expected } => {
                assert_eq!(got, 1, "got must be the found version (1)");
                assert_eq!(expected, 2, "expected must be REGISTRY_SCHEMA_VERSION (2)");
            }
            other => panic!(
                "test_BC_7_06_001_schema_v1_rejected: expected SchemaVersion error, got {:?}",
                other
            ),
        }
    }

    /// VP-078 H1: missing schema_version field rejected (E-REG-001 boundary).
    ///
    /// TOML without schema_version key — parse fails or schema_version defaults to 0,
    /// which != 2; either way the registry is rejected.
    #[test]
    fn test_BC_7_06_001_missing_schema_version_rejected() {
        let toml = r#"
[[hooks]]
name = "some-validator"
event = "PreToolUse"
plugin = "hook-plugins/some-validator.wasm"
"#;
        // Missing schema_version is either a TOML parse error (deny_unknown_fields) or
        // defaults to 0 then fails version check. Both produce Err.
        let result = Registry::parse_str(toml);
        assert!(
            result.is_err(),
            "test_BC_7_06_001_missing_schema_version_rejected: registry without schema_version must be rejected"
        );
    }

    /// VP-078 H1: schema_version = 2 with valid entries passes.
    ///
    /// GREEN: validate_async_block_invariant() is implemented (S-15.01 T-3f).
    #[test]
    fn test_BC_7_06_001_schema_v2_with_valid_entries_passes() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "capture-commit-activity"
event = "PostToolUse"
plugin = "hook-plugins/capture-commit-activity.wasm"
async = true
"#;
        let result = Registry::parse_str(toml);
        assert!(
            result.is_ok(),
            "test_BC_7_06_001_schema_v2_with_valid_entries_passes: schema_version=2 with valid entry must pass: {:?}",
            result
        );
    }

    // -----------------------------------------------------------------------
    // Harness 1b — on_error=block + async=true is E-REG-002
    // AC-006: BC-7.06.001 invariant 1, BC-1.14.001 invariant 4
    // -----------------------------------------------------------------------

    /// VP-078 H1 / VP-078 Rust unit test: on_error=block AND async=true → E-REG-002.
    ///
    /// GREEN: validate_async_block_invariant() is implemented (S-15.01 T-3f).
    #[test]
    fn test_BC_7_06_001_block_plus_async_true_rejected_e_reg_002() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "bad-plugin"
plugin = "hook-plugins/legacy-bash-adapter.wasm"
on_error = "block"
async = true
event = "PostToolUse"
priority = 400

[hooks.config]
script_path = "bad.sh"
"#;
        let result = Registry::parse_str(toml);
        assert!(
            result.is_err(),
            "test_BC_7_06_001_block_plus_async_true_rejected_e_reg_002: block+async entry must be rejected (E-REG-002)"
        );
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("on_error")
                || err_str.contains("async")
                || err_str.contains("E-REG-002"),
            "error must name the violating fields or error code: {}",
            err_str
        );
    }

    /// VP-078 H1: on_error=block with async absent (defaults false) → accepted.
    ///
    /// GREEN: validate_async_block_invariant() is implemented (S-15.01 T-3f).
    #[test]
    fn test_BC_7_06_001_block_without_async_accepted() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "valid-blocking-plugin"
plugin = "hook-plugins/legacy-bash-adapter.wasm"
on_error = "block"
event = "PostToolUse"
priority = 400

[hooks.config]
script_path = "valid.sh"
"#;
        // async absent → default false → invariant satisfied → Ok.
        let result = Registry::parse_str(toml);
        assert!(
            result.is_ok(),
            "test_BC_7_06_001_block_without_async_accepted: block without async must be accepted: {:?}",
            result
        );
    }

    /// VP-078 H1: async=true with on_error=continue (not block) → accepted.
    ///
    /// GREEN: validate_async_block_invariant() is implemented (S-15.01 T-3f).
    #[test]
    fn test_BC_7_06_001_async_true_with_continue_accepted() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "telemetry-plugin"
plugin = "hook-plugins/legacy-bash-adapter.wasm"
on_error = "continue"
async = true
event = "PostToolUse"
priority = 100

[hooks.config]
script_path = "telemetry.sh"
"#;
        let result = Registry::parse_str(toml);
        assert!(
            result.is_ok(),
            "test_BC_7_06_001_async_true_with_continue_accepted: async=true with on_error=continue must be accepted: {:?}",
            result
        );
    }
}

#[cfg(test)]
mod s15_01_vp078_harness_4_serde_default {
    use super::*;

    // -----------------------------------------------------------------------
    // Harness 4 — serde-default semantics (VP-078 H4, AC-002)
    // BC-7.06.001 postconditions 2 + 3
    // VP-077 Property #2 delegates field-absence testing to VP-078 H4.
    // -----------------------------------------------------------------------

    /// VP-078 H4a: explicit async=true → async_flag = true.
    ///
    /// GREEN: validate_async_block_invariant() is implemented (S-15.01 T-3f).
    #[test]
    fn test_BC_7_06_001_async_explicit_true_parsed_as_true() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "telemetry-plugin"
plugin = "hook-plugins/legacy-bash-adapter.wasm"
on_error = "continue"
async = true
event = "PostToolUse"
priority = 100

[hooks.config]
script_path = "telemetry.sh"
"#;
        let registry = Registry::parse_str(toml).expect("valid toml must parse");
        let entry = &registry.hooks[0];
        assert!(
            entry.async_flag,
            "test_BC_7_06_001_async_explicit_true_parsed_as_true: explicit async=true must parse as true"
        );
    }

    /// VP-078 H4b: explicit async=false → async_flag = false.
    ///
    /// GREEN: validate_async_block_invariant() is implemented (S-15.01 T-3f).
    #[test]
    fn test_BC_7_06_001_async_explicit_false_parsed_as_false() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "blocking-plugin"
plugin = "hook-plugins/legacy-bash-adapter.wasm"
on_error = "block"
async = false
event = "PreToolUse"
priority = 100

[hooks.config]
script_path = "blocking.sh"
"#;
        let registry = Registry::parse_str(toml).expect("valid toml must parse");
        let entry = &registry.hooks[0];
        assert!(
            !entry.async_flag,
            "test_BC_7_06_001_async_explicit_false_parsed_as_false: explicit async=false must parse as false"
        );
    }

    /// VP-078 H4c: async field absent → async_flag = false (serde-default).
    ///
    /// GREEN: validate_async_block_invariant() is implemented (S-15.01 T-3f).
    #[test]
    fn test_BC_7_06_001_async_absent_defaults_to_false() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "blocking-plugin"
plugin = "hook-plugins/legacy-bash-adapter.wasm"
on_error = "block"
event = "PreToolUse"
priority = 100

[hooks.config]
script_path = "blocking.sh"
"#;
        let registry = Registry::parse_str(toml).expect("valid toml must parse");
        let entry = &registry.hooks[0];
        assert!(
            !entry.async_flag,
            "test_BC_7_06_001_async_absent_defaults_to_false: absent async field must default to false (serde default — AC-002, DI-019 cite-by-reference)"
        );
    }

    /// VP-078 H4d: async = "true" (string, not bool) → parse error.
    ///
    /// TOML does not allow string where bool is expected; fails at TOML parse level.
    #[test]
    fn test_BC_7_06_001_async_string_value_is_parse_error() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "bad-plugin"
plugin = "hook-plugins/legacy-bash-adapter.wasm"
on_error = "continue"
async = "true"
event = "PostToolUse"
priority = 100

[hooks.config]
script_path = "bad.sh"
"#;
        // String where bool expected → TOML type mismatch → parse error.
        let result = Registry::parse_str(toml);
        assert!(
            result.is_err(),
            "test_BC_7_06_001_async_string_value_is_parse_error: async field with string value must produce a parse error (AC-002, BC-7.06.001 PC3)"
        );
    }
}

// ---------------------------------------------------------------------------
// S-21.10 — BC-1.01.016 v1.3: failure_policy schema extension tests
//
// Tests every postcondition (PC1–PC7) and BC edge cases EC-001..EC-003,
// EC-005..EC-007 for the new `failure_policy` field introduced by S-21.10
// (ADR-039 §Decision 1+2 Phase 1 schema leg — no enforcement change).
// EC-004 (duplicate `failure_policy` key) is a TOML-parser-layer concern and
// is not covered by registry unit tests.
//
// All tests in this module are GREEN-BY-DESIGN: the behavior is entirely
// governed by serde derive macros (`#[serde(rename_all = "kebab-case")]`,
// `#[serde(default)]`, `#[default]` on `FailurePolicy::FailOpen`) and the
// struct-layout of `RegistryEntry`. No hand-written parsing logic is required.
//
// AC-006 (PC7) gate: the existing test
// `fail_closed_timeout_with_on_error_continue_is_open` in `executor.rs` covers
// the Phase 1 no-enforcement constraint. That test is NOT modified. This module
// includes an independent registry-side Phase 1 scope-boundary guard
// (`test_BC_1_01_016_phase1_failure_policy_does_not_affect_on_error_accessor`)
// that verifies `failure_policy` value has zero influence on the `on_error()`
// accessor — the enforcement path that `plugin_fail_closed` in executor.rs reads.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod s21_10_bc_1_01_016_failure_policy {
    use super::*;

    // -----------------------------------------------------------------------
    // Helper: minimal valid TOML preamble for a single [[hooks]] entry.
    // Callers append additional fields after the last line.
    // -----------------------------------------------------------------------
    fn hook_toml_with_failure_policy(failure_policy_line: &str) -> String {
        format!(
            r#"
schema_version = 2

[[hooks]]
name = "test-plugin"
event = "PreToolUse"
plugin = "hook-plugins/test.wasm"
{failure_policy_line}
"#
        )
    }

    // -----------------------------------------------------------------------
    // AC-001 / PC1 — `"fail-closed"` parses to `FailurePolicy::FailClosed`
    // (BC-1.01.016 v1.3 postcondition 1; ADR-039 §Decision 2 schema leg)
    //
    // GREEN-BY-DESIGN: `#[serde(rename_all = "kebab-case")]` on `FailurePolicy`
    // maps the Rust variant `FailClosed` to/from the TOML string `"fail-closed"`.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 PC1: stanza with `failure_policy = "fail-closed"` parses to
    /// `RegistryEntry.failure_policy == FailurePolicy::FailClosed`.
    #[test]
    fn test_BC_1_01_016_parses_failure_policy_fail_closed() {
        let toml = hook_toml_with_failure_policy(r#"failure_policy = "fail-closed""#);
        let reg = Registry::parse_str(&toml).expect(
            "test_BC_1_01_016_parses_failure_policy_fail_closed: \
             TOML with failure_policy=\"fail-closed\" must parse without error (BC-1.01.016 PC1)",
        );
        assert_eq!(
            reg.hooks[0].failure_policy,
            FailurePolicy::FailClosed,
            "test_BC_1_01_016_parses_failure_policy_fail_closed: \
             RegistryEntry.failure_policy must be FailClosed when TOML has \"fail-closed\" \
             (BC-1.01.016 PC1; ADR-039 §Decision 2 kebab-case value format)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-002 / PC2 — `"fail-open"` parses to `FailurePolicy::FailOpen`
    // (BC-1.01.016 v1.3 postcondition 2; ADR-039 §Decision 2 schema leg)
    //
    // GREEN-BY-DESIGN: same serde derive maps `FailOpen` ↔ `"fail-open"`.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 PC2: stanza with `failure_policy = "fail-open"` parses to
    /// `RegistryEntry.failure_policy == FailurePolicy::FailOpen`.
    #[test]
    fn test_BC_1_01_016_parses_failure_policy_fail_open() {
        let toml = hook_toml_with_failure_policy(r#"failure_policy = "fail-open""#);
        let reg = Registry::parse_str(&toml).expect(
            "test_BC_1_01_016_parses_failure_policy_fail_open: \
             TOML with failure_policy=\"fail-open\" must parse without error (BC-1.01.016 PC2)",
        );
        assert_eq!(
            reg.hooks[0].failure_policy,
            FailurePolicy::FailOpen,
            "test_BC_1_01_016_parses_failure_policy_fail_open: \
             RegistryEntry.failure_policy must be FailOpen when TOML has \"fail-open\" \
             (BC-1.01.016 PC2; ADR-039 §Decision 2)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-003 / PC3 — Unknown values rejected at parse time
    // (BC-1.01.016 v1.3 postcondition 3; no-silent-default invariant)
    //
    // GREEN-BY-DESIGN: serde enum deserialization rejects unknown variant
    // strings by default (no `#[serde(other)]` fallback on `FailurePolicy`).
    // -----------------------------------------------------------------------
    /// BC-1.01.016 PC3: unknown value `"unknown-value"` causes `parse_str`
    /// to return `Err` — no silent default to any variant.
    #[test]
    fn test_BC_1_01_016_rejects_unknown_failure_policy() {
        let toml = hook_toml_with_failure_policy(r#"failure_policy = "unknown-value""#);
        let result = Registry::parse_str(&toml);
        assert!(
            result.is_err(),
            "test_BC_1_01_016_rejects_unknown_failure_policy: \
             unknown failure_policy value must produce Err, not a silent default \
             (BC-1.01.016 PC3; no-silent-default invariant; ADR-039 §Decision 2)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-003 / PC3 + EC-003 — Underscore form `"fail_closed"` must be `Err`
    // (BC-1.01.016 v1.3 edge case EC-003; the kebab-vs-snake guard)
    //
    // This is the CRITICAL EC-003 test.  `#[serde(rename_all = "kebab-case")]`
    // maps `FailClosed` to `"fail-closed"` (hyphen).  The snake_case form
    // `"fail_closed"` is an unrecognized variant string — serde MUST reject it.
    // The real hazard: copying the sibling `OnError` enum's
    // `#[serde(rename_all = "snake_case")]` would map `FailClosed` →
    // `"fail_closed"`, silently accepting the underscore form and opening a
    // bypass.  `FailurePolicy` uses `kebab-case` precisely to avoid this.
    //
    // GREEN-BY-DESIGN: the enum uses `rename_all = "kebab-case"`, so the only
    // accepted forms are `"fail-closed"` and `"fail-open"`. The underscore forms
    // are unrecognized and produce `Err`.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 EC-003: `failure_policy = "fail_closed"` (underscore) must
    /// produce `Err` — the canonical value is `"fail-closed"` (hyphen).
    #[test]
    fn test_BC_1_01_016_rejects_fail_closed_underscore_ec003() {
        let toml = hook_toml_with_failure_policy(r#"failure_policy = "fail_closed""#);
        let result = Registry::parse_str(&toml);
        assert!(
            result.is_err(),
            "test_BC_1_01_016_rejects_fail_closed_underscore_ec003: \
             underscore form \"fail_closed\" must be Err — canonical form is \"fail-closed\" \
             (BC-1.01.016 EC-003; #[serde(rename_all = \"kebab-case\")] guard)"
        );
    }

    /// BC-1.01.016 EC-003 (symmetric): `failure_policy = "fail_open"` (underscore)
    /// must produce `Err` — the canonical value is `"fail-open"` (hyphen).
    #[test]
    fn test_BC_1_01_016_rejects_fail_open_underscore_ec003_symmetric() {
        let toml = hook_toml_with_failure_policy(r#"failure_policy = "fail_open""#);
        let result = Registry::parse_str(&toml);
        assert!(
            result.is_err(),
            "test_BC_1_01_016_rejects_fail_open_underscore_ec003_symmetric: \
             underscore form \"fail_open\" must be Err — canonical form is \"fail-open\" \
             (BC-1.01.016 EC-003 symmetric; #[serde(rename_all = \"kebab-case\")] guard)"
        );
    }

    // -----------------------------------------------------------------------
    // EC-001 — Wrong-case form `"FAIL-CLOSED"` must be `Err`
    // (BC-1.01.016 v1.3 edge case EC-001; serde is case-sensitive)
    //
    // GREEN-BY-DESIGN: serde enum variant matching is case-sensitive; no
    // case-folding occurs.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 EC-001: `failure_policy = "FAIL-CLOSED"` (wrong case) must
    /// produce `Err` — serde enum matching is case-sensitive.
    #[test]
    fn test_BC_1_01_016_rejects_fail_closed_wrong_case_ec001() {
        let toml = hook_toml_with_failure_policy(r#"failure_policy = "FAIL-CLOSED""#);
        let result = Registry::parse_str(&toml);
        assert!(
            result.is_err(),
            "test_BC_1_01_016_rejects_fail_closed_wrong_case_ec001: \
             wrong-case \"FAIL-CLOSED\" must produce Err — serde enum matching is \
             case-sensitive (BC-1.01.016 EC-001)"
        );
    }

    // -----------------------------------------------------------------------
    // EC-002 — Empty string `""` must be `Err`
    // (BC-1.01.016 v1.3 edge case EC-002)
    //
    // GREEN-BY-DESIGN: empty string is not a recognized variant.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 EC-002: `failure_policy = ""` (empty string) must produce
    /// `Err` at parse time.
    #[test]
    fn test_BC_1_01_016_rejects_empty_string_failure_policy_ec002() {
        let toml = hook_toml_with_failure_policy(r#"failure_policy = """#);
        let result = Registry::parse_str(&toml);
        assert!(
            result.is_err(),
            "test_BC_1_01_016_rejects_empty_string_failure_policy_ec002: \
             empty string failure_policy must produce Err (BC-1.01.016 EC-002)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-004 / PC4 — Absent `failure_policy` field defaults to `FailOpen`
    // (BC-1.01.016 v1.3 postcondition 4; ADR-039 §Decision 1 backward-compat)
    //
    // GREEN-BY-DESIGN: `#[serde(default)]` on `RegistryEntry.failure_policy`
    // combined with `#[default]` on `FailurePolicy::FailOpen` ensures absent-
    // field deserialization produces `FailurePolicy::FailOpen`.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 PC4: a [[hooks]] stanza without a `failure_policy` field
    /// parses successfully; `RegistryEntry.failure_policy` is `FailOpen`.
    #[test]
    fn test_BC_1_01_016_absent_failure_policy_defaults_to_fail_open() {
        // No failure_policy field — current production format for all 76 entries.
        let toml = r#"
schema_version = 2

[[hooks]]
name = "legacy-plugin"
event = "PostToolUse"
plugin = "hook-plugins/legacy.wasm"
"#;
        let reg = Registry::parse_str(toml).expect(
            "test_BC_1_01_016_absent_failure_policy_defaults_to_fail_open: \
             stanza without failure_policy must parse without error (BC-1.01.016 PC4)",
        );
        assert_eq!(
            reg.hooks[0].failure_policy,
            FailurePolicy::FailOpen,
            "test_BC_1_01_016_absent_failure_policy_defaults_to_fail_open: \
             absent failure_policy must default to FailOpen (BC-1.01.016 PC4; \
             ADR-039 §Decision 1 backward-compat; #[serde(default)] + #[default] on FailOpen)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-005 / PC5 — `failure_policy` and `on_error` are independent axes
    // (BC-1.01.016 v1.3 postcondition 5; ADR-039 §Decision 1 axes-separation)
    //
    // `RegistryEntry` MUST hold `on_error: Option<OnError>` and
    // `failure_policy: FailurePolicy` as independent fields simultaneously.
    // A stanza with `on_error = "continue"` AND `failure_policy = "fail-closed"`
    // must represent both without structural conflict.
    //
    // GREEN-BY-DESIGN: the struct has two distinct fields; serde parses each
    // independently; no interaction between them.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 PC5: stanza with both `on_error = "continue"` and
    /// `failure_policy = "fail-closed"` — both fields hold their values
    /// simultaneously without conflict.
    #[test]
    fn test_BC_1_01_016_registry_entry_can_hold_continue_and_fail_closed_simultaneously() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "dual-policy-plugin"
event = "PostToolUse"
plugin = "hook-plugins/dual.wasm"
on_error = "continue"
failure_policy = "fail-closed"
"#;
        let reg = Registry::parse_str(toml).expect(
            "test_BC_1_01_016_registry_entry_can_hold_continue_and_fail_closed_simultaneously: \
             stanza with on_error=continue + failure_policy=fail-closed must parse (BC-1.01.016 PC5)"
        );
        let entry = &reg.hooks[0];
        assert_eq!(
            entry.on_error,
            Some(OnError::Continue),
            "test_BC_1_01_016_registry_entry_can_hold_continue_and_fail_closed_simultaneously: \
             on_error must be Some(OnError::Continue) (BC-1.01.016 PC5 axes-independence)"
        );
        assert_eq!(
            entry.failure_policy,
            FailurePolicy::FailClosed,
            "test_BC_1_01_016_registry_entry_can_hold_continue_and_fail_closed_simultaneously: \
             failure_policy must be FailClosed (BC-1.01.016 PC5 axes-independence; \
             ADR-039 §Decision 1 axes-separation)"
        );
    }

    // EC-005 (symmetric to AC-005): `on_error = "block"` + `failure_policy = "fail-open"`
    // — both fields coexist independently (BC-1.01.016 EC-005).
    /// BC-1.01.016 EC-005: stanza with `on_error = "block"` + `failure_policy = "fail-open"`
    /// — both fields coexist; crash blocks via on_error=block; exhaustion advisory via
    /// failure_policy=fail-open (enforcement in Phase 4 only).
    #[test]
    fn test_BC_1_01_016_registry_entry_can_hold_block_and_fail_open_simultaneously() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "block-crash-open-exhaustion"
event = "PreToolUse"
plugin = "hook-plugins/hybrid.wasm"
on_error = "block"
failure_policy = "fail-open"
"#;
        let reg = Registry::parse_str(toml).expect(
            "test_BC_1_01_016_registry_entry_can_hold_block_and_fail_open_simultaneously: \
             on_error=block + failure_policy=fail-open must parse (BC-1.01.016 EC-005)",
        );
        let entry = &reg.hooks[0];
        assert_eq!(entry.on_error, Some(OnError::Block));
        assert_eq!(entry.failure_policy, FailurePolicy::FailOpen);
    }

    // -----------------------------------------------------------------------
    // AC-006 / PC7 — Phase 1 no-enforcement gate (registry-side scope guard)
    // (BC-1.01.016 v1.3 postcondition 7; ADR-039 §Decision 3 Phase 1 boundary)
    //
    // The canonical AC-006 test is the EXISTING test
    // `fail_closed_timeout_with_on_error_continue_is_open` in `executor.rs`
    // (line ~908). That test is NOT modified by S-21.10. It asserts that
    // `plugin_fail_closed(&r, OnError::Continue)` returns `false` for a
    // `Timeout { cause: TimeoutCause::Fuel }` result — the enforcement function
    // does NOT consult `failure_policy` in Phase 1.
    //
    // This registry-side guard is complementary: it verifies that a `RegistryEntry`
    // with `failure_policy = FailClosed` does NOT alter the `on_error()` accessor
    // return value — the accessor that `plugin_fail_closed` in executor.rs reads.
    // Phase 1 is safe because the new field is parsed and stored but consulted by
    // no enforcement path.
    //
    // GREEN-BY-DESIGN: `on_error()` reads only `self.on_error` and the defaults
    // argument; it has no branch on `self.failure_policy`.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 PC7 registry-side scope guard: a `RegistryEntry` with
    /// `failure_policy = FailClosed` MUST NOT alter the `on_error(defaults)` value.
    /// Verifies Phase 1 scope boundary — `failure_policy` is stored but NOT
    /// consulted by the `on_error()` accessor path used by `plugin_fail_closed`.
    #[test]
    fn test_BC_1_01_016_phase1_failure_policy_does_not_affect_on_error_accessor() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "fail-closed-plugin"
event = "PreToolUse"
plugin = "hook-plugins/fc.wasm"
on_error = "continue"
failure_policy = "fail-closed"
"#;
        let reg = Registry::parse_str(toml).expect(
            "test_BC_1_01_016_phase1_failure_policy_does_not_affect_on_error_accessor: must parse",
        );
        let entry = &reg.hooks[0];
        // Verify failure_policy is stored as FailClosed.
        assert_eq!(
            entry.failure_policy,
            FailurePolicy::FailClosed,
            "fixture sanity: failure_policy must be FailClosed"
        );
        // The on_error() accessor must return OnError::Continue (from the entry's own
        // on_error = Some(Continue)), regardless of failure_policy = FailClosed.
        // This is the Phase 1 scope boundary: failure_policy does NOT influence the
        // on_error() code path that plugin_fail_closed in executor.rs reads.
        assert_eq!(
            entry.on_error(&RegistryDefaults::default()),
            OnError::Continue,
            "test_BC_1_01_016_phase1_failure_policy_does_not_affect_on_error_accessor: \
             on_error() must return Continue regardless of failure_policy=FailClosed \
             (BC-1.01.016 PC7; ADR-039 §Decision 3 Phase 1 no-enforcement-change boundary; \
             the canonical enforcement gate is fail_closed_timeout_with_on_error_continue_is_open \
             in executor.rs which MUST pass unmodified after S-21.10)"
        );
    }

    // -----------------------------------------------------------------------
    // AC-007 / PC6 — All production registry entries parse with `FailOpen` default
    // (BC-1.01.016 v1.3 postcondition 6; ADR-039 §Decision 1 backward-compat)
    //
    // Drives the actual `plugins/vsdd-factory/hooks-registry.toml` through the
    // updated registry loader. All 76 entries (none carry `failure_policy`) must
    // parse cleanly; all must resolve to `FailurePolicy::FailOpen`.
    //
    // GREEN-BY-DESIGN: `#[serde(default)]` + `#[default] FailOpen` ensures absent-
    // field backward-compat holds for every existing entry.
    // -----------------------------------------------------------------------
    /// BC-1.01.016 PC6 (S-25.01 updated invariant):
    ///
    /// The full production `plugins/vsdd-factory/hooks-registry.toml` parses successfully.
    ///
    /// Pre-S-25.01: ALL entries defaulted to fail-open (absent field → FailOpen).
    ///
    /// Post-S-25.01 (BC-1.18.004 PC4 + AC-016 + AC-017 — DO NOT DELETE):
    /// EXACTLY THREE Cohort A validators (ADR-047 §Decision 8a, human-confirmed gate) are
    /// assigned `failure_policy = "fail-closed"`:
    ///   - `validate-factory-path-staging`    (Cohort A-immediate; ASSIGNED-NOW — corrected
    ///     from the overclaiming EFFECTIVE-NOW label per BC-1.18.004 v1.2; its PreToolUse-only
    ///     registration structurally cannot reach the PostToolUse-only marker write path, so
    ///     the assignment currently produces ZERO observable enforcement effect)
    ///   - `validate-pr-merge-prerequisites`  (Cohort A-deferred; SET-BUT-LATENT)
    ///   - `validate-wave-gate-prerequisite`  (Cohort A-deferred; SET-BUT-LATENT)
    ///
    /// Post-S-25.04 (BC-4.16.002; BC-1.18.004 v1.4 PC4 companion statement — human-ratified
    /// BROAD trigger scope 2026-09-04): a fourth validator is sanctioned fail-closed, but it
    /// is NOT a fourth Cohort A member — BC-1.18.004 v1.4 PC4 is explicit that "the new
    /// validator is not a fourth Cohort A member; Cohort A remains closed at exactly three per
    /// ADR-047 §Decision 8a's human-confirmed gate." Its fail-closed sanction derives from a
    /// separate authority: ADR-039 §Decision 2 (the seventh validator-class plugin named under
    /// that Decision's fail-closed roadmap), not ADR-047 §Decision 8a Cohort A membership:
    ///   - `validate-factory-path-staged`     (PostToolUse detective mirror of
    ///     `validate-factory-path-staging`; native-WASM, fuel-axis-only calibration
    ///     (ADR-039 §Decision 2 seventh member); sanctioned fail-closed independently of
    ///     Cohort A; this is the first Layer-1 validator whose fail-closed assignment reaches
    ///     `write_indeterminate_marker` via a structurally-reachable PostToolUse
    ///     trigger — S-25.04 AC-001)
    ///
    /// So the full sanctioned fail-closed allowlist checked by this test = Cohort A (3,
    /// ADR-047 §D8a) PLUS `validate-factory-path-staged` (1, ADR-039 §D2 / BC-4.16.002
    /// seventh member) = 4 entries total, from two distinct authorities.
    ///
    /// ALL other entries (including the two gate plugin entries for
    /// `validate-unvalidated-mutation-marker` / `validate-unvalidated-mutation-marker-git`
    /// which MUST be fail-open per BC-1.18.002 invariant 2) MUST remain FailOpen.
    ///
    /// This test is the canonical backward-compat guard for the ~76 fail-open plugins
    /// and MUST NOT be deleted (BC-1.18.004 PC5; ADR-047 §Decision 7).
    ///
    /// If a new entry ever shows up with `failure_policy = "fail-closed"` outside the
    /// explicitly-sanctioned allowlist below, this test will fail — that is the intended
    /// sentinel behaviour. Only a human-ratified ADR amendment (ADR-047 §D8a for Cohort A,
    /// or ADR-039 §D2 for the roadmap list) may expand this set.
    #[test]
    fn test_BC_1_01_016_production_registry_all_entries_default_to_fail_open() {
        // CARGO_MANIFEST_DIR is crates/factory-dispatcher; registry is at
        // ../../plugins/vsdd-factory/hooks-registry.toml from the crate root.
        let manifest_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
        let registry_path = manifest_dir
            .join("../..")
            .join("plugins/vsdd-factory/hooks-registry.toml");
        let registry_path = registry_path
            .canonicalize()
            .expect("hooks-registry.toml must exist at plugins/vsdd-factory/hooks-registry.toml");

        let reg = Registry::load(&registry_path).expect(
            "test_BC_1_01_016_production_registry_all_entries_default_to_fail_open: \
             hooks-registry.toml must parse without error (BC-1.01.016 PC6)",
        );
        assert!(
            !reg.hooks.is_empty(),
            "production registry must have at least one hook entry"
        );

        // S-25.01 AC-016 / BC-1.18.004 PC4: the sanctioned fail-closed allowlist below is
        // NOT a single "Cohort A of four" — it is Cohort A (ADR-047 §D8a v1.3 human-ratified
        // membership, exactly three: `validate-factory-path-staging`,
        // `validate-pr-merge-prerequisites`, `validate-wave-gate-prerequisite`) PLUS one
        // entry from a distinct authority: `validate-factory-path-staged`, sanctioned by
        // ADR-039 §Decision 2 (seventh validator-class plugin named under that Decision's
        // fail-closed roadmap) per S-25.04 / BC-4.16.002 / BC-1.18.004 v1.4 PC4 companion
        // statement — human-ratified 2026-09-04. BC-1.18.004 v1.4 PC4 is explicit that this
        // addition does NOT expand Cohort A ("the new validator is not a fourth Cohort A
        // member; Cohort A remains closed at exactly three per ADR-047 §Decision 8a's
        // human-confirmed gate"). The set is checked as one flat allowlist here because both
        // authorities converge on the same functional gate (this test); the two-authority
        // provenance is documented, not erased, by keeping them in one HashSet.
        let sanctioned_fail_closed: std::collections::HashSet<&str> = [
            "validate-factory-path-staging",
            "validate-pr-merge-prerequisites",
            "validate-wave-gate-prerequisite",
            "validate-factory-path-staged",
        ]
        .into_iter()
        .collect();

        // Collect actual fail-closed entries for clear failure messages.
        let fail_closed_names: Vec<String> = reg
            .hooks
            .iter()
            .filter(|e| e.failure_policy == FailurePolicy::FailClosed)
            .map(|e| e.name.clone())
            .collect();

        // Sanity: every fail-closed entry must be in the sanctioned allowlist.
        for name in &fail_closed_names {
            assert!(
                sanctioned_fail_closed.contains(name.as_str()),
                "test_BC_1_01_016_production_registry_all_entries_default_to_fail_open: \
                 entry '{}' has failure_policy=FailClosed but is NOT in the sanctioned \
                 fail-closed allowlist (Cohort A per ADR-047 §D8a: validate-factory-path-staging, \
                 validate-pr-merge-prerequisites, validate-wave-gate-prerequisite; PLUS \
                 validate-factory-path-staged per ADR-039 §D2 seventh member / BC-4.16.002 — \
                 NOT a fourth Cohort A member, BC-1.18.004 v1.4 PC4). Only ADR-047/ADR-039- \
                 sanctioned entries may be fail-closed. This is a regression guard — do NOT \
                 silently add fail-closed entries without ADR amendment (BC-1.18.004 PC5; \
                 ADR-047 §D8a; ADR-039 §D2). DO NOT DELETE.",
                name
            );
        }

        // Sanity: EXACTLY the sanctioned allowlist entries must be fail-closed (no more, no
        // less). The allowlist size is dynamic across its two authorities (3 Cohort A +
        // 1 ADR-039 §D2 seventh-member = 4 as of S-25.04) — compared against its own `.len()`
        // rather than a hardcoded literal, so this assertion tracks future human-ratified
        // ADR-047/ADR-039 amendments without needing an update here. If any sanctioned entry
        // is missing its fail-closed assignment, that's also a defect.
        assert_eq!(
            fail_closed_names.len(),
            sanctioned_fail_closed.len(),
            "test_BC_1_01_016_production_registry_all_entries_default_to_fail_open: \
             expected exactly {} fail-closed entries (sanctioned allowlist: {:?}), found {} \
             ({:?}). ADR-047 §D8a requires all three Cohort A entries fail-closed; ADR-039 §D2 \
             additionally requires validate-factory-path-staged fail-closed as its seventh \
             member (BC-4.16.002; NOT a fourth Cohort A member per BC-1.18.004 v1.4 PC4). \
             DO NOT DELETE — this is the canonical fail-closed count sentinel (BC-1.18.004 PC4).",
            sanctioned_fail_closed.len(),
            sanctioned_fail_closed,
            fail_closed_names.len(),
            fail_closed_names,
        );

        // Confirm: every entry NOT in the sanctioned allowlist must be fail-open.
        for entry in &reg.hooks {
            if !sanctioned_fail_closed.contains(entry.name.as_str()) {
                assert_eq!(
                    entry.failure_policy,
                    FailurePolicy::FailOpen,
                    "test_BC_1_01_016_production_registry_all_entries_default_to_fail_open: \
                     non-sanctioned entry '{}' must have failure_policy=FailOpen \
                     (BC-1.01.016 PC6; ~76 fail-open plugins backward-compat). \
                     DO NOT DELETE.",
                    entry.name
                );
            }
        }
    }

    // -----------------------------------------------------------------------
    // Round-trip serialize-deserialize: canonical values MUST serialize as
    // hyphenated strings (ADR-039 §Decision 2 value format)
    //
    // GREEN-BY-DESIGN: `#[serde(rename_all = "kebab-case")]` guarantees
    // `FailClosed` serializes as `"fail-closed"` and `FailOpen` as `"fail-open"`.
    // -----------------------------------------------------------------------
    /// `FailurePolicy::FailClosed` must serialize as the JSON string `"fail-closed"`
    /// (hyphenated, per ADR-039 §Decision 2 value format).
    #[test]
    fn test_BC_1_01_016_fail_closed_serializes_as_hyphenated() {
        let json = serde_json::to_string(&FailurePolicy::FailClosed)
            .expect("FailurePolicy::FailClosed must serialize");
        assert_eq!(
            json, "\"fail-closed\"",
            "test_BC_1_01_016_fail_closed_serializes_as_hyphenated: \
             FailClosed must serialize as \"fail-closed\" (hyphenated); \
             got {json} — verifies ADR-039 §Decision 2 value format and \
             BC-1.01.016 EC-003 guard (snake_case \"fail_closed\" is rejected at \
             deserialization because the canonical form is hyphenated)"
        );
        // Round-trip: deserialize back to verify symmetry.
        let back: FailurePolicy = serde_json::from_str(&json)
            .expect("\"fail-closed\" must deserialize back to FailClosed");
        assert_eq!(back, FailurePolicy::FailClosed);
    }

    /// `FailurePolicy::FailOpen` must serialize as the JSON string `"fail-open"`
    /// (hyphenated, per ADR-039 §Decision 2 value format).
    #[test]
    fn test_BC_1_01_016_fail_open_serializes_as_hyphenated() {
        let json = serde_json::to_string(&FailurePolicy::FailOpen)
            .expect("FailurePolicy::FailOpen must serialize");
        assert_eq!(
            json, "\"fail-open\"",
            "test_BC_1_01_016_fail_open_serializes_as_hyphenated: \
             FailOpen must serialize as \"fail-open\" (hyphenated); got {json} \
             (ADR-039 §Decision 2 value format)"
        );
        let back: FailurePolicy =
            serde_json::from_str(&json).expect("\"fail-open\" must deserialize back to FailOpen");
        assert_eq!(back, FailurePolicy::FailOpen);
    }

    // -----------------------------------------------------------------------
    // Default variant guard: `FailurePolicy::default()` MUST be `FailOpen`
    // (BC-1.01.016 Invariant 2 absent-field-is-fail-open invariant)
    // -----------------------------------------------------------------------
    /// BC-1.01.016 Invariant 2: `FailurePolicy::default()` must be `FailOpen`.
    /// This is the invariant that `#[serde(default)]` on `RegistryEntry.failure_policy`
    /// relies on for absent-field backward-compat.
    #[test]
    fn test_BC_1_01_016_invariant_failure_policy_default_is_fail_open() {
        assert_eq!(
            FailurePolicy::default(),
            FailurePolicy::FailOpen,
            "test_BC_1_01_016_invariant_failure_policy_default_is_fail_open: \
             FailurePolicy::default() must be FailOpen (BC-1.01.016 Invariant 2; \
             #[default] on FailOpen variant; ADR-039 §Decision 1 backward-compat clause)"
        );
    }
}

// ---------------------------------------------------------------------------
// F-P2-011 — BC-7.06.001 Invariant 7: (name, event, tool) tuple uniqueness
// ---------------------------------------------------------------------------

#[cfg(test)]
mod f_p2_011_name_event_tool_uniqueness {
    use super::*;

    /// Identical (name, event, tool) tuples must be rejected with DuplicateEntry.
    #[test]
    fn test_validate_rejects_duplicate_name_event_tool_tuple() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "protect-secrets"
event = "PreToolUse"
tool = "Bash"
plugin = "hook-plugins/protect-secrets.wasm"
on_error = "block"

[[hooks]]
name = "protect-secrets"
event = "PreToolUse"
tool = "Bash"
plugin = "hook-plugins/protect-secrets.wasm"
on_error = "block"
"#;
        let err = Registry::parse_str(toml).unwrap_err();
        match err {
            RegistryError::DuplicateEntry { name, event, tool } => {
                assert_eq!(name, "protect-secrets");
                assert_eq!(event, "PreToolUse");
                assert_eq!(tool.as_deref(), Some("Bash"));
            }
            other => panic!(
                "test_validate_rejects_duplicate_name_event_tool_tuple: expected DuplicateEntry, got {:?}",
                other
            ),
        }
    }

    /// Same name+event but different tool values must be accepted (BC-7.06.001 Invariant 7).
    /// This is the protect-secrets pattern: two tool surfaces for one named plugin.
    #[test]
    fn test_validate_accepts_same_name_event_different_tool() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "protect-secrets"
event = "PreToolUse"
tool = "Bash"
plugin = "hook-plugins/protect-secrets.wasm"
on_error = "block"

[[hooks]]
name = "protect-secrets"
event = "PreToolUse"
tool = "Read"
plugin = "hook-plugins/protect-secrets.wasm"
on_error = "block"
"#;
        let result = Registry::parse_str(toml);
        assert!(
            result.is_ok(),
            "test_validate_accepts_same_name_event_different_tool: same name+event with different tool must be accepted: {:?}",
            result
        );
    }

    /// F-P6-002 / BC-7.06.001 v1.7 Invariant 7 (F-P3-003 amendment):
    /// String equality, not regex equivalence — `tool='^Bash$'` and `tool='Bash'` are
    /// DISTINCT entries because the uniqueness key is the raw string value, not the set of
    /// tool surfaces the pattern matches. Two entries that happen to match the same tool
    /// surface via different regex strings are NOT duplicates.
    #[test]
    fn test_validate_treats_regex_variants_as_distinct_per_v1_5_amendment() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "regex-test"
event = "PreToolUse"
tool = "^Bash$"
on_error = "continue"
plugin = "hook-plugins/regex-test.wasm"

[[hooks]]
name = "regex-test"
event = "PreToolUse"
tool = "Bash"
on_error = "continue"
plugin = "hook-plugins/regex-test.wasm"
"#;

        let result = Registry::parse_str(toml);
        assert!(
            result.is_ok(),
            "BC-7.06.001 v1.7 Invariant 7: tool='^Bash$' and tool='Bash' MUST be DISTINCT entries \
(raw-string equality, not regex equivalence). Got: {:?}",
            result.err()
        );
    }

    /// Two entries with tool = None (absent) and matching name+event must be rejected.
    #[test]
    fn test_validate_treats_two_none_tools_as_duplicate() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "x"
event = "PreToolUse"
plugin = "hook-plugins/x.wasm"

[[hooks]]
name = "x"
event = "PreToolUse"
plugin = "hook-plugins/x.wasm"
"#;
        let err = Registry::parse_str(toml).unwrap_err();
        match err {
            RegistryError::DuplicateEntry { name, event, tool } => {
                assert_eq!(name, "x");
                assert_eq!(event, "PreToolUse");
                assert!(tool.is_none(), "tool must be None for absent tool field");
            }
            other => panic!(
                "test_validate_treats_two_none_tools_as_duplicate: expected DuplicateEntry, got {:?}",
                other
            ),
        }
    }
}

// ── ADR-048 §Decision 1: E-REG-002 block_if_marker registry schema (BC-1.18.002) ─────────────

#[cfg(test)]
mod s25_01_on_error_block_if_marker {
    use super::*;

    // -----------------------------------------------------------------------
    // E-REG-002: `on_error = "block_if_marker"` registry schema invariants.
    // BC-1.18.002 v1.5 postconditions + invariants.
    // -----------------------------------------------------------------------

    /// BC-1.18.002 E-REG-002: `on_error = "block_if_marker"` MUST parse to OnError::BlockIfMarker.
    ///
    /// Verifies serde deserialization of the new "block_if_marker" string to the
    /// BlockIfMarker variant (ADR-048 §Decision 1 — third OnError value).
    #[test]
    fn test_BC_1_18_002_E_REG_002_block_if_marker_parses_to_variant() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "crash-gate"
plugin = "hook-plugins/validate-unvalidated-mutation-marker.wasm"
on_error = "block_if_marker"
event = "PreToolUse"

[hooks.config]
"#;
        let registry =
            Registry::parse_str(toml).expect("block_if_marker entry MUST parse successfully");
        let entry = &registry.hooks[0];
        assert_eq!(
            entry.on_error,
            Some(OnError::BlockIfMarker),
            "BC-1.18.002 E-REG-002: 'block_if_marker' MUST parse to OnError::BlockIfMarker"
        );
    }

    /// BC-1.18.002 E-REG-002: `on_error = "block_if_marker"` + `async = true` MUST be rejected.
    ///
    /// Same invariant as on_error=block + async=true: both can conditionally gate a dispatch
    /// and MUST NOT be combined with async=true (BC-7.06.001 Invariant 1 extended for BlockIfMarker).
    #[test]
    fn test_BC_1_18_002_E_REG_002_block_if_marker_plus_async_rejected() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "bad-crash-gate"
plugin = "hook-plugins/validate-unvalidated-mutation-marker.wasm"
on_error = "block_if_marker"
async = true
event = "PreToolUse"

[hooks.config]
"#;
        let result = Registry::parse_str(toml);
        assert!(
            result.is_err(),
            "BC-1.18.002 E-REG-002: block_if_marker + async=true MUST be rejected \
             (same invariant as on_error=block + async=true)"
        );
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("on_error")
                || err_str.contains("async")
                || err_str.contains("E-REG-002"),
            "E-REG-002 error MUST name the violating fields or error code: {err_str}"
        );
    }

    /// LOW-1 (RED): `on_error = "block_if_marker"` + `async = true` E-REG-002 error message
    /// MUST name the ACTUAL offending `on_error` value ("block_if_marker"), not a hardcoded
    /// "block" literal.
    ///
    /// `validate_async_block_invariant`'s guard `on_error_can_block = matches!(...)` matches
    /// BOTH `Block` and `BlockIfMarker`, so this invariant fires for either value — but
    /// `RegistryError::AsyncBlockConflict`'s `#[error(...)]` Display text hardcodes the
    /// literal string `on_error = "block"` regardless of which variant actually triggered
    /// it. For a `block_if_marker` + async entry, the emitted message inaccurately claims
    /// the entry used `on_error = "block"`, which is misleading for operator diagnosis.
    ///
    /// This test MUST FAIL against the current hardcoded message: the string
    /// `"block_if_marker"` does not appear anywhere in
    /// `on_error = "block" combined with async = true is forbidden ... [E-REG-002]"`.
    #[test]
    fn test_BC_1_18_002_E_REG_002_block_if_marker_error_message_names_actual_value() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "bad-crash-gate"
plugin = "hook-plugins/validate-unvalidated-mutation-marker.wasm"
on_error = "block_if_marker"
async = true
event = "PreToolUse"

[hooks.config]
"#;
        let result = Registry::parse_str(toml);
        assert!(
            result.is_err(),
            "LOW-1: block_if_marker + async=true MUST be rejected (E-REG-002)"
        );
        let err_str = result.unwrap_err().to_string();
        assert!(
            err_str.contains("block_if_marker"),
            "LOW-1: E-REG-002 error message MUST name the actual offending on_error value \
             (\"block_if_marker\"), not a hardcoded \"block\" literal — got: {err_str}"
        );
    }

    /// BC-1.18.002 E-REG-002: `on_error = "block_if_marker"` without `async` → accepted.
    ///
    /// async absent → defaults to false → invariant satisfied → Ok parse.
    #[test]
    fn test_BC_1_18_002_E_REG_002_block_if_marker_without_async_accepted() {
        let toml = r#"
schema_version = 2

[[hooks]]
name = "crash-gate"
plugin = "hook-plugins/validate-unvalidated-mutation-marker.wasm"
on_error = "block_if_marker"
event = "PreToolUse"

[hooks.config]
"#;
        let result = Registry::parse_str(toml);
        assert!(
            result.is_ok(),
            "BC-1.18.002 E-REG-002: block_if_marker without async (default false) \
             MUST be accepted: {:?}",
            result
        );
    }
}
