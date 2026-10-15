//! NHL24 Frostbite skeleton asset ingest + Legacy bone-palette naming.
//!
//! The Legacy `player_<id>.rx2` stores a 122-bone palette as bind
//! matrices with **no bone names** — bone identity is by palette index
//! only (see `docs/rx2-player-mesh-format.md`). Every bone we use today
//! (skull=24, jaw=48, tongue=63, …) was reverse-engineered empirically
//! via `playermesh bones-at`.
//!
//! The NHL24 player skeleton (`skeleton_player.bin`, a raw RIFF-EBX
//! `SkeletonAsset`) carries 433 *named* bones with model-space bind
//! poses — and the rig lives in the **same model space** as Legacy
//! (meters, standing pose; NHL24 Head bind at y=1.781, Legacy template
//! eye sub-meshes at y=1.833). So nearest-bind-position correlation
//! against the NHL24 skeleton names the Legacy palette.
//!
//! Parsing is a targeted scrape, not full EBX reflection:
//! - bone names are sequential null-terminated C-strings starting at
//!   `Reference\0` (bone 0 of EA rigs is always `Reference`);
//! - the three pose arrays (LocalPose / ModelPose / InverseModelPose)
//!   are `u32 count` followed by `count × 64` bytes of little-endian
//!   row-major 4×4 affine matrices (rows 0–2 basis, row 3 translation).
//!   ModelPose is picked as the array where the most bones sit at an
//!   anatomical height (translation y > 0.5 m) — LocalPose holds small
//!   parent-relative offsets and InverseModelPose negated values.

use thiserror::Error;

use crate::playermesh::BoneMatrix;

#[derive(Debug, Error)]
pub enum SkeletonError {
    #[error("input is too short to be a RIFF-EBX asset ({0} bytes)")]
    TooShort(usize),
    #[error("not a RIFF-EBX asset (RIFF/EBX magic mismatch)")]
    BadMagic,
    #[error("no `Reference\\0` bone-name run found")]
    NoNameRun,
    #[error("bone-name run invalid at name {index}: {reason}")]
    BadNameRun { index: usize, reason: String },
    #[error(
        "no count-prefixed matrix array found (expected LocalPose/ModelPose/InverseModelPose)"
    )]
    NoMatrixArrays,
    #[error("matrix arrays disagree on bone count: {0:?}")]
    CountMismatch(Vec<usize>),
    #[error("could not single out the ModelPose array (anatomical-height counts: {0:?})")]
    AmbiguousModelPose(Vec<usize>),
}

/// One named bone with its model-space bind pose.
#[derive(Debug, Clone)]
pub struct NamedBone {
    pub name: String,
    /// Bind translation in model space (meters).
    pub trans: [f32; 3],
    /// Bind rotation basis rows (rows 0–2 of the affine matrix).
    pub rot: [[f32; 3]; 3],
}

#[derive(Debug, Clone)]
pub struct NamedSkeleton {
    pub bones: Vec<NamedBone>,
}

/// Parse a raw RIFF-EBX `SkeletonAsset` (e.g. NHL24 `skeleton_player.bin`).
pub fn parse_skeleton_ebx(data: &[u8]) -> Result<NamedSkeleton, SkeletonError> {
    if data.len() < 16 {
        return Err(SkeletonError::TooShort(data.len()));
    }
    if &data[0..4] != b"RIFF" || &data[8..12] != b"EBX\0" {
        return Err(SkeletonError::BadMagic);
    }

    let arrays = find_matrix_arrays(data);
    if arrays.is_empty() {
        return Err(SkeletonError::NoMatrixArrays);
    }
    let count = arrays[0].1;
    if arrays.iter().any(|&(_, c)| c != count) {
        return Err(SkeletonError::CountMismatch(
            arrays.iter().map(|&(_, c)| c).collect(),
        ));
    }

    let names = read_name_run(data, count)?;
    let model = pick_model_pose(data, &arrays)?;

    let mut bones = Vec::with_capacity(count);
    for (i, name) in names.into_iter().enumerate() {
        let m = read_matrix_le(data, model + 4 + i * 64);
        bones.push(NamedBone {
            name,
            trans: [m[3][0], m[3][1], m[3][2]],
            rot: [
                [m[0][0], m[0][1], m[0][2]],
                [m[1][0], m[1][1], m[1][2]],
                [m[2][0], m[2][1], m[2][2]],
            ],
        });
    }
    Ok(NamedSkeleton { bones })
}

