## PR Review (second fresh-eyes reviewer): #844 wasmtime/wasmtime-wasi 46.0.3 -> 48.0.5 (RUSTSEC-2026-0316 et al.)

**Verdict: APPROVE.** No blocking findings. Covered SHA: `2ef47950752cb9be969204f60891ccc8668d630f`

**How it was posted:** `gh pr review 844 --approve --body-file` was tried first. GitHub refused it ("Can not approve your own pull request") because the gh account is the PR author. The same body was then posted with `--comment`, as a COMMENTED review at 2026-10-07T07:04:25Z. A formal approval needs a different account. The first reviewer's file, `pr-review.md` in this directory, is left untouched.

### What I checked

- **Lockfile.** `Cargo.lock` has exactly one entry each for `wasmtime` and `wasmtime-wasi`, both at 48.0.5. Every `wasmtime-internal-*` crate is also at 48.0.5. No 46.x versions are left anywhere in the lock. `chacha20` is at 0.10.2. `deny.toml` is unchanged. The `deny-advisories` CI check passes.
- **Sandbox equivalence (`invoke.rs`).** In the wasmtime-wasi 48.0.5 source, `preopened_dir(host, guest, FsPerms)` maps `FsPerms::ReadWrite` to `OpenMode::READ | OpenMode::WRITE`. That matches the old `DirPerms::all() | FilePerms::all()` grant. This is the only `preopened_dir` call, and no `DirPerms`/`FilePerms` uses remain in Rust source.
- **Engine hardening (`engine.rs`).** `config.wasm_component_model(false)` is set. The test `engine_rejects_component_binaries` confirms it by compiling `(component)` against the real engine and expecting a rejection.
- **Floor guard (`s21_12_version_gate.rs`).** `check_wasmtime_lockstep_floor` requires exactly one lock entry per package, a version from 48.0.4 up to but not including 49.0.0, and the two versions to match. The fixture tests cover:
  - versions below the floor
  - the 49.0.0 ceiling
  - mismatched versions in both directions
  - duplicate entries in both orders, for both packages
  - identical duplicates
  - valid versions
- **Bats (`s21-12-version-and-deny-gate.bats`).** The AC-001/AC-002 regexes now require `"48.`. In AC-003, the range test `major == 48 && (minor > 0 || patch >= 4)` and the lockstep comparison are both correct.
- **Docs and commits.** The CHANGELOG and `HOST_ABI.md` changes match the code. The four commits use conventional-commit format. Excluding the lockfile, the diff is about 260 lines.

### Findings

| # | Severity | Category | Finding | Suggestion |
|---|----------|----------|---------|------------|
| 1 | NIT | coverage | Bats AC-003 reads one version per package with `head -1`, so it would not catch a duplicate lock entry. The Rust guard does catch it, so coverage is still complete. | Count the matches with jq and fail if the count is not 1. |
| 2 | NIT | description | The CHANGELOG says "→ 48.0.4 (resolves 48.0.5)" and the PR title says 48.0.5. Both are accurate; only the wording differs. | Optional: use the same wording in both. |
| 3 | NIT | coverage | `engine_rejects_component_binaries` matches the error by substring, so a wasmtime rewording of the message could break it without any behavior change. | Acceptable as is. |
| 4 | NIT (pre-existing) | description | `HOST_ABI.md` says the `FACTORY_STATE_FILE` parent directory is also preopened, but the code only preopens the project cwd. This predates the PR. | Fix in a separate docs change. |

### Merge gate
Six jobs were still pending when I reviewed: `cargo-host` (ubuntu, macos), `bats-full-suite (linux)`, and `build-dispatcher` (darwin-arm64, darwin-x64, linux-x64, windows-x64). The approval assumes they finish green. Every completed check passed.
