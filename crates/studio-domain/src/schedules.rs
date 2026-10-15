//! Phase 1.2 — source schedules: browse and validate the per-league schedule tables that ship
//! inside the main database (research handoff P1.2-X, 2026-09-18). One table per league
//! (`nhlschedule`, `ahlschedule`, `swedishschedule`, …), one row per game with a 0-indexed
//! `month`/`day`, a running `index`, and `home`/`away` as the team's **position within its
//! league** — its rank among the league's members ordered by the team table's global `index`
//! field — never the team id. The NHL table carries an
//! All-Star row on the titles that have one (slots beyond the member count, 1 game each).
//!
//! Everything here is derived from the metadata names (`round`/`status`/`day`/`month`/`index`/
//! `home`/`away` — the studio ids differ per family), so it works on every anchor without a per-
//! title table map. Which league a table belongs to is matched by the table's name stem against
//! the catalog's league keys/labels and disclosed as such (`league_match`); a stem the catalog
//! cannot place stays a browseable table with no league. Editing a game is an ordinary cell edit
//! on the schedule table's record (the fixed-bit fields round-trip through `tdb-edit`); nothing
//! here writes, and nothing claims how a mode copies the source table into
//! `leaguescheduletable` — that is a runtime/save question.

use std::collections::BTreeMap;

use tdb_core::{FieldDescriptor, Table, TdbFile};
use tdb_defs::FieldMetaIndex;

use crate::leagues::LeagueCatalog;
use crate::Result;

/// One schedule table in the database.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleTableSummary {
    /// Studio (canonical) table id.
    pub studio_table: String,
    /// Metadata name (`nhlschedule`).
    pub name: String,
    pub rows: u32,
    pub max_records: u32,
    /// The league this table is believed to serve, when the name stem matches a catalogued
    /// league; `None` for mode-local/debug/future tables and unmatched stems.
    pub league_code: Option<u64>,
    pub league_label: Option<String>,
    /// How the league was matched (`stem:nhl → key NHL` …) or why not.
    pub league_match: String,
    /// `source` (a league's stock season), `future`, `debug`, `favorite`, `mode-local`, `variant`.
    pub role: &'static str,
    /// Tournament templates only (R115): the tournament's league, **for display grouping** — the
    /// table is shown on that league's page. Never a slot binding: `league_code` stays `None`, the
    /// slot map stays empty and the table stays browse-only.
    pub shown_with_league: Option<u64>,
}

/// One game row, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduledGame {
    pub record: u32,
    pub index: u64,
    /// 0-indexed as stored.
    pub month: u64,
    pub day: u64,
    /// `YYYY-MM-DD`-free display: `Oct 7` style (the season year is not in the table).
    pub date: String,
    pub round: u64,
    pub status: Option<u64>,
    pub home_slot: u64,
    pub away_slot: u64,
    pub home: Option<String>,
    pub away: Option<String>,
    /// Tournament tables only (research R115, 26 Sep 2026): a read-only label for the round and for
    /// each slot — "group stage", seed counts, "to be decided", a World Cup club by the one
    /// consistent join, or a labelled interpretation. `None` elsewhere. Never a binding: the
    /// table stays unbound and browse-only.
    pub round_label: Option<String>,
    pub home_label: Option<String>,
    pub away_label: Option<String>,
}

/// The three tournament templates R115 analysed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tournament {
    WorldCup,
    Champions,
    Spengler,
}

fn tournament_of(name: &str) -> Option<Tournament> {
    match name {
        "wcschedule" => Some(Tournament::WorldCup),
        "championsschedule" => Some(Tournament::Champions),
        "spenglerschedule" => Some(Tournament::Spengler),
        _ => None,
    }
}

/// The league a tournament template is shown with, by its exact catalogue key (R115: `WorldCup`,
/// `TXT_CHAMPIONS_LEAGUE`, `TXT_SPENGLER_CUP`) — display grouping only.
fn tournament_league(kind: Tournament, catalog: &LeagueCatalog) -> Option<u64> {
    let key = match kind {
        Tournament::WorldCup => "WorldCup",
        Tournament::Champions => "TXT_CHAMPIONS_LEAGUE",
        Tournament::Spengler => "TXT_SPENGLER_CUP",
    };
    catalog
        .leagues
        .iter()
        .find(|l| [l.name_key.as_deref(), l.dictionary_key.as_deref()].iter().flatten().any(|k| k.eq_ignore_ascii_case(key)))
        .map(|l| l.code)
}

