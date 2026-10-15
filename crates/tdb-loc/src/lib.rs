//! `tdb-loc` — read support for the EA NHL localization dictionaries (`eng_us.db`,
//! `nhl_eng_us.db`, …): a single-table TDB v6 (`LanguageStrings`, `GJCv`) whose string fields are
//! **pool pointers** into a Huffman-compressed varchar pool (allocation type 66).
//!
//! Layout (measured on the PS4 NHL 15–20 dictionaries and the user's NHL 21 localization handoff,
//! and observed unchanged on the PS3 / Xbox 360 `nhl_eng_us.db` anchors apart from integer
//! endianness and field order):
//!
//! * three fields — `jKhj` `hashid` (kind 3, 32 bits), `VhAs` `stringid` (kind 13: key), `bYbZ`
//!   `sourcetext` (kind 14: text); records are 16 bytes (12 used, 4 preserved);
//! * the pool starts at `record_block + allocated_records * stride` (allocated, not current) and
//!   ends before the trailing 4-byte EOF CRC;
//! * the pool begins with the serialised Huffman tree: four bytes per internal node (left marker,
//!   left value, right marker, right value); a zero marker is a leaf holding its byte symbol; the
//!   right internal child is enqueued before the left, breadth-first;
//! * a pool pointer of `0xFFFF_FFFF` is null; otherwise it is a pool-relative offset (tree
//!   included) to a decoded-length prefix — one byte for kind 13, two bytes **big-endian** for
//!   kind 14 — followed by the bit stream (MSB first, left = 0, right = 1, no terminator);
//! * decoded bytes are UTF-8; the metadata's `depth` is bits of *decoded* capacity (800 → 100
//!   bytes for keys, 32000 → 4000 bytes for text);
//! * the key hash (`hashid`) is **not** the integrity CRC: it XORs each UTF-16 code unit masked
//!   with `0xDF` into a reflected CRC-32 (`0xEDB88320`) accumulator starting at zero, no final XOR.
//!
//! Integrity: the four CRC scopes of the file are the ordinary TDB scopes (`tdb-core` recomputes
//! them); [`verify`] checks them and reports each. Everything here is bounded: offsets, counts,
//! tree size and bit positions are validated before use, and a malformed input is an error, never
//! a panic or a silently empty dictionary. The reader edits nothing; the one bounded write this
//! crate offers is [`repoint::plan_repoint`] / [`repoint::apply_repoint`] (LOC-W1): an existing
//! key's text pointer reassigned to another existing key's string, with independent verification
//! — no key insertion, no text encoding, no pool rewriting.

use std::collections::HashMap;

pub mod growth;
pub mod rebuild;
pub mod repoint;
pub mod retext;
pub use growth::{apply_growth, plan_growth, plan_growth_with, GrowthOp, GrowthOutput, GrowthPlan, GrowthRefusal, OrderClass, PlanMode, RebuildInfo};
pub use rebuild::{apply_rebuild, build_tree_bytes, plan_rebuild, DiscardedRange, RebuildAccounting, RebuildOutput};
pub use repoint::{apply_repoint, plan_repoint, Lookup, RepointOutput, RepointPlan, RowRef};
pub use retext::{apply_retext, plan_retext, tree_alphabet, RetextOutput, RetextPlan};

use tdb_core::{Endian, TdbFile};

/// Kind of the key field (1-byte decoded-length prefix).
const KIND_KEY: u32 = 13;
/// Kind of the text field (2-byte big-endian decoded-length prefix).
pub(crate) const KIND_TEXT: u32 = 14;
/// Kind of the fixed-bit hash field.
const KIND_INT: u32 = 3;
/// The Huffman pool allocation type.
const ALLOC_HUFFMAN: u32 = 66;
/// Upper bound on Huffman nodes (a byte alphabet needs at most 511).
const MAX_NODES: usize = 1024;
/// Null pool pointer.
pub(crate) const NULL_PTR: u32 = 0xFFFF_FFFF;

#[derive(Debug)]
pub enum LocError {
    Tdb(tdb_core::TdbError),
    Layout(String),
    Pool(String),
}

impl std::fmt::Display for LocError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LocError::Tdb(e) => write!(f, "localization db: {e}"),
            LocError::Layout(s) => write!(f, "localization db layout: {s}"),
            LocError::Pool(s) => write!(f, "localization db pool: {s}"),
        }
    }
}
impl std::error::Error for LocError {}

