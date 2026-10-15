//! Opt-in NHL19 QA; input images are read-only and outputs require a fresh folder.
use std::path::{Path, PathBuf};

use studio_api::Registry;
use studio_domain::{FieldValue, Platform};

fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
#[ignore = "writes only new copied database files under NHL19_PS4_QA_EDITS"]
fn nhl19_ps4_team_and_roster_edits_save_and_reopen() {
    let out=PathBuf::from(std::env::var("NHL19_PS4_QA_EDITS").expect("fresh QA edit folder"));
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let platform = Platform::Ps4;
    let anchor = repo("anchors/ps4/nhl19/databases");
    let db=out.join("nhl_game.db");
    std::fs::copy(anchor.join("nhl_game.db"),&db).unwrap();
    let roster = out.join("DATA");
    let original_roster = std::fs::read(repo("anchors/ps4/nhl19/savefiles/RosterCustom0/DATA")).unwrap();
    std::fs::write(&roster, &original_roster).unwrap();
    let meta=anchor.join("nhl_game-meta.xml");
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl19"),platform,&s(&db),Some(&s(&roster)),&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    assert!(!reg.summary(id).unwrap().roster_aligned,"this supplied pair is not safe for record-index mirroring");
    let dictionary=out.join("eng_us.db");
    std::fs::copy("H:/Emulators/ShadPS4/Games/CUSA11126/databigs/Extracted/nhl/data/kettle/loc/eng_us.db",&dictionary).unwrap();
    let parsed=tdb_loc::LocDictionary::parse(&std::fs::read(&dictionary).unwrap()).unwrap();
    let key=parsed.rows.iter().find(|r| r.key.is_some() && r.text.as_ref().is_some_and(|t| t.len()>6 && t.is_ascii())).unwrap().key.clone().unwrap();
    reg.attach_dictionary(id,&s(&dictionary)).unwrap();
    reg.dictionary_retext_set(id,&key,"QA").unwrap();
    // Localization currently holds one pending operation; persist it before another.
    reg.save_workspace(id).unwrap();
    reg.dictionary_retext_set(id,"NHL","QHL").unwrap();
    assert_eq!(reg.list_leagues(id).unwrap().leagues.iter().find(|l| l.code == 0).unwrap().display,"QHL");
    let team=reg.list_teams(id).unwrap().into_iter().find(|t| t.name.contains("Anaheim")).unwrap();
    reg.save_workspace(id).unwrap();
    // Team city is also an editable dictionary label, independent of arena city.
    let team_text = reg.team_text(id,team.team_id).unwrap().unwrap();
    let city_key = team_text.keys.iter().find(|k| k.family == "city").unwrap().key.clone();
    reg.dictionary_retext_set(id,&city_key,"QA").unwrap();
    let team_gear = reg.list_team_equipment(id,team.team_id).unwrap();
    let glove_row = team_gear.iter().find(|r| r.equipment_type_raw == 5 && r.variant_raw == 0)
        .or_else(|| team_gear.iter().find(|r| r.equipment_type_raw == 5)).unwrap();
    let glove_record = glove_row.record;
    let untouched_zones = serde_json::to_value(&glove_row.zones[1..]).unwrap();
    assert_eq!(glove_row.zones.len(),15); let packed = glove_row.zones[0].packed_field.as_ref().unwrap();
    reg.set_cells_int(id,"hHbv",glove_record,&[(packed.clone(),0x112b5b)]).unwrap();
    // Select transactions that are valid in both independently supplied files.
    let mirror_check=Registry::default();
    let mid=mirror_check.open_workspace_from_paths(Some("nhl19"),platform,&s(&roster),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let compatible = |player:u64,club:u64| {
        [reg.player_view(id,player).unwrap(),mirror_check.player_view(mid,player).unwrap()].iter().all(|v|
            v.as_ref().is_some_and(|v|v.memberships.len()==1 && v.memberships[0].team_id==club))
    };
    std::fs::write(out.join("membership-diagnostic.json"),serde_json::to_vec_pretty(&serde_json::json!({"player":3207,"database":reg.player_view(id,3207).unwrap().map(|v|v.memberships),"roster":mirror_check.player_view(mid,3207).unwrap().map(|v|v.memberships)})).unwrap()).unwrap();
    let player=reg.list_players(id).unwrap().into_iter().find(|p| !p.is_goalie && p.team_id==Some(team.team_id)
        && compatible(p.player_id,team.team_id)).unwrap();
    let gear = reg.player_view(id,player.player_id).unwrap().unwrap();
    assert_eq!(gear.equipment_table,"xmja");
    assert!(gear.equipment_record.is_some());
    let stick = gear.equipment.iter().find(|f| f.name == "stick").expect("bio gear exposed");
    let stick_value = if stick.decoded == Some(24) { 18 } else { 24 };
    reg.set_field(id,&stick.studio_table,gear.equipment_record.unwrap(),&stick.studio_field,FieldValue::Int(stick_value)).unwrap();
    let tape = gear.equipment.iter().find(|f| f.name == "stickbladetapecolor").unwrap();
    let tape_value = if tape.decoded == Some(0) { 1 } else { 0 };
    reg.set_field(id,&tape.studio_table,gear.equipment_record.unwrap(),&tape.studio_field,FieldValue::Int(tape_value)).unwrap();
    let first = gear.all_fields.iter().find(|f| f.name == "firstname").unwrap();
    reg.set_field(id,&first.studio_table,gear.bio_record,&first.studio_field,FieldValue::Text("QA".into())).unwrap();
    let rating = gear.ratings.iter().find(|f| f.name == "c_aggression").unwrap();
    // PS4 stores the displayed rating directly.
    reg.set_field(id,&rating.studio_table,gear.ratings_record.unwrap(),&rating.studio_field,FieldValue::Int(85)).unwrap();
    let view=reg.team_view(id,team.team_id).unwrap().unwrap();
    // NHL19 exposes a real arena foreign key, unlike the earlier-title harness.
    let venue = view.all_fields.iter().find(|f| f.name == "arenaid").unwrap();
    let alternate = reg.list_arenas(id).unwrap().into_iter()
        .find(|a| Some(a.arena_id as i64) != venue.decoded).unwrap();
    reg.set_field(id,&venue.studio_table,view.record,&venue.studio_field,FieldValue::Int(alternate.arena_id as i64)).unwrap();
    let primary: Vec<_> = [("feprimarycolor",0x112b5b)].iter()
        .map(|(n,v)| (view.all_fields.iter().find(|f| f.name == *n).unwrap().studio_field.clone(),*v)).collect();
    reg.set_cells_int(id,&venue.studio_table,view.record,&primary).unwrap();
    let name=view.identity.iter().find(|f| f.name=="abbrname").unwrap();
    reg.set_field(id,&name.studio_table,0,&name.studio_field,FieldValue::Text("QAD".into())).unwrap();
    assert_eq!(reg.team_view(id,team.team_id).unwrap().unwrap().all_fields.iter().find(|f| f.name == "abbrname").unwrap().text.as_deref(),Some("QAD"));
    // NHL19 has abbrname/nickname and a localized full name. Test the
    // actual fields and dictionary name without inventing a canonical alias.
    let nickname=view.all_fields.iter().find(|f|f.name=="nickname").unwrap();
    reg.set_field(id,&nickname.studio_table,view.record,&nickname.studio_field,FieldValue::Text("QA Ducks".into())).unwrap();
    reg.save_workspace(id).unwrap(); // persist the pending city rename first
    let full_name_key=team_text.keys.iter().find(|k|k.family=="full name").unwrap().key.clone();
    reg.dictionary_retext_set(id,&full_name_key,"QA Ducks").unwrap();
    let home = reg.list_arenas(id).unwrap().into_iter().find(|a| a.name.contains("Honda")).expect("Anaheim arena exists independently of team association");
    assert!(!home.name.is_empty() && !home.city.is_empty());
    let arena = reg.arena_view(id,home.arena_id).unwrap().unwrap();
    // Also qualify an independent venue's own biography.
    for (name, value) in [("arenaname", "QA Rink"), ("cityname", "QA City")] {
        let field = arena.all_fields.iter().find(|f| f.name == name).unwrap();
        assert_eq!(field.studio_table, "StEO", "NHL19 stores venue fields directly");
        reg.set_field(id,&field.studio_table,arena.record,&field.studio_field,FieldValue::Text(value.into())).unwrap();
    }
    assert!(reg.release_to_fa(id,team.team_id,player.player_id).unwrap().edits>0);
    assert!(!reg.team_roster_ids(id,team.team_id).unwrap().contains(&player.player_id));
    assert!(reg.sign_from_fa(id,player.player_id,team.team_id).unwrap().edits>0);
    assert!(reg.team_roster_ids(id,team.team_id).unwrap().contains(&player.player_id));
    reg.set_jersey_number(id,team.team_id,player.player_id,98).unwrap();
    reg.set_dressed(id,team.team_id,player.player_id,false).unwrap();
    let players = reg.list_players(id).unwrap();
    let outgoing = players.iter().find(|p| p.team_id == Some(team.team_id) && p.player_id != player.player_id && compatible(p.player_id,team.team_id)).unwrap();
    let incoming = players.iter().find(|p| p.team_id.is_some_and(|t| t != team.team_id && compatible(p.player_id,t))).unwrap();
    let other_team = incoming.team_id.unwrap();
    assert!(reg.trade_slots(id,team.team_id,outgoing.player_id,other_team,incoming.player_id).unwrap().edits > 0);
    assert!(reg.team_roster_ids(id,team.team_id).unwrap().contains(&incoming.player_id));
    assert!(reg.team_roster_ids(id,other_team).unwrap().contains(&outgoing.player_id));
    let lines = reg.lines_view(id,team.team_id).unwrap().unwrap();
    let slot = lines.slot_fields.iter().find(|f| f.name == "l1c").expect("first line centre slot");
    let replacement = lines.entries.iter().find(|e| e.player_id == Some(incoming.player_id)).unwrap();
    // Match the UI's clear-then-set operation, then verify exactly one owner after reopen.
    for previous in lines.entries.iter().filter(|e| e.slots.contains(&slot.studio_field)) {
        reg.set_field(id,"eGlu",previous.slot_record,&slot.studio_field,FieldValue::Int(0)).unwrap();
    }
    reg.set_field(id,"eGlu",replacement.slot_record,&slot.studio_field,FieldValue::Int(1)).unwrap();
    let lines_before=serde_json::to_value(reg.lines_view(id,team.team_id).unwrap()).unwrap();
    let career_player = players.iter().find(|p| p.last_name == "Crosby").unwrap();
    let career_before=reg.career_stats(id,career_player.player_id).unwrap();
    let section=career_before.sections.iter().find(|s|s.table=="tsSy").unwrap();
    assert_eq!(section.team_code_supported,Some(false));
    let rows_before=section.rows.len();
    // A requested team code must not be silently dropped on the newer schema.
    assert!(reg.add_career_season(id,"tsSy",career_player.player_id,2008,0,"QA").is_err());
    assert_eq!(reg.career_stats(id,career_player.player_id).unwrap().sections.iter().find(|s|s.table=="tsSy").unwrap().rows.len(),rows_before);
    reg.add_career_season(id,"tsSy",career_player.player_id,2008,0,"").unwrap();
    assert!(reg.summary(id).unwrap().dirty);
    reg.save_workspace(id).unwrap();
    let reopened=Registry::default();
    let rid=reopened.open_workspace_from_paths(Some("nhl19"),platform,&s(&db),Some(&s(&roster)),&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let stats = reopened.career_stats(rid,career_player.player_id).unwrap();
    let section=stats.sections.iter().find(|s|s.table=="tsSy").unwrap();
    assert_eq!(section.rows.len(),rows_before+1);
    assert!(section.rows.iter().any(|r| r.season_label == "2007-08" && r.team.is_empty()));
    assert_eq!(tdb_loc::LocDictionary::parse(&std::fs::read(&dictionary).unwrap()).unwrap().text("NHL"),Some("QHL"));
    reopened.attach_dictionary(rid,&s(&dictionary)).unwrap();
    assert_eq!(reopened.team_text(rid,team.team_id).unwrap().unwrap().keys.iter()
        .find(|k| k.family == "city").unwrap().text.as_deref(),Some("QA"));
    let gear = reopened.player_view(rid,player.player_id).unwrap().unwrap();
    assert_eq!(gear.equipment.iter().find(|f| f.name == "stick").unwrap().decoded,Some(stick_value));
    assert_eq!(gear.equipment.iter().find(|f| f.name == "stickbladetapecolor").unwrap().decoded,Some(tape_value));
    assert_eq!(gear.all_fields.iter().find(|f| f.name == "firstname").unwrap().text.as_deref(),Some("QA"));
    assert_eq!(gear.ratings.iter().find(|f| f.name == "c_aggression").unwrap().display,"85");
    assert_eq!(reopened.team_view(rid,team.team_id).unwrap().unwrap().all_fields.iter().find(|f| f.name == "abbrname").unwrap().text.as_deref(),Some("QAD"));
    assert_eq!(reopened.team_text(rid,team.team_id).unwrap().unwrap().keys.iter().find(|k|k.family=="full name").unwrap().text.as_deref(),Some("QA Ducks"));
    assert_eq!(reopened.team_view(rid,team.team_id).unwrap().unwrap().all_fields.iter().find(|f|f.name=="nickname").unwrap().text.as_deref(),Some("QA Ducks"));
    let saved_team = reopened.team_view(rid,team.team_id).unwrap().unwrap();
    assert_eq!(saved_team.arena.as_ref().unwrap().arena_id,alternate.arena_id);
    for (field,value) in &primary {
        assert_eq!(saved_team.all_fields.iter().find(|f| &f.studio_field == field).unwrap().decoded,Some(*value));
    }
    let saved_gear = reopened.list_team_equipment(rid,team.team_id).unwrap();
    let saved_glove = saved_gear.iter().find(|r| r.record == glove_record).unwrap();
    assert_eq!((saved_glove.zones[0].red,saved_glove.zones[0].green,saved_glove.zones[0].blue),(17,43,91));
    assert_eq!(serde_json::to_value(&saved_glove.zones[1..]).unwrap(),untouched_zones);
    let home = reopened.list_arenas(rid).unwrap().into_iter().find(|a| a.arena_id == home.arena_id).unwrap();
    assert_eq!((home.name.as_str(),home.city.as_str()),("QA Rink","QA City"));
    assert_eq!(serde_json::to_value(reopened.lines_view(rid,team.team_id).unwrap()).unwrap(),lines_before);
    assert!(reopened.team_roster_ids(rid,team.team_id).unwrap().contains(&player.player_id));
    assert!(reopened.team_roster_ids(rid,team.team_id).unwrap().contains(&incoming.player_id));
    assert!(reopened.team_roster_ids(rid,other_team).unwrap().contains(&outgoing.player_id));
    let re_lines = reopened.lines_view(rid,team.team_id).unwrap().unwrap();
    let owners: Vec<_> = re_lines.entries.iter().filter(|e| e.slots.contains(&slot.studio_field)).collect();
    assert_eq!(owners.len(),1);
    assert_eq!(owners[0].player_id,Some(incoming.player_id));
    assert!(tdb_core::TdbFile::parse(&std::fs::read(&db).unwrap()).unwrap().header_crc_ok());
    let saved_roster = std::fs::read(&roster).unwrap();
    assert_ne!(saved_roster, original_roster);
    let inner = tdb_savedata::ps4_extract_roster(&saved_roster).unwrap();
    assert!(tdb_core::TdbFile::parse(&inner).unwrap().header_crc_ok());
    // Independently verify entity transactions reached DATA while generic field edits did not.
    let standalone = Registry::default();
    let sid = standalone.open_workspace_from_paths(Some("nhl19"), platform, &s(&roster), None, &s(&meta), &s(&repo("defs"))).unwrap().project_id;
    // This pair is unaligned: generic record edits must stay in the main DB.
    assert_eq!(standalone.team_view(sid,team.team_id).unwrap().unwrap().all_fields.iter().find(|f| f.name == "abbrname").unwrap().text,
        mirror_check.team_view(mid,team.team_id).unwrap().unwrap().all_fields.iter().find(|f|f.name=="abbrname").unwrap().text);
    assert!(standalone.team_roster_ids(sid,team.team_id).unwrap().contains(&incoming.player_id));
    assert_eq!(standalone.player_view(sid,player.player_id).unwrap().unwrap().ratings.iter().find(|f| f.name == "c_aggression").unwrap().display,
        mirror_check.player_view(mid,player.player_id).unwrap().unwrap().ratings.iter().find(|f|f.name=="c_aggression").unwrap().display);
    assert_eq!(std::fs::read(repo("anchors/ps4/nhl19/savefiles/RosterCustom0/DATA")).unwrap(),original_roster);
    assert_eq!(tdb_loc::LocDictionary::parse(&std::fs::read(&dictionary).unwrap()).unwrap().text(&key),Some("QA"));
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"team":team.team_id,"player":player.player_id,"database_name":"QA Ducks","generic_field_mirroring":false,"compatible_entity_mirroring":true,"release_and_sign":true,"scratch_and_jersey_saved":true,"equipment_stick_saved":stick_value,"lines":lines_before,"game_verified":false})).unwrap()).unwrap();
}
