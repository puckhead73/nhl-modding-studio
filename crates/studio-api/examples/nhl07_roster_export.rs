//! Export the requested native test DB through Studio's project edit/save workflow.
//! Usage: nhl07_roster_export PROJECT OUTPUT_DIRECTORY
//! Does not build a package or deploy to the game installation.
use std::path::PathBuf;
use studio_api::Registry;
use studio_domain::tdb_txn::FileIdentity;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 { return Err("usage: PROJECT OUTPUT_DIRECTORY".into()); }
    let out = PathBuf::from(&args[2]);
    let output_db = out.join("NHL07NG.db");
    if output_db.exists() { return Err("output already exists; choose a new output directory".into()); }
    let defs = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../defs").to_string_lossy().into_owned();
    let registry = Registry::default();
    let opened = registry.project_open(&args[1], &defs)?;
    let id = opened.summary.project_id;
    assert_eq!(opened.project.game_id, "nhl07");
    assert_eq!(opened.project.platform, "xbox");
    assert!(!opened.summary.roster_attached);
    let before = registry.player_view(id, 613)?.ok_or("Demitra missing")?;
    let player = registry.list_players(id)?.into_iter().find(|p| p.player_id == 613).ok_or("Demitra missing")?;
    assert_eq!((&*player.first_name, &*player.last_name), ("Pavol", "Demitra"));
    if player.team_id != Some(28) {
        assert_eq!(player.team_id, Some(14), "unexpected original team");
        registry.move_player(id, 613, 14, 28)?;
    }
    registry.set_jersey_number(id, 28, 613, 90)?;
    let save = registry.project_save(id)?;
    registry.close_project(id);
    // Reopen the saved project, not the in-memory edit, before exporting.
    let reopened = registry.project_open(&args[1], &defs)?;
    let id = reopened.summary.project_id;
    let after = registry.player_view(id, 613)?.ok_or("Demitra missing after reopen")?;
    assert!(after.memberships.iter().any(|m| m.team_id == 28 && m.jersey == Some(90)));
    assert!(!after.memberships.iter().any(|m| m.team_id == 14));
    assert_eq!(serde_json::to_value(before.memberships.iter().filter(|m| m.team_id == 48).collect::<Vec<_>>())?,
        serde_json::to_value(after.memberships.iter().filter(|m| m.team_id == 48).collect::<Vec<_>>())?);
    let source = reopened.project.working_main_db.as_ref().ok_or("missing saved database")?;
    let bytes = std::fs::read(source)?;
    let parsed = tdb_core::TdbFile::parse(&bytes)?;
    assert!(parsed.header_crc_ok());
    assert_eq!(parsed.serialize(), bytes);
    std::fs::create_dir_all(&out)?;
    {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new().create_new(true).write(true).open(&output_db)?;
        file.write_all(&bytes)?;
        file.sync_all()?;
    }
    assert_eq!(std::fs::read(&output_db)?, bytes);
    let receipt = serde_json::json!({
        "project":args[1], "source":source, "export":output_db,
        "identity":FileIdentity::of_bytes(&bytes), "project_save":save,
        "memberships":after.memberships,
        "workflow":"Studio project_open -> set_jersey_number -> project_save -> close/reopen -> export saved database",
        "game_installation_modified":false, "package_created":false,
        "scope":"NHL07 Xbox 360; exhibition bio/roster team and number only. Stock/default tables unchanged."
    });
    std::fs::write(out.join("receipt.json"), serde_json::to_vec_pretty(&receipt)?)?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    registry.close_project(id);
    Ok(())
}
