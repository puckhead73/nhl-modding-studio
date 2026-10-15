//! Players 3.1 range audit on the newest titles (after-receipt-27 prompt, task 3; research C16 D2 / D3).
//!
//! R120 measured `intlcountry` and birth `state` as 8-bit (0..255) on the genuine NHL 20 and NHL 21,
//! 7-bit (0..127) through NHL 19; stored values stay ≤ 127. A wider field alone does not show that
//! Studio truncates it, so this case follows the actual metadata through the edit path: the field
//! view's declared range, an edit to a value above 127 (in range), the pending-review text, refusal
//! one past the declared range, undo / redo, save → reopen on a disposable copy, and an import write
//! of a nationality code through `lower_update` → `apply_import` → save → reopen. Height / weight
//! ranges are recorded alongside; raw height 0 is read as the minimum encodable height (54 in) and is
//! not reinterpreted (D3 stays unresolved — see receipt 28).
use std::path::{Path, PathBuf};

use studio_domain::import::{lower_update, PersonSeason};
use studio_domain::{EditSession, FieldValue, FieldView, Platform, Project, ProjectRole};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-identity-ranges-{tag}-{}", std::process::id()));
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

fn open(db: &Path, meta: &Path, game: &str) -> EditSession {
    let mut s = EditSession::begin(Project::load(db, Platform::Ps4, ProjectRole::Nhlng, meta, repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

fn by_name<'a>(fields: &'a [FieldView], name: &str) -> &'a FieldView {
    fields.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("{name}"))
}

fn audit(label: &str, db: &Path, meta: &Path, game: &str, tmp: &Tmp) {
    // A disposable copy: nothing is written to the anchor or the private fixture.
    let copy = tmp.0.join(format!("{game}.db"));
    std::fs::copy(db, &copy).unwrap();
    let mut s = open(&copy, meta, game);
    let pid = s.player_summaries().unwrap().into_iter().find(|p| p.team_id.is_some()).unwrap().player_id;
    let v = s.player_view(pid).unwrap().unwrap();
    let rec = v.bio_record;
    for name in ["height", "weight"] {
        let f = by_name(&v.all_fields, name);
        eprintln!("{label}: {name} declared {:?}..{:?}", f.range_low, f.range_high);
    }
    let mut edited = Vec::new();
    for name in ["intlcountry", "state"] {
        let f = by_name(&v.all_fields, name).clone();
        eprintln!("{label}: {name} declared {:?}..{:?}, stored {:?} ({})", f.range_low, f.range_high, f.decoded, f.display);
        assert_eq!((f.range_low, f.range_high), (Some(0), Some(255)), "{label}: {name} is 8-bit here (R120 D2)");
        assert!(f.editable, "{label}: {name} editable");
        let value = 200;
        s.set_field("ubPc", rec, &f.studio_field, FieldValue::Int(value)).unwrap_or_else(|e| panic!("{label}: {name} ← {value}: {e}"));
        let err = s.set_field("ubPc", rec, &f.studio_field, FieldValue::Int(256)).err().unwrap_or_else(|| panic!("{label}: {name} 256 must refuse"));
        assert!(err.to_string().contains("out of range"), "{label}: {name}: {err}");
        let now = by_name(&s.player_view(pid).unwrap().unwrap().all_fields, name).clone();
        assert_eq!((now.decoded, now.display.as_str()), (Some(value), "200"), "{label}: {name} not truncated, unlabelled code shown as a number");
        edited.push((name, f.studio_field.clone(), f.decoded.unwrap(), value));
    }
    let p = s.pending_changes(50);
    for (name, fid, before, after) in &edited {
        let cell = p.cells.iter().find(|c| c.studio_table == "ubPc" && c.record == rec && &c.studio_field == fid).unwrap_or_else(|| panic!("{label}: {name} pending"));
        assert_eq!((cell.before.decoded, cell.after.decoded, cell.after.display.as_str()), (Some(*before), Some(*after), "200"), "{label}: {name} pending");
    }
    assert!(s.undo() && s.undo(), "{label}: undo both");
    for (name, _, before, _) in &edited {
        assert_eq!(by_name(&s.player_view(pid).unwrap().unwrap().all_fields, name).decoded, Some(*before), "{label}: {name} undo");
    }
    assert!(s.redo() && s.redo(), "{label}: redo both");
    // Import: a nationality code through the import seam (the label lookup is covered by
    // `nationality_labels`; here the write path of a wide code).
    let db_now = TdbFile::parse(s.bytes()).unwrap();
    let person = PersonSeason { nationality: Some(109), ..Default::default() };
    let other = s.player_summaries().unwrap().into_iter().find(|p| p.player_id != pid && p.team_id.is_some()).unwrap();
    let (op, skipped) = lower_update(&db_now, s.project().meta(), s.project().enums(), &person, other.player_id).unwrap();
    assert!(!skipped.iter().any(|k| k.field == "Lelh"), "{label}: import skipped nationality: {skipped:?}");
    s.apply_import(std::slice::from_ref(&op)).unwrap();
    let saved = s.save().unwrap();
    let out = tmp.0.join(format!("{game}-reopened.db"));
    std::fs::write(&out, &saved).unwrap();
    let r = open(&out, meta, game);
    let back = r.player_view(pid).unwrap().unwrap();
    for (name, _, _, after) in &edited {
        assert_eq!(by_name(&back.all_fields, name).decoded, Some(*after), "{label}: {name} save → reopen");
    }
    let imported = r.player_view(other.player_id).unwrap().unwrap();
    let n = by_name(&imported.all_fields, "intlcountry");
    assert_eq!((n.decoded, n.display.as_str()), (Some(109), "Japan"), "{label}: imported nationality after save → reopen");
    eprintln!("{label}: intlcountry / state 200 edited, pending, 256 refused, undo / redo, import 109 (Japan), saved and reopened");
}

#[test]
fn nhl21_identity_ranges_follow_the_metadata_through_edit_import_and_save() {
    let tmp = Tmp::new("nhl21");
    let dir = repo("anchors/ps4/nhl21/databases");
    audit("ps4/nhl21", &dir.join("nhl_game.db"), &dir.join("nhl_game-meta.xml"), "nhl21", &tmp);
}

#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn real_nhl20_identity_ranges_follow_the_metadata_through_edit_import_and_save() {
    let root = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let (db, meta) = (root.join("nhl_game.db"), root.join("nhl_game-meta.xml"));
    assert!(db.is_file() && meta.is_file(), "fixture missing (fails, never skips)");
    let tmp = Tmp::new("nhl20");
    audit("ps4/nhl20 (real install)", &db, &meta, "nhl20", &tmp);
}
