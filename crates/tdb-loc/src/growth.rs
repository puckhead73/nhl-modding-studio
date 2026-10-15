//! LOC-W3 — **dictionary growth** with the preserving serializer (research handoff 20 rev. 3,
//! accepted as an implementation contract on 2026-09-18): longer text for an existing key (O-T),
//! a new key (O-K, class A inputs only), and null text (O-N), all by *appending* to the pool and
//! inserting records — never by reordering or re-encoding anything that exists.
//!
//! What the output is, exactly (§4-P of the handoff): the input with `k` inserted 16-byte records
//! at their deterministic positions (§3: a new row goes after every existing row whose hash is
//! smaller; existing rows keep their relative order; equal-hash runs stay intact because a new
//! hash equal to any existing hash is refused), the edited rows' 4-byte text pointers, the
//! **entire original pool copied verbatim** (tree, every string, unreachable storage, trailing
//! slack) moved by `16k`, the new strings appended after the original pool's physical end, and the
//! header words that describe the new size (`db_size`, `max_records`, `current_records`) with the
//! four CRC words recomputed. Nothing else. LOC-W1's shared pointers and LOC-W2's zeroed slack
//! survive untouched, so both are valid inputs.
//!
//! Refusals (§2): inserting a key that exists exactly (the caller wanted O-T); inserting a key
//! whose hash equals **any** existing row's hash (the full multiset, shipped duplicates included);
//! any insertion into a class-B input (at least one hash descent — NHL 08/09/10, `lng_lg`; the
//! loader's order tolerance is unknown, so the position is undefined); keys > 100 B or texts >
//! 4 000 B decoded; a NUL in a key; a text or key needing a symbol the tree lacks (that is the
//! canonical-rebuild path O-G, not offered here — the refusal lists the symbols); a file that fails
//! `verify`. Nothing here establishes that a loader accepts a larger file or finds a new key
//! (runtime gates R-A / R-B); every plan records `loader-order-compatibility: unknown`.

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use sha2::{Digest, Sha256};
use tdb_core::{Endian, TdbFile};

use crate::repoint::{resolve, RowRef};
use crate::{loc_hash, HuffmanTree, LocDictionary, LocError, Lookup, NULL_PTR};

const KIND_KEY: u32 = 13;
const KIND_TEXT: u32 = 14;
const RECORD_LEN: usize = 16;

/// One requested change.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GrowthOp {
    /// Existing key, new (typically longer) text appended to the pool; when the row's current
    /// string is shared with other rows (LOC-W1), `all_sharers` decides whether they follow
    /// (default `false`: this row only — the others keep the old string).
    TextGrow { key: String, new_text: String, all_sharers: bool },
    /// A new row. `text: None` stores a null text pointer (O-N).
    InsertKey { key: String, text: Option<String> },
    /// An existing row's text pointer set to null (O-N on an existing row). A no-op when the row
    /// is already null.
    SetNull { key: String },
}

/// How a plan is made: for the preserving serializer (missing symbols refuse with
/// `needs-rebuild`) or for the canonical rebuild (`rebuild.rs`: missing symbols are collected and
/// the tree is rebuilt at apply; `force_tree` rebuilds it even when the alphabet did not grow).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlanMode {
    Preserve,
    Rebuild { force_tree: bool },
}

/// What a rebuild plan adds to the growth plan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebuildInfo {
    pub force_tree: bool,
    /// Symbols the input tree lacks that the plan's texts/keys need (empty when the rebuild was
    /// only forced).
    pub new_symbols: Vec<char>,
}

pub(crate) fn refuse_pub(kind: &'static str, reason: impl Into<String>) -> GrowthRefusal {
    refuse(kind, reason)
}

pub(crate) fn sha256_hex_pub(b: &[u8]) -> String {
    sha256_hex(b)
}

/// Why a plan was refused (nothing written). `kind` is stable for callers:
/// `exists` / `hash-collision` / `baseline-order-anomalous` / `capacity` / `needs-rebuild` /
/// `not-found` / `nothing-to-change` / `invalid` / `verify`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowthRefusal {
    pub kind: &'static str,
    pub reason: String,
}

