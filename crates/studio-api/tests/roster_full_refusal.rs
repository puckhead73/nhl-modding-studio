//! Players 3.2 (25 Sep 2026, package P3.2-D) — moving or adding a player into a team whose roster
//! block has **no free slot** refuses, in words, on every anchor family — the case the operation
//! matrix could not run on 16 anchors (packed gen7 rosters: no vacant slot to move *into*, so its
//! move / add-membership rows read "no-fixture").
//!
//! Per anchor: a rostered skater is moved into other teams in turn; a team with a free slot takes
//! him (undone at once), the first team without one refuses. The refusal must name the **team**
//! (never the raw team id alone), say the roster is full, leave the player where he was and leave
//! the undo history untouched. Where no team is full (PS4 anchors carry slack) that is recorded,
//! not asserted away. Nothing under `anchors/` is written.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_api::{parse_platform, parse_role, Registry};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

fn anchors() -> Vec<(String, String, PathBuf, PathBuf)> {
    let root = repo("anchors");
    let pairs = [
        ("NHL07NG.db", "NHL07NG-meta.xml"),
        ("nhlng.db", "nhlng-meta.xml"),
        ("nhl_game.db", "nhl_game-meta.xml"),
    ];
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms.iter().filter(|e| e.path().is_dir()) {
        let mut games: Vec<_> = std::fs::read_dir(platform.path())
            .unwrap()
            .flatten()
            .collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs
                .iter()
                .find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file())
            {
                out.push((
                    platform.file_name().to_string_lossy().into_owned(),
                    game.file_name().to_string_lossy().into_owned(),
                    dbs.join(db),
                    dbs.join(meta),
                ));
            }
        }
    }
    assert_eq!(out.len(), 27);
    out
}

fn open(reg: &Registry, platform: &str, db: &Path, meta: &Path) -> u32 {
    reg.load_project(
        &s(db),
        parse_platform(platform).unwrap(),
        parse_role("nhlng").unwrap(),
        &s(meta),
        &s(&repo("defs")),
    )
    .unwrap_or_else(|e| panic!("{}: load: {e}", db.display()))
    .project_id
}

#[test]
fn moving_into_a_full_roster_block_refuses_in_words_and_changes_nothing() {
    let mut table: BTreeMap<String, String> = BTreeMap::new();
    let mut refusals = 0;
    for (platform, game, db, meta) in anchors() {
        let label = format!("{platform}/{game}");
        let reg = Registry::default();
        let id = open(&reg, &platform, &db, &meta);
        let teams = reg.list_teams(id).unwrap();
        let players = reg.list_players(id).unwrap();
        let subject = players
            .iter()
            .find(|p| p.team_id.is_some() && p.position_label != "G")
            .unwrap_or_else(|| panic!("{label}: a rostered skater"));
        let from = subject.team_id.unwrap();
        let mut outcome = None;
        let mut accepted = 0;
        for t in teams.iter().filter(|t| t.team_id != from).take(40) {
            match reg.move_player(id, subject.player_id, from, t.team_id) {
                Ok(_) => {
                    // A free slot took him: put him back and keep looking for a full block.
                    assert!(reg.undo(id).unwrap(), "{label}: undo the accepted move");
                    accepted += 1;
                }
                Err(e) => {
                    let msg = e.to_string();
                    let shown = if t.name.trim().is_empty() {
                        t.abbrev.trim().to_string()
                    } else {
                        t.name.clone()
                    };
                    assert!(
                        !shown.is_empty(),
                        "{label}: the team has a name or an abbreviation"
                    );
                    assert!(
                        msg.starts_with(&shown) && msg.contains("no free roster slot"),
                        "{label}: the refusal starts with the team ({shown:?}) and gives the reason: {msg}"
                    );
                    assert!(
                        !msg.contains(&format!("team {} ", t.team_id)),
                        "{label}: the raw team id is not the primary text: {msg}"
                    );
                    // Nothing moved; nothing to undo.
                    let now = reg
                        .list_players(id)
                        .unwrap()
                        .into_iter()
                        .find(|p| p.player_id == subject.player_id)
                        .unwrap();
                    assert_eq!(
                        now.team_id,
                        Some(from),
                        "{label}: the player stays where he was"
                    );
                    assert!(
                        !reg.undo(id).unwrap(),
                        "{label}: a refused move leaves no undo entry"
                    );
                    // The same block refuses an added membership too.
                    let e2 = reg
                        .add_membership(id, subject.player_id, t.team_id)
                        .err()
                        .unwrap_or_else(|| {
                            panic!("{label}: add-membership into the full block must refuse")
                        });
                    assert!(e2.to_string().starts_with(&shown), "{label}: {e2}");
                    outcome = Some(format!(
                        "refused in words at {shown} after {accepted} team(s) with a free slot — \"{}\"",
                        msg.lines().next().unwrap_or_default()
                    ));
                    refusals += 1;
                    break;
                }
            }
        }
        reg.close_project(id);
        table.insert(
            label,
            outcome.unwrap_or_else(|| format!("no full roster block among {accepted} teams tried (slack roster) — moves accepted and undone")),
        );
    }
    eprintln!("\n| anchor | move into a full roster block |");
    eprintln!("|---|---|");
    for (l, r) in &table {
        eprintln!("| {l} | {r} |");
    }
    assert_eq!(table.len(), 27);
    assert!(
        refusals >= 16,
        "the packed gen7 anchors refuse ({refusals} refusals)"
    );
}
