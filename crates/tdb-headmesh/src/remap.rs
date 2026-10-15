//! Path A — remap authored skin weights from a named source skeleton onto a target
//! bone palette.
//!
//! Resolution order per source bone (first hit wins):
//!
//! 1. **Override table** (`defs/appearance/bone-overrides.toml`) — the explicit human
//!    layer. The prior pipeline's single largest defect was *not having this*: every
//!    facial micro-bone ancestor-collapsed to Head, rigidifying ~330 weight mass of
//!    face. Known-correct redirects (Tongue → Jaw, arm chains → Spine3) live here.
//! 2. **Direct name match** into the target palette (exclusions already applied by
//!    [`crate::palette::NamedPalette::name_to_palette`]).
//! 3. **Ancestor climb** — walk the source parent chain to the nearest resolvable bone;
//!    recorded as `Collapsed { via }` so QA can report how much mass rode each collapse.
//! 4. **Drop** — mass on the bone is discarded (tracked, never silent).
//!
//! Per vertex: bucket weights by resolved palette slot, keep the strongest
//! `max_influences` (target-format budget; RW = 4 × u8), renormalize to 1.

use serde::Deserialize;
use std::collections::HashMap;
use std::path::Path;

use crate::palette::NamedPalette;

/// A named source skeleton (bone names + parent links), e.g. the NHL24 433-bone rig.
#[derive(Debug, Clone)]
pub struct SourceSkeleton {
    pub names: Vec<String>,
    /// Parallel to `names`; `None` for roots.
    pub parents: Vec<Option<u32>>,
}

/// How one source bone resolved into the target palette.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoneResolution {
    Override(u32),
    Direct(u32),
    /// Resolved through ancestor `via` (source bone index) to that ancestor's slot.
    Collapsed {
        via: u32,
        palette: u32,
    },
    Unmapped,
}

