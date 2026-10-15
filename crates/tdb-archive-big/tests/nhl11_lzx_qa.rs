//! Real-disc regression: decode the NHL11 render members missed by full extraction.
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::PathBuf};

#[test]
#[ignore = "requires read-only NHL11_QA_DUMP with archives and independently extracted files"]
fn nhl11_block_boundary_members_match_independent_files() {
    let root = PathBuf::from(std::env::var("NHL11_QA_DUMP").unwrap());
    let mut file = File::open(root.join("rendercache.big")).unwrap();
    let mut prefix = vec![0; 32];
    file.read_exact(&mut prefix).unwrap();
    let header = tdb_archive_big::eb::EbHeader::parse(&prefix).unwrap();
    prefix.resize(header.table_end(), 0);
    file.seek(SeekFrom::Start(0)).unwrap();
    file.read_exact(&mut prefix).unwrap();
    let archive = tdb_archive_big::eb::EbArchive::parse(&prefix).unwrap();
    for name in ["rendering/fetrapper/texlib_31.rx2", "rendering/pant/texlib_0_17_2.rx2", "rendering/pant/texlib_0_29_1.rx2", "rendering/pant/texlib_0_9_8.rx2", "rendering/pant/texlib_1_24_2.rx2", "rendering/skate/texlib_17.rx2"] {
        let entry = archive.entries.iter().find(|e| e.path == name).unwrap();
        file.seek(SeekFrom::Start(entry.offset)).unwrap();
        let mut raw = vec![0; entry.size as usize];
        file.read_exact(&mut raw).unwrap();
        let decoded = tdb_archive_codec::decompress_chunklzx(&raw).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(decoded, std::fs::read(root.join(name)).unwrap(), "{name}: independent plaintext");
        eprintln!("{name}: {} bytes exact", decoded.len());
    }
}

#[test]
#[ignore = "requires NHL11_QA_DUMP and NHL11_QA_FAILURES; reads failing EB members only"]
fn nhl11_all_failed_lzx_members_match_independent_files() {
    let root = PathBuf::from(std::env::var("NHL11_QA_DUMP").unwrap());
    let failures: serde_json::Value = serde_json::from_slice(&std::fs::read(std::env::var("NHL11_QA_FAILURES").unwrap()).unwrap()).unwrap();
    let mut checked = 0;
    for row in failures.as_array().unwrap().iter().filter(|r| r["note"].as_str().unwrap().starts_with("chunklzx")) {
        let archive_path = root.join(row["archive"].as_str().unwrap());
        let name = row["member"].as_str().unwrap();
        let mut file = File::open(&archive_path).unwrap();
        let mut prefix = vec![0; 32];
        file.read_exact(&mut prefix).unwrap();
        let header = tdb_archive_big::eb::EbHeader::parse(&prefix).unwrap();
        prefix.resize(header.table_end(), 0);
        file.seek(SeekFrom::Start(0)).unwrap();
        file.read_exact(&mut prefix).unwrap();
        let archive = tdb_archive_big::eb::EbArchive::parse(&prefix).unwrap();
        let entry = archive.entries.iter().find(|e| e.path == name).unwrap();
        file.seek(SeekFrom::Start(entry.offset)).unwrap();
        let mut raw = vec![0; entry.size as usize];
        file.read_exact(&mut raw).unwrap();
        let decoded = tdb_archive_codec::decompress_chunklzx(&raw).unwrap_or_else(|e| panic!("{name}: {e}"));
        assert_eq!(decoded, std::fs::read(archive_path.parent().unwrap().join(name)).unwrap(), "{name}: independent plaintext");
        eprintln!("{name}: {} bytes exact", decoded.len());
        checked += 1;
    }
    assert_eq!(checked, 34);
}

#[test]
#[ignore = "read-only NHL11_QA_DUMP nested archive extent audit"]
fn nhl11_nested_archive_extent_audit() {
    let root = PathBuf::from(std::env::var("NHL11_QA_DUMP").unwrap());
    for name in ["gamedata/aidata.big", "gamedata/nis.big", "gamedata/anim/faceposer.big"] {
        let bytes = std::fs::read(root.join(name)).unwrap();
        let archive = tdb_archive_big::eb::EbArchive::parse(&bytes).unwrap();
        for entry in &archive.entries {
            let start = entry.offset as usize;
            let end = start + entry.size as usize;
            assert!(end <= bytes.len(), "{name}: {} exceeds archive", entry.path);
            let raw = &bytes[start..end];
            let tdb_archive_big::eb::Payload::Data(decoded) = tdb_archive_big::eb::decode_payload(raw.to_vec()).unwrap() else { panic!("unsupported {}", entry.path) };
            if raw.starts_with(b"chunkzip") || raw.starts_with(b"chunklzx") {
                assert_eq!(decoded.len(), u32::from_be_bytes(raw[12..16].try_into().unwrap()) as usize);
            }
        }
        eprintln!("{name}: all {} entries bounded and decoded", archive.entries.len());
    }
}
