//! NHL18 PS4 read-only inventory plus copied editing QA. Originals are never written.
use std::path::{Path, PathBuf};
use studio_api::{Registry,AssetRootsArgsDto};
use studio_domain::Platform;
fn repo(p:&str)->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)}
fn s(p:&Path)->String {p.to_string_lossy().into_owned()}
fn obs<T:serde::Serialize,E:std::fmt::Display>(r:Result<T,E>)->serde_json::Value {
    match r {Ok(v)=>serde_json::json!({"value":v}),Err(e)=>serde_json::json!({"error":e.to_string()})}
}
#[test]
#[ignore="read-only NHL18 PS4 install inventory; creates a fresh receipt"]
fn inspect_ps4_surfaces() {
    let root=PathBuf::from("H:/Emulators/ShadPS4/Games/CUSA07569");
    let out=repo("target/overnight-20261003/nhl18/ps4-observations-1.json");
    inspect_surfaces(root,out);
}
#[test]
#[ignore="qualifies the isolated external PKG extraction; writes a fresh staging folder"]
fn open_database_from_pkg_output() {
    let root=Path::new("F:/TEMP/nhl-studio-overnight-20261003/nhl18/pkg-source-routes-1/CUSA07569");
    let detected=studio_domain::discovery::detect_install("nhl18",Platform::Ps4,root,None);
    assert!(detected.game_dir.is_some() && detected.extract_needed);
    let out=repo("target/overnight-20261003/nhl18/pkg-staged-db-1");
    assert!(!out.exists());
    std::fs::create_dir_all(&out).unwrap();
    let bytes=std::fs::read(root.join("databigs/inxp_00.big")).unwrap();
    let bound=tdb_archive_big::stage::BoundSource::bind(&mut std::io::Cursor::new(&bytes)).unwrap();
    for (member,name) in [("nhl/data/common/gamemodes/database/t3db/nhl_game.db","nhl_game.db"),("nhl/data/kettle/loc/eng_us.db","eng_us.db")] {
        let entry=bound.entries.iter().find(|e|e.path==member).unwrap();
        let payload=bytes[entry.offset as usize..entry.offset as usize+entry.size as usize].to_vec();
        match tdb_archive_big::eb::decode_payload(payload).unwrap() {
            tdb_archive_big::eb::Payload::Data(data)=>std::fs::write(out.join(name),data).unwrap(),
            _=>panic!("database member was not decoded"),
        }
    }
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl18"),Platform::Ps4,&s(&out.join("nhl_game.db")),None,&s(&repo("anchors/ps4/nhl18/databases/nhl_game-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id,&s(&out.join("eng_us.db"))).unwrap();
    let players=reg.list_players(id).unwrap().len();
    let teams=reg.list_teams(id).unwrap().len();
    assert_eq!((players,teams),(7236,296));
    std::fs::write(out.join("receipt.json"),serde_json::to_vec_pretty(&serde_json::json!({"players":players,"teams":teams,"package_root":root,"studio_pkg_ui":false,"game_verified":false})).unwrap()).unwrap();
}
#[test]
#[ignore="qualifies freshly extracted relocated NHL18 assets; requires completed source receipt"]
fn inspect_relocated_ps4_surfaces() {
    let receipt:serde_json::Value=serde_json::from_slice(&std::fs::read(repo("target/overnight-20261003/nhl18/ps4-source-receipt-1.json")).unwrap()).unwrap();
    assert_eq!(receipt["state"],"complete");
    let root=PathBuf::from("F:/TEMP/nhl-studio-overnight-20261003/nhl18/ps4-source-routes-1/NHL 18/CUSA07569");
    let direct=studio_domain::discovery::detect_install("nhl18",Platform::Ps4,&root,None);
    let parent=studio_domain::discovery::detect_install("nhl18",Platform::Ps4,root.parent().unwrap(),None);
    assert!(direct.db_paths.nhlng.is_some());
    let installed=studio_domain::discovery::detect_install("nhl18",Platform::Ps4,Path::new("H:/Emulators/ShadPS4/Games/CUSA07569"),None);
    assert_eq!(std::fs::read(direct.db_paths.nhlng.as_ref().unwrap()).unwrap(),std::fs::read(installed.db_paths.nhlng.as_ref().unwrap()).unwrap(),"freshly extracted database agrees with independently extracted installation");
    assert_eq!(std::fs::canonicalize(direct.db_paths.nhlng.unwrap()).unwrap(),std::fs::canonicalize(parent.db_paths.nhlng.unwrap()).unwrap());
    inspect_surfaces(root,repo("target/overnight-20261003/nhl18/ps4-relocated-observations-1.json"));
}
fn inspect_surfaces(root:PathBuf,out:PathBuf) {
    assert!(!out.exists());
    let a=repo("anchors/ps4/nhl18/databases");
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl18"),Platform::Ps4,&s(&a.join("nhl_game.db")),None,&s(&a.join("nhl_game-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
    reg.attach_dictionary(id,&s(&a.join("eng_us.db"))).unwrap();
    let teams=reg.list_teams(id).unwrap(); let players=reg.list_players(id).unwrap();
    let team=teams.iter().find(|t| t.name.contains("Anaheim")).unwrap();
    let player=players.iter().find(|p|p.team_id==Some(team.team_id)&&!p.is_goalie).unwrap();
    let args=AssetRootsArgsDto{install_dir:s(&root),platform:Some("ps4".into()),rendering_override:None,fe_override:None};
    let leagues=reg.list_leagues(id).unwrap();
    let logos:Vec<_>=leagues.leagues.iter().filter(|l|l.code>=0).map(|l|obs(reg.league_logo(id,l.code as u64,Some(&s(&root))))).collect();
    for variants in [reg.league_logo(id,0,Some(&s(&root))).unwrap().unwrap().variants,
        reg.team_logo_set(id,team.team_id,Some(&s(&root))).unwrap().unwrap().variants] {
        assert!(variants.iter().any(|v|v.exists==Some(true)),"real extracted logos must be found");
        for v in variants.iter().filter(|v|v.exists==Some(true)) {
            assert!(root.join(v.loose_rel_path.as_ref().unwrap()).is_file());
            assert!(v.rel_path.starts_with("nhl/ui/"),"archive member stays canonical");
        }
    }
    let arenas=reg.list_arenas(id).unwrap();
    let arts:Vec<_>=arenas.iter().take(2).map(|a|obs(reg.arena_assets_with_defs(id,&args,a.arena_id,Some(&repo("defs"))))).collect();
    std::fs::write(&out,serde_json::to_vec_pretty(&serde_json::json!({
        "players":players.len(),"teams":teams.len(),"team_id":team.team_id,"player_id":player.player_id,
        "team":obs(reg.team_view(id,team.team_id)),"player":obs(reg.player_view(id,player.player_id)),
        "team_text":obs(reg.team_text(id,team.team_id)),"leagues":leagues,"league_logos":logos,
        "arenas":arenas,"arena_art":arts,"kits":obs(reg.team_kits(id,&args,team.team_id)),
        "outfits":obs(reg.team_outfits(id,&args,team.team_id)),"lines":obs(reg.lines_view(id,team.team_id)),
        "equipment":obs(reg.player_equipment_files(id,player.player_id,&args,&repo("defs"))),
        "ps4_gear":obs(reg.ps4_skater_gear(id,&args,player.player_id,&repo("defs"))),"team_equipment":obs(reg.list_team_equipment(id,team.team_id)),"game_verified":false
    })).unwrap()).unwrap();
    assert!(!players.is_empty()&&!teams.is_empty());
    eprintln!("PS4 {} players / {} teams; receipt {}",players.len(),teams.len(),out.display());
}


