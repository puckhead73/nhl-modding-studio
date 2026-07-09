//! Human-readable rendering of the Layer 4 integrity audit — the presentation half of the
//! human-readable-surfaces rule (see CLAUDE.md § Conventions).
//!
//! [`tdb_domain_validation::validate_invariants`] produces precise but *internal* findings: studio
//! table/field ids (`eGlu.mCcl`) and numeric team/player ids. Those are exactly what an end user
//! cannot read. This module resolves them against the loaded metadata + entity model into a friendly
//! title, a location (real team / player names), and a plain-language message. The raw studio ids and
//! the original technical message ride along untouched for power-user disclosure and telemetry.

use std::collections::HashMap;

use tdb_core::TdbFile;
use tdb_defs::FieldMetaIndex;
use tdb_domain_validation::{Finding, Severity};
use tdb_enums::EnumRegistry;
use tdb_relations::{entity::ids, RosterModel};

use crate::view;

/// One integrity finding, resolved for display.
#[derive(Clone, Debug)]
pub struct HumanFinding {
    pub severity: Severity,
    /// Stable predicate id (grouping/telemetry) — never shown as primary text.
    pub predicate: &'static str,
    /// Friendly category, e.g. "Duplicate jersey number".
    pub title: String,
    /// Resolved entity context, e.g. "Wayne Gretzky — Edmonton Oilers". `None` when nothing resolves.
    pub location: Option<String>,
    /// Plain-language description (localized references, no raw ids).
    pub message: String,
    /// The original technical message (studio ids, numeric ids) — for progressive disclosure.
    pub technical: String,
    /// Raw studio ids, kept for the details affordance.
    pub studio_table: String,
    pub studio_field: String,
    pub record: Option<u32>,
}

/// The whole-DB audit, resolved for display.
#[derive(Clone, Debug, Default)]
pub struct HumanReport {
    pub findings: Vec<HumanFinding>,
}

impl HumanReport {
    pub fn count(&self, sev: Severity) -> usize {
        self.findings.iter().filter(|f| f.severity == sev).count()
    }

    /// No `Error`-severity findings — the db is safe to write.
    pub fn is_clean(&self) -> bool {
        !self.findings.iter().any(|f| f.severity == Severity::Error)
    }
}

/// Resolve the metadata display name for a studio `(table, field)` id pair, if one exists and is
/// non-empty. The meta index is keyed by the on-disk **raw** ids, so we translate through the parsed
/// table (studio id → descriptor → raw id) the same way the view layer does.
fn field_label(db: &TdbFile, meta: &FieldMetaIndex, studio_table: &str, studio_field: &str) -> Option<String> {
    let table = db.table_canonical(studio_table)?;
    let f = table.fields.iter().find(|f| f.id.canonical(db.endian) == studio_field)?;
    meta.field(&table.id.raw(), &f.id.raw())
        .map(|m| m.name.clone())
        .filter(|n| !n.is_empty())
}

fn team_label(t: &view::TeamSummary) -> String {
    let name = t.name.trim();
    if !name.is_empty() {
        name.to_string()
    } else if !t.abbrev.trim().is_empty() {
        t.abbrev.trim().to_string()
    } else {
        format!("Team #{}", t.team_id)
    }
}

fn player_label(p: &view::PlayerSummary) -> String {
    let name = format!("{} {}", p.first_name.trim(), p.last_name.trim());
    let name = name.trim();
    if name.is_empty() {
        format!("Player #{}", p.player_id)
    } else {
        name.to_string()
    }
}

/// Resolve every finding into display-ready text using the loaded metadata + entity model.
pub fn humanize(
    db: &TdbFile,
    meta: &FieldMetaIndex,
    enums: &EnumRegistry,
    findings: Vec<Finding>,
) -> HumanReport {
    // Build name lookups once. If the entity model can't be built (a db variant without the roster
    // tables), we still resolve field labels and fall back to generic entity phrasing.
    let (teams, players): (HashMap<u64, String>, HashMap<u64, String>) = match RosterModel::build(db) {
        Ok(model) => (
            view::team_summaries(db, meta, enums, &model)
                .iter()
                .map(|t| (t.team_id, team_label(t)))
                .collect(),
            view::player_summaries(db, meta, enums, &model)
                .iter()
                .map(|p| (p.player_id, player_label(p)))
                .collect(),
        ),
        Err(_) => (HashMap::new(), HashMap::new()),
    };

    let out = findings
        .into_iter()
        .map(|f| {
            let team = f.team_id.and_then(|id| teams.get(&id).cloned());
            let player = f.player_id.and_then(|id| players.get(&id).cloned());
            let flabel = field_label(db, meta, &f.table, &f.field);

            let (title, message) = render(&f, team.as_deref(), player.as_deref(), flabel.as_deref());

            // Location prefers the concrete entity; for field-scoped findings (FK / enum) the field
            // name is the most meaningful anchor.
            let location = match (&player, &team) {
                (Some(p), Some(t)) => Some(format!("{p} — {t}")),
                (Some(p), None) => Some(p.clone()),
                (None, Some(t)) => Some(t.clone()),
                (None, None) => flabel.clone(),
            };

            HumanFinding {
                severity: f.severity,
                predicate: f.predicate,
                title,
                location,
                message,
                technical: f.message,
                studio_table: f.table,
                studio_field: f.field,
                record: f.record,
            }
        })
        .collect();

    HumanReport { findings: out }
}

/// The (title, message) for a finding — plain language, referencing resolved names where available.
fn render(f: &Finding, team: Option<&str>, player: Option<&str>, flabel: Option<&str>) -> (String, String) {
    let team_or = |fallback: &str| team.map(str::to_string).unwrap_or_else(|| fallback.to_string());
    let player_or = |fallback: &str| player.map(str::to_string).unwrap_or_else(|| fallback.to_string());

    match f.predicate {
        "companion_presence" => (
            "Incomplete player data".to_string(),
            format!(
                "{}'s record is missing a required companion row, so the player is structurally incomplete.",
                player_or("A player")
            ),
        ),
        "single_captain" => (
            "Multiple captains".to_string(),
            format!("{} has more than one captain — a team may have only one.", team_or("This team")),
        ),
        "towx_coherence" => (
            "Corrupted roster block".to_string(),
            format!(
                "{}'s roster is stored out of order — its slots must form one unbroken block. This can corrupt the roster in-game.",
                team_or("A team")
            ),
        ),
        "dup_on_team" if f.field == ids::SLOT_JERSEY => (
            "Duplicate jersey number".to_string(),
            format!("Two players on {} wear the same jersey number.", team_or("one team")),
        ),
        "dup_on_team" => (
            "Player listed twice".to_string(),
            format!(
                "{} appears in two roster slots on {}.",
                player_or("A player"),
                team_or("the same team")
            ),
        ),
        "referential_integrity" => (
            "Broken reference".to_string(),
            match flabel {
                Some(l) => format!("A \u{201c}{l}\u{201d} value refers to a record that no longer exists."),
                None => "A field refers to a record that no longer exists.".to_string(),
            },
        ),
        "enum_vocabulary" => (
            "Unrecognized value".to_string(),
            match flabel {
                Some(l) => format!("\u{201c}{l}\u{201d} holds a value that isn't one of its known options."),
                None => "A field holds a value that isn't one of its known options.".to_string(),
            },
        ),
        // Any future predicate: surface a neutral title and the technical message rather than nothing.
        _ => ("Integrity finding".to_string(), f.message.clone()),
    }
}
