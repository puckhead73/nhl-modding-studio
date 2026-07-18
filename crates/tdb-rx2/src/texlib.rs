//! Xbox 360 RX2 texture container (`texlib_<id>.rx2`).
//!
//! Sibling to the mesh reader in [`crate`]: same `\x89RW4xb2` magic and
//! 24-byte file-table entries, but the entries we care about here are
//! texture-data + descriptor pairs rather than FXRS render-piece sets.
//!
//! Cross-checked against the operator's Noesis reader script
//! (`Tools/NHL-Legacy-Texture-Patcher/Noesis script (XBOX 360)/
//! tex_NHLLegacy_X360_rx2.py`) and a hex walk of `texlib_67.rx2`
//! (Sidney Crosby, head 512×512 DXT1 + hair 256×256 DXT3).
//!
//! ## Container layout summary
//!
//! ```text
//!  0x00      magic "\x89RW4xb2"
//!  0x20      file_count                  (u32 BE)
//!  0x30      file_table offset           (u32 BE) — usually 0x204
//!  0x44      hdr_size                    (u32 BE) — usually 0x1000
//!  0x54      data_size                   (u32 BE) — total pixel-data bytes
//!  ...
//!  hdr_size  pixel data region — tiled, 16-bit-pair byte-swapped DXT
//! ```
//!
//! File-table entries are 24 bytes, 5×u32 BE followed by a 4-byte tag:
//!
//! | Tag           | Meaning                                            |
//! | ------------- | -------------------------------------------------- |
//! | `00 01 00 31` | Texture data slot. `ints[0]`=offset relative to    |
//! |               | `hdr_size`, `ints[2]`=buffer bytes.                |
//! | `00 02 00 03` | Texture descriptor. `ints[0]`=absolute file off,   |
//! |               | `ints[2]`=desc size (typically 0x38).              |
//! | `00 EC 00 10` | Name table (ASCII `*.Raster` tokens, NUL-joined).  |
//!
//! Within a descriptor, the format/dimension fields live at fixed
//! offsets relative to `desc + 33` (matching the Noesis reader's
//! `>HBBBHiii` parse):
//!
//! ```text
//!  +33  H        ?
//!  +35  B        pixel format (0x52=DXT1, 0x53=DXT3, 0x54=DXT5)
//!  +36  B        ?
//!  +37  B        height-byte    →  height = (byte + 1) * 8
//!  +38  H        width-word     →  width  = (word + 1) & 0x1FFF
//!                                  (upper bits 0xE000 are format/pitch
//!                                  flags — preserved on round-trip)
//! ```
//!
//! Pixel bytes are stored Xbox-360-tiled (XGAddress2DTiledOffset) AND
//! 16-bit-pair byte-swapped (PowerPC big-endian storage of LE-blocks).
//! The base mip starts at offset 0 of each buffer; smaller mip levels
//! follow in tile-aligned chunks. See [`tile_dxt`] / [`untile_dxt`].

use std::convert::TryInto;

use thiserror::Error;

/// 7-byte magic at offset 0 of every RX2 container — shared with the
/// mesh reader.
pub const MAGIC: &[u8; 7] = b"\x89RW4xb2";

const HDR_FILE_COUNT_OFF: usize = 0x20;
const HDR_FILE_TABLE_OFF: usize = 0x30;
const HDR_HDR_SIZE_OFF: usize = 0x44;
const HDR_DATA_SIZE_OFF: usize = 0x54;

const ENTRY_STRIDE: usize = 24;
const ENTRY_INTS: usize = 5;

const TAG_TEX_DATA: [u8; 4] = [0x00, 0x01, 0x00, 0x31];
const TAG_TEX_DESC: [u8; 4] = [0x00, 0x02, 0x00, 0x03];
const TAG_NAME_TABLE: [u8; 4] = [0x00, 0xec, 0x00, 0x10];

const DESC_FIELD_OFF: usize = 33;
const DESC_FORMAT_BYTE_REL: usize = DESC_FIELD_OFF + 2;
const DESC_HEIGHT_BYTE_REL: usize = DESC_FIELD_OFF + 4;
const DESC_WIDTH_WORD_REL: usize = DESC_FIELD_OFF + 5;
const DESC_WIDTH_VALUE_MASK: u16 = 0x1FFF;

/// Pixel formats we understand. These map to the Xenos
/// `GPUTEXTUREFORMAT_*` byte stored at `desc + 35`: the DXT family plus
/// uncompressed 32-bit `8_8_8_8` (`0x86`), used by jersey nameplate
/// (`name_*`) textures.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PixelFormat {
    Dxt1,
    Dxt3,
    Dxt5,
    /// Uncompressed 32-bit `GPUTEXTUREFORMAT_8_8_8_8` — stored on disk
    /// as `[A, R, G, B]` texels (big-endian, no endian swap), tiled per
    /// 4-byte texel rather than per 4×4 block.
    Argb8888,
}

impl PixelFormat {
    pub fn from_byte(b: u8) -> Option<Self> {
        Some(match b {
            0x52 => Self::Dxt1,
            0x53 => Self::Dxt3,
            0x54 => Self::Dxt5,
            0x86 => Self::Argb8888,
            _ => return None,
        })
    }

    pub fn to_byte(self) -> u8 {
        match self {
            Self::Dxt1 => 0x52,
            Self::Dxt3 => 0x53,
            Self::Dxt5 => 0x54,
            Self::Argb8888 => 0x86,
        }
    }

    /// `true` for the DXT-compressed (4×4 block) formats; `false` for
    /// uncompressed `Argb8888`. The tiling / sizing helpers treat these
    /// two families differently (block grid vs per-texel grid).
    pub fn is_dxt(self) -> bool {
        !matches!(self, Self::Argb8888)
    }

    /// Bytes per addressing element: a 4×4 DXT block (8/16) or a single
    /// 32-bit texel (4) for the uncompressed format. The block-based
    /// tiling helpers (`tile_dxt`/`untile_dxt`/`make_descriptor`) are
    /// only called for DXT; the uncompressed path uses the generic
    /// `tile`/`untile` with an explicit pitch of 4.
    pub fn bytes_per_block(self) -> usize {
        match self {
            Self::Dxt1 => 8,
            Self::Dxt3 | Self::Dxt5 => 16,
            Self::Argb8888 => 4,
        }
    }

    /// FOURCC tag for a DDS pixel-format block, or `None` for the
    /// uncompressed `Argb8888` (which a DDS encodes via RGB bit-masks,
    /// not a FOURCC).
    pub fn fourcc(self) -> Option<&'static [u8; 4]> {
        Some(match self {
            Self::Dxt1 => b"DXT1",
            Self::Dxt3 => b"DXT3",
            Self::Dxt5 => b"DXT5",
            Self::Argb8888 => return None,
        })
    }
}

/// One decoded texture pulled out of a texlib RX2.
///
/// `base_mip_linear` holds the top mip level only, fully untiled and
/// endian-swapped — i.e. a plain little-endian DXT block stream ready
/// to feed into [`tdb_archive_codec::encode_dds`] or `texpresso`.
#[derive(Debug, Clone)]
pub struct Texture {
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub format: PixelFormat,
    pub base_mip_linear: Vec<u8>,
    /// Internal: where in the source file this texture's tiled buffer
    /// lives, so the write path can splice replacement bytes back in.
    /// Hidden behind a private field accessor: external callers only
    /// see the public read-side data.
    slot: BufferSlot,
}

impl Texture {
    /// Total bytes the underlying buffer reserves for this texture's
    /// mip chain (base mip + any trailing levels + tile padding).
    pub fn buffer_capacity(&self) -> usize {
        self.slot.size as usize
    }

    /// Absolute file offset of this texture's 56-byte descriptor block.
    /// Lets callers (e.g. the `rx2tex desc` debug command) re-read the
    /// raw descriptor bytes from `Texlib::original_bytes()` for diffing
    /// or analysis.
    pub fn descriptor_offset(&self) -> usize {
        self.slot.desc_off as usize
    }

    /// Byte offset (relative to `hdr_size`) of this texture's tiled
    /// pixel buffer inside the data region.
    pub fn data_offset(&self) -> usize {
        self.slot.data_rel_off as usize
    }
}

/// One mip level for [`Texlib::replace_with_mip_chain`]: linear (untiled,
/// little-endian) DXT block stream of the indicated dimensions.
#[derive(Debug, Clone)]
pub struct LinearMip<'a> {
    pub width: u32,
    pub height: u32,
    pub linear_dxt: &'a [u8],
}

#[derive(Debug, Clone, Copy)]
struct BufferSlot {
    /// Offset of the data buffer relative to `hdr_size`.
    data_rel_off: u32,
    /// Total bytes in the buffer (base mip + trailing mip chain).
    size: u32,
    /// Absolute file offset of the 56-byte descriptor for this texture.
    desc_off: u32,
    /// Index of the TAG_TEX_DATA entry in the file table — needed when
    /// resizing the buffer to know which file-table entry's offset/size
    /// to patch.
    tex_data_entry_idx: u32,
}

