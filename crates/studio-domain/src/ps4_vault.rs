//! PS4 NHL 15–21 **Attrib/VLT vault** reader (receipt 51; research R7 handoff 07 of 19 Sep 2026 — the format record —
//! and R135 / R135a of 28 Sep 2026). Read-only.
//!
//! The registry is the pair `gccollections.vlt` (records) + `gccollections.bin` (strings + data heap), described by
//! `gcschema.vlt` + `gcschema.bin`, shipped only inside `databigs/boot.big` under `nhl/data/common/database/attrib/`.
//!
//! ```text
//! .vlt chunk chain : tag(4, FourCC reversed) | size(4, incl. header) | payload — Vers, StrN, DepN, DatN, ExpN, PtrN, EndC
//! ExpN             : count u32, then 16-byte rows  key | tag | size | absolute .vlt offset
//! PtrN             : 12-byte fixups  slot | type u16 | flags u16 | target (a .bin offset); markers (0,2,1,·) → slots in
//!                    the .bin, (0,2,0,·) → slots in the .vlt, (0,0,0,·) ends a segment; every slot is zero in the file
//! .bin             : StrE (NUL-separated strings) | TofC | EndC | heap (instance blobs, RefSpecs, vectors)
//! record (.vlt)    : collectionKey | classKey | 0 | nInline | 1 | nInline | nTypes u16 | u16 | 0 | ptr(8) → blob |
//!                    nTypes type keys | zero pad to 8 | nInline × 24: attrKey | 0 | value / ptr (8) | typeIndex u16 | flags | 0
//! schema class     : 48-byte record: classKey | · | attrCount @8 | · | ptr @16 → attrCount × 16-byte descriptors
//!                    attrKey | typeKey | offset u16 | size u16 | count u16 | flags u8 | code u8
//! keys             : Bob Jenkins lookup2 (1996), initval 0xABCDEF00, over the name bytes
//! ```
//!
//! Every offset read is bounds-checked; a malformed vault is refused as a whole with the reason, never partially trusted.
use std::collections::{BTreeMap, HashMap};
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::{DomainError, Result};

pub const ATTRIB_DIR: &str = "nhl/data/common/database/attrib/";
pub const FILES: [&str; 4] = ["gccollections.vlt", "gccollections.bin", "gcschema.vlt", "gcschema.bin"];
const KEY_INIT: u32 = 0xABCD_EF00;

fn bad(what: impl Into<String>) -> DomainError {
    DomainError::Validation(format!("PS4 vault: {}", what.into()))
}

/// Jenkins lookup2 with initval `init` — the Attrib/VLT string key.
pub fn lookup2(data: &[u8], init: u32) -> u32 {
    fn mix(mut a: u32, mut b: u32, mut c: u32) -> (u32, u32, u32) {
        a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 13);
        b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 8);
        c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 13);
        a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 12);
        b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 16);
        c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 5);
        a = a.wrapping_sub(b).wrapping_sub(c) ^ (c >> 3);
        b = b.wrapping_sub(c).wrapping_sub(a) ^ (a << 10);
        c = c.wrapping_sub(a).wrapping_sub(b) ^ (b >> 15);
        (a, b, c)
    }
    let (mut a, mut b, mut c) = (0x9e37_79b9u32, 0x9e37_79b9u32, init);
    let mut k = data;
    while k.len() >= 12 {
        a = a.wrapping_add(u32::from_le_bytes(k[0..4].try_into().unwrap()));
        b = b.wrapping_add(u32::from_le_bytes(k[4..8].try_into().unwrap()));
        c = c.wrapping_add(u32::from_le_bytes(k[8..12].try_into().unwrap()));
        (a, b, c) = mix(a, b, c);
        k = &k[12..];
    }
    c = c.wrapping_add(data.len() as u32);
    let n = k.len();
    let byte = |i: usize| k[i] as u32;
    if n >= 11 { c = c.wrapping_add(byte(10) << 24); }
    if n >= 10 { c = c.wrapping_add(byte(9) << 16); }
    if n >= 9 { c = c.wrapping_add(byte(8) << 8); }
    if n >= 8 { b = b.wrapping_add(byte(7) << 24); }
    if n >= 7 { b = b.wrapping_add(byte(6) << 16); }
    if n >= 6 { b = b.wrapping_add(byte(5) << 8); }
    if n >= 5 { b = b.wrapping_add(byte(4)); }
    if n >= 4 { a = a.wrapping_add(byte(3) << 24); }
    if n >= 3 { a = a.wrapping_add(byte(2) << 16); }
    if n >= 2 { a = a.wrapping_add(byte(1) << 8); }
    if n >= 1 { a = a.wrapping_add(byte(0)); }
    mix(a, b, c).2
}

