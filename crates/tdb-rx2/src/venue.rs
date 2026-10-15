//! Gen7 arena **venue packages** (`rendering/stadium/stadium_<arenaartid>_container_<n>.rx2 / .rpsgl`) read for a
//! read-only board-art gallery (receipt 49; research R127 handoff 20 §6–§7, R127g §8).
//!
//! A venue package is one RW4 container holding the arena's geometry, 10–111 rasters (`0x20003` descriptors) and
//! its named materials (`0xEF0005`). The name dictionary (`0xEC0010`: count word = flag byte + 24-bit count, then
//! 12-byte `(string offset, type, object index)` entries) names materials and geometry groups — not rasters. The
//! material → raster edge is **parsed data** (R127 §6): the `0xEF0001` chunk immediately before each material holds
//! at +256 the file-table index of its slot-0 raster descriptor and at +288 a second one (slot 1) on some shaders.
//! Every index is validated before it is followed: in range, the named chunk's tag, and the chunk's bytes inside the
//! file. Nothing here says the game draws that raster at run time, and nothing is written.
//!
//! Pixels (Xbox 360 only): a descriptor's `DW8` upper field is the file-table index of its pixel entry
//! (`0x10031`; R127g — the texlib parser's "pair entries in order" rule fails here because most data entries are
//! geometry). PS3 (receipt 58; research 2026-09-28 handoff 03 §1, R137): a descriptor's first u32 is the file-table index
//! of its `0x10030` data entry, which holds the **plain** (untiled, standard block order) DXT chain, levels concatenated
//! base-first; signatures `83f1` / `83f2` / `83f3` are DXT1 / DXT3 / DXT5 and the level count is the byte at +52. An
//! entry is used only when its size equals that chain exactly (a cube map's six faces do not, and are not previewed).
use std::collections::BTreeMap;

use thiserror::Error;

use crate::texlib::{self, PixelFormat};

pub const TAG_NAMES: u32 = 0x00EC_0010;
pub const TAG_RASTER: u32 = 0x0002_0003;
pub const TAG_PIXELS_XB2: u32 = 0x0001_0031;
pub const TAG_PIXELS_PS3: u32 = 0x0001_0030;
pub const TAG_MATERIAL: u32 = 0x00EF_0005;
pub const TAG_MATERIAL_PARAMS: u32 = 0x00EF_0001;
/// Slot-word offsets inside the `0xEF0001` chunk (R127 §6).
pub const SLOT_OFFSETS: [usize; 2] = [256, 288];

