//! Shared row model — the plain structs that cross the build↔store boundary. These mirror the
//! [`crate::schema`] tables. `tdb-fhm-store` returns them; `studio-domain` maps them to view-models.

use serde::{Deserialize, Serialize};

/// One human, unified across seasons. The Historical Players master-list row.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonSummary {
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub dob: String,
    pub nationality: Option<String>,
    pub is_goalie: bool,
    /// Derived primary position label (C/LW/RW/D/G), or None when the person has no ratings row.
    pub position: Option<String>,
    /// First season the person logged games (not the raw first appearance).
    pub first_year: Option<i64>,
    /// Final season the person logged games (not the raw last appearance — phantom post-career
    /// re-listings are excluded).
    pub last_year: Option<i64>,
    pub season_count: i64,
    pub retired: bool,
    /// True when the person's final games-played season is the corpus's current season (→ "Present").
    pub active: bool,
    /// Number of distinct seasons the person actually played games — the list's headline count.
    pub seasons_played: i64,
}

/// The full person detail: canonical bio + the season-by-season snapshots + career + ratings.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonDetail {
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub nick_name: Option<String>,
    pub dob: String,
    pub height: Option<i64>,
    pub weight: Option<i64>,
    pub hand: Option<String>,
    pub nationality: Option<String>,
    pub birthcity: Option<String>,
    pub birthstate: Option<String>,
    pub retired: bool,
    pub is_goalie: bool,
    pub position: Option<String>,
    pub first_year: Option<i64>,
    pub last_year: Option<i64>,
    pub seasons: Vec<PlayerSeasonRow>,
    pub career: Vec<CareerStatRow>,
    /// Every league draft the person was selected in, NHL/major drafts first (the primary display),
    /// then junior/import drafts. Empty for the never-drafted (pre-draft era, defectors, undrafted).
    pub drafts: Vec<DraftRow>,
    pub contract: Option<ContractRow>,
    pub awards: Vec<AwardRow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContractRow {
    pub team_abbr: Option<String>,
    pub ntc: bool,
    pub nmc: bool,
    pub elc: bool,
    pub ufa: bool,
    pub scholarship: Option<String>,
    /// FHM "Average Salary" (thousands).
    pub avg_salary: Option<i64>,
    /// Years remaining (count of Major-league years carrying a salary).
    pub length: Option<i64>,
}

/// A person's roster snapshot in one season (team resolved to name within that season's namespace).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerSeasonRow {
    pub season_year: i64,
    pub team_id: Option<i64>,
    pub team_name: Option<String>,
    pub team_abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub height: Option<i64>,
    pub weight: Option<i64>,
    /// Position grades G/LD/RD/LW/C/RW from that season's ratings (for a compact per-season row).
    pub ability: Option<f64>,
    pub potential: Option<f64>,
}

/// One career-stats line (regular or playoffs, skater or goalie). Stats are a JSON name→value map.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CareerStatRow {
    pub kind: String,
    pub phase: String,
    pub year: i64,
    pub team_abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub gp: Option<i64>,
    pub stats_json: String,
}

/// One draft selection. `team` is the era-resolved drafting-team display (the historical name where
/// the franchise lineage supplies it, else the snapshot abbr). `pick` is None when FHM only recorded
/// the overall position (its `10000` sentinel); `overall` is the reliable position. `draft_name` is the
/// resolved draft label (e.g. "NHL Entry Draft", "QMJHL Entry Draft"); `is_nhl` flags the NHL/major
/// drafts that lead the display.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DraftRow {
    pub year: Option<i64>,
    pub round: Option<i64>,
    pub pick: Option<i64>,
    pub overall: Option<i64>,
    pub team: Option<String>,
    pub draft_name: Option<String>,
    pub is_nhl: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwardRow {
    pub year: Option<i64>,
    pub league_abbr: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

/// A team as it existed in one season (the Historical Teams master-list row is season-scoped).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSeasonSummary {
    pub season_year: i64,
    pub src_team_id: i64,
    pub league_abbr: Option<String>,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub abbr: Option<String>,
}

