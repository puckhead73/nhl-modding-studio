//! Qualify native parsing against privately decoded samples; no game data is bundled.
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::args().nth(1).ok_or("expected sample directory")?);
    let samples: serde_json::Value =
        serde_json::from_slice(&std::fs::read(root.join("samples.json"))?)?;
    let mut report = Vec::new();
    for sample in samples.as_array().ok_or("invalid sample list")? {
        let name = sample["file"].as_str().ok_or("missing file")?;
        let metadata = sample["resource_metadata"]
            .as_str()
            .ok_or("missing metadata")?;
        let source = std::fs::read(root.join(name))?;
        let lods = tdb_frostbite::mesh::parse(&source, metadata, |guid| {
            std::fs::read(root.join(format!("{guid}.chunk")))
        })?;
        for lod in lods {
            let output = format!("{name}_lod{}.glb", lod.level);
            let bytes = tdb_frostbite::mesh::glb(&lod)?;
            std::fs::write(root.join(&output), &bytes)?;
            report.push(
                serde_json::json!({"source":name,"output":output,"bytes":bytes.len(),
                "lod":lod.level,"sections":lod.sections.len(),
                "vertices":lod.sections.iter().map(|s|s.positions.len()).sum::<usize>(),
                "triangles":lod.sections.iter().map(|s|s.indices.len()/3).sum::<usize>()}),
            );
        }
    }
    std::fs::write(
        root.join("native-mesh-results.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{} LOD GLBs qualified", report.len());
    Ok(())
}
