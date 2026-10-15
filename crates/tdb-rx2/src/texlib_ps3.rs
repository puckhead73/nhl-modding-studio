//! PS3 RenderWare 4 texture container (`\x89RW4ps3` — the NHL 08–11 PS3
//! league-logo packages and the NHL 11–Legacy `rendering/**/*.rpsgl` texlibs).
//!
//! Sibling of [`crate::texlib`] (Xbox 360 `RW4xb2`): the same header words and
//! 24-byte file-table entries, but the pixels are stored **plain** — row-major,
//! no tiling, no 16-bit byte swap — so a slot is a linear `[A,R,G,B]` stream or
//! a little-endian DXT5 block stream with its mip levels concatenated base-first.
//! Layout per research handoff 18 (`docs/research/overnight-2026-09-18/
//! 18-rw4ps3-texlib-layout.md`), verified pixel-for-pixel against the Xbox decode
//! of the same art on 36 league logos and two rink texlibs:
//!
//! ```text
//!  0x00  "\x89RW4ps3"              magic
//!  0x20  file_count      u32 BE
//!  0x30  file_table_off  u32 BE
//!  0x44  hdr_size        u32 BE    pixel region starts here
//!  file table: 24-byte entries = 5 × u32 BE + 4-byte tag
//!    00 01 00 30  data slot:  ints[0] = offset relative to hdr_size, ints[2] = bytes
//!    00 02 00 03  descriptor: ints[0] = absolute offset, ints[2] = 0x38
//!    00 EC 00 10  name table (`*.Raster`, NUL-joined) — rink texlibs only
//!  descriptor (0x38 bytes):
//!    +32..43  format signature words: 0x6007 0x6007 0x8035 → ARGB8;
//!                                     0x83F3 0x83F3 0xFFFF → DXT5
//!    +48 u16 BE width, +50 u16 BE height, +52 u8 mip count
//! ```
//!
//! Only the two observed signatures are decoded; anything else is refused
//! rather than guessed (DXT1/DXT3 were not observed on PS3). The slot length
//! must equal the format's byte size for the declared dimensions and mips
//! (`w*h*4`, or the DXT5 chain sum with a 16-byte floor per level) — that
//! check is what distinguishes a plain texture from an unmeasured swizzled
//! one. All constant descriptor words are opaque and copied through untouched
//! by the same-dimension writer.

use std::convert::TryInto;

use thiserror::Error;

/// 7-byte magic at offset 0 of every PS3 RW4 container.
pub const MAGIC: &[u8; 7] = b"\x89RW4ps3";

const HDR_FILE_COUNT_OFF: usize = 0x20;
const HDR_FILE_TABLE_OFF: usize = 0x30;
const HDR_HDR_SIZE_OFF: usize = 0x44;

const ENTRY_STRIDE: usize = 24;
const TAG_TEX_DATA: [u8; 4] = [0x00, 0x01, 0x00, 0x30];
const TAG_TEX_DESC: [u8; 4] = [0x00, 0x02, 0x00, 0x03];
const TAG_NAME_TABLE: [u8; 4] = [0x00, 0xec, 0x00, 0x10];

const DESC_SIZE: usize = 0x38;
const DESC_SIG_OFF: usize = 32;
const DESC_WIDTH_OFF: usize = 48;
const DESC_HEIGHT_OFF: usize = 50;
const DESC_MIPS_OFF: usize = 52;

/// The two pixel layouts observed on PS3 texlibs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ps3Format {
    /// Linear 32-bit `[A,R,G,B]` texels, row-major (signature `6007 6007 8035`).
    Argb8,
    /// Linear little-endian DXT5 blocks, mip levels concatenated base-first
    /// (signature `83F3 83F3 FFFF`).
    Dxt5,
    /// Linear DXT1 blocks, 8 bytes each (signature `83F1 83F1 FFFF`; research 2026-09-21
    /// handoff 11 §1 — chain-size agreement on 28 681 / 28 753 Legacy descriptors).
    Dxt1,
    /// Linear DXT3 blocks, 16 bytes each (signature `83F2 83F2 FFFF`; same evidence).
    Dxt3,
}

