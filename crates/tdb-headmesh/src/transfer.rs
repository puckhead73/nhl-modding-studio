//! Weight-transfer stage — the Path C answer to "where do the skin weights come from?".
//!
//! Path A retargets a *named source rig* onto the target palette ([`crate::remap`]). That
//! needs the source to ship articulation. Reallusion CC5 head exports do not: the joints
//! they carry (`hd_neutral_m` / `rl_boneroot` / `cc_base_hip`) are coincident, so every
//! authored weight resolves to one rigid transform and there is nothing to retarget. A
//! naive "everything rides the Head slot" fallback ships a head that detaches from the
//! neck the first time it turns, and never opens its jaw.
//!
//! Instead the weights come from the **target donor**, which is the same anatomy in the
//! same space and is already rigged correctly for the target game. For each source vertex
//! we find the closest point on the donor head surface and inherit that point's
//! barycentrically-interpolated palette influences. Anatomy does the mapping: a vertex on
//! the source chin lands on the donor chin and picks up the donor's jaw weighting; a
//! vertex on the source neck lands on the donor neck and picks up Neck.
//!
//! This is a *proximity* transfer, not a geodesic one, so it is only sound while the two
//! surfaces are close — which is exactly the regime a head transplant lives in, and the
//! report quantifies it ([`TransferReport::max_distance_mm`]) so a bad fit is visible
//! rather than silent.
//!
//! Properties the implementation guarantees:
//! * output influences are already inside the target's budget (top-`max_influences`,
//!   renormalized to sum 1.0), so [`crate::pack_rw`] never sees an over-wide vertex;
//! * only palette slots the donor itself weights can appear — the transfer cannot invent
//!   a slot outside the census-verified used set;
//! * every vertex gets a weight (the closest point always exists on a non-empty donor).

use std::collections::BTreeMap;

/// A donor surface with its per-vertex palette influences, ready to donate weights.
pub struct DonorSkin<'a> {
    pub positions: &'a [[f32; 3]],
    pub triangles: &'a [[u32; 3]],
    /// Parallel to `positions`: (palette slot, weight) per donor vertex.
    pub influences: &'a [Vec<(u32, f32)>],
}

#[derive(Debug, thiserror::Error)]
pub enum TransferError {
    #[error("donor head has no triangles to transfer weights from")]
    EmptyDonor,
    #[error(
        "donor has {positions} vertices but {influences} influence rows — \
         donor parse is inconsistent"
    )]
    DonorMismatch { positions: usize, influences: usize },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransferReport {
    pub vertices: usize,
    /// Distance from each source vertex to the donor surface it sourced weights from.
    pub mean_distance_mm: f64,
    pub max_distance_mm: f64,
    /// 95th percentile — the number that spots a localized bad region that the mean hides.
    pub p95_distance_mm: f64,
    /// Distinct palette slots the transferred weights land on.
    pub palette_slots_used: Vec<u32>,
    /// Share of total weight mass per palette slot, most-weighted first (top 8).
    pub top_slots: Vec<(u32, f64)>,
    /// Vertices whose closest donor point was further than [`FAR_VERTEX_MM`].
    pub far_vertices: usize,
}

/// A source vertex further than this from the donor surface is flagged: the transferred
/// weighting is an extrapolation there (hair volume, a beard, an ear the donor lacks).
pub const FAR_VERTEX_MM: f64 = 15.0;

/// Uniform-grid acceleration over donor triangles — brute force is 4k × 5k point/triangle
/// tests per head, which is fine in release and painful in a debug build.
struct TriGrid {
    cell: f32,
    min: [f32; 3],
    dims: [i32; 3],
    buckets: BTreeMap<i64, Vec<u32>>,
}