/// The Attrib key of a name.
pub fn key(name: &str) -> u32 {
    lookup2(name.as_bytes(), KEY_INIT)
}

fn u16le(b: &[u8], o: usize) -> Result<u16> {
    b.get(o..o + 2).map(|s| u16::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| bad(format!("read past the end at {o:#x}")))
}
fn u32le(b: &[u8], o: usize) -> Result<u32> {
    b.get(o..o + 4).map(|s| u32::from_le_bytes(s.try_into().unwrap())).ok_or_else(|| bad(format!("read past the end at {o:#x}")))
}
fn i32le(b: &[u8], o: usize) -> Result<i32> {
    u32le(b, o).map(|v| v as i32)
}

/// `tag → (offset, size)` of a chunk chain.
fn chunks(x: &[u8]) -> Result<BTreeMap<[u8; 4], (usize, usize)>> {
    let mut out = BTreeMap::new();
    let mut pos = 0usize;
    while pos + 8 <= x.len() {
        let mut tag: [u8; 4] = x[pos..pos + 4].try_into().unwrap();
        tag.reverse();
        let size = u32le(x, pos + 4)? as usize;
        if size < 8 || pos + size > x.len() {
            break; // the unframed heap after EndC
        }
        out.entry(tag).or_insert((pos, size));
        pos += size;
        if &tag == b"EndC" {
            break;
        }
    }
    Ok(out)
}

fn need<'a>(c: &'a BTreeMap<[u8; 4], (usize, usize)>, tag: &[u8; 4], file: &str) -> Result<&'a (usize, usize)> {
    c.get(tag).ok_or_else(|| bad(format!("{file} has no {} chunk", String::from_utf8_lossy(tag))))
}

/// `(slot, target)` fixups of a PtrN chunk, split into the `.bin`-slot and `.vlt`-slot segments.
fn fixups(v: &[u8], at: (usize, usize)) -> Result<(HashMap<usize, usize>, HashMap<usize, usize>, HashMap<usize, usize>)> {
    let (pp, ps) = at;
    let n = (ps - 8) / 12;
    let (mut bin, mut vlt, mut type3) = (HashMap::new(), HashMap::new(), HashMap::new());
    let mut seg: Option<bool> = None;
    for k in 0..n {
        let o = pp + 8 + k * 12;
        let (slot, ty, fl, tgt) = (u32le(v, o)? as usize, u16le(v, o + 4)?, u16le(v, o + 6)?, u32le(v, o + 8)? as usize);
        if ty == 3 {
            type3.insert(slot, tgt);
        }
        if ty == 2 && slot == 0 {
            seg = Some(fl == 1);
        } else if ty == 0 && slot == 0 {
            seg = None;
        } else {
            match seg {
                Some(true) => {
                    bin.insert(slot, tgt);
                }
                Some(false) => {
                    vlt.insert(slot, tgt);
                }
                None => {}
            }
        }
    }
    Ok((bin, vlt, type3))
}

/// NUL-separated strings of a chunk payload, keyed by their offset in the file.
fn strings(b: &[u8], start: usize, end: usize) -> HashMap<usize, String> {
    let mut out = HashMap::new();
    let mut i = start;
    while i < end {
        let Some(n) = b[i..end].iter().position(|c| *c == 0) else { break };
        if n > 0 {
            out.insert(i, String::from_utf8_lossy(&b[i..i + n]).into_owned());
        }
        i += n + 1;
    }
    out
}

#[derive(Debug, Clone)]
pub struct AttrDesc {
    pub key: u32,
    pub type_key: u32,
    pub offset: u16,
    pub size: u16,
    pub count: u16,
    pub flags: u8,
}

#[derive(Debug, Clone, Default)]
pub struct Schema {
    pub classes: HashMap<u32, Vec<AttrDesc>>,
    /// Type / class names by key (from the schema's own string pool).
    pub names: HashMap<u32, String>,
}

