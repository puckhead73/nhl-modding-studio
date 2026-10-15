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
/// The Xenos texture fetch-constant's first word (`GPUTEXTURE_FETCH_CONSTANT` dword 0) sits five
/// bytes before [`DESC_FIELD_OFF`] — one dword ahead of the format word. Its **top bit** (`0x8000_0000`)
/// is the `tiled` flag and bits 22–30 hold the `pitch` (in 32-texel blocks). This is the reliable,
/// cross-title discriminator between an Xbox-tiled `Argb8888` (FE art — portraits, awards, logos) and
/// a LINEAR one (rendering-cache `name_*` jersey nameplates): at 32-aligned dimensions the two
/// buffers are the same byte length, and a nearby format-word flag turned out to be title-specific
/// (set on NHL 07 nameplates, clear on NHL 09's), whereas this `tiled` bit is clear on every linear
/// nameplate observed and set on every tiled FE texture. Verified: the pitch field also reproduces
/// each texture's width (award 16×32 = 512, nameplate 8×32 = 256).
const DESC_FETCH_DW0_REL: usize = DESC_FIELD_OFF - 5;
pub const DESC_FETCH_TILED_BIT: u32 = 0x8000_0000;

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
    /// The Xenos fetch-constant `tiled` bit (see [`DESC_FETCH_TILED_BIT`]). DXT slots are always
    /// tiled; an `Argb8888` slot is tiled for FE art (portraits, awards, NHL 07–11 league logos)
    /// and linear for the rendering-cache nameplates. The same-size writers mirror it.
    pub tiled: bool,
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
    /// File-table index of this texture's pixel-data entry — the value `DW8`'s upper field
    /// carries (research 2026-09-21 pm handoff R42 §1); see [`descriptor_data_entry_index`].
    pub fn data_entry_index(&self) -> u32 {
        self.slot.tex_data_entry_idx
    }

    pub fn descriptor_offset(&self) -> usize {
        self.slot.desc_off as usize
    }

    /// Byte offset (relative to `hdr_size`) of this texture's tiled
    /// pixel buffer inside the data region.
    pub fn data_offset(&self) -> usize {
        self.slot.data_rel_off as usize
    }
}

/// A level at or below 16 × 16 blocks (64 texels on either axis): the console's **packed tail**,
/// whose layout Studio's sequential model does not reproduce. The split is the one the mip
/// placement audit of 22 Sep 2026 (`tests/rw4xb2_mip_placement.rs`) found by content: above it the
/// sequential offsets decode to the shrunken base on every audited level; at or below it a
/// substantial share does not. No other measurement is cited for the boundary.
pub fn is_packed_tail_level(width: u32, height: u32) -> bool {
    width.div_ceil(4).max(1) <= 16 || height.div_ceil(4).max(1) <= 16
}

/// The audited population of the mip placement audit (22 Sep 2026), as an **applicability gate**
/// for rewriting a raster's large levels at the sequential offsets. Everything outside it falls
/// back to a base-only write — the base level's tiling is validated by every preview Studio draws,
/// independently of the audit — and says why.
///
/// The gate is what the file's own bytes can establish. It does **not** encode the title: the
/// audit ran on real Xbox 360 trees named in the audit record, and rasters of other titles that
/// share the same descriptor form pass the gate by form, not by an audit of their own.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LargeLevelSupport {
    /// The raster is inside the audited population: tiled 2D `DXT1` / `DXT3` / `DXT5`, both axes
    /// 256–1024 texels, the NHL 09–Legacy descriptor form (`DW0 = 0x00400003`), two or more
    /// declared levels. Its large levels (above 16 × 16 blocks) may be rewritten at the sequential
    /// offsets.
    LargeLevels {
        /// Declared levels above the packed-tail split, base excluded.
        large_levels: u32,
    },
    /// Outside the population: only the base level is rewritten; `reason` names the excluded
    /// property (format, shape, descriptor form, cube, single level).
    BaseOnly { reason: String },
}

/// Smallest and largest base dimension the mip placement audit examined (`≥ 256` on both axes;
/// the largest shipped raster in the audited families is 1 024²).
pub const AUDITED_MIN_DIM: u32 = 256;
pub const AUDITED_MAX_DIM: u32 = 1024;

