// Test files use .expect()/.unwrap()/.panic!() for failure reporting.
#![allow(clippy::expect_used, clippy::unwrap_used, clippy::panic)]
//! BC-1.18.011 v1.9 Postcondition 6 / BC-1.18.010 v1.10 Postcondition 4 /
//! ADR-051 §Decision 18 regression coverage for the B2 second-level
//! sub-shard chunk-boundary algorithm (`chunk_subsystem_rows_into_sub_shards`)
//! and its integration into `run_bc_index_migration` (SS-05/SS-06-class
//! over-cap subsystems must actually be sub-split, with a sub-manifest
//! produced, within the SAME one-time migration operation).
//!
//! # BC-5.38.001 Red Gate discipline — GREEN (T-11 implemented)
//!
//! `chunk_subsystem_rows_into_sub_shards` was `todo!()` at RED-Gate
//! authorship time (D-1237 spec-closure; stub added alongside this test file
//! since no prior burst declared this symbol) — every unit test
//! (PC6_UNIT_*) below panicked against that stub, and the two integration
//! tests (PC6_INTEGRATION_*), which exercise `run_bc_index_migration`,
//! failed on assertion (missing sub-shard files / manifest) because that
//! function performed ONLY the first-level split (hardcoded
//! `sub_sharded: false, sub_manifest: None` for every subsystem, never
//! calling the chunker). T-11 implemented both the chunker and its
//! `run_bc_index_migration` call site (`sub_sharded: true` wiring for
//! genuinely over-cap subsystems); every assertion below is unchanged from
//! RED-Gate authorship and is now GREEN — this file now serves as the
//! regression guard for that implementation per ADR-051 §Decision 18 and the
//! cited BC postconditions.
//!
//! VP classification: VP-142 (proptest; chunk-boundary determinism and
//! correctness, hosted on BC-1.18.011 Postcondition 6, cross-referenced from
//! BC-1.18.010 Postcondition 4). The 5 unit tests below are example-based
//! coverage of VP-142's properties (determinism, every-row-in-exactly-one-
//! chunk, cap-inclusive boundary, lone-row/letter-exhaustion edge cases) —
//! per this story's established scope note (mirrored from the sibling
//! `bc_1_18_011_b2_migration_test.rs`), a full `proptest` harness generating
//! >=1000 randomized row sets is implementer's to add alongside the real
//! implementation, not authored here.
//!
//! # Genuine ambiguity flagged for implementer/architect (not guessed at)
//!
//! ADR-051 §Decision 18 item 3 explicitly states the exact sub-shard
//! preamble heading text (e.g. `### SS-05.a` vs. `### SS-05 (part a)`) is "a
//! product-owner wording call, not an architecture concern." This file's
//! unit tests therefore construct their OWN synthetic preamble strings
//! (passed directly as the `preamble: &str` argument) rather than asserting
//! any particular heading text, and the two integration tests assert only
//! STRUCTURAL properties of `run_bc_index_migration`'s output (sub-shard
//! file count, sub-manifest schema/range correctness, row-membership
//! completeness) — never a specific preamble/heading string. Also: §Decision
//! 18 item 1 says `run_bc_index_migration` must be threaded with
//! `shard_cap_bytes` "reading the SAME config entry the live
//! `shard_cap_precheck` gate reads" — the integration tests below assume
//! this is the existing `.factory/shard-config.toml` / `ShardRegistry`
//! mechanism (`artifact_stem = "BC-INDEX"`), since that is the ONLY
//! `[[shard]]`-config-sourced value this codebase has for this artifact
//! today and §Decision 18 item 1 explicitly forbids "a new formula" or "a
//! separately-calibrated migration-time cap" — flagging this as an inferred
//! (not BC-text-pinned) wiring detail for implementer to confirm.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use factory_dispatcher::shard_manager::{
    BcId, SubShardChunk, SubsystemPrefixEntry, SubsystemPrefixSnapshot,
    chunk_subsystem_rows_into_sub_shards, extract_and_sort_bc_rows, load_shard_manifest,
    load_sub_shard_manifest, resolve_bc_shard_path, run_bc_index_migration,
};

// ---------------------------------------------------------------------------
// Shared fixture helpers
// ---------------------------------------------------------------------------

