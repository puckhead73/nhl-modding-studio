//! NHL 24 PS4 (CUSA37933 01.60) legacy database read and `game-mods` overlay write.
//!
//! The emulator stacks `game-mods` over `game-UPDATE` over `game`. Studio writes only into
//! `game-mods`: an edited `Patch/Gen4b/globals.toc` plus one new CAS holding the rebuilt
//! database batch and collector manifest as stored blocks. Every write is rebuilt from the
//! stock (non-Studio) view, Studio's files are recorded with their hashes outside the game
//! tree, and a file Studio does not own is never replaced or removed.
//!
//! Game-verified for this target only: docs/validation/nhl24-fbw1-stage-a-20261101.md.
use crate::{blocks, invalid, le, manifest, oodle::Oodle, overlay, slice, toc, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, HashMap},
    fs,
    io::{Read, Seek, SeekFrom, Write},
    path::{Path, PathBuf},
};

pub const GLOBALS_TOC: &str = "Patch/Gen4b/globals.toc";
pub const COLLECTOR: &str = "globals/chunkfilecollector";
pub const NHL_GAME_DB: &str = "gamemodes/database/t3db/nhl_game.db";
pub const NHL_GAME_META: &str = "gamemodes/database/t3db/nhl_game-meta.xml";
const RECORD: &str = "nhl24-overlay.json";
const MAX_CHUNK: usize = 256 * 1024 * 1024;
/// Executables that hold the install open while the game runs.
const EMULATORS: [&str; 3] = ["shadps4", "nhl24_fix", "nhl24_new"];

