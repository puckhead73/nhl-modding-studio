//! Save-container codec verified against the committed PS3 ROSTER anchors (one per variant).

use std::path::Path;
use tdb_core::TdbFile;
use tdb_savedata::{ContainerVersion, SaveContainer, SaveError, SaveType};

fn anchor(rel: &str) -> Vec<u8> {
    std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors").join(rel)).unwrap()
}

const V1_ROSTER: &str = "ps3/nhl10/savefiles/BLUS303450200 - ROSTER/SYS-DATA";
const V2_ROSTER: &str = "ps3/nhl12/savefiles/BLUS307910200 - ROSTER/SYS-DATA";
const V4_ROSTER: &str = "ps3/nhllegacy/savefiles/BLUS315400200 - ROSTER/SYS-DATA";

fn paths() -> [(&'static str, ContainerVersion); 3] {
    [
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
        assert!(c.header.magic.contains("RosterFile"), "{rel}: {}", c.header.magic);
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
    assert_eq!(repacked.len(), bytes.len(), "v4 keeps the original allocation");

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
    assert_eq!(c2.extract_db().unwrap(), db, "edited DB round-trips through v4");
}

/// v1 (NHL10/11) is read-only — extraction works but repack is refused.
#[test]
fn v1_is_read_only() {
    let bytes = anchor(V1_ROSTER);
    let c = SaveContainer::parse(&bytes).unwrap();
    assert!(!c.writeable());
    let db = c.extract_db().unwrap();
    assert!(TdbFile::parse(&db).is_ok());
    assert!(matches!(c.repack(&db), Err(SaveError::UnsupportedV1Write)));
}
