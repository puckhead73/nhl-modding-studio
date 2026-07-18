//! Pack new geometry into a Legacy `.rx2` by substituting a
//! sub-mesh's vertex + index buffers and patching the descriptor
//! fields that reference their sizes/counts. Variable vertex/index
//! counts ARE supported — this is the write path for replacing donor
//! geometry wholesale (PR10), complementing
//! [`crate::playermesh::patch_player_mesh`]'s same-count in-place
//! patching (PR8).
//!
//! Resurrected from the archived v1 pipeline
//! (`_backup/mesh-import-v1/crates/tdb-rx2/src/pack.rs`), whose
//! output was validated in-game (the Lanny McDonald haircap test).
//! Two deliberate changes from v1:
//!
//! - The half-float encoder is [`crate::playermesh::f32_to_half_be`]
//!   (round-to-nearest, exhaustively tested) instead of v1's local
//!   truncating copy.
//! - **UVs are taken in STORED (Xbox) convention — no V-flip here.**
//!   The decoder ([`crate::playermesh::parse_player_mesh`]) returns
//!   UVs as stored, so donor-sourced UVs round-trip losslessly.
//!   OBJ-sourced UVs (V=0 at bottom) must be flipped by the CALLER
//!   (`[u, 1.0 - v]`) before packing. v1 flipped inside the packer,
//!   which silently double-flipped donor-sourced UVs.
//!
//! The pipeline:
//!
//! 1. [`crate::container::parse`] yields a [`Container`] that owns
//!    every chunk's bytes and the file table.
//! 2. For each sub-mesh to replace, call [`substitute_submesh`] with
//!    the new geometry + skin. It encodes a fresh VB / IB payload,
//!    writes them back via [`Container::replace_payload`], and
//!    patches the descriptor + Mesh-handle count fields inside
//!    `container.header_region`.
//! 3. [`Container::encode`] re-emits the modified file.

use thiserror::Error;

use crate::container::{Container, ContainerError, TAG_RAW_PAYLOAD};
use crate::playermesh::f32_to_half_be;

// File-table tags (interpreted as u32 BE).
const TAG_MESH_HANDLE: u32 = 0x0002_0009;
const TAG_NAME_TABLE: u32 = 0x00EC_0010;
const TAG_VERTEX_BUFFER: u32 = 0x0002_0005;
const TAG_INDEX_BUFFER: u32 = 0x0002_0007;
const TAG_FX_RENDERABLE: u32 = 0x00EF_0004;