const ENTRY: usize = 24;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum VenueError {
    #[error("not an RW4 Xbox 360 / PS3 container (magic)")]
    Magic,
    #[error("the file is too short for its header ({0} bytes)")]
    Short(usize),
    #[error("the file table ({count} entries at {off:#x}) runs past the end of the file")]
    Table { count: usize, off: usize },
    #[error("raster {0} cannot be previewed: {1}")]
    Raster(usize, String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VenuePlatform {
    Xbox360,
    Ps3,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub index: usize,
    pub tag: u32,
    /// Raw offset word (absolute for header chunks; relative to `hdr_size` for pixel / data entries).
    pub offset: u32,
    pub size: u32,
}

/// One raster descriptor of the package.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueRaster {
    /// Position among the package's descriptors (file-table order).
    pub ordinal: usize,
    /// File-table index of the descriptor.
    pub chunk: usize,
    pub width: u32,
    pub height: u32,
    /// `DXT1` / `DXT3` / `DXT5` / `ARGB8` or the raw code.
    pub format: String,
    /// Xbox 360: the pixel entry when DW8 names one whose tag and size fit; `None` with `pixels_note` otherwise.
    pub pixel_chunk: Option<usize>,
    pub pixels_note: Option<String>,
}

/// A material's texture slot as stored.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Slot {
    /// The word names a raster descriptor (its ordinal).
    Raster(usize),
    /// Any other value (for example `0x400000` on the emissive power-ring materials); kept raw.
    Other(u32),
    /// The parameter chunk is absent, has another tag or is too short to hold this slot.
    Unreadable(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VenueMaterial {
    pub chunk: usize,
    pub name: String,
    /// The `*.FX` shader file named inside the material, when one is found.
    pub shader: Option<String>,
    pub slots: [Slot; 2],
}

#[derive(Debug, Clone)]
pub struct VenuePackage {
    pub platform: VenuePlatform,
    pub hdr_size: u32,
    /// `hdr_size + data_size == file length` (R127g measured it on three packages); reported, not required.
    pub sizes_consistent: bool,
    pub chunks: Vec<Chunk>,
    pub rasters: Vec<VenueRaster>,
    pub materials: Vec<VenueMaterial>,
    /// Dictionary entries whose object index or string is out of range (skipped).
    pub bad_names: usize,
}

fn be32(b: &[u8], o: usize) -> Option<u32> {
    b.get(o..o + 4).map(|s| u32::from_be_bytes(s.try_into().unwrap()))
}

fn be16(b: &[u8], o: usize) -> Option<u16> {
    b.get(o..o + 2).map(|s| u16::from_be_bytes(s.try_into().unwrap()))
}

fn cstr(b: &[u8], o: usize) -> Option<String> {
    let s = b.get(o..)?;
    let end = s.iter().take(256).position(|c| *c == 0)?;
    Some(String::from_utf8_lossy(&s[..end]).into_owned())
}

/// The chunk's bytes when they lie inside the file (header chunks: absolute offset).
fn chunk_bytes<'a>(b: &'a [u8], c: &Chunk) -> Option<&'a [u8]> {
    b.get(c.offset as usize..(c.offset as usize).checked_add(c.size as usize)?)
}

fn x360_format(code: u8) -> String {
    match PixelFormat::from_byte(code) {
        Some(f) => format!("{f:?}").to_uppercase().replace("ARGB8888", "ARGB8"),
        None => format!("format {code:#04x}"),
    }
}

