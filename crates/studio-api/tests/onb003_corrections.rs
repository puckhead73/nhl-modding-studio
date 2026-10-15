//! R71 coordinator corrections (docs/validation/user-beta-pass-20261001/r71-review): the behavior
//! each finding requires, asserted on owned temp trees through the real [`Registry`] seam.
//!
//! R1 stable identity — rows are found by key values (missing / duplicate keys refuse, nothing
//!    written); a position-bound (edits schema 1) package is refused by the installer.
//! R2 file roles — a database-only package never asks for or touches a roster (PS4 included, even
//!    when a roster is named); `both` edits check and apply each file on its own; a stale roster
//!    choice and an unmade choice among several saves refuse.
//! R3 recovery — a fault while applying rolls back and records it; a split apply is recovered by
//!    Restore; an interrupted restore is retried, also from a fresh process (reopen); the save-index
//!    header is in the reviewed plan and removed inside the restore transaction; the project
//!    deployment route shares the behavior.
//! R4 assets-only — Save is valid; staged outputs, sources and intent survive close / reopen /
//!    relocation and build a package without a database.
//! R5 asset roots — a native texture (synthetic RW4ps3 logo package) installs into the platform's
//!    asset root and restores exactly; wrong root, missing extraction, an incompatible file and a
//!    changed target refuse or conflict, nothing written.
//! R6 qualification — an Xbox 360 texture package installs on a Recompiled tree (registered shared
//!    layout) and is refused for PS3 with the exact missing-profile reason.
//!
//! Every written path is a temp tree this test owns; the anchors are read only. Nothing here is
//! runtime evidence.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::in_place_save::{Event, Hooks, TEMP_SUFFIX};
#[allow(unused_imports)]
use std::path::Path as _P;
use studio_domain::mod_package as pkg;
use studio_domain::tdb_txn::FileIdentity;
use studio_domain::{BaselineStore, EditSession, FieldValue, Platform, Project, ProjectRole};

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
        let d = std::env::temp_dir().join(format!("nhlms-r71-{tag}-{}", std::process::id()));
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

fn req(name: &str, include: &[&str], platforms: &[&str]) -> studio_api::dto::PackageBuildRequestDto {
    studio_api::dto::PackageBuildRequestDto { name: name.into(), version: "1".into(), description: String::new(), include: include.iter().map(|x| x.to_string()).collect(), exclude: vec![], platforms: platforms.iter().map(|x| x.to_string()).collect(), out_path: None }
}

/// A minimal PS4 `param.sfo` (title id + content id) so discovery recognises an owned game folder.
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

/// Owned PS3 Legacy install (RPCS3-shaped) from the PS3 anchors: `(root, db, dict, roster)`.
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

/// An owned Recompiled content tree whose save-data index needs a header for the roster:
/// `(game, db, roster, header_path)` — the header does not exist yet.
fn recomp_content_tree(base: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let (game, _, db, _, old_roster) = recomp_install(base);
    let title = base.join("content/ABCDEF0123456789/454109BE");
    let roster = title.join("00000001/ROSTER 20260627060052/ROSTER 20260627060052");
    copy(&old_roster, &roster);
    std::fs::create_dir_all(title.join("Headers/00000001")).unwrap();
    let header = title.join("Headers/00000001/ROSTER 20260627060052.header");
    assert!(!header.exists());
    (game, db, roster, header)
}

