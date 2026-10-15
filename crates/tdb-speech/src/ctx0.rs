//! `CTX0` v4.5 context records (PS3 / Xbox 360 NHL 13–Legacy) — research 2026-09-21 handoff 01
//! (`research-r24-ctx0-structure`): the file tables, and per record the trigger name (pool 2),
//! the context name, the items (speech event / context trigger, typed by namespace, with their
//! parameter values), the conditions (pool-1 names, arguments, blob index) and the extra words.
//! Every check the research states is enforced as a refusal; nothing about bytecode semantics,
//! the undecoded header words or runtime selection is claimed. NHL 11 / 12 (`CTX0` version 0.0
//! / 3.1) are refused with the version (handoff 16 is a separate parser, not built here).

use std::collections::HashMap;

use crate::SpeechError;

const FF: u32 = 0xFFFF_FFFF;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtxParam {
    /// 1 = immediate (`value`), 2 = a call block (`blob`, `args`).
    pub kind: u32,
    pub value: Option<u32>,
    pub blob: Option<u32>,
    pub args: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtxItem {
    pub slot: u32,
    pub namespace: u32,
    pub event_id: u16,
    /// `speech` (the speech module's namespace), `context` (the context module's — a trigger
    /// raised back to the engine), or `unknown-namespace`.
    pub kind: &'static str,
    /// The event name spelled inside the record.
    pub name: String,
    pub params: Vec<CtxParam>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtxCondition {
    pub name: String,
    pub blob: u32,
    pub args: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CtxRecord {
    pub index: u32,
    pub context_id: u32,
    pub name: String,
    pub trigger: Option<String>,
    /// Header words 1, 3, 4 (not decoded; recorded raw).
    pub w1: u32,
    pub w3: u32,
    pub w4: u32,
    pub items: Vec<CtxItem>,
    pub conditions: Vec<CtxCondition>,
    pub extra: Vec<u32>,
    /// Legacy only: the slot-2 four-word block (not an item), when present.
    pub slot2_block: Option<[u32; 4]>,
}

#[derive(Debug, Clone)]
pub struct Ctx0 {
    pub platform: &'static str,
    pub records: Vec<CtxRecord>,
    pub trigger_names: Vec<String>,
    pub condition_names: usize,
    /// Pool-1 strings no condition references (0 on every measured set per the research JSON census).
    pub unreferenced_condition_names: usize,
}

fn pool_strings(b: &[u8], lo: usize, hi: usize, what: &'static str) -> Result<HashMap<u32, String>, SpeechError> {
    let mut d = HashMap::new();
    let mut o = lo;
    while o < hi {
        let e = b[o..hi].iter().position(|x| *x == 0).map(|p| o + p).ok_or(SpeechError::Malformed { what, reason: format!("pool [{lo:#x},{hi:#x}) lacks a terminating NUL after {o:#x}") })?;
        d.insert((o - lo) as u32, String::from_utf8_lossy(&b[o..e]).into_owned());
        o = e + 1;
    }
    Ok(d)
}

/// Parse `pbp_eng_contexts.bin` (or a crowd `*_contexts.bin`). `speech_ns` / `ctx_ns` are the
/// EVT0 namespaces of the speech and context modules used to type the items.
pub fn parse_ctx0(b: &[u8], speech_ns: Option<u16>, ctx_ns: Option<u16>) -> Result<Ctx0, SpeechError> {
    const W: &str = "CTX0";
    let bad = |d: String| SpeechError::Malformed { what: W, reason: d };
    if b.len() < 0x30 || &b[..4] != b"CTX0" {
        return Err(SpeechError::BadMagic { what: W, magic: String::from_utf8_lossy(b.get(..8).unwrap_or(b)).into_owned() });
    }
    let u = |o: usize| -> u32 { u32::from_be_bytes(b[o..o + 4].try_into().unwrap()) };
    let platform = match &b[4..8] {
        b"X360" => "xbox",
        b"PS30" => "ps3",
        [0, 3, 0, 1] => return Err(bad("CTX0 version 3.1 (NHL 11 / 12, 4-byte magic) is not the v4.5 layout of NHL 13–Legacy; the v3.1 parser is not built".into())),
        other => return Err(bad(format!("unknown platform tag {:?}", String::from_utf8_lossy(other)))),
    };
    let version = (u16::from_be_bytes([b[8], b[9]]), u16::from_be_bytes([b[10], b[11]]));
    if version != (4, 5) {
        return Err(bad(format!("CTX0 version {}.{} is not the v4.5 layout of NHL 13–Legacy (NHL 11 / 12 carry another version)", version.0, version.1)));
    }
    let n = u(0x10) as usize;
    let a = u(0x14) as usize;
    let n2 = u(0x18) as usize;
    let pool1 = u(0x28) as usize;
    let pool2 = u(0x2C) as usize;
    if n != n2 || 0x30 + 8 * a + 16 * n > pool1 || pool1 > pool2 || pool2 > b.len() || b[0x1C..0x28] != [0u8; 12] {
        return Err(bad(format!("inconsistent header: n {n} n2 {n2} a {a} pool1 {pool1:#x} pool2 {pool2:#x} len {:#x}", b.len())));
    }
    for i in 0..a {
        if u(0x30 + 8 * i) != i as u32 || u(0x34 + 8 * i) != 0 {
            return Err(bad(format!("A-table row {i} is not ({i}, 0)")));
        }
    }
    let t1 = 0x30 + 8 * a;
    let t2 = t1 + 4 * n;
    let t3 = t2 + 8 * n;
    let t3end = t3 + 8 * n;
    let ptrs: Vec<u32> = (0..n).map(|i| u(t1 + 4 * i)).collect();
    for i in 0..n {
        if u(t2 + 8 * i) != 1 || u(t2 + 4 + 8 * i) as usize != t3 + 8 * i {
            return Err(bad(format!("T2 row {i} is not (1, T3 row)")));
        }
    }
    let mut t3_by_ptr: HashMap<u32, u32> = HashMap::new();
    let mut prev_id: Option<u32> = None;
    for i in 0..n {
        let (id, p) = (u(t3 + 8 * i), u(t3 + 4 + 8 * i));
        if prev_id.is_some_and(|q| q >= id) {
            return Err(bad("T3 ids are not strictly sorted".into()));
        }
        prev_id = Some(id);
        t3_by_ptr.insert(p, id);
    }
    let mut sorted = ptrs.clone();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != n || sorted.iter().any(|p| (*p as usize) < t3end || *p as usize >= pool1) || sorted[0] as usize != t3end {
        return Err(bad("record pointers are not distinct inside [T3 end, pool 1) starting at T3 end".into()));
    }
    if t3_by_ptr.len() != n || sorted.iter().any(|p| !t3_by_ptr.contains_key(p)) {
        return Err(bad("T3 pointer set differs from T1".into()));
    }
    let p1 = pool_strings(b, pool1, pool2, W)?;
    let p2 = pool_strings(b, pool2, b.len(), W)?;
    let ends: HashMap<u32, usize> = sorted.iter().enumerate().map(|(i, p)| (*p, sorted.get(i + 1).map(|q| *q as usize).unwrap_or(pool1))).collect();
    let zeros = if platform == "ps3" { 4usize } else { 2 };
    let fixed = 4 * (zeros + 5);
    let mut referenced_cond: std::collections::HashSet<u32> = Default::default();
    let mut records = Vec::with_capacity(n);
    for (idx, &r) in ptrs.iter().enumerate() {
        let r = r as usize;
        let e = ends[&(r as u32)];
        if e - r < 60 {
            return Err(bad(format!("record {idx}: shorter than the 60-byte header")));
        }
        let inrec = |p: u32, need: usize| -> bool { (p as usize) >= r + 60 && (p as usize) + need <= e };
        let cstr_in = |p: u32| -> Result<String, SpeechError> {
            if !inrec(p, 1) {
                return Err(bad(format!("record {idx}: string pointer {p:#x} outside the record")));
            }
            let s = p as usize;
            let z = b[s..e].iter().position(|x| *x == 0).ok_or_else(|| bad(format!("record {idx}: string at {p:#x} not terminated inside the record")))?;
            Ok(String::from_utf8_lossy(&b[s..s + z]).into_owned())
        };
        let h: Vec<u32> = (0..15).map(|k| u(r + 4 * k)).collect();
        // The context id equals the T3 id of the record's pointer (the research prose also says
        // "== the record index", but its JSON census counts 0–1 such records per set: the T3
        // identity is the verified rule and the one enforced).
        if t3_by_ptr.get(&(r as u32)) != Some(&h[0]) {
            return Err(bad(format!("record {idx}: context id {} is not the T3 id of its pointer", h[0])));
        }
        let ncond = (h[2] >> 24) as usize;
        let nextra = ((h[2] >> 16) & 0xFF) as usize;
        if h[2] & 0xFF != 100 || h[5] != 0 || h[6] != 0 || h[7] != FF {
            return Err(bad(format!("record {idx}: header words 2 / 5 / 6 / 7 are not the measured constants")));
        }
        let trigger = if h[8] == FF { None } else { Some(p2.get(&h[8]).cloned().ok_or_else(|| bad(format!("record {idx}: w8 {} is not a pool-2 string start", h[8])))?) };
        let name = cstr_in(h[14])?;
        let mut items = Vec::new();
        let mut slot2_block = None;
        for slot in 0..3u32 {
            let s = h[9 + slot as usize];
            if s == FF {
                continue;
            }
            if !inrec(s, 16) {
                return Err(bad(format!("record {idx}: slot {slot} pointer {s:#x} outside the record")));
            }
            let ns = (u(s as usize + 4) >> 16) as u16;
            if slot == 2 && Some(ns) != speech_ns && Some(ns) != ctx_ns {
                let blk = [u(s as usize), u(s as usize + 4), u(s as usize + 8), u(s as usize + 12)];
                if blk != [1, 3, 0, blk[3]] && blk != [0, 7, 6, 4] {
                    return Err(bad(format!("record {idx}: slot-2 block {blk:?} is not a measured shape")));
                }
                slot2_block = Some(blk);
                continue;
            }
            if !inrec(s, fixed) {
                return Err(bad(format!("record {idx}: item slot {slot} pointer {s:#x} outside the record")));
            }
            let w: Vec<u32> = (0..zeros + 5).map(|k| u(s as usize + 4 * k)).collect();
            if w[0] != 0 || w[2..2 + zeros].iter().any(|x| *x != 0) || w[3 + zeros] != 0 {
                return Err(bad(format!("record {idx}: item fixed words are not zero")));
            }
            let nparam = w[2 + zeros] as usize;
            if nparam > 64 {
                return Err(bad(format!("record {idx}: item nparam {nparam} implausible")));
            }
            let iname = cstr_in(w[4 + zeros])?;
            let mut params = Vec::with_capacity(nparam);
            for k in 0..nparam {
                let kp = s as usize + fixed + 8 * k;
                if !inrec(kp as u32, 8) {
                    return Err(bad(format!("record {idx}: item parameter table outside the record")));
                }
                let (kind, vp) = (u(kp), u(kp + 4));
                match kind {
                    1 => {
                        if !inrec(vp, 4) {
                            return Err(bad(format!("record {idx}: parameter value pointer outside the record")));
                        }
                        params.push(CtxParam { kind: 1, value: Some(u(vp as usize)), blob: None, args: Vec::new() });
                    }
                    2 => {
                        if !inrec(vp, 16) {
                            return Err(bad(format!("record {idx}: kind-2 parameter block outside the record")));
                        }
                        let v: Vec<u32> = (0..4).map(|j| u(vp as usize + 4 * j)).collect();
                        if v[0] != 0 || v[2] > 16 || (v[2] == 0) != (v[3] == FF) || (v[2] > 0 && v[3] != 0) {
                            return Err(bad(format!("record {idx}: kind-2 parameter block shape {v:?}")));
                        }
                        let mut args = Vec::with_capacity(v[2] as usize);
                        for j in 0..v[2] as usize {
                            let ap = vp as usize + 16 + 8 * j;
                            if !inrec(ap as u32, 8) {
                                return Err(bad(format!("record {idx}: kind-2 argument table outside the record")));
                            }
                            let (ak, aptr) = (u(ap), u(ap + 4));
                            if ak != 1 || !inrec(aptr, 4) {
                                return Err(bad(format!("record {idx}: kind-2 argument kind {ak} / pointer not supported")));
                            }
                            args.push(u(aptr as usize));
                        }
                        params.push(CtxParam { kind: 2, value: None, blob: Some(v[1]), args });
                    }
                    other => return Err(bad(format!("record {idx}: parameter kind {other} not measured"))),
                }
            }
            let kind = if speech_ns == Some(ns) { "speech" } else if ctx_ns == Some(ns) { "context" } else { "unknown-namespace" };
            items.push(CtxItem { slot, namespace: ns as u32, event_id: (w[1] & 0xFFFF) as u16, kind, name: iname, params });
        }
        if items.is_empty() {
            return Err(bad(format!("record {idx}: no item")));
        }
        let mut conditions = Vec::with_capacity(ncond);
        let cl = h[12];
        if cl == FF {
            if ncond != 0 {
                return Err(bad(format!("record {idx}: ncond {ncond} but no condition list")));
            }
        } else {
            if !inrec(cl, 4) {
                return Err(bad(format!("record {idx}: condition list pointer outside the record")));
            }
            let first = u(cl as usize);
            if first < cl || (first - cl) % 4 != 0 || ((first - cl) / 4) as usize != ncond {
                return Err(bad(format!("record {idx}: condition list length != ncond {ncond}")));
            }
            for k in 0..ncond {
                let cp = u(cl as usize + 4 * k);
                if !inrec(cp, 16) {
                    return Err(bad(format!("record {idx}: condition pointer outside the record")));
                }
                let w: Vec<u32> = (0..4).map(|j| u(cp as usize + 4 * j)).collect();
                let cname = p1.get(&w[3]).cloned().ok_or_else(|| bad(format!("record {idx}: condition name offset {} is not a pool-1 string start", w[3])))?;
                referenced_cond.insert(w[3]);
                let nargs = w[0] as usize;
                if nargs > 16 || (nargs == 0) != (w[1] == FF) {
                    return Err(bad(format!("record {idx}: condition argument shape nargs {nargs} ptr {:#x}", w[1])));
                }
                let mut args = Vec::with_capacity(nargs);
                if nargs > 0 {
                    if !inrec(w[1], 4 * nargs) {
                        return Err(bad(format!("record {idx}: condition argument list outside the record")));
                    }
                    for j in 0..nargs {
                        args.push(u(w[1] as usize + 4 * j));
                    }
                }
                conditions.push(CtxCondition { name: cname, blob: w[2], args });
            }
        }
        let mut extra = Vec::with_capacity(nextra);
        let xl = h[13];
        if xl == FF {
            if nextra != 0 {
                return Err(bad(format!("record {idx}: nextra {nextra} but no extra list")));
            }
        } else {
            if !inrec(xl, 4 * nextra.max(1)) {
                return Err(bad(format!("record {idx}: extra word list outside the record")));
            }
            for j in 0..nextra {
                extra.push(u(xl as usize + 4 * j));
            }
        }
        records.push(CtxRecord { index: idx as u32, context_id: h[0], name, trigger, w1: h[1], w3: h[3], w4: h[4], items, conditions, extra, slot2_block });
    }
    let mut trigger_names: Vec<String> = p2.values().cloned().collect();
    trigger_names.sort();
    Ok(Ctx0 { platform, records, trigger_names, condition_names: p1.len(), unreferenced_condition_names: p1.keys().filter(|k| !referenced_cond.contains(k)).count() })
}

// ------------------------------------------------------------------ v3.1 (NHL 11 / 12)

/// Whether the bytes are a `CTX0` v3.1 file (4-byte magic, `u16 3, u16 1`).
pub fn is_ctx0_v31(b: &[u8]) -> bool {
    b.len() >= 8 && &b[..4] == b"CTX0" && b[4..8] == [0, 3, 0, 1]
}

/// Parse a `CTX0` v3.1 file (NHL 11 / 12; research 2026-09-21 handoff 16 §1): the same
/// trigger → conditions → item model as v4.5 with inline names and one item per record.
/// Items are typed by the MOIR tables: `speech` when the event id carries the spelled name in
/// the speech module's table, `context` when in the context module's, else `other-module`
/// (injury stories and the like). Parameters: a trigger slot (`kind` 0, `value` = slot index),
/// an immediate (`kind` 1), or an expression (`kind` 2, `blob` = script id, `args` = the slot /
/// immediate values). The script id → `.gas` blob binding is open on v3.1 (not claimed).
pub fn parse_ctx0_v31(b: &[u8], speech: Option<&crate::Moir>, ctx: Option<&crate::Moir>) -> Result<Ctx0, SpeechError> {
    const W: &str = "CTX0";
    let bad = |d: String| SpeechError::Malformed { what: W, reason: d };
    if !is_ctx0_v31(b) {
        return Err(bad("not a CTX0 v3.1 file".into()));
    }
    let rd = |o: usize| -> Result<u32, SpeechError> { b.get(o..o + 4).map(|s| u32::from_be_bytes([s[0], s[1], s[2], s[3]])).ok_or(SpeechError::Truncated { what: W, at: o, needed: 4, have: b.len() }) };
    let n = rd(12)? as usize;
    let a = rd(16)? as usize;
    if rd(20)? as usize != n || rd(8)? != 0 {
        return Err(bad(format!("header counts disagree (n {n})")));
    }
    if n > 100_000 || a > 10_000 {
        return Err(bad(format!("implausible counts n {n} a {a}")));
    }
    let tbl = 36 + 16 * a;
    for k in 0..a {
        if rd(36 + 16 * k + 4)? != 0 || rd(36 + 16 * k + 8)? != 0 || rd(36 + 16 * k + 12)? != 0 {
            return Err(bad(format!("group-table entry {k} is not (hash, 0, 0, 0)")));
        }
    }
    let ptrs: Vec<u32> = (0..n).map(|i| rd(tbl + 4 * i)).collect::<Result<_, _>>()?;
    let cstr = |o: u32| -> Result<String, SpeechError> {
        let o = o as usize;
        if o >= b.len() {
            return Err(bad(format!("string pointer {o:#x} outside the file")));
        }
        let z = b[o..].iter().position(|x| *x == 0).ok_or_else(|| bad(format!("string at {o:#x} not terminated")))?;
        Ok(String::from_utf8_lossy(&b[o..o + z]).into_owned())
    };
    let named_in = |m: Option<&crate::Moir>, id: u16, name: &str| m.is_some_and(|m| m.names(id).iter().any(|n| *n == name));
    let mut records = Vec::with_capacity(n);
    let mut triggers: std::collections::BTreeSet<String> = Default::default();
    let mut cond_names: std::collections::BTreeSet<String> = Default::default();
    for (idx, &r) in ptrs.iter().enumerate() {
        let r = r as usize;
        let h: Vec<u32> = (0..19).map(|k| rd(r + 4 * k)).collect::<Result<_, _>>()?;
        // Header words 6–10 / 13 are 0 0 0 0 ff / ff on 2 446 of 2 833 NHL 11 records (the research's
        // `zeros_ok` count); the others carry w6 = 1 and a w13 pointer the research did not
        // decode — recorded raw (`w3` keeps its meaning; the extra pointer is not followed).
        let _ = &h[6..11];
        if h[18] == FF {
            return Err(bad(format!("record {idx}: no name")));
        }
        let name = cstr(h[18])?;
        // Trigger block: `0 | ns<<16|id | nargs | 0 | 0 | nargs × slot | name`.
        let mut slot_words: HashMap<u32, u32> = HashMap::new();
        let trigger = if h[11] == FF {
            None
        } else {
            let t = h[11] as usize;
            let nargs = rd(t + 8)? as usize;
            if rd(t)? != 0 || rd(t + 12)? != 0 || rd(t + 16)? != 0 || nargs > 32 {
                return Err(bad(format!("record {idx}: trigger block head")));
            }
            for k in 0..nargs {
                let v = rd(t + 20 + 4 * k)?;
                if v as usize != k + 1 {
                    return Err(bad(format!("record {idx}: trigger slot {k} is {v}, not {}", k + 1)));
                }
                slot_words.insert((t + 20 + 4 * k) as u32, v);
            }
            let tname = cstr((t + 20 + 4 * nargs) as u32)?;
            triggers.insert(tname.clone());
            Some(tname)
        };
        // Item block: X360 `0 | ref | 0 | 0 | nparams | 0 | name ptr | params`; PS3 two more zero words.
        let mut items = Vec::new();
        if h[12] != FF {
            let it = h[12] as usize;
            let ps3 = rd(it + 16)? == 0 && rd(it + 20)? == 0 && rd(it + 24)? < 0x100;
            let base = if ps3 { it + 24 } else { it + 16 };
            let nparam = rd(base)? as usize;
            if rd(it)? != 0 || rd(it + 8)? != 0 || rd(it + 12)? != 0 || rd(base + 4)? != 0 || nparam > 64 {
                return Err(bad(format!("record {idx}: item block head")));
            }
            let reference = rd(it + 4)?;
            let iname = cstr(rd(base + 8)?)?;
            let param_at = |kind: u32, p: u32| -> Result<CtxParam, SpeechError> {
                match kind {
                    0 => Ok(match slot_words.get(&p) {
                        Some(slot) => CtxParam { kind: 0, value: Some(*slot), blob: None, args: Vec::new() },
                        None => CtxParam { kind: 1, value: Some(rd(p as usize)?), blob: None, args: Vec::new() },
                    }),
                    1 => {
                        let p = p as usize;
                        if rd(p)? != 0 {
                            return Err(bad(format!("record {idx}: expression head")));
                        }
                        let blob = rd(p + 4)?;
                        let na = rd(p + 8)? as usize;
                        if na > 16 {
                            return Err(bad(format!("record {idx}: expression with {na} arguments")));
                        }
                        let mut args = Vec::with_capacity(na);
                        for m in 0..na {
                            let (k, q) = (rd(p + 12 + 8 * m)?, rd(p + 16 + 8 * m)?);
                            if k != 0 {
                                return Err(bad(format!("record {idx}: nested expression argument kind {k}")));
                            }
                            args.push(slot_words.get(&q).copied().unwrap_or(rd(q as usize)?));
                        }
                        Ok(CtxParam { kind: 2, value: None, blob: Some(blob), args })
                    }
                    other => Err(bad(format!("record {idx}: parameter kind {other} not measured"))),
                }
            };
            let mut params = Vec::with_capacity(nparam);
            for m in 0..nparam {
                params.push(param_at(rd(base + 12 + 8 * m)?, rd(base + 16 + 8 * m)?)?);
            }
            let id = (reference & 0xFFFF) as u16;
            let kind = if named_in(speech, id, &iname) {
                "speech"
            } else if named_in(ctx, id, &iname) {
                "context"
            } else {
                "other-module"
            };
            items.push(CtxItem { slot: 0, namespace: reference >> 16, event_id: id, kind, name: iname, params });
        }
        // Conditions: `nargs | ptr → nargs × ptr (to slot words) or ff | script id | 0 | [arg ptrs] | name`.
        let ncond = h[14] as usize;
        if ncond > 64 {
            return Err(bad(format!("record {idx}: {ncond} conditions")));
        }
        let mut conditions = Vec::with_capacity(ncond);
        for k in 0..ncond {
            let c = rd(h[15] as usize + 4 * k)? as usize;
            let na = rd(c)? as usize;
            let ap = rd(c + 4)?;
            let blob = rd(c + 8)?;
            if rd(c + 12)? != 0 || na > 16 || (na == 0) != (ap == FF) {
                return Err(bad(format!("record {idx}: condition {k} head")));
            }
            let mut args = Vec::with_capacity(na);
            for m in 0..na {
                let q = rd(ap as usize + 4 * m)?;
                args.push(slot_words.get(&q).copied().ok_or_else(|| bad(format!("record {idx}: condition argument is not a trigger slot")))?);
            }
            let name_at = if na == 0 { c + 16 } else { ap as usize + 4 * na };
            let cname = cstr(name_at as u32)?;
            cond_names.insert(cname.clone());
            conditions.push(CtxCondition { name: cname, blob, args });
        }
        let nextra = h[16] as usize;
        let mut extra = Vec::with_capacity(nextra);
        if nextra > 0 && h[17] != FF {
            for k in 0..nextra.min(64) {
                extra.push(rd(h[17] as usize + 4 * k)?);
            }
        }
        records.push(CtxRecord { index: idx as u32, context_id: h[0], name, trigger, w1: h[1], w3: h[3], w4: h[4], items, conditions, extra, slot2_block: None });
    }
    Ok(Ctx0 { platform: "gen7-v31", records, trigger_names: triggers.into_iter().collect(), condition_names: cond_names.len(), unreferenced_condition_names: 0 })
}
