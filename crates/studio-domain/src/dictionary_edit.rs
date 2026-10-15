//! LOC-W1 — the workspace side of the dictionary rename: one pending **text-pointer re-point** on
//! the attached localization dictionary (the target's own file), previewed, held as a pending
//! change beside the database edits, packaged into the same copied-target change set, and adopted
//! as the new baseline only after that transaction commits (or an in-place save writes it).
//!
//! What the workspace knows is the dictionary; what it does **not** know is which game screens
//! read a key. The key ↔ league-code binding is a fact about the executable (traced on NHL Legacy
//! Xbox 360 only, see `defs/leagues/*.toml` `dictionary_key` evidence), and every preview says so.

use std::ops::Range;
use std::sync::Arc;

use tdb_txn::FileIdentity;

use crate::leagues::AttachedDictionary;
use crate::{DomainError, Result};

/// A row named in a preview, with its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DictionaryRow {
    pub record: usize,
    pub key: String,
    pub hash: u32,
    pub text: Option<String>,
}

fn row(r: &tdb_loc::RowRef) -> DictionaryRow {
    DictionaryRow {
        record: r.record,
        key: r.key.clone(),
        hash: r.hash,
        text: r.text.clone(),
    }
}

/// Everything the user sees before deciding: what changes, how each key resolved, and every
/// consequence the dictionary itself can show.
#[derive(Clone, Debug)]
pub struct DictionaryRepointPreview {
    pub dictionary_path: String,
    pub dictionary_sha256: String,
    pub endian_big: bool,
    pub source: DictionaryRow,
    /// `exact` or `by-hash` (the game's case-folded lookup; only when unambiguous).
    pub source_lookup: String,
    pub donor: DictionaryRow,
    pub donor_lookup: String,
    pub text_before: String,
    pub text_after: String,
    pub slot_offset: usize,
    pub old_pointer: u32,
    pub new_pointer: u32,
    pub hash_collisions: Vec<DictionaryRow>,
    pub sharing_old_text: Vec<DictionaryRow>,
    pub sharing_new_text: Vec<DictionaryRow>,
    /// Plain-language cautions the preview must show.
    pub warnings: Vec<String>,
}

/// The preview of an in-place free-text re-encode (LOC-W2): the new text encoded with the
/// dictionary's own Huffman tree into the bytes the old string occupies.
#[derive(Clone, Debug)]
pub struct DictionaryRetextPreview {
    pub dictionary_path: String,
    pub dictionary_sha256: String,
    pub endian_big: bool,
    pub source: DictionaryRow,
    pub source_lookup: String,
    pub text_before: String,
    pub text_after: String,
    pub pool_offset: u32,
    pub old_bits: usize,
    pub old_extent_bytes: usize,
    pub new_bits: usize,
    pub new_extent_bytes: usize,
    /// Rows pointing at the same string: they read the new text too.
    pub sharing_text: Vec<DictionaryRow>,
    pub hash_collisions: Vec<DictionaryRow>,
    pub warnings: Vec<String>,
}