/// Whole-container parse result. Holds the originally-read bytes plus
/// per-texture metadata, and lets the caller splice replacement
/// textures back into the source with [`Texlib::replace_same_size`].
#[derive(Debug, Clone)]
pub struct Texlib {
    pub textures: Vec<Texture>,
    raw: Vec<u8>,
    hdr_size: u32,
}

impl Texlib {
    /// Original (unmodified) container bytes. Useful when the caller
    /// wants a no-op write-back or a baseline for diffing.
    pub fn original_bytes(&self) -> &[u8] {
        &self.raw
    }

    /// Raw on-disk bytes of texture `idx`'s base mip, **exactly as the Xbox 360
    /// GPU samples them from guest RAM**: for DXT, Xbox-tiled + 16-bit-pair
    /// byte-swapped (NOT the untiled `base_mip_linear`); for `Argb8888`, the
    /// linear `[A,R,G,B]` stream. Length = [`base_mip_byte_size`]. Returns
    /// `None` if `idx` is out of range or the slot is truncated.
    pub fn base_mip_tiled(&self, idx: usize) -> Option<&[u8]> {
        let tex = self.textures.get(idx)?;
        let buf_start = self.hdr_size as usize + tex.data_offset();
        let base_size = base_mip_byte_size(tex.format, tex.width, tex.height);
        self.raw.get(buf_start..buf_start + base_size)
    }

    /// Raw on-disk bytes of texture `idx`'s **entire reserved slot buffer**
    /// (base mip + trailing mip chain + tile padding), in the same tiled/swapped
    /// on-disk layout as [`base_mip_tiled`]. This is the faithful image of what
    /// the game loads into guest physical RAM at the texture's fetch-constant
    /// base address. Length = [`Texture::buffer_capacity`]. `None` on range/
    /// truncation error.
    pub fn slot_buffer_tiled(&self, idx: usize) -> Option<&[u8]> {
        let tex = self.textures.get(idx)?;
        let buf_start = self.hdr_size as usize + tex.data_offset();
        self.raw.get(buf_start..buf_start + tex.buffer_capacity())
    }