/// Parse a venue package's table, rasters and named materials. Refuses only a file that is not an RW4 container
/// or whose table does not fit; every other inconsistency is reported on the affected item.
pub fn read(b: &[u8]) -> Result<VenuePackage, VenueError> {
    let platform = if b.starts_with(texlib::MAGIC) {
        VenuePlatform::Xbox360
    } else if b.starts_with(crate::texlib_ps3::MAGIC) {
        VenuePlatform::Ps3
    } else {
        return Err(VenueError::Magic);
    };
    let (count, table, hdr_size, data_size) = match (be32(b, 0x20), be32(b, 0x30), be32(b, 0x44), be32(b, 0x54)) {
        (Some(c), Some(t), Some(h), Some(d)) => (c as usize, t as usize, h, d),
        _ => return Err(VenueError::Short(b.len())),
    };
    if count.checked_mul(ENTRY).and_then(|n| n.checked_add(table)).is_none_or(|end| end > b.len()) {
        return Err(VenueError::Table { count, off: table });
    }
    let chunks: Vec<Chunk> = (0..count)
        .map(|i| {
            let e = table + i * ENTRY;
            Chunk { index: i, tag: be32(b, e + 20).unwrap(), offset: be32(b, e).unwrap(), size: be32(b, e + 8).unwrap() }
        })
        .collect();

    // Names: object index → (name, type). Entries pointing outside the table or the file are counted, not used.
    let mut names: BTreeMap<usize, String> = BTreeMap::new();
    let mut bad_names = 0;
    for c in chunks.iter().filter(|c| c.tag == TAG_NAMES) {
        let Some(d) = chunk_bytes(b, c) else {
            bad_names += 1;
            continue;
        };
        let n = (be32(d, 0).unwrap_or(0) & 0x00FF_FFFF) as usize;
        for k in 0..n {
            let (Some(sp), Some(_ty), Some(oi)) = (be32(d, 8 + 12 * k), be32(d, 12 + 12 * k), be32(d, 16 + 12 * k)) else {
                bad_names += 1;
                break;
            };
            match cstr(b, sp as usize) {
                Some(s) if (oi as usize) < chunks.len() => {
                    names.insert(oi as usize, s);
                }
                _ => bad_names += 1,
            }
        }
    }

    // Rasters.
    let mut ordinal_of: BTreeMap<usize, usize> = BTreeMap::new();
    let mut rasters = Vec::new();
    for c in chunks.iter().filter(|c| c.tag == TAG_RASTER) {
        let ordinal = rasters.len();
        ordinal_of.insert(c.index, ordinal);
        let d = chunk_bytes(b, c);
        let mut r = VenueRaster { ordinal, chunk: c.index, width: 0, height: 0, format: "unreadable".into(), pixel_chunk: None, pixels_note: None };
        match (platform, d) {
            (_, None) => r.pixels_note = Some("the descriptor lies outside the file".into()),
            (VenuePlatform::Xbox360, Some(d)) if d.len() >= texlib::DESCRIPTOR_LEN => {
                let dw9 = be32(d, 36).unwrap();
                r.width = (dw9 & 0x1FFF) + 1;
                r.height = ((dw9 >> 13) & 0x1FFF) + 1;
                r.format = x360_format(d[35]);
                let desc: [u8; texlib::DESCRIPTOR_LEN] = d[..texlib::DESCRIPTOR_LEN].try_into().unwrap();
                let pi = texlib::descriptor_data_entry_index(&desc) as usize;
                match chunks.get(pi) {
                    None => r.pixels_note = Some(format!("DW8 names entry {pi}, past the table's {} entries", chunks.len())),
                    Some(p) if p.tag != TAG_PIXELS_XB2 => r.pixels_note = Some(format!("DW8 names entry {pi}, which is not a pixel entry (tag {:#x})", p.tag)),
                    Some(p) => match PixelFormat::from_byte(d[35]) {
                        None => r.pixels_note = Some(format!("{} is not a format Studio decodes", r.format)),
                        Some(f) => {
                            let need = texlib::base_mip_byte_size(f, r.width, r.height) as u64;
                            let start = hdr_size as u64 + p.offset as u64;
                            if (p.size as u64) < need || start + need > b.len() as u64 {
                                r.pixels_note = Some(format!("pixel entry {pi} holds {} bytes, fewer than the base level's {need}", p.size));
                            } else {
                                r.pixel_chunk = Some(pi);
                            }
                        }
                    },
                }
            }
            (VenuePlatform::Ps3, Some(d)) if d.len() >= 0x38 => {
                r.width = be16(d, 48).unwrap() as u32;
                r.height = be16(d, 50).unwrap() as u32;
                let sig = (be16(d, 34).unwrap(), be16(d, 38).unwrap(), be16(d, 42).unwrap());
                let dxt = match sig {
                    (0x83F1, 0x83F1, 0xFFFF) => Some(PixelFormat::Dxt1),
                    (0x83F2, 0x83F2, 0xFFFF) => Some(PixelFormat::Dxt3),
                    (0x83F3, 0x83F3, 0xFFFF) => Some(PixelFormat::Dxt5),
                    _ => None,
                };
                r.format = match (dxt, sig) {
                    (Some(f), _) => x360_format(f.to_byte()),
                    (None, (0x6007, 0x6007, 0x8035)) => "ARGB8".into(),
                    (None, (a, b2, c2)) => format!("PS3 signature {a:04x}/{b2:04x}/{c2:04x}"),
                };
                match dxt {
                    None => r.pixels_note = Some(format!("{} is not a PS3 venue format research measured (83f1 / 83f2 / 83f3)", r.format)),
                    Some(f) => {
                        let pi = be32(d, 0).unwrap() as usize;
                        let levels = d[52] as u32;
                        let chain = ps3_plain_chain(f, r.width, r.height, levels);
                        match chunks.get(pi) {
                            None => r.pixels_note = Some(format!("the descriptor names entry {pi}, past the table's {} entries", chunks.len())),
                            Some(p) if p.tag != TAG_PIXELS_PS3 => r.pixels_note = Some(format!("the descriptor names entry {pi}, which is not a PS3 data entry (tag {:#x})", p.tag)),
                            Some(p) if p.size as u64 != chain => {
                                r.pixels_note = Some(format!("data entry {pi} holds {} bytes, not the {levels}-level plain chain's {chain} (e.g. a cube map); not previewed", p.size))
                            }
                            Some(p) if hdr_size as u64 + p.offset as u64 + chain > b.len() as u64 => r.pixels_note = Some(format!("data entry {pi} runs past the file")),
                            Some(_) => r.pixel_chunk = Some(pi),
                        }
                    }
                }
            }
            (_, Some(_)) => r.pixels_note = Some("the descriptor is shorter than a descriptor".into()),
        }
        rasters.push(r);
    }

    // Named materials and their slots.
    let mut materials = Vec::new();
    for c in chunks.iter().filter(|c| c.tag == TAG_MATERIAL) {
        let Some(name) = names.get(&c.index).cloned() else { continue };
        let shader = chunk_bytes(b, c).and_then(find_fx);
        let params = c.index.checked_sub(1).and_then(|i| chunks.get(i));
        let slots = SLOT_OFFSETS.map(|off| match params {
            None => Slot::Unreadable("no chunk precedes the material".into()),
            Some(p) if p.tag != TAG_MATERIAL_PARAMS => Slot::Unreadable(format!("the preceding chunk is {:#x}, not the 0xEF0001 parameter block", p.tag)),
            Some(p) => match chunk_bytes(b, p).and_then(|d| be32(d, off)) {
                None => Slot::Unreadable(format!("the parameter block ({} bytes) does not reach +{off}", p.size)),
                Some(w) => match ordinal_of.get(&(w as usize)) {
                    Some(o) => Slot::Raster(*o),
                    None => Slot::Other(w),
                },
            },
        });
        materials.push(VenueMaterial { chunk: c.index, name, shader, slots });
    }

    Ok(VenuePackage {
        platform,
        hdr_size,
        sizes_consistent: hdr_size as u64 + data_size as u64 == b.len() as u64,
        chunks,
        rasters,
        materials,
        bad_names,
    })
}

