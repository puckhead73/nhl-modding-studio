//! N19-H2b — conservative Path B fitting for the fingerprinted NHL 19 head donor.
//!
//! Everything here is offline and deterministic. It shares one contract with
//! [`crate::morph`]'s Blender route — freeze the boundary, freeze the eyelid pockets,
//! recompute shading afterwards — but implements the wrap in-process rather than through
//! `blender-job`, for two reasons worth recording:
//!
//! * The hard-frozen positions must come back **bit-identical**. A round trip out to
//!   Blender and back through an interchange file cannot promise that, and the writer
//!   asserts it.
//! * [`crate::morph::MorphDonor`] describes a freeze as two radii around each eyeball
//!   plus N boundary rings. That is not enough for this donor: its `geo_headShape`
//!   carries the oral cavity as interior surface of the same shell and extends into the
//!   torso, so protection has to be a per-vertex mask set, not a radius. The mask set
//!   below is the extended input contract; the Blender route can consume the same ids.
//!
//! # What the masks are, and what they are not
//!
//! Masks are derived from the validated index buffer and from geometry — never from
//! joint names, and never from an unverified flood fill. Interior surfaces are found by
//! an explicit occlusion test (cast +Z from the vertex; if the mesh is in front of it, it
//! is interior), which is what "interior" means to a viewer. Where a classification is
//! not decidable, the vertices are frozen rather than guessed.

use std::collections::{BTreeMap, HashMap, HashSet};

use tdb_rx2::mcd_write::McdContainer;

/// Query rays are nudged off the surface by this much before testing occlusion, so a
/// vertex never reports its own triangles as being in front of it.
const RAY_EPS: f32 = 1.0e-5;

// ============================================================================= mesh

/// The donor head surface, with the adjacency an analysis needs.
pub struct DonorMesh {
    pub positions: Vec<[f32; 3]>,
    pub triangles: Vec<[u32; 3]>,
    pub ring: Vec<Vec<u32>>,
    /// Boundary components, largest first. Every boundary vertex on this donor has
    /// degree exactly two, so these are closed loops.
    pub boundary_components: Vec<Vec<u32>>,
}

impl DonorMesh {
    pub fn from_container(c: &McdContainer, submesh: &str) -> Option<Self> {
        let sub = c.submesh_index(submesh)?;
        let stream = c.position_stream_of(sub)?;
        let positions = c.streams[stream].positions().to_vec();
        let triangles = c.triangles_of(sub)?;
        Some(Self::new(positions, triangles))
    }

    pub fn new(positions: Vec<[f32; 3]>, triangles: Vec<[u32; 3]>) -> Self {
        let n = positions.len();
        let mut edge: HashMap<(u32, u32), u32> = HashMap::new();
        let mut ring_set: Vec<HashSet<u32>> = vec![HashSet::new(); n];
        for t in &triangles {
            for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
                *edge.entry((a.min(b), a.max(b))).or_default() += 1;
                ring_set[a as usize].insert(b);
                ring_set[b as usize].insert(a);
            }
        }
        let ring: Vec<Vec<u32>> = ring_set
            .into_iter()
            .map(|s| {
                let mut v: Vec<u32> = s.into_iter().collect();
                v.sort_unstable();
                v
            })
            .collect();

        let mut badj: HashMap<u32, Vec<u32>> = HashMap::new();
        for (&(a, b), &count) in &edge {
            if count == 1 {
                badj.entry(a).or_default().push(b);
                badj.entry(b).or_default().push(a);
            }
        }
        let mut keys: Vec<u32> = badj.keys().copied().collect();
        keys.sort_unstable();
        let mut seen: HashSet<u32> = HashSet::new();
        let mut comps: Vec<Vec<u32>> = Vec::new();
        for v in keys {
            if seen.contains(&v) {
                continue;
            }
            let mut stack = vec![v];
            let mut comp = Vec::new();
            while let Some(x) = stack.pop() {
                if !seen.insert(x) {
                    continue;
                }
                comp.push(x);
                stack.extend(badj[&x].iter().copied().filter(|y| !seen.contains(y)));
            }
            comp.sort_unstable();
            comps.push(comp);
        }
        comps.sort_by_key(|c| std::cmp::Reverse(c.len()));

        Self {
            positions,
            triangles,
            ring,
            boundary_components: comps,
        }
    }

    /// Vertices reachable within `rings` edge steps of `seed`, excluding `seed` itself.
    pub fn grow(&self, seed: &HashSet<u32>, rings: usize) -> HashSet<u32> {
        let mut out = seed.clone();
        let mut cur = seed.clone();
        for _ in 0..rings {
            let mut next = HashSet::new();
            for &v in &cur {
                next.extend(self.ring[v as usize].iter().copied());
            }
            next.retain(|v| !out.contains(v));
            out.extend(next.iter().copied());
            cur = next;
        }
        out.difference(seed).copied().collect()
    }

    /// Area-weighted vertex normals.
    pub fn vertex_normals(&self) -> Vec<[f32; 3]> {
        let mut n = vec![[0f32; 3]; self.positions.len()];
        for t in &self.triangles {
            let (a, b, c) = (
                self.positions[t[0] as usize],
                self.positions[t[1] as usize],
                self.positions[t[2] as usize],
            );
            let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
            let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
            let f = [
                u[1] * v[2] - u[2] * v[1],
                u[2] * v[0] - u[0] * v[2],
                u[0] * v[1] - u[1] * v[0],
            ];
            for &i in t {
                for k in 0..3 {
                    n[i as usize][k] += f[k];
                }
            }
        }
        for v in n.iter_mut() {
            let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
            if l > 1e-12 {
                for c in v.iter_mut() {
                    *c /= l;
                }
            }
        }
        n
    }

    /// Does any triangle sit in front of `v` along +Z? That is what makes a surface
    /// *interior* rather than visible: the oral cavity and the inner eye socket are
    /// occluded by the lips and lids, the back of the head by the face.
    pub fn occluded_along_z(&self, v: u32) -> bool {
        let p = self.positions[v as usize];
        let origin = [p[0], p[1], p[2] + RAY_EPS];
        for t in &self.triangles {
            if t.contains(&v) {
                continue;
            }
            if ray_z_hits(
                origin,
                self.positions[t[0] as usize],
                self.positions[t[1] as usize],
                self.positions[t[2] as usize],
            ) {
                return true;
            }
        }
        false
    }
}

