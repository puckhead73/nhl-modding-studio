//! LOC-W2 — **in-place free-text re-encode** of an existing key's text with the dictionary's own
//! Huffman tree, when the new text fits the byte extent the old string occupies.
//!
//! Research handoff P1.3-X (2026-09-18) measured every dictionary: no spare record slots, the
//! pool packed to its end, and a per-title tree alphabet (NHL 10 has no lowercase but `d`/`s`;
//! NHL 11/12 lack `j q v x z`; every title lacks `` ` | ~ ``). So the cheapest useful free-text
//! operation is exactly this one: rewrite the bits at the string's own pool offset, keep the
//! 2-byte decoded-length prefix semantics, zero the slack, change nothing else. The file length,
//! every pointer, the record block and the tree are untouched; only the string's bytes and the
//! CRC words move — so every existing placement route (in-place save, copied target, same-length
//! archive member, disc-image range patch) keeps working.
//!
//! Refusals are exact: characters the tree cannot encode are listed; a text whose encoding does
//! not fit reports the bytes needed against the bytes available; rows sharing the same string
//! pointer are reported (they change too) rather than silently rewritten. Key insertion, longer
//! text and tree rebuilds stay out of scope (the growth path).

use std::collections::HashMap;

use sha2::{Digest, Sha256};
use tdb_core::{Endian, TdbFile};

use crate::repoint::{resolve, RowRef};
use crate::{HuffmanTree, LocDictionary, LocError, Lookup, KIND_TEXT, NULL_PTR};

/// Everything a re-encode would do, computed read-only.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RetextPlan {
    pub source_sha256: String,
    pub source_len: usize,
    pub endian_big: bool,
    pub source: RowRef,
    pub source_lookup: Lookup,
    pub requested_key: String,
    pub new_text: String,
    /// Absolute file offset of the pool.
    pub pool_start: usize,
    /// Pool-relative pointer of the string (unchanged).
    pub text_pointer: u32,
    /// Bits of the old encoded stream (after the 2-byte prefix) and the byte extent it occupies.
    pub old_bits: usize,
    pub old_extent_bytes: usize,
    pub new_bits: usize,
    pub new_extent_bytes: usize,
    /// Other rows whose text pointer is the same string: they show the new text too.
    pub sharing_text: Vec<RowRef>,
    pub hash_collisions: Vec<RowRef>,
}

/// The rewritten bytes plus the audit of what changed.
#[derive(Clone, Debug)]
pub struct RetextOutput {
    pub bytes: Vec<u8>,
    pub changed_ranges: Vec<std::ops::Range<usize>>,
    pub crc_scopes_changed: Vec<&'static str>,
}

/// Characters the tree can encode, as a sorted list of printable symbols (for messages/UI).
pub fn tree_alphabet(bytes: &[u8]) -> Result<Vec<u8>, LocError> {
    let (_, pool) = pool_of(bytes)?;
    let tree = HuffmanTree::parse(pool)?;
    let mut syms: Vec<u8> = tree.codes().keys().copied().collect();
    syms.sort_unstable();
    Ok(syms)
}

fn pool_of<'a>(bytes: &'a [u8]) -> Result<(TdbFile, &'a [u8]), LocError> {
    let db = TdbFile::parse(bytes).map_err(LocError::Tdb)?;
    let t = &db.tables[0];
    let pool_start = t.record_block + t.max_records as usize * t.record_len_bytes as usize;
    if bytes.len() < 4 || pool_start > bytes.len() - 4 {
        return Err(LocError::Layout("pool start beyond the file".into()));
    }
    let pool = &bytes[pool_start..bytes.len() - 4];
    Ok((db, pool))
}

fn sha256_hex(b: &[u8]) -> String {
    let d = Sha256::digest(b);
    d.iter().map(|x| format!("{x:02x}")).collect()
}

fn read_u32(bytes: &[u8], at: usize, endian: Endian) -> u32 {
    let a = [bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]];
    match endian {
        Endian::Big => u32::from_be_bytes(a),
        Endian::Little => u32::from_le_bytes(a),
    }
}

