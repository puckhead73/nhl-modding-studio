//! Intermediate representation shared by every conversion stage.
//!
//! All positions live in the **shared model space**: meters, standing pose, Y-up, face
//! toward +Z (Legacy head occupies roughly X ∈ [-0.14, 0.14], Y ∈ [1.54, 1.94],
//! Z ∈ [-0.12, 0.14]; NHL24 rigs bind in the same frame). Sources whose native frame
//! differs must pass through a [`crate::frames::FrameAdapter`] before entering the IR —
//! nothing downstream re-checks the frame.
//!
//! Skin weights are kept against **bone names**, never palette indices: palette layouts
//! are a per-target-format encoding concern that only [`crate::remap`] may introduce.

use serde::{Deserialize, Serialize};

/// Which UV storage convention a UV set is currently in.
///
/// The RX2 packer consumes STORED convention (Xbox, V origin top-left); OBJ/glTF sources
/// arrive GL-sourced (V origin bottom-left) and need exactly one `[u, 1-v]` flip. The
/// prior pipeline double-flipped donor UVs when the flip was implicit — here the
/// convention travels with the data and the flip is a typed transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UvConvention {
    /// On-disk console convention (what `tdb_rx2::pack` expects).
    Stored,
    /// glTF / OBJ convention (V flipped relative to `Stored`).
    GlSourced,
}

/// Anatomical role of a submesh, driving per-role policy (texture binding, graft slots,
/// which submeshes a morph may touch, what the eye rule protects).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmeshRole {
    /// The face/head surface (`head_`, `mat_head`, `geo_headShape`, …).
    Head,
    /// Hair geometry: cards or shell (`cards*`, `hair_cards*`).
    Hair,
    /// Scalp cap under hair (`haircap_`).
    Haircap,
    /// Eyeballs (`eyel_` / `eyer_` / `geo_leyeShape`…). UV into the shared eye raster on
    /// RW targets; kept verbatim from the target donor, never morphed or remapped.
    Eyes,
    /// Eyelashes (`mat_eyelashes`). No render slot on RW targets — dropped or grafted
    /// explicitly, never silently carried.
    Lashes,
    /// Interior mouth bag (`mouthbag_`).
    Mouthbag,
    /// Separate teeth geometry (CC5 `Std_Upper_Teeth` / `Std_Lower_Teeth`). RW targets
    /// have no teeth slot — the donor `mouthbag_` paints them — so these are dropped.
    Teeth,
    /// Separate tongue geometry (CC5 `Std_Tongue`). Same story as [`Self::Teeth`].
    Tongue,
    /// Anything not yet classified; conversion refuses rather than guessing a policy.
    Unknown,
}

impl SubmeshRole {
    /// Classify a submesh by its source name, across the naming families seen so far
    /// (RW `head_`/`haircap_`/`eyel_`, NHL24 `mat_*`, PS4 Maya `geo_*Shape`,
    /// Reallusion CC5 `Std_*` material names).
    pub fn classify(name: &str) -> SubmeshRole {
        let n = name.to_ascii_lowercase();
        if n.contains("lash") {
            SubmeshRole::Lashes
        } else if n.contains("mouthbag") || n.contains("mouth_bag") {
            SubmeshRole::Mouthbag
        } else if n.contains("teeth") {
            SubmeshRole::Teeth
        } else if n.contains("tongue") {
            SubmeshRole::Tongue
        } else if n.contains("eye") || n.contains("cornea") {
            SubmeshRole::Eyes
        } else if n.contains("haircap") {
            SubmeshRole::Haircap
        } else if n.contains("hair") || n.contains("cards") {
            SubmeshRole::Hair
        } else if n.contains("head") {
            SubmeshRole::Head
        } else {
            SubmeshRole::Unknown
        }
    }
}

/// Per-vertex skin influences against **named** bones.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NamedInfluences {
    /// Parallel to the owning submesh's vertices; each entry is (bone name index into
    /// [`HeadIr::bone_names`], weight). Weights need not be normalized here — the remap
    /// stage renormalizes after truncation to the target's influence budget.
    pub vertices: Vec<Vec<(u32, f32)>>,
}

/// One submesh of a head asset in shared model space.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IrSubmesh {
    /// Source name, kept verbatim for provenance and role re-checks.
    pub name: String,
    pub role: SubmeshRole,
    /// XYZ triples, meters, shared model space.
    pub positions: Vec<[f32; 3]>,
    /// May be empty (recomputed downstream — flat normals band the in-game shading).
    pub normals: Vec<[f32; 3]>,
    pub uvs: Vec<[f32; 2]>,
    pub uv_convention: UvConvention,
    /// Triangle list (not strips), indices into this submesh's vertices.
    pub triangles: Vec<[u32; 3]>,
    /// Named skin weights; `None` for bare geometry (Path B sources).
    pub influences: Option<NamedInfluences>,
}

