//! Phase 1.2 — editing **one stored catalogue default** on NHL 15 PS4 (D3 / P1-BS-3), the
//! accepted contract of research handoff 22 rev. 3 §2 with the P1-BS-R1 acceptance note taking
//! precedence.
//!
//! A target is `(legacy id, context)`: `context = None` addresses `stocksettings.default` of the
//! row, `Some(c)` the `stocksettingdefaults.default` of the row `(c, legacy id)`, which must
//! already exist. Only the 21 enumerated legacy ids are offered; eligibility is evaluated on the
//! **opened bytes** every time (a file already carrying reviewed edits stays eligible; nothing is
//! exempted from bounds or identity validation), pre-existing stock anomalies are preserved, and
//! only defects the edit would introduce or that make the target ambiguous are refused.
//!
//! The write changes the bits of the selected field only; the covering body CRC (the next physical
//! table's `prior_crc`, or the EOF CRC for the last body) is the only checksum word allowed to
//! differ — table headers, the file header and their CRCs stay byte-identical, and an unrelated
//! invalid checksum is neither repaired nor allowed to change. [`verify_single_cell_diff`] states
//! that rule as an accounting of exact file offsets, and the workspace applies it after every
//! apply, rolling the edit back when the accounting fails.
//!
//! Nothing here says whether the game offers or applies a stored default, which of the two NHL 15
//! database copies it reads, or where a chosen value is persisted.

use std::collections::BTreeMap;

use tdb_core::{Endian, TdbFile};
use tdb_defs::FieldMetaIndex;

use crate::leagues::AttachedDictionary;
use crate::settings_catalogue::{self as sc, Eligibility, SettingsCatalogue, ELIGIBLE_LEGACY_IDS};

/// Raw ids of the two tables the contract writes to and of the edited field in each.
pub const STOCKSETTINGS: &str = "fRvu";
pub const STOCKSETTINGDEFAULTS: &str = "gZtX";
pub const FIELD_DEFAULT: &str = "npHF";
const FIELD_LEGACYID: &str = "jjHq";
const FIELD_CONTEXT: &str = "mmiA";

/// One stored-default cell: a legacy id and the screen context (`None` = the global default).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DefaultTarget {
    pub legacy_id: i64,
    pub context: Option<i64>,
}

impl DefaultTarget {
    pub fn describe(&self) -> String {
        match self.context {
            None => format!("legacy id {} (global)", self.legacy_id),
            Some(c) => format!("legacy id {} on context {c}", self.legacy_id),
        }
    }
}

/// The exact edit a review holds: the one cell, its before/after codes and everything the user
/// sees beside it. Apply writes `record`/`studio_field` = `after` and nothing else.
#[derive(Debug, Clone)]
pub struct DefaultEditPlan {
    pub target: DefaultTarget,
    pub setting_name: String,
    pub label_key: Option<String>,
    pub label_text: Option<String>,
    pub role: String,
    pub context_name: Option<String>,
    /// `stocksettings` / `stocksettingdefaults` (raw id, = studio id on PS4).
    pub studio_table: String,
    pub table_name: String,
    pub record: u32,
    pub studio_field: String,
    pub before: i64,
    pub before_label: Option<String>,
    pub after: i64,
    pub after_label: Option<String>,
    /// `0..=max` of the row (offset reading); the global column additionally caps at 7.
    pub min: i64,
    pub max: i64,
    pub size: i64,
    pub choices: Vec<sc::Choice>,
    /// The label source (`inferred` on NHL 15 — the executable's array order).
    pub label_source: String,
    /// Rows sharing the setting identity (informational, unchanged).
    pub siblings: Vec<i64>,
    /// The other stored defaults of the same legacy id (unchanged): `("global" | "context N", code)`.
    pub unchanged: Vec<(String, i64)>,
    /// The copy the project opened (`base 01.00` / `patch 01.06` / unrecorded).
    pub copy: String,
    /// `true` when `after == before`: a no-op review, Apply writes nothing.
    pub no_op: bool,
    /// Stock anomalies of the catalogue, reported once and preserved by the edit.
    pub stock_anomalies: Vec<String>,
    /// The human-readable review text (ids behind disclosure).
    pub description: String,
    pub notes: Vec<String>,
}

