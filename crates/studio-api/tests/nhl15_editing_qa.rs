//! Opt-in NHL15 QA; input images are read-only and outputs require a fresh folder.
use std::path::{Path, PathBuf};

use studio_api::Registry;
use studio_domain::{FieldValue, Platform};

fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
#[ignore = "writes only new copied database files under NHL15_QA_EDITS"]
fn nhl15_team_and_roster_edits_save_and_reopen() {
    let out=PathBuf::from(std::env::var("NHL15_QA_EDITS").expect("fresh QA edit folder"));
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let platform = if std::env::var("NHL15_QA_PLATFORM").as_deref() == Ok("ps3") { Platform::Ps3 } else { Platform::Xbox };
    let anchor = repo(if platform == Platform::Ps3 { "anchors/ps3/nhl15/databases" } else { "anchors/xbox360/nhl15/databases" });
    let db=out.join("nhlng.db");
    std::fs::copy(anchor.join("nhlng.db"),&db).unwrap();
    let meta=anchor.join("nhlng-meta.xml");
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl15"),platform,&s(&db),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let dictionary=out.join("nhl_eng_us.db");
    std::fs::copy(anchor.join("nhl_eng_us.db"),&dictionary).unwrap();
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
    let (blue, green, red) = studio_domain::EQUIPMENT_ZONE_FIELDS[0];
    reg.set_cells_int(id,"hHbv",glove_record,&[(red.into(),17),(green.into(),43),(blue.into(),91)]).unwrap();
    let player=reg.list_players(id).unwrap().into_iter().find(|p| p.team_id==Some(team.team_id)).unwrap();
    let gear = reg.player_view(id,player.player_id).unwrap().unwrap();
    assert_eq!(gear.equipment_table,"xmja");
    assert!(gear.equipment_record.is_some());
    let stick = gear.equipment.iter().find(|f| f.name == "stick").expect("bio gear exposed");
    let stick_value = if stick.decoded == Some(24) { 18 } else { 24 };
    reg.set_field(id,&stick.studio_table,gear.equipment_record.unwrap(),&stick.studio_field,FieldValue::Int(stick_value)).unwrap();
    let tape = gear.equipment.iter().find(|f| f.name == "sticktapecolor").unwrap();
    let tape_value = if tape.decoded == Some(0) { 1 } else { 0 };
    reg.set_field(id,&tape.studio_table,gear.equipment_record.unwrap(),&tape.studio_field,FieldValue::Int(tape_value)).unwrap();
    let first = gear.all_fields.iter().find(|f| f.name == "firstname").unwrap();
    reg.set_field(id,&first.studio_table,gear.bio_record,&first.studio_field,FieldValue::Text("QA".into())).unwrap();
    let rating = gear.ratings.iter().find(|f| f.name == "c_aggression").unwrap();
    // NHL15 uses the later six-bit +36 scale: stored 49 displays as 85.
    reg.set_field(id,&rating.studio_table,gear.ratings_record.unwrap(),&rating.studio_field,FieldValue::Int(49)).unwrap();
    let view=reg.team_view(id,team.team_id).unwrap().unwrap();
    // NHL15 exposes a real arena foreign key, unlike the earlier-title harness.
    let venue = view.all_fields.iter().find(|f| f.name == "arenaid").unwrap();
    let alternate = reg.list_arenas(id).unwrap().into_iter()
        .find(|a| Some(a.arena_id as i64) != venue.decoded).unwrap();
    reg.set_field(id,&venue.studio_table,view.record,&venue.studio_field,FieldValue::Int(alternate.arena_id as i64)).unwrap();
    let primary: Vec<_> = [("primarycolor_r",17),("primarycolor_g",43),("primarycolor_b",91)].iter()
        .map(|(n,v)| (view.all_fields.iter().find(|f| f.name == *n).unwrap().studio_field.clone(),*v)).collect();
    reg.set_cells_int(id,&venue.studio_table,view.record,&primary).unwrap();
    let name=view.identity.iter().find(|f| f.name=="fullname").unwrap();
    reg.set_field(id,&name.studio_table,0,&name.studio_field,FieldValue::Text("QA Ducks".into())).unwrap();
    assert_eq!(reg.team_view(id,team.team_id).unwrap().unwrap().name,"QA Ducks");
    let home = reg.list_arenas(id).unwrap().into_iter().find(|a| a.name.contains("Honda")).expect("Anaheim arena exists independently of team association");
    assert!(!home.name.is_empty() && !home.city.is_empty());
    let arena = reg.arena_view(id,home.arena_id).unwrap().unwrap();
    // Also qualify an independent venue's own biography.
    for (name, value) in [("arenaname", "QA Rink"), ("cityname", "QA City")] {
        let field = arena.all_fields.iter().find(|f| f.name == name).unwrap();
        assert_eq!(field.studio_table, "StEO", "NHL15 stores venue fields directly");
        reg.set_field(id,&field.studio_table,arena.record,&field.studio_field,FieldValue::Text(value.into())).unwrap();
    }
    assert!(reg.release_to_fa(id,team.team_id,player.player_id).unwrap().edits>0);
    assert!(!reg.team_roster_ids(id,team.team_id).unwrap().contains(&player.player_id));
    assert!(reg.sign_from_fa(id,player.player_id,team.team_id).unwrap().edits>0);
    assert!(reg.team_roster_ids(id,team.team_id).unwrap().contains(&player.player_id));
    reg.set_jersey_number(id,team.team_id,player.player_id,98).unwrap();
    reg.set_dressed(id,team.team_id,player.player_id,false).unwrap();
    let players = reg.list_players(id).unwrap();
    let outgoing = players.iter().find(|p| p.team_id == Some(team.team_id) && p.player_id != player.player_id).unwrap();
    let incoming = players.iter().find(|p| p.team_id.is_some_and(|t| t != team.team_id)).unwrap();
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
    reg.add_career_season(id,"tsSy",career_player.player_id,2008,0,"QA").unwrap();
    assert!(reg.summary(id).unwrap().dirty);
    reg.save_workspace(id).unwrap();
    let reopened=Registry::default();
    let rid=reopened.open_workspace_from_paths(Some("nhl15"),platform,&s(&db),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let stats = reopened.career_stats(rid,career_player.player_id).unwrap();
    assert!(stats.sections.iter().flat_map(|s| &s.rows).any(|r| r.season_label == "2007-08" && r.team == "QA"));
    assert_eq!(tdb_loc::LocDictionary::parse(&std::fs::read(&dictionary).unwrap()).unwrap().text("NHL"),Some("QHL"));
    reopened.attach_dictionary(rid,&s(&dictionary)).unwrap();
    assert_eq!(reopened.team_text(rid,team.team_id).unwrap().unwrap().keys.iter()
        .find(|k| k.family == "city").unwrap().text.as_deref(),Some("QA"));
    let gear = reopened.player_view(rid,player.player_id).unwrap().unwrap();
    assert_eq!(gear.equipment.iter().find(|f| f.name == "stick").unwrap().decoded,Some(stick_value));
    assert_eq!(gear.equipment.iter().find(|f| f.name == "sticktapecolor").unwrap().decoded,Some(tape_value));
    assert_eq!(gear.all_fields.iter().find(|f| f.name == "firstname").unwrap().text.as_deref(),Some("QA"));
    assert_eq!(gear.ratings.iter().find(|f| f.name == "c_aggression").unwrap().display,"85");
    assert_eq!(reopened.team_view(rid,team.team_id).unwrap().unwrap().name,"QA Ducks");
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
    assert_eq!(tdb_loc::LocDictionary::parse(&std::fs::read(&dictionary).unwrap()).unwrap().text(&key),Some("QA"));
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"team":team.team_id,"player":player.player_id,"name":"QA Ducks","release_and_sign":true,"scratch_and_jersey_saved":true,"equipment_stick_saved":stick_value,"lines":lines_before,"game_verified":false})).unwrap()).unwrap();
}














