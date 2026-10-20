//! Repository-local NHL24 database coexistence proof; never installs a game file.
//! Run after scripts/arena_library/nhl24_collectors.py:
//! cargo run -p tdb-core --example nhl24_arena_stage
use std::{fs, path::PathBuf};
use tdb_core::{Table, TdbFile};

fn row(db: &TdbFile<'_>, t: &Table, key: u64) -> u32 {
    let f = t.field("vfEq").unwrap();
    let matches: Vec<_> = (0..t.current_records as u32)
        .filter(|&i| db.read_u64(t, i, f) == Some(key))
        .collect();
    assert_eq!(matches.len(), 1, "missing or ambiguous primary key");
    matches[0]
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source =
        root.join("local/arena-library/nhl24-resolved/files/gamemodes/database/t3db/nhl_game.db");
    let output = root.join("local/arena-library/nhl24-stage");
    let original = fs::read(source)?;
    let db = TdbFile::parse(&original)?;
    assert_eq!(original.len(), 13_726_652, "unqualified source database");
    assert_eq!(db.serialize(), original, "source CRC/round-trip failure");
    let arenas = db.table("StEO").unwrap();
    let teams = db.table("kOtt").unwrap();
    assert_eq!((arenas.current_records, arenas.max_records), (552, 598));
    assert_eq!(teams.current_records, 531);
    for (t, id, offset, width) in [
        (arenas, "vfEq", 1654, 10),
        (arenas, "hDRl", 1639, 10),
        (teams, "vfEq", 3522, 10),
        (teams, "buDa", 3301, 10),
    ] {
        let f = t.field(id).unwrap();
        assert_eq!(
            (f.bit_offset, f.bit_width),
            (offset, width),
            "unqualified target schema"
        );
    }
    // 96 is unused in this exhibition arena table; this is not a claim that
    // every game mode or scene registry permits it as a new runtime identity.
    let new_id = 96;
    assert!((0..arenas.current_records as u32).all(|i| db.read_u64(
        arenas,
        i,
        arenas.field("vfEq").unwrap()
    ) != Some(new_id)));
    let source_row = row(&db, arenas, 2);
    let team_row = row(&db, teams, 0);
    assert_eq!(
        db.read_u64(teams, team_row, teams.field("buDa").unwrap()),
        Some(0)
    );
    let (mut staged, added) = db.append_record("StEO")?;
    let snapshot = staged.clone();
    let edit = TdbFile::parse(&snapshot)?;
    let a = edit.table("StEO").unwrap();
    let t = edit.table("kOtt").unwrap();
    assert!(edit.copy_record(&mut staged, a, source_row, added));
    assert!(edit.set_u64(&mut staged, a, added, a.field("vfEq").unwrap(), new_id));
    assert!(edit.set_inline_str(
        &mut staged,
        a,
        added,
        a.field("bAFc").unwrap(),
        "TD Garden Library Proof"
    ));
    assert!(edit.set_u64(&mut staged, t, team_row, t.field("buDa").unwrap(), new_id));
    edit.recompute_crcs(&mut staged);
    let after = TdbFile::parse(&staged)?;
    assert_eq!(after.serialize(), staged, "staged CRC round-trip failure");
    assert_eq!(staged.len(), original.len(), "expected slack allocation");
    // Verify every existing record in every table, allowing only one field
    // in Anaheim's record. CRC words and the new arena slot are checked separately.
    let mut expected_team = original.clone();
    assert!(db.set_u64(
        &mut expected_team,
        teams,
        team_row,
        teams.field("buDa").unwrap(),
        new_id
    ));
    for old in &db.tables {
        let new = after.table(&old.id.raw()).unwrap();
        assert_eq!(
            old.current_records + u16::from(old.id.raw() == "StEO"),
            new.current_records
        );
        for i in 0..old.current_records as usize {
            let n = old.record_len_bytes as usize;
            let p = old.record_block + i * n;
            let q = new.record_block + i * n;
            assert_eq!(
                &expected_team[p..p + n],
                &staged[q..q + n],
                "unrelated record changed"
            );
        }
    }
    let new = after.table("StEO").unwrap();
    assert_eq!(
        after.read_u64(new, added, new.field("hDRl").unwrap()),
        Some(2)
    );
    // Restore the assignment as an exact byte round trip on the original DB.
    let mut reverted = expected_team;
    assert!(db.set_u64(
        &mut reverted,
        teams,
        team_row,
        teams.field("buDa").unwrap(),
        0
    ));
    db.recompute_crcs(&mut reverted);
    assert_eq!(reverted, original);
    fs::create_dir_all(&output)?;
    fs::write(output.join("nhl_game.db"), staged)?;
    fs::write(
        output.join("proof.json"),
        format!(
            r#"{{
  "status": "database_only_not_installable",
  "source_arena_id": 2, "new_arena_id": 96, "new_record_ordinal": {added},
  "assigned_team_id": 0, "preserved_team_id": 2, "shared_arena_art_id": 2,
  "original_records_preserved_except_assignment": true,
  "crc_round_trip": true, "assignment_reversal_byte_exact": true,
  "separate_scene_registered": false, "game_verified": false
}}
"#
        ),
    )?;
    println!(
        "Staged arena record 96 and team 0 assignment; all existing records verified. No installation."
    );
    Ok(())
}
