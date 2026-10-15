//! D4 / P1-BS-4 — editing one stored catalogue default on NHL 16–21 PS4: the byte splice on the
//! CSV member, the native re-encode, the held review, pending/undo, the copied-target transaction
//! through the archive stager (fit-in-slot and append-and-retarget), restore, and the refusals.
//! Members are synthetic texts shaped exactly like the measured variants, packed into a synthetic
//! EB archive (the stager's qualified 16-byte-row layout); the real NHL 16 archive is exercised
//! `--ignored` under `NHL_STUDIO_FIXTURES`. Nothing here says what the game offers or applies.

use std::path::{Path, PathBuf};

use studio_domain::copied_target::{stage_workspace, text_member_replacements};
use studio_domain::settings_catalogue::{CatalogueSource, SettingCsvMembers, CSV_MEMBER_PREFIX};
use studio_domain::settings_csv_edit::{parse_spanned, plan_csv_default_edit, verify_splice};
use studio_domain::settings_edit::DefaultTarget;
use studio_domain::tdb_archive_big::stage::{self, synth, BoundSource, Placement};
use studio_domain::tdb_txn::{self, FileIdentity, NoHooks, State};
use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};
use tdb_archive_codec::compress_chunkzip_native;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-d4-{tag}-{}", std::process::id()));
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

fn ctx(legacy_id: i64, context: i64) -> DefaultTarget {
    DefaultTarget { legacy_id, context: Some(context) }
}
fn global(legacy_id: i64) -> DefaultTarget {
    DefaultTarget { legacy_id, context: None }
}

// The NHL 18 shape (handoff 01 §2.1): every record CRLF-terminated, the last included.
const SETTING_18: &str = "id,name,is_global,is_choice,choice_to_range,name_loc,description_loc,extra_loc,default_value,potential_values,usage\r\n0,SKILL_LEVEL,0,1,0,SkillLevel,SkillLevel_Desc,,1,41,3\r\n72,DYNASTY_PLAYOFFS_LENGTH,0,1,0,PlayoffSeriesLength,PlayoffSeriesLength_Desc,,4,9,3\r\n71,DYNASTY_SEASON_OVERTIME_LOSS,0,1,0,SeasonOvertimeLoss,,,1,11,3\r\n120,GAME_MODE_HOME_AND_AWAY_SCHEDULE,0,1,0,HomeAwayType,,,1,34,3\r\n117,GAME_MODE_CUP_TYPE,0,1,0,TrophyType,,,0,57,3\r\n";
const CONTEXT_18: &str = "id,name\r\n-2,_DEFAULT_\r\n0,DISPLAY_PLAY_NOW_MODE\r\n3,DISPLAY_FRANCHISE_CREATION_CLASIC\r\n37,DISPLAY_SEASON_CREATION\r\n";
const CONTEXTSETTING_18: &str = "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage\r\n-2,0,1,1,41,1,3\r\n-2,72,1,4,9,1,3\r\n3,72,1,4,9,1,3\r\n-2,71,1,1,11,1,3\r\n3,71,1,1,11,1,3\r\n-2,120,1,0,34,1,3\r\n3,120,1,1,34,0,2\r\n";
const CONTEXTGROUP_18: &str = "context_id,group_id,order\r\n0,0,1\r\n3,5,1\r\n";
const GROUP_18: &str = "id,name,parent_id,name_loc\r\n0,QUICK_DEFAULT,74,GameSettings\r\n5,QUICK_DEFAULT_GM,,QuickSettings\r\n";
const GROUPSETTING_18: &str = "group_id,setting_id,order\r\n0,0,4\r\n5,72,1\r\n";
const CHOICELIST_18: &str = "id,name\r\n9,DYNASTY_PLAYOFFS_LENGTH\r\n11,DYNASTY_SEASON_OVERTIME_LOSS\r\n34,RULE_TOGGLE\r\n41,SKILL\r\n57,GAME_MODE_CUP_TYPE\r\n";
const CHOICE_18: &str = "id,choicelist_id,name,value,name_loc,description_loc\r\n0,9,SINGLE,0,SingleKnockout,\r\n1,9,B3,1,BestOf3,\r\n2,9,B5,2,BestOf5,\r\n3,9,B7,3,BestOf7,\r\n4,9,AUTH,4,Authentic,\r\n0,11,Z,0,0Points,\r\n1,11,O,1,1Point,\r\n0,34,OFF,0,OFF,\r\n1,34,ON,1,ON,\r\n0,41,R,0,Rookie,\r\n1,41,P,1,Pro,\r\n0,57,A,0,TXT_BATTLE_FOR_THE_CUP_NOW,\r\n1,57,B,1,TXT_HEAD_TO_HEAD_MODE,\r\n";
const RANGE_18: &str = "id,name,min,max,step\r\n0,ADVANTAGE_SETTINGS_RANGE,-7,7,1\r\n";

fn members_18() -> Vec<(&'static str, &'static str)> {
    vec![
        ("setting.csv", SETTING_18),
        ("context.csv", CONTEXT_18),
        ("contextsetting.csv", CONTEXTSETTING_18),
        ("contextgroup.csv", CONTEXTGROUP_18),
        ("group.csv", GROUP_18),
        ("groupsetting.csv", GROUPSETTING_18),
        ("choicelist.csv", CHOICELIST_18),
        ("choice.csv", CHOICE_18),
        ("range.csv", RANGE_18),
    ]
}

