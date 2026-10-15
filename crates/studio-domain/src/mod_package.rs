//! ONB-003 §4–§5 — the distributable mod package: a **data-only** ZIP container (stored entries,
//! fixed timestamps, sorted names; extension `.nhlmod`) holding a manifest, the semantic edits and
//! the authored source payloads. Nothing in a package is executed; a recipient's Studio reads it,
//! validates it and converts the edits through its own adapters.
//!
//! ```text
//! <name>.nhlmod
//!   manifest.json          schema `nhlms-modpkg/1`: identity, game, declared targets, base identities,
//!                          categories, every other entry's size + SHA-256
//!   edits.json             schema `nhlms-modpkg-edits/1`: database cells, dictionary texts, textures
//!   payloads/<sha256>.<ext> the authored sources (PNG …) the asset edits are encoded from
//! ```
//!
//! **Reading is hostile-input parsing.** The container is checked before any entry is used: one
//! supported method (stored), no encryption, no zip64, every name relative / forward-slash /
//! no `..` / no drive / no UNC / no control characters, unique case-insensitively, counts and
//! sizes under [`Limits`], the central directory consistent with each local header, every entry
//! listed in the manifest with the right length and hash and no entry the manifest does not list.
//! Hashes establish integrity, not who made the package.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::baselines::sha256_hex;
use crate::{DomainError, Result};

pub const PACKAGE_SCHEMA: &str = "nhlms-modpkg/1";
/// The edits document: version 2 binds every cell to its row's **key values** (and a fingerprint
/// of the author's baseline row), never to a row position alone (R71-R1).
pub const EDITS_SCHEMA: &str = "nhlms-modpkg-edits/6";
/// Version 5: as 6, but a career-row removal cannot also rewrite surviving rows.
pub const EDITS_SCHEMA_V5: &str = "nhlms-modpkg-edits/5";
pub const EDITS_SCHEMA_V4: &str = "nhlms-modpkg-edits/4";
pub const EDITS_SCHEMA_V3: &str = "nhlms-modpkg-edits/3";
pub const EDITS_SCHEMA_V2: &str = "nhlms-modpkg-edits/2";
/// Version 1 (candidate r71) bound cells to physical row positions. Such a package still inspects,
/// but the installer refuses to replay its cells: the author rebuilds it from the project.
pub const EDITS_SCHEMA_V1: &str = "nhlms-modpkg-edits/1";
pub const PACKAGE_EXTENSION: &str = "nhlmod";
pub const MANIFEST_ENTRY: &str = "manifest.json";
pub const EDITS_ENTRY: &str = "edits.json";
pub const PAYLOADS_DIR: &str = "payloads";

/// Container limits enforced before any entry is read.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub max_entries: usize,
    pub max_entry_bytes: u64,
    pub max_total_bytes: u64,
    pub max_manifest_bytes: u64,
    pub max_name_len: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_entries: 4096,
            max_entry_bytes: 512 * 1024 * 1024,
            max_total_bytes: 2 * 1024 * 1024 * 1024,
            max_manifest_bytes: 16 * 1024 * 1024,
            max_name_len: 240,
        }
    }
}

// ---------------------------------------------------------------------- manifest

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct PackageManifest {
    pub schema: String,
    /// Content id: SHA-256 over the sorted entries' hashes and the identity fields.
    pub package_id: String,
    pub name: String,
    pub version: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub author: String,
    #[serde(default)]
    pub description: String,
    pub created: String,
    pub studio: StudioProvenance,
    pub game_id: String,
    pub title: String,
    /// The platform the package was authored on.
    pub author_platform: String,
    /// Every platform of the game, with its measured support.
    pub targets: Vec<TargetSupport>,
    /// The author's baseline identities per role (what the edits were measured against).
    pub base: Vec<BaseIdentity>,
    pub categories: Vec<CategoryCount>,
    /// Every entry other than `manifest.json`.
    pub entries: Vec<EntryRecord>,
    pub project: ProjectProvenance,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct StudioProvenance {
    pub version: String,
    #[serde(default)]
    pub inventory_head: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProjectProvenance {
    pub project_id: String,
    pub source_kind: String,
    pub save_seq: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TargetSupport {
    pub platform: String,
    /// `supported` | `unsupported`.
    pub status: String,
    pub components: Vec<ComponentSupport>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComponentSupport {
    /// `cells` | `dictionary` | `textures` | `members`.
    pub category: String,
    /// `supported` | `unsupported` | `empty`.
    pub status: String,
    pub reason: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct BaseIdentity {
    pub role: String,
    pub file_name: String,
    pub len: u64,
    pub sha256: String,
    pub provenance: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CategoryCount {
    pub category: String,
    pub count: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct EntryRecord {
    pub path: String,
    pub len: u64,
    pub sha256: String,
    /// `edits` | `payload`.
    pub kind: String,
}

// ---------------------------------------------------------------------- edits

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PackageEdits {
    pub schema: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub career_removals: Vec<CareerRemovalOp>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub schedules: Vec<ScheduleOp>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub free_agents: Option<FreeAgentPoolOp>,
    #[serde(default)]
    pub cells: Vec<CellOp>,
    #[serde(default)]
    pub dictionary: Vec<DictionaryOp>,
    #[serde(default)]
    pub textures: Vec<TextureOp>,
}

/// Ordered career-table removals bound to every original row, preventing index drift.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CareerRemovalOp {
    pub table: String,
    pub before_rows: Vec<String>,
    pub removed: Vec<u32>,
    /// Surviving rows the author also edited (edits schema 6): the original row index and the
    /// row's complete new bytes. Applied before the removals.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rewritten: Vec<CareerRowRewrite>,
}

/// One surviving career row's complete new content, by its index in the original table.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CareerRowRewrite {
    pub row: u32,
    /// Lower-case hex of the row's record bytes.
    pub bytes: String,
}

/// Lower-case hex of `bytes`.
pub fn to_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// The bytes of a hex string; `None` when it is not hex.
pub fn from_hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len()).step_by(2).map(|i| u8::from_str_radix(text.get(i..i + 2)?, 16).ok()).collect()
}

/// Explicit exhibition free-agent list, addressed by player IDs rather than row offsets.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct FreeAgentPoolOp {
    pub before: Vec<u64>,
    pub after: Vec<u64>,
}

/// Complete, bounded NHL07 source schedule; the exact before state prevents unsafe replay.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleOp {
    pub table: String,
    pub before: Vec<crate::schedule_edit::GameRow>,
    pub after: Vec<crate::schedule_edit::GameRow>,
}

/// A raw cell value as stored (bits) with its sign-aware decoding.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum CellValue {
    Int {
        raw: u64,
        /// The sign/sentinel-decoded value (what an editor stages).
        decoded: i64,
        display: String,
    },
    Text {
        text: String,
    },
}