/// The preview of a dictionary growth (LOC-W3, preserving serializer): longer text for existing
/// keys, new keys at their sorted positions, null texts — appended to the pool, nothing existing
/// moved or re-encoded. The file gets longer, so every placement that is same-length only (the
/// disc-image range patch) refuses it; the copied-target route re-plans the archive member.
#[derive(Clone, Debug)]
pub struct DictionaryGrowthPreview {
    pub dictionary_path: String,
    pub dictionary_sha256: String,
    pub endian_big: bool,
    /// `A` (hash order kept; insertion offered) or `B` (out-of-order rows; insertion refused).
    pub order_class: String,
    pub descents: Vec<usize>,
    pub studio_touched: bool,
    pub rows_before: usize,
    pub rows_after: usize,
    pub appended_bytes: usize,
    pub inserts: Vec<DictionaryGrowthInsert>,
    pub edits: Vec<DictionaryGrowthEdit>,
    pub warnings: Vec<String>,
    /// `preserving` (LOC-W3 P: appended strings, nothing existing moves) or `canonical-rebuild`
    /// (LOC-W3 C: the pool is repacked from the decoded rows; the tree is rebuilt when a text or
    /// key needs a symbol it lacks, or when forced).
    pub serializer: String,
    /// Canonical rebuild only: symbols the current tree lacks that the plan needs (the reason a
    /// rebuild is required); empty for a forced rebuild of an unchanged alphabet.
    pub new_symbols: Vec<String>,
    /// Canonical rebuild only, after apply: the accounting lines (tree kept/rebuilt, discarded
    /// ranges, moved strings, re-pointed records).
    pub rebuild_accounting: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct DictionaryGrowthInsert {
    pub key: String,
    pub hash: u32,
    pub text: Option<String>,
    pub position: usize,
    pub output_record: usize,
}

#[derive(Clone, Debug)]
pub struct DictionaryGrowthEdit {
    pub row: DictionaryRow,
    pub lookup: String,
    pub text_before: Option<String>,
    pub text_after: Option<String>,
    pub sharers: Vec<DictionaryRow>,
    pub all_sharers: bool,
    /// Input record indices whose pointer changes (empty = a no-op edit).
    pub records: Vec<usize>,
}

/// The one pending dictionary operation, whichever kind it is.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DictionaryPlan {
    /// LOC-W1: the key's text pointer re-pointed at another key's string.
    Repoint(tdb_loc::RepointPlan),
    /// LOC-W2: the key's own string re-encoded in place with new text.
    Retext(tdb_loc::RetextPlan),
    /// LOC-W3: growth (appended strings, inserted rows) — the plan and the ops it was derived from.
    Growth { plan: tdb_loc::GrowthPlan, ops: Vec<tdb_loc::GrowthOp> },
}

