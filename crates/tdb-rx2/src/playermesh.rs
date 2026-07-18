//! Xbox 360 RX2 player-mesh container (`player_<id>.rx2`).
//!
//! Port of the Noesis importer at
//! `E:/Personal Projects/NHL Legacy/Tools/noesisv4474/plugins/python/
//!  mdl_NHLLegacy_X360models_rx2.py` (v1.3). The Noesis script is the
//! ground truth for vertex-format identification — we use the same
//! 8-byte type tags it matches, the same `partID @ VB_descriptor + 40`
//! lookup that maps each sub-mesh to its VertexDescriptor, the same
//! USHORT UV decode (no sign bias), and the same triangle-strip
//! detection (scan for any `0xFFFF` in the index buffer).
//!
//! ## What Noesis gives us
//!
//! - Positions (`HALFFLOAT` tag `00 1A 23 60 00 00 00 01` or `FLOAT`
//!   tag `00 2A 23 B9 00 00 00 01`).
//! - UV0 (`USHORT` tag `00 2C 20 59 00 05 00 06` — unbiased u16/65535).
//! - Triangle indices (u16 BE; primitive-restart strips via `0xFFFF`).
//! - Sub-mesh count + per-sub-mesh VB/IB pairing.
//!
//! ## What we add on top
//!
//! - BLENDINDICES (4×u16 BE) and BLENDWEIGHTS (4×u8 normalized).
//!   Empirically verified against the template: weights sum to 1.0
//!   exactly across multiple verts. BLENDINDICES are **shader-register
//!   indices** — every observed value is a multiple of 3 (max 237 on
//!   the template head); divide by 3 for the palette index (joint 24 →
//!   palette 8 = Head). See `defs/legacy-bone-names.toml`.
//! - AnimationSkin decode → inverse-bind matrices (4×4 fp32 BE); 122 on a
//!   Legacy head, 123 on NHL 10. Chunk layout is `{u32 self_offset, u32
//!   bone_count, 8 bytes pad}` then `bone_count` matrices.
//!   **Per-title difference:** Legacy/14/15 ship ONE AnimationSkin shared by
//!   every submesh; NHL 10 repeats it once per submesh (5 copies on a
//!   5-submesh head), byte-identical apart from each copy's own `self_offset`.
//!   The read path takes the first and verifies the rest agree; the patch path
//!   refuses multi-skin meshes outright (it rewrites one chunk in place).
//! - Sub-mesh name resolution via the name-table records (end-pointer
//!   semantics — see [`decode_name_table`]).
//! - GLB writer that bundles all of the above so the operator can
//!   visually confirm in Blender / glTF Viewer.
//!
//! Normals and tangents are deliberately **not** decoded. Noesis
//! doesn't either — the downstream Blender pipeline recomputes smooth
//! normals from geometry and derives tangents from normals + UVs.
//!
//! See `docs/rx2-player-mesh-format.md` for the byte-level format spec.

use std::collections::HashMap;
use std::convert::TryInto;

use thiserror::Error;

// ---------- Container constants ----------

/// 7-byte magic at offset 0 of every RX2 container — shared with
/// texlib and the equipment-mesh reader.
pub const MAGIC: &[u8; 7] = b"\x89RW4xb2";

const HDR_FILE_COUNT_OFF: usize = 0x20;
const HDR_FILE_TABLE_OFF: usize = 0x30;
const HDR_HDR_SIZE_OFF: usize = 0x44;
const HDR_DATA_SIZE_OFF: usize = 0x54;

const ENTRY_STRIDE: usize = 24;

// File-table chunk tags (interpreted as u32 BE).
const TAG_RAW_PAYLOAD: u32 = 0x0001_0031;
const TAG_VERTEX_DESCRIPTOR: u32 = 0x0002_0004;
const TAG_VERTEX_BUFFER: u32 = 0x0002_0005;
const TAG_INDEX_BUFFER: u32 = 0x0002_0007;
const TAG_MESH_HANDLE: u32 = 0x0002_0009;
const TAG_ANIMATION_SKIN: u32 = 0x0007_0003;
const TAG_FX_RENDERABLE: u32 = 0x00EF_0004;
const TAG_NAME_TABLE: u32 = 0x00EC_0010;

// 8-byte vertex-attribute type tags (Noesis lines 259-281).
const TYPE_POS_FLOAT: [u8; 8] = [0x00, 0x2A, 0x23, 0xB9, 0x00, 0x00, 0x00, 0x01];
const TYPE_POS_HALF: [u8; 8] = [0x00, 0x1A, 0x23, 0x60, 0x00, 0x00, 0x00, 0x01];
const TYPE_UV_FLOAT: [u8; 8] = [0x00, 0x2C, 0x23, 0xA5, 0x00, 0x05, 0x00, 0x06];
const TYPE_UV_HALF: [u8; 8] = [0x00, 0x2C, 0x23, 0x5F, 0x00, 0x05, 0x00, 0x06];
const TYPE_UV_USHORT: [u8; 8] = [0x00, 0x2C, 0x20, 0x59, 0x00, 0x05, 0x00, 0x06];
const TYPE_UV_SHORT: [u8; 8] = [0x00, 0x2C, 0x21, 0x59, 0x00, 0x05, 0x00, 0x06];
// Skin tags (reverse-engineered, not in Noesis — verified empirically:
// 4×u16 BE shader-register indices (multiples of 3; ÷3 = palette
// index into the 122-entry palette), 4×u8 weights sum to 1.0 across
// every sampled vertex).
const TYPE_SKIN_INDICES: [u8; 8] = [0x00, 0x1A, 0x22, 0x5A, 0x00, 0x02, 0x00, 0x0E];
const TYPE_SKIN_WEIGHTS: [u8; 8] = [0x00, 0x1A, 0x20, 0x86, 0x00, 0x01, 0x00, 0x0F];
// NORMAL/TANGENT: packed 2:10:10:10 BE u32. Tag observed at offsets
// 8 (normal) and 12 (tangent) of stride-32 head verts.
const TYPE_NORMAL_PACKED: [u8; 8] = [0x00, 0x2A, 0x21, 0x87, 0x00, 0x03, 0x00, 0x03];

// ---------- Errors ----------

#[derive(Debug, Error)]
pub enum PlayerMeshError {
    #[error("input is shorter than the RX2 magic ({0} bytes)")]
    TooShort(usize),
    #[error("magic mismatch — expected `\\x89RW4xb2`")]
    BadMagic,
    #[error("file table at 0x{offset:X} runs past EOF")]
    TruncatedFileTable { offset: usize },
    #[error("chunk at file-table entry {entry} runs past EOF: needs {needed} bytes, have {have}")]
    TruncatedChunk {
        entry: usize,
        needed: usize,
        have: usize,
    },
    #[error("expected exactly one AnimationSkin chunk, found {0}")]
    SkinCountMismatch(usize),
    #[error(
        "this mesh carries multiple AnimationSkin chunks whose bone palettes disagree — \
         an un-decoded per-submesh rig layout"
    )]
    SkinPalettesDiffer,
    #[error("submesh {sub} VB descriptor partID {part_id} resolves outside the VertexDescriptor table ({avail} entries)")]
    PartIdOutOfRange {
        sub: usize,
        part_id: i32,
        avail: usize,
    },
    #[error("submesh {sub}: no POSITION attribute recognised in VertexDescriptor")]
    NoPositionAttr { sub: usize },
    #[error("patch input does not match the donor: {0}")]
    PatchMismatch(String),
}

