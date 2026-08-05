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

use crate::dto::{
    HockeyShopItemDto, HockeyShopSyncSummaryDto, InstallStoreItemApplyResponseDto,
    InstallStoreItemPlanDto, InstallStoreItemRequestDto,
};
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

// ── Install-to-game (write path) ─────────────────────────────────────────────────

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct InstallLedger {
    #[serde(default)]
    installed: Vec<InstalledLedgerEntry>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct InstalledLedgerEntry {
    equipment_id: u32,
    slot: String,
    entry_id: String,
    name: String,
    author: String,
    source: String,
    installed_unix: u64,
}

/// Read-only plan for installing a store item into the game: resolve overwrite vs
/// create, the donor template (create), the destination colour file + texture slot,
/// and any warnings/blocking errors. Xbox `.rx2` only. Touches no files.
pub fn plan_install(
    rendering: &Path,
    store_root: &str,
    req: &InstallStoreItemRequestDto,
) -> Result<InstallStoreItemPlanDto, ApiError> {
    let spec = spec_for(&req.slot)?;
    let store = store_slot_dir(store_root, &req.slot)?;
    let manifest = read_manifest(&store)?;
    let entry = manifest
        .items
        .iter()
        .find(|s| s.id == req.entry_id)
        .ok_or_else(|| ApiError::Workspace(format!("no store item with id `{}`", req.entry_id)))?
        .clone();

    let mut warnings: Vec<String> = Vec::new();
    let mut blocking_errors: Vec<String> = Vec::new();

    if req.target_id > spec.max_id {
        blocking_errors.push(format!(
            "target id {} is above the {}.{} ceiling ({}). Widening the field would \
             change the record bit layout and break the game binary.",
            req.target_id, spec.table, spec.field, spec.max_id
        ));
    }

    let assets = studio_domain::list_equipment_slot_assets(rendering, spec);
    let target_asset = assets.iter().find(|a| a.id == req.target_id);
    let mode = if target_asset.is_some() { "overwrite" } else { "create" };

    let mut target_existing_label: Option<String> = None;
    let mut donor_id: Option<u32> = None;
    let mut target_color_file: Option<String> = None;
    let mut cloned_files: Vec<String> = Vec::new();

    // The file whose colour slot we inspect for index + dims: the target's own file
    // (overwrite) or the donor's (create — the clone is byte-identical).
    let color_source_rel: Option<String> = match target_asset {
        Some(asset) => {
            let label = format!("{} {}", spec.label, asset.id);
            let color = pick_color_rel(&asset.files);
            if color.is_none() {
                blocking_errors
                    .push(format!("id {} ships no colour texture to overwrite.", req.target_id));
            } else {
                warnings.push(format!(
                    "id {} already ships \"{label}\" — installing overwrites its colour \
                     texture in place (a .bak is written first).",
                    req.target_id
                ));
            }
            target_existing_label = Some(label);
            target_color_file = color.clone();
            color
        }
        None => {
            let chosen = req.donor_id.or_else(|| {
                entry
                    .game_asset_id
                    .filter(|gid| assets.iter().any(|a| a.id == *gid))
                    .or_else(|| assets.first().map(|a| a.id))
            });
            match chosen.and_then(|d| assets.iter().find(|a| a.id == d)) {
                None => {
                    blocking_errors.push(format!(
                        "the {} slot ships no assets to clone as a template for the new id.",
                        spec.label.to_lowercase()
                    ));
                    None
                }
                Some(donor) => {
                    donor_id = Some(donor.id);
                    cloned_files = donor
                        .files
                        .iter()
                        .filter_map(|r| predict_target_rel(spec, r, req.target_id))
                        .collect();
                    let donor_color = pick_color_rel(&donor.files);
                    match &donor_color {
                        None => blocking_errors
                            .push(format!("donor id {} has no colour texture.", donor.id)),
                        Some(dc) => {
                            target_color_file = predict_target_rel(spec, dc, req.target_id);
                            warnings.push(format!(
                                "id {} is empty — its container is created by cloning donor id \
                                 {} ({} file(s)), then your texture is spliced in.",
                                req.target_id,
                                donor.id,
                                cloned_files.len()
                            ));
                        }
                    }
                    donor_color
                }
            }
        }
    };

    let mut target_texture_index = 0u32;
    let mut target_width = 0u32;
    let mut target_height = 0u32;
    if let Some(rel) = color_source_rel.as_ref() {
        match studio_domain::color_map_slot(&join_rel(rendering, rel)) {
            Some((idx, w, h)) => {
                target_texture_index = idx as u32;
                target_width = w;
                target_height = h;
                if entry.width != w || entry.height != h {
                    warnings.push(format!(
                        "store texture is {}×{}; the target slot is {w}×{h} — it will be \
                         resized to fit on install.",
                        entry.width, entry.height
                    ));
                }
            }
            None => blocking_errors.push(format!(
                "{rel} is not a decodable Xbox texlib — install is Xbox-only for now."
            )),
        }
    }

    warnings.push(format!(
        "Reminder: to make a player use this {}, set {}.{} = {} on their record \
         (their Equipment tab). Install never edits the .db.",
        spec.label.to_lowercase(),
        spec.table,
        spec.field,
        req.target_id
    ));

    Ok(InstallStoreItemPlanDto {
        slot: req.slot.clone(),
        entry_id: entry.id.clone(),
        entry_name: entry.name.clone(),
        target_id: req.target_id,
        mode: mode.to_owned(),
        target_existing_label,
        donor_id,
        target_color_file,
        target_texture_index,
        target_width,
        target_height,
        cloned_files,
        warnings,
        blocking_errors,
    })
}

/// Install a store item into the game. Create mode clones the donor's container into
/// the target id first; both modes then splice the store texture (resized) into the
/// target's colour map. Writes a `.bak` first. Fails fast on plan blocking errors.
pub fn apply_install(
    rendering: &Path,
    store_root: &str,
    req: &InstallStoreItemRequestDto,
) -> Result<InstallStoreItemApplyResponseDto, ApiError> {
    let spec = spec_for(&req.slot)?;
    let store = store_slot_dir(store_root, &req.slot)?;
    let plan = plan_install(rendering, store_root, req)?;
    if !plan.blocking_errors.is_empty() {
        return Err(ApiError::Workspace(format!(
            "install plan has blocking error(s); refusing to apply: {}",
            plan.blocking_errors.join("; ")
        )));
    }
    let target_color_file = plan
        .target_color_file
        .clone()
        .ok_or_else(|| ApiError::Workspace("install plan resolved no target colour file".into()))?;
    let write_backup = req.write_backup.unwrap_or(true);
    let mut files_written: Vec<String> = Vec::new();

    // Create mode: byte-clone every donor file into the target id's filenames first.
    if plan.mode == "create" {
        let donor = plan
            .donor_id
            .ok_or_else(|| ApiError::Workspace("create-mode plan resolved no donor id".into()))?;
        let assets = studio_domain::list_equipment_slot_assets(rendering, spec);
        let donor_asset = assets
            .iter()
            .find(|a| a.id == donor)
            .ok_or_else(|| ApiError::Workspace(format!("donor id {donor} vanished")))?;
        for src_rel in &donor_asset.files {
            let Some(dest_rel) = predict_target_rel(spec, src_rel, req.target_id) else {
                continue;
            };
            let src_abs = join_rel(rendering, src_rel);
            let dest_abs = join_rel(rendering, &dest_rel);
            let bytes = std::fs::read(&src_abs)
                .map_err(|e| ApiError::Workspace(format!("reading {src_rel}: {e}")))?;
            atomic_write_with_backup(&dest_abs, &bytes, write_backup)?;
            files_written.push(dest_rel);
        }
    } else if write_backup {
        // Overwrite: snapshot the target colour file before splicing.
        let target = join_rel(rendering, &target_color_file);
        backup_once(&target)?;
    }

    // Splice the store texture (resized to the slot) into the target colour map.
    let rgba = store_image_resized_rgba(&store, &plan.entry_id, plan.target_width, plan.target_height)?;
    let target_abs = join_rel(rendering, &target_color_file);
    let target_bytes = std::fs::read(&target_abs)
        .map_err(|e| ApiError::Workspace(format!("reading {target_color_file}: {e}")))?;
    let write = studio_domain::replace_texture_in_file(
        target_bytes,
        &rgba,
        plan.target_width,
        plan.target_height,
        Some(plan.target_texture_index as usize),
    )
    .map_err(ApiError::Domain)?;
    std::fs::write(&target_abs, &write.bytes)
        .map_err(|e| ApiError::Workspace(format!("writing {target_color_file}: {e}")))?;
    if !files_written.contains(&target_color_file) {
        files_written.push(target_color_file);
    }

    // Record the install so the item's name/id stays documented at the target id.
    let ledger_path = read_manifest(&store)
        .ok()
        .and_then(|m| m.items.into_iter().find(|s| s.id == plan.entry_id))
        .and_then(|entry| record_install(rendering, spec, req.target_id, &entry).ok())
        .map(|p| p.display().to_string());

    Ok(InstallStoreItemApplyResponseDto {
        plan,
        files_written,
        ledger_path,
    })
}

/// Predict the cloned destination path for one donor render file at `target_id`
/// (rendering-relative, forward slashes) — mirrors the clone rename.
fn predict_target_rel(
    spec: &studio_domain::EquipmentSlotSpec,
    donor_rel: &str,
    target_id: u32,
) -> Option<String> {
    let file_name = donor_rel.rsplit('/').next().unwrap_or(donor_rel);
    let (stem, ext) = file_name.rsplit_once('.').unwrap_or((file_name, ""));
    let new_stem = studio_domain::rename_stem(spec, stem, target_id)?;
    Some(format!("{}/{new_stem}.{ext}", spec.folder))
}

/// The best colour-map candidate among an id's render files (by [`color_rank`]).
fn pick_color_rel(rels: &[String]) -> Option<String> {
    let mut v: Vec<&String> = rels.iter().collect();
    v.sort_by(|a, b| color_rank(a).cmp(&color_rank(b)).then_with(|| a.cmp(b)));
    v.first().map(|s| (*s).clone())
}

/// Decode a store item's PNG and resize it to the target slot's dimensions.
fn store_image_resized_rgba(
    store: &Path,
    entry_id: &str,
    target_w: u32,
    target_h: u32,
) -> Result<Vec<u8>, ApiError> {
    if target_w == 0 || target_h == 0 {
        return Err(ApiError::Workspace("target texture has zero dimensions".into()));
    }
    let manifest = read_manifest(store)?;
    let entry = manifest
        .items
        .iter()
        .find(|s| s.id == entry_id)
        .ok_or_else(|| ApiError::Workspace(format!("no store item with id `{entry_id}`")))?;
    let path = store.join(&entry.file);
    let bytes = std::fs::read(&path)
        .map_err(|e| ApiError::Workspace(format!("reading {}: {e}", path.display())))?;
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png)
        .map_err(|e| ApiError::Workspace(format!("PNG decode failed: {e}")))?;
    let rgba = img.to_rgba8();
    let resized = if rgba.width() == target_w && rgba.height() == target_h {
        rgba
    } else {
        image::imageops::resize(&rgba, target_w, target_h, image::imageops::FilterType::Lanczos3)
    };
    Ok(resized.into_raw())
}

