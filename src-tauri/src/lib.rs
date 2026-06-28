//! NHL Modding Studio — Tauri app shell (Phase 2).
//!
//! Intentionally thin: it manages the [`studio_api::Registry`] state and registers the command
//! surface. All DTOs, conversion, and orchestration live in `studio-api` / `studio-domain`; the
//! TypeScript bindings are generated from `studio-api` (`cargo test -p studio-api`).

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(studio_api::Registry::default())
        .invoke_handler(studio_api::command_handler!())
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