impl DictionaryPlan {
    pub fn kind(&self) -> &'static str {
        match self {
            DictionaryPlan::Repoint(_) => "repoint",
            DictionaryPlan::Retext(_) => "retext",
            DictionaryPlan::Growth { .. } => "growth",
        }
    }
    /// The row whose text changes (growth: the first edited or inserted key).
    pub fn source_key(&self) -> &str {
        match self {
            DictionaryPlan::Repoint(p) => &p.source.key,
            DictionaryPlan::Retext(p) => &p.source.key,
            DictionaryPlan::Growth { plan, .. } => plan.edits.first().map(|e| e.row.key.as_str()).or_else(|| plan.inserts.first().map(|i| i.key.as_str())).unwrap_or(""),
        }
    }
    pub fn donor_key(&self) -> Option<&str> {
        match self {
            DictionaryPlan::Repoint(p) => Some(&p.donor.key),
            DictionaryPlan::Retext(_) | DictionaryPlan::Growth { .. } => None,
        }
    }
    pub fn text_before(&self) -> String {
        match self {
            DictionaryPlan::Repoint(p) => p.source.text.clone().unwrap_or_default(),
            DictionaryPlan::Retext(p) => p.source.text.clone().unwrap_or_default(),
            DictionaryPlan::Growth { plan, .. } => plan.edits.first().and_then(|e| e.row.text.clone()).unwrap_or_default(),
        }
    }
    pub fn text_after(&self) -> String {
        match self {
            DictionaryPlan::Repoint(p) => p.donor.text.clone().unwrap_or_default(),
            DictionaryPlan::Retext(p) => p.new_text.clone(),
            DictionaryPlan::Growth { plan, .. } => plan.edits.first().and_then(|e| e.new_text.clone()).or_else(|| plan.inserts.first().and_then(|i| i.text.clone())).unwrap_or_default(),
        }
    }
    /// The producer tag for transaction records.
    pub fn producer(&self) -> &'static str {
        match self {
            DictionaryPlan::Repoint(_) => "tdb-loc::repoint",
            DictionaryPlan::Retext(_) => "tdb-loc::retext",
            DictionaryPlan::Growth { .. } => "tdb-loc::growth",
        }
    }
    /// True when the output is longer than the input (growth): same-length placements refuse it.
    pub fn grows(&self) -> bool {
        matches!(self, DictionaryPlan::Growth { plan, .. } if !plan.is_noop())
    }
    /// Re-derive the plan on `bytes` (must equal this plan) and re-apply it, returning the output
    /// bytes, the changed byte ranges and the CRC scopes that moved.
    pub fn reapply(&self, bytes: &[u8]) -> std::result::Result<(Vec<u8>, Vec<Range<usize>>, Vec<String>), String> {
        match self {
            DictionaryPlan::Repoint(p) => {
                let fresh = tdb_loc::plan_repoint(bytes, &p.requested_key, &p.requested_donor).map_err(|e| e.to_string())?;
                if &fresh != p {
                    return Err("the rename no longer plans the same way on the attached bytes".into());
                }
                let out = tdb_loc::apply_repoint(bytes, &fresh).map_err(|e| e.to_string())?;
                Ok((out.bytes, out.changed_ranges, out.crc_scopes_changed.iter().map(|s| s.to_string()).collect()))
            }
            DictionaryPlan::Retext(p) => {
                let fresh = tdb_loc::plan_retext(bytes, &p.requested_key, &p.new_text).map_err(|e| e.to_string())?;
                if &fresh != p {
                    return Err("the text edit no longer plans the same way on the attached bytes".into());
                }
                let out = tdb_loc::apply_retext(bytes, &fresh).map_err(|e| e.to_string())?;
                Ok((out.bytes, out.changed_ranges, out.crc_scopes_changed.iter().map(|s| s.to_string()).collect()))
            }
            DictionaryPlan::Growth { plan, ops } => match &plan.rebuild {
                None => {
                    let fresh = tdb_loc::plan_growth(bytes, ops).map_err(|e| e.to_string())?;
                    if &fresh != plan {
                        return Err("the growth no longer plans the same way on the attached bytes".into());
                    }
                    let out = tdb_loc::apply_growth(bytes, &fresh).map_err(|e| e.to_string())?;
                    Ok((out.bytes, out.changed_ranges, out.crc_scopes_changed.iter().map(|s| s.to_string()).collect()))
                }
                Some(rb) => {
                    let fresh = tdb_loc::plan_rebuild(bytes, ops, rb.force_tree).map_err(|e| e.to_string())?;
                    if &fresh != plan {
                        return Err("the rebuild no longer plans the same way on the attached bytes".into());
                    }
                    let out = tdb_loc::apply_rebuild(bytes, &fresh).map_err(|e| e.to_string())?;
                    let ranges = rebuild_changed_ranges(bytes, &out.bytes);
                    Ok((out.bytes, ranges, out.accounting.crc_scopes_changed.iter().map(|s| s.to_string()).collect()))
                }
            },
        }
    }
    /// Key/value detail lines for a transaction record.
    pub fn detail(&self) -> Vec<(String, String)> {
        let mut d = vec![
            ("kind".to_string(), self.kind().to_string()),
            ("key".to_string(), self.source_key().to_string()),
            ("text_before".to_string(), self.text_before()),
            ("text_after".to_string(), self.text_after()),
        ];
        match self {
            DictionaryPlan::Repoint(p) => {
                d.push(("donor_key".into(), p.donor.key.clone()));
                d.push(("slot_offset".into(), format!("{:#x}", p.slot_offset)));
                d.push(("pointer".into(), format!("{:#010x} -> {:#010x}", p.old_pointer, p.new_pointer)));
                d.push(("hash_collisions".into(), p.hash_collisions.len().to_string()));
                d.push(("sharing_old_text".into(), p.sharing_old_text.len().to_string()));
                d.push(("sharing_new_text".into(), p.sharing_new_text.len().to_string()));
            }
            DictionaryPlan::Retext(p) => {
                d.push(("pool_offset".into(), format!("{:#x}", p.text_pointer)));
                d.push(("bits".into(), format!("{} -> {} of {} bytes", p.old_bits, p.new_bits, p.old_extent_bytes)));
                d.push(("hash_collisions".into(), p.hash_collisions.len().to_string()));
                d.push(("sharing_text".into(), p.sharing_text.len().to_string()));
            }
            DictionaryPlan::Growth { plan, .. } => {
                d.push(("order_class".into(), plan.class.as_str().to_string()));
                d.push(("rows".into(), format!("{} -> {}", plan.rows_before, plan.rows_after())));
                d.push(("inserted_keys".into(), plan.inserts.iter().map(|i| i.key.clone()).collect::<Vec<_>>().join(",")));
                d.push(("edited_keys".into(), plan.edits.iter().filter(|e| !e.records.is_empty()).map(|e| e.row.key.clone()).collect::<Vec<_>>().join(",")));
                d.push(("appended_bytes".into(), plan.appended_len.to_string()));
                d.push(("loader_order_compatibility".into(), "unknown".into()));
                d.push(("serializer".into(), if plan.rebuild.is_some() { "canonical-rebuild".into() } else { "preserving".into() }));
                if let Some(rb) = &plan.rebuild {
                    d.push(("new_symbols".into(), rb.new_symbols.iter().map(|c| c.to_string()).collect::<Vec<_>>().join("")));
                    d.push(("tree_forced".into(), rb.force_tree.to_string()));
                }
            }
        }
        d
    }
}

