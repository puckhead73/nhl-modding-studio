//! Repeatable, hardened container extraction into the game installation (ONB-002 §3).
//!
//! This is the one service behind onboarding's "Extract game assets now?" and Settings → Tools.
//! It keeps the established mapping — every member lands beside the archive that holds it, under
//! the member's own relative path (`<archive dir>/<member path>`) — and adds what the previous
//! best-effort extractor (`crate::extract`) lacked:
//!
//! * **Preflight without writes** ([`preflight`]): every supported archive is parsed, every member
//!   is mapped to its destination, pre-existing files, cross-archive name collisions and the
//!   per-archive completion markers are accounted for, and the bytes still to write are estimated
//!   against the free space of the destination volume.
//! * **Nothing pre-existing is overwritten**: a destination that already holds the member's bytes
//!   is *retained*; one that holds different bytes is a *conflict* (left alone, reported). A member
//!   shipped loose in NHL07's vanilla boot set is preserved when both archive and loose-file
//!   hashes match the bundled reference; its packed copy does not need extraction.
//!   name claimed by two archives with no loose file to compare against is *blocked* for both —
//!   precedence between a base and a patch archive is not known here and is not guessed.
//! * **Containment**: member names are joined safely (no `..`, no absolute paths), the resolved
//!   parent must stay inside the archive's directory, and a reparse point on the way is refused.
//! * **Durable per-archive markers** (`<archive>.nhlms-extracted.json`): the archive's identity and
//!   each member's outcome, written after the archive is processed. Preflight reports an archive as
//!   complete when each member is verified vanilla or has a matching marker outcome and output
//!   length. Old vanilla-only conflicts resolve on rescan. A changed archive invalidates its marker.
//! * **Progress and cancellation**: the caller's flag is checked between members; a cancelled run
//!   keeps what it wrote (owned outputs only), records a partial marker, and a rerun resumes by
//!   retaining equal files.
//!
//! NHL 07/08 Xbox 360 and NHL 08 PS3 speech IDX/BIG pairs export encoded SNR/SNS
//! pairs to bank-specific sample folders, preserving EA-XMA and EALayer3 respectively.
//! Their completion identity also covers the companion IDX so changed offsets invalidate it.
//! Streaming: EB (`EB\0\3`) archives are read per entry; BIGF/BIG4 archives are buffered as the
//! existing reader requires (a limit carried over, not introduced).