impl std::fmt::Display for GrowthRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.kind, self.reason)
    }
}
impl std::error::Error for GrowthRefusal {}

fn refuse(kind: &'static str, reason: impl Into<String>) -> GrowthRefusal {
    GrowthRefusal { kind, reason: reason.into() }
}

impl From<LocError> for GrowthRefusal {
    fn from(e: LocError) -> Self {
        refuse("invalid", e.to_string())
    }
}

/// The input's order class (§2): A = nondecreasing hashes (equal runs allowed); B = at least one
/// descent, listed by the record index at which the hash drops.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderClass {
    A,
    B { descents: Vec<usize> },
}

impl OrderClass {
    pub fn of(rows: &[crate::LocRow]) -> Self {
        let descents: Vec<usize> = (1..rows.len()).filter(|&i| rows[i].hash < rows[i - 1].hash).collect();
        if descents.is_empty() {
            OrderClass::A
        } else {
            OrderClass::B { descents }
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            OrderClass::A => "A",
            OrderClass::B { .. } => "B",
        }
    }
}

/// A planned insertion: the new row and where it goes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedInsert {
    pub key: String,
    pub hash: u32,
    pub text: Option<String>,
    /// Number of existing rows with a smaller hash = the row's position among the existing rows.
    pub position: usize,
    /// The row's index in the output record block.
    pub output_record: usize,
    /// Encoded key (1-byte prefix + bits) and text (2-byte prefix + bits; `None` = null pointer).
    pub key_encoded: Vec<u8>,
    pub text_encoded: Option<Vec<u8>>,
}

/// A planned text change on an existing row (O-T or O-N).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PlannedTextEdit {
    pub row: RowRef,
    pub lookup: Lookup,
    pub new_text: Option<String>,
    /// Encoded new text (`None` for a null pointer).
    pub text_encoded: Option<Vec<u8>>,
    /// Other rows currently pointing at the same string (LOC-W1 sharing), reported before mutation.
    pub sharers: Vec<RowRef>,
    /// Whether the sharers take the new text too.
    pub all_sharers: bool,
    /// Input record indices whose text pointer changes (`row` and, with `all_sharers`, the sharers).
    pub records: Vec<usize>,
}

/// Everything the write will do, computed read-only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GrowthPlan {
    pub source_sha256: String,
    pub source_len: usize,
    pub endian_big: bool,
    pub class: OrderClass,
    /// The input was the output of LOC-W1 (shared text pointers) and/or LOC-W2 (in-place re-text);
    /// informational.
    pub studio_touched: bool,
    pub inserts: Vec<PlannedInsert>,
    pub edits: Vec<PlannedTextEdit>,
    /// Absolute offset of the original pool and its length (tree + strings + slack).
    pub pool_start: usize,
    pub pool_len: usize,
    pub tree_bytes: usize,
    pub record_block: usize,
    pub rows_before: usize,
    /// Bytes appended after the original pool, in order: for each insert `[key][text]` (gen7) or
    /// `[text][key]` (PS4), then each edit's text.
    pub appended_len: usize,
    pub notes: Vec<String>,
    /// `Some` when the plan was made for the canonical rebuild serializer (`apply_rebuild`);
    /// `apply_growth` refuses such a plan. The encoded fields of inserts/edits are empty then
    /// (the strings are encoded with the final tree at apply).
    pub rebuild: Option<RebuildInfo>,
}

impl GrowthPlan {
    pub fn is_noop(&self) -> bool {
        self.inserts.is_empty() && self.edits.iter().all(|e| e.records.is_empty())
    }
    pub fn rows_after(&self) -> usize {
        self.rows_before + self.inserts.len()
    }
}

/// The grown bytes plus the accounting of what changed.
#[derive(Clone, Debug)]
pub struct GrowthOutput {
    pub bytes: Vec<u8>,
    /// Every output byte range that is not a verbatim copy of input bytes at the corresponding
    /// (shifted) position: the size words, the CRC words, the inserted records, the re-pointed
    /// slots and the appended strings. The moved pool and the moved tail of the record block are
    /// **not** listed (their content is identical); `pool_shift` says by how much they moved.
    pub changed_ranges: Vec<Range<usize>>,
    pub inserted_records: Vec<usize>,
    pub appended: Range<usize>,
    pub pool_shift: usize,
    pub crc_scopes_changed: Vec<&'static str>,
}