/// One key-field value of the row a cell edit belongs to, as the author's baseline held it.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct KeyValue {
    pub studio_field: String,
    pub field_name: String,
    pub value: CellValue,
}

/// One changed database cell: the file it was measured in, the semantic location and both values.
///
/// **Identity (edits schema 2).** `keys` are the table's metadata key fields with the values the
/// author's baseline row carried; the installer resolves the recipient's row by them (exactly one
/// match, else refused). A table without key fields falls back to `record` **only** when the
/// recipient's row at that position hashes to `row_sha256` (the author's baseline row bytes); a
/// different row there is refused, never edited. `record` and `entity` alone are display context.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct CellOp {
    /// `main_db` | `roster` | `both` (the same change — same row identity, same values — measured
    /// in both files of a project that holds both).
    pub file: String,
    pub studio_table: String,
    pub table_name: String,
    pub record: u32,
    pub entity: String,
    pub studio_field: String,
    pub field_name: String,
    pub before: CellValue,
    pub after: CellValue,
    #[serde(default)]
    pub keys: Vec<KeyValue>,
    /// SHA-256 of the author's baseline row bytes (the whole fixed-length record).
    #[serde(default)]
    pub row_sha256: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct DictionaryOp {
    pub key: String,
    pub before: String,
    pub after: String,
}

/// A loose texture replacement: the install-relative file, the texture inside it and the source.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct TextureOp {
    pub rel_path: String,
    #[serde(default)]
    pub texture_index: Option<u32>,
    /// `payloads/<sha256>.<ext>`.
    pub payload: String,
    pub width: u32,
    pub height: u32,
    /// The installed file the author replaced (when known): the installer reports a differing
    /// recipient file as a conflict instead of writing over it silently.
    #[serde(default)]
    pub prior_len: Option<u64>,
    #[serde(default)]
    pub prior_sha256: Option<String>,
    /// The author's platform. Another platform is declared only where the title's loose-asset
    /// layout is registered as shared (NHL Legacy on Xbox 360 ↔ Recompiled); the recipient's own
    /// file at the same relative path is then re-encoded from the authored source and every
    /// placement is verified against that recipient at install.
    pub platform: String,
}

// ---------------------------------------------------------------------- CRC32 / ZIP

fn crc32(bytes: &[u8]) -> u32 {
    static TABLE: std::sync::OnceLock<[u32; 256]> = std::sync::OnceLock::new();
    let table = TABLE.get_or_init(|| {
        let mut t = [0u32; 256];
        for (i, slot) in t.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
            }
            *slot = c;
        }
        t
    });
    let mut c = 0xFFFF_FFFFu32;
    for &b in bytes {
        c = table[((c ^ b as u32) & 0xFF) as usize] ^ (c >> 8);
    }
    c ^ 0xFFFF_FFFF
}

const LOCAL_SIG: u32 = 0x0403_4B50;
const CENTRAL_SIG: u32 = 0x0201_4B50;
const EOCD_SIG: u32 = 0x0605_4B50;
/// DOS date 2026-10-01, time 00:00:00 — the same for every entry (deterministic output).
const DOS_DATE: u16 = ((2026 - 1980) << 9) | (10 << 5) | 1;
const DOS_TIME: u16 = 0;

fn le16(v: &mut Vec<u8>, x: u16) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn le32(v: &mut Vec<u8>, x: u32) {
    v.extend_from_slice(&x.to_le_bytes());
}
fn rd16(b: &[u8], at: usize) -> Option<u16> {
    b.get(at..at + 2).map(|s| u16::from_le_bytes([s[0], s[1]]))
}
fn rd32(b: &[u8], at: usize) -> Option<u32> {
    b.get(at..at + 4).map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
}