/// A fresh Xbox 360 Legacy project with two cell edits, saved and packaged for the given platforms.
fn legacy_xbox_package(reg: &Registry, dir: &Path, platforms: &[&str]) -> studio_api::dto::PackageBuildDto {
    let st = store();
    let xbox = parse_platform("xbox").unwrap();
    let opened = reg.project_create_fresh(&s(dir), "nhllegacy", xbox, "", None, None, &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(reg, id);
    reg.project_save(id).unwrap();
    let built = reg.package_build(id, &req("Legacy cells", &["cells"], platforms), Some(&st), &defs()).unwrap();
    assert!(reg.close_project(id));
    built
}

fn temp_path(p: &Path) -> PathBuf {
    let mut n = p.file_name().unwrap().to_os_string();
    n.push(TEMP_SUFFIX);
    p.with_file_name(n)
}

/// Fault injection: fail at one publication event; optionally make the revert of one path fail
/// (a directory at its temp name) so the rollback splits.
#[derive(Clone, Copy, PartialEq, Eq)]
enum At {
    BeforeWrite,
    AfterRename,
    BeforeRemove,
    AfterRemoved,
}
struct Fault {
    fail_at: (At, PathBuf),
    break_revert_of: Option<PathBuf>,
}
impl Hooks for Fault {
    fn event(&mut self, e: &Event) -> std::io::Result<()> {
        let same = |l: &str, p: &Path| Path::new(l) == p;
        let hit = match e {
            Event::BeforeWrite(l) => self.fail_at.0 == At::BeforeWrite && same(l, &self.fail_at.1),
            Event::AfterRename(l) => self.fail_at.0 == At::AfterRename && same(l, &self.fail_at.1),
            Event::BeforeRemove(l) => self.fail_at.0 == At::BeforeRemove && same(l, &self.fail_at.1),
            Event::AfterRemoved(l) => self.fail_at.0 == At::AfterRemoved && same(l, &self.fail_at.1),
            _ => false,
        };
        if hit {
            return Err(std::io::Error::other("injected fault"));
        }
        if let (Event::BeforeRevert(l), Some(p)) = (e, &self.break_revert_of) {
            if same(l, p) {
                let _ = std::fs::create_dir(temp_path(p));
            }
        }
        Ok(())
    }
}

fn png(rgb: [u8; 3], w: u32, h: u32) -> Vec<u8> {
    studio_domain::DecodedImage { width: w, height: h, rgba: (0..w * h).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect() }.to_png().unwrap()
}

fn decode_data_url(url: &str) -> studio_domain::DecodedImage {
    use base64::Engine;
    let b64 = url.split_once(',').unwrap().1;
    let png = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
    studio_domain::decode_source_image_bytes(&png).unwrap()
}

/// A logo-layout RW4ps3 ARGB8 texture inside an X-C-shaped BIGF package (as `league_logo_packages.rs`
/// builds it): a native PS3 loose texture container the writer re-encodes into.
fn synthetic_ps3_logo_package(w: u32, h: u32, seed: u8) -> Vec<u8> {
    let hdr = 0x200usize;
    let slot: Vec<u8> = (0..w * h).flat_map(|i| [255u8, (i % 64) as u8 * 4, (i / 64) as u8 * 8, seed]).collect();
    let mut t = vec![0u8; hdr + slot.len()];
    t[..7].copy_from_slice(b"\x89RW4ps3");
    t[0x20..0x24].copy_from_slice(&2u32.to_be_bytes());
    t[0x30..0x34].copy_from_slice(&0x1B4u32.to_be_bytes());
    t[0x44..0x48].copy_from_slice(&(hdr as u32).to_be_bytes());
    let d = 0x17C;
    for (k, sg) in [0x6007u32, 0x6007, 0x8035].iter().enumerate() {
        t[d + 32 + k * 4..d + 36 + k * 4].copy_from_slice(&sg.to_be_bytes());
    }
    t[d + 48..d + 50].copy_from_slice(&(w as u16).to_be_bytes());
    t[d + 50..d + 52].copy_from_slice(&(h as u16).to_be_bytes());
    t[d + 52] = 1;
    let tb = 0x1B4;
    for (k, v) in [0u32, 0, slot.len() as u32, 0x80, 1].iter().enumerate() {
        t[tb + k * 4..tb + k * 4 + 4].copy_from_slice(&v.to_be_bytes());
    }
    t[tb + 20..tb + 24].copy_from_slice(&[0, 1, 0, 0x30]);
    for (k, v) in [d as u32, 0, 0x38, 4, 0xA].iter().enumerate() {
        t[tb + 24 + k * 4..tb + 28 + k * 4].copy_from_slice(&v.to_be_bytes());
    }
    t[tb + 44..tb + 48].copy_from_slice(&[0, 2, 0, 3]);
    t[hdr..].copy_from_slice(&slot);
    let names = ["0", "3", "sg1", "2", "sg2", "1"];
    let sizes = [240u32, 288, 0, t.len() as u32, 0, 32];
    let table_end = 16 + names.iter().map(|n| 8 + n.len() + 1).sum::<usize>();
    let data_offset = table_end + 8;
    let a64 = |x: usize| x.div_ceil(64) * 64;
    let o0 = a64(data_offset);
    let o3 = a64(o0 + 240);
    let sg1 = a64(o3 + 288);
    let o2 = sg1;
    let sg2 = a64(o2 + t.len());
    let o1 = sg2;
    let last = o1 + 32;
    let offs = [o0, o3, sg1, o2, sg2, o1];
    let mut pkg = vec![0u8; last + 16];
    pkg[..4].copy_from_slice(b"BIGF");
    pkg[4..8].copy_from_slice(&(last as u32).to_le_bytes());
    pkg[8..12].copy_from_slice(&6u32.to_be_bytes());
    pkg[12..16].copy_from_slice(&(data_offset as u32).to_be_bytes());
    let mut c = 16;
    for (k, n) in names.iter().enumerate() {
        pkg[c..c + 4].copy_from_slice(&(offs[k] as u32).to_be_bytes());
        pkg[c + 4..c + 8].copy_from_slice(&sizes[k].to_be_bytes());
        c += 8;
        pkg[c..c + n.len()].copy_from_slice(n.as_bytes());
        c += n.len() + 1;
    }
    pkg[table_end..table_end + 8].copy_from_slice(b"L286\x15\x05\x00\x01");
    pkg[o0..o0 + 14].copy_from_slice(b"Apt Data:1:5:4");
    pkg[o3..o3 + 4].copy_from_slice(&[0, 0, 1, 0x20]);
    pkg[o2..o2 + t.len()].copy_from_slice(&t);
    pkg[o1..o1 + 17].copy_from_slice(b"Apt constant file");
    for (k, b) in pkg[last..].iter_mut().enumerate() {
        *b = 0xD0 | k as u8;
    }
    pkg
}

/// An assets-only project on `game_id`/`platform` whose `rel` file (native container bytes) is
/// replaced with a solid image; saved and packaged for `platforms`. Returns the package and the
/// author's prior file identity.
fn texture_package(reg: &Registry, base: &Path, game_id: &str, platform: &str, rel: &str, native: &[u8], rgb: [u8; 3], w: u32, h: u32, platforms: &[&str]) -> Result<studio_api::dto::PackageBuildDto, studio_api::ApiError> {
    let pf = parse_platform(platform).unwrap();
    let assets = base.join("assets");
    let file = assets.join(rel);
    std::fs::create_dir_all(file.parent().unwrap()).unwrap();
    std::fs::write(&file, native).unwrap();
    let sources = studio_api::dto::ImportSourcesDto { assets_dir: Some(s(&assets)), ..Default::default() };
    let opened = reg.project_create_import(&s(&base.join("author")), game_id, pf, &sources, "", None, None, None, &defs()).unwrap();
    let id = opened.summary.project_id;
    let replacement = base.join("replacement.png");
    std::fs::write(&replacement, png(rgb, w, h)).unwrap();
    let plan = reg.texture_replace_plan(&s(&assets), rel, &s(&replacement), None).unwrap();
    reg.import_texture(&s(&assets), rel, &s(&replacement), true, None, plan.review_id).unwrap();
    let saved = reg.project_save(id).unwrap();
    assert!(saved.nothing_to_save, "an assets-only save has nothing left to write: outputs are already durable");
    let built = reg.package_build(id, &req("Texture", &["textures"], platforms), None, &defs());
    assert!(reg.close_project(id));
    built
}

// ------------------------------------------------------------------ R1

#[test]
fn r1_missing_or_duplicate_keys_refuse_without_writing() {
    let t = Tmp::new("r1-keys");
    let reg = Registry::default();
    let st = store();
    let ps4 = parse_platform("ps4").unwrap();
    let sources = studio_api::dto::ImportSourcesDto { main_db: Some(s(&repo("anchors/ps4/nhl21/databases/nhl_game.db"))), ..Default::default() };
    let opened = reg.project_create_import(&s(&t.0.join("author")), "nhl21", ps4, &sources, "", None, None, Some(&st), &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(&reg, id);
    reg.project_save(id).unwrap();
    let built = reg.package_build(id, &req("Keys", &["cells"], &["ps4"]), Some(&st), &defs()).unwrap();
    let ins = pkg::inspect_package(Path::new(&built.path), &pkg::Limits::default()).unwrap();
    assert_eq!(ins.edits.schema, pkg::EDITS_SCHEMA);
    for c in &ins.edits.cells {
        assert!(c.keys.iter().any(|k| k.studio_field == "wBIz"), "a player edit is bound to the player id key: {:?}", c.keys);
        assert!(c.row_sha256.as_deref().is_some_and(|h| h.len() == 64));
    }
    let inspected = reg.package_inspect(&built.path).unwrap();
    assert_eq!(inspected.edits_schema, pkg::EDITS_SCHEMA);

    // Recipient A: row 5 is a full copy of row 0 → the edited player's key is ambiguous.
    let game = t.0.join("dup");
    let dbpath = nhl21_game(&game);
    let original = std::fs::read(&dbpath).unwrap();
    let db = tdb_core::TdbFile::parse(&original).unwrap();
    let table = db.table_canonical("ubPc").unwrap();
    let len = table.record_len_bytes as usize;
    let a = table.record_block;
    let mut dup = original.clone();
    let b = a + 5 * len;
    let row0 = original[a..a + len].to_vec();
    dup[b..b + len].copy_from_slice(&row0);
    db.recompute_crcs(&mut dup);
    std::fs::write(&dbpath, &dup).unwrap();
    let before = FileIdentity::of_file(&dbpath).unwrap();
    let plan = reg.package_preflight(&built.path, "nhl21", ps4, &s(&game), None, None, false, &s(&t.0.join("records")), &defs()).unwrap();
    assert!(!plan.installable, "{:?}", plan.reasons);
    assert!(plan.reasons.iter().any(|r| r.contains("more than one row") && r.contains("ambiguous")), "{:?}", plan.reasons);
    assert_eq!(FileIdentity::of_file(&dbpath).unwrap(), before);
    assert!(reg.package_apply(plan.handle).is_err(), "a blocked plan never applies");
    assert_eq!(FileIdentity::of_file(&dbpath).unwrap(), before);

    // Recipient B: row 0 is overwritten by row 7 → the edited player's key is gone; the other
    // edit (row 3) still resolves. The whole plan is blocked and nothing is written.
    let game = t.0.join("missing");
    let dbpath = nhl21_game(&game);
    let mut gone = original.clone();
    let r7 = a + 7 * len;
    let row7 = original[r7..r7 + len].to_vec();
    gone[a..a + len].copy_from_slice(&row7);
    db.recompute_crcs(&mut gone);
    std::fs::write(&dbpath, &gone).unwrap();
    let before = FileIdentity::of_file(&dbpath).unwrap();
    let plan = reg.package_preflight(&built.path, "nhl21", ps4, &s(&game), None, None, false, &s(&t.0.join("records")), &defs()).unwrap();
    assert!(!plan.installable, "{:?}", plan.reasons);
    assert!(plan.reasons.iter().any(|r| r.contains("no row of") && r.contains("is not there")), "{:?}", plan.reasons);
    assert_eq!(plan.reasons.len(), 1, "exactly the missing player blocks: {:?}", plan.reasons);
    assert_eq!(FileIdentity::of_file(&dbpath).unwrap(), before);
    assert!(reg.close_project(id));
}

#[test]
fn r1_a_position_bound_package_from_r71_is_refused_by_the_installer() {
    let t = Tmp::new("r1-v1");
    let reg = Registry::default();
    let ps4 = parse_platform("ps4").unwrap();
    // A schema-1 edits document (what r71 wrote): one cell bound to a row number only.
    let cell = pkg::CellOp {
        file: "main_db".into(),
        studio_table: "ubPc".into(),
        table_name: "Players".into(),
        record: 0,
        entity: "Paul Stastny".into(),
        studio_field: "vjla".into(),
        field_name: "position".into(),
        before: pkg::CellValue::Int { raw: 0, decoded: 0, display: "C".into() },
        after: pkg::CellValue::Int { raw: 5, decoded: 5, display: "LW".into() },
        keys: vec![],
        row_sha256: None,
    };
    let manifest = pkg::PackageManifest {
        schema: pkg::PACKAGE_SCHEMA.into(),
        package_id: String::new(),
        name: "Old".into(),
        author: String::new(),
        version: "0.9".into(),
        description: String::new(),
        created: "2026-10-01T00:00:00Z".into(),
        studio: pkg::StudioProvenance { version: "r71".into(), inventory_head: None },
        game_id: "nhl21".into(),
        title: "NHL 21".into(),
        author_platform: "ps4".into(),
        targets: vec![pkg::TargetSupport { platform: "ps4".into(), status: "supported".into(), components: vec![pkg::ComponentSupport { category: "cells".into(), status: "supported".into(), reason: "author".into() }] }],
        base: vec![],
        categories: vec![pkg::CategoryCount { category: "cells".into(), count: 1 }, pkg::CategoryCount { category: "dictionary".into(), count: 0 }, pkg::CategoryCount { category: "textures".into(), count: 0 }],
        entries: vec![],
        project: pkg::ProjectProvenance { project_id: "p".into(), source_kind: "import".into(), save_seq: 1 },
    };
    let edits = pkg::PackageEdits { schema: pkg::EDITS_SCHEMA_V1.into(), free_agents: None, schedules: vec![], cells: vec![cell], dictionary: vec![], textures: vec![] };
    let bytes = pkg::build_package(manifest, &edits, &[]).unwrap();
    let path = t.0.join("old.nhlmod");
    std::fs::write(&path, &bytes).unwrap();
    // It still inspects (the author can see what it is) …
    let ins = reg.package_inspect(&s(&path)).unwrap();
    assert_eq!(ins.edits_schema, pkg::EDITS_SCHEMA_V1);
    assert_eq!(ins.cells.len(), 1);
    // … but the installer refuses to replay it, before any destination is read or written.
    let game = t.0.join("game");
    let dbpath = nhl21_game(&game);
    let before = FileIdentity::of_file(&dbpath).unwrap();
    let err = reg.package_preflight(&s(&path), "nhl21", ps4, &s(&game), None, None, false, &s(&t.0.join("records")), &defs()).err().expect("refused").to_string();
    assert!(err.contains("row positions") && err.contains("Rebuild"), "{err}");
    assert_eq!(FileIdentity::of_file(&dbpath).unwrap(), before);
}

// ------------------------------------------------------------------ R2

#[test]
fn r2_a_database_only_package_never_needs_or_touches_a_roster_even_on_ps4_with_one_named() {
    let t = Tmp::new("r2-ps4-main");
    let reg = Registry::default();
    let st = store();
    let ps4 = parse_platform("ps4").unwrap();
    let sources = studio_api::dto::ImportSourcesDto { main_db: Some(s(&repo("anchors/ps4/nhl21/databases/nhl_game.db"))), ..Default::default() };
    let opened = reg.project_create_import(&s(&t.0.join("author")), "nhl21", ps4, &sources, "", None, None, Some(&st), &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(&reg, id);
    reg.project_save(id).unwrap();
    let built = reg.package_build(id, &req("DB only", &["cells"], &["ps4"]), Some(&st), &defs()).unwrap();
    let ins = pkg::inspect_package(Path::new(&built.path), &pkg::Limits::default()).unwrap();
    assert!(ins.edits.cells.iter().all(|c| c.file == "main_db"));
    let game = t.0.join("game");
    let dbpath = nhl21_game(&game);
    // A roster is named (and does not even exist): a database-only package ignores it — no stale
    // choice refusal, no redirection of the database edits into a save, no roster requirement.
    let plan = reg.package_preflight(&built.path, "nhl21", ps4, &s(&game), Some(&s(&t.0.join("savedata"))), Some("Z:/nowhere/DATA"), false, &s(&t.0.join("records")), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    assert_eq!(plan.items.len(), 1);
    assert_eq!(plan.items[0].role.as_str(), "database");
    assert_eq!(Path::new(&plan.items[0].destination_path), dbpath.as_path());
    let roster_req = plan.requirements.iter().find(|r| r.role == "roster_save").unwrap();
    assert_eq!(roster_req.status, "not-needed");
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.destinations.len(), 1);
    assert_eq!(Path::new(&applied.destinations[0].path), dbpath.as_path());
    assert!(reg.close_project(id));
}

#[test]
fn r2_both_edits_check_and_apply_each_file_on_its_own_and_roster_choices_are_explicit() {
    let t = Tmp::new("r2-both");
    let reg = Registry::default();
    let st = store();
    let ps3 = parse_platform("ps3").unwrap();
    let opened = reg.project_create_fresh(&s(&t.0.join("author")), "nhllegacy", ps3, "", None, None, &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(&reg, id);
    reg.project_save(id).unwrap();
    let built = reg.package_build(id, &req("Both", &["cells"], &["ps3"]), Some(&st), &defs()).unwrap();
    let ins = pkg::inspect_package(Path::new(&built.path), &pkg::Limits::default()).unwrap();
    assert!(ins.edits.cells.iter().all(|c| c.file == "both"), "{:?}", ins.edits.cells.iter().map(|c| c.file.clone()).collect::<Vec<_>>());
    assert!(reg.close_project(id));

    // The recipient's roster carries its own value for the first edited cell; its database does not.
    let (root, db, _, roster) = ps3_legacy_install(&t.0);
    let meta = db.with_file_name("nhlng-meta.xml");
    {
        let project = Project::load(&roster, ps3, ProjectRole::Roster, &meta, defs()).unwrap();
        let mut session = EditSession::begin(project);
        let cur = ins.edits.cells[0].record;
        session.set_field("ubPc", cur, "vjla", FieldValue::Int(7)).unwrap();
        std::fs::write(&roster, session.save().unwrap()).unwrap();
    }
    let before = (FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap());
    let records = t.0.join("records");
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, None, false, &s(&records), &defs()).unwrap();
    assert!(!plan.installable, "{:?}", plan.reasons);
    assert_eq!(plan.conflicts.len(), 1, "{:?}", plan.conflicts);
    assert!(plan.conflicts[0].contains("(roster save)"), "the conflict names the file it was found in: {}", plan.conflicts[0]);
    assert_eq!(plan.applied, 1, "the other edit applies to both files; the conflicting one applies to neither");
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);
    // Allowed: both files take both edits, each checked against its own current value.
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, None, true, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    assert_eq!(plan.applied, 2);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let changed = diff(&repo("anchors/ps3/nhllegacy/databases/nhlng.db"), &db, &meta, ps3, ProjectRole::Nhlng);
    assert_eq!(changed.len(), 2, "{changed:?}");
    let rchanged = diff(&repo("anchors/ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA"), &roster, &meta, ps3, ProjectRole::Roster);
    assert_eq!(rchanged.len(), 2, "{rchanged:?}");
    reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);

    // Several roster saves: no silent pick — an unmade choice and a stale choice both block.
    let second = root.join("dev_hdd0/home/00000001/savedata/BLUS315400201 - ROSTER/SYS-DATA");
    copy(&repo("anchors/ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA"), &second);
    let report = studio_domain::detect_install("nhllegacy", ps3, &root, None);
    let rosters: Vec<&str> = report.saves.iter().filter(|x| x.kind == "roster" && x.writable).map(|x| x.path.as_str()).collect();
    assert_eq!(rosters.len(), 2, "the owned tree offers two roster saves: {rosters:?}");
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, None, true, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("choose which one")), "{:?}", plan.reasons);
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, Some(&s(&root.join("dev_hdd0/home/00000001/savedata/GONE - ROSTER/SYS-DATA"))), true, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("isn't one of the roster saves")), "{:?}", plan.reasons);
    // The explicit choice is the only roster touched.
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, Some(&s(&second)), true, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let first_before = FileIdentity::of_file(&roster).unwrap();
    let applied = reg.package_apply(plan.handle).unwrap();
    assert!(applied.destinations.iter().any(|d| Path::new(&d.path) == second));
    assert!(!applied.destinations.iter().any(|d| Path::new(&d.path) == roster));
    assert_eq!(FileIdentity::of_file(&roster).unwrap(), first_before, "the unchosen roster is untouched");
}

