//! `big_ls` — list the file table of an EA `.big` archive **without loading the payload**.
//!
//! The BIGF/BIG4 directory lives entirely in `[16, data_offset)`, so this reads only that prefix
//! (a few MB even for a multi-GB archive) and prints one `size\tname` line per entry, plus a summary
//! header. Read-only; never writes. A Phase-5 asset-provenance inventory helper.
//!
//! ```text
//! cargo run -p tdb-archive-big --example big_ls -- <path-to.big> [name-substring-filter]
//! ```

use std::io::{Read, Seek, SeekFrom};

fn be16(b: &[u8], o: usize) -> usize {
    ((b[o] as usize) << 8) | b[o + 1] as usize
}
fn be32(b: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]])
}
fn slot_str(slot: &[u8]) -> String {
    let end = slot.iter().position(|&c| c == 0).unwrap_or(slot.len());
    String::from_utf8_lossy(&slot[..end]).into_owned()
}

/// Version-independent EB directory listing (name + dir tables only). Returns `(size, path)` where
/// `size` is read at the PS4 file-table position — trustworthy for 0x00110400, best-effort otherwise.
fn eb_dir_listing(f: &mut std::fs::File, file_len: u64) -> Vec<(u32, String)> {
    const FILE_TABLE_OFFSET: usize = 0x30;
    const ENTRY_SIZE: usize = 20;
    let mut head = [0u8; 0x20];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_exact(&mut head).unwrap();
    let version = be32(&head, 0x08);
    let file_count = be32(&head, 0x04) as usize;
    let name_table_offset = be32(&head, 0x0C) as usize;
    let name_slot = head[0x14] as usize;
    let dir_slot = head[0x15] as usize;
    let dir_count = be16(&head, 0x16);

    let name_end = name_table_offset + file_count * name_slot;
    let dir_start = (name_end + 15) & !15; // align up 16
    let table_end = dir_start + dir_count * dir_slot;
    let need = table_end.min(file_len as usize);
    let mut buf = vec![0u8; need];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_exact(&mut buf).unwrap();

    let dirs: Vec<String> = (0..dir_count)
        .map(|i| slot_str(&buf[dir_start + i * dir_slot..dir_start + (i + 1) * dir_slot]))
        .collect();

    let mut out = Vec::with_capacity(file_count);
    for i in 0..file_count {
        let nb = name_table_offset + i * name_slot;
        let dir_index = be16(&buf, nb);
        let base = slot_str(&buf[nb + 2..nb + name_slot]);
        let dir = dirs.get(dir_index).map(String::as_str).unwrap_or("<?>");
        let path = if dir.is_empty() {
            base
        } else {
            format!("{dir}/{base}")
        };
        let eb = FILE_TABLE_OFFSET + i * ENTRY_SIZE;
        let size = if eb + 12 <= buf.len() {
            be32(&buf, eb + 8)
        } else {
            0
        };
        out.push((size, path));
    }
    eprintln!(
        "# magic=EB\\0\\3 version=0x{version:08X} entries={file_count} file_len={file_len}{}",
        if version == 0x0011_0400 {
            ""
        } else {
            "  (size UNTRUSTED for this version)"
        }
    );
    out
}

fn main() {
    let mut args = std::env::args().skip(1);
    let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("usage: big_ls <path-to.big> [name-substring-filter]");
            std::process::exit(2);
        }
    };
    let filter = args.next();

    let mut f = std::fs::File::open(&path).unwrap_or_else(|e| {
        eprintln!("open {path}: {e}");
        std::process::exit(1);
    });
    let file_len = f.metadata().map(|m| m.len()).unwrap_or(0);

    let mut magic = [0u8; 4];
    f.read_exact(&mut magic).unwrap();
    f.seek(SeekFrom::Start(0)).unwrap();

    // (size, path) pairs, dispatched on the container magic.
    let entries: Vec<(u32, String)> = if &magic == tdb_archive_big::eb::MAGIC_EB {
        // EB\0\3. The production `EbArchive::parse` only accepts the PS4 version tag (0x00110400); the
        // Xbox 360 NHL Legacy variant (0x00000400) shares the name/dir table layout but NOT the
        // file-table entry layout. For an inventory listing we parse the (version-independent) name/dir
        // tables here directly, so this tool works across variants. `size` is read at the PS4 position
        // and is **untrusted** for non-standard versions (payload reads stay unsupported until the
        // entry layout is reversed) — see the note on `eb::VERSION_TAG`.
        eb_dir_listing(&mut f, file_len)
    } else {
        // Classic BIGF/BIG4: directory lives in [16, data_offset).
        let mut head = [0u8; 16];
        f.read_exact(&mut head).unwrap();
        let data_offset = u32::from_be_bytes([head[12], head[13], head[14], head[15]]) as u64;
        let prefix_len = data_offset.min(file_len).min(256 * 1024 * 1024) as usize;
        let mut prefix = vec![0u8; prefix_len];
        f.seek(SeekFrom::Start(0)).unwrap();
        f.read_exact(&mut prefix).unwrap();
        let arc = tdb_archive_big::Archive::parse(prefix).unwrap_or_else(|e| {
            eprintln!("parse {path}: {e:?}");
            std::process::exit(1);
        });
        eprintln!(
            "# {path}\n# magic={} entries={} data_offset={data_offset} file_len={file_len}",
            arc.magic_str(),
            arc.entries.len()
        );
        arc.iter_entries()
            .map(|e| (e.size, e.name.clone()))
            .collect()
    };

    let mut shown = 0u64;
    for (size, name) in &entries {
        if let Some(sub) = &filter {
            if !name.contains(sub.as_str()) {
                continue;
            }
        }
        println!("{size}\t{name}");
        shown += 1;
    }
    eprintln!("# shown={shown}");
}
