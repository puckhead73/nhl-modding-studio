//! NHL 24 roster session end to end through the registry: open the live database, edit, save
//! into `game-mods`, reopen from the overlay, restore. Runs against the real install through
//! folder junctions in a temporary install folder, so the install's own `game-mods` and
//! `studio` folders are never touched. Selected runs fail (never skip) without the inputs:
//!
//! ```sh
//! NHL24_INSTALL=F:\Games\NHL24 NHL24_OODLE=<oo2core dll> \
//!   cargo test -p studio-api --test nhl24_roster -- --ignored
//! ```
#![cfg(windows)]
use std::path::{Path, PathBuf};
use studio_api::Registry;
use studio_domain::FieldValue;

fn env(name: &str) -> PathBuf {
    let p = PathBuf::from(std::env::var_os(name).unwrap_or_else(|| panic!("{name} is not set")));
    assert!(p.exists(), "{name} does not exist: {}", p.display());
    p
}

fn junction(link: &Path, target: &Path) {
    let ok = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(link)
        .arg(target)
        .output()
        .unwrap()
        .status
        .success();
    assert!(ok, "could not create junction {}", link.display());
}

fn repo(rel: &str) -> String {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
        .to_string_lossy()
        .into_owned()
}

#[test]
#[ignore = "needs NHL24_INSTALL and NHL24_OODLE"]
fn open_edit_save_reopen_restore() {
    let real = env("NHL24_INSTALL");
    let oodle = env("NHL24_OODLE");
    let scratch = tempfile::tempdir().unwrap();
    let install = scratch.path().join("NHL24");
    std::fs::create_dir_all(install.join("game-mods")).unwrap();
    junction(&install.join("game"), &real.join("game"));
    junction(&install.join("game-UPDATE"), &real.join("game-UPDATE"));

    let reg = Registry::default();
    let opened = reg.nhl24_open(&install, &oodle, &repo("defs")).unwrap();
    assert!(!opened.from_overlay);
    assert!(opened.status.overlay_files.is_empty());
    let id = opened.summary.project_id;
    assert!(
        reg.nhl24_open(&install, &oodle, &repo("defs")).is_err(),
        "second open refused"
    );

    // The ordinary team and player views read NHL 24's schema (values from the user's in-game
    // screenshot of 1 Nov 2026: Connor McDavid, #97, Edmonton Oilers).
    let teams = reg.list_teams(id).unwrap();
    let oilers = teams
        .iter()
        .find(|t| t.name.contains("Oilers"))
        .expect("Edmonton Oilers listed");
    let players = reg.list_players(id).unwrap();
    let mcdavid = players
        .iter()
        .find(|p| p.bio_record == 1570)
        .expect("bio row 1570 listed");
    eprintln!(
        "{} teams, {} players; {} ({}) team {}; overall {:?}, cap hit {:?}, years {:?}, position {}",
        teams.len(),
        players.len(),
        oilers.name,
        oilers.abbrev,
        oilers.team_id,
        mcdavid.overall,
        mcdavid.cap_hit_dollars,
        mcdavid.contract_years,
        mcdavid.position_label
    );
    assert_eq!(
        (mcdavid.first_name.as_str(), mcdavid.last_name.as_str()),
        ("Connor", "McDavid")
    );
    assert_eq!(mcdavid.jersey_number, Some(97));
    assert_eq!(mcdavid.team_id, Some(oilers.team_id));

    reg.set_field(id, "ubPc", 1570, "QbMR", FieldValue::Text("Studio".into()))
        .unwrap();
    assert_eq!(reg.save_workspace(id).unwrap(), 1);
    let status = reg.nhl24_status(&install).unwrap();
    assert_eq!(status.overlay_files.len(), 2);
    assert_eq!(status.open_project, Some(id));
    assert!(
        reg.nhl24_restore(&install).is_err(),
        "restore refused while open"
    );

    assert!(reg.close_project(id));
    let again = reg.nhl24_open(&install, &oodle, &repo("defs")).unwrap();
    assert!(again.from_overlay);
    let bytes = std::fs::read(&again.db_path).unwrap();
    let db = tdb_core::TdbFile::parse(&bytes).unwrap();
    let bio = db.table("ubPc").unwrap();
    assert_eq!(
        db.read_inline_str(bio, 1570, bio.field("QbMR").unwrap())
            .unwrap(),
        b"Studio"
    );
    assert!(reg.close_project(again.summary.project_id));

    let restored = reg.nhl24_restore(&install).unwrap();
    assert!(restored.overlay_files.is_empty());
    assert_eq!(
        std::fs::read_dir(install.join("game-mods"))
            .unwrap()
            .count(),
        0
    );
    // Remove the junctions before the temporary folder is deleted, so nothing follows them.
    std::fs::remove_dir(install.join("game")).unwrap();
    std::fs::remove_dir(install.join("game-UPDATE")).unwrap();
}

