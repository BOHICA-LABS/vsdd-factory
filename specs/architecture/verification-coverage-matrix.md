---
document_type: verification-coverage-matrix
level: L4
section: verification-coverage-matrix
version: "1.28"
status: draft
producer: architect
timestamp: 2026-06-24T00:00:00Z
last_amended: "2026-09-10 (v1.28) — S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (architect; per BC-1.18.008 v1.8's own Changelog routing note, F-C3-P8-002; adjudication of a finding from a fresh LOCAL adversarial pass on S-25.02 F4 cluster-3 — the pass immediately following the v1.7 burst): VP-124 SS-01 module-table row description EXTENDED with the NEW happy-path canonical-write verification invariant facet (BC-1.18.008 v1.8, F-C3-P8-002, NEW Postcondition 5 step (ii) happy-path read-back extension + NEW Postcondition 6(c) Summary paragraph + NEW Invariant 5): Postcondition 5 step (ii)'s ORDINARY, non-recovery, uninterrupted completion of the canonical-truncate write (the MORE-frequently-executed path, previously LESS verified than its own crash-recovery heal counterpart — an unsanctioned asymmetry) now receives the SAME post-hoc disk read-back verification already required for sealed-shard writes and the DANGEROUS-window heal write: after `write_atomic` returns, a FRESH on-disk `(length, SHA-256)` read-back MUST match the Backfill Recovery Manifest's `(final_bytes, final_sha256)` — already durably published in step (i) — before the migration reports success; a mismatch (EC-013, NEW) fails loud as `E-SHD-013` (NEW) rather than silently completing over corrupted content. NEW §Invariant 5 generalizes the disk-read-back discipline as one explicit principle spanning all THREE destructive one-time-migration write sites (sealed-shard write, DANGEROUS-window heal write, happy-path canonical-truncate write). The four pre-existing facets (atomicity-under-interruption; idempotency-against-already-sharded; recovery-confirmation correctness; heal slice-verification) are UNCHANGED. VP-124 v1.2→v1.3 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. subsystems_affected unchanged. Per BC-1.18.008 v1.8's own Changelog note: 'VP citation change; architect must propagate this facet to VP-INDEX.md, verification-architecture.md, and verification-coverage-matrix.md per vp_index_is_vp_catalog_source_of_truth (POLICY 9), landing alongside the still-outstanding VP-123 third-facet (v1.4), VP-124 third-facet (v1.6), and VP-124 fourth-facet (v1.7) propagations' — actioned here (all now propagated in full). Individual VP-124.md body file NOT updated this burst (out of this burst's dispatched file scope, consistent with prior precedent). BC-1.18.008.md, prd-supplements/error-taxonomy.md, code, tests, story, and STATE.md NOT touched by this burst (out of architect's scope). Refs: S-25.02, E-25, ADR-051, F-C3-P8-002, BC-1.18.008 v1.8. [Prior: 2026-09-10 (v1.27) — S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (architect; per BC-1.18.008 v1.7's own Changelog routing note, F-C3-P7-001; adjudication of two findings from a fresh-context LOCAL adversarial pass on S-25.02 F4 cluster-3 — the pass immediately following the v1.6 cross-vendor burst): VP-124 SS-01 module-table row description EXTENDED with the NEW heal slice-verification invariant facet (BC-1.18.008 v1.7, F-C3-P7-001, NEW Manifest-Authoritative Slice-and-Verify Rule §Postcondition 5 + extended §Postcondition 6(c)/§Invariant 4 disk-read-back scope + extended §Invariant 3): at the confirmed DANGEROUS window, the healed content is ALWAYS obtained by slicing the canonical file's current bytes at the Manifest-derived offset `original_bytes - final_bytes` (never from a summed shard-index `bytes_at_seal`), and that slice is NEVER written unless it independently satisfies `sliced.len() == final_bytes AND sha256(sliced) == final_sha256` — a slice that fails this verification (EC-011, NEW) fails loud as `E-SHD-012` (NEW) with no write, and the completed heal's own write receives the SAME post-hoc disk read-back verification Postcondition 6(c)/Invariant 4 already requires for sealed-shard writes. The three pre-existing facets (atomicity-under-interruption; idempotency-against-already-sharded; recovery-confirmation correctness) are UNCHANGED. VP-124 v1.1→v1.2 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. subsystems_affected unchanged. Per BC-1.18.008 v1.7's own Changelog note: 'VP citation change; architect must propagate this facet to VP-INDEX.md, verification-architecture.md, and verification-coverage-matrix.md per vp_index_is_vp_catalog_source_of_truth (POLICY 9), landing alongside the still-outstanding third-facet propagation from v1.6 (and the VP-123 per-shard-cap facet propagation still outstanding from v1.4)' — actioned here. Individual VP-124.md body file NOT updated this burst (out of this burst's dispatched file scope, consistent with the VP-123 v1.1 and VP-124 v1.1 precedent). F-C3-P7-002 (the decision-log.md marker-table regex correction, same BC-1.18.008 v1.7 burst) introduces NO VP citation change per BC-1.18.008 v1.7's own fix-burst note — it extends the EXISTING VP-123 Record-integrity facet's fixture coverage (EC-012), not a new facet or ID; no action required here. BC-1.18.008.md, prd-supplements/error-taxonomy.md, code, tests, story, and STATE.md NOT touched by this burst (out of architect's scope). Refs: S-25.02, E-25, ADR-051, F-C3-P7-001, BC-1.18.008 v1.7. [Prior: 2026-09-10 (v1.26) — S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (architect; per BC-1.18.008 v1.6's own Changelog routing note, F-C3-P6-001; adjudication of a fresh-context CROSS-VENDOR (OpenAI Codex) adversarial review): VP-124 SS-01 module-table row description EXTENDED with the NEW recovery-confirmation correctness invariant facet (BC-1.18.008 v1.6, F-C3-P6-001, NEW Backfill Recovery Manifest §Postcondition 3 + NEW Recovery-Confirmation Rule §Postcondition 5 + tightened §Invariant 3): the SAFE/DANGEROUS/AMBIGUOUS disposition of a re-invocation against an already-published shard-index is determined SOLELY by exact whole-file `(length, SHA-256)` comparison against the Backfill Recovery Manifest's `original_bytes`/`original_sha256` and `final_bytes`/`final_sha256` pairs — NEVER by a structural byte-prefix comparison against a re-concatenation of already-sealed shards, which false-positives on legitimate repeated-prefix content (EC-009) and silently destroys real data; a match against neither manifest pair fails loud as `E-SHD-011` (EC-010). The two pre-existing facets (atomicity-under-interruption; idempotency-against-already-sharded) are UNCHANGED. VP-124 v1.0→v1.1 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. subsystems_affected unchanged. Per BC-1.18.008 v1.6's own Changelog note: 'VP citation change; architect must propagate this facet to VP-INDEX.md, verification-architecture.md, and verification-coverage-matrix.md per vp_index_is_vp_catalog_source_of_truth (POLICY 9)' — actioned here. Individual VP-124.md body file NOT updated this burst (out of this burst's dispatched file scope, consistent with the VP-123 v1.1 precedent). BC-1.18.008.md, prd-supplements/error-taxonomy.md, code, tests, story, and STATE.md NOT touched by this burst (out of architect's scope). Refs: S-25.02, E-25, ADR-051, F-C3-P6-001, BC-1.18.008 v1.6. [Prior: 2026-09-10 (v1.25) — S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (architect; per BC-1.18.008 v1.4's own Changelog routing note, F-C3-P3-001): VP-123 SS-01 module-table row description EXTENDED with the NEW per-shard-cap invariant facet (BC-1.18.008 v1.4, F-C3-P3-001, NEW Postcondition 6(c) + NEW Invariant 4): every sealed shard's `bytes_at_seal` is `<= shard_cap_bytes` UNLESS flagged `oversized_record: true` (EC-002's pre-existing oversized-single-record exception, or the NEW degenerate oversized-preamble exception EC-008), enforced as Postcondition 6(c)'s fail-loud hard gate against actual on-disk bytes written to each sealed shard file — never merely implied by the packing procedure having run. The two pre-existing facets (byte-for-byte content-preservation; record-integrity) are UNCHANGED. VP-123 v1.0→v1.1 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (P/proptest); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. subsystems_affected unchanged. Per BC-1.18.008 v1.4's own Changelog note: 'VP citation change; architect must propagate this facet to VP-INDEX.md, verification-architecture.md, and verification-coverage-matrix.md per vp_index_is_vp_catalog_source_of_truth (POLICY 9)' — actioned here. Individual VP-123.md body file NOT updated this burst (out of this burst's dispatched file scope). BC-1.18.008.md, code, tests, story, and STATE.md NOT touched by this burst (out of architect's scope). Refs: S-25.02, E-25, ADR-051, F-C3-P3-001, BC-1.18.008 v1.4. [Prior: 2026-09-07 (v1.24) — S-25.02 cluster-2 LOCAL adversary pass-4 facet-extension POLICY 9 propagation (architect; per BC-1.18.006 v1.8's own routing note): VP-119 SS-01 module-table row description EXTENDED with the NEW sealed-shard write-once/immutability facet (BC-1.18.006 v1.8, F-C2-P4-002, NEW Postcondition 8): no `[[shard]]`-eligible destination path is ever overwritten by a subsequent seal-publish attempt; a `seq` collision surfaces as `HookResult::Error`/`E-SHD-009`, never a silent overwrite, per NEW §EC-024. VP-119 v1.1→v1.2 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (P/proptest); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. subsystems_affected unchanged. The companion root-cause finding (F-C2-P4-001, corrected Invariant 10, catch point (i) self-heal-first ordering, EC-023) extends the STILL-UNALLOCATED pending `VP-NNN (pending)` row (Postcondition 7 reconciliation) — consistent with the EC-013..EC-022 precedent, this VP remains F6-deferred/NOT self-allocated, so no module-table row exists to extend here; see VP-INDEX.md v3.09. Individual VP-119.md body file updated same-burst for full SoT consistency. BC-1.18.006.md, code, tests, story, and STATE.md NOT touched by this burst (out of architect's scope). Refs: S-25.02, E-25, ADR-051 §Decision 16, F-C2-P4-001, F-C2-P4-002, BC-1.18.006 v1.8. [Prior: 2026-09-07 (v1.23) — S-25.02 cluster-2 LOCAL adversary pass-3 facet-extension POLICY 9 propagation (architect; per BC-1.18.006 v1.7's own routing note): VP-120 SS-01 module-table row description extended with the NEW two-template-determinism facet (§Invariant 4 RESCOPED v1.7 from 'a single, fixed template' to 'one fixed template PER OVER-CAP CASE'; the over-cap-without-content §Postcondition 1 empty-canonical `Ok(None)` short-circuit selects the DISTINCT `build_empty_roll_retry_block_reason` template, never the unified template, per NEW §EC-021, F-C2-P3-001). VP-138/VP-139 SS-01 rows each extended with the NEW 0-byte-orphan-guard facet (NEW §Invariant 9 — both self-heal paths MUST skip, never index, a 0-byte candidate sealed-shard file, per NEW §EC-022, F-C2-P3-002). VP-120 v1.2→v1.3, VP-138 v1.0→v1.1, VP-139 v1.0→v1.1 (three facet EXTENSIONS on existing VPs, NOT new VPs). Tool columns UNCHANGED (VP-120 stays U/unit-test; VP-138/VP-139 stay I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. subsystems_affected unchanged. [Prior: 2026-09-06 (v1.22) — S-25.02 F3→F4 consistency-gate finding F-1 closure POLICY 9 propagation (formal-verifier): VP-141 (I; integration; BC-1.18.007 §PC6/§EC-006 — the archive-inclusive POLICY-1 `append_only_numbering` scan-mode invariant BC-1.18.007 v1.2 forward-referenced as `VP-NNN (pending)`) added to the SS-01 module table — a POLICY-1 append-only/gap/uniqueness audit's effective glob for a Layer-2-sharded artifact ALWAYS includes `archive/<artifact-stem>/` UNION active shards, never silently excluding an archived-only ID (POLICY-1 carve-out complement of VP-122; ADR-051 §Decision 6 POLICY-1 reconciliation subsection). SS-01 subtotal I 17→18, Row 65→66. Grand Total I 42→43, Total 140→141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. subsystems_affected unchanged. [Prior: 2026-09-06 (v1.21) — S-25.02 F2 verification-property FIX-BURST PASS-4 POLICY 9 propagation (formal-verifier): VP-140 SS-01 module-table row description extended with the amortization-advisory facet (`tracing::warn!` fires IFF `low_water_mark > floor(N/2)` while config-load ALWAYS succeeds — never `HookResult::Error` — for any `0<=low_water_mark<N`; EC-012; ADR-051 v1.4 Decision 14 Option (b), F-P4-001). VP-140 v1.0→v1.1 (a new proof LEG on an existing VP, NOT a new VP). Tool column UNCHANGED (U/unit-test); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 42, M 10, S 5, Total 140. Per-tool arithmetic 6+11+66+42+10+5=140 VERIFIED. Per-subsystem row-sum 65+5+14+17+14+5+14+1+5=140 VERIFIED. subsystems_affected unchanged. [Prior: 2026-09-05 (v1.20) — S-25.02 F2 verification-property FIX-BURST PASS-3 POLICY 9 propagation (formal-verifier): VP-140 (U; unit-test; BC-1.18.005 §Postcondition 8 — item-count trigger off-by-one `current_item_count + 1 > N` + `low_water_mark` rotation-target config validation default `floor(N/2)`/fail-loud `0<=low_water_mark<N`, EC-008/EC-010/EC-011; ADR-051 v1.3 Decision 14) added to the SS-01 module table — the dedicated PC8 item-count-trigger VP (item-count-shape analogue of VP-116's byte-boundary role; VP-116/117 both scope only the `\"flat\"` byte-size shape). SS-01 subtotal K 4, P 7, U 33→34, I 17, S 3, Row 64→65. Grand Total K 6, P 11, U 65→66, I 42, M 10, S 5, Total 139→140. Per-tool arithmetic 6+11+66+42+10+5=140 VERIFIED. Per-subsystem row-sum 65+5+14+17+14+5+14+1+5=140 VERIFIED. subsystems_affected unchanged. [Prior: 2026-09-05 (v1.19) — S-25.02 F2 verification-property FIX-BURST PASS-2 POLICY 9 propagation (formal-verifier): VP-135 (P; proptest; BC-1.18.012 B1 backfill content-preservation), VP-136 (I; integration; BC-1.18.012 census+fail-loud-rollback-E-SHD-003+idempotency), VP-137 (S; static-check; BC-1.18.012 no-new-Cohort-B), VP-138 (I; integration; BC-1.18.006 E-SHD-006 resume-from-truncate self-heal), VP-139 (I; integration; BC-1.18.006 E-SHD-007 index-reconciliation self-heal) added to the SS-01 module table; VP-118 SS-01 row title rewritten to the CORRECTED staged four-step COPY+TRUNCATE-IN-PLACE roll + append-only-tail index (F-P2-003/005), VP-120 SS-01 row title rewritten to the CORRECTED unified tool-independent retry + shard-seal-write E-SHD-001 (F-P2-002/003) per BC-1.18.006 v1.2. SS-01 subtotal K 4, P 6→7, U 33, I 14→17, S 2→3, Row 59→64. Grand Total K 6, P 10→11, U 65, I 39→42, M 10, S 4→5, Total 134→139. Per-tool arithmetic 6+11+65+42+10+5=139 VERIFIED. Per-subsystem row-sum 64+5+14+17+14+5+14+1+5=139 VERIFIED. subsystems_affected unchanged. [Prior: 2026-09-05 (v1.18) — S-25.02 F2 verification-property FIX-BURST POLICY 9 propagation (formal-verifier): VP-131 (U; unit-test; BC-1.18.009 EC-003 B1 fail-loud rotate_changelog failure E-SHD-004; closes adversary F-S2502-F2-004), VP-132 (P; proptest; BC-1.18.011 content-preservation), VP-133 (I; integration; BC-1.18.011 census+atomicity+rollback E-SHD-005+idempotency+SS-05/06 sub-split), VP-134 (S; static-check; BC-1.18.011 no-new-Cohort-B dependency) added to the SS-01 module table; VP-126 SS-01 row title rewritten to CORRECTED single-actor form (rotate_changelog ONLY; zero gate-side prepend_changelog_item call sites) per BC-1.18.009 v1.1 (F-S2502-F2-001). SS-01 subtotal K 4 unchanged, P 5→6, U 32→33, I 13→14, S 1→2, Row 55→59. Grand Total K 6, P 9→10, U 64→65, I 38→39, M 10, S 3→4, Total 130→134. Per-tool arithmetic 6+10+65+39+10+4=134 VERIFIED. Per-subsystem row-sum 59+5+14+17+14+5+14+1+5=134 VERIFIED. subsystems_affected unchanged. [Prior: 2026-09-05 (v1.17) — S-25.02 F2 verification-property extension POLICY 9 propagation (formal-verifier): VP-116..VP-128 (13 VPs) added to the SS-01 module table (K +1 = VP-116; U +4 = VP-117/VP-120/VP-122/VP-127; I +3 = VP-118/VP-124/VP-128; P +4 = VP-119/VP-121/VP-123/VP-125; S +1 = VP-126; BC-1.18.005..010; anchor S-25.02); VP-129 (S; static-check) + VP-130 (I; integration) added to the SS-07 module table (BC-7.08.001; anchor S-25.02). SS-01 subtotal K 3→4, P 1→5, U 28→32, I 10→13, S 0→1, Row 42→55. SS-07 subtotal I 5→6, S 0→1, Row 12→14. Grand Total K 5→6, P 5→9, U 60→64, I 34→38, S 1→3, Total 115→130. Per-tool arithmetic 6+9+64+38+10+3=130 VERIFIED. Per-subsystem row-sum 55+5+14+17+14+5+14+1+5=130 VERIFIED. subsystems_affected unchanged (SS-01, SS-07 already present). F4-provisional cap-formula bounds noted on VP-116/117/123/130 per ADR-051 §D2. [Prior: 2026-09-02 (v1.16) — S-15.03 VP registration POLICY 9 propagation (architect): VP-109..VP-113 (unit-test; BC-10.13.001; anchor S-15.03) added as a NEW SS-10 module table (crates/last-amended-migrate); VP-114 (unit-test; BC-5.45.001; anchor S-15.03) added to SS-05 module table; VP-115 (unit-test; BC-4.18.001; anchor S-15.03) added to SS-04 module table. SS-04 subtotal U 5→6, Row 16→17; SS-05 subtotal U 0→1, Row 13→14; new SS-10 subtotal U=5, Row=5. Grand Total U 53→60, Total 108→115. Per-tool arithmetic 5+5+60+34+10+1=115 VERIFIED. Per-subsystem row-sum 42+5+14+17+14+5+12+1+5=115 VERIFIED. subsystems_affected += SS-10. [Prior: 2026-09-01 (v1.15) — S-25.01 pass 11 F-P11-001 HIGH resolution (architect; POLICY 9 propagation + POLICY 4 mis-anchor fix): SS-01 module table VP-108 row title and BC-anchor corrected to derive from VP-108.md v1.5 SoT H1 title 'Marker Lifecycle Audited Events — Write and Clear Path Emission Correctness' (was stale 'marker.cleared Audited-Clear Event — Clear Path Emission Correctness', omitting the write path added at VP-108.md v1.3/v1.4); BC-anchor parenthetical corrected to BC-1.18.001 §PC4, BC-1.18.003 §PC1/PC3/PC4/PC5, BC-3.08.001 Events 9-10; ADR-048 §D4 (was BC-1.18.003 PC1/PC3/PC4, BC-3.08.001 Event 9; ADR-048 §D4 only — omitted write-path BC-1.18.001 §PC4 and BC-3.08.001 Event 10). Tool column (U/unit-test), subsystem (SS-01), and all other columns unchanged; description-only correction, no VP count or arithmetic change — per-tool arithmetic 5+5+53+34+10+1=108 and per-subsystem row-sum 42+5+14+16+13+5+12+1=108 remain VERIFIED. [Prior: 2026-08-31 (v1.14) — VP-108 (marker.cleared Audited-Clear Event; ADR-048 §D4; POLICY 9) added to SS-01 module table: U column (unit-test); BC-1.18.003 PC1/PC3/PC4 + BC-3.08.001 Event 9; anchor S-25.01; VP-107 title updated to T1-only scope (ADR-048 §D3 v1.1 amendment); SS-01 subtotal U 27→28, Row 41→42; Grand Total U 52→53, Total 107→108; per-tool arithmetic 5+5+53+34+10+1=108 VERIFIED; per-subsystem row-sum 42+5+14+16+13+5+12+1=108 VERIFIED; §3 SS-01 row detail updated; §3 Grand Totals updated; input-hash 7dd067b→c564dd1. [Prior: 2026-08-31 (v1.13) — VP-107 (Ungated-Escape Invariant; ADR-048 §D3; POLICY 9) added to SS-01 module table: U column (unit-test); BC-1.18.002 §INV6; anchor S-25.01; SS-01 subtotal U 26→27, Row 40→41; Grand Total U 51→52, Total 106→107; per-tool arithmetic 5+5+52+34+10+1=107 VERIFIED; per-subsystem row-sum 41+5+14+16+13+5+12+1=107 VERIFIED; §3 SS-01 row detail updated; §3 Grand Totals updated; input-hash updated. [Prior: 2026-08-30 (v1.12) — S-25.01 re-audit LOW-1 (architect): VP-105 SS-04 row title corrected to VP-105.md H1 verbatim — removed stray 'validate-unvalidated-mutation-marker ' prefix; semicolon changed to comma ('...Exists; Passes' → '...Exists, Passes'). total_vps UNCHANGED 106. input-hash 0e8390c→1f93bde. [Prior: 2026-08-30 (v1.11) — F2 POLICY 9 consistency-audit fix (architect): VP-105 SS-04 row title corrected to v1.1 verbatim (add 'and git commit/push' git arm); BC source corrected from 'BC-1.18.002 PC1+PC3' to 'BC-1.18.002 PC1+PC2+PC3+PC4' (v1.1 adds PC2 git-block arm + PC4 absent-marker-git-pass arm). total_vps UNCHANGED 106. input-hash needs recompute (state-manager). [Prior: 2026-08-30 (v1.10) — validation-integrity-layer1 F2 spec-evolution burst (architect; POLICY 9 propagation): VP-102/103/104/106 added to SS-01 module table (U column; unit-test; BC-1.18.001 — INDETERMINATE outcome model); VP-105 added to SS-04 module table (I column; integration; BC-1.18.002 — next-advance gate); SS-01 subtotal U 22→26 (+4), Row 36→40; SS-04 subtotal I 6→7 (+1), Row 15→16; Grand Total U 47→51, I 33→34, Total 101→106. Per-tool arithmetic 5+5+51+34+10+1=106 VERIFIED. Per-subsystem row-sum 40+5+14+16+13+5+12+1=106 VERIFIED. §3 Grand Totals updated. input-hash d1075c2→cfb38c7. [Prior: 2026-07-29 (v1.9) — D-943 S-21.04 pass-28 POLICY 9 propagation (state-manager): VP-097 SS-01 row title corrected — 'Cannot Escape' → 'Cannot Resolve Outside' (verbatim VP-097.md title since v1.0; POLICY 9 no-op: proof_method kani-proof unchanged); input-hash 015260d→16cc408. [Prior: 2026-07-16 (v1.8) — F-P2-001 POLICY 9 re-propagation (architect): VP-095 SS-04 catalog row title corrected to v1.3 H1 verbatim form (host::read_prefix(262144)); v1.7 propagated to VP-095 v1.2 Phase-B title (8192) which v1.3 superseded same-burst 2026-07-16; §3 SS-04 data row corrected U 4→5 I 7→6 (pre-existing omission from v1.7 partial update — §1 subtotal was correct; §3 data row not updated); input-hash f0fab9c→5279415. [Prior: 2026-07-16 (v1.7) — S-19.07 Phase-B POLICY 9 propagation (architect): VP-095 SS-04 row title updated to Phase-B form; ✓ moved from I column to U column; SS-04 subtotal U 4→5, I 7→6; Grand Total U 46→47, I 34→33; input-hash c9ec678→f0fab9c. [Prior: 2026-07-13 (v1.6) — pass-11 F-P11-001 POLICY 9 propagation (architect): VP-099 SS-07 row title updated to both-ends form ('Starts With ^' → 'Is Fully Anchored (^...$)'); input-hash 893a501→c9ec678. [Prior: 2026-07-08 (v1.5) — E-19 pass-28 VP-096 boundary-wording sync (architect): VP-096 SS-04 row title updated to exclusive form per BC-4.13.001 §Invariant 9 adjudication (D-781): 'Byte-Exact Prefix Through Second --- Delimiter' → 'Byte-Exact Prefix Up To (Excluding) Second --- Delimiter Line'. input-hash 7a7ac8c→893a501. [Prior: 2026-07-06 (v1.4) — E-19 VP package POLICY 9 propagation (architect): VP-094 added to SS-05 module table (I; BC-5.42.001); VP-095/096 added to SS-04 (I + P; BC-4.13.001); VP-097/098/100/101 added to SS-01 (K + I + I + I; BC-2.07.001+BC-2.02.011 + BC-2.07.001 + BC-3.08.001+DI-019 + BC-1.17.001); VP-099 added to SS-07 (I; no BC). SS-01 subtotal K 2→3 (+VP-097), I 7→10 (+VP-098/100/101), row total 32→36. SS-04 subtotal P 1→2 (+VP-096), I 6→7 (+VP-095), row total 13→15. SS-05 subtotal I 5→6 (+VP-094), row total 12→13. SS-07 subtotal I 4→5 (+VP-099), row total 11→12. Grand Total K 4→5, P 4→5, I 28→34, row total 93→101. Per-tool arithmetic 5+5+46+34+10+1=101 VERIFIED. Per-subsystem row-sum 36+5+14+15+13+5+12+1=101 VERIFIED. §2 VP-100 judgment call added (SS-01 primary; SS-03 secondary). §3 Grand Totals updated. [Prior: 2026-06-24 (v1.3) — S-18.04b-prereq BC authoring burst (architect POLICY 9 propagation): VP-093 added to SS-01 module table (integration; SS-01; DI-020, DI-025; BC-1.16.001 — dispatcher git_context injection on PostToolUse Bash git-commit events; four-field completeness; fail-open on git error; no injection on non-qualifying events). SS-01 subtotal I 6→7, row total 31→32. Grand Total I 27→28, row total 92→93. Per-tool arithmetic 4+4+46+28+10+1=93 VERIFIED. Per-subsystem row-sum 32+5+14+13+12+5+11+1=93 VERIFIED. [Prior: 2026-06-16 (v1.2) — D-615 E-18 STORY PASS-1 FIX WAVE INTEGRATION BURST (state-manager POLICY 9 propagation): VP-092 added to SS-06 module table (unit-test; SS-06; DI-020; BC-6.25.001 — check-state-health CLAUDE_AUTOCOMPACT_PCT_OVERRIDE advisory check; never blocks). SS-06 subtotal U 0→1, row total 4→5. Grand Total U 45→46, row total 91→92. Per-tool arithmetic 4+4+46+27+10+1=92 VERIFIED. Per-subsystem row-sum 31+5+14+13+12+5+11+1=92 VERIFIED. [Prior: 2026-06-16 (v1.1) — D-612 INTEGRATION BURST (state-manager POLICY 9 propagation): VP-091 added to SS-04 module table (unit-test; SS-04; DI-020; BC-4.15.001 — validate-heavy-op-delegation always-Continue advisory gate). SS-04 subtotal U 3→4, row total 12→13. Grand Total U 44→45, row total 90→91. Per-tool arithmetic 4+4+45+27+10+1=91 VERIFIED. Per-subsystem row-sum 31+5+14+13+12+4+11+1=91 VERIFIED. [Prior: 2026-06-16 (v1.0) — F2 gate decision: initial creation as a full production-grade architecture deliverable. Sources: VP-INDEX.md v2.29 (86 VPs) + VP-087..VP-090 (4 new E-18 VPs, unstaged). POST-INTEGRATION totals: total_vps=90, unit-test=44, integration=27, manual=10, static-check=1, kani-proof=4, proptest=4. Every VP assigned to its authoritative module per VP-INDEX.md scope column and VP file frontmatter. Authored per F2 gate human directive that deferred architecture derived-views be materialized now.]]]]]]]]]"
modified:
  - "2026-09-10 (v1.28) — S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (architect; per BC-1.18.008 v1.8's own Changelog routing note, F-C3-P8-002; adjudication of a fresh LOCAL adversarial review, the pass immediately following the v1.7 burst): VP-124 SS-01 row EXTENDED with the happy-path canonical-write verification invariant facet (NEW Postcondition 5 step (ii) happy-path read-back extension; NEW Postcondition 6(c) Summary paragraph; NEW §INV5; NEW §EC-013; `E-SHD-013`). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141; per-tool 6+11+66+43+10+5=141 VERIFIED; per-subsystem 66+5+14+17+14+5+14+1+5=141 VERIFIED"
  - "2026-09-10 (v1.27) — S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (architect; per BC-1.18.008 v1.7's own Changelog routing note, F-C3-P7-001; adjudication of a fresh-context LOCAL adversarial review, the pass immediately following the v1.6 cross-vendor burst): VP-124 SS-01 row EXTENDED with the heal slice-verification invariant facet (NEW Manifest-Authoritative Slice-and-Verify Rule §PC5; extended §PC6(c)/§INV4 disk-read-back scope; extended §INV3; NEW §EC-011; `E-SHD-012`). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141; per-tool 6+11+66+43+10+5=141 VERIFIED; per-subsystem 66+5+14+17+14+5+14+1+5=141 VERIFIED"
  - "2026-09-10 (v1.26) — S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (architect; per BC-1.18.008 v1.6's own Changelog routing note, F-C3-P6-001; adjudication of a fresh-context CROSS-VENDOR (OpenAI Codex) adversarial review): VP-124 SS-01 row EXTENDED with the recovery-confirmation correctness invariant facet (NEW Backfill Recovery Manifest §PC3; NEW Recovery-Confirmation Rule §PC5; tightened §INV3; NEW §EC-009/§EC-010; `E-SHD-011`). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141; per-tool 6+11+66+43+10+5=141 VERIFIED; per-subsystem 66+5+14+17+14+5+14+1+5=141 VERIFIED"
  - "2026-09-10 (v1.25) — S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (architect; per BC-1.18.008 v1.4's own Changelog routing note, F-C3-P3-001): VP-123 SS-01 row EXTENDED with the per-shard-cap invariant facet (NEW Postcondition 6(c); NEW Invariant 4; NEW §EC-007/§EC-008). Tool column UNCHANGED (P/proptest); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141; per-tool 6+11+66+43+10+5=141 VERIFIED; per-subsystem 66+5+14+17+14+5+14+1+5=141 VERIFIED"
  - "2026-09-07 (v1.24) — S-25.02 cluster-2 LOCAL adversary pass-4 facet-extension POLICY 9 propagation (architect): VP-119 SS-01 row EXTENDED with the sealed-shard write-once/immutability facet (NEW Postcondition 8; NEW §EC-024; `E-SHD-009`); VP-119 v1.1→v1.2 (facet extension, not a new VP); pending `VP-NNN` (Postcondition 7 reconciliation) remains F6-deferred/NOT self-allocated (EC-023 extension recorded in VP-INDEX changelog only, no module-table row). Tool column UNCHANGED; SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141; per-tool 6+11+66+43+10+5=141 VERIFIED; per-subsystem 66+5+14+17+14+5+14+1+5=141 VERIFIED"
  - "2026-09-07 (v1.23) — S-25.02 cluster-2 LOCAL adversary pass-3 facet-extension POLICY 9 propagation (architect): VP-120 SS-01 row extended with the two-template-determinism facet (§Invariant 4 rescoped; NEW §EC-021); VP-138/VP-139 SS-01 rows extended with the 0-byte-orphan-guard facet (NEW §Invariant 9; NEW §EC-022); VP-120 v1.2→v1.3, VP-138/VP-139 v1.0→v1.1 (facet extensions, not new VPs). Tool columns UNCHANGED; SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141; per-tool 6+11+66+43+10+5=141 VERIFIED; per-subsystem 66+5+14+17+14+5+14+1+5=141 VERIFIED"
  - "2026-09-06 (v1.22) — S-25.02 F3→F4 consistency-gate F-1 closure POLICY 9 propagation: VP-141 added SS-01 (I; integration; BC-1.18.007 §PC6/§EC-006 archive-inclusive POLICY-1 scan-mode). SS-01 I 17→18, Row 65→66; Grand Total I 42→43, Total 140→141; per-tool 6+11+66+43+10+5=141 VERIFIED; per-subsystem 66+5+14+17+14+5+14+1+5=141 VERIFIED"
  - "2026-09-05 (v1.18) — S-25.02 F2 verification-property FIX-BURST POLICY 9 propagation: VP-131 added SS-01 (U; BC-1.18.009 EC-003 B1 fail-loud E-SHD-004; F-S2502-F2-004); VP-132 SS-01 (P; BC-1.18.011 content-preservation); VP-133 SS-01 (I; BC-1.18.011 census+atomicity+rollback E-SHD-005+idempotency+SS-05/06 sub-split); VP-134 SS-01 (S; BC-1.18.011 no-new-Cohort-B dependency); VP-126 SS-01 row title→CORRECTED single-actor form (F-S2502-F2-001). SS-01 P 5→6, U 32→33, I 13→14, S 1→2, Row 55→59. Grand Total P 9→10, U 64→65, I 38→39, S 3→4, Total 130→134; per-tool 6+10+65+39+10+4=134 VERIFIED; per-subsystem 59+5+14+17+14+5+14+1+5=134 VERIFIED"
  - "2026-09-02 (v1.16) — S-15.03 VP registration POLICY 9 propagation: VP-109..VP-113 added as new SS-10 module table; VP-114 added to SS-05; VP-115 added to SS-04; SS-04 U 5→6 Row 16→17; SS-05 U 0→1 Row 13→14; new SS-10 U=5 Row=5; Grand Total U 53→60, Total 108→115; per-tool 5+5+60+34+10+1=115 VERIFIED; per-subsystem 42+5+14+17+14+5+12+1+5=115 VERIFIED"
  - "2026-09-01 (v1.15) — F-P11-001 HIGH fix (architect; POLICY 9 + POLICY 4): VP-108 SS-01 module table row title + BC-anchor corrected to VP-108.md v1.5 SoT ('Marker Lifecycle Audited Events — Write and Clear Path Emission Correctness'; BC-1.18.001 §PC4, BC-1.18.003 §PC1/PC3/PC4/PC5, BC-3.08.001 Events 9-10; ADR-048 §D4); description-only correction, no count/arithmetic change — per-tool 5+5+53+34+10+1=108 and per-subsystem 42+5+14+16+13+5+12+1=108 VERIFIED"
  - "2026-08-31 (v1.14) — ADR-048 v1.1 POLICY 9: VP-108 added SS-01 (U; unit-test; BC-1.18.003 PC1/PC3/PC4 + BC-3.08.001 Event 9; ADR-048 §D4; anchor S-25.01); VP-107 title→T1-only scope (ADR-048 §D3 v1.1); SS-01 U 27→28, Row 41→42; Grand Total U 52→53, Total 107→108; per-tool 5+5+53+34+10+1=108 VERIFIED; per-subsystem 42+5+14+16+13+5+12+1=108 VERIFIED; input-hash 7dd067b→c564dd1"
  - "2026-08-31 (v1.13) — S-25.01 POLICY 9: VP-107 added SS-01 (U; unit-test; BC-1.18.002 INV6; ADR-048 D3); SS-01 U 26→27, Row 40→41; Grand Total U 51→52, Total 106→107; per-tool 5+5+52+34+10+1=107 VERIFIED; per-subsystem 41+5+14+16+13+5+12+1=107 VERIFIED; input-hash updated"
  - "2026-08-30 (v1.12) — S-25.01 re-audit LOW-1: VP-105 SS-04 row title→VP-105.md H1 verbatim (remove stray prefix, semicolon→comma); input-hash 0e8390c→1f93bde"
  - "2026-08-30 (v1.11) — F2 consistency-audit: VP-105 SS-04 row title→v1.1 verbatim (add 'and git commit/push'); BC source→BC-1.18.002 PC1+PC2+PC3+PC4. total_vps UNCHANGED 106"
  - "2026-08-30 (v1.10) — E-25 F2 validation-integrity-layer1: VP-102/103/104/106 added SS-01 (U); VP-105 added SS-04 (I); SS-01 U 22→26, Row 36→40; SS-04 I 6→7, Row 15→16; Grand Total U 47→51, I 33→34, Total 101→106; input-hash d1075c2→cfb38c7"
  - "2026-07-29 (v1.9) — D-943 VP-097 SS-01 title corrected 'Cannot Escape' → 'Cannot Resolve Outside'; input-hash 015260d→16cc408"
  - "2026-07-16 (v1.8) — F-P2-001 POLICY 9 re-propagation: VP-095 SS-04 row title corrected to v1.3 form (262144); §3 SS-04 data row U 4→5 I 7→6 (omission from v1.7); input-hash f0fab9c→5279415"
  - "2026-07-16 (v1.7) — S-19.07 Phase-B POLICY 9 propagation: VP-095 SS-04 row title→Phase-B form; ✓ I→U; SS-04 subtotal U 4→5, I 7→6; Grand Total U 46→47, I 34→33; input-hash c9ec678→f0fab9c"
  - "2026-07-13 (v1.6) — pass-11 F-P11-001 POLICY 9 propagation: VP-099 SS-07 row title updated to both-ends form; input-hash 893a501→c9ec678"
  - "2026-07-08 (v1.5) — E-19 pass-28 VP-096 boundary-wording sync: VP-096 SS-04 row title updated to exclusive form per BC-4.13.001 §Invariant 9 (D-781); input-hash 7a7ac8c→893a501"
  - "2026-07-06 (v1.4) — E-19 VP package POLICY 9 propagation: VP-094 (SS-05), VP-095/096 (SS-04), VP-097/098/100/101 (SS-01), VP-099 (SS-07) added; K 4→5, P 4→5, I 28→34, Total 93→101; §2 VP-100 judgment call added; input-hash 61531bf→7a7ac8c"
  - "2026-06-24 (v1.3) — VP-093 added to SS-01 module; SS-01 I 6→7, total 31→32; grand total 92→93"
  - "2026-06-16 (v1.2) — D-615 VP-092 added to SS-06 module; SS-06 U 0→1, total 4→5; grand total 91→92"
  - "2026-06-16 (v1.1) — D-612 VP-091 added to SS-04 module; SS-04 U 3→4, total 12→13; grand total 90→91"
  - "2026-06-16 (v1.0 initial creation)"
