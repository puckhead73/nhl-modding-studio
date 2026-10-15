//! Receipt 48 — single-phase grouped schedule generation (research R123, accepted 28 Sep 2026 for single-phase
//! partition-to-matrix generation). Groups + within / across become the pair-quota matrix after their own checks;
//! the built rows are checked independently (pair counts per group, home / away splits, stored indexes, kept rows,
//! dates); the existing preview / apply / undo / save contract carries the write. The shipped groups are offered
//! only by exact source identity (see `schedule_stock_r113.rs`).
//!
//! Measured structures: X360 NHL 11 `swissschedule` (3 groups of 4, within 6 / across 4, 300 games), PS4 NHL 17
//! `finnishschedule` (5 groups of 3, within 6 / across 4, 450 games) and — ignored, private fixture — the genuine
//! NHL 20 (CUSA15439) `swissschedule`. Every write goes to a temporary copy; anchors and fixtures are only read.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_domain::schedule_edit::{GameEdit, GenerationConfig, GroupQuotas, OddHome};
use studio_domain::schedules::ScheduleView;
use studio_domain::{EditSession, Platform, Project, ProjectRole, Workspace};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

struct Scratch(PathBuf);
impl Scratch {
    fn new(tag: &str) -> Self {
        let d = std::env::temp_dir().join(format!("nhlms schedule grouped {tag} {}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        Scratch(d)
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        if self.0.starts_with(std::env::temp_dir()) && self.0.file_name().is_some_and(|n| n.to_string_lossy().starts_with("nhlms schedule grouped")) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

fn open(db: &Path, meta: &Path, platform: Platform, game: &str) -> Workspace {
    let p = Project::load(db, platform, ProjectRole::Nhlng, meta, repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(p), db.to_string_lossy().into_owned());
    ws.set_game_id(game);
    ws
}

fn table(ws: &Workspace, name: &str) -> String {
    ws.display().schedule_tables().unwrap().into_iter().find(|t| t.name == name).unwrap_or_else(|| panic!("{name}")).studio_table
}

fn stock(ws: &Workspace, t: &str) -> GroupQuotas {
    let g = ws.schedule_stock_groups(t).unwrap().unwrap_or_else(|e| panic!("stock groups: {e}"));
    GroupQuotas { groups: g.groups, within: g.within, across: g.across, origin: Some(format!("shipped {}", g.source)) }
}

fn cfg(ws: &Workspace, t: &str, g: GroupQuotas) -> GenerationConfig {
    let o = ws.schedule_generation_options(t).unwrap();
    GenerationConfig {
        participants: g.groups.iter().flatten().copied().collect(),
        rounds: 0,
        start_month: o.suggested_start_month,
        start_day: o.suggested_start_day,
        days_between_matchdays: 1,
        pair_quotas: None,
        odd_home: None,
        groups: Some(g),
        odd_hosts: None,
    }
}

/// Pair counts and home games per ordered pair of team ids, read back from a table.
fn pair_stats(view: &ScheduleView) -> (BTreeMap<(u64, u64), u32>, BTreeMap<(u64, u64), u32>) {
    let (mut count, mut home) = (BTreeMap::new(), BTreeMap::new());
    for g in &view.games {
        let (h, a) = (view.slot_map.by_slot(g.home_slot).unwrap().team_id, view.slot_map.by_slot(g.away_slot).unwrap().team_id);
        *count.entry((h.min(a), h.max(a))).or_insert(0) += 1;
        *home.entry((h, a)).or_insert(0) += 1;
    }
    (count, home)
}

/// Independently of the generator: every pair plays `within` / `across` by group, splits evenly (even counts)
/// or gives the named host one more (odd counts), and every index stays where it was.
fn assert_groups(tag: &str, before: &ScheduleView, after: &ScheduleView, g: &GroupQuotas, host: &dyn Fn(u64, u64) -> u64) {
    assert_eq!(before.games.len(), after.games.len(), "{tag}: row count");
    for (b, a) in before.games.iter().zip(&after.games) {
        assert_eq!(a.index, b.index, "{tag}: stored index unchanged at row {}", b.record);
        assert_eq!(a.round, 0, "{tag}");
    }
    let group_of: BTreeMap<u64, usize> = g.groups.iter().enumerate().flat_map(|(k, x)| x.iter().map(move |t| (*t, k))).collect();
    let (count, home) = pair_stats(after);
    let teams: Vec<u64> = group_of.keys().copied().collect();
    for (x, &i) in teams.iter().enumerate() {
        for &j in &teams[x + 1..] {
            let want = if group_of[&i] == group_of[&j] { g.within } else { g.across };
            assert_eq!(count.get(&(i.min(j), i.max(j))).copied().unwrap_or(0), want, "{tag}: {i} v {j}");
            let (hi, hj) = (home.get(&(i, j)).copied().unwrap_or(0), home.get(&(j, i)).copied().unwrap_or(0));
            if want % 2 == 0 {
                assert_eq!(hi, hj, "{tag}: {i} v {j} even split");
            } else {
                let h = host(i, j);
                assert_eq!(if h == i { hi } else { hj }, if h == i { hj } else { hi } + 1, "{tag}: {i} v {j} extra game to {h}");
            }
        }
    }
    // No club twice on one date.
    let mut seen = std::collections::BTreeSet::new();
    for gm in &after.games {
        for s in [gm.home_slot, gm.away_slot] {
            assert!(seen.insert((s, gm.month, gm.day)), "{tag}: slot {s} twice on {}", gm.date);
        }
    }
}

/// Preview → apply → undo → redo → save copy → reopen equal; then a stale preview is refused.
fn round_trip(tag: &str, ws: &mut Workspace, t: &str, c: &GenerationConfig, out: &Path, reopen: impl Fn(&Path) -> Workspace) -> ScheduleView {
    let source = ws.primary().bytes().to_vec();
    let (ticket, plan) = ws.schedule_generate_preview(t, c).unwrap();
    assert!(plan.change.validation.ok(), "{tag}: {:?}", plan.change.validation.errors);
    assert!(plan.change.notes.iter().any(|n| n.starts_with("groups")), "{tag}: {:?}", plan.change.notes);
    assert_eq!(ws.primary().bytes(), &source[..], "{tag}: preview writes nothing");
    let (_, outcome, view) = ws.schedule_apply_review(&ticket, None).unwrap();
    assert!(outcome.edits > 0);
    assert_eq!((ws.undo_depth(), ws.redo_depth()), (1, 0), "{tag}: one reviewed apply");
    let applied = ws.primary().bytes().to_vec();
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &source[..], "{tag}: undo");
    assert!(ws.redo());
    assert_eq!(ws.primary().bytes(), &applied[..], "{tag}: redo");
    std::fs::create_dir_all(out.parent().unwrap()).unwrap();
    ws.save_primary_to(out.to_str().unwrap()).unwrap();
    let back = reopen(out).display().schedule_view(t).unwrap().unwrap();
    assert_eq!(
        back.games.iter().map(|g| (g.index, g.month, g.day, g.home_slot, g.away_slot)).collect::<Vec<_>>(),
        view.games.iter().map(|g| (g.index, g.month, g.day, g.home_slot, g.away_slot)).collect::<Vec<_>>(),
        "{tag}: the saved copy reopens with the same rows"
    );
    // Stale: preview, change one row, apply the old preview → refused, nothing written.
    let (stale, _) = ws.schedule_generate_preview(t, c).unwrap();
    let g0 = view.games[5].clone();
    let (edit, _) = ws.schedule_edit_preview(t, &GameEdit { record: g0.record, home_team_id: None, away_team_id: None, month: Some(g0.month), day: Some(g0.day + 1) }).unwrap();
    ws.schedule_apply_review(&edit, None).unwrap();
    let bytes = ws.primary().bytes().to_vec();
    let e = ws.schedule_apply_review(&stale, None).unwrap_err().to_string();
    assert!(e.contains("changed since this preview") || e.contains("no longer held"), "{tag}: {e}");
    assert_eq!(ws.primary().bytes(), &bytes[..], "{tag}: stale review writes nothing");
    view
}

#[test]
fn nhl11_swiss_shipped_groups_and_user_groups_with_per_pair_hosts() {
    let d = repo("anchors/xbox360/nhl11/databases");
    let reopen = |p: &Path| open(p, &d.join("nhlng-meta.xml"), Platform::Xbox, "nhl11");
    let mut ws = reopen(&d.join("nhlng.db"));
    let t = table(&ws, "swissschedule");
    let before = ws.display().schedule_view(&t).unwrap().unwrap();
    let scratch = Scratch::new("nhl11");

    // 1. The shipped groups (exact source identity) → 300 games, all even counts.
    let g = stock(&ws, &t);
    assert_eq!((g.groups.iter().map(Vec::len).collect::<Vec<_>>(), g.within, g.across), (vec![4, 4, 4], 6, 4));
    let c = cfg(&ws, &t, g.clone());
    let after = round_trip("nhl11 stock", &mut ws, &t, &c, &scratch.0.join("stock copy").join("nhlng.db"), reopen);
    assert_groups("nhl11 stock", &before, &after, &g, &|_, _| unreachable!("no odd pair"));
    while ws.undo() {}

    // 2. User groups with odd counts: within 14 / across 1 also totals 300 (3 × 6 × 14 + 48 × 1); the 48
    //    across pairs need a host — named per pair (the club listed first in the participants hosts).
    let mut user = g.clone();
    user.within = 14;
    user.across = 1;
    user.origin = None;
    let mut c = cfg(&ws, &t, user.clone());
    let order: Vec<u64> = c.participants.clone();
    let pos = |t: u64| order.iter().position(|x| *x == t).unwrap();
    let group_of: BTreeMap<u64, usize> = user.groups.iter().enumerate().flat_map(|(k, x)| x.iter().map(move |t| (*t, k))).collect();
    let mut hosts = Vec::new();
    for (x, &i) in order.iter().enumerate() {
        for &j in &order[x + 1..] {
            if group_of[&i] != group_of[&j] {
                hosts.push((i, j));
            }
        }
    }
    assert_eq!(hosts.len(), 48);
    // Without a rule, and with incomplete / foreign / duplicate / even-pair host lists: refused.
    let refuse = |ws: &mut Workspace, c: &GenerationConfig, want: &str| {
        let (_, p) = ws.schedule_generate_preview(&t, c).unwrap();
        assert!(!p.change.validation.ok() && p.change.validation.errors.iter().any(|e| e.contains(want)), "{want}: {:?}", p.change.validation.errors);
        assert!(p.change.rows.is_empty());
    };
    refuse(&mut ws, &c, "choose which club hosts the extra game");
    c.odd_home = Some(OddHome::Explicit);
    refuse(&mut ws, &c, "per-pair hosts were chosen but none were given");
    c.odd_hosts = Some(hosts[1..].to_vec());
    refuse(&mut ws, &c, "choose the host of the extra");
    let within_pair = order.iter().flat_map(|&i| order.iter().map(move |&j| (i, j))).find(|&(i, j)| i != j && group_of[&i] == group_of[&j]).unwrap();
    c.odd_hosts = Some([hosts.clone(), vec![within_pair]].concat());
    refuse(&mut ws, &c, "not a pair with an odd number of games");
    c.odd_hosts = Some([hosts.clone(), vec![hosts[0]]].concat());
    refuse(&mut ws, &c, "twice");
    let outsider = before.slot_map.entries.iter().map(|e| e.team_id).find(|t| !order.contains(t));
    if let Some(o) = outsider {
        c.odd_hosts = Some([hosts.clone(), vec![(o, order[0])]].concat());
        refuse(&mut ws, &c, "not a pair of participants");
    }
    c.odd_hosts = Some(hosts.clone());
    let after = round_trip("nhl11 per-pair hosts", &mut ws, &t, &c, &scratch.0.join("user copy").join("nhlng.db"), reopen);
    assert_groups("nhl11 per-pair hosts", &before, &after, &user, &|i, j| if pos(i) < pos(j) { i } else { j });
    while ws.undo() {}

    // 3. The same counts with the balance rule: deterministic, every chosen host reported, checked by the
    //    independent post-check.
    let b = GenerationConfig { odd_home: Some(OddHome::Balance), odd_hosts: None, ..c.clone() };
    let (_, p1) = ws.schedule_generate_preview(&t, &b).unwrap();
    let (_, p2) = ws.schedule_generate_preview(&t, &b).unwrap();
    assert!(p1.change.validation.ok(), "{:?}", p1.change.validation.errors);
    assert_eq!(p1.change.after, p2.change.after, "balance is deterministic");
    let hosts_note = p1.change.notes.iter().find(|n| n.starts_with("extra home game (48 odd pair(s))")).expect("all 48 hosts reported");
    assert!(!hosts_note.contains("more"), "no host elided: {hosts_note}");
}

#[test]
fn group_requests_are_refused_when_they_do_not_partition_the_participants() {
    let d = repo("anchors/xbox360/nhl11/databases");
    let mut ws = open(&d.join("nhlng.db"), &d.join("nhlng-meta.xml"), Platform::Xbox, "nhl11");
    let t = table(&ws, "swissschedule");
    let g = stock(&ws, &t);
    let base = cfg(&ws, &t, g.clone());
    let mut check = |c: GenerationConfig, want: &str| {
        let (_, p) = ws.schedule_generate_preview(&t, &c).unwrap();
        assert!(p.change.validation.errors.iter().any(|e| e.contains(want)), "{want}: {:?}", p.change.validation.errors);
        assert!(p.change.rows.is_empty());
    };
    let mut two = g.clone();
    two.groups[1].push(g.groups[0][0]);
    check(GenerationConfig { groups: Some(two), ..base.clone() }, "exactly one group");
    let mut missing = g.clone();
    missing.groups[2].pop();
    check(GenerationConfig { groups: Some(missing), ..base.clone() }, "is in no group");
    let mut empty = g.clone();
    empty.groups.push(Vec::new());
    check(GenerationConfig { groups: Some(empty), ..base.clone() }, "is empty");
    let mut outsider = g.clone();
    outsider.groups[0].push(999_999);
    check(GenerationConfig { groups: Some(outsider), ..base.clone() }, "not a participant");
    check(GenerationConfig { pair_quotas: Some(vec![vec![0; 12]; 12]), ..base.clone() }, "not both");
    // Totals must equal the active rows (Studio never changes a table's row count): within 6 / across 3.
    let mut short = g.clone();
    short.across = 3;
    check(GenerationConfig { groups: Some(short), odd_home: Some(OddHome::LowerSlot), ..base.clone() }, "does not change a schedule table's row count");
    // Per-pair hosts given with another rule are refused, not ignored.
    check(GenerationConfig { odd_home: Some(OddHome::LowerSlot), odd_hosts: Some(vec![(g.groups[0][0], g.groups[1][0])]), ..base.clone() }, "not \"per pair\"");
}

#[test]
fn ps4_nhl17_finnish_five_groups_of_three() {
    let d = repo("anchors/ps4/nhl17/databases");
    let reopen = |p: &Path| open(p, &d.join("nhl_game-meta.xml"), Platform::Ps4, "nhl17");
    let mut ws = reopen(&d.join("nhl_game.db"));
    let t = table(&ws, "finnishschedule");
    let before = ws.display().schedule_view(&t).unwrap().unwrap();
    let g = stock(&ws, &t);
    assert_eq!((g.groups.len(), g.within, g.across), (5, 6, 4));
    let scratch = Scratch::new("ps4-17");
    let c = cfg(&ws, &t, g.clone());
    let after = round_trip("ps4 17 finnish", &mut ws, &t, &c, &scratch.0.join("copy").join("nhl_game.db"), reopen);
    assert_groups("ps4 17 finnish", &before, &after, &g, &|_, _| unreachable!());
    assert!(after.games.iter().all(|g| g.status.is_none()), "PS4 16+: no status field written");
}

#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn genuine_nhl20_swiss_shipped_groups() {
    let root = PathBuf::from(std::env::var("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES (fails, never skips)")).join("databases/nhl20-ps4-cusa15439/inxp_00.big");
    assert!(root.join("nhl_game.db").is_file(), "{}: missing", root.display());
    // Work on a copy: the fixture is only read.
    let scratch = Scratch::new("nhl20");
    let src = scratch.0.join("source copy").join("nhl_game.db");
    std::fs::create_dir_all(src.parent().unwrap()).unwrap();
    std::fs::copy(root.join("nhl_game.db"), &src).unwrap();
    let reopen = |p: &Path| open(p, &root.join("nhl_game-meta.xml"), Platform::Ps4, "nhl20");
    let mut ws = reopen(&src);
    let t = table(&ws, "swissschedule");
    let before = ws.display().schedule_view(&t).unwrap().unwrap();
    let g = stock(&ws, &t);
    assert!(g.origin.as_deref().unwrap().contains("ps4-nhl20-real"), "{:?}", g.origin);
    let c = cfg(&ws, &t, g.clone());
    let after = round_trip("nhl20 swiss", &mut ws, &t, &c, &scratch.0.join("saved copy").join("nhl_game.db"), reopen);
    assert_groups("nhl20 swiss", &before, &after, &g, &|_, _| unreachable!());
    // The Austrian two-phase table stays refused (tagged final block; R123 Q10).
    let aut = table(&ws, "austrianschedule");
    assert!(ws.schedule_stock_groups(&aut).unwrap().is_err());
    eprintln!("genuine NHL 20 swiss: {} groups, {} rows", g.groups.len(), after.games.len());
}