impl Schema {
    pub fn parse(sv: &[u8], sb: &[u8]) -> Result<Self> {
        let cv = chunks(sv)?;
        let cb = chunks(sb)?;
        let (ep, _) = *need(&cv, b"ExpN", "gcschema.vlt")?;
        let (_, _, type3) = fixups(sv, *need(&cv, b"PtrN", "gcschema.vlt")?)?;
        let mut names = HashMap::new();
        // gcschema.bin: the StrE / EndC chunks are empty headers and the type / class names follow them unframed —
        // printable NUL-terminated strings up to the first empty or non-printable one (R7's reading).
        let start = cb.get(b"EndC").map(|&(o, s)| o + s).unwrap_or(0);
        let mut pool = Vec::new();
        let mut i = start;
        while let Some(n) = sb.get(i..).and_then(|r| r.iter().position(|c| *c == 0)) {
            let w = &sb[i..i + n];
            if w.is_empty() || !w.iter().all(|c| (32..127).contains(c)) {
                break;
            }
            pool.push(String::from_utf8_lossy(w).into_owned());
            i += n + 1;
        }
        if let Some(&(sp, ss)) = cb.get(b"StrE") {
            pool.extend(strings(sb, sp + 8, sp + ss).into_values());
        }
        {
            for s in pool {
                names.insert(key(&s), s.clone());
                if let Some(short) = s.strip_prefix("Attrib::Gen::ClassRefSpec_") {
                    names.insert(key(short), short.to_string());
                }
            }
        }
        let count = u32le(sv, ep + 8)? as usize;
        let mut classes = HashMap::new();
        for k in 0..count {
            let r = ep + 12 + k * 16;
            let (ckey, size, off) = (u32le(sv, r)?, u32le(sv, r + 8)?, u32le(sv, r + 12)? as usize);
            if size != 48 {
                continue;
            }
            let Some(&tgt) = type3.get(&(off + 16)) else { continue };
            let n = u32le(sv, off + 8)? as usize;
            let mut attrs = Vec::with_capacity(n);
            for a in 0..n {
                let d = tgt + a * 16;
                if d + 16 > sb.len() {
                    return Err(bad(format!("class {ckey:08x}: descriptor {a} runs past gcschema.bin")));
                }
                attrs.push(AttrDesc { key: u32le(sb, d)?, type_key: u32le(sb, d + 4)?, offset: u16le(sb, d + 8)?, size: u16le(sb, d + 10)?, count: u16le(sb, d + 12)?, flags: sb[d + 14] });
            }
            classes.insert(ckey, attrs);
        }
        Ok(Schema { classes, names })
    }

    pub fn type_name(&self, k: u32) -> &str {
        self.names.get(&k).map(String::as_str).unwrap_or("")
    }
}

/// One decoded attribute value (only the kinds Studio reads).
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Int(i64),
    Text(Option<String>),
    /// A typed reference: class key, collection key (0 = unset), and whether it resolves in this vault.
    Ref { class: u32, key: u32, resolved: bool },
    Other,
}

#[derive(Debug, Clone)]
pub struct Vault {
    v: Vec<u8>,
    b: Vec<u8>,
    bin_slot: HashMap<usize, usize>,
    vlt_slot: HashMap<usize, usize>,
    strs: HashMap<usize, String>,
    /// `(class, collection) → (record offset, size)`.
    by_key: HashMap<(u32, u32), (usize, usize)>,
    by_class: HashMap<u32, Vec<(usize, usize)>>,
}

impl Vault {
    pub fn parse(v: Vec<u8>, b: Vec<u8>) -> Result<Self> {
        let cv = chunks(&v)?;
        let cb = chunks(&b)?;
        let (ep, _) = *need(&cv, b"ExpN", "gccollections.vlt")?;
        let (bin_slot, vlt_slot, _) = fixups(&v, *need(&cv, b"PtrN", "gccollections.vlt")?)?;
        let (sp, ss) = *need(&cb, b"StrE", "gccollections.bin")?;
        let strs = strings(&b, sp + 8, sp + ss);
        let count = u32le(&v, ep + 8)? as usize;
        let (mut by_key, mut by_class) = (HashMap::new(), HashMap::<u32, Vec<(usize, usize)>>::new());
        for k in 0..count {
            let r = ep + 12 + k * 16;
            let (size, off) = (u32le(&v, r + 8)? as usize, u32le(&v, r + 12)? as usize);
            if off + size > v.len() || size < 40 {
                return Err(bad(format!("record {k} ({size} bytes at {off:#x}) lies outside gccollections.vlt")));
            }
            let (ck, cl) = (u32le(&v, off)?, u32le(&v, off + 4)?);
            by_key.entry((cl, ck)).or_insert((off, size));
            by_class.entry(cl).or_default().push((off, size));
        }
        Ok(Vault { v, b, bin_slot, vlt_slot, strs, by_key, by_class })
    }