phase: 1b
inputs: [verification-properties/VP-INDEX.md]
input-hash: "f52e536"
traces_to: VP-INDEX.md
subsystems_affected:
  - SS-01
  - SS-02
  - SS-03
  - SS-04
  - SS-05
  - SS-06
  - SS-07
  - SS-09
  - SS-10
---

# Verification Coverage Matrix

> **Source-of-truth relationship:** VP-INDEX.md is the authoritative VP catalog.
> This matrix derives from VP-INDEX.md §Full Index (scope column). Any change to
> VP-INDEX — VP addition, retirement, module reassignment, tool change, or phase
> reassignment — MUST propagate to this matrix in the same burst (POLICY 9 /
> VP-INDEX Propagation Obligation).
>
> **Module assignment rule:** Each VP is assigned to exactly one primary subsystem.
> When a VP lists multiple subsystems, the PRIMARY subsystem is the first-listed
> subsystem in VP-INDEX.md §Full Index Scope column, consistent with the VP file's
> frontmatter `scope:` field. Multi-subsystem VPs appear in exactly one module table;
> their additional subsystem affiliations are noted in the Subsystems column.
>
> **Grand-total arithmetic invariant:** Each VP is counted exactly once in the grand-
> total row. The per-tool column sums (K+P+U+I+M+S) must equal 140. This invariant
> must be verified on every update to this document.