#[derive(Debug, Error)]
pub enum PackError {
    #[error("container: {0}")]
    Container(#[from] ContainerError),
    #[error("submesh `{0}` not found in name table")]
    SubmeshNotFound(String),
    #[error("submesh `{name}` resolved to a chunk with tag 0x{tag:08X} (expected Mesh handle 0x00020009)")]
    NotMeshHandle { name: String, tag: u32 },
    #[error("mesh handle for `{name}` references VB descriptor at file-table entry {vb}, IB descriptor at {ib} — out of range")]
    DescriptorOutOfRange { name: String, vb: u32, ib: u32 },
    #[error("VB descriptor at entry {vb} has no preceding raw payload to replace")]
    NoVbPayload { vb: u32 },
    #[error("IB descriptor at entry {ib} has no preceding raw payload to replace")]
    NoIbPayload { ib: u32 },
    #[error("vertex stride {stride} not supported by current encoder (need 32, 36, or 52)")]
    UnsupportedStride { stride: u8 },
    #[error("vertex count {0} exceeds the u16 index space (0xFFFF is the strip-restart marker)")]
    TooManyVertices(usize),
    #[error("header slack exhausted adding submesh: need {need} bytes, only {have} free before hdr_size")]
    HeaderSlackExhausted { need: usize, have: usize },
}

// ---------- Encoding helpers ----------

/// Clamp `value` to `[0, 1]` and encode as a 16-bit big-endian
/// unsigned integer in `[0, 65535]`.
fn f32_to_u16_be_norm(value: f32) -> [u8; 2] {
    let clamped = value.clamp(0.0, 1.0);
    let v = (clamped * 65535.0).round() as u16;
    v.to_be_bytes()
}

/// Quantize 4 floats summing to ~1.0 into 4 u8s that **exactly** sum
/// to 255. Residual quantization error is folded into the largest
/// weight so the dominant influence absorbs the drift.
pub fn quantize_weights(weights: [f32; 4]) -> [u8; 4] {
    let s: f32 = weights.iter().sum();
    if s <= 0.0 {
        return [255, 0, 0, 0];
    }
    let scale = 255.0 / s;
    let mut q = [0u8; 4];
    let mut total: i32 = 0;
    for i in 0..4 {
        let v = (weights[i] * scale).round() as i32;
        let v = v.clamp(0, 255);
        q[i] = v as u8;
        total += v;
    }
    let drift = 255 - total;
    if drift != 0 {
        // Push the residual into the slot with the largest weight.
        let mut best = 0usize;
        for i in 1..4 {
            if q[i] > q[best] {
                best = i;
            }
        }
        let adjusted = (q[best] as i32 + drift).clamp(0, 255);
        q[best] = adjusted as u8;
    }
    q
}

/// Encode a unit normal/tangent as Legacy's packed 2:10:10:10 BE u32
/// at byte offset `off`. **Critical bit order: (Z, Y, X) — Z in the
/// high 10 bits, X in the low 10 bits.** Each component is scaled by
/// 511 and stored as 10-bit two's complement. Verified against
/// Bergeron head vert 0 (`21 1F 23 8F` → (-0.221, -0.110, -0.969))
/// and the mirror-image decoder in `playermesh`.
fn write_packed_2_10_10_10_be(buf: &mut [u8], off: usize, n: [f32; 3]) {
    let q = |v: f32| -> u32 {
        let clamped = v.clamp(-1.0, 1.0);
        let scaled = (clamped * 511.0).round() as i32;
        let clipped = scaled.clamp(-511, 511);
        (clipped as u32) & 0x3FF
    };
    let packed = (q(n[2]) << 20) | (q(n[1]) << 10) | q(n[0]);
    buf[off..off + 4].copy_from_slice(&packed.to_be_bytes());
}

/// Encode (joints, weights) pairs into the Legacy STORED influence
/// layout (PR10.1 finding — matching the donor convention exactly is
/// what makes the runtime skin the vertex):
///
/// - joints are stored dominant-FIRST; zero-weight pad slots repeat
///   the LAST active joint (donor: `[24,108,108,108]`);
/// - weight BYTES are stored in REVERSE slot order — byte 3 pairs
///   with joints[0] (donor: `[0,0,44,211]` = j24@211, j108@44);
/// - quantized weights sum to exactly 255.
///
/// Input pairing is the DECODED convention (`weights[c]` pairs with
/// `joints[c]`, any order); output is the byte-level layout.
pub fn encode_influences(joints: [u16; 4], weights: [f32; 4]) -> ([u16; 4], [u8; 4]) {
    let mut pairs: Vec<(u16, f32)> = (0..4)
        .filter(|&c| weights[c] > 0.0)
        .map(|c| (joints[c], weights[c]))
        .collect();
    if pairs.is_empty() {
        pairs.push((joints[0], 1.0));
    }
    pairs.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    pairs.truncate(4);
    let mut padded = [0f32; 4];
    for (i, p) in pairs.iter().enumerate() {
        padded[i] = p.1;
    }
    let q = quantize_weights(padded); // descending, sums to 255
    let last = pairs.last().expect("non-empty").0;
    let mut j_out = [last; 4];
    let mut w_out = [0u8; 4];
    for (i, p) in pairs.iter().enumerate() {
        j_out[i] = p.0;
        w_out[3 - i] = q[i];
    }
    (j_out, w_out)
}

// ---------- Vertex packing ----------

/// Stride-32 layout (Legacy `head_` / `eyel_` / `eyer_` /
/// `mouthbag_`):
///
/// ```text
///  0– 7: POSITION      4×fp16 BE (xyz + w=1.0 pad)
///  8–11: NORMAL        packed 2:10:10:10 BE
/// 12–15: TANGENT       packed 2:10:10:10 BE (zeroed — no source)
/// 16–19: TEXCOORD0     2×u16 BE (STORED convention — caller flips)
/// 20–27: BLENDINDICES  4×u16 BE (shader-register indices)
/// 28–31: BLENDWEIGHTS  4×u8, sum exactly 255
/// ```
fn pack_vert_stride32(
    pos: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    joints: [u16; 4],
    weights: [f32; 4],
) -> [u8; 32] {
    let mut buf = [0u8; 32];
    buf[0..2].copy_from_slice(&f32_to_half_be(pos[0]));
    buf[2..4].copy_from_slice(&f32_to_half_be(pos[1]));
    buf[4..6].copy_from_slice(&f32_to_half_be(pos[2]));
    buf[6..8].copy_from_slice(&f32_to_half_be(1.0));
    write_packed_2_10_10_10_be(&mut buf, 8, normal);
    // TANGENT @ 12 — zeros (no source; renderer tolerates it).
    buf[16..18].copy_from_slice(&f32_to_u16_be_norm(uv[0]));
    buf[18..20].copy_from_slice(&f32_to_u16_be_norm(uv[1]));
    let (j, w) = encode_influences(joints, weights);
    for i in 0..4 {
        let off = 20 + i * 2;
        buf[off..off + 2].copy_from_slice(&j[i].to_be_bytes());
    }
    buf[28..32].copy_from_slice(&w);
    buf
}

/// Stride-36 layout (Legacy `haircap_`): stride-32 + TEXCOORD1 at
/// offset 20 (UV0 duplicated — sources ship one channel).
fn pack_vert_stride36(
    pos: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    joints: [u16; 4],
    weights: [f32; 4],
) -> [u8; 36] {
    let mut buf = [0u8; 36];
    buf[0..2].copy_from_slice(&f32_to_half_be(pos[0]));
    buf[2..4].copy_from_slice(&f32_to_half_be(pos[1]));
    buf[4..6].copy_from_slice(&f32_to_half_be(pos[2]));
    buf[6..8].copy_from_slice(&f32_to_half_be(1.0));
    write_packed_2_10_10_10_be(&mut buf, 8, normal);
    // TANGENT @ 12 — zeros.
    buf[16..18].copy_from_slice(&f32_to_u16_be_norm(uv[0]));
    buf[18..20].copy_from_slice(&f32_to_u16_be_norm(uv[1]));
    buf[20..22].copy_from_slice(&f32_to_u16_be_norm(uv[0]));
    buf[22..24].copy_from_slice(&f32_to_u16_be_norm(uv[1]));
    let (j, w) = encode_influences(joints, weights);
    for i in 0..4 {
        let off = 24 + i * 2;
        buf[off..off + 2].copy_from_slice(&j[i].to_be_bytes());
    }
    buf[32..36].copy_from_slice(&w);
    buf
}

/// Stride-52 layout (*roster* `haircap_`, e.g. player_67): 3 UV
/// channels as 2×fp32 BE each (UV0 duplicated into all three).
fn pack_vert_stride52(
    pos: [f32; 3],
    normal: [f32; 3],
    uv: [f32; 2],
    joints: [u16; 4],
    weights: [f32; 4],
) -> [u8; 52] {
    let mut buf = [0u8; 52];
    buf[0..2].copy_from_slice(&f32_to_half_be(pos[0]));
    buf[2..4].copy_from_slice(&f32_to_half_be(pos[1]));
    buf[4..6].copy_from_slice(&f32_to_half_be(pos[2]));
    buf[6..8].copy_from_slice(&f32_to_half_be(1.0));
    write_packed_2_10_10_10_be(&mut buf, 8, normal);
    // TANGENT @ 12 — zeros.
    for ch in 0..3 {
        let off = 16 + ch * 8;
        buf[off..off + 4].copy_from_slice(&uv[0].to_be_bytes());
        buf[off + 4..off + 8].copy_from_slice(&uv[1].to_be_bytes());
    }
    let (j, w) = encode_influences(joints, weights);
    for i in 0..4 {
        let off = 40 + i * 2;
        buf[off..off + 2].copy_from_slice(&j[i].to_be_bytes());
    }
    buf[48..52].copy_from_slice(&w);
    buf
}

// ---------- Name-table lookup (Mesh handles) ----------

/// Resolve a sub-mesh name (e.g. `"head_.Mesh"`) to its file-table
/// entry index by walking the name table (END-POINTER semantics: each
/// record's third u32 is the absolute end offset of its string).
fn find_mesh_handle_entry(c: &Container, name: &str) -> Option<usize> {
    let nt = c.chunks.iter().find(|chunk| chunk.tag == TAG_NAME_TABLE)?;
    let bytes = &nt.bytes;
    if bytes.len() < 12 {
        return None;
    }
    let count = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let records_start = 12usize;
    let strings_start = records_start + count * 12;
    let nt_abs_off = nt.raw_offset as usize;

    let mut name_start = strings_start;
    for i in 0..count {
        let rec = records_start + i * 12;
        if rec + 12 > bytes.len() {
            break;
        }
        let entry_idx = u32::from_be_bytes([
            bytes[rec + 4],
            bytes[rec + 5],
            bytes[rec + 6],
            bytes[rec + 7],
        ]);
        let end_ptr_abs = u32::from_be_bytes([
            bytes[rec + 8],
            bytes[rec + 9],
            bytes[rec + 10],
            bytes[rec + 11],
        ]) as usize;
        // `str_off` is an absolute file offset; translate to a
        // chunk-local index.
        let end_local = end_ptr_abs.saturating_sub(nt_abs_off);
        if end_local > name_start && end_local <= bytes.len() {
            let s_end = end_local.saturating_sub(1);
            let s_end = if bytes.get(s_end) == Some(&0) {
                s_end
            } else {
                end_local
            };
            if let Ok(s) = std::str::from_utf8(&bytes[name_start..s_end]) {
                if s == name {
                    return Some(entry_idx as usize);
                }
            }
        }
        name_start = end_local;
    }
    None
}

/// All sub-mesh names present in the container's name table that
/// resolve to Mesh handles — lets a caller enumerate without guessing
/// prefixes.
pub fn list_mesh_handles(c: &Container) -> Vec<String> {
    let Some(nt) = c.chunks.iter().find(|chunk| chunk.tag == TAG_NAME_TABLE) else {
        return Vec::new();
    };
    let bytes = &nt.bytes;
    if bytes.len() < 12 {
        return Vec::new();
    }
    let count = u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) as usize;
    let records_start = 12usize;
    let strings_start = records_start + count * 12;
    let nt_abs_off = nt.raw_offset as usize;

