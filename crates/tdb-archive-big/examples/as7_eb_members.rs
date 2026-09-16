//! `as7_eb_members` — package C0-AS7 (D1, D3): record the **member table** of one `EB\0\3`
//! archive through this crate's committed reader, as evidence.
//!
//! Read-only. Writes two repository-relative records for the archive:
//!
//! * `<out>/<label>.json` — specimen attribution, the archive's size and SHA-256, every header
//!   field the reader parses, the reader's provenance (this crate's `src/eb.rs` and this tool,
//!   both hashed), counts, and the stored-form magic histogram;
//! * `<out>/<label>.csv` — one row per file-table entry: index, the **literal** path the name and
//!   directory tables spell, offset, stored size, the two file-table words the reader does not
//!   interpret (`word_4`, `word_12`), the stored bytes' leading magic and SHA-256, and what
//!   [`decode_payload`] makes of the stored bytes (`raw` pass-through, `chunkzip` inflated through
//!   the committed codec, or `chunklzx` reported unsupported) with the decoded size and SHA-256
//!   where a decode exists. With `--loose-root <dir>`, each row also records whether a loose
//!   file exists at `<dir>/<path>` (and at `<dir>/<path>.bak`), with its size and SHA-256, so the
//!   overlap between an archive and a loose tree can be joined by content afterwards.
//!
//! What the record is: a listing of what the archive's own tables say, the stored bytes'
//! identity, and — for the two stored forms the committed codec handles — the identity of what
//! it yields. What it is not: a codec claim beyond the magic bytes observed, an installation
//! rule, a precedence rule between an archive and a loose file, or a runtime binding. Names
//! are literal because this format stores them as NUL-terminated ASCII; no name is inferred.
//!
//! ```sh
//! cargo run -p tdb-archive-big --example as7_eb_members -- \
//!     --archive <path/to/x.big> --label <record-label> --out <dir> \
//!     --specimen <corpus-id> --target <target-id> --relative <path-within-copy> \
//!     --identity "<how the copy was identified>" --source-commit <sha> \
//!     [--loose-root <dir>]
//! ```

use std::collections::BTreeMap;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

use tdb_archive_big::eb::{decode_payload, EbArchive, EbHeader, Payload, HEADER_LEN};
use tdb_archive_big::loose::{read_loose, validate_loose_root};

/// Schema tag of the two records this tool writes.
const RECORD_SCHEMA: &str = "c0-as7-eb-members/1";
const READER_SOURCE: &[u8] = include_bytes!("../src/eb.rs");
const SELF_SOURCE: &[u8] = include_bytes!("as7_eb_members.rs");
const FILE_TABLE_OFFSET: usize = 0x30;

struct Args {
    archive: PathBuf,
    label: String,
    out: PathBuf,
    specimen: String,
    target: String,
    relative: String,
    identity: String,
    source_commit: String,
    loose_root: Option<PathBuf>,
}

fn parse_args() -> Args {
    let mut m: BTreeMap<String, String> = BTreeMap::new();
    let mut it = std::env::args().skip(1);
    while let Some(k) = it.next() {
        let key = k
            .strip_prefix("--")
            .unwrap_or_else(|| panic!("unexpected argument {k}"))
            .to_string();
        let v = it.next().unwrap_or_else(|| panic!("--{key} needs a value"));
        m.insert(key, v);
    }
    let take = |k: &str| m.get(k).cloned().unwrap_or_else(|| panic!("missing --{k}"));
    Args {
        archive: PathBuf::from(take("archive")),
        label: take("label"),
        out: PathBuf::from(take("out")),
        specimen: take("specimen"),
        target: take("target"),
        relative: take("relative"),
        identity: take("identity"),
        source_commit: take("source-commit"),
        loose_root: m.get("loose-root").map(PathBuf::from),
    }
}

