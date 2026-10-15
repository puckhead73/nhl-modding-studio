//! N21-001 — PS4 onboarding with an independent game folder and save-data folder, through the public
//! Registry path the onboarding page calls (`detect_install` → `open_workspace`).
//!
//! Fixture: owned scratch copies of the genuine NHL 16 PS4 anchor (`anchors/ps4/nhl16`: `nhl_game.db`,
//! its metadata, `eng_us.db`, the `ROSTER` save's `DATA`) laid out as a game folder that is not named
//! after its title id, with a real-shaped `param.sfo`, plus roaming-shaped save data elsewhere. The
//! shared NHL 15–21 contract is exercised on this one genuine title; nothing here qualifies any other
//! title at runtime. Nothing outside the scratch directory is read except the committed anchors.
use std::path::{Path, PathBuf};

use studio_api::{parse_platform, Registry};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

/// Owned scratch, removed on drop.
struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-n21-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Tmp(dir)
    }
}
impl Drop for Tmp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn copy(from: PathBuf, to: &Path) {
    std::fs::create_dir_all(to.parent().unwrap()).unwrap();
    std::fs::copy(&from, to).unwrap_or_else(|e| panic!("copy {} -> {}: {e}", from.display(), to.display()));
}

/// A minimal `param.sfo` with utf-8 string keys (the shape shadPS4 installs carry).
fn sfo(pairs: &[(&str, &str)]) -> Vec<u8> {
    let (mut keys, mut vals, mut index) = (Vec::new(), Vec::new(), Vec::new());
    for (k, v) in pairs {
        let mut val = v.as_bytes().to_vec();
        val.push(0);
        index.extend_from_slice(&(keys.len() as u16).to_le_bytes());
        index.extend_from_slice(&0x0204u16.to_le_bytes());
        index.extend_from_slice(&(val.len() as u32).to_le_bytes());
        index.extend_from_slice(&(val.len() as u32).to_le_bytes());
        index.extend_from_slice(&(vals.len() as u32).to_le_bytes());
        keys.extend_from_slice(k.as_bytes());
        keys.push(0);
        vals.extend_from_slice(&val);
    }
    let key_table = 0x14 + index.len();
    let mut d = b"\x00PSF".to_vec();
    d.extend_from_slice(&0x0101_0000u32.to_le_bytes());
    d.extend_from_slice(&(key_table as u32).to_le_bytes());
    d.extend_from_slice(&((key_table + keys.len()) as u32).to_le_bytes());
    d.extend_from_slice(&(pairs.len() as u32).to_le_bytes());
    d.extend_from_slice(&index);
    d.extend_from_slice(&keys);
    d.extend_from_slice(&vals);
    d
}

/// The NHL 16 anchor as a game folder at `game`.
fn nhl16_game(game: &Path) {
    let a = repo("anchors/ps4/nhl16/databases");
    let t3db = game.join("nhl/data/common/gamemodes/database/t3db");
    copy(a.join("nhl_game.db"), &t3db.join("nhl_game.db"));
    copy(a.join("nhl_game-meta.xml"), &t3db.join("nhl_game-meta.xml"));
    copy(a.join("eng_us.db"), &game.join("nhl/data/kettle/loc/eng_us.db"));
    std::fs::create_dir_all(game.join("sce_sys")).unwrap();
    std::fs::write(
        game.join("sce_sys/param.sfo"),
        sfo(&[("TITLE", "NHL™ 16"), ("TITLE_ID", "CUSA01909"), ("CONTENT_ID", "EP0006-CUSA01909_00-NHLICEHOCKEY2016"), ("APP_VER", "01.00")]),
    )
    .unwrap();
}

/// The genuine NHL 16 roster save at `<savedata>/<profile>/CUSA01909/<save>/DATA`.
fn roster(savedata: &Path, profile: &str, save: &str) -> PathBuf {
    let data = savedata.join(profile).join("CUSA01909").join(save).join("DATA");
    copy(repo("anchors/ps4/nhl16/savefiles/84f8102f - ROSTER/DATA"), &data);
    data
}

fn defs() -> String {
    s(&repo("defs"))
}

