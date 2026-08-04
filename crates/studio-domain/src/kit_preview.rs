//! Full-**kit** preview — assemble a hockey player's uniform pieces (head,
//! helmet, jersey, gloves, pants, socks, skates) into one textured figure for
//! the 3D viewer, so the operator can see the whole outfit together.
//!
//! ## Why this is just a rigid concatenation
//!
//! Every Legacy kit piece is skinned to **one shared 122-bone rig**
//! (`skeleton_bindpose_player.rx2`): the equipment meshes (`jersey_*`, `pant_*`,
//! `playersock_*`, `glove_*`, `helmet_*`, …) carry blend indices/weights but no
//! bone palette of their own, and every vertex is authored in the **same
//! bind-pose model space** (verified: sock shins ≈ y0.2–0.8, pant hips 0.7–1.2,
//! jersey torso 1.0–1.7, head 1.5–1.95, all in meters). So laying every piece
//! into the scene at its stored positions produces a correct standing figure —
//! no skinning, no bone matrices, no per-piece transform. Skeletal *re-posing*
//! (bending the bind pose into a skating crouch) is a separate, deferred step
//! that needs the bone hierarchy the flat palette doesn't carry.
//!
//! Geometry decode, texture decode and part→map binding are shared with
//! [`crate::head_preview`]; this module adds multi-piece grouping and the
//! equipment-mesh parse (`parse_equipment_mesh`, which tolerates the missing
//! palette the head parse would reject).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::head_preview::{
    decode_textures, pick_submesh_texture, submesh_blend_mode, PreviewSubmesh, PreviewTexture,
};
use crate::{DomainError, Result};

/// One piece to assemble: its kind (drives texture binding + the UI toggle
/// label), the mesh container, and the texlib/raster files that texture it.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KitPieceInput {
    /// `head` | `helmet` | `jersey` | `gloves` | `pants` | `socks` | `skates`.
    pub kind: String,
    pub mesh_path: PathBuf,
    pub texture_paths: Vec<String>,
    /// Already-decoded textures to add to this piece's pool alongside anything
    /// decoded from `texture_paths` — for pieces whose diffuse is *composited*
    /// rather than a file on disk (gloves/helmet zone-mask paints). Defaults
    /// empty; the `#[serde(default)]` keeps older callers/paths valid.
    #[serde(default)]
    pub prebuilt_textures: Vec<PreviewTexture>,
}

/// One assembled piece: its submeshes (material indices point into
/// [`KitPreview::textures`]) plus provenance for the detail affordance.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KitPiece {
    pub kind: String,
    /// Human label for the visibility toggle, e.g. "Jersey", "Gloves".
    pub label: String,
    pub source_path: String,
    pub submeshes: Vec<PreviewSubmesh>,
}

/// Everything the viewer needs for one full kit.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct KitPreview {
    pub pieces: Vec<KitPiece>,
    /// Texture pool shared across pieces; submeshes reference by index.
    pub textures: Vec<PreviewTexture>,
    /// Framing over the union of every piece's geometry.
    pub center: [f32; 3],
    pub span: f32,
}

/// Human label for a piece kind (falls back to the raw kind, title-cased-ish).
fn piece_label(kind: &str) -> String {
    match kind {
        "head" => "Head",
        "helmet" => "Helmet",
        "jersey" => "Jersey",
        "gloves" => "Gloves",
        "pants" => "Pants",
        "socks" => "Socks",
        "skates" => "Skates",
        other => return other.to_string(),
    }
    .to_string()
}

/// Is this piece a player head (multi-submesh, keyword-bound like the head
/// preview) rather than a single-mesh equipment piece?
fn is_head(kind: &str) -> bool {
    kind == "head"
}

/// The keyword an equipment piece's color map should mention, so a bundled
/// texlib that carries several rasters (e.g. a jersey lib holding both
/// `jersey_*_cm` and `font_*_cm`) binds the piece's own body map.
fn piece_keyword(kind: &str) -> &'static str {
    match kind {
        "helmet" => "helmet",
        "jersey" => "jersey",
        "gloves" => "glove",
        "pants" => "pant",
        "socks" => "sock",
        "skates" => "skate",
        _ => "",
    }
}

