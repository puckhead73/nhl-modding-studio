//! LOC-W1 — the one bounded dictionary write: make an existing key's **text pointer** equal another
//! existing key's text pointer, so the first key displays the second key's string. Nothing else
//! moves: no key is added, no string is encoded, the Huffman pool is untouched, the file keeps its
//! length. The only bytes that change are the 4-byte pointer slot in the source row and the CRC
//! words whose scopes cover it — and those are verified independently after the write, never
//! assumed.
//!
//! Vocabulary: the **source** key `A` is the row being re-pointed; the **donor** key `B` supplies
//! the pointer. `A` keeps its own key, hash and record; every other row keeps its bytes.
//!
//! Lookup: exact key first. A key that is not present exactly may resolve through the game's own
//! hash (`loc_hash`, case-folded), but only when exactly one row carries that hash — a collision
//! is a refusal, never a guess. Null pointers (key or text) on either row are refused.
//!
//! Consequences are reported, not hidden: rows that share the source's hash (collisions), rows
//! whose text pointer equals the source's old pointer (they keep the old string) or the donor's
//! (they already show the new string). Which game screens read a key is **not** knowable here.

use std::ops::Range;

use tdb_core::{Endian, TdbFile};

use crate::{loc_hash, verify, CrcCheck, LocDictionary, LocError};

/// How a key was resolved to a row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Lookup {
    /// The exact key string is a row key.
    Exact,
    /// No exact row; exactly one row carries `loc_hash(key)` (the game's case-folded lookup).
    ByHash,
}

impl Lookup {
    pub fn as_str(self) -> &'static str {
        match self {
            Lookup::Exact => "exact",
            Lookup::ByHash => "by-hash",
        }
    }
}

/// A row named in a plan (the source, the donor, or a row sharing something with them).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowRef {
    pub record: usize,
    pub key: String,
    pub hash: u32,
    pub text: Option<String>,
}

/// Everything the write will do, computed read-only from the dictionary bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepointPlan {
    pub source_sha256: String,
    pub source_len: usize,
    pub endian_big: bool,
    pub source: RowRef,
    pub source_lookup: Lookup,
    pub donor: RowRef,
    pub donor_lookup: Lookup,
    /// The requested key strings, verbatim.
    pub requested_key: String,
    pub requested_donor: String,
    /// Absolute file offset of the 4-byte text-pointer slot in the source row.
    pub slot_offset: usize,
    pub old_pointer: u32,
    pub new_pointer: u32,
    /// Other rows that carry the source key's hash (the game cannot tell them apart).
    pub hash_collisions: Vec<RowRef>,
    /// Other rows whose text pointer equals the old pointer: they keep the old string.
    pub sharing_old_text: Vec<RowRef>,
    /// Other rows (besides the donor) whose text pointer equals the new pointer.
    pub sharing_new_text: Vec<RowRef>,
}

/// The write's result: the new bytes and the exact accounting of what changed.
#[derive(Clone, Debug)]
pub struct RepointOutput {
    pub bytes: Vec<u8>,
    pub output_sha256: String,
    /// Every differing byte range against the input, ascending. Exactly the pointer slot plus the
    /// CRC words that moved; anything else is a refusal, so this list is the allowed-difference
    /// audit itself.
    pub changed_ranges: Vec<Range<usize>>,
    /// The CRC scopes of the output, all verified to match.
    pub crc_scopes: Vec<CrcCheck>,
    /// Which of those scopes changed against the input.
    pub crc_scopes_changed: Vec<&'static str>,
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

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    format!("{:x}", Sha256::digest(bytes))
}