/// Browse labels for a tournament table (R115 §2 / §6): round labels, participant / placeholder /
/// seed slot labels, and the World Cup group stage by club name (J1: slot = rank by `index` among
/// the World Cup league's members — agrees with the group ids on the one source). Returns the note
/// shown with the table.
fn label_tournament(
    kind: Tournament,
    games: &mut [ScheduledGame],
    wc_names: &BTreeMap<u64, String>,
) -> String {
    // Seeds per knockout round = distinct slot ids used in that round.
    let mut per_round: BTreeMap<u64, std::collections::BTreeSet<u64>> = BTreeMap::new();
    for g in games.iter() {
        let e = per_round.entry(g.round).or_default();
        e.insert(g.home_slot);
        e.insert(g.away_slot);
    }
    // Spengler R-WL: the day-1 group-stage games in index order are the openers k = 0, 1, …;
    // ids 6 + 2k and 7 + 2k are "winner / loser of opener k" (which is which is unresolved).
    let mut openers: Vec<(u64, u64)> = Vec::new();
    if kind == Tournament::Spengler {
        let mut stage0: Vec<&ScheduledGame> = games.iter().filter(|g| g.round == 0).collect();
        stage0.sort_by_key(|g| (g.month, g.day, g.index));
        if let Some(first) = stage0.first().map(|g| (g.month, g.day)) {
            openers = stage0
                .iter()
                .filter(|g| (g.month, g.day) == first && g.home_slot < 6 && g.away_slot < 6)
                .map(|g| (g.home_slot, g.away_slot))
                .collect();
        }
    }
    let round_label = |round: u64| -> String {
        if round == 0 {
            return "group stage".into();
        }
        let seeds = per_round.get(&round).map(|s| s.len()).unwrap_or(0);
        match (kind, seeds) {
            (Tournament::WorldCup, _) => "knockout round — teams to be decided".into(),
            (_, 16) => "knockout round, 16 seeds (round-of-16 shape)".into(),
            (_, 8) => "knockout round, 8 seeds (quarter-final shape)".into(),
            (_, 4) => "knockout round, 4 seeds (semi-final shape)".into(),
            (_, 2) => "knockout round, 2 seeds (final)".into(),
            (_, n) => format!("knockout round, {n} seeds"),
        }
    };
    let slot_label = |round: u64, slot: u64| -> String {
        match kind {
            Tournament::WorldCup if round == 0 => match wc_names.get(&slot) {
                Some(n) => n.clone(),
                None => format!("participant slot {slot}"),
            },
            Tournament::WorldCup => "to be decided".into(),
            Tournament::Champions if round == 0 => format!("participant slot {slot}"),
            Tournament::Spengler if round == 0 && slot < 6 => format!("participant slot {slot}"),
            Tournament::Spengler if round == 0 => {
                let k = ((slot - 6) / 2) as usize;
                match openers.get(k) {
                    Some((a, b)) => format!("winner or loser of the slot {a} v slot {b} opener (interpretation)"),
                    None => format!("placeholder {slot} (interpretation)"),
                }
            }
            _ => format!("seed {slot}"),
        }
    };
    for g in games.iter_mut() {
        g.round_label = Some(round_label(g.round));
        g.home_label = Some(slot_label(g.round, g.home_slot));
        g.away_label = Some(slot_label(g.round, g.away_slot));
    }
    match kind {
        Tournament::WorldCup => "Tournament template (research R115): group-stage slots are named by the one consistent join — slot = rank by team index among the World Cup league's members, which agrees with the group ids; the knockout games are placeholders (\"to be decided\"). Browse-only: no slot is bound to a team for editing.".into(),
        Tournament::Champions => "Tournament template (research R115): group-stage slots are participant positions; 32 clubs carry the Champions League jersey flag on PS4 18–21, but which club fills which slot is not established, so none is named. Knockout ids are seeds filled at run time (interpretation). Browse-only.".into(),
        Tournament::Spengler => "Tournament template (research R115): slots 0–5 are the six clubs, but two readings of which club sits where disagree (NHL 21), so none is named; 6–9 read as the winner or loser of each group's opening game (interpretation, which is which unresolved); knockout ids are seeds filled at run time (interpretation). Browse-only.".into(),
    }
}

