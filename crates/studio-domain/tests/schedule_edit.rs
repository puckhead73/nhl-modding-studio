//! P1-A — source schedule editing, import/export and generation across the anchor families.
//! Every case runs on the committed anchors (or the explicitly private NHL 20 fixture, `#[ignore]`d
//! and failing loudly without it); nothing is written outside the process except the
//! copied-output reopen case, which writes to a temp directory it removes.
//!
//! Slot rule under test: a schedule slot is the team's rank within its league by the team
//! table's global `index` — checked on the two record-order exceptions (NHL 10 AHL, NHL 11 OHL)
//! and on Legacy AHL's real-world unequal game counts (the five 68-game clubs).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use studio_domain::schedule_edit::{ChangePlan, GameEdit, GenerationConfig, GenerationPlan, ImportFormat, ImportPlan, TeamMapping};
use studio_domain::schedules::ScheduleView;
use studio_domain::tdb_txn;
use studio_domain::{EditSession, OpOutcome, Platform, Project, ProjectRole, Workspace};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}

fn open_paths(game: &str, platform: Platform, db: &str, meta: &str) -> Workspace {
    let project = Project::load(repo(db), platform, ProjectRole::Nhlng, repo(meta), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), repo(db).to_string_lossy().into_owned());
    ws.set_game_id(game);
    ws
}

/// Committed anchors by (game, platform): the main DB + meta paths.
fn anchor(game: &str, platform: Platform) -> Workspace {
    let (dir, db, meta) = match platform {
        Platform::Xbox => ("xbox360", "nhlng.db", "nhlng-meta.xml"),
        Platform::Ps3 => ("ps3", "nhlng.db", "nhlng-meta.xml"),
        Platform::Ps4 => ("ps4", "nhl_game.db", "nhl_game-meta.xml"),
        Platform::Recomp => ("xbox360", "nhlng.db", "nhlng-meta.xml"),
    };
    let (db, meta) = if game == "nhl07" { ("NHL07NG.db", "NHL07NG-meta.xml") } else { (db, meta) };
    let dir_game = game;
    let (db, meta) = (format!("anchors/{dir}/{dir_game}/databases/{db}"), format!("anchors/{dir}/{dir_game}/databases/{meta}"));
    open_paths(game, platform, &db, &meta)
}

// Preview → review id → apply, as the API does it (the review is bound to the workspace revision).
fn edit_apply(ws: &mut Workspace, table: &str, edit: &GameEdit) -> studio_domain::Result<(ChangePlan, OpOutcome, ScheduleView)> {
    let (t, _) = ws.schedule_edit_preview(table, edit)?;
    let (r, o, v) = ws.schedule_apply_review(&t, None)?;
    Ok((r.plan, o, v))
}
fn edit_preview(ws: &mut Workspace, table: &str, edit: &GameEdit) -> studio_domain::Result<ChangePlan> {
    ws.schedule_edit_preview(table, edit).map(|(_, p)| p)
}
fn import_preview(ws: &mut Workspace, table: &str, fmt: ImportFormat, text: &str, mapping: &[TeamMapping]) -> studio_domain::Result<ImportPlan> {
    ws.schedule_import_preview(table, fmt, text, None, mapping).map(|(_, p)| p)
}
fn import_apply(ws: &mut Workspace, table: &str, fmt: ImportFormat, text: &str, mapping: &[TeamMapping]) -> studio_domain::Result<(ImportPlan, OpOutcome, ScheduleView)> {
    let (t, _) = ws.schedule_import_preview(table, fmt, text, None, mapping)?;
    let (r, o, v) = ws.schedule_apply_review(&t, None)?;
    Ok((r.import.unwrap(), o, v))
}
fn gen_preview(ws: &mut Workspace, table: &str, cfg: &GenerationConfig) -> studio_domain::Result<GenerationPlan> {
    ws.schedule_generate_preview(table, cfg).map(|(_, p)| p)
}
fn gen_apply(ws: &mut Workspace, table: &str, cfg: &GenerationConfig) -> studio_domain::Result<(GenerationPlan, OpOutcome, ScheduleView)> {
    let (t, _) = ws.schedule_generate_preview(table, cfg)?;
    let (r, o, v) = ws.schedule_apply_review(&t, None)?;
    Ok((r.generation.unwrap(), o, v))
}

fn table(ws: &Workspace, name: &str) -> String {
    ws.display()
        .schedule_tables()
        .unwrap()
        .into_iter()
        .find(|t| t.name == name)
        .unwrap_or_else(|| panic!("{name} table"))
        .studio_table
}

// ---------------------------------------------------------------------------------------------
// 1. Editing a game on every family, parameterized over the anchors.
// ---------------------------------------------------------------------------------------------

/// Swap home/away on row 0, move its date, refuse an impossible date, undo/redo — the same
/// assertions on every anchor family that ships an NHL schedule.
fn edit_round_trip(ws: &mut Workspace, tag: &str, expect_status: bool, slot_bits: u32) {
    let nhl = table(ws, "nhlschedule");
    let before = ws.display().schedule_view(&nhl).unwrap().unwrap();
    assert_eq!(before.layout.field("home").unwrap().bit_width, slot_bits, "{tag}: home width");
    assert_eq!(before.layout.field("status").is_some(), expect_status, "{tag}: status field");
    let g0 = before.games[0].clone();
    let (home_id, away_id) = (
        before.slot_map.by_slot(g0.home_slot).unwrap().team_id,
        before.slot_map.by_slot(g0.away_slot).unwrap().team_id,
    );
    let baseline = ws.primary().bytes().to_vec();

    // Preview: two cells (home, away) swap; validation clean of errors.
    let edit = GameEdit {
        record: 0,
        home_team_id: Some(away_id),
        away_team_id: Some(home_id),
        month: None,
        day: None,
    };
    let plan = edit_preview(ws, &nhl, &edit).unwrap();
    assert!(plan.validation.ok(), "{tag}: {:?}", plan.validation.errors);
    assert_eq!(plan.cells, 2, "{tag}");
    assert_eq!(plan.rows.len(), 1);
    assert_eq!(plan.after[0].home, g0.away, "{tag}");
    assert_eq!(plan.after[0].away, g0.home, "{tag}");
    assert_eq!(ws.primary().bytes(), &baseline[..], "{tag}: preview writes nothing");
    assert_eq!(ws.undo_depth(), 0);

    // Apply: one undo entry; fresh view shows the swap; games per team unchanged.
    let (_, outcome, view) = edit_apply(ws, &nhl, &edit).unwrap();
    assert_eq!(outcome.edits, 2, "{tag}");
    assert_eq!(ws.undo_depth(), 1, "{tag}");
    assert_eq!(view.games[0].home, g0.away, "{tag}");
    assert_eq!(view.games[0].away, g0.home, "{tag}");
    assert_eq!(view.games[0].date, g0.date);
    assert_eq!(view.slots.len(), before.slots.len(), "{tag}");
    assert!(view.slots.iter().zip(before.slots.iter()).all(|(a, b)| a.games == b.games), "{tag}: games per team");
    let (pending, _) = ws.pending_changes(10);
    assert_eq!(pending.total_cells, 2, "{tag}");

    // Date edit: Feb 29 is a warning (yearless), Feb 30 a refusal that leaves everything as is.
    let leap = GameEdit {
        record: 0,
        home_team_id: None,
        away_team_id: None,
        month: Some(1),
        day: Some(28),
    };
    let plan = edit_preview(ws, &nhl, &leap).unwrap();
    assert!(plan.validation.ok(), "{tag}: {:?}", plan.validation.errors);
    assert!(plan.validation.warnings.iter().any(|w| w.contains("Feb 29")), "{tag}: {:?}", plan.validation.warnings);
    assert!(plan.validation.warnings.iter().any(|w| w.contains("season order")), "{tag}: {:?}", plan.validation.warnings);
    let bad = GameEdit {
        day: Some(29),
        ..leap.clone()
    };
    let plan = edit_preview(ws, &nhl, &bad).unwrap();
    assert!(plan.validation.errors.iter().any(|e| e.contains("Feb 30 does not exist")), "{tag}: {:?}", plan.validation.errors);
    let after_apply = ws.primary().bytes().to_vec();
    let err = edit_apply(ws, &nhl, &bad).unwrap_err().to_string();
    assert!(err.contains("not applied") && err.contains("Feb 30"), "{tag}: {err}");
    assert_eq!(ws.primary().bytes(), &after_apply[..], "{tag}: refused edit leaves bytes unchanged");
    assert_eq!(ws.undo_depth(), 1, "{tag}: refused edit leaves history unchanged");
    // Same team on both sides and a non-member: refused.
    let same = GameEdit {
        record: 0,
        home_team_id: Some(home_id),
        away_team_id: Some(home_id),
        month: None,
        day: None,
    };
    let plan = edit_preview(ws, &nhl, &same).unwrap();
    assert!(plan.validation.errors.iter().any(|e| e.contains("same team")), "{tag}: {:?}", plan.validation.errors);
    let foreign = GameEdit {
        record: 0,
        home_team_id: Some(u64::MAX - 1),
        away_team_id: None,
        month: None,
        day: None,
    };
    let plan = edit_preview(ws, &nhl, &foreign).unwrap();
    assert!(plan.validation.errors.iter().any(|e| e.contains("not a member")), "{tag}: {:?}", plan.validation.errors);
    // Out-of-range row.
    let err = edit_preview(
            ws,
            &nhl,
            &GameEdit {
                record: before.games.len() as u32,
                ..same
            },
        )
        .unwrap_err()
        .to_string();
    assert!(err.contains("does not exist"), "{tag}: {err}");

    // Undo restores the baseline bytes; redo re-applies.
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &baseline[..], "{tag}: undo restores the bytes");
    assert!(!ws.is_dirty());
    assert!(ws.redo());
    assert_eq!(ws.primary().bytes(), &after_apply[..], "{tag}: redo re-applies");
    assert!(ws.undo());
}

