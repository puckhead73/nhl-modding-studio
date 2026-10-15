//! Real, opt-in roster-mode regression for the unaligned NHL19 baseline pair.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{FieldValue, Platform};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
#[ignore = "requires NHL19_PS4_ROSTER_ONLY fresh output; originals are read-only"]
fn unaligned_pair_roster_mode_saves_only_roster() {
    let out = PathBuf::from(std::env::var("NHL19_PS4_ROSTER_ONLY").unwrap());
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let anchor = repo("anchors/ps4/nhl19");
    let original_db = std::fs::read(anchor.join("databases/nhl_game.db")).unwrap();
    let original_roster = std::fs::read(anchor.join("savefiles/RosterCustom0/DATA")).unwrap();
    let db = out.join("nhl_game.db");
    let roster = out.join("DATA");
    std::fs::write(&db, &original_db).unwrap();
    std::fs::write(&roster, &original_roster).unwrap();
    let meta = anchor.join("databases/nhl_game-meta.xml");
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl19"), Platform::Ps4, &s(&db), Some(&s(&roster)), &s(&meta), &s(&repo("defs"))).unwrap().project_id;
    assert!(!reg.summary(id).unwrap().roster_aligned);
    // The real pair disagrees on this player's club. A mirror refusal must roll
    // back the database operation and identify which file rejected the request.
    let before = serde_json::to_value(reg.player_view(id,3207).unwrap()).unwrap();
    let undo_before = reg.summary(id).unwrap().undo_depth;
    let refused = reg.release_to_fa(id,0,3207).err().expect("unaligned transaction must fail").to_string();
    assert!(refused.contains("attached roster refused"),"{refused}");
    assert_eq!(serde_json::to_value(reg.player_view(id,3207).unwrap()).unwrap(),before);
    assert_eq!(reg.summary(id).unwrap().undo_depth,undo_before);
    reg.set_display_roster(id, true).unwrap();
    let team = reg.team_view(id, 0).unwrap().unwrap();
    let abbr = team.all_fields.iter().find(|f| f.name == "abbrname").unwrap();
    reg.set_field(id, &abbr.studio_table, team.record, &abbr.studio_field, FieldValue::Text("QAR".into())).unwrap();
    let player = reg.list_players(id).unwrap().into_iter().find(|p| !p.is_goalie && p.team_id == Some(0) && reg.player_view(id,p.player_id).unwrap().unwrap().memberships.len() == 1).unwrap();
    let view = reg.player_view(id,player.player_id).unwrap().unwrap();
    let first = view.all_fields.iter().find(|f| f.name == "firstname").unwrap();
    reg.set_field(id,&first.studio_table,view.bio_record,&first.studio_field,FieldValue::Text("RosterQA".into())).unwrap();
    let rating = view.ratings.iter().find(|f| f.name == "c_aggression").unwrap();
    reg.set_field(id,&rating.studio_table,view.ratings_record.unwrap(),&rating.studio_field,FieldValue::Int(84)).unwrap();
    let stick = view.equipment.iter().find(|f| f.name == "stick").unwrap();
    let stick_value = if stick.decoded == Some(24) {18} else {24};
    reg.set_field(id,&stick.studio_table,view.equipment_record.unwrap(),&stick.studio_field,FieldValue::Int(stick_value)).unwrap();
    reg.release_to_fa(id,0,player.player_id).unwrap();
    reg.sign_from_fa(id,player.player_id,0).unwrap();
    reg.set_jersey_number(id,0,player.player_id,97).unwrap();
    reg.set_dressed(id,0,player.player_id,false).unwrap();
    // Snapshot the whole line view so scratches, numbers and slots survive serialization.
    let lines = serde_json::to_value(reg.lines_view(id,0).unwrap()).unwrap();
    reg.save_workspace(id).unwrap();
    assert_eq!(std::fs::read(&db).unwrap(),original_db,"roster mode must preserve the main database");
    let saved = std::fs::read(&roster).unwrap();
    assert_ne!(saved,original_roster);
    assert!(tdb_core::TdbFile::parse(&tdb_savedata::ps4_extract_roster(&saved).unwrap()).unwrap().header_crc_ok());
    // Independently open DATA without the database or any in-memory edits.
    let reopened = Registry::default();
    let rid = reopened.open_workspace_from_paths(Some("nhl19"),Platform::Ps4,&s(&roster),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    assert_eq!(reopened.team_view(rid,0).unwrap().unwrap().all_fields.iter().find(|f|f.name=="abbrname").unwrap().text.as_deref(),Some("QAR"));
    let saved_player = reopened.player_view(rid,player.player_id).unwrap().unwrap();
    assert_eq!(saved_player.all_fields.iter().find(|f|f.name=="firstname").unwrap().text.as_deref(),Some("RosterQA"));
    assert_eq!(saved_player.ratings.iter().find(|f|f.name=="c_aggression").unwrap().display,"84");
    assert_eq!(saved_player.equipment.iter().find(|f|f.name=="stick").unwrap().decoded,Some(stick_value));
    assert_eq!(serde_json::to_value(reopened.lines_view(rid,0).unwrap()).unwrap(),lines);
    assert_eq!(std::fs::read(anchor.join("databases/nhl_game.db")).unwrap(),original_db);
    assert_eq!(std::fs::read(anchor.join("savefiles/RosterCustom0/DATA")).unwrap(),original_roster);
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"roster_mode":true,"player":player.player_id,"main_unchanged":true,"roster_independently_reopened":true,"game_verified":false})).unwrap()).unwrap();
}