/// Games per league slot, with the team occupying the slot when one does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotCount {
    pub slot: u64,
    pub team: Option<String>,
    pub team_id: Option<u64>,
    pub games: u32,
}

/// One league member at its schedule slot (its rank among the league's members by the team
/// table's global `index`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotEntry {
    pub slot: u64,
    pub team_id: u64,
    pub name: String,
    pub abbrev: String,
    /// The team table's global `index` value (what the rank is derived from — never written into
    /// a schedule row).
    pub global_index: u64,
    /// The team row's `artabbr` (NUL / space trimmed), as research R113's mapping fingerprint reads it — which can
    /// differ from the displayed abbreviation (NHL 07 slot 9: `MOR` vs `MIK`; receipt 48).
    pub art_abbrev: String,
    /// Stable export key: the abbreviation when unique within the league, else `ABBR#<team id>`.
    pub key: String,
}

/// The slot ↔ team map of one league, with a fingerprint that changes whenever the league's
/// membership or ordering changes (so a mapping made against an older state is refused).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SlotMap {
    pub league_code: Option<u64>,
    pub entries: Vec<SlotEntry>,
    /// FNV-1a over `slot:team_id:global_index` of every entry in slot order.
    pub fingerprint: String,
}

impl SlotMap {
    pub fn by_slot(&self, slot: u64) -> Option<&SlotEntry> {
        self.entries.iter().find(|e| e.slot == slot)
    }
    pub fn by_team(&self, team_id: u64) -> Option<&SlotEntry> {
        self.entries.iter().find(|e| e.team_id == team_id)
    }
    pub fn by_key(&self, key: &str) -> Option<&SlotEntry> {
        self.entries.iter().find(|e| e.key.eq_ignore_ascii_case(key))
    }
    pub fn team_name(&self, slot: u64) -> Option<String> {
        self.by_slot(slot).map(|e| e.name.clone())
    }
}

/// One editable field of a schedule row and its actual storage width on this title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleField {
    /// Metadata name (`home`, `away`, `day`, `month`, `index`, `round`, `status`).
    pub name: String,
    pub studio_field: String,
    pub bit_width: u32,
}

/// The game-row layout of one schedule table, by metadata name; widths are read from the
/// table's own field descriptors (5-bit slots on gen7, 6-bit on PS4 18+, 4-bit European …).
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ScheduleLayout {
    pub fields: Vec<ScheduleField>,
}

impl ScheduleLayout {
    pub fn field(&self, name: &str) -> Option<&ScheduleField> {
        self.fields.iter().find(|f| f.name == name)
    }
    /// Largest value a field can hold, from its bit width.
    pub fn max_value(&self, name: &str) -> Option<u64> {
        self.field(name).map(|f| if f.bit_width >= 64 { u64::MAX } else { (1u64 << f.bit_width) - 1 })
    }
    /// `true` when the table carries the day/month/home/away game layout.
    pub fn is_game_layout(&self) -> bool {
        ["day", "month", "home", "away"].iter().all(|n| self.field(n).is_some())
    }
}

/// A schedule table's rows, slot resolution and validation.
#[derive(Debug, Clone)]
pub struct ScheduleView {
    pub table: ScheduleTableSummary,
    /// Studio field ids for the editable cells, by metadata name.
    pub fields: BTreeMap<String, String>,
    /// The same fields with their storage widths.
    pub layout: ScheduleLayout,
    /// Slot ↔ team map of the matched league (empty when no league is matched).
    pub slot_map: SlotMap,
    pub games: Vec<ScheduledGame>,
    pub slots: Vec<SlotCount>,
    /// Slots referenced by games with no team at that league position (All-Star squads or
    /// dangling slots), listed with their game counts.
    pub unresolved_slots: Vec<SlotCount>,
    /// Plain-language findings (empty when everything is consistent).
    pub findings: Vec<String>,
    pub first_date: Option<String>,
    pub last_date: Option<String>,
    /// Tournament templates (R115): what the slot labels rest on and that the table is browse-only.
    pub tournament_note: Option<String>,
}

const MONTHS: [&str; 12] = ["Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec"];

fn date_label(month: u64, day: u64) -> String {
    match MONTHS.get(month as usize) {
        Some(m) => format!("{m} {}", day + 1),
        None => format!("month {month} day {}", day + 1),
    }
}

