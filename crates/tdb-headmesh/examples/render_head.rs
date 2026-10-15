//! Software-rasterise a converted head so shading can be inspected without the game.
//!
//! Every statistical check can say "as good as vanilla" while the asset still shades
//! badly, because statistics average away exactly the spatial structure the eye picks up.
//! This renders the shipped vertex data directly — same normals, same UVs, same texture
//! the console would use — under a plain directional light.
//!
//! Modes isolate the inputs:
//!   `full`    texture x lambert          (what the game roughly shows)
//!   `normals` white albedo x lambert     (the normal field alone)
//!   `albedo`  texture, no lighting       (the texture alone)
//!   `facet`   per-face geometric normal  (the shipped geometry alone)
//!
//! usage: render_head <player.rx2> <texlib.rx2> <out.png> [mode] [spec]

fn decode_head_texture(path: &str) -> (Vec<u8>, u32, u32) {
    let bytes = std::fs::read(path).unwrap();
    let lib = tdb_rx2::texlib::parse(&bytes).unwrap();
    let idx = lib
        .textures
        .iter()
        .position(|t| t.name.to_ascii_lowercase().contains("head"))
        .expect("head slot");
    let t = &lib.textures[idx];
    let mut tiled = lib.base_mip_tiled(idx).unwrap().to_vec();
    tdb_rx2::texlib::swap16_in_place(&mut tiled);
    let linear = tdb_rx2::texlib::untile_dxt(&tiled, t.width, t.height, t.format.bytes_per_block());
    let fourcc = t.format.fourcc().unwrap();
    let mut dds = Vec::new();
    dds.extend_from_slice(b"DDS ");
    let mut hdr = [0u8; 124];
    hdr[0..4].copy_from_slice(&124u32.to_le_bytes());
    hdr[4..8].copy_from_slice(&0x0008_1007u32.to_le_bytes());
    hdr[8..12].copy_from_slice(&t.height.to_le_bytes());
    hdr[12..16].copy_from_slice(&t.width.to_le_bytes());
    hdr[16..20].copy_from_slice(&(linear.len() as u32).to_le_bytes());
    hdr[72..76].copy_from_slice(&32u32.to_le_bytes());
    hdr[76..80].copy_from_slice(&0x4u32.to_le_bytes());
    hdr[80..84].copy_from_slice(fourcc);
    hdr[108..112].copy_from_slice(&0x1000u32.to_le_bytes());
    dds.extend_from_slice(&hdr);
    dds.extend_from_slice(&linear);
    let (rgba, w, h) = tdb_headmesh::texture::decode_source_dds(&dds).unwrap();
    (rgba, w, h)
}

