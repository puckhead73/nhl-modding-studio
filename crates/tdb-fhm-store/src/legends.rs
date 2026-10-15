//! Legends support — the corpus queries the Legends team builder ranks on, plus the **sidecar
//! rating cache** that keeps a formula profile's precomputed Legends ratings between sessions.
//!
//! The corpus itself stays read-only and untouched (it's a build artifact — `tdb-fhm-build`
//! regenerates it wholesale). The cache is a separate `legends_cache.db` written next to the corpus,
//! the same way `ea_formulas.json` / `ea_attribute_maxima.json` already sit beside it. Every run is
//! stamped with a corpus fingerprint so a rebuilt corpus invalidates the cache instead of silently
//! pairing stale ratings with reshuffled `person_id`s.

use std::path::Path;

use rusqlite::{params, Connection, OptionalExtension};

use crate::{FhmCorpus, Res};

// ---------------------------------------------------------------------------
// Corpus queries (read-only)
// ---------------------------------------------------------------------------

/// A person eligible for Legends consideration: identity, position, and NHL regular-season career
/// games. Every person with a derivable position is a candidate — the games threshold is applied by
/// the assignment engine, not here, because the European Elite / Windsor Legends residual teams
/// deliberately ignore it.
#[derive(Debug, Clone)]
pub struct LegendsCandidate {
    pub person_id: i64,
    pub first_name: String,
    pub last_name: String,
    /// Primary position label: `C`/`LW`/`RW`/`D`/`G`.
    pub position: String,
    pub is_goalie: bool,
    pub nationality: Option<String>,
    /// `YYYY-MM-DD`, or `?` when the corpus has no birthdate.
    pub dob: String,
    /// NHL **regular-season** career games played (0 when the person never played in the NHL).
    pub nhl_gp: i64,
}

/// How many NHL regular-season games a person played for one franchise lineage.
#[derive(Debug, Clone)]
pub struct FranchiseTenure {
    pub person_id: i64,
    pub franchise_id: i64,
    pub gp: i64,
}

