//! P1-A — source schedule **editing, import/export and generation** on top of the browse/validate
//! layer in [`crate::schedules`] (research handoff P1.2-X and its 2026-09-18 corrections).
//!
//! Everything a user changes is expressed as a *proposal* — the complete set of game rows the
//! table would hold afterwards — which is validated as a whole before any byte moves:
//!
//! * a **game edit** (home, away, date of one row) re-derives that row and validates the table
//!   around it;
//! * an **import** (CSV or JSON, the documented format below) resolves the file's stable team
//!   keys through an explicit destination mapping and must fill exactly the table's active rows;
//! * a **generation** builds a balanced round-robin for an explicitly configured set of
//!   participants, keeping every row that involves a non-participant (the All-Star row) verbatim;
//!   or, with an explicit **pair-quota matrix** supplied by the user (27 Sep 2026, receipt 34), the
//!   games each pair plays — research handoff R114 §2 "user-supplied" (symmetric, zero diagonal,
//!   non-negative; total = upper-triangle sum = the rows to fill). No stock partition, NHL pair rule
//!   or cross-title quota is inferred. Neither generator replaces a row whose `round` / `status` is
//!   non-zero (R114: the PS4 18–21 Austrian final block carries `round = 1`, meaning unknown).
//!
//! Slots are always the team's **rank within its league by the team table's global `index`**
//! ([`crate::schedules::slot_map`]); the team id, the raw `index` and the record order are never
//! written. A mapping carries the slot map's fingerprint, and a proposal made against a league
//! whose membership or ordering has since changed is refused. DB04 adds whole-table replacement
//! for the seven NHL07 Xbox source templates, including packed row insertion/deletion, CRCs and
//! release packaging. Native acceptance of resized seasons remains a user verification item.
//! Other layouts must retain `current_records`. Nothing here claims the runtime's scheduling rules (the stock NHL
//! format is division-weighted and its rules are not established) or how a game mode copies the
//! source table into `leaguescheduletable`.
//!
//! Apply goes through the ordinary edit-session path ([`crate::EditSession::set_cells_int_rows`]):
//! one transaction, one undo entry, ordinary pending review and save; a refused proposal leaves
//! bytes, history and pending state untouched.
//!
//! ## File format (`nhl-studio-schedule/1`)
//!
//! Dates in files are **calendar** numbers (month 1–12, day 1–31); the table stores them
//! 0-indexed. Teams are named by a **key**: the league abbreviation when unique in the league,
//! else `ABBR#<team id>`. The season year is not part of the table and not part of the file.
//!
//! CSV (a `#` line is a comment; a header row is required; `index`, `round`, `status` and `date`
//! are optional, `date` is ignored on import):
//!
//! ```text
//! # nhl-studio-schedule/1
//! index,date,month,day,home,away,round,status
//! 0,Oct 7,10,7,TOR,MTL,0,0
//! ```
//!
//! JSON:
//!
//! ```text
//! {"format":"nhl-studio-schedule/1",
//!  "source":{"game":"nhllegacy","platform":"Xbox","table":"nhlschedule","league":"NHL", ...},
//!  "mapping_fingerprint":"…",
//!  "teams":[{"key":"TOR","slot":27,"team_id":…,"name":"Toronto Maple Leafs"}, …],
//!  "games":[{"index":0,"month":10,"day":7,"home":"TOR","away":"MTL","round":0,"status":0}, …]}
//! ```

use std::collections::{BTreeMap, BTreeSet};

use tdb_core::{FieldDescriptor, Table, TdbFile};
use tdb_defs::FieldMetaIndex;

use crate::error::DomainError;
use crate::leagues::LeagueCatalog;
use crate::schedules::{self, ScheduleLayout, ScheduleTableSummary, ScheduledGame, SlotEntry, SlotMap};
use crate::Result;

/// The documented interchange format id.
pub const FORMAT_ID: &str = "nhl-studio-schedule/1";

const MONTH_LEN: [u64; 12] = [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31];
const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

/// One game row in stored encoding (`month`/`day` 0-indexed, slots = league ranks).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameRow {
    pub index: u64,
    pub month: u64,
    pub day: u64,
    pub home_slot: u64,
    pub away_slot: u64,
    pub round: u64,
    /// `None` on the PS4 16+ layout, which has no `status` field.
    pub status: Option<u64>,
}

/// The table as it is now — the input of every plan.
#[derive(Debug, Clone)]
pub struct ScheduleModel {
    pub nhl07_layout: bool,
    pub summary: ScheduleTableSummary,
    pub layout: ScheduleLayout,
    pub slots: SlotMap,
    pub rows: Vec<GameRow>,
    /// Other schedule tables of the same league (future/debug/…) that this operation does not
    /// touch — disclosed on the plan.
    pub siblings: Vec<String>,
}

/// Exact NHL07 source layouts; mode-local/save tables are deliberately excluded.
pub fn supports_resize(model: &ScheduleModel) -> bool {
    model.nhl07_layout && matches!((model.summary.studio_table.as_str(), model.summary.name.as_str()),
        ("vufv","intldelschedule") | ("fBWa","intlelitserienschedule") | ("ADlP","intlsmliigaschedule") |
        ("uRbu","nhlschedulelongleap") | ("ksGb","nhlschedulelongnoleap") |
        ("qyyN","nhlschedulemedium") | ("zcVp","nhlschedulesmall"))
}

pub fn replacement_plan(model: &ScheduleModel, mut rows: Vec<GameRow>) -> Result<ChangePlan> {
    if !supports_resize(model) { return Err(DomainError::Validation("Adding or removing games is not yet supported for this schedule.".into())); }
    if rows.iter().any(|r|r.month>11 || r.day>30) { return Err(DomainError::Validation("Choose a valid month and day.".into())); }
    rows.sort_by_key(|r| season_key(r.month,r.day));
    for (i,row) in rows.iter_mut().enumerate() { row.index = i as u64; }
    let validation = validate(model,&rows,None);
    Ok(diff_to_plan(model,&rows,validation,Vec::new()))
}

pub fn replacement_cells(model: &ScheduleModel, rows: &[GameRow]) -> Result<Vec<Vec<(String,u64)>>> {
    if !supports_resize(model) { return Err(DomainError::Validation("Unsupported schedule layout.".into())); }
    if rows.len() > 8192 { return Err(DomainError::Validation("A schedule cannot contain more than 8192 games.".into())); }
    rows.iter().map(|r| model.layout.fields.iter().map(|f| {
        let value = match f.name.as_str() { "index"=>r.index,"month"=>r.month,"day"=>r.day,"home"=>r.home_slot,"away"=>r.away_slot,"round"=>r.round,"status"=>r.status.unwrap_or(0), _=>return Err(DomainError::Validation("Unknown schedule field; replacement refused.".into())) };
        if value > model.layout.max_value(&f.name).unwrap_or(0) { return Err(DomainError::Validation(format!("Schedule {} exceeds its supported range.",f.name))); }
        Ok((f.studio_field.clone(),value))
    }).collect()).collect()
}

/// Games per slot after a proposal, with venue split.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotGames {
    pub slot: u64,
    pub team: Option<String>,
    pub team_id: Option<u64>,
    pub games: u32,
    pub home: u32,
    pub away: u32,
}

/// Whole-proposal validation: `errors` refuse the apply; `warnings` are disclosed and do not.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Validation {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub games_per_slot: Vec<SlotGames>,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
}

impl Validation {
    pub fn ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// A validated, apply-ready change: the cell writes per record plus what they mean.
#[derive(Debug, Clone)]
pub struct ChangePlan {
    /// NHL07 structural proposals replace the complete schedule in one transaction.
    pub replacement: Option<Vec<GameRow>>,
    pub studio_table: String,
    pub table_name: String,
    pub league_label: Option<String>,
    /// `(record, [(studio field, value)])` — only cells whose value changes.
    pub rows: Vec<(u32, Vec<(String, i64)>)>,
    pub cells: usize,
    /// The changed rows before and after, resolved (capped to the first 200 for previews).
    pub before: Vec<ScheduledGame>,
    pub after: Vec<ScheduledGame>,
    pub validation: Validation,
    /// The slot map fingerprint the plan was made against.
    pub fingerprint: String,
    pub notes: Vec<String>,
}

// ---------------------------------------------------------------------------------------------
// Reading the model
// ---------------------------------------------------------------------------------------------

fn by_name<'a>(meta: &FieldMetaIndex, table: &'a Table, name: &str) -> Option<&'a FieldDescriptor> {
    table.fields.iter().find(|f| {
        meta.field(&table.id.raw(), &f.id.raw())
            .map(|m| m.name == name)
            .unwrap_or(false)
    })
}

/// Read a schedule table as rows + slot map + layout. Refuses tables without the game layout and
/// tables whose league cannot be matched (teams could not be named, so nothing can be edited by
/// team).
pub fn read_model(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    catalog: &LeagueCatalog,
    teams: &[crate::view::TeamSummary],
    studio_table: &str,
) -> Result<ScheduleModel> {
    let all = schedules::schedule_tables(db, meta, catalog);
    let summary = all
        .iter()
        .find(|t| t.studio_table == studio_table)
        .cloned()
        .ok_or_else(|| DomainError::Validation(format!("{studio_table} is not a schedule table of this database")))?;
    let table = db.table_canonical(studio_table).expect("listed");
    let layout = schedules::layout(meta, db, table);
    if !layout.is_game_layout() {
        return Err(DomainError::Validation(format!(
            "{}: this table does not carry the day/month/home/away game layout; it cannot be edited as a schedule",
            summary.name
        )));
    }
    let Some(code) = summary.league_code else {
        return Err(DomainError::Validation(format!(
            "{}: home/away slots cannot be named ({}); editing by team is not possible",
            summary.name, summary.league_match
        )));
    };
    let slots = schedules::slot_map(db, meta, catalog, teams, code);
    if slots.entries.is_empty() {
        return Err(DomainError::Validation(format!(
            "{}: the matched league ({}) has no members with a team index on this title; slots cannot be resolved",
            summary.name,
            summary.league_label.as_deref().unwrap_or("?")
        )));
    }
    let f = |n: &str| by_name(meta, table, n);
    let (f_day, f_month, f_home, f_away) = (f("day").unwrap(), f("month").unwrap(), f("home").unwrap(), f("away").unwrap());
    let (f_index, f_round, f_status) = (f("index"), f("round"), f("status"));
    let mut rows = Vec::with_capacity(table.current_records as usize);
    for rec in 0..table.current_records as u32 {
        let rd = |fd: &FieldDescriptor| db.read_u64(table, rec, fd).unwrap_or(0);
        rows.push(GameRow {
            index: f_index.map(rd).unwrap_or(rec as u64),
            month: rd(f_month),
            day: rd(f_day),
            home_slot: rd(f_home),
            away_slot: rd(f_away),
            round: f_round.map(rd).unwrap_or(0),
            status: f_status.map(rd),
        });
    }
    let siblings = all
        .iter()
        .filter(|t| t.studio_table != studio_table && t.league_code == Some(code))
        .map(|t| format!("{} ({})", t.name, t.role))
        .collect();
    Ok(ScheduleModel {
        nhl07_layout: db.table_canonical("ksGb").and_then(|t|meta.table_name(&t.id.raw())) == Some("nhlschedulelongnoleap"),
        summary,
        layout,
        slots,
        rows,
        siblings,
    })
}

// ---------------------------------------------------------------------------------------------
// Dates
// ---------------------------------------------------------------------------------------------

/// Stored (0-indexed) month/day → `Oct 7` label.
pub fn date_label(month: u64, day: u64) -> String {
    match MONTHS.get(month as usize) {
        Some(m) => format!("{m} {}", day + 1),
        None => format!("month {month} day {}", day + 1),
    }
}

/// Season ordering key for a stored date: the season starts in September (stored month 8) and
/// runs to August, so `Sep 1` is 0 and `Aug 31` is the largest — a March game sorts after an
/// October game. Yearless by design.
pub fn season_key(month: u64, day: u64) -> u64 {
    ((month + 4) % 12) * 31 + day
}

/// Day-of-season (0 = Sep 1) with real month lengths (February = 28: the table carries no year).
fn season_day(month: u64, day: u64) -> u64 {
    let mut d = 0;
    let mut m = 8;
    while m != month % 12 {
        d += MONTH_LEN[m as usize];
        m = (m + 1) % 12;
    }
    d + day
}

/// Advance a stored date by `days`, wrapping months (and December → January); February is 28
/// days because the table carries no season year.
fn add_days(month: u64, day: u64, days: u64) -> (u64, u64) {
    let (mut m, mut d) = (month % 12, day);
    let mut left = days;
    while left > 0 {
        let room = MONTH_LEN[m as usize] - 1 - d;
        if left <= room {
            d += left;
            left = 0;
        } else {
            left -= room + 1;
            d = 0;
            m = (m + 1) % 12;
        }
    }
    (m, d)
}

