//! The player-row **encoding contract** (development audit 2026-09-25, F1).
//!
//! The weight base, the six-bit rating scale, the birth month / day index and the cap-hit unit
//! must come from the **schema family** of the database (and agree with the declared title), never
//! from the current cell population: a bulk edit that moves every weight or every rating must not
//! move the offset the next read or write uses. These cases run on disposable in-memory copies of
//! the committed anchors through the ordinary `EditSession` seams — bulk edit, scalar edit, undo,
//! redo, save → reopen — and assert display, edit form and stored bytes together.
//!
//! Anchor note (audit F2): the 27 anchor directories are **not** 27 independent titles —
//! `anchors/ps4/nhl20` is an NHL 19 base copy (no real NHL 20 anchor is committed). The real
//! NHL 20 witness is the private fixture case at the end of this file.
use std::path::{Path, PathBuf};

use studio_domain::{
    content_diagnostics, encoding_conflict, encoding_for_family, player_encoding, read_field,
    schema_family, BirthdateEncoding, EditSession, Evidence, FieldValue, FieldView, Platform,
    Project, ProjectRole, SchemaFamily,
};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn anchors() -> Vec<(String, String, PathBuf, PathBuf)> {
    let root = repo("anchors");
    let pairs = [
        ("NHL07NG.db", "NHL07NG-meta.xml"),
        ("nhlng.db", "nhlng-meta.xml"),
        ("nhl_game.db", "nhl_game-meta.xml"),
    ];
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms.iter().filter(|e| e.path().is_dir()) {
        let mut games: Vec<_> = std::fs::read_dir(platform.path())
            .unwrap()
            .flatten()
            .collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs
                .iter()
                .find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file())
            {
                out.push((
                    platform.file_name().to_string_lossy().into_owned(),
                    game.file_name().to_string_lossy().into_owned(),
                    dbs.join(db),
                    dbs.join(meta),
                ));
            }
        }
    }
    assert_eq!(
        out.len(),
        27,
        "27 anchor directories (ps4/nhl20 is an NHL 19 base copy)"
    );
    out
}

fn platform_of(dir: &str) -> Platform {
    match dir {
        "ps3" => Platform::Ps3,
        "ps4" => Platform::Ps4,
        "xbox360" => Platform::Xbox,
        other => panic!("{other}"),
    }
}

/// The family each anchor directory is expected to carry, from its title, independent of its cells.
fn expected_family(platform: &str, game: &str) -> SchemaFamily {
    if platform == "ps4" {
        return SchemaFamily::Ps4;
    }
    match game {
        "nhl07" => SchemaFamily::Gen6Nhl07,
        "nhl08" => SchemaFamily::Gen7Nhl08,
        "nhl09" => SchemaFamily::Gen7Nhl09,
        "nhl10" => SchemaFamily::Gen7Nhl10,
        _ => SchemaFamily::Gen7Nhl11ToLegacy,
    }
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-encoding-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn open(platform: &str, game: &str) -> EditSession {
    let dir = repo(&format!("anchors/{platform}/{game}/databases"));
    let (db, meta) = if game == "nhl07" {
        ("NHL07NG.db", "NHL07NG-meta.xml")
    } else if platform == "ps4" {
        ("nhl_game.db", "nhl_game-meta.xml")
    } else {
        ("nhlng.db", "nhlng-meta.xml")
    };
    let mut sess = EditSession::begin(
        Project::load(
            dir.join(db),
            platform_of(platform),
            ProjectRole::Nhlng,
            dir.join(meta),
            repo("defs"),
        )
        .unwrap(),
    );
    sess.set_game_id(Some(game.to_string()));
    sess
}

fn open_bytes(bytes: &[u8], platform: &str, game: &str, tmp: &Tmp, tag: &str) -> EditSession {
    let dir = repo(&format!("anchors/{platform}/{game}/databases"));
    let meta = if game == "nhl07" {
        "NHL07NG-meta.xml"
    } else if platform == "ps4" {
        "nhl_game-meta.xml"
    } else {
        "nhlng-meta.xml"
    };
    let out = tmp.0.join(format!("{platform}-{game}-{tag}.db"));
    std::fs::write(&out, bytes).unwrap();
    let mut sess = EditSession::begin(
        Project::load(
            &out,
            platform_of(platform),
            ProjectRole::Nhlng,
            dir.join(meta),
            repo("defs"),
        )
        .unwrap(),
    );
    sess.set_game_id(Some(game.to_string()));
    sess
}

fn player_named(sess: &EditSession, first: &str, last: &str) -> u64 {
    sess.player_summaries()
        .unwrap()
        .iter()
        .find(|p| p.first_name == first && p.last_name == last)
        .unwrap_or_else(|| panic!("{first} {last} not rostered"))
        .player_id
}

fn field<'a>(fields: &'a [FieldView], id: &str) -> &'a FieldView {
    fields
        .iter()
        .find(|f| f.studio_field == id)
        .unwrap_or_else(|| panic!("field {id}"))
}