// ---------- Public data model ----------

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PositionFormat {
    HalfFloat,
    Float,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UvFormat {
    HalfFloat,
    Float,
    /// Unsigned 16-bit, `u16 / 65535`. Default for player heads.
    UShort,
    /// Signed 16-bit, biased to `[0, 1]`.
    Short,
    None,
}

#[derive(Debug, Clone, Copy)]
pub struct Vertex {
    pub position: [f32; 3],
    /// Decoded from the packed 2:10:10:10 BE u32 at offset 8. `(0,
    /// 0, 0)` when the descriptor has no normal attribute (e.g.
    /// haircap sub-meshes which we don't currently scan for it).
    pub normal: [f32; 3],
    pub uv: [f32; 2],
    /// Raw BLENDINDICES — **shader-register indices**, multiples of 3.
    /// Divide by 3 for the palette index (joint 24 → palette 8 = Head).
    /// All zeros when the sub-mesh has no skin attributes (check
    /// [`Submesh::has_blend_indices`]).
    pub joints: [u16; 4],
    pub weights: [f32; 4],
}

#[derive(Debug, Clone)]
pub struct Submesh {
    pub name: String,
    /// Byte stride of the source vertex layout. Retained for the
    /// future encoder.
    pub stride: usize,
    pub position_format: PositionFormat,
    pub uv_format: UvFormat,
    /// Whether the vertex descriptor declared a BLENDINDICES
    /// attribute. When `false` every [`Vertex::joints`] is the `[0;
    /// 4]` placeholder — the sub-mesh is rigid (e.g. the `eyel_` /
    /// `eyer_` eyeball sub-meshes carry no skinning at all).
    pub has_blend_indices: bool,
    /// Whether the vertex descriptor declared a BLENDWEIGHTS
    /// attribute. When `false` every [`Vertex::weights`] is the
    /// `[1, 0, 0, 0]` placeholder.
    pub has_blend_weights: bool,
    pub vertices: Vec<Vertex>,
    pub triangles: Vec<[u32; 3]>,
}

pub type BoneMatrix = [[f32; 4]; 4];

#[derive(Debug, Clone)]
pub struct BonePalette {
    pub matrices: Vec<BoneMatrix>,
}

#[derive(Debug, Clone)]
pub struct PlayerAsset {
    pub submeshes: Vec<Submesh>,
    pub skin: BonePalette,
}

// ---------- Internal parse helpers ----------

#[derive(Debug, Clone)]
struct FileTableEntry {
    /// Absolute file offset for descriptor chunks; relative to
    /// `hdr_size` for `TAG_RAW_PAYLOAD`.
    raw_offset: u32,
    size: u32,
    tag: u32,
}

impl FileTableEntry {
    fn abs_offset(&self, hdr_size: u32) -> u32 {
        if self.tag == TAG_RAW_PAYLOAD {
            self.raw_offset + hdr_size
        } else {
            self.raw_offset
        }
    }
}

fn read_u32_be(data: &[u8], offset: usize) -> Result<u32, PlayerMeshError> {
    if offset + 4 > data.len() {
        return Err(PlayerMeshError::TruncatedFileTable { offset });
    }
    let bytes: [u8; 4] = data[offset..offset + 4].try_into().expect("len checked");
    Ok(u32::from_be_bytes(bytes))
}

fn read_i32_be(data: &[u8], offset: usize) -> Result<i32, PlayerMeshError> {
    Ok(read_u32_be(data, offset)? as i32)
}

fn read_u16_be(data: &[u8], offset: usize) -> Result<u16, PlayerMeshError> {
    if offset + 2 > data.len() {
        return Err(PlayerMeshError::TruncatedFileTable { offset });
    }
    Ok(u16::from_be_bytes([data[offset], data[offset + 1]]))
}

fn read_f32_be(data: &[u8], offset: usize) -> Result<f32, PlayerMeshError> {
    Ok(f32::from_bits(read_u32_be(data, offset)?))
}

fn half_be(b0: u8, b1: u8) -> f32 {
    let h = ((b0 as u32) << 8) | b1 as u32;
    let s = (h >> 15) & 1;
    let e = (h >> 10) & 0x1F;
    let m = h & 0x3FF;
    let value: f32 = if e == 0 {
        (m as f32 / 1024.0) * 2f32.powi(-14)
    } else if e == 31 {
        f32::INFINITY
    } else {
        (1.0 + m as f32 / 1024.0) * 2f32.powi(e as i32 - 15)
    };
    if s != 0 {
        -value
    } else {
        value
    }
}

// ---------- File-table walk ----------

fn walk_file_table(data: &[u8]) -> Result<(Vec<FileTableEntry>, u32), PlayerMeshError> {
    if data.len() < MAGIC.len() {
        return Err(PlayerMeshError::TooShort(data.len()));
    }
    if &data[..MAGIC.len()] != MAGIC {
        return Err(PlayerMeshError::BadMagic);
    }
    let file_count = read_u32_be(data, HDR_FILE_COUNT_OFF)? as usize;
    let file_table = read_u32_be(data, HDR_FILE_TABLE_OFF)? as usize;
    let mut hdr_size = read_u32_be(data, HDR_HDR_SIZE_OFF)?;
    // NHL 07 packs the whole container into the "header" region (`data_size == 0`, `hdr_size` == the
    // full file length) and stores raw-payload offsets absolutely; NHL 08→Legacy keep a separate data
    // region (`data_size > 0`) whose payload offsets are biased by `hdr_size`. Drop the bias when there
    // is no data region, mirroring `parse_mesh` (lib.rs) — otherwise `abs_offset` runs every raw
    // payload past EOF and the head mesh won't decode.
    let data_size = read_u32_be(data, HDR_DATA_SIZE_OFF)?;
    if data_size == 0 {
        hdr_size = 0;
    }

    let table_bytes = file_count
        .checked_mul(ENTRY_STRIDE)
        .ok_or(PlayerMeshError::TruncatedFileTable { offset: file_table })?;
    if file_table + table_bytes > data.len() {
        return Err(PlayerMeshError::TruncatedFileTable { offset: file_table });
    }

    let mut out = Vec::with_capacity(file_count);
    for i in 0..file_count {
        let base = file_table + i * ENTRY_STRIDE;
        let raw_offset = read_u32_be(data, base)?;
        let size = read_u32_be(data, base + 8)?;
        let tag = read_u32_be(data, base + 20)?;
        out.push(FileTableEntry {
            raw_offset,
            size,
            tag,
        });
    }
    Ok((out, hdr_size))
}

// ---------- Name-table decode ----------

#[derive(Debug, Default)]
struct NameTable {
    by_entry: HashMap<u32, String>,
}

fn decode_name_table(
    data: &[u8],
    nt_offset: usize,
    nt_size: usize,
) -> Result<NameTable, PlayerMeshError> {
    if nt_offset + nt_size > data.len() {
        return Err(PlayerMeshError::TruncatedChunk {
            entry: usize::MAX,
            needed: nt_size,
            have: data.len().saturating_sub(nt_offset),
        });
    }
    // Header: +0x00 u32 count, then ignored. Records start at +0x0C
    // and are 12 bytes each: (tag u32, entry_idx u32, end_pointer u32).
    // The `end_pointer` is the offset AFTER this record's name (= the
    // start of the next record's name). Names live linearly in the
    // strings region starting at records_end.
    let count = read_u32_be(data, nt_offset)? as usize;
    let records_start = nt_offset + 0x0C;
    let strings_start = records_start + count * 12;

    let mut nt = NameTable::default();
    let mut name_start = strings_start;
    for i in 0..count {
        let rec = records_start + i * 12;
        if rec + 12 > data.len() {
            break;
        }
        let entry_idx = read_u32_be(data, rec + 4)?;
        let end_ptr = read_u32_be(data, rec + 8)? as usize;
        if end_ptr > name_start && end_ptr <= data.len() {
            let s_end = end_ptr.saturating_sub(1);
            let s_end = if data.get(s_end) == Some(&0) {
                s_end
            } else {
                end_ptr
            };
            let s = String::from_utf8_lossy(&data[name_start..s_end]).into_owned();
            nt.by_entry.insert(entry_idx, s);
        }
        name_start = end_ptr;
    }
    Ok(nt)
}

// ---------- AnimationSkin decode ----------

/// `{u32 self_offset, u32 bone_count, 8 bytes pad}` ahead of the matrix block.
const SKIN_HEADER_LEN: usize = 16;

fn decode_animation_skin(
    data: &[u8],
    offset: usize,
    size: usize,
) -> Result<BonePalette, PlayerMeshError> {
    if offset + size > data.len() {
        return Err(PlayerMeshError::TruncatedChunk {
            entry: usize::MAX,
            needed: size,
            have: data.len().saturating_sub(offset),
        });
    }
    let bone_count = read_u32_be(data, offset + 4)? as usize;
    let mat_start = offset + SKIN_HEADER_LEN;
    let mut matrices = Vec::with_capacity(bone_count);
    for b in 0..bone_count {
        let m_off = mat_start + b * 64;
        if m_off + 64 > data.len() {
            return Err(PlayerMeshError::TruncatedChunk {
                entry: usize::MAX,
                needed: 64,
                have: data.len().saturating_sub(m_off),
            });
        }
        let mut m: BoneMatrix = [[0.0; 4]; 4];
        for row in 0..4 {
            for col in 0..4 {
                m[row][col] = read_f32_be(data, m_off + (row * 4 + col) * 4)?;
            }
        }
        matrices.push(m);
    }
    Ok(BonePalette { matrices })
}

// ---------- VertexDescriptor decode (Noesis lines 254-285) ----------

#[derive(Debug, Clone)]
struct VertexDescriptor {
    stride: usize,
    /// Each attribute: `(offset_in_stride, 8-byte type tag)`.
    attrs: Vec<(u32, [u8; 8])>,
}

fn decode_vertex_descriptor(
    data: &[u8],
    offset: usize,
) -> Result<VertexDescriptor, PlayerMeshError> {
    // Noesis: `info = bs.read(">iiBBBBBBBB")` at descriptor start.
    //   info[0..2] = two i32 (header words; unused).
    //   info[2]    = u8 attribute count.
    //   info[3]    = u8 vertex byte stride.
    //   info[4..8] = four more u8 (unused).
    // Then `info[2]` attribute records, each:
    //   `position = bs.read(">i")`  — offset within stride
    //   `type     = bs.readBytes(8)` — 8-byte tag
    if offset + 16 > data.len() {
        return Err(PlayerMeshError::TruncatedChunk {
            entry: usize::MAX,
            needed: 16,
            have: data.len().saturating_sub(offset),
        });
    }
    // Noesis `>iiBBBBBBBB`: 2 i32 (8 bytes) then 8 u8.
    //   info[2] = byte 8  = attribute count
    //   info[3] = byte 9  = vertex byte stride
    let n_attrs = data[offset + 8] as usize;
    let stride = data[offset + 9] as usize;
    let mut attrs = Vec::with_capacity(n_attrs);
    let mut p = offset + 16;
    for _ in 0..n_attrs {
        if p + 12 > data.len() {
            return Err(PlayerMeshError::TruncatedChunk {
                entry: usize::MAX,
                needed: 12,
                have: data.len().saturating_sub(p),
            });
        }
        let attr_off = read_u32_be(data, p)?;
        let mut tag = [0u8; 8];
        tag.copy_from_slice(&data[p + 4..p + 12]);
        attrs.push((attr_off, tag));
        p += 12;
    }
    Ok(VertexDescriptor { stride, attrs })
}

// ---------- Strip → triangle walker ----------

/// Align each triangle's winding to its vertex normals. NHL 07 head strips carry correct per-vertex
/// normals but inconsistent winding — the strip-parity/degenerate-stitch decode leaves a cluster of
/// triangles wound backwards. A double-sided renderer keys front/back on winding (`gl_FrontFacing`)
/// and flips the shading normal for "back" faces, so a back-wound triangle with outward normals gets
/// its normal flipped inward and renders as a dark patch. Swapping the winding of any triangle whose
/// geometric normal opposes its vertex normals makes the mesh consistently wound, so the renderer
/// never wrongly flips. A correctly-wound mesh (NHL 08+) is unaffected — no triangle trips the test.
fn align_winding_to_normals(vertices: &[Vertex], mut triangles: Vec<[u32; 3]>) -> Vec<[u32; 3]> {
    for t in &mut triangles {
        let p = |i: u32| vertices[i as usize].position;
        let (a, b, c) = (p(t[0]), p(t[1]), p(t[2]));
        let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        // Geometric normal from winding (right-hand cross product).
        let g = [
            ab[1] * ac[2] - ab[2] * ac[1],
            ab[2] * ac[0] - ab[0] * ac[2],
            ab[0] * ac[1] - ab[1] * ac[0],
        ];
        let n = |i: u32| vertices[i as usize].normal;
        let (n0, n1, n2) = (n(t[0]), n(t[1]), n(t[2]));
        let vn = [n0[0] + n1[0] + n2[0], n0[1] + n1[1] + n2[1], n0[2] + n1[2] + n2[2]];
        if g[0] * vn[0] + g[1] * vn[1] + g[2] * vn[2] < 0.0 {
            t.swap(1, 2);
        }
    }
    triangles
}

/// Drop triangle-strip "bridge" slivers — triangles whose longest edge is more than 8× the mesh's
/// median edge. Such a triangle spans anatomically distant vertices (e.g. lip to eyebrow) and is a
/// strip-stitch artifact, not a surface triangle. A well-formed mesh's edges are near-uniform, so the
/// 8× gate never fires on real geometry (verified: NHL 07 heads flag only 3 stitch pairs; the eye and
/// inner submeshes flag none). Returns the input unchanged for tiny meshes where a median is meaningless.
fn drop_sliver_bridges(vertices: &[Vertex], triangles: Vec<[u32; 3]>) -> Vec<[u32; 3]> {
    if triangles.len() < 8 {
        return triangles;
    }
    let edge = |a: u32, b: u32| -> f32 {
        let (p, q) = (vertices[a as usize].position, vertices[b as usize].position);
        ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt()
    };
    let mut lens: Vec<f32> = Vec::with_capacity(triangles.len() * 3);
    for t in &triangles {
        lens.push(edge(t[0], t[1]));
        lens.push(edge(t[1], t[2]));
        lens.push(edge(t[2], t[0]));
    }
    lens.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = lens[lens.len() / 2].max(f32::MIN_POSITIVE);
    let limit = median * 8.0;
    triangles
        .into_iter()
        .filter(|t| {
            edge(t[0], t[1]).max(edge(t[1], t[2])).max(edge(t[2], t[0])) <= limit
        })
        .collect()
}

fn strips_to_triangles(indices: &[u16]) -> Vec<[u32; 3]> {
    let mut out = Vec::with_capacity(indices.len() / 2);
    let mut strip: Vec<u16> = Vec::new();
    let flush = |strip: &[u16], out: &mut Vec<[u32; 3]>| {
        if strip.len() < 3 {
            return;
        }
        for k in 0..strip.len() - 2 {
            let (a, b, c) = (strip[k], strip[k + 1], strip[k + 2]);
            if a == b || b == c || a == c {
                continue;
            }
            if k % 2 == 0 {
                out.push([a as u32, b as u32, c as u32]);
            } else {
                out.push([a as u32, c as u32, b as u32]);
            }
        }
    };
    for &idx in indices {
        if idx == 0xFFFF {
            flush(&strip, &mut out);
            strip.clear();
        } else {
            strip.push(idx);
        }
    }
    flush(&strip, &mut out);
    out
}

// ---------- Top-level parse ----------

/// How the parser treats the AnimationSkin chunk(s).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinPolicy {
    /// Require a palette, and reject a mesh whose multiple AnimationSkin
    /// chunks disagree (only byte-identical copies are accepted). The
    /// contract every *edit* path needs — a mesh with an ambiguous rig must
    /// not be silently patched.
    Strict,
    /// Require a palette, but on multiple chunks keep the FIRST and tolerate
    /// the rest differing. **Preview only** — the chosen rig may be wrong, but
    /// the geometry is intact, which is all a viewer needs. This is what lets
    /// NHL 10's 312 per-submesh-rig heads render for inspection.
    LenientFirst,
    /// No palette required (equipment / body parts skin against a shared
    /// bindpose asset).
    Optional,
}