fn norm(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn main() {
    let mut a = std::env::args().skip(1);
    let mesh_path = a.next().expect("player.rx2");
    let tex_path = a.next().expect("texlib.rx2");
    let out = a.next().expect("out.png");
    let mode = a.next().unwrap_or_else(|| "full".into());
    let spec: f32 = a
        .next()
        .unwrap_or_else(|| "0.35".into())
        .parse()
        .unwrap_or(0.35);

    let data = std::fs::read(&mesh_path).unwrap();
    let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).unwrap();
    let s = asset
        .submeshes
        .iter()
        .find(|s| s.name.starts_with("head_"))
        .expect("head_");
    let pos: Vec<[f32; 3]> = s.vertices.iter().map(|v| v.position).collect();
    let nrm: Vec<[f32; 3]> = s.vertices.iter().map(|v| norm(v.normal)).collect();
    let uv: Vec<[f32; 2]> = s.vertices.iter().map(|v| v.uv).collect();
    let tris: Vec<[u32; 3]> = s.triangles.iter().map(|t| [t[0], t[1], t[2]]).collect();
    let (tex, tw, th) = decode_head_texture(&tex_path);

    // Frame the face: centre on the bbox, camera on +Z looking back.
    let mut mn = [f32::MAX; 3];
    let mut mx = [f32::MIN; 3];
    for p in &pos {
        for k in 0..3 {
            mn[k] = mn[k].min(p[k]);
            mx[k] = mx[k].max(p[k]);
        }
    }
    let c = [
        (mn[0] + mx[0]) * 0.5,
        (mn[1] + mx[1]) * 0.5 + 0.03,
        (mn[2] + mx[2]) * 0.5,
    ];
    let (w, h) = (900u32, 1000u32);
    let dist = 0.42f32;
    let f = 1.9f32; // focal, in half-heights

    let light = norm([-0.45, 0.55, 1.0]);
    let mut color = vec![0u8; (w * h * 4) as usize];
    let mut depth = vec![f32::MAX; (w * h) as usize];

    let project = |p: [f32; 3]| -> ([f32; 2], f32) {
        let e = [p[0] - c[0], p[1] - c[1], p[2] - c[2] - dist];
        let z = -e[2];
        let sx = (e[0] / z) * f * (h as f32 * 0.5) + w as f32 * 0.5;
        let sy = -(e[1] / z) * f * (h as f32 * 0.5) + h as f32 * 0.5;
        ([sx, sy], z)
    };

    for t in &tris {
        let (a0, z0) = project(pos[t[0] as usize]);
        let (a1, z1) = project(pos[t[1] as usize]);
        let (a2, z2) = project(pos[t[2] as usize]);
        let area = (a1[0] - a0[0]) * (a2[1] - a0[1]) - (a2[0] - a0[0]) * (a1[1] - a0[1]);
        if area.abs() < 1e-9 {
            continue;
        }
        // Geometric normal for the facet mode.
        let (p0, p1, p2) = (pos[t[0] as usize], pos[t[1] as usize], pos[t[2] as usize]);
        let u = [p1[0] - p0[0], p1[1] - p0[1], p1[2] - p0[2]];
        let v = [p2[0] - p0[0], p2[1] - p0[1], p2[2] - p0[2]];
        let fnrm = norm([
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ]);

        let xlo = a0[0].min(a1[0]).min(a2[0]).floor().max(0.0) as u32;
        let xhi = (a0[0].max(a1[0]).max(a2[0]).ceil() as i64).clamp(0, w as i64 - 1) as u32;
        let ylo = a0[1].min(a1[1]).min(a2[1]).floor().max(0.0) as u32;
        let yhi = (a0[1].max(a1[1]).max(a2[1]).ceil() as i64).clamp(0, h as i64 - 1) as u32;

        for py in ylo..=yhi {
            for px in xlo..=xhi {
                let p = [px as f32 + 0.5, py as f32 + 0.5];
                let w0 = ((a1[0] - p[0]) * (a2[1] - p[1]) - (a2[0] - p[0]) * (a1[1] - p[1])) / area;
                let w1 = ((a2[0] - p[0]) * (a0[1] - p[1]) - (a0[0] - p[0]) * (a2[1] - p[1])) / area;
                let w2 = 1.0 - w0 - w1;
                if w0 < 0.0 || w1 < 0.0 || w2 < 0.0 {
                    continue;
                }
                let z = w0 * z0 + w1 * z1 + w2 * z2;
                let di = (py * w + px) as usize;
                if z >= depth[di] {
                    continue;
                }
                depth[di] = z;

                let mut n = [0.0f32; 3];
                let mut t_uv = [0.0f32; 2];
                for (k, wk) in [w0, w1, w2].iter().enumerate() {
                    let vi = t[k] as usize;
                    for cc in 0..3 {
                        n[cc] += wk * nrm[vi][cc];
                    }
                    for cc in 0..2 {
                        t_uv[cc] += wk * uv[vi][cc];
                    }
                }
                let n = if mode == "facet" { fnrm } else { norm(n) };

                let albedo = if mode == "normals" {
                    [0.8f32, 0.8, 0.8]
                } else {
                    let sx = ((t_uv[0] * tw as f32) as i64).clamp(0, tw as i64 - 1) as u32;
                    let sy = ((t_uv[1] * th as f32) as i64).clamp(0, th as i64 - 1) as u32;
                    let o = ((sy * tw + sx) * 4) as usize;
                    [
                        tex[o] as f32 / 255.0,
                        tex[o + 1] as f32 / 255.0,
                        tex[o + 2] as f32 / 255.0,
                    ]
                };

                let lam = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).max(0.0);
                let view = [0.0f32, 0.0, 1.0];
                let hv = norm([light[0] + view[0], light[1] + view[1], light[2] + view[2]]);
                let sp = (n[0] * hv[0] + n[1] * hv[1] + n[2] * hv[2])
                    .max(0.0)
                    .powf(48.0)
                    * spec;
                let o = di * 4;
                for cc in 0..3 {
                    let lit = if mode == "albedo" {
                        albedo[cc]
                    } else {
                        albedo[cc] * (0.25 + 0.85 * lam) + sp
                    };
                    color[o + cc] = (lit.clamp(0.0, 1.0) * 255.0) as u8;
                }
                color[o + 3] = 255;
            }
        }
    }
    image::save_buffer(&out, &color, w, h, image::ColorType::Rgba8).unwrap();
    println!("wrote {out} ({mode})");
}
