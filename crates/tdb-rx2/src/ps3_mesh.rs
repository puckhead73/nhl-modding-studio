//! Bounded native RW4ps3 EDGE geometry codec. Binary layouts independently implemented
//! from the local RPSGL2 format review; no Python runtime is used by Studio.
//! Skin streams and opaque metadata are preserved, not interpreted as an exportable rig.
use crate::playermesh::{BonePalette, PlayerAsset, PositionFormat, Submesh, UvFormat, Vertex};
use std::collections::HashMap;
pub const MAGIC: &[u8; 8] = b"\x89RW4ps3\0";
type Result<T> = std::result::Result<T, String>;
fn need(ok: bool, msg: &str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(format!("PS3 mesh: {msg}"))
    }
}
fn bytes(b: &[u8], o: usize, n: usize) -> Result<&[u8]> {
    b.get(o..o.checked_add(n).ok_or("PS3 offset overflow")?)
        .ok_or_else(|| "PS3 mesh: truncated buffer".into())
}
fn u16be(b: &[u8], o: usize) -> Result<u16> {
    Ok(u16::from_be_bytes(bytes(b, o, 2)?.try_into().unwrap()))
}
fn u32be(b: &[u8], o: usize) -> Result<u32> {
    Ok(u32::from_be_bytes(bytes(b, o, 4)?.try_into().unwrap()))
}
fn half(v: u16) -> f32 {
    let sign = if v & 0x8000 != 0 { -1.0 } else { 1.0 };
    let e = (v >> 10) & 31;
    let m = (v & 1023) as f32;
    sign * match e {
        0 => m * 2f32.powi(-24),
        31 => {
            if m == 0.0 {
                f32::INFINITY
            } else {
                f32::NAN
            }
        }
        _ => (1.0 + m / 1024.0) * 2f32.powi(e as i32 - 15),
    }
}
fn bits(b: &[u8], o: usize, n: usize) -> Result<u32> {
    need(n <= 32, "invalid bit width")?;
    let mut v = 0;
    for i in o..o + n {
        v = (v << 1)
            | (((*b.get(i / 8).ok_or("PS3 truncated bitstream")? >> (7 - i % 8)) & 1) as u32);
    }
    Ok(v)
}
fn signed(v: u32, n: usize) -> i64 {
    if v & (1 << (n - 1)) != 0 {
        v as i64 - (1i64 << n)
    } else {
        v as i64
    }
}
#[derive(Clone)]
struct Section {
    start: usize,
    size: usize,
    kind: u32,
}
#[derive(Clone)]
struct Attr {
    offset: usize,
    format: u8,
    count: usize,
    semantic: u8,
    size: usize,
    fixed: [u8; 8],
}
#[derive(Clone)]
struct Stream {
    stride: usize,
    attrs: Vec<Attr>,
}
#[derive(Clone)]
struct Part {
    vertex: usize,
    uv: usize,
    stream: Stream,
    uvstream: Stream,
    fixed: Vec<i32>,
}
pub struct Document {
    pub asset: PlayerAsset,
    parts: Vec<Part>,
}
fn stream(b: &[u8]) -> Result<Stream> {
    let h = bytes(b, 0, 8)?;
    let blocks = h[2] as usize;
    need(h[0] as usize <= blocks && h[1] > 0, "invalid stream header")?;
    bytes(b, 0, (blocks + 1) * 8)?;
    let mut attrs = Vec::new();
    for i in 0..h[0] as usize {
        let a = bytes(b, 8 + i * 8, 8)?;
        need(
            (1..=4).contains(&a[2]) && a[4] > 0 && a[0] as usize + a[4] as usize <= h[1] as usize,
            "invalid attribute range",
        )?;
        let fixed = if a[1] == 8 {
            bytes(&b[..(blocks + 1) * 8], 8 + a[6] as usize, 8)?
                .try_into()
                .unwrap()
        } else {
            [0; 8]
        };
        need(
            !attrs.iter().any(|x: &Attr| x.semantic == a[3]),
            "duplicate stream semantic",
        )?;
        attrs.push(Attr {
            offset: a[0] as usize,
            format: a[1],
            count: a[2] as usize,
            semantic: a[3],
            size: a[4] as usize,
            fixed,
        });
    }
    Ok(Stream {
        stride: h[1] as usize,
        attrs,
    })
}
fn decode(b: &[u8], a: &Attr, fixed: &[i32]) -> Result<[f32; 4]> {
    let mut out = [0.0; 4];
    if a.format == 9 {
        need(a.size == 3, "invalid unit vector width")?;
        let w = bits(b, 0, 24)?;
        // EDGE omits the largest component. The other two are quantized
        // over ±1/sqrt(2), not ±1 (confirmed against the matching Xbox mesh).
        let x = (((w >> 14) & 1023) as f32 / 511.5 - 1.0) * std::f32::consts::FRAC_1_SQRT_2;
        let y = (((w >> 4) & 1023) as f32 / 511.5 - 1.0) * std::f32::consts::FRAC_1_SQRT_2;
        let z = (1.0 - x * x - y * y).max(0.0).sqrt() * if w & 1 != 0 { 1.0 } else { -1.0 };
        let k = ((w >> 2) & 3) as usize;
        need(k < 3, "invalid unit vector selector")?;
        out = match k {
            0 => [z, x, y, 0.0],
            1 => [x, z, y, 0.0],
            _ => [x, y, z, 0.0],
        };
    } else {
        let mut bit = 0;
        for i in 0..a.count {
            out[i] = match a.format {
                2 => f32::from_bits(u32be(b, i * 4)?),
                3 => half(u16be(b, i * 2)?),
                5 => u16be(b, i * 2)? as i16 as f32,
                1 => (u16be(b, i * 2)? as i16 as f32 * 2.0 + 1.0) / 65535.0,
                4 => *b.get(i).ok_or("PS3 truncated attribute")? as f32 / 255.0,
                7 => *b.get(i).ok_or("PS3 truncated attribute")? as f32,
                8 => {
                    let n = (a.fixed[i * 2] as usize) + (a.fixed[i * 2 + 1] as usize);
                    need(
                        n > 0 && n <= 32 && a.fixed[i * 2 + 1] <= 30,
                        "invalid fixed point width",
                    )?;
                    let value = signed(bits(b, bit, n)?, n)
                        + *fixed.get(i).ok_or("PS3 missing fixed offsets")? as i64;
                    bit += n;
                    value as f32 / 2f32.powi(a.fixed[i * 2 + 1] as i32)
                }
                _ => {
                    return Err(format!(
                        "PS3 unsupported EDGE attribute format {}",
                        a.format
                    ))
                }
            };
        }
    }
    need(out.iter().all(|v| v.is_finite()), "non-finite vertex")?;
    Ok(out)
}
fn indices(raw: &[u8], count: usize) -> Result<Vec<[u32; 3]>> {
    need(count > 0 && count % 3 == 0, "invalid index count")?;
    let n = u16be(raw, 0)? as usize;
    let delta = u16be(raw, 2)? as i32;
    let control = u16be(raw, 4)? as usize;
    let width = *bytes(raw, 6, 1)?.first().unwrap() as usize;
    need(width <= 16, "invalid delta width")?;
    let codes = 8 + control;
    let packed = codes + (count / 3 * 2).div_ceil(8);
    bytes(raw, packed, (n * width).div_ceil(8))?;
    let mut lanes = [0i32; 8];
    let mut deltas = Vec::new();
    for i in 0..n {
        lanes[i % 8] =
            (bits(&raw[packed..], i * width, width)? as i32 - delta + lanes[i % 8]) & 65535;
        deltas.push(lanes[i % 8] as u32);
    }
    let mut seq = Vec::new();
    let (mut next, mut d) = (0u32, 0usize);
    for i in 0..control * 8 {
        if bits(&raw[8..codes], i, 1)? == 0 {
            seq.push(next);
            next += 1;
        } else {
            seq.push(*deltas.get(d).ok_or("PS3 exhausted index deltas")?);
            d += 1;
        }
    }
    let mut triangles: Vec<[u32; 3]> = Vec::new();
    let mut s = 0;
    for i in 0..count / 3 {
        let code = bits(&raw[codes..packed], i * 2, 2)?;
        let take = if code == 3 { 3 } else { 1 };
        let v = seq.get(s..s + take).ok_or("PS3 exhausted index sequence")?;
        let t = if code == 3 {
            [v[0], v[1], v[2]]
        } else {
            let p = triangles
                .last()
                .ok_or("PS3 first triangle cannot reuse indices")?;
            match code {
                2 => [p[1], p[0], v[0]],
                1 => [p[2], p[1], v[0]],
                _ => [p[0], p[2], v[0]],
            }
        };
        triangles.push(t);
        s += take;
    }
    Ok(triangles)
}
pub fn parse(data: &[u8]) -> Result<Document> {
    need(data.starts_with(MAGIC), "expected RW4ps3")?;
    bytes(data, 0, 192)?;
    let n = u32be(data, 32)? as usize;
    let table = u32be(data, 48)? as usize;
    let base = u32be(data, 68)? as usize;
    need(
        n > 0 && n <= 10000 && base <= data.len(),
        "invalid section header",
    )?;
    bytes(data, table, n * 24)?;
    let mut sections = Vec::new();
    for i in 0..n {
        let o = table + i * 24;
        let kind = u32be(data, o + 20)?;
        let start = (u32be(data, o)? as usize)
            .checked_add(if kind == 0x10030 { base } else { 0 })
            .ok_or("PS3 section overflow")?;
        let size = u32be(data, o + 8)? as usize;
        bytes(data, start, size)?;
        sections.push(Section { start, size, kind });
    }
    if !sections.iter().any(|s| s.kind == 0x20050) && sections.iter().any(|s| s.kind == 0x20009) {
        return static_geometry(data, &sections);
    }
    let first = sections
        .iter()
        .filter(|s| s.kind != 0x10030)
        .map(|s| s.start)
        .min()
        .ok_or("PS3 no descriptors")?;
    need(first >= 192, "invalid descriptor offset")?;
    let manifests: Vec<_> = (192..first.saturating_sub(24))
        .step_by(4)
        .filter(|&o| u32be(data, o).ok() == Some(0x10007))
        .collect();
    need(manifests.len() == 1, "unknown reference manifest")?;
    let refs = u32be(data, manifests[0] + 20)? as usize;
    let refcount = u32be(data, manifests[0] + 24)? as usize;
    need(refcount <= data.len() / 8, "invalid reference count")?;
    bytes(data, refs, refcount * 8)?;
    let pointer = |value: u32, size: usize| -> Result<usize> {
        let (index, offset) = if value & 0x800000 != 0 {
            let i = (value & 0x7fffff) as usize;
            need(i < refcount, "invalid subreference")?;
            (
                u32be(data, refs + i * 8)? as usize,
                u32be(data, refs + i * 8 + 4)? as usize,
            )
        } else {
            (value as usize, 0)
        };
        let sec = sections.get(index).ok_or("PS3 invalid section reference")?;
        need(
            offset <= sec.size && size <= sec.size - offset,
            "stream outside section",
        )?;
        Ok(sec.start + offset)
    };
    let names = section_names(data, &sections)?;
    let (mut parts, mut submeshes) = (Vec::new(), Vec::new());
    for (section_id, sec) in sections
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == 0x20050)
    {
        need(sec.size >= 16, "short EDGE header")?;
        let m = sec.start;
        // NHL09 predates the split SPU/RSX descriptors: 128-byte configs
        // reference one interleaved stream. Counts include SPU padding to eight vertices.
        if u32be(data, m + 12)? == 0 && [1, 2].contains(&u32be(data, m + 8)?) {
            let count = u32be(data, m + 4)? as usize;
            need(count > 0 && count <= 256, "invalid early EDGE segment count")?;
            let config = pointer(u32be(data, m)?, count * 128)?;
            for j in 0..count {
                let c = config + j * 128;
                let (stride, uv_offset) = match (data[c], data[c + 2]) {
                    (4, 9) => (16, 12), // position, normal, tangent, UV
                    (5, 12) => (20, 12), // position, normal, tangent, two UV sets
                    (5, 13) => (21, 9),  // position, normal, three UV sets
                    _ => return Err("PS3 unsupported early EDGE vertex format".into()),
                };
                need([0x22, 0x23].contains(&data[c + 5]), "unsupported early EDGE index mode")?;
                let declared = u16be(data, c + 8)? as usize;
                let ni = u16be(data, c + 10)? as usize;
                need(u32be(data, c + 28)? == 0, "unsupported early EDGE secondary stream")?;
                let size = (0..3).map(|k| u16be(data, c + 32 + k * 2).map(|v| v as usize)).collect::<Result<Vec<_>>>()?.iter().sum::<usize>();
                let nv = size / stride;
                need(nv >= 3 && declared >= nv && declared - nv < 8, "invalid early EDGE vertex count")?;
                let vertex = pointer(u32be(data, c + 24)?, size)?;
                let isize = u16be(data, c + 20)? as usize + u16be(data, c + 22)? as usize;
                let ipos = pointer(u32be(data, c + 16)?, isize)?;
                let triangles = indices(bytes(data, ipos, isize)?, ni)?;
                need(triangles.iter().flatten().all(|&v| (v as usize) < nv), "early EDGE index outside vertices")?;
                need(u32be(data, c + 112)? == 16, "unsupported early EDGE fixed offsets")?;
                let fp = pointer(u32be(data, c + 120)?, 16)?;
                let fixed = (0..4).map(|k| u32be(data, fp + k * 4).map(|v| v as i32)).collect::<Result<Vec<_>>>()?;
                let stream = Stream { stride, attrs: vec![
                    Attr { offset: 0, format: 8, count: 3, semantic: 1, size: 6, fixed: [4,12,4,12,4,12,0,0] },
                    Attr { offset: 6, format: 9, count: 3, semantic: 2, size: 3, fixed: [0;8] },
                    Attr { offset: uv_offset, format: 3, count: 2, semantic: 5, size: 4, fixed: [0;8] },
                ] };
                let mut vertices = Vec::with_capacity(nv);
                for i in 0..nv {
                    let values = stream.attrs.iter().map(|a| decode(bytes(data, vertex+i*stride+a.offset,a.size)?, a, &fixed)).collect::<Result<Vec<_>>>()?;
                    vertices.push(Vertex { position: values[0][..3].try_into().unwrap(), normal: values[1][..3].try_into().unwrap(), uv: values[2][..2].try_into().unwrap(), joints:[0;4], weights:[1.0,0.0,0.0,0.0] });
                }
                submeshes.push(Submesh { name: format!("{}_segment_{j}",names.get(&section_id).cloned().unwrap_or_else(||format!("ps3_section_{section_id}"))), stride, position_format: PositionFormat::Float, uv_format: UvFormat::HalfFloat, has_blend_indices:false, has_blend_weights:false, vertices, triangles });
                // Preview/export only until the early format's write contract is qualified.
                // No patch slots: patch_mesh rejects this layout until write support is tested.
            }
            continue;
        }
        let count = u32be(data, m + 8)? as usize;
        need(
            count > 0 && count <= 256 && [1, 2].contains(&u32be(data, m + 12)?),
            "unsupported EDGE header",
        )?;
        let config = pointer(u32be(data, m)?, count * 96)?;
        let rsx = pointer(u32be(data, m + 4)?, count * 48)?;
        let mut offsets = HashMap::new();
        let mut previous = 0;
        for j in 0..count {
            let c = config + j * 96;
            let r = rsx + j * 48;
            let nv = u16be(data, c + 8)? as usize;
            let ni = u16be(data, c + 10)? as usize;
            need(
                nv >= 3 && [0x22, 0x23].contains(&data[c + 6]),
                "unsupported EDGE indices/skinning",
            )?;
            let isize = u16be(data, c + 20)? as usize + u16be(data, c + 22)? as usize;
            let io = pointer(u32be(data, c + 16)?, isize)?;
            let triangles = indices(bytes(data, io, isize)?, ni)?;
            need(
                triangles.iter().flatten().all(|&i| (i as usize) < nv),
                "index outside vertices",
            )?;
            let size = (0..3)
                .map(|k| u16be(data, c + 28 + k * 2).map(|v| v as usize))
                .collect::<Result<Vec<_>>>()?
                .iter()
                .sum();
            let vertex = pointer(u32be(data, c + 24)?, size)?;
            let fs = u32be(data, c + 40)? as usize;
            need(fs % 4 == 0, "invalid fixed offsets")?;
            let fo = pointer(u32be(data, c + 36)?, fs)?;
            let fixed = (0..fs / 4)
                .map(|i| u32be(data, fo + i * 4).map(|v| v as i32))
                .collect::<Result<Vec<_>>>()?;
            let ds = u16be(data, c + 48)? as usize;
            let dspos = pointer(u32be(data, c + 44)?, ds)?;
            let stream = stream(bytes(data, dspos, ds)?)?;
            need(nv * stream.stride <= size, "short vertex stream")?;
            let uvsize = u32be(data, r + 4)? as usize;
            let mut reference = u32be(data, r)?;
            if j > 0 && reference == 0 {
                reference = previous;
            }
            previous = reference;
            let offset = *offsets.get(&reference).unwrap_or(&0usize);
            let uv = pointer(
                reference,
                offset.checked_add(uvsize).ok_or("PS3 UV overflow")?,
            )? + offset;
            offsets.insert(reference, offset + uvsize.div_ceil(16) * 16);
            let ds = u16be(data, r + 12)? as usize;
            let dspos = pointer(u32be(data, r + 8)?, ds)?;
            let uvstream = self::stream(bytes(data, dspos, ds)?)?;
            need(nv * uvstream.stride <= uvsize, "short UV stream")?;
            let mut vertices = vec![
                Vertex {
                    position: [0.0; 3],
                    normal: [0.0; 3],
                    uv: [0.0; 2],
                    joints: [0; 4],
                    weights: [1.0, 0.0, 0.0, 0.0]
                };
                nv
            ];
            let mut found = [false; 3];
            for (s, start) in [(&stream, vertex), (&uvstream, uv)] {
                let mut fi = 0;
                for a in &s.attrs {
                    let f = if a.format == 8 {
                        let x = fixed.get(fi..fi + a.count).ok_or("PS3 missing offsets")?;
                        fi += 4;
                        x
                    } else {
                        &[]
                    };
                    if ![1, 2, 5].contains(&a.semantic) {
                        continue;
                    }
                    for (i, v) in vertices.iter_mut().enumerate() {
                        let x =
                            decode(bytes(data, start + i * s.stride + a.offset, a.size)?, a, f)?;
                        match a.semantic {
                            1 => {
                                need(a.count >= 3, "short position")?;
                                v.position = x[..3].try_into().unwrap();
                                found[0] = true
                            }
                            2 => {
                                v.normal = x[..3].try_into().unwrap();
                                found[1] = true
                            }
                            5 => {
                                need(a.count >= 2, "short UV")?;
                                v.uv = x[..2].try_into().unwrap();
                                found[2] = true
                            }
                            _ => {}
                        }
                    }
                }
            }
            need(found[0] && found[2], "missing position/UV")?;
            // NHL11 low-detail meshes omit normals. Derive preview normals from
            // their real topology; never invent texture coordinates or write normals back.
            if !found[1] {
                let mut sums = vec![[0.0f64;3]; vertices.len()];
                for triangle in &triangles {
                    let [a,b,c] = triangle.map(|i|vertices[i as usize].position);
                    let u:[f64;3]=std::array::from_fn(|k|b[k] as f64-a[k] as f64);
                    let v:[f64;3]=std::array::from_fn(|k|c[k] as f64-a[k] as f64);
                    let n=[u[1]*v[2]-u[2]*v[1],u[2]*v[0]-u[0]*v[2],u[0]*v[1]-u[1]*v[0]];
                    for &i in triangle {for k in 0..3 {sums[i as usize][k]+=n[k];}}
                }
                for (vertex,n) in vertices.iter_mut().zip(sums) {
                    let length=n.iter().map(|v|v*v).sum::<f64>().sqrt().max(1e-20);
                    vertex.normal=n.map(|v|(v/length) as f32);
                }
            }
            submeshes.push(Submesh {
                name: format!(
                    "{}_segment_{j}",
                    names
                        .get(&section_id)
                        .cloned()
                        .unwrap_or_else(|| format!("ps3_section_{section_id}"))
                ),
                stride: stream.stride,
                position_format: PositionFormat::Float,
                uv_format: UvFormat::HalfFloat,
                has_blend_indices: false,
                has_blend_weights: false,
                vertices,
                triangles,
            });
            parts.push(Part {
                vertex,
                uv,
                stream,
                uvstream,
                fixed,
            });
        }
    }
    need(
        !submeshes.is_empty(),
        "unsupported geometry layout (expected EDGE 0x20050)",
    )?;
    Ok(Document {
        asset: PlayerAsset {
            submeshes,
            skin: BonePalette { matrices: vec![] },
        },
        parts,
    })
}
// Stadium/ice and standalone static geometry share this descriptor layout.
// Read/export only: no reallocation or scene/skin metadata rewriting is admitted.
fn section_names(data: &[u8], sections: &[Section]) -> Result<HashMap<usize, String>> {
    let mut names = HashMap::new();
    for sec in sections.iter().filter(|s| s.kind == 0xec0010) {
        if sec.size < 8 || u32be(data, sec.start + 4)? != 0x1505 {
            continue;
        }
        let count = u16be(data, sec.start + 2)? as usize;
        need(8 + count * 12 <= sec.size, "invalid name table")?;
        for i in 0..count {
            let e = sec.start + 8 + i * 12;
            let ptr = u32be(data, e)? as usize;
            let key = u32be(data, e + 8)? as usize;
            need(
                key < sections.len()
                    && sections[key].kind == u32be(data, e + 4)?
                    && ptr >= sec.start + 8 + count * 12
                    && ptr < sec.start + sec.size,
                "invalid resource name reference",
            )?;
            let raw = bytes(data, ptr, sec.start + sec.size - ptr)?;
            let end = raw
                .iter()
                .position(|&c| c == 0)
                .ok_or("PS3 unterminated resource name")?;
            let name = std::str::from_utf8(&raw[..end]).map_err(|_| "PS3 invalid name")?;
            names.insert(key, name.to_string());
        }
    }
    Ok(names)
}