fn raw_of(sess: &EditSession, table: &str, rec: u32, id: &str) -> u64 {
    let db = TdbFile::parse(sess.bytes()).unwrap();
    let t = db.table_canonical(table).unwrap();
    let f = t
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == id)
        .unwrap();
    db.read_u64(t, rec, f).unwrap()
}

/// Every bio record's `sNZW` set to `raw` in one bulk operation (the schedule import's seam).
fn bulk_weights(sess: &mut EditSession, raw: i64) {
    let db = TdbFile::parse(sess.bytes()).unwrap();
    let n = db.table_canonical("ubPc").unwrap().current_records as u32;
    let rows: Vec<(u32, Vec<(String, i64)>)> = (0..n)
        .map(|r| (r, vec![("sNZW".to_string(), raw)]))
        .collect();
    sess.set_cells_int_rows("ubPc", &rows).unwrap();
}

/// Every six-bit rating cell of the first `records` skater-companion rows moved to `f(raw)`.
fn bulk_ratings(sess: &mut EditSession, records: u32, f: impl Fn(i64) -> i64) {
    let db = TdbFile::parse(sess.bytes()).unwrap();
    let meta = sess.project().meta();
    let t = db.table_canonical("dSvy").unwrap();
    let rating_fields: Vec<(String, &tdb_core::FieldDescriptor)> = t
        .fields
        .iter()
        .filter(|fd| fd.bit_width == 6)
        .filter(|fd| {
            meta.field(&t.id.raw(), &fd.id.raw()).is_some_and(|m| {
                let n = m.name.as_str();
                (n.starts_with("p_") || n.starts_with("c_"))
                    && m.range_low == Some(0)
                    && m.range_high == Some(63)
                    && !n.ends_with("fighting")
                    && !n.ends_with("style")
                    && !n.ends_with("potential")
            })
        })
        .map(|fd| (fd.id.canonical(db.endian), fd))
        .collect();
    assert!(rating_fields.len() > 10, "rating fields");
    let n = (t.current_records as u32).min(records);
    let rows: Vec<(u32, Vec<(String, i64)>)> = (0..n)
        .map(|r| {
            (
                r,
                rating_fields
                    .iter()
                    .map(|(id, fd)| (id.clone(), f(db.read_u64(t, r, fd).unwrap() as i64)))
                    .collect(),
            )
        })
        .collect();
    sess.set_cells_int_rows("dSvy", &rows).unwrap();
}

/// The studio id of a skater-companion cell that is not a six-bit rating (the style field where the
/// title has one, else the first non-six-bit column).
fn non_rating_id(sess: &EditSession) -> String {
    let db = TdbFile::parse(sess.bytes()).unwrap();
    let meta = sess.project().meta();
    let t = db.table_canonical("dSvy").unwrap();
    t.fields
        .iter()
        .find(|f| {
            meta.field(&t.id.raw(), &f.id.raw())
                .is_some_and(|m| m.name == "playerstyle" || m.name == "p_style")
        })
        .or_else(|| t.fields.iter().find(|f| f.bit_width != 6))
        .map(|f| f.id.canonical(db.endian))
        .unwrap()
}

// ------------------------------------------------------------------------------------------------