fn bc_id(subsystem_major: u32, capability_minor: u32, sequence: u32) -> BcId {
    BcId {
        subsystem_major,
        capability_minor,
        sequence,
    }
}

/// A minimal `tracing::Subscriber` that records whether any `WARN`-level
/// event fired while it was the thread-local default subscriber
/// (`tracing::subscriber::set_default` — thread-local, not global, so this
/// is safe under parallel test execution). Self-contained: uses only the
/// `tracing` crate (already a `factory-dispatcher` dependency), no
/// `tracing-subscriber` dev-dependency needed.
struct WarnCaptureSubscriber {
    warned: Arc<AtomicBool>,
}

impl tracing::Subscriber for WarnCaptureSubscriber {
    fn enabled(&self, _metadata: &tracing::Metadata<'_>) -> bool {
        true
    }
    fn new_span(&self, _span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        tracing::span::Id::from_u64(1)
    }
    fn record(&self, _span: &tracing::span::Id, _values: &tracing::span::Record<'_>) {}
    fn record_follows_from(&self, _span: &tracing::span::Id, _follows: &tracing::span::Id) {}
    fn event(&self, event: &tracing::Event<'_>) {
        if *event.metadata().level() == tracing::Level::WARN {
            self.warned.store(true, Ordering::SeqCst);
        }
    }
    fn enter(&self, _span: &tracing::span::Id) {}
    fn exit(&self, _span: &tracing::span::Id) {}
}

/// Independently reimplements ADR-051 §Decision 18's base-26
/// spreadsheet-column-style letter-exhaustion naming (`.a`..`.z`,
/// `.aa`..`.az`, `.ba`...) for a 0-based chunk index, so the letter-
/// exhaustion test can assert against a computation that does NOT call
/// production code (avoiding a tautological self-referential assertion).
fn expected_sub_shard_suffix(index0: usize) -> String {
    let mut n = index0 + 1; // 1-based bijective numeral
    let mut letters = Vec::new();
    while n > 0 {
        let rem = (n - 1) % 26;
        letters.push((b'a' + rem as u8) as char);
        n = (n - 1) / 26;
    }
    letters.reverse();
    format!(".{}", letters.into_iter().collect::<String>())
}

fn row(id: BcId, body_len: usize) -> (BcId, String) {
    // Content is irrelevant to the chunker's own byte-accounting contract
    // (it treats each row as an opaque already-extracted String) — only
    // `.len()` matters, so a fixed-width synthetic payload keeps every
    // test's hand-computed expected boundary exact and easy to audit.
    (id, "x".repeat(body_len))
}

// ---------------------------------------------------------------------------
// Unit 1 — greedy-pack correctness: rows pack into the minimum sub-shards
// where each (preamble+rows) stays <= cap; boundaries fall exactly where
// the next row would exceed cap.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC6_chunk_subsystem_rows_into_sub_shards_greedy_pack_boundary_correctness() {
    // preamble.len() == 1; cap == 10.
    //   row A: len 5 -> row_bytes 6. current_chunk empty -> append.
    //          current_bytes = 1 + 6 = 7.
    //   row B: len 5 -> row_bytes 6. 7 + 6 = 13 > 10 -> close [A], start new
    //          chunk (current_bytes reset to 1), append B -> current_bytes
    //          = 1 + 6 = 7.
    //   row C: len 2 -> row_bytes 3. 7 + 3 = 10, NOT > 10 -> stays, append
    //          -> current_bytes = 10.
    // Expected: chunk 1 = [A] alone; chunk 2 = [B, C].
    let preamble = "H";
    let cap: u64 = 10;
    let a = row(bc_id(5, 1, 1), 5);
    let b = row(bc_id(5, 2, 1), 5);
    let c = row(bc_id(5, 3, 1), 2);
    let sorted_rows = vec![a.clone(), b.clone(), c.clone()];

    let chunks = chunk_subsystem_rows_into_sub_shards(&sorted_rows, preamble, cap);