/// The three emulator layers plus Studio's state folder (outside the game tree).
#[derive(Clone, Debug)]
pub struct Roots {
    pub game: PathBuf,
    pub update: PathBuf,
    pub mods: PathBuf,
    pub state: PathBuf,
}
impl Roots {
    /// `root` is the install folder holding `game`, `game-UPDATE` and `game-mods`.
    pub fn install(root: &Path) -> Self {
        Self {
            game: root.join("game"),
            update: root.join("game-UPDATE"),
            mods: root.join("game-mods"),
            state: root.join("studio"),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct OwnedFile {
    pub path: String,
    pub sha256: String,
}
/// Studio's ownership record for `game-mods`.
#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Record {
    pub version: u32,
    pub files: Vec<OwnedFile>,
    /// Folders under `game-mods` that Studio created, deepest last.
    pub created_dirs: Vec<String>,
}

/// A forward-slash path of plain components that cannot leave the root it is joined to.
fn safe_rel(s: &str) -> Result<()> {
    if s.is_empty()
        || s.contains(['\\', ':'])
        || s.split('/').any(|p| p.is_empty() || p == "." || p == "..")
    {
        return Err(invalid(format!("unsafe relative path {s:?}")));
    }
    Ok(())
}
fn sha256(b: &[u8]) -> String {
    crate::hex(&Sha256::digest(b))
}
fn cas_rel(id: u32) -> Result<String> {
    if id >> 24 != 0 || (id >> 16) > 1 {
        return Err(invalid("unsupported PS4 CAS identifier"));
    }
    Ok(format!(
        "{}/Gen4b/superbundlelayout/nhl_installpackage_{:02}/cas_{:02}.cas",
        if id >> 16 == 0 { "Data" } else { "Patch" },
        (id >> 8) & 255,
        id & 255
    ))
}

/// A view of the layered install, with or without Studio's own overlay files.
pub struct View<'a> {
    roots: &'a Roots,
    record: Record,
    studio: bool,
}
impl<'a> View<'a> {
    pub fn open(roots: &'a Roots, studio: bool) -> Result<Self> {
        Ok(Self {
            roots,
            record: load_record(roots)?,
            studio,
        })
    }
    /// Studio's files in `game-mods`, by relative path.
    pub fn owned_files(&self) -> Vec<String> {
        self.record.files.iter().map(|f| f.path.clone()).collect()
    }
    fn owned(&self, rel: &str) -> bool {
        self.record.files.iter().any(|f| f.path == rel)
    }
    /// The file the emulator would open for `rel`, skipping Studio's files in a stock view.
    pub fn resolve(&self, rel: &str) -> Result<Option<PathBuf>> {
        safe_rel(rel)?;
        let m = self.roots.mods.join(rel);
        if m.is_file() && (self.studio || !self.owned(rel)) {
            return Ok(Some(m));
        }
        for root in [&self.roots.update, &self.roots.game] {
            let p = root.join(rel);
            if p.is_file() {
                return Ok(Some(p));
            }
        }
        Ok(None)
    }
    /// True when `rel` resolves to a `game-mods` file that Studio does not own.
    pub fn foreign(&self, rel: &str) -> bool {
        self.roots.mods.join(rel).is_file() && !self.owned(rel)
    }
    pub fn read(&self, rel: &str) -> Result<Vec<u8>> {
        let p = self
            .resolve(rel)?
            .ok_or_else(|| invalid(format!("{rel} is not in the install")))?;
        crate::install::read_index(&p)
    }
    pub fn read_span(&self, cas: u32, offset: u32, size: u32) -> Result<Vec<u8>> {
        if size as usize > MAX_CHUNK {
            return Err(invalid("span exceeds 256 MiB limit"));
        }
        let rel = cas_rel(cas)?;
        let p = self
            .resolve(&rel)?
            .ok_or_else(|| invalid(format!("{rel} is missing")))?;
        let mut f = fs::File::open(p)?;
        if offset as u64 + size as u64 > f.metadata()?.len() {
            return Err(invalid("CAS span beyond end of file"));
        }
        f.seek(SeekFrom::Start(offset as u64))?;
        let mut b = vec![0; size as usize];
        f.read_exact(&mut b)?;
        Ok(b)
    }
}

fn decode(bytes: &[u8], oodle: Option<&Oodle>) -> Result<Vec<u8>> {
    blocks::decode_with(bytes, MAX_CHUNK, |codec, data, n| match oodle {
        Some(o) => o.decode(codec, data, n),
        None => Err(invalid(format!(
            "codec {codec:#04x} requires the Oodle decoder"
        ))),
    })
}

/// One collector manifest entry (56 bytes): name pointer, stored range, decoded range, chunk.
#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct Entry {
    pub ordinal: usize,
    pub name: String,
    pub compressed_start: u64,
    pub compressed_end: u64,
    pub offset: u64,
    pub size: u64,
    pub chunk: String,
}
/// The decoded collector manifest: header, 56-byte entries, a packed-chunk size table
/// (u64 decoded size + GUID, 24 bytes each) and a string pool.
#[derive(Clone, Debug)]
pub struct Directory {
    pub bytes: Vec<u8>,
    pub entries: Vec<Entry>,
    /// `(record offset, decoded size, chunk GUID)` per packed-size record.
    pub sizes: Vec<(usize, u64, String)>,
}
fn u64_at(b: &[u8], p: usize) -> Result<u64> {
    Ok(u64::from_le_bytes(slice(b, p, 8)?.try_into().unwrap()))
}
impl Directory {
    pub fn parse(bytes: Vec<u8>) -> Result<Self> {
        let b = &bytes;
        if le(b, 0)? != 2 {
            return Err(invalid("unsupported collector manifest version"));
        }
        let count = le(b, 12)? as usize;
        let start = u64_at(b, 16)? as usize;
        if start != 64 || count > 1_000_000 {
            return Err(invalid("unsupported collector file table"));
        }
        slice(b, start, count * 56)?;
        let (n, table) = (le(b, 36)? as usize, u64_at(b, 40)? as usize);
        slice(b, table, n * 24)?;
        let mut entries = Vec::with_capacity(count);
        for i in 0..count {
            let p = start + i * 56;
            let name_at = u64_at(b, p)? as usize;
            if name_at < start + count * 56 {
                return Err(invalid("collector filename outside string pool"));
            }
            let name = crate::cstr(b, name_at)?;
            entries.push(Entry {
                ordinal: i,
                name,
                compressed_start: u64_at(b, p + 8)?,
                compressed_end: u64_at(b, p + 16)?,
                offset: u64_at(b, p + 24)?,
                size: u64_at(b, p + 32)?,
                chunk: crate::hex(slice(b, p + 40, 16)?),
            });
        }
        let sizes = (0..n)
            .map(|i| {
                let q = table + i * 24;
                Ok((q, u64_at(b, q)?, crate::hex(slice(b, q + 8, 16)?)))
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            bytes,
            entries,
            sizes,
        })
    }
    pub fn find(&self, name: &str) -> Result<&Entry> {
        let mut hits = self.entries.iter().filter(|e| e.name == name);
        let e = hits
            .next()
            .ok_or_else(|| invalid(format!("collector lists no {name}")))?;
        if hits.next().is_some() {
            return Err(invalid(format!("{name} is listed more than once")));
        }
        Ok(e)
    }
    /// The files packed in `chunk`, in decoded order; they must tile it exactly.
    pub fn packed(&self, chunk: &str, decoded_len: usize) -> Result<Vec<&Entry>> {
        let mut v: Vec<_> = self.entries.iter().filter(|e| e.chunk == chunk).collect();
        v.sort_by_key(|e| e.offset);
        let mut at = 0u64;
        for e in &v {
            if e.offset != at {
                return Err(invalid("packed chunk files are not contiguous"));
            }
            at += e.size;
        }
        if at as usize != decoded_len {
            return Err(invalid("packed chunk files do not cover the chunk"));
        }
        let recorded: Vec<_> = self.sizes.iter().filter(|s| s.2 == chunk).collect();
        if recorded.len() != 1 || recorded[0].1 != at {
            return Err(invalid("packed chunk size record missing or stale"));
        }
        Ok(v)
    }
}