    let mut out = Vec::new();
    let mut name_start = strings_start;
    for i in 0..count {
        let rec = records_start + i * 12;
        if rec + 12 > bytes.len() {
            break;
        }
        let entry_idx = u32::from_be_bytes([
            bytes[rec + 4],
            bytes[rec + 5],
            bytes[rec + 6],
            bytes[rec + 7],
        ]) as usize;
        let end_ptr_abs = u32::from_be_bytes([
            bytes[rec + 8],
            bytes[rec + 9],
            bytes[rec + 10],
            bytes[rec + 11],
        ]) as usize;
        let end_local = end_ptr_abs.saturating_sub(nt_abs_off);
        if end_local > name_start && end_local <= bytes.len() {
            let s_end = end_local.saturating_sub(1);
            let s_end = if bytes.get(s_end) == Some(&0) {
                s_end
            } else {
                end_local
            };
            if let Ok(s) = std::str::from_utf8(&bytes[name_start..s_end]) {
                if c.chunks.get(entry_idx).map(|ch| ch.tag) == Some(TAG_MESH_HANDLE) {
                    out.push(s.to_string());
                }
            }
        }
        name_start = end_local;
    }
    out
}

/// Buffer-descriptor facts for one sub-mesh, resolved through the
/// name table — the audit surface for verifying what a pack actually
/// patched (PR10.1 shredding investigation).
#[derive(Debug, Clone, serde::Serialize)]
pub struct SubmeshBufferInfo {
    pub name: String,
    pub mesh_handle_entry: usize,
    /// Absolute header-region offsets of the three descriptors.
    pub vb_desc_off: usize,
    pub ib_desc_off: usize,
    pub mesh_handle_off: usize,
    pub stride: u8,
    /// VB descriptor +0x24.
    pub vert_count: u32,
    /// IB descriptor +0x20.
    pub idx_count: u32,
    /// Mesh handle +0x04 / +0x0C (must agree with the above).
    pub handle_vert_count: u32,
    pub handle_idx_count: u32,
}

/// Resolve a sub-mesh's buffer descriptors + count fields.
pub fn submesh_buffer_info(c: &Container, name: &str) -> Option<SubmeshBufferInfo> {
    let mesh_entry_idx = find_mesh_handle_entry(c, name)?;
    let mesh_chunk = c.chunks.get(mesh_entry_idx)?;
    if mesh_chunk.tag != TAG_MESH_HANDLE {
        return None;
    }
    let ib_entry = u32::from_be_bytes([
        mesh_chunk.bytes[0x10],
        mesh_chunk.bytes[0x11],
        mesh_chunk.bytes[0x12],
        mesh_chunk.bytes[0x13],
    ]) as usize;
    let vb_entry = u32::from_be_bytes([
        mesh_chunk.bytes[0x14],
        mesh_chunk.bytes[0x15],
        mesh_chunk.bytes[0x16],
        mesh_chunk.bytes[0x17],
    ]) as usize;
    if vb_entry >= c.chunks.len() || ib_entry >= c.chunks.len() {
        return None;
    }
    let vb_desc_off = c.chunks[vb_entry].raw_offset as usize;
    let ib_desc_off = c.chunks[ib_entry].raw_offset as usize;
    let mesh_handle_off = c.chunks[mesh_entry_idx].raw_offset as usize;
    let h = &c.header_region;
    Some(SubmeshBufferInfo {
        name: name.to_string(),
        mesh_handle_entry: mesh_entry_idx,
        vb_desc_off,
        ib_desc_off,
        mesh_handle_off,
        stride: u16::from_be_bytes([h[vb_desc_off + 0x20], h[vb_desc_off + 0x21]]) as u8,
        vert_count: read_u32_be(h, vb_desc_off + 0x24),
        idx_count: read_u32_be(h, ib_desc_off + 0x20),
        handle_vert_count: read_u32_be(h, mesh_handle_off + 0x04),
        handle_idx_count: read_u32_be(h, mesh_handle_off + 0x0C),
    })
}

/// One stale-value hit from [`scan_stale_values`].
#[derive(Debug, Clone, serde::Serialize)]
pub struct StaleHit {
    /// Absolute header-region offset of the value.
    pub offset: usize,
    /// File-table entry whose descriptor bytes contain the offset
    /// (None when it falls outside every known descriptor range).
    pub chunk_entry: Option<usize>,
    pub chunk_tag: Option<u32>,
    /// Offset within that chunk's bytes.
    pub chunk_rel: Option<usize>,
    /// "u32" or "u16".
    pub width: &'static str,
    pub value: u32,
}

