//! **Appearance-catalog export** (Phase 3).
//!
//! From a decoded head this writes the catalog assets an operator edits with —
//! grounded in the Phase-1 gate (which heads are a player's own) and the Phase-2
//! [`crate::head_submesh`] partition (which submesh is what):
//!
//! * **full head** — every non-body submesh, `{base}_head_{game}.glb`
//! * **hair** — all haircaps + cards merged into one mesh, `{base}_hair_{game}.glb`
//!   (so a head with both `haircap_a` and `haircap_b` yields a single hair asset)
//! * **facial hair** — only when distinct geometry exists (rare; vanilla facial
//!   hair is painted), `{base}_facialhair_{game}.glb`
//!
//! plus the head's textures as PNG (`{base}_{map}_{game}.png`), shared by the head
//! and hair assets — the game keeps one texlib per head carrying both. Geometry is
//! written geometry-only (no armature — the appearance editor wants the mesh, and
//! the porting rig is out of scope here).
//!
//! `{base}` is the catalog name from the export slug (`last_first_dd_mm_yyyy` →
//! `last_first`), the date re-appended only when two players collide on the plain
//! name. The `{game}` token is the workspace `game_id` (`nhl14`, `nhllegacy`, …).

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use tdb_rx2::playermesh::PlayerAsset;

use crate::assets::{
    gnf_to_rgba, linear_to_rgba, parse_head_mesh, read_asset_file, DecodedImage, PlayerAssets,
};
use crate::head_submesh::{head_asset_indices, partition};
use crate::Result;

/// How to build the catalog.
#[derive(Clone, Debug)]
pub struct CatalogOptions {
    /// Game token appended to every asset name (the workspace `game_id`).
    pub game_tag: String,
    /// Decode + write the head's textures as PNG alongside the meshes.
    pub textures: bool,
}

/// One asset written into the catalog — the row the Phase-5 manifest is built from.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// File stem, e.g. `ovechkin_alexander_hair_nhllegacy`.
    pub name: String,
    /// Where this asset came from: `player` (a bespoke player's own head) or one
    /// of `generic` / `coach` / `referee` / `trainer` / `commissioner` (swept).
    pub origin: String,
    /// `head` | `hair` | `facialhair`.
    pub class: String,
    /// Game token.
    pub game: String,
    /// Relative path of the written `.glb`.
    pub mesh_file: String,
    /// Submesh names that went into this asset.
    pub submeshes: Vec<String>,
    /// Relative paths of the textures the head/hair share.
    pub textures: Vec<String>,
    /// The source mesh container this came from (provenance).
    pub source: String,
}

/// What a catalog export produced.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HeadCatalogSummary {
    pub heads_written: usize,
    pub hair_written: usize,
    pub facial_hair_written: usize,
    pub textures_written: usize,
    pub bytes_written: u64,
    /// Players skipped for lacking a usable base name (no slug).
    pub skipped_no_base: usize,
    pub failures: Vec<String>,
    /// One row per written mesh asset (head/hair/facialhair), for the manifest.
    pub entries: Vec<CatalogEntry>,
}

/// The catalog base from an export slug (`last_first_dd_mm_yyyy`) — the trailing
/// `dd_mm_yyyy` dropped, leaving `last_first`. A slug that doesn't carry a trailing
/// date is returned unchanged.
pub fn catalog_base_from_slug(slug: &str) -> String {
    let mut it = slug.rsplitn(4, '_');
    let (yyyy, mm, dd) = (it.next(), it.next(), it.next());
    // Only strip when the three trailing groups really look like a date.
    let is_num = |s: Option<&str>| s.is_some_and(|s| !s.is_empty() && s.bytes().all(|b| b.is_ascii_digit()));
    if is_num(yyyy) && is_num(mm) && is_num(dd) {
        if let Some(base) = it.next() {
            return base.to_string();
        }
    }
    slug.to_string()
}