/// Write a stored, deterministic ZIP: entries sorted by name, fixed timestamps, no extra fields.
pub fn write_zip(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let mut sorted: Vec<&(String, Vec<u8>)> = entries.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::new();
    let mut central = Vec::new();
    for (name, bytes) in sorted {
        check_entry_name(name)?;
        if bytes.len() as u64 >= u32::MAX as u64 {
            return Err(DomainError::Validation(format!("{name}: entries of 4 GiB or more are not written (no zip64)")));
        }
        let crc = crc32(bytes);
        let offset = out.len() as u32;
        le32(&mut out, LOCAL_SIG);
        le16(&mut out, 20);
        le16(&mut out, 0x0800); // UTF-8 names
        le16(&mut out, 0); // stored
        le16(&mut out, DOS_TIME);
        le16(&mut out, DOS_DATE);
        le32(&mut out, crc);
        le32(&mut out, bytes.len() as u32);
        le32(&mut out, bytes.len() as u32);
        le16(&mut out, name.len() as u16);
        le16(&mut out, 0);
        out.extend_from_slice(name.as_bytes());
        out.extend_from_slice(bytes);

        le32(&mut central, CENTRAL_SIG);
        le16(&mut central, 20);
        le16(&mut central, 20);
        le16(&mut central, 0x0800);
        le16(&mut central, 0);
        le16(&mut central, DOS_TIME);
        le16(&mut central, DOS_DATE);
        le32(&mut central, crc);
        le32(&mut central, bytes.len() as u32);
        le32(&mut central, bytes.len() as u32);
        le16(&mut central, name.len() as u16);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le16(&mut central, 0);
        le32(&mut central, 0);
        le32(&mut central, offset);
        central.extend_from_slice(name.as_bytes());
    }
    let cd_offset = out.len() as u32;
    out.extend_from_slice(&central);
    le32(&mut out, EOCD_SIG);
    le16(&mut out, 0);
    le16(&mut out, 0);
    le16(&mut out, entries.len() as u16);
    le16(&mut out, entries.len() as u16);
    le32(&mut out, central.len() as u32);
    le32(&mut out, cd_offset);
    le16(&mut out, 0);
    Ok(out)
}

/// One entry of a validated container.
#[derive(Clone, Debug)]
pub struct ZipEntry {
    pub name: String,
    pub len: u64,
    data_offset: usize,
    crc: u32,
}

/// A parsed, structurally validated container (nothing extracted yet).
#[derive(Debug)]
pub struct ZipArchive {
    bytes: Vec<u8>,
    pub entries: Vec<ZipEntry>,
}

/// The entry-name rules every package path must satisfy.
pub fn check_entry_name(name: &str) -> Result<()> {
    let bad = |why: &str| Err(DomainError::Validation(format!("package entry {name:?} is refused: {why}")));
    if name.is_empty() {
        return bad("empty name");
    }
    if name.len() > Limits::default().max_name_len {
        return bad("name longer than the limit");
    }
    if name.contains('\\') {
        return bad("backslashes are not allowed (forward slashes only)");
    }
    if name.starts_with('/') {
        return bad("absolute paths are not allowed");
    }
    if name.len() >= 2 && name.as_bytes()[1] == b':' {
        return bad("drive-letter paths are not allowed");
    }
    if name.starts_with("//") || name.starts_with("\\\\") {
        return bad("UNC paths are not allowed");
    }
    if name.ends_with('/') {
        return bad("directory entries are not allowed");
    }
    for part in name.split('/') {
        if part.is_empty() {
            return bad("empty path component");
        }
        if part == "." || part == ".." {
            return bad("`.` and `..` components are not allowed");
        }
        if part.ends_with('.') || part.ends_with(' ') {
            return bad("a component may not end with a dot or a space");
        }
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "COM2" | "COM3" | "COM4" | "COM5" | "COM6" | "COM7" | "COM8" | "COM9" | "LPT1" | "LPT2" | "LPT3" | "LPT4" | "LPT5" | "LPT6" | "LPT7" | "LPT8" | "LPT9") {
            return bad("a reserved device name is not allowed");
        }
    }
    if name.chars().any(|c| c.is_control() || matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*')) {
        return bad("control or reserved characters are not allowed");
    }
    Ok(())
}

