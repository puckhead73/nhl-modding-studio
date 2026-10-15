//! The installed-player manifest — one CSV row per player currently in the loaded EA database,
//! joining their roster identity to what art actually ships for them on disk.
//!
//! This is a reporting surface, not an editing one: it answers "who is in this database, and which
//! of them still need a portrait or a head?" in a form you can sort in a spreadsheet. The art
//! columns are **file truth** (a resolved path on disk), not the bio's id fields — a player can
//! carry a `lOnr`/`pPaD` pointing at art that isn't installed, and the whole point of the export is
//! to find exactly those.

use crate::assets::PlayerAssets;
use crate::view::{PlayerSummary, TeamSummary};
use std::collections::HashMap;

/// One player's row in the manifest.
#[derive(Debug, Clone)]
pub struct ManifestRow {
    pub player_id: u64,
    /// Record index into `ubPc` — the stable handle for this player's bio row.
    pub bio_record: u32,
    pub first_name: String,
    pub last_name: String,
    /// `lastname_firstname_dd_mm_yyyy` — the export filename stem and the bridge key to the
    /// historical corpus. Empty when the bio lacks a usable name or a plausible birthdate.
    pub slug: Option<String>,
    pub position: String,
    pub team: Option<String>,
    pub league: Option<String>,
    /// A real portrait for this player resolves on disk — EA's shared placeholder does not count.
    pub has_portrait: bool,
    /// The resolved portrait is EA's generic placeholder, shared by many un-portraited players.
    pub portrait_generic: bool,
    /// `ubPc.lOnr`.
    pub portrait_id: u64,
    /// A head mesh for this player resolves on disk.
    pub has_head_mesh: bool,
    /// The head is this player's own modelled head rather than a shared/pool one.
    pub head_bespoke: bool,
    /// This head id is referenced by more than one player — a shared generic.
    pub head_shared: bool,
    /// `ubPc.pPaD`.
    pub head_id: u64,
}

/// Column order of the exported CSV. Kept here so the header and the row writer can never drift.
const COLUMNS: &[&str] = &[
    "player_id",
    "bio_record",
    "first_name",
    "last_name",
    "slug",
    "position",
    "team",
    "league",
    "has_portrait",
    "portrait_generic",
    "portrait_id",
    "has_head_mesh",
    "head_bespoke",
    "head_shared",
    "head_id",
];

/// Join the roster summaries, the team index and the on-disk art scan into one row per player.
///
/// `assets` is matched on `bio_record` (the only key both sides carry). Players the art scan didn't
/// see still get a row with every art column false — an un-scanned player is exactly the kind of gap
/// this export exists to surface, so dropping them would defeat the purpose.
///
/// Callers must run [`crate::mark_generic_portraits`] over the scan first; without it every resolved
/// placeholder counts as a real portrait.
pub fn build_player_manifest(
    players: &[PlayerSummary],
    teams: &[TeamSummary],
    assets: &[PlayerAssets],
) -> Vec<ManifestRow> {
    let by_record: HashMap<u32, &PlayerAssets> = assets.iter().map(|a| (a.bio_record, a)).collect();
    let team_by_id: HashMap<u64, &TeamSummary> = teams.iter().map(|t| (t.team_id, t)).collect();

    players
        .iter()
        .map(|p| {
            let a = by_record.get(&p.bio_record);
            let team = p.team_id.and_then(|id| team_by_id.get(&id));
            ManifestRow {
                player_id: p.player_id,
                bio_record: p.bio_record,
                first_name: p.first_name.clone(),
                last_name: p.last_name.clone(),
                slug: a.and_then(|a| a.slug.clone()),
                position: p.position_label.clone(),
                team: team
                    .map(|t| t.name.clone())
                    .filter(|n| !n.trim().is_empty()),
                league: team.and_then(|t| t.league_label.clone()),
                // A generic placeholder is not this player's portrait, so it reads as "no portrait"
                // in the headline column and is disclosed separately.
                has_portrait: a.is_some_and(|a| a.portrait_path.is_some() && !a.generic_portrait),
                portrait_generic: a.is_some_and(|a| a.generic_portrait),
                portrait_id: a.map(|a| a.portrait_id).unwrap_or(0),
                has_head_mesh: a.is_some_and(|a| a.mesh_path.is_some()),
                head_bespoke: a.is_some_and(|a| a.bespoke_head),
                head_shared: a.is_some_and(|a| a.head_shared),
                head_id: a.map(|a| a.head_id).unwrap_or(0),
            }
        })
        .collect()
}

