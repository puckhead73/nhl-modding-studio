//! Phase 2.5 (22 Sep 2026) — the arena-asset join through the **registry seam** on the committed
//! anchors with disposable synthetic trees: the contract follows the open title (gen7 file-name
//! rule, PS4 vault reference, unmeasured), the venue model and jumbotron resolve from the arena's
//! art id, the team-keyed families come through the home team, a candidate is never promoted, and
//! an unknown arena id is `None`. Existence-only files; nothing decoded.

use std::path::{Path, PathBuf};

use studio_api::dto::AssetRootsArgsDto;
use studio_api::Registry;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn open(reg: &Registry, platform: &str, game: &str, db: &str) -> u32 {
    let dir = match platform {
        "xbox" => "xbox360",
        "ps3" => "ps3",
        _ => "ps4",
    };
    reg.open_workspace_from_paths(
        Some(game),
        studio_api::parse_platform(platform).unwrap(),
        &s(&repo(&format!("anchors/{dir}/{game}/databases/{db}.db"))),
        None,
        &s(&repo(&format!("anchors/{dir}/{game}/databases/{db}-meta.xml"))),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-arenaapi-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("rendering")).unwrap();
        Tmp(d)
    }
    fn put(&self, dir: &str, name: &str) {
        let d = self.0.join("rendering").join(dir);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join(name), b"x").unwrap();
    }
    fn args(&self, platform: &str) -> AssetRootsArgsDto {
        AssetRootsArgsDto { install_dir: s(&self.0), platform: Some(platform.to_string()), rendering_override: Some(s(&self.0.join("rendering"))), fe_override: None }
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn fam<'a>(v: &'a studio_api::dto::ArenaAssetsDto, n: &str) -> &'a studio_api::dto::AssetFamilyDto {
    v.families.iter().find(|f| f.family == n).unwrap_or_else(|| panic!("family {n}"))
}

fn no_tree(platform: &str) -> AssetRootsArgsDto {
    AssetRootsArgsDto { install_dir: String::new(), platform: Some(platform.to_string()), rendering_override: None, fe_override: None }
}

/// The first arena that has at least one tenant team (so the team-keyed families have a key).
fn tenanted_arena(reg: &Registry, id: u32) -> u64 {
    reg.list_arenas(id)
        .unwrap()
        .into_iter()
        .map(|a| a.arena_id)
        .find(|&a| reg.arena_view(id, a).unwrap().is_some_and(|v| !v.teams.is_empty()))
        .expect("a tenanted arena")
}

#[test]
fn gen7_resolves_venue_and_jumbotron_by_art_id_and_the_logo_through_the_home_team() {
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhllegacy", "nhlng");
    let arena = tenanted_arena(&reg, id);
    let bare = reg.arena_assets(id, &no_tree("xbox"), arena).unwrap().expect("arena exists");
    assert_eq!(bare.contract_status, "gen7");
    let art = bare.arena_art_id.expect("gen7 arena rows carry an art id");
    let venue = fam(&bare, "venue_model");
    assert_eq!(venue.files[0].status, "unknown", "without a tree nothing is present or missing");
    assert_eq!(venue.files[0].file_stem, format!("stadium_{art}_container_0"));
    assert!(bare.notes.iter().any(|n| n.contains("no rendering tree")));
    // The home team's art id is what the logo family will look for.
    let logo_key = {
        let v = reg.arena_assets(id, &no_tree("xbox"), arena).unwrap().unwrap();
        fam(&v, "centre_ice_logo").files.first().map(|f| f.key)
    };

    let tmp = Tmp::new("gen7");
    tmp.put("stadium", &format!("stadium_{art}_container_0.rx2"));
    tmp.put("stadium", "stadium_999_container_0.rx2"); // a spare venue no row names
    tmp.put("jumbotron", &format!("texlib_{art}.rx2"));
    if let Some(k) = logo_key {
        tmp.put("icesurface", &format!("centerlogo_{k}_cm.rx2"));
    }
    let v = reg.arena_assets(id, &tmp.args("xbox"), arena).unwrap().unwrap();
    let venue = fam(&v, "venue_model");
    assert_eq!(venue.binding, "established");
    assert_eq!(venue.files[0].status, "present");
    assert_eq!(venue.files[0].rel_path.as_deref(), Some(format!("rendering/stadium/stadium_{art}_container_0.rx2").as_str()));
    assert_eq!(venue.spare_keys, vec![999]);
    let jt = fam(&v, "jumbotron");
    assert_eq!((jt.binding.as_str(), jt.files[0].status.as_str()), ("established", "present"));
    let logo = fam(&v, "centre_ice_logo");
    assert_eq!(logo.binding, "via_home_team");
    assert!(logo.files[0].key_source.starts_with("home team "), "{}", logo.files[0].key_source);
    if logo_key.is_some() {
        assert_eq!(logo.files[0].status, "present");
    }
    // The banners were not placed in the tree: missing, with the directory named absent.
    let banners = fam(&v, "banners");
    assert_eq!(banners.files[0].status, "missing");
    assert!(banners.files[0].reason.as_deref().unwrap().contains("absent"));
    // The candidate and the unbound families are labelled, never promoted.
    assert_eq!(fam(&v, "bounce_map").binding, "candidate");
    let layers = fam(&v, "ice_layers");
    assert_eq!(layers.binding, "no_established_binding");
    assert!(layers.files.is_empty());
    assert!(v.notes.iter().any(|n| n.contains("never written")));
}

