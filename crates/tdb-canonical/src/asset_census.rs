//! Package C0-AS7 — the **record and join logic** for the file side of asset discovery and
//! identity: a per-file portrait census (D4), the member tables of `EB\0\3` archives (D1, D3) and
//! the joins of both to the ids a committed anchor stores.
//!
//! Everything here is pure: bytes and strings in, records and summaries out. The walking, hashing
//! and archive reading happen in two examples (`tdb-canonical/examples/as7_portrait_census.rs`,
//! `tdb-archive-big/examples/as7_eb_members.rs`) that run only against a private copy and write
//! the records under `docs/receipts/data/`; the tests in `asset_census_tests.rs` exercise this
//! module synthetically **and** re-derive every join from the committed records plus the
//! committed anchor, so a recorded number is a number this code recomputes on every run.
//!
//! What a join establishes: that an id stored in the anchor's field is, or is not, spelled by a
//! file name in the census or member table of **that specimen**. It does not establish that the
//! game looks the id up that way, that either copy is pristine, or an identity invariant for the
//! target; the receipt says so wherever a number is quoted.

use std::collections::{BTreeMap, BTreeSet};

/// Schema tag of the portrait census record (`docs/receipts/data/c0-as7-ps3-portrait-census.csv`).
pub const PORTRAIT_CENSUS_SCHEMA: &str = "c0-as7-portrait-census/1";
/// Schema tag of the archive member records written by `as7_eb_members`.
pub const EB_MEMBERS_SCHEMA: &str = "c0-as7-eb-members/1";

/// The header line of a portrait census record. Column order is the record's contract.
pub const PORTRAIT_CENSUS_HEADER: &str = "folder,bucket,file_name,label,numeric_id,size,sha256,bigf_members,texture_member,texture_stored_size,texture_declared_size";

/// The header line of an archive member record (`as7_eb_members`). Column order is the contract.
pub const EB_MEMBERS_HEADER: &str = "index,path,offset,size,word_4,word_12,stored_magic,stored_sha256,chunk_header,decoded_form,decoded_size,decoded_sha256,decode_error,loose_present,loose_size,loose_sha256,loose_bak_present,loose_bak_size,loose_bak_sha256";

/// Split one CSV line whose quoted cells contain no quotes or newlines (the only quoted cell in
/// these records is the codec's error text).
pub fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = line;
    loop {
        if let Some(stripped) = rest.strip_prefix('"') {
            let end = stripped.find('"').expect("closing quote");
            out.push(stripped[..end].to_string());
            rest = &stripped[end + 1..];
            match rest.strip_prefix(',') {
                Some(r) => rest = r,
                None => break,
            }
        } else {
            match rest.find(',') {
                Some(i) => {
                    out.push(rest[..i].to_string());
                    rest = &rest[i + 1..];
                }
                None => {
                    out.push(rest.to_string());
                    break;
                }
            }
        }
    }
    out
}

/// Sorted ids rendered as `a-b;c;d-e` — compact, deterministic, and readable in a record cell.
pub fn ranges(ids: &BTreeSet<u64>) -> String {
    let mut out = String::new();
    let mut run: Option<(u64, u64)> = None;
    let flush = |run: &mut Option<(u64, u64)>, out: &mut String| {
        if let Some((a, b)) = run.take() {
            if !out.is_empty() {
                out.push(';');
            }
            if a == b {
                out.push_str(&a.to_string());
            } else {
                out.push_str(&format!("{a}-{b}"));
            }
        }
    };
    for &id in ids {
        match run {
            Some((a, b)) if id == b + 1 => run = Some((a, id)),
            Some(_) => {
                flush(&mut run, &mut out);
                run = Some((id, id));
            }
            None => run = Some((id, id)),
        }
    }
    flush(&mut run, &mut out);
    out
}

// ================================================================ D4: the portrait census

/// The file-name group a census file falls into — C0-POR1-R1's neutral vocabulary.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortraitLabel {
    /// `p<digits>.big`
    Current,
    /// `p<digits>.big.bak` — the label is the file name's; nothing here says what the content is.
    BackupLabelled,
    /// Anything else in the folder.
    Other,
}

