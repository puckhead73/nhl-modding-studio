//! Validate `derive_tangents` against vanilla's OWN stored tangents.
//!
//! Self-consistency proves nothing: the packer derives and writes from the same UVs, so a
//! wrong convention stays invisible. The real question is whether the derived frame agrees
//! with the frame the shader expects — and vanilla answers it directly, because we can run
//! the generator on vanilla's shipped positions/normals/UVs and compare to the tangents EA
//! shipped alongside them.
//!
//! Also reports the derived-vs-stored angle under a flipped V, since a V-convention error
//! mirrors the tangent and is the single most likely way to get this wrong.

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn angle_deg(a: [f32; 3], b: [f32; 3]) -> f32 {
    let d = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
    d.acos().to_degrees()
}

fn report(label: &str, derived: &[[f32; 3]], stored: &[[f32; 3]]) {
    let mut ang: Vec<f32> = Vec::new();
    let mut flipped = 0usize;
    for (d, s) in derived.iter().zip(stored) {
        let sl = (s[0] * s[0] + s[1] * s[1] + s[2] * s[2]).sqrt();
        if sl < 0.2 {
            continue;
        }
        let a = angle_deg(norm(*d), norm(*s));
        if a > 90.0 {
            flipped += 1;
        }
        ang.push(a);
    }
    ang.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = ang.len();
    println!(
        "  {label:<22} n={n:<5} mean {:.1}deg  p50 {:.1}  p95 {:.1}   pointing backwards: {flipped}/{n} ({:.1}%)",
        ang.iter().sum::<f32>() / n as f32,
        ang[n / 2],
        ang[n * 95 / 100],
        100.0 * flipped as f32 / n as f32
    );
}

fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).unwrap();
        println!("\n=== {path}");
        for s in &asset.submeshes {
            if !s.name.starts_with("head_") && !s.name.starts_with("mouthbag_") {
                continue;
            }
            let pos: Vec<[f32; 3]> = s.vertices.iter().map(|v| v.position).collect();
            let nrm: Vec<[f32; 3]> = s.vertices.iter().map(|v| norm(v.normal)).collect();
            let uv: Vec<[f32; 2]> = s.vertices.iter().map(|v| v.uv).collect();
            let uvf: Vec<[f32; 2]> = uv.iter().map(|u| [u[0], 1.0 - u[1]]).collect();
            let tris: Vec<[u32; 3]> = s.triangles.iter().map(|t| [t[0], t[1], t[2]]).collect();

            // Stored tangents, decoded from the packed 2:10:10:10 word at offset 12.
            let enc = |p: [f32; 3]| {
                let mut b = Vec::new();
                for c in p {
                    b.extend_from_slice(&tdb_rx2::playermesh::f32_to_half_be(c));
                }
                b
            };
            let (v0, v1) = (enc(pos[0]), enc(pos[1]));
            let stride = s.stride;
            let Some(base) = (0..data.len().saturating_sub(stride * 2))
                .find(|&o| data[o..o + 6] == v0[..] && data[o + stride..o + stride + 6] == v1[..])
            else {
                continue;
            };
            let dec = |w: u32, sh: u32| {
                let v = ((w >> sh) & 0x3FF) as i32;
                let v = if v >= 512 { v - 1024 } else { v };
                v as f32 / 511.0
            };
            let mut stored = Vec::with_capacity(pos.len());
            for i in 0..pos.len() {
                let o = base + i * stride;
                let t = u32::from_be_bytes(data[o + 12..o + 16].try_into().unwrap());
                stored.push([dec(t, 0), dec(t, 10), dec(t, 20)]);
            }

            println!("  -- {} ({} verts)", s.name, pos.len());
            report(
                "derived (UV as stored)",
                &tdb_rx2::pack::derive_tangents(&pos, &nrm, &uv, &tris),
                &stored,
            );
            report(
                "derived (V flipped)",
                &tdb_rx2::pack::derive_tangents(&pos, &nrm, &uvf, &tris),
                &stored,
            );
        }
    }
}
