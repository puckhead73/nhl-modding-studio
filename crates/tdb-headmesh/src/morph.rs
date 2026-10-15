//! Path B — donor shrinkwrap morph (Blender, via the `blender-job` crate).
//!
//! The donor's topology, UVs, and animated weights survive; only positions change. The
//! delicate parts are data, not code: per-donor tunables live in
//! `defs/appearance/morph-donors.toml` so adding a donor (or a donor for another target
//! title) never edits the algorithm. The non-negotiables learned in the prior effort —
//! boundary freeze, the eyelid-pocket freeze radii, recomputed normals — are named
//! fields with the proven defaults.
//!
//! M3 wires the actual job execution; this module owns the donor profiles and request
//! construction so the contract is testable Blender-free.

use serde::{Deserialize, Serialize};
use std::path::Path;

pub const DONORS_SCHEMA: &str = "headmesh-morph-donors/1";

/// Everything the wrap job needs to know about one donor head.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MorphDonor {
    /// Donor key (`player_67`, `player_71`, …).
    pub key: String,
    /// Matrix key of the target game this donor is native to.
    pub target: String,
    /// Expected `head_` vertex count — the `.pv` output must match exactly.
    pub head_vertex_count: usize,
    /// Left/right eyeball centers in shared model space (meters).
    pub eye_center_l: [f32; 3],
    pub eye_center_r: [f32; 3],
    /// Freeze ramp around each eyeball: verts within `eye_freeze_inner_m` don't move at
    /// all (the rigged eyelid pocket — collapsing it is unrecoverable); influence ramps
    /// to full morph at `eye_freeze_outer_m`.
    pub eye_freeze_inner_m: f32,
    pub eye_freeze_outer_m: f32,
    /// BFS rings from open boundary loops (neck/mouth) over which morph influence ramps
    /// 0 → 1.
    pub boundary_freeze_rings: u32,
    pub smooth_factor: f32,
    pub smooth_iterations: u32,
    /// Y band (meters) within which a target boundary loop counts as an eye hole and is
    /// filled before wrapping.
    pub eye_hole_y_range: [f32; 2],
    #[serde(default)]
    pub notes: String,
}

#[derive(Debug, Deserialize)]
struct DonorsFile {
    schema: String,
    #[serde(default, rename = "donor")]
    donors: Vec<MorphDonor>,
}

#[derive(Debug, thiserror::Error)]
pub enum MorphError {
    #[error("failed to read morph donors: {0}")]
    Io(#[from] std::io::Error),
    #[error("morph donors are not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unsupported morph-donors schema {found:?} (expected {DONORS_SCHEMA:?})")]
    Schema { found: String },
    #[error("no morph donor registered for target {0:?}")]
    NoDonorForTarget(String),
}

#[derive(Debug, Clone, Default)]
pub struct MorphDonors {
    pub donors: Vec<MorphDonor>,
}

impl MorphDonors {
    pub fn load(path: &Path) -> Result<MorphDonors, MorphError> {
        Self::from_toml_str(&std::fs::read_to_string(path)?)
    }

    pub fn from_toml_str(text: &str) -> Result<MorphDonors, MorphError> {
        let file: DonorsFile = toml::from_str(text)?;
        if file.schema != DONORS_SCHEMA {
            return Err(MorphError::Schema { found: file.schema });
        }
        Ok(MorphDonors {
            donors: file.donors,
        })
    }

    pub fn for_target(&self, target: &str) -> Result<&MorphDonor, MorphError> {
        self.donors
            .iter()
            .find(|d| d.target == target)
            .ok_or_else(|| MorphError::NoDonorForTarget(target.to_owned()))
    }

    pub fn by_key(&self, key: &str) -> Option<&MorphDonor> {
        self.donors.iter().find(|d| d.key == key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_donor_profiles() {
        let d = MorphDonors::from_toml_str(
            r#"
schema = "headmesh-morph-donors/1"

[[donor]]
key = "player_67"
target = "nhllegacy-xbox360"
head_vertex_count = 2623
eye_center_l = [0.034083, 1.835745, 0.090668]
eye_center_r = [-0.031242, 1.834532, 0.090979]
eye_freeze_inner_m = 0.016
eye_freeze_outer_m = 0.028
boundary_freeze_rings = 8
smooth_factor = 0.5
smooth_iterations = 6
eye_hole_y_range = [1.74, 1.92]
"#,
        )
        .unwrap();
        let donor = d.for_target("nhllegacy-xbox360").unwrap();
        assert_eq!(donor.key, "player_67");
        assert_eq!(donor.head_vertex_count, 2623);
        assert!(d.for_target("nhl16-ps4").is_err());
        assert!(d.by_key("player_67").is_some());
    }
}
