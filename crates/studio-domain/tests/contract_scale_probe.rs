//! Contract-dollar granularity per title (Players 3.2, 25 Sep 2026, package P3.2-F).
//!
//! `ubPc.contractdollars` is 9 bits on NHL 07 (×$25 000, in-game verified), 10 bits on NHL 08–10,
//! 13 bits on NHL 11–Legacy and PS4 15–17 (×$5 000, verified) and 14 bits on PS4 18–21. The 10-bit
//! and 14-bit units were unconfirmed (the 14-bit one was assumed ×$5 000 by the `>= 13` rule; the
//! 10-bit titles showed the raw number). This probe reads the stored raw value for players whose
//! cap hit is public knowledge and prints raw × candidate units, so the unit can be pinned from
//! the data rather than assumed:
//!
//! * Sidney Crosby — $8 700 000 cap hit, 2008-09 through 2012-13 (NHL 09 / 10 / 11 / 12 / 13 rosters);
//! * Alex Ovechkin — $9 538 462 cap hit, 2008-09 through 2020-21 (NHL 09 → NHL 21);
//! * Connor McDavid — $12 500 000 cap hit, 2018-19 onward (NHL 19 / 20 / 21).
use std::path::{Path, PathBuf};

use studio_domain::player_view;
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_enums::EnumRegistry;
use tdb_relations::RosterModel;

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
    assert_eq!(out.len(), 27);
    out
}

const KNOWN: &[(&str, &str, u64)] = &[
    ("Sidney", "Crosby", 8_700_000),
    ("Alex", "Ovechkin", 9_538_462),
    ("Connor", "McDavid", 12_500_000),
];

#[test]
fn contract_dollar_raw_values_of_known_cap_hits_per_anchor() {
    eprintln!("\n| anchor | contractdollars bits | player | raw | raw × 5 000 | raw × 10 000 | raw × 25 000 | known cap hit |");
    eprintln!("|---|---|---|---|---|---|---|---|");
    for (platform, game, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let by_name = |n: &str| {
            bio.fields.iter().find(|f| {
                meta.field(&bio.id.raw(), &f.id.raw())
                    .is_some_and(|m| m.name == n)
            })
        };
        let (Some(dollars), Some(first), Some(last)) = (
            by_name("contractdollars"),
            by_name("firstname"),
            by_name("lastname"),
        ) else {
            eprintln!("| {platform}/{game} | — no contractdollars field | | | | | | |");
            continue;
        };
        let mut found = 0;
        for rec in 0..bio.current_records as u32 {
            let f = db
                .read_inline_str(bio, rec, first)
                .map(|b| String::from_utf8_lossy(b).trim().to_string())
                .unwrap_or_default();
            let l = db
                .read_inline_str(bio, rec, last)
                .map(|b| String::from_utf8_lossy(b).trim().to_string())
                .unwrap_or_default();
            for (kf, kl, cap) in KNOWN {
                if f == *kf && l == *kl {
                    let raw = db.read_u64(bio, rec, dollars).unwrap_or(0);
                    eprintln!(
                        "| {platform}/{game} | {} | {f} {l} | {raw} | {} | {} | {} | {cap} |",
                        dollars.bit_width,
                        raw * 5_000,
                        raw * 10_000,
                        raw * 25_000
                    );
                    found += 1;
                }
            }
        }
        if found == 0 {
            eprintln!(
                "| {platform}/{game} | {} | (none of the known players rostered) | | | | | |",
                dollars.bit_width
            );
        }
    }
}

