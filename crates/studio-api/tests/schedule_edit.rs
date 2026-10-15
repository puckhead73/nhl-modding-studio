//! P1-A / R1 — schedule editing / import / export / generation through the Registry seam: every
//! preview is held under a review id bound to the project revision; `schedule_apply_review` writes
//! exactly the previewed plan or refuses (project moved, source file changed, foreign/consumed id,
//! plan with errors) leaving the project clean. Includes the reviewer's F1 reproduction (previewed
//! no-op import must not overwrite a subsequent edit) through the API.

use std::path::{Path, PathBuf};

use studio_api::dto::{ScheduleGameEditDto, ScheduleGenerationConfigDto, ScheduleTeamMappingDto};
use studio_api::{parse_platform, Registry};

fn repo(rel: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..").join(rel)
}
fn s(p: &Path) -> String {
    p.to_str().unwrap().to_string()
}
fn open(reg: &Registry, game: &str, platform: &str, db: &str, meta: &str) -> u32 {
    reg.open_workspace_from_paths(Some(game), parse_platform(platform).unwrap(), &s(&repo(db)), None, &s(&repo(meta)), &s(&repo("defs")))
        .unwrap()
        .project_id
}
fn err_of<T>(r: Result<T, studio_api::ApiError>) -> String {
    match r {
        Ok(_) => panic!("expected a refusal"),
        Err(e) => e.to_string(),
    }
}
fn day_edit(record: u32, day: u64) -> ScheduleGameEditDto {
    ScheduleGameEditDto {
        record,
        home_team_id: None,
        away_team_id: None,
        month: None,
        day: Some(day),
    }
}

