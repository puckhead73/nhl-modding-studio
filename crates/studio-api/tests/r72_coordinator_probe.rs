//! ONB-003 §4–§5 — the mod package round trip through the real [`Registry`] seam: build from a
//! saved project, inspect, move the package away and make the authoring project inaccessible,
//! install onto an independent owned base (exact intended changes, everything else byte-preserved),
//! restore; conflicts block unless allowed; wrong game / undeclared platform / tampered packages
//! refuse before any write; the Legacy trio (Xbox 360 author → PS3 and Recompiled owned bases)
//! for the shared-base database edits and the key-based dictionary edit, each qualified on its own
//! synthetic install. Every written path is a temp tree this test owns; the anchors are read only.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::mod_package as pkg;
use studio_domain::tdb_txn::FileIdentity;
use studio_domain::{BaselineStore, FieldValue, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn defs() -> String {
    repo("defs").to_string_lossy().into_owned()
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn store() -> BaselineStore {
    BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap()
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(from, to).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", from.display(), to.display()));
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-onb003-pkg-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A minimal `param.sfo` with utf-8 string keys (the shape shadPS4 installs carry).
fn sfo(pairs: &[(&str, &str)]) -> Vec<u8> {
    let (mut keys, mut vals, mut index) = (Vec::new(), Vec::new(), Vec::new());
    for (k, v) in pairs {
        let mut val = v.as_bytes().to_vec();
        val.push(0);
        index.extend_from_slice(&(keys.len() as u16).to_le_bytes());
        index.extend_from_slice(&0x0204u16.to_le_bytes());
        index.extend_from_slice(&(val.len() as u32).to_le_bytes());
        index.extend_from_slice(&(val.len() as u32).to_le_bytes());
        index.extend_from_slice(&(vals.len() as u32).to_le_bytes());
        keys.extend_from_slice(k.as_bytes());
        keys.push(0);
        vals.extend_from_slice(&val);
    }
    let key_table = 0x14 + index.len();
    let mut d = b"\x00PSF".to_vec();
    d.extend_from_slice(&0x0101_0000u32.to_le_bytes());
    d.extend_from_slice(&(key_table as u32).to_le_bytes());
    d.extend_from_slice(&((key_table + keys.len()) as u32).to_le_bytes());
    d.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
    d.extend_from_slice(&index);
    d.extend_from_slice(&keys);
    d.extend_from_slice(&vals);
    d
}

/// An owned NHL 21 PS4 game folder (the anchor database) — an independent recipient base.
fn nhl21_game(game: &Path) -> PathBuf {
    let t3db = game.join("nhl/data/common/gamemodes/database/t3db");
    copy(&repo("anchors/ps4/nhl21/databases/nhl_game.db"), &t3db.join("nhl_game.db"));
    copy(&repo("anchors/ps4/nhl21/databases/nhl_game-meta.xml"), &t3db.join("nhl_game-meta.xml"));
    std::fs::create_dir_all(game.join("sce_sys")).unwrap();
    std::fs::write(game.join("sce_sys/param.sfo"), sfo(&[("TITLE", "NHL™ 21"), ("TITLE_ID", "CUSA18182"), ("CONTENT_ID", "EP0006-CUSA18182_00-NHLICEHOCKEY2021"), ("APP_VER", "01.00")])).unwrap();
    t3db.join("nhl_game.db")
}

fn edit_two(reg: &Registry, id: u32) {
    reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(5)).unwrap();
    if !reg.summary(id).unwrap().dirty {
        reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(4)).unwrap();
    }
    reg.set_field(id, "ubPc", 3, "vjla", FieldValue::Int(2)).unwrap();
    if reg.summary(id).unwrap().undo_depth < 2 {
        reg.set_field(id, "ubPc", 3, "vjla", FieldValue::Int(1)).unwrap();
    }
}

/// Decoded cell changes between two database files (what differs, named).
fn diff(a: &Path, b: &Path, meta: &Path, platform: Platform, role: ProjectRole) -> Vec<(String, u32, String, String, String)> {
    let pa = Project::load(a, platform, role, meta, defs()).unwrap();
    let pb = Project::load(b, platform, role, meta, defs()).unwrap();
    let p = studio_domain::pending::pending_changes(pa.db_bytes(), pb.db_bytes(), pb.meta(), pb.enums(), None, usize::MAX);
    assert!(p.error.is_none(), "{:?}", p.error);
    assert!(p.tables.is_empty(), "no structural change: {:?}", p.tables);
    p.cells.iter().map(|c| (c.studio_table.clone(), c.record, c.studio_field.clone(), c.before.display.clone(), c.after.display.clone())).collect()
}

#[test]
fn nhl21_database_only_package_builds_installs_on_an_independent_base_and_restores() {
    let t = Tmp::new("nhl21");
    let reg = Registry::default();
    let ps4 = parse_platform("ps4").unwrap();
    let st = store();
    // The author's project: the NHL 21 database alone, no installation at all.
    let src = t.0.join("author-src");
    copy(&repo("anchors/ps4/nhl21/databases/nhl_game.db"), &src.join("nhl_game.db"));
    copy(&repo("anchors/ps4/nhl21/databases/nhl_game-meta.xml"), &src.join("nhl_game-meta.xml"));
    let project_dir = t.0.join("author-project");
    let sources = studio_api::dto::ImportSourcesDto { main_db: Some(s(&src.join("nhl_game.db"))), ..Default::default() };
    let opened = reg.project_create_import(&s(&project_dir), "nhl21", ps4, &sources, "", None, None, Some(&st), &defs()).unwrap();
    let id = opened.summary.project_id;
    // Unsaved edits: the inventory says so and the build refuses.
    edit_two(&reg, id);
    let inv = reg.package_inventory(id, Some(&st), &defs()).unwrap();
    assert!(inv.dirty && inv.blockers.iter().any(|b| b.contains("unsaved")));
    let req = studio_api::dto::PackageBuildRequestDto { name: "Test Ratings".into(), version: "1.0".into(), description: "two cells".into(), include: vec!["cells".into()], exclude: vec![], platforms: vec!["ps4".into()], out_path: None };
    let err = reg.package_build(id, &req, Some(&st), &defs()).err().expect("dirty").to_string();
    assert!(err.contains("unsaved"), "{err}");
    reg.project_save(id).unwrap();
    let inv = reg.package_inventory(id, Some(&st), &defs()).unwrap();
    assert!(!inv.dirty && inv.blockers.is_empty());
    let cells = inv.categories.iter().find(|c| c.category == "cells").unwrap();
    assert_eq!((cells.count, cells.status.as_str()), (2, "ready"), "{:?}", cells.items);
    assert!(cells.items.iter().all(|i| i.contains("›")), "{:?}", cells.items);
    let ps4_target = inv.targets.iter().find(|x| x.platform == "ps4").unwrap();
    assert_eq!(ps4_target.status, "supported");
    assert_eq!(inv.targets.len(), 1, "NHL 21 ships on PS4 only");
    // Nothing selected and nothing excluded: refused, never silently empty.
    let err = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto { include: vec![], ..req.clone() }, Some(&st), &defs()).err().expect("refused").to_string();
    assert!(err.contains("neither included nor explicitly excluded"), "{err}");
    let err = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto { include: vec![], exclude: vec!["cells".into()], ..req.clone() }, Some(&st), &defs()).err().expect("refused").to_string();
    assert!(err.contains("nothing selected"), "{err}");
    // Build.
    let built = reg.package_build(id, &req, Some(&st), &defs()).unwrap();
    let built_path = PathBuf::from(&built.path);
    assert!(built_path.starts_with(project_dir.join("exports")), "{}", built.path);
    assert_eq!(built.len, std::fs::metadata(&built_path).unwrap().len());
    assert_eq!(built.categories.iter().find(|c| c.category == "cells").unwrap().count, 2);
    // The same build again never overwrites.
    let err = reg.package_build(id, &req, Some(&st), &defs()).err().expect("exists").to_string();
    assert!(err.contains("already exists"), "{err}");
    // Inspect: identity, declared target, contents; no absolute paths anywhere in the package.
    let ins = reg.package_inspect(&built.path).unwrap();
    assert_eq!((ins.name.as_str(), ins.version.as_str(), ins.game_id.as_str(), ins.author_platform.as_str()), ("Test Ratings", "1.0", "nhl21", "ps4"));
    assert_eq!(ins.cells.len(), 2);
    assert_eq!(ins.entries.len(), 1, "edits.json only; no payloads, no baseline dump");
    let raw = std::fs::read(&built_path).unwrap();
    let text = String::from_utf8_lossy(&raw);
    assert!(!text.contains(&s(&t.0).replace('\\', "\\\\")) && !text.contains(&s(&t.0)), "no private absolute path leaks into the package");
    assert!(!text.contains("author-project"), "no project folder name leaks into the package");
    assert!(built.len < 64 * 1024, "the package carries edits, not the 11 MB database ({} bytes)", built.len);
    assert!(reg.close_project(id));

    // Move the package to an unrelated place and make the author's project inaccessible.
    let moved = t.0.join("downloads").join("Test Ratings-1.0.nhlmod");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::rename(&built_path, &moved).unwrap();
    std::fs::remove_dir_all(&project_dir).unwrap();
    std::fs::remove_dir_all(&src).unwrap();

    // An independent owned base.
    let game = t.0.join("recipient/games/NHL21");
    let base_db = nhl21_game(&game);
    let savedata = t.0.join("recipient/shadps4/savedata");
    std::fs::create_dir_all(&savedata).unwrap();
    let base_before = FileIdentity::of_file(&base_db).unwrap();
    let records = t.0.join("recipient/installs");
    // Wrong game: refused, nothing written.
    let err = reg.package_preflight(&s(&moved), "nhl20", ps4, &s(&game), Some(&s(&savedata)), None, false, &s(&records), &defs()).err().expect("wrong game").to_string();
    assert!(err.contains("not NHL 20") && err.contains("cross-game"), "{err}");
    // Undeclared platform: refused.
    let err = reg.package_preflight(&s(&moved), "nhl21", parse_platform("ps3").unwrap(), &s(&game), None, None, false, &s(&records), &defs()).err().expect("undeclared").to_string();
    assert!(err.contains("declares no ps3 target") || err.contains("not a shipping target"), "{err}");
    assert_eq!(FileIdentity::of_file(&base_db).unwrap(), base_before);
    // Preflight on the right target: a replace of the main database only.
    let plan = reg.package_preflight(&s(&moved), "nhl21", ps4, &s(&game), Some(&s(&savedata)), None, false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    assert_eq!(plan.applied, 2);
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].state, "replace");
    assert_eq!(plan.items[0].destination_path, s(&base_db));
    assert!(plan.requirements.iter().any(|r| r.role == "main_db" && r.status == "found"));
    assert_eq!(FileIdentity::of_file(&base_db).unwrap(), base_before, "preflight writes nothing");
    assert!(!records.join("staging-").exists());
    // Apply.
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let record_dir = PathBuf::from(&applied.record_dir);
    assert!(record_dir.join("record.json").is_file() && record_dir.join("backups").is_dir());
    assert_ne!(FileIdentity::of_file(&base_db).unwrap(), base_before);
    let meta = game.join("nhl/data/common/gamemodes/database/t3db/nhl_game-meta.xml");
    let changed = diff(&repo("anchors/ps4/nhl21/databases/nhl_game.db"), &base_db, &meta, ps4, ProjectRole::Nhlng);
    assert_eq!(changed.len(), 2, "exactly the two intended cells differ: {changed:?}");
    assert!(changed.iter().all(|c| c.0 == "ubPc" && c.2 == "vjla" && (c.1 == 0 || c.1 == 3)), "{changed:?}");
    // A second install of the same package: everything already present, nothing to do.
    let again = reg.package_preflight(&s(&moved), "nhl21", ps4, &s(&game), Some(&s(&savedata)), None, false, &s(&records), &defs()).unwrap();
    assert!(!again.installable && again.already_present == 2, "{:?}", again.reasons);
    // History + restore.
    let installs = reg.package_installs(&s(&records)).unwrap();
    assert_eq!(installs.len(), 1);
    assert!(installs[0].restorable && installs[0].package_id == ins.package_id);
    // A foreign change after the installation: restore refuses and touches nothing.
    let mut bytes = std::fs::read(&base_db).unwrap();
    let keep = bytes.clone();
    bytes[1000] ^= 0x55;
    std::fs::write(&base_db, &bytes).unwrap();
    let err = reg.package_restore(&applied.record_dir).err().expect("refused").to_string();
    assert!(err.contains("changed after the installation"), "{err}");
    assert_eq!(std::fs::read(&base_db).unwrap(), bytes);
    std::fs::write(&base_db, &keep).unwrap();
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(FileIdentity::of_file(&base_db).unwrap(), base_before, "the base is byte-identical again");
    assert!(reg.package_installs(&s(&records)).unwrap()[0].state == "restored");

    // A conflicting recipient edit at one of the cells: blocked unless allowed.
    {
        let rid = reg.project_create_import(&s(&t.0.join("recipient-project")), "nhl21", ps4, &studio_api::dto::ImportSourcesDto { main_db: Some(s(&base_db)), ..Default::default() }, "", None, None, Some(&st), &defs()).unwrap().summary.project_id;
        reg.set_field(rid, "ubPc", 0, "vjla", FieldValue::Int(7)).unwrap();
        if !reg.summary(rid).unwrap().dirty {
            reg.set_field(rid, "ubPc", 0, "vjla", FieldValue::Int(6)).unwrap();
        }
        let out = base_db.with_extension("conflicted.db");
        reg.save(rid, &s(&out)).unwrap();
        assert!(reg.close_project(rid));
        std::fs::rename(&out, &base_db).unwrap();
    }
    let plan = reg.package_preflight(&s(&moved), "nhl21", ps4, &s(&game), Some(&s(&savedata)), None, false, &s(&records), &defs()).unwrap();
    assert!(!plan.installable);
    assert_eq!(plan.conflicts.len(), 1, "{:?}", plan.conflicts);
    assert!(plan.conflicts[0].contains("the recipient's own change"), "{:?}", plan.conflicts);
    assert!(plan.reasons.iter().any(|r| r.contains("conflict")), "{:?}", plan.reasons);
    let plan = reg.package_preflight(&s(&moved), "nhl21", ps4, &s(&game), Some(&s(&savedata)), None, true, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    assert_eq!(plan.applied, 2);
    assert!(plan.conflicts[0].contains("replacing it as allowed"));
}

