//! Extract stage — source container → [`HeadIr`].
//!
//! Two source families are implemented:
//!
//! * [`extract_nhl24_head`] — NHL24 Frosty GLB + `*.weights.json` sidecar (Path A).
//! * [`extract_cc5_head`] — Reallusion Character Creator 5 GLB (Path C). Self-describing:
//!   roles come from the `Std_*` material names and weights are inline, so there is no
//!   sidecar. See that function for the CC5-specific invariants.
//!
//! Original Path A note follows.
//!
//! Extract stage — NHL24 GLB + weights sidecar → [`HeadIr`].
//!
//! GLB primitives from Frosty exports carry no material names, so each primitive is
//! joined to its sidecar section **by vertex count**; the section's name (`mat_head` /
//! `mat_eyes` / `mat_eyelashes`) then drives role classification. Influences arrive in
//! skeleton bone-index space (named), positions are already in the shared model space
//! (NHL24 binds in the same frame as Legacy — the `identity` adapter of the matrix
//! row), and UVs stay [`UvConvention::GlSourced`] until pack.

use crate::glb::{self, GlbError};
use crate::ir::{HeadIr, IrSubmesh, NamedInfluences, Provenance, SubmeshRole, UvConvention};
use crate::sidecar::{Sidecar, SidecarError};

#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("failed to read GLB: {0}")]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Glb(#[from] GlbError),
    #[error(transparent)]
    Sidecar(#[from] SidecarError),
    #[error("GLB primitive {name:?} has {vertices} vertices but its sidecar section {section:?} carries {influences} influence rows")]
    InfluenceCountMismatch {
        name: String,
        section: String,
        vertices: usize,
        influences: usize,
    },
    #[error("no Head-role submesh found (sections seen: {0})")]
    NoHead(String),
    #[error("GLB carries {0} Head-role submeshes; expected exactly one (names: {1})")]
    AmbiguousHead(usize, String),
    #[error("GLB primitive {name:?} has no UVs — a head transplant needs the source atlas")]
    NoUvs { name: String },
    #[error(
        "GLB primitive {name:?} has {vertices} vertices but {weights} weight rows — \
         malformed skin"
    )]
    SkinCountMismatch {
        name: String,
        vertices: usize,
        weights: usize,
    },
}

/// How a CC5 source's bind pose related to the shared model space.
#[derive(Debug, Clone, PartialEq)]
pub struct Cc5BindReport {
    /// Joint node names, in glTF skin order.
    pub joint_names: Vec<String>,
    /// Largest absolute deviation of any joint's bind-time skinning matrix from identity.
    pub max_identity_deviation: f32,
    /// True when every joint's skinning matrix was identity within tolerance, so raw
    /// POSITION data was already in the shared model space and nothing was baked.
    pub positions_were_model_space: bool,
    /// True when the rig's joints collapse onto a single point, i.e. it carries no
    /// articulation. Authored weights then hold no retargetable information and Path C
    /// must source weights from the donor instead. False means the export ships a real
    /// skeleton and its authored weights should be remapped by name, the way Path A does.
    pub rig_is_rigid: bool,
}

/// Identity tolerance for CC5 bind matrices. Blender round-trips a 0.01 armature scale
/// against a 100× inverse-bind, so exact equality never holds; 1e-4 is far tighter than
/// any real transform and far looser than that float noise.
const BIND_IDENTITY_EPS: f32 = 1e-4;

/// Two joint origins closer than this count as the same point. A stub rig stacks its
/// joints within float noise of each other; real anatomy separates even neighbouring
/// facial bones by centimetres.
const JOINT_COINCIDENT_EPS_M: f32 = 1e-4;

fn identity_deviation(m: &[[f32; 4]; 4]) -> f32 {
    let mut worst = 0f32;
    for (r, row) in m.iter().enumerate() {
        for (c, cell) in row.iter().enumerate() {
            let want = if r == c { 1.0 } else { 0.0 };
            worst = worst.max((cell - want).abs());
        }
    }
    worst
}