/// Build a geometry subset [`PlayerAsset`] from selected submesh indices (order
/// preserved). Geometry-only export ignores the skin, so it's cloned as-is.
fn subset(asset: &PlayerAsset, idx: &[usize]) -> PlayerAsset {
    PlayerAsset {
        submeshes: idx.iter().map(|&i| asset.submeshes[i].clone()).collect(),
        skin: asset.skin.clone(),
    }
}

/// Create parents and write, returning bytes written.
fn write_out(path: &Path, bytes: &[u8]) -> std::io::Result<u64> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, bytes)?;
    Ok(bytes.len() as u64)
}

/// A map label for a texlib raster: strip numeric id groups and the `.Raster`
/// suffix — `head_67_cm.Raster` → `head_cm`. For a single-raster loose file whose
/// internal name is generic (`texture_0000`), fall back to the file stem, which
/// carries the part + map suffix (`head_10000_cm` → `head_cm`).
fn map_label(raster_name: &str, file_stem: &str, single_generic: bool) -> String {
    let src = if single_generic && raster_name.to_ascii_lowercase().starts_with("texture_") {
        file_stem
    } else {
        raster_name
    };
    let src = src.strip_suffix(".Raster").unwrap_or(src);
    let cleaned: Vec<&str> = src
        .split('_')
        .filter(|seg| !seg.is_empty() && !seg.bytes().all(|b| b.is_ascii_digit()))
        .collect();
    if cleaned.is_empty() {
        src.to_ascii_lowercase()
    } else {
        cleaned.join("_").to_ascii_lowercase()
    }
}

/// Decode one texture container (`.rx2` texlib or PS4 `.gnf`) to `(label, png)`
/// pairs. A container that can't be read/parsed yields nothing (non-fatal).
fn decode_texture_pngs(path: &Path) -> Vec<(String, Vec<u8>)> {
    let Ok(bytes) = read_asset_file(path) else {
        return Vec::new();
    };
    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();
    let mut out = Vec::new();
    if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("gnf")) {
        if let Ok(tex) = tdb_rx2::gnf::parse(&bytes) {
            if let Ok(rgba) = gnf_to_rgba(&tex) {
                if let Ok(png) = (DecodedImage { width: tex.width, height: tex.height, rgba }).to_png() {
                    out.push((map_label(&stem, &stem, false), png));
                }
            }
        }
        return out;
    }
    let Ok(lib) = tdb_rx2::texlib::parse(&bytes) else {
        return out;
    };
    let single = lib.textures.len() == 1;
    for tex in &lib.textures {
        let Ok(rgba) = linear_to_rgba(tex.format, tex.width, tex.height, &tex.base_mip_linear) else {
            continue;
        };
        let Ok(png) = (DecodedImage { width: tex.width, height: tex.height, rgba }).to_png() else {
            continue;
        };
        out.push((map_label(&tex.name, &stem, single), png));
    }
    out
}

