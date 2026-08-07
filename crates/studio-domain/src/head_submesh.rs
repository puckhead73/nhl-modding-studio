//! **Submesh classification** for the appearance catalog (Phase 2).
//!
//! A decoded head carries several named submeshes. To export the three catalog
//! assets — the full head, the hair, and (where it exists) the facial hair — we
//! need to know which submesh is which. This module is the single source of that
//! decision, grounded in the census submesh vocabulary
//! (`docs/audits/head-hair-census.md`) across both container families:
//!
//! * **RenderWare `.rx2`** — `head_`, `head__1`, `eyel_`, `eyer_`, `mouthbag_`
//!   (head-core); `haircap_`, `cardsa_`, `cardsb_`, `hair_`, `haira_`, `hairb_`
//!   (hair).
//! * **PS4 `.mcd`** (`geo_*Shape`) — `geo_headShape`, `geo_head_lowShape`,
//!   `geo_leyeShape`, `geo_reyeShape`, `geo_eyesShape`, `geo_eyelashesShape`
//!   (head-core); `geo_haircapShape`, `geo_cardsaShape`, `geo_cardsbShape` and the
//!   gear-context variants `geo_headgear_*`, `geo_hat_*`, `geo_toque_*` (hair).
//!
//! **Facial hair.** The census found no beard/moustache submesh on any vanilla
//! head — vanilla facial hair is painted into the head texture, so it is captured
//! as a texture entry, not geometry (the agreed decision). The one geometry
//! candidate is PS4's `geo_facegear_*` (hair-primitive `haircap`/`cards` geometry
//! sitting in the *face* region, appearing with/without hats — `geo_hat_facegear_*`,
//! `geo_toque_facegear_*`), which is classified here as facial hair. This is a
//! flagged heuristic pending the targeted `facegear` probe; everything else with a
//! hair primitive but no face marker is scalp hair.

use serde::{Deserialize, Serialize};

/// Which catalog asset a submesh belongs to.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SubmeshPart {
    /// Head, eyes, eyelashes, mouthbag, teeth/tongue — the base head. Kept in the
    /// full-head asset only.
    HeadCore,
    /// Scalp hair: haircaps and hair cards (incl. head/hat/toque-context variants).
    /// Goes into both the full head and the merged hair asset.
    Hair,
    /// Facial hair rendered as geometry (PS4 `facegear` cards, or an explicit
    /// beard/moustache submesh). Goes into the full head and the facial-hair asset.
    FacialHair,
    /// Body geometry bundled with a head — the coach/referee/trainer full models
    /// carry jersey/suit/hands/skates in the same container as the head. Excluded
    /// from every head asset (Phase 4 staff extraction). Player head meshes never
    /// carry these, so their body bucket is always empty.
    Body,
}

/// Face-region markers: a hair primitive carrying one of these is facial hair,
/// not scalp hair. `facegear` is the PS4 token; the rest are explicit names in
/// case a title (or a grafted/imported head) ships a beard mesh outright.
const FACE_MARKERS: &[&str] = &[
    "facegear",
    "beard",
    "moustache",
    "mustache",
    "goatee",
    "stubble",
    "sideburn",
    "facialhair",
];

/// Hair-primitive markers. `haircap`/`cards`/`hair` cover both families; the
/// `head`-prefixed `headgear` is intentionally matched by `hair` only via its own
/// `haircap`/`cards` suffix, never by the bare `head` head-core token.
const HAIR_MARKERS: &[&str] = &["haircap", "hair", "cards"];

/// Body-part markers found in staff full models (coach/referee/trainer) alongside
/// the head — jersey/suit/hands/skates/whistle. None appear in a player head mesh.
const BODY_MARKERS: &[&str] = &[
    "jersey", "pant", "suit", "hand", "skate", "whistle", "boot",
];

/// Classify one submesh by name. Case-insensitive; works on `.rx2` (`haircap_.Mesh`)
/// and PS4 (`geo_haircapShape`) naming alike.
pub fn classify_submesh(name: &str) -> SubmeshPart {
    let n = name.to_ascii_lowercase();
    // Facial hair first: `geo_facegear_haircapShape` carries both a face marker and
    // a hair primitive, and the face marker wins.
    if FACE_MARKERS.iter().any(|m| n.contains(m)) {
        return SubmeshPart::FacialHair;
    }
    if HAIR_MARKERS.iter().any(|m| n.contains(m)) {
        return SubmeshPart::Hair;
    }
    if BODY_MARKERS.iter().any(|m| n.contains(m)) {
        return SubmeshPart::Body;
    }
    SubmeshPart::HeadCore
}