/// Why a target/code was refused (nothing written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DefaultEditRefusal {
    /// `not-offered` / `excluded` / `absent` / `no-row` / `ambiguous` / `out-of-bounds` /
    /// `shape` / `declaration`.
    pub kind: &'static str,
    pub reason: String,
}

fn refuse(kind: &'static str, reason: impl Into<String>) -> DefaultEditRefusal {
    DefaultEditRefusal { kind, reason: reason.into() }
}

/// Plan the edit of one stored default: eligibility on the opened bytes, bounds, target identity,
/// review text. Reads through the catalogue reader so every disclosure it makes is the same one
/// the read-only page shows.
pub fn plan_default_edit(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    defs: Option<&sc::TitleSettingsDefs>,
    dict: Option<&AttachedDictionary>,
    target: DefaultTarget,
    code: i64,
) -> Result<DefaultEditPlan, DefaultEditRefusal> {
    let cat = sc::read_database(db, meta, defs, dict).map_err(|b| refuse("declaration", b.reason))?;
    plan_on_catalogue(db, meta, &cat, target, code)
}

fn plan_on_catalogue(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    cat: &SettingsCatalogue,
    target: DefaultTarget,
    code: i64,
) -> Result<DefaultEditPlan, DefaultEditRefusal> {
    // (c) the two tables keep their declared shapes: packed and key-sorted.
    for raw in [STOCKSETTINGS, STOCKSETTINGDEFAULTS] {
        let t = db.table(raw).ok_or_else(|| refuse("shape", format!("{raw}: table missing")))?;
        if t.max_records != t.current_records {
            return Err(refuse("shape", format!("{}: the table is no longer packed ({} of {} records) — not the declared shape", table_name(meta, raw), t.current_records, t.max_records)));
        }
    }
    if !ELIGIBLE_LEGACY_IDS.contains(&target.legacy_id) {
        return match cat.setting(target.legacy_id) {
            Some(row) => match &row.eligibility {
                Eligibility::Excluded { reason } => Err(refuse("excluded", format!("{} (id {}) is not offered for stored-default editing: {reason}", row.name, target.legacy_id))),
                _ => Err(refuse("not-offered", format!("{} (id {}) is not a league/competition setting of the stored-default contract", row.name, target.legacy_id))),
            },
            None => Err(refuse("absent", format!("no setting with legacy id {} exists in this catalogue", target.legacy_id))),
        };
    }
    // (a) the target row exists exactly once.
    let rows: Vec<&sc::SettingRow> = cat.settings.iter().filter(|s| s.legacy_id == target.legacy_id).collect();
    let row = match rows.as_slice() {
        [r] => *r,
        [] => return Err(refuse("absent", format!("no stocksettings row carries legacy id {} in the opened file", target.legacy_id))),
        _ => return Err(refuse("ambiguous", format!("legacy id {} occurs {} times in stocksettings (ambiguous row identity); nothing is written", target.legacy_id, rows.len()))),
    };
    let role = match &row.eligibility {
        Eligibility::Eligible { role } => role.clone(),
        Eligibility::Excluded { reason } => return Err(refuse("excluded", format!("{} (id {}): {reason}", row.name, row.legacy_id))),
        Eligibility::OutOfScope => return Err(refuse("not-offered", format!("{} (id {}) is outside the contract", row.name, row.legacy_id))),
    };
    // (b) the stocksettings entry decodes consistently and starts at 0 (code = value = index).
    let (min, max, size) = (row.min.unwrap_or(0), row.max.unwrap_or(0), row.size.unwrap_or(0));
    if min != 0 {
        return Err(refuse("shape", format!("{}: the row's range starts at {min}, not 0 — the code/value/index equivalence the contract relies on does not hold; not offered", row.name)));
    }
    if size != max - min + 1 {
        return Err(refuse("shape", format!("{}: size {size} ≠ max − min + 1 ({min}..{max}) on the opened file — the row's declaration is inconsistent; not offered", row.name)));
    }
    // Resolve the cell.
    let (studio_table, record, before, context_name) = match target.context {
        None => {
            let g = row.global.as_ref().ok_or_else(|| refuse("absent", "no global default cell"))?;
            let rec = find_setting_record(db, meta, target.legacy_id).map_err(|e| refuse(e.0, e.1))?;
            (STOCKSETTINGS, rec, g.code.unwrap_or(0), None)
        }
        Some(ctx) => {
            let matches: Vec<u32> = find_default_records(db, meta, ctx, target.legacy_id);
            match matches.as_slice() {
                [rec] => {
                    let cd = row.context_defaults.iter().find(|d| d.context == ctx).ok_or_else(|| refuse("no-row", "context row vanished"))?;
                    (STOCKSETTINGDEFAULTS, *rec, cd.stored.code.unwrap_or(0), cd.context_name.clone().or_else(|| cat.context(ctx).and_then(|c| c.name.clone())))
                }
                [] => {
                    let ctx_name = cat.context(ctx).and_then(|c| c.name.clone()).map(|n| format!(" ({n})")).unwrap_or_default();
                    return Err(refuse("no-row", format!("no stored default for {} on screen {ctx}{ctx_name}; adding one is a table-growth operation that is not offered. This says nothing about which default the game would use there.", row.name)));
                }
                _ => return Err(refuse("ambiguous", format!("(context {ctx}, legacy id {}) is stored {} times in stocksettingdefaults (ambiguous); nothing is written", target.legacy_id, matches.len()))),
            }
        }
    };
    // Bounds: 0 ≤ code ≤ max, code < size, and the 3-bit global column caps at 7.
    let col_cap = if target.context.is_none() { 7 } else { 10 };
    if code < 0 || code > max || code >= size {
        return Err(refuse("out-of-bounds", format!("code {code} is outside {}'s range 0..{max} (size {size}); refused", row.name)));
    }
    if code > col_cap {
        return Err(refuse("out-of-bounds", format!("code {code} does not fit the {}-bit `default` column (max {col_cap}); refused", if col_cap == 7 { 3 } else { 4 })));
    }
    let label_of = |c: i64| row.choices.iter().find(|x| x.code == c).map(|x| x.text.clone().unwrap_or_else(|| x.key.clone()));
    let before_label = label_of(before);
    let after_label = label_of(code);
    let no_op = code == before;
    let mut unchanged: Vec<(String, i64)> = Vec::new();
    if target.context.is_some() {
        if let Some(g) = &row.global {
            unchanged.push(("global".to_string(), g.code.unwrap_or(0)));
        }
    }
    for d in &row.context_defaults {
        if Some(d.context) != target.context {
            let n = d.context_name.clone().map(|n| format!("context {} ({n})", d.context)).unwrap_or_else(|| format!("context {}", d.context));
            unchanged.push((n, d.stored.code.unwrap_or(0)));
        }
    }
    let copy = match &cat.source {
        sc::CatalogueSource::Database { copy, .. } => copy.clone(),
        _ => "n/a".to_string(),
    };
    let fmt_label = |c: i64, l: &Option<String>| match l {
        Some(l) => format!("code {c} (*{l}*, label inferred)"),
        None => format!("code {c}"),
    };
    let screen = match target.context {
        None => "the global default".to_string(),
        Some(c) => match &context_name {
            Some(n) => format!("the {n} screen (context {c})"),
            None => format!("screen context {c}"),
        },
    };
    let display_name = row.label_text.clone().unwrap_or_else(|| row.name.clone());
    let unchanged_text = if unchanged.is_empty() {
        "This is the only stored default of the setting.".to_string()
    } else {
        format!("Unchanged: {}.", unchanged.iter().map(|(n, c)| format!("{n} = {c}")).collect::<Vec<_>>().join(", "))
    };
    let description = if no_op {
        format!("NHL 15 PS4 ({copy}) — stored catalogue default of *{display_name}* for {screen} is already {}; nothing to change.", fmt_label(before, &before_label))
    } else {
        format!(
            "NHL 15 PS4 ({copy}) — stored catalogue default of *{display_name}* for {screen} changes from {} to {}. {unchanged_text}{} Whether this stored default is what the game offers or applies has not been established.",
            fmt_label(before, &before_label),
            fmt_label(code, &after_label),
            if row.siblings.is_empty() { String::new() } else { format!(" Rows {} share the setting identity and are not changed.", row.siblings.iter().map(i64::to_string).collect::<Vec<_>>().join(", ")) }
        )
    };
    let mut notes = vec![
        "Only the bits of the selected `default` field change; the covering body checksum is the only checksum word that may differ. Table and file headers stay byte-identical.".to_string(),
    ];
    if copy.starts_with("base") || copy.starts_with("patch") {
        notes.push(format!("The other NHL 15 PS4 copy ({}) is not touched; which copy the installed game reads is unresolved.", if copy.starts_with("base") { "patch_inxp00.big, patch 01.06" } else { "inxp00.big, base 01.00" }));
    }
    Ok(DefaultEditPlan {
        target,
        setting_name: row.name.clone(),
        label_key: row.label_key.clone(),
        label_text: row.label_text.clone(),
        role,
        context_name,
        studio_table: studio_table.to_string(),
        table_name: table_name(meta, studio_table),
        record,
        studio_field: FIELD_DEFAULT.to_string(),
        before,
        before_label,
        after: code,
        after_label,
        min,
        max,
        size,
        choices: row.choices.clone(),
        label_source: row.label_source.as_str().to_string(),
        siblings: row.siblings.clone(),
        unchanged,
        copy,
        no_op,
        stock_anomalies: cat.anomalies.clone(),
        description,
        notes,
    })
}

