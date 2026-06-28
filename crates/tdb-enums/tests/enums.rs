//! Load the shipped `defs/enums` registry and cross-check it against a real anchor database.

use std::path::{Path, PathBuf};
use tdb_defs::FieldMetaIndex;
use tdb_enums::{reverse_id, EnumRegistry};

fn defs_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../defs/enums")
}

fn anchor_str(rel: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

/// The whole shipped registry parses, and the known core enums map as confirmed.
#[test]
fn registry_loads_all_defs() {
    let reg = EnumRegistry::from_dir(defs_dir()).unwrap();
    assert_eq!(reg.len(), 19, "expected the 19 shipped enum defs");

    // Position (ubPc.vjla) and Captaincy (eGlu.mCcl) are operator-confirmed anchors.
    assert_eq!(reg.label("ubPc", "vjla", 0), Some("C"));
    assert_eq!(reg.label("ubPc", "vjla", 4), Some("G"));
    assert_eq!(reg.label("eGlu", "mCcl", 0), Some("None"));
    assert_eq!(reg.label("eGlu", "mCcl", 1), Some("Captain"));
    assert_eq!(reg.label("eGlu", "mCcl", 2), Some("Alternate"));

    // Player Style (dSvy.QgFs) and Goalie Style (mHuy.QgFs) share the QgFs field id but are
    // distinct enums scoped by table — the registry must not collide them.
    assert_eq!(reg.label("dSvy", "QgFs", 7), Some("Sniper"));
    assert_eq!(reg.label("mHuy", "QgFs", 2), Some("Butterfly"));
    assert!(reg.for_field("dSvy", "QgFs").unwrap().name != reg.for_field("mHuy", "QgFs").unwrap().name);
}

/// Every enum's bound field, reversed into the binary id space, should exist in a real anchor's
/// metadata with a bit-width wide enough to hold the enum's largest labelled code. This ties the
/// shipped vocabularies to real data (verify-don't-guess) via the documented id-reversal rule.
#[test]
fn enum_fields_resolve_against_anchor_meta() {
    let reg = EnumRegistry::from_dir(defs_dir()).unwrap();
    let meta = FieldMetaIndex::from_xml(&anchor_str("ps3/nhllegacy/databases/nhlng-meta.xml")).unwrap();

    let mut matched = 0;
    for def in reg.defs() {
        let Some(max) = def.max_raw() else { continue };
        for b in &def.bindings {
            let (t, f) = (reverse_id(&b.table_raw_id), reverse_id(&b.field_raw_id));
            // Not every enum's table lives in nhlng.db (equipment/appearance tables differ); skip
            // the ones this anchor doesn't carry rather than failing on them.
            let Some(fm) = meta.field(&t, &f) else { continue };
            let cap = if fm.bit_depth >= 63 {
                i64::MAX
            } else {
                (1i64 << fm.bit_depth) - 1
            };
            assert!(
                max <= cap,
                "enum {:?} max code {max} does not fit {}.{} ({}-bit) in the anchor meta",
                def.name,
                t,
                f,
                fm.bit_depth
            );
            matched += 1;
        }
    }
    assert!(
        matched >= 4,
        "expected several enum fields to resolve against the legacy anchor, only {matched} did"
    );
    eprintln!("{matched} enum bindings cross-checked against the legacy anchor meta");
}
