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

/// Studio's title ids (the catalogue the start screen offers). A request naming anything else is refused rather
/// than silently read with the default file naming (receipt 47).
pub const KNOWN_TITLES: [&str; 17] = [
    "nhl07", "nhl08", "nhl09", "nhl10", "nhl11", "nhl12", "nhl13", "nhl14", "nhl15", "nhl16", "nhllegacy",
    "legacyrecomp", "nhl17", "nhl18", "nhl19", "nhl20", "nhl21",
];

/// The title whose file naming a Hockey Shop sync / install uses (receipt 47, dispatch 2026-09-28 §2).
///
/// The open project's recorded title is authoritative: a request naming another title is refused (a stale page
/// or a mixed-up install). Without a project the request's title is used, and it must be a Studio title id.
/// A slot whose file naming differs between titles (pads: NHL 12 keeps the id second) is refused when no title
/// is known, instead of falling back to the default naming.
pub fn resolve_title(slot: &str, requested: Option<&str>, project: Option<&str>) -> Result<Option<String>, ApiError> {
    let spec = spec_for(slot)?;
    let requested = requested.map(str::trim).filter(|g| !g.is_empty());
    if let Some(g) = requested {
        if !KNOWN_TITLES.contains(&g) {
            return Err(ApiError::Workspace(format!("`{g}` is not a Studio title id; the install was not planned")));
        }
    }
    let title = match (project, requested) {
        (Some(p), Some(r)) if p != r => {
            return Err(ApiError::Workspace(format!(
                "this install request is for {r}, but the open project is {p}. Reopen the Hockey Shop for the open project and review again."
            )))
        }
        (Some(p), _) => Some(p.to_string()),
        (None, r) => r.map(str::to_string),
    };
    if title.is_none() && KNOWN_TITLES.iter().any(|t| spec.for_title(Some(t)).matchers_differ(spec)) {
        return Err(ApiError::Workspace(format!(
            "{} files are named differently by title (NHL 12 keeps the id second), and Studio does not know this install's title. Open the install as a project first.",
            spec.label
        )));
    }
    Ok(title)
}

/// Receipt 67 (research O12-1 / handoff 15; research-receipt-7 review): Legacy Recompiled keeps its equipment inside EB
/// archives and ships no loose `rendering` tree. A Hockey Shop sync would read nothing and an install would write loose files
/// under the game folder, a location no logged runtime request points at; whether the recompiled executable reads a loose
/// override at all is unobserved (card R-RECOMP-LOOSE-1). Until an archive writer or a proven loose-loader contract exists,
/// sync, install planning and install apply are refused for this title — a correctness guard, not removal from scope.
pub const RECOMPILED_REFUSAL: &str = "Legacy Recompiled keeps its equipment inside its game archives, and Studio has no archive \
    writer for it yet; whether the recompiled game reads loose replacement files is not established. The Hockey Shop does not sync \
    from, plan or install to a Recompiled install. Nothing was read or written.";

