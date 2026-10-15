//! D4 — the NHL 16–21 stored-default edit through the [`Registry`] seam: preview (with the install
//! root), apply as a pending member edit, the pending review listing it, the catalogue overlay, and
//! the ordinary copied-target review installing the rewritten members into the copied archive
//! (the `archive-csv` artifact staged by the route itself), then apply, reopen and restore. A
//! synthetic archive in the stager's qualified layout; nothing here is evidence a game accepts it.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};
use studio_domain::settings_catalogue::SettingCsvMembers;
use studio_domain::tdb_txn::{self, FileIdentity};
use tdb_archive_big::stage::synth::build_archive;
use tdb_archive_codec::compress_chunkzip_native;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-d4-api-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

const SETTING_18: &str = "id,name,is_global,is_choice,choice_to_range,name_loc,description_loc,extra_loc,default_value,potential_values,usage\r\n0,SKILL_LEVEL,0,1,0,SkillLevel,SkillLevel_Desc,,1,41,3\r\n72,DYNASTY_PLAYOFFS_LENGTH,0,1,0,PlayoffSeriesLength,PlayoffSeriesLength_Desc,,4,9,3\r\n";
const CONTEXT_18: &str = "id,name\r\n-2,_DEFAULT_\r\n3,DISPLAY_FRANCHISE_CREATION_CLASIC\r\n";
const CONTEXTSETTING_18: &str = "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage\r\n-2,0,1,1,41,1,3\r\n-2,72,1,4,9,1,3\r\n3,72,1,4,9,1,3\r\n";
const CONTEXTGROUP_18: &str = "context_id,group_id,order\r\n3,5,1\r\n";
const GROUP_18: &str = "id,name,parent_id,name_loc\r\n5,QUICK_DEFAULT_GM,,QuickSettings\r\n";
const GROUPSETTING_18: &str = "group_id,setting_id,order\r\n5,72,1\r\n";
const CHOICELIST_18: &str = "id,name\r\n9,DYNASTY_PLAYOFFS_LENGTH\r\n41,SKILL\r\n";
const CHOICE_18: &str = "id,choicelist_id,name,value,name_loc,description_loc\r\n0,9,SINGLE,0,SingleKnockout,\r\n1,9,B3,1,BestOf3,\r\n2,9,B5,2,BestOf5,\r\n3,9,B7,3,BestOf7,\r\n4,9,AUTH,4,Authentic,\r\n0,41,R,0,Rookie,\r\n1,41,P,1,Pro,\r\n";

