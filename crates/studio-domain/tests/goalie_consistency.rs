//! Goalie `consistency` classification (coordinator review of packages 23–26, task 1; research R118
//! §2–3 as corrected by the R113–R118 review).
//!
//! The goalie AI table's `consistency` field stores 0 / 1 / 2 on every measured source (stock census,
//! NHL 08–15, PS4 15 / 21) whether declared 2-bit 0..3 (NHL 09 / 10), 6-bit 0..63 (NHL 08, 11 →
//! Legacy) or 7-bit 0..99 (PS4). Studio classified it as a **rating** by its declared range alone, so
//! the title contract's six-bit offset was added for display and edit (+50 on NHL 08, +36 on
//! NHL 11 → Legacy) while PS4 showed the stored value. These cases pin the field as a stored value on
//! every title — display, edit form, pending view, undo / redo, save → reopen, import lowering and the
//! attribute tier — while the fitted goalie Overall keeps the exact inputs it was calibrated on
//! (fingerprints below, recorded before the change). The declared range stays the write domain; the
//! stock 0..2 set is reported, never used as a clamp, and no labels are assigned to the three values.
//! `floppiness` (NHL 07–10) is deliberately left as it was.
use std::path::{Path, PathBuf};

use studio_domain::import::{lower_update, PersonSeason};
use studio_domain::{build_player_attributes, entity_fields, EditSession, FieldValue, FieldView, Platform, Project, ProjectRole};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn files(platform: &str, game: &str) -> (PathBuf, PathBuf) {
    let dir = repo(&format!("anchors/{platform}/{game}/databases"));
    if platform == "ps4" {
        (dir.join("nhl_game.db"), dir.join("nhl_game-meta.xml"))
    } else {
        (dir.join("nhlng.db"), dir.join("nhlng-meta.xml"))
    }
}

fn plat(p: &str) -> Platform {
    match p {
        "ps3" => Platform::Ps3,
        "ps4" => Platform::Ps4,
        _ => Platform::Xbox,
    }
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-consistency-{tag}-{}", std::process::id()));
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

/// An edit session over a **disposable copy** of the anchor.
fn open_copy(platform: &str, game: &str, tmp: &Tmp) -> EditSession {
    let (db, meta) = files(platform, game);
    let out = tmp.0.join(format!("{platform}-{game}.db"));
    std::fs::copy(&db, &out).unwrap();
    let mut s = EditSession::begin(Project::load(&out, plat(platform), ProjectRole::Nhlng, meta, repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

fn by_suffix<'a>(fields: &'a [FieldView], suffix: &str) -> Option<&'a FieldView> {
    fields.iter().find(|f| ["p_", "g_", "c_"].iter().any(|p| f.name.strip_prefix(p) == Some(suffix)))
}

/// `(platform, game, declared high, six-bit offset of the title's genuine ratings)`.
const TITLES: &[(&str, &str, i64, i64)] = &[
    ("xbox360", "nhl08", 63, 50),
    ("xbox360", "nhl09", 3, 50),
    ("ps3", "nhl11", 63, 36),
    ("ps3", "nhl14", 63, 36),
    ("ps3", "nhllegacy", 63, 36),
    ("ps4", "nhl15", 99, 0),
    ("ps4", "nhl21", 99, 0),
];

/// Run `f` for every title, collecting each title's failure instead of stopping at the first, so a
/// red run reports every title (the panic text of each is printed as it happens).
fn per_title(f: impl Fn(&str, &str, i64, i64)) {
    let mut failed = Vec::new();
    for (platform, game, high, sixbit) in TITLES {
        if std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| f(platform, game, *high, *sixbit))).is_err() {
            failed.push(format!("{platform}/{game}"));
        }
    }
    assert!(failed.is_empty(), "failed on {failed:?}");
}