impl BoneResolution {
    pub fn palette(&self) -> Option<u32> {
        match *self {
            BoneResolution::Override(p)
            | BoneResolution::Direct(p)
            | BoneResolution::Collapsed { palette: p, .. } => Some(p),
            BoneResolution::Unmapped => None,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
struct OverrideEntry {
    source: String,
    target: String,
    #[serde(default)]
    #[allow(dead_code)]
    reason: String,
}

#[derive(Debug, Deserialize)]
struct OverridesFile {
    schema: String,
    #[serde(default, rename = "override")]
    overrides: Vec<OverrideEntry>,
}

pub const OVERRIDES_SCHEMA: &str = "headmesh-bone-overrides/1";

/// Source-bone-name → target-bone-name redirects (resolved through the palette
/// inversion, so they survive a regenerated palette naming).
#[derive(Debug, Clone, Default)]
pub struct BoneOverrides {
    pub by_source: HashMap<String, String>,
}

#[derive(Debug, thiserror::Error)]
pub enum RemapError {
    #[error("failed to read bone overrides: {0}")]
    Io(#[from] std::io::Error),
    #[error("bone overrides are not valid TOML: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("unsupported bone-overrides schema {found:?} (expected {OVERRIDES_SCHEMA:?})")]
    Schema { found: String },
    #[error("override target {target:?} (for source bone {source_bone:?}) is not a resolvable target-palette bone")]
    BadOverrideTarget { source_bone: String, target: String },
}

impl BoneOverrides {
    pub fn load(path: &Path) -> Result<BoneOverrides, RemapError> {
        Self::from_toml_str(&std::fs::read_to_string(path)?)
    }

    pub fn from_toml_str(text: &str) -> Result<BoneOverrides, RemapError> {
        let file: OverridesFile = toml::from_str(text)?;
        if file.schema != OVERRIDES_SCHEMA {
            return Err(RemapError::Schema { found: file.schema });
        }
        Ok(BoneOverrides {
            by_source: file
                .overrides
                .into_iter()
                .map(|o| (o.source, o.target))
                .collect(),
        })
    }
}

/// The resolved source-skeleton → target-palette mapping table.
#[derive(Debug, Clone)]
pub struct BoneMap {
    /// Parallel to the source skeleton's bones.
    pub resolutions: Vec<BoneResolution>,
}

impl BoneMap {
    /// Build the mapping table. Fails fast if an override names a target bone that the
    /// palette inversion cannot resolve (typo or excluded slot).
    pub fn build(
        skeleton: &SourceSkeleton,
        palette: &NamedPalette,
        overrides: &BoneOverrides,
    ) -> Result<BoneMap, RemapError> {
        let inversion = palette.name_to_palette();
        // First pass: overrides + direct matches.
        let mut resolutions: Vec<BoneResolution> = Vec::with_capacity(skeleton.names.len());
        for name in &skeleton.names {
            if let Some(target_name) = overrides.by_source.get(name) {
                let slot = inversion.get(target_name.as_str()).ok_or_else(|| {
                    RemapError::BadOverrideTarget {
                        source_bone: name.clone(),
                        target: target_name.clone(),
                    }
                })?;
                resolutions.push(BoneResolution::Override(*slot));
            } else if let Some(slot) = inversion.get(name.as_str()) {
                resolutions.push(BoneResolution::Direct(*slot));
            } else {
                resolutions.push(BoneResolution::Unmapped);
            }
        }
        // Second pass: ancestor climb for the still-unmapped.
        for i in 0..resolutions.len() {
            if resolutions[i] != BoneResolution::Unmapped {
                continue;
            }
            let mut cursor = skeleton.parents[i];
            while let Some(ancestor) = cursor {
                if let Some(palette) = resolutions[ancestor as usize].palette() {
                    resolutions[i] = BoneResolution::Collapsed {
                        via: ancestor,
                        palette,
                    };
                    break;
                }
                cursor = skeleton.parents[ancestor as usize];
            }
        }
        Ok(BoneMap { resolutions })
    }
}

/// Aggregate outcome numbers for a remap run (per submesh or whole asset).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RemapStats {
    /// Weight mass that reached the palette through an ancestor collapse.
    pub collapsed_mass: f64,
    /// Weight mass on unmapped bones, discarded.
    pub unmapped_mass: f64,
    /// Weight mass discarded by the influence-budget truncation (pre-renormalize).
    pub truncated_mass: f64,
    /// Vertices that lost at least one influence to truncation.
    pub truncated_vertices: usize,
    /// Vertices that ended with no influences at all (should be zero on a sane remap).
    pub unweighted_vertices: usize,
}

/// Remap one submesh's per-vertex named influences onto the target palette.
///
/// `vertices[i]` = list of (source bone index, weight). Returns per-vertex
/// (palette slot, weight) lists, strongest-first, truncated to `max_influences` and
/// renormalized to sum 1.
pub fn remap_vertices(
    vertices: &[Vec<(u32, f32)>],
    map: &BoneMap,
    max_influences: usize,
) -> (Vec<Vec<(u32, f32)>>, RemapStats) {
    let mut stats = RemapStats::default();
    let mut out = Vec::with_capacity(vertices.len());
    for influences in vertices {
        let mut buckets: HashMap<u32, f32> = HashMap::new();
        for &(bone, weight) in influences {
            if weight <= 0.0 {
                continue;
            }
            match map.resolutions.get(bone as usize) {
                Some(res) => match res.palette() {
                    Some(slot) => {
                        if matches!(res, BoneResolution::Collapsed { .. }) {
                            stats.collapsed_mass += f64::from(weight);
                        }
                        *buckets.entry(slot).or_insert(0.0) += weight;
                    }
                    None => stats.unmapped_mass += f64::from(weight),
                },
                None => stats.unmapped_mass += f64::from(weight),
            }
        }
        let mut ranked: Vec<(u32, f32)> = buckets.into_iter().collect();
        // Strongest first; slot index as deterministic tie-break.
        ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap().then(a.0.cmp(&b.0)));
        if ranked.len() > max_influences {
            for &(_, w) in &ranked[max_influences..] {
                stats.truncated_mass += f64::from(w);
            }
            ranked.truncate(max_influences);
            stats.truncated_vertices += 1;
        }
        let total: f32 = ranked.iter().map(|(_, w)| w).sum();
        if total > 0.0 {
            for (_, w) in &mut ranked {
                *w /= total;
            }
        } else {
            stats.unweighted_vertices += 1;
        }
        out.push(ranked);
    }
    (out, stats)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn palette() -> NamedPalette {
        NamedPalette::from_toml_str(
            r#"
[bones.4]
name = "Spine3"
dist = 0.001

[bones.5]
name = "Neck"
dist = 0.001
exclude = true

[bones.6]
name = "Neck"
dist = 0.002
prefer = true

[bones.8]
name = "Head"
dist = 0.001

[bones.16]
name = "Jaw"
dist = 0.001
"#,
        )
        .unwrap()
    }

    fn skeleton() -> SourceSkeleton {
        // 0 Spine3 → 1 Neck → 2 Head → 3 Jaw → 4 Tongue → 5 TongueTip
        //                        └→ 6 RM_BrowInner_L
        // 7 LeftArm (parent Spine3)
        SourceSkeleton {
            names: vec![
                "Spine3".into(),
                "Neck".into(),
                "Head".into(),
                "Jaw".into(),
                "Tongue".into(),
                "TongueTip".into(),
                "RM_BrowInner_L".into(),
                "LeftArm".into(),
            ],
            parents: vec![
                None,
                Some(0),
                Some(1),
                Some(2),
                Some(3),
                Some(4),
                Some(2),
                Some(0),
            ],
        }
    }

    fn overrides() -> BoneOverrides {
        BoneOverrides::from_toml_str(
            r#"
schema = "headmesh-bone-overrides/1"

[[override]]
source = "Tongue"
target = "Jaw"

[[override]]
source = "TongueTip"
target = "Jaw"

[[override]]
source = "LeftArm"
target = "Spine3"
"#,
        )
        .unwrap()
    }

    #[test]
    fn resolution_order_override_direct_ancestor_drop() {
        let map = BoneMap::build(&skeleton(), &palette(), &overrides()).unwrap();
        assert_eq!(map.resolutions[0], BoneResolution::Direct(4)); // Spine3
        assert_eq!(map.resolutions[1], BoneResolution::Direct(6)); // Neck → preferred slot, never excluded 5
        assert_eq!(map.resolutions[2], BoneResolution::Direct(8)); // Head
        assert_eq!(map.resolutions[4], BoneResolution::Override(16)); // Tongue → Jaw
        assert_eq!(map.resolutions[7], BoneResolution::Override(4)); // LeftArm → Spine3
                                                                     // Facial micro-bone with no direct match climbs to Head.
        assert_eq!(
            map.resolutions[6],
            BoneResolution::Collapsed { via: 2, palette: 8 }
        );
    }

    #[test]
    fn bad_override_target_fails_fast() {
        let bad = BoneOverrides::from_toml_str(
            r#"
schema = "headmesh-bone-overrides/1"

[[override]]
source = "Tongue"
target = "Jawbreaker"
"#,
        )
        .unwrap();
        assert!(matches!(
            BoneMap::build(&skeleton(), &palette(), &bad),
            Err(RemapError::BadOverrideTarget { .. })
        ));
    }

    #[test]
    fn vertex_remap_buckets_truncates_renormalizes() {
        let map = BoneMap::build(&skeleton(), &palette(), &overrides()).unwrap();
        // One vertex spreading mass over 5 bones that resolve to 3 slots after bucketing:
        // Head(8): 0.3 (Head) + 0.2 (brow collapse) = 0.5; Jaw(16): 0.2 + 0.1 = 0.3;
        // Neck(6): 0.2.
        let verts = vec![vec![(2, 0.3), (6, 0.2), (4, 0.2), (5, 0.1), (1, 0.2)]];
        let (out, stats) = remap_vertices(&verts, &map, 2);
        assert_eq!(out[0].len(), 2);
        assert_eq!(out[0][0].0, 8);
        assert_eq!(out[0][1].0, 16);
        let sum: f32 = out[0].iter().map(|(_, w)| w).sum();
        assert!((sum - 1.0).abs() < 1e-6);
        // 0.5 / (0.5 + 0.3) after dropping Neck's 0.2.
        assert!((out[0][0].1 - 0.625).abs() < 1e-6);
        assert!((stats.truncated_mass - 0.2).abs() < 1e-6);
        assert_eq!(stats.truncated_vertices, 1);
        assert!((stats.collapsed_mass - 0.2).abs() < 1e-6);
        assert_eq!(stats.unweighted_vertices, 0);
    }
}