#[test]
fn ps4_game_and_save_locations_travel_from_detection_to_the_opened_roster() {
    let t = Tmp::new("open");
    let game = t.0.join("D-drive/PS4 games/hockey sixteen");
    nhl16_game(&game);
    let savedata = t.0.join("C-drive/Users/tester/AppData/Roaming/shadPS4/savedata");
    let first = roster(&savedata, "1", "RosterCustom0");
    let second = roster(&savedata, "2", "RosterCustom0");
    let before = std::fs::read(&second).unwrap();
    let ps4 = parse_platform("ps4").unwrap();
    let reg = Registry::default();

    // Detection: both locations, every profile's roster, identity from the package metadata.
    let r = reg.detect_install("nhl16", ps4, &s(&game), Some(&s(&savedata)));
    assert!(r.problems.is_empty(), "{:?}", r.problems.iter().map(|p| &p.message).collect::<Vec<_>>());
    assert_eq!(r.game_dir.as_deref(), Some(s(&game).as_str()));
    assert_eq!(r.save_root.as_deref(), Some(s(&savedata).as_str()));
    assert!(r.minimum_met && !r.extract_needed);
    assert!(r.nhlng_path.as_deref().is_some_and(|p| p.starts_with(&s(&game))), "database from the game folder");
    assert!(r.install_dir.as_deref().is_some_and(|p| p.starts_with(&s(&game))), "install / asset anchor from the game folder");
    let rosters: Vec<_> = r.saves.iter().filter(|x| x.kind == "roster").collect();
    assert_eq!(rosters.len(), 2);
    assert!(rosters.iter().all(|x| x.writable && x.title_id.as_deref() == Some("CUSA01909")));
    assert_eq!(rosters.iter().map(|x| x.profile.as_deref().unwrap()).collect::<Vec<_>>(), ["1", "2"]);
    assert_eq!(r.game_candidates.len(), 1);
    assert_eq!(r.game_candidates[0].title.as_deref(), Some("NHL™ 16"));

    // Two roster saves and no choice: refused, naming both, and nothing is written.
    let sidecar = Path::new(r.install_dir.as_deref().unwrap()).join(".nhlmodproj");
    let err = reg.open_workspace("nhl16", ps4, &s(&game), Some(&s(&savedata)), None, true, &defs()).err().expect("refused").to_string();
    assert!(err.contains("2 roster saves were found") && err.contains("profile 1") && err.contains("profile 2"), "{err}");
    assert!(!sidecar.exists(), "a refused open writes no sidecar");

    // A choice outside the inventory is refused.
    let stray = t.0.join("elsewhere/DATA");
    copy(repo("anchors/ps4/nhl16/savefiles/84f8102f - ROSTER/DATA"), &stray);
    let err = reg.open_workspace("nhl16", ps4, &s(&game), Some(&s(&savedata)), Some(&s(&stray)), true, &defs()).err().expect("refused").to_string();
    assert!(err.contains("isn't one of the roster saves"), "{err}");

    // The explicit choice opens exactly that save as the edited file.
    let summary = reg.open_workspace("nhl16", ps4, &s(&game), Some(&s(&savedata)), Some(&s(&second)), true, &defs()).unwrap();
    let pending = reg.pending_changes(summary.project_id, 0).unwrap();
    assert_eq!(Path::new(&pending.primary.path), second.as_path(), "the chosen profile-2 roster is the edited file");
    assert_eq!(summary.role, "roster");
    assert!(summary.dictionary_problem.is_none(), "the game folder's eng_us.db attached");
    assert_eq!(std::fs::read(&second).unwrap(), before, "opening writes nothing to the save");
    // ONB-002: opening no longer writes the install-side `.nhlmodproj`; the project folder
    // (tests/project_workspace.rs) is where the opened install is recorded now.
    assert!(!sidecar.exists(), "ONB-002: an open writes no sidecar into the game's database folder");

    // Reopening with the remembered pair still resolves; a remembered choice that disappeared is refused.
    let again = reg.open_workspace("nhl16", ps4, &s(&game), Some(&s(&savedata)), Some(&s(&second)), true, &defs()).unwrap();
    assert_eq!(Path::new(&reg.pending_changes(again.project_id, 0).unwrap().primary.path), second.as_path());
    std::fs::remove_file(&second).unwrap();
    let err = reg.open_workspace("nhl16", ps4, &s(&game), Some(&s(&savedata)), Some(&s(&second)), true, &defs()).err().expect("refused").to_string();
    assert!(err.contains("isn't one of the roster saves"), "{err}");
    // With one roster left, no choice is needed.
    let one = reg.open_workspace("nhl16", ps4, &s(&game), Some(&s(&savedata)), None, true, &defs()).unwrap();
    assert_eq!(Path::new(&reg.pending_changes(one.project_id, 0).unwrap().primary.path), first.as_path());
}