/// Write the head / hair / facial-hair assets (and shared textures) for one
/// already-decoded head under `base`. Reused by the player export here and by the
/// Phase-4 staff/generic sweep.
pub fn write_head_group(
    base: &str,
    origin: &str,
    asset: &PlayerAsset,
    texture_paths: &[String],
    source: &str,
    out_dir: &Path,
    opts: &CatalogOptions,
    summary: &mut HeadCatalogSummary,
) {
    let game = &opts.game_tag;
    let part = partition(asset);

    // Textures once, shared by the head + hair entries.
    let mut texture_rel: Vec<String> = Vec::new();
    if opts.textures {
        for src in texture_paths {
            for (label, png) in decode_texture_pngs(Path::new(src)) {
                let fname = format!("{base}_{label}_{game}.png");
                let dst = out_dir.join("textures").join(&fname);
                match write_out(&dst, &png) {
                    Ok(n) => {
                        summary.textures_written += 1;
                        summary.bytes_written += n;
                        texture_rel.push(format!("textures/{fname}"));
                    }
                    Err(e) => summary.failures.push(format!("{base} texture {label}: {e}")),
                }
            }
        }
    }

    // (class, indices) for each mesh asset to write.
    let head_idx = head_asset_indices(&part);
    let groups: [(&str, &[usize]); 3] = [
        ("head", &head_idx),
        ("hair", &part.hair),
        ("facialhair", &part.facial_hair),
    ];
    for (class, idx) in groups {
        if idx.is_empty() {
            continue;
        }
        let sub = subset(asset, idx);
        let glb = tdb_rx2::playermesh::write_glb_geometry_only(&sub);
        let fname = format!("{base}_{class}_{game}.glb");
        let dst = out_dir.join("heads").join(&fname);
        match write_out(&dst, &glb) {
            Ok(n) => {
                summary.bytes_written += n;
                match class {
                    "head" => summary.heads_written += 1,
                    "hair" => summary.hair_written += 1,
                    _ => summary.facial_hair_written += 1,
                }
                summary.entries.push(CatalogEntry {
                    name: format!("{base}_{class}_{game}"),
                    origin: origin.to_string(),
                    class: class.to_string(),
                    game: game.clone(),
                    mesh_file: format!("heads/{fname}"),
                    submeshes: idx.iter().map(|&i| asset.submeshes[i].name.clone()).collect(),
                    textures: texture_rel.clone(),
                    source: source.to_string(),
                });
            }
            Err(e) => summary.failures.push(format!("{base} {class}: {e}")),
        }
    }
}

/// Export the appearance catalog for every player who owns a head (Phase-1 gate:
/// `mesh_path.is_some()`). Non-owning players (shared/pool/no head) are left for
/// the Phase-4 generic/staff sweep.
pub fn export_player_head_catalog(
    players: &[PlayerAssets],
    out_dir: &Path,
    opts: &CatalogOptions,
) -> Result<HeadCatalogSummary> {
    let mut summary = HeadCatalogSummary::default();
    let mut seen: HashSet<String> = HashSet::new();

    for p in players.iter().filter(|p| p.mesh_path.is_some()) {
        let Some(slug) = p.slug.as_deref() else {
            summary.skipped_no_base += 1;
            continue;
        };
        // Clean base, disambiguated with the full slug (which carries the date) if
        // two players collide on the plain name.
        let plain = catalog_base_from_slug(slug);
        let base = if seen.insert(plain.clone()) {
            plain
        } else {
            seen.insert(slug.to_string());
            slug.to_string()
        };

        let mesh = p.mesh_path.as_deref().unwrap();
        let asset = match read_asset_file(Path::new(mesh))
            .and_then(|b| parse_head_mesh(Path::new(mesh), &b, true))
        {
            Ok(a) => a,
            Err(e) => {
                summary.failures.push(format!("{}: mesh — {e}", p.full_name));
                continue;
            }
        };
        write_head_group(&base, "player", &asset, &p.texture_paths, mesh, out_dir, opts, &mut summary);
    }
    Ok(summary)
}

/// Write a **hair-only** asset (a standalone hair mesh file, e.g. NHL 08/09
/// `hair_{id}.rx2` or PS4 `hair_*.mcd`) — every submesh is hair, so it becomes one
/// `{base}_hair_{game}.glb` with no head/facial split.
fn write_hair_only(
    base: &str,
    origin: &str,
    asset: &PlayerAsset,
    source: &str,
    out_dir: &Path,
    opts: &CatalogOptions,
    summary: &mut HeadCatalogSummary,
) {
    if asset.submeshes.is_empty() {
        return;
    }
    let game = &opts.game_tag;
    let glb = tdb_rx2::playermesh::write_glb_geometry_only(asset);
    let fname = format!("{base}_hair_{game}.glb");
    let dst = out_dir.join("heads").join(&fname);
    match write_out(&dst, &glb) {
        Ok(n) => {
            summary.hair_written += 1;
            summary.bytes_written += n;
            summary.entries.push(CatalogEntry {
                name: format!("{base}_hair_{game}"),
                origin: origin.to_string(),
                class: "hair".to_string(),
                game: game.clone(),
                mesh_file: format!("heads/{fname}"),
                submeshes: asset.submeshes.iter().map(|s| s.name.clone()).collect(),
                textures: Vec::new(),
                source: source.to_string(),
            });
        }
        Err(e) => summary.failures.push(format!("{base} hair: {e}")),
    }
}

