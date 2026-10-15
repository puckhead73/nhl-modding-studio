use std::path::Path;
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir()?;
    let reg = Registry::default();
    let store = BaselineStore::open(
        &root.join("baselines/INVENTORY.json"),
        &root.join("anchors"),
    )?;
    let opened = reg.project_create_fresh(
        &tmp.path().join("p").to_string_lossy(),
        "nhl07",
        Platform::Xbox,
        "",
        None,
        None,
        &store,
        &root.join("defs").to_string_lossy(),
    )?;
    let id = opened.summary.project_id;
    let tables = reg.schedule_tables(id)?;
    let views = tables
        .iter()
        .map(|t| reg.schedule_view(id, &t.studio_table))
        .collect::<Result<Vec<_>, _>>()?;
    let data = serde_json::json!({"tables":tables,"views":views,"membership":reg.team_membership_options(id,14)?,"leagues":reg.list_leagues(id)?,"team":reg.team_view(id,14)?});
    std::fs::write(
        root.join("target/db04-inventory.json"),
        serde_json::to_vec_pretty(&data)?,
    )?;
    reg.close_project(id);
    Ok(())
}