impl ZipArchive {
    /// Parse and validate the container structure under `limits`. Refuses anything the writer
    /// above would not have produced, and anything unsafe to extract.
    pub fn parse(bytes: Vec<u8>, limits: &Limits) -> Result<ZipArchive> {
        let refuse = |why: String| DomainError::Validation(format!("the package is not a valid Studio mod package: {why}"));
        let n = bytes.len();
        if n < 22 {
            return Err(refuse("too short".into()));
        }
        if n as u64 > limits.max_total_bytes {
            return Err(refuse(format!("{n} bytes exceeds the {} byte limit", limits.max_total_bytes)));
        }
        // EOCD: the last 22 bytes when there is no comment; search back over the comment window.
        let search_from = n.saturating_sub(22 + 65_535);
        let mut eocd = None;
        let mut at = n - 22;
        loop {
            if rd32(&bytes, at) == Some(EOCD_SIG) {
                eocd = Some(at);
                break;
            }
            if at == search_from {
                break;
            }
            at -= 1;
        }
        let eocd = eocd.ok_or_else(|| refuse("no end-of-central-directory record".into()))?;
        let disk = rd16(&bytes, eocd + 4).unwrap_or(1);
        let cd_disk = rd16(&bytes, eocd + 6).unwrap_or(1);
        let count = rd16(&bytes, eocd + 10).unwrap_or(0) as usize;
        let total = rd16(&bytes, eocd + 8).unwrap_or(0) as usize;
        let cd_size = rd32(&bytes, eocd + 12).unwrap_or(0) as usize;
        let cd_offset = rd32(&bytes, eocd + 16).unwrap_or(0) as usize;
        if disk != 0 || cd_disk != 0 || count != total {
            return Err(refuse("multi-disk archives are not supported".into()));
        }
        if count == 0xFFFF || cd_offset == 0xFFFF_FFFF || cd_size == 0xFFFF_FFFF {
            return Err(refuse("zip64 archives are not supported".into()));
        }
        if count > limits.max_entries {
            return Err(refuse(format!("{count} entries exceeds the limit of {}", limits.max_entries)));
        }
        if cd_offset.checked_add(cd_size).is_none_or(|end| end > eocd) {
            return Err(refuse("the central directory lies outside the file".into()));
        }
        let mut entries = Vec::with_capacity(count);
        let mut seen_fold: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut pos = cd_offset;
        let mut total_bytes: u64 = 0;
        for _ in 0..count {
            if rd32(&bytes, pos) != Some(CENTRAL_SIG) {
                return Err(refuse("a central directory entry is malformed".into()));
            }
            let flags = rd16(&bytes, pos + 8).unwrap_or(0);
            let method = rd16(&bytes, pos + 10).unwrap_or(0);
            let crc = rd32(&bytes, pos + 16).unwrap_or(0);
            let csize = rd32(&bytes, pos + 20).unwrap_or(0) as u64;
            let usize_ = rd32(&bytes, pos + 24).unwrap_or(0) as u64;
            let name_len = rd16(&bytes, pos + 28).unwrap_or(0) as usize;
            let extra_len = rd16(&bytes, pos + 30).unwrap_or(0) as usize;
            let comment_len = rd16(&bytes, pos + 32).unwrap_or(0) as usize;
            let local_offset = rd32(&bytes, pos + 42).unwrap_or(0) as usize;
            if flags & 0x0001 != 0 || flags & 0x0040 != 0 {
                return Err(refuse("encrypted entries are not supported".into()));
            }
            if flags & 0x0008 != 0 {
                return Err(refuse("data-descriptor entries are not supported".into()));
            }
            if method != 0 {
                return Err(refuse(format!("compression method {method} is not supported (stored only)")));
            }
            if csize != usize_ {
                return Err(refuse("a stored entry's sizes disagree".into()));
            }
            if usize_ > limits.max_entry_bytes {
                return Err(refuse(format!("an entry of {usize_} bytes exceeds the per-entry limit")));
            }
            if usize_ == 0xFFFF_FFFF || local_offset == 0xFFFF_FFFF {
                return Err(refuse("zip64 entries are not supported".into()));
            }
            let name_start = pos + 46;
            let name_bytes = bytes.get(name_start..name_start + name_len).ok_or_else(|| refuse("a name runs past the central directory".into()))?;
            let name = std::str::from_utf8(name_bytes).map_err(|_| refuse("an entry name is not UTF-8".into()))?.to_string();
            check_entry_name(&name)?;
            if !seen_fold.insert(name.to_lowercase()) {
                return Err(refuse(format!("entry {name:?} collides with another entry when case is ignored")));
            }
            // The local header must agree with the central one.
            if rd32(&bytes, local_offset) != Some(LOCAL_SIG) {
                return Err(refuse(format!("entry {name:?}: its local header is missing")));
            }
            let l_method = rd16(&bytes, local_offset + 8).unwrap_or(1);
            let l_crc = rd32(&bytes, local_offset + 14).unwrap_or(0);
            let l_csize = rd32(&bytes, local_offset + 18).unwrap_or(0) as u64;
            let l_usize = rd32(&bytes, local_offset + 22).unwrap_or(0) as u64;
            let l_name_len = rd16(&bytes, local_offset + 26).unwrap_or(0) as usize;
            let l_extra_len = rd16(&bytes, local_offset + 28).unwrap_or(0) as usize;
            if l_method != 0 || l_crc != crc || l_csize != csize || l_usize != usize_ || l_name_len != name_len {
                return Err(refuse(format!("entry {name:?}: its local header disagrees with the central directory")));
            }
            let l_name = bytes.get(local_offset + 30..local_offset + 30 + l_name_len).ok_or_else(|| refuse("a local name runs past the file".into()))?;
            if l_name != name_bytes {
                return Err(refuse(format!("entry {name:?}: its local header names a different file")));
            }
            let data_offset = local_offset + 30 + l_name_len + l_extra_len;
            if (data_offset as u64).checked_add(usize_).is_none_or(|end| end > cd_offset as u64) {
                return Err(refuse(format!("entry {name:?}: its data lies outside the file")));
            }
            total_bytes = total_bytes.saturating_add(usize_);
            if total_bytes > limits.max_total_bytes {
                return Err(refuse("the entries' total size exceeds the limit".into()));
            }
            entries.push(ZipEntry { name, len: usize_, data_offset, crc });
            pos = name_start + name_len + extra_len + comment_len;
        }
        if pos != cd_offset + cd_size {
            return Err(refuse("the central directory size disagrees with its entries".into()));
        }
        Ok(ZipArchive { bytes, entries })
    }