// ------------------------------------------------------------------ R3

#[test]
fn r3_a_fault_while_applying_rolls_back_and_a_split_apply_is_recovered_by_restore() {
    let t = Tmp::new("r3-apply");
    let reg = Registry::default();
    let recomp = parse_platform("recomp").unwrap();
    let built = legacy_xbox_package(&reg, &t.0.join("author"), &["xbox", "recomp"]);
    let (game, db, roster, header) = recomp_content_tree(&t.0);
    let before = (FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap());
    let records = t.0.join("records");
    // The reviewed plan names every actual output, the sidecar included.
    let plan = reg.package_preflight(&built.path, "legacyrecomp", recomp, &s(&game), Some(&s(&roster)), Some(&s(&roster)), false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let keys: Vec<&str> = plan.items.iter().map(|i| i.key.as_str()).collect();
    assert_eq!(keys, vec!["main_db", "roster", "roster.header"], "{keys:?}");
    assert_eq!(plan.items[2].state, "create");
    assert_eq!(Path::new(&plan.items[2].destination_path), header.as_path());
    // Fault at the roster write: the database (already published) is put back; the record says so.
    let err = reg.package_apply_with(plan.handle, &mut Fault { fail_at: (At::BeforeWrite, roster.clone()), break_revert_of: None }).err().expect("fault").to_string();
    assert!(err.contains("rolled-back"), "{err}");
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);
    assert!(!header.exists());
    let installs = reg.package_installs(&s(&records)).unwrap();
    assert_eq!(installs.len(), 1);
    assert_eq!((installs[0].state.as_str(), installs[0].restorable), ("rolled-back", false));
    assert!(installs[0].items.iter().all(|i| i.state == "prior"));

    // A fault after the roster was renamed in, with the database's own rollback broken: a split,
    // recorded per item, that Restore finishes once the obstacle is gone.
    let plan = reg.package_preflight(&built.path, "legacyrecomp", recomp, &s(&game), Some(&s(&roster)), Some(&s(&roster)), false, &s(&records), &defs()).unwrap();
    let err = reg.package_apply_with(plan.handle, &mut Fault { fail_at: (At::AfterRename, roster.clone()), break_revert_of: Some(db.clone()) }).err().expect("split").to_string();
    assert!(err.contains("split") && err.contains("Recover"), "{err}");
    let _ = std::fs::remove_dir(temp_path(&db));
    assert_ne!(FileIdentity::of_file(&db).unwrap(), before.0, "the database holds the new bytes");
    assert_eq!(FileIdentity::of_file(&roster).unwrap(), before.1, "the roster was put back");
    let installs = reg.package_installs(&s(&records)).unwrap();
    let split = installs.iter().find(|r| r.state == "split").expect("the split record");
    assert!(split.restorable && split.recovery);
    assert_eq!(split.items.iter().map(|i| i.state.as_str()).collect::<Vec<_>>(), vec!["installed", "prior", "prior"]);
    // Recovery from another process: a fresh registry reads the record and finishes.
    let other = Registry::default();
    let restored = other.package_restore(&split.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);
    assert!(!header.exists());
    assert!(other.package_installs(&s(&records)).unwrap().iter().all(|r| !r.restorable));
}

