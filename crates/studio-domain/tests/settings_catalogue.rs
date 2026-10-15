//! D2 / P1-BS-2 — the read-only settings catalogue: NHL 15 PS4 from the anchor's five tables,
//! NHL 16–21 PS4 from `setting_csv` members (synthetic members shaped exactly like the measured
//! variants; the real members `--ignored` under `NHL_STUDIO_FIXTURES`), and the blocker on every
//! other target. Offline implementation checks only; nothing here says what the game offers.

use std::path::{Path, PathBuf};

use studio_domain::settings_catalogue::{
    self as sc, CatalogueSource, CsvVariant, Eligibility, NameSource, SettingCsvMembers, SettingsDefs, DEFAULT_CONTEXT, ELIGIBLE_LEGACY_IDS,
};
use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};
use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn defs() -> SettingsDefs {
    SettingsDefs::load_dir(repo("defs/settings")).unwrap()
}

fn open(platform: Platform, game: &str, db: &Path, meta: &Path) -> Workspace {
    let project = Project::load(db, platform, ProjectRole::Nhlng, meta, repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.to_string_lossy().into_owned());
    ws.set_game_id(game);
    ws
}

fn nhl15_ws() -> Workspace {
    let dir = repo("anchors/ps4/nhl15/databases");
    open(Platform::Ps4, "nhl15", &dir.join("nhl_game.db"), &dir.join("nhl_game-meta.xml"))
}

// ---------------------------------------------------------------------------------------------
// 1. defs
// ---------------------------------------------------------------------------------------------

#[test]
fn the_nhl15_defs_carry_lineage_names_label_arrays_and_the_contract_lists() {
    let d = defs();
    let t = d.nhl15_ps4().expect("defs/settings/ps4-nhl15.toml");
    assert_eq!(t.witness.base_database_sha256, "ae9d1d56bf896b83b3376cc179e5bf3a6986cb594880f43de8a81e5b7e5078d8");
    assert_eq!(t.witness.patch_database_sha256, "abe36f58fd0a65ab074aff13f83f2e671b707afc539ee57c01df128200c320ff");
    assert_eq!(t.context_lineage_name(3).as_deref(), Some("FROM_DYNASTY_FLOW"));
    assert_eq!(t.context_lineage_name(37).as_deref(), Some("FROM_SEASON_MODE_FLOW"));
    assert_eq!(t.group_lineage_name(36), Some("GAMEPLAY_DEFAULT"));
    assert_eq!(t.label_keys(72).unwrap(), ["SingleKnockout", "BestOf3", "BestOf5", "BestOf7", "Authentic"]);
    assert_eq!(t.label_keys(68).unwrap().len(), 6);
    assert!(t.label_keys(74).is_none(), "LeagueName has no label array");
    assert_eq!(t.executable_record(72), Some(("PlayoffSeriesLength", 152)));
    for id in ELIGIBLE_LEGACY_IDS {
        assert!(t.eligible_role(id).is_some(), "role for {id}");
    }
    assert_eq!(t.eligible_role(72), Some("DYNASTY_PLAYOFFS_LENGTH"));
    assert!(t.excluded_reason(82).unwrap().contains("patch"));
    assert!(t.excluded_reason(15).is_some() && t.excluded_reason(72).is_none());
    assert!(d.for_title("nhl16", "ps4").is_none(), "NHL 16 has no defs file of its own (it uses the NHL 15 arrays by lineage)");
}

// ---------------------------------------------------------------------------------------------
// 2. NHL 15 PS4 — the anchor through the workspace
// ---------------------------------------------------------------------------------------------

