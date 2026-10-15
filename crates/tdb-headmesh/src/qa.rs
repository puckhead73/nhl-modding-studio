//! Distortion QA — a core pipeline stage, not a dev tool.
//!
//! Every Path A conversion deforms the head under a pose battery twice — once with the
//! **authored source weights** on the source rig, once with the **remapped palette
//! weights** on the target rig — and reports the displacement error split into:
//!
//! * **retarget error** — poses on source bones that resolved to a target slot
//!   (Direct/Override). Remap-tunable: a better bone mapping shrinks it.
//! * **DOF-loss floor** — poses on source bones whose mass ancestor-collapsed or
//!   dropped: the target rig simply cannot reproduce the motion. Irreducible without a
//!   richer target rig.
//!
//! The **facial battery** (poses resolving into the facial palette buckets of
//! `defs/appearance/facial-regions.toml`, i.e. everything except the `head`/`neck`
//! buckets) is reported separately — full-battery numbers are dominated by neck poses
//! and mislead about face quality. Prior NHL24→Legacy baseline (brett_hull): facial RMS
//! 0.50 mm / max 5.23 mm; DOF-loss floor RMS 0.10 mm.
//!
//! Battery definition here: for every source bone carrying ≥ [`MIN_BONE_MASS`] authored
//! mass, rotate [`BATTERY_ANGLE_DEG`]° about X and about Z at the bone's bind origin
//! (linear-blend skinning, single-bone poses). This is a re-derivation, not a port —
//! semantics are documented so numbers are comparable run-to-run, not to the prior
//! tool's absolute values.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;

use crate::remap::{BoneMap, BoneResolution};

/// Facial-battery thresholds (mm) for the traffic-light verdict, anchored on the proven
/// baseline with headroom. Recalibrate against corpus outcomes, not vibes.
pub const FACIAL_RMS_WARN_MM: f64 = 1.5;
pub const FACIAL_RMS_FAIL_MM: f64 = 4.0;
pub const FACIAL_MAX_WARN_MM: f64 = 8.0;
pub const FACIAL_MAX_FAIL_MM: f64 = 20.0;

/// Bones with less total authored mass than this don't get battery poses.
pub const MIN_BONE_MASS: f64 = 0.5;
pub const BATTERY_ANGLE_DEG: f32 = 15.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Caution,
    Fail,
}

#[derive(Debug, Clone, Serialize)]
pub struct Scorecard {
    /// Full retarget battery, all vertices, all mapped-bone poses.
    pub retarget_rms_mm: f64,
    pub retarget_max_mm: f64,
    /// Irreducible floor from collapsed/dropped source DOFs.
    pub dof_loss_rms_mm: f64,
    pub dof_loss_max_mm: f64,
    /// Facial battery only — the number that predicts in-game face quality.
    pub facial_rms_mm: f64,
    pub facial_max_mm: f64,
    pub vertex_count: usize,
    pub battery_poses: usize,
    pub verdict: Verdict,
}

pub fn verdict(facial_rms_mm: f64, facial_max_mm: f64) -> Verdict {
    if facial_rms_mm >= FACIAL_RMS_FAIL_MM || facial_max_mm >= FACIAL_MAX_FAIL_MM {
        Verdict::Fail
    } else if facial_rms_mm >= FACIAL_RMS_WARN_MM || facial_max_mm >= FACIAL_MAX_WARN_MM {
        Verdict::Caution
    } else {
        Verdict::Pass
    }
}

// ---------- facial buckets ----------

#[derive(Debug, Deserialize)]
struct RegionsFile {
    legacy: RegionsLegacy,
}

#[derive(Debug, Deserialize)]
struct RegionsLegacy {
    palette_buckets: HashMap<String, Vec<u32>>,
}

