//! Save Editor end to end through the registry: identify → open → edit → save (backup first) →
//! reopen, on temporary copies of committed anchor saves (NHL 07 / 08 roster change lists included). Also the refusals:
//! unacknowledged franchise saves, a game the file was not identified as, and game-file writes
//! (dictionary) from a Save Editor session.

use std::path::{Path, PathBuf};

use studio_api::Registry;
use studio_domain::{BaselineStore, FieldValue, Platform};

fn repo() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn store() -> BaselineStore {
    let r = repo();
    BaselineStore::open(&r.join("baselines/INVENTORY.json"), &r.join("anchors")).unwrap()
}

fn defs() -> String {
    repo().join("defs").to_string_lossy().into_owned()
}

/// A temporary directory removed on drop.
struct Temp(PathBuf);
impl Temp {
    fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let p = std::env::temp_dir().join(format!(
            "nhlms-save-editor-{tag}-{}-{nanos}",
            std::process::id()
        ));
        std::fs::create_dir_all(&p).unwrap();
        Temp(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Copy an anchor save's whole folder (keeping its name, which carries any title id) into `tmp`
/// and return the copied data file's path.
fn copy_save(tmp: &Temp, anchor_rel: &str) -> PathBuf {
    let src = repo().join("anchors").join(anchor_rel);
    let folder = src.parent().unwrap();
    let dest_folder = tmp.0.join("saves").join(folder.file_name().unwrap());
    let mut stack = vec![(folder.to_path_buf(), dest_folder.clone())];
    while let Some((from, to)) = stack.pop() {
        std::fs::create_dir_all(&to).unwrap();
        for e in std::fs::read_dir(&from).unwrap().flatten() {
            let target = to.join(e.file_name());
            if e.path().is_dir() {
                stack.push((e.path(), target));
            } else {
                std::fs::copy(e.path(), target).unwrap();
            }
        }
    }
    dest_folder.join(src.file_name().unwrap())
}

/// Open, renumber one rostered player, save twice (backup only on the first), reopen and check.
fn round_trip(anchor_rel: &str, game: &str, platform: Platform, acknowledged: bool) {
    let tmp = Temp::new(game);
    let file = copy_save(&tmp, anchor_rel);
    let is_database = anchor_rel.ends_with(".db");
    let original = std::fs::read(&file).unwrap();
    let backups = tmp.0.join("backups");
    let store = store();
    let reg = Registry::default();
    let path = file.to_string_lossy().into_owned();

    let id = reg.save_editor_identify(&path, &store).unwrap();
    assert!(
        id.game_confident,
        "{anchor_rel}: {:?}",
        id.candidates.iter().map(|c| &c.game_id).collect::<Vec<_>>()
    );
    assert!(id
        .candidates
        .iter()
        .any(|c| c.leading && c.game_id == game && c.platform == platform.as_str()));
    assert_ne!(
        id.decision, "refused",
        "{anchor_rel}: {:?}",
        id.decision_reason
    );

    let opened = reg
        .save_editor_open(
            &path,
            game,
            platform,
            acknowledged,
            None,
            &store,
            &defs(),
            &backups,
        )
        .unwrap_or_else(|e| panic!("{anchor_rel}: open: {e}"));
    assert!(!opened.read_only);
    // A save is supplemented from its game's database; a bare database stands alone.
    assert_eq!(
        opened.supplied_tables > 0,
        !is_database,
        "{anchor_rel}: supplied {}",
        opened.supplied_tables
    );
    let pid = opened.summary.project_id;

    // A rostered player: renumber (bio jersey) and rename (last name) through the generic field
    // edit, resolving both fields by metadata name so every schema era is covered.
    let players = reg.list_players(pid).unwrap();
    let p = players
        .iter()
        .find(|p| p.team_id.is_some() && p.jersey_number.is_some())
        .map(|p| (p.player_id, p.team_id, p.jersey_number, p.last_name.clone()))
        .expect("a rostered player with a number");
    let (player_id, team_id, old_number, old_surname) = p;
    let team = team_id.unwrap();
    // A number nobody on the team wears (NHL 07 refuses duplicates on one team).
    let taken: Vec<_> = players
        .iter()
        .filter(|q| q.team_id == Some(team))
        .filter_map(|q| q.jersey_number)
        .collect();
    let number = (77..99)
        .chain(2..77)
        .find(|n| !taken.contains(n))
        .expect("a free number");
    let surname = "Savetest".to_string();
    let view = reg
        .player_view(pid, player_id)
        .unwrap()
        .expect("player view");
    let field = |name: &str| {
        view.identity
            .iter()
            .find(|f| f.name == name && f.editable)
            .map(|f| (f.studio_table.clone(), f.studio_field.clone()))
            .unwrap_or_else(|| panic!("{anchor_rel}: no editable bio field {name}"))
    };
    let (jersey, last) = (field("jersey"), field("lastname"));
    reg.set_field(
        pid,
        &jersey.0,
        view.bio_record,
        &jersey.1,
        FieldValue::Int(number),
    )
    .unwrap();
    reg.set_field(
        pid,
        &last.0,
        view.bio_record,
        &last.1,
        FieldValue::Text(surname.clone()),
    )
    .unwrap();
    assert!(reg.summary(pid).unwrap().dirty);
    let live = reg.list_players(pid).unwrap();
    let in_session = live.iter().find(|q| q.player_id == player_id).unwrap();
    assert_eq!(
        (in_session.jersey_number, in_session.last_name.as_str()),
        (Some(number), surname.as_str())
    );
    let first = reg.save_editor_save(pid).unwrap();
    let backup = PathBuf::from(first.backup_path.clone().expect("first save backs up"));
    assert!(
        backup.starts_with(&backups),
        "backup outside the backup folder: {}",
        backup.display()
    );
    assert_eq!(
        std::fs::read(&backup).unwrap(),
        original,
        "the backup is the original file"
    );
    assert!(!first.summary.dirty);
    assert_ne!(
        std::fs::read(&file).unwrap(),
        original,
        "the edit reached the file"
    );
    let second = reg.save_editor_save(pid).unwrap();
    assert!(second.backup_path.is_none(), "one backup per session");
    assert_eq!(std::fs::read_dir(&backups).unwrap().count(), 1);
    assert_eq!(reg.close_project_outcome(pid).outcome, "closed");

    // Reopen: identified the same way, checksums valid, the edit present.
    let again = reg.save_editor_identify(&path, &store).unwrap();
    assert_ne!(
        again.checksums_valid,
        Some(false),
        "{anchor_rel}: container checksums after save"
    );
    let reopened = reg
        .save_editor_open(
            &path,
            game,
            platform,
            acknowledged,
            None,
            &store,
            &defs(),
            &backups,
        )
        .unwrap();
    let after = reg.list_players(reopened.summary.project_id).unwrap();
    let q = after.iter().find(|q| q.player_id == player_id).unwrap();
    assert_eq!(
        q.jersey_number,
        Some(number),
        "{anchor_rel}: jersey after reopen"
    );
    assert_eq!(q.last_name, surname, "{anchor_rel}: last name after reopen");
    assert_eq!(q.team_id, Some(team));
    println!(
        "{anchor_rel}: {old_surname} #{old_number:?} -> {surname} #{number} saved and reopened"
    );
}

#[test]
fn ps3_v4_roster_save_round_trips() {
    round_trip(
        "ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA",
        "nhl14",
        Platform::Ps3,
        false,
    );
}

#[test]
fn xbox360_v2_roster_save_round_trips() {
    round_trip(
        "xbox360/nhl12/savefiles/ROSTER 20260627052113/ROSTER 20260627052113",
        "nhl12",
        Platform::Xbox,
        false,
    );
}

#[test]
fn ps3_v3_gen6_roster_save_round_trips() {
    round_trip(
        "ps3/nhl09/savefiles/BLUS3016802002026 - ROSTER/SYS-DATA",
        "nhl09",
        Platform::Ps3,
        false,
    );
}

#[test]
fn ps4_roster_save_round_trips() {
    round_trip(
        "ps4/nhl17/savefiles/RosterCustom0/DATA",
        "nhl17",
        Platform::Ps4,
        false,
    );
}

#[test]
fn nhl07_change_list_roster_save_round_trips() {
    round_trip("xbox360/nhl07/savefiles/ROSTER1/ROSTER1", "nhl07", Platform::Xbox, false);
}

#[test]
fn nhl08_change_list_roster_saves_round_trip() {
    round_trip(
        "xbox360/nhl08/savefiles/ROSTER1 20260718064658/ROSTER1 20260718064658",
        "nhl08",
        Platform::Xbox,
        false,
    );
    round_trip(
        "ps3/nhl08/savefiles/BLUS30062-002-000 - ROSTER/SYS-DATA",
        "nhl08",
        Platform::Ps3,
        false,
    );
}

#[test]
fn extracted_main_database_round_trips() {
    // A bare database (no container, no title id in its path): NHL Legacy's database is shared by
    // PS3, Xbox 360 and Recompiled, so the game is settled and the platform is the user's choice.
    round_trip(
        "ps3/nhllegacy/databases/nhlng.db",
        "nhllegacy",
        Platform::Ps3,
        false,
    );
}

#[test]
fn mode_save_needs_acknowledgement_then_round_trips() {
    let tmp = Temp::new("ack");
    let file = copy_save(&tmp, "ps4/nhl18/savefiles/ThreeOnThree0/DATA");
    let path = file.to_string_lossy().into_owned();
    let store = store();
    let reg = Registry::default();
    let id = reg.save_editor_identify(&path, &store).unwrap();
    assert_eq!(
        (id.kind.as_str(), id.decision.as_str()),
        ("threes", "caution")
    );
    let err = reg
        .save_editor_open(
            &path,
            "nhl18",
            Platform::Ps4,
            false,
            None,
            &store,
            &defs(),
            &tmp.0.join("b"),
        )
        .err()
        .map(|e| e.to_string())
        .expect("expected a refusal");
    assert!(err.contains("Confirm"), "{err}");
    drop(tmp);
    round_trip(
        "ps4/nhl18/savefiles/ThreeOnThree0/DATA",
        "nhl18",
        Platform::Ps4,
        true,
    );
}

#[test]
fn franchise_saves_open_over_the_default_database_read_only() {
    // Franchise / season saves hold their mode's tables, not the rosters: the teams and players
    // they lack are shown from the bundled default database, read-only, and never written back.
    for (rel, game, platform) in [
        (
            "ps4/nhl15/savefiles/c6b05b60 - BEAGM/DATA",
            "nhl15",
            Platform::Ps4,
        ),
        (
            "xbox360/nhl10/savefiles/DYNASTY1 20260627045642/DYNASTY1 20260627045642",
            "nhl10",
            Platform::Xbox,
        ),
        (
            "ps3/nhl11/savefiles/BLUS305681100 - SEASON/SYS-DATA",
            "nhl11",
            Platform::Ps3,
        ),
    ] {
        let tmp = Temp::new("mode");
        let store = store();
        let reg = Registry::default();
        let file = copy_save(&tmp, rel);
        let original = std::fs::read(&file).unwrap();
        let f = file.to_string_lossy().into_owned();
        let id = reg.save_editor_identify(&f, &store).unwrap();
        assert_eq!(id.decision, "caution", "{rel}: {:?}", id.decision_reason);
        assert!(
            id.supplied_tables > 0 && id.notes.iter().any(|n| n.contains("read-only")),
            "{rel}"
        );
        let err = reg
            .save_editor_open(
                &f,
                game,
                platform,
                false,
                None,
                &store,
                &defs(),
                &tmp.0.join("b"),
            )
            .err()
            .map(|e| e.to_string())
            .expect("expected a refusal");
        assert!(err.contains("Confirm"), "{rel}: {err}");
        let opened = reg
            .save_editor_open(
                &f,
                game,
                platform,
                true,
                None,
                &store,
                &defs(),
                &tmp.0.join("b"),
            )
            .unwrap();
        assert_eq!(opened.supplied_tables, id.supplied_tables);
        let pid = opened.summary.project_id;

        // Players come from the default database: listed, shown read-only, and refused on edit.
        let players = reg.list_players(pid).unwrap();
        let p = players
            .iter()
            .find(|p| p.team_id.is_some())
            .expect("players from the default database");
        let view = reg.player_view(pid, p.player_id).unwrap().unwrap();
        let jersey = view
            .identity
            .iter()
            .find(|f| f.name == "jersey")
            .expect("jersey field");
        assert!(
            !jersey.editable,
            "{rel}: supplied fields read as non-editable"
        );
        let (t, fld) = (jersey.studio_table.clone(), jersey.studio_field.clone());
        let err = reg
            .set_field(pid, &t, view.bio_record, &fld, FieldValue::Int(77))
            .err()
            .map(|e| e.to_string())
            .expect("expected a refusal");
        assert!(err.contains("read-only"), "{rel}: {err}");
        assert!(!reg.summary(pid).unwrap().dirty);

        // Saving writes back exactly the save's own database.
        let saved = reg.save_editor_save(pid).unwrap();
        assert!(saved.backup_path.is_some());
        let inner = |b: &[u8]| {
            studio_domain::save_identify::probe_bytes(b.to_vec())
                .unwrap()
                .1
        };
        assert_eq!(
            inner(&std::fs::read(&file).unwrap()),
            inner(&original),
            "{rel}: the save's database is unchanged"
        );
        assert_eq!(reg.close_project_outcome(pid).outcome, "closed");
        println!(
            "{rel}: {} own tables, {} supplied read-only; {} players shown",
            id.own_tables,
            id.supplied_tables,
            players.len()
        );
    }
}

#[test]
fn refusals_hold_on_open() {
    let tmp = Temp::new("refuse");
    let store = store();
    let reg = Registry::default();
    let b = tmp.0.join("b");

    // A game the file was not identified as.
    let ps3 = copy_save(&tmp, "ps3/nhl14/savefiles/BLUS311790200 - ROSTER/SYS-DATA");
    let ps3 = ps3.to_string_lossy().into_owned();
    let err = reg
        .save_editor_open(
            &ps3,
            "nhl15",
            Platform::Ps3,
            true,
            None,
            &store,
            &defs(),
            &b,
        )
        .err()
        .map(|e| e.to_string())
        .expect("expected a refusal");
    assert!(err.contains("not identified as"), "{err}");

    // A settings save is refused even with acknowledgement.
    let settings = copy_save(
        &tmp,
        "ps3/nhl13/savefiles/BLUS310030000 - SETTINGS/SYS-DATA",
    );
    let settings = settings.to_string_lossy().into_owned();
    let err = reg
        .save_editor_open(
            &settings,
            "nhl13",
            Platform::Ps3,
            true,
            None,
            &store,
            &defs(),
            &b,
        )
        .err()
        .map(|e| e.to_string())
        .expect("expected a refusal");
    assert!(err.contains("settings"), "{err}");

    // A non-database file names the files to choose instead.
    let junk = tmp.0.join("notes.txt");
    std::fs::write(
        &junk,
        b"definitely not a save file, but long enough to be inspected",
    )
    .unwrap();
    let err = reg
        .save_editor_identify(&junk.to_string_lossy(), &store)
        .err()
        .map(|e| e.to_string())
        .expect("expected a refusal");
    assert!(err.contains("SYS-DATA"), "{err}");

    // Game-file writes are refused for a Save Editor session; nothing was backed up or written.
    let opened = reg
        .save_editor_open(
            &ps3,
            "nhl14",
            Platform::Ps3,
            false,
            None,
            &store,
            &defs(),
            &b,
        )
        .unwrap();
    let err = reg
        .dictionary_rename_set(opened.summary.project_id, "anykey", "otherkey")
        .err()
        .map(|e| e.to_string())
        .expect("expected a refusal");
    assert!(err.contains("not available in Save Editor"), "{err}");
    assert!(!b.exists());
}

#[test]
fn manual_metadata_must_describe_the_file() {
    // NHL 20 PS4 has no bundled definitions: refused by default, and a mismatched metadata file
    // supplied by hand is refused too (never borrowed from a neighbouring title silently).
    let tmp = Temp::new("manual");
    let store = store();
    let reg = Registry::default();
    let f = copy_save(&tmp, "ps4/nhl20/savefiles/ThreeOnThree0/DATA");
    let f = f.to_string_lossy().into_owned();
    let id = reg.save_editor_identify(&f, &store).unwrap();
    assert_eq!(id.decision, "refused");
    assert!(id.candidates.is_empty());
    let wrong = repo().join("anchors/ps4/nhl15/databases/nhl_game-meta.xml");
    let err = reg
        .save_editor_open(
            &f,
            "nhl20",
            Platform::Ps4,
            true,
            Some(&wrong.to_string_lossy()),
            &store,
            &defs(),
            &tmp.0.join("b"),
        )
        .err()
        .map(|e| e.to_string())
        .expect("expected a refusal");
    assert!(err.contains("does not describe"), "{err}");
}

#[test]
fn change_list_transactions_save_and_reopen() {
    // Roster moves on NHL 07 / NHL 08 change lists: recorded as the game's roster (and free-agent)
    // companions, saved inside the fixed allocation, and re-read: release one player, move another.
    for (rel, game, platform) in [
        ("xbox360/nhl07/savefiles/ROSTER1/ROSTER1", "nhl07", Platform::Xbox),
        (
            "ps3/nhl08/savefiles/BLUS30062-002-000 - ROSTER/SYS-DATA",
            "nhl08",
            Platform::Ps3,
        ),
    ] {
        let tmp = Temp::new(game);
        let file = copy_save(&tmp, rel);
        let original = std::fs::read(&file).unwrap();
        let path = file.to_string_lossy().into_owned();
        let (store, reg, backups) = (store(), Registry::default(), tmp.0.join("b"));
        let open = |reg: &Registry| {
            reg.save_editor_open(&path, game, platform, false, None, &store, &defs(), &backups)
                .unwrap_or_else(|e| panic!("{rel}: open: {e}"))
                .summary
                .project_id
        };
        let pid = open(&reg);
        let players = reg.list_players(pid).unwrap();
        let mut teams: Vec<u64> = players.iter().filter_map(|p| p.team_id).collect();
        teams.sort();
        teams.dedup();
        let on = |team: u64| {
            players
                .iter()
                .find(|p| p.team_id == Some(team) && p.player_id != 0)
                .unwrap()
                .clone()
        };
        let (a, b) = (on(teams[0]), on(teams[2]));
        // Release first (NHL 08 removes the roster row; NHL 07 frees the slot), then move.
        reg.release_to_fa(pid, teams[2], b.player_id)
            .unwrap_or_else(|e| panic!("{rel}: release: {e}"));
        reg.move_player(pid, a.player_id, teams[0], teams[2])
            .unwrap_or_else(|e| panic!("{rel}: move: {e}"));
        let expect = vec![(a.player_id, Some(teams[2]))];
        reg.save_editor_save(pid).unwrap_or_else(|e| panic!("{rel}: save: {e}"));
        let written = std::fs::read(&file).unwrap();
        assert_eq!(written.len(), original.len(), "{rel}: fixed allocation");
        assert_ne!(written, original);
        assert_eq!(reg.close_project_outcome(pid).outcome, "closed");

        let pid = open(&reg);
        let after = reg.list_players(pid).unwrap();
        let team_of = |id: u64| after.iter().find(|p| p.player_id == id).unwrap().team_id;
        for (id, team) in expect {
            assert_eq!(team_of(id), team, "{rel}: player {id}");
        }
        assert_ne!(team_of(b.player_id), Some(teams[2]), "{rel}: released");
        println!("{rel}: {} {} and {} {} saved and reopened", a.first_name, a.last_name, b.first_name, b.last_name);
    }
}
