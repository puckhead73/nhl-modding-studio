//! LEAGUE-R2 — the delta referential gate compares violation *identities*, never counts.
//!
//! Focused validator cases over hand-built before/after states of the NHL 18 (PS4) anchor, whose
//! `stockleagues` rows cover neither code 30 nor 31 (asserted below), plus the session seams.
//! Evidence level: automated-tested on committed anchors; no desktop or in-game run.

use std::path::PathBuf;

use studio_domain::referential::new_dangling_references;
use studio_domain::{EditSession, FieldValue, Platform, Project, ProjectRole};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_relations::RelationRegistry;

const NHL18: &str = "anchors/ps4/nhl18/databases/nhl_game.db";
const META18: &str = "anchors/ps4/nhl18/databases/nhl_game-meta.xml";
const REL: &str = "team-league";

fn repo(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

struct Fixture {
    bytes: Vec<u8>,
    meta: FieldMetaIndex,
    relations: RelationRegistry,
}

fn fixture() -> Fixture {
    let bytes = std::fs::read(repo(NHL18)).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(META18)).unwrap()).unwrap();
    let relations = RelationRegistry::from_dir(repo("defs/relationships")).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let fslk = db.table_canonical("fsLK").unwrap();
    let key = fslk
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "aQrQ")
        .unwrap();
    let codes: Vec<u64> = (0..fslk.current_records as u32)
        .filter_map(|i| db.read_u64(fslk, i, key))
        .collect();
    assert!(codes.contains(&0), "NHL row backs code 0");
    assert!(
        !codes.contains(&30) && !codes.contains(&31),
        "NHL 18 has no league rows for 30/31: {codes:?}"
    );
    Fixture {
        bytes,
        meta,
        relations,
    }
}

/// Write one integer cell straight into the bytes (no edit layer) and refresh the CRCs.
fn poke(bytes: &[u8], table: &str, record: u32, field: &str, value: u64) -> Vec<u8> {
    let db = TdbFile::parse(bytes).unwrap();
    let t = db.table_canonical(table).unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == field)
        .unwrap();
    let mut out = bytes.to_vec();
    let start = t.record_block + record as usize * t.record_len_bytes as usize;
    let end = start + t.record_len_bytes as usize;
    tdb_core::write_bits(
        &mut out[start..end],
        f.bit_offset,
        f.bit_width,
        value,
        db.endian,
    );
    db.recompute_crcs(&mut out);
    out
}

fn team_league_new(fx: &Fixture, before: &[u8], after: &[u8]) -> Vec<(u32, u64)> {
    let b = TdbFile::parse(before).unwrap();
    let a = TdbFile::parse(after).unwrap();
    new_dangling_references(&b, &a, &fx.relations, &fx.meta)
        .into_iter()
        .filter(|v| v.relationship == REL)
        .map(|v| (v.record, v.value))
        .collect()
}

fn kott_raw(bytes: &[u8]) -> String {
    let db = TdbFile::parse(bytes).unwrap();
    db.table_canonical("kOtt").unwrap().id.raw()
}

#[test]
fn equal_count_substitution_is_a_new_violation() {
    let fx = fixture();
    // Pre-existing: team 0 → 31. Substituting a different absent code keeps the count at one.
    let before = poke(&fx.bytes, "kOtt", 0, "xMjj", 31);
    let after = poke(&before, "kOtt", 0, "xMjj", 30);
    assert_eq!(team_league_new(&fx, &before, &after), vec![(0, 30)]);
    // The reverse substitution is equally new.
    assert_eq!(team_league_new(&fx, &after, &before), vec![(0, 31)]);
    // Same row, same value: nothing new (an unrelated cell change on that row).
    let renamed = poke(&before, "kOtt", 0, "fwiK", 2);
    assert!(team_league_new(&fx, &before, &renamed).is_empty());
}

#[test]
fn repair_plus_introduction_at_equal_and_lower_counts_is_refused() {
    let fx = fixture();
    // Equal count: repair team 0 (31 → NHL) and move the same absent code onto team 1.
    let before = poke(&fx.bytes, "kOtt", 0, "xMjj", 31);
    let after = poke(&poke(&before, "kOtt", 0, "xMjj", 0), "kOtt", 1, "xMjj", 31);
    assert_eq!(team_league_new(&fx, &before, &after), vec![(1, 31)]);
    // Lower count: two pre-existing violations repaired, one new one introduced elsewhere.
    let before2 = poke(&before, "kOtt", 1, "xMjj", 30);
    let after2 = poke(
        &poke(&poke(&before2, "kOtt", 0, "xMjj", 0), "kOtt", 1, "xMjj", 0),
        "kOtt",
        2,
        "xMjj",
        31,
    );
    assert_eq!(team_league_new(&fx, &before2, &after2), vec![(2, 31)]);
    // Pure repairs are welcome.
    let repaired = poke(&poke(&before2, "kOtt", 0, "xMjj", 0), "kOtt", 1, "xMjj", 0);
    assert!(team_league_new(&fx, &before2, &repaired).is_empty());
    // Duplicate identities are counted: a second team with the *same* key cannot exist, but the
    // same value on a different key is a distinct identity — one repaired, one introduced.
    let swapped = poke(
        &poke(&before2, "kOtt", 0, "xMjj", 30),
        "kOtt",
        1,
        "xMjj",
        31,
    );
    assert_eq!(
        team_league_new(&fx, &before2, &swapped),
        vec![(0, 30), (1, 31)]
    );
}