/// Plan the in-place re-encode of `key`'s text to `new_text`.
pub fn plan_retext(bytes: &[u8], key: &str, new_text: &str) -> Result<RetextPlan, LocError> {
    if key.is_empty() {
        return Err(LocError::Layout("empty key".into()));
    }
    if new_text.is_empty() {
        return Err(LocError::Layout(
            "empty text: a string cannot be re-encoded to nothing here (a null pointer is a different operation)".into(),
        ));
    }
    let d = LocDictionary::parse(bytes)?;
    let (db, pool) = pool_of(bytes)?;
    let t = &db.tables[0];
    let stride = t.record_len_bytes as usize;
    let pool_start = t.record_block + t.max_records as usize * stride;
    let text_f = t
        .fields
        .iter()
        .find(|f| f.kind == KIND_TEXT)
        .ok_or_else(|| LocError::Layout("no text field".into()))?;
    let (si, source_lookup) = resolve(&d, key)?;
    let row_ref = |i: usize| RowRef {
        record: i,
        key: d.rows[i].key.clone().unwrap_or_default(),
        hash: d.rows[i].hash,
        text: d.rows[i].text.clone(),
    };
    let slot = t.record_block + si * stride + (text_f.bit_offset / 8) as usize;
    let ptr = read_u32(bytes, slot, db.endian);
    if ptr == NULL_PTR {
        return Err(LocError::Layout(format!(
            "{key:?} has a null text pointer; there is no string to re-encode in place"
        )));
    }
    if d.rows[si].text.as_deref() == Some(new_text) {
        return Err(LocError::Layout(format!(
            "{key:?} already reads {new_text:?}; nothing to change"
        )));
    }
    let tree = HuffmanTree::parse(pool)?;
    let (_, old_bit_end) = tree.decode_with_extent(pool, ptr, 2)?;
    let off = ptr as usize;
    let old_bits = old_bit_end - (off + 2) * 8;
    let old_extent_bytes = 2 + old_bits.div_ceil(8);
    // Encode the new text with the tree; list every unsupported character at once.
    let codes = tree.codes();
    let mut missing: Vec<char> = Vec::new();
    let mut new_bits = 0usize;
    for ch in new_text.chars() {
        let mut buf = [0u8; 4];
        let enc = ch.encode_utf8(&mut buf);
        for b in enc.bytes() {
            match codes.get(&b) {
                Some(code) => new_bits += code.len(),
                None => {
                    if !missing.contains(&ch) {
                        missing.push(ch);
                    }
                }
            }
        }
    }
    if !missing.is_empty() {
        return Err(LocError::Layout(format!(
            "this title's dictionary cannot encode {new_text:?}: its Huffman tree has no symbol for {} (adding one would mean rebuilding the whole pool)",
            missing.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", ")
        )));
    }
    let decoded_len = new_text.len();
    if decoded_len > u16::MAX as usize || decoded_len as u32 * 8 > text_f.bit_width {
        return Err(LocError::Layout(format!(
            "text is {decoded_len} bytes; the field allows {} decoded bytes",
            text_f.bit_width / 8
        )));
    }
    let new_extent_bytes = 2 + new_bits.div_ceil(8);
    if new_extent_bytes > old_extent_bytes {
        return Err(LocError::Layout(format!(
            "{new_text:?} needs {new_extent_bytes} bytes ({new_bits} bits) in the pool but {key:?}'s string occupies {old_extent_bytes} ({old_bits} bits); a longer text needs the growth path (not available)"
        )));
    }
    let ptr_of = |i: usize| read_u32(bytes, t.record_block + i * stride + (text_f.bit_offset / 8) as usize, db.endian);
    let sharing_text = (0..d.rows.len())
        .filter(|&i| i != si && ptr_of(i) == ptr)
        .map(row_ref)
        .collect();
    let sh = d.rows[si].hash;
    let hash_collisions = (0..d.rows.len())
        .filter(|&i| i != si && d.rows[i].hash == sh)
        .map(row_ref)
        .collect();
    Ok(RetextPlan {
        source_sha256: sha256_hex(bytes),
        source_len: bytes.len(),
        endian_big: db.endian == Endian::Big,
        source: row_ref(si),
        source_lookup,
        requested_key: key.to_string(),
        new_text: new_text.to_string(),
        pool_start,
        text_pointer: ptr,
        old_bits,
        old_extent_bytes,
        new_bits,
        new_extent_bytes,
        sharing_text,
        hash_collisions,
    })
}