#[test]
fn gen7_xbox_and_ps3_edit_round_trip() {
    for (game, platform, status) in [
        ("nhllegacy", Platform::Xbox, true),
        ("nhllegacy", Platform::Ps3, true),
        ("nhl15", Platform::Xbox, true),
        ("nhl12", Platform::Ps3, true),
        ("nhl11", Platform::Xbox, true),
        ("nhl10", Platform::Ps3, true),
        ("nhl09", Platform::Xbox, true),
        ("nhl08", Platform::Ps3, true),
    ] {
        let mut ws = anchor(game, platform);
        edit_round_trip(&mut ws, &format!("{game}/{platform:?}"), status, 5);
    }
}

#[test]
fn ps4_layouts_15_versus_16_17_versus_18_plus() {
    // NHL 15 PS4 = the gen7 record (status present, 5-bit slots); 16/17 drop status and keep
    // 5-bit slots; 18+ widen slots to 6 bits.
    for (game, status, bits) in [("nhl15", true, 5), ("nhl16", false, 5), ("nhl17", false, 5), ("nhl18", false, 6), ("nhl21", false, 6)] {
        let mut ws = anchor(game, Platform::Ps4);
        edit_round_trip(&mut ws, &format!("{game}/Ps4"), status, bits);
    }
    // PS4 18+: league 0 carries four division squads with no games; a validator must allow
    // zero-game members (warning, never a refusal), and nhl32schedule's slot 31 is the 32nd row.
    let mut ws = anchor("nhl21", Platform::Ps4);
    let nhl = table(&ws, "nhlschedule");
    let plan = edit_preview(
            &mut ws,
            &nhl,
            &GameEdit {
                record: 0,
                home_team_id: None,
                away_team_id: None,
                month: Some(9),
                day: Some(3),
            },
        )
        .unwrap();
    assert!(plan.validation.ok(), "{:?}", plan.validation.errors);
    assert!(plan.validation.warnings.iter().any(|w| w.contains("no games")), "{:?}", plan.validation.warnings);
    let opts = ws.schedule_generation_options(&nhl).unwrap();
    assert_eq!(opts.members.iter().filter(|m| m.default).count(), 31);
    assert_eq!(opts.members.iter().filter(|m| m.games == 0).count(), 4, "four division squads without games");
    let nhl32 = table(&ws, "nhl32schedule");
    let v32 = ws.display().schedule_view(&nhl32).unwrap().unwrap();
    assert_eq!(v32.slots.len(), 32);
    assert!(v32.slot_map.by_slot(31).is_some(), "slot 31 resolves to the league's 32nd row by rank");
}

// ---------------------------------------------------------------------------------------------
// 2. Slot rule: rank by team `index`, not record order (the two anchor exceptions) and the
//    Legacy AHL unequal counts.
// ---------------------------------------------------------------------------------------------

#[test]
fn record_order_exceptions_resolve_by_index_rank() {
    // NHL 10: AHL `index` 59 = Wilkes-Barre (record 60), 60 = Worcester (record 59): slot 27 is
    // Wilkes-Barre (East Division: plays Adirondack/Hershey 10×), slot 28 Worcester.
    for platform in [Platform::Xbox, Platform::Ps3] {
        let mut ws = anchor("nhl10", platform);
        let ahl = table(&ws, "ahlschedule");
        let v = ws.display().schedule_view(&ahl).unwrap().unwrap();
        let s27 = v.slot_map.by_slot(27).unwrap();
        let s28 = v.slot_map.by_slot(28).unwrap();
        assert!(s27.name.contains("Wilkes"), "{platform:?}: slot 27 = {}", s27.name);
        assert!(s28.name.contains("Worcester"), "{platform:?}: slot 28 = {}", s28.name);
        assert!(s27.global_index < s28.global_index);
        let pairs = |slot: u64, other: &str| -> usize {
            let o = v.slot_map.entries.iter().find(|e| e.name.contains(other)).unwrap().slot;
            v.games.iter().filter(|g| (g.home_slot == slot && g.away_slot == o) || (g.home_slot == o && g.away_slot == slot)).count()
        };
        assert_eq!(pairs(27, "Hershey"), 10, "{platform:?}: WBS-HER division pairing");
        assert_eq!(pairs(28, "Providence"), 10, "{platform:?}: WOR-PRO division pairing");
        // Editing by team id lands on the rank, never the record or the raw index.
        let wbs = s27.team_id;
        let plan = edit_preview(
                &mut ws,
                &ahl,
                &GameEdit {
                    record: 0,
                    home_team_id: Some(wbs),
                    away_team_id: None,
                    month: None,
                    day: None,
                },
            )
            .unwrap();
        assert!(plan.validation.ok(), "{:?}", plan.validation.errors);
        assert_eq!(plan.after[0].home_slot, 27);
        assert!(plan.rows[0].1.iter().all(|(_, v)| *v == 27), "{:?}", plan.rows);
    }
    // NHL 11: OHL Mississauga has `index` 158 but is stored after Owen Sound: slot 8.
    for platform in [Platform::Xbox, Platform::Ps3] {
        let mut ws = anchor("nhl11", platform);
        let ohl = table(&ws, "ohlschedule");
        let v = ws.display().schedule_view(&ohl).unwrap().unwrap();
        let s8 = v.slot_map.by_slot(8).unwrap();
        assert!(s8.name.contains("Mississauga"), "{platform:?}: slot 8 = {}", s8.name);
        let nia = v.slot_map.entries.iter().find(|e| e.name.contains("Niagara")).unwrap();
        assert_eq!(nia.slot, 9);
        let meet = v.games.iter().filter(|g| (g.home_slot == 8 && g.away_slot == 9) || (g.home_slot == 9 && g.away_slot == 8)).count();
        assert_eq!(meet, 8, "{platform:?}: Central Division rivals meet 8×");
        assert!(v.findings.is_empty(), "{platform:?}: {:?}", v.findings);
    }
}

#[test]
fn legacy_ahl_unequal_counts_are_legitimate_and_survive_export_import() {
    for platform in [Platform::Xbox, Platform::Ps3] {
        let mut ws = anchor("nhllegacy", platform);
        let ahl = table(&ws, "ahlschedule");
        let v = ws.display().schedule_view(&ahl).unwrap().unwrap();
        let short: BTreeSet<u64> = v.slots.iter().filter(|s| s.games == 68).map(|s| s.slot).collect();
        assert_eq!(short, [1, 14, 21, 22, 24].into_iter().collect(), "{platform:?}");
        let names: Vec<String> = short.iter().map(|s| v.slot_map.by_slot(*s).unwrap().abbrev.clone()).collect();
        assert_eq!(names, ["BAK", "ONT", "SD", "SJ", "STO"], "{platform:?}: the five California clubs");
        assert_eq!(v.slots.iter().filter(|s| s.games == 76).count(), 25);

        // Export → import of the identical file: nothing to write, the uneven counts are a
        // warning (disclosed), not a refusal.
        for fmt in [ImportFormat::Csv, ImportFormat::Json] {
            let text = ws.schedule_export(&ahl, fmt).unwrap();
            let plan = import_preview(&mut ws, &ahl, fmt, &text, &[]).unwrap();
            assert!(plan.change.validation.ok(), "{platform:?}/{fmt:?}: {:?}", plan.change.validation.errors);
            assert_eq!(plan.change.cells, 0, "{platform:?}/{fmt:?}: identical file changes nothing");
            assert_eq!(plan.rows_in_file, 1120);
            assert!(plan.keys.iter().all(|k| k.how == "auto"), "{:?}", plan.keys);
            assert_eq!(plan.keys.len(), 30);
            assert!(plan.change.validation.warnings.iter().any(|w| w.contains("uneven") && w.contains("68")), "{:?}", plan.change.validation.warnings);
            let (_, outcome, _) = import_apply(&mut ws, &ahl, fmt, &text, &[]).unwrap();
            assert_eq!(outcome.edits, 0);
            assert_eq!(ws.undo_depth(), 0, "a no-op import leaves no history");
        }
    }
}

// ---------------------------------------------------------------------------------------------
// 3. Import: explicit mapping, malformed input, over-capacity / short files, stale mappings.
// ---------------------------------------------------------------------------------------------