fn main() {
    sha256_self_check();
    let a = parse_args();
    // The loose root is validated before any member is looked up (review finding P2): a
    // mistyped or unmounted root fails the run here rather than recording every member as
    // having no loose file.
    if let Some(root) = &a.loose_root {
        validate_loose_root(root).unwrap_or_else(|e| panic!("--loose-root: {e}"));
    }

    // ---- read the front tables through the committed reader
    let mut f = std::fs::File::open(&a.archive).expect("open archive read-only");
    let file_len = f.metadata().unwrap().len();
    let mut head = [0u8; HEADER_LEN];
    f.read_exact(&mut head).unwrap();
    let h = EbHeader::parse(&head).unwrap_or_else(|e| panic!("header: {e}"));
    let mut prefix = vec![0u8; h.table_end()];
    f.seek(SeekFrom::Start(0)).unwrap();
    f.read_exact(&mut prefix).unwrap();
    let arc = EbArchive::parse(&prefix).unwrap_or_else(|e| panic!("tables: {e}"));
    let esz = h.entry_size();

    // ---- whole-file digest, streamed
    f.seek(SeekFrom::Start(0)).unwrap();
    let mut hasher = Sha256::new();
    let mut buf = vec![0u8; 1 << 20];
    loop {
        let n = f.read(&mut buf).unwrap();
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    let archive_sha256 = hex(&hasher.finish());

    // ---- per-entry rows
    let mut csv = String::new();
    csv.push_str(
        "index,path,offset,size,word_4,word_12,stored_magic,stored_sha256,chunk_header,decoded_form,decoded_size,decoded_sha256,decode_error,loose_present,loose_size,loose_sha256,loose_bak_present,loose_bak_size,loose_bak_sha256\n",
    );
    let mut magics: BTreeMap<String, usize> = BTreeMap::new();
    let mut forms: BTreeMap<&'static str, usize> = BTreeMap::new();
    let mut paths: BTreeMap<&str, usize> = BTreeMap::new();
    let mut size_sum: u64 = 0;
    let mut last_end: u64 = 0;
    let mut min_off = u64::MAX;
    let mut unaligned = 0usize;
    let mut beyond_file = 0usize;
    let mut loose_present = 0usize;
    let mut loose_bak_present = 0usize;
    let mut loose_equal = 0usize;
    let mut loose_bak_equal = 0usize;
    for (i, e) in arc.entries.iter().enumerate() {
        let base = FILE_TABLE_OFFSET + i * esz;
        let word_4 = rd_u32(&prefix, base + 4);
        let word_12 = rd_u32(&prefix, base + 12);
        let end = e.offset + u64::from(e.size);
        let row = if end <= file_len {
            let mut stored = vec![0u8; e.size as usize];
            f.seek(SeekFrom::Start(e.offset)).unwrap();
            f.read_exact(&mut stored).unwrap();
            let magic = magic_label(&stored[..usize::min(8, stored.len())]);
            let stored_sha256 = hex(&Sha256::digest(&stored));
            let is_chunkzip = stored.starts_with(b"chunkzip");
            let chunk_header = chunk_header_words(&stored);
            let decoded = match decode_payload(stored) {
                Ok(Payload::Data(bytes)) => Decoded {
                    form: if is_chunkzip { "chunkzip" } else { "raw" },
                    header: chunk_header.clone(),
                    size: bytes.len().to_string(),
                    sha256: hex(&Sha256::digest(&bytes)),
                    error: String::new(),
                },
                Ok(Payload::UnsupportedLzx) => Decoded {
                    form: "chunklzx-unsupported",
                    header: chunk_header.clone(),
                    ..Decoded::default()
                },
                Err(e) => {
                    let msg = e.to_string();
                    assert!(
                        !msg.contains('"'),
                        "decode error text needs escaping: {msg}"
                    );
                    Decoded {
                        form: "decode-error",
                        header: chunk_header.clone(),
                        error: format!("\"{msg}\""),
                        ..Decoded::default()
                    }
                }
            };
            (magic, stored_sha256, decoded)
        } else {
            beyond_file += 1;
            (
                "beyond-file".to_string(),
                String::new(),
                Decoded {
                    form: "beyond-file",
                    ..Decoded::default()
                },
            )
        };
        let (magic, stored_sha256, decoded) = row;
        let form = decoded.form;
        let chunk_header = decoded.header;
        let decoded_size = decoded.size;
        let decoded_sha256 = decoded.sha256;
        let decode_error = decoded.error;
        *magics.entry(magic.clone()).or_default() += 1;
        *forms.entry(form).or_default() += 1;
        *paths.entry(e.path.as_str()).or_default() += 1;
        size_sum += u64::from(e.size);
        last_end = last_end.max(end);
        min_off = min_off.min(e.offset);
        if e.offset % 16 != 0 {
            unaligned += 1;
        }
        assert!(
            !e.path.contains(',') && !e.path.contains('"') && !e.path.contains('\n'),
            "entry {i}: path needs quoting, which this record does not do: {:?}",
            e.path
        );
        let mut loose_cells = String::from(",,,,,,");
        if let Some(root) = &a.loose_root {
            let (present, size, digest) = loose_file(&root.join(&e.path));
            let (bpresent, bsize, bdigest) = loose_file(&root.join(format!("{}.bak", e.path)));
            loose_present += usize::from(present);
            loose_bak_present += usize::from(bpresent);
            loose_equal +=
                usize::from(present && !decoded_sha256.is_empty() && digest == decoded_sha256);
            loose_bak_equal +=
                usize::from(bpresent && !decoded_sha256.is_empty() && bdigest == decoded_sha256);
            loose_cells = format!(",{present},{size},{digest},{bpresent},{bsize},{bdigest}");
        }
        csv.push_str(&format!(
            "{i},{},{},{},0x{word_4:08x},0x{word_12:08x},{magic},{stored_sha256},{chunk_header},{form},{decoded_size},{decoded_sha256},{decode_error}{loose_cells}\n",
            e.path, e.offset, e.size
        ));
    }
    let duplicate_paths: Vec<(&str, usize)> = paths
        .iter()
        .filter(|(_, c)| **c > 1)
        .map(|(p, c)| (*p, *c))
        .collect();
    let mut top_dirs: BTreeMap<String, usize> = BTreeMap::new();
    for e in &arc.entries {
        let top = e.path.split('/').next().unwrap_or("").to_string();
        *top_dirs.entry(top).or_default() += 1;
    }

    // ---- metadata record
    let mut j = String::new();
    j.push_str("{\n");
    j.push_str(&format!("  \"record\": \"{RECORD_SCHEMA}\",\n"));
    j.push_str("  \"package\": \"C0-AS7\",\n");
    j.push_str(&format!("  \"label\": \"{}\",\n", a.label));
    j.push_str("  \"specimen\": {\n");
    j.push_str(&format!("    \"corpus\": \"{}\",\n", jesc(&a.specimen)));
    j.push_str(&format!("    \"target\": \"{}\",\n", jesc(&a.target)));
    j.push_str(&format!(
        "    \"relative_path\": \"{}\",\n",
        jesc(&a.relative)
    ));
    j.push_str(&format!("    \"identity\": \"{}\"\n", jesc(&a.identity)));
    j.push_str("  },\n");
    j.push_str("  \"archive\": {\n");
    j.push_str(&format!("    \"size_bytes\": {file_len},\n"));
    j.push_str(&format!("    \"sha256\": \"{archive_sha256}\",\n"));
    j.push_str("    \"header\": {\n");
    j.push_str("      \"magic\": \"EB\\\\0\\\\3\",\n");
    j.push_str(&format!("      \"version\": \"0x{:08x}\",\n", h.version));
    j.push_str(&format!("      \"entry_size\": {esz},\n"));
    j.push_str(&format!("      \"file_count\": {},\n", h.file_count));
    j.push_str(&format!(
        "      \"name_table_offset\": {},\n",
        h.name_table_offset
    ));
    j.push_str(&format!("      \"name_slot\": {},\n", h.name_slot));
    j.push_str(&format!("      \"dir_slot\": {},\n", h.dir_slot));
    j.push_str(&format!("      \"dir_count\": {},\n", h.dir_count));
    j.push_str(&format!("      \"archive_size\": {},\n", h.archive_size));
    j.push_str(&format!("      \"table_end\": {}\n", h.table_end()));
    j.push_str("    }\n  },\n");
    j.push_str("  \"reader\": {\n");
    j.push_str("    \"crate\": \"tdb-archive-big\",\n");
    j.push_str("    \"module\": \"crates/tdb-archive-big/src/eb.rs\",\n");
    j.push_str(&format!(
        "    \"module_sha256\": \"{}\",\n",
        hex(&Sha256::digest(READER_SOURCE))
    ));
    j.push_str("    \"tool\": \"crates/tdb-archive-big/examples/as7_eb_members.rs\",\n");
    j.push_str(&format!(
        "    \"tool_sha256\": \"{}\",\n",
        hex(&Sha256::digest(SELF_SOURCE))
    ));
    j.push_str(&format!(
        "    \"source_commit\": \"{}\"\n",
        jesc(&a.source_commit)
    ));
    j.push_str("  },\n");
    j.push_str("  \"counts\": {\n");
    j.push_str(&format!("    \"entries\": {},\n", arc.entries.len()));
    j.push_str(&format!("    \"distinct_paths\": {},\n", paths.len()));
    j.push_str(&format!(
        "    \"duplicate_paths\": {},\n",
        duplicate_paths.len()
    ));
    j.push_str(&format!("    \"stored_size_sum\": {size_sum},\n"));
    j.push_str(&format!("    \"first_payload_offset\": {min_off},\n"));
    j.push_str(&format!("    \"last_payload_end\": {last_end},\n"));
    j.push_str(&format!(
        "    \"last_end_equals_archive_size\": {},\n",
        last_end == h.archive_size
    ));
    j.push_str(&format!(
        "    \"archive_size_equals_file_size\": {},\n",
        h.archive_size == file_len
    ));
    j.push_str(&format!("    \"unaligned_offsets\": {unaligned},\n"));
    j.push_str(&format!("    \"entries_beyond_file\": {beyond_file},\n"));
    j.push_str("    \"stored_magic\": {\n");
    let n = magics.len();
    for (k, (m, c)) in magics.iter().enumerate() {
        j.push_str(&format!(
            "      \"{}\": {c}{}\n",
            jesc(m),
            if k + 1 < n { "," } else { "" }
        ));
    }
    j.push_str("    },\n");
    j.push_str("    \"decoded_form\": {\n");
    let n = forms.len();
    for (k, (m, c)) in forms.iter().enumerate() {
        j.push_str(&format!(
            "      \"{m}\": {c}{}\n",
            if k + 1 < n { "," } else { "" }
        ));
    }
    j.push_str("    },\n");
    j.push_str("    \"top_level_directories\": {\n");
    let n = top_dirs.len();
    for (k, (d, c)) in top_dirs.iter().enumerate() {
        j.push_str(&format!(
            "      \"{}\": {c}{}\n",
            jesc(d),
            if k + 1 < n { "," } else { "" }
        ));
    }
    j.push_str("    }\n  },\n");
    j.push_str("  \"loose_root\": ");
    match &a.loose_root {
        None => j.push_str("null,\n"),
        Some(_) => j.push_str(&format!(
            "{{\n    \"present\": true,\n    \"members_with_loose_file\": {loose_present},\n    \"members_with_loose_bak_file\": {loose_bak_present},\n    \"loose_file_equals_decoded\": {loose_equal},\n    \"loose_bak_file_equals_decoded\": {loose_bak_equal}\n  }},\n"
        )),
    }
    j.push_str(&format!("  \"members_csv\": \"{}.csv\",\n", jesc(&a.label)));
    j.push_str(&format!(
        "  \"members_csv_sha256\": \"{}\",\n",
        hex(&Sha256::digest(csv.as_bytes()))
    ));
    j.push_str("  \"limitations\": [\n");
    j.push_str("    \"A member table read through the reader's name and directory tables. Names are the literal NUL-terminated ASCII strings those tables hold; nothing is inferred or hashed.\",\n");
    j.push_str("    \"`stored_magic` is the first eight bytes at each entry's stored offset, labelled by the magics the reader documents. It is an observation of the stored form, not a decode, and not a codec claim for the bytes behind the magic.\",\n");
    j.push_str("    \"`decoded_form` is what `eb::decode_payload` did with the stored bytes: `raw` passed them through, `chunkzip` inflated them through the committed codec, `chunklzx-unsupported` means the codec is not implemented and no decoded identity exists. A successful inflate is an observation about these bytes and this codec; it establishes no target-wide codec.\",\n");
    j.push_str("    \"Loose-file cells compare a decoded member with whatever sits at the same relative path (and its `.bak` sibling) under the loose root at read time. `loose_present = false` means the OS reported NotFound; the root was validated as a readable directory first, and any other read failure aborts the run before a record is written. Equality is a content observation; it says nothing about which of the two the game loads.\",\n");
    j.push_str("    \"`word_4` and `word_12` are the file-table words the reader does not interpret (the latter documented as an unidentified per-entry checksum). Recorded verbatim.\",\n");
    j.push_str("    \"`decode_error` is the committed codec's own error text where `decoded_form` is `decode-error`; it records that THIS codec did not inflate THESE bytes and nothing about why the bytes are as they are.\",\n");
    j.push_str("    \"Nothing was written or extracted; decoded bytes were hashed in memory and dropped. The record establishes no installation rule, runtime binding or precedence between archives or between an archive and a loose file.\",\n");
    j.push_str("    \"The copy's identity is the one the specimen field cites; a title id plus an archive version tag is not a build.\"\n");
    j.push_str("  ]\n}\n");

    std::fs::create_dir_all(&a.out).unwrap();
    let csv_path = a.out.join(format!("{}.csv", a.label));
    let json_path = a.out.join(format!("{}.json", a.label));
    std::fs::File::create(&csv_path)
        .unwrap()
        .write_all(csv.as_bytes())
        .unwrap();
    std::fs::File::create(&json_path)
        .unwrap()
        .write_all(j.as_bytes())
        .unwrap();
    println!(
        "{}: {} entries, {} distinct paths, sha256 {archive_sha256}\n  -> {}\n  -> {}",
        a.label,
        arc.entries.len(),
        paths.len(),
        csv_path.display(),
        json_path.display()
    );
}

/// Label the first stored bytes by the magics `eb::decode_payload` and the archive layout notes
/// name; anything else is reported as hex so the record never guesses.
fn magic_label(m: &[u8]) -> String {
    if m.len() >= 8 {
        match &m[..8] {
            b"chunkzip" => return "chunkzip".into(),
            b"chunklzx" => return "chunklzx".into(),
            _ => {}
        }
    }
    if m.len() >= 4 {
        match &m[..4] {
            b"EB\x00\x03" => return "EB\\0\\3".into(),
            b"BIGF" => return "BIGF".into(),
            b"GD.S" if m.len() >= 7 && &m[..7] == b"GD.STRM" => return "GD.STRM".into(),
            _ => {}
        }
    }
    let ascii = m.iter().all(|b| b.is_ascii_graphic());
    if ascii && !m.is_empty() {
        format!("ascii:{}", String::from_utf8_lossy(m))
    } else {
        format!("hex:{}", hex(m))
    }
}

/// (present, size, sha256) of a loose file, read-only. Absent means `NotFound` and nothing
/// else: any other read failure aborts the run with the path before a record is written, so a
/// permission error or an I/O fault can never be published as absence.
fn loose_file(path: &std::path::Path) -> (bool, String, String) {
    match read_loose(path).unwrap_or_else(|e| panic!("loose file: {e}")) {
        Some(bytes) => (true, bytes.len().to_string(), hex(&Sha256::digest(&bytes))),
        None => (false, String::new(), String::new()),
    }
}

/// What `decode_payload` yielded for one entry's stored bytes.
#[derive(Default)]
struct Decoded {
    form: &'static str,
    /// The chunk-container header words, for `chunkzip` / `chunklzx` stored forms only.
    header: String,
    size: String,
    sha256: String,
    error: String,
}

/// The five big-endian words after an eight-byte chunk magic, spelled the way the codec crate
/// documents them (`dummy` at +8; for `dummy <= 2`: `fullsize` +12, `chunk_size` +16, `n_chunks`
/// +20, and the word at +24), as `dummy=..;w12=..;w16=..;w20=..;w24=..`. Empty for other magics.
/// Recorded verbatim so a decode failure can be read against the header the codec saw.
fn chunk_header_words(stored: &[u8]) -> String {
    if stored.len() < 28 || !(stored.starts_with(b"chunkzip") || stored.starts_with(b"chunklzx")) {
        return String::new();
    }
    format!(
        "dummy={};w12={};w16={};w20={};w24={}",
        rd_u32(stored, 8),
        rd_u32(stored, 12),
        rd_u32(stored, 16),
        rd_u32(stored, 20),
        rd_u32(stored, 24)
    )
}

fn rd_u32(d: &[u8], o: usize) -> u32 {
    u32::from_be_bytes([d[o], d[o + 1], d[o + 2], d[o + 3]])
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn jesc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

// ------------------------------------------------------------------------------ SHA-256
//
// This crate has no digest dependency and package C0-AS7 does not change the workspace manifest or
// lock, so the digest is computed here (FIPS 180-4). `sha256_self_check` runs three published
// test vectors before any record is written and aborts on mismatch; the receipt additionally
// cross-checks every recorded archive digest against an independent implementation.

struct Sha256 {
    state: [u32; 8],
    buf: Vec<u8>,
    len: u64,
}

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

impl Sha256 {
    fn new() -> Self {
        Self {
            state: [
                0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
                0x5be0cd19,
            ],
            buf: Vec::with_capacity(64),
            len: 0,
        }
    }

    fn digest(bytes: &[u8]) -> [u8; 32] {
        let mut h = Self::new();
        h.update(bytes);
        h.finish()
    }

    fn update(&mut self, mut data: &[u8]) {
        self.len += data.len() as u64;
        if !self.buf.is_empty() {
            let need = 64 - self.buf.len();
            let take = need.min(data.len());
            self.buf.extend_from_slice(&data[..take]);
            data = &data[take..];
            if self.buf.len() == 64 {
                let block: [u8; 64] = self.buf[..].try_into().unwrap();
                self.compress(&block);
                self.buf.clear();
            }
        }
        while data.len() >= 64 {
            let block: [u8; 64] = data[..64].try_into().unwrap();
            self.compress(&block);
            data = &data[64..];
        }
        self.buf.extend_from_slice(data);
    }

    fn finish(mut self) -> [u8; 32] {
        let bit_len = self.len * 8;
        let mut pad = vec![0x80u8];
        while (self.buf.len() + pad.len()) % 64 != 56 {
            pad.push(0);
        }
        pad.extend_from_slice(&bit_len.to_be_bytes());
        // Padding is not message: feed it through the block path without touching `len`.
        let mut tail = std::mem::take(&mut self.buf);
        tail.extend_from_slice(&pad);
        debug_assert_eq!(tail.len() % 64, 0);
        for block in tail.chunks_exact(64) {
            let block: [u8; 64] = block.try_into().unwrap();
            self.compress(&block);
        }
        let mut out = [0u8; 32];
        for (i, w) in self.state.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&w.to_be_bytes());
        }
        out
    }

    fn compress(&mut self, block: &[u8; 64]) {
        let mut w = [0u32; 64];
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                block[i * 4],
                block[i * 4 + 1],
                block[i * 4 + 2],
                block[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let [mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut h] = self.state;
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ (!e & g);
            let t1 = h
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            h = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        for (s, v) in self.state.iter_mut().zip([a, b, c, d, e, f, g, h]) {
            *s = s.wrapping_add(v);
        }
    }
}

/// FIPS 180-4 / NIST CAVP known answers; abort before writing anything if any fails.
fn sha256_self_check() {
    let cases: [(&[u8], &str); 3] = [
        (
            b"",
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        ),
        (
            b"abc",
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
        ),
        (
            b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq",
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1",
        ),
    ];
    for (input, want) in cases {
        let got = hex(&Sha256::digest(input));
        assert_eq!(got, want, "SHA-256 self-check failed");
        // Also exercise the streaming path with an odd chunking.
        let mut h = Sha256::new();
        for chunk in input.chunks(5) {
            h.update(chunk);
        }
        assert_eq!(
            hex(&h.finish()),
            want,
            "SHA-256 streaming self-check failed"
        );
    }
}
