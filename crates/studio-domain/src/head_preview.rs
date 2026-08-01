//! Head-mesh **preview** — decode a player `.rx2` into per-submesh geometry and
//! its texlib textures for the 3D viewer, so the operator can toggle each
//! submesh and see what it is.
//!
//! This exists mainly to make sense of **NHL 10 heads**: 312 of the 742 carry a
//! per-submesh rig layout the strict parser refuses ([`SkinPolicy::LenientFirst`]
//! sidesteps that — the geometry is intact, only the bone palette is ambiguous).
//! The preview renders **rigid**: positions are already in bind-pose model space,
//! and identifying submeshes needs no skinning, so no bind matrices are sent and
//! the whole rig question is out of scope here.
//!
//! Texture binding, transparency and draw order are ported from the prior app's
//! `historical_head` module (name-keyed: `hair`→hair maps, `eye`→eye maps,
//! `mouth`→mouth/head, else head).

use std::path::PathBuf;

use serde::{Deserialize, Serialize};

use crate::assets::linear_to_rgba;
use crate::{DomainError, Result};

/// One decoded texlib raster, as RGBA8 for upload to a `DataTexture`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewTexture {
    /// Raster name from the texlib (e.g. `head_67_cm.Raster`) — shown in the UI
    /// so the operator can see what bound where.
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Base64 RGBA8 (`4 * width * height` bytes).
    pub rgba_base64: String,
}

/// One submesh of the head, as flat arrays ready for a `BufferGeometry`.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PreviewSubmesh {
    /// Name from the rx2 name table, e.g. `head_.Mesh`, `haircap_.Mesh`,
    /// `eyel_.Mesh`, `mouthbag_.Mesh`. This is the label the toggle shows.
    pub name: String,
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub uv1: Vec<[f32; 2]>,
    pub triangles: Vec<[u32; 3]>,
    /// Index into [`HeadPreview::textures`] for this submesh's diffuse map;
    /// `None` renders untextured.
    pub material_index: Option<u32>,
    /// Alpha-cutout hair (shells/cards) — drawn with `alphaTest`.
    pub transparent: bool,
    /// Draw order (opaque head = 0, hair after).
    pub render_order: i32,
}

/// Everything the viewer needs for one head.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HeadPreview {
    pub submeshes: Vec<PreviewSubmesh>,
    pub textures: Vec<PreviewTexture>,
    /// Framing over the union of all submeshes.
    pub center: [f32; 3],
    pub span: f32,
    /// The rx2 the mesh came from (debug/detail).
    pub source_path: String,
    /// `true` when the mesh needed the lenient parse — i.e. its AnimationSkin
    /// copies disagree (the NHL 10 per-submesh-rig case). Surfaced so the UI can
    /// flag that the rig is unresolved even though the geometry rendered.
    pub lenient_rig: bool,
}

/// Decode a head for preview.
///
/// `mesh_paths` is the head mesh followed by any companion meshes to merge in —
/// one entry for NHL 10 → Legacy (the `player_` container already bundles hair),
/// two for NHL 09 (`head_<id>.rx2` + its separate `hair_<id>_0_0_0.rx2`). The
/// first entry is the head, and it decides `source_path` and `lenient_rig`.
///
/// `texture_paths` are the texlib containers that go with them (a bundled
/// `texlib_<id>.rx2`, or the loose per-raster files NHL 09/10 use). The list may
/// be empty; submeshes then render untextured.
pub fn decode_head_preview(
    mesh_paths: &[PathBuf],
    texture_paths: &[String],
) -> Result<HeadPreview> {
    let head_path = mesh_paths
        .first()
        .ok_or_else(|| DomainError::Extract("no head mesh path given".into()))?;

    let textures = decode_textures(texture_paths);

    let mut submeshes = Vec::new();
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    let mut strict_ok = true;

    for (i, mesh_path) in mesh_paths.iter().enumerate() {
        let mesh_bytes = crate::assets::read_asset_file(mesh_path)?;
        let is_mcd = mesh_path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("mcd"));
        // The head mesh (first) drives the clean-rig flag; a companion hair mesh
        // failing strict doesn't change it (its rig isn't what we edit). The PS4
        // `.mcd` rig is external, so there's no multi-skin ambiguity to flag.
        if i == 0 {
            strict_ok = is_mcd
                || tdb_rx2::playermesh::parse_player_mesh(&mesh_bytes).is_ok();
        }
        let asset = crate::assets::parse_head_mesh(mesh_path, &mesh_bytes, true)?;

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
            let (transparent, render_order) = submesh_blend_mode(&sm.name);
            submeshes.push(PreviewSubmesh {
                material_index: pick_submesh_texture(&sm.name, &textures),
                name: sm.name.clone(),
                positions,
                normals,
                uv1,
                triangles: sm.triangles.clone(),
                transparent,
                render_order,
            });
        }
    }

    // Empty mesh (no verts anywhere): centre at origin, unit span, so the camera
    // maths never divides by zero.
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

    Ok(HeadPreview {
        submeshes,
        textures,
        center,
        span,
        source_path: head_path.to_string_lossy().into_owned(),
        lenient_rig: !strict_ok,
    })
}

