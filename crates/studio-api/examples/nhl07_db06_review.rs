//! Prepare a separate review project through production APIs; never deploy to the live game.
use std::path::Path;
use studio_api::{
    dto::{PackageBuildRequestDto, ProjectDetailsDto},
    Registry,
};
use studio_domain::{FieldValue, BaselineStore, Platform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let root = repo.join("local/DB06 Review 2026-10-11");
    let deliverables = repo.join("local/DB06 Review Deliverables");
    std::fs::create_dir_all(&deliverables)?;
    if root.exists() {
        if std::env::args().any(|a| a == "--archive-only") {
            let reg = Registry::default();
            let id = reg
                .project_open(
                    &root.to_string_lossy(),
                    &repo.join("defs").to_string_lossy(),
                )?
                .summary
                .project_id;
            assert!(reg.project_overview(id)?.notices.is_empty());
            let archive = reg.project_archive(id, &deliverables.to_string_lossy())?;
            reg.close_project(id);
            println!("Review archive: {}", archive.path);
            return Ok(());
        }
        return Err("The review project already exists; refusing to overwrite it.".into());
    }
    let reg = Registry::default();
    let store = BaselineStore::open(
        &repo.join("baselines/INVENTORY.json"),
        &repo.join("anchors"),
    )?;
    let id = reg
        .project_create_fresh(
            &root.to_string_lossy(),
            "nhl07",
            Platform::Xbox,
            "",
            None,
            None,
            &store,
            &repo.join("defs").to_string_lossy(),
        )?
        .summary
        .project_id;
    reg.project_set_details(id,ProjectDetailsDto{name:"DB06 Review".into(),version:"1.0.0".into(),author:"DB06 QA".into(),description:"Minnesota and St. Louis share DB06 Review Arena. Capacity 19000, attendance 95%. Vancouver goal/period horns and center-ice/jumbotron assignments.".into()})?;
    let source = Path::new(r"F:\NHL Installations\Xbox 360\NHL07\NHL 07 (USA)");
    if source.is_dir() {
        reg.project_relink(
            id,
            Some(&source.to_string_lossy()),
            None,
            None,
            Some(&source.to_string_lossy()),
        )?;
    }
    let exports = root.join("exports");
    std::fs::create_dir_all(&exports)?;
    let arena=reg.arena_view(id,14)?.ok_or("Minnesota venue missing")?;
    for (name,value) in [("arenaname",FieldValue::Text("DB06 Review Arena".into())),("cityname",FieldValue::Text("St. Paul QA".into())),("maxattendance",FieldValue::Int(19000)),("avgattendance",FieldValue::Int(95)),("goalhorn",FieldValue::Int(28)),("periodhorn",FieldValue::Int(28)),("icelogo",FieldValue::Int(28)),("jumbotron",FieldValue::Int(28))] {
        let f=arena.all_fields.iter().find(|f|f.name==name).ok_or("Arena field missing")?;
        reg.set_field(id,&f.studio_table,arena.record,&f.studio_field,value)?;
    }
    let team=reg.team_view(id,24)?.ok_or("St. Louis missing")?;
    let cell=team.profile.sections.iter().flat_map(|s|&s.cells).find(|c|c.label=="Home Arena").ok_or("Home Arena missing")?;
    let f=cell.field.as_ref().ok_or("Home Arena field missing")?;
    reg.set_field(id,&f.studio_table,cell.record,&f.studio_field,FieldValue::Int(14))?;
    reg.project_save(id)?;
    let overview=reg.project_overview(id)?;
    if !overview.notices.is_empty(){return Err(format!("Unexpected notices: {:?}",overview.notices).into());}
    std::fs::write(exports.join("overview.json"),serde_json::to_vec_pretty(&overview)?)?;
    reg.package_build(id,&PackageBuildRequestDto{name:"DB06 Review".into(),version:"1.0.0".into(),description:"Controlled arena verification".into(),include:vec!["cells".into()],exclude:vec![],platforms:vec!["xbox".into()],out_path:Some(exports.join("DB06_Review_xbox360_nhl07.nhlmod").to_string_lossy().into_owned())},Some(&store),&repo.join("defs").to_string_lossy())?;
    reg.project_archive(id,&deliverables.to_string_lossy())?;
    reg.close_project(id);
    println!("Review project: {}",root.display());
    Ok(())
}