    pub fn entry(&self, name: &str) -> Option<&ZipEntry> {
        self.entries.iter().find(|e| e.name == name)
    }

    /// The bytes of one entry, CRC-checked.
    pub fn read(&self, name: &str) -> Result<Vec<u8>> {
        let e = self.entry(name).ok_or_else(|| DomainError::Validation(format!("the package has no entry {name:?}")))?;
        let data = &self.bytes[e.data_offset..e.data_offset + e.len as usize];
        if crc32(data) != e.crc {
            return Err(DomainError::Validation(format!("package entry {name:?} fails its CRC check; the package is damaged")));
        }
        Ok(data.to_vec())
    }
}

// ---------------------------------------------------------------------- package I/O

/// A validated package: manifest, edits and the container.
#[derive(Debug)]
pub struct InspectedPackage {
    pub manifest: PackageManifest,
    pub edits: PackageEdits,
    pub archive: ZipArchive,
    pub file_len: u64,
    pub file_sha256: String,
}

impl InspectedPackage {
    /// A payload's bytes, verified against the manifest's record.
    pub fn payload(&self, path: &str) -> Result<Vec<u8>> {
        let rec = self.manifest.entries.iter().find(|e| e.path == path).ok_or_else(|| DomainError::Validation(format!("the package manifest lists no entry {path:?}")))?;
        let bytes = self.archive.read(path)?;
        if bytes.len() as u64 != rec.len || sha256_hex(&bytes) != rec.sha256 {
            return Err(DomainError::Validation(format!("package entry {path:?} does not match its manifest hash; the package is damaged or altered")));
        }
        Ok(bytes)
    }
}

/// Compute the content id over the identity fields and the entries.
pub fn compute_package_id(m: &PackageManifest) -> String {
    let mut probe = m.clone();
    probe.package_id = String::new();
    let json = serde_json::to_vec(&probe).unwrap_or_default();
    sha256_hex(&json)
}

/// Serialise a package: the manifest's `entries` and `package_id` are (re)computed from
/// `edits` and `payloads` here, so the manifest can never disagree with the container.
pub fn build_package(mut manifest: PackageManifest, edits: &PackageEdits, payloads: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    let edits_json = serde_json::to_vec_pretty(edits).map_err(|e| DomainError::Validation(e.to_string()))?;
    let mut entries: Vec<(String, Vec<u8>)> = vec![(EDITS_ENTRY.into(), edits_json)];
    let mut records = vec![];
    for (name, bytes) in payloads {
        if !name.starts_with("payloads/") {
            return Err(DomainError::Validation(format!("payload {name:?} must live under payloads/")));
        }
        check_entry_name(name)?;
        entries.push((name.clone(), bytes.clone()));
    }
    for (name, bytes) in &entries {
        records.push(EntryRecord { path: name.clone(), len: bytes.len() as u64, sha256: sha256_hex(bytes), kind: if name == EDITS_ENTRY { "edits".into() } else { "payload".into() } });
    }
    records.sort_by(|a, b| a.path.cmp(&b.path));
    manifest.entries = records;
    manifest.schema = PACKAGE_SCHEMA.into();
    manifest.package_id = compute_package_id(&manifest);
    let manifest_json = serde_json::to_vec_pretty(&manifest).map_err(|e| DomainError::Validation(e.to_string()))?;
    entries.push((MANIFEST_ENTRY.into(), manifest_json));
    write_zip(&entries)
}

/// Read and validate a package file completely (container, manifest, every entry's hash and
/// length, the edits document). Nothing is written.
pub fn inspect_package(path: &Path, limits: &Limits) -> Result<InspectedPackage> {
    let meta = std::fs::metadata(path).map_err(|source| DomainError::Io { path: path.display().to_string(), source })?;
    if !meta.is_file() {
        return Err(DomainError::Validation(format!("{} is not a file", path.display())));
    }
    if meta.len() > limits.max_total_bytes {
        return Err(DomainError::Validation(format!("{} is {} bytes, above the package limit", path.display(), meta.len())));
    }
    let bytes = std::fs::read(path).map_err(|source| DomainError::Io { path: path.display().to_string(), source })?;
    inspect_package_bytes(bytes, limits)
}

