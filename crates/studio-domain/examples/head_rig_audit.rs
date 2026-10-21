//! Read-only native head sampling. No rig is invented for geometry-only readers.
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<_> = std::env::args().collect();
    if args.len() == 4 && args[1] == "--skeleton" {
        let bytes = std::fs::read(&args[2])?;
        let skeleton = tdb_rx2::skeleton::parse_skeleton_ebx(&bytes)?;
        let report = json!({"source":args[2],"sha256":format!("{:x}",Sha256::digest(&bytes)),
            "bones":skeleton.bones.iter().map(|bone|json!({"name":bone.name,
                "model_translation":bone.trans,"model_rotation":bone.rot})).collect::<Vec<_>>(),
            "scope":"Measured model-space bind transforms; hierarchy/animation and per-mesh palette mapping are separate."});
        use std::io::Write;
        std::fs::OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&args[3])?
            .write_all(&serde_json::to_vec_pretty(&report)?)?;
        println!("Read {} named bones", skeleton.bones.len());
        return Ok(());
    }
    if args.len() != 3 {
        return Err("usage: head_rig_audit sources.json new-report.json".into());
    }
    let sources: Vec<Value> = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let mut reports = Vec::new();
    for source in sources {
        let catalog: Value = serde_json::from_slice(&std::fs::read(
            source["catalog"].as_str().ok_or("catalog")?,
        )?)?;
        for entry in catalog["entries"]
            .as_array()
            .ok_or("entries")?
            .iter()
            .filter(|e| e["class"] == "head" && e["origin"] == "player")
            .take(3)
        {
            let path = Path::new(entry["source"].as_str().ok_or("source")?);
            let result = (|| -> Result<Value, String> {
                let raw = std::fs::read(path).map_err(|e| e.to_string())?;
                let mut bytes =
                    tdb_archive_codec::decompress_if_wrapped(&raw).map_err(|e| e.to_string())?;
                if tdb_archive_codec::is_refpack(&bytes) {
                    bytes =
                        tdb_archive_codec::refpack_decompress(&bytes).map_err(|e| e.to_string())?;
                }
                let asset = match path.extension().and_then(|s| s.to_str()) {
                    Some("mcd") => tdb_rx2::mcd::parse_player_mesh_preview(&bytes)
                        .map_err(|e| e.to_string())?,
                    Some("rpsgl") => tdb_rx2::ps3_mesh::parse(&bytes)?.asset,
                    Some("rx2") => tdb_rx2::playermesh::parse_player_mesh_preview(&bytes)
                        .map_err(|e| e.to_string())?,
                    _ => {
                        return Err(
                            "Requires a native source resource, not a logical Frostbite name"
                                .into(),
                        )
                    }
                };
                let mut submeshes = Vec::new();
                for mesh in &asset.submeshes {
                    let mut mass = BTreeMap::<u16, f64>::new();
                    if mesh.has_blend_indices && mesh.has_blend_weights {
                        for vertex in &mesh.vertices {
                            for (&joint, &weight) in vertex.joints.iter().zip(&vertex.weights) {
                                if weight > 0.0 {
                                    *mass.entry(joint).or_default() += f64::from(weight);
                                }
                            }
                        }
                    }
                    submeshes.push(json!({"name":mesh.name,"vertices":mesh.vertices.len(),
                        "triangles":mesh.triangles.len(),"joint_channel":mesh.has_blend_indices,
                        "weight_channel":mesh.has_blend_weights,"raw_joint_weight_mass":mass}));
                }
                Ok(json!({"source_sha256":format!("{:x}",Sha256::digest(&raw)),
                    "decoded_sha256":format!("{:x}",Sha256::digest(&bytes)),
                    "bind_matrices":asset.skin.matrices,"submeshes":submeshes,
                    "note":"Raw joint values retain each native reader's convention. No cross-game bone equivalence is implied."}))
            })();
            reports.push(json!({"game":catalog["game"],"platform":source["platform"],
                "source":path,"result":result.as_ref().ok(),"error":result.as_ref().err()}));
        }
    }
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&args[2])?;
    file.write_all(&serde_json::to_vec_pretty(&reports)?)?;
    println!("Recorded {} native samples", reports.len());
    Ok(())
}