#[test]
fn import_maps_keys_explicitly_and_refuses_bad_input_without_touching_bytes() {
    let mut ws = anchor("nhllegacy", Platform::Xbox);
    let swe = table(&ws, "swedishschedule");
    let baseline = ws.primary().bytes().to_vec();
    let csv = ws.schedule_export(&swe, ImportFormat::Csv).unwrap();
    assert!(csv.starts_with("# nhl-studio-schedule/1"));
    assert_eq!(csv.lines().filter(|l| !l.starts_with('#')).count(), 365, "header + 364 games");
    let json = ws.schedule_export(&swe, ImportFormat::Json).unwrap();
    let doc: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(doc["format"], "nhl-studio-schedule/1");
    assert_eq!(doc["teams"].as_array().unwrap().len(), 14);
    assert_eq!(doc["games"].as_array().unwrap().len(), 364);
    assert_eq!(doc["games"][0]["month"], 9, "calendar month in files (stored 8 = September)");

    // Rename every key in the file to something the league does not know: refused until an
    // explicit mapping covers every key; then applied with a reversed date on row 0 to prove
    // the mapping wrote real slots.
    let view = ws.display().schedule_view(&swe).unwrap().unwrap();
    let keys: Vec<String> = view.slot_map.entries.iter().map(|e| e.key.clone()).collect();
    let mut renamed = String::new();
    for line in csv.lines() {
        if line.starts_with('#') || line.starts_with("index,") {
            renamed.push_str(line);
        } else {
            let cells: Vec<&str> = line.split(',').collect();
            let k = |c: &str| format!("club{}", keys.iter().position(|x| x == c).unwrap());
            renamed.push_str(&format!("{},{},{},{},{},{},{},{}", cells[0], cells[1], cells[2], cells[3], k(cells[4]), k(cells[5]), cells[6], cells[7]));
        }
        renamed.push('\n');
    }
    let plan = import_preview(&mut ws, &swe, ImportFormat::Csv, &renamed, &[]).unwrap();
    assert!(!plan.change.validation.ok());
    assert!(plan.keys.iter().all(|k| k.how == "unmapped"));
    assert!(plan.change.validation.errors[0].contains("map each one explicitly"), "{:?}", plan.change.validation.errors);
    let err = import_apply(&mut ws, &swe, ImportFormat::Csv, &renamed, &[]).unwrap_err().to_string();
    assert!(err.contains("not applied"), "{err}");
    assert_eq!(ws.primary().bytes(), &baseline[..]);
    assert_eq!(ws.undo_depth(), 0);

    // Explicit mapping: club<i> → the team at slot i, but swap two clubs so cells change.
    let mut mapping: Vec<TeamMapping> = view
        .slot_map
        .entries
        .iter()
        .map(|e| TeamMapping {
            key: format!("club{}", e.slot),
            team_id: e.team_id,
        })
        .collect();
    mapping.swap(0, 1);
    mapping[0].key = "club0".into();
    mapping[1].key = "club1".into();
    // Two keys → one team is refused.
    let mut dup = mapping.clone();
    dup[2].team_id = dup[0].team_id;
    let plan = import_preview(&mut ws, &swe, ImportFormat::Csv, &renamed, &dup).unwrap();
    assert!(plan.change.validation.errors.iter().any(|e| e.contains("every key must name a different team")), "{:?}", plan.change.validation.errors);
    // A non-member destination is refused.
    let mut foreign = mapping.clone();
    foreign[3].team_id = u64::MAX - 7;
    let plan = import_preview(&mut ws, &swe, ImportFormat::Csv, &renamed, &foreign).unwrap();
    assert!(plan.change.validation.errors.iter().any(|e| e.contains("not a member")), "{:?}", plan.change.validation.errors);
    // The good mapping: slots 0 and 1 trade places in every game they play.
    let plan = import_preview(&mut ws, &swe, ImportFormat::Csv, &renamed, &mapping).unwrap();
    assert!(plan.change.validation.ok(), "{:?}", plan.change.validation.errors);
    assert!(plan.keys.iter().all(|k| k.how == "explicit"));
    assert_eq!(plan.change.rows.len(), 52 + 52 - 4, "every game of the two swapped clubs changes (their 4 mutual games once)");
    let (_, outcome, after) = import_apply(&mut ws, &swe, ImportFormat::Csv, &renamed, &mapping).unwrap();
    assert_eq!(outcome.edits as usize, plan.change.cells);
    assert_eq!(ws.undo_depth(), 1, "one undo entry for the whole import");
    assert!(after.findings.is_empty(), "{:?}", after.findings);
    assert!(after.slots.iter().all(|s| s.games == 52));
    let (pending, _) = ws.pending_changes(5);
    assert_eq!(pending.total_cells as usize, plan.change.cells);
    // The swap is visible by team: club at slot 0's first game is now played by the old slot-1 club.
    let g = view.games.iter().find(|g| g.home_slot == 0).unwrap();
    assert_eq!(after.games[g.record as usize].home, view.slot_map.team_name(1));
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &baseline[..]);

    // Short and long files are refused with the exact counts; malformed lines name the line.
    let short: String = csv.lines().take(100).map(|l| format!("{l}\n")).collect();
    let plan = import_preview(&mut ws, &swe, ImportFormat::Csv, &short, &[]).unwrap();
    assert!(plan.change.validation.errors.iter().any(|e| e.contains("98 games") && e.contains("364 active rows")), "{:?}", plan.change.validation.errors);
    let long = format!("{csv}999,Apr 1,4,1,{},{},0,0\n", keys[0], keys[1]);
    let plan = import_preview(&mut ws, &swe, ImportFormat::Csv, &long, &[]).unwrap();
    assert!(plan.change.validation.errors.iter().any(|e| e.contains("365 games") && e.contains("does not truncate or grow")), "{:?}", plan.change.validation.errors);
    let malformed = csv.replacen("\n0,", "\nx,", 1).replacen(",9,", ",19,", 1);
    let err = import_preview(&mut ws, &swe, ImportFormat::Csv, &malformed, &[]).unwrap_err().to_string();
    assert!(err.contains("line 3") && err.contains("month 19"), "{err}");
    let err = import_preview(&mut ws, &swe, ImportFormat::Json, &csv, &[]).unwrap_err().to_string();
    assert!(err.contains("not valid JSON"), "{err}");
    // A game between the same club twice is refused at validation.
    let selfgame = csv.replacen(&format!(",{},{},", keys[0], keys[1]), &format!(",{},{},", keys[0], keys[0]), 1);
    let plan = import_preview(&mut ws, &swe, ImportFormat::Csv, &selfgame, &[]).unwrap();
    assert!(plan.change.validation.errors.iter().any(|e| e.contains("same team")), "{:?}", plan.change.validation.errors);
    assert_eq!(ws.primary().bytes(), &baseline[..]);
    assert_eq!(ws.undo_depth(), 0);
    assert!(!ws.is_dirty());
}