    /// Replace texture `idx`'s mip chain.
    ///
    /// `mips[0]` must be the base mip at the texture's declared dimensions
    /// and format (same constraint as [`Texlib::replace_same_size`]).
    /// Subsequent levels are half-sized (each dim halved, min 1) and are
    /// each tiled + endian-swapped and packed sequentially into the X360
    /// buffer right after the base mip. The packing stops once a tiled
    /// mip would exceed the slot's reserved size — mips smaller than
    /// ~16x16 typically can't fit a tile-aligned form in the buffer the
    /// game ships with, so they're dropped silently. Trailing buffer
    /// bytes after the final mip are zeroed.
    ///
    /// Use [`Texlib::replace_same_size`] when you have only the base mip
    /// and don't need distance LOD coverage.
    pub fn replace_with_mip_chain(
        &self,
        idx: usize,
        mips: &[LinearMip<'_>],
    ) -> Result<Vec<u8>, TexlibError> {
        let tex = self
            .textures
            .get(idx)
            .ok_or(TexlibError::TextureIndexOutOfRange {
                idx,
                count: self.textures.len(),
            })?;
        if mips.is_empty() {
            return Err(TexlibError::ReplacementSizeMismatch {
                expected: base_mip_byte_size(tex.format, tex.width, tex.height),
                have: 0,
            });
        }
        let base = &mips[0];
        let expected = base_mip_byte_size(tex.format, tex.width, tex.height);
        if base.width != tex.width || base.height != tex.height {
            return Err(TexlibError::ReplacementSizeMismatch {
                expected,
                have: base.linear_dxt.len(),
            });
        }
        if base.linear_dxt.len() != expected {
            return Err(TexlibError::ReplacementSizeMismatch {
                expected,
                have: base.linear_dxt.len(),
            });
        }
        let bpb = tex.format.bytes_per_block();
        let buf_start = self.hdr_size as usize + tex.slot.data_rel_off as usize;
        let buf_size = tex.slot.size as usize;
        if buf_start + buf_size > self.raw.len() {
            return Err(TexlibError::TruncatedBuffer {
                start: buf_start,
                end: buf_start + buf_size,
                file_len: self.raw.len(),
            });
        }

        let mut out = self.raw.clone();
        let mut cursor = 0usize;
        let mut written_mips = 0usize;
        for mip in mips {
            let tiled = tile_dxt(mip.linear_dxt, mip.width, mip.height, bpb);
            let mut swapped = tiled;
            swap16_in_place(&mut swapped);
            if cursor + swapped.len() > buf_size {
                // No room for this mip in tiled form; drop the rest of the chain.
                break;
            }
            out[buf_start + cursor..buf_start + cursor + swapped.len()].copy_from_slice(&swapped);
            cursor += swapped.len();
            written_mips += 1;
        }
        // Zero everything after the last written mip.
        for b in &mut out[buf_start + cursor..buf_start + buf_size] {
            *b = 0;
        }
        // Diagnostic via debug_assert
        debug_assert!(written_mips >= 1);
        Ok(out)
    }

    /// Replace texture `idx` with a (potentially) different size and/or
    /// pixel format, reflowing the data region and rewriting the
    /// descriptor in place.
    ///
    /// `mips[0]` must be the new base mip; `mips[0].width` /
    /// `mips[0].height` define the target dimensions. Each entry is the
    /// linear (untiled, little-endian) DXT block stream for that mip
    /// level at `mips[i].width` × `mips[i].height`. Callers are
    /// responsible for generating + sizing the chain — texpresso /
    /// intel_tex_2 / etc. live in the CLI, not this crate.
    ///
    /// Reflow steps:
    /// - The target texture's tiled buffer is rebuilt at the new dims
    ///   (each mip tiled + 16-bit-pair-swapped, concatenated, padded up
    ///   to a 0x1000 boundary like the original files).
    /// - Subsequent buffers (textures whose `data_rel_off` was past
    ///   the target's old end) are shifted by the size delta — both
    ///   their bytes inside the data region and their file-table
    ///   `data_rel_off` entries.
    /// - The header `data_size` field at 0x54 is updated.
    /// - The target's 56-byte descriptor is rewritten via
    ///   [`make_descriptor`] with the new (width, height, format,
    ///   mip_count).
    ///
    /// **Empirical caveat:** the descriptor encoding for sizes other
    /// than the 256 × 256 / 512 × 512 found in `texlib_67.rx2` is
    /// inferred from a 2-point pattern. See [`make_descriptor`] for the
    /// unverified bits.
    pub fn replace_with_resize(
        &self,
        idx: usize,
        new_format: PixelFormat,
        mips: &[LinearMip<'_>],
    ) -> Result<Vec<u8>, TexlibError> {
        let tex = self
            .textures
            .get(idx)
            .ok_or(TexlibError::TextureIndexOutOfRange {
                idx,
                count: self.textures.len(),
            })?;
        // Resize regenerates the Xenos DXT descriptor (DW7/DW12 etc.),
        // which we only understand for DXT. Uncompressed `8_8_8_8` is
        // same-size only — callers use `replace_same_size` for it.
        if !new_format.is_dxt() {
            return Err(TexlibError::ResizeUnsupportedFormat { format: new_format });
        }
        let base = mips.first().ok_or(TexlibError::ReplacementSizeMismatch {
            expected: 0,
            have: 0,
        })?;
        let new_width = base.width;
        let new_height = base.height;
        if new_width == 0 || new_height == 0 {
            return Err(TexlibError::ReplacementSizeMismatch {
                expected: 0,
                have: 0,
            });
        }

        // Validate every mip's payload size matches its declared dims.
        let bpb = new_format.bytes_per_block();
        for (i, mip) in mips.iter().enumerate() {
            let expected = base_mip_byte_size(new_format, mip.width, mip.height);
            if mip.linear_dxt.len() != expected {
                return Err(TexlibError::MipSizeMismatch {
                    level: i,
                    width: mip.width,
                    height: mip.height,
                    expected,
                    have: mip.linear_dxt.len(),
                });
            }
        }

        // Build the new tiled buffer: every mip tiled + endian-swapped,
        // concatenated. tile_dxt rounds each mip up to a 32-block macro
        // tile in both dims, which is what the XGAddress2DTiledOffset
        // formula needs to address all blocks. Then pad the total to
        // 0x1000 — every observed slot in `texlib_67.rx2` is a multiple
        // of 0x1000 and the runtime appears to expect page-aligned
        // buffers.
        let mut new_buf: Vec<u8> = Vec::new();
        for mip in mips {
            let tiled = tile_dxt(mip.linear_dxt, mip.width, mip.height, bpb);
            let mut swapped = tiled;
            swap16_in_place(&mut swapped);
            new_buf.extend_from_slice(&swapped);
        }
        let new_buf_size = (new_buf.len() + 0xFFF) & !0xFFF;
        new_buf.resize(new_buf_size, 0);

        // Re-read every TAG_TEX_DATA entry from the raw file table so
        // we know exactly which slots to reflow and which file-table
        // bytes to patch on the way out.
        let file_table_off = read_u32_be(&self.raw, HDR_FILE_TABLE_OFF) as usize;
        let file_count = read_u32_be(&self.raw, HDR_FILE_COUNT_OFF) as usize;
        let mut data_entries: Vec<DataEntry> = Vec::new();
        for i in 0..file_count {
            let base_off = file_table_off + i * ENTRY_STRIDE;
            let tag = &self.raw[base_off + 20..base_off + 24];
            if tag == TAG_TEX_DATA {
                let data_rel_off = read_u32_be(&self.raw, base_off);
                let buf_size = read_u32_be(&self.raw, base_off + 8);
                data_entries.push(DataEntry {
                    entry_idx: i,
                    orig_off: data_rel_off,
                    orig_size: buf_size,
                });
            }
        }
        // Reflow the data region in original-offset order so each
        // buffer's bytes only ever move forward/backward by the
        // accumulated delta of slots that precede it.
        data_entries.sort_by_key(|e| e.orig_off);

        // Allocate the output: header region copied verbatim + new
        // data region whose total size we compute as we walk slots.
        let hdr = self.hdr_size as usize;
        let mut layout: Vec<(DataEntry, u32)> = Vec::with_capacity(data_entries.len());
        let mut running: u32 = 0;
        for entry in &data_entries {
            let new_size = if entry.entry_idx as u32 == tex.slot.tex_data_entry_idx {
                new_buf_size as u32
            } else {
                entry.orig_size
            };
            layout.push((*entry, running));
            running = running
                .checked_add(new_size)
                .ok_or(TexlibError::DataRegionOverflow)?;
        }
        let new_data_size = running;
        let mut out = vec![0u8; hdr + new_data_size as usize];
        out[..hdr].copy_from_slice(&self.raw[..hdr]);

        // Splice each buffer into its new position.
        for (entry, new_off) in &layout {
            let dst = hdr + *new_off as usize;
            if entry.entry_idx as u32 == tex.slot.tex_data_entry_idx {
                out[dst..dst + new_buf_size].copy_from_slice(&new_buf);
            } else {
                let src_start = hdr + entry.orig_off as usize;
                let src_end = src_start + entry.orig_size as usize;
                if src_end > self.raw.len() {
                    return Err(TexlibError::TruncatedBuffer {
                        start: src_start,
                        end: src_end,
                        file_len: self.raw.len(),
                    });
                }
                out[dst..dst + entry.orig_size as usize]
                    .copy_from_slice(&self.raw[src_start..src_end]);
            }
        }

        // Patch every TAG_TEX_DATA entry in the file table: its
        // data_rel_off (ints[0]) and buffer-size (ints[2]).
        for (entry, new_off) in &layout {
            let base_off = file_table_off + entry.entry_idx * ENTRY_STRIDE;
            let new_size = if entry.entry_idx as u32 == tex.slot.tex_data_entry_idx {
                new_buf_size as u32
            } else {
                entry.orig_size
            };
            out[base_off..base_off + 4].copy_from_slice(&new_off.to_be_bytes());
            out[base_off + 8..base_off + 12].copy_from_slice(&new_size.to_be_bytes());
        }

        // Patch header data_size at 0x54.
        out[0x54..0x58].copy_from_slice(&new_data_size.to_be_bytes());

        // Rewrite the target's descriptor in place. The descriptor's
        // base-address field (DW8) must point at the slot's reflowed
        // data offset, so pull the target's post-reflow `new_off` out of
        // the layout we just computed.
        let target_data_off = layout
            .iter()
            .find(|(entry, _)| entry.entry_idx as u32 == tex.slot.tex_data_entry_idx)
            .map(|(_, new_off)| *new_off)
            .ok_or(TexlibError::DataRegionOverflow)?;
        let mip_count = mips.len() as u32;
        let new_desc = make_descriptor(
            new_width,
            new_height,
            new_format,
            mip_count,
            target_data_off,
        );
        let desc_off = tex.slot.desc_off as usize;
        if desc_off + new_desc.len() > out.len() {
            return Err(TexlibError::TruncatedDescriptor { off: desc_off });
        }
        out[desc_off..desc_off + new_desc.len()].copy_from_slice(&new_desc);

        Ok(out)
    }

    /// Replace texture `idx`'s base mip with `new_dxt_base` bytes at
    /// the *same* dimensions and format. Returns a fresh `Vec<u8>` of
    /// the rewritten container.
    ///
    /// `new_dxt_base` is the **linear** (untiled, little-endian) base
    /// mip block stream — i.e. what comes out of a DDS file after the
    /// 128-byte header is stripped. It must be exactly
    /// `base_mip_byte_size(format, width, height)` bytes. Smaller mip
    /// levels in the slot are zeroed; the game's mip-selection still
    /// works because the base mip is the largest and the game's LOD
    /// for player heads stays close to mip 0 in practice.
    ///
    /// All other bytes of the file are preserved exactly — header,
    /// file table, descriptors, name table, and the other texture's
    /// buffer.
    pub fn replace_same_size(
        &self,
        idx: usize,
        new_dxt_base: &[u8],
    ) -> Result<Vec<u8>, TexlibError> {
        let tex = self
            .textures
            .get(idx)
            .ok_or(TexlibError::TextureIndexOutOfRange {
                idx,
                count: self.textures.len(),
            })?;
        let expected = base_mip_byte_size(tex.format, tex.width, tex.height);
        if new_dxt_base.len() != expected {
            return Err(TexlibError::ReplacementSizeMismatch {
                expected,
                have: new_dxt_base.len(),
            });
        }

        // DXT: tile by 4×4 block + 16-bit-pair swap. Argb8888 texlib is
        // stored LINEAR (no tiling, no swap) — write the `[A,R,G,B]`
        // base mip straight back (see the parse-side note).
        let swapped = if tex.format.is_dxt() {
            let mut t = tile_dxt(
                new_dxt_base,
                tex.width,
                tex.height,
                tex.format.bytes_per_block(),
            );
            swap16_in_place(&mut t);
            t
        } else {
            new_dxt_base.to_vec()
        };

        let buf_start = self.hdr_size as usize + tex.slot.data_rel_off as usize;
        let buf_size = tex.slot.size as usize;
        let buf_end = buf_start + buf_size;
        if buf_end > self.raw.len() {
            return Err(TexlibError::TruncatedBuffer {
                start: buf_start,
                end: buf_end,
                file_len: self.raw.len(),
            });
        }
        if swapped.len() > buf_size {
            return Err(TexlibError::TiledSizeExceedsSlot {
                tiled: swapped.len(),
                slot: buf_size,
            });
        }

        let mut out = self.raw.clone();
        // Splice the new base mip; zero the rest of the slot (any
        // trailing mip chain) so the game doesn't render stale data
        // for distant LODs.
        out[buf_start..buf_start + swapped.len()].copy_from_slice(&swapped);
        for b in &mut out[buf_start + swapped.len()..buf_end] {
            *b = 0;
        }
        Ok(out)
    }

    /// Splice a new base mip into texture `idx` and KEEP the original
    /// trailing mip-chain bytes verbatim. The console packed-mip tail
    /// layout has never been validated for our generated chains
    /// (PR11 in-game finding: players rendered debug-blue at gameplay
    /// distance, where the GPU samples the small mips) — the donor's
    /// authored tail is bit-correct by construction and reads as a
    /// skin-tone blur at the distances it covers.
    ///
    /// Same constraints as [`Texlib::replace_same_size`]: the new base
    /// must match the slot's declared dimensions + format.
    pub fn replace_base_keep_tail(
        &self,
        idx: usize,
        new_dxt_base: &[u8],
    ) -> Result<Vec<u8>, TexlibError> {
        let tex = self
            .textures
            .get(idx)
            .ok_or(TexlibError::TextureIndexOutOfRange {
                idx,
                count: self.textures.len(),
            })?;
        let expected = base_mip_byte_size(tex.format, tex.width, tex.height);
        if new_dxt_base.len() != expected {
            return Err(TexlibError::ReplacementSizeMismatch {
                expected,
                have: new_dxt_base.len(),
            });
        }
        let swapped = if tex.format.is_dxt() {
            let mut t = tile_dxt(
                new_dxt_base,
                tex.width,
                tex.height,
                tex.format.bytes_per_block(),
            );
            swap16_in_place(&mut t);
            t
        } else {
            new_dxt_base.to_vec()
        };
        let buf_start = self.hdr_size as usize + tex.slot.data_rel_off as usize;
        let buf_end = buf_start + tex.slot.size as usize;
        if buf_end > self.raw.len() {
            return Err(TexlibError::TruncatedBuffer {
                start: buf_start,
                end: buf_end,
                file_len: self.raw.len(),
            });
        }
        if swapped.len() > tex.slot.size as usize {
            return Err(TexlibError::TiledSizeExceedsSlot {
                tiled: swapped.len(),
                slot: tex.slot.size as usize,
            });
        }
        let mut out = self.raw.clone();
        out[buf_start..buf_start + swapped.len()].copy_from_slice(&swapped);
        // bytes [buf_start + swapped.len() .. buf_end] — the original
        // mip tail — stay untouched.
        Ok(out)
    }
}

/// Internal: one TAG_TEX_DATA entry pulled out of the file table for
/// the reflow planner in [`Texlib::replace_with_resize`].
#[derive(Debug, Clone, Copy)]
struct DataEntry {
    entry_idx: usize,
    orig_off: u32,
    orig_size: u32,
}

#[derive(Debug, Error)]
pub enum TexlibError {
    #[error("file shorter than the {needed}-byte container header ({have} bytes)")]
    TooShort { needed: usize, have: usize },
    #[error("magic mismatch — expected `\\x89RW4xb2`")]
    BadMagic,
    #[error("file table at 0x{off:x} runs past EOF (need {need} bytes, have {have})")]
    TruncatedFileTable {
        off: usize,
        need: usize,
        have: usize,
    },
    #[error("texture descriptor at 0x{off:x} runs past EOF")]
    TruncatedDescriptor { off: usize },
    #[error("texture data slot at 0x{off:x} runs past EOF (need {need}, have {have})")]
    TruncatedDataSlot {
        off: usize,
        need: usize,
        have: usize,
    },
    #[error("unsupported pixel format byte 0x{0:02x}")]
    UnsupportedFormat(u8),
    #[error("data slot {data} has no matching descriptor")]
    OrphanData { data: usize },
    #[error("descriptor {desc} has no matching data slot")]
    OrphanDescriptor { desc: usize },
    #[error("texture index {idx} out of range (have {count} textures)")]
    TextureIndexOutOfRange { idx: usize, count: usize },
    #[error("replacement size mismatch — expected {expected} base-mip bytes, got {have}")]
    ReplacementSizeMismatch { expected: usize, have: usize },
    #[error("data buffer [{start:#x}..{end:#x}] truncated (file is {file_len} bytes)")]
    TruncatedBuffer {
        start: usize,
        end: usize,
        file_len: usize,
    },
    #[error("tiled replacement is {tiled} bytes but slot only holds {slot}")]
    TiledSizeExceedsSlot { tiled: usize, slot: usize },
    #[error(
        "mip level {level} ({width}x{height}) carries {have} bytes; \
         expected {expected} for the declared format"
    )]
    MipSizeMismatch {
        level: usize,
        width: u32,
        height: u32,
        expected: usize,
        have: usize,
    },
    #[error("reflowed data region exceeds u32 size limit")]
    DataRegionOverflow,
    #[error("resize is only supported for DXT formats, not {format:?} (use replace_same_size)")]
    ResizeUnsupportedFormat { format: PixelFormat },
}

