//! `tdb-fhm-store` — the read-mostly access layer over the FHM SQLite corpus. Opens a corpus,
//! verifies its [`tdb_fhm::SCHEMA_VERSION`], and answers the search/filter/detail queries the
//! Historical browse needs, returning the shared [`tdb_fhm::model`] structs.
//!
//! Everything here is read-only; the corpus is a source, never edited.

use std::cell::OnceCell;
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use rusqlite::{params, Connection, OptionalExtension, Row};
use serde::{Deserialize, Serialize};

use tdb_fhm::model::{
    AwardRow, CareerStatRow, ContractRow, DraftRow, LeagueOption, PersonDetail, PersonSummary,
    PlayerSeasonRow, SeasonOption, TeamAffiliate, TeamContractLine, TeamFranchiseDetail,
    TeamIndexEntry, TeamRosterEntry, TeamRosterMember, TeamSeasonBlock, TeamSeasonDetail,
    TeamSeasonSummary,
};
use tdb_fhm::SCHEMA_VERSION;

/// A read-only handle to an FHM corpus.
pub struct FhmCorpus {
    conn: Connection,
    /// Lazily-computed, per-position FHM attribute maxima (for the formula PositionMax mode).
    maxima: OnceCell<Arc<AttributeMaxima>>,
}

/// Aggregated NHL regular-season career stats for the formula prebuilts (see `tdb-formula`).
#[derive(Debug, Clone, Default)]
pub struct CareerAgg {
    pub career_g: u32,
    pub career_a: u32,
    pub career_gp: u32,
    pub recent_gp: u32,
    pub recent_goals: u32,
    /// League schedule length proxy for the recent year (max GP any NHL player logged that year).
    pub recent_team_schedule: u32,
}

/// Per-position corpus-wide max FHM value for each attribute (PositionMax normalization).
/// `Serialize`/`Deserialize` so it can be cached to a sidecar file (the scan is expensive).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AttributeMaxima {
    pub forward: BTreeMap<String, f64>,
    pub defenseman: BTreeMap<String, f64>,
    pub goalie: BTreeMap<String, f64>,
}

/// Minimal person identity for the formula engine.
#[derive(Debug, Clone)]
pub struct PersonIdentity {
    pub first_name: String,
    pub last_name: String,
    pub position: Option<String>,
    pub is_goalie: bool,
}

/// Errors the store can surface.
#[derive(Debug)]
pub enum FhmError {
    Sqlite(rusqlite::Error),
    /// The corpus schema version does not match this build's [`SCHEMA_VERSION`].
    SchemaMismatch { found: i64, expected: i64 },
}

impl std::fmt::Display for FhmError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FhmError::Sqlite(e) => write!(f, "corpus query failed: {e}"),
            FhmError::SchemaMismatch { found, expected } => write!(
                f,
                "FHM corpus schema v{found} is not supported (this build expects v{expected}); \
                 rebuild the data pack"
            ),
        }
    }
}
impl std::error::Error for FhmError {}
impl From<rusqlite::Error> for FhmError {
    fn from(e: rusqlite::Error) -> Self {
        FhmError::Sqlite(e)
    }
}

type Res<T> = Result<T, FhmError>;

impl FhmCorpus {
    /// Open a corpus file (read-only) and verify its schema version.
    pub fn open(path: &Path) -> Res<Self> {
        let conn = Connection::open_with_flags(
            path,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
        )?;
        Self::from_conn(conn)
    }