pub fn inspect_package_bytes(bytes: Vec<u8>, limits: &Limits) -> Result<InspectedPackage> {
    let file_len = bytes.len() as u64;
    let file_sha256 = sha256_hex(&bytes);
    let archive = ZipArchive::parse(bytes, limits)?;
    let refuse = |why: String| DomainError::Validation(format!("the package is not a valid Studio mod package: {why}"));
    let m_entry = archive.entry(MANIFEST_ENTRY).ok_or_else(|| refuse(format!("no {MANIFEST_ENTRY}")))?;
    if m_entry.len > limits.max_manifest_bytes {
        return Err(refuse("the manifest exceeds its size limit".into()));
    }
    let manifest_bytes = archive.read(MANIFEST_ENTRY)?;
    let head: serde_json::Value = serde_json::from_slice(&manifest_bytes).map_err(|e| refuse(format!("{MANIFEST_ENTRY} is not readable JSON: {e}")))?;
    let schema = head.get("schema").and_then(|v| v.as_str()).unwrap_or("");
    if schema != PACKAGE_SCHEMA {
        return Err(refuse(format!("manifest schema {:?} is not {PACKAGE_SCHEMA:?}; a newer or older Studio wrote it", if schema.is_empty() { "<missing>" } else { schema })));
    }
    let manifest: PackageManifest = serde_json::from_slice(&manifest_bytes).map_err(|e| refuse(format!("{MANIFEST_ENTRY}: {e}")))?;
    if compute_package_id(&manifest) != manifest.package_id {
        return Err(refuse("the manifest's package id does not match its contents".into()));
    }
    // Every entry other than the manifest must be listed, and every listed entry must exist with
    // the recorded length and hash.
    let listed: BTreeMap<&str, &EntryRecord> = manifest.entries.iter().map(|e| (e.path.as_str(), e)).collect();
    if listed.len() != manifest.entries.len() {
        return Err(refuse("the manifest lists an entry twice".into()));
    }
    for e in &archive.entries {
        if e.name == MANIFEST_ENTRY {
            continue;
        }
        let rec = listed.get(e.name.as_str()).ok_or_else(|| refuse(format!("entry {:?} is not listed in the manifest", e.name)))?;
        if rec.len != e.len {
            return Err(refuse(format!("entry {:?} is {} bytes but the manifest records {}", e.name, e.len, rec.len)));
        }
        let data = archive.read(&e.name)?;
        if sha256_hex(&data) != rec.sha256 {
            return Err(refuse(format!("entry {:?} does not match its manifest hash", e.name)));
        }
        if !matches!(rec.kind.as_str(), "edits" | "payload") {
            return Err(refuse(format!("entry {:?} has an unknown kind {:?}", e.name, rec.kind)));
        }
        if rec.kind == "payload" && !e.name.starts_with("payloads/") {
            return Err(refuse(format!("payload entry {:?} is outside payloads/", e.name)));
        }
    }
    for rec in &manifest.entries {
        if archive.entry(&rec.path).is_none() {
            return Err(refuse(format!("the manifest lists {:?} but the package has no such entry", rec.path)));
        }
    }
    if manifest.targets.is_empty() {
        return Err(refuse("the manifest declares no target platform".into()));
    }
    let edits_bytes = archive.read(EDITS_ENTRY).map_err(|_| refuse(format!("no {EDITS_ENTRY}")))?;
    let edits: PackageEdits = serde_json::from_slice(&edits_bytes).map_err(|e| refuse(format!("{EDITS_ENTRY}: {e}")))?;
    if edits.schema != EDITS_SCHEMA && edits.schema != EDITS_SCHEMA_V5 && edits.schema != EDITS_SCHEMA_V4 && edits.schema != EDITS_SCHEMA_V3 && edits.schema != EDITS_SCHEMA_V2 && edits.schema != EDITS_SCHEMA_V1 {
        return Err(refuse(format!("edits schema {:?} is not {EDITS_SCHEMA:?}", edits.schema)));
    }
    for t in &edits.textures {
        if listed.get(t.payload.as_str()).map(|r| r.kind.as_str()) != Some("payload") {
            return Err(refuse(format!("texture edit {} names a payload {:?} the manifest does not list", t.rel_path, t.payload)));
        }
        if crate::assets::resolve_install_path(Path::new("."), &t.rel_path).is_none() || t.rel_path.contains("..") || t.rel_path.starts_with('/') || t.rel_path.contains(':') {
            return Err(refuse(format!("texture edit names an unsafe install path {:?}", t.rel_path)));
        }
    }
    for c in &edits.cells {
        if !matches!(c.file.as_str(), "main_db" | "roster" | "both") {
            return Err(refuse(format!("cell edit names an unknown file {:?}", c.file)));
        }
        if c.studio_table.len() != 4 || c.studio_field.len() != 4 {
            return Err(refuse(format!("cell edit names a malformed table/field id {:?}/{:?}", c.studio_table, c.studio_field)));
        }
        if edits.schema != EDITS_SCHEMA_V1 {
            if c.keys.iter().any(|k| k.studio_field.len() != 4) {
                return Err(refuse(format!("cell edit {} › {} names a malformed key field", c.table_name, c.field_name)));
            }
            match &c.row_sha256 {
                Some(h) if h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()) => {}
                _ => return Err(refuse(format!("cell edit {} › {} carries no row fingerprint", c.table_name, c.field_name))),
            }
        }
    }
    let counts = |cat: &str| manifest.categories.iter().find(|c| c.category == cat).map(|c| c.count).unwrap_or(0);
    if let Some(pool) = &edits.free_agents {
        if (edits.schema != EDITS_SCHEMA && edits.schema != EDITS_SCHEMA_V5 && edits.schema != EDITS_SCHEMA_V4 && edits.schema != EDITS_SCHEMA_V3) || manifest.game_id != "nhl07" || manifest.author_platform != "xbox" {
            return Err(refuse("Explicit free-agent packages are qualified only for NHL 07 Xbox 360, edits schema 3".into()));
        }
        for ids in [&pool.before, &pool.after] {
            if ids.len() > 8191 || ids.iter().collect::<std::collections::HashSet<_>>().len() != ids.len() {
                return Err(refuse("Invalid or duplicate free-agent identities".into()));
            }
        }
    }
    let mut schedule_tables = std::collections::HashSet::new();
    for s in &edits.schedules {
        if (edits.schema != EDITS_SCHEMA && edits.schema != EDITS_SCHEMA_V5 && edits.schema != EDITS_SCHEMA_V4) || manifest.game_id != "nhl07" || manifest.author_platform != "xbox" ||
            !["vufv","fBWa","ADlP","uRbu","ksGb","qyyN","zcVp"].contains(&s.table.as_str()) ||
            !schedule_tables.insert(&s.table) || s.before.len()>8192 || s.after.len()>8192 || edits.cells.iter().any(|c| c.studio_table==s.table) {
            return Err(refuse("Invalid or overlapping NHL07 schedule replacement".into()));
        }
    }
    let mut career_tables = std::collections::HashSet::new();
    for op in &edits.career_removals {
        if (edits.schema != EDITS_SCHEMA && edits.schema != EDITS_SCHEMA_V5) || manifest.game_id != "nhl07" || manifest.author_platform != "xbox" ||
            (!op.rewritten.is_empty() && edits.schema != EDITS_SCHEMA) ||
            op.rewritten.windows(2).any(|w| w[0].row >= w[1].row) ||
            op.rewritten.iter().any(|r| r.row as usize >= op.before_rows.len() || op.removed.contains(&r.row) || from_hex(&r.bytes).is_none_or(|b| b.is_empty() || b.len() > 4096)) ||
            !["tsSy", "OAqO", "USfF", "nCDp"].contains(&op.table.as_str()) || !career_tables.insert(&op.table) ||
            op.before_rows.len() > 131072 || op.removed.is_empty() || op.removed.windows(2).any(|w| w[0] >= w[1]) ||
            op.removed.iter().any(|&r| r as usize >= op.before_rows.len()) ||
            op.before_rows.iter().any(|h| h.len() != 64 || !h.bytes().all(|b| b.is_ascii_hexdigit())) ||
            edits.cells.iter().any(|c| c.studio_table == op.table && op.removed.contains(&c.record)) {
            return Err(refuse("Invalid NHL07 career-row removal".into()));
        }
    }
    if counts("cells") != (edits.cells.len() + usize::from(edits.free_agents.is_some()) + edits.schedules.len() + edits.career_removals.len()) as u64 || counts("dictionary") != edits.dictionary.len() as u64 || counts("textures") != edits.textures.len() as u64 {
        return Err(refuse("the manifest's category counts disagree with the edits document".into()));
    }
    Ok(InspectedPackage { manifest, edits, archive, file_len, file_sha256 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manifest() -> PackageManifest {
        PackageManifest {
            schema: PACKAGE_SCHEMA.into(),
            package_id: String::new(),
            name: "t".into(),
            author: String::new(),
            version: "1".into(),
            description: String::new(),
            created: "2026-10-01T00:00:00Z".into(),
            studio: StudioProvenance { version: "0".into(), inventory_head: None },
            game_id: "nhl14".into(),
            title: "NHL 14".into(),
            author_platform: "ps3".into(),
            targets: vec![TargetSupport { platform: "ps3".into(), status: "supported".into(), components: vec![] }],
            base: vec![],
            categories: vec![CategoryCount { category: "cells".into(), count: 0 }, CategoryCount { category: "dictionary".into(), count: 0 }, CategoryCount { category: "textures".into(), count: 1 }],
            entries: vec![],
            project: ProjectProvenance { project_id: "p".into(), source_kind: "fresh".into(), save_seq: 1 },
        }
    }

    fn edits() -> PackageEdits {
        PackageEdits {
            schema: EDITS_SCHEMA.into(), free_agents: None,
            textures: vec![TextureOp { rel_path: "fe/x/logo.png".into(), texture_index: None, payload: "payloads/aa.png".into(), width: 1, height: 1, prior_len: None, prior_sha256: None, platform: "ps3".into() }],
            ..Default::default()
        }
    }

    #[test]
    fn explicit_pool_schema_and_identities_are_validated() {
        let mut m = manifest();
        m.game_id = "nhl07".into();
        m.author_platform = "xbox".into();
        m.categories = vec![CategoryCount { category: "cells".into(), count: 1 }];
        let mut e = PackageEdits { schema: EDITS_SCHEMA.into(), free_agents: Some(FreeAgentPoolOp { before: vec![1, 2], after: vec![2, 3] }), ..Default::default() };
        let check = |m: PackageManifest, e: &PackageEdits| inspect_package_bytes(build_package(m, e, &[]).unwrap(), &Limits::default());
        assert!(check(m.clone(), &e).is_ok());
        e.schema = EDITS_SCHEMA_V2.into();
        assert!(check(m.clone(), &e).is_err());
        e.schema = EDITS_SCHEMA.into();
        e.free_agents.as_mut().unwrap().after = vec![3, 3];
        assert!(check(m.clone(), &e).is_err());
        e.free_agents.as_mut().unwrap().after = vec![3];
        m.game_id = "nhl08".into();
        assert!(check(m, &e).is_err());
        // Existing schema-2 packages continue to inspect without a rebuild.
        let p = vec![("payloads/aa.png".into(), b"PNG".to_vec())];
        let mut e = edits(); e.schema = EDITS_SCHEMA_V2.into();
        assert!(inspect_package_bytes(build_package(manifest(), &e, &p).unwrap(), &Limits::default()).is_ok());
    }

    #[test]
    fn schedule_schema_is_scoped_and_rejects_duplicates() {
        let mut m=manifest();m.game_id="nhl07".into();m.author_platform="xbox".into();
        m.categories=vec![CategoryCount{category:"cells".into(),count:1}];
        let row=crate::schedule_edit::GameRow{index:0,month:9,day:0,home_slot:0,away_slot:1,round:0,status:Some(0)};
        let mut e=PackageEdits{schema:EDITS_SCHEMA.into(),schedules:vec![ScheduleOp{table:"ksGb".into(),before:vec![row.clone()],after:vec![]}],..Default::default()};
        let inspect=|m:PackageManifest,e:&PackageEdits|inspect_package_bytes(build_package(m,e,&[]).unwrap(),&Limits::default());
        assert!(inspect(m.clone(),&e).is_ok());
        e.schema=EDITS_SCHEMA_V3.into();assert!(inspect(m.clone(),&e).is_err());e.schema=EDITS_SCHEMA.into();
        e.schedules.push(e.schedules[0].clone());assert!(inspect(m.clone(),&e).is_err());e.schedules.pop();
        e.schedules[0].table="qnvy".into();assert!(inspect(m.clone(),&e).is_err());e.schedules[0].table="ksGb".into();
        m.game_id="nhl08".into();assert!(inspect(m,&e).is_err());
    }

    #[test]
    fn round_trip_is_deterministic_and_validated() {
        let p = vec![("payloads/aa.png".to_string(), b"PNGBYTES".to_vec())];
        let a = build_package(manifest(), &edits(), &p).unwrap();
        let b = build_package(manifest(), &edits(), &p).unwrap();
        assert_eq!(a, b, "same inputs, same bytes");
        let ins = inspect_package_bytes(a.clone(), &Limits::default()).unwrap();
        assert_eq!(ins.manifest.entries.len(), 2);
        assert_eq!(ins.payload("payloads/aa.png").unwrap(), b"PNGBYTES");
        assert_eq!(ins.edits.textures.len(), 1);
        // One flipped byte in a payload: refused.
        let mut t = a.clone();
        let off = ins.archive.entry("payloads/aa.png").unwrap().data_offset;
        t[off] ^= 0xFF;
        let err = inspect_package_bytes(t, &Limits::default()).unwrap_err().to_string();
        assert!(err.contains("CRC") || err.contains("hash"), "{err}");
        // A truncated file: refused.
        let err = inspect_package_bytes(a[..a.len() - 5].to_vec(), &Limits::default()).unwrap_err().to_string();
        assert!(err.contains("end-of-central-directory") || err.contains("outside"), "{err}");
    }

    #[test]
    fn unsafe_names_and_collisions_are_refused() {
        for bad in ["../x", "a/../b", "/abs", "C:/x", "a\\b", "con.png", "a/", "x?y", "payloads/A.png\u{7}"] {
            assert!(check_entry_name(bad).is_err(), "{bad}");
        }
        check_entry_name("payloads/ab.png").unwrap();
        let p = vec![("payloads/aa.png".to_string(), b"1".to_vec()), ("payloads/AA.png".to_string(), b"2".to_vec())];
        let bytes = write_zip(&p).unwrap();
        let err = ZipArchive::parse(bytes, &Limits::default()).unwrap_err().to_string();
        assert!(err.contains("case"), "{err}");
        let p = vec![("manifest.json".to_string(), b"{}".to_vec()); 3];
        let err = ZipArchive::parse(write_zip(&p).unwrap(), &Limits { max_entries: 2, ..Limits::default() }).unwrap_err().to_string();
        assert!(err.contains("entries exceeds"), "{err}");
    }

    #[test]
    fn an_unlisted_entry_or_wrong_schema_is_refused() {
        let p = vec![("payloads/aa.png".to_string(), b"PNGBYTES".to_vec()), ("payloads/extra.bin".to_string(), b"X".to_vec())];
        // Build with one payload, then splice a second: the manifest will not list it.
        let mut m = manifest();
        m.entries = vec![];
        let edits_json = serde_json::to_vec_pretty(&edits()).unwrap();
        let mut entries: Vec<(String, Vec<u8>)> = vec![(EDITS_ENTRY.into(), edits_json.clone())];
        entries.push(p[0].clone());
        let mut records = vec![];
        for (name, bytes) in &entries {
            records.push(EntryRecord { path: name.clone(), len: bytes.len() as u64, sha256: sha256_hex(bytes), kind: if name == EDITS_ENTRY { "edits".into() } else { "payload".into() } });
        }
        m.entries = records;
        m.package_id = compute_package_id(&m);
        entries.push(p[1].clone());
        entries.push((MANIFEST_ENTRY.into(), serde_json::to_vec_pretty(&m).unwrap()));
        let bytes = write_zip(&entries).unwrap();
        let err = inspect_package_bytes(bytes, &Limits::default()).unwrap_err().to_string();
        assert!(err.contains("not listed"), "{err}");
        let mut m2 = manifest();
        m2.schema = "nhlms-modpkg/9".into();
        let bytes = write_zip(&[(MANIFEST_ENTRY.to_string(), serde_json::to_vec(&m2).unwrap())]).unwrap();
        let err = inspect_package_bytes(bytes, &Limits::default()).unwrap_err().to_string();
        assert!(err.contains("schema"), "{err}");
    }
}
