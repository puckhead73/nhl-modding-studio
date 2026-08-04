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
}

/// The texture-import seam end-to-end (no Tauri): decode a source PNG, dispatch on the target file's
/// container, overwrite it, and write a one-time `.bak` — all through `Registry::import_texture`.
#[test]
fn import_texture_writes_edited_gnf_through_registry() {
    let reg = Registry::default();
    let dir = std::env::temp_dir().join("studio_api_import_texture");
    std::fs::create_dir_all(&dir).unwrap();
    let (w, h) = (64u32, 64u32);

    // A synthetic BC7 GNF target (a flat 50% grey so we can see the edit take).
    let grey = vec![128u8; (w * h * 4) as usize];
    let gnf = tdb_rx2::gnf::encode_gnf(
        tdb_rx2::gnf::BcFormat::Bc7,
        w,
        h,
        &tdb_rx2::bc7::encode(&grey, w, h),
    );
    let target = dir.join("p9000.gnf");
    std::fs::write(&target, &gnf).unwrap();

    // A source PNG: a directional gradient (R across, B down) written via the domain encoder.
    let mut rgba = vec![0u8; (w * h * 4) as usize];
    for y in 0..h {
        for x in 0..w {
            let i = ((y * w + x) * 4) as usize;
            rgba[i] = (x * 255 / w) as u8;
            rgba[i + 2] = (y * 255 / h) as u8;
            rgba[i + 3] = 255;
        }
    }
    let png = studio_domain::DecodedImage { width: w, height: h, rgba }
        .to_png()
        .unwrap();
    let src = dir.join("edit.png");
    std::fs::write(&src, png).unwrap();

    let root = dir.to_str().unwrap();
    let src_str = src.to_str().unwrap();
    let summary = reg
        .import_texture(root, "p9000.gnf", src_str, true, None)
        .expect("import_texture");
    assert_eq!(summary.container, "gnf");
    assert_eq!((summary.width, summary.height), (w, h));
    assert!(summary.bytes_written > 0);
    let bak = summary.backup_path.expect("backup written");
    assert!(std::path::Path::new(&bak).exists(), "backup file exists");
    // The backup holds the pristine original; the target now holds the edit.
    assert_eq!(std::fs::read(&bak).unwrap(), gnf, "backup == pristine original");
    assert_ne!(std::fs::read(&target).unwrap(), gnf, "target was overwritten");

    // A second import must not clobber the pristine `.bak`.
    let summary2 = reg
        .import_texture(root, "p9000.gnf", src_str, true, None)
        .expect("second import");
    assert_eq!(summary2.backup_path.as_deref(), Some(bak.as_str()));
    assert_eq!(std::fs::read(&bak).unwrap(), gnf, "bak still the pristine original");

    // A path escaping the install root is refused (containment).
    assert!(reg.import_texture(root, "../escape.gnf", src_str, false, None).is_err());
    // Missing target / missing source surface as errors, not panics.
    assert!(reg.import_texture(root, "nope.gnf", src_str, false, None).is_err());
    assert!(reg
        .import_texture(root, "p9000.gnf", dir.join("nope.png").to_str().unwrap(), false, None)
        .is_err());

    let _ = std::fs::remove_dir_all(&dir);
}

/// The Transactions pool reader end-to-end: the FA pool is exactly the `team_id == None` slice of
/// the master list (S1 occupancy model), and release/sign move a player across the boundary.
#[test]
fn free_agent_pool_through_registry() {
    let reg = Registry::default();
    reg.load_project(
        &s(repo(DB)),
        parse_platform("ps3").unwrap(),
        parse_role("nhlng").unwrap(),
        &s(repo(META)),
        &s(repo(DEFS)),
    )
    .unwrap();

    let pool = reg.list_free_agents(0).unwrap();
    assert!(!pool.is_empty(), "anchor carries unplaced (off-roster) players");
    assert!(pool.iter().all(|p| p.team_id.is_none()), "pool rows have no team");
    let expected: Vec<u64> = reg
        .list_players(0)
        .unwrap()
        .iter()
        .filter(|p| p.team_id.is_none())
        .map(|p| p.player_id)
        .collect();
    assert_eq!(pool.len(), expected.len(), "pool == master list's team-less slice");

    // Release a rostered player → joins the pool; sign them back → leaves it.
    let rostered = reg
        .list_players(0)
        .unwrap()
        .into_iter()
        .find(|p| p.team_id.is_some())
        .unwrap();
    let team = rostered.team_id.unwrap();
    reg.release_to_fa(0, team, rostered.player_id).unwrap();
    assert!(
        reg.list_free_agents(0).unwrap().iter().any(|p| p.player_id == rostered.player_id),
        "released player appears in the pool"
    );
    reg.sign_from_fa(0, rostered.player_id, team).unwrap();
    assert!(
        !reg.list_free_agents(0).unwrap().iter().any(|p| p.player_id == rostered.player_id),
        "signed player leaves the pool"
    );
    // ...and the master list shows them back on their team (the slot resolves — regression guard
    // for the `vfEq==0` occupancy-key bug).
    let back = reg.list_players(0).unwrap();
    let row = back.iter().find(|p| p.player_id == rostered.player_id).unwrap();
    assert_eq!(row.team_id, Some(team), "signed player resolves back onto the team");
}

#[test]
fn field_edit_bounds_through_registry() {
    let reg = Registry::default();
    reg.load_project(
        &s(repo(DB)),
        parse_platform("ps3").unwrap(),
        parse_role("nhlng").unwrap(),
        &s(repo(META)),
        &s(repo(DEFS)),
    )
    .unwrap();
    let pid = reg.list_players(0).unwrap()[0].player_id;
    let view = reg.player_view(0, pid).unwrap().expect("player resolves");

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