/// Apply a row-major 4×4 to a point.
fn transform_point(m: &[[f32; 4]; 4], p: [f32; 3]) -> [f32; 3] {
    let mut o = [0f32; 3];
    for (r, slot) in o.iter_mut().enumerate() {
        *slot = m[r][0] * p[0] + m[r][1] * p[1] + m[r][2] * p[2] + m[r][3];
    }
    o
}

/// Linear-blend-skin a primitive into its bind pose, for exports whose vertex data is
/// not already in the shared model space.
pub fn bake_bind_pose(
    positions: &[[f32; 3]],
    joints: &[[u16; 4]],
    weights: &[[f32; 4]],
    bind_matrices: &[[[f32; 4]; 4]],
) -> Vec<[f32; 3]> {
    positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let (Some(j), Some(w)) = (joints.get(i), weights.get(i)) else {
                return *p;
            };
            let mut acc = [0f32; 3];
            let mut total = 0f32;
            for k in 0..4 {
                let weight = w[k];
                if weight <= 0.0 {
                    continue;
                }
                let Some(m) = bind_matrices.get(usize::from(j[k])) else {
                    continue;
                };
                let q = transform_point(m, *p);
                for c in 0..3 {
                    acc[c] += q[c] * weight;
                }
                total += weight;
            }
            if total <= 1e-6 {
                return *p;
            }
            for c in &mut acc {
                *c /= total;
            }
            acc
        })
        .collect()
}

