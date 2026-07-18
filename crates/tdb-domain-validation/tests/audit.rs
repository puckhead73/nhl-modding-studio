//! Whole-DB integrity audit against a real committed anchor (PS3 NHL Legacy `nhlng.db`).

use std::path::{Path, PathBuf};
use tdb_core::TdbFile;
use tdb_domain_validation::{
    audit_edge, companion_presence, dup_on_team, referential_integrity, single_captain,
    towx_coherence, validate_invariants, Severity,
};
use tdb_enums::EnumRegistry;
use tdb_relations::{RelationRegistry, RosterModel};

fn defs(sub: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../defs")
        .join(sub)
}

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

/// The confirmed foreign keys resolve overwhelmingly against the real roster — the predicate agrees
/// with the relationship evidence (subset ~1.0). We assert a high resolution rate rather than a hard
/// zero, because the committed vanilla anchor is not the operator's `default.db` the defs were
/// promoted on; a few legitimate sentinels/free-agent rows may remain.
#[test]
fn confirmed_fks_resolve_against_anchor() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();

    let mut audited = 0;
    for rel in relations.relationships() {
        for b in &rel.bindings {
            let Some(a) = audit_edge(&db, b) else {
                continue;
            };
            // Only assert on edges with a meaningful population in this anchor.
            if a.source_populated >= 100 {
                audited += 1;
                eprintln!(
                    "{}.{} -> {}.{}: {}/{} resolve ({:.4}), {} dangling",
                    a.source_table,
                    a.source_field,
                    a.target_table,
                    a.target_field,
                    a.resolved,
                    a.source_populated,
                    a.resolution_rate(),
                    a.dangling.len()
                );
                assert!(
                    a.resolution_rate() >= 0.95,
                    "{}.{} -> {}.{} only {:.4} resolved",
                    a.source_table,
                    a.source_field,
                    a.target_table,
                    a.target_field,
                    a.resolution_rate()
                );
            }
        }
    }
    assert!(
        audited >= 2,
        "expected at least the roster/player FKs to be audited, got {audited}"
    );
}

/// The companion-presence predicate (first Error-severity invariant) finds zero broken clusters on
/// the vanilla anchor — every player has the companion pair its kind requires.
#[test]
fn companion_presence_clean_on_anchor() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let findings = companion_presence(&db);
    assert!(
        findings.is_empty(),
        "expected zero companion violations, got {}: {:?}",
        findings.len(),
        findings.iter().take(5).collect::<Vec<_>>()
    );
}

/// NHL07 (gen6 `NHL07NG.db`) ships no equipment companion tables, so `companion_presence` must raise
/// zero errors on it — an absent-in-the-schema table is not a broken cluster. Before the schema-aware
/// fix this predicate flagged all 2250 players, flooding the audit and blocking edits.
#[test]
fn companion_presence_clean_on_nhl07_without_equipment() {
    let bytes = anchor("xbox360/nhl07/databases/NHL07NG.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let findings = companion_presence(&db);
    assert!(
        findings.is_empty(),
        "NHL07 has no equipment tables; expected zero companion errors, got {}: {:?}",
        findings.len(),
        findings.iter().take(5).collect::<Vec<_>>()
    );
}

/// The single-captain predicate finds zero violations on the vanilla anchor (EA data carries at most
/// one captain per team) — so it is safe to compose into the write-clean save-time audit.
#[test]
fn single_captain_clean_on_anchor() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let findings = single_captain(&db);
    assert!(
        findings.is_empty(),
        "expected zero multi-captain teams, got {}: {:?}",
        findings.len(),
        findings.iter().take(5).collect::<Vec<_>>()
    );
}

/// The `toWX`-coherence predicate finds zero violations on the vanilla anchor — every team's `eGlu`
/// slots form a contiguous record + `toWX` run and the blocks tile the table (Spike S2). This is the
/// baseline the occupant-relocation move/trade ops must preserve.
#[test]
fn towx_coherence_clean_on_anchor() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let findings = towx_coherence(&db);
    assert!(
        findings.is_empty(),
        "expected zero block-coherence violations, got {}: {:?}",
        findings.len(),
        findings.iter().take(5).collect::<Vec<_>>()
    );
}