impl TriGrid {
    fn build(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> TriGrid {
        let mut min = [f32::MAX; 3];
        let mut max = [f32::MIN; 3];
        for p in positions {
            for k in 0..3 {
                min[k] = min[k].min(p[k]);
                max[k] = max[k].max(p[k]);
            }
        }
        // ~16 cells across the longest axis: a head is ~0.4 m, so cells land near 25 mm.
        let extent = (0..3).map(|k| max[k] - min[k]).fold(0f32, f32::max);
        let cell = (extent / 16.0).max(1e-3);
        let dims = [
            (((max[0] - min[0]) / cell).ceil() as i32 + 1).max(1),
            (((max[1] - min[1]) / cell).ceil() as i32 + 1).max(1),
            (((max[2] - min[2]) / cell).ceil() as i32 + 1).max(1),
        ];
        let mut grid = TriGrid {
            cell,
            min,
            dims,
            buckets: BTreeMap::new(),
        };
        for (ti, tri) in triangles.iter().enumerate() {
            let pts = [
                positions[tri[0] as usize],
                positions[tri[1] as usize],
                positions[tri[2] as usize],
            ];
            let mut lo = [i32::MAX; 3];
            let mut hi = [i32::MIN; 3];
            for p in &pts {
                let c = grid.cell_of(*p);
                for k in 0..3 {
                    lo[k] = lo[k].min(c[k]);
                    hi[k] = hi[k].max(c[k]);
                }
            }
            for x in lo[0]..=hi[0] {
                for y in lo[1]..=hi[1] {
                    for z in lo[2]..=hi[2] {
                        grid.buckets
                            .entry(grid_key([x, y, z]))
                            .or_default()
                            .push(ti as u32);
                    }
                }
            }
        }
        grid
    }

    fn cell_of(&self, p: [f32; 3]) -> [i32; 3] {
        let mut c = [0i32; 3];
        for k in 0..3 {
            c[k] = (((p[k] - self.min[k]) / self.cell).floor() as i32)
                .clamp(0, self.dims[k].saturating_sub(1));
        }
        c
    }
}

fn grid_key(c: [i32; 3]) -> i64 {
    // 21 bits per axis, biased — plenty for a head-sized grid.
    let f = |v: i32| i64::from(v + (1 << 20));
    (f(c[0]) << 42) | (f(c[1]) << 21) | f(c[2])
}

/// Closest point on triangle `abc` to `p`, returned with its barycentric coordinates.
/// Ericson, *Real-Time Collision Detection*, §5.1.5.
fn closest_point_on_triangle(
    p: [f32; 3],
    a: [f32; 3],
    b: [f32; 3],
    c: [f32; 3],
) -> ([f32; 3], [f32; 3]) {
    let sub = |u: [f32; 3], v: [f32; 3]| [u[0] - v[0], u[1] - v[1], u[2] - v[2]];
    let dot = |u: [f32; 3], v: [f32; 3]| u[0] * v[0] + u[1] * v[1] + u[2] * v[2];
    let ab = sub(b, a);
    let ac = sub(c, a);
    let ap = sub(p, a);
    let d1 = dot(ab, ap);
    let d2 = dot(ac, ap);
    if d1 <= 0.0 && d2 <= 0.0 {
        return (a, [1.0, 0.0, 0.0]);
    }
    let bp = sub(p, b);
    let d3 = dot(ab, bp);
    let d4 = dot(ac, bp);
    if d3 >= 0.0 && d4 <= d3 {
        return (b, [0.0, 1.0, 0.0]);
    }
    let vc = d1 * d4 - d3 * d2;
    if vc <= 0.0 && d1 >= 0.0 && d3 <= 0.0 {
        let v = d1 / (d1 - d3);
        let q = [a[0] + ab[0] * v, a[1] + ab[1] * v, a[2] + ab[2] * v];
        return (q, [1.0 - v, v, 0.0]);
    }
    let cp = sub(p, c);
    let d5 = dot(ab, cp);
    let d6 = dot(ac, cp);
    if d6 >= 0.0 && d5 <= d6 {
        return (c, [0.0, 0.0, 1.0]);
    }
    let vb = d5 * d2 - d1 * d6;
    if vb <= 0.0 && d2 >= 0.0 && d6 <= 0.0 {
        let w = d2 / (d2 - d6);
        let q = [a[0] + ac[0] * w, a[1] + ac[1] * w, a[2] + ac[2] * w];
        return (q, [1.0 - w, 0.0, w]);
    }
    let va = d3 * d6 - d5 * d4;
    if va <= 0.0 && (d4 - d3) >= 0.0 && (d5 - d6) >= 0.0 {
        let w = (d4 - d3) / ((d4 - d3) + (d5 - d6));
        let q = [
            b[0] + (c[0] - b[0]) * w,
            b[1] + (c[1] - b[1]) * w,
            b[2] + (c[2] - b[2]) * w,
        ];
        return (q, [0.0, 1.0 - w, w]);
    }
    let denom = 1.0 / (va + vb + vc);
    let v = vb * denom;
    let w = vc * denom;
    let q = [
        a[0] + ab[0] * v + ac[0] * w,
        a[1] + ab[1] * v + ac[1] * w,
        a[2] + ab[2] * v + ac[2] * w,
    ];
    (q, [1.0 - v - w, v, w])
}

/// Where one query point landed on the donor surface.
///
/// This is the correspondence every donor-sourced attribute rides on: weights today, UVs
/// when the reprojection stage needs the donor's atlas layout. Computing it once and
/// sampling it many times keeps every channel on the *same* footpoint — a vertex cannot
/// take its weights from one donor triangle and its UVs from another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SurfaceHit {
    /// Donor triangle (vertex indices) the query landed on.
    pub tri: [u32; 3],
    /// Barycentric coordinates of the footpoint within `tri`, summing to 1.
    pub bary: [f32; 3],
    /// Distance from the query to the footpoint, millimetres.
    pub distance_mm: f64,
}

