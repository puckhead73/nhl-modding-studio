//! Phase 2.6 (22 Sep 2026) — the team-kit join through the **registry seam**: the contract comes
//! from the open project's title and platform, the evidence from the attached tree, and the
//! coordinator's F1 reproduction (jersey + pants, no `sock/` directory) yields an *incomplete* kit
//! and *no complete spare* all the way out to the DTO. Committed anchors read only; the rendering
//! tree is a disposable synthetic directory of empty files (existence is all the join reads).

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

/// A disposable install root that removes itself (no `tempfile` dev-dependency in this crate).
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-kitapi-{tag}-{}", std::process::id()));
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
        AssetRootsArgsDto {
            install_dir: s(&self.0),
            platform: Some(platform.to_string()),
            rendering_override: Some(s(&self.0.join("rendering"))),
            fe_override: None,
        }
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

/// The first team of the anchor that registers at least one kit row.
fn team_with_rows(reg: &Registry, id: u32) -> u64 {
    reg.list_teams(id)
        .unwrap()
        .into_iter()
        .map(|t| t.team_id)
        .find(|&t| !reg.list_team_jerseys(id, t).unwrap().is_empty())
        .expect("a team with kit rows")
}

#[test]
fn f1_through_the_registry_a_tree_without_socks_yields_incomplete_kits_and_no_complete_spare() {
    let reg = Registry::default();
    let id = open(&reg, "xbox", "nhl15", "nhlng");
    let team = team_with_rows(&reg, id);

    // Read the rows first (no tree) to learn the triple to build the synthetic tree for.
    let bare = reg.team_kits(id, &no_tree("xbox"), team).unwrap();
    assert_eq!(bare.contract_status, "measured");
    assert_eq!(bare.sock_grammar.as_deref(), Some("single_file"));
    assert!(bare.rendering_root.is_none() && bare.directories.is_empty());
    let first = &bare.kits[0];
    assert_eq!(first.completeness, "unknown", "without a tree nothing is complete");
    assert_eq!(first.components.len(), 3);
    assert!(first.components.iter().all(|c| c.status == "unknown" && c.path.is_none() && c.rel_path.is_none()));
    let (style, art, variant) = (first.style, first.team_art_id, first.variant);
    assert_eq!(Some(art), bare.team_art_id, "the shipped row's art id is the team's own");

    // The coordinator's reproduction: jersey and pants, no `sock/` at all — plus an unused
    // jersey + pant pair that the old rule would have offered as a complete spare.
    let tmp = Tmp::new("nosock");
    tmp.put("jersey", &format!("texlib_{style}_{art}_{variant}.rx2"));
    tmp.put("pant", &format!("texlib_{style}_{art}_{variant}.rx2"));
    tmp.put("jersey", &format!("texlib_{style}_{art}_23.rx2"));
    tmp.put("pant", &format!("texlib_{style}_{art}_23.rx2"));
    let v = reg.team_kits(id, &tmp.args("xbox"), team).unwrap();
    assert_eq!(v.team_id, team);
    assert_eq!(v.rendering_root.as_deref(), Some(s(&tmp.0.join("rendering")).as_str()));
    let k = v.kits.iter().find(|k| k.record == first.record).unwrap();
    assert_eq!(k.completeness, "incomplete", "{:?}", k.components.iter().map(|c| (&c.label, &c.status)).collect::<Vec<_>>());
    let by_label = |l: &str| k.components.iter().find(|c| c.label == l).unwrap();
    assert_eq!(by_label("jersey").status, "present");
    assert_eq!(by_label("jersey").rel_path.as_deref(), Some(format!("rendering/jersey/texlib_{style}_{art}_{variant}.rx2").as_str()), "the install-relative path feeds texture_preview");
    assert_eq!(by_label("pants").status, "present");
    assert_eq!(by_label("socks").status, "missing");
    assert!(by_label("socks").reason.as_deref().unwrap().contains("absent"));
    assert!(v.directories.iter().any(|d| d.part == "sock" && d.status == "absent"));
    // The spare candidate is listed — and it is incomplete, never complete.
    let spare = v.spares.iter().find(|sp| (sp.style, sp.variant) == (style, 23)).expect("the unused pair is listed as a candidate");
    assert_eq!(spare.completeness, "incomplete");
    assert!(v.spares.iter().all(|sp| sp.completeness != "complete"), "no complete spare without every component");
    assert!(!v.reassignment_supported);
    assert!(v.notes.iter().any(|n| n.contains("`sock/` directory is absent")), "{:?}", v.notes);

    // Add the sock and the same rule turns the row complete — through the same seam.
    tmp.put("sock", &format!("sock_{style}_{art}_{variant}_cm.rx2"));
    let v = reg.team_kits(id, &tmp.args("xbox"), team).unwrap();
    let k = v.kits.iter().find(|k| k.record == first.record).unwrap();
    assert_eq!(k.completeness, "complete");
    assert!(k.components.iter().all(|c| c.status == "present" && c.rel_path.as_deref().is_some_and(|r| r.starts_with("rendering/"))));
    // The spare pair still lacks its sock, so it stays incomplete while the row is complete.
    assert_eq!(v.spares.iter().find(|sp| (sp.style, sp.variant) == (style, 23)).unwrap().completeness, "incomplete");
}