impl Ps3Format {
    fn from_signature(sig: [u32; 3]) -> Option<Self> {
        match sig {
            [0x6007, 0x6007, 0x8035] => Some(Self::Argb8),
            [0x83F3, 0x83F3, 0xFFFF] => Some(Self::Dxt5),
            [0x83F1, 0x83F1, 0xFFFF] => Some(Self::Dxt1),
            [0x83F2, 0x83F2, 0xFFFF] => Some(Self::Dxt3),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Argb8 => "ARGB8 (linear)",
            Self::Dxt5 => "DXT5 (linear)",
            Self::Dxt1 => "DXT1 (linear)",
            Self::Dxt3 => "DXT3 (linear)",
        }
    }
}

#[derive(Debug, Error)]
pub enum TexlibPs3Error {
    #[error("not a RW4ps3 container (bad magic)")]
    BadMagic,
    #[error("truncated container: need {need} bytes, have {have}")]
    Truncated { need: usize, have: usize },
    #[error("file table entry {idx} is out of range")]
    EntryOutOfRange { idx: usize },
    #[error("texture {idx}: {count_slots} data slots but {count_descs} descriptors")]
    Unpaired {
        idx: usize,
        count_slots: usize,
        count_descs: usize,
    },
    #[error("texture {idx}: unknown pixel-format signature {sig:08x?} (only ARGB8 6007/8035 and DXT5 83F3/FFFF are decoded)")]
    UnknownFormat { idx: usize, sig: [u32; 3] },
    #[error("texture {idx}: slot is {slot} bytes but {format} {width}x{height} with {mips} mip(s) needs {expected} (not a plain layout)")]
    SlotLengthMismatch {
        idx: usize,
        slot: usize,
        format: &'static str,
        width: u32,
        height: u32,
        mips: u8,
        expected: usize,
    },
    #[error("texture index {idx} out of range ({count} textures)")]
    TextureIndexOutOfRange { idx: usize, count: usize },
    #[error("replacement is {have} bytes; the slot holds exactly {expected}")]
    ReplacementSizeMismatch { expected: usize, have: usize },
    #[error("resize refused: {0}")]
    ResizeRule(String),
}

/// One raster of the container, decoded from its descriptor.
#[derive(Debug, Clone)]
pub struct Ps3Texture {
    /// `*.Raster` name from the name table when the container carries one,
    /// else `raster_<n>`.
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub mips: u8,
    pub format: Ps3Format,
    /// Absolute offset of the pixel slot and its byte length.
    slot_abs: usize,
    slot_len: usize,
    /// Absolute offset of the 0x38-byte descriptor.
    desc_off: usize,
}

impl Ps3Texture {
    pub fn descriptor_offset(&self) -> usize {
        self.desc_off
    }

    pub fn slot_len(&self) -> usize {
        self.slot_len
    }

    /// Byte length of each mip level in slot order (base first).
    pub fn mip_sizes(&self) -> Vec<usize> {
        mip_chain_sizes(self.format, self.width, self.height, self.mips)
    }
}

/// Byte length of each level of a plain PS3 mip chain: ARGB8 is `w*h*4` per
/// level; DXT5 is 16 bytes per 4×4 block with a 16-byte floor (research 18 §2:
/// the sum of levels equals the slot length on every measured raster).
pub fn mip_chain_sizes(format: Ps3Format, width: u32, height: u32, mips: u8) -> Vec<usize> {
    (0..mips.max(1) as u32)
        .map(|level| {
            let w = (width >> level).max(1) as usize;
            let h = (height >> level).max(1) as usize;
            match format {
                Ps3Format::Argb8 => w * h * 4,
                Ps3Format::Dxt5 | Ps3Format::Dxt3 => (w.div_ceil(4) * h.div_ceil(4) * 16).max(16),
                Ps3Format::Dxt1 => (w.div_ceil(4) * h.div_ceil(4) * 8).max(8),
            }
        })
        .collect()
}

