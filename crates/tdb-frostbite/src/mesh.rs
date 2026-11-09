//! Measured NHL22–24 PS4 MeshSet layout (0xb0 LOD / 0xe0 section records).
//! Geometry and UV0 only; skinning and material bindings are not inferred.
use crate::{cstr, hex, invalid, le, slice, Result};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct Section {
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub uv0: Option<Vec<[f32; 2]>>,
    pub indices: Vec<u32>,
}
#[derive(Debug, Serialize)]
pub struct Lod {
    pub level: usize,
    pub chunk: String,
    pub sections: Vec<Section>,
}
fn ptr(b: &[u8], p: usize) -> Result<usize> {
    usize::try_from(u64::from_le_bytes(slice(b, p, 8)?.try_into().unwrap()))
        .map_err(|_| invalid("mesh pointer overflow"))
}
fn u16le(b: &[u8], p: usize) -> Result<u16> {
    Ok(u16::from_le_bytes(slice(b, p, 2)?.try_into().unwrap()))
}
fn float(b: &[u8], p: usize) -> Result<f32> {
    let v = f32::from_bits(le(b, p)?);
    if !v.is_finite() {
        return Err(invalid("non-finite mesh coordinate"));
    }
    Ok(v)
}
fn half(b: &[u8], p: usize) -> Result<f32> {
    let h = u16le(b, p)?;
    let sign = if h & 0x8000 == 0 { 1.0 } else { -1.0 };
    let exponent = (h >> 10) & 31;
    let mantissa = (h & 1023) as f32;
    if exponent == 31 {
        return Err(invalid("non-finite half coordinate"));
    }
    Ok(sign
        * if exponent == 0 {
            mantissa * 2f32.powi(-24)
        } else {
            (1.0 + mantissa / 1024.0) * 2f32.powi(exponent as i32 - 15)
        })
}

