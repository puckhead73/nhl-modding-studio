//! Phase 1.2 — editing one stored catalogue default on NHL 16–21 PS4 (D3 extension after D4),
//! the per-title contract of research handoff 01 rev. 2 §3–§4 under the coordinator addendum.
//!
//! The catalogue is plain CSV text inside `databigs/inxp_00.big` (`chunkzip` members, three raw).
//! A stored default is one cell: NHL 16 `setting.csv` `default_value` (global) or a
//! `contextsetting.csv` row; NHL 17 the reserved `_DEFAULT_` (−2) row of `contextsetting.csv`
//! (global) or a screen row; NHL 18–21 the **pair** `setting.csv` `default_value` **and** the
//! `_DEFAULT_` row (global — one explicit paired edit of two cells in two members, one review, one
//! undo, atomic apply) or a screen row. The write is a **byte splice** of the target cell inside the
//! member's plain text (nothing else is re-serialised), verified by whole-buffer equality outside the
//! span and by a re-parse; the edited member is re-encoded with the native-framed `chunkzip`
//! encoder (raw members stay raw), independently decoded, and placed by the archive stager on a
//! copied target (fit-in-slot or append-and-retarget — decided by the stager from the actual
//! encoded length, predicted here from the directory).
//!
//! The first writer operates on the **measured subset** only (§3.4): ASCII, no BOM, no `"`, every
//! record CRLF-terminated (the last included), unique required headers. Anything else is refused
//! before any splice. Nothing here says what the game offers or applies.

use std::path::Path;
use std::sync::Arc;

use sha2::{Digest, Sha256};
use tdb_archive_big::eb::{self, EbHeader};

use crate::leagues::AttachedDictionary;
use crate::settings_catalogue::{self as sc, CsvVariant, Eligibility, SettingCsvMembers, SettingsCatalogue, DEFAULT_CONTEXT, ELIGIBLE_LEGACY_IDS};
use crate::settings_edit::{DefaultEditRefusal, DefaultTarget};

fn refuse(kind: &'static str, reason: impl Into<String>) -> DefaultEditRefusal {
    DefaultEditRefusal { kind, reason: reason.into() }
}

fn hex(b: &[u8]) -> String {
    b.iter().map(|x| format!("{x:02x}")).collect()
}

// ---------------------------------------------------------------------------------------------
// Span-aware subset parser
// ---------------------------------------------------------------------------------------------

/// One record of a member with the byte spans of its cells (the header is record 0).
#[derive(Debug, Clone)]
pub struct SpannedRow {
    /// 1-based line number.
    pub line: usize,
    /// `[start, end)` of each cell's bytes (no delimiter, no terminator).
    pub cells: Vec<(usize, usize)>,
}

/// A member parsed within the writer subset: header names, every record's cell spans.
#[derive(Debug, Clone)]
pub struct SpannedCsv {
    pub header: Vec<String>,
    pub rows: Vec<SpannedRow>,
}

