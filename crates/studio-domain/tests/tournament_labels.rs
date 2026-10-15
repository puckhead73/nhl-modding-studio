//! League 1.2 tournament templates — read-only browse labels (research R115, 26 Sep 2026; ledger
//! refresh 26 Sep: "implementation-ready, read-only").
//!
//! `wcschedule` (PS4 NHL 17), `championsschedule` and `spenglerschedule` (PS4 NHL 18–21) list games
//! between numbered positions. Studio labels them without binding them: round 0 is the group stage,
//! knockout rounds are named by their seed count, the World Cup group stage shows club names through
//! R115's one consistent join (slot = rank by team index among the World Cup league's members, which
//! agrees with the group ids), the World Cup knockout placeholder is "to be decided", Spengler 6–9
//! read as "winner or loser of an opener" **labelled as an interpretation**, and CHL / Spengler
//! participant slots stay unnamed. The tables stay unbound (no slot map), so nothing here can be
//! edited, imported, exported or generated through the schedule editor.
use std::path::{Path, PathBuf};

use studio_domain::{EditSession, Platform, Project, ProjectRole};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn ps4(game: &str) -> EditSession {
    let dir = format!("anchors/ps4/{game}/databases");
    let project = Project::load(repo(&format!("{dir}/nhl_game.db")), Platform::Ps4, ProjectRole::Nhlng, repo(&format!("{dir}/nhl_game-meta.xml")), repo("defs")).unwrap();
    let mut s = EditSession::begin(project);
    s.set_game_id(Some(game.to_string()));
    s
}

fn view(s: &EditSession, name: &str) -> studio_domain::schedules::ScheduleView {
    let t = s.schedule_tables().unwrap().into_iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name}"));
    s.schedule_view(&t.studio_table).unwrap().unwrap()
}

#[test]
fn the_world_cup_group_stage_is_named_by_the_consistent_join_and_the_knockout_is_to_be_decided() {
    let s = ps4("nhl17");
    let v = view(&s, "wcschedule");
    assert_eq!(v.games.len(), 17, "R115: 17 rows");
    assert!(v.slot_map.entries.is_empty(), "unbound: the editor's picker, import and generation stay off");
    assert!(v.games.iter().all(|g| g.home.is_none() && g.away.is_none()), "no team binding");
    let note = v.tournament_note.as_deref().unwrap();
    assert!(note.contains("R115") && note.contains("Browse-only"), "{note}");
    let stage: Vec<_> = v.games.iter().filter(|g| g.round == 0).collect();
    assert_eq!(stage.len(), 12);
    let mut names = std::collections::BTreeMap::new();
    for g in &stage {
        assert_eq!(g.round_label.as_deref(), Some("group stage"));
        names.insert(g.home_slot, g.home_label.clone().unwrap());
        names.insert(g.away_slot, g.away_label.clone().unwrap());
    }
    eprintln!("NHL 17 World Cup slots: {names:?}");
    assert_eq!(names.len(), 8);
    assert!(names.values().all(|n| !n.trim().is_empty() && !n.starts_with("participant slot")), "every group-stage slot named: {names:?}");
    // R115 J1's groups, displayed through the native full team names.
    let group = |slots: [u64; 4]| -> std::collections::BTreeSet<String> { slots.iter().map(|s| names[s].clone()).collect() };
    assert_eq!(group([0, 1, 2, 7]), ["Canada (World Cup)", "Czech Republic (World Cup)", "Europe (World Cup)", "USA (World Cup)"].iter().map(|s| s.to_string()).collect(), "{names:?}");
    assert_eq!(group([3, 4, 5, 6]), ["Finland (World Cup)", "North America (World Cup)", "Russia (World Cup)", "Sweden (World Cup)"].iter().map(|s| s.to_string()).collect(), "{names:?}");
    // R115 J1: groups {0, 1, 2, 7} and {3, 4, 5, 6} are the two World Cup groups (CAN / CZE / EUR / USA,
    // FIN / NA / RUS / SWE): each group plays only within itself.
    for g in &stage {
        let a = [0u64, 1, 2, 7].contains(&g.home_slot);
        let b = [0u64, 1, 2, 7].contains(&g.away_slot);
        assert_eq!(a, b, "group-stage game across groups: {} v {}", g.home_slot, g.away_slot);
    }
    for g in v.games.iter().filter(|g| g.round != 0) {
        assert_eq!((g.home_slot, g.away_slot), (8, 8), "R115: knockout placeholders are 8 v 8");
        assert_eq!(g.home_label.as_deref(), Some("to be decided"));
        assert!(g.round_label.as_deref().unwrap().contains("to be decided"));
    }
    assert!(!v.findings.iter().any(|f| f.contains("dangling")), "the placeholder is not reported as a dangling slot: {:?}", v.findings);
}

