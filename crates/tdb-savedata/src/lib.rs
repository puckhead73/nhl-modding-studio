//! `tdb-savedata` — the PS3/Xbox360 **save-container** codec (EDIT-MODEL save model).
//!
//! A save payload (`SYS-DATA`) wraps the inner TDB in a versioned container. This crate parses the
//! header, classifies the save type, **extracts** the inner `.db`, and **repacks** an edited `.db`
//! back — recomputing the two verified container checksums. All facts here are confirmed against the
//! committed anchors (`docs/formats/ps3-save-container.md`, `tdb-v6-format-spec.md §4`).
//!
//! ## Scope (this increment)
//!
//! * **Read:** v3 (gen6 NHL07-09 ROSTER), v1 (NHL10/11), v2 (NHL12/13), v4 (NHL14/15/Legacy)
//!   headers; save-type taxonomy; inner-DB extraction (raw for v3/v1/v2, zlib-inflate for v4).
//! * **Write:** v3 (gen6, tight, single `checksum_a`), v2 (raw, tight) and v4 (zlib + zero-pad)
//!   repack with checksum recompute. **v1 (NHL10/11) is writeable but experimental** — its `tag_1c`
//!   integrity word is un-reversed, so an edited body carries a stale tag (preserved verbatim);
//!   `checksum_a` is recomputed and a no-op repack is byte-identical, but in-game load is unverified.
//!
//! ## Not implemented (deferred / out of scope, marked here so callers don't assume otherwise)
//!
//! * **No PFD/AES.** Operates only on already-decrypted (RPCS3-extracted) `SYS-DATA` bytes.
//! * **No `DATA000N` multi-part reassembly.** Single-file saves only (ROSTER/SETTINGS/HOCKEYCARD, and
//!   pre-NHL13 dynasty/season). Multi-part dynasty/season saves are net-new/unverified.
//! * **v1 write** (un-reversed `tag_1c`) and the DB-internal Huffman varchar codec live elsewhere.

pub mod checksum;
pub mod extract;
pub mod header;
pub mod pack;
pub mod ps4;

pub use extract::extract_db;
pub use header::{ContainerVersion, SaveHeader, SaveType};
pub use pack::repack_db;
pub use ps4::{extract_roster as ps4_extract_roster, is_ps4_save, repack_roster as ps4_repack_roster};

/// Errors from the save-container codec.
#[derive(Debug)]
pub enum SaveError {
    /// Buffer too short to hold a container header.
    TooShort,
    /// No recognisable inner TDB / container body.
    NotTdb,
    /// A `version` word the discriminator does not handle.
    UnsupportedVersion(u32),
    /// v1 (NHL10/11) containers are read-only (`tag_1c` un-reversed).
    UnsupportedV1Write,
    /// Inflating the v4 zlib body failed (or contradicted the declared size).
    Inflate(String),
    /// Deflating the new body failed.
    Deflate(String),
    /// The recompressed body no longer fits the original file allocation.
    DeflatedTooLarge { needed: usize, allocated: usize },
    /// A PS4 `DATA` save stream was malformed (framing / no roster record).
    Ps4(String),
    /// LZ4 frame decode/encode failed (PS4 roster payload).
    Lz4(String),
}

impl std::fmt::Display for SaveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SaveError::TooShort => write!(f, "buffer too short for a save container header"),
            SaveError::NotTdb => write!(f, "no recognisable inner TDB / container body"),
            SaveError::UnsupportedVersion(v) => write!(f, "unsupported container version word {v}"),
            SaveError::UnsupportedV1Write => {
                write!(f, "v1 (NHL10/11) containers are read-only (tag_1c un-reversed)")
            }
            SaveError::Inflate(e) => write!(f, "zlib inflate failed: {e}"),
            SaveError::Deflate(e) => write!(f, "zlib deflate failed: {e}"),
            SaveError::DeflatedTooLarge { needed, allocated } => write!(
                f,
                "recompressed body needs {needed} bytes but only {allocated} are allocated"
            ),
            SaveError::Ps4(msg) => write!(f, "PS4 save: {msg}"),
            SaveError::Lz4(msg) => write!(f, "PS4 LZ4 codec: {msg}"),
        }
    }
}
impl std::error::Error for SaveError {}

/// A parsed save container over borrowed bytes — the high-level entry point.
pub struct SaveContainer<'a> {
    bytes: &'a [u8],
    pub header: SaveHeader,
}

impl<'a> SaveContainer<'a> {
    /// Parse a container's header (does not yet extract the body).
    pub fn parse(bytes: &'a [u8]) -> Result<Self, SaveError> {
        let header = SaveHeader::parse(bytes)?;
        Ok(SaveContainer { bytes, header })
    }

    /// `true` iff both stored checksums match a fresh computation over the current bytes.
    /// (v1 has only `checksum_a`.)
    pub fn checksums_valid(&self) -> bool {
        let a_ok = self.header.checksum_a == checksum::checksum_a(self.bytes);
        let b_ok = match self.header.checksum_b {
            Some(b) => b == checksum::checksum_b(self.bytes),
            None => true,
        };
        a_ok && b_ok
    }

    /// Extract the inner `.db` bytes.
    pub fn extract_db(&self) -> Result<Vec<u8>, SaveError> {
        extract_db(self.bytes)
    }

    /// Repack `new_db` into this container's shape, returning the new file bytes.
    pub fn repack(&self, new_db: &[u8]) -> Result<Vec<u8>, SaveError> {
        repack_db(self.bytes, new_db)
    }

    /// `true` iff this container variant supports the write path.
    pub fn writeable(&self) -> bool {
        self.header.version.writeable()
    }
}
