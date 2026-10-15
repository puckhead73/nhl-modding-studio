//! Players 3.1 — title-scoped nationality labels (research C16 / R120b, 26 Sep 2026).
//!
//! The nationality code of a country moves between titles as countries are inserted into the list:
//! each national team's roster (title-owned evidence) shows the Czech Republic at 79 on NHL 08 / 09,
//! 81 on NHL 10 and 82 from NHL 11. Studio's nationality vocabulary was established on NHL Legacy,
//! so before this change it called an NHL 10 Czech player "Croatia" and an NHL 08 German player
//! "Czech Republic". These cases walk every anchor directory that has the international league (NHL
//! 07's gen6 schema has none; its own country table is checked separately), find each national
//! team's roster, take the roster's dominant code (as R120b does), check it against R120b's table,
//! and require that a roster player with that code shows the team's country — on the Player page
//! identity, the pending review of an edit to that code, and the import label lookup. Codes with no
//! national-team roster on a pre-NHL-13 title are shown as numbers (no mapping is invented).
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::{read_field, EditSession, FieldValue, FieldView, Platform, Project, ProjectRole};
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

/// R120b national-team code per title era: (abbreviation, NHL 08 / 09, NHL 10, NHL 11 / 12, NHL 13 →
/// PS4 21). `data/r120b-nationality-label-table.json` `matrix`.
const R120B: &[(&str, i64, i64, i64, i64)] = &[
    ("CAN", 0, 0, 0, 0),
    ("USA", 14, 14, 14, 14),
    ("DEN", 66, 66, 66, 66),
    ("FIN", 67, 67, 67, 67),
    ("NOR", 68, 68, 68, 68),
    ("SWE", 69, 69, 69, 69),
    ("BEL", 70, 70, 70, 70),
    ("KAZ", 72, 72, 72, 72),
    ("LAT", 73, 74, 74, 74),
    ("RUS", 75, 76, 76, 76),
    ("UKR", 76, 77, 77, 77),
    ("AUS", 77, 78, 78, 78),
    ("CZE", 79, 81, 82, 82),
    ("GBR", 80, 82, 83, 83),
    ("FRA", 81, 83, 84, 84),
    ("GER", 82, 84, 85, 85),
    ("ITA", 84, 86, 87, 87),
    ("POL", 86, 89, 91, 91),
    ("SLV", 88, 93, 95, 95),
    ("SWI", 89, 95, 97, 97),
    ("JPN", 96, 105, 108, 109),
];

/// The country a national team's abbreviation names, as Studio labels it: the operator-confirmed
/// Legacy wording for that team's code ("England" for the British code) on NHL 10 onward — the roster
/// fixes the code, not the exact label semantics (coordinator review of R119–R121) — and NHL 07's own
/// user-confirmed table ("Great Britain") on NHL 08 / 09, whose player page shows that table.
fn country(abbr: &str, legacy_wording: bool) -> &'static str {
    match abbr {
        "CAN" => "Canada",
        "USA" => "United States",
        "DEN" => "Denmark",
        "FIN" => "Finland",
        "NOR" => "Norway",
        "SWE" => "Sweden",
        "BEL" => "Belarus",
        "KAZ" => "Kazakhstan",
        "LAT" => "Latvia",
        "RUS" => "Russia",
        "UKR" => "Ukraine",
        "AUS" => "Austria",
        "CZE" => "Czech Republic",
        "GBR" if legacy_wording => "England",
        "GBR" => "Great Britain",
        "FRA" => "France",
        "GER" => "Germany",
        "ITA" => "Italy",
        "POL" => "Poland",
        "SLV" => "Slovakia",
        "SWI" => "Switzerland",
        "JPN" => "Japan",
        other => panic!("{other}"),
    }
}

fn era(game: &str) -> usize {
    match game {
        "nhl08" | "nhl09" => 0,
        "nhl10" => 1,
        "nhl11" | "nhl12" => 2,
        _ => 3,
    }
}

fn expected_code(abbr: &str, game: &str) -> i64 {
    let r = R120B.iter().find(|r| r.0 == abbr).unwrap();
    [r.1, r.2, r.3, r.4][era(game)]
}

fn nat<'a>(fields: &'a [FieldView]) -> &'a FieldView {
    fields.iter().find(|f| f.name == "intlcountry").expect("intlcountry")
}

