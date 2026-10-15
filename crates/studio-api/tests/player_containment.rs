//! Phase 3 (22 Sep 2026) — the portrait / head containment through the **registry seam** on the
//! committed anchors with disposable synthetic trees: the contract follows the open title, the
//! `hasportrait` flag is reported as stored and cross-tabulated against the files, a bespoke
//! player without a head file is "shipped without asset", the unused pools are counted, and an
//! unmeasured title joins nothing. Existence-only files; nothing decoded.

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
        let d = std::env::temp_dir().join(format!("nhlms-containapi-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
    fn put(&self, rel: &str) {
        let p = self.0.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(p, b"x").unwrap();
    }
    fn args(&self, platform: &str) -> AssetRootsArgsDto {
        AssetRootsArgsDto { install_dir: s(&self.0), platform: Some(platform.to_string()), rendering_override: Some(s(&self.0.join("rendering"))), fe_override: Some(s(&self.0.join("fe"))) }
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn no_tree(platform: &str) -> AssetRootsArgsDto {
    AssetRootsArgsDto { install_dir: String::new(), platform: Some(platform.to_string()), rendering_override: None, fe_override: None }
}

#[test]
fn gen7_reports_the_flag_as_stored_and_the_files_as_found() {
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhllegacy", "nhlng");
    let bare = reg.player_containment(id, &no_tree("xbox")).unwrap();
    assert_eq!(bare.summary.contract_status, "gen7");
    assert!(bare.summary.has_flag_column);
    assert_eq!(bare.summary.players, bare.players.len());
    assert!(bare.summary.bespoke > 200 && bare.summary.pool_players > 5000, "{:?}", (bare.summary.bespoke, bare.summary.pool_players));
    assert_eq!(bare.summary.pool_floor, Some(60_000));
    assert!(bare.players.iter().all(|p| p.portrait == "unknown" && p.head == "unknown"), "without a tree nothing is present or missing");
    assert_eq!((bare.summary.flag1_file1, bare.summary.flag0_file1), (0, 0), "no file evidence without a tree");
    assert!(bare.summary.notes.iter().any(|n| n.contains("no front-end tree")));

    // A bespoke player (head id = art id) and a flagged pool player, with a synthetic tree that
    // ships the bespoke portrait but not the head.
    let bespoke = bare.players.iter().find(|p| p.class == "bespoke").expect("a bespoke player");
    let flagged_pool = bare.players.iter().find(|p| p.class == "pool" && p.has_portrait_flag == Some(true)).expect("a flagged pool player");
    let tmp = Tmp::new("gen7");
    tmp.put(&format!("fe/ion/artassets/playerheads/p0_4000/p{}.big", bespoke.portrait_id));
    tmp.put("fe/ion/artassets/playerheads/p0_4000/p3999.big"); // unreferenced? only if no row uses 3999 — counted below either way
    tmp.put(&format!("rendering/player/player_{}.rx2", flagged_pool.head_id));
    let v = reg.player_containment(id, &tmp.args("xbox")).unwrap();
    let b = v.players.iter().find(|p| p.bio_record == bespoke.bio_record).unwrap();
    assert_eq!((b.portrait.as_str(), b.head.as_str()), ("present", "missing"));
    assert!(b.note.as_deref().unwrap().starts_with("shipped without asset"), "{:?}", b.note);
    assert!(v.summary.bespoke_missing_head.contains(&b.full_name));
    let f = v.players.iter().find(|p| p.bio_record == flagged_pool.bio_record).unwrap();
    assert_eq!((f.portrait.as_str(), f.head.as_str(), f.has_portrait_flag), ("missing", "present", Some(true)));
    assert!(f.note.as_deref().unwrap().contains("`hasportrait` is set but no portrait file"));
    assert!(v.summary.flag1_file0 >= 1 && v.summary.flagged_without_file.contains(&f.full_name));
    assert_eq!(v.summary.shipped_portraits, Some(2));
    assert_eq!(v.summary.shipped_heads, Some(1));
    // The flag itself is untouched: the same rows read the same flag with and without the tree.
    assert_eq!(b.has_portrait_flag, bespoke.has_portrait_flag);
}

#[test]
fn nhl09_uses_head_files_and_ps4_reads_portraits_only_and_nhl08_is_unmeasured() {
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhl09", "nhlng");
    let bare = reg.player_containment(id, &no_tree("xbox")).unwrap();
    assert_eq!(bare.summary.pool_floor, Some(10_000));
    let bespoke = bare.players.iter().find(|p| p.class == "bespoke").unwrap();
    let tmp = Tmp::new("nhl09");
    tmp.put(&format!("rendering/player/head_{}.rx2", bespoke.head_id));
    let v = reg.player_containment(id, &tmp.args("xbox")).unwrap();
    assert_eq!(v.players.iter().find(|p| p.bio_record == bespoke.bio_record).unwrap().head, "present");
    assert!(v.summary.contract.contains("head_"));

    let id = open(&reg, "ps4", "nhl21", "nhl_game");
    let bare = reg.player_containment(id, &no_tree("ps4")).unwrap();
    assert_eq!(bare.summary.contract_status, "ps4");
    let some = bare.players.iter().find(|p| p.portrait_id > 0).unwrap();
    let tmp = Tmp::new("ps4");
    // The PS4 front-end root is the `fe` override here; the rule looks under `kettle_assets/playerportraits`.
    tmp.put(&format!("fe/kettle_assets/playerportraits/p{}.gnf", some.portrait_id));
    let v = reg.player_containment(id, &tmp.args("ps4")).unwrap();
    let p = v.players.iter().find(|p| p.bio_record == some.bio_record).unwrap();
    assert_eq!((p.portrait.as_str(), p.head.as_str()), ("present", "unknown"));
    assert!(v.summary.shipped_heads.is_none());
    assert!(v.summary.notes.iter().any(|n| n.contains("name-keyed vault")));

    let id = open(&reg, "ps3", "nhl08", "nhlng");
    let v = reg.player_containment(id, &no_tree("ps3")).unwrap();
    assert_eq!(v.summary.contract_status, "unmeasured");
    assert!(v.players.iter().all(|p| p.class == "other" && p.portrait == "unknown"));
}