impl FhmCorpus {
    /// Every person carrying a derivable primary position, with their NHL regular-season career GP.
    pub fn legends_candidates(&self) -> Res<Vec<LegendsCandidate>> {
        let mut stmt = self.conn.prepare(
            "SELECT p.person_id, p.first_name, p.last_name, p.position, p.is_goalie,
                    p.nationality, p.dob, IFNULL(g.gp, 0)
             FROM person p
             LEFT JOIN (
                 SELECT person_id, SUM(gp) AS gp
                 FROM career_stat
                 WHERE phase = 'rs' AND league_abbr = 'NHL' AND gp IS NOT NULL
                 GROUP BY person_id
             ) g ON g.person_id = p.person_id
             WHERE p.position IS NOT NULL AND p.position <> ''",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(LegendsCandidate {
                    person_id: r.get(0)?,
                    first_name: r.get(1)?,
                    last_name: r.get(2)?,
                    position: r.get(3)?,
                    is_goalie: r.get::<_, i64>(4)? != 0,
                    nationality: r.get(5)?,
                    dob: r.get(6)?,
                    nhl_gp: r.get(7)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// NHL regular-season games per (person, franchise lineage), most games first within a person.
    ///
    /// `career_stat.team_id` is namespaced to the export it came from (`src_season`), so the club
    /// resolves through `team_season` to its cross-season `team_key`, then through the franchise
    /// lineage using the year the games were actually played — a 1993 Nordiques season maps to the
    /// Colorado lineage's Quebec era, not to whatever club holds that id today.
    pub fn nhl_franchise_tenure(&self) -> Res<Vec<FranchiseTenure>> {
        let mut stmt = self.conn.prepare(
            "SELECT cs.person_id, fm.franchise_id, SUM(cs.gp) AS gp
             FROM career_stat cs
             JOIN team_season ts
               ON ts.season_year = cs.src_season AND ts.src_team_id = cs.team_id
             JOIN franchise_member fm
               ON fm.team_key = ts.team_key
              AND (fm.era_start IS NULL OR cs.year >  fm.era_start)
              AND (fm.era_end   IS NULL OR cs.year <= fm.era_end)
             WHERE cs.phase = 'rs' AND cs.league_abbr = 'NHL' AND cs.gp IS NOT NULL
             GROUP BY cs.person_id, fm.franchise_id
             ORDER BY cs.person_id, gp DESC",
        )?;
        let rows = stmt
            .query_map([], |r| {
                Ok(FranchiseTenure {
                    person_id: r.get(0)?,
                    franchise_id: r.get(1)?,
                    gp: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// A cheap identity stamp for the open corpus. Any rebuild that changes the person or career
    /// population changes this, which retires every cached Legends run built against the old one.
    pub fn legends_fingerprint(&self) -> Res<String> {
        let persons: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM person", [], |r| r.get(0))?;
        let careers: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM career_stat", [], |r| r.get(0))?;
        Ok(format!(
            "v{}:p{}:c{}",
            tdb_fhm::SCHEMA_VERSION,
            persons,
            careers
        ))
    }
}

// ---------------------------------------------------------------------------
// Sidecar rating cache (read-write)
// ---------------------------------------------------------------------------

/// One person's cached Legends rating under a formula profile.
#[derive(Debug, Clone)]
pub struct CachedRating {
    pub person_id: i64,
    /// `forward` / `defenseman` / `goalie` — the formula bucket the composite ran under.
    pub position: String,
    pub overall: i64,
    /// How many playing seasons fed the composite.
    pub seasons_used: i64,
    pub archetype: Option<String>,
    /// The composited EA rating list, as JSON (`[{ea_key, value, …}]`).
    pub attrs_json: String,
}

/// The record of one precompute pass over the corpus for a formula profile.
#[derive(Debug, Clone)]
pub struct LegendsRun {
    pub profile_slug: String,
    pub profile_name: String,
    /// The corpus fingerprint the run was built against ([`FhmCorpus::legends_fingerprint`]).
    pub fingerprint: String,
    /// RFC3339-ish timestamp, written by the caller.
    pub computed_at: String,
    pub player_count: i64,
}

const CACHE_DDL: &str = r#"
CREATE TABLE IF NOT EXISTS legends_run (
    profile_slug  TEXT PRIMARY KEY,
    profile_name  TEXT NOT NULL,
    fingerprint   TEXT NOT NULL,
    computed_at   TEXT NOT NULL,
    player_count  INTEGER NOT NULL
);
CREATE TABLE IF NOT EXISTS legends_rating (
    profile_slug  TEXT NOT NULL,
    person_id     INTEGER NOT NULL,
    position      TEXT NOT NULL,
    overall       INTEGER NOT NULL,
    seasons_used  INTEGER NOT NULL,
    archetype     TEXT,
    attrs_json    TEXT NOT NULL,
    PRIMARY KEY (profile_slug, person_id)
) WITHOUT ROWID;
CREATE INDEX IF NOT EXISTS idx_legends_rating_ovr
    ON legends_rating(profile_slug, overall DESC);

-- The materialised rosters. Assignment is a global pass over the whole rated field (two corpus
-- scans plus a sort), so its RESULT is stored rather than recomputed per page view: Puck's Picks
-- ships as a default section, and browsing it must not re-derive the league on every click. Both
-- tables are rewritten wholesale alongside their profile's run, so they can never describe a
-- different rating set than the one sitting beside them.
CREATE TABLE IF NOT EXISTS legends_team (
    profile_slug  TEXT NOT NULL,
    team_key      TEXT NOT NULL,          -- 'franchise:<id>' | a residual-team key
    name          TEXT NOT NULL,          -- resolved at build time so reads need no corpus join
    abbr          TEXT,
    franchise_id  INTEGER,                -- NULL for the two Legends-only teams
    forwards      INTEGER NOT NULL,
    defense       INTEGER NOT NULL,
    goalies       INTEGER NOT NULL,
    PRIMARY KEY (profile_slug, team_key)
) WITHOUT ROWID;

-- Roster rows carry the player's bio denormalised, for the same reason: a roster view becomes one
-- indexed read of ~26 rows with no person lookup behind it.
CREATE TABLE IF NOT EXISTS legends_team_member (
    profile_slug  TEXT NOT NULL,
    team_key      TEXT NOT NULL,
    person_id     INTEGER NOT NULL,
    slot          TEXT NOT NULL,          -- 'F' | 'D' | 'G'
    first_name    TEXT NOT NULL,
    last_name     TEXT NOT NULL,
    position      TEXT NOT NULL,
    is_goalie     INTEGER NOT NULL,
    dob           TEXT NOT NULL,
    nationality   TEXT,
    overall       INTEGER NOT NULL,
    team_gp       INTEGER NOT NULL,
    second_choice INTEGER NOT NULL,
    PRIMARY KEY (profile_slug, team_key, person_id)
) WITHOUT ROWID;
"#;

/// One built team as stored in the cache.
#[derive(Debug, Clone)]
pub struct CachedTeam {
    pub team_key: String,
    pub name: String,
    pub abbr: Option<String>,
    /// `None` for the two Legends-only teams.
    pub franchise_id: Option<i64>,
    pub forwards: i64,
    pub defense: i64,
    pub goalies: i64,
}

/// One roster row as stored in the cache — the player's bio is denormalised so a roster read needs
/// no corpus lookup at all.
#[derive(Debug, Clone)]
pub struct CachedMember {
    pub team_key: String,
    pub person_id: i64,
    /// `F` | `D` | `G`.
    pub slot: String,
    pub first_name: String,
    pub last_name: String,
    pub position: String,
    pub is_goalie: bool,
    pub dob: String,
    pub nationality: Option<String>,
    pub overall: i64,
    pub team_gp: i64,
    pub second_choice: bool,
}

/// The writable sidecar holding every profile's precomputed Legends ratings.
pub struct LegendsCache {
    conn: Connection,
}

impl LegendsCache {
    /// Open (creating if absent) the cache at `path`.
    pub fn open(path: &Path) -> Res<Self> {
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA synchronous = NORMAL;")?;
        conn.execute_batch(CACHE_DDL)?;
        Ok(Self { conn })
    }

    /// The run recorded for `slug`, if any.
    pub fn run(&self, slug: &str) -> Res<Option<LegendsRun>> {
        let row = self
            .conn
            .query_row(
                "SELECT profile_slug, profile_name, fingerprint, computed_at, player_count
                 FROM legends_run WHERE profile_slug = ?1",
                params![slug],
                map_run,
            )
            .optional()?;
        Ok(row)
    }

    /// Every recorded run, newest first.
    pub fn runs(&self) -> Res<Vec<LegendsRun>> {
        let mut stmt = self.conn.prepare(
            "SELECT profile_slug, profile_name, fingerprint, computed_at, player_count
             FROM legends_run ORDER BY computed_at DESC",
        )?;
        let rows = stmt
            .query_map([], map_run)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Every cached rating for `slug` (ordered by Overall descending — the assignment order).
    pub fn ratings(&self, slug: &str) -> Res<Vec<CachedRating>> {
        let mut stmt = self.conn.prepare(
            "SELECT person_id, position, overall, seasons_used, archetype, attrs_json
             FROM legends_rating WHERE profile_slug = ?1 ORDER BY overall DESC, person_id",
        )?;
        let rows = stmt
            .query_map(params![slug], |r| {
                Ok(CachedRating {
                    person_id: r.get(0)?,
                    position: r.get(1)?,
                    overall: r.get(2)?,
                    seasons_used: r.get(3)?,
                    archetype: r.get(4)?,
                    attrs_json: r.get(5)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Replace a profile's whole run + rating set in one transaction — a precompute pass is
    /// all-or-nothing, so a crash mid-write can never leave a half-populated profile that would
    /// then rank players against a partial field.
    pub fn replace_run(&mut self, run: &LegendsRun, ratings: &[CachedRating]) -> Res<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM legends_rating WHERE profile_slug = ?1",
            params![run.profile_slug],
        )?;
        {
            let mut ins = tx.prepare(
                "INSERT INTO legends_rating
                   (profile_slug, person_id, position, overall, seasons_used, archetype, attrs_json)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )?;
            for r in ratings {
                ins.execute(params![
                    run.profile_slug,
                    r.person_id,
                    r.position,
                    r.overall,
                    r.seasons_used,
                    r.archetype,
                    r.attrs_json,
                ])?;
            }
        }
        tx.execute(
            "INSERT INTO legends_run
               (profile_slug, profile_name, fingerprint, computed_at, player_count)
             VALUES (?1, ?2, ?3, ?4, ?5)
             ON CONFLICT(profile_slug) DO UPDATE SET
               profile_name = excluded.profile_name,
               fingerprint  = excluded.fingerprint,
               computed_at  = excluded.computed_at,
               player_count = excluded.player_count",
            params![
                run.profile_slug,
                run.profile_name,
                run.fingerprint,
                run.computed_at,
                run.player_count
            ],
        )?;
        tx.commit()?;
        Ok(())
    }

    /// True when this profile already has its rosters materialised.
    pub fn has_assignment(&self, slug: &str) -> Res<bool> {
        let n: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM legends_team WHERE profile_slug = ?1",
            params![slug],
            |r| r.get(0),
        )?;
        Ok(n > 0)
    }

    /// Every built team for `slug` — franchise teams first (by name), the residual teams last.
    pub fn teams(&self, slug: &str) -> Res<Vec<CachedTeam>> {
        let mut stmt = self.conn.prepare(
            "SELECT team_key, name, abbr, franchise_id, forwards, defense, goalies
             FROM legends_team WHERE profile_slug = ?1
             ORDER BY (franchise_id IS NULL), name",
        )?;
        let rows = stmt
            .query_map(params![slug], |r| {
                Ok(CachedTeam {
                    team_key: r.get(0)?,
                    name: r.get(1)?,
                    abbr: r.get(2)?,
                    franchise_id: r.get(3)?,
                    forwards: r.get(4)?,
                    defense: r.get(5)?,
                    goalies: r.get(6)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// One team's roster, forwards then defense then goalies, best Overall first within each.
    pub fn roster(&self, slug: &str, team_key: &str) -> Res<Vec<CachedMember>> {
        let mut stmt = self.conn.prepare(
            "SELECT team_key, person_id, slot, first_name, last_name, position, is_goalie,
                    dob, nationality, overall, team_gp, second_choice
             FROM legends_team_member
             WHERE profile_slug = ?1 AND team_key = ?2
             ORDER BY CASE slot WHEN 'F' THEN 0 WHEN 'D' THEN 1 ELSE 2 END,
                      overall DESC, person_id",
        )?;
        let rows = stmt
            .query_map(params![slug, team_key], |r| {
                Ok(CachedMember {
                    team_key: r.get(0)?,
                    person_id: r.get(1)?,
                    slot: r.get(2)?,
                    first_name: r.get(3)?,
                    last_name: r.get(4)?,
                    position: r.get(5)?,
                    is_goalie: r.get::<_, i64>(6)? != 0,
                    dob: r.get(7)?,
                    nationality: r.get(8)?,
                    overall: r.get(9)?,
                    team_gp: r.get(10)?,
                    second_choice: r.get::<_, i64>(11)? != 0,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    /// Replace a profile's materialised rosters in one transaction, for the same all-or-nothing
    /// reason [`Self::replace_run`] is transactional — a half-written league is worse than none.
    pub fn replace_assignment(
        &mut self,
        slug: &str,
        teams: &[CachedTeam],
        members: &[CachedMember],
    ) -> Res<()> {
        let tx = self.conn.transaction()?;
        tx.execute(
            "DELETE FROM legends_team WHERE profile_slug = ?1",
            params![slug],
        )?;
        tx.execute(
            "DELETE FROM legends_team_member WHERE profile_slug = ?1",
            params![slug],
        )?;
        {
            let mut ins = tx.prepare(
                "INSERT INTO legends_team
                   (profile_slug, team_key, name, abbr, franchise_id, forwards, defense, goalies)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
            )?;
            for t in teams {
                ins.execute(params![
                    slug,
                    t.team_key,
                    t.name,
                    t.abbr,
                    t.franchise_id,
                    t.forwards,
                    t.defense,
                    t.goalies
                ])?;
            }
        }
        {
            let mut ins = tx.prepare(
                "INSERT INTO legends_team_member
                   (profile_slug, team_key, person_id, slot, first_name, last_name, position,
                    is_goalie, dob, nationality, overall, team_gp, second_choice)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            )?;
            for m in members {
                ins.execute(params![
                    slug,
                    m.team_key,
                    m.person_id,
                    m.slot,
                    m.first_name,
                    m.last_name,
                    m.position,
                    m.is_goalie as i64,
                    m.dob,
                    m.nationality,
                    m.overall,
                    m.team_gp,
                    m.second_choice as i64
                ])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// Drop a profile's cached run (used when the profile itself is deleted).
    pub fn delete_run(&self, slug: &str) -> Res<()> {
        self.conn.execute(
            "DELETE FROM legends_rating WHERE profile_slug = ?1",
            params![slug],
        )?;
        self.conn.execute(
            "DELETE FROM legends_team_member WHERE profile_slug = ?1",
            params![slug],
        )?;
        self.conn.execute(
            "DELETE FROM legends_team WHERE profile_slug = ?1",
            params![slug],
        )?;
        self.conn.execute(
            "DELETE FROM legends_run WHERE profile_slug = ?1",
            params![slug],
        )?;
        Ok(())
    }
}

fn map_run(r: &rusqlite::Row<'_>) -> rusqlite::Result<LegendsRun> {
    Ok(LegendsRun {
        profile_slug: r.get(0)?,
        profile_name: r.get(1)?,
        fingerprint: r.get(2)?,
        computed_at: r.get(3)?,
        player_count: r.get(4)?,
    })
}