/// PR10.1 shredding probe: find every place in the PACKED header
/// region that still encodes one of the donor's OLD count/size values
/// (BE u32 + BE u16). The fields the packer patched will show NEW
/// values; any surviving OLD value is a candidate un-patched field
/// (e.g. a skin-batch vertex count the runtime trusts).
///
/// Only offsets where the DONOR header carried the same value are
/// reported (same-offset comparison is valid — substitution never
/// changes the header size or layout).
pub fn scan_stale_values(
    donor_header: &[u8],
    packed_header: &[u8],
    old_values: &[u32],
) -> Vec<StaleHit> {
    let mut hits = Vec::new();
    if donor_header.len() != packed_header.len() {
        return hits;
    }
    let n = donor_header.len();
    for &val in old_values {
        if val == 0 {
            continue; // zero matches everywhere — useless signal
        }
        let v32 = val.to_be_bytes();
        for off in 0..n.saturating_sub(3) {
            if donor_header[off..off + 4] == v32 && packed_header[off..off + 4] == v32 {
                hits.push(StaleHit {
                    offset: off,
                    chunk_entry: None,
                    chunk_tag: None,
                    chunk_rel: None,
                    width: "u32",
                    value: val,
                });
            }
        }
        if val <= u16::MAX as u32 {
            let v16 = (val as u16).to_be_bytes();
            for off in 0..n.saturating_sub(1) {
                if donor_header[off..off + 2] == v16 && packed_header[off..off + 2] == v16 {
                    // Skip offsets already covered by a u32 hit at
                    // off-2 (the low half of the same field).
                    hits.push(StaleHit {
                        offset: off,
                        chunk_entry: None,
                        chunk_tag: None,
                        chunk_rel: None,
                        width: "u16",
                        value: val,
                    });
                }
            }
        }
    }
    hits
}

/// Attribute each stale hit to the descriptor chunk whose
/// header-region byte range contains it.
pub fn attribute_hits(c: &Container, hits: &mut [StaleHit]) {
    for h in hits.iter_mut() {
        for (i, chunk) in c.chunks.iter().enumerate() {
            if chunk.tag == TAG_RAW_PAYLOAD {
                continue;
            }
            let start = chunk.raw_offset as usize;
            let end = start + chunk.size as usize;
            if h.offset >= start && h.offset < end {
                h.chunk_entry = Some(i);
                h.chunk_tag = Some(chunk.tag);
                h.chunk_rel = Some(h.offset - start);
                break;
            }
        }
    }
}

// ---------- Header-region helpers ----------

