//! PS4 commentary contexts — `0XTC` (little-endian `CTX0`, 64-bit pointers), NHL 15–21.
//!
//! Research 2026-09-21 pm handoff [15](../../../docs/research/overnight-2026-09-21-pm/15-ps4-ctx0-context-records-decoded.md)
//! (`research-r58-ps4-ctx0`), measured on all seven titles: the same trigger → conditions → items
//! model as the gen7 v4.5 layout this crate already reads, written little-endian with `u64`
//! pointers and absolute file offsets instead of big-endian `u32`s. Two file versions — 3 on
//! NHL 15–19, 4 on NHL 20 / 21 — share the layout; version 4 adds nothing this walk needs.
//!
//! Output is the **same** [`Ctx0`] the gen7 parser produces, so the domain listing, the API DTOs
//! and the Contexts card are shared rather than duplicated.
//!
//! ## Byte map (handoff 15 §1)
//!
//! ```text
//! 0x00  "0XTCTTEK"
//! 0x08  u32 version (3 | 4)
//! 0x0C  u32 n            record count (the pointer list holds n − 2 — see below)
//! 0x10  u32 A            group-entry count
//! 0x14  u32 n            (repeated)
//! 0x30  u64 pool1        condition-name pool (absolute)
//! 0x38  u64 pool2        trigger-name pool (absolute)
//! 0x50  A × 16-byte group entries
//!       (n − 2) × u64 record pointers
//!       T2 rows (u64 1, u64 ptr) …
//!       records │ pool1 │ pool2
//! ```
//!
//! Record header, 0x58 bytes: `u32 context id │ u32 priority-like │ u8 ncond │ u8 nextra │
//! u8 nblk │ u8 100 │ u32 flag │ u16 time-like │ u16 kind │ … │ 0x20 u64 trigger-name offset into
//! pool2 (`ff…` = none) │ 0x28 u64 item slot 0 │ 0x30 u64 item slot 1 │ 0x38 u64 block table │
//! 0x40 u64 condition list │ 0x48 u64 extra │ 0x50 u64 name`.
//!
//! **Not decoded, and not claimed:** the per-record block table (0–12 sixteen-byte entries — the
//! PS4 form of Legacy's slot block), the `.gas` bytecode, and the semantics of the priority-like /
//! time-like / kind words (handoff 05 labels them as candidates for gen7; the same labels are
//! carried in the view, behind disclosure).
//!
//! ## Deliberate deviation from the file's own count
//!
//! The header's `n` counts two more records than the pointer list holds: the two `u64` after the
//! list are the first T2 row, not record pointers (handoff 15 §1 records this on every title).
//! This parser reads `n − 2` records and reports the discrepancy in [`Ctx0Ps4Notes`] rather than
//! trusting `n` or silently dropping the observation.

use crate::{Moir, SpeechError};

const FF64: u64 = u64::MAX;
const W: &str = "0XTC";

/// What the walk observed that the shared [`Ctx0`] shape has nowhere to put.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Ctx0Ps4Notes {
    /// File version word: 3 (NHL 15–19) or 4 (NHL 20 / 21).
    pub version: u32,
    /// The header's record count, which is two more than the pointer list holds.
    pub declared_records: u32,
    /// 16-byte block-table entries across all records — parsed as a count only.
    pub block_entries: usize,
    /// Group entries at 0x50.
    pub groups: u32,
}

/// Every read is offset-checked: a pointer taken from the file can be any 64-bit value, so `o + 4`
/// must not be evaluated before it is known to fit (the same discipline the R2 correction applied
/// to the texture writers — validate before arithmetic).
fn le32(b: &[u8], o: usize) -> Result<u32, SpeechError> {
    o.checked_add(4)
        .and_then(|e| b.get(o..e))
        .map(|s| u32::from_le_bytes(s.try_into().unwrap()))
        .ok_or_else(|| SpeechError::Malformed { what: W, reason: format!("u32 at {o:#x} is past the end ({:#x})", b.len()) })
}

