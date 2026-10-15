//! NHL08 review regressions. Anchors and extracted assets are read-only.
use std::path::{Path, PathBuf};
use studio_api::{Registry, dto::AssetRootsArgsDto};
use studio_domain::{FieldValue, Platform};
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
fn open(reg: &Registry) -> u32 {
    reg.open_workspace_from_paths(Some("nhl08"), Platform::Xbox,
        &s(&repo("anchors/xbox360/nhl08/databases/nhlng.db")), None,
        &s(&repo("anchors/xbox360/nhl08/databases/nhlng-meta.xml")), &s(&repo("defs"))).unwrap().project_id
}
#[test]
fn rivals_use_direct_team_ids_and_edits_round_trip() {
    let reg=Registry::default(); let id=open(&reg);
    for (team, rival) in [(0,13),(24,10),(4,11),(11,4),(30,31),(31,30)] {
        let view=reg.team_view(id,team).unwrap().unwrap();
        let cell=view.profile.sections.iter().flat_map(|s| &s.cells).find(|c| c.label=="Rival Team").unwrap();
        assert_eq!(cell.team_offset,Some(0));assert_eq!(cell.field.as_ref().unwrap().decoded,Some(rival));
    }
    let view=reg.team_view(id,24).unwrap().unwrap();
    let cell=view.profile.sections.iter().flat_map(|s| &s.cells).find(|c| c.label=="Rival Team").unwrap();
    let f=cell.field.as_ref().unwrap();
    reg.set_field(id,&f.studio_table,cell.record,&f.studio_field,FieldValue::Int(6)).unwrap();
    let view=reg.team_view(id,24).unwrap().unwrap();
    assert_eq!(view.profile.sections.iter().flat_map(|s| &s.cells).find(|c| c.label=="Rival Team").unwrap().field.as_ref().unwrap().decoded,Some(6));
}
#[test]
fn draft_years_display_calendar_years_and_expose_the_write_offset() {
    let reg=Registry::default();let id=open(&reg);let players=reg.list_players(id).unwrap();
    for (name,year) in [("Modano",1988),("Selanne",1988),("Pronger",1993),("Sakic",1987),("Crosby",2005)] {
        let player=players.iter().find(|p|p.last_name==name).unwrap();
        let view=reg.player_view(id,player.player_id).unwrap().unwrap();
        let cell=view.profile.sections.iter().flat_map(|s| &s.cells).find(|c|c.label=="Draft Year").unwrap();
        assert!(cell.unverified.is_none());let field=cell.field.as_ref().unwrap();
        assert_eq!(field.display,year.to_string());assert_eq!(field.edit_offset,Some(1959));
        reg.set_field(id,&field.studio_table,cell.record,&field.studio_field,FieldValue::Int(2006-1959)).unwrap();
        let updated=reg.player_view(id,player.player_id).unwrap().unwrap();
        assert_eq!(updated.all_fields.iter().find(|f|f.name=="yeardrafted").unwrap().display,"2006");
    }
}
#[test]
#[ignore = "requires NHL08_REVIEW_ASSETS extracted Xbox360 game directory"]
fn real_kit_has_both_sides_and_coloured_gloves_and_helmet() {
    let root=std::env::var("NHL08_REVIEW_ASSETS").unwrap(); let reg=Registry::default();let id=open(&reg);
    let args=AssetRootsArgsDto{install_dir:root,platform:Some("xbox".into()),rendering_override:None,fe_override:None};
    let kit=reg.team_kit_preview(id,&args,0,0,0,None,false).unwrap();
    for kind in ["gloves","socks","skates"] {
        let piece=kit.pieces.iter().find(|p|p.kind==kind).unwrap();
        assert_eq!(piece.source_path.lines().count(),2,"{kind}");
        let xs:Vec<_>=piece.submeshes.iter().flat_map(|m| &m.positions).map(|p|p[0]).collect();
        assert!(xs.iter().any(|x|*x<0.) && xs.iter().any(|x|*x>0.),"{kind} must cover both sides");
    }
    // Each handed skate must select its own colour raster from the shared bundle.
    let skates = kit.pieces.iter().find(|p| p.kind == "skates").unwrap();
    let maps: std::collections::BTreeSet<_> = skates.submeshes.iter().map(|mesh| {
        kit.textures[mesh.material_index.expect("skate texture") as usize].name.as_str()
    }).collect();
    assert!(maps.iter().any(|name| name.starts_with("skate_0_0_") && name.contains("_cm")), "{maps:?}");
    assert!(maps.iter().any(|name| name.starts_with("skate_0_1_") && name.contains("_cm")), "{maps:?}");
    for kind in ["gloves","helmet"] {
        let piece=kit.pieces.iter().find(|p|p.kind==kind).unwrap();
        assert!(piece.submeshes.iter().all(|m|m.material_index.is_some()),"{kind} lacks material");
        for mesh in &piece.submeshes {let tex=&kit.textures[mesh.material_index.unwrap() as usize];assert!(tex.name.contains("cm"),"{}",tex.name);assert!(!tex.rgba_base64.is_empty());}
    }
}
#[test]
#[ignore = "requires NHL08_PS3_INSTALL; original assets read-only"]
fn ps3_team_and_shannon_equipment_previews_assemble() {
    let root=std::env::var("NHL08_PS3_INSTALL").unwrap();
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl08"),Platform::Ps3,
        &s(&repo("anchors/ps3/nhl08/databases/nhlng.db")),None,
        &s(&repo("anchors/ps3/nhl08/databases/nhlng-meta.xml")),&s(&repo("defs"))).unwrap().project_id;
    let player=reg.list_players(id).unwrap().into_iter().find(|p|p.first_name=="Ryan"&&p.last_name=="Shannon").unwrap();
    let args=AssetRootsArgsDto{install_dir:root,platform:Some("ps3".into()),rendering_override:None,fe_override:None};
    for gear in [false,true] {
        let kit=reg.team_kit_preview(id,&args,player.team_id.unwrap(),0,0,Some(player.player_id),gear).unwrap();
        let head=kit.pieces.iter().find(|p|p.kind=="head").expect("selected player head");
        assert!(head.source_path.contains("head_10010.rpsgl"));
        assert!(head.source_path.contains("hair_10010_0_0_0.rpsgl"));
        println!("gear={gear}: head submeshes {:?}", head.submeshes.iter().map(|m| &m.name).collect::<Vec<_>>());
        for kind in ["head","jersey","pants","socks","gloves","helmet","skates"] {
            let piece=kit.pieces.iter().find(|p|p.kind==kind).unwrap_or_else(||panic!("missing {kind}, gear={gear}"));
            assert!(piece.submeshes.iter().any(|m|!m.positions.is_empty()));
            assert!(piece.submeshes.iter().all(|m|m.material_index.is_some()),"{kind} lacks texture, gear={gear}");
        }
    }
}
