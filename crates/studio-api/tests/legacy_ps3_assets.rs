//! Real PS3 containers through project replacement, persistence, packaging and restoration.
//! The installation fixture is read only; every write targets a disposable test directory.
use std::path::{Path, PathBuf};
use studio_api::{dto::PackageBuildRequestDto, Registry};
use studio_domain::{BaselineStore, Platform};

fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}
fn copy(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst.parent().unwrap()).unwrap();
    std::fs::copy(src, dst).unwrap();
}

#[test]
#[ignore = "requires NHLMS_PS3_INSTALL (Legacy USRDIR); source is read only"]
fn legacy_ps3_texture_project_build_install_restore() {
    let fixture = PathBuf::from(std::env::var_os("NHLMS_PS3_INSTALL").unwrap());
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let temp = tempfile::tempdir().unwrap();
    let author = temp.path().join("author/USRDIR");
    let recipient = temp.path().join("recipient/USRDIR");
    let files = [
        "fe/ion/artassets/teamlogos/t2.big",
        "fe/ion/artassets/leaguelogos/l0.big",
        "rendering/jersey/texlib_0_2_0.rpsgl",
        "rendering/player/texlib_66.rpsgl",
    ];
    for root in [&author, &recipient] {
        std::fs::create_dir_all(root).unwrap();
        std::fs::write(root.join("EBOOT.BIN"), b"owned discovery fixture").unwrap();
        for name in ["nhlng.db", "nhlng-meta.xml", "nhl_eng_us.db"] {
            copy(
                &repo.join("anchors/ps3/nhllegacy/databases").join(name),
                &root.join("data/db").join(name),
            );
        }
        for rel in files {
            copy(&fixture.join(rel), &root.join(rel));
        }
        copy(
            &fixture.join("rendering/player/player_66.rpsgl"),
            &root.join("rendering/player/player_66.rpsgl"),
        );
    }
    let originals: Vec<_> = files
        .iter()
        .map(|rel| std::fs::read(fixture.join(rel)).unwrap())
        .collect();
    let reg = Registry::default();
    let store = BaselineStore::open(
        &repo.join("baselines/INVENTORY.json"),
        &repo.join("anchors"),
    )
    .unwrap();
    let defs = s(&repo.join("defs"));
    let project = temp.path().join("project");
    let id = reg
        .project_create_fresh(
            &s(&project),
            "nhllegacy",
            Platform::Ps3,
            &s(&author),
            None,
            None,
            &store,
            &defs,
        )
        .unwrap()
        .summary
        .project_id;
    let roots = studio_api::dto::AssetRootsArgsDto {
        install_dir: s(&author),
        platform: Some("ps3".into()),
        rendering_override: None,
        fe_override: None,
    };
    let original_logo = reg.team_logo(id, &roots, 2).unwrap().unwrap();
    let original_head = reg.head_preview(&roots, 66).unwrap();
    let mut previews = Vec::new();
    for (i, rel) in files.iter().enumerate() {
        let before = reg.texture_preview(&s(&author), rel, 0, None).unwrap();
        let color = [255, 30 + i as u8 * 50, 180, 255];
        let img = studio_domain::DecodedImage {
            width: before.width,
            height: before.height,
            rgba: color.repeat((before.width * before.height) as usize),
        };
        let source = temp.path().join(format!("source{i}.png"));
        std::fs::write(&source, img.to_png().unwrap()).unwrap();
        let plan = reg
            .texture_replace_plan(&s(&author), rel, &s(&source), None)
            .unwrap();
        reg.import_texture(&s(&author), rel, &s(&source), false, None, plan.review_id)
            .unwrap();
        let after = reg.texture_preview(&s(&author), rel, 0, None).unwrap();
        assert_ne!(before.data_url, after.data_url);
        previews.push(after.data_url);
        assert_eq!(
            std::fs::read(author.join(rel)).unwrap(),
            originals[i],
            "author install changed"
        );
    }
    assert_ne!(
        reg.team_logo(id, &roots, 2).unwrap().unwrap(),
        original_logo,
        "header must show the project logo"
    );
    let edited_head = reg.head_preview(&roots, 66).unwrap();
    assert!(
        edited_head
            .textures
            .iter()
            .zip(&original_head.textures)
            .any(|(a, b)| a.rgba_base64 != b.rgba_base64),
        "head preview must read edited raster"
    );
    let direct_head = reg
        .head_preview_paths(
            vec![s(&author.join("rendering/player/player_66.rpsgl"))],
            vec![s(&author.join("rendering/player/texlib_66.rpsgl"))],
        )
        .unwrap();
    assert!(direct_head
        .textures
        .iter()
        .zip(&edited_head.textures)
        .all(|(a, b)| a.rgba_base64 == b.rgba_base64));
    // A second raster in one container must survive alongside the first through a build.
    let rel = files[3];
    let raster = reg.texture_preview(&s(&author), rel, 0, Some(0)).unwrap();
    let second = temp.path().join("second.png");
    std::fs::write(
        &second,
        studio_domain::DecodedImage {
            width: raster.width,
            height: raster.height,
            rgba: [0u8, 255, 255, 255].repeat((raster.width * raster.height) as usize),
        }
        .to_png()
        .unwrap(),
    )
    .unwrap();
    let plan = reg
        .texture_replace_plan(&s(&author), rel, &s(&second), Some(0))
        .unwrap();
    reg.import_texture(
        &s(&author),
        rel,
        &s(&second),
        false,
        Some(0),
        plan.review_id,
    )
    .unwrap();
    previews[3] = reg
        .texture_preview(&s(&author), rel, 0, None)
        .unwrap()
        .data_url;
    let expected: Vec<_> = files
        .iter()
        .map(|rel| {
            std::fs::read(
                project
                    .join(studio_domain::project_store::WORKING_DIR)
                    .join(studio_domain::project_store::OVERLAY_DIR)
                    .join(rel),
            )
            .unwrap()
        })
        .collect();
    reg.project_save(id).unwrap();
    assert!(reg.close_project(id));
    let id = reg
        .project_open(&s(&project), &defs)
        .unwrap()
        .summary
        .project_id;
    for (i, rel) in files.iter().enumerate() {
        assert_eq!(
            reg.texture_preview(&s(&author), rel, 0, None)
                .unwrap()
                .data_url,
            previews[i]
        );
    }
    let package = reg
        .package_build(
            id,
            &PackageBuildRequestDto {
                name: "Legacy PS3 asset QA".into(),
                version: "1".into(),
                description: String::new(),
                include: vec!["textures".into()],
                exclude: vec![],
                platforms: vec!["ps3".into()],
                out_path: Some(s(&temp.path().join("test.nhlmod"))),
            },
            Some(&store),
            &defs,
        )
        .unwrap();
    reg.close_project(id);
    let plan = reg
        .package_preflight(
            &package.path,
            "nhllegacy",
            Platform::Ps3,
            &s(&recipient),
            None,
            None,
            false,
            &s(&temp.path().join("records")),
            &defs,
        )
        .unwrap();
    assert!(plan.installable, "{:?}", plan.reasons);
    let applied = reg.package_apply(plan.handle).unwrap();
    assert_eq!(applied.state, "committed");
    for (i, rel) in files.iter().enumerate() {
        assert_eq!(
            reg.texture_preview(&s(&recipient), rel, 0, None)
                .unwrap()
                .data_url,
            previews[i]
        );
        assert_eq!(
            std::fs::read(recipient.join(rel)).unwrap(),
            expected[i],
            "all raster edits must be replayed"
        );
    }
    assert_eq!(
        reg.package_restore(&applied.record_dir).unwrap().state,
        "restored"
    );
    for (i, rel) in files.iter().enumerate() {
        assert_eq!(std::fs::read(recipient.join(rel)).unwrap(), originals[i]);
        assert_eq!(std::fs::read(fixture.join(rel)).unwrap(), originals[i]);
    }
}

