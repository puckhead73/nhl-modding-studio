//! The server-driven **field profile** verified against the anchors — most intently NHL07 (gen6),
//! the title that motivated it. The profile must (a) never emit a cell for a field the loaded
//! schema lacks (no phantom "not in this game" cells), (b) surface the real gen6 fields that live
//! under a different id than the modern one, (c) route NHL07 team names to the editable stock/custom
//! row, and (d) flag values whose encoding isn't ground-truthed for the title. The same builders,
//! run across the title range, must produce only cells whose fields resolve — proving the design
//! generalizes rather than special-casing NHL07.

use std::path::{Path, PathBuf};

use studio_domain::{player_view, team_view, CellControl, EntityFieldProfile, ProfileCell};
use studio_domain::{LeagueCatalog, LeagueVocabularies};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::{PlayerKind, RosterModel};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}


/// The league catalog for a test database, resolved with the game id the fixture belongs to.
fn leagues_for(db: &TdbFile, meta: &FieldMetaIndex, game_id: Option<&str>) -> LeagueCatalog {
    let vocabs = LeagueVocabularies::from_dir(repo("defs").join("leagues")).unwrap();
    LeagueCatalog::build(db, meta, &vocabs, game_id)
}

fn fixtures(db_rel: &str, meta_rel: &str) -> (Vec<u8>, FieldMetaIndex, EnumRegistry) {
    let bytes = std::fs::read(repo(db_rel)).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(meta_rel)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
    (bytes, meta, enums)
}

/// A flat lookup of a curated cell by its label (labels are unique within one profile).
fn cell<'a>(p: &'a EntityFieldProfile, label: &str) -> Option<&'a ProfileCell> {
    p.sections
        .iter()
        .flat_map(|s| &s.cells)
        .find(|c| c.label == label)
}

fn has(p: &EntityFieldProfile, label: &str) -> bool {
    cell(p, label).is_some()
}

/// Every field-backed cell must carry a resolved field — no dangling ids ever reach the UI.
fn assert_no_dangling(p: &EntityFieldProfile) {
    for s in &p.sections {
        assert!(
            !s.cells.is_empty(),
            "section '{}' emitted with no cells",
            s.title
        );
        for c in &s.cells {
            match c.control {
                CellControl::Derived => {}
                _ => assert!(
                    c.field.is_some(),
                    "cell '{}' has a non-derived control but no resolved field",
                    c.label
                ),
            }
        }
    }
}

/// Prove that every field-backed profile cell is a real, addressable cell in this database.
fn assert_profile_targets_exist(db: &TdbFile, p: &EntityFieldProfile, anchor: &str) {
    assert_no_dangling(p);
    for cell in p.sections.iter().flat_map(|s| &s.cells) {
        // C8: a colour cell's components are addressable like its primary field.
        let fields = cell.field.iter().chain(cell.color_components.iter());
        for field in fields {
        let table = db.table_canonical(&field.studio_table).unwrap_or_else(|| {
            panic!(
                "{anchor}: '{}' targets missing table {}",
                cell.label, field.studio_table
            )
        });
        assert!(
            cell.record < table.current_records as u32,
            "{anchor}: '{}' targets record {} beyond {}.{}",
            cell.label,
            cell.record,
            field.studio_table,
            table.current_records
        );
        assert!(
            table
                .fields
                .iter()
                .any(|f| f.id.canonical(db.endian) == field.studio_field),
            "{anchor}: '{}' targets missing field {}.{}",
            cell.label,
            field.studio_table,
            field.studio_field
        );
        }
    }
}

const NHL07_DB: &str = "anchors/xbox360/nhl07/databases/NHL07NG.db";
const NHL07_META: &str = "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml";