/// The catalog name prefix for a swept file's role. A shared *player*-folder head
/// (not captured as a bespoke player's own) is generic, so `Player`/`Generic` both
/// map to `generic`.
fn origin_for_role(role: crate::head_census::Role) -> &'static str {
    use crate::head_census::Role;
    match role {
        Role::Coach => "coach",
        Role::Referee => "referee",
        Role::Trainer => "trainer",
        Role::Commissioner => "commissioner",
        Role::Timekeeper => "timekeeper",
        Role::Player | Role::Generic | Role::Unknown => "generic",
    }
}

/// Sweep the rendering tree for every head/hair mesh **not** already written as a
/// bespoke player's own — generic/pool heads, referees, coaches, trainers, and any
/// unreferenced head file — and add them to the catalog under `{role}_{id}` names
/// (`generic_60046_head_{game}`, `referee_31000_head_{game}`, `coach_100_hair_{game}`).
/// Geometry-only: staff/generic texture resolution differs per role and is a
/// follow-up; bespoke players already carry textures from the player export.
///
/// `exclude_sources` are the canonicalized mesh paths already written by the player
/// export, so a bespoke head sitting in the shared `player/` folder isn't captured
/// twice.
///
/// Known limitation: a few **rigid** `.rx2` hair meshes (e.g. NHL 09 coach hair
/// style variants `hair_*_aa/ab.rx2`) carry no AnimationSkin chunk, which the mesh
/// parser currently requires; they're reported as failures rather than captured
/// (a rigid-`.rx2` parse path is a follow-up). Everything decodable is captured.
pub fn sweep_nonplayer_heads(
    roots: &crate::assets::AssetRoots,
    exclude_sources: &HashSet<String>,
    out_dir: &Path,
    opts: &CatalogOptions,
    summary: &mut HeadCatalogSummary,
) {
    use crate::head_census::{appearance_folders, classify, Part};

    let platform = roots.platform();
    let base_dir = if platform == crate::Platform::Ps4 {
        Path::new(&roots.rendering).join("characters")
    } else {
        Path::new(&roots.rendering).to_path_buf()
    };
    let canon = |p: &Path| {
        std::fs::canonicalize(p)
            .map(|c| c.to_string_lossy().into_owned())
            .unwrap_or_else(|_| p.to_string_lossy().into_owned())
    };
    // Fallback counter for files whose name carries no id.
    let mut anon = 0u64;

    for folder in appearance_folders(platform) {
        let dir = base_dir.join(folder);
        let Ok(rd) = std::fs::read_dir(&dir) else {
            continue;
        };
        // Collect (path) for this folder and any one-level-nested per-head folders
        // (NHL 07 `player/player_{id}/head_{id}.rx2`).
        let mut files: Vec<PathBuf> = Vec::new();
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                if let Ok(inner) = std::fs::read_dir(&p) {
                    files.extend(inner.flatten().map(|e| e.path()).filter(|p| p.is_file()));
                }
            } else {
                files.push(p);
            }
        }
        files.sort();

        for path in files {
            let Some(fname) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };
            let ext = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if !matches!(ext.to_ascii_lowercase().as_str(), "rx2" | "mcd") {
                continue;
            }
            let entry = classify(folder, fname, platform);
            if entry.is_texture || !matches!(entry.part, Part::Head | Part::Hair) {
                continue;
            }
            if exclude_sources.contains(&canon(&path)) {
                continue;
            }
            let origin = origin_for_role(entry.role);
            let id = entry.id.unwrap_or_else(|| {
                anon += 1;
                900_000 + anon
            });
            let base = format!("{origin}_{id}");
            let src = path.to_string_lossy().into_owned();
            let asset = match read_asset_file(&path)
                .and_then(|b| parse_head_mesh(&path, &b, true))
            {
                Ok(a) => a,
                Err(e) => {
                    summary.failures.push(format!("{src}: mesh — {e}"));
                    continue;
                }
            };
            match entry.part {
                Part::Hair => write_hair_only(&base, origin, &asset, &src, out_dir, opts, summary),
                // A head file: split head / hair / facial-hair. Geometry-only here.
                _ => write_head_group(&base, origin, &asset, &[], &src, out_dir, opts, summary),
            }
        }
    }
}