#[test]
fn nhl15_anchor_catalogue_decodes_joins_and_discloses() {
    let ws = nhl15_ws();
    let cat = ws.display().settings_catalogue(None).unwrap().unwrap_or_else(|b| panic!("{b:?}"));
    assert_eq!((cat.game.as_str(), cat.platform.as_str()), ("nhl15", "ps4"));
    let CatalogueSource::Database { copy, tables, .. } = &cat.source else { panic!("database source") };
    assert_eq!(copy, "base 01.00");
    assert_eq!(tables.iter().map(|t| (t.name.as_str(), t.rows)).collect::<Vec<_>>(), [("stocksettings", 223), ("stocksettingdefaults", 373), ("stocksettingcontext", 197), ("stocksettinggroups", 702), ("stocksettingdependencies", 25)]);
    assert_eq!(cat.settings.len(), 223);
    assert!(cat.absent_eligible.is_empty(), "all 21 ids present on NHL 15");

    // PlayoffSeriesLength (72): id 152, 0..4, global 3 = BestOf7, contexts 3/5/30 stored 3, siblings 86/116.
    let s = cat.setting(72).unwrap();
    assert_eq!((s.name.as_str(), s.setting_id, s.min, s.max, s.size), ("PlayoffSeriesLength", Some(152), Some(0), Some(4), Some(5)));
    assert_eq!(s.siblings, [86, 116]);
    let g = s.global.as_ref().unwrap();
    assert_eq!((g.code, g.label.as_deref(), g.paired), (Some(3), Some("BestOf7"), false));
    assert!(g.cells[0].starts_with("stocksettings record 72 (legacyid 72) field default"), "{}", g.cells[0]);
    assert_eq!(s.label_source, NameSource::Inferred);
    assert_eq!(s.choices.iter().map(|c| (c.code, c.key.as_str())).collect::<Vec<_>>(), [(0, "SingleKnockout"), (1, "BestOf3"), (2, "BestOf5"), (3, "BestOf7"), (4, "Authentic")]);
    let ctxs: Vec<(i64, Option<i64>, Option<&str>)> = s.context_defaults.iter().map(|c| (c.context, c.stored.code, c.context_name.as_deref())).collect();
    assert_eq!(ctxs, [(3, Some(3), Some("FROM_DYNASTY_FLOW")), (5, Some(3), Some("FROM_MUD_CREATION_FLOW")), (30, Some(3), Some("FROM_BE_A_PRO_FLOW"))]);
    assert!(s.context_defaults[0].stored.cells[0].contains("(context 3, legacyid 72)"));
    assert_eq!(s.eligibility, Eligibility::Eligible { role: "DYNASTY_PLAYOFFS_LENGTH".into() });
    assert!(s.anomalies.is_empty(), "{:?}", s.anomalies);
    // NHLSalaryCap (70): global 0 = OFF, contexts 3 and 5 store 1 = ON.
    let s = cat.setting(70).unwrap();
    assert_eq!(s.global.as_ref().unwrap().label.as_deref(), Some("OFF"));
    assert_eq!(s.context_defaults.iter().map(|c| (c.context, c.stored.label.as_deref())).collect::<Vec<_>>(), [(3, Some("ON")), (5, Some("ON"))]);
    // Season (37) and Playoff (34) creation contexts: no stored defaults rows at all.
    assert_eq!(cat.context(37).map(|c| (c.defaults_rows, c.layout_rows)), Some((0, 3)));
    assert_eq!(cat.context(34).map(|c| c.defaults_rows), Some(0));
    assert_eq!(cat.context(3).map(|c| c.defaults_rows), Some(26));
    assert_eq!(cat.context(3).map(|c| c.name_source), Some(NameSource::Lineage));
    // Tournament NumberOfTeams (82) on the base copy: id 158, 0..1, excluded with the patch reason.
    let s = cat.setting(82).unwrap();
    assert_eq!((s.setting_id, s.max), (Some(158), Some(1)));
    assert!(matches!(&s.eligibility, Eligibility::Excluded { reason } if reason.contains("patch")));
    // A free-text row (74 LeagueName): excluded, no choices.
    let s = cat.setting(74).unwrap();
    assert!(matches!(s.eligibility, Eligibility::Excluded { .. }) && s.choices.is_empty() && s.size == Some(0));
    // Out-of-scope rows.
    assert_eq!(cat.setting(0).unwrap().eligibility, Eligibility::OutOfScope);
    // The advantage slider decodes -7..7 and TXT_LEAGUETYPE 222 shows the executable's empty record.
    let s = cat.settings.iter().find(|s| s.name == "PlayerAdvantage").unwrap();
    assert_eq!((s.min, s.max), (Some(-7), Some(7)));
    assert!(matches!(&cat.setting(222).unwrap().eligibility, Eligibility::Excluded { reason } if reason.contains("empty record")));
    assert!(cat.settings.iter().all(|s| !s.anomalies.iter().any(|a| a.contains("descriptor record carries"))), "every labelled executable record agrees with the database on the setting id");
    // Dependencies: TXT_LEAGUETYPE 115 → 13 dependents in order, first NumberOfTeams 122.
    let s = cat.setting(115).unwrap();
    assert_eq!(s.dependents.len(), 13);
    assert_eq!(s.dependents[0], 122);
    assert_eq!(cat.setting(123).unwrap().dependents, [67]);
    // Stock anomalies reported once and preserved: dangling 236 ×4, contexts 49–52 without layout,
    // container groups 36/74/75, duplicate (80, 94), 59 settings in no group, global default outside range.
    let a = cat.anomalies.join("\n");
    assert!(a.contains("4 row(s) reference a legacyid with no stocksettings row: (context 49, legacyid 236)"), "{a}");
    assert!(a.contains("stored defaults but no layout rows: 49, 50, 51, 52"), "{a}");
    assert!(a.contains("no member rows (containers): 36, 74, 75"), "{a}");
    assert!(a.contains("group 80 uses order 94 on 2 rows"), "{a}");
    assert!(a.contains("59 setting row(s) belong to no group"), "{a}");
    assert!(cat.settings.iter().find(|s| s.name == "TWOLINESOFFSIDE").unwrap().anomalies.iter().any(|x| x.contains("outside the row's own range")));
    // Groups: 36 is named by lineage and is a container (no members), its children carry parent 36.
    let g36 = cat.groups.iter().find(|g| g.id == 36).unwrap();
    assert_eq!((g36.name.as_deref(), g36.members), (Some("GAMEPLAY_DEFAULT"), 0));
    assert!(cat.groups.iter().filter(|g| g.parent == Some(36)).count() >= 6);
    // Notes keep the evidence distinctions.
    let n = cat.notes.join("\n");
    assert!(n.contains("structurally inferred") && n.contains("lineage") && n.contains("not established") && n.contains("two database copies"), "{n}");
    assert!(cat.dictionary.is_none());
}

#[test]
fn nhl15_anchor_labels_resolve_through_the_attached_dictionary() {
    let mut ws = nhl15_ws();
    ws.attach_dictionary(repo("anchors/ps4/nhl15/databases/eng_us.db").to_str().unwrap()).unwrap();
    let cat = ws.display().settings_catalogue(None).unwrap().unwrap();
    assert!(cat.dictionary.is_some());
    let s = cat.setting(68).unwrap();
    assert_eq!(s.label_key.as_deref(), Some("SeasonTieBreak"));
    assert!(s.label_text.as_deref().is_some_and(|t| t.to_lowercase().contains("tie")), "{:?}", s.label_text);
    let g = s.global.as_ref().unwrap();
    assert_eq!(g.code, Some(2));
    assert!(g.label.as_deref().is_some_and(|l| l != "5MinThenShootout" && !l.is_empty()), "resolved text, not the key: {:?}", g.label);
    assert!(s.choices.iter().all(|c| c.key == "5MinThenShootout" || c.text.is_some() || c.key.is_empty()) || s.choices.iter().any(|c| c.text.is_some()));
}