fn le64(b: &[u8], o: usize) -> Result<u64, SpeechError> {
    o.checked_add(8)
        .and_then(|e| b.get(o..e))
        .map(|s| u64::from_le_bytes(s.try_into().unwrap()))
        .ok_or_else(|| SpeechError::Malformed { what: W, reason: format!("u64 at {o:#x} is past the end ({:#x})", b.len()) })
}

/// A NUL-terminated string at an absolute offset.
fn cstr(b: &[u8], o: usize) -> Result<String, SpeechError> {
    let s = b.get(o..).ok_or_else(|| SpeechError::Malformed { what: W, reason: format!("string offset {o:#x} is past the end") })?;
    let z = s.iter().position(|x| *x == 0).ok_or_else(|| SpeechError::Malformed { what: W, reason: format!("string at {o:#x} is not terminated") })?;
    Ok(String::from_utf8_lossy(&s[..z]).into_owned())
}

/// Is this the PS4 `0XTC` container?
pub fn is_ctx0_ps4(b: &[u8]) -> bool {
    b.len() >= 8 && &b[..8] == b"0XTCTTEK"
}

/// The PS4 MOIR name table: little-endian, 24-byte entries (`u64 0 │ u64 name ptr │ u64 id`) from
/// `0x40` — a layout [`crate::parse_moir`]'s big-endian candidates do not fit (handoff 15 §2).
pub fn parse_moir_le(b: &[u8]) -> Result<Moir, SpeechError> {
    const M: &str = "MOIR(le)";
    if b.len() < 0x40 || &b[..4] != b"MOIR" {
        return Err(SpeechError::BadMagic { what: M, magic: String::from_utf8_lossy(b.get(..4).unwrap_or(b)).into_owned() });
    }
    let n = u16::from_le_bytes(b[10..12].try_into().unwrap()) as usize;
    let namespace = u16::from_le_bytes(b[16..18].try_into().unwrap());
    let table_end = 0x40 + 24 * n;
    if table_end > b.len() {
        return Err(SpeechError::Malformed { what: M, reason: format!("{n} entries of 24 bytes from 0x40 do not fit in {:#x}", b.len()) });
    }
    let mut entries = Vec::with_capacity(n);
    for i in 0..n {
        let o = 0x40 + 24 * i;
        if le64(b, o)? != 0 {
            return Err(SpeechError::Malformed { what: M, reason: format!("entry {i}: the leading word is not zero") });
        }
        let ptr = le64(b, o + 8)? as usize;
        let id = le64(b, o + 16)?;
        if ptr < table_end || ptr >= b.len() {
            return Err(SpeechError::Malformed { what: M, reason: format!("entry {i}: name pointer {ptr:#x} is outside the string region") });
        }
        let id = u16::try_from(id).map_err(|_| SpeechError::Malformed { what: M, reason: format!("entry {i}: id {id} does not fit the 16-bit event id") })?;
        entries.push((id, cstr(b, ptr)?));
    }
    Ok(Moir { namespace, entries })
}