/// Parsed container: the original bytes plus every raster's placement.
#[derive(Debug, Clone)]
pub struct TexlibPs3 {
    pub textures: Vec<Ps3Texture>,
    raw: Vec<u8>,
    pub hdr_size: u32,
}

fn be32(data: &[u8], off: usize) -> Result<u32, TexlibPs3Error> {
    data.get(off..off + 4)
        .map(|b| u32::from_be_bytes(b.try_into().unwrap()))
        .ok_or(TexlibPs3Error::Truncated {
            need: off + 4,
            have: data.len(),
        })
}

fn be16(data: &[u8], off: usize) -> Result<u16, TexlibPs3Error> {
    data.get(off..off + 2)
        .map(|b| u16::from_be_bytes(b.try_into().unwrap()))
        .ok_or(TexlibPs3Error::Truncated {
            need: off + 2,
            have: data.len(),
        })
}

/// Parse a `\x89RW4ps3` container. Data slots are paired with descriptors in
/// file-table order (the n-th slot with the n-th descriptor); the slot-length
/// check then confirms each pairing.
pub fn parse(data: &[u8]) -> Result<TexlibPs3, TexlibPs3Error> {
    if data.len() < 7 || &data[..7] != MAGIC {
        return Err(TexlibPs3Error::BadMagic);
    }
    let file_count = be32(data, HDR_FILE_COUNT_OFF)? as usize;
    let table_off = be32(data, HDR_FILE_TABLE_OFF)? as usize;
    let hdr_size = be32(data, HDR_HDR_SIZE_OFF)?;

    let mut slots: Vec<(usize, usize)> = Vec::new();
    let mut descs: Vec<usize> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for i in 0..file_count {
        let e = table_off + i * ENTRY_STRIDE;
        let entry = data
            .get(e..e + ENTRY_STRIDE)
            .ok_or(TexlibPs3Error::EntryOutOfRange { idx: i })?;
        let ints: Vec<u32> = (0..5)
            .map(|k| u32::from_be_bytes(entry[k * 4..k * 4 + 4].try_into().unwrap()))
            .collect();
        let tag: [u8; 4] = entry[20..24].try_into().unwrap();
        if tag == TAG_TEX_DATA {
            slots.push((hdr_size as usize + ints[0] as usize, ints[2] as usize));
        } else if tag == TAG_TEX_DESC {
            descs.push(ints[0] as usize);
        } else if tag == TAG_NAME_TABLE {
            let (off, len) = (ints[0] as usize, ints[2] as usize);
            if let Some(blob) = data.get(off..off + len) {
                // Early bundles include binary fields before the raster names.
                // Do not turn those nonzero bytes into bogus texture labels.
                names = crate::texlib::scan_names(blob, 0);
            }
        }
    }
    if slots.len() != descs.len() {
        return Err(TexlibPs3Error::Unpaired {
            idx: 0,
            count_slots: slots.len(),
            count_descs: descs.len(),
        });
    }

    let mut textures = Vec::with_capacity(slots.len());
    for (idx, ((slot_abs, slot_len), desc_off)) in slots.into_iter().zip(descs).enumerate() {
        if desc_off + DESC_SIZE > data.len() {
            return Err(TexlibPs3Error::Truncated {
                need: desc_off + DESC_SIZE,
                have: data.len(),
            });
        }
        if slot_abs + slot_len > data.len() {
            return Err(TexlibPs3Error::Truncated {
                need: slot_abs + slot_len,
                have: data.len(),
            });
        }
        let sig = [
            be32(data, desc_off + DESC_SIG_OFF)?,
            be32(data, desc_off + DESC_SIG_OFF + 4)?,
            be32(data, desc_off + DESC_SIG_OFF + 8)?,
        ];
        let format = Ps3Format::from_signature(sig)
            .ok_or(TexlibPs3Error::UnknownFormat { idx, sig })?;
        let width = be16(data, desc_off + DESC_WIDTH_OFF)? as u32;
        let height = be16(data, desc_off + DESC_HEIGHT_OFF)? as u32;
        let mips = data[desc_off + DESC_MIPS_OFF];
        let expected: usize = mip_chain_sizes(format, width, height, mips).iter().sum();
        if expected != slot_len || width == 0 || height == 0 {
            return Err(TexlibPs3Error::SlotLengthMismatch {
                idx,
                slot: slot_len,
                format: format.label(),
                width,
                height,
                mips,
                expected,
            });
        }
        let name = names
            .get(idx)
            .cloned()
            .unwrap_or_else(|| format!("raster_{idx}"));
        textures.push(Ps3Texture {
            name,
            width,
            height,
            mips,
            format,
            slot_abs,
            slot_len,
            desc_off,
        });
    }
    Ok(TexlibPs3 {
        textures,
        raw: data.to_vec(),
        hdr_size,
    })
}