/// The schema classifier names the title family of every anchor from its shape, the declared title
/// agrees (no conflict), a wrong title conflicts in words, and the contract values are the pinned
/// ones. The population diagnostics are printed per anchor; on stock data they agree everywhere.
#[test]
fn every_anchor_resolves_to_its_title_family_and_the_pinned_contract() {
    eprintln!("\n| anchor | family | weight | 6-bit ratings | birthdate | cap-hit unit | population diagnostics |");
    eprintln!("|---|---|---|---|---|---|---|");
    for (platform, game, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let fam =
            schema_family(&db, &meta).unwrap_or_else(|| panic!("{platform}/{game}: unresolved"));
        assert_eq!(fam, expected_family(&platform, &game), "{platform}/{game}");
        let enc = player_encoding(&db, &meta).unwrap();
        let p = platform_of(&platform);
        assert_eq!(
            SchemaFamily::of_game(&game, p),
            Some(fam),
            "{platform}/{game}: of_game"
        );
        assert_eq!(
            encoding_conflict(&db, &meta, Some(&game), p),
            None,
            "{platform}/{game}"
        );
        assert_eq!(encoding_conflict(&db, &meta, None, p), None);
        // A wrong declared title conflicts, naming both families.
        let (wrong, wrong_p) = if fam == SchemaFamily::Ps4 {
            ("nhl09", Platform::Ps3)
        } else {
            ("nhl15", Platform::Ps4)
        };
        let c = encoding_conflict(&db, &meta, Some(wrong), wrong_p).unwrap();
        assert!(c.contains(fam.label()) && c.contains("not editable"), "{c}");
        // Pinned values (receipts 11 / 13 / 17).
        let exp_weight = if matches!(fam, SchemaFamily::Gen7Nhl11ToLegacy | SchemaFamily::Ps4) {
            120
        } else {
            150
        };
        assert_eq!(enc.weight_base, exp_weight, "{platform}/{game}");
        let exp_rating = match fam {
            SchemaFamily::Ps4 => 0,
            SchemaFamily::Gen7Nhl11ToLegacy => 36,
            _ => 50,
        };
        assert_eq!(enc.sixbit_rating_offset, exp_rating, "{platform}/{game}");
        let exp_bd = if exp_weight == 150 {
            BirthdateEncoding::OneIndexed
        } else {
            BirthdateEncoding::ZeroIndexed
        };
        assert_eq!(enc.birthdate, exp_bd, "{platform}/{game}");
        let exp_cap = match fam {
            SchemaFamily::Gen6Nhl07
            | SchemaFamily::Gen7Nhl08
            | SchemaFamily::Gen7Nhl09
            | SchemaFamily::Gen7Nhl10 => Some(25_000),
            _ => Some(5_000),
        };
        assert_eq!(enc.cap_hit_unit, exp_cap, "{platform}/{game}");
        if matches!(
            fam,
            SchemaFamily::Gen7Nhl08 | SchemaFamily::Gen7Nhl09 | SchemaFamily::Gen7Nhl10
        ) {
            assert_eq!(
                enc.rating_evidence,
                Evidence::Inferred,
                "{platform}/{game}: the +50 reading is provisional"
            );
        }
        let diag = content_diagnostics(&db, &meta);
        eprintln!(
            "| {platform}/{game} | {} | +{} ({}) | +{} ({}) | {:?} | {} | {} |",
            fam.label(),
            enc.weight_base,
            enc.weight_evidence.label(),
            enc.sixbit_rating_offset,
            enc.rating_evidence.label(),
            enc.birthdate,
            enc.cap_hit_unit
                .map(|u| format!("×${u}"))
                .unwrap_or_else(|| "unwitnessed — raw".into()),
            if diag.is_empty() {
                "agree".to_string()
            } else {
                diag.join("; ")
            }
        );
        assert!(
            diag.is_empty(),
            "{platform}/{game}: stock cells disagree with the contract: {diag:?}"
        );
    }
    // The contract table itself: every family has a value for every item.
    for fam in [
        SchemaFamily::Gen6Nhl07,
        SchemaFamily::Gen7Nhl08,
        SchemaFamily::Gen7Nhl09,
        SchemaFamily::Gen7Nhl10,
        SchemaFamily::Gen7Nhl11ToLegacy,
        SchemaFamily::Ps4,
    ] {
        let e = encoding_for_family(fam);
        assert_eq!(e.family, fam);
        assert!(e.weight_base == 120 || e.weight_base == 150);
    }
}