/// Colour-map suffix tokens, most-preferred first: `cm` for the cloth pieces,
/// `tm`/`dm` for gloves/helmets whose diffuse ships as a texture/diffuse map.
const COLOR_TOKENS: [&str; 3] = ["cm", "tm", "dm"];

/// Pick an equipment piece's diffuse map from the textures decoded for it,
/// preferring one that names the piece kind and a colour-map token, then any
/// colour-map token, then the largest raster, then the first.
fn pick_equipment_texture(kind: &str, textures: &[PreviewTexture]) -> Option<u32> {
    if textures.is_empty() {
        return None;
    }
    let kw = piece_keyword(kind);
    let name = |i: usize| textures[i].name.to_ascii_lowercase();

    if !kw.is_empty() {
        for tok in COLOR_TOKENS {
            if let Some(i) = (0..textures.len()).find(|&i| {
                let n = name(i);
                n.contains(kw) && n.contains(tok)
            }) {
                return Some(i as u32);
            }
        }
    }
    for tok in COLOR_TOKENS {
        if let Some(i) = (0..textures.len()).find(|&i| name(i).contains(tok)) {
            return Some(i as u32);
        }
    }
    // No named colour map — take the largest raster (spec/normal companions are
    // usually smaller or match the body map's size, and any diffuse beats none).
    let largest = (0..textures.len())
        .max_by_key(|&i| textures[i].width as u64 * textures[i].height as u64)
        .unwrap_or(0);
    Some(largest as u32)
}

/// Fill in smooth vertex normals when a submesh's decoded normals are
/// degenerate (all ~zero) — some equipment descriptors carry no normal
/// attribute, and flat-shaded cloth reads badly. Area-weighted face normals
/// accumulated per vertex, matching the RX2 reader's own smoothing.
fn ensure_normals(sm: &mut PreviewSubmesh) {
    let any = sm
        .normals
        .iter()
        .any(|n| n[0].abs() + n[1].abs() + n[2].abs() > 1e-4);
    if any {
        return;
    }
    let mut acc = vec![[0.0f32; 3]; sm.positions.len()];
    for t in &sm.triangles {
        let [a, b, c] = [t[0] as usize, t[1] as usize, t[2] as usize];
        let (pa, pb, pc) = (sm.positions[a], sm.positions[b], sm.positions[c]);
        let u = [pb[0] - pa[0], pb[1] - pa[1], pb[2] - pa[2]];
        let v = [pc[0] - pa[0], pc[1] - pa[1], pc[2] - pa[2]];
        // Cross product (not normalized → area-weighted).
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in &[a, b, c] {
            for k in 0..3 {
                acc[i][k] += n[k];
            }
        }
    }
    for (dst, n) in sm.normals.iter_mut().zip(acc) {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        *dst = if len > 1e-8 {
            [n[0] / len, n[1] / len, n[2] / len]
        } else {
            [0.0, 1.0, 0.0]
        };
    }
}

/// The default mesh files for a kit's shared-shape pieces, resolved by scanning
/// the install's `rendering/` tree. These carry the *shape* (a shared cut / the
/// high-detail glove); the *texture* (team colorway / selected equipment) is
/// supplied separately by the caller. `None` for a piece whose folder is absent.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct KitMeshDefaults {
    /// A shared jersey **cut** — any cut shares the team texlib's UV atlas.
    pub jersey: Option<PathBuf>,
    pub pant: Option<PathBuf>,
    pub sock: Option<PathBuf>,
    /// The high-detail glove (`glove_2.rx2`) every glove texture maps onto —
    /// the reference app's `GLOVE_PREVIEW_MESH_ID = 2` (3047 verts vs 780/190).
    pub glove: Option<PathBuf>,
    pub helmet: Option<PathBuf>,
    /// The high-detail player skate (`playerskate_2.rx2`, 1938 verts vs 528/84) —
    /// same shared-shape / pin-the-best pattern as the glove.
    pub skate: Option<PathBuf>,
}

/// First file (sorted, for determinism) in `dir` whose name satisfies `pred`.
fn first_file(dir: &std::path::Path, pred: impl Fn(&str) -> bool) -> Option<PathBuf> {
    let mut names: Vec<PathBuf> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.file_name().and_then(|s| s.to_str()).is_some_and(&pred))
        .collect();
    names.sort();
    names.into_iter().next()
}

