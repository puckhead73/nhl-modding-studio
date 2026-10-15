//! Read-only PS3 source and preview qualification across supported titles.
use std::path::{Path,PathBuf};
use studio_api::{Registry,dto::AssetRootsArgsDto};
use studio_domain::{Platform,extract_service};
fn repo(p:&str)->PathBuf{Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)}
#[test]
#[ignore = "requires local PS3 sources"]
fn ps3_inventory(){
 let base=PathBuf::from(r"X:\Games\Emulation\Roms\Sony Playstation 3");
 for (game,folder) in [("nhl08","NHL 08/NHL 08 (USA)"),("nhl09","NHL 09/NHL 09 (USA) (En,Fr)"),("nhl10","NHL 10"),("nhl11","NHL 11"),("nhl12","NHL 12/[BLUS 30791] NHL® 12"),("nhl13","NHL 13"),("nhl14","NHL 14"),("nhl15","NHL 15"),("nhllegacy","NHL Legacy - Vanilla")] {
 if std::env::var("PS3_GAME_FILTER").is_ok_and(|g|g!=game){continue;}
 let root=base.join(folder).join("PS3_GAME/USRDIR");
 let pf=extract_service::preflight(&root,Platform::Ps3);
 println!("{game}: {} total={} remaining={}",pf.state,pf.members_total,pf.members_to_write);
 for a in &pf.archives {println!("  {}: {} members={} {:?}",a.path.file_name().unwrap().to_string_lossy(),a.state,a.members.len(),a.parse_error);}
 if std::env::var_os("PS3_DECODE_ARCHIVES").is_some() {
 let mut counts=std::collections::BTreeMap::<String,usize>::new();
 let mut errors=Vec::new();
 for a in &pf.archives {
 if a.path.parent()!=Some(root.as_path()) {continue;}
 let entries=directory(&a.path);
 let mut input=std::fs::File::open(&a.path).unwrap();
 for (name,offset,size) in entries {
 let name=name.replace('\\',"/");
 let parts:Vec<_>=name.split('/').collect();
 let front=parts.len()>4 && parts[0]=="fe" && ["teamlogos","leaguelogos","playerheads"].contains(&parts[3]) && name.ends_with(".big");
 if parts.len()<3 || (!front && !name.ends_with(".rpsgl")) {continue;}
 let category=if front {parts[3]}else{parts[1]};
 if !["jersey","pant","sock","glove","helmet","helmet_common","skate","player","player_common","teamlogos","leaguelogos","playerheads"].contains(&category) {continue;}
 let stem=Path::new(&name).file_stem().unwrap().to_string_lossy();
 let texture=front || stem.starts_with("texlib") || ["_cm","_nm","_sm","_am","_dm","_tm","_cnm","_cam","_ism","_rm","_csm","_snm","_ssm","_cpm","_ctm"].iter().any(|s|stem.ends_with(s));
 let key=format!("{category}/{}",if texture {"texture".to_string()}else{format!("mesh/{}",stem.split('_').next().unwrap())});
 if *counts.get(&key).unwrap_or(&0)>=3 {continue;}
 *counts.entry(key.clone()).or_default()+=1;
 use std::io::{Read,Seek,SeekFrom};
 input.seek(SeekFrom::Start(offset)).unwrap();let mut raw=vec![0u8;size as usize];input.read_exact(&mut raw).unwrap();
 let raw=match tdb_archive_big::eb::decode_payload(raw).unwrap(){tdb_archive_big::eb::Payload::Data(d)=>d,_=>panic!("unsupported compression")};
 let raw=if tdb_archive_codec::is_refpack(&raw){tdb_archive_codec::refpack_decompress(&raw).unwrap()}else{raw};
 let sample=repo(&format!("target/ps3-asset-samples/{game}/{name}"));std::fs::create_dir_all(sample.parent().unwrap()).unwrap();std::fs::write(&sample,&raw).unwrap();
 let result=if texture {studio_domain::assets::decode_texture_bytes(raw,Path::new(&name),None).map(|_|()).map_err(|e|e.to_string())}else{tdb_rx2::playermesh::parse_equipment_mesh(&raw).map(|_|()).map_err(|e|e.to_string())};
 if let Err(e)=result { errors.push(format!("{name}: {e}")); }
 }
 }
 println!("DECODE {game}: {counts:?} ERRORS {errors:?}");
 assert!(errors.is_empty(),"{game}: {errors:?}");
}
 let reg=Registry::default();let d=repo(&format!("anchors/ps3/{game}/databases"));
 let id=reg.open_workspace_from_paths(Some(game),Platform::Ps3,d.join("nhlng.db").to_str().unwrap(),None,d.join("nhlng-meta.xml").to_str().unwrap(),repo("defs").to_str().unwrap()).unwrap().project_id;
 let args=AssetRootsArgsDto{install_dir:root.to_string_lossy().into_owned(),platform:Some("ps3".into()),rendering_override:None,fe_override:None};
 println!("  jerseys={} outfits={}",reg.list_team_jerseys(id,0).unwrap().len(),reg.team_outfits(id,&args,0).unwrap().len());
 }
}