#[test]
fn tampered_and_malformed_packages_are_rejected_before_any_write() {
    let t = Tmp::new("tamper");
    let reg = Registry::default();
    let ps3 = parse_platform("ps3").unwrap();
    let st = store();
    let opened = reg.project_create_fresh(&s(&t.0.join("p")), "nhl14", ps3, "", None, None, &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(&reg, id);
    reg.project_save(id).unwrap();
    let req = studio_api::dto::PackageBuildRequestDto { name: "T".into(), version: "1".into(), description: String::new(), include: vec!["cells".into()], exclude: vec![], platforms: vec![], out_path: Some(s(&t.0.join("out/t.nhlmod"))) };
    let built = reg.package_build(id, &req, Some(&st), &defs()).unwrap();
    assert!(reg.close_project(id));
    let good = std::fs::read(&built.path).unwrap();
    reg.package_inspect(&built.path).unwrap();
    let bad = t.0.join("bad.nhlmod");
    let refuse = |label: &str, bytes: &[u8]| {
        std::fs::write(&bad, bytes).unwrap();
        let err = reg.package_inspect(&s(&bad)).err().unwrap_or_else(|| panic!("{label}: accepted")).to_string();
        err
    };
    // One flipped byte in the edits document.
    let mut flipped = good.clone();
    let off = good.windows(9).position(|w| w == b"\"schema\":").unwrap();
    flipped[off + 12] ^= 0x01;
    let err = refuse("flipped byte", &flipped);
    assert!(err.contains("CRC") || err.contains("hash"), "{err}");
    // Truncated.
    let err = refuse("truncated", &good[..good.len() - 40]);
    assert!(err.contains("not a valid Studio mod package"), "{err}");
    // Not a zip at all.
    let err = refuse("garbage", b"this is not a package");
    assert!(err.contains("not a valid Studio mod package"), "{err}");
    // An extra entry the manifest does not list (something smuggled in).
    let archive = pkg::ZipArchive::parse(good.clone(), &pkg::Limits::default()).unwrap();
    let mut entries: Vec<(String, Vec<u8>)> = archive.entries.iter().map(|e| (e.name.clone(), archive.read(&e.name).unwrap())).collect();
    entries.push(("payloads/extra.bin".into(), b"smuggled".to_vec()));
    let err = refuse("unlisted entry", &pkg::write_zip(&entries).unwrap());
    assert!(err.contains("not listed in the manifest"), "{err}");
    // A traversal name cannot even be written, and a hand-patched one is refused on read.
    assert!(pkg::write_zip(&[("../escape.txt".to_string(), b"x".to_vec())]).is_err());
    let mut patched = good.clone();
    let name_off = patched.windows(10).position(|w| w == b"edits.json").unwrap();
    patched[name_off..name_off + 10].copy_from_slice(b"../../.js\0");
    let err = refuse("patched traversal name", &patched);
    assert!(err.contains("refused") || err.contains("not a valid"), "{err}");
    // Limits: an entry count or size above the limit.
    let err = pkg::inspect_package_bytes(good.clone(), &pkg::Limits { max_entries: 1, ..pkg::Limits::default() }).unwrap_err().to_string();
    assert!(err.contains("entries exceeds"), "{err}");
    let err = pkg::inspect_package_bytes(good.clone(), &pkg::Limits { max_total_bytes: 100, ..pkg::Limits::default() }).unwrap_err().to_string();
    assert!(err.contains("limit"), "{err}");
    // A package whose manifest was edited (identity no longer matches its contents).
    let mut edited: Vec<(String, Vec<u8>)> = archive.entries.iter().map(|e| (e.name.clone(), archive.read(&e.name).unwrap())).collect();
    for (n, b) in edited.iter_mut() {
        if n == "manifest.json" {
            let text = String::from_utf8(b.clone()).unwrap().replace("\"version\": \"1\"", "\"version\": \"2\"");
            *b = text.into_bytes();
        }
    }
    let err = refuse("edited manifest", &pkg::write_zip(&edited).unwrap());
    assert!(err.contains("package id does not match"), "{err}");
}

/// Owned PS3 Legacy install (RPCS3-shaped) from the PS3 anchors.
fn ps3_legacy_install(base: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let root = base.join("rpcs3-legacy");
    let usrdir = root.join("dev_hdd0/game/BLUS31540/USRDIR/data/db");
    let a = repo("anchors/ps3/nhllegacy");
    copy(&a.join("databases/nhlng.db"), &usrdir.join("nhlng.db"));
    copy(&a.join("databases/nhlng-meta.xml"), &usrdir.join("nhlng-meta.xml"));
    copy(&a.join("databases/nhl_eng_us.db"), &usrdir.join("nhl_eng_us.db"));
    let roster = root.join("dev_hdd0/home/00000001/savedata/BLUS315400200 - ROSTER/SYS-DATA");
    copy(&a.join("savefiles/BLUS315400200 - ROSTER/SYS-DATA"), &roster);
    (root, usrdir.join("nhlng.db"), usrdir.join("nhl_eng_us.db"), roster)
}

/// Owned Recompiled install (loose Xbox-layout game folder + explicit save folder) from the Xbox
/// 360 Legacy anchors — the Recompiled baseline is derived from them (R128).
fn recomp_install(base: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf, PathBuf) {
    let game = base.join("recomp/game");
    let a = repo("anchors/xbox360/nhllegacy");
    copy(&a.join("databases/nhlng.db"), &game.join("db/nhlng.db"));
    copy(&a.join("databases/nhlng-meta.xml"), &game.join("db/nhlng-meta.xml"));
    copy(&a.join("databases/nhl_eng_us.db"), &game.join("db/nhl_eng_us.db"));
    let saves = base.join("recomp/saves");
    let roster = saves.join("ROSTER 20260627060052/ROSTER 20260627060052");
    copy(&a.join("savefiles/ROSTER 20260627060052/ROSTER 20260627060052"), &roster);
    let db = game.join("db/nhlng.db");
    let dict = game.join("db/nhl_eng_us.db");
    (game, saves, db, dict, roster)
}

#[test]
fn legacy_xbox_package_installs_on_ps3_and_recompiled_owned_bases() {
    let t = Tmp::new("legacy");
    let reg = Registry::default();
    let st = store();
    let xbox = parse_platform("xbox").unwrap();
    let ps3 = parse_platform("ps3").unwrap();
    let recomp = parse_platform("recomp").unwrap();
    // Author on Xbox 360 from the bundled baseline, no installation: a cell and a dictionary text.
    let project_dir = t.0.join("author");
    let opened = reg.project_create_fresh(&s(&project_dir), "nhllegacy", xbox, "", None, None, &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    reg.set_field(id, "ubPc", 2, "vjla", FieldValue::Int(3)).unwrap();
    if !reg.summary(id).unwrap().dirty {
        reg.set_field(id, "ubPc", 2, "vjla", FieldValue::Int(2)).unwrap();
    }
    reg.dictionary_retext_set(id, "TXT_LEAGUE_NORWAY", "Nordic").unwrap();
    reg.project_save(id).unwrap();
    let inv = reg.package_inventory(id, Some(&st), &defs()).unwrap();
    for p in ["xbox", "ps3", "recomp"] {
        let tg = inv.targets.iter().find(|x| x.platform == p).unwrap_or_else(|| panic!("{p} missing: {:?}", inv.targets.iter().map(|x| x.platform.clone()).collect::<Vec<_>>()));
        assert_eq!(tg.status, "supported", "{p}: {:?}", tg.components.iter().map(|c| format!("{} {} {}", c.category, c.status, c.reason)).collect::<Vec<_>>());
        let cells = tg.components.iter().find(|c| c.category == "cells").unwrap();
        assert_eq!(cells.status, "supported");
        if p != "xbox" {
            assert!(cells.reason.contains("byte-identical"), "{}", cells.reason);
        }
    }
    let req = studio_api::dto::PackageBuildRequestDto { name: "Legacy Nordic".into(), version: "1.0".into(), description: String::new(), include: vec!["cells".into(), "dictionary".into()], exclude: vec![], platforms: vec!["xbox".into(), "ps3".into(), "recomp".into()], out_path: Some(s(&t.0.join("legacy-nordic.nhlmod"))) };
    let built = reg.package_build(id, &req, Some(&st), &defs()).unwrap();
    assert!(reg.close_project(id));
    std::fs::remove_dir_all(&project_dir).unwrap();
    let ins = reg.package_inspect(&built.path).unwrap();
    assert_eq!(ins.targets.len(), 3);
    assert_eq!((ins.cells.len(), ins.dictionary.len()), (1, 1));
    assert_eq!((ins.dictionary[0].key.as_str(), ins.dictionary[0].before.as_str(), ins.dictionary[0].after.as_str()), ("TXT_LEAGUE_NORWAY", "Norway", "Nordic"));
    let records = t.0.join("installs");

    // --- PS3 (its own anchors; a different dictionary file, the same database bytes).
    let (root, db, dict, roster) = ps3_legacy_install(&t.0);
    let before = (FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&dict).unwrap(), FileIdentity::of_file(&roster).unwrap());
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, None, false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let roles: Vec<&str> = plan.items.iter().map(|i| i.role.as_str()).collect();
    assert!(roles.contains(&"database") && roles.contains(&"roster-save") && roles.contains(&"dictionary"), "{roles:?}");
    assert_eq!(plan.applied, 2);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let meta = db.with_file_name("nhlng-meta.xml");
    let changed = diff(&repo("anchors/ps3/nhllegacy/databases/nhlng.db"), &db, &meta, ps3, ProjectRole::Nhlng);
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert_eq!((changed[0].0.as_str(), changed[0].1, changed[0].2.as_str()), ("ubPc", 2, "vjla"));
    let rchanged = diff(&repo("anchors/ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA"), &roster, &meta, ps3, ProjectRole::Roster);
    assert_eq!(rchanged.len(), 1, "the aligned roster save mirrors the one cell: {rchanged:?}");
    let d = studio_domain::tdb_loc::LocDictionary::parse(&std::fs::read(&dict).unwrap()).unwrap();
    assert_eq!(d.text("TXT_LEAGUE_NORWAY"), Some("Nordic"));
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&dict).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);

    // --- Recompiled (loose Xbox-layout folder + explicit saves).
    let (game, saves, db, dict, roster) = recomp_install(&t.0);
    let before = (FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&dict).unwrap(), FileIdentity::of_file(&roster).unwrap());
    let plan = reg.package_preflight(&built.path, "legacyrecomp", recomp, &s(&game), Some(&s(&saves)), None, false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let meta = db.with_file_name("nhlng-meta.xml");
    let changed = diff(&repo("anchors/xbox360/nhllegacy/databases/nhlng.db"), &db, &meta, recomp, ProjectRole::Nhlng);
    assert_eq!(changed.len(), 1, "{changed:?}");
    let rchanged = diff(&repo("anchors/xbox360/nhllegacy/savefiles/ROSTER 20260627060052/ROSTER 20260627060052"), &roster, &meta, recomp, ProjectRole::Roster);
    assert_eq!(rchanged.len(), 1, "{rchanged:?}");
    let d = studio_domain::tdb_loc::LocDictionary::parse(&std::fs::read(&dict).unwrap()).unwrap();
    assert_eq!(d.text("TXT_LEAGUE_NORWAY"), Some("Nordic"));
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&dict).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);

    // --- Xbox 360 (the author's platform): a loose install folder and an explicit save folder.
    let xgame = t.0.join("xbox/game");
    let a = repo("anchors/xbox360/nhllegacy");
    copy(&a.join("databases/nhlng.db"), &xgame.join("db/nhlng.db"));
    copy(&a.join("databases/nhlng-meta.xml"), &xgame.join("db/nhlng-meta.xml"));
    copy(&a.join("databases/nhl_eng_us.db"), &xgame.join("db/nhl_eng_us.db"));
    let xsaves = t.0.join("xbox/saves");
    copy(&a.join("savefiles/ROSTER 20260627060052/ROSTER 20260627060052"), &xsaves.join("ROSTER 20260627060052/ROSTER 20260627060052"));
    let plan = reg.package_preflight(&built.path, "nhllegacy", xbox, &s(&xgame), Some(&s(&xsaves)), None, false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let changed = diff(&a.join("databases/nhlng.db"), &xgame.join("db/nhlng.db"), &xgame.join("db/nhlng-meta.xml"), xbox, ProjectRole::Nhlng);
    assert_eq!(changed.len(), 1, "{changed:?}");
    assert_eq!(reg.package_installs(&s(&records)).unwrap().len(), 3);
}

