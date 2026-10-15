//! Opt-in NHL08 QA; input images are read-only and outputs require a fresh folder.
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use studio_api::Registry;
use studio_domain::{discovery, extract_service, FieldValue, Platform};

fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }

#[test]
#[ignore = "requires decrypted NHL08_QA_PS3_ISO and fresh NHL08_QA_PS3_OUT; preserves image"]
fn ps3_iso_dump_and_relocated_files_open() {
    let iso=PathBuf::from(std::env::var("NHL08_QA_PS3_ISO").unwrap());
    let out=PathBuf::from(std::env::var("NHL08_QA_PS3_OUT").unwrap());
    assert!(!out.exists()); std::fs::create_dir(&out).unwrap();
    let disc=out.join("relocated PS3 disc");
    // Exercise the same asynchronous entry point as desktop onboarding.
    let jobs = Registry::default();
    let mut receipt = jobs.disc_extract_start(&s(&iso), &s(&disc)).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(600);
    while matches!(receipt.state.as_str(), "running" | "cancelling") {
        assert!(std::time::Instant::now() < deadline, "disc copy timed out");
        std::thread::sleep(std::time::Duration::from_millis(100));
        receipt = jobs.extract_status(receipt.job).unwrap();
    }
    assert_eq!(receipt.state, "done", "{:?}", receipt.error);
    let game=disc.join("PS3_GAME/USRDIR");
    assert!(game.join("EBOOT.BIN").is_file());
    let db=game.join("db/nhlng.db");
    let original=std::fs::read(&db).unwrap();
    assert!(tdb_core::TdbFile::parse(&original).unwrap().header_crc_ok(),"requires decrypted game data");
    eprintln!("PS3 ISO copied: {} files / {} bytes; database readable",receipt.files_written,receipt.bytes_written);
    let pre=extract_service::preflight(&game,Platform::Ps3);
    assert_ne!(pre.state,"not_applicable"); assert!(pre.members_to_write>0);
    let extracted=extract_service::extract(&game,Platform::Ps3,&AtomicBool::new(false),&mut |_|{}).unwrap();
    let found=discovery::detect_install("nhl08",Platform::Ps3,&disc,None);
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl08"),Platform::Ps3,
        found.db_paths.nhlng.as_deref().unwrap(),None,
        &s(&repo("anchors/ps3/nhl08/databases/nhlng-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id,found.db_paths.localization.as_deref().unwrap()).unwrap();
    let players=reg.list_players(id).unwrap(); let teams=reg.list_teams(id).unwrap();
    assert!(players.len()>2000 && teams.len()>100);
    assert_eq!(std::fs::read(&db).unwrap(),original);
    let after=extract_service::preflight(&game,Platform::Ps3);
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({
        "iso":iso,"disc_copy":receipt,"extraction":format!("{extracted:?}"),
        "players":players.len(),"teams":teams.len(),"state":after.state,
        "source_db_preserved":true,"game_verified":false
    })).unwrap()).unwrap();
}