/// Submesh indices grouped by catalog asset. Indices point into the decoded
/// asset's `submeshes` in original order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SubmeshPartition {
    pub head_core: Vec<usize>,
    pub hair: Vec<usize>,
    pub facial_hair: Vec<usize>,
    /// Body submeshes to exclude from head assets (staff full models only).
    pub body: Vec<usize>,
}

impl SubmeshPartition {
    /// True when there is any scalp-hair geometry to export as a hair asset.
    pub fn has_hair(&self) -> bool {
        !self.hair.is_empty()
    }
    /// True when distinct facial-hair geometry exists (rare — usually facial hair
    /// is a texture, so this is normally empty and the facet falls back to a
    /// texture entry).
    pub fn has_facial_hair(&self) -> bool {
        !self.facial_hair.is_empty()
    }
}

/// Partition a decoded head's submeshes into head-core / hair / facial-hair by
/// name. The full-head asset is every index; the hair asset is
/// [`SubmeshPartition::hair`] (all haircaps + cards merged, so a head with both
/// `haircap_a` and `haircap_b` yields one hair mesh); the facial-hair asset is
/// [`SubmeshPartition::facial_hair`].
pub fn partition(asset: &tdb_rx2::playermesh::PlayerAsset) -> SubmeshPartition {
    let mut out = SubmeshPartition::default();
    for (i, sm) in asset.submeshes.iter().enumerate() {
        match classify_submesh(&sm.name) {
            SubmeshPart::HeadCore => out.head_core.push(i),
            SubmeshPart::Hair => out.hair.push(i),
            SubmeshPart::FacialHair => out.facial_hair.push(i),
            SubmeshPart::Body => out.body.push(i),
        }
    }
    out
}

/// Submesh indices that make up the **head asset** (everything except body):
/// head-core + hair + facial hair, in original order. This is the full head an
/// operator exports — for a player it's every submesh; for a staff full model it
/// drops the jersey/suit/skates.
pub fn head_asset_indices(p: &SubmeshPartition) -> Vec<usize> {
    let mut idx: Vec<usize> = p
        .head_core
        .iter()
        .chain(&p.hair)
        .chain(&p.facial_hair)
        .copied()
        .collect();
    idx.sort_unstable();
    idx
}

#[cfg(test)]
mod tests {
    use super::SubmeshPart::*;
    use super::*;

    #[test]
    fn renderware_names_classify() {
        // head-core
        for n in ["head_.Mesh", "head__1.Mesh", "eyel_.Mesh", "eyer_.Mesh", "mouthbag_.Mesh"] {
            assert_eq!(classify_submesh(n), HeadCore, "{n}");
        }
        // hair
        for n in ["haircap_.Mesh", "cardsa_.Mesh", "cardsb_.Mesh", "hair_.Mesh", "haira_.Mesh", "hairb_.Mesh"] {
            assert_eq!(classify_submesh(n), Hair, "{n}");
        }
    }

    #[test]
    fn ps4_geo_names_classify() {
        for n in [
            "geo_headShape",
            "geo_head_lowShape",
            "geo_leyeShape",
            "geo_reyeShape",
            "geo_eyesShape",
            "geo_eyes_lowShape",
            "geo_eyelashesShape",
        ] {
            assert_eq!(classify_submesh(n), HeadCore, "{n}");
        }
        for n in [
            "geo_haircapShape",
            "geo_cardsaShape",
            "geo_cardsbShape",
            "geo_headgear_haircapShape",
            "geo_hat_haircapShape",
            "geo_toque_haircapShape",
        ] {
            assert_eq!(classify_submesh(n), Hair, "{n}");
        }
    }

    #[test]
    fn facegear_and_beards_are_facial_hair() {
        // Face marker wins even though the name also carries a hair primitive.
        for n in [
            "geo_facegear_haircapShape",
            "geo_facegear_cardsaShape",
            "geo_facegear_cardsbShape",
            "geo_hat_facegear_haircapShape",
            "geo_toque_facegear_haircapShape",
            "beard_.Mesh",
            "moustache_.Mesh",
        ] {
            assert_eq!(classify_submesh(n), FacialHair, "{n}");
        }
    }

    #[test]
    fn headgear_is_hair_not_facial_and_head_is_not_hair() {
        // `headgear` hair must not be mistaken for the bare `head` head-core token,
        // and the bare head must not be caught by the `hair` marker.
        assert_eq!(classify_submesh("geo_headgear_cardsaShape"), Hair);
        assert_eq!(classify_submesh("geo_headShape"), HeadCore);
        assert_eq!(classify_submesh("head_.Mesh"), HeadCore);
    }