/// One applied (not yet saved) dictionary operation: the plan, the rewritten bytes and the audit.
#[derive(Clone, Debug)]
pub struct DictionaryEdit {
    pub plan: DictionaryPlan,
    /// The re-point preview when the plan is a re-point (kept for the existing DTO).
    pub preview: DictionaryRepointPreview,
    /// The re-encode preview when the plan is a re-encode.
    pub retext_preview: Option<DictionaryRetextPreview>,
    pub output: Arc<Vec<u8>>,
    pub output_identity: FileIdentity,
    pub changed_ranges: Vec<Range<usize>>,
    pub crc_scopes_changed: Vec<String>,
}

fn warnings_for(plan: &tdb_loc::RepointPlan) -> Vec<String> {
    let mut w = Vec::new();
    w.push(
        "This changes the dictionary text under a key. Which game screens read this key (a league tile, a menu, a message) is not established by Studio; the league-code → key binding is traced only on NHL Legacy Xbox 360's executable and is a hypothesis elsewhere."
            .into(),
    );
    if plan.source_lookup == tdb_loc::Lookup::ByHash {
        w.push(format!(
            "The key {:?} is not an exact row; it resolved through the game's hash to the row keyed {:?}.",
            plan.requested_key, plan.source.key
        ));
    }
    if plan.donor_lookup == tdb_loc::Lookup::ByHash {
        w.push(format!(
            "The donor {:?} is not an exact row; it resolved through the game's hash to the row keyed {:?}.",
            plan.requested_donor, plan.donor.key
        ));
    }
    if !plan.hash_collisions.is_empty() {
        w.push(format!(
            "{} other row(s) carry the same hash as the source key; the game cannot tell them apart: {}",
            plan.hash_collisions.len(),
            plan.hash_collisions
                .iter()
                .map(|r| format!("{:?}", r.key))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !plan.sharing_old_text.is_empty() {
        w.push(format!(
            "{} other key(s) keep showing the old text {:?}: {}",
            plan.sharing_old_text.len(),
            plan.source.text.clone().unwrap_or_default(),
            plan.sharing_old_text
                .iter()
                .map(|r| format!("{:?}", r.key))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if !plan.sharing_new_text.is_empty() {
        w.push(format!(
            "{} other key(s) already show the new text (they share the donor string): {}",
            plan.sharing_new_text.len(),
            plan.sharing_new_text
                .iter()
                .map(|r| format!("{:?}", r.key))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    w
}

fn preview_of(d: &AttachedDictionary, plan: &tdb_loc::RepointPlan) -> DictionaryRepointPreview {
    DictionaryRepointPreview {
        dictionary_path: d.path.clone(),
        dictionary_sha256: d.sha256.clone(),
        endian_big: plan.endian_big,
        source: row(&plan.source),
        source_lookup: plan.source_lookup.as_str().into(),
        donor: row(&plan.donor),
        donor_lookup: plan.donor_lookup.as_str().into(),
        text_before: plan.source.text.clone().unwrap_or_default(),
        text_after: plan.donor.text.clone().unwrap_or_default(),
        slot_offset: plan.slot_offset,
        old_pointer: plan.old_pointer,
        new_pointer: plan.new_pointer,
        hash_collisions: plan.hash_collisions.iter().map(row).collect(),
        sharing_old_text: plan.sharing_old_text.iter().map(row).collect(),
        sharing_new_text: plan.sharing_new_text.iter().map(row).collect(),
        warnings: warnings_for(plan),
    }
}

/// Plan (read-only) the re-point of `key` at `donor`'s text on the attached dictionary `d`.
pub fn preview(d: &AttachedDictionary, key: &str, donor: &str) -> Result<DictionaryRepointPreview> {
    let plan = tdb_loc::plan_repoint(&d.bytes, key, donor)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))?;
    Ok(preview_of(d, &plan))
}

/// Plan and apply the re-point on `d`'s bytes (in memory). Nothing is written to disk.
pub fn apply(d: &AttachedDictionary, key: &str, donor: &str) -> Result<DictionaryEdit> {
    let plan = tdb_loc::plan_repoint(&d.bytes, key, donor)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))?;
    let out = tdb_loc::apply_repoint(&d.bytes, &plan)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))?;
    let output_identity = FileIdentity::of_bytes(&out.bytes);
    Ok(DictionaryEdit {
        preview: preview_of(d, &plan),
        retext_preview: None,
        plan: DictionaryPlan::Repoint(plan),
        output_identity,
        changed_ranges: out.changed_ranges,
        crc_scopes_changed: out.crc_scopes_changed.iter().map(|s| s.to_string()).collect(),
        output: Arc::new(out.bytes),
    })
}

fn retext_warnings(plan: &tdb_loc::RetextPlan) -> Vec<String> {
    let mut w = vec![
        "This rewrites the text stored under a key in place, using the dictionary's own compression tree; the file length and every other string stay unchanged. Which game screens read this key is not established by Studio.".to_string(),
    ];
    if plan.source_lookup == tdb_loc::Lookup::ByHash {
        w.push(format!(
            "The key {:?} is not an exact row; it resolved through the game's hash to the row keyed {:?}.",
            plan.requested_key, plan.source.key
        ));
    }
    if !plan.hash_collisions.is_empty() {
        w.push(format!(
            "{} other row(s) carry the same hash as this key; the game cannot tell them apart: {}",
            plan.hash_collisions.len(),
            plan.hash_collisions.iter().map(|r| format!("{:?}", r.key)).collect::<Vec<_>>().join(", ")
        ));
    }
    if !plan.sharing_text.is_empty() {
        w.push(format!(
            "{} other key(s) point at this same string and will read the new text too: {}",
            plan.sharing_text.len(),
            plan.sharing_text.iter().map(|r| format!("{:?}", r.key)).collect::<Vec<_>>().join(", ")
        ));
    }
    let slack = plan.old_extent_bytes * 8 - 16 - plan.new_bits;
    w.push(format!(
        "The new text uses {} of the {} bits available in place ({} spare); a longer text than fits here needs the growth path, which Studio does not offer.",
        plan.new_bits,
        plan.old_extent_bytes * 8 - 16,
        slack
    ));
    w
}

fn retext_preview_of(d: &AttachedDictionary, plan: &tdb_loc::RetextPlan) -> DictionaryRetextPreview {
    DictionaryRetextPreview {
        dictionary_path: d.path.clone(),
        dictionary_sha256: d.sha256.clone(),
        endian_big: plan.endian_big,
        source: row(&plan.source),
        source_lookup: plan.source_lookup.as_str().into(),
        text_before: plan.source.text.clone().unwrap_or_default(),
        text_after: plan.new_text.clone(),
        pool_offset: plan.text_pointer,
        old_bits: plan.old_bits,
        old_extent_bytes: plan.old_extent_bytes,
        new_bits: plan.new_bits,
        new_extent_bytes: plan.new_extent_bytes,
        sharing_text: plan.sharing_text.iter().map(row).collect(),
        hash_collisions: plan.hash_collisions.iter().map(row).collect(),
        warnings: retext_warnings(plan),
    }
}

/// Plan (read-only) the in-place re-encode of `key`'s text to `text` (LOC-W2).
pub fn preview_retext(d: &AttachedDictionary, key: &str, text: &str) -> Result<DictionaryRetextPreview> {
    let plan = tdb_loc::plan_retext(&d.bytes, key, text)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))?;
    Ok(retext_preview_of(d, &plan))
}

/// Plan and apply the re-encode on `d`'s bytes (in memory). Nothing is written to disk.
pub fn apply_retext(d: &AttachedDictionary, key: &str, text: &str) -> Result<DictionaryEdit> {
    let plan = tdb_loc::plan_retext(&d.bytes, key, text)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))?;
    let out = tdb_loc::apply_retext(&d.bytes, &plan)
        .map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))?;
    let output_identity = FileIdentity::of_bytes(&out.bytes);
    let rp = retext_preview_of(d, &plan);
    // The legacy preview slot carries the same source/texts so existing consumers keep working.
    let preview = DictionaryRepointPreview {
        dictionary_path: rp.dictionary_path.clone(),
        dictionary_sha256: rp.dictionary_sha256.clone(),
        endian_big: rp.endian_big,
        source: rp.source.clone(),
        source_lookup: rp.source_lookup.clone(),
        donor: rp.source.clone(),
        donor_lookup: "in-place".into(),
        text_before: rp.text_before.clone(),
        text_after: rp.text_after.clone(),
        slot_offset: 0,
        old_pointer: plan.text_pointer,
        new_pointer: plan.text_pointer,
        hash_collisions: rp.hash_collisions.clone(),
        sharing_old_text: Vec::new(),
        sharing_new_text: rp.sharing_text.clone(),
        warnings: rp.warnings.clone(),
    };
    Ok(DictionaryEdit {
        preview,
        retext_preview: Some(rp),
        plan: DictionaryPlan::Retext(plan),
        output_identity,
        changed_ranges: out.changed_ranges,
        crc_scopes_changed: out.crc_scopes_changed.iter().map(|s| s.to_string()).collect(),
        output: Arc::new(out.bytes),
    })
}