/// NHL07 player profile: exactly the gen6 surface. The modern draft/rookie/contract-rights cells the
/// schema lacks are absent (no phantoms); the real gen6 fields (single `rookie`, `yeardrafted`,
/// `positiondrafted`) surface under their friendly labels via the name synonyms.
#[test]
fn nhl07_player_profile_has_no_phantoms_and_surfaces_gen6_fields() {
    let (bytes, meta, enums) = fixtures(NHL07_DB, NHL07_META);
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let skater = model
        .players
        .iter()
        .find(|p| p.kind == PlayerKind::Skater)
        .expect("NHL07 has skaters");
    let view = player_view(&db, &meta, &enums, &model, skater.player_id).unwrap();
    let p = &view.profile;

    // Present — every bio field NHL07 carries, incl. the gen6-only draft fields via name synonyms,
    // and the gen6 player type (Archetype, from the 3-bit `p_style`).
    for label in [
        "First Name",
        "Last Name",
        "Height",
        "Weight",
        "Birth Year",
        "Position",
        "Handedness",
        "Jersey Number",
        "Main Team",
        "Cap Hit",
        "Length Of Contract",
        "Two-Way",
        "NHL Rookie",
        "Draft Year",
        "Draft Position",
        "Archetype",
    ] {
        assert!(has(p, label), "NHL07 player profile should carry '{label}'");
    }

    // Absent — the modern ids NHL07's schema has no field for. No phantom cells.
    for label in [
        "Junior Team",
        "Pro Team",
        "Year UFA",
        "AHL Rookie",
        "CHL Rookie",
        "Draft Team",
        "Draft Round",
        "Active",
    ] {
        assert!(
            !has(p, label),
            "NHL07 lacks '{label}' — it must not emit a cell"
        );
    }

    assert_no_dangling(p);
}

/// NHL07 contract/draft encodings are now ground-truthed (salary ×$25,000, draft base 1959), so the
/// cells show the real dollar figure / calendar year and are NOT flagged. Verified against Giguère
/// ($4M, drafted 1995).
#[test]
fn nhl07_contract_and_draft_encodings_are_resolved() {
    let (bytes, meta, enums) = fixtures(NHL07_DB, NHL07_META);
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let last = |bio: u32| {
        let t = db.table_canonical("ubPc").unwrap();
        let f = t
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "QbMR")
            .unwrap();
        db.read_inline_str(t, bio, f)
            .map(|b| {
                String::from_utf8_lossy(b)
                    .trim_end_matches('\0')
                    .to_string()
            })
            .unwrap_or_default()
    };
    let giguere = model
        .players
        .iter()
        .find(|p| last(p.bio_record).eq_ignore_ascii_case("Giguere"))
        .expect("Giguère is in the NHL07 roster");
    let view = player_view(&db, &meta, &enums, &model, giguere.player_id).unwrap();

    let cap = cell(&view.profile, "Cap Hit").unwrap();
    assert!(
        cap.unverified.is_none(),
        "NHL07 salary scale is now confirmed → not flagged"
    );
    assert_eq!(
        cap.derived_display.as_deref(),
        Some("$4,000,000"),
        "Giguère's contractdollars (160) × $25,000 = $4M"
    );

    // Draft Year renders through the field's friendly display (base 1959): Giguère raw 36 → 1995.
    let draft = cell(&view.profile, "Draft Year").unwrap();
    assert!(
        draft.unverified.is_none(),
        "NHL07 draft-year base is now confirmed → not flagged"
    );
    assert_eq!(
        draft.field.as_ref().unwrap().display,
        "1995",
        "raw 36 + 1959 = 1995"
    );
}

/// NHL07 team names live on the joined stock/custom row, not the thin `kOtt` index — the profile's
/// Full Name cell must target that row and be editable, so names are finally editable on gen6.
#[test]
fn nhl07_team_name_cell_targets_the_editable_stock_row() {
    let (bytes, meta, enums) = fixtures(NHL07_DB, NHL07_META);
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let team = model.teams.first().expect("NHL07 has teams");
    let view = team_view(&db, &meta, &enums, &model, &leagues_for(&db, &meta, None), team.team_id).unwrap();
    let p = &view.profile;

    let full = cell(p, "Full Name").expect("Full Name cell present");
    let field = full.field.as_ref().expect("Full Name is a real field");
    assert_eq!(
        field.studio_table, "HqzU",
        "NHL07 name resolves off the stockteamtable, not the thin kOtt row"
    );
    assert!(field.editable, "the resolved team name field is editable");
    assert!(
        has(p, "Abbreviation") && has(p, "Short Name"),
        "abbrev/short name also surface"
    );

    // NHL07 routes through exhibitionarena rather than a kOtt arena field.
    let venue = cell(p, "Home Arena").expect("separate arena selector is editable");
    assert_eq!(venue.field.as_ref().unwrap().studio_table, "StEO");
    assert!(!venue.field.as_ref().unwrap().enum_options.is_empty());
    assert_no_dangling(p);
}

