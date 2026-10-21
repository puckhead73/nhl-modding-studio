//! Verify patch provenance and exported texture decoding through the Studio API.
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use studio_domain::head_library as library;
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: head_library_patch_smoke QUERY NEW_EXPORT_FOLDER".into());
    }
    let hits = library::search(&args[1], "head", 0)?;
    for head in hits.as_array().ok_or("Invalid search result")? {
        let id = head["id"].as_str().ok_or("Missing asset ID")?;
        let details = library::detail(id)?;
        for appearance in details.as_array().ok_or("Invalid detail")? {
            if appearance["original"]["layer"] != "Patch" {
                continue;
            }
            let output = Path::new(&args[2]);
            library::export(
                id,
                appearance["id"].as_i64().ok_or("Missing appearance")?,
                output,
            )?;
            for texture in appearance["textures"]
                .as_array()
                .ok_or("Missing textures")?
            {
                let file = texture["file"].as_str().ok_or("Missing texture file")?;
                library::texture_rgba(&library::file(file)?)?;
            }
            let manifest: serde_json::Value =
                serde_json::from_slice(&std::fs::read(output.join("provenance.json"))?)?;
            assert_eq!(manifest["selected"]["original"]["layer"], "Patch");
            println!(
                "Patch search, source provenance, GLB/DDS export and preview decoding passed: {}",
                head["stem"]
            );
            return Ok(());
        }
    }
    Err("No matching patch appearance found".into())
}
