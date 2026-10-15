//! EA **RSF** resource-container mesh reader — the PS4 `.mcd` player/head/logo
//! mesh format (NHL 15 → 21 "Kettle" asset stack).
//!
//! This is the PS4 counterpart to [`crate::playermesh`] (the Xbox 360 / PS3
//! RenderWare `\x89RW4xb2` reader). Both produce the same public
//! [`PlayerAsset`](crate::playermesh::PlayerAsset) data model, so the shared
//! [`write_glb`](crate::playermesh::write_glb) exporter and the
//! `studio-domain` head-preview path work unchanged — only the binary decode
//! differs.
//!
//! ## Container (verified against real NHL 15 files)
//!
//! ```text
//! 0x00  "RSF\0"            magic
//! 0x04  u32  version/flags (0)
//! 0x08  u32  0x30          constant format tag (NOT a directory offset)
//! 0x0C  u32  0
//! 0x10  chunk stream …
//! ```
//!
//! There is **no central directory** — the body is a sequential/nested stream
//! of chunks whose tags are stored as **reversed (little-endian) FourCCs**:
//!
//! | on-disk | tag  | meaning                                   |
//! | ------- | ---- | ----------------------------------------- |
//! | `MOEG`  | GEOM | one geometry block (a submesh / morph)    |
//! | `TMFV`  | VFMT | self-describing vertex format (ASCII)     |
//! | `MRTS`  | STRM | interleaved vertex stream                 |
//! | `XDNI`  | INDX | index buffer                              |
//! | `HSEM`  | MESH | submesh record (tail)                     |
//! | `TSLT`  | TLST | primitive/triangle count (tail)           |
//! | `TIEW`  | VIEW | LOD/view container (player meshes)        |
//!
//! Chunk bodies are **byte-packed** (not 4-byte aligned). Each geometry
//! sub-chunk is immediately preceded by a `01 00 00 00` child-count word.
//!
//! ## VFMT (vertex format)
//!
//! `TMFV | u32 strlen | ASCII` where the string is space-separated attributes,
//! each `name:byteOffset(hex):??:0001:countType`, e.g.
//! `p0:00:00:0001:3f32 n0:0C:00:0001:4s16n t0:14:00:0001:2f16`. The attribute
//! name maps to a semantic ([`attr_semantic`]); `countType` is a component
//! count digit followed by a component format (`f32`/`f16`/`s16n`/`u8n`).
//!
//! Everything is little-endian (PS4), unlike the big-endian RX2 reader.

use crate::playermesh::{BonePalette, PlayerAsset, PositionFormat, Submesh, UvFormat, Vertex};

use thiserror::Error;

/// 4-byte magic at offset 0 of every RSF container (`.mcd`, `.skel`, `.rsf`).
pub const MAGIC_RSF: &[u8; 4] = b"RSF\0";

// Reversed (little-endian) FourCC chunk tags, as they appear on disk.
const TAG_GEOM: &[u8; 4] = b"MOEG"; // GEOM
const TAG_VFMT: &[u8; 4] = b"TMFV"; // VFMT
const TAG_STRM: &[u8; 4] = b"MRTS"; // STRM
const TAG_INDX: &[u8; 4] = b"XDNI"; // INDX
const TAG_MESH: &[u8; 4] = b"HSEM"; // MESH (tail submesh record)

/// Count of `u32` at `tag + STRM_COUNT_OFF` in a STRM/INDX chunk (vertex or
/// index count). Both chunk kinds place their count here — verified on real
/// files.
const CHUNK_COUNT_OFF: usize = 0x09;
/// A STRM chunk's stride `u32` sits here (`tag + 0x0D`).
const STRM_STRIDE_OFF: usize = 0x0D;
/// A STRM chunk's interleaved vertex bytes begin here (`tag + 0x15`).
const STRM_DATA_OFF: usize = 0x15;

#[derive(Debug, Error)]
pub enum McdError {
    #[error("input is shorter than the RSF magic ({0} bytes)")]
    TooShort(usize),
    #[error("magic mismatch — expected `RSF\\0`")]
    BadMagic,
    #[error("no geometry (GEOM/STRM) chunk found in the RSF stream")]
    NoGeometry,
    #[error("submesh {sub}: vertex stream runs past EOF (needs {needed} bytes, have {have})")]
    TruncatedVertexBuffer {
        sub: usize,
        needed: usize,
        have: usize,
    },
    #[error("submesh {sub}: could not locate a valid index buffer")]
    NoIndexBuffer { sub: usize },
    #[error("submesh {sub}: vertex format (VFMT) declared no POSITION attribute")]
    NoPositionAttr { sub: usize },
}

// ------------------------------------------------------------ vertex format

/// The per-component storage of one VFMT attribute.
#[derive(Debug, Clone, Copy, PartialEq)]
enum CompType {
    /// 32-bit float.
    F32,
    /// IEEE half.
    F16,
    /// 16-bit signed, normalized (`i16 / 32767`).
    S16N,
    /// 8-bit unsigned. `normalized` chooses `u8 / 255` vs the raw integer
    /// (blend indices want the raw value, weights want the normalized one).
    U8,
}

