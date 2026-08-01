//! The server-driven **field profile** verified against the anchors — most intently NHL07 (gen6),
//! the title that motivated it. The profile must (a) never emit a cell for a field the loaded
//! schema lacks (no phantom "not in this game" cells), (b) surface the real gen6 fields that live
//! under a different id than the modern one, (c) route NHL07 team names to the editable stock/custom
//! row, and (d) flag values whose encoding isn't ground-truthed for the title. The same builders,
//! run across the title range, must produce only cells whose fields resolve — proving the design
//! generalizes rather than special-casing NHL07.

use std::path::{Path, PathBuf};

use studio_domain::{player_view, team_view, CellControl, EntityFieldProfile, ProfileCell};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::{PlayerKind, RosterModel};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn fixtures(db_rel: &str, meta_rel: &str) -> (Vec<u8>, FieldMetaIndex, EnumRegistry) {
    let bytes = std::fs::read(repo(db_rel)).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(meta_rel)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
    (bytes, meta, enums)
}

/// A flat lookup of a curated cell by its label (labels are unique within one profile).
fn cell<'a>(p: &'a EntityFieldProfile, label: &str) -> Option<&'a ProfileCell> {
    p.sections.iter().flat_map(|s| &s.cells).find(|c| c.label == label)
}

fn has(p: &EntityFieldProfile, label: &str) -> bool {
    cell(p, label).is_some()
}

/// Every field-backed cell must carry a resolved field — no dangling ids ever reach the UI.
fn assert_no_dangling(p: &EntityFieldProfile) {
    for s in &p.sections {
        assert!(!s.cells.is_empty(), "section '{}' emitted with no cells", s.title);
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
        "First Name", "Last Name", "Height", "Weight", "Birth Year", "Position", "Handedness",
        "Jersey Number", "Main Team", "Cap Hit", "Length Of Contract", "Two-Way",
        "NHL Rookie", "Draft Year", "Draft Position", "Archetype",
    ] {
        assert!(has(p, label), "NHL07 player profile should carry '{label}'");
    }

    // Absent — the modern ids NHL07's schema has no field for. No phantom cells.
    for label in [
        "Junior Team", "Pro Team", "Year UFA", "AHL Rookie", "CHL Rookie", "Draft Team",
        "Draft Round", "Active",
    ] {
        assert!(!has(p, label), "NHL07 lacks '{label}' — it must not emit a cell");
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
        let f = t.fields.iter().find(|f| f.id.canonical(db.endian) == "QbMR").unwrap();
        db.read_inline_str(t, bio, f)
            .map(|b| String::from_utf8_lossy(b).trim_end_matches('\0').to_string())
            .unwrap_or_default()
    };
    let giguere = model
        .players
        .iter()
        .find(|p| last(p.bio_record).eq_ignore_ascii_case("Giguere"))
        .expect("Giguère is in the NHL07 roster");
    let view = player_view(&db, &meta, &enums, &model, giguere.player_id).unwrap();

    let cap = cell(&view.profile, "Cap Hit").unwrap();
    assert!(cap.unverified.is_none(), "NHL07 salary scale is now confirmed → not flagged");
    assert_eq!(
        cap.derived_display.as_deref(),
        Some("$4,000,000"),
        "Giguère's contractdollars (160) × $25,000 = $4M"
    );

    // Draft Year renders through the field's friendly display (base 1959): Giguère raw 36 → 1995.
    let draft = cell(&view.profile, "Draft Year").unwrap();
    assert!(draft.unverified.is_none(), "NHL07 draft-year base is now confirmed → not flagged");
    assert_eq!(draft.field.as_ref().unwrap().display, "1995", "raw 36 + 1959 = 1995");
}

/// NHL07 team names live on the joined stock/custom row, not the thin `kOtt` index — the profile's
/// Full Name cell must target that row and be editable, so names are finally editable on gen6.
#[test]
fn nhl07_team_name_cell_targets_the_editable_stock_row() {
    let (bytes, meta, enums) = fixtures(NHL07_DB, NHL07_META);
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let team = model.teams.first().expect("NHL07 has teams");
    let view = team_view(&db, &meta, &enums, &model, team.team_id).unwrap();
    let p = &view.profile;

    let full = cell(p, "Full Name").expect("Full Name cell present");
    let field = full.field.as_ref().expect("Full Name is a real field");
    assert_eq!(
        field.studio_table, "HqzU",
        "NHL07 name resolves off the stockteamtable, not the thin kOtt row"
    );
    assert!(field.editable, "the resolved team name field is editable");
    assert!(has(p, "Abbreviation") && has(p, "Short Name"), "abbrev/short name also surface");

    // NHL07 has no team→arena link — the Venue section must not appear.
    assert!(!has(p, "Arena"), "NHL07 has no arena link → no Venue cell");
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
    let pos = bio.fields.iter().find(|f| f.id.canonical(db.endian) == "vjla").unwrap();
    let m = meta.field(&bio.id.raw(), &pos.id.raw()).unwrap();
    assert_eq!(m.range_high, Some(7), "NHL07 position is the 3-bit 0..7 code space");

    let goalie = model
        .players
        .iter()
        .find(|p| p.kind == PlayerKind::Goalie)
        .expect("NHL07 has goalies");
    let raw = db.read_u64(bio, goalie.bio_record, pos).unwrap();
    assert_eq!(raw, 4, "a NHL07 goalie's position code is 4 (goalie==4 holds on gen6)");
}

/// The profile generalizes: across the title range every anchor produces player and team profiles
/// whose cells all resolve — no dangling ids, no empty sections. Modern titles additionally carry a
/// confirmed cap-hit dollar figure (13-bit contractdollars), so their Cap Hit is NOT flagged.
#[test]
fn profile_generalizes_across_anchors() {
    let anchors = [
        ("anchors/ps3/nhllegacy/databases/nhlng.db", "anchors/ps3/nhllegacy/databases/nhlng-meta.xml", true),
        ("anchors/xbox360/nhl14/databases/nhlng.db", "anchors/xbox360/nhl14/databases/nhlng-meta.xml", true),
        ("anchors/xbox360/nhl10/databases/nhlng.db", "anchors/xbox360/nhl10/databases/nhlng-meta.xml", false),
        ("anchors/ps4/nhl21/databases/nhl_game.db", "anchors/ps4/nhl21/databases/nhl_game-meta.xml", true),
    ];
    for (db_rel, meta_rel, cap_confirmed) in anchors {
        let (bytes, meta, enums) = fixtures(db_rel, meta_rel);
        let db = TdbFile::parse(&bytes).unwrap();
        let model = RosterModel::build(&db).unwrap();

        let player = model.players.first().unwrap_or_else(|| panic!("{db_rel} has players"));
        let pv = player_view(&db, &meta, &enums, &model, player.player_id).unwrap();
        assert_no_dangling(&pv.profile);

        let team = model.teams.first().unwrap_or_else(|| panic!("{db_rel} has teams"));
        let tv = team_view(&db, &meta, &enums, &model, team.team_id).unwrap();
        assert_no_dangling(&tv.profile);

        // Modern 13-bit contractdollars: the ×$5000 scale is confirmed, so Cap Hit is a dollar
        // figure and unflagged (when the player carries a contract).
        if cap_confirmed {
            if let Some(cap) = cell(&pv.profile, "Cap Hit") {
                if let Some(disp) = &cap.derived_display {
                    assert!(disp.starts_with('$'), "{db_rel}: confirmed cap hit is a dollar figure");
                    assert!(cap.unverified.is_none(), "{db_rel}: confirmed cap hit is not flagged");
                }
            }
        }
    }
}
