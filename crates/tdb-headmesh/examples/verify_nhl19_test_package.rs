//! Independent production-codec/container checks for a prepared NHL19 test package.
use serde_json::Value;
use std::{error::Error, path::Path};
use tdb_rx2::{
    gnf, mcd,
    mcd_write::{fingerprint, McdContainer},
};
fn main() -> Result<(), Box<dyn Error>> {
    let arg = std::env::args()
        .nth(1)
        .ok_or("usage: verify_nhl19_test_package package-directory")?;
    let root = Path::new(&arg);
    let report: Value = serde_json::from_slice(&std::fs::read(root.join("package.json"))?)?;
    let mesh: Value = serde_json::from_slice(&std::fs::read(root.join("native_mesh.json"))?)?;
    for arc in report["archives"].as_array().ok_or("archives")? {
        for change in arc["changes"].as_array().ok_or("changes")? {
            let packed =
                std::fs::read(root.join(change["stored_file"].as_str().ok_or("stored_file")?))?;
            let decoded = tdb_archive_codec::decompress_chunkzip(&packed)?;
            assert_eq!(
                fingerprint(&decoded),
                change["decoded_sha256"].as_str().unwrap()
            );
            assert_eq!(
                decoded,
                std::fs::read(root.join(change["decoded_file"].as_str().unwrap()))?
            );
            if decoded.starts_with(b"RSF\0") {
                let c = McdContainer::parse(&decoded)?;
                let display = mcd::parse_player_mesh(&decoded)?;
                for m in mesh["submeshes"].as_array().unwrap() {
                    let name = m["name"].as_str().unwrap();
                    let i = c.submesh_index(name).ok_or("mesh")?;
                    let positions: Vec<[f32; 3]> = serde_json::from_value(m["positions"].clone())?;
                    let tris: Vec<[u32; 3]> = serde_json::from_value(m["triangles"].clone())?;
                    assert_eq!(
                        c.streams[c.position_stream_of(i).unwrap()].positions(),
                        positions
                    );
                    assert_eq!(c.triangles_of(i).unwrap(), tris);
                    let expected: Vec<[f32; 3]> = serde_json::from_value(m["normals"].clone())?;
                    let got = display.submeshes.iter().find(|s| s.name == name).unwrap();
                    for (v, n) in got.vertices.iter().zip(expected) {
                        for (a, b) in v.normal.iter().zip(n) {
                            assert!((a - b).abs() < 0.001);
                        }
                    }
                }
            } else {
                gnf::parse(&decoded)?;
            }
            println!(
                "PASS {}: production chunkzip + decoded asset",
                change["stored_file"]
            );
        }
    }
    let diffuse = report["assets"]["textures"]["diffuse_file"]
        .as_str()
        .unwrap_or("head_jack_eichel_cm.gnf");
    let bytes = std::fs::read(root.join(diffuse))?;
    let word = |off| u32::from_le_bytes(bytes[off..off + 4].try_into().unwrap());
    let base = gnf::parse(&bytes)?;
    assert_eq!(word(12) as usize, bytes.len());
    assert_eq!(word(44) as usize, bytes.len() - 256);
    assert_eq!(((word(32) >> 13) & 16383) + 1, base.width);
    assert_eq!(
        ((word(28) >> 16) & 15) as usize + 1,
        report["assets"]["textures"]["mips"]
            .as_array()
            .unwrap()
            .len()
    );
    for mip in report["assets"]["textures"]["mips"].as_array().unwrap() {
        let level = mip["level"].as_u64().unwrap();
        let w = mip["width"].as_u64().unwrap() as u32;
        let h = mip["height"].as_u64().unwrap() as u32;
        let offset = mip["offset"].as_u64().unwrap() as usize;
        let size = mip["tiled_size"].as_u64().unwrap() as usize;
        let mut view = bytes[..256].to_vec();
        view[0x18..0x1c].copy_from_slice(&((w - 1) | ((h - 1) << 14)).to_le_bytes());
        view.extend_from_slice(&bytes[offset..offset + size]);
        let tex = gnf::parse(&view)?;
        assert_eq!(
            tex.base_mip_linear,
            std::fs::read(root.join(format!("diffuse_mip_{level}.bc1")))?
        );
        println!("PASS GNF mip {level}: {w}x{h} production detile");
    }
    Ok(())
}
