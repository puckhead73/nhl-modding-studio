//! Birth month/day indexing across the title range.
//!
//! `ubPc.JKLp`/`Kswi` are **0-indexed on NHL 11→21** (read `+1`) but **already 1-indexed on
//! NHL 10** — on both its platforms. EA's metadata is byte-identical either way
//! (`rangelow="0" rangehigh="15"`), so the convention is only decidable from the stored values:
//! month `0` cannot occur 1-indexed, month `12` cannot occur 0-indexed.
//!
//! Getting this wrong is silent and data-corrupting: applying NHL 11+'s `+1` to NHL 10 reports
//! every player a day and a month late (Jordan Staal, born 1988-09-10, reads as 1988-10-11), and
//! the same offset drives the *edit* form — so an untouched date staged back would move.
//!
//! Verified 2026-07-17 against all 21 committed anchors.

use std::path::{Path, PathBuf};

use studio_domain::{player_view, probe_birthdate_encoding, read_field, BirthdateEncoding};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::RosterModel;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn probe(rel_db: &str) -> BirthdateEncoding {
    let bytes = std::fs::read(repo(rel_db)).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let bio = db.table_canonical("ubPc").unwrap();
    probe_birthdate_encoding(&db, bio)
}

/// NHL 10 is the series' only 1-indexed title — and it is so on both platforms, which is why the
/// rule can't be "PS3 vs Xbox".
#[test]
fn nhl10_probes_one_indexed_on_both_platforms() {
    assert_eq!(
        probe("anchors/ps3/nhl10/databases/nhlng.db"),
        BirthdateEncoding::OneIndexed,
    );
    assert_eq!(
        probe("anchors/xbox360/nhl10/databases/nhlng.db"),
        BirthdateEncoding::OneIndexed,
    );
}

/// Every other title, every platform — including little-endian PS4 — is 0-indexed.
#[test]
fn nhl11_through_21_probe_zero_indexed() {
    let anchors = [
        "anchors/ps3/nhl11/databases/nhlng.db",
        "anchors/ps3/nhl12/databases/nhlng.db",
        "anchors/ps3/nhl13/databases/nhlng.db",
        "anchors/ps3/nhl14/databases/nhlng.db",
        "anchors/ps3/nhl15/databases/nhlng.db",
        "anchors/ps3/nhllegacy/databases/nhlng.db",
        "anchors/xbox360/nhl11/databases/nhlng.db",
        "anchors/xbox360/nhl12/databases/nhlng.db",
        "anchors/xbox360/nhl13/databases/nhlng.db",
        "anchors/xbox360/nhl14/databases/nhlng.db",
        "anchors/xbox360/nhl15/databases/nhlng.db",
        "anchors/xbox360/nhllegacy/databases/nhlng.db",
        "anchors/ps4/nhl15/databases/nhl_game.db",
        "anchors/ps4/nhl16/databases/nhl_game.db",
        "anchors/ps4/nhl17/databases/nhl_game.db",
        "anchors/ps4/nhl18/databases/nhl_game.db",
        "anchors/ps4/nhl19/databases/nhl_game.db",
        "anchors/ps4/nhl20/databases/nhl_game.db",
        "anchors/ps4/nhl21/databases/nhl_game.db",
    ];
    for a in anchors {
        assert_eq!(probe(a), BirthdateEncoding::ZeroIndexed, "{a}");
    }
}

/// The offsets the display and the edit form both consume.
#[test]
fn offsets_match_the_conventions() {
    assert_eq!(BirthdateEncoding::ZeroIndexed.offset(), 1);
    assert_eq!(BirthdateEncoding::OneIndexed.offset(), 0);
}

