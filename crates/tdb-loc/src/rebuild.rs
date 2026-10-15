//! LOC-W3 **canonical rebuild serializer (C)** — research handoff 20 rev. 3 §4-C, the path for
//! O-G (a text or key that needs a symbol the tree lacks) and for an explicitly requested rebuild.
//!
//! Where the preserving serializer (`growth.rs`) appends to the pool and never touches what
//! exists, this one **rebuilds the pool from the decoded rows**: a new Huffman tree from the full
//! byte histogram when the alphabet grew (or the old tree bytes verbatim when it did not and no
//! rebuild was forced), then every string in EA's per-row order (gen7 `[key][text]`, PS4
//! `[text][key]`), byte-aligned, no gaps, no slack; **shared pointers preserved as sharing** — a
//! string that `n` input rows point at is emitted once and all `n` output rows point at it; null
//! texts stay null; header words, field descriptors and reserved record bytes are copied, never
//! regenerated. It is never used silently: the plan says `tree-rebuilt` / `pool-rebuilt`, and the
//! output carries an **accounting** of every byte class that differs from the input (§4 C1):
//! the tree bytes (only when rebuilt), the trailing slack and any unreachable string storage
//! (identified ranges of the input pool), the strings that moved, the pointers of the rows whose
//! strings moved, `db_size`, the record-count words and the four CRC words. On a shipped file
//! (no sharing, per-row order, no gaps) with the tree kept and an empty plan, the rebuilt pool
//! equals the input pool minus the trailing slack — the C1 sweep asserts exactly that.
//!
//! The serialised tree (measured on the anchors, every title/platform): four bytes per internal
//! node read breadth-first, `[left marker][left value][right marker][right value]`; a leaf has
//! marker 0 and its byte symbol as the value; an internal child has marker = its 1-based position
//! in the read order and value 0; the right internal child is enqueued before the left. A byte
//! alphabet needs at most 255 internal nodes, so every marker fits.
//!
//! Semantics, refusals and the insertion policy are the growth plan's (`plan_growth` in rebuild
//! mode): class-B insertion is still refused, collisions and capacities likewise; O-G never
//! bypasses them. Nothing here establishes loader acceptance of a rebuilt pool / tree or that a
//! font has a glyph for a new symbol (runtime and glyph gates).

use std::collections::{BTreeMap, HashMap};
use std::ops::Range;

use tdb_core::{Endian, TdbFile};

use crate::growth::{refuse_pub as refuse, GrowthPlan, GrowthRefusal, PlanMode};
use crate::{HuffmanTree, LocDictionary, LocRow, NULL_PTR};

const RECORD_LEN: usize = 16;