pub fn parse_player_mesh(data: &[u8]) -> Result<PlayerAsset, PlayerMeshError> {
    parse_mesh_impl(data, SkinPolicy::Strict)
}

/// Preview-grade parse: like [`parse_player_mesh`] but tolerates the
/// per-submesh multi-skin layout NHL 10 uses (see [`SkinPolicy::LenientFirst`]).
/// Never use the result to write a mesh back.
pub fn parse_player_mesh_preview(data: &[u8]) -> Result<PlayerAsset, PlayerMeshError> {
    parse_mesh_impl(data, SkinPolicy::LenientFirst)
}

/// Equipment-mesh variant of [`parse_player_mesh`]: identical decode,
/// but tolerates assets with NO `ANIMATION_SKIN` chunk. The body-part
/// and equipment meshes (`jersey_*`, `pant_*`, `playersock_*`,
/// `playerskate_*`, `hand_*`, `glove_*`, `helmet_*`, …) ship without
/// one — they skin against the shared `skeleton_bindpose_player.rx2`
/// palette instead of carrying their own (see
/// `docs/body-mesh-spike.md`). `skin` is then an empty palette.
pub fn parse_equipment_mesh(data: &[u8]) -> Result<PlayerAsset, PlayerMeshError> {
    parse_mesh_impl(data, SkinPolicy::Optional)
}

