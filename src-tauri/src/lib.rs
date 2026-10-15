//! NHL Modding Studio — Tauri app shell (Phase 2).
//!
//! Intentionally thin: it manages the [`studio_api::Registry`] state and registers the command
//! surface. All DTOs, conversion, and orchestration live in `studio-api` / `studio-domain`; the
//! TypeScript bindings are generated from `studio-api` (`cargo test -p studio-api`).

use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // Select the private runtime before Tauri creates any threads or WebView environments.
    #[cfg(windows)]
    configure_bundled_webview().unwrap_or_else(|e| report_startup_failure(&e));
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(studio_api::Registry::default())
        .invoke_handler(studio_api::command_handler!())
        .build(tauri::generate_context!())
        .unwrap_or_else(|e| report_startup_failure(&e))
        .run(|app, event| {
            // Normal exit: abort prepared copied-target sets and remove their staging (S2-C-R2).
            // Anything that cannot be removed is reported, never silently left behind.
            if let tauri::RunEvent::Exit = event {
                for l in app.state::<studio_api::Registry>().teardown_copied_target() {
                    eprintln!(
                        "copied-target staging left behind: {} ({}: {})",
                        l.path, l.kind, l.error
                    );
                }
            }
        });
}

#[cfg(windows)]
fn configure_bundled_webview() -> std::io::Result<()> {
    use std::os::windows::process::CommandExt;
    let exe = std::env::current_exe()?;
    let runtime = exe.parent().unwrap().join("webview2");
    if !runtime.is_dir() {
        return Ok(()); // Development/minimal builds retain installed-runtime support.
    }
    if !runtime.join("msedgewebview2.exe").is_file() {
        return Err(std::io::Error::other("The bundled WebView2 runtime is incomplete. Extract the entire archive again."));
    }
    // Microsoft requires read/execute access for these AppContainer SIDs on Windows 10.
    // Grant only within our bundled runtime; no machine installation or elevation is needed.
    let icacls = std::path::PathBuf::from(std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into()))
        .join("System32").join("icacls.exe");
    let output = std::process::Command::new(icacls)
        .arg(&runtime).args(["/grant", "*S-1-15-2-2:(OI)(CI)(RX)", "*S-1-15-2-1:(OI)(CI)(RX)"])
        .creation_flags(0x08000000).output()?;
    if !output.status.success() {
        return Err(std::io::Error::other(format!("Could not prepare the bundled runtime's read permissions. Extract Studio to a local folder you own. {}", String::from_utf8_lossy(&output.stderr))));
    }
    std::env::set_var("WEBVIEW2_BROWSER_EXECUTABLE_FOLDER", runtime);
    Ok(())
}

/// The startup failure text (receipt 46). The release build has no console, so a failure to create the
/// window would otherwise end the process without a word. The WebView2 hint is a possible cause, not a
/// diagnosis: the underlying error is always shown first.
pub fn startup_failure_message(detail: &str) -> String {
    format!(
        "NHL Modding Studio could not start its window.\n\n{detail}\n\n\
         Studio needs the Microsoft Edge WebView2 Runtime. It is part of Windows 11 and is installed on most \
         Windows 10 PCs. If it is missing, install the Evergreen WebView2 Runtime from \
         https://developer.microsoft.com/microsoft-edge/webview2/ and start Studio again."
    )
}

fn report_startup_failure(e: &impl std::fmt::Display) -> ! {
    let text = startup_failure_message(&e.to_string());
    eprintln!("{text}");
    #[cfg(windows)]
    {
        use windows::core::PCWSTR;
        use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
        let wide = |s: &str| s.encode_utf16().chain(std::iter::once(0)).collect::<Vec<u16>>();
        let (t, c) = (wide(&text), wide("NHL Modding Studio"));
        // SAFETY: both buffers are NUL-terminated UTF-16 and outlive the call; no owner window.
        unsafe {
            MessageBoxW(None, PCWSTR(t.as_ptr()), PCWSTR(c.as_ptr()), MB_OK | MB_ICONERROR);
        }
    }
    std::process::exit(1)
}

#[cfg(test)]
mod tests {
    #[test]
    fn startup_failure_names_the_error_first_and_the_webview2_route() {
        let m = super::startup_failure_message("failed to create webview: WebView2 runtime not found");
        assert!(m.starts_with("NHL Modding Studio could not start its window.\n\nfailed to create webview"));
        assert!(m.contains("https://developer.microsoft.com/microsoft-edge/webview2/"));
    }
}