fn table_name(meta: &FieldMetaIndex, raw: &str) -> String {
    meta.table_name(raw).unwrap_or(raw).to_string()
}

fn find_setting_record(db: &TdbFile, meta: &FieldMetaIndex, legacy_id: i64) -> Result<u32, (&'static str, String)> {
    let t = db.table(STOCKSETTINGS).ok_or(("shape", "stocksettings missing".to_string()))?;
    let f = t.field(FIELD_LEGACYID).ok_or(("shape", "stocksettings.legacyid missing".to_string()))?;
    let m = meta.field(STOCKSETTINGS, FIELD_LEGACYID).ok_or(("declaration", "no metadata for stocksettings.legacyid".to_string()))?;
    let hits: Vec<u32> = (0..t.current_records as u32).filter(|&r| db.read_u64(t, r, f).map(|v| m.decode_raw(v)) == Some(legacy_id)).collect();
    match hits.as_slice() {
        [r] => Ok(*r),
        [] => Err(("absent", format!("legacy id {legacy_id} not found"))),
        _ => Err(("ambiguous", format!("legacy id {legacy_id} occurs {} times", hits.len()))),
    }
}

fn find_default_records(db: &TdbFile, meta: &FieldMetaIndex, context: i64, legacy_id: i64) -> Vec<u32> {
    let Some(t) = db.table(STOCKSETTINGDEFAULTS) else { return Vec::new() };
    let (Some(fc), Some(fl)) = (t.field(FIELD_CONTEXT), t.field(FIELD_LEGACYID)) else { return Vec::new() };
    let (Some(mc), Some(ml)) = (meta.field(STOCKSETTINGDEFAULTS, FIELD_CONTEXT), meta.field(STOCKSETTINGDEFAULTS, FIELD_LEGACYID)) else { return Vec::new() };
    (0..t.current_records as u32)
        .filter(|&r| db.read_u64(t, r, fc).map(|v| mc.decode_raw(v)) == Some(context) && db.read_u64(t, r, fl).map(|v| ml.decode_raw(v)) == Some(legacy_id))
        .collect()
}