    pub fn records_of(&self, class: u32) -> &[(usize, usize)] {
        self.by_class.get(&class).map(Vec::as_slice).unwrap_or(&[])
    }

    pub fn record(&self, class: u32, collection: u32) -> Option<(usize, usize)> {
        self.by_key.get(&(class, collection)).copied()
    }

    fn value(&self, schema: &Schema, type_key: u32, o: usize, in_vlt: bool) -> Result<Value> {
        let tname = schema.type_name(type_key);
        let slot = if in_vlt { self.vlt_slot.get(&o) } else { self.bin_slot.get(&o) };
        if tname == "EA::Reflection::Text" {
            return Ok(Value::Text(slot.and_then(|s| self.strs.get(s).cloned())));
        }
        let (base, at): (&[u8], usize) = match (in_vlt, slot) {
            (true, Some(s)) => (&self.b, *s),
            (true, None) => (&self.v, o),
            (false, _) => (&self.b, o),
        };
        if tname == "Attrib::RefSpec" {
            let (class, key) = (u32le(base, at)?, u32le(base, at + 4)?);
            return Ok(Value::Ref { class, key, resolved: key != 0 && self.by_key.contains_key(&(class, key)) });
        }
        let short = tname.rsplit("::").next().unwrap_or("");
        Ok(match short {
            "Int32" => Value::Int(i32le(base, at)? as i64),
            "UInt32" | "Key" => Value::Int(u32le(base, at)? as i64),
            "Int16" => Value::Int(u16le(base, at)? as i16 as i64),
            "UInt16" => Value::Int(u16le(base, at)? as i64),
            "Bool" | "UInt8" | "Int8" => Value::Int(*base.get(at).ok_or_else(|| bad("read past the end"))? as i64),
            _ if tname.starts_with("EA::AttribSys::Enums::") => Value::Int(i32le(base, at)? as i64),
            _ => Value::Other,
        })
    }

    /// Every attribute of the record at `off` that Studio can read: `(attribute key, value)`, blob attributes first,
    /// then the inline entries (arrays skipped).
    pub fn decode(&self, schema: &Schema, off: usize, size: usize) -> Result<Vec<(u32, Value)>> {
        let v = &self.v;
        let cl = u32le(v, off + 4)?;
        let mut out = Vec::new();
        if let (Some(attrs), Some(&blob)) = (schema.classes.get(&cl), self.vlt_slot.get(&(off + 32))) {
            for a in attrs.iter().filter(|a| a.count != 0xFFFF && a.flags & 1 == 0) {
                out.push((a.key, self.value(schema, a.type_key, blob + a.offset as usize, false)?));
            }
        }
        let n_types = u16le(v, off + 24)? as usize;
        let types: Vec<u32> = (0..n_types).map(|i| u32le(v, off + 40 + 4 * i)).collect::<Result<_>>()?;
        let mut p = (off + 40 + 4 * n_types + 7) & !7;
        let end = off + size;
        while p + 24 <= end && u32le(v, p)? == 0 {
            p += 8;
        }
        let arrays: std::collections::HashSet<u32> = schema.classes.get(&cl).into_iter().flatten().filter(|a| a.count == 0xFFFF).map(|a| a.key).collect();
        while p + 24 <= end {
            let ak = u32le(v, p)?;
            let ti = u16le(v, p + 16)? as usize;
            if !arrays.contains(&ak) {
                let tk = types.get(ti).copied().unwrap_or(0);
                out.push((ak, self.value(schema, tk, p + 8, true)?));
            }
            p += 24;
        }
        Ok(out)
    }

