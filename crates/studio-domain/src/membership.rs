//! Phase 1.2 — league membership as one validated operation: move a team to another league and
//! give it the competition groups (conference / division) that league uses **on this title**.
//!
//! Three families, each handled as its own data says (research handoff P1.2-X, 2026-09-18):
//!
//! * **NHL 12 → Legacy** (Xbox 360 / PS3 / Recompiled): the team row carries global group ids
//!   (`leaguegroup` / `conferencegroup` / `divisiongroup`) into a title-specific table baked in
//!   `defs/leagues/groups/<game>.toml`. The plan sets the league code, the league's own group id
//!   and a validated conference/division pair from *this* title's table; a single-group league
//!   gets its own id in all three fields, and ids are never copied from another title.
//! * **NHL 08 → 11**: `conference` (1 bit) and `division` (3 bits) are local numbers whose
//!   names are untraced; the plan sets the league code and, when the caller gives them, the raw
//!   local numbers (range-checked by the field metadata only), and says so.
//! * **PS4 NHL 15 → 21**: the league code is a foreign key into `stockleagues` (the delta
//!   referential gate refuses a dangling one). Since P1-C (research handoff P1.2-PS4) the group
//!   ids resolve through the title's own executable table baked in
//!   `defs/leagues/groups/ps4-<game>.toml`, so a PS4 database with a fitting table takes the
//!   **same** global-groups path as NHL 12 → Legacy (one editor, per-row evidence carried:
//!   traced-static / inferred / unknown). A PS4 database without a fitting table (no game id, a
//!   wrong-title claim the rows contradict) keeps the raw model: ids written as given, disclosed.
//!
//! NHL 07's thin team index (`HqzU` league code on the stock/custom table) is refused explicitly.
//! Every write goes through one transaction (one undo entry, mirrored to an aligned roster save
//! by the workspace); nothing here claims what the game does with a moved team (schedules,
//! standings and mode initialization are runtime questions — see the handoff §4).

use tdb_core::TdbFile;

use crate::error::DomainError;
use crate::leagues::{LeagueCatalog, TEAM_LEAGUE_FIELD};
use crate::session::EditSession;
use crate::Result;

/// Studio ids of the local conference/division bits on NHL 08–11 (`zsar` / `Xegx` reversed).
pub const LOCAL_CONFERENCE_FIELD: &str = "rasz";
pub const LOCAL_DIVISION_FIELD: &str = "xgeX";

/// How this title models membership groups.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupModel {
    /// Global ids into the title's competition table (NHL 12 → Legacy).
    GlobalGroups,
    /// Local per-league conference/division numbers, names untraced (NHL 08 → 11).
    LocalBits,
    /// PS4: `stockleagues` foreign key; group ids present, no fitting table for this session.
    Ps4,
    /// No group fields on the team row (or an unsupported layout).
    None,
}

impl GroupModel {
    pub fn as_str(self) -> &'static str {
        match self {
            GroupModel::GlobalGroups => "global-groups",
            GroupModel::LocalBits => "local-bits",
            GroupModel::Ps4 => "ps4-foreign-key",
            GroupModel::None => "none",
        }
    }
}

/// One choosable value with its resolved label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Choice {
    pub value: u64,
    pub label: String,
    /// The mapping's evidence (`traced-static` / `inferred` / `unknown`) when the value is a
    /// group of the title's table; `None` for raw or local numbers.
    pub evidence: Option<String>,
}

impl Choice {
    fn raw(value: u64, label: String) -> Self {
        Choice { value, label, evidence: None }
    }

    fn of_group(g: &crate::competition_groups::ResolvedGroup) -> Self {
        Choice {
            value: g.id,
            label: g.label.clone(),
            evidence: Some(g.evidence.clone()),
        }
    }
}

/// What a team's membership is right now, resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MembershipState {
    pub league_code: u64,
    pub league_label: String,
    pub league_group: Option<Choice>,
    pub conference: Option<Choice>,
    pub division: Option<Choice>,
}