// ---------------------------------------------------------------------------------------------
// Preservation accounting
// ---------------------------------------------------------------------------------------------

/// Every file offset a single-cell edit may legitimately change: the byte span of the record with
/// the bit mask of the selected field, and the one checksum word covering the edited body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccountedDiff {
    pub record_start: usize,
    pub record_len: usize,
    /// Per byte of the record, the bits that belong to the selected field.
    pub field_mask: Vec<u8>,
    /// Offset of the covering CRC word (the next physical table's `prior_crc` or the EOF CRC).
    pub covering_crc_offset: usize,
    pub covering_crc_is_eof: bool,
    /// Body range the covering CRC hashes.
    pub body_start: usize,
    pub body_end: usize,
}

/// Derive the accounted offsets for `(table raw id, record, field raw id)` on `db`, independently
/// of the writer: from the table layout (block offsets sorted physically) and the field's bit span.
pub fn accounted_diff(db: &TdbFile, table_raw: &str, record: u32, field_raw: &str) -> Result<AccountedDiff, String> {
    let (idx, table) = db.tables.iter().enumerate().find(|(_, t)| t.id.raw() == table_raw).ok_or_else(|| format!("{table_raw}: no such table"))?;
    let field = table.field(field_raw).ok_or_else(|| format!("{table_raw}.{field_raw}: no such field"))?;
    if record >= table.current_records as u32 {
        return Err(format!("{table_raw}: record {record} out of range"));
    }
    let record_len = table.record_len_bytes as usize;
    let record_start = table.record_block + record as usize * record_len;
    let mut field_mask = vec![0u8; record_len];
    for bit in field.bit_offset..field.bit_offset + field.bit_width {
        let (byte, shift) = match db.endian {
            Endian::Little => ((bit / 8) as usize, bit % 8),
            Endian::Big => ((bit / 8) as usize, 7 - bit % 8),
        };
        if byte < record_len {
            field_mask[byte] |= 1 << shift;
        }
    }
    let mut order: Vec<usize> = (0..db.tables.len()).collect();
    order.sort_by_key(|&i| db.tables[i].block_offset);
    let pos = order.iter().position(|&i| i == idx).unwrap();
    let n = db.bytes().len();
    let (covering_crc_offset, covering_crc_is_eof, body_end) = if pos + 1 < order.len() {
        let next = db.tables[order[pos + 1]].block_offset;
        (next, false, next)
    } else {
        (n - 4, true, n - 4)
    };
    Ok(AccountedDiff {
        record_start,
        record_len,
        field_mask,
        covering_crc_offset,
        covering_crc_is_eof,
        body_start: table.block_offset + 0x28,
        body_end,
    })
}