// ---------------------------------------------------------------------------------------------
// Validation
// ---------------------------------------------------------------------------------------------

fn resolved_game(model: &ScheduleModel, record: u32, row: &GameRow) -> ScheduledGame {
    ScheduledGame {
        record,
        index: row.index,
        month: row.month,
        day: row.day,
        date: date_label(row.month, row.day),
        round: row.round,
        status: row.status,
        home_slot: row.home_slot,
        away_slot: row.away_slot,
        home: model.slots.team_name(row.home_slot),
        away: model.slots.team_name(row.away_slot),
        // Edit previews run on league-bound tables only; tournament labels are a browse concern.
        round_label: None,
        home_label: None,
        away_label: None,
    }
}

/// Validate a complete proposal. `touched` limits the *refusing* checks to the rows the caller
/// changed (a single game edit must not be refused for a pre-existing oddity elsewhere); with
/// `None` every row is the caller's (import / generation).
pub fn validate(model: &ScheduleModel, rows: &[GameRow], touched: Option<&BTreeSet<usize>>) -> Validation {
    let mut v = Validation::default();
    if supports_resize(model) {
        if rows.len()>8192 {v.errors.push("A schedule cannot contain more than 8192 games.".into());}
        let counts=|rs:&[GameRow]|{let mut c=BTreeMap::new();for r in rs {if r.month>11||r.day>30 {continue;} for slot in [r.home_slot,r.away_slot]{*c.entry((r.month,r.day,slot)).or_insert(0usize)+=1;}} c};
        let old=counts(&model.rows);
        for ((m,d,slot),count) in counts(rows) {
            if count>1 && count>old.get(&(m,d,slot)).copied().unwrap_or(1){v.errors.push(format!("{} has more than one game on {}.",model.slots.team_name(slot).unwrap_or_else(||"A team".into()),date_label(m,d)));}
        }
        if model.summary.name=="nhlschedulelongnoleap" && rows.iter().any(|r|r.month==1&&r.day==28){v.errors.push("February 29 is not available in the non-leap schedule.".into());}
    }
    let is_mine = |i: usize| touched.map(|t| t.contains(&i)).unwrap_or(true);
    let home_max = model.layout.max_value("home").unwrap_or(u64::MAX);
    let away_max = model.layout.max_value("away").unwrap_or(u64::MAX);
    let mut pre_existing: BTreeMap<&'static str, u32> = BTreeMap::new();
    let mut note = |mine: bool, errors: &mut Vec<String>, kind: &'static str, msg: String| {
        if mine {
            errors.push(msg);
        } else {
            *pre_existing.entry(kind).or_default() += 1;
        }
    };
    let mut counts: BTreeMap<u64, (u32, u32)> = BTreeMap::new();
    let mut prev_key: Option<(u64, usize)> = None;
    let mut out_of_order: Option<String> = None;
    let mut leap_days = 0u32;
    let mut index_breaks = 0u32;
    for (i, r) in rows.iter().enumerate() {
        let mine = is_mine(i);
        if r.month > 11 || r.day > 30 { note(mine,&mut v.errors,"date",format!("Game {} has an invalid month or day.",i+1)); continue; }
        let label = format!("game {} (row {i})", r.index);
        if r.month > 11 {
            note(mine, &mut v.errors, "month", format!("{label}: month {} is not a calendar month", r.month + 1));
        } else if r.day + 1 > MONTH_LEN[r.month as usize] {
            if r.month == 1 && r.day == 28 {
                leap_days += 1;
            } else {
                note(
                    mine,
                    &mut v.errors,
                    "day",
                    format!("{label}: {} {} does not exist ({} has {} days)", MONTHS[r.month as usize], r.day + 1, MONTHS[r.month as usize], MONTH_LEN[r.month as usize]),
                );
            }
        }
        if r.home_slot == r.away_slot {
            note(mine, &mut v.errors, "self", format!("{label}: home and away are the same team (slot {})", r.home_slot));
        }
        if r.home_slot > home_max {
            note(mine, &mut v.errors, "width", format!("{label}: home slot {} does not fit the {}-bit home field (max {home_max})", r.home_slot, model.layout.field("home").map(|f| f.bit_width).unwrap_or(0)));
        }
        if r.away_slot > away_max {
            note(mine, &mut v.errors, "width", format!("{label}: away slot {} does not fit the {}-bit away field (max {away_max})", r.away_slot, model.layout.field("away").map(|f| f.bit_width).unwrap_or(0)));
        }
        for (slot, what) in [(r.home_slot, "home"), (r.away_slot, "away")] {
            if model.slots.by_slot(slot).is_none() {
                note(mine, &mut v.errors, "unresolved", format!("{label}: {what} slot {slot} has no team at that league position"));
            }
        }
        if r.index != i as u64 {
            index_breaks += 1;
        }
        let key = season_key(r.month, r.day);
        if let Some((pk, pi)) = prev_key {
            if key < pk && out_of_order.is_none() {
                out_of_order = Some(format!(
                    "rows are not in season order from game {} (row {i}, {}) after row {pi} ({}); Studio keeps new tables in date order: shipped tables are sorted except a few (R123: NHL 11 / 13 `czechschedule`, genuine NHL 20 / NHL 21 `qmjhlschedule`), and how the game treats an unsorted table is unverified",
                    r.index,
                    date_label(r.month, r.day),
                    date_label(rows[pi].month, rows[pi].day)
                ));
            }
        }
        prev_key = Some((key, i));
        counts.entry(r.home_slot).or_default().0 += 1;
        counts.entry(r.away_slot).or_default().1 += 1;
    }
    for (kind, n) in pre_existing {
        v.warnings.push(format!("{n} pre-existing row(s) outside this change fail the {kind} check (left as they are)"));
    }
    if leap_days > 0 {
        v.warnings.push(format!("{leap_days} game(s) on Feb 29: valid only in a leap season — the table carries no season year"));
    }
    if let Some(m) = out_of_order {
        v.warnings.push(m);
    }
    if index_breaks > 0 && touched.is_none() {
        v.warnings.push(format!("{index_breaks} row(s) where the running index differs from the row position"));
    }
    // Games per slot.
    for (slot, (h, a)) in &counts {
        let e = model.slots.by_slot(*slot);
        v.games_per_slot.push(SlotGames {
            slot: *slot,
            team: e.map(|e| e.name.clone()),
            team_id: e.map(|e| e.team_id),
            games: h + a,
            home: *h,
            away: *a,
        });
    }
    let named: Vec<&SlotGames> = v.games_per_slot.iter().filter(|s| s.team.is_some()).collect();
    if !named.is_empty() {
        let mut freq: BTreeMap<u32, u32> = BTreeMap::new();
        for s in &named {
            *freq.entry(s.games).or_default() += 1;
        }
        let season = freq.iter().max_by_key(|(g, n)| (**n, **g)).map(|(g, _)| *g).unwrap_or(0);
        let uneven: Vec<String> = named
            .iter()
            .filter(|s| s.games != season && !(s.games <= 2 && s.games < season))
            .map(|s| format!("{} {} games", s.team.as_deref().unwrap_or("?"), s.games))
            .collect();
        if !uneven.is_empty() {
            v.warnings.push(format!("uneven games per team (most teams play {season}): {}", uneven.join(", ")));
        }
        let unbalanced: Vec<String> = named
            .iter()
            .filter(|s| s.games > 2 && s.home.abs_diff(s.away) > 1)
            .map(|s| format!("{} {} home / {} away", s.team.as_deref().unwrap_or("?"), s.home, s.away))
            .collect();
        if !unbalanced.is_empty() {
            v.warnings.push(format!("home/away not balanced: {}", unbalanced.join(", ")));
        }
        let idle: Vec<String> = model
            .slots
            .entries
            .iter()
            .filter(|e| !counts.contains_key(&e.slot))
            .map(|e| e.name.clone())
            .collect();
        if !idle.is_empty() {
            v.warnings.push(format!("league members with no games in this table: {}", idle.join(", ")));
        }
    }
    let nonzero = rows.iter().filter(|r| r.round != 0 || r.status.unwrap_or(0) != 0).count();
    if nonzero > 0 {
        v.warnings.push(format!("{nonzero} row(s) carry a non-zero round/status; every stock row has 0 and their runtime meaning is unknown"));
    }
    v.first_date = rows.first().map(|r| date_label(r.month, r.day));
    v.last_date = rows.last().map(|r| date_label(r.month, r.day));
    v
}

/// Turn a proposal into cell writes: only the cells that differ from the current rows.
fn diff_to_plan(model: &ScheduleModel, proposed: &[GameRow], validation: Validation, mut notes: Vec<String>) -> ChangePlan {
    let field = |n: &str| model.layout.field(n).map(|f| f.studio_field.clone());
    let (f_month, f_day, f_home, f_away) = (field("month"), field("day"), field("home"), field("away"));
    let (f_index, f_round, f_status) = (field("index"), field("round"), field("status"));
    let mut rows = Vec::new();
    let mut before = Vec::new();
    let mut after = Vec::new();
    let mut cells = 0usize;
    for (i, (cur, new)) in model.rows.iter().zip(proposed.iter()).enumerate() {
        let mut writes: Vec<(String, i64)> = Vec::new();
        let mut push = |f: &Option<String>, a: u64, b: u64| {
            if let Some(f) = f {
                if a != b {
                    writes.push((f.clone(), b as i64));
                }
            }
        };
        push(&f_month, cur.month, new.month);
        push(&f_day, cur.day, new.day);
        push(&f_home, cur.home_slot, new.home_slot);
        push(&f_away, cur.away_slot, new.away_slot);
        push(&f_index, cur.index, new.index);
        push(&f_round, cur.round, new.round);
        if let (Some(a), Some(b)) = (cur.status, new.status) {
            push(&f_status, a, b);
        }
        if !writes.is_empty() {
            cells += writes.len();
            if before.len() < 200 {
                before.push(resolved_game(model, i as u32, cur));
                after.push(resolved_game(model, i as u32, new));
            }
            rows.push((i as u32, writes));
        }
    }
    let replacement = if supports_resize(model) && proposed != model.rows {
        if proposed.len() != model.rows.len() {
            notes.push(format!("Schedule changes from {} to {} games. Both teams' calendars update together.", model.rows.len(), proposed.len()));
            before = model.rows.iter().enumerate().take(200).map(|(i,r)| resolved_game(model,i as u32,r)).collect();
            after = proposed.iter().enumerate().take(200).map(|(i,r)| resolved_game(model,i as u32,r)).collect();
            cells += model.rows.len().abs_diff(proposed.len()) * model.layout.fields.len();
        }
        Some(proposed.to_vec())
    } else { None };
    ChangePlan {
        replacement,
        studio_table: model.summary.studio_table.clone(),
        table_name: model.summary.name.clone(),
        league_label: model.summary.league_label.clone(),
        rows,
        cells,
        before,
        after,
        validation,
        fingerprint: model.slots.fingerprint.clone(),
        notes,
    }
}

fn sibling_note(model: &ScheduleModel) -> Option<String> {
    if model.siblings.is_empty() {
        None
    } else {
        Some(format!(
            "only {} changes; the league's other schedule tables are untouched: {}",
            model.summary.name,
            model.siblings.join(", ")
        ))
    }
}

// ---------------------------------------------------------------------------------------------
// Single game edit
// ---------------------------------------------------------------------------------------------

/// Change the home team, away team and/or date of one game. Dates are stored encoding
/// (0-indexed month/day); teams by team id (resolved to the league rank here).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GameEdit {
    pub record: u32,
    pub home_team_id: Option<u64>,
    pub away_team_id: Option<u64>,
    pub month: Option<u64>,
    pub day: Option<u64>,
}

