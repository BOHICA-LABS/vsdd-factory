## PR Review: #769 fix(tests): mktemp portability + dispatcher provenance in bats suites

**Verdict: REQUEST_CHANGES**
**Reviewed head:** `66175e130497d2047452faef87b61da5534d73f6` (matches live `headRefOid` at review time)
**Posting note:** GitHub refused `gh pr review --request-changes` (the reviewing account is the PR author). The review was posted with `gh pr review --comment` instead: state COMMENTED on commit 66175e13, submitted 2026-10-07T15:25:42Z. The verdict above is still REQUEST_CHANGES.

### What I verified

- **No `mktemp ...XXXXXX.<ext>` templates remain.** I ran `git grep -nE 'mktemp.*XXXXXX\.[A-Za-z]'` over the full tree at the PR head: 0 hits. The pr-manager.md step 5c change is now `mktemp "${TMPDIR:-/tmp}/pr-comment-XXXXXX"`, which works with both BSD and GNU mktemp.
- **Cleanup is correct.**
  - TEST_TMP (f2 / host-abi / read-prefix) and SCRATCH_DIR (pr-manager-hardening) are created in setup and removed in teardown.
  - The resolver suites write sinks into FACTORY_TMP, which their guarded teardown deletes. This also fixes a leak: the old `${RANDOM}` sinks were never deleted.
  - verify-factory-lock-read-prefix uses WORK, which its teardown deletes.
- **No behavior change from scratch files that no longer exist up front.** I checked every consumer: the `-f`/`-s` guards behave the same either way, and stub-gh appends with `>>`.
- **No duplicate setup_file/setup/teardown definitions.**
- **I ran the suites locally on macOS (bats 1.13.0) at the PR head.** All 7 touched suites pass: 8/8, 9/9, 8/8, 33 ok (1 skip), 8 ok (1 skip), 2/2, 5/5.

### Findings

| # | Severity | Category | Finding |
|---|----------|----------|---------|
| B-1 | BLOCKING | coherence | In verify-factory-lock-read-prefix.bats, `setup_file` calls `emit_dispatcher_provenance` with no argument, so the helper picks debug before release. `setup()` hard-codes `DISPATCHER=target/release/...`. When both builds exist (the cargo-host CI job, most dev machines), the TAP provenance records the debug binary while the release binary is the one tested. I reproduced this locally. Fix: `emit_dispatcher_provenance "<repo>/target/release/factory-dispatcher"`. |
| B-2 | BLOCKING | coverage/description | Head 66175e13 has 0 check-runs, 0 statuses and 0 workflow runs; mergeable is UNKNOWN. The only CI runs are for 68086045 (2026-08-05). The description's "Full CI re-run on the updated head" is not true for this head. Fix: trigger CI and get green results, including the macOS bats leg and bats-full-suite. |
| S-1 | SUGGESTION | coherence | The other 3 newly wired suites only record the right binary because their resolution logic matches the helper's. Resolve DISPATCHER once and pass it explicitly to `emit_dispatcher_provenance`. |
| N-1 | NIT | description | Against develop, the scratch files were in BATS_TMPDIR, not MOCK_BIN. The diff wires 4 suites; the 5th (validate-cross-site-correspondence) was already on develop. |
| N-2 | NIT | style | The bare `rm -rf "${TEST_TMP}"` teardown doesn't match the resolver suites' guarded form, and some per-test `rm -f` lines are now redundant. |

covered_sha: 66175e130497d2047452faef87b61da5534d73f6
