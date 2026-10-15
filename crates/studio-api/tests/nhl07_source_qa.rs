//! Opt-in real-disc QA. The source ISO is read-only; every write is in NHL07_QA_OUT.
//! Run with NHL_STUDIO_XISO and NHL07_QA_OUT set, using --ignored --nocapture.
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use studio_api::Registry;
use studio_domain::{discovery, extract_service, FieldValue, Platform};

fn repo(path: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(path)
}
fn s(path: &Path) -> String { path.to_string_lossy().into_owned() }

#[test]
#[ignore = "read-only preflight of the fully extracted NHL07_QA_ASSETS copy"]
fn nhl07_extracted_disc_preflight_keeps_outer_container_scope() {
    let root = PathBuf::from(std::env::var("NHL07_QA_ASSETS").expect("copied extracted disc"));
    let p = extract_service::preflight(&root, Platform::Xbox);
    assert_eq!(p.archives.len(), 11);
    assert_eq!(p.archives.iter().filter(|a| a.parse_error.is_some()).count(), 2);
    assert_eq!(p.members_total, 15_607);
    assert_eq!(p.members_to_write, 0);
    assert!(p.collisions.is_empty());
    println!("{}", serde_json::json!({"root":root,"archives":p.archives.len(),"unreadable":2,
        "members_total":p.members_total,"members_present":p.members_present,"members_to_write":p.members_to_write,
        "state":p.state,"reason":p.reason}));
}