pub fn plan_game_edit(model: &ScheduleModel, edit: &GameEdit) -> Result<ChangePlan> {
    let i = edit.record as usize;
    let Some(cur) = model.rows.get(i) else {
        return Err(DomainError::Validation(format!(
            "{}: row {} does not exist (the table holds {} active rows)",
            model.summary.name,
            edit.record,
            model.rows.len()
        )));
    };
    let mut errors = Vec::new();
    let slot_of = |team_id: u64, what: &str, errors: &mut Vec<String>| -> Option<u64> {
        match model.slots.by_team(team_id) {
            Some(e) => Some(e.slot),
            None => {
                errors.push(format!(
                    "{what}: team id {team_id} is not a member of {} on this title",
                    model.summary.league_label.as_deref().unwrap_or("this league")
                ));
                None
            }
        }
    };
    let mut new = cur.clone();
    if let Some(t) = edit.home_team_id {
        if let Some(s) = slot_of(t, "home", &mut errors) {
            new.home_slot = s;
        }
    }
    if let Some(t) = edit.away_team_id {
        if let Some(s) = slot_of(t, "away", &mut errors) {
            new.away_slot = s;
        }
    }
    if let Some(m) = edit.month {
        new.month = m;
    }
    if let Some(d) = edit.day {
        new.day = d;
    }
    let mut proposed = model.rows.clone();
    proposed[i] = new;
    let touched: BTreeSet<usize> = [i].into_iter().collect();
    let mut validation = validate(model, &proposed, Some(&touched));
    errors.append(&mut validation.errors);
    validation.errors = errors;
    let mut notes = Vec::new();
    if edit.month.is_some() || edit.day.is_some() {
        notes.push("the row keeps its position; a date that no longer sorts with its neighbours is reported as a warning above".into());
    }
    notes.extend(sibling_note(model));
    let mut plan = diff_to_plan(model, &proposed, validation, notes);
    if plan.rows.is_empty() {
        plan.notes.push("no cell changes: the row already holds these values".into());
    }
    Ok(plan)
}

// ---------------------------------------------------------------------------------------------
// Export
// ---------------------------------------------------------------------------------------------

/// Identity of the source, written into the export header.
#[derive(Debug, Clone, Default)]
pub struct ExportContext {
    pub game_id: Option<String>,
    pub platform: String,
}

fn key_of(model: &ScheduleModel, slot: u64) -> String {
    model
        .slots
        .by_slot(slot)
        .map(|e| e.key.clone())
        .unwrap_or_else(|| format!("SLOT#{slot}"))
}

fn csv_cell(value:&str)->String { if value.contains([',','"','\n','\r']) {format!("\"{}\"",value.replace('"',"\"\""))} else {value.into()} }

/// The table as CSV (see the module docs for the format).
pub fn export_csv(model: &ScheduleModel, ctx: &ExportContext) -> String {
    // Excel needs a UTF-8 signature to preserve accented names and trademark symbols.
    let mut out = String::from("\u{feff}");
    out.push_str(&format!(
        "# {FORMAT_ID}; source {} {} {}; league {}; mapping_fingerprint {}; month/day are calendar numbers; teams by key (abbreviation, or ABBR#id when not unique); unresolved slots as SLOT#n\n",
        ctx.game_id.as_deref().unwrap_or("?"),
        ctx.platform,
        model.summary.name,
        model.summary.league_label.as_deref().unwrap_or("?"),
        model.slots.fingerprint
    ));
    let has_status = model.layout.field("status").is_some();
    out.push_str(if has_status { "index,date,month,day,home,away,round,status" } else { "index,date,month,day,home,away,round" });
    if supports_resize(model) {out.push_str(",home_id,away_id,home_name,away_name");}
    out.push('\n');
    for r in &model.rows {
        out.push_str(&format!(
            "{},{},{},{},{},{},{}",
            r.index,
            date_label(r.month, r.day),
            r.month + 1,
            r.day + 1,
            csv_cell(&key_of(model, r.home_slot)),
            csv_cell(&key_of(model, r.away_slot)),
            r.round
        ));
        if has_status {
            out.push_str(&format!(",{}", r.status.unwrap_or(0)));
        }
        if supports_resize(model) {
            let entry=|slot|model.slots.by_slot(slot);
            let quote=|name:&str|format!("\"{}\"",name.replace('"',"\"\""));
            out.push_str(&format!(",{},{},{},{}",entry(r.home_slot).map(|e|e.team_id.to_string()).unwrap_or_default(),entry(r.away_slot).map(|e|e.team_id.to_string()).unwrap_or_default(),quote(&entry(r.home_slot).map(|e|e.name.clone()).unwrap_or_default()),quote(&entry(r.away_slot).map(|e|e.name.clone()).unwrap_or_default())));
        }
        out.push('\n');
    }
    out
}

/// The table as JSON (see the module docs for the format).
pub fn export_json(model: &ScheduleModel, ctx: &ExportContext) -> String {
    use serde_json::{json, Value};
    let teams: Vec<Value> = model
        .slots
        .entries
        .iter()
        .map(|e| json!({"key": e.key, "slot": e.slot, "team_id": e.team_id, "abbrev": e.abbrev, "name": e.name, "global_index": e.global_index}))
        .collect();
    let games: Vec<Value> = model
        .rows
        .iter()
        .map(|r| {
            let mut g = json!({
                "index": r.index,
                "date": date_label(r.month, r.day),
                "month": r.month + 1,
                "day": r.day + 1,
                "home": key_of(model, r.home_slot),
                "away": key_of(model, r.away_slot),
                "round": r.round,
            });
            if let Some(s) = r.status {
                g["status"] = json!(s);
            }
            g
        })
        .collect();
    let doc = json!({
        "format": FORMAT_ID,
        "source": {
            "game": ctx.game_id,
            "platform": ctx.platform,
            "table": model.summary.name,
            "studio_table": model.summary.studio_table,
            "league_code": model.summary.league_code,
            "league": model.summary.league_label,
            "rows": model.rows.len(),
            "date_encoding": "calendar (month 1-12, day 1-31); the table stores them 0-indexed; no season year",
        },
        "mapping_fingerprint": model.slots.fingerprint,
        "teams": teams,
        "games": games,
    });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

// ---------------------------------------------------------------------------------------------
// Import
// ---------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImportFormat {
    Csv,
    Json,
}

impl ImportFormat {
    pub fn parse(s: &str) -> Result<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "csv" => Ok(ImportFormat::Csv),
            "json" => Ok(ImportFormat::Json),
            other => Err(DomainError::Validation(format!("unknown schedule format {other:?} (csv or json)"))),
        }
    }
}

/// One game as read from a file (stored encoding after parsing; keys as written).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportGame {
    /// 1-based line (CSV) or 0-based array position (JSON), for messages.
    pub at: String,
    pub month: u64,
    pub day: u64,
    pub home_key: String,
    pub away_key: String,
    pub round: Option<u64>,
    pub status: Option<u64>,
}

/// A parsed file.
#[derive(Debug, Clone, Default)]
pub struct ImportDocument {
    pub source_table: Option<String>,
    pub games: Vec<ImportGame>,
    /// Distinct team keys in first-seen order.
    pub keys: Vec<String>,
    /// The fingerprint the file declares (JSON `mapping_fingerprint`, CSV header comment).
    pub declared_fingerprint: Option<String>,
    pub source_note: Option<String>,
}

fn parse_u64(s: &str, what: &str, at: &str) -> Result<u64> {
    s.trim()
        .parse::<u64>()
        .map_err(|_| DomainError::Validation(format!("{at}: {what} {s:?} is not a whole number")))
}

fn calendar(month1: u64, day1: u64, at: &str) -> Result<(u64, u64)> {
    if !(1..=12).contains(&month1) {
        return Err(DomainError::Validation(format!("{at}: month {month1} is not 1–12")));
    }
    if !(1..=31).contains(&day1) {
        return Err(DomainError::Validation(format!("{at}: day {day1} is not 1–31")));
    }
    Ok((month1 - 1, day1 - 1))
}

fn split_csv_line(line: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    let mut chars = line.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '"' if quoted && chars.peek() == Some(&'"') => {
                cur.push('"');
                chars.next();
            }
            '"' => quoted = !quoted,
            ',' if !quoted => out.push(std::mem::take(&mut cur)),
            _ => cur.push(c),
        }
    }
    out.push(cur);
    out.iter().map(|s| s.trim().to_string()).collect()
}

fn csv_records(text:&str)->Result<Vec<(usize,String)>> {
    let mut records=Vec::new();let mut record=String::new();let mut quoted=false;let mut start=1;
    for (i,line) in text.trim_start_matches('\u{feff}').lines().enumerate(){
        if record.is_empty(){start=i+1;if line.trim_start().starts_with('#'){records.push((start,line.to_owned()));continue;}}
        else{record.push('\n');}
        record.push_str(line);
        let mut chars=line.chars().peekable();while let Some(c)=chars.next(){if c=='"'{if quoted&&chars.peek()==Some(&'"'){chars.next();}else{quoted=!quoted;}}}
        if !quoted{records.push((start,std::mem::take(&mut record)));}
    }
    if quoted{return Err(DomainError::Validation(format!("CSV line {start} has an unclosed quoted value.")));}
    Ok(records)
}
fn parse_csv(text: &str) -> Result<ImportDocument> {
    let mut doc = ImportDocument::default();
    let mut header: Option<BTreeMap<String, usize>> = None;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (line_no, raw) in csv_records(text)? {
        let raw = raw.trim_start_matches('\u{feff}');
        let line = raw.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        if let Some(c) = line.trim_start().strip_prefix('#') {
            if let Some(source) = c.split("; source ").nth(1).and_then(|v|v.split(';').next()) {
                doc.source_table=source.split_whitespace().nth(2).map(str::to_owned);
            }
            if let Some(pos) = c.find("mapping_fingerprint ") {
                let rest = &c[pos + "mapping_fingerprint ".len()..];
                let fp: String = rest.chars().take_while(|ch| ch.is_ascii_hexdigit()).collect();
                if !fp.is_empty() {
                    doc.declared_fingerprint = Some(fp);
                }
            }
            if doc.source_note.is_none() && c.contains(FORMAT_ID) {
                doc.source_note = Some(c.trim().to_string());
            }
            continue;
        }
        let cells = split_csv_line(line);
        let Some(h) = &header else {
            let mut map = BTreeMap::new();
            for (i, c) in cells.iter().enumerate() {
                map.insert(c.to_ascii_lowercase(), i);
            }
            for need in ["month", "day", "home", "away"] {
                if !map.contains_key(need) {
                    return Err(DomainError::Validation(format!(
                        "line {line_no}: the header row must name the columns month, day, home and away (found: {})",
                        cells.join(", ")
                    )));
                }
            }
            header = Some(map);
            continue;
        };
        let at = format!("line {line_no}");
        let get = |name: &str| -> Option<&str> { h.get(name).and_then(|i| cells.get(*i)).map(|s| s.as_str()) };
        let month1 = parse_u64(get("month").unwrap_or(""), "month", &at)?;
        let day1 = parse_u64(get("day").unwrap_or(""), "day", &at)?;
        let (month, day) = calendar(month1, day1, &at)?;
        let home_key = get("home_id").filter(|s|!s.is_empty()).or_else(||get("home")).unwrap_or("").to_string();
        let away_key = get("away_id").filter(|s|!s.is_empty()).or_else(||get("away")).unwrap_or("").to_string();
        if home_key.is_empty() || away_key.is_empty() {
            return Err(DomainError::Validation(format!("{at}: home and away must both name a team")));
        }
        let opt = |name: &str| -> Result<Option<u64>> {
            match get(name) {
                Some(s) if !s.is_empty() => parse_u64(s, name, &at).map(Some),
                _ => Ok(None),
            }
        };
        for k in [&home_key, &away_key] {
            if seen.insert(k.to_ascii_uppercase()) {
                doc.keys.push(k.clone());
            }
        }
        let (round, status) = (opt("round")?, opt("status")?);
        doc.games.push(ImportGame {
            at,
            month,
            day,
            home_key,
            away_key,
            round,
            status,
        });
    }
    if header.is_none() {
        return Err(DomainError::Validation("the CSV has no header row (expected: index,date,month,day,home,away,round,status)".into()));
    }
    Ok(doc)
}

