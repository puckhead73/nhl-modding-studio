//! Captures the **actual** toolchain identity at compile time so that
//! [`tdb_qa::evidence::emit::toolchain_identity`] can report it, and reports nothing when it cannot
//! be resolved.
//!
//! Contract §5.3 folds `rustc-version` and `target-triple` into the `shared-core` freshness digest,
//! and R-FRESH-UNKNOWN requires an unresolvable identity to make that digest the literal `unknown`
//! rather than a guessed value. Emitting no `cargo:rustc-env` line is exactly what produces that:
//! `option_env!` then yields `None`.
//!
//! Nothing here reads a clock, a commit or an environment the build did not already have.

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Cargo sets TARGET for a build script to the triple the crate is being compiled for.
    if let Ok(target) = std::env::var("TARGET") {
        if !target.is_empty() {
            println!("cargo:rustc-env=QA_TARGET_TRIPLE={target}");
        }
    }

    // `rustc -vV` line 1 is the canonical version string, e.g.
    // `rustc 1.96.0 (ac68faa20 2026-05-25)`. RUSTC is set by cargo to the compiler in use.
    let rustc = std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string());
    let version = std::process::Command::new(rustc)
        .arg("-vV")
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .and_then(|text| text.lines().next().map(str::trim).map(str::to_string))
        .filter(|line| !line.is_empty());
    if let Some(version) = version {
        println!("cargo:rustc-env=QA_RUSTC_VERSION={version}");
    }
}
