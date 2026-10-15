//! Measure the signed clearance between the `haircap_` shell and the `head_` surface.
//!
//! Legacy ships the scalp as its own submesh sitting a small distance outside the skull.
//! A conversion that replaces `head_` with a differently-shaped skull but keeps the
//! donor's `haircap_` verbatim will make the two surfaces cross, and a pair of co-planar
//! surfaces oscillating around each other rasterises as moiré striping — "geometric,
//! tiled" shading that follows neither the geometry nor the UVs.
//!
//! Reports the signed distance from each haircap vertex to the nearest head surface
//! point, positive = haircap outside the skull (correct).

use tdb_headmesh::transfer;

fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).expect("read");
        let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).expect("parse");
        let grab = |prefix: &str| {
            asset
                .submeshes
                .iter()
                .find(|s| s.name.starts_with(prefix))
                .map(|s| {
                    let p: Vec<[f32; 3]> = s.vertices.iter().map(|v| v.position).collect();
                    let t: Vec<[u32; 3]> = s.triangles.iter().map(|t| [t[0], t[1], t[2]]).collect();
                    (p, t)
                })
        };
        let Some((hpos, htri)) = grab("head_") else {
            continue;
        };
        println!("\n=== {path}");
        let hn = tdb_headmesh::pack_rw::smooth_normals(&hpos, &htri);

        for other in ["haircap_", "eyel_", "mouthbag_"] {
            let Some((opos, _)) = grab(other) else {
                continue;
            };
            let hits = transfer::closest_points(&opos, &hpos, &htri).expect("closest");
            let mut signed: Vec<f32> = Vec::with_capacity(opos.len());
            for (i, h) in hits.iter().enumerate() {
                // Footpoint and its interpolated head normal.
                let mut fp = [0.0f32; 3];
                let mut n = [0.0f32; 3];
                for k in 0..3 {
                    let v = h.tri[k] as usize;
                    for c in 0..3 {
                        fp[c] += h.bary[k] * hpos[v][c];
                        n[c] += h.bary[k] * hn[v][c];
                    }
                }
                let nl = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-9);
                let d: f32 = (0..3).map(|c| (opos[i][c] - fp[c]) * n[c] / nl).sum();
                signed.push(d * 1000.0);
            }
            signed.sort_by(|a, b| a.partial_cmp(b).unwrap());
            let n = signed.len();
            let inside = signed.iter().filter(|d| **d < 0.0).count();
            let zfight = signed.iter().filter(|d| d.abs() < 0.5).count();
            println!(
                "  {other:<10} n={n:<5} signed clearance mm: min {:.2}  p05 {:.2}  p50 {:.2}  p95 {:.2}  max {:.2}",
                signed[0],
                signed[n / 20],
                signed[n / 2],
                signed[n * 19 / 20],
                signed[n - 1]
            );
            println!(
                "             inside the skull {inside}/{n} ({:.1}%)   within +/-0.5mm {zfight} ({:.1}%)",
                100.0 * inside as f32 / n as f32,
                100.0 * zfight as f32 / n as f32
            );
        }
    }
}
