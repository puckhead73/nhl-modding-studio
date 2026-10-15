//! Export the fingerprinted vanilla Eichel donor, using validated native index
//! ownership (including the shared left/right eye buffer), without inventing a rig.
//! cargo run -p tdb-rx2 --example eichel_glb -- <native.mcd> <output-directory>

use serde_json::{json, Value};
use std::{error::Error, path::Path};
use tdb_rx2::{mcd, mcd_write::McdContainer, playermesh};

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 2 {
        return Err("usage: eichel_glb <native.mcd> <output-directory>".into());
    }
    let native = std::fs::read(&args[0])?;
    let validated = McdContainer::parse_with_profile(&native)?;
    let mut asset = mcd::parse_player_mesh(&native)?;
    if asset.submeshes.len() != validated.submeshes.len() {
        return Err("display/structural submesh count mismatch".into());
    }
    let mut manifest = Vec::new();
    for sub in &mut asset.submeshes {
        let i = validated
            .submesh_index(&sub.name)
            .ok_or("submesh name mismatch")?;
        let stream = validated.position_stream_of(i).ok_or("missing positions")?;
        let positions = validated.streams[stream].positions();
        if positions.len() != sub.vertices.len()
            || positions.iter().zip(&sub.vertices).any(|(a, b)| {
                a.iter()
                    .zip(b.position)
                    .any(|(x, y)| x.to_bits() != y.to_bits())
            })
        {
            return Err(format!("{}: display vertex order differs from native", sub.name).into());
        }
        // The old display decoder can choose the wrong left-eye buffer. Use the
        // profile's verified identity and decoded indices for every component.
        sub.triangles = validated.triangles_of(i).ok_or("missing triangles")?;
        manifest.push(json!({"name":sub.name,"vertices":sub.vertices.len(),
            "triangles":sub.triangles.len(),"native_position_stream":stream}));
    }
    let out = Path::new(&args[1]);
    std::fs::create_dir_all(out)?;
    for (name, selected) in [
        ("jack_eichel_native.glb", asset.clone()),
        (
            "jack_eichel_head_only.glb",
            playermesh::PlayerAsset {
                submeshes: asset
                    .submeshes
                    .iter()
                    .filter(|s| s.name == "geo_headShape")
                    .cloned()
                    .collect(),
                skin: playermesh::BonePalette { matrices: vec![] },
            },
        ),
    ] {
        let glb = with_normals(&selected)?;
        let file = out.join(name);
        std::fs::write(&file, &glb)?;
        println!(
            "{}: {} bytes, {} named mesh(es)",
            file.display(),
            glb.len(),
            selected.submeshes.len()
        );
    }
    std::fs::write(
        out.join("export.json"),
        serde_json::to_vec_pretty(&json!({
            "source_sha256":tdb_rx2::mcd_write::fingerprint(&native),
            "source":args[0], "submeshes":manifest,
            "coordinates":"native positions, Y-up, metres; identity GLB nodes; no centering or rescale",
            "vertex_order":"unchanged within each submesh; no welding; coincident lip IDs remain separate",
            "attributes":"POSITION, native TEXCOORD_0, normalized decoded NORMAL; no texture images",
            "rig":"none exported: no verified facial rig binding; native zero blend bytes are not invented weights",
            "head_only":"geo_headShape includes oral cavity, neck and upper torso; those regions are not removed"
        }))?,
    )?;
    Ok(())
}

// Reuse the existing geometry writer, adding decoded normals which it currently
// omits. Geometry-only exports deliberately omit skin nodes and placeholder weights.
fn with_normals(asset: &playermesh::PlayerAsset) -> Result<Vec<u8>, Box<dyn Error>> {
    let original = playermesh::write_glb_geometry_only(asset);
    let u32_at = |p| u32::from_le_bytes(original[p..p + 4].try_into().unwrap()) as usize;
    let json_len = u32_at(12);
    let mut doc: Value = serde_json::from_slice(&original[20..20 + json_len])?;
    let bin_header = 20 + json_len;
    let mut bin = original[bin_header + 8..bin_header + 8 + u32_at(bin_header)].to_vec();
    for (i, sub) in asset.submeshes.iter().enumerate() {
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }
        let start = bin.len();
        for v in &sub.vertices {
            let n = v.normal;
            let length = n.iter().map(|c| c * c).sum::<f32>().sqrt();
            if !length.is_finite() || length < 1e-8 {
                return Err(format!("{} has an invalid normal", sub.name).into());
            }
            for c in n {
                bin.extend_from_slice(&(c / length).to_le_bytes());
            }
        }
        let view = doc["bufferViews"].as_array().unwrap().len();
        doc["bufferViews"].as_array_mut().unwrap().push(json!({
            "buffer":0,"byteOffset":start,"byteLength":bin.len()-start,"target":34962
        }));
        let accessor = doc["accessors"].as_array().unwrap().len();
        doc["accessors"].as_array_mut().unwrap().push(json!({
            "bufferView":view,"componentType":5126,"count":sub.vertices.len(),"type":"VEC3"
        }));
        doc["meshes"][i]["primitives"][0]["attributes"]["NORMAL"] = json!(accessor);
        // Byte-level position verification against the native stream after GLB encoding.
        let a = doc["meshes"][i]["primitives"][0]["attributes"]["POSITION"]
            .as_u64()
            .unwrap() as usize;
        let v = doc["accessors"][a]["bufferView"].as_u64().unwrap() as usize;
        let off = doc["bufferViews"][v]["byteOffset"].as_u64().unwrap() as usize;
        for (j, vertex) in sub.vertices.iter().enumerate() {
            for k in 0..3 {
                assert_eq!(
                    &bin[off + j * 12 + k * 4..off + j * 12 + k * 4 + 4],
                    &vertex.position[k].to_le_bytes()
                );
            }
        }
    }
    doc["buffers"][0]["byteLength"] = json!(bin.len());
    let mut json_bytes = serde_json::to_vec(&doc)?;
    while !json_bytes.len().is_multiple_of(4) {
        json_bytes.push(b' ');
    }
    let len = 12 + 8 + json_bytes.len() + 8 + bin.len();
    let mut result = Vec::with_capacity(len);
    result.extend_from_slice(b"glTF");
    result.extend_from_slice(&2u32.to_le_bytes());
    result.extend_from_slice(&(len as u32).to_le_bytes());
    result.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
    result.extend_from_slice(b"JSON");
    result.extend(json_bytes);
    result.extend_from_slice(&(bin.len() as u32).to_le_bytes());
    result.extend_from_slice(b"BIN\0");
    result.extend(bin);
    assert_eq!(result.len(), len);
    Ok(result)
}