#[test]
#[ignore = "requires NHLMS_PS3_INSTALL (Legacy USRDIR); source is read only"]
fn legacy_ps3_head_export_catalog_and_equipment_lookup() {
    use studio_domain::assets::{self, AssetRoots, PlayerAssets};
    let root = PathBuf::from(std::env::var_os("NHLMS_PS3_INSTALL").unwrap());
    let temp = tempfile::tempdir().unwrap();
    let roots = AssetRoots::resolve(&root, Platform::Ps3);
    let mesh = roots.mesh_path(66, None).unwrap();
    let textures: Vec<String> = roots.texture_paths(66, None).iter().map(|p| s(p)).collect();
    assert!(!textures.is_empty());
    let player = PlayerAssets {
        bio_record: 0,
        full_name: "Fixture Player".into(),
        slug: Some("fixture_player_01_01_1980".into()),
        portrait_id: 66,
        head_id: 66,
        bespoke_head: true,
        pool_head: false,
        head_shared: false,
        portrait_path: None,
        portrait_small_path: None,
        mesh_path: Some(s(&mesh)),
        texture_paths: textures.clone(),
        generic_portrait: false,
    };
    let summary = assets::export_player_assets(
        &[player.clone()],
        &temp.path().join("player"),
        assets::ExportOptions {
            portraits: false,
            ..Default::default()
        },
    )
    .unwrap();
    assert!(summary.failures.is_empty(), "{:?}", summary.failures);
    assert_eq!(summary.meshes_written, 1);
    assert!(summary.textures_written > 1);
    assert!(temp
        .path()
        .join("player/heads/fixture_player_01_01_1980.rpsgl")
        .is_file());
    assert!(!temp
        .path()
        .join("player/heads/fixture_player_01_01_1980.rx2")
        .exists());
    let catalog = studio_domain::export_player_head_catalog(
        &[player],
        &temp.path().join("catalog"),
        &studio_domain::CatalogOptions {
            game_tag: "nhllegacy".into(),
            textures: true,
        },
    )
    .unwrap();
    assert!(catalog.failures.is_empty(), "{:?}", catalog.failures);
    assert!(catalog.heads_written > 0 && catalog.hair_written > 0 && catalog.textures_written > 1);
    assert!(!studio_domain::textures_for_source(&mesh).is_empty());
    let exported = assets::export_asset_files(
        &root,
        &["rendering/player/texlib_66.rpsgl".into()],
        &temp.path().join("files"),
        assets::FileExportOptions {
            meshes_glb: false,
            portraits_png: false,
            textures: true,
            copy_unrecognized: false,
            crop_portraits: false,
        },
    )
    .unwrap();
    assert!(exported.failures.is_empty(), "{:?}", exported.failures);
    let files = std::fs::read_dir(temp.path().join("files/rendering/player"))
        .unwrap()
        .count();
    assert_eq!(
        files, exported.textures_written,
        "same-suffix rasters must not overwrite one another"
    );
    // A minimal PS3-only tree catches both census filtering and non-player sweep filtering.
    let minimal = temp.path().join("minimal");
    copy(&mesh, &minimal.join("rendering/player/player_66.rpsgl"));
    for texture in textures {
        copy(
            Path::new(&texture),
            &minimal
                .join("rendering/player")
                .join(Path::new(&texture).file_name().unwrap()),
        );
    }
    let minimal_roots = AssetRoots::resolve(&minimal, Platform::Ps3);
    let mut swept = studio_domain::HeadCatalogSummary::default();
    studio_domain::sweep_nonplayer_heads(
        &minimal_roots,
        &Default::default(),
        &temp.path().join("sweep"),
        &studio_domain::CatalogOptions {
            game_tag: "nhllegacy".into(),
            textures: true,
        },
        &mut swept,
    );
    assert!(swept.failures.is_empty(), "{:?}", swept.failures);
    assert!(swept.heads_written > 0 && swept.textures_written > 1);
    let reg = Registry::default();
    let args = studio_api::dto::AssetRootsArgsDto {
        install_dir: s(&root),
        platform: Some("ps3".into()),
        rendering_override: None,
        fe_override: None,
    };
    for (slot, id) in [("skates", 0), ("gloves", 0), ("stick", 0), ("helmet", 0)] {
        let preview = reg
            .equipment_item_preview(&args, slot, id)
            .unwrap_or_else(|e| panic!("{slot}: {e}"));
        assert!(!preview.pieces.is_empty(), "{slot}");
    }
}
