//! EA RX2 mesh container reader.
//!
//! Port of Heck DB Editor v1.2's `_parse_rx2` + `_strip_to_triangles`
//! + `_half_be` + `_compute_smooth_normals` + `_compute_tangents`
//! (`eq_3d_helpers.py`).
//!
//! Reads positions, optional UV1, optional UV2, and triangle indices
//! from one or more FXRS render-piece sets inside an RX2 file. Output
//! is a flattened mesh (one buffer per attribute, indices rebased
//! across pieces) ready for upload to a GPU.
//!
//! See `docs/equipment-preview-port-audit.md` for the binary layout
//! reference.

#![forbid(unsafe_code)]
#![warn(rust_2018_idioms)]

pub mod container;
pub mod gnf;
pub mod mcd;
pub mod obj;
pub mod pack;
pub mod playermesh;
pub mod skeleton;
pub mod texlib;
pub mod xpr;

use thiserror::Error;

/// 7-byte magic at offset 0 of every RX2 mesh container.
pub const MAGIC_RX2: &[u8; 7] = b"\x89RW4xb2";

const FXRS_TYPE: i32 = 15_663_108;
const UV1_USAGE: i32 = 327_686;
const UV2_USAGE: i32 = 327_943;
const POSITION_USAGE: i32 = 1;
// UV element types observed in the wild. The default is unsigned
// (`UV_TYPE_USHORT = 2_891_865`); the signed variant flips the decode
// formula. We only match on the signed constant — anything else is
// treated as unsigned.
const UV_TYPE_SHORT: i32 = 2_892_121;

const PRIM_TYPE_LIST: i32 = 4;
const PRIM_TYPE_STRIP: i32 = 6;

#[derive(Debug, Error)]
pub enum Rx2Error {
    #[error("input is shorter than the RX2 magic ({0} bytes)")]
    TooShort(usize),
    #[error("magic mismatch — expected `\\x89RW4xb2`")]
    BadMagic,
    #[error("file table is truncated at offset {offset}")]
    TruncatedFileTable { offset: usize },
    #[error("vertex buffer for piece {piece} runs past EOF (needs {needed} bytes, have {have})")]
    TruncatedVertexBuffer {
        piece: usize,
        needed: usize,
        have: usize,
    },
    #[error("index buffer for piece {piece} runs past EOF (needs {needed} bytes, have {have})")]
    TruncatedIndexBuffer {
        piece: usize,
        needed: usize,
        have: usize,
    },
}

/// Decoded mesh suitable for upload to a WebGL/OpenGL buffer.
///
/// All position values are in the source coordinate space (typically
/// normalized to roughly `[-1, 1]`). Vertices/normals are float32 with
/// the conventional `[x, y, z]` ordering. UV coordinates are in
/// `[0, 1]`. Triangle indices are u32 to leave room for large meshes.
#[derive(Debug, Clone, Default)]
pub struct Mesh {
    pub positions: Vec<[f32; 3]>,
    pub uv1: Vec<[f32; 2]>,
    /// Empty when no UV2 channel is present.
    pub uv2: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
}

impl Mesh {
    /// Tight axis-aligned bounding box (min, max) over the position
    /// buffer. Returns `None` when the mesh is empty.
    pub fn bounds(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut iter = self.positions.iter();
        let first = *iter.next()?;
        let mut min = first;
        let mut max = first;
        for &p in iter {
            for i in 0..3 {
                if p[i] < min[i] {
                    min[i] = p[i];
                }
                if p[i] > max[i] {
                    max[i] = p[i];
                }
            }
        }
        Some((min, max))
    }