#[test]
fn early_gen7_has_no_jumbotron_and_no_home_arena_link_and_says_so() {
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhl09", "nhlng");
    let arena = reg.list_arenas(id).unwrap()[0].arena_id;
    let tmp = Tmp::new("nhl09");
    let bare = reg.arena_assets(id, &no_tree("xbox"), arena).unwrap().unwrap();
    let art = bare.arena_art_id.unwrap();
    tmp.put("stadium", &format!("stadium_{art}_container_0.rx2"));
    let v = reg.arena_assets(id, &tmp.args("xbox"), arena).unwrap().unwrap();
    assert_eq!(fam(&v, "venue_model").files[0].status, "present");
    let jt = fam(&v, "jumbotron");
    assert_eq!(jt.binding, "no_established_binding");
    assert!(jt.files.is_empty() && jt.notes[0].contains("not shipped"));
    let logo = fam(&v, "centre_ice_logo");
    assert!(logo.files.is_empty());
    assert!(logo.notes[0].contains("no home-arena link"), "{:?}", logo.notes);
}

#[test]
fn ps4_lists_venue_directories_under_the_vault_reference_and_an_unknown_arena_is_none() {
    let reg = Registry::default();
    let id = open(&reg, "ps4", "nhl21", "nhl_game");
    let arena = reg.list_arenas(id).unwrap()[0].arena_id;
    let tmp = Tmp::new("ps4");
    for m in ["anaheim", "eashl_tier3", "common"] {
        tmp.put(&format!("stadiums/{m}"), "a.gnf");
    }
    tmp.put("stadiums/common", "ice_centerlogo_12_cm.gnf");
    let v = reg.arena_assets(id, &tmp.args("ps4"), arena).unwrap().unwrap();
    assert_eq!(v.contract_status, "ps4_vault_reference");
    assert!(v.contract.contains("vault"));
    assert_eq!(v.venue_directories.len(), 3);
    assert!(v.venue_directories.iter().any(|(n, m)| n == "eashl_tier3" && *m == 1));
    let common = v.families.iter().find(|f| f.family == "common_ice").expect("common ice family");
    assert_eq!(common.binding, "candidate");
    assert!(common.files.is_empty());
    assert!(v.families.iter().all(|f| f.binding != "established"), "PS4 claims no established binding");

    assert!(reg.arena_assets(id, &no_tree("ps4"), 999_999).unwrap().is_none(), "an arena id outside the table is None");
}