#[test]
fn consistency_is_a_stored_value_on_every_title_and_genuine_ratings_keep_their_scale() {
    let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
    per_title(|platform, game, high, sixbit| {
        let tag = format!("{platform}/{game}");
        let (db_path, meta_path) = files(platform, game);
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let n = db.table_canonical("mHuy").unwrap().current_records as u32;
        let mut stock = std::collections::BTreeSet::new();
        for rec in 0..n {
            let fields = entity_fields(&db, &meta, &enums, "mHuy", rec);
            let c = by_suffix(&fields, "consistency").unwrap_or_else(|| panic!("{tag}: no consistency field"));
            assert_eq!((c.range_low, c.range_high), (Some(0), Some(high)), "{tag}: declared range");
            let raw = c.decoded.unwrap();
            stock.insert(raw);
            assert_eq!(c.edit_offset, None, "{tag} rec {rec}: consistency has no rating offset");
            assert_eq!(c.display, raw.to_string(), "{tag} rec {rec}: shown as the stored value");
            assert!(c.editable, "{tag}: editable within the declared range");
            let note = c.vocabulary_note.as_deref().unwrap_or("");
            assert!(note.contains("0 / 1 / 2") && note.contains("not established"), "{tag}: {note}");
            // A genuine goalie rating keeps the family scale.
            let g = by_suffix(&fields, "gsl").unwrap();
            if sixbit != 0 {
                assert_eq!(g.edit_offset, Some(sixbit), "{tag}: gsl keeps the family's six-bit scale");
            } else {
                assert!(matches!(g.edit_offset, None | Some(0)), "{tag}: PS4 gsl stored 0..99 ({:?})", g.edit_offset);
            }
            // The attribute view does not tint it as a rating.
            if rec == 0 {
                let a = build_player_attributes(&fields, "mHuy", false);
                let av = a.flat.iter().find(|v| v.field.as_ref().is_some_and(|f| f.studio_field == c.studio_field)).expect("consistency in the attribute list");
                assert!(!av.is_rating, "{tag}: not a tiered rating");
            }
        }
        eprintln!("{tag}: declared 0..{high}; stock values {stock:?}");
        assert!(stock.iter().all(|v| (0..=2).contains(v)), "{tag}: R118 stock census 0..2 ({stock:?})");
    });
}

#[test]
fn consistency_edits_imports_and_round_trips_as_a_stored_value() {
    let tmp = Tmp::new("edit");
    let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
    per_title(|platform, game, high, _| {
        let tag = format!("{platform}/{game}");
        let mut s = open_copy(platform, game, &tmp);
        let db = TdbFile::parse(s.bytes()).unwrap();
        let meta = s.project().meta();
        let fields = entity_fields(&db, &meta, &enums, "mHuy", 0);
        let c = by_suffix(&fields, "consistency").unwrap().clone();
        let fid = c.studio_field.clone();
        let raw0 = c.decoded.unwrap();
        // A declared-range value outside the stock 0..2 set is accepted (no clamp); past the declared
        // range is refused.
        let target = if high >= 3 { 3 } else { 2 };
        let target = if target == raw0 { 1 } else { target };
        s.set_field("mHuy", 0, &fid, FieldValue::Int(target)).unwrap_or_else(|e| panic!("{tag}: {e}"));
        assert!(s.set_field("mHuy", 0, &fid, FieldValue::Int(high + 1)).is_err(), "{tag}: past the declared range");
        let read = |s: &EditSession| {
            let db = TdbFile::parse(s.bytes()).unwrap();
            by_suffix(&entity_fields(&db, s.project().meta(), &enums, "mHuy", 0), "consistency").unwrap().clone()
        };
        assert_eq!((read(&s).decoded, read(&s).display.clone()), (Some(target), target.to_string()), "{tag}: stored as typed");
        let p = s.pending_changes(50);
        let cell = p.cells.iter().find(|c| c.studio_table == "mHuy" && c.studio_field == fid).expect("pending cell");
        assert_eq!((cell.before.display.as_str(), cell.after.display.as_str()), (raw0.to_string().as_str(), target.to_string().as_str()), "{tag}: pending view");
        assert!(s.undo());
        assert_eq!(read(&s).decoded, Some(raw0), "{tag}: undo");
        assert!(s.redo());
        assert_eq!(read(&s).decoded, Some(target), "{tag}: redo");
        let saved = s.save().unwrap();
        let (_, meta_path) = files(platform, game);
        let reopened = tmp.0.join(format!("{platform}-{game}-reopened.db"));
        std::fs::write(&reopened, &saved).unwrap();
        let mut r = EditSession::begin(Project::load(&reopened, plat(platform), ProjectRole::Nhlng, meta_path, repo("defs")).unwrap());
        r.set_game_id(Some(game.to_string()));
        assert_eq!(read(&r).decoded, Some(target), "{tag}: save → reopen");
    });
}

