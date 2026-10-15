//! Save-container codec verified against the committed PS3 ROSTER anchors (one per variant).

use std::path::Path;
use tdb_core::TdbFile;
use tdb_savedata::{ContainerVersion, SaveContainer, SaveType};

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../anchors")
            .join(rel),
    )
    .unwrap()
}

const V3_ROSTER: &str = "ps3/nhl09/savefiles/BLUS3016802002026 - ROSTER/SYS-DATA";
const V1_ROSTER: &str = "ps3/nhl10/savefiles/BLUS303450200 - ROSTER/SYS-DATA";
const V2_ROSTER: &str = "ps3/nhl12/savefiles/BLUS307910200 - ROSTER/SYS-DATA";
const V4_ROSTER: &str = "ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA";

fn paths() -> [(&'static str, ContainerVersion); 4] {
    [
        (V3_ROSTER, ContainerVersion::RawV3),
        (V1_ROSTER, ContainerVersion::RawV1),
        (V2_ROSTER, ContainerVersion::RawV2),
        (V4_ROSTER, ContainerVersion::ZlibV4),
    ]
}

/// Header parse identifies the magic, save type, and the correct variant for each anchor — and the
/// **stored checksums verify against a fresh computation** (proves both checksum routines on real
/// EA save data).
#[test]
fn parses_and_verifies_checksums() {
    for (rel, expect_ver) in paths() {
        let bytes = anchor(rel);
        let c = SaveContainer::parse(&bytes).unwrap();
        assert_eq!(c.header.save_type, SaveType::Roster, "{rel}");
        assert_eq!(c.header.version, expect_ver, "{rel}");
        assert!(
            c.header.magic.contains("RosterFile"),
            "{rel}: {}",
            c.header.magic
        );
        assert!(
            c.checksums_valid(),
            "{rel}: stored checksums do not match recomputation"
        );
    }
}

/// Every variant extracts a real, parseable inner TDB whose own header CRC and size check out.
#[test]
fn extracts_valid_inner_db() {
    for (rel, _) in paths() {
        let bytes = anchor(rel);
        let c = SaveContainer::parse(&bytes).unwrap();
        let db_bytes = c.extract_db().unwrap();
        let db = TdbFile::parse(&db_bytes).unwrap();
        assert!(db.size_ok(), "{rel}: inner db_size != extracted length");
        assert!(db.header_crc_ok(), "{rel}: inner header CRC mismatch");
        assert!(!db.tables.is_empty(), "{rel}: no tables in extracted db");
    }
}

/// RawV2 is uncompressed and tight, so re-injecting the *same* DB reproduces the original container
/// **byte for byte** — the strongest possible write-path proof.
#[test]
fn v2_repack_is_byte_identical() {
    let bytes = anchor(V2_ROSTER);
    let c = SaveContainer::parse(&bytes).unwrap();
    let db = c.extract_db().unwrap();
    let repacked = c.repack(&db).unwrap();
    assert_eq!(repacked, bytes, "v2 same-DB repack must be byte-identical");
}

/// ZlibV4 deflate need not reproduce EA's exact stream, so the write-path invariant is **semantic**:
/// repack → re-extract yields the identical inner DB, the file keeps its original allocation, and the
/// recomputed checksums verify.
#[test]
fn v4_repack_round_trips_semantically() {
    let bytes = anchor(V4_ROSTER);
    let c = SaveContainer::parse(&bytes).unwrap();
    let db = c.extract_db().unwrap();

    let repacked = c.repack(&db).unwrap();
    assert_eq!(
        repacked.len(),
        bytes.len(),
        "v4 keeps the original allocation"
    );

    let c2 = SaveContainer::parse(&repacked).unwrap();
    assert!(c2.checksums_valid(), "recomputed v4 checksums must verify");
    assert_eq!(c2.extract_db().unwrap(), db, "inner DB survives the repack");
}

/// An edit survives a v4 repack: flip a byte in the inner DB, repack, re-extract, see the change.
#[test]
fn v4_repack_carries_an_edit() {
    let bytes = anchor(V4_ROSTER);
    let c = SaveContainer::parse(&bytes).unwrap();
    let mut db = c.extract_db().unwrap();
    let probe = db.len() / 2;
    db[probe] ^= 0xFF;

    let repacked = c.repack(&db).unwrap();
    let c2 = SaveContainer::parse(&repacked).unwrap();
    assert_eq!(
        c2.extract_db().unwrap(),
        db,
        "edited DB round-trips through v4"
    );
}

/// Gen6 (NHL07-09 `RawV3`) is uncompressed and tight, so re-injecting the same DB reproduces the
/// container byte for byte — same strong write-path proof as v2, plus a single-checksum recompute.
#[test]
fn gen6_repack_is_byte_identical() {
    let bytes = anchor(V3_ROSTER);
    let c = SaveContainer::parse(&bytes).unwrap();
    assert_eq!(c.header.version, ContainerVersion::RawV3);
    assert!(
        !c.header.version.write_is_experimental(),
        "gen6 write is fully supported"
    );
    let db = c.extract_db().unwrap();
    let repacked = c.repack(&db).unwrap();
    assert_eq!(
        repacked, bytes,
        "gen6 same-DB repack must be byte-identical"
    );
}

/// An edit survives a gen6 repack and the recomputed `checksum_a` verifies.
#[test]
fn gen6_repack_carries_an_edit() {
    let bytes = anchor(V3_ROSTER);
    let c = SaveContainer::parse(&bytes).unwrap();
    let mut db = c.extract_db().unwrap();
    let probe = db.len() / 2;
    db[probe] ^= 0xFF;

    let repacked = c.repack(&db).unwrap();
    let c2 = SaveContainer::parse(&repacked).unwrap();
    assert!(
        c2.checksums_valid(),
        "recomputed gen6 checksum_a must verify"
    );
    assert_eq!(
        c2.extract_db().unwrap(),
        db,
        "edited DB round-trips through gen6"
    );
}

/// v1 (NHL10/11) is now writeable, but **experimentally** — a no-op repack is byte-identical (the
/// un-reversed `tag_1c` is preserved verbatim and `checksum_a` recomputes to the same value), and an
/// edit round-trips through re-extraction. The stale-`tag_1c` caveat is flagged via
/// `write_is_experimental`.
#[test]
fn v1_repack_round_trips_experimentally() {
    let bytes = anchor(V1_ROSTER);
    let c = SaveContainer::parse(&bytes).unwrap();
    assert!(c.writeable(), "v1 is now writeable");
    assert!(
        c.header.version.write_is_experimental(),
        "v1 write is experimental (tag_1c)"
    );

    let db = c.extract_db().unwrap();
    let repacked = c.repack(&db).unwrap();
    assert_eq!(
        repacked, bytes,
        "v1 same-DB repack must be byte-identical (tag_1c preserved)"
    );

    // An edit survives and the recomputed checksum_a verifies (tag_1c stays stale — the caveat).
    let mut edited = db.clone();
    let probe = edited.len() / 2;
    edited[probe] ^= 0xFF;
    let repacked = c.repack(&edited).unwrap();
    let c2 = SaveContainer::parse(&repacked).unwrap();
    assert!(c2.checksums_valid(), "recomputed v1 checksum_a must verify");
    assert_eq!(
        c2.extract_db().unwrap(),
        edited,
        "edited DB round-trips through v1"
    );
}