impl CompType {
    fn size(self) -> usize {
        match self {
            CompType::F32 => 4,
            CompType::F16 | CompType::S16N => 2,
            CompType::U8 => 1,
        }
    }
}

/// One decoded VFMT attribute: where it sits in a vertex and how it's stored.
#[derive(Debug, Clone)]
struct Attr {
    /// Raw name token (`p0`, `n0`, `t0`, `t7`, `t8`, …).
    name: String,
    /// Byte offset within its **stream's** vertex record.
    offset: usize,
    /// Stream (vertex-buffer) index this attribute is sourced from. NHL 15
    /// heads are single-stream (always 0); NHL 16+ splits some submeshes (the
    /// eyes) into a position stream (0) and a normal/tangent/UV stream (1).
    stream: usize,
    /// Component count (1..=4).
    count: usize,
    comp: CompType,
}

impl Attr {
    fn byte_size(&self) -> usize {
        self.count * self.comp.size()
    }
}

/// Semantic role a VFMT attribute name maps to. `t7`/`t8` carry skinning on
/// PS4 (there are no `i0`/`w0` tokens).
#[derive(Debug, Clone, Copy, PartialEq)]
enum Semantic {
    Position,
    Normal,
    Tangent,
    TexCoord,
    BlendIndices,
    BlendWeights,
    Other,
}

fn attr_semantic(name: &str) -> Semantic {
    match name {
        "p0" => Semantic::Position,
        "n0" => Semantic::Normal,
        "g0" => Semantic::Tangent,
        "t0" | "t1" => Semantic::TexCoord,
        "t7" => Semantic::BlendIndices,
        "t8" => Semantic::BlendWeights,
        _ => Semantic::Other,
    }
}

/// Parse a `countType` token like `3f32`, `4f16`, `4s16n`, `2f16`, `4u8n`.
fn parse_comp_type(tok: &str) -> Option<(usize, CompType)> {
    let count: usize = tok.get(0..1)?.parse().ok()?;
    let rest = &tok[1..];
    let comp = match rest {
        "f32" => CompType::F32,
        "f16" => CompType::F16,
        "s16n" => CompType::S16N,
        "u8n" | "u8" => CompType::U8,
        _ => return None,
    };
    Some((count, comp))
}

/// Parse a VFMT ASCII descriptor into its attributes.
///
/// Format: space-separated `name:offsetHex:streamHex:…:countType`. The tail
/// field count is **version-dependent** — NHL 15 emits five fields
/// (`p0:00:00:0001:3f32`) while NHL 16+ emits seven (`p0:00:00:0001:0000:00:3f32`) —
/// so the component type is always read from the **last** field rather than a
/// fixed index. Field 2 (`streamHex`) is the vertex-buffer index; it is `00`
/// for fully-interleaved layouts and `01` for the attribute half of a
/// split-stream submesh. Unknown attribute types are skipped.
fn parse_vfmt(desc: &str) -> Vec<Attr> {
    let mut attrs = Vec::new();
    for field in desc.split_ascii_whitespace() {
        let parts: Vec<&str> = field.split(':').collect();
        if parts.len() < 5 {
            continue;
        }
        let name = parts[0].to_string();
        let Ok(offset) = usize::from_str_radix(parts[1], 16) else {
            continue;
        };
        let stream = usize::from_str_radix(parts[2], 16).unwrap_or(0);
        let Some((count, comp)) = parse_comp_type(parts[parts.len() - 1]) else {
            continue;
        };
        attrs.push(Attr {
            name,
            offset,
            stream,
            count,
            comp,
        });
    }
    attrs
}

// ------------------------------------------------------------ chunk stream

/// One vertex stream backing a geometry block: where its packed records begin
/// and how wide each record is.
#[derive(Clone, Copy)]
struct GStream {
    /// Absolute offset of the stream's vertex bytes in the file.
    data_off: usize,
    /// Byte stride of one vertex record in this stream.
    stride: usize,
}

/// One geometry block extracted from the stream: its vertex format, the one or
/// more vertex streams that back it, and triangle indices. Single-stream
/// submeshes (all of NHL 15, most of NHL 16) have `streams.len() == 1`;
/// split-stream submeshes (NHL 16+ eyes) have a position stream at index 0 and
/// an attribute stream at index 1.
struct RawGeom {
    attrs: Vec<Attr>,
    streams: Vec<GStream>,
    vertex_count: usize,
    triangles: Vec<[u32; 3]>,
}

/// Find every occurrence of `needle` in `hay` starting at `from`.
fn find_all(hay: &[u8], needle: &[u8], from: usize) -> Vec<usize> {
    let mut hits = Vec::new();
    if needle.is_empty() || hay.len() < needle.len() {
        return hits;
    }
    let mut i = from;
    while i + needle.len() <= hay.len() {
        if &hay[i..i + needle.len()] == needle {
            hits.push(i);
            i += 1;
        } else {
            i += 1;
        }
    }
    hits
}