impl PortraitLabel {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Current => "current",
            Self::BackupLabelled => "backup-labelled",
            Self::Other => "other",
        }
    }
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "current" => Some(Self::Current),
            "backup-labelled" => Some(Self::BackupLabelled),
            "other" => Some(Self::Other),
            _ => None,
        }
    }
}

/// Classify a file name inside a portrait bucket: the label and, for the two `p<digits>` forms,
/// the numeric id the name spells. The id is read from the name only.
pub fn classify_portrait_name(file_name: &str) -> (PortraitLabel, Option<u64>) {
    let digits = |stem: &str| -> Option<u64> {
        let d = stem.strip_prefix('p')?;
        if d.is_empty() || !d.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        d.parse().ok()
    };
    if let Some(stem) = file_name.strip_suffix(".big.bak") {
        if let Some(id) = digits(stem) {
            return (PortraitLabel::BackupLabelled, Some(id));
        }
    } else if let Some(stem) = file_name.strip_suffix(".big") {
        if let Some(id) = digits(stem) {
            return (PortraitLabel::Current, Some(id));
        }
    }
    (PortraitLabel::Other, None)
}

/// What a shallow look at a `BIGF` portrait archive's own tables shows: the member count, the
/// name of the member at index 3 (the texture payload position C0-POR1 measured on every
/// archive), its stored length, and the decompressed length its RefPack header declares. Nothing
/// is decompressed; a file that is not `BIGF`, has fewer than four members, or whose fourth
/// member is not RefPack yields `None` fields rather than a guess.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BigfPeek {
    pub members: Option<usize>,
    pub texture_member: Option<String>,
    pub texture_stored_size: Option<u32>,
    pub texture_declared_size: Option<u32>,
}

/// Read the `BIGF` member table (magic; size LE at 4; count BE at 8; header size BE at 12; then
/// `offset:u32 BE, length:u32 BE, name\0` per member — the layout C0-POR1 measured on all 37 120
/// Gen7 portrait archives) and the RefPack size field of member 3.
pub fn peek_bigf_portrait(bytes: &[u8]) -> BigfPeek {
    let mut peek = BigfPeek::default();
    if bytes.len() < 16 || &bytes[..4] != b"BIGF" {
        return peek;
    }
    let count = u32::from_be_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    peek.members = Some(count);
    let mut pos = 16usize;
    let mut members: Vec<(u32, u32, String)> = Vec::with_capacity(count);
    for _ in 0..count {
        if pos + 8 > bytes.len() {
            return peek;
        }
        let off = u32::from_be_bytes([bytes[pos], bytes[pos + 1], bytes[pos + 2], bytes[pos + 3]]);
        let len = u32::from_be_bytes([
            bytes[pos + 4],
            bytes[pos + 5],
            bytes[pos + 6],
            bytes[pos + 7],
        ]);
        pos += 8;
        let Some(nul) = bytes[pos..].iter().position(|&b| b == 0) else {
            return peek;
        };
        let name = String::from_utf8_lossy(&bytes[pos..pos + nul]).into_owned();
        pos += nul + 1;
        members.push((off, len, name));
    }
    let Some((off, len, name)) = members.get(3) else {
        return peek;
    };
    peek.texture_member = Some(name.clone());
    peek.texture_stored_size = Some(*len);
    let start = *off as usize;
    if let Some(declared) = refpack_declared_size(bytes.get(start..).unwrap_or(&[])) {
        peek.texture_declared_size = Some(declared);
    }
    peek
}

/// The decompressed length a RefPack header declares (byte 0 flags: bit 7 = 4-byte sizes, bit 6 =
/// a compressed-size field precedes it; byte 1 = `0xFB`), per `tdb_archive_codec::refpack`.
pub fn refpack_declared_size(bytes: &[u8]) -> Option<u32> {
    if bytes.len() < 2 || bytes[1] != 0xFB || bytes[0] & 0x3E != 0x10 {
        return None;
    }
    let wide = bytes[0] & 0x80 != 0;
    let has_compressed = bytes[0] & 0x40 != 0;
    let width = if wide { 4 } else { 3 };
    let mut pos = 2;
    if has_compressed {
        pos += width;
    }
    let field = bytes.get(pos..pos + width)?;
    Some(field.iter().fold(0u32, |acc, &b| (acc << 8) | u32::from(b)))
}

