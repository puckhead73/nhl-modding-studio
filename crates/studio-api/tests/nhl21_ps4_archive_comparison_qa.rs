//! Read-only comparison of differing installed/package archive directories.
use std::{path::Path,io::{Read,Seek},collections::BTreeMap};
use tdb_archive_big::eb;
fn entries(path:&Path)->BTreeMap<String,(u64,u32)> {
    let mut f=std::fs::File::open(path).unwrap();let mut head=vec![0;eb::HEADER_LEN];f.read_exact(&mut head).unwrap();
    let h=eb::EbHeader::parse(&head).unwrap();let mut table=vec![0;h.table_end()];f.rewind().unwrap();f.read_exact(&mut table).unwrap();
    eb::EbArchive::parse(&table).unwrap().entries.into_iter().map(|e|(e.path,(e.offset,e.size))).collect()
}
#[test]
#[ignore="reads installed and PKG NHL21 game_90 tables; writes only a new QA receipt"]
fn compare_game_archive_directories() {
    let package=entries(Path::new("F:/TEMP/nhl-studio-overnight-20261003/nhl21/pkg-source-routes-1/CUSA18182/databigs/game_90.big"));
    let installed=entries(Path::new("H:/Emulators/ShadPS4/Games/CUSA18182/databigs/game_90.big"));
    let mut differences=Vec::new();
    for (name,value) in &package {if installed.get(name)!=Some(value) {differences.push(serde_json::json!({"member":name,"package":value,"installed":installed.get(name)}));}}
    for (name,value) in &installed {if !package.contains_key(name) {differences.push(serde_json::json!({"member":name,"package":null,"installed":value}));}}
    let out=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/overnight-20261003/nhl21/game90-directory-comparison-1.json");assert!(!out.exists());
    std::fs::write(out,serde_json::to_vec_pretty(&serde_json::json!({"package_members":package.len(),"installed_members":installed.len(),"different_directory_entries":differences,"payload_equality_not_asserted":true})).unwrap()).unwrap();
    eprintln!("package {} / installed {} members; {} differing directory entries",package.len(),installed.len(),differences.len());
}