/// Parse a texlib RX2 container into [`Texture`]s plus the metadata
/// needed to write replacements back.
pub fn parse(data: &[u8]) -> Result<Texlib, TexlibError> {
    if data.len() < 0x60 {
        return Err(TexlibError::TooShort {
            needed: 0x60,
            have: data.len(),
        });
    }
    if &data[..MAGIC.len()] != MAGIC {
        return Err(TexlibError::BadMagic);
    }

    let file_count = read_u32_be(data, HDR_FILE_COUNT_OFF) as usize;
    let file_table = read_u32_be(data, HDR_FILE_TABLE_OFF) as usize;
    let mut hdr_size = read_u32_be(data, HDR_HDR_SIZE_OFF);
    // NHL 07 single-raster textures pack everything into the header region (`data_size == 0`,
    // `hdr_size` == file length) with absolute buffer offsets; NHL 08→Legacy keep a separate pixel-data
    // region biased by `hdr_size`. Drop the bias when there is no data region (mirrors the mesh parser),
    // else every raster buffer resolves past EOF and the texture silently fails to decode.
    let no_data_region = read_u32_be(data, HDR_DATA_SIZE_OFF) == 0;
    if no_data_region {
        hdr_size = 0;
    }

    let table_bytes = file_count.saturating_mul(ENTRY_STRIDE);
    if file_table + table_bytes > data.len() {
        return Err(TexlibError::TruncatedFileTable {
            off: file_table,
            need: table_bytes,
            have: data.len().saturating_sub(file_table),
        });
    }

    // Walk the file table once, classifying entries by tag. The
    // `usize` is the entry's index in the file table — kept so the
    // reflow path knows which entry to patch when a slot grows.
    let mut data_slots: Vec<(usize, [u32; ENTRY_INTS])> = Vec::new();
    let mut desc_slots: Vec<(usize, [u32; ENTRY_INTS])> = Vec::new();
    let mut name_offsets: Vec<usize> = Vec::new();

    for i in 0..file_count {
        let base = file_table + i * ENTRY_STRIDE;
        let mut ints = [0u32; ENTRY_INTS];
        for (j, slot) in ints.iter_mut().enumerate() {
            *slot = read_u32_be(data, base + j * 4);
        }
        let tag = &data[base + 20..base + 24];
        // NHL 07's pixel-data payload carries an untagged `00 00 00 00` tag rather than the
        // `TAG_TEX_DATA` NHL 08+ use. Only accept it as a data slot in the NHL 07 layout (no data
        // region) and only when it actually holds bytes, so NHL 08+ padding/zero entries are unaffected.
        let is_nhl07_data = no_data_region && tag == [0u8, 0, 0, 0] && ints[2] > 0;
        if tag == TAG_TEX_DATA || is_nhl07_data {
            data_slots.push((i, ints));
        } else if tag == TAG_TEX_DESC {
            desc_slots.push((i, ints));
        } else if tag == TAG_NAME_TABLE {
            name_offsets.push(ints[0] as usize);
        }
    }

    // Texture names are stored as NUL-terminated ASCII anywhere in the
    // header region. The name table entry points at the first one;
    // names always come in the same order as the data slots.
    let names = if let Some(&base) = name_offsets.first() {
        scan_names(data, base)
    } else {
        Vec::new()
    };

    // Pair each data slot with the descriptor that immediately follows
    // it in the file table — this matches the Noesis reader's
    // observation that types alternate (data, desc, data, desc, ...).
    if data_slots.len() != desc_slots.len() {
        // Common cause: an orphan data or descriptor — we still try to
        // recover paired entries by file-table index ordering.
        // The error here is best-effort information for the caller.
        if data_slots.len() > desc_slots.len() {
            return Err(TexlibError::OrphanData {
                data: data_slots[desc_slots.len()].0,
            });
        } else {
            return Err(TexlibError::OrphanDescriptor {
                desc: desc_slots[data_slots.len()].0,
            });
        }
    }

    let mut textures = Vec::with_capacity(data_slots.len());
    for (pair_idx, ((data_entry_idx, data_ints), (_desc_entry_idx, desc_ints))) in
        data_slots.iter().zip(desc_slots.iter()).enumerate()
    {
        let data_rel_off = data_ints[0];
        let buf_size = data_ints[2];
        let desc_off = desc_ints[0] as usize;
        if desc_off + DESC_WIDTH_WORD_REL + 2 > data.len() {
            return Err(TexlibError::TruncatedDescriptor { off: desc_off });
        }
        let fmt_byte = data[desc_off + DESC_FORMAT_BYTE_REL];
        let height_byte = data[desc_off + DESC_HEIGHT_BYTE_REL];
        let width_word = read_u16_be(data, desc_off + DESC_WIDTH_WORD_REL);
        // Skip textures whose GPU format we don't decode (e.g. DXN/BC5
        // normal maps — format 0x71) rather than failing the whole
        // container: a jersey `texlib_*` bundles DXT color maps we DO
        // preview alongside normal maps we don't, and the color maps
        // must still surface. The skip is deterministic, so a texture's
        // index in the returned list is stable across re-parses (the
        // replace path re-parses and indexes the same way).
        let Some(format) = PixelFormat::from_byte(fmt_byte) else {
            continue;
        };
        let height = (height_byte as u32 + 1) * 8;
        let width = (width_word.wrapping_add(1) & DESC_WIDTH_VALUE_MASK) as u32;

        let buf_start = hdr_size as usize + data_rel_off as usize;
        let buf_end =
            buf_start
                .checked_add(buf_size as usize)
                .ok_or(TexlibError::TruncatedDataSlot {
                    off: buf_start,
                    need: buf_size as usize,
                    have: 0,
                })?;
        if buf_end > data.len() {
            return Err(TexlibError::TruncatedDataSlot {
                off: buf_start,
                need: buf_size as usize,
                have: data.len().saturating_sub(buf_start),
            });
        }
        let base_size = base_mip_byte_size(format, width, height);
        if base_size > buf_size as usize {
            return Err(TexlibError::TruncatedDataSlot {
                off: buf_start,
                need: base_size,
                have: buf_size as usize,
            });
        }
        let mut base_tiled = data[buf_start..buf_start + base_size].to_vec();
        let base_mip_linear = if format.is_dxt() {
            // DXT: 16-bit-pair swap, then untile by 4×4 block.
            swap16_in_place(&mut base_tiled);
            untile_dxt(&base_tiled, width, height, format.bytes_per_block())
        } else {
            // Argb8888 ships two ways. The rendering-cache `name_*` nameplates store it LINEAR
            // (row-major `[A, R, G, B]`, verified against the PS3 rpsgl ground truth). NHL 07's FE
            // portraits (`fe/ion/.../playerheads/p*.big` → RX2) instead store it **Xbox-tiled at
            // 32-aligned dimensions**: the data slot holds the full padded tiled buffer, larger than
            // `width*height*4`. Detect that by the padded buffer size and untile at the real dims
            // (`untile` reads through the 32-aligned pitch and crops to width×height); otherwise the
            // bytes are already the linear image. Verified on a real NHL 07 install (468×248 portrait
            // stored as a 480×256 tiled buffer).
            let linear_size = (width as usize) * (height as usize) * 4;
            let tiled_size = tiled_buffer_size_texels(width, height, 4);
            if tiled_size > linear_size && (buf_size as usize) >= tiled_size {
                let full = &data[buf_start..buf_start + tiled_size];
                untile(full, width, height, 4)
            } else {
                base_tiled
            }
        };

        let name = names
            .get(pair_idx)
            .cloned()
            .unwrap_or_else(|| format!("texture_{pair_idx:04}"));

        textures.push(Texture {
            name,
            width,
            height,
            format,
            base_mip_linear,
            slot: BufferSlot {
                data_rel_off,
                size: buf_size,
                desc_off: desc_off as u32,
                tex_data_entry_idx: *data_entry_idx as u32,
            },
        });
    }

    Ok(Texlib {
        textures,
        raw: data.to_vec(),
        hdr_size,
    })
}