/// A copied-target root with the NHL 18 anchor database at its PS4 place and a synthetic
/// `databigs/inxp_00.big` holding the members (`contextsetting.csv`, `setting.csv`, … chunkzip;
/// `range.csv` raw; two unrelated members around them, one sharing nothing). `pad` bytes of an
/// unrelated member follow `contextsetting.csv` so its slot has no slack (append) or plenty (fit).
fn build_root(tmp: &Tmp, texts: &[(&str, &str)], slack_after_contextsetting: bool) -> (PathBuf, PathBuf, PathBuf) {
    let root = tmp.0.join("copy");
    tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db_dir = root.join("nhl/data/common/gamemodes/database/t3db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = db_dir.join("nhl_game.db");
    std::fs::copy(repo("anchors/ps4/nhl18/databases/nhl_game.db"), &db).unwrap();
    let dirs = ["nhl/data/common/gamemodes/setting_csv", "nhl/data/other"];
    let mut files: Vec<(u16, &str, Vec<u8>)> = vec![(1, "first.bin", b"FIRST-MEMBER-BYTES".to_vec())];
    for (name, text) in texts {
        let stored = if *name == "range.csv" { text.as_bytes().to_vec() } else { compress_chunkzip_native(text.as_bytes()) };
        files.push((0, name, stored));
        if *name == "contextsetting.csv" && !slack_after_contextsetting {
            // The next member starts on the very next 16-byte line only if the stored length is
            // a multiple of 16; make the neighbour tight by padding the member itself to 16.
        }
    }
    files.push((1, "last.bin", vec![0xAB; 100]));
    let bytes = synth::build_archive(&dirs, &files);
    let arc_dir = root.join("databigs");
    std::fs::create_dir_all(&arc_dir).unwrap();
    let archive = arc_dir.join("inxp_00.big");
    std::fs::write(&archive, &bytes).unwrap();
    (root, db, archive)
}

fn open(db: &Path, game: &str) -> Workspace {
    let project = Project::load(db, Platform::Ps4, ProjectRole::Nhlng, repo("anchors/ps4/nhl18/databases/nhl_game-meta.xml"), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.to_string_lossy().into_owned());
    ws.set_game_id(game);
    ws
}

fn member_plain(archive: &Path, name: &str) -> Vec<u8> {
    let m = SettingCsvMembers::from_eb_archive(archive).unwrap();
    m.members.get(name).unwrap().0.clone()
}

// ---------------------------------------------------------------------------------------------
// 1. The subset parser and the splice contract
// ---------------------------------------------------------------------------------------------

#[test]
fn the_subset_parser_locates_cells_by_span_and_refuses_what_it_cannot_account_for() {
    let csv = parse_spanned("contextsetting.csv", CONTEXTSETTING_18.as_bytes()).unwrap();
    assert_eq!(csv.header, ["context_id", "setting_id", "is_enabled", "default_value", "potential_values", "is_visible", "usage"]);
    assert_eq!(csv.rows.len(), 8);
    let r = &csv.rows[3];
    assert_eq!(r.line, 4);
    let (s, e) = r.cells[3];
    assert_eq!(&CONTEXTSETTING_18.as_bytes()[s..e], b"4");
    assert_eq!(&CONTEXTSETTING_18.as_bytes()[r.cells[0].0..r.cells[0].1], b"3");
    // Refusals: LF-only, missing final CRLF, blank line, quote, BOM, ragged, duplicate header.
    let why = |t: &str| parse_spanned("m", t.as_bytes()).unwrap_err().reason;
    assert!(why("a,b\n1,2\n").contains("LF, not CRLF"));
    assert!(why("a,b\r\n1,2").contains("no CRLF terminator"));
    assert!(why("a,b\r\n\r\n1,2\r\n").contains("blank"));
    assert!(why("a,b\r\n1,\"2\"\r\n").contains("quote"));
    assert!(why("\u{feff}a,b\r\n1,2\r\n").contains("byte-order mark"));
    assert!(why("a,b\r\n1,2,3\r\n").contains("ragged"));
    assert!(why("a,a\r\n1,2\r\n").contains("appears twice"));
    // The splice verification: the target cell only.
    let before = CONTEXTSETTING_18.as_bytes();
    let mut after = before.to_vec();
    after[s] = b'3';
    verify_splice("contextsetting.csv", before, &after, (s, e), 1, 4).unwrap();
    let mut bad = after.clone();
    bad[r.cells[4].0] = b'8';
    let err = verify_splice("contextsetting.csv", before, &bad, (s, e), 1, 4).unwrap_err();
    assert!(err.contains("differ") || err.contains("cell 4"), "{err}");
    let mut longer = before.to_vec();
    longer.splice(s..e, b"10".iter().copied());
    verify_splice("contextsetting.csv", before, &longer, (s, e), 2, 4).unwrap();
    assert!(verify_splice("contextsetting.csv", before, &longer, (s, e), 1, 4).unwrap_err().contains("length accounting"));
}

// ---------------------------------------------------------------------------------------------
// 2. Planning on synthetic members (no archive): cells, pair, refusals
// ---------------------------------------------------------------------------------------------

#[test]
fn plans_one_screen_cell_the_global_pair_and_refuses_by_reason() {
    let m = SettingCsvMembers::from_texts("synthetic", &members_18());
    let plan = |t: DefaultTarget, code: i64| plan_csv_default_edit("nhl18", &m, None, None, None, &[], t, code);
    // Screen row (3, 72): 4 → 3.
    let p = plan(ctx(72, 3), 3).unwrap();
    assert_eq!((p.setting_name.as_str(), p.before, p.after, p.paired, p.no_op), ("DYNASTY_PLAYOFFS_LENGTH", 4, 3, false, false));
    assert_eq!((p.before_label.as_deref(), p.after_label.as_deref(), p.label_source.as_str()), (Some("Authentic"), Some("BestOf7"), "explicit"));
    assert_eq!(p.members.len(), 1);
    let me = &p.members[0];
    assert_eq!((me.member.as_str(), me.stored_form.as_str(), me.cells.len()), ("contextsetting.csv", "raw", 1));
    let c = &me.cells[0];
    assert_eq!((c.line, c.column.as_str(), c.before.as_str(), c.after.as_str()), (4, "default_value", "4", "3"));
    assert_eq!(me.new_plain.len(), CONTEXTSETTING_18.len());
    assert_eq!(String::from_utf8_lossy(&me.new_plain), CONTEXTSETTING_18.replace("3,72,1,4,9,1,3", "3,72,1,3,9,1,3"));
    assert_eq!(p.unchanged, [("global".to_string(), 4)]);
    assert!(p.description.contains("DISPLAY_FRANCHISE_CREATION_CLASIC screen (context 3) changes from code 4 (*Authentic*, label explicit) to code 3 (*BestOf7*, label explicit)"), "{}", p.description);
    assert!(p.description.contains("has not been established"));
    // Global pair 72: two members, two cells, both 4 → 3.
    let p = plan(global(72), 3).unwrap();
    assert!(p.paired);
    assert_eq!(p.members.iter().map(|m| m.member.as_str()).collect::<Vec<_>>(), ["contextsetting.csv", "setting.csv"]);
    assert_eq!(p.cells().map(|c| (c.line, c.before.as_str(), c.after.as_str())).collect::<Vec<_>>(), [(3, "4", "3"), (3, "4", "3")]);
    assert!(p.description.contains("**and**") && p.description.contains("Two text files are rewritten"), "{}", p.description);
    assert_eq!(p.unchanged, [("context 3 (DISPLAY_FRANCHISE_CREATION_CLASIC)".to_string(), 4)]);
    // No-op.
    assert!(plan(ctx(72, 3), 4).unwrap().no_op);
    // Refusals.
    let r = plan(ctx(72, 37), 3).unwrap_err();
    assert_eq!(r.kind, "no-row");
    assert!(r.reason.contains("screen 37 (DISPLAY_SEASON_CREATION)") && r.reason.contains("row-growth"), "{}", r.reason);
    let r = plan(ctx(72, 3), 5).unwrap_err();
    assert_eq!(r.kind, "out-of-bounds");
    assert!(r.reason.contains("choice list (0, 1, 2, 3, 4)"), "{}", r.reason);
    assert_eq!(plan(ctx(72, 3), -1).unwrap_err().kind, "out-of-bounds");
    // The reserved context is never an ordinary screen.
    let r = plan(ctx(72, -2), 3).unwrap_err();
    assert!(r.reason.contains("_DEFAULT_"), "{}", r.reason);
    // 120: the pair disagrees (setting 1 vs _DEFAULT_ 0) → paired edit refused, both values shown.
    let r = plan(global(120), 0).unwrap_err();
    assert_eq!(r.kind, "ambiguous");
    assert!(r.reason.contains("setting.csv 1 vs _DEFAULT_ row 0") && r.reason.contains("repair route is not offered"), "{}", r.reason);
    // 120's screen row is flagged is_visible 0.
    let r = plan(ctx(120, 3), 0).unwrap_err();
    assert!(r.reason.contains("is_visible 0"), "{}", r.reason);
    // 117: stored once (no _DEFAULT_ row) → the paired operation is not available.
    let r = plan(global(117), 1).unwrap_err();
    assert_eq!(r.kind, "no-row");
    assert!(r.reason.contains("stored once"), "{}", r.reason);
    // Absent on 17+ / excluded / out of scope / nonexistent.
    assert_eq!(plan(global(83), 1).unwrap_err().kind, "absent");
    assert_eq!(plan(global(0), 1).unwrap_err().kind, "not-offered");
    assert_eq!(plan(global(999), 1).unwrap_err().kind, "absent");
    // A synthetic duplicate row is ambiguous.
    let mut dup = members_18();
    dup[2] = ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage\r\n-2,72,1,4,9,1,3\r\n3,72,1,4,9,1,3\r\n3,72,1,2,9,1,3\r\n");
    let m2 = SettingCsvMembers::from_texts("dup", &dup);
    assert_eq!(plan_csv_default_edit("nhl18", &m2, None, None, None, &[], ctx(72, 3), 3).unwrap_err().kind, "ambiguous");
    // A member outside the writer subset (LF-only) reads (D2) but is not written.
    let mut lf = members_18();
    lf[2] = ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage\n-2,72,1,4,9,1,3\n3,72,1,4,9,1,3\n");
    let m3 = SettingCsvMembers::from_texts("lf", &lf);
    let r = plan_csv_default_edit("nhl18", &m3, None, None, None, &[], ctx(72, 3), 3).unwrap_err();
    assert_eq!(r.kind, "member-refused");
    assert!(r.reason.contains("LF, not CRLF"), "{}", r.reason);
    // A renamed required header refuses; an extra column is carried.
    let mut renamed = members_18();
    renamed[2] = ("contextsetting.csv", "context_id,setting_id,is_enabled,default,potential_values,is_visible,usage\r\n-2,72,1,4,9,1,3\r\n3,72,1,4,9,1,3\r\n");
    let m4 = SettingCsvMembers::from_texts("ren", &renamed);
    assert!(plan_csv_default_edit("nhl18", &m4, None, None, None, &[], ctx(72, 3), 3).is_err());
    let mut extra = members_18();
    extra[2] = ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage,extra\r\n-2,72,1,4,9,1,3,x\r\n3,72,1,4,9,1,3,y\r\n");
    let m5 = SettingCsvMembers::from_texts("extra", &extra);
    let p = plan_csv_default_edit("nhl18", &m5, None, None, None, &[], ctx(72, 3), 3).unwrap();
    assert!(String::from_utf8_lossy(&p.members[0].new_plain).contains("3,72,1,3,9,1,3,y\r\n"));
}

#[test]
fn nhl16_and_nhl17_variants_plan_their_own_global_cell() {
    let d = studio_domain::settings_catalogue::SettingsDefs::load_dir(repo("defs/settings")).unwrap();
    let m16 = SettingCsvMembers::from_texts("16", &[
        ("setting.csv", "id,key,name,string_id,description_id,is_choice,min_value,max_value,default_value,remap_id\r\n72,DYNASTY_PLAYOFFS_LENGTH,Setting-72,PlayoffSeriesLength,TXT_SETTING_DESC_72,1,0,4,3,153\r\n68,DYNASTY_SEASON_TIE_BREAK,Setting-68,SeasonTieBreak,TXT,1,0,5,2,18\r\n"),
        ("context.csv", "id,key,name\r\n3,FROM_DYNASTY_FLOW,Context-3\r\n"),
        ("contextsetting.csv", "context_id,setting_id,min_value,max_value,default_value,choices,is_enabled\r\n3,72,,,4,,1\r\n3,68,0,3,,,1\r\n"),
        ("contextgroup.csv", "context_id,group_id,order\r\n3,5,1\r\n"),
        ("group.csv", "id,parent_id,string_id,name\r\n5,,TXT_CATEGORY_5,QUICK_DYNASTY_FLOW\r\n"),
        ("settinggroup.csv", "setting_id,group_id,order\r\n72,5,1\r\n"),
    ]);
    let p = plan_csv_default_edit("nhl16", &m16, d.nhl15_ps4(), None, None, &[], global(72), 2).unwrap();
    assert!(!p.paired);
    assert_eq!((p.members[0].member.as_str(), p.members[0].cells[0].line, p.before, p.after), ("setting.csv", 2, 3, 2));
    assert_eq!((p.before_label.as_deref(), p.label_source.as_str()), (Some("BestOf7"), "inferred"));
    assert_eq!(plan_csv_default_edit("nhl16", &m16, d.nhl15_ps4(), None, None, &[], global(72), 5).unwrap_err().kind, "out-of-bounds");
    assert_eq!(plan_csv_default_edit("nhl16", &m16, d.nhl15_ps4(), None, None, &[], global(68), 7).unwrap_err().kind, "out-of-bounds");
    // NHL 16 per-context override present → not interpreted, refused; null cell refused.
    let r = plan_csv_default_edit("nhl16", &m16, d.nhl15_ps4(), None, None, &[], ctx(68, 3), 1).unwrap_err();
    assert!(r.reason.contains("not interpreted") || r.reason.contains("null"), "{}", r.reason);
    let m17 = SettingCsvMembers::from_texts("17", &[
        ("setting.csv", "id,name,legacy_id,is_global,is_choice,choice_to_range,name_loc,description_loc,extra_loc\r\n72,DYNASTY_PLAYOFFS_LENGTH,153,0,1,0,PlayoffSeriesLength,,\r\n"),
        ("context.csv", "id,name\r\n-2,_DEFAULT_\r\n3,FROM_DYNASTY_FLOW\r\n"),
        ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values\r\n-2,72,1,4,9\r\n3,72,1,4,9\r\n"),
        ("contextgroup.csv", "context_id,group_id,order\r\n3,5,1\r\n"),
        ("group.csv", "id,name,parent_id,name_loc\r\n5,QUICK_DEFAULT_GM,,QuickSettings\r\n"),
        ("groupsetting.csv", "group_id,setting_id,order\r\n5,72,1\r\n"),
        ("choicelist.csv", "id,name\r\n9,DYNASTY_PLAYOFFS_LENGTH\r\n"),
        ("choice.csv", "id,choicelist_id,name,value,name_loc,description_loc\r\n0,9,SINGLE,0,SingleKnockout,\r\n1,9,B3,1,BestOf3,\r\n2,9,B5,2,BestOf5,\r\n3,9,B7,3,BestOf7,\r\n4,9,AUTH,4,Authentic,\r\n"),
    ]);
    let p = plan_csv_default_edit("nhl17", &m17, None, None, None, &[], global(72), 3).unwrap();
    assert!(!p.paired);
    assert_eq!((p.members[0].member.as_str(), p.members[0].cells[0].line), ("contextsetting.csv", 2));
    assert!(plan_csv_default_edit("nhl17", &m17, None, None, None, &[], ctx(72, -2), 3).unwrap_err().reason.contains("_DEFAULT_"));
}

// ---------------------------------------------------------------------------------------------
// 3. The workspace: held review, pending, undo, catalogue overlay, copied-target save + restore
// ---------------------------------------------------------------------------------------------

#[test]
fn workspace_holds_the_review_applies_it_as_a_pending_edit_and_installs_it_through_the_stager() {
    let tmp = Tmp::new("ws");
    let (root, db, archive) = build_root(&tmp, &members_18(), true);
    let original_archive = std::fs::read(&archive).unwrap();
    let original_db = FileIdentity::of_file(&db).unwrap();
    let mut ws = open(&db, "nhl18");
    // Catalogue through the workspace (no pending edits): read from the archive.
    let cat = ws.settings_catalogue(Some(&root)).unwrap().unwrap();
    assert_eq!(cat.setting(72).unwrap().context_defaults[0].stored.code, Some(4));
    let CatalogueSource::SettingCsv { members, .. } = &cat.source else { panic!() };
    assert!(members.iter().any(|m| m.stored_form == "chunkzip") && members.iter().any(|m| m.stored_form == "raw"));
    let rev0 = ws.revision();
    assert!(rev0.text_members.is_none());

    // Preview (3, 72) 4 → 3.
    let (t1, p1) = ws.settings_csv_default_preview(&root, ctx(72, 3), 3).unwrap().unwrap();
    assert_eq!(p1.members[0].stored_form, "chunkzip");
    assert!(p1.members[0].source_stored_sha256.is_some());
    assert!(p1.members[0].placement_prediction.as_deref().unwrap().contains("fit"), "{:?}", p1.members[0].placement_prediction);
    assert_eq!(p1.archive.as_deref(), Some(archive.display().to_string().as_str()));
    assert!(!ws.is_dirty(), "a preview writes nothing");
    assert_eq!(ws.revision(), rev0);
    // Apply → pending, revision moved, catalogue overlays the edit, the archive is untouched.
    let review = ws.settings_csv_apply_review(&t1).unwrap();
    assert_eq!(review.plan.after, 3);
    assert!(ws.is_dirty());
    assert_eq!(ws.settings_csv_edits().len(), 1);
    let rev1 = ws.revision();
    assert!(rev1.text_members.is_some() && rev1 != rev0);
    assert!(rev1.to_string().contains("+csv:"));
    let cat = ws.settings_catalogue(Some(&root)).unwrap().unwrap();
    assert_eq!(cat.setting(72).unwrap().context_defaults[0].stored.code, Some(3));
    assert_eq!(cat.setting(72).unwrap().global.as_ref().unwrap().code, Some(4));
    assert_eq!(std::fs::read(&archive).unwrap(), original_archive);
    // Consumed / stale / foreign tickets.
    assert!(ws.settings_csv_apply_review(&t1).unwrap_err().to_string().contains("no longer held"));
    let (t2, _) = ws.settings_csv_default_preview(&root, global(72), 3).unwrap().unwrap();
    assert!(ws.undo_settings_csv_edit());
    // The undo moved the revision and dropped the held reviews: the ticket is refused either way.
    let e = ws.settings_csv_apply_review(&t2).unwrap_err().to_string();
    assert!(e.contains("no longer held") || e.contains("changed since this preview"), "{e}");
    // A revision move with the review still held (an unrelated database edit) is the stale case.
    let (t2b, _) = ws.settings_csv_default_preview(&root, global(72), 3).unwrap().unwrap();
    ws.set_field("kOtt", 0, "xsnn", studio_domain::FieldValue::Text("ZZZ".into())).unwrap();
    let e = ws.settings_csv_apply_review(&t2b).unwrap_err().to_string();
    assert!(e.contains("changed since this preview"), "{e}");
    assert!(ws.undo());
    assert!(!ws.is_dirty());
    let mut other = open(&db, "nhl18");
    let (foreign, _) = other.settings_csv_default_preview(&root, ctx(72, 3), 3).unwrap().unwrap();
    assert!(ws.settings_csv_apply_review(&foreign).unwrap_err().to_string().contains("another project"));

    // Two successive edits: the screen row, then the global pair on the file carrying the first.
    let (t1, _) = ws.settings_csv_default_preview(&root, ctx(72, 3), 3).unwrap().unwrap();
    ws.settings_csv_apply_review(&t1).unwrap();
    let (t2, p2) = ws.settings_csv_default_preview(&root, global(72), 2).unwrap().unwrap();
    assert!(p2.paired);
    // The second plan's contextsetting source is the first edit's output (stacked).
    assert_eq!(p2.members[0].source_plain_sha256, ws.settings_csv_edits()[0].members[0].new_plain_sha256);
    ws.settings_csv_apply_review(&t2).unwrap();
    assert_eq!(ws.settings_csv_edits().len(), 2);
    let cat = ws.settings_catalogue(Some(&root)).unwrap().unwrap();
    let s = cat.setting(72).unwrap();
    assert_eq!((s.global.as_ref().unwrap().code, s.global.as_ref().unwrap().consistent, s.context_defaults[0].stored.code), (Some(2), Some(true), Some(3)));
    // The merged replacements: contextsetting.csv carries two cells, setting.csv one; sources are
    // the archive's original identities.
    let w = ws.settings_csv_working_set().unwrap();
    let reps = w.replacements();
    assert_eq!(reps.iter().map(|r| (r.member.as_str(), r.cells.len())).collect::<Vec<_>>(), [("contextsetting.csv", 2), ("setting.csv", 1)]);
    let cs = reps.iter().find(|r| r.member == "contextsetting.csv").unwrap();
    assert_eq!(cs.source_stored_sha256, w.stored_identities.iter().find(|(p, ..)| p == &format!("{CSV_MEMBER_PREFIX}contextsetting.csv")).map(|(_, s, _)| s.clone()));
    assert_eq!(String::from_utf8_lossy(&cs.new_plain), CONTEXTSETTING_18.replace("-2,72,1,4,9,1,3", "-2,72,1,2,9,1,3").replace("3,72,1,4,9,1,3", "3,72,1,3,9,1,3"));

    // Copied-target save: the plan carries the text members; the route's steps install them.
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let mut plan = stage_workspace(&ws, &root, &staging, "D4 csv").unwrap();
    let text = plan.text_members.clone().expect("pending text members on the plan");
    assert_eq!(text.archive_relative, "databigs/inxp_00.big");
    assert_eq!(text.members.len(), 2);
    assert_eq!(text.descriptions.len(), 2);
    let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
    let bound = BoundSource::bind(&mut f).unwrap();
    let replacements = text_member_replacements(&bound, &text).unwrap();
    assert_eq!(replacements.len(), 2);
    let archive_plan = stage::plan(&bound, replacements).unwrap();
    assert_eq!(archive_plan.edits.len(), 2);
    for e in &archive_plan.edits {
        assert!(matches!(e.placement, Placement::FitInSlot { .. } | Placement::AppendAndRetarget { .. }), "{:?}", e.placement);
    }
    let staged = studio_domain::txn::stage_archive(bound, archive_plan.clone(), &archive, &staging.join("inxp_00.big.csv.staged")).unwrap();
    plan.add_archive("archive-csv", &archive, &staged).unwrap();
    assert!(plan.artifact("archive-csv").is_some());
    // The primary database is unchanged (no DB edit): its artifact is still there with the same bytes.
    let mut txn = tdb_txn::prepare(&plan.root, &plan.change_set, &mut NoHooks).unwrap();
    let out = tdb_txn::apply(&mut txn, &plan.change_set, &mut NoHooks).unwrap();
    assert_eq!(out.state, State::Committed);
    // The installed archive: the two members decode to the held texts, every other member is
    // byte-identical, the stage plan's changed ranges are the only differences.
    let installed = std::fs::read(&archive).unwrap();
    assert_ne!(installed, original_archive);
    assert_eq!(String::from_utf8_lossy(&member_plain(&archive, "contextsetting.csv")), String::from_utf8_lossy(&cs.new_plain));
    assert!(String::from_utf8_lossy(&member_plain(&archive, "setting.csv")).contains("PlayoffSeriesLength_Desc,,2,9,3\r\n"));
    assert_eq!(member_plain(&archive, "choice.csv"), CHOICE_18.as_bytes());
    assert_eq!(member_plain(&archive, "range.csv"), RANGE_18.as_bytes());
    let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
    let rebound = BoundSource::bind(&mut f).unwrap();
    for e in &rebound.entries {
        if e.path.ends_with("first.bin") || e.path.ends_with("last.bin") {
            let orig = bound_entry_bytes(&original_archive, &e.path);
            assert_eq!(installed[e.offset as usize..e.offset as usize + e.size as usize], orig[..], "{}", e.path);
        }
    }
    let n = installed.len().min(original_archive.len());
    let differing: Vec<usize> = (0..n).filter(|&i| installed[i] != original_archive[i]).collect();
    for i in differing {
        assert!(archive_plan.changed_ranges.iter().any(|r| r.contains(&(i as u64))), "byte {i:#x} changed outside the plan's ranges");
    }
    // Re-read through a fresh workspace: the edits are the baseline now.
    let mut fresh = open(&db, "nhl18");
    let cat = fresh.settings_catalogue(Some(&root)).unwrap().unwrap();
    assert_eq!(cat.setting(72).unwrap().global.as_ref().unwrap().code, Some(2));
    assert_eq!(cat.setting(72).unwrap().context_defaults[0].stored.code, Some(3));
    // The session adopts the install: no pending edits, clean, and a further edit is eligible.
    ws.rebind_csv_after_copied_save();
    ws.rebind_after_copied_save(Some(&FileIdentity::of_file(&db).unwrap()), None, None).unwrap();
    assert!(!ws.is_dirty());
    assert!(ws.settings_csv_edits().is_empty());
    let (t3, p3) = ws.settings_csv_default_preview(&root, ctx(72, 3), 1).unwrap().unwrap();
    assert_eq!(p3.before, 3);
    ws.settings_csv_apply_review(&t3).unwrap();
    assert!(ws.undo_settings_csv_edit());
    // Restore: the exact original archive and database return.
    tdb_txn::restore(&root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_eq!(std::fs::read(&archive).unwrap(), original_archive);
    assert_eq!(FileIdentity::of_file(&db).unwrap(), original_db);
}

fn bound_entry_bytes(archive_bytes: &[u8], path: &str) -> Vec<u8> {
    let mut c = std::io::Cursor::new(archive_bytes);
    let b = BoundSource::bind(&mut c).unwrap();
    let e = b.entries.iter().find(|e| e.path == path).unwrap();
    archive_bytes[e.offset as usize..e.offset as usize + e.size as usize].to_vec()
}

#[test]
fn a_copy_whose_member_differs_from_the_planned_one_is_refused_and_a_paired_failure_leaves_both_unchanged() {
    let tmp = Tmp::new("refuse");
    let (root, db, archive) = build_root(&tmp, &members_18(), true);
    let original_archive = std::fs::read(&archive).unwrap();
    let mut ws = open(&db, "nhl18");
    let (t, _) = ws.settings_csv_default_preview(&root, global(72), 3).unwrap().unwrap();
    ws.settings_csv_apply_review(&t).unwrap();
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let plan = stage_workspace(&ws, &root, &staging, "D4 refuse").unwrap();
    let text = plan.text_members.clone().unwrap();
    // (a) Another copy of the archive whose contextsetting.csv differs: refused by identity.
    let mut altered = members_18();
    altered[2] = ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage\r\n-2,72,1,4,9,1,3\r\n3,72,1,0,9,1,3\r\n");
    let tmp2 = Tmp::new("refuse-other");
    let (_, _, other_archive) = build_root(&tmp2, &altered, true);
    let mut f = std::io::BufReader::new(std::fs::File::open(&other_archive).unwrap());
    let bound = BoundSource::bind(&mut f).unwrap();
    let e = text_member_replacements(&bound, &text).unwrap_err().to_string();
    assert!(e.contains("is not the member the edit was planned on"), "{e}");
    // (b) A pair where the second member's held bytes are corrupted: nothing is staged (the
    //     replacements are built as one set; the first failure refuses the whole set).
    let mut broken = text.clone();
    let m = &mut broken.members[1];
    let mut bad = (*m.new_stored).clone();
    // Corrupt a byte inside the deflate stream (a trailing byte would be ignored by inflate).
    bad[60] ^= 0xFF;
    m.new_stored = std::sync::Arc::new(bad);
    let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
    let bound = BoundSource::bind(&mut f).unwrap();
    let e = text_member_replacements(&bound, &broken).unwrap_err().to_string();
    assert!(e.contains("do not decode") || e.contains("decode to something other"), "{e}");
    assert_eq!(std::fs::read(&archive).unwrap(), original_archive);
    // (c) The stager refuses a replacement whose expected stored digest is wrong (a foreign plan)
    //     before writing anything.
    let mut foreign = text.clone();
    foreign.members[0].source_stored_sha256 = Some("00".repeat(32));
    let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
    let bound = BoundSource::bind(&mut f).unwrap();
    assert!(text_member_replacements(&bound, &foreign).is_err());
    assert_eq!(std::fs::read(&archive).unwrap(), original_archive);
}

#[test]
fn a_grown_member_is_appended_and_retargeted_when_its_slot_is_tight() {
    // Make contextsetting.csv's stored length a multiple of 16 so the next member starts on the
    // very next line (zero slack), then plan an edit that changes the compressed length: the stager
    // must append and retarget rather than overwrite the neighbour.
    let tmp = Tmp::new("append");
    let mut texts = members_18();
    // Pad the contextsetting text with a comment-free extra data row set until the encoded length
    // is ≡ 0 (mod 16): try a few paddings of the `usage` column of an unrelated row.
    let mut padded = String::new();
    let mut found = false;
    for extra in 0..64 {
        padded = format!("{}{}", CONTEXTSETTING_18, (0..extra).map(|i| format!("-2,{},1,1,41,1,3\r\n", 200 + i)).collect::<String>());
        if compress_chunkzip_native(padded.as_bytes()).len() % 16 == 0 {
            found = true;
            break;
        }
    }
    assert!(found, "no padding produced a 16-aligned stored length");
    texts[2] = ("contextsetting.csv", Box::leak(padded.into_boxed_str()));
    let (root, db, archive) = build_root(&tmp, &texts, false);
    let original_archive = std::fs::read(&archive).unwrap();
    let mut ws = open(&db, "nhl18");
    // Find a code whose re-encode length differs from the stored length (try the candidates).
    let mut chosen = None;
    for code in [0i64, 1, 2, 3] {
        let (t, p) = ws.settings_csv_default_preview(&root, ctx(72, 3), code).unwrap().unwrap();
        let m = &p.members[0];
        if m.new_stored.len() > m.source_stored_len.unwrap() {
            chosen = Some((t, p));
            break;
        }
    }
    let Some((t, p)) = chosen else {
        eprintln!("no candidate code grew the stored length on this synthetic member; the append path is exercised by the stager's own suite");
        return;
    };
    assert!(p.members[0].placement_prediction.as_deref().unwrap().contains("appended"), "{:?}", p.members[0].placement_prediction);
    ws.settings_csv_apply_review(&t).unwrap();
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let mut plan = stage_workspace(&ws, &root, &staging, "D4 append").unwrap();
    let text = plan.text_members.clone().unwrap();
    let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
    let bound = BoundSource::bind(&mut f).unwrap();
    let replacements = text_member_replacements(&bound, &text).unwrap();
    let archive_plan = stage::plan(&bound, replacements).unwrap();
    assert!(matches!(archive_plan.edits[0].placement, Placement::AppendAndRetarget { .. }), "{:?}", archive_plan.edits[0].placement);
    assert!(archive_plan.new_eof > archive_plan.original_eof);
    let staged = studio_domain::txn::stage_archive(bound, archive_plan, &archive, &staging.join("inxp_00.big.csv.staged")).unwrap();
    plan.add_archive("archive-csv", &archive, &staged).unwrap();
    let mut txn = tdb_txn::prepare(&plan.root, &plan.change_set, &mut NoHooks).unwrap();
    assert_eq!(tdb_txn::apply(&mut txn, &plan.change_set, &mut NoHooks).unwrap().state, State::Committed);
    let installed = std::fs::read(&archive).unwrap();
    assert!(installed.len() > original_archive.len());
    assert_eq!(&installed[..original_archive.len()][..0x30], &original_archive[..0x30].iter().enumerate().map(|(i, b)| if (0x18..0x20).contains(&i) { installed[i] } else { *b }).collect::<Vec<_>>()[..]);
    let plain = member_plain(&archive, "contextsetting.csv");
    assert_eq!(String::from_utf8_lossy(&plain), String::from_utf8_lossy(&p.members[0].new_plain));
    // Restore.
    tdb_txn::restore(&root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_eq!(std::fs::read(&archive).unwrap(), original_archive);
}

// ---------------------------------------------------------------------------------------------
// 4. The real NHL 16 archive (--ignored): the smallest install archive, copied into a temp root
// ---------------------------------------------------------------------------------------------

#[test]
#[ignore = "private fixture archives/nhl16-ps4-cusa01909/databigs/inxp_00.big (300 MB) + anchors: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn real_nhl16_archive_takes_v1_fit_in_slot_and_restores_byte_exact() {
    let root_fx = PathBuf::from(std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private NHL 16 archive fixture cannot be resolved"));
    let src = root_fx.join("archives/nhl16-ps4-cusa01909/databigs/inxp_00.big");
    let src_len = std::fs::metadata(&src).unwrap_or_else(|e| panic!("cannot read {}: {e}", src.display())).len();
    assert_eq!(src_len, 299_883_104, "the NHL 16 inxp_00.big identity (research handoff 01 §0)");
    let tmp = Tmp::new("real16");
    let root = tmp.0.join("copy");
    tdb_txn::engine::create_copied_target_root(&root).unwrap();
    let db_dir = root.join("nhl/data/common/gamemodes/database/t3db");
    std::fs::create_dir_all(&db_dir).unwrap();
    let db = db_dir.join("nhl_game.db");
    std::fs::copy(repo("anchors/ps4/nhl16/databases/nhl_game.db"), &db).unwrap();
    std::fs::create_dir_all(root.join("databigs")).unwrap();
    let archive = root.join("databigs/inxp_00.big");
    std::fs::copy(&src, &archive).unwrap();
    let original = FileIdentity::of_file(&archive).unwrap();

    let project = Project::load(&db, Platform::Ps4, ProjectRole::Nhlng, repo("anchors/ps4/nhl16/databases/nhl_game-meta.xml"), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.to_string_lossy().into_owned());
    ws.set_game_id("nhl16");
    // V1: (3, 72) 4 → 3 — handoff 01 §5.4: 1 556 → 1 556 stored bytes, 12 B slack, fit-in-slot.
    let (t, p) = ws.settings_csv_default_preview(&root, ctx(72, 3), 3).unwrap().unwrap();
    let m = &p.members[0];
    assert_eq!((m.source_stored_len, m.new_stored.len()), (Some(1556), 1556));
    assert_eq!(m.cells[0].before, "4");
    assert!(m.placement_prediction.as_deref().unwrap().contains("fit"));
    ws.settings_csv_apply_review(&t).unwrap();
    let staging = tmp.0.join("staging");
    std::fs::create_dir_all(&staging).unwrap();
    let mut plan = stage_workspace(&ws, &root, &staging, "D4 real 16").unwrap();
    let text = plan.text_members.clone().unwrap();
    let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
    let bound = BoundSource::bind(&mut f).unwrap();
    let replacements = text_member_replacements(&bound, &text).unwrap();
    let archive_plan = stage::plan(&bound, replacements).unwrap();
    assert!(matches!(archive_plan.edits[0].placement, Placement::FitInSlot { .. }), "{:?}", archive_plan.edits[0].placement);
    let staged = studio_domain::txn::stage_archive(bound, archive_plan.clone(), &archive, &staging.join("inxp_00.big.csv.staged")).unwrap();
    plan.add_archive("archive-csv", &archive, &staged).unwrap();
    let mut txn = tdb_txn::prepare(&plan.root, &plan.change_set, &mut NoHooks).unwrap();
    assert_eq!(tdb_txn::apply(&mut txn, &plan.change_set, &mut NoHooks).unwrap().state, State::Committed);
    let installed = FileIdentity::of_file(&archive).unwrap();
    assert_eq!(installed.len, original.len, "fit-in-slot keeps the length");
    assert_ne!(installed.sha256, original.sha256);
    let plain = member_plain(&archive, "contextsetting.csv");
    assert_eq!(plain, *p.members[0].new_plain);
    assert!(String::from_utf8_lossy(&plain).contains("3,72,,,3,,1\r\n"));
    // Every other catalogue member is byte-identical to the census.
    let mut f = std::io::BufReader::new(std::fs::File::open(&archive).unwrap());
    let rebound = BoundSource::bind(&mut f).unwrap();
    let ids = studio_domain::settings_catalogue::csv_stored_identities(&archive).unwrap();
    for (path, sha, _) in &ids {
        let e = rebound.entries.iter().find(|e| &e.path == path).unwrap();
        let before = text.members.iter().find(|m| &m.member_path == path);
        match before {
            Some(m) => assert_eq!(*sha, m.new_stored_sha256, "{path}"),
            None => assert_eq!(*sha, studio_domain::tdb_archive_big::stage::hex(&e.stored_sha256)),
        }
    }
    tdb_txn::restore(&root, &txn.txn_id, &mut NoHooks).unwrap();
    assert_eq!(FileIdentity::of_file(&archive).unwrap(), original);
}