/// Refuse the Hockey Shop game routes for Legacy Recompiled (by title or by platform).
pub fn refuse_recompiled(title: Option<&str>, recompiled_platform: bool) -> Result<(), ApiError> {
    if recompiled_platform || title == Some("legacyrecomp") {
        return Err(ApiError::Workspace(RECOMPILED_REFUSAL.into()));
    }
    Ok(())
}

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
/// doesn't already carry. Each entry is named from the slot's enum vocabulary (e.g.
/// "CCM Ribcore 40K"), falling back to `<Label> <id>` for an uncatalogued id, and
/// tracked back to its game id via `source: "game"` + `game_asset_id`. The author
/// credits EA Sports plus the game it was extracted from ("EA Sports NHL Legacy") so
/// textures from different titles stay distinguishable in the store. Idempotent —
/// re-running only picks up new ids.
pub fn sync(
    rendering: &Path,
    store_root: &str,
    slot: &str,
    enums: &studio_domain::EnumRegistry,
    game_label: Option<&str>,
    game_id: Option<&str>,
) -> Result<HockeyShopSyncSummaryDto, ApiError> {
    // The title's file naming (NHL 12 pads: id at the second position; receipt 40 / 45 / 47).
    let game_id = resolve_title(slot, game_id, None)?;
    refuse_recompiled(game_id.as_deref(), false)?;
    let spec = &spec_for(slot)?.for_title(game_id.as_deref());
    // "EA Sports NHL Legacy" when the game is known, else bare "EA Sports".
    let author = match game_label.map(str::trim) {
        Some(g) if !g.is_empty() => format!("EA Sports {g}"),
        _ => "EA Sports".to_owned(),
    };
    let store = ensure_store_slot_dir(store_root, slot)?;
    let mut manifest = read_manifest(&store)?;
    let existing: std::collections::BTreeSet<u32> = manifest
        .items
        .iter()
        .filter_map(|s| s.game_asset_id)
        .collect();

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
            summary.failed.push(format!(
                "{} {} (no decodable texture)",
                spec.label, asset.id
            ));
            continue;
        };
        let id = format!("game-{}-{}", spec.key, asset.id);
        let file = format!("{id}.png");
        if let Err(e) = std::fs::write(store.join(&file), &png) {
            summary.failed.push(format!("{file}: {e}"));
            continue;
        }
        // Name from the anchored vocabulary where the id is catalogued (the same
        // model names the game list shows); fall back to a generated `<Label> <id>`.
        let name = studio_domain::slot_enum_label(spec, enums, asset.id)
            .unwrap_or_else(|| format!("{} {}", spec.label, asset.id));
        manifest.items.push(HockeyShopItemDto {
            id,
            name,
            author: author.clone(),
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
        return Err(ApiError::Workspace(
            "item name must not be empty".to_owned(),
        ));
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
    project_title: Option<&str>,
) -> Result<InstallStoreItemPlanDto, ApiError> {
    let title = resolve_title(&req.slot, req.game_id.as_deref(), project_title)?;
    refuse_recompiled(title.as_deref(), false)?;
    let spec = &spec_for(&req.slot)?.for_title(title.as_deref());
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
    let mode = if target_asset.is_some() {
        "overwrite"
    } else {
        "create"
    };

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
                blocking_errors.push(format!(
                    "id {} ships no colour texture to overwrite.",
                    req.target_id
                ));
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
                    // A create never replaces a file: an existing destination belongs to something the slot
                    // scan does not attribute to the target id (receipt 47).
                    for f in &cloned_files {
                        if join_rel(rendering, f).exists() {
                            blocking_errors.push(format!(
                                "{f} already exists; creating id {} would replace it. Nothing was written.",
                                req.target_id
                            ));
                        }
                    }
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
                "{rel} does not contain a supported equipment texture."
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

    let plan_fingerprint = fingerprint(rendering, &store_root_path(store_root, &req.slot)?, &entry, title.as_deref(), req, mode, donor_id, target_color_file.as_deref(), &cloned_files, spec);
    Ok(InstallStoreItemPlanDto {
        title: title.clone(),
        plan_fingerprint,
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
    project_title: Option<&str>,
) -> Result<InstallStoreItemApplyResponseDto, ApiError> {
    // Receipt 67: refused before anything is read (the plan refuses too; this keeps Apply independent of it).
    refuse_recompiled(resolve_title(&req.slot, req.game_id.as_deref(), project_title)?.as_deref(), false)?;
    let store = store_slot_dir(store_root, &req.slot)?;
    let plan = plan_install(rendering, store_root, req, project_title)?;
    let spec = &spec_for(&req.slot)?.for_title(plan.title.as_deref());
    // Receipt 47: apply only the plan the operator reviewed — the same title, files, donor, target and bytes.
    match req.reviewed_fingerprint.as_deref() {
        None => {
            return Err(ApiError::Workspace(
                "install apply needs the reviewed plan; review the install first. Nothing was written.".into(),
            ))
        }
        Some(f) if f != plan.plan_fingerprint => {
            return Err(ApiError::Workspace(
                "the install changed since it was reviewed (files, store item, title or target). Review it again. Nothing was written."
                    .into(),
            ))
        }
        Some(_) => {}
    }
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
    let mut txn = WriteTxn::default();
    let result = (|| -> Result<(), ApiError> {

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
            txn.write(&dest_abs, &bytes, false)?;
            files_written.push(dest_rel);
        }
    }

    // Splice the store texture (resized to the slot) into the target colour map.
    let rgba = store_image_resized_rgba(
        &store,
        &plan.entry_id,
        plan.target_width,
        plan.target_height,
    )?;
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
    // Overwrite mode backs the original up to `.bak` once (never replacing an earlier backup).
    txn.write(&target_abs, &write.bytes, write_backup && plan.mode != "create")?;
    if !files_written.contains(&target_color_file) {
        files_written.push(target_color_file.clone());
    }
    Ok(())
    })();
    if let Err(e) = result {
        let r = txn.rollback();
        let cleanup = if r.cleanup.is_empty() { String::new() } else { format!(" Not cleaned up: {}.", r.cleanup.join("; ")) };
        return Err(ApiError::Workspace(if r.restored() {
            format!("{e}; every file this install had written was restored or removed.{cleanup}")
        } else {
            format!("{e}; restoring was incomplete: {}.{cleanup}", r.unrestored.join("; "))
        }));
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

fn store_root_path(store_root: &str, slot: &str) -> Result<PathBuf, ApiError> {
    store_slot_dir(store_root, slot)
}

fn sha_file(p: &Path) -> String {
    use sha2::Digest;
    match std::fs::read(p) {
        Ok(b) => format!("{:x}", sha2::Sha256::digest(&b)),
        Err(_) => "absent".into(),
    }
}

/// What the operator reviewed: title, roots, store item bytes, target, mode, donor, every file the install reads
/// or writes and its current bytes (receipt 47). Apply refuses unless the recomputed plan has the same value.
#[allow(clippy::too_many_arguments)]
fn fingerprint(
    rendering: &Path,
    store: &Path,
    entry: &HockeyShopItemDto,
    title: Option<&str>,
    req: &InstallStoreItemRequestDto,
    mode: &str,
    donor_id: Option<u32>,
    target_color_file: Option<&str>,
    cloned_files: &[String],
    spec: &studio_domain::EquipmentSlotSpec,
) -> String {
    use sha2::Digest;
    let mut h = sha2::Sha256::new();
    let mut line = |s: String| {
        h.update(s.as_bytes());
        h.update(b"\n");
    };
    line(format!("title={}", title.unwrap_or("")));
    line(format!("rendering={}", rendering.display()));
    line(format!("slot={} entry={} store={}", req.slot, entry.id, sha_file(&store.join(&entry.file))));
    line(format!("target={} mode={mode} donor={donor_id:?} backup={:?}", req.target_id, req.write_backup));
    let mut files: Vec<String> = cloned_files.to_vec();
    if let Some(t) = target_color_file {
        files.push(t.to_string());
    }
    if let Some(d) = donor_id {
        if let Some(a) = studio_domain::list_equipment_slot_assets(rendering, spec).into_iter().find(|a| a.id == d) {
            files.extend(a.files);
        }
    }
    files.sort();
    files.dedup();
    for f in files {
        line(format!("{f}={}", sha_file(&join_rel(rendering, &f))));
    }
    format!("{:x}", h.finalize())
}

/// The files one install writes, with what was there before, so a failure part-way restores them (receipt 47; recovery
/// corrected for coordinator finding F1, receipt 61).
///
/// Ownership is explicit: a backup or temporary file counts as this transaction's only when this transaction created it
/// (`create_new`), so a file that already carries such a name is never overwritten or deleted. Rollback restores each replaced
/// file through a new temporary file; a transaction-created backup is retired only once its destination holds the original bytes
/// again, and when restoring fails the original is kept on disk (the backup, or a recovery copy written for the purpose) and named
/// in the report. This is in-memory best-effort recovery: it is not crash-safe and does not guard against a concurrent writer.
#[derive(Default)]
struct WriteTxn {
    done: Vec<TxnEntry>,
    /// Deterministic fault injection for the ownership tests (receipt 66).
    #[cfg(test)]
    fault: Option<TxnFault>,
}

#[cfg(test)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum TxnFault {
    /// Another writer's file appears at the temporary name after the existence check, before creation.
    ForeignTmpAppears,
    /// Writing the temporary file fails after this transaction created it.
    FailAfterCreate,
}

struct TxnEntry {
    path: PathBuf,
    /// The bytes before this install (`None`: the install created the file).
    prior: Option<Vec<u8>>,
    /// The bytes this install wrote.
    written: Vec<u8>,
    /// A backup this transaction created (holding `prior`).
    backup: Option<PathBuf>,
    /// Whether the destination was actually replaced.
    replaced: bool,
    /// A temporary file this transaction created and could not remove.
    stray_tmp: Option<PathBuf>,
}

/// What a rollback achieved. `unrestored` is empty only when every destination holds its original state again.
#[derive(Debug, Default)]
struct RollbackReport {
    /// Destinations not restored, each with where its original bytes are kept.
    unrestored: Vec<String>,
    /// Transaction-created files that could not be retired (restoration itself succeeded).
    cleanup: Vec<String>,
}

impl RollbackReport {
    fn restored(&self) -> bool {
        self.unrestored.is_empty()
    }
}

/// How [`write_new`] failed (receipt 66). Only `Created` means the file at the name is this call's: `NotCreated` — including
/// `AlreadyExists` when another writer's file appeared after an earlier existence check — leaves whatever is there untouched.
/// This is name ownership for one call, not protection against a concurrent writer in general.
#[derive(Debug)]
enum NewFileError {
    /// Nothing was created at the name.
    NotCreated(std::io::Error),
    /// This call created the file, then failed to fill it; the (partial) file is this call's.
    Created(std::io::Error),
}

impl std::fmt::Display for NewFileError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NewFileError::NotCreated(e) => write!(f, "{e}"),
            NewFileError::Created(e) => write!(f, "{e} (after the file was created)"),
        }
    }
}