#[test]
fn nhl15_edited_working_copy_is_read_as_unrecorded_and_undo_restores_the_base_copy() {
    let mut ws = nhl15_ws();
    let before = ws.display().settings_catalogue(None).unwrap().unwrap();
    assert_eq!(before.setting(72).unwrap().global.as_ref().unwrap().code, Some(3));
    // The generic cell editor (studio ids = raw ids on PS4) changes the global default of
    // PlayoffSeriesLength; the catalogue reads the working state, not the file.
    ws.set_field("fRvu", 72, "npHF", studio_domain::FieldValue::Int(2)).unwrap();
    let after = ws.display().settings_catalogue(None).unwrap().unwrap();
    let g = after.setting(72).unwrap().global.as_ref().unwrap();
    assert_eq!((g.code, g.label.as_deref()), (Some(2), Some("BestOf5")));
    let CatalogueSource::Database { copy, .. } = &after.source else { panic!() };
    assert!(copy.starts_with("unrecorded"), "{copy}");
    // Siblings and other rows are untouched.
    assert_eq!(after.setting(86).unwrap().global.as_ref().unwrap().code, Some(2));
    assert_eq!(after.setting(116).unwrap().global.as_ref().unwrap().code, Some(3));
    assert!(ws.undo());
    let back = ws.display().settings_catalogue(None).unwrap().unwrap();
    assert_eq!(back.setting(72).unwrap().global.as_ref().unwrap().code, Some(3));
    let CatalogueSource::Database { copy, .. } = &back.source else { panic!() };
    assert_eq!(copy, "base 01.00");
}

/// The private patch 01.06 database: the same declaration, `patch 01.06` copy, the re-pointed row
/// (82 → id 186, 0..3) now excluded by the same reason, and the six context-9 defaults.
#[test]
#[ignore = "private fixture databases/nhl15-ps4-cusa00561/patch_inxp00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn nhl15_patch_database_is_read_as_the_patch_copy() {
    let root = std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private NHL 15 PS4 patch fixture cannot be resolved");
    let db = PathBuf::from(root).join("databases/nhl15-ps4-cusa00561/patch_inxp00.big/nhl_game.db");
    let bytes = std::fs::read(&db).unwrap_or_else(|e| panic!("cannot read {}: {e}", db.display()));
    assert_eq!(bytes.len(), 16_981_204);
    let ws = open(Platform::Ps4, "nhl15", &db, &repo("anchors/ps4/nhl15/databases/nhl_game-meta.xml"));
    let cat = ws.display().settings_catalogue(None).unwrap().unwrap();
    let CatalogueSource::Database { copy, tables, .. } = &cat.source else { panic!() };
    assert_eq!(copy, "patch 01.06");
    assert_eq!(tables.iter().map(|t| t.rows).collect::<Vec<_>>(), [223, 379, 201, 704, 25]);
    let s = cat.setting(82).unwrap();
    assert_eq!((s.setting_id, s.min, s.max, s.size), (Some(186), Some(0), Some(3), Some(4)));
    assert!(s.choices.is_empty() || s.choices.len() == 2, "the defs carry the base copy's 2-entry array; a 4-value row does not take it: {:?}", s.choices);
    assert!(s.anomalies.iter().any(|a| a.contains("label array")), "{:?}", s.anomalies);
    let ctx9: Vec<(i64, Option<i64>)> = cat.settings.iter().flat_map(|s| s.context_defaults.iter().filter(|c| c.context == 9).map(move |c| (s.legacy_id, c.stored.code))).collect();
    assert_eq!(ctx9.len(), 6);
    assert!(ctx9.contains(&(86, Some(3))) && ctx9.contains(&(73, Some(0))) && ctx9.contains(&(82, Some(3))));
    assert_eq!(cat.context(9).map(|c| c.defaults_rows), Some(6));
}

// ---------------------------------------------------------------------------------------------
// 3. NHL 16–21 — synthetic members shaped exactly like the measured variants
// ---------------------------------------------------------------------------------------------

const NHL16: &[(&str, &str)] = &[
    ("setting.csv", "id,key,name,string_id,description_id,is_choice,min_value,max_value,default_value,remap_id\r\n0,SKILL_LEVEL,Setting-0,SkillLevel,TXT_SETTING_DESC_0,1,0,3,1,9\r\n67,DYNASTY_STARTING_ROSTERS,Setting-67,StartingRosters,TXT_SETTING_DESC_67,1,0,1,0,154\r\n72,DYNASTY_PLAYOFFS_LENGTH,Setting-72,PlayoffSeriesLength,TXT_SETTING_DESC_72,1,0,4,3,153\r\n71,DYNASTY_SEASON_OVERTIME_LOSS,Setting-71,SeasonOvertimeLoss,TXT_SETTING_DESC_71,1,0,1,1,155\r\n74,LEAGUE_NAME,Setting-74,LeagueName,TXT_SETTING_DESC_74,0,0,0,,212\r\n"),
    ("context.csv", "id,key,name\r\n0,FROM_PLAY_NOW_FLOW,Context-0\r\n3,FROM_DYNASTY_FLOW,Context-3\r\n37,FROM_SEASON_MODE_FLOW,Context-37\r\n"),
    ("contextsetting.csv", "context_id,setting_id,min_value,max_value,default_value,choices,is_enabled\r\n0,0,,,1,,1\r\n3,72,,,4,,1\r\n37,72,,,4,,1\r\n3,71,,,1,,1\r\n3,0,,,,,1\r\n0,5,,,2,,1\r\n"),
    ("contextgroup.csv", "context_id,group_id,order\r\n0,0,1\r\n3,5,1\r\n37,11,1\r\n"),
    ("group.csv", "id,parent_id,string_id,name\r\n0,74,TXT_CATEGORY_0,QUICK_DEFAULT\r\n5,,TXT_CATEGORY_5,QUICK_DYNASTY_FLOW\r\n11,,TXT_CATEGORY_11,QUICK_SEASON_DEFAULT\r\n74,,TXT_CATEGORY_74,RULES_AND_GAMEPLAY_DEFAULT\r\n"),
    ("settinggroup.csv", "setting_id,group_id,order\r\n0,0,1\r\n72,5,1\r\n72,11,2\r\n"),
    ("stocksettingdependencies.csv", "LegacyID,Order,DependentLegacyID\r\n123,1,67\r\n"),
    ("choice.csv", "id,setting_id,key,string_id,name\r\n0,0,CHOICE_0,TXT_CHOICE_0,Choice-0\r\n"),
];