fn read_u32_le(data: &[u8], off: usize) -> Option<u32> {
    data.get(off..off + 4)
        .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

/// A parsed VFMT: its attributes and the per-stream strides they imply.
struct VfmtEntry {
    attrs: Vec<Attr>,
    /// `(stream index, stride)` for each stream this format references, sorted
    /// by stream index. A stream's stride = max(offset + size) over the
    /// attributes assigned to it.
    streams: Vec<(usize, usize)>,
}

impl VfmtEntry {
    /// The stride of this format's lowest-numbered stream (stream 0 — the
    /// position stream). Used as the match key against a STRM chunk.
    fn lead_stride(&self) -> usize {
        self.streams.first().map(|&(_, s)| s).unwrap_or(0)
    }
}

/// Compute `(stream, stride)` pairs for a set of attributes, sorted by stream.
fn stream_strides(attrs: &[Attr]) -> Vec<(usize, usize)> {
    let mut streams: Vec<(usize, usize)> = Vec::new();
    for a in attrs {
        let end = a.offset + a.byte_size();
        match streams.iter_mut().find(|(s, _)| *s == a.stream) {
            Some((_, stride)) => *stride = (*stride).max(end),
            None => streams.push((a.stream, end)),
        }
    }
    streams.sort_by_key(|&(s, _)| s);
    streams
}

/// Parse every **real** VFMT (a POSITION plus at least one companion attribute),
/// in file order. Standalone single-attribute morph-delta formats (`p0:4f32`,
/// `t7:4f32`, `t8:4f32`) and the bare per-stream sub-descriptors (a position-only
/// or attributes-only fragment) are excluded here — morph decode is separate and
/// the combined descriptor already names every stream.
fn real_vfmts(data: &[u8]) -> Vec<VfmtEntry> {
    let mut out = Vec::new();
    for vfmt in find_all(data, TAG_VFMT, 0x10) {
        let Some(strlen) = read_u32_le(data, vfmt + 4).map(|l| l as usize) else {
            continue;
        };
        let desc_start = vfmt + 8;
        let Some(desc) = data
            .get(desc_start..desc_start + strlen)
            .and_then(|b| std::str::from_utf8(b).ok())
        else {
            continue;
        };
        let attrs = parse_vfmt(desc);
        let has_position = attrs
            .iter()
            .any(|a| attr_semantic(&a.name) == Semantic::Position);
        if attrs.len() < 2 || !has_position {
            continue;
        }
        let streams = stream_strides(&attrs);
        out.push(VfmtEntry { attrs, streams });
    }
    out
}

/// A parsed STRM chunk: vertex count, record stride, and where its packed
/// vertex bytes begin.
#[derive(Clone, Copy)]
struct StrmRec {
    vertex_count: usize,
    stride: usize,
    data_off: usize,
}

/// Parse every STRM chunk in file order, dropping any whose header is malformed
/// or whose declared bytes run past EOF.
fn strm_records(data: &[u8]) -> Vec<StrmRec> {
    let mut out = Vec::new();
    for strm in find_all(data, TAG_STRM, 0x10) {
        let Some(vertex_count) = read_u32_le(data, strm + CHUNK_COUNT_OFF).map(|c| c as usize)
        else {
            continue;
        };
        let Some(stride) = read_u32_le(data, strm + STRM_STRIDE_OFF).map(|s| s as usize) else {
            continue;
        };
        if stride == 0 || vertex_count == 0 || vertex_count > 8_000_000 {
            continue;
        }
        let data_off = strm + STRM_DATA_OFF;
        if data_off + vertex_count * stride > data.len() {
            continue;
        }
        out.push(StrmRec {
            vertex_count,
            stride,
            data_off,
        });
    }
    out
}

/// Walk the RSF stream and return each geometry block, in file order.
///
/// PS4 `.mcd` uses two layouts: simple meshes (logos) interleave VFMT→STRM→INDX
/// per submesh, while player heads use a `TIEW` struct-of-arrays — all VFMTs
/// clustered up front, then all STRMs, then all INDX. Both are handled by a
/// **STRM cursor** walked in file order: at each STRM we find a VFMT whose lead
/// (stream-0) stride matches, then consume one further STRM per additional
/// stream that VFMT declares (NHL 16+ splits the eyes into a stride-12 position
/// stream and a stride-24 attribute stream). VFMTs are taken forward-first, with
/// reuse when none remain ahead — the two eye submeshes share one format. Index
/// buffers are then assigned to whichever geometry they validate against (every
/// index `< vertex_count`), which disambiguates blocks of differing counts.
fn walk_geometry(data: &[u8]) -> Vec<RawGeom> {
    let vfmts = real_vfmts(data);
    if vfmts.is_empty() {
        return Vec::new();
    }
    let strms = strm_records(data);
    let indx_hits = find_all(data, TAG_INDX, 0x10);

    let mut geoms = Vec::new();
    let mut vi = 0usize; // forward cursor into `vfmts`
    let mut si = 0usize; // cursor into `strms`
    while si < strms.len() {
        let lead = strms[si];

        // Match a VFMT by its lead (stream-0) stride: prefer the next such
        // format ahead of the cursor, else reuse the last matching one (shared
        // formats, e.g. the paired eyes).
        let vfmt = match vfmts[vi..]
            .iter()
            .position(|v| v.lead_stride() == lead.stride)
        {
            Some(rel) => {
                vi += rel + 1;
                &vfmts[vi - 1]
            }
            None => match vfmts.iter().rev().find(|v| v.lead_stride() == lead.stride) {
                Some(v) => v,
                None => {
                    si += 1;
                    continue;
                }
            },
        };

        // Bind each stream the VFMT declares to a consecutive STRM, validating
        // stride and equal vertex count. The lead stream is the current STRM;
        // any additional streams follow immediately.
        let mut streams = vec![GStream {
            data_off: lead.data_off,
            stride: lead.stride,
        }];
        let mut ok = true;
        for &(_, stride) in vfmt.streams.iter().skip(1) {
            let next = si + streams.len();
            match strms.get(next) {
                Some(s) if s.stride == stride && s.vertex_count == lead.vertex_count => {
                    streams.push(GStream {
                        data_off: s.data_off,
                        stride: s.stride,
                    });
                }
                _ => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            si += 1;
            continue;
        }

        let consumed = streams.len();
        geoms.push(RawGeom {
            attrs: vfmt.attrs.clone(),
            streams,
            vertex_count: lead.vertex_count,
            triangles: Vec::new(),
        });
        si += consumed;
    }

    // Assign each index buffer to the first geometry (in order) it validates
    // against. Prefer an unconsumed buffer, but fall back to reusing a consumed
    // one when none is free: submeshes with identical topology share a single
    // index buffer (the paired left/right eyes, 66 verts each, carry one INDX
    // between them), so consuming it for the first eye would leave the second
    // untriangulated and invisible.
    let mut consumed = vec![false; indx_hits.len()];
    for g in geoms.iter_mut() {
        let mut fallback: Option<Vec<[u32; 3]>> = None;
        for (k, &ix) in indx_hits.iter().enumerate() {
            let Some(tris) = read_indices(data, ix, g.vertex_count) else {
                continue;
            };
            if !consumed[k] {
                g.triangles = tris;
                consumed[k] = true;
                fallback = None;
                break;
            }
            // A validating-but-already-used buffer: remember it in case nothing
            // free validates for this geometry.
            fallback.get_or_insert(tris);
        }
        if g.triangles.is_empty() {
            if let Some(tris) = fallback {
                g.triangles = tris;
            }
        }
    }
    geoms
}

/// Read an INDX chunk's triangle list. The index-count lives at
/// `tag + CHUNK_COUNT_OFF`; the element width comes from the reversed ASCII
/// token (`61DI`→`ID16`=u16, `23DI`→`ID32`=u32). The data offset (the header
/// carries variable padding) is resolved by trying the candidate positions and
/// keeping the first where every index is `< vertex_count` — self-validating.
fn read_indices(data: &[u8], tag: usize, vertex_count: usize) -> Option<Vec<[u32; 3]>> {
    let count = read_u32_le(data, tag + CHUNK_COUNT_OFF)? as usize;
    if count == 0 || !count.is_multiple_of(3) {
        return None;
    }
    // Reversed "ID16"/"ID32" token within the header.
    let header = data.get(tag..(tag + 0x20).min(data.len()))?;
    let width = if find_all(header, b"61DI", 0).is_empty() {
        if find_all(header, b"23DI", 0).is_empty() {
            2 // default to 16-bit
        } else {
            4
        }
    } else {
        2
    };

    let read_at = |start: usize| -> Option<Vec<[u32; 3]>> {
        let need = count * width;
        let bytes = data.get(start..start + need)?;
        let mut idx = Vec::with_capacity(count);
        for c in bytes.chunks_exact(width) {
            let v = if width == 2 {
                u16::from_le_bytes([c[0], c[1]]) as u32
            } else {
                u32::from_le_bytes([c[0], c[1], c[2], c[3]])
            };
            if v as usize >= vertex_count {
                return None; // out of range at this offset — wrong anchor
            }
            idx.push(v);
        }
        Some(idx.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect())
    };

    // Candidate data starts: just after the count+token header region. The
    // token sits at tag+0x0D; padding after it varies, so try the plausible
    // window and keep the first anchor whose indices all validate in range.
    for cand in tag + 0x11..=tag + 0x18 {
        if let Some(tris) = read_at(cand) {
            return Some(tris);
        }
    }
    None
}

// ------------------------------------------------------------ tail names

/// Extract the submesh names from the tail, in order. Each tail submesh record
/// carries a null-terminated ASCII name (e.g. `logo_bostonShape`,
/// `geo_headShape`) followed by a 6-float AABB. We collect the printable
/// null-terminated strings that follow a MESH/GEOM marker in the tail.
fn tail_names(data: &[u8]) -> Vec<String> {
    let Some(first_mesh) = find_all(data, TAG_MESH, 0x10).first().copied() else {
        return Vec::new();
    };
    let mut names = Vec::new();
    for geom in find_all(data, TAG_GEOM, first_mesh) {
        // The name string starts a little after the GEOM tag (tag + version +
        // size words). Scan the next window for the first plausible
        // null-terminated ASCII run.
        let scan = geom + 8;
        if let Some(name) = read_c_string(data, scan, 64) {
            if is_plausible_name(&name) {
                names.push(name);
            }
        }
    }
    names
}

/// Read a null-terminated ASCII string beginning within `[start, start+window)`,
/// skipping leading non-printable bytes.
fn read_c_string(data: &[u8], start: usize, window: usize) -> Option<String> {
    let end = (start + window).min(data.len());
    let slice = data.get(start..end)?;
    // Find the first printable ASCII byte.
    let begin = slice.iter().position(|&b| (0x20..0x7f).contains(&b))?;
    let mut s = Vec::new();
    for &b in &slice[begin..] {
        if b == 0 {
            break;
        }
        if !(0x20..0x7f).contains(&b) {
            return None; // ran into binary before a terminator — not a string
        }
        s.push(b);
    }
    if s.is_empty() {
        None
    } else {
        String::from_utf8(s).ok()
    }
}

fn is_plausible_name(s: &str) -> bool {
    s.len() >= 3
        && s.len() <= 60
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
        // Real submesh names (Maya shape nodes: `geo_headShape`,
        // `logo_bostonShape`) always carry lowercase letters; the reversed
        // FourCC chunk tags (`TMFV`, `MOEG`, …) are all-caps, so requiring a
        // lowercase letter rejects a tag mistaken for a name.
        && s.chars().any(|c| c.is_ascii_lowercase())
}

// ------------------------------------------------------------ decode

fn half_to_f32(h: u16) -> f32 {
    let sign = ((h >> 15) & 1) as u32;
    let exp = ((h >> 10) & 0x1f) as u32;
    let mant = (h & 0x3ff) as u32;
    let bits = if exp == 0 {
        if mant == 0 {
            sign << 31
        } else {
            // subnormal
            let mut e = -14i32;
            let mut m = mant;
            while m & 0x400 == 0 {
                m <<= 1;
                e -= 1;
            }
            m &= 0x3ff;
            (sign << 31) | (((e + 127) as u32) << 23) | (m << 13)
        }
    } else if exp == 0x1f {
        (sign << 31) | (0xff << 23) | (mant << 13)
    } else {
        (sign << 31) | ((exp + 112) << 23) | (mant << 13)
    };
    f32::from_bits(bits)
}

/// Read `count` components of `comp` starting at `off`.
fn read_components(data: &[u8], off: usize, count: usize, comp: CompType) -> [f32; 4] {
    let mut out = [0.0f32; 4];
    for (i, slot) in out.iter_mut().enumerate().take(count.min(4)) {
        let p = off + i * comp.size();
        *slot = match comp {
            CompType::F32 => data
                .get(p..p + 4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .unwrap_or(0.0),
            CompType::F16 => data
                .get(p..p + 2)
                .map(|b| half_to_f32(u16::from_le_bytes([b[0], b[1]])))
                .unwrap_or(0.0),
            CompType::S16N => data
                .get(p..p + 2)
                .map(|b| (i16::from_le_bytes([b[0], b[1]]) as f32 / 32767.0).clamp(-1.0, 1.0))
                .unwrap_or(0.0),
            CompType::U8 => data.get(p).map(|&b| b as f32 / 255.0).unwrap_or(0.0),
        };
    }
    out
}

/// Read `count` raw `u8` components (for blend indices — the value IS the joint
/// index, not a normalized fraction).
fn read_u8s(data: &[u8], off: usize, count: usize) -> [u16; 4] {
    let mut out = [0u16; 4];
    for (i, slot) in out.iter_mut().enumerate().take(count.min(4)) {
        *slot = data.get(off + i).copied().unwrap_or(0) as u16;
    }
    out
}

fn position_format_of(attrs: &[Attr]) -> PositionFormat {
    attrs
        .iter()
        .find(|a| attr_semantic(&a.name) == Semantic::Position)
        .map(|a| match a.comp {
            CompType::F16 => PositionFormat::HalfFloat,
            _ => PositionFormat::Float,
        })
        .unwrap_or(PositionFormat::Float)
}

fn uv_format_of(attrs: &[Attr]) -> UvFormat {
    attrs
        .iter()
        .find(|a| attr_semantic(&a.name) == Semantic::TexCoord)
        .map(|a| match a.comp {
            CompType::F16 => UvFormat::HalfFloat,
            CompType::F32 => UvFormat::Float,
            CompType::S16N => UvFormat::Short,
            CompType::U8 => UvFormat::UShort,
        })
        .unwrap_or(UvFormat::None)
}

/// Build a [`Submesh`] from one raw geometry block.
fn decode_submesh(
    data: &[u8],
    geom: &RawGeom,
    name: String,
    sub: usize,
) -> Result<Submesh, McdError> {
    let pos_attr = geom
        .attrs
        .iter()
        .find(|a| attr_semantic(&a.name) == Semantic::Position)
        .ok_or(McdError::NoPositionAttr { sub })?;
    let normal_attr = geom
        .attrs
        .iter()
        .find(|a| attr_semantic(&a.name) == Semantic::Normal);
    let uv_attr = geom
        .attrs
        .iter()
        .find(|a| attr_semantic(&a.name) == Semantic::TexCoord);
    let bi_attr = geom
        .attrs
        .iter()
        .find(|a| attr_semantic(&a.name) == Semantic::BlendIndices);
    let bw_attr = geom
        .attrs
        .iter()
        .find(|a| attr_semantic(&a.name) == Semantic::BlendWeights);

    // Every stream must cover all vertices.
    for s in &geom.streams {
        let need = s.data_off + geom.vertex_count * s.stride;
        if need > data.len() {
            return Err(McdError::TruncatedVertexBuffer {
                sub,
                needed: need,
                have: data.len(),
            });
        }
    }

    // Absolute byte offset of attribute `a`'s bytes for vertex `v`, reading from
    // whichever stream the attribute is bound to. Returns `None` if the format
    // names a stream this geometry didn't bind (attribute treated as absent).
    let attr_base = |a: &Attr, v: usize| -> Option<usize> {
        let s = geom.streams.get(a.stream)?;
        Some(s.data_off + v * s.stride + a.offset)
    };

    let mut vertices = Vec::with_capacity(geom.vertex_count);
    for v in 0..geom.vertex_count {
        let position = attr_base(pos_attr, v)
            .map(|b| {
                let p = read_components(data, b, 3, pos_attr.comp);
                [p[0], p[1], p[2]]
            })
            .unwrap_or([0.0, 0.0, 0.0]);
        let normal = normal_attr
            .and_then(|a| attr_base(a, v).map(|b| (a, b)))
            .map(|(a, b)| {
                let n = read_components(data, b, 3, a.comp);
                [n[0], n[1], n[2]]
            })
            .unwrap_or([0.0, 0.0, 0.0]);
        let uv = uv_attr
            .and_then(|a| attr_base(a, v).map(|b| (a, b)))
            .map(|(a, b)| {
                let t = read_components(data, b, 2, a.comp);
                [t[0], t[1]]
            })
            .unwrap_or([0.0, 0.0]);
        let joints = bi_attr
            .and_then(|a| attr_base(a, v).map(|b| (a, b)))
            .map(|(a, b)| read_u8s(data, b, a.count))
            .unwrap_or([0, 0, 0, 0]);
        let weights = bw_attr
            .and_then(|a| attr_base(a, v).map(|b| (a, b)))
            .map(|(a, b)| read_components(data, b, a.count, a.comp))
            .unwrap_or([1.0, 0.0, 0.0, 0.0]);

        vertices.push(Vertex {
            position,
            normal,
            uv,
            joints,
            weights,
        });
    }

    Ok(Submesh {
        name,
        // Informational (retained for a future encoder): the position stream's
        // record width. Split-stream submeshes have no single vertex stride.
        stride: geom.streams.first().map(|s| s.stride).unwrap_or(0),
        position_format: position_format_of(&geom.attrs),
        uv_format: uv_format_of(&geom.attrs),
        has_blend_indices: bi_attr.is_some(),
        has_blend_weights: bw_attr.is_some(),
        vertices,
        triangles: geom.triangles.clone(),
    })
}

/// Parse a PS4 `.mcd` mesh into the shared [`PlayerAsset`] model.
///
/// The skeleton/bind-pose is **external** on PS4 (it lives in the companion
/// `.skel`), so `skin` is an empty palette here — geometry, UVs, normals, and
/// (when present) blend indices/weights are decoded. Feed the result to
/// [`crate::playermesh::write_glb`] for GLB export or to the head-preview path.
pub fn parse_player_mesh(data: &[u8]) -> Result<PlayerAsset, McdError> {
    if data.len() < 4 {
        return Err(McdError::TooShort(data.len()));
    }
    if &data[0..4] != MAGIC_RSF {
        return Err(McdError::BadMagic);
    }

    let geoms = walk_geometry(data);
    if geoms.is_empty() {
        return Err(McdError::NoGeometry);
    }
    let names = tail_names(data);

    let mut submeshes = Vec::with_capacity(geoms.len());
    for (i, geom) in geoms.iter().enumerate() {
        let name = names.get(i).cloned().unwrap_or_else(|| format!("mesh_{i}"));
        submeshes.push(decode_submesh(data, geom, name, i)?);
    }

    Ok(PlayerAsset {
        submeshes,
        skin: BonePalette {
            matrices: Vec::new(),
        },
    })
}

/// Preview-grade parse. Identical to [`parse_player_mesh`] today (the PS4 rig is
/// external, so there's no multi-skin ambiguity to tolerate); kept as a distinct
/// entry point to mirror [`crate::playermesh`] so callers can dispatch on
/// extension without special-casing.
pub fn parse_player_mesh_preview(data: &[u8]) -> Result<PlayerAsset, McdError> {
    parse_player_mesh(data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vfmt_parse_logo_layout() {
        let desc = "p0:00:00:0001:3f32 n0:0C:00:0001:4s16n t0:14:00:0001:2f16";
        let attrs = parse_vfmt(desc);
        assert_eq!(attrs.len(), 3);
        assert_eq!(attrs[0].name, "p0");
        assert_eq!(attrs[0].offset, 0x00);
        assert_eq!(attrs[0].comp, CompType::F32);
        assert_eq!(attrs[1].name, "n0");
        assert_eq!(attrs[1].offset, 0x0c);
        assert_eq!(attrs[1].comp, CompType::S16N);
        assert_eq!(attrs[2].offset, 0x14);
        assert_eq!(attrs[2].comp, CompType::F16);
        // stride = max(offset + size) = 0x14 + 2*2 = 0x18
        let stride = attrs
            .iter()
            .map(|a| a.offset + a.byte_size())
            .max()
            .unwrap();
        assert_eq!(stride, 0x18);
    }

    #[test]
    fn comp_type_tokens() {
        assert_eq!(parse_comp_type("3f32"), Some((3, CompType::F32)));
        assert_eq!(parse_comp_type("4f16"), Some((4, CompType::F16)));
        assert_eq!(parse_comp_type("4s16n"), Some((4, CompType::S16N)));
        assert_eq!(parse_comp_type("2f16"), Some((2, CompType::F16)));
        assert_eq!(parse_comp_type("4u8n"), Some((4, CompType::U8)));
        assert_eq!(parse_comp_type("nonsense"), None);
    }

    #[test]
    fn half_decode_known_values() {
        assert_eq!(half_to_f32(0x0000), 0.0);
        assert_eq!(half_to_f32(0x3c00), 1.0);
        assert_eq!(half_to_f32(0xbc00), -1.0);
        assert!((half_to_f32(0x3555) - 0.3333).abs() < 0.001);
    }

    #[test]
    fn semantic_map() {
        assert_eq!(attr_semantic("p0"), Semantic::Position);
        assert_eq!(attr_semantic("n0"), Semantic::Normal);
        assert_eq!(attr_semantic("t0"), Semantic::TexCoord);
        assert_eq!(attr_semantic("t7"), Semantic::BlendIndices);
        assert_eq!(attr_semantic("t8"), Semantic::BlendWeights);
    }

    #[test]
    fn rejects_non_rsf() {
        assert!(matches!(
            parse_player_mesh(b"\x89RW4xb2........"),
            Err(McdError::BadMagic)
        ));
    }

    /// Live fixture: the operator's extracted NHL 15 (PS4) tree. Skipped when
    /// absent (CI won't have it mounted).
    const EXTRACT_ROOT: &str =
        r"H:\Emulators\ShadPS4\Games\CUSA00561\databigs\Extracted\nhl\data\kettle\rendering";
    /// Live fixture: the operator's extracted NHL 16 (PS4) tree — the split-stream
    /// VFMT variant (`p0:00:00:0001:0000:00:3f32`, stream-indexed eyes).
    const EXTRACT_ROOT_NHL16: &str =
        r"H:\Emulators\ShadPS4\Games\CUSA01909\databigs\Extracted\nhl\data\kettle\rendering";

    fn read_real(rel: &str) -> Option<Vec<u8>> {
        let path = format!("{EXTRACT_ROOT}\\{rel}");
        match std::fs::read(&path) {
            Ok(d) => Some(d),
            Err(_) => {
                eprintln!("skipping: {path} not present");
                None
            }
        }
    }

    fn read_real_nhl16(rel: &str) -> Option<Vec<u8>> {
        let path = format!("{EXTRACT_ROOT_NHL16}\\{rel}");
        match std::fs::read(&path) {
            Ok(d) => Some(d),
            Err(_) => {
                eprintln!("skipping: {path} not present");
                None
            }
        }
    }

    #[test]
    fn decodes_real_logo_mesh() {
        let Some(data) = read_real(r"model\logos_boston.mcd") else {
            return;
        };
        let asset = parse_player_mesh(&data).expect("logos_boston.mcd must parse");
        assert_eq!(asset.submeshes.len(), 1, "one submesh");
        let sm = &asset.submeshes[0];
        assert_eq!(sm.vertices.len(), 7231, "verified vertex count");
        assert_eq!(sm.stride, 24);
        assert_eq!(sm.name, "logo_bostonShape");

        // Vertex 0 verified by hand: pos ≈ (0.0120, -0.5103, 0.0235).
        let v0 = &sm.vertices[0];
        assert!(
            (v0.position[0] - 0.0120).abs() < 0.001,
            "v0.x = {}",
            v0.position[0]
        );
        assert!(
            (v0.position[1] + 0.5103).abs() < 0.001,
            "v0.y = {}",
            v0.position[1]
        );

        // Every position finite and in a sane object-space range.
        for v in &sm.vertices {
            assert!(v.position.iter().all(|c| c.is_finite()));
            assert!(v.position.iter().all(|c| c.abs() < 10.0));
            // Normal (4s16n) decodes to unit length.
            let n = (v.normal[0].powi(2) + v.normal[1].powi(2) + v.normal[2].powi(2)).sqrt();
            assert!(n > 0.9 && n < 1.1, "normal not unit: {n}");
            // UVs in [0,1]-ish.
            assert!(v.uv.iter().all(|c| c.is_finite()));
        }

        // Indices in range, non-empty.
        assert!(!sm.triangles.is_empty(), "index buffer decoded");
        for t in &sm.triangles {
            assert!(t.iter().all(|&i| (i as usize) < sm.vertices.len()));
        }

        // GLB export works on the shared writer.
        let glb = crate::playermesh::write_glb(&asset);
        assert!(glb.len() > 12 && &glb[0..4] == b"glTF", "valid GLB header");
    }

    #[test]
    fn decodes_real_player_head() {
        let Some(data) = read_real(r"characters\player\alexander_ovechkin.mcd") else {
            return;
        };
        let asset = parse_player_mesh(&data).expect("ovechkin head must parse");
        assert!(!asset.submeshes.is_empty(), "head has submeshes");
        let total: usize = asset.submeshes.iter().map(|s| s.vertices.len()).sum();
        assert!(total > 0);
        // Head geometry sits in character space around y≈1.7–1.9.
        let mut max_y = f32::MIN;
        for s in &asset.submeshes {
            for v in &s.vertices {
                assert!(v.position.iter().all(|c| c.is_finite()));
                max_y = max_y.max(v.position[1]);
            }
        }
        assert!(max_y > 1.5 && max_y < 2.2, "head top y = {max_y}");
        let names: Vec<&str> = asset.submeshes.iter().map(|s| s.name.as_str()).collect();
        eprintln!("ovechkin submeshes: {names:?}");

        // Both eyes must be triangulated — they share one index buffer, so a
        // naive consume-once assignment leaves the second eye invisible.
        for eye in ["leye", "reye"] {
            let sm = asset
                .submeshes
                .iter()
                .find(|s| s.name.contains(eye))
                .unwrap_or_else(|| panic!("{eye} submesh present"));
            assert!(!sm.triangles.is_empty(), "{eye} has triangles");
        }
    }

    /// NHL 16 introduced the seven-field VFMT descriptor and split-stream
    /// (position / attribute) submeshes for the eyes. The decoder must handle
    /// both, or the whole head fails with `NoGeometry`.
    #[test]
    fn decodes_real_nhl16_player_head() {
        let Some(data) = read_real_nhl16(r"characters\player\bobby_ryan.mcd") else {
            return;
        };
        let asset = parse_player_mesh(&data).expect("NHL16 bobby_ryan head must parse");
        assert!(!asset.submeshes.is_empty(), "head has submeshes");

        // The main face (a stride-40 skinned block) and a stride-32 block are
        // single-stream; the two eyes are split into a stride-12 position stream
        // and a stride-24 attribute stream. All four should decode.
        let counts: Vec<usize> = asset.submeshes.iter().map(|s| s.vertices.len()).collect();
        eprintln!("NHL16 submesh vertex counts: {counts:?}");
        assert!(counts.contains(&3285), "main face (3285 verts) decoded");
        assert!(counts.contains(&677), "stride-32 block (677 verts) decoded");
        assert!(
            counts.iter().filter(|&&c| c == 66).count() >= 2,
            "both 66-vert eyes (split-stream) decoded"
        );

        // Geometry sits in character head space and is finite; split-stream
        // vertices must have real positions (not the origin-clustered garbage a
        // stream mixup would produce).
        let mut max_y = f32::MIN;
        for s in &asset.submeshes {
            for v in &s.vertices {
                assert!(v.position.iter().all(|c| c.is_finite()));
                max_y = max_y.max(v.position[1]);
            }
            assert!(!s.triangles.is_empty(), "submesh {} triangulated", s.name);
        }
        assert!(max_y > 1.5 && max_y < 2.2, "head top y = {max_y}");
    }
}
