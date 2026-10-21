//! Exercise the same library functions used by Studio against a compiled corpus.
use std::path::Path;
fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    use studio_domain::head_library as library;
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: head_library_smoke NEW_EXPORT_DIRECTORY".into());
    }
    let status = library::status()?;
    assert_eq!(status["available"], true);
    let heads = library::search("aaron ekblad", "head", 0)?;
    let head = heads
        .as_array()
        .and_then(|a| a.first())
        .ok_or("Expected Ekblad corpus fixture")?;
    let id = head["id"].as_str().unwrap();
    let appearances = library::detail(id)?;
    let selected = &appearances[0];
    let mut companions = Vec::new();
    for class in ["hair", "facialhair"] {
        let matches = library::search("", class, 0)?;
        companions.push(
            matches[0]["id"]
                .as_str()
                .ok_or("Missing companion")?
                .to_owned(),
        );
    }
    library::export_composition(
        id,
        selected["id"].as_i64().unwrap(),
        &companions,
        Path::new(&args[1]),
    )?;
    for folder in ["", "hair", "facialhair"] {
        let root = Path::new(&args[1]).join(folder);
        let manifest: serde_json::Value =
            serde_json::from_slice(&std::fs::read(root.join("provenance.json"))?)?;
        assert!(root.join(manifest["mesh"].as_str().unwrap()).is_file());
        for texture in manifest["selected"]["textures"].as_array().unwrap() {
            let bytes = std::fs::read(root.join(texture["file"].as_str().unwrap()))?;
            library::texture_rgba(&bytes)?;
        }
    }
    assert!(library::file("../headmeshes.sqlite").is_err());
    assert!(library::export(id, selected["id"].as_i64().unwrap(), Path::new(&args[1])).is_err());
    println!("Search, source selection, composed export, DDS previews, containment and collision checks passed.");
    Ok(())
}
