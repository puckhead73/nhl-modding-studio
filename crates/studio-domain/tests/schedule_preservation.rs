//! Receipt 41 — schedule generation keeps retained rows in place (research R123 R2; dispatch after
//! receipt 40 §2). A generation replaces only the rows between two selected participants; a row
//! involving anyone else (an All-Star game, a game against a club left out) keeps its position and
//! every stored value (index, date, clubs, round, status). Generated games fill the other positions
//! in date order and keep each position's index. Kept games of a selected club occupy that club's
//! date: a generated game never lands on it, and when nothing else fits the plan is refused with
//! nothing written. Before this receipt both generators merged the kept rows by date and re-indexed
//! every row (`merge_by_date`), moving the retained row and overwriting its index.
//!
//! Fixtures are **synthetic**: built in memory from the committed NHL 07 Xbox 360 anchor's 436-row
//! `nhlschedulesmall` table (30 clubs × 29 games + the All-Star row), written through the ordinary
//! edit path, saved to a temporary copy under a path with spaces and reopened as the source. The
//! anchor itself is never written. The temporary directory is removed by the guard.
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use studio_domain::schedule_edit::{season_key, GameEdit, GenerationConfig, GenerationPlan, OddHome};
use studio_domain::schedules::{ScheduleView, ScheduledGame};
use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

const META: &str = "anchors/xbox360/nhl07/databases/NHL07NG-meta.xml";

fn open(db: &Path) -> Workspace {
    let project = Project::load(db, Platform::Xbox, ProjectRole::Nhlng, repo(META), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.to_string_lossy().into_owned());
    ws.set_game_id("nhl07");
    ws
}