/// Write `bytes` through `fill` to a file that must not exist yet (never replacing an unowned file).
fn write_new_with(path: &Path, bytes: &[u8], fill: impl FnOnce(&mut std::fs::File, &[u8]) -> std::io::Result<()>) -> Result<(), NewFileError> {
    let mut f = std::fs::OpenOptions::new().write(true).create_new(true).open(path).map_err(NewFileError::NotCreated)?;
    fill(&mut f, bytes).map_err(NewFileError::Created)
}

/// [`write_new`], removing the file only when this call created it. `Err` carries the message and, when this call's own
/// partial file could not be removed, its path.
fn write_new_owned(path: &Path, bytes: &[u8], fill: impl FnOnce(&mut std::fs::File, &[u8]) -> std::io::Result<()>) -> Result<(), (String, Option<PathBuf>)> {
    match write_new_with(path, bytes, fill) {
        Ok(()) => Ok(()),
        Err(e @ NewFileError::NotCreated(_)) => Err((e.to_string(), None)),
        Err(e @ NewFileError::Created(_)) => {
            let stray = std::fs::remove_file(path).err().map(|_| path.to_path_buf());
            Err((e.to_string(), stray))
        }
    }
}

fn fill_all(f: &mut std::fs::File, b: &[u8]) -> std::io::Result<()> {
    use std::io::Write as _;
    f.write_all(b)?;
    f.sync_all()
}