/// Where an IR came from — carried through to install manifests and QA reports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Provenance {
    /// Matrix key of the source game/corpus (e.g. `nhl24`, `nhllegacy-xbox360`).
    pub source_game: String,
    /// Files this IR was extracted from.
    pub source_paths: Vec<String>,
    /// Library entry id when the source came from a registered head library.
    pub library_entry: Option<String>,
}

/// A head asset decoupled from any container format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeadIr {
    pub submeshes: Vec<IrSubmesh>,
    /// Bone-name table referenced by [`NamedInfluences`]; empty for unrigged sources.
    pub bone_names: Vec<String>,
    pub provenance: Provenance,
}

impl HeadIr {
    /// The single `Head`-role submesh, if unambiguous.
    pub fn head(&self) -> Option<&IrSubmesh> {
        let mut heads = self
            .submeshes
            .iter()
            .filter(|s| s.role == SubmeshRole::Head);
        match (heads.next(), heads.next()) {
            (Some(h), None) => Some(h),
            _ => None,
        }
    }
}

impl IrSubmesh {
    /// Flip this submesh's UVs into the requested convention (no-op when already there).
    pub fn into_uv_convention(&mut self, target: UvConvention) {
        if self.uv_convention != target {
            for uv in &mut self.uvs {
                uv[1] = 1.0 - uv[1];
            }
            self.uv_convention = target;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_classification_covers_known_families() {
        // RW Legacy names
        assert_eq!(SubmeshRole::classify("head_"), SubmeshRole::Head);
        assert_eq!(SubmeshRole::classify("haircap_"), SubmeshRole::Haircap);
        assert_eq!(SubmeshRole::classify("eyel_"), SubmeshRole::Eyes);
        assert_eq!(SubmeshRole::classify("mouthbag_"), SubmeshRole::Mouthbag);
        assert_eq!(SubmeshRole::classify("cardsA_"), SubmeshRole::Hair);
        // NHL24 GLB prim names
        assert_eq!(SubmeshRole::classify("mat_head"), SubmeshRole::Head);
        assert_eq!(SubmeshRole::classify("mat_eyes"), SubmeshRole::Eyes);
        assert_eq!(SubmeshRole::classify("mat_eyelashes"), SubmeshRole::Lashes);
        // PS4 Maya shape names
        assert_eq!(SubmeshRole::classify("geo_headShape"), SubmeshRole::Head);
        assert_eq!(SubmeshRole::classify("geo_leyeShape"), SubmeshRole::Eyes);
        assert_eq!(SubmeshRole::classify("hair_cardsShape"), SubmeshRole::Hair);
        assert_eq!(
            SubmeshRole::classify("geo_haircapShape"),
            SubmeshRole::Haircap
        );
        // eyelash beats eye
        assert_eq!(
            SubmeshRole::classify("geo_eyelashShape"),
            SubmeshRole::Lashes
        );
    }

    #[test]
    fn role_classification_covers_cc5_material_names() {
        assert_eq!(
            SubmeshRole::classify("Std_Skin_Head.005"),
            SubmeshRole::Head
        );
        assert_eq!(SubmeshRole::classify("Std_Eye_L.005"), SubmeshRole::Eyes);
        assert_eq!(SubmeshRole::classify("Std_Eye_R.005"), SubmeshRole::Eyes);
        // Cornea carries no "eye" substring — it needs its own arm.
        assert_eq!(SubmeshRole::classify("Std_Cornea_L.005"), SubmeshRole::Eyes);
        assert_eq!(
            SubmeshRole::classify("Std_Eyelash.005"),
            SubmeshRole::Lashes
        );
        assert_eq!(
            SubmeshRole::classify("Std_Upper_Teeth.005"),
            SubmeshRole::Teeth
        );
        assert_eq!(
            SubmeshRole::classify("Std_Lower_Teeth.005"),
            SubmeshRole::Teeth
        );
        assert_eq!(SubmeshRole::classify("Std_Tongue.005"), SubmeshRole::Tongue);
    }

    #[test]
    fn uv_flip_is_an_involution_and_tracks_convention() {
        let mut s = IrSubmesh {
            name: "head_".into(),
            role: SubmeshRole::Head,
            positions: vec![],
            normals: vec![],
            uvs: vec![[0.25, 0.1]],
            uv_convention: UvConvention::GlSourced,
            triangles: vec![],
            influences: None,
        };
        s.into_uv_convention(UvConvention::Stored);
        assert_eq!(s.uvs[0], [0.25, 0.9]);
        assert_eq!(s.uv_convention, UvConvention::Stored);
        // Already there → no double flip (the prior pipeline's v1 bug).
        s.into_uv_convention(UvConvention::Stored);
        assert_eq!(s.uvs[0], [0.25, 0.9]);
        s.into_uv_convention(UvConvention::GlSourced);
        assert!((s.uvs[0][1] - 0.1).abs() < 1e-6);
    }
}
