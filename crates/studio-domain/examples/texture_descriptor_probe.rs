//! Read-only RX2 descriptor evidence for appearance-export qualification.
fn main() {
    for path in std::env::args().skip(1) {
        let mut bytes = std::fs::read(&path).unwrap();
        if tdb_archive_codec::is_refpack(&bytes) {
            bytes = tdb_archive_codec::refpack_decompress(&bytes).unwrap();
        }
        assert!(bytes.starts_with(tdb_rx2::texlib::MAGIC));
        let word = |off| u32::from_be_bytes(bytes[off..off + 4].try_into().unwrap()) as usize;
        println!(
            "{path}: length={} header={} pixels={}",
            bytes.len(),
            word(0x44),
            word(0x54)
        );
        for i in 0..word(0x20) {
            let entry = word(0x30) + i * 24;
            let offset = word(entry);
            let size = word(entry + 8);
            let tag = word(entry + 20);
            if tag == 0x20003 {
                println!("  descriptor index={i} offset={offset} size={size} format={:02x} fetch={:08x} dimensions={:08x}",
                    bytes[offset + 35], word(offset + 28), word(offset + 36));
            } else if tag == 0x10031 {
                println!("  pixels index={i} offset={offset} size={size}");
            }
        }
    }
}