/// The stock inputs a database write is rebuilt from.
#[derive(Clone, Debug)]
pub struct Stock {
    pub toc: Vec<u8>,
    pub manifest_chunk: String,
    /// Decoded block sizes of the stock manifest; the rewrite keeps the same framing.
    pub manifest_blocks: Vec<usize>,
    pub directory: Directory,
    pub batch_chunk: String,
    pub batch: Vec<u8>,
}

/// The current legacy database, read through the effective view.
#[derive(Debug)]
pub struct Database {
    pub db: Vec<u8>,
    pub meta_xml: Vec<u8>,
    /// True when Studio's overlay currently supplies the database.
    pub from_overlay: bool,
}

/// Resolve the collector manifest and the database batch for a view.
fn locate(view: &View, oodle: Option<&Oodle>) -> Result<Stock> {
    let toc_bytes = view.read(GLOBALS_TOC)?;
    let slots = overlay::chunk_slots(&toc_bytes)?;
    let mut chunks: HashMap<String, &overlay::ChunkSlot> = HashMap::new();
    for s in &slots {
        if chunks.insert(s.guid.clone(), s).is_some() {
            return Err(invalid("duplicate chunk in Patch globals.toc"));
        }
    }
    let parsed = toc::parse(&toc_bytes)?;
    let mut ebx = None;
    for b in &parsed.bundles {
        let m = &b.spans[0];
        let assets = manifest::parse(&view.read_span(m.cas, m.offset, m.size)?)?;
        if b.spans.len() != assets.len() + 1 {
            return Err(invalid("asset count does not match spans"));
        }
        if let Some(i) = assets
            .iter()
            .position(|a| a.kind == "ebx" && a.name == COLLECTOR)
        {
            if ebx.is_some() {
                return Err(invalid("collector EBX appears in more than one bundle"));
            }
            ebx = Some(b.spans[i + 1].clone());
        }
    }
    let e = ebx.ok_or_else(|| invalid("Patch globals.toc has no collector EBX"))?;
    let ebx = decode(&view.read_span(e.cas, e.offset, e.size)?, oodle)?;
    // The collector names its manifest chunk by GUID. Accept exactly one GUID-sized window
    // that is a Patch chunk and decodes to a directory listing the database.
    let mut found = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for i in 0..ebx.len().saturating_sub(15) {
        let g = crate::hex(&ebx[i..i + 16]);
        if !seen.insert(g.clone()) {
            continue;
        }
        if let Some(s) = chunks.get(&g) {
            let raw = view.read_span(s.cas, s.offset, s.size)?;
            if let Ok(d) = decode(&raw, oodle).and_then(Directory::parse) {
                if d.find(NHL_GAME_DB).is_ok() {
                    let sizes = blocks::inspect(&raw)?
                        .iter()
                        .map(|b| b.decoded_size as usize)
                        .collect::<Vec<_>>();
                    found.push((g, sizes, d));
                }
            }
        }
    }
    if found.len() != 1 {
        return Err(invalid(format!(
            "expected one collector manifest, found {}",
            found.len()
        )));
    }
    let (manifest_chunk, manifest_blocks, directory) = found.pop().unwrap();
    let batch_chunk = directory.find(NHL_GAME_DB)?.chunk.clone();
    let s = chunks
        .get(&batch_chunk)
        .ok_or_else(|| invalid("database batch is not a Patch globals chunk"))?;
    let batch = decode(&view.read_span(s.cas, s.offset, s.size)?, oodle)?;
    directory.packed(&batch_chunk, batch.len())?;
    Ok(Stock {
        toc: toc_bytes,
        manifest_chunk,
        manifest_blocks,
        directory,
        batch_chunk,
        batch,
    })
}