fn parse_mesh_impl(data: &[u8], skin_policy: SkinPolicy) -> Result<PlayerAsset, PlayerMeshError> {
    let (entries, hdr_size) = walk_file_table(data)?;
    // NHL 07 packs the container into the header region (`data_size == 0`). It also stores geometry
    // payloads NOT as the entry preceding each descriptor (the NHL 08+ layout) but as separate,
    // untagged (`tag == 0`) trailing entries: each index-buffer descriptor names its payload by
    // file-table entry index (u16 at descriptor+26), and the vertex-buffer payloads are the remaining
    // untagged entries in order. Resolved after the tag walk below.
    let no_data_region = read_u32_be(data, HDR_DATA_SIZE_OFF)? == 0;

    // Per-tag collections (Noesis lines 164-203). Order matters:
    // sub-meshes use the i-th VB descriptor with the i-th IB descriptor.
    //
    // VOff/VBuf — raw payload offset/size (preceding entry).
    // VInfo    — VB descriptor file offset.
    // FOff/FBuf — raw payload offset/size of index buffer.
    // FInfo    — IB descriptor file offset.
    let mut vb_payloads: Vec<(u32, u32)> = Vec::new(); // (rel_off, size)
    let mut vb_descs: Vec<u32> = Vec::new();
    let mut vb_entries: Vec<u32> = Vec::new(); // file-table entry index of each VB descriptor
    let mut ib_payloads: Vec<(u32, u32)> = Vec::new();
    let mut ib_descs: Vec<u32> = Vec::new();
    let mut vertex_descriptors: Vec<(u32, VertexDescriptor)> = Vec::new(); // (FT entry idx, parsed)
    let mut mesh_count = 0usize;
    let mut name_table = NameTable::default();
    let mut skin: Option<BonePalette> = None;
    // The raw matrix block of the first AnimationSkin seen, for the multi-skin
    // equality check below.
    let mut skin_block: Option<&[u8]> = None;

    for (i, e) in entries.iter().enumerate() {
        match e.tag {
            TAG_VERTEX_BUFFER => {
                if i == 0 {
                    continue;
                }
                // NHL 07 resolves payloads after the walk (see below); NHL 08+ payload is the
                // preceding entry.
                if !no_data_region {
                    let prev = &entries[i - 1];
                    vb_payloads.push((prev.raw_offset, prev.size));
                }
                vb_descs.push(e.raw_offset);
                vb_entries.push(i as u32);
            }
            TAG_INDEX_BUFFER => {
                if i == 0 {
                    continue;
                }
                if !no_data_region {
                    let prev = &entries[i - 1];
                    ib_payloads.push((prev.raw_offset, prev.size));
                }
                ib_descs.push(e.raw_offset);
            }
            TAG_VERTEX_DESCRIPTOR => {
                let vd = decode_vertex_descriptor(data, e.raw_offset as usize)?;
                vertex_descriptors.push((i as u32, vd));
            }
            TAG_MESH_HANDLE => {
                mesh_count += 1;
            }
            TAG_NAME_TABLE => {
                name_table =
                    decode_name_table(data, e.abs_offset(hdr_size) as usize, e.size as usize)?;
            }
            // Legacy/14/15 carry ONE AnimationSkin shared by every submesh.
            // NHL 10 instead repeats the palette once per submesh (5 chunks for
            // a 5-submesh head). Verified on its whole `rendering/player` set:
            // the copies are byte-identical apart from each chunk's own
            // self-referential offset at bytes 0..4, so the first is lossless.
            // Guard rather than assume — if the palettes ever really diverge,
            // that's a layout we haven't decoded, and silently taking the first
            // would bake in a wrong rig.
            TAG_ANIMATION_SKIN => {
                let off = e.abs_offset(hdr_size) as usize;
                // Compare the raw matrix block, NOT the decoded matrices: every
                // bone carries exactly one NaN sentinel, and `NaN != NaN` makes
                // f32 equality report byte-identical palettes as different.
                let block = data
                    .get(off + SKIN_HEADER_LEN..off + e.size as usize)
                    .ok_or(PlayerMeshError::TruncatedChunk {
                        entry: i,
                        needed: e.size as usize,
                        have: data.len().saturating_sub(off),
                    })?;
                match &skin_block {
                    None => {
                        skin_block = Some(block);
                        skin = Some(decode_animation_skin(data, off, e.size as usize)?);
                    }
                    Some(first) if *first == block => {}
                    // The copies genuinely differ. A preview keeps the first
                    // (geometry is unaffected); an edit refuses.
                    Some(_) if skin_policy == SkinPolicy::LenientFirst => {}
                    Some(_) => return Err(PlayerMeshError::SkinPalettesDiffer),
                }
            }
            _ => {}
        }
    }

    // NHL 07 payload resolution. Each IB descriptor names its payload by file-table entry index at
    // descriptor byte +26; the VB payloads are the untagged (`tag == 0`) entries the IBs didn't claim,
    // taken in file-table order to match the VB descriptors. Verified against a real install: a head's
    // index buffer (6546 indices) sits in a separate 13104-byte entry the descriptor points at, while
    // the "preceding entry" the NHL 08+ path would use is an unrelated 36-byte descriptor.
    if no_data_region {
        let mut ib_claimed: Vec<bool> = vec![false; entries.len()];
        for &desc_off in &ib_descs {
            let idx = read_u16_be(data, desc_off as usize + 26)? as usize;
            let pe = entries
                .get(idx)
                .ok_or(PlayerMeshError::TruncatedFileTable { offset: idx })?;
            ib_payloads.push((pe.raw_offset, pe.size));
            ib_claimed[idx] = true;
        }
        let vb_pool: Vec<(u32, u32)> = entries
            .iter()
            .enumerate()
            .filter(|(idx, e)| e.tag == 0 && e.size > 0 && !ib_claimed[*idx])
            .map(|(_, e)| (e.raw_offset, e.size))
            .collect();
        for k in 0..vb_descs.len() {
            match vb_pool.get(k) {
                Some(&p) => vb_payloads.push(p),
                None => break,
            }
        }
    }

    let skin = match skin {
        Some(s) => s,
        None if skin_policy != SkinPolicy::Optional => {
            return Err(PlayerMeshError::SkinCountMismatch(0))
        }
        None => BonePalette {
            matrices: Vec::new(),
        },
    };
    let descriptor_count = vertex_descriptors.len();
    let first_vd_entry = vertex_descriptors
        .first()
        .map(|(idx, _)| *idx as i32)
        .unwrap_or(0);

    // NHL 07 head meshes can declare more mesh handles than they carry vertex/index buffers (one
    // handle has no geometry of its own). Clamp to the submeshes actually backed by a VB+IB pair
    // rather than indexing past the buffer arrays. NHL 08→Legacy are 1:1, so this is a no-op there.
    let mesh_count = mesh_count.min(vb_descs.len()).min(ib_descs.len());

    // Resolve sub-meshes by name table when possible. Mesh handles are
    // the 5 entries with tag 00 02 00 09 — name-table entry_idx for
    // each one yields "eyel_.Mesh", "head_.Mesh", etc.
    let mut mesh_handle_indices: Vec<u32> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.tag == TAG_MESH_HANDLE)
        .map(|(i, _)| i as u32)
        .collect();
    mesh_handle_indices.truncate(mesh_count);

    let mut submeshes = Vec::with_capacity(mesh_count);
    for i in 0..mesh_count {
        // Map sub-mesh i → VertexDescriptor. When fewer descriptors than
        // sub-meshes, read partID from VB-desc + 40 (Noesis line 248).
        let vb_desc_off = vb_descs[i] as usize;
        let descriptor_idx = if descriptor_count < mesh_count {
            let part_id = read_i32_be(data, vb_desc_off + 40)?;
            let resolved = part_id - first_vd_entry;
            if resolved < 0 || (resolved as usize) >= descriptor_count {
                return Err(PlayerMeshError::PartIdOutOfRange {
                    sub: i,
                    part_id,
                    avail: descriptor_count,
                });
            }
            resolved as usize
        } else {
            i.min(descriptor_count.saturating_sub(1))
        };
        let vd = vertex_descriptors[descriptor_idx].1.clone();

        // VB descriptor +32 holds (stride : u16, ?, vert_count : u32).
        // Noesis: `VData = bs.read(">HHi")` then VCount = VData[2].
        let v_stride = read_u16_be(data, vb_desc_off + 32)? as usize;
        let v_count = read_u32_be(data, vb_desc_off + 36)? as usize;
        debug_assert_eq!(
            v_stride, vd.stride,
            "VB-desc stride disagrees with VertexDescriptor"
        );

        // IB descriptor +32 holds (?, idx_count : u16).
        let ib_desc_off = ib_descs[i] as usize;
        let raw_idx_count = read_u16_be(data, ib_desc_off + 34)? as usize;

        // Raw payload offsets are stored relative to hdr_size.
        let vb_data_off = (vb_payloads[i].0 + hdr_size) as usize;
        let vb_data_size = vb_payloads[i].1 as usize;
        let ib_data_off = (ib_payloads[i].0 + hdr_size) as usize;
        let ib_data_size = ib_payloads[i].1 as usize;

        if vb_data_off + vb_data_size > data.len() || ib_data_off + ib_data_size > data.len() {
            return Err(PlayerMeshError::TruncatedChunk {
                entry: i,
                needed: vb_data_size + ib_data_size,
                have: data.len(),
            });
        }

        // Identify position and UV attributes from the type tag.
        let mut pos_off: Option<(u32, PositionFormat)> = None;
        let mut uv_off: (Option<u32>, UvFormat) = (None, UvFormat::None);
        let mut joints_off: Option<u32> = None;
        let mut weights_off: Option<u32> = None;
        // First-found NORMAL packed slot — NHL Legacy puts normal at
        // offset 8 (the first `0x002A2187` attribute) and tangent at
        // offset 12 (the second). We only decode normal.
        let mut normal_off: Option<u32> = None;
        for (off, tag) in &vd.attrs {
            if *tag == TYPE_POS_FLOAT {
                pos_off = Some((*off, PositionFormat::Float));
            } else if *tag == TYPE_POS_HALF {
                pos_off = Some((*off, PositionFormat::HalfFloat));
            } else if *tag == TYPE_UV_USHORT {
                uv_off = (Some(*off), UvFormat::UShort);
            } else if *tag == TYPE_UV_SHORT {
                uv_off = (Some(*off), UvFormat::Short);
            } else if *tag == TYPE_UV_HALF {
                uv_off = (Some(*off), UvFormat::HalfFloat);
            } else if *tag == TYPE_UV_FLOAT {
                uv_off = (Some(*off), UvFormat::Float);
            } else if *tag == TYPE_SKIN_INDICES {
                joints_off = Some(*off);
            } else if *tag == TYPE_SKIN_WEIGHTS {
                weights_off = Some(*off);
            } else if *tag == TYPE_NORMAL_PACKED && normal_off.is_none() {
                normal_off = Some(*off);
            }
        }
        let (p_off, p_fmt) = pos_off.ok_or(PlayerMeshError::NoPositionAttr { sub: i })?;

        // Decode vertices.
        let mut vertices = Vec::with_capacity(v_count);
        for v_idx in 0..v_count {
            let base = vb_data_off + v_idx * vd.stride;
            let position = match p_fmt {
                PositionFormat::HalfFloat => [
                    half_be(data[base + p_off as usize], data[base + p_off as usize + 1]),
                    half_be(
                        data[base + p_off as usize + 2],
                        data[base + p_off as usize + 3],
                    ),
                    half_be(
                        data[base + p_off as usize + 4],
                        data[base + p_off as usize + 5],
                    ),
                ],
                PositionFormat::Float => [
                    read_f32_be(data, base + p_off as usize)?,
                    read_f32_be(data, base + p_off as usize + 4)?,
                    read_f32_be(data, base + p_off as usize + 8)?,
                ],
            };
            let uv = if let Some(u_off) = uv_off.0 {
                let o = base + u_off as usize;
                match uv_off.1 {
                    UvFormat::UShort => {
                        let u = u16::from_be_bytes([data[o], data[o + 1]]);
                        let v = u16::from_be_bytes([data[o + 2], data[o + 3]]);
                        [u as f32 / 65535.0, v as f32 / 65535.0]
                    }
                    UvFormat::Short => {
                        let u = i16::from_be_bytes([data[o], data[o + 1]]);
                        let v = i16::from_be_bytes([data[o + 2], data[o + 3]]);
                        [
                            (u as i32 + 32768) as f32 / 65535.0,
                            (v as i32 + 32768) as f32 / 65535.0,
                        ]
                    }
                    UvFormat::HalfFloat => [
                        half_be(data[o], data[o + 1]),
                        half_be(data[o + 2], data[o + 3]),
                    ],
                    UvFormat::Float => [read_f32_be(data, o)?, read_f32_be(data, o + 4)?],
                    UvFormat::None => [0.0, 0.0],
                }
            } else {
                [0.0, 0.0]
            };
            let joints = if let Some(off) = joints_off {
                let o = base + off as usize;
                [
                    u16::from_be_bytes([data[o], data[o + 1]]),
                    u16::from_be_bytes([data[o + 2], data[o + 3]]),
                    u16::from_be_bytes([data[o + 4], data[o + 5]]),
                    u16::from_be_bytes([data[o + 6], data[o + 7]]),
                ]
            } else {
                [0; 4]
            };
            // BLENDWEIGHT bytes pair with BLENDINDICES in REVERSE slot
            // order: weights byte 3 belongs to joints[0], byte 2 to
            // joints[1], etc. (PR10.1 finding, 2026-06-04 — proven by
            // donor vertices like joints [24,108,108,108] / bytes
            // [0,0,44,211], which only decode to a sane 83%/17%
            // head/eyelid blend under reversed pairing; forward
            // pairing splits one bone across two slots and was the
            // root cause of the in-game shredding of packed assets.)
            // Decoded `weights[c]` ALWAYS pairs with `joints[c]`;
            // dominant influence lands in slot 0 (joints are stored
            // dominant-first, with pads repeating an active joint).
            let weights = if let Some(off) = weights_off {
                let o = base + off as usize;
                [
                    data[o + 3] as f32 / 255.0,
                    data[o + 2] as f32 / 255.0,
                    data[o + 1] as f32 / 255.0,
                    data[o] as f32 / 255.0,
                ]
            } else {
                [1.0, 0.0, 0.0, 0.0]
            };
            // Decode packed 2:10:10:10 BE normal if the descriptor
            // declared one. Bit order matches the encoder (Z high,
            // X low) — see `pack::write_packed_2_10_10_10_be`.
            let normal = if let Some(off) = normal_off {
                let o = base + off as usize;
                let p = u32::from_be_bytes([data[o], data[o + 1], data[o + 2], data[o + 3]]);
                let x_raw = (p & 0x3FF) as i32;
                let y_raw = ((p >> 10) & 0x3FF) as i32;
                let z_raw = ((p >> 20) & 0x3FF) as i32;
                let sx = if x_raw >= 512 { x_raw - 1024 } else { x_raw } as f32 / 511.0;
                let sy = if y_raw >= 512 { y_raw - 1024 } else { y_raw } as f32 / 511.0;
                let sz = if z_raw >= 512 { z_raw - 1024 } else { z_raw } as f32 / 511.0;
                [sx, sy, sz]
            } else {
                [0.0, 0.0, 0.0]
            };
            vertices.push(Vertex {
                position,
                normal,
                uv,
                joints,
                weights,
            });
        }

        // Decode indices. Noesis: scan the whole buffer for any 0xFFFF
        // to decide strip vs list (lines 296-301).
        let u16_count = (ib_data_size / 2).min(raw_idx_count);
        let mut indices = Vec::with_capacity(u16_count);
        let mut is_strip = false;
        for k in 0..u16_count {
            let p = ib_data_off + k * 2;
            let idx = u16::from_be_bytes([data[p], data[p + 1]]);
            if idx == 0xFFFF {
                is_strip = true;
            }
            indices.push(idx);
        }
        let triangles = if is_strip {
            strips_to_triangles(&indices)
        } else {
            // Noesis: when count % 3 != 0 subtract 2 (lines 305-307).
            let trim = if raw_idx_count % 3 != 0 { 2 } else { 0 };
            let usable = raw_idx_count.saturating_sub(trim);
            indices
                .chunks_exact(3)
                .take(usable / 3)
                .map(|c| [c[0] as u32, c[1] as u32, c[2] as u32])
                .collect()
        };

        let name = name_table
            .by_entry
            .get(&mesh_handle_indices[i])
            .cloned()
            .unwrap_or_else(|| format!("mesh_{i}"));

        // NHL 07 head strips occasionally bridge two distant regions of the face with a thin sliver
        // triangle (a strip-stitch artifact the game hides but a double-sided preview shows as a
        // stretch, with its dark underside reading as a hole). They are unambiguous — the sliver's
        // longest edge is an order of magnitude past the mesh's typical edge — so drop them. Gated to
        // the NHL 07 layout; NHL 08+ meshes carry no such slivers and never trip the threshold.
        let triangles = if no_data_region {
            let triangles = drop_sliver_bridges(&vertices, triangles);
            align_winding_to_normals(&vertices, triangles)
        } else {
            triangles
        };

        submeshes.push(Submesh {
            name,
            stride: vd.stride,
            position_format: p_fmt,
            uv_format: uv_off.1,
            has_blend_indices: joints_off.is_some(),
            has_blend_weights: weights_off.is_some(),
            vertices,
            triangles,
        });
    }

    Ok(PlayerAsset { submeshes, skin })
}