    /// Center + uniform "span" (max axis size) — handy for fitting
    /// the mesh into the viewer's camera frame. Returns `(center,
    /// span)`. `span` is clamped to 1.0 for empty/degenerate meshes
    /// (matches Heck's `or 1.0`).
    pub fn center_and_span(&self) -> ([f32; 3], f32) {
        match self.bounds() {
            Some((min, max)) => {
                let center = [
                    (min[0] + max[0]) * 0.5,
                    (min[1] + max[1]) * 0.5,
                    (min[2] + max[2]) * 0.5,
                ];
                let span = (max[0] - min[0])
                    .max(max[1] - min[1])
                    .max(max[2] - min[2])
                    .max(1.0);
                (center, span)
            }
            None => ([0.0, 0.0, 0.0], 1.0),
        }
    }

    /// Smooth per-vertex normals (face-normal-weighted average,
    /// normalized).
    pub fn compute_smooth_normals(&self) -> Vec<[f32; 3]> {
        let mut acc = vec![[0f64; 3]; self.positions.len()];
        for tri in &self.triangles {
            let [a, b, c] = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
            let v0 = self.positions[a];
            let v1 = self.positions[b];
            let v2 = self.positions[c];
            let e1 = [
                (v1[0] - v0[0]) as f64,
                (v1[1] - v0[1]) as f64,
                (v1[2] - v0[2]) as f64,
            ];
            let e2 = [
                (v2[0] - v0[0]) as f64,
                (v2[1] - v0[1]) as f64,
                (v2[2] - v0[2]) as f64,
            ];
            let n = [
                e1[1] * e2[2] - e1[2] * e2[1],
                e1[2] * e2[0] - e1[0] * e2[2],
                e1[0] * e2[1] - e1[1] * e2[0],
            ];
            for &i in &[a, b, c] {
                acc[i][0] += n[0];
                acc[i][1] += n[1];
                acc[i][2] += n[2];
            }
        }
        acc.into_iter()
            .map(|n| {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                let inv = if len > 0.0 { 1.0 / len } else { 1.0 };
                [
                    (n[0] * inv) as f32,
                    (n[1] * inv) as f32,
                    (n[2] * inv) as f32,
                ]
            })
            .collect()
    }
}

