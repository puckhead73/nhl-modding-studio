//! Read-only diagnostics for the two NHL12 members not extracted by the first pass.
use std::{fs::File, io::{Read, Seek, SeekFrom}, path::PathBuf};

#[test]
#[ignore = "requires NHL12_QA_DUMP and NHL12_QA_INDEPENDENT"]
fn inspect_remaining_members() {
    let root = PathBuf::from(std::env::var("NHL12_QA_DUMP").unwrap());
    let independent = PathBuf::from(std::env::var("NHL12_QA_INDEPENDENT").unwrap());
    for (archive_name, name) in [
        ("cacherender.big", "rendering/crowddata/crowd_29.cwd"),
        ("gamedata/anim/faceposer.big", "gamedata/faceposer/faceposer_983.cba"),
    ] {
        let mut file = File::open(root.join(archive_name)).unwrap();
        let mut prefix = vec![0; 32];
        file.read_exact(&mut prefix).unwrap();
        let header = tdb_archive_big::eb::EbHeader::parse(&prefix).unwrap();
        prefix.resize(header.table_end(), 0);
        file.rewind().unwrap(); file.read_exact(&mut prefix).unwrap();
        let archive = tdb_archive_big::eb::EbArchive::parse(&prefix).unwrap();
        let entry = archive.entries.iter().find(|e| e.path == name).unwrap();
        let index = archive.entries.iter().position(|e| e.path == name).unwrap();
        let row = 48 + index * tdb_archive_big::eb::entry_size_for(header.version);
        eprintln!("raw row {:02x?}", &prefix[row..row+16]);
        eprintln!("{archive_name}: {name}, offset {}, size {}, archive length {}", entry.offset, entry.size, file.metadata().unwrap().len());
        file.seek(SeekFrom::Start(entry.offset)).unwrap();
        let mut raw = vec![0; entry.size as usize];
        file.read_exact(&mut raw).unwrap();
        if raw.starts_with(b"chunklzx") {
            let be = |i| u32::from_be_bytes(raw[i..i+4].try_into().unwrap()) as usize;
            let mut cursor = 40usize;
            for i in 0..be(20) {
                let offset = if cursor == 40 {40} else {((cursor - 8 + 15) & !15) + 8};
                let size = be(offset); let flag = be(offset + 4);
                eprintln!("chunk {i}: offset {offset} size {size} flag {flag} prefix {:02x?}", &raw[offset+8..(offset+24).min(raw.len())]);
                cursor = offset + 8 + size;
                if flag == 4 {
                    let plain = std::fs::read(independent.join(name)).unwrap();
                    assert_eq!(&raw[offset+8..cursor], &plain[plain.len()-size..], "flag4 is exact independent plaintext tail");
                    eprintln!("flag4 matches {size} final independent plaintext bytes; declared total {} actual {}", be(12), plain.len());
                }
            }
        }
        let decoded = tdb_archive_big::eb::decode_payload(raw);
        match decoded {
            Ok(tdb_archive_big::eb::Payload::Data(bytes)) => {
                if name.ends_with(".cwd") {
                    assert_eq!(bytes, std::fs::read(independent.join(name)).unwrap());
                    eprintln!("independent plaintext matches: {} bytes", bytes.len());
                } else {
                    // The independent dump does not contain this nested member.
                    assert_eq!(bytes.len(), 19484, "declared decoded size");
                    eprintln!("faceposer bounded decode matches declared length; no independent plaintext available");
                }
            }
            other => panic!("decode result: {other:?}"),
        }
    }
}

#[test]
#[ignore = "read-only NHL12_QA_DUMP nested archive audit"]
fn every_faceposer_member_is_bounded_and_decodes() {
    let root = PathBuf::from(std::env::var("NHL12_QA_DUMP").unwrap());
    let bytes = std::fs::read(root.join("gamedata/anim/faceposer.big")).unwrap();
    let archive = tdb_archive_big::eb::EbArchive::parse(&bytes).unwrap();
    assert_eq!(tdb_archive_big::eb::EbHeader::parse(&bytes).unwrap().version, tdb_archive_big::eb::VERSION_TAG_PS4_16);
    for entry in &archive.entries {
        let start = entry.offset as usize;
        let end = start + entry.size as usize;
        assert!(end <= bytes.len(), "{} exceeds source", entry.path);
        let raw = &bytes[start..end];
        let tdb_archive_big::eb::Payload::Data(decoded) = tdb_archive_big::eb::decode_payload(raw.to_vec()).unwrap() else { panic!("unsupported {}", entry.path) };
        if raw.starts_with(b"chunkzip") || raw.starts_with(b"chunklzx") {
            assert_eq!(decoded.len(), u32::from_be_bytes(raw[12..16].try_into().unwrap()) as usize, "{}", entry.path);
        }
    }
    eprintln!("{} nested faceposer members bounded and decoded", archive.entries.len());
}
