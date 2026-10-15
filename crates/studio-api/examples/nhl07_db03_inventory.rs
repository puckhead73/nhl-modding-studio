use std::path::Path;
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
 let root=Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
 let tmp=tempfile::tempdir()?; let reg=Registry::default();
 let store=BaselineStore::open(&root.join("baselines/INVENTORY.json"), &root.join("anchors"))?;
 let opened=reg.project_create_fresh(&tmp.path().join("project").to_string_lossy(),"nhl07",Platform::Xbox,"",None,None,&store,&root.join("defs").to_string_lossy())?;
 let id=opened.summary.project_id;let players=reg.list_players(id)?;
 let mut views=Vec::new();
 for p in &players { if p.player_id==613 || ["Jagr","Luongo","Gaborik","Fernandez"].contains(&p.last_name.as_str()) { views.push(reg.player_view(id,p.player_id)?); } }
 std::fs::write(root.join("target/db03-inventory.json"),serde_json::to_vec_pretty(&views)?)?;
 reg.close_project(id); Ok(())
}
