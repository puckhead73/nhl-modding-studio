//! Receipts 45 / 47 — the Hockey Shop reads and writes NHL 12 goalie pads at the title's id position (research R125:
//! NHL 12 `fegoaliepad_0_<id>_0_dm`, the id second).
//!
//! Receipt 45 checked sync and a planned install. Receipt 47 (dispatch 2026-09-28 §2) **applies** installs on an
//! owned disposable copy of the minimum NHL 12 donor / target tree (never the registered install), re-reads the
//! output with the title's matching, checks that no other file changed, the backup behaviour, the stale-review
//! refusal, and the title context (omitted / wrong / unknown). The real X360 tree is only read, to make the copy.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::Digest;
use studio_api::dto::InstallStoreItemRequestDto;
use studio_api::hockey_shop;

const REAL: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL 12\NHL 12\rendering";

struct Scratch(PathBuf);
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn sha(p: &Path) -> String {
    format!("{:x}", sha2::Sha256::digest(std::fs::read(p).unwrap()))
}

/// Every file under `dir`, relative, with its SHA-256.
fn tree(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.insert(p.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/"), sha(&p));
            }
        }
    }
    out
}

fn nhl12_pads() -> studio_domain::EquipmentSlotSpec {
    studio_domain::slot_spec("pads").unwrap().for_title(Some("nhl12"))
}

