//! Teams 2.2 (overnight 2026-09-24, package T1) — the team's **relationship** edits as named
//! parity cases on every committed main-database anchor (27: 10 Xbox 360, 10 PS3, 7 PS4):
//!
//! * **home arena** — the disclosure shown before the write (`arena_assignment_preview`: the exact
//!   cell, who stays at the arena the team leaves or that it becomes vacant, who already plays at
//!   the arena it joins; refusals for the same arena, an uncatalogued id and a title without the
//!   link), then the ordinary validated `set_field` on that cell, the arena list's tenant counts
//!   moving, no new integrity findings, **save → reopen** on a disposable copy, and one undo;
//! * **retired numbers** — retire, save → reopen, un-retire / undo; refused where `wcSL` is absent;
//! * **draft picks** — trade to another team, save → reopen, undo; skipped where `qHav` is absent;
//! * **strategies** — one tactic cell written in range, save → reopen, undo.
//!
//! Every anchor is exercised in memory and saved to a temp file; nothing under `anchors/` is
//! written. The per-family table the test prints is the coverage record; a family where a surface
//! is absent is asserted absent (not silently skipped), and nothing here is game evidence.
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use studio_api::{parse_platform, parse_role, Registry};
use studio_domain::FieldValue;

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_string_lossy().into_owned()
}

