//! Resolve a *selected* equipment id (from a player's `xmja`/`fMVl` record, or a
//! Hockey Shop pick) to the on-disk render files that texture it — the mesh
//! container plus the one colour map that drives its diffuse. Ported from the prior
//! app's Equipment Studio piece preview (clean-room port authorized for this
//! feature), producing paths the existing [`crate::kit_preview::decode_kit_preview`]
//! assembler consumes rather than decoding here.
//!
//! Xbox 360 `.rx2` and PS3 `.rpsgl` mesh containers feed the native preview
//! decoder. Texture-only containers contribute their colour map independently.

use std::path::{Path, PathBuf};

use crate::equipment_slots::{list_equipment_slot_assets, slot_spec};
use crate::kit_preview::KitPieceInput;

/// Texture-map filename suffixes, ordered by preview preference: the colour map
/// first, then diffuse/albedo-ish fallbacks. Normal / spec maps are never the
/// diffuse (flat-diffuse preview, same rule as heads).
const COLOR_SUFFIX_PREFERENCE: [&str; 3] = ["cm", "dm", "am"];
const NON_DIFFUSE_SUFFIXES: [&str; 2] = ["nm", "sm"];

/// The render files that texture one selected equipment piece, resolved to
/// absolute paths for [`crate::kit_preview::KitPieceInput`].
#[derive(Debug, Clone, Default)]
pub struct EquipmentPiece {
    /// The mesh container (Xbox `.rx2` or PS3 `.rpsgl`), when the slot ships a per-id mesh. `None`
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

/// Cheap magic sniff for either supported RenderWare platform container.
fn is_rx2_file(abs: &Path) -> bool {
    use std::io::Read;
    let magic = tdb_rx2::MAGIC_RX2;
    let Ok(mut f) = std::fs::File::open(abs) else {
        return false;
    };
    let mut head = vec![0u8; magic.len()];
    f.read_exact(&mut head).is_ok() && (head.starts_with(magic) || head.starts_with(&tdb_rx2::ps3_mesh::MAGIC[..7]))
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
        let stem = Path::new(rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
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
                let stem = Path::new(rel)
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("");
                if matches!(classify_stem(stem), PieceFile::Mesh) {
                    mesh_rel.push(rel.clone());
                    break 'borrow;
                }
            }
        }
    }

    // Mesh: the first supported native container in the selected bundle.
    let mesh_path = mesh_rel.iter().find_map(|rel| {
        let abs = join_rel(rendering_root, rel);
        is_rx2_file(&abs).then_some(abs)
    });

    // Colour texture: one standalone map in `cm > dm > am` order first, else the
    // first texlib. Returned as a path; the assembler decodes + binds it.
    let texture_paths = pick_color_texture(rendering_root, &map_rel, &texlib_rel)
        .into_iter()
        .collect();

    let piece = EquipmentPiece {
        mesh_path,
        texture_paths,
    };
    (piece.mesh_path.is_some() || !piece.texture_paths.is_empty()).then_some(piece)
}

/// A selected helmet id's bundle split into its real parts. EA helmets ship the **shell** mesh
/// (`helmet_<ID>_<v>.rx2`) alongside a separate **chinstrap** mesh (`chinstrap_<ID>_…`); the
/// generic first-mesh pick in [`resolve_equipment_piece`] lands on the alphabetically-first
/// `chinstrap_*`, which is how the player-gear preview showed a floating strap with no helmet.
pub struct HelmetPieces {
    pub shell: Option<PathBuf>,
    pub chinstrap: Option<PathBuf>,
    /// The bundle's chosen colour map (shared by shell and strap).
    pub texture_paths: Vec<String>,
}

