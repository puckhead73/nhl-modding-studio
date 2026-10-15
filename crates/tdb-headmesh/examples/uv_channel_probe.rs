//! Check whether a stride-52 `haircap_` really carries three identical UV channels.
//!
//! `pack_vert_stride52` duplicates UV0 into all three channels. Re-substituting the
//! haircap (rather than byte-patching its positions) is only lossless if the donor does
//! the same — otherwise the repack would silently flatten two real channels.

fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).unwrap();
        let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).unwrap();
        println!("\n=== {path}");
        for s in &asset.submeshes {
            if s.vertices.len() < 2 {
                continue;
            }
            let enc = |p: [f32; 3]| {
                let mut b = Vec::new();
                for c in p {
                    b.extend_from_slice(&tdb_rx2::playermesh::f32_to_half_be(c));
                }
                b
            };
            let (v0, v1) = (enc(s.vertices[0].position), enc(s.vertices[1].position));
            let stride = s.stride;
            let Some(base) = (0..data.len().saturating_sub(stride * 2))
                .find(|&o| data[o..o + 6] == v0[..] && data[o + stride..o + stride + 6] == v1[..])
            else {
                continue;
            };
            if stride != 52 {
                println!(
                    "  {:<16} stride {stride} — not the 3-channel layout",
                    s.name
                );
                continue;
            }
            let f = |o: usize| f32::from_be_bytes(data[o..o + 4].try_into().unwrap());
            let (mut eq01, mut eq02) = (0usize, 0usize);
            let mut max_d = 0f32;
            for i in 0..s.vertices.len() {
                let o = base + i * stride + 16;
                let ch: Vec<[f32; 2]> = (0..3).map(|c| [f(o + c * 8), f(o + c * 8 + 4)]).collect();
                if ch[0] == ch[1] {
                    eq01 += 1;
                }
                if ch[0] == ch[2] {
                    eq02 += 1;
                }
                for c in ch.iter().take(3).skip(1) {
                    for (k, axis) in c.iter().enumerate().take(2) {
                        max_d = max_d.max((axis - ch[0][k]).abs());
                    }
                }
            }
            let n = s.vertices.len();
            println!(
                "  {:<16} stride 52 verts {n}: UV1==UV0 {eq01}/{n}, UV2==UV0 {eq02}/{n}, max deviation {max_d:.6}",
                s.name
            );
        }
    }
}
