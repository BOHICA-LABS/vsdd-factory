---
kind: packaging
id: install-check
package: R0
sources: [packaging.md#2.8, packaging.md#2.13, BLUEPRINT.md#9.1]
---

# The marketplace install check

`install-check` is a workflow in `bohica-labs/claude-mp`, run on every pull request and push there. It is the check
that would have caught the five releases (rc.2 to rc.6) that installed nothing and reported success. It has no
dependency and protects cutover step X1, so it is built first (package R0, [[package:R0]]).

```sh
export CLAUDE_CONFIG_DIR="$(mktemp -d)"                 # a clean room; the runner's own state is untouched
claude plugin validate .
claude plugin marketplace add "$PWD"
for p in $(jq -r '.plugins[].name' .claude-plugin/marketplace.json); do
  claude plugin install "$p@claude-mp"
  manifest=$(find "$CLAUDE_CONFIG_DIR/plugins/cache/claude-mp/$p" -path '*/.claude-plugin/plugin.json' | head -1)
  [ -n "$manifest" ] || { echo "install of $p populated nothing"; exit 1; }
  claude plugin validate "$(dirname "$(dirname "$manifest")")"
done
```

It also asserts that the installed manifest's `version` equals the entry's. It needs no API key if plugin commands
work without credentials, which is assumption N3 below.

What it rests on, from the assumptions of the packaging design ([[packaging:packaging-assumptions]]):

- **N3** — `claude plugin validate`, `marketplace add`, `install` and `tag` run in CI with no credentials, and honour
  `CLAUDE_CONFIG_DIR`. R0 is the test; if it fails, `install-check` runs with a key, or is run by hand before each
  marketplace merge.
- **N4** — a `git-subdir` entry whose `ref` is a tag and which also sets `sha` installs correctly. Tested by R0 on
  the X1 pull request, which pins `v1.0.0-rc.25` ([[packaging:cutover-steps]]).
- The check is the mitigation for the risk of silent marketplace install failure
  ([[topic:risks-packaging]]).