#[test]
fn champions_and_spengler_slots_stay_unnamed_with_seed_and_interpretation_labels() {
    for game in ["nhl18", "nhl19", "nhl21"] {
        let s = ps4(game);
        let chl = view(&s, "championsschedule");
        assert_eq!(chl.games.len(), 125, "{game}: R115 CHL rows");
        assert!(chl.slot_map.entries.is_empty() && chl.games.iter().all(|g| g.home.is_none()), "{game}: unbound");
        for g in &chl.games {
            let (rl, hl) = (g.round_label.as_deref().unwrap(), g.home_label.as_deref().unwrap());
            match g.round {
                0 => assert!(rl == "group stage" && hl.starts_with("participant slot"), "{game}: {rl} / {hl}"),
                3 => assert!(rl.contains("16 seeds") && hl.starts_with("seed"), "{game}: {rl}"),
                4 => assert!(rl.contains("8 seeds"), "{game}: {rl}"),
                5 => assert!(rl.contains("4 seeds"), "{game}: {rl}"),
                6 => assert!(rl.contains("final"), "{game}: {rl}"),
                r => panic!("{game}: unexpected CHL round {r}"),
            }
        }
        assert!(chl.tournament_note.as_deref().unwrap().contains("not established"));

        let sp = view(&s, "spenglerschedule");
        assert_eq!(sp.games.len(), 11, "{game}: R115 Spengler rows");
        assert!(sp.slot_map.entries.is_empty(), "{game}: unbound");
        let placeholders: Vec<String> = sp
            .games
            .iter()
            .filter(|g| g.round == 0)
            .flat_map(|g| [(g.home_slot, g.home_label.clone().unwrap()), (g.away_slot, g.away_label.clone().unwrap())])
            .filter(|(s, _)| *s >= 6)
            .map(|(_, l)| l)
            .collect();
        eprintln!("{game} Spengler placeholders: {placeholders:?}");
        assert!(!placeholders.is_empty() && placeholders.iter().all(|l| l.contains("opener") && l.contains("(interpretation)")), "{game}: {placeholders:?}");
        // R115: 6 / 7 only oppose club 2 and belong to the 0 v 1 opener; 8 / 9 to the 3 v 4 opener.
        assert!(sp.games.iter().filter(|g| g.round == 0 && (g.home_slot == 6 || g.away_slot == 6)).all(|g| g.home_label.as_deref().unwrap().contains("slot 0 v slot 1") || g.away_label.as_deref().unwrap().contains("slot 0 v slot 1")), "{game}");
        assert!(sp.games.iter().filter(|g| g.round != 0).all(|g| g.home_label.as_deref().unwrap().starts_with("seed")), "{game}");
    }
}

#[test]
fn league_tables_carry_no_tournament_labels() {
    let s = ps4("nhl21");
    let v = view(&s, "nhlschedule");
    assert!(v.tournament_note.is_none());
    assert!(v.games.iter().all(|g| g.round_label.is_none() && g.home_label.is_none() && g.away_label.is_none()));
    assert!(!v.slot_map.entries.is_empty(), "league tables stay bound and editable");
}
