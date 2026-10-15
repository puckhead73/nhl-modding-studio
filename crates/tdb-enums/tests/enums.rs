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
    // 30 → 34 (receipt 28): four title-scoped nationality files (NHL 07, 08 / 09, 10, 11 / 12) beside
    // the Legacy-anchored table, which is now scoped to NHL 13 → 21 and Legacy.
    assert_eq!(reg.len(), 34, "expected the 34 shipped enum defs");

    // Arena Type (StEO.dxQr) — the ice-surface flag.
    assert_eq!(reg.label("StEO", "dxQr", 0), Some("NHL Ice (200×85)"));
    assert_eq!(
        reg.label("StEO", "dxQr", 1),
        Some("International Ice (200×100)")
    );

    // Team classification (kOtt league / conference / division) is deliberately NOT a global
    // enum: the codes change layout per title, so `studio-domain::leagues` resolves them from
    // per-layout vocabularies (`defs/leagues`) or the PS4 `stockleagues` table.
    assert!(reg.for_field("kOtt", "xMjj").is_none());
    assert!(reg.for_field("kOtt", "OwTu").is_none());
    assert!(reg.for_field("kOtt", "CGMK").is_none());

    // Career League (the four stock career tables' xMjj flag) resolves on every binding site.
    for t in ["tsSy", "OAqO", "USfF", "nCDp"] {
        assert_eq!(reg.label(t, "xMjj", 0), Some("NHL"));
        assert_eq!(reg.label(t, "xMjj", 1), Some("AHL"));
    }

    // Position (ubPc.vjla) and Captaincy (eGlu.mCcl) are operator-confirmed anchors.
    assert_eq!(reg.label("ubPc", "vjla", 0), Some("C"));
    assert_eq!(reg.label("ubPc", "vjla", 4), Some("G"));
    assert_eq!(reg.label("eGlu", "mCcl", 0), Some("None"));
    assert_eq!(reg.label("eGlu", "mCcl", 1), Some("Captain"));
    assert_eq!(reg.label("eGlu", "mCcl", 2), Some("Alternate"));

    // Player Style (skater, 1-based on disk) and Goalie Style (mHuy.QgFs, 0-based) share the QgFs
    // field id but are distinct enums scoped by table — the registry must not collide them.
    // Spike S4 re-verified the skater mapping against the anchor and bound it to every skater `QgFs`
    // site incl. the roster table (eGlu, the S4 target) and the career-mode mirrors.
    // Scoped to NHL 11 → 21 and Legacy since 27 Sep 2026 (research R122: NHL 07–10 number skater
    // styles differently), so it resolves only for a title inside that scope.
    let legacy = reg.scoped(Some("nhllegacy"), "xbox");
    assert_eq!(legacy.label("dSvy", "QgFs", 7), Some("Sniper"));
    assert_eq!(legacy.label("dSvy", "QgFs", 9), Some("Two-Way Forward"));
    assert_eq!(legacy.label("eGlu", "QgFs", 7), Some("Sniper"));
    assert_eq!(legacy.label("eGlu", "QgFs", 1), Some("Defensive Defenseman"));
    assert_eq!(reg.scoped(Some("nhl10"), "xbox").label("eGlu", "QgFs", 4), None, "NHL 10: no Legacy label");
    assert_eq!(legacy.label("mHuy", "QgFs", 2), Some("Butterfly"));
    assert!(
        legacy.for_field("dSvy", "QgFs").unwrap().name != legacy.for_field("mHuy", "QgFs").unwrap().name
    );
    // Same enum instance backs the roster table and the skater-AI table.
    assert_eq!(
        legacy.for_field("eGlu", "QgFs").unwrap().name,
        legacy.for_field("dSvy", "QgFs").unwrap().name
    );

    // Player Type (ZBac.RREB) — the S4-follow-up resolution: RREB is the player source
    // discriminator (TNHLPlayerType), NOT roster status.
    assert_eq!(reg.label("ZBac", "RREB", 0), Some("Built-In"));
    assert_eq!(reg.label("ZBac", "RREB", 1), Some("User-Created"));
    assert_eq!(reg.label("ZBac", "RREB", 3), Some("Developer"));

    // Team strategies (EeYo) — each tactic vocabulary resolves on both the exhibition table and
    // its career-mode mirror (JUfQ), and the shared Line Style vocab binds all four line fields.
    assert_eq!(reg.label("EeYo", "RBGh", 0), Some("Weak Side Lock"));
    assert_eq!(reg.label("JUfQ", "RBGh", 3), Some("1-2-2 Aggressive"));
    assert_eq!(reg.label("EeYo", "uAeC", 2), Some("1-2-2 Red Line"));
    assert_eq!(reg.label("EeYo", "rKer", 2), Some("Tight Point"));
    assert_eq!(reg.label("EeYo", "bame", 1), Some("Overload"));
    assert_eq!(reg.label("EeYo", "NzPy", 2), Some("Diamond"));
    assert_eq!(reg.label("EeYo", "TTVS", 4), Some("Full Attack"));
    assert_eq!(reg.label("EeYo", "xKvv", 0), Some("Protect Net"));
    for line_style in ["eRIF", "LIlj", "ALBZ", "CPwO"] {
        assert_eq!(reg.label("EeYo", line_style, 1), Some("Crash The Net"));
        assert_eq!(reg.label("JUfQ", line_style, 1), Some("Crash The Net"));
    }
}

/// Every enum's bound field, reversed into the binary id space, should exist in a real anchor's
/// metadata with a bit-width wide enough to hold the enum's largest labelled code. This ties the
/// shipped vocabularies to real data (verify-don't-guess) via the documented id-reversal rule.
#[test]
fn enum_fields_resolve_against_anchor_meta() {
    let reg = EnumRegistry::from_dir(defs_dir()).unwrap();
    let meta =
        FieldMetaIndex::from_xml(&anchor_str("ps3/nhllegacy/databases/nhlng-meta.xml")).unwrap();

    let mut matched = 0;
    for def in reg.defs() {
        let Some(max) = def.max_raw() else { continue };
        for b in &def.bindings {
            let (t, f) = (reverse_id(&b.table_raw_id), reverse_id(&b.field_raw_id));
            // Not every enum's table lives in nhlng.db (equipment/appearance tables differ); skip
            // the ones this anchor doesn't carry rather than failing on them.
            let Some(fm) = meta.field(&t, &f) else {
                continue;
            };
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