    assert_eq!(
        chunks.len(),
        2,
        "the minimum number of sub-shards for this row set/cap is 2, got {} chunk(s): {chunks:?}",
        chunks.len()
    );
    assert_eq!(
        chunks[0],
        SubShardChunk {
            sub_shard_id: ".a".to_string(),
            body: format!("{preamble}{}\n", a.1),
            range_start: a.0,
            range_end: a.0,
        },
        "chunk 1 must contain exactly row A alone (the boundary must fall before row B, which \
         would have pushed current_bytes to 13 > cap 10)"
    );
    assert_eq!(
        chunks[1],
        SubShardChunk {
            sub_shard_id: ".b".to_string(),
            body: format!("{preamble}{}\n{}\n", b.1, c.1),
            range_start: b.0,
            range_end: c.0,
        },
        "chunk 2 must contain rows B and C together (B+C fits exactly at cap 10, the \
         exactly-at-cap inclusive rule)"
    );
    for chunk in &chunks {
        assert!(
            chunk.body.len() as u64 <= cap,
            "every non-lone-row chunk's body must stay <= shard_cap_bytes, got {} bytes for \
             {:?}",
            chunk.body.len(),
            chunk.sub_shard_id
        );
    }
}

// ---------------------------------------------------------------------------
// Unit 2 — determinism/idempotency: identical (sorted_rows, preamble, cap)
// run twice yields byte-identical Vec<SubShardChunk>; the steady-state
// full-rebuild path (re-derived row set, same content/order, fresh
// allocation) reproduces migration-time boundaries exactly for the same
// row set (ADR-051 §Decision 18 item 7 — pure function, no
// migration-specific state).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC6_chunk_subsystem_rows_into_sub_shards_deterministic_and_steady_state_reproducible()
 {
    let preamble = "### SS-05\n\n| BC ID | Title | Status | Capability | Stories |\n";
    let cap: u64 = 120;
    let sorted_rows: Vec<(BcId, String)> = (1..=8)
        .map(|n| row(bc_id(5, n, 1), 20 + (n as usize % 3) * 5))
        .collect();

    // Scenario (a): direct double-invocation over the identical slice —
    // the "run it twice" determinism claim.
    let run_1 = chunk_subsystem_rows_into_sub_shards(&sorted_rows, preamble, cap);
    let run_2 = chunk_subsystem_rows_into_sub_shards(&sorted_rows, preamble, cap);
    assert_eq!(
        run_1, run_2,
        "chunking the identical (sorted_rows, preamble, shard_cap_bytes) input twice must \
         produce byte-identical Vec<SubShardChunk>, on any invocation"
    );
    assert!(
        run_1.len() > 1,
        "fixture must actually require multiple chunks for this test to be meaningful, got {} \
         chunk(s)",
        run_1.len()
    );

    // Scenario (b): the future steady-state FULL REBUILD path re-derives
    // its row set independently (e.g. re-read from disk via the
    // sub-manifest + the new in-flight row, re-sorted) rather than reusing
    // the migration-time Vec's own allocation. A freshly and independently
    // constructed (but content/order-identical) row set must still produce
    // BYTE-IDENTICAL chunk boundaries to the migration-time run — this is
    // ADR-051 §Decision 18 item 7's "migrate once" vs. "grow into it
    // later" full idempotency/determinism parity claim.
    let steady_state_rebuilt_rows: Vec<(BcId, String)> = sorted_rows
        .iter()
        .map(|(id, body)| (*id, body.clone()))
        .collect();
    let steady_state_run =
        chunk_subsystem_rows_into_sub_shards(&steady_state_rebuilt_rows, preamble, cap);
    assert_eq!(
        run_1, steady_state_run,
        "the steady-state full-rebuild path (independently reconstructed row set, identical \
         content/order) must reproduce migration-time chunk boundaries exactly for the same row \
         set (ADR-051 §Decision 18 item 7)"
    );
}

