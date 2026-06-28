//! Drive the registry + DTO seam without Tauri: load a real anchor, validate, save, and check the
//! DTOs and error paths.

use std::path::{Path, PathBuf};

use studio_api::{parse_platform, parse_role, Registry};
use studio_domain::FieldValue;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn s(p: PathBuf) -> String {
    p.to_str().unwrap().to_string()
}

const DB: &str = "anchors/ps3/nhllegacy/databases/nhlng.db";
const META: &str = "anchors/ps3/nhllegacy/databases/nhlng-meta.xml";
const DEFS: &str = "defs";

#[test]
fn load_validate_save_through_registry() {
    let reg = Registry::default();
    let platform = parse_platform("ps3").unwrap();
    let role = parse_role("nhlng").unwrap();

    let summary = reg
        .load_project(&s(repo(DB)), platform, role, &s(repo(META)), &s(repo(DEFS)))
        .unwrap();
    assert_eq!(summary.project_id, 0);
    assert_eq!(summary.source, "raw_db");
    assert_eq!(summary.platform, "ps3");
    assert_eq!(summary.role, "nhlng");
    assert!(!summary.dirty);
    assert!(summary.table_count > 0);
    assert_eq!(summary.tables.len(), summary.table_count);

    let report = reg.validate(0).unwrap();
    assert!(report.clean, "{} error findings", report.error_count);

    // Save with no edits == the source file, byte for byte.
    let out = std::env::temp_dir().join("studio_api_nhlng_out.db");
    let written = reg.save(0, out.to_str().unwrap()).unwrap();
    let original = std::fs::read(repo(DB)).unwrap();
    assert_eq!(written as usize, original.len());
    assert_eq!(std::fs::read(&out).unwrap(), original);
    let _ = std::fs::remove_file(&out);

    // Unknown ids and bad enums surface as errors, not panics.
    assert!(reg.summary(99).is_err());
    assert!(parse_platform("dreamcast").is_err());
    assert!(parse_role("savegame").is_err());
}

#[test]
fn players_listing_view_and_edit_through_registry() {
    let reg = Registry::default();
    reg.load_project(
        &s(repo(DB)),
        parse_platform("ps3").unwrap(),
        parse_role("nhlng").unwrap(),
        &s(repo(META)),
        &s(repo(DEFS)),
    )
    .unwrap();

    let players = reg.list_players(0).unwrap();
    assert!(!players.is_empty(), "anchor has players");

    let pid = players[0].player_id;
    let view = reg.player_view(0, pid).unwrap().expect("player resolves");
    assert!(view.identity.iter().any(|f| f.name == "position"));
    assert!(reg.player_view(0, u64::MAX).unwrap().is_none());

    // Edit the first editable, ranged integer field to its low bound; it should validate clean.
    let target = view
        .all_fields
        .iter()
        .find(|f| f.editable && !f.is_string && f.range_low.is_some())
        .expect("an editable integer bio field");
    let out = reg
        .set_field(
            0,
            &target.studio_table,
            view.bio_record,
            &target.studio_field,
            FieldValue::Int(target.range_low.unwrap()),
        )
        .unwrap();
    assert_eq!(out.edits, 1);
    assert!(reg.validate(0).unwrap().clean);

    // Out-of-range is refused.
    if let Some(hi) = target.range_high {
        assert!(reg
            .set_field(
                0,
                &target.studio_table,
                view.bio_record,
                &target.studio_field,
                FieldValue::Int(hi + 1_000_000),
            )
            .is_err());
    }
}

#[test]
fn teams_and_arenas_through_registry() {
    let reg = Registry::default();
    reg.load_project(
        &s(repo(DB)),
        parse_platform("ps3").unwrap(),
        parse_role("nhlng").unwrap(),
        &s(repo(META)),
        &s(repo(DEFS)),
    )
    .unwrap();

    let teams = reg.list_teams(0).unwrap();
    assert!(!teams.is_empty(), "anchor has teams");
    let team = reg.team_view(0, teams[0].team_id).unwrap().expect("team resolves");
    assert!(team.identity.iter().any(|f| f.name == "fullname"));
    assert!(reg.team_view(0, u64::MAX).unwrap().is_none());

    let arenas = reg.list_arenas(0).unwrap();
    assert!(!arenas.is_empty(), "anchor has arenas");
    let arena = reg.arena_view(0, arenas[0].arena_id).unwrap().expect("arena resolves");
    assert!(arena.identity.iter().any(|f| f.name == "arenaname"));
}