/// Parse a PS4 `pbp_contexts.bin`. `speech` / `ctx` are the title's speech and context MOIR tables
/// (read with [`parse_moir_le`]), used to type each item and to check the name spelled in the
/// record against the table's own name — the check handoff 15 §2 reports at 100 % on every title.
pub fn parse_ctx0_ps4(b: &[u8], speech: Option<&Moir>, ctx: Option<&Moir>) -> Result<(crate::Ctx0, Ctx0Ps4Notes), SpeechError> {
    let bad = |d: String| SpeechError::Malformed { what: W, reason: d };
    if !is_ctx0_ps4(b) {
        return Err(SpeechError::BadMagic { what: W, magic: String::from_utf8_lossy(b.get(..8).unwrap_or(b)).into_owned() });
    }
    let version = le32(b, 0x08)?;
    if version != 3 && version != 4 {
        return Err(bad(format!("0XTC version {version} is neither of the measured versions (3 on NHL 15–19, 4 on NHL 20 / 21)")));
    }
    let n = le32(b, 0x0C)? as usize;
    let a = le32(b, 0x10)? as usize;
    let n2 = le32(b, 0x14)? as usize;
    if n != n2 {
        return Err(bad(format!("the two record counts disagree ({n} vs {n2})")));
    }
    if n < 2 {
        return Err(bad(format!("record count {n} is below the two trailing non-record pointers")));
    }
    let pool1 = le64(b, 0x30)? as usize;
    let pool2 = le64(b, 0x38)? as usize;
    if pool1 >= pool2 || pool2 > b.len() {
        return Err(bad(format!("pools are not ordered inside the file (pool1 {pool1:#x}, pool2 {pool2:#x}, len {:#x})", b.len())));
    }
    // The pointer list holds n − 2 entries (handoff 15 §1); the two after it are the first T2 row.
    let count = n - 2;
    let list = 0x50 + 16 * a;
    if list.checked_add(8 * count).is_none_or(|e| e > pool1) {
        return Err(bad(format!("the record-pointer list does not fit before pool 1 (list {list:#x}, {count} pointers, pool1 {pool1:#x})")));
    }
    let mut ptrs = Vec::with_capacity(count);
    for i in 0..count {
        let p = le64(b, list + 8 * i)? as usize;
        if p < list || p.checked_add(0x58).is_none_or(|e| e > pool1) {
            return Err(bad(format!("record pointer {i} ({p:#x}) does not address a 0x58-byte header before pool 1")));
        }
        ptrs.push(p);
    }
    let speech_ns = speech.map(|m| m.namespace);
    let ctx_ns = ctx.map(|m| m.namespace);
    fn name_of<'m>(m: Option<&'m Moir>, id: u16) -> Option<&'m str> {
        m.and_then(|t| t.entries.iter().find(|(i, _)| *i == id).map(|(_, s)| s.as_str()))
    }

    let mut notes = Ctx0Ps4Notes { version, declared_records: n as u32, block_entries: 0, groups: a as u32 };
    let mut trigger_names: Vec<String> = Vec::new();
    let mut records = Vec::with_capacity(count);
    for (idx, &r) in ptrs.iter().enumerate() {
        let context_id = le32(b, r)?;
        let w1 = le32(b, r + 4)?;
        let ncond = b[r + 8] as usize;
        let nextra = b[r + 9] as usize;
        let nblk = b[r + 10] as usize;
        let w3 = le32(b, r + 12)?;
        let time_like = u16::from_le_bytes(b[r + 16..r + 18].try_into().unwrap()) as u32;
        let kind = u16::from_le_bytes(b[r + 18..r + 20].try_into().unwrap()) as u32;
        notes.block_entries += nblk;

        let trig_off = le64(b, r + 0x20)?;
        let trigger = if trig_off == FF64 {
            None
        } else {
            let at = pool2.checked_add(trig_off as usize).ok_or_else(|| bad(format!("record {idx}: trigger offset overflows")))?;
            if at >= b.len() {
                return Err(bad(format!("record {idx}: trigger offset {trig_off:#x} is past pool 2")));
            }
            let t = cstr(b, at)?;
            if !trigger_names.contains(&t) {
                trigger_names.push(t.clone());
            }
            Some(t)
        };
        let name = cstr(b, le64(b, r + 0x50)? as usize)?;

        // Items: slot 0 and slot 1.
        let mut items = Vec::new();
        for slot in 0..2u32 {
            let ip = le64(b, r + 0x28 + 8 * slot as usize)?;
            if ip == 0 || ip == FF64 {
                continue;
            }
            let ip = ip as usize;
            if ip.checked_add(56).is_none_or(|e| e > b.len()) {
                return Err(bad(format!("record {idx}: item slot {slot} at {ip:#x} does not hold a 56-byte item")));
            }
            let refw = le32(b, ip + 8)?;
            let namespace = (refw & 0xFFFF) as u32;
            let event_id = (refw >> 16) as u16;
            let item_name = cstr(b, le64(b, ip + 40)? as usize)?;
            let kind_str = if Some(namespace as u16) == speech_ns {
                "speech"
            } else if Some(namespace as u16) == ctx_ns {
                "context"
            } else {
                "unknown-namespace"
            };
            // Handoff 15 §2: the name spelled in the record equals the MOIR name, 100 % on every
            // title. Enforced as a refusal, like the gen7 parser's checks.
            let table = if kind_str == "speech" { speech } else if kind_str == "context" { ctx } else { None };
            if let Some(expected) = name_of(table, event_id) {
                if expected != item_name {
                    return Err(bad(format!("record {idx}: item names its event `{item_name}` but namespace {namespace} id {event_id} is `{expected}`")));
                }
            } else if table.is_some() {
                return Err(bad(format!("record {idx}: item event id {event_id} is not in the {kind_str} name table")));
            }
            let nparam = le64(b, ip + 48)? as usize;
            let mut params = Vec::with_capacity(nparam);
            for k in 0..nparam {
                let po = ip + 56 + 16 * k;
                let pk = le64(b, po)?;
                let pp = le64(b, po + 8)? as usize;
                match pk {
                    1 => params.push(crate::CtxParam { kind: 1, value: Some(le32(b, pp)?), blob: None, args: Vec::new() }),
                    2 => {
                        let blob = le32(b, pp + 8)?;
                        let nargs = le32(b, pp + 12)? as usize;
                        let mut args = Vec::with_capacity(nargs);
                        for j in 0..nargs {
                            let ao = pp + 24 + 16 * j;
                            args.push(le32(b, le64(b, ao + 8)? as usize)?);
                        }
                        params.push(crate::CtxParam { kind: 2, value: None, blob: Some(blob), args });
                    }
                    other => return Err(bad(format!("record {idx}: parameter {k} has kind {other}, not the measured 1 (immediate) or 2 (script call)"))),
                }
            }
            items.push(crate::CtxItem { slot, namespace, event_id, kind: kind_str, name: item_name, params });
        }

        // Conditions.
        let mut conditions = Vec::with_capacity(ncond);
        if ncond > 0 {
            // The word at 0x40 addresses an array of `u64` **pointers**, one per condition, each to
            // a 24-byte condition struct — not a flat array of structs. Handoff 15 §1 gives the
            // struct but not how the list is addressed; measured here on NHL 15 (440 / 440 sampled
            // conditions resolve as a pointer array, 67–88 / 440 for any flat stride) and asserted
            // on all seven titles by `tests/ps4_contexts.rs`.
            let cl = le64(b, r + 0x40)? as usize;
            for k in 0..ncond {
                let co = le64(b, cl.checked_add(8 * k).ok_or_else(|| bad(format!("record {idx}: condition list overflows")))?)? as usize;
                let nargs = le32(b, co)? as usize;
                let blob = le32(b, co + 4)?;
                let argp = le64(b, co + 8)?;
                let noff = le64(b, co + 16)? as usize;
                let cname = cstr(b, pool1.checked_add(noff).ok_or_else(|| bad(format!("record {idx}: condition {k} name offset overflows")))?)?;
                // Condition arguments: the struct carries a count and a pointer, but handoff 15
                // does not specify the element layout and the gen7 form does not fit it. The count
                // is kept (it is what the listing shows); the values are **not** decoded, so none
                // are invented. `args` stays empty and `nargs` is reported through the name.
                let cname = if nargs == 0 { cname } else { format!("{cname} ({nargs} args, values not decoded)") };
                let _ = argp;
                conditions.push(crate::CtxCondition { name: cname, blob, args: Vec::new() });
            }
        }

        // Extra words, when the record carries them.
        let mut extra = Vec::with_capacity(nextra);
        if nextra > 0 {
            let ep = le64(b, r + 0x48)?;
            if ep != FF64 && ep != 0 {
                for k in 0..nextra {
                    extra.push(le32(b, ep as usize + 4 * k)?);
                }
            }
        }

        records.push(crate::CtxRecord {
            index: idx as u32,
            context_id,
            name,
            trigger,
            w1,
            w3,
            w4: (kind << 16) | time_like,
            items,
            conditions,
            extra,
            slot2_block: None,
        });
    }

    let condition_names = records.iter().map(|r| r.conditions.len()).sum();
    Ok((
        crate::Ctx0 { platform: "ps4", records, trigger_names, condition_names, unreferenced_condition_names: 0 },
        notes,
    ))
}