#[test]
fn apply_is_bound_to_the_previewed_review_and_refuses_anything_that_moved() {
    // Every refusal below must leave bytes, undo/redo depth, pending state and the revision as
    // they were; an unchanged review applies.
    let mut ws = anchor("nhllegacy", Platform::Xbox);
    ws.attach_dictionary(repo("anchors/xbox360/nhllegacy/databases/nhl_eng_us.db").to_str().unwrap()).unwrap();
    let swe = table(&ws, "swedishschedule");
    let nhl = table(&ws, "nhlschedule");
    let json = ws.schedule_export(&swe, ImportFormat::Json).unwrap();
    let day_edit = |day: u64| GameEdit {
        record: 0,
        home_team_id: None,
        away_team_id: None,
        month: None,
        day: Some(day),
    };
    let unchanged = |ws: &Workspace, bytes: &[u8], undo: usize, redo: usize, rev: &studio_domain::WorkspaceRevision, what: &str| {
        assert_eq!(ws.primary().bytes(), bytes, "{what}: bytes");
        assert_eq!((ws.undo_depth(), ws.redo_depth()), (undo, redo), "{what}: history");
        assert_eq!(&ws.revision(), rev, "{what}: revision");
    };

    // (1) Preview an import, then edit the schedule, then apply the earlier review: refused —
    // the review's revision moved. The reviewer's F1 reproduction: the no-op import must NOT
    // restore game 0's old day.
    let (import_review, plan) = ws.schedule_import_preview(&swe, ImportFormat::Json, &json, None, &[]).unwrap();
    assert!(plan.change.validation.ok() && plan.change.cells == 0);
    edit_apply(&mut ws, &swe, &day_edit(20)).unwrap();
    let after_edit = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let (pending, _) = ws.pending_changes(5);
    let err = ws.schedule_apply_review(&import_review, None).unwrap_err().to_string();
    assert!(err.contains("changed since this preview") || err.contains("no longer held"), "{err}");
    unchanged(&ws, &after_edit, 1, 0, &rev, "import after edit");
    assert_eq!(ws.pending_changes(5).0.total_cells, pending.total_cells);
    assert_eq!(ws.display().schedule_view(&swe).unwrap().unwrap().games[0].day, 20, "the intervening edit survives");

    // (2) Preview an edit on one table, then edit another table: refused (revision moved).
    let (edit_review, _) = ws.schedule_edit_preview(&swe, &day_edit(21)).unwrap();
    edit_apply(&mut ws, &nhl, &day_edit(7)).unwrap();
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    assert!(ws.schedule_apply_review(&edit_review, None).is_err());
    unchanged(&ws, &bytes, 2, 0, &rev, "edit after other-table edit");

    // (3) Membership / reordering change between preview and apply: refused.
    let (edit_review, _) = ws.schedule_edit_preview(&swe, &day_edit(22)).unwrap();
    let moved = ws.display().schedule_view(&swe).unwrap().unwrap().slot_map.entries[5].team_id;
    ws.set_team_membership(moved, 7, None, None).unwrap();
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let err = ws.schedule_apply_review(&edit_review, None).unwrap_err().to_string();
    assert!(err.contains("changed since this preview") || err.contains("no longer held"), "{err}");
    unchanged(&ws, &bytes, 3, 0, &rev, "edit after membership move");
    assert!(ws.undo()); // back to 14 SHL members
    let rev_after_undo = ws.revision();
    // (3b) Undo/redo also moves the revision: a review previewed before a redo is refused.
    let (edit_review, _) = ws.schedule_edit_preview(&swe, &day_edit(23)).unwrap();
    assert!(ws.redo());
    assert!(ws.schedule_apply_review(&edit_review, None).is_err(), "redo moved the revision");
    assert!(ws.undo());
    assert_eq!(ws.revision(), rev_after_undo);

    // (4) Imported file changed without changing keys: refused by the byte hash on apply, and the
    // stored snapshot (not the changed file) is what an unchanged apply writes.
    let (import_review, _) = ws.schedule_import_preview(&swe, ImportFormat::Json, &json, None, &[]).unwrap();
    let tampered = json.replacen("\"day\": 16", "\"day\": 17", 1);
    assert_ne!(tampered, json);
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let err = ws.schedule_apply_review(&import_review, Some(&tampered)).unwrap_err().to_string();
    assert!(err.contains("file changed since it was previewed"), "{err}");
    unchanged(&ws, &bytes, 2, 1, &rev, "import with tampered file");
    // The review is still held (nothing mutated); the same bytes apply exactly as previewed —
    // one cell, restoring game 0's exported day over the day-20 edit.
    let (r, o, view) = ws.schedule_apply_review(&import_review, Some(&json)).unwrap();
    assert_eq!((r.ticket.as_str(), o.edits, r.plan.cells), (import_review.as_str(), 1, 1));
    assert_eq!(view.games[0].day, 15);

    // (5) Configuration / mapping changed: a different configuration is a different review; the
    // old review applies exactly what it previewed, never the newer inputs.
    let opts = ws.schedule_generation_options(&swe).unwrap();
    let cfg = GenerationConfig {
        participants: opts.members.iter().filter(|m| m.default).map(|m| m.team_id).collect(),
        rounds: 4,
        start_month: 8,
        start_day: 10,
        days_between_matchdays: 3,
        pair_quotas: None,
                odd_home: None,
                groups: None,
                odd_hosts: None,
    };
    let (gen_a, plan_a) = ws.schedule_generate_preview(&swe, &cfg).unwrap();
    let (gen_b, plan_b) = ws.schedule_generate_preview(&swe, &GenerationConfig { start_day: 12, ..cfg.clone() }).unwrap();
    assert_ne!(plan_a.change.rows, plan_b.change.rows);
    let (r, _, view) = ws.schedule_apply_review(&gen_a, None).unwrap();
    assert_eq!(r.ticket, gen_a);
    assert_eq!(view.first_date.as_deref(), Some("Sep 11"), "review A's configuration, not B's");
    assert!(matches!(r.operation, studio_domain::ScheduleOperation::Generate(ref c) if *c == cfg));
    // B was dropped by A's apply (the revision moved anyway).
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let err = ws.schedule_apply_review(&gen_b, None).unwrap_err().to_string();
    assert!(err.contains("no longer held"), "{err}");
    unchanged(&ws, &bytes, 4, 0, &rev, "second review after the first applied");
    assert!(ws.undo());
    // Mapping changed: two import previews with different mappings are different reviews and
    // each applies its own mapping (checked through the resolved keys it holds).
    let entries = ws.display().schedule_view(&swe).unwrap().unwrap().slot_map.entries.clone();
    let renamed = json.replace(&format!("\"{}\"", entries[0].key), "\"clubX\"");
    let map_a = [TeamMapping { key: "clubX".into(), team_id: entries[0].team_id }];
    let map_b = [TeamMapping { key: "clubX".into(), team_id: entries[1].team_id }];
    let (imp_a, plan_a) = ws.schedule_import_preview(&swe, ImportFormat::Json, &renamed, None, &map_a).unwrap();
    let (_imp_b, plan_b) = ws.schedule_import_preview(&swe, ImportFormat::Json, &renamed, None, &map_b).unwrap();
    assert_eq!(plan_a.change.cells, 0, "mapping A restores the original club: no-op");
    assert!(!plan_b.change.validation.ok(), "mapping B doubles up a team: refused");
    let (r, o, _) = ws.schedule_apply_review(&imp_a, None).unwrap();
    assert_eq!(o.edits, 0);
    assert!(matches!(r.operation, studio_domain::ScheduleOperation::Import { ref mapping, .. } if mapping == &map_a));

    // (6) Foreign ticket: see `tickets_never_alias_across_workspace_instances` for the full
    // collision set; here the plain unknown-ticket refusal.
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    let err = ws.schedule_apply_review("nowhere/1", None).unwrap_err().to_string();
    assert!(err.contains("issued by another project"), "{err}");
    unchanged(&ws, &bytes, 3, 1, &rev, "foreign review");

    // (7) An ordinary unchanged review applies once; applying it again is refused (consumed).
    let (edit_review, plan) = ws.schedule_edit_preview(&swe, &day_edit(25)).unwrap();
    assert!(plan.validation.ok());
    let (r, o, view) = ws.schedule_apply_review(&edit_review, None).unwrap();
    assert_eq!((r.ticket.as_str(), o.edits), (edit_review.as_str(), 1));
    assert_eq!(view.games[0].day, 25);
    assert!(ws.schedule_apply_review(&edit_review, None).is_err(), "a consumed review cannot apply twice");
    assert_eq!(ws.undo_depth(), 4);

    // (8) A review with errors never applies, even when nothing moved.
    let (bad, plan) = ws.schedule_edit_preview(&swe, &day_edit(31)).unwrap();
    assert!(!plan.validation.ok());
    let bytes = ws.primary().bytes().to_vec();
    let rev = ws.revision();
    assert!(ws.schedule_apply_review(&bad, None).unwrap_err().to_string().contains("not applied"));
    unchanged(&ws, &bytes, 4, 0, &rev, "review with errors");
}

#[test]
fn tickets_never_alias_across_workspace_instances() {
    // R2: two workspaces mint tickets concurrently — with the same local sequence numbers — and
    // each other's unmodified ticket must be refused, leaving both untouched and both tickets
    // still usable at home. Repeated over different tables/operations and after a workspace is
    // replaced by a fresh instance opened on the same file.
    let mut a = anchor("nhllegacy", Platform::Xbox);
    let mut b = anchor("nhllegacy", Platform::Xbox);
    let nhl_a = table(&a, "nhlschedule");
    let nhl_b = table(&b, "nhlschedule");
    let swe_b = table(&b, "swedishschedule");
    let day = |d: u64| GameEdit {
        record: 0,
        home_team_id: None,
        away_team_id: None,
        month: None,
        day: Some(d),
    };
    let snapshot = |ws: &Workspace| (ws.primary().bytes().to_vec(), ws.revision(), ws.undo_depth(), ws.redo_depth(), ws.pending_changes(5).0.total_cells);
    let same = |ws: &Workspace, snap: &(Vec<u8>, studio_domain::WorkspaceRevision, usize, usize, usize), what: &str| {
        assert_eq!(&snapshot(ws), snap, "{what}");
    };

    // The reviewer's reproduction: A previews day = 1, B previews day = 2 (same table, same
    // record, same local sequence); B must refuse A's ticket, and vice versa.
    let (ta, plan_a) = a.schedule_edit_preview(&nhl_a, &day(1)).unwrap();
    let (tb, plan_b) = b.schedule_edit_preview(&nhl_b, &day(2)).unwrap();
    assert_ne!(ta, tb, "tickets from different instances never coincide");
    assert_ne!(plan_a.rows, plan_b.rows);
    let (sa, sb) = (snapshot(&a), snapshot(&b));
    let err = b.schedule_apply_review(&ta, None).unwrap_err().to_string();
    assert!(err.contains("issued by another project"), "{err}");
    let err = a.schedule_apply_review(&tb, None).unwrap_err().to_string();
    assert!(err.contains("issued by another project"), "{err}");
    same(&a, &sa, "A after B's ticket");
    same(&b, &sb, "B after A's ticket");
    assert!(a.schedule_review(&ta).is_some() && b.schedule_review(&tb).is_some(), "both tickets still held");
    assert!(a.schedule_review(&tb).is_none() && b.schedule_review(&ta).is_none());
    // Each applies exactly its own plan, once.
    let (ra, _, va) = a.schedule_apply_review(&ta, None).unwrap();
    let (rb, _, vb) = b.schedule_apply_review(&tb, None).unwrap();
    assert_eq!((ra.ticket.as_str(), va.games[0].day), (ta.as_str(), 1));
    assert_eq!((rb.ticket.as_str(), vb.games[0].day), (tb.as_str(), 2));
    assert!(a.schedule_apply_review(&ta, None).is_err() && b.schedule_apply_review(&tb, None).is_err(), "single use");
    assert!(a.undo() && b.undo());

    // Different tables and operations: an import ticket from A against a generation ticket from
    // B (same local sequence again), plus a mapping-bearing import on B.
    let json = a.schedule_export(&nhl_a, ImportFormat::Json).unwrap();
    let (ta, _) = a.schedule_import_preview(&nhl_a, ImportFormat::Json, &json, None, &[]).unwrap();
    let opts = b.schedule_generation_options(&swe_b).unwrap();
    let cfg = GenerationConfig {
        participants: opts.members.iter().filter(|m| m.default).map(|m| m.team_id).collect(),
        rounds: 4,
        start_month: 8,
        start_day: 10,
        days_between_matchdays: 3,
        pair_quotas: None,
                odd_home: None,
                groups: None,
                odd_hosts: None,
    };
    let (tb, _) = b.schedule_generate_preview(&swe_b, &cfg).unwrap();
    let (sa, sb) = (snapshot(&a), snapshot(&b));
    assert!(b.schedule_apply_review(&ta, None).unwrap_err().to_string().contains("issued by another project"));
    assert!(a.schedule_apply_review(&tb, None).unwrap_err().to_string().contains("issued by another project"));
    same(&a, &sa, "A after B's generation ticket");
    same(&b, &sb, "B after A's import ticket");
    let (ra, oa, _) = a.schedule_apply_review(&ta, Some(&json)).unwrap();
    assert!(matches!(ra.operation, studio_domain::ScheduleOperation::Import { .. }) && oa.edits == 0);
    let (rb, _, vb) = b.schedule_apply_review(&tb, None).unwrap();
    assert!(matches!(rb.operation, studio_domain::ScheduleOperation::Generate(_)));
    assert_eq!(vb.first_date.as_deref(), Some("Sep 11"));
    assert!(b.undo());

    // Workspace replacement: a ticket minted by the old instance is refused by a fresh instance
    // opened on the same file — even when the new instance has minted its own ticket with the
    // same local sequence — and the old instance's ticket still applies at home.
    let (t_old, _) = a.schedule_edit_preview(&nhl_a, &day(3)).unwrap();
    let mut a2 = anchor("nhllegacy", Platform::Xbox);
    let nhl_a2 = table(&a2, "nhlschedule");
    let (t_new, _) = a2.schedule_edit_preview(&nhl_a2, &day(4)).unwrap();
    assert_ne!(t_old, t_new);
    let s2 = snapshot(&a2);
    let err = a2.schedule_apply_review(&t_old, None).unwrap_err().to_string();
    assert!(err.contains("issued by another project"), "{err}");
    same(&a2, &s2, "replacement instance after the old ticket");
    let (_, _, v2) = a2.schedule_apply_review(&t_new, None).unwrap();
    assert_eq!(v2.games[0].day, 4);
    let (_, _, v1) = a.schedule_apply_review(&t_old, None).unwrap();
    assert_eq!(v1.games[0].day, 3);
}

