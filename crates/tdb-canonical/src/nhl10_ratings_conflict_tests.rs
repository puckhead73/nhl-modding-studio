//! The NHL 10 six-bit rating regime (package C0-AS4, following the C0-REG3 measurement).
//!
//! Two committed sources disagree about which offset the NHL 10 ratings carry. The ratings audit
//! (`docs/audits/ratings-write-codec.md`) scopes its 6-bit / offset-36 regime row to "NHL 10 →
//! Legacy / 11–21 (PS3/X360)" by title name. The production probe that the same audit designates as
//! the offset's source (`studio_domain::view::sixbit_rating_offset`) returns 36 only when some
//! 6-bit field on the skater or goalie block stores a value above 49, else 50 — and on the NHL 10
//! anchors (PS3 and Xbox 360 main DBs are byte-identical) the largest such value is 49.
//!
//! These cases do not decide between 36 and 50 and do not touch either source. They pin what is
//! measured on the Xbox 360 file itself, and hold the two NHL 10 adapters to the same quarantined
//! state so the correction cannot drift apart from the PS3 registration it follows. What would
//! resolve the conflict is an independent witness of the game's interpretation, recorded as a C1
//! finding in the C0-AS4 receipt — nothing in this repository is one.

use super::*;
use crate::target_evidence::*;
use std::collections::BTreeMap;
use tdb_core::TdbFile;

const XBOX360: &str = "anchors/xbox360/nhl10/databases/nhlng.db";
const XBOX360_SHA256: &str = "e87b8c806b1d351ae504b9bf0b54c070e999a880e88773328e72f2c4f988d703";
const PS3: &str = "anchors/ps3/nhl10/databases/nhlng.db";
const CASE: &str = "crates/tdb-canonical/src/nhl10_ratings_conflict_tests.rs#the_xbox360_anchor_stores_the_same_six_bit_maximum_and_the_adapter_records_the_same_conflict";

fn adapter(name: &str) -> Adapter {
    let text = std::fs::read_to_string(repo_root().join("defs/adapters").join(name)).unwrap();
    parse_adapter(&text).unwrap_or_else(|e| panic!("{name} parses: {e}"))
}

/// The maximum stored in any 6-bit field of the skater and goalie companion tables, with the field
/// counts — the exact input of the probe's rule — and the stored range of `p_speed`.
fn six_bit_measurement(db: &TdbFile) -> (BTreeMap<&'static str, usize>, u64, (u64, u64)) {
    let mut counts = BTreeMap::new();
    let mut largest = 0;
    for tbl in ["dSvy", "mHuy"] {
        let t = table(db, tbl);
        let mut n = 0;
        for f in t.fields.iter().filter(|f| f.bit_width == 6) {
            n += 1;
            for rec in 0..u32::from(t.current_records) {
                largest = largest.max(read(db, t, rec, f));
            }
        }
        counts.insert(tbl, n);
    }
    let skater = table(db, "dSvy");
    let speed = field(db, skater, "qPWY");
    let range = (0..u32::from(skater.current_records))
        .map(|rec| read(db, skater, rec, &speed))
        .fold((u64::MAX, 0), |(lo, hi), v| (lo.min(v), hi.max(v)));
    (counts, largest, range)
}