/// Read the database the game currently loads (Studio's overlay included).
pub fn read_database(roots: &Roots, oodle: Option<&Oodle>) -> Result<Database> {
    let view = View::open(roots, true)?;
    let from_overlay = !view.record.files.is_empty();
    let s = locate(&view, oodle)?;
    let cut = |name| -> Result<Vec<u8>> {
        let e = s.directory.find(name)?;
        Ok(slice(&s.batch, e.offset as usize, e.size as usize)?.to_vec())
    };
    Ok(Database {
        db: cut(NHL_GAME_DB)?,
        meta_xml: cut(NHL_GAME_META)?,
        from_overlay,
    })
}

/// The stock inputs for a write. Refuses when another mod replaces `globals.toc`.
pub fn stock(roots: &Roots, oodle: Option<&Oodle>) -> Result<Stock> {
    let view = View::open(roots, false)?;
    if view.foreign(GLOBALS_TOC) {
        return Err(invalid(
            "another mod in game-mods replaces Patch/Gen4b/globals.toc; remove it first",
        ));
    }
    locate(&view, oodle)
}

/// Files to place in `game-mods`, keyed by relative path.
#[derive(Clone, Debug)]
pub struct Plan {
    pub files: BTreeMap<String, Vec<u8>>,
    pub db_len: (usize, usize),
}

/// Rebuild the batch around `db`, rewrite the collector manifest, and repoint both chunks
/// at one new CAS in `cas_id`. Verified by reading the result back before it is returned.
pub fn plan(stock: &Stock, db: &[u8], cas_id: u32) -> Result<Plan> {
    let d = &stock.directory;
    let files = d.packed(&stock.batch_chunk, stock.batch.len())?;
    let mut parts: Vec<(&str, &[u8])> = files
        .iter()
        .map(|e| {
            Ok((
                e.name.as_str(),
                slice(&stock.batch, e.offset as usize, e.size as usize)?,
            ))
        })
        .collect::<Result<_>>()?;
    let old_len = d.find(NHL_GAME_DB)?.size as usize;
    for p in &mut parts {
        if p.0 == NHL_GAME_DB {
            p.1 = db;
        }
    }
    let batch: Vec<u8> = parts.iter().flat_map(|p| p.1.iter().copied()).collect();
    let batch_stored = overlay::stored(&batch);
    let mut man = d.bytes.clone();
    let mut at = 0usize;
    for (e, (_, data)) in files.iter().zip(&parts) {
        let p = 64 + e.ordinal * 56;
        let (lo, hi) = overlay::stored_range(at, data.len(), batch_stored.len());
        for (k, v) in [lo, hi, at as u64, data.len() as u64]
            .into_iter()
            .enumerate()
        {
            man[p + 8 + k * 8..p + 16 + k * 8].copy_from_slice(&v.to_le_bytes());
        }
        at += data.len();
    }
    let (q, ..) = d
        .sizes
        .iter()
        .find(|s| s.2 == stock.batch_chunk)
        .ok_or_else(|| invalid("batch size record missing"))?;
    man[*q..*q + 8].copy_from_slice(&(batch.len() as u64).to_le_bytes());
    let man_stored = overlay::stored_with(&man, &stock.manifest_blocks)?;
    let mut cas = batch_stored.clone();
    cas.extend_from_slice(&man_stored);
    let len32 = |n: usize| u32::try_from(n).map_err(|_| invalid("CAS exceeds 4 GiB"));
    let toc = overlay::repoint(
        &stock.toc,
        &[
            (
                stock.batch_chunk.clone(),
                cas_id,
                0,
                len32(batch_stored.len())?,
            ),
            (
                stock.manifest_chunk.clone(),
                cas_id,
                len32(batch_stored.len())?,
                len32(man_stored.len())?,
            ),
        ],
    )?;
    // Read back exactly as the game resolves it.
    let slots = overlay::chunk_slots(&toc)?;
    let get = |g: &str| -> Result<Vec<u8>> {
        let s = slots
            .iter()
            .find(|s| s.guid == g && s.cas == cas_id)
            .ok_or_else(|| invalid("repointed chunk missing"))?;
        blocks::decode_stored(slice(&cas, s.offset as usize, s.size as usize)?, MAX_CHUNK)
    };
    let (b2, d2) = (
        get(&stock.batch_chunk)?,
        Directory::parse(get(&stock.manifest_chunk)?)?,
    );
    for (name, data) in &parts {
        let e = d2.find(name)?;
        if slice(&b2, e.offset as usize, e.size as usize)? != *data {
            return Err(invalid(format!("read-back mismatch for {name}")));
        }
    }
    d2.packed(&stock.batch_chunk, b2.len())?;
    if d2.entries.len() != d.entries.len() {
        return Err(invalid("collector entry count changed"));
    }
    let mut out = BTreeMap::new();
    out.insert(cas_rel(cas_id)?, cas);
    out.insert(GLOBALS_TOC.to_string(), toc);
    Ok(Plan {
        files: out,
        db_len: (old_len, db.len()),
    })
}

