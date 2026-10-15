//! Receipt 67 (research O12-1 / handoff 15; research-receipt-7 review) — the Hockey Shop refuses Legacy Recompiled sync, install
//! planning and install apply through the public Registry path, with nothing read into the store and nothing written into the
//! tree, including stale and forged apply requests. The same owned minimal tree still installs for NHL Legacy (X360), with its
//! backup, so the guard is what refuses.
//!
//! The tree is an owned temporary copy of one X360 NHL Legacy glove (read only from the real tree) under a `rendering` folder,
//! i.e. exactly the loose output a Recompiled install would receive without the guard.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use sha2::Digest;
use studio_api::dto::{AssetRootsArgsDto, InstallStoreItemRequestDto};
use studio_api::{hockey_shop, parse_platform, Registry};

const REAL: &str = r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy\rendering";

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// An exclusive per-run scratch folder, removed on drop.
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Self {
        let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let d = std::env::temp_dir().join(format!("nhlms-r67-shop-recomp-{}-{nanos}", std::process::id()));
        std::fs::create_dir(&d).expect("an exclusive scratch folder");
        Scratch(d)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn tree(dir: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut stack = vec![dir.to_path_buf()];
    while let Some(d) = stack.pop() {
        for e in std::fs::read_dir(&d).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() {
                stack.push(p);
            } else {
                out.insert(p.strip_prefix(dir).unwrap().to_string_lossy().replace('\\', "/"), format!("{:x}", sha2::Sha256::digest(std::fs::read(&p).unwrap())));
            }
        }
    }
    out
}