#[test]
#[ignore = "extracts a real NHL07 image into fresh NHL07_QA_DISC_OUT"]
fn nhl07_production_disc_extraction() {
    let iso = PathBuf::from(std::env::var("NHL_STUDIO_XISO").expect("source ISO"));
    let out = PathBuf::from(std::env::var("NHL07_QA_DISC_OUT").expect("fresh destination"));
    let receipt = studio_domain::disc_extract::extract(&iso,&out,&AtomicBool::new(false),&mut |_,_|{}).unwrap();
    let disc = tdb_xiso::Xiso::open(&iso).unwrap();
    let files: Vec<_> = disc.members().iter().filter(|m| !m.is_dir()).collect();
    assert_eq!(disc.members().len(),311); // 293 files plus 18 directory entries.
    assert_eq!(receipt.members,files.len());
    assert_eq!(receipt.bytes,files.iter().map(|m| m.size as u64).sum::<u64>());
    for member in files {
        assert_eq!(std::fs::metadata(out.join(&member.path)).unwrap().len(),member.size as u64);
    }
    assert!(out.join("default.xex").is_file());
    let source = studio_domain::xiso_source::locate_databases(&iso).unwrap();
    let expected = studio_domain::xiso_source::read_located(&iso,source.nhlng.as_ref().unwrap()).unwrap();
    assert_eq!(std::fs::read(out.join("db/NHL07NG.db")).unwrap(),expected);
    let found = discovery::detect_install("nhl07",Platform::Xbox,&out,None);
    assert!(found.db_paths.nhlng.is_some());
    // Store the receipt beside the disc, not inside its game namespace.
    std::fs::write(out.with_extension("receipt.json"),serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}

#[test]
#[ignore = "copies native textures into a fresh NHL07_QA_TEXTURES directory"]
fn nhl07_native_texture_replace_roundtrip() {
    let source = PathBuf::from(std::env::var("NHL07_QA_ASSETS").expect("copied extracted disc"));
    let out = PathBuf::from(std::env::var("NHL07_QA_TEXTURES").expect("fresh texture output"));
    assert!(!out.exists(), "preserve earlier evidence");
    std::fs::create_dir_all(&out).unwrap();
    let reg = Registry::default();
    let mut results = Vec::new();
    for rel in ["fe/ion/artassets/teamlogos/t0.big", "fe/ion/artassets/playerheads/p1.big",
        "rendering/jersey/jersey_0/jersey_0_0_0_cm.rx2",
        "rendering/stick/common/sticktape_0_cm.rx2", "rendering/ice/common/texture.rx2"] {
        let original = std::fs::read(source.join(rel)).unwrap();
        let target = out.join(rel);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(&target, &original).unwrap();
        // Exercise the production preview/plan/write path, not just the inner codec.
        let result = (|| -> Result<serde_json::Value, String> {
            let before = reg.texture_preview(&s(&out), rel, 0, Some(0)).map_err(|e| e.to_string())?;
            let rgba = [240, 10, 180, 255].repeat((before.width * before.height) as usize);
            let png = studio_domain::DecodedImage { width:before.width, height:before.height, rgba }.to_png().unwrap();
            let input = target.with_extension("qa.png");
            std::fs::write(&input, png).unwrap();
            let plan = reg.texture_replace_plan(&s(&out), rel, &s(&input), Some(0)).map_err(|e| e.to_string())?;
            if !plan.allowed { return Err(format!("review refused: {:?}", plan.refusal)); }
            let written = reg.import_texture(&s(&out), rel, &s(&input), true, Some(0), plan.review_id).map_err(|e| e.to_string())?;
            let after = Registry::default().texture_preview(&s(&out), rel, 0, Some(0)).map_err(|e| e.to_string())?;
            assert_eq!((before.width, before.height), (after.width, after.height));
            assert_ne!(before.data_url, after.data_url, "decoded replacement must differ");
            assert_ne!(std::fs::read(&target).unwrap(), original);
            assert_eq!(std::fs::read(format!("{}.bak", s(&target))).unwrap(), original);
            Ok(serde_json::json!({"width":after.width,"height":after.height,"container":after.container,"write":written,"decoded_changed":true}))
        })();
        assert_eq!(std::fs::read(source.join(rel)).unwrap(), original, "source stays untouched");
        results.push(match result {
            Ok(value) => serde_json::json!({"path":rel,"roundtrip":value}),
            Err(error) => serde_json::json!({"path":rel,"blocker":error}),
        });
    }
    std::fs::write(out.join("receipt.json"), serde_json::to_vec_pretty(&serde_json::json!({"textures":results,"game_verified":false})).unwrap()).unwrap();
    assert!(results.iter().all(|r| r.get("roundtrip").is_some()), "native texture replacement regression: {results:?}");
}

#[test]
#[ignore = "read-only asset observations on the previously copied NHL07 disc"]
fn nhl07_asset_observations() {
    let root = PathBuf::from(std::env::var("NHL07_QA_ASSETS").expect("copied extracted disc"));
    let out = PathBuf::from(std::env::var("NHL07_QA_OBSERVATIONS").expect("new JSON receipt"));
    assert!(!out.exists());
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl07"), Platform::Xbox,
        &s(&root.join("db/NHL07NG.db")), None, &s(&root.join("db/NHL07NG-meta.xml")), &s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id, &s(&root.join("fe/loc/nhl_eng_us.db"))).unwrap();
    let args = studio_api::AssetRootsArgsDto { install_dir:s(&root), platform:Some("xbox".into()), rendering_override:None, fe_override:None };
    let teams = reg.list_teams(id).unwrap();
    let team = teams.iter().find(|t| t.name.contains("Anaheim")).expect("Anaheim exists");
    let player = reg.list_players(id).unwrap().into_iter().find(|p| p.team_id == Some(team.team_id)).unwrap();
    // Every failed view is preserved as an observation, never counted as editing support.
    fn obs<T:serde::Serialize, E:std::fmt::Display>(r:Result<T,E>) -> serde_json::Value {
        match r { Ok(v)=>serde_json::json!({"value":v}), Err(e)=>serde_json::json!({"error":e.to_string()}) }
    }
    let leagues=reg.list_leagues(id).unwrap();
    let logos:Vec<_>=leagues.leagues.iter().filter(|l| l.code >= 0).map(|l| obs(reg.league_logo(id,l.code as u64,Some(&s(&root))))).collect();
    let arenas=reg.list_arenas(id).unwrap();
    let arena_art:Vec<_>=arenas.iter().take(2).map(|a| obs(reg.arena_assets(id,&args,a.arena_id))).collect();
    let outfits = reg.team_outfits(id,&args,team.team_id).unwrap();
    let font = outfits.iter().find(|o| o.file_name == "font_0_0_0_cm.rx2").expect("Anaheim loose font raster");
    assert_eq!(font.piece,"font");
    assert!(font.label.contains("color map"), "loose file map identity: {}",font.label);
    let receipt=serde_json::json!({
        "team":obs(reg.team_view(id,team.team_id)), "player":obs(reg.player_view(id,player.player_id)),
        "leagues":leagues, "league_logos":logos, "arenas":arenas, "arena_art":arena_art,
        "team_logo":obs(reg.team_logo(id,&args,team.team_id).map(|v| v.map(|png| png.len()))),
        "team_logo_set":obs(reg.team_logo_set(id,team.team_id,Some(&s(&root)))),
        "kits":obs(reg.team_kits(id,&args,team.team_id)), "outfits":{"value":outfits}, "lines":obs(reg.lines_view(id,team.team_id)),
        "equipment":obs(reg.player_equipment_files(id,player.player_id,&args,&repo("defs"))),
        "player_assets":obs(reg.list_player_assets(id,&args))
    });
    std::fs::write(out,serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
}

#[test]
#[ignore = "writes only new copied database files under NHL07_QA_EDITS"]
fn nhl07_team_and_roster_edits_save_and_reopen() {
    let out=PathBuf::from(std::env::var("NHL07_QA_EDITS").expect("fresh QA edit folder"));
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let db=out.join("NHL07NG.db");
    std::fs::copy(repo("anchors/xbox360/nhl07/databases/NHL07NG.db"),&db).unwrap();
    let meta=repo("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml");
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl07"),Platform::Xbox,&s(&db),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let dictionary=out.join("nhl_eng_us.db");
    std::fs::copy(repo("anchors/xbox360/nhl07/databases/nhl_eng_us.db"),&dictionary).unwrap();
    let parsed=tdb_loc::LocDictionary::parse(&std::fs::read(&dictionary).unwrap()).unwrap();
    let key=parsed.rows.iter().find(|r| r.key.is_some() && r.text.as_ref().is_some_and(|t| t.len()>6 && t.is_ascii())).unwrap().key.clone().unwrap();
    reg.attach_dictionary(id,&s(&dictionary)).unwrap();
    reg.dictionary_retext_set(id,&key,"QA").unwrap();
    let team=reg.list_teams(id).unwrap().into_iter().find(|t| t.name.contains("Anaheim")).unwrap();
    let player=reg.list_players(id).unwrap().into_iter().find(|p| p.team_id==Some(team.team_id)).unwrap();
    let gear = reg.player_view(id,player.player_id).unwrap().unwrap();
    assert_eq!(gear.equipment_table,"ubPc");
    assert_eq!(gear.equipment_record,Some(player.bio_record));
    let stick = gear.equipment.iter().find(|f| f.name == "stick").expect("bio gear exposed");
    let stick_value = if stick.decoded == Some(1) { 2 } else { 1 };
    reg.set_field(id,&stick.studio_table,gear.equipment_record.unwrap(),&stick.studio_field,FieldValue::Int(stick_value)).unwrap();
    let view=reg.team_view(id,team.team_id).unwrap().unwrap();
    let name=view.identity.iter().find(|f| f.name=="fullname").unwrap();
    reg.set_field(id,&name.studio_table,0,&name.studio_field,FieldValue::Text("QA Ducks".into())).unwrap();
    assert_eq!(reg.team_view(id,team.team_id).unwrap().unwrap().name,"QA Ducks");
    let home = reg.team_view(id,team.team_id).unwrap().unwrap().arena.expect("thin arena selector resolves");
    assert!(!home.name.is_empty() && !home.city.is_empty());
    let arena = reg.arena_view(id,home.arena_id).unwrap().unwrap();
    assert!(arena.teams.iter().any(|t| t.team_id == team.team_id));
    for (name, value) in [("arenaname", "QA Rink"), ("cityname", "QA City")] {
        let field = arena.all_fields.iter().find(|f| f.name == name).unwrap();
        assert_eq!(field.studio_table, "cubz", "edit the stock row, never the selector");
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
    assert!(reg.summary(id).unwrap().dirty);
    reg.save_workspace(id).unwrap();
    let reopened=Registry::default();
    let rid=reopened.open_workspace_from_paths(Some("nhl07"),Platform::Xbox,&s(&db),None,&s(&meta),&s(&repo("defs"))).unwrap().project_id;
    let gear = reopened.player_view(rid,player.player_id).unwrap().unwrap();
    assert_eq!(gear.equipment.iter().find(|f| f.name == "stick").unwrap().decoded,Some(stick_value));
    assert_eq!(reopened.team_view(rid,team.team_id).unwrap().unwrap().name,"QA Ducks");
    let home = reopened.team_view(rid,team.team_id).unwrap().unwrap().arena.unwrap();
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
fn nhl07_main_database_roster_ops_work_and_incompatible_mirror_is_refused() {
    // Read-only anchors; edits stay in memory, with a fresh workspace per operation.
    let db = repo("anchors/xbox360/nhl07/databases/NHL07NG.db");
    let meta = repo("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml");
    let roster = repo("anchors/xbox360/nhl07/savefiles/ROSTER1/ROSTER1");
    let roster_bytes = std::fs::read(&roster).unwrap();
    let inner = tdb_savedata::SaveContainer::parse(&roster_bytes).unwrap().extract_db().unwrap();
    let parsed = tdb_core::TdbFile::parse(&inner).unwrap();
    eprintln!("Roster schema: {:?}", parsed.tables.iter().map(|t| t.id.canonical(parsed.endian)).collect::<Vec<_>>());
    for with_roster in [false, true] {
        for operation in ["jersey", "captain", "release"] {
            let reg = Registry::default();
            let save = s(&roster);
            let opened = reg.open_workspace_from_paths(Some("nhl07"), Platform::Xbox, &s(&db), with_roster.then_some(save.as_str()), &s(&meta), &s(&repo("defs")));
            if with_roster {
                let error = match opened { Ok(_) => panic!("a difflist must not be admitted as a snapshot mirror"), Err(e) => e.to_string() };
                assert!(error.contains("roster stores changes to the game database"), "{error}");
                continue;
            }
            let p = opened.unwrap();
            let anaheim = reg.list_teams(p.project_id).unwrap().into_iter().find(|t| t.name.contains("Anaheim")).unwrap();
            assert!(reg.team_logo_set(p.project_id, anaheim.team_id, None).unwrap().is_some(), "NHL07 stock-team art id must resolve through the thin selector");
            let home = reg.team_view(p.project_id,anaheim.team_id).unwrap().unwrap().arena.expect("NHL07 home arena join");
            assert!(!home.name.is_empty() && !home.city.is_empty());
            let arena = reg.arena_view(p.project_id,home.arena_id).unwrap().unwrap();
            assert!(arena.identity.iter().any(|f| f.name == "arenaname" && f.studio_table == "cubz"));
            assert!(arena.teams.iter().any(|t| t.team_id == anaheim.team_id));
            let assignment = reg.arena_assignment_preview(p.project_id,anaheim.team_id,home.arena_id).unwrap();
            assert!(!assignment.allowed && assignment.refusal.unwrap().contains("separate arena selector"));
            let players = reg.list_players(p.project_id).unwrap();
            let player = players.iter().find(|p| p.team_id.is_some()).unwrap();
            let gear = reg.player_view(p.project_id,player.player_id).unwrap().unwrap();
            assert_eq!(gear.equipment_table,"ubPc");
            assert_eq!(gear.equipment_record,Some(player.bio_record));
            assert!(gear.equipment.iter().any(|f| f.name == "stick" && f.studio_table == "ubPc"));
            let team = player.team_id.unwrap();
            let outcome = match operation {
                "jersey" => reg.set_jersey_number(p.project_id, team, player.player_id, 98),
                "captain" => reg.set_captaincy(p.project_id, team, player.player_id, 1),
                _ => reg.release_to_fa(p.project_id, team, player.player_id),
            };
            let outcome = outcome.unwrap_or_else(|e| panic!("main DB {operation}: {e}"));
            assert!(outcome.edits > 0);
            assert!(reg.summary(p.project_id).unwrap().dirty);
        }
    }
}

#[test]
#[ignore = "requires a local NHL07 Xbox360 ISO and a fresh writable QA output folder"]
fn real_nhl07_iso_stage_edit_reopen_and_dumped_disc_extract() {
    let iso = PathBuf::from(std::env::var("NHL_STUDIO_XISO").expect("source ISO"));
    let out = PathBuf::from(std::env::var("NHL07_QA_OUT").expect("fresh output folder"));
    assert!(!out.exists(), "choose a fresh QA directory; never overwrite prior evidence");
    std::fs::create_dir_all(&out).unwrap();
    let reg = Registry::default();
    let meta = repo("anchors/xbox360/nhl07/databases/NHL07NG-meta.xml");
    let defs = repo("defs");
    let source = studio_domain::xiso_source::locate_databases(&iso).unwrap();
    let original = studio_domain::xiso_source::read_located(&iso, source.nhlng.as_ref().unwrap()).unwrap();
    let stage = out.join("arbitrary location with spaces/iso staging");
    let (staged, project) = reg.iso_open(&s(&iso), "nhl07", "xbox360", &s(&meta), &s(&stage), &s(&defs)).unwrap();
    let id = project.project_id;
    assert_eq!(std::fs::read(&staged.db_path).unwrap(), original);
    let players = reg.list_players(id).unwrap();
    let teams = reg.list_teams(id).unwrap();
    assert!(!players.is_empty() && !teams.is_empty());
    eprintln!("ISO opened: {} players, {} teams", players.len(), teams.len());
    // This field is measured by the NHL07 anchor and the existing edit-QA runner.
    reg.set_field(id, "ubPc", 0, "HdeP", FieldValue::Text("QA".into())).unwrap();
    assert!(reg.summary(id).unwrap().dirty);
    reg.save_workspace(id).unwrap();
    let reopened = Registry::default();
    let (_, second) = reopened.iso_open(&s(&iso), "nhl07", "xbox360", &s(&meta), &s(&stage), &s(&defs)).unwrap();
    assert!(!reopened.summary(second.project_id).unwrap().dirty);
    assert_eq!(reopened.list_players(second.project_id).unwrap().iter().find(|p| p.bio_record == 0).unwrap().first_name, "QA");
    let saved = std::fs::read(&staged.db_path).unwrap();
    assert_ne!(saved, original);
    assert!(tdb_core::TdbFile::parse(&saved).unwrap().header_crc_ok());
    assert_eq!(studio_domain::xiso_source::read_located(&iso, source.nhlng.as_ref().unwrap()).unwrap(), original);

    // Dump the actual disc to an unrelated directory, streaming each member.
    // No hardcoded emulator/library path participates in discovery or extraction.
    let dump = out.join("relocated disc dump");
    studio_domain::disc_extract::extract(&iso,&dump,&AtomicBool::new(false),&mut |_,_|{}).unwrap();
    let before = discovery::detect_install("nhl07", Platform::Xbox, &dump, None);
    assert!(before.db_paths.nhlng.is_some());
    let result = extract_service::extract(&dump, Platform::Xbox, &AtomicBool::new(false), &mut |_| {}).unwrap();
    eprintln!("Dumped disc extraction: {result:?}");
    let found = discovery::detect_install("nhl07", Platform::Xbox, &dump, None);
    let db = found.db_paths.nhlng.unwrap();
    let dict = found.db_paths.localization.unwrap();
    let metadata = found.db_paths.meta_xml.unwrap();
    let loose = Registry::default();
    let opened = loose.open_workspace_from_paths(Some("nhl07"), Platform::Xbox, &db, None, &metadata, &s(&defs)).unwrap();
    loose.attach_dictionary(opened.project_id, &dict).unwrap();
    assert_eq!(loose.list_players(opened.project_id).unwrap().len(), players.len());
    assert_eq!(loose.list_teams(opened.project_id).unwrap().len(), teams.len());
    std::fs::write(out.join("source-routes.json"), serde_json::to_vec_pretty(&serde_json::json!({
        "iso":iso, "staging":stage, "dump":dump, "players":players.len(), "teams":teams.len(),
        "extraction":format!("{result:?}"), "iso_source_db_unchanged":true, "staged_edit_reopened":true
    })).unwrap()).unwrap();
}