/// Resolve a selected helmet id (`xmja.YUZp`) into its shell + chinstrap meshes and colour map,
/// so the kit preview can show **both** pieces. `None` when the id ships no render files.
pub fn resolve_helmet_pieces(rendering_root: &Path, id: u32) -> Option<HelmetPieces> {
    let spec = slot_spec("helmet")?;
    let assets = list_equipment_slot_assets(rendering_root, spec);
    let asset = assets.iter().find(|a| a.id == id)?;

    let mut map_rel: Vec<(String, &'static str)> = Vec::new();
    let mut texlib_rel: Vec<String> = Vec::new();
    let mut shell: Option<PathBuf> = None;
    let mut chinstrap: Option<PathBuf> = None;
    for rel in &asset.files {
        let stem = Path::new(rel)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("");
        match classify_stem(stem) {
            PieceFile::Texlib => texlib_rel.push(rel.clone()),
            PieceFile::Map(s) => map_rel.push((rel.clone(), s)),
            PieceFile::Mesh => {
                let abs = join_rel(rendering_root, rel);
                if !is_rx2_file(&abs) {
                    continue; // unsupported container
                }
                if stem.starts_with("chinstrap") {
                    chinstrap.get_or_insert(abs);
                } else {
                    shell.get_or_insert(abs);
                }
            }
        }
    }
    let texture_paths: Vec<String> = pick_color_texture(rendering_root, &map_rel, &texlib_rel)
        .into_iter()
        .collect();
    (shell.is_some() || chinstrap.is_some() || !texture_paths.is_empty()).then_some(HelmetPieces {
        shell,
        chinstrap,
        texture_paths,
    })
}

/// Best-effort visor mesh for a skater's `xmja.BFnc` visor type (1–3; 0 = none): the helmet
/// folder's `visor_*` meshes, preferring a stem keyed by the type token (`visor_<type>_…`), else
/// the first that decodes. The visor filename keying is UNVERIFIED (the visor files are
/// deliberately excluded from the helmet-slot matchers) — this is a preview aid, not a format
/// assertion.
pub fn resolve_visor_mesh(rendering_root: &Path, visor_type: u32) -> Option<PathBuf> {
    let dir = rendering_root.join("helmet");
    find_prefixed_mesh(&dir, &format!("visor_{visor_type}"))
        .or_else(|| find_prefixed_mesh(&dir, "visor"))
}

/// Assemble the preview pieces for ONE equipment item (the Hockey Shop's 3D view):
/// its mesh — the per-id mesh where the slot has one, else the slot's shared mesh
/// (skates/sticks share a single `<mesh_prefix>_*` shape) — textured with its colour
/// map, plus the flat tape/cage companion mesh where the slot has one. Empty when no
/// decodable mesh resolves (the caller falls back to flat texture tiles).
pub fn slot_preview_pieces(rendering_root: &Path, slot: &str, id: u32) -> Vec<KitPieceInput> {
    let Some(spec) = slot_spec(slot) else {
        return Vec::new();
    };
    let resolved = resolve_equipment_piece(rendering_root, slot, id);
    let texture_paths = resolved
        .as_ref()
        .map(|p| p.texture_paths.clone())
        .unwrap_or_default();
    let folder = rendering_root.join(spec.folder);
    let main_mesh = resolved
        .and_then(|p| p.mesh_path)
        .or_else(|| find_prefixed_mesh(&folder, spec.mesh_prefix));

    let mut pieces = Vec::new();
    if let Some(mesh_path) = main_mesh {
        pieces.push(KitPieceInput {
            kind: spec.key.to_string(),
            mesh_path,
            texture_paths,
            prebuilt_textures: Vec::new(),
            mirror_x: false,
            raise_frac: 0.0,
        });
    }
    // Sticks render a flat-black tape wrap; the goalie mask a flat-metal cage.
    if let Some(tape_prefix) = spec.tape_prefix {
        if let Some(tape_mesh) = find_prefixed_mesh(&folder, tape_prefix) {
            let kind = if tape_prefix == "cage" {
                "cage"
            } else {
                "tape"
            };
            pieces.push(KitPieceInput {
                kind: kind.to_string(),
                mesh_path: tape_mesh,
                texture_paths: Vec::new(),
                prebuilt_textures: Vec::new(),
                mirror_x: false,
                raise_frac: 0.0,
            });
        }
    }
    pieces
}

/// Find the first `<prefix>_*.rx2` file under `dir` whose bytes actually decode as a
/// mesh (so a same-prefixed *texture* container is skipped). `None` when none do.
fn find_prefixed_mesh(dir: &Path, prefix: &str) -> Option<PathBuf> {
    let Ok(read) = std::fs::read_dir(dir) else {
        return None;
    };
    let mut candidates: Vec<PathBuf> = read
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .and_then(|s| s.to_str())
                .is_some_and(|e| e.eq_ignore_ascii_case("rx2") || e.eq_ignore_ascii_case("rpsgl"))
                && p.file_stem()
                    .and_then(|s| s.to_str())
                    .is_some_and(|s| s.starts_with(&format!("{prefix}_")) || s == prefix)
        })
        .collect();
    candidates.sort();
    candidates
        .into_iter()
        .find(|p| std::fs::read(p).is_ok_and(|bytes| decodes_as_mesh(&bytes)))
}

/// Locate the COLOUR-map texture slot inside a native RenderWare texlib: the
/// `(texture_index, width, height)` of the texture whose name carries `_cm`, else
/// texture 0. `None` when the file isn't a decodable texlib.
pub fn color_map_slot(path: &Path) -> Option<(usize, u32, u32)> {
    let bytes = std::fs::read(path).ok()?;
    let textures = crate::assets::read_render_rasters(&bytes).ok()?;
    let idx = textures
        .iter()
        .position(|t| t.name.to_ascii_lowercase().contains("_cm"))
        .unwrap_or(0);
    let t = textures.get(idx)?;
    Some((idx, t.width, t.height))
}

/// Whether these bytes decode as an equipment mesh with either reader.
fn decodes_as_mesh(bytes: &[u8]) -> bool {
    if let Ok(mesh) = tdb_rx2::parse_mesh(bytes) {
        if !mesh.positions.is_empty() && !mesh.triangles.is_empty() {
            return true;
        }
    }
    tdb_rx2::playermesh::parse_equipment_mesh(bytes)
        .map(|a| a.submeshes.iter().any(|s| !s.triangles.is_empty()))
        .unwrap_or(false)
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
        assert!(matches!(
            classify_stem("helmet_0_14_dm"),
            PieceFile::Map("dm")
        ));
        assert!(matches!(
            classify_stem("goaliehelmet_2_330_tm"),
            PieceFile::Map("other")
        ));
    }
}
