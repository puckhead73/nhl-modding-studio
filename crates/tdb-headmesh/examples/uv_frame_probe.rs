//! Per-triangle UV handedness, texel density and stretch anisotropy.
//!
//! The vertex format has no bitangent-sign field (vanilla leaves the 2 spare bits of the
//! tangent word at zero), so the shader must build the bitangent as `cross(N,T)` with one
//! fixed handedness for the whole mesh. Any triangle whose UV winding is mirrored relative
//! to the majority therefore gets its tangent-space detail lighting inverted.
//!
//! Anisotropy matters for the same shader: a shared detail-normal map sampled through a
//! strongly stretched UV smears directionally, which reads as striping.

fn stats(mut v: Vec<f32>) -> (f32, f32, f32) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    (
        v[n / 2],
        v[(n * 95 / 100).min(n - 1)],
        v[(n * 99 / 100).min(n - 1)],
    )
}

fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).unwrap();
        println!("\n=== {path}");
        for s in &asset.submeshes {
            if !s.name.starts_with("head_") {
                continue;
            }
            let pos: Vec<[f32; 3]> = s.vertices.iter().map(|v| v.position).collect();
            let uv: Vec<[f32; 2]> = s.vertices.iter().map(|v| v.uv).collect();

            let (mut pos_w, mut neg_w) = (0usize, 0usize);
            let mut dens = Vec::new();
            let mut aniso = Vec::new();
            for t in &s.triangles {
                let (i0, i1, i2) = (t[0] as usize, t[1] as usize, t[2] as usize);
                let (a, b, c) = (uv[i0], uv[i1], uv[i2]);
                let uv_area = (b[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (b[1] - a[1]);
                if uv_area > 0.0 {
                    pos_w += 1;
                } else if uv_area < 0.0 {
                    neg_w += 1;
                }
                let (p0, p1, p2) = (pos[i0], pos[i1], pos[i2]);
                let e1 = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
                let e2 = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
                let cr = [
                    e1[1] * e2[2] - e1[2] * e2[1],
                    e1[2] * e2[0] - e1[0] * e2[2],
                    e1[0] * e2[1] - e1[1] * e2[0],
                ];
                let area3 = 0.5 * (cr[0] * cr[0] + cr[1] * cr[1] + cr[2] * cr[2]).sqrt();
                if area3 < 1e-12 || uv_area.abs() < 1e-12 {
                    continue;
                }
                // texels per mm, on a 512 atlas
                dens.push((uv_area.abs() * 0.5).sqrt() * 512.0 / (area3.sqrt() * 1000.0));

                // Stretch anisotropy: singular values of the UV -> 3D map.
                let du1 = b[0] - a[0];
                let dv1 = b[1] - a[1];
                let du2 = c[0] - a[0];
                let dv2 = c[1] - a[1];
                let det = du1 * dv2 - du2 * dv1;
                if det.abs() < 1e-12 {
                    continue;
                }
                let r = 1.0 / det;
                let ts = [
                    (dv2 * e1[0] - dv1 * e2[0]) * r,
                    (dv2 * e1[1] - dv1 * e2[1]) * r,
                    (dv2 * e1[2] - dv1 * e2[2]) * r,
                ];
                let bs = [
                    (du1 * e2[0] - du2 * e1[0]) * r,
                    (du1 * e2[1] - du2 * e1[1]) * r,
                    (du1 * e2[2] - du2 * e1[2]) * r,
                ];
                let lt = (ts[0] * ts[0] + ts[1] * ts[1] + ts[2] * ts[2]).sqrt();
                let lb = (bs[0] * bs[0] + bs[1] * bs[1] + bs[2] * bs[2]).sqrt();
                if lt > 0.0 && lb > 0.0 {
                    aniso.push(lt.max(lb) / lt.min(lb));
                }
            }
            let (dm, d95, d99) = stats(dens);
            let (am, a95, a99) = stats(aniso);
            let tot = pos_w + neg_w;
            println!("  {} tris={tot}", s.name);
            println!(
                "    UV winding: positive {pos_w} ({:.1}%)  negative {neg_w} ({:.1}%)  -> minority {} tris",
                100.0 * pos_w as f32 / tot as f32,
                100.0 * neg_w as f32 / tot as f32,
                pos_w.min(neg_w)
            );
            println!("    texel density  p50 {dm:.2}  p95 {d95:.2}  p99 {d99:.2} texels/mm");
            println!("    UV anisotropy  p50 {am:.2}  p95 {a95:.2}  p99 {a99:.2}");
        }
    }
}