/// Ray from `o` along +Z against triangle `abc`; true when it hits in front.
fn ray_z_hits(o: [f32; 3], a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> bool {
    // 2D point-in-triangle in XY, then depth compare.
    let d1 = (o[0] - b[0]) * (a[1] - b[1]) - (a[0] - b[0]) * (o[1] - b[1]);
    let d2 = (o[0] - c[0]) * (b[1] - c[1]) - (b[0] - c[0]) * (o[1] - c[1]);
    let d3 = (o[0] - a[0]) * (c[1] - a[1]) - (c[0] - a[0]) * (o[1] - a[1]);
    let neg = d1 < 0.0 || d2 < 0.0 || d3 < 0.0;
    let pos = d1 > 0.0 || d2 > 0.0 || d3 > 0.0;
    if neg && pos {
        return false;
    }
    // Barycentric interpolation of z at (o.x, o.y).
    let den = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if den.abs() < 1e-12 {
        return false;
    }
    let l1 = ((b[1] - c[1]) * (o[0] - c[0]) + (c[0] - b[0]) * (o[1] - c[1])) / den;
    let l2 = ((c[1] - a[1]) * (o[0] - c[0]) + (a[0] - c[0]) * (o[1] - c[1])) / den;
    let l3 = 1.0 - l1 - l2;
    let z = l1 * a[2] + l2 * b[2] + l3 * c[2];
    z > o[2]
}

// ============================================================================ masks

/// One named vertex-id set, with how it was produced.
#[derive(Debug, Clone)]
pub struct MaskSet {
    pub name: String,
    pub ids: Vec<u32>,
    pub method: String,
}

/// The full protection contract for one donor.
#[derive(Debug, Clone)]
pub struct DonorMasks {
    pub donor_sha256: String,
    pub submesh: String,
    pub vertex_count: usize,
    pub sets: Vec<MaskSet>,
    /// Union of every hard-frozen set: these positions must come back bit-identical.
    pub hard_frozen: HashSet<u32>,
    /// Explicitly constructed, not the complement of `hard_frozen`.
    pub editable: HashSet<u32>,
    /// Coincident groups along the oral rim: exact-match groups and their pair distances.
    pub seam_exact_groups: usize,
    pub seam_rounded_groups: usize,
    pub seam_max_pair_distance: f32,
    pub notes: Vec<String>,
}

impl DonorMasks {
    pub fn get(&self, name: &str) -> &[u32] {
        self.sets
            .iter()
            .find(|s| s.name == name)
            .map(|s| s.ids.as_slice())
            .unwrap_or(&[])
    }

    /// Invariants a caller may rely on. Returns the failures, empty when sound.
    pub fn validate(&self) -> Vec<String> {
        let mut bad = Vec::new();
        for s in &self.sets {
            if let Some(v) = s.ids.iter().find(|&&v| v as usize >= self.vertex_count) {
                bad.push(format!("mask {:?} holds out-of-range id {v}", s.name));
            }
            let mut sorted = s.ids.clone();
            sorted.sort_unstable();
            sorted.dedup();
            if sorted.len() != s.ids.len() {
                bad.push(format!("mask {:?} holds duplicate ids", s.name));
            }
        }
        let overlap: Vec<u32> = self
            .editable
            .intersection(&self.hard_frozen)
            .copied()
            .collect();
        if !overlap.is_empty() {
            bad.push(format!(
                "{} vertices are both editable and hard-frozen (e.g. {:?})",
                overlap.len(),
                &overlap[..overlap.len().min(8)]
            ));
        }
        bad
    }
}

/// Rounding used to group coincident seam vertices, in metres. Reported alongside the
/// exact-match count so the two are never conflated.
pub const SEAM_ROUND_M: f32 = 1.0e-6;

/// Build the conservative mask set.
///
/// `covered` lists the donor vertices for which the aligned reference actually supplies a
/// trustworthy corresponding surface — the caller decides that by correspondence distance,
/// not by a bounding box, because a box says nothing about whether a surface is there.
/// Everything outside it is frozen. Passing `None` freezes on geometry alone.
pub fn build_masks(
    c: &McdContainer,
    mesh: &DonorMesh,
    submesh: &str,
    donor_sha256: &str,
    covered: Option<&HashSet<u32>>,
) -> DonorMasks {
    let n = mesh.positions.len();
    let mut sets: Vec<MaskSet> = Vec::new();
    let mut notes: Vec<String> = Vec::new();
    let add = |sets: &mut Vec<MaskSet>, name: &str, ids: &HashSet<u32>, method: &str| {
        let mut v: Vec<u32> = ids.iter().copied().collect();
        v.sort_unstable();
        sets.push(MaskSet {
            name: name.to_string(),
            ids: v,
            method: method.to_string(),
        });
    };

    // ---- Outer shell cut: the largest boundary component. On this donor it is the
    // shoulder/chest rim plus the rear midline scalp seam — the body attachment.
    let shell: HashSet<u32> = mesh
        .boundary_components
        .first()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .collect();
    let shell_t = mesh.grow(&shell, 2);
    add(
        &mut sets,
        "shell_boundary",
        &shell,
        "largest boundary component",
    );
    add(
        &mut sets,
        "shell_transition",
        &shell_t,
        "2 rings from shell_boundary",
    );

    // ---- Oral rim: every other boundary component, each verified by location before it
    // is called oral. Boundary size alone is not an anatomical classification.
    let mut oral: HashSet<u32> = HashSet::new();
    let mut non_oral: HashSet<u32> = HashSet::new();
    // Reference: the eyeball submeshes give a reliable mid-face height.
    let eye_y = ["geo_reyeShape", "geo_leyeShape"]
        .iter()
        .filter_map(|nm| c.submesh_index(nm).and_then(|i| c.submeshes[i].bounds))
        .map(|(mn, mx)| (mn[1] + mx[1]) * 0.5)
        .sum::<f32>()
        / 2.0;
    for comp in mesh.boundary_components.iter().skip(1) {
        let (mut lo, mut hi) = ([f32::MAX; 3], [f32::MIN; 3]);
        for &v in comp {
            let p = mesh.positions[v as usize];
            for k in 0..3 {
                lo[k] = lo[k].min(p[k]);
                hi[k] = hi[k].max(p[k]);
            }
        }
        let cy = (lo[1] + hi[1]) * 0.5;
        let cx = (lo[0] + hi[0]) * 0.5;
        // Below the eyes, near the midline, and on the front half of the head.
        let looks_oral = cy < eye_y && cx.abs() < 0.06 && hi[2] > 0.0;
        if looks_oral {
            oral.extend(comp.iter().copied());
        } else {
            non_oral.extend(comp.iter().copied());
            notes.push(format!(
                "boundary component of {} vertices at x[{:.4},{:.4}] y[{:.4},{:.4}] z[{:.4},{:.4}] \
                 is not classified oral; frozen as unclassified",
                comp.len(),
                lo[0],
                hi[0],
                lo[1],
                hi[1],
                lo[2],
                hi[2]
            ));
        }
    }
    add(
        &mut sets,
        "oral_rim",
        &oral,
        "non-shell boundary components below the eyes, near the midline, front half",
    );
    if !non_oral.is_empty() {
        add(
            &mut sets,
            "unclassified_boundary",
            &non_oral,
            "boundary component failing the oral location test",
        );
    }

    // ---- Oral cavity: interior surface behind the lip seam. Bounded by the rim's own
    // extent (plus a margin) so no traversal can flood the outer face, and confirmed by
    // the occlusion test rather than by connectivity alone.
    let (mut olo, mut ohi) = ([f32::MAX; 3], [f32::MIN; 3]);
    for &v in &oral {
        let p = mesh.positions[v as usize];
        for k in 0..3 {
            olo[k] = olo[k].min(p[k]);
            ohi[k] = ohi[k].max(p[k]);
        }
    }
    let margin = [0.020f32, 0.020, 0.060];
    let mut cavity: HashSet<u32> = HashSet::new();
    for v in 0..n as u32 {
        if oral.contains(&v) {
            continue;
        }
        let p = mesh.positions[v as usize];
        let inside = (0..3).all(|k| p[k] >= olo[k] - margin[k] && p[k] <= ohi[k] + margin[k]);
        if inside && mesh.occluded_along_z(v) {
            cavity.insert(v);
        }
    }
    add(
        &mut sets,
        "oral_cavity",
        &cavity,
        "inside the oral-rim box (+20/20/60 mm) and occluded along +Z",
    );
    let oral_seed: HashSet<u32> = oral.union(&cavity).copied().collect();
    let oral_t = mesh.grow(&oral_seed, 2);
    add(
        &mut sets,
        "oral_transition",
        &oral_t,
        "2 rings from oral_rim ∪ oral_cavity",
    );

    // ---- Coincident seam groups: exact, and rounded, reported separately.
    let mut exact: BTreeMap<[u32; 3], Vec<u32>> = BTreeMap::new();
    let mut rounded: BTreeMap<(i64, i64, i64), Vec<u32>> = BTreeMap::new();
    for &v in &oral {
        let p = mesh.positions[v as usize];
        exact
            .entry([p[0].to_bits(), p[1].to_bits(), p[2].to_bits()])
            .or_default()
            .push(v);
        rounded
            .entry((
                (p[0] / SEAM_ROUND_M).round() as i64,
                (p[1] / SEAM_ROUND_M).round() as i64,
                (p[2] / SEAM_ROUND_M).round() as i64,
            ))
            .or_default()
            .push(v);
    }
    let seam_exact_groups = exact.values().filter(|g| g.len() > 1).count();
    let seam_rounded_groups = rounded.values().filter(|g| g.len() > 1).count();
    let mut max_pair = 0f32;
    for g in rounded.values().filter(|g| g.len() > 1) {
        for i in 0..g.len() {
            for j in i + 1..g.len() {
                let (a, b) = (mesh.positions[g[i] as usize], mesh.positions[g[j] as usize]);
                let dd =
                    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt();
                max_pair = max_pair.max(dd);
            }
        }
    }

    // Upper/lower is a *proposal* from 1-ring mean height, not proof of lip mechanics.
    // Anything not a clean pair, or whose two sides are not clearly separated, stays
    // ambiguous and frozen.
    let ring_y = |v: u32| -> f32 {
        let r = &mesh.ring[v as usize];
        if r.is_empty() {
            return mesh.positions[v as usize][1];
        }
        r.iter()
            .map(|&j| mesh.positions[j as usize][1])
            .sum::<f32>()
            / r.len() as f32
    };
    const RING_Y_MIN_SEP: f32 = 2.0e-4;
    let (mut upper, mut lower, mut ambiguous) = (HashSet::new(), HashSet::new(), HashSet::new());
    for g in rounded.values() {
        if g.len() == 2 {
            let (a, b) = (g[0], g[1]);
            let (ya, yb) = (ring_y(a), ring_y(b));
            if (ya - yb).abs() < RING_Y_MIN_SEP {
                ambiguous.extend(g.iter().copied());
            } else if ya > yb {
                upper.insert(a);
                lower.insert(b);
            } else {
                upper.insert(b);
                lower.insert(a);
            }
        } else {
            ambiguous.extend(g.iter().copied());
        }
    }
    add(
        &mut sets,
        "oral_seam_upper_proposed",
        &upper,
        "1-ring mean height, clean coincident pairs only",
    );
    add(
        &mut sets,
        "oral_seam_lower_proposed",
        &lower,
        "1-ring mean height, clean coincident pairs only",
    );
    add(
        &mut sets,
        "oral_seam_ambiguous",
        &ambiguous,
        "non-pair groups, or pairs separated by < 0.2 mm",
    );

    // ---- Eyes: pockets around each eyeball, plus the inner socket behind it.
    let mut eyelids: HashMap<&str, HashSet<u32>> = HashMap::new();
    for nm in ["geo_reyeShape", "geo_leyeShape"] {
        let Some(i) = c.submesh_index(nm) else {
            continue;
        };
        let Some((mn, mx)) = c.submeshes[i].bounds else {
            continue;
        };
        let ctr = [
            (mn[0] + mx[0]) * 0.5,
            (mn[1] + mx[1]) * 0.5,
            (mn[2] + mx[2]) * 0.5,
        ];
        let half = ((mx[0] - mn[0]).powi(2) + (mx[1] - mn[1]).powi(2) + (mx[2] - mn[2]).powi(2))
            .sqrt()
            * 0.5;
        // Wider than H2a's 1.35: the fold above the lid and the crease below it sit
        // outside the eyeball's own diagonal.
        let r = half * 1.9;
        let mut s: HashSet<u32> = (0..n as u32)
            .filter(|&k| {
                let p = mesh.positions[k as usize];
                (p[0] - ctr[0]).powi(2) + (p[1] - ctr[1]).powi(2) + (p[2] - ctr[2]).powi(2) < r * r
            })
            .collect();
        // Inner socket: anything in the eye's own box that the lids occlude.
        let box_r = half * 2.2;
        for k in 0..n as u32 {
            let p = mesh.positions[k as usize];
            let near = (0..3).all(|a| (p[a] - ctr[a]).abs() < box_r);
            if near && !s.contains(&k) && mesh.occluded_along_z(k) {
                s.insert(k);
            }
        }
        eyelids.insert(nm, s);
    }
    let re = eyelids.remove("geo_reyeShape").unwrap_or_default();
    let le = eyelids.remove("geo_leyeShape").unwrap_or_default();
    add(
        &mut sets,
        "eyelid_pocket_right",
        &re,
        "within 1.9x eyeball half-diagonal, plus occluded socket",
    );
    add(
        &mut sets,
        "eyelid_pocket_left",
        &le,
        "within 1.9x eyeball half-diagonal, plus occluded socket",
    );

    // ---- Everything the reference cannot vouch for. Torso and scalp are frozen because
    // the source has no corresponding surface there, not because of a height guess.
    let mut no_correspondence: HashSet<u32> = HashSet::new();
    if let Some(cov) = covered {
        for v in 0..n as u32 {
            if !cov.contains(&v) {
                no_correspondence.insert(v);
            }
        }
        add(
            &mut sets,
            "no_source_correspondence",
            &no_correspondence,
            "no aligned reference surface within the caller's correspondence threshold",
        );
    } else {
        notes.push("no source coverage supplied; only geometric protection applied".into());
    }

    // Back-facing and interior surfaces are never fit from a front-facing reference.
    let normals = mesh.vertex_normals();
    let mut interior_or_back: HashSet<u32> = HashSet::new();
    for v in 0..n as u32 {
        if normals[v as usize][2] <= 0.0 || mesh.occluded_along_z(v) {
            interior_or_back.insert(v);
        }
    }
    add(
        &mut sets,
        "interior_or_back_facing",
        &interior_or_back,
        "vertex normal z <= 0, or occluded along +Z",
    );

    let mut hard_frozen: HashSet<u32> = HashSet::new();
    for name in [
        "shell_boundary",
        "shell_transition",
        "oral_rim",
        "oral_cavity",
        "oral_transition",
        "oral_seam_ambiguous",
        "unclassified_boundary",
        "eyelid_pocket_right",
        "eyelid_pocket_left",
        "no_source_correspondence",
        "interior_or_back_facing",
    ] {
        hard_frozen.extend(
            sets.iter()
                .find(|s| s.name == name)
                .map(|s| s.ids.clone())
                .unwrap_or_default(),
        );
    }

    // ---- Editable set, constructed positively: a front-facing outer-face vertex, inside
    // the reference's coverage, that no protection claims.
    let mut editable: HashSet<u32> = HashSet::new();
    for v in 0..n as u32 {
        if hard_frozen.contains(&v) {
            continue;
        }
        if normals[v as usize][2] <= 0.0 {
            continue;
        }
        if let Some(cov) = covered {
            if !cov.contains(&v) {
                continue;
            }
        }
        editable.insert(v);
    }
    add(
        &mut sets,
        "editable_outer_face",
        &editable,
        "front-facing, inside reference coverage, unclaimed by any protection",
    );

    DonorMasks {
        donor_sha256: donor_sha256.to_string(),
        submesh: submesh.to_string(),
        vertex_count: n,
        sets,
        hard_frozen,
        editable,
        seam_exact_groups,
        seam_rounded_groups,
        seam_max_pair_distance: max_pair,
        notes,
    }
}

// ======================================================================== alignment

/// A similarity transform: uniform scale, rotation, translation.
#[derive(Debug, Clone, Copy)]
pub struct Similarity {
    pub scale: f32,
    pub rot: [[f32; 3]; 3],
    pub translation: [f32; 3],
}

impl Similarity {
    pub fn identity() -> Self {
        Self {
            scale: 1.0,
            rot: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            translation: [0.0; 3],
        }
    }

    pub fn apply(&self, p: [f32; 3]) -> [f32; 3] {
        let r = &self.rot;
        [
            self.scale * (r[0][0] * p[0] + r[0][1] * p[1] + r[0][2] * p[2]) + self.translation[0],
            self.scale * (r[1][0] * p[0] + r[1][1] * p[1] + r[1][2] * p[2]) + self.translation[1],
            self.scale * (r[2][0] * p[0] + r[2][1] * p[1] + r[2][2] * p[2]) + self.translation[2],
        ]
    }
}

/// Build a similarity from two eye centres plus a nose tip, via an explicit orthonormal
/// frame.
///
/// A three-point Umeyama fit is **rank deficient**: three points span a plane, the
/// covariance has a zero singular value, and the rotation about that plane's normal — and
/// with it the reflection sign — is not determined. Fitting this triple with
/// [`umeyama`] on the real donor produced a reflected transform with a scale of −0.099.
/// Constructing the frame instead is exact, reflection-free by construction, and gives a
/// proper rotation (det = +1) for any non-degenerate landmark triple.
///
/// The frame is: `x` along right→left eye, `y` from the nose towards the eye midpoint
/// (up), `z = x × y`, then `y` re-orthogonalised. Scale is the interocular ratio.
pub fn frame_similarity(
    src_right_eye: [f32; 3],
    src_left_eye: [f32; 3],
    src_nose: [f32; 3],
    dst_right_eye: [f32; 3],
    dst_left_eye: [f32; 3],
    dst_nose: [f32; 3],
) -> Option<Similarity> {
    fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
    }
    fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
        [
            a[1] * b[2] - a[2] * b[1],
            a[2] * b[0] - a[0] * b[2],
            a[0] * b[1] - a[1] * b[0],
        ]
    }
    fn unit(v: [f32; 3]) -> Option<[f32; 3]> {
        let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
        (l > 1e-9).then(|| [v[0] / l, v[1] / l, v[2] / l])
    }
    /// Columns `x`, `y`, `z` of an orthonormal frame, plus its origin.
    fn frame(re: [f32; 3], le: [f32; 3], nose: [f32; 3]) -> Option<([[f32; 3]; 3], [f32; 3], f32)> {
        let span = sub(le, re);
        let width = (span[0] * span[0] + span[1] * span[1] + span[2] * span[2]).sqrt();
        let x = unit(span)?;
        let mid = [
            (re[0] + le[0]) * 0.5,
            (re[1] + le[1]) * 0.5,
            (re[2] + le[2]) * 0.5,
        ];
        let up = sub(mid, nose);
        let z = unit(cross(x, up))?;
        let y = cross(z, x); // already unit: z ⟂ x, both unit
        Some(([x, y, z], mid, width))
    }

    let (fs, ms, ws) = frame(src_right_eye, src_left_eye, src_nose)?;
    let (fd, md, wd) = frame(dst_right_eye, dst_left_eye, dst_nose)?;
    if ws <= 1e-9 {
        return None;
    }
    let scale = wd / ws;
    // R = Fd · Fsᵀ, where each frame's columns are its axes.
    let mut rot = [[0f32; 3]; 3];
    for (i, row) in rot.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| fd[k][i] * fs[k][j]).sum();
        }
    }
    let mut translation = [0f32; 3];
    for k in 0..3 {
        translation[k] =
            md[k] - scale * (rot[k][0] * ms[0] + rot[k][1] * ms[1] + rot[k][2] * ms[2]);
    }
    Some(Similarity {
        scale,
        rot,
        translation,
    })
}

