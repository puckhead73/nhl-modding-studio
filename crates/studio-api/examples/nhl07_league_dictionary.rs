use std::path::Path;
fn main()->Result<(),Box<dyn std::error::Error>>{
 let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
 let bytes=std::fs::read(root.join("anchors/xbox360/nhl07/databases/nhl_eng_us.db"))?;
 let d=tdb_loc::LocDictionary::parse(&bytes)?;
 for r in &d.rows {let text=r.text.as_deref().unwrap_or(""); let key=r.key.as_deref().unwrap_or("");let q=format!("{key} {text}").to_lowercase();if ["league","conference","division","elitser","liiga","western","eastern","northwest","pacific"].iter().any(|s|q.contains(s))||["NHL","National","DEL"].contains(&text){println!("{key} = {text}");}}
 for key in ["NHL","NATIONAL","CHTM_LEAGUE_SWEDISH","CHTM_LEAGUE_FINNISH","CHTM_LEAGUE_GERMAN","CUSTOM TEAMS"]{println!("LOOKUP {key}: {:?}",d.text(key).or_else(||d.rows.iter().find(|r|r.hash==tdb_loc::loc_hash(key)).and_then(|r|r.text.as_deref())));}
 let path=Path::new("F:/NHL Installations/Xbox 360/NHL07/NHL 07 (USA)/fe/ion/main.big");
 if path.is_file(){println!("RESOURCE SCAN {}",path.display());scan("main.big",std::fs::read(path)?,0)?;println!("RESOURCE SCAN complete");}
 Ok(())
}

fn scan(name:&str,mut bytes:Vec<u8>,depth:usize)->Result<(),Box<dyn std::error::Error>>{
 if tdb_archive_codec::is_refpack(&bytes){bytes=tdb_archive_codec::refpack_decompress(&bytes)?;}
 if depth<5 && bytes.starts_with(b"BIG") {
  let a=tdb_archive_big::Archive::parse(bytes)?;
  for (i,e) in a.iter_entries().enumerate(){scan(&format!("{name}/{}",e.name),a.read_entry_at(i)?,depth+1)?;}
 }else{
  let text=String::from_utf8_lossy(&bytes);
  for needle in ["CHTM_LEAGUE","Elitserien","SMLIIGA","LeagueIon","leaguelogo","EasternConference","WesternConference"] {
   if text.contains(needle){println!("RESOURCE MATCH {name}: {needle}");}
  }
 }
 Ok(())
}
