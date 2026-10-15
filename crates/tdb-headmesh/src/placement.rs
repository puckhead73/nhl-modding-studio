//! Submesh placement measurement — where the *source* asset put its eyeballs and mouth.
//!
//! Path A ships the source's head surface, whose eyelids and lips were authored around
//! the source's own eyeball/mouthbag positions. The donor's rigid companion submeshes
//! (`eyel_` / `eyer_` / `mouthbag_`) sit at *donor* positions, so on a differently
//! proportioned head they read sunken or floating. The fix is measurement, not levers:
//!
//! * **eyes** — the source component ships its eyeball geometry (`mat_eyes`, or a CC5
//!   eyeball + cornea pair per side); split it into left/right clusters and take
//!   centroids. The donor eyeballs are moved onto those centers. Gaze/animation ride the
//!   per-draw register-0 slot and the UVs into the shared eye raster are untouched, so
//!   only position and size are ours to set.
//! * **mouthbag** — the source may ship no mouthbag at all (NHL24) or teeth and a tongue
//!   in its place (CC5); either way both heads have an open mouth boundary loop, so the
//!   mouthbag translates by the mouth-centroid delta. This replaces the prior effort's
//!   per-asset hand-tuned mm levers.
//!
//! Size is measured the same way position is — see [`uniform_scale_from_width`] for why
//! the ratio comes off the X extent and nothing else. The source's own eye/mouth geometry
//! is never packed; it exists here only to be measured.

/// A two-eye split: the left (+x) and right (−x) point sets.
pub type EyeClusters = (Vec<[f32; 3]>, Vec<[f32; 3]>);

/// Partition a two-eye vertex soup into (left = +x, right = −x) point sets.
///
/// Splitting at the x-midpoint of the extent is robust to unequal cluster sizes, which
/// matters because a source may ship several primitives per eye (a CC5 export has an
/// eyeball *and* a cornea shell on each side). Returns `None` when either side comes out
/// empty — that is not a two-eye mesh, and the caller must not treat it as one.
pub fn split_eye_clusters(positions: &[[f32; 3]]) -> Option<EyeClusters> {
    if positions.len() < 8 {
        return None;
    }
    let (min_x, max_x) = positions.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
        (lo.min(p[0]), hi.max(p[0]))
    });
    let mid = (min_x + max_x) / 2.0;
    let (left, right): (Vec<_>, Vec<_>) = positions.iter().partition(|p| p[0] >= mid);
    (!left.is_empty() && !right.is_empty()).then_some((left, right))
}

/// Split a two-eye vertex soup into (left = +x, right = −x) centroids.
/// Returns `None` when either side is empty (not a two-eye mesh).
pub fn split_eye_centers(positions: &[[f32; 3]]) -> Option<([f32; 3], [f32; 3])> {
    let (left, right) = split_eye_clusters(positions)?;
    let avg = |set: &[[f32; 3]]| {
        let mut sum = [0f64; 3];
        for p in set {
            for k in 0..3 {
                sum[k] += f64::from(p[k]);
            }
        }
        let n = set.len() as f64;
        [
            (sum[0] / n) as f32,
            (sum[1] / n) as f32,
            (sum[2] / n) as f32,
        ]
    };
    Some((avg(&left), avg(&right)))
}

/// Axis-aligned bounds of a point set. `None` when empty.
pub fn bounds(positions: &[[f32; 3]]) -> Option<([f32; 3], [f32; 3])> {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    (!positions.is_empty()).then_some((lo, hi))
}

/// Per-axis extent of a point set (`hi - lo`). Zero-length input gives `None`.
pub fn extents(positions: &[[f32; 3]]) -> Option<[f32; 3]> {
    bounds(positions).map(|(lo, hi)| [hi[0] - lo[0], hi[1] - lo[1], hi[2] - lo[2]])
}

/// Uniform scale that resizes a donor companion submesh to the source's measured size,
/// taken from the **X extent** ratio (source ÷ donor).
///
/// X is the only axis that is reliably commensurable between these two topologies:
///
/// * RW donor eyeballs are spherical **caps** — only the visible front of the eye is
///   modelled — so their Y and Z extents are a fraction of the sphere they came from,
///   while a CC5 eyeball is a full sphere. Both, however, span the eye's full width in X.
/// * The donor mouthbag is a closed interior volume that runs deeper than the source's
///   teeth/tongue cluster, so its Z extent overstates the mouth; the X extents are both
///   simply mouth width.
///
/// Returns `None` when either side is degenerate (a flat or empty cluster), so callers
/// fall back to translation-only rather than scaling by a meaningless ratio.
pub fn uniform_scale_from_width(source: &[[f32; 3]], donor: &[[f32; 3]]) -> Option<f32> {
    let s = extents(source)?[0];
    let d = extents(donor)?[0];
    (s > 1e-6 && d > 1e-6).then(|| s / d)
}

