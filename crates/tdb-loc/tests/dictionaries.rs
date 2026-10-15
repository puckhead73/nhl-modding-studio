//! Fixture checks over every committed localization dictionary (O6 / L1-support-LOC).
//!
//! Row counts and reference strings are compared with the authorized reference reader's results
//! (`tool.mjs` on NHL 15/16/17/18/20: 24 251 / 27 468 / 30 799 / 33 396 / 37 734 rows, all CRCs
//! matching, `NHL` → "NHL®", `TXT_ROW_LEAGUE` → "Rest Of World"). The big-endian PS3 / Xbox 360
//! dictionaries are decoded with the same code path; their acceptance rests on the checks here
//! (CRCs, strict UTF-8, hash agreement with every key) — no reference reader exists for them.
//!
//! PROVENANCE (REG-1, 2026-09-18): the file committed as `anchors/ps4/nhl20/databases/eng_us.db`
//! is the **NHL 19 base** dictionary (byte-identical to CUSA11126 `inxp_00.big` member 936); the
//! NHL 20 install ships a different 40 535-row file with no anchor. The `"nhl20"` row below is
//! therefore an NHL 19 base measurement kept under the anchor's path. The PS3 NHL 08 / NHL 09
//! `nhl_eng_us.db` anchors are the Xbox 360 files (the PS3 discs ship different ones). The real
//! sources are covered by the private-fixture cases in `tests/private_dictionaries.rs`.

use std::path::{Path, PathBuf};

use tdb_loc::{loc_hash, verify, LocDictionary};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn load(rel: &str) -> LocDictionary {
    let bytes = std::fs::read(repo(rel)).unwrap();
    for c in verify(&bytes).unwrap() {
        assert!(
            c.matches(),
            "{rel}: CRC scope {} stored {:08x} computed {:08x}",
            c.name,
            c.stored,
            c.computed
        );
    }
    LocDictionary::parse(&bytes).unwrap_or_else(|e| panic!("{rel}: {e}"))
}

#[test]
fn ps4_dictionaries_match_the_reference_reader() {
    for (game, rows) in [
        ("nhl15", 24_251usize),
        ("nhl16", 27_468),
        ("nhl17", 30_799),
        ("nhl18", 33_396),
        ("nhl20", 37_734),
    ] {
        let d = load(&format!("anchors/ps4/{game}/databases/eng_us.db"));
        assert!(!d.stats.endian_big);
        assert_eq!(d.len(), rows, "{game}");
        assert_eq!(
            d.stats.allocated_rows, rows,
            "{game}: allocated == current on originals"
        );
        assert_eq!(d.stats.stride, 16);
        assert_eq!(d.stats.invalid_utf8, 0, "{game}");
        assert_eq!(
            d.stats.hash_mismatches, 0,
            "{game}: every stored hash is loc_hash(key)"
        );
        assert!(
            d.stats.symbols >= 150 && d.stats.symbols <= 200,
            "{game}: {} symbols",
            d.stats.symbols
        );
        assert_eq!(d.text("NHL"), Some("NHL®"), "{game}");
        assert_eq!(d.text("TXT_ROW_LEAGUE"), Some("Rest Of World"), "{game}");
        assert_eq!(d.text("OHL"), Some("OHL"), "{game}");
        assert!(
            !d.contains_key("AHL"),
            "{game}: AHL is not a key in the supplied dictionaries"
        );
    }
    let d20 = load("anchors/ps4/nhl20/databases/eng_us.db");
    assert_eq!(
        d20.text("TXT_LEAGUE_NHL_ALUMNI"),
        Some("NHL Alumni Association")
    );
    assert_eq!(d20.text("elitserien"), Some("SHL"));
    assert_eq!(
        d20.text("TXT_LEAGUE_EBEL"),
        Some("Erste Bank Eishockey Liga")
    );
    let d15 = load("anchors/ps4/nhl15/databases/eng_us.db");
    assert!(!d15.contains_key("TXT_LEAGUE_EBEL"));
}

#[test]
fn big_endian_dictionaries_decode_with_the_same_layout() {
    // Every PS3 / Xbox 360 anchor ships a dictionary (NHL 10 PS3 has no metadata XML but the DB
    // parses on its own). PS3 and Xbox 360 copies are byte-identical per title.
    let mut checked = 0;
    for platform in ["ps3", "xbox360"] {
        for game in [
            "nhl07",
            "nhl08",
            "nhl09",
            "nhl10",
            "nhl11",
            "nhl12",
            "nhl13",
            "nhl14",
            "nhl15",
            "nhllegacy",
        ] {
            let rel = format!("anchors/{platform}/{game}/databases/nhl_eng_us.db");
            if !repo(&rel).is_file() {
                continue;
            }
            let d = load(&rel);
            assert!(d.stats.endian_big, "{rel}");
            assert!(d.len() > 1000, "{rel}: {} rows", d.len());
            assert_eq!(d.stats.invalid_utf8, 0, "{rel}");
            assert_eq!(
                d.stats.hash_mismatches, 0,
                "{rel}: stored hashes equal loc_hash(key)"
            );
            assert_eq!(d.stats.distinct_keys, d.len(), "{rel}: keys are distinct");
            assert!(
                d.rows.iter().all(|r| r.key.is_some()),
                "{rel}: every row has a key"
            );
            checked += 1;
        }
    }
    assert_eq!(checked, 20);
    let legacy = load("anchors/ps3/nhllegacy/databases/nhl_eng_us.db");
    assert_eq!(legacy.len(), 25_783);
    // Reference strings the gen7 dictionaries are expected to carry.
    assert!(
        legacy.contains_key("NHL") || legacy.rows.iter().any(|r| r.text.as_deref() == Some("NHL")),
        "Legacy dictionary carries an NHL string"
    );
}