/// Build a Huffman tree over `hist` (byte → count; zero counts are absent) and serialise it in
/// the pool's tree format. Deterministic: ties broken by (count, lowest symbol / earliest node).
pub fn build_tree_bytes(hist: &BTreeMap<u8, u64>) -> Result<Vec<u8>, GrowthRefusal> {
    // Arena nodes: `Leaf(symbol)` or `Internal(left, right)` by index.
    #[derive(Clone, Copy)]
    enum N {
        Leaf(u8),
        Internal(usize, usize),
    }
    let mut nodes: Vec<N> = Vec::new();
    let mut forest: Vec<(u64, usize, usize)> = Vec::new(); // (count, tiebreak, node index)
    for (&b, &c) in hist {
        nodes.push(N::Leaf(b));
        forest.push((c, b as usize, nodes.len() - 1));
    }
    if forest.is_empty() {
        return Err(refuse("invalid", "an empty alphabet has no tree"));
    }
    if forest.len() == 1 {
        // A one-symbol alphabet still needs an internal node: a second leaf of the same symbol
        // (its code bit is never emitted for real data, but the tree stays well-formed).
        let (c, s, i) = forest[0];
        nodes.push(nodes[i]);
        forest.push((c, s + 256, nodes.len() - 1));
    }
    let mut seq = 1000usize;
    while forest.len() > 1 {
        forest.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
        let (c1, _, n1) = forest.pop().unwrap();
        let (c2, _, n2) = forest.pop().unwrap();
        seq += 1;
        // The lighter subtree goes right (the decoder's "right = 1" side), the heavier left.
        nodes.push(N::Internal(n2, n1));
        forest.push((c1 + c2, seq, nodes.len() - 1));
    }
    let root = forest.pop().unwrap().2;
    // Breadth-first serialisation with the measured numbering: a queue of internal nodes; each
    // internal child gets marker = its queue position (1-based); the right child is enqueued
    // before the left.
    let mut queue: Vec<usize> = vec![root];
    let mut out = Vec::new();
    let mut qi = 0usize;
    while qi < queue.len() {
        let N::Internal(l, r) = nodes[queue[qi]] else { unreachable!("only internal nodes are queued") };
        qi += 1;
        let mut rec = [0u8; 4];
        for (child, slot) in [(r, 2usize), (l, 0usize)] {
            match nodes[child] {
                N::Leaf(b) => {
                    rec[slot] = 0;
                    rec[slot + 1] = b;
                }
                N::Internal(..) => {
                    queue.push(child);
                    if queue.len() > 256 {
                        return Err(refuse("capacity", "the Huffman tree would need more than 255 internal nodes"));
                    }
                    rec[slot] = (queue.len() - 1) as u8;
                    rec[slot + 1] = 0;
                }
            }
        }
        out.extend_from_slice(&rec);
    }
    Ok(out)
}

/// Where an output string's bytes come from, for sharing: the same source → one emission.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Src {
    /// An input pool pointer (existing string; rows that shared it keep sharing).
    Existing(u32),
    /// A new string: `Insert(i, is_text)` or `Edit(i)` (all its records share one emission).
    Insert(usize, bool),
    Edit(usize),
}

/// One identified range of the input pool that the rebuild discards.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiscardedRange {
    /// `trailing-slack` (after the last reachable string) or `unreachable` (a string no row
    /// points at, or an interior gap).
    pub kind: &'static str,
    /// Pool-relative range in the **input**.
    pub range: Range<usize>,
}

/// The accounting of a rebuild against its input (§4 C1 classes).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RebuildAccounting {
    pub tree_rebuilt: bool,
    pub tree_bytes_before: usize,
    pub tree_bytes_after: usize,
    pub pool_len_before: usize,
    pub pool_len_after: usize,
    pub discarded: Vec<DiscardedRange>,
    /// Existing strings whose pool-relative offset changed.
    pub moved_strings: usize,
    /// Output records whose key/text pointer differs from the corresponding input record's
    /// (moved strings, re-pointed edits); inserted records are counted separately.
    pub repointed_records: usize,
    pub inserted_records: usize,
    /// `true` when, with the tree kept, the output pool equals the input pool up to the end of
    /// the last reachable string (the shipped-file expectation).
    pub pool_prefix_identical: bool,
    pub crc_scopes_changed: Vec<&'static str>,
    pub notes: Vec<String>,
}

