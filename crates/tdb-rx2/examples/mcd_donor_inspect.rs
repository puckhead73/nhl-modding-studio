//! Read-only structural/display agreement and native geometry evidence for a new donor.
use std::{error::Error, fs};
use tdb_rx2::{
    mcd,
    mcd_write::{fingerprint, McdContainer},
};
fn main() -> Result<(), Box<dyn Error>> {
    let a: Vec<String> = std::env::args().collect();
    if a.len() != 3 {
        return Err("usage: mcd_donor_inspect input.mcd output.json".into());
    }
    let bytes = fs::read(&a[1])?;
    let c = McdContainer::parse(&bytes)?;
    let display = mcd::parse_player_mesh(&bytes)?;
    let mut subs = Vec::new();
    for (i, s) in c.submeshes.iter().enumerate() {
        let d = display
            .submeshes
            .iter()
            .find(|d| d.name == s.name)
            .ok_or("missing display mesh")?;
        let p = c.streams[c.position_stream_of(i).ok_or("positions")?].positions();
        assert_eq!(p, d.vertices.iter().map(|v| v.position).collect::<Vec<_>>());
        let triangles = c.triangles_of(i).ok_or("triangles")?;
        assert_eq!(triangles.len(), s.triangles);
        subs.push(serde_json::json!({"name":s.name,"positions":p,"original_positions":p,"triangles":triangles,"uv":d.vertices.iter().map(|v|v.uv).collect::<Vec<_>>(),"normals":d.vertices.iter().map(|v|v.normal).collect::<Vec<_>>(),"index_chunk":s.index_chunk,"shares_index_buffer":s.shares_index_buffer,"format":s.format,"streams":s.streams.iter().map(|&j|serde_json::json!({"count":c.streams[j].count,"stride":c.streams[j].stride,"data_off":c.streams[j].data_off})).collect::<Vec<_>>()}));
        println!(
            "{} {}v {}t fmt{} index{} shared{} streams{:?}",
            s.name,
            p.len(),
            s.triangles,
            s.format,
            s.index_chunk,
            s.shares_index_buffer,
            s.streams
                .iter()
                .map(|&j| (c.streams[j].count, c.streams[j].stride))
                .collect::<Vec<_>>()
        );
    }
    fs::write(
        &a[2],
        serde_json::to_vec(
            &serde_json::json!({"donor_sha256":fingerprint(&bytes),"size":bytes.len(),"submeshes":subs}),
        )?,
    )?;
    Ok(())
}