pub fn parse(
    data: &[u8],
    metadata: &str,
    mut read_chunk: impl FnMut(&str) -> Result<Vec<u8>>,
) -> Result<Vec<Lod>> {
    if metadata.len() != 32
        || !metadata.is_ascii()
        || !(metadata.ends_with("b000e000") || metadata.ends_with("a000e000"))
    {
        return Err(invalid("unsupported MeshSet record layout"));
    }
    let meta = (0..16)
        .map(|i| {
            u8::from_str_radix(&metadata[i * 2..i * 2 + 2], 16)
                .map_err(|_| invalid("invalid resource metadata"))
        })
        .collect::<Result<Vec<_>>>()?;
    let count = u16le(data, 0x94)? as usize;
    if !(1..=6).contains(&count) {
        return Err(invalid("unsupported MeshSet LOD count"));
    }
    let mut lods = Vec::new();
    let mut decoded_bytes = 0usize;
    for level in 0..count {
        let lod = ptr(data, 0x20 + level * 8)?;
        slice(data, lod, 0xb0)?;
        if le(data, lod + 0x54)? != 0x21 {
            return Err(invalid("unsupported MeshSet index declaration"));
        }
        let section_count = le(data, lod + 8)? as usize;
        let section_table = ptr(data, lod + 12)?;
        if section_count > 1024 {
            return Err(invalid("excessive mesh sections"));
        }
        slice(data, section_table, section_count * 0xe0)?;
        let index_bytes = le(data, lod + 0x58)? as usize;
        let vertex_bytes = le(data, lod + 0x5c)? as usize;
        let guid = hex(slice(data, lod + 0x60, 16)?);
        let buffer = if guid == "00000000000000000000000000000000" {
            let start = le(&meta, 0)? as usize;
            let size = le(&meta, 4)? as usize;
            let offset = le(data, lod + 0x70)? as usize;
            slice(
                slice(data, start, size)?,
                offset,
                vertex_bytes + index_bytes,
            )?
            .to_vec()
        } else {
            read_chunk(&guid)?
        };
        slice(
            &buffer,
            0,
            vertex_bytes
                .checked_add(index_bytes)
                .ok_or_else(|| invalid("mesh size overflow"))?,
        )?;
        let mut sections = Vec::new();
        for index in 0..section_count {
            let s = section_table + index * 0xe0;
            let triangles = le(data, s + 0x20)? as usize;
            let first_index = le(data, s + 0x24)? as usize;
            let vertex_start = le(data, s + 0x28)? as usize;
            let vertices = le(data, s + 0x2c)? as usize;
            if vertices > 1_000_000 || triangles > 2_000_000 {
                return Err(invalid("mesh geometry exceeds limits"));
            }
            decoded_bytes += vertices * 20 + triangles * 12;
            if decoded_bytes > 256 * 1024 * 1024 {
                return Err(invalid("decoded mesh exceeds 256 MiB limit"));
            }
            let elements = data[s + 0xac] as usize;
            let streams = data[s + 0xad] as usize;
            if !(1..=16).contains(&elements) || !(1..=16).contains(&streams) {
                return Err(invalid("unsupported mesh vertex declaration"));
            }
            let strides: Vec<usize> = (0..streams)
                .map(|i| data[s + 0x8c + i * 2] as usize)
                .collect();
            let stride: usize = strides.iter().sum();
            if vertex_start + vertices * stride > vertex_bytes {
                return Err(invalid("vertex streams exceed declared buffer"));
            }
            let mut positions = None;
            let mut uv0 = None;
            for element in 0..elements {
                let e = s + 0x4c + element * 4;
                let (usage, format, offset, stream) = (
                    data[e],
                    data[e + 1],
                    data[e + 2] as usize,
                    data[e + 3] as usize,
                );
                if usage != 1 && usage != 0x21 {
                    continue;
                }
                if stream >= streams {
                    return Err(invalid("vertex element stream out of range"));
                }
                let size = match (usage, format) {
                    (1, 3) => 12,
                    (1, 8) => 8,
                    (0x21, 6) => 4,
                    (0x21, 2) => 8,
                    _ => {
                        return Err(invalid(format!(
                            "unsupported vertex usage/format {usage:#x}/{format:#x}"
                        )))
                    }
                };
                if offset + size > strides[stream] {
                    return Err(invalid("vertex element exceeds stream stride"));
                }
                let start =
                    vertex_start + vertices * strides[..stream].iter().sum::<usize>() + offset;
                if usage == 1 {
                    if positions.is_some() {
                        return Err(invalid("duplicate position declaration"));
                    }
                    positions = Some(
                        (0..vertices)
                            .map(|i| {
                                let p = start + i * strides[stream];
                                if format == 8 {
                                    Ok([
                                        half(&buffer, p)?,
                                        half(&buffer, p + 2)?,
                                        half(&buffer, p + 4)?,
                                    ])
                                } else {
                                    Ok([
                                        float(&buffer, p)?,
                                        float(&buffer, p + 4)?,
                                        float(&buffer, p + 8)?,
                                    ])
                                }
                            })
                            .collect::<Result<Vec<_>>>()?,
                    );
                } else {
                    if uv0.is_some() {
                        return Err(invalid("duplicate UV0 declaration"));
                    }
                    uv0 = Some(
                        (0..vertices)
                            .map(|i| {
                                let p = start + i * strides[stream];
                                if format == 6 {
                                    Ok([half(&buffer, p)?, half(&buffer, p + 2)?])
                                } else {
                                    Ok([float(&buffer, p)?, float(&buffer, p + 4)?])
                                }
                            })
                            .collect::<Result<Vec<_>>>()?,
                    );
                }
            }
            let positions = positions.ok_or_else(|| invalid("missing position declaration"))?;
            if first_index * 2 + triangles * 6 > index_bytes {
                return Err(invalid("triangle indices exceed declared buffer"));
            }
            let indices = (0..triangles * 3)
                .map(|i| {
                    let v = u16le(&buffer, vertex_bytes + (first_index + i) * 2)? as u32;
                    if v as usize >= vertices {
                        return Err(invalid("triangle vertex index out of range"));
                    }
                    Ok(v)
                })
                .collect::<Result<Vec<_>>>()?;
            sections.push(Section {
                name: cstr(data, ptr(data, s + 8)?)?,
                positions,
                uv0,
                indices,
            });
        }
        lods.push(Lod {
            level,
            chunk: guid,
            sections,
        });
    }
    Ok(lods)
}