/// The pinned units: raw × $25 000 on the 9- and 10-bit titles (NHL 07–10), raw × $5 000 on the
/// 13- and 14-bit ones (NHL 11 → PS4 21). Read through the player card's Cap Hit.
#[test]
fn known_cap_hits_read_back_in_dollars_on_every_title_that_rosters_the_player() {
    // (first, last, game filter, expected dollars)
    let expect: &[(&str, &str, &dyn Fn(&str) -> bool, &str)] = &[
        ("Sidney", "Crosby", &|g: &str| g == "nhl07", "$850,000"),
        (
            "Sidney",
            "Crosby",
            &|g: &str| !matches!(g, "nhl07" | "nhl08"),
            "$8,700,000",
        ),
        (
            "Connor",
            "McDavid",
            &|g: &str| matches!(g, "nhl19" | "nhl20" | "nhl21"),
            "$12,500,000",
        ),
    ];
    let mut checked = 0;
    for (platform, game, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let enums = EnumRegistry::from_dir(repo("defs").join("enums")).unwrap();
        let model = RosterModel::build(&db).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let by_name = |n: &str| {
            bio.fields
                .iter()
                .find(|f| {
                    meta.field(&bio.id.raw(), &f.id.raw())
                        .is_some_and(|m| m.name == n)
                })
                .unwrap()
        };
        let (first, last) = (by_name("firstname"), by_name("lastname"));
        let id_f = bio
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "wBIz")
            .unwrap();
        for rec in 0..bio.current_records as u32 {
            let f = db
                .read_inline_str(bio, rec, first)
                .map(|b| String::from_utf8_lossy(b).trim().to_string())
                .unwrap_or_default();
            let l = db
                .read_inline_str(bio, rec, last)
                .map(|b| String::from_utf8_lossy(b).trim().to_string())
                .unwrap_or_default();
            for (kf, kl, on, dollars) in expect {
                if f == *kf && l == *kl && on(&game) {
                    let pid = db.read_u64(bio, rec, id_f).unwrap();
                    let Some(v) = player_view(&db, &meta, &enums, &model, pid) else {
                        continue;
                    };
                    // Duplicate stubs (PS4 NHL 18 carries a second, empty Crosby / McDavid row) read no contract.
                    if v.data.cap_hit.is_none() {
                        continue;
                    }
                    assert_eq!(
                        v.data.cap_hit.as_deref(),
                        Some(*dollars),
                        "{platform}/{game}: {f} {l}"
                    );
                    checked += 1;
                }
            }
        }
    }
    assert_eq!(
        checked, 28,
        "{checked} known cap hits checked (Crosby 07 ×2, Crosby 09–PS4 21 ×23, McDavid 19–21 ×3)"
    );
}