/// The CAS Studio writes into: its own existing one, else the lowest Patch catalogue-0
/// index that no layer holds. Index 2 is the one game-verified so far.
pub fn choose_cas(roots: &Roots) -> Result<u32> {
    let record = load_record(roots)?;
    for f in &record.files {
        for n in 1..=255u32 {
            if f.path == cas_rel((1 << 16) | n)? {
                return Ok((1 << 16) | n);
            }
        }
    }
    for n in 1..=255u32 {
        let rel = cas_rel((1 << 16) | n)?;
        if [&roots.mods, &roots.update, &roots.game]
            .iter()
            .all(|r| !r.join(&rel).exists())
        {
            return Ok((1 << 16) | n);
        }
    }
    Err(invalid("no free CAS index in Patch catalogue 0"))
}

fn load_record(roots: &Roots) -> Result<Record> {
    let p = roots.state.join(RECORD);
    if !p.is_file() {
        return Ok(Record {
            version: 1,
            ..Default::default()
        });
    }
    let r: Record = serde_json::from_slice(&fs::read(p)?).map_err(|e| invalid(e.to_string()))?;
    if r.version != 1 {
        return Err(invalid("unsupported overlay record version"));
    }
    // The record drives deletions in game-mods; never follow a path out of it.
    for p in r.files.iter().map(|f| &f.path).chain(&r.created_dirs) {
        safe_rel(p)?;
    }
    Ok(r)
}
fn save_record(roots: &Roots, r: &Record) -> Result<()> {
    fs::create_dir_all(&roots.state)?;
    let mut t = tempfile::NamedTempFile::new_in(&roots.state)?;
    t.write_all(&serde_json::to_vec_pretty(r).map_err(|e| invalid(e.to_string()))?)?;
    t.as_file().sync_all()?;
    t.persist(roots.state.join(RECORD)).map_err(|e| e.error)?;
    Ok(())
}

/// The running emulator, if any; writes are refused while it holds the install.
pub fn running_emulator() -> Option<String> {
    #[cfg(windows)]
    let out = std::process::Command::new("tasklist")
        .args(["/FO", "CSV", "/NH"])
        .output();
    #[cfg(not(windows))]
    let out = std::process::Command::new("ps")
        .args(["-A", "-o", "comm="])
        .output();
    let text = String::from_utf8_lossy(&out.ok()?.stdout).to_lowercase();
    text.lines()
        .map(|l| l.trim().trim_start_matches('"'))
        .find(|l| EMULATORS.iter().any(|e| l.starts_with(e)))
        .map(|l| l.split('"').next().unwrap_or(l).to_string())
}

/// Check every target before touching anything: absent, or Studio's with an unchanged hash.
fn check_owned(roots: &Roots, record: &Record, rel: &str) -> Result<()> {
    let p = roots.mods.join(rel);
    if !p.exists() {
        return Ok(());
    }
    let f = record
        .files
        .iter()
        .find(|f| f.path == rel)
        .ok_or_else(|| invalid(format!("game-mods/{rel} belongs to another mod")))?;
    if sha256(&fs::read(&p)?) != f.sha256 {
        return Err(invalid(format!(
            "game-mods/{rel} changed since Studio wrote it; restore or remove it first"
        )));
    }
    Ok(())
}