// ---------- Minimal GLB writer ----------

/// Write `asset` as a glTF 2.0 binary container (`.glb`).
///
/// Per-submesh: positions, UVs, joints, weights, triangle indices.
/// Normals/tangents are intentionally omitted — Blender recomputes
/// smooth normals from geometry on import, which is what we want for
/// the wrap pipeline.
///
/// Skin: 122-bone palette emitted as a flat-hierarchy armature so
/// vertex-group names (`bone_NNN`) survive the round-trip into and
/// out of Blender.
pub fn write_glb(asset: &PlayerAsset) -> Vec<u8> {
    use serde_json::{json, Value};

    let mut bin: Vec<u8> = Vec::new();
    let mut buffer_views: Vec<Value> = Vec::new();
    let mut accessors: Vec<Value> = Vec::new();

    fn align4(bin: &mut Vec<u8>) {
        while bin.len() % 4 != 0 {
            bin.push(0);
        }
    }

    fn push_view(
        bin: &mut Vec<u8>,
        views: &mut Vec<Value>,
        bytes: &[u8],
        target: Option<u32>,
    ) -> usize {
        align4(bin);
        let off = bin.len();
        bin.extend_from_slice(bytes);
        let mut v = json!({
            "buffer": 0,
            "byteOffset": off,
            "byteLength": bytes.len(),
        });
        if let Some(t) = target {
            v["target"] = json!(t);
        }
        views.push(v);
        views.len() - 1
    }

    fn push_accessor(
        accessors: &mut Vec<Value>,
        view_idx: usize,
        component_type: u32,
        count: usize,
        ty: &str,
        min_max: Option<(Vec<f32>, Vec<f32>)>,
        normalized: bool,
    ) -> usize {
        let mut v = json!({
            "bufferView": view_idx,
            "componentType": component_type,
            "count": count,
            "type": ty,
        });
        if normalized {
            v["normalized"] = json!(true);
        }
        if let Some((min, max)) = min_max {
            v["min"] = json!(min);
            v["max"] = json!(max);
        }
        accessors.push(v);
        accessors.len() - 1
    }

    const FLOAT: u32 = 5126;
    const UBYTE: u32 = 5121;
    const USHORT: u32 = 5123;
    const UINT: u32 = 5125;
    const ARRAY_BUFFER: u32 = 34962;
    const ELEMENT_ARRAY_BUFFER: u32 = 34963;

    // Inverse-bind matrices for the skin (column-major LE).
    //
    // The AnimationSkin chunk stores each bone's frame in MODEL
    // space as 4 rows:
    //   m[0] = bone X-axis (x, y, z, 0)
    //   m[1] = bone Y-axis (x, y, z, 0)
    //   m[2] = bone Z-axis (x, y, z, 0)
    //   m[3] = bone origin (x, y, z, 0)
    // i.e. a bone-to-model (BTM) bind matrix with basis vectors as
    // rows. glTF skinning expects:
    //   - node.matrix = BTM         (bone in model space at bind)
    //   - IBM accessor = inverse(BTM)
    // For an orthonormal rotation R and origin O,
    //   inverse(BTM) = (R^T, -R^T * O).
    let mut ibm_bytes = Vec::with_capacity(asset.skin.matrices.len() * 64);
    for m in &asset.skin.matrices {
        let ibm = inverse_btm_col_major(m);
        for f in &ibm {
            ibm_bytes.extend_from_slice(&f.to_le_bytes());
        }
    }
    let ibm_view = push_view(&mut bin, &mut buffer_views, &ibm_bytes, None);
    let ibm_accessor = push_accessor(
        &mut accessors,
        ibm_view,
        FLOAT,
        asset.skin.matrices.len(),
        "MAT4",
        None,
        false,
    );

    let mut mesh_entries: Vec<Value> = Vec::new();
    for sm in &asset.submeshes {
        let mut pos_bytes = Vec::with_capacity(sm.vertices.len() * 12);
        let mut min = [f32::INFINITY; 3];
        let mut max = [f32::NEG_INFINITY; 3];
        for v in &sm.vertices {
            for c in 0..3 {
                pos_bytes.extend_from_slice(&v.position[c].to_le_bytes());
                if v.position[c] < min[c] {
                    min[c] = v.position[c];
                }
                if v.position[c] > max[c] {
                    max[c] = v.position[c];
                }
            }
        }
        let pos_view = push_view(&mut bin, &mut buffer_views, &pos_bytes, Some(ARRAY_BUFFER));
        let pos_acc = push_accessor(
            &mut accessors,
            pos_view,
            FLOAT,
            sm.vertices.len(),
            "VEC3",
            Some((min.to_vec(), max.to_vec())),
            false,
        );

        let mut uv_bytes = Vec::with_capacity(sm.vertices.len() * 8);
        for v in &sm.vertices {
            uv_bytes.extend_from_slice(&v.uv[0].to_le_bytes());
            uv_bytes.extend_from_slice(&v.uv[1].to_le_bytes());
        }
        let uv_view = push_view(&mut bin, &mut buffer_views, &uv_bytes, Some(ARRAY_BUFFER));
        let uv_acc = push_accessor(
            &mut accessors,
            uv_view,
            FLOAT,
            sm.vertices.len(),
            "VEC2",
            None,
            false,
        );

        let mut joints_bytes = Vec::with_capacity(sm.vertices.len() * 8);
        for v in &sm.vertices {
            for c in 0..4 {
                joints_bytes.extend_from_slice(&v.joints[c].to_le_bytes());
            }
        }
        let joints_view = push_view(
            &mut bin,
            &mut buffer_views,
            &joints_bytes,
            Some(ARRAY_BUFFER),
        );
        let joints_acc = push_accessor(
            &mut accessors,
            joints_view,
            USHORT,
            sm.vertices.len(),
            "VEC4",
            None,
            false,
        );

        let mut weights_bytes = Vec::with_capacity(sm.vertices.len() * 4);
        for v in &sm.vertices {
            for c in 0..4 {
                let q = (v.weights[c] * 255.0).round().clamp(0.0, 255.0) as u8;
                weights_bytes.push(q);
            }
        }
        let weights_view = push_view(
            &mut bin,
            &mut buffer_views,
            &weights_bytes,
            Some(ARRAY_BUFFER),
        );
        let weights_acc = push_accessor(
            &mut accessors,
            weights_view,
            UBYTE,
            sm.vertices.len(),
            "VEC4",
            None,
            true,
        );

        let mut idx_bytes = Vec::with_capacity(sm.triangles.len() * 12);
        for t in &sm.triangles {
            for c in 0..3 {
                idx_bytes.extend_from_slice(&t[c].to_le_bytes());
            }
        }
        let idx_view = push_view(
            &mut bin,
            &mut buffer_views,
            &idx_bytes,
            Some(ELEMENT_ARRAY_BUFFER),
        );
        let idx_acc = push_accessor(
            &mut accessors,
            idx_view,
            UINT,
            sm.triangles.len() * 3,
            "SCALAR",
            None,
            false,
        );

        let primitive = json!({
            "attributes": {
                "POSITION": pos_acc,
                "TEXCOORD_0": uv_acc,
                "JOINTS_0": joints_acc,
                "WEIGHTS_0": weights_acc,
            },
            "indices": idx_acc,
            "mode": 4,
        });
        mesh_entries.push(json!({
            "name": sm.name,
            "primitives": [primitive],
        }));
    }

    let root = 0u32;
    let armature = 1u32;
    let joints_start = 2u32;
    let joint_count = asset.skin.matrices.len() as u32;
    let submeshes_start = joints_start + joint_count;

    let mut nodes: Vec<Value> =
        Vec::with_capacity(2 + joint_count as usize + asset.submeshes.len());
    let root_children: Vec<u32> = {
        let mut c = vec![armature];
        for i in 0..asset.submeshes.len() {
            c.push(submeshes_start + i as u32);
        }
        c
    };
    nodes.push(json!({
        "name": "root",
        "children": root_children,
    }));
    nodes.push(json!({
        "name": "armature",
        "children": (0..joint_count).map(|j| joints_start + j).collect::<Vec<_>>(),
    }));
    for j in 0..joint_count {
        let m = &asset.skin.matrices[j as usize];
        // Bone-to-model transform as glTF node.matrix (column-major
        // 4x4). Basis vectors from m[0..3], origin from m[3].
        let btm = btm_col_major(m);
        nodes.push(json!({
            "name": format!("bone_{:03}", j),
            "matrix": btm.to_vec(),
        }));
    }
    let joints_array: Vec<u32> = (0..joint_count).map(|j| joints_start + j).collect();
    for (i, _) in asset.submeshes.iter().enumerate() {
        nodes.push(json!({
            "name": format!("{}_node", asset.submeshes[i].name),
            "mesh": i,
            "skin": 0,
        }));
    }

    let skins = vec![json!({
        "inverseBindMatrices": ibm_accessor,
        "joints": joints_array,
        "skeleton": armature,
    })];

    let gltf = json!({
        "asset": { "version": "2.0", "generator": "tdb-rx2 playermesh decoder" },
        "scene": 0,
        "scenes": [{ "nodes": [root] }],
        "nodes": nodes,
        "meshes": mesh_entries,
        "accessors": accessors,
        "bufferViews": buffer_views,
        "buffers": [{ "byteLength": bin.len() }],
        "skins": skins,
    });

    let json_text = serde_json::to_string(&gltf).expect("json serialization");
    let mut json_bytes = json_text.into_bytes();
    while json_bytes.len() % 4 != 0 {
        json_bytes.push(b' ');
    }
    while bin.len() % 4 != 0 {
        bin.push(0);
    }

    let total = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&json_bytes);
    out.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin);

    out
}