    /// Wrap an already-open connection (used by tests over an in-memory corpus).
    pub fn from_conn(conn: Connection) -> Res<Self> {
        let found: i64 = conn
            .query_row(
                "SELECT CAST(value AS INTEGER) FROM meta WHERE key='schema_version'",
                [],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(0);
        if found != SCHEMA_VERSION {
            return Err(FhmError::SchemaMismatch {
                found,
                expected: SCHEMA_VERSION,
            });
        }
        Self::register_functions(&conn)?;
        Ok(Self {
            conn,
            maxima: OnceCell::new(),
        })
    }

    /// Register the app-defined SQL helpers the queries rely on. `fold_name(text)` lowercases and
    /// ASCII-folds Latin diacritics (via the same [`tdb_fhm::slug::normalize_name`] used to build
    /// person slugs), so search can match "sundstrom" against "Sundström". Deterministic, so SQLite
    /// may cache/optimize it.
    fn register_functions(conn: &Connection) -> Res<()> {
        use rusqlite::functions::FunctionFlags;
        conn.create_scalar_function(
            "fold_name",
            1,
            FunctionFlags::SQLITE_UTF8 | FunctionFlags::SQLITE_DETERMINISTIC,
            |ctx| {
                let text = ctx.get_raw(0).as_str().unwrap_or("");
                Ok(tdb_fhm::slug::normalize_name(text))
            },
        )?;
        Ok(())
    }

    /// A `meta` value by key.
    pub fn meta(&self, key: &str) -> Res<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM meta WHERE key=?1", params![key], |r| {
                r.get(0)
            })
            .optional()?)
    }

    /// Distinct seasons present (newest first), for the season filter dropdown.
    pub fn seasons(&self) -> Res<Vec<SeasonOption>> {
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT season_year FROM player_season ORDER BY season_year DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                let year: i64 = r.get(0)?;
                Ok(SeasonOption {
                    year,
                    label: format!("{}-{}", year - 1, year),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The newest season the corpus covers — the "current" season, used to render a still-active
    /// player's career end as "Present" rather than that year. `None` for an empty corpus.
    pub fn latest_season(&self) -> Res<Option<i64>> {
        // MAX over an empty table still yields one NULL row, so `Option<i64>` covers "no corpus".
        Ok(self.conn.query_row(
            "SELECT MAX(season_year) FROM player_season",
            [],
            |r| r.get::<_, Option<i64>>(0),
        )?)
    }

    /// Distinct leagues present (by abbr), for the league filter dropdown.
    pub fn leagues(&self) -> Res<Vec<LeagueOption>> {
        let mut stmt = self.conn.prepare(
            "SELECT abbr, MIN(name) FROM league_season
             WHERE abbr IS NOT NULL AND abbr <> ''
             GROUP BY abbr ORDER BY abbr",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(LeagueOption {
                    abbr: r.get(0)?,
                    name: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Person master-list (the Historical Players browse). Only lists people who actually carry FHM
    /// ratings (excludes staff / unrated historical entries). `search` matches "last first" / "first
    /// last" case- and diacritic-insensitively (both sides folded via `fold_name`, so e.g.
    /// "sundstrom" matches "Sundström" and "o'reilly" matches "O'Reilly"); `goalie_filter` is a
    /// mutually-exclusive skater/goalie toggle (`None` = all). Results cap at `limit`, ordered by
    /// last then first name.
    pub fn person_summaries(
        &self,
        search: &str,
        goalie_filter: Option<bool>,
        limit: i64,
    ) -> Res<Vec<PersonSummary>> {
        // Fold the needle the same way `fold_name` folds each row, so both sides share one
        // normalization (lowercased, ASCII-folded, punctuation-stripped). This also neutralizes LIKE
        // wildcards in user input — `normalize_name` drops `%`/`_` as non-alphanumeric.
        let needle = tdb_fhm::slug::normalize_name(search);
        let like = format!("%{needle}%");
        let goalie_clause = match goalie_filter {
            Some(true) => "AND is_goalie = 1",
            Some(false) => "AND is_goalie = 0",
            None => "",
        };
        // The corpus's current season decides "active" (→ "Present"). `-1` never matches a real year.
        let current = self.latest_season()?.unwrap_or(-1);
        // A "played season" is one the player was actually rated in — a `ratings_season` row. FHM
        // re-lists retired players every later season with NO ratings (a `retired` snapshot), so this
        // both trims the phantom tail AND, unlike career stats, works for the ~40% of players (Euro /
        // junior / minor leagues) who carry no `career_stat` lines at all. First/last/count all
        // derive from it; the raw person span is a fallback for the (rare) rated-but-spanless person.
        let sql = format!(
            "SELECT person_id, first_name, last_name, dob, nationality, is_goalie,
                    (SELECT MIN(rs.season_year) FROM ratings_season rs WHERE rs.person_id = person.person_id),
                    (SELECT MAX(rs.season_year) FROM ratings_season rs WHERE rs.person_id = person.person_id),
                    season_count, retired, position, first_year, last_year,
                    (SELECT COUNT(*) FROM ratings_season rs WHERE rs.person_id = person.person_id)
             FROM person
             WHERE (?1 = ''
                    OR fold_name(last_name || ' ' || first_name) LIKE ?2
                    OR fold_name(first_name || ' ' || last_name) LIKE ?2)
             {goalie_clause}
             AND EXISTS (SELECT 1 FROM ratings_season r WHERE r.person_id = person.person_id)
             ORDER BY last_name, first_name
             LIMIT ?3"
        );
        let mut stmt = self.conn.prepare(&sql)?;
        let rows = stmt
            .query_map(params![needle, like, limit], |r| {
                person_summary_row(r, current)
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The full person detail: canonical bio + season snapshots + career + draft + awards.
    pub fn person_detail(&self, person_id: i64) -> Res<Option<PersonDetail>> {
        let base = self
            .conn
            .query_row(
                "SELECT person_id, first_name, last_name, nick_name, dob, height, weight, hand,
                        nationality, birthcity, birthstate, retired, is_goalie, first_year, last_year,
                        position
                 FROM person WHERE person_id = ?1",
                params![person_id],
                |r| {
                    Ok(PersonDetail {
                        person_id: r.get(0)?,
                        first_name: r.get(1)?,
                        last_name: r.get(2)?,
                        nick_name: r.get(3)?,
                        dob: r.get(4)?,
                        height: r.get(5)?,
                        weight: r.get(6)?,
                        hand: r.get(7)?,
                        nationality: r.get(8)?,
                        birthcity: r.get(9)?,
                        birthstate: r.get(10)?,
                        retired: r.get::<_, i64>(11)? != 0,
                        is_goalie: r.get::<_, i64>(12)? != 0,
                        first_year: r.get(13)?,
                        last_year: r.get(14)?,
                        position: r.get(15)?,
                        seasons: Vec::new(),
                        career: Vec::new(),
                        drafts: Vec::new(),
                        contract: None,
                        awards: Vec::new(),
                    })
                },
            )
            .optional()?;
        let mut detail = match base {
            Some(d) => d,
            None => return Ok(None),
        };
        detail.seasons = self.person_seasons(person_id)?;
        detail.career = self.person_career(person_id)?;
        detail.drafts = self.person_drafts(person_id)?;
        detail.contract = self.person_contract(person_id)?;
        detail.awards = self.person_awards(person_id)?;
        Ok(Some(detail))
    }

    fn person_seasons(&self, person_id: i64) -> Res<Vec<PlayerSeasonRow>> {
        // Join each season snapshot to its team name (via the season-namespaced team_season table)
        // and its ratings headline. Newest season first.
        //
        // FHM keeps re-listing a player in every later season once they stop playing (often with no
        // team, `team_id = -1`), which surfaces as phantom seasons. Those re-listings carry no
        // ratings, so requiring a `ratings_season` row (INNER JOIN) keeps exactly the seasons the
        // player was actually rated/active in. This works for every league — unlike career stats,
        // which are absent for ~40% of players (Euro / junior / minor leagues).
        let mut stmt = self.conn.prepare(
            "SELECT ps.season_year, ps.team_id, t.name, t.abbr, lg.abbr,
                    ps.height, ps.weight, r.ability, r.potential
             FROM player_season ps
             JOIN ratings_season r
                    ON r.person_id = ps.person_id AND r.season_year = ps.season_year
             LEFT JOIN team_season t
                    ON t.season_year = ps.season_year AND t.src_team_id = ps.team_id
             LEFT JOIN league_season lg
                    ON lg.season_year = ps.season_year AND lg.src_league_id = t.league_id
             WHERE ps.person_id = ?1
             ORDER BY ps.season_year DESC",
        )?;
        let rows = stmt
            .query_map(params![person_id], |r| {
                Ok(PlayerSeasonRow {
                    season_year: r.get(0)?,
                    team_id: r.get(1)?,
                    team_name: r.get(2)?,
                    team_abbr: r.get(3)?,
                    league_abbr: r.get(4)?,
                    height: r.get(5)?,
                    weight: r.get(6)?,
                    ability: r.get(7)?,
                    potential: r.get(8)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The season years a person was actually rated/active — exactly the set the detail's season
    /// dropdown offers (newest first). Drives the career/peak rating compilations. Sourced from
    /// `ratings_season` (the phantom retired re-listings carry no ratings).
    pub fn person_played_years(&self, person_id: i64) -> Res<Vec<i64>> {
        let mut stmt = self.conn.prepare(
            "SELECT season_year FROM ratings_season WHERE person_id = ?1 ORDER BY season_year DESC",
        )?;
        let years = stmt
            .query_map(params![person_id], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(years)
    }

    fn person_career(&self, person_id: i64) -> Res<Vec<CareerStatRow>> {
        // The full career record. `career_stat` holds only real reported career lines (it carries no
        // phantom re-listings), so no games-played filter is applied here — that would wrongly drop
        // legitimate lines and is not the phantom-season signal (ratings absence is; see
        // `person_played_years`).
        let mut stmt = self.conn.prepare(
            "SELECT kind, phase, year, team_abbr, league_abbr, gp, stats_json
             FROM career_stat WHERE person_id = ?1
             ORDER BY kind, phase, year",
        )?;
        let rows = stmt
            .query_map(params![person_id], |r| {
                Ok(CareerStatRow {
                    kind: r.get(0)?,
                    phase: r.get(1)?,
                    year: r.get(2)?,
                    team_abbr: r.get(3)?,
                    league_abbr: r.get(4)?,
                    gp: r.get(5)?,
                    stats_json: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn person_drafts(&self, person_id: i64) -> Res<Vec<DraftRow>> {
        // Every draft the person was selected in. NHL/major drafts first (they lead the display),
        // then most-recent; `team` prefers the era-resolved name, `draft_name`/`is_nhl` come from the
        // draft-league label. src_season is the id namespace for both team and draft names.
        let mut stmt = self.conn.prepare(
            "SELECT d.year, d.round, d.pick, d.overall,
                    COALESCE(d.team_name, d.team_abbr), sd.name,
                    CASE WHEN sd.name LIKE 'NHL%' THEN 1 ELSE 0 END AS is_nhl
             FROM draft_info d
             LEFT JOIN season_draft sd
                    ON sd.season_year = d.src_season AND sd.draft_id = d.draft_id
             WHERE d.person_id = ?1
             ORDER BY is_nhl DESC, d.year DESC",
        )?;
        let rows = stmt
            .query_map(params![person_id], |r| {
                Ok(DraftRow {
                    year: r.get(0)?,
                    round: r.get(1)?,
                    pick: r.get(2)?,
                    overall: r.get(3)?,
                    team: r.get(4)?,
                    draft_name: r.get(5)?,
                    is_nhl: r.get::<_, i64>(6)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn person_contract(&self, person_id: i64) -> Res<Option<ContractRow>> {
        Ok(self
            .conn
            .query_row(
                "SELECT team_abbr, ntc, nmc, elc, ufa, scholarship, avg_salary, length
                 FROM contract WHERE person_id = ?1",
                params![person_id],
                |r| {
                    Ok(ContractRow {
                        team_abbr: r.get(0)?,
                        ntc: r.get::<_, i64>(1)? != 0,
                        nmc: r.get::<_, i64>(2)? != 0,
                        elc: r.get::<_, i64>(3)? != 0,
                        ufa: r.get::<_, i64>(4)? != 0,
                        scholarship: r.get(5)?,
                        avg_salary: r.get(6)?,
                        length: r.get(7)?,
                    })
                },
            )
            .optional()?)
    }

    fn person_awards(&self, person_id: i64) -> Res<Vec<AwardRow>> {
        let mut stmt = self.conn.prepare(
            "SELECT year, league_abbr, name, description FROM award
             WHERE person_id = ?1 ORDER BY year",
        )?;
        let rows = stmt
            .query_map(params![person_id], |r| {
                Ok(AwardRow {
                    year: r.get(0)?,
                    league_abbr: r.get(1)?,
                    name: r.get(2)?,
                    description: r.get(3)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Team master-list for one season (the Historical Teams browse is season-scoped, since team ids
    /// are per-season). `search` matches name/nickname/abbr.
    pub fn team_summaries(
        &self,
        season_year: i64,
        search: &str,
        limit: i64,
    ) -> Res<Vec<TeamSeasonSummary>> {
        let needle = search.trim().to_lowercase();
        let like = format!("%{}%", needle.replace('%', ""));
        let mut stmt = self.conn.prepare(
            "SELECT t.season_year, t.src_team_id, lg.abbr, t.name, t.nickname, t.abbr
             FROM team_season t
             LEFT JOIN league_season lg
                    ON lg.season_year = t.season_year AND lg.src_league_id = t.league_id
             WHERE t.season_year = ?1
               AND (?2 = '' OR lower(t.name || ' ' || IFNULL(t.nickname,'') || ' ' || IFNULL(t.abbr,'')) LIKE ?3)
             ORDER BY t.name
             LIMIT ?4",
        )?;
        let rows = stmt
            .query_map(params![season_year, needle, like, limit], |r| {
                Ok(TeamSeasonSummary {
                    season_year: r.get(0)?,
                    src_team_id: r.get(1)?,
                    league_abbr: r.get(2)?,
                    name: r.get(3)?,
                    nickname: r.get(4)?,
                    abbr: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Full team-season detail: identity + that season's roster (players link back to their person).
    pub fn team_detail(&self, season_year: i64, src_team_id: i64) -> Res<Option<TeamSeasonDetail>> {
        let base = self
            .conn
            .query_row(
                "SELECT t.season_year, t.src_team_id, t.name, t.nickname, t.abbr, lg.abbr,
                        c.name, d.name, t.primary_color, t.secondary_color
                 FROM team_season t
                 LEFT JOIN league_season lg
                        ON lg.season_year = t.season_year AND lg.src_league_id = t.league_id
                 LEFT JOIN conference_season c
                        ON c.season_year = t.season_year AND c.league_id = t.league_id
                       AND c.conference_id = t.conference_id
                 LEFT JOIN division_season d
                        ON d.season_year = t.season_year AND d.league_id = t.league_id
                       AND d.conference_id = t.conference_id AND d.division_id = t.division_id
                 WHERE t.season_year = ?1 AND t.src_team_id = ?2",
                params![season_year, src_team_id],
                |r| {
                    Ok(TeamSeasonDetail {
                        season_year: r.get(0)?,
                        src_team_id: r.get(1)?,
                        name: r.get(2)?,
                        nickname: r.get(3)?,
                        abbr: r.get(4)?,
                        league_abbr: r.get(5)?,
                        conference_name: r.get(6)?,
                        division_name: r.get(7)?,
                        primary_color: r.get(8)?,
                        secondary_color: r.get(9)?,
                        roster: Vec::new(),
                    })
                },
            )
            .optional()?;
        let mut detail = match base {
            Some(d) => d,
            None => return Ok(None),
        };
        let mut stmt = self.conn.prepare(
            "SELECT p.person_id, p.first_name, p.last_name, p.is_goalie, r.ability, r.potential
             FROM player_season ps
             JOIN person p ON p.person_id = ps.person_id
             LEFT JOIN ratings_season r
                    ON r.person_id = ps.person_id AND r.season_year = ps.season_year
             WHERE ps.season_year = ?1 AND ps.team_id = ?2
             ORDER BY r.ability DESC NULLS LAST, p.last_name",
        )?;
        detail.roster = stmt
            .query_map(params![season_year, src_team_id], |r| {
                Ok(TeamRosterEntry {
                    person_id: r.get(0)?,
                    first_name: r.get(1)?,
                    last_name: r.get(2)?,
                    is_goalie: r.get::<_, i64>(3)? != 0,
                    ability: r.get(4)?,
                    potential: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Some(detail))
    }

    // ---- franchise-consolidated team browse (v5) -------------------------

    /// Distinct leagues across the cross-season `team` identity table (for the team-browse league
    /// filter). Every NHL club is consolidated into a franchise, but `NHL` still appears here (its
    /// `team_key`s exist in `team`), so selecting it lists the franchises.
    pub fn team_leagues(&self) -> Res<Vec<LeagueOption>> {
        let mut stmt = self.conn.prepare(
            "SELECT league_abbr, MIN(COALESCE(league_name, league_abbr)) FROM team
             WHERE league_abbr IS NOT NULL AND league_abbr <> ''
             GROUP BY league_abbr ORDER BY league_abbr",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(LeagueOption {
                    abbr: r.get(0)?,
                    name: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// The franchise-consolidated team index: NHL **franchises** (one row each, spanning all their
    /// relocations/renames) unioned with **standalone** teams (every `team_key` not part of a
    /// franchise — i.e. all non-NHL clubs). `league` filters by league abbr (`"NHL"` → the franchises;
    /// any other → that league's standalone teams; `None` → everything). `search` matches the display
    /// name. Merged, name-sorted, capped at `limit`.
    pub fn team_index(
        &self,
        league: Option<&str>,
        search: &str,
        limit: i64,
    ) -> Res<Vec<TeamIndexEntry>> {
        let needle = search.trim().to_lowercase();
        let like = format!("%{}%", needle.replace('%', ""));
        let mut out: Vec<TeamIndexEntry> = Vec::new();

        // Franchises are all NHL, so they belong only to the "all" and "NHL" views.
        let include_franchises = league.map_or(true, |l| l.eq_ignore_ascii_case("NHL"));
        if include_franchises {
            let mut stmt = self.conn.prepare(
                "SELECT f.franchise_id, f.display_name, f.abbr, f.is_defunct,
                        MIN(ts.season_year), MAX(ts.season_year)
                 FROM franchise f
                 JOIN franchise_member m ON m.franchise_id = f.franchise_id
                 LEFT JOIN team_season ts ON ts.team_key = m.team_key
                        AND (m.era_start IS NULL OR ts.season_year > m.era_start)
                        AND (m.era_end IS NULL OR ts.season_year <= m.era_end)
                 WHERE (?1 = '' OR lower(f.display_name) LIKE ?2)
                 GROUP BY f.franchise_id
                 ORDER BY f.display_name",
            )?;
            let rows = stmt
                .query_map(params![needle, like], |r| {
                    let fid: i64 = r.get(0)?;
                    Ok(TeamIndexEntry {
                        kind: "franchise".to_string(),
                        key: fid.to_string(),
                        name: r.get(1)?,
                        abbr: r.get(2)?,
                        league_abbr: Some("NHL".to_string()),
                        first_year: r.get(4)?,
                        last_year: r.get(5)?,
                        is_defunct: r.get::<_, i64>(3)? != 0,
                    })
                })?
                .collect::<Result<Vec<_>, _>>()?;
            out.extend(rows);
        }

        // Standalone teams: every `team_key` that is not a franchise member. The league passes
        // straight through — `"NHL"` here matches only standalone NHL clubs (none exist, since every
        // NHL team_key is a franchise member), so the NHL view is exactly the franchises.
        let league_param = league.unwrap_or("");
        let mut stmt = self.conn.prepare(
            "SELECT t.team_key, t.name, t.abbr, t.league_abbr,
                    MIN(ts.season_year), MAX(ts.season_year)
             FROM team t
             LEFT JOIN team_season ts ON ts.team_key = t.team_key
             WHERE t.team_key NOT IN (SELECT team_key FROM franchise_member)
               AND (?1 = '' OR t.league_abbr = ?1)
               AND (?2 = '' OR lower(IFNULL(t.name,'')) LIKE ?3)
             GROUP BY t.team_key
             ORDER BY t.name",
        )?;
        let rows = stmt
            .query_map(params![league_param, needle, like], |r| {
                Ok(TeamIndexEntry {
                    kind: "team".to_string(),
                    key: r.get(0)?,
                    name: r.get::<_, Option<String>>(1)?.unwrap_or_default(),
                    abbr: r.get(2)?,
                    league_abbr: r.get(3)?,
                    first_year: r.get(4)?,
                    last_year: r.get(5)?,
                    is_defunct: false,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        // When the NHL filter is active there are no standalone NHL teams, so `rows` is empty there.
        out.extend(rows);

        out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        out.truncate(limit.max(0) as usize);
        Ok(out)
    }

    /// The franchise/standalone-team detail for a selected season. `kind` is `"franchise"` (key = the
    /// franchise id) or `"team"` (key = the `team_key`). `season_year` selects the season; `None`
    /// defaults to the newest the entry iced a team. Returns `None` when the key is unknown.
    pub fn team_franchise_detail(
        &self,
        kind: &str,
        key: &str,
        season_year: Option<i64>,
    ) -> Res<Option<TeamFranchiseDetail>> {
        match kind {
            "franchise" => match key.parse::<i64>() {
                Ok(fid) => self.franchise_detail(fid, season_year),
                Err(_) => Ok(None),
            },
            _ => self.standalone_team_detail(key, season_year),
        }
    }

    fn franchise_detail(
        &self,
        franchise_id: i64,
        season_year: Option<i64>,
    ) -> Res<Option<TeamFranchiseDetail>> {
        let base = self
            .conn
            .query_row(
                "SELECT display_name, abbr, is_defunct, notes FROM franchise WHERE franchise_id = ?1",
                params![franchise_id],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, i64>(2)? != 0,
                        r.get::<_, Option<String>>(3)?,
                    ))
                },
            )
            .optional()?;
        let Some((display_name, abbr, is_defunct, notes)) = base else {
            return Ok(None);
        };

        // Seasons the franchise iced a team, honouring each member's era bounds. Newest first.
        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT ts.season_year
             FROM franchise_member m
             JOIN team_season ts ON ts.team_key = m.team_key
                    AND (m.era_start IS NULL OR ts.season_year > m.era_start)
                    AND (m.era_end IS NULL OR ts.season_year <= m.era_end)
             WHERE m.franchise_id = ?1
             ORDER BY ts.season_year DESC",
        )?;
        let seasons: Vec<i64> = stmt
            .query_map(params![franchise_id], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        if seasons.is_empty() {
            return Ok(None);
        }
        let target = season_year.filter(|y| seasons.contains(y)).unwrap_or(seasons[0]);

        // Constituent (team_key, era_name, src_team_id) rows active that season.
        let mut cstmt = self.conn.prepare(
            "SELECT m.era_name, ts.src_team_id
             FROM franchise_member m
             JOIN team_season ts ON ts.team_key = m.team_key AND ts.season_year = ?2
             WHERE m.franchise_id = ?1
                    AND (m.era_start IS NULL OR ?2 > m.era_start)
                    AND (m.era_end IS NULL OR ?2 <= m.era_end)
             ORDER BY m.era_name",
        )?;
        let picks: Vec<(Option<String>, i64)> = cstmt
            .query_map(params![franchise_id, target], |r| Ok((r.get(0)?, r.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let mut constituents = Vec::new();
        for (era_name, src_team_id) in picks {
            if let Some(block) = self.team_season_block(target, src_team_id, era_name)? {
                constituents.push(block);
            }
        }

        Ok(Some(TeamFranchiseDetail {
            kind: "franchise".to_string(),
            key: franchise_id.to_string(),
            display_name,
            abbr,
            league_abbr: Some("NHL".to_string()),
            is_defunct,
            notes,
            seasons,
            season_year: target,
            constituents,
        }))
    }

    fn standalone_team_detail(
        &self,
        team_key: &str,
        season_year: Option<i64>,
    ) -> Res<Option<TeamFranchiseDetail>> {
        let base = self
            .conn
            .query_row(
                "SELECT name, abbr, league_abbr FROM team WHERE team_key = ?1",
                params![team_key],
                |r| {
                    Ok((
                        r.get::<_, Option<String>>(0)?,
                        r.get::<_, Option<String>>(1)?,
                        r.get::<_, Option<String>>(2)?,
                    ))
                },
            )
            .optional()?;
        let Some((name, abbr, league_abbr)) = base else {
            return Ok(None);
        };

        let mut stmt = self.conn.prepare(
            "SELECT DISTINCT season_year FROM team_season WHERE team_key = ?1
             ORDER BY season_year DESC",
        )?;
        let seasons: Vec<i64> = stmt
            .query_map(params![team_key], |r| r.get(0))?
            .collect::<Result<Vec<_>, _>>()?;
        if seasons.is_empty() {
            return Ok(None);
        }
        let target = season_year.filter(|y| seasons.contains(y)).unwrap_or(seasons[0]);

        let src_team_id: Option<i64> = self
            .conn
            .query_row(
                "SELECT src_team_id FROM team_season WHERE team_key = ?1 AND season_year = ?2",
                params![team_key, target],
                |r| r.get(0),
            )
            .optional()?;

        let mut constituents = Vec::new();
        if let Some(src_team_id) = src_team_id {
            // Standalone teams keep their corpus name (no era override).
            if let Some(block) = self.team_season_block(target, src_team_id, None)? {
                constituents.push(block);
            }
        }

        Ok(Some(TeamFranchiseDetail {
            kind: "team".to_string(),
            key: team_key.to_string(),
            display_name: name.unwrap_or_else(|| team_key.to_string()),
            abbr,
            league_abbr,
            is_defunct: false,
            notes: None,
            seasons,
            season_year: target,
            constituents,
        }))
    }

    /// Build one club's full season block: identity + colours + affiliates + roster + cap sheet.
    /// `name_override` supplies the era-appropriate name (franchise members); `None` keeps the
    /// season's own corpus name.
    fn team_season_block(
        &self,
        season_year: i64,
        src_team_id: i64,
        name_override: Option<String>,
    ) -> Res<Option<TeamSeasonBlock>> {
        let base = self
            .conn
            .query_row(
                "SELECT t.team_key, t.name, t.nickname, t.abbr, lg.abbr,
                        c.name, d.name, t.primary_color, t.secondary_color, t.text_color
                 FROM team_season t
                 LEFT JOIN league_season lg
                        ON lg.season_year = t.season_year AND lg.src_league_id = t.league_id
                 LEFT JOIN conference_season c
                        ON c.season_year = t.season_year AND c.league_id = t.league_id
                       AND c.conference_id = t.conference_id
                 LEFT JOIN division_season d
                        ON d.season_year = t.season_year AND d.league_id = t.league_id
                       AND d.conference_id = t.conference_id AND d.division_id = t.division_id
                 WHERE t.season_year = ?1 AND t.src_team_id = ?2",
                params![season_year, src_team_id],
                |r| {
                    Ok((
                        r.get::<_, Option<String>>(0)?, // team_key
                        r.get::<_, Option<String>>(1)?, // name
                        r.get::<_, Option<String>>(2)?, // nickname
                        r.get::<_, Option<String>>(3)?, // abbr
                        r.get::<_, Option<String>>(4)?, // league abbr
                        r.get::<_, Option<String>>(5)?, // conference
                        r.get::<_, Option<String>>(6)?, // division
                        r.get::<_, Option<String>>(7)?, // primary
                        r.get::<_, Option<String>>(8)?, // secondary
                        r.get::<_, Option<String>>(9)?, // text
                    ))
                },
            )
            .optional()?;
        let Some((
            team_key,
            name,
            nickname,
            abbr,
            league_abbr,
            conference_name,
            division_name,
            primary_color,
            secondary_color,
            text_color,
        )) = base
        else {
            return Ok(None);
        };
        let display = name_override
            .or(name)
            .unwrap_or_else(|| format!("Team {src_team_id}"));

        Ok(Some(TeamSeasonBlock {
            affiliates: self.team_affiliates(season_year, src_team_id)?,
            roster: self.team_roster(season_year, src_team_id, team_key.as_deref())?,
            contracts: self.team_contracts(season_year, src_team_id)?,
            team_key,
            src_team_id,
            season_year,
            name: display,
            nickname,
            abbr,
            league_abbr,
            conference_name,
            division_name,
            primary_color,
            secondary_color,
            text_color,
        }))
    }

    fn team_affiliates(&self, season_year: i64, src_team_id: i64) -> Res<Vec<TeamAffiliate>> {
        // Read the `team_parent` edge both ways: children (farm teams whose parent is this club) as
        // "affiliate", and this club's own parent as "parent". Affiliates first.
        let mut stmt = self.conn.prepare(
            "SELECT 'affiliate' AS relation, ch.src_team_id, ch.name, ch.abbr, lg.abbr
             FROM team_parent tp
             JOIN team_season ch
                    ON ch.season_year = tp.season_year AND ch.src_team_id = tp.src_team_id
             LEFT JOIN league_season lg
                    ON lg.season_year = ch.season_year AND lg.src_league_id = ch.league_id
             WHERE tp.season_year = ?1 AND tp.parent_team_id = ?2
             UNION ALL
             SELECT 'parent' AS relation, pt.src_team_id, pt.name, pt.abbr, lg.abbr
             FROM team_parent tp
             JOIN team_season pt
                    ON pt.season_year = tp.season_year AND pt.src_team_id = tp.parent_team_id
             LEFT JOIN league_season lg
                    ON lg.season_year = pt.season_year AND lg.src_league_id = pt.league_id
             WHERE tp.season_year = ?1 AND tp.src_team_id = ?2
             ORDER BY relation, 3",
        )?;
        let rows = stmt
            .query_map(params![season_year, src_team_id], |r| {
                Ok(TeamAffiliate {
                    relation: r.get(0)?,
                    src_team_id: r.get(1)?,
                    name: r.get(2)?,
                    abbr: r.get(3)?,
                    league_abbr: r.get(4)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn team_roster(
        &self,
        season_year: i64,
        src_team_id: i64,
        team_key: Option<&str>,
    ) -> Res<Vec<TeamRosterMember>> {
        // Jersey/captaincy are keyed by the cross-season `team_key`; use correlated subqueries so a
        // player wearing multiple numbers doesn't multiply the roster row (take the lowest number).
        let mut stmt = self.conn.prepare(
            "SELECT p.person_id, p.first_name, p.last_name, p.is_goalie, p.position, p.dob,
                    p.nationality, r.ability, r.potential,
                    (SELECT MIN(jn.number) FROM jersey_number jn
                       WHERE jn.person_id = ps.person_id AND jn.season_year = ps.season_year
                         AND jn.team_key = ?3),
                    (SELECT cap.role FROM captaincy cap
                       WHERE cap.person_id = ps.person_id AND cap.season_year = ps.season_year
                         AND cap.team_key = ?3 LIMIT 1)
             FROM player_season ps
             JOIN person p ON p.person_id = ps.person_id
             LEFT JOIN ratings_season r
                    ON r.person_id = ps.person_id AND r.season_year = ps.season_year
             WHERE ps.season_year = ?1 AND ps.team_id = ?2
             ORDER BY r.ability DESC NULLS LAST, p.last_name",
        )?;
        let rows = stmt
            .query_map(params![season_year, src_team_id, team_key], |r| {
                Ok(TeamRosterMember {
                    person_id: r.get(0)?,
                    first_name: r.get(1)?,
                    last_name: r.get(2)?,
                    is_goalie: r.get::<_, i64>(3)? != 0,
                    position: r.get(4)?,
                    dob: r.get(5)?,
                    nationality: r.get(6)?,
                    ability: r.get(7)?,
                    potential: r.get(8)?,
                    jersey: r.get(9)?,
                    captaincy: r.get(10)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    fn team_contracts(&self, season_year: i64, src_team_id: i64) -> Res<Vec<TeamContractLine>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.person_id, p.first_name, p.last_name, sc.avg_salary,
                    sc.ntc, sc.nmc, sc.elc, sc.ufa
             FROM season_contract sc
             JOIN person p ON p.person_id = sc.person_id
             WHERE sc.season_year = ?1 AND sc.team_id = ?2
             ORDER BY sc.avg_salary DESC NULLS LAST, p.last_name",
        )?;
        let rows = stmt
            .query_map(params![season_year, src_team_id], |r| {
                Ok(TeamContractLine {
                    person_id: r.get(0)?,
                    first_name: r.get(1)?,
                    last_name: r.get(2)?,
                    avg_salary: r.get(3)?,
                    ntc: r.get::<_, Option<i64>>(4)?.unwrap_or(0) != 0,
                    nmc: r.get::<_, Option<i64>>(5)?.unwrap_or(0) != 0,
                    elc: r.get::<_, Option<i64>>(6)?.unwrap_or(0) != 0,
                    ufa: r.get::<_, Option<i64>>(7)?.unwrap_or(0) != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    // ---- formula inputs (Stage 2) ----------------------------------------

    /// Lightweight identity (name + position) for one person — cheap inputs for the formula engine.
    pub fn person_identity(&self, person_id: i64) -> Res<Option<PersonIdentity>> {
        Ok(self
            .conn
            .query_row(
                "SELECT first_name, last_name, position, is_goalie FROM person WHERE person_id=?1",
                params![person_id],
                |r| {
                    Ok(PersonIdentity {
                        first_name: r.get(0)?,
                        last_name: r.get(1)?,
                        position: r.get(2)?,
                        is_goalie: r.get::<_, i64>(3)? != 0,
                    })
                },
            )
            .optional()?)
    }

    /// A person-season's FHM attribute values as a name→number map (parsed from `ratings_season`'s
    /// attribute JSON). Empty when the person has no ratings that season.
    pub fn player_fhm_values(
        &self,
        person_id: i64,
        season_year: i64,
    ) -> Res<BTreeMap<String, f64>> {
        let json: Option<String> = self
            .conn
            .query_row(
                "SELECT attrs_json FROM ratings_season WHERE person_id=?1 AND season_year=?2",
                params![person_id, season_year],
                |r| r.get(0),
            )
            .optional()?;
        Ok(json.map(|j| parse_fhm_json(&j)).unwrap_or_default())
    }

    /// Aggregate NHL regular-season career stats for the formula prebuilts.
    pub fn career_agg(&self, person_id: i64, is_goalie: bool) -> Res<CareerAgg> {
        let kind = if is_goalie { "goalie" } else { "skater" };
        let mut agg = CareerAgg::default();

        // Career-wide NHL RS rows: year, gp, stats_json (for G/A).
        let mut stmt = self.conn.prepare(
            "SELECT year, gp, stats_json FROM career_stat
             WHERE person_id=?1 AND kind=?2 AND phase='rs' AND league_abbr='NHL'",
        )?;
        let rows: Vec<(i64, Option<i64>, String)> = stmt
            .query_map(params![person_id, kind], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;

        let mut recent_year: Option<i64> = None;
        for (year, _gp, _json) in &rows {
            recent_year = Some(recent_year.map_or(*year, |y| y.max(*year)));
        }
        for (year, gp, json) in &rows {
            let g = stat_of(json, "G");
            let a = stat_of(json, "A");
            let gp = gp.unwrap_or(0).max(0) as u32;
            agg.career_g += g;
            agg.career_a += a;
            agg.career_gp += gp;
            if Some(*year) == recent_year {
                agg.recent_gp += gp;
                agg.recent_goals += g;
            }
        }

        // Schedule-length proxy: the max GP any NHL player logged that year.
        if let Some(year) = recent_year {
            let sched: Option<i64> = self
                .conn
                .query_row(
                    "SELECT MAX(gp) FROM career_stat WHERE year=?1 AND phase='rs' AND league_abbr='NHL'",
                    params![year],
                    |r| r.get(0),
                )
                .optional()?
                .flatten();
            agg.recent_team_schedule = sched.unwrap_or(0).max(0) as u32;
        }
        Ok(agg)
    }

    /// Corpus-wide per-position FHM attribute maxima (cached in-process; computed once on first use).
    /// Used only by the PositionMax normalization mode — a full scan of `ratings_season`, so it is
    /// lazy. Prefer [`compute_maxima_at`] off an independent connection when the caller must not hold
    /// a lock during the scan (see `studio-api`'s registry).
    pub fn attribute_maxima(&self) -> Res<Arc<AttributeMaxima>> {
        if let Some(m) = self.maxima.get() {
            return Ok(m.clone());
        }
        let computed = Arc::new(scan_attribute_maxima(&self.conn)?);
        // OnceCell: first writer wins; a racing compute would just be discarded (single-threaded here).
        let _ = self.maxima.set(computed.clone());
        Ok(self.maxima.get().cloned().unwrap_or(computed))
    }
}

/// Compute per-position attribute maxima from a **fresh** read-only connection opened at `path`.
/// This is the expensive `ratings_season` scan; running it on its own connection lets a caller do
/// it without holding whatever lock guards the primary [`FhmCorpus`] handle, so the rest of the app
/// stays responsive. Result is stable per data pack, so callers should cache it.
pub fn compute_maxima_at(path: &Path) -> Res<AttributeMaxima> {
    let conn = Connection::open_with_flags(
        path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY | rusqlite::OpenFlags::SQLITE_OPEN_URI,
    )?;
    scan_attribute_maxima(&conn)
}

/// The `ratings_season` scan itself: bucket each player-season by best position and track the max
/// observed value per FHM attribute. Shared by the in-corpus and independent-connection paths.
fn scan_attribute_maxima(conn: &Connection) -> Res<AttributeMaxima> {
    let mut out = AttributeMaxima::default();
    let mut stmt = conn.prepare("SELECT g, ld, rd, lw, c, rw, attrs_json FROM ratings_season")?;
    let mut rows = stmt.query([])?;
    while let Some(r) = rows.next()? {
        let g: i64 = r.get::<_, Option<i64>>(0)?.unwrap_or(0);
        let ld: i64 = r.get::<_, Option<i64>>(1)?.unwrap_or(0);
        let rd: i64 = r.get::<_, Option<i64>>(2)?.unwrap_or(0);
        let lw: i64 = r.get::<_, Option<i64>>(3)?.unwrap_or(0);
        let c: i64 = r.get::<_, Option<i64>>(4)?.unwrap_or(0);
        let rw: i64 = r.get::<_, Option<i64>>(5)?.unwrap_or(0);
        let json: String = r.get(6)?;
        let skater_max = [ld, rd, lw, c, rw].into_iter().max().unwrap_or(0);
        let bucket = if g > 0 && g >= skater_max {
            &mut out.goalie
        } else if ld.max(rd) > lw.max(c).max(rw) {
            &mut out.defenseman
        } else {
            &mut out.forward
        };
        for (k, v) in parse_fhm_json(&json) {
            let e = bucket.entry(k).or_insert(0.0);
            if v > *e {
                *e = v;
            }
        }
    }
    Ok(out)
}

/// Parse a stored attribute JSON object into a name→number map (non-numeric values dropped).
fn parse_fhm_json(json: &str) -> BTreeMap<String, f64> {
    let map: serde_json::Map<String, serde_json::Value> =
        serde_json::from_str(json).unwrap_or_default();
    map.into_iter()
        .filter_map(|(k, v)| v.as_f64().map(|n| (k, n)))
        .collect()
}

/// One integer stat from a career row's stats JSON (0 when absent/non-numeric).
fn stat_of(json: &str, key: &str) -> u32 {
    serde_json::from_str::<serde_json::Value>(json)
        .ok()
        .and_then(|v| v.get(key).and_then(|x| x.as_i64()))
        .unwrap_or(0)
        .max(0) as u32
}

fn person_summary_row(r: &Row, current_season: i64) -> rusqlite::Result<PersonSummary> {
    // Cols 6/7 are the games-played first/last; 11/12 the raw person span used only as a fallback.
    let gp_last: Option<i64> = r.get(7)?;
    Ok(PersonSummary {
        person_id: r.get(0)?,
        first_name: r.get(1)?,
        last_name: r.get(2)?,
        dob: r.get(3)?,
        nationality: r.get(4)?,
        is_goalie: r.get::<_, i64>(5)? != 0,
        first_year: r.get::<_, Option<i64>>(6)?.or(r.get(11)?),
        last_year: gp_last.or(r.get(12)?),
        season_count: r.get(8)?,
        retired: r.get::<_, i64>(9)? != 0,
        position: r.get(10)?,
        active: gp_last == Some(current_season),
        seasons_played: r.get(13)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tdb_fhm::SCHEMA_SQL;

    /// Build a tiny in-memory corpus with two people and one team so the queries have something to
    /// chew on without needing the multi-GB real data pack.
    fn fixture() -> FhmCorpus {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        conn.execute_batch(
            r#"
            INSERT INTO meta(key,value) VALUES('schema_version','6');
            INSERT INTO league_season VALUES(2024,0,'National Hockey League','NHL');
            INSERT INTO team_season VALUES(2024,15,'nhl__pit',0,'Pittsburgh','Penguins','PIT','#000','#fc0','#fff',0,0);
            INSERT INTO conference_season VALUES(2024,0,0,'Eastern Conference');
            INSERT INTO division_season VALUES(2024,0,0,0,'Metropolitan');
            INSERT INTO team VALUES('nhl__pit','Pittsburgh','Penguins','PIT','nhl','NHL','National Hockey League');
            INSERT INTO team VALUES('ahl__wbs','Wilkes-Barre/Scranton','Penguins','WBS','ahl','AHL','American Hockey League');
            INSERT INTO franchise VALUES(0,'Pittsburgh Penguins org','Pittsburgh Penguins','PIT',0,'Expansion 1967');
            INSERT INTO franchise_member VALUES(0,'nhl__pit',NULL,NULL,'Pittsburgh Penguins');
            INSERT INTO person VALUES(0,'crosby|sidney|1987-08-07','Sidney','Crosby',NULL,'1987-08-07',
                1987,71,201,'Left','Canada','Cole Harbour','NS',0,2006,2024,19,0,'C');
            INSERT INTO person VALUES(1,'fleury|marc-andre|1984-11-28','Marc-Andre','Fleury',NULL,
                '1984-11-28',1984,75,185,'Left','Canada','Sorel','QC',0,2004,2024,21,1,'G');
            INSERT INTO player_season VALUES(0,2024,31,15,15,71,201,'Cole Harbour','NS','Canada',NULL,NULL,0,'Left',0);
            INSERT INTO player_season VALUES(1,2024,42,15,15,75,185,'Sorel','QC','Canada',NULL,NULL,0,'Left',0);
            INSERT INTO ratings_season VALUES(0,2024,0,8,5,19,20,13,0.9,0.9,'{"Speed":18}');
            INSERT INTO ratings_season VALUES(1,2024,18,0,0,0,0,0,0.85,0.85,'{"Reflexes":17}');
            INSERT INTO career_stat VALUES(0,'skater','rs',2024,2024,15,0,'PIT','NHL',82,'{"G":33,"A":58}');
            INSERT INTO season_draft VALUES(2024,0,'NHL Entry Draft');
            INSERT INTO season_draft VALUES(2024,3,'QMJHL Entry Draft');
            -- Two drafts for one person: the NHL entry draft (era-resolved team, real pick) must lead;
            -- the earlier QMJHL junior draft (pick sentinel already nulled at ingest) follows.
            INSERT INTO draft_info VALUES(0,2005,1,1,1,2024,15,'PIT','Pittsburgh Penguins',0,NULL);
            INSERT INTO draft_info VALUES(0,2003,11,NULL,126,2024,15,'RIM',NULL,3,NULL);
            INSERT INTO award VALUES(0,2007,'NHL','Hart Trophy','MVP');
            INSERT INTO jersey_number VALUES(0,'nhl__pit',2024,87);
            INSERT INTO captaincy VALUES(0,'nhl__pit',2024,'C');
            INSERT INTO season_contract VALUES(0,2024,15,0,0,0,0,NULL,12600000);
            "#,
        )
        .unwrap();
        FhmCorpus::from_conn(conn).unwrap()
    }

    #[test]
    fn seasons_and_leagues() {
        let c = fixture();
        assert_eq!(c.seasons().unwrap()[0].label, "2023-2024");
        assert_eq!(c.leagues().unwrap()[0].abbr, "NHL");
    }

    #[test]
    fn person_search_and_goalie_filter() {
        let c = fixture();
        assert_eq!(c.person_summaries("cros", None, 50).unwrap().len(), 1);
        assert_eq!(c.person_summaries("sidney crosby", None, 50).unwrap().len(), 1);
        assert_eq!(c.person_summaries("", Some(true), 50).unwrap().len(), 1); // only Fleury
        assert_eq!(c.person_summaries("", Some(false), 50).unwrap().len(), 1); // only Crosby
        assert_eq!(c.person_summaries("", None, 50).unwrap().len(), 2);
    }

    #[test]
    fn search_is_diacritic_insensitive() {
        let c = fixture();
        // A rated skater whose name carries a diacritic FHM would export as "Sundström".
        c.conn
            .execute_batch(
                r#"
                INSERT INTO person VALUES(2,'sundstrom|erik|1990-01-01','Erik','Sundström',NULL,
                    '1990-01-01',1990,73,190,'Left','Sweden','Stockholm','',0,2010,2024,10,0,'C');
                INSERT INTO ratings_season VALUES(2,2024,0,8,5,19,20,13,0.8,0.8,'{"Speed":15}');
                "#,
            )
            .unwrap();
        let hits = |q: &str| c.person_summaries(q, None, 50).unwrap().len();
        assert_eq!(hits("sundstrom"), 1, "ASCII spelling should match the accented name");
        assert_eq!(hits("Sundström"), 1, "accented spelling still matches");
        assert_eq!(hits("erik sundstrom"), 1, "first-last order matches");
        assert_eq!(hits("sundstr"), 1, "partial substring matches");
        assert_eq!(hits("nomatch"), 0, "unrelated needle finds nothing");
    }

    #[test]
    fn person_detail_assembles_related_rows() {
        let c = fixture();
        let d = c.person_detail(0).unwrap().unwrap();
        assert_eq!(d.last_name, "Crosby");
        assert_eq!(d.seasons.len(), 1);
        assert_eq!(d.seasons[0].team_abbr.as_deref(), Some("PIT"));
        assert_eq!(d.career.len(), 1);
        // All drafts, NHL first: [NHL entry (real pick, era-resolved team), QMJHL junior (nulled pick)].
        assert_eq!(d.drafts.len(), 2);
        let nhl = &d.drafts[0];
        assert!(nhl.is_nhl);
        assert_eq!(nhl.draft_name.as_deref(), Some("NHL Entry Draft"));
        assert_eq!(nhl.overall, Some(1));
        assert_eq!(nhl.pick, Some(1));
        assert_eq!(nhl.team.as_deref(), Some("Pittsburgh Penguins"));
        let jr = &d.drafts[1];
        assert!(!jr.is_nhl);
        assert_eq!(jr.pick, None); // 10000 sentinel nulled at ingest
        assert_eq!(jr.overall, Some(126));
        assert_eq!(jr.team.as_deref(), Some("RIM")); // no lineage -> snapshot abbr
        assert_eq!(d.awards.len(), 1);
    }

    #[test]
    fn seasons_are_rated_seasons_only() {
        let c = fixture();
        // Fleury (person 1): really rated in 2023 + 2024, then FHM re-lists him each later season
        // (2025, 2026) with no team and NO ratings — the retired phantoms. Only the rated seasons
        // should surface; the phantom tail is dropped because it carries no `ratings_season` row.
        c.conn
            .execute_batch(
                r#"
                INSERT INTO player_season VALUES(1,2023,42,15,15,75,185,'Sorel','QC','Canada',NULL,NULL,0,'Left',0);
                INSERT INTO ratings_season VALUES(1,2023,18,0,0,0,0,0,0.85,0.85,'{"Reflexes":16}');
                INSERT INTO player_season VALUES(1,2025,42,-1,15,75,185,'Sorel','QC','Canada',NULL,NULL,1,'Left',0);
                INSERT INTO player_season VALUES(1,2026,42,-1,15,75,185,'Sorel','QC','Canada',NULL,NULL,1,'Left',0);
                "#,
            )
            .unwrap();
        let years: Vec<i64> = c.person_seasons(1).unwrap().iter().map(|s| s.season_year).collect();
        assert_eq!(years, vec![2024, 2023], "only rated seasons appear; phantom re-listings drop");
        assert_eq!(c.person_played_years(1).unwrap(), vec![2024, 2023]);

        // Crosby (person 0): his single rated season stays.
        assert_eq!(c.person_seasons(0).unwrap().len(), 1);
    }

    #[test]
    fn summary_span_and_count_are_rated_seasons() {
        let c = fixture();
        // A retired Euro-league oldtimer with NO career-stat lines at all (like Henrik Aaby): rated
        // 1948 + 1950, then re-listed with no ratings through the current season (2024). Span, count,
        // and played-years must all come from the rated seasons, never the phantom re-listings.
        c.conn
            .execute_batch(
                r#"
                INSERT INTO person VALUES(3,'aaltonen|risto|1930-01-01','Risto','Aaltonen',NULL,'1930-01-01',1930,70,175,'Left','Finland','Helsinki','',1,1948,2024,3,0,'C');
                INSERT INTO ratings_season VALUES(3,1948,0,8,5,19,20,13,0.5,0.5,'{"Speed":10}');
                INSERT INTO ratings_season VALUES(3,1950,0,8,5,19,20,13,0.5,0.5,'{"Speed":11}');
                INSERT INTO player_season VALUES(3,1948,99,15,15,70,175,'Helsinki','','Finland',NULL,NULL,0,'Left',0);
                INSERT INTO player_season VALUES(3,1950,99,15,15,70,175,'Helsinki','','Finland',NULL,NULL,0,'Left',0);
                INSERT INTO player_season VALUES(3,2024,99,-1,15,70,175,'Helsinki','','Finland',NULL,NULL,1,'Left',0);
                "#,
            )
            .unwrap();
        let a = c.person_summaries("aaltonen", None, 50).unwrap().remove(0);
        assert_eq!(a.first_year, Some(1948));
        assert_eq!(a.last_year, Some(1950), "phantom re-listings must not extend the span");
        assert!(!a.active, "last rated season predates the current season → not active");
        assert_eq!(a.seasons_played, 2, "rated seasons 1948, 1950 — no career-stat lines needed");
        assert_eq!(c.person_played_years(3).unwrap(), vec![1950, 1948]);

        // Crosby's last rated season (2024) is the corpus's current season → still active.
        let cros = c.person_summaries("crosby", None, 50).unwrap().remove(0);
        assert_eq!(cros.last_year, Some(2024));
        assert!(cros.active);
    }

    #[test]
    fn team_detail_lists_roster_linked_to_persons() {
        let c = fixture();
        let t = c.team_detail(2024, 15).unwrap().unwrap();
        assert_eq!(t.abbr.as_deref(), Some("PIT"));
        assert_eq!(t.division_name.as_deref(), Some("Metropolitan"));
        assert_eq!(t.roster.len(), 2);
    }

    #[test]
    fn team_index_consolidates_nhl_into_franchise() {
        let c = fixture();
        // NHL + AHL are league options (their team_keys live in `team`).
        let leagues: Vec<String> = c.team_leagues().unwrap().into_iter().map(|l| l.abbr).collect();
        assert!(leagues.contains(&"NHL".to_string()) && leagues.contains(&"AHL".to_string()));
        // "All" = the franchise + the standalone AHL club; the NHL club surfaces once (its franchise).
        let all = c.team_index(None, "", 100).unwrap();
        assert_eq!(all.len(), 2);
        let pens: Vec<_> = all.iter().filter(|e| e.kind == "franchise").collect();
        assert_eq!(pens.len(), 1, "Penguins appear once (franchise), not also as a raw team_key");
        // The NHL filter yields ONLY the franchises — the standalone AHL club must not leak in.
        let nhl = c.team_index(Some("NHL"), "", 100).unwrap();
        assert_eq!(nhl.len(), 1, "NHL filter is the franchises only");
        assert_eq!(nhl[0].kind, "franchise");
        // A non-NHL league yields only its standalone teams; a bogus league yields nothing.
        let ahl = c.team_index(Some("AHL"), "", 100).unwrap();
        assert_eq!(ahl.len(), 1);
        assert_eq!(ahl[0].kind, "team");
        assert_eq!(c.team_index(Some("KHL"), "", 100).unwrap().len(), 0);
        // Search matches the display name.
        assert_eq!(c.team_index(None, "pittsburgh", 100).unwrap().len(), 1);
        assert_eq!(c.team_index(None, "nomatch", 100).unwrap().len(), 0);
    }

    #[test]
    fn franchise_detail_builds_era_named_block_with_rich_roster() {
        let c = fixture();
        let d = c
            .team_franchise_detail("franchise", "0", None)
            .unwrap()
            .unwrap();
        assert_eq!(d.display_name, "Pittsburgh Penguins");
        assert_eq!(d.season_year, 2024);
        assert_eq!(d.seasons, vec![2024]);
        assert_eq!(d.constituents.len(), 1);
        let block = &d.constituents[0];
        // Era name drives the block title; colours + text_color carried through.
        assert_eq!(block.name, "Pittsburgh Penguins");
        assert_eq!(block.text_color.as_deref(), Some("#fff"));
        assert_eq!(block.division_name.as_deref(), Some("Metropolitan"));
        assert_eq!(block.roster.len(), 2);
        // Crosby carries jersey 87 + captaincy, resolved via team_key.
        let cros = block.roster.iter().find(|m| m.last_name == "Crosby").unwrap();
        assert_eq!(cros.jersey, Some(87));
        assert_eq!(cros.captaincy.as_deref(), Some("C"));
        assert_eq!(cros.position.as_deref(), Some("C"));
        // Cap sheet picks up the season contract.
        assert_eq!(block.contracts.len(), 1);
        assert_eq!(block.contracts[0].avg_salary, Some(12_600_000));
    }

    #[test]
    fn standalone_team_detail_absent_key_is_none() {
        let c = fixture();
        assert!(c.team_franchise_detail("team", "no__such", None).unwrap().is_none());
        assert!(c.team_franchise_detail("franchise", "999", None).unwrap().is_none());
    }

    #[test]
    fn schema_mismatch_is_rejected() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch(SCHEMA_SQL).unwrap();
        conn.execute_batch("INSERT INTO meta(key,value) VALUES('schema_version','999');")
            .unwrap();
        assert!(matches!(
            FhmCorpus::from_conn(conn),
            Err(FhmError::SchemaMismatch { .. })
        ));
    }
}