/// Umeyama similarity fit mapping `from` onto `to` (equal length, >= 3 points).
pub fn umeyama(from: &[[f32; 3]], to: &[[f32; 3]]) -> Option<Similarity> {
    let n = from.len();
    if n < 3 || to.len() != n {
        return None;
    }
    let mean = |v: &[[f32; 3]]| {
        let mut m = [0f64; 3];
        for p in v {
            for k in 0..3 {
                m[k] += p[k] as f64;
            }
        }
        [m[0] / n as f64, m[1] / n as f64, m[2] / n as f64]
    };
    let (mf, mt) = (mean(from), mean(to));
    let mut cov = [[0f64; 3]; 3];
    let mut var_f = 0f64;
    for i in 0..n {
        let a = [
            from[i][0] as f64 - mf[0],
            from[i][1] as f64 - mf[1],
            from[i][2] as f64 - mf[2],
        ];
        let b = [
            to[i][0] as f64 - mt[0],
            to[i][1] as f64 - mt[1],
            to[i][2] as f64 - mt[2],
        ];
        var_f += a[0] * a[0] + a[1] * a[1] + a[2] * a[2];
        for r in 0..3 {
            for c in 0..3 {
                cov[r][c] += b[r] * a[c];
            }
        }
    }
    for r in cov.iter_mut() {
        for c in r.iter_mut() {
            *c /= n as f64;
        }
    }
    var_f /= n as f64;
    let (u, s, vt) = svd3(cov);
    // Reflection guard.
    let det = det3(&mat_mul(&u, &vt));
    let d = if det < 0.0 {
        [1.0, 1.0, -1.0]
    } else {
        [1.0; 3]
    };
    let mut dm = [[0f64; 3]; 3];
    for k in 0..3 {
        dm[k][k] = d[k];
    }
    let rot = mat_mul(&mat_mul(&u, &dm), &vt);
    let trace: f64 = (0..3).map(|k| s[k] * d[k]).sum();
    let scale = if var_f > 1e-18 { trace / var_f } else { 1.0 };
    let mut t = [0f64; 3];
    for k in 0..3 {
        t[k] = mt[k] - scale * (rot[k][0] * mf[0] + rot[k][1] * mf[1] + rot[k][2] * mf[2]);
    }
    Some(Similarity {
        scale: scale as f32,
        rot: [
            [rot[0][0] as f32, rot[0][1] as f32, rot[0][2] as f32],
            [rot[1][0] as f32, rot[1][1] as f32, rot[1][2] as f32],
            [rot[2][0] as f32, rot[2][1] as f32, rot[2][2] as f32],
        ],
        translation: [t[0] as f32, t[1] as f32, t[2] as f32],
    })
}

