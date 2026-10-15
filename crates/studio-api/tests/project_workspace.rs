//! ONB-002 — the project-folder workflow through the real [`Registry`] seam (no Tauri): fresh
//! creation from the bundled baseline inventory, import creation from owned copies, isolation of
//! anchors / sources / destinations through edit → save → close → reopen, the write guards, the
//! reviewed deployment with backups and restore, two projects on one destination, the explicit
//! baseline gaps, and the extraction service on an owned synthetic install.
//!
//! Every written path is a temp tree this test owns; the committed anchors are read only and are
//! hashed before and after.

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
fn id_of(p: &Path) -> FileIdentity {
    FileIdentity::of_file(p).unwrap()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms-onb002-{tag}-{}", std::process::id()));
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

#[test]
fn xbox_save_destination_can_be_linked_then_cleared_without_changing_the_save() {
    let t = Tmp::new("xbox-save-unlink");
    let game = t.0.join("game");
    let saves = t.0.join("chosen-saves");
    std::fs::create_dir(&game).unwrap();
    std::fs::write(game.join("fixture.asset"), b"owned asset-root fixture").unwrap();
    std::fs::create_dir(&saves).unwrap();
    let save = saves.join("ROSTER");
    std::fs::copy(repo("anchors/xbox360/nhl14/savefiles/ROSTER 20260627054746/ROSTER 20260627054746"), &save).unwrap();
    let before = id_of(&save);
    let source = studio_api::dto::ImportSourcesDto {
        main_db: None, meta: None, dictionary: None, roster_save: None,
        assets_dir: Some(s(&game)),
    };
    let project = t.0.join("project");
    let reg = Registry::default();
    let opened = reg.project_create_import(&s(&project), "nhl14", parse_platform("xbox360").unwrap(),
        &source, &s(&game), Some(&s(&saves)), None, None, &defs()).unwrap();
    assert!(opened.project.destinations.roster_save.is_none());
    reg.project_relink(opened.summary.project_id, None, Some(""), None, None).unwrap();
    drop(reg);
    let reopened = Registry::default().project_open(&s(&project), &defs()).unwrap();
    assert!(reopened.project.destinations.roster_save.is_none());
    assert!(reopened.project.destinations.save_root.is_none());
    assert_eq!(before, id_of(&save));
}

/// An owned RPCS3-shaped NHL 14 install (vanilla copies): the deployment destination.
struct Install {
    root: PathBuf,
    nhlng: PathBuf,
    dict: PathBuf,
    roster: PathBuf,
}
fn ps3_install(base: &Path) -> Install {
    let root = base.join("rpcs3");
    let usrdir = root.join("dev_hdd0/game/BLUS31179/USRDIR");
    std::fs::create_dir_all(&usrdir).unwrap();
    let nhlng = usrdir.join("nhlng.db");
    std::fs::copy(repo(NHLNG), &nhlng).unwrap();
    std::fs::copy(repo(META), usrdir.join("nhlng-meta.xml")).unwrap();
    let dict = usrdir.join("nhl_eng_us.db");
    std::fs::copy(repo(DICT), &dict).unwrap();
    let save = root.join("dev_hdd0/home/00000001/savedata/BLUS311790200 - ROSTER");
    std::fs::create_dir_all(&save).unwrap();
    let roster = save.join("SYS-DATA");
    std::fs::copy(repo(ROSTER), &roster).unwrap();
    Install { root, nhlng, dict, roster }
}
fn tree_hashes(root: &Path) -> Vec<(String, FileIdentity)> {
    let mut out = Vec::new();
    fn walk(d: &Path, root: &Path, out: &mut Vec<(String, FileIdentity)>) {
        for e in std::fs::read_dir(d).unwrap().flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, root, out);
            } else if !p.to_string_lossy().ends_with(".nhlms-extracted.json") {
                // Extraction markers are the service's own records (timestamped); the comparison
                // is about the installation's files.
                out.push((p.strip_prefix(root).unwrap().to_string_lossy().into_owned(), id_of(&p)));
            }
        }
    }
    walk(root, root, &mut out);
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}
fn anchor_hashes() -> Vec<FileIdentity> {
    [NHLNG, META, DICT, ROSTER].iter().map(|r| id_of(&repo(r))).collect()
}
fn edit(reg: &Registry, id: u32, value: i64) {
    reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(value)).unwrap();
    if !reg.summary(id).unwrap().dirty {
        // The record already held `value`: pick the neighbour so the session really changes.
        reg.set_field(id, "ubPc", 0, "vjla", FieldValue::Int(value - 1)).unwrap();
    }
    assert!(reg.summary(id).unwrap().dirty);
}
fn same_path(a: Option<&str>, b: &Path) -> bool {
    a.is_some_and(|a| Path::new(a) == b)
}
fn player_position(reg: &Registry, id: u32) -> String {
    let players = reg.list_players(id).unwrap();
    let view = reg.player_view(id, players[0].player_id).unwrap().unwrap();
    view.identity.iter().find(|f| f.name == "position").map(|f| f.display.clone()).unwrap_or_default()
}