/// The byte ranges a canonical rebuild changes: the header words the rebuild may touch plus the
/// whole record block and pool (the layout is repacked, so nothing after the record block is a
/// verbatim copy at its old position).
fn rebuild_changed_ranges(input: &[u8], output: &[u8]) -> Vec<std::ops::Range<usize>> {
    let mut ranges = vec![0x08..0x0C, 0x14..0x18];
    if let Ok(db) = tdb_core::TdbFile::parse(input) {
        if let Some(t) = db.tables.first() {
            ranges.push(t.block_offset..t.block_offset + 4);
            ranges.push(t.block_offset + 0x14..t.block_offset + 0x18);
            ranges.push(t.block_offset + 0x24..t.block_offset + 0x28);
            ranges.push(t.record_block..output.len());
            return ranges;
        }
    }
    ranges.push(0..output.len());
    ranges
}

fn growth_warnings(plan: &tdb_loc::GrowthPlan) -> Vec<String> {
    let mut w = match &plan.rebuild {
        None => vec![
            format!(
                "This grows the dictionary: {} row(s) inserted, {} text(s) appended ({} bytes); every existing string, the compression tree and the row order stay exactly as they are. The file gets longer, so it cannot be written through a same-length route (a disc-image patch); a packed archive member is re-planned by the copied-target save.",
                plan.inserts.len(),
                plan.edits.iter().filter(|e| !e.records.is_empty()).count(),
                plan.appended_len
            ),
        ],
        Some(rb) => vec![
            format!(
                "This REBUILDS the dictionary: {} row(s) inserted, {} text(s) changed; the whole string pool is repacked from the decoded rows in the game's own order (existing shared strings stay shared, empty texts stay empty) and {}. Every existing row keeps its key, hash and order; only the pool layout changes. The file length changes, so it cannot be written through a same-length route (a disc-image patch); a packed archive member is re-planned by the copied-target save.",
                plan.inserts.len(),
                plan.edits.iter().filter(|e| !e.records.is_empty()).count(),
                if rb.new_symbols.is_empty() { "the compression tree is rebuilt as requested".to_string() } else { format!("the compression tree is rebuilt because it has no symbol for {}", rb.new_symbols.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", ")) }
            ),
            "Whether the game accepts a rebuilt string pool and compression tree is not established by Studio (runtime gate); a new symbol may also lack a glyph in the game's fonts even though the dictionary now encodes it (glyph gate) — only a run shows either.".to_string(),
        ],
    };
    w.push("Whether the game accepts a larger dictionary and reads an appended string (runtime gate R-A), and whether it finds a newly inserted key (gate R-B), is not established by Studio; the loader's tolerance of row order is unknown.".to_string());
    w.extend(plan.notes.iter().cloned());
    w
}