/// One file of the portrait census.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortraitFile {
    pub folder: String,
    pub bucket: String,
    pub file_name: String,
    pub label: PortraitLabel,
    pub numeric_id: Option<u64>,
    pub size: u64,
    pub sha256: String,
    pub peek: BigfPeek,
}

impl PortraitFile {
    pub fn relative_path(&self) -> String {
        format!(
            "fe/ion/artassets/{}/{}/{}",
            self.folder, self.bucket, self.file_name
        )
    }
}

/// Render the census as the record's CSV (header + one row per file, in the given order).
pub fn render_portrait_census_csv(files: &[PortraitFile]) -> String {
    let mut out = String::from(PORTRAIT_CENSUS_HEADER);
    out.push('\n');
    let opt = |v: Option<u32>| v.map(|v| v.to_string()).unwrap_or_default();
    for f in files {
        assert!(
            !f.file_name.contains(',') && !f.file_name.contains('"'),
            "file name needs quoting, which this record does not do: {:?}",
            f.file_name
        );
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{}\n",
            f.folder,
            f.bucket,
            f.file_name,
            f.label.as_str(),
            f.numeric_id.map(|v| v.to_string()).unwrap_or_default(),
            f.size,
            f.sha256,
            f.peek.members.map(|v| v.to_string()).unwrap_or_default(),
            f.peek.texture_member.clone().unwrap_or_default(),
            opt(f.peek.texture_stored_size),
            opt(f.peek.texture_declared_size),
        ));
    }
    out
}

/// Parse a census record back. Refuses a wrong header or a malformed row rather than skipping it.
pub fn parse_portrait_census_csv(text: &str) -> Result<Vec<PortraitFile>, String> {
    let mut lines = text.lines();
    let header = lines.next().ok_or("empty record")?;
    if header != PORTRAIT_CENSUS_HEADER {
        return Err(format!("unexpected header: {header}"));
    }
    let mut files = Vec::new();
    for (n, line) in lines.enumerate() {
        let c = split_csv_line(line);
        if c.len() != 11 {
            return Err(format!("row {}: {} cells", n + 2, c.len()));
        }
        let opt_u32 = |s: &str| -> Result<Option<u32>, String> {
            if s.is_empty() {
                Ok(None)
            } else {
                s.parse()
                    .map(Some)
                    .map_err(|e| format!("row {}: {e}", n + 2))
            }
        };
        let label = PortraitLabel::parse(&c[3]).ok_or(format!("row {}: label {}", n + 2, c[3]))?;
        let numeric_id = if c[4].is_empty() {
            None
        } else {
            Some(c[4].parse().map_err(|e| format!("row {}: {e}", n + 2))?)
        };
        // The recorded label and id must be what the name classifies to — the record carries
        // both so a reader can check them, and this parser does.
        if classify_portrait_name(&c[2]) != (label, numeric_id) {
            return Err(format!("row {}: label/id disagree with the name", n + 2));
        }
        files.push(PortraitFile {
            folder: c[0].clone(),
            bucket: c[1].clone(),
            file_name: c[2].clone(),
            label,
            numeric_id,
            size: c[5].parse().map_err(|e| format!("row {}: {e}", n + 2))?,
            sha256: c[6].clone(),
            peek: BigfPeek {
                members: opt_u32(&c[7])?.map(|v| v as usize),
                texture_member: if c[8].is_empty() {
                    None
                } else {
                    Some(c[8].clone())
                },
                texture_stored_size: opt_u32(&c[9])?,
                texture_declared_size: opt_u32(&c[10])?,
            },
        });
    }
    Ok(files)
}