const NHL17: &[(&str, &str)] = &[
    ("setting.csv", "id,name,legacy_id,is_global,is_choice,choice_to_range,name_loc,description_loc,extra_loc\r\n0,SKILL_LEVEL,9,0,1,0,SkillLevel,SkillLevel_Desc,\r\n72,DYNASTY_PLAYOFFS_LENGTH,153,0,1,0,PlayoffSeriesLength,PlayoffSeriesLength_Desc,\r\n86,PLAYOFF_ELIMINATION_SERIES_LENGTH,153,0,1,0,PlayoffSeriesLength,,\r\n88,TOURNAMENT_ROUND_ROBIN_TIE_BREAK,18,0,1,0,RoundRobinTieBreak,,\r\n4,PLAYER_ADVANTAGE,1,0,0,1,PlayerAdvantage,,\r\n"),
    ("context.csv", "id,name\r\n-2,_DEFAULT_\r\n0,FROM_PLAY_NOW_FLOW\r\n3,FROM_DYNASTY_FLOW\r\n80,FROM_TOURNAMENT_FLOW\r\n"),
    ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values\r\n-2,0,1,1,41\r\n-2,72,1,4,9\r\n3,72,1,4,9\r\n-2,86,1,3,53\r\n-2,88,1,0,10\r\n80,88,1,2,12\r\n-2,4,1,7,0\r\n"),
    ("contextgroup.csv", "context_id,group_id,order\r\n0,0,1\r\n3,5,1\r\n"),
    ("group.csv", "id,name,parent_id,name_loc\r\n0,QUICK_DEFAULT,74,QuickSettings\r\n5,QUICK_DEFAULT_GM,,QuickSettings\r\n"),
    ("groupsetting.csv", "group_id,setting_id,order\r\n0,0,4\r\n5,72,1\r\n"),
    ("choicelist.csv", "id,name\r\n9,DYNASTY_PLAYOFFS_LENGTH\r\n10,DYNASTY_PLAYOFFS_TIE_BREAK\r\n12,DYNASTY_SEASON_TIE_BREAK\r\n41,SKILL\r\n53,PLAYOFF_SERIES_LENGTH\r\n"),
    ("choice.csv", "id,choicelist_id,name,value,name_loc,description_loc\r\n0,9,SINGLE,0,SingleKnockout,\r\n1,9,B3,1,BestOf3,\r\n2,9,B5,2,BestOf5,\r\n3,9,B7,3,BestOf7,\r\n4,9,AUTH,4,Authentic,\r\n0,10,C,0,ContinuousOT,\r\n1,10,S,1,Shootout,\r\n2,10,O3,2,OT3ThenShootout,\r\n3,10,M5,3,5MinThenShootout,\r\n0,12,C,0,ContinuousOT,\r\n1,12,S,1,Shootout,\r\n2,12,O3,2,OT3ThenShootout,\r\n3,12,M5,3,5MinThenShootout,\r\n4,12,S44,4,SingleOt4on4,\r\n5,12,S55,5,SingleOt5on5,\r\n6,12,NO,6,NoOvertime,\r\n7,12,A,7,authentic,\r\n0,41,R,0,Rookie,\r\n1,41,P,1,Pro,\r\n0,53,SINGLE,0,SingleKnockout,\r\n1,53,B3,1,BestOf3,\r\n2,53,B5,2,BestOf5,\r\n3,53,B7,3,BestOf7,\r\n4,53,AUTH,4,Authentic,\r\n"),
    ("stocksettingdependencies.csv", "LegacyID,Order,DependentLegacyID\r\n123,1,67\r\n"),
];