/// Removes its directory on drop (only a directory this test created under the temp root).
struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms schedule preservation {tag} {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Scratch(d)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if self.0.starts_with(std::env::temp_dir()) && self.0.file_name().is_some_and(|n| n.to_string_lossy().starts_with("nhlms schedule preservation")) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

fn small_table(ws: &Workspace) -> String {
    ws.display().schedule_tables().unwrap().into_iter().find(|t| t.name == "nhlschedulesmall").unwrap().studio_table
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct Row {
    index: u64,
    month: u64,
    day: u64,
    home: u64,
    away: u64,
    round: u64,
    status: Option<u64>,
}
fn row(g: &ScheduledGame) -> Row {
    Row { index: g.index, month: g.month, day: g.day, home: g.home_slot, away: g.away_slot, round: g.round, status: g.status }
}

/// Write `rows` over the table (one transaction) and save a copy to `out`.
fn write_rows(ws: &mut Workspace, table: &str, view: &ScheduleView, rows: &[Row], out: &Path) {
    let f = |n: &str| view.layout.field(n).map(|f| f.studio_field.clone());
    let mut writes = Vec::new();
    for (i, (cur, new)) in view.games.iter().map(row).zip(rows).enumerate() {
        let mut w = Vec::new();
        for (name, a, b) in [("month", cur.month, new.month), ("day", cur.day, new.day), ("home", cur.home, new.home), ("away", cur.away, new.away), ("round", cur.round, new.round)] {
            if a != b {
                w.push((f(name).unwrap_or_else(|| panic!("{name} field")), b as i64));
            }
        }
        if cur.status != new.status {
            w.push((f("status").unwrap(), new.status.unwrap() as i64));
        }
        if !w.is_empty() {
            writes.push((i as u32, w));
        }
    }
    ws.set_cells_int_rows(table, &writes).unwrap();
    ws.save_primary_to(out.to_str().unwrap()).unwrap();
}

/// Synthetic source A: the All-Star row moved from its stock position to row 60, dated like its
/// new neighbours (the table stays in season order), with round 1 and status 2 — values
/// a generation must not touch. Every other row keeps the stock game, indexed by position.
/// `conflict_at`: additionally turn that row into "a selected club v the All-Star squad" dated
/// `Oct 1` (the generation start) — a kept game of a participant. Returns (source path, All-Star
/// row, conflict row, the participant it involves, the pair whose stock game it replaced).
struct Fixture {
    db: PathBuf,
    star_at: usize,
    star: Row,
    conflict: Option<(usize, Row, u64, (u64, u64))>,
}

fn synthetic(scratch: &Scratch, name: &str, conflict_at: Option<usize>) -> Fixture {
    let mut ws = open(&repo("anchors/xbox360/nhl07/databases/NHL07NG.db"));
    let t = small_table(&ws);
    let view = ws.display().schedule_view(&t).unwrap().unwrap();
    assert_eq!(view.games.len(), 436);
    assert!(view.layout.field("index").is_some() && view.layout.field("round").is_some() && view.layout.field("status").is_some(), "NHL 07 X360 layout carries index / round / status");
    let mut rows: Vec<Row> = view.games.iter().map(row).collect();
    eprintln!("stock small table: row 0 {}, row 59 {}, row 200 {}, row 435 {}; All-Star at row {}", view.games[0].date, view.games[59].date, view.games[200].date, view.games[435].date, rows.iter().position(|r| r.home >= 30).unwrap());
    let s = rows.iter().position(|r| r.home >= 30).unwrap();
    let mut star = rows.remove(s);
    let star_at = 60;
    assert!(s != star_at, "the stock All-Star row is not already at row {star_at}");
    star.month = rows[star_at - 1].month;
    star.day = rows[star_at - 1].day;
    star.round = 1;
    star.status = Some(2);
    rows.insert(star_at, star);
    // The index field is the table's key (not writable through an edit): every row keeps its
    // positional index, the retained row included.
    for (i, r) in rows.iter_mut().enumerate() {
        r.index = i as u64;
    }
    let conflict = conflict_at.map(|at| {
        let replaced = (rows[at].home, rows[at].away);
        let p = rows[at].home;
        rows[at] = Row { index: rows[at].index, month: 9, day: 0, home: p, away: rows[star_at].home, round: 0, status: Some(0) };
        // Keep the table in season order around it: everything before it moves to Oct 1 too.
        for r in rows[..at].iter_mut() {
            (r.month, r.day) = (9, 0);
        }
        (at, rows[at].clone(), p, replaced)
    });
    let star = rows[star_at].clone();
    let db = scratch.0.join(name).join("NHL07NG.db");
    std::fs::create_dir_all(db.parent().unwrap()).unwrap();
    write_rows(&mut ws, &t, &view, &rows, &db);
    Fixture { db, star_at, star, conflict }
}

fn team_of(view: &ScheduleView, slot: u64) -> u64 {
    view.slot_map.by_slot(slot).unwrap().team_id
}

/// Collisions in a table: (date, club) pairs that occur more than once, split into those between
/// two kept rows (pre-existing) and those involving a generated row (introduced).
fn collisions(rows: &[Row], kept: &BTreeSet<usize>) -> (usize, usize) {
    let mut by: BTreeMap<(u64, u64, u64), Vec<usize>> = BTreeMap::new();
    for (i, r) in rows.iter().enumerate() {
        for s in [r.home, r.away] {
            by.entry((r.month, r.day, s)).or_default().push(i);
        }
    }
    let (mut pre, mut new) = (0, 0);
    for v in by.values().filter(|v| v.len() > 1) {
        if v.iter().all(|i| kept.contains(i)) {
            pre += 1
        } else {
            new += 1
        }
    }
    (pre, new)
}

/// Every property the preservation rule promises, checked on a table read back from the bytes.
fn assert_preserved(tag: &str, before: &ScheduleView, after: &ScheduleView, chosen: &BTreeSet<u64>) {
    assert_eq!(before.games.len(), after.games.len(), "{tag}: row count");
    let mut kept = BTreeSet::new();
    for (i, (b, a)) in before.games.iter().zip(&after.games).enumerate() {
        if !(chosen.contains(&b.home_slot) && chosen.contains(&b.away_slot)) {
            assert_eq!(row(a), row(b), "{tag}: kept row {i} unchanged in place");
            kept.insert(i);
        } else {
            assert_eq!(a.index, b.index, "{tag}: generated row {i} keeps its position's index");
            assert!(chosen.contains(&a.home_slot) && chosen.contains(&a.away_slot), "{tag}: row {i} is a generated game");
            assert_eq!((a.round, a.status), (0, Some(0)), "{tag}: generated row {i}");
        }
    }
    for w in after.games.windows(2) {
        assert!(season_key(w[0].month, w[0].day) <= season_key(w[1].month, w[1].day), "{tag}: season order at row {} ({} → {})", w[1].record, w[0].date, w[1].date);
    }
    let rows: Vec<Row> = after.games.iter().map(row).collect();
    assert_eq!(collisions(&rows, &kept).1, 0, "{tag}: no club plays twice on a date because of a generated game");
}

fn chosen_slots(view: &ScheduleView, participants: &[u64]) -> BTreeSet<u64> {
    view.slot_map.entries.iter().filter(|e| participants.contains(&e.team_id)).map(|e| e.slot).collect()
}

/// Preview → apply (one reviewed transaction) → undo (source bytes) → redo (applied bytes) → save a
/// copy → reopen → identical rows. Returns the applied view.
fn apply_round_trip(tag: &str, ws: &mut Workspace, t: &str, cfg: &GenerationConfig, out: &Path) -> (GenerationPlan, ScheduleView) {
    let source = ws.primary().bytes().to_vec();
    let before = ws.display().schedule_view(t).unwrap().unwrap();
    let (ticket, plan) = ws.schedule_generate_preview(t, cfg).unwrap();
    assert!(plan.change.validation.ok(), "{tag}: {:?}", plan.change.validation.errors);
    assert_eq!(ws.primary().bytes(), &source[..], "{tag}: preview writes nothing");
    let (_, outcome, view) = ws.schedule_apply_review(&ticket, None).unwrap();
    assert!(outcome.edits > 0);
    assert_eq!((ws.undo_depth(), ws.redo_depth()), (1, 0), "{tag}: one reviewed apply = one undo entry");
    assert_preserved(tag, &before, &view, &chosen_slots(&before, &cfg.participants));
    let applied = ws.primary().bytes().to_vec();
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &source[..], "{tag}: undo restores the source bytes");
    assert!(ws.redo());
    assert_eq!(ws.primary().bytes(), &applied[..], "{tag}: redo restores the applied bytes");
    ws.save_primary_to(out.to_str().unwrap()).unwrap();
    let reopened = open(out);
    let v2 = reopened.display().schedule_view(t).unwrap().unwrap();
    assert_eq!(v2.games.iter().map(row).collect::<Vec<_>>(), view.games.iter().map(row).collect::<Vec<_>>(), "{tag}: saved copy reopens with the same rows");
    (plan, view)
}

#[test]
fn both_generators_keep_the_interleaved_retained_row_in_place_with_every_value() {
    let scratch = Scratch::new("a");
    let fx = synthetic(&scratch, "source a", None);
    let mut ws = open(&fx.db);
    let t = small_table(&ws);
    let before = ws.display().schedule_view(&t).unwrap().unwrap();
    assert_eq!(row(&before.games[fx.star_at]), fx.star, "fixture: the retained row as written");
    let clubs: Vec<u64> = before.slot_map.entries.iter().filter(|e| e.slot < 30).map(|e| e.team_id).collect();
    assert_eq!(clubs.len(), 30);

    // Round-robin: one cycle of 30 clubs from Oct 1 every 4 days. Fifteen games per matchday, so
    // four matchdays (Oct 1 → Oct 13) fill rows 0–59; the stock dates put the retained row later
    // (row 59's date), and the old by-date merge moved it down past the Oct 17 matchday.
    let rr = GenerationConfig { participants: clubs.clone(), rounds: 1, start_month: 9, start_day: 0, days_between_matchdays: 4, pair_quotas: None, odd_home: None, groups: None, odd_hosts: None };
    let out = scratch.0.join("saved rr").join("NHL07NG.db");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let (plan, view) = apply_round_trip("round-robin", &mut ws, &t, &rr, &out);
    assert_eq!((plan.generated_rows, plan.kept_rows), (435, 1));
    assert_eq!(row(&view.games[fx.star_at]), fx.star, "round-robin: the All-Star row stays at row 60 with its date, clubs, index, round 1 and status 2");
    assert!(plan.change.rows.iter().all(|(r, _)| *r as usize != fx.star_at), "round-robin: no write touches the retained row");
    assert!(plan.change.notes.iter().any(|n| n.contains("keep their position")), "{:?}", plan.change.notes);
    eprintln!("round-robin: retained row 60 kept (round 1, status 2); {} cells; notes: {:?}", plan.change.cells, plan.change.notes);

    // Stale review: preview, then an unrelated edit — the review no longer applies, nothing moves.
    assert!(ws.undo());
    let (ticket, _) = ws.schedule_generate_preview(&t, &rr).unwrap();
    let g = before.games[200].clone();
    let (edit, _) = ws.schedule_edit_preview(&t, &GameEdit { record: 200, home_team_id: None, away_team_id: None, month: Some(g.month), day: Some(g.day + 1) }).unwrap();
    ws.schedule_apply_review(&edit, None).unwrap();
    let bytes = ws.primary().bytes().to_vec();
    let err = ws.schedule_apply_review(&ticket, None).unwrap_err().to_string();
    assert!(err.contains("changed since this preview") || err.contains("no longer held"), "{err}");
    assert_eq!(ws.primary().bytes(), &bytes[..], "stale review: nothing written");
    assert!(ws.undo());
    assert_eq!(ws.undo_depth(), 0);

    // Pair quotas: every pair once (the same 435 games) with the extra-home rule "balance".
    let n = clubs.len();
    let ones: Vec<Vec<u32>> = (0..n).map(|i| (0..n).map(|j| u32::from(i != j)).collect()).collect();
    let q = GenerationConfig { pair_quotas: Some(ones), odd_home: Some(OddHome::Balance), ..rr.clone() };
    let out = scratch.0.join("saved quota").join("NHL07NG.db");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let (plan, view) = apply_round_trip("quota", &mut ws, &t, &q, &out);
    assert_eq!(row(&view.games[fx.star_at]), fx.star, "quota: the All-Star row stays at row 60 with every value");
    assert!(plan.change.rows.iter().all(|(r, _)| *r as usize != fx.star_at));
    eprintln!("quota: retained row 60 kept; {} cells", plan.change.cells);
}

#[test]
fn kept_games_of_a_selected_club_occupy_its_date_or_the_plan_is_refused() {
    let scratch = Scratch::new("b");
    // (1) Satisfiable: row 0 = "club P v the All-Star squad" on Oct 1, the generation start. The
    //     first generated matchday would put P on Oct 1 as well; it moves to the next free date.
    let fx = synthetic(&scratch, "source b", Some(0));
    let (at, conflict, p, (h, a)) = fx.conflict.clone().unwrap();
    let mut ws = open(&fx.db);
    let t = small_table(&ws);
    let before = ws.display().schedule_view(&t).unwrap().unwrap();
    assert_eq!(row(&before.games[at]), conflict);
    let mut clubs: Vec<u64> = before.slot_map.entries.iter().filter(|e| e.slot < 30).map(|e| e.team_id).collect();
    // P first: its pairs lead the meeting queue, so it plays on the first generated matchday.
    let pt = team_of(&before, p);
    clubs.retain(|c| *c != pt);
    clubs.insert(0, pt);
    let n = clubs.len();
    let idx = |slot: u64| clubs.iter().position(|c| *c == team_of(&before, slot)).unwrap();
    let (hi, ai) = (idx(h), idx(a));
    let quotas: Vec<Vec<u32>> = (0..n).map(|i| (0..n).map(|j| u32::from(i != j && !((i, j) == (hi, ai) || (i, j) == (ai, hi)))).collect()).collect();
    let cfg = GenerationConfig { participants: clubs.clone(), rounds: 1, start_month: 9, start_day: 0, days_between_matchdays: 2, pair_quotas: Some(quotas.clone()), odd_home: Some(OddHome::Balance), groups: None, odd_hosts: None };
    let out = scratch.0.join("saved b").join("NHL07NG.db");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    let (plan, view) = apply_round_trip("occupied date", &mut ws, &t, &cfg, &out);
    assert_eq!((plan.generated_rows, plan.kept_rows), (434, 2));
    assert_eq!(row(&view.games[at]), conflict, "the kept game of P stays at row 0");
    assert_eq!(row(&view.games[fx.star_at]), fx.star);
    let p_on_oct1 = view.games.iter().filter(|g| (g.home_slot == p || g.away_slot == p) && (g.month, g.day) == (9, 0)).count();
    assert_eq!(p_on_oct1, 1, "P plays only its kept game on Oct 1");
    assert!(plan.change.validation.warnings.iter().all(|w| !w.contains("not created by the generator")), "{:?}", plan.change.validation.warnings);
    assert!(plan.change.notes.iter().any(|n| n.contains("already plays a kept game")), "the move is disclosed: {:?}", plan.change.notes);
    eprintln!("occupied date: P's generated games avoid Oct 1; first generated date {:?}", view.games.iter().find(|g| g.record != at as u32).map(|g| g.date.clone()));

    // (2) Unavoidable: the kept game sits at row 1, so row 0's generated game (P's first meeting)
    //     must be dated no later than Oct 1 — the only such date, where P already plays. Refused,
    //     nothing written.
    let fx = synthetic(&scratch, "source c", Some(1));
    let (_, _, p2, (h1, a1)) = fx.conflict.clone().unwrap();
    let mut ws = open(&fx.db);
    let before_bytes = ws.primary().bytes().to_vec();
    let v = ws.display().schedule_view(&t).unwrap().unwrap();
    let mut clubs: Vec<u64> = v.slot_map.entries.iter().filter(|e| e.slot < 30).map(|e| e.team_id).collect();
    let pt = team_of(&v, p2);
    clubs.retain(|c| *c != pt);
    clubs.insert(0, pt);
    let idx = |slot: u64| clubs.iter().position(|c| *c == team_of(&v, slot)).unwrap();
    let (hi, ai) = (idx(h1), idx(a1));
    let quotas: Vec<Vec<u32>> = (0..n).map(|i| (0..n).map(|j| u32::from(i != j && !((i, j) == (hi, ai) || (i, j) == (ai, hi)))).collect()).collect();
    let cfg = GenerationConfig { participants: clubs.clone(), pair_quotas: Some(quotas), ..cfg };
    let (ticket, plan) = ws.schedule_generate_preview(&t, &cfg).unwrap();
    let errs = plan.change.validation.errors.join(" | ");
    assert!(!plan.change.validation.ok() && errs.contains("already plays"), "refused, naming the kept game: {errs}");
    assert!(ws.schedule_apply_review(&ticket, None).is_err());
    assert_eq!(ws.primary().bytes(), &before_bytes[..], "refusal writes nothing");
    assert_eq!(ws.undo_depth(), 0);
    eprintln!("unavoidable: {errs}");
}

#[test]
fn a_club_left_out_keeps_all_its_games_and_they_block_its_opponents_dates() {
    // Real stock table: 29 of the 30 clubs selected, one round-robin (406 games); the 29 games of
    // the club left out and the All-Star row are kept (30 rows) — each of those 29 involves a
    // selected club. Either the plan fits every kept row in place with no generated game on an
    // opponent's kept date, or it is refused before any write.
    let mut ws = open(&repo("anchors/xbox360/nhl07/databases/NHL07NG.db"));
    let t = small_table(&ws);
    let before = ws.display().schedule_view(&t).unwrap().unwrap();
    let clubs: Vec<u64> = before.slot_map.entries.iter().filter(|e| e.slot < 30).map(|e| e.team_id).collect();
    let participants: Vec<u64> = clubs[..29].to_vec();
    let bytes = ws.primary().bytes().to_vec();
    let mut fits = 0;
    for (start_month, start_day, spacing) in [(9u64, 4u64, 2u32), (9, 4, 4), (0, 1, 1), (0, 1, 2), (0, 1, 3)] {
        let cfg = GenerationConfig { participants: participants.clone(), rounds: 1, start_month, start_day, days_between_matchdays: spacing, pair_quotas: None, odd_home: None, groups: None, odd_hosts: None };
        let plan = ws.schedule_generate_preview(&t, &cfg).unwrap().1;
        assert_eq!(plan.kept_rows, 30);
        if plan.change.validation.ok() {
            let (ticket, _) = ws.schedule_generate_preview(&t, &cfg).unwrap();
            let (_, _, view) = ws.schedule_apply_review(&ticket, None).unwrap();
            assert_preserved(&format!("left out, every {spacing}"), &before, &view, &chosen_slots(&before, &participants));
            assert!(ws.undo());
            fits += 1;
            eprintln!("left out, from {start_month}/{start_day} every {spacing} day(s): fits; notes {:?}", plan.change.notes);
        } else {
            assert_eq!(ws.primary().bytes(), &bytes[..]);
            eprintln!("left out, from {start_month}/{start_day} every {spacing} day(s): refused — {}", plan.change.validation.errors.join(" | "));
        }
    }
    assert_eq!(ws.primary().bytes(), &bytes[..]);
}

/// Card V-76's recipe (corrected): the Legacy Swiss table holds 300 rows and no kept row; its twelve
/// clubs at four games per pair make 66 × 4 = 264, 36 short. Three groups of four by league slot,
/// six games within a group (18 pairs × 2 more = 36) and four across → exactly 300. Checked on the
/// committed X360 Legacy anchor; the group members are printed for the card.
#[test]
fn v76_recipe_three_groups_of_four_totals_300_on_legacy_swiss() {
    let db = repo("anchors/xbox360/nhllegacy/databases/nhlng.db");
    let project = Project::load(&db, Platform::Xbox, ProjectRole::Nhlng, repo("anchors/xbox360/nhllegacy/databases/nhlng-meta.xml"), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.to_string_lossy().into_owned());
    ws.set_game_id("nhllegacy");
    let t = ws.display().schedule_tables().unwrap().into_iter().find(|t| t.name == "swissschedule").unwrap().studio_table;
    let opts = ws.schedule_generation_options(&t).unwrap();
    let members: Vec<_> = opts.members.iter().filter(|m| m.default).collect();
    assert_eq!((members.len(), opts.rows_to_fill, opts.kept_rows), (12, 300, 0));
    assert_eq!(66 * 4, 264, "four per pair is 36 short");
    let n = members.len();
    let group = |i: usize| i / 4;
    let q: Vec<Vec<u32>> = (0..n).map(|i| (0..n).map(|j| if i == j { 0 } else if group(i) == group(j) { 6 } else { 4 }).collect()).collect();
    let total: u32 = (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).map(|(i, j)| q[i][j]).sum();
    assert_eq!(total, 300);
    let four_pairs_to_six: u32 = 264 + 4 * 2;
    assert_eq!(four_pairs_to_six, 272, "the old card's recipe (four pairs raised to six) totals 272, not 300");
    let cfg = GenerationConfig { participants: members.iter().map(|m| m.team_id).collect(), rounds: 1, start_month: opts.suggested_start_month, start_day: opts.suggested_start_day, days_between_matchdays: 2, pair_quotas: Some(q), odd_home: None, groups: None, odd_hosts: None };
    let bytes = ws.primary().bytes().to_vec();
    let plan = ws.schedule_generate_preview(&t, &cfg).unwrap().1;
    assert!(plan.change.validation.ok(), "{:?}", plan.change.validation.errors);
    assert_eq!(plan.generated_rows, 300);
    assert_eq!(ws.primary().bytes(), &bytes[..]);
    for g in 0..3 {
        eprintln!("group {}: {}", g + 1, members[g * 4..g * 4 + 4].iter().map(|m| format!("{} (slot {})", m.name, m.slot)).collect::<Vec<_>>().join(", "));
    }
    eprintln!("start {} every 2 days: {} matchdays; 66 pairs × 4 = 264, + 18 within-group pairs × 2 = 300", date_of(opts.suggested_start_month, opts.suggested_start_day), plan.matchdays);
}

fn date_of(m: u64, d: u64) -> String {
    studio_domain::schedule_edit::date_label(m, d)
}