/// Measured on the Xbox 360 file by path and hash (not inferred from the PS3 identity): 24 six-bit
/// skater fields, 21 six-bit goalie fields, largest stored value 49, `p_speed` 2..47. The adapter
/// declares the mapping blocked, the transform unknown, the probe's 50 as a derived candidate that
/// cites this case, and names both sources.
#[test]
fn the_xbox360_anchor_stores_the_same_six_bit_maximum_and_the_adapter_records_the_same_conflict() {
    let bytes = read_pinned(XBOX360, XBOX360_SHA256);
    let db = TdbFile::parse(&bytes).unwrap();
    let (counts, largest, range) = six_bit_measurement(&db);
    assert_eq!(counts, BTreeMap::from([("dSvy", 24), ("mHuy", 21)]));
    assert_eq!(largest, 49, "the probe's threshold is `> 49`");
    assert_eq!(range, (2, 47));
    // The same bytes as the PS3 anchor — stated as an observation, with the measurement above
    // taken on this file regardless.
    assert_eq!(bytes, std::fs::read(repo_root().join(PS3)).unwrap());

    let a = adapter("nhl10-xbox360.toml");
    let m = mapping(&a, "player.rating_speed");
    assert_eq!(m.qualification, "blocked");
    assert!(transform_unresolved(m));
    assert!(!m.inverse_verified);
    assert!(m.automated_case.is_none());
    assert_eq!(m.transform.state, ConstraintState::Unknown);
    assert!(m.transform.provenance.is_empty());
    assert!(m.provenance.iter().any(|p| p == CASE));
    let c = m
        .transform
        .candidate
        .as_ref()
        .expect("the probe's result is kept");
    assert_eq!(c.confidence.as_deref(), Some("derived"));
    assert!(c.provenance.iter().any(|p| p == CASE));
    let v = c.value.as_ref().unwrap();
    assert_eq!((v.kind.as_str(), v.offset), ("linear-offset", Some(50)));
    assert!(c.note.contains("NOT OPERATIVE"));
    for s in [
        "docs/audits/ratings-write-codec.md",
        "crates/studio-domain/src/view.rs#sixbit_rating_offset",
    ] {
        assert!(m.provenance.iter().any(|p| p == s), "{s}");
    }
    for s in ["36", "50", "49", "nhl10-ps3"] {
        assert!(m.note.contains(s), "{s}");
    }
    assert_eq!(human_range(m), None);
    assert_eq!(validate_adapter(&a, &schema_for_validation()), vec![]);
}

/// The two NHL 10 adapters — one measured on its own fixture (PS3, C0-REG3), one a structural
/// C0-CAN1 adapter corrected by C0-AS4 — hold the same quarantine on the same field: same storage
/// layout, same blocked state, same candidate value, and both name the same two sources. Only the
/// case each cites differs, because each was measured on its own path.
#[test]
fn both_nhl10_adapters_hold_the_same_quarantine_on_the_speed_rating() {
    let x = adapter("nhl10-xbox360.toml");
    let p = adapter("nhl10-ps3.toml");
    let mx = mapping(&x, "player.rating_speed");
    let mp = mapping(&p, "player.rating_speed");
    assert_eq!(
        (mx.table.as_deref(), mx.field.as_deref()),
        (Some("dSvy"), Some("qPWY"))
    );
    assert_eq!(
        (mx.table.as_deref(), mx.field.as_deref()),
        (mp.table.as_deref(), mp.field.as_deref())
    );
    assert_eq!(mx.storage.established(), mp.storage.established());
    for m in [mx, mp] {
        assert_eq!(m.qualification, "blocked");
        assert_eq!(
            (m.read_transform.as_str(), m.write_transform.as_str()),
            ("unresolved", "unresolved")
        );
        assert_eq!(m.transform.state, ConstraintState::Unknown);
        let c = m.transform.candidate.as_ref().unwrap();
        assert_eq!(c.confidence.as_deref(), Some("derived"));
        assert_eq!(c.value.as_ref().unwrap().offset, Some(50));
        assert!(m
            .provenance
            .iter()
            .any(|p| p == "docs/audits/ratings-write-codec.md"));
        assert!(m
            .provenance
            .iter()
            .any(|p| p == "crates/studio-domain/src/view.rs#sixbit_rating_offset"));
        assert!(m.note.contains("two committed sources disagree"));
    }
    // Neither adapter carries an operative offset anywhere on this field.
    for m in [mx, mp] {
        assert!(m.transform.established().is_none());
        assert_ne!(m.read_transform, "offset+36");
        assert_ne!(m.read_transform, "offset+50");
    }
}

/// The two sources still say what the adapters cite them for — so a later change to either one
/// (a corrected audit row, a changed probe) surfaces here and forces the adapters to be revisited
/// rather than silently going stale.
#[test]
fn the_cited_sources_still_disagree_as_recorded() {
    let audit =
        std::fs::read_to_string(repo_root().join("docs/audits/ratings-write-codec.md")).unwrap();
    assert!(audit.contains("NHL 10 → Legacy / 11–21 (PS3/X360) 6-bit | 0..63 | **36**"));
    let probe =
        std::fs::read_to_string(repo_root().join("crates/studio-domain/src/view.rs")).unwrap();
    assert!(probe.contains("fn sixbit_rating_offset"));
    assert!(probe.contains("> 49"));
    assert!(probe.contains("return 36;"));
    assert!(probe.contains("50 // no 6-bit rating exceeds 49"));
}