/// One dictionary row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LocRow {
    pub hash: u32,
    /// `None` for a null pointer (distinct from an empty key).
    pub key: Option<String>,
    /// `None` for a null pointer (distinct from an empty string).
    pub text: Option<String>,
}

/// Measured facts about a parsed dictionary, for diagnostics and fixture tests.
#[derive(Clone, Debug, Default)]
pub struct LocStats {
    pub endian_big: bool,
    pub rows: usize,
    pub allocated_rows: usize,
    pub stride: u32,
    pub pool_bytes: usize,
    pub tree_bytes: usize,
    pub symbols: usize,
    /// Strings whose bytes were not valid UTF-8 (kept lossily).
    pub invalid_utf8: usize,
    pub distinct_keys: usize,
    pub distinct_hashes: usize,
    /// Rows whose stored hash does not equal `loc_hash(key)`.
    pub hash_mismatches: usize,
}

/// A parsed, read-only dictionary.
#[derive(Clone, Debug)]
pub struct LocDictionary {
    pub rows: Vec<LocRow>,
    pub stats: LocStats,
    by_key: HashMap<String, usize>,
}

impl LocDictionary {
    /// Parse a dictionary file (either byte order).
    pub fn parse(bytes: &[u8]) -> Result<Self, LocError> {
        let db = TdbFile::parse(bytes).map_err(LocError::Tdb)?;
        if !db.size_ok() {
            return Err(LocError::Layout(format!(
                "stored size {} != file length {}",
                db.db_size,
                bytes.len()
            )));
        }
        if db.tables.len() != 1 {
            return Err(LocError::Layout(format!(
                "expected one table, found {}",
                db.tables.len()
            )));
        }
        let t = &db.tables[0];
        if t.data_alloc_type != ALLOC_HUFFMAN {
            return Err(LocError::Layout(format!(
                "allocation type {} is not the Huffman pool ({ALLOC_HUFFMAN})",
                t.data_alloc_type
            )));
        }
        let mut key_f = None;
        let mut text_f = None;
        let mut hash_f = None;
        for f in &t.fields {
            match f.kind {
                KIND_KEY => key_f = Some(f),
                KIND_TEXT => text_f = Some(f),
                KIND_INT => hash_f = Some(f),
                other => {
                    return Err(LocError::Layout(format!(
                        "unexpected field kind {other} ({})",
                        f.id.raw()
                    )))
                }
            }
        }
        let (Some(key_f), Some(text_f), Some(hash_f)) = (key_f, text_f, hash_f) else {
            return Err(LocError::Layout(
                "key/text/hash fields not all present".into(),
            ));
        };
        // Varchar descriptors carry the *decoded* capacity as their width (800 / 32000 bits); the
        // record slot itself is always a byte-aligned 32-bit pointer. The hash is a real 32-bit field.
        for f in [key_f, text_f, hash_f] {
            if !f.bit_offset.is_multiple_of(8) || f.bit_offset / 8 + 4 > t.record_len_bytes {
                return Err(LocError::Layout(format!(
                    "field {} must be a byte-aligned 32-bit slot inside the record",
                    f.id.raw()
                )));
            }
        }
        if hash_f.bit_width != 32 {
            return Err(LocError::Layout("hash field must be 32 bits".into()));
        }
        let stride = t.record_len_bytes as usize;
        let pool_start = t
            .record_block
            .checked_add(
                (t.max_records as usize)
                    .checked_mul(stride)
                    .ok_or_else(|| LocError::Layout("record block size overflows".into()))?,
            )
            .ok_or_else(|| LocError::Layout("pool start overflows".into()))?;
        if bytes.len() < 4 || pool_start > bytes.len() - 4 {
            return Err(LocError::Layout(format!(
                "pool start {pool_start} is beyond the file body ({} bytes)",
                bytes.len()
            )));
        }
        let pool = &bytes[pool_start..bytes.len() - 4];
        let tree = HuffmanTree::parse(pool)?;

        let mut rows = Vec::with_capacity(t.current_records as usize);
        let mut invalid_utf8 = 0usize;
        let mut hash_mismatches = 0usize;
        let mut by_key: HashMap<String, usize> = HashMap::new();
        let mut hashes = std::collections::HashSet::new();
        let read_ptr = |rec: u32, f: &tdb_core::FieldDescriptor| -> Result<u32, LocError> {
            let start = t.record_block + rec as usize * stride + (f.bit_offset / 8) as usize;
            let a = bytes
                .get(start..start + 4)
                .ok_or_else(|| LocError::Layout(format!("record {rec} out of range")))?;
            let a = [a[0], a[1], a[2], a[3]];
            Ok(match db.endian {
                Endian::Big => u32::from_be_bytes(a),
                Endian::Little => u32::from_le_bytes(a),
            })
        };
        for rec in 0..t.current_records as u32 {
            let hash = read_ptr(rec, hash_f)?;
            let key = tree.decode(pool, read_ptr(rec, key_f)?, 1)?;
            let text = tree.decode(pool, read_ptr(rec, text_f)?, 2)?;
            let to_string = |b: Option<Vec<u8>>, bad: &mut usize| -> Option<String> {
                b.map(|bytes| match String::from_utf8(bytes) {
                    Ok(s) => s,
                    Err(e) => {
                        *bad += 1;
                        String::from_utf8_lossy(e.as_bytes()).into_owned()
                    }
                })
            };
            let key = to_string(key, &mut invalid_utf8);
            let text = to_string(text, &mut invalid_utf8);
            if let Some(k) = &key {
                if loc_hash(k) != hash {
                    hash_mismatches += 1;
                }
                // First occurrence wins; duplicates are legitimate (whitespace variants) and
                // stay addressable through `rows`.
                by_key.entry(k.clone()).or_insert(rows.len());
            }
            hashes.insert(hash);
            rows.push(LocRow { hash, key, text });
        }
        let stats = LocStats {
            endian_big: db.endian == Endian::Big,
            rows: rows.len(),
            allocated_rows: t.max_records as usize,
            stride: t.record_len_bytes,
            pool_bytes: pool.len(),
            tree_bytes: tree.tree_bytes,
            symbols: tree.symbols,
            invalid_utf8,
            distinct_keys: by_key.len(),
            distinct_hashes: hashes.len(),
            hash_mismatches,
        };
        Ok(LocDictionary {
            rows,
            stats,
            by_key,
        })
    }

