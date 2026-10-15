//! ONB-003 §1 — flexible project inputs through the real [`Registry`] seam: a roster save alone, a
//! main database alone, extracted assets alone, a compatible combination, and a fresh project with
//! no game installation. Each mode opens, permits its own independent work, saves into the project
//! and reopens; an absent input gates only the features that need it. Every written path is a temp
//! tree this test owns; the committed anchors are read only.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::tdb_txn::FileIdentity;
use studio_domain::{BaselineStore, FieldValue};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn defs() -> String {
    repo("defs").to_string_lossy().into_owned()
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn store() -> BaselineStore {
    BaselineStore::open(&repo("baselines/INVENTORY.json"), &repo("anchors")).unwrap()
}
fn copy(from: &Path, to: &Path) {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(from, to).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", from.display(), to.display()));
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-onb003-inputs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Tmp(d)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const NHLNG: &str = "anchors/ps3/nhl14/databases/nhlng.db";
const META: &str = "anchors/ps3/nhl14/databases/nhlng-meta.xml";
const DICT: &str = "anchors/ps3/nhl14/databases/nhl_eng_us.db";
const ROSTER: &str = "anchors/ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA";

fn edit(reg: &Registry, id: u32) {
    reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(5)).unwrap();
    if !reg.summary(id).unwrap().dirty {
        reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(4)).unwrap();
    }
    assert!(reg.summary(id).unwrap().dirty);
}

/// A solid 8×8 PNG.
fn png(rgb: [u8; 3]) -> Vec<u8> {
    let rgba: Vec<u8> = (0..64).flat_map(|_| [rgb[0], rgb[1], rgb[2], 255]).collect();
    studio_domain::DecodedImage { width: 8, height: 8, rgba }.to_png().unwrap()
}

#[test]
fn roster_save_import_is_reserved_for_save_editor_without_writing() {
    let t = Tmp::new("save-editor-only");
    let src = t.0.join("ROSTER");
    copy(&repo(ROSTER), &src);
    let before = FileIdentity::of_file(&src).unwrap();
    let reg = Registry::default();
    let st = store();
    let sources = studio_api::dto::ImportSourcesDto { roster_save: Some(s(&src)), ..Default::default() };
    let review = reg.project_import_review("nhl14", parse_platform("ps3").unwrap(), &sources, Some(&st), &defs()).unwrap();
    assert!(!review.ok);
    assert!(review.items[0].detail.contains("Save Editor"));
    let dir = t.0.join("project");
    assert!(reg.project_create_import(&s(&dir), "nhl14", parse_platform("ps3").unwrap(), &sources, "", None, None, Some(&st), &defs()).is_err());
    assert!(!dir.exists());
    assert_eq!(before, FileIdentity::of_file(&src).unwrap());
}

#[test]
fn main_database_alone_without_an_installation() {
    let t = Tmp::new("db-only");
    let src_dir = t.0.join("mine");
    copy(&repo("anchors/ps4/nhl21/databases/nhl_game.db"), &src_dir.join("nhl_game.db"));
    copy(&repo("anchors/ps4/nhl21/databases/nhl_game-meta.xml"), &src_dir.join("nhl_game-meta.xml"));
    let reg = Registry::default();
    let ps4 = parse_platform("ps4").unwrap();
    let sources = studio_api::dto::ImportSourcesDto { main_db: Some(s(&src_dir.join("nhl_game.db"))), ..Default::default() };
    let review = reg.project_import_review("nhl21", ps4, &sources, None, &defs()).unwrap();
    assert!(review.ok, "{:?}", review.items.iter().map(|i| i.detail.clone()).collect::<Vec<_>>());
    assert_eq!(review.meta_source.as_deref(), Some("sibling"));
    let project_dir = t.0.join("project");
    let opened = reg.project_create_import(&s(&project_dir), "nhl21", ps4, &sources, "", None, None, None, &defs()).unwrap();
    let id = opened.summary.project_id;
    assert_eq!(opened.project.mode, "database");
    assert!(opened.project.working_roster_save.is_none());
    assert!(!opened.project.inputs.iter().any(|i| i.role == "roster_save"));
    assert!(opened.project.inputs.iter().any(|i| i.role == "assets" && i.origin == "absent"));
    assert!(opened.discovery.install_dir.is_none(), "no asset tree is pretended");
    assert!(!reg.list_teams(id).unwrap().is_empty());
    edit(&reg, id);
    assert!(reg.project_save(id).unwrap().saved);
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(!plan.deployable && plan.items.len() == 1 && plan.items[0].state == "unresolved", "{:?}", plan.reasons);
    assert!(reg.close_project(id));
    let reopened = reg.project_open(&s(&project_dir), &defs()).unwrap();
    assert_eq!(reopened.project.mode, "database");
    assert!(reg.close_project(reopened.summary.project_id));
}

