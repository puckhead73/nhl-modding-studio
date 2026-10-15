//! `eb_extract` — extract one entry from an `EB\0\3` archive through the **production**
//! `eb::EbArchive` API (header → `table_end()` → parse → seek `offset`, read `size`, `decode_payload`).
//! Prints the decoded payload's leading magic; optionally writes it out. Read-only on the source.
//!
//! ```text
//! cargo run -p tdb-archive-big --example eb_extract -- <archive.big> <path-substring> [out-file]
//! ```

use std::io::{Read, Seek, SeekFrom};

use tdb_archive_big::eb::{decode_payload, EbArchive, EbHeader, Payload};

fn main() {
    let mut args = std::env::args().skip(1);
    let (path, needle, out) = (args.next(), args.next(), args.next());
    let (Some(path), Some(needle)) = (path, needle) else {
        eprintln!("usage: eb_extract <archive.big> <path-substring> [out-file]");
        std::process::exit(2);
    };

    let mut f = std::fs::File::open(&path).unwrap_or_else(|e| {
        eprintln!("open {path}: {e}");
        std::process::exit(1);
    });

    let mut head = [0u8; tdb_archive_big::eb::HEADER_LEN];
    f.read_exact(&mut head).unwrap();
    let header = EbHeader::parse(&head).unwrap_or_else(|e| {
        eprintln!("header {path}: {e}");
        std::process::exit(1);
    });
    eprintln!(
        "# {path}: version=0x{:08X} entry_size={}",
        header.version,
        header.entry_size()
    );

    let mut prefix = vec![0u8; header.table_end()];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_exact(&mut prefix).unwrap();
    let arc = EbArchive::parse(&prefix).unwrap();

    let entry = arc
        .entries
        .iter()
        .find(|e| e.path.contains(&needle))
        .unwrap_or_else(|| {
            eprintln!("no entry matching {needle:?}");
            std::process::exit(1);
        });
    eprintln!(
        "# entry: {}  offset=0x{:X} size={}",
        entry.path, entry.offset, entry.size
    );

    let mut raw = vec![0u8; entry.size as usize];
    f.seek(SeekFrom::Start(entry.offset)).unwrap();
    f.read_exact(&mut raw).unwrap();

    let magic: String = raw
        .iter()
        .take(8)
        .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
        .collect();
    eprintln!("# stored magic: {magic:?}  ({} bytes)", raw.len());

    match decode_payload(raw) {
        Ok(Payload::Data(bytes)) => {
            let dm: String = bytes
                .iter()
                .take(8)
                .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
                .collect();
            eprintln!("# decoded: {} bytes, leading {dm:?}", bytes.len());
            if let Some(out) = out {
                std::fs::write(&out, &bytes).unwrap();
                eprintln!("# wrote {out}");
            }
        }
        Ok(Payload::UnsupportedLzx) => {
            eprintln!("# payload is chunklzx (LZX) — decoder not implemented")
        }
        Err(e) => eprintln!("# decode failed: {e}"),
    }
}