/// Full team-season detail: identity + that season's roster (each entry links back to a person).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSeasonDetail {
    pub season_year: i64,
    pub src_team_id: i64,
    pub name: Option<String>,
    pub nickname: Option<String>,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub conference_name: Option<String>,
    pub division_name: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub roster: Vec<TeamRosterEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRosterEntry {
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub is_goalie: bool,
    pub ability: Option<f64>,
    pub potential: Option<f64>,
}

/// One entry in the franchise-consolidated team index (the Historical Teams browse). Either an NHL
/// **franchise** — one row that spans all its relocations/renames — or a **standalone** team (a
/// non-franchise `team_key`, i.e. every non-NHL club). `key` is the franchise id (as a string) or the
/// `team_key`; `kind` disambiguates. Span is the min/max season the entry actually iced a team.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamIndexEntry {
    /// `"franchise"` | `"team"`.
    pub kind: String,
    pub key: String,
    pub name: String,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub first_year: Option<i64>,
    pub last_year: Option<i64>,
    pub is_defunct: bool,
}

/// The franchise/team detail: a franchise-level header + the seasons it existed + the constituent
/// team-season block(s) for the selected season. Usually one constituent; more when concurrent clubs
/// co-existed under one franchise (e.g. the California Golden Seals and Minnesota North Stars both
/// under the Dallas Stars lineage in 1975) — the "show all constituents" case.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamFranchiseDetail {
    /// `"franchise"` | `"team"`.
    pub kind: String,
    pub key: String,
    /// Franchise `display_name` (modern/canonical), or the standalone team's name.
    pub display_name: String,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub is_defunct: bool,
    pub notes: Option<String>,
    /// Seasons this entry iced a team, newest first (the per-team season dropdown).
    pub seasons: Vec<i64>,
    /// The selected season the constituents describe.
    pub season_year: i64,
    pub constituents: Vec<TeamSeasonBlock>,
}

/// One club as it existed in one season — the rich detail unit (identity + colours + affiliates +
/// roster + cap sheet). `name` is the **era-appropriate** name: a franchise member's era name (our
/// corpus carries only the modern name), or the standalone team's own name.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamSeasonBlock {
    pub team_key: Option<String>,
    pub src_team_id: i64,
    pub season_year: i64,
    pub name: String,
    pub nickname: Option<String>,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
    pub conference_name: Option<String>,
    pub division_name: Option<String>,
    pub primary_color: Option<String>,
    pub secondary_color: Option<String>,
    pub text_color: Option<String>,
    pub affiliates: Vec<TeamAffiliate>,
    pub roster: Vec<TeamRosterMember>,
    pub contracts: Vec<TeamContractLine>,
}

/// A related club that season (resolved to its name within the season namespace). `relation` is
/// `"affiliate"` (a farm team *under* this club) or `"parent"` (the club this one reports up to) —
/// the `team_parent` edge read in both directions so an NHL club shows its AHL/ECHL affiliates and a
/// minor club shows its NHL parent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamAffiliate {
    /// `"affiliate"` | `"parent"`.
    pub relation: String,
    pub src_team_id: i64,
    pub name: Option<String>,
    pub abbr: Option<String>,
    pub league_abbr: Option<String>,
}

/// A roster member of a team-season, enriched with position, jersey, captaincy, DOB and nationality.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamRosterMember {
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub is_goalie: bool,
    pub position: Option<String>,
    pub dob: String,
    pub nationality: Option<String>,
    pub jersey: Option<i64>,
    /// `"C"` (captain) | `"A"` (alternate) | None.
    pub captaincy: Option<String>,
    pub ability: Option<f64>,
    pub potential: Option<f64>,
}

/// One line of a team-season cap sheet (from `season_contract`, dollars).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TeamContractLine {
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    pub avg_salary: Option<i64>,
    pub ntc: bool,
    pub nmc: bool,
    pub elc: bool,
    pub ufa: bool,
}

/// A `(year, label)` option for the season filter dropdown.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SeasonOption {
    pub year: i64,
    pub label: String,
}

/// A `(id, name)` option for the league filter dropdown (ids vary per season, so filter by name).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LeagueOption {
    pub abbr: String,
    pub name: String,
}