#[test]
#[ignore = "copies one glove from the real X360 NHL Legacy tree under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn recompiled_hockey_shop_routes_are_refused_with_nothing_written() {
    let real = PathBuf::from(REAL);
    assert!(real.join("glove").is_dir(), "{}: missing (fails, never skips)", real.display());
    let spec = studio_domain::slot_spec("gloves").unwrap().for_title(Some("nhllegacy"));
    let assets = studio_domain::list_equipment_slot_assets(&real, &spec);
    // A glove the X360 Legacy route can overwrite: every file of it is small enough to copy, one is a decodable texlib.
    let glove = assets
        .iter()
        .filter(|a| a.id != 999 && a.id != 0 && !a.files.is_empty())
        .find(|a| a.files.iter().any(|f| studio_domain::color_map_slot(&real.join(f)).is_some()))
        .expect("a shipped glove with a colour texture");

    let sc = Scratch::new();
    let rendering = sc.0.join("install").join("game").join("rendering");
    for rel in &glove.files {
        let dest = rendering.join(rel);
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        std::fs::copy(real.join(rel), &dest).unwrap();
    }
    let store = sc.0.join("store");
    let store_root = s(&store);
    let mut img = image::RgbaImage::new(32, 32);
    for (x, y, p) in img.enumerate_pixels_mut() {
        *p = image::Rgba([(x * 8) as u8, (y * 8) as u8, 90, 255]);
    }
    let mut png = Vec::new();
    img.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png).unwrap();
    use base64::Engine as _;
    let item = hockey_shop::import(&store_root, "gloves", "Receipt 67 gradient", "test", &base64::engine::general_purpose::STANDARD.encode(&png)).unwrap();
    let tree0 = tree(&rendering);
    let store0 = tree(&store);

    // Two workspaces over copies of the X360 Legacy anchor: Legacy Recompiled and NHL Legacy (X360).
    let anchor = repo("anchors/xbox360/nhllegacy/databases");
    let (db, meta) = (sc.0.join("nhlng.db"), sc.0.join("nhlng-meta.xml"));
    std::fs::copy(anchor.join("nhlng.db"), &db).unwrap();
    std::fs::copy(anchor.join("nhlng-meta.xml"), &meta).unwrap();
    let reg = Registry::default();
    let open = |game: &str, platform: &str| reg.open_workspace_from_paths(Some(game), parse_platform(platform).unwrap(), &s(&db), None, &s(&meta), &s(&repo("defs"))).unwrap().project_id;
    let recomp = open("legacyrecomp", "recomp");
    let legacy = open("nhllegacy", "xbox");
    let roots = |platform: &str| AssetRootsArgsDto { install_dir: s(&sc.0.join("install")), platform: Some(platform.into()), rendering_override: Some(s(&rendering)), fe_override: None };
    let req = |game: Option<&str>, fp: Option<&str>| InstallStoreItemRequestDto {
        slot: "gloves".into(),
        entry_id: item.id.clone(),
        target_id: glove.id,
        donor_id: None,
        write_backup: Some(true),
        game_id: game.map(str::to_string),
        reviewed_fingerprint: fp.map(str::to_string),
    };
    let refused = |r: Result<String, String>, what: &str| {
        let e = r.err().unwrap_or_else(|| panic!("{what}: expected a refusal"));
        assert!(e.contains("Legacy Recompiled keeps its equipment inside its game archives") && e.contains("Nothing was read or written"), "{what}: {e}");
    };
    let err = |e: studio_api::ApiError| e.to_string();

    // The control first: NHL Legacy (X360) on the same tree plans an allowed overwrite, so the tree itself is installable.
    let control = reg.hockey_shop_install_plan(Some(legacy), &roots("xbox"), &store_root, &req(None, None)).unwrap();
    assert!(control.blocking_errors.is_empty() && control.mode == "overwrite", "{:?}", control.blocking_errors);

    // Legacy Recompiled: sync, plan, apply (unreviewed, forged, and the control's genuine fingerprint) — all refused.
    let rr = roots("recomp");
    refused(reg.hockey_shop_sync(Some(recomp), &rr, &store_root, "gloves", &repo("defs"), Some("NHL Legacy Recompiled"), None).map(|_| String::new()).map_err(err), "sync");
    refused(reg.hockey_shop_install_plan(Some(recomp), &rr, &store_root, &req(None, None)).map(|_| String::new()).map_err(err), "plan");
    for fp in [None, Some("forged"), Some(control.plan_fingerprint.as_str())] {
        refused(reg.hockey_shop_install_apply(Some(recomp), &rr, &store_root, &req(None, fp)).map(|_| String::new()).map_err(err), &format!("apply {fp:?}"));
    }
    // Without a project: by the requested title, and by the roots' platform even when the request names NHL Legacy.
    refused(reg.hockey_shop_install_plan(None, &roots("xbox"), &store_root, &req(Some("legacyrecomp"), None)).map(|_| String::new()).map_err(err), "project-less title");
    refused(reg.hockey_shop_install_apply(None, &rr, &store_root, &req(Some("nhllegacy"), Some(&control.plan_fingerprint))).map(|_| String::new()).map_err(err), "project-less platform");
    // The domain functions refuse the title on their own too.
    refused(hockey_shop::apply_install(&rendering, &store_root, &req(Some("legacyrecomp"), Some(&control.plan_fingerprint)), None).map(|_| String::new()).map_err(err), "domain apply");
    assert_eq!(tree(&rendering), tree0, "nothing written into the tree (no clone, no .bak, no temporary file)");
    assert_eq!(tree(&store), store0, "nothing synced into the store");

    // NHL Legacy (X360) still installs, with its backup, on the same tree.
    let applied = reg.hockey_shop_install_apply(Some(legacy), &roots("xbox"), &store_root, &req(None, Some(&control.plan_fingerprint))).unwrap();
    let after = tree(&rendering);
    let target = applied.plan.target_color_file.clone().unwrap();
    assert_ne!(after.get(&target), tree0.get(&target), "the control install wrote the colour file");
    assert_eq!(after.get(&format!("{target}.bak")), tree0.get(&target), "…after backing up the original");
    eprintln!("glove {} ({} files): Recompiled sync / plan / 3 applies / 2 project-less / domain apply refused, tree unchanged; X360 Legacy control installed {target} with .bak", glove.id, glove.files.len());
}
