//! Pack a converted head into an RW (`\x89RW4xb2`) donor container.
//!
//! First consumer of `tdb_rx2::pack`. The donor's non-head submeshes (eyes, mouthbag)
//! stay verbatim — the eye rule: donor eyeballs animate against the donor eyelid pocket
//! and UV into the shared eye raster, so they are never replaced. The `head_` slot gets
//! the converted geometry:
//!
//! * UVs are transitioned to STORED convention through the typed
//!   [`UvConvention`][crate::ir::UvConvention] (exactly one flip for GL-sourced data);
//! * normals are recomputed area-weighted smooth over the new surface (shipping empty
//!   normals bands the in-game shading);
//! * skin influences arrive in palette space and are encoded as shader-register
//!   BLENDINDICES (`palette × 3`);
//! * the haircap can be collapsed to a degenerate micro-triangle under the scalp for
//!   heads whose hair the donor's cap doesn't fit (the donor cap otherwise remains).

use tdb_rx2::container::{self, Container};
use tdb_rx2::pack::{self, PackError, SubstituteReport};
use tdb_rx2::playermesh::{self, PlayerMeshError};

use crate::ir::{IrSubmesh, SubmeshRole, UvConvention};

/// Legacy Head palette slot — the anchor for degenerate/collapsed geometry.
const HEAD_PALETTE: u16 = 8;

#[derive(Debug, thiserror::Error)]
pub enum PackRwError {
    #[error("donor container: {0}")]
    Container(#[from] container::ContainerError),
    #[error("pack: {0}")]
    Pack(#[from] PackError),
    #[error("donor has no {0:?}-role submesh (handles: {1})")]
    MissingRole(SubmeshRole, String),
    #[error("submesh has no skin influences — Path A pack requires remapped weights")]
    NoInfluences,
    #[error(
        "vertex {vertex} carries {count} influences after remap (target budget is 4) — \
         remap must truncate before pack"
    )]
    TooManyInfluences { vertex: usize, count: usize },
    #[error("packed container fails strict re-parse: {0}")]
    Reparse(#[from] PlayerMeshError),
    #[error(
        "eye alignment verification failed: {submesh} centroid is {err_mm:.2} mm off its \
         target after the position patch (parse-order mismatch?)"
    )]
    AlignmentDrift { submesh: String, err_mm: f32 },
    #[error("haircap conform: {0}")]
    Transfer(#[from] crate::transfer::TransferError),
}

/// Displacement beyond which a shell conform stops trusting its correspondence and
/// clamps. Hair geometry that stands well off the scalp has no meaningful closest point
/// on the skull, and an unbounded move would fling it.
pub const SHELL_CONFORM_LIMIT_MM: f32 = 30.0;

/// Projection passes. One pass overshoots: displacing a vertex changes which triangle is
/// closest to it, so the clearance it actually lands on is not the one solved for. Four
/// passes converge (the first run left 12% of the cap still buried; iterating clears it).
const CONFORM_ITERATIONS: usize = 4;

/// Laplacian passes over the shell's own 1-ring, applied to the displacement field before
/// it moves anything. Closest-point correspondence is piecewise — it jumps between distant
/// parts of the skull around the ears and the lash line — and those jumps are noise, not
/// shape. Left in, they crumple the cap: the unsmoothed field turned its normals by 9.9°
/// on average, and `patch_player_mesh` writes positions only, so that error would ship.
const CONFORM_SMOOTHING_PASSES: usize = 12;

/// Clearance floor, metres. Vanilla `player_67`'s cap effectively never penetrates its
/// skull (1.5% of vertices, worst case 0.04 mm — numerical noise), so the conform refuses
/// to seat anything below this even where the donor itself did.
const MIN_SHELL_CLEARANCE_M: f32 = 0.0003;

/// The two skulls a shell conform sits between: the one the donor's rigid shell was
/// authored against, and the one that just replaced it.
#[derive(Debug, Clone, Copy)]
pub struct ShellConform<'a> {
    pub donor_head: &'a [[f32; 3]],
    pub donor_triangles: &'a [[u32; 3]],
    pub new_head: &'a [[f32; 3]],
    pub new_triangles: &'a [[u32; 3]],
}

#[derive(Debug, Clone, PartialEq)]
pub struct HaircapConformReport {
    pub vertices: usize,
    /// Displacement applied, millimetres.
    pub mean_mm: f32,
    pub p95_mm: f32,
    pub max_mm: f32,
    pub clamped: usize,
    /// (median clearance mm, fraction of vertices inside the skull), before and after.
    /// The donor's own figures are the target — vanilla shells never penetrate.
    pub clearance_before: (f32, f32),
    pub clearance_after: (f32, f32),
    /// Angle by which the conform turns the shell's own smooth normals — i.e. how much the
    /// cap actually changed shape. The patch rewrites NORMAL and TANGENT to match, so this
    /// is a measure of the deformation, not of leftover error.
    pub normal_drift_mean_deg: f32,
    pub normal_drift_p95_deg: f32,
}

fn pct(mut v: Vec<f32>, q: f32) -> f32 {
    if v.is_empty() {
        return 0.0;
    }
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    v[((v.len() as f32 - 1.0) * q).round() as usize]
}

fn unit(v: [f32; 3]) -> [f32; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-12);
    [v[0] / l, v[1] / l, v[2] / l]
}

