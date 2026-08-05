//! Hockey Shop — the per-slot equipment "store": a disk-backed library of
//! equipment textures that lives OUTSIDE the game install, under an app-managed
//! root the frontend supplies (Tauri's app-data dir). Ported from the prior app's
//! Hockey Shop (clean-room port authorized for this feature).
//!
//! ```text
//! <store_root>/<store_dir>/
//!     store.json          ← manifest (one entry per item)
//!     game-gloves-7.png   ← extracted game texture
//!     my-custom-pads.png  ← operator import
//! ```
//!
//! Two entry sources: `"game"` — a representative colour texture extracted from the
//! slot's installed assets by [`sync`] (idempotent); and `"import"` — an operator
//! PNG via [`import`]. Textures are normalised to PNG on ingest. Install-to-game
//! (writing a store texture back into a texlib) lives in the install path.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::dto::{HockeyShopItemDto, HockeyShopSyncSummaryDto};
use crate::error::ApiError;

/// `texlib_999` (and siblings) is the shared sentinel/placeholder asset, not a real
/// model — skip it during the game sync so it never lands as a bogus store entry.
const SENTINEL_ASSET_ID: u32 = 999;

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct StoreManifest {
    #[serde(default)]
    items: Vec<HockeyShopItemDto>,
}

// ── Public store operations ─────────────────────────────────────────────────────

/// Every item in a slot's store (manifest order). Missing store dir → empty list.
pub fn list(store_root: &str, slot: &str) -> Result<Vec<HockeyShopItemDto>, ApiError> {
    let dir = store_slot_dir(store_root, slot)?;
    Ok(read_manifest(&dir)?.items)
}

/// Extract a representative colour texture for every game equipment id the store
/// doesn't already carry (named `<Label> <id>`, credited to EA Sports). Idempotent.
pub fn sync(
    rendering: &Path,
    store_root: &str,
    slot: &str,
) -> Result<HockeyShopSyncSummaryDto, ApiError> {
    let spec = spec_for(slot)?;
    let store = ensure_store_slot_dir(store_root, slot)?;
    let mut manifest = read_manifest(&store)?;
    let existing: std::collections::BTreeSet<u32> =
        manifest.items.iter().filter_map(|s| s.game_asset_id).collect();

    let mut summary = HockeyShopSyncSummaryDto {
        added: 0,
        already_present: existing.len() as u32,
        failed: Vec::new(),
        store_dir: store.display().to_string(),
    };

    for asset in studio_domain::list_equipment_slot_assets(rendering, spec) {
        if asset.id == SENTINEL_ASSET_ID || existing.contains(&asset.id) {
            continue;
        }
        let Some((width, height, png)) = decode_representative(rendering, &asset.files) else {
            summary.failed.push(format!("{} {} (no decodable texture)", spec.label, asset.id));
            continue;
        };
        let id = format!("game-{}-{}", spec.key, asset.id);
        let file = format!("{id}.png");
        if let Err(e) = std::fs::write(store.join(&file), &png) {
            summary.failed.push(format!("{file}: {e}"));
            continue;
        }
        manifest.items.push(HockeyShopItemDto {
            id,
            name: format!("{} {}", spec.label, asset.id),
            author: "EA Sports".to_owned(),
            source: "game".to_owned(),
            game_asset_id: Some(asset.id),
            file,
            width,
            height,
            added_unix: now_unix(),
        });
        summary.added += 1;
    }
    // Game extractions in id order, imports (no game id) after them.
    manifest
        .items
        .sort_by_key(|s| (s.game_asset_id.is_none(), s.game_asset_id, s.added_unix));
    write_manifest(&store, &manifest)?;
    Ok(summary)
}