#[test]
#[ignore = "read-only observations of NHL08_QA_ASSETS, writes a new NHL08_QA_OBSERVATIONS receipt"]
fn xbox_asset_and_editing_surfaces() {
    let root = PathBuf::from(std::env::var("NHL08_QA_ASSETS").unwrap());
    let out = PathBuf::from(std::env::var("NHL08_QA_OBSERVATIONS").unwrap());
    assert!(!out.exists());
    let ps3 = std::env::var("NHL08_QA_PLATFORM").as_deref() == Ok("ps3");
    let platform = if ps3 { Platform::Ps3 } else { Platform::Xbox };
    let meta = repo(if ps3 { "anchors/ps3/nhl08/databases/nhlng-meta.xml" } else { "anchors/xbox360/nhl08/databases/nhlng-meta.xml" });
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl08"),platform,
        &s(&root.join("db/nhlng.db")),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let found = discovery::detect_install("nhl08",platform,&root,None);
    reg.attach_dictionary(id,found.db_paths.localization.as_ref().unwrap()).unwrap();
    let args = studio_api::AssetRootsArgsDto { install_dir:s(&root), platform:Some(platform.as_str().into()), rendering_override:None, fe_override:None };
    let teams = reg.list_teams(id).unwrap();
    let team = teams.iter().find(|t| t.name.contains("Anaheim")).unwrap();
    let player = reg.list_players(id).unwrap().into_iter().find(|p| p.team_id == Some(team.team_id)).unwrap();
    fn obs<T:serde::Serialize,E:std::fmt::Display>(r:Result<T,E>)->serde_json::Value {
        match r {Ok(v)=>serde_json::json!({"value":v}),Err(e)=>serde_json::json!({"error":e.to_string()})}
    }
    let leagues = reg.list_leagues(id).unwrap();
    let logos:Vec<_> = leagues.leagues.iter().filter(|l| l.code>=0).map(|l|obs(reg.league_logo(id,l.code as u64,Some(&s(&root))))).collect();
    let arenas=reg.list_arenas(id).unwrap();
    let arena_art:Vec<_>=arenas.iter().take(2).map(|a|obs(reg.arena_assets(id,&args,a.arena_id))).collect();
    let outfits = reg.team_outfits(id,&args,team.team_id).unwrap();
    assert!(!outfits.is_empty(), "extracted native uniforms must be discoverable");
    assert!(outfits.iter().any(|t| t.piece == "jersey" && t.label.contains("color map")), "jersey color must not be hidden behind the number font");
    assert!(outfits.iter().any(|t| t.rel_path.replace('\\', "/") == if ps3 { "rendering/jersey/texlib_0_0_0.rpsgl" } else { "rendering/jersey/texlib_0_0_0.rx2" }
        && t.piece == "jersey" && t.label.contains("color map") && t.texture_index == if ps3 { 3 } else { 2 }));
    if ps3 {
        for index in 0..6 {
            let (_, decoded) = studio_domain::decode_texture_file(&root.join("rendering/jersey/texlib_0_0_0.rpsgl"),Some(index)).unwrap();
            let preview = out.with_extension(format!("slot-{index}.png"));
            assert!(!preview.exists());
            std::fs::write(preview, decoded.to_png().unwrap()).unwrap();
        }
    }
    let receipt=serde_json::json!({
        "team":obs(reg.team_view(id,team.team_id)),"player":obs(reg.player_view(id,player.player_id)),
        "leagues":leagues,"league_logos":logos,"arenas":arenas,"arena_art":arena_art,
        "team_logos":obs(reg.team_logo_set(id,team.team_id,Some(&s(&root)))),
        "kits":obs(reg.team_kits(id,&args,team.team_id)),"outfits":{"value":outfits},
        "lines":obs(reg.lines_view(id,team.team_id)),"equipment":obs(reg.player_equipment_files(id,player.player_id,&args,&repo("defs"))),
        "player_assets":obs(reg.list_player_assets(id,&args)),"game_verified":false
    });
    std::fs::write(out,serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}

#[test]
#[ignore = "requires NHL08_QA_ISO and a fresh NHL08_QA_OUT; copies the real Xbox360 disc"]
fn xbox_iso_dump_and_relocated_files_open_and_preserve_staged_edits() {
    let iso = PathBuf::from(std::env::var("NHL08_QA_ISO").expect("read-only ISO"));
    let out = PathBuf::from(std::env::var("NHL08_QA_OUT").expect("fresh output"));
    assert!(!out.exists(), "never overwrite prior QA evidence");
    std::fs::create_dir_all(&out).unwrap();
    let meta = repo("anchors/xbox360/nhl08/databases/nhlng-meta.xml");
    let defs = repo("defs");
    let located = studio_domain::xiso_source::locate_databases(&iso).unwrap();
    let original = studio_domain::xiso_source::read_located(&iso, located.nhlng.as_ref().unwrap()).unwrap();
    let stage = out.join("arbitrary staging location");
    let reg = Registry::default();
    let (staged, project) = reg.iso_open(&s(&iso), "nhl08", "xbox360", &s(&meta), &s(&stage), &s(&defs)).unwrap();
    assert_eq!(std::fs::read(&staged.db_path).unwrap(), original);
    let players = reg.list_players(project.project_id).unwrap();
    let teams = reg.list_teams(project.project_id).unwrap();
    assert!(!players.is_empty() && !teams.is_empty());
    // Resolve the editable field from the loaded schema, not a later-title column guess.
    let player = reg.player_view(project.project_id, players[0].player_id).unwrap().unwrap();
    let first = player.all_fields.iter().find(|f| f.name == "firstname").unwrap();
    reg.set_field(project.project_id, &first.studio_table, player.bio_record, &first.studio_field, FieldValue::Text("QA".into())).unwrap();
    reg.save_workspace(project.project_id).unwrap();
    let again = Registry::default();
    let (_, reopened) = again.iso_open(&s(&iso), "nhl08", "xbox360", &s(&meta), &s(&stage), &s(&defs)).unwrap();
    assert_eq!(again.list_players(reopened.project_id).unwrap().iter().find(|p| p.player_id == players[0].player_id).unwrap().first_name, "QA");
    assert!(tdb_core::TdbFile::parse(&std::fs::read(&staged.db_path).unwrap()).unwrap().header_crc_ok());
    assert_eq!(studio_domain::xiso_source::read_located(&iso, located.nhlng.as_ref().unwrap()).unwrap(), original);
    eprintln!("NHL08 ISO edit/reopen passed: {} players, {} teams", players.len(), teams.len());

    // A materialized disc is the dumped-disc input; extract only this new copy.
    let dump = out.join("relocated disc dump");
    let copied = studio_domain::disc_extract::extract(&iso, &dump, &AtomicBool::new(false), &mut |_,_| {}).unwrap();
    eprintln!("Copied {} disc files / {} bytes", copied.members, copied.bytes);
    let extracted = extract_service::extract(&dump, Platform::Xbox, &AtomicBool::new(false), &mut |_| {}).unwrap();
    let found = discovery::detect_install("nhl08", Platform::Xbox, &dump, None);
    let db = found.db_paths.nhlng.as_ref().expect("extracted DB found");
    let schema = found.db_paths.meta_xml.as_deref().map(str::to_string).unwrap_or_else(|| s(&meta));
    let loose = Registry::default();
    let opened = loose.open_workspace_from_paths(Some("nhl08"), Platform::Xbox, db, None, &schema, &s(&defs)).unwrap();
    assert_eq!(loose.list_players(opened.project_id).unwrap().len(), players.len());
    assert_eq!(loose.list_teams(opened.project_id).unwrap().len(), teams.len());
    let dict = found.db_paths.localization.as_ref().expect("dictionary found");
    loose.attach_dictionary(opened.project_id, dict).unwrap();
    let rescan = extract_service::preflight(&dump, Platform::Xbox);
    std::fs::write(out.join("source-routes.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "iso":iso, "staging":stage, "dump":dump, "copy":copied,
        "players":players.len(), "teams":teams.len(), "metadata":schema,
        "extraction":format!("{extracted:?}"), "rescan_state":rescan.state,
        "rescan_archives":rescan.archives.len(), "remaining_members":rescan.members_to_write,
        "staged_edit_reopened":true, "source_db_unchanged":true, "game_verified":false
    })).unwrap()).unwrap();
    eprintln!("NHL08 source routes complete: {} archives, {} remaining", rescan.archives.len(), rescan.members_to_write);
}
#[test]
#[ignore = "writes only new copied database files under NHL08_QA_EDITS"]
fn nhl08_team_and_roster_edits_save_and_reopen() {
    let out=PathBuf::from(std::env::var("NHL08_QA_EDITS").expect("fresh QA edit folder"));
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let platform = if std::env::var("NHL08_QA_PLATFORM").as_deref() == Ok("ps3") { Platform::Ps3 } else { Platform::Xbox };
    let anchor = repo(if platform == Platform::Ps3 { "anchors/ps3/nhl08/databases" } else { "anchors/xbox360/nhl08/databases" });
    let db=out.join("nhlng.db");
    std::fs::copy(anchor.join("nhlng.db"),&db).unwrap();
    let meta=anchor.join("nhlng-meta.xml");
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl08"),platform,&s(&db),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
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
    let player=reg.list_players(id).unwrap().into_iter().find(|p| p.team_id==Some(team.team_id)).unwrap();
    let gear = reg.player_view(id,player.player_id).unwrap().unwrap();
    assert_eq!(gear.equipment_table,"xmja");
    assert!(gear.equipment_record.is_some());
    let stick = gear.equipment.iter().find(|f| f.name == "stick").expect("bio gear exposed");
    let stick_value = if stick.decoded == Some(1) { 2 } else { 1 };
    reg.set_field(id,&stick.studio_table,gear.equipment_record.unwrap(),&stick.studio_field,FieldValue::Int(stick_value)).unwrap();
    let tape = gear.equipment.iter().find(|f| f.name == "sticktapecolor").unwrap();
    let tape_value = if tape.decoded == Some(0) { 1 } else { 0 };
    reg.set_field(id,&tape.studio_table,gear.equipment_record.unwrap(),&tape.studio_field,FieldValue::Int(tape_value)).unwrap();
    let first = gear.all_fields.iter().find(|f| f.name == "firstname").unwrap();
    reg.set_field(id,&first.studio_table,gear.bio_record,&first.studio_field,FieldValue::Text("QA".into())).unwrap();
    let rating = gear.ratings.iter().find(|f| f.name == "p_aggression").unwrap();
    // Field editing consumes the stored value; the UI adds 50 for this schema.
    reg.set_field(id,&rating.studio_table,gear.ratings_record.unwrap(),&rating.studio_field,FieldValue::Int(35)).unwrap();
    let view=reg.team_view(id,team.team_id).unwrap().unwrap();
    let name=view.identity.iter().find(|f| f.name=="fullname").unwrap();
    reg.set_field(id,&name.studio_table,0,&name.studio_field,FieldValue::Text("QA Ducks".into())).unwrap();
    assert_eq!(reg.team_view(id,team.team_id).unwrap().unwrap().name,"QA Ducks");
    let home = reg.list_arenas(id).unwrap().into_iter().find(|a| a.name.contains("Honda")).expect("Anaheim arena exists independently of team association");
    assert!(!home.name.is_empty() && !home.city.is_empty());
    let arena = reg.arena_view(id,home.arena_id).unwrap().unwrap();
    // NHL08 team-to-arena association is not yet verified; edit the explicitly selected venue.
    for (name, value) in [("arenaname", "QA Rink"), ("cityname", "QA City")] {
        let field = arena.all_fields.iter().find(|f| f.name == name).unwrap();
        assert_eq!(field.studio_table, "StEO", "NHL08 stores venue fields directly");
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
    let rid=reopened.open_workspace_from_paths(Some("nhl08"),platform,&s(&db),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let stats = reopened.career_stats(rid,career_player.player_id).unwrap();
    assert!(stats.sections.iter().flat_map(|s| &s.rows).any(|r| r.season_label == "2007-08" && r.team == "QA"));
    assert_eq!(tdb_loc::LocDictionary::parse(&std::fs::read(&dictionary).unwrap()).unwrap().text("NHL"),Some("QHL"));
    let gear = reopened.player_view(rid,player.player_id).unwrap().unwrap();
    assert_eq!(gear.equipment.iter().find(|f| f.name == "stick").unwrap().decoded,Some(stick_value));
    assert_eq!(gear.equipment.iter().find(|f| f.name == "sticktapecolor").unwrap().decoded,Some(tape_value));
    assert_eq!(gear.all_fields.iter().find(|f| f.name == "firstname").unwrap().text.as_deref(),Some("QA"));
    assert_eq!(gear.ratings.iter().find(|f| f.name == "p_aggression").unwrap().display,"85");
    assert_eq!(reopened.team_view(rid,team.team_id).unwrap().unwrap().name,"QA Ducks");
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


#[test]
#[ignore = "copies native textures into a fresh NHL08_QA_TEXTURES directory"]
fn nhl08_native_texture_replace_roundtrip() {
    let source = PathBuf::from(std::env::var("NHL08_QA_ASSETS").expect("copied extracted disc"));
    let out = PathBuf::from(std::env::var("NHL08_QA_TEXTURES").expect("fresh texture output"));
    assert!(!out.exists(), "preserve earlier evidence");
    std::fs::create_dir_all(&out).unwrap();
    let reg = Registry::default();
    let mut results = Vec::new();
    for rel in ["fe/ion/artassets/teamlogos/t0.big", "fe/ion/artassets/playerheads/h1.big", "fe/ion/artassets/leaguelogos/l0.big",
        "rendering/jersey/texlib_0_0_0.rx2",
        "rendering/stick/texlib_0.rx2", "rendering/ice/texlib.rx2"] {
        // PS3 uses the corresponding native RenderWare PSGL package suffix.
        let native = if std::env::var("NHL08_QA_PLATFORM").as_deref() == Ok("ps3") {
            rel.replace(".rx2", ".rpsgl")
        } else { rel.to_string() };
        // Visually verified color slots: Xbox has two preceding font maps; PS3 has three.
        let index = if rel == "rendering/jersey/texlib_0_0_0.rx2" {
            if std::env::var("NHL08_QA_PLATFORM").as_deref() == Ok("ps3") { 3 } else { 2 }
        } else { 0 };
        let rel = native.as_str();
        let original = std::fs::read(source.join(rel)).unwrap();
        let target = out.join(rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, &original).unwrap();
        // Exercise the production preview/plan/write path, not just the inner codec.
        let result = (|| -> Result<serde_json::Value, String> {
            let before = reg.texture_preview(&s(&out), rel, 0, Some(index)).map_err(|e| e.to_string())?;
            let rgba = [240, 10, 180, 255].repeat((before.width * before.height) as usize);
            let png = studio_domain::DecodedImage { width:before.width, height:before.height, rgba }.to_png().unwrap();
            let input = target.with_extension("qa.png");
            std::fs::write(&input, png).unwrap();
            let plan = reg.texture_replace_plan(&s(&out), rel, &s(&input), Some(index)).map_err(|e| e.to_string())?;
            if !plan.allowed { return Err(format!("review refused: {:?}", plan.refusal)); }
            let written = reg.import_texture(&s(&out), rel, &s(&input), true, Some(index), plan.review_id).map_err(|e| e.to_string())?;
            let after = Registry::default().texture_preview(&s(&out), rel, 0, Some(index)).map_err(|e| e.to_string())?;
            assert_eq!((before.width, before.height), (after.width, after.height));
            assert_ne!(before.data_url, after.data_url, "decoded replacement must differ");
            assert_ne!(std::fs::read(&target).unwrap(), original);
            assert_eq!(std::fs::read(format!("{}.bak", s(&target))).unwrap(), original);
            Ok(serde_json::json!({"width":after.width,"height":after.height,"container":after.container,"write":written,"decoded_changed":true}))
        })();
        assert_eq!(std::fs::read(source.join(rel)).unwrap(), original, "source stays untouched");
        results.push(match result {
            Ok(value) => serde_json::json!({"path":rel,"texture_index":index,"roundtrip":value}),
            Err(error) => serde_json::json!({"path":rel,"blocker":error}),
        });
    }
    std::fs::write(out.join("receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({"textures":results,"game_verified":false})).unwrap()).unwrap();
    assert!(results.iter().all(|r| r.get("roundtrip").is_some()), "native texture replacement regression: {results:?}");
}