const NHL18: &[(&str, &str)] = &[
    ("setting.csv", "id,name,is_global,is_choice,choice_to_range,name_loc,description_loc,extra_loc,default_value,potential_values,usage\r\n0,SKILL_LEVEL,0,1,0,SkillLevel,SkillLevel_Desc,,1,41,3\r\n72,DYNASTY_PLAYOFFS_LENGTH,0,1,0,PlayoffSeriesLength,PlayoffSeriesLength_Desc,,4,9,3\r\n120,GAME_MODE_HOME_AND_AWAY_SCHEDULE,0,1,0,HomeAwayType,,,1,34,3\r\n117,GAME_MODE_CUP_TYPE,0,1,0,TrophyType,,,0,57,3\r\n"),
    ("context.csv", "id,name\r\n-2,_DEFAULT_\r\n0,DISPLAY_PLAY_NOW_MODE\r\n3,DISPLAY_FRANCHISE_CREATION_CLASIC\r\n"),
    ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage\r\n-2,0,1,1,41,1,3\r\n-2,72,1,4,9,1,3\r\n3,72,1,4,9,1,3\r\n-2,120,1,0,34,1,3\r\n3,120,1,1,34,0,2\r\n"),
    ("contextgroup.csv", "context_id,group_id,order\r\n0,0,1\r\n3,5,1\r\n"),
    ("group.csv", "id,name,parent_id,name_loc\r\n0,QUICK_DEFAULT,74,GameSettings\r\n5,QUICK_DEFAULT_GM,,QuickSettings\r\n"),
    ("groupsetting.csv", "group_id,setting_id,order\r\n0,0,4\r\n5,72,1\r\n"),
    ("choicelist.csv", "id,name\r\n9,DYNASTY_PLAYOFFS_LENGTH\r\n34,RULE_TOGGLE\r\n41,SKILL\r\n57,GAME_MODE_CUP_TYPE\r\n"),
    ("choice.csv", "id,choicelist_id,name,value,name_loc,description_loc\r\n0,9,SINGLE,0,SingleKnockout,\r\n1,9,B3,1,BestOf3,\r\n2,9,B5,2,BestOf5,\r\n3,9,B7,3,BestOf7,\r\n4,9,AUTH,4,Authentic,\r\n0,34,OFF,0,OFF,\r\n1,34,ON,1,ON,\r\n0,41,R,0,Rookie,\r\n1,41,P,1,Pro,\r\n0,57,A,0,TXT_BATTLE_FOR_THE_CUP_NOW,\r\n1,57,B,1,TXT_HEAD_TO_HEAD_MODE,\r\n"),
];

#[test]
fn nhl16_variant_reads_one_global_cell_with_inferred_labels_and_reports_nulls() {
    let d = defs();
    let m = SettingCsvMembers::from_texts("synthetic-nhl16", NHL16);
    let cat = sc::read_setting_csv("nhl16", &m, d.nhl15_ps4(), None).unwrap_or_else(|b| panic!("{b:?}"));
    let CatalogueSource::SettingCsv { variant, members, .. } = &cat.source else { panic!() };
    assert_eq!(*variant, CsvVariant::Nhl16);
    assert_eq!(members.len(), 8);
    let s = cat.setting(72).unwrap();
    assert_eq!((s.name.as_str(), s.min, s.max, s.size, s.setting_id), ("DYNASTY_PLAYOFFS_LENGTH", Some(0), Some(4), Some(5), None));
    assert_eq!(s.label_key.as_deref(), Some("PlayoffSeriesLength"));
    assert_eq!(s.label_source, NameSource::Inferred);
    assert_eq!(s.choices.len(), 5);
    let g = s.global.as_ref().unwrap();
    assert_eq!((g.code, g.label.as_deref(), g.paired), (Some(3), Some("BestOf7"), false));
    assert_eq!(g.cells, ["setting.csv line 4 column default_value"]);
    assert_eq!(s.context_defaults.iter().map(|c| (c.context, c.stored.code, c.stored.label.as_deref(), c.context_name.as_deref())).collect::<Vec<_>>(), [(3, Some(4), Some("Authentic"), Some("FROM_DYNASTY_FLOW")), (37, Some(4), Some("Authentic"), Some("FROM_SEASON_MODE_FLOW"))]);
    assert_eq!(s.context_defaults[0].stored.cells, ["contextsetting.csv line 3 (context 3, setting 72) column default_value"]);
    assert_eq!(s.context_defaults[0].enabled, Some(true));
    assert_eq!(s.context_defaults[0].visible, None);
    assert_eq!(s.groups, [5, 11]);
    assert_eq!(s.eligibility, Eligibility::Eligible { role: "DYNASTY_PLAYOFFS_LENGTH".into() });
    // StartingRosters 67 is 0..1 on NHL 16 while the NHL 15 array has 3 entries: labels withheld.
    let s = cat.setting(67).unwrap();
    assert!(s.choices.is_empty() && s.anomalies.iter().any(|a| a.contains("3 entries")), "{:?}", s.anomalies);
    // Null default and a free-text row.
    assert!(cat.anomalies.iter().any(|a| a.contains("1 row(s) store an empty default")), "{:?}", cat.anomalies);
    let s = cat.setting(74).unwrap();
    assert_eq!(s.global.as_ref().unwrap().code, None);
    assert!(matches!(s.eligibility, Eligibility::Excluded { .. }));
    // A contextsetting row for an unknown setting (0,5) is counted as dangling.
    assert!(cat.anomalies.iter().any(|a| a.contains("1 row(s) reference a setting id with no setting.csv row")));
    assert_eq!(cat.absent_eligible.len(), 18);
    assert_eq!(cat.context(3).map(|c| (c.defaults_rows, c.layout_rows, c.name_source)), Some((3, 1, NameSource::Explicit)));
    assert_eq!(cat.setting(72).unwrap().dependents, Vec::<i64>::new());
}

