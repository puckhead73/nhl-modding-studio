//! Re-export explicitly listed native MCDs to new GLBs, never touching sources.
use serde_json::{json, Value};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: repair_mcd_exports requests.json new-report.json".into());
    }
    let requests: Vec<Value> = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let mut report = Vec::new();
    for item in requests {
        let source = item["source"].as_str().ok_or("source")?;
        let output = item["output"].as_str().ok_or("output")?;
        let result = (|| -> Result<Value, Box<dyn std::error::Error>> {
            let bytes = std::fs::read(source)?;
            let mesh = tdb_rx2::mcd::parse_player_mesh_preview(&bytes)?;
            use std::io::Write;
            std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(output)?
                .write_all(&tdb_rx2::playermesh::write_glb_geometry_only(&mesh))?;
            Ok(
                json!({"submeshes":mesh.submeshes.iter().map(|s|json!({"name":s.name,"vertices":s.vertices.len(),"triangles":s.triangles.len()})).collect::<Vec<_>>()}),
            )
        })();
        report.push(json!({"source":source,"output":output,"result":result.as_ref().ok(),"error":result.as_ref().err().map(|e|e.to_string())}));
    }
    std::fs::write(&args[2], serde_json::to_vec_pretty(&report)?)?;
    let failures = report.iter().filter(|r| !r["error"].is_null()).count();
    println!("{} exports; {failures} failures", report.len());
    if failures > 0 {
        return Err("Some exports failed; inspect report".into());
    }
    Ok(())
}