/// The full appearance catalog: bespoke player heads (with textures) plus the
/// non-player sweep (generic / staff / unreferenced heads + hair). The sweep skips
/// any file already written for a bespoke player.
pub fn export_full_catalog(
    players: &[PlayerAssets],
    roots: &crate::assets::AssetRoots,
    out_dir: &Path,
    opts: &CatalogOptions,
) -> Result<HeadCatalogSummary> {
    let mut summary = export_player_head_catalog(players, out_dir, opts)?;
    let exclude: HashSet<String> = summary
        .entries
        .iter()
        .map(|e| {
            std::fs::canonicalize(&e.source)
                .map(|c| c.to_string_lossy().into_owned())
                .unwrap_or_else(|_| e.source.clone())
        })
        .collect();
    sweep_nonplayer_heads(roots, &exclude, out_dir, opts, &mut summary);
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_base_strips_the_trailing_date() {
        assert_eq!(catalog_base_from_slug("ovechkin_alexander_17_09_1985"), "ovechkin_alexander");
        assert_eq!(catalog_base_from_slug("crosby_sidney_07_08_1987"), "crosby_sidney");
        // No trailing date → unchanged.
        assert_eq!(catalog_base_from_slug("mystery_head"), "mystery_head");
        // A hyphen-free single-token name with a date still strips.
        assert_eq!(catalog_base_from_slug("cher_x_01_02_2000"), "cher_x");
    }

    #[test]
    fn map_label_strips_ids_and_raster_suffix() {
        assert_eq!(map_label("head_67_cm.Raster", "texlib_67", false), "head_cm");
        assert_eq!(map_label("hair_512_am.Raster", "texlib_512", false), "hair_am");
        // Generic single-raster name falls back to the file stem's part+map.
        assert_eq!(map_label("texture_0000", "head_10000_cm", true), "head_cm");
    }

    fn vtx() -> tdb_rx2::playermesh::Vertex {
        tdb_rx2::playermesh::Vertex {
            position: [0.0; 3],
            normal: [0.0; 3],
            uv: [0.0; 2],
            joints: [0; 4],
            weights: [0.0; 4],
        }
    }

    fn sm(name: &str) -> tdb_rx2::playermesh::Submesh {
        use tdb_rx2::playermesh::{PositionFormat, Submesh, UvFormat};
        Submesh {
            name: name.to_string(),
            stride: 0,
            position_format: PositionFormat::Float,
            uv_format: UvFormat::Float,
            has_blend_indices: false,
            has_blend_weights: false,
            // One degenerate triangle so the GLB writer has geometry to emit.
            vertices: vec![vtx(), vtx(), vtx()],
            triangles: vec![[0, 1, 2]],
        }
    }

    #[test]
    fn write_group_emits_head_and_hair_not_facial_when_absent() {
        use tdb_rx2::playermesh::{BonePalette, PlayerAsset};
        let asset = PlayerAsset {
            submeshes: vec![sm("head_.Mesh"), sm("eyel_.Mesh"), sm("haircap_.Mesh"), sm("cardsa_.Mesh")],
            skin: BonePalette { matrices: Vec::new() },
        };
        let tmp = std::env::temp_dir().join(format!("hc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let mut summary = HeadCatalogSummary::default();
        write_head_group(
            "ovechkin_alexander",
            "player",
            &asset,
            &[],
            "player_1083.rx2",
            &tmp,
            &CatalogOptions { game_tag: "nhllegacy".into(), textures: false },
            &mut summary,
        );
        assert_eq!(summary.heads_written, 1);
        assert_eq!(summary.hair_written, 1);
        assert_eq!(summary.facial_hair_written, 0, "no facial-hair geometry present");
        assert!(tmp.join("heads/ovechkin_alexander_head_nhllegacy.glb").is_file());
        assert!(tmp.join("heads/ovechkin_alexander_hair_nhllegacy.glb").is_file());
        assert!(!tmp.join("heads/ovechkin_alexander_facialhair_nhllegacy.glb").exists());
        // The hair entry carries only the two hair submeshes (haircap + cards merged).
        let hair = summary.entries.iter().find(|e| e.class == "hair").unwrap();
        assert_eq!(hair.submeshes, vec!["haircap_.Mesh", "cardsa_.Mesh"]);
        // The head entry carries head-core (head + eye) — hair is a separate asset.
        let head = summary.entries.iter().find(|e| e.class == "head").unwrap();
        assert!(head.submeshes.contains(&"head_.Mesh".to_string()));
        std::fs::remove_dir_all(&tmp).ok();
    }

    /// End-to-end on a real title (mount-gated): scan the Legacy anchor + mount,
    /// export the catalog for the bespoke players, and assert real head + hair GLBs
    /// and PNG textures land with the catalog naming. Legacy heads bundle hair as a
    /// `haircap_` submesh, so every full head should also yield a hair asset.
    #[test]
    fn legacy_catalog_export_writes_head_hair_and_png_textures() {
        use crate::assets::{scan_player_assets, AssetRoots};
        use crate::Platform;
        use tdb_core::TdbFile;
        use tdb_defs::FieldMetaIndex;

        let rendering = std::path::Path::new(
            r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering",
        );
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("anchors")
            .join("xbox360")
            .join("nhllegacy")
            .join("databases");
        if !rendering.join("player").is_dir() || !base.join("nhlng.db").is_file() {
            eprintln!("skipping: Legacy mount/anchor absent");
            return;
        }
        let db_bytes = std::fs::read(base.join("nhlng.db")).unwrap();
        let db = TdbFile::parse(&db_bytes).expect("parse anchor");
        let meta =
            FieldMetaIndex::from_xml(&std::fs::read_to_string(base.join("nhlng-meta.xml")).unwrap())
                .expect("meta");
        let rendering_str = rendering.to_string_lossy().into_owned();
        let roots = AssetRoots::resolve(std::path::Path::new("/unused"), Platform::Xbox)
            .with_overrides(Some(&rendering_str), None);
        let players = scan_player_assets(&db, &meta, &roots).expect("scan");

        // A handful of bespoke players is enough to prove the pipeline.
        let sample: Vec<_> = players
            .into_iter()
            .filter(|p| p.mesh_path.is_some() && p.slug.is_some())
            .take(5)
            .collect();
        assert!(!sample.is_empty(), "no bespoke players resolved on Legacy");

        let tmp = std::env::temp_dir().join(format!("hc_e2e_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let summary = export_player_head_catalog(
            &sample,
            &tmp,
            &CatalogOptions { game_tag: "nhllegacy".into(), textures: true },
        )
        .expect("catalog export");

        assert_eq!(summary.heads_written, sample.len(), "one head per bespoke player");
        assert!(summary.hair_written > 0, "Legacy heads bundle hair — expected hair assets");
        assert!(summary.textures_written > 0, "textures should decode to PNG");
        assert!(summary.failures.is_empty(), "unexpected failures: {:?}", summary.failures);

        // Every written GLB is a real binary glTF and every texture a real PNG.
        for e in &summary.entries {
            let p = tmp.join(&e.mesh_file);
            let bytes = std::fs::read(&p).unwrap_or_else(|_| panic!("missing {}", e.mesh_file));
            assert!(bytes.len() > 100, "{} too small", e.mesh_file);
            assert_eq!(&bytes[..4], b"glTF", "{} is not a GLB", e.mesh_file);
            assert!(e.name.ends_with("_nhllegacy"), "{} misnamed", e.name);
            for t in &e.textures {
                let tb = std::fs::read(tmp.join(t)).unwrap_or_else(|_| panic!("missing {t}"));
                assert_eq!(&tb[..8], b"\x89PNG\r\n\x1a\n", "{t} is not a PNG");
            }
        }
        eprintln!(
            "Legacy catalog: {} heads, {} hair, {} textures written",
            summary.heads_written, summary.hair_written, summary.textures_written
        );
        std::fs::remove_dir_all(&tmp).ok();
    }

    /// Full catalog (players + non-player sweep) on NHL 09 (mount-gated): it has a
    /// `coachhead/` folder (staff head + hair meshes) and separate `hair_*` player
    /// files, so the sweep should capture generic + coach + hair assets alongside
    /// the bespoke players — and never write a bespoke player's file twice.
    #[test]
    fn nhl09_full_catalog_sweeps_generic_and_staff_heads() {
        use crate::assets::{scan_player_assets, AssetRoots};
        use crate::Platform;
        use std::collections::HashSet;
        use tdb_core::TdbFile;
        use tdb_defs::FieldMetaIndex;

        let rendering =
            std::path::Path::new(r"X:\Games\Emulation\Roms\XBOX360\NHL 09\NHL 09 (USA)\rendering");
        let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("anchors")
            .join("xbox360")
            .join("nhl09")
            .join("databases");
        if !rendering.join("player").is_dir() || !base.join("nhlng.db").is_file() {
            eprintln!("skipping: NHL 09 mount/anchor absent");
            return;
        }
        let db_bytes = std::fs::read(base.join("nhlng.db")).unwrap();
        let db = TdbFile::parse(&db_bytes).expect("parse anchor");
        let meta =
            FieldMetaIndex::from_xml(&std::fs::read_to_string(base.join("nhlng-meta.xml")).unwrap())
                .expect("meta");
        let rendering_str = rendering.to_string_lossy().into_owned();
        let roots = AssetRoots::resolve(std::path::Path::new("/unused"), Platform::Xbox)
            .with_overrides(Some(&rendering_str), None);
        let players = scan_player_assets(&db, &meta, &roots).expect("scan");

        let tmp = std::env::temp_dir().join(format!("hc_sweep_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        let summary = export_full_catalog(
            &players,
            &roots,
            &tmp,
            &CatalogOptions { game_tag: "nhl09".into(), textures: false },
        )
        .expect("full catalog");

        let origins: HashSet<&str> = summary.entries.iter().map(|e| e.origin.as_str()).collect();
        assert!(origins.contains("player"), "bespoke players captured");
        assert!(origins.contains("generic"), "generic/shared heads swept");
        assert!(origins.contains("coach"), "coach heads swept from coachhead/");
        // Separate hair files are captured as hair assets.
        assert!(summary.hair_written > 0, "hair assets written");
        // Names are unique (no file overwritten by the sweep).
        let names: Vec<&str> = summary.entries.iter().map(|e| e.name.as_str()).collect();
        let uniq: HashSet<&str> = names.iter().copied().collect();
        assert_eq!(names.len(), uniq.len(), "duplicate catalog asset names written");
        // Naming prefixes match the convention.
        for e in &summary.entries {
            let ok = e.name.starts_with("coach_")
                || e.name.starts_with("generic_")
                || e.name.starts_with("referee_")
                || e.name.starts_with("trainer_")
                || e.name.starts_with("commissioner_")
                || e.origin == "player";
            assert!(ok, "unexpected name {}", e.name);
            assert!(e.name.ends_with("_nhl09"));
        }
        let n_player = summary.entries.iter().filter(|e| e.origin == "player").count();
        let n_generic = summary.entries.iter().filter(|e| e.origin == "generic").count();
        let n_coach = summary.entries.iter().filter(|e| e.origin == "coach").count();
        eprintln!(
            "NHL09 full catalog: {} entries (player={n_player}, generic={n_generic}, coach={n_coach}); \
             heads={}, hair={}, failures={}",
            summary.entries.len(),
            summary.heads_written,
            summary.hair_written,
            summary.failures.len()
        );
        std::fs::remove_dir_all(&tmp).ok();
    }
}