fn write_u32_be(region: &mut [u8], offset: usize, value: u32) {
    region[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
}

fn read_u32_be(region: &[u8], offset: usize) -> u32 {
    u32::from_be_bytes([
        region[offset],
        region[offset + 1],
        region[offset + 2],
        region[offset + 3],
    ])
}

// ---------- Public substitution API ----------

/// Replace a sub-mesh's geometry + skin in `container`. Variable
/// vertex/triangle counts are supported.
///
/// `positions`, `normals`, `uvs`, `joints`, `weights` are per-vertex
/// with `vert_count = positions.len()`. Pass an empty `normals` slice
/// to leave normal slots zero. **UVs must already be in the STORED
/// (Xbox, V=0 at top) convention** — flip OBJ-sourced UVs
/// (`[u, 1.0 - v]`) before calling. `triangles` is a list of 0-based
/// vertex-index triples; the encoder emits them as 1-tri triangle
/// strips with `0xFFFF` restart markers, matching the source-template
/// convention (player_20500, player_67).
#[allow(clippy::too_many_arguments)]
pub fn substitute_submesh(
    container: &mut Container,
    submesh_name: &str,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    triangles: &[[u32; 3]],
) -> Result<SubstituteReport, PackError> {
    if positions.len() >= 0xFFFF {
        return Err(PackError::TooManyVertices(positions.len()));
    }
    let mesh_entry_idx = find_mesh_handle_entry(container, submesh_name)
        .ok_or_else(|| PackError::SubmeshNotFound(submesh_name.into()))?;
    let mesh_chunk = container
        .chunks
        .get(mesh_entry_idx)
        .ok_or_else(|| PackError::SubmeshNotFound(submesh_name.into()))?;
    if mesh_chunk.tag != TAG_MESH_HANDLE {
        return Err(PackError::NotMeshHandle {
            name: submesh_name.into(),
            tag: mesh_chunk.tag,
        });
    }
    // Mesh handle: +0x10 u32 = IB descriptor entry, +0x14 = VB entry.
    let ib_entry = u32::from_be_bytes([
        mesh_chunk.bytes[0x10],
        mesh_chunk.bytes[0x11],
        mesh_chunk.bytes[0x12],
        mesh_chunk.bytes[0x13],
    ]);
    let vb_entry = u32::from_be_bytes([
        mesh_chunk.bytes[0x14],
        mesh_chunk.bytes[0x15],
        mesh_chunk.bytes[0x16],
        mesh_chunk.bytes[0x17],
    ]);
    if vb_entry as usize >= container.chunks.len() || ib_entry as usize >= container.chunks.len() {
        return Err(PackError::DescriptorOutOfRange {
            name: submesh_name.into(),
            vb: vb_entry,
            ib: ib_entry,
        });
    }

    let vb_payload_idx = find_preceding_raw(&container.chunks, vb_entry as usize)
        .ok_or(PackError::NoVbPayload { vb: vb_entry })?;
    let ib_payload_idx = find_preceding_raw(&container.chunks, ib_entry as usize)
        .ok_or(PackError::NoIbPayload { ib: ib_entry })?;

    let vb_desc_abs = container.chunks[vb_entry as usize].raw_offset as usize;
    let ib_desc_abs = container.chunks[ib_entry as usize].raw_offset as usize;
    let mesh_handle_abs = container.chunks[mesh_entry_idx].raw_offset as usize;

    // Existing vertex stride (u16 BE at VB descriptor +0x20) — the
    // new geometry re-packs at this stride so the VertexDescriptor
    // still applies.
    let stride = u16::from_be_bytes([
        container.header_region[vb_desc_abs + 0x20],
        container.header_region[vb_desc_abs + 0x21],
    ]) as u8;

    // New vertex buffer payload.
    let vert_count = positions.len();
    let mut vb_payload: Vec<u8> = Vec::with_capacity(vert_count * stride as usize);
    for v_idx in 0..vert_count {
        let p = positions[v_idx];
        let n = normals.get(v_idx).copied().unwrap_or([0.0, 0.0, 1.0]);
        let u = uvs.get(v_idx).copied().unwrap_or([0.0, 0.0]);
        let j = joints.get(v_idx).copied().unwrap_or([0; 4]);
        let w = weights.get(v_idx).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]);
        match stride {
            32 => vb_payload.extend_from_slice(&pack_vert_stride32(p, n, u, j, w)),
            36 => vb_payload.extend_from_slice(&pack_vert_stride36(p, n, u, j, w)),
            52 => vb_payload.extend_from_slice(&pack_vert_stride52(p, n, u, j, w)),
            other => return Err(PackError::UnsupportedStride { stride: other }),
        }
    }

    // New index buffer: u16 BE 1-tri strips with 0xFFFF restarts —
    // `(a, b, c, FFFF, d, e, f, FFFF, …)`, trailing FFFF omitted.
    let mut ib_payload: Vec<u8> = Vec::with_capacity(triangles.len() * 8);
    for (i, t) in triangles.iter().enumerate() {
        for c in 0..3 {
            ib_payload.extend_from_slice(&(t[c] as u16).to_be_bytes());
        }
        if i + 1 < triangles.len() {
            ib_payload.extend_from_slice(&0xFFFFu16.to_be_bytes());
        }
    }
    let idx_count = (ib_payload.len() / 2) as u32;

    // Pad both raw payloads to a 16-byte boundary (every source
    // payload is 16-aligned; the runtime assumes it). The descriptor
    // size field uses the padded length; vert_count / idx_count stay
    // at the real counts so the loader only iterates real data.
    pad_to_16(&mut vb_payload);
    pad_to_16(&mut ib_payload);

    let new_vb_size = vb_payload.len() as u32;
    let new_ib_size = ib_payload.len() as u32;

    // Patch the header-region descriptor fields BEFORE the payload
    // swap (the header region is preserved by the container layer).
    let old_vb_size_encoded = read_u32_be(&container.header_region, vb_desc_abs + 0x1C);
    // VB descriptor +0x1C: high byte 0x10 flag, low 24 bits = size+2.
    let vb_size_encoded: u32 = 0x1000_0000u32 | ((new_vb_size + 2) & 0x00FF_FFFF);
    write_u32_be(
        &mut container.header_region,
        vb_desc_abs + 0x1C,
        vb_size_encoded,
    );
    // VB descriptor +0x24: vertex count.
    write_u32_be(
        &mut container.header_region,
        vb_desc_abs + 0x24,
        vert_count as u32,
    );
    // IB descriptor +0x1C: buffer byte size; +0x20: index count.
    write_u32_be(
        &mut container.header_region,
        ib_desc_abs + 0x1C,
        new_ib_size,
    );
    write_u32_be(&mut container.header_region, ib_desc_abs + 0x20, idx_count);
    // Mesh handle +0x04: vertex count; +0x0C: index count.
    write_u32_be(
        &mut container.header_region,
        mesh_handle_abs + 0x04,
        vert_count as u32,
    );
    write_u32_be(
        &mut container.header_region,
        mesh_handle_abs + 0x0C,
        idx_count,
    );

    let old_vb_size = container.chunks[vb_payload_idx].size;
    let old_ib_size = container.chunks[ib_payload_idx].size;
    container.replace_payload(vb_payload_idx, vb_payload)?;
    container.replace_payload(ib_payload_idx, ib_payload)?;

    Ok(SubstituteReport {
        submesh: submesh_name.to_string(),
        stride,
        old_vb_size,
        new_vb_size,
        old_ib_size,
        new_ib_size,
        old_vb_size_encoded,
        new_vb_size_encoded: vb_size_encoded,
        vert_count: vert_count as u32,
        idx_count,
    })
}