    /// The text stored under an exact key (byte-for-byte; keys are never trimmed or case-folded).
    /// `None` when the key is absent **or** its text pointer is null.
    pub fn text(&self, key: &str) -> Option<&str> {
        self.by_key
            .get(key)
            .and_then(|&i| self.rows[i].text.as_deref())
    }

    /// Whether an exact key exists (even with null text).
    pub fn contains_key(&self, key: &str) -> bool {
        self.by_key.contains_key(key)
    }

    pub fn len(&self) -> usize {
        self.rows.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

/// The localization key hash (`hashid`): UTF-16 code units masked with `0xDF`, reflected CRC-32
/// steps with `0xEDB88320`, accumulator starting at zero, no final XOR. Distinct from the file's
/// integrity CRC.
pub fn loc_hash(key: &str) -> u32 {
    let mut h: u32 = 0;
    for unit in key.encode_utf16() {
        h ^= (unit & 0xDF) as u32;
        for _ in 0..8 {
            h = if h & 1 != 0 {
                (h >> 1) ^ 0xEDB8_8320
            } else {
                h >> 1
            };
        }
    }
    h
}

/// One integrity scope of a dictionary file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrcCheck {
    pub name: &'static str,
    pub start: usize,
    pub end: usize,
    pub stored_at: usize,
    pub stored: u32,
    pub computed: u32,
}

impl CrcCheck {
    pub fn matches(&self) -> bool {
        self.stored == self.computed
    }
}

/// Verify the four CRC scopes of a single-table dictionary (directory, table header, body, file
/// header). Fails on a file whose layout is not the single-table dictionary shape.
pub fn verify(bytes: &[u8]) -> Result<Vec<CrcCheck>, LocError> {
    let db = TdbFile::parse(bytes).map_err(LocError::Tdb)?;
    if db.tables.len() != 1 {
        return Err(LocError::Layout(format!(
            "expected one table, found {}",
            db.tables.len()
        )));
    }
    if bytes.len() < 76 {
        return Err(LocError::Layout("file too short".into()));
    }
    let t = db.tables[0].block_offset;
    if t + 40 > bytes.len() - 4 {
        return Err(LocError::Layout("table block beyond the file".into()));
    }
    let rd = |o: usize| -> u32 {
        let a = [bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]];
        match db.endian {
            Endian::Big => u32::from_be_bytes(a),
            Endian::Little => u32::from_le_bytes(a),
        }
    };
    let scopes = [
        ("directory", 24usize, 32usize, t),
        ("table-header", t + 4, t + 36, t + 36),
        ("body", t + 40, bytes.len() - 4, bytes.len() - 4),
        ("file-header", 0, 20, 20),
    ];
    Ok(scopes
        .iter()
        .map(|&(name, start, end, stored_at)| CrcCheck {
            name,
            start,
            end,
            stored_at,
            stored: rd(stored_at),
            computed: tdb_core::crc::crc32_mpeg2(&bytes[start..end]),
        })
        .collect())
}

