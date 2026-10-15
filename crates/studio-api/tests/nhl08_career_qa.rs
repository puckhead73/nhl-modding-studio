//! NHL08 career-year display regression from playtest feedback.
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::Platform;
fn repo(p: &str) -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(p) }
fn s(p: &Path) -> String { p.to_string_lossy().into_owned() }
#[test]
fn career_seasons_and_player_join_match_nhl08_on_both_schemas() {
    for (folder, platform) in [("xbox360", Platform::Xbox), ("ps3", Platform::Ps3)] {
    let reg = Registry::default();
    let id = reg.open_workspace_from_paths(Some("nhl08"), platform,
        &s(&repo(&format!("anchors/{folder}/nhl08/databases/nhlng.db"))), None,
        &s(&repo(&format!("anchors/{folder}/nhl08/databases/nhlng-meta.xml"))), &s(&repo("defs"))).unwrap().project_id;
    let players = reg.list_players(id).unwrap();
    for last in ["Crosby", "Santala"] {
        let player = players.iter().find(|p| p.last_name == last).unwrap();
        let view = reg.player_view(id, player.player_id).unwrap().unwrap();
        // A stale biography pointer must not assign a free agent to Anaheim.
        if player.team_id.is_none() && view.memberships.is_empty() {
            assert_eq!(view.header.primary_team_id, None);
            assert_eq!(view.header.primary_team_name, None);
        }
        let stats = reg.career_stats(id, player.player_id).unwrap();
        let season = stats.sections.iter().find(|s| s.kind == "skater_season").unwrap();
        assert!(!season.rows.is_empty());
        let points = season.columns.iter().position(|c| c.label == "PTS").unwrap();
        let gp = season.columns.iter().position(|c| c.label == "GP").unwrap();
        // Crosby's known 79 GP / 120 PTS season catches the old +1 join, which
        // wrongly returned Getzlaf's 82 GP / 58 PTS even after correcting the year.
        let (year, games, pts) = if last == "Crosby" { ("2006-07",79,120) } else { ("2003-04",33,3) };
        assert!(season.rows.iter().any(|r| r.season_label == year
            && r.cells[gp].as_ref().and_then(|v| v.decoded) == Some(games)
            && r.cells[points].as_ref().and_then(|v| v.decoded) == Some(pts)), "{folder}/{last}: wrong player or season");
        assert!(stats.sections.iter().flat_map(|s| &s.rows).all(|r| r.season_label.starts_with("199") || r.season_label.starts_with("200")), "{last}: career years cannot predate this player's birth");
        // In-memory authoring must use the same join and year encoding as display.
        let added = reg.add_career_season(id,"tsSy",player.player_id,2008,0,"QA").unwrap();
        let after = reg.career_stats(id,player.player_id).unwrap();
        assert!(after.sections.iter().filter(|s| s.kind == "skater_season").flat_map(|s| &s.rows)
            .any(|r| r.record == added.record && r.season_label == "2007-08" && r.team == "QA"));
        assert!(reg.add_career_season(id,"tsSy",player.player_id,1950,0,"QA").is_err());
    }
    }
}

