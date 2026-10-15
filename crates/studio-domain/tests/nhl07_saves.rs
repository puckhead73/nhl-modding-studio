//! NHL07 (Xbox 360) save containers. The gen6 ROSTER save is a `v3` container (`RawV3`) whose body
//! is a raw big-endian TDB — now parsed **and** writeable (see `tdb-savedata` gen6 support), so it is
//! an editable surface alongside the loose `NHL07NG.db`. DYNASTY (a different multi-part gen6 body)
//! and USER PROFILE (settings, no inner db) remain non-extractable / read-only.

use std::path::{Path, PathBuf};

use tdb_savedata::SaveContainer;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

/// The gen6 ROSTER save now yields an editable inner db and a writeable container; DYNASTY and USER
/// PROFILE still do not (their bodies aren't a plain TDB the codec supports).
#[test]
fn nhl07_gen6_roster_is_writeable_others_read_only() {
    // ROSTER: gen6 RawV3 — extractable + writeable.
    let roster = std::fs::read(repo("anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1")).unwrap();
    let c = SaveContainer::parse(&roster).expect("gen6 ROSTER parses");
    assert!(c.writeable(), "gen6 ROSTER is writeable");
    assert!(
        !c.header.version.write_is_experimental(),
        "gen6 ROSTER write is fully supported (single CRC-32/IEEE checksum)"
    );
    let inner = c.extract_db().expect("gen6 ROSTER yields an inner db");
    assert!(!inner.is_empty(), "extracted gen6 roster db is non-empty");
    // A same-db repack reproduces the container byte for byte (tight, single-checksum write).
    assert_eq!(
        c.repack(&inner).unwrap(),
        roster,
        "gen6 ROSTER repack is byte-identical"
    );

    // USER PROFILE is a settings save (its version word / layout is not a gen6 roster body), so it
    // yields no editable inner db and stays read-only. (DYNASTY happens to carry a gen6 DB body too,
    // but discovery classifies it `beagm`, never `roster`, so it is not an editing target.)
    let profile = std::fs::read(repo(
        "anchors/xbox360/nhl07/savefiles/USER PROFILE1/USER PROFILE1",
    ))
    .unwrap();
    let extractable = SaveContainer::parse(&profile)
        .ok()
        .and_then(|c| c.extract_db().ok())
        .is_some();
    assert!(
        !extractable,
        "USER PROFILE settings save yields no editable inner db"
    );
}