/// The Edmonton Oilers as NHL 24 01.60 shows them (the user's roster-screen screenshots of
/// 1 Nov 2026): style, cap hit and years for nine players; McDavid's header (6'1", 193 lb,
/// age 26, shoots left). Birth dates are the players' public ones, and the in-game ages agree
/// with them. Nationalities are public facts.
#[test]
#[ignore = "needs NHL24_INSTALL and NHL24_OODLE"]
fn oilers_match_the_in_game_roster_screen() {
    let real = env("NHL24_INSTALL");
    let oodle = env("NHL24_OODLE");
    let scratch = tempfile::tempdir().unwrap();
    let install = scratch.path().join("NHL24");
    std::fs::create_dir_all(install.join("game-mods")).unwrap();
    junction(&install.join("game"), &real.join("game"));
    junction(&install.join("game-UPDATE"), &real.join("game-UPDATE"));
    let reg = Registry::default();
    let id = reg
        .nhl24_open(&install, &oodle, &repo("defs"))
        .unwrap()
        .summary
        .project_id;
    let oilers = reg
        .list_teams(id)
        .unwrap()
        .into_iter()
        .find(|t| t.name == "Edmonton Oilers")
        .unwrap();
    let players = reg.list_players(id).unwrap();
    #[rustfmt::skip]
    let expected = [
        ("McDavid", "Playmaker", 12_500_000, 3, "1997-01-13", "Canada"),
        ("Draisaitl", "Sniper", 8_500_000, 2, "1995-10-27", "Germany"),
        ("Hyman", "Power Forward", 5_500_000, 5, "1992-06-09", "Canada"),
        ("Ekholm", "Two-Way Defenseman", 6_000_000, 3, "1990-05-24", "Sweden"),
        ("Nurse", "Two-Way Defenseman", 9_250_000, 7, "1995-02-04", "Canada"),
        ("Bouchard", "Offensive Defenseman", 3_900_000, 2, "1999-10-20", "Canada"),
        ("Nugent-Hopkins", "Two-Way Forward", 5_125_000, 6, "1993-04-12", "Canada"),
        ("Kane", "Power Forward", 5_125_000, 3, "1991-08-02", "Canada"),
        ("Foegele", "Power Forward", 2_750_000, 1, "1996-04-01", "Canada"),
    ];
    for (last, style, cap, years, born, nation) in expected {
        let p = players
            .iter()
            .find(|p| p.team_id == Some(oilers.team_id) && p.last_name == last)
            .unwrap_or_else(|| panic!("{last} on the Oilers"));
        let v = reg.player_view(id, p.player_id).unwrap().unwrap();
        assert_eq!(p.style_label.as_deref(), Some(style), "{last} style");
        assert_eq!(p.cap_hit_dollars, Some(cap), "{last} cap hit");
        assert_eq!(p.contract_years, Some(years), "{last} years");
        assert_eq!(
            v.header.birthdate.as_deref(),
            Some(born),
            "{last} birth date"
        );
        assert_eq!(
            v.header.nationality_label.as_deref(),
            Some(nation),
            "{last} nationality"
        );
        if last == "McDavid" {
            assert_eq!(v.header.height.as_deref(), Some("6'1\""));
            assert_eq!(v.header.weight.as_deref(), Some("193 lb"));
            assert_eq!(v.header.handedness_label.as_deref(), Some("Left"));
            // The encoded cells are editable now that NHL 24 carries the PS4 contract.
            let field = |name: &str| {
                v.all_fields
                    .iter()
                    .chain(&v.ratings)
                    .find(|f| f.name == name)
                    .unwrap_or_else(|| panic!("field {name}"))
            };
            for name in [
                "weight",
                "height",
                "year",
                "month",
                "day",
                "contractdollars",
            ] {
                assert!(field(name).editable, "{name} editable");
            }
            assert!(v.ratings.iter().any(|f| f.editable), "ratings editable");
            // Traits use NHL 24's own names (gamemodes/creation_zone_csv/playertraits.csv).
            let traits: Vec<&str> = v
                .attributes
                .traits
                .iter()
                .map(|t| t.label.as_str())
                .collect();
            for name in [
                "Setup Man Low",
                "Shoots On The Rush",
                "Speedster",
                "Risky Dangler",
                "Perimeter Sniper",
                "Datsyuk Toe Drag",
                "Between Legs One Timer",
            ] {
                assert!(traits.contains(&name), "McDavid trait {name} in {traits:?}");
            }
            assert!(
                traits.iter().all(|t| !t.starts_with("Trait #")),
                "{traits:?}"
            );
            assert!(v
                .attributes
                .trait_options
                .iter()
                .any(|o| o.label == "Kucherov Deke"));
        }
    }
    assert!(reg.close_project(id));
    std::fs::remove_dir(install.join("game")).unwrap();
    std::fs::remove_dir(install.join("game-UPDATE")).unwrap();
}