fn growth_preview_of(d: &AttachedDictionary, plan: &tdb_loc::GrowthPlan) -> DictionaryGrowthPreview {
    let (order_class, descents) = match &plan.class {
        tdb_loc::OrderClass::A => ("A".to_string(), Vec::new()),
        tdb_loc::OrderClass::B { descents } => ("B".to_string(), descents.clone()),
    };
    DictionaryGrowthPreview {
        dictionary_path: d.path.clone(),
        dictionary_sha256: d.sha256.clone(),
        endian_big: plan.endian_big,
        order_class,
        descents,
        studio_touched: plan.studio_touched,
        rows_before: plan.rows_before,
        rows_after: plan.rows_after(),
        appended_bytes: plan.appended_len,
        inserts: plan.inserts.iter().map(|i| DictionaryGrowthInsert { key: i.key.clone(), hash: i.hash, text: i.text.clone(), position: i.position, output_record: i.output_record }).collect(),
        edits: plan.edits.iter().map(|e| DictionaryGrowthEdit { row: row(&e.row), lookup: e.lookup.as_str().into(), text_before: e.row.text.clone(), text_after: e.new_text.clone(), sharers: e.sharers.iter().map(row).collect(), all_sharers: e.all_sharers, records: e.records.clone() }).collect(),
        warnings: growth_warnings(plan),
        serializer: if plan.rebuild.is_some() { "canonical-rebuild".into() } else { "preserving".into() },
        new_symbols: plan.rebuild.as_ref().map(|r| r.new_symbols.iter().map(|c| c.to_string()).collect()).unwrap_or_default(),
        rebuild_accounting: Vec::new(),
    }
}

