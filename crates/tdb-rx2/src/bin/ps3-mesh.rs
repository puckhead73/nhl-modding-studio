//! Native PS3 mesh interchange. Output must be a new file; source is never overwritten.
use std::{io::Write, path::Path};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    if a.len() < 3 {
        return Err("Usage: ps3-mesh export input.rpsgl output.obj|glb OR ps3-mesh import original.rpsgl edited.obj output.rpsgl".into());
    }
    let source = std::fs::read(&a[1])?;
    let (out, data) = match a[0].as_str() {
        "export" if a.len() == 3 => {
            let d = tdb_rx2::ps3_mesh::parse(&source)?;
            let data = match Path::new(&a[2]).extension().and_then(|e| e.to_str()) {
                Some("obj") => tdb_rx2::ps3_mesh::export_obj(&d.asset).into_bytes(),
                Some("glb") => tdb_rx2::playermesh::write_glb_geometry_only(&d.asset),
                _ => return Err("Export must end in .obj or .glb".into()),
            };
            (&a[2], data)
        }
        "import" if a.len() == 4 => {
            let text = std::fs::read_to_string(&a[2])?;
            (&a[3], tdb_rx2::ps3_mesh::import_obj(&source, &text)?)
        }
        _ => return Err("Invalid command or argument count".into()),
    };
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(out)?;
    f.write_all(&data)?;
    f.sync_all()?;
    println!("Wrote {} bytes to {out}", data.len());
    Ok(())
}
