//! P2-A (21 Sep 2026) — the promoted `chunklzx` decoder against the research vector manifest
//! (`docs/research/overnight-2026-09-18/data/chunklzx-vectors/manifest.json`, handoff 40
//! `codec-chunklzx-framing`): 50 members of six Xbox 360 titles (NHL 11–15, Legacy), each read
//! from its real archive on `X:` by the manifest's offset/size, identity-checked (input SHA-256),
//! decoded by `tdb_archive_codec::decompress_chunklzx` and compared with the manifest's output
//! SHA-256 / length **and** with the loose extracted plaintext in the same tree. A missing mount
//! fails (never skips). Read only.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

#[test]
#[ignore = "real Xbox 360 installs under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn every_research_vector_decodes_byte_exact() {
    let m: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-18/data/chunklzx-vectors/manifest.json")).unwrap()).unwrap();
    assert_eq!(m["window_bits"].as_u64(), Some(17));
    let archives = m["archives"].as_object().unwrap();
    let vectors = m["vectors"].as_array().unwrap();
    assert_eq!(vectors.len(), 50);
    let mut ran = 0usize;
    let mut multi_chunk = 0usize;
    let mut full32k_last = 0usize;
    let mut store_ok = 0usize;
    let r30: serde_json::Value = serde_json::from_slice(&std::fs::read(repo("docs/research/overnight-2026-09-21/data/r30-chunklzx-store-roundtrip.json")).unwrap()).unwrap();
    for v in vectors {
        let key = format!("{}/{}", v["title"].as_str().unwrap(), v["archive"].as_str().unwrap());
        let apath = PathBuf::from(archives[&key]["path"].as_str().unwrap());
        assert!(apath.is_file(), "{key}: archive missing (fails, never skips)");
        let offset = v["offset"].as_u64().unwrap();
        let size = v["stored_size"].as_u64().unwrap() as usize;
        let mut f = std::fs::File::open(&apath).unwrap();
        f.seek(SeekFrom::Start(offset)).unwrap();
        let mut raw = vec![0u8; size];
        f.read_exact(&mut raw).unwrap();
        assert_eq!(hex(&Sha256::digest(&raw)), v["input_sha256"].as_str().unwrap(), "{key} {}: input identity", v["member"]);
        assert_eq!(&raw[..8], b"chunklzx");
        let t0 = std::time::Instant::now();
        let out = tdb_archive_codec::decompress_chunklzx(&raw).unwrap_or_else(|e| panic!("{key} {}: {e}", v["member"]));
        assert_eq!(out.len() as u64, v["output_length"].as_u64().unwrap(), "{key} {}: length", v["member"]);
        assert_eq!(hex(&Sha256::digest(&out)), v["output_sha256"].as_str().unwrap(), "{key} {}: output hash", v["member"]);
        let plain = PathBuf::from(v["plaintext"]["path"].as_str().unwrap());
        assert!(plain.is_file(), "{key} {}: loose plaintext missing", v["member"]);
        assert_eq!(out, std::fs::read(&plain).unwrap(), "{key} {}: equals the loose extracted file", v["member"]);
        // P3 (21 Sep 2026, research handoff 06 / r30): the store route re-encodes the plaintext into
        // a chunklzx member of exactly the research encoder's size that decodes back byte-exact.
        let store = tdb_archive_codec::compress_chunklzx_store(&out).unwrap();
        let r30v = r30["vectors"].as_array().unwrap().iter().find(|x| x["title"] == v["title"] && x["member"] == v["member"]).unwrap_or_else(|| panic!("{key} {}: no r30 row", v["member"]));
        assert_eq!(store.len() as u64, r30v["store_encoded_size"].as_u64().unwrap(), "{key} {}: store size as the research encoder", v["member"]);
        assert_eq!(tdb_archive_codec::decompress_chunklzx(&store).unwrap(), out, "{key} {}: store round trip", v["member"]);
        assert!(tdb_archive_big::eb::is_chunklzx(&store));
        store_ok += 1;
        let chunks = v["chunks"].as_array().unwrap();
        if chunks.len() > 1 {
            multi_chunk += 1;
        }
        if chunks.iter().any(|c| c["last_frame_usize"].as_u64() == Some(32768)) {
            full32k_last += 1;
        }
        eprintln!("{key} {}: {} chunks, {} -> {} B, byte-exact ({:?})", v["member"], chunks.len(), size, out.len(), t0.elapsed());
        ran += 1;
    }
    assert_eq!(ran, 50);
    assert_eq!(store_ok, 50, "store route round-trips every vector");
    assert!(multi_chunk >= 6, "two-chunk members covered ({multi_chunk})");
    assert!(full32k_last >= 6, "a full 32 768-byte last frame covered ({full32k_last})");
}
