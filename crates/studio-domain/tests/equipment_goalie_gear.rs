//! Equipment 4.5 goalie gear (26 Sep 2026, after the receipt-27 review): the goalie equipment
//! companion's mask, pads, blocker, trapper, goalie-stick and related columns on every committed anchor
//! directory, plus the genuine NHL 20 install database (private fixture).
//!
//! Per title the case records which columns exist and their declared ranges, then edits each one to
//! another in-range value through the ordinary validated `set_field` and checks: the re-read value;
//! the pending-review text (the stored number — no goalie-gear catalogue exists for any title, so no
//! label may appear); a value past the declared range refused; undo and redo; save → reopen on a
//! disposable copy. NHL 07 (gen6) has no goalie equipment companion — recorded, not edited. PS4 keeps
//! the same table (`exhibitiongoalieequipment`). Nothing claims which mask, pad or stick the game
//! renders for an id; the 27 anchor directories are not the 27 shipping targets (NHL 07 PS3 is
//! research-only; Legacy Recompiled has no independent anchor and is not covered here).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::{EditSession, FieldValue, FieldView, Platform, Project, ProjectRole};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn anchors() -> Vec<(String, String, PathBuf, PathBuf)> {
    let root = repo("anchors");
    let pairs = [("NHL07NG.db", "NHL07NG-meta.xml"), ("nhlng.db", "nhlng-meta.xml"), ("nhl_game.db", "nhl_game-meta.xml")];
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms.iter().filter(|e| e.path().is_dir()) {
        let mut games: Vec<_> = std::fs::read_dir(platform.path()).unwrap().flatten().collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs.iter().find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file()) {
                out.push((platform.file_name().to_string_lossy().into_owned(), game.file_name().to_string_lossy().into_owned(), dbs.join(db), dbs.join(meta)));
            }
        }
    }
    assert_eq!(out.len(), 27, "27 anchor directories (ps4/nhl20 is an NHL 19 base copy)");
    out
}