/// Mean distance of a cluster's vertices to its centroid — a rough eyeball radius, for
/// reporting size mismatch between source and donor eyes.
pub fn cluster_radius(positions: &[[f32; 3]], center: [f32; 3]) -> f32 {
    if positions.is_empty() {
        return 0.0;
    }
    let sum: f64 = positions
        .iter()
        .map(|p| {
            let d = [p[0] - center[0], p[1] - center[1], p[2] - center[2]];
            f64::from((d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt())
        })
        .sum();
    (sum / positions.len() as f64) as f32
}

/// Vertex sets of open boundary loops (edges referenced by exactly one triangle),
/// grouped by connectivity.
pub fn boundary_loops(vertex_count: usize, triangles: &[[u32; 3]]) -> Vec<Vec<u32>> {
    use std::collections::HashMap;
    let mut edge_count: HashMap<(u32, u32), u32> = HashMap::new();
    for t in triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = (a.min(b), a.max(b));
            *edge_count.entry(key).or_insert(0) += 1;
        }
    }
    // Adjacency restricted to boundary edges.
    let mut adj: HashMap<u32, Vec<u32>> = HashMap::new();
    for (&(a, b), &n) in &edge_count {
        if n == 1 && (a as usize) < vertex_count && (b as usize) < vertex_count {
            adj.entry(a).or_default().push(b);
            adj.entry(b).or_default().push(a);
        }
    }
    let mut seen: std::collections::HashSet<u32> = std::collections::HashSet::new();
    let mut loops = Vec::new();
    let mut seeds: Vec<u32> = adj.keys().copied().collect();
    seeds.sort_unstable();
    for seed in seeds {
        if seen.contains(&seed) {
            continue;
        }
        let mut group = Vec::new();
        let mut frontier = vec![seed];
        seen.insert(seed);
        while let Some(v) = frontier.pop() {
            group.push(v);
            for &w in adj.get(&v).into_iter().flatten() {
                if seen.insert(w) {
                    frontier.push(w);
                }
            }
        }
        group.sort_unstable();
        loops.push(group);
    }
    loops
}

fn centroid(positions: &[[f32; 3]], verts: &[u32]) -> [f32; 3] {
    let mut sum = [0f64; 3];
    for &v in verts {
        for k in 0..3 {
            sum[k] += f64::from(positions[v as usize][k]);
        }
    }
    let n = verts.len().max(1) as f64;
    [
        (sum[0] / n) as f32,
        (sum[1] / n) as f32,
        (sum[2] / n) as f32,
    ]
}

