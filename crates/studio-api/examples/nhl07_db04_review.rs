//! Prepare a separate review project through production APIs; never deploy to the live game.
use std::path::Path;
use studio_api::{
    dto::{PackageBuildRequestDto, ProjectDetailsDto},
    Registry,
};
use studio_domain::{schedule_edit::GameRow, BaselineStore, Platform};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()?;
    let root = repo.join("local/DB04 Review 2026-10-08");
    let deliverables = repo.join("local/DB04 Review Deliverables");
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
    reg.project_set_details(id,ProjectDetailsDto{name:"DB04 Review".into(),version:"1.0.0".into(),author:"DB04 QA".into(),description:"Minnesota moves to Central; St. Louis moves to Northwest. Minnesota–Colorado moves from Oct 5 to Oct 6. Minnesota hosts St. Louis on Oct 8. Only the NHL 82-game non-leap schedule changes.".into()})?;
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
    let original = reg.schedule_export(
        id,
        "ksGb",
        "csv",
        Some(&exports.join("original.csv").to_string_lossy()),
    )?;
    let invalid = original.text.lines().take(2).collect::<Vec<_>>().join("\n")
        + "\n0,Feb 30,2,30,MIN,STL,0,0,14,24,Minnesota,St. Louis\n";
    std::fs::write(exports.join("invalid-date.csv"), invalid)?;
    reg.set_team_membership(id, 14, 0, Some(0), Some(3))?;
    reg.set_team_membership(id, 24, 0, Some(0), Some(4))?;
    let snapshot = |v: &studio_api::dto::ScheduleViewDto| {
        v.games
            .iter()
            .map(|g| GameRow {
                index: g.index,
                month: g.month,
                day: g.day,
                home_slot: g.home_slot,
                away_slot: g.away_slot,
                round: g.round,
                status: g.status,
            })
            .collect::<Vec<_>>()
    };
    let view = reg
        .schedule_view(id, "ksGb")?
        .ok_or("Schedule unavailable")?;
    let changed = reg.team_schedule_edit(
        id,
        "ksGb",
        14,
        &view.fingerprint,
        Some(9),
        false,
        Some(7),
        true,
        10,
        6,
        &snapshot(&view),
    )?;
    reg.team_schedule_edit(
        id,
        "ksGb",
        14,
        &changed.fingerprint,
        None,
        false,
        Some(24),
        true,
        10,
        8,
        &snapshot(&changed),
    )?;
    reg.project_save(id)?;
    reg.schedule_export(
        id,
        "ksGb",
        "csv",
        Some(&exports.join("review.csv").to_string_lossy()),
    )?;
    reg.package_build(
        id,
        &PackageBuildRequestDto {
            name: "DB04 Review".into(),
            version: "1.0.0".into(),
            description: "Controlled DB04 native verification".into(),
            include: vec!["cells".into()],
            exclude: vec![],
            platforms: vec!["xbox".into()],
            out_path: Some(
                exports
                    .join("DB04_Review_xbox360_nhl07.nhlmod")
                    .to_string_lossy()
                    .into_owned(),
            ),
        },
        Some(&store),
        &repo.join("defs").to_string_lossy(),
    )?;
    reg.project_archive(id, &deliverables.to_string_lossy())?;
    let overview = reg.project_overview(id)?;
    if !overview.notices.is_empty() {
        return Err(format!("Unexpected overview notices: {:?}", overview.notices).into());
    }
    reg.close_project(id);
    println!("Review project: {}", root.display());
    Ok(())
}
