// Parity fixture (S-25.10 AC-009 / red test T11, writer-bytes half). TEST-SUPPORT module:
// S-25.06 imports it at its rebase (`#[path = ".../s2510_support/parity.rs"] mod parity;`
// next to the `api` and `support` modules) to prove its `backfill-append-logs` coordinator
// produces the SAME writer bytes as `migrate-bc-index` through the ONE shared module.
// S-25.11 AC-005 extends the SAME fixture with the DONE field-population assertion once the
// B-2 population logic exists.
#![allow(dead_code, clippy::expect_used, clippy::unwrap_used, clippy::panic)]

use crate::support::{Rec, sha256_hex};

/// One migration's parameter set: its txn id and the ordered target list (the plan).
pub struct ParitySet {
    pub name: &'static str,
    pub txn_id: String,
    /// `(target_canonical, staging_path)` in plan order. `len()` is N.
    pub targets: Vec<(String, String)>,
}

/// `migrate-bc-index`: N = the number of staged shards (one per subsystem shard, the shard
/// manifest, and the lean `BC-INDEX.md` body). Five here.
pub fn migrate_bc_index_set() -> ParitySet {
    let canon = "/proj/.factory/specs/behavioral-contracts";
    let gen_dir = "/proj/.factory/migration-state/gen-7c1d2e3f-4a5b-4c6d-8e7f-9a0b1c2d3e4f";
    ParitySet {
        name: "migrate-bc-index",
        txn_id: "0f8e4c1a-6b7d-4e2f-9a3c-5d1b2e7f8a90".into(),
        targets: vec![
            (
                format!("{canon}/shards/BC-INDEX-SS-01.md"),
                format!("{gen_dir}/shards/BC-INDEX-SS-01.md"),
            ),
            (
                format!("{canon}/shards/BC-INDEX-SS-02.md"),
                format!("{gen_dir}/shards/BC-INDEX-SS-02.md"),
            ),
            (
                format!("{canon}/shards/BC-INDEX-SS-03.md"),
                format!("{gen_dir}/shards/BC-INDEX-SS-03.md"),
            ),
            (
                format!("{canon}/shard-manifest.toml"),
                format!("{gen_dir}/shard-manifest.toml"),
            ),
            (
                format!("{canon}/BC-INDEX.md"),
                format!("{gen_dir}/BC-INDEX.md"),
            ),
        ],
    }
}

/// `backfill-append-logs`: N is the FIXED four `decision-log.md`, `burst-log.md`,
/// `lessons.md`, `session-checkpoints.md` targets (BC-1.18.013 Precondition 5).
pub fn backfill_append_logs_set() -> ParitySet {
    let cycle = "/proj/.factory/cycles/v1.0-feature-engine-discipline-pass-1";
    let gen_dir = "/proj/.factory/migration-state/gen-9a8b7c6d-5e4f-4a3b-8c2d-1e0f9a8b7c6d";
    let names = [
        "decision-log.md",
        "burst-log.md",
        "lessons.md",
        "session-checkpoints.md",
    ];
    ParitySet {
        name: "backfill-append-logs",
        txn_id: "5b3f9d21-7c4e-4a80-b6d2-0e1f2a3b4c5d".into(),
        targets: names
            .iter()
            .map(|n| (format!("{cycle}/{n}"), format!("{gen_dir}/{n}")))
            .collect(),
    }
}

pub fn sets() -> Vec<ParitySet> {
    vec![migrate_bc_index_set(), backfill_append_logs_set()]
}

impl ParitySet {
    /// The logical records of a full successful run: every INTENT (fencing 1), then every
    /// DONE (fencing 2) copying the INTENT's four fields. Post/pre hashes are a pure
    /// function of the target INDEX, so the two sets share them for equal indices.
    pub fn records(&self) -> Vec<Rec> {
        let intents: Vec<Rec> = self
            .targets
            .iter()
            .enumerate()
            .map(|(i, (target, staging))| Rec {
                txn_id: self.txn_id.clone(),
                fencing: 1,
                rtype: "INTENT".into(),
                target: target.clone(),
                staging: staging.clone(),
                post: sha256_hex(format!("staged-content-{i}").as_bytes()),
                pre: if i.is_multiple_of(2) {
                    "MISSING".into()
                } else {
                    sha256_hex(format!("old-content-{i}").as_bytes())
                },
                ts: format!("2026-10-08T12:00:{:02}Z", i),
            })
            .collect();
        let dones: Vec<Rec> = intents
            .iter()
            .enumerate()
            .map(|(i, r)| r.done_of(2, &format!("2026-10-08T12:01:{:02}Z", i)))
            .collect();
        intents.into_iter().chain(dones).collect()
    }

    pub fn n(&self) -> usize {
        self.targets.len()
    }
}
