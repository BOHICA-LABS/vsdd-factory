---
document_type: security-review
level: ops
version: "1.0"
status: final
producer: security-reviewer
timestamp: 2026-09-26T00:00:00
phase: 5
inputs:
  - plugins/vsdd-factory/config/artifact-path-registry.yaml
  - crates/hook-plugins/validate-artifact-path/src/lib.rs
  - crates/factory-dispatcher/src/shard_manager.rs
traces_to: "PR #843 / S-25.06"
total_findings: 0
critical: 0
high: 0
medium: 0
low: 0
files_reviewed: 1
input-hash: "77e627f"
---

# Security Review: PR #843 — S-25.06 shard-config registry entry

- **PR:** #843
- **Branch:** `feature/S-25.06-shard-config-registry`
- **Head:** `419d061d`
- **Base:** `develop`
- **CI:** green (17/17, reported by caller — not independently re-run by this review)
- **Reviewer:** vsdd-factory:security-reviewer (fresh manual CWE/OWASP review; no formal-verifier scan report present for this PR)

## Executive Summary

The PR is a single 5-line additive entry to `plugins/vsdd-factory/config/artifact-path-registry.yaml`, registering `.factory/shard-config.toml` as a permitted canonical write target at `enforcement_level: block` (the registry's strictest/only enforcement level — see Positive Findings). No source code, no consuming logic, and no other registry entries are touched. **No findings. Verdict: PASS.**

## Diff Reviewed

```diff
+  - artifact_type: shard-config
+    canonical_path_pattern: ".factory/shard-config.toml"
+    description: "[[shard]] registry consumed by shard_cap_precheck/shard_cap_gate_check (BC-1.18.005/006) — declares every shard-cap-gated artifact, its shape, and its shard_cap_bytes ceiling"
+    enforcement_level: block
```

Verified via `git diff origin/develop...origin/feature/S-25.06-shard-config-registry`: 1 file changed, 5 insertions(+), 0 deletions(-). Purely additive.

## Checklist Analysis

### 1. Does this loosen any existing enforcement?

**No.** `enforcement_level: block` is confirmed (by inspection of the full registry file, 74 pre-existing entries plus this one) to be the *only* enforcement level used anywhere in `artifact-path-registry.yaml` — the file header itself documents `block` as "this path pattern is the CANONICAL location; validate-artifact-path hook allows writes here," with `warn`/`advisory` reserved but unused. Since this is a brand-new `artifact_type` entry (no prior entry for `shard-config` existed — confirmed via full-file grep, zero pre-existing matches), it cannot downgrade or shadow any existing rule. It can only ADD a new permitted path; before this change, any Write/Edit to `.factory/shard-config.toml` was unconditionally rejected by the hook as `ARTIFACT_PATH_UNREGISTERED` (per `crates/hook-plugins/validate-artifact-path/src/lib.rs`'s `MatchResult::NoMatch` branch). This is a narrowing-then-permitting change, not a loosening one.

Ordering/shadowing check: `matches_canonical()` in `validate-artifact-path/src/lib.rs` is first-match-wins over `registry.artifacts` in file order. I confirmed by full-file scan that no earlier entry in the registry uses a placeholder pattern broad enough to already match `.factory/shard-config.toml` (e.g., no root-level `.factory/{filename}` catch-all exists — root-level files like `STATE.md`, `policies.yaml`, `release-config.yaml`, `reference-manifest.yaml` are all literal, non-placeholder patterns, same as the new entry). So the new entry is reachable and does not silently get shadowed by, nor does it shadow, any pre-existing rule.

### 2. Path-traversal / pattern-injection concern in `canonical_path_pattern`?

**No.** The pattern `.factory/shard-config.toml` is a fixed literal string with **zero** `{placeholder}` tokens. The matcher (`pattern_matches()` in `validate-artifact-path/src/lib.rs`) requires exact byte-for-byte literal consumption when there are no placeholders (`pos == path_bytes.len()` branch) — there is no placeholder-expansion surface here at all, so the placeholder-related concerns that *would* apply to other registry entries (e.g., ensuring `{cycle-id}` can't smuggle a `/` to escape its directory — which the hook's own invariant 6 already defends against, per its `contains(&b'/')` checks) are categorically inapplicable to this entry. No CWE-22 (Path Traversal) surface is introduced by this pattern.

### 3. Does newly permitting writes to `.factory/shard-config.toml` create a security surface?

**No new surface beyond what already exists and was implemented (and presumably reviewed) in prior merged PRs (#818, #824, #831, #832, #842 — S-25.02 clusters 1–5).** Specifically:

- The registry entry only affects the `validate-artifact-path` PreToolUse hook's decision to allow vs. block a *write path*. It grants no new privilege beyond "an agent may write to this one specific file" — the same class of privilege every other one of the 74 existing entries already grants for its own canonical file.
- Writes to `.factory/shard-config.toml` remain subject to every OTHER gate in the hook chain (POL-3 no-bypass, other PreToolUse/PostToolUse validators, git hooks on commit, etc.) — this entry does not disable or bypass any other control; it only removes the specific `ARTIFACT_PATH_UNREGISTERED` block for this one path.
- The consumer of this file (`crates/factory-dispatcher/src/shard_manager.rs`, `ShardRegistry::load()`) parses it with `toml::from_str` into typed structs (`ShardEntry` etc.) via `serde` — a memory-safe, non-`eval`, non-shell-invoking deserialization path. No `Command::new`/`process::Command` invocation was found anywhere in `shard_manager.rs` that consumes fields from this config. There is no code-execution or shell-injection surface (CWE-78) from the file's contents.
- This is pre-existing, already-implemented, already-merged consumer code from S-25.02 (BC-1.18.005/006/008/009/010/011), not new code introduced by this PR. This PR's scope is exclusively the registry-entry addition; it does not modify `shard_manager.rs` or any parsing/consumption logic. A security assessment of that consumer's own internal handling of `shard_cap_bytes` ceilings, artifact-path fields, etc. is out of scope for this PR's diff (5 lines, config-registry only) and was the responsibility of the security reviews on the S-25.02 cluster PRs that introduced that code.
- No information disclosure risk: `description` field is static documentation text, not user input, and is never echoed back to an untrusted party in a way that differs from any other entry's description field.

No CWE applies. No OWASP Top 10 category applies (this change touches no authentication, authorization boundary, input-validation surface, or crypto — it is a declarative allow-list addition consumed by a pre-existing, already-reviewed pure-parsing code path).

## Findings

None. `total_findings: 0`.

## Summary Table

| ID | Severity | CWE | Location | Status |
|----|----------|-----|----------|--------|
| — | — | — | — | No findings |

## Positive Findings (Defensive Measures Present)

- The artifact-path-registry defaults to deny-by-default (`ARTIFACT_PATH_UNREGISTERED` blocks any unmatched `.factory/` write) — a sound allow-list security posture (CWE-1173-adjacent "improper access control" is avoided by construction).
- `enforcement_level: block` is universally applied across all 75 entries (including this one) — no entry in the registry uses a weaker `warn`/`advisory` level, so there is no precedent this PR could have quietly matched down to.
- The new pattern is a fixed literal path with no placeholder-expansion surface, avoiding the entire class of path-traversal concerns that apply to `{placeholder}`-bearing entries elsewhere in the file.
- The consuming code (`shard_manager.rs`) parses the newly-writable file via safe, typed `toml::from_str`/`serde` deserialization — no shell invocation, no `eval`, no dynamic code execution driven by file contents.

## Recommendations Priority

### Immediate (before merge)
None.

### Before Release
None.

### Post-Release
None. No follow-up security work is attached to this change.

## Verdict

**PASS — no findings (CRITICAL/HIGH/MEDIUM/LOW: 0/0/0/0).** This PR does not merge with unresolved CRITICAL or HIGH findings because there are none. Security review does not authorize merge on its own — this file records the review disposition only; merge decision remains with pr-manager/human per standard process.