/// Audit F2 (25 Sep 2026): **several independently identified contracts per family**, and the
/// `contractdollars` column told apart from `salary`. Candidates are public cap hits (AAV) of
/// the season each title models; EA's authored figure need not coincide (pre-extension values,
/// rounding to the unit, bonuses), so this case **prints** every rostered candidate with the
/// stored raw of both columns and the dollars each reads at the family's contract unit, and the
/// pinned assertions below it name only the exact matches. Season per title: NHL 07 = 2006-07 …
/// NHL 15 / PS4 15 = 2014-15, Legacy / PS4 16 = 2015-16, PS4 17 = 2016-17 … PS4 21 = 2020-21;
/// `ps4/nhl20` is the NHL 19 base copy (2018-19).
#[test]
fn candidate_public_cap_hits_against_both_dollar_columns_per_anchor() {
    use studio_domain::{player_encoding, schema_family};
    // (first, last, cap hit, first season, last season) — seasons as the title's year (NHL 09 = 2009).
    let candidates: &[(&str, &str, u64, u32, u32)] = &[
        ("Sidney", "Crosby", 850_000, 2006, 2008),
        ("Sidney", "Crosby", 8_700_000, 2009, 2021),
        ("Alex", "Ovechkin", 984_200, 2006, 2008),
        ("Alex", "Ovechkin", 9_538_462, 2009, 2021),
        ("Evgeni", "Malkin", 8_700_000, 2010, 2014),
        ("Evgeni", "Malkin", 9_500_000, 2015, 2021),
        ("Nicklas", "Lidstrom", 7_600_000, 2007, 2008),
        ("Nicklas", "Lidstrom", 7_450_000, 2009, 2010),
        ("Joe", "Thornton", 6_666_667, 2006, 2008),
        ("Joe", "Thornton", 7_200_000, 2009, 2011),
        ("Roberto", "Luongo", 6_750_000, 2007, 2010),
        ("Roberto", "Luongo", 5_333_333, 2011, 2021),
        ("Brad", "Richards", 7_800_000, 2007, 2011),
        ("Brad", "Richards", 6_666_667, 2012, 2014),
        ("Dany", "Heatley", 7_500_000, 2009, 2014),
        ("Jarome", "Iginla", 7_000_000, 2008, 2013),
        ("Ilya", "Kovalchuk", 6_389_000, 2006, 2010),
        ("Ilya", "Kovalchuk", 6_666_667, 2011, 2013),
        ("Vincent", "Lecavalier", 6_875_000, 2006, 2009),
        ("Vincent", "Lecavalier", 7_727_273, 2010, 2013),
        ("Chris", "Pronger", 6_250_000, 2006, 2009),
        ("Jonathan", "Toews", 6_300_000, 2011, 2015),
        ("Jonathan", "Toews", 10_500_000, 2016, 2021),
        ("Patrick", "Kane", 6_300_000, 2011, 2015),
        ("Patrick", "Kane", 10_500_000, 2016, 2021),
        ("Steven", "Stamkos", 7_500_000, 2012, 2016),
        ("Steven", "Stamkos", 8_500_000, 2017, 2021),
        ("Shea", "Weber", 7_857_143, 2013, 2021),
        ("Zach", "Parise", 7_538_462, 2013, 2021),
        ("Ryan", "Suter", 7_538_462, 2013, 2021),
        ("Drew", "Doughty", 7_000_000, 2012, 2019),
        ("Drew", "Doughty", 11_000_000, 2020, 2021),
        ("Anze", "Kopitar", 6_800_000, 2009, 2016),
        ("Anze", "Kopitar", 10_000_000, 2017, 2021),
        ("Ryan", "Getzlaf", 8_250_000, 2014, 2021),
        ("Corey", "Perry", 8_625_000, 2014, 2021),
        ("P.K.", "Subban", 9_000_000, 2015, 2021),
        ("Connor", "McDavid", 925_000, 2016, 2018),
        ("Connor", "McDavid", 12_500_000, 2019, 2021),
        ("John", "Tavares", 5_500_000, 2013, 2018),
        ("John", "Tavares", 11_000_000, 2019, 2021),
        ("Carey", "Price", 6_500_000, 2013, 2018),
        ("Carey", "Price", 10_500_000, 2019, 2021),
        ("Leon", "Draisaitl", 8_500_000, 2018, 2021),
        ("Jack", "Eichel", 10_000_000, 2019, 2021),
        ("Auston", "Matthews", 11_634_000, 2020, 2021),
        ("Mitchell", "Marner", 10_893_000, 2020, 2021),
        ("Artemi", "Panarin", 11_642_857, 2020, 2021),
        ("Erik", "Karlsson", 6_500_000, 2013, 2019),
        ("Erik", "Karlsson", 11_500_000, 2020, 2021),
        ("Jamie", "Benn", 5_250_000, 2013, 2017),
        ("Jamie", "Benn", 9_500_000, 2018, 2021),
    ];
    eprintln!("\n| anchor | family | unit | player | contractdollars raw → $ | salary raw → $ | public cap hit | contractdollars match |");
    eprintln!("|---|---|---|---|---|---|---|---|");
    let mut matched_per_family: std::collections::BTreeMap<String, std::collections::BTreeSet<String>> = Default::default();
    for (platform, game, db_path, meta_path) in anchors() {
        let bytes = std::fs::read(&db_path).unwrap();
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&std::fs::read_to_string(&meta_path).unwrap()).unwrap();
        let bio = db.table_canonical("ubPc").unwrap();
        let fam = schema_family(&db, &meta).unwrap();
        let enc = player_encoding(&db, &meta).unwrap();
        // Measurement unit for the table: the contract's, or the width-implied candidate on NHL 08.
        let unit = enc.cap_hit_unit.unwrap_or(25_000);
        let year: u32 = if game == "nhllegacy" { 2016 } else { game[3..].parse::<u32>().unwrap() + 2000 };
        let year = if platform == "ps4" && game == "nhl20" { 2019 } else { year };
        let by_name = |n: &str| bio.fields.iter().find(|f| meta.field(&bio.id.raw(), &f.id.raw()).is_some_and(|m| m.name == n));
        let (Some(dollars), Some(first), Some(last)) = (by_name("contractdollars"), by_name("firstname"), by_name("lastname")) else { continue };
        let salary = by_name("salary");
        for rec in 0..bio.current_records as u32 {
            let f = db.read_inline_str(bio, rec, first).map(|b| String::from_utf8_lossy(b).trim().to_string()).unwrap_or_default();
            let l = db.read_inline_str(bio, rec, last).map(|b| String::from_utf8_lossy(b).trim().to_string()).unwrap_or_default();
            for (kf, kl, cap, from, to) in candidates {
                if f != *kf || l != *kl || year < *from || year > *to { continue; }
                let raw = db.read_u64(bio, rec, dollars).unwrap_or(0);
                let sal = salary.and_then(|s| db.read_u64(bio, rec, s));
                let read = raw * unit;
                // Exact to the unit: EA rounds to the unit (Ovechkin 9 538 462 → 9 535 000 / 9 540 000).
                let matches = (read as i64 - *cap as i64).unsigned_abs() < unit;
                eprintln!(
                    "| {platform}/{game} | {} | ×${unit}{} | {f} {l} | {raw} → ${read} | {} | ${cap} | {} |",
                    fam.label(),
                    if enc.cap_hit_unit.is_none() { " (candidate — unwitnessed)" } else { "" },
                    sal.map(|s| format!("{s} → ${}", s * unit)).unwrap_or_else(|| "— (no salary column)".into()),
                    if matches { "match" } else { "differs" }
                );
                if matches && enc.cap_hit_unit.is_some() {
                    matched_per_family.entry(fam.label().to_string()).or_default().insert(format!("{f} {l}"));
                }
            }
        }
    }
    eprintln!("\nindependent matched players per family (witnessed units only): {matched_per_family:?}");
    for (fam, players) in &matched_per_family {
        assert!(players.len() >= 2, "{fam}: fewer than two independent witnesses: {players:?}");
    }
}