/// F1 red → green: a bulk weight edit that moves every stored weight across the old median
/// threshold must leave the base, the display, the edit form and a re-applied human value exactly
/// where the title's contract puts them — through undo, redo and save → reopen — and touch no
/// other cell.
#[test]
fn a_bulk_weight_edit_cannot_move_the_weight_base() {
    let tmp = Tmp::new("weight");
    // (platform, game, base, bulk pounds that would flip the old median rule)
    for (platform, game, base, bulk_lb) in [
        ("ps3", "nhl15", 120i64, 170i64),
        ("xbox360", "nhl09", 150, 250),
        ("ps4", "nhl17", 120, 170),
    ] {
        let mut sess = open(platform, game);
        let kukkonen = player_named(&sess, "Lasse", "Kukkonen");
        let v0 = sess.player_view(kukkonen).unwrap().unwrap();
        let rec = v0.bio_record;
        let w0 = field(&v0.all_fields, "sNZW");
        let raw0 = raw_of(&sess, "ubPc", rec, "sNZW");
        assert_eq!(
            w0.edit_offset,
            Some(base),
            "{platform}/{game}: stock edit form"
        );
        assert_eq!(w0.decoded, Some(raw0 as i64));
        let height0 = raw_of(&sess, "ubPc", rec, "ypBQ");
        let stock_lb = raw0 as i64 + base;
        assert_eq!(
            v0.header.weight.as_deref(),
            Some(format!("{stock_lb} lb").as_str()),
            "{platform}/{game}"
        );

        // The threshold-crossing bulk edit (every player to `bulk_lb`).
        bulk_weights(&mut sess, bulk_lb - base);
        let v1 = sess.player_view(kukkonen).unwrap().unwrap();
        let w1 = field(&v1.all_fields, "sNZW");
        assert_eq!(
            w1.edit_offset,
            Some(base),
            "{platform}/{game}: the base moved with the population"
        );
        assert_eq!(
            w1.display,
            format!("{bulk_lb} lb"),
            "{platform}/{game}: display after the bulk edit"
        );
        assert_eq!(
            v1.header.weight.as_deref(),
            Some(format!("{bulk_lb} lb").as_str()),
            "{platform}/{game}: header"
        );
        assert_eq!(
            raw_of(&sess, "ubPc", rec, "ypBQ"),
            height0,
            "{platform}/{game}: unrelated cell untouched"
        );

        // Re-applying the stock human value through the edit form reproduces the stock raw byte.
        let staged = stock_lb - w1.edit_offset.unwrap();
        sess.set_field("ubPc", rec, "sNZW", FieldValue::Int(staged))
            .unwrap();
        assert_eq!(
            raw_of(&sess, "ubPc", rec, "sNZW"),
            raw0,
            "{platform}/{game}: same human value → same raw"
        );
        let v2 = sess.player_view(kukkonen).unwrap().unwrap();
        assert_eq!(
            v2.header.weight.as_deref(),
            Some(format!("{stock_lb} lb").as_str())
        );

        // Undo / redo keep the contract and the bytes in step.
        assert!(sess.undo());
        assert_eq!(raw_of(&sess, "ubPc", rec, "sNZW"), (bulk_lb - base) as u64);
        assert_eq!(
            field(
                &sess.player_view(kukkonen).unwrap().unwrap().all_fields,
                "sNZW"
            )
            .edit_offset,
            Some(base)
        );
        assert!(sess.redo());
        assert_eq!(raw_of(&sess, "ubPc", rec, "sNZW"), raw0);
        assert!(sess.undo() && sess.undo(), "back to stock");
        assert_eq!(raw_of(&sess, "ubPc", rec, "sNZW"), raw0);

        // Save the bulk-edited population and reopen it: the reopened file must not re-learn a base.
        bulk_weights(&mut sess, bulk_lb - base);
        let saved = sess.save().unwrap();
        let re = open_bytes(&saved, platform, game, &tmp, "bulk-weight");
        let vr = re.player_view(kukkonen).unwrap().unwrap();
        let wr = field(&vr.all_fields, "sNZW");
        assert_eq!(
            wr.edit_offset,
            Some(base),
            "{platform}/{game}: reopened file re-learned the base"
        );
        assert_eq!(
            wr.display,
            format!("{bulk_lb} lb"),
            "{platform}/{game}: reopened display"
        );
        assert_eq!(raw_of(&re, "ubPc", rec, "ypBQ"), height0);
        let diag = content_diagnostics(&TdbFile::parse(re.bytes()).unwrap(), re.project().meta());
        eprintln!("{platform}/{game} after the bulk edit, diagnostics: {diag:?}");
    }
}

/// F1 red → green: an all-zero (unset) weight population is not a reason to pick another base.
#[test]
fn an_all_zero_weight_population_keeps_the_contract_base() {
    for (platform, game, base) in [("xbox360", "nhl09", 150i64), ("ps3", "nhl14", 120)] {
        let mut sess = open(platform, game);
        let kukkonen = player_named(&sess, "Lasse", "Kukkonen");
        bulk_weights(&mut sess, 0);
        let v = sess.player_view(kukkonen).unwrap().unwrap();
        let w = field(&v.all_fields, "sNZW");
        assert_eq!(w.edit_offset, Some(base), "{platform}/{game}");
        assert_eq!(v.header.weight, None, "{platform}/{game}: 0 = unset");
        // Staging 190 lb on the empty population writes the contract's raw.
        sess.set_field(
            "ubPc",
            v.bio_record,
            "sNZW",
            FieldValue::Int(190 - w.edit_offset.unwrap()),
        )
        .unwrap();
        assert_eq!(
            raw_of(&sess, "ubPc", v.bio_record, "sNZW") as i64,
            190 - base,
            "{platform}/{game}"
        );
    }
}

