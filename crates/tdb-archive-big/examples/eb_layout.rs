//! `eb_layout` — dump the exact byte layout of an `EB\0\3` archive so a writer can reproduce it
//! byte-for-byte: every header field, every raw file-table entry (`block`, reserved `+4`, `size`,
//! `crc/+12`, and the trailing `+16` word on 20-byte variants), the name/dir slot widths, and the
//! **gaps** between consecutive payload blocks (to see EA's inter-block padding + any dedup).
//! Read-only. `cargo run -p tdb-archive-big --example eb_layout -- <archive.big>`

use std::io::{Read, Seek, SeekFrom};

use tdb_archive_big::eb::{EbArchive, EbHeader};

fn rd_u32(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: eb_layout <archive.big>");
    let mut f = std::fs::File::open(&path).unwrap();
    let flen = f.metadata().unwrap().len();

    let mut head = [0u8; tdb_archive_big::eb::HEADER_LEN];
    f.read_exact(&mut head).unwrap();
    let h = EbHeader::parse(&head).unwrap();
    let esz = h.entry_size();
    println!(
        "# {path}\n# file_len={flen} version=0x{:08X} entry_size={esz} file_count={} dir_count={}",
        h.version, h.file_count, h.dir_count
    );
    println!(
        "# name_table_offset=0x{:X} name_slot={} dir_slot={} archive_size={} table_end=0x{:X}",
        h.name_table_offset, h.name_slot, h.dir_slot, h.archive_size, h.table_end()
    );
    // Raw header bytes for the fields the parser ignores.
    print!("# header[0x00..0x20]:");
    for b in &head {
        print!(" {b:02X}");
    }
    println!();

    // Read the whole table prefix to inspect raw entries.
    let mut prefix = vec![0u8; h.table_end()];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_exact(&mut prefix).unwrap();
    let arc = EbArchive::parse(&prefix).unwrap();

    // Raw file-table entries.
    println!("# --- file table (raw) ---");
    let ft = 0x30usize;
    let mut ends: Vec<(u64, u64, usize)> = Vec::new(); // (offset, end, index)
    for i in 0..h.file_count {
        let e = ft + i * esz;
        let block = rd_u32(&prefix, e);
        let resv = rd_u32(&prefix, e + 4);
        let size = rd_u32(&prefix, e + 8);
        let crc = rd_u32(&prefix, e + 12);
        let tail = if esz >= 20 { rd_u32(&prefix, e + 16) } else { 0 };
        let off = (block as u64) << 4;
        println!(
            "  [{i}] block=0x{block:08X} off=0x{off:X} resv=0x{resv:08X} size={size} crc=0x{crc:08X} tail=0x{tail:08X}  {}",
            arc.entries[i].path
        );
        ends.push((off, off + size as u64, i));
    }

    // Data-region gaps between consecutive blocks (in offset order) → padding/dedup.
    println!("# --- data blocks in offset order (gaps show padding/dedup) ---");
    let mut sorted = ends.clone();
    sorted.sort_by_key(|&(o, _, _)| o);
    let data_start = tdb_archive_big::eb::HEADER_LEN; // informational
    let _ = data_start;
    for w in sorted.windows(2) {
        let (o0, end0, i0) = w[0];
        let (o1, _end1, i1) = w[1];
        let gap = o1 as i64 - end0 as i64;
        println!(
            "  [{i0}] off=0x{o0:X} end=0x{end0:X}  -> [{i1}] off=0x{o1:X}   gap={gap} (end%16={})",
            end0 % 16
        );
    }
    if let Some(&(o, end, i)) = sorted.last() {
        println!(
            "  [last {i}] off=0x{o:X} end=0x{end:X}  archive_size={}  tail_gap={}",
            h.archive_size,
            h.archive_size as i64 - end as i64
        );
    }
    // Where does the first data block start relative to table_end?
    if let Some(&(o, _, i)) = sorted.first() {
        println!(
            "# first block [{i}] off=0x{o:X}; table_end=0x{:X}; pad_before_data={}",
            h.table_end(),
            o as i64 - h.table_end() as i64
        );
    }
}