/// Assert that `after` differs from `before` only inside the accounted offsets of a single-cell
/// edit, and that the covering CRC word equals the CRC of the new body. Returns the first
/// unaccounted difference as the error text.
pub fn verify_single_cell_diff(before: &[u8], after: &[u8], acc: &AccountedDiff, endian: Endian) -> Result<(), String> {
    if before.len() != after.len() {
        return Err(format!("length changed {} → {}", before.len(), after.len()));
    }
    let rec = acc.record_start..acc.record_start + acc.record_len;
    let crc = acc.covering_crc_offset..acc.covering_crc_offset + 4;
    for (i, (b, a)) in before.iter().zip(after).enumerate() {
        if b == a {
            continue;
        }
        if crc.contains(&i) {
            continue;
        }
        if rec.contains(&i) {
            let mask = acc.field_mask[i - acc.record_start];
            if (b ^ a) & !mask == 0 {
                continue;
            }
            return Err(format!("byte {i:#x} inside the record changed outside the selected field's bits (mask {mask:#04x}: {b:#04x} → {a:#04x})"));
        }
        return Err(format!("byte {i:#x} changed outside the selected cell and its covering checksum ({b:#04x} → {a:#04x})"));
    }
    let expected = tdb_core::crc::crc32_mpeg2(&after[acc.body_start..acc.body_end]);
    let word = &after[crc.clone()];
    let actual = match endian {
        Endian::Little => u32::from_le_bytes([word[0], word[1], word[2], word[3]]),
        Endian::Big => u32::from_be_bytes([word[0], word[1], word[2], word[3]]),
    };
    if actual != expected {
        return Err(format!("covering checksum at {:#x} is {actual:#010x}, expected {expected:#010x}", acc.covering_crc_offset));
    }
    Ok(())
}

/// The checksum words of a database by file offset (`prior_crc`, `header_crc` per table, EOF,
/// file-header hash) — for tests that assert which ones moved.
pub fn checksum_offsets(db: &TdbFile) -> BTreeMap<usize, &'static str> {
    let mut m = BTreeMap::new();
    for t in &db.tables {
        m.insert(t.block_offset, "prior_crc");
        m.insert(t.block_offset + 0x24, "header_crc");
    }
    m.insert(db.bytes().len() - 4, "eof_crc");
    m.insert(0x14, "file_header_hash");
    m
}