fn sha256_hex(b: &[u8]) -> String {
    Sha256::digest(b).iter().map(|x| format!("{x:02x}")).collect()
}

fn read_u32(bytes: &[u8], at: usize, endian: Endian) -> u32 {
    let a = [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
    match endian {
        Endian::Big => u32::from_be_bytes(a),
        Endian::Little => u32::from_le_bytes(a),
    }
}

fn write_u32(bytes: &mut [u8], at: usize, v: u32, endian: Endian) {
    let a = match endian {
        Endian::Big => v.to_be_bytes(),
        Endian::Little => v.to_le_bytes(),
    };
    bytes[at..at + 4].copy_from_slice(&a);
}

fn write_u16(bytes: &mut [u8], at: usize, v: u16, endian: Endian) {
    let a = match endian {
        Endian::Big => v.to_be_bytes(),
        Endian::Little => v.to_le_bytes(),
    };
    bytes[at..at + 2].copy_from_slice(&a);
}

/// Encode `s` with the tree: `prefix` bytes of decoded length (1 = u8, 2 = u16 BE) then the bit
/// stream MSB-first, zero-padded to a byte. Missing symbols are returned instead.
fn encode(codes: &HashMap<u8, Vec<bool>>, s: &str, prefix: usize) -> Result<Vec<u8>, Vec<char>> {
    let mut missing: Vec<char> = Vec::new();
    let mut bits = 0usize;
    for ch in s.chars() {
        let mut buf = [0u8; 4];
        for b in ch.encode_utf8(&mut buf).bytes() {
            match codes.get(&b) {
                Some(c) => bits += c.len(),
                None => {
                    if !missing.contains(&ch) {
                        missing.push(ch);
                    }
                }
            }
        }
    }
    if !missing.is_empty() {
        return Err(missing);
    }
    // EA's stored length is `bits / 8 + 1` (one byte past the last full byte; measured on every
    // committed dictionary — see `rebuild::stream_bytes`), so appended strings are EA-shaped.
    let mut out = vec![0u8; prefix + crate::rebuild::stream_bytes(bits)];
    match prefix {
        1 => out[0] = s.len() as u8,
        _ => out[0..2].copy_from_slice(&(s.len() as u16).to_be_bytes()),
    }
    let mut bit = prefix * 8;
    for b in s.bytes() {
        for &one in &codes[&b] {
            if one {
                out[bit >> 3] |= 0x80 >> (bit & 7);
            }
            bit += 1;
        }
    }
    Ok(out)
}

struct Layout<'a> {
    db: TdbFile<'a>,
    stride: usize,
    key_off: usize,
    text_off: usize,
    hash_off: usize,
    key_cap: usize,
    text_cap: usize,
    pool_start: usize,
    pool: &'a [u8],
}

fn layout(bytes: &[u8]) -> Result<Layout<'_>, GrowthRefusal> {
    let db = TdbFile::parse(bytes).map_err(|e| refuse("invalid", e.to_string()))?;
    if db.tables.len() != 1 {
        return Err(refuse("invalid", "not a single-table dictionary"));
    }
    let t = &db.tables[0];
    let stride = t.record_len_bytes as usize;
    if stride != RECORD_LEN {
        return Err(refuse("invalid", format!("record length {stride} is not the measured 16")));
    }
    let f = |kind: u32| t.fields.iter().find(|f| f.kind == kind).ok_or_else(|| refuse("invalid", format!("no field of kind {kind}")));
    let (kf, tf, hf) = (f(KIND_KEY)?, f(KIND_TEXT)?, f(3)?);
    let pool_start = t.record_block + t.max_records as usize * stride;
    if bytes.len() < 4 || pool_start > bytes.len() - 4 {
        return Err(refuse("invalid", "pool start beyond the file"));
    }
    Ok(Layout {
        stride,
        key_off: (kf.bit_offset / 8) as usize,
        text_off: (tf.bit_offset / 8) as usize,
        hash_off: (hf.bit_offset / 8) as usize,
        key_cap: (kf.bit_width / 8) as usize,
        text_cap: (tf.bit_width / 8) as usize,
        pool_start,
        pool: &bytes[pool_start..bytes.len() - 4],
        db,
    })
}