/// How a growth is serialized: the preserving serializer (default) or the canonical rebuild
/// (explicitly chosen; the only path for a text/key that needs a new symbol).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrowthSerializer {
    Preserving,
    /// `force_tree`: rebuild the tree even when the alphabet did not grow.
    CanonicalRebuild { force_tree: bool },
}

fn plan_for(d: &AttachedDictionary, ops: &[tdb_loc::GrowthOp], how: GrowthSerializer) -> Result<tdb_loc::GrowthPlan> {
    match how {
        GrowthSerializer::Preserving => tdb_loc::plan_growth(&d.bytes, ops),
        GrowthSerializer::CanonicalRebuild { force_tree } => tdb_loc::plan_rebuild(&d.bytes, ops, force_tree),
    }
    .map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))
}

/// Plan (read-only) a growth of the attached dictionary (LOC-W3).
pub fn preview_growth(d: &AttachedDictionary, ops: &[tdb_loc::GrowthOp]) -> Result<DictionaryGrowthPreview> {
    preview_growth_with(d, ops, GrowthSerializer::Preserving)
}

/// [`preview_growth`] with an explicit serializer.
pub fn preview_growth_with(d: &AttachedDictionary, ops: &[tdb_loc::GrowthOp], how: GrowthSerializer) -> Result<DictionaryGrowthPreview> {
    let plan = plan_for(d, ops, how)?;
    Ok(growth_preview_of(d, &plan))
}

/// Plan and apply a growth on `d`'s bytes (in memory). Nothing is written to disk.
pub fn apply_growth(d: &AttachedDictionary, ops: &[tdb_loc::GrowthOp]) -> Result<(DictionaryEdit, DictionaryGrowthPreview)> {
    apply_growth_with(d, ops, GrowthSerializer::Preserving)
}