/// `<prefix><digit>…` — a geometry file (`jersey_0…`, `pant_0…`,
/// `playersock_0…`), excluding the texture siblings (`texlib_`, `name_`) and
/// the map suffixes (`_cm`/`_sm`/`_nm`/`_am`).
fn is_mesh_named(name: &str, prefix: &str) -> bool {
    let Some(rest) = name.strip_prefix(prefix) else {
        return false;
    };
    rest.chars().next().is_some_and(|c| c.is_ascii_digit())
        && name.to_ascii_lowercase().ends_with(".rx2")
        && !["_cm.", "_sm.", "_nm.", "_am.", "_dm.", "_tm."]
            .iter()
            .any(|s| name.to_ascii_lowercase().contains(s))
}

/// Resolve the shared-shape default meshes under an install's `rendering` root.
pub fn kit_mesh_defaults(rendering_root: &std::path::Path) -> KitMeshDefaults {
    let jersey_dir = rendering_root.join("jersey");
    let pant_dir = rendering_root.join("pant");
    let sock_dir = rendering_root.join("sock");
    let glove_dir = rendering_root.join("glove");
    let helmet_dir = rendering_root.join("helmet");
    let skate_dir = rendering_root.join("skate");

    // Gloves are pinned to the one high-detail mesh (shared UV); fall back to any
    // `glove_<n>` shape if that specific file is missing.
    let glove = {
        let pinned = glove_dir.join("glove_2.rx2");
        if pinned.is_file() {
            Some(pinned)
        } else {
            first_file(&glove_dir, |n| is_mesh_named(n, "glove_"))
        }
    };
    // Helmet: the reference app's known-good skater helmet, else any helmet shape.
    let helmet = {
        let pinned = helmet_dir.join("helmet_14_2.rx2");
        if pinned.is_file() {
            Some(pinned)
        } else {
            first_file(&helmet_dir, |n| is_mesh_named(n, "helmet_"))
        }
    };

    // Skates: pin the high-detail player skate, else any player skate shape.
    let skate = {
        let pinned = skate_dir.join("playerskate_2.rx2");
        if pinned.is_file() {
            Some(pinned)
        } else {
            first_file(&skate_dir, |n| is_mesh_named(n, "playerskate_"))
        }
    };

    KitMeshDefaults {
        jersey: first_file(&jersey_dir, |n| is_mesh_named(n, "jersey_")),
        pant: first_file(&pant_dir, |n| is_mesh_named(n, "pant_")),
        sock: first_file(&sock_dir, |n| is_mesh_named(n, "playersock_")),
        glove,
        helmet,
        skate,
    }
}