// ---------- Bone-matrix helpers ----------

/// Bone-to-model transform (BTM) as a 16-float column-major matrix
/// suitable for glTF's `node.matrix` field. The source stores basis
/// vectors as rows m[0..3] and the bone origin in row m[3]; the
/// homogeneous column 4 is set to (0, 0, 0, 1).
// ---------- In-place patching (PR8 write-back) ----------
//
// The PR8 serialization rule: mutate ONLY vertex positions and the
// AnimationSkin palette, byte-surgically, inside the donor's own
// container — everything else (weights, blend indices, topology,
// sub-mesh structure, materials, UVs, draw calls, metadata) stays
// byte-for-byte donor-authored BY CONSTRUCTION.

/// f32 → IEEE half, big-endian, round-to-nearest (carry into the
/// exponent on mantissa overflow is the correct behavior).
pub fn f32_to_half_be(v: f32) -> [u8; 2] {
    let bits = v.to_bits();
    let sign = (bits >> 16) & 0x8000;
    let exp = ((bits >> 23) & 0xFF) as i32;
    let man = bits & 0x7F_FFFF;
    let h: u32 = if exp == 0xFF {
        // Inf / NaN.
        sign | 0x7C00 | u32::from(man != 0)
    } else {
        let e = exp - 127 + 15;
        if e >= 0x1F {
            sign | 0x7C00 // overflow → ±Inf
        } else if e <= 0 {
            if e < -10 {
                sign // underflow → ±0
            } else {
                // Subnormal half.
                let full = man | 0x80_0000;
                let shift = (14 - e) as u32;
                let half_man = full >> shift;
                let round = (full >> (shift - 1)) & 1;
                sign | (half_man + round)
            }
        } else {
            let half_man = man >> 13;
            let round = (man >> 12) & 1;
            sign | ((((e as u32) << 10) | half_man) + round)
        }
    };
    [(h >> 8) as u8, h as u8]
}

/// One file-table chunk, for external diff tooling.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChunkInfo {
    pub index: usize,
    pub tag: u32,
    pub abs_offset: u32,
    pub size: u32,
}