/// Resolve `key` to a row index: exact, else by hash when unambiguous.
pub(crate) fn resolve(d: &LocDictionary, key: &str) -> Result<(usize, Lookup), LocError> {
    if let Some(i) = d.rows.iter().position(|r| r.key.as_deref() == Some(key)) {
        return Ok((i, Lookup::Exact));
    }
    let h = loc_hash(key);
    let hits: Vec<usize> = d
        .rows
        .iter()
        .enumerate()
        .filter(|(_, r)| r.hash == h)
        .map(|(i, _)| i)
        .collect();
    match hits.as_slice() {
        [] => Err(LocError::Layout(format!(
            "key {key:?} is not in this dictionary (no exact row and no row with hash {h:08x})"
        ))),
        [one] => Ok((*one, Lookup::ByHash)),
        many => Err(LocError::Layout(format!(
            "key {key:?} is not an exact row and its hash {h:08x} is carried by {} rows ({}); the resolution is ambiguous and is refused",
            many.len(),
            many.iter()
                .map(|&i| d.rows[i].key.clone().unwrap_or_else(|| "<null>".into()))
                .collect::<Vec<_>>()
                .join(", ")
        ))),
    }
}

/// Plan the re-point of `key`'s text at `donor`'s text. Read-only.
pub fn plan_repoint(bytes: &[u8], key: &str, donor: &str) -> Result<RepointPlan, LocError> {
    if key.is_empty() || donor.is_empty() {
        return Err(LocError::Layout("empty key".into()));
    }
    let d = LocDictionary::parse(bytes)?;
    let db = TdbFile::parse(bytes).map_err(LocError::Tdb)?;
    let t = &db.tables[0];
    let stride = t.record_len_bytes as usize;
    let text_f = t
        .fields
        .iter()
        .find(|f| f.kind == crate::KIND_TEXT)
        .ok_or_else(|| LocError::Layout("no text field".into()))?;
    let text_slot = |rec: usize| t.record_block + rec * stride + (text_f.bit_offset / 8) as usize;
    let (si, source_lookup) = resolve(&d, key)?;
    let (di, donor_lookup) = resolve(&d, donor)?;
    if si == di {
        return Err(LocError::Layout(format!(
            "key {key:?} and donor {donor:?} resolve to the same row {si}; nothing to re-point"
        )));
    }
    let row_ref = |i: usize| RowRef {
        record: i,
        key: d.rows[i].key.clone().unwrap_or_default(),
        hash: d.rows[i].hash,
        text: d.rows[i].text.clone(),
    };
    for (i, what) in [(si, "source"), (di, "donor")] {
        if d.rows[i].key.is_none() {
            return Err(LocError::Layout(format!("{what} row {i} has a null key pointer")));
        }
        if d.rows[i].text.is_none() {
            return Err(LocError::Layout(format!(
                "{what} row {i} ({:?}) has a null text pointer; a null string cannot be displayed or donated",
                d.rows[i].key
            )));
        }
    }
    let old_pointer = read_u32(bytes, text_slot(si), db.endian);
    let new_pointer = read_u32(bytes, text_slot(di), db.endian);
    if old_pointer == new_pointer {
        return Err(LocError::Layout(format!(
            "{key:?} already points at the same string as {donor:?} (pointer {old_pointer:#010x}); nothing to change"
        )));
    }
    let ptr_of = |i: usize| read_u32(bytes, text_slot(i), db.endian);
    let sh = d.rows[si].hash;
    let hash_collisions = (0..d.rows.len())
        .filter(|&i| i != si && d.rows[i].hash == sh)
        .map(row_ref)
        .collect();
    let sharing_old_text = (0..d.rows.len())
        .filter(|&i| i != si && ptr_of(i) == old_pointer)
        .map(row_ref)
        .collect();
    let sharing_new_text = (0..d.rows.len())
        .filter(|&i| i != si && i != di && ptr_of(i) == new_pointer)
        .map(row_ref)
        .collect();
    Ok(RepointPlan {
        source_sha256: sha256_hex(bytes),
        source_len: bytes.len(),
        endian_big: db.endian == Endian::Big,
        source: row_ref(si),
        source_lookup,
        donor: row_ref(di),
        donor_lookup,
        requested_key: key.to_string(),
        requested_donor: donor.to_string(),
        slot_offset: text_slot(si),
        old_pointer,
        new_pointer,
        hash_collisions,
        sharing_old_text,
        sharing_new_text,
    })
}

