---
document_type: holdout-scenario
level: ops
version: "1.0"
status: draft
producer: "[agent-id]"
timestamp: YYYY-MM-DDTHH:MM:SS
phase: 2
inputs: [stories/, behavioral-contracts/, prd.md]
input-hash: "[md5]"
traces_to: ""
id: "HS-NNN"
category: "integration-boundaries | edge-case-combinations | behavioral-subtleties | security-probes | ci-integration | performance-probes | real-world-corpus"
must_pass: "true | false"
priority: "must-pass | should-pass"
epic_id: "EPIC-NNN"
behavioral_contracts: []
# Tier and reuse fields (DF-038)
# scope: story | wave | final
#   story — per-story holdout gate (BC-5.39.003); 2-4 per story; single-use;
#            stored under .factory/holdout-scenarios/story-scenarios/STORY-NNN/
#   wave  — wave integration holdout pool; cycle-scoped; stored under wave-scenarios/<wave>/
#   final — end-of-pipeline HS-NNN lifecycle pool; rotated 80%; stored at root level
scope: final                    # default; set to "story" or "wave" when applicable
story_id: null                  # STORY-NNN when scope == story (required)
single_use: false               # true when scope == story (consumed after first evaluation)
# Lifecycle fields (DF-030)
lifecycle_status: active        # active | stale | retired | consumed (story-scope only)
introduced: vX.Y.Z             # cycle that created this scenario
last_evaluated: null            # last cycle where this scenario was evaluated
staleness_check: null           # last time maintenance sweep checked (YYYY-MM-DD)
stale_reason: null              # why stale (e.g., "Scenario tests feature removed in v1.2.0")
retired: null                   # cycle that retired (null if active)
consumed: null                  # ISO timestamp when consumed by state-manager (story-scope only)
# ASM/R source traceability (optional)
assumption_source: null         # ASM-NNN if this scenario tests an assumption
risk_source: null               # R-NNN if this scenario tests a risk
---

# Holdout Scenario: [Human-Readable Title]

> **One-per-file:** Each holdout scenario lives in its own file.
> Filename convention: `HS-NNN-[short-description].md`
> The product-owner agent produces individual files under
> `.factory/holdout-scenarios/` and a companion `HS-INDEX.md`.

> **WARNING:** This file is stored in `.factory/holdout-scenarios/` and must
> NEVER be shown to the implementer or test-writer agents. The information
> asymmetry between builder and evaluator is the core quality mechanism.

## Scenario

Describe the expected behavior using numbered behavioral assertions (preferred) or
BDD Given/When/Then format. Both are acceptable.

**Behavioral format (preferred):**
1. [Precondition -- system state before the action]
2. [User action -- what the user does]
3. [Expected behavior -- observable outcome]

**BDD format (acceptable):**
**Given** [precondition -- system state before the action]
**When** [user action -- what the user does]
**Then** [expected behavior -- observable outcome]

## Behavioral Contract Linkage

Each holdout scenario should trace to one or more behavioral contracts to ensure
the scenario validates specified system behavior, not ad-hoc expectations.

| BC ID | Clause Tested | Scenario Aspect |
|-------|--------------|-----------------|
| BC-S.SS.NNN | [precondition / postcondition / invariant N] | [which part of this scenario exercises it] |

## Verification Approach

[Guidance for the holdout-evaluator on HOW to test this scenario.
This may include:
- Specific CLI commands to run
- HTTP requests to make (method, path, expected status)
- Library function calls with specific inputs
- Expected output patterns (exact match, contains, regex)
- State to verify after the action (file contents, DB state, etc.)]

## Evaluation Rubric

[Defines what "satisfied" means for this scenario in behavioral terms. This rubric
is used by the LLM-as-judge evaluator to score satisfaction (0.0-1.0). The rubric
is deliberately separated from the Verification Approach to prevent agents from
gaming assertions while missing behavioral intent.

Rubric dimensions (rate each 0.0-1.0, take weighted average):
- **Functional correctness** (weight: 0.4): Does the response match expected behavior?
- **Edge case handling** (weight: 0.2): Are boundary conditions handled gracefully?
- **Error quality** (weight: 0.2): Are error responses informative and appropriate?
- **Performance** (weight: 0.1): Is the response timely (within NFR thresholds)?
- **Data integrity** (weight: 0.1): Is output data consistent and well-formed?

Override these weights per scenario as appropriate.]

## Edge Conditions

[Any boundary conditions this scenario specifically covers:
- What happens with empty input?
- What happens at scale?
- What happens with concurrent access?
- What happens after error recovery?]

## Failure Guidance

[If this scenario fails, what ONE-LINE message should the evaluator
send back to the builder? This must be descriptive enough to guide
fixing but vague enough to prevent gaming.]

Template: "HOLDOUT LOW: HS-NNN (satisfaction: 0.XX) -- [one sentence describing what didn't work]"

## Category: real-world-corpus

Real-world corpus scenarios test the product against actual, publicly
available data from production systems. They catch false positives,
encoding issues, scale problems, and edge cases that synthetic test
data misses.

### Required Fields for real-world-corpus Scenarios

| Field | Description |
|-------|-------------|
| corpus_source | URL or name of the real-world dataset |
| corpus_size | Approximate size (files, lines, records) |
| known_edge_cases | Edge cases known to exist in this corpus |
| false_positive_threshold | Max acceptable false positive rate |
| false_negative_threshold | Max acceptable false negative rate |