/// Every anchor directory except NHL 07 (no international league), as `(platform, game)`.
fn titles() -> Vec<(String, String)> {
    let mut out = Vec::new();
    for platform in ["ps3", "ps4", "xbox360"] {
        let mut games: Vec<_> = std::fs::read_dir(repo(&format!("anchors/{platform}"))).unwrap().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
        games.sort();
        for g in games {
            let dbs = repo(&format!("anchors/{platform}/{g}/databases"));
            if g != "nhl07" && (dbs.join("nhlng.db").is_file() || dbs.join("nhl_game.db").is_file()) {
                out.push((platform.to_string(), g));
            }
        }
    }
    out
}

fn check_title(platform: &str, game: &str) -> Result<String, String> {
    let s = open(platform, game);
    let teams = s.team_summaries().unwrap();
    // The stored code of every player, read once (a full player view per roster member is slow).
    let db = TdbFile::parse(s.bytes()).unwrap();
    let bio = db.table_canonical("ubPc").unwrap();
    let fd = bio.fields.iter().find(|f| f.id.canonical(db.endian) == "Lelh").unwrap();
    let records: BTreeMap<u64, u32> = s.player_summaries().unwrap().into_iter().map(|p| (p.player_id, p.bio_record)).collect();
    let code_of = |pid: u64| records.get(&pid).map(|r| read_field(&db, s.project().meta(), s.project().enums(), bio, *r, fd).decoded.unwrap());
    let legacy_wording = era(game) >= 1;
    let mut errors = Vec::new();
    let mut seen = 0;
    for (abbr, ..) in R120B {
        // The national team: the team with this abbreviation whose roster is dominated by one code.
        let Some((team, members)) = teams.iter().filter(|t| t.abbrev == *abbr).find_map(|t| {
            let ids = s.team_roster_ids(t.team_id).ok()?;
            (ids.len() >= 15).then_some((t, ids))
        }) else {
            errors.push(format!("{abbr}: no national-team roster"));
            continue;
        };
        let mut counts: BTreeMap<i64, Vec<u64>> = BTreeMap::new();
        for pid in &members {
            if let Some(c) = code_of(*pid) {
                counts.entry(c).or_default().push(*pid);
            }
        }
        let (code, holders) = counts.iter().max_by_key(|(_, v)| v.len()).unwrap();
        let want = expected_code(abbr, game);
        if *code != want {
            errors.push(format!("{abbr} ({}): roster code {code} ≠ R120b {want}", team.name));
            continue;
        }
        let v = s.player_view(holders[0]).unwrap().unwrap();
        let f = nat(&v.identity);
        let label = country(abbr, legacy_wording);
        if f.display != label {
            errors.push(format!("{abbr}: raw {code} shown as {:?}, want {label:?}", f.display));
        }
        if !f.enum_options.iter().any(|(r, l)| *r == want && l == label) {
            errors.push(format!("{abbr}: dropdown lacks {want} = {label}"));
        }
        seen += 1;
    }
    if errors.is_empty() {
        Ok(format!("{seen} national teams labelled"))
    } else {
        Err(errors.join("; "))
    }
}

#[test]
fn every_national_team_shows_its_own_country_on_every_title() {
    let mut failed = Vec::new();
    for (platform, game) in titles() {
        match check_title(&platform, &game) {
            Ok(m) => eprintln!("{platform}/{game}: {m}"),
            Err(e) => {
                eprintln!("{platform}/{game}: FAIL {e}");
                failed.push(format!("{platform}/{game}"));
            }
        }
    }
    assert!(failed.is_empty(), "wrong nationality labels on {failed:?}");
}

