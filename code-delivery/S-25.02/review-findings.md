---
document_type: pr-review-findings
story_id: S-25.02
pr_number: 832
status: "in-review"
producer: pr-manager
timestamp: "2026-09-11T21:00:00"
---

# PR Review Findings: S-25.02 Cluster 4 (PR #832)

## Convergence Summary

| Cycle | Findings | Blocking | Suggestion | Nit | Fixed | Remaining |
|-------|----------|----------|-----------|-----|-------|-----------|
| 1 | 5+ | 3 (B1,B2,SEC-001) | 4 | 0 | SEC-001,SEC-002 pushed c74c3014; B1/B2 in-flight | 2 (B1,B2) |
| 2 | 4 | 4 (C2-1,C2-2,C2-3,C2-4) | 2 | 0 | B1/B2/SEC-001/SEC-002 code fixes CORRECT per reviewer | 4 (missing tests, E-SHD-015 cite, demo README, doc stub) |

**Verdict:** REQUEST_CHANGES (cycle 2) — covered_sha 265753f58c7d846ec2030932db6469d25635779a — fixes in progress

## Finding Detail

| ID | Cycle | Severity | Category | Finding | Resolution |
|----|-------|----------|----------|---------|------------|
| SEC-001 | 1 | MEDIUM | security | CWE-22: rotate_changelog_at unconstrained archive_path | FIXED in c74c3014 (PathNotAllowed guard) |
| SEC-002 | 1 | LOW | security | CWE-252: target_path.parent() silent CWD fallback | FIXED in c74c3014 (E-SHD-015 error) |
| B1 | 1 | BLOCKING | spec-fidelity | Absolute path written into changelog_archive: pointer in BC-INDEX.md frontmatter | FIXED in 265753f5 (relative path via .factory ancestor walk) |
| B2 | 1 | BLOCKING | spec-fidelity | Hardcoded BC-INDEX-changelog-archive.md in shape-generic handler; wrong for non-BC-INDEX artifacts | FIXED in 265753f5 (entry.artifact_stem used) |
| S-1 | 1 | SUGGESTION | code-quality | B1 arm reimplements shard_sibling_path inline instead of reusing helper | FIXED in 265753f5 (shard_sibling_path used) |
| S-2 | 1 | SUGGESTION | code-quality | E-SHD-014 message omits Bash-escape-hatch guidance | Deferred to S-12.13 (E-SHD lint gate) |
| S-3 | 1 | SUGGESTION | doc-quality | build_b1_block_reason doc comment still says "stub" | NOT FIXED — cycle 2 finding BLK-C2-4; routing to implementer |
| S-4 | 1 | SUGGESTION | doc-quality | Demo README cites BC-1.18.009 v1.6 instead of v1.7 | NOT FIXED — cycle 2 finding BLK-C2-3; routing to implementer |
| BLK-C2-1 | 2 | BLOCKING | test-coverage | Zero discriminating tests for SEC-001 (ParentDir path), SEC-002 (no-parent fallback), B2 (non-BC-INDEX artifact_stem); TD-VSDD-059 paper-fix pattern | Routing to test-writer |
| BLK-C2-2 | 2 | BLOCKING | spec-fidelity | E-SHD-015 code minted in c74c3014 has no entry in error-taxonomy.md; allocation gap | Routing to implementer |
| BLK-C2-3 | 2 | BLOCKING | doc-quality | Demo README docs/demo-evidence/S-25.02/README.md lines 10+15 still cite BC-1.18.009 v1.6; cycle-1 S-4 not applied | Routing to implementer |
| BLK-C2-4 | 2 | BLOCKING | doc-quality | build_b1_block_reason doc comment still says "passes against this stub"/"test-writer MUST still write a test"; cycle-1 S-3 not applied | Routing to implementer |

## Triage Routing

| Finding ID | Routed To | Status |
|------------|-----------|--------|
| SEC-001 | implementer a46b135753f5df964 | FIXED (c74c3014) |
| SEC-002 | implementer a46b135753f5df964 | FIXED (c74c3014) |
| B1 | implementer a4af818ea3d841201 | FIXED (265753f5) |
| B2 | implementer a4af818ea3d841201 | FIXED (265753f5) |
| STATE.md banner 421→423 | state-manager a2d3268d72168d6ec | FIXED (factory-artifacts 616daf36) |
| BLK-C2-1 | test-writer af1b46663e0e6ab66 | FIXED (5633ce4b) |
| BLK-C2-2 | implementer a4afa39c3d22ac972 | FIXED (factory-artifacts 2f005d94; feature e7e99840) |
| BLK-C2-3 | implementer a4afa39c3d22ac972 | FIXED (e7e99840) |
| BLK-C2-4 | implementer a4afa39c3d22ac972 | FIXED (e7e99840) |

## Review Cycle History

### Cycle 1

- **Reviewer model:** vsdd-factory:pr-reviewer (fresh-eyes)
- **Verdict:** REQUEST_CHANGES at 44d83e99
- **Findings:** 2 BLOCKING (B1/B2) + 2 security (SEC-001 MEDIUM, SEC-002 LOW) + 4 SUGGESTIONS
- **Action taken:** SEC-001/002 fixed in c74c3014; B1/B2 routed to implementer; STATE.md banner fixed in factory-artifacts 616daf36; B1/B2 fixed in 265753f5

### Cycle 2

- **Reviewer model:** vsdd-factory:pr-reviewer (fresh-eyes)
- **Verdict:** REQUEST_CHANGES at 265753f58c7d846ec2030932db6469d25635779a
- **Findings:** 4 BLOCKING (BLK-C2-1 missing tests, BLK-C2-2 E-SHD-015 taxonomy, BLK-C2-3 demo README v1.6, BLK-C2-4 doc stub comment)
- **Review notes:** All 4 code fixes (B1/B2/SEC-001/SEC-002) verified CORRECT; blocking items are test gaps and documentation misses
- **Action taken:** Dispatching implementer (BLK-C2-2,3,4) + test-writer (BLK-C2-1)

## Security Review

- **Reviewer:** vsdd-factory:security-reviewer
- **Verdict:** CLEAN (after fixes applied)
- **Findings:**
  - SEC-001 MEDIUM CWE-22: `rotate_changelog_at` accepts unconstrained archive_path with no containment check → routed to implementer, fix = `..` component guard + MigrateError::InvalidPath
  - SEC-002 LOW CWE-252: `target_path.parent().unwrap_or_else(...)` silent CWD fallback → routed to implementer, fix = `ok_or_else` returning `HookResult::Error { E-SHD-015 }`
  - SEC-003 LOW CWE-209: Internal paths in error payloads → accepted as-is (local-only tool, no network surface)
