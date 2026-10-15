//! Profile-gated, all-submesh positional pack with a byte-layout receipt for shading.
use serde::Deserialize;
use serde_json::json;
use std::{error::Error, path::Path};
use tdb_rx2::{
    mcd,
    mcd_write::{fingerprint, McdContainer},
};

#[derive(Deserialize)]
struct Mesh {
    name: String,
    original_positions: Vec<[f32; 3]>,
    positions: Vec<[f32; 3]>,
    triangles: Vec<[u32; 3]>,
    uv: Vec<[f32; 2]>,
}
#[derive(Deserialize)]
struct Input {
    donor_sha256: String,
    submeshes: Vec<Mesh>,
}
fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 3 {
        return Err("usage: mcd_game_test_pack donor.mcd native_mesh.json output_prefix".into());
    }
    let data = std::fs::read(&args[0])?;
    let input: Input = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    if !tdb_rx2::mcd_write::KNOWN_DONORS
        .iter()
        .any(|p| p.sha256 == input.donor_sha256)
    {
        return Err("wrong fit origin".into());
    }
    let mut c = McdContainer::parse_with_profile(&data)?;
    let decoded = mcd::parse_player_mesh(&data)?;
    if input.submeshes.len() != c.submeshes.len() {
        return Err("submesh count mismatch".into());
    }
    let mut names = std::collections::BTreeSet::new();
    let mut layout = Vec::new();
    for mesh in &input.submeshes {
        if !names.insert(&mesh.name) {
            return Err("duplicate mesh".into());
        }
        let i = c.submesh_index(&mesh.name).ok_or("unknown submesh")?;
        let s = c.position_stream_of(i).ok_or("no position stream")?;
        if c.streams[s].positions() != mesh.original_positions
            || c.triangles_of(i).as_ref() != Some(&mesh.triangles)
        {
            return Err(format!("native order/topology mismatch {}", mesh.name).into());
        }
        let display = decoded
            .submeshes
            .iter()
            .find(|m| m.name == mesh.name)
            .ok_or("missing display mesh")?;
        if display.vertices.len() != mesh.uv.len()
            || display
                .vertices
                .iter()
                .zip(&mesh.uv)
                .any(|(v, uv)| v.uv.iter().zip(uv).any(|(a, b)| (a - b).abs() > 1e-7))
        {
            return Err("native UV mismatch".into());
        }
        let shading_stream = if mesh.name.contains("eyeShape") {
            c.submeshes[i].streams[1]
        } else {
            s
        };
        let st = &c.streams[shading_stream];
        let n_off = if shading_stream == s { 12 } else { 0 };
        layout.push(json!({"name":mesh.name,"count":st.count,"stride":st.stride,"data_off":st.data_off,"normal_offset":n_off,"tangent_offset":n_off+8,"format_index":c.submeshes[i].format}));
        c.set_positions(s, &mesh.positions)?;
        if !c.refresh_bounds(i) {
            return Err("no submesh bounds".into());
        }
    }
    let packed = c.serialize();
    let spans = c.writable_spans();
    for (i, (a, b)) in data.iter().zip(&packed).enumerate() {
        if a != b && !spans.iter().any(|&(l, h)| i >= l && i < h) {
            return Err("opaque byte changed".into());
        }
    }
    let check = McdContainer::parse(&packed)?;
    for m in &input.submeshes {
        let i = check.submesh_index(&m.name).ok_or("reparse mesh")?;
        if check.streams[check.position_stream_of(i).ok_or("reparse positions")?].positions()
            != m.positions
        {
            return Err("reparse positions changed".into());
        }
    }
    if let Some(p) = Path::new(&args[2]).parent() {
        std::fs::create_dir_all(p)?;
    }
    std::fs::write(format!("{}.positions.mcd", args[2]), &packed)?;
    std::fs::write(
        format!("{}.layout.json", args[2]),
        serde_json::to_vec_pretty(
            &json!({"donor_sha256":fingerprint(&data),"position_sha256":fingerprint(&packed),"streams":layout,"allowed_spans":spans}),
        )?,
    )?;
    println!(
        "PACKED {} bytes, {} verified submeshes; {}",
        packed.len(),
        input.submeshes.len(),
        fingerprint(&packed)
    );
    Ok(())
}