#[test]
fn nhl17_variant_reads_the_default_context_row_explicit_lists_and_context_specific_domains() {
    let d = defs();
    let m = SettingCsvMembers::from_texts("synthetic-nhl17", NHL17);
    let cat = sc::read_setting_csv("nhl17", &m, d.nhl15_ps4(), None).unwrap_or_else(|b| panic!("{b:?}"));
    let CatalogueSource::SettingCsv { variant, .. } = &cat.source else { panic!() };
    assert_eq!(*variant, CsvVariant::Nhl17);
    let s = cat.setting(72).unwrap();
    assert_eq!((s.setting_id, s.siblings.as_slice(), s.min, s.max, s.size), (Some(153), &[86][..], Some(0), Some(4), Some(5)));
    assert_eq!(s.label_source, NameSource::Explicit);
    let g = s.global.as_ref().unwrap();
    assert_eq!((g.code, g.label.as_deref(), g.paired), (Some(4), Some("Authentic"), false));
    assert_eq!(g.cells, ["contextsetting.csv line 3 (_DEFAULT_ context -2, setting 72) column default_value"]);
    // The reserved context is never listed as an ordinary screen row of the setting.
    assert_eq!(s.context_defaults.iter().map(|c| c.context).collect::<Vec<_>>(), [3]);
    assert!(cat.context(DEFAULT_CONTEXT).unwrap().reserved);
    assert!(!cat.context(3).unwrap().reserved);
    // 88: global list 10 (4 values), tournament context 80 list 12 (8 values, default 2 = OT3ThenShootout).
    let s = cat.setting(88).unwrap();
    assert_eq!(s.choices.len(), 4);
    assert_eq!(s.global.as_ref().unwrap().label.as_deref(), Some("ContinuousOT"));
    let t = &s.context_defaults[0];
    assert_eq!((t.context, t.stored.code, t.stored.label.as_deref()), (80, Some(2), Some("OT3ThenShootout")));
    assert_eq!(t.choices.as_ref().map(Vec::len), Some(8));
    assert!(t.domain_note.as_deref().unwrap().contains("DYNASTY_SEASON_TIE_BREAK"));
    // A range setting is flagged; the eligible ids absent on 17+ are listed.
    assert!(cat.setting(4).unwrap().anomalies.iter().any(|a| a.contains("range-based")));
    assert!(cat.absent_eligible.contains(&83) && cat.absent_eligible.contains(&123));
    assert_eq!(cat.setting(86).unwrap().eligibility, Eligibility::Eligible { role: "PLAYOFF_ELIMINATION_SERIES_LENGTH".into() });
}

#[test]
fn nhl18_variant_reads_the_global_pair_with_its_consistency_and_visibility() {
    let d = defs();
    let m = SettingCsvMembers::from_texts("synthetic-nhl18", NHL18);
    let cat = sc::read_setting_csv("nhl18", &m, d.nhl15_ps4(), None).unwrap_or_else(|b| panic!("{b:?}"));
    let CatalogueSource::SettingCsv { variant, .. } = &cat.source else { panic!() };
    assert_eq!(*variant, CsvVariant::Nhl18Plus);
    let s = cat.setting(72).unwrap();
    let g = s.global.as_ref().unwrap();
    assert_eq!((g.code, g.paired, g.consistent), (Some(4), true, Some(true)));
    assert_eq!(g.values, [Some(4), Some(4)]);
    assert_eq!(g.cells, ["setting.csv line 3 column default_value", "contextsetting.csv line 3 (_DEFAULT_ context -2, setting 72) column default_value"]);
    assert!(s.anomalies.is_empty());
    assert_eq!(s.setting_id, None);
    // 120: the pair disagrees (0 vs … no: setting 1, _DEFAULT_ 0) → inconsistent, both values shown.
    let s = cat.setting(120).unwrap();
    let g = s.global.as_ref().unwrap();
    assert_eq!((g.consistent, g.values.as_slice()), (Some(false), &[Some(1), Some(0)][..]));
    assert!(s.anomalies.iter().any(|a| a.contains("disagree")), "{:?}", s.anomalies);
    assert_eq!(s.choices.len(), 2, "RULE_TOGGLE has two entries on 17+");
    let c = &s.context_defaults[0];
    assert_eq!((c.context, c.enabled, c.visible), (3, Some(true), Some(false)));
    // 117: no _DEFAULT_ row at all → stored once, reported.
    let s = cat.setting(117).unwrap();
    assert!(!s.global.as_ref().unwrap().paired);
    assert!(s.anomalies.iter().any(|a| a.contains("no _DEFAULT_ row")));
    assert_eq!(s.choices.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(), ["TXT_BATTLE_FOR_THE_CUP_NOW", "TXT_HEAD_TO_HEAD_MODE"]);
    assert!(cat.notes.iter().any(|n| n.contains("stored twice")));
}