#[test]
fn the_contract_follows_the_open_title_and_the_two_file_grammar_lists_both_sock_files() {
    let reg = Registry::default();
    // NHL 11: two sock files per kit, both required, both listed.
    let id = open(&reg, "xbox", "nhl11", "nhlng");
    let team = team_with_rows(&reg, id);
    let bare = reg.team_kits(id, &no_tree("xbox"), team).unwrap();
    assert_eq!(bare.sock_grammar.as_deref(), Some("two_file"));
    let first = &bare.kits[0];
    assert_eq!(first.components.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(), ["jersey", "pants", "socks (file 0)", "socks (file 1)"]);
    let (style, art, variant) = (first.style, first.team_art_id, first.variant);
    let tmp = Tmp::new("twofile");
    tmp.put("jersey", &format!("texlib_{style}_{art}_{variant}.rx2"));
    tmp.put("pant", &format!("texlib_{style}_{art}_{variant}.rx2"));
    tmp.put("sock", &format!("sock_{style}_0_{art}_{variant}_cm.rx2")); // only file 0
    let v = reg.team_kits(id, &tmp.args("xbox"), team).unwrap();
    let k = v.kits.iter().find(|k| k.record == first.record).unwrap();
    assert_eq!(k.completeness, "incomplete");
    assert_eq!(k.components[2].status, "present");
    assert_eq!(k.components[3].status, "missing");
    assert_eq!(k.components[3].file_stem, format!("sock_{style}_1_{art}_{variant}_cm"));
    assert!(v.reassignment_reason.contains("sub-index"));

    // NHL 10: no selector table — rows (none) and a contract that says why.
    let id = open(&reg, "xbox", "nhl10", "nhlng");
    let team = reg.list_teams(id).unwrap()[0].team_id;
    let v = reg.team_kits(id, &no_tree("xbox"), team).unwrap();
    assert_eq!(v.contract_status, "no_selector_table");
    assert!(v.contract.contains("NHL 09 / 10"));
    assert!(v.kits.is_empty() && v.spares.is_empty());

    // NHL 21 PS4: unmeasured — rows list, nothing is joined, nothing is complete.
    let id = open(&reg, "ps4", "nhl21", "nhl_game");
    let team = team_with_rows(&reg, id);
    let v = reg.team_kits(id, &no_tree("ps4"), team).unwrap();
    assert_eq!(v.contract_status, "unmeasured");
    assert!(v.contract.contains("PS4"));
    assert!(!v.kits.is_empty());
    assert!(v.kits.iter().all(|k| k.components.is_empty() && k.completeness == "unknown"));
    assert!(v.spares.is_empty());
}

#[test]
fn a_ps3_project_expects_rpsgl_and_names_a_platform_disagreement_with_the_attached_install() {
    let reg = Registry::default();
    let id = open(&reg, "ps3", "nhl15", "nhlng");
    let team = team_with_rows(&reg, id);
    let bare = reg.team_kits(id, &no_tree("ps3"), team).unwrap();
    let first = &bare.kits[0];
    let (style, art, variant) = (first.style, first.team_art_id, first.variant);
    let tmp = Tmp::new("ps3");
    tmp.put("jersey", &format!("texlib_{style}_{art}_{variant}.rpsgl"));
    tmp.put("pant", &format!("texlib_{style}_{art}_{variant}.rpsgl"));
    tmp.put("sock", &format!("sock_{style}_{art}_{variant}_cm.rpsgl"));
    let v = reg.team_kits(id, &tmp.args("ps3"), team).unwrap();
    let k = v.kits.iter().find(|k| k.record == first.record).unwrap();
    assert_eq!(k.completeness, "complete");
    assert!(k.components.iter().all(|c| c.path.as_deref().unwrap().ends_with(".rpsgl")));
    assert!(v.contract.contains(".rpsgl"));
    assert!(!v.notes.iter().any(|n| n.contains("attached install is read as")));

    // The same tree declared as an Xbox install: the contract still follows the PS3 database, and
    // the disagreement is named rather than silently resolved either way.
    let v = reg.team_kits(id, &tmp.args("xbox"), team).unwrap();
    assert!(v.notes.iter().any(|n| n.contains("attached install is read as `xbox`") && n.contains("`ps3`")), "{:?}", v.notes);
    assert_eq!(v.kits.iter().find(|k| k.record == first.record).unwrap().completeness, "complete", "the tree still carries the .rpsgl files the PS3 contract expects");
}