/// Decode every raster in the given texlib containers to RGBA. A container that
/// fails to read or parse is skipped rather than sinking the whole preview —
/// the mesh still renders, just untextured for the affected maps.
fn decode_textures(texture_paths: &[String]) -> Vec<PreviewTexture> {
    let mut out = Vec::new();
    for path in texture_paths {
        let p = std::path::Path::new(path);
        let Ok(bytes) = crate::assets::read_asset_file(p) else {
            continue;
        };
        // PS4 GNF: de-tile + decode to RGBA directly (a single texture per file,
        // named by the file stem).
        if p.extension().is_some_and(|e| e.eq_ignore_ascii_case("gnf")) {
            if let Ok(tex) = tdb_rx2::gnf::parse(&bytes) {
                if let Ok(rgba) = crate::assets::gnf_to_rgba(&tex) {
                    let name = p
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    out.push(PreviewTexture {
                        name,
                        width: tex.width,
                        height: tex.height,
                        rgba_base64: base64_encode(&rgba),
                    });
                }
            }
            continue;
        }
        let Ok(lib) = tdb_rx2::texlib::parse(&bytes) else {
            continue;
        };
        // The name feeds `pick_submesh_texture`'s part/map matching (`head`/`eye`/`cm`/`am`). A
        // bundled Legacy `texlib_*` names each raster internally (`head_67_cm.Raster`), but the
        // single-raster files NHL 07/10 ship one map per file (`head_<id>_0_0_0_cm.rx2`) leave the
        // internal name generic (`texture_0000`). Fall back to the file stem, which carries the part
        // and `cm`/`am` map suffix, so the head map isn't mis-assigned to a black alpha map.
        let file_stem = std::path::Path::new(path)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        for tex in &lib.textures {
            let Ok(rgba) = linear_to_rgba(tex.format, tex.width, tex.height, &tex.base_mip_linear)
            else {
                continue;
            };
            let name = if tex.name.starts_with("texture_") && lib.textures.len() == 1 {
                file_stem.clone()
            } else {
                tex.name.clone()
            };
            out.push(PreviewTexture {
                name,
                width: tex.width,
                height: tex.height,
                rgba_base64: base64_encode(&rgba),
            });
        }
    }
    out
}

/// Blend classification by submesh name, ported from the prior app: hair
/// shells/cards are alpha-cutout and draw after the opaque head (cards after
/// the haircap), so layered hair composites correctly.
fn submesh_blend_mode(name: &str) -> (bool, i32) {
    let n = name.to_ascii_lowercase();
    if n.contains("cards") {
        (true, 2)
    } else if n.contains("hair") {
        (true, 1)
    } else {
        (false, 0)
    }
}

/// Resolve a submesh's diffuse map by the part keyword its name implies,
/// preferring the `cm` (color-map) raster. Ported from the prior app's
/// `pick_submesh_texture`: `hair`/`cards`→hair maps, `eye`→eye maps (no head
/// fallback — eye UVs are authored against a standalone eye raster), `mouth`→
/// mouth/teeth/tongue then head, else head.
fn pick_submesh_texture(submesh_name: &str, textures: &[PreviewTexture]) -> Option<u32> {
    let n = submesh_name.to_ascii_lowercase();
    let is_eye = n.contains("eye");
    let keywords: &[&str] = if n.contains("hair") || n.contains("cards") {
        &["hair"]
    } else if is_eye {
        &["eye"]
    } else if n.contains("mouth") {
        &["mouth", "teeth", "tongue"]
    } else {
        &["head"]
    };

    let lc = |t: &PreviewTexture| t.name.to_ascii_lowercase();
    let find = |pred: &dyn Fn(&str) -> bool| -> Option<u32> {
        textures.iter().position(|t| pred(&lc(t))).map(|i| i as u32)
    };

    for kw in keywords {
        if let Some(i) = find(&|name| name.contains(kw) && name.contains("cm")) {
            return Some(i);
        }
    }
    for kw in keywords {
        if let Some(i) = find(&|name| name.contains(kw)) {
            return Some(i);
        }
    }
    if is_eye {
        return None;
    }
    find(&|name| name.contains("head") && name.contains("cm"))
        .or_else(|| find(&|name| name.contains("head")))
        .or_else(|| (!textures.is_empty()).then_some(0))
}