fn directory(path:&Path)->Vec<(String,u64,u64)> {
 use std::io::{Read,Seek,SeekFrom,BufRead};
 let mut f=std::fs::File::open(path).unwrap();let mut h=[0u8;48];f.read_exact(&mut h).unwrap();
 if h.starts_with(b"EB") {let header=tdb_archive_big::eb::EbHeader::parse(&h).unwrap();let mut b=vec![0;header.table_end()];f.seek(SeekFrom::Start(0)).unwrap();f.read_exact(&mut b).unwrap();return tdb_archive_big::eb::EbArchive::parse(&b).unwrap().entries.into_iter().map(|e|(e.path,e.offset,e.size as u64)).collect();}
 f.seek(SeekFrom::Start(16)).unwrap();let mut r=std::io::BufReader::new(f);let mut b=h[..16].to_vec();
 let width=if matches!(&h[..4],b"BIG5"|b"BIG6"|b"BIG7"|b"BIG8"|b"BIG9"){9}else{8};
 for _ in 0..u32::from_be_bytes(h[8..12].try_into().unwrap()) {let mut n=vec![0;width];r.read_exact(&mut n).unwrap();b.extend(n);r.read_until(0,&mut b).unwrap();}
 tdb_archive_big::Archive::parse(b).unwrap().entries.into_iter().map(|e|(e.name,e.offset as u64,e.size as u64)).collect()
}

#[test]
#[ignore = "extracts NHL09 PS3 into a workspace-owned QA directory"]
fn nhl09_extraction_and_preview(){
 let root=Path::new(r"X:\Games\Emulation\Roms\Sony Playstation 3\NHL 09\NHL 09 (USA) (En,Fr)\PS3_GAME\USRDIR");
 let output=repo("target/nhl09-ps3-art-qa");std::fs::create_dir_all(&output).unwrap();
 if extract_service::preflight_to(root,&output,Platform::Ps3).state != "complete" {
 let result=extract_service::extract_to(root,&output,Platform::Ps3,&std::sync::atomic::AtomicBool::new(false),&mut |_|{}).unwrap();
 assert_eq!(result.archives_failed,0);assert_eq!(result.files_unsupported,0);
 }
 assert_eq!(extract_service::preflight_to(root,&output,Platform::Ps3).state,"complete");
 let r=Registry::default();let d=repo("anchors/ps3/nhl09/databases");let id=r.open_workspace_from_paths(Some("nhl09"),Platform::Ps3,d.join("nhlng.db").to_str().unwrap(),None,d.join("nhlng-meta.xml").to_str().unwrap(),repo("defs").to_str().unwrap()).unwrap().project_id;
 let args=AssetRootsArgsDto{install_dir:output.to_string_lossy().into_owned(),platform:Some("ps3".into()),rendering_override:None,fe_override:None};
 assert!(!r.team_outfits(id,&args,0).unwrap().is_empty());
 for gear in [false,true] {
 let kit=r.team_kit_preview(id,&args,0,0,0,None,gear).unwrap();
 for p in &kit.pieces {println!("PIECE {} {} materials {}/{}",p.kind,p.source_path,p.submeshes.iter().filter(|s|s.material_index.is_some()).count(),p.submeshes.len());}
 for kind in ["head","jersey","pants","socks","gloves","skates","helmet"]{let p=kit.pieces.iter().find(|p|p.kind==kind).unwrap_or_else(||panic!("missing {kind}"));assert!(p.submeshes.iter().all(|s|s.material_index.is_some()),"{kind}: missing texture");}
 }
}