#[test]
fn legacy_schedule_workflow_through_the_registry() {
    let reg = Registry::default();
    let id = open(&reg, "nhllegacy", "xbox360", "anchors/xbox360/nhllegacy/databases/nhlng.db", "anchors/xbox360/nhllegacy/databases/nhlng-meta.xml");
    let tables = reg.schedule_tables(id).unwrap();
    let nhl = tables.iter().find(|t| t.name == "nhlschedule").unwrap().studio_table.clone();
    let view = reg.schedule_view(id, &nhl).unwrap().unwrap();
    assert_eq!(view.slot_map.len(), 32);
    assert!(view.field_widths.iter().any(|(n, w)| n == "home" && *w == 5));
    let tor = view.slot_map.iter().find(|e| e.abbrev == "TOR").unwrap();
    let mtl = view.slot_map.iter().find(|e| e.abbrev == "MTL").unwrap();
    assert_eq!((tor.slot, mtl.slot, tor.key.as_str()), (27, 15, "TOR"));

    // Preview: nothing written, not dirty; the plan is held under a review id.
    let edit = ScheduleGameEditDto {
        record: 0,
        home_team_id: Some(mtl.team_id),
        away_team_id: Some(tor.team_id),
        month: None,
        day: None,
    };
    let plan = reg.schedule_edit_preview(id, &nhl, &edit).unwrap();
    assert!(plan.validation.errors.is_empty(), "{:?}", plan.validation.errors);
    assert!(plan.ticket.contains('/'), "{}", plan.ticket);
    assert_eq!((plan.rows_changed, plan.cells), (1, 2));
    assert_eq!(plan.after[0].home.as_deref(), view.games[0].away.as_deref());
    assert!(!reg.summary(id).unwrap().dirty);

    // Apply the review: dirty, one undo step; the pending review names the schedule row.
    let r = reg.schedule_apply_review(id, &plan.ticket).unwrap();
    assert_eq!((r.kind.as_str(), r.ticket.as_str(), r.outcome.edits), ("edit", plan.ticket.as_str(), 2));
    assert_eq!(r.view.games[0].home.as_deref(), view.games[0].away.as_deref());
    assert!(reg.summary(id).unwrap().dirty);
    let pending = reg.pending_changes(id, 20).unwrap();
    assert_eq!(pending.primary.total_cells, 2);
    assert!(pending.primary.cells.iter().all(|c| c.table_name == "nhlschedule" && c.record == 0));
    assert_eq!(pending.primary.cells[0].entity, "nhlschedule game 0 (Oct 7)");
    // Consumed: a second apply of the same review is refused.
    assert!(reg.schedule_apply_review(id, &plan.ticket).is_err());
    assert!(reg.undo(id).unwrap());
    assert!(!reg.summary(id).unwrap().dirty);

    // Export to a file, re-import from the file path: a no-op that validates clean.
    let dir = std::env::temp_dir().join(format!("nhl-studio-p1a-api-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let out = dir.join("nhlschedule.json");
    let exp = reg.schedule_export(id, &nhl, "json", Some(out.to_str().unwrap())).unwrap();
    assert_eq!((exp.format.as_str(), exp.suggested_filename.as_str()), ("json", "nhlschedule.json"));
    assert_eq!(std::fs::read_to_string(&out).unwrap(), exp.text);
    let plan = reg.schedule_import_preview(id, &nhl, "json", None, Some(out.to_str().unwrap()), &[]).unwrap();
    assert!(plan.change.validation.errors.is_empty(), "{:?}", plan.change.validation.errors);
    assert_eq!((plan.change.cells, plan.rows_in_file, plan.rows_in_table), (0, 1231, 1231));
    assert_eq!(plan.keys.len(), 32, "30 clubs + the two All-Star squads by key");
    assert!(plan.keys.iter().all(|k| k.how == "auto"));

    // F1 reproduction through the API: the previewed no-op import, applied after a later edit,
    // must be refused and must not restore game 0's old day.
    let noop_review = plan.change.ticket.clone();
    let e = reg.schedule_edit_preview(id, &nhl, &day_edit(0, 9)).unwrap();
    reg.schedule_apply_review(id, &e.ticket).unwrap();
    let before = reg.schedule_view(id, &nhl).unwrap().unwrap();
    assert_eq!(before.games[0].day, 9);
    let err = err_of(reg.schedule_apply_review(id, &noop_review));
    assert!(err.contains("changed since this preview") || err.contains("no longer held"), "{err}");
    let after = reg.schedule_view(id, &nhl).unwrap().unwrap();
    assert_eq!(after.games[0].day, 9, "the intervening edit survives");
    assert_eq!(reg.summary(id).unwrap().undo_depth, 1);
    assert!(reg.undo(id).unwrap());

    // The source file changes between preview and apply (same keys): refused by the byte hash.
    let plan = reg.schedule_import_preview(id, &nhl, "json", None, Some(out.to_str().unwrap()), &[]).unwrap();
    let tampered = exp.text.replacen("\"day\": 7", "\"day\": 8", 1);
    assert_ne!(tampered, exp.text);
    std::fs::write(&out, &tampered).unwrap();
    let err = err_of(reg.schedule_apply_review(id, &plan.change.ticket));
    assert!(err.contains("file changed since it was previewed"), "{err}");
    assert!(!reg.summary(id).unwrap().dirty);
    std::fs::write(&out, &exp.text).unwrap();
    let r = reg.schedule_apply_review(id, &plan.change.ticket).unwrap();
    assert_eq!((r.kind.as_str(), r.outcome.edits), ("import", 0));
    assert!(r.import.is_some());
    assert!(!reg.summary(id).unwrap().dirty, "a no-op import leaves no history");

    // An explicit mapping that swaps Toronto and Montréal changes every game of both.
    let mapping = vec![
        ScheduleTeamMappingDto {
            key: "TOR".into(),
            team_id: mtl.team_id,
        },
        ScheduleTeamMappingDto {
            key: "MTL".into(),
            team_id: tor.team_id,
        },
    ];
    let plan = reg.schedule_import_preview(id, &nhl, "json", None, Some(out.to_str().unwrap()), &mapping).unwrap();
    assert!(plan.change.validation.errors.is_empty(), "{:?}", plan.change.validation.errors);
    assert_eq!(plan.keys.iter().filter(|k| k.how == "explicit").count(), 2);
    assert_eq!(plan.change.rows_changed, 82 + 82 - 4, "their four mutual games count once");
    let r = reg.schedule_apply_review(id, &plan.change.ticket).unwrap();
    assert_eq!(r.view.games[0].home.as_deref(), Some(mtl.name.as_str()));
    assert!(r.view.findings.len() == 1 && r.view.findings[0].contains("All-Star"), "{:?}", r.view.findings);
    assert!(reg.undo(id).unwrap());

    // Refusals: a truncated file names the counts; a missing file is an I/O error; no state changes.
    let short = dir.join("short.csv");
    let csv = reg.schedule_export(id, &nhl, "csv", None).unwrap().text;
    std::fs::write(&short, csv.lines().take(50).map(|l| format!("{l}\n")).collect::<String>()).unwrap();
    let plan = reg.schedule_import_preview(id, &nhl, "csv", None, Some(short.to_str().unwrap()), &[]).unwrap();
    assert!(plan.change.validation.errors.iter().any(|e| e.contains("48 games") && e.contains("1231 active rows")), "{:?}", plan.change.validation.errors);
    assert!(err_of(reg.schedule_apply_review(id, &plan.change.ticket)).contains("not applied"));
    assert!(reg.schedule_import_preview(id, &nhl, "csv", None, Some(dir.join("missing.csv").to_str().unwrap()), &[]).is_err());
    let err = match reg.schedule_import_preview(id, &nhl, "csv", None, None, &[]) {
        Ok(_) => panic!("no text and no path must be refused"),
        Err(e) => e.to_string(),
    };
    assert!(err.contains("no schedule file"), "{err}");
    assert!(reg.schedule_export(id, &nhl, "xml", None).is_err());
    assert!(!reg.summary(id).unwrap().dirty);
    std::fs::remove_dir_all(&dir).unwrap();

    // R2: tickets minted concurrently by two projects (same local sequence) are refused by the
    // other project unmodified, leaving both projects untouched; each still applies its own once.
    let other = open(&reg, "nhllegacy", "ps3", "anchors/ps3/nhllegacy/databases/nhlng.db", "anchors/ps3/nhllegacy/databases/nhlng-meta.xml");
    let nhl_ps3 = reg.schedule_tables(other).unwrap().iter().find(|t| t.name == "nhlschedule").unwrap().studio_table.clone();
    let foreign = reg.schedule_edit_preview(other, &nhl_ps3, &day_edit(0, 8)).unwrap();
    let mine = reg.schedule_edit_preview(id, &nhl, &day_edit(0, 9)).unwrap();
    assert_ne!(foreign.ticket, mine.ticket);
    let err = err_of(reg.schedule_apply_review(id, &foreign.ticket));
    assert!(err.contains("issued by another project"), "{err}");
    let err = err_of(reg.schedule_apply_review(other, &mine.ticket));
    assert!(err.contains("issued by another project"), "{err}");
    assert!(!reg.summary(id).unwrap().dirty && !reg.summary(other).unwrap().dirty);
    let r = reg.schedule_apply_review(id, &mine.ticket).unwrap();
    assert_eq!(r.view.games[0].day, 9);
    let r = reg.schedule_apply_review(other, &foreign.ticket).unwrap();
    assert_eq!(r.view.games[0].day, 8);
    assert!(reg.schedule_apply_review(id, &mine.ticket).is_err() && reg.schedule_apply_review(other, &foreign.ticket).is_err(), "single use");
    assert!(reg.undo(id).unwrap() && reg.undo(other).unwrap());
    // A ticket from a closed-and-reopened project (a replacement instance) is refused there.
    let stale = reg.schedule_edit_preview(other, &nhl_ps3, &day_edit(0, 6)).unwrap();
    assert!(reg.close_project(other));
    let reopened = open(&reg, "nhllegacy", "ps3", "anchors/ps3/nhllegacy/databases/nhlng.db", "anchors/ps3/nhllegacy/databases/nhlng-meta.xml");
    let fresh = reg.schedule_edit_preview(reopened, &nhl_ps3, &day_edit(0, 5)).unwrap();
    assert_ne!(stale.ticket, fresh.ticket);
    let err = err_of(reg.schedule_apply_review(reopened, &stale.ticket));
    assert!(err.contains("issued by another project"), "{err}");
    assert!(!reg.summary(reopened).unwrap().dirty);
    let r = reg.schedule_apply_review(reopened, &fresh.ticket).unwrap();
    assert_eq!(r.view.games[0].day, 5);
    assert!(reg.undo(reopened).unwrap());

    // Generation: the NHL table is blocked with the numbers; the SHL table generates.
    let opts = reg.schedule_generation_options(id, &nhl).unwrap();
    assert!(opts.blocker.as_deref().unwrap().contains("not a whole number of round-robins"));
    let swe = tables.iter().find(|t| t.name == "swedishschedule").unwrap().studio_table.clone();
    let opts = reg.schedule_generation_options(id, &swe).unwrap();
    assert_eq!(opts.suggested_rounds, Some(4));
    let cfg = ScheduleGenerationConfigDto {
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
    let plan = reg.schedule_generate_preview(id, &swe, &cfg).unwrap();
    assert!(plan.change.validation.errors.is_empty(), "{:?}", plan.change.validation.errors);
    assert_eq!((plan.participants, plan.matchdays, plan.generated_rows), (14, 52, 364));
    // A second preview with a different configuration does not alter what the first one applies.
    let later = reg.schedule_generate_preview(id, &swe, &ScheduleGenerationConfigDto { start_day: 12, ..cfg }).unwrap();
    assert_ne!(later.change.ticket, plan.change.ticket);
    assert!(!reg.summary(id).unwrap().dirty);
    let r = reg.schedule_apply_review(id, &plan.change.ticket).unwrap();
    assert_eq!(r.kind, "generate");
    assert_eq!(r.view.first_date.as_deref(), Some("Sep 11"), "the first review's start date");
    assert!(r.view.slots.iter().all(|s| s.games == 52));
    assert!(reg.schedule_apply_review(id, &later.change.ticket).is_err(), "dropped by the apply");
    assert!(reg.undo(id).unwrap());
    assert!(!reg.summary(id).unwrap().dirty);
}

/// Receipt 48 (R123 accepted for single-phase groups): the shipped groups through the Registry by exact source
/// identity, a grouped preview, and the coordinator's `odd_home = "invalid-rule"` case refused before anything is
/// built (it used to read as "no rule").
#[test]
fn grouped_generation_and_host_rule_values_through_the_registry() {
    let reg = Registry::default();
    let id = open(&reg, "nhl11", "xbox", "anchors/xbox360/nhl11/databases/nhlng.db", "anchors/xbox360/nhl11/databases/nhlng-meta.xml");
    let swiss = reg.schedule_tables(id).unwrap().into_iter().find(|t| t.name == "swissschedule").unwrap().studio_table;
    let stock = reg.schedule_stock_groups(id, &swiss).unwrap();
    assert!(stock.available, "{:?}", stock.reason);
    assert_eq!(stock.group_names.iter().map(Vec::len).collect::<Vec<_>>(), [4, 4, 4]);
    let groups = stock.groups.clone().unwrap();
    let opts = reg.schedule_generation_options(id, &swiss).unwrap();
    let cfg = ScheduleGenerationConfigDto {
        participants: groups.groups.iter().flatten().copied().collect(),
        rounds: 0,
        start_month: opts.suggested_start_month,
        start_day: opts.suggested_start_day,
        days_between_matchdays: 1,
        pair_quotas: None,
        odd_home: None,
        groups: Some(groups.clone()),
        odd_hosts: None,
    };
    let plan = reg.schedule_generate_preview(id, &swiss, &cfg).unwrap();
    assert!(plan.change.validation.errors.is_empty(), "{:?}", plan.change.validation.errors);
    assert!(plan.change.notes.iter().any(|n| n.contains("the shipped swissschedule groups of xbox360-nhl11")), "{:?}", plan.change.notes);
    for bad in ["invalid-rule", "", "LOWER_SLOT", "home"] {
        let e = err_of(reg.schedule_generate_preview(id, &swiss, &ScheduleGenerationConfigDto { odd_home: Some(bad.into()), ..cfg.clone() }));
        assert!(e.contains("not a host rule Studio supports"), "{bad}: {e}");
    }
    // A table without recorded groups says why.
    let nhl = reg.schedule_tables(id).unwrap().into_iter().find(|t| t.name == "nhlschedule").unwrap().studio_table;
    let none = reg.schedule_stock_groups(id, &nhl).unwrap();
    assert!(!none.available && none.reason.unwrap().contains("no shipped single-phase groups"));
}