/// The DB half of a portrait join: what one anchor field stores, as `value -> rows`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IdHistogram {
    pub table: String,
    pub field: String,
    pub rows: usize,
    pub by_value: BTreeMap<u64, usize>,
}

impl IdHistogram {
    pub fn distinct(&self) -> BTreeSet<u64> {
        self.by_value.keys().copied().collect()
    }
    pub fn zero_rows(&self) -> usize {
        self.by_value.get(&0).copied().unwrap_or(0)
    }
    /// Non-zero values stored by more than one row.
    pub fn shared(&self) -> BTreeMap<u64, usize> {
        self.by_value
            .iter()
            .filter(|(v, c)| **v != 0 && **c > 1)
            .map(|(v, c)| (*v, *c))
            .collect()
    }
    pub fn distinct_nonzero(&self) -> BTreeSet<u64> {
        self.distinct().into_iter().filter(|v| *v != 0).collect()
    }
}

/// The ids a folder's `patch.big` members spell, for the same two folders — the D3 overlap input.
pub type PatchPortraitIds = BTreeMap<String, BTreeSet<u64>>;

/// Join the census and the archive members of one folder to the anchor's field, as long-format
/// rows `(folder, measure, value)`. Every measure is a count or a range list; the vocabulary is
/// fixed so a record diff reads as a change of fact, not of wording.
pub fn join_portrait_folder(
    folder: &str,
    files: &[PortraitFile],
    db: &IdHistogram,
    patch_ids: &BTreeSet<u64>,
    outside_lo: u64,
    outside_hi: u64,
) -> Vec<(String, String, String)> {
    let f: Vec<&PortraitFile> = files.iter().filter(|f| f.folder == folder).collect();
    let mut current_by_id: BTreeMap<u64, Vec<&PortraitFile>> = BTreeMap::new();
    let mut backup_by_id: BTreeMap<u64, Vec<&PortraitFile>> = BTreeMap::new();
    let mut other = 0usize;
    for file in &f {
        match (file.label, file.numeric_id) {
            (PortraitLabel::Current, Some(id)) => current_by_id.entry(id).or_default().push(file),
            (PortraitLabel::BackupLabelled, Some(id)) => {
                backup_by_id.entry(id).or_default().push(file)
            }
            _ => other += 1,
        }
    }
    let current_ids: BTreeSet<u64> = current_by_id.keys().copied().collect();
    let backup_ids: BTreeSet<u64> = backup_by_id.keys().copied().collect();
    let multi_current: BTreeSet<u64> = current_by_id
        .iter()
        .filter(|(_, v)| v.len() > 1)
        .map(|(k, _)| *k)
        .collect();
    let with_backup_sibling: BTreeSet<u64> =
        current_ids.intersection(&backup_ids).copied().collect();
    let backup_without_current: BTreeSet<u64> =
        backup_ids.difference(&current_ids).copied().collect();
    let db_ids = db.distinct_nonzero();
    let db_matched: BTreeSet<u64> = db_ids.intersection(&current_ids).copied().collect();
    let db_unmatched: BTreeSet<u64> = db_ids.difference(&current_ids).copied().collect();
    let unmatched_outside: BTreeSet<u64> = db_unmatched
        .iter()
        .copied()
        .filter(|v| (outside_lo..=outside_hi).contains(v))
        .collect();
    let unmatched_elsewhere: BTreeSet<u64> = db_unmatched
        .difference(&unmatched_outside)
        .copied()
        .collect();
    let files_without_row: BTreeSet<u64> = current_ids.difference(&db_ids).copied().collect();
    let patch_with_row: BTreeSet<u64> = patch_ids.intersection(&db_ids).copied().collect();
    let patch_without_row: BTreeSet<u64> = patch_ids.difference(&db_ids).copied().collect();
    let patch_and_loose: BTreeSet<u64> = patch_ids.intersection(&current_ids).copied().collect();
    let patch_only_rows: BTreeSet<u64> = db_unmatched.intersection(patch_ids).copied().collect();
    let neither: BTreeSet<u64> = db_unmatched.difference(patch_ids).copied().collect();
    let neither_outside: BTreeSet<u64> = neither
        .iter()
        .copied()
        .filter(|v| (outside_lo..=outside_hi).contains(v))
        .collect();
    let patch_outside: BTreeSet<u64> = patch_ids
        .iter()
        .copied()
        .filter(|v| (outside_lo..=outside_hi).contains(v))
        .collect();
    let patch_above: BTreeSet<u64> = patch_ids
        .iter()
        .copied()
        .filter(|v| *v > outside_hi)
        .collect();
    let silhouettes: BTreeSet<u64> = current_ids
        .iter()
        .copied()
        .filter(|v| *v >= 60_000)
        .collect();
    let regular: BTreeSet<u64> = current_ids.difference(&silhouettes).copied().collect();
    let has_zero_file = current_ids.contains(&0) || backup_ids.contains(&0);

    let row = |m: &str, v: String| (folder.to_string(), m.to_string(), v);
    let n = |s: &BTreeSet<u64>| s.len().to_string();
    vec![
        row("files", f.len().to_string()),
        row(
            "files_current",
            current_by_id
                .values()
                .map(Vec::len)
                .sum::<usize>()
                .to_string(),
        ),
        row(
            "files_backup_labelled",
            backup_by_id
                .values()
                .map(Vec::len)
                .sum::<usize>()
                .to_string(),
        ),
        row("files_other", other.to_string()),
        row("current_distinct_ids", n(&current_ids)),
        row("current_ids_with_multiple_files", n(&multi_current)),
        row(
            "current_ids_with_multiple_files_ranges",
            ranges(&multi_current),
        ),
        row(
            "current_regular_id_range",
            match (regular.first(), regular.last()) {
                (Some(a), Some(b)) => format!("{a}-{b}"),
                _ => String::new(),
            },
        ),
        row("current_silhouette_ids", ranges(&silhouettes)),
        row("current_ids_with_backup_sibling", n(&with_backup_sibling)),
        row("backup_ids_without_current", n(&backup_without_current)),
        row(
            "backup_ids_without_current_ranges",
            ranges(&backup_without_current),
        ),
        row("file_for_id_zero", has_zero_file.to_string()),
        row("db_field", format!("{}.{}", db.table, db.field)),
        row("db_rows", db.rows.to_string()),
        row("db_distinct_values", db.distinct().len().to_string()),
        row("db_zero_rows", db.zero_rows().to_string()),
        row(
            "db_shared_ids",
            db.shared()
                .iter()
                .map(|(v, c)| format!("{v}x{c}"))
                .collect::<Vec<_>>()
                .join(";"),
        ),
        row("db_distinct_nonzero_ids", n(&db_ids)),
        row("db_ids_with_current_file", n(&db_matched)),
        row("db_ids_without_current_file", n(&db_unmatched)),
        row(
            "db_ids_without_current_file_outside_regular_ranges",
            n(&unmatched_outside),
        ),
        row(
            "db_ids_without_current_file_elsewhere",
            n(&unmatched_elsewhere),
        ),
        row(
            "db_ids_without_current_file_elsewhere_ranges",
            ranges(&unmatched_elsewhere),
        ),
        row(
            "db_ids_with_current_file_and_backup_sibling",
            n(&db_matched
                .intersection(&with_backup_sibling)
                .copied()
                .collect()),
        ),
        row("current_ids_without_db_row", n(&files_without_row)),
        row("patch_member_ids", n(patch_ids)),
        row("patch_member_ids_ranges", ranges(patch_ids)),
        row("patch_member_ids_with_db_row", n(&patch_with_row)),
        row("patch_member_ids_without_db_row", n(&patch_without_row)),
        row(
            "patch_member_ids_without_db_row_ranges",
            ranges(&patch_without_row),
        ),
        row("patch_member_ids_also_loose_current", n(&patch_and_loose)),
        row("patch_member_ids_outside_regular_ranges", n(&patch_outside)),
        row("patch_member_ids_above_db_max", n(&patch_above)),
        row("db_ids_only_in_patch", n(&patch_only_rows)),
        row("db_ids_only_in_patch_ranges", ranges(&patch_only_rows)),
        row("db_ids_in_neither", n(&neither)),
        row(
            "db_ids_in_neither_outside_regular_ranges",
            n(&neither_outside),
        ),
        row("db_ids_in_neither_ranges", ranges(&neither)),
    ]
}

