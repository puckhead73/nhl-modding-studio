//! Mod packages over databases whose rows do not have unique keys or shift: a roster table that
//! carries the same key on two rows (seen in an imported NHL 08 database) resolves each edited row
//! by its fingerprint, and an NHL 07 career-row removal combined with an edit to a surviving row
//! packages, installs and restores.
use std::path::{Path, PathBuf};

use studio_api::dto::{ImportSourcesDto, PackageBuildRequestDto};
use studio_api::Registry;
use studio_domain::{BaselineStore, FieldValue, Platform};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn defs() -> String {
    s(&repo("defs"))
}
fn store() -> BaselineStore {
    BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap()
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(from, to).unwrap();
}

/// `(row, field, value)` raw reads of `ulGe` in an NHL 08 database.
fn roster_field(bytes: &[u8], row: u32, field: &str) -> u64 {
    let db = tdb_core::TdbFile::parse(bytes).unwrap();
    let t = db.table("ulGe").unwrap();
    db.read_u64(t, row, t.field(field).unwrap()).unwrap()
}

#[test]
fn duplicate_roster_keys_resolve_by_row_fingerprint() {
    let t = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    let ps3 = Platform::Ps3;
    // An NHL 08 database whose roster rows 624 and 625 share key 701 (as the tester's import did).
    let mut base = std::fs::read(repo("anchors/ps3/nhl08/databases/nhlng.db")).unwrap();
    {
        let view = base.clone();
        let db = tdb_core::TdbFile::parse(&view).unwrap();
        let table = db.table("ulGe").unwrap();
        let key = table.field("XWot").unwrap();
        assert_eq!(db.read_u64(table, 624, key), Some(701));
        let start = table.record_block + 625 * table.record_len_bytes as usize;
        tdb_core::write_bits(&mut base[start..start + table.record_len_bytes as usize], key.bit_offset, key.bit_width, 701, db.endian);
        db.recompute_crcs(&mut base);
    }
    let source = t.path().join("source/nhlng.db");
    copy(&repo("anchors/ps3/nhl08/databases/nhlng.db"), &source);
    std::fs::write(&source, &base).unwrap();
    let sources = ImportSourcesDto {
        main_db: Some(s(&source)),
        meta: Some(s(&repo("anchors/ps3/nhl08/databases/nhlng-meta.xml"))),
        ..Default::default()
    };
    let st = store();
    let id = reg
        .project_create_import(&s(&t.path().join("author")), "nhl08", ps3, &sources, "", None, None, Some(&st), &defs())
        .unwrap()
        .summary
        .project_id;
    // Edit both rows that share the key.
    let status: Vec<u64> = [624, 625].iter().map(|&r| roster_field(&base, r, "jZSh")).collect();
    for (row, old) in [624u32, 625].iter().zip(&status) {
        reg.set_field(id, "eGlu", *row, "hSZj", FieldValue::Int(if *old == 0 { 1 } else { 0 })).unwrap();
    }
    reg.project_save(id).unwrap();
    let req = PackageBuildRequestDto { name: "Dup keys".into(), version: "1".into(), description: String::new(), include: vec!["cells".into()], exclude: vec![], platforms: vec!["ps3".into()], out_path: Some(s(&t.path().join("dup.nhlmod"))) };
    let built = reg.package_build(id, &req, Some(&st), &defs()).unwrap();

    // Install onto an owned copy of the same database.
    let game = t.path().join("hdd/game/BLUS30062/USRDIR/db");
    copy(&source, &game.join("nhlng.db"));
    copy(&repo("anchors/ps3/nhl08/databases/nhlng-meta.xml"), &game.join("nhlng-meta.xml"));
    let records = t.path().join("records");
    let plan = reg
        .package_preflight(&built.path, "nhl08", ps3, &s(&t.path().join("hdd")), None, None, false, &s(&records), &defs())
        .unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let after = std::fs::read(game.join("nhlng.db")).unwrap();
    for (row, old) in [624u32, 625].iter().zip(&status) {
        assert_ne!(roster_field(&after, *row, "jZSh"), *old, "row {row}");
        assert_eq!(roster_field(&after, *row, "XWot"), 701);
    }
    assert_eq!(reg.package_restore(&applied.record_dir).unwrap().state, "restored");
    assert_eq!(std::fs::read(game.join("nhlng.db")).unwrap(), base);
}

#[test]
fn nhl07_career_removal_with_an_edited_survivor_packages_and_installs() {
    let t = tempfile::tempdir().unwrap();
    let reg = Registry::default();
    let st = store();
    let root = t.path().join("project");
    let id = reg.project_create_fresh(&s(&root), "nhl07", Platform::Xbox, "", None, None, &st, &defs()).unwrap().summary.project_id;
    // Remove rows 2, 4 and 5 of the skater season table, and edit the surviving row 3 (now 2).
    for row in [5, 4, 2] {
        reg.delete_career_row(id, "tsSy", row).unwrap();
    }
    // points (`myyY`, canonical `Yyym`) of the row that now sits at index 2.
    reg.set_field(id, "tsSy", 2, "Yyym", FieldValue::Int(77)).unwrap();
    reg.project_save(id).unwrap();
    let req = PackageBuildRequestDto { name: "Career edit".into(), version: "1".into(), description: String::new(), include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()], out_path: Some(s(&t.path().join("career.nhlmod"))) };
    let built = reg.package_build(id, &req, Some(&st), &defs()).unwrap_or_else(|e| panic!("{e}"));
    let review = reg.package_inspect(&built.path).unwrap();
    let edited = review.changes.iter().filter(|c| c.after == "Edited").count();
    let removed = review.changes.iter().filter(|c| c.after == "Removed").count();
    assert_eq!((removed, edited), (3, 1));

    let game = t.path().join("game");
    std::fs::create_dir_all(game.join("db")).unwrap();
    std::fs::write(game.join("default.xex"), b"XEX2").unwrap();
    for file in ["NHL07NG.db", "NHL07NG-meta.xml"] {
        copy(&repo(&format!("anchors/xbox360/nhl07/databases/{file}")), &game.join("db").join(file));
    }
    let db = game.join("db/NHL07NG.db");
    let original = std::fs::read(&db).unwrap();
    let expected = std::fs::read(root.join("working/db/NHL07NG.db")).unwrap();
    let records = s(&t.path().join("records"));
    let plan = reg.package_preflight(&built.path, "nhl07", Platform::Xbox, &s(&game), None, None, false, &records, &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons.iter().take(5).collect::<Vec<_>>());
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert!(std::fs::read(&db).unwrap() == expected, "installed database equals the author's saved database");
    assert_eq!(reg.package_restore(&applied.record_dir).unwrap().state, "restored");
    assert!(std::fs::read(&db).unwrap() == original);
}