/// Parse `bytes` within the first writer's measured subset (handoff 01 §3.4): ASCII only, no
/// BOM, no `"` anywhere, CRLF after every record including the last, no blank records; the header
/// names unique. Ragged records are refused here (the writer never edits a member it cannot
/// account for cell by cell).
pub fn parse_spanned(name: &str, bytes: &[u8]) -> Result<SpannedCsv, DefaultEditRefusal> {
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return Err(refuse("member-refused", format!("{name}: a byte-order mark is outside the writer's subset")));
    }
    if let Some(pos) = bytes.iter().position(|b| *b == b'"') {
        return Err(refuse("member-refused", format!("{name}: a quote character at byte {pos} — quoting is not interpreted; the member is browse-only")));
    }
    if bytes.iter().any(|b| *b >= 0x80 || *b == 0) {
        return Err(refuse("member-refused", format!("{name}: non-ASCII or NUL bytes are outside the writer's subset")));
    }
    if bytes.is_empty() {
        return Err(refuse("member-refused", format!("{name}: empty member")));
    }
    let mut rows = Vec::new();
    let mut pos = 0usize;
    let mut line = 0usize;
    while pos < bytes.len() {
        line += 1;
        let Some(rel) = bytes[pos..].iter().position(|b| *b == b'\n') else {
            return Err(refuse("member-refused", format!("{name}: line {line} has no CRLF terminator (the last record must end with CRLF too)")));
        };
        let end = pos + rel;
        if end == 0 || bytes[end - 1] != b'\r' {
            return Err(refuse("member-refused", format!("{name}: line {line} ends with LF, not CRLF — outside the writer's subset")));
        }
        let content_end = end - 1;
        if content_end == pos {
            return Err(refuse("member-refused", format!("{name}: line {line} is blank — outside the writer's subset")));
        }
        if bytes[pos..content_end].contains(&b'\r') {
            return Err(refuse("member-refused", format!("{name}: line {line} carries a bare CR")));
        }
        let mut cells = Vec::new();
        let mut cs = pos;
        for i in pos..=content_end {
            if i == content_end || bytes[i] == b',' {
                cells.push((cs, i));
                cs = i + 1;
            }
        }
        rows.push(SpannedRow { line, cells });
        pos = end + 1;
    }
    let header: Vec<String> = rows[0].cells.iter().map(|(s, e)| String::from_utf8_lossy(&bytes[*s..*e]).trim().to_string()).collect();
    let mut seen = std::collections::BTreeSet::new();
    for h in &header {
        if !seen.insert(h.clone()) {
            return Err(refuse("member-refused", format!("{name}: header column `{h}` appears twice (ambiguous binding)")));
        }
    }
    for r in &rows[1..] {
        if r.cells.len() != header.len() {
            return Err(refuse("member-refused", format!("{name}: line {} has {} cells, the header {} — a ragged record is outside the writer's subset", r.line, r.cells.len(), header.len())));
        }
    }
    Ok(SpannedCsv { header, rows })
}

impl SpannedCsv {
    pub fn col(&self, name: &str) -> Option<usize> {
        self.header.iter().position(|h| h == name)
    }
    fn cell<'a>(&self, bytes: &'a [u8], row: &SpannedRow, col: usize) -> &'a str {
        let (s, e) = row.cells[col];
        std::str::from_utf8(&bytes[s..e]).unwrap_or("")
    }
    /// The records (excluding the header) whose `key_cols` equal `keys`.
    fn find(&self, bytes: &[u8], keys: &[(&str, i64)]) -> Result<Vec<&SpannedRow>, DefaultEditRefusal> {
        let cols: Vec<usize> = keys.iter().map(|(c, _)| self.col(c).ok_or_else(|| refuse("member-refused", format!("required column `{c}` is missing")))).collect::<Result<_, _>>()?;
        Ok(self.rows[1..]
            .iter()
            .filter(|r| cols.iter().zip(keys).all(|(ci, (_, v))| self.cell(bytes, r, *ci).trim().parse::<i64>().ok() == Some(*v)))
            .collect())
    }
}

// ---------------------------------------------------------------------------------------------
// Model
// ---------------------------------------------------------------------------------------------

/// One spliced cell.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvCellEdit {
    pub member: String,
    pub line: usize,
    pub column: String,
    pub start: usize,
    pub end: usize,
    pub before: String,
    pub after: String,
}

/// One member's rewrite: the plain text before and after, the stored (encoded) bytes after, and
/// the identities the copied-target route binds to.
#[derive(Debug, Clone)]
pub struct CsvMemberEdit {
    /// File name (`contextsetting.csv`).
    pub member: String,
    /// Full archive path (`nhl/data/common/gamemodes/setting_csv/contextsetting.csv`).
    pub member_path: String,
    /// `chunkzip` / `raw` — the source entry's form; the output keeps it.
    pub stored_form: String,
    pub source_plain_sha256: String,
    pub source_plain_len: usize,
    /// The archive entry's stored bytes identity when the edit was planned from the archive
    /// (`None` for fixture/text sources).
    pub source_stored_sha256: Option<String>,
    pub source_stored_len: Option<usize>,
    pub new_plain: Arc<Vec<u8>>,
    pub new_plain_sha256: String,
    pub new_stored: Arc<Vec<u8>>,
    pub new_stored_sha256: String,
    pub cells: Vec<CsvCellEdit>,
    /// `fits its slot` / `will be appended at the end of the archive` — predicted from the
    /// directory; the stager decides.
    pub placement_prediction: Option<String>,
}

