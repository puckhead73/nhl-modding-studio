use std::path::Path;
fn main()->Result<(),Box<dyn std::error::Error>> {
 let root=Path::new("F:/NHL Installations/Xbox 360/NHL07/NHL 07 (USA)/rendering");
 let meshes=studio_domain::kit_mesh_defaults(root);
 for (kind,path) in [("jersey",meshes.jersey),("pants",meshes.pant),("socks",meshes.sock),("gloves",meshes.glove),("helmet",meshes.helmet),("skates",meshes.skate)] {
  match path {Some(p)=>{let mut bytes=std::fs::read(&p)?;if tdb_archive_codec::is_refpack(&bytes){bytes=tdb_archive_codec::refpack_decompress(&bytes)?;}match tdb_rx2::playermesh::parse_equipment_mesh(&bytes){Ok(m)=>println!("{kind}: {} submeshes from {}",m.submeshes.len(),p.display()),Err(e)=>println!("{kind}: ERROR {e} {}",p.display())}},None=>println!("{kind}: MISSING")}
 }
 let repo=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let tmp=tempfile::tempdir()?;
 let reg=studio_api::Registry::default();let store=studio_domain::BaselineStore::open(&repo.join("baselines/INVENTORY.json"),&repo.join("anchors"))?;
 let id=reg.project_create_fresh(&tmp.path().join("p").to_string_lossy(),"nhl07",studio_domain::Platform::Xbox,"",None,None,&store,&repo.join("defs").to_string_lossy())?.summary.project_id;
 let roots=studio_api::dto::AssetRootsArgsDto{install_dir:root.parent().unwrap().to_string_lossy().into_owned(),platform:Some("xbox".into()),rendering_override:None,fe_override:None};
 for team in [2] {

 let kit=reg.team_kit_preview(id,&roots,team,0,0,None,false)?;
 for kind in ["jersey","pants","socks","skates"] {assert!(kit.pieces.iter().any(|p|p.kind==kind&&!p.submeshes.is_empty()),"Team {team} missing {kind}");}
 println!("TEAM {team}: {}",kit.pieces.iter().map(|p|format!("{}: {} meshes",p.kind,p.submeshes.len())).collect::<Vec<_>>().join(", "));
 for piece in &kit.pieces {let xs:Vec<_>=piece.submeshes.iter().flat_map(|s|s.positions.iter().map(|v|v[0])).collect();println!("BOUNDS {} {}..{} textures {:?}",piece.kind,xs.iter().copied().fold(f32::INFINITY,f32::min),xs.iter().copied().fold(f32::NEG_INFINITY,f32::max),piece.submeshes.iter().map(|s|s.material_index).collect::<Vec<_>>());}
 for kind in ["skates","socks","gloves"] {
 let piece=kit.pieces.iter().find(|p|p.kind==kind).unwrap();assert_eq!(piece.submeshes.len(),2,"{kind} needs both meshes");
 let xs:Vec<_>=piece.submeshes.iter().flat_map(|m|m.positions.iter().map(|p|p[0])).collect();assert!(xs.iter().any(|x|*x < -0.001)&&xs.iter().any(|x|*x > 0.001),"{kind} needs both sides");
 assert!(piece.submeshes.iter().all(|m|m.material_index.is_some()),"{kind} missing colour texture");
 }
 assert!(kit.pieces.iter().find(|p|p.kind=="helmet").unwrap().submeshes.iter().all(|m|m.material_index.is_some()));
 let outfits=reg.team_outfits(id,&roots,team)?;
 for t in outfits.iter().filter(|t|t.piece=="jersey"&&t.label.ends_with("color map")).take(2){println!("COLOR {team}: {} #{}",t.rel_path,t.texture_index);}
 }
 reg.close_project(id);
 Ok(())
}