fn parse_json(text: &str) -> Result<ImportDocument> {
    use serde_json::Value;
    let v: Value = serde_json::from_str(text).map_err(|e| DomainError::Validation(format!("not valid JSON: {e}")))?;
    let obj = v
        .as_object()
        .ok_or_else(|| DomainError::Validation("the JSON document must be an object with a \"games\" array".into()))?;
    if let Some(f) = obj.get("format").and_then(Value::as_str) {
        if f != FORMAT_ID {
            return Err(DomainError::Validation(format!("unsupported format {f:?} (expected {FORMAT_ID})")));
        }
    }
    let mut doc = ImportDocument {
        declared_fingerprint: obj.get("mapping_fingerprint").and_then(Value::as_str).map(str::to_string),
        source_note: obj.get("source").map(|s| s.to_string()),
        source_table: obj.get("source").and_then(|s|s.get("table")).and_then(Value::as_str).map(str::to_owned),
        ..Default::default()
    };
    let games = obj
        .get("games")
        .and_then(Value::as_array)
        .ok_or_else(|| DomainError::Validation("the JSON document has no \"games\" array".into()))?;
    let mut seen: BTreeSet<String> = BTreeSet::new();
    for (i, g) in games.iter().enumerate() {
        let at = format!("game[{i}]");
        let o = g
            .as_object()
            .ok_or_else(|| DomainError::Validation(format!("{at}: not an object")))?;
        let num = |k: &str| -> Result<u64> {
            o.get(k)
                .and_then(Value::as_u64)
                .ok_or_else(|| DomainError::Validation(format!("{at}: {k} is missing or not a whole number")))
        };
        let key = |k: &str| -> Result<String> {
            match o.get(k) {
                Some(Value::String(s)) if !s.trim().is_empty() => Ok(s.trim().to_string()),
                Some(Value::Number(n)) => Ok(n.to_string()),
                Some(Value::Object(t)) => t
                    .get("key")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .ok_or_else(|| DomainError::Validation(format!("{at}: {k} object has no \"key\""))),
                _ => Err(DomainError::Validation(format!("{at}: {k} must name a team key"))),
            }
        };
        let (month, day) = calendar(num("month")?, num("day")?, &at)?;
        let home_key = key("home")?;
        let away_key = key("away")?;
        for k in [&home_key, &away_key] {
            if seen.insert(k.to_ascii_uppercase()) {
                doc.keys.push(k.clone());
            }
        }
        let opt = |k: &str| -> Result<Option<u64>> {
            match o.get(k) {
                None | Some(Value::Null) => Ok(None),
                Some(v) => v
                    .as_u64()
                    .map(Some)
                    .ok_or_else(|| DomainError::Validation(format!("{at}: {k} is not a whole number"))),
            }
        };
        let (round, status) = (opt("round")?, opt("status")?);
        doc.games.push(ImportGame {
            at,
            month,
            day,
            home_key,
            away_key,
            round,
            status,
        });
    }
    Ok(doc)
}

/// Parse a schedule file; malformed input is an error naming the line/entry.
pub fn parse_import(text: &str, format: ImportFormat) -> Result<ImportDocument> {
    let doc = match format {
        ImportFormat::Csv => parse_csv(text)?,
        ImportFormat::Json => parse_json(text)?,
    };
    if doc.games.is_empty() {
        return Err(DomainError::Validation("the file contains no games".into()));
    }
    Ok(doc)
}

/// One explicit destination assignment: file key → team on this title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamMapping {
    pub key: String,
    pub team_id: u64,
}

/// How a file key was resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyResolution {
    pub key: String,
    pub team_id: Option<u64>,
    pub team: Option<String>,
    pub slot: Option<u64>,
    /// `explicit` (from the mapping), `auto` (exact key / id / unique abbreviation match),
    /// `ambiguous` (several members match), `unmapped`.
    pub how: &'static str,
    pub games: u32,
}

/// An import plan: the change plus how every key was mapped.
#[derive(Debug, Clone)]
pub struct ImportPlan {
    pub change: ChangePlan,
    pub keys: Vec<KeyResolution>,
    pub rows_in_file: usize,
    pub rows_in_table: usize,
}

/// Resolve keys and build the whole-table proposal. `expected_fingerprint` is the slot-map
/// fingerprint the mapping was made against (from a previous preview or the file); a different
/// current fingerprint refuses the plan as stale.
pub fn plan_import(
    model: &ScheduleModel,
    doc: &ImportDocument,
    mapping: &[TeamMapping],
    expected_fingerprint: Option<&str>,
) -> Result<ImportPlan> {
    let mut errors = Vec::new();
    let mut notes = Vec::new();
    if let Some(table)=&doc.source_table { if table!=&model.summary.name { errors.push(format!("This file belongs to {table}; select that schedule before importing.")); } }
    // Key resolution.
    let mut games_per_key: BTreeMap<String, u32> = BTreeMap::new();
    for g in &doc.games {
        *games_per_key.entry(g.home_key.to_ascii_uppercase()).or_default() += 1;
        *games_per_key.entry(g.away_key.to_ascii_uppercase()).or_default() += 1;
    }
    let mut resolved: BTreeMap<String, KeyResolution> = BTreeMap::new();
    for key in &doc.keys {
        let upper = key.to_ascii_uppercase();
        let games = games_per_key.get(&upper).copied().unwrap_or(0);
        let explicit = mapping.iter().find(|m| m.key.eq_ignore_ascii_case(key));
        let (entry, how): (Option<&SlotEntry>, &'static str) = if let Some(m) = explicit {
            match model.slots.by_team(m.team_id) {
                Some(e) => (Some(e), "explicit"),
                None => {
                    errors.push(format!(
                        "mapping for {key:?}: team id {} is not a member of {} on this title",
                        m.team_id,
                        model.summary.league_label.as_deref().unwrap_or("this league")
                    ));
                    (None, "unmapped")
                }
            }
        } else if let Some(e) = model.slots.by_key(key) {
            (Some(e), "auto")
        } else if let Some(e) = key.parse::<u64>().ok().and_then(|id| model.slots.by_team(id)) {
            (Some(e), "auto")
        } else {
            let by_abbrev: Vec<&SlotEntry> = model
                .slots
                .entries
                .iter()
                .filter(|e| e.abbrev.eq_ignore_ascii_case(key))
                .collect();
            match by_abbrev.len() {
                1 => (Some(by_abbrev[0]), "auto"),
                0 => (None, "unmapped"),
                _ => (None, "ambiguous"),
            }
        };
        resolved.insert(
            upper,
            KeyResolution {
                key: key.clone(),
                team_id: entry.map(|e| e.team_id),
                team: entry.map(|e| e.name.clone()),
                slot: entry.map(|e| e.slot),
                how,
                games,
            },
        );
    }
    let unmapped: Vec<String> = resolved.values().filter(|r| r.how == "unmapped").map(|r| r.key.clone()).collect();
    if !unmapped.is_empty() {
        errors.push(format!(
            "{} team key(s) in the file match no member of {} — map each one explicitly: {}",
            unmapped.len(),
            model.summary.league_label.as_deref().unwrap_or("this league"),
            unmapped.join(", ")
        ));
    }
    let ambiguous: Vec<String> = resolved.values().filter(|r| r.how == "ambiguous").map(|r| r.key.clone()).collect();
    if !ambiguous.is_empty() {
        errors.push(format!(
            "{} team key(s) match more than one member — choose the destination explicitly: {}",
            ambiguous.len(),
            ambiguous.join(", ")
        ));
    }
    // Two keys → one team.
    let mut by_team: BTreeMap<u64, Vec<String>> = BTreeMap::new();
    for r in resolved.values() {
        if let Some(t) = r.team_id {
            by_team.entry(t).or_default().push(r.key.clone());
        }
    }
    for (t, keys) in by_team.iter().filter(|(_, k)| k.len() > 1) {
        errors.push(format!(
            "keys {} all map to {} — every key must name a different team",
            keys.join(" and "),
            model.slots.by_team(*t).map(|e| e.name.clone()).unwrap_or_else(|| format!("team {t}"))
        ));
    }
    // Stale mapping.
    if let Some(exp) = expected_fingerprint {
        if exp != model.slots.fingerprint {
            errors.push(format!(
                "the team mapping was made against a different league state (fingerprint {exp} vs {} now): team ordering or membership changed since — preview again",
                model.slots.fingerprint
            ));
        }
    }
    if let Some(fp) = &doc.declared_fingerprint {
        if fp != &model.slots.fingerprint {
            notes.push(format!(
                "the file was exported from a different league state or title (fingerprint {fp}); teams are matched by key, not by slot"
            ));
        }
    }
    // Row count.
    let rows_in_table = model.rows.len();
    if doc.games.len() != rows_in_table && !supports_resize(model) {
        errors.push(format!(
            "the file has {} games but {} holds {} active rows; Studio does not truncate or grow a schedule table (a shorter table's engine handling is unverified and unused rows have no established representation) — supply exactly {} games",
            doc.games.len(),
            model.summary.name,
            rows_in_table,
            rows_in_table
        ));
    }
    // Proposal (only meaningful when counts match; otherwise the diff is against a prefix).
    let has_status = model.layout.field("status").is_some();
    let mut proposed: Vec<GameRow> = Vec::with_capacity(doc.games.len());
    for (i, g) in doc.games.iter().enumerate() {
        let slot = |k: &str| resolved.get(&k.to_ascii_uppercase()).and_then(|r| r.slot);
        let cur = model.rows.get(i);
        proposed.push(GameRow {
            index: i as u64,
            month: g.month,
            day: g.day,
            home_slot: slot(&g.home_key).unwrap_or(u64::MAX),
            away_slot: slot(&g.away_key).unwrap_or(u64::MAX),
            round: g.round.or(cur.map(|c| c.round)).unwrap_or(0),
            status: if has_status { Some(g.status.or(cur.and_then(|c| c.status)).unwrap_or(0)) } else { None },
        });
    }
    let mut validation = if errors.is_empty() {
        validate(model, &proposed, None)
    } else {
        Validation::default()
    };
    errors.append(&mut validation.errors);
    validation.errors = errors;
    let rows_in_file = doc.games.len();
    let mut keys: Vec<KeyResolution> = resolved.into_values().collect();
    keys.sort_by(|a, b| a.key.cmp(&b.key));
    notes.push("round/status not given in the file keep the row's current value (0 in every stock row)".into());
    notes.extend(sibling_note(model));
    let change = if validation.ok() {
        diff_to_plan(model, &proposed, validation, notes)
    } else {
        ChangePlan {
            replacement: None,
            studio_table: model.summary.studio_table.clone(),
            table_name: model.summary.name.clone(),
            league_label: model.summary.league_label.clone(),
            rows: Vec::new(),
            cells: 0,
            before: Vec::new(),
            after: Vec::new(),
            validation,
            fingerprint: model.slots.fingerprint.clone(),
            notes,
        }
    };
    Ok(ImportPlan {
        change,
        keys,
        rows_in_file,
        rows_in_table,
    })
}

// ---------------------------------------------------------------------------------------------
// Generation
// ---------------------------------------------------------------------------------------------

/// What the generator is asked to build. `start_month`/`start_day` are stored encoding
/// (0-indexed); `days_between_matchdays` spaces the round-robin matchdays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenerationConfig {
    /// Team ids (league members) that play the round-robin; every row involving any other slot
    /// is kept verbatim.
    pub participants: Vec<u64>,
    /// How many times each pair meets (2 = one home, one away).
    pub rounds: u32,
    pub start_month: u64,
    pub start_day: u64,
    pub days_between_matchdays: u32,
    /// Receipt 34: games per pair, a square matrix in the order of `participants` (symmetric, zero
    /// diagonal). When given, `rounds` is ignored and the pair-quota generator runs.
    pub pair_quotas: Option<Vec<Vec<u32>>>,
    /// Who hosts the extra game of a pair with an odd count (research R123 Q11: required when any
    /// count is odd; never assumed).
    pub odd_home: Option<OddHome>,
    /// Receipt 48 (R123 single-phase partition, accepted 28 Sep 2026): groups of participants with games per
    /// pair inside a group and across groups. Converted into the pair-quota matrix after its own checks; not
    /// combinable with `pair_quotas`.
    pub groups: Option<GroupQuotas>,
    /// `OddHome::Explicit`: the host of every odd pair, as (home team id, away team id). Must name each odd pair
    /// exactly once and nothing else.
    pub odd_hosts: Option<Vec<(u64, u64)>>,
}

/// A single-phase partition request (R123 Q5): every participant in exactly one non-empty group.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroupQuotas {
    /// Team ids per group.
    pub groups: Vec<Vec<u64>>,
    /// Games between two clubs of the same group.
    pub within: u32,
    /// Games between two clubs of different groups.
    pub across: u32,
    /// Where the groups came from, for the notes (for example the shipped table's recorded source).
    pub origin: Option<String>,
}

/// The host of the extra game of an odd pair count (R123 `odd_home`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OddHome {
    /// The club at the lower league slot.
    LowerSlot,
    /// The club at the higher league slot.
    HigherSlot,
    /// Deterministic balance (receipt 48 defines it): the odd pairs are taken in participant order (i < j,
    /// row by row); each extra home game goes to the club of the pair with fewer home games so far — counting
    /// the even halves of every pair and the extras already given — and, on a tie, to the club at the lower
    /// league slot. Every chosen host is reported.
    Balance,
    /// The user names the host of every odd pair (`GenerationConfig::odd_hosts`).
    Explicit,
}

/// One league member as the generator sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Participant {
    pub team_id: u64,
    pub slot: u64,
    pub name: String,
    pub abbrev: String,
    /// Games in the current table.
    pub games: u32,
    /// Proposed by default (plays a full season now; All-Star / exhibition squads are not).
    pub default: bool,
}