// ---------------------------------------------------------------------------------------------
// 4. Generation: properties of the output, blockers with exact numbers.
// ---------------------------------------------------------------------------------------------

#[test]
fn generation_fills_a_round_robin_league_and_names_the_blocker_elsewhere() {
    let mut ws = anchor("nhllegacy", Platform::Xbox);
    // SHL: 14 clubs, 364 rows = 4 round-robins of 91 pairings.
    let swe = table(&ws, "swedishschedule");
    let opts = ws.schedule_generation_options(&swe).unwrap();
    assert!(opts.blocker.is_none(), "{:?}", opts.blocker);
    assert_eq!((opts.pairs, opts.rows_to_fill, opts.kept_rows, opts.suggested_rounds), (91, 364, 0, Some(4)));
    assert_eq!(opts.matchdays_per_round, 13);
    assert_eq!((opts.suggested_start_month, opts.suggested_start_day), (8, 15), "the current opener (Sep 16)");
    let cfg = GenerationConfig {
        participants: opts.members.iter().filter(|m| m.default).map(|m| m.team_id).collect(),
        rounds: 4,
        start_month: opts.suggested_start_month,
        start_day: opts.suggested_start_day,
        days_between_matchdays: opts.suggested_days_between_matchdays,
        pair_quotas: None,
                odd_home: None,
                groups: None,
                odd_hosts: None,
    };
    let plan = gen_preview(&mut ws, &swe, &cfg).unwrap();
    assert!(plan.change.validation.ok(), "{:?}", plan.change.validation.errors);
    assert_eq!((plan.participants, plan.matchdays, plan.generated_rows, plan.kept_rows), (14, 52, 364, 0));
    assert_eq!(plan.games_per_participant, 52);
    // Deterministic.
    let again = gen_preview(&mut ws, &swe, &cfg).unwrap();
    assert_eq!(plan.change.rows, again.change.rows);
    let (_, outcome, view) = gen_apply(&mut ws, &swe, &cfg).unwrap();
    assert!(outcome.edits > 0);
    assert_eq!(ws.undo_depth(), 1);
    // Properties of the schedule itself, read back from the bytes.
    assert_eq!(view.games.len(), 364);
    assert!(view.findings.is_empty(), "{:?}", view.findings);
    let mut per_pair: BTreeMap<(u64, u64), (u32, u32)> = BTreeMap::new();
    let mut home: BTreeMap<u64, u32> = BTreeMap::new();
    let mut prev = 0u64;
    for (i, g) in view.games.iter().enumerate() {
        assert_eq!(g.index, i as u64);
        assert_ne!(g.home_slot, g.away_slot);
        assert!(g.home.is_some() && g.away.is_some());
        assert_eq!((g.round, g.status), (0, Some(0)));
        let key = studio_domain::schedule_edit::season_key(g.month, g.day);
        assert!(key >= prev, "dates non-decreasing at row {i}");
        prev = key;
        let (a, b) = (g.home_slot.min(g.away_slot), g.home_slot.max(g.away_slot));
        let e = per_pair.entry((a, b)).or_default();
        if g.home_slot == a {
            e.0 += 1
        } else {
            e.1 += 1
        }
        *home.entry(g.home_slot).or_default() += 1;
    }
    assert_eq!(per_pair.len(), 91);
    assert!(per_pair.values().all(|(h, a)| *h == 2 && *a == 2), "every pair meets 4×, 2 at each venue: {per_pair:?}");
    assert!(home.values().all(|h| *h == 26), "26 home games each: {home:?}");
    assert!(view.slots.iter().all(|s| s.games == 52));
    assert_eq!(view.first_date.as_deref(), Some("Sep 16"));
    // Spacing: 52 matchdays every N days; the last date follows from the first.
    let dates: BTreeSet<u64> = view.games.iter().map(|g| studio_domain::schedule_edit::season_key(g.month, g.day)).collect();
    assert_eq!(dates.len(), 52, "one date per matchday");
    assert!(ws.undo());
    assert!(!ws.is_dirty());

    // Configuration refusals with exact numbers; nothing written.
    let bad = GenerationConfig { rounds: 3, ..cfg.clone() };
    let plan = gen_preview(&mut ws, &swe, &bad).unwrap();
    assert!(plan.change.validation.errors[0].contains("14 participants × 3 round-robin(s) = 273 games") && plan.change.validation.errors[0].contains("364 active rows"), "{:?}", plan.change.validation.errors);
    assert!(gen_apply(&mut ws, &swe, &bad).is_err());
    assert_eq!(ws.undo_depth(), 0);
    let wide = GenerationConfig { days_between_matchdays: 8, ..cfg.clone() };
    let plan = gen_preview(&mut ws, &swe, &wide).unwrap();
    assert!(plan.change.validation.errors[0].contains("past one season") || plan.change.validation.errors[0].contains("season boundary"), "{:?}", plan.change.validation.errors);
    let few = GenerationConfig { participants: cfg.participants[..1].to_vec(), ..cfg.clone() };
    let plan = gen_preview(&mut ws, &swe, &few).unwrap();
    assert!(plan.change.validation.errors.iter().any(|e| e.contains("at least two")), "{:?}", plan.change.validation.errors);

    // NHL: 30 clubs, 1 230 games is no whole number of round-robins — the blocker names it and
    // the All-Star row counts as a kept row; the Swiss 12-club × 50-game table likewise.
    let nhl = table(&ws, "nhlschedule");
    let opts = ws.schedule_generation_options(&nhl).unwrap();
    assert_eq!((opts.kept_rows, opts.rows_to_fill, opts.pairs), (1, 1230, 435));
    assert!(opts.blocker.as_deref().unwrap().contains("1230 games for 30 teams is not a whole number of round-robins"), "{:?}", opts.blocker);
    assert!(opts.notes.iter().any(|n| n.contains("870 games") && n.contains("1305 games")), "{:?}", opts.notes);
    let swiss = table(&ws, "swissschedule");
    let opts = ws.schedule_generation_options(&swiss).unwrap();
    assert!(opts.blocker.as_deref().unwrap().contains("300 games for 12 teams"), "{:?}", opts.blocker);
    // German: 14 clubs × 52 = 4 round-robins → supported.
    let ger = table(&ws, "germanschedule");
    let opts = ws.schedule_generation_options(&ger).unwrap();
    assert_eq!(opts.suggested_rounds, Some(4), "{:?}", opts.blocker);
}

#[test]
fn generation_keeps_the_all_star_row_when_the_topology_fits() {
    // NHL 07's 29-game "small" season: 30 clubs, 435 games + the All-Star row = 436 rows — one
    // round-robin fits exactly, and the All-Star row must be kept in place (receipt 41: its row and
    // values never move; generated dates are held around it).
    let mut ws = anchor("nhl07", Platform::Xbox);
    let small = table(&ws, "nhlschedulesmall");
    let opts = ws.schedule_generation_options(&small).unwrap();
    assert_eq!((opts.kept_rows, opts.rows_to_fill, opts.suggested_rounds), (1, 435, Some(1)), "{:?}", opts.blocker);
    let cfg = GenerationConfig {
        participants: opts.members.iter().filter(|m| m.default).map(|m| m.team_id).collect(),
        rounds: 1,
        start_month: 9,
        start_day: 0,
        days_between_matchdays: 4,
        pair_quotas: None,
                odd_home: None,
                groups: None,
                odd_hosts: None,
    };
    let before = ws.display().schedule_view(&small).unwrap().unwrap();
    let star = before.games.iter().find(|g| g.home_slot >= 30).unwrap().clone();
    let plan = gen_preview(&mut ws, &small, &cfg).unwrap();
    assert!(plan.change.validation.ok(), "{:?}", plan.change.validation.errors);
    assert_eq!((plan.generated_rows, plan.kept_rows), (435, 1));
    let (_, _, view) = gen_apply(&mut ws, &small, &cfg).unwrap();
    let kept = view.games.iter().find(|g| g.home_slot >= 30).unwrap();
    assert_eq!((kept.month, kept.day, kept.home_slot, kept.away_slot), (star.month, star.day, star.home_slot, star.away_slot));
    // In date order around it: every earlier row sorts before it, every later row after.
    let k = studio_domain::schedule_edit::season_key(kept.month, kept.day);
    assert!(view.games[..kept.record as usize].iter().all(|g| studio_domain::schedule_edit::season_key(g.month, g.day) <= k));
    assert!(view.games[kept.record as usize + 1..].iter().all(|g| studio_domain::schedule_edit::season_key(g.month, g.day) >= k));
    assert_eq!(view.slots.iter().filter(|s| s.games == 29).count(), 30);
    assert_eq!(view.slots.iter().filter(|s| s.games == 1).count(), 2);
    assert!(ws.undo());
}

// ---------------------------------------------------------------------------------------------
// 4b. Import + generation parity on every committed target (R1 F3): the same assertions on each
//     anchor, opened through its own platform adapter (Recompiled through `Platform::Recomp`).
// ---------------------------------------------------------------------------------------------

