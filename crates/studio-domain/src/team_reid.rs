//! Phase 2.1 — team **re-identification** under the refuse-or-cascade contract of research
//! 2026-09-21 handoff 10 (`research-r37-team-reid-contract`, a policy proposal over the measured
//! reference census of 2026-09-20 handoff 07 §1; consumed under the overnight authorization, not
//! Codex-accepted).
//!
//! Two identities: the team **index** (`team_id`, the row every roster / jersey / equipment /
//! award / retired-number row points at, and the rank order the schedule tables use) and the
//! **art id** (`exhibitionteams.artid`, the number the art files and the jersey / award rows are
//! keyed by). The contract:
//!
//! * renumbering the index is **refused** (the dependency counts are shown);
//! * the art id is never edited as a bare cell (the profile shows it read-only) — the one
//!   supported change is a **swap between two existing teams**: both art sets exist, so only the
//!   database columns follow (`exhibitionteams.artid` of both, `teamartid` of both teams' rows in
//!   `stockteamjerseys` / `customteamjerseys` / `stockawardstable`), as one transaction with one
//!   undo step; refused when either id is a spare-kit id (255, 1000–1004), a third team shares
//!   one of them, or the two teams are the same;
//! * an art-id move to an unused value, a change to another team's value without a swap, and any
//!   re-identification inside a mode-local (`league*`) copy are refused with the reason.
//!
//! Which screens draw which art after a swap is a runtime question (user card).

use serde::{Deserialize, Serialize};
use tdb_core::{Table, TdbFile};

use crate::view::descriptor_by_name;
use crate::{DomainError, EditSession, Result};

/// The tables whose rows carry a team's art id, with the column naming the team (index) and the
/// column carrying the art id.
const ARTID_TABLES: &[(&str, &str, &str)] = &[
    ("stockteamjerseys", "teamid", "teamartid"),
    ("customteamjerseys", "teamid", "teamartid"),
    ("stockawardstable", "team", "teamartid"),
];

/// The tables whose rows carry the team index (the refusal's dependency counts).
const INDEX_TABLES: &[(&str, &str)] = &[
    ("exhibitionrostertable", "team"),
    ("exhibitionplayerbiotable", "team"),
    ("exhibitionplayerbiotable", "juniorteam"),
    ("stockteamjerseys", "teamid"),
    ("stockgoalieequipment", "team"),
    ("stockteamequipmenttable", "team"),
    ("stockretirednumbertable", "team"),
    ("stockawardstable", "team"),
    ("customteamjerseys", "teamid"),
    ("customteamequipmenttable", "team"),
    ("exhibitionteams", "ahlaffiliate"),
    ("exhibitionteams", "rivalteam"),
];

const SPARE_KIT_ART_IDS: &[u64] = &[255, 1000, 1001, 1002, 1003, 1004];

/// Whether `(table name, field name)` (metadata names) is an identity the contract keeps out of
/// the bare cell editor: the team art id. Used by the profile (read-only cell), the scalar edit
/// paths (refused with the reason) and the read-matrix invariant.
pub fn is_guarded_identity(table_name: &str, field_name: &str) -> bool {
    (table_name == "exhibitionarena" && field_name == "table")
        || (field_name == "artid" && matches!(table_name, "exhibitionteams" | "stockteamtable" | "customteamtable"))
        || (table_name == "exhibitionteams" && matches!(field_name, "teamid" | "table"))
}

/// The refusal text for a bare edit of a guarded identity.
pub fn guarded_identity_reason(table_name: &str, field_name: &str) -> String {
    if table_name == "exhibitionarena" && field_name == "table" {
        return "Changing between stock and custom arena records requires an arena creation workflow that is not yet supported.".into();
    }
    if table_name == "exhibitionteams" && matches!(field_name, "teamid" | "table") {
        return "This field selects the team's underlying identity. Use the supported team editors; changing the selector directly is not supported.".into();
    }
    format!("{table_name}.{field_name} is a team identity with dependents (jersey / award rows and the art files keyed by the number): it is not edited as a bare cell — swap art ids between two teams instead (research 2026-09-21 handoff 10)")
}

/// One dependency line: rows of `table.field` that carry the identity value.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DependencyCount {
    pub table: String,
    pub field: String,
    pub rows: usize,
}

/// What a team's two identities are and who depends on them (read-only).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TeamIdentityDependencies {
    pub team_id: u64,
    pub team_name: String,
    pub artid: Option<u64>,
    pub index_dependencies: Vec<DependencyCount>,
    pub artid_dependencies: Vec<DependencyCount>,
    /// Other teams sharing this art id (a state the contract does not support creating).
    pub teams_sharing_artid: Vec<String>,
    pub index_change: String,
    pub artid_change: String,
}