/// Render join rows as the record's CSV.
pub fn render_join_csv(rows: &[(String, String, String)]) -> String {
    let mut out = String::from("folder,measure,value\n");
    for (a, b, c) in rows {
        assert!(!c.contains(',') && !c.contains('"'), "{a}/{b}: {c:?}");
        out.push_str(&format!("{a},{b},{c}\n"));
    }
    out
}

// ================================================================ D1 / D3: archive member tables

/// One row of an `as7_eb_members` record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EbMemberRow {
    pub index: usize,
    pub path: String,
    pub offset: u64,
    pub size: u32,
    pub word_4: String,
    pub word_12: String,
    pub stored_magic: String,
    pub stored_sha256: String,
    pub chunk_header: String,
    pub decoded_form: String,
    pub decoded_size: Option<u64>,
    pub decoded_sha256: String,
    pub decode_error: String,
    pub loose_present: Option<bool>,
    pub loose_size: Option<u64>,
    pub loose_sha256: String,
    pub loose_bak_present: Option<bool>,
    pub loose_bak_size: Option<u64>,
    pub loose_bak_sha256: String,
}

impl EbMemberRow {
    pub fn directory(&self) -> &str {
        self.path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
    }
    pub fn base_name(&self) -> &str {
        self.path
            .rsplit_once('/')
            .map(|(_, b)| b)
            .unwrap_or(&self.path)
    }
}

