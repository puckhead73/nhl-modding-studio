use std::path::{Path,PathBuf};
use studio_api::{Registry,dto::AssetRootsArgsDto};
use studio_domain::Platform;
fn repo(p:&str)->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)}
fn open()->(Registry,u32){let r=Registry::default();let d=repo("anchors/xbox360/nhl09/databases");let id=r.open_workspace_from_paths(Some("nhl09"),Platform::Xbox,d.join("nhlng.db").to_str().unwrap(),None,d.join("nhlng-meta.xml").to_str().unwrap(),repo("defs").to_str().unwrap()).unwrap().project_id;(r,id)}
#[test]
fn draft_years_use_the_verified_nhl09_offset() {
    let (reg,id)=open();
    let players=reg.list_players(id).unwrap();
    for (name,year) in [("Modano",1988),("Selanne",1988),("Pronger",1993),("Sakic",1987),("Crosby",2005)] {
        let p=players.iter().find(|p|p.last_name==name).unwrap();
        let v=reg.player_view(id,p.player_id).unwrap().unwrap();
        let cell=v.profile.sections.iter().flat_map(|s|&s.cells).find(|c|c.label=="Draft Year").unwrap();
        assert!(cell.unverified.is_none());
        let f=cell.field.as_ref().unwrap();
        assert_eq!(f.display,year.to_string());assert_eq!(f.edit_offset,Some(1900));
        reg.set_field(id,&f.studio_table,cell.record,&f.studio_field,studio_domain::FieldValue::Int(108)).unwrap();
        assert_eq!(reg.player_view(id,p.player_id).unwrap().unwrap().all_fields.iter().find(|f|f.name=="yeardrafted").unwrap().display,"2008");
    }
}
#[test]
#[ignore = "requires NHL09_ASSETS; all source assets read-only"]
fn real_kit_has_paired_textured_equipment_and_eyes() {
    let(reg,id)=open();
    let args=AssetRootsArgsDto{install_dir:std::env::var("NHL09_ASSETS").unwrap(),platform:Some("xbox".into()),rendering_override:None,fe_override:None};
    for gear in [false,true] {
        let kit=reg.team_kit_preview(id,&args,0,0,0,None,gear).unwrap();
        for kind in ["head","jersey","pants","socks","gloves","skates","helmet"] {
            let p=kit.pieces.iter().find(|p|p.kind==kind).unwrap_or_else(||panic!("missing {kind}"));
            assert!(p.submeshes.iter().any(|m|!m.positions.is_empty()));
            assert!(p.submeshes.iter().all(|m|m.material_index.is_some()),"{kind} has missing textures, gear={gear}");
            if ["socks","gloves","skates"].contains(&kind) {assert_eq!(p.source_path.lines().count(),2,"{kind}");}
            if kind=="head" {for eye in p.submeshes.iter().filter(|m|m.name.contains("eye")){assert!(kit.textures[eye.material_index.unwrap() as usize].name.contains("eyes"));}}
        }
        println!("NHL09 gear={gear}: paired geometry, helmet and all textures verified");
    }
}
