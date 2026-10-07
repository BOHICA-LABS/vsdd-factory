//! Typed projection of the stdin envelope Claude Code writes to the
//! dispatcher. This is the pre-dispatch shape — the `dispatcher_trace_id`
//! is assigned by the dispatcher and added before the payload reaches a
//! plugin (see `vsdd_hook_sdk::HookPayload` for the plugin-facing type).

use serde::{Deserialize, Serialize};
use thiserror::Error;

/// What Claude Code writes to the dispatcher's stdin on every hook
/// invocation.
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
pub struct HookPayload {
    /// Claude Code event name — e.g. `"PreToolUse"`, `"SessionEnd"`.
    ///
    /// Claude Code's official hooks documentation uses
    /// `hook_event_name` for this field; `serde(alias)` makes the
    /// dispatcher accept either spelling so envelopes from the real
    /// harness parse cleanly. The canonical name remains `event_name`
    /// (matches `vsdd_hook_sdk::HookPayload`); the alias is for
    /// dispatcher-input only. Caught when v1.0.0-beta.1 dogfood
    /// surfaced "missing field `event_name`" errors on every real
    /// hook invocation.
    #[serde(alias = "hook_event_name")]
    pub event_name: String,

    /// Tool being invoked, when the event is tool-scoped. Empty string
    /// for session / lifecycle events that don't carry a tool.
    #[serde(default)]
    pub tool_name: String,

    /// Claude Code session id; stable across every hook in a session.
    pub session_id: String,

    /// Tool-specific input. Schema varies per tool, so this is kept as
    /// an opaque `serde_json::Value`.
    #[serde(default)]
    pub tool_input: serde_json::Value,

    /// Tool-specific response. `None` for `PreToolUse` and other
    /// pre-call events; present for `PostToolUse` and lifecycle events
    /// that carry a result.
    #[serde(default)]
    pub tool_response: Option<serde_json::Value>,

    /// Pass-through fields not explicitly modeled above. This captures
    /// event-specific fields (e.g. SubagentStop's `agent_type`,
    /// `subagent_name`, `last_assistant_message`, `result` per
    /// BC-2.02.012) so they are forwarded to plugins unchanged.
    /// The dispatcher only needs `event_name` and `tool_name` for
    /// routing; all other fields are plugin-owned. Using `flatten`
    /// ensures unknown fields survive the parse→serialize round-trip.
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, serde_json::Value>,
}

#[derive(Debug, Error)]
pub enum PayloadError {
    #[error("stdin read failed: {0}")]
    Io(#[from] std::io::Error),
    #[error("payload json parse failed: {0}")]
    Json(#[from] serde_json::Error),
    #[error("payload missing required field: {0}")]
    MissingField(&'static str),
}

impl HookPayload {
    /// Parse from a raw byte buffer. Used by both stdin reading and
    /// tests that want to feed a fixture without round-tripping
    /// through a real pipe.
    ///
    /// Pre-processes the JSON to canonicalize `hook_event_name` /
    /// `event_name`: when both are present (e.g. test payloads that
    /// include both for completeness), `hook_event_name` is removed
    /// so the serde alias does not see a duplicate field.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, PayloadError> {
        // Parse to Value first so we can strip the duplicate before
        // the typed deserialization sees the alias conflict.
        let mut value: serde_json::Value = serde_json::from_slice(bytes)?;
        if let Some(obj) = value.as_object_mut() {
            // Both present: keep event_name (canonical), drop alias.
            if obj.contains_key("event_name") && obj.contains_key("hook_event_name") {
                obj.remove("hook_event_name");
            }
        }
        let parsed: Self = serde_json::from_value(value)?;
        parsed.validate()?;
        Ok(parsed)
    }

    /// Read + parse from any `io::Read`. Primary use: stdin.
    pub fn from_reader<R: std::io::Read>(mut reader: R) -> Result<Self, PayloadError> {
        let mut buf = Vec::with_capacity(4096);
        reader.read_to_end(&mut buf)?;
        Self::from_bytes(&buf)
    }

    fn validate(&self) -> Result<(), PayloadError> {
        if self.event_name.is_empty() {
            return Err(PayloadError::MissingField("event_name"));
        }
        if self.session_id.is_empty() {
            return Err(PayloadError::MissingField("session_id"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pretooluse() {
        let json = br#"{
            "event_name": "PreToolUse",
            "tool_name": "Bash",
            "session_id": "sess-1",
            "tool_input": {"command": "ls"}
        }"#;
        let p = HookPayload::from_bytes(json).unwrap();
        assert_eq!(p.event_name, "PreToolUse");
        assert_eq!(p.tool_name, "Bash");
        assert!(p.tool_response.is_none());
    }

    #[test]
    fn parses_posttooluse_with_response() {
        let json = br#"{
            "event_name": "PostToolUse",
            "tool_name": "Edit",
            "session_id": "s",
            "tool_input": {"file_path": "src/lib.rs"},
            "tool_response": {"success": true}
        }"#;
        let p = HookPayload::from_bytes(json).unwrap();
        assert_eq!(p.event_name, "PostToolUse");
        assert!(p.tool_response.is_some());
    }

    #[test]
    fn accepts_session_event_without_tool_name() {
        let json = br#"{
            "event_name": "SessionStart",
            "session_id": "s"
        }"#;
        let p = HookPayload::from_bytes(json).unwrap();
        assert_eq!(p.event_name, "SessionStart");
        assert_eq!(p.tool_name, "");
    }

    #[test]
    fn rejects_missing_event_name() {
        let json = br#"{"session_id":"s"}"#;
        let err = HookPayload::from_bytes(json).unwrap_err();
        assert!(matches!(err, PayloadError::Json(_)));
    }

    #[test]
    fn rejects_empty_event_name() {
        let json = br#"{"event_name":"","session_id":"s"}"#;
        let err = HookPayload::from_bytes(json).unwrap_err();
        match err {
            PayloadError::MissingField(f) => assert_eq!(f, "event_name"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn rejects_empty_session_id() {
        let json = br#"{"event_name":"x","session_id":""}"#;
        let err = HookPayload::from_bytes(json).unwrap_err();
        match err {
            PayloadError::MissingField(f) => assert_eq!(f, "session_id"),
            other => panic!("expected MissingField, got {other:?}"),
        }
    }

    #[test]
    fn rejects_malformed_json() {
        let err = HookPayload::from_bytes(b"not json").unwrap_err();
        assert!(matches!(err, PayloadError::Json(_)));
    }

    #[test]
    fn accepts_hook_event_name_alias_from_real_harness() {
        // Claude Code's documented field name is `hook_event_name`, not
        // `event_name`. The alias on HookPayload.event_name lets the
        // dispatcher parse envelopes from the real harness without
        // manual translation. v1.0.0-beta.1 dogfood regression guard.
        let json = br#"{
            "hook_event_name": "PostToolUse",
            "tool_name": "Bash",
            "session_id": "real-harness-session",
            "tool_input": {"command": "git commit"},
            "tool_response": {"exit_code": 0}
        }"#;
        let p = HookPayload::from_bytes(json).unwrap();
        assert_eq!(p.event_name, "PostToolUse");
        assert_eq!(p.session_id, "real-harness-session");
    }
}