/// Parse a member record. Refuses a wrong header or a malformed row rather than skipping it.
pub fn parse_eb_members_csv(text: &str) -> Result<Vec<EbMemberRow>, String> {
    let mut lines = text.lines();
    let header = lines.next().ok_or("empty record")?;
    if header != EB_MEMBERS_HEADER {
        return Err(format!("unexpected header: {header}"));
    }
    let mut rows = Vec::new();
    for (n, line) in lines.enumerate() {
        let c = split_csv_line(line);
        if c.len() != 19 {
            return Err(format!("row {}: {} cells", n + 2, c.len()));
        }
        let e = |e: std::num::ParseIntError| format!("row {}: {e}", n + 2);
        let opt_u64 = |s: &str| -> Result<Option<u64>, String> {
            if s.is_empty() {
                Ok(None)
            } else {
                s.parse().map(Some).map_err(e)
            }
        };
        let opt_bool = |s: &str| -> Result<Option<bool>, String> {
            match s {
                "" => Ok(None),
                "true" => Ok(Some(true)),
                "false" => Ok(Some(false)),
                _ => Err(format!("row {}: bool {s}", n + 2)),
            }
        };
        let index: usize = c[0].parse().map_err(e)?;
        if index != n {
            return Err(format!("row {}: index {index} out of order", n + 2));
        }
        rows.push(EbMemberRow {
            index,
            path: c[1].clone(),
            offset: c[2].parse().map_err(e)?,
            size: c[3].parse().map_err(e)?,
            word_4: c[4].clone(),
            word_12: c[5].clone(),
            stored_magic: c[6].clone(),
            stored_sha256: c[7].clone(),
            chunk_header: c[8].clone(),
            decoded_form: c[9].clone(),
            decoded_size: opt_u64(&c[10])?,
            decoded_sha256: c[11].clone(),
            decode_error: c[12].clone(),
            loose_present: opt_bool(&c[13])?,
            loose_size: opt_u64(&c[14])?,
            loose_sha256: c[15].clone(),
            loose_bak_present: opt_bool(&c[16])?,
            loose_bak_size: opt_u64(&c[17])?,
            loose_bak_sha256: c[18].clone(),
        });
    }
    Ok(rows)
}