/// The rebuilt bytes plus the accounting.
#[derive(Clone, Debug)]
pub struct RebuildOutput {
    pub bytes: Vec<u8>,
    pub accounting: RebuildAccounting,
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

/// EA's stored length of a bit stream of `bits` bits: `bits / 8 + 1` bytes — always one byte
/// past the last full byte, so a stream ending exactly on a byte boundary carries one extra zero
/// byte. Measured on every committed dictionary: with plain `ceil(bits / 8)` packing, 1 in 8
/// strings (956 of 7 904 on NHL 07, 8 356 of 66 792 on PS4 NHL 18) left a 1-byte zero "gap"
/// before the next string; with this rule the repacked pool of every shipped file equals the
/// input pool byte for byte up to its trailing slack.
pub(crate) fn stream_bytes(bits: usize) -> usize {
    bits / 8 + 1
}

/// Encode `s` with `codes` (all symbols present — checked by the caller).
fn encode(codes: &HashMap<u8, Vec<bool>>, s: &[u8], prefix: usize) -> Vec<u8> {
    let bits: usize = s.iter().map(|b| codes[b].len()).sum();
    let mut out = vec![0u8; prefix + stream_bytes(bits)];
    match prefix {
        1 => out[0] = s.len() as u8,
        _ => out[0..2].copy_from_slice(&(s.len() as u16).to_be_bytes()),
    }
    let mut bit = prefix * 8;
    for b in s {
        for &one in &codes[b] {
            if one {
                out[bit >> 3] |= 0x80 >> (bit & 7);
            }
            bit += 1;
        }
    }
    out
}

/// Plan a canonical rebuild: the growth plan's semantics in rebuild mode (missing symbols are
/// collected, not refused). `force_tree` rebuilds the tree even when the alphabet did not grow.
pub fn plan_rebuild(bytes: &[u8], ops: &[crate::GrowthOp], force_tree: bool) -> Result<GrowthPlan, GrowthRefusal> {
    crate::growth::plan_growth_with(bytes, ops, PlanMode::Rebuild { force_tree })
}

/// Apply a rebuild plan (from [`plan_rebuild`]) with the canonical serializer and verify the
/// result independently: re-parse, CRCs, decoded rows against the requested result, sharing
/// relation as planned; every byte class that differs from the input is accounted.
pub fn apply_rebuild(bytes: &[u8], plan: &GrowthPlan) -> Result<RebuildOutput, GrowthRefusal> {
    let Some(rb) = &plan.rebuild else {
        return Err(refuse("invalid", "this plan was made for the preserving serializer; use apply_growth (or plan the rebuild explicitly)"));
    };
    if crate::growth::sha256_hex_pub(bytes) != plan.source_sha256 || bytes.len() != plan.source_len {
        return Err(refuse("invalid", "the dictionary bytes no longer match the plan (changed since it was made)"));
    }
    let before = LocDictionary::parse(bytes)?;
    let db = TdbFile::parse(bytes).map_err(|e| refuse("invalid", e.to_string()))?;
    let t = &db.tables[0];
    let endian = db.endian;
    let stride = t.record_len_bytes as usize;
    let kf = t.fields.iter().find(|f| f.kind == 13).ok_or_else(|| refuse("invalid", "no key field"))?;
    let tf = t.fields.iter().find(|f| f.kind == 14).ok_or_else(|| refuse("invalid", "no text field"))?;
    let hf = t.fields.iter().find(|f| f.kind == 3).ok_or_else(|| refuse("invalid", "no hash field"))?;
    let (key_off, text_off, hash_off) = ((kf.bit_offset / 8) as usize, (tf.bit_offset / 8) as usize, (hf.bit_offset / 8) as usize);
    let pool_start = t.record_block + t.max_records as usize * stride;
    let pool = &bytes[pool_start..bytes.len() - 4];
    let old_tree = HuffmanTree::parse(pool)?;
    let old_codes = old_tree.codes();
    let ptr_of = |i: usize, off: usize| read_u32(bytes, t.record_block + i * stride + off, endian);

    // ---- the output rows with their string sources
    struct OutRow {
        hash: u32,
        /// `None` = a null key pointer (kept null, never turned into an empty key).
        key: Option<Vec<u8>>,
        key_src: Src,
        text: Option<Vec<u8>>,
        text_src: Src,
        /// Input record index (None for an insert).
        input: Option<usize>,
    }
    let k = plan.inserts.len();
    let mut rows: Vec<OutRow> = Vec::with_capacity(before.rows.len() + k);
    let mut next_insert = 0usize;
    for i in 0..=before.rows.len() {
        while next_insert < k && plan.inserts[next_insert].position == i {
            let ins = &plan.inserts[next_insert];
            rows.push(OutRow {
                hash: ins.hash,
                key: Some(ins.key.as_bytes().to_vec()),
                key_src: Src::Insert(next_insert, false),
                text: ins.text.as_ref().map(|s| s.as_bytes().to_vec()),
                text_src: Src::Insert(next_insert, true),
                input: None,
            });
            next_insert += 1;
        }
        if i == before.rows.len() {
            break;
        }
        let r = &before.rows[i];
        let key = r.key.clone().map(String::into_bytes);
        let (text, text_src) = match plan.edits.iter().enumerate().find(|(_, e)| e.records.contains(&i)) {
            Some((ei, e)) => (e.new_text.as_ref().map(|s| s.as_bytes().to_vec()), Src::Edit(ei)),
            None => (r.text.clone().map(String::into_bytes), Src::Existing(ptr_of(i, text_off))),
        };
        rows.push(OutRow { hash: r.hash, key, key_src: Src::Existing(ptr_of(i, key_off)), text, text_src, input: Some(i) });
    }

    // ---- the tree
    let mut hist: BTreeMap<u8, u64> = BTreeMap::new();
    for r in &rows {
        for b in r.key.iter().flatten() {
            *hist.entry(*b).or_default() += 1;
        }
        if let Some(t) = &r.text {
            for b in t {
                *hist.entry(*b).or_default() += 1;
            }
        }
    }
    let alphabet_grew = hist.keys().any(|b| !old_codes.contains_key(b));
    let tree_rebuilt = alphabet_grew || rb.force_tree;
    let (tree_bytes, codes): (Vec<u8>, HashMap<u8, Vec<bool>>) = if tree_rebuilt {
        let tb = build_tree_bytes(&hist)?;
        let parsed = HuffmanTree::parse(&tb)?;
        if parsed.tree_bytes != tb.len() {
            return Err(refuse("verify", "the rebuilt tree does not re-parse to its own length"));
        }
        (tb, parsed.codes())
    } else {
        (pool[..old_tree.tree_bytes].to_vec(), old_codes.clone())
    };

    // ---- the pool: per-row order, sharing by source
    let mut new_pool: Vec<u8> = tree_bytes.clone();
    let mut emitted: HashMap<Src, u32> = HashMap::new();
    let mut key_ptrs = Vec::with_capacity(rows.len());
    let mut text_ptrs = Vec::with_capacity(rows.len());
    for r in &rows {
        let mut emit = |src: Src, s: Option<&[u8]>, prefix: usize, pool: &mut Vec<u8>| -> u32 {
            let Some(s) = s else { return NULL_PTR };
            if let Some(p) = emitted.get(&src) {
                return *p;
            }
            let p = pool.len() as u32;
            pool.extend_from_slice(&encode(&codes, s, prefix));
            emitted.insert(src, p);
            p
        };
        if endian == Endian::Big {
            key_ptrs.push(emit(r.key_src, r.key.as_deref(), 1, &mut new_pool));
            text_ptrs.push(emit(r.text_src, r.text.as_deref(), 2, &mut new_pool));
        } else {
            text_ptrs.push(emit(r.text_src, r.text.as_deref(), 2, &mut new_pool));
            key_ptrs.push(emit(r.key_src, r.key.as_deref(), 1, &mut new_pool));
        }
    }

    // ---- assemble
    let mut out: Vec<u8> = Vec::with_capacity(t.record_block + rows.len() * RECORD_LEN + new_pool.len() + 4);
    out.extend_from_slice(&bytes[..t.record_block]);
    let mut repointed_records = 0usize;
    for (n, r) in rows.iter().enumerate() {
        let mut rec = [0u8; RECORD_LEN];
        if let Some(i) = r.input {
            let src = t.record_block + i * stride;
            rec.copy_from_slice(&bytes[src..src + RECORD_LEN]); // reserved bytes verbatim
            if read_u32(&rec, key_off, endian) != key_ptrs[n] || read_u32(&rec, text_off, endian) != text_ptrs[n] {
                repointed_records += 1;
            }
        }
        write_u32(&mut rec, key_off, key_ptrs[n], endian);
        write_u32(&mut rec, text_off, text_ptrs[n], endian);
        write_u32(&mut rec, hash_off, r.hash, endian);
        out.extend_from_slice(&rec);
    }
    out.extend_from_slice(&new_pool);
    out.extend_from_slice(&[0u8; 4]);
    let new_len = out.len();
    write_u32(&mut out, 0x08, new_len as u32, endian);
    write_u16(&mut out, t.block_offset + 0x14, (t.max_records as usize + k) as u16, endian);
    write_u16(&mut out, t.block_offset + 0x16, (t.current_records as usize + k) as u16, endian);
    let crc_before: Vec<u32> = [t.block_offset, t.block_offset + 0x24].iter().map(|&o| read_u32(bytes, o, endian)).chain([read_u32(bytes, bytes.len() - 4, endian), read_u32(bytes, 0x14, endian)]).collect();
    {
        let snapshot = out.clone();
        let parsed = TdbFile::parse(&snapshot).map_err(|e| refuse("invalid", format!("the rebuilt file does not re-parse: {e}")))?;
        parsed.recompute_crcs(&mut out);
    }
    let crc_after: Vec<u32> = [t.block_offset, t.block_offset + 0x24].iter().map(|&o| read_u32(&out, o, endian)).chain([read_u32(&out, out.len() - 4, endian), read_u32(&out, 0x14, endian)]).collect();
    let mut crc_scopes_changed = Vec::new();
    for (i, name) in ["prior", "header", "body", "file-header"].iter().enumerate() {
        if crc_before[i] != crc_after[i] {
            crc_scopes_changed.push(*name);
        }
    }

    // ---- independent verification
    for c in crate::verify(&out).map_err(|e| refuse("verify", format!("the rebuilt file fails verification: {e}")))? {
        if !c.matches() {
            return Err(refuse("verify", format!("the rebuilt file's CRC scope {} does not verify", c.name)));
        }
    }
    let after = LocDictionary::parse(&out)?;
    let expected: Vec<LocRow> = rows
        .iter()
        .map(|r| LocRow { hash: r.hash, key: r.key.as_ref().map(|k| String::from_utf8_lossy(k).into_owned()), text: r.text.as_ref().map(|t| String::from_utf8_lossy(t).into_owned()) })
        .collect();
    if after.rows != expected {
        let bad = after.rows.iter().zip(&expected).position(|(a, b)| a != b).unwrap_or(0);
        return Err(refuse("verify", format!("the rebuilt file's rows differ from the requested result at record {bad}")));
    }
    // Sharing relation: output rows share a text pointer iff their sources are equal.
    let out_ptr = |n: usize, off: usize| read_u32(&out, t.record_block + n * RECORD_LEN + off, endian);
    let mut by_src: HashMap<Src, u32> = HashMap::new();
    for (n, r) in rows.iter().enumerate() {
        for (src, off, is_null) in [(r.text_src, text_off, r.text.is_none()), (r.key_src, key_off, r.key.is_none())] {
            let p = out_ptr(n, off);
            if is_null {
                if p != NULL_PTR {
                    return Err(refuse("verify", format!("record {n}: a null pointer was expected")));
                }
                continue;
            }
            match by_src.insert(src, p) {
                Some(prev) if prev != p => return Err(refuse("verify", format!("record {n}: sharing relation not preserved"))),
                _ => {}
            }
        }
    }
    let distinct_ptrs: std::collections::HashSet<u32> = by_src.values().copied().collect();
    if distinct_ptrs.len() != by_src.len() {
        return Err(refuse("verify", "two different sources landed on one pool pointer"));
    }
    // Reserved record bytes verbatim for every carried row.
    for (n, r) in rows.iter().enumerate() {
        if let Some(i) = r.input {
            let a = &bytes[t.record_block + i * stride + 12..t.record_block + i * stride + 16];
            let b = &out[t.record_block + n * RECORD_LEN + 12..t.record_block + n * RECORD_LEN + 16];
            if a != b {
                return Err(refuse("verify", format!("record {n}: reserved bytes changed")));
            }
        }
    }
    // Opaque metadata: bytes 0..record_block equal apart from db_size, the file-header hash, the
    // record-count words and the two table CRC words.
    let allowed: Vec<Range<usize>> = vec![0x08..0x0C, 0x14..0x18, t.block_offset..t.block_offset + 4, t.block_offset + 0x14..t.block_offset + 0x18, t.block_offset + 0x24..t.block_offset + 0x28];
    for i in 0..t.record_block {
        if bytes[i] != out[i] && !allowed.iter().any(|r| r.contains(&i)) {
            return Err(refuse("verify", format!("header byte {i} changed outside the accounted words")));
        }
    }

    // ---- accounting against the input pool
    // Reachable input strings: (pool offset, encoded length) for every non-null pointer.
    let mut reach: BTreeMap<u32, usize> = BTreeMap::new();
    for i in 0..before.rows.len() {
        for (off, prefix) in [(key_off, 1usize), (text_off, 2usize)] {
            let p = ptr_of(i, off);
            if p == NULL_PTR {
                continue;
            }
            let (_, bit_end) = old_tree.decode_with_extent(pool, p, prefix).map_err(|e| refuse("invalid", e.to_string()))?;
            let bits = bit_end - (p as usize + prefix) * 8;
            reach.insert(p, prefix + stream_bytes(bits));
        }
    }
    let mut discarded = Vec::new();
    let mut cursor = old_tree.tree_bytes;
    for (&p, &len) in &reach {
        let p = p as usize;
        if p > cursor {
            discarded.push(DiscardedRange { kind: "unreachable", range: cursor..p });
        }
        cursor = cursor.max(p + len);
    }
    if cursor < pool.len() {
        discarded.push(DiscardedRange { kind: "trailing-slack", range: cursor..pool.len() });
    }
    let last_reachable_end = cursor;
    let mut moved_strings = 0usize;
    for (src, &np) in &by_src {
        if let Src::Existing(op) = src {
            if *op != np {
                moved_strings += 1;
            }
        }
    }
    let pool_prefix_identical = !tree_rebuilt && new_pool.len() >= last_reachable_end && new_pool[..last_reachable_end] == pool[..last_reachable_end];
    let mut notes = plan.notes.clone();
    notes.push(if tree_rebuilt { format!("tree-rebuilt: {} symbols ({} B; was {} B){}", hist.len(), tree_bytes.len(), old_tree.tree_bytes, if rb.new_symbols.is_empty() { String::new() } else { format!("; new symbols {}", rb.new_symbols.iter().map(|c| format!("{c:?}")).collect::<Vec<_>>().join(", ")) }) } else { "tree kept (alphabet unchanged)".to_string() });
    notes.push(format!("pool-rebuilt: {} -> {} B; {} discarded range(s); {} string(s) moved; {} record(s) re-pointed; {} inserted", pool.len(), new_pool.len(), discarded.len(), moved_strings, repointed_records, k));
    Ok(RebuildOutput {
        bytes: out,
        accounting: RebuildAccounting {
            tree_rebuilt,
            tree_bytes_before: old_tree.tree_bytes,
            tree_bytes_after: tree_bytes.len(),
            pool_len_before: pool.len(),
            pool_len_after: new_pool.len(),
            discarded,
            moved_strings,
            repointed_records,
            inserted_records: k,
            pool_prefix_identical,
            crc_scopes_changed,
            notes,
        },
    })
}
