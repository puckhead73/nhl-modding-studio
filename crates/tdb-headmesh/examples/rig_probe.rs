//! Inspect what a CC5 GLB actually contains: joint count, bind-pose sanity, submesh roles,
//! and whether the extractor reads it as an articulated rig or a stub.
//!
//! The distinction decides everything downstream — an articulated export uses its authored
//! weights, a stub falls back to donor transfer — so this is the first thing to run against
//! a new source.
fn main() {
    let p = std::env::args().nth(1).unwrap();
    let data = std::fs::read(&p).unwrap();
    let mesh = tdb_headmesh::glb::parse_glb(&data).unwrap();
    println!("joints {}", mesh.joint_names.len());
    let mut worst = 0f32;
    for m in &mesh.joint_bind_matrices {
        for (r, cells) in m.iter().enumerate() {
            for (c, cell) in cells.iter().enumerate() {
                let want = if r == c { 1.0 } else { 0.0 };
                worst = worst.max((cell - want).abs());
            }
        }
    }
    println!("max identity deviation across bind matrices: {worst:.4}");
    for pr in &mesh.primitives {
        let role = tdb_headmesh::ir::SubmeshRole::classify(&pr.name);
        println!("  {:<34} {:?} verts {}", pr.name, role, pr.positions.len());
    }
    match tdb_headmesh::extract::extract_cc5_head(&data, vec![]) {
        Ok((ir, rep)) => {
            let h = ir.head().unwrap();
            let b = tdb_headmesh::placement::bounds(&h.positions).unwrap();
            println!(
                "\nextract OK: head {} verts, model-space {} (dev {:.2e}), rigid {}\n  bounds {:?} .. {:?}",
                h.positions.len(), rep.positions_were_model_space, rep.max_identity_deviation,
                rep.rig_is_rigid, b.0, b.1
            );
        }
        Err(e) => println!("\nextract FAILED: {e}"),
    }
}