// ----------------------------------------------------------------------------- Huffman

#[derive(Clone, Copy, Debug)]
pub(crate) enum Node {
    Leaf(u8),
    Internal { left: usize, right: usize },
    Pending,
}

pub(crate) struct HuffmanTree {
    pub(crate) nodes: Vec<Node>,
    pub(crate) tree_bytes: usize,
    pub(crate) symbols: usize,
}

impl HuffmanTree {
    /// Parse the serialised tree at the head of the pool (see the module docs).
    pub(crate) fn parse(pool: &[u8]) -> Result<Self, LocError> {
        let mut nodes = vec![Node::Pending];
        let mut queue = vec![0usize];
        let mut pos = 0usize;
        let mut qi = 0usize;
        let mut symbols = 0usize;
        while qi < queue.len() {
            let parent = queue[qi];
            qi += 1;
            if pos + 4 > pool.len() {
                return Err(LocError::Pool("Huffman tree runs past the pool".into()));
            }
            let mut children = [0usize; 2];
            for (j, child) in children.iter_mut().enumerate() {
                let marker = pool[pos + 2 * j];
                let value = pool[pos + 2 * j + 1];
                let idx = nodes.len();
                if marker == 0 {
                    nodes.push(Node::Leaf(value));
                    symbols += 1;
                } else {
                    nodes.push(Node::Pending);
                }
                *child = idx;
            }
            pos += 4;
            nodes[parent] = Node::Internal {
                left: children[0],
                right: children[1],
            };
            // Right internal child before left (the verified breadth-first order).
            for &c in [children[1], children[0]].iter() {
                if matches!(nodes[c], Node::Pending) {
                    queue.push(c);
                }
            }
            if nodes.len() > MAX_NODES {
                return Err(LocError::Pool(format!(
                    "Huffman tree exceeds {MAX_NODES} nodes"
                )));
            }
        }
        if nodes.iter().any(|n| matches!(n, Node::Pending)) {
            return Err(LocError::Pool(
                "Huffman tree left an unresolved node".into(),
            ));
        }
        Ok(HuffmanTree {
            nodes,
            tree_bytes: pos,
            symbols,
        })
    }

    /// Decode the string at pool offset `ptr` with a `prefix`-byte decoded-length prefix
    /// (1 = u8, 2 = u16 big-endian). `None` for the null pointer.
    fn decode(&self, pool: &[u8], ptr: u32, prefix: usize) -> Result<Option<Vec<u8>>, LocError> {
        if ptr == NULL_PTR {
            return Ok(None);
        }
        let off = ptr as usize;
        if off < self.tree_bytes || off + prefix > pool.len() {
            return Err(LocError::Pool(format!(
                "string pointer {off} outside the pool body ({}..{})",
                self.tree_bytes,
                pool.len()
            )));
        }
        let len = match prefix {
            1 => pool[off] as usize,
            _ => u16::from_be_bytes([pool[off], pool[off + 1]]) as usize,
        };
        let mut bit = (off + prefix) * 8;
        let bit_end = pool.len() * 8;
        let mut out = Vec::with_capacity(len);
        for _ in 0..len {
            let mut ni = 0usize;
            loop {
                match self.nodes[ni] {
                    Node::Leaf(v) => {
                        out.push(v);
                        break;
                    }
                    Node::Internal { left, right } => {
                        if bit >= bit_end {
                            return Err(LocError::Pool(format!(
                                "string at {off} runs past the pool"
                            )));
                        }
                        let b = (pool[bit >> 3] >> (7 - (bit & 7))) & 1;
                        bit += 1;
                        ni = if b == 1 { right } else { left };
                    }
                    Node::Pending => return Err(LocError::Pool("unresolved Huffman node".into())),
                }
            }
        }
        Ok(Some(out))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loc_hash_matches_the_reference_vector() {
        assert_eq!(loc_hash("MOD_LEGENDS_AUDIT_LABEL"), 1887588376);
    }

    #[test]
    fn short_or_foreign_input_is_an_error_not_a_panic() {
        assert!(LocDictionary::parse(b"").is_err());
        assert!(LocDictionary::parse(b"DBxxxxxxxxxxxxxxxxxxxxxxxxxx").is_err());
        assert!(verify(b"DB").is_err());
    }
}