## §Changelog

| Version | Date | Author | Change |
|---------|------|--------|--------|
| v1.28 | 2026-09-10 | architect | S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (per BC-1.18.008 v1.8's own Changelog routing note, F-C3-P8-002; adjudication of a fresh LOCAL adversarial pass, the pass immediately following the v1.7 burst): VP-124 SS-01 row EXTENDED with the NEW happy-path canonical-write verification invariant facet (BC-1.18.008 v1.8, NEW Postcondition 5 step (ii) happy-path read-back extension + NEW Postcondition 6(c) Summary paragraph + NEW Invariant 5) — Postcondition 5 step (ii)'s ORDINARY, non-recovery, uninterrupted completion of the canonical-truncate write (the MORE-frequently-executed path, previously LESS verified than its own crash-recovery heal counterpart — an unsanctioned asymmetry) now receives the SAME post-hoc disk read-back verification already required for sealed-shard writes and the DANGEROUS-window heal write: after `write_atomic` returns, a FRESH on-disk `(length, SHA-256)` read-back MUST match the Backfill Recovery Manifest's `(final_bytes, final_sha256)` — already durably published in step (i) — before the migration reports success; a mismatch (EC-013, NEW) fails loud as `E-SHD-013` (NEW) rather than silently completing over corrupted content. NEW §Invariant 5 generalizes the disk-read-back discipline as one explicit principle spanning all THREE destructive one-time-migration write sites (sealed-shard write, DANGEROUS-window heal write, happy-path canonical-truncate write). The four pre-existing facets (atomicity-under-interruption; idempotency-against-already-sharded; recovery-confirmation correctness; heal slice-verification) are UNCHANGED. VP-124 v1.2→v1.3 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. |
| v1.27 | 2026-09-10 | architect | S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (per BC-1.18.008 v1.7's own Changelog routing note, F-C3-P7-001; adjudication of two findings from a fresh-context LOCAL adversarial pass, the pass immediately following the v1.6 cross-vendor burst): VP-124 SS-01 row EXTENDED with the NEW heal slice-verification invariant facet (BC-1.18.008 v1.7, NEW Manifest-Authoritative Slice-and-Verify Rule §Postcondition 5 + extended §Postcondition 6(c)/§Invariant 4 disk-read-back scope + extended §Invariant 3) — at the confirmed DANGEROUS window, the healed content is ALWAYS obtained by slicing the canonical file's current bytes at the Manifest-derived offset `original_bytes - final_bytes` (never from a summed shard-index `bytes_at_seal`), and that slice is NEVER written unless it independently satisfies `sliced.len() == final_bytes AND sha256(sliced) == final_sha256` — a slice that fails this verification (EC-011, NEW) fails loud as `E-SHD-012` (NEW) with no write, and the completed heal's own write receives the SAME post-hoc disk read-back verification Postcondition 6(c)/Invariant 4 already requires for sealed-shard writes. The three pre-existing facets (atomicity-under-interruption; idempotency-against-already-sharded; recovery-confirmation correctness) are UNCHANGED. VP-124 v1.1→v1.2 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. |
| v1.26 | 2026-09-10 | architect | S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (per BC-1.18.008 v1.6's own Changelog routing note, F-C3-P6-001; adjudication of a fresh-context CROSS-VENDOR (OpenAI Codex) adversarial review): VP-124 SS-01 row EXTENDED with the NEW recovery-confirmation correctness invariant facet (BC-1.18.008 v1.6, NEW Backfill Recovery Manifest §Postcondition 3 + NEW Recovery-Confirmation Rule §Postcondition 5 + tightened §Invariant 3) — the SAFE/DANGEROUS/AMBIGUOUS disposition of a re-invocation against an already-published shard-index is determined SOLELY by exact whole-file `(length, SHA-256)` comparison against the Backfill Recovery Manifest's `original_bytes`/`original_sha256` and `final_bytes`/`final_sha256` pairs, NEVER by a structural byte-prefix comparison against a re-concatenation of already-sealed shards (the prior heuristic false-positived on realistic repeated-prefix content, EC-009, and silently destroyed data); a match against neither manifest pair fails loud as `E-SHD-011` (EC-010). The two pre-existing facets (atomicity-under-interruption; idempotency-against-already-sharded) are UNCHANGED. VP-124 v1.0→v1.1 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (I/integration); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. |
| v1.25 | 2026-09-10 | architect | S-25.02 F4 cluster-3 fix-burst facet-extension POLICY 9 propagation (per BC-1.18.008 v1.4's own Changelog routing note, F-C3-P3-001): VP-123 SS-01 row EXTENDED with the NEW per-shard-cap invariant facet (BC-1.18.008 v1.4, NEW Postcondition 6(c) + NEW Invariant 4) — every sealed shard's `bytes_at_seal` is `<= shard_cap_bytes` UNLESS flagged `oversized_record: true` (EC-002's pre-existing exception, or the NEW degenerate oversized-preamble exception EC-008), enforced as Postcondition 6(c)'s fail-loud hard gate against actual on-disk bytes. The two pre-existing facets (byte-for-byte content-preservation; record-integrity) are UNCHANGED. VP-123 v1.0→v1.1 (a facet EXTENSION on an existing VP — NOT a new VP). Tool column UNCHANGED (P/proptest); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. |
| v1.24 | 2026-09-07 | architect | S-25.02 cluster-2 LOCAL adversary pass-4 facet-extension POLICY 9 propagation (per BC-1.18.006 v1.8's own routing note): VP-119 SS-01 row EXTENDED with the NEW sealed-shard write-once/immutability facet (NEW Postcondition 8, F-C2-P4-002) — no `[[shard]]`-eligible destination path is ever overwritten by a subsequent seal-publish attempt; a `seq` collision surfaces as `HookResult::Error`/`E-SHD-009`, never a silent overwrite, per NEW §EC-024. VP-119 v1.1→v1.2 (a facet EXTENSION on an existing VP — NOT a new VP). The companion root-cause finding (F-C2-P4-001, corrected Invariant 10, catch point (i) self-heal-first ordering, EC-023) extends the STILL-UNALLOCATED pending `VP-NNN (pending)` row (Postcondition 7 reconciliation) — consistent with the EC-013..EC-022 precedent, this VP remains F6-deferred/NOT self-allocated, so no module-table row exists to extend here; see VP-INDEX.md v3.09. Tool column UNCHANGED (P/proptest); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. |
| v1.23 | 2026-09-07 | architect | S-25.02 cluster-2 LOCAL adversary pass-3 facet-extension POLICY 9 propagation (per BC-1.18.006 v1.7's own routing note): VP-120 SS-01 row extended with the NEW two-template-determinism facet (§Invariant 4 RESCOPED to one fixed template PER over-cap case; empty-canonical `Ok(None)` short-circuit selects the DISTINCT `build_empty_roll_retry_block_reason` template, per NEW §EC-021, F-C2-P3-001). VP-138/VP-139 SS-01 rows each extended with the NEW 0-byte-orphan-guard facet (NEW §Invariant 9 — both self-heal paths MUST skip, never index, a 0-byte candidate, per NEW §EC-022, F-C2-P3-002). VP-120 v1.2→v1.3, VP-138/VP-139 v1.0→v1.1 (three facet EXTENSIONS, NOT new VPs). Tool columns UNCHANGED (VP-120 stays U, VP-138/VP-139 stay I); SS-01 subtotal and Grand Total UNCHANGED. Grand Total K 6, P 11, U 66, I 43, M 10, S 5, Total 141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. |
| v1.22 | 2026-09-06 | formal-verifier | S-25.02 F3→F4 consistency-gate finding F-1 closure POLICY 9 propagation: VP-141 (I; integration; BC-1.18.007 §PC6/§EC-006 — the archive-inclusive POLICY-1 `append_only_numbering` scan-mode invariant BC-1.18.007 v1.2 forward-referenced as `VP-NNN (pending)`) added to the SS-01 module table: a POLICY-1 append-only/gap/uniqueness audit's effective glob for a Layer-2-sharded artifact ALWAYS includes `archive/<artifact-stem>/` UNION active shards, never silently excluding an archived-only ID (POLICY-1 carve-out complement of VP-122; ADR-051 §Decision 6 POLICY-1 reconciliation subsection). SS-01 subtotal I 17→18, Row 65→66. Grand Total I 42→43, Total 140→141. Per-tool arithmetic 6+11+66+43+10+5=141 VERIFIED. Per-subsystem row-sum 66+5+14+17+14+5+14+1+5=141 VERIFIED. |
| v1.21 | 2026-09-06 | formal-verifier | S-25.02 F2 verification-property FIX-BURST PASS-4 POLICY 9 propagation: VP-140 SS-01 module-table row description extended with the amortization-advisory facet (`tracing::warn!` IFF `low_water_mark > floor(N/2)`; config-load ALWAYS succeeds — never `HookResult::Error` — for any `0<=low_water_mark<N`; EC-012; ADR-051 v1.4 Decision 14 Option (b), F-P4-001; VP-140 v1.0→v1.1, a new proof LEG on an existing VP). Tool column UNCHANGED (U/unit-test); SS-01 subtotal + Grand Total UNCHANGED. Grand Total U 66, Total 140. Per-tool arithmetic 6+11+66+42+10+5=140 VERIFIED. Per-subsystem row-sum 65+5+14+17+14+5+14+1+5=140 VERIFIED. |
| v1.20 | 2026-09-05 | formal-verifier | S-25.02 F2 verification-property FIX-BURST PASS-3 POLICY 9 propagation: VP-140 (U; unit-test; BC-1.18.005 §Postcondition 8 — item-count trigger off-by-one `current_item_count + 1 > N` + `low_water_mark` rotation-target config validation, default `floor(N/2)`, fail-loud `0<=low_water_mark<N`, EC-008/EC-010/EC-011; ADR-051 v1.3 Decision 14) added to SS-01 module table — the dedicated PC8 item-count-trigger VP (item-count-shape analogue of VP-116's byte-boundary role; VP-116/117 both scope only the `"flat"` byte-size shape). SS-01 subtotal U 33→34, Row 64→65. Grand Total U 65→66, Total 139→140. Per-tool arithmetic 6+11+66+42+10+5=140 VERIFIED. Per-subsystem row-sum 65+5+14+17+14+5+14+1+5=140 VERIFIED. |
| v1.19 | 2026-09-05 | formal-verifier | S-25.02 F2 verification-property FIX-BURST PASS-2 POLICY 9 propagation: VP-135 (P; proptest; BC-1.18.012 PC4/PC2 B1 backfill content-preservation), VP-136 (I; integration; BC-1.18.012 PC3/PC5/PC7 census+fail-loud-rollback-E-SHD-003+idempotency), VP-137 (S; static-check; BC-1.18.012 PC6/INV4/EC-005 no-new-Cohort-B), VP-138 (I; integration; BC-1.18.006 PC1 step (c)/EC-010 E-SHD-006 resume-from-truncate self-heal), VP-139 (I; integration; BC-1.18.006 PC1 step (d)/EC-011 E-SHD-007 index-reconciliation self-heal) added to SS-01 module table. VP-118 SS-01 row title rewritten to CORRECTED staged four-step COPY+TRUNCATE-IN-PLACE roll + append-only-tail index (F-P2-003/005); VP-120 SS-01 row title rewritten to CORRECTED unified tool-independent retry + shard-seal-write E-SHD-001 (F-P2-002/003) per BC-1.18.006 v1.2. SS-01 subtotal K 4, P 6→7, U 33, I 14→17, S 2→3, Row 59→64. Grand Total K 6, P 10→11, U 65, I 39→42, M 10, S 4→5, Total 134→139. Per-tool arithmetic 6+11+65+42+10+5=139 VERIFIED. Per-subsystem row-sum 64+5+14+17+14+5+14+1+5=139 VERIFIED. |
| v1.18 | 2026-09-05 | formal-verifier | S-25.02 F2 verification-property FIX-BURST POLICY 9 propagation: VP-131 (U; unit-test; BC-1.18.009 EC-003 — B1 fail-loud rotate_changelog failure E-SHD-004; closes adversary F-S2502-F2-004), VP-132 (P; proptest; BC-1.18.011 PC1 content-preservation), VP-133 (I; integration; BC-1.18.011 PC2/PC3/PC4/PC5/PC6 census+atomicity+rollback E-SHD-005+idempotency+SS-05/06 sub-split), VP-134 (S; static-check; BC-1.18.011 PC7/INV4/EC-005 no-new-Cohort-B dependency) added to SS-01 module table. VP-126 SS-01 row title rewritten to CORRECTED single-actor form (rotate_changelog ONLY; zero gate-side prepend_changelog_item call sites) per BC-1.18.009 v1.1 (F-S2502-F2-001). SS-01 subtotal K 4, P 5→6, U 32→33, I 13→14, S 1→2, Row 55→59. Grand Total K 6, P 9→10, U 64→65, I 38→39, M 10, S 3→4, Total 130→134. Per-tool arithmetic 6+10+65+39+10+4=134 VERIFIED. Per-subsystem row-sum 59+5+14+17+14+5+14+1+5=134 VERIFIED. |
| v1.17 | 2026-09-05 | formal-verifier | S-25.02 F2 verification-property extension POLICY 9 propagation: VP-116..VP-128 (13 VPs) added to SS-01 module table (K +1 VP-116; U +4 VP-117/VP-120/VP-122/VP-127; I +3 VP-118/VP-124/VP-128; P +4 VP-119/VP-121/VP-123/VP-125; S +1 VP-126; BC-1.18.005..010); VP-129 (S) + VP-130 (I) added to SS-07 module table (BC-7.08.001). SS-01 subtotal K 3→4, P 1→5, U 28→32, I 10→13, S 0→1, Row 42→55. SS-07 subtotal I 5→6, S 0→1, Row 12→14. Grand Total K 5→6, P 5→9, U 60→64, I 34→38, S 1→3, Total 115→130. Per-tool arithmetic 6+9+64+38+10+3=130 VERIFIED. Per-subsystem row-sum 55+5+14+17+14+5+14+1+5=130 VERIFIED. F4-provisional cap-formula bounds noted on VP-116/117/123/130 per ADR-051 §D2. |
| v1.16 | 2026-09-02 | architect | S-15.03 VP registration POLICY 9 propagation (pre-PR spec-package completion): VP-109..VP-113 (unit-test; BC-10.13.001 §PC3/§PC4/§PC5/§PC7/Inv2/Inv3/Inv4; anchor S-15.03) added as a NEW SS-10 module table (crates/last-amended-migrate — first VPs anchored to this module in this document); VP-114 (unit-test; BC-5.45.001 §PC1/§PC2/Inv4; anchor S-15.03) added to SS-05 module table; VP-115 (unit-test; BC-4.18.001 §PC1/§PC2/§PC3; anchor S-15.03) added to SS-04 module table. SS-04 subtotal U 5→6, Row 16→17. SS-05 subtotal U 0→1, Row 13→14. New SS-10 subtotal U=5, Row=5. Grand Total U 53→60, Total 108→115. Per-tool arithmetic 5+5+60+34+10+1=115 VERIFIED. Per-subsystem row-sum 42+5+14+17+14+5+12+1+5=115 VERIFIED. subsystems_affected += SS-10. |
| v1.15 | 2026-09-01 | architect | F-P11-001 HIGH fix (S-25.01 pass 11 adversarial review; POLICY 9 propagation + POLICY 4 mis-anchor): SS-01 module table VP-108 row title corrected from stale 'marker.cleared Audited-Clear Event — Clear Path Emission Correctness' to SoT-derived 'Marker Lifecycle Audited Events — Write and Clear Path Emission Correctness' (VP-108.md v1.5 H1). BC-anchor parenthetical corrected to BC-1.18.001 §PC4, BC-1.18.003 §PC1/PC3/PC4/PC5, BC-3.08.001 Events 9-10; ADR-048 §D4 (previously omitted write-path BC-1.18.001 §PC4 and BC-3.08.001 Event 10). Tool (U/unit-test), subsystem (SS-01), and all other columns unchanged. No VP count or arithmetic change — per-tool arithmetic 5+5+53+34+10+1=108 VERIFIED; per-subsystem row-sum 42+5+14+16+13+5+12+1=108 VERIFIED. |
| v1.14 | 2026-08-31 | architect | ADR-048 v1.1 POLICY 9 propagation: VP-108 (marker.cleared Audited-Clear Event; BC-1.18.003 PC1/PC3/PC4 + BC-3.08.001 Event 9; ADR-048 §D4; S-25.01) added to SS-01 module table (U; unit-test). VP-107 title updated to T1-only scope per ADR-048 §D3 v1.1 amendment. SS-01 subtotal U 27→28, Row 41→42. Grand Total U 52→53, Total 107→108. §3 Grand Totals updated; §SS-01 row detail updated; arithmetic invariant 107→108. Per-tool arithmetic 5+5+53+34+10+1=108 VERIFIED. Per-subsystem row-sum 42+5+14+16+13+5+12+1=108 VERIFIED. input-hash 7dd067b→c564dd1. |
| v1.13 | 2026-08-31 | architect | S-25.01 POLICY 9 propagation: VP-107 (Ungated-Escape Invariant; ADR-048 §D3; BC-1.18.002 §INV6; anchor S-25.01) added to SS-01 module table (U; unit-test). SS-01 subtotal U 26→27, Row 40→41. Grand Total U 51→52, Total 106→107. §3 Grand Totals updated; §SS-01 row detail updated. Per-tool arithmetic 5+5+52+34+10+1=107 VERIFIED. Per-subsystem row-sum 41+5+14+16+13+5+12+1=107 VERIFIED. input-hash updated (VP-INDEX v2.91 drift). |
| v1.12 | 2026-08-30 | architect | S-25.01 re-audit LOW-1: VP-105 SS-04 row title corrected to VP-105.md H1 verbatim — removed stray 'validate-unvalidated-mutation-marker ' prefix; semicolon changed to comma ('...Exists; Passes' → '...Exists, Passes'). total_vps UNCHANGED 106. input-hash 0e8390c→1f93bde. |
| v1.11 | 2026-08-30 | architect | F2 POLICY 9 consistency-audit fix: VP-105 SS-04 row title corrected to v1.1 verbatim ('...Agent Dispatch and git commit/push While Marker Exists...'); BC source corrected from 'BC-1.18.002 PC1+PC3' to 'BC-1.18.002 PC1+PC2+PC3+PC4'. total_vps UNCHANGED 106. |
| v1.10 | 2026-08-30 | architect | E-25 F2 validation-integrity-layer1 POLICY 9 propagation: VP-102/103/104/106 added to SS-01 module table (U; unit-test; BC-1.18.001 — INDETERMINATE outcome model, durable marker, fail-closed classification); VP-105 added to SS-04 module table (I; integration; BC-1.18.002 — next-advance gate). SS-01 subtotal U 22→26 (+4), Row 36→40. SS-04 subtotal I 6→7 (+1), Row 15→16. Grand Total U 47→51, I 33→34, Total 101→106. §3 Grand Totals table updated; §SS-01 row detail updated; arithmetic invariant note 101→106. Per-tool arithmetic 5+5+51+34+10+1=106 VERIFIED. Per-subsystem row-sum 40+5+14+16+13+5+12+1=106 VERIFIED. input-hash d1075c2→cfb38c7. |
| v1.9 | 2026-07-29 | state-manager | D-943 S-21.04 pass-28 POLICY 9 propagation: VP-097 SS-01 row title corrected 'Cannot Escape' → 'Cannot Resolve Outside' (verbatim VP-097.md title since v1.0; POLICY 9 no-op: proof_method kani-proof unchanged). input-hash 015260d→16cc408. |
| v1.8 | 2026-07-16 | architect | F-P2-001 POLICY 9 re-propagation: VP-095 SS-04 catalog row title corrected to v1.3 H1 verbatim form ('verify-factory-lock Never Receives output_too_large for Any STATE.md Size — Structural Guarantee via host::read_prefix(262144) (BC-1.17.001 PC-3); Large-STATE.md Frontmatter Correctly Parsed from 262144-Byte Prefix'); v1.7 propagated to VP-095 v1.2 Phase-B title (8192) which VP-095 v1.3 superseded same-burst 2026-07-16. §3 SS-04 data row corrected U 4→5 I 7→6 (pre-existing omission from v1.7 partial update; §1 subtotal was already correct at U=5 I=6). Per-tool arithmetic 5+5+47+33+10+1=101 VERIFIED. Per-subsystem row-sum 36+5+14+15+13+5+12+1=101 VERIFIED. input-hash f0fab9c→5279415. |
| v1.7 | 2026-07-16 | architect | S-19.07 Phase-B POLICY 9 propagation: VP-095 SS-04 row title updated to Phase-B form ('verify-factory-lock Never Receives output_too_large for Any STATE.md Size — Structural Guarantee via host::read_prefix(8192) (BC-1.17.001 PC-3); Large-STATE.md Frontmatter Correctly Parsed from 8192-Byte Prefix'); ✓ moved from I column to U column (proof_method integration→unit+static per VP-095 v1.2 amendment). SS-04 subtotal U 4→5, I 7→6, row total 15 unchanged. Grand Total U 46→47, I 34→33, total 101 unchanged. Per-tool arithmetic 5+5+47+33+10+1=101 VERIFIED. Per-subsystem row-sum 36+5+14+15+13+5+12+1=101 VERIFIED. input-hash c9ec678→f0fab9c. |
| v1.6 | 2026-07-13 | architect | pass-11 F-P11-001 POLICY 9 propagation: VP-099 SS-07 row title updated to both-ends form ('Starts With ^' → 'Is Fully Anchored (^...$)'). input-hash 893a501→c9ec678. |
| v1.5 | 2026-07-08 | architect | E-19 pass-28 VP-096 boundary-wording sync: VP-096 SS-04 row title updated to exclusive form per BC-4.13.001 §Invariant 9 adjudication (D-781) — 'Byte-Exact Prefix Through Second --- Delimiter' → 'Byte-Exact Prefix Up To (Excluding) Second --- Delimiter Line'. input-hash 7a7ac8c→893a501. |
| v1.4 | 2026-07-06 | architect | E-19 VP package POLICY 9 propagation: VP-094 added to SS-05 (I; BC-5.42.001; S-19.01); VP-095/096 added to SS-04 (I + P; BC-4.13.001; S-19.02); VP-097/098/100/101 added to SS-01 (K + I + I + I; BC-2.07.001+BC-2.02.011 + BC-2.07.001 + BC-3.08.001+DI-019 + BC-1.17.001); VP-099 added to SS-07 (I; no BC; S-19.04). All 8 abbreviated titles corrected from prior placeholder values. SS-01 subtotal K 2→3, I 7→10, row 32→36. SS-04 subtotal P 1→2, I 6→7, row 13→15. SS-05 subtotal I 5→6, row 12→13. SS-07 subtotal I 4→5, row 11→12. Grand Total K 4→5, P 4→5, I 28→34, total 93→101. Per-tool arithmetic 5+5+46+34+10+1=101 VERIFIED. Per-subsystem row-sum 36+5+14+15+13+5+12+1=101 VERIFIED. §2 VP-100 judgment call added (SS-01 primary; SS-03 secondary). |
| v1.3 | 2026-06-24 | architect | S-18.04b-prereq POLICY 9 propagation: VP-093 added to SS-01 module table (integration; SS-01; DI-020, DI-025; BC-1.16.001 — dispatcher git_context injection on PostToolUse Bash git-commit events; four-field injection; fail-open on git error; no injection on non-qualifying events; exec-free WASM boundary; HOST_ABI_VERSION unchanged; anchor S-18.04b-prereq). SS-01 subtotal I 6→7, row total 31→32. Grand Total I 27→28, row total 92→93. Per-tool arithmetic 4+4+46+28+10+1=93 VERIFIED. Per-subsystem row-sum 32+5+14+13+12+5+11+1=93 VERIFIED. |
| v1.2 | 2026-06-16 | state-manager | D-615 POLICY 9 propagation: VP-092 added to SS-06 module table (unit-test; SS-06; DI-020; BC-6.25.001 — check-state-health CLAUDE_AUTOCOMPACT_PCT_OVERRIDE advisory check; never blocks; PC1 absent→ADVISORY; PC2 >80→ADVISORY; PC3 <=80→PASS). SS-06 subtotal U 0→1, row total 4→5. Grand Total U 45→46, row total 91→92. Per-tool arithmetic 4+4+46+27+10+1=92 VERIFIED. Per-subsystem row-sum 31+5+14+13+12+5+11+1=92 VERIFIED. |
| v1.1 | 2026-06-16 | state-manager | D-612 POLICY 9 propagation: VP-091 added to SS-04 module table (unit-test; SS-04; DI-020; BC-4.15.001 — validate-heavy-op-delegation always-Continue advisory gate). SS-04 subtotal U 3→4, row total 12→13. Grand Total U 44→45, row total 90→91. Per-tool arithmetic 4+4+45+27+10+1=91 VERIFIED. Per-subsystem row-sum 31+5+14+13+12+4+11+1=91 VERIFIED. |
| v1.0 | 2026-06-16 | architect | Initial creation — F2 gate decision. Sources: VP-INDEX.md v2.29 (86 VPs) + VP-087..VP-090 (4 new E-18 VPs). POST-INTEGRATION totals: total_vps=90, unit-test=44, integration=27, manual=10, static-check=1, kani-proof=4, proptest=4. |

---

## §1 Coverage by Module (VP-to-Module Table)

Column key:
- **K** = kani-proof
- **P** = proptest
- **U** = unit-test
- **I** = integration
- **M** = manual
- **S** = static-check

---

### Module: crates/factory-dispatcher (SS-01 — Hook Dispatcher Core)

VPs whose primary subsystem is SS-01. Includes multi-subsystem VPs where SS-01 is
first-listed (VP-007, VP-008, VP-009, VP-026, VP-051, VP-073, VP-074, VP-075, VP-077,
VP-086, VP-100 per assignment notes in §2 below).

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-001 | Tier Execution Is Sequential; Intra-Tier Is Parallel | SS-01 | | | | ✓ | | |
| VP-002 | Plugin Crash or Timeout Does Not Block Sibling Plugins | SS-01 | | | | ✓ | | |
| VP-003 | block_intent Is Aggregate; Tier Runs to Completion | SS-01 | | | ✓ | | | |
| VP-004 | Capability Denial Produces Return Code AND Audit Event | SS-01 | | | ✓ | | | |
| VP-005 | Shell Interpreters Require Explicit shell_bypass_acknowledged | SS-01 | | | ✓ | | | |
| VP-006 | Setuid/Setgid Binaries Refused Unconditionally | SS-01 | | | ✓ | | | |
| VP-007 | Dispatcher Self-Telemetry Is Always-On and Never Panics | SS-01, SS-03 | | | ✓ | | | |
| VP-008 | Internal Log Filename Derived from Event Timestamp | SS-01, SS-03 | | | ✓ | | | |
| VP-009 | prune_old Removes Only Dispatcher-Internal Files | SS-01, SS-03 | | | ✓ | | | |
| VP-010 | Plugin Stderr Capped at 4 KiB with Truncation Marker | SS-01 | | | ✓ | | | |
| VP-014 | Schema Version Mismatch Is a Hard Load Error | SS-01 | | | ✓ | | | |
| VP-016 | Each Registry Entry Sees Only Its Own plugin_config | SS-01 | | | ✓ | | | |
| VP-017 | dispatcher_trace_id Present on Every Emitted Event | SS-01 | | | ✓ | | | |
| VP-018 | Registry Rejects Malformed Configurations at Load Time | SS-01 | | | ✓ | | | |
| VP-019 | Routing Is Deterministic — Same Input Yields Same Plugin Selection | SS-01 | | | ✓ | | | |
| VP-020 | Epoch Timeout Rounds Up and Terminates Infinite Loops | SS-01 | | | ✓ | | | |
| VP-021 | Capability Deny-by-Default | SS-01 | | | ✓ | | | |
| VP-022 | Dispatcher Exit Code Semantics — 0 Non-Block, 2 Block | SS-01 | | | ✓ | | | |
| VP-023 | Wire Format Decoders Reject Truncated Input Without Panic | SS-01, SS-02 | | | ✓ | | | |
| VP-024 | Plugin Cache Is Keyed by Path and Invalidated by mtime | SS-01 | | | ✓ | | | |
| VP-025 | Host Function ABI Surface Is Complete and Stable | SS-01, SS-02 | | | | ✓ | | |
| VP-026 | InternalEvent Serializes Flat with No Null Optional Fields | SS-01, SS-03 | | | ✓ | | | |
| VP-027 | HookPayload Parsing Is Robust for All Envelope Types | SS-01 | | | ✓ | | | |
| VP-050 | exec_subprocess Timeout Is Enforced — Hung Commands Are Killed | SS-01 | | | ✓ | | | |
| VP-051 | Dispatcher Startup Flow Writes Parseable JSONL | SS-01, SS-03 | | | | ✓ | | |
| VP-052 | Epoch Ticker Shuts Down Cooperatively and Idempotently | SS-01 | | | ✓ | | | |
| VP-073 | Resolver-Load Purity | SS-01, SS-04 | | | | ✓ | | |
| VP-074 | Resolver-Error Isolation | SS-01, SS-04 | ✓ | | | | | |
| VP-075 | Context-Injection Determinism | SS-01, SS-04 | | ✓ | | | | |
| VP-077 | Dispatcher Partition Correctness (6 properties) | SS-01 | ✓ | | | | | |
| VP-086 | Dispatcher Exit-2 Propagation for PreCompact Block-Intent | SS-01, SS-04 | | | | ✓ | | |
| VP-093 | Dispatcher Injects git_context Into payload.extra on PostToolUse Bash git-commit Events; Fail-Open on Git Error | SS-01 | | | | ✓ | | |
| VP-097 | path_util::resolve_path_for_allowlist Traversal Defense — .. Sequences Cannot Resolve Outside Allowlist Prefixes | SS-01 | ✓ | | | | | |
| VP-098 | Allowlisted-but-Absent File Returns NOT_FOUND (-5); Zero CAPABILITY_DENIED False-Positives | SS-01 | | | | ✓ | | |
| VP-100 | Drain-Timer Expiry Emits Exactly One plugin.abandoned Per In-Flight; No plugin.completed Follows for Same Trace | SS-01, SS-03 | | | | ✓ | | |
| VP-101 | host::read_prefix Returns Byte-Exact Prefix; Never OUTPUT_TOO_LARGE; Absent Returns NOT_FOUND (-5) | SS-01 | | | | ✓ | | |
| VP-102 | Fuel-Exhaustion and Epoch-Timeout Yield INDETERMINATE Outcome for fail-closed Plugin | SS-01 | | | ✓ | | | |
| VP-103 | Host OutputTooLarge Then Plugin Ok(exit:0) Yields INDETERMINATE for fail-closed Plugin | SS-01 | | | ✓ | | | |
| VP-104 | INDETERMINATE for fail-closed Plugin Writes Unvalidated-Mutation Marker with Required Fields | SS-01 | | | ✓ | | | |
| VP-106 | Successful Re-Validation Deletes Marker; fail-open INDETERMINATE Writes No Marker | SS-01 | | | ✓ | | | |
| VP-107 | Ungated-Escape Invariant: Edit/Write Re-Validation Dispatch Is Not Matched by Either Gate Arm (T1 Primary Recovery; BC-1.18.002 §INV6; ADR-048 §D3) | SS-01 | | | ✓ | | | |
| VP-108 | Marker Lifecycle Audited Events — Write and Clear Path Emission Correctness (BC-1.18.001 §PC4, BC-1.18.003 §PC1/PC3/PC4/PC5, BC-3.08.001 Events 9-10; ADR-048 §D4) | SS-01 | | | ✓ | | | |
| VP-116 | Shard-Cap Comparison Arithmetic Overflow-Safe + Boundary-Inclusive (BC-1.18.005; cap PROVISIONAL-until-F4) | SS-01 | ✓ | | | | | |
| VP-117 | Native Trigger — Zero-Cost Bypass, Cross-Validator Minimum Rule, Byte Denomination (BC-1.18.005; cap NUMBERS PROVISIONAL-until-F4) | SS-01 | | | ✓ | | | |
| VP-118 | Staged Four-Step Roll (Read→Publish Sealed Copy→Atomic-Truncate Canonical In-Place→Append Index) Before Block; Same-Invocation Atomicity; Append-Only-Tail Index (BC-1.18.006) | SS-01 | | | | ✓ | | |
| VP-119 | No Sealed Shard Over Recorded Cap; Canonical Filename Never Renamed Away; Sealed-Shard Write-Once/Immutability — `seq` Collision Fails Loud as `E-SHD-009`, Never a Silent Overwrite (EXTENDED v1.8: NEW Postcondition 8, EC-024) (BC-1.18.006) | SS-01 | | ✓ | | | | |
| VP-120 | Unified Tool-Independent Retry Template; Shard-Seal-Write Failure (Steps a-b) Fails Loud E-SHD-001; Two-Template Determinism — DISTINCT Empty-Canonical Template Per Over-Cap Case (EC-021) (BC-1.18.006) | SS-01 | | | ✓ | | | |
| VP-121 | Retention Honest O(Active) — Bounded Active Count + No Index-Entry Loss (BC-1.18.007) | SS-01 | | ✓ | | | | |
| VP-122 | Default Glob Excludes Archived Shards; Missing/Corrupt Index Fails Loud (BC-1.18.007) | SS-01 | | | ✓ | | | |
| VP-123 | Backfill-Split Content Preservation — Byte-for-Byte + Every Record in One Shard + Per-Shard Cap Bound Unless Flagged Oversized (BC-1.18.008; cap PROVISIONAL-until-F4) | SS-01 | | ✓ | | | | |
| VP-124 | Backfill-Split Atomic Under Interruption + Idempotent + Recovery-Confirmation Exact-Manifest-Based, Never a Byte-Prefix Heuristic (EXTENDED v1.6: NEW Backfill Recovery Manifest §PC3, NEW Recovery-Confirmation Rule §PC5, tightened §INV3, EC-009/EC-010, `E-SHD-011`) + Heal Slice Manifest-Derived and Manifest-Verified Before Every Write (EXTENDED v1.7: NEW Manifest-Authoritative Slice-and-Verify Rule §PC5, disk-read-back scope extended §PC6(c)/§INV4, EC-011, `E-SHD-012`) + Happy-Path Canonical-Truncate Write Receives the SAME Post-Hoc Disk Read-Back Verification (EXTENDED v1.8: NEW §PC5 step (ii) happy-path read-back extension, NEW §PC6(c) Summary paragraph, NEW §INV5, EC-013, `E-SHD-013`) (BC-1.18.008) | SS-01 | | | | ✓ | | |
| VP-125 | BC-INDEX changelog: Rotation Bounds Live Sequence at N With Zero History Loss (BC-1.18.009) | SS-01 | | ✓ | | | | |
| VP-126 | BC-INDEX changelog: Gate Rotation Reuses rotate_changelog ONLY — No Reimplemented Logic + Zero Gate-Side prepend_changelog_item Call Sites (BC-1.18.009) | SS-01 | | | | | | ✓ |
| VP-127 | BC-INDEX First-Level Addressing Zero-Lookup — Pure Function of BC-ID Prefix (BC-1.18.010) | SS-01 | | | ✓ | | | |
| VP-128 | BC-INDEX Second-Level Manifest-Keyed + Single-Authoritative-Row + ARCH-INDEX-Sourced Mapping (BC-1.18.010) | SS-01 | | | | ✓ | | |
| VP-131 | BC-INDEX changelog: rotate_changelog Failure Fails Loud as HookResult::Error (E-SHD-004); Pre-Rotation State Preserved (BC-1.18.009) | SS-01 | | | ✓ | | | |
| VP-132 | B2 BC-INDEX Body Split Content Preservation — Per-Subsystem Shards + Retained Body Reproduce Original Byte-for-Byte (BC-1.18.011; cap PROVISIONAL-until-F4) | SS-01 | | ✓ | | | | |
| VP-133 | B2 Independent-Census + Crash-Atomicity + Fail-Loud Rollback (E-SHD-005) + Idempotency + SS-05/SS-06 Sub-Split Census (BC-1.18.011) | SS-01 | | | | ✓ | | |
| VP-134 | B2 Migration Adds No New Cohort-B Sequencing Dependency — BC-7.08.001 Flip Gating Never Cites BC-1.18.011 (BC-1.18.011) | SS-01 | | | | | | ✓ |
| VP-135 | B1 Backfill Content Preservation — Every Migrated changelog: Item Byte-for-Byte in the Single Evergreen Archive; Concat Reproduces Original (BC-1.18.012; N PROVISIONAL-until-F4) | SS-01 | | ✓ | | | | |
| VP-136 | B1 Backfill Independent-Census + Fail-Loud Rollback (E-SHD-003) + Idempotency (BC-1.18.012) | SS-01 | | | | ✓ | | |
| VP-137 | B1 Backfill Adds No New Cohort-B / No Cross-Migration Dependency (BC-1.18.012) | SS-01 | | | | | | ✓ |
| VP-138 | Staged Roll Self-Heals Truncate-After-Seal Crash (E-SHD-006) — Resume-From-Truncate, Idempotent, No Data Loss; 0-Byte-Orphan Guard (EC-022) (BC-1.18.006) | SS-01 | | | | ✓ | | |
| VP-139 | Staged Roll Self-Heals Index-After-Truncate Crash (E-SHD-007) — Append-Only Index Reconciliation, No Data Loss; 0-Byte-Orphan Guard (EC-022) (BC-1.18.006) | SS-01 | | | | ✓ | | |
| VP-140 | Item-Count Trigger Off-By-One (`current_item_count + 1 > N`) + `low_water_mark` Config Validation (Default `floor(N/2)`; Fail-Loud on `>= N`/Negative) + Amortization Advisory (`tracing::warn!` IFF `low_water_mark > floor(N/2)`; load always succeeds, EC-012) — dedicated PC8 item-count-trigger VP (BC-1.18.005) | SS-01 | | | ✓ | | | |
| VP-141 | POLICY-1 (`append_only_numbering`) Audit Scope Always Archive-Inclusive for a Layer-2-Sharded Artifact — effective glob ALWAYS = `<stem>*.md` UNION `archive/<artifact-stem>/<stem>*.md`, never silently excluding an archived-only ID (archived-only `D-042` + reuse → violation); integration/fixture + static/compliance (`consistency-validator`/adversary-prompt + POLICY-1 `verification_steps`, `lint_hook: null`). POLICY-1 carve-out complement of VP-122 (BC-1.18.007 PC6/EC-006) | SS-01 | | | | ✓ | | |
| **SS-01 subtotal** | | | **4** | **7** | **34** | **18** | **0** | **3** |

---

### Module: crates/hook-sdk (SS-02 — Hook SDK and Plugin ABI)

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-038 | SDK HookResult Exit Codes Are Stable | SS-02 | | | ✓ | | | |
| VP-039 | SDK Wire Format Encoding Is Symmetric | SS-02 | | | ✓ | | | |
| VP-040 | SDK HookPayload Round-Trips via Serde | SS-02 | | | ✓ | | | |
| VP-041 | SDK Panic Handler Extracts Message | SS-02 | | | ✓ | | | |
| VP-042 | SDK HostError Code Mapping Is Stable | SS-02 | | | ✓ | | | |
| **SS-02 subtotal** | | | **0** | **0** | **5** | **0** | **0** | **0** |

---

### Module: crates/sink-core, crates/sink-file (SS-03 — Event Emission)

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-011 | Sink submit Must Not Block the Dispatcher | SS-03 | | | ✓ | | | |
| VP-012 | Sink Failure Affects Only That Sink | SS-03 | | | ✓ | | | |
| VP-013 | Unknown Sink Driver Types Are Non-Fatal | SS-03 | | | ✓ | | | |
| VP-028 | Sink Fan-Out — Every Event Reaches Every Accepting Sink | SS-03 | | | | ✓ | | |
| VP-029 | File Sink Path Template Substitutes {date}, {name}, {project} | SS-03 | | | ✓ | | | |
| VP-030 | Sink Shutdown Drains Queued Events | SS-03 | | | ✓ | | | |
| VP-031 | Tag Enrichment Does Not Overwrite Producer Fields | SS-03 | | | ✓ | | | |
| VP-032 | RoutingFilter Default Accepts All Events | SS-03 | | | ✓ | | | |
| VP-033 | OTLP LogRecord Mapping Is Correct | SS-03 | | | | ✓ | | |
| VP-034 | OTLP Sink Batch Trigger Thresholds Are Independent | SS-03 | | | ✓ | | | |
| VP-035 | File Sink Auto-Creates Missing Parent Directories | SS-03 | | | ✓ | | | |
| VP-036 | Disabled Sink Drops Every Event Without Writing | SS-03 | | | ✓ | | | |
| VP-037 | OTLP Resource Attributes — Operator Overrides Win | SS-03 | | | ✓ | | | |
| VP-079 | Async-Semantics Event Types — Payload Schema Conformance | SS-03 | | | | ✓ | | |
| **SS-03 subtotal** | | | **0** | **0** | **11** | **3** | **0** | **0** |

---

### Module: crates/hook-plugins/* (SS-04 — Plugin Ecosystem)

VPs where SS-04 is the first-listed subsystem. Does not include multi-subsystem VPs
where SS-01 or SS-05 is listed first (see assignment notes in §2).

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-044 | Legacy Bash Adapter Exit Code Mapping | SS-04, SS-07 | | | ✓ | | | |
| VP-045 | Legacy Bash Adapter Strips plugin_config | SS-04 | | | ✓ | | | |
| VP-065 | Session-Start Plugin Surface Invariant | SS-04 | | | | ✓ | | |
| VP-066 | Session-End Plugin Surface Invariant | SS-04 | | | | ✓ | | |
| VP-067 | Worktree Hook Plugin Surface Invariant | SS-04 | | | | ✓ | | |
| VP-068 | Tool-Failure Hook Plugin Surface Invariant | SS-04 | | | | ✓ | | |
| VP-069 | validate-artifact-path Registry-Load Purity | SS-04 | | ✓ | | | | |
| VP-070 | validate-artifact-path Path-Pattern Matching | SS-04 | ✓ | | | | | |
| VP-071 | validate-per-story-adversary-convergence Block Invariant | SS-04 | ✓ | | | | | |
| VP-072 | artifact-path-registry.yaml Single Source of Truth | SS-04 | | | | ✓ | | |
| VP-076 | Resolver-Capability Confinement | SS-04 | | | | ✓ | | |
| VP-083 | Completeness Gate Is No-Op on Wave-1 or Non-HANDOFF.md Writes | SS-04 | | | ✓ | | | |
| VP-091 | validate-heavy-op-delegation Emits DelegationRecommended Advisory (Never Blocks) | SS-04 | | | ✓ | | | |
| VP-095 | verify-factory-lock Never Receives output_too_large for Any STATE.md Size — Structural Guarantee via host::read_prefix(262144) (BC-1.17.001 PC-3); Large-STATE.md Frontmatter Correctly Parsed from 262144-Byte Prefix | SS-04 | | | ✓ | | | |
| VP-096 | extract_frontmatter Purity — Byte-Exact Prefix Up To (Excluding) Second --- Delimiter Line; Deterministic | SS-04 | | ✓ | | | | |
| VP-105 | Next-Advance Gate Blocks Agent Dispatch and git commit/push While Marker Exists, Passes When Absent (BC-1.18.002 PC1+PC2+PC3+PC4) | SS-04 | | | | ✓ | | |
| VP-115 | Bounded last_amended Byte Length Across Cumulative Bursts (Fuel-Relief Structural Proxy) (BC-4.18.001 §PC1/§PC2/§PC3) | SS-04 | | | ✓ | | | |
| **SS-04 subtotal** | | | **2** | **2** | **6** | **7** | **0** | **0** |

---

### Module: plugins/vsdd-factory/agents, workflows (SS-05 — Pipeline Orchestration)

VPs where SS-05 is the first-listed or primary subsystem.

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-053 | Lobster Workflow DAG Is Acyclic | SS-05 | | | | | ✓ | |
| VP-054 | Workflow Loop Blocks Are Bounded | SS-05 | | | | | ✓ | |
| VP-055 | state-manager Runs Last in Every Burst | SS-05 | | | | | ✓ | |
| VP-056 | on_failure Semantics — retry → escalate → abort | SS-05 | | | | | ✓ | |
| VP-057 | Adversarial Review Convergence | SS-05 | | | | | ✓ | |
| VP-061 | Agent Prompt Discipline Rules Present in All Three Agent Files | SS-05 | | | | | | ✓ |
| VP-062 | S-7.02 Process-Codification Surface Invariant | SS-05, SS-07, SS-08 | | | | ✓ | | |
| VP-063 | RED_RATIO computation correctness | SS-05 | | | | ✓ | | |
| VP-064 | facade-mode mutation gate enforcement | SS-05, SS-06 | | | | | ✓ | |
| VP-081 | Wave Cannot Close Without Verified Handoff (wave_id > 1) | SS-04, SS-05, SS-07 | | | | ✓ | | |
| VP-084 | PreCompact Flush Commit Is Lifecycle-Distinct | SS-05, SS-04 | | | | ✓ | | |
| VP-087 | wave-state.yaml Produced Atomically With HANDOFF.md | SS-05 | | | | ✓ | | |
| VP-094 | pr-manager READY-Verdict Covered-SHA Pin, Stale-Verdict Halt, Release-PR Merge-Strategy Enforcement | SS-05 | | | | ✓ | | |
| VP-114 | Exactly-One changelog: Prepend, Byte-for-Byte-Untouched Pre-Existing Items, and Never-Wrap last_amended (BC-5.45.001 §PC1/§PC2/Inv4) | SS-05 | | | ✓ | | | |
| **SS-05 subtotal** | | | **0** | **0** | **1** | **6** | **6** | **1** |

> **Assignment note (VP-081):** VP-081 lists scope SS-04, SS-05, SS-07. The primary
> owning subsystem is SS-05 (Pipeline Orchestration) because the behavioral contract
> is BC-5.41.001 — a wave-gate orchestration step. The WASM gate (SS-04) and shell
> script (SS-07) are components invoked by the orchestration step.
>
> **Assignment note (VP-084):** VP-084 lists scope SS-05, SS-04. The lifecycle-
> distinctness invariant (BC-5.41.003) governs when the MULTI_COMMIT_CHAIN_NOT_ALLOWED
> detector is suppressed — an orchestration policy. SS-05 is primary.

---

### Module: plugins/vsdd-factory/skills (SS-06 — Skill Catalog)

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-058 | create-adr Atomicity | SS-06 | | | | ✓ | | |
| VP-059 | ID Monotonicity — Allocated ADR-NNN | SS-06 | | ✓ | | | | |
| VP-060 | Bidirectional Supersession | SS-06 | | | | ✓ | | |
| VP-088 | rehydrate-wave Reads wave-state.yaml From Git | SS-06 | | | | ✓ | | |
| VP-092 | check-state-health CLAUDE_AUTOCOMPACT_PCT_OVERRIDE Advisory Check (Never Blocks) | SS-06 | | | ✓ | | | |
| **SS-06 subtotal** | | | **0** | **1** | **1** | **3** | **0** | **0** |

---

### Module: plugins/vsdd-factory/hooks/*.sh, hooks-registry.toml (SS-07 — Hook Bash Layer)

VPs where SS-07 is the first-listed subsystem. Note VP-043 lists SS-07, SS-01 —
SS-07 is primary because the property tests the registry file (hooks-registry.toml),
not the dispatcher routing engine.

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-043 | Every hooks-registry.toml Entry Routes Through legacy-bash-adapter.wasm | SS-07, SS-01 | | | | ✓ | | |
| VP-046 | All hooks-registry.toml Entries Correspond to Registered Hook Scripts | SS-07 | | | | | ✓ | |
| VP-047 | Validator Hooks Exit 0 or 2 — No Other Codes | SS-07 | | | | | ✓ | |
| VP-048 | protect-secrets.sh Fails Closed When jq Is Missing | SS-07 | | | | | ✓ | |
| VP-049 | Generated hooks-registry.toml Round-Trips Through Registry::load | SS-07, SS-09 | | | | ✓ | | |
| VP-078 | CI Lint Invariant — on_error=block implies async=false | SS-07, SS-01 | | | | ✓ | | |
| VP-080 | block-ai-attribution PostToolUse arm: detect_attribution | SS-07 | | ✓ | | | | |
| VP-082 | PreCompact Flush Commits to factory-artifacts | SS-07, SS-04 | | | | ✓ | | |
| VP-085 | PreCompact Flush Hook Is Hermetic | SS-07 | | | ✓ | | | |
| VP-089 | postcompact-reanchor.sh Emits Re-Anchor Block | SS-07 | | | ✓ | | | |
| VP-090 | precompact-flush-log Pruning | SS-07 | | | ✓ | | | |
| VP-099 | hooks-registry Tool-Filter Anchoring Invariant — Every tool= Value Is Fully Anchored (^...$) or Carries # intent: Comment | SS-07 | | | | ✓ | | |
| VP-129 | Cohort B Flip Never Fail-Closes validate-burst-log ^Bash$ Arm; Adds Exactly Three fail-closed Entries (Closed Cohort) (BC-7.08.001) | SS-07 | | | | | | ✓ |
| VP-130 | Cohort B Flip Sequenced After BC-1.18.008 Backfill-Split; Per-Validator Calibration Evidence (BC-7.08.001; fuel ceiling PROVISIONAL-until-F4) | SS-07 | | | | ✓ | | |
| **SS-07 subtotal** | | | **0** | **1** | **3** | **6** | **3** | **1** |

---

### Module: plugins/vsdd-factory/.claude-plugin, hooks.json (SS-09 — Configuration and Activation)

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-015 | Per-Project Activation Required Before Dispatcher Can Run | SS-09 | | | | | ✓ | |
| **SS-09 subtotal** | | | **0** | **0** | **0** | **0** | **1** | **0** |

---

### Module: crates/last-amended-migrate (SS-10 — CLI Tools and Bin)

VPs where SS-10 is the primary subsystem. First module table addition for SS-10
(S-15.03 — `last_amended`/`changelog:` migration, full-recovery split, rotation tooling).

| VP ID | Title (abbreviated) | Subsystems | K | P | U | I | M | S |
|-------|---------------------|-----------|---|---|---|---|---|---|
| VP-109 | Full-Recovery Split Recovers Every Chained Entry With Zero Data Loss and Correct Newest-First Ordering (BC-10.13.001 §PC7) | SS-10 | | | ✓ | | | |
| VP-110 | Full-Recovery Split Completes in Bounded Time/Memory for Arbitrarily Long Input (BC-10.13.001 Inv3/§PC7 step 7) | SS-10 | | | ✓ | | | |
| VP-111 | Migration Subcommand Is Idempotent, Including Immediately After a PC7 Full-Recovery Split (BC-10.13.001 §PC4/Inv2) | SS-10 | | | ✓ | | | |
| VP-112 | Rotation Archives the changelog: Sequence Without Data Loss (BC-10.13.001 §PC5) | SS-10 | | | ✓ | | | |
| VP-113 | D-1144 Escape Remediation Produces Strictly-Valid YAML (BC-10.13.001 §PC3/Inv4) | SS-10 | | | ✓ | | | |
| **SS-10 subtotal** | | | **0** | **0** | **5** | **0** | **0** | **0** |

---

## §2 Module Assignment Judgment Calls and Coverage Gaps

The following VP assignments required explicit judgment due to multi-subsystem scope.
All decisions defer to VP-INDEX.md §Full Index Scope column as authoritative; the
rationale below documents the reasoning applied when scope order determines primary.

| VP | Scope Column | Primary Assignment | Rationale |
|----|-------------|-------------------|-----------|
| VP-007 | SS-01, SS-03 | SS-01 | Dispatcher self-telemetry invariant; unit-test exercises dispatcher emit path |
| VP-008 | SS-01, SS-03 | SS-01 | Internal log filename logic lives in dispatcher crate |
| VP-009 | SS-01, SS-03 | SS-01 | prune_old is a dispatcher-crate function |
| VP-023 | SS-01, SS-02 | SS-01 | Wire format decoder is tested at dispatcher boundary |
| VP-025 | SS-01, SS-02 | SS-01 | Host ABI completeness is a dispatcher guarantee |
| VP-026 | SS-01, SS-03 | SS-01 | InternalEvent is a dispatcher-core struct |
| VP-043 | SS-07, SS-01 | SS-07 | Property tests hooks-registry.toml (SS-07 file), not dispatcher routing engine |
| VP-044 | SS-04, SS-07 | SS-04 | legacy-bash-adapter WASM lives in crates/hook-plugins/ (SS-04 territory) |
| VP-049 | SS-07, SS-09 | SS-07 | Property tests the generated TOML file structure (SS-07 owns the file) |
| VP-051 | SS-01, SS-03 | SS-01 | Dispatcher startup sequence is the module under test |
| VP-062 | SS-05, SS-07, SS-08 | SS-05 | Process-codification artifact is an orchestration pipeline invariant |
| VP-064 | SS-05, SS-06 | SS-05 | facade-mode mutation gate enforcement is a pipeline orchestration step |
| VP-073 | SS-01, SS-04 | SS-01 | Resolver-load purity: the dispatcher loads resolver modules; dispatcher is subject |
| VP-074 | SS-01, SS-04 | SS-01 | Kani proof targets dispatcher process boundary (error isolation) |
| VP-075 | SS-01, SS-04 | SS-01 | Context-injection determinism at dispatcher boundary |
| VP-078 | SS-07, SS-01 | SS-07 | CI lint invariant checks hooks-registry.toml; SS-07 owns the file |
| VP-080 | SS-07 | SS-07 | VP-INDEX §Full Index lists SS-07; behavioral arm VP follows hook-layer convention |
| VP-081 | SS-04, SS-05, SS-07 | SS-05 | Primary contract is BC-5.41.001 wave-gate orchestration step (SS-05) |
| VP-082 | SS-07, SS-04 | SS-07 | precompact-flush.sh is a SS-07 shell script |
| VP-084 | SS-05, SS-04 | SS-05 | Lifecycle-distinctness invariant (BC-5.41.003) is an orchestration policy |
| VP-086 | SS-01, SS-04 | SS-01 | Dispatcher exit-2 propagation: dispatcher binary is the module under test |
| VP-100 | SS-01, SS-03 | SS-01 | Drain-timer expiry is a dispatcher-core async-drain behavior (SS-01 primary); async-semantics event schema (SS-03) is secondary — the plugin.abandoned event must conform to BC-3.08.001 schema, but the causal mechanism is dispatcher drain-timer |

---

## §3 Grand Totals — Summary by Method

Each VP counted exactly once in the row for its primary subsystem. The grand-total
per-tool column sums equal 141 (total_vps POST-INTEGRATION).

| Subsystem | K | P | U | I | M | S | Row Total |
|-----------|---|---|---|---|---|---|-----------|
| SS-01 | 4 | 7 | 34 | 18 | 0 | 3 | 66 |
| SS-02 | 0 | 0 | 5 | 0 | 0 | 0 | 5 |
| SS-03 | 0 | 0 | 11 | 3 | 0 | 0 | 14 |
| SS-04 | 2 | 2 | 6 | 7 | 0 | 0 | 17 |
| SS-05 | 0 | 0 | 1 | 6 | 6 | 1 | 14 |
| SS-06 | 0 | 1 | 1 | 3 | 0 | 0 | 5 |
| SS-07 | 0 | 1 | 3 | 6 | 3 | 1 | 14 |
| SS-09 | 0 | 0 | 0 | 0 | 1 | 0 | 1 |
| SS-10 | 0 | 0 | 5 | 0 | 0 | 0 | 5 |
| **Grand Total** | **6** | **11** | **66** | **43** | **10** | **5** | **141** |

**Per-tool arithmetic check:** 6 + 11 + 66 + 43 + 10 + 5 = **141** ✓

**Per-subsystem row-sum check:** 66 + 5 + 14 + 17 + 14 + 5 + 14 + 1 + 5 = **141** ✓

**Per-tool column matches VP-INDEX.md POST-INTEGRATION targets:**
- kani-proof: **6** ✓ (VP-070, VP-071, VP-074, VP-077, VP-097, VP-116)
- proptest: **11** ✓ (VP-059, VP-069, VP-075, VP-080, VP-096, VP-119, VP-121, VP-123, VP-125, VP-132, VP-135)
- unit-test: **66** ✓ (65 prior + VP-140)
- integration: **43** ✓ (42 prior + VP-141)
- manual: **10** ✓ (unchanged from VP-INDEX v2.29)
- static-check: **5** ✓ (VP-061 + VP-126 + VP-129 + VP-134 + VP-137)

**SS-01 row detail:** K=4 (VP-074, VP-077, VP-097, VP-116), P=7 (VP-075, VP-119, VP-121, VP-123,
VP-125, VP-132, VP-135), U=34 (VP-003..010, VP-014, VP-016..024, VP-026..027, VP-050, VP-052, VP-102, VP-103,
VP-104, VP-106, VP-107, VP-108, VP-117, VP-120, VP-122, VP-127, VP-131, VP-140), I=18 (VP-001, VP-002, VP-025,
VP-051, VP-073, VP-086, VP-093, VP-098, VP-100, VP-101, VP-118, VP-124, VP-128, VP-133, VP-136, VP-138,
VP-139, VP-141), M=0, S=3 (VP-126, VP-134, VP-137). Row sum = 4+7+34+18+0+3 = **66** ✓

All 141 VPs are accounted for with no omissions and no double-counts.

---

## §4 Domain Invariant Verification Map

Maps each active domain invariant to the VPs that directly verify it. VP assignment is
authoritative in VP-INDEX.md §Full Index (BC/Invariant Anchor column). DI descriptions
derive from `.factory/specs/domain-spec/invariants.md`.

| DI | Description (brief) | Verified By (VP IDs) | Priority |
|----|---------------------|----------------------|----------|
| DI-001 | Tiers execute sequentially; plugins within a tier execute in parallel | VP-001, VP-019, VP-020, VP-052 | P0 |
| DI-002 | A plugin crash or timeout does not block sibling plugins | VP-002, VP-020, VP-050, VP-074 | P0 |
| DI-003 | block_intent is aggregate; tier runs to completion regardless | VP-003, VP-044, VP-047 | P0 |
| DI-004 | Capability denial always produces a return code AND audit event | VP-004, VP-021, VP-023, VP-025, VP-076 | P0 |
| DI-005 | Shell interpreters require explicit shell_bypass_acknowledged | VP-005, VP-021 | P0 |
| DI-006 | Setuid/setgid binaries refused unconditionally on Unix | VP-006 | P0 |
| DI-007 | Dispatcher self-telemetry is always-on | VP-007, VP-035, VP-051 | P1 |
| DI-008 | Internal log filenames derived from event timestamps, not wall clock | VP-008, VP-029 | P1 |
| DI-009 | Internal logs pruned to 30 days at dispatcher start | VP-009 | P1 |
| DI-010 | Plugin stderr capped at 4 KiB with truncation marker | VP-010 | P1 |
| DI-011 | Sink submit must not block the dispatcher | VP-011, VP-028, VP-030, VP-032, VP-034 | P1 |
| DI-012 | A sink failure affects only that sink | VP-012, VP-028, VP-031, VP-037 | P1 |
| DI-013 | Unknown sink driver types are non-fatal | VP-013, VP-036 | P1 |
| DI-014 | Schema version mismatch is a hard load error | VP-014, VP-018, VP-022, VP-046, VP-049 | P0 |
| DI-015 | Per-project activation required before dispatcher can run | VP-015 | P1 |
| DI-016 | Each registry entry sees only its own plugin_config | VP-016, VP-043, VP-045 | P0 |
| DI-017 | trace_id present on every emitted event; wire-format exclusivity | VP-017, VP-026, VP-027, VP-033, VP-051, VP-079 | P1 |
| DI-018 | (not active — captured as KL-005) | — | — |
| DI-019 | ASYNC_DRAIN_WINDOW_MS = 100 ms (runtime constant) | VP-079, VP-100 | P1 |
| DI-020 | Wave/phase boundary transitions must not lose load-bearing pipeline state | VP-081, VP-082, VP-083, VP-084, VP-092, VP-093 | P0 |
| DI-021 | Handoff claims cross-checked against verifiable external ground truth | VP-081, VP-082, VP-085 | P0 |
| DI-022 | PreCompact flush derives state exclusively from durable persisted sources | VP-082, VP-085 | P0 |
| DI-023 | Wave/phase identity derives from real persisted substrate; no phantom fields | VP-081, VP-087, VP-088 | P0 |
| DI-024 | PostCompact re-anchor is best-effort; not in CAP-032 continuity-guarantee chain | VP-089 | P1 |
| DI-025 | PreCompact flush commits lifecycle-orthogonal to state-manager burst commits | VP-082, VP-084, VP-085, VP-090, VP-093 | P0 |