// ---------------------------------------------------------------------------
// Unit 3 — lone-oversized-row: a single row larger than cap becomes its own
// over-cap lone sub-shard, never split mid-row, never fail-loud; a
// non-blocking tracing::warn! is emitted.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC6_chunk_subsystem_rows_into_sub_shards_lone_oversized_row_never_fails_loud_and_warns()
 {
    let preamble = "H"; // len 1
    let cap: u64 = 5;
    // row_bytes = 20 + 1 = 21, alone already far exceeds cap 5 even with
    // only the 1-byte preamble.
    let oversized = row(bc_id(5, 1, 1), 20);
    let sorted_rows = vec![oversized.clone()];

    let warned = Arc::new(AtomicBool::new(false));
    let subscriber = WarnCaptureSubscriber {
        warned: warned.clone(),
    };
    let chunks = tracing::subscriber::with_default(subscriber, || {
        // The non-blocking-path assertion: this call must return normally
        // (no panic / no fail-loud abort) even though the lone row
        // nominally exceeds shard_cap_bytes.
        chunk_subsystem_rows_into_sub_shards(&sorted_rows, preamble, cap)
    });

    assert_eq!(
        chunks.len(),
        1,
        "a lone oversized row must become its own single sub-shard, never split across two \
         files and never dropped, got {} chunk(s): {chunks:?}",
        chunks.len()
    );
    assert_eq!(chunks[0].range_start, oversized.0);
    assert_eq!(chunks[0].range_end, oversized.0);
    assert!(
        chunks[0].body.contains(&oversized.1),
        "the lone chunk's body must contain the full, unsplit row content"
    );
    assert!(
        chunks[0].body.len() as u64 > cap,
        "this chunk's body is EXPECTED to exceed shard_cap_bytes — that is the lone-oversized-\
         row edge case itself, not a bug (BC-1.18.005 Postcondition 6's MAX_SINGLE_RECORD_BYTES \
         margin already bounds this case)"
    );
    assert!(
        warned.load(Ordering::SeqCst),
        "a lone oversized row must emit a non-blocking tracing::warn! so the anomaly remains \
         visible (ADR-051 §Decision 18 edge-case table) — no WARN-level event was observed"
    );
}

// ---------------------------------------------------------------------------
// Unit 4 — exactly-at-cap: a chunk filled to EXACTLY shard_cap_bytes is
// allowed to keep the boundary row (<= inclusive), matching BC-1.18.005
// Postcondition 3's `projected_size <= shard_cap_bytes -> Continue`
// convention verbatim.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC6_chunk_subsystem_rows_into_sub_shards_exactly_at_cap_inclusive_boundary() {
    // preamble.len() == 1; cap == 10.
    //   row A: len 4 -> row_bytes 5. current_chunk empty -> append.
    //          current_bytes = 1 + 5 = 6.
    //   row B: len 3 -> row_bytes 4. 6 + 4 = 10 == cap, NOT > cap -> stays
    //          in the SAME chunk (the `<=` inclusive rule under test).
    //          current_bytes = 10.
    let preamble = "H";
    let cap: u64 = 10;
    let a = row(bc_id(5, 1, 1), 4);
    let b = row(bc_id(5, 2, 1), 3);
    let sorted_rows = vec![a.clone(), b.clone()];

    let chunks = chunk_subsystem_rows_into_sub_shards(&sorted_rows, preamble, cap);

    assert_eq!(
        chunks.len(),
        1,
        "an exactly-at-cap boundary (current_bytes + row_bytes == shard_cap_bytes) must stay in \
         the CURRENT chunk, not spill into a new one — got {} chunk(s): {chunks:?}",
        chunks.len()
    );
    assert_eq!(chunks[0].range_start, a.0);
    assert_eq!(chunks[0].range_end, b.0);
    assert_eq!(
        chunks[0].body.len() as u64,
        cap,
        "the single chunk's body must be exactly shard_cap_bytes (10) long — this is the exact \
         boundary this test pins, not merely <= cap"
    );
}