/// One GLB per LOD. UV-less depth sections remain UV-less.
pub fn glb(lod: &Lod) -> Result<Vec<u8>> {
    use serde_json::json;
    let mut binary = Vec::<u8>::new();
    let mut views = Vec::new();
    let mut accessors = Vec::new();
    let mut meshes = Vec::new();
    let mut nodes = Vec::new();
    for section in &lod.sections {
        if section.positions.is_empty() || section.indices.is_empty() {
            continue;
        }
        let start = binary.len();
        for v in &section.positions {
            for c in v {
                binary.extend_from_slice(&c.to_le_bytes());
            }
        }
        views.push(
            json!({"buffer":0,"byteOffset":start,"byteLength":binary.len()-start,"target":34962}),
        );
        let min: Vec<_> = (0..3)
            .map(|a| {
                section
                    .positions
                    .iter()
                    .map(|v| v[a])
                    .fold(f32::INFINITY, f32::min)
            })
            .collect();
        let max: Vec<_> = (0..3)
            .map(|a| {
                section
                    .positions
                    .iter()
                    .map(|v| v[a])
                    .fold(f32::NEG_INFINITY, f32::max)
            })
            .collect();
        let position = accessors.len();
        accessors.push(json!({"bufferView":views.len()-1,"componentType":5126,"count":section.positions.len(),"type":"VEC3","min":min,"max":max}));
        let mut attributes = json!({"POSITION":position});
        if let Some(uv) = &section.uv0 {
            let start = binary.len();
            for v in uv {
                for c in v {
                    binary.extend_from_slice(&c.to_le_bytes());
                }
            }
            views.push(json!({"buffer":0,"byteOffset":start,"byteLength":binary.len()-start,"target":34962}));
            attributes["TEXCOORD_0"] = json!(accessors.len());
            accessors.push(json!({"bufferView":views.len()-1,"componentType":5126,"count":uv.len(),"type":"VEC2"}));
        }
        let start = binary.len();
        for i in &section.indices {
            binary.extend_from_slice(&i.to_le_bytes());
        }
        views.push(
            json!({"buffer":0,"byteOffset":start,"byteLength":binary.len()-start,"target":34963}),
        );
        let indices = accessors.len();
        accessors.push(json!({"bufferView":views.len()-1,"componentType":5125,"count":section.indices.len(),"type":"SCALAR"}));
        nodes.push(json!({"name":section.name,"mesh":meshes.len()}));
        meshes.push(json!({"name":section.name,"primitives":[{"attributes":attributes,"indices":indices,"mode":4}]}));
    }
    if nodes.is_empty() {
        return Err(invalid("LOD contains no triangles"));
    }
    let document = json!({"asset":{"version":"2.0","generator":"NHL Modding Studio"},
        "scene":0,"scenes":[{"nodes":(0..nodes.len()).collect::<Vec<_>>()}],
        "nodes":nodes,"meshes":meshes,"buffers":[{"byteLength":binary.len()}],
        "bufferViews":views,"accessors":accessors});
    let mut json = serde_json::to_vec(&document).map_err(|e| invalid(e.to_string()))?;
    while json.len() % 4 != 0 {
        json.push(b' ');
    }
    let length = 28usize + json.len() + binary.len();
    let mut out = Vec::new();
    for word in [
        0x46546c67u32,
        2,
        u32::try_from(length).map_err(|_| invalid("GLB too large"))?,
        json.len() as u32,
        0x4e4f534a,
    ] {
        out.extend_from_slice(&word.to_le_bytes());
    }
    out.extend_from_slice(&json);
    out.extend_from_slice(&(binary.len() as u32).to_le_bytes());
    out.extend_from_slice(&0x004e4942u32.to_le_bytes());
    out.extend_from_slice(&binary);
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_half_positions_uvs_and_index_bounds_in_both_headers() {
        for header in [0xa0usize, 0xb0] {
            let section = header + 0xb0;
            let mut data = vec![0u8; 0x300 + 42];
            for (offset, value) in [
                (0x20, header as u32),
                (0x94, 1),
                (header + 8, 1),
                (header + 12, section as u32),
                (header + 0x54, 0x21),
                (header + 0x58, 6),
                (header + 0x5c, 36),
                (section + 8, 0x250),
                (section + 0x20, 1),
                (section + 0x2c, 3),
            ] {
                data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
            }
            data[0x250..0x255].copy_from_slice(b"head\0");
            data[section + 0x4c..section + 0x54].copy_from_slice(&[1, 8, 0, 0, 0x21, 6, 0, 1]);
            data[section + 0x8c..section + 0x90].copy_from_slice(&[8, 0, 4, 0]);
            data[section + 0xac..section + 0xae].copy_from_slice(&[2, 2]);
            let words = [
                0u16, 0, 0, 0x3c00, 0x3c00, 0, 0, 0x3c00, 0, 0x3c00, 0, 0x3c00, 0, 0, 0x3c00, 0, 0,
                0x3c00, 0, 1, 2,
            ];
            for (i, value) in words.iter().enumerate() {
                data[0x300 + i * 2..0x302 + i * 2].copy_from_slice(&value.to_le_bytes());
            }
            let metadata = format!("000300002a00000000000000{header:02x}00e000");
            let lods = parse(&data, &metadata, |_| {
                panic!("inline data must not read a CAS chunk")
            })
            .unwrap();
            assert_eq!(
                lods[0].sections[0].positions,
                vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]]
            );
            assert_eq!(
                lods[0].sections[0].uv0.as_ref().unwrap(),
                &vec![[0., 0.], [1., 0.], [0., 1.]]
            );
            data[0x300 + 40..0x300 + 42].copy_from_slice(&3u16.to_le_bytes());
            assert!(parse(&data, &metadata, |_| panic!("inline")).is_err());
        }
    }

    #[test]
    fn rejects_unqualified_layout_before_chunk_access() {
        assert!(parse(&[], "", |_| panic!("unexpected chunk access")).is_err());
    }
    #[test]
    fn glb_preserves_uvless_sections() {
        let lod = Lod {
            level: 0,
            chunk: String::new(),
            sections: vec![Section {
                name: "depth".into(),
                positions: vec![[0., 0., 0.], [1., 0., 0.], [0., 1., 0.]],
                uv0: None,
                indices: vec![0, 1, 2],
            }],
        };
        let bytes = glb(&lod).unwrap();
        assert_eq!(le(&bytes, 8).unwrap() as usize, bytes.len());
        let json: serde_json::Value =
            serde_json::from_slice(&bytes[20..20 + le(&bytes, 12).unwrap() as usize]).unwrap();
        assert!(json["meshes"][0]["primitives"][0]["attributes"]
            .get("TEXCOORD_0")
            .is_none());
    }
}