/// The first `name.FX` string in a material chunk.
fn find_fx(d: &[u8]) -> Option<String> {
    let at = d.windows(3).position(|w| w == b".FX")?;
    let start = d[..at].iter().rposition(|c| !(c.is_ascii_alphanumeric() || *c == b'_')).map_or(0, |p| p + 1);
    // A length byte read as a letter is research's noted noise; the name starts at an upper- or lower-case letter.
    let s = String::from_utf8_lossy(&d[start..at + 3]).into_owned();
    (s.len() > 3).then_some(s)
}

/// Bytes of a plain (PS3) DXT chain of `levels` levels: blocks per level with a one-block floor, base first.
pub fn ps3_plain_chain(f: PixelFormat, w: u32, h: u32, levels: u32) -> u64 {
    let (mut w, mut h, mut s) = (w.max(1), h.max(1), 0u64);
    for _ in 0..levels.max(1) {
        s += w.div_ceil(4).max(1) as u64 * h.div_ceil(4).max(1) as u64 * f.bytes_per_block() as u64;
        w = (w / 2).max(1);
        h = (h / 2).max(1);
    }
    s
}

/// The base level of a raster in linear (standard DXT) order with its format: Xbox 360 untiled and byte-swapped, PS3 as
/// stored (receipt 58).
pub fn raster_linear(b: &[u8], pkg: &VenuePackage, ordinal: usize) -> Result<(PixelFormat, u32, u32, Vec<u8>), VenueError> {
    let r = pkg.rasters.get(ordinal).ok_or_else(|| VenueError::Raster(ordinal, "no such raster".into()))?;
    let pi = r.pixel_chunk.ok_or_else(|| VenueError::Raster(ordinal, r.pixels_note.clone().unwrap_or_default()))?;
    if pkg.platform == VenuePlatform::Ps3 {
        let f = match r.format.as_str() {
            "DXT1" => PixelFormat::Dxt1,
            "DXT3" => PixelFormat::Dxt3,
            "DXT5" => PixelFormat::Dxt5,
            other => return Err(VenueError::Raster(ordinal, format!("{other} is not decoded"))),
        };
        let start = pkg.hdr_size as usize + pkg.chunks[pi].offset as usize;
        let base = texlib::base_mip_byte_size(f, r.width, r.height);
        let span = b.get(start..start + base).ok_or_else(|| VenueError::Raster(ordinal, "the base level runs past the file".into()))?;
        return Ok((f, r.width, r.height, span.to_vec()));
    }
    let d = &b[pkg.chunks[r.chunk].offset as usize..];
    let f = PixelFormat::from_byte(d[35]).ok_or_else(|| VenueError::Raster(ordinal, format!("{} is not decoded", r.format)))?;
    let fetch_dw0 = be32(d, 0).unwrap_or(0);
    let tiled = fetch_dw0 & texlib::DESC_FETCH_TILED_BIT != 0;
    let p = &pkg.chunks[pi];
    let start = pkg.hdr_size as usize + p.offset as usize;
    let avail = &b[start..(start + p.size as usize).min(b.len())];
    let (w, h) = (r.width, r.height);
    let linear = if f.is_dxt() {
        let tiled_size = texlib::tiled_buffer_size(w, h, f.bytes_per_block());
        let base = texlib::base_mip_byte_size(f, w, h);
        let mut span = if avail.len() >= tiled_size { avail[..tiled_size].to_vec() } else { avail[..base].to_vec() };
        texlib::swap16_in_place(&mut span);
        texlib::untile_dxt(&span, w, h, f.bytes_per_block())
    } else if tiled {
        let tiled_size = texlib::tiled_buffer_size_texels(w, h, 4);
        if avail.len() < tiled_size {
            return Err(VenueError::Raster(ordinal, "the tiled ARGB8 buffer is short".into()));
        }
        texlib::untile(&avail[..tiled_size], w, h, 4)
    } else {
        avail[..texlib::base_mip_byte_size(f, w, h)].to_vec()
    };
    Ok((f, w, h, linear))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn put(b: &mut [u8], o: usize, v: u32) {
        b[o..o + 4].copy_from_slice(&v.to_be_bytes());
    }

    /// A hand-built RW4xb2 venue package: pixel entry 0, raster descriptor 1 (64 × 64 DXT1, DW8 → 0), a second
    /// descriptor 2 whose DW8 names the parameter block, parameter block 3 (+256 → descriptor 1, +288 → 7),
    /// material 4 (`env_boards.FX`), a material 5 whose preceding chunk is not a parameter block, names 6.
    fn package(slot0: u32) -> Vec<u8> {
        let hdr = 0x800usize;
        let pixels = texlib::tiled_buffer_size(64, 64, 8);
        let mut b = vec![0u8; hdr + pixels];
        b[..7].copy_from_slice(texlib::MAGIC);
        let entries: [(u32, u32, u32); 7] = [
            (TAG_PIXELS_XB2, 0, pixels as u32),
            (TAG_RASTER, 0x200, 56),
            (TAG_RASTER, 0x240, 56),
            (TAG_MATERIAL_PARAMS, 0x280, 320),
            (TAG_MATERIAL, 0x3C0, 32),
            (TAG_MATERIAL, 0x3E0, 32),
            (TAG_NAMES, 0x400, 8 + 12 * 3),
        ];
        put(&mut b, 0x20, entries.len() as u32);
        put(&mut b, 0x30, 0x60);
        put(&mut b, 0x44, hdr as u32);
        put(&mut b, 0x54, pixels as u32);
        for (i, (tag, off, size)) in entries.iter().enumerate() {
            let e = 0x60 + i * ENTRY;
            put(&mut b, e, *off);
            put(&mut b, e + 8, *size);
            put(&mut b, e + 20, *tag);
        }
        for (d, dw8_index) in [(0x200usize, 0u32), (0x240, 3)] {
            b[d + 35] = PixelFormat::Dxt1.to_byte();
            put(&mut b, d + 32, (dw8_index << 12) | 0x52);
            put(&mut b, d + 36, (63 << 13) | 63);
        }
        put(&mut b, 0x280 + 256, slot0);
        put(&mut b, 0x280 + 288, 7);
        b[0x3C4..0x3C4 + 13].copy_from_slice(b"env_boards.FX");
        put(&mut b, 0x400, 0x0100_0003);
        for (k, (sp, oi)) in [(0x480u32, 4u32), (0x490, 5), (0x4A0, 99)].iter().enumerate() {
            put(&mut b, 0x408 + 12 * k, *sp);
            put(&mut b, 0x40C + 12 * k, TAG_MATERIAL);
            put(&mut b, 0x410 + 12 * k, *oi);
        }
        b[0x480..0x489].copy_from_slice(b"adbd_WPG\0");
        b[0x490..0x49E].copy_from_slice(b"jumbotron_WPG\0");
        b[0x4A0..0x4A5].copy_from_slice(b"ghost");
        b
    }

    #[test]
    fn materials_follow_validated_slot_words_to_rasters() {
        let b = package(1);
        let p = read(&b).unwrap();
        assert_eq!(p.platform, VenuePlatform::Xbox360);
        assert!(p.sizes_consistent);
        assert_eq!(p.rasters.len(), 2);
        assert_eq!((p.rasters[0].width, p.rasters[0].height, p.rasters[0].format.as_str()), (64, 64, "DXT1"));
        assert_eq!(p.rasters[0].pixel_chunk, Some(0));
        // DW8 naming a non-pixel chunk is reported, never followed.
        assert!(p.rasters[1].pixel_chunk.is_none() && p.rasters[1].pixels_note.as_deref().unwrap().contains("not a pixel entry"));
        // Two named materials; the out-of-range dictionary entry (object 99) and the unterminated string are skipped.
        assert_eq!(p.bad_names, 1);
        let m: Vec<&str> = p.materials.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(m, ["adbd_WPG", "jumbotron_WPG"]);
        assert_eq!(p.materials[0].shader.as_deref(), Some("env_boards.FX"));
        assert_eq!(p.materials[0].slots, [Slot::Raster(0), Slot::Other(7)]);
        // The second material's preceding chunk is a material, not a parameter block.
        assert!(matches!(&p.materials[1].slots[0], Slot::Unreadable(r) if r.contains("not the 0xEF0001")));
        let (f, w, h, linear) = raster_linear(&b, &p, 0).unwrap();
        assert_eq!((f, w, h, linear.len()), (PixelFormat::Dxt1, 64, 64, 64 * 64 / 2));
        assert!(raster_linear(&b, &p, 1).is_err());
    }

    #[test]
    fn slot_words_that_are_not_raster_descriptors_stay_raw() {
        for word in [0u32, 3, 6, 0x0040_0000, 9999] {
            let p = read(&package(word)).unwrap();
            assert_eq!(p.materials[0].slots[0], Slot::Other(word), "{word:#x}: chunk {word} is not a raster descriptor");
        }
    }

    #[test]
    fn a_table_past_the_end_or_a_foreign_magic_is_refused() {
        let mut b = package(1);
        put(&mut b, 0x20, 100_000);
        assert!(matches!(read(&b), Err(VenueError::Table { .. })));
        assert_eq!(read(b"not a package at all, not even close........................................................................").unwrap_err(), VenueError::Magic);
    }
}
