//! Read-only headless entry point for the same installation check used by Studio.
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("Usage: game-file-check <manifest.json> <game-folder>".into());
    }
    let manifest: studio_domain::game_manifest::Manifest =
        serde_json::from_reader(std::fs::File::open(&args[1])?)?;
    let report = studio_domain::game_manifest::check(
        std::path::Path::new(&args[2]),
        &manifest,
        &manifest.game_id,
        &manifest.platform,
    );
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}