    /// The `FileRef` of a `ge_texture` record named by a typed reference, when it resolves.
    pub fn texture_file_ref(&self, schema: &Schema, class: u32, collection: u32) -> Result<Option<String>> {
        let Some((off, size)) = self.record(class, collection) else { return Ok(None) };
        let fr = key("FileRef");
        Ok(self.decode(schema, off, size)?.into_iter().find_map(|(k, v)| match (k == fr, v) {
            (true, Value::Text(t)) => t,
            _ => None,
        }))
    }
}

/// The four vault members of one install with their identities.
#[derive(Debug, Clone)]
pub struct VaultSource {
    pub archive: String,
    /// `(member name, SHA-256 of the unwrapped bytes)`.
    pub members: Vec<(String, String)>,
    /// The title whose recorded identity all four members match, if any.
    pub title: Option<String>,
    /// SHA-256 of the identity-definition file (`defs/ps4/vault-identities.json`) the title was matched against, as read.
    pub identities_sha256: String,
}

impl VaultSource {
    /// The exact identity of this read (receipt 66): the archive, the definitions file hash and every member's full hash.
    /// Two reads with the same key read the same member bytes against the same definitions. The granularity is
    /// intentionally coarse on the definitions side: any change to the file — another title's entry included — changes it.
    pub fn identity_key(&self) -> String {
        let mut k = format!("{}|vault-identities.json {}", self.archive, self.identities_sha256);
        for (n, s) in &self.members {
            k.push_str(&format!("|{n} {s}"));
        }
        k
    }

    /// The full hashes as review lines: one per member, then the definitions file.
    pub fn identity_lines(&self) -> Vec<String> {
        let mut out: Vec<String> = self.members.iter().map(|(n, s)| format!("{}: {n} sha256 {s}", self.archive)).collect();
        out.push(format!("defs/ps4/vault-identities.json sha256 {}", self.identities_sha256));
        out
    }

    /// Refuse unless every member matches the recorded identity of `project_title` (never another title's vault).
    pub fn require_title(&self, project_title: &str) -> Result<()> {
        match self.title.as_deref() {
            Some(t) if t == project_title => Ok(()),
            Some(t) => Err(DomainError::Validation(format!(
                "source mismatch: the attached install's vault is {t}'s, but the open project is {project_title}; Studio never checks an asset against another title's catalogue"
            ))),
            None => Err(DomainError::Validation(format!(
                "the attached install's vault ({}) matches no recorded PS4 title identity; it is not used",
                self.members.iter().map(|(n, s)| format!("{n} {}", &s[..12])).collect::<Vec<_>>().join(", ")
            ))),
        }
    }
}

#[derive(Debug, Clone, serde::Deserialize)]
struct Identities {
    titles: BTreeMap<String, BTreeMap<String, String>>,
}

/// The directory of one PS4 EB archive (entries only; payloads are read on demand with [`eb_read`]).
pub fn eb_index(archive: &Path) -> Result<Vec<tdb_archive_big::eb::EbEntry>> {
    let io = |e: std::io::Error| bad(format!("{}: {e}", archive.display()));
    let mut f = std::fs::File::open(archive).map_err(io)?;
    let mut hb = vec![0u8; tdb_archive_big::eb::HEADER_LEN];
    f.read_exact(&mut hb).map_err(io)?;
    let header = tdb_archive_big::eb::EbHeader::parse(&hb).map_err(|e| bad(format!("{}: {e}", archive.display())))?;
    let mut table = vec![0u8; header.table_end()];
    f.seek(SeekFrom::Start(0)).map_err(io)?;
    f.read_exact(&mut table).map_err(io)?;
    Ok(tdb_archive_big::eb::EbArchive::parse(&table).map_err(|e| bad(format!("{}: {e}", archive.display())))?.entries)
}

/// One member's unwrapped bytes (`chunkzip` decoded; `chunklzx` refused).
pub fn eb_read(archive: &Path, e: &tdb_archive_big::eb::EbEntry) -> Result<Vec<u8>> {
    let io = |x: std::io::Error| bad(format!("{}: {x}", archive.display()));
    let mut f = std::fs::File::open(archive).map_err(io)?;
    let mut raw = vec![0u8; e.size as usize];
    f.seek(SeekFrom::Start(e.offset)).map_err(io)?;
    f.read_exact(&mut raw).map_err(io)?;
    match tdb_archive_big::eb::decode_payload(raw) {
        Ok(tdb_archive_big::eb::Payload::Data(d)) => Ok(d),
        Ok(_) => Err(bad(format!("{}: an undecodable member form", e.path))),
        Err(x) => Err(bad(format!("{}: {x}", e.path))),
    }
}