/// R4 continuation (23 Sep 2026), Teams 2.5 PS4 — research R73 through the seam: with the shipped
/// level table, a PS4 arena lists every vault level carrying its `arenaartid` (Honda Center's id 0
/// is shared: ambiguous, both named, none chosen), an ordinary arena is unambiguous with its folder
/// status from the tree, NHL 15's Carolina Arena is a level without a shipped folder, the shared
/// directories are listed once, and no re-point is offered. Without the defs root the listing is
/// `unmeasured`; on gen7 there is none.
#[test]
fn ps4_arenas_list_their_vault_level_candidates_from_the_r73_table_and_never_choose() {
    let reg = Registry::default();
    let defs = repo("defs");
    let tmp = Tmp::new("ps4-levels");
    for m in ["anaheim", "common_ice_int", "boston", "common", "unused"] {
        tmp.put(&format!("stadiums/{m}"), "a.gnf");
    }
    let id = open(&reg, "ps4", "nhl21", "nhl_game");
    let arenas = reg.list_arenas(id).unwrap();
    // Honda Center: the arena row whose art id is 0.
    let honda = arenas
        .iter()
        .map(|a| a.arena_id)
        .find(|&a| reg.arena_assets(id, &no_tree("ps4"), a).unwrap().unwrap().arena_art_id == Some(0))
        .expect("an arena row with art id 0");
    let v = reg.arena_assets_with_defs(id, &tmp.args("ps4"), honda, Some(&defs)).unwrap().unwrap();
    let l = v.ps4_venue.as_ref().expect("a PS4 listing");
    assert_eq!(l.class, "ambiguous", "{}", l.reason);
    let names: Vec<&str> = l.candidates.iter().map(|c| c.level_name.as_str()).collect();
    assert!(names.contains(&"anaheim") && names.contains(&"common_ice_int"), "{names:?}");
    assert!(l.candidates.iter().all(|c| c.folder == "present" && c.members == Some(1)), "{:?}", l.candidates.iter().map(|c| (c.level_name.clone(), c.dir.clone(), c.folder.clone(), c.members)).collect::<Vec<_>>());
    assert!(l.reason.contains("none is chosen"));
    assert!(!l.reassignment_supported && l.reassignment_reason.contains("cascade"));
    assert!(l.shared_content.iter().any(|(n, m)| n == "common" && *m == Some(1)) && l.shared_content.iter().any(|(n, _)| n == "unused"));
    assert!(!l.shared_content.iter().any(|(n, _)| n == "anaheim"));
    assert!(l.key_label.contains("not a documented selector"));
    // An arena whose id is not shared: unambiguous, one candidate; its folder is missing from this
    // synthetic tree unless it is boston.
    let other = arenas
        .iter()
        .map(|a| a.arena_id)
        .find(|&a| {
            let v = reg.arena_assets_with_defs(id, &no_tree("ps4"), a, Some(&defs)).unwrap().unwrap();
            v.ps4_venue.as_ref().is_some_and(|l| l.class == "unambiguous")
        })
        .expect("an unambiguous arena");
    let v = reg.arena_assets_with_defs(id, &tmp.args("ps4"), other, Some(&defs)).unwrap().unwrap();
    let l = v.ps4_venue.as_ref().unwrap();
    assert_eq!(l.candidates.len(), 1);
    assert!(matches!(l.candidates[0].folder.as_str(), "present" | "missing"), "{:?}", l.candidates[0].folder);
    assert!(l.candidates[0].dir.as_deref().is_some_and(|d| d.starts_with("stadiums/")));
    // Without a tree the folder is unknown; without the defs root the listing is unmeasured.
    let v = reg.arena_assets_with_defs(id, &no_tree("ps4"), other, Some(&defs)).unwrap().unwrap();
    assert_eq!(v.ps4_venue.as_ref().unwrap().candidates[0].folder, "unknown");
    let v = reg.arena_assets(id, &no_tree("ps4"), other).unwrap().unwrap();
    assert_eq!(v.ps4_venue.as_ref().unwrap().class, "unmeasured");
    // NHL 15: Carolina Arena (art id 5) has a level and no shipped folder.
    let id15 = open(&reg, "ps4", "nhl15", "nhl_game");
    let carolina = reg
        .list_arenas(id15)
        .unwrap()
        .iter()
        .map(|a| a.arena_id)
        .find(|&a| reg.arena_assets(id15, &no_tree("ps4"), a).unwrap().unwrap().arena_art_id == Some(5))
        .expect("art id 5 on NHL 15");
    let v = reg.arena_assets_with_defs(id15, &tmp.args("ps4"), carolina, Some(&defs)).unwrap().unwrap();
    let l = v.ps4_venue.as_ref().unwrap();
    assert_eq!(l.class, "level_without_folder", "{}", l.reason);
    assert_eq!(l.candidates[0].folder, "no_folder_shipped");
    // gen7: no PS4 listing.
    let idx = open(&reg, "xbox", "nhllegacy", "nhlng");
    let arena = reg.list_arenas(idx).unwrap().first().unwrap().arena_id;
    assert!(reg.arena_assets_with_defs(idx, &no_tree("xbox"), arena, Some(&defs)).unwrap().unwrap().ps4_venue.is_none());
}