/// A member name split into its stem components and extension: `texlib_10_0_0.rx2` →
/// `["texlib", "10", "0", "0"]`, `rx2`.
pub fn name_parts(base_name: &str) -> (Vec<&str>, &str) {
    let (stem, ext) = base_name.rsplit_once('.').unwrap_or((base_name, ""));
    (stem.split('_').collect(), ext)
}

/// The name shape with every all-digit component replaced by `N` — the histogram key the D1
/// summary groups member names by.
pub fn name_shape(base_name: &str) -> String {
    let (parts, ext) = name_parts(base_name);
    let shaped: Vec<&str> = parts
        .iter()
        .map(|p| {
            if !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()) {
                "N"
            } else {
                *p
            }
        })
        .collect();
    if ext.is_empty() {
        shaped.join("_")
    } else {
        format!("{}.{ext}", shaped.join("_"))
    }
}

/// One declared or observed name grammar over a directory: which members it matches and which
/// component (0-based, within the stem) carries the id the DB field is compared to.
#[derive(Debug, Clone)]
pub struct Grammar {
    /// Row key, e.g. `logo/texlib_{team-id}_0_0`.
    pub label: &'static str,
    /// Directory inside the archive, e.g. `rendering/logo`.
    pub directory: &'static str,
    /// The member name shape pattern (see [`name_shape`] and [`shape_matches`]) this grammar
    /// covers.
    pub shape: &'static str,
    /// Extra literal requirements on components, `(component index, literal)`.
    pub literal: &'static [(usize, &'static str)],
    /// Index of the id component, or the components of a composite id in order.
    pub id_components: &'static [usize],
    /// The anchor field(s) the id is compared to, for the record.
    pub db_field: &'static str,
    /// Where the grammar comes from: `declared` (a committed profile template), `source` (HEAD
    /// repository source such as `SLOT_SPECS`) or `observed` (a shape the listing shows that no
    /// committed template spells).
    pub origin: &'static str,
}

/// Does a name shape match a grammar's shape pattern? The pattern is a shape whose non-numeric
/// components may be `*`, standing for any ONE non-numeric component (a map suffix such as `dm`
/// or `tm`); `N` matches only `N`, everything else matches literally, and the extension must be
/// equal. Component counts must agree, so a pattern never absorbs an extra component.
pub fn shape_matches(pattern: &str, shape: &str) -> bool {
    let (pp, pe) = name_parts(pattern);
    let (sp, se) = name_parts(shape);
    pe == se
        && pp.len() == sp.len()
        && pp
            .iter()
            .zip(&sp)
            .all(|(p, s)| if *p == "*" { *s != "N" } else { p == s })
}

/// The id a member spells under a grammar, if the member matches it.
pub fn grammar_id(g: &Grammar, row: &EbMemberRow) -> Option<String> {
    if row.directory() != g.directory {
        return None;
    }
    let base = row.base_name();
    if !shape_matches(g.shape, &name_shape(base)) {
        return None;
    }
    let (parts, _) = name_parts(base);
    for (i, lit) in g.literal {
        if parts.get(*i) != Some(lit) {
            return None;
        }
    }
    let mut id = Vec::with_capacity(g.id_components.len());
    for i in g.id_components {
        id.push(*parts.get(*i)?);
    }
    Some(id.join("_"))
}

/// One row of the member join record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrammarJoin {
    pub label: String,
    pub archive: String,
    pub directory: String,
    pub shape: String,
    pub origin: String,
    pub directory_members: usize,
    pub matching_members: usize,
    pub distinct_ids: usize,
    pub id_range: String,
    pub db_field: String,
    pub db_distinct_ids: usize,
    pub db_ids_with_member: usize,
    pub db_ids_without_member: usize,
    pub db_ids_without_member_list: String,
    pub member_ids_without_db_row: usize,
    pub member_ids_without_db_row_list: String,
}