/// The single member at `path` (case-insensitive) of `archive`, unwrapped; refused when absent or listed twice.
pub fn eb_member(archive: &Path, entries: &[tdb_archive_big::eb::EbEntry], path: &str) -> Result<Vec<u8>> {
    let hits: Vec<_> = entries.iter().filter(|e| e.path.eq_ignore_ascii_case(path)).collect();
    let [e] = hits.as_slice() else {
        return Err(bad(format!("{} holds {} copies of {path}", archive.display(), hits.len())));
    };
    eb_read(archive, e)
}

/// Read the four vault members from `<install>/databigs/boot.big` and identify them against
/// `defs/ps4/vault-identities.json`. Returns the bytes (`gccollections.vlt`, `.bin`, `gcschema.vlt`, `.bin`).
pub fn read_install_vault(install_root: &Path, defs_root: &Path) -> Result<(VaultSource, [Vec<u8>; 4])> {
    let archive = install_root.join("databigs").join("boot.big");
    let entries = eb_index(&archive)?;
    let mut bytes: Vec<Vec<u8>> = Vec::new();
    let mut members = Vec::new();
    for name in FILES {
        let plain = eb_member(&archive, &entries, &format!("{ATTRIB_DIR}{name}"))?;
        members.push((name.to_string(), format!("{:x}", Sha256::digest(&plain))));
        bytes.push(plain);
    }
    // Receipt 66: the definitions are hashed as read, so a review binds the exact identity records it was matched against.
    let raw = std::fs::read(defs_root.join("ps4/vault-identities.json")).map_err(|e| bad(format!("vault identities: {e}")))?;
    let identities_sha256 = format!("{:x}", Sha256::digest(&raw));
    let ids: Identities = serde_json::from_slice(&raw).map_err(|e| bad(format!("vault identities: {e}")))?;
    let title = ids.titles.iter().find(|(_, h)| members.iter().all(|(n, s)| h.get(n) == Some(s))).map(|(t, _)| t.clone());
    let [a, b, c, d]: [Vec<u8>; 4] = bytes.try_into().map_err(|_| bad("member count"))?;
    Ok((VaultSource { archive: "databigs/boot.big".into(), members, title, identities_sha256 }, [a, b, c, d]))
}

/// Every value of the attributes of `class` whose type is the enum `…Enums::<enum_name>::<enum_name>`.
pub fn enum_values(vault: &Vault, schema: &Schema, class: &str, enum_name: &str) -> Result<std::collections::BTreeSet<i64>> {
    let ck = key(class);
    let suffix = format!("Enums::{enum_name}::{enum_name}");
    let keys: Vec<u32> = schema.classes.get(&ck).into_iter().flatten().filter(|a| schema.type_name(a.type_key).ends_with(&suffix)).map(|a| a.key).collect();
    let mut out = std::collections::BTreeSet::new();
    if keys.is_empty() {
        return Ok(out);
    }
    for &(off, size) in vault.records_of(ck) {
        for (k, v) in vault.decode(schema, off, size)? {
            if let (true, Value::Int(i)) = (keys.contains(&k), v) {
                out.insert(i);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lookup2_matches_the_research_keys() {
        // Class and attribute keys the research recorded for NHL 21 (R7 §1; `vlt_hash` in the probe).
        // Values computed with research's own `vlt_hash` (probes/vlt_collections.py), across the 12-byte boundary.
        for (s, k) in [
            ("", 0x82fc1624u32),
            ("a", 0x628300e4),
            ("FileRef", 0xb35ab3db),
            ("DebugName", 0x6adabcfa),
            ("GoalieMaskTexture", 0x98497d7b),
            ("GoalieHelmetBrand", 0xa3a019b1),
            ("ge_nhl_goaliehelmettexturepack", 0x7fb904e1),
            ("abcdefghijkl", 0x0e433798),
            ("abcdefghijklm", 0xee05aae8),
            ("ge_texture", 0xed5e8a2c),
        ] {
            assert_eq!(key(s), k, "{s:?}");
        }
    }
}