/// Build the IR from a Reallusion **Character Creator 5** head GLB (Path C source).
///
/// CC5 exports are self-describing where NHL24's are not: primitives are named by their
/// `Std_*` material, so roles classify directly, and skin weights ride inline
/// `JOINTS_0`/`WEIGHTS_0` — there is no sidecar to join by vertex count.
///
/// Two CC5 invariants are checked rather than assumed:
///
/// 1. **Frame.** Blender's CC5 export cancels the armature's 0.01 scale against a 100×
///    inverse-bind, leaving every joint's bind-time skinning matrix at identity — so raw
///    POSITION data is already in the shared model space (meters, Y-up, +Z face) and the
///    matrix row's `identity` frame adapter is literally true. When that does not hold,
///    the primitive is skinned into its bind pose instead of trusting the raw stream.
/// 2. **Rig.** A head-only CC5 export often ships a stub skeleton — `hd_neutral_m` /
///    `rl_boneroot` / `cc_base_hip` stacked on one point — whose authored weights are
///    informationally empty and must not be retargeted; the caller sources weights from
///    the target donor instead. A full CC5 export ships the real 101-joint skeleton
///    (`CC_Base_Head`, `CC_Base_JawRoot`, `CC_Base_NeckTwist01`…), whose weights SHOULD be
///    remapped by name. [`Cc5BindReport::rig_is_rigid`] distinguishes the two.
///
/// Roles other than `Head` are carried through for measurement (eye centers, mouth
/// placement); the RW pack stage decides which of them reach the container.
pub fn extract_cc5_head(
    glb_bytes: &[u8],
    source_paths: Vec<String>,
) -> Result<(HeadIr, Cc5BindReport), ExtractError> {
    let mesh = glb::parse_glb(glb_bytes)?;

    let max_identity_deviation = mesh
        .joint_bind_matrices
        .iter()
        .map(identity_deviation)
        .fold(0f32, f32::max);
    let positions_were_model_space = max_identity_deviation <= BIND_IDENTITY_EPS;
    // Articulation is about the JOINTS occupying distinct places, not about the bind
    // matrices. At bind pose `globalJointTransform × inverseBindMatrix` is identity for
    // every joint by definition, so comparing bind matrices reports "rigid" for any
    // correctly-exported rig — it only gave the right answer on the 3-joint stub export by
    // coincidence. Count distinct joint origins instead: a stub rig collapses them onto one
    // point, a real rig does not.
    let rig_is_rigid = {
        let mut distinct: Vec<[f32; 3]> = Vec::new();
        for p in &mesh.joint_model_positions {
            if !distinct
                .iter()
                .any(|q| (0..3).all(|k| (q[k] - p[k]).abs() <= JOINT_COINCIDENT_EPS_M))
            {
                distinct.push(*p);
            }
        }
        distinct.len() < 2
    };

    let mut submeshes = Vec::new();
    let mut names = Vec::new();
    for prim in mesh.primitives {
        names.push(prim.name.clone());
        if prim.uvs.len() != prim.positions.len() {
            return Err(ExtractError::NoUvs { name: prim.name });
        }
        let skinned = !prim.joints.is_empty();
        if skinned
            && (prim.joints.len() != prim.positions.len()
                || prim.weights.len() != prim.positions.len())
        {
            return Err(ExtractError::SkinCountMismatch {
                name: prim.name,
                vertices: prim.positions.len(),
                weights: prim.weights.len().min(prim.joints.len()),
            });
        }
        let positions = if positions_were_model_space || !skinned {
            prim.positions
        } else {
            bake_bind_pose(
                &prim.positions,
                &prim.joints,
                &prim.weights,
                &mesh.joint_bind_matrices,
            )
        };
        // Carry the authored influences verbatim (named against the glTF joints). Path C
        // ignores them for a rigid rig, but a future articulated CC export retargets.
        let influences = skinned.then(|| NamedInfluences {
            vertices: prim
                .joints
                .iter()
                .zip(&prim.weights)
                .map(|(j, w)| {
                    (0..4)
                        .filter(|k| w[*k] > 0.0)
                        .map(|k| (u32::from(j[k]), w[k]))
                        .collect()
                })
                .collect(),
        });
        submeshes.push(IrSubmesh {
            name: prim.name.clone(),
            role: SubmeshRole::classify(&prim.name),
            positions,
            normals: Vec::new(), // recomputed at pack
            uvs: prim.uvs,
            // STORED, not GlSourced — and the distinction is per-exporter, not per-format.
            // glTF 2.0 puts the texture origin at the UPPER-left (§3.9.2), and Blender's
            // glTF I/O flips Blender's bottom-left V on the way out to comply. So a CC5
            // export's UVs already address its PNG atlas top-down, which is exactly the
            // console STORED convention: the correct number of flips is zero. (Path A's
            // NHL24 Frosty exports are the opposite case — bottom-left V, one flip — which
            // is why the convention travels with the data instead of being assumed from
            // the container.) Shipping GlSourced here renders the face upside down.
            uv_convention: UvConvention::Stored,
            triangles: prim.triangles,
            influences,
        });
    }

    let heads = submeshes
        .iter()
        .filter(|s| s.role == SubmeshRole::Head)
        .count();
    match heads {
        1 => {}
        0 => return Err(ExtractError::NoHead(names.join(", "))),
        n => return Err(ExtractError::AmbiguousHead(n, names.join(", "))),
    }

    let ir = HeadIr {
        submeshes,
        bone_names: mesh.joint_names.clone(),
        provenance: Provenance {
            source_game: "cc5".into(),
            source_paths,
            library_entry: None,
        },
    };
    Ok((
        ir,
        Cc5BindReport {
            joint_names: mesh.joint_names,
            max_identity_deviation,
            positions_were_model_space,
            rig_is_rigid,
        },
    ))
}

