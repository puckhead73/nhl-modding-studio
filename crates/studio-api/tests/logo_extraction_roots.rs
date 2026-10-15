//! Logo discovery must distinguish loose extraction paths from archive members.
use std::path::{Path,PathBuf};
use studio_api::Registry;
use studio_domain::Platform;
fn repo(p:&str)->PathBuf {Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p)}
#[test]
fn ps4_logos_keep_archive_names_when_extraction_is_nested() {
    let root=repo("target/logo-root-tests").join(format!("{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&root).unwrap();
    let a=repo("anchors/ps4/nhl15/databases");
    let reg=Registry::default();
    let id=reg.open_workspace_from_paths(Some("nhl15"),Platform::Ps4,&a.join("nhl_game.db").to_string_lossy(),None,&a.join("nhl_game-meta.xml").to_string_lossy(),&repo("defs").to_string_lossy()).unwrap().project_id;
    let league="nhl/ui/assets/leaguelogos/size64/l0.png";
    let team=reg.team_logo_set(id,0,None).unwrap().unwrap().variants[0].rel_path.clone();
    // Independently cover Studio's beside-archive layout and older Extracted layouts.
    for prefix in ["databigs/Extracted","databigs","Extracted",""] {
        let picked=root.join(prefix.replace('/',"_"));
        for member in [league,team.as_str()] {
            let file=picked.join(prefix).join(member);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(file,b"existence-only fixture").unwrap();
        }
        let l=reg.league_logo(id,0,Some(&picked.to_string_lossy())).unwrap().unwrap();
        let t=reg.team_logo_set(id,0,Some(&picked.to_string_lossy())).unwrap().unwrap();
        for (v,member) in [(&l.variants[0],league),(&t.variants[0],team.as_str())] {
            assert_eq!(v.exists,Some(true));
            assert_eq!(v.rel_path,member,"archive member must not gain extraction prefix");
            assert_eq!(v.loose_rel_path.as_deref(),Some(if prefix.is_empty(){member.to_string()}else{format!("{prefix}/{member}")}.as_str()));
            assert_eq!(v.archive.as_deref(),Some("databigs/inxp10.big"));
        }
    }
    assert!(studio_domain::league_logos::loose_logo_path(&root,"../outside.png").is_none());
}