fn mat_mul(a: &[[f64; 3]; 3], b: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut o = [[0f64; 3]; 3];
    for (i, row) in o.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..3).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

fn det3(m: &[[f64; 3]; 3]) -> f64 {
    m[0][0] * (m[1][1] * m[2][2] - m[1][2] * m[2][1])
        - m[0][1] * (m[1][0] * m[2][2] - m[1][2] * m[2][0])
        + m[0][2] * (m[1][0] * m[2][1] - m[1][1] * m[2][0])
}

fn transpose(m: &[[f64; 3]; 3]) -> [[f64; 3]; 3] {
    let mut o = [[0f64; 3]; 3];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = m[j][i];
        }
    }
    o
}

/// One-sided Jacobi SVD of a 3×3, returning `(U, singular values, Vᵀ)`.
fn svd3(a: [[f64; 3]; 3]) -> ([[f64; 3]; 3], [f64; 3], [[f64; 3]; 3]) {
    // Work on the columns of `w`, accumulating the right-hand rotations in `v`.
    let mut w = a;
    let mut v = [[0f64; 3]; 3];
    for (k, row) in v.iter_mut().enumerate() {
        row[k] = 1.0;
    }
    for _ in 0..64 {
        let mut off = 0f64;
        for p in 0..2 {
            for q in p + 1..3 {
                let mut alpha = 0f64;
                let mut beta = 0f64;
                let mut gamma = 0f64;
                for row in w.iter() {
                    alpha += row[p] * row[p];
                    beta += row[q] * row[q];
                    gamma += row[p] * row[q];
                }
                off += gamma.abs();
                if gamma.abs() < 1e-18 {
                    continue;
                }
                let zeta = (beta - alpha) / (2.0 * gamma);
                let t = zeta.signum() / (zeta.abs() + (1.0 + zeta * zeta).sqrt());
                let c = 1.0 / (1.0 + t * t).sqrt();
                let s = c * t;
                for k in 0..3 {
                    let (wp, wq) = (w[k][p], w[k][q]);
                    w[k][p] = c * wp - s * wq;
                    w[k][q] = s * wp + c * wq;
                    let (vp, vq) = (v[k][p], v[k][q]);
                    v[k][p] = c * vp - s * vq;
                    v[k][q] = s * vp + c * vq;
                }
            }
        }
        if off < 1e-15 {
            break;
        }
    }
    let mut sv = [0f64; 3];
    let mut u = [[0f64; 3]; 3];
    for j in 0..3 {
        let norm = (0..3).map(|k| w[k][j] * w[k][j]).sum::<f64>().sqrt();
        sv[j] = norm;
        for k in 0..3 {
            u[k][j] = if norm > 1e-18 { w[k][j] / norm } else { 0.0 };
        }
    }
    // Guarantee an orthonormal U even when a singular value collapses.
    if sv[2] <= 1e-18 {
        let c0 = [u[0][0], u[1][0], u[2][0]];
        let c1 = [u[0][1], u[1][1], u[2][1]];
        let c2 = [
            c0[1] * c1[2] - c0[2] * c1[1],
            c0[2] * c1[0] - c0[0] * c1[2],
            c0[0] * c1[1] - c0[1] * c1[0],
        ];
        for k in 0..3 {
            u[k][2] = c2[k];
        }
    }
    (u, sv, transpose(&v))
}