/// Import lowering (`lower_update`, the seam `apply_import` uses): the historical formula computes
/// Consistency on its native 0..2 tier (`tdb_formula` `vezina_consistency`); it must be written as
/// the stored value, not shifted by a rating offset into an out-of-range raw and skipped. The case
/// asserts the produced write (companion target, field, encoded value), then applies the import on a
/// disposable copy and checks it through save → reopen (receipt-27 review, follow-up 3).
#[test]
fn import_lowering_writes_the_consistency_tier_as_the_stored_value() {
    use tdb_edit::txn::{ImportCell, ImportOp, ImportTarget};
    let tmp = Tmp::new("import");
    let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
    per_title(|platform, game, _, _| {
        let tag = format!("{platform}/{game}");
        let mut s = open_copy(platform, game, &tmp);
        let db = TdbFile::parse(s.bytes()).unwrap();
        let fid = by_suffix(&entity_fields(&db, s.project().meta(), &enums, "mHuy", 0), "consistency").unwrap().studio_field.clone();
        let (goalie, rec) = s
            .player_summaries()
            .unwrap()
            .into_iter()
            .find_map(|p| {
                let v = s.player_view(p.player_id).ok().flatten()?;
                (v.kind == "goalie").then_some(())?;
                Some((p.player_id, v.ratings_record?))
            })
            .expect("a goalie");
        let read = |s: &EditSession| {
            let db = TdbFile::parse(s.bytes()).unwrap();
            by_suffix(&entity_fields(&db, s.project().meta(), &enums, "mHuy", rec), "consistency").unwrap().decoded
        };
        let before = read(&s).unwrap();
        let value = if before == 2 { 1 } else { 2 };
        let person = PersonSeason { ratings: vec![(fid.clone(), value)], ..Default::default() };
        let (op, skipped) = lower_update(&db, s.project().meta(), &enums, &person, goalie).unwrap();
        let hit: Vec<_> = skipped.iter().filter(|sk| sk.field == fid).map(|sk| sk.reason.clone()).collect();
        assert!(hit.is_empty(), "{tag}: import skipped consistency {value}: {hit:?}");
        let ImportOp::Update { player_id, writes } = &op else { panic!("{tag}: an update op") };
        assert_eq!(*player_id, goalie, "{tag}: the matched goalie");
        let w: Vec<_> = writes.iter().filter(|w| w.field == fid).collect();
        assert_eq!(w.len(), 1, "{tag}: exactly one consistency write");
        assert!(matches!(w[0].target, ImportTarget::Companion), "{tag}: written to the goalie companion");
        assert!(matches!(w[0].cell, ImportCell::Int(v) if v == value as u64), "{tag}: encoded as the stored value {value}: {:?}", w[0].cell);
        s.apply_import(std::slice::from_ref(&op)).unwrap_or_else(|e| panic!("{tag}: apply: {e}"));
        assert_eq!(read(&s), Some(value), "{tag}: applied");
        let saved = s.save().unwrap();
        let (_, meta_path) = files(platform, game);
        let reopened = tmp.0.join(format!("{platform}-{game}-import-reopened.db"));
        std::fs::write(&reopened, &saved).unwrap();
        let mut r = EditSession::begin(Project::load(&reopened, plat(platform), ProjectRole::Nhlng, meta_path, repo("defs")).unwrap());
        r.set_game_id(Some(game.to_string()));
        assert_eq!(read(&r), Some(value), "{tag}: import → save → reopen");
    });
}

/// The fitted goalie Overall (roster column and the Attributes tab) is unchanged by the
/// reclassification: the fingerprints were recorded on the code before the change.
#[test]
fn the_fitted_goalie_overall_keeps_its_calibrated_inputs() {
    let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
    per_title(|platform, game, _, _| {
        let tag = format!("{platform}/{game}");
        let (db_path, meta_path) = files(platform, game);
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let n = db.table_canonical("mHuy").unwrap().current_records as u32;
        let (mut attr_sum, mut attr_n) = (0i64, 0usize);
        for rec in 0..n {
            let a = build_player_attributes(&entity_fields(&db, &meta, &enums, "mHuy", rec), "mHuy", false);
            if let Some(v) = a.computed_overall {
                attr_sum += v;
                attr_n += 1;
            }
        }
        let mut s = EditSession::begin(Project::load(&db_path, plat(platform), ProjectRole::Nhlng, &meta_path, repo("defs")).unwrap());
        s.set_game_id(Some(game.to_string()));
        let (mut roster_sum, mut roster_n) = (0i64, 0usize);
        for p in s.player_summaries().unwrap() {
            if let Some(o) = p.overall {
                roster_sum += o;
                roster_n += 1;
            }
        }
        eprintln!("FINGERPRINT {tag}: attributes goalie computed Σ {attr_sum} over {attr_n}; roster overall Σ {roster_sum} over {roster_n}");
        let expected = FINGERPRINTS.iter().find(|(t, ..)| *t == tag.as_str()).map(|(_, a, an, r, rn)| (*a, *an, *r, *rn));
        assert_eq!(Some((attr_sum, attr_n, roster_sum, roster_n)), expected, "{tag}: goalie Overall inputs changed");
    });
}

/// `(title, attributes Σ, attributes n, roster Σ, roster n)` — recorded before the change.
const FINGERPRINTS: &[(&str, i64, usize, i64, usize)] = &[
    // NHL 08 / 09 / 11: the era gate reports no derived Overall (`c_` vocabulary only).
    ("xbox360/nhl08", 0, 0, 0, 0),
    ("xbox360/nhl09", 0, 0, 0, 0),
    ("ps3/nhl11", 0, 0, 0, 0),
    ("ps3/nhl14", 31_313, 523, 328_266, 5_374),
    ("ps3/nhllegacy", 35_427, 566, 362_403, 5_632),
    ("ps4/nhl15", 33_420, 559, 360_682, 5_555),
    ("ps4/nhl21", 46_381, 749, 463_869, 7_247),
];
