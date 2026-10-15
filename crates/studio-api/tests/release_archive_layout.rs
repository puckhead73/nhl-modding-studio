//! Receipt 43 — the extracted release archive resolves its resources the way Studio does, without the
//! repository: `defs\` beside the executable (the rule `resolve_defs` applies to Tauri's resource
//! directory, which for this no-installer layout is the executable's folder), readable enough to open a
//! title and name its teams. Select with `--ignored` and set `NHLMS_RELEASE_ROOT` to the extracted root;
//! a missing root fails, never skips. The executable is not launched.
use std::path::{Path, PathBuf};

use studio_domain::{EditSession, Platform, Project, ProjectRole};

#[test]
#[ignore = "an extracted release archive named by NHLMS_RELEASE_ROOT: select with --ignored"]
fn the_extracted_archive_resolves_defs_beside_the_executable() {
    let root = PathBuf::from(std::env::var_os("NHLMS_RELEASE_ROOT").expect("NHLMS_RELEASE_ROOT is not set"));
    assert!(root.join("nhl-studio.exe").is_file(), "{}: no nhl-studio.exe", root.display());
    let defs = studio_api::commands::bundled_defs(&root).unwrap_or_else(|| panic!("{}: no defs\\enums beside the executable", root.display()));
    assert_eq!(defs, root.join("defs"));
    assert!(studio_api::commands::bundled_defs(&root.join("helpers")).is_none(), "a folder without defs\\enums is not taken");
    // Open a committed anchor with the archive's defs (the repository's own defs are not used).
    let anchors = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../anchors/xbox360/nhllegacy/databases");
    let project = Project::load(anchors.join("nhlng.db"), Platform::Xbox, ProjectRole::Nhlng, anchors.join("nhlng-meta.xml"), &defs).unwrap();
    let mut s = EditSession::begin(project);
    s.set_game_id(Some("nhllegacy".into()));
    let teams = s.team_summaries().unwrap();
    assert!(teams.len() > 30 && teams.iter().any(|t| t.name.contains("Maple Leafs")), "teams named through the archive's defs");
    eprintln!("defs: {} ({} teams named; cwd {})", defs.display(), teams.len(), std::env::current_dir().unwrap().display());

    // ONB-002: the bundled baselines resolve beside the executable too, verify against their
    // inventory, and a fresh project can be created from them without the repository's anchors.
    let baselines = studio_api::commands::bundled_baselines(&root).unwrap_or_else(|| panic!("{}: no baselines\\INVENTORY.json beside the executable", root.display()));
    let store = studio_domain::BaselineStore::open_bundled(&baselines).unwrap().unwrap();
    assert_eq!(store.files_root, baselines.join("files"));
    assert!(store.verify_all().is_empty(), "every bundled baseline file matches its inventory hash");
    assert_eq!(store.inventory.targets.len(), 27);
    let unavailable: Vec<&str> = store.inventory.targets.iter().filter(|t| !t.is_available()).map(|t| t.title.as_str()).collect();
    assert_eq!(unavailable, vec!["NHL 20 (PS4)"], "the NHL 19-base alias is never bundled as NHL 20");
    let scratch = std::env::temp_dir().join(format!("nhlms-release-baselines-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();
    let reg = studio_api::Registry::default();
    let opened = reg
        .project_create_fresh(&scratch.join("p").to_string_lossy(), "nhllegacy", Platform::Xbox, &scratch.join("no-install").to_string_lossy(), None, None, &store, &defs.to_string_lossy())
        .unwrap();
    assert!(!opened.summary.roster_attached, "fresh mod projects edit the default database without attaching a roster save");
    assert!(std::path::Path::new(opened.project.working_main_db.as_deref().unwrap()).starts_with(&scratch));
    eprintln!("baselines: {} ({} targets; {} unavailable)", baselines.display(), store.inventory.targets.len(), unavailable.len());
    let _ = std::fs::remove_dir_all(&scratch);
}