#[test]
fn r3_an_interrupted_restore_is_retried_and_a_split_restore_resumes_after_reopen() {
    let t = Tmp::new("r3-restore");
    let reg = Registry::default();
    let recomp = parse_platform("recomp").unwrap();
    let built = legacy_xbox_package(&reg, &t.0.join("author"), &["xbox", "recomp"]);
    let (game, db, roster, header) = recomp_content_tree(&t.0);
    let before = (FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap());
    let records = t.0.join("records");
    let plan = reg.package_preflight(&built.path, "legacyrecomp", recomp, &s(&game), Some(&s(&roster)), Some(&s(&roster)), false, &s(&records), &defs()).unwrap();
    let applied = reg.package_apply(plan.handle).unwrap();
    assert!(header.exists());
    let installed = (FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap(), FileIdentity::of_file(&header).unwrap());

    // The header's removal fails: the database and roster puts-back are rolled back inside the
    // same step set; the record stays committed; nothing is stranded.
    let err = reg.package_restore_with(&applied.record_dir, &mut Fault { fail_at: (At::BeforeRemove, header.clone()), break_revert_of: None }).err().expect("fault").to_string();
    assert!(err.contains("did not complete") && err.contains("retry"), "{err}");
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap(), FileIdentity::of_file(&header).unwrap()), installed);
    let rec = &reg.package_installs(&s(&records)).unwrap()[0];
    assert_eq!((rec.state.as_str(), rec.restorable, rec.recovery), ("committed", true, false));
    // Retry: completes.
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);
    assert!(!header.exists());

    // Install again; now the restore splits (the header is removed, the roster is put back, the
    // database's own rollback is broken): recorded per item as restore-incomplete.
    let plan = reg.package_preflight(&built.path, "legacyrecomp", recomp, &s(&game), Some(&s(&roster)), Some(&s(&roster)), false, &s(&records), &defs()).unwrap();
    let applied = reg.package_apply(plan.handle).unwrap();
    let err = reg.package_restore_with(&applied.record_dir, &mut Fault { fail_at: (At::AfterRemoved, header.clone()), break_revert_of: Some(db.clone()) }).err().expect("split").to_string();
    assert!(err.contains("restore-incomplete"), "{err}");
    let _ = std::fs::remove_dir(temp_path(&db));
    let rec = reg.package_installs(&s(&records)).unwrap().into_iter().find(|r| r.record_dir == applied.record_dir).unwrap();
    assert_eq!((rec.state.as_str(), rec.restorable, rec.recovery), ("restore-incomplete", true, true));
    assert_eq!(rec.items.iter().map(|i| i.state.as_str()).collect::<Vec<_>>(), vec!["prior", "installed", "installed"], "{:?}", rec.items.iter().map(|i| (i.key.clone(), i.state.clone())).collect::<Vec<_>>());
    assert_eq!(FileIdentity::of_file(&db).unwrap(), before.0);
    // A foreign change meanwhile refuses the recovery and touches nothing.
    let hold = std::fs::read(&roster).unwrap();
    let mut foreign = hold.clone();
    foreign[2000] ^= 0x5A;
    std::fs::write(&roster, &foreign).unwrap();
    let err = Registry::default().package_restore(&applied.record_dir).err().expect("foreign").to_string();
    assert!(err.contains("changed after the installation"), "{err}");
    assert_eq!(std::fs::read(&roster).unwrap(), foreign);
    std::fs::write(&roster, &hold).unwrap();
    // Reopened (fresh process): the recovery finishes exactly the items still holding installed bytes.
    let restored = Registry::default().package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);
    assert!(!header.exists());
}

