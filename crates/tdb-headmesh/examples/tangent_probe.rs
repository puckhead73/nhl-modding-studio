//! Report NORMAL and TANGENT occupancy per submesh of an RW player mesh.
//!
//! Written to settle whether the tangent slot matters: it does. Every vanilla vertex
//! carries a non-zero tangent (2623/2623 on `player_67`'s `head_`), and the packer used
//! to write zeros there, which made the skin shader's tangent-space lighting break up into
//! geometric, tiled noise. Use this to confirm a converted asset fills the slot.

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: tangent_probe <player.rx2>");
    let data = std::fs::read(&path).unwrap();
    let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).unwrap();
    for s in &asset.submeshes {
        if s.vertices.is_empty() {
            continue;
        }
        // Locate this submesh's vertex buffer by matching vertex 0's fp16 position bytes,
        // then confirm with vertex 1 at +stride.
        let enc = |p: [f32; 3]| {
            let mut b = Vec::new();
            for c in p {
                b.extend_from_slice(&tdb_rx2::playermesh::f32_to_half_be(c));
            }
            b
        };
        let v0 = enc(s.vertices[0].position);
        let v1 = enc(s.vertices[1].position);
        let stride = s.stride;
        let mut found = None;
        for off in 0..data.len().saturating_sub(stride * 2) {
            if data[off..off + 6] == v0[..] && data[off + stride..off + stride + 6] == v1[..] {
                found = Some(off);
                break;
            }
        }
        let Some(base) = found else {
            println!("{:<16} stride {stride}: vertex buffer not located", s.name);
            continue;
        };
        let mut nonzero_tangent = 0usize;
        let mut nonzero_normal = 0usize;
        let mut sample = String::new();
        for i in 0..s.vertices.len() {
            let o = base + i * stride;
            if o + 16 > data.len() {
                break;
            }
            let n = u32::from_be_bytes(data[o + 8..o + 12].try_into().unwrap());
            let t = u32::from_be_bytes(data[o + 12..o + 16].try_into().unwrap());
            if n != 0 {
                nonzero_normal += 1;
            }
            if t != 0 {
                nonzero_tangent += 1;
            }
            if i < 3 {
                sample.push_str(&format!(" [n={n:08X} t={t:08X}]"));
            }
        }
        println!(
            "{:<16} stride {stride} verts {}: normals non-zero {} / tangents non-zero {}\n   {}",
            s.name,
            s.vertices.len(),
            nonzero_normal,
            nonzero_tangent,
            sample
        );
    }
}