/// The exact edit a review holds for NHL 16–21.
#[derive(Debug, Clone)]
pub struct CsvDefaultPlan {
    pub game: String,
    pub variant: CsvVariant,
    pub target: DefaultTarget,
    pub setting_name: String,
    pub label_text: Option<String>,
    pub role: String,
    pub context_name: Option<String>,
    pub before: i64,
    pub before_label: Option<String>,
    pub after: i64,
    pub after_label: Option<String>,
    pub choices: Vec<sc::Choice>,
    pub label_source: String,
    /// `true` for the NHL 18–21 global edit: two cells in two members, one operation.
    pub paired: bool,
    pub members: Vec<CsvMemberEdit>,
    pub unchanged: Vec<(String, i64)>,
    pub no_op: bool,
    pub archive: Option<String>,
    pub stock_anomalies: Vec<String>,
    pub description: String,
    pub notes: Vec<String>,
}

impl CsvDefaultPlan {
    /// Every cell the edit changes (one, or two for the pair).
    pub fn cells(&self) -> impl Iterator<Item = &CsvCellEdit> {
        self.members.iter().flat_map(|m| m.cells.iter())
    }
}

/// The directory facts a placement prediction needs: per member path, `(offset, stored size,
/// slot end = next entry's offset or the archive size)`.
#[derive(Debug, Clone, Default)]
pub struct ArchiveSlots {
    pub archive: String,
    pub archive_len: u64,
    pub slots: Vec<(String, u64, u32, u64)>,
}

impl ArchiveSlots {
    /// Read the front tables of an EB archive (no payloads).
    pub fn read(path: &Path) -> Result<Self, DefaultEditRefusal> {
        use std::io::{Read, Seek, SeekFrom};
        let io = |e: std::io::Error| refuse("archive-missing", format!("{}: {e}", path.display()));
        let mut file = std::fs::File::open(path).map_err(io)?;
        let len = file.metadata().map_err(io)?.len();
        let mut header_buf = vec![0u8; eb::HEADER_LEN];
        file.read_exact(&mut header_buf).map_err(io)?;
        let header = EbHeader::parse(&header_buf).map_err(|e| refuse("parse", format!("{}: {e}", path.display())))?;
        let mut table_buf = vec![0u8; header.table_end()];
        file.seek(SeekFrom::Start(0)).map_err(io)?;
        file.read_exact(&mut table_buf).map_err(io)?;
        let archive = eb::EbArchive::parse(&table_buf).map_err(|e| refuse("parse", format!("{}: {e}", path.display())))?;
        let mut starts: Vec<u64> = archive.entries.iter().map(|e| e.offset).collect();
        starts.sort_unstable();
        starts.dedup();
        let slots = archive
            .entries
            .iter()
            .map(|e| {
                let end = e.offset + e.size as u64;
                let next = starts.iter().copied().find(|s| *s >= end).unwrap_or(len);
                (e.path.clone(), e.offset, e.size, next)
            })
            .collect();
        Ok(Self { archive: path.display().to_string(), archive_len: len, slots })
    }

    fn predict(&self, member_path: &str, new_len: usize) -> Option<String> {
        let (_, off, size, slot_end) = self.slots.iter().find(|(p, ..)| p == member_path)?;
        let slack = slot_end - (off + *size as u64);
        Some(if new_len as u64 <= *size as u64 + slack {
            format!("{new_len} bytes fit the member's slot ({size} + {slack} slack)")
        } else {
            format!("{new_len} bytes exceed the member's slot ({size} + {slack} slack): the member will be appended at the end of the archive and its row re-pointed")
        })
    }
}

// ---------------------------------------------------------------------------------------------
// Planning
// ---------------------------------------------------------------------------------------------