/// The topology the current table supports, or the precise reason it cannot be generated.
#[derive(Debug, Clone)]
pub struct GenerationOptions {
    pub members: Vec<Participant>,
    pub kept_rows: usize,
    pub rows_to_fill: usize,
    pub pairs: usize,
    /// `rows_to_fill / pairs` when whole.
    pub suggested_rounds: Option<u32>,
    pub suggested_start_month: u64,
    pub suggested_start_day: u64,
    pub suggested_days_between_matchdays: u32,
    pub matchdays_per_round: u32,
    pub blocker: Option<String>,
    pub notes: Vec<String>,
}

fn kept_rows(model: &ScheduleModel, slots: &BTreeSet<u64>) -> Vec<GameRow> {
    model
        .rows
        .iter()
        .filter(|r| !(slots.contains(&r.home_slot) && slots.contains(&r.away_slot)))
        .cloned()
        .collect()
}

fn options_for(model: &ScheduleModel, participants: &[u64]) -> GenerationOptions {
    let mut counts: BTreeMap<u64, u32> = BTreeMap::new();
    for r in &model.rows {
        *counts.entry(r.home_slot).or_default() += 1;
        *counts.entry(r.away_slot).or_default() += 1;
    }
    let mut freq: BTreeMap<u32, u32> = BTreeMap::new();
    for e in &model.slots.entries {
        let g = counts.get(&e.slot).copied().unwrap_or(0);
        if g > 0 {
            *freq.entry(g).or_default() += 1;
        }
    }
    let season = freq.iter().max_by_key(|(g, n)| (**n, **g)).map(|(g, _)| *g).unwrap_or(0);
    let members: Vec<Participant> = model
        .slots
        .entries
        .iter()
        .map(|e| {
            let games = counts.get(&e.slot).copied().unwrap_or(0);
            Participant {
                team_id: e.team_id,
                slot: e.slot,
                name: e.name.clone(),
                abbrev: e.abbrev.clone(),
                games,
                default: games > 2 || (games > 0 && games >= season),
            }
        })
        .collect();
    let chosen: BTreeSet<u64> = participants
        .iter()
        .filter_map(|t| model.slots.by_team(*t).map(|e| e.slot))
        .collect();
    let n = chosen.len();
    let kept = kept_rows(model, &chosen).len();
    let rows_to_fill = model.rows.len().saturating_sub(kept);
    let pairs = n * n.saturating_sub(1) / 2;
    let suggested_rounds = if pairs > 0 && rows_to_fill % pairs == 0 { Some((rows_to_fill / pairs) as u32) } else { None };
    let matchdays_per_round = if n % 2 == 0 { n.saturating_sub(1) } else { n } as u32;
    // Date suggestions from the current table's span.
    let (first, last) = (model.rows.first(), model.rows.last());
    let (sm, sd) = first.map(|r| (r.month, r.day)).unwrap_or((9, 0));
    let span = match (first, last) {
        (Some(a), Some(b)) => season_day(b.month, b.day).saturating_sub(season_day(a.month, a.day)),
        _ => 180,
    };
    let total_matchdays = suggested_rounds.unwrap_or(2) * matchdays_per_round.max(1);
    let spacing = if total_matchdays > 1 { (span / (total_matchdays as u64 - 1)).clamp(1, 14) as u32 } else { 2 };
    let mut notes = vec![
        format!(
            "topology: every pair of the {n} participants meets the same number of times (balanced round-robin; venue alternates per pair); {kept} row(s) involving other slots (All-Star / exhibition / clubs left out) stay in place with every value; a generated game never shares a date with a kept game of its clubs"
        ),
        "the stock NHL format is division-weighted; its rules are not established and are not reproduced".into(),
        "round and status are written as 0 (every stock league row except the PS4 18–21 Austrian final block, R114 — a generation that would replace a non-zero round / status row is refused); dates are yearless (February = 28 days)".into(),
    ];
    let blocker = if n < 2 {
        Some("choose at least two participants".to_string())
    } else if !supports_resize(model) && pairs > 0 && rows_to_fill % pairs != 0 {
        let lo = rows_to_fill / pairs;
        notes.push(format!("nearest whole round-robins: {lo} → {} games, {} → {} games", lo * pairs, lo + 1, (lo + 1) * pairs));
        Some(format!(
            "{rows_to_fill} games for {n} teams is not a whole number of round-robins ({pairs} pairings per round-robin): this table's season cannot be generated as a balanced round-robin without changing its row count, which Studio does not do — edit and import remain available"
        ))
    } else {
        None
    };
    GenerationOptions {
        members,
        kept_rows: kept,
        rows_to_fill,
        pairs,
        suggested_rounds,
        suggested_start_month: sm,
        suggested_start_day: sd,
        suggested_days_between_matchdays: spacing,
        matchdays_per_round,
        blocker,
        notes,
    }
}

/// The default topology for the table (participants = members playing a full season now).
pub fn generation_options(model: &ScheduleModel) -> GenerationOptions {
    let probe = options_for(model, &[]);
    let defaults: Vec<u64> = probe.members.iter().filter(|m| m.default).map(|m| m.team_id).collect();
    options_for(model, &defaults)
}

/// Round-robin matchdays by the circle method over `n` seats (a bye seat is appended when `n`
/// is odd). Returns, per matchday, the pairs `(a, b)` as seat indices with `a < b`.
fn circle_matchdays(n: usize) -> Vec<Vec<(usize, usize)>> {
    let seats = if n % 2 == 0 { n } else { n + 1 };
    let bye = if n % 2 == 0 { None } else { Some(n) };
    let mut ring: Vec<usize> = (1..seats).collect();
    let mut out = Vec::with_capacity(seats - 1);
    for _ in 0..seats - 1 {
        let mut day = Vec::with_capacity(seats / 2);
        let arr: Vec<usize> = std::iter::once(0).chain(ring.iter().copied()).collect();
        for k in 0..seats / 2 {
            let (a, b) = (arr[k], arr[seats - 1 - k]);
            if Some(a) == bye || Some(b) == bye {
                continue;
            }
            day.push((a.min(b), a.max(b)));
        }
        out.push(day);
        ring.rotate_right(1);
    }
    out
}

/// A generation plan: the change plus its shape.
#[derive(Debug, Clone)]
pub struct GenerationPlan {
    pub change: ChangePlan,
    pub participants: usize,
    pub rounds: u32,
    pub matchdays: u32,
    pub generated_rows: usize,
    pub kept_rows: usize,
    pub games_per_participant: u32,
}

/// Build the deterministic proposal for `cfg`. Refuses (with the exact numbers) when the
/// topology does not fill the table's active rows exactly, when a participant is not a league
/// member, or when the dates run past the season boundary.
pub fn plan_generation(model: &ScheduleModel, cfg: &GenerationConfig) -> Result<GenerationPlan> {
    if let Some(g) = &cfg.groups {
        if cfg.pair_quotas.is_some() {
            return Ok(refused(model, vec!["choose groups or a games-per-pair table, not both".into()]));
        }
        return match groups_to_matrix(model, &cfg.participants, g) {
            Ok(q) => {
                let mut plan = plan_quota_generation(model, &GenerationConfig { pair_quotas: Some(q), ..cfg.clone() })?;
                if plan.change.validation.ok() {
                    plan.change.notes.insert(0, group_note(model, g));
                }
                Ok(plan)
            }
            Err(errors) => Ok(refused(model, errors)),
        };
    }
    if cfg.pair_quotas.is_some() {
        return plan_quota_generation(model, cfg);
    }
    let mut errors = Vec::new();
    // Participants → slots, in slot order (deterministic).
    let mut seen = BTreeSet::new();
    let mut seats: Vec<&SlotEntry> = Vec::new();
    for t in &cfg.participants {
        if !seen.insert(*t) {
            errors.push(format!("team id {t} is listed twice"));
            continue;
        }
        match model.slots.by_team(*t) {
            Some(e) => seats.push(e),
            None => errors.push(format!(
                "team id {t} is not a member of {} on this title",
                model.summary.league_label.as_deref().unwrap_or("this league")
            )),
        }
    }
    seats.sort_by_key(|e| e.slot);
    let n = seats.len();
    let opts = options_for(model, &cfg.participants);
    if let Some(b) = &opts.blocker {
        if n < 2 {
            errors.push(b.clone());
        }
    }
    if cfg.rounds == 0 {
        errors.push("rounds must be at least 1".into());
    }
    if cfg.start_month > 11 || cfg.start_day + 1 > MONTH_LEN.get(cfg.start_month as usize).copied().unwrap_or(0) {
        errors.push(format!("start date {} does not exist", date_label(cfg.start_month, cfg.start_day)));
    }
    if cfg.days_between_matchdays == 0 {
        errors.push("days between matchdays must be at least 1".into());
    }
    let chosen: BTreeSet<u64> = seats.iter().map(|e| e.slot).collect();
    let kept = kept_rows(model, &chosen);
    errors.extend(round_tag_refusal(model, &chosen));
    let generated_rows = cfg.rounds as usize * n * n.saturating_sub(1) / 2;
    if supports_resize(model) && generated_rows + kept.len()>8192 {errors.push("A schedule cannot contain more than 8192 games.".into());}
    if errors.is_empty() && !supports_resize(model) && generated_rows + kept.len() != model.rows.len() {
        errors.push(format!(
            "{} participants × {} round-robin(s) = {generated_rows} games, plus {} kept row(s) = {}, but {} holds {} active rows; Studio does not change a schedule table's row count — {}",
            n,
            cfg.rounds,
            kept.len(),
            generated_rows + kept.len(),
            model.summary.name,
            model.rows.len(),
            opts.blocker.clone().unwrap_or_else(|| match opts.suggested_rounds {
                Some(r) => format!("{r} round-robin(s) would fit exactly"),
                None => "no whole number of round-robins fits".into(),
            })
        ));
    }
    let matchdays_per_round = circle_matchdays(n.max(2)).len() as u32;
    let matchdays = cfg.rounds * matchdays_per_round;
    let span_days = (matchdays.saturating_sub(1) as u64) * cfg.days_between_matchdays as u64;
    if errors.is_empty() && span_days >= 365 {
        errors.push(format!(
            "{matchdays} matchdays every {} day(s) span {span_days} days — past one season; shorten the spacing",
            cfg.days_between_matchdays
        ));
    }
    if errors.is_empty() {
        let (em, ed) = add_days(cfg.start_month, cfg.start_day, span_days);
        if season_key(em, ed) < season_key(cfg.start_month, cfg.start_day) {
            errors.push(format!(
                "the last matchday ({}) would cross the September season boundary from a {} start; shorten the spacing or start earlier",
                date_label(em, ed),
                date_label(cfg.start_month, cfg.start_day)
            ));
        }
    }
    if !errors.is_empty() {
        let validation = Validation {
            errors,
            ..Default::default()
        };
        return Ok(GenerationPlan {
            change: ChangePlan {
            replacement: None,
                studio_table: model.summary.studio_table.clone(),
                table_name: model.summary.name.clone(),
                league_label: model.summary.league_label.clone(),
                rows: Vec::new(),
                cells: 0,
                before: Vec::new(),
                after: Vec::new(),
                validation,
                fingerprint: model.slots.fingerprint.clone(),
                notes: opts.notes,
            },
            participants: n,
            rounds: cfg.rounds,
            matchdays,
            generated_rows,
            kept_rows: kept.len(),
            games_per_participant: cfg.rounds * n.saturating_sub(1) as u32,
        });
    }
    // Build: cycle → matchday → pairs; venue by (cycle + a + b) parity so consecutive cycles
    // swap venues for every pair (an even number of rounds is exactly balanced).
    let circle = circle_matchdays(n);
    let mut days: Vec<Vec<(u64, u64)>> = Vec::with_capacity(matchdays as usize);
    for cycle in 0..cfg.rounds as usize {
        for day in &circle {
            days.push(
                day.iter()
                    .map(|&(a, b)| {
                        let (home, away) = if (cycle + a + b) % 2 == 0 { (a, b) } else { (b, a) };
                        (seats[home].slot, seats[away].slot)
                    })
                    .collect(),
            );
        }
    }
    let (proposed, placement_notes) = match place_generated(model, &chosen, &days, (cfg.start_month, cfg.start_day), cfg.days_between_matchdays) {
        Ok(p) => p,
        Err(e) => {
            return Ok(GenerationPlan {
                change: ChangePlan {
            replacement: None,
                    studio_table: model.summary.studio_table.clone(),
                    table_name: model.summary.name.clone(),
                    league_label: model.summary.league_label.clone(),
                    rows: Vec::new(),
                    cells: 0,
                    before: Vec::new(),
                    after: Vec::new(),
                    validation: Validation { errors: vec![e], ..Default::default() },
                    fingerprint: model.slots.fingerprint.clone(),
                    notes: opts.notes,
                },
                participants: n,
                rounds: cfg.rounds,
                matchdays,
                generated_rows,
                kept_rows: kept.len(),
                games_per_participant: cfg.rounds * (n as u32 - 1),
            })
        }
    };
    let validation = validate(model, &proposed, None);
    let mut notes = opts.notes.clone();
    notes.push(format!(
        "{n} participants, {} round-robin(s), {matchdays} matchdays from {} every {} day(s); {} kept row(s)",
        cfg.rounds,
        date_label(cfg.start_month, cfg.start_day),
        cfg.days_between_matchdays,
        kept.len()
    ));
    notes.extend(placement_notes);
    notes.extend(sibling_note(model));
    let change = diff_to_plan(model, &proposed, validation, notes);
    Ok(GenerationPlan {
        change,
        participants: n,
        rounds: cfg.rounds,
        matchdays,
        generated_rows,
        kept_rows: kept.len(),
        games_per_participant: cfg.rounds * (n as u32 - 1),
    })
}