/// The reproduction the review names: on NHL 10 a Czech national-team player (raw 81) was shown as
/// "Croatia". After the change the pending review of an edit also names the title's country, and
/// a code no national team carries on NHL 10 stays a number.
#[test]
fn nhl10_czech_player_edit_review_and_unmeasured_codes() {
    for platform in ["xbox360", "ps3"] {
        let mut s = open(platform, "nhl10");
        let cze = s.team_summaries().unwrap().into_iter().find(|t| t.abbrev == "CZE" && s.team_roster_ids(t.team_id).unwrap().len() >= 15).unwrap();
        let pid = s.team_roster_ids(cze.team_id).unwrap().into_iter().find(|p| nat(&s.player_view(*p).unwrap().unwrap().identity).decoded == Some(81)).expect("a Czech player with raw 81");
        let v = s.player_view(pid).unwrap().unwrap();
        let f = nat(&v.identity).clone();
        assert_eq!(f.display, "Czech Republic", "{platform}: raw 81 on NHL 10");
        let note = f.vocabulary_note.clone().unwrap_or_default();
        assert!(note.contains("NHL 10") && note.contains("national-team"), "{platform}: provenance note: {note}");
        // Edit to Germany (84 on NHL 10) and read the pending review text.
        s.set_field("ubPc", v.bio_record, &f.studio_field, FieldValue::Int(84)).unwrap();
        let p = s.pending_changes(20);
        let cell = p.cells.iter().find(|c| c.studio_field == f.studio_field).unwrap();
        assert_eq!((cell.before.display.as_str(), cell.after.display.as_str()), ("Czech Republic", "Germany"), "{platform}: pending");
        // 71 is Estonia on NHL Legacy; no national team carries it on NHL 10 — shown as a number.
        s.set_field("ubPc", v.bio_record, &f.studio_field, FieldValue::Int(71)).unwrap();
        let after = nat(&s.player_view(pid).unwrap().unwrap().identity).clone();
        assert_eq!((after.display.as_str(), after.enum_label.as_deref()), ("71", None), "{platform}: unmeasured code stays a number");
    }
}

/// The import resolves a corpus country through the registry's label lookup (`resolve_enum`). It
/// must land on the title's own code: before this change every title resolved NHL Legacy's code
/// (an imported Czech player would have been written 82 — Germany on NHL 07 / 08 / 09, Great Britain
/// on NHL 10).
#[test]
fn the_import_label_lookup_resolves_each_titles_own_code() {
    let cases: &[(&str, &str, &[(&str, Option<i64>)])] = &[
        ("xbox360", "nhl07", &[("Czech Republic", Some(79)), ("Germany", Some(82)), ("Belgium", None)]),
        ("xbox360", "nhl08", &[("Czech Republic", Some(79)), ("Germany", Some(82)), ("Lithuania", None)]),
        ("ps3", "nhl10", &[("Czech Republic", Some(81)), ("Japan", Some(105)), ("Estonia", None)]),
        ("ps3", "nhl12", &[("Czech Republic", Some(82)), ("Japan", Some(108)), ("Fiji", None)]),
        ("ps3", "nhl13", &[("Czech Republic", Some(82)), ("Japan", Some(109)), ("Estonia", Some(71))]),
        ("ps4", "nhl21", &[("Czech Republic", Some(82)), ("Japan", Some(109))]),
    ];
    for (platform, game, want) in cases {
        let s = open(platform, game);
        let def = s.project().enums().for_field("ubPc", "Lelh");
        for (label, code) in *want {
            assert_eq!(def.and_then(|d| d.raw_of(label)), *code, "{platform}/{game}: {label}");
        }
    }
}

/// Birth state ("Origin"): the country region (66 and up) mirrors the nationality numbering (R120 §7,
/// continuity evidence). Before NHL 13 it follows the title's national-team codes with a provenance
/// note; the North-American region keeps its labels; NHL Legacy's own table is unchanged.
#[test]
fn birth_state_country_codes_follow_the_title_numbering_before_nhl13() {
    let state = |s: &EditSession, pid: u64| {
        let v = s.player_view(pid).unwrap().unwrap();
        v.all_fields.iter().find(|f| f.studio_field == "QvxI").unwrap().clone()
    };
    for (platform, game, cases) in [
        ("ps3", "nhl10", vec![(81, "Czech Republic"), (71, "71"), (9, "Canada, Ontario"), (38, "USA, Minnesota")]),
        ("xbox360", "nhl11", vec![(108, "Japan"), (82, "Czech Republic"), (109, "109")]),
        ("ps3", "nhllegacy", vec![(94, "Serbia"), (82, "Czech Republic"), (109, "Japan")]),
    ] {
        let mut s = open(platform, game);
        let pid = s.player_summaries().unwrap().into_iter().find(|p| p.team_id.is_some()).unwrap();
        let fid = state(&s, pid.player_id).studio_field.clone();
        for (raw, want) in cases {
            s.set_field("ubPc", pid.bio_record, &fid, FieldValue::Int(raw)).unwrap();
            let f = state(&s, pid.player_id);
            assert_eq!(f.display, want, "{platform}/{game}: birth state {raw}");
            let note = f.vocabulary_note.unwrap_or_default();
            assert_eq!(note.contains("continuity"), game != "nhllegacy", "{platform}/{game}: note {note:?}");
        }
    }
}