/// A planned art-id swap: every cell it writes, per table, plus the summary.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ArtIdSwapPlan {
    pub team_a: u64,
    pub team_b: u64,
    pub team_a_name: String,
    pub team_b_name: String,
    pub artid_a: u64,
    pub artid_b: u64,
    /// `(table name, rows written)` in order.
    pub tables: Vec<(String, usize)>,
    pub total_cells: usize,
    pub summary: String,
    /// Internal: `(studio table, [(record, studio field, value)])`.
    #[serde(skip)]
    pub cells: Vec<(String, Vec<(u32, String, i64)>)>,
}

pub(crate) fn table_by_name<'a>(db: &'a TdbFile, meta: &tdb_defs::FieldMetaIndex, name: &str) -> Option<&'a Table> {
    db.tables.iter().find(|t| meta.table_name(&t.id.raw()) == Some(name))
}

fn count_rows(db: &TdbFile, meta: &tdb_defs::FieldMetaIndex, table: &str, field: &str, value: u64) -> Option<usize> {
    let t = table_by_name(db, meta, table)?;
    let f = descriptor_by_name(meta, t, field)?;
    Some((0..t.current_records as u32).filter(|&r| db.read_u64(t, r, f) == Some(value)).count())
}

pub(crate) fn team_row(session: &EditSession, team_id: u64) -> Result<(u32, String)> {
    let teams = session.team_summaries()?;
    let t = teams.iter().find(|t| t.team_id == team_id).ok_or_else(|| DomainError::Validation(format!("no team with id {team_id}")))?;
    Ok((t.record, t.name.clone()))
}

/// The dependency listing for one team (read-only).
pub fn team_identity_dependencies(session: &EditSession, team_id: u64) -> Result<TeamIdentityDependencies> {
    let (record, team_name) = team_row(session, team_id)?;
    let bytes = session.bytes().to_vec();
    let db = TdbFile::parse(&bytes)?;
    let meta = session.project().meta();
    let kott = db.table_canonical(tdb_relations::entity::ids::TEAM).ok_or_else(|| DomainError::Validation("no team table".into()))?;
    let artid = descriptor_by_name(meta, kott, "artid").and_then(|f| db.read_u64(kott, record, f));
    let mut index_dependencies = Vec::new();
    for (table, field) in INDEX_TABLES {
        if let Some(rows) = count_rows(&db, meta, table, field, team_id) {
            index_dependencies.push(DependencyCount { table: table.to_string(), field: field.to_string(), rows });
        }
    }
    let mut artid_dependencies = Vec::new();
    let mut teams_sharing_artid = Vec::new();
    if let Some(a) = artid {
        for (table, _, art_field) in ARTID_TABLES {
            if let Some(rows) = count_rows(&db, meta, table, art_field, a) {
                artid_dependencies.push(DependencyCount { table: table.to_string(), field: art_field.to_string(), rows });
            }
        }
        if let Some(f) = descriptor_by_name(meta, kott, "artid") {
            for t in session.team_summaries()? {
                if t.team_id != team_id && db.read_u64(kott, t.record, f) == Some(a) {
                    teams_sharing_artid.push(t.name);
                }
            }
        }
    }
    let index_rows: usize = index_dependencies.iter().map(|d| d.rows).sum();
    Ok(TeamIdentityDependencies {
        team_id,
        team_name,
        artid,
        index_dependencies,
        artid_dependencies,
        teams_sharing_artid,
        index_change: format!("refused: renumbering a team is not offered — {index_rows} rows in {} tables point at this index and the schedule tables rank teams by their order inside the league; no cascade is established (research 2026-09-21 handoff 10 §2)", INDEX_TABLES.len()),
        artid_change: "the art id is not edited as a cell: the supported change is a swap of art ids between two existing teams (both art sets exist; the jersey / award rows of both follow in one step); a move to an unused id or to another team's id without a swap is refused".to_string(),
    })
}

/// Current art IDs and their owning teams, following NHL07's stock/custom identity join.
pub fn artid_options(session: &EditSession) -> Result<Vec<(u64, u64, String)>> {
    let bytes = session.bytes().to_vec();
    let db = TdbFile::parse(&bytes)?;
    let meta = session.project().meta();
    let table = db.table_canonical(tdb_relations::entity::ids::TEAM).ok_or_else(|| DomainError::Validation("no team table".into()))?;
    let mut options = Vec::new();
    for team in session.team_summaries()? {
        let source = crate::view::team_name_source(&db, meta, table, team.record);
        if let Some(id) = descriptor_by_name(meta, source.table, "artid").and_then(|f| db.read_u64(source.table, source.record, f)) {
            options.push((team.team_id, id, team.name));
        }
    }
    options.sort_by_key(|row| (row.1, row.0));
    Ok(options)
}

