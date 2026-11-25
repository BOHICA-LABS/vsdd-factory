# Rule case tables

One TOML table per rule, named `<rule>.toml` (the rule id, dashes as written),
loaded by `../test_rule_cases.py` alongside `../fixture_cases/probe.toml`.
Case format:

```toml
[[case]]
name = "what the case proves"
module = "vsdd.rules.tests_first"      # optional; must match the registry row's module
payload = "PreToolUse/edit-main.json"  # relative to ../payloads/, or "broken" for garbage
set = { "tool_input.file_path" = "/x" } # dotted overrides on the payload
expect = { outcome = "block", rule = "tests-first", code = "no_red_evidence" }
launcher = true                         # also run through the real launcher, sh, bash and dash
```

`expect.outcome` is `allow` or `block`; a block names the rule and code. The
coverage meta-test requires every shipped gate rule to have one blocking and
one allowing launcher case here.
