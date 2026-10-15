//! Read-only native PS3 pipeline verification; creates only a disposable project.
use std::path::PathBuf;
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let install =
        PathBuf::from(std::env::var_os("NHLMS_PS3_INSTALL").expect("Legacy USRDIR fixture root"));
    let repo = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let tmp = tempfile::tempdir()?;
    let reg = studio_api::Registry::default();
    let store = studio_domain::BaselineStore::open(
        &repo.join("baselines/INVENTORY.json"),
        &repo.join("anchors"),
    )?;
    let id = reg
        .project_create_fresh(
            &tmp.path().join("p").to_string_lossy(),
            "nhllegacy",
            studio_domain::Platform::Ps3,
            "",
            None,
            None,
            &store,
            &repo.join("defs").to_string_lossy(),
        )?
        .summary
        .project_id;
    let roots = studio_api::dto::AssetRootsArgsDto {
        install_dir: install.to_string_lossy().into_owned(),
        platform: Some("ps3".into()),
        rendering_override: None,
        fe_override: None,
    };
    for (goalie, player_gear) in [(false, false), (true, false), (false, true), (true, true)] {
        let player = reg
            .list_players(id)?
            .into_iter()
            .find(|p| p.team_id == Some(2) && p.is_goalie == goalie)
            .expect("fixture roster has both roles");
        let kit = reg.team_kit_preview(id, &roots, 2, 0, 0, Some(player.player_id), player_gear)?;
        for piece in &kit.pieces {
            println!(
                "goalie={goalie} player_gear={player_gear} {}: {} meshes; textures {:?}",
                piece.kind,
                piece.submeshes.len(),
                piece
                    .submeshes
                    .iter()
                    .map(|m| m.material_index)
                    .collect::<Vec<_>>()
            );
        }
        for kind in if goalie {
            vec![
                "jersey", "pants", "socks", "skates", "pads", "blocker", "trapper", "mask", "cage",
            ]
        } else {
            vec![
                "jersey", "pants", "socks", "skates", "gloves", "helmet", "head",
            ]
        } {
            let piece = kit
                .pieces
                .iter()
                .find(|p| p.kind == kind)
                .unwrap_or_else(|| panic!("Missing {kind}"));
            assert!(!piece.submeshes.is_empty());
            if ["gloves", "socks", "skates", "pads"].contains(&kind) {
                let xs: Vec<_> = piece
                    .submeshes
                    .iter()
                    .flat_map(|m| m.positions.iter().map(|p| p[0]))
                    .collect();
                assert!(
                    xs.iter().any(|x| *x < -0.01) && xs.iter().any(|x| *x > 0.01),
                    "{kind} must include both sides: goalie={goalie}, player gear={player_gear}"
                );
            }
            if kind != "cage" {
                assert!(
                    piece.submeshes.iter().all(|s| s.material_index.is_some()),
                    "{kind} lacks texture"
                );
            }
        }
    }
    reg.close_project(id);
    Ok(())
}