/// Parse an RX2 mesh payload. Caller should have already decompressed
/// any `chunkzip` wrapping via `tdb-archive-codec`. Skips degenerate
/// FXRS entries silently (matching Heck).
pub fn parse_mesh(data: &[u8]) -> Result<Mesh, Rx2Error> {
    if data.len() < MAGIC_RX2.len() {
        return Err(Rx2Error::TooShort(data.len()));
    }
    if &data[..MAGIC_RX2.len()] != MAGIC_RX2 {
        return Err(Rx2Error::BadMagic);
    }

    // Header fields (big-endian i32). Heck's positions: file_count
    // at 0x20, file_table at 0x30, hdr_size at 0x44, data_size at
    // 0x54. When data_size is 0 the header treats hdr_size as 0 too.
    let file_count = read_i32_be(data, 0x20)?;
    let file_table = read_i32_be(data, 0x30)? as usize;
    let mut hdr_size = read_i32_be(data, 0x44)? as usize;
    let data_size = read_i32_be(data, 0x54)?;
    if data_size == 0 {
        hdr_size = 0;
    }
    if file_count <= 0 {
        return Ok(Mesh::default());
    }
    let file_count = file_count as usize;

    let file_table_size = file_count
        .checked_mul(24)
        .ok_or(Rx2Error::TruncatedFileTable { offset: file_table })?;
    if file_table + file_table_size > data.len() {
        return Err(Rx2Error::TruncatedFileTable { offset: file_table });
    }

    // file_list[i] = (offset, _, _, _, _, type) — 6 i32 BE per entry.
    let mut file_list = Vec::with_capacity(file_count);
    for i in 0..file_count {
        let base = file_table + i * 24;
        let e = [
            read_i32_be(data, base)?,
            read_i32_be(data, base + 4)?,
            read_i32_be(data, base + 8)?,
            read_i32_be(data, base + 12)?,
            read_i32_be(data, base + 16)?,
            read_i32_be(data, base + 20)?,
        ];
        file_list.push(e);
    }

    let mut mesh = Mesh::default();
    let mut piece_idx = 0usize;

    for i in 0..file_count {
        if file_list[i][5] != FXRS_TYPE {
            continue;
        }
        let rs_off = file_list[i][0] as usize;
        if rs_off + 12 > data.len() {
            continue;
        }
        let mesh_idx = read_i32_be(data, rs_off)? as usize;
        let vb_desc_idx = read_i32_be(data, rs_off + 4)? as usize;
        let prim_type = read_i32_be(data, rs_off + 8)?;
        if mesh_idx >= file_count || vb_desc_idx >= file_count {
            continue;
        }

        // Vertex-buffer descriptor: i32 i32 u8 u8 u8 u8 i32 then
        // (byte_off, d_type, usage) per element starting at +16.
        let vbd_off = file_list[vb_desc_idx][0] as usize;
        if vbd_off + 16 > data.len() {
            continue;
        }
        let n_elems = data[vbd_off + 8];
        let stride = data[vbd_off + 9] as usize;
        let mut pos_off = 0usize;
        let mut uv_off: Option<(usize, i32)> = None;
        let mut uv2_off: Option<usize> = None;
        let mut elem_ptr = vbd_off + 16;
        for _ in 0..n_elems {
            if elem_ptr + 12 > data.len() {
                break;
            }
            let byte_off = read_i32_be(data, elem_ptr)? as usize;
            let d_type = read_i32_be(data, elem_ptr + 4)?;
            let usage = read_i32_be(data, elem_ptr + 8)?;
            elem_ptr += 12;
            if usage == POSITION_USAGE {
                pos_off = byte_off;
            } else if usage == UV1_USAGE {
                uv_off = Some((byte_off, d_type));
            } else if usage == UV2_USAGE {
                uv2_off = Some(byte_off);
            }
        }

        // mesh_off entry: 6 i32 BE: [_, _, _, _, ib_section_idx, vb_section_idx].
        let mesh_off = file_list[mesh_idx][0] as usize;
        if mesh_off + 24 > data.len() {
            continue;
        }
        let om = [
            read_i32_be(data, mesh_off)?,
            read_i32_be(data, mesh_off + 4)?,
            read_i32_be(data, mesh_off + 8)?,
            read_i32_be(data, mesh_off + 12)?,
            read_i32_be(data, mesh_off + 16)?,
            read_i32_be(data, mesh_off + 20)?,
        ];
        let ib_meta = om[4] as usize;
        let vb_meta = om[5] as usize;
        if ib_meta >= file_count || vb_meta >= file_count {
            continue;
        }

        // Index-buffer header lives at file_list[ib_meta].offset + 24:
        // i32 i32 i32 → [ib_section, _, idx_count].
        let ib_hdr = file_list[ib_meta][0] as usize + 24;
        if ib_hdr + 12 > data.len() {
            continue;
        }
        let ib_section = read_i32_be(data, ib_hdr)? as usize;
        let idx_count = read_i32_be(data, ib_hdr + 8)? as usize;
        if ib_section >= file_count {
            continue;
        }
        let ib_data_off = (file_list[ib_section][0] as usize) + hdr_size;

        // Vertex-buffer header at file_list[vb_meta].offset + 24:
        // i32 i32 i32 i32 i32 → [vb_section_packed, _, _, n_verts, _].
        let vb_hdr = file_list[vb_meta][0] as usize + 24;
        if vb_hdr + 20 > data.len() {
            continue;
        }
        let vb_section = (read_i32_be(data, vb_hdr)? as usize) >> 2;
        let n_verts = read_i32_be(data, vb_hdr + 12)? as usize;
        if vb_section >= file_count {
            continue;
        }
        let vb_data_off = (file_list[vb_section][0] as usize) + hdr_size;

        if n_verts == 0 || idx_count == 0 {
            continue;
        }
        let vb_needed = n_verts.checked_mul(stride).unwrap_or(usize::MAX);
        if vb_data_off
            .checked_add(vb_needed)
            .map_or(true, |end| end > data.len())
        {
            return Err(Rx2Error::TruncatedVertexBuffer {
                piece: piece_idx,
                needed: vb_needed,
                have: data.len().saturating_sub(vb_data_off),
            });
        }
        let ib_needed = idx_count.checked_mul(2).unwrap_or(usize::MAX);
        if ib_data_off
            .checked_add(ib_needed)
            .map_or(true, |end| end > data.len())
        {
            return Err(Rx2Error::TruncatedIndexBuffer {
                piece: piece_idx,
                needed: ib_needed,
                have: data.len().saturating_sub(ib_data_off),
            });
        }

        // Per-vertex decode.
        let base_index = mesh.positions.len() as u32;
        for v in 0..n_verts {
            let off = vb_data_off + v * stride;
            let x = half_be(data[off + pos_off], data[off + pos_off + 1]);
            let y = half_be(data[off + pos_off + 2], data[off + pos_off + 3]);
            let z = half_be(data[off + pos_off + 4], data[off + pos_off + 5]);
            mesh.positions.push([x, y, z]);
            let (u, v_coord) = match uv_off {
                Some((uoff, d_type)) if d_type == UV_TYPE_SHORT => {
                    let ur = i16::from_be_bytes([data[off + uoff], data[off + uoff + 1]]);
                    let vr = i16::from_be_bytes([data[off + uoff + 2], data[off + uoff + 3]]);
                    (
                        (ur as i32 + 32768) as f32 / 65535.0,
                        (vr as i32 + 32768) as f32 / 65535.0,
                    )
                }
                Some((uoff, _)) => {
                    let ur = u16::from_be_bytes([data[off + uoff], data[off + uoff + 1]]);
                    let vr = u16::from_be_bytes([data[off + uoff + 2], data[off + uoff + 3]]);
                    (ur as f32 / 65535.0, vr as f32 / 65535.0)
                }
                None => (0.0, 0.0),
            };
            mesh.uv1.push([u, v_coord]);
            if let Some(uoff) = uv2_off {
                let ur = u16::from_be_bytes([data[off + uoff], data[off + uoff + 1]]);
                let vr = u16::from_be_bytes([data[off + uoff + 2], data[off + uoff + 3]]);
                mesh.uv2.push([ur as f32 / 65535.0, vr as f32 / 65535.0]);
            }
        }

        // Decode indices.
        let mut indices = Vec::with_capacity(idx_count);
        for k in 0..idx_count {
            let p = ib_data_off + k * 2;
            indices.push(u16::from_be_bytes([data[p], data[p + 1]]));
        }
        let tris: Vec<[u32; 3]> = match prim_type {
            PRIM_TYPE_STRIP => {
                let mut out = Vec::new();
                let mut strip: Vec<u16> = Vec::new();
                for idx in &indices {
                    if *idx == 0xFFFF {
                        flush_strip(&strip, &mut out);
                        strip.clear();
                    } else {
                        strip.push(*idx);
                    }
                }
                flush_strip(&strip, &mut out);
                out
            }
            PRIM_TYPE_LIST => indices
                .chunks_exact(3)
                .map(|c| [c[0] as u32, c[1] as u32, c[2] as u32])
                .collect(),
            _ => Vec::new(),
        };
        for t in tris {
            mesh.triangles
                .push([t[0] + base_index, t[1] + base_index, t[2] + base_index]);
        }
        piece_idx += 1;
    }
    Ok(mesh)
}