/// Decode and assemble a full kit for preview. Pieces are laid into a shared
/// model space at their stored bind-pose positions (see the module docs). A
/// piece whose mesh fails to read/parse is skipped with the rest still
/// assembling — a preview is best-effort, not all-or-nothing.
pub fn decode_kit_preview(pieces: &[KitPieceInput]) -> Result<KitPreview> {
    if pieces.is_empty() {
        return Err(DomainError::Extract("no kit pieces given".into()));
    }

    let mut out_pieces: Vec<KitPiece> = Vec::new();
    let mut textures: Vec<PreviewTexture> = Vec::new();
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];

    for p in pieces {
        let Ok(mesh_bytes) = crate::assets::read_asset_file(&p.mesh_path) else {
            continue;
        };
        // Head pieces carry their own rig (lenient preview parse); equipment
        // pieces skin against the external bindpose and need the palette-optional
        // parse. Both yield the shared PlayerAsset model.
        let asset = if is_head(&p.kind) {
            crate::assets::parse_head_mesh(&p.mesh_path, &mesh_bytes, true)
        } else {
            tdb_rx2::playermesh::parse_equipment_mesh(&mesh_bytes)
                .map_err(|e| DomainError::Extract(format!("{}: {e}", p.mesh_path.display())))
        };
        let Ok(asset) = asset else { continue };

        // This piece's textures occupy [base, base+len) in the shared pool:
        // those decoded from files, then any pre-composited (glove/helmet) ones.
        let mut piece_textures = decode_textures(&p.texture_paths);
        piece_textures.extend(p.prebuilt_textures.iter().cloned());
        let base = textures.len() as u32;
        let equip_mat = if is_head(&p.kind) {
            None
        } else {
            pick_equipment_texture(&p.kind, &piece_textures).map(|i| base + i)
        };
        textures.extend(piece_textures.iter().cloned());

        let mut submeshes = Vec::with_capacity(asset.submeshes.len());
        for sm in &asset.submeshes {
            let mut positions = Vec::with_capacity(sm.vertices.len());
            let mut normals = Vec::with_capacity(sm.vertices.len());
            let mut uv1 = Vec::with_capacity(sm.vertices.len());
            for v in &sm.vertices {
                positions.push(v.position);
                normals.push(v.normal);
                uv1.push(v.uv);
                for a in 0..3 {
                    min[a] = min[a].min(v.position[a]);
                    max[a] = max[a].max(v.position[a]);
                }
            }
            let material_index = if is_head(&p.kind) {
                pick_submesh_texture(&sm.name, &piece_textures).map(|i| base + i)
            } else {
                equip_mat
            };
            let (transparent, render_order) = submesh_blend_mode(&sm.name);
            let mut out = PreviewSubmesh {
                material_index,
                name: sm.name.clone(),
                positions,
                normals,
                uv1,
                triangles: sm.triangles.clone(),
                transparent,
                render_order,
            };
            ensure_normals(&mut out);
            submeshes.push(out);
        }

        out_pieces.push(KitPiece {
            kind: p.kind.clone(),
            label: piece_label(&p.kind),
            source_path: p.mesh_path.to_string_lossy().into_owned(),
            submeshes,
        });
    }

    if out_pieces.is_empty() {
        return Err(DomainError::Extract(
            "no kit piece could be decoded".into(),
        ));
    }

    let has_extent = min[0] <= max[0];
    let center = if has_extent {
        [
            (min[0] + max[0]) * 0.5,
            (min[1] + max[1]) * 0.5,
            (min[2] + max[2]) * 0.5,
        ]
    } else {
        [0.0, 0.0, 0.0]
    };
    let span = if has_extent {
        (0..3)
            .map(|a| max[a] - min[a])
            .fold(0.0_f32, f32::max)
            .max(f32::MIN_POSITIVE)
    } else {
        1.0
    };

    Ok(KitPreview {
        pieces: out_pieces,
        textures,
        center,
        span,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tex(name: &str, w: u32, h: u32) -> PreviewTexture {
        PreviewTexture {
            name: name.to_string(),
            width: w,
            height: h,
            rgba_base64: String::new(),
        }
    }

    #[test]
    fn equipment_texture_prefers_kind_named_color_map() {
        // A jersey texlib carries both the body map and the number-font map —
        // the jersey mesh must bind its own `jersey_*_cm`, not `font_*_cm`.
        let texs = vec![
            tex("font_10_cm.Raster", 512, 512),
            tex("jersey_10_cm.Raster", 1024, 1024),
            tex("jersey_10_sm.Raster", 1024, 1024),
        ];
        assert_eq!(pick_equipment_texture("jersey", &texs), Some(1));
    }

    #[test]
    fn equipment_texture_falls_back_to_diffuse_then_largest() {
        // Gloves ship a `_tm` diffuse — pick it over a spec/normal companion.
        let texs = vec![tex("feglove_100_0_sm", 256, 256), tex("feglove_100_0_tm", 512, 512)];
        assert_eq!(pick_equipment_texture("gloves", &texs), Some(1));

        // No colour-map token at all → largest raster wins.
        let texs = vec![tex("raster_a", 64, 64), tex("raster_b", 256, 256)];
        assert_eq!(pick_equipment_texture("gloves", &texs), Some(1));

        assert_eq!(pick_equipment_texture("gloves", &[]), None);
    }

    #[test]
    fn ensure_normals_fills_degenerate_only() {
        // A single upward-facing triangle with zeroed normals → +Y after fill.
        let mut sm = PreviewSubmesh {
            material_index: None,
            name: "mesh_0".into(),
            positions: vec![[0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0]],
            normals: vec![[0.0; 3]; 3],
            uv1: vec![[0.0; 2]; 3],
            triangles: vec![[0, 1, 2]],
            transparent: false,
            render_order: 0,
        };
        ensure_normals(&mut sm);
        for n in &sm.normals {
            assert!((n[1] - 1.0).abs() < 1e-3, "expected +Y normal, got {n:?}");
        }

        // Existing normals are left untouched.
        let mut sm2 = sm.clone();
        sm2.normals = vec![[1.0, 0.0, 0.0]; 3];
        ensure_normals(&mut sm2);
        assert_eq!(sm2.normals[0], [1.0, 0.0, 0.0]);
    }

    #[test]
    fn is_mesh_named_excludes_textures_and_requires_digit() {
        assert!(is_mesh_named("jersey_0_0_2_0_0_0.rx2", "jersey_"));
        assert!(is_mesh_named("playersock_0_0.rx2", "playersock_"));
        assert!(is_mesh_named("glove_2.rx2", "glove_"));
        // Textures and sentinels are rejected.
        assert!(!is_mesh_named("texlib_0_0_0.rx2", "jersey_"));
        assert!(!is_mesh_named("jersey_0_0_2_0_0_0_cm.rx2", "jersey_"));
        assert!(!is_mesh_named("feglove_100_0_tm.rx2", "glove_"));
        assert!(!is_mesh_named("jerseytie_0.rx2", "jersey_")); // prefix must be exact word
    }

    #[test]
    fn legacy_x360_kit_mesh_defaults_pin_glove_2() {
        let root = std::path::Path::new(
            r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering",
        );
        if !root.is_dir() {
            eprintln!("skipping: {} not present", root.display());
            return;
        }
        let d = kit_mesh_defaults(root);
        assert!(
            d.glove.as_ref().is_some_and(|p| p.ends_with("glove_2.rx2")),
            "glove pinned to the high-detail glove_2, got {:?}",
            d.glove
        );
        assert!(
            d.skate.as_ref().is_some_and(|p| p.ends_with("playerskate_2.rx2")),
            "skate pinned to the high-detail playerskate_2, got {:?}",
            d.skate
        );
        for (name, got) in [("jersey", &d.jersey), ("pant", &d.pant), ("sock", &d.sock), ("helmet", &d.helmet), ("skate", &d.skate)] {
            assert!(got.is_some(), "{name} default mesh resolved");
            assert!(got.as_ref().unwrap().is_file(), "{name} default mesh exists on disk");
        }
    }

    // Mount-gated end-to-end: assemble a real full kit and prove every piece
    // decoded into a common model space. Skipped in CI / clean-room.
    #[test]
    fn legacy_x360_full_kit_assembles() {
        let root = std::path::Path::new(
            r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering",
        );
        if !root.is_dir() {
            eprintln!("skipping: {} not present", root.display());
            return;
        }
        let piece = |kind: &str, rel: &str| KitPieceInput {
            kind: kind.into(),
            mesh_path: root.join(rel),
            texture_paths: vec![],
            prebuilt_textures: vec![],
        };
        let inputs = vec![
            piece("head", "player/player_10.rx2"),
            piece("helmet", "helmet/helmet_110_0.rx2"),
            piece("jersey", "jersey/jersey_0_0_2_0_0_0.rx2"),
            piece("gloves", "glove/glove_0.rx2"),
            piece("pants", "pant/pant_0_0_0_0.rx2"),
            piece("socks", "sock/playersock_0_0.rx2"),
        ];
        let kit = decode_kit_preview(&inputs).expect("kit assembles");
        // Every requested piece present, each with geometry.
        assert_eq!(kit.pieces.len(), inputs.len(), "all pieces decoded");
        for p in &kit.pieces {
            let verts: usize = p.submeshes.iter().map(|s| s.positions.len()).sum();
            assert!(verts > 0, "{} has geometry", p.kind);
            // Normals filled everywhere (either decoded or smoothed).
            for s in &p.submeshes {
                assert_eq!(s.positions.len(), s.normals.len());
                assert_eq!(s.positions.len(), s.uv1.len());
                assert!(s.normals.iter().all(|n| n[0].is_finite()));
            }
        }
        // The union spans a whole standing figure (skates/socks near the ice to
        // the helmet ~2 m up), not a single piece.
        assert!(kit.span > 1.2, "kit spans a full figure, got {}", kit.span);
    }
}