/// NHL07's `position` is a 3-bit 0..7 code space, wider than the confirmed 0..4; verify the
/// `goalie == 4` split the entity model relies on actually holds on this anchor (a real goalie reads
/// position code 4), so the skater/goalie companion routing is sound on gen6.
#[test]
fn nhl07_goalie_position_code_is_four() {
    let (bytes, meta, _enums) = fixtures(NHL07_DB, NHL07_META);
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let bio = db.table_canonical("ubPc").unwrap();
    let pos = bio
        .fields
        .iter()
        .find(|f| f.id.canonical(db.endian) == "vjla")
        .unwrap();
    let m = meta.field(&bio.id.raw(), &pos.id.raw()).unwrap();
    assert_eq!(
        m.range_high,
        Some(7),
        "NHL07 position is the 3-bit 0..7 code space"
    );

    let goalie = model
        .players
        .iter()
        .find(|p| p.kind == PlayerKind::Goalie)
        .expect("NHL07 has goalies");
    let raw = db.read_u64(bio, goalie.bio_record, pos).unwrap();
    assert_eq!(
        raw, 4,
        "a NHL07 goalie's position code is 4 (goalie==4 holds on gen6)"
    );
}

/// The profile generalizes: across the title range every anchor produces player and team profiles
/// whose cells all resolve — no dangling ids, no empty sections. Modern titles additionally carry a
/// confirmed cap-hit dollar figure (13-bit contractdollars), so their Cap Hit is NOT flagged.
#[test]
fn profile_generalizes_across_anchors() {
    let anchors = [
        (
            "anchors/ps3/nhllegacy/databases/nhlng.db",
            "anchors/ps3/nhllegacy/databases/nhlng-meta.xml",
            true,
        ),
        (
            "anchors/xbox360/nhl14/databases/nhlng.db",
            "anchors/xbox360/nhl14/databases/nhlng-meta.xml",
            true,
        ),
        (
            "anchors/xbox360/nhl10/databases/nhlng.db",
            "anchors/xbox360/nhl10/databases/nhlng-meta.xml",
            false,
        ),
        (
            "anchors/ps4/nhl21/databases/nhl_game.db",
            "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
            true,
        ),
    ];
    for (db_rel, meta_rel, cap_confirmed) in anchors {
        let (bytes, meta, enums) = fixtures(db_rel, meta_rel);
        let db = TdbFile::parse(&bytes).unwrap();
        let model = RosterModel::build(&db).unwrap();

        let player = model
            .players
            .first()
            .unwrap_or_else(|| panic!("{db_rel} has players"));
        let pv = player_view(&db, &meta, &enums, &model, player.player_id).unwrap();
        assert_no_dangling(&pv.profile);

        let team = model
            .teams
            .first()
            .unwrap_or_else(|| panic!("{db_rel} has teams"));
        let tv = team_view(&db, &meta, &enums, &model, &leagues_for(&db, &meta, None), team.team_id).unwrap();
        assert_no_dangling(&tv.profile);

        // Modern 13-bit contractdollars: the ×$5000 scale is confirmed, so Cap Hit is a dollar
        // figure and unflagged (when the player carries a contract).
        if cap_confirmed {
            if let Some(cap) = cell(&pv.profile, "Cap Hit") {
                if let Some(disp) = &cap.derived_display {
                    assert!(
                        disp.starts_with('$'),
                        "{db_rel}: confirmed cap hit is a dollar figure"
                    );
                    assert!(
                        cap.unverified.is_none(),
                        "{db_rel}: confirmed cap hit is not flagged"
                    );
                }
            }
        }
    }
}