/// Receipt 49 — the venue-art gallery through the Registry on the real X360 NHL Legacy tree (read only): the arena
/// whose `arenaartid` is 1 lists `stadium_1_container_0.rx2` with `adbd_WPG` → a 512 × 1024 DXT1 raster, previews it,
/// and refuses another arena's package, a path outside `stadium/`, a changed hash and an unmeasured title.
#[test]
#[ignore = "real X360 NHL Legacy tree under X:/Games/Emulation/Roms (read only): select with --ignored"]
fn venue_art_gallery_through_the_registry() {
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhllegacy", "nhlng");
    let install = PathBuf::from(r"X:\Games\Emulation\Roms\XBOX360\NHL Legacy\NHL Legacy");
    assert!(install.join("rendering").join("stadium").is_dir(), "missing tree (fails, never skips)");
    let args = AssetRootsArgsDto { install_dir: s(&install), platform: Some("xbox".into()), rendering_override: None, fe_override: None };
    let arena = reg
        .list_arenas(id)
        .unwrap()
        .into_iter()
        .map(|a| a.arena_id)
        .find(|&a| reg.arena_venue_art(id, &args, a).unwrap().is_some_and(|v| v.arena_art_id == Some(1)))
        .expect("an arena with arenaartid 1");
    let v = reg.arena_venue_art(id, &args, arena).unwrap().unwrap();
    assert!(v.measured);
    let p = &v.packages[0];
    assert_eq!(p.file, "stadium/stadium_1_container_0.rx2");
    let m = p.materials.iter().find(|m| m.name == "adbd_WPG").unwrap();
    let ordinal = m.slots[0].raster.unwrap();
    let prev = reg.arena_venue_raster(id, &args, arena, &p.file, &p.sha256, ordinal, 256).unwrap();
    assert_eq!((prev.width, prev.height), (512, 1024));
    assert!(prev.png_data_url.starts_with("data:image/png;base64,"));
    for (file, sha, want) in [
        ("stadium/stadium_2_container_0.rx2", p.sha256.as_str(), "not one of this arena's venue packages"),
        ("stadium/../stadium/stadium_1_container_0.rx2", p.sha256.as_str(), "not one of this arena's venue packages"),
        ("stadium/stadium_1_container_0.rx2", "00", "changed since it was listed"),
    ] {
        let e = match reg.arena_venue_raster(id, &args, arena, file, sha, ordinal, 256) {
            Ok(_) => panic!("{file}: expected a refusal"),
            Err(e) => e.to_string(),
        };
        assert!(e.contains(want), "{file}: {e}");
    }
    // An unmeasured title (NHL 09 anchor; NHL 12 is measured since receipt 58) lists nothing and previews nothing.
    let id12 = open(&reg, "xbox", "nhl09", "nhlng");
    let a12 = reg.list_arenas(id12).unwrap()[0].arena_id;
    let v12 = reg.arena_venue_art(id12, &args, a12).unwrap().unwrap();
    assert!(!v12.measured && v12.packages.is_empty(), "{:?}", v12.notes);
    assert!(reg.arena_venue_raster(id12, &args, a12, &p.file, &p.sha256, ordinal, 256).is_err());
}