use std::collections::{BTreeMap, BTreeSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Component, Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tdb_archive_big::eb::{self, EbHeader, Payload};
use tdb_archive_big::Archive;

use crate::extract::find_big_archives;
use crate::{DomainError, Platform, Result};

pub const MARKER_SUFFIX: &str = ".nhlms-extracted.json";
pub const MARKER_SCHEMA: &str = "nhlms-extraction/1";
const HEADER_HASH_LEN: u64 = 64 * 1024;

/// The archive identity a marker binds to (length, modification time, hash of the first 64 KiB).
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArchiveIdentity {
    pub len: u64,
    pub mtime_secs: u64,
    pub head_sha256: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub speech_index_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MarkerMember {
    pub name: String,
    /// `written` | `retained` | `preserved_vanilla` | `conflict` | `blocked` | `unsupported` | `unsafe` | `failed`.
    pub outcome: String,
    #[serde(default)]
    pub len: Option<u64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Marker {
    pub schema: String,
    pub archive: ArchiveIdentity,
    /// `true` when every member was written, retained, or preserved as a verified vanilla loose file.
    pub complete: bool,
    pub cancelled: bool,
    pub written_at: String,
    pub members: Vec<MarkerMember>,
}

/// One member as the preflight maps it.
#[derive(Clone, Debug)]
pub struct MemberPlan {
    pub name: String,
    pub dest: Option<PathBuf>,
    pub stored_len: u64,
    /// `absent` | `present` | `vanilla` | `collision` | `unsafe`.
    pub state: &'static str,
}

#[derive(Clone, Debug)]
pub struct ArchivePreflight {
    pub output_base: PathBuf,
    pub marker_archive: PathBuf,
    pub path: PathBuf,
    pub format: String,
    pub members: Vec<MemberPlan>,
    pub parse_error: Option<String>,
    /// `complete` | `partial` | `pending` | `unsupported` | `failed`.
    pub state: &'static str,
    pub note: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Collision {
    pub member: String,
    pub archives: Vec<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct Preflight {
    pub root: PathBuf,
    /// `not_applicable` | `complete` | `needed` | `partial` | `blocked`.
    pub state: &'static str,
    pub reason: String,
    pub archives: Vec<ArchivePreflight>,
    pub collisions: Vec<Collision>,
    pub members_total: u64,
    pub members_present: u64,
    pub members_to_write: u64,
    /// Stored (compressed) bytes still to write — a lower bound for chunkzip members.
    pub bytes_to_write: u64,
    pub free_bytes: Option<u64>,
}

/// Running counters, reported to the caller as the run proceeds.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    pub current_files_done: u64,
    pub current_files_total: u64,
    pub files_done: u64,
    pub files_total: u64,
    pub current: String,
    pub archives_done: u64,
    pub archives_total: u64,
    pub files_written: u64,
    pub files_retained: u64,
    pub files_conflicting: u64,
    pub files_blocked: u64,
    pub files_unsupported: u64,
    pub files_unsafe: u64,
    pub archives_failed: u64,
    pub bytes_written: u64,
    pub problems: Vec<String>,
    pub cancelled: bool,
}

fn io(path: &Path) -> impl Fn(std::io::Error) -> DomainError + '_ {
    move |source| DomainError::Io { path: path.display().to_string(), source }
}

pub fn marker_path(archive: &Path) -> PathBuf {
    let mut name = archive.file_name().unwrap_or_default().to_os_string();
    name.push(MARKER_SUFFIX);
    archive.with_file_name(name)
}

pub fn archive_identity(archive: &Path) -> Result<ArchiveIdentity> {
    let meta = std::fs::metadata(archive).map_err(io(archive))?;
    let mut f = std::fs::File::open(archive).map_err(io(archive))?;
    let mut buf = vec![0u8; HEADER_HASH_LEN.min(meta.len()) as usize];
    f.read_exact(&mut buf).map_err(io(archive))?;
    let mut h = Sha256::new();
    h.update(&buf);
    Ok(ArchiveIdentity {
        len: meta.len(),
        mtime_secs: meta.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0),
        head_sha256: format!("{:x}", h.finalize()),
        speech_index_sha256: if speech_bank_name(archive).is_some() {
            Some(crate::game_manifest::hash(speech_index(archive).map_err(DomainError::Extract)?.as_slice()).map_err(io(archive))?)
        } else { None },
    })
}

fn read_marker(archive: &Path) -> Option<Marker> {
    let text = std::fs::read_to_string(marker_path(archive)).ok()?;
    let m: Marker = serde_json::from_str(&text).ok()?;
    (m.schema == MARKER_SCHEMA).then_some(m)
}

fn write_marker(archive: &Path, marker: &Marker) -> Result<()> {
    let text = serde_json::to_string_pretty(marker).map_err(|e| DomainError::Validation(e.to_string()))?;
    crate::baselines::write_atomic(&marker_path(archive), text.as_bytes())
}

/// Join an archive member name onto `base`, refusing anything that would escape it.
fn safe_join(base: &Path, name: &str) -> Option<PathBuf> {
    let mut out = base.to_path_buf();
    let mut pushed = false;
    for raw in name.split(['/', '\\']) {
        if raw.is_empty() || raw == "." {
            continue;
        }
        match Path::new(raw).components().next() {
            Some(Component::Normal(c)) if raw != ".." => {
                out.push(c);
                pushed = true;
            }
            _ => return None,
        }
    }
    pushed.then_some(out)
}

/// The resolved destination must stay inside `base`, and no existing component between the two
/// may be a reparse point.
fn contained(base: &Path, dest: &Path) -> bool {
    let Ok(base_c) = std::fs::canonicalize(base) else { return false };
    let mut cur = dest.to_path_buf();
    let mut tail = Vec::new();
    loop {
        match std::fs::symlink_metadata(&cur) {
            Ok(m) => {
                if m.file_type().is_symlink() {
                    return false;
                }
                break;
            }
            Err(_) => {
                let Some(name) = cur.file_name() else { return false };
                tail.push(name.to_os_string());
                let Some(p) = cur.parent() else { return false };
                cur = p.to_path_buf();
            }
        }
    }
    let Ok(mut canon) = std::fs::canonicalize(&cur) else { return false };
    for t in tail.iter().rev() {
        canon.push(t);
    }
    crate::project_store::is_within(&canon, &base_c)
}

enum Entries {
    Bigf(Archive),
    Eb { file: std::fs::File, archive: eb::EbArchive },
    Speech { file: std::fs::File, bank: String, samples: Vec<crate::speech_legacy::Sample> },
}

fn speech_bank_name(path: &Path) -> Option<String> {
    let name = path.file_stem()?.to_str()?.to_lowercase();
    matches!(name.as_str(), "pbp_eng" | "rink").then_some(name)
}

// NHL08 ships these indexes inside data0.big. Resolve the same bytes before or after
// extraction, so archive ordering and a subsequent loose index do not invalidate completion.
fn speech_index(bank: &Path) -> std::result::Result<Vec<u8>, String> {
    let path = bank.with_extension("idx");
    const LIMIT: u64 = 32 * 1024 * 1024;
    match std::fs::metadata(&path) {
        Ok(md) => {
            if md.len() > LIMIT { return Err("speech index exceeds the 32 MiB limit".into()); }
            let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
            // BIG extraction preserves native RefPack wrappers. Interpret a loose index
            // exactly like its packed counterpart without changing the file on disk.
            if tdb_archive_codec::is_refpack(&bytes) {
                let header = tdb_archive_codec::refpack::parse_refpack_header(&bytes).ok_or("truncated compressed speech index")?;
                if header.uncompressed_len as u64 > LIMIT { return Err("decoded speech index exceeds the 32 MiB limit".into()); }
                return tdb_archive_codec::refpack_decompress(&bytes).map_err(|e| format!("compressed speech index: {e}"));
            }
            return Ok(bytes);
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.to_string()),
    }
    let speech = bank.parent().ok_or("speech bank has no parent")?;
    let audio = speech.parent().ok_or("speech bank has no audio folder")?;
    let root = audio.parent().ok_or("speech bank has no installation root")?;
    if speech.file_name().is_some_and(|n| n.eq_ignore_ascii_case("speech")) && audio.file_name().is_some_and(|n| n.eq_ignore_ascii_case("audio")) {
        let archive = root.join("data0.big");
        let member = format!("audio/speech/{}.idx", speech_bank_name(bank).ok_or("unknown speech bank")?);
        if crate::gen7_speech_preview::locate_member(&archive, &member).is_some_and(|(_, size)| size > LIMIT) {
            return Err("packed speech index exceeds the 32 MiB limit".into());
        }
        if let Some((bytes, _, _, _)) = crate::gen7_speech_preview::read_member_unwrapped(&archive, &member)? {
            if bytes.len() as u64 > LIMIT { return Err("speech index exceeds the 32 MiB limit".into()); }
            return Ok(bytes);
        }
    }
    Err(format!("speech bank requires its matching .idx file: {}", path.display()))
}

fn open_entries(archive_path: &Path) -> std::result::Result<(String, Entries), String> {
    let mut file = std::fs::File::open(archive_path).map_err(|e| e.to_string())?;
    let mut magic = [0u8; 4];
    file.read_exact(&mut magic).map_err(|e| e.to_string())?;
    if &magic == eb::MAGIC_EB {
        let mut header_buf = vec![0u8; eb::HEADER_LEN];
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        file.read_exact(&mut header_buf).map_err(|e| e.to_string())?;
        let header = EbHeader::parse(&header_buf).map_err(|e| e.to_string())?;
        let mut table_buf = vec![0u8; header.table_end()];
        file.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
        file.read_exact(&mut table_buf).map_err(|e| e.to_string())?;
        let archive = eb::EbArchive::parse(&table_buf).map_err(|e| e.to_string())?;
        Ok(("EB".into(), Entries::Eb { file, archive }))
    } else if tdb_archive_big::KNOWN_MAGICS.iter().any(|m| *m == &magic) {
        let bytes = std::fs::read(archive_path).map_err(|e| e.to_string())?;
        let archive = Archive::parse(bytes).map_err(|e| e.to_string())?;
        Ok((String::from_utf8_lossy(&magic).into_owned(), Entries::Bigf(archive)))
    } else if let Some(bank) = speech_bank_name(archive_path) {
        let index = speech_index(archive_path)?;
        let samples = crate::speech_legacy::parse(&index, file.metadata().map_err(|e|e.to_string())?.len())?;
        Ok(("IDX/BIG speech".into(), Entries::Speech { file, bank, samples }))
    } else {
        Err(format!("not an EA archive this build reads (magic {:?})", String::from_utf8_lossy(&magic)))
    }
}

fn entry_list(e: &Entries) -> Vec<(String, u64)> {
    match e {
        Entries::Bigf(a) => a.entries.iter().map(|x| (x.name.clone(), x.size as u64)).collect(),
        Entries::Eb { archive, .. } => archive.entries.iter().map(|x| (x.path.clone(), x.size as u64)).collect(),
        Entries::Speech { bank, samples, .. } => samples.iter().flat_map(|s| {
            let name = s.name(bank);
            [(format!("{name}.snr"),8), (format!("{name}.sns"),s.limit-s.offset)]
        }).collect(),
    }
}

/// Preflight needs names and sizes, never payloads. Read only the BIG directory
/// instead of loading multi-gigabyte containers before onboarding can continue.
fn preflight_entries(path: &Path) -> std::result::Result<(String, Vec<(String, u64)>), String> {
    let mut file = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut header = [0u8; 16];
    if file.read_exact(&mut header).is_ok()
        && tdb_archive_big::KNOWN_MAGICS.iter().any(|m| **m == header[..4]) {
        let len = file.metadata().map_err(|e| e.to_string())?.len();
        // Some BIG4 headers understate the directory length. Walk the declared
        // entries instead of trusting data_offset as a table boundary.
        let count = u32::from_be_bytes(header[8..12].try_into().unwrap());
        if count as u64 > (len - 16) / 9 { return Err("invalid BIG file count".into()); }
        let mut table = header.to_vec();
        let mut reader = std::io::BufReader::new(file);
        let padded = matches!(&header[..4], b"BIG5" | b"BIG6" | b"BIG7" | b"BIG8" | b"BIG9");
        for _ in 0..count {
            let mut fixed = [0u8; 9];
            let width = if padded { 9 } else { 8 };
            reader.read_exact(&mut fixed[..width]).map_err(|e| e.to_string())?;
            table.extend_from_slice(&fixed[..width]);
            if std::io::BufRead::read_until(&mut reader, 0, &mut table).map_err(|e| e.to_string())? == 0
                || table.last() != Some(&0) { return Err("unterminated BIG member name".into()); }
        }
        let archive = Archive::parse(table).map_err(|e| e.to_string())?;
        if archive.entries.iter().any(|e| e.offset as u64 + e.size as u64 > len) {
            return Err("BIG member extends beyond the archive".into());
        }
        Ok((archive.magic_str().into(), archive.entries.into_iter().map(|e| (e.name, e.size as u64)).collect()))
    } else {
        let (format, entries) = open_entries(path)?;
        Ok((format, entry_list(&entries)))
    }
}

fn read_entry(e: &mut Entries, idx: usize) -> std::result::Result<Vec<u8>, String> {
    match e {
        Entries::Bigf(a) => a.read_entry_at(idx).map_err(|x| x.to_string()),
        Entries::Speech { file, samples, .. } => {
            let sample = &samples[idx/2];
            if idx%2==0 { Ok(sample.snr.to_vec()) } else { crate::speech_legacy::read_sample(file, sample) }
        }
        Entries::Eb { file, archive } => {
            let entry = &archive.entries[idx];
            let mut raw = vec![0u8; entry.size as usize];
            file.seek(SeekFrom::Start(entry.offset)).map_err(|x| x.to_string())?;
            file.read_exact(&mut raw).map_err(|x| x.to_string())?;
            match eb::decode_payload(raw) {
                Ok(Payload::Data(d)) => Ok(d),
                Ok(Payload::UnsupportedLzx) => Err("chunklzx payload this build does not decode".into()),
                Err(x) => Err(x.to_string()),
            }
        }
    }
}

fn fold(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\");
    if cfg!(windows) { s.to_lowercase() } else { s }
}

/// An extracted .big may be a texture, screen, or nested container. Its name in
/// an outer archive makes it an output of this operation, not another input.
/// Derive ownership from actual archive tables, so this also works without markers.
/// Cycles without an independent outer archive stay visible rather than disappearing.
fn outer_archives(plans: Vec<ArchivePreflight>) -> Vec<ArchivePreflight> {
    let paths: BTreeMap<String, usize> = plans.iter().enumerate().map(|(i, a)| (fold(&a.path), i)).collect();
    let mut children = vec![Vec::new(); plans.len()];
    let mut owned = BTreeSet::new();
    for (i, a) in plans.iter().enumerate() {
        for m in &a.members {
            if let Some(child) = m.dest.as_ref().and_then(|p| paths.get(&fold(p))) {
                if *child != i {
                    children[i].push(*child);
                    owned.insert(*child);
                }
            }
        }
    }
    let mut stack: Vec<usize> = (0..plans.len()).filter(|i| !owned.contains(i)).collect();
    let mut outputs = BTreeSet::new();
    while let Some(i) = stack.pop() {
        for &child in &children[i] {
            if outputs.insert(child) { stack.push(child); }
        }
    }
    plans.into_iter().enumerate().filter_map(|(i, a)| (!outputs.contains(&i)).then_some(a)).collect()
}

/// Read-only preflight of `root`.
pub fn preflight(root: &Path, platform: Platform) -> Preflight {
    let manifest = matches!(platform, Platform::Xbox).then(|| {
        serde_json::from_str::<crate::game_manifest::Manifest>(include_str!("../../../anchors/xbox360/nhl07/game-files.json")).expect("bundled NHL07 manifest")
    });
    preflight_with_manifest(root, manifest.as_ref())
}

// NHL07 ships some boot.big members separately, with different, legitimate vanilla bytes.
// Scope this exception to the verified archive AND verified loose files, never filenames alone.
fn preserve_vanilla_boot_files(root: &Path, ap: &mut ArchivePreflight, manifest: &crate::game_manifest::Manifest) {
    if ap.path != root.join("boot.big") { return; }
    let matches_reference = |path: &Path, entry: &crate::game_manifest::Entry| {
        contained(root, path)
            && std::fs::metadata(path).is_ok_and(|m| m.is_file() && m.len() == entry.size)
            && std::fs::File::open(path).ok().and_then(|f| crate::game_manifest::hash(f).ok()).is_some_and(|h| h == entry.sha256)
    };
    let Some(archive) = manifest.files.iter().find(|e| e.path == "boot.big" && e.kind == "container") else { return; };
    if !matches_reference(&ap.path, archive) { return; }
    for member in &mut ap.members {
        if member.state != "present" { continue; }
        let Some(dest) = &member.dest else { continue; };
        let relative = dest.strip_prefix(root).unwrap_or(dest).to_string_lossy().replace('\\', "/");
        if manifest.files.iter().any(|e| e.kind == "loose" && e.path.eq_ignore_ascii_case(&relative) && matches_reference(dest, e)) {
            member.state = "vanilla";
        }
    }
}

fn preflight_with_manifest(root: &Path, manifest: Option<&crate::game_manifest::Manifest>) -> Preflight {
    let mut out = Preflight {
        root: root.to_path_buf(),
        state: "needed",
        reason: String::new(),
        archives: Vec::new(),
        collisions: Vec::new(),
        members_total: 0,
        members_present: 0,
        members_to_write: 0,
        bytes_to_write: 0,
        free_bytes: crate::disk::free_space(root),
    };
    // Determine support from the containers present, not the console label.
    // Early PS3 titles carry BIG archives just like their Xbox360 counterparts.
    let mut archives = find_big_archives(root);
    // Parents first: a container's nested outputs do not need opening again.
    archives.sort_by_key(|p| p.components().count());
    if archives.is_empty() {
        out.state = "not_applicable";
        out.reason = format!("no .big archives under {}", root.display());
        return out;
    }
    // Pass 1: parse every archive and map its members.
    let mut nested_outputs = BTreeSet::new();
    for path in &archives {
        if nested_outputs.contains(&fold(path)) { continue; }
        let base = path.parent().unwrap_or(root).to_path_buf();
        let mut ap = ArchivePreflight { output_base: base.clone(), marker_archive: path.clone(), path: path.clone(), format: String::new(), members: Vec::new(), parse_error: None, state: "pending", note: None };
        match preflight_entries(path) {
            Err(e) => {
                ap.parse_error = Some(e.clone());
                ap.state = if e.starts_with("not an EA archive") { "unsupported" } else { "failed" };
                ap.note = Some(e);
            }
            Ok((format, entries)) => {
                ap.format = format;
                for (name, stored_len) in entries {
                    let plan = match safe_join(&base, &name) {
                        None => MemberPlan { name: name.clone(), dest: None, stored_len, state: "unsafe" },
                        Some(dest) => {
                            // Only skip deeper outputs. Safe relative member paths
                            // cannot point back up to their owner and create a cycle.
                            if dest.components().count() > path.components().count()
                                && dest.extension().is_some_and(|e| e.eq_ignore_ascii_case("big")) {
                                nested_outputs.insert(fold(&dest));
                            }
                            let state = if dest.exists() { "present" } else { "absent" };
                            MemberPlan { name: name.clone(), dest: Some(dest), stored_len, state }
                        }
                    };
                    ap.members.push(plan);
                }
            }
        }
        if let Some(manifest) = manifest { preserve_vanilla_boot_files(root, &mut ap, manifest); }
        out.archives.push(ap);
    }
    out.archives = outer_archives(out.archives);
    // Preserve extraction order: speech banks may resolve an index still packed
    // in a later archive; directory-scan ordering must not change that workflow.
    out.archives.sort_by(|a, b| a.path.cmp(&b.path));
    let mut claims: BTreeMap<String, Vec<(usize, usize)>> = BTreeMap::new();
    for (ai, ap) in out.archives.iter().enumerate() {
        for (mi, m) in ap.members.iter().enumerate() {
            if let Some(dest) = &m.dest { claims.entry(fold(dest)).or_default().push((ai, mi)); }
        }
    }
    // Pass 2: collisions — a destination claimed by two archives with no loose file to compare.
    for (_, claimers) in claims.iter().filter(|(_, v)| v.len() > 1) {
        let (a0, m0) = claimers[0];
        let member = out.archives[a0].members[m0].name.clone();
        let present = matches!(out.archives[a0].members[m0].state, "present" | "vanilla");
        if !present {
            for (ai, mi) in claimers {
                out.archives[*ai].members[*mi].state = "collision";
            }
        }
        out.collisions.push(Collision { member, archives: claimers.iter().map(|(ai, _)| out.archives[*ai].path.clone()).collect() });
    }
    // Pass 3: markers and totals.
    let mut any_blocked = false;
    for ap in out.archives.iter_mut() {
        if ap.parse_error.is_some() {
            continue;
        }
        let marker = read_marker(&ap.path);
        let identity = archive_identity(&ap.path).ok();
        let marker_ok = match (&marker, &identity) {
            (Some(m), Some(id)) if m.archive == *id => Some(m),
            (Some(_), _) => {
                ap.note = Some("the archive changed since it was extracted; its marker no longer applies".into());
                None
            }
            _ => None,
        };
        let present = ap.members.iter().filter(|m| matches!(m.state, "present" | "vanilla")).count() as u64;
        let total = ap.members.len() as u64;
        out.members_total += total;
        out.members_present += present;
        for m in &ap.members {
            match m.state {
                "absent" => {
                    out.members_to_write += 1;
                    out.bytes_to_write += m.stored_len;
                }
                "collision" | "unsafe" => any_blocked = true,
                _ => {}
            }
        }
        let recorded_all_exist = marker_ok.is_some_and(|m| {
            // Speech banks have tens of thousands of outputs: index marker records once,
            // rather than scanning the whole marker again for each output on every reopen.
            let recorded: BTreeMap<_, _> = m.members.iter().map(|x| (x.name.as_str(), x)).collect();
            !m.cancelled && ap.members.iter().all(|plan| {
                plan.state == "vanilla" || recorded.get(plan.name.as_str()).is_some_and(|x| {
                    matches!(x.outcome.as_str(), "written" | "retained")
                        && plan.dest.as_ref().is_some_and(|d| std::fs::metadata(d).is_ok_and(|md| md.is_file() && x.len == Some(md.len())))
                })
            })
        });
        ap.state = if recorded_all_exist {
            "complete"
        } else if present == total && total > 0 {
            // Every destination exists, but without a complete marker nothing says the bytes are the
            // archive's: a conflict or a hand-placed file looks the same from here.
            if ap.members.iter().all(|m| m.state == "vanilla") { "complete" } else { "partial" }
        } else if present > 0 {
            "partial"
        } else {
            "pending"
        };
        if ap.state != "complete" && ap.note.is_none() && marker_ok.is_none() && marker.is_some() {
            ap.note = Some("a previous run did not complete this archive".into());
        }
    }
    let supported: Vec<&ArchivePreflight> = out.archives.iter().filter(|a| a.parse_error.is_none()).collect();
    let all_complete = !supported.is_empty() && supported.iter().all(|a| a.state == "complete");
    let unsupported = out.archives.iter().filter(|a| a.parse_error.is_some()).count();
    if all_complete && out.members_to_write == 0 {
        out.state = if unsupported == 0 { "complete" } else { "partial" };
        out.reason = format!("{} archive(s) extracted and verified by their markers{}", supported.len(), if unsupported > 0 { format!("; {unsupported} archive(s) could not be parsed and stay unextracted") } else { String::new() });
    } else if out.members_to_write == 0 && any_blocked {
        out.state = "blocked";
        out.reason = "every remaining member is blocked (name claimed by more than one archive, or an unsafe name); nothing can be written".into();
    } else if out.members_present > 0 || supported.iter().any(|a| a.state == "complete") {
        out.state = "partial";
        out.reason = format!("{} of {} container member paths already exist; {} still to write. Existing files may have shipped loose on the disc.", out.members_present, out.members_total, out.members_to_write);
    } else {
        out.state = "needed";
        out.reason = format!("{} members in {} archive(s) are not extracted", out.members_total, supported.len());
    }
    if let Some(free) = out.free_bytes {
        if out.bytes_to_write > free {
            out.state = if out.state == "complete" { "complete" } else { "blocked" };
            out.reason = format!("at least {} MiB to write but only {} MiB free on the destination volume", out.bytes_to_write / (1024 * 1024), free / (1024 * 1024));
        }
    }
    out
}

/// Extract every supported archive under `root` with the rules in the module docs. `report` is
/// called after every member; `cancel` is honoured between members. The returned progress is the
/// final tally (`cancelled` set when the flag stopped it).
pub fn extract(root: &Path, platform: Platform, cancel: &AtomicBool, report: &mut dyn FnMut(&Progress)) -> Result<Progress> {
    let pre = preflight(root, platform);
    extract_preflight(pre, cancel, report)
}

/// Map a read-only source scan onto an optional separate assets tree.
pub fn preflight_to(root: &Path, destination: &Path, platform: Platform) -> Preflight {
    let mut pre = preflight(root, platform);
    if root == destination { return pre; }
    pre.root = destination.to_path_buf();
    pre.members_present = 0; pre.members_to_write = 0; pre.bytes_to_write = 0;
    pre.free_bytes = crate::disk::free_space(destination);
    for ap in &mut pre.archives {
        ap.marker_archive = destination.join(ap.path.strip_prefix(root).unwrap_or(&ap.path));
        ap.output_base = ap.marker_archive.parent().unwrap_or(destination).to_path_buf();
        let marker = read_marker(&ap.marker_archive).filter(|m| archive_identity(&ap.path).is_ok_and(|id| id == m.archive));
        for m in &mut ap.members {
            if let Some(old) = &m.dest {
                let dest = destination.join(old.strip_prefix(root).unwrap_or(old));
                m.state = if dest.exists() { "present" } else if m.state == "collision" { "collision" } else { "absent" };
                if m.state == "present" { pre.members_present += 1; }
                if m.state == "absent" { pre.members_to_write += 1; pre.bytes_to_write += m.stored_len; }
                m.dest = Some(dest);
            }
        }
        if ap.parse_error.is_none() {
            let recorded: BTreeMap<_, _> = marker.as_ref().map(|m| m.members.iter().map(|x| (x.name.as_str(), x)).collect()).unwrap_or_default();
            ap.state = if marker.as_ref().is_some_and(|m| m.complete && !m.cancelled) && ap.members.iter().all(|m| recorded.get(m.name.as_str()).is_some_and(|r| m.dest.as_ref().is_some_and(|p| std::fs::metadata(p).is_ok_and(|md| md.is_file() && Some(md.len()) == r.len)))) { "complete" } else { "pending" };
        }
    }
    if !pre.archives.is_empty() { pre.state = if pre.archives.iter().all(|a| a.state == "complete") { "complete" } else { "needed" }; }
    pre
}

/// A separate assets tree retains the source layout, including files shipped loose.
/// Existing destination files are never overwritten; conflicts remain visible.
pub fn extract_to(root: &Path, destination: &Path, platform: Platform, cancel: &AtomicBool, report: &mut dyn FnMut(&Progress)) -> Result<Progress> {
    if root == destination { return extract(root, platform, cancel, report); }
    if destination.starts_with(root) || root.starts_with(destination) {
        return Err(DomainError::Validation("Choose an assets folder outside the game source folder".into()));
    }
    std::fs::create_dir_all(destination).map_err(io(destination))?;
    let mut pre = preflight(root, platform);
    let archives: BTreeSet<_> = pre.archives.iter().map(|a| a.path.clone()).collect();
    let mut folders = vec![root.to_path_buf()];
    while let Some(folder) = folders.pop() {
        for entry in std::fs::read_dir(&folder).map_err(io(&folder))? {
            if cancel.load(Ordering::Relaxed) { return Ok(Progress { cancelled: true, ..Default::default() }); }
            let entry = entry.map_err(io(&folder))?;
            let path = entry.path();
            let ty = entry.file_type().map_err(io(&path))?;
            if ty.is_symlink() { continue; }
            if ty.is_dir() { folders.push(path); continue; }
            if !ty.is_file() || archives.contains(&path) || path.to_string_lossy().ends_with(MARKER_SUFFIX) { continue; }
            let dest = destination.join(path.strip_prefix(root).map_err(|e| DomainError::Validation(e.to_string()))?);
            if !contained(destination, &dest) { return Err(DomainError::Validation("Assets destination contains an unsafe link".into())); }
            if !dest.exists() {
                if let Some(parent) = dest.parent() { std::fs::create_dir_all(parent).map_err(io(parent))?; }
                // Publish only complete loose files, so an interrupted copy can be retried.
                let stamp = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_nanos();
                let temporary = dest.with_file_name(format!(".nhlms-copy-{}-{stamp}.tmp", std::process::id()));
                let result = (|| -> std::io::Result<()> {
                    let mut input = std::fs::File::open(&path)?;
                    let mut output = std::fs::OpenOptions::new().write(true).create_new(true).open(&temporary)?;
                    std::io::copy(&mut input, &mut output)?; output.sync_all()?; drop(output);
                    if dest.exists() { return Err(std::io::Error::new(std::io::ErrorKind::AlreadyExists, "Assets destination changed during copying")); }
                    std::fs::rename(&temporary, &dest)
                })();
                if let Err(error) = result { let _ = std::fs::remove_file(&temporary); return Err(io(&dest)(error)); }
            }
        }
    }
    pre.root = destination.to_path_buf();
    pre.bytes_to_write = 0;
    pre.members_to_write = 0;
    pre.free_bytes = crate::disk::free_space(destination);
    for ap in &mut pre.archives {
        ap.marker_archive = destination.join(ap.path.strip_prefix(root).map_err(|e| DomainError::Validation(e.to_string()))?);
        ap.output_base = ap.marker_archive.parent().unwrap_or(destination).to_path_buf();
        if ap.parse_error.is_none() { ap.state = "pending"; }
        for m in &mut ap.members {
            if let Some(old) = &m.dest {
                let dest = destination.join(old.strip_prefix(root).map_err(|e| DomainError::Validation(e.to_string()))?);
                m.state = if m.state == "vanilla" && std::fs::read(old).ok() == std::fs::read(&dest).ok() { "vanilla" }
                    else if dest.exists() { "present" } else if m.state == "collision" { "collision" } else { "absent" };
                if m.state == "absent" { pre.bytes_to_write += m.stored_len; pre.members_to_write += 1; }
                m.dest = Some(dest);
            }
        }
    }
    pre.state = "needed";
    extract_preflight(pre, cancel, report)
}

fn extract_preflight(pre: Preflight, cancel: &AtomicBool, report: &mut dyn FnMut(&Progress)) -> Result<Progress> {
    if pre.state == "not_applicable" {
        return Err(DomainError::Extract(pre.reason));
    }
    if pre.state == "blocked" && pre.members_to_write == 0 {
        return Err(DomainError::Extract(pre.reason));
    }
    if let Some(free) = pre.free_bytes {
        if pre.bytes_to_write > free {
            return Err(DomainError::Extract(format!("at least {} MiB to write but only {} MiB free; nothing was written", pre.bytes_to_write / (1024 * 1024), free / (1024 * 1024))));
        }
    }
    let mut p = Progress { files_total: pre.members_total, archives_total: pre.archives.iter().filter(|a| a.parse_error.is_none()).count() as u64, ..Default::default() };
    for ap in &pre.archives {
        if cancel.load(Ordering::Relaxed) {
            p.cancelled = true;
            break;
        }
        if let Some(e) = &ap.parse_error {
            p.archives_failed += 1;
            p.problems.push(format!("{}: {e}", ap.path.display()));
            continue;
        }
        p.current = ap.path.display().to_string();
        p.current_files_done = 0;
        p.current_files_total = ap.members.len() as u64;
        report(&p);
        if ap.state == "complete" {
            p.current_files_done = p.current_files_total;
            p.files_done += p.current_files_total;
            p.files_retained += ap.members.len() as u64;
            p.archives_done += 1;
            p.current = ap.path.display().to_string();
            report(&p);
            continue;
        }
        let base = ap.output_base.clone();
        std::fs::create_dir_all(&base).map_err(io(&base))?;
        p.current = ap.path.display().to_string();
        let (_, mut entries) = match open_entries(&ap.path) {
            Ok(x) => x,
            Err(e) => {
                p.archives_failed += 1;
                p.problems.push(format!("{}: {e}", ap.path.display()));
                continue;
            }
        };
        let identity = archive_identity(&ap.path)?;
        let mut members: Vec<MarkerMember> = Vec::new();
        let mut complete = true;
        let mut cancelled_here = false;
        for (mi, m) in ap.members.iter().enumerate() {
            if cancel.load(Ordering::Relaxed) {
                cancelled_here = true;
                break;
            }
            let mut rec = MarkerMember { name: m.name.clone(), outcome: String::new(), len: None, note: None };
            match m.state {
                "vanilla" => {
                    p.files_retained += 1;
                    rec.outcome = "preserved_vanilla".into();
                    rec.len = m.dest.as_ref().and_then(|d| std::fs::metadata(d).ok()).map(|md| md.len());
                    rec.note = Some("original loose disc file; archive copy is not needed".into());
                }
                "unsafe" => {
                    p.files_unsafe += 1;
                    complete = false;
                    rec.outcome = "unsafe".into();
                    rec.note = Some("the member name would escape the archive's folder".into());
                }
                "collision" => {
                    p.files_blocked += 1;
                    complete = false;
                    rec.outcome = "blocked".into();
                    rec.note = Some("claimed by more than one archive; precedence is not known and is not guessed".into());
                }
                _ => {
                    let dest = m.dest.clone().expect("mapped");
                    if !contained(&base, &dest) {
                        p.files_unsafe += 1;
                        complete = false;
                        rec.outcome = "unsafe".into();
                        rec.note = Some("the destination resolves outside the archive's folder or through a reparse point".into());
                    } else {
                        match read_entry(&mut entries, mi) {
                            Err(e) => {
                                p.files_unsupported += 1;
                                complete = false;
                                rec.outcome = "unsupported".into();
                                rec.note = Some(e);
                            }
                            Ok(data) => match std::fs::read(&dest) {
                                Ok(existing) => {
                                    if existing == data {
                                        p.files_retained += 1;
                                        rec.outcome = "retained".into();
                                        rec.len = Some(data.len() as u64);
                                    } else {
                                        p.files_conflicting += 1;
                                        complete = false;
                                        rec.outcome = "conflict".into();
                                        rec.len = Some(existing.len() as u64);
                                        rec.note = Some(format!("{} already exists with different bytes; it was not overwritten", dest.display()));
                                        p.problems.push(format!("{}: {} exists with different bytes and was left as it is", ap.path.display(), dest.display()));
                                    }
                                }
                                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                                    match crate::baselines::write_atomic(&dest, &data) {
                                        Ok(()) => {
                                            p.files_written += 1;
                                            p.bytes_written += data.len() as u64;
                                            rec.outcome = "written".into();
                                            rec.len = Some(data.len() as u64);
                                        }
                                        Err(e) => {
                                            complete = false;
                                            rec.outcome = "failed".into();
                                            rec.note = Some(e.to_string());
                                            p.problems.push(format!("{}: {e}", dest.display()));
                                            // A write failure (space, permissions) stops the run: nothing else is
                                            // likely to succeed and the user should see the cause now.
                                            members.push(rec);
                                            let marker = Marker { schema: MARKER_SCHEMA.into(), archive: identity.clone(), complete: false, cancelled: false, written_at: crate::project_store::now_iso(), members: members.clone() };
                                            let _ = write_marker(&ap.marker_archive, &marker);
                                            return Err(DomainError::Extract(format!("writing {} failed: {e}; the run stopped and what was written stays", dest.display())));
                                        }
                                    }
                                }
                                Err(e) => {
                                    complete = false;
                                    p.files_blocked += 1;
                                    rec.outcome = "failed".into();
                                    rec.note = Some(e.to_string());
                                    p.problems.push(format!("{}: cannot be read: {e}", dest.display()));
                                }
                            },
                        }
                    }
                }
            }
            members.push(rec);
            p.current_files_done += 1;
            p.files_done += 1;
            report(&p);
        }
        if matches!(&entries, Entries::Speech { .. }) && archive_identity(&ap.path)? != identity {
            return Err(DomainError::Extract(format!("{} or its speech index changed during extraction; extracted files remain but completion was not recorded", ap.path.display())));
        }
        let marker = Marker { schema: MARKER_SCHEMA.into(), archive: identity, complete: complete && !cancelled_here, cancelled: cancelled_here, written_at: crate::project_store::now_iso(), members };
        write_marker(&ap.marker_archive, &marker)?;
        if cancelled_here {
            p.cancelled = true;
            break;
        }
        p.archives_done += 1;
        report(&p);
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tdb_archive_big::ArchiveBuilder;

    #[test]
    fn speech_banks_extract_pairs_and_bind_completion_to_the_index() {
        let t=Tmp::new("speech");
        let (idx,bank)=crate::speech_legacy::tests::fixture();
        fs::write(t.0.join("pbp_eng.idx"),&idx).unwrap();
        fs::write(t.0.join("pbp_eng.big"),&bank).unwrap();
        let before=preflight(&t.0,Platform::Xbox);
        assert_eq!(before.members_total,2);
        assert_eq!(before.archives[0].format,"IDX/BIG speech");
        let result=run(&t.0);
        assert_eq!((result.archives_failed,result.files_written,result.files_unsupported),(0,2,0));
        assert_eq!(preflight(&t.0,Platform::Xbox).state,"complete");
        assert_eq!(run(&t.0).files_written,0);
        let mut changed=idx.clone();changed[71]=1;
        fs::write(t.0.join("pbp_eng.idx"),changed).unwrap();
        assert_ne!(preflight(&t.0,Platform::Xbox).state,"complete","index changes invalidate completion");
        let output=t.0.join(crate::speech_legacy::parse(&idx,64).unwrap()[0].name("pbp_eng")+".sns");
        fs::write(&output,b"KEEP-MOD").unwrap();
        assert_eq!(run(&t.0).files_conflicting,1);
        assert_eq!(fs::read(output).unwrap(),b"KEEP-MOD");
        assert_eq!(fs::read(t.0.join("pbp_eng.big")).unwrap(),bank);
        fs::remove_file(t.0.join("pbp_eng.idx")).unwrap();
        assert!(preflight(&t.0,Platform::Xbox).archives[0].parse_error.as_ref().unwrap().contains("matching .idx"));
    }

    #[test]
    fn changed_speech_index_during_extraction_never_records_completion() {
        let t=Tmp::new("speech-changed");
        let (mut idx,bank)=crate::speech_legacy::tests::fixture();
        fs::write(t.0.join("rink.idx"),&idx).unwrap();
        fs::write(t.0.join("rink.big"),bank).unwrap();
        let mut changed=false;
        let result=extract(&t.0,Platform::Xbox,&AtomicBool::new(false),&mut |p| {
            if p.files_written>0 && !changed {
                idx[71]=3;
                fs::write(t.0.join("rink.idx"),&idx).unwrap();
                changed=true;
            }
        });
        assert!(result.unwrap_err().to_string().contains("changed during extraction"));
        assert!(!marker_path(&t.0.join("rink.big")).exists());
    }

    #[test]
    fn packed_speech_index_works_before_extraction_and_keeps_identity_when_unpacked() {
        let t = Tmp::new("packed-speech-index");
        let (idx, bank) = crate::speech_legacy::tests::fixture();
        let compressed = tdb_archive_codec::refpack_compress(&idx);
        let (archive, bh) = tdb_archive_big::big4::synth::build(&[("audio/speech/pbp_eng.idx", &compressed, idx.len() as u32)], &[]);
        fs::write(t.0.join("data0.big"), archive).unwrap();
        fs::write(t.0.join("data0.bh"), bh).unwrap();
        let speech = t.0.join("audio/speech");
        fs::create_dir_all(&speech).unwrap();
        let path = speech.join("pbp_eng.big");
        fs::write(&path, bank).unwrap();
        let before = archive_identity(&path).unwrap();
        assert_eq!(speech_index(&path).unwrap(), idx);
        assert!(preflight(&t.0, Platform::Xbox).archives.iter().all(|a| a.parse_error.is_none()), "{:?}", preflight(&t.0, Platform::Xbox));
        let result = run(&t.0);
        assert_eq!((result.files_written, result.archives_failed, result.files_unsupported), (3, 0, 0));
        assert_eq!(fs::read(speech.join("pbp_eng.idx")).unwrap(), compressed);
        assert_eq!(speech_index(&path).unwrap(), idx);
        assert_eq!(archive_identity(&path).unwrap(), before);
        assert_eq!(preflight(&t.0, Platform::Xbox).state, "complete");
        assert_eq!(run(&t.0).files_written, 0);
        fs::write(speech.join("pbp_eng.idx"), b"invalid edited index").unwrap();
        let pre = preflight(&t.0, Platform::Xbox);
        assert!(pre.archives.iter().find(|a| a.path == path).unwrap().parse_error.is_some(), "do not silently replace an invalid loose index with its packed original");
    }

    struct Tmp(PathBuf);
    impl Tmp {
        fn new(tag: &str) -> Self {
            let d = std::env::temp_dir().join(format!("nhlms-xsvc-{tag}-{}", std::process::id()));
            let _ = fs::remove_dir_all(&d);
            fs::create_dir_all(&d).unwrap();
            Tmp(d)
        }
    }
    impl Drop for Tmp {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn write_big(path: &Path, entries: &[(&str, &[u8])]) {
        let mut b = ArchiveBuilder::new();
        for (n, body) in entries {
            b.add_entry(*n, body.to_vec());
        }
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, b.build().unwrap()).unwrap();
    }

    fn run(root: &Path) -> Progress {
        let cancel = AtomicBool::new(false);
        extract(root, Platform::Xbox, &cancel, &mut |_| {}).unwrap()
    }

    #[test]
    fn separate_destination_progress_resume_and_source_preservation() {
        let t = Tmp::new("separate-progress");
        let source = t.0.join("source"); let dest = t.0.join("assets");
        write_big(&source.join("data.big"), &[("rendering/a.bin", b"alpha"), ("rendering/b.bin", b"bravo")]);
        fs::write(source.join("loose.bin"), b"loose").unwrap();
        let cancel = AtomicBool::new(false);
        let first = extract_to(&source, &dest, Platform::Xbox, &cancel, &mut |p| { if p.files_done == 1 { cancel.store(true, Ordering::Relaxed); } }).unwrap();
        assert!(first.cancelled); assert_eq!(first.files_total, 2); assert_eq!(first.current_files_done, 1);
        assert!(!source.join("rendering").exists()); assert_eq!(fs::read(dest.join("loose.bin")).unwrap(), b"loose");
        let done = extract_to(&source, &dest, Platform::Xbox, &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!((done.files_done, done.files_total, done.current_files_done), (2, 2, 2));
        assert_eq!(preflight_to(&source, &dest, Platform::Xbox).state, "complete");
        fs::write(dest.join("rendering/a.bin"), b"modded").unwrap();
        let conflicts = extract_to(&source, &dest, Platform::Xbox, &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(conflicts.files_conflicting, 1); assert_eq!(fs::read(dest.join("rendering/a.bin")).unwrap(), b"modded");
        assert_ne!(preflight_to(&source, &dest, Platform::Xbox).state, "complete");
    }

    #[test]
    #[ignore = "requires NHL08_INSTALL; original files read-only, exports into owned target directory"]
    fn nhl08_packed_speech_indexes_validate_every_stream() {
        let root = PathBuf::from(std::env::var("NHL08_INSTALL").unwrap());
        let pre = preflight(&root, Platform::Xbox);
        for a in &pre.archives { assert!(a.parse_error.is_none(), "{}: {:?}", a.path.display(), a.parse_error); }
        assert_eq!(pre.archives.len(), 8);
        let owned = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/nhl08-speech-qa").join(crate::project_store::now_iso().replace([':', '.'], "-"));
        fs::create_dir_all(&owned).unwrap();
        let mut sources = Vec::new();
        for bank in ["pbp_eng", "rink"] {
            let path = root.join(format!("audio/speech/{bank}.big"));
            let index = speech_index(&path).unwrap();
            sources.push((path.clone(), crate::game_manifest::hash(fs::File::open(&path).unwrap()).unwrap()));
            fs::copy(&path, owned.join(format!("{bank}.big"))).unwrap();
            fs::write(owned.join(format!("{bank}.idx")), &index).unwrap();
            let samples = crate::speech_legacy::parse(&index, std::fs::metadata(&path).unwrap().len()).unwrap();
            let mut file = std::fs::File::open(&path).unwrap();
            for sample in &samples {
                crate::speech_legacy::read_sample(&mut file, sample).unwrap_or_else(|e| panic!("{bank} {}: {e}", sample.name(bank)));
            }
            println!("{bank}: {} streams validated", samples.len());
        }
        let result = run(&owned);
        assert_eq!((result.files_written, result.files_conflicting, result.files_unsupported, result.archives_failed), (76426, 0, 0, 0));
        for bank in ["pbp_eng", "rink"] {
            let path = root.join(format!("audio/speech/{bank}.big"));
            let samples = crate::speech_legacy::parse(&speech_index(&path).unwrap(), fs::metadata(&path).unwrap().len()).unwrap();
            let mut file = fs::File::open(&path).unwrap();
            for sample in samples {
                let name = sample.name(bank);
                assert_eq!(fs::read(owned.join(format!("{name}.snr"))).unwrap(), sample.snr);
                assert_eq!(fs::read(owned.join(format!("{name}.sns"))).unwrap(), crate::speech_legacy::read_sample(&mut file, &sample).unwrap());
            }
        }
        assert_eq!(preflight(&owned, Platform::Xbox).state, "complete");
        let repeated = run(&owned);
        assert_eq!((repeated.files_written, repeated.files_retained), (0, 76426));
        for (path, before) in sources { assert_eq!(crate::game_manifest::hash(fs::File::open(path).unwrap()).unwrap(), before); }
        println!("76,426 exports verified; repeat writes zero files. Outputs: {}", owned.display());
    }

    #[test]
    fn vanilla_boot_conflicts_are_resolved_without_overwriting_or_rerunning() {
        let t = Tmp::new("vanilla-boot");
        let archive = t.0.join("boot.big");
        write_big(&archive, &[("index.bh", b"PACKED"), ("loading.tex", b"TEXTURE")]);
        fs::write(t.0.join("index.bh"), b"LOOSE!").unwrap();
        let mut manifest: crate::game_manifest::Manifest = serde_json::from_str(include_str!("../../../anchors/xbox360/nhl07/game-files.json")).unwrap();
        manifest.files = [("boot.big", "container"), ("index.bh", "loose")].into_iter().map(|(name, kind)| {
            let bytes = fs::read(t.0.join(name)).unwrap();
            crate::game_manifest::Entry { path: name.into(), size: bytes.len() as u64, sha256: crate::game_manifest::hash(bytes.as_slice()).unwrap(), kind: kind.into(), optional: false }
        }).collect();
        let pre = preflight_with_manifest(&t.0, Some(&manifest));
        assert_eq!(pre.members_to_write, 1, "loading texture still needs extraction");
        assert_eq!(pre.archives[0].members.iter().find(|m| m.name == "index.bh").unwrap().state, "vanilla");
        assert_eq!(run(&t.0).files_conflicting, 1, "simulate old extraction marker");
        assert_eq!(preflight_with_manifest(&t.0, Some(&manifest)).state, "complete");
        assert_eq!(fs::read(t.0.join("index.bh")).unwrap(), b"LOOSE!");
        fs::write(t.0.join("index.bh"), b"MODDED").unwrap();
        assert_ne!(preflight_with_manifest(&t.0, Some(&manifest)).state, "complete", "same-size modifications are not vanilla");
        fs::write(t.0.join("index.bh"), b"LOOSE!").unwrap();
        fs::remove_file(t.0.join("loading.tex")).unwrap();
        assert_ne!(preflight_with_manifest(&t.0, Some(&manifest)).state, "complete", "missing extracted outputs still need extraction");
        fs::write(t.0.join("loading.tex"), b"TEXTURE").unwrap();
        write_big(&archive, &[("index.bh", b"CHANGED"), ("loading.tex", b"TEXTURE")]);
        assert_ne!(preflight_with_manifest(&t.0, Some(&manifest)).state, "complete", "changed archive cannot use the vanilla exception");
        // A fresh run uses the same policy, rather than recording another conflict.
        write_big(&archive, &[("index.bh", b"PACKED"), ("loading.tex", b"TEXTURE")]);
        fs::remove_file(marker_path(&archive)).unwrap();
        fs::remove_file(t.0.join("loading.tex")).unwrap();
        let fresh = extract_preflight(preflight_with_manifest(&t.0, Some(&manifest)), &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!((fresh.files_written, fresh.files_retained, fresh.files_conflicting), (1, 1, 0));
        assert!(read_marker(&archive).unwrap().complete);
        assert_eq!(preflight_with_manifest(&t.0, Some(&manifest)).state, "complete");
        fs::write(t.0.join("index.bh"), b"MODDED").unwrap();
        assert_ne!(preflight_with_manifest(&t.0, Some(&manifest)).state, "complete", "preserved vanilla outcomes must be reverified");
    }

    #[test]
    fn extracted_big_members_are_outputs_not_new_inputs_on_rescan() {
        let t = Tmp::new("nested-output");
        let mut nested = ArchiveBuilder::new();
        nested.add_entry("texture.bin", b"PIXELS".to_vec());
        let nested = nested.build().unwrap();
        write_big(&t.0.join("game.big"), &[("fe/logo.big", b"texture payload"), ("fe/nested.big", &nested)]);
        // An independent unreadable source must still be reported.
        fs::write(t.0.join("speech.big"), b"unknown speech format").unwrap();
        let first = run(&t.0);
        assert_eq!(first.files_written, 2);
        assert_eq!(first.archives_failed, 1);
        let after = preflight(&t.0, Platform::Xbox);
        assert_eq!(after.archives.len(), 2);
        assert_eq!(after.members_total, 2);
        assert_eq!(after.members_to_write, 0);
        assert_eq!(after.state, "partial", "unreadable independent archives are not complete");
        let second = run(&t.0);
        assert_eq!(second.files_written, 0);
        assert_eq!(second.files_retained, 2);
        assert_eq!(second.archives_failed, 1);
        assert!(!t.0.join("fe/texture.bin").exists(), "do not recursively unpack native assets");
        // Manually extracted installs have no marker; archive ownership still applies.
        fs::remove_file(marker_path(&t.0.join("game.big"))).unwrap();
        assert_eq!(preflight(&t.0, Platform::Xbox).archives.len(), 2);
        // Selecting the nested directory explicitly makes it an independent extraction scope.
        assert_eq!(preflight(&t.0.join("fe"), Platform::Xbox).archives.len(), 2);
    }

    #[test]
    fn preflight_then_extract_then_complete_and_rerun_retains() {
        let t = Tmp::new("basic");
        write_big(&t.0.join("game.big"), &[("db/nhlng.db", b"DB"), ("fe/x.txt", b"X")]);
        let pre = preflight(&t.0, Platform::Xbox);
        assert_eq!(pre.state, "needed");
        assert_eq!((pre.members_total, pre.members_to_write), (2, 2));
        assert!(!t.0.join("db/nhlng.db").exists(), "preflight writes nothing");
        let p = run(&t.0);
        assert_eq!((p.files_written, p.files_retained, p.archives_done), (2, 0, 1));
        assert_eq!(fs::read(t.0.join("db/nhlng.db")).unwrap(), b"DB");
        let marker = read_marker(&t.0.join("game.big")).unwrap();
        assert!(marker.complete);
        let pre = preflight(&t.0, Platform::Xbox);
        assert_eq!(pre.state, "complete", "{}", pre.reason);
        let p = run(&t.0);
        assert_eq!((p.files_written, p.files_retained), (0, 2));
    }

    #[test]
    fn a_differing_loose_file_is_never_overwritten_and_an_equal_one_is_retained() {
        let t = Tmp::new("conflict");
        write_big(&t.0.join("game.big"), &[("db/nhlng.db", b"DB"), ("fe/x.txt", b"X")]);
        fs::create_dir_all(t.0.join("db")).unwrap();
        fs::write(t.0.join("db/nhlng.db"), b"EDITED").unwrap();
        fs::create_dir_all(t.0.join("fe")).unwrap();
        fs::write(t.0.join("fe/x.txt"), b"X").unwrap();
        let pre = preflight(&t.0, Platform::Xbox);
        assert_eq!(pre.state, "partial");
        let p = run(&t.0);
        assert_eq!((p.files_written, p.files_retained, p.files_conflicting), (0, 1, 1));
        assert_eq!(fs::read(t.0.join("db/nhlng.db")).unwrap(), b"EDITED");
        assert!(!read_marker(&t.0.join("game.big")).unwrap().complete);
        assert_ne!(preflight(&t.0, Platform::Xbox).state, "complete");
    }

    #[test]
    fn a_name_claimed_by_two_archives_is_blocked_for_both() {
        let t = Tmp::new("collide");
        write_big(&t.0.join("base.big"), &[("db/nhlng.db", b"BASE"), ("only_base.txt", b"b")]);
        write_big(&t.0.join("patch.big"), &[("db/nhlng.db", b"PATCH")]);
        let pre = preflight(&t.0, Platform::Xbox);
        assert_eq!(pre.collisions.len(), 1);
        let p = run(&t.0);
        assert_eq!((p.files_written, p.files_blocked), (1, 2));
        assert!(!t.0.join("db/nhlng.db").exists(), "neither archive wins");
        assert!(t.0.join("only_base.txt").is_file());
        let pre = preflight(&t.0, Platform::Xbox);
        assert_eq!(pre.state, "blocked", "{}", pre.reason);
    }

    #[test]
    fn traversal_names_are_refused_and_counted() {
        let t = Tmp::new("evil");
        write_big(&t.0.join("evil.big"), &[("../escape.txt", b"nope"), ("ok.txt", b"yes")]);
        let p = run(&t.0);
        assert_eq!((p.files_unsafe, p.files_written), (1, 1));
        assert!(!t.0.parent().unwrap().join("escape.txt").exists());
    }

    #[test]
    fn cancellation_keeps_written_files_and_a_rerun_finishes() {
        let t = Tmp::new("cancel");
        write_big(&t.0.join("game.big"), &[("a.txt", b"A"), ("b.txt", b"B"), ("c.txt", b"C")]);
        let cancel = AtomicBool::new(false);
        let p = extract(&t.0, Platform::Xbox, &cancel, &mut |pr| {
            if pr.files_written == 1 {
                cancel.store(true, Ordering::Relaxed);
            }
        })
        .unwrap();
        assert!(p.cancelled);
        assert_eq!(p.files_written, 1);
        assert!(t.0.join("a.txt").is_file() && !t.0.join("c.txt").exists());
        let m = read_marker(&t.0.join("game.big")).unwrap();
        assert!(m.cancelled && !m.complete);
        let pre = preflight(&t.0, Platform::Xbox);
        assert_eq!(pre.state, "partial");
        let p = run(&t.0);
        assert_eq!((p.files_written, p.files_retained), (2, 1));
        assert_eq!(preflight(&t.0, Platform::Xbox).state, "complete");
    }

    #[test]
    fn a_changed_archive_invalidates_its_marker() {
        let t = Tmp::new("changed");
        write_big(&t.0.join("game.big"), &[("a.txt", b"A")]);
        run(&t.0);
        assert_eq!(preflight(&t.0, Platform::Xbox).state, "complete");
        write_big(&t.0.join("game.big"), &[("a.txt", b"A"), ("b.txt", b"B")]);
        let pre = preflight(&t.0, Platform::Xbox);
        assert_ne!(pre.state, "complete");
        assert_eq!(pre.members_to_write, 1);
    }

    #[test]
    fn archive_free_roots_are_not_applicable() {
        let t = Tmp::new("na");
        assert_eq!(preflight(&t.0, Platform::Ps3).state, "not_applicable");
        assert_eq!(preflight(&t.0, Platform::Xbox).state, "not_applicable");
        let cancel = AtomicBool::new(false);
        assert!(extract(&t.0, Platform::Xbox, &cancel, &mut |_| {}).is_err());
    }

    #[test]
    fn ps3_supported_archives_extract_without_overwriting_existing_assets() {
        let t = Tmp::new("ps3-big");
        write_big(&t.0.join("data0.big"), &[("fe/logo.dds", b"original"), ("db/nhlng.db", b"database")]);
        fs::create_dir_all(t.0.join("fe")).unwrap();
        fs::write(t.0.join("fe/logo.dds"), b"owned edit").unwrap();
        let pre = preflight(&t.0, Platform::Ps3);
        assert_ne!(pre.state, "not_applicable");
        let result = extract(&t.0, Platform::Ps3, &AtomicBool::new(false), &mut |_| {}).unwrap();
        assert_eq!(result.files_written, 1);
        assert_eq!(fs::read(t.0.join("db/nhlng.db")).unwrap(), b"database");
        assert_eq!(fs::read(t.0.join("fe/logo.dds")).unwrap(), b"owned edit");
        assert_eq!(preflight(&t.0, Platform::Ps3).members_to_write, 0);
    }

    #[test]
    fn an_unparseable_big_counts_as_failed_not_as_extracted() {
        let t = Tmp::new("bad");
        fs::write(t.0.join("junk.big"), b"NOPE not an archive at all").unwrap();
        write_big(&t.0.join("ok.big"), &[("a.txt", b"A")]);
        let pre = preflight(&t.0, Platform::Xbox);
        assert_eq!(pre.archives.iter().filter(|a| a.state == "unsupported").count(), 1);
        let p = run(&t.0);
        assert_eq!((p.archives_failed, p.files_written), (1, 1));
        let pre = preflight(&t.0, Platform::Xbox);
        assert_eq!(pre.state, "partial");
        assert!(pre.reason.contains("could not be parsed"), "{}", pre.reason);
    }
}