/// [`apply_growth`] with an explicit serializer.
pub fn apply_growth_with(d: &AttachedDictionary, ops: &[tdb_loc::GrowthOp], how: GrowthSerializer) -> Result<(DictionaryEdit, DictionaryGrowthPreview)> {
    let plan = plan_for(d, ops, how)?;
    if plan.is_noop() && !matches!(how, GrowthSerializer::CanonicalRebuild { force_tree: true }) {
        return Err(DomainError::Validation(format!("{}: nothing to change", d.path)));
    }
    let (out_bytes, changed_ranges, crc_scopes_changed, accounting): (Vec<u8>, Vec<std::ops::Range<usize>>, Vec<&'static str>, Vec<String>) = match how {
        GrowthSerializer::Preserving => {
            let out = tdb_loc::apply_growth(&d.bytes, &plan).map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))?;
            (out.bytes, out.changed_ranges, out.crc_scopes_changed, Vec::new())
        }
        GrowthSerializer::CanonicalRebuild { .. } => {
            let out = tdb_loc::apply_rebuild(&d.bytes, &plan).map_err(|e| DomainError::Validation(format!("{}: {e}", d.path)))?;
            let ranges = rebuild_changed_ranges(&d.bytes, &out.bytes);
            let a = &out.accounting;
            let mut lines = vec![
                if a.tree_rebuilt { format!("compression tree rebuilt: {} -> {} bytes", a.tree_bytes_before, a.tree_bytes_after) } else { "compression tree kept (alphabet unchanged)".to_string() },
                format!("string pool repacked: {} -> {} bytes; {} string(s) moved; {} record(s) re-pointed; {} inserted", a.pool_len_before, a.pool_len_after, a.moved_strings, a.repointed_records, a.inserted_records),
            ];
            for dr in &a.discarded {
                lines.push(format!("discarded {} bytes of {} at pool offset {}", dr.range.end - dr.range.start, dr.kind, dr.range.start));
            }
            if a.pool_prefix_identical {
                lines.push("the repacked pool equals the original pool up to its trailing slack (nothing moved)".to_string());
            }
            (out.bytes, ranges, a.crc_scopes_changed.clone(), lines)
        }
    };
    let out = tdb_loc::GrowthOutput { bytes: out_bytes, changed_ranges, inserted_records: Vec::new(), appended: 0..0, pool_shift: 0, crc_scopes_changed };
    let output_identity = FileIdentity::of_bytes(&out.bytes);
    let mut gp = growth_preview_of(d, &plan);
    gp.rebuild_accounting = accounting;
    let first = plan.edits.first().map(|e| row(&e.row)).or_else(|| plan.inserts.first().map(|i| DictionaryRow { record: i.output_record, key: i.key.clone(), hash: i.hash, text: i.text.clone() })).unwrap_or(DictionaryRow { record: 0, key: String::new(), hash: 0, text: None });
    let preview = DictionaryRepointPreview {
        dictionary_path: d.path.clone(),
        dictionary_sha256: d.sha256.clone(),
        endian_big: plan.endian_big,
        source: first.clone(),
        source_lookup: "growth".into(),
        donor: first.clone(),
        donor_lookup: "appended".into(),
        text_before: plan.edits.first().and_then(|e| e.row.text.clone()).unwrap_or_default(),
        text_after: plan.edits.first().and_then(|e| e.new_text.clone()).or_else(|| plan.inserts.first().and_then(|i| i.text.clone())).unwrap_or_default(),
        slot_offset: 0,
        old_pointer: 0,
        new_pointer: 0,
        hash_collisions: Vec::new(),
        sharing_old_text: Vec::new(),
        sharing_new_text: Vec::new(),
        warnings: gp.warnings.clone(),
    };
    let edit = DictionaryEdit {
        preview,
        retext_preview: None,
        plan: DictionaryPlan::Growth { plan, ops: ops.to_vec() },
        output_identity,
        changed_ranges: out.changed_ranges,
        crc_scopes_changed: out.crc_scopes_changed.iter().map(|s| s.to_string()).collect(),
        output: Arc::new(out.bytes),
    };
    Ok((edit, gp))
}

/// The pending-review view of a dictionary edit (the analogue of a changed cell).
#[derive(Clone, Debug)]
pub struct PendingDictionary {
    pub path: String,
    pub baseline_sha256: String,
    pub working_sha256: String,
    /// `repoint` (LOC-W1), `retext` (LOC-W2) or `growth` (LOC-W3).
    pub kind: String,
    pub key: String,
    pub before: String,
    pub after: String,
    pub donor_key: String,
    pub changed_bytes: usize,
    pub changed_ranges: Vec<Range<usize>>,
    pub crc_scopes_changed: Vec<String>,
    pub warnings: Vec<String>,
}

impl DictionaryEdit {
    pub fn pending(&self, baseline: &AttachedDictionary) -> PendingDictionary {
        PendingDictionary {
            path: baseline.path.clone(),
            baseline_sha256: baseline.sha256.clone(),
            working_sha256: self.output_identity.sha256.clone(),
            kind: self.plan.kind().to_string(),
            key: self.plan.source_key().to_string(),
            before: self.plan.text_before(),
            after: self.plan.text_after(),
            donor_key: self.plan.donor_key().unwrap_or("").to_string(),
            changed_bytes: self.changed_ranges.iter().map(|r| r.end - r.start).sum(),
            changed_ranges: self.changed_ranges.clone(),
            crc_scopes_changed: self.crc_scopes_changed.clone(),
            warnings: self.preview.warnings.clone(),
        }
    }
}