/// Add a brand-new 6th sub-mesh to a player `.rx2` (e.g. the NHL24
/// eyelashes, which blink because they ride the eyelid bones). The
/// container only had `substitute_submesh` (replace in place); this is
/// the first op that GROWS the submesh set.
///
/// Approach (no header-region growth, no name-table rebuild — the
/// parser names a nameless handle `mesh_N`):
/// - clone the VertexDescriptor/stride + VB-desc + IB-desc + Mesh-handle
///   layout from `clone_from` (must be the same stride as the new
///   geometry, e.g. `head_` stride-32);
/// - clone the (transparent) material chunk from `material_from`
///   (e.g. `haircap_`, transparency=1) so the lashes alpha-blend;
/// - encode the new VB/IB payloads, append them + the cloned-and-patched
///   descriptors as 6 new file-table entries, drop the descriptors into
///   the free header slack after the (grown) file table, bump file_count.
///
/// Returns the new submesh's mesh-handle file-table entry index.
#[allow(clippy::too_many_arguments)]
pub fn add_submesh(
    container: &mut Container,
    clone_from: &str,
    material_from: &str,
    new_name: &str,
    positions: &[[f32; 3]],
    normals: &[[f32; 3]],
    uvs: &[[f32; 2]],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    triangles: &[[u32; 3]],
) -> Result<AddSubmeshReport, PackError> {
    if positions.len() >= 0xFFFF {
        return Err(PackError::TooManyVertices(positions.len()));
    }
    // ---- Resolve the clone-from submesh's descriptors. ----
    let src_mesh_idx = find_mesh_handle_entry(container, clone_from)
        .ok_or_else(|| PackError::SubmeshNotFound(clone_from.into()))?;
    let src_handle = &container.chunks[src_mesh_idx];
    let vb_entry = read_u32_be(&src_handle.bytes, 0x14) as usize;
    let ib_entry = read_u32_be(&src_handle.bytes, 0x10) as usize;
    let vb_desc_src = container.chunks[vb_entry].clone();
    let ib_desc_src = container.chunks[ib_entry].clone();
    let handle_src = src_handle.clone();
    let stride = u16::from_be_bytes([vb_desc_src.bytes[0x20], vb_desc_src.bytes[0x21]]) as u8;

    // ---- Resolve the material to clone (transparent). ----
    let mat_mesh_idx = find_mesh_handle_entry(container, material_from)
        .ok_or_else(|| PackError::SubmeshNotFound(material_from.into()))?;
    let mat_src = container
        .chunks
        .iter()
        .find(|c| {
            c.tag == TAG_FX_RENDERABLE && read_u32_be(&c.bytes, 0x00) as usize == mat_mesh_idx
        })
        .cloned()
        .ok_or_else(|| PackError::SubmeshNotFound(format!("{material_from} material")))?;

    // ---- Encode the new VB/IB payloads. ----
    let vert_count = positions.len();
    let mut vb_payload: Vec<u8> = Vec::with_capacity(vert_count * stride as usize);
    for v in 0..vert_count {
        let p = positions[v];
        let n = normals.get(v).copied().unwrap_or([0.0, 0.0, 1.0]);
        let u = uvs.get(v).copied().unwrap_or([0.0, 0.0]);
        let j = joints.get(v).copied().unwrap_or([0; 4]);
        let w = weights.get(v).copied().unwrap_or([1.0, 0.0, 0.0, 0.0]);
        match stride {
            32 => vb_payload.extend_from_slice(&pack_vert_stride32(p, n, u, j, w)),
            36 => vb_payload.extend_from_slice(&pack_vert_stride36(p, n, u, j, w)),
            52 => vb_payload.extend_from_slice(&pack_vert_stride52(p, n, u, j, w)),
            other => return Err(PackError::UnsupportedStride { stride: other }),
        }
    }
    let mut ib_payload: Vec<u8> = Vec::with_capacity(triangles.len() * 8);
    for (i, t) in triangles.iter().enumerate() {
        for c in 0..3 {
            ib_payload.extend_from_slice(&(t[c] as u16).to_be_bytes());
        }
        if i + 1 < triangles.len() {
            ib_payload.extend_from_slice(&0xFFFFu16.to_be_bytes());
        }
    }
    let idx_count = (ib_payload.len() / 2) as u32;
    pad_to_16(&mut vb_payload);
    pad_to_16(&mut ib_payload);
    let vb_size = vb_payload.len() as u32;
    let ib_size = ib_payload.len() as u32;

    // ---- New file-table entry indices (appended). ----
    let c0 = container.chunks.len();
    let vb_raw_i = c0;
    let vb_desc_i = c0 + 1;
    let ib_raw_i = c0 + 2;
    let ib_desc_i = c0 + 3;
    let handle_i = c0 + 4;
    let mat_i = c0 + 5;
    let new_count = c0 + 6;

    // ---- New buffer-binding indices (count existing). ----
    let vb_index = container
        .chunks
        .iter()
        .filter(|c| c.tag == TAG_VERTEX_BUFFER)
        .count() as u32;
    let ib_index = container
        .chunks
        .iter()
        .filter(|c| c.tag == TAG_INDEX_BUFFER)
        .count() as u32;

    // ---- Patch cloned descriptors. ----
    let mut vb_desc = vb_desc_src.bytes.clone();
    write_u32_be(
        &mut vb_desc,
        0x1C,
        0x1000_0000u32 | ((vb_size + 2) & 0x00FF_FFFF),
    );
    write_u32_be(&mut vb_desc, 0x24, vert_count as u32);
    write_u32_be(&mut vb_desc, 0x18, (vb_index << 3) | 0x0B);
    let mut ib_desc = ib_desc_src.bytes.clone();
    write_u32_be(&mut ib_desc, 0x1C, ib_size);
    write_u32_be(&mut ib_desc, 0x20, idx_count);
    write_u32_be(&mut ib_desc, 0x18, ib_index * 2 + 12);
    let mut handle = handle_src.bytes.clone();
    write_u32_be(&mut handle, 0x04, vert_count as u32);
    write_u32_be(&mut handle, 0x0C, idx_count);
    write_u32_be(&mut handle, 0x10, ib_desc_i as u32);
    write_u32_be(&mut handle, 0x14, vb_desc_i as u32);
    let mut material = mat_src.bytes.clone();
    write_u32_be(&mut material, 0x00, handle_i as u32);
    // Name at +0x0C (NUL-terminated), within the cloned chunk's bounds.
    {
        let name_off = 0x0C;
        let avail = material.len().saturating_sub(name_off);
        let nb = new_name.as_bytes();
        let n = nb.len().min(avail.saturating_sub(1));
        for b in material[name_off..].iter_mut() {
            *b = 0;
        }
        material[name_off..name_off + n].copy_from_slice(&nb[..n]);
    }

    // ---- Place the 4 descriptor blobs in the header slack. ----
    let align16 = |x: usize| (x + 15) & !15;
    let table_end = container.file_table_off as usize + new_count * 24;
    let mut off = align16(table_end);
    let put = |hdr: &mut Vec<u8>, off: &mut usize, bytes: &[u8]| -> Result<u32, PackError> {
        let start = *off;
        if start + bytes.len() > hdr.len() {
            return Err(PackError::HeaderSlackExhausted {
                need: bytes.len(),
                have: hdr.len().saturating_sub(start),
            });
        }
        hdr[start..start + bytes.len()].copy_from_slice(bytes);
        *off = align16(start + bytes.len());
        Ok(start as u32)
    };
    let vb_desc_off = put(&mut container.header_region, &mut off, &vb_desc)?;
    let ib_desc_off = put(&mut container.header_region, &mut off, &ib_desc)?;
    let handle_off = put(&mut container.header_region, &mut off, &handle)?;
    let mat_off = put(&mut container.header_region, &mut off, &material)?;

    // ---- Append the 6 file-table entries (order = adjacency). ----
    use crate::container::ChunkRef;
    let raw_align =
        container.chunks[find_preceding_raw(&container.chunks, vb_entry).unwrap()].align;
    let raw_kind =
        container.chunks[find_preceding_raw(&container.chunks, vb_entry).unwrap()].kind_idx;
    container.chunks.push(ChunkRef {
        raw_offset: 0,
        size: vb_size,
        align: raw_align,
        kind_idx: raw_kind,
        tag: TAG_RAW_PAYLOAD,
        bytes: vb_payload,
    });
    container.chunks.push(ChunkRef {
        raw_offset: vb_desc_off,
        size: vb_desc.len() as u32,
        align: vb_desc_src.align,
        kind_idx: vb_desc_src.kind_idx,
        tag: TAG_VERTEX_BUFFER,
        bytes: vb_desc,
    });
    container.chunks.push(ChunkRef {
        raw_offset: 0,
        size: ib_size,
        align: raw_align,
        kind_idx: raw_kind,
        tag: TAG_RAW_PAYLOAD,
        bytes: ib_payload,
    });
    container.chunks.push(ChunkRef {
        raw_offset: ib_desc_off,
        size: ib_desc.len() as u32,
        align: ib_desc_src.align,
        kind_idx: ib_desc_src.kind_idx,
        tag: TAG_INDEX_BUFFER,
        bytes: ib_desc,
    });
    container.chunks.push(ChunkRef {
        raw_offset: handle_off,
        size: handle.len() as u32,
        align: handle_src.align,
        kind_idx: handle_src.kind_idx,
        tag: TAG_MESH_HANDLE,
        bytes: handle,
    });
    container.chunks.push(ChunkRef {
        raw_offset: mat_off,
        size: material.len() as u32,
        align: mat_src.align,
        kind_idx: mat_src.kind_idx,
        tag: TAG_FX_RENDERABLE,
        bytes: material,
    });
    let _ = (vb_raw_i, ib_raw_i, mat_i);

    // ---- Bump file_count (encoder copies header_region verbatim). ----
    write_u32_be(&mut container.header_region, 0x20, new_count as u32);

    // ---- Re-flow raw payload offsets + update data_size. ----
    let mut running = 0u32;
    for c in container.chunks.iter_mut() {
        if c.tag == TAG_RAW_PAYLOAD {
            c.raw_offset = running;
            running += c.size;
        }
    }
    container.data_size = running;

    Ok(AddSubmeshReport {
        submesh: new_name.to_string(),
        mesh_entry: handle_i,
        stride,
        vert_count: vert_count as u32,
        idx_count,
        new_chunk_count: new_count,
    })
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct AddSubmeshReport {
    pub submesh: String,
    pub mesh_entry: usize,
    pub stride: u8,
    pub vert_count: u32,
    pub idx_count: u32,
    pub new_chunk_count: usize,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct SubstituteReport {
    pub submesh: String,
    pub stride: u8,
    pub old_vb_size: u32,
    pub new_vb_size: u32,
    pub old_ib_size: u32,
    pub new_ib_size: u32,
    pub old_vb_size_encoded: u32,
    pub new_vb_size_encoded: u32,
    pub vert_count: u32,
    pub idx_count: u32,
}

fn find_preceding_raw(chunks: &[crate::container::ChunkRef], idx: usize) -> Option<usize> {
    (0..idx).rev().find(|&j| chunks[j].tag == TAG_RAW_PAYLOAD)
}

fn pad_to_16(buf: &mut Vec<u8>) {
    while buf.len() % 16 != 0 {
        buf.push(0);
    }
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container::parse;
    use std::path::Path;

    type tdb_rx2_vertex = crate::playermesh::Vertex;

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
    fn packed_normal_encodes_to_bergeron_vert0_bytes() {
        // Bergeron head vert 0 normal bytes are 21 1F 23 8F →
        // (-0.221, -0.110, -0.969); encoding back must reproduce them.
        let mut buf = [0u8; 4];
        write_packed_2_10_10_10_be(&mut buf, 0, [-0.221, -0.110, -0.969]);
        assert_eq!(buf, [0x21, 0x1F, 0x23, 0x8F]);
    }

    #[test]
    fn encode_influences_matches_donor_convention() {
        // Rigid: dominant joint everywhere, weight byte in slot 3.
        let (j, w) = encode_influences([24, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]);
        assert_eq!(j, [24, 24, 24, 24]);
        assert_eq!(w, [0, 0, 0, 255]);
        // Two influences — donor pattern `[24,108,108,108]/[0,0,·,·]`
        // with the DOMINANT byte LAST.
        let (j, w) = encode_influences([24, 108, 0, 0], [0.83, 0.17, 0.0, 0.0]);
        assert_eq!(j, [24, 108, 108, 108]);
        assert_eq!(w[0], 0);
        assert_eq!(w[1], 0);
        assert!(w[3] > w[2], "dominant weight must land in byte 3");
        assert_eq!(w.iter().map(|&b| b as u32).sum::<u32>(), 255);
        // Four influences, unsorted input — joints dominant-first,
        // bytes reversed.
        let (j, w) = encode_influences([51, 48, 57, 54], [0.02, 0.45, 0.18, 0.35]);
        assert_eq!(j, [48, 54, 57, 51]);
        assert!(w[3] >= w[2] && w[2] >= w[1] && w[1] >= w[0]);
        assert_eq!(w.iter().map(|&b| b as u32).sum::<u32>(), 255);
        // Round trip through a packed stride-32 vertex + the decoder
        // pairing (weights byte 3-c pairs joints[c]).
        let buf = pack_vert_stride32(
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [0.0, 0.0],
            [24, 108, 0, 0],
            [0.83, 0.17, 0.0, 0.0],
        );
        let j0 = u16::from_be_bytes([buf[20], buf[21]]);
        let w0 = buf[28 + 3]; // reversed: byte 3 pairs slot 0
        assert_eq!(j0, 24);
        assert!((w0 as f32 / 255.0 - 0.83).abs() < 0.01);
    }

    #[test]
    fn quantize_weights_sums_to_255() {
        for src in [
            [1.0, 0.0, 0.0, 0.0],
            [0.5, 0.5, 0.0, 0.0],
            [0.25, 0.25, 0.25, 0.25],
            [0.6, 0.2, 0.1, 0.1],
            [0.0, 0.0, 0.173, 0.827],
        ] {
            let q = quantize_weights(src);
            let total: u32 = q.iter().map(|b| *b as u32).sum();
            assert_eq!(total, 255, "{src:?} -> {q:?}");
        }
    }

    #[test]
    fn uvs_are_not_flipped_by_the_packer() {
        // The decoder returns stored UVs; packing them back must be
        // identity (the v1 packer's hidden V-flip is gone).
        let buf = pack_vert_stride32(
            [0.0; 3],
            [0.0, 0.0, 1.0],
            [0.25, 0.75],
            [0; 4],
            [1.0, 0.0, 0.0, 0.0],
        );
        let u = u16::from_be_bytes([buf[16], buf[17]]) as f32 / 65535.0;
        let v = u16::from_be_bytes([buf[18], buf[19]]) as f32 / 65535.0;
        assert!((u - 0.25).abs() < 1e-4);
        assert!(
            (v - 0.75).abs() < 1e-4,
            "v={v} — V must NOT be flipped here"
        );
    }

    #[test]
    fn template_lists_all_five_mesh_handles() {
        let Some(data) = load_template() else {
            eprintln!("skipping: template player_20500.rx2 not present");
            return;
        };
        let c = parse(&data).expect("parse");
        let names = list_mesh_handles(&c);
        for want in [
            "eyel_.Mesh",
            "eyer_.Mesh",
            "haircap_.Mesh",
            "head_.Mesh",
            "mouthbag_.Mesh",
        ] {
            assert!(
                names.iter().any(|n| n == want),
                "missing {want} in {names:?}"
            );
        }
    }

    #[test]
    fn substitute_head_round_trips_geometry_weights_and_uvs() {
        let Some(data) = load_template() else {
            eprintln!("skipping: template player_20500.rx2 not present");
            return;
        };
        let mut c = parse(&data).expect("parse");
        // The template's own head geometry as a substitute — every
        // decoded field must survive the encode→decode round trip.
        let asset = crate::playermesh::parse_player_mesh(&data).unwrap();
        let head = asset
            .submeshes
            .iter()
            .find(|s| s.name == "head_.Mesh")
            .unwrap();
        let positions: Vec<[f32; 3]> = head.vertices.iter().map(|v| v.position).collect();
        let normals: Vec<[f32; 3]> = head.vertices.iter().map(|v| v.normal).collect();
        let uvs: Vec<[f32; 2]> = head.vertices.iter().map(|v| v.uv).collect();
        let joints: Vec<[u16; 4]> = head.vertices.iter().map(|v| v.joints).collect();
        let weights: Vec<[f32; 4]> = head.vertices.iter().map(|v| v.weights).collect();
        let report = substitute_submesh(
            &mut c,
            "head_.Mesh",
            &positions,
            &normals,
            &uvs,
            &joints,
            &weights,
            &head.triangles,
        )
        .expect("substitute");
        assert_eq!(report.stride, 32);
        assert_eq!(report.vert_count as usize, head.vertices.len());
        // Strip-with-restart: 3 indices + 1 FFFF per tri, minus the
        // last trailing FFFF; padded to 16.
        let raw_strip_bytes = (head.triangles.len() * 4 - 1) as u32 * 2;
        assert_eq!(report.new_ib_size, (raw_strip_bytes + 15) & !15);

        let new_bytes = c.encode();
        let asset2 = crate::playermesh::parse_player_mesh(&new_bytes).expect("re-decode");
        assert_eq!(asset2.submeshes.len(), asset.submeshes.len());
        let head2 = asset2
            .submeshes
            .iter()
            .find(|s| s.name == "head_.Mesh")
            .unwrap();
        assert_eq!(head2.vertices.len(), head.vertices.len());
        assert_eq!(head2.triangles.len(), head.triangles.len());
        // Field-level round trip within quantization floors. Joints/
        // weights compare as ACTIVE influence sets — the encoder
        // normalizes pad slots (repeat-last) and sorts dominant-first,
        // which is decode-equivalent but not byte-identical to every
        // donor pad pattern.
        let actives = |v: &tdb_rx2_vertex| -> std::collections::BTreeMap<u16, f32> {
            let mut m = std::collections::BTreeMap::new();
            for c in 0..4 {
                if v.weights[c] > 0.0 {
                    *m.entry(v.joints[c]).or_insert(0.0) += v.weights[c];
                }
            }
            m
        };
        for (v1, v2) in head.vertices.iter().zip(&head2.vertices) {
            for k in 0..3 {
                assert!(
                    (v1.position[k] - v2.position[k]).abs() < 0.001,
                    "position drift"
                );
                assert!((v1.normal[k] - v2.normal[k]).abs() < 0.005, "normal drift");
            }
            for k in 0..2 {
                assert!(
                    (v1.uv[k] - v2.uv[k]).abs() < 1.0 / 65535.0 + 1e-6,
                    "uv drift"
                );
            }
            let (a, b) = (actives(v1), actives(v2));
            assert_eq!(
                a.keys().collect::<Vec<_>>(),
                b.keys().collect::<Vec<_>>(),
                "active joint sets must match"
            );
            for (j, w) in &a {
                assert!(
                    (w - b[j]).abs() <= 2.0 / 255.0,
                    "weight drift beyond requantization on joint {j}"
                );
            }
        }
        // The untouched sub-meshes must be byte-equivalent in decode.
        for (s1, s2) in asset.submeshes.iter().zip(&asset2.submeshes) {
            if s1.name == "head_.Mesh" {
                continue;
            }
            assert_eq!(s1.vertices.len(), s2.vertices.len(), "{}", s1.name);
            assert_eq!(s1.triangles, s2.triangles, "{}", s1.name);
        }
    }

    #[test]
    fn substitute_with_different_counts_grows_and_reparses() {
        let Some(data) = load_template() else {
            return;
        };
        let mut c = parse(&data).expect("parse");
        // A tiny 4-vert / 2-tri quad replaces the head — the loader
        // counts and buffer sizes must all follow.
        let positions = [
            [0.0, 1.8, 0.1],
            [0.05, 1.8, 0.1],
            [0.05, 1.85, 0.1],
            [0.0, 1.85, 0.1],
        ];
        let normals = [[0.0, 0.0, 1.0]; 4];
        let uvs = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
        let joints = [[24u16, 0, 0, 0]; 4];
        let weights = [[1.0f32, 0.0, 0.0, 0.0]; 4];
        let triangles = [[0u32, 1, 2], [0, 2, 3]];
        let report = substitute_submesh(
            &mut c,
            "head_.Mesh",
            &positions,
            &normals,
            &uvs,
            &joints,
            &weights,
            &triangles,
        )
        .expect("substitute");
        assert_eq!(report.vert_count, 4);
        assert_eq!(report.idx_count, 7); // 3 + 1 + 3
        let new_bytes = c.encode();
        let asset2 = crate::playermesh::parse_player_mesh(&new_bytes).expect("re-decode");
        let head2 = asset2
            .submeshes
            .iter()
            .find(|s| s.name == "head_.Mesh")
            .unwrap();
        assert_eq!(head2.vertices.len(), 4);
        assert_eq!(head2.triangles.len(), 2);
        // Rigid vert in donor convention: joints repeat the active
        // bone; decoded dominant weight lands in slot 0.
        assert_eq!(head2.vertices[0].joints, [24, 24, 24, 24]);
        assert_eq!(head2.vertices[0].weights[0], 1.0);
    }
}
