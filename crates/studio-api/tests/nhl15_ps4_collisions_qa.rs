//! Read-only evidence for collisions in the owned PS4 dump; never selects a winner.
use std::{path::Path,io::{Read,Seek,SeekFrom},collections::BTreeMap};
use studio_domain::{extract_service,Platform};
use tdb_archive_big::eb;
#[test]
#[ignore="reads real NHL15 PS4 archives; writes only a new evidence receipt"]
fn compare_colliding_members() {
    let root=Path::new("S:/TEMP/nhl-studio-overnight-20261003/nhl15/ps4-source-routes-1/NHL 15/CUSA00561");
    let pre=extract_service::preflight(root,Platform::Ps4);
    let mut archives=BTreeMap::new();
    for ap in &pre.archives {
        let mut f=std::fs::File::open(&ap.path).unwrap();
        let mut h=vec![0;eb::HEADER_LEN]; f.read_exact(&mut h).unwrap();
        let header=eb::EbHeader::parse(&h).unwrap();
        let mut table=vec![0;header.table_end()]; f.rewind().unwrap(); f.read_exact(&mut table).unwrap();
        archives.insert(ap.path.clone(),(f,eb::EbArchive::parse(&table).unwrap()));
    }
    let mut results=Vec::new();
    for c in &pre.collisions {
        let mut versions=Vec::new();
        for p in &c.archives {
            let (f,a)=archives.get_mut(p).unwrap();
            let e=a.entries.iter().find(|e|e.path.eq_ignore_ascii_case(&c.member)).unwrap();
            let mut raw=vec![0;e.size as usize]; f.seek(SeekFrom::Start(e.offset)).unwrap(); f.read_exact(&mut raw).unwrap();
            let eb::Payload::Data(bytes)=eb::decode_payload(raw).unwrap() else {panic!("unsupported")};
            versions.push(serde_json::json!({"archive":p,"len":bytes.len(),"sha256":studio_domain::baselines::sha256_hex(&bytes)}));
        }
        let equal=versions.iter().all(|v|v["sha256"]==versions[0]["sha256"]);
        results.push(serde_json::json!({"member":c.member,"identical":equal,"versions":versions}));
    }
    let out=Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target/overnight-20261003/nhl15/ps4-collisions-1.json");
    assert!(!out.exists());
    std::fs::write(out,serde_json::to_vec_pretty(&results).unwrap()).unwrap();
    eprintln!("{} collisions: {} byte-identical decoded groups",results.len(),results.iter().filter(|v|v["identical"]==true).count());
}