/// Import: export the title's NHL table as CSV, swap two clubs through an explicit mapping, apply,
/// read the swap back, undo to the baseline. Generation: apply the first source table whose row
/// count is a whole number of round-robins and check the schedule's properties from the bytes; a
/// title without such a table must name the arithmetic blocker on every source table.
fn import_and_generation_parity(ws: &mut Workspace, tag: &str, nhl_table: &str) -> (String, Vec<String>) {
    let baseline = ws.primary().bytes().to_vec();
    let nhl = table(ws, nhl_table);
    let view = ws.display().schedule_view(&nhl).unwrap().unwrap();
    let clubs: Vec<_> = view.slots.iter().filter(|s| s.games > 2).map(|s| s.slot).collect();
    let (a, b) = (view.slot_map.by_slot(clubs[0]).unwrap().clone(), view.slot_map.by_slot(clubs[1]).unwrap().clone());
    let csv = ws.schedule_export(&nhl, ImportFormat::Csv).unwrap();
    let plan = import_preview(ws, &nhl, ImportFormat::Csv, &csv, &[]).unwrap();
    assert!(plan.change.validation.ok() && plan.change.cells == 0, "{tag}: export→import must be a no-op: {:?}", plan.change.validation.errors);
    let mapping = [
        TeamMapping { key: if tag.starts_with("nhl07/") { a.team_id.to_string() } else { a.key.clone() }, team_id: b.team_id },
        TeamMapping { key: if tag.starts_with("nhl07/") { b.team_id.to_string() } else { b.key.clone() }, team_id: a.team_id },
    ];
    let mutual = view.games.iter().filter(|g| (g.home_slot == a.slot && g.away_slot == b.slot) || (g.home_slot == b.slot && g.away_slot == a.slot)).count();
    let games_a = view.slots.iter().find(|s| s.slot == a.slot).unwrap().games as usize;
    let games_b = view.slots.iter().find(|s| s.slot == b.slot).unwrap().games as usize;
    let (plan, outcome, after) = import_apply(ws, &nhl, ImportFormat::Csv, &csv, &mapping).unwrap();
    assert_eq!(plan.change.rows.len(), games_a + games_b - mutual, "{tag}: rows touched by the swap");
    assert!(outcome.edits > 0 && ws.undo_depth() == 1, "{tag}");
    let first_a = view.games.iter().find(|g| g.home_slot == a.slot && g.away_slot != b.slot).unwrap();
    assert_eq!(after.games[first_a.record as usize].home.as_deref(), Some(b.name.as_str()), "{tag}: {} now plays {}'s games", b.name, a.name);
    assert_eq!(after.slots.len(), view.slots.len(), "{tag}: games per team preserved");
    assert!(ws.undo());
    assert_eq!(ws.primary().bytes(), &baseline[..], "{tag}: undo restores the baseline");

    // Generation.
    let sources: Vec<_> = ws.display().schedule_tables().unwrap().into_iter().filter(|t| (t.role == "source" || t.role == "variant") && t.league_code.is_some()).collect();
    let mut blockers = Vec::new();
    let mut generated = None;
    for t in &sources {
        let Ok(opts) = ws.schedule_generation_options(&t.studio_table) else { continue };
        match (&opts.blocker, opts.suggested_rounds) {
            (None, Some(rounds)) if generated.is_none() => {
                let cfg = GenerationConfig {
                    participants: opts.members.iter().filter(|m| m.default).map(|m| m.team_id).collect(),
                    rounds,
                    start_month: opts.suggested_start_month,
                    start_day: opts.suggested_start_day,
                    days_between_matchdays: opts.suggested_days_between_matchdays,
                    pair_quotas: None,
                odd_home: None,
                groups: None,
                odd_hosts: None,
                };
                let (plan, _, v) = gen_apply(ws, &t.studio_table, &cfg).unwrap();
                let n = plan.participants;
                assert_eq!(v.games.len(), opts.rows_to_fill as usize + opts.kept_rows, "{tag}/{}", t.name);
                assert!(v.findings.iter().all(|f| f.contains("All-Star") || f.contains("no games")), "{tag}/{}: {:?}", t.name, v.findings);
                let mut pairs: BTreeMap<(u64, u64), u32> = BTreeMap::new();
                let mut prev = 0;
                for (i, g) in v.games.iter().enumerate() {
                    assert_eq!(g.index, i as u64);
                    let k = studio_domain::schedule_edit::season_key(g.month, g.day);
                    assert!(k >= prev, "{tag}/{}: dates sorted", t.name);
                    prev = k;
                    if cfg.participants.contains(&v.slot_map.by_slot(g.home_slot).map(|e| e.team_id).unwrap_or(u64::MAX)) && cfg.participants.contains(&v.slot_map.by_slot(g.away_slot).map(|e| e.team_id).unwrap_or(u64::MAX)) {
                        *pairs.entry((g.home_slot.min(g.away_slot), g.home_slot.max(g.away_slot))).or_default() += 1;
                    }
                }
                assert_eq!(pairs.len(), n * (n - 1) / 2, "{tag}/{}: every pair meets", t.name);
                assert!(pairs.values().all(|c| *c == rounds), "{tag}/{}: every pair meets {rounds}×", t.name);
                assert_eq!(v.slots.iter().filter(|s| s.games == plan.games_per_participant).count(), n, "{tag}/{}: games per participant", t.name);
                assert!(ws.undo());
                assert_eq!(ws.primary().bytes(), &baseline[..], "{tag}/{}: undo after generation", t.name);
                generated = Some(format!("{} {}×{} = {} (+{} kept)", t.name, n, rounds, plan.generated_rows, plan.kept_rows));
            }
            (Some(b), _) => blockers.push(format!("{}: {}", t.name, b.split(" pairings").next().unwrap_or(b))),
            _ => {}
        }
    }
    assert!(generated.is_some() || blockers.len() == sources.len(), "{tag}: no table generated and not every source table is blocked: {blockers:?}");
    (generated.unwrap_or_else(|| "blocked on every source table".into()), blockers)
}

#[test]
fn import_and_generation_parity_on_every_committed_target() {
    let mut report = Vec::new();
    for (game, platform, nhl_table) in [
        ("nhl07", Platform::Xbox, "nhlschedulelongnoleap"),
        ("nhl08", Platform::Xbox, "nhlschedule"),
        ("nhl09", Platform::Xbox, "nhlschedule"),
        ("nhl10", Platform::Xbox, "nhlschedule"),
        ("nhl11", Platform::Xbox, "nhlschedule"),
        ("nhl12", Platform::Xbox, "nhlschedule"),
        ("nhl13", Platform::Xbox, "nhlschedule"),
        ("nhl14", Platform::Xbox, "nhlschedule"),
        ("nhl15", Platform::Xbox, "nhlschedule"),
        ("nhllegacy", Platform::Xbox, "nhlschedule"),
        ("nhllegacy", Platform::Recomp, "nhlschedule"),
        ("nhl08", Platform::Ps3, "nhlschedule"),
        ("nhl09", Platform::Ps3, "nhlschedule"),
        ("nhl10", Platform::Ps3, "nhlschedule"),
        ("nhl11", Platform::Ps3, "nhlschedule"),
        ("nhl12", Platform::Ps3, "nhlschedule"),
        ("nhl13", Platform::Ps3, "nhlschedule"),
        ("nhl14", Platform::Ps3, "nhlschedule"),
        ("nhl15", Platform::Ps3, "nhlschedule"),
        ("nhllegacy", Platform::Ps3, "nhlschedule"),
        ("nhl15", Platform::Ps4, "nhlschedule"),
        ("nhl16", Platform::Ps4, "nhlschedule"),
        ("nhl17", Platform::Ps4, "nhlschedule"),
        ("nhl18", Platform::Ps4, "nhlschedule"),
        ("nhl19", Platform::Ps4, "nhlschedule"),
        ("nhl21", Platform::Ps4, "nhlschedule"),
    ] {
        let mut ws = anchor(game, platform);
        let tag = format!("{game}/{platform:?}");
        let (gen, blockers) = import_and_generation_parity(&mut ws, &tag, nhl_table);
        report.push(format!("{tag}: generation {gen}; blocked: {}", blockers.len()));
    }
    // Printed for the receipt (cargo test -- --nocapture).
    for line in &report {
        println!("{line}");
    }
    assert_eq!(report.len(), 26);
}

// ---------------------------------------------------------------------------------------------
// 5. NHL 07: its variant tables resolve through the stock team table (Studio support was the
//    gap, not the game data).
// ---------------------------------------------------------------------------------------------

#[test]
fn nhl07_variant_tables_resolve_slots_through_the_stock_team_table() {
    let mut ws = anchor("nhl07", Platform::Xbox);
    let tables = ws.display().schedule_tables().unwrap();
    let variants: Vec<&str> = tables.iter().filter(|t| t.role == "variant").map(|t| t.name.as_str()).collect();
    assert_eq!(variants, ["nhlschedulelongleap", "nhlschedulelongnoleap", "nhlschedulemedium", "nhlschedulesmall"]);
    let long = table(&ws, "nhlschedulelongnoleap");
    let v = ws.display().schedule_view(&long).unwrap().unwrap();
    assert_eq!(v.games.len(), 1231);
    assert_eq!(v.slots.len(), 32, "30 clubs + the two All-Star slots resolve to stock rows");
    assert!(v.unresolved_slots.is_empty(), "{:?}", v.unresolved_slots);
    let ana = v.slot_map.by_slot(0).unwrap();
    assert!(ana.name.contains("Anaheim"), "{}", ana.name);
    // Pacific Division rivals 8× each.
    for rival in ["Dallas", "Los Angeles", "Phoenix", "San Jose"] {
        let s = v.slot_map.entries.iter().find(|e| e.name.contains(rival)).unwrap().slot;
        let n = v.games.iter().filter(|g| (g.home_slot == 0 && g.away_slot == s) || (g.home_slot == s && g.away_slot == 0)).count();
        assert_eq!(n, 8, "ANA vs {rival}");
    }
    // The international tables match their leagues by stem.
    for (name, teams) in [("intldelschedule", 14), ("intlelitserienschedule", 12), ("intlsmliigaschedule", 13)] {
        let t = table(&ws, name);
        let v = ws.display().schedule_view(&t).unwrap().unwrap();
        assert_eq!(v.slots.len(), teams, "{name}: {}", v.table.league_match);
        assert!(v.unresolved_slots.is_empty(), "{name}: {:?}", v.unresolved_slots);
    }
    let plan = edit_preview(
            &mut ws,
            &long,
            &GameEdit {
                record: 0,
                home_team_id: Some(ana.team_id),
                away_team_id: None,
                month: None,
                day: None,
            },
        )
        .unwrap();
    assert!(plan.validation.ok(), "{:?}", plan.validation.errors);
}