#[test]
fn csv_members_outside_the_subset_or_with_ambiguous_headers_are_refused() {
    let d = defs();
    let mut quoted: Vec<(&str, &str)> = NHL18.to_vec();
    quoted[2] = ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage\r\n-2,72,1,\"4\",9,1,3\r\n");
    let m = SettingCsvMembers::from_texts("q", &quoted);
    let b = sc::read_setting_csv("nhl18", &m, d.nhl15_ps4(), None).unwrap_err();
    assert_eq!(b.kind, "member-refused");
    assert!(b.reason.contains("quote"), "{}", b.reason);

    let mut dup: Vec<(&str, &str)> = NHL18.to_vec();
    dup[0] = ("setting.csv", "id,name,is_global,is_choice,choice_to_range,name_loc,description_loc,extra_loc,default_value,potential_values,usage,default_value\r\n");
    let m = SettingCsvMembers::from_texts("d", &dup);
    let b = sc::read_setting_csv("nhl18", &m, d.nhl15_ps4(), None).unwrap_err();
    assert!(b.reason.contains("`default_value` appears twice"), "{}", b.reason);

    let mut bom: Vec<(&str, &str)> = NHL18.to_vec();
    bom[1] = ("context.csv", "\u{feff}id,name\r\n-2,_DEFAULT_\r\n");
    let m = SettingCsvMembers::from_texts("b", &bom);
    assert!(sc::read_setting_csv("nhl18", &m, d.nhl15_ps4(), None).unwrap_err().reason.contains("byte-order mark"));

    let mut missing: Vec<(&str, &str)> = NHL18.to_vec();
    missing.remove(2);
    let m = SettingCsvMembers::from_texts("m", &missing);
    let b = sc::read_setting_csv("nhl18", &m, d.nhl15_ps4(), None).unwrap_err();
    assert_eq!(b.kind, "archive-missing");

    let mut unknown: Vec<(&str, &str)> = NHL18.to_vec();
    unknown[0] = ("setting.csv", "id,foo\r\n1,2\r\n");
    let m = SettingCsvMembers::from_texts("u", &unknown);
    assert_eq!(sc::read_setting_csv("nhl18", &m, d.nhl15_ps4(), None).unwrap_err().kind, "parse");

    // Ragged records are skipped and reported, extra columns are carried, LF-only records parse.
    let mut ragged: Vec<(&str, &str)> = NHL18.to_vec();
    ragged[2] = ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage,extra\n-2,72,1,4,9,1,3,x\n3,72,1,4\n3,72,1,3,9,1,3,y\n");
    let m = SettingCsvMembers::from_texts("r", &ragged);
    let cat = sc::read_setting_csv("nhl18", &m, d.nhl15_ps4(), None).unwrap();
    assert!(cat.anomalies.iter().any(|a| a.contains("contextsetting.csv: 1 record(s) with a column count")), "{:?}", cat.anomalies);
    assert_eq!(cat.setting(72).unwrap().context_defaults[0].stored.code, Some(3));
    // Duplicate rows are ambiguous, never silently merged.
    let mut dupe: Vec<(&str, &str)> = NHL18.to_vec();
    dupe[2] = ("contextsetting.csv", "context_id,setting_id,is_enabled,default_value,potential_values,is_visible,usage\r\n-2,72,1,4,9,1,3\r\n3,72,1,4,9,1,3\r\n3,72,1,2,9,1,3\r\n");
    let m = SettingCsvMembers::from_texts("dd", &dupe);
    let cat = sc::read_setting_csv("nhl18", &m, d.nhl15_ps4(), None).unwrap();
    assert!(cat.anomalies.iter().any(|a| a.contains("(context 3, setting 72) stored 2 times")));
    assert_eq!(cat.setting(72).unwrap().context_defaults.len(), 2);
}

// ---------------------------------------------------------------------------------------------
// 4. Session routing and blockers
// ---------------------------------------------------------------------------------------------

