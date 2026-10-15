//! Players 3.x — per-title skater style labels on the roster cell (research R122 = 2026-09-26 handoff 15,
//! observations S1 / S2; 27 Sep 2026).
//!
//! Skater styles are numbered in three eras (7 codes on NHL 07–09, 9 on NHL 10, 10 from NHL 11). The
//! roster style cell (`eGlu.QgFs`, the Lines page and the roster table) used to carry the NHL Legacy ten-
//! label table on every title: on NHL 08 / 09 code 3 read "Grinder" on the player page (NHL 07 vocabulary,
//! `dSvy.mQoE`) and "Enforcer Defenseman" on the roster cell; on NHL 10 forwards with code 4 read
//! "Two-Way Defenseman". Now: NHL 08 / 09 roster cells use the player page's NHL 07 vocabulary (same code,
//! same label in both places), NHL 07 and NHL 10 show numbers, NHL 11 onward keep the Legacy labels.
use std::path::{Path, PathBuf};

use studio_domain::{read_field, EditSession, FieldView, Platform, Project, ProjectRole};
use tdb_core::TdbFile;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn open(platform: &str, game: &str) -> EditSession {
    let dir = repo(&format!("anchors/{platform}/{game}/databases"));
    let (db, meta) = if platform == "ps4" { (dir.join("nhl_game.db"), dir.join("nhl_game-meta.xml")) } else if game == "nhl07" { (dir.join("NHL07NG.db"), dir.join("NHL07NG-meta.xml")) } else { (dir.join("nhlng.db"), dir.join("nhlng-meta.xml")) };
    let p = match platform {
        "ps3" => Platform::Ps3,
        "ps4" => Platform::Ps4,
        _ => Platform::Xbox,
    };
    let mut s = EditSession::begin(Project::load(db, p, ProjectRole::Nhlng, meta, repo("defs")).unwrap());
    s.set_game_id(Some(game.to_string()));
    s
}

/// Every record's field view of `table.field` whose stored code is `code`.
fn cells(s: &EditSession, table: &str, field: &str, code: i64) -> Vec<FieldView> {
    let db = TdbFile::parse(s.bytes()).unwrap();
    let Some(t) = db.table_canonical(table) else { return Vec::new() };
    let Some(fd) = t.fields.iter().find(|f| f.id.canonical(db.endian) == field) else { return Vec::new() };
    (0..t.current_records as u32)
        .map(|r| read_field(&db, s.project().meta(), s.project().enums(), t, r, fd))
        .filter(|v| v.decoded == Some(code))
        .collect()
}

#[test]
fn nhl08_09_roster_cell_uses_the_player_page_vocabulary() {
    for (platform, game) in [("xbox360", "nhl08"), ("xbox360", "nhl09"), ("ps3", "nhl08"), ("ps3", "nhl09")] {
        let s = open(platform, game);
        for (code, label) in [(1, "Defensive"), (3, "Grinder"), (5, "Sniper"), (7, "Dangler")] {
            let roster = cells(&s, "eGlu", "QgFs", code);
            let companion = cells(&s, "dSvy", "mQoE", code);
            assert!(!roster.is_empty() && !companion.is_empty(), "{platform} {game}: code {code} present in both");
            for v in roster.iter().chain(&companion) {
                assert_eq!(v.enum_label.as_deref(), Some(label), "{platform} {game} {}.{} code {code}", v.studio_table, v.studio_field);
            }
            // The roster cell offers exactly the seven NHL 07 styles, none beyond the 3-bit field.
            let r = &roster[0];
            assert_eq!(r.enum_options.iter().map(|(c, _)| *c).collect::<Vec<_>>(), (1..=7).collect::<Vec<_>>());
            assert!(r.range_high.unwrap_or(0) <= 7, "{:?}", r.range_high);
            assert!(r.vocabulary_note.as_deref().unwrap_or("").contains("R122"), "{:?}", r.vocabulary_note);
        }
        println!("{platform} {game}: roster codes 1 / 3 / 5 / 7 = Defensive / Grinder / Sniper / Dangler on both the roster cell and the player page");
    }
}

#[test]
fn nhl07_and_nhl10_roster_cells_show_numbers() {
    // NHL 07: the roster cell agrees with p_style for only 73 % of skaters (R122) — meaning unresolved.
    let s = open("xbox360", "nhl07");
    let v = &cells(&s, "eGlu", "QgFs", 3)[0];
    assert_eq!((v.enum_label.as_deref(), v.enum_options.len()), (None, 0), "NHL 07 roster cell: a number");
    // NHL 10: nine styles; code 4 is used only by forwards there, the Legacy table called it a defence role.
    for platform in ["xbox360", "ps3"] {
        let s = open(platform, "nhl10");
        let four = cells(&s, "eGlu", "QgFs", 4);
        assert!(four.len() > 500, "{platform}: {} forwards with code 4", four.len());
        let v = &four[0];
        assert_eq!((v.enum_label.as_deref(), v.enum_options.len()), (None, 0), "{platform} NHL 10 roster cell: a number");
        let note = v.vocabulary_note.as_deref().unwrap_or("");
        assert!(note.contains("no catalogue for NHL 10") && note.contains("stored id is shown as a number"), "{note}");
    }
}

#[test]
fn nhl11_onward_keep_the_legacy_table() {
    for (platform, game) in [("xbox360", "nhl11"), ("xbox360", "nhl13"), ("ps3", "nhllegacy"), ("ps4", "nhl21")] {
        let s = open(platform, game);
        let v = &cells(&s, "eGlu", "QgFs", 9)[0];
        assert_eq!(v.enum_label.as_deref(), Some("Two-Way Forward"), "{platform} {game}");
        assert_eq!(v.enum_options.len(), 10);
    }
    // The companion style on NHL 14+ keeps its labels too.
    let s = open("ps4", "nhl21");
    assert_eq!(cells(&s, "dSvy", "QgFs", 9)[0].enum_label.as_deref(), Some("Two-Way Forward"));
}

/// Coordinator decision, 28 Sep (R122 S3, `docs/validation/research-20260928-receipt5-review.md`): the companion style on NHL
/// 11–13 keeps its numbers — continuity, the position split and the dictionary's label set do not establish the mapping there.
/// Receipt 64 locks the current behaviour; the roster cell's accepted labels and NHL 14+ are unchanged (tests above).
#[test]
fn nhl11_to_13_companion_style_stays_numeric() {
    for (platform, game) in [("xbox360", "nhl11"), ("xbox360", "nhl12"), ("xbox360", "nhl13"), ("ps3", "nhl11"), ("ps3", "nhl13")] {
        let s = open(platform, game);
        // The skater-AI companion carries `p_style` (`mQoE`) on these titles, not the roster table's `QgFs`.
        let v = cells(&s, "dSvy", "mQoE", 3);
        assert!(!v.is_empty(), "{platform} {game}: companion p_style code 3 exists");
        assert_eq!((v[0].enum_label.as_deref(), v[0].enum_options.len()), (None, 0), "{platform} {game}: companion style shown as a number");
    }
}
