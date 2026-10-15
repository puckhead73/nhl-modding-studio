//! Creates an isolated, ready-to-build DB03 review project; never writes to the game source.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = PathBuf::from(std::env::args().nth(1).ok_or("Pass a new project directory")?);
    let source = std::env::args().nth(2).unwrap_or_default();
    if out.exists() { return Err("Review destination already exists; nothing replaced".into()); }
    let store = BaselineStore::open(&repo.join("baselines/INVENTORY.json"), &repo.join("anchors"))?;
    let reg = Registry::default();
    let opened = reg.project_create_fresh(&out.to_string_lossy(), "nhl07", Platform::Xbox,
        &source, None, None, &store, &repo.join("defs").to_string_lossy())?;
    let id = opened.summary.project_id;
    reg.project_set_details(id, studio_api::dto::ProjectDetailsDto { name: "DB03 Review".into(), version: "1.0.0".into(), author: "NHL Modding Studio QA".into(), description: "Demitra: Jagr head, portrait, commentary and gear. Fernandez: Luongo pads, trapper, blocker and stick. Database assignments only.".into() })?;
    for (player, field, choice) in [
        (613,"head","404:1:0:0"),(613,"portrait","404"),(613,"audio","404:370"),
        (613,"stick","2"),(613,"helmet","0:0"),(613,"gloves","0"),(613,"skates","7"),(613,"visor","1"),
        (734,"pads","15:0"),(734,"trapper","15:0"),(734,"blocker","15:0"),(734,"stick","0"),
    ] { reg.set_player_assignment(id,player,field,choice)?; }
    reg.project_save(id)?;
    let catalogue = serde_json::json!({"demitra":reg.player_assignments(id,613)?,"fernandez":reg.player_assignments(id,734)?,"overview":reg.project_overview(id)?});
    std::fs::write(repo.join("target/db03-review-catalogue.json"),serde_json::to_vec_pretty(&catalogue)?)?;
    let package = reg.package_build(id,&studio_api::dto::PackageBuildRequestDto {
        name:"DB03 Review".into(), version:"1.0.0".into(), description:"Controlled NHL07 assignment review".into(),
        include:vec!["cells".into()],exclude:vec![],platforms:vec!["xbox".into()],out_path:Some(out.join("exports/DB03 Review_xbox360_nhl07.nhlmod").to_string_lossy().into_owned()),
    },Some(&store),&repo.join("defs").to_string_lossy())?;
    reg.close_project(id);
    println!("Project: {}\nBuild: {}",out.display(),package.path);
    Ok(())
}