/// Linear (untiled) base-mip byte size for the given format + dims.
/// DXT is block-based; `Argb8888` is `width * height * 4`.
pub fn base_mip_byte_size(format: PixelFormat, width: u32, height: u32) -> usize {
    if !format.is_dxt() {
        return (width as usize) * (height as usize) * 4;
    }
    let bw = ((width + 3) / 4).max(1) as usize;
    let bh = ((height + 3) / 4).max(1) as usize;
    bw * bh * format.bytes_per_block()
}

/// Length of the 56-byte descriptor block we write in
/// [`make_descriptor`] / parse off disk.
pub const DESCRIPTOR_LEN: usize = 56;

/// Synthesise the 56-byte descriptor block for a texture at the given
/// dimensions, format, and mip-count.
///
/// **Encoding — Xenos `GPUTEXTURE_FETCH_CONSTANT` (resolved 2026-05-29):**
/// the 56-byte EA block wraps a real Xbox 360 texture fetch constant at
/// DW7–DW12. The encoding was originally fit from only two samples (hair
/// 256 × 256 DXT3 + head 512 × 512 DXT1 in `texlib_67.rx2`), which left
/// several DWORDs ambiguous. Decoding both samples against the documented
/// Xenos field layout showed every dimension-derived field
/// (pitch, 2D size, max_mip_level, DW12/DW13 size-class) already
/// extrapolates correctly — **the only field that was actually wrong was
/// DW8's base address**, which the old code mis-modelled as a
/// resolution flag (`(log2−8) × 0x20`). It is in fact the slot's pixel
/// base address: `DW8 = (data_off >> 4) | format_byte`. This only
/// coincidentally matched at 512 (head data lived at 0x20000, and
/// `0x20000 >> 4 = 0x2000` happened to equal the bogus `0x20` flag in
/// the same byte position); at 1024 the old formula pointed the GPU at
/// double the real address, producing the vertical-stripe corruption
/// seen in the 2026-05-28 in-game test.
///
/// `data_off` is the target slot's pixel-buffer offset **relative to
/// `hdr_size`** (i.e. the file-table `data_rel_off` after reflow), which
/// is exactly the address the runtime fetches from.
///
/// Field map (relative to the start of the descriptor):
///
/// ```text
///   off  field      value
///   --   ---------  -----------------------------------------------
///   0x00 DW0        0x00400003 (constant tag/type)
///   0x04 DW1        0x01000000 (constant)
///   0x08-0x13       all-zero
///   0x14 DW5        0xFFFF0000 (color-key low word)
///   0x18 DW6        0xFFFF0000 (color-key high word)
///   0x1C DW7        tiled(b31)=1 + pitch(b22-30)=width/32, then
///                   0x00 0x00 0x02. High byte = 0x80 | (pitch/4) =
///                   0x80 | (width/128).
///   0x20 DW8        base_address | format:
///                   `(data_off >> 4) | format_byte`
///                   (0x52 DXT1 / 0x53 DXT3 / 0x54 DXT5 in the low byte;
///                   page-aligned data_off keeps the low byte clear).
///   0x24 DW9        2D Size: height_byte = (height/8 − 1),
///                   width_word = 0xE000 | ((width − 1) & 0x1FFF).
///   0x28 DW10       0x00000D10 (constant)
///   0x2C DW11       max_mip_level(b6-9) = log2(max_dim); byte 0x2F =
///                   (log2(max_dim) − 8) × 0x40.
///   0x30 DW12       0x00, `log2(max_dim) − 7`, 0x0A, 0x00.
///   0x34 DW13       0x08, 0x00, `log2(max_dim) + 1`, 0x00
///                   (the +1 byte = bit_length(max_dim)).
/// ```
///
/// `mip_count` is currently informational — none of the encoded bits
/// change with mip count in the reference samples. Plumbed through
/// anyway so we can light it up once we identify the field.
pub fn make_descriptor(
    width: u32,
    height: u32,
    format: PixelFormat,
    _mip_count: u32,
    data_off: u32,
) -> [u8; DESCRIPTOR_LEN] {
    let mut d = [0u8; DESCRIPTOR_LEN];
    let max_dim = width.max(height).max(1);
    // log2 of the largest power-of-two ≤ max_dim. For pow2 inputs this
    // is exact: 256 → 8, 512 → 9, 1024 → 10.
    let log2_max = 31 - max_dim.leading_zeros() as i32;
    let above256 = (log2_max - 8).max(0) as u32; // 256 → 0, 512 → 1, 1024 → 2

    // DW0 + DW1 — constant header bytes.
    d[0..4].copy_from_slice(&0x00400003u32.to_be_bytes());
    d[4..8].copy_from_slice(&0x01000000u32.to_be_bytes());
    // DW2..DW4 — all zero (already initialised).
    // DW5 + DW6 — observed-constant color-key words.
    d[20..24].copy_from_slice(&0xFFFF0000u32.to_be_bytes());
    d[24..28].copy_from_slice(&0xFFFF0000u32.to_be_bytes());

    // DW7 — tiled + pitch. High byte = 0x80 | (width/128) = 0x80 | (pitch/4).
    d[28] = 0x80 | ((width / 128).min(0x7F) as u8);
    d[29] = 0x00;
    d[30] = 0x00;
    d[31] = 0x02;

    // DW8 — base address | format. The pixel-buffer offset (relative to
    // hdr_size) shifted right by 4, OR'd with the format byte in the low
    // 8 bits. Page-aligned (0x1000) data_off guarantees the low byte is
    // zero, so the format never collides with the address.
    let dw8 = (data_off >> 4) | (format.to_byte() as u32);
    d[32..36].copy_from_slice(&dw8.to_be_bytes());

    // DW9 — height byte + width word (both confirmed from parsing).
    let height_byte = ((height / 8).saturating_sub(1)).min(0xFF) as u8;
    let width_word: u16 = 0xE000 | ((width.saturating_sub(1)) as u16 & DESC_WIDTH_VALUE_MASK);
    d[36] = 0x00;
    d[37] = height_byte;
    d[38..40].copy_from_slice(&width_word.to_be_bytes());

    // DW10 — observed constant.
    d[40..44].copy_from_slice(&0x00000D10u32.to_be_bytes());

    // DW11 — byte 0x2F scales with log2 above 256 (×0x40 per step).
    d[44] = 0x00;
    d[45] = 0x00;
    d[46] = 0x02;
    d[47] = (above256 * 0x40).min(0xFF) as u8;

    // DW12 — bytes 0x30..0x31 (BE u16) = the mip-chain START ADDRESS in
    // 64 KB pages, i.e. the offset (relative to this texture's data) where
    // mip level 1 begins. That offset is exactly the tile-aligned base-mip
    // size, which for power-of-two dims is `base_mip_byte_size >> 16`:
    //   256² DXT3 → 0x10000 >> 16 = 1
    //   512² DXT1 → 0x20000 >> 16 = 2
    //   1024² DXT3 → 0x100000 >> 16 = 16 (0x10)   ← confirmed vs real
    //                                                centerlogo_0_cm.rx2
    // The old `log2_max - 7` formula only coincidentally matched 256/512;
    // at 1024 it wrote 3 instead of 16, pointing the GPU at 0x30000 for
    // the mip chain (deep inside the base mip) → out-of-range read → crash.
    let mip_pages = (base_mip_byte_size(format, width, height) >> 16) as u16;
    d[48..50].copy_from_slice(&mip_pages.to_be_bytes());
    d[50] = 0x0A;
    d[51] = 0x00;

    // DW13 — byte 0x36 = bit_length(max_dim) = log2_max + 1.
    d[52] = 0x08;
    d[53] = 0x00;
    d[54] = ((log2_max + 1).max(1).min(0xFF)) as u8;
    d[55] = 0x00;

    d
}

fn read_u32_be(data: &[u8], off: usize) -> u32 {
    let bytes: [u8; 4] = data[off..off + 4]
        .try_into()
        .expect("caller already bounds-checked");
    u32::from_be_bytes(bytes)
}

fn read_u16_be(data: &[u8], off: usize) -> u16 {
    let bytes: [u8; 2] = data[off..off + 2]
        .try_into()
        .expect("caller already bounds-checked");
    u16::from_be_bytes(bytes)
}