// =========================================================================== source

/// The reference head, with its scene transform already applied exactly once and its
/// parts identified by **node/mesh name**, not by material (both primitives in the
/// supplied file share one material called `None`).
#[derive(Debug, Clone)]
pub struct SourceHead {
    pub head: Vec<[f32; 3]>,
    pub head_triangles: Vec<[u32; 3]>,
    pub head_uvs: Vec<[f32; 2]>,
    pub eyes: Vec<[f32; 3]>,
    pub eyes_triangles: Vec<[u32; 3]>,
    /// One line per node describing what was found and what transform was applied.
    pub provenance: Vec<String>,
}

#[derive(Debug, thiserror::Error)]
pub enum SourceError {
    #[error("glb: {0}")]
    Glb(#[from] crate::glb::GlbError),
    #[error("glb json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("glb is truncated before its JSON chunk")]
    Truncated,
    #[error(
        "this reader handles one primitive per mesh (found {meshes} mesh(es) and {prims} \
         primitive(s)); refusing rather than pairing them by order"
    )]
    PrimitiveMeshMismatch { meshes: usize, prims: usize },
    #[error("no node whose name contains {0:?}")]
    MissingPart(&'static str),
}

/// Multiply a 4×4 (row-major) by a point.
fn xform(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2] + m[0][3],
        m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2] + m[1][3],
        m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2] + m[2][3],
    ]
}

fn mat4_mul(a: &[[f32; 4]; 4], b: &[[f32; 4]; 4]) -> [[f32; 4]; 4] {
    let mut o = [[0f32; 4]; 4];
    for (i, row) in o.iter_mut().enumerate() {
        for (j, cell) in row.iter_mut().enumerate() {
            *cell = (0..4).map(|k| a[i][k] * b[k][j]).sum();
        }
    }
    o
}