    #[test]
    fn staff_body_parts_classify_as_body() {
        for n in [
            "coachhands_.Mesh",
            "coachsuit_.Mesh",
            "geo_referee_jersey_highShape",
            "geo_referee_pants_lowShape",
            "refereeleftskate_.Mesh",
            "refereeleftwhistle_.Mesh",
        ] {
            assert_eq!(classify_submesh(n), Body, "{n}");
        }
    }

    /// Regression guard over the **complete** submesh vocabulary the Phase-0
    /// census observed across every title + platform (docs/audits/head-hair-census.md,
    /// 39 distinct names). If a future title introduces a name that misclassifies,
    /// re-run the census and extend this table.
    #[test]
    fn full_census_vocabulary_classifies_as_expected() {
        let cases: &[(&str, SubmeshPart)] = &[
            // hair
            ("cardsa_.mesh", Hair),
            ("cardsb_.mesh", Hair),
            ("geo_cardsashape", Hair),
            ("geo_cardsbshape", Hair),
            ("geo_haircapshape", Hair),
            ("geo_hat_haircapshape", Hair),
            ("geo_headgear_cardsashape", Hair),
            ("geo_headgear_haircapshape", Hair),
            ("geo_toque_haircapshape", Hair),
            ("hair_.mesh", Hair),
            ("haira_.mesh", Hair),
            ("hairb_.mesh", Hair),
            ("haircap_.mesh", Hair),
            // facial hair (facegear cards/haircap in the face region)
            ("geo_facegear_cardsbshape", FacialHair),
            ("geo_facegear_haircapshape", FacialHair),
            ("geo_hat_facegear_haircapshape", FacialHair),
            ("geo_toque_facegear_haircapshape", FacialHair),
            // body (staff full models)
            ("coachhands_.mesh", Body),
            ("coachsuit_.mesh", Body),
            ("geo_referee_jersey_highshape", Body),
            ("geo_referee_jersey_lowshape", Body),
            ("geo_referee_pants_highshape", Body),
            ("geo_referee_pants_lowshape", Body),
            // head-core (head/eyes/mouthbag + unnamed/other fall back here, kept in head)
            ("eyel_.mesh", HeadCore),
            ("eyer_.mesh", HeadCore),
            ("geo_eyelashesshape", HeadCore),
            ("geo_eyes_lowshape", HeadCore),
            ("geo_eyesshape", HeadCore),
            ("geo_head_lowshape", HeadCore),
            ("geo_headshape", HeadCore),
            ("geo_leyeshape", HeadCore),
            ("geo_reyeshape", HeadCore),
            ("head_.mesh", HeadCore),
            ("head__1.mesh", HeadCore),
            ("mouthbag_.mesh", HeadCore),
            ("mesh_0", HeadCore),
            ("mesh_1", HeadCore),
            ("mesh_2", HeadCore),
            ("tmfvk", HeadCore),
        ];
        assert_eq!(cases.len(), 39, "census recorded 39 distinct submesh names");
        for (name, want) in cases {
            assert_eq!(classify_submesh(name), *want, "{name}");
        }
    }

    #[test]
    fn partition_groups_indices_in_order() {
        use tdb_rx2::playermesh::{BonePalette, PlayerAsset, PositionFormat, Submesh, UvFormat};
        let sm = |name: &str| Submesh {
            name: name.to_string(),
            stride: 0,
            position_format: PositionFormat::Float,
            uv_format: UvFormat::Float,
            has_blend_indices: false,
            has_blend_weights: false,
            vertices: Vec::new(),
            triangles: Vec::new(),
        };
        let asset = PlayerAsset {
            submeshes: vec![
                sm("head_.Mesh"),         // 0 head-core
                sm("eyel_.Mesh"),         // 1 head-core
                sm("haircap_.Mesh"),      // 2 hair
                sm("cardsa_.Mesh"),       // 3 hair
                sm("mouthbag_.Mesh"),     // 4 head-core
                sm("beard_.Mesh"),        // 5 facial hair
                sm("refereejersey_.Mesh"), // 6 body
            ],
            skin: BonePalette { matrices: Vec::new() },
        };
        let p = partition(&asset);
        assert_eq!(p.head_core, vec![0, 1, 4]);
        assert_eq!(p.hair, vec![2, 3]);
        assert_eq!(p.facial_hair, vec![5]);
        assert_eq!(p.body, vec![6]);
        assert!(p.has_hair() && p.has_facial_hair());
        // The head asset is everything except the jersey, in original order.
        assert_eq!(head_asset_indices(&p), vec![0, 1, 2, 3, 4, 5]);
    }
}