fn scan_names(data: &[u8], start: usize) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = start;
    let suffixes: &[&[u8]] = &[b".Raster", b".Texture", b".RasterData", b".RasterTexture"];
    while i < data.len() {
        if !is_name_byte(data[i]) {
            i += 1;
            continue;
        }
        let s = i;
        while i < data.len() && is_name_byte(data[i]) {
            i += 1;
        }
        let token = &data[s..i];
        if suffixes.iter().any(|sfx| token.ends_with(sfx)) {
            if let Ok(name) = std::str::from_utf8(token) {
                out.push(name.to_owned());
            }
        }
    }
    out
}

fn is_name_byte(b: u8) -> bool {
    matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-')
}

// -----------------------------------------------------------------------
// Xbox 360 tiling
// -----------------------------------------------------------------------

/// Canonical Microsoft `XGAddress2DTiledOffset` — returns the byte
/// offset in the **tiled** buffer for the (x, y)-th 4×4 block of an
/// otherwise linear block grid.
///
/// `width` is the block-width of the texture (i.e. `width_px / 4`);
/// `texel_pitch` is the bytes-per-block (8 for DXT1, 16 for DXT3/5).
fn xg_address_2d_tiled(x: u32, y: u32, width: u32, texel_pitch: u32) -> u32 {
    let aligned_w = (width + 31) & !31;
    // log2(texel_pitch) for texel_pitch in {8, 16} → {3, 4}. Microsoft's
    // funky formula handles other widths too but only these two matter
    // for DXT.
    let log_bpp = (texel_pitch >> 2) + ((texel_pitch >> 1) >> (texel_pitch >> 2));
    let macro_ = ((x >> 5) + (y >> 5) * (aligned_w >> 5)) << (log_bpp + 7);
    let micro = ((x & 7) + ((y & 6) << 2)) << log_bpp;
    let offset = macro_
        + ((micro & !0xF) << 1)
        + (micro & 0xF)
        + ((y & 8) << (3 + log_bpp))
        + ((y & 1) << 4);
    ((offset & !0x1FF) << 3)
        + ((y & 16) << 7)
        + ((offset & 0x1C0) << 2)
        + (((((y & 8) >> 2) + (x >> 3)) & 3) << 6)
        + (offset & 0x3F)
}

/// Translate a tiled element stream into a linear one (row-major
/// element order, top-left to bottom-right) over a `grid_w × grid_h`
/// grid of `element_pitch`-byte elements. An "element" is a 4×4 DXT
/// block (pitch 8/16) for compressed textures or a single texel
/// (pitch 4 for `8_8_8_8`) for uncompressed ones — the
/// `XGAddress2DTiledOffset` formula is element-agnostic.
pub fn untile(tiled: &[u8], grid_w: u32, grid_h: u32, element_pitch: usize) -> Vec<u8> {
    let gw = grid_w.max(1);
    let gh = grid_h.max(1);
    let total = (gw as usize) * (gh as usize) * element_pitch;
    let mut out = vec![0u8; total];
    for y in 0..gh {
        for x in 0..gw {
            let src = xg_address_2d_tiled(x, y, gw, element_pitch as u32) as usize;
            let dst = ((y * gw + x) as usize) * element_pitch;
            if src + element_pitch <= tiled.len() {
                out[dst..dst + element_pitch].copy_from_slice(&tiled[src..src + element_pitch]);
            }
        }
    }
    out
}

/// Inverse of [`untile`]: take a linear row-major element stream and
/// produce a tiled buffer (sized to [`tiled_buffer_size_texels`]) ready
/// to splice into an RX2/XPR2 data slot. Positions the linear input
/// doesn't cover stay zero.
pub fn tile(linear: &[u8], grid_w: u32, grid_h: u32, element_pitch: usize) -> Vec<u8> {
    let gw = grid_w.max(1);
    let gh = grid_h.max(1);
    let aligned_w = ((gw as usize) + 31) & !31;
    let aligned_h = ((gh as usize) + 31) & !31;
    let mut out = vec![0u8; aligned_w * aligned_h * element_pitch];
    for y in 0..gh {
        for x in 0..gw {
            let dst = xg_address_2d_tiled(x, y, gw, element_pitch as u32) as usize;
            let src = ((y * gw + x) as usize) * element_pitch;
            if src + element_pitch <= linear.len() && dst + element_pitch <= out.len() {
                out[dst..dst + element_pitch].copy_from_slice(&linear[src..src + element_pitch]);
            }
        }
    }
    out
}

/// Translate a tiled DXT block stream into a linear one (row-major
/// block order, top-left to bottom-right).
pub fn untile_dxt(tiled: &[u8], width: u32, height: u32, bytes_per_block: usize) -> Vec<u8> {
    let bw = ((width + 3) / 4).max(1);
    let bh = ((height + 3) / 4).max(1);
    untile(tiled, bw, bh, bytes_per_block)
}

/// Inverse of [`untile_dxt`]: take a linear row-major block stream and
/// produce a tiled buffer ready to splice into an RX2 data slot.
///
/// The output is sized to `tiled_buffer_size(width, height, bpb)` —
/// both dimensions rounded up to 32-block alignment — so every block
/// the address formula reaches lands in-bounds, even when the texture
/// is smaller than a single macro tile. Any positions not covered by
/// the linear input stay zero.
pub fn tile_dxt(linear: &[u8], width: u32, height: u32, bytes_per_block: usize) -> Vec<u8> {
    let bw = ((width + 3) / 4).max(1);
    let bh = ((height + 3) / 4).max(1);
    let mut out = vec![0u8; tiled_buffer_size(width, height, bytes_per_block)];
    for by in 0..bh {
        for bx in 0..bw {
            let dst = xg_address_2d_tiled(bx, by, bw, bytes_per_block as u32) as usize;
            let src = ((by * bw + bx) as usize) * bytes_per_block;
            if src + bytes_per_block <= linear.len() && dst + bytes_per_block <= out.len() {
                out[dst..dst + bytes_per_block]
                    .copy_from_slice(&linear[src..src + bytes_per_block]);
            }
        }
    }
    out
}

/// Bytes a tiled DXT buffer must reserve for a given texture size.
/// Both dimensions are rounded up to a 32-block macro-tile boundary,
/// which is the upper bound the XGAddress2DTiledOffset formula can
/// address.
pub fn tiled_buffer_size(width: u32, height: u32, bytes_per_block: usize) -> usize {
    let bw = ((width + 3) / 4).max(1) as usize;
    let bh = ((height + 3) / 4).max(1) as usize;
    let aligned_w = (bw + 31) & !31;
    let aligned_h = (bh + 31) & !31;
    aligned_w * aligned_h * bytes_per_block
}

/// Bytes a tiled buffer must reserve for an *uncompressed* texture
/// whose grid is `width × height` elements of `element_pitch` bytes
/// (e.g. `8_8_8_8` = 4-byte texels). Each dimension is rounded up to a
/// 32-element macro-tile boundary — the texel-grid analogue of
/// [`tiled_buffer_size`]'s block-grid alignment.
pub fn tiled_buffer_size_texels(width: u32, height: u32, element_pitch: usize) -> usize {
    let w = (width.max(1) as usize + 31) & !31;
    let h = (height.max(1) as usize + 31) & !31;
    w * h * element_pitch
}