/// Node TRS → matrix, following the glTF order `T · R · S`.
fn trs_matrix(node: &serde_json::Value) -> [[f32; 4]; 4] {
    if let Some(m) = node.get("matrix").and_then(|v| v.as_array()) {
        // glTF stores column-major; transpose into our row-major convention.
        let f: Vec<f32> = m
            .iter()
            .filter_map(|x| x.as_f64())
            .map(|x| x as f32)
            .collect();
        if f.len() == 16 {
            let mut o = [[0f32; 4]; 4];
            for c in 0..4 {
                for r in 0..4 {
                    o[r][c] = f[c * 4 + r];
                }
            }
            return o;
        }
    }
    let get3 = |k: &str, d: [f32; 3]| -> [f32; 3] {
        node.get(k)
            .and_then(|v| v.as_array())
            .map(|a| {
                let f: Vec<f32> = a
                    .iter()
                    .filter_map(|x| x.as_f64())
                    .map(|x| x as f32)
                    .collect();
                if f.len() == 3 {
                    [f[0], f[1], f[2]]
                } else {
                    d
                }
            })
            .unwrap_or(d)
    };
    let t = get3("translation", [0.0; 3]);
    let s = get3("scale", [1.0; 3]);
    let q = node
        .get("rotation")
        .and_then(|v| v.as_array())
        .map(|a| {
            let f: Vec<f32> = a
                .iter()
                .filter_map(|x| x.as_f64())
                .map(|x| x as f32)
                .collect();
            if f.len() == 4 {
                [f[0], f[1], f[2], f[3]]
            } else {
                [0.0, 0.0, 0.0, 1.0]
            }
        })
        .unwrap_or([0.0, 0.0, 0.0, 1.0]);
    let (x, y, z, w) = (q[0], q[1], q[2], q[3]);
    let r = [
        [
            1.0 - 2.0 * (y * y + z * z),
            2.0 * (x * y - z * w),
            2.0 * (x * z + y * w),
        ],
        [
            2.0 * (x * y + z * w),
            1.0 - 2.0 * (x * x + z * z),
            2.0 * (y * z - x * w),
        ],
        [
            2.0 * (x * z - y * w),
            2.0 * (y * z + x * w),
            1.0 - 2.0 * (x * x + y * y),
        ],
    ];
    let mut o = [[0f32; 4]; 4];
    for i in 0..3 {
        for j in 0..3 {
            o[i][j] = r[i][j] * s[j];
        }
        o[i][3] = t[i];
    }
    o[3][3] = 1.0;
    o
}

impl SourceHead {
    /// Parse a GLB, walking `scenes → nodes → mesh` and accumulating each node's
    /// transform down the tree so it is applied exactly once.
    pub fn parse(glb: &[u8]) -> Result<Self, SourceError> {
        let mesh = crate::glb::parse_glb(glb)?;
        if glb.len() < 20 {
            return Err(SourceError::Truncated);
        }
        let json_len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let body = glb.get(20..20 + json_len).ok_or(SourceError::Truncated)?;
        let doc: serde_json::Value = serde_json::from_slice(body)?;

        let meshes = doc["meshes"].as_array().map(|a| a.len()).unwrap_or(0);
        if meshes != mesh.primitives.len() {
            return Err(SourceError::PrimitiveMeshMismatch {
                meshes,
                prims: mesh.primitives.len(),
            });
        }

        // Walk the active scene, accumulating transforms.
        let nodes = doc["nodes"].as_array().cloned().unwrap_or_default();
        let scene_ix = doc["scene"].as_u64().unwrap_or(0) as usize;
        let roots: Vec<usize> = doc["scenes"][scene_ix]["nodes"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|v| v.as_u64())
                    .map(|v| v as usize)
                    .collect()
            })
            .unwrap_or_default();

        let identity = {
            let mut m = [[0f32; 4]; 4];
            for (k, row) in m.iter_mut().enumerate() {
                row[k] = 1.0;
            }
            m
        };
        let mut found: Vec<(String, usize, [[f32; 4]; 4])> = Vec::new();
        let mut stack: Vec<(usize, [[f32; 4]; 4])> = roots.iter().map(|&r| (r, identity)).collect();
        while let Some((ni, parent)) = stack.pop() {
            let Some(node) = nodes.get(ni) else { continue };
            let world = mat4_mul(&parent, &trs_matrix(node));
            if let Some(mi) = node.get("mesh").and_then(|v| v.as_u64()) {
                let name = node
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                found.push((name, mi as usize, world));
            }
            if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
                for ch in children.iter().filter_map(|v| v.as_u64()) {
                    stack.push((ch as usize, world));
                }
            }
        }

        /// Positions, triangles and UVs of one transformed primitive.
        type Part = (Vec<[f32; 3]>, Vec<[u32; 3]>, Vec<[f32; 2]>);
        /// Positions and triangles of one transformed primitive.
        type Shape = (Vec<[f32; 3]>, Vec<[u32; 3]>);

        let mut provenance = Vec::new();
        let mut head: Option<Part> = None;
        let mut eyes: Option<Shape> = None;
        for (name, mi, world) in &found {
            let prim = &mesh.primitives[*mi];
            let pos: Vec<[f32; 3]> = prim.positions.iter().map(|&p| xform(world, p)).collect();
            let lower = name.to_ascii_lowercase();
            provenance.push(format!(
                "node {name:?} -> mesh {mi}: {} verts, {} tris, transform rows {:?}",
                pos.len(),
                prim.triangles.len(),
                world
            ));
            if lower.contains("eye") {
                eyes = Some((pos, prim.triangles.clone()));
            } else {
                head = Some((pos, prim.triangles.clone(), prim.uvs.clone()));
            }
        }
        let (hp, ht, hu) = head.ok_or(SourceError::MissingPart("head"))?;
        let (ep, et) = eyes.ok_or(SourceError::MissingPart("eye"))?;
        Ok(Self {
            head: hp,
            head_triangles: ht,
            head_uvs: hu,
            eyes: ep,
            eyes_triangles: et,
            provenance,
        })
    }

    /// Left/right eye centroids, split on the sign of x. Returns `(right, left)` using
    /// the donor's convention (negative x is the character's right).
    pub fn eye_centres(&self) -> Option<([f32; 3], [f32; 3])> {
        let mut acc = [[0f64; 3]; 2];
        let mut n = [0usize; 2];
        for p in &self.eyes {
            let s = usize::from(p[0] >= 0.0);
            for k in 0..3 {
                acc[s][k] += p[k] as f64;
            }
            n[s] += 1;
        }
        if n[0] == 0 || n[1] == 0 {
            return None;
        }
        let c = |s: usize| {
            [
                (acc[s][0] / n[s] as f64) as f32,
                (acc[s][1] / n[s] as f64) as f32,
                (acc[s][2] / n[s] as f64) as f32,
            ]
        };
        Some((c(0), c(1)))
    }

    pub fn bounds(&self) -> ([f32; 3], [f32; 3]) {
        let mut mn = [f32::MAX; 3];
        let mut mx = [f32::MIN; 3];
        for p in &self.head {
            for k in 0..3 {
                mn[k] = mn[k].min(p[k]);
                mx[k] = mx[k].max(p[k]);
            }
        }
        (mn, mx)
    }
}

// ============================================================================== fit

