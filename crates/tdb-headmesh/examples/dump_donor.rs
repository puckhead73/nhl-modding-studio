//! Inspect an RW player-head container: submeshes, roles, bounds, UV extents and the
//! palette mass each submesh weights.
//!
//! Used to characterise a donor before a conversion and to verify a converted
//! `player_<id>.rx2` afterwards (the packed head should read back with the source's
//! vertex/triangle counts, the source UV extents, and palette usage contained inside the
//! donor's own used set).

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: dump_donor <player_*.rx2>");
    let data = std::fs::read(&path).expect("read");
    let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).expect("parse");
    println!("file: {path}");
    println!("palette matrices: {}", asset.skin.matrices.len());
    for s in &asset.submeshes {
        let role = tdb_headmesh::ir::SubmeshRole::classify(&s.name);
        let mut mn = [f32::MAX; 3];
        let mut mx = [f32::MIN; 3];
        let mut uvmn = [f32::MAX; 2];
        let mut uvmx = [f32::MIN; 2];
        for v in &s.vertices {
            for k in 0..3 {
                mn[k] = mn[k].min(v.position[k]);
                mx[k] = mx[k].max(v.position[k]);
            }
            for k in 0..2 {
                uvmn[k] = uvmn[k].min(v.uv[k]);
                uvmx[k] = uvmx[k].max(v.uv[k]);
            }
        }
        println!(
            "\n  {:<16} role={:?} verts={} tris={}",
            s.name,
            role,
            s.vertices.len(),
            s.triangles.len()
        );
        println!("    pos  {mn:?} .. {mx:?}");
        println!("    uv   {uvmn:?} .. {uvmx:?}");
        // Palette usage: BLENDINDICES are palette*3.
        let mut mass: std::collections::BTreeMap<u16, f32> = Default::default();
        for v in &s.vertices {
            for i in 0..4 {
                let w = v.weights[i];
                if w > 0.0 {
                    *mass.entry(v.joints[i] / 3).or_default() += w;
                }
            }
        }
        let mut rows: Vec<_> = mass.into_iter().collect();
        rows.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        let total: f32 = rows.iter().map(|r| r.1).sum();
        print!("    palette mass:");
        for (slot, m) in rows.iter().take(12) {
            print!(" {slot}={:.1}%", 100.0 * m / total);
        }
        println!(" ({} slots used)", rows.len());
    }
}
