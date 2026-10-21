//! Exact-profile exchange and bounded position writer; no game installation.
#[path = "../../../../src/adapters/mcd_profile.rs"]
mod profile;
use serde_json::{Value, json};
use std::{fs, path::Path};
use tdb_rx2::mcd_write::McdContainer;
fn positions(v: &Value) -> Result<Vec<[f32; 3]>, Box<dyn std::error::Error>> {
    let p: Vec<[f32; 3]> = serde_json::from_value(v.clone())?;
    if p.iter().flatten().any(|x| !x.is_finite()) {
        return Err("nonfinite positions".into());
    }
    Ok(p)
}
fn compare_position_streams(
    expected: &McdContainer,
    actual: &McdContainer,
) -> Result<(), &'static str> {
    if expected.streams.len() != actual.streams.len() {
        return Err("stream count mismatch");
    }
    for (a, b) in expected.streams.iter().zip(&actual.streams) {
        if a.pos_off != b.pos_off {
            return Err("position declaration mismatch");
        }
        // Attribute-only streams must never be interpreted as XYZ, even if a
        // future dependency changes how their generic positions cache behaves.
        if a.pos_off.is_some() && a.positions() != b.positions() {
            return Err("positions mismatch");
        }
    }
    Ok(())
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let a: Vec<_> = std::env::args().skip(1).collect();
    if a.len() < 4 {
        return Err("export donor profile output | pack donor profile input output | verify donor profile input final gnf bc1".into());
    }
    let d = fs::read(&a[1])?;
    let p: Value = serde_json::from_slice(&fs::read(&a[2])?)?;
    let mut c = profile::parse_pinned(&d, &p)?;
    if a[0] == "export" {
        if Path::new(&a[3]).exists() {
            return Err("output exists".into());
        }
        let subs:Vec<_>=c.submeshes.iter().enumerate().map(|(i,s)|json!({"name":s.name,"format":s.format,"streams":s.streams,"original_positions":c.streams[c.position_stream_of(i).unwrap()].positions(),"positions":c.streams[c.position_stream_of(i).unwrap()].positions(),"triangles":c.triangles_of(i).unwrap()})).collect();
        fs::write(
            &a[3],
            serde_json::to_vec_pretty(
                &json!({"donor_sha256":profile::hash(&d),"submeshes":subs,"layout":profile::layout(&c)}),
            )?,
        )?;
        return Ok(());
    }
    let input: Value = serde_json::from_slice(&fs::read(&a[3])?)?;
    if input["donor_sha256"] != profile::hash(&d) {
        return Err("wrong fit origin".into());
    }
    let subs = input["submeshes"].as_array().ok_or("submeshes")?;
    if subs.len() != c.submeshes.len() {
        return Err("count mismatch".into());
    }
    let mut names = std::collections::BTreeSet::new();
    for m in subs {
        let name = m["name"].as_str().ok_or("name")?;
        if !names.insert(name) {
            return Err("duplicate name".into());
        }
        let i = c.submesh_index(name).ok_or("name absent")?;
        let j = c.position_stream_of(i).ok_or("positions absent")?;
        if c.streams[j].positions() != positions(&m["original_positions"])?
            || serde_json::to_value(c.triangles_of(i).unwrap())? != m["triangles"]
        {
            return Err("origin/topology mismatch".into());
        }
        c.set_positions(j, &positions(&m["positions"])?)?;
        if !c.refresh_bounds(i) {
            return Err("bounds absent".into());
        }
    }
    if a[0] == "pack" {
        if a.len() != 5 || Path::new(&a[4]).exists() {
            return Err("bad args/output exists".into());
        }
        let edited = c.serialize();
        let mask = profile::position_bounds_mask(&c);
        if d.iter()
            .zip(&edited)
            .enumerate()
            .any(|(i, (x, y))| x != y && !mask[i])
        {
            return Err("protected byte changed".into());
        }
        fs::write(&a[4], edited)?;
        return Ok(());
    }
    if a[0] == "verify" && a.len() == 7 {
        let data = fs::read(&a[4])?;
        let check = McdContainer::parse(&data)?;
        if check.serialize() != data || profile::layout(&check) != profile::layout(&c) {
            return Err("reparse/layout mismatch".into());
        }
        compare_position_streams(&c, &check)?;
        let tex = tdb_rx2::gnf::parse(&fs::read(&a[5])?)?;
        if tex.width != 1024 || tex.height != 1024 || tex.base_mip_linear != fs::read(&a[6])? {
            return Err("GNF decode mismatch".into());
        }
        println!(
            "VERIFIED NHL21 native final positions/layout/no-op and independent GNF base decode"
        );
        return Ok(());
    }
    Err("bad mode".into())
}

#[cfg(test)]
mod stream_tests {
    use super::*;
    #[test]
    fn eye_attribute_edit_is_not_a_position_edit_but_eye_xyz_is_checked() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let d = fs::read(
            root.join("local/assets/codex/nhl21-profile-01/game_90__11580__aaron_ekblad.mcd"),
        )
        .expect("required native fixture");
        let c = McdContainer::parse(&d).unwrap();
        let eye = c.submesh_index("geo_eyesShape").unwrap();
        let xyz = c.position_stream_of(eye).unwrap();
        let attributes = c.submeshes[eye].streams[1];
        assert_eq!(c.streams[attributes].pos_off, None);
        assert!(c.streams[attributes].positions().is_empty());
        let mut edited = d.clone();
        let off = c.streams[attributes].data_off;
        edited[off..off + 2].copy_from_slice(&0x3800u16.to_le_bytes()); // finite f16 normal X
        let shading = McdContainer::parse(&edited).unwrap();
        assert!(compare_position_streams(&c, &shading).is_ok());
        let off = c.streams[xyz].data_off;
        let x = f32::from_le_bytes(edited[off..off + 4].try_into().unwrap());
        edited[off..off + 4].copy_from_slice(&(x + 0.001).to_le_bytes());
        let changed_xyz = McdContainer::parse(&edited).unwrap();
        assert!(compare_position_streams(&c, &changed_xyz).is_err());
    }
}