/// The choices the UI offers for a team, and how the title models them.
#[derive(Debug, Clone)]
pub struct MembershipOptions {
    pub team_id: u64,
    pub model: GroupModel,
    /// Evidence class of the group table for this target (`traced` / `by-reference`), when one.
    pub group_evidence: Option<String>,
    pub current: MembershipState,
    pub leagues: Vec<Choice>,
    /// Per league code: the conferences and divisions the title's table lists for it.
    pub groups_by_league: Vec<LeagueGroups>,
    /// Plain-language limits of this title's model.
    pub notes: Vec<String>,
    /// Why membership cannot be edited here at all, when it cannot.
    pub blocker: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LeagueGroups {
    pub league_code: u64,
    /// The league's own group id.
    pub league_group: u64,
    /// The league group's evidence (`traced-static` / `inferred` / `unknown`).
    pub league_group_evidence: String,
    /// Choosable conference-slot values (distinct, table order; the own id where witnessed).
    pub conferences: Vec<Choice>,
    /// Choosable division-slot values over all conferences (the UI filters by `pairs`).
    pub divisions: Vec<Choice>,
    /// The witnessed regular (conference, division) arrangements; a move must be one of them.
    pub pairs: Vec<(u64, u64)>,
    /// Exceptional stock arrangements of the block, described (never offered).
    pub exceptional: Vec<String>,
}

/// A validated plan: the exact cells one transaction will write.
#[derive(Debug, Clone)]
pub struct MembershipPlan {
    pub team_id: u64,
    pub table: String,
    pub record: u32,
    pub writes: Vec<(String, i64)>,
    pub before: MembershipState,
    pub after: MembershipState,
    pub notes: Vec<String>,
}

/// Research R133 (2026-09-26 handoff 26 §4; receipt 44): on the NHL 08–11 `kOtt` titles a local division
/// number has no traced name — the executables' name lists do not index by it — so its neutral label lists
/// the clubs this title files there: `Division 3: CBJ · CHI · DET · NSH · STL`. Members share the league
/// code, the local conference and the division (the All-Star rows, at conference 0 / division 0 of the NHL
/// block, therefore stay apart from the Eastern clubs that division 0 holds). `None` on other layouts, for
/// NHL 07 (its stock table carries the membership and its own vocabulary), or when nobody is filed there.
pub fn local_division_label(db: &TdbFile, meta: &tdb_defs::FieldMetaIndex, league: u64, conference: u64, division: u64) -> Option<String> {
    let kott = db.table_canonical(tdb_relations::entity::ids::TEAM)?;
    if !field_exists(db, kott, LOCAL_CONFERENCE_FIELD) || !field_exists(db, kott, LOCAL_DIVISION_FIELD) || !field_exists(db, kott, TEAM_LEAGUE_FIELD) {
        return None;
    }
    let mut members = Vec::new();
    for rec in 0..kott.current_records as u32 {
        let r = |f: &str| crate::view::read_u64_by_studio(db, kott, rec, f);
        if r(TEAM_LEAGUE_FIELD) == Some(league) && r(LOCAL_CONFERENCE_FIELD) == Some(conference) && r(LOCAL_DIVISION_FIELD) == Some(division) {
            let a = crate::view::read_str_by_name(db, meta, kott, rec, "abbrname");
            if !a.is_empty() {
                members.push(a);
            }
        }
    }
    if members.is_empty() {
        return None;
    }
    members.sort();
    let shown = members.iter().take(8).cloned().collect::<Vec<_>>().join(" · ");
    let more = if members.len() > 8 { format!(" +{}", members.len() - 8) } else { String::new() };
    Some(format!("Division {division}: {shown}{more}"))
}

fn field_exists(db: &TdbFile, table: &tdb_core::Table, studio: &str) -> bool {
    table.fields.iter().any(|f| f.id.canonical(db.endian) == studio)
}

fn state(
    db: &TdbFile,
    meta: &tdb_defs::FieldMetaIndex,
    table: &tdb_core::Table,
    record: u32,
    catalog: &LeagueCatalog,
    model: GroupModel,
) -> Result<MembershipState> {
    let code = crate::view::read_u64_by_studio(db, table, record, TEAM_LEAGUE_FIELD)
        .ok_or_else(|| DomainError::Validation("the team row has no league code".into()))?;
    let league_label = catalog.display(code);
    let read = |f: &str| crate::view::read_u64_by_studio(db, table, record, f);
    let (league_group, conference, division) = match model {
        GroupModel::GlobalGroups => {
            let c = catalog.competition.as_ref().expect("model implies a table");
            let choice = |f: &str| {
                read(f).map(|v| match c.group(v) {
                    Some(g) => Choice::of_group(g),
                    None => Choice::raw(v, format!("group {v} (not in this title's table)")),
                })
            };
            (
                choice(&c.fields.league),
                choice(&c.fields.conference),
                choice(&c.fields.division),
            )
        }
        GroupModel::LocalBits => {
            // A vocabulary may name the local numbers under the league code (NHL 07: research
            // 2026-09-21 handoff 13 §1 — the six 2006-07 divisions); otherwise the number shows.
            let local = |f: &str, what: &str| {
                read(f).map(|v| match catalog.group_label(f, code, v) {
                    Some(label) => Choice::raw(v, label),
                    None => {
                        // R133: an untraced division is labelled by its members on this title.
                        let members = (f == LOCAL_DIVISION_FIELD)
                            .then(|| read(LOCAL_CONFERENCE_FIELD).and_then(|c| local_division_label(db, meta, code, c, v)))
                            .flatten();
                        match members {
                            Some(m) => Choice::raw(v, format!("{m} (name untraced)")),
                            None => Choice::raw(v, format!("{what} {v} (local number; name untraced)")),
                        }
                    }
                })
            };
            (None, local(LOCAL_CONFERENCE_FIELD, "conference"), local(LOCAL_DIVISION_FIELD, "division"))
        }
        GroupModel::Ps4 => {
            let raw = |f: &str, what: &str| {
                read(f).map(|v| Choice::raw(v, format!("{what} group {v} (no group table applies to this session)")))
            };
            (
                raw("iIBl", "league"),
                raw("OwTu", "conference"),
                raw("CGMK", "division"),
            )
        }
        GroupModel::None => (None, None, None),
    };
    Ok(MembershipState {
        league_code: code,
        league_label,
        league_group,
        conference,
        division,
    })
}

fn model_for(db: &TdbFile, table: &tdb_core::Table, catalog: &LeagueCatalog) -> GroupModel {
    if catalog.competition.is_some() {
        return GroupModel::GlobalGroups;
    }
    if catalog.source == crate::leagues::LeagueSource::StockLeagues {
        return GroupModel::Ps4;
    }
    if field_exists(db, table, LOCAL_CONFERENCE_FIELD) && field_exists(db, table, LOCAL_DIVISION_FIELD) {
        return GroupModel::LocalBits;
    }
    GroupModel::None
}

/// The team table row a membership edit targets (`kOtt` with its own league field); NHL 07's thin
/// index is refused.
fn target<'a>(db: &'a TdbFile, catalog: &LeagueCatalog, team_id: u64) -> Result<(&'a tdb_core::Table, u32)> {
    let model = tdb_relations::RosterModel::build(db)
        .map_err(|e| DomainError::Validation(format!("roster model: {e}")))?;
    let team = model
        .team(team_id)
        .ok_or_else(|| DomainError::Validation(format!("no team with id {team_id}")))?;
    let table = db
        .table_canonical(&catalog.team_league_table)
        .ok_or_else(|| DomainError::Validation("no team table".into()))?;
    // NHL 07 (research 2026-09-21 handoff 13 §1 / §3): the membership lives on `stockteamtable`
    // (`league` / `conference` / `division` per stock row) and `exhibitionteams` is a thin
    // stock-only selector (`table` 0 / `teamid` = stock index) — the edit target is the stock row
    // the selector points at; a custom-table row (`table` 1; none in the stock DB) is refused.
    if catalog.team_league_table == tdb_relations::entity::ids::STOCK_TEAM {
        use tdb_relations::entity::ids::{STOCK_TEAM, TEAM, TEAM_NAME_ID, TEAM_NAME_TABLE_DISC};
        let kott = db.table_canonical(TEAM).ok_or_else(|| DomainError::Validation("no team selector table".into()))?;
        let disc = crate::view::read_u64_by_studio(db, kott, team.record, TEAM_NAME_TABLE_DISC).unwrap_or(0);
        if disc != 0 {
            return Err(DomainError::Validation(format!(
                "team {team_id} selects a custom-table row (table discriminator {disc}); NHL 07 membership editing covers the stock team table only"
            )));
        }
        let teamid = crate::view::read_u64_by_studio(db, kott, team.record, TEAM_NAME_ID)
            .ok_or_else(|| DomainError::Validation(format!("team {team_id}: the selector row carries no teamid")))?;
        let stock = db.table_canonical(STOCK_TEAM).ok_or_else(|| DomainError::Validation("no stock team table".into()))?;
        // `index` (shortname `qEfv`, studio `vfEq`).
        let key = stock
            .fields
            .iter()
            .find(|f| f.id.canonical(db.endian) == "vfEq")
            .ok_or_else(|| DomainError::Validation("the stock team table has no index key".into()))?;
        let record = (0..stock.current_records as u32)
            .find(|&i| db.read_u64(stock, i, key) == Some(teamid))
            .ok_or_else(|| DomainError::Validation(format!("team {team_id}: no stock row with index {teamid}")))?;
        if !field_exists(db, stock, TEAM_LEAGUE_FIELD) {
            return Err(DomainError::Validation("the stock team table carries no league code".into()));
        }
        return Ok((stock, record));
    }
    if catalog.team_league_table != tdb_relations::entity::ids::TEAM || !field_exists(db, table, TEAM_LEAGUE_FIELD) {
        return Err(DomainError::Validation(format!(
            "league membership editing is not supported on this title: the league code lives on the {} table, not on the team row",
            catalog.team_league_table
        )));
    }
    Ok((table, team.record))
}