fn by_name<'a>(meta: &FieldMetaIndex, table: &'a Table, name: &str) -> Option<&'a FieldDescriptor> {
    table.fields.iter().find(|f| {
        meta.field(&table.id.raw(), &f.id.raw())
            .map(|m| m.name == name)
            .unwrap_or(false)
    })
}

/// Map a schedule table name stem to a league of the catalog: by dictionary key / native key /
/// label containment, disclosed. Stems: `nhl`, `ahl`, `ohl`, `qmjhl`, `whl`, `swedish`,
/// `finnish`, `german`, `czech`, `swiss`, `echl`, `austrian`, `swedish2nd`, `champions`, `spengler`,
/// `worldchamp`, `worldjunior`, `intldel` / `intlelitserien` / `intlsmliiga` (NHL 07), `nhl32`,
/// `ahl32`, `ahl31`.
fn match_league(stem: &str, catalog: &LeagueCatalog) -> (Option<u64>, Option<String>, String) {
    // Candidate keys/labels per stem (case-insensitive containment on key or label).
    let wants: &[&str] = match stem {
        "nhl" | "nhl32" => &["NHL"],
        "ahl" | "ahl31" | "ahl32" => &["AHL"],
        "ohl" => &["OHL"],
        "qmjhl" => &["QMJHL"],
        "whl" => &["WHL"],
        "swedish" | "intlelitserien" => &["elitserien", "SHL", "Sweden"],
        "swedish2nd" => &["allsvenskan", "HockeyAllsvenskan"],
        "finnish" | "intlsmliiga" => &["SMLIIGA", "Liiga", "Finland"],
        "german" | "intldel" => &["TXT_LEAGUE_DEL", "DEL", "Germany"],
        "czech" => &["EXTRALIGA", "Extraliga", "Czech"],
        "swiss" => &["SWISS", "National League", "NLA", "Switzerland"],
        "austrian" => &["TXT_LEAGUE_EBEL", "EBEL", "Austria"],
        "echl" => &["ECHL", "TXT_ECHL"],
        // `champions` (NHL 18+) uses 32 slots while the stem-named league has 8 members, and
        // `spengler` uses 12 slots for a 6-member league: the stem binding is contradicted by the
        // data, so both stay unbound (a research question), as do `wc`, `worldchamp`,
        // `worldjunior`.
        "norwegian" => &["TXT_LEAGUE_NORWAY", "Norway"],
        _ => &[],
    };
    if wants.is_empty() {
        return (None, None, format!("stem {stem:?}: no league mapping known"));
    }
    // Exact key matches first (so `NHL` never lands on `TXT_LEAGUE_NHL_ALUMNI`), then
    // containment on the key or label (PS4 keys such as `TXT_LEAGUE_O2_EXTRALIGA` /
    // `TXT_LEAGUE_SWISS_LEAGUE` carry the stem inside a longer key).
    for exact in [true, false] {
        for w in wants {
            let wl = w.to_ascii_lowercase();
            let hit = catalog.leagues.iter().find(|l| {
                let keys = [l.name_key.as_deref(), l.dictionary_key.as_deref()];
                if exact {
                    keys.iter().flatten().any(|k| k.eq_ignore_ascii_case(w))
                } else {
                    keys.iter().flatten().any(|k| k.to_ascii_lowercase().contains(&wl))
                        || l.label.as_deref().is_some_and(|lb| lb.to_ascii_lowercase().contains(&wl))
                }
            });
            if let Some(l) = hit {
                return (
                    Some(l.code),
                    Some(l.display()),
                    format!("stem {stem:?} matched league code {} via {w:?}", l.code),
                );
            }
        }
    }
    (None, None, format!("stem {stem:?}: no catalogued league matches {:?}", wants))
}

fn classify(name: &str) -> (&str, &'static str) {
    let n = name.strip_suffix("schedule").unwrap_or(name);
    if name == "leaguescheduletable" || name == "leaguetempscheduletable" {
        return ("", "mode-local");
    }
    if let Some(s) = name.strip_suffix("scheduledebug") {
        return (s, "debug");
    }
    if name == "favoriteteamschedule" {
        return ("", "favorite");
    }
    if let Some(s) = name.strip_suffix("futureschedule") {
        return (s, "future");
    }
    // NHL 07 length variants: `nhlschedulesmall` / `…medium` / `…longleap` / `…longnoleap`.
    for v in ["small", "medium", "longleap", "longnoleap"] {
        if let Some(s) = n.strip_suffix(v) {
            return (s.strip_suffix("schedule").unwrap_or(s), "variant");
        }
    }
    (n, "source")
}