/// Ingest an operator-picked PNG as a new store item (normalised to PNG; slugged id).
pub fn import(
    store_root: &str,
    slot: &str,
    name: &str,
    author: &str,
    image_base64: &str,
) -> Result<HockeyShopItemDto, ApiError> {
    let name = name.trim();
    let author = author.trim();
    if name.is_empty() {
        return Err(ApiError::Workspace("item name must not be empty".to_owned()));
    }
    if author.is_empty() {
        return Err(ApiError::Workspace("author must not be empty".to_owned()));
    }
    spec_for(slot)?;
    use base64::Engine as _;
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(image_base64)
        .map_err(|e| ApiError::Workspace(format!("image_base64 didn't decode: {e}")))?;
    // Decode + re-encode to a normalised PNG (one decode path on read); also yields
    // the true dimensions. Non-PNG input (e.g. raw DDS) isn't supported yet.
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| ApiError::Workspace(format!("only PNG imports are supported ({e})")))?;
    let rgba = img.to_rgba8();
    let (width, height) = (rgba.width(), rgba.height());
    let mut png: Vec<u8> = Vec::new();
    image::DynamicImage::ImageRgba8(rgba)
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| ApiError::Workspace(format!("PNG encode failed: {e}")))?;

    let store = ensure_store_slot_dir(store_root, slot)?;
    let mut manifest = read_manifest(&store)?;
    let id = unique_slug(name, &manifest.items);
    let file = format!("{id}.png");
    std::fs::write(store.join(&file), &png)
        .map_err(|e| ApiError::Workspace(format!("writing {file}: {e}")))?;

    let entry = HockeyShopItemDto {
        id,
        name: name.to_owned(),
        author: author.to_owned(),
        source: "import".to_owned(),
        game_asset_id: None,
        file,
        width,
        height,
        added_unix: now_unix(),
    };
    manifest.items.push(entry.clone());
    write_manifest(&store, &manifest)?;
    Ok(entry)
}

/// Remove one store item: drop its manifest entry and delete its PNG. A missing PNG
/// is tolerated (the entry still clears).
pub fn delete(store_root: &str, slot: &str, entry_id: &str) -> Result<(), ApiError> {
    let dir = store_slot_dir(store_root, slot)?;
    let mut manifest = read_manifest(&dir)?;
    let idx = manifest
        .items
        .iter()
        .position(|s| s.id == entry_id)
        .ok_or_else(|| ApiError::Workspace(format!("no store item with id `{entry_id}`")))?;
    let removed = manifest.items.remove(idx);
    let path = dir.join(&removed.file);
    if path.is_file() {
        std::fs::remove_file(&path)
            .map_err(|e| ApiError::Workspace(format!("deleting {}: {e}", path.display())))?;
    }
    write_manifest(&dir, &manifest)?;
    Ok(())
}

/// One store item's PNG as a `data:image/png;base64,…` URL for the frontend `<img>`.
pub fn load_texture_data_url(
    store_root: &str,
    slot: &str,
    entry_id: &str,
) -> Result<String, ApiError> {
    let dir = store_slot_dir(store_root, slot)?;
    let manifest = read_manifest(&dir)?;
    let entry = manifest
        .items
        .iter()
        .find(|s| s.id == entry_id)
        .ok_or_else(|| ApiError::Workspace(format!("no store item with id `{entry_id}`")))?;
    let path = dir.join(&entry.file);
    let bytes = std::fs::read(&path)
        .map_err(|e| ApiError::Workspace(format!("reading {}: {e}", path.display())))?;
    use base64::Engine as _;
    let b64 = base64::engine::general_purpose::STANDARD.encode(&bytes);
    Ok(format!("data:image/png;base64,{b64}"))
}

// ── Internals ───────────────────────────────────────────────────────────────────

fn spec_for(slot: &str) -> Result<&'static studio_domain::EquipmentSlotSpec, ApiError> {
    studio_domain::slot_spec(slot)
        .ok_or_else(|| ApiError::Workspace(format!("unknown equipment slot `{slot}`")))
}

/// `<store_root>/<slot store_dir>` — validated but NOT created (reads tolerate a
/// missing store). The frontend passes an app-managed `store_root`.
fn store_slot_dir(store_root: &str, slot: &str) -> Result<PathBuf, ApiError> {
    let spec = spec_for(slot)?;
    let trimmed = store_root.trim();
    if trimmed.is_empty() {
        return Err(ApiError::Workspace("the store root is not set".to_owned()));
    }
    Ok(PathBuf::from(trimmed).join(spec.store_dir))
}

fn ensure_store_slot_dir(store_root: &str, slot: &str) -> Result<PathBuf, ApiError> {
    let dir = store_slot_dir(store_root, slot)?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| ApiError::Workspace(format!("creating {}: {e}", dir.display())))?;
    Ok(dir)
}

