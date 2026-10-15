//! NHL24 Frosty `*.weights.json` sidecar — the authored-weights source for Path A.
//!
//! Produced by the Frosty GltfSkinnedExport plugin (C#, camelCase JSON): the full named
//! 433-bone skeleton (index / name / parent / model-space translation) plus, per mesh
//! section, the authored per-vertex influences in **local bone-list space**. On NHL24
//! head components the `boneList` is the identity mapping over the whole skeleton, but
//! the indirection is applied anyway — and indices are masked `& 0x7FFF` (the exporter
//! flags some indices via the high bit; `highBitIndices` records them).
//!
//! Verified against `head_aaron_ekblad_mesh.weights.json` (433 bones; sections
//! `mat_eyes` 198 v / `mat_head` 3157 v / `mat_eyelashes` 64 v, `lod` 0).

use serde::Deserialize;
use std::path::Path;

use crate::remap::SourceSkeleton;

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarBone {
    pub index: u32,
    pub name: String,
    /// `-1` for the root.
    pub parent: i64,
    /// Model-space translation (meters, shared model space). `[0,0,0]` on
    /// non-anatomical helper bones.
    #[serde(default)]
    pub model_trans: [f32; 3],
}

#[derive(Debug, Clone, Deserialize)]
pub struct VertexInfluences {
    /// Local bone indices (into the section's `boneList`), high bit masked off.
    pub i: Vec<u16>,
    /// Parallel weights.
    pub w: Vec<f32>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SidecarSection {
    pub name: String,
    #[serde(default)]
    pub lod: u32,
    pub vertex_count: usize,
    /// Local → skeleton bone-index mapping (identity on NHL24 heads).
    #[serde(default)]
    pub bone_list: Vec<u16>,
    pub vertices_local: Vec<VertexInfluences>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Sidecar {
    #[serde(default)]
    pub asset: String,
    #[serde(default)]
    pub bone_count: usize,
    pub bones: Vec<SidecarBone>,
    pub sections: Vec<SidecarSection>,
}

#[derive(Debug, thiserror::Error)]
pub enum SidecarError {
    #[error("failed to read weights sidecar: {0}")]
    Io(#[from] std::io::Error),
    #[error("weights sidecar is not valid JSON: {0}")]
    Json(#[from] serde_json::Error),
    #[error("sidecar bone indices are not dense 0..{expected} (bone {index} out of place)")]
    SparseBones { expected: usize, index: u32 },
    #[error("no lod-0 section with {0} vertices in the sidecar")]
    NoSectionForCount(usize),
    #[error("section {name:?} vertex {vertex}: influence index {index} outside the bone list")]
    BadInfluenceIndex {
        name: String,
        vertex: usize,
        index: u16,
    },
}

impl Sidecar {
    pub fn load(path: &Path) -> Result<Sidecar, SidecarError> {
        let text = std::fs::read_to_string(path)?;
        let sidecar: Sidecar = serde_json::from_str(&text)?;
        sidecar.validate()?;
        Ok(sidecar)
    }

    pub fn from_json_str(text: &str) -> Result<Sidecar, SidecarError> {
        let sidecar: Sidecar = serde_json::from_str(text)?;
        sidecar.validate()?;
        Ok(sidecar)
    }

    fn validate(&self) -> Result<(), SidecarError> {
        for (pos, bone) in self.bones.iter().enumerate() {
            if bone.index as usize != pos {
                return Err(SidecarError::SparseBones {
                    expected: self.bones.len(),
                    index: bone.index,
                });
            }
        }
        Ok(())
    }

    /// The named source skeleton for remapping.
    pub fn skeleton(&self) -> SourceSkeleton {
        SourceSkeleton {
            names: self.bones.iter().map(|b| b.name.clone()).collect(),
            parents: self
                .bones
                .iter()
                .map(|b| u32::try_from(b.parent).ok())
                .collect(),
        }
    }

    /// Model-space bind translations, parallel to the skeleton (for the QA battery's
    /// rotation centers).
    pub fn bind_translations(&self) -> Vec<[f32; 3]> {
        self.bones.iter().map(|b| b.model_trans).collect()
    }

    /// Find the lod-0 section matching a GLB primitive by vertex count (the GLB prims
    /// carry no material names, so count is the join key — the prior pipeline's rule).
    pub fn section_for_vertex_count(&self, count: usize) -> Result<&SidecarSection, SidecarError> {
        self.sections
            .iter()
            .find(|s| s.lod == 0 && s.vertex_count == count)
            .ok_or(SidecarError::NoSectionForCount(count))
    }
}

impl SidecarSection {
    /// Per-vertex influences in **skeleton bone-index space**: local indices routed
    /// through `boneList` (identity when empty), high bit masked.
    pub fn skeleton_influences(&self) -> Result<Vec<Vec<(u32, f32)>>, SidecarError> {
        let mut out = Vec::with_capacity(self.vertices_local.len());
        for (vi, v) in self.vertices_local.iter().enumerate() {
            let mut list = Vec::with_capacity(v.i.len());
            for (raw, &w) in v.i.iter().zip(v.w.iter()) {
                let local = raw & 0x7FFF;
                let skeleton_index = if self.bone_list.is_empty() {
                    u32::from(local)
                } else {
                    let mapped = self.bone_list.get(local as usize).ok_or_else(|| {
                        SidecarError::BadInfluenceIndex {
                            name: self.name.clone(),
                            vertex: vi,
                            index: local,
                        }
                    })?;
                    u32::from(*mapped & 0x7FFF)
                };
                if w > 0.0 {
                    list.push((skeleton_index, w));
                }
            }
            out.push(list);
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
        "asset": "head_test_mesh",
        "boneCount": 4,
        "bones": [
            {"index": 0, "name": "Reference", "parent": -1, "modelTrans": [0, 0, 0]},
            {"index": 1, "name": "Hips", "parent": 0, "modelTrans": [0, 1.0886, 0.0139]},
            {"index": 2, "name": "Head", "parent": 1, "modelTrans": [0, 1.781, 0.01]},
            {"index": 3, "name": "Jaw", "parent": 2, "modelTrans": [0, 1.72, 0.05]}
        ],
        "sections": [
            {"name": "mat_head", "lod": 0, "vertexCount": 2,
             "boneList": [0, 1, 2, 3],
             "verticesLocal": [
                {"i": [2], "w": [1.0]},
                {"i": [2, 32771], "w": [0.75, 0.25]}
             ]},
            {"name": "mat_head", "lod": 1, "vertexCount": 1,
             "boneList": [], "verticesLocal": [{"i": [0], "w": [1.0]}]}
        ]
    }"#;

    #[test]
    fn parses_and_routes_influences_to_skeleton_space() {
        let sc = Sidecar::from_json_str(SAMPLE).unwrap();
        let skel = sc.skeleton();
        assert_eq!(skel.names.len(), 4);
        assert_eq!(skel.parents[0], None);
        assert_eq!(skel.parents[3], Some(2));
        assert_eq!(sc.bind_translations()[2][1], 1.781);

        let section = sc.section_for_vertex_count(2).unwrap();
        assert_eq!(section.lod, 0);
        let infl = section.skeleton_influences().unwrap();
        assert_eq!(infl[0], vec![(2, 1.0)]);
        // 32771 = 0x8003 → high bit masked → local 3 → skeleton 3 (Jaw).
        assert_eq!(infl[1], vec![(2, 0.75), (3, 0.25)]);

        assert!(sc.section_for_vertex_count(99).is_err());
    }

    #[test]
    fn sparse_bone_table_is_rejected() {
        let bad = SAMPLE.replace(r#""index": 3"#, r#""index": 7"#);
        assert!(matches!(
            Sidecar::from_json_str(&bad),
            Err(SidecarError::SparseBones { .. })
        ));
    }
}