/// Every schedule table of the database with its league match.
pub fn schedule_tables(db: &TdbFile, meta: &FieldMetaIndex, catalog: &LeagueCatalog) -> Vec<ScheduleTableSummary> {
    let mut out = Vec::new();
    for t in &db.tables {
        let Some(name) = meta.table_name(&t.id.raw()) else {
            continue;
        };
        if !name.contains("schedule") {
            continue;
        }
        let (stem, role) = classify(name);
        let (league_code, league_label, league_match) = if role == "source" || role == "future" || role == "debug" || role == "variant" {
            match_league(stem, catalog)
        } else {
            (None, None, format!("{role} table; not a league's stock season"))
        };
        let shown_with_league = tournament_of(name).and_then(|k| tournament_league(k, catalog));
        out.push(ScheduleTableSummary {
            studio_table: t.id.canonical(db.endian),
            name: name.to_string(),
            rows: t.current_records as u32,
            max_records: t.max_records as u32,
            league_code,
            league_label,
            league_match,
            role,
            shown_with_league,
        });
    }
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// FNV-1a 64 over a string, rendered as hex (no crypto needed: this only detects change).
fn fnv1a(text: &str) -> String {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{h:016x}")
}

/// The slot ↔ team map of one league: the league's members ordered by the team table's global
/// `index`, ranked from 0. The team table's `index` is global (Legacy: NHL 0..31, SHL 62..75),
/// so a schedule slot is the team's rank among its league's members by that index — never the
/// raw `index` value, never the team id and never the record order (NHL 10 AHL and NHL 11 OHL
/// store one row out of `index` order; the rank rule is the one the stock schedules follow).
pub fn slot_map(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    catalog: &LeagueCatalog,
    teams: &[crate::view::TeamSummary],
    code: u64,
) -> SlotMap {
    let mut out = SlotMap {
        league_code: Some(code),
        ..Default::default()
    };
    let Some(kott) = db.table_canonical(&catalog.team_league_table) else {
        return out;
    };
    let Some(idx) = by_name(meta, kott, "index") else {
        return out;
    };
    // NHL 07: `kOtt` is a thin index and the league code + `index` live on the stock team table,
    // so the summary's (thin) record is followed through the (table, teamid) join first.
    let thin = if catalog.team_league_table == tdb_relations::entity::ids::TEAM {
        None
    } else {
        db.table_canonical(tdb_relations::entity::ids::TEAM)
    };
    let mut members: Vec<(u64, &crate::view::TeamSummary, u32)> = teams
        .iter()
        .filter(|t| t.league == Some(code as i64))
        .filter_map(|t| {
            let record = match thin {
                Some(thin) => crate::view::team_name_source(db, meta, thin, t.record).record,
                None => t.record,
            };
            db.read_u64(kott, record, idx).map(|i| (i, t, record))
        })
        .collect();
    members.sort_by_key(|m| (m.0, m.1.record));
    // Export keys: the abbreviation when unique within the league, else disambiguated by id.
    let mut abbrev_counts: BTreeMap<String, u32> = BTreeMap::new();
    for (_, t, _) in &members {
        *abbrev_counts.entry(t.abbrev.trim().to_ascii_uppercase()).or_default() += 1;
    }
    let mut digest = String::new();
    for (rank, (global_index, t, record)) in members.into_iter().enumerate() {
        let abbrev = t.abbrev.trim().to_string();
        let upper = abbrev.to_ascii_uppercase();
        let key = if !upper.is_empty() && abbrev_counts.get(&upper) == Some(&1) {
            upper
        } else if upper.is_empty() {
            format!("TEAM#{}", t.team_id)
        } else {
            format!("{upper}#{}", t.team_id)
        };
        digest.push_str(&format!("{rank}:{}:{global_index};", t.team_id));
        out.entries.push(SlotEntry {
            slot: rank as u64,
            team_id: t.team_id,
            name: t.name.clone(),
            abbrev,
            global_index,
            art_abbrev: crate::view::read_str_by_name(db, meta, kott, record, "artabbr").trim_matches(|c: char| c == '\0' || c.is_whitespace()).to_string(),
            key,
        });
    }
    out.fingerprint = fnv1a(&digest);
    out
}

/// The game-row layout of a schedule table, by metadata name.
pub fn layout(meta: &FieldMetaIndex, db: &TdbFile, table: &Table) -> ScheduleLayout {
    let mut fields = Vec::new();
    for n in ["round", "status", "day", "month", "index", "home", "away"] {
        if let Some(f) = by_name(meta, table, n) {
            fields.push(ScheduleField {
                name: n.to_string(),
                studio_field: f.id.canonical(db.endian),
                bit_width: f.bit_width,
            });
        }
    }
    ScheduleLayout { fields }
}

/// Browse + validate one schedule table.
pub fn schedule_view(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    catalog: &LeagueCatalog,
    teams: &[crate::view::TeamSummary],
    studio_table: &str,
) -> Result<Option<ScheduleView>> {
    let Some(summary) = schedule_tables(db, meta, catalog).into_iter().find(|t| t.studio_table == studio_table) else {
        return Ok(None);
    };
    let table = db.table_canonical(studio_table).expect("listed");
    let layout = layout(meta, db, table);
    let fields: BTreeMap<String, String> = layout.fields.iter().map(|f| (f.name.clone(), f.studio_field.clone())).collect();
    let slot_map = summary
        .league_code
        .map(|c| slot_map(db, meta, catalog, teams, c))
        .unwrap_or_default();
    let need = |n: &str| by_name(meta, table, n);
    let (Some(f_day), Some(f_month), Some(f_home), Some(f_away)) = (need("day"), need("month"), need("home"), need("away")) else {
        return Ok(Some(ScheduleView {
            table: summary,
            fields,
            layout,
            slot_map,
            games: Vec::new(),
            slots: Vec::new(),
            unresolved_slots: Vec::new(),
            findings: vec!["this table does not carry the day/month/home/away game layout; shown as a plain table".into()],
            first_date: None,
            last_date: None,
            tournament_note: None,
        }));
    };
    let f_index = need("index");
    let f_round = need("round");
    let f_status = need("status");
    let slots: BTreeMap<u64, (u64, String)> = slot_map
        .entries
        .iter()
        .map(|e| (e.slot, (e.team_id, e.name.clone())))
        .collect();
    let mut games = Vec::with_capacity(table.current_records as usize);
    let mut counts: BTreeMap<u64, u32> = BTreeMap::new();
    let mut findings = Vec::new();
    let mut prev_index: Option<u64> = None;
    let mut out_of_order = 0u32;
    let mut self_games = 0u32;
    let mut bad_dates = 0u32;
    for rec in 0..table.current_records {
        let rec32 = rec as u32;
        let rd = |f: &FieldDescriptor| db.read_u64(table, rec32, f).unwrap_or(0);
        let month = rd(f_month);
        let day = rd(f_day);
        let home_slot = rd(f_home);
        let away_slot = rd(f_away);
        let index = f_index.map(rd).unwrap_or(rec32 as u64);
        if month > 11 || day > 30 {
            bad_dates += 1;
        }
        if home_slot == away_slot {
            self_games += 1;
        }
        if let Some(p) = prev_index {
            if index != p + 1 {
                out_of_order += 1;
            }
        }
        prev_index = Some(index);
        *counts.entry(home_slot).or_default() += 1;
        *counts.entry(away_slot).or_default() += 1;
        games.push(ScheduledGame {
            record: rec32,
            index,
            month,
            day,
            date: date_label(month, day),
            round: f_round.map(rd).unwrap_or(0),
            status: f_status.map(rd),
            home_slot,
            away_slot,
            home: slots.get(&home_slot).map(|(_, n)| n.clone()),
            away: slots.get(&away_slot).map(|(_, n)| n.clone()),
            round_label: None,
            home_label: None,
            away_label: None,
        });
    }
    // R115 browse labels for the three tournament templates; the table itself stays unbound.
    let tournament_note = tournament_of(&summary.name).map(|kind| {
        let wc_names: BTreeMap<u64, String> = if kind == Tournament::WorldCup {
            catalog
                .leagues
                .iter()
                .find(|l| [l.name_key.as_deref(), l.dictionary_key.as_deref()].iter().flatten().any(|k| k.eq_ignore_ascii_case("WorldCup")))
                .map(|l| self::slot_map(db, meta, catalog, teams, l.code).entries
                        .into_iter()
                        // National teams can carry no display name; the abbreviation is their human
                        // reference (R115 names them CAN / CZE / EUR / USA, FIN / NA / RUS / SWE).
                        .map(|e| {
                            let label = if !e.name.trim().is_empty() {
                                e.name
                            } else if !e.abbrev.trim().is_empty() {
                                e.abbrev
                            } else {
                                format!("team {}", e.team_id)
                            };
                            (e.slot, label)
                        })
                        .collect())
                .unwrap_or_default()
        } else {
            BTreeMap::new()
        };
        label_tournament(kind, &mut games, &wc_names)
    });
    let mut slot_counts = Vec::new();
    let mut unresolved = Vec::new();
    for (slot, games_n) in &counts {
        let sc = SlotCount {
            slot: *slot,
            team: slots.get(slot).map(|(_, n)| n.clone()),
            team_id: slots.get(slot).map(|(id, _)| *id),
            games: *games_n,
        };
        if sc.team.is_some() {
            slot_counts.push(sc);
        } else {
            unresolved.push(sc);
        }
    }
    // Findings.
    if summary.league_code.is_none() && summary.role == "source" {
        findings.push(format!("{}: home/away slots cannot be named ({})", summary.name, summary.league_match));
    }
    if !slot_counts.is_empty() {
        // The modal games-per-team is the season length; teams with one or two games are the
        // All-Star / exhibition squads the titles up to Legacy ship as league members (disclosed,
        // not an error); anything else uneven is a finding.
        let mut freq: BTreeMap<u32, u32> = BTreeMap::new();
        for sc in &slot_counts {
            *freq.entry(sc.games).or_default() += 1;
        }
        let season = freq.iter().max_by_key(|(g, n)| (**n, **g)).map(|(g, _)| *g).unwrap_or(0);
        let exhibition: Vec<String> = slot_counts
            .iter()
            .filter(|s| s.games <= 2 && s.games < season)
            .map(|s| format!("{} ({} game{})", s.team.as_deref().unwrap_or("?"), s.games, if s.games == 1 { "" } else { "s" }))
            .collect();
        if !exhibition.is_empty() {
            findings.push(format!(
                "All-Star / exhibition squads with their own slot: {} — the regular teams play {season}",
                exhibition.join(", ")
            ));
        }
        let uneven: Vec<String> = slot_counts
            .iter()
            .filter(|s| s.games != season && !(s.games <= 2 && s.games < season))
            .map(|s| format!("{} ({} games, others {season})", s.team.as_deref().unwrap_or("?"), s.games))
            .collect();
        if !uneven.is_empty() {
            findings.push(format!("uneven games per team: {}", uneven.join(", ")));
        }
        for (slot, (_, name)) in &slots {
            if !counts.contains_key(slot) {
                findings.push(format!("{name} (league slot {slot}) has no games in this table"));
            }
        }
    }
    if !unresolved.is_empty() && tournament_note.is_none() {
        let n: u32 = unresolved.iter().map(|s| s.games).sum();
        findings.push(format!(
            "{} slot(s) with no team at that league position carry {n} game(s) (an All-Star row on the titles that ship one; otherwise dangling): {}",
            unresolved.len(),
            unresolved.iter().map(|s| format!("slot {} × {}", s.slot, s.games)).collect::<Vec<_>>().join(", ")
        ));
    }
    if bad_dates > 0 {
        findings.push(format!("{bad_dates} row(s) with a month > 11 or day > 30"));
    }
    if self_games > 0 {
        findings.push(format!("{self_games} row(s) where home and away are the same slot"));
    }
    if out_of_order > 0 {
        findings.push(format!("{out_of_order} row(s) where the running index does not increase by one"));
    }
    let first_date = games.first().map(|g| g.date.clone());
    let last_date = games.last().map(|g| g.date.clone());
    Ok(Some(ScheduleView {
        table: summary,
        fields,
        layout,
        slot_map,
        games,
        slots: slot_counts,
        unresolved_slots: unresolved,
        findings,
        first_date,
        last_date,
        tournament_note,
    }))
}