/// Write `plan` into `game-mods`. CAS before TOC; on failure every target is put back.
pub fn apply(roots: &Roots, plan: &Plan, running: impl Fn() -> Option<String>) -> Result<Record> {
    if let Some(p) = running() {
        return Err(invalid(format!("close {p} before saving to NHL 24")));
    }
    let mut record = load_record(roots)?;
    for rel in plan.files.keys() {
        check_owned(roots, &record, rel)?;
    }
    let stale: Vec<OwnedFile> = record
        .files
        .iter()
        .filter(|f| !plan.files.contains_key(&f.path))
        .cloned()
        .collect();
    for f in &stale {
        check_owned(roots, &record, &f.path)?;
    }
    // Original bytes of every target, for rollback (None = did not exist).
    let mut prior: Vec<(PathBuf, Option<Vec<u8>>)> = Vec::new();
    let mut created = record.created_dirs.clone();
    let order: Vec<_> = plan
        .files
        .iter()
        .filter(|(k, _)| k.ends_with(".cas"))
        .chain(plan.files.iter().filter(|(k, _)| !k.ends_with(".cas")))
        .collect();
    let result = (|| -> Result<()> {
        for (rel, bytes) in &order {
            let p = roots.mods.join(rel);
            let parent = p.parent().ok_or_else(|| invalid("bad target"))?;
            let mut missing = Vec::new();
            let mut d = parent.to_path_buf();
            while !d.exists() && d.starts_with(&roots.mods) {
                missing.push(d.clone());
                d = d.parent().unwrap().to_path_buf();
            }
            for m in missing.iter().rev() {
                fs::create_dir(m)?;
                let r = m
                    .strip_prefix(&roots.mods)
                    .unwrap()
                    .to_string_lossy()
                    .replace('\\', "/");
                created.push(r);
            }
            prior.push((
                p.clone(),
                if p.exists() {
                    Some(fs::read(&p)?)
                } else {
                    None
                },
            ));
            let mut t = tempfile::NamedTempFile::new_in(parent)?;
            t.write_all(bytes)?;
            t.as_file().sync_all()?;
            t.persist(&p).map_err(|e| e.error)?;
        }
        for f in &stale {
            let p = roots.mods.join(&f.path);
            prior.push((p.clone(), Some(fs::read(&p)?)));
            fs::remove_file(&p)?;
        }
        Ok(())
    })();
    if let Err(e) = result {
        for (p, old) in prior.iter().rev() {
            let _ = match old {
                Some(b) => fs::write(p, b),
                None => fs::remove_file(p),
            };
        }
        return Err(e);
    }
    record.files = plan
        .files
        .iter()
        .map(|(k, v)| OwnedFile {
            path: k.clone(),
            sha256: sha256(v),
        })
        .collect();
    record.created_dirs = created;
    save_record(roots, &record)?;
    Ok(record)
}

