//! Phase 1.2 — league membership as one validated operation across the three group models
//! (global groups on NHL 12 → Legacy and, since P1-C, PS4 15 → 21; local bits on NHL 08 → 11;
//! the PS4 foreign key with raw ids where no table fits) plus the
//! explicit NHL 07 refusal. Anchors are opened from temp copies where a save happens; here the
//! edits stay in memory.

use std::path::{Path, PathBuf};

use studio_domain::membership::GroupModel;
use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}

fn open(game: &str, platform: Platform, db: &str, meta: &str) -> Workspace {
    let project = Project::load(repo(db), platform, ProjectRole::Nhlng, repo(meta), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), repo(db).to_string_lossy().into_owned());
    ws.set_game_id(game);
    ws
}

fn team_by_abbrev(ws: &Workspace, abbrev: &str) -> u64 {
    ws.display()
        .team_summaries()
        .unwrap()
        .iter()
        .find(|t| t.abbrev == abbrev)
        .unwrap_or_else(|| panic!("team {abbrev}"))
        .team_id
}

#[test]
fn legacy_moves_a_team_with_validated_global_group_ids() {
    let mut ws = open(
        "nhllegacy",
        Platform::Xbox,
        "anchors/xbox360/nhllegacy/databases/nhlng.db",
        "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml",
    );
    ws.attach_dictionary(repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db").to_str().unwrap())
        .unwrap();
    let bos = team_by_abbrev(&ws, "BOS");
    let opts = ws.team_membership_options(bos).unwrap();
    assert_eq!(opts.model, GroupModel::GlobalGroups);
    assert_eq!(opts.group_evidence.as_deref(), Some("traced"));
    assert_eq!(opts.current.league_label, "NHL\u{ae}");
    assert_eq!(opts.current.conference.as_ref().unwrap().label, "Eastern Conference");
    assert_eq!(opts.current.division.as_ref().unwrap().label, "Atlantic Division");
    let nhl = opts.groups_by_league.iter().find(|g| g.league_code == 0).unwrap();
    assert_eq!(nhl.league_group, 0);
    assert_eq!(nhl.conferences.len(), 2);
    assert_eq!(nhl.divisions.len(), 4);
    let norway = opts.groups_by_league.iter().find(|g| g.league_code == 7).unwrap();
    assert!(norway.conferences.is_empty() && norway.divisions.is_empty());
    assert_eq!(norway.league_group, 19);

    // Refusals: unknown league; NHL without a conference; an AHL division under the NHL.
    assert!(ws.set_team_membership(bos, 77, None, None).is_err());
    let err = ws.set_team_membership(bos, 0, None, None).unwrap_err().to_string();
    assert!(err.contains("choose one"), "{err}");
    let err = ws.set_team_membership(bos, 0, Some(1), Some(10)).unwrap_err().to_string();
    assert!(err.contains("not paired with Western Conference"), "{err}");
    // R1 F1: Western + Atlantic are both NHL ids but not a witnessed pair; the All-Star
    // arrangement (a conference id in the division slot) is exceptional and never offered.
    let err = ws.set_team_membership(bos, 0, Some(1), Some(5)).unwrap_err().to_string();
    assert!(err.contains("not paired with Western Conference"), "{err}");
    let err = ws.set_team_membership(bos, 0, Some(1), Some(1)).unwrap_err().to_string();
    assert!(err.contains("exceptional stock arrangement"), "{err}");
    assert_eq!(ws.undo_depth(), 0, "refusals leave no history");

    // Move Boston West/Pacific: one transaction, four cells, resolved before/after.
    let (plan, outcome) = ws.set_team_membership(bos, 0, Some(1), Some(3)).unwrap();
    assert_eq!(outcome.edits, 4, "four cells written in one transaction");
    assert_eq!(plan.writes.len(), 4);
    assert_eq!(plan.after.conference.as_ref().unwrap().label, "Western Conference");
    assert_eq!(plan.after.division.as_ref().unwrap().label, "Pacific Division");
    assert_eq!(ws.undo_depth(), 1);
    let view = ws.display().team_view(bos).unwrap().unwrap();
    assert_eq!(view.conference_label.as_deref(), Some("Western Conference"));
    assert_eq!(view.division_label.as_deref(), Some("Pacific Division"));
    let (pending, _) = ws.pending_changes(100);
    assert_eq!(pending.total_cells, 2);

    // Move Boston to Norway (single group): its own id in all three fields; explicit other ids refused.
    let err = ws.set_team_membership(bos, 7, Some(1), None).unwrap_err().to_string();
    assert!(err.contains("single-group"), "{err}");
    // R2 F1 on the shared gen7 path: each supplied slot is checked on its own.
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let (depth, redo) = (ws.undo_depth(), ws.redo_depth());
    let pending_before = ws.pending_changes(100).0.total_cells;
    for (conf, div, slot, what) in [
        (Some(19), Some(3), "division", "correct conference + wrong division"),
        (Some(1), Some(19), "conference", "wrong conference + correct division"),
        (None, Some(3), "division", "omitted conference + wrong division"),
    ] {
        let err = ws.set_team_membership(bos, 7, conf, div).unwrap_err().to_string();
        assert!(err.contains("single-group") && err.contains(&format!("the {slot} given (")), "{what}: {err}");
        assert_eq!(ws.primary().bytes(), &bytes[..], "{what}: bytes unchanged");
        assert_eq!(ws.revision(), rev, "{what}: revision unchanged");
        assert_eq!((ws.undo_depth(), ws.redo_depth()), (depth, redo), "{what}: history unchanged");
        assert_eq!(ws.pending_changes(100).0.total_cells, pending_before, "{what}: pending unchanged");
    }
    for (conf, div) in [(Some(19), Some(19)), (Some(19), None), (None, Some(19))] {
        let (plan, _) = ws.set_team_membership(bos, 7, conf, div).unwrap();
        assert_eq!(plan.writes.iter().filter(|(_, v)| *v == 19).count(), 3, "{conf:?}/{div:?}");
        assert!(ws.undo());
    }
    let (plan, _) = ws.set_team_membership(bos, 7, None, None).unwrap();
    assert_eq!(plan.after.league_label, "Norway");
    assert!(plan.writes.iter().filter(|(_, v)| *v == 19).count() == 3);
    let view = ws.display().team_view(bos).unwrap().unwrap();
    assert_eq!(view.league_label.as_deref(), Some("Norway"));
    assert_eq!(view.conference_label.as_deref(), Some("Norway"));
    // Undo both: back to the baseline.
    assert!(ws.undo() && ws.undo());
    assert!(!ws.is_dirty());
    let view = ws.display().team_view(bos).unwrap().unwrap();
    assert_eq!(view.division_label.as_deref(), Some("Atlantic Division"));
}

#[test]
fn nhl11_uses_local_bits_and_says_the_names_are_untraced() {
    let mut ws = open(
        "nhl11",
        Platform::Ps3,
        "anchors/ps3/nhl11/databases/nhlng.db",
        "anchors/ps3/nhl11/databases/nhlng-meta.xml",
    );
    let bos = team_by_abbrev(&ws, "BOS");
    let opts = ws.team_membership_options(bos).unwrap();
    assert_eq!(opts.model, GroupModel::LocalBits);
    assert!(opts.groups_by_league.is_empty());
    // Receipt 44 (R133): NHL 11 conferences carry the executable's name by position, marked inferred.
    assert_eq!(opts.current.conference.as_ref().unwrap().label, "Eastern Conference (inferred)");
    assert!(opts.current.division.as_ref().unwrap().label.starts_with("Division 0: BOS · BUF"));
    assert!(opts.notes.iter().any(|n| n.contains("not traced")));
    // League code only; then with local numbers; out-of-range division refused by the metadata.
    let (plan, outcome) = ws.set_team_membership(bos, 1, None, None).unwrap();
    assert_eq!(plan.writes.len(), 1);
    assert_eq!(outcome.edits, 1);
    let (plan, _) = ws.set_team_membership(bos, 0, Some(1), Some(2)).unwrap();
    assert_eq!(plan.writes.len(), 3);
    assert!(ws.set_team_membership(bos, 0, Some(1), Some(9)).is_err(), "division is 3 bits");
    assert_eq!(ws.undo_depth(), 2);
}

#[test]
fn ps4_refuses_a_dangling_league_and_validates_group_ids_through_its_table() {
    // P1-C: with the NHL 21 PS4 table baked (research 17) the PS4 title takes the same
    // global-groups path as Legacy; the raw model remains only where no table fits (see
    // tests/ps4_groups.rs for the full matrix).
    let mut ws = open(
        "nhl21",
        Platform::Ps4,
        "anchors/ps4/nhl21/databases/nhl_game.db",
        "anchors/ps4/nhl21/databases/nhl_game-meta.xml",
    );
    let bos = team_by_abbrev(&ws, "BOS");
    let opts = ws.team_membership_options(bos).unwrap();
    assert_eq!(opts.model, GroupModel::GlobalGroups);
    assert_eq!(opts.group_evidence.as_deref(), Some("traced"));
    assert_eq!(opts.current.conference.as_ref().unwrap().label, "Eastern Conference");
    let ahl = opts
        .leagues
        .iter()
        .find(|l| l.label == "AHL")
        .expect("AHL resolves natively through stockleagues (key shown when no dictionary)")
        .value;
    let err = ws.set_team_membership(bos, 99, None, None).unwrap_err().to_string();
    assert!(err.contains("not a league on this title"), "{err}");
    // An NHL division under the AHL is refused, and so is the AHL's own Atlantic under its
    // Western Conference (R1 F1: not a witnessed pair); East / Atlantic is written (4 cells).
    let err = ws.set_team_membership(bos, ahl, Some(13), Some(8)).unwrap_err().to_string();
    assert!(err.contains("not paired"), "{err}");
    let err = ws.set_team_membership(bos, ahl, Some(13), Some(15)).unwrap_err().to_string();
    assert!(err.contains("not paired with Western Conference under AHL"), "{err}");
    assert_eq!(ws.undo_depth(), 0);
    let (plan, outcome) = ws.set_team_membership(bos, ahl, Some(14), Some(15)).unwrap();
    assert_eq!(plan.writes.len(), 4);
    assert_eq!(outcome.edits, 4);
    assert_eq!(plan.after.division.as_ref().unwrap().label, "Atlantic Division");
    assert!(plan.notes.iter().any(|n| n.contains("this title's own table")), "{:?}", plan.notes);
    assert!(ws.undo());
}

/// NHL 07 (research 2026-09-21 handoff 13): the membership lives on `stockteamtable`; the thin
/// `exhibitionteams` selector points at the stock row. Boston reads league 0 / conference 1
/// (Eastern) / division 0 (Northeast) with the 2006-07 labels; Chicago is Western / Central; a
/// league move and a local-bit write go through and read back; the All-Star rows 30 / 31 refuse
/// a league change; the team view carries the labels; undo restores the file.
#[test]
fn nhl07_membership_edits_the_stock_row_with_labelled_local_bits() {
    let mut ws = open(
        "nhl07",
        Platform::Xbox,
        "anchors/xbox360/nhl07/databases/NHL07NG.db",
        "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml",
    );
    let bos = team_by_abbrev(&ws, "BOS");
    let opts = ws.team_membership_options(bos).unwrap();
    assert!(opts.blocker.is_none(), "{:?}", opts.blocker);
    assert_eq!(opts.model, GroupModel::LocalBits);
    assert_eq!(opts.current.league_code, 0);
    assert_eq!((opts.current.conference.as_ref().unwrap().value, opts.current.conference.as_ref().unwrap().label.as_str()), (1, "Eastern"));
    assert_eq!((opts.current.division.as_ref().unwrap().value, opts.current.division.as_ref().unwrap().label.as_str()), (0, "Northeast"));
    assert!(opts.notes.iter().any(|n| n.contains("NHL 07")), "{:?}", opts.notes);
    let chi = team_by_abbrev(&ws, "CHI");
    let c = ws.team_membership_options(chi).unwrap();
    assert_eq!((c.current.conference.as_ref().unwrap().label.as_str(), c.current.division.as_ref().unwrap().label.as_str()), ("Western", "Central"));
    let view = ws.display().team_view(bos).unwrap().unwrap();
    assert_eq!((view.conference_label.as_deref(), view.division_label.as_deref()), (Some("Eastern"), Some("Northeast")));
    // A national team carries 1 / 0 with no label under league 1: shown raw.
    let can = team_by_abbrev(&ws, "CAN");
    let n = ws.team_membership_options(can).unwrap();
    assert_eq!(n.current.league_code, 1);
    assert!(n.current.division.as_ref().unwrap().label.contains("untraced"), "{}", n.current.division.as_ref().unwrap().label);
    // Writes: a local-bit change on Boston (division 0 → 1) and a league move to the DEL; both read back.
    let (plan, outcome) = ws.set_team_membership(bos, 0, Some(1), Some(1)).unwrap();
    assert_eq!(plan.table, "HqzU", "the stock team table is the target");
    assert_eq!((plan.writes.len(), outcome.edits), (3, 3));
    let after = ws.team_membership_options(bos).unwrap();
    assert_eq!(after.current.division.as_ref().unwrap().label, "Atlantic");
    assert!(ws.set_team_membership(bos, 4, Some(0), Some(0)).is_err(), "A scheduled team needs a league swap");
    let del=ws.display().team_summaries().unwrap().into_iter().find(|t|t.league==Some(4)).unwrap().team_id;
    ws.swap_team_memberships(bos,del).unwrap();
    assert_eq!(ws.team_membership_options(bos).unwrap().current.league_code,4);
    // The All-Star rows (stock 30 / 31) refuse a league change.
    let eas = team_by_abbrev(&ws, "EAS");
    let e = match ws.set_team_membership(eas, 2, None, None) {
        Ok(_) => panic!("All-Star row must refuse a league change"),
        Err(e) => e.to_string(),
    };
    assert!(!e.is_empty());
    assert!(ws.undo() && ws.undo());
    assert_eq!(ws.team_membership_options(bos).unwrap().current.division.as_ref().unwrap().label, "Northeast");
    assert!(!ws.is_dirty());
}

/// D7 (P1 accounting): every local-bits anchor — Xbox 360 and PS3 NHL 08–11 — shows the local
/// conference/division numbers raw with the "untraced" label, offers no group table, takes a
/// league-code-only write and a write with local numbers, and refuses an out-of-range division.
#[test]
fn every_local_bits_anchor_shows_raw_numbers_and_takes_local_writes() {
    for platform in ["xbox360", "ps3"] {
        for game in ["nhl08", "nhl09", "nhl10", "nhl11"] {
            let p = if platform == "ps3" { Platform::Ps3 } else { Platform::Xbox };
            let mut ws = open(
                game,
                p,
                &format!("anchors/{platform}/{game}/databases/nhlng.db"),
                &format!("anchors/{platform}/{game}/databases/nhlng-meta.xml"),
            );
            let label = format!("{platform}/{game}");
            let bos = team_by_abbrev(&ws, "BOS");
            let opts = ws.team_membership_options(bos).unwrap();
            assert_eq!(opts.model, GroupModel::LocalBits, "{label}");
            assert!(opts.groups_by_league.is_empty(), "{label}");
            assert!(opts.blocker.is_none(), "{label}");
            let conf = opts.current.conference.as_ref().unwrap_or_else(|| panic!("{label}: conference"));
            // Receipt 44 (R133): NHL 10 / 11 conferences are inferred by position; 08 / 09 stay untraced numbers.
            let want = if matches!(game, "nhl10" | "nhl11") { "(inferred)" } else { "untraced" };
            assert!(conf.label.contains(want), "{label}: {}", conf.label);
            assert!(opts.notes.iter().any(|n| n.contains("not traced")), "{label}");
            let (plan, outcome) = ws.set_team_membership(bos, 1, None, None).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!((plan.writes.len(), outcome.edits), (1, 1), "{label}");
            let (plan, _) = ws.set_team_membership(bos, 0, Some(1), Some(2)).unwrap_or_else(|e| panic!("{label}: {e}"));
            assert_eq!(plan.writes.len(), 3, "{label}");
            assert!(ws.set_team_membership(bos, 0, Some(1), Some(9)).is_err(), "{label}: division is 3 bits");
            let after = ws.team_membership_options(bos).unwrap();
            assert_eq!(after.current.conference.as_ref().unwrap().value, 1, "{label}: written conference read back");
            assert_eq!(after.current.division.as_ref().unwrap().value, 2, "{label}: written division read back");
        }
    }
}