/// Where a query point sits relative to a surface: its footpoint, the interpolated normal
/// there, and the signed distance along that normal (positive = outside).
#[derive(Debug, Clone, Copy, Default)]
struct Foot {
    point: [f32; 3],
    normal: [f32; 3],
    distance: f32,
}

fn feet(
    query: &[[f32; 3]],
    surface: &[[f32; 3]],
    triangles: &[[u32; 3]],
    normals: &[[f32; 3]],
) -> Result<Vec<Foot>, crate::transfer::TransferError> {
    let hits = crate::transfer::closest_points(query, surface, triangles)?;
    Ok(hits
        .iter()
        .zip(query)
        .map(|(h, q)| {
            let mut point = [0f32; 3];
            let mut n = [0f32; 3];
            for k in 0..3 {
                let v = h.tri[k] as usize;
                for c in 0..3 {
                    point[c] += h.bary[k] * surface[v][c];
                    n[c] += h.bary[k] * normals[v][c];
                }
            }
            let normal = unit(n);
            Foot {
                point,
                normal,
                distance: (0..3).map(|c| (q[c] - point[c]) * normal[c]).sum::<f32>(),
            }
        })
        .collect())
}

fn clearance_summary(c: &[Foot]) -> (f32, f32) {
    if c.is_empty() {
        return (0.0, 0.0);
    }
    let inside = c.iter().filter(|f| f.distance < 0.0).count();
    (
        pct(c.iter().map(|f| f.distance * 1000.0).collect(), 0.5),
        inside as f32 / c.len() as f32,
    )
}