/// Find a player by name and return their displayed `(month, day)` plus the edit offsets.
fn shown_birthdate(rel_db: &str, rel_meta: &str, first: &str, last: &str) -> (String, String, i64) {
    let bytes = std::fs::read(repo(rel_db)).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(rel_meta)).unwrap()).unwrap();
    let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
    let bio = db.table_canonical("ubPc").unwrap();

    let by_id = |id: &str| {
        bio.fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == id)
            .unwrap()
    };
    let (f_first, f_last) = (by_id("HdeP"), by_id("QbMR"));
    let text = |rec: u32, f| {
        db.read_inline_str(bio, rec, f)
            .map(|b| String::from_utf8_lossy(b).trim().to_string())
            .unwrap_or_default()
    };

    let rec = (0..bio.current_records as u32)
        .find(|&i| text(i, f_first) == first && text(i, f_last) == last)
        .unwrap_or_else(|| panic!("{first} {last} not found in {rel_db}"));

    let month = read_field(&db, &meta, &enums, bio, rec, by_id("JKLp"));
    let day = read_field(&db, &meta, &enums, bio, rec, by_id("Kswi"));
    (month.display, day.display, month.edit_offset.unwrap())
}

/// The regression this all exists for: the *same real player* must show the *same real birthday*
/// in NHL 10 and in Legacy, despite the two titles storing it differently.
///
/// Jordan Staal was born 1988-09-10.
#[test]
fn same_player_shows_the_same_real_birthday_across_titles() {
    let (m, d, off) = shown_birthdate(
        "anchors/xbox360/nhl10/databases/nhlng.db",
        "anchors/xbox360/nhl10/databases/nhlng-meta.xml",
        "Jordan",
        "Staal",
    );
    assert_eq!((m.as_str(), d.as_str()), ("9", "10"), "NHL 10 (1-indexed)");
    // The edit form must agree with the display, or staging an untouched value would move the date.
    assert_eq!(off, 0, "NHL 10 edit offset");

    let (m, d, off) = shown_birthdate(
        "anchors/ps3/nhllegacy/databases/nhlng.db",
        "anchors/ps3/nhllegacy/databases/nhlng-meta.xml",
        "Jordan",
        "Staal",
    );
    assert_eq!((m.as_str(), d.as_str()), ("9", "10"), "Legacy (0-indexed)");
    assert_eq!(off, 1, "Legacy edit offset");
}

/// The Player banner composes its own date string (a third path, separate from the field display
/// and the edit form), so it needs its own guard.
#[test]
fn the_player_banner_composes_the_same_real_birthday_across_titles() {
    let banner = |rel_db: &str, rel_meta: &str| -> String {
        let bytes = std::fs::read(repo(rel_db)).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta =
            FieldMetaIndex::from_xml(&std::fs::read_to_string(repo(rel_meta)).unwrap()).unwrap();
        let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
        let model = RosterModel::build(&db).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let by_id = |id: &str| {
            bio.fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == id)
                .unwrap()
        };
        let text = |rec: u32, f| {
            db.read_inline_str(bio, rec, f)
                .map(|b| String::from_utf8_lossy(b).trim().to_string())
                .unwrap_or_default()
        };
        let (f_first, f_last) = (by_id("HdeP"), by_id("QbMR"));
        let rec = (0..bio.current_records as u32)
            .find(|&i| text(i, f_first) == "Jordan" && text(i, f_last) == "Staal")
            .unwrap();
        let wbiz = db.read_u64(bio, rec, by_id("wBIz")).unwrap();
        player_view(&db, &meta, &enums, &model, wbiz)
            .unwrap()
            .header
            .birthdate
            .unwrap()
    };

    assert_eq!(
        banner(
            "anchors/xbox360/nhl10/databases/nhlng.db",
            "anchors/xbox360/nhl10/databases/nhlng-meta.xml",
        ),
        "1988-09-10",
        "NHL 10 banner",
    );
    assert_eq!(
        banner(
            "anchors/ps3/nhllegacy/databases/nhlng.db",
            "anchors/ps3/nhllegacy/databases/nhlng-meta.xml",
        ),
        "1988-09-10",
        "Legacy banner",
    );
}
