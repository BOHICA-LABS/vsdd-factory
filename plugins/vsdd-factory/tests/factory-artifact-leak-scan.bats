#!/usr/bin/env bats
# factory-artifact-leak-scan.bats — TAP tests for the content-based factory-artifact
# leak detector (crates/factory-artifact-leak-scan, Rust binary per POLICY 21),
# the systemic guard for #515.
#
# The scanner reads its factory-doctype set from the real plugin templates/ dir
# (CLAUDE_PLUGIN_ROOT) and scans tracked files under a fixture git repo
# (VSDD_REPO_ROOT). Each test builds a throwaway repo, mirroring bin.bats /
# relocate-artifact.bats conventions. Detection-logic unit tests live in the
# crate (`cargo test -p factory-artifact-leak-scan`); this suite covers the
# CLI contract end-to-end.

setup() {
  PLUGIN_ROOT="$(cd "$BATS_TEST_DIRNAME/.." && pwd)"
  REPO_ROOT_SRC="$(cd "$BATS_TEST_DIRNAME/../../.." && pwd)"
  SCANNER="$REPO_ROOT_SRC/target/release/factory-artifact-leak-scan"
  if [ ! -x "$SCANNER" ]; then
    skip "factory-artifact-leak-scan binary not built — run: cargo build --release -p factory-artifact-leak-scan"
  fi
  WORK="$(mktemp -d)"
  cd "$WORK"
  git init --quiet
  git config user.email "test@test.com"
  git config user.name "Test"
  export CLAUDE_PLUGIN_ROOT="$PLUGIN_ROOT"
  export VSDD_REPO_ROOT="$WORK"
}

teardown() {
  rm -rf "$WORK"
}

# Write a factory-frontmatter red-gate-log at $1.
_write_red_gate() {
  local path="$1"
  mkdir -p "$(dirname "$path")"
  printf '%s\n' '---' 'document_type: red-gate-log' 'story_id: S-12.08' \
    'status: RED_GATE_VERIFIED' '---' '# Red Gate Log' 'body' > "$path"
}

@test "factory-artifact-leak-scan: tracked factory artifact outside .factory is detected (#515)" {
  # The exact shape #515 removed: a red-gate-log tracked at the repo root.
  _write_red_gate red-gate-log-S-12.08.md
  git add -A
  run "$SCANNER" --list
  [ "$status" -eq 1 ]
  [[ "$output" == *"red-gate-log-S-12.08.md"* ]]
}

@test "factory-artifact-leak-scan: same artifact under .factory/ is clean (#515)" {
  # Identical content inside the artifact worktree path is legitimate — excluded
  # by path even when force-tracked (real .factory/ is gitignored).
  _write_red_gate .factory/cycles/v1.0/S-12.08/implementation/red-gate-log.md
  git add -f .factory/cycles/v1.0/S-12.08/implementation/red-gate-log.md
  run "$SCANNER" --count
  [ "$status" -eq 0 ]
  [ "$output" -eq 0 ]
}

@test "factory-artifact-leak-scan: product file without factory frontmatter is clean (#515)" {
  mkdir -p docs src
  printf '# Design Notes\n\nNo document_type frontmatter here.\n' > docs/design-notes.md
  printf 'fn main() {}\n' > src/main.rs
  git add -A
  run "$SCANNER" --count
  [ "$status" -eq 0 ]
  [ "$output" -eq 0 ]
}

@test "factory-artifact-leak-scan: product-deliverable doctype in docs/ is clean (#515)" {
  # demo-evidence-report is a template-backed factory doctype with NO .factory/
  # home in the artifact-path-registry — the project ships it in the product tree.
  # It must be allowlisted, not flagged.
  mkdir -p docs/demo-evidence/S-99.01
  printf '%s\n' '---' 'document_type: demo-evidence-report' '---' '# Evidence' \
    > docs/demo-evidence/S-99.01/evidence-report.md
  git add -A
  run "$SCANNER" --count
  [ "$status" -eq 0 ]
  [ "$output" -eq 0 ]
}

@test "factory-artifact-leak-scan: leak in an arbitrary deep product path is detected (#515)" {
  # The blind spot #515 names beyond #341: not just the root — any product path.
  _write_red_gate docs/reports/nested/decision-record.md
  git add -A
  run "$SCANNER" --list
  [ "$status" -eq 1 ]
  [[ "$output" == *"docs/reports/nested/decision-record.md"* ]]
}

@test "factory-artifact-leak-scan: untracked leak is not flagged (only tracked files scanned) (#515)" {
  # The guard's contract is about what is COMMITTED to the product branch.
  _write_red_gate stray-red-gate.md   # created but never `git add`-ed
  run "$SCANNER" --count
  [ "$status" -eq 0 ]
  [ "$output" -eq 0 ]
}

@test "factory-artifact-leak-scan: plugin template/fixture frontmatter is not flagged (#515)" {
  # plugins/vsdd-factory/templates|tests|skills|rules carry document_type: as
  # examples; they must never be treated as leaks.
  mkdir -p plugins/vsdd-factory/templates plugins/vsdd-factory/tests/fixtures
  printf '%s\n' '---' 'document_type: red-gate-log' '---' '# template' \
    > plugins/vsdd-factory/templates/red-gate-log-template.md
  printf '%s\n' '---' 'document_type: story' '---' '# fixture' \
    > plugins/vsdd-factory/tests/fixtures/example-story.md
  git add -A
  run "$SCANNER" --count
  [ "$status" -eq 0 ]
  [ "$output" -eq 0 ]
}

@test "factory-artifact-leak-scan: clean product tree reports registry-clean (#515)" {
  printf '# README\n' > README.md
  git add -A
  run "$SCANNER"
  [ "$status" -eq 0 ]
  [[ "$output" == *"Product tree is clean"* ]]
}

@test "factory-artifact-leak-scan: subdirectory-template doctype is in the leak universe (M1)" {
  # adversary-prompt-template is declared only by templates in a SUBDIRECTORY
  # (templates/adversary-prompt-templates/*.md). A -maxdepth 1 doctype build
  # missed it entirely, making this leak class invisible (review finding M1).
  printf '%s\n' '---' 'document_type: adversary-prompt-template' '---' '# Leaked' \
    > leaked-adversary-template.md
  git add -A
  run "$SCANNER" --count
  [ "$status" -eq 1 ]
  [ "$output" -eq 1 ]
}

@test "factory-artifact-leak-scan: product-deliverable doctype OUTSIDE its home is a leak (M2)" {
  # The demo-evidence exemption is path-scoped to docs/demo-evidence/. The same
  # doctype at the repo root is exactly the leak class the guard exists to
  # catch (review finding M2) — a doctype-global allowlist silently exempted it.
  printf '%s\n' '---' 'document_type: demo-evidence-report' '---' '# Stray' \
    > stray-evidence-report.md
  git add -A
  run "$SCANNER" --count
  [ "$status" -eq 1 ]
  [ "$output" -eq 1 ]
}

@test "factory-artifact-leak-scan: mid-document thematic break is not frontmatter (M3)" {
  # A '---' thematic break past line 1 must not open a phantom frontmatter
  # block: a doc that MENTIONS a factory doctype after a horizontal rule is
  # prose, not a leaked artifact (review finding M3 — the opening fence is
  # anchored to line 1).
  mkdir -p docs
  printf '%s\n' '# Factory Notes' '' '---' 'document_type: red-gate-log' '---' \
    'Discussion of the red-gate-log doctype.' > docs/factory-notes.md
  git add -A
  run "$SCANNER" --count
  [ "$status" -eq 0 ]
  [ "$output" -eq 0 ]
}