/// Reproduce the exact corruption the old `dXSB`-retag `move_player` caused: retag a slot that sits
/// *inside* one team's contiguous block with a *different* team's id. That punches a hole in the
/// donor block and drops a stray row into the foreign block — both non-contiguous — so the predicate
/// must flag it. Proves the audit would have caught the bug the occupant-relocation fix removed.
#[test]
fn towx_coherence_flags_a_foreign_team_id_inside_a_block() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();

    // Pick a team whose block has >= 3 slots and take an *interior* record (so removing it leaves a
    // genuine gap, not just a shrunk-but-contiguous range).
    let mut by_team: std::collections::HashMap<u64, Vec<u32>> = std::collections::HashMap::new();
    for s in &model.slots {
        by_team.entry(s.team_id).or_default().push(s.record);
    }
    let (&donor_team, records) = by_team
        .iter()
        .find(|(_, recs)| recs.len() >= 3)
        .expect("a team block with >= 3 slots");
    let mut recs = records.clone();
    recs.sort_unstable();
    let victim_record = recs[recs.len() / 2]; // strictly interior
    let foreign_team = *by_team
        .keys()
        .find(|&&t| t != donor_team)
        .expect("a second team");

    // Write the foreign team id onto the victim slot's dXSB and fix CRCs.
    let eglu = db.table_canonical("eGlu").unwrap();
    let dxsb = eglu
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "dXSB")
        .unwrap();
    let mut buf = bytes.clone();
    assert!(db.set_u64(&mut buf, eglu, victim_record, dxsb, foreign_team));
    db.recompute_crcs(&mut buf);

    let dirty = TdbFile::parse(&buf).unwrap();
    let findings = towx_coherence(&dirty);
    assert!(
        !findings.is_empty(),
        "retagging record {victim_record} from team {donor_team} to {foreign_team} must break block \
         coherence"
    );
    assert!(
        findings.iter().all(|f| f.severity == Severity::Error),
        "block-coherence violations are errors"
    );
}

/// The dup-on-team predicate is clean on the vanilla anchor — no player is double-slotted on a team
/// and no two occupied slots share a non-sentinel jersey (team 107's three `0`-jersey slots are the
/// exempt "unassigned" sentinel, not a collision).
#[test]
fn dup_on_team_clean_on_anchor() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let findings = dup_on_team(&db);
    assert!(
        findings.is_empty(),
        "expected zero dup-on-team violations, got {}: {:?}",
        findings.len(),
        findings.iter().take(5).collect::<Vec<_>>()
    );
}

/// Patch a fixed-bit field of an `eGlu` record and fix CRCs — a dirty-fixture helper.
fn patch_eglu(bytes: &[u8], record: u32, field: &str, value: u64) -> Vec<u8> {
    let db = TdbFile::parse(bytes).unwrap();
    let eglu = db.table_canonical("eGlu").unwrap();
    let f = eglu.fields.iter().find(|f| f.id.canonical(db.endian) == field).unwrap();
    let mut buf = bytes.to_vec();
    assert!(db.set_u64(&mut buf, eglu, record, f, value));
    db.recompute_crcs(&mut buf);
    buf
}

/// Pointing a second slot's `XSWT` at a player already slotted on the same team is flagged as a
/// same-player duplicate. Records 0 and 1 are both occupied slots on team 0 (the front block).
#[test]
fn dup_on_team_flags_same_player_twice() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let eglu = db.table_canonical("eGlu").unwrap();
    let xswt = eglu.fields.iter().find(|f| f.id.canonical(db.endian) == "XSWT").unwrap();
    // Both records must live on team 0; copy record 0's occupant onto record 1.
    let occupant0 = db.read_u64(eglu, 0, xswt).unwrap();
    drop(db);

    let buf = patch_eglu(&bytes, 1, "XSWT", occupant0);
    let dirty = TdbFile::parse(&buf).unwrap();
    let findings = dup_on_team(&dirty);
    assert!(
        findings.iter().any(|f| f.field == "XSWT" && f.severity == Severity::Error),
        "same player on two slots of a team must be flagged, got {findings:?}"
    );
}

/// Two occupied slots on one team wearing the same non-zero jersey is flagged.
#[test]
fn dup_on_team_flags_duplicate_jersey() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    // Records 0 and 1 are occupied team-0 slots; force both to jersey 77.
    let buf = patch_eglu(&bytes, 0, "sVRt", 77);
    let buf = patch_eglu(&buf, 1, "sVRt", 77);
    let dirty = TdbFile::parse(&buf).unwrap();
    let findings = dup_on_team(&dirty);
    assert!(
        findings.iter().any(|f| f.field == "sVRt" && f.severity == Severity::Error),
        "duplicate jersey within a team must be flagged, got {findings:?}"
    );

    // The `0` sentinel stays exempt: setting both to 0 is not a violation.
    let z = patch_eglu(&bytes, 0, "sVRt", 0);
    let z = patch_eglu(&z, 1, "sVRt", 0);
    let clean = TdbFile::parse(&z).unwrap();
    assert!(
        !dup_on_team(&clean).iter().any(|f| f.field == "sVRt"),
        "jersey 0 is the unassigned sentinel and must not trip the uniqueness check"
    );
}