/// Plan the swap of art ids between teams `a` and `b` (read-only; refusals name the rule).
pub fn plan_artid_swap(session: &EditSession, a: u64, b: u64) -> Result<ArtIdSwapPlan> {
    if a == b {
        return Err(DomainError::Validation("an art-id swap needs two different teams".into()));
    }
    if session.project().role == crate::ProjectRole::Roster {
        return Err(DomainError::Validation("re-identification inside a mode-local / roster copy is refused: the mode tables mirror the stock ids and no cascade path is established".into()));
    }
    let (rec_a, name_a) = team_row(session, a)?;
    let (rec_b, name_b) = team_row(session, b)?;
    let bytes = session.bytes().to_vec();
    let db = TdbFile::parse(&bytes)?;
    let meta = session.project().meta();
    let kott = db.table_canonical(tdb_relations::entity::ids::TEAM).ok_or_else(|| DomainError::Validation("no team table".into()))?;
    let src_a = crate::view::team_name_source(&db, meta, kott, rec_a);
    let src_b = crate::view::team_name_source(&db, meta, kott, rec_b);
    let art_f = descriptor_by_name(meta, src_a.table, "artid").ok_or_else(|| DomainError::Validation("this title's team table has no art id column; nothing to swap".into()))?;
    let art_b = descriptor_by_name(meta, src_b.table, "artid").ok_or_else(|| DomainError::Validation("destination team has no art id".into()))?;
    let artid_a = db.read_u64(src_a.table, src_a.record, art_f).ok_or_else(|| DomainError::Validation(format!("{name_a}: art id unreadable")))?;
    let artid_b = db.read_u64(src_b.table, src_b.record, art_b).ok_or_else(|| DomainError::Validation(format!("{name_b}: art id unreadable")))?;
    if artid_a == artid_b {
        return Err(DomainError::Validation(format!("{name_a} and {name_b} already share art id {artid_a}; a swap would change nothing")));
    }
    for (name, id) in [(&name_a, artid_a), (&name_b, artid_b)] {
        if SPARE_KIT_ART_IDS.contains(&id) {
            return Err(DomainError::Validation(format!("{name}'s art id {id} is a spare-kit id (255, 1000–1004); swaps involving it are refused")));
        }
    }
    let teams = session.team_summaries()?;
    for t in &teams {
        if t.team_id != a && t.team_id != b {
            let src = crate::view::team_name_source(&db, meta, kott, t.record);
            if let Some(v) = descriptor_by_name(meta, src.table, "artid").and_then(|f| db.read_u64(src.table, src.record, f)) {
                if v == artid_a || v == artid_b {
                    return Err(DomainError::Validation(format!("{} also carries art id {v}; a swap with a shared art id is refused (three teams would exchange art)", t.name)));
                }
            }
        }
    }
    let mut cells: Vec<(String, Vec<(u32, String, i64)>)> = Vec::new();
    let mut tables = Vec::new();
    for (src, field, value) in [(&src_a, art_f, artid_b), (&src_b, art_b, artid_a)] {
        let studio = src.table.id.canonical(db.endian);
        let row = (src.record, field.id.canonical(db.endian), value as i64);
        if let Some((_, rows)) = cells.iter_mut().find(|(id, _)| *id == studio) { rows.push(row); }
        else { cells.push((studio, vec![row])); }
        let name = meta.table_name(&src.table.id.raw()).unwrap_or("team").to_string();
        if let Some((_, count)) = tables.iter_mut().find(|(n, _)| *n == name) { *count += 1; }
        else { tables.push((name, 1usize)); }
    }
    for (table, team_field, art_field) in ARTID_TABLES {
        let Some(t) = table_by_name(&db, meta, table) else { continue };
        let (Some(tf), Some(af)) = (descriptor_by_name(meta, t, team_field), descriptor_by_name(meta, t, art_field)) else { continue };
        let mut rows = Vec::new();
        for r in 0..t.current_records as u32 {
            let owner = db.read_u64(t, r, tf);
            let new = if owner == Some(a) { Some(artid_b) } else if owner == Some(b) { Some(artid_a) } else { None };
            if let Some(v) = new {
                // Only rows that carry the team's own art id follow (a spare-kit art id on a
                // jersey row is a different art set and stays).
                let cur = db.read_u64(t, r, af);
                if cur == Some(artid_a) || cur == Some(artid_b) {
                    rows.push((r, af.id.canonical(db.endian), v as i64));
                }
            }
        }
        if !rows.is_empty() {
            tables.push((table.to_string(), rows.len()));
            cells.push((t.id.canonical(db.endian), rows));
        }
    }
    let total_cells = cells.iter().map(|(_, r)| r.len()).sum();
    let summary = format!(
        "swap art ids of {name_a} ({artid_a}) and {name_b} ({artid_b}): {} tables, {total_cells} cells, 0 files — {}",
        tables.len(),
        tables.iter().map(|(t, n)| format!("{t} ×{n}")).collect::<Vec<_>>().join(", ")
    );
    Ok(ArtIdSwapPlan { team_a: a, team_b: b, team_a_name: name_a, team_b_name: name_b, artid_a, artid_b, tables, total_cells, summary, cells })
}