impl WriteTxn {
    fn write(&mut self, path: &Path, bytes: &[u8], backup: bool) -> Result<(), ApiError> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| ApiError::Workspace(format!("creating {}: {e}", parent.display())))?;
        }
        if path.exists() && !path.is_file() {
            return Err(ApiError::Workspace(format!("{} exists and is not a file; nothing was replaced", path.display())));
        }
        let tmp = tmp_path(path);
        if tmp.exists() {
            return Err(ApiError::Workspace(format!(
                "{} already exists and was not written by this install; Studio does not overwrite it, so {} was not replaced",
                tmp.display(),
                path.display()
            )));
        }
        let prior = if path.is_file() {
            Some(std::fs::read(path).map_err(|e| ApiError::Workspace(format!("reading {}: {e}", path.display())))?)
        } else {
            None
        };
        let mut new_backup = None;
        if backup {
            if let Some(p) = &prior {
                let bak = bak_path(path);
                if !bak.exists() {
                    // A backup this call did not create (it appeared after the check) is never removed; a partial one it did
                    // create is removed, or named when that fails.
                    write_new_owned(&bak, p, fill_all).map_err(|(e, stray)| {
                        ApiError::Workspace(match stray {
                            None => format!("backing up {}: {e}", path.display()),
                            Some(s) => format!("backing up {}: {e}; the partial backup {} could not be removed", path.display(), s.display()),
                        })
                    })?;
                    new_backup = Some(bak);
                }
            }
        }
        #[cfg(test)]
        let fault = self.fault;
        #[cfg(test)]
        if fault == Some(TxnFault::ForeignTmpAppears) {
            std::fs::write(&tmp, b"another writer's file").expect("fault injection");
        }
        // Record before touching the destination, so a failure below still retires the backup.
        self.done.push(TxnEntry { path: path.to_path_buf(), prior, written: bytes.to_vec(), backup: new_backup, replaced: false, stray_tmp: None });
        let entry = self.done.last_mut().expect("just pushed");
        let fill = |f: &mut std::fs::File, b: &[u8]| {
            #[cfg(test)]
            if fault == Some(TxnFault::FailAfterCreate) {
                return Err(std::io::Error::other("injected write failure"));
            }
            fill_all(f, b)
        };
        // Receipt 66: only a temporary file this call created is removed — never one that appeared after the check above.
        if let Err((e, stray)) = write_new_owned(&tmp, bytes, fill) {
            entry.stray_tmp = stray;
            return Err(ApiError::Workspace(format!("writing {}: {e}", tmp.display())));
        }
        if let Err(e) = std::fs::rename(&tmp, path) {
            if std::fs::remove_file(&tmp).is_err() {
                entry.stray_tmp = Some(tmp.clone());
            }
            return Err(ApiError::Workspace(format!("replacing {}: {e}", path.display())));
        }
        entry.replaced = true;
        Ok(())
    }

    /// Undo in reverse order. Never deletes a file this transaction did not create, and never retires a backup whose
    /// destination was not restored.
    fn rollback(self) -> RollbackReport {
        let mut report = RollbackReport::default();
        for e in self.done.into_iter().rev() {
            let restored = if !e.replaced {
                true // the destination was never replaced
            } else {
                match &e.prior {
                    Some(prior) => match restore_bytes(&e.path, prior) {
                        Ok(()) => true,
                        Err(why) => {
                            let kept = match &e.backup {
                                Some(b) if std::fs::read(b).is_ok_and(|x| &x == prior) => format!("its original bytes are in {}", b.display()),
                                _ => {
                                    let rec = recovery_path(&e.path);
                                    match write_new_owned(&rec, prior, fill_all) {
                                        Ok(()) => format!("its original bytes were saved to {}", rec.display()),
                                        Err((re, None)) => format!("its original bytes could not be saved ({}: {re})", rec.display()),
                                        Err((re, Some(s))) => format!("its original bytes could not be saved ({}: {re}); an incomplete copy remains at {}", rec.display(), s.display()),
                                    }
                                }
                            };
                            report.unrestored.push(format!("{} was not restored ({why}); {kept}", e.path.display()));
                            false
                        }
                    },
                    None => {
                        // Created by this install: removed only while it is still the file this install wrote.
                        if !e.path.exists() {
                            true
                        } else if e.path.is_file() && std::fs::read(&e.path).is_ok_and(|x| x == e.written) {
                            match std::fs::remove_file(&e.path) {
                                Ok(()) => true,
                                Err(err) => {
                                    report.unrestored.push(format!("{} (created by this install) could not be removed: {err}", e.path.display()));
                                    false
                                }
                            }
                        } else {
                            report.unrestored.push(format!("{} (created by this install) has changed since, so it was left in place", e.path.display()));
                            false
                        }
                    }
                }
            };
            if let Some(b) = &e.backup {
                if restored {
                    let still_ours = e.prior.as_ref().is_some_and(|p| std::fs::read(b).is_ok_and(|x| &x == p));
                    if !still_ours {
                        report.cleanup.push(format!("{} changed after this install created it, so it was left in place", b.display()));
                    } else if let Err(err) = std::fs::remove_file(b) {
                        report.cleanup.push(format!("{} (a backup this install created) could not be removed: {err}", b.display()));
                    }
                }
            }
            if let Some(t) = &e.stray_tmp {
                if std::fs::read(t).is_ok_and(|x| x == e.written) && std::fs::remove_file(t).is_ok() {
                } else if t.exists() {
                    report.cleanup.push(format!("{} (a temporary file of this install) could not be removed", t.display()));
                }
            }
        }
        report
    }
}