/// Plan `ops` on `bytes` (read-only). Every op is validated before any is planned; the first
/// refusal wins and names its reason.
pub fn plan_growth(bytes: &[u8], ops: &[GrowthOp]) -> Result<GrowthPlan, GrowthRefusal> {
    plan_growth_with(bytes, ops, PlanMode::Preserve)
}

/// [`plan_growth`] in an explicit mode (see [`PlanMode`]).
pub fn plan_growth_with(bytes: &[u8], ops: &[GrowthOp], mode: PlanMode) -> Result<GrowthPlan, GrowthRefusal> {
    if ops.is_empty() {
        // An empty plan is legal: the preserving serializer's no-op proof (P1).
    }
    let rebuild_mode = matches!(mode, PlanMode::Rebuild { .. });
    // In rebuild mode a missing symbol is collected instead of refusing; the encoding is left
    // empty (the final tree is not known until apply).
    let mut new_symbols: Vec<char> = Vec::new();
    for c in crate::verify(bytes).map_err(|e| refuse("verify", e.to_string()))? {
        if !c.matches() {
            return Err(refuse("verify", format!("CRC scope {} stored {:08x} computed {:08x}", c.name, c.stored, c.computed)));
        }
    }
    let d = LocDictionary::parse(bytes)?;
    let l = layout(bytes)?;
    let t = &l.db.tables[0];
    if t.max_records != t.current_records {
        return Err(refuse("invalid", format!("max_records {} != current_records {} (spare slots are not part of the measured layout)", t.max_records, t.current_records)));
    }
    let tree = HuffmanTree::parse(l.pool)?;
    let codes = tree.codes();
    let mut enc = |s: &str, prefix: usize| -> Result<Vec<u8>, GrowthRefusal> {
        match encode(&codes, s, prefix) {
            Ok(v) => Ok(v),
            Err(missing) if rebuild_mode => {
                for c in missing {
                    if !new_symbols.contains(&c) {
                        new_symbols.push(c);
                    }
                }
                Ok(Vec::new())
            }
            Err(missing) => Err(needs_rebuild(s, &missing)),
        }
    };
    let class = OrderClass::of(&d.rows);
    let ptr_of = |i: usize, off: usize| read_u32(bytes, t.record_block + i * l.stride + off, l.db.endian);
    // Studio-touched detection (informational): shared text pointers (LOC-W1) — EA never ships them.
    let mut seen_ptr: HashMap<u32, usize> = HashMap::new();
    let mut shared = false;
    for i in 0..d.rows.len() {
        let p = ptr_of(i, l.text_off);
        if p != NULL_PTR && seen_ptr.insert(p, i).is_some() {
            shared = true;
        }
    }
    let row_ref = |i: usize| RowRef { record: i, key: d.rows[i].key.clone().unwrap_or_default(), hash: d.rows[i].hash, text: d.rows[i].text.clone() };
    let hashes: BTreeMap<u32, Vec<usize>> = d.rows.iter().enumerate().fold(BTreeMap::new(), |mut m, (i, r)| {
        m.entry(r.hash).or_default().push(i);
        m
    });

    let mut inserts: Vec<PlannedInsert> = Vec::new();
    let mut edits: Vec<PlannedTextEdit> = Vec::new();
    let mut edited_records: Vec<usize> = Vec::new();
    for op in ops {
        match op {
            GrowthOp::InsertKey { key, text } => {
                if key.is_empty() || key.as_bytes().contains(&0) {
                    return Err(refuse("invalid", format!("key {key:?} is empty or contains a NUL byte")));
                }
                if key.len() > l.key_cap {
                    return Err(refuse("capacity", format!("key {key:?} is {} bytes; the field allows {} decoded bytes", key.len(), l.key_cap)));
                }
                if let Some(t) = text {
                    if t.len() > l.text_cap {
                        return Err(refuse("capacity", format!("text for {key:?} is {} bytes; the field allows {} decoded bytes", t.len(), l.text_cap)));
                    }
                }
                if d.contains_key(key) {
                    return Err(refuse("exists", format!("key {key:?} already exists (record {}); change its text instead of inserting it", d.rows.iter().position(|r| r.key.as_deref() == Some(key)).unwrap_or(0))));
                }
                let h = loc_hash(key);
                if let Some(rows) = hashes.get(&h) {
                    let names: Vec<String> = rows.iter().map(|&i| format!("record {i} {:?}", d.rows[i].key.clone().unwrap_or_default())).collect();
                    return Err(refuse("hash-collision", format!("key {key:?} hashes to {h:#010x}, the hash of {}; a second row with the same hash is not inserted (row identity is the hash)", names.join(", "))));
                }
                if inserts.iter().any(|p| p.hash == h) {
                    return Err(refuse("hash-collision", format!("two new keys hash to {h:#010x} ({key:?} and another); not inserted")));
                }
                if let OrderClass::B { descents } = &class {
                    return Err(refuse("baseline-order-anomalous", format!(
                        "this dictionary's rows are not in hash order (descents at records {}); the loader's order tolerance is unknown, so a new row has no defined position — insertion is not offered on it (text changes are)",
                        descents.iter().take(12).map(usize::to_string).collect::<Vec<_>>().join(", ")
                    )));
                }
                let key_encoded = enc(key, 1)?;
                let text_encoded = match text {
                    Some(t) => Some(enc(t, 2)?),
                    None => None,
                };
                let position = d.rows.iter().filter(|r| r.hash < h).count();
                inserts.push(PlannedInsert { key: key.clone(), hash: h, text: text.clone(), position, output_record: 0, key_encoded, text_encoded });
            }
            GrowthOp::TextGrow { key, new_text, all_sharers } => {
                if new_text.len() > l.text_cap {
                    return Err(refuse("capacity", format!("text for {key:?} is {} bytes; the field allows {} decoded bytes", new_text.len(), l.text_cap)));
                }
                let (si, lookup) = resolve(&d, key).map_err(|e| refuse("not-found", e.to_string()))?;
                if edited_records.contains(&si) {
                    return Err(refuse("invalid", format!("record {si} is edited twice in one plan")));
                }
                if d.rows[si].text.as_deref() == Some(new_text.as_str()) {
                    return Err(refuse("nothing-to-change", format!("{key:?} already reads {new_text:?}")));
                }
                let text_encoded = enc(new_text, 2)?;
                let p = ptr_of(si, l.text_off);
                let sharers: Vec<RowRef> = if p == NULL_PTR { Vec::new() } else { (0..d.rows.len()).filter(|&i| i != si && ptr_of(i, l.text_off) == p).map(row_ref).collect() };
                let mut records = vec![si];
                if *all_sharers {
                    records.extend(sharers.iter().map(|r| r.record));
                }
                edited_records.extend(records.iter().copied());
                edits.push(PlannedTextEdit { row: row_ref(si), lookup, new_text: Some(new_text.clone()), text_encoded: Some(text_encoded), sharers, all_sharers: *all_sharers, records });
            }
            GrowthOp::SetNull { key } => {
                let (si, lookup) = resolve(&d, key).map_err(|e| refuse("not-found", e.to_string()))?;
                if edited_records.contains(&si) {
                    return Err(refuse("invalid", format!("record {si} is edited twice in one plan")));
                }
                let p = ptr_of(si, l.text_off);
                let sharers: Vec<RowRef> = if p == NULL_PTR { Vec::new() } else { (0..d.rows.len()).filter(|&i| i != si && ptr_of(i, l.text_off) == p).map(row_ref).collect() };
                // Already null: a no-op edit (kept in the plan with no records so the report says so).
                let records = if p == NULL_PTR { Vec::new() } else { vec![si] };
                edited_records.extend(records.iter().copied());
                edits.push(PlannedTextEdit { row: row_ref(si), lookup, new_text: None, text_encoded: None, sharers, all_sharers: false, records });
            }
        }
    }
    // Deterministic order: inserts by hash; output record index = position + (inserts before it).
    inserts.sort_by_key(|p| p.hash);
    for i in 0..inserts.len() {
        inserts[i].output_record = inserts[i].position + i;
    }
    if d.rows.len() + inserts.len() > u16::MAX as usize {
        return Err(refuse("capacity", "the record count would exceed the 16-bit table limit"));
    }
    let appended_len: usize = inserts.iter().map(|p| p.key_encoded.len() + p.text_encoded.as_ref().map_or(0, Vec::len)).sum::<usize>()
        + edits.iter().map(|e| if e.records.is_empty() { 0 } else { e.text_encoded.as_ref().map_or(0, Vec::len) }).sum::<usize>();
    let mut notes = vec!["loader-order-compatibility: unknown (no evidence says which lookup the game uses; existing rows are never reordered)".to_string()];
    for e in &edits {
        if !e.sharers.is_empty() {
            notes.push(format!(
                "{:?}'s current string is shared with {} other row(s) ({}); {}",
                e.row.key,
                e.sharers.len(),
                e.sharers.iter().map(|s| format!("record {} {:?}", s.record, s.key)).collect::<Vec<_>>().join(", "),
                if e.all_sharers { "they take the new text too" } else { "they keep the old string" }
            ));
        }
    }
    if shared {
        notes.push("the input carries shared text pointers (a LOC-W1 output); they are preserved except where this plan re-points a row".to_string());
    }
    let rebuild = match mode {
        PlanMode::Preserve => None,
        PlanMode::Rebuild { force_tree } => {
            notes.push(if new_symbols.is_empty() {
                "canonical rebuild requested: the pool is repacked in per-row order (no slack); the tree is kept unless forced".to_string()
            } else {
                format!("canonical rebuild: the tree lacks {} and is rebuilt from the full symbol histogram; the pool is repacked in per-row order", new_symbols.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", "))
            });
            notes.push("new-glyph runtime gate: a rebuilt tree may still lack a font glyph for a new symbol; only a run shows it".to_string());
            Some(RebuildInfo { force_tree, new_symbols: new_symbols.clone() })
        }
    };
    Ok(GrowthPlan {
        source_sha256: sha256_hex(bytes),
        source_len: bytes.len(),
        endian_big: l.db.endian == Endian::Big,
        class,
        studio_touched: shared,
        inserts,
        edits,
        pool_start: l.pool_start,
        pool_len: l.pool.len(),
        tree_bytes: tree.tree_bytes,
        record_block: t.record_block,
        rows_before: d.rows.len(),
        appended_len,
        notes,
        rebuild,
    })
}