/// Admit only complete permutations of owned art IDs when replaying saved project edits.
/// This keeps raw scalar editing blocked and requires every dependent row to follow.
pub(crate) fn package_art_cells(session: &EditSession, cells: &[(String,u32,String,crate::FieldValue)]) -> Result<std::collections::HashSet<(String,u32,String)>> {
    let bytes=session.bytes().to_vec();let db=TdbFile::parse(&bytes)?;let meta=session.project().meta();
    let mut admitted=std::collections::HashSet::new();
    if !cells.iter().any(|(table,_,field,_)| {
        db.table_canonical(table).and_then(|t| t.fields.iter().find(|f|f.id.canonical(db.endian)==*field).and_then(|f| meta.field(&t.id.raw(),&f.id.raw()).map(|m|m.name=="artid" && meta.table_name(&t.id.raw()).is_some_and(|n|is_guarded_identity(n,&m.name))))).unwrap_or(false)
    }) {return Ok(admitted)}

    let mut moves=Vec::new();
    let kott=db.table_canonical(tdb_relations::entity::ids::TEAM).ok_or_else(||DomainError::Validation("no team table".into()))?;
    let options=artid_options(session)?;
    for team in session.team_summaries()? {
        let src=crate::view::team_name_source(&db,meta,kott,team.record);
        let Some(f)=descriptor_by_name(meta,src.table,"artid") else {continue};
        let key=(src.table.id.canonical(db.endian),src.record,f.id.canonical(db.endian));
        let Some((_,_,_,value))=cells.iter().find(|(t,r,f,_)|*t==key.0 && *r==key.1 && *f==key.2) else {continue};
        let crate::FieldValue::Int(new)=value else {return Err(DomainError::Validation("Art ID must be an integer".into()))};
        let old=db.read_u64(src.table,src.record,f).unwrap();
        if *new<0 || SPARE_KIT_ART_IDS.contains(&old) || SPARE_KIT_ART_IDS.contains(&(*new as u64)) || options.iter().filter(|r|r.1==old).count()!=1 {
            return Err(DomainError::Validation("Art ID swap includes a reserved or shared ID".into()));
        }
        let fm=meta.field(&src.table.id.raw(),&f.id.raw()).unwrap();fm.validate(*new).map_err(|e|DomainError::Validation(e.to_string()))?;
        admitted.insert(key);moves.push((team.team_id,old,*new as u64));
    }
    if moves.is_empty(){return Ok(admitted)}
    let mut before:Vec<_>=moves.iter().map(|r|r.1).collect();let mut after:Vec<_>=moves.iter().map(|r|r.2).collect();before.sort();after.sort();
    if before!=after {return Err(DomainError::Validation("Art ID changes must include both sides of each swap".into()))}
    for (name,owner,art) in ARTID_TABLES {
        let Some(t)=table_by_name(&db,meta,name) else {continue};
        let (Some(tf),Some(af))=(descriptor_by_name(meta,t,owner),descriptor_by_name(meta,t,art)) else {continue};
        for r in 0..t.current_records as u32 {
            let Some((_,old,new))=moves.iter().find(|m|Some(m.0)==db.read_u64(t,r,tf)) else {continue};
            if db.read_u64(t,r,af)!=Some(*old)||old==new {continue}
            let key=(t.id.canonical(db.endian),r,af.id.canonical(db.endian));
            if !cells.iter().any(|(table,row,field,value)| *table==key.0&&*row==r&&*field==key.2&&matches!(value,crate::FieldValue::Int(v) if *v==*new as i64)) {
                return Err(DomainError::Validation(format!("Art ID swap is missing its {name} dependent edit")));
            }
        }
    }
    Ok(admitted)
}
