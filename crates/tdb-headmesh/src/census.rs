//! Donor-usage census — which palette slots a class of vanilla assets actually weights.
//!
//! The prior effort's hardest-won lesson: name matching is secondary to *usage*. Vanilla
//! Legacy heads reference only 37 of their 122 palette slots; sending remapped mass to an
//! unused slot means multiplying vertices by a garbage matrix (the palette-5 Neck bug —
//! heavy distortion in-game). The census is a rerunnable stage: run it over a corpus of
//! vanilla assets for the target (title, platform, asset class), and every remap target
//! must be inside the used set. The `exclude` flags in the palette defs are validated
//! against — never a substitute for — a fresh census.

use std::collections::BTreeMap;

use tdb_rx2::playermesh::PlayerAsset;

/// Aggregated weight mass per palette slot.
#[derive(Debug, Clone, Default)]
pub struct Census {
    /// palette index → total weight mass (sum of vertex weights, so comparable across
    /// meshes of similar vertex count).
    pub mass: BTreeMap<u32, f64>,
    /// Number of assets folded in.
    pub assets: usize,
    /// Submesh names skipped because they carry no skin attributes (rigid eyeballs etc.).
    pub skipped_rigid: Vec<String>,
}

impl Census {
    /// Fold one parsed vanilla asset into the census. Joint values are raw BLENDINDICES
    /// (shader-register indices, palette × 3).
    pub fn add_asset(&mut self, asset: &PlayerAsset) {
        for sm in &asset.submeshes {
            if !sm.has_blend_indices || !sm.has_blend_weights {
                self.skipped_rigid.push(sm.name.clone());
                continue;
            }
            for v in &sm.vertices {
                for (j, w) in v.joints.iter().zip(v.weights.iter()) {
                    if *w <= 0.0 {
                        continue;
                    }
                    let palette = u32::from(*j) / 3;
                    *self.mass.entry(palette).or_insert(0.0) += f64::from(*w);
                }
            }
        }
        self.assets += 1;
    }

    /// Palette slots carrying at least `min_mass` total weight.
    pub fn used_slots(&self, min_mass: f64) -> Vec<u32> {
        self.mass
            .iter()
            .filter(|(_, m)| **m >= min_mass)
            .map(|(idx, _)| *idx)
            .collect()
    }

    /// Excluded slots (per the palette defs) that the census nevertheless sees real mass
    /// on — an exclusion list that disagrees with vanilla usage is wrong and must be
    /// re-derived, not overridden.
    pub fn contested_exclusions(
        &self,
        palette: &crate::palette::NamedPalette,
        min_mass: f64,
    ) -> Vec<u32> {
        self.used_slots(min_mass)
            .into_iter()
            .filter(|idx| palette.get(*idx).is_some_and(|b| b.exclude))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tdb_rx2::playermesh::{BonePalette, PositionFormat, Submesh, UvFormat, Vertex};

    fn vert(joints: [u16; 4], weights: [f32; 4]) -> Vertex {
        Vertex {
            position: [0.0; 3],
            normal: [0.0; 3],
            uv: [0.0; 2],
            joints,
            weights,
        }
    }

    #[test]
    fn census_divides_registers_and_skips_rigid() {
        let asset = PlayerAsset {
            submeshes: vec![
                Submesh {
                    name: "head_".into(),
                    stride: 32,
                    position_format: PositionFormat::HalfFloat,
                    uv_format: UvFormat::UShort,
                    has_blend_indices: true,
                    has_blend_weights: true,
                    vertices: vec![
                        vert([24, 48, 0, 0], [0.5, 0.5, 0.0, 0.0]),
                        vert([24, 0, 0, 0], [1.0, 0.0, 0.0, 0.0]),
                    ],
                    triangles: vec![],
                },
                Submesh {
                    name: "eyel_".into(),
                    stride: 32,
                    position_format: PositionFormat::HalfFloat,
                    uv_format: UvFormat::UShort,
                    has_blend_indices: false,
                    has_blend_weights: false,
                    vertices: vec![vert([0; 4], [1.0, 0.0, 0.0, 0.0])],
                    triangles: vec![],
                },
            ],
            skin: BonePalette { matrices: vec![] },
        };
        let mut c = Census::default();
        c.add_asset(&asset);
        // register 24 → palette 8 (Head), register 48 → palette 16 (Jaw).
        assert_eq!(c.mass[&8], 1.5);
        assert_eq!(c.mass[&16], 0.5);
        assert_eq!(c.used_slots(0.1), vec![8, 16]);
        assert_eq!(c.skipped_rigid, vec!["eyel_".to_string()]);
    }
}