/// Stored date of a day-of-season (0 = Sep 1; February = 28 days).
fn from_season_day(sd: u64) -> (u64, u64) {
    add_days(8, 0, sd)
}

fn season_day_label(sd: u64) -> String {
    let (m, d) = from_season_day(sd);
    date_label(m, d)
}

/// Receipt 41 — where generated games go (research R123 R2; dispatch after receipt 40 §2).
///
/// A row involving a non-participant (an All-Star game, a game against a club left out) is
/// **retained**: it keeps its position and every stored value (index, date, clubs, round, status).
/// Generated games fill only the other positions, in matchday order, and keep each position's
/// existing index (the generator never re-indexes a table). Dates follow the user's start and
/// spacing, with two adjustments, each disclosed in the notes:
/// - **date order around a retained row**: the games placed before a retained row must be dated
///   no later than it and the games after it no earlier (Studio keeps generated tables date-sorted — almost every stock table is, R123 lists the Czech / QMJHL exceptions; the
///   engine's tolerance of an unsorted table is unverified). A matchday split by a retained row is
///   two disjoint sets of clubs, so its later part may move forward to the retained row's date.
///   Nothing is moved earlier: a matchday that cannot be dated by the retained row after it
///   refuses the plan.
/// - **dates a club already plays a retained game**: a generated game never shares a date with a
///   retained game of either of its clubs; the matchday moves forward a day at a time, and one
///   that runs past the next retained row that way refuses the plan.
///
/// Double bookings among the retained rows themselves are source anomalies: reported as such,
/// never attributed to the generation. Returns the proposed rows and notes, or the refusal.
fn place_generated(
    model: &ScheduleModel,
    chosen: &BTreeSet<u64>,
    days: &[Vec<(u64, u64)>],
    start: (u64, u64),
    spacing: u32,
) -> std::result::Result<(Vec<GameRow>, Vec<String>), String> {
    let has_status = model.layout.field("status").is_some();
    let is_kept = |r: &GameRow| !(chosen.contains(&r.home_slot) && chosen.contains(&r.away_slot));
    if supports_resize(model) && days.iter().map(Vec::len).sum::<usize>() + model.rows.iter().filter(|r|is_kept(r)).count() != model.rows.len() {
        let mut proposed:Vec<GameRow>=model.rows.iter().filter(|r|is_kept(r)).cloned().collect();
        let mut occupied=BTreeSet::new();for r in &proposed {for slot in [r.home_slot,r.away_slot]{occupied.insert((season_day(r.month,r.day),slot));}}
        let mut date=season_day(start.0,start.1);
        for (i,matchday) in days.iter().enumerate() {
            if i>0 {date+=spacing as u64;}
            while matchday.iter().any(|(h,a)|occupied.contains(&(date,*h))||occupied.contains(&(date,*a))) {date+=1;}
            if date>=365{return Err("The generated games extend beyond this season. Start earlier or reduce the spacing.".into());}
            let (month,day)=from_season_day(date);
            for &(home_slot,away_slot) in matchday {proposed.push(GameRow{index:0,month,day,home_slot,away_slot,round:0,status:has_status.then_some(0)});occupied.insert((date,home_slot));occupied.insert((date,away_slot));}
        }
        proposed.sort_by_key(|r|season_key(r.month,r.day));for (i,r) in proposed.iter_mut().enumerate(){r.index=i as u64;}
        return Ok((proposed,vec!["Non-participating fixtures retain their dates, opponents and game settings.".into()]));
    }
    let kept_pos: Vec<usize> = (0..model.rows.len()).filter(|i| is_kept(&model.rows[*i])).collect();
    let free: Vec<usize> = (0..model.rows.len()).filter(|i| !is_kept(&model.rows[*i])).collect();
    let games: Vec<(usize, u64, u64)> = days.iter().enumerate().flat_map(|(m, d)| d.iter().map(move |&(h, a)| (m, h, a))).collect();
    if games.len() != free.len() {
        return Err(format!("internal: {} generated games for {} replaceable rows; refused", games.len(), free.len()));
    }
    let club = |s: u64| model.slots.team_name(s).unwrap_or_else(|| format!("slot {s}"));
    let describe = |i: usize| {
        let r = &model.rows[i];
        format!("row {i} (game {}, {}: {} v {})", r.index, date_label(r.month, r.day), club(r.home_slot), club(r.away_slot))
    };
    let selected_club = |i: usize| {
        let r = &model.rows[i];
        club(if chosen.contains(&r.home_slot) { r.home_slot } else { r.away_slot })
    };
    // Retained games of a selected club: (day of season, club) → the retained row.
    let mut occupied: BTreeMap<(u64, u64), usize> = BTreeMap::new();
    for &i in &kept_pos {
        let r = &model.rows[i];
        for s in [r.home_slot, r.away_slot] {
            if chosen.contains(&s) {
                occupied.entry((season_day(r.month, r.day), s)).or_insert(i);
            }
        }
    }
    // Units: runs of one matchday's games between the same two retained rows.
    let region_of = |pos: usize| kept_pos.partition_point(|k| *k < pos);
    let mut units: Vec<(usize, usize, std::ops::Range<usize>)> = Vec::new();
    for (g, &(m, _, _)) in games.iter().enumerate() {
        let r = region_of(free[g]);
        match units.last_mut() {
            Some((um, ur, range)) if *um == m && *ur == r => range.end = g + 1,
            _ => units.push((m, r, g..g + 1)),
        }
    }
    let start_sd = season_day(start.0, start.1);
    let mut dates = vec![0u64; games.len()];
    let mut prev: Option<(usize, u64)> = None;
    let (mut held_for_order, mut moved_off) = (Vec::<String>::new(), Vec::<String>::new());
    for (m, region, range) in &units {
        let natural = match prev {
            None => start_sd,
            Some((pm, pd)) if pm == *m => pd,
            Some((_, pd)) => pd + spacing as u64,
        };
        let lower = (*region > 0).then(|| {
            let k = kept_pos[region - 1];
            season_day(model.rows[k].month, model.rows[k].day)
        });
        let mut date = natural.max(lower.unwrap_or(0));
        if date > natural {
            held_for_order.push(format!("{} → {} (after {})", season_day_label(natural), season_day_label(date), describe(kept_pos[region - 1])));
        }
        let before_bump = date;
        let mut blocker: Option<usize> = None;
        while let Some(k) = games[range.clone()].iter().find_map(|&(_, h, a)| occupied.get(&(date, h)).or_else(|| occupied.get(&(date, a))).copied()) {
            blocker.get_or_insert(k);
            date += 1;
        }
        if let Some(b) = blocker {
            moved_off.push(format!("{} → {} ({} already plays a kept game: {})", season_day_label(before_bump), season_day_label(date), selected_club(b), describe(b)));
        }
        if let Some(&k) = kept_pos.get(*region) {
            let ud = season_day(model.rows[k].month, model.rows[k].day);
            if date > ud {
                let why = match blocker {
                    Some(b) if before_bump <= ud => format!(
                        "every such date is taken for a club of that matchday — {} already plays a kept game on {} ({})",
                        selected_club(b),
                        season_day_label(before_bump),
                        describe(b)
                    ),
                    _ => format!(
                        "from {} every {} day(s) the {} generated game(s) placed before it run to {}",
                        date_label(start.0, start.1),
                        spacing,
                        free.partition_point(|p| *p < k),
                        season_day_label(date)
                    ),
                };
                let advice = if spacing > 1 { "start earlier, use a shorter spacing, or change the participants" } else { "start earlier or change the participants" };
                return Err(format!(
                    "{} involves a club outside the participants, so it keeps its place and every value; the generated games before it must be dated no later than {}, but {why}. To generate, {advice}; Studio does not move a kept row",
                    describe(k),
                    date_label(model.rows[k].month, model.rows[k].day)
                ));
            }
        }
        if date > 364 {
            return Err(format!(
                "the matchdays from {} every {} day(s), with the dates held for the kept rows, would cross the September season boundary; start earlier or shorten the spacing",
                date_label(start.0, start.1),
                spacing
            ));
        }
        for g in range.clone() {
            dates[g] = date;
        }
        prev = Some((*m, date));
    }
    let mut out = model.rows.clone();
    for (g, &(_, h, a)) in games.iter().enumerate() {
        let (month, day) = from_season_day(dates[g]);
        let pos = free[g];
        out[pos] = GameRow { index: model.rows[pos].index, month, day, home_slot: h, away_slot: a, round: 0, status: if has_status { Some(0) } else { None } };
    }
    // Double bookings: introduced (a generated row on a used club-date) must be none; those among
    // retained rows are the source's own.
    let kept_set: BTreeSet<usize> = kept_pos.iter().copied().collect();
    let mut seen: BTreeMap<(u64, u64), bool> = BTreeMap::new();
    let (mut introduced, mut pre_existing) = (0usize, 0usize);
    for (i, r) in out.iter().enumerate() {
        let generated = !kept_set.contains(&i);
        for s in [r.home_slot, r.away_slot] {
            let key = (season_day(r.month, r.day), s);
            match seen.get_mut(&key) {
                Some(any_generated) => {
                    if generated || *any_generated {
                        introduced += 1;
                    } else {
                        pre_existing += 1;
                    }
                    *any_generated |= generated;
                }
                None => {
                    seen.insert(key, generated);
                }
            }
        }
    }
    if introduced > 0 {
        return Err(format!("internal: the placement would put a club twice on one date ({introduced} time(s)); refused"));
    }
    let mut notes = vec![format!(
        "{} row(s) involving a non-participant keep their position and every stored value (index, date, clubs, round, status); the {} generated game(s) fill the other rows in date order and keep each row's index",
        kept_pos.len(),
        games.len()
    )];
    let list = |v: &[String]| {
        let shown: Vec<&str> = v.iter().take(8).map(|s| s.as_str()).collect();
        format!("{}{}", shown.join("; "), if v.len() > shown.len() { format!("; … {} more", v.len() - shown.len()) } else { String::new() })
    };
    if !held_for_order.is_empty() {
        notes.push(format!("{} matchday(s) moved later to keep the table in date order around a kept row: {}", held_for_order.len(), list(&held_for_order)));
    }
    if !moved_off.is_empty() {
        notes.push(format!("{} matchday(s) moved off a date where one of their clubs already plays a kept game: {}", moved_off.len(), list(&moved_off)));
    }
    if pre_existing > 0 {
        notes.push(format!("{pre_existing} club-date double booking(s) among the kept rows are already in the source table and are left as they are (not introduced by this generation)"));
    }
    Ok((out, notes))
}

/// A generation replaces every row between two participants and writes `round` / `status` 0. A
/// row it would replace that carries a non-zero value (R114: `austrianschedule` on PS4 18–21,
/// `round = 1` on its final within-group block; the runtime meaning is not established) is not
/// silently rewritten: the generation is refused and the reason names the rows.
fn round_tag_refusal(model: &ScheduleModel, chosen: &BTreeSet<u64>) -> Option<String> {
    let tagged: Vec<&GameRow> = model
        .rows
        .iter()
        .filter(|r| chosen.contains(&r.home_slot) && chosen.contains(&r.away_slot) && (r.round != 0 || r.status.unwrap_or(0) != 0))
        .collect();
    if tagged.is_empty() {
        return None;
    }
    let rounds: BTreeSet<u64> = tagged.iter().map(|r| r.round).collect();
    Some(format!(
        "{} of the rows this generation would replace carry a non-zero round / status (round values {:?}, from game {}); research R114 measured such a block on the PS4 18–21 Austrian table (a final within-group phase tagged round = 1) but its runtime meaning is not established, and a generation writes 0 — refused rather than rewrite it. Editing single games and importing a file keep each row's round.",
        tagged.len(),
        rounds,
        tagged[0].index
    ))
}

