//! `eb_find` — locate every entry whose internal path contains a substring, across one or
//! more `EB\0\3` archives, and report the **raw file-table row** for each hit.
//!
//! Unlike [`eb_extract`], which stops at the first substring match in one archive, this
//! enumerates *all* matches in *all* named archives so archive precedence can be resolved
//! from evidence rather than guessed. Strictly read-only: it seeks and reads, never writes
//! to the source. Payload bytes are streamed one entry at a time.
//!
//! ```text
//! cargo run -p tdb-archive-big --example eb_find -- <needle> <archive.big>...
//! cargo run -p tdb-archive-big --example eb_find -- --dump <dir> <needle> <archive.big>...
//! ```
//!
//! With `--dump <dir>` each hit's decoded payload is written to
//! `<dir>/<archive-stem>#<index>_<basename>` so the caller can hash and compare them.
//! Output is one tab-separated row per hit, with a `#`-prefixed header.
//!
//! **Exit status is evidence.** A run exits 0 only when every named archive opened and
//! parsed *and* every matching entry decoded. Any unreadable archive, unparseable table,
//! or entry that came back `chunklzx`/errored makes the final summary say so and the
//! process exit 1 — a clean-looking manifest with a silently skipped entry is exactly the
//! failure this probe exists to prevent.

use std::io::{Read, Seek, SeekFrom};

use tdb_archive_big::eb::{decode_payload, EbArchive, EbHeader, Payload, HEADER_LEN};

/// File table start and PS4 row stride — mirrored from `eb.rs`, which keeps them private
/// because its public [`EbArchive`] model exposes only the fields it interprets. This probe
/// deliberately reports the *uninterpreted* words too.
const FILE_TABLE_OFFSET: usize = 0x30;

fn be32(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

fn main() {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    let mut dump: Option<String> = None;
    if args.first().map(String::as_str) == Some("--dump") {
        args.remove(0);
        if args.is_empty() {
            eprintln!("--dump needs a directory");
            std::process::exit(2);
        }
        dump = Some(args.remove(0));
    }
    if args.len() < 2 {
        eprintln!("usage: eb_find [--dump <dir>] <path-substring> <archive.big>...");
        std::process::exit(2);
    }
    let needle = args.remove(0);

    println!("# archive\tindex\tpath\tblock\tword1\tsize\tcrc\tword4\toffset\tstored_magic\tdecoded_len\tdumped");
    let mut hits = 0usize;
    let mut undecoded: Vec<String> = Vec::new();
    let mut unscanned: Vec<String> = Vec::new();
    for path in &args {
        match scan(path, &needle, dump.as_deref(), &mut undecoded) {
            Ok(n) => hits += n,
            Err(e) => unscanned.push(format!("{path}: {e}")),
        }
    }
    eprintln!(
        "# {hits} hit(s) for {needle:?} across {} archive(s); {} not scanned, {} entr(ies) not decoded",
        args.len(),
        unscanned.len(),
        undecoded.len()
    );
    for u in &unscanned {
        eprintln!("# NOT SCANNED: {u}");
    }
    for u in &undecoded {
        eprintln!("# NOT DECODED: {u}");
    }
    if unscanned.is_empty() && undecoded.is_empty() {
        eprintln!("# OK: every named archive parsed and every matching entry decoded");
    } else {
        eprintln!("# INCOMPLETE: this manifest does not cover every entry");
        std::process::exit(1);
    }
}

fn scan(
    path: &str,
    needle: &str,
    dump: Option<&str>,
    undecoded: &mut Vec<String>,
) -> std::io::Result<usize> {
    let mut f = std::fs::File::open(path)?;
    let mut head = [0u8; HEADER_LEN];
    f.read_exact(&mut head)?;
    let header = match EbHeader::parse(&head) {
        Ok(h) => h,
        Err(e) => {
            undecoded.push(format!("{path}: header: {e}"));
            return Ok(0);
        }
    };
    let mut prefix = vec![0u8; header.table_end()];
    f.seek(SeekFrom::Start(0))?;
    f.read_exact(&mut prefix)?;
    let arc = match EbArchive::parse(&prefix) {
        Ok(a) => a,
        Err(e) => {
            undecoded.push(format!("{path}: tables: {e}"));
            return Ok(0);
        }
    };
    eprintln!(
        "# {path}: version=0x{:08X} entries={} entry_size={} archive_size={}",
        header.version,
        arc.entries.len(),
        header.entry_size(),
        header.archive_size
    );

    let stem = std::path::Path::new(path)
        .file_stem()
        .map_or_else(|| path.to_string(), |s| s.to_string_lossy().into_owned());

    let mut hits = 0;
    for (i, entry) in arc.entries.iter().enumerate() {
        if !entry.path.contains(needle) {
            continue;
        }
        hits += 1;
        let base = FILE_TABLE_OFFSET + i * header.entry_size();
        let row = &prefix[base..base + header.entry_size()];
        let (block, w1, size, crc) = (be32(row, 0), be32(row, 4), be32(row, 8), be32(row, 12));
        let w4 = if header.entry_size() >= 20 {
            format!("{:08X}", be32(row, 16))
        } else {
            "-".to_string()
        };

        let mut raw = vec![0u8; entry.size as usize];
        f.seek(SeekFrom::Start(entry.offset))?;
        f.read_exact(&mut raw)?;
        let magic: String = raw
            .iter()
            .take(8)
            .map(|&b| if b.is_ascii_graphic() { b as char } else { '.' })
            .collect();

        let (decoded_len, dumped) = match decode_payload(raw) {
            Ok(Payload::Data(bytes)) => {
                let n = bytes.len();
                let mut where_ = "-".to_string();
                if let Some(dir) = dump {
                    let name = entry.path.rsplit('/').next().unwrap_or("entry");
                    let out = format!("{dir}/{stem}#{i}_{name}");
                    std::fs::write(&out, &bytes)?;
                    where_ = out;
                }
                (n.to_string(), where_)
            }
            Ok(Payload::UnsupportedLzx) => {
                undecoded.push(format!(
                    "{stem}#{i} {}: chunklzx (codec unimplemented)",
                    entry.path
                ));
                ("chunklzx".to_string(), "-".to_string())
            }
            Err(e) => {
                undecoded.push(format!("{stem}#{i} {}: {e}", entry.path));
                (format!("error:{e}"), "-".to_string())
            }
        };

        println!(
            "{stem}\t{i}\t{}\t{block:08X}\t{w1:08X}\t{size}\t{crc:08X}\t{w4}\t{}\t{magic:?}\t{decoded_len}\t{dumped}",
            entry.path, entry.offset
        );
    }
    Ok(hits)
}