fn platform_of(dir: &str) -> Platform {
    match dir {
        "ps3" => Platform::Ps3,
        "ps4" => Platform::Ps4,
        _ => Platform::Xbox,
    }
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-goalie-gear-{tag}-{}", std::process::id()));
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

fn open(platform: &str, db: &Path, meta: &Path, game: &str) -> EditSession {
    let mut s = EditSession::begin(Project::load(db, platform_of(platform), ProjectRole::Nhlng, meta, repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

/// 4.5 goalie gear columns of the goalie equipment companion, by metadata name; each is edited where
/// the title declares it (mask: `maskid` NHL 10 →; on NHL 08 / 09 the paint is `maskcolor`, with
/// `maskpattern`, `maskrank` and `hascustommask` beside it — research R125 G1, 27 Sep; `masktype`,
/// `pads`, `blocker`, `trapper`, `stick`, `neckprotector` everywhere measured).
const COLUMNS: &[&str] = &["maskid", "maskcolor", "masktype", "maskpattern", "maskrank", "hascustommask", "pads", "blocker", "trapper", "stick", "neckprotector"];

fn equip<'a>(fields: &'a [FieldView], name: &str) -> Option<&'a FieldView> {
    fields.iter().find(|f| f.name == name)
}

/// Another in-range value: a different catalogue entry where the title has one, else the next raw
/// value (wrapping to the floor).
fn other_value(f: &FieldView) -> i64 {
    if let Some((raw, _)) = f.enum_options.iter().find(|(r, _)| Some(*r) != f.decoded) {
        return *raw;
    }
    let (lo, hi) = (f.range_low.unwrap_or(0), f.range_high.unwrap_or(1));
    let cur = f.decoded.unwrap_or(lo);
    if cur < hi {
        cur + 1
    } else {
        lo
    }
}

/// Sweep one database: returns the coverage row.
fn sweep(label: &str, platform: &str, game: &str, db: &Path, meta: &Path, tmp: &Tmp) -> String {
    let mut s = open(platform, db, meta, game);
    let subject = s.player_summaries().unwrap().into_iter().find(|p| p.is_goalie && p.team_id.is_some()).unwrap_or_else(|| panic!("{label}: a rostered goalie"));
    let v = s.player_view(subject.player_id).unwrap().unwrap();
    let Some(rec) = v.equipment_record else {
        assert_eq!(game, "nhl07", "{label}: only NHL 07 lacks the goalie equipment companion");
        let db_bytes = std::fs::read(db).unwrap();
        let tdb = TdbFile::parse(&db_bytes).unwrap();
        assert!(tdb.tables.iter().any(|t| s.project().meta().table_name(&t.id.raw()) == Some("customplayerbiotable")), "{label}: custom-player bio table");
        return "n/a — no goalie equipment companion (gen6); `glovesortrapper` / `helmetbrandorgoaliemask` live on the custom-player bio table only (recorded, not edited)".into();
    };
    let table = v.equipment_table;
    assert_eq!(v.equipment_table, "fMVl", "{label}: the goalie equipment companion");
    let mut cols = Vec::new();
    let mut edits: Vec<(String, String, i64, i64)> = Vec::new(); // (name, field id, before, after)
    for name in COLUMNS {
        let Some(f) = equip(&v.equipment, name) else { continue };
        let (lo, hi) = (f.range_low.unwrap(), f.range_high.unwrap());
        // No goalie-gear catalogue exists for any title: never a label.
        assert!(f.enum_options.is_empty() && f.enum_label.is_none(), "{label}: {name} carries a label");
        cols.push(format!("{name}[{lo}..{hi}]"));
        assert!(f.editable, "{label}: {name} editable");
        let before = f.decoded.unwrap();
        let after = other_value(f);
        s.set_field(table, rec, &f.studio_field, FieldValue::Int(after)).unwrap_or_else(|e| panic!("{label}: {name} → {after}: {e}"));
        let err = s.set_field(table, rec, &f.studio_field, FieldValue::Int(hi + 1)).err().unwrap_or_else(|| panic!("{label}: {name} {} must refuse", hi + 1));
        assert!(err.to_string().contains("out of range"), "{label}: {name}: {err}");
        edits.push((name.to_string(), f.studio_field.clone(), before, after));
    }
    for need in ["pads[", "blocker[", "trapper[", "stick[", "masktype["] {
        assert!(cols.iter().any(|c| c.starts_with(need)), "{label}: {need} missing from {cols:?}");
    }
    assert!(cols.iter().any(|c| c.starts_with("maskid[") || c.starts_with("maskcolor[")), "{label}: a mask paint column: {cols:?}");
    if matches!(game, "nhl08" | "nhl09") {
        for need in ["maskcolor[", "hascustommask[", "maskpattern["] {
            assert!(cols.iter().any(|c| c.starts_with(need)), "{label}: {need} (R125 G1) missing from {cols:?}");
        }
        let note = |n: &str| equip(&v.equipment, n).and_then(|f| f.vocabulary_note.clone()).unwrap_or_default();
        assert!(note("maskcolor").contains("Mask paint number") && note("maskpattern").contains("0 on every shipped goalie"), "{label}: mask notes");
    }
    let view_now = s.player_view(subject.player_id).unwrap().unwrap();
    let p = s.pending_changes(200);
    for (name, fid, before, after) in &edits {
        let f = equip(&view_now.equipment, name).unwrap();
        assert_eq!(f.decoded, Some(*after), "{label}: {name} re-reads");
        let cell = p.cells.iter().find(|c| c.studio_table == table && c.record == rec && &c.studio_field == fid).unwrap_or_else(|| panic!("{label}: {name} pending cell"));
        assert_eq!((cell.before.decoded, cell.after.decoded), (Some(*before), Some(*after)), "{label}: {name} pending values");
        assert_eq!(cell.after.display, after.to_string(), "{label}: {name} pending text is the stored number");
    }
    // Undo all, redo all.
    for _ in 0..edits.len() {
        assert!(s.undo(), "{label}: undo");
    }
    let restored = s.player_view(subject.player_id).unwrap().unwrap();
    for (name, _, before, _) in &edits {
        assert_eq!(equip(&restored.equipment, name).unwrap().decoded, Some(*before), "{label}: undo restores {name}");
    }
    for _ in 0..edits.len() {
        assert!(s.redo(), "{label}: redo");
    }
    // Save → reopen on a disposable copy.
    let saved = s.save().unwrap();
    let out = tmp.0.join(format!("{}.db", label.replace('/', "-")));
    std::fs::write(&out, &saved).unwrap();
    let r = open(platform, &out, meta, game);
    let reopened = r.player_view(subject.player_id).unwrap().unwrap();
    for (name, _, _, after) in &edits {
        assert_eq!(equip(&reopened.equipment, name).unwrap().decoded, Some(*after), "{label}: {name} after save → reopen");
    }
    format!(
        "impl+tested — {} ({} goalie equipment fields); {} edited, pending shown as stored numbers, out-of-range refused, undo / redo, saved and reopened",
        cols.join(" "),
        v.equipment.len(),
        edits.len()
    )
}

#[test]
fn goalie_gear_columns_edit_review_undo_save_and_reopen_on_every_anchor() {
    let tmp = Tmp::new("sweep");
    let mut table: BTreeMap<String, String> = BTreeMap::new();
    for (platform, game, db, meta) in anchors() {
        let label = format!("{platform}/{game}");
        let row = sweep(&label, &platform, &game, &db, &meta, &tmp);
        table.insert(label, row);
    }
    eprintln!("\n| anchor | goalie gear (4.5) |");
    eprintln!("|---|---|");
    for (l, r) in &table {
        eprintln!("| {l} | {r} |");
    }
    assert_eq!(table.len(), 27);
}

/// The genuine NHL 20 install database (private fixture; identity pinned in `encoding_contract`).
#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn goalie_gear_columns_on_the_real_nhl20_database() {
    let root = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let (db, meta) = (root.join("nhl_game.db"), root.join("nhl_game-meta.xml"));
    assert!(db.is_file() && meta.is_file(), "fixture missing (fails, never skips)");
    let tmp = Tmp::new("nhl20");
    // The fixture is read-only: the sweep edits an in-memory session and saves to the owned temp root.
    let row = sweep("ps4/nhl20 (real install)", "ps4", "nhl20", &db, &meta, &tmp);
    eprintln!("| ps4/nhl20 (real install) | {row} |");
}