#[derive(Debug, thiserror::Error)]
pub enum QaError {
    #[error("failed to read facial regions: {0}")]
    Io(#[from] std::io::Error),
    #[error("facial regions are not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("battery input lengths disagree: {0}")]
    BadInput(String),
}

/// Palette slots counted as *facial* — every bucket except `head` and `neck`.
pub fn facial_palette_set(regions_toml: &Path) -> Result<HashSet<u32>, QaError> {
    let file: RegionsFile = toml::from_str(&std::fs::read_to_string(regions_toml)?)?;
    Ok(file
        .legacy
        .palette_buckets
        .into_iter()
        .filter(|(name, _)| name != "head" && name != "neck")
        .flat_map(|(_, slots)| slots)
        .collect())
}

// ---------- battery ----------

fn rot(axis: usize, deg: f32) -> [[f32; 3]; 3] {
    let (s, c) = deg.to_radians().sin_cos();
    match axis {
        0 => [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]],
        1 => [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]],
        _ => [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]],
    }
}

fn apply(m: &[[f32; 3]; 3], v: [f32; 3]) -> [f32; 3] {
    [
        m[0][0] * v[0] + m[0][1] * v[1] + m[0][2] * v[2],
        m[1][0] * v[0] + m[1][1] * v[1] + m[1][2] * v[2],
        m[2][0] * v[0] + m[2][1] * v[1] + m[2][2] * v[2],
    ]
}

/// Displacement of `v` under a single-bone LBS pose: the bone rotates by `r` about its
/// bind origin `o`, the vertex holds weight `w` on it (all other bones identity).
fn lbs_displacement(v: [f32; 3], o: [f32; 3], r: &[[f32; 3]; 3], w: f32) -> [f32; 3] {
    if w <= 0.0 {
        return [0.0; 3];
    }
    let local = [v[0] - o[0], v[1] - o[1], v[2] - o[2]];
    let moved = apply(r, local);
    [
        w * (moved[0] - local[0]),
        w * (moved[1] - local[1]),
        w * (moved[2] - local[2]),
    ]
}

struct ErrorAccum {
    sq_sum: f64,
    count: u64,
    max: f64,
}

impl ErrorAccum {
    fn new() -> Self {
        ErrorAccum {
            sq_sum: 0.0,
            count: 0,
            max: 0.0,
        }
    }
    fn add(&mut self, err_m: f64) {
        let mm = err_m * 1000.0;
        self.sq_sum += mm * mm;
        self.count += 1;
        if mm > self.max {
            self.max = mm;
        }
    }
    fn rms(&self) -> f64 {
        if self.count == 0 {
            0.0
        } else {
            (self.sq_sum / self.count as f64).sqrt()
        }
    }
}