// ---------------------------------------------------------------------------
// Unit 5 — letter exhaustion: >26 sub-shards extend to base-26 .aa/.ab...
// naming, never fail-loud.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC6_chunk_subsystem_rows_into_sub_shards_letter_exhaustion_extends_to_base26() {
    // preamble is empty (len 0); cap == 5; every row body has len 4
    // (row_bytes = 5). Two rows would need 5 + 5 = 10 > 5, so EVERY row
    // becomes its own single-row chunk — a clean, deterministic way to
    // force exactly 28 chunks (indices 0..27), spanning past the 26-letter
    // single-letter exhaustion point (index 25 = ".z", index 26 = ".aa").
    let preamble = "";
    let cap: u64 = 5;
    let sorted_rows: Vec<(BcId, String)> = (0..28).map(|i| row(bc_id(5, i + 1, 1), 4)).collect();

    let chunks = chunk_subsystem_rows_into_sub_shards(&sorted_rows, preamble, cap);

    assert_eq!(
        chunks.len(),
        28,
        "fixture is constructed so every row is its own chunk — expected exactly 28 chunks, got \
         {}: {chunks:?}",
        chunks.len()
    );
    for (index0, chunk) in chunks.iter().enumerate() {
        let expected = expected_sub_shard_suffix(index0);
        assert_eq!(
            chunk.sub_shard_id, expected,
            "chunk at 0-based index {index0} must be named {expected:?} per ADR-051 §Decision \
             18's base-26 spreadsheet-column-style letter-exhaustion scheme (never fail-loud on \
             >26 chunks), got {:?}",
            chunk.sub_shard_id
        );
    }
    // Spot-check the two most important transition points explicitly, so
    // a future refactor of `expected_sub_shard_suffix` itself cannot
    // silently defeat this test's own intent.
    assert_eq!(chunks[0].sub_shard_id, ".a");
    assert_eq!(chunks[25].sub_shard_id, ".z");
    assert_eq!(
        chunks[26].sub_shard_id, ".aa",
        "the 27th chunk (0-based index 26) is exactly where single-letter naming exhausts and \
         must extend to the two-letter base-26 scheme, never fail-loud"
    );
    assert_eq!(chunks[27].sub_shard_id, ".ab");
}

// ---------------------------------------------------------------------------
// Integration fixtures shared by tests 6 and 7.
// ---------------------------------------------------------------------------

const SUBSHARD_SHARD_CONFIG: &str = "\
[[shard]]
artifact_stem = \"BC-INDEX\"
artifact_path = \".factory/specs/behavioral-contracts/BC-INDEX.md\"
practical_fuel_ceiling = 8000000
worst_case_fuel_per_byte = 106.36
max_single_record_bytes = 16384
safety_margin = 8192
shard_cap_bytes = 200
shape = \"flat\"
";

fn write_shard_config(cwd: &Path) {
    let factory_dir = cwd.join(".factory");
    std::fs::create_dir_all(&factory_dir).unwrap();
    std::fs::write(factory_dir.join("shard-config.toml"), SUBSHARD_SHARD_CONFIG).unwrap();
}

/// Builds a synthetic `### SS-<ss_num>` section with `row_count` rows, each
/// individually small but collectively far exceeding the 200-byte
/// `shard_cap_bytes` fixture cap above, so this subsystem genuinely
/// requires second-level sub-sharding (mirroring SS-05/SS-06's real
/// over-cap shape at a fixture-appropriate scale).
fn synthetic_over_cap_subsystem_section(ss_num: u32, row_count: u32) -> String {
    let mut body = format!(
        "### SS-{ss_num:02}\n\n\
         | BC ID | Title | Status | Capability | Stories |\n\
         |-------|-------|--------|-----------|---------|\n"
    );
    for n in 1..=row_count {
        body.push_str(&format!(
            "| [BC-{ss_num}.{n:02}.001](ss-{ss_num:02}/BC-{ss_num}.{n:02}.001.md) | Row {n} of \
             synthetic over-cap subsystem SS-{ss_num:02} | draft | CAP-TBD | TBD |\n"
        ));
    }
    body
}

fn synthetic_under_cap_subsystem_section(ss_num: u32) -> String {
    format!(
        "### SS-{ss_num:02}\n\n\
         | BC ID | Title | Status | Capability | Stories |\n\
         |-------|-------|--------|-----------|---------|\n\
         | [BC-{ss_num}.01.001](ss-{ss_num:02}/BC-{ss_num}.01.001.md) | Lone small row | draft \
         | CAP-TBD | TBD |\n"
    )
}

fn write_bc_index_fixture(cwd: &Path, total_bcs: usize, body: &str) {
    let bc_index_dir = cwd.join(".factory/specs/behavioral-contracts");
    std::fs::create_dir_all(&bc_index_dir).unwrap();
    let content = format!(
        "---\n\
         total_bcs: {total_bcs}\n\
         ---\n\
         ## Summary\n\n\
         {body}"
    );
    std::fs::write(bc_index_dir.join("BC-INDEX.md"), content).unwrap();
}

