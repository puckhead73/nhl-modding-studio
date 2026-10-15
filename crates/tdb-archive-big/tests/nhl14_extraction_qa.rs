//! Real NHL14 read-only regression for the newly encountered faceposer variant.
#[test]
#[ignore = "requires NHL14_QA_DUMP; reads all nested faceposer payloads"]
fn faceposer_members_match_declared_sizes_and_stay_within_archive() {
    use tdb_archive_big::eb::{EbArchive, EbFile, EbError, EbHeader, Payload, decode_payload, VERSION_TAG_GEN7_FACEPOSER};
    let root = std::path::PathBuf::from(std::env::var("NHL14_QA_DUMP").unwrap());
    let bytes = std::fs::read(root.join("gamedata/anim/faceposer.big")).unwrap();
    let header = EbHeader::parse(&bytes).unwrap();
    assert_eq!(header.version, VERSION_TAG_GEN7_FACEPOSER);
    assert_eq!(header.entry_size(), 16);
    let archive = EbArchive::parse(&bytes).unwrap();
    assert_eq!(archive.entries.len(), 841);
    let mut previous_end = header.table_end();
    for (i, entry) in archive.entries.iter().enumerate() {
        let start = entry.offset as usize;
        let end = start.checked_add(entry.size as usize).unwrap();
        assert!(start >= previous_end && end <= bytes.len(), "{} bounds", entry.path);
        previous_end = end;
        let Payload::Data(decoded) = decode_payload(bytes[start..end].to_vec()).unwrap() else { panic!("unsupported {}", entry.path) };
        let row = 48 + i * 16;
        let declared = u32::from_be_bytes(bytes[row+8..row+12].try_into().unwrap()) as usize;
        assert_eq!(decoded.len(), declared, "{}", entry.path);
    }
    assert!(matches!(EbFile::parse(&bytes), Err(EbError::SeparateCompressedSizeWrite)));
    eprintln!("841 faceposer members decoded with bounded, non-overlapping payloads and matching lengths");
}