#[test]
fn a_ps4_csv_title_needs_its_install_root_and_a_missing_archive_is_a_blocker() {
    let dir = repo("anchors/ps4/nhl18/databases");
    let ws = open(Platform::Ps4, "nhl18", &dir.join("nhl_game.db"), &dir.join("nhl_game-meta.xml"));
    let b = ws.display().settings_catalogue(None).unwrap().unwrap_err();
    assert_eq!(b.kind, "no-install-root");
    let tmp = std::env::temp_dir().join(format!("nms-settings-{}", std::process::id()));
    std::fs::create_dir_all(&tmp).unwrap();
    let b = ws.display().settings_catalogue(Some(&tmp)).unwrap().unwrap_err();
    assert_eq!(b.kind, "archive-missing");
    assert!(b.reason.contains("inxp_00.big"));
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn gen7_and_ps3_targets_are_blocked_with_the_reason() {
    let dir = repo("anchors/xbox360/nhllegacy/databases");
    let ws = open(Platform::Xbox, "nhllegacy", &dir.join("nhlng.db"), &dir.join("nhlng-meta.xml"));
    let b = ws.display().settings_catalogue(None).unwrap().unwrap_err();
    assert_eq!(b.kind, "unsupported-title");
    assert!(b.reason.contains("compiled table of the executable"), "{}", b.reason);
    let dir = repo("anchors/ps3/nhl12/databases");
    let ws = open(Platform::Ps3, "nhl12", &dir.join("nhlng.db"), &dir.join("nhlng-meta.xml"));
    let b = ws.display().settings_catalogue(None).unwrap().unwrap_err();
    assert!(b.reason.contains("PS3") && b.reason.contains("encrypted"), "{}", b.reason);
    let dir = repo("anchors/xbox360/nhl09/databases");
    let ws = open(Platform::Xbox, "nhl09", &dir.join("nhlng.db"), &dir.join("nhlng-meta.xml"));
    let b = ws.display().settings_catalogue(None).unwrap().unwrap_err();
    assert!(b.reason.contains("no settings catalogue was found"), "{}", b.reason);
}

/// A database with the five tables under a different declaration is refused rather than misread.
#[test]
fn a_mismatched_declaration_is_refused_not_misread() {
    let bytes = std::fs::read(repo("anchors/ps4/nhl15/databases/nhl_game.db")).unwrap();
    let db = TdbFile::parse(&bytes).unwrap();
    let xml = std::fs::read_to_string(repo("anchors/ps4/nhl15/databases/nhl_game-meta.xml")).unwrap();
    // Re-declare `min` as 5 bits: the scoped site no longer matches, so the reader refuses.
    let altered = xml.replace(r#"name="min" shortname="SbEu" type="FIELDTYPE_INTEGER" depth="4""#, r#"name="min" shortname="SbEu" type="FIELDTYPE_INTEGER" depth="5""#);
    assert_ne!(altered, xml);
    let meta = FieldMetaIndex::from_xml(&altered).unwrap();
    let d = defs();
    let b = sc::read_database(&db, &meta, d.nhl15_ps4(), None).unwrap_err();
    assert_eq!(b.kind, "declaration-mismatch");
    assert!(b.reason.contains("stocksettings.min"), "{}", b.reason);
}

// ---------------------------------------------------------------------------------------------
// 5. Private fixtures: the real NHL 16–21 members (--ignored)
// ---------------------------------------------------------------------------------------------

fn fixture_dir(rel: &str) -> PathBuf {
    let root = std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the private setting_csv fixtures cannot be resolved");
    let p = PathBuf::from(root).join(rel);
    assert!(p.is_dir(), "{} is not a directory", p.display());
    p
}

#[test]
#[ignore = "private fixtures databases/nhl{16..21}-ps4-*/inxp_00.big/setting_csv: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn real_nhl16_to_21_members_read_with_the_measured_counts() {
    let d = defs();
    // (title, dir, variant, setting rows, context rows, contextsetting rows, eligible present, absent)
    let cases = [
        ("nhl16", "databases/nhl16-ps4-cusa01909/inxp_00.big/setting_csv", CsvVariant::Nhl16, 267, 61, 570, 21),
        ("nhl17", "databases/nhl17-ps4-cusa04326/inxp_00.big/setting_csv", CsvVariant::Nhl17, 351, 102, 8306, 16),
        ("nhl18", "databases/nhl18-ps4-cusa07569/inxp_00.big/setting_csv", CsvVariant::Nhl18Plus, 376, 111, 8944, 16),
        ("nhl19", "databases/nhl19-ps4-cusa11126/inxp_00.big/setting_csv", CsvVariant::Nhl18Plus, 389, 111, 10232, 16),
        ("nhl20", "databases/nhl20-ps4-cusa15439/inxp_00.big/setting_csv", CsvVariant::Nhl18Plus, 409, 117, 10703, 16),
        ("nhl21", "databases/nhl21-ps4-cusa18182/inxp_00.big/setting_csv", CsvVariant::Nhl18Plus, 427, 120, 10969, 16),
    ];
    for (game, rel, variant, settings, contexts, cs_rows, eligible) in cases {
        let m = SettingCsvMembers::from_dir(&fixture_dir(rel)).unwrap();
        let cat = sc::read_setting_csv(game, &m, d.nhl15_ps4(), None).unwrap_or_else(|b| panic!("{game}: {b:?}"));
        let CatalogueSource::SettingCsv { variant: v, members, .. } = &cat.source else { panic!() };
        assert_eq!(*v, variant, "{game}");
        assert!(members.len() >= 9, "{game}: {} members", members.len());
        assert_eq!(cat.settings.len(), settings, "{game} setting rows");
        assert_eq!(cat.contexts.len(), contexts, "{game} contexts");
        assert_eq!(cat.contexts.iter().map(|c| c.defaults_rows).sum::<usize>(), cs_rows, "{game} contextsetting rows");
        let present = cat.settings.iter().filter(|s| matches!(s.eligibility, Eligibility::Eligible { .. })).count();
        assert_eq!(present, eligible, "{game} eligible present");
        assert_eq!(cat.absent_eligible.len(), 21 - eligible, "{game} absent");
        // No ambiguity in stock: no duplicate keys anywhere.
        assert!(!cat.anomalies.iter().any(|a| a.contains("ambiguous")), "{game}: {:?}", cat.anomalies);
        // 72 DYNASTY_PLAYOFFS_LENGTH: global 3 on NHL 16, 4 on 17–21; context 3 stores 4 everywhere.
        let s = cat.setting(72).unwrap();
        assert_eq!(s.name, "DYNASTY_PLAYOFFS_LENGTH", "{game}");
        let g = s.global.as_ref().unwrap();
        assert_eq!(g.code, Some(if game == "nhl16" { 3 } else { 4 }), "{game} global 72");
        assert_eq!(g.paired, variant == CsvVariant::Nhl18Plus, "{game} paired");
        if variant == CsvVariant::Nhl18Plus {
            assert_eq!(g.consistent, Some(true), "{game}");
        }
        assert_eq!(s.context_defaults.iter().find(|c| c.context == 3).map(|c| c.stored.code), Some(Some(4)), "{game} (3,72)");
        assert_eq!(s.choices.iter().map(|c| c.key.as_str()).collect::<Vec<_>>(), ["SingleKnockout", "BestOf3", "BestOf5", "BestOf7", "Authentic"], "{game}");
        // Every eligible row: consistent pair on 18–21; every stored default a value of its list.
        for s in cat.settings.iter().filter(|s| matches!(s.eligibility, Eligibility::Eligible { .. })) {
            let g = s.global.as_ref().unwrap();
            if variant == CsvVariant::Nhl18Plus {
                assert_eq!(g.consistent, Some(true), "{game} {} pair", s.legacy_id);
            }
            assert!(g.code.is_some(), "{game} {} null global", s.legacy_id);
            for c in &s.context_defaults {
                assert!(c.anomaly.is_none(), "{game} {} ctx {}: {:?}", s.legacy_id, c.context, c.anomaly);
                assert_ne!(c.enabled, Some(false), "{game} {} ctx {}", s.legacy_id, c.context);
            }
            if variant != CsvVariant::Nhl16 {
                assert!(!s.choices.is_empty(), "{game} {} explicit list", s.legacy_id);
            }
        }
        // NHL 16: 88 empty defaults (none eligible); NHL 17+: 88's tournament rows use list 12.
        if variant == CsvVariant::Nhl16 {
            assert!(cat.anomalies.iter().any(|a| a.contains("88 row(s) store an empty default")), "{:?}", cat.anomalies);
            assert_eq!(cat.setting(67).unwrap().max, Some(1));
        } else {
            let s = cat.setting(88).unwrap();
            let t = s.context_defaults.iter().find(|c| c.choices.is_some()).expect("a tournament row with its own list");
            assert_eq!((t.stored.code, t.choices.as_ref().map(Vec::len)), (Some(2), Some(8)), "{game} 88 tournament row");
            assert_eq!(cat.setting(117).unwrap().choices.len(), 2, "{game} cup type");
            assert_eq!(cat.setting(68).unwrap().global.as_ref().unwrap().code, Some(7), "{game} tie-break authentic");
        }
    }
}