#[test]
fn fresh_project_is_isolated_saves_into_itself_and_reopens_with_its_edit() {
    let t = Tmp::new("fresh");
    let install = ps3_install(&t.0);
    let install_before = tree_hashes(&install.root);
    let anchors_before = anchor_hashes();
    let reg = Registry::default();
    let st = store();
    let project_dir = t.0.join("project-a");

    // Inside the install: refused, nothing created.
    let err = reg
        .project_create_fresh(&s(&install.root.join("studio-project")), "nhl14", parse_platform("ps3").unwrap(), &s(&install.root), None, None, &st, &defs())
        .err()
        .expect("refused")
        .to_string();
    assert!(err.contains("inside the game installation"), "{err}");
    assert!(!install.root.join("studio-project").exists());

    let opened = reg
        .project_create_fresh(&s(&project_dir), "nhl14", parse_platform("ps3").unwrap(), &s(&install.root), None, None, &st, &defs())
        .unwrap();
    let id = opened.summary.project_id;
    assert_eq!(opened.summary.game_id.as_deref(), Some("nhl14"));
    assert!(!opened.summary.roster_attached);
    assert!(opened.project.working_roster_save.is_none());
    assert!(!project_dir.join("working/saves").exists());
    assert!(!project_dir.join("baseline/saves").exists());
    assert_eq!(opened.project.source_kind, "fresh");
    assert!(opened.project.gaps.is_empty(), "NHL 14 PS3 is fully available: {:?}", opened.project.gaps.iter().map(|g| &g.component).collect::<Vec<_>>());
    assert!(same_path(opened.project.destinations.main_db_path.as_deref(), &install.nhlng), "{:?}", opened.project.destinations.main_db_path);
    assert!(opened.project.destinations.roster_save.is_none());
    assert!(opened.project.destinations.save_root.is_none());
    assert!(opened.project.legacy_sidecar.is_none());
    assert!(!install.root.join("dev_hdd0/game/BLUS31179/USRDIR/.nhlmodproj").exists(), "no sidecar is written into the install");
    let working_db = PathBuf::from(opened.project.working_main_db.as_deref().unwrap());
    assert!(working_db.starts_with(&project_dir));
    assert_eq!(id_of(&working_db), id_of(&repo(NHLNG)), "the working copy starts as the verified anchor");
    assert_eq!(id_of(&project_dir.join("baseline/db/nhlng.db")), id_of(&repo(NHLNG)));
    assert!(project_dir.join("baseline/BASELINE.json").is_file());
    // The project id is a fresh session; the summary is the usual one.
    assert_eq!(reg.summary(id).unwrap().project_id, id);

    // Edit, then save the project: only the working copy changes.
    edit(&reg, id, 2);
    let pos = player_position(&reg, id);
    let saved = reg.project_save(id).unwrap();
    assert!(saved.saved && !saved.nothing_to_save);
    assert_eq!(saved.seq, Some(1));
    assert!(!reg.summary(id).unwrap().dirty);
    assert_ne!(id_of(&working_db), id_of(&repo(NHLNG)), "the working copy holds the edit");
    assert_eq!(tree_hashes(&install.root), install_before, "the installation is untouched by edit + save");
    assert_eq!(anchor_hashes(), anchors_before, "the anchors are untouched");
    let again = reg.project_save(id).unwrap();
    assert!(again.nothing_to_save);

    // The guards: a Save As into the installation is refused and writes nothing.
    let err = reg.save(id, &s(&install.nhlng)).err().expect("refused").to_string();
    assert!(err.contains("would write into"), "{err}");
    let err = reg.save(id, &s(&install.root.join("dev_hdd0/game/BLUS31179/USRDIR/copy.db"))).err().expect("refused").to_string();
    assert!(err.contains("would write into"), "{err}");
    assert_eq!(tree_hashes(&install.root), install_before);
    let export = project_dir.join("exports/nhlng.db");
    reg.save(id, &s(&export)).unwrap();
    assert!(export.is_file());

    // Close and reopen: the saved edit is the new session's baseline; the manifest counts the save.
    assert!(reg.close_project(id));
    let reopened = reg.project_open(&s(&project_dir), &defs()).unwrap();
    let id2 = reopened.summary.project_id;
    assert!(!reopened.summary.dirty);
    assert_eq!(player_position(&reg, id2), pos, "the saved edit is what reopens");
    assert_eq!(reopened.project.saves, 1);
    assert!(reopened.project.last_saved.is_some());
    // A frontend reconnect reuses the existing native session.
    assert_eq!(reg.project_open(&s(&project_dir), &defs()).unwrap().summary.project_id, id2);
    assert_eq!(tree_hashes(&install.root), install_before, "reopening touches nothing in the install");
}