#[test]
fn extracted_assets_alone_stage_into_the_project_and_gate_database_routes() {
    let t = Tmp::new("assets-only");
    let assets = t.0.join("extracted");
    let rel = "fe/ion/artassets/leaguelogos/l0.png";
    let logo = assets.join(rel);
    std::fs::create_dir_all(logo.parent().unwrap()).unwrap();
    std::fs::write(&logo, png([10, 20, 30])).unwrap();
    let original = FileIdentity::of_file(&logo).unwrap();
    let reg = Registry::default();
    let xbox = parse_platform("xbox").unwrap();
    let sources = studio_api::dto::ImportSourcesDto { assets_dir: Some(s(&assets)), ..Default::default() };
    let review = reg.project_import_review("nhllegacy", xbox, &sources, None, &defs()).unwrap();
    assert!(review.ok, "{:?}", review.items.iter().map(|i| i.detail.clone()).collect::<Vec<_>>());
    assert!(review.items.iter().any(|i| i.role == "assets" && i.detail.contains("fe (")), "{:?}", review.items.iter().map(|i| i.detail.clone()).collect::<Vec<_>>());
    // An empty folder is not an input.
    let empty = t.0.join("empty");
    std::fs::create_dir_all(&empty).unwrap();
    let bad = reg.project_import_review("nhllegacy", xbox, &studio_api::dto::ImportSourcesDto { assets_dir: Some(s(&empty)), ..Default::default() }, None, &defs()).unwrap();
    assert!(!bad.ok && bad.items[0].detail.contains("empty"));
    // Nothing at all is refused with the reason.
    let none = reg.project_import_review("nhllegacy", xbox, &studio_api::dto::ImportSourcesDto::default(), None, &defs()).unwrap();
    assert!(!none.ok && none.items[0].detail.contains("at least one input"));

    let project_dir = t.0.join("project");
    let opened = reg.project_create_import(&s(&project_dir), "nhllegacy", xbox, &sources, "", None, None, None, &defs()).unwrap();
    let id = opened.summary.project_id;
    assert_eq!(opened.summary.role, "assets");
    assert_eq!(opened.summary.table_count, 0);
    assert_eq!(opened.project.mode, "assets");
    assert_eq!(opened.discovery.install_dir.as_deref(), Some(s(&assets).as_str()), "the assets folder is the asset read root");
    assert_eq!(opened.summary.surfaces.players.state, "not-applicable");
    // Database routes say why, instead of "unknown project".
    let err = reg.list_players(id).err().expect("gated").to_string();
    assert!(err.contains("no main database or roster save"), "{err}");
    // The loose texture route works from the assets folder and stages into the project overlay.
    let src_png = t.0.join("new-logo.png");
    std::fs::write(&src_png, png([200, 0, 0])).unwrap();
    let plan = reg.texture_replace_plan(&s(&assets), rel, &s(&src_png), None).unwrap();
    let written = reg.import_texture(&s(&assets), rel, &s(&src_png), true, None, plan.review_id).unwrap();
    assert!(Path::new(&written.target_path).starts_with(project_dir.join("working").join("overlay")), "{}", written.target_path);
    assert!(written.backup_path.is_none(), "no .bak inside a project");
    assert_eq!(FileIdentity::of_file(&logo).unwrap(), original, "the extracted file is never written");
    let info = reg.project_info(id).unwrap();
    assert_eq!(info.staged_files.len(), 1);
    assert!(info.staged_files[0].source_sha256.is_some(), "the source image is kept with the project");
    assert_eq!(info.sources, 1);
    let sha = info.staged_files[0].source_sha256.clone().unwrap();
    assert!(project_dir.join("sources").join(format!("{sha}.png")).is_file());
    // Deployment of the staged output goes back to the assets folder, after review.
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(plan.deployable, "{:?}", plan.reasons);
    assert_eq!(plan.items.len(), 1);
    assert_eq!(Path::new(plan.items[0].destination_path.as_deref().unwrap()), logo.as_path());
    let applied = reg.project_deploy_apply(id, plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_ne!(FileIdentity::of_file(&logo).unwrap(), original);
    reg.project_deploy_restore(id, applied.seq).unwrap();
    assert_eq!(FileIdentity::of_file(&logo).unwrap(), original);
    // Close releases the binding; reopen gives the same assets-only project.
    let closed = reg.close_project_outcome(id);
    assert_eq!(closed.outcome, "closed", "{:?}", closed.reason);
    let reopened = reg.project_open(&s(&project_dir), &defs()).unwrap();
    assert_eq!(reopened.summary.role, "assets");
    assert_eq!(reopened.project.staged_files.len(), 1);
    assert!(reg.close_project(reopened.summary.project_id));
}

#[test]
fn a_combination_keeps_every_input_role_and_a_fresh_project_needs_no_installation() {
    let t = Tmp::new("combined");
    let src = t.0.join("mine");
    copy(&repo(NHLNG), &src.join("nhlng.db"));
    copy(&repo(META), &src.join("nhlng-meta.xml"));
    copy(&repo(DICT), &src.join("nhl_eng_us.db"));
    copy(&repo(ROSTER), &src.join("save/SYS-DATA"));
    let assets = t.0.join("extracted");
    std::fs::create_dir_all(assets.join("fe/x")).unwrap();
    std::fs::write(assets.join("fe/x/a.png"), png([1, 2, 3])).unwrap();
    let reg = Registry::default();
    let ps3 = parse_platform("ps3").unwrap();
    let sources = studio_api::dto::ImportSourcesDto {
        main_db: Some(s(&src.join("nhlng.db"))),
        meta: None,
        dictionary: Some(s(&src.join("nhl_eng_us.db"))),
        roster_save: None,
        assets_dir: Some(s(&assets)),
    };
    let opened = reg.project_create_import(&s(&t.0.join("p")), "nhl14", ps3, &sources, "", None, None, None, &defs()).unwrap();
    assert!(!opened.summary.roster_attached);
    let inputs: Vec<(String, String)> = opened.project.inputs.iter().map(|i| (i.role.clone(), i.origin.clone())).collect();
    for role in ["main_db", "dictionary", "assets"] {
        assert!(inputs.contains(&(role.into(), "import".into())), "{role}: {inputs:?}");
    }
    assert_eq!(opened.project.destinations.assets_dir.as_deref(), Some(s(&assets).as_str()));
    assert_eq!(opened.project.destinations.install_dir.as_deref(), Some(s(&assets).as_str()));
    // The project folder may not be inside the assets folder.
    let err = reg.project_create_import(&s(&assets.join("p")), "nhl14", ps3, &sources, "", None, None, None, &defs()).err().expect("refused").to_string();
    assert!(err.contains("inside the extracted-assets folder"), "{err}");
    assert!(reg.close_project(opened.summary.project_id));

    // Start fresh with no installation: the bundled baseline alone.
    let st = store();
    let fresh = reg.project_create_fresh(&s(&t.0.join("fresh")), "nhl14", ps3, "", None, None, &st, &defs()).unwrap();
    let id = fresh.summary.project_id;
    assert!(!fresh.summary.roster_attached);
    assert_eq!(fresh.project.destinations.picked_root, "");
    assert!(fresh.project.inputs.iter().any(|i| i.role == "assets" && i.origin == "absent"));
    edit(&reg, id);
    reg.project_save(id).unwrap();
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(!plan.deployable && plan.items.iter().all(|i| i.state == "unresolved"));
    // Relink to an owned RPCS3-shaped install later: discovery resolves, deployment opens up.
    let rpcs3 = t.0.join("rpcs3");
    let usrdir = rpcs3.join("dev_hdd0/game/BLUS31179/USRDIR");
    copy(&repo(NHLNG), &usrdir.join("nhlng.db"));
    copy(&repo(META), &usrdir.join("nhlng-meta.xml"));
    copy(&repo(DICT), &usrdir.join("nhl_eng_us.db"));
    copy(&repo(ROSTER), &rpcs3.join("dev_hdd0/home/00000001/savedata/BLUS311790200 - ROSTER/SYS-DATA"));
    let relinked = reg.project_relink(id, Some(&s(&rpcs3)), None, None, None).unwrap();
    assert_eq!(relinked.project.destinations.picked_root, s(&rpcs3));
    assert!(relinked.project.destinations.main_db_path.is_some() && relinked.project.destinations.roster_save.is_none(), "{:?} / {:?}", relinked.project.destinations.main_db_path, relinked.project.destinations.roster_save);
    assert!(relinked.warnings.is_empty(), "{:?}", relinked.warnings);
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(plan.deployable, "{:?}", plan.reasons);
    // A relink that would put the project inside the installation is refused.
    let err = reg.project_relink(id, Some(&s(&t.0)), None, None, None).err().expect("refused").to_string();
    assert!(err.contains("contains the game installation") || err.contains("inside"), "{err}");
    assert!(reg.close_project(id));
}

#[test]
fn nhl21_names_and_leagues_survive_edit_save_reopen_without_assets() {
    let t = Tmp::new("nhl21-names");
    let reg = Registry::default();
    let dir = t.0.join("project");
    let opened = reg.project_create_fresh(&s(&dir), "nhl21", parse_platform("ps4").unwrap(), "", None, None, &store(), &defs()).unwrap();
    let id = opened.summary.project_id;
    let teams = reg.list_teams(id).unwrap();
    // Optional developer rendering evidence: serialize actual read-only DTOs for a browser fixture.
    if let Ok(path) = std::env::var("NHL_STUDIO_QA_EXPORT") {
        let players = reg.list_players(id).unwrap();
        let fixtures = serde_json::json!({
            "project_open": &opened,
            "project_summary": reg.summary(id).unwrap(),
            "project_info": &opened.project,
            "list_teams": &teams,
            "team_view": reg.team_view(id, teams[0].team_id).unwrap(),
            "list_leagues": reg.list_leagues(id).unwrap(),
            "league_view": reg.league_view(id, 0).unwrap(),
            "list_players": &players,
            "player_view": reg.player_view(id, players[0].player_id).unwrap(),
            "list_arenas": reg.list_arenas(id).unwrap(),
        });
        std::fs::write(path, serde_json::to_vec(&fixtures).unwrap()).unwrap();
    }
    assert_eq!(teams[0].name, "Anaheim Ducks®");
    assert!(teams.iter().take(30).all(|t| !t.name.trim().is_empty()));
    assert_eq!(reg.team_view(id, teams[0].team_id).unwrap().unwrap().name, teams[0].name);
    // Every league detail must be readable, including empty/custom groups.
    let catalog = reg.list_leagues(id).unwrap();
    for league in &catalog.leagues {
        let view = reg.league_view(id, league.code as u64).unwrap().unwrap();
        assert_eq!(view.summary.code, league.code);
        for member in view.members {
            assert_eq!(member.name, teams.iter().find(|t| t.team_id == member.team_id).unwrap().name);
        }
    }
    // Inline names remain authoritative even without the optional dictionary.
    reg.detach_dictionary(id).unwrap();
    reg.set_field(id, "kOtt", teams[0].record, "AUsv", FieldValue::Text("Overnight Ducks".into())).unwrap();
    assert_eq!(reg.list_teams(id).unwrap()[0].name, "Overnight Ducks");
    assert_eq!(reg.team_view(id, teams[0].team_id).unwrap().unwrap().name, "Overnight Ducks");
    let league = &catalog.leagues[0];
    reg.set_field(id, "fsLK", league.record.unwrap(), "vIys", FieldValue::Text("AHL".into())).unwrap();
    assert_eq!(reg.league_view(id, league.code as u64).unwrap().unwrap().summary.name_key.as_deref(), Some("AHL"));
    assert!(reg.project_save(id).unwrap().saved);
    assert!(reg.close_project(id));
    let reopened = reg.project_open(&s(&dir), &defs()).unwrap();
    let id = reopened.summary.project_id;
    assert_eq!(reg.list_teams(id).unwrap()[0].name, "Overnight Ducks");
    assert_eq!(reg.league_view(id, league.code as u64).unwrap().unwrap().summary.name_key.as_deref(), Some("AHL"));
    assert!(reg.close_project(id));
}