/// F1 red → green: lowering every sampled rating below 50 on a 36-scale title, or raising them
/// above 49 on a 50-scale title, must not move the scale — display, edit form, re-applied human
/// value and save → reopen all stay on the family contract.
#[test]
fn a_bulk_rating_edit_cannot_move_the_rating_scale() {
    let tmp = Tmp::new("ratings");
    for (platform, game, offset, mover) in [
        (
            "xbox360",
            "nhl14",
            36i64,
            (|r: i64| r.min(40)) as fn(i64) -> i64,
        ),
        ("ps3", "nhl12", 36, |r: i64| r.min(40)),
        ("xbox360", "nhl09", 50, |r: i64| r.max(60)),
    ] {
        let mut sess = open(platform, game);
        let subject = sess
            .player_summaries()
            .unwrap()
            .into_iter()
            .find(|p| !p.is_goalie && p.team_id.is_some() && p.bio_record < 500)
            .unwrap();
        let v0 = sess.player_view(subject.player_id).unwrap().unwrap();
        let speed0 = v0
            .ratings
            .iter()
            .find(|f| f.name == "c_speed" || f.name == "p_speed")
            .unwrap()
            .clone();
        let rec = v0.ratings_record.unwrap();
        assert_eq!(
            speed0.edit_offset,
            Some(offset),
            "{platform}/{game}: stock scale ({} {:?}..{:?} decoded {:?} display {:?})",
            speed0.name,
            speed0.range_low,
            speed0.range_high,
            speed0.decoded,
            speed0.display
        );
        assert_eq!(
            speed0.display,
            (speed0.decoded.unwrap() + offset).to_string()
        );
        let other_id = non_rating_id(&sess); // a companion cell that is not a rating
        let other0 = raw_of(&sess, "dSvy", rec, &other_id);

        // Move every rating cell of the first 1 200 companion rows across the old 1 % threshold.
        bulk_ratings(&mut sess, 1200, mover);
        let v1 = sess.player_view(subject.player_id).unwrap().unwrap();
        let speed1 = v1
            .ratings
            .iter()
            .find(|f| f.name == "c_speed" || f.name == "p_speed")
            .unwrap();
        assert_eq!(
            speed1.edit_offset,
            Some(offset),
            "{platform}/{game}: the scale moved with the population"
        );
        assert_eq!(
            speed1.display,
            (speed1.decoded.unwrap() + offset).to_string(),
            "{platform}/{game}"
        );
        assert_eq!(
            raw_of(&sess, "dSvy", rec, &other_id),
            other0,
            "{platform}/{game}: non-rating cell untouched"
        );
        // The roster list's derived Overall reads the same scale (its reader has its own probe seat).
        let list = sess.player_summaries().unwrap();
        let mine = list
            .iter()
            .find(|p| p.player_id == subject.player_id)
            .unwrap();
        if let Some(o) = mine.overall {
            assert!((1..=99).contains(&o), "{platform}/{game}: overall {o}");
        }

        // Re-apply a human 88 through the edit form → raw 88 − offset, and read back 88.
        sess.set_field(
            "dSvy",
            rec,
            &speed1.studio_field,
            FieldValue::Int(88 - offset),
        )
        .unwrap();
        assert_eq!(
            raw_of(&sess, "dSvy", rec, &speed1.studio_field) as i64,
            88 - offset
        );
        let v2 = sess.player_view(subject.player_id).unwrap().unwrap();
        assert_eq!(
            v2.ratings
                .iter()
                .find(|f| f.name == "c_speed" || f.name == "p_speed")
                .unwrap()
                .display,
            "88"
        );
        assert!(sess.undo());
        assert_eq!(
            raw_of(&sess, "dSvy", rec, &speed1.studio_field),
            mover(speed0.decoded.unwrap()) as u64
        );
        assert!(sess.redo());

        let saved = sess.save().unwrap();
        let re = open_bytes(&saved, platform, game, &tmp, "bulk-ratings");
        let vr = re.player_view(subject.player_id).unwrap().unwrap();
        let sr = vr
            .ratings
            .iter()
            .find(|f| f.name == "c_speed" || f.name == "p_speed")
            .unwrap();
        assert_eq!(
            sr.edit_offset,
            Some(offset),
            "{platform}/{game}: reopened file re-learned the scale"
        );
        assert_eq!(sr.display, "88", "{platform}/{game}: reopened display");
        let diag = content_diagnostics(&TdbFile::parse(re.bytes()).unwrap(), re.project().meta());
        eprintln!("{platform}/{game} after the bulk rating edit, diagnostics: {diag:?}");
    }
}