/// 16-bit byte-pair swap in place. Xbox 360 stores DXT block bytes
/// pair-swapped on disk because the GPU reads them as big-endian u16
/// words (the block-color endpoints are u16s).
pub fn swap16_in_place(buf: &mut [u8]) {
    let mut i = 0;
    while i + 1 < buf.len() {
        buf.swap(i, i + 1);
        i += 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_then_untile_is_identity_dxt1() {
        // 64×64 DXT1 → 16×16 blocks (well above the 32-block tile
        // alignment, so we exercise the macro + micro paths). Fill
        // each block with a unique fingerprint so a swap shows up.
        let bw = 16usize;
        let bh = 16usize;
        let mut linear = vec![0u8; bw * bh * 8];
        for (i, chunk) in linear.chunks_exact_mut(8).enumerate() {
            chunk[0] = i as u8;
            chunk[1] = (i >> 8) as u8;
            chunk[7] = 0xA5;
        }
        let tiled = tile_dxt(&linear, 64, 64, 8);
        let back = untile_dxt(&tiled, 64, 64, 8);
        for i in 0..(bw * bh) {
            let want = &linear[i * 8..i * 8 + 8];
            let got = &back[i * 8..i * 8 + 8];
            assert_eq!(
                want,
                got,
                "block {i} (bx={}, by={}) mismatch",
                i % bw,
                i / bw
            );
        }
    }

    #[test]
    fn tile_then_untile_is_identity_dxt3() {
        let bw = 32usize;
        let bh = 8usize;
        let mut linear = vec![0u8; bw * bh * 16];
        for (i, chunk) in linear.chunks_exact_mut(16).enumerate() {
            chunk[0] = (i * 7) as u8;
            chunk[15] = (i * 11) as u8;
        }
        let tiled = tile_dxt(&linear, (bw * 4) as u32, (bh * 4) as u32, 16);
        let back = untile_dxt(&tiled, (bw * 4) as u32, (bh * 4) as u32, 16);
        assert_eq!(back[..linear.len()], linear[..]);
    }

    #[test]
    fn swap16_is_its_own_inverse() {
        let mut buf = [1u8, 2, 3, 4, 5, 6, 7, 8];
        let original = buf;
        swap16_in_place(&mut buf);
        assert_eq!(buf, [2, 1, 4, 3, 6, 5, 8, 7]);
        swap16_in_place(&mut buf);
        assert_eq!(buf, original);
    }

    #[test]
    fn rejects_short_input() {
        let err = parse(b"\x89RW4xb2").unwrap_err();
        assert!(matches!(err, TexlibError::TooShort { .. }));
    }

    #[test]
    fn rejects_bad_magic() {
        let mut data = vec![0u8; 0x100];
        data[..7].copy_from_slice(b"NOTRW4!");
        let err = parse(&data).unwrap_err();
        assert!(matches!(err, TexlibError::BadMagic));
    }

    #[test]
    fn base_mip_size_matches_block_arithmetic() {
        assert_eq!(
            base_mip_byte_size(PixelFormat::Dxt1, 512, 512),
            128 * 128 * 8
        );
        assert_eq!(
            base_mip_byte_size(PixelFormat::Dxt3, 256, 256),
            64 * 64 * 16
        );
        assert_eq!(
            base_mip_byte_size(PixelFormat::Dxt5, 1024, 1024),
            256 * 256 * 16
        );
        // Non-power-of-2 dims still round up to whole 4×4 blocks.
        assert_eq!(base_mip_byte_size(PixelFormat::Dxt1, 5, 5), 2 * 2 * 8);
        // Uncompressed 8_8_8_8 is plain w*h*4 (no block rounding).
        assert_eq!(
            base_mip_byte_size(PixelFormat::Argb8888, 256, 256),
            256 * 256 * 4
        );
        assert_eq!(base_mip_byte_size(PixelFormat::Argb8888, 5, 5), 5 * 5 * 4);
    }

    #[test]
    fn argb8888_format_byte_round_trips() {
        assert_eq!(PixelFormat::from_byte(0x86), Some(PixelFormat::Argb8888));
        assert_eq!(PixelFormat::Argb8888.to_byte(), 0x86);
        assert!(!PixelFormat::Argb8888.is_dxt());
        assert!(PixelFormat::Dxt1.is_dxt());
        assert_eq!(PixelFormat::Argb8888.fourcc(), None);
        assert_eq!(PixelFormat::Dxt5.fourcc(), Some(b"DXT5"));
    }

    #[test]
    fn tile_then_untile_is_identity_argb8888() {
        // 64×64 uncompressed → a 64×64 grid of 4-byte texels (> the
        // 32-element macro-tile, so macro + micro paths both run).
        let (w, h) = (64u32, 64u32);
        let mut linear = vec![0u8; (w * h * 4) as usize];
        for (i, px) in linear.chunks_exact_mut(4).enumerate() {
            px[0] = i as u8; // A
            px[1] = (i >> 8) as u8; // R
            px[2] = (i * 3) as u8; // G
            px[3] = 0xC3; // B fingerprint
        }
        let tiled = tile(&linear, w, h, 4);
        let back = untile(&tiled, w, h, 4);
        assert_eq!(
            back, linear,
            "per-texel tile→untile must be identity at pitch 4"
        );
    }

    /// Live fixture test: requires the operator's real Xbox 360 cache
    /// at the path below. Skipped silently when absent (CI machines
    /// won't have the cache mounted).
    #[test]
    fn parses_real_texlib_67() {
        let path = r"H:\Emulators\games\Decompiled\Original\rendering\player\texlib_67.rx2";
        let Ok(data) = std::fs::read(path) else {
            eprintln!("skipping: {path} not present");
            return;
        };
        let parsed = parse(&data).expect("real file must parse");
        assert_eq!(parsed.textures.len(), 2, "texlib_67 carries hair + head");
        let names: Vec<&str> = parsed.textures.iter().map(|t| t.name.as_str()).collect();
        assert!(names.contains(&"hair_67_cm.Raster"), "names = {names:?}");
        assert!(names.contains(&"head_67_cm.Raster"), "names = {names:?}");
        let head = parsed
            .textures
            .iter()
            .find(|t| t.name == "head_67_cm.Raster")
            .unwrap();
        assert_eq!(head.width, 512);
        assert_eq!(head.height, 512);
        assert_eq!(head.format, PixelFormat::Dxt1);
        assert_eq!(head.base_mip_linear.len(), 128 * 128 * 8);
        let hair = parsed
            .textures
            .iter()
            .find(|t| t.name == "hair_67_cm.Raster")
            .unwrap();
        assert_eq!(hair.width, 256);
        assert_eq!(hair.height, 256);
        assert_eq!(hair.format, PixelFormat::Dxt3);
    }

    /// Path to the working-assets copy of `texlib_67.rx2`. Bundled in
    /// the repo so this test runs in CI; the legacy operator cache at
    /// `H:\\Emulators\\...` is no longer mounted.
    const TEMPLATE_PATH: &str = r"E:\Repositories\nhl-database-studio\Working Assets\Template Texture\NHL Legacy\texlib_67.rx2";

    fn load_template() -> Option<Vec<u8>> {
        match std::fs::read(TEMPLATE_PATH) {
            Ok(b) => Some(b),
            Err(_) => {
                eprintln!("skipping: {TEMPLATE_PATH} not present");
                None
            }
        }
    }

    /// Real 1024 × 1024 DXT3 reference, pulled from the modded Xbox 360
    /// cache (`rendering/icesurface/centerlogo_0_cm.rx2`) and bundled so
    /// the third-resolution descriptor is pinned without depending on the
    /// H: mount. This is the ground truth that confirmed the DW8 base-
    /// address and DW12 mip-start-page fixes (see
    /// `make_descriptor_matches_real_centerlogo_1024`).
    const CENTERLOGO_1024_PATH: &str = r"E:\Repositories\nhl-database-studio\Working Assets\Template Texture\NHL Legacy\centerlogo_0_cm.rx2";

    /// **Cross-validation #1.** `make_descriptor(256, 256, DXT3, …)`
    /// must reproduce hair's actual descriptor bytes from
    /// `texlib_67.rx2` exactly. If this drifts, the 2-point pattern
    /// has changed — likely because we added a third reference whose
    /// bits override one of the "unverified" formulas.
    #[test]
    fn make_descriptor_matches_hair_256_dxt3() {
        let Some(data) = load_template() else { return };
        let lib = parse(&data).expect("template must parse");
        let hair = lib
            .textures
            .iter()
            .find(|t| t.name == "hair_67_cm.Raster")
            .expect("hair texture must be present");
        let desc_off = hair.descriptor_offset();
        let real = &data[desc_off..desc_off + DESCRIPTOR_LEN];
        // hair's pixel buffer is the first slot (data_rel_off = 0).
        let synth = make_descriptor(256, 256, PixelFormat::Dxt3, 5, hair.data_offset() as u32);
        if real != synth.as_slice() {
            eprintln!("real  : {:02x?}", real);
            eprintln!("synth : {:02x?}", synth);
            for i in 0..DESCRIPTOR_LEN {
                if real[i] != synth[i] {
                    eprintln!(
                        "  diff @ 0x{i:02x}: real={:02x} synth={:02x}",
                        real[i], synth[i]
                    );
                }
            }
            panic!("hair descriptor does not match make_descriptor output");
        }
    }

    /// **Cross-validation #2.** `make_descriptor(512, 512, DXT1, …)`
    /// must reproduce head's actual descriptor bytes exactly.
    #[test]
    fn make_descriptor_matches_head_512_dxt1() {
        let Some(data) = load_template() else { return };
        let lib = parse(&data).expect("template must parse");
        let head = lib
            .textures
            .iter()
            .find(|t| t.name == "head_67_cm.Raster")
            .expect("head texture must be present");
        let desc_off = head.descriptor_offset();
        let real = &data[desc_off..desc_off + DESCRIPTOR_LEN];
        // head's pixel buffer is the second slot (data_rel_off = 0x20000).
        let synth = make_descriptor(512, 512, PixelFormat::Dxt1, 6, head.data_offset() as u32);
        if real != synth.as_slice() {
            eprintln!("real  : {:02x?}", real);
            eprintln!("synth : {:02x?}", synth);
            for i in 0..DESCRIPTOR_LEN {
                if real[i] != synth[i] {
                    eprintln!(
                        "  diff @ 0x{i:02x}: real={:02x} synth={:02x}",
                        real[i], synth[i]
                    );
                }
            }
            panic!("head descriptor does not match make_descriptor output");
        }
    }

    /// **Ground-truth #3 — real 1024 × 1024 DXT3.** `make_descriptor`
    /// must reproduce the actual descriptor of `centerlogo_0_cm.rx2`
    /// (pulled from the modded Xbox 360 cache) byte-for-byte. This is the
    /// reference that settled the 1024 encoding: it confirmed DW8 is a
    /// base address (centerlogo data_off=0 → 0x00000053) and that DW12 is
    /// the mip-start page count (1024² DXT3 base = 0x100000 >> 16 = 0x10),
    /// NOT `log2−7`. Both of those were the 2026-05-28 in-game crash.
    #[test]
    fn make_descriptor_matches_real_centerlogo_1024() {
        let data = match std::fs::read(CENTERLOGO_1024_PATH) {
            Ok(b) => b,
            Err(_) => {
                eprintln!("skipping: {CENTERLOGO_1024_PATH} not present");
                return;
            }
        };
        let lib = parse(&data).expect("centerlogo must parse");
        let tex = &lib.textures[0];
        assert_eq!((tex.width, tex.height), (1024, 1024));
        assert_eq!(tex.format, PixelFormat::Dxt3);
        let desc_off = tex.descriptor_offset();
        let real = &data[desc_off..desc_off + DESCRIPTOR_LEN];
        let synth = make_descriptor(
            tex.width,
            tex.height,
            tex.format,
            11,
            tex.data_offset() as u32,
        );
        if real != synth.as_slice() {
            eprintln!("real  : {real:02x?}");
            eprintln!("synth : {synth:02x?}");
            for i in 0..DESCRIPTOR_LEN {
                if real[i] != synth[i] {
                    eprintln!(
                        "  diff @ 0x{i:02x}: real={:02x} synth={:02x}",
                        real[i], synth[i]
                    );
                }
            }
            panic!("centerlogo 1024 descriptor does not match make_descriptor output");
        }
    }

    /// **1024 prediction lock-in (DXT1 head variant).** Complements the
    /// real-DXT3 ground truth above by pinning the DXT1 head case (half
    /// the base-mip size → different DW12 page count) at the realistic
    /// 0x20000 slot offset a resized head lands at. DW8 = 0x00002052
    /// (base 0x2000 | format 0x52); DW12 = 0x0008 pages (1024² DXT1 base
    /// = 0x80000 >> 16 = 8). The old code emitted DW8=0x00004052 and
    /// DW12=0x0003 — the corruption + crash.
    #[test]
    fn make_descriptor_predicts_1024_dxt1_head() {
        let synth = make_descriptor(1024, 1024, PixelFormat::Dxt1, 7, 0x20000);
        let expected: [u8; DESCRIPTOR_LEN] = [
            0x00, 0x40, 0x00, 0x03, // DW0 constant
            0x01, 0x00, 0x00, 0x00, // DW1 constant
            0x00, 0x00, 0x00, 0x00, // DW2
            0x00, 0x00, 0x00, 0x00, // DW3
            0x00, 0x00, 0x00, 0x00, // DW4
            0xFF, 0xFF, 0x00, 0x00, // DW5 color-key
            0xFF, 0xFF, 0x00, 0x00, // DW6 color-key
            0x88, 0x00, 0x00, 0x02, // DW7 tiled + pitch (1024/128 = 8 → 0x88)
            0x00, 0x00, 0x20, 0x52, // DW8 base 0x2000 | format 0x52  ← the fix
            0x00, 0x7F, 0xE3, 0xFF, // DW9 size: h-1=1023, w-1=1023
            0x00, 0x00, 0x0D, 0x10, // DW10 constant
            0x00, 0x00, 0x02, 0x80, // DW11 max_mip_level = 10
            0x00, 0x08, 0x0A,
            0x00, // DW12 mip-start pages: 1024² DXT1 base = 0x80000 >> 16 = 8
            0x08, 0x00, 0x0B, 0x00, // DW13 log2+1 = 0x0B
        ];
        assert_eq!(
            synth.as_slice(),
            expected.as_slice(),
            "1024 descriptor diverged from the predicted Xenos encoding"
        );
    }

    /// **Reflow regression #1.** Resizing a slot to its *current*
    /// dimensions + format with a self-derived mip chain must produce
    /// a file that re-parses with the same texture metadata, and the
    /// base mip must round-trip through tile/swap/untile.
    #[test]
    fn replace_with_resize_same_dims_round_trip() {
        let Some(data) = load_template() else { return };
        let lib = parse(&data).expect("template must parse");
        let head_idx = lib
            .textures
            .iter()
            .position(|t| t.name == "head_67_cm.Raster")
            .unwrap();
        let head = &lib.textures[head_idx];
        // Build a single-mip chain at the current dims from the
        // already-untiled base mip — confirms our tile-back path is
        // bijective even when we pass through replace_with_resize.
        let mip = LinearMip {
            width: head.width,
            height: head.height,
            linear_dxt: &head.base_mip_linear,
        };
        let rewritten = lib
            .replace_with_resize(head_idx, head.format, &[mip])
            .expect("resize same-dims must succeed");
        let reparsed = parse(&rewritten).expect("rewritten file must re-parse");
        let new_head = reparsed
            .textures
            .iter()
            .find(|t| t.name == "head_67_cm.Raster")
            .unwrap();
        assert_eq!(new_head.width, 512);
        assert_eq!(new_head.height, 512);
        assert_eq!(new_head.format, PixelFormat::Dxt1);
        assert_eq!(
            new_head.base_mip_linear, head.base_mip_linear,
            "base mip must survive tile→swap→untile through replace_with_resize"
        );
        // The hair texture (which sits before head in the data region)
        // must be untouched.
        let new_hair = reparsed
            .textures
            .iter()
            .find(|t| t.name == "hair_67_cm.Raster")
            .unwrap();
        let orig_hair = lib
            .textures
            .iter()
            .find(|t| t.name == "hair_67_cm.Raster")
            .unwrap();
        assert_eq!(new_hair.base_mip_linear, orig_hair.base_mip_linear);
    }

    /// **Reflow regression #2.** Resizing hair (256×256 DXT3, the
    /// *earlier* slot in the data region) upward forces the head
    /// buffer to shift forward — exercises the file-table patching
    /// path that `replace_with_resize` runs for the non-target slot.
    #[test]
    fn replace_with_resize_grows_earlier_slot() {
        let Some(data) = load_template() else { return };
        let lib = parse(&data).expect("template must parse");
        let hair_idx = lib
            .textures
            .iter()
            .position(|t| t.name == "hair_67_cm.Raster")
            .unwrap();
        let head_orig = lib
            .textures
            .iter()
            .find(|t| t.name == "head_67_cm.Raster")
            .unwrap()
            .clone();

        // Synthesize a 512×512 DXT3 base mip + one mip level (256×256)
        // as zero bytes — content is irrelevant for this test, we only
        // care that the reflow + file-table updates are consistent.
        let base = vec![0u8; base_mip_byte_size(PixelFormat::Dxt3, 512, 512)];
        let mip1 = vec![0u8; base_mip_byte_size(PixelFormat::Dxt3, 256, 256)];
        let mips = [
            LinearMip {
                width: 512,
                height: 512,
                linear_dxt: &base,
            },
            LinearMip {
                width: 256,
                height: 256,
                linear_dxt: &mip1,
            },
        ];
        let rewritten = lib
            .replace_with_resize(hair_idx, PixelFormat::Dxt3, &mips)
            .expect("grow earlier slot must succeed");
        let reparsed = parse(&rewritten).expect("rewritten file must re-parse");
        let new_hair = reparsed
            .textures
            .iter()
            .find(|t| t.name == "hair_67_cm.Raster")
            .unwrap();
        assert_eq!(new_hair.width, 512);
        assert_eq!(new_hair.height, 512);
        assert_eq!(new_hair.format, PixelFormat::Dxt3);
        // head must still parse, with original dims + format.
        let new_head = reparsed
            .textures
            .iter()
            .find(|t| t.name == "head_67_cm.Raster")
            .unwrap();
        assert_eq!(new_head.width, head_orig.width);
        assert_eq!(new_head.height, head_orig.height);
        assert_eq!(new_head.format, head_orig.format);
        assert_eq!(
            new_head.base_mip_linear, head_orig.base_mip_linear,
            "head base mip must round-trip through the reflow shift"
        );
    }

    /// Same-size replacement round-trip: parse → replace one texture
    /// with bytes derived from itself → parse the result → expect the
    /// new base mip to come back exactly. Confirms tile/untile cancel
    /// and that the splice preserves every other byte.
    #[test]
    fn replace_same_size_round_trip() {
        let path = r"H:\Emulators\games\Decompiled\Original\rendering\player\texlib_67.rx2";
        let Ok(data) = std::fs::read(path) else {
            eprintln!("skipping: {path} not present");
            return;
        };
        let parsed = parse(&data).unwrap();
        // Build a "new" base mip by inverting every byte — distinct
        // from the original, same length.
        let head_idx = parsed
            .textures
            .iter()
            .position(|t| t.name == "head_67_cm.Raster")
            .unwrap();
        let new_base: Vec<u8> = parsed.textures[head_idx]
            .base_mip_linear
            .iter()
            .map(|b| !*b)
            .collect();
        let rewritten = parsed.replace_same_size(head_idx, &new_base).unwrap();
        assert_eq!(rewritten.len(), data.len(), "same-size keeps file size");
        let reparsed = parse(&rewritten).unwrap();
        assert_eq!(
            reparsed.textures[head_idx].base_mip_linear, new_base,
            "rewritten base mip must round-trip through tile/swap/untile"
        );
        // Other texture untouched.
        let other_idx = 1 - head_idx;
        assert_eq!(
            reparsed.textures[other_idx].base_mip_linear,
            parsed.textures[other_idx].base_mip_linear,
            "other texture must be byte-identical after replace"
        );
    }
}