#[test]
fn deployment_is_explicit_reviewed_backed_up_and_restorable() {
    let t = Tmp::new("deploy");
    let install = ps3_install(&t.0);
    let reg = Registry::default();
    let st = store();
    let project_dir = t.0.join("project");
    let opened = reg
        .project_create_fresh(&s(&project_dir), "nhl14", parse_platform("ps3").unwrap(), &s(&install.root), None, None, &st, &defs())
        .unwrap();
    let id = opened.summary.project_id;
    let before = tree_hashes(&install.root);
    let orig_nhlng = id_of(&install.nhlng);
    let orig_roster = id_of(&install.roster);

    // Nothing differs yet: every item is already current and the plan says so.
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(!plan.deployable);
    assert!(plan.reasons.iter().any(|r| r.contains("nothing to deploy")), "{:?}", plan.reasons);
    assert!(plan.items.iter().all(|i| i.state == "already-current"), "{:?}", plan.items.iter().map(|i| (&i.key, &i.state)).collect::<Vec<_>>());

    // An unsaved edit never deploys.
    edit(&reg, id, 2);
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(plan.dirty && !plan.deployable);
    assert!(plan.reasons.iter().any(|r| r.contains("unsaved")), "{:?}", plan.reasons);
    let err = reg.project_deploy_apply(id, plan.handle).err().expect("refused").to_string();
    assert!(err.contains("unsaved"), "{err}");
    assert_eq!(tree_hashes(&install.root), before);

    // Save, review, apply: only the changed database is written; saves remain untouched.
    reg.project_save(id).unwrap();
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(plan.deployable, "{:?}", plan.reasons);
    let keys: Vec<(&str, &str)> = plan.items.iter().map(|i| (i.key.as_str(), i.state.as_str())).collect();
    assert!(keys.contains(&("main_db", "unchanged")), "{keys:?}");
    assert!(!keys.iter().any(|(key, _)| *key == "roster"), "{keys:?}");
    assert!(keys.contains(&("dictionary", "already-current")), "{keys:?}");
    assert!(same_path(plan.items.iter().find(|i| i.key == "main_db").unwrap().destination_path.as_deref(), &install.nhlng));
    // A stale handle (superseded by a new review) is refused.
    let stale = plan.handle;
    let plan = reg.project_deploy_plan(id, false).unwrap();
    let err = reg.project_deploy_apply(id, stale).err().expect("refused").to_string();
    assert!(err.contains("not current"), "{err}");
    let applied = reg.project_deploy_apply(id, plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    assert_eq!(applied.seq, 1);
    let working_db = PathBuf::from(opened.project.working_main_db.as_deref().unwrap());
    assert_eq!(id_of(&install.nhlng), id_of(&working_db), "the destination holds the project's database");
    assert_eq!(id_of(&install.roster), orig_roster);
    assert_eq!(id_of(&install.dict), id_of(&repo(DICT)), "the unchanged dictionary was not rewritten");
    let record_dir = PathBuf::from(&applied.record_dir);
    assert!(record_dir.starts_with(&project_dir));
    assert!(record_dir.join("record.json").is_file());
    let backups: Vec<_> = std::fs::read_dir(record_dir.join("backups")).unwrap().flatten().collect();
    assert_eq!(backups.len(), 1, "one backup per replaced destination");
    assert!(applied.destinations.iter().all(|d| d.state == "deployed"));
    // Applying twice is refused (the handle was consumed) and the files are now current.
    let err = reg.project_deploy_apply(id, plan.handle).err().expect("refused").to_string();
    assert!(err.contains("not current"), "{err}");
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(plan.items.iter().all(|i| i.state == "already-current"));
    let list = reg.project_deployments(id).unwrap();
    assert_eq!((list.len(), list[0].seq, list[0].state.as_str(), list[0].restorable), (1, 1, "committed", true));

    // Restore: the originals come back byte for byte.
    let restored = reg.project_deploy_restore(id, 1).unwrap();
    assert_eq!(restored.state, "restored");
    assert_eq!(id_of(&install.nhlng), orig_nhlng);
    assert_eq!(id_of(&install.roster), orig_roster);
    assert_eq!(tree_hashes(&install.root), before, "the install is exactly as it was");
    assert!(!reg.project_deployments(id).unwrap()[0].restorable);
    let err = reg.project_deploy_restore(id, 1).err().expect("refused").to_string();
    assert!(err.contains("nothing to restore or recover"), "{err}");

    // A destination changed outside Studio blocks the plan until the user allows it.
    let mut bytes = std::fs::read(&install.nhlng).unwrap();
    let last = bytes.len() - 1;
    bytes[last] ^= 0xFF;
    std::fs::write(&install.nhlng, &bytes).unwrap();
    let plan = reg.project_deploy_plan(id, false).unwrap();
    let item = plan.items.iter().find(|i| i.key == "main_db").unwrap();
    assert_eq!(item.state, "changed-externally");
    assert!(!plan.deployable && item.blocked.as_deref().is_some_and(|b| b.contains("changed outside Studio")));
    let plan = reg.project_deploy_plan(id, true).unwrap();
    assert!(plan.deployable, "{:?}", plan.reasons);
    // ...and a change between review and apply is refused by the publisher's identity check.
    std::fs::write(&install.nhlng, std::fs::read(repo(NHLNG)).unwrap()).unwrap();
    let err = reg.project_deploy_apply(id, plan.handle).err().expect("refused").to_string();
    assert!(err.contains("changed since") || err.contains("is "), "{err}");
    assert_eq!(id_of(&install.nhlng), orig_nhlng, "the refused apply wrote nothing");
    assert!(!project_dir.join("deployments/2").exists(), "a refused deployment leaves no record behind");
}

#[test]
fn two_projects_on_one_destination_keep_their_own_staging() {
    let t = Tmp::new("two");
    let install = ps3_install(&t.0);
    let before = tree_hashes(&install.root);
    let reg = Registry::default();
    let st = store();
    let a = reg.project_create_fresh(&s(&t.0.join("a")), "nhl14", parse_platform("ps3").unwrap(), &s(&install.root), None, None, &st, &defs()).unwrap();
    let b = reg.project_create_fresh(&s(&t.0.join("b")), "nhl14", parse_platform("ps3").unwrap(), &s(&install.root), None, None, &st, &defs()).unwrap();
    edit(&reg, a.summary.project_id, 2);
    let pa = player_position(&reg, a.summary.project_id);
    // B gets a different position than A ended up with (the record's starting value decides
    // which of the skater positions that is).
    let mut pb = String::new();
    for candidate in [3, 2, 1] {
        reg.set_field(b.summary.project_id, "ubPc", 0, "vjla", FieldValue::Int(candidate)).unwrap();
        pb = player_position(&reg, b.summary.project_id);
        if pb != pa && reg.summary(b.summary.project_id).unwrap().dirty {
            break;
        }
    }
    assert_ne!(pa, pb);
    reg.project_save(a.summary.project_id).unwrap();
    reg.project_save(b.summary.project_id).unwrap();
    assert!(reg.close_project(a.summary.project_id) && reg.close_project(b.summary.project_id));
    let a2 = reg.project_open(&s(&t.0.join("a")), &defs()).unwrap();
    let b2 = reg.project_open(&s(&t.0.join("b")), &defs()).unwrap();
    assert_eq!(player_position(&reg, a2.summary.project_id), pa);
    assert_eq!(player_position(&reg, b2.summary.project_id), pb);
    assert_eq!(tree_hashes(&install.root), before);
    // The old install-side sidecar is never consulted or written.
    assert!(!install.root.join("dev_hdd0/game/BLUS31179/USRDIR/.nhlmodproj").exists());
}

#[test]
fn import_main_database_without_sibling_schema_uses_bundled_metadata() {
    let t = Tmp::new("import-disc-schema");
    std::fs::create_dir(t.0.join("disc")).unwrap();
    let source = t.0.join("disc/NHLNG.DB");
    std::fs::copy(repo("anchors/ps3/nhl08/databases/nhlng.db"), &source).unwrap();
    let original = std::fs::read(&source).unwrap();
    let sources = studio_api::dto::ImportSourcesDto {
        main_db: Some(s(&source)), meta: None, dictionary: None,
        roster_save: None, assets_dir: None,
    };
    let reg = Registry::default();
    let baseline = store();
    let platform = parse_platform("ps3").unwrap();
    let review = reg.project_import_review("nhl08",platform,&sources,Some(&baseline),&defs()).unwrap();
    assert!(review.ok, "{}", serde_json::to_string(&review).unwrap());
    assert!(review.meta_source.unwrap().starts_with("bundled"));
    let project = reg.project_create_import(&s(&t.0.join("project")),"nhl08",platform,&sources,"",None,None,Some(&baseline),&defs()).unwrap();
    assert!(reg.list_players(project.summary.project_id).unwrap().len()>2000);
    assert_eq!(std::fs::read(&source).unwrap(),original);
    // An explicitly selected missing schema must still fail, never silently fall back.
    let explicit = studio_api::dto::ImportSourcesDto { meta:Some(s(&t.0.join("missing.xml"))), ..sources };
    assert!(!reg.project_import_review("nhl08",platform,&explicit,Some(&baseline),&defs()).unwrap().ok);
}

#[test]
fn import_project_copies_the_sources_and_never_writes_them() {
    let t = Tmp::new("import");
    let install = ps3_install(&t.0);
    let src = t.0.join("my-files");
    std::fs::create_dir_all(src.join("save")).unwrap();
    std::fs::copy(repo(NHLNG), src.join("nhlng.db")).unwrap();
    std::fs::copy(repo(META), src.join("nhlng-meta.xml")).unwrap();
    std::fs::copy(repo(DICT), src.join("nhl_eng_us.db")).unwrap();
    std::fs::copy(repo(ROSTER), src.join("save/SYS-DATA")).unwrap();
    std::fs::write(src.join("save/PARAM.SFO"), b"not a real sfo, just a companion file").unwrap();
    let sources_before = tree_hashes(&src);
    let reg = Registry::default();
    let sources = studio_api::dto::ImportSourcesDto {
        main_db: Some(s(&src.join("nhlng.db"))),
        meta: None,
        dictionary: Some(s(&src.join("nhl_eng_us.db"))),
        roster_save: None,
        assets_dir: None,
    };
    let review = reg.project_import_review("nhl14", parse_platform("ps3").unwrap(), &sources, None, &defs()).unwrap();
    assert!(review.ok, "{:?}", review.items.iter().map(|i| (&i.role, &i.detail)).collect::<Vec<_>>());
    assert_eq!(review.items.len(), 3);
    // A wrong file is named, not silently accepted.
    let bad = studio_api::dto::ImportSourcesDto { roster_save: Some(s(&src.join("nhl_eng_us.db"))), ..sources.clone() };
    let review = reg.project_import_review("nhl14", parse_platform("ps3").unwrap(), &bad, None, &defs()).unwrap();
    assert!(!review.ok);
    assert!(review.items.iter().any(|i| i.role == "roster_save" && !i.ok && i.detail.contains("Save Editor")));
    let err = reg
        .project_create_import(&s(&t.0.join("p")), "nhl14", parse_platform("ps3").unwrap(), &bad, &s(&install.root), None, None, None, &defs())
        .err()
        .expect("refused")
        .to_string();
    assert!(err.contains("not accepted"), "{err}");
    assert!(!t.0.join("p").exists());

    let opened = reg
        .project_create_import(&s(&t.0.join("p")), "nhl14", parse_platform("ps3").unwrap(), &sources, &s(&install.root), None, None, None, &defs())
        .unwrap();
    assert_eq!(opened.project.source_kind, "import");
    assert!(!opened.summary.roster_attached);
    let roles: Vec<&str> = opened.project.components.iter().map(|c| c.role.as_str()).collect();
    assert!(roles.contains(&"main_db") && roles.contains(&"meta") && roles.contains(&"dictionary") && !roles.contains(&"roster_save") && !roles.contains(&"roster_save_aux"), "{roles:?}");
    assert!(!t.0.join("p/working/saves").exists());
    let id = opened.summary.project_id;
    edit(&reg, id, 2);
    reg.project_save(id).unwrap();
    assert_eq!(tree_hashes(&src), sources_before, "the sources are never written");
    // A Save As into an imported source is refused too.
    let err = reg.save(id, &s(&src.join("nhlng.db"))).err().expect("refused").to_string();
    assert!(err.contains("imported main db"), "{err}");
    // A project folder inside the sources is refused.
    let err = reg
        .project_create_import(&s(&src.join("proj")), "nhl14", parse_platform("ps3").unwrap(), &sources, &s(&install.root), None, None, None, &defs())
        .err()
        .expect("refused")
        .to_string();
    assert!(err.contains("inside the imported main database's folder"), "{err}");
}

#[test]
fn baseline_gaps_are_explicit_never_substituted() {
    let t = Tmp::new("gaps");
    let reg = Registry::default();
    let st = store();
    let empty = t.0.join("no-install");
    std::fs::create_dir_all(&empty).unwrap();
    // NHL 20 PS4: the committed set is NHL 19 base; Start fresh is refused with that reason.
    let err = reg
        .project_create_fresh(&s(&t.0.join("nhl20")), "nhl20", parse_platform("ps4").unwrap(), &s(&empty), None, None, &st, &defs())
        .err()
        .expect("refused")
        .to_string();
    assert!(err.contains("not available") && err.contains("NHL 19 base"), "{err}");
    assert!(!t.0.join("nhl20").exists());
    // NHL 21 PS4: available without a dictionary or roster save; both gaps travel with the project.
    let opened = reg
        .project_create_fresh(&s(&t.0.join("nhl21")), "nhl21", parse_platform("ps4").unwrap(), &s(&empty), None, None, &st, &defs())
        .unwrap();
    assert_eq!(opened.summary.game_id.as_deref(), Some("nhl21"));
    assert!(!opened.summary.roster_attached);
    let gaps: Vec<&str> = opened.project.gaps.iter().map(|g| g.component.as_str()).collect();
    assert_eq!(gaps, ["dictionary"]);
    assert!(opened.project.working_dictionary.is_none() && opened.project.working_roster_save.is_none());
    assert!(opened.project.destinations.main_db_path.is_none(), "no install was found: the destination stays unresolved");
    let plan = reg.project_deploy_plan(opened.summary.project_id, false).unwrap();
    assert!(plan.items.iter().any(|i| i.key == "main_db" && i.state == "unresolved"));
    // Recompiled: the derived Legacy Xbox 360 baseline, labelled as such.
    let opened = reg
        .project_create_fresh(&s(&t.0.join("recomp")), "legacyrecomp", parse_platform("recomp").unwrap(), &s(&empty), None, None, &st, &defs())
        .unwrap();
    assert!(opened.project.components.iter().all(|c| c.provenance == "derived"), "{:?}", opened.project.components.iter().map(|c| &c.provenance).collect::<Vec<_>>());
    assert!(opened.project.components.iter().any(|c| c.witness.contains("R128")));
    assert!(!opened.summary.roster_attached);
    // PS4 fresh projects no longer substitute a live save for the template.
    // This does not qualify default-database editing or remove its existing guards.
    let opened = reg
        .project_create_fresh(&s(&t.0.join("nhl15ps4")), "nhl15", parse_platform("ps4").unwrap(), &s(&empty), None, None, &st, &defs())
        .unwrap();
    assert_eq!(opened.summary.role, "nhlng");
    assert!(opened.project.working_roster_save.is_none());
    assert!(!opened.summary.roster_attached);
    // A damaged bundle is never used as a baseline.
    let bad_dir = t.0.join("bad-bundle");
    std::fs::create_dir_all(bad_dir.join("files/ps3/nhl14/databases")).unwrap();
    std::fs::copy(repo("baselines/INVENTORY.json"), bad_dir.join("INVENTORY.json")).unwrap();
    std::fs::write(bad_dir.join("files/ps3/nhl14/databases/nhlng.db"), b"not the anchor").unwrap();
    let bad = BaselineStore::open_bundled(&bad_dir).unwrap().unwrap();
    let err = reg
        .project_create_fresh(&s(&t.0.join("from-bad")), "nhl14", parse_platform("ps3").unwrap(), &s(&empty), None, None, &bad, &defs())
        .err()
        .expect("refused")
        .to_string();
    assert!(err.contains("damaged or substituted") || err.contains("cannot be read") || err.contains("io error"), "{err}");
    assert!(!t.0.join("from-bad").exists(), "a failed creation leaves no half-made project");
}

#[test]
fn every_shipping_target_with_an_available_baseline_creates_and_opens() {
    let t = Tmp::new("all27");
    let reg = Registry::default();
    let st = store();
    let empty = t.0.join("no-install");
    std::fs::create_dir_all(&empty).unwrap();
    let mut opened_count = 0;
    let mut unavailable = Vec::new();
    for target in studio_domain::SHIPPING_TARGETS {
        let dir = t.0.join(format!("{}-{}", target.game_id, target.platform.as_str()));
        match reg.project_create_fresh(&s(&dir), target.game_id, target.platform, &s(&empty), None, None, &st, &defs()) {
            Ok(o) => {
                assert!(!o.summary.roster_attached);
                assert!(o.project.working_roster_save.is_none());
                assert!(o.project.destinations.roster_save.is_none());
                assert!(o.project.destinations.save_root.is_none());
                assert!(!o.project.components.iter().any(|c| c.role.starts_with("roster_save")));
                assert!(!o.project.gaps.iter().any(|g| g.component.starts_with("roster_save")));
                assert_eq!(o.summary.game_id.as_deref(), Some(target.game_id));
                assert_eq!(o.project.platform, target.platform.as_str());
                let teams = reg.list_teams(o.summary.project_id).unwrap();
                assert!(!teams.is_empty(), "{} {}", target.game_id, target.platform.as_str());
                assert!(teams.iter().take(30).all(|team| !team.name.trim().is_empty()), "{} {} has unnamed NHL clubs", target.game_id, target.platform.as_str());
                let detail = reg.team_view(o.summary.project_id, teams[0].team_id).unwrap().unwrap();
                assert_eq!(detail.name, teams[0].name);

                assert!(reg.close_project(o.summary.project_id));
                opened_count += 1;
            }
            Err(e) => unavailable.push(format!("{} {}: {e}", target.game_id, target.platform.as_str())),
        }
    }
    assert_eq!(unavailable.len(), 1, "{unavailable:?}");
    assert!(unavailable[0].starts_with("nhl20 ps4"));
    assert_eq!(opened_count, 26);
}

#[test]
fn fresh_project_ignores_legacy_save_arguments_and_preserves_the_save() {
    let t = Tmp::new("fresh-ignore-saves");
    let install = ps3_install(&t.0);
    let before = id_of(&install.roster);
    let reg = Registry::default();
    let opened = reg.project_create_fresh(
        &s(&t.0.join("project")), "nhl14", parse_platform("ps3").unwrap(),
        &s(&install.root), Some(&s(install.roster.parent().unwrap())),
        Some("a stale roster selection that must not be resolved"), &store(), &defs(),
    ).unwrap();
    assert!(!opened.summary.roster_attached);
    assert!(opened.project.working_roster_save.is_none());
    assert!(opened.project.destinations.roster_save.is_none());
    assert!(opened.project.destinations.save_root.is_none());
    assert_eq!(before, id_of(&install.roster));
}

#[test]
fn legacy_paired_project_preserves_save_but_never_edits_builds_or_deploys_it() {
    use studio_domain::{EditSession, Project, ProjectRole};
    let t = Tmp::new("legacy-pair-separation");
    let install = ps3_install(&t.0);
    let reg = Registry::default();
    let st = store();
    let dir = t.0.join("project");
    let platform = parse_platform("ps3").unwrap();
    let opened = reg.project_create_fresh(&s(&dir), "nhl14", platform, &s(&install.root), None, None, &st, &defs()).unwrap();
    reg.close_project(opened.summary.project_id);
    // Recreate the old paired manifest with a save-only edit that must be retained.
    let save = dir.join("working/saves/legacy/SYS-DATA");
    let baseline = dir.join("baseline/saves/legacy/SYS-DATA");
    std::fs::create_dir_all(save.parent().unwrap()).unwrap();
    std::fs::create_dir_all(baseline.parent().unwrap()).unwrap();
    std::fs::copy(repo(ROSTER), &baseline).unwrap();
    let mut session = EditSession::begin(Project::load(repo(ROSTER), platform, ProjectRole::Roster, repo(META), defs()).unwrap());
    session.set_field("ubPc", 0, "vjla", FieldValue::Int(7)).unwrap();
    std::fs::write(&save, session.save().unwrap()).unwrap();
    let before = id_of(&save);
    let installed = id_of(&install.roster);
    let manifest_path = dir.join("nhl-studio-project.json");
    let mut manifest: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    manifest["working"]["roster_save"] = "working/saves/legacy/SYS-DATA".into();
    manifest["working"]["roster_container_dir"] = "legacy".into();
    manifest["destinations"]["roster_save"] = s(&install.roster).into();
    let mut component = manifest["source"]["components"][0].clone();
    let identity = id_of(&baseline);
    component["role"] = "roster_save".into();
    component["file_name"] = "SYS-DATA".into();
    component["baseline"] = "baseline/saves/legacy/SYS-DATA".into();
    component["len"] = identity.len.into();
    component["sha256"] = identity.sha256.into();
    manifest["source"]["components"].as_array_mut().unwrap().push(component);
    std::fs::write(&manifest_path, serde_json::to_vec_pretty(&manifest).unwrap()).unwrap();
    let opened = reg.project_open(&s(&dir), &defs()).unwrap();
    let id = opened.summary.project_id;
    assert!(!opened.summary.roster_attached);
    assert!(opened.warnings.iter().any(|w| w.contains("preserved")));
    assert!(opened.project.working_roster_save.is_none());
    // Save-only historical edits must not appear in a new mod package.
    let inventory = reg.package_inventory(id, Some(&st), &defs()).unwrap();
    assert_eq!(inventory.categories.iter().find(|c| c.category == "cells").unwrap().count, 0);
    edit(&reg, id, 2);
    reg.project_save(id).unwrap();
    let request = studio_api::dto::PackageBuildRequestDto { name: "DB Only".into(), version: "1".into(), description: "legacy separation".into(), include: vec!["cells".into()], exclude: vec![], platforms: vec!["ps3".into()], out_path: Some(s(&t.0.join("db-only.nhlmod"))) };
    let built = reg.package_build(id, &request, Some(&st), &defs()).unwrap();
    assert!(!built.path.is_empty());
    let archive = studio_domain::mod_package::ZipArchive::parse(std::fs::read(&built.path).unwrap(), &studio_domain::mod_package::Limits::default()).unwrap();
    let edits: serde_json::Value = serde_json::from_slice(&archive.read("edits.json").unwrap()).unwrap();
    let cells = edits["cells"].as_array().unwrap();
    assert!(!cells.is_empty());
    assert!(cells.iter().all(|cell| cell["file"] == "main_db"));
    let plan = reg.project_deploy_plan(id, false).unwrap();
    assert!(!plan.items.iter().any(|i| i.key.starts_with("roster")));
    reg.project_deploy_apply(id, plan.handle).unwrap();
    assert_eq!(before, id_of(&save));
    assert_eq!(installed, id_of(&install.roster));
    reg.close_project(id);
    let reopened = reg.project_open(&s(&dir), &defs()).unwrap();
    assert!(!reopened.summary.roster_attached);
    reg.close_project(reopened.summary.project_id);
    let retained: serde_json::Value = serde_json::from_slice(&std::fs::read(&manifest_path).unwrap()).unwrap();
    assert_eq!(retained["working"]["roster_save"], manifest["working"]["roster_save"]);
    assert_eq!(before, id_of(&save));
    // The unused historical save must not become a reopen dependency.
    let moved_save = save.with_extension("retained");
    std::fs::rename(&save, &moved_save).unwrap();
    let reopened = reg.project_open(&s(&dir), &defs()).unwrap();
    reg.close_project(reopened.summary.project_id);
    std::fs::rename(&moved_save, &save).unwrap();
    // A save-only legacy project requires the separate editor, without rewriting it.
    let mut save_only = retained;
    save_only["working"]["main_db"] = serde_json::Value::Null;
    let manifest_bytes = serde_json::to_vec_pretty(&save_only).unwrap();
    std::fs::write(&manifest_path, &manifest_bytes).unwrap();
    let error = reg.project_open(&s(&dir), &defs()).err().unwrap().to_string();
    assert!(error.contains("Save Editor"), "{error}");
    assert_eq!(std::fs::read(&manifest_path).unwrap(), manifest_bytes);
    assert_eq!(before, id_of(&save));
}

#[test]
fn extraction_preflight_and_skip_never_write_and_a_run_is_resumable() {
    let t = Tmp::new("extract");
    // An owned Xbox-shaped install: the database packed in a .big, plus a loose edited file.
    let install = t.0.join("xbox");
    std::fs::create_dir_all(&install).unwrap();
    std::fs::write(install.join("default.xex"), b"XEX2").unwrap();
    let mut b = studio_domain::tdb_archive_big::ArchiveBuilder::new();
    b.add_entry("db/nhlng.db", std::fs::read(repo("anchors/xbox360/nhl14/databases/nhlng.db")).unwrap());
    b.add_entry("fe/loc/nhl_eng_us.db", std::fs::read(repo("anchors/xbox360/nhl14/databases/nhl_eng_us.db")).unwrap());
    b.add_entry("rendering/edited.txt", b"SHIPPED".to_vec());
    std::fs::write(install.join("cacheboot.big"), b.build().unwrap()).unwrap();
    std::fs::create_dir_all(install.join("rendering")).unwrap();
    std::fs::write(install.join("rendering/edited.txt"), b"USER EDIT").unwrap();
    let before = tree_hashes(&install);
    let reg = Registry::default();
    let xbox = parse_platform("xbox").unwrap();
    let pre = reg.extract_preflight(&s(&install), xbox).unwrap();
    assert_eq!(pre.state, "partial", "{}", pre.reason);
    assert_eq!((pre.members_total, pre.members_present, pre.members_to_write), (3, 1, 2));
    assert_eq!(tree_hashes(&install), before, "preflight writes nothing");
    // Supported containers are extractable on PS3 too; the console label is not a gate.
    assert_eq!(reg.extract_preflight(&s(&install), parse_platform("ps3").unwrap()).unwrap().state, "partial");
    // Into a project folder: refused.
    let st = store();
    let project = reg.project_create_fresh(&s(&t.0.join("p")), "nhl14", xbox, &s(&install), None, None, &st, &defs()).unwrap();
    let err = reg.extract_preflight(&s(&t.0.join("p/working")), xbox).err().expect("refused").to_string();
    assert!(err.contains("inside the project folder"), "{err}");
    // Skip: recorded, nothing written.
    let info = reg.project_set_extraction(project.summary.project_id, "skipped").unwrap();
    assert_eq!(info.extraction_choice, "skipped");
    assert_eq!(tree_hashes(&install), before);
    // Run: the packed files appear, the user's edited file is left alone and reported.
    let job = reg.extract_start(&s(&install), xbox).unwrap();
    let err = reg.extract_start(&s(&install), xbox).err().map(|e| e.to_string());
    assert!(err.map_or(true, |e| e.contains("already running")), "a second job for the same destination is refused while the first runs");
    let mut status = job.clone();
    for _ in 0..600 {
        status = reg.extract_status(job.job).unwrap();
        if status.state != "running" && status.state != "cancelling" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!(status.state, "done", "{:?}", status.error);
    assert_eq!((status.files_written, status.files_conflicting), (2, 1));
    assert_eq!(std::fs::read(install.join("rendering/edited.txt")).unwrap(), b"USER EDIT");
    assert_eq!(id_of(&install.join("db/nhlng.db")), id_of(&repo("anchors/xbox360/nhl14/databases/nhlng.db")));
    assert!(status.problems.iter().any(|p| p.contains("edited.txt")));
    let pre = reg.extract_preflight(&s(&install), xbox).unwrap();
    assert_ne!(pre.state, "complete", "a conflict keeps the archive from counting as complete: {}", pre.reason);
    assert!(install.join("cacheboot.big.nhlms-extracted.json").is_file());
    // A rerun retains the equal files and reports the same conflict; nothing else changes.
    let after = tree_hashes(&install);
    let job2 = reg.extract_start(&s(&install), xbox).unwrap();
    let mut status2 = job2.clone();
    for _ in 0..600 {
        status2 = reg.extract_status(job2.job).unwrap();
        if status2.state != "running" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert_eq!((status2.files_written, status2.files_retained, status2.files_conflicting), (0, 2, 1));
    assert_eq!(tree_hashes(&install), after);
}