/// A project opened as a title whose family disagrees with the schema refuses to write the encoded
/// cells — in words, leaving bytes and history untouched — while unrelated cells still write; the
/// matching title lifts the refusal.
#[test]
fn a_declared_title_that_disagrees_with_the_schema_refuses_encoded_writes_in_words() {
    let mut sess = open("ps3", "nhl15");
    let kukkonen = player_named(&sess, "Lasse", "Kukkonen");
    let v = sess.player_view(kukkonen).unwrap().unwrap();
    let rec = v.bio_record;
    let before = sess.bytes().to_vec();
    sess.set_game_id(Some("nhl09".to_string()));
    for (table, r, fld) in [("ubPc", rec, "sNZW"), ("ubPc", rec, "JKLp")] {
        let err = sess
            .set_field(table, r, fld, FieldValue::Int(10))
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("opened as nhl09")
                && err.contains("NHL 09")
                && err.contains("NHL 11 – NHL Legacy"),
            "{err}"
        );
        assert!(
            !err.contains("sNZW") && !err.contains("JKLp"),
            "raw ids stay out of the text: {err}"
        );
    }
    let speed = v
        .ratings
        .iter()
        .find(|f| f.name == "c_speed" || f.name == "p_speed")
        .unwrap();
    let err = sess
        .set_field(
            "dSvy",
            v.ratings_record.unwrap(),
            &speed.studio_field,
            FieldValue::Int(10),
        )
        .unwrap_err()
        .to_string();
    assert!(err.contains("not editable"), "{err}");
    assert_eq!(sess.bytes(), &before[..], "refusals leave the bytes alone");
    assert_eq!(sess.undo_depth(), 0);
    // An unrelated cell (jersey handedness is not title-encoded) still writes under the conflict.
    let hand = field(&v.all_fields, "GRkp");
    let other = if hand.decoded == Some(0) { 1 } else { 0 };
    sess.set_field("ubPc", rec, "GRkp", FieldValue::Int(other))
        .unwrap();
    assert!(sess.undo());
    // The matching title lifts the refusal.
    sess.set_game_id(Some("nhl15".to_string()));
    sess.set_field("ubPc", rec, "sNZW", FieldValue::Int(70))
        .unwrap();
    assert!(sess.undo());
    assert_eq!(sess.bytes(), &before[..]);
}

/// The import lowering reads its offsets from the same contract: after the bulk weight edit the
/// bio table's weight field still lowers 190 lb to raw 190 − base (`read_field` is the import's seam).
#[test]
fn the_import_seam_reads_the_contract_offset_after_a_bulk_edit() {
    for (platform, game, base) in [("ps3", "nhl15", 120i64), ("xbox360", "nhl10", 150)] {
        let mut sess = open(platform, game);
        bulk_weights(&mut sess, if base == 120 { 50 } else { 100 });
        let db = TdbFile::parse(sess.bytes()).unwrap();
        let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let f = bio
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "sNZW")
            .unwrap();
        let fv = read_field(&db, sess.project().meta(), &enums, bio, 0, f);
        assert_eq!(fv.edit_offset, Some(base), "{platform}/{game}: import seam");
    }
}

/// F2: the real NHL 20 database (private fixture, recorded identity) carries the PS4 contract and
/// the same witnessed cap hits as the NHL 19 base copy under `anchors/ps4/nhl20`.
#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn the_real_nhl20_database_carries_the_ps4_contract() {
    let root = std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set");
    let root = PathBuf::from(root).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    let db_path = root.join("nhl_game.db");
    let meta_path = root.join("nhl_game-meta.xml");
    let bytes = std::fs::read(&db_path).unwrap_or_else(|e| panic!("{}: {e}", db_path.display()));
    assert_eq!(
        tdb_txn::FileIdentity::of_bytes(&bytes).sha256,
        "1e2004784bccfdd6bed6e82533a67595798379f2c2d763c0636d9d370977a57b",
        "real NHL 20 nhl_game.db identity"
    );
    let meta_text = std::fs::read_to_string(&meta_path).unwrap();
    assert_eq!(
        tdb_txn::FileIdentity::of_bytes(meta_text.as_bytes()).sha256,
        "2fac0e733c9792038a64f966f277b163136517438509283e7f646d2920ad1f93",
        "real NHL 20 nhl_game-meta.xml identity"
    );
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&meta_text).unwrap();
    assert_eq!(schema_family(&db, &meta), Some(SchemaFamily::Ps4));
    let enc = player_encoding(&db, &meta).unwrap();
    assert_eq!(
        (enc.weight_base, enc.sixbit_rating_offset, enc.cap_hit_unit),
        (120, 0, Some(5_000))
    );
    assert_eq!(
        encoding_conflict(&db, &meta, Some("nhl20"), Platform::Ps4),
        None
    );
    assert!(content_diagnostics(&db, &meta).is_empty());
    let mut sess = EditSession::begin(
        Project::load(
            &db_path,
            Platform::Ps4,
            ProjectRole::Nhlng,
            &meta_path,
            repo("defs"),
        )
        .unwrap(),
    );
    sess.set_game_id(Some("nhl20".to_string()));
    let mcdavid = player_named(&sess, "Connor", "McDavid");
    let v = sess.player_view(mcdavid).unwrap().unwrap();
    assert_eq!(
        v.data.cap_hit.as_deref(),
        Some("$12,500,000"),
        "real NHL 20 McDavid"
    );
    let kukkonen = sess
        .player_summaries()
        .unwrap()
        .iter()
        .find(|p| p.last_name == "Kukkonen")
        .map(|p| p.player_id);
    if let Some(k) = kukkonen {
        let kv = sess.player_view(k).unwrap().unwrap();
        eprintln!("real NHL 20 Kukkonen: {:?}", kv.header.weight);
    }
}

