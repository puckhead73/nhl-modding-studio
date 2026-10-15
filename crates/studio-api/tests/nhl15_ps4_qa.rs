//! NHL15 PS4 read-only inventory plus copied editing QA. Originals are never written.
use std::path::{Path, PathBuf};
use studio_api::{Registry,AssetRootsArgsDto};
use studio_domain::Platform;
fn repo(p:&str)->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)}
fn s(p:&Path)->String {p.to_string_lossy().into_owned()}
fn obs<T:serde::Serialize,E:std::fmt::Display>(r:Result<T,E>)->serde_json::Value {
    match r {Ok(v)=>serde_json::json!({"value":v}),Err(e)=>serde_json::json!({"error":e.to_string()})}
}
#[test]
#[ignore="read-only NHL15 PS4 install inventory; creates a fresh receipt"]
fn inspect_ps4_surfaces() {
    let root=PathBuf::from("H:/Emulators/ShadPS4/Games/CUSA00561");
    let out=repo("target/overnight-20261003/nhl15/ps4-observations-3.json");
    assert!(!out.exists());
    let a=repo("anchors/ps4/nhl15/databases");
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl15"),Platform::Ps4,&s(&a.join("nhl_game.db")),None,&s(&a.join("nhl_game-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
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