/// OpenGL triangle strips alternate winding, including across degenerate joins.
fn static_triangles(indices: &[u32], primitive: u32) -> Result<Vec<[u32; 3]>> {
    match primitive {
        4 => {
            need(indices.len() % 3 == 0, "invalid triangle list")?;
            Ok(indices.chunks_exact(3).map(|t| [t[0], t[1], t[2]]).collect())
        }
        5 => Ok(indices.windows(3).enumerate().filter_map(|(i, t)| {
            if t[0] == t[1] || t[1] == t[2] || t[0] == t[2] { return None; }
            Some(if i % 2 == 0 { [t[0], t[1], t[2]] } else { [t[1], t[0], t[2]] })
        }).collect()),
        _ => Err("PS3 unsupported primitive".into()),
    }
}

fn static_geometry(data: &[u8], sections: &[Section]) -> Result<Document> {
    let names = section_names(data, sections)?;
    let get = |id: u32, kind: u32| -> Result<&Section> {
        let s = sections
            .get(id as usize)
            .ok_or("PS3 bad static reference")?;
        need(s.kind == kind, "incorrect static section kind")?;
        Ok(s)
    };
    let mut meshes = Vec::new();
    for (id, m) in sections
        .iter()
        .enumerate()
        .filter(|(_, s)| s.kind == 0x20009)
    {
        need(
            m.size == 32 && u32be(data, m.start)? == 1,
            "unsupported static draw",
        )?;
        let ih = get(u32be(data, m.start + 24)?, 0x20007)?;
        let vh = get(u32be(data, m.start + 28)?, 0x20005)?;
        need(ih.size == 32 && vh.size == 36, "unsupported static header")?;
        let vd = get(u32be(data, vh.start)?, 0x20004)?;
        let vb = get(u32be(data, vh.start + 32)?, 0x10030)?;
        let ib = get(u32be(data, ih.start + 16)?, 0x10030)?;
        let h = bytes(data, vd.start, vd.size)?;
        let count = u32be(data, vh.start + 4)? as usize;
        let ni = u32be(data, ih.start + 4)? as usize;
        let na = *h.first().ok_or("PS3 short static descriptor")? as usize;
        let stride = *h.get(1).ok_or("PS3 short static stride")? as usize;
        need(
            vd.size == 12 + 16 * na
                && u32be(h, 4)? as usize == vd.size
                && stride > 0
                && count >= 3
                && count.checked_mul(stride) == Some(vb.size),
            "invalid static vertex layout",
        )?;
        let primitive = u32be(data, ih.start + 8)?;
        let first = u32be(data, m.start + 8)? as usize;
        let draw_count = u32be(data, m.start + 12)? as usize;
        need(
            matches!(primitive, 4 | 5)
                && ni >= 3
                && draw_count >= 3
                && (primitive == 5 || draw_count % 3 == 0)
                && first <= ni && draw_count <= ni - first,
            "unsupported static draw range",
        )?;
        let width = match u32be(data, ih.start + 28)? {
            0x21403 => 2,
            0x41405 => 4,
            _ => return Err("PS3 unsupported static index format".into()),
        };
        need(
            ni.checked_mul(width) == Some(ib.size),
            "invalid static indices",
        )?;
        let mut attrs = HashMap::new();
        for a in 0..na {
            let o = 12 + a * 16;
            let key = u32be(h, o)?;
            let typ = u32be(h, o + 4)?;
            let offset = u32be(h, o + 8)? as usize;
            let components = h[o + 13] as usize;
            let size = h[o + 14] as usize;
            need(
                offset <= stride
                    && size <= stride - offset
                    && components > 0
                    && components <= 4
                    && attrs.insert(key, (typ, offset, components, size)).is_none(),
                "invalid static attribute",
            )?;
        }
        let floats = |key: u32, i: usize| -> Result<[f32; 4]> {
            let &(typ, offset, n, size) = attrs.get(&key).ok_or("PS3 missing static attribute")?;
            let width = match typ {
                0x1406 => 4,
                0x140b => 2,
                _ => return Err("PS3 unsupported float type".into()),
            };
            need(size == n * width, "invalid static attribute size")?;
            let mut out = [0.0; 4];
            for k in 0..n {
                let o = vb.start + i * stride + offset + k * width;
                out[k] = if width == 4 {
                    f32::from_bits(u32be(data, o)?)
                } else {
                    half(u16be(data, o)?)
                };
            }
            need(
                out.iter().all(|x| x.is_finite()),
                "non-finite static vertex",
            )?;
            Ok(out)
        };
        need(
            attrs.get(&0).is_some_and(|a| a.2 >= 3),
            "missing static position",
        )?;
        let mut vertices = Vec::new();
        for i in 0..count {
            let position = floats(0, i)?[..3].try_into().unwrap();
            let uv = if attrs.contains_key(&8) {
                floats(8, i)?[..2].try_into().unwrap()
            } else {
                [0.0; 2]
            };
            let normal = if let Some(&(typ, offset, _, size)) = attrs.get(&2) {
                if typ == 0x6020 && size == 4 {
                    let v = u32be(data, vb.start + i * stride + offset)?;
                    [
                        signed(v & 2047, 11) as f32 / 1023.0,
                        signed((v >> 11) & 2047, 11) as f32 / 1023.0,
                        signed((v >> 22) & 1023, 10) as f32 / 511.0,
                    ]
                } else {
                    floats(2, i)?[..3].try_into().unwrap()
                }
            } else {
                [0.0; 3]
            };
            vertices.push(Vertex {
                position,
                uv,
                normal,
                joints: [0; 4],
                weights: [1.0, 0.0, 0.0, 0.0],
            });
        }
        let mut indices = Vec::with_capacity(ni);
        for i in 0..ni {
            let o = ib.start + i * width;
            let index = if width == 2 { u16be(data, o)? as u32 } else { u32be(data, o)? };
            need((index as usize) < count, "static index out of range")?;
            indices.push(index);
        }
        let triangles = static_triangles(&indices[first..first + draw_count], primitive)?;
        if !attrs.contains_key(&2) {
            let mut accumulated=vec![[0f64;3];vertices.len()];
            for t in &triangles {
                let [a, b, c] = t.map(|i| vertices[i as usize].position);
                let u: [f64; 3] = std::array::from_fn(|k| b[k] as f64 - a[k] as f64);
                let v: [f64; 3] = std::array::from_fn(|k| c[k] as f64 - a[k] as f64);
                let n = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                for &i in t {
                    for k in 0..3 {
                        accumulated[i as usize][k] += n[k];
                    }
                }
            }
            for (v,n) in vertices.iter_mut().zip(accumulated) {let length=n.iter().map(|x|x*x).sum::<f64>().sqrt().max(1e-20);v.normal=n.map(|x|(x/length) as f32);}
        }
        meshes.push(Submesh {
            name: names.get(&id).cloned().unwrap_or_else(|| format!("ps3_static_{id}")),
            stride,
            position_format: PositionFormat::Float,
            uv_format: UvFormat::HalfFloat,
            has_blend_indices: false,
            has_blend_weights: false,
            vertices,
            triangles,
        });
    }
    Ok(Document {
        asset: PlayerAsset {
            submeshes: meshes,
            skin: BonePalette { matrices: vec![] },
        },
        parts: vec![],
    })
}
fn normalize(v: [f32; 3]) -> Result<[f32; 3]> {
    let n = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    need(n.is_finite() && n > 1e-8, "invalid normal/tangent")?;
    Ok(v.map(|x| x / n))
}
fn unit_bytes(v: [f32; 3], old: &[u8]) -> Result<[u8; 3]> {
    let v = normalize(v)?;
    let k = (0..3)
        .max_by(|&a, &b| v[a].abs().total_cmp(&v[b].abs()))
        .unwrap();
    let pair: Vec<_> = (0..3)
        .filter(|&i| i != k)
        .map(|i| ((v[i] * std::f32::consts::SQRT_2 + 1.0) * 511.5).round().clamp(0.0, 1023.0) as u32)
        .collect();
    let word = (pair[0] << 14)
        | (pair[1] << 4)
        | ((k as u32) << 2)
        | (old[2] as u32 & 2)
        | u32::from(v[k] >= 0.0);
    let b = word.to_be_bytes();
    Ok([b[1], b[2], b[3]])
}
fn tangents(s: &Submesh) -> Result<Vec<[f32; 3]>> {
    let mut accum = vec![[0.0; 3]; s.vertices.len()];
    for t in &s.triangles {
        let [a, b, c] = t.map(|i| s.vertices[i as usize]);
        let u = [b.uv[0] - a.uv[0], b.uv[1] - a.uv[1]];
        let v = [c.uv[0] - a.uv[0], c.uv[1] - a.uv[1]];
        let d = u[0] * v[1] - u[1] * v[0];
        if d.abs() < 1e-10 {
            continue;
        }
        for k in 0..3 {
            let x = ((b.position[k] - a.position[k]) * v[1]
                - (c.position[k] - a.position[k]) * u[1])
                / d;
            for &i in t {
                accum[i as usize][k] += x;
            }
        }
    }
    accum
        .into_iter()
        .zip(&s.vertices)
        .map(|(mut t, v)| {
            let n = normalize(v.normal)?;
            let dot = (0..3).map(|k| t[k] * n[k]).sum::<f32>();
            for k in 0..3 {
                t[k] -= dot * n[k];
            }
            if t.iter().map(|x| x * x).sum::<f32>() < 1e-12 {
                let axis = if n[0].abs() < 0.9 {
                    [1.0, 0.0, 0.0]
                } else {
                    [0.0, 1.0, 0.0]
                };
                let d = (0..3).map(|k| axis[k] * n[k]).sum::<f32>();
                t = std::array::from_fn(|k| axis[k] - d * n[k]);
            }
            normalize(t)
        })
        .collect()
}
/// Rewrite only admitted vertex attributes. Sections, topology, skinning, other UVs,
/// compressed indices and culling metadata remain byte-identical. Validate before returning.
pub fn patch_positions(original: &[u8], edited: &PlayerAsset) -> Result<Vec<u8>> {
    patch_mesh(original, edited)
}
pub fn patch_mesh(original: &[u8], edited: &PlayerAsset) -> Result<Vec<u8>> {
    let doc = parse(original)?;
    need(
        edited.submeshes.len() == doc.parts.len(),
        "keep original mesh parts",
    )?;
    let mut out = original.to_vec();
    for ((old, new), part) in doc
        .asset
        .submeshes
        .iter()
        .zip(&edited.submeshes)
        .zip(&doc.parts)
    {
        need(
            old.vertices.len() == new.vertices.len() && old.triangles == new.triangles,
            "keep original vertex order/count and triangles",
        )?;
        need(
            new.vertices.iter().all(|v| {
                v.position
                    .iter()
                    .chain(&v.normal)
                    .chain(&v.uv)
                    .all(|x| x.is_finite())
            }),
            "non-finite vertex",
        )?;
        let changed = old
            .vertices
            .iter()
            .zip(&new.vertices)
            .any(|(a, b)| a.position != b.position || a.normal != b.normal || a.uv != b.uv);
        if !changed {
            continue;
        }
        let tangent = tangents(new)?;
        for attr in &part.stream.attrs {
            need(
                [1, 2, 3].contains(&attr.semantic),
                "unsupported writable vertex semantic",
            )?;
            if attr.semantic == 1 {
                need(
                    attr.format == 8
                        && attr.size == 6
                        && attr.count == 3
                        && attr.fixed[..6] == [2, 14, 2, 14, 2, 14]
                        && part.fixed.len() >= 3,
                    "unsupported position packing",
                )?;
            } else {
                need(
                    attr.format == 9 && attr.size == 3,
                    "unsupported shading packing",
                )?;
            }
            for (i, (a, b)) in old.vertices.iter().zip(&new.vertices).enumerate() {
                let o = part.vertex + i * part.stream.stride + attr.offset;
                match attr.semantic {
                    1 => {
                        for k in 0..3 {
                            if a.position[k] == b.position[k] {
                                continue;
                            }
                            let value =
                                (b.position[k] as f64 * 16384.0).round() - part.fixed[k] as f64;
                            need(
                                (-32768.0..=32767.0).contains(&value),
                                "position exceeds original fixed-point range",
                            )?;
                            out[o + k * 2..o + k * 2 + 2]
                                .copy_from_slice(&(value as i16).to_be_bytes());
                        }
                    }
                    2 => {
                        if a.normal != b.normal {
                            out[o..o + 3]
                                .copy_from_slice(&unit_bytes(b.normal, &original[o..o + 3])?);
                        }
                    }
                    3 => {
                        out[o..o + 3].copy_from_slice(&unit_bytes(tangent[i], &original[o..o + 3])?)
                    }
                    _ => unreachable!(),
                }
            }
        }
        for attr in part.uvstream.attrs.iter().filter(|a| a.semantic == 5) {
            need(
                attr.format == 3 && attr.count == 2 && attr.size == 4,
                "unsupported UV packing",
            )?;
            for (i, (a, b)) in old.vertices.iter().zip(&new.vertices).enumerate() {
                let o = part.uv + i * part.uvstream.stride + attr.offset;
                for k in 0..2 {
                    if a.uv[k] == b.uv[k] {
                        continue;
                    }
                    let encoded = crate::playermesh::f32_to_half_be(b.uv[k]);
                    need(
                        half(u16::from_be_bytes(encoded)).is_finite(),
                        "UV exceeds half-float range",
                    )?;
                    out[o + k * 2..o + k * 2 + 2].copy_from_slice(&encoded);
                }
            }
        }
    }
    parse(&out)?;
    Ok(out)
}
/// OBJ interchange deliberately retains one parallel index per original vertex.
pub fn export_obj(asset: &PlayerAsset) -> String {
    use std::fmt::Write;
    let mut s = String::from("# Studio PS3 mesh: preserve vertex ordering and triangle indices\n");
    for m in &asset.submeshes {
        for v in &m.vertices {
            writeln!(s, "v {} {} {}", v.position[0], v.position[1], v.position[2]).unwrap();
        }
    }
    for m in &asset.submeshes {
        for v in &m.vertices {
            writeln!(s, "vt {} {}", v.uv[0], v.uv[1]).unwrap();
        }
    }
    for m in &asset.submeshes {
        for v in &m.vertices {
            writeln!(s, "vn {} {} {}", v.normal[0], v.normal[1], v.normal[2]).unwrap();
        }
    }
    let mut base = 1u32;
    for m in &asset.submeshes {
        writeln!(s, "g {}", m.name).unwrap();
        for t in &m.triangles {
            let [a, b, c] = t.map(|i| i + base);
            writeln!(s, "f {a}/{a}/{a} {b}/{b}/{b} {c}/{c}/{c}").unwrap();
        }
        base += m.vertices.len() as u32;
    }
    s
}
/// Strict OBJ import avoids generic OBJ vertex deduplication, which loses EDGE slot identity.
pub fn import_obj(original: &[u8], text: &str) -> Result<Vec<u8>> {
    let mut d = parse(original)?;
    let (mut pos, mut uv, mut normal, mut faces) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for line in text.lines() {
        let w: Vec<_> = line.split_whitespace().collect();
        if w.is_empty() {
            continue;
        }
        let numbers = |n: usize| -> Result<Vec<f32>> {
            need(w.len() == n + 1, "unexpected OBJ attribute count")?;
            w[1..]
                .iter()
                .map(|s| {
                    s.parse::<f32>()
                        .map_err(|_| "PS3 invalid OBJ number".into())
                })
                .collect()
        };
        match w[0] {
            "v" => pos.push(<[f32; 3]>::try_from(numbers(3)?).unwrap()),
            "vt" => uv.push(<[f32; 2]>::try_from(numbers(2)?).unwrap()),
            "vn" => normal.push(<[f32; 3]>::try_from(numbers(3)?).unwrap()),
            "f" => {
                need(w.len() == 4, "PS3 imports require triangles")?;
                let mut t = [0; 3];
                for i in 0..3 {
                    let parts: Vec<_> = w[i + 1].split('/').collect();
                    need(
                        parts.len() == 3 && parts[0] == parts[1] && parts[0] == parts[2],
                        "keep matching OBJ position/UV/normal indices",
                    )?;
                    t[i] = parts[0]
                        .parse::<u32>()
                        .ok()
                        .and_then(|v| v.checked_sub(1))
                        .ok_or("PS3 invalid OBJ index")?;
                }
                faces.push(t);
            }
            _ => {}
        }
    }
    let count = d
        .asset
        .submeshes
        .iter()
        .map(|s| s.vertices.len())
        .sum::<usize>();
    need(
        pos.len() == count && uv.len() == count && normal.len() == count,
        "keep all original vertices, UVs and normals",
    )?;
    let (mut vi, mut ti) = (0usize, 0usize);
    for m in &mut d.asset.submeshes {
        for t in &m.triangles {
            need(
                faces.get(ti) == Some(&t.map(|i| i + vi as u32)),
                "keep original vertex ordering and triangles",
            )?;
            ti += 1;
        }
        for (i, v) in m.vertices.iter_mut().enumerate() {
            v.position = pos[vi + i];
            v.uv = uv[vi + i];
            v.normal = normal[vi + i];
        }
        vi += m.vertices.len();
    }
    need(ti == faces.len(), "unexpected extra faces")?;
    patch_mesh(original, &d.asset)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn edge_normals_use_smallest_pair_range() {
        let a = Attr { offset: 0, format: 9, count: 3, semantic: 2, size: 3, fixed: [0; 8] };
        // Getzlaf's first head vertex: independently decoded Xbox normal at
        // the same position is [0.125245, -0.583170, 0.802348].
        let word: u32 = (603 << 14) | (89 << 4) | (2 << 2) | 1;
        let bytes = word.to_be_bytes();
        let n = decode(&bytes[1..], &a, &[]).unwrap();
        for (actual, expected) in n[..3].iter().zip([0.125245, -0.583170, 0.802348]) {
            assert!((actual - expected).abs() < 0.003);
        }
        for v in [[0.2, -0.5, 0.84], [-0.9, 0.3, 0.2], [0.3, 0.9, -0.1]] {
            let v = normalize(v).unwrap();
            let packed = unit_bytes(v, &[0, 0, 2]).unwrap();
            let decoded = decode(&packed, &a, &[]).unwrap();
            for i in 0..3 { assert!((decoded[i] - v[i]).abs() < 0.003); }
            assert_eq!(packed[2] & 2, 2, "preserve tangent handedness");
        }
    }
    #[test]
    fn explicit_and_reused_indices() {
        assert_eq!(
            indices(&[0, 0, 0, 0, 0, 1, 0, 0, 0, 0xe0], 6).unwrap(),
            vec![[0, 1, 2], [1, 0, 3]]
        );
    }
    #[test]
    fn rejects_truncated_headers() {
        for n in 0..192 {
            assert!(parse(&vec![0; n]).is_err());
        }
    }
    #[test]
    fn rejects_reuse_without_triangle() {
        assert!(indices(&[0, 0, 0, 0, 0, 1, 0, 0, 0, 0], 3).is_err());
    }
}

#[cfg(test)]
mod static_strip_tests {
    use super::*;
    #[test]
    fn strips_preserve_winding_through_degenerate_joins() {
        assert_eq!(static_triangles(&[0,1,2,3,3,4,4,5,6,7],5).unwrap(), vec![[0,1,2],[2,1,3],[4,5,6],[6,5,7]]);
        assert_eq!(static_triangles(&[0,1,2,2,1,3],4).unwrap(),vec![[0,1,2],[2,1,3]]);
        assert!(static_triangles(&[0,1,2,3],4).is_err());
        assert!(static_triangles(&[0,1,2],6).is_err());
    }
}
