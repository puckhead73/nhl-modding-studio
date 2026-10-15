//! Prepare an exhibition-only NHL07 native test through production project/build/install APIs.
//! Usage: NEW_PROJECT INSTALL_ROOT PACKAGE INSTALL_RECORDS PREVIOUS_NATIVE_TEST_PROJECT
//! Preserves the original QATest; restores the prior eight-field experiment before installation.
use std::path::{Path, PathBuf};
use studio_api::{Registry, dto::PackageBuildRequestDto};
use studio_domain::{BaselineStore, Platform, tdb_txn::FileIdentity};

fn text(path: &Path) -> String { path.to_string_lossy().into_owned() }
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 6 { return Err("usage: NEW_PROJECT INSTALL_ROOT PACKAGE INSTALL_RECORDS PREVIOUS_NATIVE_TEST_PROJECT".into()); }
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let defs = text(&repo.join("defs"));
    let store = BaselineStore::open(&repo.join("baselines/INVENTORY.json"), &repo.join("anchors"))?;
    let reg = Registry::default();
    let opened = reg.project_create_fresh(&args[1], "nhl07", Platform::Xbox, &args[2], None, None, &store, &defs)?;
    let id = opened.summary.project_id;
    reg.move_player(id, 613, 14, 28)?;
    reg.set_jersey_number(id, 28, 613, 91)?;
    let pending = reg.pending_changes(id, 100)?;
    assert_eq!(pending.primary.total_cells, 4);
    assert!(pending.roster.is_none());
    assert!(reg.project_save(id)?.saved);
    reg.close_project(id);
    let opened = reg.project_open(&args[1], &defs)?;
    let id = opened.summary.project_id;
    let player = reg.player_view(id, 613)?.ok_or("player missing after reopen")?;
    assert!(player.memberships.iter().any(|m| m.team_id == 28 && m.jersey == Some(91)));
    assert!(player.memberships.iter().any(|m| m.team_id == 48 && m.jersey == Some(38)));
    let built = reg.package_build(id, &PackageBuildRequestDto {
        name: "Demitra Exhibition Test".into(), version: "1".into(), description: "NHL07 native table qualification: exhibition Vancouver #91, stock/default Minnesota #38".into(),
        include: vec!["cells".into()], exclude: vec![], platforms: vec!["xbox".into()], out_path: Some(args[3].clone()),
    }, Some(&store), &defs)?;
    assert_eq!(reg.package_inspect(&built.path)?.cells.len(), 4);
    // Remove the previous experiment's stock/default edits through its recorded restore.
    let dest = PathBuf::from(&args[2]).join("db/NHL07NG.db");
    let original = std::fs::read(repo.join("anchors/xbox360/nhl07/databases/NHL07NG.db"))?;
    let installed = FileIdentity::of_file(&dest)?;
    if installed.sha256 == "50851d694a431262a5a9278cce88f987b9b597871349cd464db55a03ca2bf3cb" {
        let prior = reg.project_open(&args[5], &defs)?;
        let restored = reg.project_deploy_restore(prior.summary.project_id, 1)?;
        assert_eq!(restored.state, "restored");
        reg.close_project(prior.summary.project_id);
        println!("PREVIOUS_TEST_RESTORED {}", serde_json::to_string(&restored)?);
    }
    assert_eq!(std::fs::read(&dest)?, original, "installation is not the verified vanilla baseline; nothing further will be installed");
    let plan = reg.package_preflight(&built.path, "nhl07", Platform::Xbox, &args[2], None, None, false, &args[4], &defs)?;
    println!("INSTALL_PLAN {}", serde_json::to_string(&plan)?);
    if !plan.installable { return Err("package preflight refused; installation remains vanilla".into()); }
    assert_eq!(plan.applied, 4);
    let applied = reg.package_apply(plan.handle)?;
    assert_eq!(applied.state, "committed");
    let working = opened.project.working_main_db.as_ref().ok_or("missing working database")?;
    assert_eq!(std::fs::read(&dest)?, std::fs::read(working)?);
    let receipt = serde_json::json!({
        "test":"exhibition-only Vancouver #91; stock/default Minnesota #38",
        "project":args[1],"package":built.path,"installation":args[2],
        "installed_identity":FileIdentity::of_file(&dest)?,"install":applied,
        "memberships":player.memberships,"pending":pending,
        "native_verified":false,
        "instructions":"Cold launch extracted default.xex, Restore Default Rosters, report Demitra team and number. Vancouver #91 means exhibition-only changes are sufficient for this path; Minnesota #38 means stock/default copies need further qualification."
    });
    std::fs::write(PathBuf::from(&args[1]).join("native-test-receipt.json"),serde_json::to_vec_pretty(&receipt)?)?;
    println!("RESULT {}", serde_json::to_string(&receipt)?);
    reg.close_project(id);
    Ok(())
}