impl TexlibPs3 {
    pub fn original_bytes(&self) -> &[u8] {
        &self.raw
    }

    /// The whole pixel slot of texture `idx` (base mip followed by the smaller levels).
    pub fn slot_bytes(&self, idx: usize) -> Option<&[u8]> {
        let t = self.textures.get(idx)?;
        self.raw.get(t.slot_abs..t.slot_abs + t.slot_len)
    }

    /// The base mip of texture `idx`, already linear: `[A,R,G,B]` texels or LE DXT5 blocks.
    pub fn base_mip_linear(&self, idx: usize) -> Option<&[u8]> {
        let t = self.textures.get(idx)?;
        let base = t.mip_sizes()[0];
        self.raw.get(t.slot_abs..t.slot_abs + base)
    }

    /// Replace texture `idx`'s whole slot with `chain` — the plain base level
    /// followed by each smaller level, exactly `slot_len` bytes (ARGB8 texels
    /// or LE DXT5 blocks). Header, file table, descriptor and every other slot
    /// are copied through untouched; the container length never changes.
    pub fn replace_slot(&self, idx: usize, chain: &[u8]) -> Result<Vec<u8>, TexlibPs3Error> {
        let t = self
            .textures
            .get(idx)
            .ok_or(TexlibPs3Error::TextureIndexOutOfRange {
                idx,
                count: self.textures.len(),
            })?;
        if chain.len() != t.slot_len {
            return Err(TexlibPs3Error::ReplacementSizeMismatch {
                expected: t.slot_len,
                have: chain.len(),
            });
        }
        let mut out = self.raw.clone();
        out[t.slot_abs..t.slot_abs + t.slot_len].copy_from_slice(chain);
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A minimal two-entry container in the logo layout: header 0x200, one
    /// ARGB8 4×2 slot, one descriptor at 0x17C, file table at 0x1B4.
    fn synth(format: Ps3Format, w: u16, h: u16, mips: u8, slot: &[u8]) -> Vec<u8> {
        let hdr = 0x200usize;
        let mut b = vec![0u8; hdr + slot.len()];
        b[..7].copy_from_slice(MAGIC);
        b[HDR_FILE_COUNT_OFF..HDR_FILE_COUNT_OFF + 4].copy_from_slice(&2u32.to_be_bytes());
        b[HDR_FILE_TABLE_OFF..HDR_FILE_TABLE_OFF + 4].copy_from_slice(&0x1B4u32.to_be_bytes());
        b[HDR_HDR_SIZE_OFF..HDR_HDR_SIZE_OFF + 4].copy_from_slice(&(hdr as u32).to_be_bytes());
        let d = 0x17C;
        let sig: [u32; 3] = match format {
            Ps3Format::Argb8 => [0x6007, 0x6007, 0x8035],
            Ps3Format::Dxt5 => [0x83F3, 0x83F3, 0xFFFF],
            Ps3Format::Dxt1 => [0x83F1, 0x83F1, 0xFFFF],
            Ps3Format::Dxt3 => [0x83F2, 0x83F2, 0xFFFF],
        };
        for (k, s) in sig.iter().enumerate() {
            b[d + DESC_SIG_OFF + k * 4..d + DESC_SIG_OFF + k * 4 + 4].copy_from_slice(&s.to_be_bytes());
        }
        b[d + DESC_WIDTH_OFF..d + DESC_WIDTH_OFF + 2].copy_from_slice(&w.to_be_bytes());
        b[d + DESC_HEIGHT_OFF..d + DESC_HEIGHT_OFF + 2].copy_from_slice(&h.to_be_bytes());
        b[d + DESC_MIPS_OFF] = mips;
        let t = 0x1B4;
        let e0 = [0u32, 0, slot.len() as u32, 0x80, 1];
        let e1 = [d as u32, 0, 0x38, 4, 0xA];
        for (k, v) in e0.iter().enumerate() {
            b[t + k * 4..t + k * 4 + 4].copy_from_slice(&v.to_be_bytes());
        }
        b[t + 20..t + 24].copy_from_slice(&TAG_TEX_DATA);
        for (k, v) in e1.iter().enumerate() {
            b[t + 24 + k * 4..t + 24 + k * 4 + 4].copy_from_slice(&v.to_be_bytes());
        }
        b[t + 44..t + 48].copy_from_slice(&TAG_TEX_DESC);
        b[hdr..].copy_from_slice(slot);
        b
    }

    #[test]
    fn parses_argb8_and_replaces_in_place() {
        let slot: Vec<u8> = (0..4 * 2 * 4).map(|i| i as u8).collect();
        let bytes = synth(Ps3Format::Argb8, 4, 2, 1, &slot);
        let lib = parse(&bytes).unwrap();
        assert_eq!(lib.textures.len(), 1);
        let t = &lib.textures[0];
        assert_eq!((t.width, t.height, t.mips, t.format), (4, 2, 1, Ps3Format::Argb8));
        assert_eq!(lib.base_mip_linear(0).unwrap(), &slot[..]);
        let new: Vec<u8> = slot.iter().map(|b| b ^ 0xFF).collect();
        let out = lib.replace_slot(0, &new).unwrap();
        assert_eq!(out.len(), bytes.len());
        assert_eq!(&out[..0x200], &bytes[..0x200], "header, table and descriptor untouched");
        assert_eq!(parse(&out).unwrap().base_mip_linear(0).unwrap(), &new[..]);
        assert!(matches!(
            lib.replace_slot(0, &new[..8]),
            Err(TexlibPs3Error::ReplacementSizeMismatch { expected: 32, have: 8 })
        ));
    }

    #[test]
    fn dxt5_chain_sizes_have_a_16_byte_floor_and_gate_the_slot() {
        assert_eq!(mip_chain_sizes(Ps3Format::Dxt5, 128, 64, 1), vec![8192]);
        assert_eq!(
            mip_chain_sizes(Ps3Format::Dxt5, 256, 128, 9),
            vec![32768, 8192, 2048, 512, 128, 32, 16, 16, 16]
        );
        assert_eq!(mip_chain_sizes(Ps3Format::Dxt5, 256, 128, 9).iter().sum::<usize>(), 43728);
        let ok = synth(Ps3Format::Dxt5, 8, 4, 1, &[0u8; 32]);
        assert_eq!(parse(&ok).unwrap().textures[0].format, Ps3Format::Dxt5);
        let short = synth(Ps3Format::Dxt5, 8, 8, 1, &[0u8; 32]);
        assert!(matches!(parse(&short), Err(TexlibPs3Error::SlotLengthMismatch { expected: 64, slot: 32, .. })));
    }

    #[test]
    fn unknown_signature_is_refused() {
        let mut b = synth(Ps3Format::Argb8, 4, 4, 1, &[0u8; 64]);
        b[0x17C + DESC_SIG_OFF + 3] = 0x01; // 0x6007 → 0x6001
        assert!(matches!(parse(&b), Err(TexlibPs3Error::UnknownFormat { .. })));
        assert!(matches!(parse(b"\x89RW4xb2\0"), Err(TexlibPs3Error::BadMagic)));
    }
}

// ----------------------------------------------------------- P2-B resize (research handoff 11 §1)

const DESC_FLAGS_OFF: usize = 24; // word 6
const DESC_FLAG_CUBE: u32 = 0x1000;
const HDR_DATA_SIZE_OFF: usize = 0x54;

impl TexlibPs3 {
    /// Resize texture `idx` to `width × height` with `mips` levels and the plain linear `chain`
    /// (base level first; every level's bytes per [`mip_chain_sizes`]) — research 2026-09-21
    /// handoff 11 §1: a PS3 resize is descriptor word 12 (`width << 16 | height`) and word 13
    /// (`mips << 24`) plus the re-sized linear slot; every other descriptor word, the header, the
    /// name table and the other slots are kept, the slots after the target shift by the length
    /// delta with their file-table offsets, and the target's file-table length and the header's
    /// data size follow. Cube maps (word 6 bit `0x1000`, slot = 6 × the chain) are refused.
    /// Runtime acceptance of a changed size is a card (R33-PS3-1).
    pub fn replace_slot_resized(&self, idx: usize, width: u32, height: u32, mips: u8, chain: &[u8]) -> Result<Vec<u8>, TexlibPs3Error> {
        let t = self.textures.get(idx).ok_or(TexlibPs3Error::TextureIndexOutOfRange { idx, count: self.textures.len() })?;
        // Representability: words 12 / 13 hold 16-bit dimensions and a one-byte level count; a
        // chain cannot have more levels than the size halves to (audited under coordinator
        // review R1 F1, 21 Sep 2026 — the 16-bit gate was already here; the level bound and the
        // u32 length checks below are the audit's additions).
        if width == 0 || height == 0 || width > 0xFFFF || height > 0xFFFF || mips == 0 {
            return Err(TexlibPs3Error::ResizeRule(format!("{width}x{height} with {mips} mips is not a valid raster size (1–65535 texels, at least one level)")));
        }
        let halvings = 32 - width.max(height).leading_zeros();
        if mips as u32 > halvings {
            return Err(TexlibPs3Error::ResizeRule(format!("{mips} mips from {width}x{height}: the size halves to 1x1 in {halvings} levels")));
        }
        let flags = be32(&self.raw, t.desc_off + DESC_FLAGS_OFF)?;
        if flags & DESC_FLAG_CUBE != 0 {
            return Err(TexlibPs3Error::ResizeRule("the raster is a cube map (word 6 bit 0x1000); a 2D resize does not apply".into()));
        }
        let expected: usize = mip_chain_sizes(t.format, width, height, mips).iter().sum();
        if chain.len() != expected {
            return Err(TexlibPs3Error::ReplacementSizeMismatch { expected, have: chain.len() });
        }
        let new_len = self.raw.len() - t.slot_len + chain.len();
        if u32::try_from(chain.len()).is_err() || u32::try_from(new_len).is_err() {
            return Err(TexlibPs3Error::ResizeRule(format!("a {}-byte slot in a {new_len}-byte container exceeds the 32-bit file table", chain.len())));
        }
        let delta = chain.len() as i64 - t.slot_len as i64;
        let old_end = t.slot_abs + t.slot_len;
        let mut out = Vec::with_capacity((self.raw.len() as i64 + delta) as usize);
        out.extend_from_slice(&self.raw[..t.slot_abs]);
        out.extend_from_slice(chain);
        out.extend_from_slice(&self.raw[old_end..]);
        // Descriptor: word 12 / 13.
        let d = t.desc_off;
        out[d + DESC_WIDTH_OFF..d + DESC_WIDTH_OFF + 2].copy_from_slice(&(width as u16).to_be_bytes());
        out[d + DESC_HEIGHT_OFF..d + DESC_HEIGHT_OFF + 2].copy_from_slice(&(height as u16).to_be_bytes());
        out[d + DESC_MIPS_OFF] = mips;
        // File table: the target's length; every data slot after the target shifts by `delta`.
        let file_count = be32(&self.raw, HDR_FILE_COUNT_OFF)? as usize;
        let table_off = be32(&self.raw, HDR_FILE_TABLE_OFF)? as usize;
        let hdr = self.hdr_size as usize;
        for i in 0..file_count {
            let e = table_off + i * ENTRY_STRIDE;
            let tag: [u8; 4] = self.raw[e + 20..e + 24].try_into().unwrap();
            if tag != TAG_TEX_DATA {
                continue;
            }
            let rel = be32(&self.raw, e)? as usize;
            let len = be32(&self.raw, e + 8)? as usize;
            if hdr + rel == t.slot_abs && len == t.slot_len {
                out[e + 8..e + 12].copy_from_slice(&(chain.len() as u32).to_be_bytes());
            } else if hdr + rel >= old_end {
                let new_rel = (rel as i64 + delta) as u32;
                out[e..e + 4].copy_from_slice(&new_rel.to_be_bytes());
            }
        }
        // Header data size (the byte count after the header) when the file carries it.
        if let Ok(ds) = be32(&self.raw, HDR_DATA_SIZE_OFF) {
            if ds as usize == self.raw.len() - hdr {
                let new_ds = (out.len() - hdr) as u32;
                out[HDR_DATA_SIZE_OFF..HDR_DATA_SIZE_OFF + 4].copy_from_slice(&new_ds.to_be_bytes());
            }
        }
        // Read the result back: the target must parse at the requested size and level count.
        let re = parse(&out)?;
        match re.textures.get(idx) {
            Some(r) if (r.width, r.height, r.mips, r.slot_len) == (width, height, mips, chain.len()) => Ok(out),
            Some(r) => Err(TexlibPs3Error::ResizeRule(format!("the rewritten container reads back as {}x{} / {} mips / {} bytes, not the requested {width}x{height} / {mips} / {}", r.width, r.height, r.mips, r.slot_len, chain.len()))),
            None => Err(TexlibPs3Error::ResizeRule(format!("the rewritten container no longer lists raster {idx}"))),
        }
    }
}

#[cfg(test)]
mod resize_tests {
    use super::*;