#[test]
fn r3_the_project_deployment_route_plans_the_header_and_removes_it_inside_the_restore() {
    let t = Tmp::new("r3-deploy");
    let reg = Registry::default();
    let st = store();
    let recomp = parse_platform("recomp").unwrap();
    let (game, db, roster, header) = recomp_content_tree(&t.0);
    let before = (FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap());
    let opened = reg.project_create_fresh(&s(&t.0.join("project")), "legacyrecomp", recomp, &s(&game), Some(&s(&roster)), Some(&s(&roster)), &st, &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(&reg, id);
    reg.project_save(id).unwrap();
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(plan.deployable, "{:?}", plan.reasons);
    let header_item = plan.items.iter().find(|i| i.key == "roster.header").expect("the sidecar is in the reviewed plan");
    assert_eq!(header_item.state.as_str(), "absent");
    assert_eq!(header_item.destination_path.as_deref().map(Path::new), Some(header.as_path()));
    let applied = reg.project_deploy_apply(id, plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert!(header.exists());
    // The header's removal fails: nothing is stranded, the deployment stays restorable.
    let err = reg.project_deploy_restore_with(id, applied.seq, &mut Fault { fail_at: (At::BeforeRemove, header.clone()), break_revert_of: None }).err().expect("fault").to_string();
    assert!(err.contains("did not complete"), "{err}");
    assert!(header.exists());
    assert_ne!(FileIdentity::of_file(&db).unwrap(), before.0);
    let d = &reg.project_deployments(id).unwrap()[0];
    assert_eq!((d.state.as_str(), d.restorable, d.recovery), ("committed", true, false));
    let restored = reg.project_deploy_restore(id, applied.seq).unwrap();
    assert_eq!(restored.state, "restored");
    assert!(!header.exists());
    assert_eq!((FileIdentity::of_file(&db).unwrap(), FileIdentity::of_file(&roster).unwrap()), before);
    assert!(reg.close_project(id));
}

// ------------------------------------------------------------------ R4

#[test]
fn r4_assets_only_save_reopen_relocate_and_build_without_a_database() {
    let t = Tmp::new("r4");
    let reg = Registry::default();
    let ps4 = parse_platform("ps4").unwrap();
    let assets = t.0.join("assets");
    let rel = "fe/ion/artassets/leaguelogos/l0.png";
    let logo = assets.join(rel);
    std::fs::create_dir_all(logo.parent().unwrap()).unwrap();
    std::fs::write(&logo, png([10, 20, 30], 8, 8)).unwrap();
    let project_dir = t.0.join("author");
    let sources = studio_api::dto::ImportSourcesDto { assets_dir: Some(s(&assets)), ..Default::default() };
    let opened = reg.project_create_import(&s(&project_dir), "nhl21", ps4, &sources, "", None, None, None, &defs()).unwrap();
    let id = opened.summary.project_id;
    assert_eq!(opened.summary.role, "assets");
    let replacement = t.0.join("replacement.png");
    let source_png = png([200, 0, 0], 8, 8);
    std::fs::write(&replacement, &source_png).unwrap();
    let plan = reg.texture_replace_plan(&s(&assets), rel, &s(&replacement), None).unwrap();
    reg.import_texture(&s(&assets), rel, &s(&replacement), true, None, plan.review_id).unwrap();
    // Save is valid and accurate: nothing is left to write.
    let saved = reg.project_save(id).unwrap();
    assert!(saved.nothing_to_save && !saved.saved);
    assert_eq!(reg.summary(id).unwrap().role, "assets");
    reg.project_set_view(id, "/app/assets/logos").unwrap();
    assert!(reg.close_project(id));
    // Relocate the project folder; reopen in a fresh process.
    let moved = t.0.join("elsewhere/author-moved");
    std::fs::create_dir_all(moved.parent().unwrap()).unwrap();
    std::fs::rename(&project_dir, &moved).unwrap();
    let other = Registry::default();
    let reopened = other.project_open(&s(&moved), &defs()).unwrap();
    let id = reopened.summary.project_id;
    assert_eq!(reopened.summary.role, "assets");
    assert_eq!(reopened.last_view.as_deref(), Some("/app/assets/logos"));
    let info = other.project_info(id).unwrap();
    assert_eq!(info.staged_files.len(), 1);
    assert_eq!(info.staged_files[0].relative, rel);
    assert_eq!(info.staged_files[0].origin, "texture-replace");
    assert!(info.staged_files[0].source_sha256.is_some(), "the editable source is kept");
    assert_eq!(info.sources, 1);
    assert!(other.project_save(id).unwrap().nothing_to_save);
    let inv = other.package_inventory(id, None, &defs()).unwrap();
    assert_eq!(inv.categories.iter().find(|c| c.category == "textures").unwrap().count, 1);
    let built = other.package_build(id, &req("Logo", &["textures"], &["ps4"]), None, &defs()).unwrap();
    let ins = pkg::inspect_package(Path::new(&built.path), &pkg::Limits::default()).unwrap();
    assert_eq!(ins.edits.textures.len(), 1);
    assert_eq!(ins.payload(&ins.edits.textures[0].payload).unwrap(), source_png, "the package carries the authored source bytes");
    assert!(other.close_project(id));
}

// ------------------------------------------------------------------ R5

#[test]
fn r5_a_native_ps3_texture_installs_into_the_asset_root_and_bad_destinations_refuse() {
    let t = Tmp::new("r5");
    let reg = Registry::default();
    let ps3 = parse_platform("ps3").unwrap();
    let rel = "fe/ion/artassets/leaguelogos/l0.big";
    let native = synthetic_ps3_logo_package(64, 32, 0x40);
    let built = texture_package(&reg, &t.0, "nhllegacy", "ps3", rel, &native, [200, 0, 0], 64, 32, &["ps3"]).unwrap();
    let records = t.0.join("records");

    // Wrong root: an empty folder is no NHL Legacy installation; nothing is located or written.
    let empty = t.0.join("not-a-game");
    std::fs::create_dir_all(&empty).unwrap();
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&empty), None, None, false, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("installation was recognised") || r.contains("wrong folder")), "reasons {:?}; items {:?}; requirements {:?}", plan.reasons, plan.items.iter().map(|i| i.destination_path.clone()).collect::<Vec<_>>(), plan.requirements.iter().map(|r| (r.role.clone(), r.status.clone(), r.detail.clone())).collect::<Vec<_>>());

    // Missing extraction: a recognised install without the loose file.
    let (root, _, _, _) = ps3_legacy_install(&t.0);
    let report = studio_domain::detect_install("nhllegacy", ps3, &root, None);
    let install_dir = PathBuf::from(report.install_dir.clone().expect("the owned PS3 tree is recognised"));
    assert!(!install_dir.ends_with("db"), "the asset root is the install folder, not the database folder: {}", install_dir.display());
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, None, false, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("no such file") && r.contains("extract")), "{:?}", plan.reasons);

    // Incompatible file: the recipient's file at that path is not a texture container at all.
    let dest = install_dir.join(rel);
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::fs::write(&dest, b"not a texture container; a stray text file at the logo's path".repeat(64)).unwrap();
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, None, true, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("not a texture container") || r.contains("unrecognized texture container")), "{:?}", plan.reasons);

    // Changed target: a valid container that is not the file the author replaced → a conflict.
    let other_native = synthetic_ps3_logo_package(64, 32, 0x41);
    std::fs::write(&dest, &other_native).unwrap();
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, None, false, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.conflicts.len() == 1 && plan.conflicts[0].contains("not the file the author replaced"), "{:?} {:?}", plan.reasons, plan.conflicts);
    assert_eq!(std::fs::read(&dest).unwrap(), other_native, "preflight writes nothing");

    // The actual file: installs into the asset root, decodes to the authored image, restores exactly.
    std::fs::write(&dest, &native).unwrap();
    let plan = reg.package_preflight(&built.path, "nhllegacy", ps3, &s(&root), None, None, false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    assert_eq!(plan.items.len(), 1);
    assert_eq!(Path::new(&plan.items[0].destination_path), dest.as_path());
    assert_eq!(std::fs::read(&dest).unwrap(), native, "preflight writes nothing");
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let installed = std::fs::read(&dest).unwrap();
    assert_ne!(installed, native);
    assert!(installed.starts_with(b"BIGF"), "the recipient's own container shape is kept");
    let preview = decode_data_url(&reg.texture_preview(&s(&install_dir), rel, 0, None).unwrap().data_url);
    assert_eq!((preview.width, preview.height), (64, 32));
    assert!(preview.rgba.chunks(4).all(|p| p[0] == 200 && p[1] == 0 && p[2] == 0), "the installed texture decodes to the authored image");
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&dest).unwrap(), native);
}