#[test]
fn unchanged_violations_survive_record_movement() {
    let fx = fixture();
    // Ordered shift: a violation at record 5 lands at record 4 when record 0 is removed.
    let before = poke(&fx.bytes, "kOtt", 5, "xMjj", 31);
    let db = TdbFile::parse(&before).unwrap();
    let shifted = db.remove_record_ordered(&kott_raw(&before), 0).unwrap();
    let after = TdbFile::parse(&shifted).unwrap();
    let kott = after.table_canonical("kOtt").unwrap();
    let league = kott
        .fields
        .iter()
        .find(|f| f.id.canonical(after.endian) == "xMjj")
        .unwrap();
    assert_eq!(after.read_u64(kott, 4, league), Some(31), "moved, not lost");
    assert!(team_league_new(&fx, &before, &shifted).is_empty());

    // Swap-pop: the dangling tail row drops into the removed slot.
    let last = db.table_canonical("kOtt").unwrap().current_records as u32 - 1;
    let before_tail = poke(&fx.bytes, "kOtt", last, "xMjj", 31);
    let db_tail = TdbFile::parse(&before_tail).unwrap();
    let (popped, moved) = db_tail.remove_record(&kott_raw(&before_tail), 0).unwrap();
    assert_eq!(moved, Some(last));
    assert!(team_league_new(&fx, &before_tail, &popped).is_empty());

    // Movement does not hide a genuinely new violation either.
    let popped_bad = poke(&popped, "kOtt", 1, "xMjj", 30);
    assert_eq!(team_league_new(&fx, &popped, &popped_bad), vec![(1, 30)]);
}

#[test]
fn titles_without_the_league_table_have_no_rule() {
    let fx = fixture();
    let legacy = std::fs::read(repo("anchors/ps3/nhllegacy/databases/nhlng.db")).unwrap();
    let meta = FieldMetaIndex::from_xml(
        &std::fs::read_to_string(repo("anchors/ps3/nhllegacy/databases/nhlng-meta.xml")).unwrap(),
    )
    .unwrap();
    let after = poke(&legacy, "kOtt", 0, "xMjj", 31);
    let b = TdbFile::parse(&legacy).unwrap();
    let a = TdbFile::parse(&after).unwrap();
    assert!(new_dangling_references(&b, &a, &fx.relations, &meta)
        .iter()
        .all(|v| v.relationship != REL));
}

/// The reviewer's NHL 18 reproduction at the session seams: invalid 31 → invalid 30 is refused
/// on `set_field` and `set_cells_int`; bytes, undo/redo and pending state stay put; an unrelated
/// edit on the bad row and a valid repair are accepted.
#[test]
fn nhl18_invalid_to_invalid_substitution_refused_on_every_edit_seam() {
    let fx = fixture();
    let seeded = poke(&fx.bytes, "kOtt", 0, "xMjj", 31);
    let dir = std::env::temp_dir().join(format!("nhlms-league-r2-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("nhl_game.db");
    std::fs::write(&path, &seeded).unwrap();

    let project = Project::load(
        path.clone(),
        Platform::Ps4,
        ProjectRole::Nhlng,
        repo(META18),
        repo("defs"),
    )
    .unwrap();
    let mut session = EditSession::begin(project);
    session.set_game_id(Some("nhl18".to_string()));
    let baseline = session.bytes().to_vec();
    assert_eq!(baseline, seeded);

    for seam in ["set_field", "set_cells_int"] {
        let err = match seam {
            "set_field" => session
                .set_field("kOtt", 0, "xMjj", FieldValue::Int(30))
                .err(),
            _ => session
                .set_cells_int("kOtt", 0, &[("xMjj".to_string(), 30)])
                .err(),
        }
        .unwrap_or_else(|| panic!("{seam}: invalid 31 -> invalid 30 must be refused"));
        assert!(
            err.to_string()
                .contains("league = 30 refers to no stockleagues row"),
            "{seam}: {err}"
        );
        assert_eq!(session.bytes(), &baseline[..], "{seam}: bytes untouched");
        assert_eq!(session.undo_depth(), 0, "{seam}: no undo entry");
        assert!(!session.undo(), "{seam}: nothing to undo");
        assert!(!session.redo(), "{seam}: nothing to redo");
        assert!(!session.pending_changes(10).dirty, "{seam}: not dirty");
    }
    assert_eq!(
        std::fs::read(&path).unwrap(),
        seeded,
        "destination untouched"
    );

    // Unrelated edit on the still-dangling row: accepted, violation preserved.
    session
        .set_field("kOtt", 0, "xsnn", FieldValue::Text("QQQ".into()))
        .unwrap();
    assert_eq!(session.undo_depth(), 1);
    // Repair to a valid row: accepted; re-introducing an absent code afterwards is refused.
    session
        .set_field("kOtt", 0, "xMjj", FieldValue::Int(0))
        .unwrap();
    assert!(session
        .set_field("kOtt", 0, "xMjj", FieldValue::Int(31))
        .is_err());
    assert_eq!(session.undo_depth(), 2);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_dir(&dir);
}