/// Matrix gate: auto-discover every committed main DB. Adding an anchor expands this test without a
/// hand-maintained title list; the count assertion also makes accidental anchor removal visible.
#[test]
fn every_anchor_exposes_only_addressable_profile_fields() {
    let root = repo("anchors");
    let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
    let mut checked = Vec::new();
    for platform in std::fs::read_dir(&root)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
    {
        for game in std::fs::read_dir(platform.path())
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_dir())
        {
            let databases = game.path().join("databases");
            let pair = [
                ("NHL07NG.db", "NHL07NG-meta.xml"),
                ("nhlng.db", "nhlng-meta.xml"),
                ("nhl_game.db", "nhl_game-meta.xml"),
            ]
            .into_iter()
            .find(|(db, meta)| databases.join(db).is_file() && databases.join(meta).is_file());
            let Some((db_name, meta_name)) = pair else {
                continue;
            };
            let label = format!(
                "{}/{}",
                platform.file_name().to_string_lossy(),
                game.file_name().to_string_lossy()
            );
            let bytes = std::fs::read(databases.join(db_name)).unwrap();
            let db = TdbFile::parse(&bytes).unwrap_or_else(|e| panic!("{label}: {e}"));
            let meta = FieldMetaIndex::from_xml(
                &std::fs::read_to_string(databases.join(meta_name)).unwrap(),
            )
            .unwrap_or_else(|e| panic!("{label}: {e}"));
            let model = RosterModel::build(&db).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert!(!model.players.is_empty(), "{label}: no players");
            assert!(!model.teams.is_empty(), "{label}: no teams");
            for kind in [PlayerKind::Skater, PlayerKind::Goalie] {
                if let Some(player) = model.players.iter().find(|p| p.kind == kind) {
                    let view = player_view(&db, &meta, &enums, &model, player.player_id)
                        .unwrap_or_else(|| panic!("{label}: player view failed"));
                    assert_profile_targets_exist(&db, &view.profile, &label);
                }
            }
            let team = &model.teams[0];
            let view = team_view(&db, &meta, &enums, &model, &leagues_for(&db, &meta, None), team.team_id)
                .unwrap_or_else(|| panic!("{label}: team view failed"));
            assert_profile_targets_exist(&db, &view.profile, &label);
            checked.push(label);
        }
    }
    assert_eq!(checked.len(), 27, "anchor inventory changed: {checked:#?}");
}

/// C8 (Phase 2.1 identity, 20 Sep 2026): the team colour cells and the split name cells, measured
/// per title on the anchors. Shapes: NHL 08-10 carry no colour on the team row; NHL 11-15 / Legacy
/// (both consoles) and PS4 15 carry two `_r/_g/_b` triples; PS4 16 six triples (front-end,
/// presentation, overlay primary + secondary); PS4 17-21 nine packed words (those plus the three
/// tertiaries). Byte order of the
/// packed word `(r << 16) | (g << 8) | b` is proven by the same team's value on the two shapes.
#[test]
fn team_colour_and_name_cells_match_the_measured_per_title_shapes() {
    // (anchor, expected (label, component count) list, expected identity labels present)
    let expect: &[(&str, &str, &[(&str, usize)], &[&str])] = &[
        ("anchors/xbox360/nhl10/databases/nhlng.db", "anchors/xbox360/nhl10/databases/nhlng-meta.xml", &[], &[]),
        ("anchors/xbox360/nhl11/databases/nhlng.db", "anchors/xbox360/nhl11/databases/nhlng-meta.xml", &[("Primary", 3), ("Secondary", 3)], &[]),
        ("anchors/xbox360/nhl14/databases/nhlng.db", "anchors/xbox360/nhl14/databases/nhlng-meta.xml", &[("Primary", 3), ("Secondary", 3)], &[]),
        ("anchors/xbox360/nhllegacy/databases/nhlng.db", "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml", &[("Primary", 3), ("Secondary", 3)], &[]),
        ("anchors/ps3/nhllegacy/databases/nhlng.db", "anchors/ps3/nhllegacy/databases/nhlng-meta.xml", &[("Primary", 3), ("Secondary", 3)], &[]),
        ("anchors/ps4/nhl15/databases/nhl_game.db", "anchors/ps4/nhl15/databases/nhl_game-meta.xml", &[("Primary", 3), ("Secondary", 3)], &[]),
        ("anchors/ps4/nhl16/databases/nhl_game.db", "anchors/ps4/nhl16/databases/nhl_game-meta.xml",
            &[("Front-end primary", 3), ("Front-end secondary", 3), ("Presentation primary", 3), ("Presentation secondary", 3), ("Overlay primary", 3), ("Overlay secondary", 3)],
            &["Short Name", "Nickname"]),
        ("anchors/ps4/nhl17/databases/nhl_game.db", "anchors/ps4/nhl_game-meta.xml",
            &[("Front-end primary", 1), ("Front-end secondary", 1), ("Front-end tertiary", 1), ("Presentation primary", 1), ("Presentation secondary", 1), ("Presentation tertiary", 1), ("Overlay primary", 1), ("Overlay secondary", 1), ("Overlay tertiary", 1)],
            &["City", "Team Name", "Nickname"]),
        ("anchors/ps4/nhl21/databases/nhl_game.db", "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
            &[("Front-end primary", 1), ("Front-end secondary", 1), ("Front-end tertiary", 1), ("Presentation primary", 1), ("Presentation secondary", 1), ("Presentation tertiary", 1), ("Overlay primary", 1), ("Overlay secondary", 1), ("Overlay tertiary", 1)],
            &["City", "Team Name", "Nickname"]),
    ];
    for (db_rel, meta_rel, colours, names) in expect {
        let meta_rel = if meta_rel.ends_with("ps4/nhl_game-meta.xml") { "anchors/ps4/nhl17/databases/nhl_game-meta.xml" } else { meta_rel };
        let (bytes, meta, enums) = fixtures(db_rel, meta_rel);
        let db = TdbFile::parse(&bytes).unwrap();
        let model = RosterModel::build(&db).unwrap();
        let team = &model.teams[0];
        let tv = team_view(&db, &meta, &enums, &model, &leagues_for(&db, &meta, None), team.team_id).unwrap();
        let got: Vec<(String, usize)> = tv
            .profile
            .sections
            .iter()
            .filter(|s| s.title == "Colours")
            .flat_map(|s| &s.cells)
            .map(|c| {
                assert!(matches!(c.control, CellControl::Color), "{db_rel}: '{}' in Colours is not a colour cell", c.label);
                (c.label.clone(), c.color_components.len())
            })
            .collect();
        let want: Vec<(String, usize)> = colours.iter().map(|(l, n)| (l.to_string(), *n)).collect();
        assert_eq!(got, want, "{db_rel}: colour cells");
        for label in *names {
            assert!(has(&tv.profile, label), "{db_rel}: identity cell '{label}' missing");
        }
        assert_profile_targets_exist(&db, &tv.profile, db_rel);
    }
}