/// The options and current state for one team.
pub fn options(session: &EditSession, team_id: u64) -> Result<MembershipOptions> {
    let db = TdbFile::parse(session.bytes())?;
    let catalog = session.league_catalog()?;
    let (table, record) = match target(&db, &catalog, team_id) {
        Ok(t) => t,
        Err(e) => {
            return Ok(MembershipOptions {
                team_id,
                model: GroupModel::None,
                group_evidence: None,
                current: MembershipState {
                    league_code: 0,
                    league_label: String::new(),
                    league_group: None,
                    conference: None,
                    division: None,
                },
                leagues: Vec::new(),
                groups_by_league: Vec::new(),
                notes: Vec::new(),
                blocker: Some(e.to_string()),
            })
        }
    };
    let model = model_for(&db, table, &catalog);
    let current = state(&db, session.project().meta(), table, record, &catalog, model)?;
    let leagues = catalog
        .leagues
        .iter()
        .map(|l| Choice::raw(l.code, l.display()))
        .collect();
    let mut notes = Vec::new();
    let mut groups_by_league = Vec::new();
    let mut group_evidence = None;
    match model {
        GroupModel::GlobalGroups => {
            let c = catalog.competition.as_ref().unwrap();
            group_evidence = Some(c.evidence.clone());
            for l in &catalog.leagues {
                if let Some(lg) = c.league_group(l.code) {
                    let choices = |v: Vec<&crate::competition_groups::ResolvedGroup>, slot: &str| -> Vec<Choice> {
                        v.iter()
                            .map(|g| {
                                let mut ch = Choice::of_group(g);
                                if g.id == lg.id {
                                    ch.label = format!("{} (no {slot}: the league's own id)", g.label);
                                }
                                ch
                            })
                            .collect()
                    };
                    let pairs = c.regular_pairs(l.code);
                    let single = pairs.is_empty() || pairs == [(lg.id, lg.id)];
                    groups_by_league.push(LeagueGroups {
                        league_code: l.code,
                        league_group: lg.id,
                        league_group_evidence: lg.evidence.clone(),
                        conferences: if single { Vec::new() } else { choices(c.conference_choices(l.code), "conference") },
                        divisions: if single { Vec::new() } else { choices(c.division_choices(l.code, None), "division") },
                        pairs: if single { Vec::new() } else { pairs },
                        exceptional: c
                            .exceptional_pairs(l.code)
                            .iter()
                            .map(|p| {
                                format!(
                                    "{} / {} ({}; {} row{}: team id{} {})",
                                    c.group(p.conference).map(|g| g.label.clone()).unwrap_or_else(|| format!("group {}", p.conference)),
                                    c.group(p.division).map(|g| g.label.clone()).unwrap_or_else(|| format!("group {}", p.division)),
                                    p.note.as_deref().unwrap_or("exceptional"),
                                    p.rows,
                                    if p.rows == 1 { "" } else { "s" },
                                    if p.teams.len() == 1 { "" } else { "s" },
                                    p.teams.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(", ")
                                )
                            })
                            .collect(),
                    });
                }
            }
            notes.push(format!(
                "conference and division are global group ids from this title's own table ({}; {}); only the (conference, division) arrangements the title's stock rows carry for a league are offered, and a single-group league uses its own id for all three fields. Whether standings, schedules and mode initialization follow a moved team is a runtime question",
                c.evidence, c.witness
            ));
            if let Some(p) = &c.provenance {
                notes.push(format!("title provenance: {}", p.describe()));
            }
            if !c.has_pairs() {
                notes.push("this table records no witnessed conference/division arrangements: structure is unresolved here, and only a league's own id can be written".into());
            }
            if let Some(fit) = &c.fit {
                notes.push(format!("row agreement (compatibility, not identity): {}", fit.summary()));
                let without: Vec<String> = fit
                    .codes_without_group
                    .iter()
                    .map(|(code, n)| format!("{} ({n} team{})", catalog.display(*code), if *n == 1 { "" } else { "s" }))
                    .collect();
                if !without.is_empty() {
                    notes.push(format!(
                        "leagues whose rows carry a group id the table does not list (browse only; a move into them is refused): {}",
                        without.join(", ")
                    ));
                }
            }
            let unknown = c.unknown();
            if !unknown.is_empty() {
                notes.push(format!(
                    "group ids with no known name on this title (shown by number; a move into their leagues writes the id the rows already use): {}",
                    unknown
                        .iter()
                        .map(|g| {
                            let leagues: Vec<String> = g.league_codes.iter().map(|&lc| catalog.display(lc)).collect();
                            format!(
                                "{} — {}{}",
                                g.id,
                                leagues.join(" / "),
                                g.hypothesis_key
                                    .as_deref()
                                    .map(|h| format!(" (research hypothesis, not a name: {h})"))
                                    .unwrap_or_default()
                            )
                        })
                        .collect::<Vec<_>>()
                        .join("; ")
                ));
            }
            let inferred = c.all().filter(|g| g.evidence == crate::competition_groups::EVIDENCE_INFERRED).count();
            if inferred > 0 {
                notes.push(format!(
                    "{inferred} group name{} on this title are inferred from database usage rather than read from the executable in id order; they are marked as such",
                    if inferred == 1 { "" } else { "s" }
                ));
            }
            for n in &c.witness_notes {
                notes.push(format!("witness: {n}"));
            }
            let unlisted: Vec<String> = catalog
                .leagues
                .iter()
                .filter(|l| c.league_group(l.code).is_none() && !c.fit.as_ref().is_some_and(|f| f.codes_without_group.iter().any(|(code, _)| *code == l.code)))
                .map(|l| l.display())
                .collect();
            if !unlisted.is_empty() {
                notes.push(format!(
                    "leagues with no teams and no group established on this title (a move into them is refused because no valid group ids are known): {}",
                    unlisted.join(", ")
                ));
            }
        }
        GroupModel::LocalBits => {
            if catalog.team_league_table == tdb_relations::entity::ids::STOCK_TEAM {
                for league in &catalog.leagues {
                    let mut pairs=std::collections::BTreeSet::new();
                    for row in 0..table.current_records as u32 {
                        let read=|f:&str|crate::view::read_u64_by_studio(&db,table,row,f);
                        if read(TEAM_LEAGUE_FIELD)!=Some(league.code){continue;}
                        if let (Some(c),Some(d))=(read(LOCAL_CONFERENCE_FIELD),read(LOCAL_DIVISION_FIELD)) {
                            // NHL All-Star divisions are separate from the six regular divisions.
                            if league.code==0 && d>5 {continue;}
                            pairs.insert((c,d));
                        }
                    }
                    if league.code==0 {pairs=[(1,0),(1,1),(1,2),(0,3),(0,4),(0,5)].into_iter().collect();}
                    let choices=|field:&str,which:usize| -> Vec<Choice> {
                        pairs.iter().map(|p|if which==0{p.0}else{p.1}).collect::<std::collections::BTreeSet<_>>().into_iter().map(|v|Choice::raw(v,
                            catalog.group_label(field,league.code,v).unwrap_or_else(||format!("{} group {}",if which==0{"Conference"}else{"Division"},v+1)))).collect()
                    };
                    groups_by_league.push(LeagueGroups{league_code:league.code,league_group:league.code,league_group_evidence:"database".into(),conferences:choices(LOCAL_CONFERENCE_FIELD,0),divisions:choices(LOCAL_DIVISION_FIELD,1),pairs:pairs.into_iter().collect(),exceptional:vec![]});
                }
                notes.push("NHL 07 league swaps exchange both teams’ places and scheduled opponents. Other teams keep their existing fixtures.".into());
            } else {
                notes.push("this title stores a local conference (0–1) and division (0–7) per team whose names are not traced; they are written as given. Divisions are labelled by the clubs filed there; on NHL 10 / 11 a conference shows the executable's name by position, marked inferred (research R133)".into());
            }
        }
        GroupModel::Ps4 => {
            notes.push(
                "the league is a stockleagues row (a dangling code is refused); the row's group ids point into the game executable's table, and no baked table applies to this session, so they are written raw when given".into(),
            );
            if let Some(b) = &catalog.competition_blocker {
                notes.push(b.clone());
            }
        }
        GroupModel::None => notes.push("this title's team row carries only the league code".into()),
    }
    Ok(MembershipOptions {
        team_id,
        model,
        group_evidence,
        current,
        leagues,
        groups_by_league,
        notes,
        blocker: None,
    })
}