    /// Two DXT5 rasters (8 × 8, 2 mips; 4 × 4, 1 mip) in one synthetic container: resizing the
    /// first to 16 × 16 / 3 mips grows its slot, shifts the second slot and its table offset,
    /// rewrites words 12 / 13, keeps the second raster's bytes, and re-parses; a wrong chain
    /// length and a cube flag are refused.
    #[test]
    fn resize_reflows_the_following_slot_and_rewrites_words_12_and_13() {
        let hdr = 0x200usize;
        let c0: Vec<u8> = (0..(64 + 16)).map(|i| i as u8).collect(); // 8×8 DXT5 (4 blocks) + 4×4 (1 block) = 80
        let c1: Vec<u8> = vec![0xEE; 16];
        let mut b = vec![0u8; hdr + c0.len() + c1.len()];
        b[..7].copy_from_slice(MAGIC);
        b[HDR_FILE_COUNT_OFF..HDR_FILE_COUNT_OFF + 4].copy_from_slice(&4u32.to_be_bytes());
        b[HDR_FILE_TABLE_OFF..HDR_FILE_TABLE_OFF + 4].copy_from_slice(&0x180u32.to_be_bytes());
        b[HDR_HDR_SIZE_OFF..HDR_HDR_SIZE_OFF + 4].copy_from_slice(&(hdr as u32).to_be_bytes());
        b[HDR_DATA_SIZE_OFF..HDR_DATA_SIZE_OFF + 4].copy_from_slice(&((c0.len() + c1.len()) as u32).to_be_bytes());
        let descs = [0x100usize, 0x140];
        for (k, (d, (w, h, m))) in descs.iter().zip([(8u16, 8u16, 2u8), (4, 4, 1)]).enumerate() {
            for (j, s) in [0x83F3u32, 0x83F3, 0xFFFF].iter().enumerate() {
                b[d + DESC_SIG_OFF + j * 4..d + DESC_SIG_OFF + j * 4 + 4].copy_from_slice(&s.to_be_bytes());
            }
            b[d + DESC_WIDTH_OFF..d + DESC_WIDTH_OFF + 2].copy_from_slice(&w.to_be_bytes());
            b[d + DESC_HEIGHT_OFF..d + DESC_HEIGHT_OFF + 2].copy_from_slice(&h.to_be_bytes());
            b[d + DESC_MIPS_OFF] = m;
            let _ = k;
        }
        b[hdr..hdr + c0.len()].copy_from_slice(&c0);
        b[hdr + c0.len()..].copy_from_slice(&c1);
        let entries: [(u32, u32, [u8; 4]); 4] = [(0, c0.len() as u32, TAG_TEX_DATA), (0x100, 0x38, TAG_TEX_DESC), (c0.len() as u32, c1.len() as u32, TAG_TEX_DATA), (0x140, 0x38, TAG_TEX_DESC)];
        for (i, (off, len, tag)) in entries.iter().enumerate() {
            let e = 0x180 + i * ENTRY_STRIDE;
            b[e..e + 4].copy_from_slice(&off.to_be_bytes());
            b[e + 8..e + 12].copy_from_slice(&len.to_be_bytes());
            b[e + 20..e + 24].copy_from_slice(tag);
        }
        let lib = parse(&b).unwrap();
        assert_eq!(lib.textures.len(), 2);
        let new_chain: Vec<u8> = (0..(256 + 64 + 16)).map(|i| (i * 3) as u8).collect(); // 16×16 + 8×8 + 4×4
        let out = lib.replace_slot_resized(0, 16, 16, 3, &new_chain).unwrap();
        let lib2 = parse(&out).unwrap();
        assert_eq!((lib2.textures[0].width, lib2.textures[0].height, lib2.textures[0].mips), (16, 16, 3));
        assert_eq!(lib2.slot_bytes(0).unwrap(), &new_chain[..]);
        assert_eq!(lib2.slot_bytes(1).unwrap(), &c1[..], "the following slot moved intact");
        assert_eq!((lib2.textures[1].width, lib2.textures[1].height, lib2.textures[1].mips), (4, 4, 1));
        assert_eq!(out.len(), hdr + new_chain.len() + c1.len());
        assert_eq!(be32(&out, HDR_DATA_SIZE_OFF).unwrap() as usize, new_chain.len() + c1.len());
        assert_eq!(&out[0x100 + 24..0x100 + 28], &b[0x100 + 24..0x100 + 28], "word 6 kept");
        assert!(lib.replace_slot_resized(0, 16, 16, 3, &new_chain[..100]).is_err());
        let mut cube = b.clone();
        cube[0x100 + DESC_FLAGS_OFF + 2] |= 0x10;
        let e = match parse(&cube).unwrap().replace_slot_resized(0, 16, 16, 3, &new_chain) {
            Ok(_) => panic!("cube refused"),
            Err(e) => e.to_string(),
        };
        assert!(e.contains("cube"), "{e}");
    }
}
