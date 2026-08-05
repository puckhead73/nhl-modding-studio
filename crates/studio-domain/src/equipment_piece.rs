//! Resolve a *selected* equipment id (from a player's `xmja`/`fMVl` record, or a
//! Hockey Shop pick) to the on-disk render files that texture it — the mesh
//! container plus the one colour map that drives its diffuse. Ported from the prior
//! app's Equipment Studio piece preview (clean-room port authorized for this
//! feature), producing paths the existing [`crate::kit_preview::decode_kit_preview`]
//! assembler consumes rather than decoding here.
//!
//! Xbox 360 `.rx2` only: PS3 `.rpsgl` caches carry no mesh codec in this app and
//! PS3 mesh preview is disabled, so a non-`.rx2` mesh file is skipped (the piece
//! still contributes its colour texture where one decodes).

use std::path::{Path, PathBuf};

use crate::equipment_slots::{list_equipment_slot_assets, slot_spec};

/// Texture-map filename suffixes, ordered by preview preference: the colour map
/// first, then diffuse/albedo-ish fallbacks. Normal / spec maps are never the
/// diffuse (flat-diffuse preview, same rule as heads).
const COLOR_SUFFIX_PREFERENCE: [&str; 3] = ["cm", "dm", "am"];
const NON_DIFFUSE_SUFFIXES: [&str; 2] = ["nm", "sm"];

/// The render files that texture one selected equipment piece, resolved to
/// absolute paths for [`crate::kit_preview::KitPieceInput`].
#[derive(Debug, Clone, Default)]
pub struct EquipmentPiece {
    /// The mesh container (Xbox `.rx2`), when the slot ships a per-id mesh. `None`
    /// for texture-only slots (skates/sticks share one mesh across ids) — the
    /// caller keeps its default mesh and applies just `texture_paths`.
    pub mesh_path: Option<PathBuf>,
    /// The single chosen colour-map file (absolute path), or empty when none
    /// decoded — fed to the kit assembler's `texture_paths`.
    pub texture_paths: Vec<String>,
}