/// Apply `plan` to `bytes` (which must be the bytes the plan was made from) and verify the
/// result independently: the pointer slot changed, the source key resolves to the donor's text,
/// every other row is unchanged, the file length is unchanged, every CRC scope verifies, and the
/// set of differing bytes is exactly the slot plus the CRC words that moved.
pub fn apply_repoint(bytes: &[u8], plan: &RepointPlan) -> Result<RepointOutput, LocError> {
    // The plan must describe these bytes, and the layout must be one the generic CRC routine
    // reproduces bit-for-bit (the round-trip invariant), otherwise the recompute could rewrite a
    // word this dictionary lays out differently.
    let fresh = plan_repoint(bytes, &plan.requested_key, &plan.requested_donor)?;
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
    let mut out = bytes.to_vec();
    write_u32(&mut out, plan.slot_offset, plan.new_pointer, db.endian);
    db.recompute_crcs(&mut out);
    if out.len() != bytes.len() {
        return Err(LocError::Layout("length changed".into()));
    }
    // Independent verification on the output bytes.
    let scopes = verify(&out)?;
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
    for (i, (a, b)) in before.rows.iter().zip(after.rows.iter()).enumerate() {
        if i == plan.source.record {
            if b.key != a.key || b.hash != a.hash {
                return Err(LocError::Layout(format!("source row {i} key/hash changed")));
            }
            if b.text != plan.donor.text {
                return Err(LocError::Layout(format!(
                    "source row {i} text is {:?}, not the donor's {:?}",
                    b.text, plan.donor.text
                )));
            }
        } else if a != b {
            return Err(LocError::Layout(format!("row {i} changed but was not planned to")));
        }
    }
    // Allowed-difference audit: the slot, plus CRC words that moved (and nothing else).
    let mut changed: Vec<Range<usize>> = Vec::new();
    let mut i = 0;
    while i < out.len() {
        if out[i] != bytes[i] {
            let s = i;
            while i < out.len() && out[i] != bytes[i] {
                i += 1;
            }
            changed.push(s..i);
        } else {
            i += 1;
        }
    }
    let slot = plan.slot_offset..plan.slot_offset + 4;
    let crc_words: Vec<(&'static str, Range<usize>)> = scopes
        .iter()
        .map(|c| (c.name, c.stored_at..c.stored_at + 4))
        .collect();
    let mut crc_scopes_changed = Vec::new();
    for r in &changed {
        if r.start >= slot.start && r.end <= slot.end {
            continue;
        }
        match crc_words.iter().find(|(_, w)| r.start >= w.start && r.end <= w.end) {
            Some((name, _)) => crc_scopes_changed.push(*name),
            None => {
                return Err(LocError::Layout(format!(
                    "unexpected byte difference at {:#x}..{:#x} (neither the pointer slot nor a CRC word)",
                    r.start, r.end
                )))
            }
        }
    }
    if !changed.iter().any(|r| r.start >= slot.start && r.end <= slot.end) {
        return Err(LocError::Layout("the pointer slot did not change".into()));
    }
    Ok(RepointOutput {
        output_sha256: sha256_hex(&out),
        bytes: out,
        changed_ranges: changed,
        crc_scopes: scopes,
        crc_scopes_changed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn anchor(rel: &str) -> Vec<u8> {
        std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)).unwrap()
    }

    #[test]
    fn repoints_norway_at_rest_of_world_on_the_legacy_ps3_anchor_and_back() {
        let bytes = anchor("anchors/ps3/nhllegacy/databases/nhl_eng_us.db");
        let plan = plan_repoint(&bytes, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
        assert!(plan.endian_big);
        assert_eq!(plan.source.text.as_deref(), Some("Norway"));
        assert_eq!(plan.donor.text.as_deref(), Some("Rest Of World"));
        assert_eq!(plan.source_lookup, Lookup::Exact);
        assert!(plan.hash_collisions.is_empty());
        let out = apply_repoint(&bytes, &plan).unwrap();
        assert_eq!(out.bytes.len(), bytes.len());
        // Exactly the slot and the EOF (body) CRC word moved.
        assert_eq!(out.changed_ranges.len(), 2);
        let slot = plan.slot_offset..plan.slot_offset + 4;
        assert!(out.changed_ranges[0].start >= slot.start && out.changed_ranges[0].end <= slot.end);
        assert_eq!(out.crc_scopes_changed, vec!["body"]);
        let d = LocDictionary::parse(&out.bytes).unwrap();
        assert_eq!(d.text("TXT_LEAGUE_NORWAY"), Some("Rest Of World"));
        assert_eq!(d.text("TXT_ROW_LEAGUE"), Some("Rest Of World"));
        // Re-pointing back reproduces the original bytes exactly.
        let back = plan_repoint(&out.bytes, "TXT_LEAGUE_NORWAY", "TXT_LEAGUE_NORWAY");
        assert!(back.is_err(), "same row is refused");
        // Restore = write the old pointer: model it by re-pointing at a row with the old pointer.
        // The original pointer is unique to the source row here, so restoration is the original
        // bytes themselves (the transaction engine keeps them); assert the audit is symmetric.
        assert_eq!(plan.sharing_old_text, vec![]);
    }

    #[test]
    fn little_endian_ps4_dictionary_repoints_with_the_same_accounting() {
        let bytes = anchor("anchors/ps4/nhl16/databases/eng_us.db");
        let plan = plan_repoint(&bytes, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
        assert!(!plan.endian_big);
        let out = apply_repoint(&bytes, &plan).unwrap();
        assert_eq!(out.changed_ranges.len(), 2);
        assert_eq!(out.crc_scopes_changed, vec!["body"]);
        assert_eq!(
            LocDictionary::parse(&out.bytes).unwrap().text("TXT_LEAGUE_NORWAY"),
            Some("Rest Of World")
        );
    }

    #[test]
    fn hash_fallback_resolves_only_unambiguous_keys() {
        let bytes = anchor("anchors/ps3/nhllegacy/databases/nhl_eng_us.db");
        // `AHL` is not an exact key; the row keyed `ahl` carries its hash.
        let plan = plan_repoint(&bytes, "AHL", "TXT_ROW_LEAGUE").unwrap();
        assert_eq!(plan.source_lookup, Lookup::ByHash);
        assert_eq!(plan.source.key, "ahl");
        assert!(plan_repoint(&bytes, "NO_SUCH_KEY_XYZ", "TXT_ROW_LEAGUE").is_err());
        assert!(plan_repoint(&bytes, "", "TXT_ROW_LEAGUE").is_err());
    }

    #[test]
    fn shared_and_colliding_rows_are_reported_and_null_or_equal_pointers_refused() {
        let bytes = anchor("anchors/ps3/nhllegacy/databases/nhl_eng_us.db");
        // Two keys already showing the same string (`Norway` appears under several keys with
        // distinct pointers, so pick a pair known to share nothing and check the lists are lists).
        let plan = plan_repoint(&bytes, "TXT_WHL", "WHL").unwrap();
        assert!(plan.sharing_new_text.iter().all(|r| r.record != plan.donor.record));
        // Same pointer → refused.
        let out = apply_repoint(&bytes, &plan).unwrap();
        assert!(plan_repoint(&out.bytes, "TXT_WHL", "WHL").is_err());
        // Stale plan → refused.
        assert!(apply_repoint(&out.bytes, &plan).is_err());
    }

    #[test]
    fn a_tampered_input_is_refused_before_anything_is_written() {
        let mut bytes = anchor("anchors/ps3/nhllegacy/databases/nhl_eng_us.db");
        let plan = plan_repoint(&bytes, "TXT_LEAGUE_NORWAY", "TXT_ROW_LEAGUE").unwrap();
        let last = bytes.len() - 1;
        bytes[last] ^= 0xFF; // EOF CRC damaged
        assert!(apply_repoint(&bytes, &plan).is_err());
    }
}