/// The container's file table with absolute offsets resolved.
pub fn file_table_chunks(data: &[u8]) -> Result<Vec<ChunkInfo>, PlayerMeshError> {
    let (entries, hdr_size) = walk_file_table(data)?;
    Ok(entries
        .iter()
        .enumerate()
        .map(|(index, e)| ChunkInfo {
            index,
            tag: e.tag,
            abs_offset: e.abs_offset(hdr_size),
            size: e.size,
        })
        .collect())
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SubmeshPatchStat {
    pub name: String,
    pub position_format: &'static str,
    pub vertices_total: usize,
    pub vertices_changed: usize,
    /// Max |written-back − requested| after format quantization (mm).
    pub max_quantization_error_mm: f32,
    /// Max |requested − donor| (mm) — how far this sub-mesh moved.
    pub max_requested_delta_mm: f32,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PatchReport {
    pub submeshes: Vec<SubmeshPatchStat>,
    pub palette_bones_total: usize,
    pub palette_bones_changed: usize,
    /// Max bind-pivot move across changed palette bones (mm).
    pub max_palette_translation_mm: f32,
    pub bytes_changed: usize,
}

/// Overwrite vertex positions (per sub-mesh, parse order) and the
/// AnimationSkin palette inside `data`. Counts must match the donor
/// exactly — any mismatch is a hard error and `data` may be partially
/// written (callers keep the original buffer for recovery/diffing).
pub fn patch_player_mesh(
    data: &mut [u8],
    new_positions: &[Vec<[f32; 3]>],
    new_palette: Option<&[BoneMatrix]>,
) -> Result<PatchReport, PlayerMeshError> {
    // Re-derive the exact layout the parser uses.
    let (entries, hdr_size) = walk_file_table(data)?;
    let mut vb_payloads: Vec<(u32, u32)> = Vec::new();
    let mut vb_descs: Vec<u32> = Vec::new();
    let mut vertex_descriptors: Vec<(u32, VertexDescriptor)> = Vec::new();
    let mut mesh_count = 0usize;
    let mut name_table = NameTable::default();
    let mut skin_entry: Option<(usize, u32, u32)> = None; // (abs_off, size) + idx
    let mut skin_count = 0usize;
    for (i, e) in entries.iter().enumerate() {
        match e.tag {
            TAG_VERTEX_BUFFER => {
                if i == 0 {
                    continue;
                }
                let prev = &entries[i - 1];
                vb_payloads.push((prev.raw_offset, prev.size));
                vb_descs.push(e.raw_offset);
            }
            TAG_VERTEX_DESCRIPTOR => {
                let vd = decode_vertex_descriptor(data, e.raw_offset as usize)?;
                vertex_descriptors.push((i as u32, vd));
            }
            TAG_MESH_HANDLE => mesh_count += 1,
            TAG_NAME_TABLE => {
                name_table =
                    decode_name_table(data, e.abs_offset(hdr_size) as usize, e.size as usize)?;
            }
            TAG_ANIMATION_SKIN => {
                skin_count += 1;
                skin_entry = Some((i, e.abs_offset(hdr_size), e.size));
            }
            _ => {}
        }
    }
    if new_positions.len() != mesh_count {
        return Err(PlayerMeshError::PatchMismatch(format!(
            "donor has {mesh_count} sub-meshes, {} position sets supplied",
            new_positions.len()
        )));
    }
    let descriptor_count = vertex_descriptors.len();
    let first_vd_entry = vertex_descriptors
        .first()
        .map(|(idx, _)| *idx as i32)
        .unwrap_or(0);
    let mesh_handle_indices: Vec<u32> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| e.tag == TAG_MESH_HANDLE)
        .map(|(i, _)| i as u32)
        .take(mesh_count)
        .collect();

    let mut bytes_changed = 0usize;
    let mut submeshes = Vec::with_capacity(mesh_count);
    for i in 0..mesh_count {
        let vb_desc_off = vb_descs[i] as usize;
        let descriptor_idx = if descriptor_count < mesh_count {
            let part_id = read_i32_be(data, vb_desc_off + 40)?;
            let resolved = part_id - first_vd_entry;
            if resolved < 0 || (resolved as usize) >= descriptor_count {
                return Err(PlayerMeshError::PartIdOutOfRange {
                    sub: i,
                    part_id,
                    avail: descriptor_count,
                });
            }
            resolved as usize
        } else {
            i.min(descriptor_count.saturating_sub(1))
        };
        let vd = vertex_descriptors[descriptor_idx].1.clone();
        let v_count = read_u32_be(data, vb_desc_off + 36)? as usize;
        let vb_data_off = (vb_payloads[i].0 + hdr_size) as usize;

        if new_positions[i].len() != v_count {
            return Err(PlayerMeshError::PatchMismatch(format!(
                "sub-mesh {i}: donor has {v_count} vertices, {} positions supplied",
                new_positions[i].len()
            )));
        }

        let mut pos_off: Option<(u32, PositionFormat)> = None;
        for (off, tag) in &vd.attrs {
            if *tag == TYPE_POS_FLOAT {
                pos_off = Some((*off, PositionFormat::Float));
            } else if *tag == TYPE_POS_HALF {
                pos_off = Some((*off, PositionFormat::HalfFloat));
            }
        }
        let (p_off, p_fmt) = pos_off.ok_or(PlayerMeshError::NoPositionAttr { sub: i })?;

        let name = name_table
            .by_entry
            .get(&mesh_handle_indices.get(i).copied().unwrap_or(u32::MAX))
            .cloned()
            .unwrap_or_else(|| format!("submesh_{i}"));

        let mut changed = 0usize;
        let mut max_q = 0f32;
        let mut max_delta = 0f32;
        for (v_idx, p) in new_positions[i].iter().enumerate() {
            let o = vb_data_off + v_idx * vd.stride + p_off as usize;
            // Donor value for the delta stat.
            let donor = match p_fmt {
                PositionFormat::HalfFloat => [
                    half_be(data[o], data[o + 1]),
                    half_be(data[o + 2], data[o + 3]),
                    half_be(data[o + 4], data[o + 5]),
                ],
                PositionFormat::Float => [
                    read_f32_be(data, o)?,
                    read_f32_be(data, o + 4)?,
                    read_f32_be(data, o + 8)?,
                ],
            };
            let d =
                ((p[0] - donor[0]).powi(2) + (p[1] - donor[1]).powi(2) + (p[2] - donor[2]).powi(2))
                    .sqrt();
            max_delta = max_delta.max(d * 1000.0);

            let mut vert_changed = false;
            match p_fmt {
                PositionFormat::HalfFloat => {
                    for k in 0..3 {
                        let enc = f32_to_half_be(p[k]);
                        let dec = half_be(enc[0], enc[1]);
                        max_q = max_q.max((dec - p[k]).abs() * 1000.0);
                        let slot = o + k * 2;
                        if data[slot] != enc[0] || data[slot + 1] != enc[1] {
                            vert_changed = true;
                            bytes_changed += usize::from(data[slot] != enc[0])
                                + usize::from(data[slot + 1] != enc[1]);
                            data[slot] = enc[0];
                            data[slot + 1] = enc[1];
                        }
                    }
                }
                PositionFormat::Float => {
                    for k in 0..3 {
                        let enc = p[k].to_be_bytes();
                        let slot = o + k * 4;
                        if data[slot..slot + 4] != enc {
                            vert_changed = true;
                            bytes_changed += data[slot..slot + 4]
                                .iter()
                                .zip(&enc)
                                .filter(|(a, b)| a != b)
                                .count();
                            data[slot..slot + 4].copy_from_slice(&enc);
                        }
                    }
                }
            }
            if vert_changed {
                changed += 1;
            }
        }
        submeshes.push(SubmeshPatchStat {
            name,
            position_format: match p_fmt {
                PositionFormat::HalfFloat => "half",
                PositionFormat::Float => "float",
            },
            vertices_total: v_count,
            vertices_changed: changed,
            max_quantization_error_mm: max_q,
            max_requested_delta_mm: max_delta,
        });
    }

    // Palette.
    let mut bones_changed = 0usize;
    let mut max_translation = 0f32;
    let mut bones_total = 0usize;
    if let Some(palette) = new_palette {
        // The read path tolerates NHL 10's per-submesh palette copies (they're
        // identical), but this patches ONE chunk in place — on a multi-skin mesh
        // that would leave the other copies stale and the rig inconsistent.
        // Refuse instead of writing a half-patched file.
        if skin_count > 1 {
            return Err(PlayerMeshError::SkinCountMismatch(skin_count));
        }
        let Some((_, skin_off, skin_size)) = skin_entry else {
            return Err(PlayerMeshError::SkinCountMismatch(0));
        };
        let skin_off = skin_off as usize;
        let bone_count = read_u32_be(data, skin_off + 4)? as usize;
        bones_total = bone_count;
        if palette.len() != bone_count {
            return Err(PlayerMeshError::PatchMismatch(format!(
                "donor palette has {bone_count} bones, {} supplied",
                palette.len()
            )));
        }
        let _ = skin_size; // matrices length is validated by the reads below
        for (b, m) in palette.iter().enumerate() {
            let m_off = skin_off + 16 + b * 64;
            // Translation stat from the bind pivots.
            let mut old: BoneMatrix = [[0.0; 4]; 4];
            for row in 0..4 {
                for col in 0..4 {
                    old[row][col] = read_f32_be(data, m_off + (row * 4 + col) * 4)?;
                }
            }
            let (po, _) = crate::skeleton::palette_bind_pose(&old);
            let (pn, _) = crate::skeleton::palette_bind_pose(m);
            let t = ((pn[0] - po[0]).powi(2) + (pn[1] - po[1]).powi(2) + (pn[2] - po[2]).powi(2))
                .sqrt()
                * 1000.0;
            let mut bone_changed = false;
            for row in 0..4 {
                for col in 0..4 {
                    let enc = m[row][col].to_be_bytes();
                    let slot = m_off + (row * 4 + col) * 4;
                    if data[slot..slot + 4] != enc {
                        bone_changed = true;
                        bytes_changed += data[slot..slot + 4]
                            .iter()
                            .zip(&enc)
                            .filter(|(a, b)| a != b)
                            .count();
                        data[slot..slot + 4].copy_from_slice(&enc);
                    }
                }
            }
            if bone_changed {
                bones_changed += 1;
                max_translation = max_translation.max(t);
            }
        }
    }

    Ok(PatchReport {
        submeshes,
        palette_bones_total: bones_total,
        palette_bones_changed: bones_changed,
        max_palette_translation_mm: max_translation,
        bytes_changed,
    })
}

pub fn btm_col_major(m: &BoneMatrix) -> [f32; 16] {
    [
        m[0][0], m[0][1], m[0][2], 0.0, // col 0 = X-basis
        m[1][0], m[1][1], m[1][2], 0.0, // col 1 = Y-basis
        m[2][0], m[2][1], m[2][2], 0.0, // col 2 = Z-basis
        m[3][0], m[3][1], m[3][2], 1.0, // col 3 = origin (w=1)
    ]
}

/// Inverse of the BTM as a 16-float column-major matrix suitable for
/// the glTF `inverseBindMatrices` accessor. Assumes the basis is
/// orthonormal so the inverse is `(R^T, -R^T * O)`.
pub fn inverse_btm_col_major(m: &BoneMatrix) -> [f32; 16] {
    let (xx, xy, xz) = (m[0][0], m[0][1], m[0][2]);
    let (yx, yy, yz) = (m[1][0], m[1][1], m[1][2]);
    let (zx, zy, zz) = (m[2][0], m[2][1], m[2][2]);
    let (ox, oy, oz) = (m[3][0], m[3][1], m[3][2]);
    let nx = -(xx * ox + xy * oy + xz * oz);
    let ny = -(yx * ox + yy * oy + yz * oz);
    let nz = -(zx * ox + zy * oy + zz * oz);
    [
        xx, yx, zx, 0.0, // col 0
        xy, yy, zy, 0.0, // col 1
        xz, yz, zz, 0.0, // col 2
        nx, ny, nz, 1.0, // col 3
    ]
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;

    fn template_path() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("Working Assets")
            .join("Template Mesh")
            .join("player_20500.rx2")
    }

    fn load_template() -> Option<Vec<u8>> {
        let p = template_path();
        if !p.is_file() {
            return None;
        }
        std::fs::read(p).ok()
    }

    #[test]
    fn half_encoder_round_trips_every_finite_half() {
        // Every finite half value must encode back to its own bits.
        for h in 0u32..0x1_0000 {
            let e = (h >> 10) & 0x1F;
            if e == 31 {
                continue; // Inf/NaN — not produced by mesh data
            }
            let b0 = (h >> 8) as u8;
            let b1 = h as u8;
            let v = half_be(b0, b1);
            let enc = f32_to_half_be(v);
            assert_eq!(enc, [b0, b1], "half 0x{h:04X} ({v}) did not round-trip");
        }
    }

    #[test]
    fn half_encoder_rounds_to_nearest() {
        // 1.0 + 1.5 ulp (ulp = 2^-10 in [1,2)) must round up to
        // 1.0 + 2 ulp.
        let ulp = 2f32.powi(-10);
        let enc = f32_to_half_be(1.0 + 1.5 * ulp);
        let dec = half_be(enc[0], enc[1]);
        assert!((dec - (1.0 + 2.0 * ulp)).abs() < 1e-7, "got {dec}");
        // Just under half an ulp rounds down.
        let enc = f32_to_half_be(1.0 + 0.49 * ulp);
        let dec = half_be(enc[0], enc[1]);
        assert_eq!(dec, 1.0);
    }

    #[test]
    fn patch_round_trips_positions_and_palette_on_template() {
        let Some(data) = load_template() else {
            eprintln!("skipping: template player_20500.rx2 not present");
            return;
        };
        let donor = parse_player_mesh(&data).unwrap();

        // Move every vertex by a fixed, format-representable offset
        // and translate palette bone 16 (Jaw) by 5 mm in z.
        let new_positions: Vec<Vec<[f32; 3]>> = donor
            .submeshes
            .iter()
            .map(|sm| {
                sm.vertices
                    .iter()
                    .map(|v| [v.position[0], v.position[1], v.position[2] + 0.004])
                    .collect()
            })
            .collect();
        let mut palette = donor.skin.matrices.clone();
        // t' = t − Δ·R with Δ = (0,0,0.005).
        for k in 0..3 {
            palette[16][3][k] -= 0.005 * palette[16][2][k];
        }

        let mut patched = data.clone();
        let report = patch_player_mesh(&mut patched, &new_positions, Some(&palette)).unwrap();
        assert_eq!(report.submeshes.len(), donor.submeshes.len());
        assert_eq!(report.palette_bones_total, donor.skin.matrices.len());
        assert_eq!(report.palette_bones_changed, 1);
        assert!((report.max_palette_translation_mm - 5.0).abs() < 0.05);

        // Re-parse: counts identical, weights/joints/topology
        // byte-identical, positions at the requested values within
        // format quantization.
        let re = parse_player_mesh(&patched).unwrap();
        assert_eq!(re.submeshes.len(), donor.submeshes.len());
        assert_eq!(re.skin.matrices.len(), donor.skin.matrices.len());
        for (i, (a, b)) in donor.submeshes.iter().zip(&re.submeshes).enumerate() {
            assert_eq!(a.name, b.name);
            assert_eq!(
                a.vertices.len(),
                b.vertices.len(),
                "submesh {i} vertex count"
            );
            assert_eq!(a.triangles, b.triangles, "submesh {i} topology");
            for (va, vb) in a.vertices.iter().zip(&b.vertices) {
                assert_eq!(va.joints, vb.joints, "blend indices must not change");
                assert_eq!(va.weights, vb.weights, "weights must not change");
            }
            // Positions: z moved by ~4 mm, within half quantization
            // (~0.5 mm ulp at y≈1.8 but z≈0.1 → much finer).
            for (vb, want) in b.vertices.iter().zip(&new_positions[i]) {
                for k in 0..3 {
                    assert!(
                        (vb.position[k] - want[k]).abs() < 0.0006,
                        "submesh {i}: got {:?} want {:?}",
                        vb.position,
                        want
                    );
                }
            }
        }
        // The non-jaw palette bones are byte-identical.
        for (b, (ma, mb)) in donor
            .skin
            .matrices
            .iter()
            .zip(&re.skin.matrices)
            .enumerate()
        {
            if b == 16 {
                continue;
            }
            assert_eq!(ma, mb, "bone {b} must be untouched");
        }

        // Mismatched inputs are hard errors.
        let mut bad = data.clone();
        let short: Vec<Vec<[f32; 3]>> = vec![Vec::new(); donor.submeshes.len()];
        assert!(matches!(
            patch_player_mesh(&mut bad, &short, None),
            Err(PlayerMeshError::PatchMismatch(_))
        ));
    }

    #[test]
    fn parses_template_into_five_submeshes() {
        let Some(data) = load_template() else {
            eprintln!("skipping: template player_20500.rx2 not present");
            return;
        };
        let asset = parse_player_mesh(&data).expect("parse must succeed");
        assert_eq!(asset.submeshes.len(), 5);
        let expected = [
            "eyel_.Mesh",
            "eyer_.Mesh",
            "haircap_.Mesh",
            "head_.Mesh",
            "mouthbag_.Mesh",
        ];
        let got: Vec<&str> = asset.submeshes.iter().map(|s| s.name.as_str()).collect();
        for e in expected {
            assert!(got.contains(&e), "missing submesh {e}; got {got:?}");
        }
    }

    #[test]
    fn head_decodes_as_halffloat_position_and_ushort_uv() {
        let Some(data) = load_template() else {
            return;
        };
        let asset = parse_player_mesh(&data).unwrap();
        let head = asset
            .submeshes
            .iter()
            .find(|s| s.name == "head_.Mesh")
            .expect("head submesh present");
        assert_eq!(head.position_format, PositionFormat::HalfFloat);
        assert_eq!(head.uv_format, UvFormat::UShort);
        assert_eq!(head.vertices.len(), 2751);
        for t in &head.triangles {
            assert!((t[0] as usize) < head.vertices.len());
            assert!((t[1] as usize) < head.vertices.len());
            assert!((t[2] as usize) < head.vertices.len());
        }
        // Vertex 0: raw bytes 27 D2 3F 53 2D 57 .. → halffloat
        // (0.0305, 1.8311, 0.0834). UV bytes 9B BD 53 ED as ushort
        // (39869, 21485) → (0.6084, 0.3279).
        let v0 = &head.vertices[0];
        assert!(
            (v0.position[0] - 0.0305).abs() < 1e-3,
            "got {:?}",
            v0.position
        );
        assert!((v0.position[1] - 1.8311).abs() < 1e-3);
        assert!((v0.position[2] - 0.0834).abs() < 1e-3);
        assert!((v0.uv[0] - 0.6084).abs() < 1e-3, "uv0 got {}", v0.uv[0]);
        assert!((v0.uv[1] - 0.3279).abs() < 1e-3, "uv1 got {}", v0.uv[1]);
        // Skin weights still sum to ~1.0.
        let wsum: f32 = v0.weights.iter().sum();
        assert!((wsum - 1.0).abs() < 1e-2, "weights sum = {wsum}");
    }

    #[test]
    fn skin_palette_is_122_bones() {
        let Some(data) = load_template() else {
            return;
        };
        let asset = parse_player_mesh(&data).unwrap();
        assert_eq!(asset.skin.matrices.len(), 122);
    }

    #[test]
    fn glb_output_round_trips_through_serde_chunks() {
        let Some(data) = load_template() else {
            return;
        };
        let asset = parse_player_mesh(&data).unwrap();
        let glb = write_glb(&asset);
        assert!(glb.len() > 32);
        assert_eq!(&glb[0..4], b"glTF");
        let total = u32::from_le_bytes([glb[8], glb[9], glb[10], glb[11]]) as usize;
        assert_eq!(total, glb.len());
        let json_len = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
        let parsed: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
        assert_eq!(parsed["asset"]["version"], "2.0");
        assert_eq!(parsed["meshes"].as_array().unwrap().len(), 5);
    }
}