/// Every distinct `BcId` present across `section_body`'s own rows, via the
/// SAME canonical-row-extraction primitive the migration itself uses — an
/// independent-enough oracle for THIS test's purposes because it operates
/// on the pre-migration fixture content, never on the migration's own
/// staged output.
fn row_ids_in(section_body: &str) -> BTreeSet<BcId> {
    extract_and_sort_bc_rows(section_body)
        .expect("fixture section body must be well-formed")
        .into_iter()
        .map(|(id, _)| id)
        .collect()
}

// ---------------------------------------------------------------------------
// Integration 6 — full `run_bc_index_migration` round-trip against a
// synthetic over-cap SS fixture: N sub-shard files + a sub-manifest
// (BC-ID-range SubShardRangeEntry schema matching the READ-side
// resolve_bc_shard_path) are actually produced, and the independent census
// passes over the sub-split.
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC6_run_bc_index_migration_produces_sub_shard_files_and_manifest_for_over_cap_ss()
 {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path());

    let ss01 = synthetic_under_cap_subsystem_section(1);
    let ss05 = synthetic_over_cap_subsystem_section(5, 20);
    let ss05_ids = row_ids_in(&ss05);
    assert!(
        ss05.len() as u64 > 200,
        "fixture sanity: SS-05's synthetic section body must itself exceed the 200-byte \
         shard_cap_bytes fixture cap, got {} bytes",
        ss05.len()
    );

    let total_bcs = 1 + ss05_ids.len();
    write_bc_index_fixture(dir.path(), total_bcs, &format!("{ss01}\n{ss05}"));

    let outcome = run_bc_index_migration(dir.path())
        .expect("migration against a well-formed over-cap fixture must complete successfully");
    assert!(
        matches!(
            outcome,
            factory_dispatcher::shard_manager::BcIndexMigrationOutcome::Completed { .. }
        ),
        "expected a fresh Completed outcome, got {outcome:?}"
    );

    let shards_root = dir
        .path()
        .join(".factory/specs/behavioral-contracts/shards");
    let top_manifest = load_shard_manifest(&shards_root.join("BC-INDEX.shard-manifest.toml"))
        .expect(
            "the top-level shard manifest must be published at its canonical path post-migration",
        );
    let ss05_entry = top_manifest
        .subsystem_shard
        .iter()
        .find(|e| e.ss_id == "SS-05")
        .expect("the top-level manifest must carry an SS-05 entry");
    assert!(
        ss05_entry.sub_sharded,
        "SS-05 genuinely exceeds shard_cap_bytes in this fixture — the top-level manifest entry \
         must have sub_sharded=true (T-11 wires this from the previously-hardcoded false; this \
         assertion is now the GREEN regression guard for that fix)"
    );
    let sub_manifest_rel = ss05_entry
        .sub_manifest
        .clone()
        .expect("sub_sharded=true must be accompanied by a Some(sub_manifest) path");

    let sub_manifest =
        load_sub_shard_manifest(&shards_root.parent().unwrap().join(&sub_manifest_rel))
            .expect("the SS-05 sub-manifest must be readable at its declared canonical path");
    assert_eq!(sub_manifest.schema_version, 1);
    assert_eq!(sub_manifest.ss_id, "SS-05");
    assert!(
        sub_manifest.sub_shard.len() > 1,
        "SS-05's ~{} bytes against a 200-byte cap must require MORE THAN ONE sub-shard, got {}",
        ss05.len(),
        sub_manifest.sub_shard.len()
    );

    // Every sub-shard file must exist on disk and its range entry's
    // [range_start, range_end] must correctly bound the BcIds it actually
    // contains — the schema `resolve_bc_shard_path` (the already-
    // implemented reader) range-compares against.
    let mut union_ids: BTreeSet<BcId> = BTreeSet::new();
    for entry in &sub_manifest.sub_shard {
        let sub_shard_path = dir
            .path()
            .join(".factory/specs/behavioral-contracts")
            .join(&entry.path);
        let content = std::fs::read_to_string(&sub_shard_path).unwrap_or_else(|e| {
            panic!("sub-shard file {entry:?} must exist on disk post-migration: {e}")
        });
        let ids = row_ids_in(&content);
        assert!(
            !ids.is_empty(),
            "sub-shard {:?} must not be an empty chunk",
            entry.sub_shard_id
        );
        let range_start: BcId = factory_dispatcher::shard_manager::parse_bc_id(&entry.range_start)
            .expect("range_start must be a well-formed BC-X.YY.NNN");
        let range_end: BcId = factory_dispatcher::shard_manager::parse_bc_id(&entry.range_end)
            .expect("range_end must be a well-formed BC-X.YY.NNN");
        for id in &ids {
            assert!(
                *id >= range_start && *id <= range_end,
                "row {id} found in sub-shard {:?}'s file content lies OUTSIDE its own manifest \
                 range [{range_start}, {range_end}]",
                entry.sub_shard_id
            );
        }
        for id in &ids {
            assert!(
                union_ids.insert(*id),
                "row {id} appears in MORE THAN ONE SS-05 sub-shard — independent-census \
                 duplication"
            );
        }
    }
    assert_eq!(
        union_ids, ss05_ids,
        "the union of every SS-05 sub-shard's row IDs must equal EXACTLY the pre-split SS-05 \
         row-ID set — no row dropped, none duplicated"
    );

    // resolve_bc_shard_path (the already-implemented READ side) must
    // successfully round-trip a real BC-ID through the newly-staged
    // manifests, for both the first and last row in canonical order.
    let prefixes = SubsystemPrefixSnapshot {
        schema_version: 1,
        arch_index_sha: "fixture-sha".to_string(),
        prefix: vec![
            SubsystemPrefixEntry {
                bc_prefix_major: 1,
                ss_id: "SS-01".to_string(),
            },
            SubsystemPrefixEntry {
                bc_prefix_major: 5,
                ss_id: "SS-05".to_string(),
            },
        ],
    };
    let shards_dir_root = dir.path().join(".factory/specs/behavioral-contracts");
    for target_id in [bc_id(5, 1, 1), bc_id(5, 20, 1)] {
        let resolved = resolve_bc_shard_path(&target_id, &prefixes, &shards_dir_root)
            .unwrap_or_else(|e| panic!("resolve_bc_shard_path must resolve {target_id}: {e}"));
        let resolved_content = std::fs::read_to_string(shards_dir_root.join(&resolved))
            .unwrap_or_else(|e| panic!("resolved path {resolved:?} must exist on disk: {e}"));
        assert!(
            row_ids_in(&resolved_content).contains(&target_id),
            "resolve_bc_shard_path({target_id}) returned {resolved:?}, whose content does not \
             actually contain that BC-ID"
        );
    }
}