/// Standard base64 (no line breaks) — the payloads cross the Tauri boundary as
/// one string per texture. Kept local so the module has no base64 dependency.
fn base64_encode(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (b[0] as u32) << 16 | (b[1] as u32) << 8 | b[2] as u32;
        out.push(T[(n >> 18 & 63) as usize] as char);
        out.push(T[(n >> 12 & 63) as usize] as char);
        out.push(if chunk.len() > 1 {
            T[(n >> 6 & 63) as usize] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            T[(n & 63) as usize] as char
        } else {
            '='
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// End-to-end viewer path on a real PS4 `.mcd`: the extension dispatch, the
    /// per-submesh flatten, and the framing all run. Skipped when the extracted
    /// tree isn't mounted (CI).
    #[test]
    fn ps4_mcd_head_decodes_for_preview() {
        let path = r"H:\Emulators\ShadPS4\Games\CUSA00561\databigs\Extracted\nhl\data\kettle\rendering\characters\player\alexander_ovechkin.mcd";
        if !std::path::Path::new(path).is_file() {
            eprintln!("skipping: {path} not present");
            return;
        }
        let preview =
            decode_head_preview(&[PathBuf::from(path)], &[]).expect("mcd head must preview");
        assert!(!preview.submeshes.is_empty(), "submeshes present");
        assert!(preview.span > 0.0, "framing computed");
        // Named submeshes drive texture binding downstream.
        assert!(
            preview.submeshes.iter().any(|s| s.name.contains("head")),
            "a head submesh is named"
        );
        // Every submesh flattened to parallel arrays with in-range indices.
        for s in &preview.submeshes {
            assert_eq!(s.positions.len(), s.normals.len());
            assert_eq!(s.positions.len(), s.uv1.len());
            for t in &s.triangles {
                assert!(t.iter().all(|&i| (i as usize) < s.positions.len()));
            }
        }
    }

    fn tex(name: &str) -> PreviewTexture {
        PreviewTexture {
            name: name.to_string(),
            width: 1,
            height: 1,
            rgba_base64: String::new(),
        }
    }

    #[test]
    fn blend_mode_flags_hair_and_cards() {
        assert_eq!(submesh_blend_mode("head_.Mesh"), (false, 0));
        assert_eq!(submesh_blend_mode("haircap_.Mesh"), (true, 1));
        assert_eq!(submesh_blend_mode("cardsA_.Mesh"), (true, 2));
    }

    #[test]
    fn texture_binding_follows_part_keywords() {
        let texs = vec![
            tex("hair_67_cm.Raster"),
            tex("head_67_cm.Raster"),
            tex("eyes_3_cm.Raster"),
        ];
        assert_eq!(pick_submesh_texture("head_.Mesh", &texs), Some(1));
        assert_eq!(pick_submesh_texture("haircap_.Mesh", &texs), Some(0));
        assert_eq!(pick_submesh_texture("eyel_.Mesh", &texs), Some(2));
        // Mouthbag falls back to the head map (its UVs sit in the head atlas).
        assert_eq!(pick_submesh_texture("mouthbag_.Mesh", &texs), Some(1));
    }

    #[test]
    fn eyes_never_fall_back_to_the_head_map() {
        // Only a head map present — eyes stay untextured rather than binding skin.
        let texs = vec![tex("head_512_cm.Raster")];
        assert_eq!(pick_submesh_texture("eyel_.Mesh", &texs), None);
        assert_eq!(pick_submesh_texture("head_.Mesh", &texs), Some(0));
    }

    #[test]
    fn base64_matches_known_vectors() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }
}