#[test]
fn a_package_authored_on_a_platform_specific_base_declares_only_that_platform() {
    // NHL 15: the PS3 and Xbox 360 baselines share one nhlng.db (measured: supported both ways),
    // the PS4 nhl_game.db is a different database (measured: unsupported, never inferred).
    let t = Tmp::new("nhl15");
    let reg = Registry::default();
    let st = store();
    let ps3 = parse_platform("ps3").unwrap();
    let opened = reg.project_create_fresh(&s(&t.0.join("p")), "nhl15", ps3, "", None, None, &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(&reg, id);
    reg.project_save(id).unwrap();
    let inv = reg.package_inventory(id, Some(&st), &defs()).unwrap();
    let ps4 = inv.targets.iter().find(|x| x.platform == "ps4").unwrap();
    assert_eq!(ps4.status, "unsupported");
    assert!(ps4.components.iter().any(|c| c.category == "cells" && c.status == "unsupported" && c.reason.contains("differs from the author's")), "{:?}", ps4.components.iter().map(|c| c.reason.clone()).collect::<Vec<_>>());
    let xbox = inv.targets.iter().find(|x| x.platform == "xbox").unwrap();
    assert_eq!(xbox.status, "supported", "{:?}", xbox.components.iter().map(|c| c.reason.clone()).collect::<Vec<_>>());
    // Declaring PS4 anyway is refused; the author's platform alone builds.
    let req = studio_api::dto::PackageBuildRequestDto { name: "N15".into(), version: "1".into(), description: String::new(), include: vec!["cells".into()], exclude: vec![], platforms: vec!["ps3".into(), "ps4".into()], out_path: Some(s(&t.0.join("n15.nhlmod"))) };
    let err = reg.package_build(id, &req, Some(&st), &defs()).err().expect("refused").to_string();
    assert!(err.contains("ps4: cells"), "{err}");
    let built = reg.package_build(id, &studio_api::dto::PackageBuildRequestDto { platforms: vec!["ps3".into()], ..req }, Some(&st), &defs()).unwrap();
    let ins = reg.package_inspect(&built.path).unwrap();
    assert_eq!(ins.targets.len(), 1);
    assert_eq!(ins.targets[0].platform, "ps3");
    assert!(reg.close_project(id));
}

#[test]
fn coordinator_roster_only_package_needs_no_game_installation() {
    let t = Tmp::new("r72-coordinator-roster-only");
    let reg = Registry::default();
    let st = store();
    let xbox = parse_platform("xbox").unwrap();
    let anchor = repo("anchors/xbox360/nhllegacy/savefiles/ROSTER 20260627060052/ROSTER 20260627060052");
    let sources = studio_api::dto::ImportSourcesDto { roster_save: Some(s(&anchor)), ..Default::default() };
    let opened = reg.project_create_import(&s(&t.0.join("author")), "nhllegacy", xbox, &sources, "", None, None, Some(&st), &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(&reg, id);
    reg.project_save(id).unwrap();
    let req = studio_api::dto::PackageBuildRequestDto { name: "Roster only".into(), version: "1".into(), description: String::new(), include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()], out_path: None };
    let built = reg.package_build(id, &req, Some(&st), &defs()).unwrap();
    let ins = pkg::inspect_package(Path::new(&built.path), &pkg::Limits::default()).unwrap();
    assert!(!ins.edits.cells.is_empty() && ins.edits.cells.iter().all(|c| c.file == "roster"));
    let saves = t.0.join("recipient-saves");
    let roster = saves.join("ROSTER 20260627060052/ROSTER 20260627060052");
    copy(&anchor, &roster);
    let before = FileIdentity::of_file(&roster).unwrap();
    let empty_root = reg.package_preflight(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), Some(&s(&roster)), false, &s(&t.0.join("records")), &defs());
    let save_root = reg.package_preflight(&built.path, "nhllegacy", xbox, &s(&saves), Some(&s(&saves)), Some(&s(&roster)), false, &s(&t.0.join("records")), &defs());
    let describe = |p: &Result<studio_api::dto::InstallPlanDto, studio_api::ApiError>| match p { Ok(p) => format!("installable={} reasons={:?}", p.installable, p.reasons), Err(e) => e.to_string() };
    eprintln!("Empty game root: {}", describe(&empty_root));
    eprintln!("Save folder as root: {}", describe(&save_root));
    assert_eq!(FileIdentity::of_file(&roster).unwrap(), before);
    assert!(empty_root.as_ref().is_ok_and(|p| p.installable), "A roster-only recipient must not need a game installation: {}", describe(&empty_root));
}
