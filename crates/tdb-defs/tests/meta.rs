//! Cross-check the FieldMeta index against the binary databases (the anchors).

use std::path::Path;
use tdb_core::TdbFile;
use tdb_defs::{FieldMetaIndex, FieldType};

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

fn anchor_str(rel: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

/// Every binary field (raw ids) has a meta entry with a matching bit-depth — for BE and LE.
#[test]
fn meta_matches_binary() {
    for (db_rel, meta_rel) in [
        (
            "ps3/nhl14/databases/nhlng.db",
            "ps3/nhl14/databases/nhlng-meta.xml",
        ),
        (
            "ps4/nhl19/databases/nhl_game.db",
            "ps4/nhl19/databases/nhl_game-meta.xml",
        ),
    ] {
        let bytes = anchor(db_rel);
        let db = TdbFile::parse(&bytes).unwrap();
        let meta = FieldMetaIndex::from_xml(&anchor_str(meta_rel)).unwrap();

        let mut checked = 0;
        for t in &db.tables {
            let traw = t.id.raw();
            for f in &t.fields {
                let fm = meta
                    .field(&traw, &f.id.raw())
                    .unwrap_or_else(|| panic!("no meta for {}.{} in {db_rel}", traw, f.id.raw()));
                assert_eq!(
                    fm.bit_depth,
                    f.bit_width,
                    "depth mismatch {}.{}",
                    traw,
                    f.id.raw()
                );
                checked += 1;
            }
        }
        assert!(checked > 1000, "{db_rel}: only {checked} fields checked");
        eprintln!("{db_rel}: {checked} fields match meta (id + bit-depth)");
    }
}

/// Decoded integer values from real records stay within their declared legal range.
#[test]
fn decoded_values_in_range() {
    let bytes = anchor("ps3/nhllegacy/databases/nhlng.db");
    let db = TdbFile::parse(&bytes).unwrap();
    let meta =
        FieldMetaIndex::from_xml(&anchor_str("ps3/nhllegacy/databases/nhlng-meta.xml")).unwrap();

    // A spread of player/team/AI tables (raw ids of ubPc/dSvy/eGlu/kOtt).
    for traw in ["cPbu", "yvSd", "ulGe", "ttOk"] {
        let t = db.table(traw).unwrap();
        let rows = t.current_records.min(200) as u32;
        for f in &t.fields {
            // Fixed-bit uints only, with a declared range.
            if f.kind != 3 || f.bit_width > 64 {
                continue;
            }
            let Some(fm) = meta.field(traw, &f.id.raw()) else {
                continue;
            };
            if fm.kind != FieldType::Integer {
                continue;
            }
            let (Some(lo), Some(hi)) = (fm.range_low, fm.range_high) else {
                continue;
            };
            for rec in 0..rows {
                let raw = db.read_u64(t, rec, f).unwrap();
                let v = fm.decode_raw(raw);
                assert!(
                    v >= lo && v <= hi,
                    "{}.{} rec {rec}: decoded {v} outside [{lo}, {hi}]",
                    traw,
                    f.id.raw()
                );
            }
        }
    }
}