// ---------------------------------------------------------------------------
// Integration 7 — SS-05/SS-06 real-scale row-count sum vs. independent
// census: every BC row appears in exactly one sub-shard across the
// sub-split, census sum equals the pre-split count (BC-1.18.011 EC-004).
// ---------------------------------------------------------------------------

#[test]
fn test_BC_1_18_011_PC6_EC004_run_bc_index_migration_ss05_ss06_row_count_sum_matches_independent_census()
 {
    let dir = tempfile::tempdir().unwrap();
    write_shard_config(dir.path());

    let ss01 = synthetic_under_cap_subsystem_section(1);
    let ss05 = synthetic_over_cap_subsystem_section(5, 15);
    let ss06 = synthetic_over_cap_subsystem_section(6, 12);
    let ss01_ids = row_ids_in(&ss01);
    let ss05_ids = row_ids_in(&ss05);
    let ss06_ids = row_ids_in(&ss06);
    assert!(ss05.len() as u64 > 200 && ss06.len() as u64 > 200);

    let mut original_census: BTreeSet<BcId> = BTreeSet::new();
    original_census.extend(&ss01_ids);
    original_census.extend(&ss05_ids);
    original_census.extend(&ss06_ids);
    let total_bcs = original_census.len();

    write_bc_index_fixture(dir.path(), total_bcs, &format!("{ss01}\n{ss05}\n{ss06}"));

    run_bc_index_migration(dir.path())
        .expect("migration against a well-formed two-over-cap-subsystem fixture must complete");

    let shards_root = dir
        .path()
        .join(".factory/specs/behavioral-contracts/shards");
    let bc_index_root = dir.path().join(".factory/specs/behavioral-contracts");
    let top_manifest = load_shard_manifest(&shards_root.join("BC-INDEX.shard-manifest.toml"))
        .expect("top-level manifest must be published");

    // SS-01 stays flat (never sub-sharded) — a selective-application
    // regression control: only genuinely over-cap subsystems get a
    // second-level sub-split.
    let ss01_entry = top_manifest
        .subsystem_shard
        .iter()
        .find(|e| e.ss_id == "SS-01")
        .expect("SS-01 entry must exist");
    assert!(
        !ss01_entry.sub_sharded,
        "SS-01's synthetic section body is well under the 200-byte cap and must NOT be \
         sub-sharded"
    );
    let ss01_content = std::fs::read_to_string(bc_index_root.join(&ss01_entry.path))
        .expect("SS-01's single flat shard file must exist");
    let ss01_post_ids = row_ids_in(&ss01_content);
    assert_eq!(ss01_post_ids, ss01_ids);

    // A single closure computing "the full row-ID set recovered from an
    // over-cap subsystem's own sub-split," reused for SS-05 and SS-06.
    let recovered_ids_for = |ss_id: &str| -> BTreeSet<BcId> {
        let entry = top_manifest
            .subsystem_shard
            .iter()
            .find(|e| e.ss_id == ss_id)
            .unwrap_or_else(|| panic!("{ss_id} entry must exist in the top-level manifest"));
        assert!(
            entry.sub_sharded,
            "{ss_id} genuinely exceeds shard_cap_bytes in this fixture and must be sub_sharded"
        );
        let sub_manifest_rel = entry
            .sub_manifest
            .clone()
            .unwrap_or_else(|| panic!("{ss_id}: sub_sharded=true requires Some(sub_manifest)"));
        let sub_manifest = load_sub_shard_manifest(&bc_index_root.join(&sub_manifest_rel))
            .unwrap_or_else(|e| panic!("{ss_id} sub-manifest must be readable: {e}"));
        let mut ids: BTreeSet<BcId> = BTreeSet::new();
        for sub_entry in &sub_manifest.sub_shard {
            let content = std::fs::read_to_string(bc_index_root.join(&sub_entry.path))
                .unwrap_or_else(|e| panic!("{ss_id} sub-shard {sub_entry:?} must exist: {e}"));
            for id in row_ids_in(&content) {
                assert!(
                    ids.insert(id),
                    "{ss_id}: row {id} appears in MORE THAN ONE sub-shard — census violation"
                );
            }
        }
        ids
    };

    let ss05_recovered = recovered_ids_for("SS-05");
    let ss06_recovered = recovered_ids_for("SS-06");
    assert_eq!(
        ss05_recovered, ss05_ids,
        "SS-05's recovered sub-split row-ID set must equal exactly its pre-split BC-5.* row set \
         (BC-1.18.011 EC-004's own independent pre-split count requirement)"
    );
    assert_eq!(
        ss06_recovered, ss06_ids,
        "SS-06's recovered sub-split row-ID set must equal exactly its pre-split BC-6.* row set"
    );

    // The grand union across ALL subsystems (flat SS-01 + sub-split SS-05 +
    // sub-split SS-06) must equal the whole-corpus independent census
    // exactly — no drops, no duplicates, anywhere in the sub-split.
    let mut grand_union: BTreeSet<BcId> = BTreeSet::new();
    grand_union.extend(&ss01_post_ids);
    grand_union.extend(&ss05_recovered);
    grand_union.extend(&ss06_recovered);
    assert_eq!(
        grand_union.len(),
        total_bcs,
        "the union of every post-split row (flat + both sub-splits) must sum to EXACTLY the \
         independent pre-split census count ({total_bcs}), got {}",
        grand_union.len()
    );
    assert_eq!(
        grand_union, original_census,
        "the post-split grand union must equal the pre-split independent census SET exactly, \
         not merely match in cardinality"
    );
}