/// Copy `path` to `path.bak` once (never clobber an existing backup).
fn backup_once(path: &Path) -> Result<(), ApiError> {
    if !path.is_file() {
        return Ok(());
    }
    let mut bak = path.as_os_str().to_owned();
    bak.push(".bak");
    let bak = PathBuf::from(bak);
    if !bak.exists() {
        std::fs::copy(path, &bak)
            .map_err(|e| ApiError::Workspace(format!("backing up {}: {e}", path.display())))?;
    }
    Ok(())
}

/// Write `bytes` to `path`, backing the existing file up to `.bak` first (once).
fn atomic_write_with_backup(path: &Path, bytes: &[u8], backup: bool) -> Result<(), ApiError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ApiError::Workspace(format!("creating {}: {e}", parent.display())))?;
    }
    if backup {
        backup_once(path)?;
    }
    std::fs::write(path, bytes)
        .map_err(|e| ApiError::Workspace(format!("writing {}: {e}", path.display())))
}

/// Upsert an install record into the slot's game-side ledger
/// (`<rendering>/<folder>/hockey_shop_installed.json`) — the latest install per id
/// wins. Invisible to the slot asset scan (it only reads `.rx2`/`.rpsgl`).
fn record_install(
    rendering: &Path,
    spec: &studio_domain::EquipmentSlotSpec,
    target_id: u32,
    entry: &HockeyShopItemDto,
) -> Result<PathBuf, ApiError> {
    let dir = rendering.join(spec.folder);
    std::fs::create_dir_all(&dir)
        .map_err(|e| ApiError::Workspace(format!("creating {}: {e}", dir.display())))?;
    let path = dir.join("hockey_shop_installed.json");
    let mut ledger: InstallLedger = if path.is_file() {
        std::fs::read(&path)
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    } else {
        InstallLedger::default()
    };
    ledger.installed.retain(|e| e.equipment_id != target_id);
    ledger.installed.push(InstalledLedgerEntry {
        equipment_id: target_id,
        slot: spec.key.to_owned(),
        entry_id: entry.id.clone(),
        name: entry.name.clone(),
        author: entry.author.clone(),
        source: entry.source.clone(),
        installed_unix: now_unix(),
    });
    ledger.installed.sort_by_key(|e| e.equipment_id);
    let json = serde_json::to_vec_pretty(&ledger)
        .map_err(|e| ApiError::Workspace(format!("ledger serialize: {e}")))?;
    std::fs::write(&path, json)
        .map_err(|e| ApiError::Workspace(format!("writing {}: {e}", path.display())))?;
    Ok(path)
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
