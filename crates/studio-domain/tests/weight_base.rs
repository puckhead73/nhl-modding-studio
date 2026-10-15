//! Player weight base across the title range (Players 3.1, 25 Sep 2026).
//!
//! `ubPc.sNZW` stores pounds minus a base. Measured on all 27 committed anchors (see the
//! `player_identity` sweep's distribution table): NHL 07 → NHL 10 store `lb − 150` (non-zero raw
//! mean 44–46, Lasse Kukkonen raw 40 = 190 lb), NHL 11 → Legacy and every PS4 title store `lb − 120`
//! (mean 69–73, Kukkonen raw 70 = 190 lb). EA's metadata is identical on every title (8-bit,
//! 0..255), so the base is decidable only from the stored values.
//!
//! The base used to be inferred from the *rating* scale probe, which is a different question: it
//! fired 150 on the PS4 titles (no 6-bit rating field there) and 120 on NHL 08 (its goalie table
//! carries a 6-bit value above 49) — 30 lb wrong on display and, through the mirrored edit form,
//! 30 lb wrong on every staged weight.
use std::path::{Path, PathBuf};

use studio_domain::{player_view, probe_weight_base};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::RosterModel;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The 27 main-DB anchors (platform, game, db, meta).
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
    assert_eq!(out.len(), 27);
    out
}

fn expected_base(game: &str) -> i64 {
    match game {
        "nhl07" | "nhl08" | "nhl09" | "nhl10" => 150,
        _ => 120,
    }
}

#[test]
fn the_weight_base_probes_150_on_nhl07_to_10_and_120_from_nhl11_and_on_every_ps4_title() {
    for (platform, game, db_path, _) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        assert_eq!(
            probe_weight_base(&db, bio),
            expected_base(&game),
            "{platform}/{game}"
        );
    }
}

/// The same player weighs the same on every title that carries him: Lasse Kukkonen, 190 lb (raw 40
/// on the base-150 titles, raw 70 on the base-120 titles; EA's NHL 08 roster has him at 187), through
/// the header's formatted weight.
#[test]
fn kukkonen_weighs_190_lb_on_every_title_that_lists_him() {
    let mut seen = 0;
    for (platform, game, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
        let model = RosterModel::build(&db).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let (first, last) = (
            bio.fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == "HdeP")
                .unwrap(),
            bio.fields
                .iter()
                .find(|f| f.id.canonical(db.endian) == "QbMR")
                .unwrap(),
        );
        let id_f = bio
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "wBIz")
            .unwrap();
        for rec in 0..bio.current_records as u32 {
            let f = db
                .read_inline_str(bio, rec, first)
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            let l = db
                .read_inline_str(bio, rec, last)
                .map(|b| String::from_utf8_lossy(b).into_owned())
                .unwrap_or_default();
            if f == "Lasse" && l == "Kukkonen" {
                let pid = db.read_u64(bio, rec, id_f).unwrap();
                let v = player_view(&db, &meta, &enums, &model, pid)
                    .unwrap_or_else(|| panic!("{platform}/{game}: view"));
                // EA's NHL 08 roster lists him at 187 lb (raw 37 on the 150 base); 190 lb (raw 40 / raw 70) everywhere after.
                let expect = if game == "nhl08" { "187 lb" } else { "190 lb" };
                assert_eq!(
                    v.header.weight.as_deref(),
                    Some(expect),
                    "{platform}/{game}"
                );
                assert_eq!(
                    v.header.height.as_deref(),
                    Some("6'1\""),
                    "{platform}/{game}"
                );
                seen += 1;
            }
        }
    }
    assert_eq!(
        seen, 24,
        "Kukkonen is on every anchor but NHL 07 and PS4 NHL 21"
    );
}
