//! Verification protocol for the roster-block repair (`repair_roster_blocks`), per the go/no-go
//! gates in `docs/plans/roster-block-repair-spike.md`, run against the NHL Legacy anchor:
//!
//! 1. reproduce the real corruption shapes (the old `dXSB`-retag `move_player`, and a physically
//!    displaced row à la swap-pop);
//! 2. repair, then assert `towx_coherence` is empty and the full save-time audit is write-clean;
//! 3. assert **row conservation** — the multiset of complete `eGlu` rows is unchanged (the repair
//!    is an order-only permutation, so no membership can change);
//! 4. assert unrelated blocks kept their relative order;
//! 5. idempotence — repairing a clean anchor is a byte-identical no-op.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tdb_core::TdbFile;
use tdb_domain_validation::{towx_coherence, validate_invariants};
use tdb_edit::txn::EditTransaction;
use tdb_enums::EnumRegistry;
use tdb_relations::{RelationRegistry, RosterModel};

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

fn defs(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../defs")
        .join(sub)
}

const DB: &str = "ps3/nhllegacy/databases/nhlng.db";

fn assert_write_clean(db_bytes: &[u8]) {
    let db = TdbFile::parse(db_bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();
    let enums = EnumRegistry::from_dir(defs("enums")).unwrap();
    let report = validate_invariants(&db, &relations, &enums);
    assert!(
        report.is_clean(),
        "audit not clean: {:?}",
        report.errors().take(5).collect::<Vec<_>>()
    );
}

/// Every `eGlu` row as the complete tuple of its fixed-bit field values, as a sorted multiset —
/// the conservation currency: a pure permutation must keep this identical.
fn eglu_row_multiset(db_bytes: &[u8]) -> Vec<Vec<u64>> {
    let db = TdbFile::parse(db_bytes).unwrap();
    let t = db.table_canonical("eGlu").unwrap();
    let mut rows: Vec<Vec<u64>> = (0..t.current_records as u32)
        .map(|i| {
            t.fields
                .iter()
                .map(|f| db.read_u64(t, i, f).expect("readable eGlu field"))
                .collect()
        })
        .collect();
    rows.sort();
    rows
}

/// Per-team sequence of slot keys (`toWX`) in record order — the "relative order within each
/// team's block" that the repair must preserve.
fn per_team_slot_sequences(db_bytes: &[u8]) -> HashMap<u64, Vec<u64>> {
    let db = TdbFile::parse(db_bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let mut seq: HashMap<u64, Vec<u64>> = HashMap::new();
    for s in &model.slots {
        seq.entry(s.team_id).or_default().push(s.slot_key);
    }
    seq
}

/// Retag `count` interior slots (one per donor team) with a foreign team id — the exact shape the
/// old buggy `move_player` produced — and return the corrupted bytes (CRCs fixed). The victims are
/// picked the way the old op would have found them mid-move: no captaincy, and a jersey that does
/// not collide on the destination team, so the corruption is purely *structural* (block-coherence)
/// and the healed database can be asserted fully write-clean.
fn corrupt_by_retag(bytes: &[u8], count: usize) -> Vec<u8> {
    let db = TdbFile::parse(bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let mut by_team: HashMap<u64, Vec<&tdb_relations::RosterSlot>> = HashMap::new();
    for s in &model.slots {
        by_team.entry(s.team_id).or_default().push(s);
    }
    let mut donors: Vec<u64> = by_team
        .iter()
        .filter(|(_, r)| r.len() >= 5)
        .map(|(&t, _)| t)
        .collect();
    donors.sort_unstable();
    assert!(
        donors.len() > count,
        "need {count} donor teams plus a foreign one"
    );

    // Numbers worn per team, updated as retags land so retagged players never collide either.
    let mut worn: HashMap<u64, std::collections::HashSet<u64>> = by_team
        .iter()
        .map(|(&t, slots)| {
            (
                t,
                slots
                    .iter()
                    .filter_map(|s| s.jersey)
                    .filter(|&n| n != 0)
                    .collect(),
            )
        })
        .collect();

    let eglu = db.table_canonical("eGlu").unwrap();
    let dxsb = eglu
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "dXSB")
        .unwrap();
    let mut buf = bytes.to_vec();
    for k in 0..count {
        let donor = donors[k];
        let foreign = donors[(k + 1) % donors.len()];
        assert_ne!(donor, foreign);
        let slots = &by_team[&donor];
        // Strictly interior victim (punches a real hole) with no captaincy and no jersey clash.
        let victim = slots[1..slots.len() - 1]
            .iter()
            .find(|s| {
                s.captaincy.unwrap_or(0) == 0
                    && s.jersey
                        .is_none_or(|n| n == 0 || !worn[&foreign].contains(&n))
            })
            .unwrap_or_else(|| panic!("no clean interior victim on team {donor}"));
        if let Some(n) = victim.jersey.filter(|&n| n != 0) {
            worn.get_mut(&foreign).unwrap().insert(n);
        }
        assert!(db.set_u64(&mut buf, eglu, victim.record, dxsb, foreign));
    }
    db.recompute_crcs(&mut buf);
    buf
}

/// The retag shape (a mislabeled slot inside another team's block): the repair regroups the rows
/// into contiguous blocks, conserves every row byte-for-byte, and leaves the full audit clean.
#[test]
fn repairs_single_retag_stray() {
    let clean = anchor(DB);
    let corrupt = corrupt_by_retag(&clean, 1);
    assert!(!towx_coherence(&TdbFile::parse(&corrupt).unwrap()).is_empty());
    let rows_before = eglu_row_multiset(&corrupt);
    let order_before = per_team_slot_sequences(&corrupt);

    let mut tx = EditTransaction::begin(&corrupt).unwrap();
    let (outcome, summary) = tx.repair_roster_blocks().unwrap();
    assert!(outcome.edits > 0, "the stray forces repositioning");
    assert_eq!(outcome.edits, summary.moved_slots);
    assert!(!summary.affected_teams.is_empty());

    let out = tx.commit().unwrap();
    let healed = TdbFile::parse(&out).unwrap();
    assert!(healed.header_crc_ok(), "CRCs refreshed on commit");
    assert!(
        towx_coherence(&healed).is_empty(),
        "coherence findings all cleared"
    );
    assert_write_clean(&out);

    // Conservation: the multiset of complete rows is untouched — only positions changed.
    assert_eq!(
        eglu_row_multiset(&out),
        rows_before,
        "no row content changed"
    );
    // Every team keeps its slots in the same relative order (keyed by toWX).
    assert_eq!(
        per_team_slot_sequences(&out),
        order_before,
        "in-block order preserved"
    );
}

/// Dense corruption — many strays across many donor teams (the 1000+-finding shape seen on real
/// databases corrupted by older tools). One repair pass clears all of it.
#[test]
fn repairs_dense_retag_corruption() {
    let clean = anchor(DB);
    let corrupt = corrupt_by_retag(&clean, 40);
    let findings = towx_coherence(&TdbFile::parse(&corrupt).unwrap()).len();
    assert!(
        findings >= 40,
        "dense corruption expected, got {findings} findings"
    );
    let rows_before = eglu_row_multiset(&corrupt);

    let mut tx = EditTransaction::begin(&corrupt).unwrap();
    let (outcome, _) = tx.repair_roster_blocks().unwrap();
    assert!(outcome.edits > 0);

    let out = tx.commit().unwrap();
    assert!(towx_coherence(&TdbFile::parse(&out).unwrap()).is_empty());
    assert_write_clean(&out);
    assert_eq!(eglu_row_multiset(&out), rows_before);
    assert!(TdbFile::parse(&out).unwrap().header_crc_ok());
}

/// The physically-displaced shape (what a swap-pop delete leaves behind): two whole rows from
/// different teams exchange record positions. The repair puts each back inside its label's block.
#[test]
fn repairs_physically_swapped_rows() {
    let clean = anchor(DB);
    let db = TdbFile::parse(&clean).unwrap();
    let model = RosterModel::build(&db).unwrap();
    // Two records far apart, from different teams.
    let a = model.slots.first().unwrap();
    let b = model
        .slots
        .iter()
        .rev()
        .find(|s| s.team_id != a.team_id)
        .unwrap();
    let (rec_a, rec_b) = (a.record, b.record);

    // Swap the full rows field-by-field (every fixed-bit field, both directions).
    let eglu = db.table_canonical("eGlu").unwrap();
    let mut buf = clean.clone();
    for f in &eglu.fields {
        let va = db.read_u64(eglu, rec_a, f).unwrap();
        let vb = db.read_u64(eglu, rec_b, f).unwrap();
        assert!(db.set_u64(&mut buf, eglu, rec_a, f, vb));
        assert!(db.set_u64(&mut buf, eglu, rec_b, f, va));
    }
    db.recompute_crcs(&mut buf);
    assert!(!towx_coherence(&TdbFile::parse(&buf).unwrap()).is_empty());
    let rows_before = eglu_row_multiset(&buf);

    let mut tx = EditTransaction::begin(&buf).unwrap();
    let (outcome, _) = tx.repair_roster_blocks().unwrap();
    assert!(outcome.edits > 0);

    let out = tx.commit().unwrap();
    assert!(towx_coherence(&TdbFile::parse(&out).unwrap()).is_empty());
    assert_write_clean(&out);
    assert_eq!(eglu_row_multiset(&out), rows_before);
}

/// Idempotence gate: on an already-coherent database the repair is a zero-edit no-op and the
/// working bytes are untouched.
#[test]
fn noop_on_clean_anchor() {
    let clean = anchor(DB);
    let mut tx = EditTransaction::begin(&clean).unwrap();
    let (outcome, summary) = tx.repair_roster_blocks().unwrap();
    assert_eq!(outcome.edits, 0);
    assert_eq!(summary.moved_slots, 0);
    assert!(summary.affected_teams.is_empty());
    assert_eq!(
        tx.bytes(),
        &clean[..],
        "no-op leaves the bytes byte-identical"
    );

    // And repairing a just-repaired database is likewise a no-op (converges in one pass).
    let corrupt = corrupt_by_retag(&clean, 3);
    let mut tx = EditTransaction::begin(&corrupt).unwrap();
    let (first, _) = tx.repair_roster_blocks().unwrap();
    assert!(first.edits > 0);
    let (second, _) = tx.repair_roster_blocks().unwrap();
    assert_eq!(second.edits, 0, "second pass finds nothing to move");
}