/// What [`Texlib::replace_large_levels_keep_tail`] wrote and what it left alone.
#[derive(Debug, Clone)]
pub struct KeepTailWrite {
    pub bytes: Vec<u8>,
    /// Which route the gate selected for this raster.
    pub support: LargeLevelSupport,
    /// The base plus every large level written (1 on the base-only route, or when the base itself
    /// is a tail-sized raster).
    pub levels_written: u32,
    /// Declared levels below the last written one, kept as shipped.
    pub tail_levels_kept: u32,
    /// Slot bytes after the last written level, kept as shipped.
    pub tail_bytes_kept: usize,
    /// The **byte-write mask**: every `[start, end)` range of the output file whose bytes were
    /// written, in file order. Every byte outside these ranges is identical to the input — the
    /// preservation test asserts it, and a caller can re-check it against its own copy.
    pub written_ranges: Vec<(usize, usize)>,
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
    /// The container's header size: every slot's `data_offset` is relative to it, so
    /// `header_size() + data_offset()` is the absolute file offset of a slot's tiled buffer.
    pub fn header_size(&self) -> u32 {
        self.hdr_size
    }

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
    /// - The target's 56-byte descriptor is re-derived from its shipped
    ///   bytes via [`patch_descriptor_for_resize`] (research 2026-09-21
    ///   handoff 11 §2: pitch, `DW9`, `DW11`, `DW12`, `DW13` recomputed;
    ///   `DW8`'s slot reference and every constant kept). The rule is
    ///   censused against every loose Legacy / NHL 11 / NHL 09 raster
    ///   (`tests/rw4xb2_descriptor_census.rs`); its refusals (NHL 07 /
    ///   08 layouts, linear rasters, cubes, rasters under 128 texels)
    ///   are returned as [`TexlibError::ResizeRule`].
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
        // Resize regenerates the Xenos descriptor (DW7 / DW9 / DW11–13). DXT: any measured chain.
        // Uncompressed `8_8_8_8`: only a **single-level, tiled** raster of the same format — the
        // front-end art (logos, portraits) whose descriptor rule research R132 measured on 6 926 /
        // 6 926 NHL 08–11 rasters (pitch = align(w, 32) / 32 texel units, DW9 = (w−1) | (h−1) << 13,
        // payload = align(w, 32) × align(h, 32) × 4, every other byte constant). Linear `8_8_8_8`
        // (nameplates) and uncompressed chains are refused.
        let argb_single = matches!(new_format, PixelFormat::Argb8888) && tex.format == new_format && tex.tiled && mips.len() == 1;
        if !new_format.is_dxt() && !argb_single {
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

        // The NHL 07 container form (research R42 §1: file table at EOF, untagged payload entries,
        // `data_size` 0 and absolute buffer offsets) is parsed with `hdr_size` forced to 0. The
        // reflow below rewrites the `0x54` data-size word and re-bases every `data_rel_off`
        // against `hdr_size`, neither of which describes that layout — so the *descriptor* rule now
        // covers NHL 07 / 08 (`DESC_MEASURED_DW0`) while the *container* writer still refuses this
        // one, with its own reason instead of a generation refusal.
        if self.hdr_size == 0 {
            return Err(TexlibError::ResizeRule(
                "this container keeps its buffers in the header region with absolute offsets (the NHL 07 table-at-EOF layout); its reflow is not implemented, so a different-size replacement is refused".into(),
            ));
        }
        // The descriptor first (coordinator review R1 F1, 21 Sep 2026): the rule's refusals —
        // including a size or level count its fields cannot store — happen before any buffer is
        // built, so an unrepresentable request never reaches the reflow.
        let mip_count = mips.len() as u32;
        let desc_off = tex.slot.desc_off as usize;
        if desc_off + DESCRIPTOR_LEN > self.raw.len() {
            return Err(TexlibError::TruncatedDescriptor { off: desc_off });
        }
        let orig_desc: [u8; DESCRIPTOR_LEN] = self.raw[desc_off..desc_off + DESCRIPTOR_LEN].try_into().expect("bounds checked");
        // `DW8` is preserved verbatim by the patch (this reflow keeps the file table's membership
        // and order, so the index it holds stays correct). Research 2026-09-21 pm handoff R42 §1
        // turned that word from "a slot reference" into an exact join — the file-table index of
        // this texture's pixel-data entry — so it is now *checked* rather than assumed: a
        // container whose descriptor points elsewhere is one whose pairing Studio reads
        // differently, and rewriting it would move a raster's pixels out from under its
        // descriptor.
        let dw8_index = descriptor_data_entry_index(&orig_desc);
        if dw8_index != tex.slot.tex_data_entry_idx {
            return Err(TexlibError::ResizeRule(
                DescriptorRuleRefusal::SlotJoinMismatch { dw8_index, data_entry_index: tex.slot.tex_data_entry_idx }.to_string(),
            ));
        }
        let new_desc = patch_descriptor_for_resize(&orig_desc, new_width, new_height, new_format, mip_count).map_err(|e| TexlibError::ResizeRule(e.to_string()))?;
        debug_assert_eq!(descriptor_data_entry_index(&new_desc), dw8_index, "the patch must preserve DW8's index");

        // Validate every mip's payload size matches its declared dims, and that the chain halves.
        let bpb = new_format.bytes_per_block();
        for (i, mip) in mips.iter().enumerate() {
            let (ew, eh) = ((new_width >> i).max(1), (new_height >> i).max(1));
            if (mip.width, mip.height) != (ew, eh) {
                return Err(TexlibError::ResizeRule(format!("mip level {i} is {}x{}, expected {ew}x{eh} (the chain must halve from {new_width}x{new_height})", mip.width, mip.height)));
            }
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
            if new_format.is_dxt() {
                let tiled = tile_dxt(mip.linear_dxt, mip.width, mip.height, bpb);
                let mut swapped = tiled;
                swap16_in_place(&mut swapped);
                new_buf.extend_from_slice(&swapped);
            } else {
                // `8_8_8_8` FE art: tiled per texel, no byte swap — the same encoding
                // `replace_same_size` writes (and the parse side reads) for a tiled slot; the
                // buffer is align(w, 32) × align(h, 32) × 4 bytes (a multiple of 0x1000).
                new_buf.extend_from_slice(&tile(mip.linear_dxt, mip.width, mip.height, 4));
            }
        }
        let new_buf_size = (new_buf.len() + 0xFFF) & !0xFFF;
        if u32::try_from(new_buf_size).is_err() {
            return Err(TexlibError::DataRegionOverflow);
        }
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
        // The raster's pixels must be one of the tagged data entries, or the reflow below would
        // re-describe a buffer it never writes (some NHL 07 FE-art members carry no tagged payload
        // entry — research R132d): refuse before building anything.
        if !data_entries.iter().any(|e| e.entry_idx as u32 == tex.slot.tex_data_entry_idx) {
            return Err(TexlibError::ResizeRule(format!(
                "the raster's pixel data (file-table entry {}) is not a tagged data entry; its layout is not read, so a different-size replacement is refused",
                tex.slot.tex_data_entry_idx
            )));
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

        // Rewrite the target's descriptor in place from its shipped bytes: `DW8` is a slot
        // reference (2 × the descriptor index), not a byte address, so it is kept as is even
        // though the slot's data offset moved (research 2026-09-21 handoff 11 §2 corrected the
        // earlier `data_off >> 4` reading, which only ever matched slot 0).
        out[desc_off..desc_off + DESCRIPTOR_LEN].copy_from_slice(&new_desc);

        // Read the result back: the target must parse at the requested size, format and mip count.
        let re = parse(&out)?;
        match re.textures.get(idx) {
            Some(t) if (t.width, t.height, t.format) == (new_width, new_height, new_format) && descriptor_mip_count(&new_desc) == mip_count => Ok(out),
            Some(t) => Err(TexlibError::ResizeRule(format!("the rewritten container reads back as {}x{} {:?}, not the requested {new_width}x{new_height} {new_format:?}", t.width, t.height, t.format))),
            None => Err(TexlibError::ResizeRule(format!("the rewritten container no longer lists texture {idx}"))),
        }
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

        // DXT: tile by 4×4 block + 16-bit-pair swap. Argb8888: mirror the parse side — tiled
        // per texel (no swap) when the descriptor's tiled bit is set (FE art), else the
        // `[A,R,G,B]` base mip goes straight back (linear nameplates).
        let swapped = if tex.format.is_dxt() {
            let mut t = tile_dxt(
                new_dxt_base,
                tex.width,
                tex.height,
                tex.format.bytes_per_block(),
            );
            swap16_in_place(&mut t);
            t
        } else if tex.tiled {
            tile(new_dxt_base, tex.width, tex.height, 4)
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

    /// The declared mip count of texture `idx` (1 when the descriptor is unreadable).
    pub fn declared_mip_count(&self, idx: usize) -> u32 {
        let Some(tex) = self.textures.get(idx) else { return 1 };
        let desc_off = tex.slot.desc_off as usize;
        if desc_off + DESCRIPTOR_LEN <= self.raw.len() {
            let d: [u8; DESCRIPTOR_LEN] = self.raw[desc_off..desc_off + DESCRIPTOR_LEN].try_into().expect("bounds checked");
            descriptor_mip_count(&d).max(1)
        } else {
            1
        }
    }

    /// The applicability gate of [`Texlib::replace_large_levels_keep_tail`] for texture `idx`:
    /// whether its large levels may be rewritten at the sequential offsets (the audited
    /// population) or only the base (everything else), with the reason. See
    /// [`LargeLevelSupport`].
    pub fn large_level_support(&self, idx: usize) -> Result<LargeLevelSupport, TexlibError> {
        let tex = self
            .textures
            .get(idx)
            .ok_or(TexlibError::TextureIndexOutOfRange {
                idx,
                count: self.textures.len(),
            })?;
        let base_only = |reason: String| Ok(LargeLevelSupport::BaseOnly { reason });
        if !tex.format.is_dxt() {
            return base_only(format!("{:?} is not a DXT format; the audit decoded DXT1 / DXT3 / DXT5 only", tex.format));
        }
        if !tex.tiled {
            return base_only("the raster is stored linear, not tiled; the audit examined tiled rasters only".into());
        }
        let desc_off = tex.slot.desc_off as usize;
        if desc_off + DESCRIPTOR_LEN > self.raw.len() {
            return base_only("the descriptor is truncated, so the declared level count is unknown".into());
        }
        let d: [u8; DESCRIPTOR_LEN] = self.raw[desc_off..desc_off + DESCRIPTOR_LEN].try_into().expect("bounds checked");
        let dw0 = u32::from_be_bytes([d[0], d[1], d[2], d[3]]);
        if dw0 != DESC_MEASURED_DW0[0] {
            return base_only(format!(
                "descriptor form DW0 = 0x{dw0:08X} is not the NHL 09–Legacy form (0x{:08X}) the audit population carries",
                DESC_MEASURED_DW0[0]
            ));
        }
        let declared = descriptor_mip_count(&d).max(1);
        if declared < 2 {
            return base_only("the raster declares a single level; there are no smaller levels to place".into());
        }
        if tex.width < AUDITED_MIN_DIM || tex.height < AUDITED_MIN_DIM || tex.width > AUDITED_MAX_DIM || tex.height > AUDITED_MAX_DIM {
            return base_only(format!(
                "{}×{} is outside the audited shapes ({AUDITED_MIN_DIM}–{AUDITED_MAX_DIM} texels on both axes)",
                tex.width, tex.height
            ));
        }
        let bpb = tex.format.bytes_per_block();
        let chain: usize = (0..declared).map(|l| tiled_buffer_size((tex.width >> l).max(1), (tex.height >> l).max(1), bpb)).sum();
        if tex.slot.size as usize >= 6 * chain {
            return base_only("the slot holds at least six chains (a cube map); the audit excluded cubes".into());
        }
        let large_levels = (1..declared).filter(|&l| !is_packed_tail_level((tex.width >> l).max(1), (tex.height >> l).max(1))).count() as u32;
        Ok(LargeLevelSupport::LargeLevels { large_levels })
    }

    /// Rewrite the base level and — for a raster inside the audited population
    /// ([`Texlib::large_level_support`]) — every **large** level of texture `idx` at the offsets
    /// Studio's sequential model predicts, keeping the shipped **packed tail** (every level at or
    /// below 16 × 16 blocks, 64 texels on either axis) byte-for-byte. Outside the population only
    /// the base is rewritten and every other byte of the slot is kept.
    ///
    /// Why the split (mip placement audit, 22 Sep 2026, `tests/rw4xb2_mip_placement.rs`, RGB
    /// content comparison on real Xbox 360 rasters): decoding each shipped level at the model's
    /// offset and comparing it with a box-downscaled base matches on every audited level above
    /// 16 blocks and fails to match on a substantial share of the tail levels; Studio's per-level
    /// 32-block tiling also allocates more than every shipped slot. That is static evidence about
    /// placement, not a runtime observation. (Historical note, kept as attributed context only: a
    /// source comment introduced with commit `99ce2a5` recorded an earlier in-game finding, "PR11:
    /// players rendered debug-blue at gameplay distance" after a full-chain rewrite; its original
    /// evidence is not in this repository.) Until an independent packed-tail contract exists, this
    /// writer changes only what the audit covers and leaves the authored tail in place — the kept
    /// levels then carry the *previous* image's content, which the caller must disclose as a
    /// partial replacement.
    ///
    /// Same base constraints as [`Texlib::replace_with_mip_chain`]; nothing after the last written
    /// level is zeroed, and [`KeepTailWrite::written_ranges`] is the exact byte-write mask.
    pub fn replace_large_levels_keep_tail(
        &self,
        idx: usize,
        mips: &[LinearMip<'_>],
    ) -> Result<KeepTailWrite, TexlibError> {
        let tex = self
            .textures
            .get(idx)
            .ok_or(TexlibError::TextureIndexOutOfRange {
                idx,
                count: self.textures.len(),
            })?;
        let expected = base_mip_byte_size(tex.format, tex.width, tex.height);
        let base = mips.first().ok_or(TexlibError::ReplacementSizeMismatch { expected, have: 0 })?;
        if base.width != tex.width || base.height != tex.height || base.linear_dxt.len() != expected {
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
        let declared = self.declared_mip_count(idx);
        let support = self.large_level_support(idx)?;
        let write_large = matches!(support, LargeLevelSupport::LargeLevels { .. });

        let mut out = self.raw.clone();
        let mut cursor = 0usize;
        let mut levels_written = 0u32;
        let mut written_ranges: Vec<(usize, usize)> = Vec::new();
        for (i, mip) in mips.iter().enumerate() {
            // The base is always written; a smaller level only on the large-level route, while it
            // is a large level of the declared chain and fits the slot.
            if i > 0 && (!write_large || i as u32 >= declared || is_packed_tail_level(mip.width, mip.height)) {
                break;
            }
            let (ew, eh) = ((tex.width >> i).max(1), (tex.height >> i).max(1));
            if (mip.width, mip.height) != (ew, eh) || mip.linear_dxt.len() != base_mip_byte_size(tex.format, ew, eh) {
                return Err(TexlibError::MipSizeMismatch {
                    level: i,
                    width: mip.width,
                    height: mip.height,
                    expected: base_mip_byte_size(tex.format, ew, eh),
                    have: mip.linear_dxt.len(),
                });
            }
            let mut swapped = tile_dxt(mip.linear_dxt, mip.width, mip.height, bpb);
            swap16_in_place(&mut swapped);
            if cursor + swapped.len() > buf_size {
                break;
            }
            out[buf_start + cursor..buf_start + cursor + swapped.len()].copy_from_slice(&swapped);
            written_ranges.push((buf_start + cursor, buf_start + cursor + swapped.len()));
            cursor += swapped.len();
            levels_written += 1;
        }
        Ok(KeepTailWrite {
            bytes: out,
            support,
            levels_written,
            tail_levels_kept: declared.saturating_sub(levels_written),
            tail_bytes_kept: buf_size - cursor,
            written_ranges,
        })
    }

    /// Splice a new base mip into texture `idx` and KEEP the original
    /// trailing mip-chain bytes verbatim. The console packed-mip tail
    /// layout is not established for Studio's generated chains (mip
    /// placement audit, 22 Sep 2026). Historical note, attributed and
    /// unverified here: this comment originally cited "PR11 in-game
    /// finding: players rendered debug-blue at gameplay distance"
    /// (commit `99ce2a5`); the original evidence is not in this
    /// repository. The donor's authored tail is bit-correct by
    /// construction; what it looks like at distance is not claimed.
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
        } else if tex.tiled {
            tile(new_dxt_base, tex.width, tex.height, 4)
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
    #[error("resize refused: {0}")]
    ResizeRule(String),
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

    if no_data_region {
        // NHL07 mixed mesh/texture containers also have untagged geometry buffers.
        // DW8 names the pixel entry explicitly (measured R42); ordering/counting all
        // untagged buffers misclassifies geometry and rejects otherwise valid rasters.
        let mut referenced = Vec::with_capacity(desc_slots.len());
        let mut texture_descs = Vec::with_capacity(desc_slots.len());
        for &(desc_index, ints) in &desc_slots {
            let off = ints[0] as usize;
            if off.checked_add(DESC_WIDTH_WORD_REL + 2).is_none_or(|end| end > data.len()) {
                return Err(TexlibError::TruncatedDescriptor { off });
            }
            if PixelFormat::from_byte(data[off + DESC_FORMAT_BYTE_REL]).is_none() {
                continue;
            }
            let pixel_index = (read_u32_be(data, off + 32) >> DESC_DW8_INDEX_SHIFT) as usize;
            let slot = data_slots.iter().find(|(index, _)| *index == pixel_index)
                .ok_or(TexlibError::OrphanDescriptor { desc: desc_index })?;
            referenced.push(*slot);
            texture_descs.push((desc_index, ints));
        }
        data_slots = referenced;
        desc_slots = texture_descs;
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
        // DW9 = `(height − 1) << 13 | (width − 1)`, 13 + 13 bits (research 2026-09-21 handoff 11
        // §2). The earlier 8-bit height byte read `(h / 8 − 1)` and silently halved every raster
        // taller than 2 048 (the 512 × 4 096 `effects/lcd_*` strips) — found by the descriptor
        // census `tests/rw4xb2_descriptor_census.rs`.
        let dw9 = read_u32_be(data, desc_off + DESC_HEIGHT_BYTE_REL - 1);
        // The Xenos tiled flag (see `DESC_FETCH_TILED_BIT`): set = Xbox-tiled, clear = linear.
        let fetch_dw0 = read_u32_be(data, desc_off + DESC_FETCH_DW0_REL);
        let is_tiled = (fetch_dw0 & DESC_FETCH_TILED_BIT) != 0;
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
        let height = ((dw9 >> 13) & 0x1FFF) + 1;
        let width = (dw9 & DESC_WIDTH_VALUE_MASK as u32) + 1;

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
            // DXT: 16-bit-pair swap, then untile by 4×4 block. The tiled base level spans the
            // 32×32-block macro-tile-aligned buffer, which is larger than the linear base when a
            // dimension is under 32 blocks (a 128×64 DXT5 league logo: 8 192 linear bytes inside a
            // 16 384-byte tiled span) — read the whole span when the slot holds it, else the
            // linear-sized slice (the same rule the `Argb8888` branch applies).
            let tiled_size = tiled_buffer_size(width, height, format.bytes_per_block());
            let mut span = if (buf_size as usize) >= tiled_size && buf_start + tiled_size <= data.len() {
                data[buf_start..buf_start + tiled_size].to_vec()
            } else {
                std::mem::take(&mut base_tiled)
            };
            swap16_in_place(&mut span);
            untile_dxt(&span, width, height, format.bytes_per_block())
        } else if is_tiled {
            // Argb8888 FE art (portraits, awards, logos) is stored **Xbox-tiled**. Untile at the
            // real dims — `untile` reads through the 32-aligned pitch and crops to width×height, so
            // it handles both the padded case (e.g. 468×248 stored as a 480×256 buffer) and the
            // already-aligned case (e.g. a 512×256 award, where the tiled and linear buffers are the
            // same length and only the descriptor's tiled flag tells them apart). The full tiled
            // buffer may be larger than `base_size`, so re-slice it here.
            let tiled_size = tiled_buffer_size_texels(width, height, 4);
            if (buf_size as usize) >= tiled_size && buf_start + tiled_size <= data.len() {
                let full = &data[buf_start..buf_start + tiled_size];
                untile(full, width, height, 4)
            } else {
                base_tiled
            }
        } else {
            // Argb8888 stored LINEAR (row-major `[A, R, G, B]`) — the rendering-cache `name_*`
            // nameplates, flagged not-tiled in the descriptor. The bytes are already the image.
            base_tiled
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
            tiled: is_tiled,
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
    let bw = width.div_ceil(4).max(1) as usize;
    let bh = height.div_ceil(4).max(1) as usize;
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
    d[54] = (log2_max + 1).clamp(1, 0xFF) as u8;
    d[55] = 0x00;

    d
}

/// Why a descriptor cannot be re-derived for a resize (research 2026-09-21 handoff 11 §2: the
/// exceptions the census lists).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DescriptorRuleRefusal {
    /// `DW0` is neither of the two **measured** fetch-constant generations: `0x00400003`
    /// (NHL 09–Legacy) or `0x00000003` (NHL 07 / 08 — research 2026-09-21 pm handoff R42 §2, where
    /// every dimension-dependent word obeys the same rules: NHL 07 pitch 10 666 / 10 666 and
    /// `DW12` 9 554 / 9 555, NHL 08 14 066 / 14 066 after its RefPack wrapper is removed).
    UnmeasuredGeneration { dw0: u32 },
    /// The fetch constant's `tiled` bit is clear (a few FE 8_8_8_8 rasters): the tiled-base
    /// mip-address rule does not describe it.
    NotTiled,
    /// Cube maps (`dimension` 3): 6 × the chain; not a 2D resize.
    CubeMap,
    /// Below the tiled-base minimum: BCn textures under 128 texels in either dimension pack their
    /// mips into the base tile (`DW12` = 0 in 75 / 202 tiny cases) and 16-wide strips miss the
    /// pitch rule; refused rather than guessed.
    TooSmall { width: u32, height: u32, minimum: u32 },
    /// The requested output does not fit the descriptor's field (`DW9` 13-bit width / height,
    /// `DW11` 4-bit mip count, a chain longer than the size halves to): refused, never masked.
    Unrepresentable { what: &'static str, value: u32, max: u32 },
    /// `DW8`'s upper field (bits 12+) does not address this texture's pixel-data entry. Research
    /// 2026-09-21 pm handoff R42 measured that field as the **file-table index of the paired
    /// data entry** on 100 % of NHL 08–Legacy descriptors and 10 666 / 10 675 on NHL 07, so a
    /// descriptor that disagrees is either a container whose pairing Studio reads differently or
    /// a non-texture record — refused rather than rewritten with a word we cannot justify.
    SlotJoinMismatch { dw8_index: u32, data_entry_index: u32 },
    /// A descriptor whose format byte is null (`0x00`): the nine `fx/common/texture.xml.rx2`
    /// records on NHL 07 are not textures (handoff R42 §1) — refused by format.
    NullFormatDescriptor,
}

impl std::fmt::Display for DescriptorRuleRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnmeasuredGeneration { dw0 } => write!(f, "descriptor DW0 = {dw0:#010x} is neither measured fetch-constant generation (0x00400003 on NHL 09–Legacy, 0x00000003 on NHL 07 / 08)"),
            Self::NotTiled => write!(f, "the raster is stored linear (tiled bit clear); the resize rule covers tiled rasters only"),
            Self::CubeMap => write!(f, "the raster is a cube map; a 2D resize does not apply"),
            Self::TooSmall { width, height, minimum } => write!(f, "{width}x{height} is below the {minimum}-texel minimum the resize rule covers (tiny rasters pack their mips into the base tile)"),
            Self::Unrepresentable { what, value, max } => write!(f, "{what} {value} cannot be stored in the descriptor (1–{max})"),
            Self::SlotJoinMismatch { dw8_index, data_entry_index } => write!(f, "the descriptor's DW8 addresses file-table entry {dw8_index}, not this texture's pixel-data entry {data_entry_index}; the container's descriptor-to-slot join is not the measured one and is not rewritten"),
            Self::NullFormatDescriptor => write!(f, "the descriptor carries a null format byte: it does not describe a texture (NHL 07 `fx/common/texture.xml.rx2` records)"),
        }
    }
}

/// Re-derive the dimension-dependent fields of an existing 56-byte `RW4xb2` descriptor for a
/// resize to `width × height` with `mip_count` levels, per research 2026-09-21 handoff 11 §2
/// (censused on 190 000+ descriptors of NHL 09–Legacy): `DW7` pitch = `ceil(width / 32)` in bits
/// 22–30; `DW9` = `(height − 1) << 13 | (width − 1)`; `DW11` bits 6–9 = `mips − 1`; `DW12` bits
/// 12+ = tiled base-mip bytes / 4096 (0 for a single mip); `DW13` byte 2 = mip count. Everything
/// else — the constants, the colour-key words, **`DW8` (the slot reference, `2 × index`, not a
/// byte address)**, `DW10`'s per-shape filter word — is kept from `orig`. The format byte of
/// `DW8` follows `format`.
pub fn patch_descriptor_for_resize(
    orig: &[u8; DESCRIPTOR_LEN],
    width: u32,
    height: u32,
    format: PixelFormat,
    mip_count: u32,
) -> Result<[u8; DESCRIPTOR_LEN], DescriptorRuleRefusal> {
    let rd = |i: usize| u32::from_be_bytes(orig[i * 4..i * 4 + 4].try_into().unwrap());
    let dw0 = rd(0);
    if !DESC_MEASURED_DW0.contains(&dw0) {
        return Err(DescriptorRuleRefusal::UnmeasuredGeneration { dw0 });
    }
    if orig[DESC_FORMAT_BYTE_REL] == 0 {
        return Err(DescriptorRuleRefusal::NullFormatDescriptor);
    }
    let dw7 = rd(7);
    if dw7 & DESC_FETCH_TILED_BIT == 0 {
        return Err(DescriptorRuleRefusal::NotTiled);
    }
    // Xenos fetch-constant `dimension` (DW10 bits 9–10 of the public layout sits in the fourth
    // fetch word; the census keys cubes by the 6× slot): refuse when the original chain is 6 × the
    // 2D chain is the caller's check; here the `DW12` rule is applied for a 2D raster only.
    // Xenos fetch-constant `dimension` (bits 9–10 of the fourth word, `DW12` here): 3 = cube.
    if (rd(12) >> 9) & 3 == 3 {
        return Err(DescriptorRuleRefusal::CubeMap);
    }
    // ---- Validation first; no dimension-dependent arithmetic above this line.
    //
    // Representability (coordinator review R1 F1, 21 Sep 2026; **order** corrected under the R2
    // followup, 22 Sep 2026): `DW9` stores `width − 1` / `height − 1` in 13 bits, `DW11` the mip
    // count − 1 in 4 bits, and a chain cannot have more levels than the size halves to. A request
    // outside a field is refused, never masked into another value and never measured first — the
    // tile-aligned size of an out-of-range request overflows (`u32::MAX` DXT5 panicked inside
    // `tiled_buffer_size` before this bound was reached). (`DW7`'s 9-bit pitch and `DW12`'s
    // 20-bit mip page follow from the dimension bound; both are asserted below.)
    let unrep = |what: &'static str, value: u32, max: u32| DescriptorRuleRefusal::Unrepresentable {
        what,
        value,
        max,
    };
    if width > DESC_MAX_DIM {
        return Err(unrep("width", width, DESC_MAX_DIM));
    }
    if height > DESC_MAX_DIM {
        return Err(unrep("height", height, DESC_MAX_DIM));
    }
    if mip_count == 0 || mip_count > DESC_MAX_MIPS {
        return Err(unrep("mip count", mip_count, DESC_MAX_MIPS));
    }
    let (min_dim, tile_texels) = if format.is_dxt() { (128u32, 128u32) } else { (32u32, 32u32) };
    if width < min_dim || height < min_dim {
        return Err(DescriptorRuleRefusal::TooSmall { width, height, minimum: min_dim });
    }
    let halvings = 32 - width.max(height).leading_zeros();
    if mip_count > halvings {
        return Err(unrep("mip count (for this size)", mip_count, halvings));
    }

    // ---- Dimension-dependent arithmetic, on dimensions now known to be 128 … 8 192 (32 … 8 192
    // uncompressed). The checked helpers keep that guarantee auditable rather than implicit: a
    // `None` here would mean the bound above stopped matching the arithmetic, so it refuses
    // rather than panicking.
    let tiled_base = if format.is_dxt() {
        try_tiled_buffer_size(width, height, format.bytes_per_block())
    } else {
        try_tiled_buffer_size_texels(width, height, 4)
    }
    .ok_or(unrep("tiled buffer bytes", width, DESC_MAX_DIM))?;
    let mut d = *orig;
    let wr = |d: &mut [u8; DESCRIPTOR_LEN], i: usize, v: u32| d[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
    // DW7: pitch (bits 22–30, 32-texel units) of the **tile-aligned** width — the shipped
    // 160-wide rasters carry pitch 8 (256 texels), so the pitch is the padded row, not
    // `ceil(width / 32)` (the census's "special widths"); tiled bit and the rest kept.
    let pitch = width.div_ceil(tile_texels) * tile_texels / 32;
    if pitch > 0x1FF {
        return Err(unrep("pitch (32-texel units)", pitch, 0x1FF));
    }
    wr(&mut d, 7, (dw7 & !(0x1FF << 22)) | (pitch << 22));
    // DW8: slot reference kept; format byte follows the new format.
    wr(&mut d, 8, (rd(8) & !0xFF) | format.to_byte() as u32);
    // DW9: (height − 1) << 13 | (width − 1), 13 + 13 bits; the upper bits kept.
    wr(&mut d, 9, (rd(9) & !0x03FF_FFFF) | ((height - 1) << 13) | (width - 1));
    // DW11: mip_max_level bits 6–9 = mips − 1.
    wr(&mut d, 11, (rd(11) & !(0xF << 6)) | ((mip_count - 1) << 6));
    // DW12: mip address = tiled base / 4096 in bits 12+ (0 when there is no second level).
    let mip_addr = if mip_count > 1 {
        u32::try_from(tiled_base / 4096).ok().filter(|p| *p < (1 << 20)).ok_or(unrep("mip page (tiled base / 4096)", u32::MAX, (1 << 20) - 1))?
    } else {
        0
    };
    wr(&mut d, 12, (rd(12) & 0xFFF) | (mip_addr << 12));
    // DW13: byte 2 = mip count.
    d[54] = mip_count as u8;
    Ok(d)
}

/// The fetch-constant `DW0` values whose dimension-dependent words are measured. `0x00400003` is
/// NHL 09–Legacy; `0x00000003` is NHL 07 / 08, added 22 Sep 2026 on research 2026-09-21 pm handoff
/// R42 §2 — the pitch, `DW9`, `DW11`, `DW12` and `DW13` rules were re-measured on the full
/// population of both generations and are identical. The **container** layouts still differ: see
/// [`Texlib::replace_with_resize`], which refuses NHL 07's table-at-EOF form, and the NHL 08
/// RefPack wrapper, which the caller removes.
pub const DESC_MEASURED_DW0: [u32; 2] = [0x0040_0003, 0x0000_0003];

/// `DW8`'s upper field — the file-table index of the texture's pixel-data entry (research
/// 2026-09-21 pm handoff R42 §1: 100 % on NHL 08–Legacy, 10 666 / 10 675 on NHL 07). The low byte
/// is the Xenos format code, so the index starts at bit 12.
pub fn descriptor_data_entry_index(desc: &[u8; DESCRIPTOR_LEN]) -> u32 {
    u32::from_be_bytes(desc[32..36].try_into().unwrap()) >> DESC_DW8_INDEX_SHIFT
}

/// Bit position of `DW8`'s pixel-entry index (the "base address" field of the fetch constant).
const DESC_DW8_INDEX_SHIFT: u32 = 12;

/// The largest width / height a `DW9` field stores (13 bits of `dimension − 1`).
pub const DESC_MAX_DIM: u32 = 0x2000;
/// The most mip levels `DW11` declares (4 bits of `mips − 1`).
pub const DESC_MAX_MIPS: u32 = 16;

/// Build a minimal, single-level **synthetic** `RW4xb2` texlib of tiled DXT rasters — for tests
/// and fixtures only, never a from-scratch encoder for the game (real containers carry name
/// tables, colour-key words and per-shape filter words this does not synthesise). Each raster is
/// `(width, height, format, linear base mip)`; the header region is `0x200` bytes with the file
/// table at `0x60` (a data / descriptor entry pair per raster) and the shipped-shape NHL 09–Legacy
/// descriptors (`DW0 = 0x00400003`, tiled, one level) at `0x100 + 0x40 × i`; the tiled slots
/// follow in order. Round-trips through [`parse`], and the descriptors pass
/// [`patch_descriptor_for_resize`]'s gate, so the resize route can be exercised without a
/// private fixture. Panics on a non-DXT format, a zero dimension or more than four rasters.
pub fn encode_synthetic_texlib(rasters: &[(u32, u32, PixelFormat, &[u8])]) -> Vec<u8> {
    assert!(!rasters.is_empty() && rasters.len() <= 4, "1–4 rasters");
    let hdr = 0x200usize;
    let mut slots: Vec<Vec<u8>> = Vec::new();
    for (w, h, f, linear) in rasters {
        assert!(f.is_dxt() && *w > 0 && *h > 0, "DXT rasters with non-zero dimensions");
        assert_eq!(linear.len(), base_mip_byte_size(*f, *w, *h), "linear base mip size");
        let mut tiled = tile_dxt(linear, *w, *h, f.bytes_per_block());
        swap16_in_place(&mut tiled);
        let padded = (tiled.len() + 0xFFF) & !0xFFF;
        tiled.resize(padded, 0);
        slots.push(tiled);
    }
    let data_size: usize = slots.iter().map(Vec::len).sum();
    let mut b = vec![0u8; hdr + data_size];
    b[..MAGIC.len()].copy_from_slice(MAGIC);
    let wr = |b: &mut [u8], off: usize, v: u32| b[off..off + 4].copy_from_slice(&v.to_be_bytes());
    wr(&mut b, HDR_FILE_COUNT_OFF, (rasters.len() * 2) as u32);
    wr(&mut b, HDR_FILE_TABLE_OFF, 0x60);
    wr(&mut b, HDR_HDR_SIZE_OFF, hdr as u32);
    wr(&mut b, HDR_DATA_SIZE_OFF, data_size as u32);
    let mut rel = 0usize;
    for (i, ((w, h, f, _), slot)) in rasters.iter().zip(&slots).enumerate() {
        let desc_off = 0x100 + 0x40 * i;
        let entries: [([u32; ENTRY_INTS], [u8; 4]); 2] = [
            ([rel as u32, 0, slot.len() as u32, 0x1000, 1], TAG_TEX_DATA),
            ([desc_off as u32, 0, DESCRIPTOR_LEN as u32, 4, 0xA], TAG_TEX_DESC),
        ];
        for (k, (ints, tag)) in entries.iter().enumerate() {
            let e = 0x60 + (i * 2 + k) * ENTRY_STRIDE;
            for (j, v) in ints.iter().enumerate() {
                wr(&mut b, e + j * 4, *v);
            }
            b[e + 20..e + 24].copy_from_slice(tag);
        }
        let mut d = [0u8; DESCRIPTOR_LEN];
        let dw = |d: &mut [u8; DESCRIPTOR_LEN], i: usize, v: u32| d[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
        dw(&mut d, 0, 0x0040_0003);
        dw(&mut d, 7, DESC_FETCH_TILED_BIT | ((w.div_ceil(128) * 128 / 32) << 22));
        dw(&mut d, 8, (2 * i as u32) << 8 | f.to_byte() as u32);
        dw(&mut d, 9, ((h - 1) << 13) | (w - 1));
        d[54] = 1;
        b[desc_off..desc_off + DESCRIPTOR_LEN].copy_from_slice(&d);
        b[hdr + rel..hdr + rel + slot.len()].copy_from_slice(slot);
        rel += slot.len();
    }
    b
}

/// The mip count a shipped descriptor declares (`DW13` byte 2; research handoff 11 §2:
/// single-mip rasters carry 1).
pub fn descriptor_mip_count(desc: &[u8; DESCRIPTOR_LEN]) -> u32 {
    desc[54] as u32
}

fn read_u32_be(data: &[u8], off: usize) -> u32 {
    let bytes: [u8; 4] = data[off..off + 4]
        .try_into()
        .expect("caller already bounds-checked");
    u32::from_be_bytes(bytes)
}


pub(crate) fn scan_names(data: &[u8], start: usize) -> Vec<String> {
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
    let bw = width.div_ceil(4).max(1);
    let bh = height.div_ceil(4).max(1);
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
    let bw = width.div_ceil(4).max(1);
    let bh = height.div_ceil(4).max(1);
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
    try_tiled_buffer_size(width, height, bytes_per_block)
        .unwrap_or_else(|| panic!("tiled_buffer_size({width}, {height}, {bytes_per_block}) is not representable; callers with unvalidated dimensions must use try_tiled_buffer_size"))
}

/// The checked form of [`tiled_buffer_size`]: `None` when the tile-aligned block grid or its
/// product with the block size does not fit a `usize`. Callers whose dimensions are not already
/// bounded by the descriptor rule use this (coordinator review R2, 22 Sep 2026: the infallible
/// form overflowed on `u32::MAX` before `patch_descriptor_for_resize` reached its bound).
pub fn try_tiled_buffer_size(width: u32, height: u32, bytes_per_block: usize) -> Option<usize> {
    let bw = width.div_ceil(4).max(1) as usize;
    let bh = height.div_ceil(4).max(1) as usize;
    let aligned_w = bw.checked_add(31)? & !31;
    let aligned_h = bh.checked_add(31)? & !31;
    aligned_w
        .checked_mul(aligned_h)?
        .checked_mul(bytes_per_block)
}

/// Bytes a tiled buffer must reserve for an *uncompressed* texture
/// whose grid is `width × height` elements of `element_pitch` bytes
/// (e.g. `8_8_8_8` = 4-byte texels). Each dimension is rounded up to a
/// 32-element macro-tile boundary — the texel-grid analogue of
/// [`tiled_buffer_size`]'s block-grid alignment.
pub fn tiled_buffer_size_texels(width: u32, height: u32, element_pitch: usize) -> usize {
    try_tiled_buffer_size_texels(width, height, element_pitch)
        .unwrap_or_else(|| panic!("tiled_buffer_size_texels({width}, {height}, {element_pitch}) is not representable; callers with unvalidated dimensions must use try_tiled_buffer_size_texels"))
}

/// The checked form of [`tiled_buffer_size_texels`] — see [`try_tiled_buffer_size`].
pub fn try_tiled_buffer_size_texels(
    width: u32,
    height: u32,
    element_pitch: usize,
) -> Option<usize> {
    let w = (width.max(1) as usize).checked_add(31)? & !31;
    let h = (height.max(1) as usize).checked_add(31)? & !31;
    w.checked_mul(h)?.checked_mul(element_pitch)
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
    use crate::fixtures::{require, LEGACY_CENTERLOGO, LEGACY_TEXLIB_67};

    #[test]
    fn nhl07_mixed_container_uses_descriptor_pixel_reference() {
        let pixels = vec![0x31; base_mip_byte_size(PixelFormat::Dxt1, 128, 128)];
        let mut bytes = encode_synthetic_texlib(&[(128, 128, PixelFormat::Dxt1, &pixels)]);
        let pixel_start = read_u32_be(&bytes, 0x60) + 0x200;
        let geometry_start = bytes.len();
        bytes.extend_from_slice(&[0xa5; 64]);
        // Absolute, untagged NHL07 buffers, including a non-texture payload.
        bytes[0x60..0x64].copy_from_slice(&pixel_start.to_be_bytes());
        bytes[0x74..0x78].fill(0);
        bytes[0x90..0x94].copy_from_slice(&(geometry_start as u32).to_be_bytes());
        bytes[0x98..0x9c].copy_from_slice(&64u32.to_be_bytes());
        bytes[HDR_FILE_COUNT_OFF..HDR_FILE_COUNT_OFF + 4].copy_from_slice(&3u32.to_be_bytes());
        bytes[HDR_DATA_SIZE_OFF..HDR_DATA_SIZE_OFF + 4].fill(0);
        let parsed = parse(&bytes).unwrap();
        assert_eq!(parsed.textures.len(), 1);
        assert_eq!(parsed.textures[0].base_mip_linear, pixels);
        let edited = parsed.replace_same_size(0, &vec![0x42; pixels.len()]).unwrap();
        assert_eq!(&edited[geometry_start..], &bytes[geometry_start..]);
        assert_eq!(parse(&edited).unwrap().textures[0].base_mip_linear, vec![0x42; pixels.len()]);
        // An explicit dangling reference must fail, never fall back to ordinal pairing.
        let descriptor = read_u32_be(&bytes, 0x78) as usize;
        let word = read_u32_be(&bytes, descriptor + 32);
        bytes[descriptor + 32..descriptor + 36].copy_from_slice(&((word & 0xfff) | (9 << 12)).to_be_bytes());
        assert!(matches!(parse(&bytes), Err(TexlibError::OrphanDescriptor { .. })));
    }

    #[test]
    fn nhl07_award_argb8888_is_xbox_tiled() {
        // Real NHL 07 install (skips when absent, like the other real-file tests here). The award
        // art is an uncompressed `Argb8888` texlib stored **Xbox-tiled** at 32-aligned dims — the
        // case the descriptor's tiled flag (`DESC_FETCH_TILED_BIT`) disambiguates from the LINEAR
        // jersey nameplates, since the two buffers are the same byte length. Regression for the
        // horizontal-banding corruption when a tiled buffer was mistaken for a linear image.
        let path =
            r"X:\Games\Emulation\Roms\XBOX360\NHL 07\NHL 07 (USA)\fe\ion\artassets\awards\a0.big";
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("skipping: {path} not present");
            return;
        };
        // The award `.big` is a BIGF archive; its RW4 texlib entry starts at the RW4 magic. Scan for
        // it rather than pulling the big-archive crate into this unit test.
        let pos = bytes.windows(MAGIC.len()).position(|w| w == MAGIC).unwrap();
        let lib = parse(&bytes[pos..]).expect("award texlib must parse");
        let t = &lib.textures[0];
        assert_eq!((t.width, t.height), (512, 256));
        assert_eq!(t.format, PixelFormat::Argb8888);
        // Correctly untiled, the trophy body at texel (256, 128) is this metal colour `[A, R, G, B]`.
        // A still-tiled (banded) decode reads a different texel here.
        let i = (128 * t.width as usize + 256) * 4;
        assert_eq!(
            &t.base_mip_linear[i..i + 4],
            &[187, 111, 105, 96],
            "award Argb8888 must be untiled"
        );
    }

    #[test]
    fn nhl09_nameplate_argb8888_is_linear() {
        // The counterpart to the award: a jersey nameplate whose `Argb8888` is stored **linear**
        // (tiled flag clear) — untiling it would band it into noise. NHL 09's descriptor clears the
        // format-word bit an earlier heuristic keyed on, so this pins the fix to the fetch-constant
        // `tiled` bit instead. The correct image is yellow number/patch regions on a white field:
        // the top-centre patch is opaque yellow, the empty field around it is opaque white.
        let path = r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)\rendering\jersey\name_1_130_1_cm.rx2";
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("skipping: {path} not present");
            return;
        };
        let pos = bytes.windows(MAGIC.len()).position(|w| w == MAGIC).unwrap();
        let lib = parse(&bytes[pos..]).expect("nameplate texlib must parse");
        let t = &lib.textures[0];
        assert_eq!((t.width, t.height), (256, 256));
        assert_eq!(t.format, PixelFormat::Argb8888);
        let texel = |x: usize, y: usize| {
            let i = (y * t.width as usize + x) * 4;
            &t.base_mip_linear[i..i + 4] // [A, R, G, B]
        };
        // Top-centre patch: opaque yellow (R,G high, B low). A tiled misread scrambles this.
        let patch = texel(128, 40);
        assert_eq!(patch[0], 255, "patch alpha");
        assert!(
            patch[1] > 200 && patch[2] > 200 && patch[3] < 80,
            "patch not yellow: {patch:?}"
        );
        // Bottom field is the empty white background.
        let field = texel(128, 230);
        assert_eq!(field, &[255, 255, 255, 255], "field not white");
    }

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

    /// Requires the real NHL Legacy `rendering/player/texlib_67.rx2`,
    /// supplied through the fixture root rather than an operator-specific
    /// absolute path.
    #[test]
    #[ignore = "private fixture legacy/texlib_67.rx2: set NHL_STUDIO_FIXTURES and select with --ignored (docs/validation/c0-fx1-explicit-fixture-tests.md)"]
    fn parses_real_texlib_67() {
        let data = load_template();
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

    /// The NHL Legacy `texlib_67.rx2` — hair 256×256 DXT3 plus head
    /// 512×512 DXT1. Private game data, supplied through the fixture root;
    /// every case that needs it is `#[ignore]`d — see [`crate::fixtures`].
    fn load_template() -> Vec<u8> {
        require(LEGACY_TEXLIB_67)
    }

    /// Real 1024 × 1024 DXT3 reference (`rendering/icesurface/
    /// centerlogo_0_cm.rx2`). This is the ground truth that confirmed the
    /// DW8 base-address and DW12 mip-start-page fixes (see
    /// `make_descriptor_matches_real_centerlogo_1024`). Private game data,
    /// supplied through the fixture root.
    fn load_centerlogo() -> Vec<u8> {
        require(LEGACY_CENTERLOGO)
    }

    /// **Cross-validation #1.** `make_descriptor(256, 256, DXT3, …)`
    /// must reproduce hair's actual descriptor bytes from
    /// `texlib_67.rx2` exactly. If this drifts, the 2-point pattern
    /// has changed — likely because we added a third reference whose
    /// bits override one of the "unverified" formulas.
    #[test]
    #[ignore = "private fixture legacy/texlib_67.rx2: set NHL_STUDIO_FIXTURES and select with --ignored (docs/validation/c0-fx1-explicit-fixture-tests.md)"]
    fn make_descriptor_matches_hair_256_dxt3() {
        let data = load_template();
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
    #[ignore = "private fixture legacy/texlib_67.rx2: set NHL_STUDIO_FIXTURES and select with --ignored (docs/validation/c0-fx1-explicit-fixture-tests.md)"]
    fn make_descriptor_matches_head_512_dxt1() {
        let data = load_template();
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
    #[ignore = "private fixture legacy/centerlogo_0_cm.rx2: set NHL_STUDIO_FIXTURES and select with --ignored (docs/validation/c0-fx1-explicit-fixture-tests.md)"]
    fn make_descriptor_matches_real_centerlogo_1024() {
        let data = load_centerlogo();
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
    #[ignore = "private fixture legacy/texlib_67.rx2: set NHL_STUDIO_FIXTURES and select with --ignored (docs/validation/c0-fx1-explicit-fixture-tests.md)"]
    fn replace_with_resize_same_dims_round_trip() {
        let data = load_template();
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
    #[ignore = "private fixture legacy/texlib_67.rx2: set NHL_STUDIO_FIXTURES and select with --ignored (docs/validation/c0-fx1-explicit-fixture-tests.md)"]
    fn replace_with_resize_grows_earlier_slot() {
        let data = load_template();
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
    #[ignore = "private fixture legacy/texlib_67.rx2: set NHL_STUDIO_FIXTURES and select with --ignored (docs/validation/c0-fx1-explicit-fixture-tests.md)"]
    fn replace_same_size_round_trip() {
        let data = load_template();
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

    // ------------------------------------------------------------- R1 F1: representability

    /// A shipped-shape NHL 09–Legacy descriptor (`DW0 = 0x00400003`, tiled, `w × h`, `format`,
    /// `mips`) — the reviewer's 56-byte probe with the words the rule reads populated.
    fn synthetic_descriptor(w: u32, h: u32, format: PixelFormat, mips: u32) -> [u8; DESCRIPTOR_LEN] {
        let mut d = [0u8; DESCRIPTOR_LEN];
        let wr = |d: &mut [u8; DESCRIPTOR_LEN], i: usize, v: u32| d[i * 4..i * 4 + 4].copy_from_slice(&v.to_be_bytes());
        wr(&mut d, 0, 0x0040_0003);
        wr(&mut d, 7, DESC_FETCH_TILED_BIT | ((w.div_ceil(128) * 128 / 32) << 22));
        wr(&mut d, 8, format.to_byte() as u32);
        wr(&mut d, 9, ((h - 1) << 13) | (w - 1));
        wr(&mut d, 11, (mips - 1) << 6);
        d[54] = mips as u8;
        d
    }

    /// Mip placement audit (22 Sep 2026): the keep-tail writer rewrites the base and every level
    /// above 16 blocks at the sequential offsets, leaves every packed-tail level's bytes exactly as
    /// they were, and reports what it kept.
    #[test]
    fn keep_tail_writer_rewrites_only_the_content_verified_levels() {
        // A 512 × 512 DXT1 raster grown to a 10-level chain (512 → 1) through the resize route, so
        // the slot holds a full chain with distinctive bytes per level.
        let bytes = synthetic_texlib(512, 512);
        let lib = parse(&bytes).unwrap();
        let chain: Vec<(u32, u32, Vec<u8>)> = (0..10u32)
            .map(|l| {
                let (w, h) = ((512u32 >> l).max(1), (512u32 >> l).max(1));
                (w, h, vec![0x10 + l as u8; base_mip_byte_size(PixelFormat::Dxt1, w, h)])
            })
            .collect();
        let mips: Vec<LinearMip<'_>> = chain.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
        let grown = lib.replace_with_resize(0, PixelFormat::Dxt1, &mips).unwrap();
        let lib = parse(&grown).unwrap();
        let t = &lib.textures[0];
        assert_eq!(descriptor_mip_count(&grown[t.descriptor_offset()..t.descriptor_offset() + DESCRIPTOR_LEN].try_into().unwrap()), 10);
        let shipped_slot = lib.slot_buffer_tiled(0).unwrap().to_vec();

        // Levels 512 … 128 are large (> 16 blocks); 64 and below are the packed tail.
        assert!(!is_packed_tail_level(128, 128) && is_packed_tail_level(64, 64) && is_packed_tail_level(64, 512));
        let large_bytes: usize = (0..3).map(|l| tiled_buffer_size(512 >> l, 512 >> l, 8)).sum();

        let new_chain: Vec<(u32, u32, Vec<u8>)> = chain.iter().map(|(w, h, d)| (*w, *h, vec![0xA0 | (d[0] & 0xF); d.len()])).collect();
        let new_mips: Vec<LinearMip<'_>> = new_chain.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
        assert_eq!(lib.large_level_support(0).unwrap(), LargeLevelSupport::LargeLevels { large_levels: 2 });
        let w = lib.replace_large_levels_keep_tail(0, &new_mips).unwrap();
        assert_eq!((w.levels_written, w.tail_levels_kept), (3, 7));
        assert_eq!(w.tail_bytes_kept, shipped_slot.len() - large_bytes);
        assert_eq!(w.support, LargeLevelSupport::LargeLevels { large_levels: 2 });
        // The byte-write mask: exactly the three large levels, contiguous from the slot start, and
        // every byte outside the mask identical to the input (the independent preservation check).
        let slot_start = lib.hdr_size as usize + t.data_offset();
        assert_eq!(w.written_ranges.len(), 3);
        assert_eq!(w.written_ranges[0].0, slot_start);
        assert_eq!(w.written_ranges[2].1, slot_start + large_bytes);
        assert_bytes_preserved_outside(&grown, &w.bytes, &w.written_ranges);
        let after = parse(&w.bytes).unwrap();
        let slot = after.slot_buffer_tiled(0).unwrap();
        // The three large levels carry the new bytes at the model's offsets …
        let mut off = 0usize;
        for l in 0..3u32 {
            let size = tiled_buffer_size(512 >> l, 512 >> l, 8);
            let mut expect = tile_dxt(&new_chain[l as usize].2, 512 >> l, 512 >> l, 8);
            swap16_in_place(&mut expect);
            assert_eq!(&slot[off..off + size], &expect[..], "level {l}");
            off += size;
        }
        // … and every byte after them is exactly what shipped (the tail is not zeroed or rewritten).
        assert_eq!(&slot[large_bytes..], &shipped_slot[large_bytes..]);
        assert_eq!(after.textures[0].base_mip_linear, new_chain[0].2, "the base reads back");
        // Everything outside the slot is untouched.
        let (a, b) = (&w.bytes, &grown);
        assert_eq!(a.len(), b.len());
        let start = lib.hdr_size as usize + t.data_offset();
        assert_eq!(&a[..start], &b[..start]);
        assert_eq!(&a[start + shipped_slot.len()..], &b[start + shipped_slot.len()..]);

        // A tail-sized raster writes its base only and keeps everything else.
        let small = encode_synthetic_texlib(&[(64, 64, PixelFormat::Dxt1, &vec![7u8; base_mip_byte_size(PixelFormat::Dxt1, 64, 64)])]);
        let lib = parse(&small).unwrap();
        let base = vec![9u8; base_mip_byte_size(PixelFormat::Dxt1, 64, 64)];
        let w = lib.replace_large_levels_keep_tail(0, &[LinearMip { width: 64, height: 64, linear_dxt: &base }]).unwrap();
        assert_eq!((w.levels_written, w.tail_levels_kept), (1, 0));
        assert!(matches!(w.support, LargeLevelSupport::BaseOnly { ref reason } if reason.contains("single level")), "{:?}", w.support);
        assert_eq!(w.written_ranges.len(), 1);
        assert_bytes_preserved_outside(&small, &w.bytes, &w.written_ranges);
        assert_eq!(parse(&w.bytes).unwrap().textures[0].base_mip_linear, base);
    }

    /// Every byte of `after` outside `ranges` equals `before` (same length): the preservation
    /// check behind the byte-write mask.
    fn assert_bytes_preserved_outside(before: &[u8], after: &[u8], ranges: &[(usize, usize)]) {
        assert_eq!(before.len(), after.len(), "a masked write never changes the length");
        let mut cursor = 0usize;
        for &(start, end) in ranges {
            assert!(start >= cursor && end >= start && end <= after.len(), "ranges are ordered and in bounds: {ranges:?}");
            assert_eq!(&before[cursor..start], &after[cursor..start], "bytes before range {start:#x}..{end:#x} changed");
            cursor = end;
        }
        assert_eq!(&before[cursor..], &after[cursor..], "bytes after the last range changed");
    }

    /// Coordinator review of receipts 10–17 (F2, 22 Sep 2026): the large-level rewrite must not
    /// inherit coverage the audit did not measure. A mipped raster outside the audited population
    /// — here a 128 × 128 chain (below the 256-texel floor) and a 512 × 512 chain whose descriptor
    /// carries the NHL 07 / 08 form (`DW0 = 0x00000003`) — takes the **base-only** route: one
    /// written range, the gate's reason naming the excluded property, every other byte kept.
    #[test]
    fn keep_tail_writer_falls_back_to_base_only_outside_the_audited_population() {
        // (a) 128 × 128, 8 levels: shape outside the audited floor.
        let bytes = synthetic_texlib(128, 128);
        let lib = parse(&bytes).unwrap();
        let chain: Vec<(u32, u32, Vec<u8>)> = (0..8u32)
            .map(|l| {
                let (w, h) = ((128u32 >> l).max(1), (128u32 >> l).max(1));
                (w, h, vec![0x10 + l as u8; base_mip_byte_size(PixelFormat::Dxt1, w, h)])
            })
            .collect();
        let mips: Vec<LinearMip<'_>> = chain.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
        let grown = lib.replace_with_resize(0, PixelFormat::Dxt1, &mips).unwrap();
        let lib = parse(&grown).unwrap();
        assert_eq!(lib.declared_mip_count(0), 8);
        match lib.large_level_support(0).unwrap() {
            LargeLevelSupport::BaseOnly { reason } => assert!(reason.contains("128×128") && reason.contains("outside the audited shapes"), "{reason}"),
            other => panic!("{other:?}"),
        }
        let new_chain: Vec<(u32, u32, Vec<u8>)> = chain.iter().map(|(w, h, d)| (*w, *h, vec![0xA0 | (d[0] & 0xF); d.len()])).collect();
        let new_mips: Vec<LinearMip<'_>> = new_chain.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
        let w = lib.replace_large_levels_keep_tail(0, &new_mips).unwrap();
        assert_eq!((w.levels_written, w.tail_levels_kept), (1, 7));
        assert_eq!(w.written_ranges.len(), 1);
        assert_eq!(w.written_ranges[0].1 - w.written_ranges[0].0, tiled_buffer_size(128, 128, 8));
        assert_bytes_preserved_outside(&grown, &w.bytes, &w.written_ranges);
        assert_eq!(parse(&w.bytes).unwrap().textures[0].base_mip_linear, new_chain[0].2);

        // (b) 512 × 512, 10 levels, but the NHL 07 / 08 descriptor form.
        let bytes = synthetic_texlib(512, 512);
        let lib = parse(&bytes).unwrap();
        let chain: Vec<(u32, u32, Vec<u8>)> = (0..10u32)
            .map(|l| {
                let (w, h) = ((512u32 >> l).max(1), (512u32 >> l).max(1));
                (w, h, vec![0x10 + l as u8; base_mip_byte_size(PixelFormat::Dxt1, w, h)])
            })
            .collect();
        let mips: Vec<LinearMip<'_>> = chain.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
        let mut grown = lib.replace_with_resize(0, PixelFormat::Dxt1, &mips).unwrap();
        let desc_off = parse(&grown).unwrap().textures[0].descriptor_offset();
        grown[desc_off..desc_off + 4].copy_from_slice(&DESC_MEASURED_DW0[1].to_be_bytes());
        let lib = parse(&grown).unwrap();
        match lib.large_level_support(0).unwrap() {
            LargeLevelSupport::BaseOnly { reason } => assert!(reason.contains("0x00000003") && reason.contains("descriptor form"), "{reason}"),
            other => panic!("{other:?}"),
        }
        let new_chain: Vec<(u32, u32, Vec<u8>)> = chain.iter().map(|(w, h, d)| (*w, *h, vec![0xA0 | (d[0] & 0xF); d.len()])).collect();
        let new_mips: Vec<LinearMip<'_>> = new_chain.iter().map(|(w, h, d)| LinearMip { width: *w, height: *h, linear_dxt: d }).collect();
        let w = lib.replace_large_levels_keep_tail(0, &new_mips).unwrap();
        assert_eq!((w.levels_written, w.tail_levels_kept), (1, 9));
        assert_eq!(w.written_ranges.len(), 1);
        assert_bytes_preserved_outside(&grown, &w.bytes, &w.written_ranges);
    }

    /// Two DXT1 rasters (`w × h` and 128 × 128) through the public synthetic builder.
    fn synthetic_texlib(w: u32, h: u32) -> Vec<u8> {
        let a = vec![0u8; base_mip_byte_size(PixelFormat::Dxt1, w, h)];
        let b: Vec<u8> = (0..base_mip_byte_size(PixelFormat::Dxt1, 128, 128)).map(|i| 0x30 | (i as u8 & 0xF)).collect();
        encode_synthetic_texlib(&[(w, h, PixelFormat::Dxt1, &a), (128, 128, PixelFormat::Dxt1, &b)])
    }

    /// `DW9` holds `width − 1` / `height − 1` in 13 bits each (max 8 192), `DW11` bits 6–9 the mip
    /// count − 1 (max 16). Coordinator review R1 (F1, 21 Sep 2026): a request of 8 196 × 128 was
    /// masked to width 4 and reported as success. Now: 8 192 on either axis is re-derived and
    /// reads back exactly; 8 196 / 8 196-high, a zero dimension, zero mips, 17 mips and more mips
    /// than the size halves to are refused with the field named — never a masked value.
    #[test]
    fn patch_descriptor_refuses_unrepresentable_dimensions_instead_of_masking_them() {
        let orig = synthetic_descriptor(128, 128, PixelFormat::Dxt1, 1);
        // The reviewer's reproduction: 8 196 × 128 must not become 4 × 128.
        let err = patch_descriptor_for_resize(&orig, 8196, 128, PixelFormat::Dxt1, 1).expect_err("8 196 wide is not representable in 13 bits");
        let text = err.to_string();
        assert!(text.contains("width") && text.contains("8196") && text.contains("8192"), "{text}");
        for (w, h) in [(8192u32, 128u32), (128, 8192)] {
            let d = patch_descriptor_for_resize(&orig, w, h, PixelFormat::Dxt1, 1).unwrap_or_else(|e| panic!("{w}x{h}: {e}"));
            let dw9 = u32::from_be_bytes(d[36..40].try_into().unwrap());
            assert_eq!(((dw9 & 0x1FFF) + 1, ((dw9 >> 13) & 0x1FFF) + 1), (w, h), "{w}x{h} reads back");
        }
        for (w, h, m) in [(8196u32, 128u32, 1u32), (128, 8196, 1), (0, 128, 1), (128, 0, 1), (128, 128, 0), (128, 128, 17), (128, 128, 9)] {
            let err = patch_descriptor_for_resize(&orig, w, h, PixelFormat::Dxt1, m).expect_err("must be refused");
            // A zero dimension is caught by the existing minimum-size gate; everything else by the field bound.
            assert!(matches!(err, DescriptorRuleRefusal::Unrepresentable { .. }) || (w.min(h) == 0 && matches!(err, DescriptorRuleRefusal::TooSmall { .. })), "{w}x{h}/{m}: {err}");
        }
        // 128 × 128 affords 8 levels (128 → 1).
        let d = patch_descriptor_for_resize(&orig, 128, 128, PixelFormat::Dxt1, 8).unwrap();
        assert_eq!(descriptor_mip_count(&d), 8);
    }

    /// Coordinator review R1 followup (R2, 22 Sep 2026): the size arithmetic ran **before** the
    /// dimension bound, so `u32::MAX × u32::MAX` DXT5 panicked on a multiplication overflow inside
    /// `tiled_buffer_size` instead of returning the refusal. The exact reproduction, plus
    /// width-only / height-only extremes and the non-DXT (`8_8_8_8`) branch, on both of which the
    /// tile-aligned product also overflows. Each must return a descriptive refusal naming the
    /// axis — no panic, no allocation — while the valid maxima keep working.
    #[test]
    fn patch_descriptor_refuses_extreme_dimensions_without_overflowing() {
        let orig = synthetic_descriptor(128, 128, PixelFormat::Dxt1, 1);
        // The reviewer's reproduction, verbatim.
        let err = patch_descriptor_for_resize(&orig, u32::MAX, u32::MAX, PixelFormat::Dxt5, 1).expect_err("u32::MAX square must refuse");
        assert!(matches!(err, DescriptorRuleRefusal::Unrepresentable { what: "width", .. }), "{err}");
        assert!(err.to_string().contains(&u32::MAX.to_string()) && err.to_string().contains("8192"), "{err}");
        // One axis at a time, on both branches, over every format the writer accepts. The
        // tile-aligned block grid of `u32::MAX` overflows `usize` arithmetic on 64-bit too once it
        // is multiplied by the other axis, so the height-only case matters as much as the width.
        for format in [PixelFormat::Dxt1, PixelFormat::Dxt3, PixelFormat::Dxt5, PixelFormat::Argb8888] {
            for (w, h, axis) in [(u32::MAX, 128u32, "width"), (128, u32::MAX, "height"), (u32::MAX, u32::MAX, "width"), (u32::MAX - 1, u32::MAX - 1, "width"), (0x2001, 0x2001, "width")] {
                let err = patch_descriptor_for_resize(&orig, w, h, format, 1).expect_err("must refuse");
                assert!(matches!(err, DescriptorRuleRefusal::Unrepresentable { what, .. } if what == axis), "{w}x{h} {format:?}: {err}");
            }
            // An extreme mip count alongside an extreme size still refuses on a bound, not a panic.
            let err = patch_descriptor_for_resize(&orig, u32::MAX, u32::MAX, format, u32::MAX).expect_err("must refuse");
            assert!(matches!(err, DescriptorRuleRefusal::Unrepresentable { .. }), "{format:?}: {err}");
        }
        // The valid maxima are unchanged by the reordering: the largest representable square on
        // each branch still yields a descriptor, and its `DW9` reads back exactly.
        for (format, w, h) in [(PixelFormat::Dxt5, DESC_MAX_DIM, DESC_MAX_DIM), (PixelFormat::Dxt1, DESC_MAX_DIM, 128), (PixelFormat::Argb8888, DESC_MAX_DIM, 32)] {
            let d = patch_descriptor_for_resize(&orig, w, h, format, 1).unwrap_or_else(|e| panic!("{w}x{h} {format:?}: {e}"));
            let dw9 = u32::from_be_bytes(d[36..40].try_into().unwrap());
            assert_eq!(((dw9 & 0x1FFF) + 1, ((dw9 >> 13) & 0x1FFF) + 1), (w, h));
        }
        // And the tile-size helpers themselves report an unrepresentable product instead of
        // overflowing, so no other caller can be surprised by one.
        assert_eq!(try_tiled_buffer_size(u32::MAX, u32::MAX, 16), None);
        assert_eq!(try_tiled_buffer_size_texels(u32::MAX, u32::MAX, 4), None);
        assert_eq!(try_tiled_buffer_size(128, 128, 8), Some(tiled_buffer_size(128, 128, 8)));
        assert_eq!(try_tiled_buffer_size_texels(64, 64, 4), Some(tiled_buffer_size_texels(64, 64, 4)));
    }

    /// The same request through the container writer: the refusal happens before any buffer is
    /// built, and a 8 192-wide resize (the boundary) reflows, re-parses at the requested size and
    /// leaves the other raster byte-identical. The source `Texlib` is never mutated (the writer
    /// returns a fresh buffer), so the caller's bytes are intact by construction — asserted anyway.
    #[test]
    fn replace_with_resize_refuses_before_building_and_round_trips_the_boundary() {
        let bytes = synthetic_texlib(256, 256);
        let lib = parse(&bytes).expect("synthetic texlib parses");
        assert_eq!(lib.textures.len(), 2);
        assert_eq!((lib.textures[0].width, lib.textures[0].height), (256, 256));
        let too_wide = vec![0u8; base_mip_byte_size(PixelFormat::Dxt1, 8196, 128)];
        let err = lib.replace_with_resize(0, PixelFormat::Dxt1, &[LinearMip { width: 8196, height: 128, linear_dxt: &too_wide }]).expect_err("8 196 wide refused");
        assert!(matches!(err, TexlibError::ResizeRule(_)), "{err}");
        assert!(err.to_string().contains("8196"), "{err}");
        assert_eq!(lib.raw, bytes, "the source bytes are intact");
        // A chain that does not halve is refused too.
        let l0 = vec![0x5Au8; base_mip_byte_size(PixelFormat::Dxt1, 512, 256)];
        let l1 = vec![0x5Au8; base_mip_byte_size(PixelFormat::Dxt1, 128, 64)];
        let err = lib.replace_with_resize(0, PixelFormat::Dxt1, &[LinearMip { width: 512, height: 256, linear_dxt: &l0 }, LinearMip { width: 128, height: 64, linear_dxt: &l1 }]).expect_err("non-halving chain");
        assert!(matches!(err, TexlibError::ResizeRule(_)), "{err}");
        // The boundary: 8 192 × 128, one level.
        let wide: Vec<u8> = (0..base_mip_byte_size(PixelFormat::Dxt1, 8192, 128)).map(|i| (i % 251) as u8).collect();
        let out = lib.replace_with_resize(0, PixelFormat::Dxt1, &[LinearMip { width: 8192, height: 128, linear_dxt: &wide }]).expect("8 192 wide");
        let re = parse(&out).expect("re-parses");
        assert_eq!((re.textures[0].width, re.textures[0].height), (8192, 128));
        assert_eq!(re.textures[0].base_mip_linear, wide);
        assert_eq!(re.textures[1].base_mip_linear, lib.textures[1].base_mip_linear, "the other raster is untouched");
        assert_eq!(re.textures[0].slot.size as usize, (tiled_buffer_size(8192, 128, 8) + 0xFFF) & !0xFFF);
        assert_eq!(read_u32_be(&out, 0x54) as usize, out.len() - 0x200, "header data size");
    }
}