// ------------------------------------------------------------------ R6

#[test]
fn r6_an_xbox_texture_installs_on_a_recompiled_tree_and_is_refused_for_ps3_with_the_reason() {
    use tdb_rx2::texlib::{base_mip_byte_size, PixelFormat};
    let t = Tmp::new("r6");
    let reg = Registry::default();
    let recomp = parse_platform("recomp").unwrap();
    let rel = "rendering/logo/texlib_10_0_0.rx2";
    let raster = vec![0u8; base_mip_byte_size(PixelFormat::Dxt1, 64, 64)];
    let native = tdb_rx2::texlib::encode_synthetic_texlib(&[(64, 64, PixelFormat::Dxt1, &raster)]);
    // PS3 declared: refused with the exact missing-profile reason.
    let err = texture_package(&reg, &t.0.join("a"), "nhllegacy", "xbox", rel, &native, [0, 0, 200], 64, 64, &["xbox", "ps3"]).err().expect("refused").to_string();
    assert!(err.contains("ps3: textures") && err.contains("no registered loose-asset profile"), "{err}");
    // Xbox 360 + Recompiled: the registered shared layout.
    let built = texture_package(&reg, &t.0.join("b"), "nhllegacy", "xbox", rel, &native, [0, 0, 200], 64, 64, &["xbox", "recomp"]).unwrap();
    let ins = reg.package_inspect(&built.path).unwrap();
    let rc = ins.targets.iter().find(|x| x.platform == "recomp").unwrap();
    assert_eq!(rc.status, "supported");
    assert!(rc.components.iter().any(|c| c.category == "textures" && c.reason.contains("registered inheritance")), "{:?}", rc.components.iter().map(|c| c.reason.clone()).collect::<Vec<_>>());
    // An owned Recompiled tree keeps its art under `_compiled/` beside the executable folder.
    let (game, saves, _, _, _) = recomp_install(&t.0);
    let report = studio_domain::detect_install("legacyrecomp", recomp, &game, Some(&saves));
    let install_dir = PathBuf::from(report.install_dir.clone().expect("the owned Recompiled tree is recognised"));
    let dest = install_dir.join("_compiled").join(rel);
    std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
    std::fs::write(&dest, &native).unwrap();
    let records = t.0.join("records");
    let plan = reg.package_preflight(&built.path, "legacyrecomp", recomp, &s(&game), Some(&s(&saves)), None, false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    assert_eq!(Path::new(&plan.items[0].destination_path), dest.as_path(), "placed under the Recompiled `_compiled/` art root");
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let installed = std::fs::read(&dest).unwrap();
    assert_ne!(installed, native);
    assert_eq!(&installed[..7], &native[..7], "the recipient's own texlib container is kept");
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(std::fs::read(&dest).unwrap(), native);
}

// ------------------------------------------------------------------ R72-R1 (roster-only installation)

/// A roster-only Legacy Xbox 360 package authored from the bundled roster alone.
fn roster_only_xbox_package(reg: &Registry, dir: &Path) -> (studio_api::dto::PackageBuildDto, PathBuf) {
    let st = store();
    let xbox = parse_platform("xbox").unwrap();
    let anchor = repo("anchors/xbox360/nhllegacy/savefiles/ROSTER 20260627060052/ROSTER 20260627060052");
    let sources = studio_api::dto::ImportSourcesDto { roster_save: Some(s(&anchor)), ..Default::default() };
    let opened = reg.project_create_import(&s(dir), "nhllegacy", xbox, &sources, "", None, None, Some(&st), &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(reg, id);
    reg.project_save(id).unwrap();
    let built = reg.package_build(id, &req("Roster only", &["cells"], &["xbox"]), Some(&st), &defs()).unwrap();
    let ins = pkg::inspect_package(Path::new(&built.path), &pkg::Limits::default()).unwrap();
    assert!(!ins.edits.cells.is_empty() && ins.edits.cells.iter().all(|c| c.file == "roster"));
    assert!(reg.close_project(id));
    (built, anchor)
}

/// A baseline store **outside the checkout**: the inventory plus only the files `keep` names,
/// copied into a temp folder (what the shipped `baselines/` bundle provides).
fn store_outside_checkout(dir: &Path, keep: &[&str]) -> studio_domain::BaselineStore {
    let inv = dir.join("INVENTORY.json");
    copy(&repo("baselines/INVENTORY.json"), &inv);
    for rel in keep {
        copy(&repo("anchors").join(rel), &dir.join("files").join(rel));
    }
    studio_domain::BaselineStore::open(&inv, &dir.join("files")).unwrap()
}

#[test]
fn r72_r1_a_roster_only_package_installs_from_the_save_location_alone_with_bundled_schema() {
    let t = Tmp::new("r72-roster-only");
    let reg = Registry::default();
    let xbox = parse_platform("xbox").unwrap();
    let (built, anchor) = roster_only_xbox_package(&reg, &t.0.join("author"));
    // The schema comes from a store that is not the repository checkout.
    let outside = store_outside_checkout(&t.0.join("shipped-baselines"), &["xbox360/nhllegacy/databases/nhlng-meta.xml"]);
    reg.set_baselines(outside);
    // An independent recipient: a save folder with the genuine roster, no game anywhere.
    let saves = t.0.join("recipient-saves");
    let roster = saves.join("ROSTER 20260627060052/ROSTER 20260627060052");
    copy(&anchor, &roster);
    let before = FileIdentity::of_file(&roster).unwrap();
    let records = t.0.join("records");
    // Save-data location only (the single save is picked), empty game root.
    let plan = reg.package_preflight(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), None, false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let req_of = |role: &str| plan.requirements.iter().find(|r| r.role == role).map(|r| (r.status.clone(), r.detail.clone())).unwrap();
    assert_eq!(req_of("main_db").0, "not-needed");
    assert_eq!(req_of("schema").0, "found");
    assert!(req_of("schema").1.contains("bundled nhlng-meta.xml"), "{:?}", req_of("schema"));
    assert_eq!(req_of("roster_save").0, "found");
    assert_eq!(plan.items.len(), 1, "the roster alone: {:?}", plan.items.iter().map(|i| i.key.clone()).collect::<Vec<_>>());
    assert_eq!(Path::new(&plan.items[0].destination_path), roster.as_path());
    assert_eq!(plan.applied, 2);
    assert_eq!(FileIdentity::of_file(&roster).unwrap(), before, "preflight writes nothing");
    // The roster save itself as the only path: its folder is the location.
    let plan2 = reg.package_preflight(&built.path, "nhllegacy", xbox, "", None, Some(&s(&roster)), false, &s(&records), &defs()).unwrap();
    assert!(plan2.installable, "{:?}", plan2.reasons);
    let applied = reg.package_apply(plan2.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_eq!(applied.destinations.len(), 1);
    // Exactly the two intended cells differ in the native save; nothing else exists to change.
    let meta = repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
    let changed = diff(&anchor, &roster, &meta, xbox, ProjectRole::Roster);
    assert_eq!(changed.len(), 2, "{changed:?}");
    assert!(changed.iter().all(|c| c.0 == "ubPc" && c.2 == "vjla"), "{changed:?}");
    assert!(!saves.join("db").exists() && !saves.join("nhlng.db").exists(), "no database was created anywhere");
    // Exact restore.
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(FileIdentity::of_file(&roster).unwrap(), before);
}

#[test]
fn r72_r1_schema_lookup_failures_and_bad_saves_refuse_precisely_without_writing() {
    let t = Tmp::new("r72-schema");
    let xbox = parse_platform("xbox").unwrap();
    let records = t.0.join("records");
    let (built, anchor) = roster_only_xbox_package(&Registry::default(), &t.0.join("author"));
    let saves = t.0.join("saves");
    let roster = saves.join("ROSTER 20260627060052/ROSTER 20260627060052");
    copy(&anchor, &roster);
    let before = FileIdentity::of_file(&roster).unwrap();
    // No store at all.
    let reg = Registry::default();
    let plan = reg.package_preflight_with(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), None, false, &s(&records), &defs(), None).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("no bundled schema metadata is available")), "{:?}", plan.reasons);
    // A store whose inventory lists the schema but whose file is missing: the verification names it.
    let missing = store_outside_checkout(&t.0.join("missing"), &[]);
    let plan = reg.package_preflight_with(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), None, false, &s(&records), &defs(), Some(&missing)).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("does not verify")), "{:?}", plan.reasons);
    // A store whose schema file is not the inventoried one (wrong bytes): refused, named.
    let wrong = store_outside_checkout(&t.0.join("wrong"), &["xbox360/nhllegacy/databases/nhlng-meta.xml"]);
    std::fs::write(t.0.join("wrong/files/xbox360/nhllegacy/databases/nhlng-meta.xml"), b"<not the schema/>").unwrap();
    let plan = reg.package_preflight_with(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), None, false, &s(&records), &defs(), Some(&wrong)).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("does not verify")), "{:?}", plan.reasons);
    assert_eq!(FileIdentity::of_file(&roster).unwrap(), before);
    // With the proper schema: a malformed save is refused by name, nothing written.
    let good = store_outside_checkout(&t.0.join("good"), &["xbox360/nhllegacy/databases/nhlng-meta.xml"]);
    let garbage = t.0.join("garbage/ROSTER X/ROSTER X");
    std::fs::create_dir_all(garbage.parent().unwrap()).unwrap();
    std::fs::write(&garbage, vec![0x5Au8; 4096]).unwrap();
    let r = reg.package_preflight_with(&built.path, "nhllegacy", xbox, "", Some(&s(&t.0.join("garbage"))), Some(&s(&garbage)), false, &s(&records), &defs(), Some(&good));
    let why = match r {
        Ok(p) => {
            assert!(!p.installable, "{:?}", p.reasons);
            p.reasons.join("; ")
        }
        Err(e) => e.to_string(),
    };
    assert!(why.contains("roster save") && (why.contains("isn't one of the roster saves") || why.contains("does not open") || why.contains("no writable roster save")), "{why}");
    assert_eq!(std::fs::read(&garbage).unwrap(), vec![0x5Au8; 4096]);
    // A save of another title (NHL 10 Xbox 360 roster) offered to a Legacy package: refused, untouched.
    let other = repo("anchors/xbox360/nhl10/savefiles");
    let other_roster = std::fs::read_dir(&other)
        .ok()
        .and_then(|rd| rd.flatten().map(|e| e.path()).find(|p| p.is_dir()))
        .and_then(|d| std::fs::read_dir(d).ok().and_then(|rd| rd.flatten().map(|e| e.path()).find(|p| p.is_file())));
    if let Some(other_roster) = other_roster {
        let foreign = t.0.join("foreign").join(other_roster.parent().unwrap().file_name().unwrap()).join(other_roster.file_name().unwrap());
        copy(&other_roster, &foreign);
        let fb = FileIdentity::of_file(&foreign).unwrap();
        let r = reg.package_preflight_with(&built.path, "nhllegacy", xbox, "", Some(&s(&t.0.join("foreign"))), Some(&s(&foreign)), false, &s(&records), &defs(), Some(&good));
        let installable = r.as_ref().map(|p| p.installable).unwrap_or(false);
        assert!(!installable, "a save of another title must not install: {:?}", r.map(|p| p.reasons));
        assert_eq!(FileIdentity::of_file(&foreign).unwrap(), fb);
    }
    // A mixed package (database + roster) still needs the installation.
    let reg2 = Registry::default();
    let mixed = legacy_xbox_package(&reg2, &t.0.join("mixed-author"), &["xbox"]);
    let err = reg2.package_preflight(&mixed.path, "nhllegacy", xbox, "", Some(&s(&saves)), Some(&s(&roster)), false, &s(&records), &defs()).err().expect("needs the installation").to_string();
    assert!(err.contains("choose the destination game installation first"), "{err}");
    assert_eq!(FileIdentity::of_file(&roster).unwrap(), before);
}