/// Whether a donor triangle may serve a given query vertex.
///
/// `accept(query_index, donor_triangle_index)`. Used to keep the correspondence
/// anatomically honest — see [`closest_points_gated`].
pub type Gate<'a> = dyn Fn(usize, usize) -> bool + 'a;

/// Closest point on the donor surface for every query position, unconstrained.
pub fn closest_points(
    positions: &[[f32; 3]],
    donor_positions: &[[f32; 3]],
    donor_triangles: &[[u32; 3]],
) -> Result<Vec<SurfaceHit>, TransferError> {
    closest_points_gated(positions, donor_positions, donor_triangles, &|_, _| true)
        .map(|(hits, _)| hits)
}

/// Closest **acceptable** point on the donor surface for every query position.
///
/// Pure proximity is anatomically blind. In the bind pose the mouth is closed, so the
/// nearest donor surface to a lower-lip vertex is frequently the *upper* lip — inheriting
/// its weights welds the mouth shut. `accept` rejects those candidates so the search
/// continues to the nearest legitimate one.
///
/// A gate that rejects everything would leave a vertex with no correspondence at all, so
/// each such vertex falls back to the ungated nearest point rather than going unweighted;
/// the count of those rescues is returned, because a large number means the gate itself is
/// miscalibrated and the operator needs to know rather than get quietly worse weights.
pub fn closest_points_gated(
    positions: &[[f32; 3]],
    donor_positions: &[[f32; 3]],
    donor_triangles: &[[u32; 3]],
    accept: &Gate<'_>,
) -> Result<(Vec<SurfaceHit>, usize), TransferError> {
    if donor_triangles.is_empty() || donor_positions.is_empty() {
        return Err(TransferError::EmptyDonor);
    }
    let grid = TriGrid::build(donor_positions, donor_triangles);
    let mut hits = Vec::with_capacity(positions.len());
    let mut rescued = 0usize;
    for (qi, p) in positions.iter().enumerate() {
        let mut hit = search(&grid, donor_positions, donor_triangles, *p, &|ti| {
            accept(qi, ti)
        });
        if hit.is_none() {
            rescued += 1;
            hit = search(&grid, donor_positions, donor_triangles, *p, &|_| true);
        }
        hits.push(hit.expect("non-empty donor always yields a closest point"));
    }
    Ok((hits, rescued))
}

