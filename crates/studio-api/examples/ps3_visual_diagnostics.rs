//! Read-only diagnostics for PS3 nameplate and head preview fixtures.
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::var_os("NHLMS_PS3_INSTALL").expect("USRDIR"));
    let out = PathBuf::from("target/ps3-visual-diagnostics");
    std::fs::create_dir_all(&out)?;
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir()?;
    let reg = studio_api::Registry::default();
    let store = studio_domain::BaselineStore::open(&repo.join("baselines/INVENTORY.json"), &repo.join("anchors"))?;
    let id = reg.project_create_fresh(&tmp.path().join("p").to_string_lossy(), "nhllegacy", studio_domain::Platform::Ps3, "", None, None, &store, &repo.join("defs").to_string_lossy())?.summary.project_id;
    for p in reg.list_players(id)?.into_iter().filter(|p| p.team_id == Some(0)) {
        let v = reg.player_view(id, p.player_id)?.unwrap();
        println!("{} {}: head {:?}", p.first_name, p.last_name, v.all_fields.iter().find(|f| f.studio_field == "pPaD").and_then(|f|f.decoded));
    }
    let result = studio_domain::assets::export_asset_files(&root,
        &["rendering/jersey/texlib_1_0_2.rpsgl".into(), "rendering/player/texlib_380.rpsgl".into()], &out,
        studio_domain::assets::FileExportOptions { meshes_glb: false, portraits_png: true, textures: true, copy_unrecognized: false, crop_portraits: false })?;
    println!("export failures {:?}", result.failures);
    let image = image::open(out.join("rendering/player/texlib_380_cm_1.png"))?.into_rgba8();
    let transparent = image.pixels().filter(|p| p[3] < 128).count();
    println!("head alpha cutouts {transparent}");
    let mesh = root.join("rendering/player/player_380.rpsgl");
    let preview = studio_domain::head_preview::decode_head_preview(&[mesh], &[root.join("rendering/player/texlib_380.rpsgl").to_string_lossy().into_owned()], &[])?;
    let xbox = PathBuf::from(r"X:\Games\Emulation\Roms\XBOX360\ISOs\Archives\NHL Legacy\NHL Legacy Edition (USA, Europe) (En,Fr,De,Ru)\rendering\player\player_380.rx2");
    if xbox.is_file() {
        let other = studio_domain::head_preview::decode_head_preview(&[xbox], &[], &[])?;
        for sm in preview.submeshes.iter().filter(|s|s.name.contains("headShape")) {
            let mut bad_uv=0; let mut bad_normal=0; let mut matched=0;
            for (i,p) in sm.positions.iter().enumerate() {
                let mut best = f32::INFINITY; let mut pair = None;
                for x in other.submeshes.iter().filter(|s|s.name.to_lowercase().contains("head")) {
                    for (j,q) in x.positions.iter().enumerate() {
                        let d: f32 = (0..3).map(|k|(p[k]-q[k]).powi(2)).sum();
                        if d < best {best=d;pair=Some((x,j));}
                    }
                }
                if best < 0.000001 { let (x,j)=pair.unwrap(); matched+=1;
                    if i < 4 { println!("normal sample {:?} PS3 {:?} Xbox {:?}",p,sm.normals[i],x.normals[j]); }
                    if (0..2).any(|k|(sm.uv1[i][k]-x.uv1[j][k]).abs()>0.01) {bad_uv+=1;}
                    if (0..3).any(|k|(sm.normals[i][k]-x.normals[j][k]).abs()>0.1) {bad_normal+=1;}
                }
            }
            println!("Xbox comparison {} matched {matched} UV mismatch {bad_uv} normal mismatch {bad_normal}",sm.name);
        }
    }
    for sm in &preview.submeshes {
        let min = sm.uv1.iter().flatten().copied().fold(f32::INFINITY, f32::min);
        let max = sm.uv1.iter().flatten().copied().fold(f32::NEG_INFINITY, f32::max);
        println!("{}: {} verts; UV {min}..{max}; material {:?}", sm.name, sm.positions.len(), sm.material_index.map(|i| &preview.textures[i as usize].name));
        let mut bad = 0;
        for tri in &sm.triangles {
            let a = sm.positions[tri[0] as usize]; let b = sm.positions[tri[1] as usize]; let c = sm.positions[tri[2] as usize];
            let u = [b[0]-a[0], b[1]-a[1], b[2]-a[2]]; let v = [c[0]-a[0], c[1]-a[1], c[2]-a[2]];
            let n = [u[1]*v[2]-u[2]*v[1], u[2]*v[0]-u[0]*v[2], u[0]*v[1]-u[1]*v[0]];
            let dot: f32 = tri.iter().map(|i| { let x=sm.normals[*i as usize]; x[0]*n[0]+x[1]*n[1]+x[2]*n[2] }).sum();
            if dot < 0.0 { bad += 1; }
        }
        println!("opposing normals {bad}/{}", sm.triangles.len());
        let bbox: Vec<_> = (0..3).map(|a| (sm.positions.iter().map(|v|v[a]).fold(f32::INFINITY,f32::min), sm.positions.iter().map(|v|v[a]).fold(f32::NEG_INFINITY,f32::max))).collect();
        println!("bounds {bbox:?}");
    }
    Ok(())
}