struct Tmp(PathBuf);
impl Tmp {
    fn new(tag: &str) -> Self {
        let dir = std::env::temp_dir().join(format!("nhlms-teamrel-{tag}-{}", std::process::id()));
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

/// The 27 main-DB anchors (platform dir, game dir, db, meta), sorted — the read-matrix discovery.
fn anchors() -> Vec<(String, String, PathBuf, PathBuf)> {
    let root = repo("anchors");
    let pairs = [("NHL07NG.db", "NHL07NG-meta.xml"), ("nhlng.db", "nhlng-meta.xml"), ("nhl_game.db", "nhl_game-meta.xml")];
    let mut out = Vec::new();
    let mut platforms: Vec<_> = std::fs::read_dir(&root).unwrap().flatten().collect();
    platforms.sort_by_key(|e| e.file_name());
    for platform in platforms.iter().filter(|e| e.path().is_dir()) {
        let mut games: Vec<_> = std::fs::read_dir(platform.path()).unwrap().flatten().collect();
        games.sort_by_key(|e| e.file_name());
        for game in games.iter().filter(|e| e.path().is_dir()) {
            let dbs = game.path().join("databases");
            if let Some((db, meta)) = pairs.iter().find(|(d, m)| dbs.join(d).is_file() && dbs.join(m).is_file()) {
                out.push((platform.file_name().to_string_lossy().into_owned(), game.file_name().to_string_lossy().into_owned(), dbs.join(db), dbs.join(meta)));
            }
        }
    }
    assert_eq!(out.len(), 27, "27 main-database anchors are committed");
    out
}

fn open(reg: &Registry, platform: &str, db: &Path, meta: &Path) -> u32 {
    reg.load_project(&s(db), parse_platform(platform).unwrap(), parse_role("nhlng").unwrap(), &s(meta), &s(&repo("defs")))
        .unwrap_or_else(|e| panic!("{}: load: {e}", db.display()))
        .project_id
}

/// Save project `id` to a fresh file and open that file as a new project.
fn save_and_reopen(reg: &Registry, id: u32, platform: &str, meta: &Path, out: &Path) -> u32 {
    let _ = std::fs::remove_file(out);
    reg.save(id, &s(out)).unwrap_or_else(|e| panic!("save {}: {e}", out.display()));
    open(reg, platform, out, meta)
}

#[derive(Default, Debug)]
struct Row {
    arena: String,
    retired: String,
    draft: String,
    strategy: String,
}

#[test]
fn team_relationship_edits_disclose_save_reopen_and_undo_on_every_anchor() {
    let tmp = Tmp::new("sweep");
    let mut table: BTreeMap<String, Row> = BTreeMap::new();
    let mut arena_families_with_link = 0;
    let mut arena_families_without = 0;
    for (platform, game, db, meta) in anchors() {
        let label = format!("{platform}/{game}");
        let reg = Registry::default();
        let id = open(&reg, &platform, &db, &meta);
        let mut row = Row::default();
        let teams = reg.list_teams(id).unwrap();
        assert!(teams.len() >= 2, "{label}: at least two teams");
        let before_report = reg.validate(id).unwrap();

        // ------------------------------------------------------------------ home arena
        let arenas = reg.list_arenas(id).unwrap();
        // The first team whose arena cell exists and points at a catalogued arena.
        let subject = teams.iter().find_map(|t| {
            let v = reg.team_view(id, t.team_id).unwrap()?;
            let has_cell = v.profile.sections.iter().flat_map(|s| s.cells.iter()).any(|c| c.control == "arena" && c.field.is_some());
            if has_cell { v.arena.map(|a| (t.team_id, t.name.clone(), a)) } else { None }
        });
        match subject {
            None => {
                // No exposed reassignment cell: distinguish an unresolved relationship
                // from NHL07's resolved but not yet editable separate arena selector.
                let t = &teams[0];
                let p = reg.arena_assignment_preview(id, t.team_id, arenas.first().map(|a| a.arena_id).unwrap_or(1)).unwrap();
                assert!(!p.allowed && p.table.is_none(), "{label}: {p:?}");
                let reason = if p.from.is_some() { "separate arena selector" } else { "no home-arena link" };
                assert!(p.refusal.as_deref().unwrap().contains(reason), "{label}: {:?}", p.refusal);
                row.arena = format!("reassignment unsupported — {reason} (preview refuses with the reason)");
                arena_families_without += 1;
            }
            Some((team_id, team_name, from)) => {
                arena_families_with_link += 1;
                // Prefer a target that already has tenants, so the shared-arena disclosure is exercised.
                let target = arenas
                    .iter()
                    .filter(|a| a.arena_id != from.arena_id)
                    .max_by_key(|a| a.team_count)
                    .unwrap_or_else(|| panic!("{label}: a second arena exists"));
                let p = reg.arena_assignment_preview(id, team_id, target.arena_id).unwrap();
                assert!(p.allowed, "{label}: {:?}", p.refusal);
                assert_eq!(p.team_name, team_name);
                assert_eq!(p.from.as_ref().map(|a| a.arena_id), Some(from.arena_id));
                assert_eq!(p.to.as_ref().map(|a| a.arena_id), Some(target.arena_id));
                assert_eq!(p.to_tenants.len() as u32, target.team_count, "{label}: the disclosure names every team already at the target");
                let from_count = arenas.iter().find(|a| a.arena_id == from.arena_id).map(|a| a.team_count).unwrap_or(0);
                assert_eq!(p.from_other_tenants.len() as u32 + 1, from_count.max(1), "{label}: the disclosure names who stays behind");
                assert_eq!(p.from_becomes_vacant, p.from_other_tenants.is_empty());
                // (Names may legitimately collide between teams — placeholders, All-Star squads — so the
                // disclosure is filtered by team id, and only the counts are asserted here.)
                let (table_name, record, field) = (p.table.clone().unwrap(), p.record.unwrap(), p.field.clone().unwrap());
                // Refusals: same arena; an id that is not a catalogued arena.
                let same = reg.arena_assignment_preview(id, team_id, from.arena_id).unwrap();
                assert!(!same.allowed && same.refusal.as_deref().unwrap().contains("already plays"), "{label}: {same:?}");
                let bogus = arenas.iter().map(|a| a.arena_id).max().unwrap() + 1000;
                let unk = reg.arena_assignment_preview(id, team_id, bogus).unwrap();
                assert!(!unk.allowed && unk.refusal.as_deref().unwrap().contains("not in this database's arena table"), "{label}: {unk:?}");
                assert_eq!(reg.team_view(id, team_id).unwrap().unwrap().arena.map(|a| a.arena_id), Some(from.arena_id), "{label}: a preview writes nothing");

                // The write: the previewed cell, then the tenant counts move and nothing new fails validation.
                reg.set_field(id, &table_name, record, &field, FieldValue::Int(target.arena_id as i64)).unwrap_or_else(|e| panic!("{label}: assign: {e}"));
                let after = reg.team_view(id, team_id).unwrap().unwrap();
                assert_eq!(after.arena.as_ref().map(|a| a.arena_id), Some(target.arena_id), "{label}");
                let counts = reg.list_arenas(id).unwrap();
                let c = |aid: u64| counts.iter().find(|a| a.arena_id == aid).map(|a| a.team_count).unwrap();
                assert_eq!(c(target.arena_id), target.team_count + 1, "{label}: the target gained a tenant");
                assert_eq!(c(from.arena_id) + 1, from_count, "{label}: the old arena lost one");
                let report = reg.validate(id).unwrap();
                assert!(report.error_count <= before_report.error_count, "{label}: the move introduced errors ({} → {})", before_report.error_count, report.error_count);
                // Save → reopen on a disposable copy.
                let out = tmp.0.join(format!("{platform}-{game}-arena.db"));
                let re = save_and_reopen(&reg, id, &platform, &meta, &out);
                assert_eq!(reg.team_view(re, team_id).unwrap().unwrap().arena.map(|a| a.arena_id), Some(target.arena_id), "{label}: the assignment survives save and reopen");
                assert_eq!(reg.list_arenas(re).unwrap().iter().find(|a| a.arena_id == target.arena_id).unwrap().team_count, target.team_count + 1);
                reg.close_project(re);
                // One undo restores the original arena and the counts.
                assert!(reg.undo(id).unwrap(), "{label}: one undo");
                assert_eq!(reg.team_view(id, team_id).unwrap().unwrap().arena.map(|a| a.arena_id), Some(from.arena_id), "{label}: undo restores");
                assert_eq!(reg.list_arenas(id).unwrap().iter().find(|a| a.arena_id == target.arena_id).unwrap().team_count, target.team_count);
                row.arena = format!("impl+tested — {} → {} ({} already there; {} stay behind{})", from.name, target.name, p.to_tenants.len(), p.from_other_tenants.len(), if p.from_becomes_vacant { ", old arena vacant" } else { "" });
            }
        }

        // ------------------------------------------------------------------ retired numbers
        let t = &teams[0];
        let view = reg.team_view(id, t.team_id).unwrap().unwrap();
        let number = (1..=98u32).find(|n| !view.retired_numbers.contains(n)).unwrap();
        match reg.retire_jersey(id, t.team_id, number as u64) {
            Err(e) => {
                assert_eq!(reg.team_view(id, t.team_id).unwrap().unwrap().retired_numbers, view.retired_numbers, "{label}: a refused retire changes nothing");
                row.retired = format!("refused — {}", e.to_string().lines().next().unwrap_or_default());
            }
            Ok(out) => {
                assert!(out.edits >= 1, "{label}");
                assert!(reg.team_view(id, t.team_id).unwrap().unwrap().retired_numbers.contains(&number), "{label}: retired");
                let report = reg.validate(id).unwrap();
                assert!(report.error_count <= before_report.error_count, "{label}: retiring introduced errors");
                let out_path = tmp.0.join(format!("{platform}-{game}-retired.db"));
                let re = save_and_reopen(&reg, id, &platform, &meta, &out_path);
                assert!(reg.team_view(re, t.team_id).unwrap().unwrap().retired_numbers.contains(&number), "{label}: the retirement survives save and reopen");
                // Un-retire on the reopened copy is its own operation and restores the list.
                reg.unretire_jersey(re, t.team_id, number as u64).unwrap_or_else(|e| panic!("{label}: unretire: {e}"));
                assert_eq!(reg.team_view(re, t.team_id).unwrap().unwrap().retired_numbers, view.retired_numbers, "{label}: unretire restores");
                reg.close_project(re);
                assert!(reg.undo(id).unwrap(), "{label}: one undo");
                assert_eq!(reg.team_view(id, t.team_id).unwrap().unwrap().retired_numbers, view.retired_numbers, "{label}: undo restores");
                row.retired = format!("impl+tested — #{number} retired, saved, reopened, un-retired, undone");
            }
        }

        // ------------------------------------------------------------------ draft picks
        let picks = reg.team_draft_picks(id, t.team_id).unwrap_or_default();
        if picks.is_empty() {
            // Either the table is absent on the era, or this team holds none: both are recorded as observed.
            row.draft = "no draft picks listed for the first team (table absent on 07–10, or none held)".into();
        } else {
            let pick = &picks[0];
            let other = teams.iter().find(|x| x.team_id != t.team_id).unwrap();
            reg.trade_draft_pick(id, pick.record, other.team_id).unwrap_or_else(|e| panic!("{label}: trade: {e}"));
            assert!(reg.team_draft_picks(id, other.team_id).unwrap().iter().any(|p| p.record == pick.record), "{label}: the pick moved");
            assert!(!reg.team_draft_picks(id, t.team_id).unwrap().iter().any(|p| p.record == pick.record), "{label}: the pick left");
            let out_path = tmp.0.join(format!("{platform}-{game}-draft.db"));
            let re = save_and_reopen(&reg, id, &platform, &meta, &out_path);
            assert!(reg.team_draft_picks(re, other.team_id).unwrap().iter().any(|p| p.record == pick.record), "{label}: the trade survives save and reopen");
            reg.close_project(re);
            assert!(reg.undo(id).unwrap());
            assert!(reg.team_draft_picks(id, t.team_id).unwrap().iter().any(|p| p.record == pick.record), "{label}: undo returns the pick");
            row.draft = format!("impl+tested — {} {} rd {} traded to {}, saved, reopened, undone", pick.year, pick.round, pick.overall, other.name);
        }

        // ------------------------------------------------------------------ strategies
        match reg.team_strategies(id, t.team_id) {
            Err(e) => row.strategy = format!("refused — {}", e.to_string().lines().next().unwrap_or_default()),
            Ok(None) => row.strategy = "no strategy row for the first team".into(),
            Ok(Some(st)) => {
                let cell = st.tactics.iter().chain(st.team_biases.iter()).find(|c| c.field.editable && c.field.decoded.is_some() && c.field.range_high.is_some() && c.field.range_high > c.field.range_low);
                match cell {
                    None => row.strategy = "no editable in-range tactic cell".into(),
                    Some(c) => {
                        let cur = c.field.decoded.unwrap();
                        let (lo, hi) = (c.field.range_low.unwrap_or(0), c.field.range_high.unwrap());
                        let new = if cur >= hi { lo } else { cur + 1 };
                        reg.set_field(id, &c.field.studio_table, st.record, &c.field.studio_field, FieldValue::Int(new)).unwrap_or_else(|e| panic!("{label}: strategy {}: {e}", c.label));
                        let read = |pid: u32| reg.team_strategies(pid, t.team_id).unwrap().unwrap().tactics.iter().chain(reg.team_strategies(pid, t.team_id).unwrap().unwrap().team_biases.iter()).find(|x| x.field.studio_field == c.field.studio_field).unwrap().field.decoded;
                        assert_eq!(read(id), Some(new), "{label}: {}", c.label);
                        let out_path = tmp.0.join(format!("{platform}-{game}-strategy.db"));
                        let re = save_and_reopen(&reg, id, &platform, &meta, &out_path);
                        assert_eq!(read(re), Some(new), "{label}: the strategy survives save and reopen");
                        reg.close_project(re);
                        assert!(reg.undo(id).unwrap());
                        assert_eq!(read(id), Some(cur), "{label}: undo restores");
                        row.strategy = format!("impl+tested — {} {cur} → {new}, saved, reopened, undone", c.label);
                    }
                }
            }
        }
        reg.close_project(id);
        table.insert(label, row);
    }
    println!("| anchor | home arena | retired numbers | draft picks | strategies |");
    println!("|---|---|---|---|---|");
    for (k, r) in &table {
        println!("| {k} | {} | {} | {} | {} |", r.arena, r.retired, r.draft, r.strategy);
    }
    println!("arena link: {arena_families_with_link} anchors with, {arena_families_without} without");
    // Measured on this sweep (24 Sep 2026): the `kOtt` home-arena link exists on NHL 13 / 14 / 15 /
    // Legacy on both consoles and on every PS4 title. Earlier titles lack this kOtt cell;
    // NHL07 resolves its relationship separately, without a reassignment editor yet.
    assert_eq!((arena_families_with_link, arena_families_without), (15, 12));
    for (k, r) in &table {
        let game = k.rsplit('/').next().unwrap();
        let expect_link = k.starts_with("ps4/") || matches!(game, "nhl13" | "nhl14" | "nhl15" | "nhllegacy");
        assert_eq!(r.arena.starts_with("impl+tested"), expect_link, "{k}: {}", r.arena);
        // Retired numbers: `wcSL` is absent on NHL 07 only.
        assert_eq!(r.retired.starts_with("impl+tested"), game != "nhl07", "{k}: {}", r.retired);
        // Draft picks: `qHav` rows exist from NHL 11 on.
        let expect_draft = k.starts_with("ps4/") || matches!(game, "nhl11" | "nhl12" | "nhl13" | "nhl14" | "nhl15" | "nhllegacy");
        assert_eq!(r.draft.starts_with("impl+tested"), expect_draft, "{k}: {}", r.draft);
        assert!(r.strategy.starts_with("impl+tested"), "{k}: {}", r.strategy);
    }
}
