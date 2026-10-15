//! LEAGUE-SMOKE-1 / LEAGUE-RESP-1 — the commands the shell, the Leagues screens, Pending Edits,
//! the copied-target panel and the project picker dispatch must run **off Tauri's main thread**,
//! on the blocking pool. A plain `#[tauri::command]` on a sync `fn` runs on the main thread and
//! holds the window for as long as it waits on the registry's mutexes (the desktop smoke measured
//! a 32 s freeze on Close during an apply's preflight); `#[tauri::command(async)]` on a sync body
//! runs it inline on a runtime worker instead of a blocking thread. The wanted shape is
//! `pub async fn … { off_main(app, …).await }`.
//!
//! This is a *source* guard: the Tauri runtime cannot be exercised here. It is not the evidence
//! that contention is resolved — that is the registry-level barrier regressions in
//! `copied_target_contention.rs` and the desktop measurements in the package receipt. It only
//! keeps a later edit from silently turning one of these back into a main-thread command.

const OFF_MAIN_THREAD: &[&str] = &[
    // shell / close
    "close_project",
    "project_summary",
    "validate_project",
    "set_edit_override",
    "set_display_roster",
    "autocorrect_jerseys",
    "repair_roster_blocks",
    "save_workspace",
    // picker (close → reopen)
    "detect_install",
    "open_workspace",
    "baseline_inventory",
    "project_create_fresh",
    "project_import_review",
    "project_create_import",
    "project_open",
    "project_archive_preview",
    "project_import_archive",
    "project_info",
    "project_save",
    "project_set_extraction",
    "project_deploy_plan",
    "project_deploy_apply",
    "project_deploy_restore",
    "project_deployments",
    "extract_preflight",
    "extract_start",
    "extract_status",
    "extract_cancel",
    "extract_running_for",
    "pkg_extract_start",
    // ONB-003
    "project_set_view",
    "project_relink",
    "package_inventory",
    "package_build",
    "package_inspect",
    "package_preflight",
    "package_apply",
    "package_installs",
    "package_restore",
    // leagues
    "list_leagues",
    "team_schedule_edit",
    "swap_team_memberships",
    "league_view",
    "attach_dictionary",
    "set_field",
    "player_view",
    "player_assignments",
    "set_player_assignment",
    "list_arenas",
    // pending edits
    "pending_changes",
    "undo",
    "redo",
    "save_project",
    // copied target
    "copied_target_root_for",
    "copied_target_create_root",
    "copied_target_review",
    "copied_target_review_archive_member",
    "copied_target_apply",
    "copied_target_cancel",
    "copied_target_discard",
    "copied_target_status",
    "copied_target_recover",
    "copied_target_restore",
    "copied_target_reload",
];

#[test]
fn affected_commands_run_on_the_blocking_pool() {
    let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands.rs"))
        .expect("commands.rs is readable");
    let lines: Vec<&str> = src.lines().collect();
    for name in OFF_MAIN_THREAD {
        let sync_decl = format!("pub fn {name}(");
        assert!(
            !lines.iter().any(|l| l.trim_start().starts_with(&sync_decl)),
            "{name} is declared as a sync command (main thread); it must be `pub async fn`"
        );
        let decl = format!("pub async fn {name}(");
        let at = lines
            .iter()
            .position(|l| l.trim_start().starts_with(&decl))
            .unwrap_or_else(|| panic!("{name} is not declared `pub async fn` in commands.rs"));
        let attr = lines[..at]
            .iter()
            .rev()
            .find(|l| l.trim_start().starts_with("#[tauri::command"))
            .unwrap_or_else(|| panic!("{name} carries no #[tauri::command] attribute"));
        assert_eq!(
            attr.trim(),
            "#[tauri::command]",
            "{name}: an async fn takes the plain attribute"
        );
        // The body forwards through `off_main` (spawn_blocking) before the next command starts.
        let end = lines[at..]
            .iter()
            .position(|l| l.starts_with("#[tauri::command") || l.starts_with("// ----"))
            .map(|n| at + n)
            .unwrap_or(lines.len());
        let body = lines[at..end].join("\n");
        assert!(
            body.contains("off_main(app,") && body.contains(".await"),
            "{name} must forward through off_main(app, …).await; body:\n{body}"
        );
        assert!(
            !body.contains("State<'_, Registry>"),
            "{name} must not borrow State into the pool (use the AppHandle)"
        );
    }
    // The helper itself uses the blocking pool, not a runtime worker.
    let helper = src
        .find("async fn off_main<")
        .expect("off_main is declared");
    let helper_body = &src[helper..helper + 600];
    assert!(
        helper_body.contains("tauri::async_runtime::spawn_blocking("),
        "off_main must use spawn_blocking"
    );
}