fn search(
    grid: &TriGrid,
    donor_positions: &[[f32; 3]],
    donor_triangles: &[[u32; 3]],
    p: [f32; 3],
    accept: &dyn Fn(usize) -> bool,
) -> Option<SurfaceHit> {
    {
        // Grow a cell-radius ring around the query until a hit is found, then take one
        // extra ring: a triangle in a further cell can still be nearer than one found in
        // a closer cell, and one ring covers that at this cell size.
        let home = grid.cell_of(p);
        let mut best: Option<(f32, [f32; 3], [u32; 3])> = None;
        let max_ring = grid.dims.iter().copied().max().unwrap_or(1);
        let mut ring = 0i32;
        let mut rings_after_hit = 0;
        while ring <= max_ring {
            let mut candidates: Vec<u32> = Vec::new();
            for x in (home[0] - ring)..=(home[0] + ring) {
                for y in (home[1] - ring)..=(home[1] + ring) {
                    for z in (home[2] - ring)..=(home[2] + ring) {
                        // Only the newly-added shell of the cube.
                        let on_shell = (x - home[0]).abs() == ring
                            || (y - home[1]).abs() == ring
                            || (z - home[2]).abs() == ring;
                        if !on_shell {
                            continue;
                        }
                        if let Some(b) = grid.buckets.get(&grid_key([x, y, z])) {
                            candidates.extend_from_slice(b);
                        }
                    }
                }
            }
            for ti in candidates {
                if !accept(ti as usize) {
                    continue;
                }
                let tri = donor_triangles[ti as usize];
                let (q, bary) = closest_point_on_triangle(
                    p,
                    donor_positions[tri[0] as usize],
                    donor_positions[tri[1] as usize],
                    donor_positions[tri[2] as usize],
                );
                let d2 = (0..3).map(|k| (q[k] - p[k]).powi(2)).sum::<f32>();
                if best.is_none_or(|(bd, _, _)| d2 < bd) {
                    best = Some((d2, bary, tri));
                }
            }
            if best.is_some() {
                rings_after_hit += 1;
                if rings_after_hit > 1 {
                    break;
                }
            }
            ring += 1;
        }
        best.map(|(d2, bary, tri)| SurfaceHit {
            tri,
            bary,
            distance_mm: f64::from(d2.max(0.0).sqrt()) * 1000.0,
        })
    }
}

/// Per-vertex palette influences: for each vertex, its (palette slot, weight) pairs.
pub type Influences = Vec<Vec<(u32, f32)>>;

/// Transfer palette skin weights from `donor` onto `positions` by closest surface point.
///
/// Returns per-vertex palette influences (already truncated to `max_influences` and
/// renormalized) plus a [`TransferReport`] quantifying the fit.
pub fn transfer_weights(
    positions: &[[f32; 3]],
    donor: &DonorSkin<'_>,
    max_influences: usize,
) -> Result<(Influences, TransferReport), TransferError> {
    transfer_weights_gated(positions, donor, max_influences, &|_, _| true).map(|(w, r, _)| (w, r))
}

