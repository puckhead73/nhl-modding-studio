//! `tdb-speech` — **read-only** parsers for the gen7 (PS3 / Xbox 360 NHL 11–Legacy) commentary
//! data of `audio/speech/`, consumed for the League / Team / Player audio listing (roadmap 1.5 /
//! 2.7 / 3.5). Every layout here is the research lane's *parsed* reading — research 2026-09-20
//! handoff 13 rev. 4 (`EVT0`, `MOIR`, sentence hash, `KWD0`, `SBKR`) and 2026-09-21 handoff 04
//! (the speech model read from EA's own XML source form of NHL 09 / 10: phrase = one fixed
//! `SecretName_SampleBank` tag + event-parameter tags + a repetition pool; the `GRF0` graffiti
//! table `(tag, value, NumTags)` → sample ids) — verified on every record of every measured set
//! (16 trees). This crate binds to those readings; the crate's integration test compares its
//! counts with the research data (`r27-gen7-speech-model.json`) on the real trees.
//!
//! What this crate does **not** do: decode any sample (no codec), decide which context fires or
//! which take plays (runtime, unobserved), read the `CTX0` context records (a later step), the
//! `.gas` bytecode, or the older NHL 07 / 08 `pbp_eng.{big,csi,...}` system; and it writes nothing.
//! Malformed inputs are errors (bounded reads everywhere), never panics or empty tables.
//!
//! All integers are big-endian. Text is Latin-1.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, HashMap};

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SpeechError {
    #[error("{what}: file is {have} bytes, shorter than the {needed} needed at offset {at}")]
    Truncated { what: &'static str, at: usize, needed: usize, have: usize },
    #[error("{what}: unexpected magic {magic:?}")]
    BadMagic { what: &'static str, magic: String },
    #[error("{what}: unsupported version {major}.{minor}")]
    UnsupportedVersion { what: &'static str, major: u16, minor: u16 },
    #[error("{what}: {reason}")]
    Malformed { what: &'static str, reason: String },
}

pub mod ctx0;
pub use ctx0::{is_ctx0_v31, parse_ctx0, parse_ctx0_v31, Ctx0, CtxCondition, CtxItem, CtxParam, CtxRecord};
pub mod ctx0_ps4;
pub use ctx0_ps4::{is_ctx0_ps4, parse_ctx0_ps4, parse_moir_le, Ctx0Ps4Notes};

fn u32_at(b: &[u8], at: usize, what: &'static str) -> Result<u32, SpeechError> {
    b.get(at..at + 4)
        .map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or(SpeechError::Truncated { what, at, needed: 4, have: b.len() })
}

fn u16_at(b: &[u8], at: usize, what: &'static str) -> Result<u16, SpeechError> {
    b.get(at..at + 2)
        .map(|s| u16::from_be_bytes([s[0], s[1]]))
        .ok_or(SpeechError::Truncated { what, at, needed: 2, have: b.len() })
}

/// NUL-terminated Latin-1 string at `at` (bounded to the buffer; a missing terminator is an error).
fn cstr_at(b: &[u8], at: usize, what: &'static str) -> Result<String, SpeechError> {
    let tail = b.get(at..).ok_or(SpeechError::Truncated { what, at, needed: 1, have: b.len() })?;
    let end = tail.iter().position(|x| *x == 0).ok_or(SpeechError::Malformed { what, reason: format!("unterminated string at {at}") })?;
    Ok(tail[..end].iter().map(|&c| c as char).collect())
}

/// Whether the 8-byte magic carries a platform tag (`PS30` / `X360`) after the 4-byte type.
fn platform_tag(b: &[u8]) -> Option<&'static str> {
    match b.get(4..8) {
        Some(b"PS30") => Some("PS3"),
        Some(b"X360") => Some("X360"),
        _ => None,
    }
}

/// FNV-1 32-bit (offset 0x811c9dc5, prime 0x01000193, multiply then xor) of a Latin-1 name —
/// the sentence hash-table rule (slot = `fnv1_32(name) % slots`), verified on every sentence of
/// every measured binary set.
pub fn fnv1_32(name: &str) -> u32 {
    let mut h: u32 = 0x811c_9dc5;
    for c in name.chars() {
        h = h.wrapping_mul(0x0100_0193) ^ (c as u32 & 0xff);
    }
    h
}

// ------------------------------------------------------------------ EVT0

/// One entry of the `EVT0` parameter table: an `Int` parameter or an enum with its values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parameter {
    pub id: u32,
    pub name: String,
    /// `None` for an `Int` parameter; `Some(values)` for an enum (`name → value`, bit flags).
    pub enum_values: Option<BTreeMap<String, u32>>,
}

/// One speech event: its 16-bit id (EA's `interfaceCrc`), name and ordered parameter ids.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    pub id: u16,
    pub name: String,
    pub params: Vec<u32>,
    /// `record_list` (the main pointer list) or `secondary_list` (groups after the parameter
    /// table; records the main list does not carry).
    pub source: &'static str,
}

/// A parsed `EVT0` file (v3.3 on NHL 13–Legacy, v1.2 on NHL 11 / 12).
#[derive(Debug, Clone)]
pub struct Evt0 {
    pub platform: Option<&'static str>,
    pub version: (u16, u16),
    pub namespace: u16,
    pub params: Vec<Parameter>,
    pub events: Vec<Event>,
    /// Records in the pointer lists that are neither parameters nor events of the measured shape.
    pub unclassified: usize,
}

impl Evt0 {
    pub fn param_name(&self, id: u32) -> Option<&str> {
        self.params.iter().find(|p| p.id == id).map(|p| p.name.as_str())
    }
    pub fn event(&self, name: &str) -> Option<&Event> {
        self.events.iter().find(|e| e.name == name)
    }
}

/// Parse `pbp_eng_speecheventsubscr.bin` (research 2026-09-20 handoff 13 §1; 2026-09-21 handoff
/// 04 §2 for the enum entries). Header (8-byte magic): `EVT0 <tag> | u16 major, u16 minor | u32 0 |
/// u16 namespace, u16 0 | u32 record count | u32 group count G | u32 0x24 | G × u32 group ptr |
/// count × u32 record ptr`; groups[0] → the parameter pointer list; groups[1..] → further event
/// records. Event record: `u32 size (= 4n) | u32 n | 12 zero | u32 ns<<16|id | zero run | n × u32
/// param id | name`. Parameter entry: `u32 3 | id | 0 | 0 | 0 | name` (Int) or `u32 1 | id | count |
/// 0 | ptr | name` (enum; `ptr → count × ptr → u32 0 | u32 value | name`). NHL 11's 4-byte-magic
/// file is the same layout with every header field 4 bytes earlier (handled by `shift`).
pub fn parse_evt0(b: &[u8]) -> Result<Evt0, SpeechError> {
    const W: &str = "EVT0";
    if b.len() < 0x20 || &b[..4] != b"EVT0" {
        return Err(SpeechError::BadMagic { what: W, magic: String::from_utf8_lossy(b.get(..4).unwrap_or(b)).into_owned() });
    }
    let platform = platform_tag(b);
    // 8-byte magic: header at 8; 4-byte magic (NHL 11): header at 4.
    let h = if platform.is_some() { 8 } else { 4 };
    let version = (u16_at(b, h, W)?, u16_at(b, h + 2, W)?);
    if !matches!(version, (3, 3) | (1, 2)) {
        return Err(SpeechError::UnsupportedVersion { what: W, major: version.0, minor: version.1 });
    }
    let namespace = u16_at(b, h + 8, W)?;
    let n = u32_at(b, h + 12, W)? as usize;
    let g = u32_at(b, h + 16, W)? as usize;
    let lists = h + 24;
    if lists + 4 * (g + n) > b.len() {
        return Err(SpeechError::Truncated { what: W, at: lists, needed: 4 * (g + n), have: b.len() });
    }
    let groups: Vec<usize> = (0..g).map(|i| u32_at(b, lists + 4 * i, W).map(|v| v as usize)).collect::<Result<_, _>>()?;
    let ptrs: Vec<usize> = (0..n).map(|i| u32_at(b, lists + 4 * g + 4 * i, W).map(|v| v as usize)).collect::<Result<_, _>>()?;
    let ptrset: std::collections::HashSet<usize> = ptrs.iter().copied().collect();
    // Parameter table: groups[0] points at a list of pointers into the record set.
    let mut params = Vec::new();
    let mut param_offs = std::collections::HashSet::new();
    if let Some(&q0) = groups.first() {
        let mut q = q0;
        while q + 4 <= b.len() {
            let r = u32_at(b, q, W)? as usize;
            if !ptrset.contains(&r) {
                break;
            }
            let w0 = u32_at(b, r, W)?;
            let id = u32_at(b, r + 4, W)?;
            let count = u32_at(b, r + 8, W)?;
            let w4 = u32_at(b, r + 16, W)?;
            let name = cstr_at(b, r + 20, W)?;
            let enum_values = if w0 == 1 && count > 0 && w4 != 0 {
                let mut vals = BTreeMap::new();
                for k in 0..count as usize {
                    let e = u32_at(b, w4 as usize + 4 * k, W)? as usize;
                    if u32_at(b, e, W)? != 0 {
                        return Err(SpeechError::Malformed { what: W, reason: format!("enum {name}: entry {k} at {e} does not start with 0") });
                    }
                    let value = u32_at(b, e + 4, W)?;
                    vals.insert(cstr_at(b, e + 8, W)?, value);
                }
                Some(vals)
            } else {
                None
            };
            params.push(Parameter { id, name, enum_values });
            param_offs.insert(r);
            q += 4;
        }
    }
    let mut events = Vec::new();
    let mut unclassified = 0usize;
    let mut seen = std::collections::HashSet::new();
    let sources = ptrs.iter().map(|p| (*p, "record_list")).chain(groups.iter().skip(1).filter(|p| !ptrset.contains(p)).map(|p| (*p, "secondary_list")));
    for (p, source) in sources {
        if param_offs.contains(&p) || !seen.insert(p) {
            continue;
        }
        let size = u32_at(b, p, W)?;
        let cnt = u32_at(b, p + 4, W)? as usize;
        let idword = u32_at(b, p + 20, W)?;
        let zero12 = b.get(p + 8..p + 20).is_some_and(|z| z.iter().all(|x| *x == 0));
        if cnt >= 4096 || size as usize != 4 * cnt || (idword >> 16) as u16 != namespace || !zero12 {
            unclassified += 1;
            continue;
        }
        let mut q = p + 24;
        while q + 4 <= b.len() && u32_at(b, q, W)? == 0 {
            q += 4;
        }
        if cnt == 0 {
            while q < b.len() && b[q] == 0 {
                q += 1;
            }
        }
        let fixed = q - p;
        let prm: Vec<u32> = (0..cnt).map(|i| u32_at(b, p + fixed + 4 * i, W)).collect::<Result<_, _>>()?;
        let name = cstr_at(b, p + fixed + 4 * cnt, W)?;
        events.push(Event { id: (idword & 0xffff) as u16, name, params: prm, source });
    }
    Ok(Evt0 { platform, version, namespace, params, events, unclassified })
}

// ------------------------------------------------------------------ MOIR

/// The `MOIR` id → name table (`pbp_eng_speecheventdef.csi`): entries sorted by id; 16-bit ids
/// repeat, so an id can carry several names (every candidate is kept; nothing is chosen).
#[derive(Debug, Clone)]
pub struct Moir {
    pub namespace: u16,
    pub entries: Vec<(u16, String)>,
}

impl Moir {
    /// Every name an id carries, in table order.
    pub fn names(&self, id: u16) -> Vec<&str> {
        self.entries.iter().filter(|(i, _)| *i == id).map(|(_, n)| n.as_str()).collect()
    }
    pub fn ambiguous_ids(&self) -> BTreeMap<u16, Vec<String>> {
        let mut m: BTreeMap<u16, Vec<String>> = BTreeMap::new();
        for (i, n) in &self.entries {
            m.entry(*i).or_default().push(n.clone());
        }
        m.retain(|_, v| v.len() > 1);
        m
    }
}

/// Parse a `MOIR` file: `MOIR | 00 03 03 00 | u16 w | u16 count | u32 0 | u16 namespace | … |
/// count × entries | string pool to EOF`; entry = `u32 string ptr | u16 id | pad` at stride 24
/// (table @0x40, 12-byte zero prefix, PS3) or stride 12 (@0x28, 4-byte prefix, Xbox 360) — the
/// layout is the one whose first string pointer equals `table_start + stride × count`.
pub fn parse_moir(b: &[u8]) -> Result<Moir, SpeechError> {
    const W: &str = "MOIR";
    if b.len() < 0x28 || &b[..4] != b"MOIR" {
        return Err(SpeechError::BadMagic { what: W, magic: String::from_utf8_lossy(b.get(..4).unwrap_or(b)).into_owned() });
    }
    let n = u16_at(b, 10, W)? as usize;
    let namespace = u16_at(b, 16, W)?;
    let mut layout = None;
    for (start, stride, pre) in [(0x40usize, 24usize, 12usize), (0x28, 12, 4), (0x28, 12, 0), (0x30, 12, 0), (0x40, 16, 0)] {
        if start + stride * n + 4 <= b.len() && u32_at(b, start + pre, W)? as usize == start + stride * n {
            layout = Some((start, stride, pre));
            break;
        }
    }
    let Some((start, stride, pre)) = layout else {
        return Err(SpeechError::Malformed { what: W, reason: format!("no measured table layout fits {n} entries in {} bytes", b.len()) });
    };
    let mut entries = Vec::with_capacity(n);
    let mut last_ptr = 0usize;
    for i in 0..n {
        let o = start + stride * i;
        let ptr = u32_at(b, o + pre, W)? as usize;
        let id = u16_at(b, o + pre + 4, W)?;
        if ptr < start + stride * n || ptr <= last_ptr && i > 0 {
            return Err(SpeechError::Malformed { what: W, reason: format!("entry {i}: string pointer {ptr} is not increasing past the table") });
        }
        last_ptr = ptr;
        entries.push((id, cstr_at(b, ptr, W)?));
    }
    Ok(Moir { namespace, entries })
}

// ------------------------------------------------------------------ SPC0 sentences

/// One tag of a phrase: the fixed sample-group pair or a reference to the event's parameter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PhraseTag {
    /// `(TagId, TagValue)` — the `SecretName_SampleBank` group on every measured phrase (one per phrase).
    Fixed { tag: u32, value: u32 },
    /// The event's parameter at `event_index` supplies the value of tag `tag` at runtime.
    EventRef { tag: u32, event_index: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Phrase {
    pub pool_id: u32,
    pub num_parameters: u32,
    pub tags: Vec<PhraseTag>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Body {
    pub id: u32,
    /// The v5 `100 / 1` words or the v4 `0 / 0 / w4` words — constant, not decoded.
    pub words: Vec<u32>,
    pub phrases: Vec<Phrase>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Sentence {
    /// The sentence name = the speech event name that triggers it.
    pub name: String,
    pub slot: u32,
    /// Whether `fnv1_32(name) % slots == slot` (true on every measured sentence).
    pub slot_ok: bool,
    pub bodies: Vec<Body>,
}

#[derive(Debug, Clone)]
pub struct Sentences {
    pub platform: Option<&'static str>,
    pub version: (u16, u16),
    pub slots: u32,
    pub sentences: Vec<Sentence>,
}

impl Sentences {
    pub fn by_name(&self, name: &str) -> Vec<&Sentence> {
        self.sentences.iter().filter(|s| s.name == name).collect()
    }
}

/// Parse `pbp_eng_sentencedata.bin` (`SPC0` v5.3 on NHL 13–Legacy; v4.2 / v4.4 on NHL 11 / 12):
/// `SPC0 [<tag>] | u16 major, u16 minor | u32 0 | u32 slot count N | u32 0 | N × (u32 n, u32 ptr)`;
/// slot ptr → `n × (u32 a, u32 ptr)`; sentence: `u32 c | u32 w0 | c × u32 body ptr | name`;
/// v5 body `u32 id | u32 100 | u32 1 | u32 nphrases | ptrs`, v5 phrase `u8 pool | 0 | u8 numParameters
/// | 0 | u8 nwords | ff ff ff | nwords × u32 (tag, value pairs) | (numParameters − nwords/2) × (u8 1,
/// idx, idx+1, idx | u32 tag | ffffffff)`; v4 body `0 | 0 | u32 nphrases | u32 id | u32 w4 | ptrs`,
/// v4 phrase `0 | u32 pool | 0 | [0 on v4.4] | u32 numParameters | numParameters × (u32 kind 2 =
/// fixed / 1 = event ref, u32 tag, u32 value)`.
pub fn parse_sentences(b: &[u8]) -> Result<Sentences, SpeechError> {
    const W: &str = "SPC0";
    if b.len() < 0x20 || &b[..4] != b"SPC0" {
        return Err(SpeechError::BadMagic { what: W, magic: String::from_utf8_lossy(b.get(..4).unwrap_or(b)).into_owned() });
    }
    let platform = platform_tag(b);
    let ml = if platform.is_some() { 8 } else { 4 };
    let version = (u16_at(b, ml, W)?, u16_at(b, ml + 2, W)?);
    if !matches!(version, (5, 3) | (4, 2) | (4, 4)) {
        return Err(SpeechError::UnsupportedVersion { what: W, major: version.0, minor: version.1 });
    }
    let n = u32_at(b, ml + 8, W)?;
    let base = ml + 16;
    if base + 8 * n as usize > b.len() {
        return Err(SpeechError::Truncated { what: W, at: base, needed: 8 * n as usize, have: b.len() });
    }
    let mut sentences = Vec::new();
    for i in 0..n {
        let cnt = u32_at(b, base + 8 * i as usize, W)? as usize;
        let ptr = u32_at(b, base + 4 + 8 * i as usize, W)?;
        if cnt == 0 || ptr == 0xffff_ffff {
            continue;
        }
        if cnt > 4096 {
            return Err(SpeechError::Malformed { what: W, reason: format!("slot {i}: {cnt} variants") });
        }
        let ptr = ptr as usize;
        for j in 0..cnt {
            let vp = u32_at(b, ptr + 4 + 8 * j, W)? as usize;
            let c = u32_at(b, vp, W)? as usize;
            if c > 4096 {
                return Err(SpeechError::Malformed { what: W, reason: format!("slot {i} sentence {j}: {c} bodies") });
            }
            let body_ptrs: Vec<usize> = (0..c).map(|k| u32_at(b, vp + 8 + 4 * k, W).map(|v| v as usize)).collect::<Result<_, _>>()?;
            let name = cstr_at(b, vp + 8 + 4 * c, W)?;
            let mut bodies = Vec::with_capacity(c);
            for bp in body_ptrs {
                let (id, words, nph, pptr) = if version.0 == 5 {
                    (u32_at(b, bp, W)?, vec![u32_at(b, bp + 4, W)?, u32_at(b, bp + 8, W)?], u32_at(b, bp + 12, W)? as usize, bp + 16)
                } else {
                    (u32_at(b, bp + 12, W)?, vec![u32_at(b, bp, W)?, u32_at(b, bp + 4, W)?, u32_at(b, bp + 16, W)?], u32_at(b, bp + 8, W)? as usize, bp + 20)
                };
                if nph > 4096 {
                    return Err(SpeechError::Malformed { what: W, reason: format!("{name}: body at {bp} declares {nph} phrases") });
                }
                let mut phrases = Vec::with_capacity(nph);
                for k in 0..nph {
                    let pp = u32_at(b, pptr + 4 * k, W)? as usize;
                    let mut tags = Vec::new();
                    let (pool_id, num_parameters) = if version.0 == 5 {
                        let hdr = b.get(pp..pp + 8).ok_or(SpeechError::Truncated { what: W, at: pp, needed: 8, have: b.len() })?;
                        let (pool, npar, nw) = (hdr[0] as u32, hdr[2] as u32, hdr[4] as usize);
                        if hdr[1] != 0 || hdr[3] != 0 || &hdr[5..8] != b"\xff\xff\xff" || nw % 2 != 0 {
                            return Err(SpeechError::Malformed { what: W, reason: format!("{name}: phrase at {pp} is not the measured v5 shape ({hdr:02x?})") });
                        }
                        let words: Vec<u32> = (0..nw).map(|m| u32_at(b, pp + 8 + 4 * m, W)).collect::<Result<_, _>>()?;
                        for m in (0..nw).step_by(2) {
                            tags.push(PhraseTag::Fixed { tag: words[m], value: words[m + 1] });
                        }
                        let refs = (npar as usize).saturating_sub(nw / 2);
                        let mut o = pp + 8 + 4 * nw;
                        for _ in 0..refs {
                            let h = b.get(o..o + 4).ok_or(SpeechError::Truncated { what: W, at: o, needed: 12, have: b.len() })?;
                            let tag = u32_at(b, o + 4, W)?;
                            let ff = u32_at(b, o + 8, W)?;
                            if h[0] != 1 || h[2] != h[1].wrapping_add(1) || h[3] != h[1] || ff != 0xffff_ffff {
                                return Err(SpeechError::Malformed { what: W, reason: format!("{name}: event ref at {o} is not the measured shape ({h:02x?} {ff:08x})") });
                            }
                            tags.push(PhraseTag::EventRef { tag, event_index: h[1] });
                            o += 12;
                        }
                        (pool, npar)
                    } else {
                        let (pool, npar, mut o) = if version == (4, 2) {
                            (u32_at(b, pp + 4, W)?, u32_at(b, pp + 12, W)?, pp + 16)
                        } else {
                            (u32_at(b, pp + 4, W)?, u32_at(b, pp + 16, W)?, pp + 20)
                        };
                        if npar > 256 {
                            return Err(SpeechError::Malformed { what: W, reason: format!("{name}: phrase at {pp} declares {npar} parameters") });
                        }
                        for _ in 0..npar {
                            let (kind, tag, v) = (u32_at(b, o, W)?, u32_at(b, o + 4, W)?, u32_at(b, o + 8, W)?);
                            match kind {
                                2 => tags.push(PhraseTag::Fixed { tag, value: v }),
                                1 => tags.push(PhraseTag::EventRef { tag, event_index: v as u8 }),
                                other => return Err(SpeechError::Malformed { what: W, reason: format!("{name}: phrase tag kind {other} at {o}") }),
                            }
                            o += 12;
                        }
                        (pool, npar)
                    };
                    phrases.push(Phrase { pool_id, num_parameters, tags });
                }
                bodies.push(Body { id, words, phrases });
            }
            let slot_ok = fnv1_32(&name) % n == i;
            sentences.push(Sentence { name, slot: i, slot_ok, bodies });
        }
    }
    Ok(Sentences { platform, version, slots: n, sentences })
}

// ------------------------------------------------------------------ GRF0 graffiti

/// One graffiti table: the sample ids carrying `(tag, value)` with `num_tags` tags in total.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GraffitiTable {
    pub tag: u32,
    pub value: u32,
    pub num_tags: u32,
    pub sample_ids: Vec<u32>,
    /// `<tag>_<value>_<numtags>` on the named v2.1 files (NHL 11 / 12); `None` on v3.3.
    pub name: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Graffiti {
    pub version: (u16, u16),
    /// Whether the entries carry names (the 4-byte-magic v2.1 file of NHL 11; NHL 12's 8-byte-magic
    /// v2.1 file and the v3.3 files do not).
    pub named: bool,
    pub tables: Vec<GraffitiTable>,
    /// Table entries whose pointers ran past the file (listed, not fatal).
    pub malformed: usize,
    index: HashMap<(u32, u32, u32), usize>,
}

impl Graffiti {
    pub fn table(&self, tag: u32, value: u32, num_tags: u32) -> Option<&GraffitiTable> {
        self.index.get(&(tag, value, num_tags)).map(|i| &self.tables[*i])
    }
    /// The tables of `(tag, value)` at any tag count.
    pub fn tables_for(&self, tag: u32, value: u32) -> Vec<&GraffitiTable> {
        self.tables.iter().filter(|t| t.tag == tag && t.value == value).collect()
    }
    /// The tag-name prefixes a named (v2.1) file carries: `tag id → name`.
    pub fn tag_names(&self) -> BTreeMap<u32, String> {
        let mut out = BTreeMap::new();
        for t in &self.tables {
            if let Some(n) = &t.name {
                let suffix = format!("_{}_{}", t.value, t.num_tags);
                if let Some(stem) = n.strip_suffix(&suffix) {
                    out.entry(t.tag).or_insert_with(|| stem.to_string());
                }
            }
        }
        out
    }
}

/// Parse `pbp_eng_graffitidata.bin`: 8-byte magic (NHL 12 v2.1, NHL 13+ v3.3): slots @0x14, table
/// @0x1c, no names; 4-byte magic (NHL 11 v2.1): slots @0x10, table @0x18, named entries. `slots × (u32 count, u32 ptr rel.
/// table)` → `count × (u32 tag | u32 value | u32 num_tags | u32 ptr rel. table)` → `u32 num_refs |
/// refs [| name on v2.1]`.
pub fn parse_graffiti(b: &[u8]) -> Result<Graffiti, SpeechError> {
    const W: &str = "GRF0";
    if b.len() < 0x20 || &b[..4] != b"GRF0" {
        return Err(SpeechError::BadMagic { what: W, magic: String::from_utf8_lossy(b.get(..4).unwrap_or(b)).into_owned() });
    }
    let named = platform_tag(b).is_none();
    let (version, n, base) = if named {
        ((u16_at(b, 4, W)?, u16_at(b, 6, W)?), u32_at(b, 0x10, W)? as usize, 0x18usize)
    } else {
        ((u16_at(b, 8, W)?, u16_at(b, 10, W)?), u32_at(b, 0x14, W)? as usize, 0x1cusize)
    };
    if !matches!(version, (2, 1) | (3, 3)) {
        return Err(SpeechError::UnsupportedVersion { what: W, major: version.0, minor: version.1 });
    }
    if base + 8 * n > b.len() {
        return Err(SpeechError::Truncated { what: W, at: base, needed: 8 * n, have: b.len() });
    }
    let mut tables = Vec::new();
    let mut malformed = 0usize;
    for i in 0..n {
        let c = u32_at(b, base + 8 * i, W)? as usize;
        let p = u32_at(b, base + 4 + 8 * i, W)?;
        if p == 0xffff_ffff {
            continue;
        }
        for k in 0..c {
            let o = base + p as usize + 16 * k;
            let Ok((tag, value, num_tags, pp)) = (|| -> Result<_, SpeechError> { Ok((u32_at(b, o, W)?, u32_at(b, o + 4, W)?, u32_at(b, o + 8, W)?, u32_at(b, o + 12, W)?)) })() else {
                malformed += 1;
                continue;
            };
            let q = base + pp as usize;
            let Ok(nr) = u32_at(b, q, W) else {
                malformed += 1;
                continue;
            };
            if q + 4 + 4 * nr as usize > b.len() {
                malformed += 1;
                continue;
            }
            let sample_ids: Vec<u32> = (0..nr as usize).map(|j| u32_at(b, q + 4 + 4 * j, W)).collect::<Result<_, _>>()?;
            let name = if named { cstr_at(b, q + 4 + 4 * nr as usize, W).ok() } else { None };
            tables.push(GraffitiTable { tag, value, num_tags, sample_ids, name });
        }
    }
    let mut index = HashMap::with_capacity(tables.len());
    for (i, t) in tables.iter().enumerate() {
        index.entry((t.tag, t.value, t.num_tags)).or_insert(i);
    }
    Ok(Graffiti { version, named, tables, malformed, index })
}

// ------------------------------------------------------------------ SBKR bank

/// One sample of a gen7 `SBKR` bank: its id and its offset in the `.sbs` stream.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BankSample {
    pub id: u32,
    pub sbs_offset: u32,
}

#[derive(Debug, Clone)]
pub struct Bank {
    pub bank_id: u32,
    pub samples: Vec<BankSample>,
    /// All-zero id-table slots (id 0, record count 0, pointer 0) — measured only at the end of the
    /// Xbox 360 NHL 14 `ann_eng` bank (8 slots, receipt 31); skipped, never followed.
    pub empty_slots: usize,
    index: HashMap<u32, usize>,
}

impl Bank {
    pub fn sample(&self, id: u32) -> Option<&BankSample> {
        self.index.get(&id).map(|i| &self.samples[*i])
    }
    pub fn max_offset(&self) -> u32 {
        self.samples.iter().map(|s| s.sbs_offset).max().unwrap_or(0)
    }
}

/// Parse a gen7 `SBKR` bank (`pbp_eng_samples.sbr`): `SBKR | u32 0x110 | 12 × 0 | 0 | u32 bank id |
/// 0 | u32 count | u32 0x30 | u32 id-table offset | u32 SNS1 tag offset …`; id table `count × 10
/// bytes: u32 sample id | u16 count | u32 ptr → 6-byte record: u16 0 | u32 ptr → u32 .sbs offset`.
pub fn parse_bank(b: &[u8]) -> Result<Bank, SpeechError> {
    const W: &str = "SBKR";
    if b.len() < 0x30 || &b[..4] != b"SBKR" {
        return Err(SpeechError::BadMagic { what: W, magic: String::from_utf8_lossy(b.get(..4).unwrap_or(b)).into_owned() });
    }
    let bank_id = u32_at(b, 0x14, W)?;
    let count = u32_at(b, 0x1c, W)? as usize;
    let table = u32_at(b, 0x24, W)? as usize;
    if table + 10 * count > b.len() {
        return Err(SpeechError::Truncated { what: W, at: table, needed: 10 * count, have: b.len() });
    }
    let mut samples = Vec::with_capacity(count);
    let mut index = HashMap::with_capacity(count);
    let mut empty_slots = 0;
    for i in 0..count {
        let o = table + 10 * i;
        let id = u32_at(b, o, W)?;
        let n = u16::from_be_bytes([b[o + 4], b[o + 5]]);
        let p = u32_at(b, o + 6, W)? as usize;
        if n == 0 {
            // An empty slot must be empty throughout; a partly zero entry is refused, not guessed.
            if id != 0 || p != 0 {
                return Err(SpeechError::Truncated { what: W, at: o, needed: 10, have: b.len() });
            }
            empty_slots += 1;
            continue;
        }
        let q = u32_at(b, p + 2, W)? as usize;
        let sbs_offset = u32_at(b, q, W)?;
        index.entry(id).or_insert(samples.len());
        samples.push(BankSample { id, sbs_offset });
    }
    Ok(Bank { bank_id, samples, empty_slots, index })
}

// ------------------------------------------------------------------ KWD0 keywords

/// The keyword (mention) index `pbp_eng_keywords.bin`: keyword id → sample ids. The team / arena /
/// city / chant `audioid` columns of the database are keyword ids of this table (research
/// 2026-09-20 handoff 07 §3); it is **not** the phrase-resolution path (handoff 04 2026-09-21).
#[derive(Debug, Clone)]
pub struct Keywords {
    pub pairs: usize,
    pub by_keyword: BTreeMap<u32, Vec<u32>>,
}

/// `0DWK` (LE, count @12, pairs @16) or big-endian 8-byte magic (count @16, pairs @20); pair =
/// `(sample id, keyword id)`.
pub fn parse_keywords(b: &[u8]) -> Result<Keywords, SpeechError> {
    const W: &str = "KWD0";
    if b.len() < 20 {
        return Err(SpeechError::Truncated { what: W, at: 0, needed: 20, have: b.len() });
    }
    let (n, base, le) = if &b[..4] == b"0DWK" {
        (u32::from_le_bytes([b[12], b[13], b[14], b[15]]) as usize, 16usize, true)
    } else if &b[..4] == b"KWD0" {
        (u32_at(b, 16, W)? as usize, 20usize, false)
    } else {
        return Err(SpeechError::BadMagic { what: W, magic: String::from_utf8_lossy(&b[..4]).into_owned() });
    };
    if base + 8 * n > b.len() {
        return Err(SpeechError::Truncated { what: W, at: base, needed: 8 * n, have: b.len() });
    }
    let mut by_keyword: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for i in 0..n {
        let o = base + 8 * i;
        let (sid, k) = if le {
            (u32::from_le_bytes([b[o], b[o + 1], b[o + 2], b[o + 3]]), u32::from_le_bytes([b[o + 4], b[o + 5], b[o + 6], b[o + 7]]))
        } else {
            (u32_at(b, o, W)?, u32_at(b, o + 4, W)?)
        };
        by_keyword.entry(k).or_default().push(sid);
    }
    Ok(Keywords { pairs: n, by_keyword })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fnv1_matches_the_research_rule() {
        // FNV-1 (multiply then xor) of the empty string is the offset basis.
        assert_eq!(fnv1_32(""), 0x811c_9dc5);
        assert_eq!(fnv1_32("a"), 0x811c_9dc5u32.wrapping_mul(0x0100_0193) ^ 0x61);
    }

    #[test]
    fn short_or_foreign_inputs_are_errors_not_panics() {
        assert!(parse_evt0(b"").is_err());
        assert!(parse_evt0(b"EVT0PS30\x00\x03\x00\x03").is_err());
        assert!(parse_moir(b"MOIR").is_err());
        assert!(parse_sentences(b"SPC0PS30\x00\x05\x00\x03\x00\x00\x00\x00\xff\xff\xff\xff").is_err());
        assert!(parse_graffiti(b"GRF0PS30").is_err());
        assert!(parse_bank(b"SBKR").is_err());
        assert!(parse_keywords(b"KWD0").is_err());
        assert!(matches!(parse_sentences(b"SPC0PS30\x00\x09\x00\x03\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00"), Err(SpeechError::UnsupportedVersion { .. })));
    }

    /// Receipt 31: all-zero id-table slots (the tail of the X360 NHL 14 `ann_eng` bank) are counted
    /// and skipped, never followed; a slot that is only partly zero is refused.
    #[test]
    fn sbkr_empty_slots_are_skipped_and_partial_ones_refused() {
        let build = |slots: &[(u32, u16, bool)]| {
            let mut b = vec![0u8; 0x30];
            b[..4].copy_from_slice(b"SBKR");
            b[0x14..0x18].copy_from_slice(&0x1992_5777u32.to_be_bytes());
            b[0x1c..0x20].copy_from_slice(&(slots.len() as u32).to_be_bytes());
            b[0x24..0x28].copy_from_slice(&0x30u32.to_be_bytes());
            let recs = 0x30 + 10 * slots.len();
            b.resize(recs, 0);
            for (i, (id, n, real)) in slots.iter().enumerate() {
                let o = 0x30 + 10 * i;
                let p = if *real { (recs + 10 * i) as u32 } else { 0 };
                b[o..o + 4].copy_from_slice(&id.to_be_bytes());
                b[o + 4..o + 6].copy_from_slice(&n.to_be_bytes());
                b[o + 6..o + 10].copy_from_slice(&p.to_be_bytes());
            }
            for (i, (_, _, real)) in slots.iter().enumerate() {
                // 6-byte record (u16 0 | u32 ptr) then the u32 .sbs offset it points at.
                let q = (recs + 10 * i + 6) as u32;
                b.extend_from_slice(&0u16.to_be_bytes());
                b.extend_from_slice(&(if *real { q } else { 0 }).to_be_bytes());
                b.extend_from_slice(&(16 + 100 * i as u32).to_be_bytes());
            }
            b
        };
        let bank = parse_bank(&build(&[(45, 1, true), (46, 1, true), (0, 0, false), (0, 0, false)])).unwrap();
        assert_eq!((bank.samples.len(), bank.empty_slots), (2, 2));
        assert_eq!(bank.sample(46).unwrap().sbs_offset, 116);
        assert!(parse_bank(&build(&[(45, 1, true), (7, 0, false)])).is_err(), "an id with no record is not an empty slot");
    }

    #[test]
    fn a_synthetic_v5_sentence_file_parses_and_hashes() {
        // One slot holding one sentence with one body and one phrase (fixed pair + one event ref).
        let name = b"GOAL_SCORED_GENERIC";
        let n = 1u32;
        let mut f = Vec::new();
        f.extend_from_slice(b"SPC0PS30");
        f.extend_from_slice(&5u16.to_be_bytes());
        f.extend_from_slice(&3u16.to_be_bytes());
        f.extend_from_slice(&0u32.to_be_bytes());
        f.extend_from_slice(&n.to_be_bytes());
        f.extend_from_slice(&0u32.to_be_bytes());
        let slot_tbl = f.len();
        f.extend_from_slice(&[0u8; 8]);
        let slot_ptr = f.len();
        f.extend_from_slice(&[0u8; 8]);
        let sentence = f.len();
        f.extend_from_slice(&1u32.to_be_bytes()); // c
        f.extend_from_slice(&0u32.to_be_bytes()); // w0
        let body_ptr_pos = f.len();
        f.extend_from_slice(&[0u8; 4]);
        f.extend_from_slice(name);
        f.push(0);
        while f.len() % 4 != 0 {
            f.push(0);
        }
        let body = f.len();
        f.extend_from_slice(&7u32.to_be_bytes());
        f.extend_from_slice(&100u32.to_be_bytes());
        f.extend_from_slice(&1u32.to_be_bytes());
        f.extend_from_slice(&1u32.to_be_bytes());
        let phrase_ptr_pos = f.len();
        f.extend_from_slice(&[0u8; 4]);
        let phrase = f.len();
        f.extend_from_slice(&[12, 0, 2, 0, 2, 0xff, 0xff, 0xff]);
        f.extend_from_slice(&287u32.to_be_bytes());
        f.extend_from_slice(&41u32.to_be_bytes());
        f.extend_from_slice(&[1, 0, 1, 0]);
        f.extend_from_slice(&5u32.to_be_bytes());
        f.extend_from_slice(&0xffff_ffffu32.to_be_bytes());
        f[slot_tbl..slot_tbl + 4].copy_from_slice(&1u32.to_be_bytes());
        f[slot_tbl + 4..slot_tbl + 8].copy_from_slice(&(slot_ptr as u32).to_be_bytes());
        f[slot_ptr + 4..slot_ptr + 8].copy_from_slice(&(sentence as u32).to_be_bytes());
        f[body_ptr_pos..body_ptr_pos + 4].copy_from_slice(&(body as u32).to_be_bytes());
        f[phrase_ptr_pos..phrase_ptr_pos + 4].copy_from_slice(&(phrase as u32).to_be_bytes());
        let s = parse_sentences(&f).unwrap();
        assert_eq!(s.version, (5, 3));
        assert_eq!(s.sentences.len(), 1);
        let st = &s.sentences[0];
        assert_eq!(st.name, "GOAL_SCORED_GENERIC");
        assert!(st.slot_ok, "one slot: every hash lands in it");
        assert_eq!(st.bodies[0].id, 7);
        assert_eq!(st.bodies[0].phrases[0], Phrase { pool_id: 12, num_parameters: 2, tags: vec![PhraseTag::Fixed { tag: 287, value: 41 }, PhraseTag::EventRef { tag: 5, event_index: 0 }] });
        // A broken event-ref head is refused, not misread.
        let mut g = f.clone();
        g[phrase + 16] = 2;
        assert!(matches!(parse_sentences(&g), Err(SpeechError::Malformed { .. })));
        // A truncated file is refused.
        assert!(parse_sentences(&f[..phrase + 10]).is_err());
    }
}