/// Remove every Studio file from `game-mods`. Refuses, before deleting anything, if any
/// of them changed since Studio wrote it.
pub fn restore(roots: &Roots, running: impl Fn() -> Option<String>) -> Result<usize> {
    if let Some(p) = running() {
        return Err(invalid(format!("close {p} before restoring NHL 24")));
    }
    let record = load_record(roots)?;
    for f in &record.files {
        check_owned(roots, &record, &f.path)?;
    }
    let mut n = 0;
    for f in &record.files {
        let p = roots.mods.join(&f.path);
        if p.exists() {
            fs::remove_file(p)?;
            n += 1;
        }
    }
    for d in record.created_dirs.iter().rev() {
        let _ = fs::remove_dir(roots.mods.join(d));
    }
    let p = roots.state.join(RECORD);
    if p.exists() {
        fs::remove_file(p)?;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn guid(n: u8) -> [u8; 16] {
        let mut g = [0u8; 16];
        g[0] = n;
        g[15] = 0xee;
        g
    }

    /// A collector manifest with `files` packed into chunk `batch` (plus one unrelated entry).
    fn manifest(files: &[(&str, usize)], batch: [u8; 16], other: [u8; 16]) -> Vec<u8> {
        let all: Vec<(&str, usize, [u8; 16])> = files
            .iter()
            .map(|(n, s)| (*n, *s, batch))
            .chain([("other/file.bin", 5, other)])
            .collect();
        let count = all.len();
        let table = 64 + count * 56;
        let pool = table + 2 * 24;
        let mut b = vec![0u8; pool];
        b[0..4].copy_from_slice(&2u32.to_le_bytes());
        b[12..16].copy_from_slice(&(count as u32).to_le_bytes());
        b[16..24].copy_from_slice(&64u64.to_le_bytes());
        b[36..40].copy_from_slice(&2u32.to_le_bytes());
        b[40..48].copy_from_slice(&(table as u64).to_le_bytes());
        let mut at = 0u64;
        for (i, (name, size, g)) in all.iter().enumerate() {
            let p = 64 + i * 56;
            let name_at = b.len() as u64;
            b.extend_from_slice(name.as_bytes());
            b.push(0);
            let off = if *g == batch { at } else { 0 };
            for (k, v) in [name_at, 0, 0, off, *size as u64].into_iter().enumerate() {
                b[p + k * 8..p + 8 + k * 8].copy_from_slice(&v.to_le_bytes());
            }
            b[p + 40..p + 56].copy_from_slice(g);
            if *g == batch {
                at += *size as u64;
            }
        }
        b[table..table + 8].copy_from_slice(&at.to_le_bytes());
        b[table + 8..table + 24].copy_from_slice(&batch);
        b[table + 24..table + 32].copy_from_slice(&5u64.to_le_bytes());
        b[table + 32..table + 48].copy_from_slice(&other);
        b
    }

    fn stock_fixture() -> Stock {
        let (bg, mg, og) = (guid(1), guid(2), guid(3));
        let files = [
            ("a.csv", 10),
            (NHL_GAME_META, 7),
            (NHL_GAME_DB, 70_000),
            ("z.csv", 3),
        ];
        let batch: Vec<u8> = (0..70_020u32).map(|i| (i % 251) as u8).collect();
        let toc = crate::overlay::tests_support::toc(&[
            (bg, 0x10001, 100, 50),
            (mg, 0x10001, 200, 60),
            (og, 0x10001, 300, 70),
        ]);
        Stock {
            toc,
            manifest_chunk: crate::hex(&mg),
            manifest_blocks: vec![manifest(&files, bg, og).len()],
            directory: Directory::parse(manifest(&files, bg, og)).unwrap(),
            batch_chunk: crate::hex(&bg),
            batch,
        }
    }

    #[test]
    fn plan_resizes_the_database_and_rewrites_the_directory() {
        let s = stock_fixture();
        let db: Vec<u8> = vec![9; 200_000];
        let p = plan(&s, &db, 0x10002).unwrap();
        assert_eq!(p.db_len, (70_000, 200_000));
        let cas = &p.files["Patch/Gen4b/superbundlelayout/nhl_installpackage_00/cas_02.cas"];
        let toc = &p.files[GLOBALS_TOC];
        assert_eq!(
            toc[..overlay::WRAPPED_PAYLOAD],
            s.toc[..overlay::WRAPPED_PAYLOAD]
        );
        let slots = overlay::chunk_slots(toc).unwrap();
        let m = slots.iter().find(|x| x.guid == s.manifest_chunk).unwrap();
        let d = Directory::parse(
            blocks::decode_stored(&cas[m.offset as usize..][..m.size as usize], MAX_CHUNK).unwrap(),
        )
        .unwrap();
        let e = d.find(NHL_GAME_DB).unwrap();
        assert_eq!((e.offset, e.size), (17, 200_000));
        assert_eq!(d.find("z.csv").unwrap().offset, 200_017);
        assert_eq!(
            d.find("other/file.bin").unwrap(),
            s.directory.find("other/file.bin").unwrap()
        );
        // The unrelated chunk keeps its stock TOC record.
        let o = slots
            .iter()
            .find(|x| x.guid == crate::hex(&guid(3)))
            .unwrap();
        assert_eq!((o.cas, o.offset, o.size), (0x10001, 300, 70));
    }

    #[test]
    fn plan_refuses_a_directory_that_does_not_tile_the_batch() {
        let mut s = stock_fixture();
        s.batch.push(0);
        assert!(plan(&s, &[1, 2, 3], 0x10002).is_err());
    }

    fn roots(dir: &Path) -> Roots {
        let r = Roots::install(dir);
        for p in [&r.game, &r.update, &r.mods] {
            fs::create_dir_all(p).unwrap();
        }
        r
    }

    fn sample_plan() -> Plan {
        let mut files = BTreeMap::new();
        files.insert(
            "Patch/Gen4b/superbundlelayout/nhl_installpackage_00/cas_02.cas".into(),
            vec![1, 2, 3],
        );
        files.insert(GLOBALS_TOC.into(), vec![4, 5]);
        Plan {
            files,
            db_len: (0, 0),
        }
    }

    #[test]
    fn apply_and_restore_leave_foreign_files_alone() {
        let dir = tempfile::tempdir().unwrap();
        let r = roots(dir.path());
        let foreign = r.mods.join("Patch/Gen4b/contentlaunchsb.toc");
        fs::create_dir_all(foreign.parent().unwrap()).unwrap();
        fs::write(&foreign, b"deke").unwrap();
        fs::write(r.mods.join("eboot.bin"), b"elf").unwrap();
        let rec = apply(&r, &sample_plan(), || None).unwrap();
        assert_eq!(rec.files.len(), 2);
        assert_eq!(fs::read(r.mods.join(GLOBALS_TOC)).unwrap(), vec![4, 5]);
        assert_eq!(choose_cas(&r).unwrap(), 0x10002);
        // Re-applying over Studio's own files is allowed.
        apply(&r, &sample_plan(), || None).unwrap();
        assert_eq!(restore(&r, || None).unwrap(), 2);
        assert!(!r.mods.join(GLOBALS_TOC).exists());
        assert!(!r.mods.join("Patch/Gen4b/superbundlelayout").exists());
        assert_eq!(fs::read(&foreign).unwrap(), b"deke");
        assert_eq!(fs::read(r.mods.join("eboot.bin")).unwrap(), b"elf");
        assert!(!r.state.join(RECORD).exists());
    }

    #[test]
    fn apply_refuses_foreign_targets_and_a_running_emulator() {
        let dir = tempfile::tempdir().unwrap();
        let r = roots(dir.path());
        assert!(apply(&r, &sample_plan(), || Some("shadps4.exe".into())).is_err());
        fs::create_dir_all(r.mods.join("Patch/Gen4b")).unwrap();
        fs::write(r.mods.join(GLOBALS_TOC), b"other mod").unwrap();
        let err = apply(&r, &sample_plan(), || None).unwrap_err();
        assert!(err.to_string().contains("another mod"));
        // Nothing was written: the CAS target does not exist.
        assert!(!r
            .mods
            .join("Patch/Gen4b/superbundlelayout/nhl_installpackage_00/cas_02.cas")
            .exists());
        assert!(View::open(&r, false).unwrap().foreign(GLOBALS_TOC));
    }

    #[test]
    fn restore_refuses_a_studio_file_changed_by_hand() {
        let dir = tempfile::tempdir().unwrap();
        let r = roots(dir.path());
        apply(&r, &sample_plan(), || None).unwrap();
        fs::write(r.mods.join(GLOBALS_TOC), b"edited").unwrap();
        assert!(restore(&r, || None).is_err());
        // Nothing was deleted.
        assert!(r
            .mods
            .join("Patch/Gen4b/superbundlelayout/nhl_installpackage_00/cas_02.cas")
            .exists());
    }

    #[test]
    fn a_record_pointing_outside_game_mods_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let r = roots(dir.path());
        fs::create_dir_all(&r.state).unwrap();
        let bad =
            r#"{"version":1,"files":[{"path":"../game/eboot.bin","sha256":""}],"created_dirs":[]}"#;
        fs::write(r.state.join(RECORD), bad).unwrap();
        assert!(restore(&r, || None).is_err());
        assert!(View::open(&r, true).is_err());
    }

    #[test]
    fn choose_cas_skips_indices_held_by_any_layer() {
        let dir = tempfile::tempdir().unwrap();
        let r = roots(dir.path());
        let pkg = "Patch/Gen4b/superbundlelayout/nhl_installpackage_00";
        fs::create_dir_all(r.update.join(pkg)).unwrap();
        fs::write(r.update.join(pkg).join("cas_01.cas"), b"").unwrap();
        fs::create_dir_all(r.game.join(pkg)).unwrap();
        fs::write(r.game.join(pkg).join("cas_02.cas"), b"").unwrap();
        assert_eq!(choose_cas(&r).unwrap(), 0x10003);
    }
}