/// A refused generation plan (nothing proposed).
fn refused(model: &ScheduleModel, errors: Vec<String>) -> GenerationPlan {
    GenerationPlan {
        change: ChangePlan {
            replacement: None,
            studio_table: model.summary.studio_table.clone(),
            table_name: model.summary.name.clone(),
            league_label: model.summary.league_label.clone(),
            rows: Vec::new(),
            cells: 0,
            before: Vec::new(),
            after: Vec::new(),
            validation: Validation { errors, ..Default::default() },
            fingerprint: model.slots.fingerprint.clone(),
            notes: Vec::new(),
        },
        participants: 0,
        rounds: 0,
        matchdays: 0,
        generated_rows: 0,
        kept_rows: 0,
        games_per_participant: 0,
    }
}

fn team_label(model: &ScheduleModel, team: u64) -> String {
    model.slots.by_team(team).map(|e| e.abbrev.clone()).filter(|a| !a.is_empty()).unwrap_or_else(|| format!("team {team}"))
}

/// Receipt 48 — R123 Q5: the groups cover the participants exactly once, no group is empty, nothing outside the
/// participants; then `within` / `across` become the square pair-quota matrix in the participants' order.
fn groups_to_matrix(model: &ScheduleModel, participants: &[u64], g: &GroupQuotas) -> std::result::Result<Vec<Vec<u32>>, Vec<String>> {
    let mut errors = Vec::new();
    let mut group_of: BTreeMap<u64, usize> = BTreeMap::new();
    if g.groups.is_empty() {
        errors.push("add at least one group".into());
    }
    for (k, grp) in g.groups.iter().enumerate() {
        if grp.is_empty() {
            errors.push(format!("group {} is empty; every group needs at least one club", k + 1));
        }
        for t in grp {
            if let Some(prev) = group_of.insert(*t, k) {
                errors.push(format!(
                    "{} is in group {} and group {}; every participating club must be in exactly one group",
                    team_label(model, *t),
                    prev + 1,
                    k + 1
                ));
            }
            if !participants.contains(t) {
                errors.push(format!("{} is in group {} but is not a participant", team_label(model, *t), k + 1));
            }
        }
    }
    for t in participants {
        if !group_of.contains_key(t) {
            errors.push(format!("{} participates but is in no group; every participating club must be in exactly one group", team_label(model, *t)));
        }
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    let n = participants.len();
    Ok((0..n)
        .map(|i| {
            (0..n)
                .map(|j| if i == j { 0 } else if group_of[&participants[i]] == group_of[&participants[j]] { g.within } else { g.across })
                .collect()
        })
        .collect())
}

fn group_note(model: &ScheduleModel, g: &GroupQuotas) -> String {
    format!(
        "groups{}: {} — {} game(s) per pair inside a group, {} across groups (converted into the games-per-pair table below and checked against it after building)",
        g.origin.as_deref().map(|o| format!(" ({o})")).unwrap_or_default(),
        g.groups
            .iter()
            .enumerate()
            .map(|(k, grp)| format!("{}: {}", k + 1, grp.iter().map(|t| team_label(model, *t)).collect::<Vec<_>>().join(" · ")))
            .collect::<Vec<_>>()
            .join("; "),
        g.within,
        g.across
    )
}

/// Receipt 48 — the built proposal checked **independently** of how it was built (R123 R1–R9, with the production
/// corrections of the 28 Sep review): row count; kept rows unchanged in value and position; every stored index
/// unchanged (R123's `index == position` describes its stock replay, not a licence to rewrite a non-updatable
/// key); per-pair counts equal the request, zero pairs included; the home / away split of every pair (even:
/// equal; odd: the reported host has one more); generated rows only between two different participants, round
/// 0; no club twice on one date among generated rows, nor a generated game on a date its club already plays a
/// kept game. Returns every failure.
fn check_generated(
    model: &ScheduleModel,
    proposed: &[GameRow],
    seats: &[&SlotEntry],
    q: &[Vec<u32>],
    odd_host: &BTreeMap<(usize, usize), usize>,
) -> Vec<String> {
    let mut f = Vec::new();
    if supports_resize(model) {
        let chosen:BTreeSet<_>=seats.iter().map(|s|s.slot).collect();
        let kept=|rows:&[GameRow]|{let mut rs:Vec<_>=rows.iter().filter(|r|!(chosen.contains(&r.home_slot)&&chosen.contains(&r.away_slot))).map(|r|(r.month,r.day,r.home_slot,r.away_slot,r.round,r.status)).collect();rs.sort();rs};
        if kept(&model.rows)!=kept(proposed){f.push("A non-participating fixture changed.".into());}
    }
    if !supports_resize(model) && proposed.len() != model.rows.len() {
        f.push(format!("R1: {} rows proposed for a table of {}", proposed.len(), model.rows.len()));
        return f;
    }
    let seat_of: BTreeMap<u64, usize> = seats.iter().enumerate().map(|(i, e)| (e.slot, i)).collect();
    let chosen: BTreeSet<u64> = seat_of.keys().copied().collect();
    let n = seats.len();
    let mut count = vec![vec![0u32; n]; n];
    let mut home = vec![vec![0u32; n]; n];
    let mut kept_dates: BTreeSet<(u64, u64, u64)> = BTreeSet::new();
    let mut gen_dates: BTreeSet<(u64, u64, u64)> = BTreeSet::new();
    let audit_rows=if supports_resize(model){proposed}else{&model.rows};
    for (pos, (before, after)) in audit_rows.iter().zip(proposed).enumerate() {
        if after.index != before.index {
            f.push(format!("R3: row {pos} index {} became {}", before.index, after.index));
        }
        let kept = !(chosen.contains(&before.home_slot) && chosen.contains(&before.away_slot));
        if kept {
            if after != before {
                f.push(format!("R2: kept row {pos} changed"));
            }
            for s in [before.home_slot, before.away_slot] {
                if chosen.contains(&s) {
                    kept_dates.insert((s, before.month, before.day));
                }
            }
            continue;
        }
        let (Some(&h), Some(&a)) = (seat_of.get(&after.home_slot), seat_of.get(&after.away_slot)) else {
            f.push(format!("R6: generated row {pos} involves a non-participant"));
            continue;
        };
        if h == a {
            f.push(format!("R6: generated row {pos} is a self game"));
            continue;
        }
        if after.round != 0 {
            f.push(format!("R7: generated row {pos} carries round {}", after.round));
        }
        count[h.min(a)][h.max(a)] += 1;
        home[h][a] += 1;
        for s in [after.home_slot, after.away_slot] {
            if !gen_dates.insert((s, after.month, after.day)) {
                f.push(format!("R9: slot {s} plays twice on {} among generated games", date_label(after.month, after.day)));
            }
        }
    }
    for d in &gen_dates {
        if kept_dates.contains(d) {
            f.push(format!("R9: a generated game of slot {} lands on {}, where it already plays a kept game", d.0, date_label(d.1, d.2)));
        }
    }
    for i in 0..n {
        for j in i + 1..n {
            if count[i][j] != q[i][j] {
                f.push(format!("R4: {} v {} play {} game(s); {} requested", seats[i].abbrev, seats[j].abbrev, count[i][j], q[i][j]));
                continue;
            }
            let (hi, hj) = (home[i][j], home[j][i]);
            if q[i][j] % 2 == 0 {
                if hi != hj {
                    f.push(format!("R5: {} v {} split {hi}–{hj} for an even count", seats[i].abbrev, seats[j].abbrev));
                }
            } else {
                let host = odd_host.get(&(i, j)).copied();
                let ok = match host {
                    Some(x) if x == i => hi == hj + 1,
                    Some(_) => hj == hi + 1,
                    None => false,
                };
                if !ok {
                    f.push(format!("R5: {} v {} split {hi}–{hj}; the reported host does not have the extra home game", seats[i].abbrev, seats[j].abbrev));
                }
            }
        }
    }
    f
}

/// Receipt 34 — the **user-supplied pair-quota** generator (research R114 §2, "user-supplied"):
/// `cfg.pair_quotas[i][j]` games between `participants[i]` and `participants[j]`. Refuses, with the
/// numbers and before anything is written: a non-square, asymmetric or non-zero-diagonal matrix; a
/// total (upper-triangle sum) that differs from the rows to fill (the table's active rows minus the
/// rows kept because they involve a non-participant — Studio never changes a row count); a replaced
/// round-tagged row; dates past the season. Home / away: each pair splits its games evenly; an odd
/// count gives the extra home game to the club with fewer home games so far (then the lower slot) —
/// every such pair is listed in the notes. Matchdays: meetings are queued by meeting number, then
/// pair order, and packed greedily so no club plays twice on one matchday; one date per matchday,
/// `days_between_matchdays` apart. Deterministic for the same input.
fn plan_quota_generation(model: &ScheduleModel, cfg: &GenerationConfig) -> Result<GenerationPlan> {
    let q = cfg.pair_quotas.as_ref().expect("pair quotas");
    let mut errors = Vec::new();
    let mut seen = BTreeSet::new();
    let mut seats: Vec<&SlotEntry> = Vec::new();
    for t in &cfg.participants {
        if !seen.insert(*t) {
            errors.push(format!("team id {t} is listed twice"));
            continue;
        }
        match model.slots.by_team(*t) {
            Some(e) => seats.push(e),
            None => errors.push(format!("team id {t} is not a member of {} on this title", model.summary.league_label.as_deref().unwrap_or("this league"))),
        }
    }
    let n = cfg.participants.len();
    if n < 2 {
        errors.push("choose at least two participants".into());
    }
    if q.len() != n || q.iter().any(|row| row.len() != n) {
        errors.push(format!("the pair-quota matrix must be {n} × {n} (one row and column per participant, in the participants' order)"));
    }
    if errors.is_empty() {
        for i in 0..n {
            if q[i][i] != 0 {
                errors.push(format!("{} cannot play itself: the diagonal must be 0 (row {} holds {})", seats[i].name, i + 1, q[i][i]));
            }
            for j in i + 1..n {
                if q[i][j] != q[j][i] {
                    errors.push(format!("the matrix is not symmetric: {} v {} = {} but {} v {} = {}", seats[i].name, seats[j].name, q[i][j], seats[j].name, seats[i].name, q[j][i]));
                }
            }
        }
    }
    if cfg.start_month > 11 || cfg.start_day + 1 > MONTH_LEN.get(cfg.start_month as usize).copied().unwrap_or(0) {
        errors.push(format!("start date {} does not exist", date_label(cfg.start_month, cfg.start_day)));
    }
    if cfg.days_between_matchdays == 0 {
        errors.push("days between matchdays must be at least 1".into());
    }
    let chosen: BTreeSet<u64> = seats.iter().map(|e| e.slot).collect();
    let kept = kept_rows(model, &chosen);
    let rows_to_fill = model.rows.len() - kept.len();
    let total: u64 = if errors.is_empty() { (0..n).flat_map(|i| (i + 1..n).map(move |j| (i, j))).map(|(i, j)| q[i][j] as u64).sum() } else { 0 };
    if errors.is_empty() {
        for i in 0..n {
            if (0..n).all(|j| q[i][j] == 0) {
                errors.push(format!("{} would have no games (R123 Q12): give it at least one pair or leave it out of the participants", seats[i].name));
            }
        }
        let odd = (0..n).any(|i| (i + 1..n).any(|j| q[i][j] % 2 == 1));
        if odd && cfg.odd_home.is_none() {
            errors.push("some pairs play an odd number of games: choose which club hosts the extra game (lower slot, higher slot, balanced or per pair) (R123 Q11)".into());
        }
        // Receipt 48: explicit hosts must name every odd pair exactly once, a club of that pair, and nothing else.
        match (cfg.odd_home, &cfg.odd_hosts) {
            (Some(OddHome::Explicit), None) => errors.push("per-pair hosts were chosen but none were given".into()),
            (Some(OddHome::Explicit), Some(hosts)) => {
                let pos: BTreeMap<u64, usize> = cfg.participants.iter().enumerate().map(|(k, t)| (*t, k)).collect();
                let mut named: BTreeSet<(usize, usize)> = BTreeSet::new();
                for (h, a) in hosts {
                    let (Some(&i), Some(&j)) = (pos.get(h), pos.get(a)) else {
                        errors.push(format!("the host list names {} v {}, which is not a pair of participants", team_label(model, *h), team_label(model, *a)));
                        continue;
                    };
                    let key = (i.min(j), i.max(j));
                    if i == j || q[key.0][key.1] % 2 == 0 {
                        errors.push(format!("the host list names {} v {}, which is not a pair with an odd number of games", seats[i].name, seats[j].name));
                    } else if !named.insert(key) {
                        errors.push(format!("the host list names {} v {} twice", seats[key.0].name, seats[key.1].name));
                    }
                }
                for i in 0..n {
                    for j in i + 1..n {
                        if q[i][j] % 2 == 1 && !named.contains(&(i, j)) {
                            errors.push(format!("choose the host of the extra {} v {} game", seats[i].name, seats[j].name));
                        }
                    }
                }
            }
            (_, Some(h)) if !h.is_empty() => errors.push("per-pair hosts were given, but the host rule is not \"per pair\"".into()),
            _ => {}
        }
    }
    if supports_resize(model) && total + kept.len() as u64>8192 {errors.push("A schedule cannot contain more than 8192 games.".into());}
    if errors.is_empty() && !supports_resize(model) && total != rows_to_fill as u64 {
        errors.push(format!(
            "the matrix asks for {total} games (the sum above the diagonal), but {} holds {} active rows of which {} are kept (they involve a non-participant), leaving {rows_to_fill} to fill; Studio does not change a schedule table's row count",
            model.summary.name,
            model.rows.len(),
            kept.len()
        ));
    }
    errors.extend(round_tag_refusal(model, &chosen));
    let empty = |errors: Vec<String>, matchdays: u32| GenerationPlan {
        change: ChangePlan {
            replacement: None,
            studio_table: model.summary.studio_table.clone(),
            table_name: model.summary.name.clone(),
            league_label: model.summary.league_label.clone(),
            rows: Vec::new(),
            cells: 0,
            before: Vec::new(),
            after: Vec::new(),
            validation: Validation { errors, ..Default::default() },
            fingerprint: model.slots.fingerprint.clone(),
            notes: Vec::new(),
        },
        participants: n,
        rounds: 0,
        matchdays,
        generated_rows: total as usize,
        kept_rows: kept.len(),
        games_per_participant: 0,
    };
    if !errors.is_empty() {
        return Ok(empty(errors, 0));
    }
    // Venues: even split; odd pairs give the extra home game by the chosen rule.
    let mut home_left = vec![vec![0u32; n]; n];
    let mut home_total = vec![0u32; n];
    for i in 0..n {
        for j in i + 1..n {
            home_left[i][j] = q[i][j] / 2;
            home_left[j][i] = q[i][j] / 2;
            home_total[i] += q[i][j] / 2;
            home_total[j] += q[i][j] / 2;
        }
    }
    let mut extra_home = Vec::new();
    let mut odd_host: BTreeMap<(usize, usize), usize> = BTreeMap::new();
    let explicit: BTreeMap<(u64, u64), u64> = cfg
        .odd_hosts
        .iter()
        .flatten()
        .map(|&(h, a)| ((h.min(a), h.max(a)), h))
        .collect();
    for i in 0..n {
        for j in i + 1..n {
            if q[i][j] % 2 == 1 {
                let lower = if seats[i].slot < seats[j].slot { i } else { j };
                let higher = if lower == i { j } else { i };
                let h = match cfg.odd_home.expect("checked above") {
                    OddHome::LowerSlot => lower,
                    OddHome::HigherSlot => higher,
                    OddHome::Balance => {
                        if (home_total[i], seats[i].slot) <= (home_total[j], seats[j].slot) {
                            i
                        } else {
                            j
                        }
                    }
                    OddHome::Explicit => {
                        let (ti, tj) = (cfg.participants[i], cfg.participants[j]);
                        if explicit.get(&(ti.min(tj), ti.max(tj))) == Some(&ti) {
                            i
                        } else {
                            j
                        }
                    }
                };
                odd_host.insert((i, j), h);
                let a = if h == i { j } else { i };
                home_left[h][a] += 1;
                home_total[h] += 1;
                extra_home.push(format!("{} (over {})", seats[h].name, seats[a].name));
            }
        }
    }
    // Meeting queue: meeting k of every pair with more than k games, pairs in participant order;
    // the venue goes to the club with more home games left in the pair (ties alternate).
    let max_q = q.iter().flatten().copied().max().unwrap_or(0);
    let mut queue: Vec<(usize, usize)> = Vec::with_capacity(total as usize);
    for k in 0..max_q {
        for i in 0..n {
            for j in i + 1..n {
                if q[i][j] > k {
                    let home = match home_left[i][j].cmp(&home_left[j][i]) {
                        std::cmp::Ordering::Greater => i,
                        std::cmp::Ordering::Less => j,
                        std::cmp::Ordering::Equal => if k % 2 == 0 { i } else { j },
                    };
                    let away = if home == i { j } else { i };
                    home_left[home][away] -= 1;
                    queue.push((home, away));
                }
            }
        }
    }
    // Greedy matchdays: each takes, in queue order, every game whose two clubs are still free.
    let mut days: Vec<Vec<(usize, usize)>> = Vec::new();
    let mut pending = queue;
    while !pending.is_empty() {
        let mut busy = vec![false; n];
        let mut today = Vec::new();
        let mut rest = Vec::with_capacity(pending.len());
        for (h, a) in pending {
            if !busy[h] && !busy[a] {
                busy[h] = true;
                busy[a] = true;
                today.push((h, a));
            } else {
                rest.push((h, a));
            }
        }
        days.push(today);
        pending = rest;
    }
    let matchdays = days.len() as u32;
    let span_days = (matchdays.saturating_sub(1) as u64) * cfg.days_between_matchdays as u64;
    if span_days >= 365 {
        return Ok(empty(vec![format!("{matchdays} matchdays every {} day(s) span {span_days} days — past one season; shorten the spacing", cfg.days_between_matchdays)], matchdays));
    }
    let (em, ed) = add_days(cfg.start_month, cfg.start_day, span_days);
    if season_key(em, ed) < season_key(cfg.start_month, cfg.start_day) {
        return Ok(empty(
            vec![format!(
                "the last matchday ({}) would cross the September season boundary from a {} start; shorten the spacing or start earlier",
                date_label(em, ed),
                date_label(cfg.start_month, cfg.start_day)
            )],
            matchdays,
        ));
    }
    let slot_days: Vec<Vec<(u64, u64)>> = days.iter().map(|d| d.iter().map(|&(h, a)| (seats[h].slot, seats[a].slot)).collect()).collect();
    let (proposed, placement_notes) = match place_generated(model, &chosen, &slot_days, (cfg.start_month, cfg.start_day), cfg.days_between_matchdays) {
        Ok(p) => p,
        Err(e) => return Ok(empty(vec![e], matchdays)),
    };
    // Receipt 48: never plan a write the independent check does not accept.
    let check = check_generated(model, &proposed, &seats, q, &odd_host);
    if !check.is_empty() {
        let mut e = vec!["internal check failed; nothing was proposed. Please report this:".to_string()];
        e.extend(check);
        return Ok(empty(e, matchdays));
    }
    let validation = validate(model, &proposed, None);
    let games: Vec<u32> = (0..n).map(|i| (0..n).map(|j| q[i][j]).sum()).collect();
    let mut notes = vec![
        format!(
            "user-supplied pair quotas: {n} participants, {total} games, {matchdays} matchdays from {} every {} day(s); {} kept row(s); no club plays twice on a date",
            date_label(cfg.start_month, cfg.start_day),
            cfg.days_between_matchdays,
            kept.len()
        ),
        format!("games per participant: {}", seats.iter().zip(&games).map(|(s, g)| format!("{} {g}", s.abbrev)).collect::<Vec<_>>().join(", ")),
        format!(
            "home / away: each pair splits its games evenly; an odd count gives the extra home game to {}",
            match cfg.odd_home {
                Some(OddHome::LowerSlot) => "the club at the lower league slot (your choice)",
                Some(OddHome::HigherSlot) => "the club at the higher league slot (your choice)",
                Some(OddHome::Balance) => "the club of the pair with fewer home games so far (pairs in participant order), then the lower slot (your choice)",
                Some(OddHome::Explicit) => "the club you named for each odd pair",
                None => "— (no odd pair)",
            }
        ),
        "the quotas are the user's choice: no stock partition, NHL pair rule or other title's quotas are inferred, and nothing says the game plays this table as intended (runtime card)".into(),
    ];
    if !extra_home.is_empty() {
        // Every chosen host is reported (receipt 48: the balance rule's choices are visible, not implied).
        let shown: Vec<&String> = extra_home.iter().collect();
        notes.push(format!(
            "extra home game ({} odd pair(s)): {}{}",
            extra_home.len(),
            shown.iter().map(|s| s.as_str()).collect::<Vec<_>>().join("; "),
            if extra_home.len() > shown.len() { format!("; … {} more", extra_home.len() - shown.len()) } else { String::new() }
        ));
    }
    notes.extend(placement_notes);
    notes.extend(sibling_note(model));
    let change = diff_to_plan(model, &proposed, validation, notes);
    let per = games.iter().copied().max().unwrap_or(0);
    Ok(GenerationPlan { change, participants: n, rounds: 0, matchdays, generated_rows: total as usize, kept_rows: kept.len(), games_per_participant: per })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn season_dates_wrap_and_add() {
        assert!(season_key(9, 6) < season_key(0, 0));
        assert!(season_key(2, 30) < season_key(3, 8));
        assert_eq!(add_days(9, 30, 1), (10, 0));
        assert_eq!(add_days(11, 30, 1), (0, 0));
        assert_eq!(add_days(1, 27, 1), (2, 0), "February is 28 days (yearless)");
        assert_eq!(season_day(8, 0), 0);
        assert_eq!(season_day(9, 0), 30);
    }

    #[test]
    fn circle_method_pairs_everyone_once_per_cycle() {
        for n in [2usize, 3, 4, 7, 14, 30, 31] {
            let days = circle_matchdays(n);
            assert_eq!(days.len(), if n % 2 == 0 { n - 1 } else { n });
            let mut pairs = BTreeSet::new();
            for day in &days {
                let mut seen = BTreeSet::new();
                for &(a, b) in day {
                    assert!(a < b && b < n);
                    assert!(seen.insert(a) && seen.insert(b), "a seat plays twice on one matchday");
                    assert!(pairs.insert((a, b)), "pair repeated within a cycle");
                }
            }
            assert_eq!(pairs.len(), n * (n - 1) / 2);
        }
    }

    #[test]
    fn csv_and_json_parse_and_refuse_malformed_input() {
        let csv = "# nhl-studio-schedule/1 mapping_fingerprint 0123abcd x\nindex,date,month,day,home,away,round,status\n0,Oct 7,10,7,TOR,MTL,0,0\n1,,10,8,mtl,BOS,,\n";
        let d = parse_import(csv, ImportFormat::Csv).unwrap();
        assert_eq!(d.games.len(), 2);
        assert_eq!((d.games[0].month, d.games[0].day), (9, 6));
        assert_eq!(d.keys, ["TOR", "MTL", "BOS"]);
        assert_eq!(d.declared_fingerprint.as_deref(), Some("0123abcd"));
        assert!(d.games[1].round.is_none());
        let e = parse_import("month,day,home\n10,7,TOR\n", ImportFormat::Csv).unwrap_err().to_string();
        assert!(e.contains("header row"), "{e}");
        let e = parse_import("month,day,home,away\n13,7,TOR,MTL\n", ImportFormat::Csv).unwrap_err().to_string();
        assert!(e.contains("line 2") && e.contains("month 13"), "{e}");
        let e = parse_import("month,day,home,away\n10,x,TOR,MTL\n", ImportFormat::Csv).unwrap_err().to_string();
        assert!(e.contains("not a whole number"), "{e}");
        let e = parse_import("month,day,home,away\n", ImportFormat::Csv).unwrap_err().to_string();
        assert!(e.contains("no games"), "{e}");
        let j = r#"{"format":"nhl-studio-schedule/1","games":[{"month":10,"day":7,"home":"TOR","away":{"key":"MTL"}}]}"#;
        let d = parse_import(j, ImportFormat::Json).unwrap();
        assert_eq!(d.games[0].away_key, "MTL");
        let e = parse_import("{\"games\": 3}", ImportFormat::Json).unwrap_err().to_string();
        assert!(e.contains("\"games\" array"), "{e}");
        let e = parse_import("{\"format\":\"other/9\",\"games\":[]}", ImportFormat::Json).unwrap_err().to_string();
        assert!(e.contains("unsupported format"), "{e}");
        let e = parse_import("not json", ImportFormat::Json).unwrap_err().to_string();
        assert!(e.contains("not valid JSON"), "{e}");
    }
}