fn build_root(tmp: &Tmp) -> (PathBuf, PathBuf, PathBuf) {
    let root = tmp.0.join("copy");
    tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db_dir = root.join("nhl/data/common/gamemodes/database/t3db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = db_dir.join("nhl_game.db");
    std::fs::copy(repo("anchors/ps4/nhl18/databases/nhl_game.db"), &db).unwrap();
    let texts = [
        ("setting.csv", SETTING_18),
        ("context.csv", CONTEXT_18),
        ("contextsetting.csv", CONTEXTSETTING_18),
        ("contextgroup.csv", CONTEXTGROUP_18),
        ("group.csv", GROUP_18),
        ("groupsetting.csv", GROUPSETTING_18),
        ("choicelist.csv", CHOICELIST_18),
        ("choice.csv", CHOICE_18),
    ];
    let mut files: Vec<(u16, &str, Vec<u8>)> = vec![(1, "first.bin", b"FIRST".to_vec())];
    for (n, t) in texts {
        files.push((0, n, compress_chunkzip_native(t.as_bytes())));
    }
    files.push((1, "last.bin", vec![0xAB; 64]));
    let bytes = build_archive(&["nhl/data/common/gamemodes/setting_csv", "nhl/data/other"], &files);
    std::fs::create_dir_all(root.join("databigs")).unwrap();
    let archive = root.join("databigs/inxp_00.big");
    std::fs::write(&archive, &bytes).unwrap();
    (root, db, archive)
}

fn open(reg: &Registry, db: &Path) -> u32 {
    reg.open_workspace_from_paths(
        Some("nhl18"),
        parse_platform("ps4").unwrap(),
        &s(db),
        None,
        &s(&repo("anchors/ps4/nhl18/databases/nhl_game-meta.xml")),
        &s(&repo("defs")),
    )
    .unwrap()
    .project_id
}

#[test]
fn csv_default_edit_flows_through_preview_apply_pending_review_and_the_copied_target_save() {
    let tmp = Tmp::new("flow");
    let (root, db, archive) = build_root(&tmp);
    let original_archive = std::fs::read(&archive).unwrap();
    let original_db = FileIdentity::of_file(&db).unwrap();
    let reg = Registry::default();
    let id = open(&reg, &db);
    let root_s = s(&root);

    // Catalogue through the route: the CSV source, the row, the value.
    let cat = reg.settings_catalogue(id, Some(&root_s)).unwrap();
    let c = cat.catalogue.as_ref().unwrap_or_else(|| panic!("{:?}", cat.blocker));
    assert_eq!(c.source.kind, "setting-csv");
    let row = c.settings.iter().find(|s| s.legacy_id == 72).unwrap();
    assert_eq!(row.context_defaults[0].stored.code, Some(4));
    // Without the install root: the refusal, not a plan.
    let p = reg.settings_default_preview(id, 72, Some(3), 3, None).unwrap();
    assert_eq!(p.refusal_kind.as_deref(), Some("no-install-root"));
    // Preview + apply the screen cell.
    let p = reg.settings_default_preview(id, 72, Some(3), 3, Some(&root_s)).unwrap();
    let plan = p.plan.unwrap_or_else(|| panic!("{:?}", p.refusal));
    assert_eq!((plan.kind.as_str(), plan.before, plan.after, plan.paired, plan.members.len()), ("csv", 4, 3, false, 1));
    assert_eq!(plan.members[0].cells, [(4u32, "default_value".to_string(), "4".to_string(), "3".to_string())]);
    assert!(plan.members[0].placement_prediction.as_deref().unwrap().contains("fit"));
    let applied = reg.settings_apply_review(id, &plan.ticket).unwrap();
    assert_eq!((applied.plan.kind.as_str(), applied.outcome.edits), ("csv", 1));
    assert!(applied.summary.dirty);
    // Consumed.
    assert!(reg.settings_apply_review(id, &plan.ticket).is_err());
    // The catalogue overlays the pending value; the pending review lists it.
    let cat = reg.settings_catalogue(id, Some(&root_s)).unwrap();
    assert_eq!(cat.catalogue.unwrap().settings.iter().find(|s| s.legacy_id == 72).unwrap().context_defaults[0].stored.code, Some(3));
    let pending = reg.pending_changes(id, 100).unwrap();
    assert!(pending.dirty);
    assert_eq!(pending.primary.total_cells, 0, "no database cell changed");
    let csv = pending.settings_csv.as_ref().expect("pending csv edits");
    assert_eq!(csv.len(), 1);
    assert_eq!((csv[0].setting_name.as_str(), csv[0].context, csv[0].before, csv[0].after, csv[0].paired), ("DYNASTY_PLAYOFFS_LENGTH", Some(3), 4, 3, false));
    assert_eq!(pending.settings_csv_archive.as_deref(), Some("databigs/inxp_00.big"));
    // Add the paired global edit on top.
    let p = reg.settings_default_preview(id, 72, None, 2, Some(&root_s)).unwrap();
    let plan2 = p.plan.unwrap();
    assert!(plan2.paired && plan2.members.len() == 2);
    reg.settings_apply_review(id, &plan2.ticket).unwrap();
    assert_eq!(reg.pending_changes(id, 100).unwrap().settings_csv.unwrap().len(), 2);
    // Undo the last one and re-apply it (the pending stack).
    assert!(reg.settings_csv_undo(id).unwrap());
    assert_eq!(reg.pending_changes(id, 100).unwrap().settings_csv.unwrap().len(), 1);
    let p = reg.settings_default_preview(id, 72, None, 2, Some(&root_s)).unwrap();
    reg.settings_apply_review(id, &p.plan.unwrap().ticket).unwrap();

    // The ordinary copied-target review stages the archive with the pending members itself.
    let review = reg.copied_target_review(id, &root_s).unwrap();
    let keys: Vec<&str> = review.artifacts.iter().map(|a| a.key.as_str()).collect();
    assert!(keys.contains(&"archive-csv"), "{keys:?}");
    assert!(keys.contains(&"primary"));
    let apply = reg.copied_target_apply(id, review.handle).unwrap();
    assert_eq!(apply.state, "committed", "{}", apply.state);
    // Installed: the members decode to the edited texts; the database is unchanged.
    let members = SettingCsvMembers::from_eb_archive(&archive).unwrap();
    let cs = String::from_utf8_lossy(&members.members["contextsetting.csv"].0).into_owned();
    assert!(cs.contains("-2,72,1,2,9,1,3\r\n") && cs.contains("3,72,1,3,9,1,3\r\n"), "{cs}");
    let st = String::from_utf8_lossy(&members.members["setting.csv"].0).into_owned();
    assert!(st.contains("PlayoffSeriesLength_Desc,,2,9,3\r\n"), "{st}");
    assert_eq!(members.members["choice.csv"].0, CHOICE_18.as_bytes());
    assert_eq!(FileIdentity::of_file(&db).unwrap(), original_db);
    // The session adopted the install: clean, no pending, the catalogue reads the archive.
    let summary = reg.summary(id).unwrap();
    assert!(!summary.dirty, "the session should be clean after the install");
    assert!(reg.pending_changes(id, 100).unwrap().settings_csv.is_none());
    let cat = reg.settings_catalogue(id, Some(&root_s)).unwrap().catalogue.unwrap();
    let row = cat.settings.iter().find(|s| s.legacy_id == 72).unwrap();
    assert_eq!((row.global.as_ref().unwrap().code, row.global.as_ref().unwrap().consistent, row.context_defaults[0].stored.code), (Some(2), Some(true), Some(3)));
    // A further edit on the installed copy is eligible.
    let p = reg.settings_default_preview(id, 72, Some(3), 1, Some(&root_s)).unwrap();
    assert_eq!(p.plan.unwrap().before, 3);
    // Restore through the route: the original archive returns byte-exact.
    let restored = reg.copied_target_restore(&root_s, &apply.txn_id).unwrap();
    assert_eq!(restored.state, "restored", "{}", restored.state);
    assert_eq!(std::fs::read(&archive).unwrap(), original_archive);
}

#[test]
fn a_copied_root_without_the_archive_refuses_the_review_and_keeps_the_pending_edit() {
    let tmp = Tmp::new("noarchive");
    let (root, db, archive) = build_root(&tmp);
    let reg = Registry::default();
    let id = open(&reg, &db);
    let root_s = s(&root);
    let p = reg.settings_default_preview(id, 72, Some(3), 3, Some(&root_s)).unwrap();
    reg.settings_apply_review(id, &p.plan.unwrap().ticket).unwrap();
    // The copied root's archive is gone: the review refuses before anything is prepared.
    std::fs::remove_file(&archive).unwrap();
    let e = match reg.copied_target_review(id, &root_s) {
        Ok(_) => panic!("expected a refusal"),
        Err(e) => e.to_string(),
    };
    assert!(e.contains("has no databigs/inxp_00.big"), "{e}");
    assert_eq!(reg.pending_changes(id, 100).unwrap().settings_csv.unwrap().len(), 1);
    assert!(reg.summary(id).unwrap().dirty);
}
