//! Phase 1.2 — source schedule browse/validation on the committed anchors (research handoff
//! P1.2-X): tables classified and matched to leagues, slots resolved through the team `index`,
//! the Legacy opener is TOR vs MTL on Oct 7, games per team are 82 with the two All-Star slots
//! unresolved, and the per-title exceptions (NHL 10's 1 230 rows, PS4's 31 teams) hold.

use std::path::{Path, PathBuf};

use studio_domain::{EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn session(game: &str, platform: Platform, db: &str, meta: &str) -> EditSession {
    let project = Project::load(repo(db), platform, ProjectRole::Nhlng, repo(meta), repo("defs")).unwrap();
    let mut s = EditSession::begin(project);
    s.set_game_id(Some(game.to_string()));
    s
}

#[test]
fn legacy_nhl_schedule_resolves_slots_and_validates() {
    let s = session("nhllegacy", Platform::Xbox, "anchors/xbox360/nhllegacy/databases/nhlng.db", "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
    let tables = s.schedule_tables().unwrap();
    let names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
    assert!(names.contains(&"nhlschedule") && names.contains(&"swedishschedule") && names.contains(&"leaguescheduletable"));
    let nhl = tables.iter().find(|t| t.name == "nhlschedule").unwrap();
    assert_eq!((nhl.rows, nhl.role), (1231, "source"));
    assert_eq!(nhl.league_code, Some(0));
    let mode_local = tables.iter().find(|t| t.name == "leaguescheduletable").unwrap();
    assert_eq!((mode_local.rows, mode_local.role, mode_local.league_code), (0, "mode-local", None));
    let dbg = tables.iter().find(|t| t.name == "nhlscheduledebug").unwrap();
    assert_eq!((dbg.role, dbg.league_code), ("debug", Some(0)));
    let swe = tables.iter().find(|t| t.name == "swedishschedule").unwrap();
    assert_eq!(swe.league_code, Some(2), "{}", swe.league_match);

    let v = s.schedule_view(&nhl.studio_table).unwrap().unwrap();
    assert_eq!(v.games.len(), 1231);
    let g0 = &v.games[0];
    assert_eq!((g0.month, g0.day, g0.date.as_str()), (9, 6, "Oct 7"));
    assert!(g0.home.as_deref().unwrap().contains("Toronto"), "{:?}", g0.home);
    assert!(g0.away.as_deref().unwrap().contains("Montr"), "{:?}", g0.away);
    assert_eq!(v.first_date.as_deref(), Some("Oct 7"));
    assert_eq!(v.last_date.as_deref(), Some("Apr 9"));
    // 30 NHL teams at 82 games plus the two All-Star squads (league members at slots 30/31, one
    // game each — the single All-Star row).
    assert_eq!(v.slots.len(), 32);
    assert_eq!(v.slots.iter().filter(|sc| sc.games == 82).count(), 30);
    let stars: Vec<&studio_domain::schedules::SlotCount> = v.slots.iter().filter(|sc| sc.games == 1).collect();
    assert_eq!(stars.iter().map(|s| s.slot).collect::<Vec<_>>(), [30, 31]);
    assert!(v.unresolved_slots.is_empty(), "{:?}", v.unresolved_slots);
    assert_eq!(v.findings.len(), 1, "{:?}", v.findings);
    assert!(v.findings[0].contains("All-Star") && v.findings[0].contains("82"), "{}", v.findings[0]);
    assert!(v.fields.contains_key("home") && v.fields.contains_key("status"));

    // The Swedish table resolves its 14 slots to the SHL teams; 52 games each.
    let sv = s.schedule_view(&swe.studio_table).unwrap().unwrap();
    assert_eq!(sv.games.len(), 364);
    assert_eq!(sv.slots.len(), 14);
    assert!(sv.slots.iter().all(|sc| sc.games == 52));
    assert!(sv.unresolved_slots.is_empty() && sv.findings.is_empty(), "{:?}", sv.findings);
}

#[test]
fn nhl10_has_no_all_star_row_and_ps4_has_31_teams() {
    let s10 = session("nhl10", Platform::Ps3, "anchors/ps3/nhl10/databases/nhlng.db", "anchors/ps3/nhl10/databases/nhlng-meta.xml");
    let t = s10.schedule_tables().unwrap();
    let nhl = t.iter().find(|t| t.name == "nhlschedule").unwrap();
    assert_eq!(nhl.rows, 1230);
    let v = s10.schedule_view(&nhl.studio_table).unwrap().unwrap();
    assert_eq!(v.slots.len(), 30, "no All-Star row in 2009-10: only the 30 clubs have games");
    assert!(v.unresolved_slots.is_empty());
    // The two All-Star team rows exist in league 0 but have no games this year: disclosed.
    assert_eq!(v.findings.len(), 2, "{:?}", v.findings);
    assert!(v.findings.iter().all(|f| f.contains("All-Stars") && f.contains("no games")), "{:?}", v.findings);

    let s21 = session("nhl21", Platform::Ps4, "anchors/ps4/nhl21/databases/nhl_game.db", "anchors/ps4/nhl21/databases/nhl_game-meta.xml");
    let t = s21.schedule_tables().unwrap();
    let nhl = t.iter().find(|t| t.name == "nhlschedule").unwrap();
    assert_eq!(nhl.rows, 1271);
    assert_eq!(nhl.league_code, Some(0), "{}", nhl.league_match);
    let v = s21.schedule_view(&nhl.studio_table).unwrap().unwrap();
    assert!(!v.fields.contains_key("status"), "PS4 rows have no status field");
    assert_eq!(v.slots.len(), 31);
    assert!(v.slots.iter().all(|sc| sc.games == 82));
    assert!(v.unresolved_slots.is_empty());
    let nhl32 = t.iter().find(|t| t.name == "nhl32schedule").unwrap();
    assert_eq!(nhl32.rows, 1312);
    let v32 = s21.schedule_view(&nhl32.studio_table).unwrap().unwrap();
    assert_eq!(v32.slots.len(), 32, "the 32-team layout resolves against the league's members (Seattle is a stock row)");
    assert!(v32.slots.iter().all(|sc| sc.games == 82));
}