#[test]
fn hash_agrees_on_the_shipped_collision_pair() {
    // The NHL 21 snapshot documents `" TXT_STATS_RANK"` and `"TXT_STATS_RANK"` sharing hash
    // 1507078883 (the mask strips the leading-space difference). The routine reproduces that.
    assert_eq!(loc_hash("TXT_STATS_RANK"), 1507078883);
    assert_eq!(loc_hash(" TXT_STATS_RANK"), 1507078883);
}

#[test]
fn corrupt_inputs_fail_safely() {
    let good = std::fs::read(repo("anchors/ps4/nhl15/databases/eng_us.db")).unwrap();
    // Truncated: the stored size no longer matches → layout error.
    let truncated = &good[..good.len() / 2];
    assert!(LocDictionary::parse(truncated).is_err());
    // A pointer driven outside the pool → pool error, not a panic. Records start at
    // block + 40 + 3*16; the first record's text pointer is its first 4 bytes.
    let mut bad = good.clone();
    let t = 32 + u32::from_le_bytes([good[28], good[29], good[30], good[31]]) as usize;
    let rec0 = t + 40 + 3 * 16;
    bad[rec0..rec0 + 4].copy_from_slice(&0x7FFF_FFF0u32.to_le_bytes());
    let err = LocDictionary::parse(&bad).unwrap_err().to_string();
    assert!(err.contains("outside the pool"), "{err}");
    // A damaged tree marker → bounded error.
    let mut bad = good.clone();
    let pool_start = {
        let d = tdb_core::TdbFile::parse(&good).unwrap();
        let tb = &d.tables[0];
        tb.record_block + tb.max_records as usize * tb.record_len_bytes as usize
    };
    for b in bad[pool_start..pool_start + 700].iter_mut() {
        *b = 1;
    }
    assert!(LocDictionary::parse(&bad).is_err());
    // CRC verification reports a mismatch instead of silently accepting an altered body.
    let mut altered = good.clone();
    altered[pool_start + 800] ^= 0xFF;
    let checks = verify(&altered).unwrap();
    assert!(checks.iter().any(|c| c.name == "body" && !c.matches()));
}

/// LOC-W1: on every committed dictionary (both byte orders, 25 files) the CRC layout is the one the
/// generic recompute reproduces (round-trip invariant), and a text-pointer re-point of the first
/// two rows with distinct non-null pointers changes exactly the slot plus the body CRC word —
/// measured per file, not assumed from one layout.
#[test]
fn every_committed_dictionary_repoints_with_exactly_one_crc_scope_moving() {
    let mut checked = 0;
    let mut files: Vec<String> = Vec::new();
    for platform in ["ps3", "xbox360"] {
        for game in [
            "nhl07", "nhl08", "nhl09", "nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15",
            "nhllegacy",
        ] {
            files.push(format!("anchors/{platform}/{game}/databases/nhl_eng_us.db"));
        }
    }
    for game in ["nhl15", "nhl16", "nhl17", "nhl18", "nhl20"] {
        files.push(format!("anchors/ps4/{game}/databases/eng_us.db"));
    }
    for rel in files {
        let bytes = std::fs::read(repo(&rel)).unwrap();
        assert_eq!(
            tdb_core::TdbFile::parse(&bytes).unwrap().serialize(),
            bytes,
            "{rel}: round-trip invariant"
        );
        let d = LocDictionary::parse(&bytes).unwrap();
        let a = d.rows.iter().position(|r| r.key.is_some() && r.text.is_some()).unwrap();
        let b = d
            .rows
            .iter()
            .enumerate()
            .position(|(i, r)| i != a && r.key.is_some() && r.text.is_some() && r.text != d.rows[a].text)
            .unwrap();
        let (ka, kb) = (d.rows[a].key.clone().unwrap(), d.rows[b].key.clone().unwrap());
        let plan = tdb_loc::plan_repoint(&bytes, &ka, &kb).unwrap_or_else(|e| panic!("{rel}: {e}"));
        let out = tdb_loc::apply_repoint(&bytes, &plan).unwrap_or_else(|e| panic!("{rel}: {e}"));
        assert_eq!(out.bytes.len(), bytes.len(), "{rel}");
        assert_eq!(out.crc_scopes_changed, vec!["body"], "{rel}");
        assert!(out.changed_ranges.len() <= 2, "{rel}: {:?}", out.changed_ranges);
        assert_eq!(
            LocDictionary::parse(&out.bytes).unwrap().text(&ka),
            d.text(&kb),
            "{rel}"
        );
        checked += 1;
    }
    assert_eq!(checked, 25);
}
