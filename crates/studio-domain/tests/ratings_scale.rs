//! Era-correct rating scale, verified against the anchors. PS3/X360 store ratings 6-bit `0..63`
//! (display/edit `+36` → the 36..99 scale); every PS4 title stores them 7-bit `0..99` — the human
//! value directly, no offset. The classification is metadata-driven (`p_`/`g_`/`c_` name prefixes
//! + declared range), so NHL 10–13's `p_*`/`g_*` fields resolve too. This is the regression guard
//! for the PS4 `+36` bug (ids are identical across generations, so an id-keyed offset silently
//! corrupted every PS4 rating display and edit).

use std::path::{Path, PathBuf};

use studio_domain::{entity_fields, FieldView};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn fields(rel_db: &str, rel_meta: &str, table: &str) -> Vec<FieldView> {
    let bytes = std::fs::read(repo(rel_db)).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta =
        FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(rel_meta)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo("defs/enums")).unwrap();
    entity_fields(&db, &meta, &enums, table, 0)
}

fn by_name<'a>(fields: &'a [FieldView], name: &str) -> &'a FieldView {
    fields.iter().find(|f| f.name == name).unwrap_or_else(|| panic!("{name} present"))
}

/// Legacy (PS3): `c_speed` is 6-bit 0..63 storage — +36 display and edit offset.
#[test]
fn ps3_ratings_display_and_edit_plus_36() {
    let f = fields(
        "anchors/ps3/nhllegacy/databases/nhlng.db",
        "anchors/ps3/nhllegacy/databases/nhlng-meta.xml",
        "dSvy",
    );
    let speed = by_name(&f, "c_speed");
    assert_eq!(speed.edit_offset, Some(36));
    let decoded = speed.decoded.unwrap();
    assert_eq!(speed.display, (decoded + 36).to_string());
}

/// NHL 11 (PS3): the early-era `p_speed` (different id, same meaning) gets the same +36 scale via
/// the name-based classification.
#[test]
fn nhl11_p_ratings_get_the_same_scale() {
    let f = fields(
        "anchors/ps3/nhl11/databases/nhlng.db",
        "anchors/ps3/nhl11/databases/nhlng-meta.xml",
        "dSvy",
    );
    let speed = by_name(&f, "p_speed");
    assert_eq!(speed.edit_offset, Some(36));
    let decoded = speed.decoded.unwrap();
    assert_eq!(speed.display, (decoded + 36).to_string());

    // Fighting *frequency* (2-bit 0..3) must NOT classify as a rating.
    let fighting = by_name(&f, "p_fighting");
    assert_eq!(fighting.edit_offset, None);
}

/// PS4 NHL 21: same field id as PS3 (`c_speed`), but 7-bit 0..99 storage — the stored value IS the
/// rating. No offset on display or edit (the +36 here was a live bug before the metadata-driven
/// classification).
#[test]
fn ps4_ratings_are_stored_on_the_human_scale() {
    let f = fields(
        "anchors/ps4/nhl21/databases/nhl_game.db",
        "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
        "dSvy",
    );
    let speed = by_name(&f, "c_speed");
    assert_eq!(speed.edit_offset, None, "0..99 storage edits as-is");
    let decoded = speed.decoded.unwrap();
    assert_eq!(speed.display, decoded.to_string());
    assert!(
        (0..=99).contains(&decoded),
        "PS4 rating {decoded} should already sit on the human scale"
    );
}

/// PS4 NHL 15 goalie table: the LE first-year PS4 db follows the same 0..99 rule.
#[test]
fn ps4_nhl15_goalies_follow_the_same_rule() {
    let f = fields(
        "anchors/ps4/nhl15/databases/nhl_game.db",
        "anchors/ps4/nhl15/databases/nhl_game-meta.xml",
        "mHuy",
    );
    let glove_high = by_name(&f, "c_gsh");
    assert_eq!(glove_high.edit_offset, None);
    let decoded = glove_high.decoded.unwrap();
    assert_eq!(glove_high.display, decoded.to_string());
}