#[test]
fn r72_r1_roster_only_multi_save_choice_stale_choice_and_changed_target() {
    let t = Tmp::new("r72-choice");
    let reg = Registry::default();
    let xbox = parse_platform("xbox").unwrap();
    let (built, anchor) = roster_only_xbox_package(&reg, &t.0.join("author"));
    let saves = t.0.join("saves");
    let a = saves.join("ROSTER A/ROSTER A");
    let b = saves.join("ROSTER B/ROSTER B");
    copy(&anchor, &a);
    copy(&anchor, &b);
    let records = t.0.join("records");
    // Two saves, no choice: blocked; a stale choice: blocked; nothing written.
    let plan = reg.package_preflight(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), None, false, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("choose which one")), "{:?}", plan.reasons);
    let plan = reg.package_preflight(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), Some(&s(&saves.join("ROSTER GONE/ROSTER GONE"))), false, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.reasons.iter().any(|r| r.contains("isn't one of the roster saves")), "{:?}", plan.reasons);
    // The explicit choice is the only save touched.
    let (ia, ib) = (FileIdentity::of_file(&a).unwrap(), FileIdentity::of_file(&b).unwrap());
    let plan = reg.package_preflight(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), Some(&s(&b)), false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(FileIdentity::of_file(&a).unwrap(), ia);
    assert_ne!(FileIdentity::of_file(&b).unwrap(), ib);
    // Changed target: a different edit to A makes A a conflict for this package.
    {
        let meta = repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
        let project = Project::load(&a, xbox, ProjectRole::Roster, &meta, defs()).unwrap();
        let mut session = EditSession::begin(project);
        session.set_field("ubPc", 0, "vjla", FieldValue::Int(7)).unwrap();
        std::fs::write(&a, session.save().unwrap()).unwrap();
    }
    let plan = reg.package_preflight(&built.path, "nhllegacy", xbox, "", Some(&s(&saves)), Some(&s(&a)), false, &s(&records), &defs()).unwrap();
    assert!(!plan.installable && plan.conflicts.len() == 1 && plan.conflicts[0].contains("(roster save)"), "{:?} {:?}", plan.reasons, plan.conflicts);
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(FileIdentity::of_file(&b).unwrap(), ib);
}