fn flush_strip(strip: &[u16], out: &mut Vec<[u32; 3]>) {
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

fn read_i32_be(data: &[u8], offset: usize) -> Result<i32, Rx2Error> {
    if offset + 4 > data.len() {
        return Err(Rx2Error::TruncatedFileTable { offset });
    }
    let bytes: [u8; 4] = data[offset..offset + 4].try_into().expect("len checked");
    Ok(i32::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load_asset(name: &str) -> Option<Vec<u8>> {
        let p = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("tdb-gui-api")
            .join("assets")
            .join("equipment-preview")
            .join(name);
        if !p.is_file() {
            return None;
        }
        std::fs::read(&p).ok()
    }

    #[test]
    fn half_be_zero_and_one() {
        assert_eq!(half_be(0, 0), 0.0);
        assert_eq!(half_be(0x3C, 0x00), 1.0);
        assert!((half_be(0xBC, 0x00) + 1.0).abs() < 1e-6); // -1.0
    }

    #[test]
    fn parses_glove_mesh_smoke() {
        let Some(data) = load_asset("feprop_glove_0.rx2") else {
            eprintln!("skipping: bundled glove mesh not present");
            return;
        };
        let mesh = parse_mesh(&data).expect("parse must succeed");
        assert!(
            mesh.positions.len() >= 100,
            "glove mesh should have plenty of vertices, got {}",
            mesh.positions.len()
        );
        assert!(!mesh.triangles.is_empty());
        // Triangle indices must all stay in range — guards against
        // truncated index-buffer bugs.
        for t in &mesh.triangles {
            assert!((t[0] as usize) < mesh.positions.len());
            assert!((t[1] as usize) < mesh.positions.len());
            assert!((t[2] as usize) < mesh.positions.len());
        }
        // UV1 should track vertex count when present.
        assert_eq!(mesh.uv1.len(), mesh.positions.len());
    }

    #[test]
    fn parses_helmet_mesh_smoke() {
        let Some(data) = load_asset("helmet_14_2.rx2") else {
            eprintln!("skipping: bundled helmet mesh not present");
            return;
        };
        let mesh = parse_mesh(&data).expect("parse must succeed");
        assert!(mesh.positions.len() >= 100);
        // Heck's `tm_uses_uv2_offset` config flag implies UV2 *might*
        // be present, but it's not actually a hard requirement of the
        // helmet RX2 — when the channel is absent the UV2-corrected
        // path falls through to behavior equivalent to the standard
        // mask lookup. We don't assert UV2 presence here; the
        // observation lives in [`uv2_observation_on_bundled_helmet`].
        if !mesh.uv2.is_empty() {
            assert_eq!(mesh.uv2.len(), mesh.positions.len());
        }
    }

    /// Observation: does the bundled helmet RX2 actually carry a UV2
    /// element? If yes, the helmet-mask compositor exercises the
    /// KDTree path; if no, it degenerates to a standard mask lookup
    /// (UV2 = zero → sampling at the same UV1 the vertex shader
    /// uses). Either is faithful to Heck — captures a "ground truth"
    /// fact about the asset rather than a behavior we enforce.
    #[test]
    fn uv2_observation_on_bundled_helmet() {
        let Some(data) = load_asset("helmet_14_2.rx2") else {
            return;
        };
        let mesh = parse_mesh(&data).unwrap();
        eprintln!(
            "bundled helmet UV2 presence: {} entries (positions = {})",
            mesh.uv2.len(),
            mesh.positions.len()
        );
    }

    #[test]
    fn bounding_box_and_normals_make_sense() {
        let Some(data) = load_asset("feprop_glove_0.rx2") else {
            return;
        };
        let mesh = parse_mesh(&data).unwrap();
        let (center, span) = mesh.center_and_span();
        assert!(span > 0.0);
        // The center should land somewhere inside the bounding box.
        let (min, max) = mesh.bounds().unwrap();
        for i in 0..3 {
            assert!(min[i] - 1e-3 <= center[i] && center[i] <= max[i] + 1e-3);
        }
        let normals = mesh.compute_smooth_normals();
        assert_eq!(normals.len(), mesh.positions.len());
        // Most normals should be unit-length.
        let unit_count = normals
            .iter()
            .filter(|n| {
                let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                (len - 1.0).abs() < 1e-3
            })
            .count();
        assert!(
            unit_count as f32 / normals.len() as f32 > 0.99,
            "expected ≥99% unit-length normals, got {}/{}",
            unit_count,
            normals.len()
        );
    }
}
