//! Measure vertex-scale surface roughness, and the fp16 W component.
//!
//! Magnitude of normal error is not what the eye sees — spatial frequency is. A smooth
//! 3-degree difference between an authored normal and the geometry is invisible; 3 degrees
//! of *uncorrelated vertex-to-vertex* noise reads as striping. This probe reports, for the
//! surface as actually shipped, the angle between each vertex's geometric normal and the
//! mean of its 1-ring neighbours — a direct measure of high-frequency roughness.

use std::collections::{HashMap, HashSet};

fn angle_deg(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    d.acos().to_degrees()
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn stats(mut v: Vec<f32>) -> (f32, f32, f32) {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    (
        v.iter().sum::<f32>() / n as f32,
        v[n / 2],
        v[(n * 95 / 100).min(n - 1)],
    )
}

fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).expect("read");
        let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).expect("parse");
        println!("\n=== {path}");
        for s in &asset.submeshes {
            if !s.name.starts_with("head_") {
                continue;
            }
            let pos: Vec<[f32; 3]> = s.vertices.iter().map(|v| v.position).collect();
            let tris: Vec<[u32; 3]> = s.triangles.iter().map(|t| [t[0], t[1], t[2]]).collect();

            // Weld coincident positions so the 1-ring crosses UV seams.
            let mut rep: HashMap<[u32; 3], u32> = HashMap::new();
            let mut wid = vec![0u32; pos.len()];
            for (i, p) in pos.iter().enumerate() {
                let k = [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
                let e = *rep.entry(k).or_insert(i as u32);
                wid[i] = e;
            }
            let mut ring: HashMap<u32, HashSet<u32>> = HashMap::new();
            for t in &tris {
                for k in 0..3 {
                    let a = wid[t[k] as usize];
                    let b = wid[t[(k + 1) % 3] as usize];
                    if a != b {
                        ring.entry(a).or_default().insert(b);
                        ring.entry(b).or_default().insert(a);
                    }
                }
            }

            let geo = tdb_headmesh::pack_rw::smooth_normals(&pos, &tris);
            let stored: Vec<[f32; 3]> = s.vertices.iter().map(|v| norm(v.normal)).collect();

            // Roughness of the shipped surface, and of the shipped normal field.
            let mut geo_rough = Vec::new();
            let mut stored_rough = Vec::new();
            for (v, nb) in &ring {
                if nb.len() < 3 {
                    continue;
                }
                let mut ag = [0.0f32; 3];
                let mut ast = [0.0f32; 3];
                for &j in nb {
                    for c in 0..3 {
                        ag[c] += geo[j as usize][c];
                        ast[c] += stored[j as usize][c];
                    }
                }
                geo_rough.push(angle_deg(geo[*v as usize], norm(ag)));
                stored_rough.push(angle_deg(stored[*v as usize], norm(ast)));
            }
            let (gm, gp50, gp95) = stats(geo_rough);
            let (sm, sp50, sp95) = stats(stored_rough);

            println!("  {} verts={} welded={}", s.name, pos.len(), ring.len());
            println!(
                "    geometry roughness (1-ring): mean {gm:.2}deg  p50 {gp50:.2}  p95 {gp95:.2}"
            );
            println!(
                "    normal-field roughness     : mean {sm:.2}deg  p50 {sp50:.2}  p95 {sp95:.2}"
            );

            // fp16 W at offset 6 — constant 1.0 here, but is it in vanilla?
            let enc = |p: [f32; 3]| {
                let mut b = Vec::new();
                for c in p {
                    b.extend_from_slice(&tdb_rx2::playermesh::f32_to_half_be(c));
                }
                b
            };
            let v0 = enc(pos[0]);
            let v1 = enc(pos[1]);
            let stride = s.stride;
            if let Some(base) = (0..data.len().saturating_sub(stride * 2))
                .find(|&o| data[o..o + 6] == v0[..] && data[o + stride..o + stride + 6] == v1[..])
            {
                let mut ws: HashMap<u16, usize> = HashMap::new();
                for i in 0..pos.len() {
                    let o = base + i * stride;
                    if o + 8 > data.len() {
                        break;
                    }
                    *ws.entry(u16::from_be_bytes([data[o + 6], data[o + 7]]))
                        .or_default() += 1;
                }
                let mut rows: Vec<_> = ws.into_iter().collect();
                rows.sort_by_key(|r| std::cmp::Reverse(r.1));
                print!("    position.w halfs:");
                for (v, c) in rows.iter().take(5) {
                    print!(" {v:04X}x{c}");
                }
                println!(" ({} distinct)", rows.len());
            }
        }
    }
}