// ------------------------------------------------------------------------------------------------
// Review 2026-09-26 F2: byte order alone is not a layout. The classifier checks a bounded signature
// of the encoded descriptors; altered or contradictory descriptors are unresolved (raw, read-only),
// and a declared title / platform with no known encoding is a conflict, never agreement.

fn anchor_paths(platform: &str, game: &str) -> (PathBuf, PathBuf) {
    let dir = repo(&format!("anchors/{platform}/{game}/databases"));
    if platform == "ps4" {
        (dir.join("nhl_game.db"), dir.join("nhl_game-meta.xml"))
    } else {
        (dir.join("nhlng.db"), dir.join("nhlng-meta.xml"))
    }
}

/// The anchor's metadata with every `(from, to)` substitution applied (each must occur).
fn altered_meta(platform: &str, game: &str, edits: &[(&str, &str)]) -> String {
    let (_, meta) = anchor_paths(platform, game);
    let mut text = std::fs::read_to_string(meta).unwrap();
    for (from, to) in edits {
        assert!(text.contains(from), "{platform}/{game}: `{from}` not in the metadata");
        text = text.replace(from, to);
    }
    text
}

fn classify(platform: &str, game: &str, meta_text: &str) -> Option<SchemaFamily> {
    let (db, _) = anchor_paths(platform, game);
    let bytes = std::fs::read(db).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    schema_family(&db, &FieldMetaIndex::from_xml(meta_text).unwrap())
}

const PS4_SPEED: &str = r#"name="c_speed" shortname="AdEb" type="FIELDTYPE_INTEGER" depth="7" null="False" rangehigh="99""#;
const PS4_MONTH: &str = r#"name="month" shortname="JKLp" type="FIELDTYPE_INTEGER" depth="4" null="False" rangehigh="11""#;
const PS4_WEIGHT: &str = r#"name="weight" shortname="sNZW" type="FIELDTYPE_INTEGER""#;
const GEN7_DRAFT: &str = r#"name="draftyear" shortname="WzKY""#;

