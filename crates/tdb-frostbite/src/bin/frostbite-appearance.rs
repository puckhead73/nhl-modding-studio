//! Read-only appearance export / corpus qualification using an external decoder.
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().collect();
    if a.len() < 6 {
        return Err("usage: frostbite-appearance ROOT Data|Patch nhl22|nhl23|nhl24 NEW_OUTPUT OODLE_DLL [--validate-only]".into());
    }
    let report = tdb_frostbite::appearance::export(
        Path::new(&a[1]),
        &a[2],
        &a[3],
        Path::new(&a[4]),
        Path::new(&a[5]),
        a.iter().any(|s| s == "--validate-only"),
    )?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    if !report.failures.is_empty() {
        std::process::exit(1);
    }
    Ok(())
}