/// [`transfer_weights`], but only sourcing from donor triangles the gate accepts.
///
/// Also returns how many vertices had to fall back to an ungated nearest point because the
/// gate rejected every candidate.
pub fn transfer_weights_gated(
    positions: &[[f32; 3]],
    donor: &DonorSkin<'_>,
    max_influences: usize,
    accept: &Gate<'_>,
) -> Result<(Influences, TransferReport, usize), TransferError> {
    if donor.positions.len() != donor.influences.len() {
        return Err(TransferError::DonorMismatch {
            positions: donor.positions.len(),
            influences: donor.influences.len(),
        });
    }
    let (hits, rescued) =
        closest_points_gated(positions, donor.positions, donor.triangles, accept)?;

    let mut out = Vec::with_capacity(positions.len());
    let mut distances = Vec::with_capacity(positions.len());
    let mut slot_mass: BTreeMap<u32, f64> = BTreeMap::new();

    for hit in &hits {
        let (bary, tri) = (hit.bary, hit.tri);
        distances.push(hit.distance_mm);

        // Barycentric blend of the three corners' influences.
        let mut acc: BTreeMap<u32, f32> = BTreeMap::new();
        for (corner, &vi) in tri.iter().enumerate() {
            let bw = bary[corner];
            if bw <= 0.0 {
                continue;
            }
            for &(slot, w) in &donor.influences[vi as usize] {
                *acc.entry(slot).or_insert(0.0) += w * bw;
            }
        }
        let mut list: Vec<(u32, f32)> = acc.into_iter().filter(|(_, w)| *w > 0.0).collect();
        list.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        list.truncate(max_influences);
        let total: f32 = list.iter().map(|(_, w)| *w).sum();
        if total > 0.0 {
            for (_, w) in &mut list {
                *w /= total;
            }
        }
        for (slot, w) in &list {
            *slot_mass.entry(*slot).or_insert(0.0) += f64::from(*w);
        }
        out.push(list);
    }

    let mut sorted = distances.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let pct = |q: f64| -> f64 {
        if sorted.is_empty() {
            return 0.0;
        }
        let idx = ((sorted.len() - 1) as f64 * q).round() as usize;
        sorted[idx]
    };
    let mut top_slots: Vec<(u32, f64)> = slot_mass.iter().map(|(k, v)| (*k, *v)).collect();
    let mass_total: f64 = top_slots.iter().map(|(_, m)| *m).sum();
    top_slots.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    top_slots.truncate(8);
    if mass_total > 0.0 {
        for (_, m) in &mut top_slots {
            *m = *m / mass_total * 100.0;
        }
    }

    let report = TransferReport {
        vertices: positions.len(),
        mean_distance_mm: if distances.is_empty() {
            0.0
        } else {
            distances.iter().sum::<f64>() / distances.len() as f64
        },
        max_distance_mm: sorted.last().copied().unwrap_or(0.0),
        p95_distance_mm: pct(0.95),
        palette_slots_used: slot_mass.keys().copied().collect(),
        top_slots,
        far_vertices: distances.iter().filter(|d| **d > FAR_VERTEX_MM).count(),
    };
    Ok((out, report, rescued))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Two triangles forming a unit quad in the z=0 plane, corners weighted to distinct
    /// palette slots so a transfer's provenance is readable from the result.
    fn quad_donor() -> (Vec<[f32; 3]>, Vec<[u32; 3]>, Influences) {
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [1.0, 1.0, 0.0],
        ];
        let triangles = vec![[0, 1, 2], [1, 3, 2]];
        let influences = vec![
            vec![(8u32, 1.0f32)],
            vec![(16, 1.0)],
            vec![(6, 1.0)],
            vec![(4, 1.0)],
        ];
        (positions, triangles, influences)
    }

    #[test]
    fn transfers_the_nearest_corner_weight_and_measures_zero_distance() {
        let (positions, triangles, influences) = quad_donor();
        let donor = DonorSkin {
            positions: &positions,
            triangles: &triangles,
            influences: &influences,
        };
        // Exactly on donor vertex 0 → pure slot 8, zero distance.
        let (w, report) = transfer_weights(&[[0.0, 0.0, 0.0]], &donor, 4).unwrap();
        assert_eq!(w[0], vec![(8, 1.0)]);
        assert!(report.max_distance_mm < 1e-6, "{report:?}");
        assert_eq!(report.far_vertices, 0);
    }

    #[test]
    fn blends_corners_barycentrically_and_normalizes() {
        let (positions, triangles, influences) = quad_donor();
        let donor = DonorSkin {
            positions: &positions,
            triangles: &triangles,
            influences: &influences,
        };
        // Midpoint of the 0–1 edge → half slot 8, half slot 16.
        let (w, _) = transfer_weights(&[[0.5, 0.0, 0.0]], &donor, 4).unwrap();
        let map: BTreeMap<u32, f32> = w[0].iter().copied().collect();
        assert!((map[&8] - 0.5).abs() < 1e-5, "{:?}", w[0]);
        assert!((map[&16] - 0.5).abs() < 1e-5, "{:?}", w[0]);
        let sum: f32 = w[0].iter().map(|(_, x)| *x).sum();
        assert!((sum - 1.0).abs() < 1e-6);
    }

    #[test]
    fn offset_query_reports_its_distance_and_still_inherits_weights() {
        let (positions, triangles, influences) = quad_donor();
        let donor = DonorSkin {
            positions: &positions,
            triangles: &triangles,
            influences: &influences,
        };
        // 20 mm off the surface above vertex 0.
        let (w, report) = transfer_weights(&[[0.0, 0.0, 0.02]], &donor, 4).unwrap();
        assert_eq!(w[0], vec![(8, 1.0)]);
        assert!((report.max_distance_mm - 20.0).abs() < 1e-3, "{report:?}");
        assert_eq!(report.far_vertices, 1, "20 mm is past the far-vertex gate");
    }

    #[test]
    fn truncates_to_the_influence_budget_and_renormalizes() {
        // One triangle whose corners each carry two slots → 6 candidate slots, budget 4.
        let positions = vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let triangles = vec![[0, 1, 2]];
        let influences = vec![
            vec![(1u32, 0.5f32), (2, 0.5)],
            vec![(3, 0.5), (4, 0.5)],
            vec![(5, 0.5), (6, 0.5)],
        ];
        let donor = DonorSkin {
            positions: &positions,
            triangles: &triangles,
            influences: &influences,
        };
        let centroid = [1.0 / 3.0, 1.0 / 3.0, 0.0];
        let (w, report) = transfer_weights(&[centroid], &donor, 4).unwrap();
        assert_eq!(w[0].len(), 4);
        let sum: f32 = w[0].iter().map(|(_, x)| *x).sum();
        assert!((sum - 1.0).abs() < 1e-6, "renormalized after truncation");
        assert!(report.palette_slots_used.len() <= 6);
    }

    #[test]
    fn closest_points_reports_the_footpoint_the_weights_ride_on() {
        let (positions, triangles, influences) = quad_donor();
        let donor = DonorSkin {
            positions: &positions,
            triangles: &triangles,
            influences: &influences,
        };
        let query = [[0.5f32, 0.0, 0.0]]; // midpoint of the 0–1 edge
        let hits = closest_points(&query, &positions, &triangles).unwrap();
        assert_eq!(hits[0].tri, triangles[0]);
        assert!((hits[0].bary[0] - 0.5).abs() < 1e-5, "{:?}", hits[0]);
        assert!((hits[0].bary[1] - 0.5).abs() < 1e-5, "{:?}", hits[0]);
        assert!(hits[0].distance_mm < 1e-6);
        // The blended weights follow from that same footpoint.
        let (w, _) = transfer_weights(&query, &donor, 4).unwrap();
        let map: BTreeMap<u32, f32> = w[0].iter().copied().collect();
        assert!((map[&8] - 0.5).abs() < 1e-5);
    }

    #[test]
    fn closest_points_refuses_an_empty_donor() {
        assert!(matches!(
            closest_points(&[[0.0; 3]], &[], &[]),
            Err(TransferError::EmptyDonor)
        ));
    }

    #[test]
    fn refuses_an_empty_donor() {
        let donor = DonorSkin {
            positions: &[],
            triangles: &[],
            influences: &[],
        };
        assert!(matches!(
            transfer_weights(&[[0.0; 3]], &donor, 4),
            Err(TransferError::EmptyDonor)
        ));
    }

    #[test]
    fn never_invents_a_palette_slot_the_donor_lacks() {
        let (positions, triangles, influences) = quad_donor();
        let donor = DonorSkin {
            positions: &positions,
            triangles: &triangles,
            influences: &influences,
        };
        let queries: Vec<[f32; 3]> = (0..50)
            .map(|i| [i as f32 / 49.0, (i % 7) as f32 / 6.0, 0.01])
            .collect();
        let (_, report) = transfer_weights(&queries, &donor, 4).unwrap();
        let donor_slots = [4u32, 6, 8, 16];
        for slot in &report.palette_slots_used {
            assert!(donor_slots.contains(slot), "invented slot {slot}");
        }
    }
}