#[test]
fn altered_or_contradictory_descriptors_are_unresolved_and_the_stock_signature_is_not() {
    // Sanity: the untouched metadata classifies.
    assert_eq!(classify("ps4", "nhl17", &altered_meta("ps4", "nhl17", &[])), Some(SchemaFamily::Ps4));
    assert_eq!(classify("ps3", "nhl15", &altered_meta("ps3", "nhl15", &[])), Some(SchemaFamily::Gen7Nhl11ToLegacy));
    let cases: &[(&str, &str, &str, &str, &str)] = &[
        ("ps4", "nhl17", "a seven-bit rating declared 0..63", PS4_SPEED, &PS4_SPEED.replace("99", "63")),
        ("ps4", "nhl17", "month declared 0..12", PS4_MONTH, &PS4_MONTH.replace("\"11\"", "\"12\"")),
        ("ps4", "nhl17", "weight renamed", PS4_WEIGHT, &PS4_WEIGHT.replace("\"weight\"", "\"mass\"")),
        ("ps4", "nhl17", "weight re-typed as a string", PS4_WEIGHT, &PS4_WEIGHT.replace("INTEGER", "STRING")),
        ("ps3", "nhl15", "no draft-year field of either kind", GEN7_DRAFT, &GEN7_DRAFT.replace("draftyear", "draftslot")),
    ];
    for (platform, game, what, from, to) in cases {
        let text = altered_meta(platform, game, &[(from, to)]);
        assert_eq!(classify(platform, game, &text), None, "{platform}/{game}: {what} must be unresolved");
    }
    // Six-bit fields declared 0..99 on a big-endian title contradict their width.
    let text = std::fs::read_to_string(anchor_paths("ps3", "nhl15").1).unwrap();
    let speed = text.match_indices(r#"name="c_speed""#).count();
    assert!(speed > 0);
    let altered = text.replace(r#"depth="6" null="False" rangehigh="63""#, r#"depth="6" null="False" rangehigh="99""#);
    assert_ne!(altered, text);
    assert_eq!(classify("ps3", "nhl15", &altered), None, "six-bit ratings declared 0..99");
}

#[test]
fn an_unresolved_layout_shows_encoded_cells_raw_and_refuses_them_while_other_cells_write() {
    let tmp = Tmp::new("unresolved");
    let meta_path = tmp.0.join("nhl_game-meta.xml");
    std::fs::write(&meta_path, altered_meta("ps4", "nhl17", &[(PS4_MONTH, &PS4_MONTH.replace("\"11\"", "\"12\""))])).unwrap();
    let (db_path, _) = anchor_paths("ps4", "nhl17");
    let mut sess = EditSession::begin(Project::load(&db_path, Platform::Ps4, ProjectRole::Nhlng, &meta_path, repo("defs")).unwrap());
    sess.set_game_id(Some("nhl17".into()));
    let kukkonen = player_named(&sess, "Lasse", "Kukkonen");
    let v = sess.player_view(kukkonen).unwrap().unwrap();
    let w = field(&v.all_fields, "sNZW");
    assert!(!w.editable && w.edit_offset.is_none(), "weight must be read-only when unresolved");
    assert!(w.display.contains("unresolved"), "{}", w.display);
    assert_eq!(v.header.weight, None, "no weight composed on a guessed base");
    let before = sess.bytes().to_vec();
    let e = sess.set_field("ubPc", v.bio_record, "sNZW", FieldValue::Int(70)).unwrap_err().to_string();
    assert!(e.contains("unresolved"), "{e}");
    assert_eq!(sess.bytes(), &before[..]);
    let hand = field(&v.all_fields, "GRkp");
    let other = if hand.decoded == Some(0) { 1 } else { 0 };
    sess.set_field("ubPc", v.bio_record, "GRkp", FieldValue::Int(other)).unwrap();
    assert!(sess.undo());
}

#[test]
fn a_declared_title_or_platform_without_a_known_encoding_is_a_conflict_not_agreement() {
    // PS3 NHL 15 opened as a PS4-only title; PS4 NHL 17 opened as a gen7 title.
    for (platform, game, declared) in [("ps3", "nhl15", "nhl16"), ("ps4", "nhl17", "nhl09"), ("ps3", "nhl15", "nhl99")] {
        let (db_path, meta_path) = anchor_paths(platform, game);
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let c = encoding_conflict(&db, &meta, Some(declared), platform_of(platform)).unwrap_or_else(|| panic!("{platform}/{game} as {declared}"));
        assert!(c.contains(declared) && c.contains("not editable"), "{c}");
        let mut sess = open(platform, game);
        sess.set_game_id(Some(declared.to_string()));
        let kukkonen = player_named(&sess, "Lasse", "Kukkonen");
        let rec = sess.player_view(kukkonen).unwrap().unwrap().bio_record;
        assert!(sess.set_field("ubPc", rec, "sNZW", FieldValue::Int(70)).is_err());
        // The semantic import seam refuses too (an empty cart is enough to reach the guard).
        let e = sess.apply_import(&[]).unwrap_err().to_string();
        assert!(e.contains("cannot apply the import"), "{e}");
    }
    // Legacy Recompiled declared on its own platform agrees with the Legacy layout.
    let (db_path, meta_path) = anchor_paths("xbox360", "nhllegacy");
    let bytes = std::fs::read(&db_path).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
    assert_eq!(encoding_conflict(&db, &meta, Some("nhllegacy-recompiled"), Platform::Recomp), None);
    // Receipt 59: the catalogue's Recompiled id is `legacyrecomp` (app/src/lib/catalog.ts); a project opened with it had its
    // weight / ratings / birth dates / contract dollars refused as "not a title with a known encoding".
    assert_eq!(encoding_conflict(&db, &meta, Some("legacyrecomp"), Platform::Recomp), None);
    // No declared title stays "unknown title", not a conflict (the legacy file-pick path).
    assert_eq!(encoding_conflict(&db, &meta, None, Platform::Xbox), None);
}