#[test]
#[ignore = "real X360 NHL 12 tree under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn nhl12_pads_sync_and_install_plan_follow_the_title_naming() {
    let rendering = PathBuf::from(REAL);
    assert!(rendering.join("goaliepad").is_dir(), "{}: missing (fails, never skips)", rendering.display());
    let store = std::env::temp_dir().join(format!("nhlms-hockey-shop-title-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&store);
    std::fs::create_dir_all(&store).unwrap();
    let _g = Scratch(store.clone());
    let store_root = store.to_str().unwrap().to_string();
    let enums = studio_domain::EnumRegistry::default();
    let spec = nhl12_pads();
    let title_ids: Vec<u32> = studio_domain::list_equipment_slot_assets(&rendering, &spec).into_iter().map(|a| a.id).collect();
    let default_ids: Vec<u32> = studio_domain::list_equipment_slot_assets(&rendering, studio_domain::slot_spec("pads").unwrap()).into_iter().map(|a| a.id).collect();
    // Receipt 47: 41 pads — the shared meshes `goaliepad_<side>_<lod>` no longer read as pads 0 / 2 / 3.
    assert_eq!(title_ids.len(), 41, "{title_ids:?}");
    assert!(!title_ids.iter().any(|i| [0, 2, 3].contains(i)), "{title_ids:?}");
    assert_ne!(title_ids, default_ids, "the two readings name different id sets on NHL 12");

    let summary = hockey_shop::sync(&rendering, &store_root, "pads", &enums, Some("NHL 12"), Some("nhl12")).unwrap();
    let items = hockey_shop::list(&store_root, "pads").unwrap();
    let synced: Vec<u32> = items.iter().filter_map(|i| i.game_asset_id).collect();
    assert!(!synced.is_empty() && synced.iter().all(|id| title_ids.contains(id)), "store ids = the title's pads ids: {synced:?}");
    eprintln!("NHL 12 pads: title reading {} ids, default reading {} ids; synced {} (failed {:?})", title_ids.len(), default_ids.len(), summary.added, summary.failed);
    // Receipt 47: a sync without any title is refused for pads instead of naming items by the default reading.
    assert!(hockey_shop::sync(&rendering, &store_root, "pads", &enums, None, None).is_err());
}

/// The applied write regression on an owned copy (receipt 47).
#[test]
#[ignore = "copies part of the real X360 NHL 12 goaliepad folder (read only) into a temporary tree: select with --ignored"]
fn nhl12_pads_install_applies_on_a_disposable_copy() {
    let real = PathBuf::from(REAL).join("goaliepad");
    assert!(real.is_dir(), "{}: missing (fails, never skips)", real.display());
    let spec = nhl12_pads();
    let real_assets = studio_domain::list_equipment_slot_assets(Path::new(REAL), &spec);
    let donor = real_assets.iter().find(|a| a.id == 54).expect("NHL 12 pads 54");
    let other = real_assets.iter().find(|a| a.id == 11).expect("NHL 12 pads 11");

    // The minimum tree: donor 54, the overwrite target 11, the shared meshes / ambient maps (must stay untouched).
    let root = std::env::temp_dir().join(format!("nhlms hockey shop nhl12 {}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let _g = Scratch(root.clone());
    let rendering = root.join("rendering");
    let dest = rendering.join("goaliepad");
    std::fs::create_dir_all(&dest).unwrap();
    let mut copied = 0;
    for rel in donor.files.iter().chain(other.files.iter()) {
        let name = rel.rsplit('/').next().unwrap();
        std::fs::copy(real.join(name), dest.join(name)).unwrap();
        copied += 1;
    }
    for name in ["goaliepad_0_0.rx2", "goaliepad_0_2.rx2", "goaliepad_1_3.rx2", "goaliepad_0_am.rx2"] {
        std::fs::copy(real.join(name), dest.join(name)).unwrap();
        copied += 1;
    }
    let before = tree(&rendering);
    assert_eq!(before.len(), copied);

    // A store item: a synthetic 64 × 64 gradient (detectably different from any shipped texture).
    let store = root.join("store");
    let store_root = store.to_str().unwrap().to_string();
    let mut img = image::RgbaImage::new(64, 64);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = image::Rgba([(x * 4) as u8, (y * 4) as u8, 200, 255]);
    }
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    use base64::Engine as _;
    let item = hockey_shop::import(&store_root, "pads", "Receipt 47 gradient", "test", &base64::engine::general_purpose::STANDARD.encode(&png)).unwrap();

    let req = |target: u32, game: Option<&str>| InstallStoreItemRequestDto {
        slot: "pads".into(),
        entry_id: item.id.clone(),
        target_id: target,
        donor_id: Some(54),
        write_backup: Some(true),
        game_id: game.map(str::to_string),
        reviewed_fingerprint: None,
    };

    // ── Title context through the public path ────────────────────────────────────────────────────────────────
    let e = hockey_shop::plan_install(&rendering, &store_root, &req(60, None), None).err().expect("refused").to_string();
    assert!(e.contains("does not know this install's title"), "omitted title: {e}");
    let e = hockey_shop::plan_install(&rendering, &store_root, &req(60, Some("nhl13")), Some("nhl12")).err().expect("refused").to_string();
    assert!(e.contains("open project is nhl12"), "stale / wrong title: {e}");
    let e = hockey_shop::plan_install(&rendering, &store_root, &req(60, Some("NHL 12")), None).err().expect("refused").to_string();
    assert!(e.contains("not a Studio title id"), "unknown title string: {e}");
    // With the project's title the request may omit it; with a matching title both agree.
    let plan = hockey_shop::plan_install(&rendering, &store_root, &req(60, None), Some("nhl12")).unwrap();
    let same = hockey_shop::plan_install(&rendering, &store_root, &req(60, Some("nhl12")), Some("nhl12")).unwrap();
    assert_eq!(plan.plan_fingerprint, same.plan_fingerprint);
    assert_eq!(plan.title.as_deref(), Some("nhl12"));
    assert!(plan.blocking_errors.is_empty(), "{:?}", plan.blocking_errors);
    assert_eq!((plan.mode.as_str(), plan.donor_id, plan.cloned_files.len()), ("create", Some(54), donor.files.len()));
    assert_eq!(tree(&rendering), before, "planning writes nothing");

    // ── Apply refusals: no reviewed plan, a stale review ─────────────────────────────────────────────────────
    let e = hockey_shop::apply_install(&rendering, &store_root, &req(60, Some("nhl12")), Some("nhl12")).err().expect("refused").to_string();
    assert!(e.contains("needs the reviewed plan"), "{e}");
    let e = hockey_shop::apply_install(&rendering, &store_root, &InstallStoreItemRequestDto { reviewed_fingerprint: Some(plan.plan_fingerprint.clone()), ..req(60, Some("nhl13")) }, Some("nhl12")).err().expect("refused").to_string();
    assert!(e.contains("open project is nhl12"), "{e}");
    let donor_color = dest.join(plan.cloned_files.iter().find(|f| f.ends_with("_dm.rx2") && f.contains("fegoaliepad")).unwrap().replace("goaliepad/", "").replace("_60_", "_54_"));
    let original = std::fs::read(&donor_color).unwrap();
    std::fs::write(&donor_color, [original.as_slice(), b"x"].concat()).unwrap();
    let e = hockey_shop::apply_install(&rendering, &store_root, &InstallStoreItemRequestDto { reviewed_fingerprint: Some(plan.plan_fingerprint.clone()), ..req(60, None) }, Some("nhl12")).err().expect("refused").to_string();
    assert!(e.contains("changed since it was reviewed"), "stale review: {e}");
    std::fs::write(&donor_color, &original).unwrap();
    assert_eq!(tree(&rendering), before, "refused applies write nothing");

    // ── Create: clone donor 54 to 60 and splice the store texture ────────────────────────────────────────────
    let applied = hockey_shop::apply_install(&rendering, &store_root, &InstallStoreItemRequestDto { reviewed_fingerprint: Some(plan.plan_fingerprint.clone()), ..req(60, None) }, Some("nhl12")).unwrap();
    let after = tree(&rendering);
    let assets = studio_domain::list_equipment_slot_assets(&rendering, &spec);
    let created = assets.iter().find(|a| a.id == 60).expect("id 60 re-read with the title's matching");
    assert_eq!(created.files.len(), donor.files.len());
    for f in &created.files {
        let n: Vec<u32> = f.rsplit('/').next().unwrap().split('.').next().unwrap().split('_').filter_map(|t| t.parse().ok()).collect();
        assert_eq!(n.get(1), Some(&60), "{f}: the target id second");
    }
    let color = plan.target_color_file.clone().unwrap();
    for (rel, h) in &before {
        assert_eq!(after.get(rel), Some(h), "{rel}: an existing file changed during a create");
    }
    let new_files: Vec<&String> = after.keys().filter(|k| !before.contains_key(*k)).collect();
    let ledger = "goaliepad/hockey_shop_installed.json".to_string();
    assert_eq!(new_files.len(), donor.files.len() + 1, "{new_files:?}");
    assert!(new_files.contains(&&ledger) && !after.keys().any(|k| k.ends_with(".bak") || k.ends_with(".nhlms-tmp")));
    // Every clone equals its donor file, except the colour file, which now carries the store texture.
    for f in &created.files {
        let src = f.replace("_60_", "_54_");
        if *f == color {
            assert_ne!(after[f], before[&src], "{f}: the store texture was spliced in");
            let slot = studio_domain::color_map_slot(&rendering.join(f)).expect("the spliced file still parses");
            assert_eq!((slot.1, slot.2), (plan.target_width, plan.target_height));
        } else {
            assert_eq!(after[f], before[&src], "{f}: a byte clone of {src}");
        }
    }
    assert!(applied.files_written.contains(&color));

    // ── Overwrite: id 11's colour file only, `.bak` first, kept on a second install ──────────────────────────
    let oplan = hockey_shop::plan_install(&rendering, &store_root, &req(11, None), Some("nhl12")).unwrap();
    assert_eq!(oplan.mode, "overwrite", "{:?}", oplan.blocking_errors);
    let ocolor = oplan.target_color_file.clone().unwrap();
    let original11 = after[&ocolor].clone();
    hockey_shop::apply_install(&rendering, &store_root, &InstallStoreItemRequestDto { reviewed_fingerprint: Some(oplan.plan_fingerprint.clone()), ..req(11, None) }, Some("nhl12")).unwrap();
    let after2 = tree(&rendering);
    let bak = format!("{ocolor}.bak");
    assert_eq!(after2.get(&bak), Some(&original11), "the backup is the original");
    assert_ne!(after2[&ocolor], original11);
    for (rel, h) in &after {
        if *rel != ocolor && *rel != ledger {
            assert_eq!(after2.get(rel), Some(h), "{rel} changed during the overwrite");
        }
    }
    // A second overwrite keeps the first backup (the shipped original), never replacing it with the first install.
    let oplan2 = hockey_shop::plan_install(&rendering, &store_root, &req(11, None), Some("nhl12")).unwrap();
    hockey_shop::apply_install(&rendering, &store_root, &InstallStoreItemRequestDto { reviewed_fingerprint: Some(oplan2.plan_fingerprint.clone()), ..req(11, None) }, Some("nhl12")).unwrap();
    assert_eq!(tree(&rendering).get(&bak), Some(&original11));
    // Installing at 60 again is an overwrite now (its files exist), never a second create.
    let again = hockey_shop::plan_install(&rendering, &store_root, &req(60, None), Some("nhl12")).unwrap();
    assert_eq!(again.mode, "overwrite");
    // A create whose destination name already exists but is not read as that id (different case on Windows) is
    // blocked, or read as the id and planned as an overwrite — never silently replaced.
    std::fs::write(dest.join("FEGOALIEPAD_0_62_0_DM.rx2"), b"not a pad texture").unwrap();
    let p62 = hockey_shop::plan_install(&rendering, &store_root, &req(62, None), Some("nhl12")).unwrap();
    assert!(p62.mode == "overwrite" || p62.blocking_errors.iter().any(|e| e.contains("already exists")), "{} {:?}", p62.mode, p62.blocking_errors);
    eprintln!(
        "create 54 → 60: {} files ({} new incl. ledger), colour {color}; overwrite 11: {ocolor} (+ .bak); copy tree {} files",
        created.files.len(),
        new_files.len(),
        before.len()
    );
}