#[test]
fn r72_r1_a_ps4_roster_only_package_installs_into_the_profile_and_title_save_folder() {
    let t = Tmp::new("r72-ps4");
    let reg = Registry::default();
    let st = store();
    let ps4 = parse_platform("ps4").unwrap();
    let anchor_dir = repo("anchors/ps4/nhl17/savefiles/RosterCustom0");
    // Author from the bundled roster alone.
    let sources = studio_api::dto::ImportSourcesDto { roster_save: Some(s(&anchor_dir.join("DATA"))), ..Default::default() };
    let opened = reg.project_create_import(&s(&t.0.join("author")), "nhl17", ps4, &sources, "", None, None, Some(&st), &defs()).unwrap();
    let id = opened.summary.project_id;
    edit_two(&reg, id);
    reg.project_save(id).unwrap();
    let built = reg.package_build(id, &req("PS4 roster", &["cells"], &["ps4"]), Some(&st), &defs()).unwrap();
    let ins = pkg::inspect_package(Path::new(&built.path), &pkg::Limits::default()).unwrap();
    assert!(ins.edits.cells.iter().all(|c| c.file == "roster"));
    assert!(reg.close_project(id));
    // An independent shadPS4-shaped save-data tree: <savedata>/<profile>/<TITLE_ID>/<save>/…
    let savedata = t.0.join("shadps4/savedata");
    let title_id = studio_domain::discovery::ps4_title_ids("nhl17")[0];
    let save_dir = savedata.join("1").join(title_id).join("RosterCustom0");
    let members = ["DATA", "INDEX", "NAME", "sce_sys/param.sfo", "sce_sys/icon0.png"];
    for name in members {
        copy(&anchor_dir.join(name), &save_dir.join(name));
    }
    let data = save_dir.join("DATA");
    let before: Vec<(String, FileIdentity)> = members.iter().map(|n| (n.to_string(), FileIdentity::of_file(&save_dir.join(n)).unwrap())).collect();
    let records = t.0.join("records");
    let plan = reg.package_preflight(&built.path, "nhl17", ps4, "", Some(&s(&savedata)), None, false, &s(&records), &defs()).unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    assert_eq!(plan.items.len(), 1);
    assert_eq!(Path::new(&plan.items[0].destination_path), data.as_path());
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    let meta = repo("anchors/ps4/nhl17/databases/nhl_game-meta.xml");
    let changed = diff(&anchor_dir.join("DATA"), &data, &meta, ps4, ProjectRole::Roster);
    assert_eq!(changed.len(), 2, "{changed:?}");
    for (n, id) in &before {
        if n != "DATA" {
            assert_eq!(&FileIdentity::of_file(&save_dir.join(n)).unwrap(), id, "{n} untouched");
        }
    }
    let restored = reg.package_restore(&applied.record_dir).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(FileIdentity::of_file(&data).unwrap(), before[0].1);
}