/// The composed save-time audit runs end-to-end and stays **write-clean** (no Error-severity
/// findings) on the vanilla anchor, while correctly *surfacing* the documented edge cases as
/// Warnings — the partial-subset `team-links` orphans (`PuTc.vfEq`) and the `ZBac.ykFq` secondary
/// null sentinel. This proves the engine flags-for-inspection without crying wolf on clean data.
#[test]
fn vanilla_anchor_is_write_clean_and_flags_documented_exceptions() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();
    let enums = EnumRegistry::from_dir(defs("enums")).unwrap();

    let report = validate_invariants(&db, &relations, &enums);
    eprintln!(
        "findings: {} error, {} warning, {} info",
        report.count(Severity::Error),
        report.count(Severity::Warning),
        report.count(Severity::Info),
    );

    // Write-clean: nothing provably illegal on vanilla data.
    assert!(
        report.is_clean(),
        "vanilla anchor must have no Error-severity findings; first few: {:?}",
        report.errors().take(5).collect::<Vec<_>>()
    );

    // The genuine partial-subset FK orphans ARE surfaced as warnings...
    let warns: Vec<_> = report
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Warning)
        .collect();
    assert!(!warns.is_empty(), "expected the documented FK exceptions as warnings");
    assert!(
        warns.iter().any(|f| f.table == "PuTc" && f.field == "vfEq"),
        "team-links orphan-by-design values should warn"
    );
    // ...but the documented secondary null sentinel is now recognized as a placeholder and excluded
    // (it is not a dangling key, so flagging it only cried wolf).
    assert!(
        !report.findings.iter().any(|f| f.table == "ZBac" && f.field == "ykFq"),
        "ZBac.ykFq = 16382 is a known null sentinel and must no longer be flagged"
    );
}

/// NHL07 stores `eGlu` *interleaved* (each team's slots scattered, not in one contiguous run) and
/// carries no `toWX` slot key, so the block-coherence invariant does not apply. The predicate must
/// skip it rather than report the (valid) interleave as corruption — this was 2453 false errors.
#[test]
fn towx_coherence_skipped_on_nhl07_interleaved_roster() {
    let bytes = anchor("xbox360/nhl07/databases/NHL07NG.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let findings = towx_coherence(&db);
    assert!(
        findings.is_empty(),
        "NHL07 has no toWX-keyed block layout; expected zero, got {}",
        findings.len()
    );
}

/// NHL07's player index (`ZBac`) carries a free list: ~200 pre-allocated rows keyed below the roster's
/// player-id base, referenced by no roster slot. Their `ykFq` resolves to no bio row, but they are
/// unallocated capacity, not dangling references. `referential_integrity` must exclude them (roster
/// reachability), so the player-index edge raises no false "broken reference" warnings — this was 199.
#[test]
fn nhl07_free_list_index_rows_are_not_dangling_refs() {
    let bytes = anchor("xbox360/nhl07/databases/NHL07NG.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();

    // The raw edge still shows the free-list rows as unresolved...
    let index_edge = relations
        .relationships()
        .iter()
        .flat_map(|r| &r.bindings)
        .find(|b| b.source_table == "ZBac" && b.source_field == "ykFq")
        .expect("player-index-to-bio edge");
    let raw = audit_edge(&db, index_edge).unwrap();
    assert!(raw.dangling.len() >= 100, "raw edge sees the free-list rows as unresolved");

    // ...but the predicate recognizes them as unreferenced free-list slots and raises none.
    let findings = referential_integrity(&db, &relations);
    assert!(
        !findings.iter().any(|f| f.table == "ZBac" && f.field == "ykFq"),
        "free-list index rows must not surface as broken player references; got {}",
        findings.iter().filter(|f| f.table == "ZBac").count()
    );
}

/// End-to-end: the composed audit is write-clean on the NHL07 anchor with zero Error **and** zero
/// Warning findings — the two false-positive floods (companion presence, roster blocks) and the
/// free-list "broken references" are all gone. Remaining findings, if any, are `Info` (unlabelled
/// enum codes), which don't block editing.
#[test]
fn nhl07_audit_is_write_clean_without_false_floods() {
    let bytes = anchor("xbox360/nhl07/databases/NHL07NG.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let relations = RelationRegistry::from_dir(defs("relationships")).unwrap();
    let enums = EnumRegistry::from_dir(defs("enums")).unwrap();

    let report = validate_invariants(&db, &relations, &enums);
    eprintln!(
        "NHL07 findings: {} error, {} warning, {} info",
        report.count(Severity::Error),
        report.count(Severity::Warning),
        report.count(Severity::Info),
    );
    assert_eq!(report.count(Severity::Error), 0, "no Error findings on NHL07");
    assert_eq!(report.count(Severity::Warning), 0, "no Warning findings on NHL07");
}
