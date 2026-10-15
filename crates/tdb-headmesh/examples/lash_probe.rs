//! How close does the `haircap_` sit to the eyeballs?
//!
//! Legacy carries the eyelashes on the haircap, so "the lashes float in front of the eye"
//! is measurable as the standoff between the cap and the `eyel_`/`eyer_` surfaces. Vanilla
//! is the reference: its lashes hug the eyeball. A conversion that moves the eyes onto the
//! source's anatomy but leaves the donor cap where it was opens exactly this gap.

/// A decoded mesh surface: vertex positions and the triangle indices over them.
type Surface = (Vec<[f32; 3]>, Vec<[u32; 3]>);

use tdb_headmesh::transfer;

fn main() {
    for path in std::env::args().skip(1) {
        let data = std::fs::read(&path).expect("read");
        let asset = tdb_rx2::playermesh::parse_player_mesh_preview(&data).expect("parse");
        let grab = |prefix: &str| -> Option<Surface> {
            asset
                .submeshes
                .iter()
                .find(|s| s.name.starts_with(prefix))
                .map(|s| {
                    (
                        s.vertices.iter().map(|v| v.position).collect(),
                        s.triangles.iter().map(|t| [t[0], t[1], t[2]]).collect(),
                    )
                })
        };
        let (Some((cap, _)), Some((eye, etri))) = (grab("haircap_"), grab("eyel_")) else {
            continue;
        };
        // Only the cap vertices on the left eye's side, so the right eye doesn't dominate.
        let cx = eye.iter().map(|p| p[0]).sum::<f32>() / eye.len() as f32;
        let near: Vec<[f32; 3]> = cap
            .iter()
            .copied()
            .filter(|p| (p[0] - cx).abs() < 0.035)
            .collect();
        let hits = transfer::closest_points(&near, &eye, &etri).expect("closest");
        let mut d: Vec<f64> = hits.iter().map(|h| h.distance_mm).collect();
        d.sort_by(|a, b| a.partial_cmp(b).unwrap());
        // The lash band is whatever part of the cap comes nearest the eyeball.
        let band = &d[..20.min(d.len())];
        println!(
            "{:<62} cap verts over the eye: {:<4} nearest {:.2} mm  lash-band mean {:.2} mm",
            path.rsplit(['/', '\\'])
                .take(2)
                .collect::<Vec<_>>()
                .join("/"),
            near.len(),
            d[0],
            band.iter().sum::<f64>() / band.len() as f64
        );
    }
}