/// Plan a stored-default edit on the CSV catalogue `members` (the current working plain text of
/// every member). `stored_forms`/`stored_identities` come from the archive entries when known.
pub fn plan_csv_default_edit(
    game: &str,
    members: &SettingCsvMembers,
    nhl15_defs: Option<&sc::TitleSettingsDefs>,
    dict: Option<&AttachedDictionary>,
    slots: Option<&ArchiveSlots>,
    stored_identities: &[(String, String, usize)],
    target: DefaultTarget,
    code: i64,
) -> Result<CsvDefaultPlan, DefaultEditRefusal> {
    let cat = sc::read_setting_csv(game, members, nhl15_defs, dict).map_err(|b| refuse(b.kind, b.reason))?;
    let sc::CatalogueSource::SettingCsv { variant, .. } = cat.source else { unreachable!() };
    plan_on_catalogue(game, variant, members, &cat, slots, stored_identities, target, code)
}

#[allow(clippy::too_many_arguments)]
fn plan_on_catalogue(
    game: &str,
    variant: CsvVariant,
    members: &SettingCsvMembers,
    cat: &SettingsCatalogue,
    slots: Option<&ArchiveSlots>,
    stored_identities: &[(String, String, usize)],
    target: DefaultTarget,
    code: i64,
) -> Result<CsvDefaultPlan, DefaultEditRefusal> {
    if !ELIGIBLE_LEGACY_IDS.contains(&target.legacy_id) {
        return match cat.setting(target.legacy_id) {
            Some(row) => match &row.eligibility {
                Eligibility::Excluded { reason } => Err(refuse("excluded", format!("{} (id {}) is not offered: {reason}", row.name, target.legacy_id))),
                _ => Err(refuse("not-offered", format!("{} (id {}) is not a league/competition setting of the stored-default contract", row.name, target.legacy_id))),
            },
            None => Err(refuse("absent", format!("no setting with id {} exists in this catalogue", target.legacy_id))),
        };
    }
    let rows: Vec<&sc::SettingRow> = cat.settings.iter().filter(|s| s.legacy_id == target.legacy_id).collect();
    let row = match rows.as_slice() {
        [r] => *r,
        [] => return Err(refuse("absent", format!("setting id {} is not present in this title's catalogue (absent from setting.csv)", target.legacy_id))),
        _ => return Err(refuse("ambiguous", format!("setting id {} occurs {} times in setting.csv (ambiguous row identity)", target.legacy_id, rows.len()))),
    };
    let role = match &row.eligibility {
        Eligibility::Eligible { role } => role.clone(),
        Eligibility::Excluded { reason } => return Err(refuse("excluded", format!("{} (id {}): {reason}", row.name, row.legacy_id))),
        Eligibility::OutOfScope => return Err(refuse("not-offered", format!("{} (id {}) is outside the contract", row.name, row.legacy_id))),
    };
    if row.is_choice == Some(false) {
        return Err(refuse("shape", format!("{}: is_choice ≠ 1 — not a choice row; not offered", row.name)));
    }
    if row.anomalies.iter().any(|a| a.contains("range-based")) {
        return Err(refuse("shape", format!("{}: choice_to_range ≠ 0 — a range-based row; not offered", row.name)));
    }
    if variant != CsvVariant::Nhl17 && target.context == Some(DEFAULT_CONTEXT) {
        return Err(refuse("not-offered", "the reserved _DEFAULT_ context (-2) is the global default's store; edit the global default (which on NHL 18–21 changes both stored copies together), not this row"));
    }
    if variant == CsvVariant::Nhl17 && target.context == Some(DEFAULT_CONTEXT) {
        return Err(refuse("not-offered", "the reserved _DEFAULT_ context (-2) is the global default on NHL 17; edit the global default"));
    }

    let plain = |name: &str| -> Result<&[u8], DefaultEditRefusal> {
        members.members.get(name).map(|(b, _)| b.as_slice()).ok_or_else(|| refuse("archive-missing", format!("{name}: member not found")))
    };
    let form = |name: &str| members.members.get(name).map(|(_, f)| f.clone()).unwrap_or_default();

    // Resolve the cell(s), the domain and the current code.
    struct Cell {
        member: &'static str,
        line: usize,
        column: &'static str,
        span: (usize, usize),
        before: String,
    }
    let mut cells: Vec<Cell> = Vec::new();
    let (choices, before_code, context_name, domain_note): (Vec<sc::Choice>, Option<i64>, Option<String>, Option<String>);
    match target.context {
        None => {
            let g = row.global.as_ref().ok_or_else(|| refuse("absent", "no global default cell"))?;
            match variant {
                CsvVariant::Nhl16 => {
                    let bytes = plain("setting.csv")?;
                    let csv = parse_spanned("setting.csv", bytes)?;
                    let hits = csv.find(bytes, &[("id", target.legacy_id)])?;
                    let r = match hits.as_slice() {
                        [r] => *r,
                        [] => return Err(refuse("absent", "setting.csv row vanished")),
                        _ => return Err(refuse("ambiguous", format!("setting.csv: id {} occurs {} times", target.legacy_id, hits.len()))),
                    };
                    let c = csv.col("default_value").ok_or_else(|| refuse("member-refused", "setting.csv: no default_value column"))?;
                    cells.push(Cell { member: "setting.csv", line: r.line, column: "default_value", span: r.cells[c], before: csv.cell(bytes, r, c).to_string() });
                }
                CsvVariant::Nhl17 | CsvVariant::Nhl18Plus => {
                    let bytes = plain("contextsetting.csv")?;
                    let csv = parse_spanned("contextsetting.csv", bytes)?;
                    let hits = csv.find(bytes, &[("context_id", DEFAULT_CONTEXT), ("setting_id", target.legacy_id)])?;
                    match hits.as_slice() {
                        [r] => {
                            let c = csv.col("default_value").ok_or_else(|| refuse("member-refused", "contextsetting.csv: no default_value column"))?;
                            cells.push(Cell { member: "contextsetting.csv", line: r.line, column: "default_value", span: r.cells[c], before: csv.cell(bytes, r, c).to_string() });
                        }
                        [] if variant == CsvVariant::Nhl17 => return Err(refuse("no-row", format!("no _DEFAULT_ row stores a global default for {}; adding one is a row-growth operation that is not offered", row.name))),
                        [] => {}
                        _ => return Err(refuse("ambiguous", format!("contextsetting.csv: (-2, {}) is stored {} times (ambiguous)", target.legacy_id, hits.len()))),
                    }
                    if variant == CsvVariant::Nhl18Plus {
                        if !g.paired {
                            return Err(refuse("no-row", format!("{}: the global default is stored once (no _DEFAULT_ row); the paired edit the contract offers needs both copies, and a single-copy write is not offered", row.name)));
                        }
                        if g.consistent != Some(true) {
                            return Err(refuse("ambiguous", format!(
                                "{}: the two stored global copies disagree (setting.csv {} vs _DEFAULT_ row {}); the paired edit requires equal values and the same choice list, and a separate repair route is not offered",
                                row.name,
                                g.values.first().copied().flatten().map_or("null".into(), |v| v.to_string()),
                                g.values.get(1).copied().flatten().map_or("null".into(), |v| v.to_string())
                            )));
                        }
                        let bytes = plain("setting.csv")?;
                        let csv = parse_spanned("setting.csv", bytes)?;
                        let hits = csv.find(bytes, &[("id", target.legacy_id)])?;
                        let r = match hits.as_slice() {
                            [r] => *r,
                            _ => return Err(refuse("ambiguous", "setting.csv row missing or duplicated")),
                        };
                        let c = csv.col("default_value").ok_or_else(|| refuse("member-refused", "setting.csv: no default_value column"))?;
                        cells.push(Cell { member: "setting.csv", line: r.line, column: "default_value", span: r.cells[c], before: csv.cell(bytes, r, c).to_string() });
                    }
                }
            }
            choices = row.choices.clone();
            before_code = g.code;
            context_name = None;
            domain_note = None;
        }
        Some(ctx) => {
            let cds: Vec<&sc::ContextDefault> = row.context_defaults.iter().filter(|d| d.context == ctx).collect();
            let cd = match cds.as_slice() {
                [d] => *d,
                [] => {
                    let n = cat.context(ctx).and_then(|c| c.name.clone()).map(|n| format!(" ({n})")).unwrap_or_default();
                    return Err(refuse("no-row", format!("no stored default for {} on screen {ctx}{n}; adding one is a row-growth operation that is not offered. This says nothing about which default the game would use there.", row.name)));
                }
                _ => return Err(refuse("ambiguous", format!("contextsetting.csv: (context {ctx}, setting {}) is stored {} times (ambiguous)", target.legacy_id, cds.len()))),
            };
            if cd.enabled == Some(false) || cd.visible == Some(false) {
                return Err(refuse("not-offered", format!("{} on screen {ctx}: the row is flagged {} (meaning unknown); not offered", row.name, if cd.enabled == Some(false) { "is_enabled 0" } else { "is_visible 0" })));
            }
            if let Some(n) = &cd.domain_note {
                if n.contains("not interpreted") || n.contains("ambiguous") || n.contains("not found") {
                    return Err(refuse("not-offered", format!("{} on screen {ctx}: {n}; not offered", row.name)));
                }
            }
            let bytes = plain("contextsetting.csv")?;
            let csv = parse_spanned("contextsetting.csv", bytes)?;
            let hits = csv.find(bytes, &[("context_id", ctx), ("setting_id", target.legacy_id)])?;
            let r = match hits.as_slice() {
                [r] => *r,
                _ => return Err(refuse("ambiguous", "contextsetting.csv row missing or duplicated")),
            };
            let c = csv.col("default_value").ok_or_else(|| refuse("member-refused", "contextsetting.csv: no default_value column"))?;
            cells.push(Cell { member: "contextsetting.csv", line: r.line, column: "default_value", span: r.cells[c], before: csv.cell(bytes, r, c).to_string() });
            choices = cd.choices.clone().unwrap_or_else(|| row.choices.clone());
            before_code = cd.stored.code;
            context_name = cd.context_name.clone();
            domain_note = cd.domain_note.clone();
        }
    }
    // Null cell.
    let Some(before) = before_code else {
        return Err(refuse("not-offered", format!("{}: the stored default is empty (null); writing a value where EA stores none is not offered", row.name)));
    };
    // Domain.
    match variant {
        CsvVariant::Nhl16 => {
            let (lo, hi) = (row.min.unwrap_or(0), row.max.unwrap_or(-1));
            if code < lo || code > hi {
                return Err(refuse("out-of-bounds", format!("code {code} is outside {}'s range {lo}..{hi}; refused", row.name)));
            }
        }
        _ => {
            if choices.is_empty() {
                return Err(refuse("shape", format!("{}: no choice list is bound to this row (list numbering ambiguous or missing); not offered", row.name)));
            }
            if !choices.iter().any(|c| c.code == code) {
                return Err(refuse("out-of-bounds", format!("code {code} is not a value of {}'s choice list ({}); refused", row.name, choices.iter().map(|c| c.code.to_string()).collect::<Vec<_>>().join(", "))));
            }
        }
    }
    // Every cell must currently read the code the catalogue reported (the same text).
    for c in &cells {
        if c.before.trim().parse::<i64>().ok() != Some(before) {
            return Err(refuse("ambiguous", format!("{} line {}: the cell reads `{}`, not {before}", c.member, c.line, c.before)));
        }
        if c.before.trim() != c.before || c.before.starts_with('+') || (c.before.len() > 1 && c.before.starts_with('0')) {
            return Err(refuse("member-refused", format!("{} line {}: the cell `{}` is not a plain decimal code; not rewritten", c.member, c.line, c.before)));
        }
    }
    let label_of = |c: i64| choices.iter().find(|x| x.code == c).map(|x| x.text.clone().unwrap_or_else(|| x.key.clone()));
    let no_op = code == before;
    let after_text = code.to_string();

    // Splice + verify + encode per member.
    let mut member_edits: Vec<CsvMemberEdit> = Vec::new();
    for c in &cells {
        let bytes = plain(c.member)?;
        let (s, e) = c.span;
        let mut new_plain = Vec::with_capacity(bytes.len() + 4);
        new_plain.extend_from_slice(&bytes[..s]);
        new_plain.extend_from_slice(after_text.as_bytes());
        new_plain.extend_from_slice(&bytes[e..]);
        verify_splice(c.member, bytes, &new_plain, (s, e), after_text.len(), c.line).map_err(|r| refuse("verify", r))?;
        let f = form(c.member);
        let new_stored: Vec<u8> = if f == "chunkzip" {
            let enc = tdb_archive_codec::compress_chunkzip_native(&new_plain);
            tdb_archive_codec::is_native_chunkzip_framing(&enc).map_err(|r| refuse("verify", format!("{}: re-encoded member is not native-framed: {r}", c.member)))?;
            let dec = tdb_archive_codec::decompress_chunkzip(&enc).map_err(|r| refuse("verify", format!("{}: re-encoded member does not decode: {r}", c.member)))?;
            if dec != new_plain {
                return Err(refuse("verify", format!("{}: independent decode of the re-encoded member differs from the edited text", c.member)));
            }
            enc
        } else if f == "raw" || f == "file" || f == "text" {
            new_plain.clone()
        } else {
            return Err(refuse("member-refused", format!("{}: stored as {f}; only chunkzip and raw members are rewritten", c.member)));
        };
        let member_path = format!("{}{}", sc::CSV_MEMBER_PREFIX, c.member);
        let ident = stored_identities.iter().find(|(p, ..)| p == &member_path);
        let placement_prediction = slots.and_then(|sl| sl.predict(&member_path, new_stored.len()));
        member_edits.push(CsvMemberEdit {
            member: c.member.to_string(),
            member_path,
            stored_form: if f == "chunkzip" { "chunkzip".into() } else { "raw".into() },
            source_plain_sha256: hex(&Sha256::digest(bytes)),
            source_plain_len: bytes.len(),
            source_stored_sha256: ident.map(|(_, s, _)| s.clone()),
            source_stored_len: ident.map(|(.., l)| *l),
            new_plain_sha256: hex(&Sha256::digest(&new_plain)),
            new_stored_sha256: hex(&Sha256::digest(&new_stored)),
            new_plain: Arc::new(new_plain),
            new_stored: Arc::new(new_stored),
            cells: vec![CsvCellEdit { member: c.member.to_string(), line: c.line, column: c.column.to_string(), start: s, end: e, before: c.before.clone(), after: after_text.clone() }],
            placement_prediction,
        });
    }
    let paired = cells.len() == 2;
    let mut unchanged: Vec<(String, i64)> = Vec::new();
    if target.context.is_some() {
        if let Some(g) = &row.global {
            if let Some(c) = g.code {
                unchanged.push(("global".into(), c));
            }
        }
    }
    for d in &row.context_defaults {
        if Some(d.context) != target.context {
            if let Some(c) = d.stored.code {
                unchanged.push((d.context_name.clone().map(|n| format!("context {} ({n})", d.context)).unwrap_or_else(|| format!("context {}", d.context)), c));
            }
        }
    }
    let display = row.label_text.clone().unwrap_or_else(|| row.name.clone());
    let screen = match target.context {
        None => "the global default".to_string(),
        Some(c) => match &context_name {
            Some(n) => format!("the {n} screen (context {c})"),
            None => format!("screen context {c}"),
        },
    };
    let src = if row.label_source == sc::NameSource::Explicit && cells.iter().all(|_| true) { "explicit" } else { row.label_source.as_str() };
    let fmt = |c: i64| match label_of(c) {
        Some(l) => format!("code {c} (*{l}*, label {src})"),
        None => format!("code {c}"),
    };
    let members_text = member_edits.iter().map(|m| format!("`{}` line {} ({} → {} bytes stored{})", m.member, m.cells[0].line, m.source_stored_len.map_or("?".into(), |l| l.to_string()), m.new_stored.len(), m.placement_prediction.as_ref().map(|p| format!("; {p}")).unwrap_or_default())).collect::<Vec<_>>().join(" and ");
    let title = game.to_uppercase();
    let description = if no_op {
        format!("{title} PS4 — stored catalogue default of *{display}* for {screen} is already {}; nothing to change.", fmt(before))
    } else if paired {
        format!(
            "{title} PS4 — stored catalogue default of *{display}*: the global default stored in `setting.csv` row {} **and** in `contextsetting.csv` row `_DEFAULT_`/{}, both from {} to {}. Two text files are rewritten (one cell each) and re-compressed: {members_text}. {} Whether this stored default is what the game offers or applies has not been established.",
            target.legacy_id, target.legacy_id, fmt(before), fmt(code), unchanged_text(&unchanged)
        )
    } else {
        format!(
            "{title} PS4 — stored catalogue default of *{display}* for {screen} changes from {} to {}. The text file is rewritten (one cell) and re-compressed: {members_text}. {} Whether this stored default is what the game offers or applies has not been established.",
            fmt(before), fmt(code), unchanged_text(&unchanged)
        )
    };
    let mut notes = vec![
        "The edited member is re-encoded with the native-framed chunkzip encoder (zlib level 9 reproduces every measured stock member); native loader acceptance of a re-encoded or re-pointed member is not established.".to_string(),
        "Only the target cell's bytes change inside the text; every other byte of the member is copied verbatim.".to_string(),
    ];
    if let Some(n) = domain_note {
        notes.push(format!("This screen's row uses its own choice list: {n}."));
    }
    if paired {
        notes.push("Stock keeps the two global copies equal; that equality is observed, not an engine requirement. The edit keeps them equal; an unequal pair is refused.".to_string());
    }
    Ok(CsvDefaultPlan {
        game: game.to_string(),
        variant,
        target,
        setting_name: row.name.clone(),
        label_text: row.label_text.clone(),
        role,
        context_name,
        before,
        before_label: label_of(before),
        after: code,
        after_label: label_of(code),
        choices,
        label_source: src.to_string(),
        paired,
        members: member_edits,
        unchanged,
        no_op,
        archive: slots.map(|s| s.archive.clone()),
        stock_anomalies: cat.anomalies.clone(),
        description,
        notes,
    })
}