#[test]
fn ps4_older_emulator_folder_configuration_still_opens_and_wrong_titles_refuse() {
    let t = Tmp::new("legacy");
    // The configuration r67 remembered: the shadPS4 folder as the root, no save location.
    let emu = t.0.join("Shadps4");
    nhl16_game(&emu.join("Games/CUSA01909"));
    let data = roster(&emu.join("user/savedata"), "1", "RosterCustom0");
    let ps4 = parse_platform("ps4").unwrap();
    let reg = Registry::default();
    let r = reg.detect_install("nhl16", ps4, &s(&emu), None);
    assert!(r.problems.is_empty() && r.minimum_met, "{:?}", r.problems.iter().map(|p| &p.message).collect::<Vec<_>>());
    let summary = reg.open_workspace("nhl16", ps4, &s(&emu), None, None, true, &defs()).unwrap();
    assert_eq!(Path::new(&reg.pending_changes(summary.project_id, 0).unwrap().primary.path), data.as_path());

    // The NHL 16 folder opened as NHL 17 is refused with the reason, before anything is written.
    let game = emu.join("Games/CUSA01909");
    let err = reg.open_workspace("nhl17", ps4, &s(&game), None, None, true, &defs()).err().expect("refused").to_string();
    assert!(err.contains("different game") && err.contains("NHL™ 16 (CUSA01909)") && err.contains("NHL 17"), "{err}");
}

/// The operator's real installs, detection only (directory listings, `param.sfo`, save headers —
/// nothing is opened or written; `open_workspace` would write the profile sidecar into the game
/// folder, so it is not called here). The NHL 21 game folder under the shadPS4 library and this
/// user's roaming save data are chosen independently, as N21-001 reported them.
#[test]
#[ignore = "reads H:/Emulators/ShadPS4/Games and %APPDATA%/shadPS4/savedata (read only): select with --ignored"]
fn ps4_real_installs_resolve_from_their_own_folders() {
    let games = Path::new(r"H:\Emulators\ShadPS4\Games");
    let savedata = studio_domain::ps4_default_save_root().expect("APPDATA");
    assert!(games.is_dir() && savedata.is_dir(), "real locations missing (fails, never skips)");
    let ps4 = parse_platform("ps4").unwrap();
    let reg = Registry::default();
    let r = reg.detect_install("nhl21", ps4, &s(&games.join("CUSA18182")), Some(&s(&savedata)));
    let c = &r.game_candidates[0];
    assert!(c.matches && c.title_id.as_deref() == Some("CUSA18182"), "{:?}", c.reason);
    assert!(r.nhlng_path.is_some() && r.meta_xml_path.is_some(), "NHL 21's database from its own folder: {:?}", r.problems.iter().map(|p| &p.message).collect::<Vec<_>>());
    assert!(r.saves.iter().any(|x| x.kind == "roster" && x.profile.as_deref() == Some("1") && x.title_id.as_deref() == Some("CUSA18182")));
    eprintln!("nhl21: db {:?}; saves {:?}", r.nhlng_path, r.saves.iter().map(|x| (&x.kind, &x.profile, x.writable, &x.problem)).collect::<Vec<_>>());
    // Each installed PS4 title picked directly by its own folder: identified from its param.sfo only.
    for (gid, dir) in [("nhl15", "CUSA00561"), ("nhl16", "CUSA01909"), ("nhl17", "CUSA04326"), ("nhl18", "CUSA07569"), ("nhl19", "CUSA11126"), ("nhl20", "CUSA15439"), ("nhl21", "CUSA18182")] {
        let r = reg.detect_install(gid, ps4, &s(&games.join(dir)), Some(&s(&savedata)));
        assert!(
            r.game_candidates.len() == 1 && r.game_candidates[0].matches,
            "{gid}: {:?}",
            r.game_candidates.iter().map(|c| (&c.title, &c.reason)).collect::<Vec<_>>()
        );
        assert!(r.game_dir.is_some(), "{gid}");
        eprintln!("{gid}: {:?} {:?} db={} extract_needed={}", r.game_candidates[0].title, r.game_candidates[0].content_id, r.nhlng_path.is_some(), r.extract_needed);
    }
    // FIFA 16 sits in the same library and is never NHL 16.
    let r = reg.detect_install("nhl16", ps4, &s(&games.join("CUSA02138")), None);
    assert!(r.game_dir.is_none() && r.problems[0].kind == "game_wrong_title");
}

