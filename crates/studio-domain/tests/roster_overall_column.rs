//! The roster list's **Overall column** obeys the same era / coverage rule as the Attributes tab
//! (Players 3.2, 25 Sep 2026).
//!
//! `player_summaries` used to run the skater regression on *every* title. The regression is an
//! NHL 14 / 15-era fit (`tests/overall_formula.rs`): on the pre-NHL 14 `p_` / `g_` vocabulary it
//! reads a flat ~13 low (NHL 13) or has too few inputs to mean anything (NHL 10–12), while the
//! Attributes tab already suppresses the number there and falls back to the stored `overall`. The
//! roster list, the lines page and the contracts tab therefore showed a confidently wrong OVR next
//! to a player whose own tab showed the stored one. Now: skaters take the derived Overall only on
//! the calibrated `c_*` era with enough inputs resolved, otherwise the stored `overall` rating on
//! the display scale, otherwise nothing; goalies keep the stored rating as before.
use std::path::{Path, PathBuf};

use studio_domain::{build_player_attributes, entity_fields, player_summaries};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::{PlayerKind, RosterModel};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn load(rel_dir: &str, db_name: &str) -> (Vec<u8>, FieldMetaIndex, EnumRegistry) {
    let dir = repo(rel_dir).join("databases");
    let bytes = std::fs::read(dir.join(db_name)).unwrap();
    let meta = FieldMetaIndex::from_xml(
        &std::fs::read_to_string(dir.join(format!("{}-meta.xml", db_name.trim_end_matches(".db"))))
            .unwrap(),
    )
    .unwrap();
    let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
    (bytes, meta, enums)
}

/// For every skater with a companion row: the list's Overall against the Attributes tab's
/// `computed_overall` (calibrated era) or `stored_overall` (everything else).
fn check(rel_dir: &str, db_name: &str, expect_calibrated: bool) -> (usize, usize) {
    let (bytes, meta, enums) = load(rel_dir, db_name);
    let db = TdbFile::parse(&bytes).unwrap();
    let model = RosterModel::build(&db).unwrap();
    let summaries = player_summaries(&db, &meta, &enums, &model);
    let (mut checked, mut with_value) = (0, 0);
    for p in model
        .players
        .iter()
        .filter(|p| p.kind == PlayerKind::Skater)
    {
        let Some(rec) = p.skater_ai else { continue };
        let s = summaries
            .iter()
            .find(|s| s.player_id == p.player_id)
            .unwrap();
        let fields = entity_fields(&db, &meta, &enums, "dSvy", rec);
        let a = build_player_attributes(
            &fields,
            "dSvy",
            p.position == tdb_relations::Position::Defense,
        );
        let expected = if expect_calibrated {
            assert!(
                a.computed_overall.is_some(),
                "{rel_dir}: the calibrated era computes an Overall (player {})",
                p.player_id
            );
            a.computed_overall
        } else {
            assert!(a.computed_overall.is_none(), "{rel_dir}: the Attributes tab suppresses the derived Overall outside the calibrated era");
            a.stored_overall
        };
        assert_eq!(
            s.overall, expected,
            "{rel_dir}: roster list Overall for player {} ({} {})",
            p.player_id, s.first_name, s.last_name
        );
        checked += 1;
        with_value += s.overall.is_some() as usize;
    }
    assert!(checked > 100, "{rel_dir}: {checked} skaters compared");
    (checked, with_value)
}

/// NHL 13 (`p_` vocabulary, carries `p_overall`): the list shows the **stored** Overall, not the
/// ~13-low regression.
#[test]
fn nhl13_roster_list_shows_the_stored_overall_not_the_uncalibrated_formula() {
    let (checked, with_value) = check("anchors/ps3/nhl13", "nhlng.db", false);
    assert_eq!(
        checked, with_value,
        "NHL 13 carries a stored overall for every skater companion"
    );
    check("anchors/xbox360/nhl13", "nhlng.db", false);
}

/// NHL 10 / 11 / 12 (`p_` vocabulary, **no** overall field): the list shows nothing rather than a
/// formula over missing inputs.
#[test]
fn nhl10_to_12_roster_list_shows_no_overall() {
    for dir in [
        "anchors/ps3/nhl10",
        "anchors/xbox360/nhl11",
        "anchors/ps3/nhl12",
    ] {
        let (_, with_value) = check(dir, "nhlng.db", false);
        assert_eq!(with_value, 0, "{dir}: no stored overall field, so no value");
    }
}

/// NHL 15 PS3 and NHL 17 PS4 (`c_*` era): the list keeps the derived Overall, equal to the tab's.
#[test]
fn calibrated_era_roster_list_keeps_the_derived_overall() {
    let (checked, with_value) = check("anchors/ps3/nhl15", "nhlng.db", true);
    assert_eq!(checked, with_value);
    let (checked, with_value) = check("anchors/ps4/nhl17", "nhl_game.db", true);
    assert_eq!(checked, with_value);
}

/// NHL 07 (gen6 `p_` vocabulary, no overall field) and NHL 09 likewise show nothing.
#[test]
fn early_titles_roster_list_shows_no_overall() {
    let (_, with_value) = check("anchors/xbox360/nhl07", "NHL07NG.db", false);
    assert_eq!(with_value, 0, "NHL 07: no overall field");
    let (_, with_value) = check("anchors/xbox360/nhl09", "nhlng.db", false);
    assert_eq!(with_value, 0, "NHL 09: no overall field");
}