/// Put `prior` back at `path` through a new temporary file (never truncating the destination in place).
fn restore_bytes(path: &Path, prior: &[u8]) -> std::result::Result<(), String> {
    if path.exists() && !path.is_file() {
        return Err(format!("{} is no longer a file", path.display()));
    }
    let tmp = tmp_path(path);
    write_new_owned(&tmp, prior, fill_all).map_err(|(e, stray)| match stray {
        None => format!("writing {}: {e}", tmp.display()),
        Some(s) => format!("writing {}: {e}; the partial temporary file {} could not be removed", tmp.display(), s.display()),
    })?;
    // The temporary file is this call's (created above), so a failed rename may remove it.
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

fn recovery_path(path: &Path) -> PathBuf {
    let mut b = path.as_os_str().to_owned();
    b.push(".nhlms-original");
    PathBuf::from(b)
}

fn bak_path(path: &Path) -> PathBuf {
    let mut b = path.as_os_str().to_owned();
    b.push(".bak");
    PathBuf::from(b)
}

fn tmp_path(path: &Path) -> PathBuf {
    let mut b = path.as_os_str().to_owned();
    b.push(".nhlms-tmp");
    PathBuf::from(b)
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
        return Err(ApiError::Workspace(
            "target texture has zero dimensions".into(),
        ));
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
        image::imageops::resize(
            &rgba,
            target_w,
            target_h,
            image::imageops::FilterType::Lanczos3,
        )
    };
    Ok(resized.into_raw())
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
    serde_json::from_slice(&bytes).map_err(|e| {
        ApiError::Workspace(format!(
            "{} is not a valid store manifest: {e}",
            path.display()
        ))
    })
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

    #[test]
    fn titles_come_from_the_project_and_pads_need_one() {
        assert_eq!(resolve_title("pads", None, Some("nhl12")).unwrap().as_deref(), Some("nhl12"));
        assert_eq!(resolve_title("pads", Some("nhl12"), Some("nhl12")).unwrap().as_deref(), Some("nhl12"));
        assert_eq!(resolve_title("pads", Some("nhl13"), None).unwrap().as_deref(), Some("nhl13"));
        assert!(resolve_title("pads", Some("nhl13"), Some("nhl12")).is_err(), "request ≠ project");
        assert!(resolve_title("pads", None, None).is_err(), "pads naming differs by title");
        assert!(resolve_title("pads", Some("NHL 12"), None).is_err(), "a label is not a title id");
        // Slots named the same on every title need no title.
        assert_eq!(resolve_title("gloves", None, None).unwrap(), None);
    }

    /// An owned scratch folder under the system temp directory, removed on drop. Receipt 66: exclusive per run — the name
    /// carries the process and a counter and is created with `create_dir`, so a pre-existing folder is never reused or cleared.
    struct Scratch(PathBuf);
    impl Scratch {
        fn new(tag: &str) -> Self {
            static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
            let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir = std::env::temp_dir().join(format!("nhlms shop txn {tag} {} {n} {nanos}", std::process::id()));
            std::fs::create_dir(&dir).expect("an exclusive scratch folder");
            Scratch(dir)
        }
    }
    impl Drop for Scratch {
        fn drop(&mut self) {
            // Clear read-only flags the tests set, then remove only this folder.
            for e in std::fs::read_dir(&self.0).into_iter().flatten().flatten() {
                if let Ok(m) = e.metadata() {
                    let mut p = m.permissions();
                    #[allow(clippy::permissions_set_readonly_false)]
                    p.set_readonly(false);
                    let _ = std::fs::set_permissions(e.path(), p);
                }
            }
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn a_failed_install_restores_what_it_wrote() {
        let d = Scratch::new("ordinary");
        let existing = d.0.join("a_1_dm.rx2");
        std::fs::write(&existing, b"original").unwrap();
        let mut txn = WriteTxn::default();
        txn.write(&d.0.join("b_2_dm.rx2"), b"created", false).unwrap();
        txn.write(&existing, b"replaced", true).unwrap();
        assert_eq!(std::fs::read(&existing).unwrap(), b"replaced");
        assert_eq!(std::fs::read(bak_path(&existing)).unwrap(), b"original");
        // A write that cannot happen (the destination is a folder) fails; rollback restores the rest.
        std::fs::create_dir_all(d.0.join("c_3_dm.rx2")).unwrap();
        assert!(txn.write(&d.0.join("c_3_dm.rx2"), b"x", false).is_err());
        let r = txn.rollback();
        assert!(r.restored() && r.cleanup.is_empty(), "{r:?}");
        assert_eq!(std::fs::read(&existing).unwrap(), b"original");
        assert!(!d.0.join("b_2_dm.rx2").exists() && !bak_path(&existing).exists(), "created file and new backup retired");
        assert!(!tmp_path(&d.0.join("c_3_dm.rx2")).exists() && !tmp_path(&existing).exists());
    }

    /// F1: restoring fails after an overwrite with a new backup — the backup (the original bytes) survives and is named.
    #[test]
    fn a_failed_restore_keeps_the_backup_and_names_it() {
        let d = Scratch::new("f1");
        let p = d.0.join("asset.rx2");
        std::fs::write(&p, b"original").unwrap();
        let mut txn = WriteTxn::default();
        txn.write(&p, b"replacement", true).unwrap();
        // Something outside Studio puts a folder where the file was: restoration cannot happen, and the folder is not deleted.
        std::fs::remove_file(&p).unwrap();
        std::fs::create_dir(&p).unwrap();
        let r = txn.rollback();
        assert!(!r.restored(), "{r:?}");
        let msg = r.unrestored.join("; ");
        assert!(msg.contains(&p.display().to_string()) && msg.contains(&bak_path(&p).display().to_string()), "{msg}");
        assert_eq!(std::fs::read(bak_path(&p)).unwrap(), b"original", "the original stays recoverable on disk");
        assert!(p.is_dir(), "the unowned folder is left alone");
    }

    /// Without a backup (create-mode donor clones or `write_backup = false`), a failed restore writes a recovery copy.
    #[test]
    fn a_failed_restore_without_a_backup_saves_a_recovery_copy() {
        let d = Scratch::new("recovery");
        let p = d.0.join("asset.rx2");
        std::fs::write(&p, b"original").unwrap();
        let mut txn = WriteTxn::default();
        txn.write(&p, b"replacement", false).unwrap();
        std::fs::remove_file(&p).unwrap();
        std::fs::create_dir(&p).unwrap();
        let r = txn.rollback();
        let rec = recovery_path(&p);
        assert!(!r.restored() && r.unrestored[0].contains(&rec.display().to_string()), "{r:?}");
        assert_eq!(std::fs::read(&rec).unwrap(), b"original");
    }

    /// A backup that existed before the install is not the install's: it is never replaced or removed.
    #[test]
    fn a_pre_existing_backup_survives() {
        let d = Scratch::new("prebak");
        let p = d.0.join("asset.rx2");
        std::fs::write(&p, b"current").unwrap();
        std::fs::write(bak_path(&p), b"shipped").unwrap();
        let mut txn = WriteTxn::default();
        txn.write(&p, b"replacement", true).unwrap();
        assert_eq!(std::fs::read(bak_path(&p)).unwrap(), b"shipped");
        let r = txn.rollback();
        assert!(r.restored() && r.cleanup.is_empty(), "{r:?}");
        assert_eq!(std::fs::read(&p).unwrap(), b"current");
        assert_eq!(std::fs::read(bak_path(&p)).unwrap(), b"shipped");
    }

    /// Create mode: a file the install created is removed; one changed since is left in place and reported.
    #[test]
    fn created_files_are_removed_only_while_unchanged() {
        let d = Scratch::new("create");
        let (a, b) = (d.0.join("x_1.rx2"), d.0.join("x_2.rx2"));
        let mut txn = WriteTxn::default();
        txn.write(&a, b"clone a", false).unwrap();
        txn.write(&b, b"clone b", false).unwrap();
        std::fs::write(&b, b"someone else's edit").unwrap();
        let r = txn.rollback();
        assert!(!a.exists(), "the unchanged clone is removed");
        assert_eq!(std::fs::read(&b).unwrap(), b"someone else's edit");
        assert!(!r.restored() && r.unrestored[0].contains("changed since"), "{r:?}");
    }

    /// A file already named like the install's temporary file is not the install's: the write refuses before touching anything.
    #[test]
    fn an_unowned_temporary_name_is_never_overwritten() {
        let d = Scratch::new("tmpname");
        let p = d.0.join("asset.rx2");
        std::fs::write(&p, b"original").unwrap();
        std::fs::write(tmp_path(&p), b"not studio's").unwrap();
        let mut txn = WriteTxn::default();
        let e = txn.write(&p, b"replacement", true).unwrap_err().to_string();
        assert!(e.contains("was not written by this install"), "{e}");
        assert_eq!(std::fs::read(tmp_path(&p)).unwrap(), b"not studio's");
        assert_eq!(std::fs::read(&p).unwrap(), b"original");
        assert!(!bak_path(&p).exists(), "no backup was made for a write that did not start");
        assert!(txn.rollback().restored());
        assert_eq!(std::fs::read(tmp_path(&p)).unwrap(), b"not studio's", "rollback does not touch it either");
    }

    /// Receipt 66: another writer's file appears at the temporary name **after** the existence check. Creation fails with
    /// `AlreadyExists`; that file is not this install's, so neither the write nor the rollback removes it.
    #[test]
    fn a_temporary_name_taken_after_the_check_is_not_removed() {
        let d = Scratch::new("race");
        let p = d.0.join("asset.rx2");
        std::fs::write(&p, b"original").unwrap();
        let mut txn = WriteTxn { fault: Some(TxnFault::ForeignTmpAppears), ..Default::default() };
        let e = txn.write(&p, b"replacement", true).unwrap_err().to_string();
        assert!(e.contains(&tmp_path(&p).display().to_string()), "{e}");
        assert_eq!(std::fs::read(tmp_path(&p)).unwrap(), b"another writer's file", "the other writer's file survives the write");
        assert_eq!(std::fs::read(&p).unwrap(), b"original");
        let r = txn.rollback();
        assert!(r.restored() && r.cleanup.is_empty(), "{r:?}");
        assert_eq!(std::fs::read(tmp_path(&p)).unwrap(), b"another writer's file", "…and the rollback");
        assert!(!bak_path(&p).exists(), "the backup this install made is retired");
    }

    /// Receipt 66: writing fails after this install created the temporary file — its own partial file is removed, the
    /// destination is untouched, and the backup it made is retired by the rollback.
    #[test]
    fn a_temporary_file_this_install_created_is_removed_after_a_write_failure() {
        let d = Scratch::new("partial");
        let p = d.0.join("asset.rx2");
        std::fs::write(&p, b"original").unwrap();
        let mut txn = WriteTxn { fault: Some(TxnFault::FailAfterCreate), ..Default::default() };
        let e = txn.write(&p, b"replacement", true).unwrap_err().to_string();
        assert!(e.contains("injected write failure") && e.contains("after the file was created"), "{e}");
        assert!(!tmp_path(&p).exists(), "the install's own partial file is removed");
        assert_eq!(std::fs::read(&p).unwrap(), b"original");
        let r = txn.rollback();
        assert!(r.restored() && r.cleanup.is_empty(), "{r:?}");
        assert!(!bak_path(&p).exists());
        // The helper keeps the two outcomes distinct.
        let q = d.0.join("taken");
        std::fs::write(&q, b"theirs").unwrap();
        assert!(matches!(write_new_with(&q, b"ours", fill_all), Err(NewFileError::NotCreated(e)) if e.kind() == std::io::ErrorKind::AlreadyExists));
        assert!(write_new_owned(&q, b"ours", fill_all).is_err_and(|(_, stray)| stray.is_none()));
        assert_eq!(std::fs::read(&q).unwrap(), b"theirs");
    }

    /// The replacement step fails (a read-only destination on Windows): the temporary file is removed, the destination is
    /// untouched, and rollback retires the new backup.
    #[test]
    fn a_failed_replacement_leaves_the_destination_and_no_temporary_file() {
        let d = Scratch::new("rename");
        let p = d.0.join("asset.rx2");
        std::fs::write(&p, b"original").unwrap();
        let mut perm = std::fs::metadata(&p).unwrap().permissions();
        perm.set_readonly(true);
        std::fs::set_permissions(&p, perm).unwrap();
        let mut txn = WriteTxn::default();
        let res = txn.write(&p, b"replacement", true);
        if cfg!(windows) {
            assert!(res.is_err(), "a read-only destination is not replaced on Windows");
            assert!(!tmp_path(&p).exists(), "the temporary file is removed");
            assert_eq!(std::fs::read(&p).unwrap(), b"original");
            assert!(bak_path(&p).exists());
            let r = txn.rollback();
            assert!(r.restored() && r.cleanup.is_empty(), "{r:?}");
            assert!(!bak_path(&p).exists(), "the new backup is retired once nothing was replaced");
        }
    }
}
