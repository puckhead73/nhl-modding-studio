//! Phase 2.3 — the dictionary text a team owns: the per-team key families keyed by the team row's
//! `artabbr` (research handoff P2.3-X, 2026-09-18; observed on the Legacy Xbox 360 anchor and the
//! family census on all ten Xbox dictionaries). Joining on `artabbr` (not `abbrname`) resolves
//! 220 of 222 Legacy teams; the two misses are the EASHL placeholders.
//!
//! This module only *resolves* — which keys exist for the team on the attached dictionary and
//! what they read — so a team-text edit is the ordinary dictionary operation (LOC-W1 re-point or
//! LOC-W2 in-place re-encode) on one of those keys. Which screen shows which family, and which
//! screens read the database strings (`fullname`/`shortname`/`abbrname`) instead, are runtime
//! questions the handoff queues.

use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;

use crate::leagues::{resolve_text, AttachedDictionary, Resolved};

/// The key families, in the order they are commonly consumed. `{}` is the `artabbr`.
pub const FAMILIES: &[(&str, &str, &str)] = &[
    ("full name", "NHLTeamName_{}", "team name (NHL 07 → Legacy)"),
    ("city", "NHLCityName_{}", "city (07 → Legacy)"),
    ("abbreviation", "NHLTeamName_Abbr3_{}", "3-letter abbreviation (07 → Legacy)"),
    ("line 1", "TEAMLINE1_{}", "two-line name, first line (older `Teamline1_` spelling resolves by hash)"),
    ("line 2", "TEAMLINE2_{}", "two-line name, second line"),
    ("nickname", "TXT_NICKNAME_{}", "commentary/tweet nickname (NHL 14+)"),
    ("nickname (short)", "TXT_NICKNAME_ALT_{}", "short nickname (NHL 14+)"),
    ("ticker", "X_XLA_TEAM_{}", "scoreboard/ticker abbreviation (NHL 09+)"),
    ("nick line 2", "NICKLINE2_{}", "nickname line (mostly NHL + national teams)"),
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TeamTextKey {
    pub family: String,
    pub note: String,
    /// The key as the family template names it.
    pub key: String,
    /// The dictionary row's own key when it resolved (exact, or the case-folded hash twin).
    pub row_key: Option<String>,
    /// `exact` / `by-hash` / `absent`.
    pub lookup: String,
    pub text: Option<String>,
}

#[derive(Debug, Clone)]
pub struct TeamText {
    pub team_id: u64,
    /// `kOtt.artabbr` as stored; `None` when the schema has no such field.
    pub artabbr: Option<String>,
    /// The database's own strings (what the row carries, independent of the dictionary).
    pub db_fullname: Option<String>,
    pub db_shortname: Option<String>,
    pub db_abbrname: Option<String>,
    pub keys: Vec<TeamTextKey>,
    pub notes: Vec<String>,
}

fn read_str(db: &TdbFile, meta: &FieldMetaIndex, table: &tdb_core::Table, record: u32, name: &str) -> Option<String> {
    let f = table.fields.iter().find(|f| {
        meta.field(&table.id.raw(), &f.id.raw())
            .map(|m| m.name == name)
            .unwrap_or(false)
    })?;
    db.read_inline_str(table, record, f)
        .map(|b| String::from_utf8_lossy(b).trim_end_matches('\0').to_string())
}

/// Resolve a team's dictionary text on the attached dictionary (or list the keys with `absent`
/// when none is attached).
pub fn team_text(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    team_table: &tdb_core::Table,
    record: u32,
    team_id: u64,
    dictionary: Option<&AttachedDictionary>,
) -> TeamText {
    let artabbr = read_str(db, meta, team_table, record, "artabbr").filter(|s| !s.is_empty());
    let mut notes = Vec::new();
    let keys = match &artabbr {
        None => {
            notes.push("this team row has no `artabbr`; the per-team dictionary keys cannot be derived".into());
            Vec::new()
        }
        Some(a) => FAMILIES
            .iter()
            .map(|(family, template, note)| {
                let key = template.replace("{}", a);
                let (row_key, lookup, text) = match dictionary.and_then(|d| resolve_text(d, &key)) {
                    Some((t, Resolved::Exact)) => (Some(key.clone()), "exact", Some(t.to_string())),
                    Some((t, Resolved::ByHash(rk))) => (Some(rk.clone()), "by-hash", Some(t.to_string())),
                    None => (None, "absent", None),
                };
                TeamTextKey {
                    family: family.to_string(),
                    note: note.to_string(),
                    key,
                    row_key,
                    lookup: lookup.to_string(),
                    text,
                }
            })
            .collect(),
    };
    if dictionary.is_none() {
        notes.push("no dictionary attached: keys are listed by template only".into());
    }
    notes.push("keys are joined on the team row's `artabbr` (P2.3-X); which screens read these keys versus the database strings is a runtime question".into());
    TeamText {
        team_id,
        artabbr,
        db_fullname: read_str(db, meta, team_table, record, "fullname"),
        db_shortname: read_str(db, meta, team_table, record, "shortname"),
        db_abbrname: read_str(db, meta, team_table, record, "abbrname"),
        keys,
        notes,
    }
}