/// Run the battery.
///
/// * `positions` — head vertices (shared model space, meters).
/// * `source_influences[v]` — authored (source-skeleton bone, weight) list per vertex.
/// * `source_binds[b]` — source bone bind translations.
/// * `bone_map` — the remap resolution table.
/// * `target_influences[v]` — remapped (palette slot, weight) list per vertex.
/// * `target_binds[p]` — palette-slot bind origins of the donor rig.
/// * `facial` — facial palette-slot set for the facial split.
#[allow(clippy::too_many_arguments)]
pub fn run_battery(
    positions: &[[f32; 3]],
    source_influences: &[Vec<(u32, f32)>],
    source_binds: &[[f32; 3]],
    bone_map: &BoneMap,
    target_influences: &[Vec<(u32, f32)>],
    target_binds: &[[f32; 3]],
    facial: &HashSet<u32>,
) -> Result<Scorecard, QaError> {
    if positions.len() != source_influences.len() || positions.len() != target_influences.len() {
        return Err(QaError::BadInput(format!(
            "positions {} / source {} / target {}",
            positions.len(),
            source_influences.len(),
            target_influences.len()
        )));
    }

    // Total authored mass per source bone → battery membership.
    let mut bone_mass: HashMap<u32, f64> = HashMap::new();
    for infl in source_influences {
        for &(b, w) in infl {
            *bone_mass.entry(b).or_insert(0.0) += f64::from(w);
        }
    }
    // Per-vertex weight on one source bone / one palette slot, precomputed as maps.
    let src_weight = |v: usize, bone: u32| -> f32 {
        source_influences[v]
            .iter()
            .find(|(b, _)| *b == bone)
            .map(|(_, w)| *w)
            .unwrap_or(0.0)
    };
    let tgt_weight = |v: usize, slot: u32| -> f32 {
        target_influences[v]
            .iter()
            .find(|(p, _)| *p == slot)
            .map(|(_, w)| *w)
            .unwrap_or(0.0)
    };

    let mut retarget = ErrorAccum::new();
    let mut dof_loss = ErrorAccum::new();
    let mut facial_acc = ErrorAccum::new();
    let mut poses = 0usize;

    let mut bones: Vec<(&u32, &f64)> = bone_mass.iter().collect();
    bones.sort_by_key(|(b, _)| **b);
    for (&bone, &mass) in bones {
        if mass < MIN_BONE_MASS {
            continue;
        }
        let Some(&src_o) = source_binds.get(bone as usize) else {
            continue;
        };
        let resolution = bone_map
            .resolutions
            .get(bone as usize)
            .copied()
            .unwrap_or(BoneResolution::Unmapped);
        let mapped = match resolution {
            BoneResolution::Direct(p) | BoneResolution::Override(p) => Some(p),
            BoneResolution::Collapsed { .. } | BoneResolution::Unmapped => None,
        };
        let facial_pose = match resolution {
            BoneResolution::Direct(p)
            | BoneResolution::Override(p)
            | BoneResolution::Collapsed { palette: p, .. } => facial.contains(&p),
            BoneResolution::Unmapped => false,
        };

        for axis in [0usize, 2] {
            let r = rot(axis, BATTERY_ANGLE_DEG);
            poses += 1;
            for (v, &pos) in positions.iter().enumerate() {
                let sw = src_weight(v, bone);
                if sw <= 0.0 {
                    continue;
                }
                let d_src = lbs_displacement(pos, src_o, &r, sw);
                let err = match mapped {
                    Some(slot) => {
                        let tw = tgt_weight(v, slot);
                        let t_o = target_binds.get(slot as usize).copied().unwrap_or(src_o);
                        let d_tgt = lbs_displacement(pos, t_o, &r, tw);
                        let dx = [
                            f64::from(d_src[0] - d_tgt[0]),
                            f64::from(d_src[1] - d_tgt[1]),
                            f64::from(d_src[2] - d_tgt[2]),
                        ];
                        (dx[0] * dx[0] + dx[1] * dx[1] + dx[2] * dx[2]).sqrt()
                    }
                    None => {
                        let dx = [
                            f64::from(d_src[0]),
                            f64::from(d_src[1]),
                            f64::from(d_src[2]),
                        ];
                        (dx[0] * dx[0] + dx[1] * dx[1] + dx[2] * dx[2]).sqrt()
                    }
                };
                match mapped {
                    Some(_) => retarget.add(err),
                    None => dof_loss.add(err),
                }
                if facial_pose {
                    facial_acc.add(err);
                }
            }
        }
    }

    let facial_rms = facial_acc.rms();
    let facial_max = facial_acc.max;
    Ok(Scorecard {
        retarget_rms_mm: retarget.rms(),
        retarget_max_mm: retarget.max,
        dof_loss_rms_mm: dof_loss.rms(),
        dof_loss_max_mm: dof_loss.max,
        facial_rms_mm: facial_rms,
        facial_max_mm: facial_max,
        vertex_count: positions.len(),
        battery_poses: poses,
        verdict: verdict(facial_rms, facial_max),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::palette::NamedPalette;
    use crate::remap::{BoneMap, BoneOverrides, SourceSkeleton};

    #[test]
    fn baseline_passes_and_bands_hold() {
        assert_eq!(verdict(0.50, 5.23), Verdict::Pass);
        assert_eq!(verdict(2.0, 5.0), Verdict::Caution);
        assert_eq!(verdict(0.5, 10.0), Verdict::Caution);
        assert_eq!(verdict(4.5, 5.0), Verdict::Fail);
        assert_eq!(verdict(0.5, 25.0), Verdict::Fail);
    }

    /// A perfect remap (same weights, same bind origins) scores zero retarget error;
    /// an unmapped bone's motion lands in the DOF-loss bucket.
    #[test]
    fn battery_zero_for_perfect_remap_and_dof_loss_for_dropped() {
        let palette = NamedPalette::from_toml_str(
            "[bones.8]\nname = \"Head\"\ndist = 0.001\n[bones.16]\nname = \"Jaw\"\ndist = 0.001\n",
        )
        .unwrap();
        let skeleton = SourceSkeleton {
            names: vec!["Head".into(), "Jaw".into(), "Wiggler".into()],
            parents: vec![None, Some(0), None],
        };
        let map = BoneMap::build(&skeleton, &palette, &BoneOverrides::default()).unwrap();

        let positions = vec![[0.0, 1.8, 0.1], [0.02, 1.75, 0.08]];
        // Authored: vertex 0 on Head, vertex 1 split Jaw/Wiggler (Wiggler mass = 0.5,
        // exactly at the battery threshold so it earns poses).
        let src = vec![vec![(0u32, 1.0f32)], vec![(1, 0.5), (2, 0.5)]];
        // Remapped perfectly for the mapped bones (Wiggler dropped, renormalized off in
        // this synthetic case to keep Jaw weight identical).
        let tgt = vec![vec![(8u32, 1.0f32)], vec![(16, 0.5)]];
        let src_binds = vec![[0.0, 1.78, 0.01], [0.0, 1.72, 0.05], [0.0, 1.8, 0.0]];
        // Palette binds indexed by slot: 8 and 16 match the source bones exactly.
        let mut tgt_binds = vec![[0.0f32; 3]; 17];
        tgt_binds[8] = [0.0, 1.78, 0.01];
        tgt_binds[16] = [0.0, 1.72, 0.05];
        let facial: HashSet<u32> = [16].into_iter().collect();

        let card = run_battery(
            &positions, &src, &src_binds, &map, &tgt, &tgt_binds, &facial,
        )
        .unwrap();
        assert!(card.retarget_rms_mm < 1e-6, "perfect remap ⇒ zero retarget");
        assert!(
            card.dof_loss_rms_mm > 0.0,
            "Wiggler motion is unreproducible"
        );
        assert_eq!(card.verdict, Verdict::Pass);
        assert_eq!(card.battery_poses, 6); // 3 bones × 2 axes
    }

    /// Weight lost in remap shows up as retarget error.
    #[test]
    fn battery_flags_weight_drift() {
        let palette =
            NamedPalette::from_toml_str("[bones.16]\nname = \"Jaw\"\ndist = 0.001\n").unwrap();
        let skeleton = SourceSkeleton {
            names: vec!["Jaw".into()],
            parents: vec![None],
        };
        let map = BoneMap::build(&skeleton, &palette, &BoneOverrides::default()).unwrap();
        let positions = vec![[0.0, 1.70, 0.10]];
        let src = vec![vec![(0u32, 1.0f32)]];
        let tgt = vec![vec![(16u32, 0.5f32)]]; // half the mass lost
        let src_binds = vec![[0.0, 1.72, 0.05]];
        let mut tgt_binds = vec![[0.0f32; 3]; 17];
        tgt_binds[16] = [0.0, 1.72, 0.05];
        let facial: HashSet<u32> = [16].into_iter().collect();
        let card = run_battery(
            &positions, &src, &src_binds, &map, &tgt, &tgt_binds, &facial,
        )
        .unwrap();
        assert!(card.retarget_rms_mm > 1.0);
        assert!(card.facial_rms_mm > 1.0);
    }
}