/// Join one grammar over one archive's rows against a set of rendered DB ids.
pub fn join_grammar(
    g: &Grammar,
    archive: &str,
    rows: &[EbMemberRow],
    db_ids: &BTreeSet<String>,
) -> GrammarJoin {
    let directory_members = rows.iter().filter(|r| r.directory() == g.directory).count();
    let mut member_ids: BTreeSet<String> = BTreeSet::new();
    let mut matching = 0usize;
    for r in rows {
        if let Some(id) = grammar_id(g, r) {
            matching += 1;
            member_ids.insert(id);
        }
    }
    let with: BTreeSet<&String> = db_ids.intersection(&member_ids).collect();
    let without: Vec<&String> = db_ids.difference(&member_ids).collect();
    let extra: Vec<&String> = member_ids.difference(db_ids).collect();
    let numeric: BTreeSet<u64> = member_ids.iter().filter_map(|s| s.parse().ok()).collect();
    let id_range = if g.id_components.len() == 1 && numeric.len() == member_ids.len() {
        match (numeric.first(), numeric.last()) {
            (Some(a), Some(b)) => format!("{a}-{b}"),
            _ => String::new(),
        }
    } else {
        String::new()
    };
    let list = |v: &[&String]| -> String {
        if g.id_components.len() == 1 {
            let set: BTreeSet<u64> = v.iter().filter_map(|s| s.parse().ok()).collect();
            if set.len() == v.len() {
                return ranges(&set);
            }
        }
        v.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(";")
    };
    GrammarJoin {
        label: g.label.to_string(),
        archive: archive.to_string(),
        directory: g.directory.to_string(),
        shape: g.shape.to_string(),
        origin: g.origin.to_string(),
        directory_members,
        matching_members: matching,
        distinct_ids: member_ids.len(),
        id_range,
        db_field: g.db_field.to_string(),
        db_distinct_ids: db_ids.len(),
        db_ids_with_member: with.len(),
        db_ids_without_member: without.len(),
        db_ids_without_member_list: list(&without),
        member_ids_without_db_row: extra.len(),
        member_ids_without_db_row_list: list(&extra),
    }
}

/// The member join record's header.
pub const MEMBER_JOIN_HEADER: &str = "grammar,archive,directory,shape,origin,directory_members,matching_members,distinct_ids,id_range,db_field,db_distinct_ids,db_ids_with_member,db_ids_without_member,db_ids_without_member_list,member_ids_without_db_row,member_ids_without_db_row_list";

pub fn render_member_join_csv(rows: &[GrammarJoin]) -> String {
    let mut out = String::from(MEMBER_JOIN_HEADER);
    out.push('\n');
    for r in rows {
        for cell in [
            &r.label,
            &r.archive,
            &r.directory,
            &r.shape,
            &r.origin,
            &r.id_range,
            &r.db_field,
            &r.db_ids_without_member_list,
            &r.member_ids_without_db_row_list,
        ] {
            assert!(!cell.contains(',') && !cell.contains('"'), "{cell:?}");
        }
        out.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
            r.label,
            r.archive,
            r.directory,
            r.shape,
            r.origin,
            r.directory_members,
            r.matching_members,
            r.distinct_ids,
            r.id_range,
            r.db_field,
            r.db_distinct_ids,
            r.db_ids_with_member,
            r.db_ids_without_member,
            r.db_ids_without_member_list,
            r.member_ids_without_db_row,
            r.member_ids_without_db_row_list,
        ));
    }
    out
}

/// Name-shape histogram of one archive's members by directory, as `(directory, shape) -> count`,
/// the D1 summary a reader can check a grammar against.
pub fn shape_histogram(rows: &[EbMemberRow]) -> BTreeMap<(String, String), usize> {
    let mut out = BTreeMap::new();
    for r in rows {
        *out.entry((r.directory().to_string(), name_shape(r.base_name())))
            .or_default() += 1;
    }
    out
}

pub fn render_shape_histogram_csv(archive: &str, rows: &[EbMemberRow]) -> String {
    let mut out = String::from("archive,directory,shape,members\n");
    for ((d, s), c) in shape_histogram(rows) {
        assert!(!d.contains(',') && !s.contains(','));
        out.push_str(&format!("{archive},{d},{s},{c}\n"));
    }
    out
}