/// Render the manifest as RFC 4180 CSV (CRLF line endings, quoted fields where needed).
pub fn manifest_to_csv(rows: &[ManifestRow]) -> String {
    let mut out = String::with_capacity(rows.len() * 96 + 128);
    out.push_str(&COLUMNS.join(","));
    out.push_str("\r\n");
    for r in rows {
        let fields = [
            r.player_id.to_string(),
            r.bio_record.to_string(),
            r.first_name.clone(),
            r.last_name.clone(),
            r.slug.clone().unwrap_or_default(),
            r.position.clone(),
            r.team.clone().unwrap_or_default(),
            r.league.clone().unwrap_or_default(),
            bool_field(r.has_portrait),
            bool_field(r.portrait_generic),
            r.portrait_id.to_string(),
            bool_field(r.has_head_mesh),
            bool_field(r.head_bespoke),
            bool_field(r.head_shared),
            r.head_id.to_string(),
        ];
        for (i, f) in fields.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push_str(&csv_field(f));
        }
        out.push_str("\r\n");
    }
    out
}

fn bool_field(v: bool) -> String {
    if v { "yes" } else { "no" }.to_string()
}

/// Quote a field when it carries a comma, quote, or newline; doubling embedded quotes. Player names
/// really do contain commas and apostrophes, so this is not theoretical.
fn csv_field(v: &str) -> String {
    if v.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", v.replace('"', "\"\""))
    } else {
        v.to_string()
    }
}

/// Headline counts for the UI to report after a run.
#[derive(Debug, Clone, Default)]
pub struct ManifestSummary {
    pub players: usize,
    pub with_portrait: usize,
    pub with_head_mesh: usize,
    pub with_slug: usize,
}

pub fn summarize(rows: &[ManifestRow]) -> ManifestSummary {
    ManifestSummary {
        players: rows.len(),
        with_portrait: rows.iter().filter(|r| r.has_portrait).count(),
        with_head_mesh: rows.iter().filter(|r| r.has_head_mesh).count(),
        with_slug: rows.iter().filter(|r| r.slug.is_some()).count(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row(name: &str, team: Option<&str>) -> ManifestRow {
        ManifestRow {
            player_id: 1,
            bio_record: 2,
            first_name: name.into(),
            last_name: "O'Reilly, Jr".into(),
            slug: Some("oreilly_ryan_07_02_1991".into()),
            position: "C".into(),
            team: team.map(str::to_string),
            league: Some("NHL".into()),
            has_portrait: true,
            portrait_generic: false,
            portrait_id: 1234,
            has_head_mesh: false,
            head_bespoke: false,
            head_shared: true,
            head_id: 60001,
        }
    }

    #[test]
    fn header_matches_field_count() {
        let csv = manifest_to_csv(&[row("Ryan", Some("Blues"))]);
        let mut lines = csv.split("\r\n");
        let header = lines.next().unwrap();
        let data = lines.next().unwrap();
        assert_eq!(header.split(',').count(), COLUMNS.len());
        // The data row has one extra comma-separated piece because a quoted field contains a comma.
        assert!(data.starts_with("1,2,Ryan,\"O'Reilly, Jr\","));
    }

    #[test]
    fn fields_with_commas_and_quotes_are_escaped() {
        assert_eq!(csv_field("plain"), "plain");
        assert_eq!(csv_field("a,b"), "\"a,b\"");
        assert_eq!(csv_field("say \"hi\""), "\"say \"\"hi\"\"\"");
    }

    #[test]
    fn missing_team_and_slug_render_empty() {
        let mut r = row("Ryan", None);
        r.slug = None;
        let csv = manifest_to_csv(&[r]);
        let data = csv.split("\r\n").nth(1).unwrap();
        // slug empty, team empty — both render as bare empty fields, not "None".
        assert!(data.contains(",,C,,NHL,"), "got: {data}");
    }

    #[test]
    fn summary_counts_each_signal() {
        let mut a = row("A", Some("T"));
        let mut b = row("B", Some("T"));
        b.has_portrait = false;
        b.has_head_mesh = true;
        b.slug = None;
        a.has_head_mesh = false;
        let s = summarize(&[a, b]);
        assert_eq!(
            (s.players, s.with_portrait, s.with_head_mesh, s.with_slug),
            (2, 1, 1, 1)
        );
    }
}