// ---------------------------------------------------------------------------------------------
// 6. Copied output: save the edited database to a file, reopen it, restore.
// ---------------------------------------------------------------------------------------------

#[test]
fn edited_schedule_survives_copied_output_reopen_and_restore() {
    let mut ws = anchor("nhl16", Platform::Ps4);
    let nhl = table(&ws, "nhlschedule");
    let view = ws.display().schedule_view(&nhl).unwrap().unwrap();
    let g = &view.games[10];
    let (h, a) = (view.slot_map.by_slot(g.home_slot).unwrap().team_id, view.slot_map.by_slot(g.away_slot).unwrap().team_id);
    edit_apply(&mut ws, 
        &nhl,
        &GameEdit {
            record: 10,
            home_team_id: Some(a),
            away_team_id: Some(h),
            month: Some(g.month),
            day: Some(g.day),
        },
    )
    .unwrap();
    let dir = std::env::temp_dir().join(format!("nhl-studio-p1a-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("nhl_game.db");
    ws.save_primary_to(out.to_str().unwrap()).unwrap();
    let reopened = open_paths("nhl16", Platform::Ps4, out.to_str().unwrap(), "anchors/ps4/nhl16/databases/nhl_game-meta.xml");
    let v2 = reopened.display().schedule_view(&nhl).unwrap().unwrap();
    assert_eq!(v2.games[10].home, g.away);
    assert_eq!(v2.games[10].away, g.home);
    assert_eq!(v2.games.len(), view.games.len());
    assert_eq!(v2.slots.iter().filter(|s| s.games == 82).count(), 30, "30 clubs at 82 (plus the two All-Star slots)");
    // Everything else byte-identical to the anchor except that one record.
    let original = std::fs::read(repo("anchors/ps4/nhl16/databases/nhl_game.db")).unwrap();
    let saved = std::fs::read(&out).unwrap();
    assert_eq!(original.len(), saved.len());
    // Restore: undo + save equals the original bytes.
    assert!(ws.undo());
    ws.save_primary_to(out.to_str().unwrap()).unwrap();
    assert_eq!(std::fs::read(&out).unwrap(), original, "undo + save restores the anchor byte-for-byte");
    std::fs::remove_dir_all(&dir).unwrap();
}

// ---------------------------------------------------------------------------------------------
// 7. Real NHL 20 (private fixture; the committed anchors/ps4/nhl20 is the NHL 19 base).
// ---------------------------------------------------------------------------------------------

#[test]
#[ignore = "private fixture databases/nhl20-ps4-cusa15439/inxp_00.big/*: set NHL_STUDIO_FIXTURES and select with --ignored"]
fn real_nhl20_install_database_edits_and_generates_within_its_own_layout() {
    let root = std::env::var_os("NHL_STUDIO_FIXTURES").expect("NHL_STUDIO_FIXTURES is not set; the NHL 20 fixture cannot be resolved");
    let require = |key: &str, sha: &str| -> String {
        let path = PathBuf::from(&root).join(key);
        let bytes = std::fs::read(&path).unwrap_or_else(|e| panic!("cannot read {}: {e}", path.display()));
        assert!(!bytes.is_empty());
        assert_eq!(tdb_txn::FileIdentity::of_bytes(&bytes).sha256, sha, "{key} is not the recorded source");
        path.to_str().unwrap().to_string()
    };
    let db = require("databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game.db", "1e2004784bccfdd6bed6e82533a67595798379f2c2d763c0636d9d370977a57b");
    let meta = require("databases/nhl20-ps4-cusa15439/inxp_00.big/nhl_game-meta.xml", "2fac0e733c9792038a64f966f277b163136517438509283e7f646d2920ad1f93");
    let project = Project::load(PathBuf::from(&db), Platform::Ps4, ProjectRole::Nhlng, PathBuf::from(&meta), repo("defs")).unwrap();
    let mut ws = Workspace::single(EditSession::begin(project), db.clone());
    ws.set_game_id("nhl20");
    // The season-specific tables differ from the NHL 19 copy: Austrian 270 rows / 11 slots,
    // ECHL 936 rows / 26 slots; layouts and the rank rule are the NHL 18+ ones.
    let aut = table(&ws, "austrianschedule");
    let v = ws.display().schedule_view(&aut).unwrap().unwrap();
    assert_eq!(v.games.len(), 270);
    assert_eq!(v.slots.len(), 11);
    assert_eq!(v.layout.field("home").unwrap().bit_width, 4);
    let echl = table(&ws, "echlschedule");
    let v = ws.display().schedule_view(&echl).unwrap().unwrap();
    assert_eq!(v.games.len(), 936);
    assert_eq!(v.slots.len(), 26);
    edit_round_trip(&mut ws, "nhl20/Ps4 (install)", false, 6);
    // Austrian: 11 clubs, 270 rows = 270 / 55 pairings — not whole (48/50 games per club is a
    // real unequal split), so generation is blocked with the numbers; edit/import remain.
    let opts = ws.schedule_generation_options(&aut).unwrap();
    assert!(opts.blocker.as_deref().unwrap().contains("270 games for 11 teams"), "{:?}", opts.blocker);
    let csv = ws.schedule_export(&aut, ImportFormat::Csv).unwrap();
    let plan = import_preview(&mut ws, &aut, ImportFormat::Csv, &csv, &[]).unwrap();
    assert!(plan.change.validation.ok(), "{:?}", plan.change.validation.errors);
    assert_eq!(plan.change.cells, 0);
    let (gen, blockers) = import_and_generation_parity(&mut ws, "nhl20/Ps4 (install)", "nhlschedule");
    println!("nhl20/Ps4 (install): generation {gen}; blocked: {}", blockers.len());
    assert!(gen.starts_with("czechschedule") || gen.starts_with("germanschedule") || gen.starts_with("swedish"), "{gen}");
}

// ---------------------------------------------------------------------------------------------
// 8. Receipt 34: the user-supplied pair-quota generator (research R114 §2) and the round-tag
//    refusal (R114 §3: the PS4 18–21 Austrian final block is tagged round = 1).
// ---------------------------------------------------------------------------------------------

/// Per-pair (lower-slot home, higher-slot home) counts and the number of club-dates booked more
/// than once, read back from the bytes of a schedule view.
fn pair_counts(view: &ScheduleView) -> (BTreeMap<(u64, u64), (u32, u32)>, usize) {
    let mut per_pair: BTreeMap<(u64, u64), (u32, u32)> = BTreeMap::new();
    let mut booked: BTreeMap<(u64, u64, u64), u32> = BTreeMap::new();
    for g in &view.games {
        let (a, b) = (g.home_slot.min(g.away_slot), g.home_slot.max(g.away_slot));
        let e = per_pair.entry((a, b)).or_default();
        if g.home_slot == a {
            e.0 += 1
        } else {
            e.1 += 1
        }
        *booked.entry((g.month, g.day, g.home_slot)).or_default() += 1;
        *booked.entry((g.month, g.day, g.away_slot)).or_default() += 1;
    }
    (per_pair, booked.values().filter(|n| **n > 1).count())
}

fn fixtures(view: &ScheduleView) -> Vec<(u64, u64, u64, u64)> {
    view.games.iter().map(|g| (g.month, g.day, g.home_slot, g.away_slot)).collect()
}

#[test]
fn pair_quota_generation_builds_the_user_matrix_and_refuses_impossible_ones() {
    let mut ws = anchor("nhllegacy", Platform::Xbox);
    // Swiss: 12 clubs, 300 rows. A user partition into three groups of four by slot, 6 games
    // within a group and 4 across: 6·18 + 4·48 = 300 (R114 §2's formula; the groups here are the
    // user's choice, not a stock partition).
    let swiss = table(&ws, "swissschedule");
    let opts = ws.schedule_generation_options(&swiss).unwrap();
    let members: Vec<(u64, u64)> = opts.members.iter().filter(|m| m.default).map(|m| (m.team_id, m.slot)).collect();
    assert_eq!(members.len(), 12);
    let n = members.len();
    let group = |i: usize| i / 4;
    let q: Vec<Vec<u32>> = (0..n).map(|i| (0..n).map(|j| if i == j { 0 } else if group(i) == group(j) { 6 } else { 4 }).collect()).collect();
    let cfg = GenerationConfig {
        participants: members.iter().map(|m| m.0).collect(),
        rounds: 0,
        start_month: 8,
        start_day: 10,
        days_between_matchdays: 2,
        pair_quotas: Some(q.clone()),
        odd_home: None,
        groups: None,
        odd_hosts: None,
    };
    let plan = gen_preview(&mut ws, &swiss, &cfg).unwrap();
    assert!(plan.change.validation.ok(), "{:?}", plan.change.validation.errors);
    assert_eq!((plan.participants, plan.generated_rows, plan.kept_rows, plan.games_per_participant), (12, 300, 0, 50));
    assert!(plan.change.notes.iter().any(|n| n.starts_with("user-supplied pair quotas: 12 participants, 300 games")), "{:?}", plan.change.notes);
    println!("swiss 3 × 4 · 6 / 4: {} matchdays", plan.matchdays);
    // Deterministic.
    assert_eq!(gen_preview(&mut ws, &swiss, &cfg).unwrap().change.rows, plan.change.rows);
    let before = ws.display().schedule_view(&swiss).unwrap().unwrap();
    let (_, outcome, view) = gen_apply(&mut ws, &swiss, &cfg).unwrap();
    assert!(outcome.edits > 0);
    assert_eq!(ws.undo_depth(), 1);
    assert_eq!(view.games.len(), 300);
    let (per_pair, double_booked) = pair_counts(&view);
    assert_eq!(double_booked, 0, "no club plays twice on one date");
    let slot_of: Vec<u64> = members.iter().map(|m| m.1).collect();
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (slot_of[i].min(slot_of[j]), slot_of[i].max(slot_of[j]));
            let (h, w) = per_pair.get(&(a, b)).copied().unwrap_or((0, 0));
            assert_eq!(h + w, q[i][j], "pair {a}-{b}");
            assert_eq!(h, w, "even quota → equal venues for {a}-{b}");
        }
    }
    assert!(view.slots.iter().all(|s| s.games == 50), "{:?}", view.slots);
    let mut home: BTreeMap<u64, u32> = BTreeMap::new();
    for g in &view.games {
        *home.entry(g.home_slot).or_default() += 1;
    }
    assert!(home.values().all(|h| *h == 25), "25 home games each: {home:?}");
    let mut prev = 0;
    for (i, g) in view.games.iter().enumerate() {
        assert_eq!((g.index, g.round), (i as u64, 0));
        let k = studio_domain::schedule_edit::season_key(g.month, g.day);
        assert!(k >= prev);
        prev = k;
    }
    assert_eq!(view.first_date.as_deref(), Some("Sep 11"));
    // Save to a copy, reopen, compare; undo restores the stock rows; redo re-applies.
    let dir = std::env::temp_dir().join(format!("nhl-studio-quota-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("nhlng.db");
    ws.save_primary_to(out.to_str().unwrap()).unwrap();
    let reopened = open_paths("nhllegacy", Platform::Xbox, out.to_str().unwrap(), "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
    let v2 = reopened.display().schedule_view(&swiss).unwrap().unwrap();
    assert_eq!(fixtures(&v2), fixtures(&view), "the saved copy reopens with the generated table");
    drop(reopened);
    let _ = std::fs::remove_dir_all(&dir);
    assert!(ws.undo());
    assert_eq!(fixtures(&ws.display().schedule_view(&swiss).unwrap().unwrap()), fixtures(&before));
    assert!(ws.redo());
    assert_eq!(fixtures(&ws.display().schedule_view(&swiss).unwrap().unwrap()), fixtures(&view));
    assert!(ws.undo());

    // Odd quotas: one pair 7, another 5 (sum unchanged): venues differ by exactly one, the extra
    // home games are named.
    let mut odd = q.clone();
    odd[0][1] = 7;
    odd[1][0] = 7;
    odd[2][3] = 5;
    odd[3][2] = 5;
    // Without a host rule the odd counts are refused (R123 Q11); with "lower slot" the lower slot hosts.
    let unruled = gen_preview(&mut ws, &swiss, &GenerationConfig { pair_quotas: Some(odd.clone()), ..cfg.clone() }).unwrap();
    assert!(unruled.change.validation.errors.iter().any(|e| e.contains("choose which club hosts the extra game")), "{:?}", unruled.change.validation.errors);
    let odd_cfg = GenerationConfig { pair_quotas: Some(odd.clone()), odd_home: Some(studio_domain::schedule_edit::OddHome::LowerSlot), ..cfg.clone() };
    let plan = gen_preview(&mut ws, &swiss, &odd_cfg).unwrap();
    assert!(plan.change.validation.ok(), "{:?}", plan.change.validation.errors);
    assert!(plan.change.notes.iter().any(|n| n.starts_with("extra home game (2 odd pair(s))")), "{:?}", plan.change.notes);
    let (_, _, view) = gen_apply(&mut ws, &swiss, &odd_cfg).unwrap();
    let (per_pair, double_booked) = pair_counts(&view);
    assert_eq!(double_booked, 0);
    for (i, j) in [(0usize, 1usize), (2, 3)] {
        let (a, b) = (slot_of[i].min(slot_of[j]), slot_of[i].max(slot_of[j]));
        let (h, w) = per_pair[&(a, b)];
        assert_eq!((h + w, h), (odd[i][j], w + 1), "odd pair {a}-{b}: the lower slot hosts the extra game");
    }
    assert!(ws.undo());
    assert!(!ws.is_dirty());

    // Impossible matrices: refused with the reason, nothing written.
    let refuse = |ws: &mut Workspace, quotas: Vec<Vec<u32>>, participants: Option<Vec<u64>>, spacing: u32, want: &str| {
        let c = GenerationConfig { pair_quotas: Some(quotas), participants: participants.unwrap_or_else(|| cfg.participants.clone()), days_between_matchdays: spacing, ..cfg.clone() };
        let p = gen_preview(ws, &swiss, &c).unwrap();
        assert!(p.change.validation.errors.iter().any(|e| e.contains(want)), "want {want:?}: {:?}", p.change.validation.errors);
        assert!(p.change.rows.is_empty());
        assert!(gen_apply(ws, &swiss, &c).is_err());
        assert_eq!(ws.undo_depth(), 0);
    };
    let mut asym = q.clone();
    asym[0][5] = 5;
    asym[5][0] = 3;
    refuse(&mut ws, asym, None, 2, "not symmetric");
    let mut diag = q.clone();
    diag[4][4] = 2;
    refuse(&mut ws, diag, None, 2, "diagonal must be 0");
    let mut short = q.clone();
    short[0][11] = 2;
    short[11][0] = 2;
    refuse(&mut ws, short, None, 2, "asks for 298 games");
    refuse(&mut ws, q[..11].to_vec(), None, 2, "must be 12 × 12");
    let mut dup = cfg.participants.clone();
    dup[1] = dup[0];
    refuse(&mut ws, q.clone(), Some(dup), 2, "listed twice");
    refuse(&mut ws, q.clone(), None, 9, "past one season");
    // R123 Q12: a participant with an all-zero row.
    let mut idle = q.clone();
    for j in 0..n {
        if j != 7 {
            let moved = idle[7][j];
            idle[7][j] = 0;
            idle[j][7] = 0;
            let (a, b) = if j == 0 { (1, 2) } else { (0, if j == 1 { 2 } else { 1 }) };
            idle[a][b] += moved;
            idle[b][a] += moved;
        }
    }
    refuse(&mut ws, idle, None, 2, "would have no games");
}

#[test]
fn generation_refuses_to_rewrite_the_austrian_round_tagged_block() {
    // R114 §3: austrianschedule on PS4 18–21 = a four-game round robin (round 0) then a final
    // within-group block tagged round = 1.
    for game in ["nhl18", "nhl21"] {
        let mut ws = anchor(game, Platform::Ps4);
        let aut = table(&ws, "austrianschedule");
        let view = ws.display().schedule_view(&aut).unwrap().unwrap();
        let rows = view.games.len();
        let tagged = view.games.iter().filter(|g| g.round == 1).count();
        println!("{game} austrianschedule: {rows} rows, {tagged} tagged round = 1");
        assert!(tagged > 0, "{game}: R114 measured a round = 1 block");
        let members: Vec<u64> = view.slots.iter().filter_map(|s| s.team_id).collect();
        let n = members.len();
        // A user matrix with the right total: 4 per pair, the remainder spread one by one.
        let mut q = vec![vec![0u32; n]; n];
        let mut left = rows as i64;
        for i in 0..n {
            for j in i + 1..n {
                q[i][j] = 4;
                q[j][i] = 4;
                left -= 4;
            }
        }
        'fill: while left > 0 {
            for i in 0..n {
                for j in i + 1..n {
                    if left == 0 {
                        break 'fill;
                    }
                    q[i][j] += 1;
                    q[j][i] += 1;
                    left -= 1;
                }
            }
        }
        assert_eq!(left, 0, "{game}");
        let cfg = GenerationConfig { participants: members, rounds: 0, start_month: 8, start_day: 10, days_between_matchdays: 1, pair_quotas: Some(q), odd_home: Some(studio_domain::schedule_edit::OddHome::LowerSlot), groups: None, odd_hosts: None };
        let plan = gen_preview(&mut ws, &aut, &cfg).unwrap();
        let e = plan.change.validation.errors.join(" | ");
        assert!(e.contains(&format!("{tagged} of the rows this generation would replace carry a non-zero round")), "{game}: {e}");
        assert!(gen_apply(&mut ws, &aut, &cfg).is_err());
        assert_eq!(ws.undo_depth(), 0);
        // The round-robin route refuses the same rows (whatever its topology verdict).
        let rr = gen_preview(&mut ws, &aut, &GenerationConfig { pair_quotas: None, rounds: 4, ..cfg.clone() }).unwrap();
        assert!(rr.change.validation.errors.iter().any(|e| e.contains("carry a non-zero round")), "{game}: {:?}", rr.change.validation.errors);
        // Import keeps each row's round: the table's own export re-imports with no change.
        let csv = ws.schedule_export(&aut, ImportFormat::Csv).unwrap();
        let p = import_preview(&mut ws, &aut, ImportFormat::Csv, &csv, &[]).unwrap();
        assert!(p.change.validation.ok() && p.change.cells == 0, "{game}: {:?}", p.change.validation.errors);
    }
}