/// Centroid of the mouth-opening boundary loop of a head surface.
///
/// Rule: drop the lowest-centroid loop (the neck), then among the rest pick the
/// largest near-centerline loop (|x| < 0.03 m). `None` when no candidate survives
/// (closed-mouth head) — callers skip mouthbag alignment and say so.
pub fn mouth_centroid(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> Option<[f32; 3]> {
    mouth_loop(positions, triangles).map(|(_, c)| c)
}

/// The mouth-opening boundary loop's own vertex positions — the divider between the upper
/// and lower lip halves of the rig ([`crate::facial::MouthCrease`]).
pub fn mouth_loop_points(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> Option<Vec<[f32; 3]>> {
    mouth_loop(positions, triangles).map(|(pts, _)| pts)
}

/// Shared selection: drop the lowest-centroid loop (the neck), then take the largest
/// near-centerline loop remaining.
fn mouth_loop(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> Option<(Vec<[f32; 3]>, [f32; 3])> {
    let loops = boundary_loops(positions.len(), triangles);
    if loops.len() < 2 {
        return None;
    }
    let centroids: Vec<[f32; 3]> = loops.iter().map(|l| centroid(positions, l)).collect();
    let neck = centroids
        .iter()
        .enumerate()
        .min_by(|a, b| a.1[1].partial_cmp(&b.1[1]).unwrap())?
        .0;
    loops
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != neck && centroids[*i][0].abs() < 0.03)
        .max_by_key(|(_, l)| l.len())
        .map(|(i, l)| {
            (
                l.iter().map(|&v| positions[v as usize]).collect(),
                centroids[i],
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn eye_split_finds_two_centers() {
        // Two 4-point clusters around (±0.03, 1.83, 0.09).
        let mut pts = Vec::new();
        for s in [1.0f32, -1.0] {
            for d in [
                [0.005, 0.0, 0.0],
                [-0.005, 0.0, 0.0],
                [0.0, 0.005, 0.0],
                [0.0, -0.005, 0.0],
            ] {
                pts.push([0.03 * s + d[0], 1.83 + d[1], 0.09 + d[2]]);
            }
        }
        let (l, r) = split_eye_centers(&pts).unwrap();
        assert!((l[0] - 0.03).abs() < 1e-4 && (r[0] + 0.03).abs() < 1e-4);
        assert!((l[1] - 1.83).abs() < 1e-4);
        assert!(cluster_radius(&pts[0..4], l) > 0.004);
    }

    /// A strip with two holes: a low "neck" ring and a higher centered "mouth" ring.
    #[test]
    fn mouth_loop_picked_over_neck() {
        // Build a 3×3 quad grid (16 verts) with the center quad removed → one outer
        // boundary + one inner hole.
        let mut positions = Vec::new();
        for y in 0..4 {
            for x in 0..4 {
                positions.push([x as f32 * 0.01 - 0.015, 1.70 + y as f32 * 0.01, 0.05]);
            }
        }
        let idx = |x: u32, y: u32| y * 4 + x;
        let mut triangles = Vec::new();
        for y in 0..3u32 {
            for x in 0..3u32 {
                if x == 1 && y == 1 {
                    continue; // the hole
                }
                triangles.push([idx(x, y), idx(x + 1, y), idx(x, y + 1)]);
                triangles.push([idx(x + 1, y), idx(x + 1, y + 1), idx(x, y + 1)]);
            }
        }
        let loops = boundary_loops(positions.len(), &triangles);
        assert_eq!(loops.len(), 2);
        let m = mouth_centroid(&positions, &triangles).unwrap();
        // The hole ring centroid sits mid-grid (~1.715), above the outer ring's
        // centroid — and the outer (lower-centroid) ring was dropped as "neck".
        assert!((m[1] - 1.715).abs() < 0.003, "mouth y = {}", m[1]);
    }

    #[test]
    fn width_scale_compares_x_extents_only() {
        // A donor "eyeball cap": full width in x, but shallow in y/z. The source is a
        // full sphere of the same width. A radius-based ratio would wrongly shrink the
        // donor; the width ratio correctly reads 1.0.
        let donor = vec![
            [-0.015, 1.83, 0.09],
            [0.015, 1.83, 0.09],
            [0.0, 1.835, 0.093],
            [0.0, 1.825, 0.093],
        ];
        let source = vec![
            [-0.015, 1.83, 0.09],
            [0.015, 1.83, 0.09],
            [0.0, 1.845, 0.09],
            [0.0, 1.815, 0.09],
            [0.0, 1.83, 0.105],
            [0.0, 1.83, 0.075],
        ];
        let s = uniform_scale_from_width(&source, &donor).unwrap();
        assert!((s - 1.0).abs() < 1e-4, "width ratio = {s}");
        assert!(
            cluster_radius(&source, [0.0, 1.83, 0.09])
                > cluster_radius(&donor, [0.0, 1.83, 0.0915]),
            "radius measures disagree across cap-vs-sphere — which is why width is used"
        );
    }

    #[test]
    fn width_scale_tracks_a_genuinely_bigger_source() {
        let donor = vec![[-0.01, 0.0, 0.0], [0.01, 0.0, 0.0]];
        let source = vec![[-0.015, 0.0, 0.0], [0.015, 0.0, 0.0]];
        assert!((uniform_scale_from_width(&source, &donor).unwrap() - 1.5).abs() < 1e-5);
    }

    #[test]
    fn width_scale_refuses_a_degenerate_cluster() {
        let flat = vec![[0.0, 1.0, 2.0], [0.0, 1.5, 2.5]]; // zero x extent
        let ok = vec![[-0.01, 0.0, 0.0], [0.01, 0.0, 0.0]];
        assert_eq!(uniform_scale_from_width(&flat, &ok), None);
        assert_eq!(uniform_scale_from_width(&ok, &flat), None);
        assert_eq!(uniform_scale_from_width(&[], &ok), None);
    }

    #[test]
    fn eye_clusters_split_multi_primitive_sources() {
        // Two prims per side (eyeball + cornea), as a CC5 export ships them.
        let mut pts = Vec::new();
        for s in [1.0f32, -1.0] {
            for r in [0.005f32, 0.0052] {
                for d in [[r, 0.0, 0.0], [-r, 0.0, 0.0], [0.0, r, 0.0], [0.0, -r, 0.0]] {
                    pts.push([0.03 * s + d[0], 1.83 + d[1], 0.09 + d[2]]);
                }
            }
        }
        let (left, right) = split_eye_clusters(&pts).unwrap();
        assert_eq!(left.len(), 8, "both prims of the +x eye land together");
        assert_eq!(right.len(), 8);
        // Each side spans one eye's width, not the interpupillary distance.
        assert!(extents(&left).unwrap()[0] < 0.012);
    }

    #[test]
    fn closed_surface_has_no_mouth() {
        // Single triangle: one boundary loop only → None.
        let positions = vec![[0.0, 1.7, 0.0], [0.01, 1.7, 0.0], [0.0, 1.71, 0.0]];
        assert_eq!(mouth_centroid(&positions, &[[0, 1, 2]]), None);
    }
}