/// The packed word's byte order: on the anchors the same team's front-end primary reads as the
/// `_r/_g/_b` triple on PS4 16 and as one word on PS4 17; `(r << 16) | (g << 8) | b` reproduces it.
#[test]
fn packed_colour_word_is_r_g_b_big_endian_order() {
    let load = |db_rel: &str, meta_rel: &str| {
        let (bytes, meta, enums) = fixtures(db_rel, meta_rel);
        let db = TdbFile::parse(&bytes).unwrap();
        let model = RosterModel::build(&db).unwrap();
        let mut out = std::collections::BTreeMap::new();
        for team in &model.teams {
            let tv = team_view(&db, &meta, &enums, &model, &leagues_for(&db, &meta, None), team.team_id).unwrap();
            let abbr = cell(&tv.profile, "Abbreviation").and_then(|c| c.field.as_ref()).map(|f| f.display.trim().to_string()).unwrap_or_default();
            let Some(c) = cell(&tv.profile, "Front-end primary") else { continue };
            let comps: Vec<i64> = c.color_components.iter().map(|f| f.decoded.unwrap_or(0)).collect();
            let packed = if comps.len() == 3 { (comps[0] << 16) | (comps[1] << 8) | comps[2] } else { comps[0] };
            out.insert(abbr, packed);
        }
        out
    };
    let a = load("anchors/ps4/nhl16/databases/nhl_game.db", "anchors/ps4/nhl16/databases/nhl_game-meta.xml");
    let b = load("anchors/ps4/nhl17/databases/nhl_game.db", "anchors/ps4/nhl17/databases/nhl_game-meta.xml");
    let mut same = 0;
    let mut differ = Vec::new();
    for (abbr, v) in &a {
        if let Some(w) = b.get(abbr) {
            if v == w { same += 1 } else { differ.push((abbr.clone(), *v, *w)) }
        }
    }
    assert!(same >= 20, "at least 20 NHL teams share their front-end primary across NHL 16/17 (same {same}, differ {differ:?})");
}