fn needs_rebuild(s: &str, missing: &[char]) -> GrowthRefusal {
    refuse("needs-rebuild", format!(
        "this dictionary's Huffman tree has no symbol for {} (in {s:?}); adding one means rebuilding the whole pool (the canonical-rebuild path), which is not part of this operation",
        missing.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", ")
    ))
}

/// Apply `plan` to `bytes` (the bytes it was made from) with the preserving serializer and verify
/// the result independently (re-parse, CRC scopes, decoded rows against the requested result).
pub fn apply_growth(bytes: &[u8], plan: &GrowthPlan) -> Result<GrowthOutput, GrowthRefusal> {
    if plan.rebuild.is_some() {
        return Err(refuse("invalid", "this plan was made for the canonical rebuild serializer; apply it with apply_rebuild"));
    }
    if sha256_hex(bytes) != plan.source_sha256 || bytes.len() != plan.source_len {
        return Err(refuse("invalid", "the dictionary bytes no longer match the plan (changed since it was made)"));
    }
    let before = LocDictionary::parse(bytes)?;
    let l = layout(bytes)?;
    let t = &l.db.tables[0];
    let endian = l.db.endian;
    let k = plan.inserts.len();
    let shift = k * RECORD_LEN;
    let old_pool_len = l.pool.len();
    // ---- assemble
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() + shift + plan.appended_len);
    out.extend_from_slice(&bytes[..t.record_block]);
    // Records: existing rows in order, inserts at their positions; re-pointed slots patched.
    let mut edit_ptr: HashMap<usize, u32> = HashMap::new(); // input record → new text pointer
    let mut appended: Vec<u8> = Vec::new();
    let mut insert_ptrs: Vec<(u32, u32)> = Vec::new(); // (key ptr, text ptr) per insert
    for ins in &plan.inserts {
        let (kp, tp) = if endian == Endian::Big {
            let kp = (old_pool_len + appended.len()) as u32;
            appended.extend_from_slice(&ins.key_encoded);
            let tp = match &ins.text_encoded {
                Some(te) => {
                    let p = (old_pool_len + appended.len()) as u32;
                    appended.extend_from_slice(te);
                    p
                }
                None => NULL_PTR,
            };
            (kp, tp)
        } else {
            let tp = match &ins.text_encoded {
                Some(te) => {
                    let p = (old_pool_len + appended.len()) as u32;
                    appended.extend_from_slice(te);
                    p
                }
                None => NULL_PTR,
            };
            let kp = (old_pool_len + appended.len()) as u32;
            appended.extend_from_slice(&ins.key_encoded);
            (kp, tp)
        };
        insert_ptrs.push((kp, tp));
    }
    for e in &plan.edits {
        if e.records.is_empty() {
            continue;
        }
        let p = match &e.text_encoded {
            Some(te) => {
                let p = (old_pool_len + appended.len()) as u32;
                appended.extend_from_slice(te);
                p
            }
            None => NULL_PTR,
        };
        for r in &e.records {
            edit_ptr.insert(*r, p);
        }
    }
    debug_assert_eq!(appended.len(), plan.appended_len);
    let mut changed: Vec<Range<usize>> = Vec::new();
    let mut inserted_records = Vec::new();
    let mut next_insert = 0usize;
    let mut out_rec = 0usize;
    let rec_start = |n: usize| t.record_block + n * RECORD_LEN;
    for i in 0..=before.rows.len() {
        while next_insert < k && plan.inserts[next_insert].position == i {
            let ins = &plan.inserts[next_insert];
            let mut rec = [0u8; RECORD_LEN];
            let (kp, tp) = insert_ptrs[next_insert];
            write_u32(&mut rec, l.key_off, kp, endian);
            write_u32(&mut rec, l.text_off, tp, endian);
            write_u32(&mut rec, l.hash_off, ins.hash, endian);
            let at = rec_start(out_rec);
            debug_assert_eq!(at, out.len());
            out.extend_from_slice(&rec);
            changed.push(at..at + RECORD_LEN);
            inserted_records.push(out_rec);
            debug_assert_eq!(ins.output_record, out_rec);
            out_rec += 1;
            next_insert += 1;
        }
        if i == before.rows.len() {
            break;
        }
        let src = t.record_block + i * RECORD_LEN;
        let mut rec = [0u8; RECORD_LEN];
        rec.copy_from_slice(&bytes[src..src + RECORD_LEN]);
        let at = rec_start(out_rec);
        if let Some(p) = edit_ptr.get(&i) {
            write_u32(&mut rec, l.text_off, *p, endian);
            changed.push(at + l.text_off..at + l.text_off + 4);
        }
        out.extend_from_slice(&rec);
        out_rec += 1;
    }
    // Pool verbatim, then the appended strings, then the EOF CRC slot.
    let new_pool_start = out.len();
    debug_assert_eq!(new_pool_start, l.pool_start + shift);
    out.extend_from_slice(l.pool);
    let appended_range = out.len()..out.len() + appended.len();
    out.extend_from_slice(&appended);
    out.extend_from_slice(&[0u8; 4]);
    // Size words.
    let new_len = out.len();
    write_u32(&mut out, 0x08, new_len as u32, endian);
    write_u16(&mut out, t.block_offset + 0x14, (t.max_records as usize + k) as u16, endian);
    write_u16(&mut out, t.block_offset + 0x16, (t.current_records as usize + k) as u16, endian);
    // CRC words.
    let crc_before: Vec<u32> = [t.block_offset, t.block_offset + 0x24].iter().map(|&o| read_u32(bytes, o, endian)).chain([read_u32(bytes, bytes.len() - 4, endian), read_u32(bytes, 0x14, endian)]).collect();
    {
        // Re-parse the assembled image (the table layout is read from it) and recompute the four
        // CRC scopes in place; the parse borrows a snapshot so the buffer can be patched.
        let snapshot = out.clone();
        let parsed = TdbFile::parse(&snapshot).map_err(|e| refuse("invalid", format!("the grown file does not re-parse: {e}")))?;
        parsed.recompute_crcs(&mut out);
    }
    let crc_after: Vec<u32> = [t.block_offset, t.block_offset + 0x24].iter().map(|&o| read_u32(&out, o, endian)).chain([read_u32(&out, out.len() - 4, endian), read_u32(&out, 0x14, endian)]).collect();
    let mut crc_scopes_changed = Vec::new();
    for (i, name) in ["prior", "header", "body", "file-header"].iter().enumerate() {
        if crc_before[i] != crc_after[i] || (i == 2 && (k > 0 || !appended.is_empty())) {
            crc_scopes_changed.push(*name);
        }
    }
    if k > 0 || !appended.is_empty() {
        changed.push(0x08..0x0C);
        changed.push(0x14..0x18);
        changed.push(t.block_offset + 0x14..t.block_offset + 0x18);
        changed.push(t.block_offset + 0x24..t.block_offset + 0x28);
        if crc_before[0] != crc_after[0] {
            changed.push(t.block_offset..t.block_offset + 4);
        }
        if !appended.is_empty() {
            changed.push(appended_range.clone());
        }
        changed.push(out.len() - 4..out.len());
    }
    changed.sort_by_key(|r| (r.start, r.end));
    // ---- independent verification
    for c in crate::verify(&out).map_err(|e| refuse("verify", format!("the grown file fails verification: {e}")))? {
        if !c.matches() {
            return Err(refuse("verify", format!("the grown file's CRC scope {} does not verify", c.name)));
        }
    }
    let after = LocDictionary::parse(&out)?;
    if after.rows.len() != plan.rows_after() {
        return Err(refuse("verify", format!("the grown file has {} rows, expected {}", after.rows.len(), plan.rows_after())));
    }
    // Expected rows: the input rows with the plan's edits, the inserts at their positions.
    let mut expected: Vec<crate::LocRow> = Vec::with_capacity(plan.rows_after());
    let mut next_insert = 0usize;
    for i in 0..=before.rows.len() {
        while next_insert < k && plan.inserts[next_insert].position == i {
            let ins = &plan.inserts[next_insert];
            expected.push(crate::LocRow { hash: ins.hash, key: Some(ins.key.clone()), text: ins.text.clone() });
            next_insert += 1;
        }
        if i == before.rows.len() {
            break;
        }
        let mut r = before.rows[i].clone();
        if let Some(e) = plan.edits.iter().find(|e| e.records.contains(&i)) {
            r.text = e.new_text.clone();
        }
        expected.push(r);
    }
    if after.rows != expected {
        let bad = after.rows.iter().zip(&expected).position(|(a, b)| a != b).unwrap_or(0);
        return Err(refuse("verify", format!("the grown file's rows differ from the requested result at record {bad}")));
    }
    // Class A outputs have no descent; class B outputs keep their descent set exactly.
    match (&plan.class, OrderClass::of(&after.rows)) {
        (OrderClass::A, OrderClass::A) => {}
        (OrderClass::B { descents }, OrderClass::B { descents: d2 }) if *descents == d2 => {}
        (a, b) => return Err(refuse("verify", format!("the grown file's order class changed ({} → {})", a.as_str(), b.as_str()))),
    }
    // The pool bytes are the original pool verbatim at the shifted position.
    if out[new_pool_start..new_pool_start + old_pool_len] != *l.pool {
        return Err(refuse("verify", "the original pool was not copied verbatim"));
    }
    Ok(GrowthOutput { bytes: out, changed_ranges: changed, inserted_records, appended: appended_range, pool_shift: shift, crc_scopes_changed })
}