fn unchanged_text(unchanged: &[(String, i64)]) -> String {
    if unchanged.is_empty() {
        "This is the only stored default of the setting.".to_string()
    } else {
        format!("Unchanged: {}.", unchanged.iter().map(|(n, c)| format!("{n} = {c}")).collect::<Vec<_>>().join(", "))
    }
}

/// The one-field preservation contract (handoff 01 §4): whole-buffer equality outside the spliced
/// span; both buffers re-parse within the subset with the same header, row count and cells except
/// the one `(line, column)`.
pub fn verify_splice(member: &str, before: &[u8], after: &[u8], span: (usize, usize), new_len: usize, line: usize) -> Result<(), String> {
    let (s, e) = span;
    if after.len() != before.len() - (e - s) + new_len {
        return Err(format!("{member}: length accounting failed ({} → {}, span {}..{}, new {new_len})", before.len(), after.len(), s, e));
    }
    if before[..s] != after[..s] {
        return Err(format!("{member}: bytes before the cell differ"));
    }
    if before[e..] != after[s + new_len..] {
        return Err(format!("{member}: bytes after the cell differ"));
    }
    let a = parse_spanned(member, before).map_err(|r| r.reason)?;
    let b = parse_spanned(member, after).map_err(|r| r.reason)?;
    if a.header != b.header {
        return Err(format!("{member}: the header changed"));
    }
    if a.rows.len() != b.rows.len() {
        return Err(format!("{member}: the record count changed ({} → {})", a.rows.len(), b.rows.len()));
    }
    for (ra, rb) in a.rows.iter().zip(&b.rows) {
        if ra.cells.len() != rb.cells.len() {
            return Err(format!("{member}: line {} changed its cell count", ra.line));
        }
        for (ci, ((sa, ea), (sb, eb))) in ra.cells.iter().zip(&rb.cells).enumerate() {
            let same = before[*sa..*ea] == after[*sb..*eb];
            let is_target = ra.line == line && *sa == s && *ea == e;
            if !same && !is_target {
                return Err(format!("{member}: line {} cell {ci} (`{}`) changed", ra.line, a.header.get(ci).map(String::as_str).unwrap_or("?")));
            }
        }
    }
    Ok(())
}

/// Apply `plan`'s member rewrites on top of `members` (the working texts), producing the next
/// working set; used when a second edit is planned on a file that already carries one.
pub fn apply_to_members(members: &SettingCsvMembers, plan: &CsvDefaultPlan) -> SettingCsvMembers {
    let mut out = members.clone();
    for m in &plan.members {
        if let Some(entry) = out.members.get_mut(&m.member) {
            entry.0 = (*m.new_plain).clone();
        }
    }
    out
}
