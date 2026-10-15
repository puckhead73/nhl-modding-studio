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
    let arenas=reg.list_arenas(id)?;
    let views=arenas.iter().map(|a|reg.arena_view(id,a.arena_id)).collect::<Result<Vec<_>,_>>()?;
    let bytes=std::fs::read(root.join("anchors/xbox360/nhl07/databases/NHL07NG.db"))?;
    let db=tdb_core::TdbFile::parse(&bytes)?;
    let counts=["StEO","cubz","SulW","vFSP","KFxR"].iter().map(|name|(*name,db.table_canonical(name).map(|t|t.current_records))).collect::<std::collections::BTreeMap<_,_>>();
    let data=serde_json::json!({"record_counts":counts,"arenas":arenas,"views":views,"venue":reg.arena_venue_view(id,14,&root.join("defs"),None)?,"team":reg.team_view(id,14)?});
    std::fs::write(
        root.join("target/db06-inventory.json"),
        serde_json::to_vec_pretty(&data)?,
    )?;
    reg.close_project(id);
    Ok(())
}