/// Apply `plan` to `bytes` (the bytes it was made from) and verify independently.
pub fn apply_retext(bytes: &[u8], plan: &RetextPlan) -> Result<RetextOutput, LocError> {
    let fresh = plan_retext(bytes, &plan.requested_key, &plan.new_text)?;
    if &fresh != plan {
        return Err(LocError::Layout(
            "the dictionary bytes no longer match the plan (changed since it was made)".into(),
        ));
    }
    let db = TdbFile::parse(bytes).map_err(LocError::Tdb)?;
    if db.serialize() != bytes {
        return Err(LocError::Layout(
            "this dictionary's CRC layout is not reproduced by the generic recompute (round-trip invariant fails); refusing to write".into(),
        ));
    }
    let before = LocDictionary::parse(bytes)?;
    let (_, pool) = pool_of(bytes)?;
    let tree = HuffmanTree::parse(pool)?;
    let codes = tree.codes();
    // Build the new string: 2-byte big-endian decoded length, then the bit stream, zero slack.
    let mut region = vec![0u8; plan.old_extent_bytes];
    let len = plan.new_text.len() as u16;
    region[0..2].copy_from_slice(&len.to_be_bytes());
    let mut bit = 16usize;
    for b in plan.new_text.bytes() {
        for &one in &codes[&b] {
            if one {
                region[bit >> 3] |= 0x80 >> (bit & 7);
            }
            bit += 1;
        }
    }
    debug_assert_eq!(bit - 16, plan.new_bits);
    let start = plan.pool_start + plan.text_pointer as usize;
    let mut out = bytes.to_vec();
    out[start..start + plan.old_extent_bytes].copy_from_slice(&region);
    db.recompute_crcs(&mut out);
    if out.len() != bytes.len() {
        return Err(LocError::Layout("length changed".into()));
    }
    let scopes = crate::verify(&out)?;
    if let Some(bad) = scopes.iter().find(|c| !c.matches()) {
        return Err(LocError::Layout(format!(
            "output CRC scope {} does not verify (stored {:08x}, computed {:08x})",
            bad.name, bad.stored, bad.computed
        )));
    }
    let after = LocDictionary::parse(&out)?;
    if after.len() != before.len() {
        return Err(LocError::Layout("row count changed".into()));
    }
    let changed: std::collections::HashSet<usize> =
        std::iter::once(plan.source.record).chain(plan.sharing_text.iter().map(|r| r.record)).collect();
    for (i, (a, b)) in before.rows.iter().zip(after.rows.iter()).enumerate() {
        if a.key != b.key || a.hash != b.hash {
            return Err(LocError::Layout(format!("row {i} key/hash changed unexpectedly")));
        }
        if changed.contains(&i) {
            if b.text.as_deref() != Some(plan.new_text.as_str()) {
                return Err(LocError::Layout(format!(
                    "row {i} reads {:?} after the write, not {:?}",
                    b.text, plan.new_text
                )));
            }
        } else if a.text != b.text {
            return Err(LocError::Layout(format!("row {i} text changed unexpectedly")));
        }
    }
    // Differing bytes: the region plus CRC words.
    let mut changed_ranges = Vec::new();
    let mut i = 0usize;
    while i < out.len() {
        if out[i] != bytes[i] {
            let s = i;
            while i < out.len() && out[i] != bytes[i] {
                i += 1;
            }
            changed_ranges.push(s..i);
        } else {
            i += 1;
        }
    }
    let region_range = start..start + plan.old_extent_bytes;
    let mut crc_scopes_changed = Vec::new();
    for r in &changed_ranges {
        if r.start >= region_range.start && r.end <= region_range.end {
            continue;
        }
        let name = if r.end <= 24 { "header" } else if r.end == out.len() { "eof" } else { "body" };
        if !crc_scopes_changed.contains(&name) {
            crc_scopes_changed.push(name);
        }
    }
    Ok(RetextOutput {
        bytes: out,
        changed_ranges,
        crc_scopes_changed,
    })
}

impl HuffmanTree {
    /// Symbol → bit path (`true` = right).
    pub(crate) fn codes(&self) -> HashMap<u8, Vec<bool>> {
        let mut out = HashMap::new();
        let mut stack: Vec<(usize, Vec<bool>)> = vec![(0, Vec::new())];
        while let Some((ni, path)) = stack.pop() {
            match self.nodes[ni] {
                crate::Node::Leaf(v) => {
                    out.insert(v, path);
                }
                crate::Node::Internal { left, right } => {
                    let mut l = path.clone();
                    l.push(false);
                    let mut r = path;
                    r.push(true);
                    stack.push((left, l));
                    stack.push((right, r));
                }
                crate::Node::Pending => {}
            }
        }
        out
    }

    /// Decode like [`Self::decode`] and also return the bit position just past the stream.
    pub(crate) fn decode_with_extent(
        &self,
        pool: &[u8],
        ptr: u32,
        prefix: usize,
    ) -> Result<(Vec<u8>, usize), LocError> {
        let off = ptr as usize;
        if ptr == NULL_PTR || off < self.tree_bytes || off + prefix > pool.len() {
            return Err(LocError::Pool(format!("string pointer {off} outside the pool body")));
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
                    crate::Node::Leaf(v) => {
                        out.push(v);
                        break;
                    }
                    crate::Node::Internal { left, right } => {
                        if bit >= bit_end {
                            return Err(LocError::Pool(format!("string at {off} runs past the pool")));
                        }
                        let b = (pool[bit >> 3] >> (7 - (bit & 7))) & 1;
                        bit += 1;
                        ni = if b == 1 { right } else { left };
                    }
                    crate::Node::Pending => return Err(LocError::Pool("unresolved Huffman node".into())),
                }
            }
        }
        Ok((out, bit))
    }
}
