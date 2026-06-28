//! Save-container header: magic / save-type taxonomy, the v1/v2/v4 variant discriminator, and the
//! verified header field layout (spec §4). All fields big-endian **except `compressed_size` (LE)**.

use crate::SaveError;

/// 16-byte NUL-padded ASCII magic at offset 0.
pub const MAGIC_LEN: usize = 16;

/// The container variant — decides header length, body codec, and writeability.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContainerVersion {
    /// 32-byte header, raw TDB body. NHL10/11. **Read-only** (`tag_1c` un-reversed).
    RawV1,
    /// 44-byte header, raw TDB body written tight. NHL12/13.
    RawV2,
    /// 48-byte header, zlib body + zero pad. NHL14/15/Legacy.
    ZlibV4,
}

impl ContainerVersion {
    /// Byte offset where the body begins.
    pub fn header_len(self) -> usize {
        match self {
            ContainerVersion::RawV1 => 0x20,
            ContainerVersion::RawV2 => 0x2C,
            ContainerVersion::ZlibV4 => 0x30,
        }
    }

    /// Whether the write path supports this variant (v1 is read-only).
    pub fn writeable(self) -> bool {
        matches!(self, ContainerVersion::RawV2 | ContainerVersion::ZlibV4)
    }
}

/// The six save types per game, keyed off the container magic.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveType {
    Settings,
    BeAGm,
    Roster,
    BeAPro,
    HockeyCard,
    Season,
    /// A magic we don't recognise (kept rather than rejected).
    Unknown,
}

impl SaveType {
    /// Classify from the (NUL-trimmed) magic, tolerating the optional `PS3` prefix (Xbox360/PC drop
    /// it: `RosterFile`, `DynastyFile`, …).
    fn from_magic(magic: &str) -> Self {
        let core = magic.strip_prefix("PS3").unwrap_or(magic);
        match core {
            "UserProfileFile" => SaveType::Settings,
            "DynastyFile" => SaveType::BeAGm,
            "RosterFile" => SaveType::Roster,
            "BeAProFile" => SaveType::BeAPro,
            "HockeyCardFile" => SaveType::HockeyCard,
            "SeasonFile" => SaveType::Season,
            _ => SaveType::Unknown,
        }
    }
}

fn be32(b: &[u8], off: usize) -> u32 {
    u32::from_be_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// A parsed container header. Optional fields are present only on the variants that carry them.
#[derive(Clone, Debug)]
pub struct SaveHeader {
    pub magic: String,
    pub save_type: SaveType,
    pub version: ContainerVersion,
    /// The raw `version` byte-word @ `0x14` (unreliable on its own — see the discriminator).
    pub version_word: u32,
    pub checksum_a: u32,
    pub checksum_b: Option<u32>,
    pub uncompressed_size: Option<u32>,
    pub compressed_size: Option<u32>,
    /// `field_20` @ `0x20`. Preserved verbatim — the anchors show it is **not** a reliable platform
    /// marker (a real PS3 v4 save carries `1`, contradicting the "2=PS3" note), so we never derive
    /// platform from it.
    pub field_20: Option<u32>,
}

impl SaveHeader {
    /// Parse the container header, resolving the variant by **body marker**, not the version word.
    pub fn parse(file: &[u8]) -> Result<Self, SaveError> {
        if file.len() < 0x30 {
            return Err(SaveError::TooShort);
        }
        let magic_bytes = &file[..MAGIC_LEN];
        let n = magic_bytes.iter().position(|&b| b == 0).unwrap_or(MAGIC_LEN);
        let magic = String::from_utf8_lossy(&magic_bytes[..n]).to_string();
        let save_type = SaveType::from_magic(&magic);

        let checksum_a = be32(file, 0x10);
        let version_word = be32(file, 0x14);

        // Discriminator (spec §4): the version word is shared across raw/zlib, so disambiguate v2 by
        // looking for the zlib magic at 0x30 vs the "DB" marker at 0x2C.
        let zlib_at_30 = file.get(0x30) == Some(&0x78);
        let db_at_2c = file.get(0x2C..0x2E) == Some(b"DB");
        let version = match version_word {
            1 => ContainerVersion::RawV1,
            0 => ContainerVersion::RawV2,
            4 => ContainerVersion::ZlibV4,
            2 if zlib_at_30 => ContainerVersion::ZlibV4,
            2 if db_at_2c => ContainerVersion::RawV2,
            2 => return Err(SaveError::NotTdb),
            other => return Err(SaveError::UnsupportedVersion(other)),
        };

        let (checksum_b, uncompressed_size, compressed_size, field_20) = match version {
            ContainerVersion::RawV1 => (None, None, None, None),
            ContainerVersion::RawV2 => (
                Some(be32(file, 0x28)),
                Some(be32(file, 0x24)),
                None,
                Some(be32(file, 0x20)),
            ),
            ContainerVersion::ZlibV4 => (
                Some(be32(file, 0x28)),
                Some(be32(file, 0x24)),
                // compressed_size is the only LITTLE-endian field.
                Some(u32::from_le_bytes([
                    file[0x2C], file[0x2D], file[0x2E], file[0x2F],
                ])),
                Some(be32(file, 0x20)),
            ),
        };

        Ok(SaveHeader {
            magic,
            save_type,
            version,
            version_word,
            checksum_a,
            checksum_b,
            uncompressed_size,
            compressed_size,
            field_20,
        })
    }
}