/// Build the IR from a component GLB + its authored-weights sidecar.
pub fn extract_nhl24_head(
    glb_bytes: &[u8],
    sidecar: &Sidecar,
    source_paths: Vec<String>,
) -> Result<HeadIr, ExtractError> {
    let mesh = glb::parse_glb(glb_bytes)?;
    let mut submeshes = Vec::new();
    let mut section_names = Vec::new();
    for prim in mesh.primitives {
        let section = sidecar.section_for_vertex_count(prim.positions.len())?;
        section_names.push(section.name.clone());
        let influences = section.skeleton_influences()?;
        if influences.len() != prim.positions.len() {
            return Err(ExtractError::InfluenceCountMismatch {
                name: prim.name.clone(),
                section: section.name.clone(),
                vertices: prim.positions.len(),
                influences: influences.len(),
            });
        }
        submeshes.push(IrSubmesh {
            name: section.name.clone(),
            role: SubmeshRole::classify(&section.name),
            positions: prim.positions,
            normals: Vec::new(), // recomputed at pack; GLB normals are not carried
            uvs: prim.uvs,
            uv_convention: UvConvention::GlSourced,
            triangles: prim.triangles,
            influences: Some(NamedInfluences {
                vertices: influences,
            }),
        });
    }
    let ir = HeadIr {
        submeshes,
        bone_names: sidecar.skeleton().names,
        provenance: Provenance {
            source_game: "nhl24".into(),
            source_paths,
            library_entry: None,
        },
    };
    if ir.head().is_none() {
        return Err(ExtractError::NoHead(section_names.join(", ")));
    }
    Ok(ir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::UvConvention;

    /// A minimal CC5-shaped GLB: one head primitive named by a `Std_*` material, inline
    /// skin weights, and an armature/inverse-bind pair that cancels to identity the way
    /// Blender's CC5 export does (0.01 scale against a 100× inverse-bind).
    fn cc5_glb(armature_scale: f32, inverse_bind_scale: f32) -> Vec<u8> {
        let mut bin: Vec<u8> = Vec::new();
        for p in [[0.0f32, 1.7, 0.1], [0.1, 1.8, 0.1], [-0.1, 1.8, 0.1]] {
            for c in p {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let uv_off = bin.len();
        // V values chosen so a flip is unmistakable: 0.25 → 0.75.
        for uv in [[0.0f32, 0.25], [1.0, 0.25], [0.5, 0.9]] {
            for c in uv {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let joints_off = bin.len();
        for j in [[0u8, 0, 0, 0]; 3] {
            bin.extend_from_slice(&j);
        }
        let weights_off = bin.len();
        for w in [[1.0f32, 0.0, 0.0, 0.0]; 3] {
            for c in w {
                bin.extend_from_slice(&c.to_le_bytes());
            }
        }
        let ibm_off = bin.len();
        // Column-major diagonal scale matrix.
        for k in 0..16 {
            let v = match k {
                0 | 5 | 10 => inverse_bind_scale,
                15 => 1.0,
                _ => 0.0,
            };
            bin.extend_from_slice(&v.to_le_bytes());
        }
        let idx_off = bin.len();
        for i in [0u16, 1, 2] {
            bin.extend_from_slice(&i.to_le_bytes());
        }
        while !bin.len().is_multiple_of(4) {
            bin.push(0);
        }

        let json = serde_json::json!({
            "asset": {"version": "2.0"},
            "materials": [{"name": "Std_Skin_Head.005"}],
            "meshes": [{
                "name": "0.005",
                "primitives": [{
                    "attributes": {"POSITION": 0, "TEXCOORD_0": 1, "JOINTS_0": 2, "WEIGHTS_0": 3},
                    "indices": 5,
                    "material": 0
                }]
            }],
            "nodes": [
                {"name": "hd_neutral_m"},
                {"name": "Armature", "children": [0], "scale": [armature_scale, armature_scale, armature_scale]}
            ],
            "skins": [{"joints": [0], "inverseBindMatrices": 4}],
            "accessors": [
                {"bufferView": 0, "componentType": 5126, "count": 3, "type": "VEC3"},
                {"bufferView": 1, "componentType": 5126, "count": 3, "type": "VEC2"},
                {"bufferView": 2, "componentType": 5121, "count": 3, "type": "VEC4"},
                {"bufferView": 3, "componentType": 5126, "count": 3, "type": "VEC4"},
                {"bufferView": 4, "componentType": 5126, "count": 1, "type": "MAT4"},
                {"bufferView": 5, "componentType": 5123, "count": 3, "type": "SCALAR"}
            ],
            "bufferViews": [
                {"buffer": 0, "byteOffset": 0, "byteLength": uv_off},
                {"buffer": 0, "byteOffset": uv_off, "byteLength": joints_off - uv_off},
                {"buffer": 0, "byteOffset": joints_off, "byteLength": weights_off - joints_off},
                {"buffer": 0, "byteOffset": weights_off, "byteLength": ibm_off - weights_off},
                {"buffer": 0, "byteOffset": ibm_off, "byteLength": idx_off - ibm_off},
                {"buffer": 0, "byteOffset": idx_off, "byteLength": 6}
            ],
            "buffers": [{"byteLength": bin.len()}]
        });
        let mut json_bytes = serde_json::to_vec(&json).unwrap();
        while !json_bytes.len().is_multiple_of(4) {
            json_bytes.push(b' ');
        }
        let mut glb = Vec::new();
        glb.extend_from_slice(b"glTF");
        glb.extend_from_slice(&2u32.to_le_bytes());
        glb.extend_from_slice(&((12 + 8 + json_bytes.len() + 8 + bin.len()) as u32).to_le_bytes());
        glb.extend_from_slice(&(json_bytes.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"JSON");
        glb.extend_from_slice(&json_bytes);
        glb.extend_from_slice(&(bin.len() as u32).to_le_bytes());
        glb.extend_from_slice(b"BIN\0");
        glb.extend_from_slice(&bin);
        glb
    }

    /// The bug the user hit in-game: shipping CC5 UVs as GlSourced flips V and renders the
    /// face upside down. glTF's origin is upper-left, so the correct flip count is zero.
    #[test]
    fn cc5_uvs_are_stored_convention_and_survive_pack_unflipped() {
        let (ir, _) = extract_cc5_head(&cc5_glb(0.01, 100.0), vec![]).unwrap();
        let head = ir.head().unwrap();
        assert_eq!(head.uv_convention, UvConvention::Stored);
        assert_eq!(head.uvs[0], [0.0, 0.25], "V must not be flipped at extract");
        // The typed transition is a no-op, which is the whole point of tracking it.
        let mut copy = head.clone();
        copy.into_uv_convention(UvConvention::Stored);
        assert_eq!(copy.uvs[0], [0.0, 0.25]);
    }

    #[test]
    fn identity_bind_is_detected_and_positions_pass_through() {
        let (ir, report) = extract_cc5_head(&cc5_glb(0.01, 100.0), vec![]).unwrap();
        assert!(report.positions_were_model_space, "{report:?}");
        assert!(report.rig_is_rigid, "a single joint is trivially rigid");
        assert!(report.max_identity_deviation < 1e-4);
        assert_eq!(ir.head().unwrap().positions[0], [0.0, 1.7, 0.1]);
    }

    #[test]
    fn non_identity_bind_is_baked_rather_than_trusted() {
        // Armature scale no longer cancelled by the inverse bind → skinning matrix is 2×.
        let (ir, report) = extract_cc5_head(&cc5_glb(0.02, 100.0), vec![]).unwrap();
        assert!(!report.positions_were_model_space, "{report:?}");
        let p = ir.head().unwrap().positions[0];
        assert!((p[1] - 3.4).abs() < 1e-4, "expected 2× bake, got {p:?}");
    }

    #[test]
    fn cc5_roles_come_from_material_names() {
        let (ir, _) = extract_cc5_head(&cc5_glb(0.01, 100.0), vec![]).unwrap();
        assert_eq!(ir.submeshes.len(), 1);
        assert_eq!(ir.submeshes[0].name, "Std_Skin_Head.005");
        assert_eq!(ir.submeshes[0].role, SubmeshRole::Head);
        assert_eq!(ir.bone_names, vec!["hd_neutral_m".to_string()]);
    }
}