/// N21-001-R1: the database-only open the onboarding page sends after an explicit choice (no roster,
/// none recognised) opens the template database, not a save; and an incomplete save inventory reaches
/// the DTO as its own problem instead of "no saves".
#[test]
fn ps4_database_only_open_and_incomplete_inventory_through_the_registry() {
    let t = Tmp::new("dbonly");
    let game = t.0.join("game");
    nhl16_game(&game);
    let empty = t.0.join("savedata");
    std::fs::create_dir_all(empty.join("1")).unwrap();
    let ps4 = parse_platform("ps4").unwrap();
    let reg = Registry::default();
    let r = reg.detect_install("nhl16", ps4, &s(&game), Some(&s(&empty)));
    assert!(r.saves.is_empty() && r.minimum_met);
    assert_eq!(r.problems.iter().map(|p| p.kind.as_str()).collect::<Vec<_>>(), ["saves_not_found"]);
    let summary = reg.open_workspace("nhl16", ps4, &s(&game), Some(&s(&empty)), None, true, &defs()).unwrap();
    assert_ne!(summary.role, "roster", "database only");
    let pending = reg.pending_changes(summary.project_id, 0).unwrap();
    assert!(pending.primary.path.ends_with("nhl_game.db"), "{}", pending.primary.path);

    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        roster(&empty, "2", "RosterCustom0");
        let locked = empty.join("2");
        let _hold = std::fs::OpenOptions::new().read(true).share_mode(0).custom_flags(0x0200_0000).open(&locked).unwrap();
        assert!(std::fs::read_dir(&locked).is_err());
        let r = reg.detect_install("nhl16", ps4, &s(&game), Some(&s(&empty)));
        assert!(r.saves.is_empty());
        let kinds: Vec<&str> = r.problems.iter().map(|p| p.kind.as_str()).collect();
        assert_eq!(kinds, ["save_inventory_incomplete"], "never \"no saves\" from an incomplete scan");
        assert!(r.problems[0].message.contains(&s(&locked)), "{}", r.problems[0].message);
    }
}

#[test]
fn ps3_onboarding_ignores_the_ps4_roster_choice() {
    // Shared seam: the PS3 route opens exactly as before whatever `roster_save` carries.
    let t = Tmp::new("ps3");
    let a = repo("anchors/ps3/nhl14");
    let usrdir = t.0.join("dev_hdd0/game/BLUS31179/USRDIR");
    copy(a.join("databases/nhlng.db"), &usrdir.join("nhlng.db"));
    copy(a.join("databases/nhlng-meta.xml"), &usrdir.join("nhlng-meta.xml"));
    copy(a.join("databases/nhl_eng_us.db"), &usrdir.join("nhl_eng_us.db"));
    copy(a.join("savefiles/BLUS311790200 - ROSTER/SYS-DATA"), &t.0.join("dev_hdd0/home/00000001/savedata/BLUS311790200 - ROSTER/SYS-DATA"));
    let ps3 = parse_platform("ps3").unwrap();
    let reg = Registry::default();
    let r = reg.detect_install("nhl14", ps3, &s(&t.0), Some("ignored"));
    assert!(r.minimum_met && r.problems.is_empty() && r.game_candidates.is_empty() && r.save_root.is_none());
    let summary = reg.open_workspace("nhl14", ps3, &s(&t.0), None, Some("not-a-save"), true, &defs()).unwrap();
    assert!(summary.roster_attached, "the PS3 roster mirror attaches as before");
}