/// File-stem classification within one equipment id's bundle.
enum PieceFile {
    /// `texlib_<…>` — multi-texture library.
    Texlib,
    /// `<part>_<…>_<map>` — one standalone texture map (suffix kept).
    Map(&'static str),
    /// No map suffix → mesh-bearing container (`glove_7`, `helmet_14_2`, …).
    Mesh,
}

fn classify_stem(stem: &str) -> PieceFile {
    if stem.starts_with("texlib") {
        return PieceFile::Texlib;
    }
    let last = stem.rsplit('_').next().unwrap_or("");
    for s in COLOR_SUFFIX_PREFERENCE {
        if last.eq_ignore_ascii_case(s) {
            return PieceFile::Map(s);
        }
    }
    for s in NON_DIFFUSE_SUFFIXES {
        if last.eq_ignore_ascii_case(s) {
            return PieceFile::Map(s);
        }
    }
    // Numeric tail → a mesh stem; unknown alpha suffix (`tm`, `rm`, …) → a texture
    // map we don't preview rather than risk a mesh-parse of a texture container.
    if last.chars().all(|c| c.is_ascii_digit()) || last.is_empty() {
        PieceFile::Mesh
    } else {
        PieceFile::Map("other")
    }
}

/// Whether a render file on disk is an Xbox `.rx2` container (as opposed to a PS3
/// `.rpsgl`, which has no mesh codec here). Cheap magic sniff of the first bytes.
fn is_rx2_file(abs: &Path) -> bool {
    use std::io::Read;
    let magic = tdb_rx2::MAGIC_RX2;
    let Ok(mut f) = std::fs::File::open(abs) else {
        return false;
    };
    let mut head = vec![0u8; magic.len()];
    f.read_exact(&mut head).is_ok() && head.starts_with(magic)
}

/// Resolve one selected `(slot, id)` to its mesh + colour-map paths. `None` when
/// the slot key is unknown or the id ships no render files.
pub fn resolve_equipment_piece(
    rendering_root: &Path,
    slot: &str,
    id: u32,
) -> Option<EquipmentPiece> {
    let spec = slot_spec(slot)?;
    let assets = list_equipment_slot_assets(rendering_root, spec);
    let asset = assets.iter().find(|a| a.id == id)?;

    // Partition the id's bundle (files are already name-sorted by the lister).
    let mut mesh_rel: Vec<String> = Vec::new();
    let mut map_rel: Vec<(String, &'static str)> = Vec::new();
    let mut texlib_rel: Vec<String> = Vec::new();
    for rel in &asset.files {
        let stem = Path::new(rel).file_stem().and_then(|s| s.to_str()).unwrap_or("");
        match classify_stem(stem) {
            PieceFile::Texlib => texlib_rel.push(rel.clone()),
            PieceFile::Map(s) => map_rel.push((rel.clone(), s)),
            PieceFile::Mesh => mesh_rel.push(rel.clone()),
        }
    }

    // Some slots ship ONE shared mesh shape with per-id textures (gloves:
    // `glove_0.rx2` + `glove_<ID>_*` maps). When this id's own bundle has no mesh,
    // borrow the shape from the lowest-id bundle that has one — what the game does.
    if mesh_rel.is_empty() {
        'borrow: for other in &assets {
            for rel in &other.files {
                let stem = Path::new(rel).file_stem().and_then(|s| s.to_str()).unwrap_or("");
                if matches!(classify_stem(stem), PieceFile::Mesh) {
                    mesh_rel.push(rel.clone());
                    break 'borrow;
                }
            }
        }
    }

    // Mesh: the first Xbox `.rx2` mesh candidate. Non-rx2 (PS3) is skipped.
    let mesh_path = mesh_rel.iter().find_map(|rel| {
        let abs = join_rel(rendering_root, rel);
        is_rx2_file(&abs).then_some(abs)
    });

    // Colour texture: one standalone map in `cm > dm > am` order first, else the
    // first texlib. Returned as a path; the assembler decodes + binds it.
    let texture_paths = pick_color_texture(rendering_root, &map_rel, &texlib_rel)
        .into_iter()
        .collect();

    let piece = EquipmentPiece { mesh_path, texture_paths };
    (piece.mesh_path.is_some() || !piece.texture_paths.is_empty()).then_some(piece)
}

/// Pick ONE colour-map file: standalone maps in `cm > dm > am` preference, then the
/// first texlib. Returns the absolute path as a string (kit-assembler input).
fn pick_color_texture(
    rendering_root: &Path,
    map_rel: &[(String, &'static str)],
    texlib_rel: &[String],
) -> Option<String> {
    for pref in COLOR_SUFFIX_PREFERENCE {
        for (rel, suffix) in map_rel {
            if *suffix == pref {
                return Some(join_rel(rendering_root, rel).to_string_lossy().into_owned());
            }
        }
    }
    texlib_rel
        .first()
        .map(|rel| join_rel(rendering_root, rel).to_string_lossy().into_owned())
}

/// Join a `<rendering>/`-relative forward-slash path onto the root, component by
/// component, so it resolves natively on any platform.
fn join_rel(root: &Path, rel: &str) -> PathBuf {
    let mut p = root.to_path_buf();
    for part in rel.split('/') {
        p.push(part);
    }
    p
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_stem_shapes() {
        assert!(matches!(classify_stem("texlib_14"), PieceFile::Texlib));
        assert!(matches!(classify_stem("glove_7"), PieceFile::Mesh));
        assert!(matches!(classify_stem("helmet_14_2"), PieceFile::Mesh));
        assert!(matches!(classify_stem("chinstrap_3_0"), PieceFile::Mesh));
        assert!(matches!(classify_stem("glove_7_cm"), PieceFile::Map("cm")));
        assert!(matches!(classify_stem("glove_7_am"), PieceFile::Map("am")));
        assert!(matches!(classify_stem("glove_7_nm"), PieceFile::Map("nm")));
        assert!(matches!(classify_stem("helmet_0_14_dm"), PieceFile::Map("dm")));
        assert!(matches!(classify_stem("goaliehelmet_2_330_tm"), PieceFile::Map("other")));
    }
}