/// Weld a mesh by exact position and build the 1-ring over the representatives.
///
/// Returns `(representative per vertex, neighbours per representative)`. Exact bits, not an
/// epsilon — the same rule [`smooth_normals`] uses, and for the same reason.
type WeldRing = (Vec<u32>, std::collections::HashMap<u32, Vec<u32>>);
fn weld_ring(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> WeldRing {
    use std::collections::{HashMap, HashSet};
    let mut first: HashMap<[u32; 3], u32> = HashMap::new();
    let mut rep = vec![0u32; positions.len()];
    for (i, p) in positions.iter().enumerate() {
        let k = [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
        rep[i] = *first.entry(k).or_insert(i as u32);
    }
    let mut sets: HashMap<u32, HashSet<u32>> = HashMap::new();
    for t in triangles {
        for k in 0..3 {
            let (a, b) = (
                rep[t[k] as usize % positions.len()],
                rep[t[(k + 1) % 3] as usize % positions.len()],
            );
            if a != b {
                sets.entry(a).or_default().insert(b);
                sets.entry(b).or_default().insert(a);
            }
        }
    }
    (
        rep,
        sets.into_iter()
            .map(|(k, v)| (k, v.into_iter().collect()))
            .collect(),
    )
}

/// Laplacian-smooth a per-vertex vector field over a welded 1-ring, in place.
///
/// Smoothing happens on representatives and is then broadcast back to every duplicate, so
/// coincident vertices keep identical displacement and seams stay closed.
fn smooth_field(
    field: &mut [[f32; 3]],
    rep: &[u32],
    ring: &std::collections::HashMap<u32, Vec<u32>>,
    passes: usize,
) {
    if field.is_empty() {
        return;
    }
    let mut cur: std::collections::HashMap<u32, [f32; 3]> =
        ring.keys().map(|&v| (v, field[v as usize])).collect();
    for _ in 0..passes {
        let src = cur.clone();
        for (&v, nb) in ring {
            if nb.is_empty() {
                continue;
            }
            let mut acc = [0f32; 3];
            for &j in nb {
                let s = src.get(&j).copied().unwrap_or_default();
                for c in 0..3 {
                    acc[c] += s[c];
                }
            }
            let n = nb.len() as f32;
            let s = src[&v];
            // Half-weight towards the ring mean: enough to kill correspondence jumps
            // over a dozen passes without collapsing the field's overall magnitude.
            for c in 0..3 {
                cur.get_mut(&v).unwrap()[c] = 0.5 * s[c] + 0.5 * (acc[c] / n);
            }
        }
    }
    for i in 0..field.len() {
        if let Some(v) = cur.get(&rep[i]) {
            field[i] = *v;
        }
    }
}

/// Re-seat a rigid donor shell onto a replaced skull, preserving its original clearance.
///
/// The donor `haircap_` is authored to hug the donor skull — vanilla `player_67` never
/// penetrates it (1.5% of vertices, worst case 0.04 mm, i.e. numerical noise). Dropping a
/// differently-shaped skull underneath leaves the shell fitting nothing: on the CC5
/// reference it floated 3x too far out on average while punching up to 7 mm *through* the
/// scalp across 20% of its vertices. Anything riding the shell — the eyelashes especially
/// — stays at the donor's anatomy while the face moves out from under it.
///
/// So each shell vertex is moved along the *new* skull's normal until its clearance to the
/// new surface equals what it had to the old one. Clearance is the invariant the shell was
/// authored with; preserving it re-seats the lashes on the eyes for free, because the
/// closest point on the skull tracks the eyelid.
///
/// Positions only — `patch_player_mesh` cannot rewrite normals, so the residual normal
/// drift is measured into the report instead of being left silent.
pub fn conform_shell(
    shell: &mut [[f32; 3]],
    shell_triangles: &[[u32; 3]],
    ctx: &ShellConform,
) -> Result<HaircapConformReport, crate::transfer::TransferError> {
    let donor_n = smooth_normals(ctx.donor_head, ctx.donor_triangles);
    let new_n = smooth_normals(ctx.new_head, ctx.new_triangles);

    let before = shell.to_vec();
    // Target clearance: what the shell had over the skull it was authored against.
    let old = feet(shell, ctx.donor_head, ctx.donor_triangles, &donor_n)?;
    let first = feet(shell, ctx.new_head, ctx.new_triangles, &new_n)?;

    // Weld the shell so duplicated seam vertices share one displacement — otherwise the
    // smoothing pass moves the two copies apart and opens a crack.
    let (rep, ring) = weld_ring(&before, shell_triangles);

    // Pass 1 — warp. Move each shell vertex by however far its own footpoint moved when
    // the skull was replaced. This carries tangential shape change, which a
    // displacement-along-the-normal cannot express: the clearance difference is a scalar,
    // so it can only push the cap in and out, never slide it around the ears or down the
    // lash line. Warping first leaves only a small residual for the normal pass to clean
    // up, and being a difference of two smooth footpoint fields it is smooth by
    // construction rather than by filtering.
    let mut warp: Vec<[f32; 3]> = (0..shell.len())
        .map(|i| {
            let (a, b) = (first[i].point, old[i].point);
            [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
        })
        .collect();
    smooth_field(&mut warp, &rep, &ring, CONFORM_SMOOTHING_PASSES / 2);
    for (p, d) in shell.iter_mut().zip(&warp) {
        for c in 0..3 {
            p[c] += d[c];
        }
    }

    // Pass 2 — restore clearance, with the smoothing relaxed each iteration so the bulk
    // motion is established smoothly and individual vertices can still reach their target
    // (holding the filter strong to the end is what left 15% of the cap buried).
    for it in 0..CONFORM_ITERATIONS {
        let cur = feet(shell, ctx.new_head, ctx.new_triangles, &new_n)?;
        let mut disp: Vec<[f32; 3]> = (0..shell.len())
            .map(|i| {
                // Never re-seat a vertex *into* the skull: vanilla caps do not penetrate,
                // so the donor's own rare negatives are floored out rather than copied.
                let target = old[i].distance.max(MIN_SHELL_CLEARANCE_M);
                let d = target - cur[i].distance;
                let n = cur[i].normal;
                [n[0] * d, n[1] * d, n[2] * d]
            })
            .collect();
        let passes = (CONFORM_SMOOTHING_PASSES >> (it + 1)).max(1);
        smooth_field(&mut disp, &rep, &ring, passes);
        for (p, d) in shell.iter_mut().zip(&disp) {
            for c in 0..3 {
                p[c] += d[c];
            }
        }
    }

    // Clamp total travel against the original position, not per pass.
    let mut moved = Vec::with_capacity(shell.len());
    let mut clamped = 0usize;
    for (i, p) in shell.iter_mut().enumerate() {
        let d = [
            p[0] - before[i][0],
            p[1] - before[i][1],
            p[2] - before[i][2],
        ];
        let mm = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() * 1000.0;
        if mm > SHELL_CONFORM_LIMIT_MM {
            let k = SHELL_CONFORM_LIMIT_MM / mm;
            for c in 0..3 {
                p[c] = before[i][c] + d[c] * k;
            }
            clamped += 1;
            moved.push(SHELL_CONFORM_LIMIT_MM);
        } else {
            moved.push(mm);
        }
    }

    let cur = first;
    let after = feet(shell, ctx.new_head, ctx.new_triangles, &new_n)?;

    // Normal drift the position-only patch leaves behind.
    let nb = smooth_normals(&before, shell_triangles);
    let na = smooth_normals(shell, shell_triangles);
    let drift: Vec<f32> = nb
        .iter()
        .zip(&na)
        .map(|(a, b)| {
            let d = (a[0] * b[0] + a[1] * b[1] + a[2] * b[2]).clamp(-1.0, 1.0);
            d.acos().to_degrees()
        })
        .collect();

    Ok(HaircapConformReport {
        vertices: shell.len(),
        mean_mm: if moved.is_empty() {
            0.0
        } else {
            moved.iter().sum::<f32>() / moved.len() as f32
        },
        p95_mm: pct(moved.clone(), 0.95),
        max_mm: pct(moved, 1.0),
        clamped,
        clearance_before: clearance_summary(&cur),
        clearance_after: clearance_summary(&after),
        normal_drift_mean_deg: if drift.is_empty() {
            0.0
        } else {
            drift.iter().sum::<f32>() / drift.len() as f32
        },
        normal_drift_p95_deg: pct(drift, 0.95),
    })
}

/// Where — and how big — the source asset wants the donor's rigid companion submeshes.
/// Measured, never hand-tuned — see [`crate::placement`].
///
/// The source's own eye/mouth geometry is a *measuring stick*: it never reaches the
/// container (the donor's eyeballs UV into the shared eye raster and animate against the
/// donor eyelid pocket; the mouthbag is the target's mouth-interior convention). It only
/// says where those donor submeshes belong and how large they should be.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct SubmeshAlignment {
    /// Source (left = +x, right = −x) eyeball centers; donor `eyel_`/`eyer_` translate
    /// onto them (gaze rides the per-draw register slot, UVs into the shared eye raster
    /// are untouched).
    pub eye_centers: Option<([f32; 3], [f32; 3])>,
    /// Uniform scale for the donor eyeballs, applied about each eye's own centroid before
    /// it is moved onto its target center. `None` keeps donor size.
    pub eye_scale: Option<f32>,
    /// Mouth-opening centroid delta (source − donor); donor `mouthbag_` translates by
    /// it so the bag sits behind the source's lips.
    pub mouth_delta: Option<[f32; 3]>,
    /// Uniform scale for the donor mouthbag, applied about its own centroid before the
    /// delta is added. `None` keeps donor size.
    pub mouth_scale: Option<f32>,
}

/// Scale a point set about its own centroid, in place.
fn scale_about_centroid(set: &mut [[f32; 3]], scale: f32) {
    if set.is_empty() || (scale - 1.0).abs() < 1e-6 {
        return;
    }
    let mut c = [0f64; 3];
    for p in set.iter() {
        for k in 0..3 {
            c[k] += f64::from(p[k]);
        }
    }
    let n = set.len() as f64;
    let centroid = [(c[0] / n) as f32, (c[1] / n) as f32, (c[2] / n) as f32];
    for p in set.iter_mut() {
        for k in 0..3 {
            p[k] = centroid[k] + (p[k] - centroid[k]) * scale;
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PackRwOptions {
    /// Collapse the donor `haircap_` to a hidden degenerate triangle.
    pub collapse_haircap: bool,
    /// Re-seat the donor `haircap_` onto the converted skull, preserving the clearance it
    /// had over the donor's. Ignored when the cap is collapsed. See [`conform_shell`].
    pub conform_haircap: bool,
    pub alignment: SubmeshAlignment,
}

#[derive(Debug)]
pub struct PackRwReport {
    pub head: SubstituteReport,
    pub haircap_collapsed: bool,
    /// Vertex/triangle counts of the packed head.
    pub vertices: usize,
    pub triangles: usize,
    /// (submesh, shift in mm) for each aligned eye submesh.
    pub eye_shifts_mm: Vec<(String, f32)>,
    pub mouthbag_shift_mm: Option<f32>,
    /// Uniform scale actually applied to the donor eyeballs / mouthbag (1.0 = untouched).
    pub eye_scale: f32,
    pub mouthbag_scale: f32,
    /// Present when the donor haircap was re-seated onto the converted skull.
    pub haircap_conform: Option<HaircapConformReport>,
}

/// The pack API's fixed-4 skinning arrays: per-vertex joint indices and their weights.
type JointWeightArrays = (Vec<[u16; 4]>, Vec<[f32; 4]>);

/// A decoded mesh surface: vertex positions and the triangle indices over them.
type Surface = (Vec<[f32; 3]>, Vec<[u32; 3]>);

/// Per-vertex palette influences → the pack API's fixed-4 arrays.
/// `influences[i]` = (palette slot, weight), at most 4 entries (remap's budget).
fn influence_arrays(influences: &[Vec<(u32, f32)>]) -> Result<JointWeightArrays, PackRwError> {
    let mut joints = Vec::with_capacity(influences.len());
    let mut weights = Vec::with_capacity(influences.len());
    for (vi, list) in influences.iter().enumerate() {
        if list.len() > 4 {
            return Err(PackRwError::TooManyInfluences {
                vertex: vi,
                count: list.len(),
            });
        }
        let mut j = [HEAD_PALETTE * 3; 4];
        let mut w = [0f32; 4];
        for (slot, (palette, weight)) in list.iter().enumerate() {
            j[slot] = (*palette as u16) * 3;
            w[slot] = *weight;
        }
        if list.is_empty() {
            // Never ship an unweighted vertex — rigid to Head.
            w[0] = 1.0;
        }
        joints.push(j);
        weights.push(w);
    }
    Ok((joints, weights))
}

/// Area-weighted smooth normals over the given surface (mean-dot 0.98 vs donor
/// authored normals in the prior effort — flat normals band the shading).
///
/// **Welded across coincident positions.** A UV seam is stored as duplicated vertices
/// sharing one position, because RW keeps UVs per-vertex. Accumulating per vertex *index*
/// gives each copy only its own side's triangles, so every seam becomes a hard shading
/// break. Measured on the CC5 head: 157 split groups / 320 vertices whose two copies
/// disagreed by 33.9° on average and up to 135°, drawing visible lines down the neck, round
/// the ears and across the temples in-game. EA's own donor normals are welded (7° residual,
/// which is just their smoothing model), so welding is matching the target convention, not
/// inventing one.
///
/// Welding is by exact position equality (bit pattern, via the ordered key below) — copies
/// created by a UV split are *exactly* coincident, so no epsilon is needed and none is used:
/// a tolerance here would risk merging genuinely distinct surfaces that happen to touch,
/// such as the lips at a closed mouth.
pub fn smooth_normals(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut acc = accumulate_face_normals(positions, triangles);

    // Weld: sum each coincident group, then hand every member the same vector.
    let mut groups: std::collections::HashMap<[u32; 3], Vec<u32>> =
        std::collections::HashMap::new();
    for (i, p) in positions.iter().enumerate() {
        groups
            .entry([p[0].to_bits(), p[1].to_bits(), p[2].to_bits()])
            .or_default()
            .push(i as u32);
    }
    for members in groups.values() {
        if members.len() < 2 {
            continue;
        }
        let mut sum = [0f32; 3];
        for &m in members {
            for k in 0..3 {
                sum[k] += acc[m as usize][k];
            }
        }
        for &m in members {
            acc[m as usize] = sum;
        }
    }

    for n in &mut acc {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len > 1e-12 {
            for axis in n.iter_mut() {
                *axis /= len;
            }
        } else {
            *n = [0.0, 0.0, 1.0];
        }
    }
    acc
}

/// Unnormalized area-weighted face-normal accumulation, per vertex index.
fn accumulate_face_normals(positions: &[[f32; 3]], triangles: &[[u32; 3]]) -> Vec<[f32; 3]> {
    let mut acc = vec![[0f32; 3]; positions.len()];
    for tri in triangles {
        let [a, b, c] = [
            positions[tri[0] as usize],
            positions[tri[1] as usize],
            positions[tri[2] as usize],
        ];
        let u = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
        let v = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
        // Cross product magnitude is 2×area — accumulating unnormalized weights by area.
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in tri {
            for k in 0..3 {
                acc[i as usize][k] += n[k];
            }
        }
    }
    acc
}

fn find_role_handle(c: &Container, role: SubmeshRole) -> Option<String> {
    pack::list_mesh_handles(c)
        .into_iter()
        .find(|name| SubmeshRole::classify(name) == role)
}

/// Pack a remapped head submesh into the donor container.
///
/// `head` must carry palette-space influences in `palette_influences` (the remap
/// output) — the IR's named influences are the *source*-space weights and are not
/// consumed here.
pub fn pack_head_into_donor(
    donor_rx2: &[u8],
    head: &IrSubmesh,
    palette_influences: &[Vec<(u32, f32)>],
    options: PackRwOptions,
) -> Result<(Vec<u8>, PackRwReport), PackRwError> {
    if head.influences.is_none() && palette_influences.is_empty() {
        return Err(PackRwError::NoInfluences);
    }
    // The donor skull, captured before the substitution overwrites it — a shell conform
    // needs the surface the donor haircap was actually authored against.
    let donor_skull: Option<Surface> = if options.conform_haircap && !options.collapse_haircap {
        playermesh::parse_player_mesh_preview(donor_rx2)?
            .submeshes
            .iter()
            .find(|s| SubmeshRole::classify(&s.name) == SubmeshRole::Head)
            .map(|s| {
                (
                    s.vertices.iter().map(|v| v.position).collect(),
                    s.triangles.iter().map(|t| [t[0], t[1], t[2]]).collect(),
                )
            })
    } else {
        None
    };

    let mut container = container::parse(donor_rx2)?;
    let handles = pack::list_mesh_handles(&container).join(", ");
    let head_handle = find_role_handle(&container, SubmeshRole::Head)
        .ok_or_else(|| PackRwError::MissingRole(SubmeshRole::Head, handles.clone()))?;

    // UVs: exactly one flip when the geometry came from glTF/OBJ.
    let mut uvs = head.uvs.clone();
    if head.uv_convention == UvConvention::GlSourced {
        for uv in &mut uvs {
            uv[1] = 1.0 - uv[1];
        }
    }
    let normals = if head.normals.len() == head.positions.len() {
        head.normals.clone()
    } else {
        smooth_normals(&head.positions, &head.triangles)
    };
    let (joints, weights) = influence_arrays(palette_influences)?;

    let head_report = pack::substitute_submesh(
        &mut container,
        &head_handle,
        &head.positions,
        &normals,
        &uvs,
        &joints,
        &weights,
        &head.triangles,
    )?;

    let mut haircap_collapsed = false;
    if options.collapse_haircap {
        if let Some(haircap_handle) = find_role_handle(&container, SubmeshRole::Haircap) {
            // A degenerate micro-triangle inside the skull, rigid to Head: invisible,
            // keeps the submesh/material machinery intact.
            let p = [[0.0f32, 1.78, 0.0], [0.0, 1.7801, 0.0], [0.0001, 1.78, 0.0]];
            let n = [[0.0f32, 0.0, 1.0]; 3];
            let uv = [[0.5f32, 0.5]; 3];
            let j = [[HEAD_PALETTE * 3; 4]; 3];
            let w = [[1.0f32, 0.0, 0.0, 0.0]; 3];
            pack::substitute_submesh(
                &mut container,
                &haircap_handle,
                &p,
                &n,
                &uv,
                &j,
                &w,
                &[[0, 1, 2]],
            )?;
            haircap_collapsed = true;
        }
    }

    let mut bytes = container.encode();

    // Align the donor's rigid companion submeshes to the source's measured placement
    // (in-place position patch — geometry only, weights/UVs/palette untouched).
    let conform = donor_skull
        .as_ref()
        .map(|(donor_head, donor_triangles)| ShellConform {
            donor_head,
            donor_triangles,
            new_head: &head.positions,
            new_triangles: &head.triangles,
        });
    let (eye_shifts_mm, mouthbag_shift_mm, haircap_conform) =
        apply_alignment(&mut bytes, &options.alignment, conform.as_ref())?;

    // Gate: the packed container must strict-parse (single coherent skin, sane
    // descriptors) before it is allowed anywhere near an install plan.
    playermesh::parse_player_mesh(&bytes)?;

    Ok((
        bytes,
        PackRwReport {
            head: head_report,
            haircap_collapsed,
            vertices: head.positions.len(),
            triangles: head.triangles.len(),
            eye_shifts_mm,
            mouthbag_shift_mm,
            eye_scale: options.alignment.eye_scale.unwrap_or(1.0),
            mouthbag_scale: options.alignment.mouth_scale.unwrap_or(1.0),
            haircap_conform,
        },
    ))
}

/// Translate `eyel_`/`eyer_` onto the source eye centers and `mouthbag_` by the mouth
/// delta, via the same-count position patch. Returns the applied shift magnitudes (mm).
#[allow(clippy::type_complexity)]
fn apply_alignment(
    bytes: &mut [u8],
    alignment: &SubmeshAlignment,
    conform: Option<&ShellConform>,
) -> Result<
    (
        Vec<(String, f32)>,
        Option<f32>,
        Option<HaircapConformReport>,
    ),
    PackRwError,
> {
    if alignment.eye_centers.is_none() && alignment.mouth_delta.is_none() && conform.is_none() {
        return Ok((Vec::new(), None, None));
    }
    let parsed = playermesh::parse_player_mesh(bytes)?;
    let mut position_sets: Vec<Vec<[f32; 3]>> = parsed
        .submeshes
        .iter()
        .map(|s| s.vertices.iter().map(|v| v.position).collect())
        .collect();

    let mut eye_shifts = Vec::new();
    let mut mouth_shift = None;
    let mut haircap_conform = None;
    let mut shading: Vec<Option<playermesh::SubmeshShading>> = vec![None; parsed.submeshes.len()];
    let mut changed = false;
    for (i, sm) in parsed.submeshes.iter().enumerate() {
        match SubmeshRole::classify(&sm.name) {
            SubmeshRole::Eyes => {
                let Some((left, right)) = alignment.eye_centers else {
                    continue;
                };
                let set = &mut position_sets[i];
                if set.is_empty() {
                    continue;
                }
                // Resize about the donor eye's own centroid first, then move that
                // centroid onto the source's — so the scale never drags the eye off the
                // measured center.
                if let Some(s) = alignment.eye_scale {
                    scale_about_centroid(set, s);
                }
                let mut c = [0f64; 3];
                for p in set.iter() {
                    for k in 0..3 {
                        c[k] += f64::from(p[k]);
                    }
                }
                let n = set.len() as f64;
                let centroid = [(c[0] / n) as f32, (c[1] / n) as f32, (c[2] / n) as f32];
                // Side by x sign, not by name — names have lied before.
                let target = if centroid[0] >= 0.0 { left } else { right };
                let delta = [
                    target[0] - centroid[0],
                    target[1] - centroid[1],
                    target[2] - centroid[2],
                ];
                for p in set.iter_mut() {
                    for k in 0..3 {
                        p[k] += delta[k];
                    }
                }
                let mm = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt()
                    * 1000.0;
                eye_shifts.push((sm.name.clone(), mm));
                changed = true;
            }
            SubmeshRole::Mouthbag => {
                let Some(delta) = alignment.mouth_delta else {
                    continue;
                };
                if let Some(s) = alignment.mouth_scale {
                    scale_about_centroid(&mut position_sets[i], s);
                }
                for p in position_sets[i].iter_mut() {
                    for k in 0..3 {
                        p[k] += delta[k];
                    }
                }
                mouth_shift = Some(
                    (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt()
                        * 1000.0,
                );
                changed = true;
            }
            SubmeshRole::Haircap => {
                let Some(ctx) = conform else { continue };
                if position_sets[i].is_empty() {
                    continue;
                }
                let tris: Vec<[u32; 3]> = sm.triangles.iter().map(|t| [t[0], t[1], t[2]]).collect();
                haircap_conform = Some(conform_shell(&mut position_sets[i], &tris, ctx)?);
                // A conform deforms the cap, so its donor-authored normals and tangents no
                // longer describe it (measured: 8.8 deg mean drift on the CC5 reference).
                // Re-substituting the submesh would fix them but is not an option here —
                // the stride-52 roster haircap carries three genuinely distinct UV
                // channels (verified on player_67: UV2 matches UV0 on 0/758 vertices), and
                // the stride-52 packer duplicates UV0 into all three. So shading is
                // patched in place alongside the positions instead.
                let uvs: Vec<[f32; 2]> = sm.vertices.iter().map(|v| v.uv).collect();
                let normals = smooth_normals(&position_sets[i], &tris);
                let tangents =
                    tdb_rx2::pack::derive_tangents(&position_sets[i], &normals, &uvs, &tris);
                shading[i] = Some(playermesh::SubmeshShading { normals, tangents });
                changed = true;
            }
            _ => {}
        }
    }
    if changed {
        playermesh::patch_player_mesh(bytes, &position_sets, Some(&shading), None)?;
        // Close the loop: the patched eyes must actually sit on the requested centers
        // (guards against a parse-order/position-set mismatch scrambling geometry —
        // fp16 storage allows ~1 mm of quantization slack).
        if let Some((left, right)) = alignment.eye_centers {
            let reparsed = playermesh::parse_player_mesh(bytes)?;
            for sm in &reparsed.submeshes {
                if SubmeshRole::classify(&sm.name) != SubmeshRole::Eyes || sm.vertices.is_empty() {
                    continue;
                }
                let mut c = [0f64; 3];
                for v in &sm.vertices {
                    for (k, axis) in c.iter_mut().enumerate() {
                        *axis += f64::from(v.position[k]);
                    }
                }
                let n = sm.vertices.len() as f64;
                let centroid = [(c[0] / n) as f32, (c[1] / n) as f32, (c[2] / n) as f32];
                let target = if centroid[0] >= 0.0 { left } else { right };
                let err_mm = ((centroid[0] - target[0]).powi(2)
                    + (centroid[1] - target[1]).powi(2)
                    + (centroid[2] - target[2]).powi(2))
                .sqrt()
                    * 1000.0;
                if err_mm > 1.5 {
                    return Err(PackRwError::AlignmentDrift {
                        submesh: sm.name.clone(),
                        err_mm,
                    });
                }
            }
        }
    }
    Ok((eye_shifts, mouth_shift, haircap_conform))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// An `n`x`n` triangulated surface over `[-extent, extent]` in x/z, height from `f`,
    /// wound so the smooth normal points +y.
    fn surface(
        n: usize,
        extent: f32,
        f: impl Fn(f32, f32) -> f32,
    ) -> (Vec<[f32; 3]>, Vec<[u32; 3]>) {
        let mut p = Vec::new();
        for i in 0..n {
            for j in 0..n {
                let x = -extent + 2.0 * extent * (i as f32) / ((n - 1) as f32);
                let z = -extent + 2.0 * extent * (j as f32) / ((n - 1) as f32);
                p.push([x, f(x, z), z]);
            }
        }
        let mut t = Vec::new();
        for i in 0..n - 1 {
            for j in 0..n - 1 {
                let (a, b) = ((i * n + j) as u32, ((i + 1) * n + j) as u32);
                let (c, d) = ((i * n + j + 1) as u32, ((i + 1) * n + j + 1) as u32);
                t.push([a, c, b]);
                t.push([b, c, d]);
            }
        }
        (p, t)
    }

    fn ctx<'a>(
        donor: &'a (Vec<[f32; 3]>, Vec<[u32; 3]>),
        new: &'a (Vec<[f32; 3]>, Vec<[u32; 3]>),
    ) -> ShellConform<'a> {
        ShellConform {
            donor_head: &donor.0,
            donor_triangles: &donor.1,
            new_head: &new.0,
            new_triangles: &new.1,
        }
    }

    #[test]
    fn conforming_a_shell_carries_its_clearance_onto_the_new_skull() {
        // The whole point: the cap keeps the standoff it was authored with, over a skull
        // that moved. 5 mm above a donor surface that drops 10 mm must end 5 mm above the
        // new one — this is what re-seats the eyelashes when the eye plane moves back.
        let donor = surface(9, 0.10, |_, _| 0.0);
        let new = surface(9, 0.10, |_, _| -0.010);
        let (mut shell, stri) = surface(5, 0.05, |_, _| 0.005);
        let rep = conform_shell(&mut shell, &stri, &ctx(&donor, &new)).expect("conform");

        for p in &shell {
            assert!(
                (p[1] + 0.005).abs() < 5e-4,
                "vertex at y={} should have followed the skull down to -5 mm",
                p[1]
            );
        }
        assert_eq!(rep.clearance_after.1, 0.0, "nothing may end up buried");
        assert!((rep.clearance_after.0 - 5.0).abs() < 0.5);
    }

    #[test]
    fn a_shell_swallowed_by_the_new_skull_is_pushed_back_out() {
        // The defect this fixes: a CC5 skull that does not match the donor left 20% of the
        // cap *inside* the head. Vanilla caps never penetrate, so the conform floors it.
        let donor = surface(9, 0.10, |_, _| 0.0);
        let new = surface(9, 0.10, |_, _| 0.020);
        let (mut shell, stri) = surface(5, 0.05, |_, _| 0.005);
        let rep = conform_shell(&mut shell, &stri, &ctx(&donor, &new)).expect("conform");

        assert_eq!(
            rep.clearance_before.1, 1.0,
            "fixture must start fully buried"
        );
        assert_eq!(rep.clearance_after.1, 0.0, "and end fully outside");
        for p in &shell {
            assert!(
                p[1] > 0.020,
                "vertex at y={} is still inside the skull",
                p[1]
            );
        }
    }

    #[test]
    fn coincident_shell_vertices_stay_welded_through_the_conform() {
        // The smoothing pass averages over the 1-ring, and a UV seam is a duplicated
        // vertex whose two copies have different neighbours. Smoothing them independently
        // would pull the copies apart and crack the cap open, so the field is solved on
        // welded representatives and broadcast back.
        let donor = surface(9, 0.10, |_, _| 0.0);
        // Tilted, so the displacement genuinely varies across the seam.
        let new = surface(9, 0.10, |x, _| -0.010 + 0.25 * x);

        // Two quads meeting along x = 0.02, with that edge duplicated per quad.
        let mut shell = vec![
            [0.00, 0.005, 0.00],
            [0.02, 0.005, 0.00],
            [0.00, 0.005, 0.02],
            [0.02, 0.005, 0.02],
            [0.02, 0.005, 0.00], // duplicate of 1
            [0.02, 0.005, 0.02], // duplicate of 3
            [0.04, 0.005, 0.00],
            [0.04, 0.005, 0.02],
        ];
        let stri = [[0, 2, 1], [1, 2, 3], [4, 5, 6], [6, 5, 7]];
        conform_shell(&mut shell, &stri, &ctx(&donor, &new)).expect("conform");

        assert_eq!(shell[1], shell[4], "seam vertex split apart");
        assert_eq!(shell[3], shell[5], "seam vertex split apart");
    }

    #[test]
    fn scaling_about_the_centroid_leaves_the_centroid_put() {
        // The packer scales first and translates second, which is only safe because
        // scaling cannot move the centroid — otherwise the resize would drag the eye off
        // the center the source measured.
        let mut set = vec![
            [0.02f32, 1.83, 0.09],
            [0.04, 1.83, 0.09],
            [0.03, 1.84, 0.09],
            [0.03, 1.82, 0.09],
        ];
        let before = centroid_of(&set);
        scale_about_centroid(&mut set, 1.5);
        let after = centroid_of(&set);
        for k in 0..3 {
            assert!(
                (before[k] - after[k]).abs() < 1e-6,
                "centroid moved on axis {k}"
            );
        }
        // Extent grew by exactly the factor.
        let width = set.iter().fold(f32::MIN, |m, p| m.max(p[0]))
            - set.iter().fold(f32::MAX, |m, p| m.min(p[0]));
        assert!(
            (width - 0.03).abs() < 1e-6,
            "0.02 wide × 1.5 = 0.03, got {width}"
        );
    }

    #[test]
    fn normals_are_welded_across_a_uv_seam_split() {
        // Two quads meeting at a 90° fold, split along the fold the way a UV seam does:
        // verts 2,3 and 4,5 are coincident pairs. Unwelded, each copy sees only its own
        // face and the seam shades as a hard edge; welded, both copies share the average.
        let positions = vec![
            [0.0f32, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0], // seam copy A
            [1.0, 1.0, 0.0], // seam copy A
            [0.0, 1.0, 0.0], // seam copy B (same position as vert 2)
            [1.0, 1.0, 0.0], // seam copy B (same position as vert 3)
            [0.0, 1.0, 1.0],
            [1.0, 1.0, 1.0],
        ];
        let triangles = vec![
            [0, 1, 2],
            [1, 3, 2], // flat panel, normal +z
            [4, 5, 6],
            [5, 7, 6], // folded panel, normal -y
        ];
        let n = smooth_normals(&positions, &triangles);
        for (a, b) in [(2usize, 4usize), (3, 5)] {
            let dot = n[a][0] * n[b][0] + n[a][1] * n[b][1] + n[a][2] * n[b][2];
            assert!(
                dot > 0.999,
                "seam copies {a}/{b} must share a normal, got {:?} vs {:?}",
                n[a],
                n[b]
            );
        }
        // And the welded normal is the AREA-weighted blend of both panels, not either one
        // alone: this corner gets two flat-panel triangles (+z, weight 2) and one folded
        // triangle (−y, weight 1), so it lands on normalize(0, −1, 2).
        let expect = {
            let l = (1.0f32 + 4.0).sqrt();
            [0.0f32, -1.0 / l, 2.0 / l]
        };
        for k in 0..3 {
            assert!(
                (n[2][k] - expect[k]).abs() < 1e-5,
                "welded normal {:?}, expected {expect:?}",
                n[2]
            );
        }
    }

    #[test]
    fn welding_leaves_an_unsplit_mesh_untouched() {
        let positions = vec![[0.0f32, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0]];
        let triangles = vec![[0, 1, 2]];
        let n = smooth_normals(&positions, &triangles);
        for v in &n {
            assert!((v[2] - 1.0).abs() < 1e-6, "flat +z triangle: {v:?}");
        }
    }

    #[test]
    fn welding_does_not_merge_distinct_surfaces_that_merely_touch() {
        // Two triangles 0.1 mm apart — the closed-mouth lip case. Exact-equality welding
        // must leave them independent; an epsilon weld would fuse the lips.
        let positions = vec![
            [0.0f32, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0001],
            [1.0, 0.0, 0.0001],
            [0.0, -1.0, 0.0001],
        ];
        // Wound so the second triangle faces −z against the first's +z.
        let triangles = vec![[0, 1, 2], [3, 4, 5]];
        let n = smooth_normals(&positions, &triangles);
        assert!((n[0][2] - 1.0).abs() < 1e-6, "first face +z: {:?}", n[0]);
        assert!((n[3][2] + 1.0).abs() < 1e-6, "second face −z: {:?}", n[3]);
        let dot = n[0][0] * n[3][0] + n[0][1] * n[3][1] + n[0][2] * n[3][2];
        assert!(
            dot < 0.0,
            "0.1 mm apart must NOT weld — an epsilon weld would fuse a closed mouth: {dot}"
        );
    }

    #[test]
    fn unit_scale_is_a_no_op() {
        let original = vec![[0.1f32, 0.2, 0.3], [0.4, 0.5, 0.6]];
        let mut set = original.clone();
        scale_about_centroid(&mut set, 1.0);
        assert_eq!(set, original);
    }

    #[test]
    fn empty_set_scales_without_panicking() {
        let mut set: Vec<[f32; 3]> = Vec::new();
        scale_about_centroid(&mut set, 2.0);
        assert!(set.is_empty());
    }

    fn centroid_of(set: &[[f32; 3]]) -> [f32; 3] {
        let mut c = [0f64; 3];
        for p in set {
            for k in 0..3 {
                c[k] += f64::from(p[k]);
            }
        }
        let n = set.len() as f64;
        [(c[0] / n) as f32, (c[1] / n) as f32, (c[2] / n) as f32]
    }
}
