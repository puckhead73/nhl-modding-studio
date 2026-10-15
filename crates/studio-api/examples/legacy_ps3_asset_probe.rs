//! Read-only Legacy PS3 asset survey. Exports and replacement trials stay in a temp tree.
use std::path::{Path, PathBuf};
use studio_domain::{assets, AssetRoots, Platform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let root = PathBuf::from(std::env::var_os("NHLMS_PS3_INSTALL").expect("USRDIR fixture root"));
    let tmp = tempfile::tempdir()?;
    let mut failures = Vec::new();
    let mut previews = 0;
    let mut replacements = 0;
    // Each FE family has a different container/alpha/mip profile.
    for folder in [
        "teamlogos",
        "teamlogoscalendar",
        "teamlogosdynasty",
        "teamlogosreflection",
        "teamlogossmall",
        "teamlogoswide",
        "leaguelogos",
        "leaguelogosbig",
        "leaguelogossmall",
        "leaguelogoswide",
        "playerheads/p0_4000",
        "jerseys",
    ] {
        let dir = root.join("fe/ion/artassets").join(folder);
        let mut files: Vec<_> = std::fs::read_dir(&dir)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "big"))
            .collect();
        files.sort();
        let mut count = 0;
        for (i, path) in files.iter().enumerate() {
            // These measured packages contain Apt drawing records, not bitmap rasters.
            if folder == "jerseys"
                && ["custom.big", "customhome.big", "customaway.big"]
                    .iter()
                    .any(|n| path.file_name().unwrap() == *n)
            {
                assert!(studio_domain::decode_texture_file(path, None).is_err());
                println!("{}: interface drawing package (no bitmap)", path.display());
                continue;
            }
            match studio_domain::decode_texture_file(path, None) {
                Ok((_, img)) => {
                    previews += 1;
                    count += 1;
                    if i == 0 {
                        replacement(path, None, &img)?;
                        replacements += 1;
                    }
                }
                Err(e) => {
                    eprintln!("FAIL {}: {e}", path.display());
                    failures.push(format!("{}: {e}", path.display()));
                }
            }
        }
        println!("FE {folder}: {count}/{} previews", files.len());
    }
    for folder in [
        "player",
        "jersey",
        "pant",
        "sock",
        "logo",
        "helmet",
        "glove",
        "skate",
        "playerstick",
        "goaliehelmet",
        "goaliepad",
        "blocker",
        "trapper",
    ] {
        let dir = root.join("rendering").join(folder);
        let mut files: Vec<_> = std::fs::read_dir(&dir)?
            .flatten()
            .map(|e| e.path())
            .filter(|p| {
                p.extension().is_some_and(|e| e == "rpsgl")
                    && p.file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with("texlib")
            })
            .collect();
        files.sort();
        // First and last exercise naming variants without exporting a whole installation.
        for path in files
            .first()
            .into_iter()
            .chain(files.last())
            .collect::<std::collections::HashSet<_>>()
        {
            let (_, img) = studio_domain::decode_texture_file(path, None)?;
            previews += 1;
            replacement(path, None, &img)?;
            replacements += 1;
            let rel = path
                .strip_prefix(&root)?
                .to_string_lossy()
                .replace('\\', "/");
            let result = assets::export_asset_files(
                &root,
                &[rel],
                tmp.path(),
                assets::FileExportOptions {
                    meshes_glb: true,
                    portraits_png: true,
                    textures: true,
                    copy_unrecognized: false,
                    crop_portraits: false,
                },
            )?;
            assert!(result.failures.is_empty(), "{:?}", result.failures);
            assert!(result.textures_written > 0);
        }
        println!(
            "rendering {folder}: {} texture bundles available",
            files.len()
        );
    }
    let roots = AssetRoots::resolve(&root, Platform::Ps3);
    for art in [0, 1, 2, 10, 20, 29] {
        assert!(
            studio_domain::decode_team_logo(&roots, art).is_some(),
            "team crest {art}"
        );
        let mut fallback = roots.clone();
        fallback.fe.clear();
        assert!(
            studio_domain::decode_team_logo(&fallback, art).is_some(),
            "rendering crest {art}"
        );
    }
    for failure in &failures {
        eprintln!("FAIL {failure}");
    }
    assert!(failures.is_empty(), "{} failed previews", failures.len());
    println!(
        "PASS: {previews} previews, {replacements} replacement/redecode checks, logo fallbacks"
    );
    Ok(())
}

fn replacement(
    path: &Path,
    index: Option<usize>,
    img: &studio_domain::DecodedImage,
) -> Result<(), Box<dyn std::error::Error>> {
    let original = std::fs::read(path)?;
    let rgba = [255u8, 0, 255, 255].repeat((img.width * img.height) as usize);
    let plan = studio_domain::plan_texture_replace(&original, img.width, img.height, index)?;
    assert!(
        plan.refusal.is_none(),
        "{}: {:?}",
        path.display(),
        plan.refusal
    );
    let edited =
        studio_domain::replace_texture_in_file(original, &rgba, img.width, img.height, index)?;
    let (_, decoded) = studio_domain::decode_texture_bytes(edited.bytes, path, index)?;
    assert_eq!((decoded.width, decoded.height), (img.width, img.height));
    assert!(decoded
        .rgba
        .chunks_exact(4)
        .all(|p| p[0] > 240 && p[1] < 15 && p[2] > 240 && p[3] > 240));
    Ok(())
}