/// Tunables for one conservative fit.
#[derive(Debug, Clone, Copy)]
pub struct FitParams {
    /// Hard cap on any vertex's total displacement, metres.
    pub max_displacement_m: f32,
    /// Correspondences farther than this are treated as "no trustworthy source surface".
    pub max_correspondence_m: f32,
    /// Rings over which influence ramps 0 → 1 away from hard-frozen vertices.
    pub falloff_rings: usize,
    pub iterations: usize,
    pub smooth_factor: f32,
    pub smooth_iterations: usize,
    /// Reject a source triangle whose normal disagrees with the donor vertex normal by
    /// more than this cosine, so a query never wraps onto a facing-away surface.
    pub min_normal_dot: f32,
}

impl Default for FitParams {
    fn default() -> Self {
        Self {
            max_displacement_m: 0.008,
            max_correspondence_m: 0.020,
            falloff_rings: 8,
            iterations: 3,
            smooth_factor: 0.35,
            smooth_iterations: 2,
            min_normal_dot: 0.2,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FitReport {
    pub moved: usize,
    pub editable: usize,
    pub no_correspondence: usize,
    pub capped: usize,
    pub max_displacement_m: f32,
    pub mean_displacement_m: f32,
    pub max_boundary_motion_m: f32,
    pub frozen_bit_identical: bool,
    pub new_inverted_triangles: usize,
    pub new_degenerate_triangles: usize,
    pub pre_existing_degenerate: usize,
    pub outliers: Vec<(u32, f32)>,
    /// How many editable vertices sit at full influence (1.0) versus damped by the
    /// falloff ramp. A fit that is mostly damped is bounded by the mask geometry, not by
    /// the reference — which is the difference between "the likeness is limited" and
    /// "the fit is weak".
    pub editable_at_full_influence: usize,
    pub editable_damped: usize,
    pub mean_influence: f32,
}

fn tri_normal(a: [f32; 3], b: [f32; 3], c: [f32; 3]) -> [f32; 3] {
    let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
    let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
    [
        u[1] * v[2] - u[2] * v[1],
        u[2] * v[0] - u[0] * v[2],
        u[0] * v[1] - u[1] * v[0],
    ]
}

fn norm(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

/// Fit the editable donor surface onto `source` (already in donor space).
///
/// Hard-frozen positions are restored bit-for-bit after every step, so the invariant does
/// not depend on the smoothing or projection happening to leave them alone.
pub fn fit_donor(
    mesh: &DonorMesh,
    masks: &DonorMasks,
    source: &[[f32; 3]],
    source_triangles: &[[u32; 3]],
    p: FitParams,
) -> (Vec<[f32; 3]>, FitReport) {
    let original = mesh.positions.clone();
    let mut cur = original.clone();
    let n = original.len();

    // Influence ramp: 0 on hard-frozen vertices, 1 at `falloff_rings` away.
    let mut ring_dist = vec![usize::MAX; n];
    let mut frontier: Vec<u32> = masks.hard_frozen.iter().copied().collect();
    frontier.sort_unstable();
    for &v in &frontier {
        ring_dist[v as usize] = 0;
    }
    let mut cur_ring = frontier;
    for step in 1..=p.falloff_rings {
        let mut next = Vec::new();
        for &v in &cur_ring {
            for &w in &mesh.ring[v as usize] {
                if ring_dist[w as usize] == usize::MAX {
                    ring_dist[w as usize] = step;
                    next.push(w);
                }
            }
        }
        cur_ring = next;
    }
    let influence = |v: usize| -> f32 {
        match ring_dist[v] {
            usize::MAX => 1.0,
            0 => 0.0,
            d if d >= p.falloff_rings => 1.0,
            d => {
                let t = d as f32 / p.falloff_rings as f32;
                t * t * (3.0 - 2.0 * t) // smoothstep
            }
        }
    };

    let mut editable: Vec<u32> = masks.editable.iter().copied().collect();
    editable.sort_unstable();
    let mut no_corr = 0usize;
    let mut capped = 0usize;

    for _ in 0..p.iterations {
        let donor_normals = DonorMesh::new(cur.clone(), mesh.triangles.clone()).vertex_normals();
        let query: Vec<[f32; 3]> = editable.iter().map(|&v| cur[v as usize]).collect();
        // Gate: a source triangle must face roughly the same way as the donor vertex.
        let gate = |qi: usize, ti: usize| -> bool {
            let t = source_triangles[ti];
            let nrm = tri_normal(
                source[t[0] as usize],
                source[t[1] as usize],
                source[t[2] as usize],
            );
            let l = norm(nrm);
            if l < 1e-12 {
                return false;
            }
            let dn = donor_normals[editable[qi] as usize];
            let dot = (nrm[0] * dn[0] + nrm[1] * dn[1] + nrm[2] * dn[2]) / l;
            dot >= p.min_normal_dot
        };
        let Ok((hits, _rescued)) =
            crate::transfer::closest_points_gated(&query, source, source_triangles, &gate)
        else {
            break;
        };

        no_corr = 0;
        for (qi, &v) in editable.iter().enumerate() {
            let hit = hits[qi];
            let dist_m = (hit.distance_mm / 1000.0) as f32;
            if dist_m > p.max_correspondence_m {
                no_corr += 1;
                continue;
            }
            let t = hit.tri;
            let mut target = [0f32; 3];
            for k in 0..3 {
                target[k] = hit.bary[0] * source[t[0] as usize][k]
                    + hit.bary[1] * source[t[1] as usize][k]
                    + hit.bary[2] * source[t[2] as usize][k];
            }
            let w = influence(v as usize);
            let cp = cur[v as usize];
            let mut delta = [
                (target[0] - cp[0]) * w,
                (target[1] - cp[1]) * w,
                (target[2] - cp[2]) * w,
            ];
            // Cap against the *original* position so iterations cannot drift past it.
            let op = original[v as usize];
            let total = [
                cp[0] + delta[0] - op[0],
                cp[1] + delta[1] - op[1],
                cp[2] + delta[2] - op[2],
            ];
            let tl = norm(total);
            if tl > p.max_displacement_m {
                let s = p.max_displacement_m / tl;
                delta = [
                    op[0] + total[0] * s - cp[0],
                    op[1] + total[1] * s - cp[1],
                    op[2] + total[2] * s - cp[2],
                ];
                capped += 1;
            }
            for k in 0..3 {
                cur[v as usize][k] += delta[k];
            }
        }
        // Restore every hard-frozen position exactly, then smooth only the editable set
        // and restore again.
        for &v in &masks.hard_frozen {
            cur[v as usize] = original[v as usize];
        }
        for _ in 0..p.smooth_iterations {
            let snapshot = cur.clone();
            for &v in &editable {
                let r = &mesh.ring[v as usize];
                if r.is_empty() {
                    continue;
                }
                let mut avg = [0f32; 3];
                for &j in r {
                    for k in 0..3 {
                        avg[k] += snapshot[j as usize][k];
                    }
                }
                // The falloff has to govern *all* displacement, not just the projection.
                // Un-gated Laplacian smoothing was pulling vertices that sit one ring from
                // a hard-frozen vertex toward their far-moved neighbours, which put a ~7 mm
                // step across a ~5 mm edge and creased the surface at every mask boundary.
                let sf = p.smooth_factor * influence(v as usize);
                for k in 0..3 {
                    avg[k] /= r.len() as f32;
                    cur[v as usize][k] = snapshot[v as usize][k] * (1.0 - sf) + avg[k] * sf;
                }
            }
            for &v in &masks.hard_frozen {
                cur[v as usize] = original[v as usize];
            }
        }
    }

    // ---------------------------------------------------------------- measurements
    let frozen_ok = masks.hard_frozen.iter().all(|&v| {
        (0..3).all(|k| cur[v as usize][k].to_bits() == original[v as usize][k].to_bits())
    });
    let mut moved = 0usize;
    let mut sum = 0f64;
    let mut maxd = 0f32;
    let mut outliers: Vec<(u32, f32)> = Vec::new();
    for v in 0..n as u32 {
        let d = norm([
            cur[v as usize][0] - original[v as usize][0],
            cur[v as usize][1] - original[v as usize][1],
            cur[v as usize][2] - original[v as usize][2],
        ]);
        if d > 0.0 {
            moved += 1;
            sum += d as f64;
            maxd = maxd.max(d);
            outliers.push((v, d));
        }
    }
    outliers.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
    outliers.truncate(10);

    // Motion right at the protected boundary should be ~0 by construction; measure it.
    let mut max_boundary = 0f32;
    for &v in &masks.hard_frozen {
        for &w in &mesh.ring[v as usize] {
            if masks.editable.contains(&w) {
                max_boundary = max_boundary.max(norm([
                    cur[w as usize][0] - original[w as usize][0],
                    cur[w as usize][1] - original[w as usize][1],
                    cur[w as usize][2] - original[w as usize][2],
                ]));
            }
        }
    }

    let (mut full, mut damped, mut inf_sum) = (0usize, 0usize, 0f64);
    for &v in &editable {
        let w = influence(v as usize);
        inf_sum += w as f64;
        if w >= 0.999 {
            full += 1;
        } else {
            damped += 1;
        }
    }

    let (mut inverted, mut degenerate, mut pre_degenerate) = (0usize, 0usize, 0usize);
    for t in &mesh.triangles {
        let before = tri_normal(
            original[t[0] as usize],
            original[t[1] as usize],
            original[t[2] as usize],
        );
        let after = tri_normal(cur[t[0] as usize], cur[t[1] as usize], cur[t[2] as usize]);
        let (lb, la) = (norm(before), norm(after));
        if lb <= 1e-12 {
            pre_degenerate += 1;
            continue;
        }
        if la <= 1e-12 {
            degenerate += 1;
            continue;
        }
        let dot = (before[0] * after[0] + before[1] * after[1] + before[2] * after[2]) / (lb * la);
        if dot < 0.0 {
            inverted += 1;
        }
    }

    (
        cur,
        FitReport {
            moved,
            editable: editable.len(),
            no_correspondence: no_corr,
            capped,
            max_displacement_m: maxd,
            mean_displacement_m: if moved > 0 {
                (sum / moved as f64) as f32
            } else {
                0.0
            },
            max_boundary_motion_m: max_boundary,
            frozen_bit_identical: frozen_ok,
            new_inverted_triangles: inverted,
            new_degenerate_triangles: degenerate,
            pre_existing_degenerate: pre_degenerate,
            outliers,
            editable_at_full_influence: full,
            editable_damped: damped,
            mean_influence: if editable.is_empty() {
                0.0
            } else {
                (inf_sum / editable.len() as f64) as f32
            },
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn umeyama_recovers_a_known_similarity() {
        let pts = [
            [0.0f32, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.3, -0.7, 0.2],
        ];
        // 90 degrees about Y, scale 2, translate.
        let known = Similarity {
            scale: 2.0,
            rot: [[0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0]],
            translation: [0.5, -0.25, 3.0],
        };
        let to: Vec<[f32; 3]> = pts.iter().map(|&p| known.apply(p)).collect();
        let got = umeyama(&pts, &to).expect("solvable");
        assert!((got.scale - 2.0).abs() < 1e-4, "scale {}", got.scale);
        for (a, b) in pts.iter().zip(&to) {
            let g = got.apply(*a);
            for k in 0..3 {
                assert!((g[k] - b[k]).abs() < 1e-3, "{g:?} vs {b:?}");
            }
        }
    }

    #[test]
    fn umeyama_refuses_degenerate_input() {
        assert!(umeyama(&[[0.0; 3]; 2], &[[0.0; 3]; 2]).is_none());
        assert!(umeyama(&[[0.0; 3]; 3], &[[0.0; 3]; 4]).is_none());
    }

    /// A quad in front of the query must occlude it; one behind must not.
    #[test]
    fn occlusion_test_sees_only_what_is_in_front() {
        let positions = vec![
            [0.0, 0.0, 0.0],   // query
            [-1.0, -1.0, 1.0], // front quad
            [1.0, -1.0, 1.0],
            [0.0, 1.0, 1.0],
            [-1.0, -1.0, -1.0], // rear quad
            [1.0, -1.0, -1.0],
            [0.0, 1.0, -1.0],
        ];
        let front = DonorMesh::new(positions.clone(), vec![[1, 2, 3]]);
        assert!(front.occluded_along_z(0));
        let rear = DonorMesh::new(positions, vec![[4, 5, 6]]);
        assert!(!rear.occluded_along_z(0));
    }

    #[test]
    fn boundary_components_come_out_largest_first() {
        // Two disjoint triangles: each is its own 3-vertex boundary loop.
        let positions = vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [5.0, 0.0, 0.0],
            [6.0, 0.0, 0.0],
            [5.0, 1.0, 0.0],
        ];
        let m = DonorMesh::new(positions, vec![[0, 1, 2], [3, 4, 5]]);
        assert_eq!(m.boundary_components.len(), 2);
        assert!(m.boundary_components[0].len() >= m.boundary_components[1].len());
    }

    #[test]
    fn masks_validate_reports_overlap_and_range() {
        let mut m = DonorMasks {
            donor_sha256: "x".into(),
            submesh: "geo_headShape".into(),
            vertex_count: 4,
            sets: vec![MaskSet {
                name: "bad".into(),
                ids: vec![9],
                method: "test".into(),
            }],
            hard_frozen: [1u32].into_iter().collect(),
            editable: [1u32].into_iter().collect(),
            seam_exact_groups: 0,
            seam_rounded_groups: 0,
            seam_max_pair_distance: 0.0,
            notes: vec![],
        };
        let bad = m.validate();
        assert_eq!(bad.len(), 2, "{bad:?}");
        m.sets[0].ids = vec![0];
        m.editable = [2u32].into_iter().collect();
        assert!(m.validate().is_empty());
    }
}