/// Scan for `u32 count` immediately followed by `count` 64-byte
/// matrices whose first matrix is identity-like (bone 0 = `Reference`
/// binds at the origin in every pose array). Returns `(count_offset,
/// count)` per hit.
fn find_matrix_arrays(data: &[u8]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    let mut i = 0usize;
    while i + 4 + 64 <= data.len() {
        let n = u32::from_le_bytes(data[i..i + 4].try_into().unwrap()) as usize;
        if (16..=2048).contains(&n) && i + 4 + n * 64 <= data.len() {
            let m = read_matrix_le(data, i + 4);
            if is_identity_like(&m) {
                out.push((i, n));
                // Skip past this array — its payload can't contain
                // another array's count prefix we'd want.
                i += 4 + n * 64;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// Among the discovered pose arrays, ModelPose is the one where the
/// most bones sit at an anatomical height (trans.y > 0.5 m).
fn pick_model_pose(data: &[u8], arrays: &[(usize, usize)]) -> Result<usize, SkeletonError> {
    let mut scored: Vec<(usize, usize)> = arrays
        .iter()
        .map(|&(off, n)| {
            let tall = (0..n)
                .filter(|&b| {
                    let m = read_matrix_le(data, off + 4 + b * 64);
                    m[3][1] > 0.5
                })
                .count();
            (tall, off)
        })
        .collect();
    scored.sort_by_key(|s| std::cmp::Reverse(s.0));
    let best = scored[0];
    if scored.len() > 1 && scored[1].0 == best.0 {
        return Err(SkeletonError::AmbiguousModelPose(
            scored.iter().map(|&(t, _)| t).collect(),
        ));
    }
    Ok(best.1)
}

/// Read `count` consecutive null-terminated bone names starting at a
/// `Reference\0` occurrence. Tries every occurrence until one yields a
/// clean run (all names non-empty printable ASCII ≤ 63 chars).
fn read_name_run(data: &[u8], count: usize) -> Result<Vec<String>, SkeletonError> {
    let needle = b"Reference\0";
    let mut last_err = SkeletonError::NoNameRun;
    let mut search_from = 0usize;
    while let Some(rel) = find_bytes(&data[search_from..], needle) {
        let start = search_from + rel;
        match try_name_run(data, start, count) {
            Ok(names) => return Ok(names),
            Err(e) => last_err = e,
        }
        search_from = start + 1;
    }
    Err(last_err)
}

fn try_name_run(data: &[u8], start: usize, count: usize) -> Result<Vec<String>, SkeletonError> {
    let mut names = Vec::with_capacity(count);
    let mut p = start;
    for index in 0..count {
        let end = match data[p..].iter().position(|&b| b == 0) {
            Some(rel) => p + rel,
            None => {
                return Err(SkeletonError::BadNameRun {
                    index,
                    reason: "ran past EOF".into(),
                })
            }
        };
        let raw = &data[p..end];
        if raw.is_empty() || raw.len() > 63 {
            return Err(SkeletonError::BadNameRun {
                index,
                reason: format!("name length {} out of range 1..=63", raw.len()),
            });
        }
        if !raw.iter().all(|&b| (0x20..0x7F).contains(&b)) {
            return Err(SkeletonError::BadNameRun {
                index,
                reason: "non-printable byte in name".into(),
            });
        }
        names.push(String::from_utf8_lossy(raw).into_owned());
        p = end + 1;
    }
    Ok(names)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

fn read_matrix_le(data: &[u8], offset: usize) -> BoneMatrix {
    let mut m: BoneMatrix = [[0.0; 4]; 4];
    for (row, cells) in m.iter_mut().enumerate() {
        for (col, cell) in cells.iter_mut().enumerate() {
            let o = offset + (row * 4 + col) * 4;
            *cell = f32::from_le_bytes(data[o..o + 4].try_into().unwrap());
        }
    }
    m
}

fn is_identity_like(m: &BoneMatrix) -> bool {
    const EPS: f32 = 1e-4;
    for (row, cells) in m.iter().enumerate().take(3) {
        for (col, cell) in cells.iter().enumerate().take(3) {
            let want = if row == col { 1.0 } else { 0.0 };
            if (cell - want).abs() > EPS {
                return false;
            }
        }
    }
    // Row 3 (translation) must be at the origin for `Reference`.
    m[3][0].abs() <= EPS && m[3][1].abs() <= EPS && m[3][2].abs() <= EPS
}

// ---------- Palette correlation ----------

/// Decompose a Legacy palette matrix into the bone's model-space bind
/// `(origin, basis-rows)`, directly comparable to a [`NamedBone`].
///
/// The palette stores **inverse-bind** matrices in row-vector form
/// (`v_bone = v_model · R + t`), NOT forward bone-to-model transforms:
/// bone 0 of `player_20500.rx2` decomposes to origin (0, 1.0886,
/// 0.0139) with basis rows equal to the NHL24 `Hips` ModelPose rows to
/// 4 decimals — proof both of the convention and that the rigs share a
/// model space. (The bind-pose GLB export can't tell the difference:
/// `node.matrix × IBM` cancels either way.) So:
///   origin_i  = -(row_i · t)        (orthonormal R)
///   basis row j = column j of R     (R⁻¹ = Rᵀ)
pub fn palette_bind_pose(m: &BoneMatrix) -> ([f32; 3], [[f32; 3]; 3]) {
    let t = [m[3][0], m[3][1], m[3][2]];
    let origin = [
        -(m[0][0] * t[0] + m[0][1] * t[1] + m[0][2] * t[2]),
        -(m[1][0] * t[0] + m[1][1] * t[1] + m[1][2] * t[2]),
        -(m[2][0] * t[0] + m[2][1] * t[1] + m[2][2] * t[2]),
    ];
    let rot = [
        [m[0][0], m[1][0], m[2][0]],
        [m[0][1], m[1][1], m[2][1]],
        [m[0][2], m[1][2], m[2][2]],
    ];
    (origin, rot)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    /// Unique nearest bone, well within tolerance.
    Exact,
    /// Within the inter-generation drift band (5–20 mm) — almost
    /// certainly the right bone, but a densely-packed facial neighbor
    /// isn't impossible; check the candidate list.
    Near,
    /// Several skeleton bones are coincident at this position and
    /// rotation doesn't separate them decisively.
    Ambiguous,
    /// Nearest bone is farther than the drift band — the rigs may have
    /// diverged here; treat the name as a hint only.
    Low,
}

impl Confidence {
    pub fn as_str(self) -> &'static str {
        match self {
            Confidence::Exact => "exact",
            Confidence::Near => "near",
            Confidence::Ambiguous => "ambiguous",
            Confidence::Low => "low",
        }
    }
}

#[derive(Debug, Clone)]
pub struct BoneMatch {
    pub palette_index: usize,
    pub name: String,
    /// Bind-translation distance to the named bone (meters).
    pub dist: f32,
    /// Top candidates as `(name, dist)`, best first.
    pub candidates: Vec<(String, f32)>,
    pub confidence: Confidence,
}

/// Distance within which a unique match is `Exact` (5 mm).
pub const DIST_TOLERANCE: f32 = 0.005;
/// Distance within which a match is `Near` — the observed Legacy↔NHL24
/// rig drift band on the template head (most facial bones land 5–25 mm
/// from their NHL24 counterpart). Beyond this it's `Low`.
pub const DRIFT_TOLERANCE: f32 = 0.02;
/// Candidates within this much of the best distance are "coincident"
/// and get re-ranked by rotation similarity (0.5 mm).
const TIE_EPS: f32 = 0.0005;
/// Minimum rotation-similarity margin for a coincident tie to count as
/// decisively separated.
const ROT_MARGIN: f32 = 1e-3;

/// Name each Legacy palette matrix by nearest NHL24 bind translation,
/// tie-breaking coincident bones by rotation-row similarity.
pub fn correlate_palette(palette: &[BoneMatrix], skel: &NamedSkeleton) -> Vec<BoneMatch> {
    palette
        .iter()
        .enumerate()
        .map(|(palette_index, m)| {
            let (pt, prot) = palette_bind_pose(m);
            let mut ranked: Vec<(usize, f32)> = skel
                .bones
                .iter()
                .enumerate()
                .map(|(i, b)| (i, dist3(pt, b.trans)))
                .collect();
            ranked.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));

            let best_dist = ranked[0].1;
            // Re-rank the coincident head of the list by rotation
            // similarity (e.g. Head/Face/Helmet share a translation).
            let tie_count = ranked
                .iter()
                .take_while(|&&(_, d)| d - best_dist <= TIE_EPS)
                .count();
            let mut rot_scores: Vec<(usize, f32)> = ranked[..tie_count]
                .iter()
                .map(|&(i, _)| (i, rot_similarity(&prot, &skel.bones[i].rot)))
                .collect();
            rot_scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
            let ordered: Vec<usize> = rot_scores
                .iter()
                .map(|&(i, _)| i)
                .chain(ranked[tie_count..].iter().map(|&(i, _)| i))
                .collect();

            let dist_of = |i: usize| dist3(pt, skel.bones[i].trans);
            let candidates: Vec<(String, f32)> = ordered
                .iter()
                .take(3)
                .map(|&i| (skel.bones[i].name.clone(), dist_of(i)))
                .collect();

            let confidence = if best_dist > DRIFT_TOLERANCE {
                Confidence::Low
            } else if tie_count > 1 && rot_scores[0].1 - rot_scores[1].1 < ROT_MARGIN {
                Confidence::Ambiguous
            } else if best_dist > DIST_TOLERANCE {
                Confidence::Near
            } else {
                Confidence::Exact
            };

            BoneMatch {
                palette_index,
                name: skel.bones[ordered[0]].name.clone(),
                dist: best_dist,
                candidates,
                confidence,
            }
        })
        .collect()
}

fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

/// Sum of dot products of the three basis rows — 3.0 for identical
/// orthonormal bases, lower as they diverge.
fn rot_similarity(a: &[[f32; 3]; 3], b: &[[f32; 3]; 3]) -> f32 {
    (0..3)
        .map(|r| a[r][0] * b[r][0] + a[r][1] * b[r][1] + a[r][2] * b[r][2])
        .sum()
}

// ---------- Tests ----------

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a minimal RIFF-EBX-shaped buffer: header, three
    /// count-prefixed matrix arrays (local / model / inverse), then the
    /// bone-name run. Mirrors the layout scraped from the real
    /// `skeleton_player.bin` (which stores names *after* the arrays).
    fn synthetic_skeleton(n: usize) -> (Vec<u8>, Vec<[f32; 3]>) {
        assert!(n >= 16);
        let mut model_trans = Vec::with_capacity(n);
        for i in 0..n {
            // Bones 0/1 at origin (Reference/AITrajectory), the rest at
            // anatomical heights ≈1.0–1.9 m.
            let t = if i < 2 {
                [0.0, 0.0, 0.0]
            } else {
                [0.01 * i as f32, 1.0 + 0.02 * i as f32, 0.05]
            };
            model_trans.push(t);
        }

        let mut buf = Vec::new();
        buf.extend_from_slice(b"RIFF");
        buf.extend_from_slice(&0u32.to_le_bytes()); // size — unchecked
        buf.extend_from_slice(b"EBX\0");
        buf.extend_from_slice(b"EBXD");
        buf.extend_from_slice(&[0u8; 32]); // filler

        let push_matrix = |buf: &mut Vec<u8>, trans: [f32; 3]| {
            let mut m = [[0f32; 4]; 4];
            m[0][0] = 1.0;
            m[1][1] = 1.0;
            m[2][2] = 1.0;
            m[3] = [trans[0], trans[1], trans[2], 0.0];
            for row in m {
                for v in row {
                    buf.extend_from_slice(&v.to_le_bytes());
                }
            }
        };

        // LocalPose: parent-relative — small offsets, bone 0 identity.
        buf.extend_from_slice(&(n as u32).to_le_bytes());
        for i in 0..n {
            let t = if i < 2 {
                [0.0, 0.0, 0.0]
            } else {
                [0.0, 0.05, 0.0]
            };
            push_matrix(&mut buf, t);
        }
        // ModelPose.
        buf.extend_from_slice(&(n as u32).to_le_bytes());
        for &t in &model_trans {
            push_matrix(&mut buf, t);
        }
        // InverseModelPose: negated translations.
        buf.extend_from_slice(&(n as u32).to_le_bytes());
        for &t in &model_trans {
            push_matrix(&mut buf, [-t[0], -t[1], -t[2]]);
        }

        // Name run.
        buf.extend_from_slice(b"Reference\0AITrajectory\0");
        for i in 2..n {
            buf.extend_from_slice(format!("Bone{i:03}\0").as_bytes());
        }
        (buf, model_trans)
    }

    #[test]
    fn parses_synthetic_and_picks_model_pose() {
        let (buf, model_trans) = synthetic_skeleton(20);
        let skel = parse_skeleton_ebx(&buf).expect("parse");
        assert_eq!(skel.bones.len(), 20);
        assert_eq!(skel.bones[0].name, "Reference");
        assert_eq!(skel.bones[1].name, "AITrajectory");
        assert_eq!(skel.bones[19].name, "Bone019");
        // Translations must come from the ModelPose array, not the
        // local or inverse ones.
        for (b, want) in skel.bones.iter().zip(&model_trans) {
            for (k, (got, wanted)) in b.trans.iter().zip(want.iter()).enumerate() {
                assert!(
                    (got - wanted).abs() < 1e-6,
                    "bone {} axis {k}: got {} want {}",
                    b.name,
                    b.trans[k],
                    want[k]
                );
            }
        }
    }

    #[test]
    fn rejects_bad_magic() {
        assert!(matches!(
            parse_skeleton_ebx(b"NOPE-this-is-not-an-ebx-asset-at-all"),
            Err(SkeletonError::BadMagic)
        ));
    }

    fn bone(name: &str, trans: [f32; 3], rot: [[f32; 3]; 3]) -> NamedBone {
        NamedBone {
            name: name.into(),
            trans,
            rot,
        }
    }

    const ID_ROT: [[f32; 3]; 3] = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    // 90° about Z — clearly separated from identity.
    const Z90_ROT: [[f32; 3]; 3] = [[0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 0.0, 1.0]];

    /// Build a palette matrix in the stored inverse-bind convention
    /// from a desired model-space bind `(origin, basis-rows)` — the
    /// exact inverse of [`palette_bind_pose`].
    fn palette_entry(trans: [f32; 3], rot: [[f32; 3]; 3]) -> BoneMatrix {
        let mut m = [[0f32; 4]; 4];
        for i in 0..3 {
            for j in 0..3 {
                m[i][j] = rot[j][i]; // stored column j = bind basis row j
            }
            m[3][i] = -(rot[i][0] * trans[0] + rot[i][1] * trans[1] + rot[i][2] * trans[2]);
        }
        m
    }

    /// Ground truth captured from `player_20500.rx2` bone 0 vs the
    /// NHL24 skeleton's `Hips`: the stored palette matrix decomposes to
    /// Hips' exact model-space bind pose (origin + basis to 4 decimals).
    #[test]
    fn palette_bind_pose_matches_real_hips() {
        let stored: BoneMatrix = [
            [0.0, 0.0, 1.0, 0.0],
            [0.9805, 0.1968, 0.0, 0.0],
            [-0.1968, 0.9805, 0.0, 0.0],
            [-1.0646, -0.2279, 0.0, 0.0],
        ];
        let (origin, rot) = palette_bind_pose(&stored);
        let want_origin = [0.0, 1.0886, 0.0139];
        let want_rot = [
            [0.0, 0.9805, -0.1968],
            [0.0, 0.1968, 0.9805],
            [1.0, 0.0, 0.0],
        ];
        for k in 0..3 {
            assert!(
                (origin[k] - want_origin[k]).abs() < 1e-3,
                "origin[{k}]={}",
                origin[k]
            );
            for j in 0..3 {
                assert!(
                    (rot[k][j] - want_rot[k][j]).abs() < 1e-4,
                    "rot[{k}][{j}]={}",
                    rot[k][j]
                );
            }
        }
        // And the test helper must round-trip back to the same matrix.
        let rebuilt = palette_entry(origin, rot);
        for r in 0..4 {
            for c in 0..3 {
                assert!((rebuilt[r][c] - stored[r][c]).abs() < 1e-3);
            }
        }
    }

    #[test]
    fn coincident_bones_disambiguated_by_rotation() {
        let skel = NamedSkeleton {
            bones: vec![
                bone("Head", [0.0, 1.78, 0.02], ID_ROT),
                bone("Helmet", [0.0, 1.78, 0.02], Z90_ROT),
                bone("Jaw", [0.0, 1.74, 0.03], ID_ROT),
            ],
        };
        let palette = vec![
            palette_entry([0.0, 1.78, 0.02], ID_ROT),
            palette_entry([0.0, 1.78, 0.02], Z90_ROT),
        ];
        let matches = correlate_palette(&palette, &skel);
        assert_eq!(matches[0].name, "Head");
        assert_eq!(matches[0].confidence, Confidence::Exact);
        assert_eq!(matches[1].name, "Helmet");
        assert_eq!(matches[1].confidence, Confidence::Exact);
    }

    #[test]
    fn coincident_bones_with_same_rotation_are_ambiguous() {
        let skel = NamedSkeleton {
            bones: vec![
                bone("Head", [0.0, 1.78, 0.02], ID_ROT),
                bone("Face", [0.0, 1.78, 0.02], ID_ROT),
            ],
        };
        let palette = vec![palette_entry([0.0, 1.78, 0.02], ID_ROT)];
        let matches = correlate_palette(&palette, &skel);
        assert_eq!(matches[0].confidence, Confidence::Ambiguous);
        assert_eq!(matches[0].candidates.len(), 2);
    }

    #[test]
    fn drift_band_match_is_near_confidence() {
        let skel = NamedSkeleton {
            bones: vec![bone("Head", [0.0, 1.78, 0.02], ID_ROT)],
        };
        // 10 mm off — inside the drift band, outside exact tolerance.
        let palette = vec![palette_entry([0.0, 1.79, 0.02], ID_ROT)];
        let matches = correlate_palette(&palette, &skel);
        assert_eq!(matches[0].confidence, Confidence::Near);
    }

    #[test]
    fn distant_match_is_low_confidence() {
        let skel = NamedSkeleton {
            bones: vec![bone("Head", [0.0, 1.78, 0.02], ID_ROT)],
        };
        let palette = vec![palette_entry([0.0, 1.60, 0.02], ID_ROT)];
        let matches = correlate_palette(&palette, &skel);
        assert_eq!(matches[0].confidence, Confidence::Low);
        assert!(matches[0].dist > DIST_TOLERANCE);
    }
}