/// Plan the move. `conference` / `division` are group ids (global model), local numbers (local
/// model) or raw group ids (PS4); omitted values keep the current row value except on the global
/// model, where a multi-group league requires both and a single-group league sets its own id.
pub fn plan(
    session: &EditSession,
    team_id: u64,
    league_code: u64,
    conference: Option<u64>,
    division: Option<u64>,
) -> Result<MembershipPlan> {
    let db = TdbFile::parse(session.bytes())?;
    let catalog = session.league_catalog()?;
    let (table, record) = target(&db, &catalog, team_id)?;
    let model = model_for(&db, table, &catalog);
    let before = state(&db, session.project().meta(), table, record, &catalog, model)?;
    if catalog.league(league_code).is_none() {
        return Err(DomainError::Validation(format!(
            "league code {league_code} is not a league on this title ({} catalogued); a team cannot be moved to an unknown code",
            catalog.leagues.len()
        )));
    }
    if session.project().game_id()==Some("nhl07") {
        let opts=options(session,team_id)?;
        let c=conference.or(before.conference.as_ref().map(|v|v.value));
        let d=division.or(before.division.as_ref().map(|v|v.value));
        let unchanged=league_code==before.league_code && c==before.conference.as_ref().map(|v|v.value) && d==before.division.as_ref().map(|v|v.value);
        if (record==30 || record==31) && !unchanged { return Err(DomainError::Validation("All-Star teams retain their original league and groups.".into())); }
        if !unchanged && !opts.groups_by_league.iter().find(|g|g.league_code==league_code).is_some_and(|g|c.zip(d).is_some_and(|p|g.pairs.contains(&p))) {
            return Err(DomainError::Validation("Choose a conference and division belonging to this league.".into()));
        }
    }
    let mut writes: Vec<(String, i64)> = vec![(TEAM_LEAGUE_FIELD.to_string(), league_code as i64)];
    let mut notes = Vec::new();
    match model {
        GroupModel::GlobalGroups => {
            let c = catalog.competition.as_ref().unwrap();
            let lg = c.league_group(league_code).ok_or_else(|| {
                DomainError::Validation(format!(
                    "{} has no group in this title's competition table; its group ids cannot be set from here",
                    catalog.display(league_code)
                ))
            })?;
            let shape = c.shape(league_code).expect("league group exists");
            let (conf, div) = match shape {
                crate::competition_groups::LeagueShape::Single { .. } => {
                    // Each supplied slot is checked on its own: a valid conference never excuses
                    // a conflicting division (or the reverse). Omission selects the own id.
                    for (slot, given) in [("conference", conference), ("division", division)] {
                        if let Some(x) = given.filter(|&x| x != lg.id) {
                            return Err(DomainError::Validation(format!(
                                "{} is a single-group league on this title: its conference and division are its own group id {}; the {slot} given ({x}) is not that id",
                                lg.label, lg.id
                            )));
                        }
                    }
                    (lg.id, lg.id)
                }
                crate::competition_groups::LeagueShape::Structured { pairs, .. } => {
                    // Omitted on a league whose rows also carry the own-id arrangement: that.
                    if conference.is_none() && division.is_none() && pairs.contains(&(lg.id, lg.id)) {
                        (lg.id, lg.id)
                    } else {
                        let list = |v: Vec<&crate::competition_groups::ResolvedGroup>| {
                            v.iter().map(|g| format!("{} = {}", g.id, g.label_with_evidence())).collect::<Vec<_>>().join(", ")
                        };
                        let conf = conference.ok_or_else(|| {
                            DomainError::Validation(format!(
                                "{} has conferences / pools on this title ({}); choose one for the conference slot",
                                lg.label,
                                list(c.conference_choices(league_code))
                            ))
                        })?;
                        let div = division.ok_or_else(|| {
                            DomainError::Validation(format!(
                                "{} pairs {} with ({}) on this title; choose one for the division slot",
                                lg.label,
                                c.group(conf).map(|g| g.label.clone()).unwrap_or_else(|| format!("group {conf}")),
                                list(c.division_choices(league_code, Some(conf)))
                            ))
                        })?;
                        (conf, div)
                    }
                }
            };
            c.validate_membership(league_code, conf, div)
                .map_err(DomainError::Validation)?;
            writes.push((c.fields.league.clone(), lg.id as i64));
            writes.push((c.fields.conference.clone(), conf as i64));
            writes.push((c.fields.division.clone(), div as i64));
            notes.push(format!(
                "group ids from this title's own table ({}): league {} · conference {} · division {}",
                c.evidence, lg.id, conf, div
            ));
            for g in [lg.id, conf, div].iter().filter_map(|&id| c.group(id)) {
                if g.evidence != crate::competition_groups::EVIDENCE_TRACED_STATIC {
                    notes.push(format!(
                        "group {} ({}) is {}: {}",
                        g.id,
                        g.label,
                        g.evidence,
                        g.note.as_deref().unwrap_or("see the title's table")
                    ));
                }
            }
        }
        GroupModel::LocalBits => {
            if let Some(x) = conference {
                writes.push((LOCAL_CONFERENCE_FIELD.to_string(), x as i64));
            }
            if let Some(x) = division {
                writes.push((LOCAL_DIVISION_FIELD.to_string(), x as i64));
            }
            if catalog.team_league_table == tdb_relations::entity::ids::STOCK_TEAM {
                // NHL 07 (handoff 13 §3): the schedule tables address teams by their rank inside
                // the league (the order of the league's stock rows), and the long / medium /
                // small tables address the All-Star rows 30 / 31 in league 0.
                if league_code != before.league_code && (record == 30 || record == 31) {
                    return Err(DomainError::Validation(format!(
                        "stock row {record} is an All-Star team the NHL schedule tables address directly; it must stay in league 0 (research 2026-09-21 handoff 13 §3)"
                    )));
                }
                if league_code != before.league_code {
                    notes.push("NHL 07: the schedule tables reference teams by their rank inside the league (the order of the league's rows in the stock table); moving a team between leagues re-ranks both leagues' schedules — the schedule tables are not rewritten here".into());
                }
                notes.push("NHL 07: conference / division are the stock row's local numbers, labelled by the 2006-07 structure where the vocabulary names them".into());
            } else {
                notes.push("local conference/division numbers on this title; their names are untraced".into());
            }
        }
        GroupModel::Ps4 => {
            if let Some(x) = conference {
                writes.push(("OwTu".to_string(), x as i64));
            }
            if let Some(x) = division {
                writes.push(("CGMK".to_string(), x as i64));
            }
            notes.push(format!(
                "PS4 group ids are written raw when given; no baked group table applies to this session ({})",
                catalog.competition_blocker.as_deref().unwrap_or("no reason recorded")
            ));
        }
        GroupModel::None => {}
    }
    // The "after" state is the row as it would read: apply the writes to a copy of the values.
    let mut after = before.clone();
    after.league_code = league_code;
    after.league_label = catalog.display(league_code);
    let pick = |f: &str| writes.iter().find(|(k, _)| k == f).map(|(_, v)| *v as u64);
    if model == GroupModel::GlobalGroups {
        let c = catalog.competition.as_ref().unwrap();
        let ch = |f: &str| {
            pick(f).map(|v| match c.group(v) {
                Some(g) => Choice::of_group(g),
                None => Choice::raw(v, String::new()),
            })
        };
        after.league_group = ch(&c.fields.league);
        after.conference = ch(&c.fields.conference);
        after.division = ch(&c.fields.division);
    } else {
        if let Some(v) = pick(LOCAL_CONFERENCE_FIELD).or(pick("OwTu")) {
            after.conference = Some(Choice::raw(v, format!("{v}")));
        }
        if let Some(v) = pick(LOCAL_DIVISION_FIELD).or(pick("CGMK")) {
            after.division = Some(Choice::raw(v, format!("{v}")));
        }
    }
    Ok(MembershipPlan {
        team_id,
        table: catalog.team_league_table.clone(),
        record,
        writes,
        before,
        after,
        notes,
    })
}
