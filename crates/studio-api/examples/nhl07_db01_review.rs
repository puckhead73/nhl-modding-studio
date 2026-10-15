//! Prepare an untouched, isolated review project through the same API as onboarding.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::{BaselineStore, Platform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let out = PathBuf::from(std::env::args().nth(1).ok_or("Pass a new project directory")?);
    if out.exists() { return Err("Review destination already exists; nothing replaced".into()); }
    let store = BaselineStore::open(&repo.join("baselines/INVENTORY.json"), &repo.join("anchors"))?;
    let reg = Registry::default();
    let opened = reg.project_create_fresh(&out.to_string_lossy(), "nhl07", Platform::Xbox,
        "", None, None, &store, &repo.join("defs").to_string_lossy())?;
    reg.project_save(opened.summary.project_id)?;
    reg.close_project(opened.summary.project_id);
    println!("Created vanilla review project: {}", out.display());
    Ok(())
}