/// Pick the best colour texture among an id's render files and decode it to a PNG.
/// `_cm` maps are tried first, then `_dm`, then plain texlibs, then normal/spec maps.
fn decode_representative(rendering: &Path, rels: &[String]) -> Option<(u32, u32, Vec<u8>)> {
    let mut ordered: Vec<&String> = rels.iter().collect();
    ordered.sort_by(|a, b| color_rank(a).cmp(&color_rank(b)).then_with(|| a.cmp(b)));
    ordered.into_iter().find_map(|rel| {
        let path = join_rel(rendering, rel);
        let (_container, img) = studio_domain::decode_texture_file(&path, None).ok()?;
        let png = img.to_png().ok()?;
        Some((img.width, img.height, png))
    })
}

/// Rank a render file by how likely it carries the COLOR map (lower = better).
fn color_rank(rel: &str) -> u8 {
    let stem = rel.rsplit('/').next().unwrap_or(rel).to_ascii_lowercase();
    if stem.contains("_cm") {
        0
    } else if stem.contains("_dm") {
        1
    } else if !stem.contains("_nm") && !stem.contains("_sm") && !stem.contains("_am") {
        2
    } else {
        3
    }
}

fn join_rel(root: &Path, rel: &str) -> PathBuf {
    let mut p = root.to_path_buf();
    for part in rel.split('/') {
        p.push(part);
    }
    p
}

fn manifest_path(dir: &Path) -> PathBuf {
    dir.join("store.json")
}

fn read_manifest(dir: &Path) -> Result<StoreManifest, ApiError> {
    let path = manifest_path(dir);
    if !path.is_file() {
        return Ok(StoreManifest::default());
    }
    let bytes = std::fs::read(&path)
        .map_err(|e| ApiError::Workspace(format!("reading {}: {e}", path.display())))?;
    serde_json::from_slice(&bytes)
        .map_err(|e| ApiError::Workspace(format!("{} is not a valid store manifest: {e}", path.display())))
}

fn write_manifest(dir: &Path, manifest: &StoreManifest) -> Result<(), ApiError> {
    let path = manifest_path(dir);
    let json = serde_json::to_vec_pretty(manifest)
        .map_err(|e| ApiError::Workspace(format!("manifest serialize: {e}")))?;
    std::fs::write(&path, json)
        .map_err(|e| ApiError::Workspace(format!("writing {}: {e}", path.display())))
}

/// Kebab slug of `name`, deduped against existing ids with a `-2`, `-3`, … suffix.
fn unique_slug(name: &str, existing: &[HockeyShopItemDto]) -> String {
    let base: String = {
        let mut out = String::with_capacity(name.len());
        let mut last_dash = true;
        for c in name.chars() {
            let c = c.to_ascii_lowercase();
            if c.is_ascii_alphanumeric() {
                out.push(c);
                last_dash = false;
            } else if !last_dash {
                out.push('-');
                last_dash = true;
            }
        }
        let trimmed = out.trim_end_matches('-').to_owned();
        if trimmed.is_empty() {
            "item".to_owned()
        } else {
            trimmed
        }
    };
    let taken = |candidate: &str| existing.iter().any(|s| s.id == candidate);
    if !taken(&base) {
        return base;
    }
    (2..)
        .map(|n| format!("{base}-{n}"))
        .find(|c| !taken(c))
        .unwrap_or(base)
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str) -> HockeyShopItemDto {
        HockeyShopItemDto {
            id: id.to_owned(),
            name: id.to_owned(),
            author: "x".to_owned(),
            source: "import".to_owned(),
            game_asset_id: None,
            file: format!("{id}.png"),
            width: 4,
            height: 4,
            added_unix: 0,
        }
    }

    #[test]
    fn slugs_are_kebab_and_deduped() {
        assert_eq!(unique_slug("CCM Ribcore 40K", &[]), "ccm-ribcore-40k");
        assert_eq!(unique_slug("  Wood (vintage)!! ", &[]), "wood-vintage");
        assert_eq!(unique_slug("円円円", &[]), "item");
        assert_eq!(unique_slug("Dupe", &[item("dupe")]), "dupe-2");
    }

    #[test]
    fn color_rank_orders_cm_first() {
        assert!(color_rank("glove/glove_7_cm.rx2") < color_rank("glove/glove_7_dm.rx2"));
        assert!(color_rank("glove/glove_7_dm.rx2") < color_rank("skate/texlib_3.rx2"));
        assert!(color_rank("skate/texlib_3.rx2") < color_rank("glove/glove_7_nm.rx2"));
    }

    #[test]
    fn unknown_slot_errors() {
        assert!(list("/tmp", "elbow-pads").is_err());
    }
}
